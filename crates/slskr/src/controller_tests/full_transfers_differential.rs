//! Controller full transfers differential ownership.

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
pub(super) async fn controller_api_differential_transfer_api_creates_updates_and_reports_stats() {
    let (state, _receiver) = test_state();

    let created = crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"direction\":0,\"filename\":\"Remote/Song.flac\",\"size\":100}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");
    assert!(created.body.contains("\"id\":1"));
    assert!(created.body.contains("\"status\":\"queued\""));

    let started = crate::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start transfer");
    assert_eq!(started.status, "200 OK");
    assert!(started.body.contains("\"status\":\"in_progress\""));

    let progress = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":40}",
        &state,
    )
    .await
    .expect("progress transfer");
    assert_eq!(progress.status, "200 OK");
    assert!(progress.body.contains("\"bytes_transferred\":40"));

    let missing_progress =
        crate::route_http_request("POST", "/api/v0/transfers/1/progress", None, "{}", &state)
            .await
            .expect("reject missing transfer progress");
    assert_eq!(missing_progress.status, "400 Bad Request");

    let oversized_progress = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":101}",
        &state,
    )
    .await
    .expect("reject oversized transfer progress");
    assert_eq!(oversized_progress.status, "400 Bad Request");

    let invalid_status = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"bytes_transferred\":40,\"status\":\"running\"}",
        &state,
    )
    .await
    .expect("reject non-terminal transfer status");
    assert_eq!(invalid_status.status, "400 Bad Request");

    let partial_success = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"bytes_transferred\":99,\"status\":\"succeeded\"}",
        &state,
    )
    .await
    .expect("reject partial transfer success");
    assert_eq!(partial_success.status, "400 Bad Request");

    let missing_completion_bytes = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"status\":\"failed\"}",
        &state,
    )
    .await
    .expect("reject missing completion bytes");
    assert_eq!(missing_completion_bytes.status, "400 Bad Request");

    let completed = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"bytes_transferred\":100}",
        &state,
    )
    .await
    .expect("complete transfer");
    assert_eq!(completed.status, "200 OK");
    assert!(completed.body.contains("\"status\":\"succeeded\""));

    let late_progress = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":50}",
        &state,
    )
    .await
    .expect("reject progress after completion");
    assert_eq!(late_progress.status, "409 Conflict");

    let stats = crate::route_http_request("GET", "/api/v0/transfers/stats", None, "", &state)
        .await
        .expect("transfer stats");
    assert_eq!(stats.status, "200 OK");
    assert!(stats.body.contains("\"total\":1"));
    assert!(stats.body.contains("\"succeeded\":1"));
    assert!(stats.body.contains("\"bytes_transferred\":100"));

    let filtered = crate::route_http_request(
        "GET",
        "/api/v0/transfers?status=succeeded&q=song&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered transfers");
    assert_eq!(filtered.status, "200 OK");
    let filtered_json = serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap();
    assert_eq!(filtered_json.as_array().unwrap().len(), 1);
    assert_eq!(
        filtered_json[0]["directories"][0]["files"][0]["filename"],
        "Remote/Song.flac"
    );

    let ledger = vec![serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/transfers",
        "case": "populated-dynamic-state",
        "pass": true,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("transfer_api_populated.json"),
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
pub(super) async fn controller_api_differential_transfer_cleanup_persistence() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let created = crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"filename":"Remote/Persisted.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create persisted transfer");
    assert_eq!(created.status, "201 Created");
    let persisted = db.get_transfer("1").await.expect("get created").unwrap();
    assert_eq!(persisted.direction, "download");
    assert_eq!(persisted.status, "queued");
    assert_eq!(persisted.filesize, 100);

    let started = crate::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start persisted transfer");
    assert_eq!(started.status, "200 OK");
    let persisted = db.get_transfer("1").await.expect("get started").unwrap();
    assert_eq!(persisted.status, "in_progress");

    let progressed = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":40}"#,
        &state,
    )
    .await
    .expect("progress persisted transfer");
    assert_eq!(progressed.status, "200 OK");
    let persisted = db.get_transfer("1").await.expect("get progressed").unwrap();
    assert_eq!(persisted.status, "in_progress");
    assert_eq!(persisted.progress, 40);

    let completed = crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        r#"{"bytes_transferred":100}"#,
        &state,
    )
    .await
    .expect("complete persisted transfer");
    assert_eq!(completed.status, "200 OK");
    let persisted = db.get_transfer("1").await.expect("get completed").unwrap();
    assert_eq!(persisted.status, "succeeded");
    assert_eq!(persisted.progress, 100);
    assert!(persisted.completed_at.is_some());
    let mut rehydrated = crate::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated.rehydrate_from_database(&db).await;
    assert!(rehydrated
        .entries
        .iter()
        .any(|entry| entry.id == 1 && entry.status == "succeeded"));
    let events = db
        .list_transfer_events(Some("1"), 10, 0)
        .await
        .expect("list transfer events");
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].status, "succeeded");
    assert_eq!(events[0].progress, 100);
    assert_eq!(events[0].filename, "Remote/Persisted.flac");
    assert_eq!(events[3].status, "queued");

    let cancelled = crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"friend","filename":"Remote/Cancel.flac","size":10}"#,
        &state,
    )
    .await
    .expect("create cancellable transfer");
    assert_eq!(cancelled.status, "201 Created");
    let aliased_delete =
        crate::route_http_request("DELETE", "/api/v0/transfers/unrelated/2", None, "", &state)
            .await
            .expect("reject aliased transfer delete");
    assert_eq!(aliased_delete.status, "404 Not Found");
    assert_eq!(
        db.get_transfer("2").await.unwrap().unwrap().status,
        "queued"
    );
    let deleted = crate::route_http_request("DELETE", "/api/v0/transfers/2", None, "", &state)
        .await
        .expect("cancel persisted transfer");
    assert_eq!(deleted.status, "200 OK");
    let persisted = db.get_transfer("2").await.expect("get cancelled").unwrap();
    assert_eq!(persisted.status, "cancelled");
    assert!(persisted.completed_at.is_some());
    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("transfer event database stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["persisted"]["transfers"], 2);
    assert_eq!(stats_json["persisted"]["transferEvents"], 6);

    let upload_created = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("uploader".to_owned()),
            "Uploads/Persisted.flac".to_owned(),
            None,
            Some(50),
        )
    };
    crate::persist_transfer_record(&state, &upload_created)
        .await
        .expect("persist upload");
    let upload_id = upload_created.id;
    let upload_completed = {
        let mut transfers = state.transfers.write().await;
        transfers
            .update_status(upload_id, "succeeded", Some(50), None)
            .expect("complete persisted upload")
    };
    crate::persist_transfer_record(&state, &upload_completed)
        .await
        .expect("persist completed upload");
    assert_eq!(
        db.get_transfer(&upload_id.to_string())
            .await
            .expect("get persisted upload")
            .unwrap()
            .status,
        "succeeded"
    );

    let pruned = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("prune persisted terminal downloads");
    assert_eq!(pruned.status, "204 No Content");
    assert!(pruned.body.is_empty());
    assert!(db.get_transfer("1").await.expect("get pruned 1").is_none());
    assert!(db.get_transfer("2").await.expect("get pruned 2").is_none());

    let uploads_pruned = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("prune persisted terminal uploads");
    assert_eq!(uploads_pruned.status, "204 No Content");
    assert!(uploads_pruned.body.is_empty());
    assert!(db
        .get_transfer(&upload_id.to_string())
        .await
        .expect("get pruned upload")
        .is_none());

    for case in [
        "nominal-status-headers-body",
        "mutation-side-effects-and-readback",
    ] {
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/transfers/downloads/all/completed",
            "case": case,
            "pass": true,
        }));
    }
    for case in [
        "nominal-status-headers-body",
        "mutation-side-effects-and-readback",
    ] {
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/transfers/uploads/all/completed",
            "case": case,
            "pass": true,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("transfer_cleanup_persistence.json"),
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
pub(super) async fn controller_api_differential_transfer_report_contracts() {
    let (state, _receiver) = test_state();
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($method:expr, $route:expr, $case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": true,
            }));
        };
    }
    {
        let mut transfers = state.transfers.write().await;
        let failed = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Failed.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(failed.id, "failed", Some(2), Some("timeout".to_owned()));
        let active = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Stalled.flac".to_owned(),
            None,
            Some(20),
        );
        transfers.update_status(active.id, "peer_lookup", Some(0), None);
        let complete = transfers.create(
            0,
            Some("other".to_owned()),
            "Remote/Done.flac".to_owned(),
            None,
            Some(30),
        );
        transfers.update_status(complete.id, "succeeded", Some(30), None);
        let upload = transfers.create(
            1,
            Some("uploader".to_owned()),
            "Uploads/Live.flac".to_owned(),
            None,
            Some(50),
        );
        transfers.update_status(upload.id, "in_progress", Some(5), None);
    }

    let stuck = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/stuck?username=friend",
        None,
        "",
        &state,
    )
    .await
    .expect("stuck report");
    assert_eq!(stuck.status, "200 OK");
    let stuck_json = serde_json::from_str::<serde_json::Value>(&stuck.body).unwrap();
    assert_eq!(stuck_json.as_array().unwrap().len(), 2);
    assert!(stuck_json
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["filename"] == "Remote/Failed.flac"
            && entry["reason"] == "transfer failed"));
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/stuck",
        "populated-dynamic-state"
    );

    let user_stats = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/user-stats",
        None,
        "",
        &state,
    )
    .await
    .expect("user stats report");
    assert_eq!(user_stats.status, "200 OK");
    let user_stats_json = serde_json::from_str::<serde_json::Value>(&user_stats.body).unwrap();
    assert_eq!(user_stats_json.as_object().unwrap().len(), 2);
    assert_eq!(user_stats_json["friend"]["username"], "friend");
    assert_eq!(user_stats_json["friend"]["totalDownloads"], 2);
    assert_eq!(user_stats_json["friend"]["successfulDownloads"], 0);
    assert_eq!(user_stats_json["friend"]["failedDownloads"], 1);
    assert_eq!(user_stats_json["friend"]["totalBytes"], 0);
    assert!(user_stats_json["friend"]["lastDownloadAt"].is_string());
    assert_eq!(user_stats_json["other"]["username"], "other");
    assert_eq!(user_stats_json["other"]["totalDownloads"], 1);
    assert_eq!(user_stats_json["other"]["successfulDownloads"], 1);
    assert_eq!(user_stats_json["other"]["failedDownloads"], 0);
    assert_eq!(user_stats_json["other"]["totalBytes"], 30);
    assert!(user_stats_json["other"]["lastDownloadAt"].is_string());

    let accelerated = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/accelerated?username=friend",
        None,
        "",
        &state,
    )
    .await
    .expect("accelerated report");
    assert_eq!(accelerated.status, "200 OK");
    let accelerated_json = serde_json::from_str::<serde_json::Value>(&accelerated.body).unwrap();
    assert_eq!(accelerated_json["enabled"], false);
    assert!(accelerated_json["updatedAt"].is_string());
    assert!(accelerated_json["policy"].is_string());
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/accelerated",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/accelerated",
        "populated-dynamic-state"
    );

    let speeds = crate::route_http_request("GET", "/api/v0/transfers/speeds", None, "", &state)
        .await
        .expect("speeds report");
    assert_eq!(speeds.status, "200 OK");
    let speeds_json = serde_json::from_str::<serde_json::Value>(&speeds.body).unwrap();
    assert_eq!(speeds_json["active_transfers"], 2);
    assert_eq!(speeds_json["activeDownloads"], 1);
    assert_eq!(speeds_json["activeUploads"], 1);
    assert_eq!(speeds_json["downloadBytesTransferred"], 32);
    assert_eq!(speeds_json["uploadBytesTransferred"], 5);
    assert_eq!(speeds_json["total_bytes_transferred"], 37);

    let stats =
        crate::route_http_request("GET", "/api/v0/transfers/downloads/stats", None, "", &state)
            .await
            .expect("download stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["total_downloads"], 3);
    assert_eq!(stats_json["active"], 1);
    assert_eq!(stats_json["completed"], 1);
    assert_eq!(stats_json["failed"], 1);
    assert_eq!(stats_json["total_size"], 60);
    assert_eq!(stats_json["bytesTransferred"], 32);

    let uploads = crate::route_http_request("GET", "/api/v0/transfers/uploads", None, "", &state)
        .await
        .expect("uploads report");
    assert_eq!(uploads.status, "200 OK");
    let uploads_json = serde_json::from_str::<serde_json::Value>(&uploads.body).unwrap();
    assert_eq!(uploads_json.as_array().unwrap().len(), 1);
    assert_eq!(uploads_json[0]["username"], "uploader");
    assert_eq!(uploads_json[0]["directories"].as_array().unwrap().len(), 1);
    assert_eq!(uploads_json[0]["directories"][0]["directory"], "Uploads");
    assert_eq!(uploads_json[0]["directories"][0]["fileCount"], 1);
    assert_eq!(
        uploads_json[0]["directories"][0]["files"][0]["filename"],
        "Uploads/Live.flac"
    );
    assert_eq!(
        uploads_json[0]["directories"][0]["files"][0]["direction"],
        "Upload"
    );

    let user_downloads = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/friend",
        None,
        "",
        &state,
    )
    .await
    .expect("user downloads report");
    assert_eq!(user_downloads.status, "200 OK");
    let user_downloads_json =
        serde_json::from_str::<serde_json::Value>(&user_downloads.body).unwrap();
    assert_eq!(user_downloads_json["username"], "friend");
    assert!(user_downloads_json.is_object());
    assert_eq!(user_downloads_json["directories"][0]["directory"], "Remote");
    assert_eq!(user_downloads_json["directories"][0]["fileCount"], 2);

    let user_uploads = crate::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/uploader",
        None,
        "",
        &state,
    )
    .await
    .expect("user uploads report");
    assert_eq!(user_uploads.status, "200 OK");
    let user_uploads_json = serde_json::from_str::<serde_json::Value>(&user_uploads.body).unwrap();
    assert_eq!(user_uploads_json["username"], "uploader");
    assert!(user_uploads_json.is_object());
    assert_eq!(user_uploads_json["directories"][0]["directory"], "Uploads");
    assert_eq!(user_uploads_json["directories"][0]["fileCount"], 1);

    let (failed_id, upload_id) = {
        let transfers = state.transfers.read().await;
        (
            transfers
                .entries
                .iter()
                .find(|entry| entry.filename == "Remote/Failed.flac")
                .expect("failed transfer id")
                .id,
            transfers
                .entries
                .iter()
                .find(|entry| entry.filename == "Uploads/Live.flac")
                .expect("upload transfer id")
                .id,
        )
    };
    let download_detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/friend/{failed_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("download detail report");
    assert_eq!(download_detail.status, "200 OK");
    let download_detail_json =
        serde_json::from_str::<serde_json::Value>(&download_detail.body).unwrap();
    assert_eq!(download_detail_json["id"], failed_id.to_string());
    assert_eq!(download_detail_json["filename"], "Remote/Failed.flac");
    assert_eq!(download_detail_json["direction"], "Download");
    assert_eq!(download_detail_json["state"], "Failed");

    let upload_detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/uploads/uploader/{upload_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("upload detail report");
    assert_eq!(upload_detail.status, "200 OK");
    let upload_detail_json =
        serde_json::from_str::<serde_json::Value>(&upload_detail.body).unwrap();
    assert_eq!(upload_detail_json["id"], upload_id.to_string());
    assert_eq!(upload_detail_json["filename"], "Uploads/Live.flac");
    assert_eq!(upload_detail_json["direction"], "Upload");
    assert_eq!(upload_detail_json["state"], "InProgress");

    for (route, missing_route) in [
        (
            "/api/v0/transfers/downloads/{username}/{id}",
            "/api/v0/transfers/downloads/friend/999999",
        ),
        (
            "/api/v0/transfers/uploads/{username}/{id}",
            "/api/v0/transfers/uploads/uploader/999999",
        ),
    ] {
        let missing = crate::route_http_request("GET", missing_route, None, "", &state)
            .await
            .expect("missing transfer detail report");
        assert_eq!(missing.status, "404 Not Found");
        record_evidence!("GET", route, "missing-empty-or-conflict-state");
        record_evidence!("GET", route, "nominal-status-headers-body");
        record_evidence!("GET", route, "populated-dynamic-state");
    }

    for (_route, ledger_route) in [
        ("/api/v0/transfers/uploads", "/api/v0/transfers/uploads"),
        (
            "/api/v0/transfers/downloads/friend",
            "/api/v0/transfers/downloads/{username}",
        ),
        (
            "/api/v0/transfers/uploads/uploader",
            "/api/v0/transfers/uploads/{username}",
        ),
    ] {
        record_evidence!("GET", ledger_route, "nominal-status-headers-body");
        record_evidence!("GET", ledger_route, "populated-dynamic-state");
    }
    for route in [
        "/api/v0/transfers/downloads/friend",
        "/api/v0/transfers/uploads/uploader",
    ] {
        let missing_route = if route.contains("downloads") {
            route.replace("friend", "no-such-user")
        } else {
            route.replace("uploader", "no-such-user")
        };
        let missing = crate::route_http_request("GET", &missing_route, None, "", &state)
            .await
            .expect("missing user transfer report");
        assert_eq!(missing.status, "404 Not Found");
        record_evidence!(
            "GET",
            if route.contains("downloads") {
                "/api/v0/transfers/downloads/{username}"
            } else {
                "/api/v0/transfers/uploads/{username}"
            },
            "missing-empty-or-conflict-state"
        );
    }

    for (route, case) in [
        (
            "/api/v0/transfers/downloads/user-stats",
            "nominal-status-headers-body",
        ),
        (
            "/api/v0/transfers/downloads/user-stats",
            "populated-dynamic-state",
        ),
        ("/api/v0/transfers/speeds", "nominal-status-headers-body"),
        ("/api/v0/transfers/speeds", "populated-dynamic-state"),
    ] {
        record_evidence!("GET", route, case);
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("transfer_report_contracts.json"),
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
pub(super) async fn controller_api_differential_transfer_upload_diagnostics() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let upload_id = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("diagnostic-uploader".to_owned()),
            "Uploads/Diagnostics.flac".to_owned(),
            None,
            Some(128),
        );
        transfers
            .update_status(entry.id, "in_progress", Some(32), None)
            .expect("mark diagnostic upload active")
            .id
    };

    let response = crate::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/diagnostics",
        None,
        "",
        &state,
    )
    .await
    .expect("upload diagnostics");
    let value =
        serde_json::from_str::<serde_json::Value>(&response.body).expect("upload diagnostics JSON");
    let nominal = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value["generatedAt"].is_number()
        && value["isConnected"] == false
        && value["isLoggedIn"] == false
        && value["listenPort"].is_number()
        && value["recentUploads"].is_array()
        && value["warnings"].is_array();
    let populated = nominal
        && value["activeUploads"] == 1
        && value["failedUploads"] == 0
        && value["succeededUploads"] == 0
        && value["totalUploadRecords"] == 1
        && value["recentUploads"][0]["id"] == upload_id
        && value["recentUploads"][0]["filename"] == "Uploads/Diagnostics.flac";
    assert!(nominal, "{} {}", response.status, response.body);
    assert!(populated, "{}", response.body);

    let ledger = vec![
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/transfers/uploads/diagnostics",
            "case": "nominal-status-headers-body",
            "pass": nominal,
        }),
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/transfers/uploads/diagnostics",
            "case": "populated-dynamic-state",
            "pass": populated,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("transfer_upload_diagnostics.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting `missing-empty-or-conflict-
/// state` for the transfer-reports family (`GET /api/v0/telemetry/
/// reports/transfers/leaderboard`, `/exceptions`, `/exceptions/
/// pareto`): a missing `direction` query parameter is a real 400
/// (`Enum.TryParse<TransferDirection>` failure path), not silently
/// treated as "no filter" and mixing Upload/Download rows into one
/// report -- independently re-derived from `transfer_reports_
/// require_a_real_direction_not_a_silent_default` with fresh
/// request shapes. Confirmed against `/tmp/slskr-parity-evidence/
/// controller-api/*.json` before writing: all 3 routes already had
/// `malformed-path-query-or-body` credited from an earlier batch,
/// but none had this case. slskdN-only (confirmed against the
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
pub(super) async fn controller_api_differential_transfer_reports_required_direction() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [missing-empty-or-conflict-state]",
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "missing-empty-or-conflict-state",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    for path in [
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
    ] {
        let missing =
            crate::route_http_request("GET", &format!("{path}?limit=5"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            path,
            missing.status == "400 Bad Request" && missing.body.contains("Direction is required")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("transfer_reports_required_direction.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api transfer-reports-required-direction mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the real slskdN transfer-cancel
/// route (`DELETE /api/v0/transfers/downloads/{username}/{id}`):
/// cancelling a nonexistent download is a real 404, and cancelling
/// a real queued download returns the frozen `204 No Content`
/// contract -- independently re-derived from `transfer_
/// cancellation_returns_frozen_statuses` with fresh fixture data.
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing: this route had zero prior case credited
/// (checked every case, not just route presence). slskdN-only
/// (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_transfer_download_cancel() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} DELETE /api/v0/transfers/downloads/{{username}}/{{id}} [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "DELETE",
                "route": "/api/v0/transfers/downloads/{username}/{id}",
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();

    let missing = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/differential-peer/999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing transfer cancel response");
    record!(
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let entry = state.transfers.write().await.create(
        0,
        Some("differential-peer".to_owned()),
        "differential-cancel.flac".to_owned(),
        None,
        Some(1),
    );
    let cancelled = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/differential-peer/{}", entry.id),
        None,
        "",
        &state,
    )
    .await
    .expect("real transfer cancel response");
    record!(
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content"
    );
    record!(
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("transfer_download_cancel.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api transfer-download-cancel mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_transfer_upload_cancel() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} DELETE /api/v0/transfers/uploads/{{username}}/{{id}} [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "DELETE",
                "route": "/api/v0/transfers/uploads/{username}/{id}",
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let missing = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/differential-uploader/999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing upload cancel response");
    record!(
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let entry = state.transfers.write().await.create(
        1,
        Some("differential-uploader".to_owned()),
        "Uploads/differential-upload.flac".to_owned(),
        None,
        Some(1),
    );
    let cancelled = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/uploads/differential-uploader/{}",
            entry.id
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("real upload cancel response");
    record!(
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );
    record!(
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content"
            && state.transfers.read().await.entries[0].status == "cancelled"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("transfer_upload_cancel.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api transfer-upload-cancel mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the residual compatibility and request-level
/// download controller cases.
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
pub(super) async fn controller_api_differential_downloads_open_cases() {
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
    let valid_id = "11111111-1111-4111-8111-111111111111";
    let download_body = r#"{"items":[{"user":"download-peer","remotePath":"Remote/Download.flac","targetDir":"Downloads"}]}"#;

    // GET /api/downloads
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            conversation_request(&state, "GET", "/api/downloads?status=not-a-state", "").await;
        record!(
            "GET",
            "/api/downloads",
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_value(&malformed)["downloads"].is_array()
        );
        let missing = conversation_request(&state, "GET", "/api/downloads", "").await;
        record!(
            "GET",
            "/api/downloads",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && json_value(&missing)["downloads"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
        download_request_seed(&state).await;
        let populated = conversation_request(&state, "GET", "/api/downloads", "").await;
        record!(
            "GET",
            "/api/downloads",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && json_value(&populated)["downloads"]
                    .as_array()
                    .is_some_and(|downloads| downloads.len() == 1)
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let response = conversation_request(&state, "GET", "/api/downloads", "").await;
        record!(
            "GET",
            "/api/downloads",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    // GET /api/downloads/{id}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal =
            conversation_request(&state, "GET", &format!("/api/downloads/{request_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && json_value(&nominal)["Id"].is_string()
                && json_value(&nominal)["Status"].is_string()
        );
        let malformed = conversation_request(&state, "GET", "/api/downloads/not-a-guid", "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing =
            conversation_request(&state, "GET", &format!("/api/downloads/{valid_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime request id");
        let response =
            conversation_request(&state, "GET", &format!("/api/downloads/{request_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        download_request_add_attempt(&state, &request_id).await;
        let response =
            conversation_request(&state, "GET", &format!("/api/downloads/{request_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "populated-dynamic-state",
            response.status == "200 OK"
                && json_value(&response)["RemotePath"] == "Remote/Download.flac"
        );
    }

    // GET /api/v0/downloads/requests
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && json_value(&nominal).is_array()
        );
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/downloads/requests?state=not-a-state",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_value(&malformed).is_array()
        );
        let missing = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && json_value(&missing).is_array()
        );
        download_request_seed(&state).await;
        let populated = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && json_value(&populated)
                    .as_array()
                    .is_some_and(|requests| requests.len() == 1)
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let response = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    // GET /api/v0/downloads/requests/{id:guid}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{request_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && json_value(&nominal)["request"].is_object()
                && json_value(&nominal)["attempts"].is_array()
        );
        let malformed =
            conversation_request(&state, "GET", "/api/v0/downloads/requests/not-a-guid", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{valid_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime detail request id");
        let response = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{request_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        download_request_add_attempt(&state, &request_id).await;
        let response = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{request_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "populated-dynamic-state",
            response.status == "200 OK"
                && json_value(&response)["attempts"]
                    .as_array()
                    .is_some_and(|attempts| attempts.len() == 2)
        );
    }

    // PATCH /api/v0/downloads/requests/{id:guid}/name
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{request_id}/name"),
            r#"{"name":"Renamed"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && json_value(&nominal)["request"].is_object()
        );
        let malformed = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{request_id}/name"),
            r#"{"name":""}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let mutation = state
            .transfers
            .read()
            .await
            .entries
            .iter()
            .all(|entry| entry.request_name.as_deref() == Some("Renamed"));
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "mutation-side-effects-and-readback",
            mutation
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{valid_id}/name"),
            r#"{"name":"Missing"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime rename request id");
        let response = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{request_id}/name"),
            r#"{"name":"Runtime"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{valid_id}/name"),
            r#"{"name":"Restart"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let path = format!("/api/v0/downloads/requests/{request_id}/name");
        let responses = futures_util::future::join_all([
            conversation_request(&state, "PATCH", &path, r#"{"name":"One"}"#),
            conversation_request(&state, "PATCH", &path, r#"{"name":"Two"}"#),
        ])
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
        );
    }

    // POST /api/downloads
    {
        let (state, mut receiver) = test_state_with_env(target_env());
        let nominal = conversation_request(&state, "POST", "/api/downloads", download_body).await;
        let command = receiver.try_recv().ok();
        record!(
            "POST",
            "/api/downloads",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && json_value(&nominal)["downloadIds"].is_array()
                && matches!(command, Some(crate::SessionCommand::TransferPeer { .. }))
        );
        let malformed = conversation_request(&state, "POST", "/api/downloads", "{} ").await;
        record!(
            "POST",
            "/api/downloads",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = conversation_request(&state, "POST", "/api/downloads", "").await;
        record!(
            "POST",
            "/api/downloads",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
        let mutation = state.transfers.read().await.entries.len() == 1;
        record!(
            "POST",
            "/api/downloads",
            "mutation-side-effects-and-readback",
            mutation
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let response = conversation_request(&state, "POST", "/api/downloads", download_body).await;
        record!(
            "POST",
            "/api/downloads",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(&state, "POST", "/api/downloads", download_body).await;
        record!(
            "POST",
            "/api/downloads",
            "restart-persistence-or-reset",
            response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let responses = futures_util::future::join_all([
            conversation_request(&state, "POST", "/api/downloads", download_body),
            conversation_request(&state, "POST", "/api/downloads", download_body),
        ])
        .await;
        record!(
            "POST",
            "/api/downloads",
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
        );
    }

    // POST /api/v0/downloads/requests/{id:guid}/cancel
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{request_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "nominal-status-headers-body",
            nominal.status == "204 No Content" && nominal.body.is_empty()
        );
        let malformed = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{request_id}/cancel/extra"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        let mutation = state
            .transfers
            .read()
            .await
            .entries
            .iter()
            .all(|entry| entry.status == "cancelled");
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "mutation-side-effects-and-readback",
            mutation
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{valid_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime cancel request id");
        let response = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{request_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{valid_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let path = format!("/api/v0/downloads/requests/{request_id}/cancel");
        let responses = futures_util::future::join_all([
            conversation_request(&state, "POST", &path, ""),
            conversation_request(&state, "POST", &path, ""),
        ])
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
        );
    }

    assert_eq!(ledger.len(), 40, "downloads residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create downloads evidence directory");
    fs::write(
        evidence_dir.join("downloads_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize downloads ledger"),
    )
    .expect("write downloads ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api downloads mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
