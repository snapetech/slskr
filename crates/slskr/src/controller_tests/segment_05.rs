#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_projection_persists_status_and_surfaces_failures() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let entry = super::TransferEntry {
        id: 99,
        direction: 0,
        token: 7,
        peer_username: Some("friend".to_owned()),
        filename: "Remote/Projected.flac".to_owned(),
        local_path: None,
        batch_id: None,
        request_id: None,
        wishlist_item_id: None,
        request_name: None,
        destination_directory: None,
        bit_rate: None,
        sample_rate: None,
        bit_depth: None,
        length_seconds: None,
        artist: None,
        album: None,
        title: None,
        track_number: None,
        year: None,
        attempts: 1,
        auto_replace_attempts: 0,
        next_attempt_at: None,
        size: Some(100),
        bytes_transferred: 40,
        status: "in_progress".to_owned(),
        reason: None,
        requested_at: 1,
        started_at: Some(1),
        start_offset: 0,
        updated_at: 2,
        updated_at_ms: 2_000,
        previous_status: None,
    };

    super::persist_transfer_projection(&state, &entry).await;
    let persisted = db.get_transfer("99").await.unwrap().unwrap();
    assert_eq!(persisted.status, "in_progress");
    assert_eq!(persisted.progress, 40);

    db.close_for_test().await;
    super::persist_transfer_projection(&state, &entry).await;
    let session = state.session.read().await;
    let error = session.last_error.as_deref().unwrap_or_default();
    assert!(error.contains("transfer 99 persistence failed"));
    assert!(error.contains("failed to persist transfer"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn completed_transfer_cleanup_rolls_back_on_persistence_failure() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"filename":"Remote/Completed.flac","size":10}"#,
        &state,
    )
    .await
    .expect("create transfer");
    super::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        r#"{"bytes_transferred":10}"#,
        &state,
    )
    .await
    .expect("complete transfer");
    let entries_before_failure = state.transfers.read().await.entries.clone();
    db.close_for_test().await;

    let response = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup response");

    assert_eq!(response.status, "503 Service Unavailable");
    assert_eq!(state.transfers.read().await.entries, entries_before_failure);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_cancel_rolls_back_on_persistence_failure() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let original = state.transfers.write().await.create(
        0,
        Some("friend".to_owned()),
        "Remote/Keep.flac".to_owned(),
        None,
        Some(10),
    );
    db.close_for_test().await;

    let response = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/{}", original.id),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel response");

    assert_eq!(response.status, "503 Service Unavailable");
    assert_eq!(state.transfers.read().await.entries, vec![original]);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_start_enforces_max_active_policy() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_MAX_ACTIVE", "1"));

    for filename in ["Remote/One.flac", "Remote/Two.flac"] {
        let body = format!("{{\"filename\":\"{}\",\"size\":10}}", filename);
        let created = super::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
            .await
            .expect("create transfer");
        assert_eq!(created.status, "201 Created");
    }

    let started = super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start first");
    assert_eq!(started.status, "200 OK");

    let blocked = super::route_http_request("POST", "/api/v0/transfers/2/start", None, "", &state)
        .await
        .expect("start second");
    assert_eq!(blocked.status, "409 Conflict");
    assert_eq!(blocked.body, "{\"error\":\"transfer limit reached\"}");

    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(1, "cancelled", None, None);
    }

    let unblocked =
        super::route_http_request("POST", "/api/v0/transfers/2/start", None, "", &state)
            .await
            .expect("retry second");
    assert_eq!(unblocked.status, "200 OK");
    assert!(unblocked.body.contains("\"status\":\"in_progress\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_retry_requeues_failed_download_and_clears_reason() {
    let (state, mut receiver) = test_state();
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(
            entry.id,
            "failed",
            Some(4),
            Some("peer timed out".to_owned()),
        );
    }

    let retried = super::route_http_request("POST", "/api/v0/transfers/1/retry", None, "", &state)
        .await
        .expect("retry transfer");

    assert_eq!(retried.status, "200 OK");
    assert!(retried.body.contains("\"status\":\"peer_lookup\""));
    assert!(retried.body.contains("\"bytes_transferred\":4"));
    assert!(retried.body.contains("\"reason\":null"));
    assert_eq!(
        receiver.try_recv().expect("transfer command"),
        super::SessionCommand::TransferPeer {
            id: 1,
            username: "friend".to_owned(),
        }
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn download_requests_preserve_metadata_path_and_runtime_lifecycle() {
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
        "{uploader}/{remote_parent}/{request_name}",
    ));
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
    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"filename":"Remote/Album/Song.flac","peer_username":"friend","size":10,"requestName":"Archive Cut","bitRate":1411,"sampleRate":96000,"bitDepth":24,"length":245,"artist":"Archive Artist","album":"Open Sessions","title":"Song"}"#,
        &state,
    )
    .await
    .expect("create request transfer");
    assert_eq!(created.status, "201 Created");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let request_id = created_json["request_id"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(request_id).is_ok());
    assert_eq!(created_json["bit_rate"], 1411);
    let local_path = state.transfers.read().await.entries[0]
        .local_path
        .clone()
        .unwrap();
    let expected_suffix = std::path::Path::new("friend")
        .join("Album")
        .join("Archive Cut")
        .join("Song.flac");
    assert!(std::path::Path::new(&local_path).ends_with(expected_suffix));

    let listed = super::route_http_request(
        "GET",
        "/api/v0/downloads/requests?state=active",
        None,
        "",
        &state,
    )
    .await
    .expect("list requests");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    assert_eq!(listed_json[0]["request"]["id"], request_id);
    assert_eq!(listed_json[0]["request"]["sampleRate"], 96_000);
    record_evidence!(
        "GET",
        "/api/v0/downloads/requests",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/downloads/requests",
        "populated-dynamic-state"
    );

    let renamed = super::route_http_request(
        "PATCH",
        &format!("/api/v0/downloads/requests/{request_id}/name"),
        None,
        r#"{"name":"Renamed request"}"#,
        &state,
    )
    .await
    .expect("rename request");
    assert_eq!(renamed.status, "200 OK");
    assert_eq!(
        state.transfers.read().await.entries[0]
            .request_name
            .as_deref(),
        Some("Renamed request")
    );
    record_evidence!(
        "PATCH",
        "/api/v0/downloads/requests/{id:guid}/name",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "PATCH",
        "/api/v0/downloads/requests/{id:guid}/name",
        "mutation-side-effects-and-readback"
    );

    let cancelled = super::route_http_request(
        "POST",
        &format!("/api/v0/downloads/requests/{request_id}/cancel"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel request");
    assert_eq!(cancelled.status, "204 No Content");
    assert_eq!(state.transfers.read().await.entries[0].status, "cancelled");
    record_evidence!(
        "POST",
        "/api/v0/downloads/requests/{id:guid}/cancel",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/downloads/requests/{id:guid}/cancel",
        "mutation-side-effects-and-readback"
    );

    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/downloads/requests/{request_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("request detail");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap();
    assert_eq!(detail_json["request"]["state"], "Cancelled");
    assert_eq!(detail_json["attempts"].as_array().unwrap().len(), 1);
    record_evidence!(
        "GET",
        "/api/v0/downloads/requests/{id:guid}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/downloads/requests/{id:guid}",
        "populated-dynamic-state"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("download_requests_runtime_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_failure_projection_exposes_bounded_recovery_not_internal_reason() {
    let mut transfers = super::TransferQueue::new_in_memory(10);
    let entry = transfers.create(
        0,
        Some("friend".to_owned()),
        "Remote/Song.flac".to_owned(),
        None,
        Some(10),
    );
    let failed = transfers
        .update_status(
            entry.id,
            "rejected",
            None,
            Some("Transfer rejected: File not shared; internal=/secret".to_owned()),
        )
        .unwrap();
    let json = failed.json();
    assert!(json.contains(r#""failure_code":"remote_file_unavailable""#));
    assert!(json.contains(r#""recovery_label":"Find other sources""#));
    assert!(!json.contains("/secret"));
    assert!(!json.contains("File not shared"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn completed_audio_metadata_parsers_cover_mp3_and_wav_headers() {
    let mp3 =
        super::mp3_technical_metadata(&[0xff, 0xfb, 0x90, 0x00], 128_000).expect("mp3 metadata");
    assert_eq!(mp3.bit_rate, Some(128));
    assert_eq!(mp3.sample_rate, Some(44_100));
    assert_eq!(mp3.length_seconds, Some(8));

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&176_436_u32.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&44_100_u32.to_le_bytes());
    wav.extend_from_slice(&176_400_u32.to_le_bytes());
    wav.extend_from_slice(&4_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&176_400_u32.to_le_bytes());
    let wav = super::wav_technical_metadata(&wav).expect("wav metadata");
    assert_eq!(wav.bit_rate, Some(1_411));
    assert_eq!(wav.sample_rate, Some(44_100));
    assert_eq!(wav.bit_depth, Some(16));
    assert_eq!(wav.length_seconds, Some(1));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_retry_rejects_non_terminal_download() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Song.flac\",\"peer_username\":\"friend\",\"size\":10}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");

    let blocked = super::route_http_request("POST", "/api/v0/transfers/1/retry", None, "", &state)
        .await
        .expect("retry transfer");

    assert_eq!(blocked.status, "409 Conflict");
    assert_eq!(blocked.body, "{\"error\":\"transfer is not retryable\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_transfer_position_requests_and_returns_the_remote_queue_place() {
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/transfers/downloads/{username}/{id}/position",
                "case": $case,
                "pass": true,
            }));
        };
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind queue position fixture");
    let local_addr = listener
        .local_addr()
        .expect("queue position fixture address");
    let server = tokio::spawn(async move {
        for (expected_filename, expected_place) in
            [("Remote/Two.flac", 4_u32), ("Remote/One.flac", 7_u32)]
        {
            let (stream, _) = listener.accept().await.expect("accept queue position");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            assert_eq!(
                init.receive().await.expect("queue position init"),
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "tester".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("queue position request"),
                super::PeerMessage::PlaceInQueueRequest {
                    filename: expected_filename.to_owned(),
                }
            );
            peer.send(&super::PeerMessage::PlaceInQueueResponse {
                filename: expected_filename.to_owned(),
                place: expected_place,
            })
            .await
            .expect("queue position response");
        }
    });
    let endpoint = format!("friend={local_addr}");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", &endpoint),
    );
    state.session.write().await.state = "connected";
    let (first_id, second_id) = {
        let mut transfers = state.transfers.write().await;
        let first = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/One.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.update_status(first.id, "completed", Some(1), None);
        let second = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Two.flac".to_owned(),
            None,
            Some(2),
        );
        (first.id, second.id)
    };

    let active = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/friend/{second_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("active position");
    assert_eq!(active.status, "200 OK", "{}", active.body);
    assert_eq!(active.body, "4");
    record_evidence!("nominal-status-headers-body");
    record_evidence!("populated-dynamic-state");

    let completed = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/friend/{first_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("completed position");
    assert_eq!(completed.status, "200 OK");
    assert_eq!(completed.body, "7");

    // An id that isn't a real download for this username must 404,
    // not silently report a position.
    let unknown_id = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/friend/999999/position",
        None,
        "",
        &state,
    )
    .await
    .expect("unknown id");
    assert_eq!(unknown_id.status, "404 Not Found");

    let wrong_username = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/other/{second_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("wrong username");
    assert_eq!(wrong_username.status, "404 Not Found");
    record_evidence!("missing-empty-or-conflict-state");
    server.await.expect("queue position fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("transfer_position_contracts.json"),
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
async fn controller_api_differential_transfer_report_contracts() {
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

    let stuck = super::route_http_request(
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

    let user_stats = super::route_http_request(
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

    let accelerated = super::route_http_request(
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

    let speeds = super::route_http_request("GET", "/api/v0/transfers/speeds", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/transfers/downloads/stats", None, "", &state)
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

    let uploads = super::route_http_request("GET", "/api/v0/transfers/uploads", None, "", &state)
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

    let user_downloads = super::route_http_request(
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

    let user_uploads = super::route_http_request(
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
    let download_detail = super::route_http_request(
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

    let upload_detail = super::route_http_request(
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
        let missing = super::route_http_request("GET", missing_route, None, "", &state)
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
        let missing = super::route_http_request("GET", &missing_route, None, "", &state)
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
async fn controller_api_differential_transfer_upload_diagnostics() {
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

    let response = super::route_http_request(
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_find_alternative_and_replace_use_cached_search_results() {
    let (state, mut receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Album/Song.flac\",\"peer_username\":\"stale\",\"size\":100}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");
    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(1, "failed", Some(25), Some("peer offline".to_owned()));
    }

    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"song\"}",
        &state,
    )
    .await
    .expect("create search");
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":1,\"peer_username\":\"fresh\",\"filename\":\"Other/Path/Song.flac\",\"size\":100,\"slot_free\":true,\"average_speed\":2048}",
        &state,
    )
    .await
    .expect("ingest alternative");

    let alternatives = super::route_http_request(
        "POST",
        "/api/transfers/downloads/find-alternative",
        None,
        "{\"transfer_id\":1}",
        &state,
    )
    .await
    .expect("find alternative");
    assert_eq!(alternatives.status, "200 OK");
    let alternatives_json = serde_json::from_str::<serde_json::Value>(&alternatives.body).unwrap();
    assert_eq!(alternatives_json["count"], 1);
    assert_eq!(alternatives_json["alternatives"][0]["username"], "fresh");

    let replaced = super::route_http_request(
        "POST",
        "/api/transfers/downloads/replace",
        None,
        "{\"transfer_id\":1,\"username\":\"fresh\"}",
        &state,
    )
    .await
    .expect("replace transfer");
    assert_eq!(replaced.status, "202 Accepted");
    let replaced_json = serde_json::from_str::<serde_json::Value>(&replaced.body).unwrap();
    assert_eq!(replaced_json["replacement_queued"], true);
    assert_eq!(replaced_json["replacement"]["id"], 2);
    assert_eq!(replaced_json["replacement"]["peer_username"], "fresh");
    assert_eq!(replaced_json["replacement"]["status"], "peer_lookup");
    assert_eq!(
        receiver.try_recv().expect("replacement peer lookup"),
        super::SessionCommand::TransferPeer {
            id: 2,
            username: "fresh".to_owned(),
        }
    );

    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == 1)
        .unwrap();
    assert_eq!(original.status, "cancelled");
    assert_eq!(original.bytes_transferred, 25);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_auto_replace_queues_failed_download_alternatives() {
    let (state, mut receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Album/Song.flac\",\"peer_username\":\"stale\",\"size\":100}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");
    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(1, "failed", Some(40), Some("peer offline".to_owned()));
    }

    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"song\"}",
        &state,
    )
    .await
    .expect("create search");
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":1,\"peer_username\":\"fresh\",\"filename\":\"Other/Path/Song.flac\",\"size\":100}",
        &state,
    )
    .await
    .expect("ingest alternative");

    let replaced = super::route_http_request(
        "POST",
        "/api/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("auto replace");
    assert_eq!(replaced.status, "202 Accepted");
    let json = serde_json::from_str::<serde_json::Value>(&replaced.body).unwrap();
    assert_eq!(json["replacement_queued"], true);
    assert_eq!(json["status"], "queued");
    assert_eq!(json["alternatives"].as_array().unwrap().len(), 1);
    assert_eq!(json["alternatives"][0]["username"], "fresh");
    assert_eq!(json["replacements"][0]["transfer_id"], 1);
    assert_eq!(json["replacements"][0]["replacement"]["id"], 2);
    assert_eq!(
        json["replacements"][0]["replacement"]["peer_username"],
        "fresh"
    );
    assert_eq!(
        receiver.try_recv().expect("replacement peer lookup"),
        super::SessionCommand::TransferPeer {
            id: 2,
            username: "fresh".to_owned(),
        }
    );

    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == 1)
        .unwrap();
    assert_eq!(original.status, "cancelled");
    assert_eq!(original.bytes_transferred, 40);
    assert_eq!(
        original.reason.as_deref(),
        Some("auto-replaced by alternative source")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_replacements_reject_before_mutation_when_dispatch_is_unavailable() {
    for (path, body) in [
        (
            "/api/transfers/downloads/replace",
            r#"{"transfer_id":1,"username":"fresh"}"#,
        ),
        (
            "/api/transfers/downloads/auto-replace",
            r#"{"transfer_id":1}"#,
        ),
    ] {
        let (state, mut receiver) = test_state();
        state.session.write().await.state = "connected";
        super::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            r#"{"filename":"Remote/Album/Song.flac","peer_username":"stale","size":100}"#,
            &state,
        )
        .await
        .expect("create transfer");
        state.transfers.write().await.update_status(
            1,
            "failed",
            Some(25),
            Some("peer offline".to_owned()),
        );
        super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            r#"{"query":"song"}"#,
            &state,
        )
        .await
        .expect("create search");
        assert!(matches!(
            receiver.try_recv(),
            Ok(super::SessionCommand::Search { .. })
        ));
        super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            r#"{"token":1,"peer_username":"fresh","filename":"Other/Path/Song.flac","size":100}"#,
            &state,
        )
        .await
        .expect("ingest alternative");
        drop(receiver);

        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .expect("unavailable replacement response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("session manager is not running"),
            "{path}"
        );
        let transfers = state.transfers.read().await;
        assert_eq!(transfers.entries.len(), 1, "{path}");
        assert_eq!(transfers.entries[0].status, "failed", "{path}");
        assert_eq!(transfers.entries[0].bytes_transferred, 25, "{path}");
        assert_eq!(
            transfers.entries[0].reason.as_deref(),
            Some("peer offline"),
            "{path}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_replacements_roll_back_when_persistence_fails() {
    for (path, body) in [
        (
            "/api/transfers/downloads/replace",
            r#"{"transfer_id":1,"username":"fresh"}"#,
        ),
        (
            "/api/transfers/downloads/auto-replace",
            r#"{"transfer_id":1}"#,
        ),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state.session.write().await.state = "connected";
        super::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            r#"{"filename":"Remote/Album/Song.flac","peer_username":"stale","size":100}"#,
            &state,
        )
        .await
        .expect("create transfer");
        state.transfers.write().await.update_status(
            1,
            "failed",
            Some(25),
            Some("peer offline".to_owned()),
        );
        super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            r#"{"query":"song"}"#,
            &state,
        )
        .await
        .expect("create search");
        assert!(matches!(
            receiver.try_recv(),
            Ok(super::SessionCommand::Search { .. })
        ));
        super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            r#"{"token":1,"peer_username":"fresh","filename":"Other/Path/Song.flac","size":100}"#,
            &state,
        )
        .await
        .expect("ingest alternative");
        let (entries, next_id, next_token) = {
            let transfers = state.transfers.read().await;
            (
                transfers.entries.clone(),
                transfers.next_id,
                transfers.next_token,
            )
        };
        db.close_for_test().await;

        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .expect("persistence failure response");

        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        let transfers = state.transfers.read().await;
        assert_eq!(transfers.entries, entries, "{path}");
        assert_eq!(transfers.next_id, next_id, "{path}");
        assert_eq!(transfers.next_token, next_token, "{path}");
        assert!(receiver.try_recv().is_err(), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_start_rejects_peer_transfer_when_outbound_disabled() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_ALLOW_OUTBOUND", "false"));

    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Song.flac\",\"peer_username\":\"friend\",\"size\":10}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");

    let blocked = super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start transfer");
    assert_eq!(blocked.status, "409 Conflict");
    assert_eq!(
        blocked.body,
        "{\"error\":\"outbound transfers are disabled\"}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn peer_transfer_actions_reject_before_mutation_when_dispatch_is_unavailable() {
    for (action, initial_status, initial_reason) in [
        ("start", "queued", None),
        ("retry", "failed", Some("peer offline")),
    ] {
        let (state, receiver) = test_state();
        super::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            r#"{"filename":"Remote/Song.flac","peer_username":"friend","size":10}"#,
            &state,
        )
        .await
        .expect("create transfer");
        if action == "retry" {
            state.transfers.write().await.update_status(
                1,
                initial_status,
                Some(4),
                initial_reason.map(str::to_owned),
            );
        }
        drop(receiver);

        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/transfers/1/{action}"),
            None,
            "",
            &state,
        )
        .await
        .expect("unavailable transfer action response");
        assert_eq!(response.status, "503 Service Unavailable", "{action}");
        assert!(
            response.body.contains("session manager is not running"),
            "{action}"
        );
        let transfers = state.transfers.read().await;
        assert_eq!(transfers.entries[0].status, initial_status, "{action}");
        assert_eq!(
            transfers.entries[0].reason.as_deref(),
            initial_reason,
            "{action}"
        );
        assert_eq!(
            transfers.entries[0].bytes_transferred,
            if action == "retry" { 4 } else { 0 },
            "{action}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_create_rejects_upload_local_path() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-local-{}-ok.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write local file");
    let body = format!(
        "{{\"direction\":1,\"filename\":\"Remote/Song.flac\",\"local_path\":\"{}\"}}",
        super::json_escape(&path.display().to_string())
    );

    let created = super::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
        .await
        .expect("create transfer");
    assert_eq!(created.status, "400 Bad Request");
    assert!(created
        .body
        .contains("local_path is not accepted for uploads"));
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_create_rejects_missing_upload_local_path() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-local-{}-missing.bin",
        std::process::id()
    ));
    let body = format!(
        "{{\"direction\":1,\"filename\":\"Remote/Song.flac\",\"local_path\":\"{}\"}}",
        super::json_escape(&path.display().to_string())
    );

    let created = super::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
        .await
        .expect("create transfer");
    assert_eq!(created.status, "400 Bad Request");
    assert!(created
        .body
        .contains("local_path is not accepted for uploads"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn download_path_normalizes_protocol_separators_and_rejects_traversal() {
    let state_dir = PathBuf::from("state");
    let root = state_dir.join("downloads");
    let expected = root.join("Remote").join("Song.flac");
    assert_eq!(
        super::safe_download_path(&root, "Remote/Song.flac").unwrap(),
        expected
    );
    assert_eq!(
        super::safe_download_path(&root, "Remote\\Song.flac").unwrap(),
        expected
    );

    for filename in [
        "../outside.flac",
        "..\\outside.flac",
        "Remote/../../outside.flac",
        "Remote\\..\\..\\outside.flac",
        "/absolute.flac",
        "\\absolute.flac",
    ] {
        let error = super::safe_download_path(&root, filename)
            .expect_err("absolute and traversal paths must fail");
        assert!(error.contains("must be relative"), "{filename:?}: {error}");
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn completed_download_path_templates_reject_invalid_or_oversized_expansion() {
    let invalid_date = super::file_transfer_runtime::render_completed_download_path(
        "{date:%Q}",
        "friend",
        "Remote/Song.flac",
        None,
        None,
        1,
    )
    .expect_err("invalid date format must not reach Chrono display");
    assert!(invalid_date.contains("invalid date format"));

    let remote_folder = "x".repeat(super::MAX_TRANSFER_LOCAL_PATH_BYTES);
    let remote_filename = format!("{remote_folder}/Song.flac");
    let oversized = super::file_transfer_runtime::render_completed_download_path(
        "{remote_folder}/{remote_folder}",
        "friend",
        &remote_filename,
        None,
        None,
        1,
    )
    .expect_err("repeated tokens must remain bounded");
    assert!(oversized.contains("local path limit"));
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn download_path_rejects_final_symlink() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-download-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let root = super::file_transfer_runtime::download_root(&state_dir);
    let path = super::safe_download_path(&root, "Remote/Song.flac").expect("download path");
    let parent = path.parent().expect("download parent");
    std::fs::create_dir_all(parent).expect("download parent dir");
    let target = state_dir.join("outside.flac");
    std::fs::write(&target, [1_u8]).expect("target file");
    symlink(&target, &path).expect("download symlink");

    let error = super::ensure_scoped_download_path(&root, &path.display().to_string())
        .expect_err("symlink rejected");
    assert!(error.contains("must not be a symlink"));
    assert!(super::file_transfer_runtime::open_download_file(
        &super::file_transfer_runtime::download_root(&state_dir),
        &path
    )
    .is_err());

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn download_confined_open_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-download-parent-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let root = super::file_transfer_runtime::download_root(&state_dir);
    let outside = state_dir.join("outside");
    std::fs::create_dir_all(&root).expect("download root");
    std::fs::create_dir_all(&outside).expect("outside directory");
    symlink(&outside, root.join("Remote")).expect("symlinked parent");
    let path = root.join("Remote/Song.flac");

    assert!(super::file_transfer_runtime::open_download_file(&root, &path).is_err());
    assert!(!outside.join("Song.flac").exists());

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn download_parent_creation_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-download-parent-create-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let root = super::file_transfer_runtime::download_root(&state_dir);
    let outside = state_dir.join("outside");
    std::fs::create_dir_all(&root).expect("download root");
    std::fs::create_dir_all(&outside).expect("outside directory");
    symlink(&outside, root.join("Remote")).expect("symlinked parent");
    let path = root.join("Remote/New/Song.flac");

    let error = super::ensure_scoped_download_path(&root, &path.display().to_string())
        .expect_err("symlinked parent must be rejected before directory creation");
    assert!(error.contains("confined open failed"), "{error}");
    assert!(!outside.join("New").exists());

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn upload_share_lookup_rejects_swapped_symlink() {
    use std::os::unix::fs::symlink;

    let (state, _receiver) = test_state();
    let dir = std::env::temp_dir().join(format!(
        "slskr-upload-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("test dir");
    let shared_path = dir.join("shared.flac");
    let target_path = dir.join("outside.flac");
    std::fs::write(&shared_path, [1_u8, 2, 3, 4]).expect("shared file");
    std::fs::write(&target_path, [5_u8, 6, 7, 8]).expect("target file");
    add_test_share(&state, "Remote/Song.flac", &shared_path, 4).await;
    std::fs::remove_file(&shared_path).expect("remove shared file");
    symlink(&target_path, &shared_path).expect("swap symlink");

    assert!(super::find_shared_local_file(&state, "Remote/Song.flac")
        .await
        .is_none());
    assert!(super::open_shared_local_file(&state, &shared_path)
        .await
        .is_err());

    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn shared_file_confined_open_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-parent-symlink-test-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-share-parent-symlink-outside-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.flac"), b"secret").unwrap();
    symlink(&outside, root.join("album")).unwrap();

    let error = super::preview_stream_controller::open_shared_local_file_unix(
        std::slice::from_ref(&root),
        &root.join("album/secret.flac"),
    )
    .expect_err("symlinked share parent must be rejected");
    assert!(error.contains("confined open failed"));

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_sync_chunk_reads_remain_confined_to_share_roots() {
    use slskr_client::mesh_sync::{
        MeshMessageType, MeshReqChunkMessage, MeshSyncBase, MeshSyncMessage,
    };
    use std::os::unix::fs::symlink;

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-sync-share-root-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-mesh-sync-share-outside-{}-{unique}",
        std::process::id()
    ));
    let album = root.join("album");
    fs::create_dir_all(&album).expect("create mesh-sync share root");
    fs::create_dir_all(&outside).expect("create mesh-sync outside directory");
    let local_path = album.join("secret.flac");
    let inside_bytes = b"inside-mesh-sync";
    fs::write(&local_path, inside_bytes).expect("write mesh-sync shared file");
    fs::write(outside.join("secret.flac"), b"outside-mesh-sync")
        .expect("write mesh-sync outside file");

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let (filename, size) = {
        let shares = state.shares.read().await;
        shares
            .entries
            .iter()
            .find_map(|entry| {
                shares
                    .local_paths
                    .get(&entry.filename)
                    .filter(|path| path.as_path() == local_path.as_path())
                    .map(|_| (entry.filename.clone(), entry.size))
            })
            .expect("mesh-sync fixture must be indexed")
    };
    let flac_key = super::content_discovery::generate_flac_key(&filename, size);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[67; 32]);
    let mut request = MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key: flac_key.clone(),
        offset: 0,
        length: 6,
    });
    request
        .sign_at(&signing_key, super::unix_timestamp_millis() as i64)
        .expect("sign mesh-sync confined read request");
    let response = super::mesh_sync::handle_signed_message(&state, "mesh-peer", request)
        .await
        .expect("mesh-sync response before path swap");
    match response {
        MeshSyncMessage::RespChunk(message) => assert!(message.success),
        other => panic!("unexpected mesh-sync response before path swap: {other:?}"),
    }

    fs::rename(&album, root.join("album-real")).expect("move original mesh-sync directory");
    symlink(&outside, &album).expect("swap mesh-sync parent with symlink");
    let mut swapped_request = MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key,
        offset: 0,
        length: 6,
    });
    swapped_request
        .sign_at(&signing_key, super::unix_timestamp_millis() as i64)
        .expect("sign swapped mesh-sync read request");
    let response = super::mesh_sync::handle_signed_message(&state, "mesh-peer", swapped_request)
        .await
        .expect("mesh-sync response after path swap");
    match response {
        MeshSyncMessage::RespChunk(message) => {
            assert!(!message.success, "swapped share parent must fail closed");
            assert!(message.data_base64.is_empty());
        }
        other => panic!("unexpected mesh-sync response after path swap: {other:?}"),
    }

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
    let _ = fs::remove_dir_all(state.config.state_dir.clone());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_start_with_peer_requests_peer_address() {
    let (state, mut receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-start-{}-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write shared file");
    add_test_share(&state, "Remote/Song.flac", &path, 4).await;
    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"direction\":1,\"peer_username\":\"friend\",\"filename\":\"Remote/Song.flac\",\"size\":4}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");
    assert!(created.body.contains("\"token\":1"));

    let started = super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start transfer");

    assert_eq!(started.status, "200 OK");
    assert!(started.body.contains("\"status\":\"peer_lookup\""));
    assert_eq!(
        receiver.try_recv().expect("transfer command"),
        super::SessionCommand::TransferPeer {
            id: 1,
            username: "friend".to_owned(),
        }
    );
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn peer_address_response_negotiates_pending_transfer() {
    let (state, _receiver) = test_state();
    let token = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            None,
            Some(4),
        );
        let token = entry.token;
        transfers.update_status(entry.id, "peer_lookup", None, None);
        token
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token,
                filename: "Remote/Song.flac".to_owned(),
                size: Some(4),
            })
        );
        peer.send(&super::PeerMessage::TransferResponse(
            super::TransferResponse::Allowed {
                token,
                size: Some(4),
            },
        ))
        .await
        .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    super::project_peer_transfer_response(&state, &address).await;
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "accepted");
    assert_eq!(record.size, Some(4));
    assert_eq!(record.reason, None);
    assert!(transfers.stats_json().contains("\"in_progress\":1"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn queued_peer_download_adopts_the_remote_upload_token() {
    let (state, _receiver) = test_state();
    let path = super::safe_download_path(&state.config.downloads_dir, "Remote/Queued.flac")
        .expect("download path");
    let request_token = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Queued.flac".to_owned(),
            Some(path.display().to_string()),
            Some(4),
        );
        let token = entry.token;
        transfers.update_status(entry.id, "peer_lookup", None, None);
        token
    };
    let upload_token = request_token.saturating_add(76);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        init.receive().await.expect("init");
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("download request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 0,
                token: request_token,
                filename: "Remote/Queued.flac".to_owned(),
                size: None,
            })
        );
        peer.send(&super::PeerMessage::TransferResponse(
            super::TransferResponse::Rejected {
                token: request_token,
                reason: "Queued".to_owned(),
            },
        ))
        .await
        .expect("queued response");
        peer.send(&super::PeerMessage::TransferRequest(
            super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token: upload_token,
                filename: "Remote/Queued.flac".to_owned(),
                size: Some(4),
            },
        ))
        .await
        .expect("upload request");
        assert_eq!(
            peer.receive().await.expect("upload acknowledgement"),
            super::PeerMessage::TransferResponse(super::TransferResponse::Allowed {
                token: upload_token,
                size: Some(4),
            })
        );
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    super::project_peer_transfer_response(&state, &address).await;
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "accepted");
    assert_eq!(record.token, upload_token);
    assert_eq!(record.size, Some(4));
    assert_eq!(record.reason, None);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn peer_transfer_request_reuses_remembered_legacy_encoding() {
    let (state, _receiver) = test_state();
    let filename = "Музыка/песня.flac";
    let token = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            filename.to_owned(),
            None,
            Some(4),
        );
        transfers.update_status(entry.id, "peer_lookup", None, None);
        entry.token
    };
    state.remote_path_encodings.write().await.remember(
        "friend",
        filename,
        super::ProtocolTextEncoding::Windows1251,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        init.receive().await.expect("init");
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: super::ProtocolTextEncoding::Windows1251,
                direction: 0,
                token,
                filename: filename.to_owned(),
                size: None,
            })
        );
        peer.send(&super::PeerMessage::TransferResponse(
            super::TransferResponse::Allowed {
                token,
                size: Some(4),
            },
        ))
        .await
        .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    super::project_peer_transfer_response(&state, &address).await;
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    assert_eq!(transfers.entries.first().unwrap().status, "accepted");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn peer_address_response_uploads_accepted_local_file_transfer() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write upload file");
    add_test_share(&state, "Remote/Song.flac", &path, 4).await;
    let token = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some(path.display().to_string()),
            Some(4),
        );
        let token = entry.token;
        transfers.update_status(entry.id, "peer_lookup", None, None);
        token
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept peer-message");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("peer-message init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token,
                filename: "Remote/Song.flac".to_owned(),
                size: Some(4),
            })
        );
        peer.send(&super::PeerMessage::TransferResponse(
            super::TransferResponse::Allowed {
                token,
                size: Some(4),
            },
        ))
        .await
        .expect("transfer response");

        let (stream, _) = listener.accept().await.expect("accept file-transfer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("file init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        assert_eq!(file.receive_token().await.expect("token"), token);
        file.send_offset(1).await.expect("offset");
        file.read_chunk(3).await.expect("chunk")
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    super::project_peer_transfer_response(&state, &address).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![2, 3, 4]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded", "reason={:?}", record.reason);
    assert_eq!(record.bytes_transferred, 4);
    assert_eq!(record.size, Some(4));
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn peer_address_response_downloads_accepted_file_transfer_with_resume() {
    let (state, _receiver) = test_state();
    let downloads_dir = state.config.downloads_dir.display().to_string();
    if let Some(destination) = state
        .destinations
        .write()
        .await
        .records
        .iter_mut()
        .find(|destination| destination.is_default)
    {
        destination.path = downloads_dir;
    }
    let path = super::safe_download_path(&state.config.downloads_dir, "Remote/Song.flac")
        .expect("download path");
    std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
    std::fs::write(&path, *b"f").expect("write partial download file");
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some(path.display().to_string()),
            Some(4),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "peer_lookup", Some(1), None);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept peer-message");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("peer-message init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 0,
                token: 1,
                filename: "Remote/Song.flac".to_owned(),
                size: None,
            })
        );
        peer.send(&super::PeerMessage::TransferResponse(
            super::TransferResponse::Allowed {
                token: 1,
                size: Some(4),
            },
        ))
        .await
        .expect("transfer response");

        let (stream, _) = listener.accept().await.expect("accept file-transfer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("file init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        file.send_token(1).await.expect("token");
        assert_eq!(file.receive_offset().await.expect("offset"), 1);
        file.write_chunk(b"LaC").await.expect("chunk");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    super::project_peer_transfer_response(&state, &address).await;
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded", "reason={:?}", record.reason);
    assert_eq!(record.bytes_transferred, 4);
    assert_eq!(record.size, Some(4));
    assert_eq!(std::fs::read(&path).expect("download file"), b"fLaC");
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn queued_outgoing_upload_accepts_remote_download_resume_request() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-queued-upload-{}-resume.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write upload file");
    add_test_share(&state, "Remote/QueuedUpload.flac", &path, 4).await;
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/QueuedUpload.flac".to_owned(),
            Some(path.display().to_string()),
            Some(4),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "queued", None, Some("Queued".to_owned()));
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server_state = state.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept peer-message");
        super::handle_plain_peer_messages(
            &server_state,
            slskr_client::stream::PeerMessageConnection::new(stream),
            Some("friend".to_owned()),
        )
        .await
    });

    let stream = tokio::net::TcpStream::connect(local_addr)
        .await
        .expect("connect peer-message");
    let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
    peer.send(&super::PeerMessage::TransferRequest(
        super::TransferRequest {
            filename_encoding: Default::default(),
            direction: 0,
            token: 77,
            filename: "Remote/QueuedUpload.flac".to_owned(),
            size: None,
        },
    ))
    .await
    .expect("send resume request");
    assert_eq!(
        peer.receive().await.expect("resume response"),
        super::PeerMessage::TransferResponse(super::TransferResponse::Allowed {
            token: 77,
            size: Some(4),
        })
    );
    drop(peer);
    server.await.expect("server task").expect("server result");

    let transfers = state.transfers.read().await;
    assert_eq!(transfers.entries.len(), 1);
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.direction, 1);
    assert_eq!(record.token, 77);
    assert_eq!(record.status, "accepted");
    assert_eq!(record.size, Some(4));
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn queue_upload_and_place_in_queue_messages_drive_the_group_scheduler() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_FROZEN_TRANSFER_UPLOAD_JSON", r#"{"slots":1}"#),
    );
    let path = std::env::temp_dir().join(format!(
        "slskr-queue-upload-message-{}.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).unwrap();
    add_test_share(&state, "Remote/Queued.flac", &path, 4).await;
    {
        let mut transfers = state.transfers.write().await;
        let blocker = transfers.create(
            1,
            Some("busy".to_owned()),
            "Remote/Busy.flac".to_owned(),
            Some(path.display().to_string()),
            Some(4),
        );
        transfers.update_status(blocker.id, "in_progress", None, None);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server_state = state.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        super::handle_plain_peer_messages(
            &server_state,
            slskr_client::stream::PeerMessageConnection::new(stream),
            Some("friend".to_owned()),
        )
        .await
    });
    let stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
    peer.send(&super::PeerMessage::QueueUpload {
        filename: "Remote/Queued.flac".to_owned(),
    })
    .await
    .unwrap();
    drop(peer);
    server.await.unwrap().unwrap();

    let queued = {
        let transfers = state.transfers.read().await;
        transfers
            .entries
            .iter()
            .find(|entry| entry.peer_username.as_deref() == Some("friend"))
            .cloned()
            .unwrap()
    };
    assert_eq!(queued.status, "queued");
    assert_eq!(queued.reason.as_deref(), Some("Queued"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server_state = state.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        super::handle_plain_peer_messages(
            &server_state,
            slskr_client::stream::PeerMessageConnection::new(stream),
            Some("friend".to_owned()),
        )
        .await
    });
    let stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
    peer.send(&super::PeerMessage::PlaceInQueueRequest {
        filename: "Remote/Queued.flac".to_owned(),
    })
    .await
    .unwrap();
    assert_eq!(
        peer.receive().await.unwrap(),
        super::PeerMessage::PlaceInQueueResponse {
            filename: "Remote/Queued.flac".to_owned(),
            // Frozen slskd's round-robin estimator reports the first file
            // in a user's local queue as zero even while another user is active.
            place: 0,
        }
    );
    drop(peer);
    server.await.unwrap().unwrap();
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn prefer_mode_uses_obfuscated_file_transfer_when_available() {
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
        super::SearchStore::new(),
        None,
    );
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-obfuscated-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [5_u8, 6, 7]).expect("write upload file");
    add_test_share(&state, "Remote/Obfuscated.flac", &path, 3).await;
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Obfuscated.flac".to_owned(),
            Some(path.display().to_string()),
            Some(3),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "peer_lookup", None, None);
    }

    let listener = slskr_client::listener::Listener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let unused_regular = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("unused regular listener");
        listener.local_addr().expect("unused regular addr").port()
    };
    let server = tokio::spawn(async move {
        let (incoming, _) = listener.accept_obfuscated().await.expect("accept p");
        let slskr_client::listener::IncomingConnection::ObfuscatedPeerMessages(mut peer) = incoming
        else {
            panic!("expected obfuscated peer messages");
        };
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token: 1,
                filename: "Remote/Obfuscated.flac".to_owned(),
                size: Some(3),
            })
        );
        peer.send(&super::PeerMessage::TransferResponse(
            super::TransferResponse::Allowed {
                token: 1,
                size: Some(3),
            },
        ))
        .await
        .expect("transfer response");

        let (incoming, _) = listener.accept_obfuscated().await.expect("accept f");
        let slskr_client::listener::IncomingConnection::PeerInit {
            username,
            kind,
            token,
            stream,
            obfuscated,
        } = incoming
        else {
            panic!("expected obfuscated file-transfer peer init");
        };
        assert_eq!(username, "tester");
        assert_eq!(kind, super::ConnectionKind::FileTransfer);
        assert_eq!(token, 0);
        assert!(obfuscated);
        let mut file = slskr_client::file_transfer::FileTransferConnection::new_obfuscated(stream);
        assert_eq!(file.receive_token().await.expect("token"), 1);
        file.send_offset(0).await.expect("offset");
        file.read_chunk(3).await.expect("chunk")
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(unused_regular),
        obfuscation_type: super::ROTATED_OBFUSCATION_TYPE,
        obfuscated_port: local_addr.port(),
    };

    super::project_peer_transfer_response(&state, &address).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![5, 6, 7]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded");
    assert_eq!(record.bytes_transferred, 3);
    assert_eq!(record.size, Some(3));
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn peer_address_response_falls_back_to_plain_file_transfer_when_obfuscated_fails() {
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
        super::SearchStore::new(),
        None,
    );
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-obfuscated-fallback-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [8_u8, 9]).expect("write upload file");
    add_test_share(&state, "Remote/Fallback.flac", &path, 2).await;
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Fallback.flac".to_owned(),
            Some(path.display().to_string()),
            Some(2),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "peer_lookup", None, None);
    }

    let unused_obfuscated = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("unused listener");
        listener.local_addr().expect("unused addr").port()
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("plain listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept p");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("peer init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token: 1,
                filename: "Remote/Fallback.flac".to_owned(),
                size: Some(2),
            })
        );
        peer.send(&super::PeerMessage::TransferResponse(
            super::TransferResponse::Allowed {
                token: 1,
                size: Some(2),
            },
        ))
        .await
        .expect("transfer response");

        let (stream, _) = listener.accept().await.expect("accept f");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("file init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        assert_eq!(file.receive_token().await.expect("token"), 1);
        file.send_offset(0).await.expect("offset");
        file.read_chunk(2).await.expect("chunk")
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: super::ROTATED_OBFUSCATION_TYPE,
        obfuscated_port: unused_obfuscated,
    };

    super::project_peer_transfer_response(&state, &address).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![8, 9]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded");
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn indirect_transfer_command_requests_connect_to_peer() {
    let (state, mut receiver) = test_state();
    super::session_runtime::try_send_session_command(
        &state,
        super::SessionCommand::IndirectTransfer {
            id: 7,
            username: "friend".to_owned(),
            token: 42,
        },
    )
    .expect("queue indirect transfer command");

    assert_eq!(
        receiver.try_recv().expect("indirect command"),
        super::SessionCommand::IndirectTransfer {
            id: 7,
            username: "friend".to_owned(),
            token: 42,
        }
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn connect_to_peer_response_executes_indirect_file_upload() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-indirect-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [8_u8, 9, 10]).expect("write upload file");
    add_test_share(&state, "Remote/Indirect.flac", &path, 3).await;
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Indirect.flac".to_owned(),
            Some(path.display().to_string()),
            Some(3),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "indirect_pending", None, None);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept indirect");
        let incoming = slskr_client::listener::demux_incoming(stream)
            .await
            .expect("demux indirect");
        let slskr_client::listener::IncomingConnection::PierceFirewall { token, stream } = incoming
        else {
            panic!("expected pierce firewall");
        };
        assert_eq!(token, 1);
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(stream);
        assert_eq!(file.receive_token().await.expect("token"), 1);
        file.send_offset(1).await.expect("offset");
        file.read_chunk(2).await.expect("chunk")
    });
    let response = super::ConnectToPeerResponse {
        username: "friend".to_owned(),
        connection_type: "F".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        token: 1,
        privileged: false,
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    super::project_indirect_transfer_response(&state, &response).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![9, 10]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded");
    assert_eq!(record.bytes_transferred, 3);
    assert_eq!(record.size, Some(3));
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn connect_to_peer_response_executes_indirect_browse() {
    let (state, _receiver) = test_state();
    {
        let mut browse = state.browse.write().await;
        browse.request("friend".to_owned());
        assert_eq!(
            browse.mark_indirect_pending("friend", "direct failed".to_owned()),
            Some(1)
        );
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept indirect");
        let incoming = slskr_client::listener::demux_incoming(stream)
            .await
            .expect("demux indirect");
        let slskr_client::listener::IncomingConnection::PierceFirewall { token, stream } = incoming
        else {
            panic!("expected pierce firewall");
        };
        assert_eq!(token, 1);
        let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
        assert_eq!(
            peer.receive().await.expect("browse request"),
            super::PeerMessage::GetShareFileList
        );
        let entries =
            crate::config::parse_share_entries("Remote/Indirect.flac=55").expect("entries");
        let payload = super::build_shared_file_list_payload(&entries).expect("payload");
        peer.send(&super::PeerMessage::SharedFileListResponse(payload))
            .await
            .expect("response");
    });
    let response = super::ConnectToPeerResponse {
        username: "friend".to_owned(),
        connection_type: "P".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        token: 1,
        privileged: false,
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    super::project_indirect_browse_response(&state, &response).await;
    server.await.expect("server task");

    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "ready");
    assert_eq!(record.indirect_token, None);
    assert_eq!(record.entries.len(), 1);
    assert_eq!(record.entries[0].filename, "Remote/Indirect.flac");
    assert_eq!(record.entries[0].size, 55);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn inbound_transfer_request_serves_shared_file_over_pierce_firewall() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-inbound-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write shared file");
    {
        let mut shares = state.shares.write().await;
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Virtual/Inbound.flac".to_owned(),
            size: 4,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
        shares
            .local_paths
            .insert("Virtual/Inbound.flac".to_owned(), path.clone());
    }

    assert!(
        super::find_shared_local_file(&state, r"Virtual\Inbound.flac")
            .await
            .is_some()
    );

    super::handle_peer_message(
        &state,
        super::PeerMessage::TransferRequest(super::TransferRequest {
            filename_encoding: Default::default(),
            direction: 0,
            token: 7,
            filename: r"Virtual\Inbound.flac".to_owned(),
            size: None,
        }),
        None,
        |response| async move {
            assert_eq!(
                response,
                super::PeerMessage::TransferResponse(super::TransferResponse::Allowed {
                    token: 7,
                    size: Some(4),
                },)
            );
            Ok(())
        },
    )
    .await
    .expect("transfer request");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let inbound_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept file-transfer");
        super::handle_inbound_file_transfer(
            &inbound_state,
            slskr_client::file_transfer::FileTransferConnection::new(stream),
            Some(7),
        )
        .await
        .expect("serve inbound file");
    });
    let stream = tokio::net::TcpStream::connect(local_addr)
        .await
        .expect("connect file-transfer");
    let mut file = slskr_client::file_transfer::FileTransferConnection::new(stream);
    assert_eq!(file.receive_token().await.expect("token"), 7);
    file.send_offset(2).await.expect("offset");
    assert_eq!(file.read_chunk(2).await.expect("chunk"), vec![3, 4]);
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded");
    // Progress tracks the remote position, including the resumed two-byte offset.
    assert_eq!(record.bytes_transferred, 4);
    assert_eq!(record.size, Some(4));
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn daemon_outbound_capability_probe_requires_matching_signed_acknowledgement() {
    use ed25519_dalek::SigningKey;

    let (state, _receiver) = test_state();
    state
        .advanced_networking
        .write()
        .await
        .mesh
        .enable_soulseek_rendezvous = true;
    let remote_key = SigningKey::from_bytes(&[9_u8; 32]);
    let remote_descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "remote-peer",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        Vec::new(),
        std::time::Duration::from_secs(300),
        &remote_key,
        std::time::SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.with_overlay_port(Some(50_305)).sign(&remote_key))
    .expect("signed remote descriptor");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local address");
    let server_descriptor = remote_descriptor.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("peer init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        let hello = peer.receive().await.expect("capability hello");
        let hello = slskr_client::capabilities::decode_peer_capability_message(&hello)
            .expect("decode hello")
            .expect("capability envelope");
        assert_eq!(
            hello.message_type,
            slskr_client::capabilities::PeerCapabilityMessageType::Hello
        );
        hello
            .descriptor
            .verify(std::time::SystemTime::now())
            .expect("verify local descriptor");
        let acknowledgement = slskr_client::capabilities::peer_capability_message(
            &slskr_client::capabilities::PeerCapabilityEnvelope::new(
                slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge,
                hello.nonce,
                server_descriptor,
            ),
        )
        .expect("acknowledgement");
        peer.send(&acknowledgement)
            .await
            .expect("send acknowledgement");
    });
    super::remember_peer_endpoint(
        &state,
        slskr_client::protocol::server::PeerAddress {
            username: "remote-peer".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    let observed = super::probe_peer_capability(&state, "remote-peer")
        .await
        .expect("capability probe");
    server.await.expect("server task");
    assert_eq!(observed.username, "remote-peer");
    assert_eq!(observed.peer_id, remote_descriptor.peer_id);
    assert_eq!(observed.overlay_port, Some(50_305));
    assert_eq!(state.mesh.read().await.capability_records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn peer_endpoint_cache_does_not_alias_case_distinct_usernames() {
    let (state, _receiver) = test_state();
    super::remember_peer_endpoint(
        &state,
        slskr_client::protocol::server::PeerAddress {
            username: "CasePeer".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: 40_001,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    assert!(super::cached_peer_endpoint(&state, "CasePeer")
        .await
        .is_some());
    assert!(super::cached_peer_endpoint(&state, "casepeer")
        .await
        .is_none());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn daemon_ingests_verified_peer_capabilities_and_acknowledges_hello() {
    use ed25519_dalek::SigningKey;

    let (state, _receiver) = test_state();
    let signing_key = SigningKey::from_bytes(&[8_u8; 32]);
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "mesh-source",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        vec!["https://mesh.example/content".to_owned()],
        std::time::Duration::from_secs(300),
        &signing_key,
        std::time::SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.sign(&signing_key))
    .expect("signed capability descriptor");
    let nonce = "03030303030303030303030303030303";
    let hello = slskr_client::capabilities::peer_capability_message(
        &slskr_client::capabilities::PeerCapabilityEnvelope::new(
            slskr_client::capabilities::PeerCapabilityMessageType::Hello,
            nonce,
            descriptor.clone(),
        ),
    )
    .expect("capability hello");
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();

    super::handle_peer_message(&state, hello, Some("mesh-source"), |response| async move {
        response_tx
            .send(response)
            .map_err(|_| "capability response receiver closed".to_owned())
    })
    .await
    .expect("ingest capability hello");

    let response = response_rx.await.expect("capability acknowledgement");
    let acknowledgement = slskr_client::capabilities::decode_peer_capability_message(&response)
        .expect("decode acknowledgement")
        .expect("capability envelope");
    assert_eq!(
        acknowledgement.message_type,
        slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge
    );
    assert_eq!(acknowledgement.nonce, nonce);
    acknowledgement
        .descriptor
        .verify(std::time::SystemTime::now())
        .expect("verify local acknowledgement descriptor");
    let mesh = state.mesh.read().await;
    assert_eq!(mesh.capability_records.len(), 1);
    assert_eq!(mesh.capability_records[0].username, "mesh-source");
    assert_eq!(mesh.capability_records[0].peer_id, descriptor.peer_id);
    assert_eq!(mesh.capability_records[0].features, descriptor.features);
    assert!(
        mesh.capability_records[0].expires_at_unix
            <= super::unix_timestamp() + super::PEER_CAPABILITY_LEASE_SECONDS
    );
    drop(mesh);

    let replay = slskr_client::capabilities::peer_capability_message(
        &slskr_client::capabilities::PeerCapabilityEnvelope::new(
            slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge,
            "replayed-under-another-user",
            descriptor.clone(),
        ),
    )
    .expect("replayed capability message");
    let replay_error =
        super::handle_peer_message(&state, replay, Some("attacker"), |_| async { Ok(()) })
            .await
            .expect_err("reject capability peer ID alias");
    assert!(replay_error.contains("already registered to another username"));

    let mut tampered = descriptor;
    tampered.signature = Some([0_u8; 64]);
    let invalid = slskr_client::capabilities::peer_capability_message(
        &slskr_client::capabilities::PeerCapabilityEnvelope::new(
            slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge,
            "04040404040404040404040404040404",
            tampered,
        ),
    )
    .expect("tampered capability message");
    let error =
        super::handle_peer_message(&state, invalid, Some("mesh-source"), |_| async { Ok(()) })
            .await
            .expect_err("reject invalid descriptor");
    assert!(error.contains("signature is invalid"));
    assert_eq!(state.mesh.read().await.capability_records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn inbound_transfer_request_rejects_when_active_limit_is_full() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_MAX_ACTIVE", "1"));
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(1, None, "Remote/Busy.flac".to_owned(), None, Some(1));
        transfers.update_status(entry.id, "in_progress", None, None);
    }

    super::handle_peer_message(
        &state,
        super::PeerMessage::TransferRequest(super::TransferRequest {
            filename_encoding: Default::default(),
            direction: 0,
            token: 99,
            filename: "Virtual/Test.flac".to_owned(),
            size: None,
        }),
        None,
        |response| async move {
            assert_eq!(
                response,
                super::PeerMessage::TransferResponse(super::TransferResponse::Rejected {
                    token: 99,
                    reason: "transfer limit reached".to_owned(),
                },)
            );
            Ok(())
        },
    )
    .await
    .expect("reject transfer request");

    let transfers = state.transfers.read().await;
    assert_eq!(transfers.entries.len(), 2);
    let rejected = transfers.entries.get(1).expect("rejection");
    assert_eq!(rejected.status, "rejected");
    assert_eq!(rejected.reason.as_deref(), Some("transfer limit reached"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn inbound_transfer_request_rejects_when_inbound_disabled() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_ALLOW_INBOUND", "false"));

    super::handle_peer_message(
        &state,
        super::PeerMessage::TransferRequest(super::TransferRequest {
            filename_encoding: Default::default(),
            direction: 0,
            token: 55,
            filename: "Virtual/Test.flac".to_owned(),
            size: None,
        }),
        None,
        |response| async move {
            assert_eq!(
                response,
                super::PeerMessage::TransferResponse(super::TransferResponse::Rejected {
                    token: 55,
                    reason: "inbound transfers are disabled".to_owned(),
                },)
            );
            Ok(())
        },
    )
    .await
    .expect("reject transfer request");

    let transfers = state.transfers.read().await;
    let rejected = transfers.entries.first().expect("rejection");
    assert_eq!(rejected.status, "rejected");
    assert_eq!(
        rejected.reason.as_deref(),
        Some("inbound transfers are disabled")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_api_rejects_missing_filename() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request("POST", "/api/v0/transfers", None, "{}", &state)
        .await
        .expect("bad transfer");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(response.body, "{\"error\":\"filename is required\"}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn user_store_rejects_new_records_at_limit_but_updates_existing_users() {
    let mut users = super::UserStore::with_max_records(1);
    users.watch("alice".to_owned()).unwrap();

    assert!(users.watch("bob".to_owned()).is_none());
    let updated = users
        .apply_status(&super::UserStatus {
            username: "alice".to_owned(),
            status: 2,
            privileged: false,
        })
        .unwrap();
    assert_eq!(updated.status.as_deref(), Some("2"));
    assert!(users
        .apply_status(&super::UserStatus {
            username: "remote-unique".to_owned(),
            status: 1,
            privileged: false,
        })
        .is_none());
    assert_eq!(users.records.len(), 1);
    assert_eq!(users.records[0].username, "alice");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn user_store_bounds_and_normalizes_retained_usernames() {
    let oversized_username = "é".repeat(super::MAX_USER_USERNAME_BYTES);
    let mut users = super::UserStore::with_max_records(2);
    let watched = users.watch(oversized_username.clone()).unwrap();
    assert!(watched.username.len() <= super::MAX_USER_USERNAME_BYTES);

    let updated = users
        .apply_status(&super::UserStatus {
            username: oversized_username.clone(),
            status: 2,
            privileged: false,
        })
        .unwrap();
    assert_eq!(updated.status.as_deref(), Some("2"));
    assert_eq!(users.records.len(), 1);

    let unwatched = users.unwatch(&oversized_username).unwrap();
    assert!(!unwatched.watched);
    assert_eq!(users.records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn users_api_watches_lists_and_unwatches_users() {
    let (state, mut receiver) = test_state();

    let watched = super::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        "{\"username\":\"friend\"}",
        &state,
    )
    .await
    .expect("watch user");
    assert_eq!(watched.status, "201 Created");
    assert!(watched.body.contains("\"username\":\"friend\""));
    assert!(watched.body.contains("\"watched\":true"));
    assert_eq!(
        receiver.try_recv().expect("watch command"),
        super::SessionCommand::WatchUser("friend".to_owned())
    );

    let listed = super::route_http_request("GET", "/api/v0/users", None, "", &state)
        .await
        .expect("list users");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));

    let stats_request = super::route_http_request(
        "POST",
        "/api/v1/users/friend/stats/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request user stats");
    assert_eq!(stats_request.status, "202 Accepted");
    assert_eq!(
        receiver.try_recv().expect("stats command"),
        super::SessionCommand::RequestUserStats("friend".to_owned())
    );

    {
        let mut users = state.users.write().await;
        users.apply_stats(
            "friend".to_owned(),
            &super::UserStats {
                average_speed: 1234,
                upload_count: 5,
                unknown: 0,
                file_count: 42,
                directory_count: 7,
            },
        );
    }
    let listed = super::route_http_request("GET", "/api/v0/users", None, "", &state)
        .await
        .expect("list users with stats");
    assert!(listed.body.contains("\"average_speed\":1234"));
    assert!(listed.body.contains("\"upload_count\":5"));
    assert!(listed.body.contains("\"file_count\":42"));
    assert!(listed.body.contains("\"directory_count\":7"));

    let unwatched =
        super::route_http_request("DELETE", "/api/v2/users/friend/watch", None, "", &state)
            .await
            .expect("unwatch user");
    assert_eq!(unwatched.status, "200 OK");
    assert!(unwatched.body.contains("\"watched\":false"));
    assert_eq!(
        receiver.try_recv().expect("unwatch command"),
        super::SessionCommand::UnwatchUser("friend".to_owned())
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn user_watch_routes_reject_before_mutation_when_dispatch_is_unavailable() {
    let (state, receiver) = test_state();
    drop(receiver);
    let watched = super::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("unavailable watch response");
    assert_eq!(watched.status, "503 Service Unavailable");
    assert!(watched.body.contains("session manager is not running"));
    assert!(state.users.read().await.records.is_empty());

    let (state, receiver) = test_state();
    state
        .users
        .write()
        .await
        .watch("friend".to_owned())
        .unwrap();
    drop(receiver);
    let unwatched =
        super::route_http_request("DELETE", "/api/v2/users/friend/watch", None, "", &state)
            .await
            .expect("unavailable unwatch response");
    assert_eq!(unwatched.status, "503 Service Unavailable");
    assert!(unwatched.body.contains("session manager is not running"));
    let users = state.users.read().await;
    assert_eq!(users.records.len(), 1);
    assert!(users.records[0].watched);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn user_watch_routes_roll_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let watched = super::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("failed watch persistence response");
    assert_eq!(watched.status, "503 Service Unavailable");
    assert!(watched.body.contains("user projection persistence failed"));
    assert!(state.users.read().await.records.is_empty());
    assert!(receiver.try_recv().is_err());

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .users
        .write()
        .await
        .watch("friend".to_owned())
        .unwrap();
    db.close_for_test().await;
    let unwatched =
        super::route_http_request("DELETE", "/api/v2/users/friend/watch", None, "", &state)
            .await
            .expect("failed unwatch persistence response");
    assert_eq!(unwatched.status, "503 Service Unavailable");
    assert!(unwatched
        .body
        .contains("user projection persistence failed"));
    assert!(state.users.read().await.records[0].watched);
    assert!(receiver.try_recv().is_err());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn native_user_group_projects_transfer_group_memberships_and_live_user_classification() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"leechers":{"thresholds":{"files":2,"directories":2}},"blacklisted":{"members":["blocked"]},"user_defined":{"trusted":{"upload":{"priority":10},"members":["friend"]}}}"#,
            ),
    );

    let group = super::route_http_request("GET", "/api/users/friend/group", None, "", &state)
        .await
        .expect("user group");
    assert_eq!(group.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<String>(&group.body).unwrap(),
        "trusted"
    );

    let unknown =
        super::route_http_request("GET", "/api/v0/users/stranger/group", None, "", &state)
            .await
            .expect("unknown user group");
    assert_eq!(
        serde_json::from_str::<String>(&unknown.body).unwrap(),
        "default"
    );

    let groups = super::route_http_request(
        "GET",
        "/api/v0/users/groups?UserNames=%20friend%20&usernames=FRIEND&usernames=stranger&usernames=",
        None,
        "",
        &state,
    )
    .await
    .expect("user group batch");
    assert_eq!(groups.status, "200 OK");
    let groups_json = serde_json::from_str::<serde_json::Value>(&groups.body).unwrap();
    assert_eq!(groups_json.as_object().unwrap().len(), 2);
    assert_eq!(groups_json["friend"], "trusted");
    assert_eq!(groups_json["stranger"], "default");

    // Matches the oracle's cache-only UserService.GetGroup: an unknown
    // username remains in the default group until its user record exists.
    let blocked = super::route_http_request("GET", "/api/v0/users/blocked/group", None, "", &state)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&blocked.body).unwrap(),
        "default"
    );

    {
        let mut users = state.users.write().await;
        users.apply_stats(
            "leecher".to_owned(),
            &super::UserStats {
                average_speed: 1,
                upload_count: 0,
                unknown: 0,
                file_count: 1,
                directory_count: 10,
            },
        );
        users.apply_status(&super::UserStatus {
            username: "supporter".to_owned(),
            status: 2,
            privileged: true,
        });
        // Matches the oracle's real precedence: IsBlacklisted is
        // checked before privileged status, so a blacklisted user
        // stays blacklisted even if the Soulseek server also reports
        // them as privileged.
        users.apply_status(&super::UserStatus {
            username: "blocked".to_owned(),
            status: 2,
            privileged: true,
        });
    }
    let blocked_but_privileged =
        super::route_http_request("GET", "/api/v0/users/blocked/group", None, "", &state)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&blocked_but_privileged.body).unwrap(),
        "blacklisted"
    );
    let leecher = super::route_http_request("GET", "/api/v0/users/leecher/group", None, "", &state)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&leecher.body).unwrap(),
        "leechers"
    );
    let privileged =
        super::route_http_request("GET", "/api/v0/users/supporter/group", None, "", &state)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&privileged.body).unwrap(),
        "privileged"
    );

    let too_many = (0..=super::MAX_USER_GROUP_BATCH)
        .map(|index| format!("usernames=user-{index}"))
        .collect::<Vec<_>>()
        .join("&");
    let rejected = super::route_http_request(
        "GET",
        &format!("/api/v0/users/groups?{too_many}"),
        None,
        "",
        &state,
    )
    .await
    .expect("bounded user group batch");
    assert_eq!(rejected.status, "400 Bad Request");

    let (controller_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    for path in [
        "/api/v0/users/friend/group",
        "/api/v0/users/groups?usernames=friend",
    ] {
        let response = super::route_http_request("GET", path, None, "", &controller_state)
            .await
            .unwrap();
        assert_eq!(response.status, "404 Not Found", "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_group_runtime_enforces_membership_limits_slots_file_types_and_speed() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with(
                "SLSKR_FROZEN_TRANSFER_UPLOAD_JSON",
                r#"{"slots":2,"speed_limit":100,"limits":{"queued":{"files":1}}}"#,
            )
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"user_defined":{"friends":{"upload":{"priority":5,"slots":2,"speed_limit":50,"allowed_file_types":[".flac"],"limits":{"queued":{"files":2}}},"members":["alice"]}}}"#,
            ),
    );

    assert_eq!(
        super::file_transfer_runtime::effective_transfer_group(&state, "alice").await,
        "friends"
    );
    assert_eq!(
        super::effective_upload_speed_limit(&state, "alice").await,
        50
    );
    assert_eq!(
        super::effective_upload_speed_limit(&state, "stranger").await,
        100
    );
    assert_eq!(
        super::file_transfer_runtime::inbound_upload_policy(&state, "alice", "Music/track.mp3", 10)
            .await
            .unwrap_err(),
        "File type .mp3 is not permitted."
    );

    {
        let mut transfers = state.transfers.write().await;
        let alice = transfers.create(
            1,
            Some("alice".to_owned()),
            "Music/first.flac".to_owned(),
            Some("/tmp/first.flac".to_owned()),
            Some(10),
        );
        transfers.update_status(alice.id, "in_progress", None, None);
    }
    assert_eq!(
        super::file_transfer_runtime::inbound_upload_policy(
            &state,
            "alice",
            "Music/second.flac",
            10
        )
        .await
        .unwrap(),
        "friends"
    );

    {
        let mut transfers = state.transfers.write().await;
        let stranger = transfers.create(
            1,
            Some("stranger".to_owned()),
            "Music/first.flac".to_owned(),
            Some("/tmp/stranger.flac".to_owned()),
            Some(10),
        );
        transfers.update_status(stranger.id, "in_progress", None, None);
    }
    assert_eq!(
        super::file_transfer_runtime::inbound_upload_policy(
            &state,
            "alice",
            "Music/third.flac",
            10
        )
        .await
        .unwrap_err(),
        "Queued"
    );
    assert_eq!(
        super::file_transfer_runtime::inbound_upload_policy(
            &state,
            "stranger",
            "Music/second.flac",
            10
        )
        .await
        .unwrap_err(),
        "Too many files"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_group_runtime_uses_frozen_limit_rejection_reasons_and_priority_resolution() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with(
                "SLSKR_FROZEN_TRANSFER_UPLOAD_JSON",
                r#"{"slots":10,"limits":{"daily":{"failures":1},"weekly":{"files":1,"megabytes":1}}}"#,
            )
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"user_defined":{"lower":{"upload":{"priority":20},"members":["alice"]},"higher":{"upload":{"priority":5},"members":["alice"]}}}"#,
            ),
    );
    assert_eq!(
        super::file_transfer_runtime::effective_transfer_group(&state, "alice").await,
        "higher"
    );

    {
        let mut transfers = state.transfers.write().await;
        let completed = transfers.create(
            1,
            Some("alice".to_owned()),
            "one.flac".to_owned(),
            Some("/tmp/one.flac".to_owned()),
            Some(10),
        );
        transfers.update_status(completed.id, "in_progress", None, None);
        transfers.update_status(completed.id, "completed", Some(10), None);
    }
    assert_eq!(
        super::file_transfer_runtime::inbound_upload_policy(&state, "alice", "two.flac", 10)
            .await
            .unwrap_err(),
        "Too many files this week"
    );

    {
        let mut transfers = state.transfers.write().await;
        let failed = transfers.create(
            1,
            Some("bob".to_owned()),
            "failed.flac".to_owned(),
            Some("/tmp/failed.flac".to_owned()),
            Some(10),
        );
        transfers.update_status(failed.id, "in_progress", None, None);
        transfers.update_status(failed.id, "failed", None, Some("failure".to_owned()));
    }
    assert_eq!(
        super::file_transfer_runtime::inbound_upload_policy(&state, "bob", "next.flac", 10)
            .await
            .unwrap_err(),
        "Too many failed transfers today"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_group_queue_schedules_priority_slots_and_reports_frozen_positions() {
    let (state, mut receiver) = test_state_with_env(
        MapEnv::default()
            .with(
                "SLSKR_FROZEN_TRANSFER_UPLOAD_JSON",
                r#"{"slots":2}"#,
            )
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"default":{"upload":{"priority":20,"strategy":"firstinfirstout","slots":2}},"user_defined":{"friends":{"upload":{"priority":5,"strategy":"firstinfirstout","slots":1},"members":["alice","ally"]}}}"#,
            ),
    );
    let (bob, alice, ally) = {
        let mut transfers = state.transfers.write().await;
        let mut queue = |username: &str, filename: &str| {
            let entry = transfers.create(
                1,
                Some(username.to_owned()),
                filename.to_owned(),
                Some(format!("/tmp/{filename}")),
                Some(10),
            );
            transfers
                .update_status(entry.id, "queued", None, Some("Queued".to_owned()))
                .unwrap()
        };
        (
            queue("bob", "bob.flac"),
            queue("alice", "alice.flac"),
            queue("ally", "ally.flac"),
        )
    };
    {
        let transfers = state.transfers.read().await;
        let upload = state.transfer_upload_settings.read().await;
        let groups = state.transfer_groups_settings.read().await;
        let users = state.users.read().await;
        assert_eq!(
            super::next_queued_upload_id(&transfers, &upload, &groups, &users),
            Some(alice.id)
        );
    }
    super::schedule_queued_uploads(&state).await;
    assert_eq!(
        receiver.try_recv().unwrap(),
        super::SessionCommand::RequestPeerEndpoint("alice".to_owned())
    );
    assert_eq!(
        receiver.try_recv().unwrap(),
        super::SessionCommand::RequestPeerEndpoint("bob".to_owned())
    );
    assert!(receiver.try_recv().is_err());
    let transfers = state.transfers.read().await;
    assert_eq!(
        transfers
            .entries
            .iter()
            .find(|entry| entry.id == alice.id)
            .unwrap()
            .status,
        "peer_lookup"
    );
    assert_eq!(
        transfers
            .entries
            .iter()
            .find(|entry| entry.id == bob.id)
            .unwrap()
            .status,
        "peer_lookup"
    );
    assert_eq!(
        transfers
            .entries
            .iter()
            .find(|entry| entry.id == ally.id)
            .unwrap()
            .status,
        "queued"
    );
    drop(transfers);

    let (round_robin, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_FROZEN_TRANSFER_UPLOAD_JSON", r#"{"slots":1}"#)
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"default":{"upload":{"strategy":"roundrobin","slots":1}}}"#,
            ),
    );
    {
        let mut transfers = round_robin.transfers.write().await;
        for (username, filename) in [
            ("alice", "one.flac"),
            ("alice", "two.flac"),
            ("bob", "bob.flac"),
        ] {
            let entry = transfers.create(
                1,
                Some(username.to_owned()),
                filename.to_owned(),
                Some(format!("/tmp/{filename}")),
                Some(10),
            );
            transfers.update_status(entry.id, "queued", None, Some("Queued".to_owned()));
        }
    }
    assert_eq!(
        super::upload_queue_position(&round_robin, "alice", "two.flac").await,
        Some(2)
    );
    {
        let mut transfers = round_robin.transfers.write().await;
        let first = transfers.entries[0].id;
        transfers.update_status(first, "in_progress", None, None);
    }
    assert_eq!(
        super::upload_queue_forecast(&round_robin, "charlie").await,
        (1, 3)
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_group_mutations_require_exact_paths_and_decode_members() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Trusted","description":"original"}"#,
        &state,
    )
    .await
    .unwrap();
    let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    super::route_http_request(
        "POST",
        &format!("/api/sharegroups/{group_id}/members"),
        None,
        r#"{"username":"Peer One"}"#,
        &state,
    )
    .await
    .unwrap();

    super::route_http_request(
        "PUT",
        &format!("/api/sharegroups/{group_id}/extra"),
        None,
        r#"{"name":"Corrupted","description":"wrong"}"#,
        &state,
    )
    .await
    .unwrap();
    super::route_http_request(
        "POST",
        &format!("/api/sharegroups/{group_id}/extra/members"),
        None,
        r#"{"username":"intruder"}"#,
        &state,
    )
    .await
    .unwrap();
    super::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/members/Peer%20One/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    super::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();

    let group = state.sharegroups.read().await.get(&group_id).unwrap();
    assert_eq!(group.name, "Trusted");
    assert_eq!(group.description, "original");
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].username, "Peer One");

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/members/Peer%20One"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state
        .sharegroups
        .read()
        .await
        .get(&group_id)
        .unwrap()
        .members
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_group_member_revocation_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let group_id = state
        .sharegroups
        .write()
        .await
        .create("Trusted".to_owned(), String::new())
        .expect("share group")
        .id;
    state
        .sharegroups
        .write()
        .await
        .add_member(&group_id, "friend".to_owned())
        .expect("member capacity")
        .expect("share group");
    db.close_for_test().await;

    let response = super::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/members/friend"),
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("share group persistence failed"));
    let group = state
        .sharegroups
        .read()
        .await
        .get(&group_id)
        .expect("rolled-back group");
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].username, "friend");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_group_revocation_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let group_id = state
        .sharegroups
        .write()
        .await
        .create("Trusted".to_owned(), String::new())
        .expect("share group")
        .id;
    state
        .sharegroups
        .write()
        .await
        .add_member(&group_id, "friend".to_owned())
        .expect("member capacity")
        .expect("share group");
    db.close_for_test().await;

    let response = super::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("share group revocation persistence failed"));
    let group = state
        .sharegroups
        .read()
        .await
        .get(&group_id)
        .expect("rolled-back group");
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].username, "friend");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_group_snapshot_database_write_is_atomic() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let original = super::persistence::ShareGroupRecord {
        id: "group-1".to_owned(),
        name: "Original".to_owned(),
        description: String::new(),
        created_at: 1,
        updated_at: 1,
    };
    let member = super::persistence::ShareGroupMemberRecord {
        group_id: original.id.clone(),
        username: "friend".to_owned(),
        added_at: 1,
    };
    db.upsert_share_group(&original).await.unwrap();
    db.upsert_share_group_member(&member).await.unwrap();

    let changed = super::persistence::ShareGroupRecord {
        name: "Changed".to_owned(),
        updated_at: 2,
        ..original.clone()
    };
    assert!(db
        .replace_share_group(&changed, &[member.clone(), member.clone()])
        .await
        .is_err());

    let groups = db.list_share_groups(10, 0).await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "Original");
    let members = db.list_share_group_members(10, 0).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].username, "friend");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_group_writes_roll_back_when_persistence_fails() {
    let create_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (create_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(create_db.clone()),
    );
    create_db.close_for_test().await;
    let create = super::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Transient"}"#,
        &create_state,
    )
    .await
    .expect("failed create response");
    assert_eq!(create.status, "503 Service Unavailable");
    assert!(create.body.contains("share group persistence failed"));
    assert!(create_state.sharegroups.read().await.records.is_empty());

    let update_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (update_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(update_db.clone()),
    );
    let update_id = update_state
        .sharegroups
        .write()
        .await
        .create("Original".to_owned(), String::new())
        .expect("share group")
        .id;
    update_db.close_for_test().await;
    let update = super::route_http_request(
        "PUT",
        &format!("/api/sharegroups/{update_id}"),
        None,
        r#"{"name":"Changed"}"#,
        &update_state,
    )
    .await
    .expect("failed update response");
    assert_eq!(update.status, "503 Service Unavailable");
    assert_eq!(
        update_state
            .sharegroups
            .read()
            .await
            .get(&update_id)
            .expect("rolled-back group")
            .name,
        "Original"
    );

    let member_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (member_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(member_db.clone()),
    );
    let member_group_id = member_state
        .sharegroups
        .write()
        .await
        .create("Trusted".to_owned(), String::new())
        .expect("share group")
        .id;
    member_db.close_for_test().await;
    let member = super::route_http_request(
        "POST",
        &format!("/api/sharegroups/{member_group_id}/members"),
        None,
        r#"{"username":"friend"}"#,
        &member_state,
    )
    .await
    .expect("failed member response");
    assert_eq!(member.status, "503 Service Unavailable");
    assert!(member.body.contains("share group persistence failed"));
    assert!(member_state
        .sharegroups
        .read()
        .await
        .get(&member_group_id)
        .expect("rolled-back group")
        .members
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn compatibility_projections_use_local_state_for_core_stores() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));

    {
        let mut shares = state.shares.write().await;
        shares.roots.push(super::ShareRoot {
            label: "Virtual".to_owned(),
            local_path: PathBuf::from("Virtual"),
            raw: "Virtual".to_owned(),
            directories: 0,
            files: 2,
            bytes: 12,
            extensions: Vec::new(),
            statistics_ready: true,
        });
    }
    let shared = super::route_http_request("GET", "/api/shared", None, "", &state)
        .await
        .expect("shared projection");
    let shared_json = serde_json::from_str::<serde_json::Value>(&shared.body).unwrap();
    assert_eq!(shared_json[0]["id"], super::share_root_id("Virtual"));
    assert_eq!(shared_json[0]["files"], 2);
    let exact_share = super::route_http_request(
        "GET",
        &format!("/api/shares/{}", super::share_root_id("Virtual")),
        None,
        "",
        &state,
    )
    .await
    .expect("exact share resource");
    assert_eq!(exact_share.status, "200 OK");
    let aliased_contents = super::route_http_request(
        "GET",
        "/api/shares/Virtual/extra/contents",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased share contents");
    assert_eq!(aliased_contents.status, "404 Not Found");

    let contact = super::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"nearby"}"#,
        &state,
    )
    .await
    .expect("create contact");
    let contact_json = serde_json::from_str::<serde_json::Value>(&contact.body).unwrap();
    let contact_id = contact_json["id"].as_str().unwrap();
    super::route_http_request(
        "PUT",
        &format!("/api/contacts/{contact_id}"),
        None,
        r#"{"online":true}"#,
        &state,
    )
    .await
    .expect("update contact");
    let nearby = super::route_http_request("GET", "/api/contacts/nearby", None, "", &state)
        .await
        .expect("nearby contacts");
    let nearby_json = serde_json::from_str::<serde_json::Value>(&nearby.body).unwrap();
    assert_eq!(nearby_json[0]["username"], "nearby");
    assert_eq!(nearby_json[0]["online"], true);

    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Favorites"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_json = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap();
    let collection_id = collection_json["id"].as_str().unwrap();
    let first = super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"1","artist":"A","title":"First"}"#,
        &state,
    )
    .await
    .expect("first item");
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    let first_id = first_json["id"].as_str().unwrap();
    let second = super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"2","artist":"B","title":"Second"}"#,
        &state,
    )
    .await
    .expect("second item");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap();
    let second_id = second_json["id"].as_str().unwrap();

    let updated_item = super::route_http_request(
        "PUT",
        &format!("/api/collections/items/{first_id}"),
        None,
        r#"{"artist":"Updated","title":"Renamed","kind":"Video"}"#,
        &state,
    )
    .await
    .expect("update item");
    let updated_item_json = serde_json::from_str::<serde_json::Value>(&updated_item.body).unwrap();
    assert_eq!(updated_item_json["artist"], "Updated");
    assert_eq!(updated_item_json["kind"], "Video");

    let reordered = super::route_http_request(
        "PUT",
        &format!("/api/collections/{collection_id}/items/reorder"),
        None,
        &format!(r#"{{"itemIds":["{second_id}","{first_id}"]}}"#),
        &state,
    )
    .await
    .expect("reorder items");
    let reordered_json = serde_json::from_str::<serde_json::Value>(&reordered.body).unwrap();
    assert_eq!(reordered_json["reordered"], true);
    assert_eq!(reordered_json["items"][0]["id"], second_id);

    let deleted_item = super::route_http_request(
        "DELETE",
        &format!("/api/collections/items/{first_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete item");
    let deleted_item_json = serde_json::from_str::<serde_json::Value>(&deleted_item.body).unwrap();
    assert_eq!(deleted_item_json["deleted"], true);
    assert_eq!(deleted_item_json["item"]["id"], first_id);

    let wish = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Old","title":"Needle"}"#,
        &state,
    )
    .await
    .expect("create wishlist");
    let wish_json = serde_json::from_str::<serde_json::Value>(&wish.body).unwrap();
    let wish_id = wish_json["id"].as_str().unwrap();
    let wish_update = super::route_http_request(
        "PUT",
        &format!("/api/wishlist/{wish_id}"),
        None,
        r#"{"artist":"New","title":"Needle"}"#,
        &state,
    )
    .await
    .expect("update wishlist");
    let wish_update_json = serde_json::from_str::<serde_json::Value>(&wish_update.body).unwrap();
    assert_eq!(wish_update_json["artist"], "New");
    let wish_delete = super::route_http_request(
        "DELETE",
        &format!("/api/wishlist/{wish_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete wishlist");
    let wish_delete_json = serde_json::from_str::<serde_json::Value>(&wish_delete.body).unwrap();
    assert_eq!(wish_delete_json["deleted"], true);
    assert_eq!(wish_delete_json["item_id"], wish_id);

    super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Song.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create transfer");
    super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":40}"#,
        &state,
    )
    .await
    .expect("transfer progress");
    let bridge =
        super::route_http_request("GET", "/api/bridge/transfer/1/progress", None, "", &state)
            .await
            .expect("bridge progress");
    let bridge_json = serde_json::from_str::<serde_json::Value>(&bridge.body).unwrap();
    assert_eq!(bridge_json["status"], "in_progress");
    assert_eq!(bridge_json["bytesTransferred"], 40);
    assert_eq!(bridge_json["progress"], 40.0);
    let aliased_bridge = super::route_http_request(
        "GET",
        "/api/bridge/transfer/1/extra/progress",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased bridge progress");
    assert_eq!(aliased_bridge.status, "404 Not Found");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn collection_item_collection_routes_require_exact_paths() {
    let (state, _receiver) = test_state();
    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Exact"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/extra/items"),
        None,
        r#"{"content_id":"wrong","title":"Wrong"}"#,
        &state,
    )
    .await
    .unwrap();
    assert!(state
        .collections
        .read()
        .await
        .get(&collection_id)
        .unwrap()
        .items
        .is_empty());
    assert_eq!(
        super::collection_items_id(&format!("/api/collections/{collection_id}/extra/items")),
        None
    );

    let exact = super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"right","title":"Right"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(exact.status, "201 Created");
    assert_eq!(
        super::collection_items_id(&format!("/api/collections/{collection_id}/items")),
        Some(collection_id.as_str())
    );
    assert_eq!(
        state
            .collections
            .read()
            .await
            .get(&collection_id)
            .unwrap()
            .items
            .len(),
        1
    );
    let item_id = serde_json::from_str::<serde_json::Value>(&exact.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        super::collection_item_action_ids(&format!(
            "/api/collections/{collection_id}/items/{item_id}"
        )),
        Some((item_id.as_str(), Some(collection_id.as_str())))
    );
    assert_eq!(
        super::collection_item_action_ids(&format!("/api/collections/items/{item_id}")),
        Some((item_id.as_str(), None))
    );
    assert_eq!(
        super::collection_item_action_ids(&format!(
            "/api/collections/{collection_id}/extra/items/{item_id}"
        )),
        None
    );

    let nested_update = super::route_http_request(
        "PUT",
        &format!("/api/collections/{collection_id}/items/{item_id}"),
        None,
        r#"{"title":"Nested update"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(nested_update.status, "200 OK");
    let nested_delete = super::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(nested_delete.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn wishlist_item_search_requires_exact_action_path() {
    let (state, _receiver) = test_state();
    let wish = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Track"}"#,
        &state,
    )
    .await
    .unwrap();
    let item_id = serde_json::from_str::<serde_json::Value>(&wish.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    super::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/extra/search"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert!(state.searches.read().await.records.is_empty());
    assert_eq!(
        super::wishlist_search_item_id(&format!("/api/wishlist/{item_id}/extra/search")),
        None
    );

    let exact = super::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(exact.status, "202 Accepted");
    assert_eq!(
        super::wishlist_search_item_id(&format!("/api/wishlist/{item_id}/search")),
        Some(item_id.as_str())
    );
    let searches = state.searches.read().await;
    assert_eq!(searches.records.len(), 1);
    assert_eq!(searches.records[0].query, "Artist Track");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn user_status_routes_require_exact_user_segments() {
    let (state, _receiver) = test_state();
    {
        let mut users = state.users.write().await;
        let record = users.watch("friend".to_owned()).unwrap();
        users
            .records
            .iter_mut()
            .find(|candidate| candidate.username == record.username)
            .unwrap()
            .status = Some("Online".to_owned());
    }
    state
        .browse
        .write()
        .await
        .request("friend".to_owned())
        .unwrap();

    let exact_status =
        super::route_http_request("GET", "/api/users/friend/status", None, "", &state)
            .await
            .unwrap();
    let malformed_status =
        super::route_http_request("GET", "/api/users/friend/extra/status", None, "", &state)
            .await
            .unwrap();
    assert!(exact_status.body.contains("Online"));
    assert_ne!(malformed_status.body, exact_status.body);

    let exact_browse =
        super::route_http_request("GET", "/api/users/friend/browse/status", None, "", &state)
            .await
            .unwrap();
    let malformed_browse = super::route_http_request(
        "GET",
        "/api/users/friend/extra/browse/status",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert!(exact_browse.body.contains("\"username\":\"friend\""));
    assert_ne!(malformed_browse.body, exact_browse.body);
    assert_eq!(
        super::user_route_username("/api/users/friend/status", "/status"),
        Some("friend".to_owned())
    );
    assert_eq!(
        super::user_route_username("/api/users/friend/extra/status", "/status"),
        None
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn contacts_are_bounded_deduplicated_and_report_discovery_truthfully() {
    let mut store = super::ContactStore::with_max_records(1);
    let (_, created) = store.create("Alice".to_owned()).unwrap();
    assert!(created);
    let (existing, created) = store.create("alice".to_owned()).unwrap();
    assert!(!created);
    assert_eq!(existing.username, "Alice");
    assert!(store.create("Bob".to_owned()).is_err());
    assert_eq!(store.records.len(), 1);

    let (state, _receiver) = test_state();
    let empty =
        super::route_http_request("POST", "/api/contacts/from-discovery", None, "{}", &state)
            .await
            .expect("empty discovery response");
    assert_eq!(empty.status, "400 Bad Request");

    let first = super::route_http_request(
        "POST",
        "/api/contacts/from-discovery",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("first discovery response");
    assert_eq!(first.status, "201 Created");
    assert!(first.body.contains("\"added\":true"));

    let duplicate = super::route_http_request(
        "POST",
        "/api/contacts/from-discovery",
        None,
        r#"{"username":"FRIEND"}"#,
        &state,
    )
    .await
    .expect("duplicate discovery response");
    assert_eq!(duplicate.status, "200 OK");
    assert!(duplicate.body.contains("\"added\":false"));
    assert_eq!(state.contacts.read().await.records.len(), 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_groups_bound_groups_and_case_insensitive_members() {
    let mut groups = super::ShareGroupStore::with_limits(1, 1);
    let group = groups.create("Trusted".to_owned(), String::new()).unwrap();
    assert!(groups
        .create("Overflow".to_owned(), String::new())
        .is_none());

    let (_, added) = groups
        .add_member(&group.id, "Alice".to_owned())
        .unwrap()
        .unwrap();
    assert!(added);
    let (_, added) = groups
        .add_member(&group.id, "alice".to_owned())
        .unwrap()
        .unwrap();
    assert!(!added);
    assert!(groups.add_member(&group.id, "Bob".to_owned()).is_err());
    assert!(groups
        .add_member("missing", "Bob".to_owned())
        .unwrap()
        .is_none());
    assert!(groups.remove_member(&group.id, "ALICE").is_some());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn contacts_and_share_groups_bound_text_and_aggregate_members() {
    let oversized_username = "é".repeat(super::MAX_USER_USERNAME_BYTES);
    let mut contacts = super::ContactStore::with_max_records(2);
    let (contact, created) = contacts.create(oversized_username.clone()).unwrap();
    assert!(created);
    assert!(contact.username.len() <= super::MAX_USER_USERNAME_BYTES);
    let (_, duplicate) = contacts.create(oversized_username.clone()).unwrap();
    assert!(!duplicate);

    let mut groups = super::ShareGroupStore::with_limits(14, super::MAX_SHARE_GROUP_MEMBERS + 1);
    let first = groups
        .create(
            "n".repeat(super::MAX_LIST_NAME_BYTES + 1),
            "d".repeat(super::MAX_LIST_DESCRIPTION_BYTES + 1),
        )
        .unwrap();
    assert_eq!(first.name.len(), super::MAX_LIST_NAME_BYTES);
    assert_eq!(first.description.len(), super::MAX_LIST_DESCRIPTION_BYTES);
    let (_, added) = groups
        .add_member(&first.id, oversized_username.clone())
        .unwrap()
        .unwrap();
    assert!(added);
    assert!(groups.records[0].members[0].username.len() <= super::MAX_USER_USERNAME_BYTES);

    groups.records[0].members.clear();
    while groups.records.len() < 13 {
        groups.create("group".to_owned(), String::new()).unwrap();
    }
    let template = super::ShareGroupMember {
        username: "peer".to_owned(),
        added_at: 0,
    };
    for record in groups.records.iter_mut().take(12) {
        record.members = vec![template.clone(); super::MAX_SHARE_GROUP_MEMBERS];
    }
    groups.records[12].members = vec![
        template;
        super::MAX_TOTAL_SHARE_GROUP_MEMBERS
            - (12 * super::MAX_SHARE_GROUP_MEMBERS)
    ];
    let last = groups.create("last".to_owned(), String::new()).unwrap();
    assert!(groups.add_member(&last.id, "overflow".to_owned()).is_err());
    assert_eq!(groups.total_members(), super::MAX_TOTAL_SHARE_GROUP_MEMBERS);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn contact_update_rejects_duplicate_username() {
    let (state, _receiver) = test_state();
    let first = super::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"Alice"}"#,
        &state,
    )
    .await
    .unwrap();
    let first_id = serde_json::from_str::<serde_json::Value>(&first.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = super::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"Bob"}"#,
        &state,
    )
    .await
    .unwrap();
    let second_id = serde_json::from_str::<serde_json::Value>(&second.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let duplicate = super::route_http_request(
        "PUT",
        &format!("/api/contacts/{first_id}"),
        None,
        r#"{"username":"bOB"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(duplicate.status, "409 Conflict");
    assert_eq!(
        duplicate.body,
        "{\"error\":\"contact username already exists\"}"
    );
    let contacts = state.contacts.read().await;
    assert_eq!(contacts.get(&first_id).unwrap().username, "Alice");
    assert_eq!(contacts.get(&second_id).unwrap().username, "Bob");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn collections_bound_nested_state_and_allocate_unique_item_ids() {
    let mut collections = super::CollectionStore::with_limits(1, 2);
    let collection = collections
        .create(String::new(), "Road Trip".to_owned(), String::new())
        .unwrap();
    assert!(collections
        .create(String::new(), "Overflow".to_owned(), String::new())
        .is_none());

    let first = collections
        .add_item(
            &collection.id,
            "content-1".to_owned(),
            "Artist".to_owned(),
            "First".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    let second = collections
        .add_item(
            &collection.id,
            "content-2".to_owned(),
            "Artist".to_owned(),
            "Second".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    assert_ne!(first.id, second.id);
    assert!(collections
        .add_item(
            &collection.id,
            "content-3".to_owned(),
            String::new(),
            "Third".to_owned(),
            "Audio".to_owned(),
        )
        .is_err());
    assert!(collections
        .add_item(
            "missing",
            String::new(),
            String::new(),
            String::new(),
            "Audio".to_owned(),
        )
        .unwrap()
        .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn collections_and_wishlist_bound_text_and_aggregate_items() {
    let mut collections = super::CollectionStore::with_limits(6, super::MAX_COLLECTION_ITEMS + 1);
    let first = collections
        .create(
            String::new(),
            "n".repeat(super::MAX_LIST_NAME_BYTES + 1),
            "d".repeat(super::MAX_LIST_DESCRIPTION_BYTES + 1),
        )
        .unwrap();
    assert_eq!(first.name.len(), super::MAX_LIST_NAME_BYTES);
    assert_eq!(first.description.len(), super::MAX_LIST_DESCRIPTION_BYTES);
    let item = collections
        .add_item(
            &first.id,
            "c".repeat(super::MAX_LIST_CONTENT_ID_BYTES + 1),
            "a".repeat(super::MAX_LIST_ARTIST_BYTES + 1),
            "t".repeat(super::MAX_LIST_TITLE_BYTES + 1),
            "k".repeat(super::MAX_LIST_KIND_BYTES + 1),
        )
        .unwrap()
        .unwrap();
    assert_eq!(item.content_id.len(), super::MAX_LIST_CONTENT_ID_BYTES);
    assert_eq!(item.artist.len(), super::MAX_LIST_ARTIST_BYTES);
    assert_eq!(item.title.len(), super::MAX_LIST_TITLE_BYTES);
    assert_eq!(item.kind.len(), super::MAX_LIST_KIND_BYTES);

    let template = super::CollectionItem {
        id: "item".to_owned(),
        content_id: "content".to_owned(),
        artist: String::new(),
        title: String::new(),
        kind: String::new(),
        file_name: String::new(),
        album: String::new(),
        content_hash: String::new(),
        added_at: 0,
    };
    collections.records[0].items.clear();
    while collections.records.len() < 5 {
        collections
            .create(String::new(), "collection".to_owned(), String::new())
            .unwrap();
    }
    for record in &mut collections.records {
        record.items = vec![template.clone(); super::MAX_COLLECTION_ITEMS];
    }
    let last = collections
        .create(String::new(), "last".to_owned(), String::new())
        .unwrap();
    assert!(collections
        .add_item(
            &last.id,
            "overflow".to_owned(),
            String::new(),
            String::new(),
            String::new(),
        )
        .is_err());
    assert_eq!(collections.total_items(), super::MAX_TOTAL_COLLECTION_ITEMS);

    let mut wishlist = super::WishlistStore::with_max_items(1);
    let wish = wishlist
        .add_item(
            "a".repeat(super::MAX_LIST_ARTIST_BYTES + 1),
            "t".repeat(super::MAX_LIST_TITLE_BYTES + 1),
            "k".repeat(super::MAX_LIST_KIND_BYTES + 1),
        )
        .unwrap();
    assert_eq!(wish.artist.len(), super::MAX_LIST_ARTIST_BYTES);
    assert_eq!(wish.title.len(), super::MAX_LIST_TITLE_BYTES);
    assert_eq!(wish.kind.len(), super::MAX_LIST_KIND_BYTES);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn wishlist_bounds_items_and_allocates_unique_ids() {
    let mut wishlist = super::WishlistStore::with_max_items(2);
    let first = wishlist
        .add_item("Artist".to_owned(), "First".to_owned(), "Audio".to_owned())
        .unwrap();
    let second = wishlist
        .add_item("Artist".to_owned(), "Second".to_owned(), "Audio".to_owned())
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(wishlist.remaining_capacity(), 0);
    assert!(wishlist
        .add_item(String::new(), "Overflow".to_owned(), "Audio".to_owned())
        .is_err());
    wishlist.next_item_id = u64::MAX;
    assert!(!wishlist.can_add_items(1));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn collection_and_wishlist_ids_wrap_without_collisions() {
    let mut collections = super::CollectionStore::with_limits(3, 3);
    collections.next_id = u64::MAX;
    let max_collection = collections
        .create(String::new(), "Max".to_owned(), String::new())
        .unwrap();
    let wrapped_collection = collections
        .create(String::new(), "Wrapped".to_owned(), String::new())
        .unwrap();
    assert_eq!(max_collection.id, format!("col-{}", u64::MAX));
    assert_eq!(wrapped_collection.id, "col-1");

    collections.next_item_id = u64::MAX;
    let max_item = collections
        .add_item(
            &max_collection.id,
            "max".to_owned(),
            String::new(),
            "Max".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    let wrapped_item = collections
        .add_item(
            &wrapped_collection.id,
            "wrapped".to_owned(),
            String::new(),
            "Wrapped".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(max_item.id, format!("item-{}", u64::MAX));
    assert_eq!(wrapped_item.id, "item-1");

    let mut wishlist = super::WishlistStore::with_max_items(3);
    wishlist.next_item_id = u64::MAX;
    assert!(wishlist.can_add_items(2));
    let max_wish = wishlist
        .add_item(String::new(), "Max".to_owned(), "Audio".to_owned())
        .unwrap();
    let wrapped_wish = wishlist
        .add_item(String::new(), "Wrapped".to_owned(), "Audio".to_owned())
        .unwrap();
    assert_eq!(max_wish.id, format!("wish-{}", u64::MAX));
    assert_eq!(wrapped_wish.id, "wish-1");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn library_health_scans_are_bounded_snapshots_with_unique_ids() {
    let mut library = super::LibraryStore::new();
    library
        .create("Artist".to_owned(), "Title".to_owned(), String::new())
        .unwrap();
    let first = library.create_health_scan("/music".to_owned()).unwrap();
    let second = library.create_health_scan("/music".to_owned()).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.issues.len(), 1);

    library.fix_health_issues();
    assert_eq!(library.health_scan(&first.id).unwrap().issues.len(), 1);
    for _ in 2..super::MAX_LIBRARY_HEALTH_SCANS {
        library.create_health_scan("/music".to_owned()).unwrap();
    }
    assert_eq!(library.health_scans.len(), super::MAX_LIBRARY_HEALTH_SCANS);
    library.create_health_scan("/music".to_owned()).unwrap();
    assert_eq!(library.health_scans.len(), super::MAX_LIBRARY_HEALTH_SCANS);
    assert!(library.health_scan(&first.id).is_none());
    assert!(library.health_scan("scan-does-not-exist").is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn library_health_group_totals_match_the_bounded_groups() {
    let mut library = super::LibraryStore::new();
    library
        .create("Artist A".to_owned(), "Release A".to_owned(), String::new())
        .unwrap();
    library
        .create("Artist B".to_owned(), "Release B".to_owned(), String::new())
        .unwrap();
    library
        .create("Artist C".to_owned(), String::new(), "Audio".to_owned())
        .unwrap();
    library
        .create("Artist B".to_owned(), "Release B".to_owned(), String::new())
        .unwrap();

    let artists =
        serde_json::from_str::<serde_json::Value>(&library.health_issues_by_artist_json(1))
            .unwrap();
    assert_eq!(artists["groups"].as_array().unwrap().len(), 1);
    assert_eq!(artists["totalArtists"], 1);

    let releases =
        serde_json::from_str::<serde_json::Value>(&library.health_issues_by_release_json(1))
            .unwrap();
    assert_eq!(releases["groups"].as_array().unwrap().len(), 1);
    assert_eq!(releases["totalReleases"], 1);
    assert_eq!(releases["groups"][0]["count"], 2);
    assert!(releases["groups"]
        .as_array()
        .unwrap()
        .iter()
        .all(|group| !group["album"].as_str().unwrap().is_empty()));
    let all_releases =
        serde_json::from_str::<serde_json::Value>(&library.health_issues_by_release_json(100))
            .unwrap();
    assert_eq!(all_releases["groups"].as_array().unwrap().len(), 2);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn songid_runs_are_bounded_snapshots_with_real_lookup() {
    let mut runtime = super::RuntimeCompatState::new();
    let first = runtime
        .record_songid_run(vec![serde_json::json!({ "score": 1.0 })], 1, 1)
        .unwrap();
    let first_id = first["id"].as_str().unwrap().to_owned();
    for _ in 1..super::MAX_SONGID_RUNS {
        runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    }
    assert_eq!(runtime.songid_run_records.len(), super::MAX_SONGID_RUNS);
    assert!(runtime.songid_run(&first_id).is_some());
    runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    assert_eq!(runtime.songid_run_records.len(), super::MAX_SONGID_RUNS);
    assert!(runtime.songid_run(&first_id).is_none());
    assert!(runtime.songid_run("songid-does-not-exist").is_none());

    runtime.songid_runs = u64::MAX;
    let wrapped = runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    assert_eq!(wrapped["id"], "songid-1");
    runtime.songid_runs = u64::MAX;
    let collision_avoiding = runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    assert_eq!(collision_avoiding["id"], "songid-2");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn notes_and_interests_bound_growth_and_ids() {
    let mut notes = super::UserNoteStore::new();
    let bounded_note = notes
        .create(
            "é".repeat(super::MAX_USER_USERNAME_BYTES),
            "n".repeat(super::MAX_USER_NOTE_BYTES + 1),
        )
        .unwrap();
    assert!(bounded_note.username.len() <= super::MAX_USER_USERNAME_BYTES);
    assert_eq!(bounded_note.note.len(), super::MAX_USER_NOTE_BYTES);
    notes.records.clear();
    for index in 0..super::MAX_USER_NOTES {
        notes
            .create(format!("user-{index}"), "note".to_owned())
            .unwrap();
    }
    assert!(notes.create("overflow".to_owned(), String::new()).is_none());
    let mut exhausted_notes = super::UserNoteStore::new();
    exhausted_notes.next_id = u64::MAX;
    assert_eq!(
        exhausted_notes
            .create("user".to_owned(), String::new())
            .unwrap()
            .id,
        format!("note-{}", u64::MAX)
    );

    let mut interests = super::InterestStore::new();
    let (bounded_interest, created) = interests
        .add_liked("i".repeat(super::MAX_INTEREST_NAME_BYTES + 1))
        .unwrap();
    assert!(created);
    assert_eq!(bounded_interest.name.len(), super::MAX_INTEREST_NAME_BYTES);
    interests.liked.clear();
    interests.next_id = 1;
    let (liked, created) = interests.add_liked("Ambient".to_owned()).unwrap();
    assert!(created);
    let (duplicate_liked, created) = interests.add_liked("ambient".to_owned()).unwrap();
    assert!(!created);
    assert_eq!(duplicate_liked.id, liked.id);
    let (hated, created) = interests.add_hated("Noise".to_owned()).unwrap();
    assert!(created);
    let (duplicate_hated, created) = interests.add_hated("NOISE".to_owned()).unwrap();
    assert!(!created);
    assert_eq!(duplicate_hated.id, hated.id);
    assert_eq!(interests.liked.len(), 1);
    assert_eq!(interests.hated.len(), 1);
    interests.next_id = u64::MAX;
    assert_eq!(
        interests.add_liked("Jazz".to_owned()).unwrap().0.id,
        format!("liked-{}", u64::MAX)
    );
    assert_eq!(
        interests.add_hated("Pop".to_owned()).unwrap().0.id,
        "hated-3"
    );
}
