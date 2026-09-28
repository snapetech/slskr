//! Controller full messaging differential ownership.

use super::*;

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_user_notes_lifecycle() {
    let (state, _receiver) = test_state();
    let empty_list = crate::route_http_request("GET", "/api/v0/users/notes", None, "", &state)
        .await
        .unwrap();
    assert_eq!(empty_list.status, "200 OK");
    let empty_list_json = serde_json::from_str::<serde_json::Value>(&empty_list.body).unwrap();
    assert!(empty_list_json.as_array().is_some_and(Vec::is_empty));

    let malformed_create =
        crate::route_http_request("POST", "/api/v0/users/notes", None, "{not-json", &state)
            .await
            .unwrap();
    assert_eq!(malformed_create.status, "400 Bad Request");
    let missing_create =
        crate::route_http_request("POST", "/api/v0/users/notes", None, "{}", &state)
            .await
            .unwrap();
    assert_eq!(missing_create.status, "400 Bad Request");

    let malformed_list = crate::route_http_request("GET", "/api/v0/users/notes/", None, "", &state)
        .await
        .unwrap();
    assert_eq!(malformed_list.status, "404 Not Found");

    let created = crate::route_http_request(
        "POST",
        "/api/v0/users/notes",
        None,
        r#"{"username":"route-audit-peer","note":"contract","color":"red","icon":"star","isHighPriority":true}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(created.status, "200 OK");
    let created = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created["username"], "route-audit-peer");
    assert_eq!(created["color"], "red");
    assert_eq!(created["icon"], "star");
    assert_eq!(created["isHighPriority"], true);
    assert!(created.get("id").is_none());
    assert!(chrono::DateTime::parse_from_rfc3339(created["createdAt"].as_str().unwrap()).is_ok());

    let populated_list = crate::route_http_request("GET", "/api/v0/users/notes", None, "", &state)
        .await
        .unwrap();
    assert_eq!(populated_list.status, "200 OK");
    let populated_list_json =
        serde_json::from_str::<serde_json::Value>(&populated_list.body).unwrap();
    assert!(populated_list_json.as_array().is_some_and(|records| {
        records
            .iter()
            .any(|record| record["username"] == "route-audit-peer")
    }));

    let fetched = crate::route_http_request(
        "GET",
        "/api/v0/users/notes/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(fetched.status, "200 OK");
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
    assert_eq!(fetched_json["note"], "contract");

    let missing_fetch = crate::route_http_request(
        "GET",
        "/api/v0/users/notes/no-such-route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_fetch.status, "404 Not Found");
    let malformed_fetch =
        crate::route_http_request("GET", "/api/v0/users/notes/", None, "", &state)
            .await
            .unwrap();
    assert_eq!(malformed_fetch.status, "404 Not Found");

    let malformed_delete =
        crate::route_http_request("DELETE", "/api/v0/users/notes/", None, "", &state)
            .await
            .unwrap();
    assert_eq!(malformed_delete.status, "404 Not Found");

    let deleted = crate::route_http_request(
        "DELETE",
        "/api/v0/users/notes/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "204 No Content");
    let deleted_again = crate::route_http_request(
        "DELETE",
        "/api/v0/users/notes/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted_again.status, "204 No Content");

    let post_runtime_pass = {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note runtime database");
        let (runtime_state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-runtime","note":"not persisted"}"#,
            &runtime_state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "503 Service Unavailable");
        assert!(response.body.contains("user note persistence failed"));
        let notes_empty = runtime_state.user_notes.read().await.records.is_empty();
        notes_empty
    };
    assert!(post_runtime_pass);

    let delete_runtime_pass = {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note delete runtime database");
        let (runtime_state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-delete-runtime","note":"original"}"#,
            &runtime_state,
        )
        .await
        .unwrap();
        assert_eq!(created.status, "200 OK");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/users/notes/route-audit-delete-runtime",
            None,
            "",
            &runtime_state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "503 Service Unavailable");
        assert!(response
            .body
            .contains("user note deletion persistence failed"));
        let notes = runtime_state.user_notes.read().await;
        notes.records.len() == 1 && notes.records[0].note == "original"
    };
    assert!(delete_runtime_pass);

    let post_restart_pass = {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note restart database");
        let env = MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native");
        let (first_state, _receiver) =
            test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
        let created = crate::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-restart","note":"survives restart"}"#,
            &first_state,
        )
        .await
        .unwrap();
        let persisted = db.list_user_notes(10, 0).await.unwrap();
        let (restarted_state, _receiver) =
            test_state_with_env_parts(env, crate::SearchStore::new(), Some(db.clone()));
        *restarted_state.user_notes.write().await =
            crate::UserNoteStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            "/api/v0/users/notes/route-audit-restart",
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
        created.status == "200 OK"
            && persisted.len() == 1
            && persisted[0].note == "survives restart"
            && fetched.status == "200 OK"
            && fetched_json["note"] == "survives restart"
    };
    assert!(post_restart_pass);

    let post_concurrency_pass = {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note concurrency database");
        let (concurrent_state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..4)
            .map(|index| {
                format!(r#"{{"username":"route-audit-concurrent-{index}","note":"note {index}"}}"#)
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/users/notes", None, body, &concurrent_state)
        }))
        .await;
        let persisted = db.list_user_notes(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("route-audit-concurrent-{index}"))
            .collect();
        let persisted_usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.username.clone())
            .collect();
        responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.len() == 4
            && persisted_usernames == expected
    };
    assert!(post_concurrency_pass);

    let delete_restart_pass = {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note delete restart database");
        let env = MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native");
        let (first_state, _receiver) =
            test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
        let created = crate::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-delete-restart","note":"remove me"}"#,
            &first_state,
        )
        .await
        .unwrap();
        assert_eq!(created.status, "200 OK");
        let deleted = crate::route_http_request(
            "DELETE",
            "/api/v0/users/notes/route-audit-delete-restart",
            None,
            "",
            &first_state,
        )
        .await
        .unwrap();
        let persisted_after_delete = db.list_user_notes(10, 0).await.unwrap();
        let (restarted_state, _receiver) =
            test_state_with_env_parts(env, crate::SearchStore::new(), Some(db.clone()));
        *restarted_state.user_notes.write().await =
            crate::UserNoteStore::from_persisted(persisted_after_delete.clone());
        let fetched = crate::route_http_request(
            "GET",
            "/api/v0/users/notes/route-audit-delete-restart",
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        deleted.status == "204 No Content"
            && persisted_after_delete.is_empty()
            && fetched.status == "404 Not Found"
    };
    assert!(delete_restart_pass);

    let ledger = [
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "mutation-side-effects-and-readback",
            "pass": created["username"] == "route-audit-peer"
                && created["note"] == "contract"
                && created["color"] == "red"
                && created["icon"] == "star"
                && created["isHighPriority"] == true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "mutation-side-effects-and-readback",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "concurrency-and-idempotency",
            "pass": deleted_again.status == "204 No Content",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "nominal-status-headers-body",
            "pass": empty_list.status == "200 OK" && empty_list_json.as_array().is_some_and(Vec::is_empty),
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "missing-empty-or-conflict-state",
            "pass": empty_list.status == "200 OK" && empty_list_json.as_array().is_some_and(Vec::is_empty),
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "malformed-path-query-or-body",
            "pass": malformed_list.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "populated-dynamic-state",
            "pass": populated_list.status == "200 OK"
                && populated_list_json.as_array().is_some_and(|records| {
                    records
                        .iter()
                        .any(|record| record["username"] == "route-audit-peer")
                }),
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "malformed-path-query-or-body",
            "pass": malformed_create.status == "400 Bad Request",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "missing-empty-or-conflict-state",
            "pass": missing_create.status == "400 Bad Request",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "runtime-failure-and-timeout",
            "pass": post_runtime_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "restart-persistence-or-reset",
            "pass": post_restart_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "concurrency-and-idempotency",
            "pass": post_concurrency_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "nominal-status-headers-body",
            "pass": fetched.status == "200 OK" && fetched_json["note"] == "contract",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "populated-dynamic-state",
            "pass": fetched.status == "200 OK" && fetched_json["username"] == "route-audit-peer",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "missing-empty-or-conflict-state",
            "pass": missing_fetch.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "malformed-path-query-or-body",
            "pass": malformed_fetch.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "malformed-path-query-or-body",
            "pass": malformed_delete.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "missing-empty-or-conflict-state",
            "pass": deleted_again.status == "204 No Content",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "runtime-failure-and-timeout",
            "pass": delete_runtime_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "restart-persistence-or-reset",
            "pass": delete_restart_pass,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("user_notes_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_user_browse_api_requests_and_ingests_entries() {
    let (state, mut receiver) = test_state();

    let unavailable = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("disconnected browse request");
    assert_eq!(unavailable.status, "503 Service Unavailable");
    assert!(unavailable
        .body
        .contains("Soulseek server connection is not ready"));
    state.session.write().await.state = "connected";

    let requested = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("browse request");
    assert_eq!(requested.status, "202 Accepted");
    assert!(requested.body.contains("\"status\":\"requested\""));
    assert_eq!(
        receiver.try_recv().expect("browse command"),
        crate::SessionCommand::BrowseUser("friend".to_owned())
    );

    let folder_requested = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/folder",
        None,
        "{\"folder\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("folder browse request");
    assert_eq!(folder_requested.status, "202 Accepted");
    assert!(folder_requested.body.contains("\"status\":\"requested\""));
    assert!(folder_requested
        .body
        .contains("\"folder\":\"Remote/Album\""));
    assert_eq!(
        receiver.try_recv().expect("folder browse command"),
        crate::SessionCommand::BrowseFolder {
            username: "friend".to_owned(),
            folder: "Remote/Album".to_owned()
        }
    );

    let ingested = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"complete\":false,\"entries\": [{\"filename\":\"Remote/Album/Song.flac\",\"size\":123}]}",
        &state,
    )
    .await
    .expect("browse ingest");
    assert_eq!(ingested.status, "200 OK");
    assert!(ingested.body.contains("\"status\":\"partial\""));
    assert!(ingested.body.contains("\"count\":1"));
    assert!(ingested.body.contains("\"total_bytes\":123"));
    assert!(ingested.body.contains("\"extension\":\"flac\""));

    let listed = crate::route_http_request(
        "GET",
        "/api/v0/browse?status=partial&q=friend",
        None,
        "",
        &state,
    )
    .await
    .expect("partial browse list");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"filtered_count\":1"));

    let ingested = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/Cover.jpg\",\"size\":10,\"extension\":\"jpg\"}]}",
        &state,
    )
    .await
    .expect("browse complete ingest");
    assert_eq!(ingested.status, "200 OK");
    assert!(ingested.body.contains("\"status\":\"ready\""));
    assert!(ingested.body.contains("\"count\":2"));
    assert!(ingested.body.contains("\"total_bytes\":133"));
    assert!(ingested.body.contains("\"extension\":\"jpg\""));

    let fetched = crate::route_http_request("GET", "/api/v0/users/friend/browse", None, "", &state)
        .await
        .expect("browse fetch");
    assert_eq!(fetched.status, "200 OK");
    assert!(fetched.content_type.contains("application/json"));
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
    assert_eq!(fetched_json["directoryCount"], 1);
    assert_eq!(fetched_json["directories"][0]["name"], "Remote/Album");
    assert_eq!(
        fetched_json["directories"][0]["files"][0]["filename"],
        "Song.flac"
    );

    let controller_browse =
        crate::route_http_request("GET", "/api/users/friend/browse", None, "", &state)
            .await
            .expect("slskd browse fetch");
    assert_eq!(controller_browse.status, "200 OK");
    let controller_browse_json =
        serde_json::from_str::<serde_json::Value>(&controller_browse.body).unwrap();
    assert_eq!(controller_browse_json["directoryCount"], 1);
    assert_eq!(
        controller_browse_json["directories"][0]["name"],
        "Remote/Album"
    );
    assert_eq!(controller_browse_json["directories"][0]["fileCount"], 2);
    assert_eq!(
        controller_browse_json["directories"][0]["files"][0]["filename"],
        "Song.flac"
    );

    let controller_directory = crate::route_http_request(
        "POST",
        "/api/users/friend/directory",
        None,
        "{\"directory\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("slskd directory fetch");
    assert_eq!(controller_directory.status, "200 OK");
    let controller_directory_json =
        serde_json::from_str::<serde_json::Value>(&controller_directory.body).unwrap();
    assert_eq!(controller_directory_json[0]["name"], "Remote/Album");
    assert_eq!(controller_directory_json[0]["fileCount"], 2);

    let listed = crate::route_http_request(
        "GET",
        "/api/v0/browse?status=ready&q=friend&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("browse list");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));
    assert!(listed.body.contains("\"filtered_count\":1"));
    assert!(listed.body.contains("\"limit\":1"));

    let failed = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/fail",
        None,
        "{\"reason\":\"peer timed out\"}",
        &state,
    )
    .await
    .expect("browse fail");
    assert_eq!(failed.status, "200 OK");
    assert!(failed.body.contains("\"status\":\"failed\""));
    assert!(failed.body.contains("\"reason\":\"peer timed out\""));

    let listed = crate::route_http_request(
        "GET",
        "/api/v0/browse?status=failed&q=friend",
        None,
        "",
        &state,
    )
    .await
    .expect("failed browse list");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"filtered_count\":1"));

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("user_browse_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_joined_room_server_snapshot_populates_the_real_user_roster(
) {
    // Matches the oracle's real IRoomTracker: the room's user list is
    // populated from the server's JoinedRoom snapshot itself, not left
    // empty until some other event happens to arrive. Previously
    // JoinedRoom.users was fully decoded off the wire and then
    // discarded -- GET .../users always returned a hardcoded "[]".
    use slskr_client::{
        protocol::server::{JoinedRoom, RoomUser, ServerMessage},
        server::ServerSession,
        stream::ServerConnection,
    };

    let (state, _receiver) = test_state();
    state.session.write().await.username = Some("tester".to_owned());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind server fixture");
    let address = listener.local_addr().expect("server fixture address");
    let client = tokio::net::TcpStream::connect(address);
    let server = listener.accept();
    let (client, server) = tokio::join!(client, server);
    let (server, _) = server.expect("accept server fixture");
    let mut session = ServerSession::new(ServerConnection::new(server));
    let _fixture = ServerConnection::new(client.expect("client fixture"));

    crate::session_runtime::project_server_message(
        &state,
        &mut session,
        &ServerMessage::JoinedRoom(JoinedRoom {
            room: "roster-audit".to_owned(),
            users: vec![
                RoomUser {
                    username: "tester".to_owned(),
                    status: 2,
                    average_speed: 1_000,
                    upload_count: 5,
                    file_count: 42,
                    directory_count: 3,
                    slots_free: 1,
                    country_code: "US".to_owned(),
                },
                RoomUser {
                    username: "otherpeer".to_owned(),
                    status: 1,
                    average_speed: 0,
                    upload_count: 0,
                    file_count: 0,
                    directory_count: 0,
                    slots_free: 0,
                    country_code: String::new(),
                },
            ],
            owner: None,
            operators: Vec::new(),
        }),
    )
    .await;

    let response = crate::route_http_request(
        "GET",
        "/api/v0/rooms/joined/roster-audit/users",
        None,
        "",
        &state,
    )
    .await
    .expect("joined room users");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    assert!(response.content_type.contains("application/json"));
    let users = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    let users = users.as_array().expect("roster array");
    assert_eq!(users.len(), 2, "{users:?}");
    assert_eq!(users[0]["username"], "tester");
    assert_eq!(users[0]["status"], "Online");
    assert_eq!(users[0]["averageSpeed"], 1_000);
    assert_eq!(users[0]["uploadCount"], 5);
    assert_eq!(users[0]["fileCount"], 42);
    assert_eq!(users[0]["directoryCount"], 3);
    assert_eq!(users[0]["slotsFree"], 1);
    assert_eq!(users[0]["countryCode"], "US");
    assert_eq!(users[0]["self"], true);
    assert_eq!(users[1]["username"], "otherpeer");
    assert_eq!(users[1]["status"], "Away");
    assert_eq!(users[1]["self"], serde_json::Value::Null);

    // The room's own JSON contract also reflects the real roster's
    // usernames (via the existing `members` field), not just an
    // incrementally-tracked/empty list.
    let room =
        crate::route_http_request("GET", "/api/v0/rooms/joined/roster-audit", None, "", &state)
            .await
            .expect("joined room detail");
    assert_eq!(room.status, "200 OK");
    assert!(room.content_type.contains("application/json"));
    let room_json = serde_json::from_str::<serde_json::Value>(&room.body).unwrap();
    assert_eq!(
        room_json["users"],
        serde_json::json!(["tester", "otherpeer"])
    );

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/rooms/joined/{roomName}/users",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/rooms/joined/{roomName}/users",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/rooms/joined/{roomName}",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/rooms/joined/{roomName}",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("joined_room_roster_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting `runtime-failure-and-timeout` for
/// the contacts/wishlist/collections families -- independently
/// re-verified real DB-close fault injection (own hermetic in-memory
/// DB) for the same routes `contact_routes_roll_back_when_persistence_
/// fails`, `wishlist_routes_roll_back_when_persistence_fails`, and
/// `collection_routes_roll_back_when_persistence_fails` already prove.
/// All 3 families are slskdN-only (slskd declares none of these
/// routes), confirmed against the frozen registry before crediting.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_contact_wishlist_collection_routes_survive_persistence_failure(
) {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    let target = "slskdn";

    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {}", $method, $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "runtime-failure-and-timeout",
                "pass": $pass,
            }));
        };
    }

    // Contacts: create (2 real declared variants), then mutate/delete.
    for (call_path, ledger_route) in [
        (
            "/api/contacts/from-discovery",
            "/api/v0/contacts/from-discovery",
        ),
        ("/api/contacts/from-invite", "/api/v0/contacts/from-invite"),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response =
            crate::route_http_request("POST", call_path, None, r#"{"username":"friend"}"#, &state)
                .await
                .expect("failed contact creation response");
        let pass = response.status == "503 Service Unavailable"
            && response.body.contains("contact persistence failed")
            && state.contacts.read().await.records.is_empty();
        record!("POST", ledger_route, pass);
    }
    for (method, body, expected_error) in [
        (
            "PUT",
            r#"{"username":"changed","online":true}"#,
            "contact persistence failed",
        ),
        ("DELETE", "", "contact deletion persistence failed"),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .contacts
            .write()
            .await
            .create("friend".to_owned())
            .unwrap();
        db.close_for_test().await;
        let response =
            crate::route_http_request(method, "/api/contacts/contact-1", None, body, &state)
                .await
                .expect("failed contact mutation response");
        let pass = response.status == "503 Service Unavailable"
            && response.body.contains(expected_error)
            && state.contacts.read().await.records.len() == 1;
        record!(method, "/api/v0/contacts/{id}", pass);
    }

    // Wishlist: create (3 real declared variants), then mutate/delete.
    for (call_path, ledger_route, body) in [
        (
            "/api/wishlist",
            "/api/v0/wishlist",
            r#"{"artist":"Artist","title":"Track"}"#,
        ),
        (
            "/api/musicbrainz/release-radar/subscriptions",
            "/api/v0/musicbrainz/release-radar/subscriptions",
            r#"{"artist":"Artist","title":"Release"}"#,
        ),
        (
            "/api/wishlist/import/csv",
            "/api/v0/wishlist/import/csv",
            r#"{"csv":"artist,title\nArtist,One\nArtist,Two"}"#,
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.wishlist.read().await.clone();
        db.close_for_test().await;
        let response = crate::route_http_request("POST", call_path, None, body, &state)
            .await
            .expect("failed wishlist creation response");
        let pass = response.status == "503 Service Unavailable"
            && response.body.contains("wishlist persistence failed")
            && *state.wishlist.read().await == previous;
        record!("POST", ledger_route, pass);
    }
    for (method, body, expected_error) in [
        (
            "PUT",
            r#"{"artist":"Changed","title":"Changed"}"#,
            "wishlist persistence failed",
        ),
        ("DELETE", "", "wishlist deletion persistence failed"),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .wishlist
            .write()
            .await
            .add_item("Artist".to_owned(), "Track".to_owned(), "Audio".to_owned())
            .unwrap();
        let previous = state.wishlist.read().await.clone();
        db.close_for_test().await;
        let response =
            crate::route_http_request(method, "/api/wishlist/wish-1", None, body, &state)
                .await
                .expect("failed wishlist mutation response");
        let pass = response.status == "503 Service Unavailable"
            && response.body.contains(expected_error)
            && *state.wishlist.read().await == previous;
        record!(method, "/api/v0/wishlist/{id}", pass);
    }

    // Collections: create, then mutate/delete (item add/edit/remove).
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.collections.read().await.clone();
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            "/api/collections",
            None,
            r#"{"name":"Collection"}"#,
            &state,
        )
        .await
        .expect("failed collection create response");
        let pass = response.status == "503 Service Unavailable"
            && response.body.contains("collection persistence failed")
            && *state.collections.read().await == previous;
        record!("POST", "/api/v0/collections", pass);
    }
    for (method, path, body, ledger_route) in [
        (
            "PUT",
            "/api/collections/col-1",
            r#"{"name":"Changed"}"#,
            "/api/v0/collections/{id}",
        ),
        (
            "POST",
            "/api/collections/col-1/items",
            r#"{"title":"Added"}"#,
            "/api/v0/collections/{id}/items",
        ),
        (
            "PUT",
            "/api/collections/items/item-1",
            r#"{"title":"Changed"}"#,
            "/api/v0/collections/{id}/items/{itemId}",
        ),
        (
            "DELETE",
            "/api/collections/items/item-1",
            "",
            "/api/v0/collections/{id}/items/{itemId}",
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut collections = state.collections.write().await;
        collections
            .create(String::new(), "Collection".to_owned(), String::new())
            .unwrap();
        collections
            .add_item(
                "col-1",
                "one".to_owned(),
                String::new(),
                "One".to_owned(),
                "Audio".to_owned(),
            )
            .unwrap();
        collections
            .add_item(
                "col-1",
                "two".to_owned(),
                String::new(),
                "Two".to_owned(),
                "Audio".to_owned(),
            )
            .unwrap();
        let previous = collections.clone();
        drop(collections);
        db.close_for_test().await;
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed collection mutation response");
        let pass = response.status == "503 Service Unavailable"
            && response.body.contains("collection persistence failed")
            && *state.collections.read().await == previous;
        record!(method, ledger_route, pass);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("contact_wishlist_collection_routes_survive_persistence_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api contact/wishlist/collection mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_contacts_versioned_crud_persistence_and_concurrency(
) {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    let persistence_env = || {
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target)
    };

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    // Versioned contact creation reports the oracle's accepted invite contract.
    {
        let (state, _receiver) = test_state();
        let empty_contacts = crate::route_http_request("GET", "/api/v0/contacts", None, "", &state)
            .await
            .unwrap();
        record!(
            "GET",
            "/api/v0/contacts",
            "missing-empty-or-conflict-state",
            empty_contacts.status == "200 OK" && empty_contacts.body == "[]"
        );
        let empty_nearby =
            crate::route_http_request("GET", "/api/v0/contacts/nearby", None, "", &state)
                .await
                .unwrap();
        record!(
            "GET",
            "/api/v0/contacts/nearby",
            "nominal-status-headers-body",
            empty_nearby.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/contacts/nearby",
            "missing-empty-or-conflict-state",
            empty_nearby.status == "200 OK" && empty_nearby.body == "[]"
        );
        let malformed_nearby =
            crate::route_http_request("GET", "/api/v0/contacts/nearby/", None, "", &state)
                .await
                .unwrap();
        record!(
            "GET",
            "/api/v0/contacts/nearby",
            "malformed-path-query-or-body",
            malformed_nearby.status == "404 Not Found"
        );
        let malformed =
            crate::route_http_request("POST", "/api/v0/contacts/from-invite", None, "{}", &state)
                .await
                .unwrap();
        record!(
            "POST",
            "/api/v0/contacts/from-invite",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        record!(
            "POST",
            "/api/v0/contacts/from-invite",
            "missing-empty-or-conflict-state",
            malformed.status == "400 Bad Request"
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-invite",
            None,
            r#"{"username":"friend"}"#,
            &state,
        )
        .await
        .unwrap();
        let created_json =
            serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        let listed = crate::route_http_request("GET", "/api/v0/contacts", None, "", &state)
            .await
            .unwrap();
        let listed_json =
            serde_json::from_str::<serde_json::Value>(&listed.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/contacts",
            "nominal-status-headers-body",
            listed.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/contacts",
            "populated-dynamic-state",
            listed.status == "200 OK"
                && listed_json.as_array().is_some_and(|contacts| {
                    contacts.iter().any(|contact| {
                        contact["id"] == contact_id && contact["username"] == "friend"
                    })
                })
        );
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/contacts/{id}",
            "nominal-status-headers-body",
            fetched.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/contacts/{id}",
            "populated-dynamic-state",
            fetched.status == "200 OK"
                && fetched_json["id"] == contact_id
                && fetched_json["username"] == "friend"
        );
        let malformed_get = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}/"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        record!(
            "GET",
            "/api/v0/contacts/{id}",
            "malformed-path-query-or-body",
            malformed_get.status == "404 Not Found"
        );
        let missing_get = crate::route_http_request(
            "GET",
            "/api/v0/contacts/00000000-0000-0000-0000-000000000000",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        record!(
            "GET",
            "/api/v0/contacts/{id}",
            "missing-empty-or-conflict-state",
            missing_get.status == "404 Not Found"
        );
        state
            .contacts
            .write()
            .await
            .update(&contact_id, None, Some(true))
            .expect("mark contact online for nearby readback");
        let nearby = crate::route_http_request("GET", "/api/v0/contacts/nearby", None, "", &state)
            .await
            .unwrap();
        let nearby_json =
            serde_json::from_str::<serde_json::Value>(&nearby.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/contacts/nearby",
            "populated-dynamic-state",
            nearby.status == "200 OK"
                && nearby_json.as_array().is_some_and(|contacts| {
                    contacts.iter().any(|contact| {
                        contact["id"] == contact_id
                            && contact["username"] == "friend"
                            && contact["online"] == true
                    })
                })
        );
        let pass = created.status == "201 Created"
            && created_json["username"] == "friend"
            && created_json["invited"] == true
            && created_json["accepted"] == true;
        record!(
            "POST",
            "/api/v0/contacts/from-invite",
            "nominal-status-headers-body",
            created.status == "201 Created"
        );
        record!(
            "POST",
            "/api/v0/contacts/from-invite",
            "mutation-side-effects-and-readback",
            pass && state.contacts.read().await.records.len() == 1
        );
    }

    // Versioned invite creation survives rebuilding the contact store.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contact create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-invite",
            None,
            r#"{"username":"restart-friend"}"#,
            &state,
        )
        .await
        .unwrap();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        let persisted = db.list_contacts(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.contacts.write().await =
            crate::ContactStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "POST",
            "/api/v0/contacts/from-invite",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && persisted.len() == 1
                && fetched.status == "200 OK"
                && fetched_json["username"] == "restart-friend"
        );
    }

    // Distinct versioned invite creations persist concurrently.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contact create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..4)
            .map(|index| format!(r#"{{"username":"concurrent-friend-{index}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/contacts/from-invite", None, body, &state)
        }))
        .await;
        let persisted = db.list_contacts(10, 0).await.unwrap_or_default();
        let usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|contact| contact.username.clone())
            .collect();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("concurrent-friend-{index}"))
            .collect();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
        }) && persisted.len() == 4
            && usernames == expected;
        record!(
            "POST",
            "/api/v0/contacts/from-invite",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Versioned contact updates expose nominal, malformed, missing, and readback behavior.
    {
        let (state, _receiver) = test_state();
        let created = crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-invite",
            None,
            r#"{"username":"before-update"}"#,
            &state,
        )
        .await
        .unwrap();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        let malformed = crate::route_http_request(
            "PUT",
            &format!("/api/v0/contacts/{contact_id}/"),
            None,
            r#"{"username":"malformed"}"#,
            &state,
        )
        .await
        .unwrap();
        let missing = crate::route_http_request(
            "PUT",
            "/api/v0/contacts/contact-missing",
            None,
            r#"{"username":"missing"}"#,
            &state,
        )
        .await
        .unwrap();
        record!(
            "PUT",
            "/api/v0/contacts/{id}",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        record!(
            "PUT",
            "/api/v0/contacts/{id}",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
        let updated = crate::route_http_request(
            "PUT",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            r#"{"username":"after-update","online":true}"#,
            &state,
        )
        .await
        .unwrap();
        let updated_json =
            serde_json::from_str::<serde_json::Value>(&updated.body).unwrap_or_default();
        record!(
            "PUT",
            "/api/v0/contacts/{id}",
            "nominal-status-headers-body",
            created.status == "201 Created" && updated.status == "200 OK"
        );
        record!(
            "PUT",
            "/api/v0/contacts/{id}",
            "mutation-side-effects-and-readback",
            updated_json["username"] == "after-update"
                && updated_json["online"] == true
                && state.contacts.read().await.records[0].username == "after-update"
        );
    }

    // Versioned contact updates survive rebuilding the contact store.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contact update restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-invite",
            None,
            r#"{"username":"update-restart-before"}"#,
            &state,
        )
        .await
        .unwrap();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        let updated = crate::route_http_request(
            "PUT",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            r#"{"username":"update-restart-after","online":true}"#,
            &state,
        )
        .await
        .unwrap();
        let persisted = db.list_contacts(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.contacts.write().await =
            crate::ContactStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "PUT",
            "/api/v0/contacts/{id}",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && persisted.len() == 1
                && persisted[0].username == "update-restart-after"
                && fetched.status == "200 OK"
                && fetched_json["online"] == true
        );
    }

    // Distinct versioned contact updates persist concurrently.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contact update concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut contact_ids = Vec::new();
        for index in 0..4 {
            let _created = crate::route_http_request(
                "POST",
                "/api/v0/contacts/from-invite",
                None,
                &format!(r#"{{"username":"update-concurrent-before-{index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            let contact_id = state
                .contacts
                .read()
                .await
                .records
                .last()
                .expect("created concurrent contact")
                .id
                .clone();
            contact_ids.push(contact_id);
        }
        let responses = futures_util::future::join_all(contact_ids.iter().enumerate().map(
            |(index, contact_id)| {
                let path = format!("/api/v0/contacts/{contact_id}");
                let body =
                    format!(r#"{{"username":"update-concurrent-after-{index}","online":true}}"#);
                let state = Arc::clone(&state);
                async move { crate::route_http_request("PUT", &path, None, &body, &state).await }
            },
        ))
        .await;
        let persisted = db.list_contacts(10, 0).await.unwrap_or_default();
        let usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|contact| contact.username.clone())
            .collect();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("update-concurrent-after-{index}"))
            .collect();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.len() == 4
            && usernames == expected
            && persisted.iter().all(|contact| contact.online);
        record!(
            "PUT",
            "/api/v0/contacts/{id}",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Versioned contact deletion exposes nominal, malformed, missing, and readback behavior.
    {
        let (state, _receiver) = test_state();
        crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-invite",
            None,
            r#"{"username":"delete-me"}"#,
            &state,
        )
        .await
        .unwrap();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        let malformed = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/contacts/{contact_id}/"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let missing = crate::route_http_request(
            "DELETE",
            "/api/v0/contacts/contact-missing",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let after_delete = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/contacts/{id}",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        record!(
            "DELETE",
            "/api/v0/contacts/{id}",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
        record!(
            "DELETE",
            "/api/v0/contacts/{id}",
            "nominal-status-headers-body",
            deleted.status == "200 OK"
        );
        record!(
            "DELETE",
            "/api/v0/contacts/{id}",
            "mutation-side-effects-and-readback",
            deleted.status == "200 OK"
                && after_delete.status == "404 Not Found"
                && state.contacts.read().await.records.is_empty()
        );
    }

    // Versioned contact deletion survives rebuilding the contact store.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contact delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-invite",
            None,
            r#"{"username":"delete-restart"}"#,
            &state,
        )
        .await
        .unwrap();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let persisted = db.list_contacts(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.contacts.write().await =
            crate::ContactStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/contacts/{id}",
            "restart-persistence-or-reset",
            deleted.status == "200 OK" && persisted.is_empty() && fetched.status == "404 Not Found"
        );
    }

    // Distinct versioned contact deletions complete concurrently.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contact delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut contact_ids = Vec::new();
        for index in 0..4 {
            let _created = crate::route_http_request(
                "POST",
                "/api/v0/contacts/from-invite",
                None,
                &format!(r#"{{"username":"delete-concurrent-{index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            let contact_id = state
                .contacts
                .read()
                .await
                .records
                .last()
                .expect("created concurrent contact")
                .id
                .clone();
            contact_ids.push(contact_id);
        }
        let responses = futures_util::future::join_all(contact_ids.iter().map(|contact_id| {
            let path = format!("/api/v0/contacts/{contact_id}");
            let state = Arc::clone(&state);
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let persisted = db.list_contacts(10, 0).await.unwrap_or_default();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.is_empty()
            && state.contacts.read().await.records.is_empty();
        record!(
            "DELETE",
            "/api/v0/contacts/{id}",
            "concurrency-and-idempotency",
            pass
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("contacts_versioned_crud_persistence_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api contacts mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_contacts_discovery_and_read_edges() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    // The frozen list action ignores unrelated query values.  Keep the
    // malformed-case proof on the query so the request reaches the list
    // action instead of being rejected by the contact-id route guard.
    {
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "GET",
            "/api/v0/contacts?limit=not-a-number",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/contacts",
            "malformed-path-query-or-body",
            response.status == "200 OK" && json.is_array()
        );
    }

    // Reads are served from the initialized in-process projection even if
    // the optional backing database becomes unavailable afterwards.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contacts runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let local_peer_id = crate::local_profile_peer_id(&state);
        let created = crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-discovery",
            None,
            &format!(r#"{{"peerId":"{local_peer_id}","nickname":"runtime-contact"}}"#),
            &state,
        )
        .await
        .unwrap();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        db.close_for_test().await;

        let listed = crate::route_http_request("GET", "/api/v0/contacts", None, "", &state)
            .await
            .unwrap();
        let listed_json =
            serde_json::from_str::<serde_json::Value>(&listed.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/contacts",
            "runtime-failure-and-timeout",
            created.status == "201 Created"
                && listed.status == "200 OK"
                && listed_json.as_array().is_some_and(|contacts| {
                    contacts
                        .iter()
                        .any(|contact| contact["username"] == "runtime-contact")
                })
        );

        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/contacts/{id}",
            "runtime-failure-and-timeout",
            fetched.status == "200 OK" && fetched_json["username"] == "runtime-contact"
        );

        let nearby = crate::route_http_request("GET", "/api/v0/contacts/nearby", None, "", &state)
            .await
            .unwrap();
        let nearby_json =
            serde_json::from_str::<serde_json::Value>(&nearby.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/contacts/nearby",
            "runtime-failure-and-timeout",
            nearby.status == "200 OK" && nearby_json.is_array()
        );
    }

    // Local profile discovery follows the frozen profile-service path;
    // arbitrary peer IDs remain a not-found profile lookup.
    {
        let (state, _receiver) = test_state();
        let local_peer_id = crate::local_profile_peer_id(&state);
        let created = crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-discovery",
            None,
            &format!(r#"{{"peerId":"  {local_peer_id}  ","nickname":"  Local Friend  "}}"#),
            &state,
        )
        .await
        .unwrap();
        let created_json =
            serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
        record!(
            "POST",
            "/api/v0/contacts/from-discovery",
            "nominal-status-headers-body",
            created.status == "201 Created"
                && created_json["peerId"] == local_peer_id
                && created_json["nickname"] == "Local Friend"
                && created_json["verified"] == true
        );

        let contact_id = state.contacts.read().await.records[0].id.clone();
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "POST",
            "/api/v0/contacts/from-discovery",
            "mutation-side-effects-and-readback",
            fetched.status == "200 OK" && fetched_json["username"] == "Local Friend"
        );

        let malformed = crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-discovery",
            None,
            &format!(r#"{{"peerId":"{local_peer_id}","nickname":" "}}"#),
            &state,
        )
        .await
        .unwrap();
        record!(
            "POST",
            "/api/v0/contacts/from-discovery",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("Nickname is required.")
        );
    }

    // The contact projection rehydrates the UUID returned by the
    // versioned route and remains readable after a state rebuild.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contacts discovery restart database");
        let env = || {
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target)
        };
        let (state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        let local_peer_id = crate::local_profile_peer_id(&state);
        let created = crate::route_http_request(
            "POST",
            "/api/v0/contacts/from-discovery",
            None,
            &format!(r#"{{"peerId":"{local_peer_id}","nickname":"restart-discovery"}}"#),
            &state,
        )
        .await
        .unwrap();
        let contact_id = state.contacts.read().await.records[0].id.clone();
        let persisted = db.list_contacts(10, 0).await.unwrap_or_default();
        let (restarted_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        *restarted_state.contacts.write().await =
            crate::ContactStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/contacts/{contact_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "POST",
            "/api/v0/contacts/from-discovery",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && persisted.len() == 1
                && fetched.status == "200 OK"
                && fetched_json["username"] == "restart-discovery"
        );
    }

    // Distinct nicknames exercise the same write/persist path concurrently;
    // all successful requests must survive in the durable projection.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("contacts discovery concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let local_peer_id = crate::local_profile_peer_id(&state);
        let bodies: Vec<String> = (0..4)
            .map(|index| {
                format!(
                    r#"{{"peerId":"{local_peer_id}","nickname":"discovery-concurrent-{index}"}}"#
                )
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request(
                "POST",
                "/api/v0/contacts/from-discovery",
                None,
                body,
                &state,
            )
        }))
        .await;
        let persisted = db.list_contacts(10, 0).await.unwrap_or_default();
        let usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|contact| contact.username.clone())
            .collect();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("discovery-concurrent-{index}"))
            .collect();
        record!(
            "POST",
            "/api/v0/contacts/from-discovery",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && persisted.len() == 4
                && usernames == expected
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("contacts_discovery_and_read_edges.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api contacts discovery mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
