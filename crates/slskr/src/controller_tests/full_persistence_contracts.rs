//! Controller full persistence contracts ownership.

use super::*;

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn state_directory_is_private_and_rejects_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let root = std::env::temp_dir().join(format!(
        "slskr-private-state-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let state_dir = root.join("state");
    crate::ensure_private_state_dir(&state_dir).expect("create private state dir");
    assert_eq!(
        std::fs::metadata(&state_dir)
            .expect("state metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );

    let linked_state = root.join("linked-state");
    symlink(&state_dir, &linked_state).expect("create state symlink");
    let error =
        crate::ensure_private_state_dir(&linked_state).expect_err("state symlink must be rejected");
    assert!(error.contains("must be a real directory"), "{error}");

    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn persisted_http_logs_redact_stream_ticket_paths() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKD_HTTP_LOGGING", "true"));
    crate::record_http_log(
        &state,
        "request-1",
        &crate::logging::HttpTransactionLog {
            request: crate::logging::HttpRequestLog {
                method: "GET".to_owned(),
                path: "/api/v0/peer-streams/bearer-ticket-secret".to_owned(),
                query: None,
                remote_addr: None,
                timestamp: "fixture".to_owned(),
            },
            response: crate::logging::HttpResponseLog {
                status_code: 200,
                content_length: 0,
                duration_ms: 1,
                error: None,
            },
        },
    )
    .await;

    let events = state.events.read().await;
    let detail = events.records[0].detail.as_deref().expect("log detail");
    assert!(detail.contains("/api/v0/peer-streams/<redacted>"));
    assert!(!detail.contains("bearer-ticket-secret"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn database_admin_aliases_report_live_state_and_cleanup() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let old_message = crate::persistence::MessageRecord {
        id: "old".to_owned(),
        username: "friend".to_owned(),
        content: "old".to_owned(),
        direction: "inbound".to_owned(),
        read: false,
        created_at: 1,
        source_id: None,
        source_timestamp: None,
        was_replayed: false,
    };
    db.insert_message(&old_message).await.expect("old message");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    {
        let mut transfers = state.transfers.write().await;
        let failed = transfers.create(
            0,
            Some("peer".to_owned()),
            "Remote/Failed.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(failed.id, "failed", Some(1), Some("offline".to_owned()));
    }

    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("admin database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["enabled"], true);
    assert_eq!(stats_json["healthy"], true);
    assert_eq!(stats_json["messages"], 1);
    assert_eq!(stats_json["persisted"]["messages"], 1);
    assert_eq!(stats_json["projections"]["transfers"], 1);

    let cleanup = crate::route_http_request(
        "POST",
        "/api/database/cleanup",
        None,
        "{\"days\":0}",
        &state,
    )
    .await
    .expect("legacy database cleanup");
    assert_eq!(cleanup.status, "200 OK");
    let cleanup_json = serde_json::from_str::<serde_json::Value>(&cleanup.body).unwrap();
    assert_eq!(cleanup_json["status"], "ok");
    assert_eq!(cleanup_json["persisted"]["enabled"], true);
    assert_eq!(cleanup_json["cleaned"], 1);
    assert_eq!(cleanup_json["pruned_transfers"], 1);

    let vacuum = crate::route_http_request("POST", "/api/admin/database/vacuum", None, "", &state)
        .await
        .expect("admin database vacuum");
    assert_eq!(vacuum.status, "200 OK");
    let vacuum_json = serde_json::from_str::<serde_json::Value>(&vacuum.body).unwrap();
    assert_eq!(vacuum_json["enabled"], true);
    assert_eq!(vacuum_json["vacuumed"], true);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn database_admin_errors_are_redacted_and_do_not_report_success() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    db.close_for_test().await;
    let raw_error = db
        .get_stats()
        .await
        .expect_err("closed database should reject statistics")
        .to_string();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db),
    );

    let stats = crate::database_stats_value(&state).await;
    assert_eq!(stats["healthy"], false);
    assert_eq!(
        stats["persisted"]["error"],
        "database statistics unavailable"
    );

    let cleanup = crate::database_cleanup_value(&state, "{\"days\":0}").await;
    assert_eq!(cleanup["status"], "error");
    assert_eq!(
        cleanup["persisted"]["error"],
        "database cleanup unavailable"
    );

    let vacuum = crate::database_vacuum_value(&state).await;
    assert_eq!(vacuum["status"], "error");
    assert_eq!(vacuum["error"], "database vacuum unavailable");

    for value in [stats, cleanup, vacuum] {
        let json = value.to_string();
        assert!(!json.contains(&raw_error), "leaked database error: {json}");
        assert!(!json.to_ascii_lowercase().contains("pool closed"), "{json}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn durable_room_routes_surface_connected_dispatch_failures_in_session_health() {
    let (state, receiver) = test_state();
    state.session.write().await.state = "connected";
    drop(receiver);

    let joined = crate::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .expect("join response");
    assert_eq!(joined.status, "201 Created");
    assert!(state.rooms.read().await.records[0].joined);
    assert_eq!(
        state.session.read().await.last_error.as_deref(),
        Some("room join for music dispatch failed: session manager is not running")
    );

    let left = crate::route_http_request("DELETE", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .expect("leave response");
    assert_eq!(left.status, "200 OK");
    assert!(!state.rooms.read().await.records[0].joined);
    assert_eq!(
        state.session.read().await.last_error.as_deref(),
        Some("room leave for music dispatch failed: session manager is not running")
    );

    let events = state.events.read().await;
    assert!(events.records.iter().any(|event| {
        event.kind == "log.created"
            && event
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("room join for music dispatch failed"))
    }));
    assert!(events.records.iter().any(|event| {
        event.kind == "log.created"
            && event
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("room leave for music dispatch failed"))
    }));
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn state_file_io_rejects_symlinks_without_touching_targets() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-state-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let target = state_dir.join("target.json");
    std::fs::write(&target, "keep").expect("target");

    let state_path = crate::transfer_state_path(&state_dir);
    symlink(&target, &state_path).expect("state symlink");
    let error = crate::load_transfer_state(&state_path, 100).expect_err("reject state symlink");
    assert!(error.contains("transfer state open failed"));

    let destination = state_dir.join("destination.json");
    let temporary = state_dir.join("temporary.json");
    symlink(&target, &temporary).expect("temporary symlink");
    let error =
        crate::write_file_atomic_with_temp_path(&destination, &temporary, br#"{"version":1}"#)
            .expect_err("reject occupied temporary path");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        std::fs::read_to_string(&target).expect("target unchanged"),
        "keep"
    );
    assert!(!destination.exists());

    let events_path = crate::transfer_events_path(&state_dir);
    symlink(&target, &events_path).expect("event symlink");
    let entry = crate::TransferEntry {
        id: 1,
        direction: 0,
        token: 1,
        peer_username: None,
        filename: "Remote/Song.flac".to_owned(),
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
        size: Some(1),
        bytes_transferred: 0,
        status: "queued".to_owned(),
        reason: None,
        requested_at: 1,
        started_at: None,
        start_offset: 0,
        updated_at: 1,
        updated_at_ms: 1_000,
        previous_status: None,
    };
    let error =
        crate::append_transfer_event(&events_path, &entry).expect_err("reject event symlink");
    assert!(error.contains("must be a regular file"));
    assert_eq!(
        std::fs::read_to_string(&target).expect("target still unchanged"),
        "keep"
    );

    let _ = std::fs::remove_dir_all(state_dir);
}
