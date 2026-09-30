use super::fixtures::*;

#[test]
fn folder_contents_response_parser_accepts_controller_wire_shape() {
    let entries =
        crate::config::parse_share_entries("open-commons-fixtures/commons-click-track.ogg=168370")
            .expect("fixture share entry");
    let payload = crate::build_folder_contents_payload(
        &entries,
        7,
        "open-commons-fixtures",
        Default::default(),
    )
    .expect("folder response payload");
    let parsed = crate::folder_entries_from_peer_message(
        crate::PeerMessage::FolderContentsResponse(payload),
        "open-commons-fixtures",
        Default::default(),
    )
    .expect("parse folder response");
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].filename,
        "open-commons-fixtures/commons-click-track.ogg"
    );
    assert_eq!(parsed[0].size, 168370);
}

#[test]
fn folder_request_replaces_previous_browse_entries() {
    let mut browse = crate::BrowseStore::new();
    browse.request("friend".to_owned()).expect("browse record");
    browse.add_entries(
        "friend".to_owned(),
        vec![crate::BrowseEntry {
            path_encoding: Default::default(),
            filename: "open-commons-fixtures/commons-click-track.ogg".to_owned(),
            size: 168370,
            extension: "ogg".to_owned(),
        }],
        true,
    );
    let requested = browse
        .request_folder("friend".to_owned(), "open-commons-fixtures".to_owned())
        .expect("folder record");
    assert!(requested.entries.is_empty());
}

#[tokio::test]
async fn batch_operations_reuse_router_statuses_timeouts_and_nested_guard() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let missing = crate::route_http_request("GET", "/api/not-a-route", None, "", &state)
        .await
        .expect("direct missing-route response");
    assert_eq!(missing.status, "404 Not Found");
    let body = serde_json::json!({
        "operations": [
            {"id": "health", "method": "GET", "path": "/api/health"},
            {"id": "version", "method": "GET", "path": "/api/version"},
            {"id": "nested", "method": "POST", "path": "/api/v0/batch"},
            {"id": "missing", "method": "GET", "path": "/api/not-a-route"}
        ]
    })
    .to_string();
    let response = crate::route_http_request("POST", "/api/batch", None, &body, &state)
        .await
        .expect("batch response");
    assert_eq!(response.status, "202 Accepted");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).expect("batch JSON");
    assert_eq!(json["accepted"], true);
    assert_eq!(json["results"].as_array().map(Vec::len), Some(4));
    assert_eq!(json["executed"], 2);
    assert_eq!(json["failed"], 2);
    assert!(json["total_time_ms"].is_u64());
    assert_eq!(json["results"][0]["status"], 200);
    assert_eq!(json["results"][2]["status"], 400);
    assert_eq!(json["results"][3]["status"], 404);

    let stop_on_error = serde_json::json!({
        "operations": [
            {"id": "ok", "method": "GET", "path": "/api/health"},
            {"id": "missing", "method": "GET", "path": "/api/not-a-route"},
            {"id": "skipped", "method": "GET", "path": "/api/version"}
        ],
        "config": {"continueOnError": false}
    })
    .to_string();
    let response = crate::route_http_request("POST", "/api/batch", None, &stop_on_error, &state)
        .await
        .expect("batch stop-on-error response");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).expect("batch JSON");
    assert_eq!(json["results"].as_array().map(Vec::len), Some(2));
    assert_eq!(json["executed"], 1);
    assert_eq!(json["failed"], 1);
}

#[tokio::test]
async fn unversioned_empty_search_put_persists_cancellation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let create = serde_json::json!({
        "id": "search-stop-fixture",
        "query": "stop-me"
    })
    .to_string();
    let created = crate::route_http_request("POST", "/api/searches", None, &create, &state)
        .await
        .expect("create unversioned search");
    assert_eq!(created.status, "200 OK");

    let stopped =
        crate::route_http_request("PUT", "/api/searches/search-stop-fixture", None, "", &state)
            .await
            .expect("stop unversioned search");
    assert_eq!(stopped.status, "200 OK");

    let record =
        crate::route_http_request("GET", "/api/searches/search-stop-fixture", None, "", &state)
            .await
            .expect("read stopped search");
    let json = serde_json::from_str::<serde_json::Value>(&record.body).expect("search JSON");
    assert_eq!(json["status"], "cancelled");
    assert_eq!(json["state"], "Cancelled");
    assert!(json["endedAt"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[tokio::test]
async fn controller_debug_view_projects_frozen_default_authentication_values() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let overlay = state.options_overlay.read().await;
    let debug = crate::controller_options_debug_view(&state, &overlay);

    assert!(state.config.controller_metrics_password.is_empty());
    assert!(state
        .controller_web_auth_password
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_empty());
    assert_eq!(
        debug
            .matches("password=slskd (DefaultValueConfigurationProvider)")
            .count(),
        2
    );
}

#[tokio::test]
async fn versioned_search_accepts_web_acquisition_profiles() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    for profile in [
        "lossless-exact",
        "fast-good-enough",
        "album-complete",
        "rare-hunt",
        "conservative-network",
        "mesh-preferred",
        "metadata-strict",
    ] {
        let body = serde_json::json!({
            "searchText": "profile-validation",
            "acquisitionProfile": profile,
        })
        .to_string();
        let response = crate::route_http_request("POST", "/api/v0/searches", None, &body, &state)
            .await
            .expect("versioned search response");
        assert_eq!(
            response.status, "409 Conflict",
            "{profile}: {}",
            response.body
        );
    }

    let rejected = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"searchText":"profile-validation","acquisitionProfile":"made-up-profile"}"#,
        &state,
    )
    .await
    .expect("rejected versioned search response");
    assert_eq!(rejected.status, "400 Bad Request");
    assert!(rejected.body.contains("known acquisition profile"));
}

#[tokio::test]
async fn shadow_merge_rolls_back_records_when_realm_index_persistence_fails() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));
    *state.content_discovery.write().await =
        crate::content_discovery::ContentDiscoveryStore::load(&state.config.state_dir)
            .expect("load file-backed content-discovery store");
    *state.realm_subject_indexes.write().await =
        crate::realm_subject_index::Store::load_with_identity(
            &state.config.state_dir,
            crate::realm_subject_index::DEFAULT_REALM_ID,
            [crate::realm_subject_index::DEFAULT_GOVERNANCE_ROOT],
        )
        .expect("load file-backed realm-index store");
    fs::create_dir(state.config.state_dir.join("realm-subject-indexes.json"))
        .expect("make realm-index persistence fail");

    let mut index = serde_json::json!({
        "id": "index-persistence-failure",
        "realmId": crate::realm_subject_index::DEFAULT_REALM_ID,
        "subjectNamespace": "music",
        "revision": 1,
        "publishedAt": "2026-08-01T00:00:00Z",
        "entries": [{
            "subjectId": "subject-persistence-failure",
            "workRef": {
                "domain": "music",
                "title": "Persistence Failure",
                "creator": "Artist",
            },
            "externalIds": {
                "musicbrainz:recording": "recording-persistence-failure",
                "discogs": "discogs-persistence-failure",
            },
            "aliases": ["persistence-failure-alias"],
        }],
        "signature": {
            "signer": crate::realm_subject_index::DEFAULT_GOVERNANCE_ROOT,
            "algorithm": "realm-governance-sha256",
            "payloadHash": "",
            "value": "signature",
        },
    });
    index["signature"]["payloadHash"] =
        serde_json::json!(crate::realm_subject_index::compute_payload_hash(&index));

    let response = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"recording-persistence-failure","peerIds":["peer-a"]}],
            "realmIndexes": [index],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("failed shadow/realm-index merge response");

    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("realm subject-index storage is unavailable"));
    assert!(state
        .content_discovery
        .read()
        .await
        .shadow_records()
        .is_empty());
    assert!(state
        .realm_subject_indexes
        .read()
        .await
        .indexes_for_realm(crate::realm_subject_index::DEFAULT_REALM_ID)
        .is_empty());
    let persisted = crate::content_discovery::ContentDiscoveryStore::load(&state.config.state_dir)
        .expect("reload rolled-back shadow records");
    assert!(persisted.shadow_records().is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn database_cleanup_deletes_terminal_transfers_and_persists_tombstones() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create database cleanup persistence database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let transfer = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("cleanup-peer".to_owned()),
            "Remote/Complete.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_status(entry.id, "completed", Some(10), None)
            .expect("mark cleanup transfer completed")
    };
    crate::persist_transfer_record(&state, &transfer)
        .await
        .expect("persist terminal transfer before cleanup");

    let cleanup = crate::database_cleanup_value(&state, "{\"days\":0}").await;
    assert_eq!(cleanup["status"], "ok");
    assert_eq!(cleanup["pruned_transfers"], 1);
    assert_eq!(cleanup["transferCleanup"]["healthy"], true);
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .all(|entry| entry.id != transfer.id));
    assert!(db
        .list_transfers(None, 10, 0)
        .await
        .expect("read transfers after cleanup")
        .is_empty());
    assert_eq!(
        db.max_transfer_id()
            .await
            .expect("read transfer id retained by tombstone"),
        transfer.id
    );

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn database_cleanup_restores_terminal_transfers_when_sqlite_delete_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create database cleanup rollback database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let old_message = seed_old_message_for_database_cleanup(&state, &db).await;
    let transfer = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("cleanup-peer".to_owned()),
            "Remote/Failed.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_status(entry.id, "failed", Some(1), Some("offline".to_owned()))
            .expect("mark cleanup transfer failed")
    };
    crate::persist_transfer_record(&state, &transfer)
        .await
        .expect("persist terminal transfer before failed cleanup");
    db.close_for_test().await;

    let cleanup = crate::database_cleanup_value(&state, "{\"days\":0}").await;
    assert_eq!(cleanup["status"], "error");
    assert_eq!(cleanup["transferCleanup"]["healthy"], false);
    assert_eq!(
        cleanup["transferCleanup"]["error"],
        "transfer cleanup unavailable"
    );
    assert!(state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|entry| entry.id == transfer.id));
    assert_eq!(cleanup["pruned_messages"], 0);
    assert!(state
        .messages
        .read()
        .await
        .records
        .iter()
        .any(|record| record.id == old_message.id));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn database_cleanup_orders_message_projection_with_sqlite_deletion() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create message cleanup database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let old_message = seed_old_message_for_database_cleanup(&state, &db).await;
    let persistence_turn = state.message_persistence_lock.lock().await;
    let cleanup_state = Arc::clone(&state);
    let mut cleanup_task =
        tokio::spawn(
            async move { crate::database_cleanup_value(&cleanup_state, "{\"days\":0}").await },
        );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut cleanup_task)
            .await
            .is_err()
    );

    let readable = crate::route_http_request("GET", "/api/messages", None, "", &state)
        .await
        .expect("message reads stay responsive while cleanup waits");
    assert_eq!(readable.status, "200 OK");
    let readable_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    assert_eq!(readable_json["count"], 1);
    assert_eq!(readable_json["entries"][0]["id"], old_message.id);
    assert_eq!(
        db.list_messages(10, 0)
            .await
            .expect("message rows remain while cleanup waits")
            .len(),
        1
    );

    drop(persistence_turn);
    let cleanup = cleanup_task.await.expect("message cleanup task completes");
    assert_eq!(cleanup["status"], "ok");
    assert_eq!(cleanup["cleaned"], 1);
    assert_eq!(cleanup["pruned_messages"], 1);
    assert_eq!(
        state.messages.read().await.records.len(),
        0,
        "message projection is pruned with SQLite"
    );
    assert!(db
        .list_messages(10, 0)
        .await
        .expect("read messages after cleanup")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn runtime_compatibility_mutation_waits_for_persistence_turn_without_blocking_reads() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let persistence_turn = state.runtime_persistence_lock.lock().await;
    let task_state = Arc::clone(&state);
    let mut mutation = tokio::spawn(async move {
        crate::mutate_runtime_compat_state(&task_state, |runtime, _| runtime.record_gc()).await
    });

    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut mutation)
            .await
            .is_err()
    );
    assert_eq!(state.runtime.read().await.gc_runs, 0);

    drop(persistence_turn);
    mutation
        .await
        .expect("runtime mutation task should join")
        .expect("runtime mutation should persist");
    assert_eq!(state.runtime.read().await.gc_runs, 1);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn runtime_compatibility_mutation_releases_store_guards_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-runtime-compat-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = crate::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create runtime compatibility database");
    db.execute_raw_for_test(
        "CREATE TRIGGER fail_runtime_compat_insert BEFORE INSERT ON runtime_compat_state \
         BEGIN SELECT RAISE(ABORT, 'forced runtime compatibility failure'); END",
    )
    .await
    .expect("install persistence failure trigger");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let blocker_pool = sqlx_sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx_sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await
        .expect("open SQLite lock connection");
    let mut blocker = blocker_pool
        .acquire()
        .await
        .expect("acquire SQLite lock connection");
    sqlx_core::query::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold SQLite write lock");

    let task_state = Arc::clone(&state);
    let mutation = tokio::spawn(async move {
        crate::mutate_runtime_compat_state(&task_state, |runtime, _| runtime.record_gc()).await
    });
    let read_saw_mutation = tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if let Ok(runtime) = state.runtime.try_read() {
                if runtime.gc_runs == 1 {
                    break true;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let mutation_waited_for_sqlite = !mutation.is_finished();
    let unrelated_update_survived = tokio::time::timeout(Duration::from_millis(200), async {
        state.runtime.write().await.application_reconnect_pending = true;
    })
    .await
    .is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let mutation_result = tokio::time::timeout(Duration::from_secs(2), mutation)
        .await
        .expect("runtime mutation should finish after releasing SQLite")
        .expect("runtime mutation task should join");
    assert!(
        mutation_result.is_err(),
        "trigger should fail the persistence write"
    );
    assert!(
        read_saw_mutation,
        "runtime reads should proceed during SQLite I/O"
    );
    assert!(
        mutation_waited_for_sqlite,
        "the write should wait on SQLite"
    );
    assert!(
        unrelated_update_survived,
        "unrelated runtime updates should proceed during SQLite I/O"
    );
    let runtime = state.runtime.read().await;
    assert_eq!(
        runtime.gc_runs, 0,
        "failed persistent mutation should roll back"
    );
    assert!(runtime.application_reconnect_pending);
    drop(runtime);

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[tokio::test]
async fn runtime_compatibility_transient_latches_stay_process_local() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create runtime compatibility database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    crate::mutate_runtime_compat_state_in_memory(&state, |runtime| {
        runtime.set_restart_requested(true);
        runtime.bridge_running = true;
    })
    .await;
    crate::mutate_runtime_compat_state(&state, |runtime, _| runtime.record_gc())
        .await
        .expect("persist durable runtime counter");

    let live = state.runtime.read().await;
    assert!(live.application_restart_requested);
    assert!(live.bridge_running);
    drop(live);

    let persisted = db
        .get_runtime_compat_state()
        .await
        .expect("read runtime compatibility state")
        .expect("durable runtime state was persisted");
    assert_eq!(persisted.gc_runs, 1);
    assert!(!persisted.application_restart_requested);
    assert!(!persisted.bridge_running);

    let rehydrated = crate::RuntimeCompatState::try_from_persisted(&persisted)
        .expect("rehydrate durable runtime state");
    assert!(!rehydrated.application_restart_requested);
    assert!(!rehydrated.bridge_running);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn lidarr_rejection_waits_for_search_turn_before_mutating_wishlist() {
    let (state, _session_commands) = test_state_with_env(
        MapEnv::default().with("SLSKR_LIDARR_BLACKLIST_REJECTED_DOWNLOADS", "true"),
    );
    let item = state
        .wishlist
        .write()
        .await
        .add_item("Artist".to_owned(), "Album".to_owned(), "Audio".to_owned())
        .expect("wishlist item");
    let transfer = {
        let mut transfers = state.transfers.write().await;
        transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Remote/Album/track.flac".to_owned(),
            None,
            Some(8),
            None,
            crate::TransferRequestDetails {
                wishlist_item_id: Some(item.id.clone()),
                ..Default::default()
            },
        )
    };
    let result = serde_json::json!({
        "rejectedCandidateCount": 1,
        "rejectedFilenames": []
    });

    let search_turn = state.search_persistence_lock.lock().await;
    let policy_state = Arc::clone(&state);
    let mut policy = tokio::spawn(async move {
        crate::transfer_completion::apply_lidarr_rejection_policy(
            &policy_state,
            &transfer,
            "",
            &result,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut policy)
        .await
        .is_err());
    assert!(state
        .wishlist
        .read()
        .await
        .list_ignored_results(&item.id)
        .expect("wishlist item remains available")
        .is_empty());

    drop(search_turn);
    policy
        .await
        .expect("Lidarr policy task completes")
        .expect("apply Lidarr rejection policy");
    assert_eq!(
        state
            .wishlist
            .read()
            .await
            .list_ignored_results(&item.id)
            .expect("wishlist item remains available")
            .len(),
        1
    );
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn browse_request_and_cancel_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create browse ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    state.session.write().await.state = "connected";

    let persistence_turn = state.browse_persistence_lock.lock().await;
    let request_state = Arc::clone(&state);
    let mut request = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/request",
            None,
            "",
            &request_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut request)
            .await
            .is_err()
    );

    let cancel_state = Arc::clone(&state);
    let mut cancel = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/cancel",
            None,
            r#"{"reason":"cancelled"}"#,
            &cancel_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut cancel)
        .await
        .is_err());

    let readable = crate::route_http_request("GET", "/api/v0/browse", None, "", &state)
        .await
        .expect("browse read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        request
            .await
            .expect("browse request completes")
            .expect("request browse")
            .status,
        "202 Accepted"
    );
    assert_eq!(
        cancel
            .await
            .expect("browse cancellation completes")
            .expect("cancel browse")
            .status,
        "200 OK"
    );

    assert_eq!(
        state.browse.read().await.get("friend").unwrap().status,
        "cancelled"
    );
    let persisted = db
        .list_browse_records(10, 0)
        .await
        .expect("read final browse rows");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].status, "cancelled");

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn user_watch_and_unwatch_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create user projection ordering database");
    let (state, mut session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let record = state
        .users
        .write()
        .await
        .watch("friend".to_owned())
        .expect("seed watched user");
    crate::user_store::persist_user_projection(&state, &record)
        .await
        .expect("persist watched user");

    let persistence_turn = state.user_persistence_lock.lock().await;
    let watch_state = Arc::clone(&state);
    let mut watch = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/users/watch",
            None,
            r#"{"username":"friend"}"#,
            &watch_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut watch)
        .await
        .is_err());

    let unwatch_state = Arc::clone(&state);
    let mut unwatch = tokio::spawn(async move {
        crate::route_http_request(
            "DELETE",
            "/api/v0/users/friend/watch",
            None,
            "",
            &unwatch_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut unwatch)
            .await
            .is_err()
    );

    let readable = crate::route_http_request("GET", "/api/v0/users", None, "", &state)
        .await
        .expect("user read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(readable.body.contains("\"watched\":true"));

    drop(persistence_turn);
    assert_eq!(
        watch
            .await
            .expect("user watch completes")
            .expect("watch user")
            .status,
        "201 Created"
    );
    assert_eq!(
        unwatch
            .await
            .expect("user unwatch completes")
            .expect("unwatch user")
            .status,
        "200 OK"
    );
    assert_eq!(
        session_commands.try_recv().expect("watch session command"),
        crate::SessionCommand::WatchUser("friend".to_owned())
    );
    assert_eq!(
        session_commands
            .try_recv()
            .expect("unwatch session command"),
        crate::SessionCommand::UnwatchUser("friend".to_owned())
    );
    assert!(!state.users.read().await.records[0].watched);
    let persisted = db
        .list_user_projections(10, 0)
        .await
        .expect("read final user projection");
    assert_eq!(persisted.len(), 1);
    assert!(!persisted[0].watched);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn event_api_and_background_writes_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create event ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.event_persistence_lock.lock().await;
    let api_state = Arc::clone(&state);
    let mut api_event = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""queued""#,
            &api_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut api_event)
            .await
            .is_err()
    );

    let background_state = Arc::clone(&state);
    let mut background_event = tokio::spawn(async move {
        crate::record_event(&background_state, "background.event", "resource", None).await;
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut background_event)
            .await
            .is_err()
    );

    let readable = crate::route_http_request("GET", "/api/v0/events/records", None, "", &state)
        .await
        .expect("event read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    let readable_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    assert_eq!(readable_json["count"], 0);
    assert!(readable_json["entries"].as_array().unwrap().is_empty());

    drop(persistence_turn);
    assert_eq!(
        api_event
            .await
            .expect("API event completes")
            .expect("inject event")
            .status,
        "201 Created"
    );
    background_event
        .await
        .expect("background event persistence completes");

    let memory = state.events.read().await;
    assert_eq!(memory.records.len(), 2);
    assert_eq!(memory.records[0].id, 1);
    assert_eq!(memory.records[0].kind, "Noop");
    assert_eq!(memory.records[1].id, 2);
    assert_eq!(memory.records[1].kind, "background.event");
    drop(memory);
    let persisted = db.list_events(10, 0).await.expect("read final event rows");
    assert_eq!(persisted.len(), 2);
    assert_eq!(persisted[0].id, 1);
    assert_eq!(persisted[0].kind, "Noop");
    assert_eq!(persisted[1].id, 2);
    assert_eq!(persisted[1].kind, "background.event");

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}
