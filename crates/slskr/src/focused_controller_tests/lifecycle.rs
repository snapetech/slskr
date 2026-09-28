use super::fixtures::*;

#[tokio::test]
async fn lifecycle_command_delay_is_owned_and_rejects_after_shutdown() {
    let (mut state, _session) = test_state_with_env(MapEnv::default());
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    Arc::get_mut(&mut state).unwrap().lifecycle_commands = Some(sender.clone());
    crate::schedule_lifecycle_command(&state, crate::LifecycleCommand::Restart);
    assert_eq!(sender.strong_count(), 3);
    state.shutdown_managed_tasks().await;
    assert_eq!(sender.strong_count(), 2);
    assert!(receiver.try_recv().is_err());
    crate::schedule_lifecycle_command(&state, crate::LifecycleCommand::Shutdown);
    assert_eq!(sender.strong_count(), 2);
    assert!(receiver.try_recv().is_err());
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn lifecycle_command_preserves_flush_delay_and_joins_a_blocked_sender() {
    let (mut state, _session) = test_state_with_env(MapEnv::default());
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    Arc::get_mut(&mut state).unwrap().lifecycle_commands = Some(sender.clone());
    crate::schedule_lifecycle_command(&state, crate::LifecycleCommand::Restart);
    assert!(receiver.try_recv().is_err());
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), receiver.recv())
            .await
            .unwrap(),
        Some(crate::LifecycleCommand::Restart)
    );
    sender
        .send(crate::LifecycleCommand::Shutdown)
        .await
        .unwrap();
    crate::schedule_lifecycle_command(&state, crate::LifecycleCommand::Restart);
    tokio::time::sleep(Duration::from_millis(150)).await;
    state.shutdown_managed_tasks().await;
    assert_eq!(sender.strong_count(), 2);
    assert_eq!(
        receiver.try_recv().unwrap(),
        crate::LifecycleCommand::Shutdown
    );
    assert!(receiver.try_recv().is_err());
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn asynchronous_swarm_routes_reject_admission_after_shutdown() {
    for profile in ["slskd", "slskdn"] {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", profile));
        state.shutdown_managed_tasks().await;
        let body = serde_json::json!({
            "filename": "stopped.flac", "size": 4,
            "expectedHash": "00".repeat(32),
            "sources": [
                {"username": "first", "url": "http://0.0.0.0:1/file"},
                {"username": "second", "url": "http://0.0.0.0:2/file"}
            ]
        })
        .to_string();
        for path in [
            "/api/multisource/swarm/async",
            "/api/v0/multisource/swarm/async",
        ] {
            let response = crate::route_http_request("POST", path, None, &body, &state)
                .await
                .expect("stopped swarm route");
            assert_eq!(
                response.status, "503 Service Unavailable",
                "{profile} {path}"
            );
        }
        let jobs = state.multisource.read().await;
        assert_eq!(jobs.list().len(), 2);
        assert!(jobs.list().iter().all(|job| job.status == "failed"));
        drop(jobs);
        fs::remove_dir_all(&state.config.state_dir).unwrap();
    }
}

#[tokio::test]
async fn swarm_admission_after_shutdown_fails_without_creating_output() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    state.shutdown_managed_tasks().await;
    let request = crate::multisource::SwarmRequest {
        filename: "stopped.flac".to_owned(),
        file_size: 4,
        expected_hash: Some("00".repeat(32)),
        output_path: None,
        chunk_size: 2,
        sources: Vec::new(),
    };
    let id = "stopped-job".to_owned();
    let output = state.config.state_dir.join("stopped.flac");
    state
        .multisource
        .write()
        .await
        .insert(crate::multisource::new_job(
            id.clone(),
            &request,
            "stopped.flac".to_owned(),
            crate::unix_timestamp(),
        ));
    assert!(
        !crate::multisource::spawn_managed(
            &state,
            id.clone(),
            request,
            output.clone(),
            "stopped.flac".to_owned(),
        )
        .await
    );
    let jobs = state.multisource.read().await;
    let job = jobs.get(&id).unwrap();
    assert_eq!(job.status, "failed");
    assert_eq!(
        job.result.as_ref().unwrap().error.as_deref(),
        Some("daemon is shutting down")
    );
    assert!(!output.exists());
    drop(jobs);
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn http_listener_connection_tasks_share_capacity_and_join_with_managed_shutdown() {
    use std::sync::atomic::{AtomicBool, Ordering};

    struct DropMarker(Arc<AtomicBool>);

    impl Drop for DropMarker {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }

    let connections = Arc::new(crate::Semaphore::new(1));
    let registry = crate::ManagedTaskRegistry::default();
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = DropMarker(Arc::clone(&dropped));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();

    assert!(registry.spawn_bounded_http(&connections, async move {
        let _marker = marker;
        let _ = started_tx.send(());
        std::future::pending::<()>().await;
    },));
    started_rx.await.expect("HTTPS task should start");

    assert!(
        !registry.spawn_bounded_http(&connections, async {}),
        "the Unix listener must share the HTTPS connection limit"
    );
    assert_eq!(connections.available_permits(), 0);

    registry.shutdown().await;
    assert!(dropped.load(Ordering::Acquire));
    assert_eq!(connections.available_permits(), 1);

    assert!(
        !registry.spawn_bounded_http(&connections, async {}),
        "connection handlers must not be accepted after managed shutdown"
    );
}

#[tokio::test]
async fn application_projection_exposes_selected_runtime_profile() {
    for (reference_fixture, expected_profile) in [("slskd", "legacy"), ("slskdn", "native")] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", reference_fixture),
        );
        let response = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
            .await
            .expect("application projection");
        let body = serde_json::from_str::<serde_json::Value>(&response.body)
            .expect("application projection JSON");

        assert_eq!(response.status, "200 OK");
        assert_eq!(body["runtimeProfile"], expected_profile);
    }
}

#[tokio::test]
async fn watched_obfuscation_changes_mark_reconnect_once_while_connected() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    state.session.write().await.state = "connected";
    let yaml = "soulseek:\n  obfuscation:\n    enabled: false\n    mode: prefer\n    listen_port: 50302\n    advertise_regular_port: false\n    prefer_outbound: false\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(state.runtime.read().await.application_reconnect_pending);
    state.runtime.write().await.set_reconnect_pending(false);

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(!state.runtime.read().await.application_reconnect_pending);
}

#[tokio::test]
async fn watched_share_reload_cancels_and_rejects_stale_index_publication() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create watched-share database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let stale_snapshot = state.shares.read().await.clone();
    let stale_generation = state
        .share_settings_generation
        .load(std::sync::atomic::Ordering::Acquire);
    let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&cancellation));

    let new_root = state.config.state_dir.join("watched-share");
    fs::create_dir_all(&new_root).expect("create reloaded share root");
    fs::write(new_root.join("new.flac"), b"new share").expect("write reloaded share fixture");
    let yaml = format!(
        "shares:\n  directories:\n    - '[Reloaded]{}'\n",
        new_root.display()
    );
    fs::write(state.config.state_dir.join("slskd.yml"), &yaml)
        .expect("write watched share configuration");
    let mut cli_environment = state.controller_cli_environment.clone();
    cli_environment.remove("SLSKR_SHARE_FIXTURE");

    crate::apply_watched_controller_configuration(&state, Some(&yaml), &cli_environment).await;

    assert!(cancellation.load(std::sync::atomic::Ordering::Acquire));
    let generation_after_directory_reload = state
        .share_settings_generation
        .load(std::sync::atomic::Ordering::Acquire);

    let hidden_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some(Arc::clone(&hidden_cancellation));
    let mut hidden_environment = cli_environment.clone();
    hidden_environment.insert("SLSKR_SHARE_INCLUDE_HIDDEN".to_owned(), "true".to_owned());
    crate::apply_watched_controller_configuration(&state, Some(&yaml), &hidden_environment).await;
    assert!(hidden_cancellation.load(std::sync::atomic::Ordering::Acquire));
    assert!(state.share_settings.read().await.include_hidden);
    assert!(
        state
            .share_settings_generation
            .load(std::sync::atomic::Ordering::Acquire)
            > generation_after_directory_reload
    );
    let generation_after_hidden_reload = state
        .share_settings_generation
        .load(std::sync::atomic::Ordering::Acquire);

    let regex_cancellation = Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&regex_cancellation));
    let mut regex_environment = hidden_environment.clone();
    regex_environment.insert("SLSKD_CASE_SENSITIVE_REGEX".to_owned(), "true".to_owned());
    crate::apply_watched_controller_configuration(&state, Some(&yaml), &regex_environment).await;
    assert!(regex_cancellation.load(std::sync::atomic::Ordering::Acquire));
    assert!(*state
        .controller_case_sensitive_regex
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner));
    assert!(
        state
            .share_settings_generation
            .load(std::sync::atomic::Ordering::Acquire)
            > generation_after_hidden_reload
    );
    assert!(
        state
            .share_settings_generation
            .load(std::sync::atomic::Ordering::Acquire)
            > stale_generation
    );
    assert!(state.share_lifecycle.read().await.scan_pending);

    assert_eq!(
        crate::commit_share_index_snapshot_checked(&state, &stale_snapshot, stale_generation)
            .await
            .expect_err("old scan generation must be rejected"),
        crate::SHARE_SCAN_CANCELLED_ERROR
    );
    assert!(db
        .list_share_files(100, 0)
        .await
        .expect("read persisted shares")
        .is_empty());
    let shares = state.shares.read().await;
    assert_eq!(shares.roots[0].label, "Reloaded");
    drop(shares);
    let lifecycle = state.share_lifecycle.read().await;
    assert!(!lifecycle.scanning);
    assert!(lifecycle.cancelled);
    assert!(lifecycle.scan_pending);
    drop(lifecycle);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[test]
fn profile_static_roots_serve_the_selected_spa_on_dashboard() {
    let root = std::env::temp_dir().join(format!(
        "slskr-profile-static-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(root.join("legacy")).expect("create legacy profile root");
    fs::create_dir_all(root.join("native")).expect("create native profile root");
    fs::write(root.join("legacy/index.html"), "legacy-ui").expect("write legacy index");
    fs::write(root.join("native/index.html"), "native-ui").expect("write native index");

    let (legacy_root, legacy_index, _) = crate::web_static_file_for_request(
        "/dashboard",
        Some(&root),
        Some(crate::ControllerProfile::Legacy),
    )
    .expect("legacy dashboard SPA root");
    let (native_root, native_index, _) = crate::web_static_file_for_request(
        "/dashboard",
        Some(&root),
        Some(crate::ControllerProfile::Native),
    )
    .expect("native dashboard SPA root");

    assert!(legacy_root.ends_with("legacy"));
    assert!(legacy_index.ends_with("legacy/index.html"));
    assert!(native_root.ends_with("native"));
    assert!(native_index.ends_with("native/index.html"));
    assert!(crate::web_static_file_for_request(
        "/health",
        Some(&root),
        Some(crate::ControllerProfile::Legacy),
    )
    .is_none());
    assert!(crate::web_static_file_for_request(
        "/health/mesh",
        Some(&root),
        Some(crate::ControllerProfile::Native),
    )
    .is_none());
    assert!(crate::web_static_file_for_request(
        "/health?probe=1",
        Some(&root),
        Some(crate::ControllerProfile::Legacy),
    )
    .is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn relay_share_database_storage_uses_restart_stable_suffix() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let token = uuid::Uuid::new_v4();
    let stored = crate::persist_relay_share_database(&state, token, "shares.sqlite", b"fixture")
        .expect("persist relay share database");
    let expected = format!("share-{}.db", token.simple());
    assert_eq!(
        stored.file_name().and_then(|name| name.to_str()),
        Some(expected.as_str())
    );
    assert!(stored.is_file());
    assert!(!state
        .config
        .state_dir
        .join("relay/incoming")
        .join(format!("share-{}.sqlite", token.simple()))
        .exists());
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[test]
fn cancelled_share_scan_stops_before_indexing() {
    let root = std::env::temp_dir().join(format!(
        "slskr-cancelled-share-scan-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("create cancelled scan root");
    fs::write(root.join("not-indexed.flac"), b"fixture").expect("write cancelled scan file");
    let directories = crate::config::parse_share_directories(root.to_str().unwrap())
        .expect("parse cancelled scan directory");
    let cancellation = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let scan = crate::scan_share_dirs_with_cancellation(crate::ShareScanRequest {
        directories: &directories,
        follow_symlinks: false,
        include_hidden: false,
        max_files: 100,
        probe_media_attributes: false,
        workers: 1,
        filters: &[],
        cancellation,
    });

    assert!(scan.cancelled);
    assert!(scan.entries.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn graceful_shutdown_cancels_registered_share_rebuild_before_publish() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create share-index persistence database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let before = state.shares.read().await.clone();

    // Keep the production rebuild between cancellation registration and worker
    // startup, so shutdown necessarily overlaps the active rebuild operation.
    let lifecycle_guard = state.share_lifecycle.write().await;
    let scan_state = Arc::clone(&state);
    let mut rebuild = tokio::spawn(async move { crate::rebuild_share_index(&scan_state).await });
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let registered = state
                .share_scan_cancellation
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_some();
            if registered {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("rebuild registers its shutdown cancellation token");

    crate::initiate_graceful_shutdown(&state).await;
    assert!(state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .is_some_and(|cancellation| cancellation.load(std::sync::atomic::Ordering::Acquire)));
    drop(lifecycle_guard);

    let rebuild_result = tokio::time::timeout(Duration::from_secs(2), &mut rebuild)
        .await
        .expect("cancelled rebuild returns")
        .expect("join rebuild task");
    assert_eq!(
        rebuild_result.expect_err("shutdown must cancel the rebuild"),
        crate::SHARE_SCAN_CANCELLED_ERROR
    );
    assert_eq!(state.shares.read().await.entries, before.entries);
    let lifecycle = state.share_lifecycle.read().await;
    assert!(!lifecycle.scanning);
    assert!(lifecycle.cancelled);
    drop(lifecycle);
    assert!(db.list_share_files(100, 0).await.unwrap().is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn distributed_persistence_worker_commits_the_latest_snapshot() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory distributed database");
    let initial = crate::DistributedRuntime::new(Some("local-user")).persistence_snapshot();
    let (snapshot_sender, snapshot_receiver) = tokio::sync::watch::channel(initial);
    let (status_sender, mut status_receiver) =
        tokio::sync::watch::channel(crate::DistributedPersistenceStatus {
            revision: 0,
            result: Ok(()),
        });
    let worker = tokio::spawn(crate::run_distributed_persistence_worker(
        Some(db.clone()),
        snapshot_receiver,
        status_sender,
    ));

    snapshot_sender.send_replace(crate::DistributedPersistenceSnapshot {
        revision: 1,
        branch_level: 2,
        branch_root: "intermediate-root".to_owned(),
        parent_username: Some("intermediate-parent".to_owned()),
        children: vec![("intermediate-child".to_owned(), 1)],
    });
    snapshot_sender.send_replace(crate::DistributedPersistenceSnapshot {
        revision: 2,
        branch_level: 5,
        branch_root: "latest-root".to_owned(),
        parent_username: Some("latest-parent".to_owned()),
        children: vec![("latest-child".to_owned(), 4)],
    });

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = status_receiver.borrow_and_update().clone();
            if status.revision >= 2 {
                assert!(status.result.is_ok(), "latest snapshot should be durable");
                break;
            }
            status_receiver
                .changed()
                .await
                .expect("persistence worker remains available");
        }
    })
    .await
    .expect("latest distributed snapshot was persisted");

    let (tree_state, children) = db
        .load_distributed_state()
        .await
        .expect("load distributed snapshot");
    assert_eq!(
        tree_state,
        Some((
            5,
            "latest-root".to_owned(),
            Some("latest-parent".to_owned())
        ))
    );
    assert_eq!(children, vec![("latest-child".to_owned(), 4)]);

    drop(snapshot_sender);
    worker.await.expect("persistence worker exits");
    db.close_for_test().await;
}

#[tokio::test]
async fn distributed_runtime_rehydrates_one_persisted_snapshot_after_restart() {
    let root = std::env::temp_dir().join(format!(
        "slskr-distributed-runtime-restart-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create distributed restart directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create distributed restart database");
    let children = vec![("child-a".to_owned(), 2), ("child-b".to_owned(), 4)];
    db.save_distributed_state(6, "branch-root", Some("parent-user"), &children)
        .await
        .expect("persist distributed runtime state");
    db.close_for_test().await;

    let reopened = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("reopen distributed restart database");
    let runtime = RwLock::new(crate::DistributedRuntime::new(Some("local-user")));
    crate::hydrate_distributed_runtime(&runtime, &reopened)
        .await
        .expect("hydrate distributed runtime");
    let runtime = runtime.read().await;
    assert_eq!(runtime.branch_level, 6);
    assert_eq!(runtime.branch_root, "branch-root");
    assert_eq!(runtime.parent.as_deref(), Some("parent-user"));
    assert_eq!(
        runtime.child_depths,
        children.into_iter().collect::<BTreeMap<_, _>>()
    );
    drop(runtime);
    reopened.close_for_test().await;
    fs::remove_dir_all(root).expect("remove distributed restart directory");
}

#[tokio::test]
async fn managed_shutdown_persists_final_distributed_snapshot_after_worker_stops() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-distributed-shutdown-flush-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create distributed shutdown directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create distributed shutdown database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let (worker_started_tx, worker_started_rx) = tokio::sync::oneshot::channel();
    let (worker_stopped_tx, worker_stopped_rx) = tokio::sync::oneshot::channel::<()>();
    state.spawn_managed_task(async move {
        let mut worker_stopped_tx = Some(worker_stopped_tx);
        worker_started_tx
            .send(())
            .expect("observe managed shutdown worker startup");
        std::future::pending::<()>().await;
        drop(worker_stopped_tx.take());
    });
    worker_started_rx
        .await
        .expect("managed shutdown worker starts before shutdown");

    // Hold the runtime lock while shutdown aborts and joins managed workers.
    // The final snapshot read must wait for this lock and include the latest
    // revision published while shutdown is in progress.
    let mut runtime = state.distributed_network.write().await;
    let shutdown_state = Arc::clone(&state);
    let shutdown = tokio::spawn(async move { shutdown_state.shutdown_managed_tasks().await });
    assert!(
        tokio::time::timeout(Duration::from_secs(1), worker_stopped_rx)
            .await
            .expect("shutdown drops the managed worker")
            .is_err()
    );
    runtime.branch_level = 9;
    runtime.branch_root = "shutdown-root".to_owned();
    runtime.parent = Some("shutdown-parent".to_owned());
    runtime.child_depths = [("shutdown-child".to_owned(), 7)].into_iter().collect();
    runtime.persistence_revision = runtime.persistence_revision.saturating_add(1);
    let snapshot = runtime.persistence_snapshot();
    state
        .distributed_persistence_snapshots
        .send_replace(snapshot);
    drop(runtime);

    shutdown
        .await
        .expect("managed shutdown and final persistence join");
    let (tree_state, children) = db
        .load_distributed_state()
        .await
        .expect("load final distributed shutdown snapshot");
    assert_eq!(
        tree_state,
        Some((
            9,
            "shutdown-root".to_owned(),
            Some("shutdown-parent".to_owned())
        ))
    );
    assert_eq!(children, vec![("shutdown-child".to_owned(), 7)]);

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove distributed shutdown directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn managed_shutdown_closes_distributed_child_and_persists_latest_depth() {
    let root = std::env::temp_dir().join(format!(
        "slskr-distributed-child-shutdown-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("create child shutdown state directory");
    let db_path = root.join("slskr.db");
    let db = crate::persistence::DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .expect("open child shutdown database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (client, accepted) = tokio::join!(
        tokio::net::TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    let (server, _) = accepted.unwrap();
    crate::register_distributed_child(Arc::clone(&state), "child".to_owned(), server, false)
        .await
        .expect("register distributed child");
    let mut peer = slskr_client::stream::DistributedConnection::new(client.unwrap());
    peer.receive().await.expect("receive branch level");
    peer.receive().await.expect("receive branch root");
    peer.send(&crate::DistributedMessage::ChildDepth { depth: 3 })
        .await
        .expect("send latest child depth");
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state
                .distributed_network
                .read()
                .await
                .child_depths
                .get("child")
                == Some(&3)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("observe child depth before shutdown");

    state.shutdown_managed_tasks().await;
    assert!(tokio::time::timeout(Duration::from_secs(1), peer.receive())
        .await
        .expect("managed shutdown closes the child socket")
        .is_err());
    let (_, children) = db
        .load_distributed_state()
        .await
        .expect("read final child snapshot");
    assert_eq!(children, vec![("child".to_owned(), 3)]);
    db.close_for_test().await;
    fs::remove_dir_all(root).unwrap();
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn peer_listener_shutdown_joins_stalled_handshakes_and_handlers() {
    use slskr_client::protocol::init::InitMessage;
    use tokio::io::AsyncReadExt;

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_NO_CONNECT", "true")
            .with("SLSKR_LISTENER_BIND", "127.0.0.1:0")
            .with("SLSKD_SLSK_OBFUSCATION_LISTEN_PORT", "0"),
    );
    state
        .advanced_networking
        .write()
        .await
        .security
        .network_guard
        .enabled = true;
    let capacity = state.incoming_connections.available_permits();
    let (regular_tx, regular_rx) = mpsc::channel(1);
    let (obfuscated_tx, obfuscated_rx) = mpsc::channel(1);
    crate::spawn_configured_listeners(Arc::clone(&state), regular_rx, obfuscated_rx, true);
    let address = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(address) = state.listeners.read().await.regular_local_addr.clone() {
                break address;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("listener binds");
    let mut stalled = tokio::net::TcpStream::connect(&address).await.unwrap();
    let mut peer = tokio::net::TcpStream::connect(&address).await.unwrap();
    slskr_client::io::write_init_frame(
        &mut peer,
        &InitMessage::PeerInit {
            username: "shutdown-peer".to_owned(),
            connection_type: "P".to_owned(),
            token: 0,
        }
        .encode()
        .unwrap(),
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if state.incoming_connections.available_permits() == capacity - 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("peer handler registers");
    assert_eq!(
        state
            .incoming_connection_ips
            .lock()
            .unwrap()
            .values()
            .sum::<usize>(),
        1
    );
    state.shutdown_managed_tasks().await;
    let mut byte = [0u8; 1];
    for stream in [&mut stalled, &mut peer] {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), stream.read(&mut byte))
                .await
                .expect("shutdown closes socket")
                .unwrap(),
            0
        );
    }
    assert_eq!(state.incoming_connections.available_permits(), capacity);
    assert!(state.incoming_connection_ips.lock().unwrap().is_empty());
    drop((regular_tx, obfuscated_tx));
    let state_dir = state.config.state_dir.clone();
    drop(state);
    fs::remove_dir_all(state_dir).expect("remove isolated listener fixture");
}

#[tokio::test]
async fn incoming_search_shutdown_reclaims_queued_and_rejected_work() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let capacity = state.incoming_searches.available_permits();
    let held_capacity = Arc::clone(&state.incoming_searches)
        .acquire_many_owned(capacity as u32)
        .await
        .unwrap();
    crate::session_runtime::schedule_incoming_search_response(
        Arc::clone(&state),
        "peer".to_owned(),
        1,
        "Test".to_owned(),
    )
    .await;
    assert_eq!(
        state
            .incoming_search_queue_depth
            .load(crate::Ordering::Acquire),
        1
    );
    state.shutdown_managed_tasks().await;
    assert_eq!(
        state
            .incoming_search_queue_depth
            .load(crate::Ordering::Acquire),
        0
    );
    crate::session_runtime::schedule_incoming_search_response(
        Arc::clone(&state),
        "peer".to_owned(),
        2,
        "Test".to_owned(),
    )
    .await;
    assert_eq!(
        state
            .incoming_search_queue_depth
            .load(crate::Ordering::Acquire),
        0
    );
    drop(held_capacity);
    assert_eq!(state.incoming_searches.available_permits(), capacity);
    let state_dir = state.config.state_dir.clone();
    drop(state);
    fs::remove_dir_all(state_dir).expect("remove isolated search fixture");
}

async fn bridge_client_socket_pair() -> (tokio::net::TcpStream, tokio::net::TcpStream) {
    use tokio::net::{TcpListener, TcpStream};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (client, accepted) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(TcpStream::connect(address), listener.accept())
    })
    .await
    .expect("bounded bridge fixture connection");
    (client.unwrap(), accepted.unwrap().0)
}

async fn assert_bridge_client_closed(client: &mut tokio::net::TcpStream) {
    use tokio::io::AsyncReadExt;
    let mut byte = [0];
    let result = tokio::time::timeout(Duration::from_secs(5), client.read(&mut byte))
        .await
        .expect("bridge client socket closes before deadline");
    match result {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::NotConnected
            ) => {}
        other => panic!("bridge client remained readable after shutdown: {other:?}"),
    }
}

#[tokio::test]
async fn bridge_listener_stop_cancels_and_joins_stalled_client_handlers() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let (mut client, accepted) = bridge_client_socket_pair().await;
    state.runtime.write().await.bridge_active_clients.insert(
        "stalled".to_owned(),
        serde_json::json!({"clientId": "stalled"}),
    );
    let mut clients = crate::soulfind_bridge_runtime::BridgeClientTasks::new();
    assert!(clients.spawn(&state, "stalled".to_owned(), accepted));
    tokio::task::yield_now().await;
    clients.shutdown(&state).await;
    assert!(state.runtime.read().await.bridge_active_clients.is_empty());
    assert_bridge_client_closed(&mut client).await;
    state.shutdown_managed_tasks().await;
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn daemon_shutdown_joins_bridge_clients_and_clears_runtime_records() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let (mut client, accepted) = bridge_client_socket_pair().await;
    {
        let mut runtime = state.runtime.write().await;
        runtime.bridge_running = true;
        runtime.bridge_active_clients.insert(
            "stalled".to_owned(),
            serde_json::json!({"clientId": "stalled"}),
        );
    }
    let mut clients = crate::soulfind_bridge_runtime::BridgeClientTasks::new();
    assert!(clients.spawn(&state, "stalled".to_owned(), accepted));
    tokio::task::yield_now().await;
    state.shutdown_managed_tasks().await;
    {
        let runtime = state.runtime.read().await;
        assert!(!runtime.bridge_running);
        assert!(runtime.bridge_active_clients.is_empty());
    }
    assert_bridge_client_closed(&mut client).await;
    clients.shutdown(&state).await;
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn bridge_client_admission_after_daemon_shutdown_closes_the_socket() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    state.shutdown_managed_tasks().await;
    let (mut client, accepted) = bridge_client_socket_pair().await;
    let mut clients = crate::soulfind_bridge_runtime::BridgeClientTasks::new();
    assert!(!clients.spawn(&state, "late".to_owned(), accepted));
    assert_bridge_client_closed(&mut client).await;
    assert!(state.runtime.read().await.bridge_active_clients.is_empty());
    clients.shutdown(&state).await;
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn dropped_bridge_listener_owner_cancels_stalled_clients() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let (mut client, accepted) = bridge_client_socket_pair().await;
    state.runtime.write().await.bridge_active_clients.insert(
        "stalled".to_owned(),
        serde_json::json!({"clientId": "stalled"}),
    );
    let mut clients = crate::soulfind_bridge_runtime::BridgeClientTasks::new();
    assert!(clients.spawn(&state, "stalled".to_owned(), accepted));
    tokio::task::yield_now().await;
    drop(clients);
    assert_bridge_client_closed(&mut client).await;
    assert!(state.runtime.read().await.bridge_active_clients.is_empty());
    state.shutdown_managed_tasks().await;
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn managed_shutdown_closes_shared_gateway_stalled_tls_handshake() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &state.config.state_dir,
            None,
        )
        .await
        .unwrap(),
    );
    let (mut client, accepted) = bridge_client_socket_pair().await;
    gateway
        .handle_accepted_tcp(accepted, Arc::clone(&state))
        .await;
    tokio::task::yield_now().await;
    state.shutdown_managed_tasks().await;
    assert_bridge_client_closed(&mut client).await;
    drop(gateway);
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn shared_gateway_rejects_stalled_tls_handshake_after_shutdown() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &state.config.state_dir,
            None,
        )
        .await
        .unwrap(),
    );
    state.shutdown_managed_tasks().await;
    let (mut client, accepted) = bridge_client_socket_pair().await;
    gateway
        .handle_accepted_tcp(accepted, Arc::clone(&state))
        .await;
    assert_bridge_client_closed(&mut client).await;
    drop(gateway);
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn daemon_shutdown_closes_forwarding_listener_and_rejects_late_rules() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let reserved = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = reserved.local_addr().unwrap().port();
    drop(reserved);
    let request = crate::port_forwarding::StartRequest {
        local_port: port,
        pod_id: "pod:test".into(),
        destination_host: "service".into(),
        destination_port: 80,
        service_name: None,
        gateway_username: "gateway".into(),
        gateway_endpoints: vec!["127.0.0.1:9".parse().unwrap()],
        gateway_certificate_sha256: [7; 32],
        local_username: "local".into(),
        authentication_key: Arc::new(ed25519_dalek::SigningKey::from_bytes(&[9; 32])),
    };
    state.port_forwarding.start(request.clone()).await.unwrap();
    state.shutdown_managed_tasks().await;
    assert!(state.port_forwarding.statuses().await.is_empty());
    assert!(state
        .port_forwarding
        .start(request)
        .await
        .unwrap_err()
        .contains("shut down"));
    let rebound = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    drop(rebound);
}
