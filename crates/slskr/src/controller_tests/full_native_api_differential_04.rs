//! Controller full native api differential 04 ownership.

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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_native_transfers_malformed_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [malformed-path-query-or-body]",
                    $method, $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }};
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    for (path, route) in [
        (
            "/api/v0/transfers/downloads/malformed-peer/not-a-number",
            "/api/v0/transfers/downloads/{username}/{id}",
        ),
        (
            "/api/v0/transfers/uploads/malformed-peer/not-a-number",
            "/api/v0/transfers/uploads/{username}/{id}",
        ),
    ] {
        let response = crate::route_http_request("DELETE", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("DELETE {path}: {error}"));
        record!("DELETE", route, response.status == "400 Bad Request");
    }

    for (path, route) in [
        (
            "/api/v0/transfers/downloads/all/completed/extra",
            "/api/v0/transfers/downloads/all/completed",
        ),
        (
            "/api/v0/transfers/uploads/all/completed/extra",
            "/api/v0/transfers/uploads/all/completed",
        ),
    ] {
        let response = crate::route_http_request("DELETE", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("DELETE {path}: {error}"));
        record!("DELETE", route, response.status == "404 Not Found");
    }

    let checks = [
        (
            "GET",
            "/api/v0/transfers?direction=sideways",
            "/api/v0/transfers",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/changes?since=not-a-number",
            "/api/v0/transfers/changes",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads?includeCompleted=not-a-bool",
            "/api/v0/transfers/downloads",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/malformed-peer/not-a-number",
            "/api/v0/transfers/downloads/{username}/{id}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/%20",
            "/api/v0/transfers/downloads/{username}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/malformed-peer/not-a-number/position",
            "/api/v0/transfers/downloads/{username}/{id}/position",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/accelerated/extra",
            "/api/v0/transfers/downloads/accelerated",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/auto-replace/status/extra",
            "/api/v0/transfers/downloads/auto-replace/status",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/batches/not-a-uuid",
            "/api/v0/transfers/downloads/batches/{id}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/stuck/extra",
            "/api/v0/transfers/downloads/stuck",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/user-stats/extra",
            "/api/v0/transfers/downloads/user-stats",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/history?direction=sideways",
            "/api/v0/transfers/history",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/speeds/extra",
            "/api/v0/transfers/speeds",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads?includeRemoved=not-a-bool",
            "/api/v0/transfers/uploads",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads/malformed-peer/not-a-number",
            "/api/v0/transfers/uploads/{username}/{id}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads/%20",
            "/api/v0/transfers/uploads/{username}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads/diagnostics/extra",
            "/api/v0/transfers/uploads/diagnostics",
            "",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/malformed-peer",
            "/api/v0/transfers/downloads/{username}",
            "not-json",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/auto-replace",
            "/api/v0/transfers/downloads/auto-replace",
            "not-json",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/find-alternative",
            "/api/v0/transfers/downloads/find-alternative",
            "not-json",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/replace",
            "/api/v0/transfers/downloads/replace",
            "not-json",
            "400 Bad Request",
        ),
        (
            "PUT",
            "/api/v0/transfers/downloads/accelerated",
            "/api/v0/transfers/downloads/accelerated",
            "not-json",
            "400 Bad Request",
        ),
    ];
    for (method, path, route, body, expected_status) in checks {
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(method, route, response.status == expected_status);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_malformed_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn transfers malformed ledger"),
    )
    .expect("write slskdn transfers malformed ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers malformed mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_native_transfers_nominal_populated_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind slskdn queue-position fixture");
    let endpoint = listener
        .local_addr()
        .expect("slskdn queue-position fixture address");
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept slskdn queue-position request");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("slskdn queue-position init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("slskdn queue-position request"),
            crate::PeerMessage::PlaceInQueueRequest {
                filename: "Remote/Position.flac".to_owned(),
            }
        );
        peer.send(&crate::PeerMessage::PlaceInQueueResponse {
            filename: "Remote/Position.flac".to_owned(),
            place: 3,
        })
        .await
        .expect("slskdn queue-position response");
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                &format!("position-peer={endpoint}"),
            ),
    );
    state.session.write().await.state = "connected";
    let (position_id, batch_id) = {
        let mut transfers = state.transfers.write().await;
        let position = transfers.create(
            0,
            Some("position-peer".to_owned()),
            "Remote/Position.flac".to_owned(),
            None,
            Some(10),
        );
        let batch_id = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee".to_owned();
        transfers.create_with_batch(
            0,
            Some("batch-peer".to_owned()),
            "Remote/Batch.flac".to_owned(),
            None,
            Some(20),
            Some(batch_id.clone()),
        );
        let failed = transfers.create(
            0,
            Some("failed-peer".to_owned()),
            "Remote/Failed.flac".to_owned(),
            None,
            Some(30),
        );
        transfers.update_status(failed.id, "failed", Some(0), Some("timeout".to_owned()));
        (position.id, batch_id)
    };

    let position = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/position-peer/{position_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn queue-position response");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "nominal-status-headers-body",
        position.status == "200 OK" && position.body == "3"
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "populated-dynamic-state",
        position.status == "200 OK" && position.body == "3"
    );

    let batch = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn populated transfer batch");
    let batch_json =
        serde_json::from_str::<serde_json::Value>(&batch.body).unwrap_or(serde_json::Value::Null);
    let batch_pass = batch.status == "200 OK"
        && batch_json["id"] == batch_id
        && batch_json["transferCount"] == 1
        && batch_json["transfers"]
            .as_array()
            .is_some_and(|rows| rows.len() == 1);
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "nominal-status-headers-body",
        batch_pass
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "populated-dynamic-state",
        batch_pass
    );

    let auto_replace_status = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/auto-replace/status",
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn populated auto-replace status");
    let auto_replace_status_json =
        serde_json::from_str::<serde_json::Value>(&auto_replace_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/downloads/auto-replace/status",
        "populated-dynamic-state",
        auto_replace_status.status == "200 OK"
            && auto_replace_status_json["stuckCount"] == 1
            && auto_replace_status_json["enabled"] == false
            && auto_replace_status_json["intervalSeconds"] == 300
    );

    let auto_replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn nominal auto-replace");
    let auto_json = serde_json::from_str::<serde_json::Value>(&auto_replace.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "nominal-status-headers-body",
        auto_replace.status == "200 OK"
            && auto_json["replaced"] == 0
            && auto_json["failed"] == 0
            && auto_json["skipped"] == 0
            && auto_json["details"].is_array()
    );

    let alternatives = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn nominal find-alternative");
    let alternatives_json = serde_json::from_str::<serde_json::Value>(&alternatives.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "nominal-status-headers-body",
        alternatives.status == "200 OK"
            && alternatives_json.is_array()
            && alternatives_json.as_array().is_some_and(Vec::is_empty)
    );

    let accelerated = crate::route_http_request(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("slskdn nominal accelerated update");
    let accelerated_json = serde_json::from_str::<serde_json::Value>(&accelerated.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "nominal-status-headers-body",
        accelerated.status == "200 OK"
            && accelerated_json["enabled"] == true
            && accelerated_json["policy"].is_string()
            && accelerated_json["updatedAt"].is_string()
    );
    let accelerated_readback = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/accelerated",
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn accelerated readback");
    let accelerated_readback_json =
        serde_json::from_str::<serde_json::Value>(&accelerated_readback.body)
            .unwrap_or(serde_json::Value::Null);
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "mutation-side-effects-and-readback",
        accelerated_readback.status == "200 OK"
            && accelerated_readback_json["enabled"] == true
            && accelerated_readback_json["policy"].is_string()
    );

    server.await.expect("slskdn queue-position fixture task");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_nominal_populated_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn transfers nominal ledger"),
    )
    .expect("write slskdn transfers nominal ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers nominal mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_native_transfers_restart_and_concurrency() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn transfer restart database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with(
            "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
            "restart-peer=127.0.0.1:2234",
        );
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));

    let queued = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/restart-peer",
        None,
        r#"{"files":[{"filename":"Restart/Track.flac","size":42}]}"#,
        &state,
    )
    .await
    .expect("slskdn persisted download");
    let persisted = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read slskdn persisted download");
    let mut rehydrated = crate::TransferQueue::new(&state.config);
    rehydrated.rehydrate_from_database(&db).await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "restart-persistence-or-reset",
        queued.status == "200 OK"
            && persisted
                .iter()
                .any(|entry| entry.filename == "Restart/Track.flac")
            && rehydrated
                .entries
                .iter()
                .any(|entry| entry.filename == "Restart/Track.flac")
    );

    let first_permit = Arc::clone(&state.download_requests)
        .acquire_owned()
        .await
        .expect("first slskdn transfer permit");
    let second_permit = Arc::clone(&state.download_requests)
        .acquire_owned()
        .await
        .expect("second slskdn transfer permit");
    let throttled = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/concurrent-peer",
        None,
        r#"{"files":[{"filename":"Concurrent/Track.flac","size":42}]}"#,
        &state,
    )
    .await
    .expect("slskdn concurrent download");
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "concurrency-and-idempotency",
        throttled.status == "429 Too Many Requests"
            && state.transfers.read().await.entries.len() == 1
    );
    drop(second_permit);
    drop(first_permit);

    let download_id = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.filename == "Restart/Track.flac")
        .map(|entry| entry.id)
        .expect("slskdn persisted download id");
    let cancelled_download = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/restart-peer/{download_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskdn download");
    let repeated_cancelled_download = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/restart-peer/{download_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cancel slskdn download");
    let persisted_cancelled = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read cancelled slskdn download");
    let mut rehydrated_cancelled = crate::TransferQueue::new(&state.config);
    rehydrated_cancelled.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_download.status == "204 No Content"
            && persisted_cancelled.iter().any(|entry| {
                entry.id == download_id.to_string() && entry.status == "cancelled"
            })
            && rehydrated_cancelled
                .entries
                .iter()
                .any(|entry| entry.id == download_id && entry.status == "cancelled")
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "concurrency-and-idempotency",
        cancelled_download.status == "204 No Content"
            && repeated_cancelled_download.status == "204 No Content"
    );

    let upload = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("restart-peer".to_owned()),
            "Restart/Upload.flac".to_owned(),
            None,
            Some(42),
        )
    };
    crate::persist_transfer_record(&state, &upload)
        .await
        .expect("persist slskdn upload");
    let cancelled_upload = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/restart-peer/{}", upload.id),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskdn upload");
    let repeated_cancelled_upload = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/restart-peer/{}", upload.id),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cancel slskdn upload");
    let persisted_upload = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read cancelled slskdn upload");
    let mut rehydrated_upload = crate::TransferQueue::new(&state.config);
    rehydrated_upload.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_upload.status == "204 No Content"
            && persisted_upload
                .iter()
                .any(|entry| { entry.id == upload.id.to_string() && entry.status == "cancelled" })
            && rehydrated_upload
                .entries
                .iter()
                .any(|entry| entry.id == upload.id && entry.status == "cancelled")
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "concurrency-and-idempotency",
        cancelled_upload.status == "204 No Content"
            && repeated_cancelled_upload.status == "204 No Content"
    );

    let completed_download = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("completed-peer".to_owned()),
            "Completed/Download.flac".to_owned(),
            None,
            Some(42),
        );
        transfers
            .update_status(entry.id, "succeeded", Some(42), None)
            .expect("complete slskdn download")
    };
    let completed_upload = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("completed-peer".to_owned()),
            "Completed/Upload.flac".to_owned(),
            None,
            Some(42),
        );
        transfers
            .update_status(entry.id, "completed", Some(42), None)
            .expect("complete slskdn upload")
    };
    crate::persist_transfer_records(
        &state,
        &[completed_download.clone(), completed_upload.clone()],
    )
    .await
    .expect("persist completed slskdn transfers");

    let clear_downloads = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("clear completed slskdn downloads");
    let repeat_clear_downloads = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat clear completed slskdn downloads");
    let after_download_cleanup = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read after slskdn download cleanup");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "restart-persistence-or-reset",
        clear_downloads.status == "204 No Content"
            && repeat_clear_downloads.status == "204 No Content"
            && !after_download_cleanup
                .iter()
                .any(|entry| entry.id == completed_download.id.to_string())
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "concurrency-and-idempotency",
        clear_downloads.status == "204 No Content"
            && repeat_clear_downloads.status == "204 No Content"
    );

    let clear_uploads = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("clear completed slskdn uploads");
    let repeat_clear_uploads = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat clear completed slskdn uploads");
    let after_upload_cleanup = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read after slskdn upload cleanup");
    let mut rehydrated_after_cleanup =
        crate::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated_after_cleanup.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "restart-persistence-or-reset",
        clear_uploads.status == "204 No Content"
            && repeat_clear_uploads.status == "204 No Content"
            && !after_upload_cleanup
                .iter()
                .any(|entry| entry.id == completed_upload.id.to_string())
            && !rehydrated_after_cleanup
                .entries
                .iter()
                .any(|entry| entry.id == completed_upload.id)
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "concurrency-and-idempotency",
        clear_uploads.status == "204 No Content" && repeat_clear_uploads.status == "204 No Content"
    );

    let (reset_state, _reset_receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(db.clone()));
    let auto_replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn auto-replace restart state");
    let reset_auto_replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &reset_state,
    )
    .await
    .expect("reset slskdn auto-replace state");
    let repeated_auto_replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("repeat slskdn auto-replace");
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "restart-persistence-or-reset",
        auto_replace.status == reset_auto_replace.status
            && auto_replace.body == reset_auto_replace.body
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "concurrency-and-idempotency",
        auto_replace.status == "200 OK"
            && repeated_auto_replace.status == "200 OK"
            && auto_replace.body == repeated_auto_replace.body
    );

    let find_alternative = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn find-alternative restart state");
    let reset_find_alternative = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &reset_state,
    )
    .await
    .expect("reset slskdn find-alternative state");
    let repeated_find_alternative = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &state,
    )
    .await
    .expect("repeat slskdn find-alternative");
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "restart-persistence-or-reset",
        find_alternative.status == reset_find_alternative.status
            && find_alternative.body == reset_find_alternative.body
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "concurrency-and-idempotency",
        find_alternative.status == "200 OK"
            && repeated_find_alternative.status == "200 OK"
            && find_alternative.body == repeated_find_alternative.body
    );

    let replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn replace restart state");
    let reset_replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/replace",
        None,
        "{}",
        &reset_state,
    )
    .await
    .expect("reset slskdn replace state");
    let repeated_replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("repeat slskdn replace");
    record!(
        "POST",
        "/api/v0/transfers/downloads/replace",
        "restart-persistence-or-reset",
        replace.status == reset_replace.status && replace.body == reset_replace.body
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/replace",
        "concurrency-and-idempotency",
        replace.status == "500 Internal Server Error"
            && repeated_replace.status == "500 Internal Server Error"
            && replace.body == repeated_replace.body
    );

    let accelerated = crate::route_http_request(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("enable slskdn accelerated downloads");
    let repeated_accelerated = crate::route_http_request(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("repeat enable slskdn accelerated downloads");
    let persisted_runtime = {
        let runtime = state.runtime.read().await;
        let relay = state.relay.read().await;
        runtime.persistence_record(&relay)
    };
    let reset_runtime = crate::RuntimeCompatState::from_persisted(&persisted_runtime);
    let accelerated_json =
        serde_json::from_str::<serde_json::Value>(&accelerated.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "restart-persistence-or-reset",
        accelerated.status == "200 OK" && !reset_runtime.accelerated_downloads_enabled
    );
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "concurrency-and-idempotency",
        accelerated.status == "200 OK"
            && repeated_accelerated.status == "200 OK"
            && accelerated_json["enabled"] == true
            && serde_json::from_str::<serde_json::Value>(&repeated_accelerated.body)
                .ok()
                .is_some_and(|value| value["enabled"] == true)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_restart_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn transfers restart ledger"),
    )
    .expect("write slskdn transfers restart ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers restart/concurrency mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining native slskdn controller cases.
/// The probes use the same local state stores as the handlers, including
/// closed SQLite pools for storage-failure branches and independent state
/// instances for reset behavior.  The v0 nominal and malformed warm-cache
/// cases, plus v0 remediation success, are already credited by earlier
/// ledgers and are intentionally not duplicated here.
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_native_native_open_cases() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
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
                "pass": pass,
            }));
        }};
    }

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let json_value = |response: &crate::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let capabilities_contract = |response: &crate::routing::HttpResponse,
                                 scene_pod_bridge: bool| {
        let value = json_value(response);
        let required_features = [
            "mbid_jobs",
            "discography_jobs",
            "label_crate_jobs",
            "canonical_scoring",
            "rescue_mode",
            "library_health",
            "warm_cache",
            "job_manifests",
            "session_traces",
            "playback_aware",
        ];
        let features = value["features"].as_array();
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["impl"] == "slskdn"
            && value["compat"] == "slskd"
            && features.is_some_and(|features| {
                required_features.iter().all(|feature| {
                    features
                        .iter()
                        .any(|value| value.as_str() == Some(*feature))
                })
            })
            && value["obfuscation"]["supportedConnectionTypes"]
                == serde_json::json!(["P", "D", "F"])
            && value["feature"]["scenePodBridge"] == scene_pod_bridge
            && (!scene_pod_bridge
                || features.is_some_and(|features| {
                    features
                        .iter()
                        .any(|value| value.as_str() == Some("scene_pod_bridge"))
                }))
    };
    let health_contract = |response: &crate::routing::HttpResponse, empty: bool| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["path"] == "(all)"
            && value["summary"]["total_issues"].is_u64()
            && value["summary"]["issues_open"].is_u64()
            && value["summary"]["issues_resolved"].is_u64()
            && value["issues"].is_array()
            && (!empty || value["summary"]["total_issues"] == 0)
    };
    let remediation_contract = |response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value.is_object()
            && (value["id"].as_str().is_some()
                || value["job_id"].as_str().is_some()
                || value["jobId"].as_str().is_some())
    };

    for (path, route) in [
        ("/api/slskdn/capabilities", "/api/slskdn/capabilities"),
        ("/api/v0/slskdn/capabilities", "/api/v0/slskdn/capabilities"),
    ] {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed_path = format!("{path}/extra");
        let malformed = crate::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {malformed_path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            capabilities_contract(&empty, false)
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("capabilities runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path} with closed database: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            capabilities_contract(&runtime, false)
        );

        if path.starts_with("/api/v0/") {
            let (populated_state, _receiver) = test_state_with_env(target_env());
            populated_state
                .media_services
                .write()
                .await
                .features
                .scene_pod_bridge = true;
            let populated = crate::route_http_request("GET", path, None, "", &populated_state)
                .await
                .unwrap_or_else(|error| panic!("GET {path} populated: {error}"));
            record!(
                "GET",
                route,
                "populated-dynamic-state",
                capabilities_contract(&populated, true)
            );
        }
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("native library-health runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime =
            crate::route_http_request("GET", "/api/slskdn/library/health", None, "", &state)
                .await
                .expect("unversioned native library-health runtime response");
        record!(
            "GET",
            "/api/slskdn/library/health",
            "runtime-failure-and-timeout",
            health_contract(&runtime, true)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing =
            crate::route_http_request("GET", "/api/v0/slskdn/library/health", None, "", &state)
                .await
                .expect("versioned native library-health empty response");
        record!(
            "GET",
            "/api/v0/slskdn/library/health",
            "missing-empty-or-conflict-state",
            health_contract(&missing, true)
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned native library-health runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request(
            "GET",
            "/api/v0/slskdn/library/health",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("versioned native library-health runtime response");
        record!(
            "GET",
            "/api/v0/slskdn/library/health",
            "runtime-failure-and-timeout",
            health_contract(&runtime, true)
        );
    }

    for path in ["/api/slskdn/library/remediate"] {
        let (state, _receiver) = test_state_with_env(target_env());
        let issue_id = seed_kindless_library_item(&state, "Native remediation").await;
        let success = crate::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [issue_id]}).to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            remediation_contract(&success)
        );
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            remediation_contract(&success) && state.library.read().await.health_issues().is_empty()
        );

        let malformed_state = test_state_with_env(target_env()).0;
        let malformed = crate::route_http_request("POST", path, None, "not-json", &malformed_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} malformed: {error}"));
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = crate::route_http_request("POST", path, None, "{}", &malformed_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("native remediation runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        let runtime_issue = seed_kindless_library_item(&runtime_state, "Runtime remediation").await;
        db.close_for_test().await;
        let runtime = crate::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [runtime_issue]}).to_string(),
            &runtime_state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path} runtime: {error}"));
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
                && runtime_state.library.read().await.health_issues().len() == 1
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset_issue = seed_kindless_library_item(&reset_state, "Reset remediation").await;
        let reset = crate::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [reset_issue]}).to_string(),
            &reset_state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path} reset: {error}"));
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request(
            "GET",
            "/api/slskdn/library/health",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("native remediation reset health response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            remediation_contract(&reset) && health_contract(&restarted, true)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let left_issue = seed_kindless_library_item(&concurrent_state, "Concurrent left").await;
        let right_issue = seed_kindless_library_item(&concurrent_state, "Concurrent right").await;
        let left_body = serde_json::json!({"issue_ids": [left_issue]}).to_string();
        let right_body = serde_json::json!({"issue_ids": [right_issue]}).to_string();
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", path, None, &left_body, &concurrent_state),
            crate::route_http_request("POST", path, None, &right_body, &concurrent_state)
        );
        let left = left.expect("left native remediation concurrency response");
        let right = right.expect("right native remediation concurrency response");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            remediation_contract(&left)
                && remediation_contract(&right)
                && concurrent_state
                    .library
                    .read()
                    .await
                    .health_issues()
                    .is_empty()
        );
    }

    for path in ["/api/v0/slskdn/library/remediate"] {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request("POST", path, None, "not-json", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} malformed: {error}"));
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = crate::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned native remediation runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        let runtime_issue =
            seed_kindless_library_item(&runtime_state, "Versioned runtime remediation").await;
        db.close_for_test().await;
        let runtime = crate::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [runtime_issue]}).to_string(),
            &runtime_state,
        )
        .await
        .expect("versioned native remediation runtime response");
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
                && runtime_state.library.read().await.health_issues().len() == 1
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset_issue =
            seed_kindless_library_item(&reset_state, "Versioned reset remediation").await;
        let reset = crate::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [reset_issue]}).to_string(),
            &reset_state,
        )
        .await
        .expect("versioned native remediation reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request(
            "GET",
            "/api/v0/slskdn/library/health",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("versioned native remediation reset health response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            remediation_contract(&reset) && health_contract(&restarted, true)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let left_issue =
            seed_kindless_library_item(&concurrent_state, "Versioned concurrent left").await;
        let right_issue =
            seed_kindless_library_item(&concurrent_state, "Versioned concurrent right").await;
        let left_body = serde_json::json!({"issue_ids": [left_issue]}).to_string();
        let right_body = serde_json::json!({"issue_ids": [right_issue]}).to_string();
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", path, None, &left_body, &concurrent_state),
            crate::route_http_request("POST", path, None, &right_body, &concurrent_state)
        );
        let left = left.expect("left versioned remediation concurrency response");
        let right = right.expect("right versioned remediation concurrency response");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            remediation_contract(&left)
                && remediation_contract(&right)
                && concurrent_state
                    .library
                    .read()
                    .await
                    .health_issues()
                    .is_empty()
        );
    }

    let write_warm_cache_config = |state: &Arc<crate::AppState>| {
        fs::write(
            state.config.state_dir.join("slskd.yml"),
            "warmCache:\n  enabled: true\n",
        )
        .expect("write native warm-cache config");
    };
    let warm_body = r#"{"mb_release_ids":[" rel-native ","REL-NATIVE"],"mb_artist_ids":["artist-native"],"mb_label_ids":[]}"#;
    let warm_success = |response: &crate::routing::HttpResponse| {
        response.status == "200 OK" && response.body == r#"{"accepted":true}"#
    };

    for path in ["/api/slskdn/warm-cache/hints"] {
        let (disabled_state, _receiver) = test_state_with_env(target_env());
        let missing = crate::route_http_request("POST", path, None, "{}", &disabled_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let (warm_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&warm_state);
        let accepted = crate::route_http_request("POST", path, None, warm_body, &warm_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} nominal: {error}"));
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            warm_success(&accepted)
        );
        let features = warm_state.controller_features.read().await;
        let popularity_pass = features
            .get("warm-cache/popularity/mb:release:rel-native")
            .is_some_and(|value| value["hits"] == 1)
            && features
                .get("warm-cache/popularity/mb:artist:artist-native")
                .is_some_and(|value| value["hits"] == 1);
        drop(features);
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            popularity_pass
        );

        let malformed = crate::route_http_request(
            "POST",
            path,
            None,
            r#"{"mb_release_ids":[42]}"#,
            &warm_state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path} malformed: {error}"));
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("native warm-cache runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        write_warm_cache_config(&runtime_state);
        db.close_for_test().await;
        let runtime = crate::route_http_request("POST", path, None, warm_body, &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} runtime: {error}"));
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            warm_success(&runtime)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&reset_state);
        let reset = crate::route_http_request("POST", path, None, warm_body, &reset_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} reset: {error}"));
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request("POST", path, None, warm_body, &restarted_state)
            .await
            .expect("native warm-cache reset response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            warm_success(&reset)
                && restarted.status == "400 Bad Request"
                && restarted.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&concurrent_state);
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", path, None, warm_body, &concurrent_state),
            crate::route_http_request("POST", path, None, warm_body, &concurrent_state)
        );
        let left = left.expect("left native warm-cache concurrency response");
        let right = right.expect("right native warm-cache concurrency response");
        let features = concurrent_state.controller_features.read().await;
        let concurrent_hits = features
            .get("warm-cache/popularity/mb:release:rel-native")
            .and_then(|value| value["hits"].as_u64())
            == Some(2);
        drop(features);
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            warm_success(&left) && warm_success(&right) && concurrent_hits
        );
    }

    for path in ["/api/v0/slskdn/warm-cache/hints"] {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = crate::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned native warm-cache runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        write_warm_cache_config(&runtime_state);
        db.close_for_test().await;
        let runtime = crate::route_http_request("POST", path, None, warm_body, &runtime_state)
            .await
            .expect("versioned native warm-cache runtime response");
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            warm_success(&runtime)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&reset_state);
        let reset = crate::route_http_request("POST", path, None, warm_body, &reset_state)
            .await
            .expect("versioned native warm-cache reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request("POST", path, None, warm_body, &restarted_state)
            .await
            .expect("versioned native warm-cache restarted response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            warm_success(&reset)
                && restarted.status == "400 Bad Request"
                && restarted.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&concurrent_state);
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", path, None, warm_body, &concurrent_state),
            crate::route_http_request("POST", path, None, warm_body, &concurrent_state)
        );
        let left = left.expect("left versioned warm-cache concurrency response");
        let right = right.expect("right versioned warm-cache concurrency response");
        let features = concurrent_state.controller_features.read().await;
        let concurrent_hits = features
            .get("warm-cache/popularity/mb:release:rel-native")
            .and_then(|value| value["hits"].as_u64())
            == Some(2);
        drop(features);
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            warm_success(&left) && warm_success(&right) && concurrent_hits
        );
    }

    assert_eq!(ledger.len(), 33, "slskdn native residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create slskdn native evidence directory");
    fs::write(
        evidence_dir.join("native_native_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn native ledger"),
    )
    .expect("write slskdn native ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn native controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
