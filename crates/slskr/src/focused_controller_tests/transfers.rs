use super::fixtures::*;

#[tokio::test]
async fn versioned_download_range_sources_use_verified_executor() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));
    let response = crate::route_http_request(
        "POST",
        "/api/v0/multisource/download",
        None,
        &serde_json::json!({
            "filename": "Track.flac",
            "fileSize": 42,
            "expectedHash": "00".repeat(32),
            "sources": [
                {"username": "alice", "url": "http://127.0.0.1:9/file"},
                {"username": "bob", "url": "http://127.0.0.1:9/file"}
            ]
        })
        .to_string(),
        &state,
    )
    .await
    .expect("versioned download response");
    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"success\":false"));
    assert!(!response
        .body
        .contains("Multi-source download is unavailable"));
    let multisource_store = state.multisource.read().await;
    let jobs = multisource_store.list();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, "failed");

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn versioned_swarm_requires_expected_hash_before_queueing() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));

    let response = crate::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        r#"{"filename":"Track.flac","size":42}"#,
        &state,
    )
    .await
    .expect("missing expected hash response");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("expectedHash is required for verified swarm execution"));
    assert!(state.multisource.read().await.list().is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_mutation_rollback_preserves_newer_rows() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let original = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Original.flac".to_owned(),
            None,
            Some(10),
        )
    };
    let (previous, mutated, failed_creation_id) = {
        let mut transfers = state.transfers.write().await;
        let previous = transfers.mutation_snapshot();
        transfers
            .update_status(
                original.id,
                "cancelled",
                None,
                Some("failed cancellation".to_owned()),
            )
            .expect("mutate original transfer");
        let failed_creation = transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Failed-Creation.flac".to_owned(),
            None,
            Some(11),
        );
        let mutated = transfers.mutation_snapshot();
        (previous, mutated, failed_creation.id)
    };
    let concurrent = {
        let mut transfers = state.transfers.write().await;
        transfers
            .update_status(original.id, "in_progress", Some(4), None)
            .expect("apply newer transfer update");
        transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Concurrent.flac".to_owned(),
            None,
            Some(12),
        )
    };

    assert!(crate::rollback_transfer_mutation_if_unchanged(&state, previous, mutated).await);
    {
        let transfers = state.transfers.read().await;
        let original_after = transfers
            .entries
            .iter()
            .find(|entry| entry.id == original.id)
            .expect("newer original transfer remains");
        assert_eq!(original_after.status, "in_progress");
        assert_eq!(original_after.bytes_transferred, 4);
        assert!(transfers
            .entries
            .iter()
            .all(|entry| entry.id != failed_creation_id));
        assert!(transfers
            .entries
            .iter()
            .any(|entry| entry.id == concurrent.id));
        assert!(transfers.next_id > concurrent.id);
    }

    let (previous, mutated) = {
        let mut transfers = state.transfers.write().await;
        let previous = transfers.mutation_snapshot();
        transfers
            .update_status(
                concurrent.id,
                "cancelled",
                None,
                Some("failed cancellation".to_owned()),
            )
            .expect("mutate concurrent transfer");
        (previous, transfers.mutation_snapshot())
    };
    assert!(crate::rollback_transfer_mutation_if_unchanged(&state, previous, mutated).await);
    let restored = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.id == concurrent.id)
        .cloned()
        .expect("restored transfer remains");
    assert_eq!(restored.status, "queued");

    let deleted = {
        state.transfers.write().await.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Failed-Deletion.flac".to_owned(),
            None,
            Some(13),
        )
    };
    let (previous, mutated) = {
        let mut transfers = state.transfers.write().await;
        let previous = transfers.mutation_snapshot();
        assert_eq!(transfers.remove_entries(&[deleted.id]).len(), 1);
        (previous, transfers.mutation_snapshot())
    };
    let concurrent_after_delete = {
        state.transfers.write().await.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Concurrent-After-Deletion.flac".to_owned(),
            None,
            Some(14),
        )
    };
    assert!(crate::rollback_transfer_mutation_if_unchanged(&state, previous, mutated).await);
    {
        let transfers = state.transfers.read().await;
        assert!(transfers.entries.iter().any(|entry| entry == &deleted));
        assert!(transfers
            .entries
            .iter()
            .any(|entry| entry.id == concurrent_after_delete.id));
    }

    let stale_expected = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Changed-Before-Removal.flac".to_owned(),
            None,
            Some(15),
        );
        transfers
            .update_status(entry.id, "in_progress", Some(5), None)
            .expect("update staged transfer before rollback");
        entry
    };
    assert!(crate::remove_transfer_entries_if_unchanged(
        &state,
        std::slice::from_ref(&stale_expected)
    )
    .await
    .is_empty());
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|entry| entry.id == stale_expected.id && entry.status == "in_progress"));

    let removable = {
        state.transfers.write().await.create(
            0,
            Some("rollback-peer".to_owned()),
            "Remote/Unchanged-Staged-Transfer.flac".to_owned(),
            None,
            Some(16),
        )
    };
    assert_eq!(
        crate::remove_transfer_entries_if_unchanged(&state, std::slice::from_ref(&removable)).await,
        vec![removable.clone()]
    );
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .all(|entry| entry.id != removable.id));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}
#[tokio::test]
async fn transfer_durability_snapshots_after_lock_and_rehydrates_in_event_order() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let events_path = crate::transfer_events_path(&state.config.state_dir);
    let state_path = crate::transfer_state_path(&state.config.state_dir);
    let events_before = fs::read_to_string(&events_path).expect("transfer event header");
    let state_before = fs::read_to_string(&state_path).ok();

    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("restart-peer".to_owned()),
            "Remote/Restart.flac".to_owned(),
            None,
            Some(100),
        );
        transfers.update_status(entry.id, "in_progress", Some(25), None);

        // Queue mutations only snapshot under the lock; no event append or
        // state rewrite is allowed until the guard has been released.
        assert_eq!(
            fs::read_to_string(&events_path).expect("events while locked"),
            events_before
        );
        assert_eq!(fs::read_to_string(&state_path).ok(), state_before);
    }

    crate::persist_transfer_durability(&state).await;
    let events_after = fs::read_to_string(&events_path).expect("persisted transfer events");
    assert!(events_after.contains("\tqueued\t"));
    assert!(events_after.contains("\t25\tin_progress\t"));
    assert_eq!(
        events_after.lines().count(),
        events_before.lines().count() + 2
    );
    let persisted = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&state_path).expect("persisted transfer state"),
    )
    .expect("transfer state JSON");
    assert_eq!(persisted["entries"][0]["status"], "in_progress");

    let restarted = crate::TransferQueue::new(&state.config);
    assert_eq!(restarted.entries.len(), 1);
    assert_eq!(restarted.entries[0].status, "queued");
    assert_eq!(
        restarted.entries[0].reason.as_deref(),
        Some("resumed after restart")
    );
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_request_name_update_advances_existing_database_revision() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create transfer request-name database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let entry = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("revision-peer".to_owned()),
            "Remote/Revision.flac".to_owned(),
            None,
            Some(100),
        );
        let mut entry = entry;
        entry.updated_at_ms = entry.updated_at_ms.saturating_add(30_000);
        let current = transfers
            .entries
            .iter_mut()
            .find(|current| current.id == entry.id)
            .expect("created transfer remains in queue");
        current.updated_at_ms = entry.updated_at_ms;
        entry
    };
    let initial_revision = entry.updated_at_ms;
    db.insert_transfer_records_with_events(&[(
        crate::persisted_transfer_record(&entry),
        crate::persisted_transfer_event_record(&entry),
    )])
    .await
    .expect("seed durable transfer snapshot");

    let request_id = entry.request_id.as_deref().expect("download request id");
    let headers = crate::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:1".parse().expect("loopback test address")),
        ..crate::RequestSecurityHeaders::default()
    };
    let response = crate::route_dispatch::route_http_request_with_headers(
        "PATCH",
        &format!("/api/v0/downloads/requests/{request_id}/name"),
        None,
        r#"{"name":"renamed request"}"#,
        &state,
        &headers,
    )
    .await
    .expect("rename download request");
    assert_eq!(response.status, "200 OK");

    let updated = db
        .get_transfer(&entry.id.to_string())
        .await
        .expect("read renamed transfer")
        .expect("renamed transfer remains persisted");
    assert_eq!(updated.request_name.as_deref(), Some("renamed request"));
    assert!(
        updated.updated_at_ms > i64::try_from(initial_revision).unwrap(),
        "the persisted transfer revision must advance past its prior value"
    );
    let live = state.transfers.read().await;
    let current = live
        .entries
        .iter()
        .find(|current| current.id == entry.id)
        .expect("renamed transfer remains in memory");
    assert_eq!(current.request_name.as_deref(), Some("renamed request"));
    assert_eq!(
        i64::try_from(current.updated_at_ms).unwrap(),
        updated.updated_at_ms
    );
    drop(live);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_durability_wait_does_not_block_queue_mutations() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("slow-disk-peer".to_owned()),
            "Remote/SlowDisk.flac".to_owned(),
            None,
            Some(100),
        );
    }

    let coordinator = state
        .transfers
        .read()
        .await
        .durability_snapshot()
        .expect("created transfer has a pending durability snapshot")
        .coordinator;
    // Hold the serialization gate to model a slow durability write. The
    // queue lock must be available while that persistence work is delayed.
    let coordinator_guard = coordinator.state.lock().await;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let persistence_state = Arc::clone(&state);
    let persistence = tokio::spawn(async move {
        let _ = started_tx.send(());
        crate::persist_transfer_durability(&persistence_state).await;
    });
    started_rx.await.expect("persistence task started");
    assert!(
        !persistence.is_finished(),
        "persistence is waiting on the gate"
    );

    let transfers = tokio::time::timeout(Duration::from_secs(1), state.transfers.write())
        .await
        .expect("durability work must not retain the transfer queue lock");
    assert_eq!(transfers.entries.len(), 1);
    drop(transfers);

    drop(coordinator_guard);
    persistence
        .await
        .expect("transfer durability task must finish");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_event_sync_delay_keeps_queue_writable() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let events_path = crate::transfer_events_path(&state.config.state_dir);
    {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("delayed-sync-peer".to_owned()),
            "Remote/DelayedSync.flac".to_owned(),
            None,
            Some(100),
        );
    }

    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    crate::transfer_state_io::pause_transfer_event_sync(
        events_path.clone(),
        started_tx,
        release_rx,
    );
    let persistence_state = Arc::clone(&state);
    let persistence = tokio::spawn(async move {
        crate::persist_transfer_durability(&persistence_state).await;
    });
    tokio::task::spawn_blocking(move || {
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("file-backed append reached the delayed sync point");
    })
    .await
    .expect("sync pause signal task finishes");

    let mut transfers = tokio::time::timeout(Duration::from_secs(1), state.transfers.write())
        .await
        .expect("slow file sync must not retain the transfer queue lock");
    assert_eq!(transfers.entries.len(), 1);
    transfers.update_status(1, "in_progress", Some(25), None);
    drop(transfers);

    release_tx.send(()).expect("release file sync");
    persistence.await.expect("durability flush finishes");
    crate::persist_transfer_durability(&state).await;
    let events = fs::read_to_string(events_path).expect("read durable transfer events");
    assert!(events.contains("\tqueued\t"));
    assert!(events.contains("\t25\tin_progress\t"));
    let restarted = crate::TransferQueue::new(&state.config);
    assert_eq!(restarted.entries.len(), 1);
    assert_eq!(restarted.entries[0].bytes_transferred, 25);
    assert_eq!(restarted.entries[0].status, "queued");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_durability_survives_killed_writer_process() {
    if let Some(root) = std::env::var_os("SLSKR_RF047_CHILD_DIR") {
        let root = std::path::PathBuf::from(root);
        let (state, _receiver) = test_state_with_env(
            MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().expect("state path")),
        );
        for index in 0..32 {
            {
                let mut transfers = state.transfers.write().await;
                let entry = transfers.create(
                    0,
                    Some("crash-peer".to_owned()),
                    format!("Remote/Crash-{index}.flac"),
                    None,
                    Some(100),
                );
                transfers.update_status(entry.id, "in_progress", Some(25), None);
            }
            crate::persist_transfer_durability(&state).await;
        }
        fs::write(root.join("ready"), b"durable").expect("announce durable transfer state");
        use std::io::Read;
        let _ = std::io::stdin().read(&mut [0_u8; 1]);
        return;
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "slskr-transfer-crash-restart-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create transfer restart directory");
    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "focused_controller_tests::transfers::transfer_durability_survives_killed_writer_process",
        ])
        .env("SLSKR_RF047_CHILD_DIR", &root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("start transfer writer process");
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !root.join("ready").exists() && std::time::Instant::now() < deadline {
        if child.try_wait().expect("check writer process").is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = root.join("ready").exists();
    if !ready {
        let _ = child.kill();
        let _ = child.wait();
        panic!("writer did not publish durable state before timeout");
    }
    child.kill().expect("kill transfer writer without shutdown");
    child.wait().expect("reap killed transfer writer");

    let (restarted, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().expect("state path")),
    );
    let transfers = restarted.transfers.read().await;
    assert_eq!(transfers.entries.len(), 32);
    for entry in &transfers.entries {
        assert_eq!(entry.bytes_transferred, 25);
        assert_eq!(entry.status, "queued");
        assert_eq!(entry.reason.as_deref(), Some("resumed after restart"));
    }
    drop(transfers);
    let events = fs::read_to_string(root.join("transfer-events.tsv"))
        .expect("read transfer events after restart");
    assert_eq!(events.matches("\tqueued\t").count(), 32);
    assert_eq!(events.matches("\t25\tin_progress\t").count(), 32);
    fs::remove_dir_all(root).expect("remove transfer restart directory");
}

#[tokio::test(flavor = "current_thread")]
async fn transfer_batch_path_preparation_releases_transfer_lock() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let held_destinations = state.destinations.write().await;
    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let mut preparation = Box::pin(crate::prepare_download_batch_transfer(
        &state,
        0,
        "peer",
        "Music/A.flac",
        42,
        batch_id,
        None,
    ));
    let waiting_on_destination = std::future::poll_fn(|context| {
        Poll::Ready(preparation.as_mut().poll(context).is_pending())
    })
    .await;
    assert!(
        waiting_on_destination,
        "path resolution should wait for destinations"
    );
    assert!(
        state.transfers.try_write().is_ok(),
        "batch path preparation must not retain the transfer lock"
    );

    drop(held_destinations);
    let prepared = preparation.await.expect("focused batch path preparation");
    assert_eq!(prepared.filename, "Music/A.flac");
    assert_eq!(prepared.size, 42);
    assert!(prepared.local_path.is_some());
    {
        let mut transfers = state.transfers.write().await;
        transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Music/A.flac".to_owned(),
            None,
            None,
            None,
            crate::TransferRequestDetails::default(),
        );
    }
    let (staged, failures) =
        crate::commit_prepared_download_batch_transfers(&state, "peer", batch_id, vec![prepared])
            .await;
    assert!(staged.is_empty());
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].1["filename"], "Music/A.flac");
    assert_eq!(state.transfers.read().await.entries.len(), 1);
    fs::remove_dir_all(&state.config.state_dir).expect("remove transfer batch lock test state");
}

#[tokio::test(flavor = "current_thread")]
async fn legacy_transfer_array_path_resolution_releases_transfer_lock() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let held_destinations = state.destinations.write().await;
    let body = r#"{"username":"peer","files":[{"filename":"Music/A.flac","size":42}]}"#;
    let mut request = Box::pin(crate::route_http_request(
        "POST",
        "/api/transfers",
        None,
        body,
        &state,
    ));
    let waiting_on_destination =
        std::future::poll_fn(|context| Poll::Ready(request.as_mut().poll(context).is_pending()))
            .await;
    assert!(
        waiting_on_destination,
        "legacy array enqueue should wait for destination resolution"
    );
    assert!(
        state.transfers.try_write().is_ok(),
        "legacy path resolution must not retain the transfer lock"
    );

    drop(held_destinations);
    let response = request.await.expect("legacy array enqueue response");
    assert_eq!(response.status, "200 OK");
    let response: serde_json::Value =
        serde_json::from_str(&response.body).expect("legacy enqueue response JSON");
    assert_eq!(response["queued"], 1);
    assert_eq!(response["transfers"].as_array().unwrap().len(), 1);
    assert_eq!(state.transfers.read().await.entries.len(), 1);
    fs::remove_dir_all(&state.config.state_dir).expect("remove legacy transfer lock test state");
}

#[tokio::test(flavor = "current_thread")]
async fn legacy_per_user_transfer_path_resolution_releases_transfer_lock() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let held_destinations = state.destinations.write().await;
    let body = r#"{"files":[{"filename":"Music/A.flac","size":42}]}"#;
    let mut request = Box::pin(crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        body,
        &state,
    ));
    let waiting_on_destination =
        std::future::poll_fn(|context| Poll::Ready(request.as_mut().poll(context).is_pending()))
            .await;
    assert!(
        waiting_on_destination,
        "per-user enqueue should wait for destination resolution"
    );
    assert!(
        state.transfers.try_write().is_ok(),
        "per-user path resolution must not retain the transfer lock"
    );

    drop(held_destinations);
    let response = request.await.expect("per-user enqueue response");
    assert_eq!(response.status, "200 OK");
    let response: serde_json::Value =
        serde_json::from_str(&response.body).expect("per-user enqueue response JSON");
    assert_eq!(response["queued"], 1);
    assert_eq!(response["transfers"].as_array().unwrap().len(), 1);
    assert_eq!(state.transfers.read().await.entries.len(), 1);
    fs::remove_dir_all(&state.config.state_dir).expect("remove per-user transfer lock test state");
}
