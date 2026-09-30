use super::*;

#[test]
fn database_counts_reject_negative_values_without_wrapping() {
    assert_eq!(nonnegative_database_count(0), Ok(0));
    assert_eq!(nonnegative_database_count(i64::MAX), Ok(i64::MAX as u64));
    assert!(nonnegative_database_count(-1).is_err());
}

#[tokio::test]
async fn test_database_creation() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let stats = db.get_stats().await.unwrap();
    assert_eq!(stats.search_count, 0);
    assert_eq!(stats.search_result_count, 0);
    assert_eq!(stats.transfer_count, 0);
    assert_eq!(stats.message_count, 0);
    assert_eq!(stats.user_count, 0);
    assert_eq!(stats.room_count, 0);
}

#[tokio::test]
async fn manual_import_transaction_updates_only_its_runtime_counter_and_rolls_back_together() {
    fn runtime_record(gc_runs: i64, manual_imports: i64, updated_at: i64) -> RuntimeCompatRecord {
        RuntimeCompatRecord {
            id: "runtime".to_owned(),
            application_restart_requested: false,
            gc_runs,
            autoreplace_enabled: true,
            relay_enabled: false,
            relay_agent_enabled: false,
            bridge_running: false,
            bridge_config_updates: 7,
            options_updates: 8,
            options_yaml_uploads: 9,
            options_yaml_validations: 10,
            profile_invites_created: 11,
            cache_warm_runs: 12,
            backfill_runs: 13,
            songid_runs: 14,
            songid_run_records_json: "[]".to_owned(),
            lidarr_sync_runs: 15,
            lidarr_manual_imports: manual_imports,
            updated_at,
        }
    }

    let db = DatabaseManager::in_memory().await.unwrap();
    db.upsert_runtime_compat_state(&runtime_record(41, 4, 100))
        .await
        .unwrap();
    let successful_item = LibraryItemRecord {
        id: "manual-import-success".to_owned(),
        artist: "Artist".to_owned(),
        title: "Successful import".to_owned(),
        kind: "Audio".to_owned(),
        created_at: 100,
    };
    db.create_library_item_and_record_manual_import(&successful_item, &runtime_record(0, 5, 101))
        .await
        .unwrap();

    let runtime = db.get_runtime_compat_state().await.unwrap().unwrap();
    assert_eq!(runtime.gc_runs, 41);
    assert_eq!(runtime.lidarr_manual_imports, 5);
    assert_eq!(runtime.updated_at, 101);

    db.execute_raw_for_test(
        r#"
        CREATE TRIGGER reject_manual_import_counter
        BEFORE UPDATE ON runtime_compat_state
        WHEN NEW.lidarr_manual_imports = 6
        BEGIN
            SELECT RAISE(ABORT, 'forced manual import counter failure');
        END
        "#,
    )
    .await
    .unwrap();
    let rejected_item = LibraryItemRecord {
        id: "manual-import-rollback".to_owned(),
        artist: "Artist".to_owned(),
        title: "Rejected import".to_owned(),
        kind: "Audio".to_owned(),
        created_at: 102,
    };
    assert!(db
        .create_library_item_and_record_manual_import(&rejected_item, &runtime_record(0, 6, 102),)
        .await
        .is_err());

    let library_items = db.list_library_items(10, 0).await.unwrap();
    assert_eq!(library_items.len(), 1);
    assert_eq!(library_items[0].id, successful_item.id);
    let runtime = db.get_runtime_compat_state().await.unwrap().unwrap();
    assert_eq!(runtime.gc_runs, 41);
    assert_eq!(runtime.lidarr_manual_imports, 5);
    assert_eq!(runtime.updated_at, 101);
}

#[tokio::test]
async fn common_paged_reads_use_ordered_indexes() {
    let db = DatabaseManager::in_memory().await.unwrap();

    let message_plan = query(
        "EXPLAIN QUERY PLAN SELECT id FROM messages WHERE username = 'user' ORDER BY created_at DESC LIMIT 100 OFFSET 0",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let message_details = message_plan
        .iter()
        .map(|row| row.try_get::<String, _>("detail").unwrap())
        .collect::<Vec<_>>();
    assert!(message_details
        .iter()
        .any(|detail| detail.contains("idx_messages_username_created")));
    assert!(!message_details
        .iter()
        .any(|detail| detail.contains("TEMP B-TREE")));

    let library_plan = query(
        "EXPLAIN QUERY PLAN SELECT id FROM library_items ORDER BY created_at DESC LIMIT 100 OFFSET 0",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let library_details = library_plan
        .iter()
        .map(|row| row.try_get::<String, _>("detail").unwrap())
        .collect::<Vec<_>>();
    assert!(library_details
        .iter()
        .any(|detail| detail.contains("idx_library_items_created")));
    assert!(!library_details
        .iter()
        .any(|detail| detail.contains("TEMP B-TREE")));

    let search_result_plan = query(
        "EXPLAIN QUERY PLAN SELECT id FROM search_results WHERE search_id = 'search' ORDER BY id LIMIT 100 OFFSET 0",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let search_result_details = search_result_plan
        .iter()
        .map(|row| row.try_get::<String, _>("detail").unwrap())
        .collect::<Vec<_>>();
    assert!(search_result_details
        .iter()
        .any(|detail| detail.contains("idx_search_results_search")));
    assert!(!search_result_details
        .iter()
        .any(|detail| detail.contains("TEMP B-TREE")));

    let transfer_plan = query(
        "EXPLAIN QUERY PLAN SELECT id FROM transfers WHERE status = 'queued' ORDER BY started_at DESC LIMIT 100 OFFSET 0",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let transfer_details = transfer_plan
        .iter()
        .map(|row| row.try_get::<String, _>("detail").unwrap())
        .collect::<Vec<_>>();
    assert!(transfer_details
        .iter()
        .any(|detail| detail.contains("idx_transfers_status_started")));
    assert!(!transfer_details
        .iter()
        .any(|detail| detail.contains("TEMP B-TREE")));

    let webhook_log_plan = query(
        "EXPLAIN QUERY PLAN SELECT id FROM webhook_logs WHERE webhook_id = 'webhook' ORDER BY timestamp DESC LIMIT 100 OFFSET 0",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let webhook_log_details = webhook_log_plan
        .iter()
        .map(|row| row.try_get::<String, _>("detail").unwrap())
        .collect::<Vec<_>>();
    assert!(webhook_log_details
        .iter()
        .any(|detail| detail.contains("idx_webhook_logs_webhook_timestamp")));
    assert!(!webhook_log_details
        .iter()
        .any(|detail| detail.contains("TEMP B-TREE")));
}

#[tokio::test]
async fn opening_database_drops_redundant_search_result_index() {
    let root = std::env::temp_dir().join(format!(
        "slskr-search-index-migration-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("slskr.db");
    let previous = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    query(
        "CREATE TABLE search_results (id INTEGER PRIMARY KEY AUTOINCREMENT, \
         search_id TEXT NOT NULL, peer_username TEXT, filename TEXT NOT NULL, \
         size INTEGER NOT NULL, extension TEXT NOT NULL, bit_rate INTEGER, \
         sample_rate INTEGER, bit_depth INTEGER, length_seconds INTEGER, \
         locked INTEGER NOT NULL, slot_free INTEGER, average_speed INTEGER, \
         queue_length INTEGER, created_at INTEGER NOT NULL)",
    )
    .execute(&previous)
    .await
    .unwrap();
    query("CREATE INDEX idx_search_results_search_id ON search_results(search_id, id)")
        .execute(&previous)
        .await
        .unwrap();
    previous.close().await;

    let db = DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .unwrap();
    let redundant_index = query(
        "SELECT name FROM sqlite_master WHERE type = 'index' \
         AND name = 'idx_search_results_search_id'",
    )
    .fetch_optional(&db.pool)
    .await
    .unwrap();
    let retained_index = query(
        "SELECT name FROM sqlite_master WHERE type = 'index' \
         AND name = 'idx_search_results_search'",
    )
    .fetch_optional(&db.pool)
    .await
    .unwrap();
    assert!(redundant_index.is_none());
    assert!(retained_index.is_some());

    db.close_for_test().await;
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn database_file_is_private_and_rejects_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let root = std::env::temp_dir().join(format!(
        "slskr-private-db-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let db_path = root.join("slskr.db");
    std::fs::write(&db_path, []).unwrap();
    std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o666)).unwrap();

    let db = DatabaseManager::new(db_path.to_str().unwrap())
        .await
        .unwrap();
    db.close_for_test().await;
    assert_eq!(
        std::fs::metadata(&db_path).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let linked_path = root.join("linked.db");
    symlink(&db_path, &linked_path).unwrap();
    assert!(DatabaseManager::new(linked_path.to_str().unwrap())
        .await
        .is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn test_search_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let record = SearchRecord {
        id: "search_1".to_string(),
        query: "test query".to_string(),
        status: "completed".to_string(),
        result_count: 42,
        created_at: now,
        completed_at: Some(now + 100),
        room: None,
        target: None,
        fallback_attempts: 0,
    };

    db.insert_search(&record).await.unwrap();
    db.upsert_search_identity("search_1", "external-search-1")
        .await
        .unwrap();
    let retrieved = db.get_search("search_1").await.unwrap().unwrap();
    assert_eq!(retrieved.query, "test query");
    assert_eq!(retrieved.result_count, 42);
    assert_eq!(
        db.list_search_identities().await.unwrap().get("search_1"),
        Some(&"external-search-1".to_owned())
    );

    db.update_search_status("search_1", "archived")
        .await
        .unwrap();
    let updated = db.get_search("search_1").await.unwrap().unwrap();
    assert_eq!(updated.status, "archived");

    db.delete_search("search_1").await.unwrap();
    assert!(db.list_search_identities().await.unwrap().is_empty());
}

#[tokio::test]
async fn ignored_wishlist_searches_preserve_metadata_and_batch_rows() {
    let root = std::env::temp_dir().join(format!(
        "slskr-ignored-search-restart-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().unwrap();
    let db = DatabaseManager::new(db_path_string).await.unwrap();
    query("INSERT INTO wishlist_items (id, artist, title, kind, added_at) VALUES (?, ?, ?, ?, ?)")
        .bind("wish_1")
        .bind("Artist")
        .bind("Title")
        .bind("album")
        .bind(1_i64)
        .execute(&db.pool)
        .await
        .unwrap();
    let rule = WishlistIgnoredResultRecord {
        id: "ignored_1".to_owned(),
        wishlist_item_id: "wish_1".to_owned(),
        username: "peer".to_owned(),
        directory: "folder".to_owned(),
        created_at: 1,
    };
    let search = |id: &str, fallback_attempts: i64| SearchRecord {
        id: id.to_owned(),
        query: "ambient".to_owned(),
        status: "completed".to_owned(),
        result_count: 1,
        created_at: 1,
        completed_at: Some(2),
        room: Some("music".to_owned()),
        target: Some("room".to_owned()),
        fallback_attempts,
    };
    let result = SearchResultRecord {
        id: 0,
        search_id: "ignored_search_1".to_owned(),
        peer_username: Some("peer".to_owned()),
        filename: "track.flac".to_owned(),
        size: 10,
        extension: "flac".to_owned(),
        bit_rate: Some(320),
        sample_rate: Some(44_100),
        bit_depth: Some(16),
        length_seconds: Some(180),
        locked: false,
        slot_free: Some(true),
        average_speed: Some(1),
        queue_length: Some(0),
        created_at: 2,
    };

    db.upsert_wishlist_ignored_result_and_searches(
        &rule,
        &[
            (
                search("ignored_search_1", 3),
                "external_1".to_owned(),
                vec![result],
            ),
            (
                search("ignored_search_2", 5),
                "external_2".to_owned(),
                vec![],
            ),
        ],
    )
    .await
    .unwrap();

    let persisted = db.get_search("ignored_search_1").await.unwrap().unwrap();
    assert_eq!(persisted.fallback_attempts, 3);
    assert_eq!(persisted.room.as_deref(), Some("music"));
    assert_eq!(persisted.target.as_deref(), Some("room"));
    assert_eq!(
        db.list_search_identities()
            .await
            .unwrap()
            .get("ignored_search_1"),
        Some(&"external_1".to_owned())
    );
    assert_eq!(
        db.list_search_results(Some("ignored_search_1"), 10, 0)
            .await
            .unwrap()
            .len(),
        1
    );

    db.close_for_test().await;
    let reopened = DatabaseManager::new(db_path_string).await.unwrap();
    let persisted = reopened
        .get_search("ignored_search_1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(persisted.fallback_attempts, 3);
    assert_eq!(persisted.room.as_deref(), Some("music"));
    assert_eq!(persisted.target.as_deref(), Some("room"));
    assert_eq!(
        reopened
            .list_search_identities()
            .await
            .unwrap()
            .get("ignored_search_1"),
        Some(&"external_1".to_owned())
    );
    assert_eq!(
        reopened
            .list_search_results(Some("ignored_search_1"), 10, 0)
            .await
            .unwrap()
            .len(),
        1
    );
    let persisted_rules = reopened
        .list_wishlist_ignored_results("wish_1")
        .await
        .unwrap();
    assert_eq!(persisted_rules.len(), 1);
    assert_eq!(persisted_rules[0].id, "ignored_1");
    assert_eq!(persisted_rules[0].username, "peer");
    reopened.close_for_test().await;
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn ignored_wishlist_search_batch_rolls_back_rule_and_searches() {
    let db = DatabaseManager::in_memory().await.unwrap();
    query("INSERT INTO wishlist_items (id, artist, title, kind, added_at) VALUES (?, ?, ?, ?, ?)")
        .bind("wish_rollback")
        .bind("Artist")
        .bind("Title")
        .bind("album")
        .bind(1_i64)
        .execute(&db.pool)
        .await
        .unwrap();
    query(
        r#"
        CREATE TRIGGER reject_ignored_search_result
        BEFORE INSERT ON search_results
        WHEN NEW.search_id = 'ignored_search_bad'
        BEGIN
            SELECT RAISE(ABORT, 'forced ignored search result failure');
        END
        "#,
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let rule = WishlistIgnoredResultRecord {
        id: "ignored_rollback".to_owned(),
        wishlist_item_id: "wish_rollback".to_owned(),
        username: "peer".to_owned(),
        directory: "folder".to_owned(),
        created_at: 1,
    };
    let search = SearchRecord {
        id: "ignored_search_bad".to_owned(),
        query: "ambient".to_owned(),
        status: "completed".to_owned(),
        result_count: 1,
        created_at: 1,
        completed_at: Some(2),
        room: Some("music".to_owned()),
        target: Some("room".to_owned()),
        fallback_attempts: 7,
    };
    let result = SearchResultRecord {
        id: 0,
        search_id: search.id.clone(),
        peer_username: Some("peer".to_owned()),
        filename: "rejected.flac".to_owned(),
        size: 10,
        extension: "flac".to_owned(),
        bit_rate: Some(320),
        sample_rate: Some(44_100),
        bit_depth: Some(16),
        length_seconds: Some(180),
        locked: false,
        slot_free: Some(true),
        average_speed: Some(1),
        queue_length: Some(0),
        created_at: 2,
    };

    assert!(db
        .upsert_wishlist_ignored_result_and_searches(
            &rule,
            &[(search.clone(), "external_bad".to_owned(), vec![result])],
        )
        .await
        .is_err());
    assert!(db
        .list_wishlist_ignored_results("wish_rollback")
        .await
        .unwrap()
        .is_empty());
    assert!(db.get_search(&search.id).await.unwrap().is_none());
    assert!(!db
        .list_search_identities()
        .await
        .unwrap()
        .contains_key(&search.id));
    assert!(db
        .list_search_results(Some(&search.id), 10, 0)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn search_result_replacement_rolls_back_on_insert_failure() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let result = |filename: &str| SearchResultRecord {
        id: 0,
        search_id: "search_atomic".to_owned(),
        peer_username: Some("peer".to_owned()),
        filename: filename.to_owned(),
        size: 10,
        extension: "flac".to_owned(),
        bit_rate: None,
        sample_rate: None,
        bit_depth: None,
        length_seconds: None,
        locked: false,
        slot_free: Some(true),
        average_speed: Some(1),
        queue_length: Some(0),
        created_at: 1,
    };
    db.replace_search_results("search_atomic", &[result("original.flac")])
        .await
        .unwrap();
    query(
        r#"
        CREATE TRIGGER reject_bad_search_result
        BEFORE INSERT ON search_results
        WHEN NEW.filename = 'rejected.flac'
        BEGIN
            SELECT RAISE(ABORT, 'forced search result failure');
        END
        "#,
    )
    .execute(&db.pool)
    .await
    .unwrap();

    assert!(db
        .replace_search_results(
            "search_atomic",
            &[result("new.flac"), result("rejected.flac")],
        )
        .await
        .is_err());
    let persisted = db
        .list_search_results(Some("search_atomic"), 10, 0)
        .await
        .unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].filename, "original.flac");
}

#[tokio::test]
async fn appending_search_results_preserves_existing_projection_rows() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let record = SearchRecord {
        id: "search_delta".to_owned(),
        query: "ambient".to_owned(),
        status: "active".to_owned(),
        result_count: 1,
        created_at: 1,
        completed_at: None,
        room: None,
        target: Some("global".to_owned()),
        fallback_attempts: 0,
    };
    let result = |filename: &str, created_at: i64| SearchResultRecord {
        id: 0,
        search_id: record.id.clone(),
        peer_username: Some("peer".to_owned()),
        filename: filename.to_owned(),
        size: 10,
        extension: "flac".to_owned(),
        bit_rate: None,
        sample_rate: None,
        bit_depth: None,
        length_seconds: None,
        locked: false,
        slot_free: Some(true),
        average_speed: Some(1),
        queue_length: Some(0),
        created_at,
    };
    db.persist_search(&record, "external_delta", &[result("first.flac", 1)])
        .await
        .unwrap();

    let mut updated = record.clone();
    updated.result_count = 2;
    db.append_search_results(&updated, "external_delta", &[result("second.flac", 2)])
        .await
        .unwrap();

    let rows = db
        .list_search_results(Some(&record.id), 10, 0)
        .await
        .unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| row.filename.as_str())
            .collect::<Vec<_>>(),
        ["first.flac", "second.flac",]
    );
    assert_eq!(
        db.get_search(&record.id)
            .await
            .unwrap()
            .unwrap()
            .result_count,
        2
    );
}

#[tokio::test]
async fn test_transfer_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let record = TransferRecord {
        id: "transfer_1".to_string(),
        direction: "download".to_string(),
        filename: "test.mp3".to_string(),
        peer_username: "user1".to_string(),
        filesize: 1000000,
        progress: 500000,
        status: "active".to_string(),
        started_at: now,
        completed_at: None,
        request_id: Some("request_1".to_owned()),
        wishlist_item_id: Some("wish-1".to_owned()),
        request_name: Some("Test".to_owned()),
        destination_directory: Some("Artist/Album".to_owned()),
        local_path: Some("downloads/Artist/Album/test.mp3".to_owned()),
        batch_id: None,
        reason: None,
        bit_rate: Some(320),
        sample_rate: Some(44_100),
        bit_depth: Some(16),
        length_seconds: Some(180),
        artist: Some("Artist".to_owned()),
        album: Some("Album".to_owned()),
        title: Some("Test".to_owned()),
        track_number: Some(1),
        year: Some(2026),
        attempts: 1,
        auto_replace_attempts: 0,
        next_attempt_at: None,
        updated_at_ms: now.saturating_mul(1_000),
    };

    db.insert_transfer(&record).await.unwrap();
    let retrieved = db.get_transfer("transfer_1").await.unwrap().unwrap();
    assert_eq!(retrieved.filename, "test.mp3");
    assert_eq!(retrieved.progress, 500000);
    assert_eq!(retrieved.bit_rate, Some(320));
    assert_eq!(retrieved.request_id.as_deref(), Some("request_1"));
    assert_eq!(retrieved.wishlist_item_id.as_deref(), Some("wish-1"));

    db.update_transfer_progress("transfer_1", 750000, record.updated_at_ms as u64 + 1)
        .await
        .unwrap();
    let updated = db.get_transfer("transfer_1").await.unwrap().unwrap();
    assert_eq!(updated.progress, 750000);

    db.update_transfer_progress("transfer_1", u64::MAX, record.updated_at_ms as u64 + 2)
        .await
        .unwrap();
    let saturated = db.get_transfer("transfer_1").await.unwrap().unwrap();
    assert_eq!(saturated.progress, i64::MAX);

    db.insert_transfer_event(&TransferEventRecord {
        id: 0,
        transfer_id: "transfer_1".to_owned(),
        direction: "download".to_owned(),
        token: 1,
        filename: "test.mp3".to_owned(),
        peer_username: Some("user1".to_owned()),
        filesize: 1_000_000,
        progress: 750_000,
        status: "peer_lookup".to_owned(),
        reason: None,
        created_at: now,
        updated_at_ms: record.updated_at_ms + 2,
    })
    .await
    .unwrap();
    db.rollback_staged_transfers(&[("transfer_1".to_owned(), record.updated_at_ms + 3)])
        .await
        .unwrap();
    assert!(db.get_transfer("transfer_1").await.unwrap().is_none());
    assert!(db
        .list_transfer_events(Some("transfer_1"), 10, 0)
        .await
        .unwrap()
        .is_empty());

    let mut ids = Vec::new();
    for suffix in ["a", "b", "c"] {
        let mut additional = record.clone();
        additional.id = format!("transfer_{suffix}");
        db.insert_transfer(&additional).await.unwrap();
        ids.push(additional.id);
    }
    db.delete_transfers(&ids).await.unwrap();
    assert!(db.get_transfer("transfer_a").await.unwrap().is_none());
    assert!(db.get_transfer("transfer_b").await.unwrap().is_none());
    assert!(db.get_transfer("transfer_c").await.unwrap().is_none());
    db.delete_transfers(&[]).await.unwrap();
}

#[tokio::test]
async fn transfer_record_and_event_batches_roll_back_atomically() {
    let db = DatabaseManager::in_memory().await.unwrap();
    query(
        r#"
        CREATE TRIGGER reject_bad_transfer_event
        BEFORE INSERT ON transfer_events
        WHEN NEW.transfer_id = 'transfer_bad'
        BEGIN
            SELECT RAISE(ABORT, 'forced transfer event failure');
        END
        "#,
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let transfer = |id: &str| TransferRecord {
        id: id.to_owned(),
        direction: "download".to_owned(),
        filename: format!("{id}.flac"),
        peer_username: "peer".to_owned(),
        filesize: 10,
        progress: 0,
        status: "queued".to_owned(),
        started_at: 1,
        completed_at: None,
        request_id: None,
        wishlist_item_id: None,
        request_name: None,
        destination_directory: None,
        local_path: None,
        batch_id: None,
        reason: None,
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
        updated_at_ms: 10,
    };
    let event = |id: &str| TransferEventRecord {
        id: 0,
        transfer_id: id.to_owned(),
        direction: "download".to_owned(),
        token: 1,
        filename: format!("{id}.flac"),
        peer_username: Some("peer".to_owned()),
        filesize: 10,
        progress: 0,
        status: "queued".to_owned(),
        reason: None,
        created_at: 1,
        updated_at_ms: 10,
    };
    let records = [
        (transfer("transfer_good"), event("transfer_good")),
        (transfer("transfer_bad"), event("transfer_bad")),
    ];

    assert!(db
        .insert_transfer_records_with_events(&records)
        .await
        .is_err());
    for id in ["transfer_good", "transfer_bad"] {
        assert!(db.get_transfer(id).await.unwrap().is_none(), "{id}");
        assert!(
            db.list_transfer_events(Some(id), 10, 0)
                .await
                .unwrap()
                .is_empty(),
            "{id}"
        );
    }
}

#[tokio::test]
async fn transfer_projection_rejects_stale_snapshots_and_deleted_rows() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let transfer = |revision: i64, progress: i64, status: &str| TransferRecord {
        id: "7".to_owned(),
        direction: "download".to_owned(),
        filename: "ordered.flac".to_owned(),
        peer_username: "peer".to_owned(),
        filesize: 100,
        progress,
        status: status.to_owned(),
        started_at: 1,
        completed_at: None,
        request_id: None,
        wishlist_item_id: None,
        request_name: None,
        destination_directory: None,
        local_path: None,
        batch_id: None,
        reason: None,
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
        updated_at_ms: revision,
    };
    let event = |revision: i64, progress: i64, status: &str| TransferEventRecord {
        id: 0,
        transfer_id: "7".to_owned(),
        direction: "download".to_owned(),
        token: 7,
        filename: "ordered.flac".to_owned(),
        peer_username: Some("peer".to_owned()),
        filesize: 100,
        progress,
        status: status.to_owned(),
        reason: None,
        created_at: 1,
        updated_at_ms: revision,
    };

    for (revision, progress, status) in [
        (10, 10, "queued"),
        (20, 20, "completed"),
        (15, 15, "in_progress"),
    ] {
        db.insert_transfer_records_with_events(&[(
            transfer(revision, progress, status),
            event(revision, progress, status),
        )])
        .await
        .unwrap();
    }

    let stored = db.get_transfer("7").await.unwrap().unwrap();
    assert_eq!(stored.status, "completed");
    assert_eq!(stored.progress, 20);
    assert_eq!(stored.updated_at_ms, 20);
    let events = db.list_transfer_events(Some("7"), 10, 0).await.unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.updated_at_ms)
            .collect::<Vec<_>>(),
        [20, 15, 10]
    );

    db.delete_transfer_records(&[("7".to_owned(), 30)])
        .await
        .unwrap();
    db.insert_transfer_records_with_events(&[(
        transfer(25, 25, "in_progress"),
        event(25, 25, "in_progress"),
    )])
    .await
    .unwrap();
    assert!(db.get_transfer("7").await.unwrap().is_none());
    assert_eq!(
        db.list_transfer_events(Some("7"), 10, 0)
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(db.max_transfer_id().await.unwrap(), 7);
}

#[tokio::test]
async fn existing_transfer_schema_migrates_revision_columns_and_tombstones() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    query(
        r#"
        CREATE TABLE transfers (
            id TEXT PRIMARY KEY,
            direction TEXT NOT NULL,
            filename TEXT NOT NULL,
            peer_username TEXT NOT NULL,
            filesize INTEGER NOT NULL,
            progress INTEGER DEFAULT 0,
            status TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            completed_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    query(
        r#"
        CREATE TABLE transfer_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            transfer_id TEXT NOT NULL,
            direction TEXT NOT NULL,
            token INTEGER NOT NULL,
            filename TEXT NOT NULL,
            peer_username TEXT,
            filesize INTEGER NOT NULL,
            progress INTEGER NOT NULL,
            status TEXT NOT NULL,
            reason TEXT,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    query(
        "INSERT INTO transfers (id, direction, filename, peer_username, filesize, progress, status, started_at) VALUES ('9', 'download', 'legacy.flac', 'peer', 10, 0, 'queued', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    query(
        "INSERT INTO transfer_events (transfer_id, direction, token, filename, peer_username, filesize, progress, status, created_at) VALUES ('9', 'download', 9, 'legacy.flac', 'peer', 10, 0, 'queued', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let db = DatabaseManager { pool };
    db.initialize().await.unwrap();
    let transfer = db.get_transfer("9").await.unwrap().unwrap();
    assert_eq!(transfer.updated_at_ms, 0);
    assert_eq!(transfer.attempts, 1);
    let events = db.list_transfer_events(Some("9"), 10, 0).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].updated_at_ms, 0);

    db.delete_transfer_records(&[("9".to_owned(), 1)])
        .await
        .unwrap();
    let stale = TransferRecord {
        id: "9".to_owned(),
        direction: "download".to_owned(),
        filename: "legacy.flac".to_owned(),
        peer_username: "peer".to_owned(),
        filesize: 10,
        progress: 0,
        status: "queued".to_owned(),
        started_at: 1,
        completed_at: None,
        request_id: None,
        wishlist_item_id: None,
        request_name: None,
        destination_directory: None,
        local_path: None,
        batch_id: None,
        reason: None,
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
        updated_at_ms: 0,
    };
    db.insert_transfer_records_with_events(&[(
        stale,
        TransferEventRecord {
            id: 0,
            transfer_id: "9".to_owned(),
            direction: "download".to_owned(),
            token: 9,
            filename: "legacy.flac".to_owned(),
            peer_username: Some("peer".to_owned()),
            filesize: 10,
            progress: 0,
            status: "queued".to_owned(),
            reason: None,
            created_at: 1,
            updated_at_ms: 0,
        },
    )])
    .await
    .unwrap();
    assert!(db.get_transfer("9").await.unwrap().is_none());
    assert_eq!(
        db.list_transfer_events(Some("9"), 10, 0)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(db.max_transfer_id().await.unwrap(), 9);
    db.close_for_test().await;
}

#[tokio::test]
async fn webhook_delete_rolls_back_log_deletion_on_failure() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let webhook = WebhookRecord {
        id: "hook_atomic".to_owned(),
        url: "https://example.com/hook".to_owned(),
        events: "search.created".to_owned(),
        secret: crate::webhooks::Webhook::generate_secret().expect("test randomness"),
        active: true,
        created_at: 1,
        last_triggered: None,
        retry_count: 0,
        max_retries: 3,
        timeout_seconds: 30,
    };
    db.insert_webhook(&webhook).await.unwrap();
    db.insert_webhook_log(&WebhookLogRecord {
        id: "log_atomic".to_owned(),
        webhook_id: webhook.id.clone(),
        event: "search.created".to_owned(),
        correlation_id: "correlation".to_owned(),
        status: "success".to_owned(),
        request_body: "{}".to_owned(),
        response_status: Some(200),
        response_body: None,
        error_message: None,
        attempt: 1,
        timestamp: 1,
    })
    .await
    .unwrap();
    query(
        r#"
        CREATE TRIGGER reject_webhook_delete
        BEFORE DELETE ON webhooks
        WHEN OLD.id = 'hook_atomic'
        BEGIN
            SELECT RAISE(ABORT, 'forced webhook delete failure');
        END
        "#,
    )
    .execute(&db.pool)
    .await
    .unwrap();

    assert!(db.delete_webhook(&webhook.id).await.is_err());
    assert!(db.get_webhook(&webhook.id).await.unwrap().is_some());
    let logs = db.get_webhook_logs(&webhook.id, 10, 0).await.unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].id, "log_atomic");
}

#[tokio::test]
async fn test_message_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let record = MessageRecord {
        id: "msg_1".to_string(),
        username: "user1".to_string(),
        content: "Hello!".to_string(),
        direction: "incoming".to_string(),
        read: false,
        created_at: now,
        source_id: None,
        source_timestamp: None,
        was_replayed: false,
    };

    db.insert_message(&record).await.unwrap();
    let messages = db.list_messages_from_user("user1", 10, 0).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello!");

    db.mark_message_read("msg_1").await.unwrap();
    let messages = db.list_messages_from_user("user1", 10, 0).await.unwrap();
    assert!(messages[0].read);

    let newer = MessageRecord {
        id: "msg_2".to_string(),
        username: "user1".to_string(),
        content: "Later".to_string(),
        direction: "outgoing".to_string(),
        read: false,
        created_at: now + 1,
        source_id: None,
        source_timestamp: None,
        was_replayed: false,
    };
    db.insert_message(&newer).await.unwrap();

    let first_page = db.list_messages_from_user("user1", 1, 0).await.unwrap();
    assert_eq!(first_page.len(), 1);
    assert_eq!(first_page[0].id, "msg_2");

    let second_page = db.list_messages_from_user("user1", 1, 1).await.unwrap();
    assert_eq!(second_page.len(), 1);
    assert_eq!(second_page[0].id, "msg_1");
}

#[tokio::test]
async fn bulk_message_operations_cross_sqlite_bind_chunk_boundary() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let records = (0..101)
        .map(|index| MessageRecord {
            id: format!("bulk_msg_{index}"),
            username: "bulk-user".to_owned(),
            content: format!("message {index}"),
            direction: "incoming".to_owned(),
            read: false,
            created_at: index,
            source_id: None,
            source_timestamp: None,
            was_replayed: false,
        })
        .collect::<Vec<_>>();

    db.insert_messages(&records).await.unwrap();
    let ids = records
        .iter()
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();
    db.mark_messages_read(&ids).await.unwrap();

    let persisted = db.list_messages(200, 0).await.unwrap();
    assert_eq!(persisted.len(), records.len());
    assert!(persisted.iter().all(|record| record.read));
}

#[tokio::test]
async fn test_room_subscription_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();

    db.subscribe_room("music", Some("owner")).await.unwrap();
    db.subscribe_room("chat", None).await.unwrap();

    let rooms = db.list_subscribed_rooms().await.unwrap();
    assert_eq!(rooms.len(), 2);
    assert_eq!(rooms[0].name, "chat");
    assert_eq!(rooms[0].owner, None);
    assert!(rooms[0].subscribed);
    assert_eq!(rooms[1].name, "music");
    assert_eq!(rooms[1].owner.as_deref(), Some("owner"));
    assert!(rooms[1].joined_at > 0);
    assert!(rooms[1].last_activity >= rooms[1].joined_at);

    db.unsubscribe_room("chat").await.unwrap();
    let rooms = db.list_subscribed_rooms().await.unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].name, "music");
}

#[tokio::test]
async fn test_user_stats_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();

    db.update_user_stats("testuser", 10, 5, 1000000, 500000)
        .await
        .unwrap();
    let stats = db.get_user_stats("testuser").await.unwrap();
    assert!(stats.is_some());
    let s = stats.unwrap();
    assert_eq!(s.uploads, 10);
    assert_eq!(s.downloads, 5);
}

#[tokio::test]
async fn test_user_projection_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let record = UserProjectionRecord {
        username: "friend".to_owned(),
        watched: true,
        status: Some("Online".to_owned()),
        average_speed: Some(2048),
        upload_count: Some(7),
        file_count: Some(123),
        directory_count: Some(4),
        updated_at: 42,
    };

    db.upsert_user_projection(&record).await.unwrap();
    let records = db.list_user_projections(10, 0).await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].username, "friend");
    assert!(records[0].watched);
    assert_eq!(records[0].status.as_deref(), Some("Online"));
    assert_eq!(records[0].file_count, Some(123));

    let stats = db.get_stats().await.unwrap();
    assert_eq!(stats.user_projection_count, 1);
}

#[tokio::test]
async fn test_oauth_state_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let record = OAuthStateRecord {
        state: "state-token".to_owned(),
        provider: "spotify".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        created_at: 10,
        expires_at: 20,
    };

    db.upsert_oauth_state(&record).await.unwrap();
    let records = db.list_oauth_states(11, 10, 0).await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "state-token");
    assert_eq!(records[0].provider, "spotify");

    assert!(db.list_oauth_states(20, 10, 0).await.unwrap().is_empty());
    assert_eq!(db.delete_expired_oauth_states(20).await.unwrap(), 1);

    db.upsert_oauth_state(&record).await.unwrap();
    db.delete_oauth_state("state-token").await.unwrap();
    assert!(db.list_oauth_states(11, 10, 0).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_webhook_operations_persist_config_and_logs() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let webhook = WebhookRecord {
        id: "hook_1".to_owned(),
        url: "https://example.com/hook".to_owned(),
        events: "search.created,message.sent".to_owned(),
        secret: crate::webhooks::Webhook::generate_secret().expect("test randomness"),
        active: true,
        created_at: 10,
        last_triggered: None,
        retry_count: 0,
        max_retries: 3,
        timeout_seconds: 30,
    };

    db.insert_webhook(&webhook).await.unwrap();
    let webhooks = db.list_webhooks().await.unwrap();
    assert_eq!(webhooks.len(), 1);
    assert_eq!(webhooks[0].id, "hook_1");

    db.update_webhook_active("hook_1", false).await.unwrap();
    let inactive = db.get_webhook("hook_1").await.unwrap().unwrap();
    assert!(!inactive.active);

    let log = WebhookLogRecord {
        id: "log_1".to_owned(),
        webhook_id: "hook_1".to_owned(),
        event: "search.created".to_owned(),
        correlation_id: "search_1".to_owned(),
        status: "queued".to_owned(),
        request_body: "{}".to_owned(),
        response_status: None,
        response_body: None,
        error_message: None,
        attempt: 1,
        timestamp: 11,
    };
    db.insert_webhook_log(&log).await.unwrap();
    let logs = db.get_webhook_logs("hook_1", 10, 0).await.unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].event, "search.created");

    assert_eq!(
        db.complete_webhook_logs("hook_1", "search_1", "failed", Some("delivery rejected"),)
            .await
            .unwrap(),
        1
    );
    let logs = db.get_webhook_logs("hook_1", 10, 0).await.unwrap();
    assert_eq!(logs[0].status, "failed");
    assert_eq!(logs[0].error_message.as_deref(), Some("delivery rejected"));
    assert_eq!(logs[0].attempt, 1);
    assert_eq!(db.get_failed_webhook_logs(10).await.unwrap().len(), 1);

    let mut successful_log = log.clone();
    successful_log.id = "log_2".to_owned();
    successful_log.correlation_id = "search_2".to_owned();
    db.insert_webhook_log(&successful_log).await.unwrap();
    assert_eq!(
        db.complete_webhook_logs("hook_1", "search_2", "success", None)
            .await
            .unwrap(),
        1
    );
    let logs = db.get_webhook_logs("hook_1", 10, 0).await.unwrap();
    assert!(logs.iter().any(|record| {
        record.correlation_id == "search_2"
            && record.status == "success"
            && record.error_message.is_none()
    }));

    let stats = db.get_stats().await.unwrap();
    assert_eq!(stats.webhook_count, 1);
    assert_eq!(stats.webhook_log_count, 2);

    db.delete_webhook("hook_1").await.unwrap();
    assert!(db.list_webhooks().await.unwrap().is_empty());
}

#[tokio::test]
async fn wishlist_scheduler_state_rejects_out_of_range_values() {
    let db = DatabaseManager::in_memory().await.unwrap();
    db.save_wishlist_scheduler_state(1, Some(60)).await.unwrap();

    db.execute_raw_for_test("UPDATE wishlist_scheduler_state SET next_index = -1 WHERE id = 1")
        .await
        .unwrap();
    assert!(db.load_wishlist_scheduler_state().await.is_err());
    db.execute_raw_for_test("UPDATE wishlist_scheduler_state SET next_index = 1, server_interval_seconds = -1 WHERE id = 1")
        .await
        .unwrap();
    assert!(db.load_wishlist_scheduler_state().await.is_err());

    assert!(db
        .save_wishlist_scheduler_state(1, Some(u64::MAX))
        .await
        .is_err());
    if usize::BITS > 63 {
        assert!(db
            .save_wishlist_scheduler_state(usize::MAX, None)
            .await
            .is_err());
    }
}

#[tokio::test]
async fn test_distributed_tree_state_operations() {
    let root = std::env::temp_dir().join(format!(
        "slskr-distributed-restart-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().unwrap();
    let db = DatabaseManager::new(db_path_string).await.unwrap();

    // Initially no state
    let state = db.load_distributed_tree_state().await.unwrap();
    assert!(state.is_none());

    // Initially no children
    let children = db.load_distributed_children().await.unwrap();
    assert!(children.is_empty());

    let children = vec![
        ("child1".to_owned(), 2),
        ("child2".to_owned(), 3),
        ("child3".to_owned(), 1),
    ];
    db.save_distributed_state(5, "root-user", Some("parent-user"), &children)
        .await
        .unwrap();

    // Load tree state and children from one read snapshot
    let (state, loaded_children) = db.load_distributed_state().await.unwrap();
    let (branch_level, branch_root, parent_username) = state.unwrap();
    assert_eq!(branch_level, 5);
    assert_eq!(branch_root, "root-user");
    assert_eq!(parent_username, Some("parent-user".to_owned()));
    assert_eq!(loaded_children.len(), 3);
    assert!(loaded_children.contains(&("child1".to_owned(), 2)));
    assert!(loaded_children.contains(&("child2".to_owned(), 3)));
    assert!(loaded_children.contains(&("child3".to_owned(), 1)));

    // Update tree state
    let new_children = vec![("child4".to_owned(), 5)];
    db.save_distributed_state(10, "new-root", None, &new_children)
        .await
        .unwrap();

    db.close_for_test().await;
    let reopened = DatabaseManager::new(db_path_string).await.unwrap();
    let (state, children) = reopened.load_distributed_state().await.unwrap();
    let (branch_level, branch_root, parent_username) =
        state.expect("distributed tree state survives database reopen");
    assert_eq!(branch_level, 10);
    assert_eq!(branch_root, "new-root");
    assert_eq!(parent_username, None);
    assert_eq!(children, new_children);
    reopened.close_for_test().await;
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn distributed_state_load_rejects_out_of_range_depths() {
    let db = DatabaseManager::in_memory().await.unwrap();
    db.save_distributed_state(1, "root", None, &[("child".to_owned(), 1)])
        .await
        .unwrap();

    for invalid in ["-1", "4294967296"] {
        db.execute_raw_for_test(&format!(
            "UPDATE distributed_tree_state SET branch_level = {invalid} WHERE id = 1"
        ))
        .await
        .unwrap();
        assert!(db.load_distributed_tree_state().await.is_err());
        assert!(db.load_distributed_state().await.is_err());
    }

    db.execute_raw_for_test("UPDATE distributed_tree_state SET branch_level = 1 WHERE id = 1")
        .await
        .unwrap();
    for invalid in ["-1", "4294967296"] {
        db.execute_raw_for_test(&format!(
            "UPDATE distributed_children SET depth = {invalid} WHERE username = 'child'"
        ))
        .await
        .unwrap();
        assert!(db.load_distributed_children().await.is_err());
        assert!(db.load_distributed_state().await.is_err());
    }
}

#[tokio::test]
async fn distributed_children_replacement_rolls_back_across_batches() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let original = vec![("old-child-1".to_owned(), 1), ("old-child-2".to_owned(), 2)];
    db.save_distributed_children(&original).await.unwrap();
    db.execute_raw_for_test(
        r#"
        CREATE TRIGGER reject_distributed_child
        BEFORE INSERT ON distributed_children
        WHEN NEW.username = 'reject-child'
        BEGIN
            SELECT RAISE(ABORT, 'forced distributed child failure');
        END
        "#,
    )
    .await
    .unwrap();

    let mut replacement = (0..(SQLITE_PARAMETER_CHUNK / 3))
        .map(|index| (format!("new-child-{index}"), index as u32))
        .collect::<Vec<_>>();
    replacement.push(("reject-child".to_owned(), 99));

    assert!(db.save_distributed_children(&replacement).await.is_err());
    assert_eq!(db.load_distributed_children().await.unwrap(), original);
}

#[tokio::test]
async fn distributed_state_replacement_rolls_back_tree_and_children_together() {
    let db = DatabaseManager::in_memory().await.unwrap();
    let original_children = vec![("old-child".to_owned(), 4)];
    db.save_distributed_state(7, "old-root", Some("old-parent"), &original_children)
        .await
        .unwrap();
    db.execute_raw_for_test(
        r#"
        CREATE TRIGGER reject_distributed_child_state
        BEFORE INSERT ON distributed_children
        WHEN NEW.username = 'reject-child'
        BEGIN
            SELECT RAISE(ABORT, 'forced distributed child failure');
        END
        "#,
    )
    .await
    .unwrap();

    let replacement_children = vec![("reject-child".to_owned(), 9)];
    assert!(db
        .save_distributed_state(11, "new-root", Some("new-parent"), &replacement_children)
        .await
        .is_err());
    let (state, children) = db.load_distributed_state().await.unwrap();
    assert_eq!(
        state,
        Some((7, "old-root".to_owned(), Some("old-parent".to_owned())))
    );
    assert_eq!(children, original_children);
}

#[tokio::test]
async fn test_wishlist_scheduler_state_operations() {
    let db = DatabaseManager::in_memory().await.unwrap();

    // Initially no state
    let state = db.load_wishlist_scheduler_state().await.unwrap();
    assert!(state.is_none());

    // Save state
    db.save_wishlist_scheduler_state(5, Some(300))
        .await
        .unwrap();

    // Load state
    let state = db.load_wishlist_scheduler_state().await.unwrap();
    assert!(state.is_some());
    let (next_index, server_interval) = state.unwrap();
    assert_eq!(next_index, 5);
    assert_eq!(server_interval, Some(300));

    // Update state
    db.save_wishlist_scheduler_state(10, None).await.unwrap();
    let state = db.load_wishlist_scheduler_state().await.unwrap();
    assert!(state.is_some());
    let (next_index, server_interval) = state.unwrap();
    assert_eq!(next_index, 10);
    assert_eq!(server_interval, None);
}

#[tokio::test]
async fn webhook_reconciliation_is_indexed_idempotent_and_preserves_terminal_outcomes() {
    let directory =
        std::env::temp_dir().join(format!("slskr-webhook-recovery-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("state.db");
    let db = DatabaseManager::new(path.to_str().unwrap()).await.unwrap();
    db.insert_webhook(&WebhookRecord {
        id: "recovery-hook".to_owned(),
        url: "https://example.invalid/hook".to_owned(),
        events: "search.created".to_owned(),
        secret: "fixture-secret".to_owned(),
        active: true,
        created_at: 1,
        last_triggered: None,
        retry_count: 0,
        max_retries: 3,
        timeout_seconds: 30,
    })
    .await
    .unwrap();
    for status in ["queued", "success", "failed"] {
        db.insert_webhook_log(&WebhookLogRecord {
            id: status.to_owned(),
            webhook_id: "recovery-hook".to_owned(),
            event: "search.created".to_owned(),
            correlation_id: status.to_owned(),
            status: status.to_owned(),
            request_body: "{}".to_owned(),
            response_status: Some(200),
            response_body: Some("preserved".to_owned()),
            error_message: Some("original".to_owned()),
            attempt: 2,
            timestamp: 11,
        })
        .await
        .unwrap();
    }
    db.pool.close().await;
    drop(db);
    let db = DatabaseManager::new(path.to_str().unwrap()).await.unwrap();
    let plan = query(
        "EXPLAIN QUERY PLAN UPDATE webhook_logs SET status = 'failed' WHERE status = 'queued'",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert!(plan.iter().any(|row| row
        .try_get::<String, _>("detail")
        .unwrap()
        .contains("idx_webhook_logs_queued")));
    assert_eq!(
        db.fail_unconfirmed_webhook_logs("outcome unknown after restart")
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        db.fail_unconfirmed_webhook_logs("second recovery")
            .await
            .unwrap(),
        0
    );
    let logs = db.get_webhook_logs("recovery-hook", 10, 0).await.unwrap();
    for log in logs {
        assert_eq!(log.attempt, 2);
        assert_eq!(log.timestamp, 11);
        assert_eq!(log.response_body.as_deref(), Some("preserved"));
        if log.id == "queued" {
            assert_eq!(log.status, "failed");
            assert_eq!(
                log.error_message.as_deref(),
                Some("outcome unknown after restart")
            );
        } else {
            assert_eq!(log.status, log.id);
            assert_eq!(log.error_message.as_deref(), Some("original"));
        }
    }
    db.pool.close().await;
    drop(db);
    std::fs::remove_dir_all(directory).unwrap();
}
