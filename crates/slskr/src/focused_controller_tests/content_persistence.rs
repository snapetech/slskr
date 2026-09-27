use super::fixtures::*;

#[tokio::test]
async fn wishlist_completion_releases_guard_when_item_is_missing() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let item_id = {
        let mut wishlist = state.wishlist.write().await;
        wishlist
            .add_item("Artist".to_owned(), "Track".to_owned(), "Audio".to_owned())
            .expect("wishlist item")
            .id
    };
    let token = {
        let mut searches = state.searches.write().await;
        searches
            .create_scheduled_wishlist_for_item("one two".to_owned(), Some(item_id.clone()), 300)
            .expect("wishlist search")
            .record
            .token
    };
    assert!(state.wishlist.write().await.remove_item(&item_id).is_some());

    let response = tokio::time::timeout(
        Duration::from_secs(1),
        crate::route_http_request(
            "POST",
            &format!("/api/searches/{token}/complete"),
            None,
            "",
            &state,
        ),
    )
    .await
    .expect("wishlist completion must not wait on its own write guard")
    .expect("wishlist completion response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"status\":\"completed\""));
}

#[tokio::test]
async fn hash_db_writers_wait_before_mutating_and_commit_in_memory_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create HashDb ordering database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "slskdn")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = crate::hash_db_persistence_turn().await;
    let store_state = Arc::clone(&state);
    let mut store = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/hashdb/hash",
            None,
            r#"{"filename":"Ordered/Stored.flac","size":4096,"byteHash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","sampleRate":44100,"channels":2,"bitDepth":16}"#,
            &store_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut store)
        .await
        .is_err());

    let merge_state = Arc::clone(&state);
    let merge_body = serde_json::json!({
        "entries": [{
            "flacKey": "ordered-merge-key",
            "byteHash": "b".repeat(64),
            "size": 4097,
        }]
    })
    .to_string();
    let mut merge = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/hashdb/sync/merge",
            None,
            &merge_body,
            &merge_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut merge)
        .await
        .is_err());

    assert!(state
        .content_discovery
        .read()
        .await
        .hash_entries()
        .is_empty());
    let readable =
        crate::route_http_request("GET", "/api/v0/hashdb/hash/by-size/4096", None, "", &state)
            .await
            .expect("HashDb reads stay responsive while mutations wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["count"],
        0
    );
    assert!(db
        .list_hash_db_entries()
        .await
        .expect("read empty HashDb before release")
        .is_empty());

    drop(persistence_turn);
    assert_eq!(
        store
            .await
            .expect("HashDb store completes")
            .expect("store HashDb entry")
            .status,
        "200 OK"
    );
    assert_eq!(
        merge
            .await
            .expect("HashDb merge completes")
            .expect("merge HashDb entry")
            .status,
        "200 OK"
    );

    let memory = state.content_discovery.read().await;
    assert_eq!(memory.hash_entries().len(), 2);
    let latest_seq = memory.latest_seq();
    drop(memory);
    let persisted = db
        .list_hash_db_entries()
        .await
        .expect("read final HashDb rows");
    assert_eq!(persisted.len(), 2);
    let persisted_latest_seq = db
        .get_hash_db_state("latest_seq")
        .await
        .expect("read final HashDb sequence")
        .expect("persisted HashDb sequence")
        .value
        .expect("sequence value")
        .parse::<u64>()
        .expect("numeric sequence");
    assert_eq!(persisted_latest_seq, latest_seq);

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn hash_db_history_backfill_waits_before_advancing_persisted_progress() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create HashDb history backfill database");
    let (state, _receiver) = test_state_with_db(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "slskdn")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    {
        let mut searches = state.searches.write().await;
        for index in 1..=11_u64 {
            searches.records.push(crate::SearchRecord {
                id: format!("history-{index}"),
                token: u32::try_from(index).unwrap(),
                query: format!("history {index}"),
                target: "global",
                target_name: None,
                status: "completed",
                results: Vec::new(),
                raw_response_count: 1,
                filtered_out_count: 0,
                ignored_result_count: 0,
                hidden_locked_count: 0,
                fallback_attempts: 0,
                ttl_seconds: crate::DEFAULT_SEARCH_TTL_SECONDS,
                expires_at: 0,
                created_at: index,
                updated_at: index,
            });
        }
    }

    let persistence_turn = crate::hash_db_persistence_turn().await;
    let route_state = Arc::clone(&state);
    let mut first_backfill = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/hashdb/backfill/from-history?batchSize=10",
            None,
            "",
            &route_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut first_backfill)
            .await
            .is_err()
    );
    let route_state = Arc::clone(&state);
    let mut second_backfill = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/v0/hashdb/backfill/from-history?batchSize=10",
            None,
            "",
            &route_state,
        )
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut second_backfill)
            .await
            .is_err()
    );

    assert!(state
        .controller_features
        .read()
        .await
        .get("hashdb/backfill/progress")
        .is_none());
    assert!(db
        .get_hash_db_state("hashdb/backfill/progress")
        .await
        .expect("read queued backfill cursor")
        .is_none());
    let readable = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/backfill/candidates?limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("HashDb reads stay responsive during queued backfill");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["count"],
        0
    );

    drop(persistence_turn);
    let first_response = first_backfill
        .await
        .expect("first history backfill completes")
        .expect("first history backfill request");
    assert_eq!(first_response.status, "200 OK");
    let first_json = serde_json::from_str::<serde_json::Value>(&first_response.body).unwrap();
    assert_eq!(first_json["searchesProcessed"], 10);
    assert_eq!(first_json["remainingSearches"], 1);

    let second_response = second_backfill
        .await
        .expect("second history backfill completes")
        .expect("second history backfill request");
    assert_eq!(second_response.status, "200 OK");
    let second_json = serde_json::from_str::<serde_json::Value>(&second_response.body).unwrap();
    assert_eq!(second_json["searchesProcessed"], 1);
    assert_eq!(second_json["remainingSearches"], 0);
    assert_eq!(second_json["complete"], true);
    let persisted = db
        .get_hash_db_state("hashdb/backfill/progress")
        .await
        .expect("read persisted backfill cursor")
        .expect("backfill cursor was stored");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            persisted.value.as_deref().expect("backfill cursor value")
        )
        .unwrap()["lastProcessedAt"],
        1
    );

    db.close_for_test().await;
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
    crate::route_dispatch::rollback_library_runtime_if_unchanged(
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
    crate::route_dispatch::rollback_library_runtime_if_unchanged(
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

    let root_response = crate::route_http_request(
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

    let folder_response = crate::route_http_request(
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

    let album_response = crate::route_http_request(
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
    assert!(crate::open_primary_stream_file(&state, content_id, None)
        .await
        .expect("open sha256 stream")
        .is_some());

    let encoded_content_id = content_id.replace(':', "%3A");
    let stream_status = crate::route_http_request(
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

    let ticket = crate::route_http_request(
        "POST",
        &format!("/api/v0/streams/{encoded_content_id}/ticket"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("sha256 stream ticket");
    assert_eq!(ticket.status, "200 OK", "{}", ticket.body);

    let invalid_path = crate::route_http_request(
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
    let db = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create wishlist restart database");
    db.upsert_wishlist_item(&crate::persistence::WishlistItemRecord {
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
        &crate::persistence::WishlistIgnoredResultRecord {
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

    let reopened = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("reopen wishlist restart database");
    let wishlist = crate::load_wishlist_store(Some(&reopened))
        .await
        .expect("load wishlist state through the application startup path");
    reopened.close_for_test().await;

    let (state, mut session_commands) = test_state_with_env(MapEnv::default());
    *state.wishlist.write().await = wishlist;
    let ignored = crate::route_http_request(
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
        crate::route_http_request("POST", "/api/wishlist/wish-17/search", None, "", &state)
            .await
            .expect("start search for rehydrated wishlist item");
    assert_eq!(started.status, "202 Accepted");
    let token = serde_json::from_str::<serde_json::Value>(&started.body).unwrap()["token"]
        .as_u64()
        .expect("search token");
    assert!(matches!(
        session_commands.try_recv().expect("wishlist search command"),
        crate::SessionCommand::Search {
            token: command_token,
            target: crate::SearchDispatchTarget::Wishlist,
            ..
        } if u64::from(command_token) == token
    ));

    for filename in ["Remote/Album/Blocked.flac", "Remote/Other/Allowed.flac"] {
        crate::route_http_request(
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
async fn wishlist_ignore_and_search_response_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
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
    let started = crate::route_http_request(
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
        crate::route_http_request(
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
        crate::route_http_request(
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
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist snapshot-order database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
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
    let started = crate::route_http_request(
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
    crate::route_http_request(
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
        crate::route_http_request(
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
        crate::persist_search_record(&snapshot_state, &stale_snapshot).await
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
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create wishlist item ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
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
        crate::route_http_request(
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
        crate::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable =
        crate::route_http_request("GET", &format!("/api/wishlist/{item_id}"), None, "", &state)
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
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create library item ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
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
        crate::route_http_request(
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
        crate::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = crate::route_http_request(
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
