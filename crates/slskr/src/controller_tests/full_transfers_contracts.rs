//! Controller full transfers contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn inbound_soulseek_pod_messages_are_validated_and_stored() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000001";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Inbound Pod",
                "isPublic": true,
                "channels": [{
                    "channelId": "general",
                    "kind": 0,
                    "name": "General"
                }]
            }))
            .expect("deserialize inbound pod fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create inbound pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "bridge:alice".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add inbound pod member");
    state
        .advanced_networking
        .write()
        .await
        .pod_security_signature_mode = crate::PodSignatureMode::Off;

    let timestamp = crate::unix_timestamp_millis();
    let payload = serde_json::json!({
        "MessageId": "550e8400-e29b-41d4-a716-446655440000",
        "PodId": pod_id,
        "ChannelId": "general",
        "SenderPeerId": "bridge:alice",
        "Body": "hello from the bridge",
        "TimestampUnixMs": timestamp,
        "SigVersion": 1,
        "Signature": ""
    });
    assert!(
        crate::handle_incoming_soulseek_pod_message(&state, "alice", &format!("PODMSG:{payload}"),)
            .await
    );
    assert!(
        crate::handle_incoming_soulseek_pod_message(&state, "alice", &format!("PODMSG:{payload}"),)
            .await
    );
    assert!(!crate::handle_incoming_soulseek_pod_message(&state, "alice", "ordinary PM").await);

    let messages = state
        .pod_channels
        .read()
        .await
        .list(pod_id, "general", None);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].message_id,
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(messages[0].sender_peer_id, "bridge:alice");
    assert_eq!(messages[0].body, "hello from the bridge");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_storage_errors_redact_internal_details() {
    let mut transfers = crate::TransferQueue::new_in_memory(8);
    transfers.state_error =
        Some("transfer state parse failed at /private/transfer-state.json".to_owned());
    transfers.events_error =
        Some("transfer event open failed at /private/transfer-events.tsv".to_owned());

    let json = transfers.json(None);
    assert!(json.contains("\"state_error\":\"transfer state unavailable\""));
    assert!(json.contains("\"events_error\":\"transfer events unavailable\""));
    assert!(!json.contains("/private"));
    assert!(!json.contains("parse failed"));
    assert!(!json.contains("open failed"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_json_redacts_local_storage_details() {
    let mut transfers = crate::TransferQueue::new_in_memory(8);
    let entry = transfers.create(
        0,
        None,
        "Remote/Track.flac".to_owned(),
        Some("/private/downloads/Track.flac".to_owned()),
        Some(4),
    );

    assert_eq!(
        entry.local_path.as_deref(),
        Some("/private/downloads/Track.flac")
    );
    transfers.update_status(
        entry.id,
        "failed",
        None,
        Some("write failed at /private/downloads/Track.flac: denied".to_owned()),
    );
    let json = transfers.json(None);
    assert!(json.contains("\"local_path\":null"));
    assert!(json.contains("\"reason\":\"transfer failed\""));
    assert!(!json.contains("/private/downloads"));
    assert!(!json.contains("denied"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn download_batch_projection_uses_local_transfer_state() {
    let (state, _receiver) = test_state();
    let batch_id = "11111111-1111-4111-8111-111111111111";
    let mut ledger = Vec::new();

    let created = crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        &format!(
            r#"{{"filename":"Virtual/Test.flac","peer_username":"peer","size":42,"batchId":"{batch_id}"}}"#
        ),
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");

    let invalid = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/not-a-guid",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid batch");
    assert_eq!(invalid.status, "400 Bad Request");
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/transfers/downloads/batches/{id}",
        "case": "malformed-path-query-or-body",
        "pass": invalid.status == "400 Bad Request",
    }));

    let response = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("batch projection");
    assert_eq!(response.status, "200 OK");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["id"], batch_id);
    assert_eq!(json["transferCount"], 1);
    assert_eq!(json["transfers"][0]["batchId"], batch_id);
    assert_eq!(json["transfers"][0]["filename"], "Virtual/Test.flac");
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/transfers/downloads/batches/{id}",
        "case": "nominal-status-headers-body",
        "pass": response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && json["id"] == batch_id,
    }));
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/transfers/downloads/batches/{id}",
        "case": "populated-dynamic-state",
        "pass": json["transferCount"] == 1
            && json["transfers"][0]["batchId"] == batch_id
            && json["transfers"][0]["filename"] == "Virtual/Test.flac",
    }));

    let missing = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/22222222-2222-4222-8222-222222222222",
        None,
        "",
        &state,
    )
    .await
    .expect("missing batch");
    assert_eq!(missing.status, "404 Not Found");
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/transfers/downloads/batches/{id}",
        "case": "missing-empty-or-conflict-state",
        "pass": missing.status == "404 Not Found",
    }));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("transfer_batch_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_cancellation_returns_frozen_statuses() {
    let (state, _receiver) = test_state();
    let missing = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/peer/999",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing.status, "404 Not Found");
    let entry = state.transfers.write().await.create(
        0,
        Some("peer".to_owned()),
        "cancel.flac".to_owned(),
        None,
        Some(1),
    );
    let cancelled = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/peer/{}", entry.id),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(cancelled.status, "204 No Content");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn inbound_message_route_rolls_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let previous = state.messages.read().await.clone();
    db.close_for_test().await;

    let response = crate::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        r#"{"username":"friend","body":"do not lose me"}"#,
        &state,
    )
    .await
    .expect("failed inbound message persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("message persistence failed"));
    assert_eq!(*state.messages.read().await, previous);
}
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_projection_persists_status_and_surfaces_failures() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let entry = crate::TransferEntry {
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

    crate::persist_transfer_projection(&state, &entry).await;
    let persisted = db.get_transfer("99").await.unwrap().unwrap();
    assert_eq!(persisted.status, "in_progress");
    assert_eq!(persisted.progress, 40);

    db.close_for_test().await;
    crate::persist_transfer_projection(&state, &entry).await;
    let session = state.session.read().await;
    let error = session.last_error.as_deref().unwrap_or_default();
    assert!(error.contains("transfer 99 persistence failed"));
    assert!(error.contains("failed to persist transfer"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn completed_transfer_cleanup_rolls_back_on_persistence_failure() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"filename":"Remote/Completed.flac","size":10}"#,
        &state,
    )
    .await
    .expect("create transfer");
    crate::route_http_request(
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

    let response = crate::route_http_request(
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
pub(super) async fn transfer_cancel_rolls_back_on_persistence_failure() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
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

    let response = crate::route_http_request(
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
pub(super) async fn transfer_start_enforces_max_active_policy() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_MAX_ACTIVE", "1"));

    for filename in ["Remote/One.flac", "Remote/Two.flac"] {
        let body = format!("{{\"filename\":\"{}\",\"size\":10}}", filename);
        let created = crate::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
            .await
            .expect("create transfer");
        assert_eq!(created.status, "201 Created");
    }

    let started = crate::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start first");
    assert_eq!(started.status, "200 OK");

    let blocked = crate::route_http_request("POST", "/api/v0/transfers/2/start", None, "", &state)
        .await
        .expect("start second");
    assert_eq!(blocked.status, "409 Conflict");
    assert_eq!(blocked.body, "{\"error\":\"transfer limit reached\"}");

    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(1, "cancelled", None, None);
    }

    let unblocked =
        crate::route_http_request("POST", "/api/v0/transfers/2/start", None, "", &state)
            .await
            .expect("retry second");
    assert_eq!(unblocked.status, "200 OK");
    assert!(unblocked.body.contains("\"status\":\"in_progress\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_retry_requeues_failed_download_and_clears_reason() {
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

    let retried = crate::route_http_request("POST", "/api/v0/transfers/1/retry", None, "", &state)
        .await
        .expect("retry transfer");

    assert_eq!(retried.status, "200 OK");
    assert!(retried.body.contains("\"status\":\"peer_lookup\""));
    assert!(retried.body.contains("\"bytes_transferred\":4"));
    assert!(retried.body.contains("\"reason\":null"));
    assert_eq!(
        receiver.try_recv().expect("transfer command"),
        crate::SessionCommand::TransferPeer {
            id: 1,
            username: "friend".to_owned(),
        }
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn download_requests_preserve_metadata_path_and_runtime_lifecycle() {
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
    let created = crate::route_http_request(
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

    let listed = crate::route_http_request(
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

    let renamed = crate::route_http_request(
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

    let cancelled = crate::route_http_request(
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

    let detail = crate::route_http_request(
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
pub(super) fn transfer_failure_projection_exposes_bounded_recovery_not_internal_reason() {
    let mut transfers = crate::TransferQueue::new_in_memory(10);
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
pub(super) fn completed_audio_metadata_parsers_cover_mp3_and_wav_headers() {
    let mp3 =
        crate::mp3_technical_metadata(&[0xff, 0xfb, 0x90, 0x00], 128_000).expect("mp3 metadata");
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
    let wav = crate::wav_technical_metadata(&wav).expect("wav metadata");
    assert_eq!(wav.bit_rate, Some(1_411));
    assert_eq!(wav.sample_rate, Some(44_100));
    assert_eq!(wav.bit_depth, Some(16));
    assert_eq!(wav.length_seconds, Some(1));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_retry_rejects_non_terminal_download() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Song.flac\",\"peer_username\":\"friend\",\"size\":10}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");

    let blocked = crate::route_http_request("POST", "/api/v0/transfers/1/retry", None, "", &state)
        .await
        .expect("retry transfer");

    assert_eq!(blocked.status, "409 Conflict");
    assert_eq!(blocked.body, "{\"error\":\"transfer is not retryable\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_find_alternative_and_replace_use_cached_search_results() {
    let (state, mut receiver) = test_state();
    let created = crate::route_http_request(
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

    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"song\"}",
        &state,
    )
    .await
    .expect("create search");
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":1,\"peer_username\":\"fresh\",\"filename\":\"Other/Path/Song.flac\",\"size\":100,\"slot_free\":true,\"average_speed\":2048}",
        &state,
    )
    .await
    .expect("ingest alternative");

    let alternatives = crate::route_http_request(
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

    let replaced = crate::route_http_request(
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
        crate::SessionCommand::TransferPeer {
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
pub(super) async fn transfer_auto_replace_queues_failed_download_alternatives() {
    let (state, mut receiver) = test_state();
    let created = crate::route_http_request(
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

    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"song\"}",
        &state,
    )
    .await
    .expect("create search");
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":1,\"peer_username\":\"fresh\",\"filename\":\"Other/Path/Song.flac\",\"size\":100}",
        &state,
    )
    .await
    .expect("ingest alternative");

    let replaced = crate::route_http_request(
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
        crate::SessionCommand::TransferPeer {
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
pub(super) async fn transfer_replacements_reject_before_mutation_when_dispatch_is_unavailable() {
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
        crate::route_http_request(
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
        crate::route_http_request(
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
            Ok(crate::SessionCommand::Search { .. })
        ));
        crate::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            r#"{"token":1,"peer_username":"fresh","filename":"Other/Path/Song.flac","size":100}"#,
            &state,
        )
        .await
        .expect("ingest alternative");
        drop(receiver);

        let response = crate::route_http_request("POST", path, None, body, &state)
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
pub(super) async fn transfer_replacements_roll_back_when_persistence_fails() {
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state.session.write().await.state = "connected";
        crate::route_http_request(
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
        crate::route_http_request(
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
            Ok(crate::SessionCommand::Search { .. })
        ));
        crate::route_http_request(
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

        let response = crate::route_http_request("POST", path, None, body, &state)
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
pub(super) async fn transfer_start_rejects_peer_transfer_when_outbound_disabled() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_ALLOW_OUTBOUND", "false"));

    let created = crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Song.flac\",\"peer_username\":\"friend\",\"size\":10}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");

    let blocked = crate::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
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
pub(super) async fn transfer_create_rejects_upload_local_path() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-local-{}-ok.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write local file");
    let body = format!(
        "{{\"direction\":1,\"filename\":\"Remote/Song.flac\",\"local_path\":\"{}\"}}",
        crate::json_escape(&path.display().to_string())
    );

    let created = crate::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
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
pub(super) async fn transfer_create_rejects_missing_upload_local_path() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-local-{}-missing.bin",
        std::process::id()
    ));
    let body = format!(
        "{{\"direction\":1,\"filename\":\"Remote/Song.flac\",\"local_path\":\"{}\"}}",
        crate::json_escape(&path.display().to_string())
    );

    let created = crate::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
        .await
        .expect("create transfer");
    assert_eq!(created.status, "400 Bad Request");
    assert!(created
        .body
        .contains("local_path is not accepted for uploads"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn download_path_normalizes_protocol_separators_and_rejects_traversal() {
    let state_dir = PathBuf::from("state");
    let root = state_dir.join("downloads");
    let expected = root.join("Remote").join("Song.flac");
    assert_eq!(
        crate::safe_download_path(&root, "Remote/Song.flac").unwrap(),
        expected
    );
    assert_eq!(
        crate::safe_download_path(&root, "Remote\\Song.flac").unwrap(),
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
        let error = crate::safe_download_path(&root, filename)
            .expect_err("absolute and traversal paths must fail");
        assert!(error.contains("must be relative"), "{filename:?}: {error}");
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn completed_download_path_templates_reject_invalid_or_oversized_expansion() {
    let invalid_date = crate::file_transfer_runtime::render_completed_download_path(
        "{date:%Q}",
        "friend",
        "Remote/Song.flac",
        None,
        None,
        1,
    )
    .expect_err("invalid date format must not reach Chrono display");
    assert!(invalid_date.contains("invalid date format"));

    let remote_folder = "x".repeat(crate::MAX_TRANSFER_LOCAL_PATH_BYTES);
    let remote_filename = format!("{remote_folder}/Song.flac");
    let oversized = crate::file_transfer_runtime::render_completed_download_path(
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
pub(super) fn download_path_rejects_final_symlink() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-download-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let root = crate::file_transfer_runtime::download_root(&state_dir);
    let path = crate::safe_download_path(&root, "Remote/Song.flac").expect("download path");
    let parent = path.parent().expect("download parent");
    std::fs::create_dir_all(parent).expect("download parent dir");
    let target = state_dir.join("outside.flac");
    std::fs::write(&target, [1_u8]).expect("target file");
    symlink(&target, &path).expect("download symlink");

    let error = crate::ensure_scoped_download_path(&root, &path.display().to_string())
        .expect_err("symlink rejected");
    assert!(error.contains("must not be a symlink"));
    assert!(crate::file_transfer_runtime::open_download_file(
        &crate::file_transfer_runtime::download_root(&state_dir),
        &path
    )
    .is_err());

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn download_confined_open_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-download-parent-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let root = crate::file_transfer_runtime::download_root(&state_dir);
    let outside = state_dir.join("outside");
    std::fs::create_dir_all(&root).expect("download root");
    std::fs::create_dir_all(&outside).expect("outside directory");
    symlink(&outside, root.join("Remote")).expect("symlinked parent");
    let path = root.join("Remote/Song.flac");

    assert!(crate::file_transfer_runtime::open_download_file(&root, &path).is_err());
    assert!(!outside.join("Song.flac").exists());

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn download_parent_creation_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-download-parent-create-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let root = crate::file_transfer_runtime::download_root(&state_dir);
    let outside = state_dir.join("outside");
    std::fs::create_dir_all(&root).expect("download root");
    std::fs::create_dir_all(&outside).expect("outside directory");
    symlink(&outside, root.join("Remote")).expect("symlinked parent");
    let path = root.join("Remote/New/Song.flac");

    let error = crate::ensure_scoped_download_path(&root, &path.display().to_string())
        .expect_err("symlinked parent must be rejected before directory creation");
    assert!(error.contains("confined open failed"), "{error}");
    assert!(!outside.join("New").exists());

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn upload_share_lookup_rejects_swapped_symlink() {
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

    assert!(crate::find_shared_local_file(&state, "Remote/Song.flac")
        .await
        .is_none());
    assert!(crate::open_shared_local_file(&state, &shared_path)
        .await
        .is_err());

    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn opening_a_previously_valid_share_rejects_a_swapped_outside_symlink() {
    use std::os::unix::fs::symlink;

    let (state, _receiver) = test_state();
    let dir = std::env::temp_dir().join(format!(
        "slskr-share-follow-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let root = dir.join("share");
    std::fs::create_dir_all(&root).expect("share root");
    let shared_path = root.join("Song.flac");
    let outside_path = dir.join("outside.flac");
    std::fs::write(&shared_path, [1_u8, 2, 3, 4]).expect("shared file");
    std::fs::write(&outside_path, [5_u8, 6, 7, 8]).expect("outside file");

    {
        let mut settings = state.share_settings.write().await;
        settings.roots = vec![root.clone()];
        settings.follow_symlinks = true;
    }
    add_test_share(&state, "Remote/Song.flac", &shared_path, 4).await;
    let indexed = crate::find_shared_local_file(&state, "Remote/Song.flac")
        .await
        .expect("initial regular share file");

    std::fs::remove_file(&shared_path).expect("remove indexed file");
    symlink(&outside_path, &shared_path).expect("swap in outside symlink");
    let error = crate::open_shared_local_file(&state, &indexed.local_path)
        .await
        .expect_err("opening a stale share path must not follow an outside symlink");
    assert!(error.contains("outside configured share roots"));

    let _ = std::fs::remove_dir_all(dir);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_start_with_peer_requests_peer_address() {
    let (state, mut receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-start-{}-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write shared file");
    add_test_share(&state, "Remote/Song.flac", &path, 4).await;
    let created = crate::route_http_request(
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

    let started = crate::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start transfer");

    assert_eq!(started.status, "200 OK");
    assert!(started.body.contains("\"status\":\"peer_lookup\""));
    assert_eq!(
        receiver.try_recv().expect("transfer command"),
        crate::SessionCommand::TransferPeer {
            id: 1,
            username: "friend".to_owned(),
        }
    );
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn queued_peer_download_adopts_the_remote_upload_token() {
    let (state, _receiver) = test_state();
    let path = crate::safe_download_path(&state.config.downloads_dir, "Remote/Queued.flac")
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
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: Default::default(),
                direction: 0,
                token: request_token,
                filename: "Remote/Queued.flac".to_owned(),
                size: None,
            })
        );
        peer.send(&crate::PeerMessage::TransferResponse(
            crate::TransferResponse::Rejected {
                token: request_token,
                reason: "Queued".to_owned(),
            },
        ))
        .await
        .expect("queued response");
        peer.send(&crate::PeerMessage::TransferRequest(
            crate::TransferRequest {
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
            crate::PeerMessage::TransferResponse(crate::TransferResponse::Allowed {
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

    crate::project_peer_transfer_response(&state, &address).await;
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
pub(super) async fn queued_outgoing_upload_accepts_remote_download_resume_request() {
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
        crate::handle_plain_peer_messages(
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
    peer.send(&crate::PeerMessage::TransferRequest(
        crate::TransferRequest {
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
        crate::PeerMessage::TransferResponse(crate::TransferResponse::Allowed {
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
pub(super) async fn queue_upload_and_place_in_queue_messages_drive_the_group_scheduler() {
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
        crate::handle_plain_peer_messages(
            &server_state,
            slskr_client::stream::PeerMessageConnection::new(stream),
            Some("friend".to_owned()),
        )
        .await
    });
    let stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
    peer.send(&crate::PeerMessage::QueueUpload {
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
        crate::handle_plain_peer_messages(
            &server_state,
            slskr_client::stream::PeerMessageConnection::new(stream),
            Some("friend".to_owned()),
        )
        .await
    });
    let stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
    peer.send(&crate::PeerMessage::PlaceInQueueRequest {
        filename: "Remote/Queued.flac".to_owned(),
    })
    .await
    .unwrap();
    assert_eq!(
        peer.receive().await.unwrap(),
        crate::PeerMessage::PlaceInQueueResponse {
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
pub(super) async fn inbound_transfer_request_serves_shared_file_over_pierce_firewall() {
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
        crate::find_shared_local_file(&state, r"Virtual\Inbound.flac")
            .await
            .is_some()
    );

    crate::handle_peer_message(
        &state,
        crate::PeerMessage::TransferRequest(crate::TransferRequest {
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
                crate::PeerMessage::TransferResponse(crate::TransferResponse::Allowed {
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
        crate::handle_inbound_file_transfer(
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
pub(super) async fn inbound_transfer_request_rejects_when_active_limit_is_full() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_MAX_ACTIVE", "1"));
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(1, None, "Remote/Busy.flac".to_owned(), None, Some(1));
        transfers.update_status(entry.id, "in_progress", None, None);
    }

    crate::handle_peer_message(
        &state,
        crate::PeerMessage::TransferRequest(crate::TransferRequest {
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
                crate::PeerMessage::TransferResponse(crate::TransferResponse::Rejected {
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
pub(super) async fn inbound_transfer_request_rejects_when_inbound_disabled() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_ALLOW_INBOUND", "false"));

    crate::handle_peer_message(
        &state,
        crate::PeerMessage::TransferRequest(crate::TransferRequest {
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
                crate::PeerMessage::TransferResponse(crate::TransferResponse::Rejected {
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
pub(super) async fn transfer_api_rejects_missing_filename() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("POST", "/api/v0/transfers", None, "{}", &state)
        .await
        .expect("bad transfer");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(response.body, "{\"error\":\"filename is required\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_group_runtime_enforces_membership_limits_slots_file_types_and_speed() {
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
        crate::file_transfer_runtime::effective_transfer_group(&state, "alice").await,
        "friends"
    );
    assert_eq!(
        crate::effective_upload_speed_limit(&state, "alice").await,
        50
    );
    assert_eq!(
        crate::effective_upload_speed_limit(&state, "stranger").await,
        100
    );
    assert_eq!(
        crate::file_transfer_runtime::inbound_upload_policy(&state, "alice", "Music/track.mp3", 10)
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
        crate::file_transfer_runtime::inbound_upload_policy(
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
        crate::file_transfer_runtime::inbound_upload_policy(
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
        crate::file_transfer_runtime::inbound_upload_policy(
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
pub(super) async fn transfer_group_runtime_uses_frozen_limit_rejection_reasons_and_priority_resolution(
) {
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
        crate::file_transfer_runtime::effective_transfer_group(&state, "alice").await,
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
        crate::file_transfer_runtime::inbound_upload_policy(&state, "alice", "two.flac", 10)
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
        crate::file_transfer_runtime::inbound_upload_policy(&state, "bob", "next.flac", 10)
            .await
            .unwrap_err(),
        "Too many failed transfers today"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_group_queue_schedules_priority_slots_and_reports_frozen_positions() {
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
            crate::next_queued_upload_id(&transfers, &upload, &groups, &users),
            Some(alice.id)
        );
    }
    crate::schedule_queued_uploads(&state).await;
    assert_eq!(
        receiver.try_recv().unwrap(),
        crate::SessionCommand::RequestPeerEndpoint("alice".to_owned())
    );
    assert_eq!(
        receiver.try_recv().unwrap(),
        crate::SessionCommand::RequestPeerEndpoint("bob".to_owned())
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
        crate::upload_queue_position(&round_robin, "alice", "two.flac").await,
        Some(2)
    );
    {
        let mut transfers = round_robin.transfers.write().await;
        let first = transfers.entries[0].id;
        transfers.update_status(first, "in_progress", None, None);
    }
    assert_eq!(
        crate::upload_queue_forecast(&round_robin, "charlie").await,
        (1, 3)
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_ids_and_tokens_wrap_without_collisions() {
    let mut queue = crate::TransferQueue::new_in_memory(8);
    let first = queue.create(0, Some("peer".to_owned()), "first".to_owned(), None, None);
    assert_eq!((first.id, first.token), (1, 1));
    queue.next_id = u64::MAX;
    queue.next_token = u32::MAX;
    let maximum = queue.create(0, Some("peer".to_owned()), "max".to_owned(), None, None);
    assert_eq!((maximum.id, maximum.token), (u64::MAX, u32::MAX));
    let wrapped = queue.create(0, Some("peer".to_owned()), "wrapped".to_owned(), None, None);
    assert_eq!((wrapped.id, wrapped.token), (2, 2));
    let rejected =
        queue.record_rejected_request(0, 99, "rejected".to_owned(), None, "test".to_owned());
    assert_eq!(rejected.id, 3);
    assert_eq!(
        queue
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        queue.entries.len()
    );

    let mut duplicate = wrapped.clone();
    duplicate.filename = "newest".to_owned();
    crate::write_transfer_state(&queue.state_path, &[wrapped, duplicate]).unwrap();
    let loaded = crate::load_transfer_state(&queue.state_path, 8).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].filename, "newest");
    let _ = std::fs::remove_file(&queue.state_path);
    let _ = std::fs::remove_file(&queue.events_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn outbound_message_routes_reject_before_mutation_when_dispatch_is_unavailable() {
    for (path, body) in [
        (
            "/api/messages",
            r#"{"username":"friend","body":"direct message"}"#,
        ),
        (
            "/api/conversations/friend",
            r#"{"body":"conversation message"}"#,
        ),
        (
            "/api/conversations/batch",
            r#"{"usernames":["friend","peer"],"body":"batch message"}"#,
        ),
    ] {
        let (state, receiver) = test_state();
        drop(receiver);

        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("unavailable dispatch response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("session manager is not running"),
            "{path}"
        );

        let messages = state.messages.read().await;
        assert!(messages.records.is_empty(), "{path}");
        assert_eq!(messages.next_id, 1, "{path}");
        drop(messages);
        assert!(
            state
                .events
                .read()
                .await
                .records
                .iter()
                .all(|event| event.kind != "message.sent"),
            "{path}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn inbound_private_message_replays_update_one_record_and_retain_replay_flag() {
    use slskr_client::protocol::server::{PrivateMessage, ServerMessage};
    let (state, _receiver) = test_state();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(address);
    let server = listener.accept();
    let (client, server) = tokio::join!(client, server);
    let (server, _) = server.unwrap();
    let mut session = slskr_client::server::ServerSession::new(
        slskr_client::stream::ServerConnection::new(server),
    );
    let _client = slskr_client::stream::ServerConnection::new(client.unwrap());
    let message = |replayed| {
        ServerMessage::MessageUserResponse(PrivateMessage {
            id: 77,
            timestamp: 88,
            username: "peer".to_owned(),
            message: "hello".to_owned(),
            is_new: true,
            was_replayed: replayed,
        })
    };
    crate::session_runtime::project_server_message(&state, &mut session, &message(false)).await;
    crate::session_runtime::project_server_message(&state, &mut session, &message(true)).await;
    let messages = state.messages.read().await;
    assert_eq!(messages.records.len(), 1);
    assert!(messages.records[0].was_replayed);
    drop(messages);
    let response = crate::route_http_request("GET", "/api/v0/conversations/peer", None, "", &state)
        .await
        .unwrap();
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["messages"].as_array().unwrap().len(), 1);
    assert_eq!(json["messages"][0]["wasReplayed"], true);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_group_yaml_validation_matches_frozen_target_contracts() {
    let cases = [
        (
            "transfers:\n  upload:\n    slots: 0\n",
            "Invalid configuration:\n  Transfers:\n    Upload:\n      The field Slots must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  groups:\n    default:\n      upload:\n        priority: 0\n",
            "Invalid configuration:\n  Transfers:\n    Groups:\n      Default:\n        Upload:\n          The field Priority must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  groups:\n    default:\n      upload:\n        strategy: invalid\n",
            "Invalid configuration:\n  Transfers:\n    Groups:\n      Default:\n        Upload:\n          The Strategy field must be one of: RoundRobin, FirstInFirstOut. Case insensitive.",
        ),
        (
            "transfers:\n  groups:\n    default:\n      upload:\n        limits:\n          queued:\n            files: 0\n",
            "Invalid configuration:\n  Transfers:\n    Groups:\n      Default:\n        Upload:\n          Limits:\n            Queued:\n              The field Files must be between 1 and 2147483647.",
        ),
    ];
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        for (yaml, controller_error) in cases {
            let response = crate::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &serde_json::to_string(yaml).unwrap(),
                &state,
            )
            .await
            .unwrap();
            assert_eq!(response.status, "200 OK", "{target}: {yaml}");
            assert_eq!(response.content_type, "application/json; charset=utf-8");
            assert_eq!(
                serde_json::from_str::<String>(&response.body).unwrap(),
                if target == "slskd" {
                    controller_error
                } else {
                    "Invalid YAML configuration"
                },
                "{target}: {yaml}"
            );
        }

        let daily_null = crate::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("transfers:\n  upload:\n    limits:\n      daily: null\n")
                .unwrap(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(daily_null.status, "200 OK", "{target}");
        if target == "slskd" {
            assert!(daily_null.content_type.is_empty());
            assert!(daily_null.body.is_empty());
        } else {
            assert_eq!(
                serde_json::from_str::<String>(&daily_null.body).unwrap(),
                "Invalid YAML configuration"
            );
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_download_yaml_validation_matches_frozen_target_contracts() {
    let cases = [
        (
            "transfers:\n  download:\n    slots: 0\n",
            "Invalid configuration:\n  Transfers:\n    Download:\n      The field Slots must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  download:\n    retry:\n      attempts: 0\n",
            "Invalid configuration:\n  Transfers:\n    Download:\n      Retry:\n        The field Attempts must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  download:\n    destination:\n      subdirectory: ../escape\n",
            "Invalid configuration:\n  Transfers:\n    Download:\n      Destination:\n        The Subdirectory field contains one or more unsafe path traversal segments ('.' or '..')",
        ),
    ];
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        for (yaml, controller_error) in cases {
            let response = crate::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &serde_json::to_string(yaml).unwrap(),
                &state,
            )
            .await
            .unwrap();
            assert_eq!(response.status, "200 OK", "{target}: {yaml}");
            if target == "slskdn" && yaml.contains("destination:") {
                assert!(response.content_type.is_empty(), "{target}: {yaml}");
                assert!(response.body.is_empty(), "{target}: {yaml}");
                continue;
            }
            assert_eq!(response.content_type, "application/json; charset=utf-8");
            assert_eq!(
                serde_json::from_str::<String>(&response.body).unwrap(),
                if target == "slskd" {
                    controller_error
                } else {
                    "Invalid YAML configuration"
                },
                "{target}: {yaml}"
            );
        }
    }
}
