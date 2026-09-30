//! Controller full browse contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn browse_errors_redact_internal_details() {
    let record = crate::BrowseRecord {
        username: "friend".to_owned(),
        status: "failed",
        entries: Vec::new(),
        reason: Some(
            "plain browse connect failed at 10.0.0.9:2242: /private/socket denied".to_owned(),
        ),
        folder: None,
        indirect_token: None,
        requested_at: Some(1),
        updated_at: 2,
    };

    for body in [record.json(), record.controller_status_json()] {
        assert!(body.contains("browse failed"));
        assert!(!body.contains("10.0.0.9"));
        assert!(!body.contains("/private"));
        assert!(!body.contains("denied"));
    }
    assert!(record.reason.as_deref().unwrap().contains("10.0.0.9"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn browse_failure_events_redact_internal_details() {
    let (state, _receiver) = test_state();
    crate::record_event(
        &state,
        "browse.failed",
        "friend",
        Some("connect to 10.0.0.9:2242 via /private/socket denied".to_owned()),
    )
    .await;

    let events = state.events.read().await;
    let event = events.records.last().expect("browse failure event");
    assert_eq!(event.detail.as_deref(), Some("browse failed"));
    for body in [event.json(), event.controller_json().to_string()] {
        assert!(!body.contains("10.0.0.9"));
        assert!(!body.contains("/private"));
        assert!(!body.contains("denied"));
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn browse_cache_persists_and_rehydrates_records() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";

    let requested = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request browse");
    assert_eq!(requested.status, "202 Accepted");
    let _ = receiver.try_recv();

    let ingested = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        r#"{"username":"friend","directories":[{"name":"Remote/Album","files":[{"filename":"Track.flac","size":123,"extension":"flac"}]}]}"#,
        &state,
    )
    .await
    .expect("ingest browse");
    assert_eq!(ingested.status, "200 OK");

    let persisted = db.list_browse_records(10, 0).await.expect("list browse");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].username, "friend");
    assert_eq!(persisted[0].status, "ready");
    assert!(persisted[0]
        .entries_json
        .contains("Remote/Album/Track.flac"));

    let rehydrated = crate::BrowseStore::from_persisted(persisted);
    assert!(rehydrated
        .json(None)
        .contains("\"filename\":\"Remote/Album/Track.flac\""));
    assert!(rehydrated
        .get("friend")
        .expect("rehydrated browse")
        .controller_status_json()
        .contains("\"state\":\"Completed\""));

    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("browse database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["browse"], 1);
    assert_eq!(stats_json["persisted"]["browse"], 1);
    assert_eq!(stats_json["projections"]["browse"], 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn browse_indirect_tokens_wrap_without_aliasing_pending_records() {
    let mut browse = crate::BrowseStore::with_limits(4, 4);
    browse.request("alice".to_owned()).unwrap();
    browse.next_indirect_token = u32::MAX;
    assert_eq!(
        browse.mark_indirect_pending("alice", "fallback".to_owned()),
        Some(u32::MAX)
    );
    browse.request("bob".to_owned()).unwrap();
    assert_eq!(
        browse.mark_indirect_pending("bob", "fallback".to_owned()),
        Some(1)
    );
    browse.request("carol".to_owned()).unwrap();
    browse.next_indirect_token = u32::MAX;
    assert_eq!(
        browse.mark_indirect_pending("carol", "fallback".to_owned()),
        Some(2)
    );
    let next_token = browse.next_indirect_token;
    assert_eq!(browse.mark_indirect_pending("missing", String::new()), None);
    assert_eq!(browse.next_indirect_token, next_token);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn browse_store_bounds_records_and_entries_but_updates_existing_users() {
    let mut browse = crate::BrowseStore::with_limits(1, 2);
    browse.request("alice".to_owned()).unwrap();
    assert!(browse.request("bob".to_owned()).is_none());

    let entries = (0..3)
        .map(|index| crate::BrowseEntry {
            path_encoding: Default::default(),
            filename: format!("file-{index}.flac"),
            size: index,
            extension: "flac".to_owned(),
        })
        .collect();
    let record = browse
        .add_entries("alice".to_owned(), entries, false)
        .unwrap();
    assert_eq!(record.entries.len(), 2);
    assert_eq!(record.entries[0].filename, "file-0.flac");
    assert_eq!(record.entries[1].filename, "file-1.flac");
    assert!(browse
        .add_entries("bob".to_owned(), Vec::new(), true)
        .is_none());
    assert_eq!(browse.records.len(), 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn browse_store_bounds_text_and_aggregate_entries() {
    let oversized_username = "é".repeat(crate::browse_store::MAX_BROWSE_USERNAME_BYTES);
    let mut browse =
        crate::BrowseStore::with_limits(2, crate::browse_store::MAX_TOTAL_BROWSE_ENTRIES + 1);
    let requested = browse.request(oversized_username.clone()).unwrap();
    assert!(requested.username.len() <= crate::browse_store::MAX_BROWSE_USERNAME_BYTES);
    browse.request("other".to_owned()).unwrap();

    let oversized_entry = crate::BrowseEntry {
        path_encoding: Default::default(),
        filename: "f".repeat(crate::browse_store::MAX_BROWSE_FILENAME_BYTES + 1),
        size: 1,
        extension: "e".repeat(crate::browse_store::MAX_BROWSE_EXTENSION_BYTES + 1),
    };
    let record = browse
        .add_entries(oversized_username.clone(), vec![oversized_entry], false)
        .unwrap();
    assert_eq!(
        record.entries[0].filename.len(),
        crate::browse_store::MAX_BROWSE_FILENAME_BYTES
    );
    assert_eq!(
        record.entries[0].extension.len(),
        crate::browse_store::MAX_BROWSE_EXTENSION_BYTES
    );

    browse.records[0].entries = (0..crate::browse_store::MAX_TOTAL_BROWSE_ENTRIES)
        .map(|index| crate::BrowseEntry {
            path_encoding: Default::default(),
            filename: format!("file-{index}"),
            size: 1,
            extension: String::new(),
        })
        .collect();
    let record = browse
        .add_entries(
            "other".to_owned(),
            vec![crate::BrowseEntry {
                path_encoding: Default::default(),
                filename: "rejected".to_owned(),
                size: 1,
                extension: String::new(),
            }],
            true,
        )
        .unwrap();
    assert!(record.entries.is_empty());
    assert_eq!(
        browse.total_entries(),
        crate::browse_store::MAX_TOTAL_BROWSE_ENTRIES
    );
    assert!(browse.get(&oversized_username).is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn browse_routes_roll_back_when_persistence_fails() {
    for (path, body) in [
        ("/api/v0/users/friend/browse/request", ""),
        (
            "/api/v0/users/friend/browse/folder",
            r#"{"folder":"Remote/Album"}"#,
        ),
        ("/api/v0/users/friend/browse/fail", r#"{"reason":"failed"}"#),
        (
            "/api/v0/users/friend/browse/cancel",
            r#"{"reason":"cancelled"}"#,
        ),
        (
            "/api/v0/browse-responses",
            r#"{"username":"friend","entries":[{"filename":"Song.flac"}]}"#,
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state.session.write().await.state = "connected";
        let previous = state.browse.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("failed browse persistence response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("browse persistence failed"),
            "{path}"
        );
        assert_eq!(*state.browse.read().await, previous, "{path}");
        assert!(receiver.try_recv().is_err(), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn browse_response_api_accepts_single_flattened_entry() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"filename\":\"Remote/One.mp3\",\"size\":7}",
        &state,
    )
    .await
    .expect("flat browse response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"count\":1"));
    assert!(response.body.contains("\"extension\":\"mp3\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn browse_response_api_accepts_controller_directory_payload() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"directories\":[{\"name\":\"Remote/Album\",\"files\":[{\"filename\":\"One.flac\",\"size\":1},{\"filename\":\"Remote/Album/Two.mp3\",\"size\":2}]}]}",
        &state,
    )
    .await
    .expect("slskd browse response");

    assert_eq!(response.status, "200 OK");
    let record_json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(record_json["count"], 2);
    assert_eq!(
        record_json["entries"][0]["filename"],
        "Remote/Album/One.flac"
    );
    assert_eq!(
        record_json["entries"][1]["filename"],
        "Remote/Album/Two.mp3"
    );

    let root = crate::route_http_request("GET", "/api/users/friend/browse", None, "", &state)
        .await
        .expect("slskd browse root");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["directoryCount"], 1);
    assert_eq!(root_json["directories"][0]["name"], "Remote/Album");
    assert_eq!(root_json["directories"][0]["fileCount"], 2);

    let directory = crate::route_http_request(
        "POST",
        "/api/users/friend/directory",
        None,
        "{\"directory\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("slskd directory");
    let directory_json = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap();
    assert_eq!(
        directory_json[0]["files"][0]["filename"],
        "Remote/Album/One.flac"
    );
    assert_eq!(
        directory_json[0]["files"][1]["filename"],
        "Remote/Album/Two.mp3"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn browse_cancel_route_preserves_terminal_status_projection() {
    let (state, _receiver) = test_state();
    state.session.write().await.state = "connected";

    crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request browse");

    let cancelled = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/cancel",
        None,
        r#"{"reason":"user navigated away"}"#,
        &state,
    )
    .await
    .expect("cancel browse");
    assert_eq!(cancelled.status, "200 OK");
    let cancelled_json = serde_json::from_str::<serde_json::Value>(&cancelled.body).unwrap();
    assert_eq!(cancelled_json["status"], "cancelled");
    assert_eq!(cancelled_json["reason"], "browse cancelled");

    let status =
        crate::route_http_request("GET", "/api/users/friend/browse/status", None, "", &state)
            .await
            .expect("cancelled browse status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "cancelled");
    assert_eq!(status_json["state"], "Cancelled");
    assert_eq!(status_json["reason"], "browse cancelled");

    let listed =
        crate::route_http_request("GET", "/api/v0/browse?status=cancelled", None, "", &state)
            .await
            .expect("cancelled browse list");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    assert_eq!(listed_json["filtered_count"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn browse_response_api_rejects_missing_fields() {
    let (state, _receiver) = test_state();

    let response =
        crate::route_http_request("POST", "/api/v0/browse-responses", None, "{}", &state)
            .await
            .expect("bad browse response");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(response.body, "{\"error\":\"username is required\"}");
}
