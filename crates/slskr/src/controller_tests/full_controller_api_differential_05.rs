//! Controller full controller api differential 05 ownership.

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
pub(super) async fn controller_api_differential_controller_upload_lifecycle() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let upload_id = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("slskd-upload-peer".to_owned()),
            "Uploads/Parity.flac".to_owned(),
            None,
            Some(50),
        );
        transfers
            .update_status(entry.id, "in_progress", Some(17), None)
            .expect("mark slskd upload active")
            .id
    };

    let uploads = crate::route_http_request("GET", "/api/v0/transfers/uploads", None, "", &state)
        .await
        .expect("slskd upload list");
    let uploads_json = serde_json::from_str::<serde_json::Value>(&uploads.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/uploads",
        "nominal-status-headers-body",
        uploads.status == "200 OK" && uploads_json.is_array()
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads",
        "populated-dynamic-state",
        uploads.status == "200 OK"
            && uploads_json.as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["username"] == "slskd-upload-peer"
                        && row["directories"].as_array().is_some_and(|directories| {
                            directories.iter().any(|directory| {
                                directory["files"].as_array().is_some_and(|files| {
                                    files
                                        .iter()
                                        .any(|file| file["filename"] == "Uploads/Parity.flac")
                                })
                            })
                        })
                })
            })
    );

    let user_uploads = crate::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/slskd-upload-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd user uploads");
    let user_uploads_json =
        serde_json::from_str::<serde_json::Value>(&user_uploads.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "nominal-status-headers-body",
        user_uploads.status == "200 OK" && user_uploads_json["username"] == "slskd-upload-peer"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "populated-dynamic-state",
        user_uploads.status == "200 OK"
            && user_uploads_json["directories"][0]["files"][0]["filename"] == "Uploads/Parity.flac"
    );

    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/uploads/slskd-upload-peer/{upload_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd upload detail");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "nominal-status-headers-body",
        detail.status == "200 OK"
            && detail_json["filename"] == "Uploads/Parity.flac"
            && detail_json["direction"] == "Upload"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "populated-dynamic-state",
        detail.status == "200 OK"
            && detail_json["id"] == upload_id
            && detail_json["bytesTransferred"] == 17
    );

    let missing_user = crate::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/no-such-user",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd upload user");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "missing-empty-or-conflict-state",
        missing_user.status == "404 Not Found"
    );

    let missing_detail = crate::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/slskd-upload-peer/999999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd upload detail");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "missing-empty-or-conflict-state",
        missing_detail.status == "404 Not Found"
    );

    let malformed_detail = crate::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/slskd-upload-peer/not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd upload detail");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "malformed-path-query-or-body",
        malformed_detail.status == "400 Bad Request"
    );

    let cancelled = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/slskd-upload-peer/{upload_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskd upload");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content"
            && state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .find(|entry| entry.id == upload_id)
                .is_some_and(|entry| entry.status == "cancelled")
    );

    let pruned = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("prune slskd uploads");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "nominal-status-headers-body",
        pruned.status == "204 No Content" && pruned.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "mutation-side-effects-and-readback",
        pruned.status == "204 No Content"
            && state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .all(|entry| entry.id != upload_id)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_upload_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd upload ledger"),
    )
    .expect("write slskd upload ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd upload controller mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_transfer_failure_restart_and_idempotency(
) {
    let target = "slskd";
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

    let empty_state =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target)).0;
    for (path, route) in [
        ("/api/v0/transfers/downloads", "/api/v0/transfers/downloads"),
        ("/api/v0/transfers/uploads", "/api/v0/transfers/uploads"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &empty_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_default()
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    let malformed_download = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        "{}",
        &empty_state,
    )
    .await
    .expect("malformed slskd download request");
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "malformed-path-query-or-body",
        malformed_download.status == "400 Bad Request"
    );

    // Frozen slskd admits two legacy enqueue operations at a time and
    // rejects the next one before touching transfer state. Hold both
    // permits to exercise the same deterministic 429 boundary.
    let first_download_permit = Arc::clone(&empty_state.download_requests)
        .acquire_owned()
        .await
        .expect("first legacy download limiter permit");
    let second_download_permit = Arc::clone(&empty_state.download_requests)
        .acquire_owned()
        .await
        .expect("second legacy download limiter permit");
    let throttled_download = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/concurrent-peer",
        None,
        r#"{"files":[{"filename":"Concurrent/Track.flac","size":42}]}"#,
        &empty_state,
    )
    .await
    .expect("throttled slskd download request");
    let state_unchanged = empty_state.transfers.read().await.entries.is_empty();
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "concurrency-and-idempotency",
        throttled_download.status == "429 Too Many Requests" && state_unchanged
    );
    drop(second_download_permit);
    drop(first_download_permit);

    let malformed_batch = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        "{}",
        &empty_state,
    )
    .await
    .expect("malformed slskd download batch");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "malformed-path-query-or-body",
        malformed_batch.status == "400 Bad Request"
    );

    // Both direction-specific cancel routes return the same stable
    // missing-resource contract and can be safely repeated for an
    // already-cancelled transfer.
    for (direction, route) in [
        ("downloads", "/api/v0/transfers/downloads/{username}/{id}"),
        ("uploads", "/api/v0/transfers/uploads/{username}/{id}"),
    ] {
        let missing = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/missing-peer/999999"),
            None,
            "",
            &empty_state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE missing {direction}: {error}"));
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }

    for (direction, route) in [
        ("downloads", "/api/v0/transfers/downloads/all/completed"),
        ("uploads", "/api/v0/transfers/uploads/all/completed"),
    ] {
        let first = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/all/completed"),
            None,
            "",
            &empty_state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE empty {direction}: {error}"));
        let second = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/all/completed"),
            None,
            "",
            &empty_state,
        )
        .await
        .unwrap_or_else(|error| panic!("repeat DELETE empty {direction}: {error}"));
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            first.status == "204 No Content" && second.status == "204 No Content"
        );
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            first.status == "204 No Content" && second.status == "204 No Content"
        );
    }

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskd transfer differential database");
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
        r#"{"files":[{"filename":"Persist/Track.flac","size":42}]}"#,
        &state,
    )
    .await
    .expect("persist slskd queued transfer");
    let persisted = db
        .list_transfers(None, 10, 0)
        .await
        .expect("read persisted slskd transfer");
    let mut rehydrated = crate::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated.rehydrate_from_database(&db).await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "restart-persistence-or-reset",
        queued.status == "200 OK"
            && persisted
                .iter()
                .any(|entry| entry.filename == "Persist/Track.flac")
            && rehydrated
                .entries
                .iter()
                .any(|entry| entry.filename == "Persist/Track.flac")
    );

    let transfer_id = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.filename == "Persist/Track.flac")
        .map(|entry| entry.id)
        .expect("persisted transfer id");
    for (direction, route) in [
        ("downloads", "/api/v0/transfers/downloads/{username}/{id}"),
        ("uploads", "/api/v0/transfers/uploads/{username}/{id}"),
    ] {
        let (username, id) = if direction == "downloads" {
            ("restart-peer", transfer_id)
        } else {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(
                1,
                Some("restart-peer".to_owned()),
                "Persist/Upload.flac".to_owned(),
                None,
                Some(42),
            );
            ("restart-peer", entry.id)
        };
        let first = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/{username}/{id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE {direction} transfer: {error}"));
        let second = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/{username}/{id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("repeat DELETE {direction} transfer: {error}"));
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            first.status == "204 No Content" && second.status == "204 No Content"
        );
    }

    // A failed durable write must not leave a transfer in the in-memory
    // queue.  This is the real rollback contract for the slskd enqueue
    // route, not just an assertion about the returned error.
    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskd transfer failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    let failed = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/failure-peer",
        None,
        r#"{"files":[{"filename":"Failure/Track.flac","size":42}]}"#,
        &failure_state,
    )
    .await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "runtime-failure-and-timeout",
        failed.is_ok_and(|response| response.status == "503 Service Unavailable")
            && failure_state.transfers.read().await.entries.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_transfer_failure_restart_and_idempotency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd transfer failure ledger"),
    )
    .expect("write slskd transfer failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd transfer failure/restart mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_transfer_batch_cleanup_and_failures() {
    let target = "slskd";
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

    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with(
            "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
            "batch-peer=127.0.0.1:2234",
        );
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("transfer batch database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
    let batch_id = "55555555-5555-4555-8555-555555555555";
    let batch_body = format!(
        r#"{{"id":"{batch_id}","username":"batch-peer","files":[{{"filename":"Batch/One.flac","size":101}},{{"filename":"Batch/Two.flac","size":202}}]}}"#
    );
    let batch = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("create transfer batch");
    let persisted_batch_transfers = db
        .list_transfers(None, 20, 0)
        .await
        .expect("list batch transfers");
    let mut rehydrated = crate::TransferQueue::new(&state.config);
    rehydrated.rehydrate_from_database(&db).await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "restart-persistence-or-reset",
        batch.status == "201 Created"
            && persisted_batch_transfers.len() == 2
            && persisted_batch_transfers
                .iter()
                .all(|entry| entry.batch_id.as_deref() == Some(batch_id))
            && rehydrated
                .entries
                .iter()
                .filter(|entry| entry.batch_id.as_deref() == Some(batch_id))
                .count()
                == 2
    );
    let _ = receiver.try_recv();
    let duplicate_batch = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("duplicate transfer batch");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "concurrency-and-idempotency",
        duplicate_batch.status == "409 Conflict"
    );

    let batch_entry_ids = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .filter(|entry| entry.batch_id.as_deref() == Some(batch_id))
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    let cancelled_download = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/downloads/batch-peer/{}",
            batch_entry_ids[0]
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel persisted batch download");
    let persisted_cancelled_download = db
        .get_transfer(&batch_entry_ids[0].to_string())
        .await
        .expect("read cancelled batch download")
        .expect("cancelled batch download row");
    let mut rehydrated_cancelled =
        crate::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated_cancelled.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_download.status == "204 No Content"
            && persisted_cancelled_download.status == "cancelled"
            && rehydrated_cancelled
                .entries
                .iter()
                .any(|entry| entry.id == batch_entry_ids[0] && entry.status == "cancelled")
    );
    let repeated_cancelled_download = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/downloads/batch-peer/{}",
            batch_entry_ids[0]
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat batch download cancellation");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "concurrency-and-idempotency",
        repeated_cancelled_download.status == "204 No Content"
    );

    let upload_entry = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("upload-peer".to_owned()),
            "Upload/Track.flac".to_owned(),
            None,
            Some(303),
        )
    };
    crate::persist_transfer_record(&state, &upload_entry)
        .await
        .expect("persist upload entry");
    let cancelled_upload = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/upload-peer/{}", upload_entry.id),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel persisted upload");
    let persisted_cancelled_upload = db
        .get_transfer(&upload_entry.id.to_string())
        .await
        .expect("read cancelled upload")
        .expect("cancelled upload row");
    let mut rehydrated_upload =
        crate::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated_upload.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_upload.status == "204 No Content"
            && persisted_cancelled_upload.status == "cancelled"
            && rehydrated_upload
                .entries
                .iter()
                .any(|entry| entry.id == upload_entry.id && entry.status == "cancelled")
    );
    let repeated_cancelled_upload = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/upload-peer/{}", upload_entry.id),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat upload cancellation");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "concurrency-and-idempotency",
        repeated_cancelled_upload.status == "204 No Content"
    );

    let completed_download = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("cleanup-peer".to_owned()),
            "Cleanup.flac".to_owned(),
            None,
            Some(404),
        )
    };
    let completed_upload = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("cleanup-peer".to_owned()),
            "Cleanup-upload.flac".to_owned(),
            None,
            Some(405),
        )
    };
    for entry in [&completed_download, &completed_upload] {
        let completed = state
            .transfers
            .write()
            .await
            .update_status(entry.id, "completed", entry.size, None)
            .expect("mark cleanup transfer completed");
        crate::persist_transfer_record(&state, &completed)
            .await
            .expect("persist cleanup transfer");
    }
    let cleanup_download = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup completed downloads");
    let persisted_after_download_cleanup = db
        .get_transfer(&completed_download.id.to_string())
        .await
        .expect("read cleaned download");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "restart-persistence-or-reset",
        cleanup_download.status == "204 No Content" && persisted_after_download_cleanup.is_none()
    );
    let repeated_cleanup_download = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cleanup completed downloads");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "concurrency-and-idempotency",
        repeated_cleanup_download.status == "204 No Content"
    );
    let cleanup_upload = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup completed uploads");
    let persisted_after_upload_cleanup = db
        .get_transfer(&completed_upload.id.to_string())
        .await
        .expect("read cleaned upload");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "restart-persistence-or-reset",
        cleanup_upload.status == "204 No Content" && persisted_after_upload_cleanup.is_none()
    );
    let repeated_cleanup_upload = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cleanup completed uploads");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "concurrency-and-idempotency",
        repeated_cleanup_upload.status == "204 No Content"
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("batch failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_batch = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        r#"{"id":"66666666-6666-4666-8666-666666666666","username":"batch-peer","files":[{"filename":"Failure/Batch.flac","size":1}]}"#,
        &failure_state,
    )
    .await
    .expect("batch persistence failure response");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "runtime-failure-and-timeout",
        failed_batch.status == "503 Service Unavailable"
            && failure_state.transfers.read().await.entries.is_empty()
            && failure_state
                .controller_features
                .read()
                .await
                .get("slskd/transfer-batch/66666666-6666-4666-8666-666666666666")
                .is_none()
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("download cancellation failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_download = {
        let mut transfers = failure_state.transfers.write().await;
        transfers.create(
            0,
            Some("failure-peer".to_owned()),
            "Failure/Download.flac".to_owned(),
            None,
            Some(1),
        )
    };
    failure_db.close_for_test().await;
    let failed_download_cancel = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/downloads/failure-peer/{}",
            failure_download.id
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("download cancellation failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "runtime-failure-and-timeout",
        failed_download_cancel.status == "503 Service Unavailable"
            && failure_state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .any(|entry| entry.id == failure_download.id && entry.status == "queued")
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("upload cancellation failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_upload = {
        let mut transfers = failure_state.transfers.write().await;
        transfers.create(
            1,
            Some("failure-peer".to_owned()),
            "Failure/Upload.flac".to_owned(),
            None,
            Some(1),
        )
    };
    failure_db.close_for_test().await;
    let failed_upload_cancel = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/uploads/failure-peer/{}",
            failure_upload.id
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("upload cancellation failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "runtime-failure-and-timeout",
        failed_upload_cancel.status == "503 Service Unavailable"
            && failure_state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .any(|entry| entry.id == failure_upload.id && entry.status == "queued")
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("download cleanup failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    {
        let mut transfers = failure_state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("failure-peer".to_owned()),
            "Failure/Cleanup.flac".to_owned(),
            None,
            Some(1),
        );
        transfers
            .update_status(entry.id, "completed", entry.size, None)
            .expect("seed completed download cleanup");
    }
    let previous_cleanup_entries = failure_state.transfers.read().await.entries.clone();
    failure_db.close_for_test().await;
    let failed_cleanup = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("download cleanup failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "runtime-failure-and-timeout",
        failed_cleanup.status == "503 Service Unavailable"
            && failure_state.transfers.read().await.entries == previous_cleanup_entries
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("upload cleanup failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(failure_db.clone()));
    {
        let mut transfers = failure_state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("failure-peer".to_owned()),
            "Failure/UploadCleanup.flac".to_owned(),
            None,
            Some(1),
        );
        transfers
            .update_status(entry.id, "completed", entry.size, None)
            .expect("seed completed upload cleanup");
    }
    let previous_upload_cleanup_entries = failure_state.transfers.read().await.entries.clone();
    failure_db.close_for_test().await;
    let failed_upload_cleanup = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("upload cleanup failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "runtime-failure-and-timeout",
        failed_upload_cleanup.status == "503 Service Unavailable"
            && failure_state.transfers.read().await.entries == previous_upload_cleanup_entries
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_transfer_batch_cleanup_and_failures.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd transfer batch ledger"),
    )
    .expect("write slskd transfer batch ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd transfer batch/failure mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_user_browse_contracts() {
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let missing_browse =
        crate::route_http_request("GET", "/api/v0/users/browse-peer/browse", None, "", &state)
            .await
            .expect("missing slskd browse");
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "missing-empty-or-conflict-state",
        missing_browse.status == "500 Internal Server Error"
    );
    let missing_status = crate::route_http_request(
        "GET",
        "/api/v0/users/browse-peer/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd browse status");
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "missing-empty-or-conflict-state",
        missing_status.status == "404 Not Found"
    );

    let ingested = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        r#"{"username":"browse-peer","entries":[{"filename":"Remote/Album/Track.flac","size":123}]}"#,
        &state,
    )
    .await
    .expect("seed slskd browse");
    assert_eq!(ingested.status, "200 OK");
    state.session.write().await.state = "connected";

    let browse =
        crate::route_http_request("GET", "/api/v0/users/browse-peer/browse", None, "", &state)
            .await
            .expect("slskd browse root");
    let browse_json = serde_json::from_str::<serde_json::Value>(&browse.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "nominal-status-headers-body",
        browse.status == "200 OK"
            && browse_json["directoryCount"] == 1
            && browse_json["directories"].is_array()
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "populated-dynamic-state",
        browse.status == "200 OK"
            && browse_json["directories"][0]["name"] == "Remote/Album"
            && browse_json["directories"][0]["files"][0]["filename"] == "Track.flac"
    );

    let status = crate::route_http_request(
        "GET",
        "/api/v0/users/browse-peer/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd browse status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "nominal-status-headers-body",
        status.status == "200 OK" && status_json["status"] == "ready"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "populated-dynamic-state",
        status.status == "200 OK"
            && status_json["isComplete"] == true
            && status_json["fileCount"] == 1
            && status_json["bytesTransferred"] == 123
    );

    let directory = crate::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &state,
    )
    .await
    .expect("slskd browse directory");
    let directory_json =
        serde_json::from_str::<serde_json::Value>(&directory.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "nominal-status-headers-body",
        directory.status == "200 OK" && directory_json.is_array()
    );
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "mutation-side-effects-and-readback",
        directory.status == "200 OK"
            && directory_json[0]["name"] == "Remote/Album"
            && directory_json[0]["files"][0]["filename"] == "Remote/Album/Track.flac"
    );

    let malformed_directory = crate::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        "{}",
        &state,
    )
    .await
    .expect("malformed slskd browse directory");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "malformed-path-query-or-body",
        malformed_directory.status == "400 Bad Request"
    );

    let nominal_command = receiver.try_recv().ok();
    let (failure_state, failure_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    failure_state.session.write().await.state = "connected";
    drop(failure_receiver);
    let failed_directory = crate::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &failure_state,
    )
    .await
    .expect("slskd directory session failure");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "runtime-failure-and-timeout",
        failed_directory.status == "503 Service Unavailable"
            && failure_state.browse.read().await.records.is_empty()
    );

    let (reset_state, mut reset_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    reset_state.session.write().await.state = "connected";
    let reset_directory = crate::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &reset_state,
    )
    .await
    .expect("slskd directory fresh-state reset");
    let reset_json =
        serde_json::from_str::<serde_json::Value>(&reset_directory.body).unwrap_or_default();
    let reset_command = reset_receiver.try_recv().ok();
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "restart-persistence-or-reset",
        reset_directory.status == "200 OK"
            && reset_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["name"] == "Remote/Album"
                    && rows[0]["files"].as_array().is_some_and(Vec::is_empty)
                    && rows[0]["totalBytes"] == 0
            })
            && matches!(
                reset_command,
                Some(crate::SessionCommand::BrowseFolder { username, folder })
                    if username == "browse-peer" && folder == "Remote/Album"
            )
    );

    while receiver.try_recv().is_ok() {}
    let concurrent_directories = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/users/browse-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/users/browse-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
    ])
    .await;
    let concurrent_commands = [receiver.try_recv().ok(), receiver.try_recv().ok()];
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "concurrency-and-idempotency",
        nominal_command.is_some_and(|command| {
            matches!(
                command,
                crate::SessionCommand::BrowseFolder { username, folder }
                    if username == "browse-peer" && folder == "Remote/Album"
            )
        }) && concurrent_directories.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_commands.iter().all(|command| {
            command.as_ref().is_some_and(|command| {
                matches!(
                    command,
                    crate::SessionCommand::BrowseFolder { username, folder }
                        if username == "browse-peer" && folder == "Remote/Album"
                )
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_user_browse_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd browse ledger"),
    )
    .expect("write slskd browse ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd browse controller mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_download_edge_contracts() {
    let target = "slskd";
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

    let batch_id = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let (_active_id, completed_id) = {
        let mut transfers = state.transfers.write().await;
        let active = transfers.create_with_details(
            0,
            Some("download-edge-peer".to_owned()),
            "Remote/Active.flac".to_owned(),
            None,
            Some(100),
            Some(batch_id.to_owned()),
            crate::TransferRequestDetails {
                request_name: Some("Active".to_owned()),
                ..crate::TransferRequestDetails::default()
            },
        );
        let active = transfers
            .update_status(active.id, "in_progress", Some(25), None)
            .expect("active slskd download");
        let completed = transfers.create(
            0,
            Some("download-edge-peer".to_owned()),
            "Remote/Completed.flac".to_owned(),
            None,
            Some(20),
        );
        let completed = transfers
            .update_status(completed.id, "succeeded", Some(20), None)
            .expect("completed slskd download");
        (active.id, completed.id)
    };

    let user = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/download-edge-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd download user projection");
    let user_json = serde_json::from_str::<serde_json::Value>(&user.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "nominal-status-headers-body",
        user.status == "200 OK"
            && user_json["username"] == "download-edge-peer"
            && user_json["directories"].is_array()
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "populated-dynamic-state",
        user.status == "200 OK"
            && user_json["directories"]
                .as_array()
                .is_some_and(|directories| {
                    directories.iter().any(|directory| {
                        directory["files"].as_array().is_some_and(|files| {
                            files
                                .iter()
                                .any(|file| file["filename"] == "Remote/Active.flac")
                        })
                    })
                })
    );

    let missing_user = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/no-such-download-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd download user projection");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "missing-empty-or-conflict-state",
        missing_user.status == "404 Not Found"
    );

    let malformed_detail = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/download-edge-peer/not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd download detail");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "malformed-path-query-or-body",
        malformed_detail.status == "400 Bad Request"
    );

    let batch = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd download batch projection");
    let batch_json = serde_json::from_str::<serde_json::Value>(&batch.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "nominal-status-headers-body",
        batch.status == "200 OK"
            && batch_json["id"] == batch_id
            && batch_json["transfers"].is_array()
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "populated-dynamic-state",
        batch.status == "200 OK"
            && batch_json["transferCount"] == 1
            && batch_json["transfers"][0]["filename"] == "Remote/Active.flac"
    );

    let missing_batch = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/ffffffff-ffff-4fff-8fff-ffffffffffff",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd download batch");
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "missing-empty-or-conflict-state",
        missing_batch.status == "404 Not Found"
    );

    let malformed_batch = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/not-a-guid",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd download batch");
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "malformed-path-query-or-body",
        malformed_batch.status == "400 Bad Request"
    );

    let malformed_enqueue = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/download-edge-peer",
        None,
        "{}",
        &state,
    )
    .await
    .expect("malformed slskd download enqueue");
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "malformed-path-query-or-body",
        malformed_enqueue.status == "400 Bad Request"
    );

    let malformed_cancel = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/download-edge-peer/not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd download cancellation");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "malformed-path-query-or-body",
        malformed_cancel.status == "400 Bad Request"
    );

    let cleared = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("clear completed slskd downloads");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "nominal-status-headers-body",
        cleared.status == "204 No Content" && cleared.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "mutation-side-effects-and-readback",
        cleared.status == "204 No Content"
            && state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .all(|entry| entry.id != completed_id)
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind slskd queue position fixture");
    let endpoint = listener
        .local_addr()
        .expect("slskd queue position fixture address");
    let server = tokio::spawn(async move {
        for (expected_filename, expected_place) in
            [("Remote/Position.flac", 3_u32), ("Remote/Done.flac", 6_u32)]
        {
            let (stream, _) = listener
                .accept()
                .await
                .expect("accept slskd queue position");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            assert_eq!(
                init.receive().await.expect("slskd queue position init"),
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "tester".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("slskd queue position request"),
                crate::PeerMessage::PlaceInQueueRequest {
                    filename: expected_filename.to_owned(),
                }
            );
            peer.send(&crate::PeerMessage::PlaceInQueueResponse {
                filename: expected_filename.to_owned(),
                place: expected_place,
            })
            .await
            .expect("slskd queue position response");
        }
    });
    let (position_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                &format!("position-peer={endpoint}"),
            ),
    );
    position_state.session.write().await.state = "connected";
    let (position_id, completed_position_id) = {
        let mut transfers = position_state.transfers.write().await;
        let active = transfers.create(
            0,
            Some("position-peer".to_owned()),
            "Remote/Position.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(active.id, "in_progress", Some(2), None);
        let completed = transfers.create(
            0,
            Some("position-peer".to_owned()),
            "Remote/Done.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(completed.id, "succeeded", Some(10), None);
        (active.id, completed.id)
    };
    let position = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/position-peer/{position_id}/position"),
        None,
        "",
        &position_state,
    )
    .await
    .expect("slskd active queue position");
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
    let completed_position = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/position-peer/{completed_position_id}/position"),
        None,
        "",
        &position_state,
    )
    .await
    .expect("slskd completed queue position");
    assert_eq!(completed_position.status, "200 OK");
    assert_eq!(completed_position.body, "6");
    let missing_position = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/position-peer/999999/position",
        None,
        "",
        &position_state,
    )
    .await
    .expect("missing slskd queue position");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "missing-empty-or-conflict-state",
        missing_position.status == "404 Not Found"
    );
    let malformed_position = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/position-peer/not-a-number/position",
        None,
        "",
        &position_state,
    )
    .await
    .expect("malformed slskd queue position");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "malformed-path-query-or-body",
        malformed_position.status == "400 Bad Request"
    );
    server.await.expect("slskd queue position fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_download_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd download ledger"),
    )
    .expect("write slskd download ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd download edge mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_runtime_failure_isolation_contracts() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [runtime-failure-and-timeout]",
                    $method, $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "runtime-failure-and-timeout",
                "pass": pass,
            }));
        }};
    }

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskd runtime-failure database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with("SLSKR_REMOTE_CONFIGURATION", "true")
        .with("SLSKR_DEBUG", "true")
        .with("SLSKR_NO_CONFIG_WATCH", "true")
        .with("SLSKD_USERNAME", "slskd")
        .with("SLSKD_PASSWORD", "runtime-login-secret");
    let (state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(db.clone()));

    {
        let mut shares = state.shares.write().await;
        shares.roots.push(crate::ShareRoot {
            label: "Virtual".to_owned(),
            local_path: PathBuf::from("/srv/music"),
            raw: "Virtual".to_owned(),
            directories: 1,
            files: 1,
            bytes: 42,
            extensions: Vec::new(),
            statistics_ready: true,
        });
    }
    state
        .rooms
        .write()
        .await
        .join("runtime-room".to_owned())
        .expect("runtime room fixture");
    state
        .browse
        .write()
        .await
        .request("runtime-browse-peer".to_owned())
        .expect("runtime browse fixture");
    let (download_id, upload_id) = {
        let mut transfers = state.transfers.write().await;
        let download = transfers.create(
            0,
            Some("runtime-download-peer".to_owned()),
            "Remote/Runtime.flac".to_owned(),
            None,
            Some(42),
        );
        let upload = transfers.create(
            1,
            Some("runtime-upload-peer".to_owned()),
            "Remote/Upload.flac".to_owned(),
            None,
            Some(24),
        );
        (download.id, upload.id)
    };
    let share_id = crate::share_root_id("Virtual");
    let _scan_permit = Arc::clone(&state.share_scans)
        .acquire_owned()
        .await
        .expect("hold share scan for runtime cancellation");

    // A closed transfer database is the injected runtime failure.  The
    // frozen controllers keep the synchronous projections alive, while
    // transfer detail/queue reads surface the service failure.
    db.close_for_test().await;

    let cancel_scan = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd share cancellation under transfer-store failure");
    record!(
        "DELETE",
        "/api/v0/shares",
        cancel_scan.status == "204 No Content" && cancel_scan.body.is_empty()
    );

    let application = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("slskd application under transfer-store failure");
    let application_json =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        application.status == "200 OK" && application_json.is_object()
    );

    let version = crate::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("slskd application version under transfer-store failure");
    record!(
        "GET",
        "/api/v0/application/version",
        version.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&version.body).is_ok()
    );

    let logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("slskd logs under transfer-store failure");
    record!(
        "GET",
        "/api/v0/logs",
        logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&logs.body)
                .ok()
                .is_some_and(|value| value.is_array())
    );

    for route in [
        "/api/v0/options/debug",
        "/api/v0/options/startup",
        "/api/v0/options/yaml/location",
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {route} under transfer-store failure: {error}"));
        record!(
            "GET",
            route,
            response.status == "200 OK" && !response.body.is_empty()
        );
    }

    let joined = crate::route_http_request("GET", "/api/v0/rooms/joined", None, "", &state)
        .await
        .expect("slskd joined rooms under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined",
        joined.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&joined.body)
                .ok()
                .is_some_and(|value| {
                    value
                        .as_array()
                        .is_some_and(|rooms| rooms.iter().any(|room| room == "runtime-room"))
                })
    );

    let room =
        crate::route_http_request("GET", "/api/v0/rooms/joined/runtime-room", None, "", &state)
            .await
            .expect("slskd room detail under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        room.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&room.body)
                .ok()
                .is_some_and(|value| value["name"] == "runtime-room")
    );

    let messages = crate::route_http_request(
        "GET",
        "/api/v0/rooms/joined/runtime-room/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd room messages under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        messages.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&messages.body)
                .ok()
                .is_some_and(|value| value.is_array())
    );

    let users = crate::route_http_request(
        "GET",
        "/api/v0/rooms/joined/runtime-room/users",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd room users under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        users.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&users.body)
                .ok()
                .is_some_and(|value| value.is_array())
    );

    let server = crate::route_http_request("GET", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd server under transfer-store failure");
    record!(
        "GET",
        "/api/v0/server",
        server.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&server.body)
                .ok()
                .is_some_and(|value| value.is_object())
    );

    let session = crate::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("slskd session under transfer-store failure");
    record!(
        "GET",
        "/api/v0/session",
        session.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&session.body)
                .ok()
                .is_some_and(|value| value.is_object())
    );

    let session_enabled =
        crate::route_http_request("GET", "/api/v0/session/enabled", None, "", &state)
            .await
            .expect("slskd session enabled under transfer-store failure");
    record!(
        "GET",
        "/api/v0/session/enabled",
        session_enabled.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&session_enabled.body)
                .ok()
                .is_some_and(|value| value == false)
    );

    let shares = crate::route_http_request("GET", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd shares under transfer-store failure");
    let shares_json = serde_json::from_str::<serde_json::Value>(&shares.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares",
        shares.status == "200 OK" && shares_json["local"].is_array()
    );

    let share = crate::route_http_request(
        "GET",
        &format!("/api/v0/shares/{share_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd share detail under transfer-store failure");
    let share_json = serde_json::from_str::<serde_json::Value>(&share.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares/{id}",
        share.status == "200 OK" && share_json["id"] == share_id
    );

    let metrics = crate::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
        .await
        .expect("slskd metrics under transfer-store failure");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        metrics.status == "200 OK" && metrics.body.contains("slskr_telemetry_transfers")
    );

    let kpis = crate::route_http_request("GET", "/api/v0/telemetry/metrics/kpis", None, "", &state)
        .await
        .expect("slskd KPIs under transfer-store failure");
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpis",
        kpis.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&kpis.body)
                .ok()
                .is_some_and(|value| value.is_object())
    );

    let download = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/runtime-download-peer/{download_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd download detail under transfer-store failure");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        download.status == "500 Internal Server Error"
    );

    let position = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/runtime-download-peer/{download_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd queue position under transfer-store failure");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        position.status == "500 Internal Server Error"
    );

    let upload = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/uploads/runtime-upload-peer/{upload_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd upload detail under transfer-store failure");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        upload.status == "500 Internal Server Error"
    );

    let browse_status = crate::route_http_request(
        "GET",
        "/api/v0/users/runtime-browse-peer/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd browse status under transfer-store failure");
    let browse_json =
        serde_json::from_str::<serde_json::Value>(&browse_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        browse_status.status == "200 OK" && browse_json["status"] == "requested"
    );

    let loopback = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"closed-db"}"#,
        &state,
    )
    .await
    .expect("slskd loopback under transfer-store failure");
    record!(
        "POST",
        "/api/v0/application/loopback",
        loopback.status == "200 OK" && loopback.content_type.is_empty() && loopback.body.is_empty()
    );

    let login = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"slskd","password":"runtime-login-secret"}"#,
        &state,
    )
    .await
    .expect("slskd session login under transfer-store failure");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/session",
        login.status == "200 OK" && login_json["tokenType"] == "Bearer"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_runtime_failure_isolation_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd runtime-failure ledger"),
    )
    .expect("write slskd runtime-failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd runtime-failure mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
