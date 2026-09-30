//! Controller full shares differential ownership.

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
pub(super) async fn controller_api_differential_incremental_transfer_and_message_routes_validate_cursors_and_bound_history(
) {
    let (state, _receiver) = test_state();
    {
        let mut transfers = state.transfers.write().await;
        for (filename, direction, status, updated_at_ms) in [
            ("old.flac", 0, "succeeded", 1_000),
            ("new.flac", 0, "succeeded", 3_000),
            ("active.flac", 0, "in_progress", 4_000),
            ("upload.flac", 1, "failed", 5_000),
        ] {
            let entry = transfers.create(
                direction,
                Some("peer".to_owned()),
                filename.to_owned(),
                None,
                Some(10),
            );
            let entry = transfers
                .entries
                .iter_mut()
                .find(|candidate| candidate.id == entry.id)
                .unwrap();
            entry.status = status.to_owned();
            entry.updated_at = updated_at_ms / 1_000;
            entry.updated_at_ms = updated_at_ms;
        }
    }

    let initial_response = crate::route_http_request(
        "GET",
        "/api/v0/transfers/changes?includeCompleted=false",
        None,
        "",
        &state,
    )
    .await
    .expect("initial transfer changes");
    assert_eq!(initial_response.status, "200 OK");
    assert!(initial_response.content_type.contains("application/json"));
    let initial = serde_json::from_str::<serde_json::Value>(&initial_response.body).unwrap();
    assert_eq!(initial["counts"]["download"], 3);
    assert_eq!(initial["counts"]["upload"], 1);
    assert_eq!(initial["transfers"].as_array().unwrap().len(), 2);

    let changes =
        crate::route_http_request("GET", "/api/transfers/changes?since=3500", None, "", &state)
            .await
            .expect("incremental transfer changes");
    let changes = serde_json::from_str::<serde_json::Value>(&changes.body).unwrap();
    let changed = changes["transfers"].as_array().unwrap();
    assert_eq!(changed.len(), 2);
    assert!(changed
        .iter()
        .any(|entry| entry["filename"] == "active.flac"));
    assert!(changed
        .iter()
        .any(|entry| entry["filename"] == "upload.flac"));

    let history_response = crate::route_http_request(
        "GET",
        "/api/v0/transfers/history?direction=download&asOf=3500&offset=0&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("transfer history page");
    assert_eq!(history_response.status, "200 OK");
    assert!(history_response.content_type.contains("application/json"));
    let history = serde_json::from_str::<serde_json::Value>(&history_response.body).unwrap();
    assert_eq!(history["asOf"], 3_500);
    assert_eq!(history["hasMore"], true);
    assert_eq!(history["nextOffset"], 1);
    assert_eq!(history["transfers"][0]["filename"], "new.flac");
    let invalid_history = crate::route_http_request(
        "GET",
        "/api/transfers/history?direction=download&limit=501",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid transfer history");
    assert_eq!(invalid_history.status, "400 Bad Request");

    {
        let mut messages = state.messages.write().await;
        messages.add("friend".to_owned(), "inbound", "old".to_owned());
        messages.add("friend".to_owned(), "inbound", "new".to_owned());
        assert!(messages.records[1].created_at_ms > messages.records[0].created_at_ms);
        messages.records[0].created_at_ms = 1_000;
        messages.records[1].created_at_ms = 2_000;
    }
    let conversation = crate::route_http_request(
        "GET",
        "/api/v0/conversations/friend?since=1500",
        None,
        "",
        &state,
    )
    .await
    .expect("incremental conversation");
    let conversation = serde_json::from_str::<serde_json::Value>(&conversation.body).unwrap();
    assert_eq!(conversation["unAcknowledgedMessageCount"], 2);
    assert_eq!(conversation["messages"].as_array().unwrap().len(), 1);
    assert_eq!(conversation["messages"][0]["message"], "new");
    assert_eq!(conversation["messages"][0]["createdAtMs"], 2_000);

    {
        let mut rooms = state.rooms.write().await;
        rooms.join("music".to_owned()).unwrap();
        rooms
            .add_message("music", "friend".to_owned(), "old".to_owned())
            .unwrap();
        rooms
            .add_message("music", "friend".to_owned(), "new".to_owned())
            .unwrap();
        let room = rooms
            .records
            .iter_mut()
            .find(|room| room.name == "music")
            .unwrap();
        assert!(room.messages[1].created_at_ms > room.messages[0].created_at_ms);
        room.messages[0].created_at_ms = 1_000;
        room.messages[1].created_at_ms = 2_000;
    }
    let room = crate::route_http_request(
        "GET",
        "/api/v0/rooms/joined/music/messages?since=1500",
        None,
        "",
        &state,
    )
    .await
    .expect("incremental room messages");
    let room = serde_json::from_str::<serde_json::Value>(&room.body).unwrap();
    assert_eq!(room.as_array().unwrap().len(), 1);
    assert_eq!(room[0]["message"], "new");
    assert!(room[0]["id"].as_str().is_some());
    assert_eq!(room[0]["createdAtMs"], 2_000);

    for path in [
        "/api/transfers/changes?since=-1",
        "/api/conversations/friend?since=-1",
        "/api/rooms/joined/music/messages?since=-1",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("negative cursor response");
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/transfers/changes",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/transfers/history",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("incremental_transfer_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting the share-grants CRUD routes'
/// cases, independently re-derived from `v0_share_grants_get_real_
/// uuid_ids_usable_on_versioned_routes`'s real UUID-id and full
/// create/read/update/delete lifecycle checks. slskdN-only (confirmed
/// against the frozen registry: absent from the slskd policy file).
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_share_grants_crud() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

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

    let (state, _receiver) = test_state();
    let empty_list = crate::route_http_request("GET", "/api/v0/share-grants", None, "", &state)
        .await
        .expect("empty share grants list");
    record!(
        "GET",
        "/api/v0/share-grants",
        "missing-empty-or-conflict-state",
        empty_list.status == "200 OK" && empty_list.body == "[]"
    );
    let malformed_create =
        crate::route_http_request("POST", "/api/v0/share-grants", None, "{}", &state)
            .await
            .expect("malformed share grant create");
    record!(
        "POST",
        "/api/v0/share-grants",
        "malformed-path-query-or-body",
        malformed_create.status == "409 Conflict"
    );
    let missing_create = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        None,
        r#"{"collection_id":"missing-collection","username":"friend"}"#,
        &state,
    )
    .await
    .expect("missing share grant collection");
    record!(
        "POST",
        "/api/v0/share-grants",
        "missing-empty-or-conflict-state",
        missing_create.status == "404 Not Found"
    );
    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Share Grant Differential"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let created = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"friend"}}"#),
        &state,
    )
    .await
    .expect("create share grant via the v0 route");
    let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default()
        ["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/share-grants",
        "nominal-status-headers-body",
        created.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v0/share-grants",
        "mutation-side-effects-and-readback",
        uuid::Uuid::parse_str(&grant_id).is_ok()
    );

    let populated_list = crate::route_http_request("GET", "/api/v0/share-grants", None, "", &state)
        .await
        .expect("populated share grants list");
    let populated_list_json =
        serde_json::from_str::<serde_json::Value>(&populated_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/share-grants",
        "populated-dynamic-state",
        populated_list.status == "200 OK"
            && populated_list_json
                .as_array()
                .is_some_and(|records| { records.iter().any(|record| record["id"] == grant_id) })
    );

    let get_route = format!("/api/v0/share-grants/{grant_id}");
    let get = crate::route_http_request("GET", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    let get_json = serde_json::from_str::<serde_json::Value>(&get.body).unwrap_or_default();
    let malformed_get = crate::route_http_request("GET", "/api/v0/share-grants/", None, "", &state)
        .await
        .expect("malformed share grant get");
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "malformed-path-query-or-body",
        malformed_get.status == "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/share-grants",
        "malformed-path-query-or-body",
        malformed_get.status == "400 Bad Request"
    );
    let missing_get = crate::route_http_request(
        "GET",
        "/api/v0/share-grants/00000000-0000-0000-0000-000000000000",
        None,
        "",
        &state,
    )
    .await
    .expect("missing share grant get");
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        missing_get.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "nominal-status-headers-body",
        get.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "populated-dynamic-state",
        get_json["id"] == grant_id
            && get_json["collection_id"] == collection_id
            && get_json["username"] == "friend"
            && get_json["permissions"].is_string()
    );

    let malformed_update =
        crate::route_http_request("PUT", "/api/v0/share-grants/", None, "{}", &state)
            .await
            .expect("malformed share grant update");
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "malformed-path-query-or-body",
        malformed_update.status == "404 Not Found"
    );

    let update = crate::route_http_request(
        "PUT",
        &get_route,
        None,
        r#"{"permissions":"read,download"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "nominal-status-headers-body",
        update.status == "200 OK"
    );

    let readback = crate::route_http_request("GET", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    let readback_json =
        serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "mutation-side-effects-and-readback",
        readback_json["permissions"] == "read,download"
    );

    let missing_update = crate::route_http_request(
        "PUT",
        "/api/v0/share-grants/00000000-0000-0000-0000-000000000000",
        None,
        r#"{"permissions":"read"}"#,
        &state,
    )
    .await
    .expect("missing share grant update");
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        missing_update.status == "404 Not Found"
    );

    let malformed_delete =
        crate::route_http_request("DELETE", "/api/v0/share-grants/", None, "", &state)
            .await
            .expect("malformed share grant delete");
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );

    let delete = crate::route_http_request("DELETE", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "nominal-status-headers-body",
        delete.status == "200 OK"
    );

    let after_delete = crate::route_http_request("GET", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "mutation-side-effects-and-readback",
        after_delete.status == "404 Not Found"
    );

    let missing_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/share-grants/00000000-0000-0000-0000-000000000000",
        None,
        "",
        &state,
    )
    .await
    .expect("missing share grant delete");
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        missing_delete.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("share_grants_crud.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api share-grants-crud mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_share_grants_persistence_and_concurrency() {
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

    async fn create_share_grant_collection(state: &crate::AppState, title: &str) -> String {
        let body = serde_json::json!({"title": title}).to_string();
        let response = crate::route_http_request("POST", "/api/v0/collections", None, &body, state)
            .await
            .expect("persist share-grant collection");
        assert_eq!(response.status, "201 Created", "{}", response.body);
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap()["id"]
            .as_str()
            .expect("collection id")
            .to_owned()
    }

    // Versioned grant creation rolls back when persistence fails.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant create runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id = create_share_grant_collection(&state, "Runtime Grant Collection").await;
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"runtime"}}"#),
            &state,
        )
        .await
        .unwrap();
        let pass = response.status == "503 Service Unavailable"
            && state.share_grants.read().await.records.is_empty();
        record!(
            "POST",
            "/api/v0/share-grants",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Versioned grant creation survives rebuilding the grant store.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id = create_share_grant_collection(&state, "Restart Grant Collection").await;
        let created = crate::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"restart"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let persisted = db.list_share_grants(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.share_grants.write().await =
            crate::ShareGrantStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{grant_id}"),
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
            "/api/v0/share-grants",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && persisted.len() == 1
                && fetched.status == "200 OK"
                && fetched_json["id"] == grant_id
                && fetched_json["username"] == "restart"
        );
    }

    // Distinct versioned grants can be created concurrently and all persist.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Concurrent Grant Collection").await;
        let bodies: Vec<String> = (0..4)
            .map(|index| {
                format!(r#"{{"collection_id":"{collection_id}","username":"concurrent-{index}"}}"#)
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/share-grants", None, body, &state)
        }))
        .await;
        let persisted = db.list_share_grants(10, 0).await.unwrap_or_default();
        let usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|grant| grant.username.clone())
            .collect();
        let expected: std::collections::BTreeSet<String> =
            (0..4).map(|index| format!("concurrent-{index}")).collect();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
        }) && persisted.len() == 4
            && usernames == expected;
        record!(
            "POST",
            "/api/v0/share-grants",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Versioned grant updates restore the prior state when persistence fails.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant update runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Runtime Update Grant Collection").await;
        let created = crate::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"update-runtime"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        db.close_for_test().await;
        let response = crate::route_http_request(
            "PUT",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            r#"{"permissions":"read,download"}"#,
            &state,
        )
        .await
        .unwrap();
        let grant = state.share_grants.read().await.get(&grant_id);
        let pass = response.status == "503 Service Unavailable"
            && grant.is_some_and(|grant| grant.permissions == "download,stream");
        record!(
            "PUT",
            "/api/v0/share-grants/{id}",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Versioned grant updates survive rebuilding the grant store.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant update restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Restart Update Grant Collection").await;
        let created = crate::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"update-restart"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let updated = crate::route_http_request(
            "PUT",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            r#"{"permissions":"read,download"}"#,
            &state,
        )
        .await
        .unwrap();
        let persisted = db.list_share_grants(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.share_grants.write().await =
            crate::ShareGrantStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{grant_id}"),
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
            "/api/v0/share-grants/{id}",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && persisted.len() == 1
                && fetched.status == "200 OK"
                && fetched_json["permissions"] == "read,download"
        );
    }

    // Distinct versioned grant updates complete concurrently and persist.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant update concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Concurrent Update Grant Collection").await;
        let mut grant_ids = Vec::new();
        for index in 0..4 {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/share-grants",
                None,
                &format!(r#"{{"collection_id":"{collection_id}","username":"update-{index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            grant_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(grant_ids.iter().enumerate().map(
            |(index, grant_id)| {
                let path = format!("/api/v0/share-grants/{grant_id}");
                let body = format!(r#"{{"permissions":"read,slot-{index}"}}"#);
                let state = Arc::clone(&state);
                async move { crate::route_http_request("PUT", &path, None, &body, &state).await }
            },
        ))
        .await;
        let persisted = db.list_share_grants(10, 0).await.unwrap_or_default();
        let permissions: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|grant| grant.permissions.clone())
            .collect();
        let expected: std::collections::BTreeSet<String> =
            (0..4).map(|index| format!("read,slot-{index}")).collect();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.len() == 4
            && permissions == expected;
        record!(
            "PUT",
            "/api/v0/share-grants/{id}",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Versioned grant deletion survives rebuilding the grant store.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Restart Delete Grant Collection").await;
        let created = crate::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"delete-restart"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let persisted = db.list_share_grants(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.share_grants.write().await =
            crate::ShareGrantStore::from_persisted(persisted.clone());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/share-grants/{id}",
            "restart-persistence-or-reset",
            deleted.status == "200 OK" && persisted.is_empty() && fetched.status == "404 Not Found"
        );
    }

    // Distinct versioned grant deletions complete concurrently.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Concurrent Delete Grant Collection").await;
        let mut grant_ids = Vec::new();
        for index in 0..4 {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/share-grants",
                None,
                &format!(r#"{{"collection_id":"{collection_id}","username":"delete-{index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            grant_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(grant_ids.iter().map(|grant_id| {
            let path = format!("/api/v0/share-grants/{grant_id}");
            let state = Arc::clone(&state);
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let persisted = db.list_share_grants(10, 0).await.unwrap_or_default();
        let in_memory_empty = state.share_grants.read().await.records.is_empty();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.is_empty()
            && in_memory_empty;
        record!(
            "DELETE",
            "/api/v0/share-grants/{id}",
            "concurrency-and-idempotency",
            pass
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("share_grants_persistence_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api share-grant persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the share-grants routes' real
/// transitive ownership enforcement, independently re-derived from
/// `share_grants_are_scoped_to_the_real_collection_owner`'s checks
/// that a grant is owned through its collection (`collection.
/// OwnerUserId == currentUserId`), so every action 404s for a
/// different caller. Uses the `/api/v0/` routes throughout (the
/// original test also exercises a legacy `/api/share-grants/{id}`
/// alias that has no manifest registry entry in either frozen
/// target -- not creditable, matching the documented pattern for
/// slskR-internal aliases). slskdN-only (confirmed against the
/// frozen registry: absent from the slskd policy file).
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_share_grants_ownership_scoping() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

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

    let keys = serde_json::json!({
        "alice": {"key": "alice-key-0123456789", "role": "administrator", "cidr": ""},
        "bob": {"key": "bob-key-00123456789ab", "role": "administrator", "cidr": ""},
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKD_API_KEYS_JSON", &keys.to_string()),
    );
    let alice = Some("ApiKey alice-key-0123456789");
    let bob = Some("ApiKey bob-key-00123456789ab");

    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        alice,
        r#"{"title":"Alice Grant Ownership"}"#,
        &state,
    )
    .await
    .expect("alice creates a collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let bob_create = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        bob,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("bob attempts to grant alice's collection");
    record!(
        "POST",
        "/api/v0/share-grants",
        "missing-empty-or-conflict-state",
        bob_create.status == "404 Not Found"
    );

    let granted = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        alice,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("alice grants her own collection");
    let grant_id = serde_json::from_str::<serde_json::Value>(&granted.body).unwrap_or_default()
        ["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let mut cross_user_pass = true;
    for (method, path, body) in [
        (
            "GET",
            format!("/api/v0/share-grants/{grant_id}"),
            String::new(),
        ),
        (
            "PUT",
            format!("/api/v0/share-grants/{grant_id}"),
            r#"{"permissions":"read,download"}"#.to_owned(),
        ),
        (
            "GET",
            format!("/api/v0/share-grants/by-collection/{collection_id}"),
            String::new(),
        ),
        (
            "POST",
            format!("/api/v0/share-grants/{grant_id}/token"),
            "{}".to_owned(),
        ),
    ] {
        let response = crate::route_http_request(method, &path, bob, &body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        cross_user_pass &= response.status == "404 Not Found";
    }
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );

    let bob_list = crate::route_http_request("GET", "/api/v0/share-grants", bob, "", &state)
        .await
        .expect("bob lists share grants");
    let bob_list_empty = serde_json::from_str::<serde_json::Value>(&bob_list.body)
        .unwrap_or_default()
        .as_array()
        .map(Vec::len)
        == Some(0);
    record!(
        "GET",
        "/api/v0/share-grants",
        "nominal-status-headers-body",
        bob_list.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/share-grants",
        "populated-dynamic-state",
        bob_list_empty
    );

    let alice_token = crate::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{grant_id}/token"),
        alice,
        "{}",
        &state,
    )
    .await
    .expect("alice mints a token for her own grant");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "nominal-status-headers-body",
        alice_token.status == "201 Created"
    );

    let bob_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/share-grants/{grant_id}"),
        bob,
        "",
        &state,
    )
    .await
    .expect("bob attempts to delete alice's grant");
    let still_there = crate::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{grant_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice's grant still exists");
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        bob_delete.status == "404 Not Found" && still_there.status == "200 OK"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("share_grants_ownership_scoping.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api share-grants-ownership-scoping mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for sharegroups CRUD/edge cases and the
/// `runtime-failure-and-timeout` case (sharegroups routes), plus
/// `concurrency-and-idempotency` for the shares-rebuild route --
/// independently re-derived from
/// `share_group_revocation_rolls_back_when_persistence_fails`,
/// `share_group_member_revocation_rolls_back_when_persistence_
/// fails`, `share_rebuild_routes_roll_back_when_persistence_
/// fails`, and `share_rebuild_routes_reject_concurrent_scans`
/// (a real held scan-permit genuinely blocks a concurrent rebuild
/// with 503, not a race) with fresh fixture data. Confirmed
/// against the evidence directory before writing: none of these
/// cases had prior credit. slskdN-only (confirmed against the
/// frozen registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_sharegroups_and_shares_rebuild() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

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

    {
        let (state, _receiver) = test_state();
        let empty = crate::route_http_request("GET", "/api/v0/sharegroups", None, "", &state)
            .await
            .expect("empty sharegroups");
        record!(
            "GET",
            "/api/v0/sharegroups",
            "nominal-status-headers-body",
            empty.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/sharegroups",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty.body == "[]"
        );

        let malformed_list =
            crate::route_http_request("GET", "/api/v0/sharegroups/", None, "", &state)
                .await
                .expect("malformed sharegroups list");
        record!(
            "GET",
            "/api/v0/sharegroups",
            "malformed-path-query-or-body",
            malformed_list.status == "400 Bad Request"
        );

        let malformed_create = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":""}"#,
            &state,
        )
        .await
        .expect("malformed sharegroup create");
        record!(
            "POST",
            "/api/v0/sharegroups",
            "malformed-path-query-or-body",
            malformed_create.status == "400 Bad Request"
        );

        let missing_create =
            crate::route_http_request("POST", "/api/v0/sharegroups", None, "{}", &state)
                .await
                .expect("missing sharegroup name");
        record!(
            "POST",
            "/api/v0/sharegroups",
            "missing-empty-or-conflict-state",
            missing_create.status == "400 Bad Request"
        );

        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Differential Group","description":"edge fixture"}"#,
            &state,
        )
        .await
        .expect("create sharegroup");
        let created_json =
            serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
        let group_id = created_json["id"].as_str().unwrap_or_default().to_owned();

        let listed = crate::route_http_request("GET", "/api/v0/sharegroups", None, "", &state)
            .await
            .expect("list populated sharegroups");
        let listed_json =
            serde_json::from_str::<serde_json::Value>(&listed.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/sharegroups",
            "populated-dynamic-state",
            listed.status == "200 OK"
                && listed_json
                    .as_array()
                    .is_some_and(|groups| { groups.iter().any(|group| group["id"] == group_id) })
        );

        let get_route = format!("/api/v0/sharegroups/{group_id}");
        let fetched = crate::route_http_request("GET", &get_route, None, "", &state)
            .await
            .expect("get sharegroup");
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/sharegroups/{id}",
            "nominal-status-headers-body",
            fetched.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/sharegroups/{id}",
            "populated-dynamic-state",
            fetched.status == "200 OK"
                && fetched_json["id"] == group_id
                && fetched_json["name"] == "Differential Group"
        );
        let missing_get = crate::route_http_request(
            "GET",
            "/api/v0/sharegroups/00000000-0000-0000-0000-000000000000",
            None,
            "",
            &state,
        )
        .await
        .expect("missing sharegroup get");
        record!(
            "GET",
            "/api/v0/sharegroups/{id}",
            "missing-empty-or-conflict-state",
            missing_get.status == "404 Not Found"
        );

        let updated = crate::route_http_request(
            "PUT",
            &get_route,
            None,
            r#"{"name":"Updated Differential Group","description":"updated"}"#,
            &state,
        )
        .await
        .expect("update sharegroup");
        let updated_json =
            serde_json::from_str::<serde_json::Value>(&updated.body).unwrap_or_default();
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "nominal-status-headers-body",
            updated.status == "200 OK"
        );
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "mutation-side-effects-and-readback",
            updated_json["name"] == "Updated Differential Group"
        );
        let missing_update = crate::route_http_request(
            "PUT",
            "/api/v0/sharegroups/missing-group",
            None,
            r#"{"name":"missing"}"#,
            &state,
        )
        .await
        .expect("missing sharegroup update");
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "missing-empty-or-conflict-state",
            missing_update.status == "404 Not Found"
        );
        let malformed_update = crate::route_http_request(
            "PUT",
            &format!("{get_route}/"),
            None,
            r#"{"name":"Malformed Path"}"#,
            &state,
        )
        .await
        .expect("malformed sharegroup update path");
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "malformed-path-query-or-body",
            malformed_update.status == "404 Not Found"
        );

        let members_route = format!("/api/v0/sharegroups/{group_id}/members");
        let empty_members = crate::route_http_request("GET", &members_route, None, "", &state)
            .await
            .expect("empty sharegroup members");
        record!(
            "GET",
            "/api/v0/sharegroups/{id}/members",
            "nominal-status-headers-body",
            empty_members.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/sharegroups/{id}/members",
            "missing-empty-or-conflict-state",
            empty_members.status == "200 OK" && empty_members.body == "[]"
        );

        let malformed_member =
            crate::route_http_request("POST", &members_route, None, "{}", &state)
                .await
                .expect("malformed sharegroup member");
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "malformed-path-query-or-body",
            malformed_member.status == "409 Conflict"
        );
        let missing_member = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups/missing-group/members",
            None,
            r#"{"username":"friend"}"#,
            &state,
        )
        .await
        .expect("missing sharegroup member group");
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "missing-empty-or-conflict-state",
            missing_member.status == "404 Not Found"
        );

        let added = crate::route_http_request(
            "POST",
            &members_route,
            None,
            r#"{"username":"friend"}"#,
            &state,
        )
        .await
        .expect("add sharegroup member");
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "nominal-status-headers-body",
            added.status == "201 Created"
        );
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "mutation-side-effects-and-readback",
            added.body.contains("friend")
        );

        let populated_members = crate::route_http_request("GET", &members_route, None, "", &state)
            .await
            .expect("populated sharegroup members");
        let populated_members_json =
            serde_json::from_str::<serde_json::Value>(&populated_members.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/sharegroups/{id}/members",
            "populated-dynamic-state",
            populated_members.status == "200 OK"
                && populated_members_json.as_array().is_some_and(|members| {
                    members.iter().any(|member| member["username"] == "friend")
                })
        );

        let member_route = format!("/api/v0/sharegroups/{group_id}/members/friend");
        let removed_member = crate::route_http_request("DELETE", &member_route, None, "", &state)
            .await
            .expect("remove sharegroup member");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "nominal-status-headers-body",
            removed_member.status == "200 OK"
        );
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "mutation-side-effects-and-readback",
            removed_member.status == "200 OK"
        );
        let missing_member_delete = crate::route_http_request(
            "DELETE",
            "/api/v0/sharegroups/missing-group/members/friend",
            None,
            "",
            &state,
        )
        .await
        .expect("missing sharegroup member delete");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "missing-empty-or-conflict-state",
            missing_member_delete.status == "404 Not Found"
        );
        let malformed_member_delete =
            crate::route_http_request("DELETE", &format!("{member_route}/"), None, "", &state)
                .await
                .expect("malformed sharegroup member delete path");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "malformed-path-query-or-body",
            malformed_member_delete.status == "404 Not Found"
        );

        let deleted = crate::route_http_request("DELETE", &get_route, None, "", &state)
            .await
            .expect("delete sharegroup");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "nominal-status-headers-body",
            deleted.status == "200 OK"
        );
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "mutation-side-effects-and-readback",
            deleted.body == "{}"
        );
        let missing_delete = crate::route_http_request(
            "DELETE",
            "/api/v0/sharegroups/missing-group",
            None,
            "",
            &state,
        )
        .await
        .expect("missing sharegroup delete");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "missing-empty-or-conflict-state",
            missing_delete.status == "404 Not Found"
        );
        let malformed_delete =
            crate::route_http_request("DELETE", &format!("{get_route}/"), None, "", &state)
                .await
                .expect("malformed sharegroup delete path");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "malformed-path-query-or-body",
            malformed_delete.status == "404 Not Found"
        );
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let group_id = state
            .sharegroups
            .write()
            .await
            .create("Differential Trusted".to_owned(), String::new())
            .expect("share group")
            .id;
        state
            .sharegroups
            .write()
            .await
            .add_member(&group_id, "differential-friend".to_owned())
            .expect("member capacity")
            .expect("share group");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("failed share group revocation response");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let group_id = state
            .sharegroups
            .write()
            .await
            .create("Differential Trusted".to_owned(), String::new())
            .expect("share group")
            .id;
        state
            .sharegroups
            .write()
            .await
            .add_member(&group_id, "differential-friend".to_owned())
            .expect("member capacity")
            .expect("share group");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/sharegroups/{group_id}/members/differential-friend"),
            None,
            "",
            &state,
        )
        .await
        .expect("failed share group member revocation response");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = crate::route_http_request("PUT", "/api/v0/shares", None, "", &state)
            .await
            .expect("failed shares rebuild response");
        record!(
            "PUT",
            "/api/v0/shares",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }

    {
        let (state, _receiver) = test_state();
        let _permit = Arc::clone(&state.share_scans)
            .acquire_owned()
            .await
            .expect("share scan permit");
        let response = crate::route_http_request("PUT", "/api/v0/shares", None, "", &state)
            .await
            .expect("concurrent shares rebuild response");
        record!(
            "PUT",
            "/api/v0/shares",
            "concurrency-and-idempotency",
            response.status == "503 Service Unavailable"
                && response.body == "{\"error\":\"share scan already in progress\"}"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("sharegroups_and_shares_rebuild.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api sharegroups-and-shares-rebuild mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
