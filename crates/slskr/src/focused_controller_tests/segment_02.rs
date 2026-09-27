#[tokio::test]
async fn database_cleanup_orders_message_projection_with_sqlite_deletion() {
    let db = super::persistence::DatabaseManager::in_memory()
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
            async move { super::database_cleanup_value(&cleanup_state, "{\"days\":0}").await },
        );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut cleanup_task)
            .await
            .is_err()
    );

    let readable = super::route_http_request("GET", "/api/messages", None, "", &state)
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
async fn browse_response_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_entries = (0..=super::MAX_BROWSE_WIRE_FILES_PER_RESPONSE)
        .map(|index| serde_json::json!({"filename": format!("file-{index}.flac")}))
        .collect::<Vec<_>>();
    let entries_response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-entries",
            "entries": oversized_entries
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized browse entries response");
    assert_eq!(entries_response.status, "400 Bad Request");
    assert!(entries_response
        .body
        .contains("browse response exceeds wire entry limits"));
    assert!(state.browse.read().await.get("oversized-entries").is_none());

    let oversized_directory_files = super::MAX_BROWSE_WIRE_FILES_PER_RESPONSE / 2 + 1;
    let nested_response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-nested-files",
            "directories": [
                {
                    "name": "one",
                    "files": (0..oversized_directory_files)
                        .map(|index| serde_json::json!({"filename": format!("one-{index}.flac")}))
                        .collect::<Vec<_>>()
                },
                {
                    "name": "two",
                    "files": (0..oversized_directory_files)
                        .map(|index| serde_json::json!({"filename": format!("two-{index}.flac")}))
                        .collect::<Vec<_>>()
                }
            ]
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized nested browse entries response");
    assert_eq!(nested_response.status, "400 Bad Request");
    assert!(state
        .browse
        .read()
        .await
        .get("oversized-nested-files")
        .is_none());

    let oversized_directories = (0..=super::MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE)
        .map(|index| serde_json::json!({"name": format!("folder-{index}")}))
        .collect::<Vec<_>>();
    let directories_response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-directories",
            "directories": oversized_directories
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized browse directory response");
    assert_eq!(directories_response.status, "400 Bad Request");
    assert!(state
        .browse
        .read()
        .await
        .get("oversized-directories")
        .is_none());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn search_response_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let search_response = super::route_http_request(
        "POST",
        "/api/searches",
        None,
        r#"{"query":"oversized-response"}"#,
        &state,
    )
    .await
    .expect("create search for oversized response test");
    assert_eq!(search_response.status, "200 OK", "{}", search_response.body);
    let token = state
        .searches
        .read()
        .await
        .records
        .first()
        .map(|record| record.token)
        .expect("created search token");

    let oversized_files = (0..=super::MAX_SEARCH_RESULTS_PER_SEARCH)
        .map(|index| serde_json::json!({"filename": format!("file-{index}.flac")}))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        &serde_json::json!({"token": token, "files": oversized_files}).to_string(),
        &state,
    )
    .await
    .expect("oversized search response");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("search response exceeds result limits"));

    let searches = state.searches.read().await;
    let record = searches
        .records
        .iter()
        .find(|record| record.token == token)
        .expect("search remains present");
    assert!(record.results.is_empty());
    assert_eq!(record.raw_response_count, 0);
    drop(searches);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn collection_reorder_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let (collection_id, item_id) = {
        let mut collections = state.collections.write().await;
        let collection = collections
            .create(
                "tester".to_owned(),
                "Reorder bounds".to_owned(),
                String::new(),
            )
            .expect("create collection");
        let item = collections
            .add_item(
                &collection.id,
                "content-1".to_owned(),
                "Artist".to_owned(),
                "Track".to_owned(),
                "Audio".to_owned(),
            )
            .expect("add collection item")
            .expect("collection item");
        (collection.id, item.id)
    };

    let oversized_ids = (0..=super::MAX_COLLECTION_ITEMS)
        .map(|index| serde_json::json!(format!("item-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items/reorder"),
        None,
        &serde_json::json!({"itemIds": oversized_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized collection reorder");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("collection reorder exceeds item limits"));

    let collections = state.collections.read().await;
    let record = collections
        .get(&collection_id)
        .expect("collection remains present");
    assert_eq!(record.items.len(), 1);
    assert_eq!(record.items[0].id, item_id);
    drop(collections);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn musicbrainz_rejects_oversized_json_batches_before_state_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_recording_ids = (0..=super::MAX_LIBRARY_ITEMS)
        .map(|index| serde_json::json!(format!("recording-{index}")))
        .collect::<Vec<_>>();
    let diff_response = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/diffs",
        None,
        &serde_json::json!({"recordingIds": oversized_recording_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized MusicBrainz diff");
    assert_eq!(diff_response.status, "400 Bad Request");
    assert!(diff_response
        .body
        .contains("recordingIds must contain at most"));

    let oversized_suggestions = (0..=super::MAX_WISHLIST_ITEMS)
        .map(|index| serde_json::json!({"artist": "Artist", "title": format!("Release {index}")}))
        .collect::<Vec<_>>();
    let wishlist_response = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/wishlist",
        None,
        &serde_json::json!({"suggestions": oversized_suggestions}).to_string(),
        &state,
    )
    .await
    .expect("oversized MusicBrainz wishlist");
    assert_eq!(wishlist_response.status, "400 Bad Request");
    assert!(wishlist_response
        .body
        .contains("suggestions must contain at most"));
    assert!(state.wishlist.read().await.records.is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn failed_library_runtime_transaction_rolls_back_unchanged_store_independently() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let previous_library = state.library.read().await.clone();
    let previous_runtime = state.runtime.read().await.clone();

    let (mutated_library, mutated_runtime, failed_library_item) = {
        let mut library = state.library.write().await;
        let mut runtime = state.runtime.write().await;
        let record = library
            .create(
                "Artist".to_owned(),
                "Failed import".to_owned(),
                "Audio".to_owned(),
            )
            .expect("failed import candidate should fit");
        runtime.record_lidarr_manual_import(1, false, "/music/failed".to_owned(), Vec::new());
        (library.clone(), runtime.clone(), record)
    };

    // Model a runtime-only write that succeeds after the cross-store SQL
    // transaction fails but before its error path reacquires the state locks.
    let concurrent_runtime = {
        let mut runtime = state.runtime.write().await;
        runtime.record_gc();
        runtime.clone()
    };
    super::route_dispatch::rollback_library_runtime_if_unchanged(
        &state,
        previous_library.clone(),
        mutated_library,
        previous_runtime.clone(),
        mutated_runtime,
        failed_library_item.clone(),
    )
    .await;
    assert_eq!(*state.library.read().await, previous_library);
    let mut expected_runtime = concurrent_runtime;
    expected_runtime.lidarr_manual_imports = previous_runtime.lidarr_manual_imports;
    assert_eq!(*state.runtime.read().await, expected_runtime);

    // A concurrent library-only write likewise must survive while the failed
    // transaction's otherwise unchanged runtime candidate is rolled back.
    let (mutated_library, mutated_runtime, failed_library_item) = {
        let mut library = state.library.write().await;
        let mut runtime = state.runtime.write().await;
        let record = library
            .create(
                "Artist".to_owned(),
                "Second failed import".to_owned(),
                "Audio".to_owned(),
            )
            .expect("second failed import candidate should fit");
        runtime.record_lidarr_manual_import(
            1,
            false,
            "/music/second-failed".to_owned(),
            Vec::new(),
        );
        (library.clone(), runtime.clone(), record)
    };
    let previous_runtime = state.runtime.read().await.clone();
    let concurrent_library = {
        let mut library = state.library.write().await;
        library
            .create(
                "Artist".to_owned(),
                "Concurrent library write".to_owned(),
                "Audio".to_owned(),
            )
            .expect("concurrent library candidate should fit");
        library.clone()
    };
    let mut expected_library = concurrent_library.clone();
    expected_library
        .records
        .retain(|record| record.id != failed_library_item.id);
    super::route_dispatch::rollback_library_runtime_if_unchanged(
        &state,
        previous_library.clone(),
        mutated_library,
        previous_runtime.clone(),
        mutated_runtime,
        failed_library_item,
    )
    .await;
    assert_eq!(*state.library.read().await, expected_library);
    assert_eq!(*state.runtime.read().await, previous_runtime);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn runtime_compatibility_mutation_waits_for_persistence_turn_without_blocking_reads() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let persistence_turn = state.runtime_persistence_lock.lock().await;
    let task_state = Arc::clone(&state);
    let mut mutation = tokio::spawn(async move {
        super::mutate_runtime_compat_state(&task_state, |runtime, _| runtime.record_gc()).await
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
    let db = super::persistence::DatabaseManager::new(
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
        super::mutate_runtime_compat_state(&task_state, |runtime, _| runtime.record_gc()).await
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
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create runtime compatibility database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    super::mutate_runtime_compat_state_in_memory(&state, |runtime| {
        runtime.set_restart_requested(true);
        runtime.bridge_running = true;
    })
    .await;
    super::mutate_runtime_compat_state(&state, |runtime, _| runtime.record_gc())
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

    let rehydrated = super::RuntimeCompatState::try_from_persisted(&persisted)
        .expect("rehydrate durable runtime state");
    assert!(!rehydrated.application_restart_requested);
    assert!(!rehydrated.bridge_running);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_rejects_oversized_file_batches_before_queue_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let files = (0..=super::MAX_TRANSFER_REQUEST_FILES)
        .map(|index| serde_json::json!({"filename": format!("Music/{index}.flac"), "size": 1}))
        .collect::<Vec<_>>();
    let body = serde_json::json!({"username": "peer", "files": files}).to_string();
    for path in [
        "/api/v0/transfers/downloads/batches",
        "/api/v0/transfers/downloads/peer",
        "/api/transfers",
    ] {
        let response = super::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized transfer request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains("file limits"),
            "{path}: {}",
            response.body
        );
    }
    assert!(state.transfers.read().await.entries.is_empty());
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn string_array_routes_reject_oversized_wire_batches_before_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let recipients = (0..=super::MAX_PRIVATE_MESSAGE_RECIPIENTS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/conversations/batch",
        None,
        &serde_json::json!({"usernames": recipients, "body": "hello"}).to_string(),
        &state,
    )
    .await
    .expect("oversized conversation batch");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("conversation batch exceeds recipient limits"));
    assert!(state.messages.read().await.records.is_empty());

    let item_ids = (0..=super::MAX_WISHLIST_ITEMS)
        .map(|index| serde_json::json!(format!("00000000-0000-0000-0000-{index:012x}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "PUT",
        "/api/v0/wishlist/bulk-filter",
        None,
        &serde_json::json!({"itemIds": item_ids, "filter": "flac"}).to_string(),
        &state,
    )
    .await
    .expect("oversized wishlist filter batch");
    assert_eq!(response.status, "400 Bad Request");
    assert!(
        response
            .body
            .contains("wishlist bulk filter exceeds item limits"),
        "{}",
        response.body
    );
    assert!(state.wishlist.read().await.records.is_empty());

    let capabilities = (0..=super::MAX_CAPABILITY_NEGOTIATION_ITEMS)
        .map(|index| serde_json::json!(format!("capability-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/capabilities/negotiate",
        None,
        &serde_json::json!({"capabilities": capabilities}).to_string(),
        &state,
    )
    .await
    .expect("oversized capability negotiation");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("capabilities negotiation exceeds item limits"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn mediacore_rejects_oversized_wire_batches_before_work() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let descriptors = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!({"contentId": format!("content:test:{index}")}))
        .collect::<Vec<_>>();
    let content_ids = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("content:test:{index}")))
        .collect::<Vec<_>>();
    let keys = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("cache-key-{index}")))
        .collect::<Vec<_>>();
    for (path, body, expected) in [
        (
            "/api/v0/mediacore/publish/batch",
            serde_json::json!({"descriptors": descriptors}).to_string(),
            "descriptors must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/publish/republish",
            serde_json::json!({"contentIds": content_ids.clone()}).to_string(),
            "contentIds must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/retrieve/batch",
            serde_json::json!({"contentIds": content_ids}).to_string(),
            "contentIds must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/retrieve/cache/clear",
            serde_json::json!({"keys": keys}).to_string(),
            "keys must contain at most 100 items",
        ),
    ] {
        let response = super::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized MediaCore request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn remaining_controller_array_routes_reject_oversized_wire_batches_before_work() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());

    let download_items = (0..=super::MAX_EXTENDED_DOWNLOAD_ITEMS)
        .map(|index| {
            serde_json::json!({
                "user": "peer",
                "remotePath": format!("Music/{index}.flac"),
            })
        })
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/downloads",
        None,
        &serde_json::json!({"items": download_items}).to_string(),
        &state,
    )
    .await
    .expect("oversized extended download request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("items must contain at most 1000 items"));
    assert!(state.transfers.read().await.entries.is_empty());

    let links = (0..=super::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| {
            serde_json::json!({
                "name": format!("link-{index}"),
                "target": format!("target-{index}"),
            })
        })
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/mediacore/ipld/links/content:test:oversized",
        None,
        &serde_json::json!({"links": links}).to_string(),
        &state,
    )
    .await
    .expect("oversized MediaCore links request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("links must contain at most 100 items"));

    let entries = (0..=super::MAX_MEDIACORE_PORTABILITY_ENTRIES)
        .map(|index| serde_json::json!({"contentId": format!("content:test:{index}")}))
        .collect::<Vec<_>>();
    for (path, body, expected) in [
        (
            "/api/v0/mediacore/portability/analyze",
            serde_json::json!({"package": {"entries": entries.clone()}}).to_string(),
            "entries must contain at most 1000 items",
        ),
        (
            "/api/v0/mediacore/portability/import",
            serde_json::json!({"package": {"entries": entries}}).to_string(),
            "entries must contain at most 1000 items",
        ),
    ] {
        let response = super::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized MediaCore portability request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let content_ids = (0..=super::MAX_MEDIACORE_PORTABILITY_ENTRIES)
        .map(|index| serde_json::json!(format!("content:test:export:{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/export",
        None,
        &serde_json::json!({"contentIds": content_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized MediaCore export request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("contentIds must contain at most 1000 items"));

    let sources = (0..=super::multisource::MAX_SOURCES)
        .map(|index| serde_json::json!({"username": format!("peer-{index}")}))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/multisource/test",
        None,
        &serde_json::json!({"sources": sources}).to_string(),
        &state,
    )
    .await
    .expect("oversized multisource test request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("source count exceeds the 16 source limit"));

    let jurors = (0..=super::MAX_QUARANTINE_JURY_ITEMS)
        .map(|index| serde_json::json!(format!("juror-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        &serde_json::json!({
            "localReason": "test",
            "jurors": jurors,
            "evidence": [{"reference": "safe-reference", "summary": "test"}],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized quarantine request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("quarantine jury arrays must contain at most 100 items"));

    let usernames = (0..=super::MAX_RANKING_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/ranking/history",
        None,
        &serde_json::to_string(&usernames).expect("ranking usernames JSON"),
        &state,
    )
    .await
    .expect("oversized ranking history request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response.body.contains("at most 1000 usernames"));

    let candidates = (0..=super::MAX_RANKING_BATCH_ITEMS)
        .map(|index| {
            serde_json::json!({
                "username": format!("peer-{index}"),
                "filename": "Music/test.flac",
            })
        })
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        &serde_json::to_string(&candidates).expect("ranking candidates JSON"),
        &state,
    )
    .await
    .expect("oversized ranking request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response.body.contains("at most 1000 source candidates"));

    let issue_ids = (0..=25)
        .map(|index| serde_json::json!(format!("issue-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/slskdn/library/remediate",
        None,
        &serde_json::json!({"issue_ids": issue_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized library remediation request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("issue_ids must contain 1 to 25 values"));

    let muted_release_group_ids = (0..=super::MAX_RADAR_MUTED_RELEASE_GROUPS)
        .map(|index| serde_json::json!(format!("release-group-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        &serde_json::json!({
            "artistId": "artist:test",
            "mutedReleaseGroupIds": muted_release_group_ids,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized release-radar subscription request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("mutedReleaseGroupIds must contain at most 256 items"));

    let target_peer_ids = (0..=super::MAX_ROUTING_TARGET_PEERS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        None,
        &serde_json::json!({
            "message": {"messageId": "message:test", "channelId": "channel:test"},
            "targetPeerIds": target_peer_ids,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized PodCore routing request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("targetPeerIds must contain at most 256 items"));

    let tags = (0..=super::MAX_POD_DISCOVERY_TAGS)
        .map(|index| serde_json::json!(format!("tag-{index}")))
        .collect::<Vec<_>>();
    let response = super::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/register",
        None,
        &serde_json::json!({
            "podId": "pod:test",
            "visibility": "listed",
            "name": "test",
            "tags": tags,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized PodCore discovery request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("tags must contain at most 100 items"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_csv_import_rejects_oversized_row_batches() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_csv = (0..=super::MAX_CSV_IMPORT_ROWS)
        .map(|index| format!("artist-{index},title-{index}"))
        .collect::<Vec<_>>()
        .join("\n");

    let unversioned_response = super::route_http_request(
        "POST",
        "/api/wishlist/import/csv",
        None,
        &oversized_csv,
        &state,
    )
    .await
    .expect("oversized unversioned CSV import");
    assert_eq!(unversioned_response.status, "400 Bad Request");
    assert!(unversioned_response
        .body
        .contains("CSV import exceeds 10000 rows"));

    let versioned_response = super::route_http_request(
        "POST",
        "/api/v0/wishlist/import/csv",
        None,
        &serde_json::json!({"csvText": oversized_csv}).to_string(),
        &state,
    )
    .await
    .expect("oversized versioned CSV import");
    assert_eq!(versioned_response.status, "400 Bad Request");
    assert!(versioned_response
        .body
        .contains("CSV import exceeds 10000 rows"));
    let preview_response = super::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &serde_json::json!({
            "sourceText": oversized_csv,
            "sourceKind": "csv",
            "fetchProviderUrls": false,
            "limit": 500,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized source preview");
    assert_eq!(preview_response.status, "400 Bad Request");
    assert!(preview_response
        .body
        .contains("CSV import exceeds 10000 rows"));
    assert!(state.wishlist.read().await.records.is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn library_browser_projects_share_tree_and_sha256_stream_ids() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let root = std::env::temp_dir().join(format!(
        "slskr-browser-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(root.join("Album")).expect("create browser fixture directory");
    let local_file = root.join("Album").join("Track.flac");
    fs::write(&local_file, b"browser stream fixture").expect("write browser fixture");
    {
        let mut shares = state.shares.write().await;
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Local/Album/Track.flac".to_owned(),
            size: fs::metadata(&local_file)
                .expect("browser fixture metadata")
                .len(),
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
        shares
            .local_paths
            .insert("Local/Album/Track.flac".to_owned(), local_file.clone());
    }

    let root_response = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?kinds=Audio",
        None,
        "",
        &state,
    )
    .await
    .expect("library browser root response");
    assert_eq!(root_response.status, "200 OK");
    let root_body =
        serde_json::from_str::<serde_json::Value>(&root_response.body).expect("browser root JSON");
    assert_eq!(root_body["files"].as_array().unwrap().len(), 0);
    assert!(root_body["directories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|directory| directory["path"] == "Local"));

    let folder_response = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?kinds=Audio&path=Local",
        None,
        "",
        &state,
    )
    .await
    .expect("library browser folder response");
    assert_eq!(folder_response.status, "200 OK");
    let folder_body = serde_json::from_str::<serde_json::Value>(&folder_response.body)
        .expect("browser folder JSON");
    let directories = folder_body["directories"].as_array().unwrap();
    assert_eq!(directories[0]["path"], "Local/Album");
    assert_eq!(directories[0]["fileCount"], 1);

    let album_response = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?kinds=Audio&path=Local%2FAlbum",
        None,
        "",
        &state,
    )
    .await
    .expect("library browser album response");
    let album_body = serde_json::from_str::<serde_json::Value>(&album_response.body)
        .expect("browser album JSON");
    let file = &album_body["files"][0];
    assert_eq!(file["fileName"], "Track.flac");
    let content_id = file["contentId"].as_str().unwrap();
    assert!(content_id.starts_with("sha256:"));
    assert!(super::open_primary_stream_file(&state, content_id, None)
        .await
        .expect("open sha256 stream")
        .is_some());

    let encoded_content_id = content_id.replace(':', "%3A");
    let stream_status = super::route_http_request(
        "GET",
        &format!("/api/v0/streams/{encoded_content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("sha256 stream status");
    assert_eq!(stream_status.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stream_status.body).unwrap()["status"],
        "available"
    );

    let ticket = super::route_http_request(
        "POST",
        &format!("/api/v0/streams/{encoded_content_id}/ticket"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("sha256 stream ticket");
    assert_eq!(ticket.status, "200 OK", "{}", ticket.body);

    let invalid_path = super::route_http_request(
        "GET",
        "/api/v0/library/items/browser?path=..",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid browser path response");
    assert_eq!(invalid_path.status, "400 Bad Request");

    let _ = fs::remove_dir_all(root);
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

    let (legacy_root, legacy_index, _) = super::web_static_file_for_request(
        "/dashboard",
        Some(&root),
        Some(super::ControllerProfile::Legacy),
    )
    .expect("legacy dashboard SPA root");
    let (native_root, native_index, _) = super::web_static_file_for_request(
        "/dashboard",
        Some(&root),
        Some(super::ControllerProfile::Native),
    )
    .expect("native dashboard SPA root");

    assert!(legacy_root.ends_with("legacy"));
    assert!(legacy_index.ends_with("legacy/index.html"));
    assert!(native_root.ends_with("native"));
    assert!(native_index.ends_with("native/index.html"));
    assert!(super::web_static_file_for_request(
        "/health",
        Some(&root),
        Some(super::ControllerProfile::Legacy),
    )
    .is_none());
    assert!(super::web_static_file_for_request(
        "/health/mesh",
        Some(&root),
        Some(super::ControllerProfile::Native),
    )
    .is_none());
    assert!(super::web_static_file_for_request(
        "/health?probe=1",
        Some(&root),
        Some(super::ControllerProfile::Legacy),
    )
    .is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn relay_share_database_storage_uses_restart_stable_suffix() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let token = uuid::Uuid::new_v4();
    let stored = super::persist_relay_share_database(&state, token, "shares.sqlite", b"fixture")
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
    let scan = super::scan_share_dirs_with_cancellation(super::ShareScanRequest {
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
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create share-index persistence database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let before = state.shares.read().await.clone();

    // Keep the production rebuild between cancellation registration and worker
    // startup, so shutdown necessarily overlaps the active rebuild operation.
    let lifecycle_guard = state.share_lifecycle.write().await;
    let scan_state = Arc::clone(&state);
    let mut rebuild = tokio::spawn(async move { super::rebuild_share_index(&scan_state).await });
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

    super::initiate_graceful_shutdown(&state).await;
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
        super::SHARE_SCAN_CANCELLED_ERROR
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
async fn transfer_durability_snapshots_after_lock_and_rehydrates_in_event_order() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let events_path = super::transfer_events_path(&state.config.state_dir);
    let state_path = super::transfer_state_path(&state.config.state_dir);
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

    super::persist_transfer_durability(&state).await;
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

    let restarted = super::TransferQueue::new(&state.config);
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
    let db = super::persistence::DatabaseManager::in_memory()
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
        super::persisted_transfer_record(&entry),
        super::persisted_transfer_event_record(&entry),
    )])
    .await
    .expect("seed durable transfer snapshot");

    let request_id = entry.request_id.as_deref().expect("download request id");
    let headers = super::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:1".parse().expect("loopback test address")),
        ..super::RequestSecurityHeaders::default()
    };
    let response = super::route_dispatch::route_http_request_with_headers(
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
async fn distributed_persistence_worker_commits_the_latest_snapshot() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory distributed database");
    let initial = super::DistributedRuntime::new(Some("local-user")).persistence_snapshot();
    let (snapshot_sender, snapshot_receiver) = tokio::sync::watch::channel(initial);
    let (status_sender, mut status_receiver) =
        tokio::sync::watch::channel(super::DistributedPersistenceStatus {
            revision: 0,
            result: Ok(()),
        });
    let worker = tokio::spawn(super::run_distributed_persistence_worker(
        Some(db.clone()),
        snapshot_receiver,
        status_sender,
    ));

    snapshot_sender.send_replace(super::DistributedPersistenceSnapshot {
        revision: 1,
        branch_level: 2,
        branch_root: "intermediate-root".to_owned(),
        parent_username: Some("intermediate-parent".to_owned()),
        children: vec![("intermediate-child".to_owned(), 1)],
    });
    snapshot_sender.send_replace(super::DistributedPersistenceSnapshot {
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
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create distributed restart database");
    let children = vec![("child-a".to_owned(), 2), ("child-b".to_owned(), 4)];
    db.save_distributed_state(6, "branch-root", Some("parent-user"), &children)
        .await
        .expect("persist distributed runtime state");
    db.close_for_test().await;

    let reopened = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("reopen distributed restart database");
    let runtime = RwLock::new(super::DistributedRuntime::new(Some("local-user")));
    super::hydrate_distributed_runtime(&runtime, &reopened)
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
        super::persist_transfer_durability(&persistence_state).await;
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
    let events_path = super::transfer_events_path(&state.config.state_dir);
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
    super::transfer_state_io::pause_transfer_event_sync(
        events_path.clone(),
        started_tx,
        release_rx,
    );
    let persistence_state = Arc::clone(&state);
    let persistence = tokio::spawn(async move {
        super::persist_transfer_durability(&persistence_state).await;
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
    super::persist_transfer_durability(&state).await;
    let events = fs::read_to_string(events_path).expect("read durable transfer events");
    assert!(events.contains("\tqueued\t"));
    assert!(events.contains("\t25\tin_progress\t"));
    let restarted = super::TransferQueue::new(&state.config);
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
            super::persist_transfer_durability(&state).await;
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
            "focused_controller_tests::transfer_durability_survives_killed_writer_process",
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

#[tokio::test]
async fn wishlist_startup_loader_rehydrates_ignored_rules_after_database_reopen() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-wishlist-startup-reload-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create wishlist restart directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create wishlist restart database");
    db.upsert_wishlist_item(&super::persistence::WishlistItemRecord {
        id: "wish-17".to_owned(),
        artist: "Artist".to_owned(),
        title: "Rehydrated Album".to_owned(),
        kind: "Audio".to_owned(),
        filter: "flac".to_owned(),
        enabled: true,
        auto_download: false,
        max_results: 10,
        max_downloads: None,
        last_viewed_at: None,
        last_searched_at: None,
        last_match_count: 0,
        last_visible_hit_count: 0,
        last_hidden_locked_hit_count: 0,
        last_filtered_out_hit_count: 0,
        last_ignored_result_hit_count: 0,
        last_response_count: 0,
        total_search_count: 0,
        total_download_count: 0,
        last_search_id: None,
        lidarr_album_id: None,
        lidarr_track_id: None,
        lidarr_track_count: None,
        lidarr_duration_seconds: None,
        lidarr_release_disambiguation: None,
        added_at: 1,
    })
    .await
    .expect("persist wishlist item");
    db.upsert_wishlist_ignored_result_and_searches(
        &super::persistence::WishlistIgnoredResultRecord {
            id: "ignored-17".to_owned(),
            wishlist_item_id: "wish-17".to_owned(),
            username: "PeerOne".to_owned(),
            directory: "Remote/Album".to_owned(),
            created_at: 2,
        },
        &[],
    )
    .await
    .expect("persist ignored-result rule");
    db.close_for_test().await;

    let reopened = super::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("reopen wishlist restart database");
    let wishlist = super::load_wishlist_store(Some(&reopened))
        .await
        .expect("load wishlist state through the application startup path");
    reopened.close_for_test().await;

    let (state, mut session_commands) = test_state_with_env(MapEnv::default());
    *state.wishlist.write().await = wishlist;
    let ignored = super::route_http_request(
        "GET",
        "/api/wishlist/wish-17/ignored-results",
        None,
        "",
        &state,
    )
    .await
    .expect("read rehydrated ignored-result rule");
    assert_eq!(ignored.status, "200 OK");
    let ignored_json = serde_json::from_str::<serde_json::Value>(&ignored.body).unwrap();
    assert_eq!(ignored_json[0]["id"], "ignored-17");
    assert_eq!(ignored_json[0]["directory"], "Remote/Album");

    let started =
        super::route_http_request("POST", "/api/wishlist/wish-17/search", None, "", &state)
            .await
            .expect("start search for rehydrated wishlist item");
    assert_eq!(started.status, "202 Accepted");
    let token = serde_json::from_str::<serde_json::Value>(&started.body).unwrap()["token"]
        .as_u64()
        .expect("search token");
    assert!(matches!(
        session_commands.try_recv().expect("wishlist search command"),
        super::SessionCommand::Search {
            token: command_token,
            target: super::SearchDispatchTarget::Wishlist,
            ..
        } if u64::from(command_token) == token
    ));

    for filename in ["Remote/Album/Blocked.flac", "Remote/Other/Allowed.flac"] {
        super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            &format!(
                r#"{{"token":{token},"username":"peerone","filename":"{filename}","size":1}}"#
            ),
            &state,
        )
        .await
        .expect("ingest result after wishlist startup hydration");
    }
    let search = state.searches.read().await.records[0].clone();
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0].filename, "Remote/Other/Allowed.flac");

    fs::remove_dir_all(root).expect("remove wishlist restart directory");
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
            super::TransferRequestDetails {
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
        super::transfer_completion::apply_lidarr_rejection_policy(
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
async fn wishlist_ignore_and_search_response_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Album","filter":"flac","enabled":true,"autoDownload":false,"maxResults":25}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    assert_eq!(created.status, "201 Created");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("wishlist item id")
        .to_owned();
    let started = super::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "",
        &state,
    )
    .await
    .expect("start wishlist search");
    assert_eq!(started.status, "202 Accepted");
    let started_json = serde_json::from_str::<serde_json::Value>(&started.body).unwrap();
    let token = started_json["token"].as_u64().expect("search token");
    let search_id = started_json["search_id"]
        .as_str()
        .expect("search id")
        .to_owned();

    // Queue a matching result before the ignore request. The shared turn must
    // let the response persist first, then commit the rule and the suppressed
    // search snapshot together.
    let persistence_turn = state.wishlist_search_persistence_lock.lock().await;
    let response_state = Arc::clone(&state);
    let mut response = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            &format!(
                r#"{{"token":{token},"username":"PeerOne","filename":"Remote/Album/Blocked.flac","size":1}}"#
            ),
            &response_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut response)
            .await
            .is_err()
    );

    let ignore_state = Arc::clone(&state);
    let ignore_path = format!("/api/wishlist/{item_id}/ignored-results");
    let mut ignore = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            &ignore_path,
            None,
            r#"{"username":"PeerOne","directory":"Remote/Album"}"#,
            &ignore_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut ignore)
        .await
        .is_err());

    drop(persistence_turn);
    let response = response
        .await
        .expect("search response task completes")
        .expect("ingest matching search response");
    assert_eq!(response.status, "200 OK");
    let ignored = ignore
        .await
        .expect("ignored-result task completes")
        .expect("create ignored-result rule");
    assert_eq!(ignored.status, "201 Created");
    assert!(state.searches.read().await.records[0].results.is_empty());
    assert!(db
        .list_search_results(Some(&search_id), 10, 0)
        .await
        .expect("read ordered persisted search results")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_ignore_prevents_a_queued_search_snapshot_from_restoring_results() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist snapshot-order database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Album","filter":"flac","enabled":true,"autoDownload":false,"maxResults":25}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("wishlist item id")
        .to_owned();
    let started = super::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "",
        &state,
    )
    .await
    .expect("start wishlist search");
    let token = serde_json::from_str::<serde_json::Value>(&started.body).unwrap()["token"]
        .as_u64()
        .expect("search token") as u32;
    let search_id = serde_json::from_str::<serde_json::Value>(&started.body).unwrap()["search_id"]
        .as_str()
        .expect("search id")
        .to_owned();
    super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        &format!(
            r#"{{"token":{token},"username":"PeerOne","filename":"Remote/Album/Blocked.flac","size":1}}"#
        ),
        &state,
    )
    .await
    .expect("persist matching search result");

    // Let ignore creation queue first for SQLite. A later status snapshot must
    // re-read the committed in-memory record after the ignore transaction.
    let persistence_turn = state.search_persistence_lock.lock().await;
    let ignore_state = Arc::clone(&state);
    let ignore_path = format!("/api/wishlist/{item_id}/ignored-results");
    let mut ignore = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            &ignore_path,
            None,
            r#"{"username":"PeerOne","directory":"Remote/Album"}"#,
            &ignore_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut ignore)
        .await
        .is_err());

    let stale_snapshot = {
        let mut searches = state.searches.write().await;
        let (record, transitioned) = searches.complete(token).expect("complete wishlist search");
        assert!(transitioned);
        record
    };
    let snapshot_state = Arc::clone(&state);
    let mut snapshot_write = tokio::spawn(async move {
        super::persist_search_record(&snapshot_state, &stale_snapshot).await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut snapshot_write)
            .await
            .is_err()
    );

    drop(persistence_turn);
    let ignored = ignore
        .await
        .expect("ignored-result task completes")
        .expect("create ignored-result rule");
    assert_eq!(ignored.status, "201 Created");
    snapshot_write
        .await
        .expect("queued snapshot task completes")
        .expect("persist latest search snapshot");

    let current = state
        .searches
        .read()
        .await
        .get(token)
        .expect("search remains");
    assert_eq!(current.status, "completed");
    assert!(current.results.is_empty());
    assert!(db
        .list_search_results(Some(&search_id), 10, 0)
        .await
        .expect("read suppressed persisted results")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_item_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist item ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Original","filter":"flac","enabled":true,"autoDownload":false,"maxResults":25}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    assert_eq!(created.status, "201 Created");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("wishlist item id")
        .to_owned();

    let persistence_turn = state.wishlist_search_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/wishlist/{item_id}");
    let mut update = tokio::spawn(async move {
        super::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"artist":"Artist","title":"Updated","filter":"flac"}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/wishlist/{item_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable =
        super::route_http_request("GET", &format!("/api/wishlist/{item_id}"), None, "", &state)
            .await
            .expect("wishlist read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["title"],
        "Original"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("wishlist update completes")
        .expect("update wishlist item");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("wishlist delete completes")
        .expect("delete wishlist item");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.wishlist.read().await.get_item(&item_id).is_none());
    assert!(!db
        .list_wishlist_items(10, 0)
        .await
        .expect("read final wishlist rows")
        .iter()
        .any(|item| item.id == item_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn library_snapshot_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create library item ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/v0/library/items",
        None,
        r#"{"artist":"Artist","title":"Album","kind":""}"#,
        &state,
    )
    .await
    .expect("create library item with a fixable kind");
    assert_eq!(created.status, "201 Created");
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("library item id")
        .to_owned();

    let persistence_turn = state.library_persistence_lock.lock().await;
    let fix_state = Arc::clone(&state);
    let mut fix = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/v0/library/health/issues/fix",
            None,
            "{}",
            &fix_state,
        )
        .await
    });
    if let Ok(result) = tokio::time::timeout(Duration::from_millis(10), &mut fix).await {
        let response = result
            .expect("library remediation request does not panic")
            .expect("library remediation route handles the request");
        panic!(
            "library remediation bypassed its persistence turn: {} {}",
            response.status, response.body
        );
    }

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/v0/library/items/{item_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request(
        "GET",
        &format!("/api/v0/library/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("library read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["kind"],
        ""
    );

    drop(persistence_turn);
    let fixed = fix
        .await
        .expect("library remediation completes")
        .expect("fix library item");
    assert_eq!(fixed.status, "200 OK");
    let deleted = delete
        .await
        .expect("library delete completes")
        .expect("delete library item");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.library.read().await.get(&item_id).is_none());
    assert!(!db
        .list_library_items(10, 0)
        .await
        .expect("read final library rows")
        .iter()
        .any(|item| item.id == item_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn contact_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create contact ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("create contact");
    assert_eq!(created.status, "201 Created");
    let contact_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("contact id")
        .to_owned();

    let persistence_turn = state.contact_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/contacts/{contact_id}");
    let mut update = tokio::spawn(async move {
        super::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"username":"updated","online":true}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/contacts/{contact_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request(
        "GET",
        &format!("/api/contacts/{contact_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("contact read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["username"],
        "friend"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("contact update completes")
        .expect("update contact");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("contact delete completes")
        .expect("delete contact");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.contacts.read().await.get(&contact_id).is_none());
    assert!(!db
        .list_contacts(10, 0)
        .await
        .expect("read final contact rows")
        .iter()
        .any(|contact| contact.id == contact_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn user_note_update_and_delete_share_one_persistence_order() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("create user-note ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = super::route_http_request(
        "POST",
        "/api/users/notes",
        None,
        r#"{"username":"friend","note":"Original"}"#,
        &state,
    )
    .await
    .expect("create user note");
    assert_eq!(created.status, "201 Created");
    let note_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("user-note id")
        .to_owned();

    let persistence_turn = state.user_note_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/users/notes/{note_id}");
    let mut update = tokio::spawn(async move {
        super::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"note":"Updated"}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/users/notes/{note_id}");
    let mut delete = tokio::spawn(async move {
        super::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = super::route_http_request(
        "GET",
        &format!("/api/users/notes/{note_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("user-note read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["note"],
        "Original"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("user-note update completes")
        .expect("update user note");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("user-note delete completes")
        .expect("delete user note");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.user_notes.read().await.get(&note_id).is_none());
    assert!(!db
        .list_user_notes(10, 0)
        .await
        .expect("read final user-note rows")
        .iter()
        .any(|note| note.id == note_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}
