#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_event_open_rejects_fifo_without_blocking() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-events-fifo-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let events_path = super::transfer_events_path(&state_dir);
    let status = std::process::Command::new("mkfifo")
        .arg(&events_path)
        .status()
        .expect("run mkfifo");
    assert!(status.success());

    let error = super::open_transfer_event_file(&events_path)
        .expect_err("FIFO transfer event path must be rejected");
    assert!(error.contains("transfer event open failed"));

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_event_append_rotates_oversized_event_file() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-events-size-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let events_path = super::transfer_events_path(&state_dir);
    std::fs::write(
        &events_path,
        vec![b' '; (super::MAX_TRANSFER_EVENTS_BYTES as usize) + 1],
    )
    .expect("oversized transfer events");

    let entry = super::TransferEntry {
        id: 7,
        direction: 0,
        token: 9,
        peer_username: Some("friend".to_owned()),
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
        size: Some(100),
        bytes_transferred: 25,
        status: "in_progress".to_owned(),
        reason: None,
        requested_at: 11,
        started_at: Some(11),
        start_offset: 0,
        updated_at: 12,
        updated_at_ms: 12_000,
        previous_status: None,
    };
    super::append_transfer_event(&events_path, &entry).expect("append rotated event");

    let body = std::fs::read_to_string(&events_path).expect("events body");
    assert!(body.starts_with("slskr-transfer-events-v2\n"));
    assert!(body.contains("7\t0\t9\t100\t25\tin_progress\t\tRemote/Song.flac\n"));
    assert!(events_path.with_extension("tsv.old").exists());
    assert!(
        std::fs::metadata(&events_path)
            .expect("events metadata")
            .len()
            < 1024
    );

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn backfill_hash_matches_frozen_flac_header_policy() {
    use sha2::{Digest, Sha256};

    let mut header = vec![0_u8; 42];
    header[..4].copy_from_slice(b"fLaC");
    header[4] = 0;
    header[7] = 34;
    for (index, byte) in header[8..].iter_mut().enumerate() {
        *byte = u8::try_from(index).unwrap();
    }
    assert_eq!(
        super::parse_flac_backfill_hash(&header).expect("valid FLAC header"),
        hex::encode(Sha256::digest(&header))
    );
    header[0] = b'X';
    assert_eq!(
        super::parse_flac_backfill_hash(&header).unwrap_err(),
        "failed to parse FLAC header"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn backfill_daily_peer_limits_survive_restart() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-backfill-state-test-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&state_dir).unwrap();
    let now = super::unix_timestamp();
    let mut state = super::BackfillState::load(&state_dir).unwrap();
    state.record_peer_success("Peer-A", now);
    state.record_peer_success("peer-a", now);
    state.persist().unwrap();

    let mut reloaded = super::BackfillState::load(&state_dir).unwrap();
    assert_eq!(reloaded.peer_count_today("PEER-A", now), 2);
    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn queued_backfill_transfer_reads_only_the_bounded_header() {
    let (state, _receiver) = test_state();
    let token = 77_u32;
    let mut header = vec![0_u8; 42];
    header[..4].copy_from_slice(b"fLaC");
    header[7] = 34;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(address);
    let server = listener.accept();
    let (client, server) = tokio::join!(client, server);
    let (server, _) = server.unwrap();
    let (response, result) = tokio::sync::oneshot::channel();
    state.pending_backfill_transfers.write().await.insert(
        token,
        super::PendingBackfillTransfer {
            expected_size: header.len() as u64,
            response,
        },
    );
    let task_state = Arc::clone(&state);
    let handler = tokio::spawn(async move {
        super::handle_inbound_file_transfer(
            &task_state,
            slskr_client::file_transfer::FileTransferConnection::new(server),
            Some(token),
        )
        .await
    });
    let mut client = slskr_client::file_transfer::FileTransferConnection::new(client.unwrap());
    client.send_token(token).await.unwrap();
    assert_eq!(client.receive_offset().await.unwrap(), 0);
    client.write_chunk(&header).await.unwrap();
    assert_eq!(result.await.unwrap().unwrap(), header);
    handler.await.unwrap().unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn lidarr_scheduler_tracks_the_next_cycle_without_syncing_when_disabled_by_policy() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", "http://127.0.0.1:9")
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_SYNC_WANTED", "false")
            .with("SLSKR_LIDARR_SYNC_INTERVAL", "600"),
    );
    state.lidarr_sync_state.write().await.next_sync_at = None;

    let delay = super::run_lidarr_sync_scheduler_cycle(&state).await;

    assert_eq!(delay, std::time::Duration::from_secs(600));
    let sync = state.lidarr_sync_state.read().await;
    assert!(!sync.is_syncing);
    assert!(sync.last_sync_at.is_none());
    assert!(sync.last_result.is_none());
    assert!(sync.last_error.is_none());
    assert!(sync.next_sync_at.is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn lidarr_scheduler_records_external_failure_and_clears_running_state() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let unavailable = listener.local_addr().unwrap();
    drop(listener);
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", &format!("http://{unavailable}"))
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_TIMEOUT", "1")
            .with("SLSKR_LIDARR_SYNC_WANTED", "true")
            .with("SLSKR_LIDARR_SYNC_INTERVAL", "600"),
    );

    let delay = super::run_lidarr_sync_scheduler_cycle(&state).await;

    assert_eq!(delay, std::time::Duration::from_secs(600));
    let sync = state.lidarr_sync_state.read().await;
    assert!(!sync.is_syncing);
    assert!(sync.last_sync_at.is_none());
    assert!(sync.last_result.is_none());
    assert!(sync
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("Lidarr wanted request failed")));
    assert!(sync.next_sync_at.is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn lidarr_wanted_sync_rolls_back_wishlist_when_persistence_fails() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Lidarr wanted fixture");
    let address = listener.local_addr().expect("Lidarr fixture address");
    let fixture = tokio::spawn(async move {
        serve_json_fixture(
            &listener,
            serde_json::json!({
                "totalRecords": 1,
                "records": [{
                    "title": "Persistence Failure Album",
                    "artist": {"artistName": "Persistence Failure Artist"}
                }]
            }),
        )
        .await
    });
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", &format!("http://{address}"))
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_TIMEOUT", "1"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let previous_wishlist = state.wishlist.read().await.clone();
    let lidarr = state.integration_settings.read().await.lidarr.clone();
    db.close_for_test().await;

    let error = super::sync_lidarr_wanted_to_wishlist(&state, &lidarr)
        .await
        .expect_err("closed database should fail the enabled wanted sync");
    let request = fixture.await.expect("Lidarr wanted fixture task");

    assert!(error.starts_with("wishlist persistence failed:"), "{error}");
    assert!(request.starts_with("GET /api/v1/wanted/missing?page=1&pageSize=250"));
    assert_eq!(*state.wishlist.read().await, previous_wishlist);
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_lidarr_and_source_feed_contracts() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
    let server = tokio::spawn(async move {
        let responses = [
            serde_json::json!({"appName": "Lidarr", "version": "2.1.0"}).to_string(),
            serde_json::json!({
                "totalRecords": 1,
                "records": [{
                    "id": 7,
                    "title": "Fixture Album",
                    "artist": {"id": 8, "artistName": "Fixture Artist"}
                }]
            })
            .to_string(),
            serde_json::json!([{
                "id": 9,
                "path": "/lidarr/Fixture Album/track.flac",
                "artist": {"id": 8, "artistName": "Fixture Artist"},
                "album": {"id": 10, "title": "Fixture Album"},
                "albumReleaseId": 11,
                "tracks": [{"id": 12, "title": "Track"}],
                "quality": {"quality": {"id": 1}},
                "additionalFile": false,
                "rejections": []
            }])
            .to_string(),
            serde_json::json!({"id": 42, "name": "ManualImport", "status": "queued"}).to_string(),
            serde_json::json!([{
                "id": 19,
                "path": "/lidarr/Auto Album/track.flac",
                "artist": {"id": 18, "artistName": "Auto Artist"},
                "album": {"id": 20, "title": "Auto Album"},
                "albumReleaseId": 21,
                "tracks": [{"id": 22, "title": "Track"}],
                "quality": {"quality": {"id": 1}},
                "additionalFile": false,
                "rejections": []
            }])
            .to_string(),
            serde_json::json!({"id": 43, "name": "ManualImport", "status": "queued"}).to_string(),
        ];
        for response in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = vec![0_u8; 65_536];
            let count = stream.read(&mut bytes).await.unwrap();
            request_tx
                .send(String::from_utf8_lossy(&bytes[..count]).to_string())
                .unwrap();
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.len(),
                response
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
        }
    });

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", &format!("http://{address}"))
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_SYNC_WANTED", "true")
            .with("SLSKR_LIDARR_SYNC_MAX_ITEMS", "1")
            .with("SLSKR_LIDARR_AUTO_DOWNLOAD", "true")
            .with("SLSKR_LIDARR_WISHLIST_FILTER", "flac")
            .with("SLSKR_LIDARR_WISHLIST_MAX_RESULTS", "37")
            .with("SLSKR_LIDARR_AUTO_IMPORT_COMPLETED", "true")
            .with("SLSKR_LIDARR_IMPORT_PATH_FROM", "/downloads")
            .with("SLSKR_LIDARR_IMPORT_PATH_TO", "/lidarr")
            .with("SLSKR_LIDARR_IMPORT_MODE", "copy")
            .with("SLSKR_LIDARR_IMPORT_REPLACE_EXISTING", "true"),
    );

    let status = super::route_http_request(
        "GET",
        "/api/v0/integrations/lidarr/status",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(status.status, "200 OK", "{}", status.body);
    assert_eq!(status.content_type, "application/json; charset=utf-8");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&status.body).unwrap(),
        serde_json::json!({"appName": "Lidarr", "version": "2.1.0"})
    );

    let sync = super::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/wanted/sync",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(sync.status, "200 OK", "{}", sync.body);
    assert_eq!(sync.content_type, "application/json; charset=utf-8");
    let sync: serde_json::Value = serde_json::from_str(&sync.body).unwrap();
    assert_eq!(sync["wantedCount"], 1);
    assert_eq!(sync["createdCount"], 1);
    let item = state
        .wishlist
        .read()
        .await
        .records
        .iter()
        .flat_map(|record| record.items.iter())
        .next()
        .cloned()
        .unwrap();
    assert_eq!(item.search_text(), "Fixture Artist Fixture Album");
    assert_eq!(item.filter, "flac");
    assert!(item.auto_download);
    assert_eq!(item.max_results, 37);

    let import = super::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/downloads/Fixture Album"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(import.status, "200 OK", "{}", import.body);
    let import: serde_json::Value = serde_json::from_str(&import.body).unwrap();
    assert_eq!(import["directory"], "/lidarr/Fixture Album");
    assert_eq!(import["candidateCount"], 1);
    assert_eq!(import["safeCandidateCount"], 1);
    assert_eq!(import["commandId"], 42);
    assert_eq!(import["importMode"], "Copy");

    let repeated = super::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/downloads/Fixture Album"}"#,
        &state,
    )
    .await
    .unwrap();
    let repeated: serde_json::Value = serde_json::from_str(&repeated.body).unwrap();
    assert_eq!(repeated["skippedReason"], "Recently processed");

    let completed = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Remote/Auto Album/track.flac".to_owned(),
            Some("/downloads/Auto Album/track.flac".to_owned()),
            Some(1),
            None,
            super::TransferRequestDetails::default(),
        );
        transfers
            .update_local_execution(entry.id, "succeeded", 1, Some(1), None)
            .unwrap()
    };
    super::maybe_import_lidarr_completed_download(&state, &completed).await;

    let status_request = request_rx.recv().await.unwrap();
    let wanted_request = request_rx.recv().await.unwrap();
    let candidates_request = request_rx.recv().await.unwrap();
    let command_request = request_rx.recv().await.unwrap();
    assert!(status_request.contains("/api/v1/system/status"));
    assert!(wanted_request
        .to_ascii_lowercase()
        .contains("x-api-key: fixture-key"));
    assert!(wanted_request.contains("page=1&pageSize=250&includeArtist=true&monitored=true"));
    assert!(candidates_request.contains("folder=%2Flidarr%2FFixture%20Album"));
    assert!(candidates_request.contains("replaceExistingFiles=true"));
    assert!(command_request.contains("\"importMode\":\"Copy\""));
    assert!(command_request.contains("\"replaceExistingFiles\":true"));
    assert!(command_request.contains("\"foreignArtistId\":\"\""));
    let automatic_candidates_request = request_rx.recv().await.unwrap();
    let automatic_command_request = request_rx.recv().await.unwrap();
    assert!(automatic_candidates_request.contains("folder=%2Flidarr%2FAuto%20Album"));
    assert!(automatic_command_request.contains("\"name\":\"ManualImport\""));
    server.await.unwrap();
    assert!(!state.runtime.read().await.application_restart_requested);

    let yaml = r#"integrations:
  lidarr:
    enabled: false
    timeout_seconds: 31
    sync_interval_seconds: 600
    max_items_per_sync: 9
    wishlist_max_results: 41
  spotify:
    enabled: true
    client_id: watched-client
    client_secret: watched-secret
    redirect_uri: http://127.0.0.1/spotify-callback
    timeout_seconds: 32
    max_items_per_import: 43
    market: CA
"#;
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    let mut cli_environment = state.controller_cli_environment.clone();
    cli_environment.retain(|name, _| {
        !name.starts_with("SLSKR_LIDARR_") && !name.starts_with("SLSKR_SPOTIFY_")
    });
    super::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;
    let integrations = state.integration_settings.read().await;
    assert!(!integrations.lidarr.enabled);
    assert_eq!(integrations.lidarr.timeout_seconds, 31);
    assert!(integrations.spotify.enabled);
    assert_eq!(
        integrations.spotify.client_id.as_deref(),
        Some("watched-client")
    );
    assert_eq!(integrations.spotify.timeout_seconds, 32);
    assert_eq!(integrations.spotify.max_items_per_import, 43);
    assert_eq!(integrations.spotify.market, "CA");
    drop(integrations);
    assert!(!state.runtime.read().await.application_restart_requested);

    let disabled_import = super::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/downloads/Fixture Album"}"#,
        &state,
    )
    .await
    .unwrap();
    let disabled_import: serde_json::Value = serde_json::from_str(&disabled_import.body).unwrap();
    assert_eq!(disabled_import["enabled"], false);

    let plugins = super::route_http_request("GET", "/api/config/plugins", None, "", &state)
        .await
        .unwrap();
    let plugins: serde_json::Value = serde_json::from_str(&plugins.body).unwrap();
    assert_eq!(plugins["plugins"][0]["enabled"], true);
    assert_eq!(plugins["plugins"][1]["enabled"], false);

    let source_text = (1..=50)
        .map(|index| format!("Artist {index} - Track {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let empty_unversioned_history = super::route_http_request(
        "GET",
        "/api/source-feed-imports/history?limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(empty_unversioned_history.status, "200 OK");
    assert_eq!(
        empty_unversioned_history.content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(empty_unversioned_history.body, "[]");

    let preview = super::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &serde_json::json!({
            "sourceText": source_text,
            "sourceKind": "text",
            "fetchProviderUrls": false,
            "limit": 500,
        })
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    let preview: serde_json::Value = serde_json::from_str(&preview.body).unwrap();
    assert_eq!(preview["totalRows"], 43);
    assert_eq!(preview["suggestionCount"], 43);
    assert_eq!(preview["suggestions"].as_array().unwrap().len(), 43);

    let history = super::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history?limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let history: serde_json::Value = serde_json::from_str(&history.body).unwrap();
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["limit"], 500);
    assert_eq!(history[0]["totalRows"], 43);
    assert_eq!(history[0]["suggestions"].as_array().unwrap().len(), 25);
    assert_eq!(history[0]["sourceFingerprint"].as_str().unwrap().len(), 64);
    let import_id = history[0]["importId"].as_str().unwrap();
    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/source-feed-imports/history/{import_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&detail.body).unwrap(),
        history[0]
    );
    let missing_unversioned_detail = super::route_http_request(
        "GET",
        "/api/source-feed-imports/history/does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_unversioned_detail.status, "404 Not Found");
    let unversioned_history = super::route_http_request(
        "GET",
        "/api/source-feed-imports/history?limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unversioned_history.status, "200 OK");
    assert_eq!(
        unversioned_history.content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(unversioned_history.body, history.to_string());
    let unversioned_detail = super::route_http_request(
        "GET",
        &format!("/api/source-feed-imports/history/{import_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unversioned_detail.status, "200 OK");
    assert_eq!(
        unversioned_detail.content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unversioned_detail.body).unwrap(),
        history[0]
    );
    let reloaded = super::SourceFeedImportHistoryStore::load(&state.config.state_dir);
    assert_eq!(reloaded.history.len(), 1);
    assert_eq!(reloaded.history[0]["importId"], import_id);

    for (method, route, case) in [
        (
            "GET",
            "/api/v0/integrations/lidarr/status",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/wanted/sync",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/wanted/sync",
            "mutation-side-effects-and-readback",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "mutation-side-effects-and-readback",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "concurrency-and-idempotency",
        ),
        (
            "GET",
            "/api/v0/source-feed-imports/history",
            "populated-dynamic-state",
        ),
        (
            "GET",
            "/api/v0/source-feed-imports/history/{importId}",
            "populated-dynamic-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history",
            "missing-empty-or-conflict-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/source-feed-imports/history",
            "populated-dynamic-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history/{importId}",
            "missing-empty-or-conflict-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history/{importId}",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/source-feed-imports/history/{importId}",
            "populated-dynamic-state",
        ),
    ] {
        record_evidence!(method, route, case);
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("lidarr_and_source_feed_contracts.json"),
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_source_feed_open_cases() {
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

    let env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env.clone());

    for (path, route) in [
        (
            "/api/source-feed-imports/history/extra/segment",
            "/api/source-feed-imports/history",
        ),
        (
            "/api/source-feed-imports/history/open-id/extra",
            "/api/source-feed-imports/history/{importId}",
        ),
        (
            "/api/v0/source-feed-imports/history/extra/segment",
            "/api/v0/source-feed-imports/history",
        ),
        (
            "/api/v0/source-feed-imports/history/open-id/extra",
            "/api/v0/source-feed-imports/history/{importId}",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("malformed source-feed history path");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let empty_versioned_history = super::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &state,
    )
    .await
    .expect("empty versioned source-feed history");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history",
        "missing-empty-or-conflict-state",
        empty_versioned_history.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&empty_versioned_history.body)
                .is_ok_and(|value| value == serde_json::json!([]))
    );
    let missing_versioned_detail = super::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history/missing-open-id",
        None,
        "",
        &state,
    )
    .await
    .expect("missing versioned source-feed detail");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history/{importId}",
        "missing-empty-or-conflict-state",
        missing_versioned_detail.status == "404 Not Found"
    );

    let preview_body = serde_json::json!({
        "sourceText": "Open Artist - Open Track",
        "sourceKind": "text",
        "fetchProviderUrls": false,
        "limit": 10,
    })
    .to_string();
    let preview = super::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &preview_body,
        &state,
    )
    .await
    .expect("versioned source-feed preview");
    let preview_json = serde_json::from_str::<serde_json::Value>(&preview.body).unwrap_or_default();
    let history = super::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &state,
    )
    .await
    .expect("populated versioned source-feed history");
    let history_json = serde_json::from_str::<serde_json::Value>(&history.body).unwrap_or_default();
    let import_id = history_json[0]["importId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/source-feed-imports/history/{import_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("versioned source-feed detail");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history/{importId}",
        "nominal-status-headers-body",
        detail.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&detail.body)
                .is_ok_and(|value| value["importId"] == import_id)
    );

    let unversioned_preview = super::route_http_request(
        "POST",
        "/api/source-feed-imports/preview",
        None,
        r#"{"sourceText":"Unversioned Artist - Unversioned Track"}"#,
        &state,
    )
    .await
    .expect("unversioned source-feed preview");
    let unversioned_preview_json =
        serde_json::from_str::<serde_json::Value>(&unversioned_preview.body).unwrap_or_default();
    let unversioned_history_after = super::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned history after unversioned source-feed rejection");
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "mutation-side-effects-and-readback",
        unversioned_preview.status == "400 Bad Request"
            && unversioned_preview_json["code"] == "ApiVersionUnspecified"
            && serde_json::from_str::<serde_json::Value>(&unversioned_history_after.body)
                .is_ok_and(|value| value.as_array().is_some_and(|rows| rows.len() == 1))
    );
    let unversioned_restart = test_state_with_env(env.clone()).0;
    let unversioned_restart_history = super::route_http_request(
        "GET",
        "/api/source-feed-imports/history",
        None,
        "",
        &unversioned_restart,
    )
    .await
    .expect("unversioned source-feed restart");
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "restart-persistence-or-reset",
        unversioned_preview.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&unversioned_restart_history.body)
                .is_ok_and(|value| value == serde_json::json!([]))
    );
    let unversioned_concurrent_bodies = [
        r#"{"sourceText":"Concurrent Artist A - Track A"}"#,
        r#"{"sourceText":"Concurrent Artist B - Track B"}"#,
    ];
    let unversioned_concurrent =
        futures_util::future::join_all(unversioned_concurrent_bodies.into_iter().map(|body| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "POST",
                    "/api/source-feed-imports/preview",
                    None,
                    body,
                    &state,
                )
                .await
            }
        }))
        .await;
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "concurrency-and-idempotency",
        unversioned_concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "400 Bad Request")
        })
    );

    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "mutation-side-effects-and-readback",
        preview.status == "200 OK"
            && preview_json["suggestionCount"] == 1
            && history_json.as_array().is_some_and(|rows| rows.len() == 1)
            && !import_id.is_empty()
    );
    let reloaded_history = super::SourceFeedImportHistoryStore::load(&state.config.state_dir);
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "restart-persistence-or-reset",
        reloaded_history
            .history
            .iter()
            .any(|entry| { entry["importId"].as_str() == Some(import_id.as_str()) })
    );
    let versioned_concurrent_bodies: Vec<String> = (0..2)
        .map(|index| {
            serde_json::json!({
                "sourceText": format!("Versioned Concurrent {index} - Track {index}"),
                "sourceKind": "text",
                "fetchProviderUrls": false,
            })
            .to_string()
        })
        .collect();
    let versioned_concurrent =
        futures_util::future::join_all(versioned_concurrent_bodies.iter().map(|body| {
            let body = body.clone();
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "POST",
                    "/api/v0/source-feed-imports/preview",
                    None,
                    &body,
                    &state,
                )
                .await
            }
        }))
        .await;
    let reloaded_after_concurrent =
        super::SourceFeedImportHistoryStore::load(&state.config.state_dir);
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "concurrency-and-idempotency",
        versioned_concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && reloaded_after_concurrent.history.len() >= 3
    );

    let runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("source-feed runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        env.clone().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    let runtime_unversioned = super::route_http_request(
        "POST",
        "/api/source-feed-imports/preview",
        None,
        r#"{"sourceText":"Runtime Artist - Runtime Track"}"#,
        &runtime_state,
    )
    .await
    .expect("runtime unversioned source-feed preview");
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "runtime-failure-and-timeout",
        runtime_unversioned.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&runtime_unversioned.body)
                .is_ok_and(|value| value["code"] == "ApiVersionUnspecified")
    );
    let runtime_history = super::route_http_request(
        "GET",
        "/api/source-feed-imports/history",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime unversioned source-feed history");
    let runtime_detail = super::route_http_request(
        "GET",
        "/api/source-feed-imports/history/runtime-missing",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime unversioned source-feed detail");
    record!(
        "GET",
        "/api/source-feed-imports/history",
        "runtime-failure-and-timeout",
        runtime_history.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_history.body)
                .is_ok_and(|value| value.is_array())
    );
    record!(
        "GET",
        "/api/source-feed-imports/history/{importId}",
        "runtime-failure-and-timeout",
        runtime_detail.status == "404 Not Found"
    );
    let runtime_versioned_history = super::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime versioned source-feed history");
    let runtime_versioned_detail = super::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history/runtime-missing",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime versioned source-feed detail");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history",
        "runtime-failure-and-timeout",
        runtime_versioned_history.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_versioned_history.body)
                .is_ok_and(|value| value.is_array())
    );
    record!(
        "GET",
        "/api/v0/source-feed-imports/history/{importId}",
        "runtime-failure-and-timeout",
        runtime_versioned_detail.status == "404 Not Found"
    );

    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-source-feed-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file")
        .expect("create source-feed state conflict");
    let mut conflict_state = test_state_with_env(env).0;
    Arc::get_mut(&mut conflict_state)
        .expect("exclusive source-feed conflict state")
        .config
        .state_dir = conflict_root.clone();
    let runtime_versioned_preview = super::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &preview_body,
        &conflict_state,
    )
    .await
    .expect("runtime versioned source-feed preview");
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "malformed-path-query-or-body",
        super::route_http_request(
            "POST",
            "/api/v0/source-feed-imports/preview/extra",
            None,
            &preview_body,
            &state,
        )
        .await
        .is_ok_and(|response| response.status == "404 Not Found")
    );
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "missing-empty-or-conflict-state",
        super::route_http_request(
            "POST",
            "/api/v0/source-feed-imports/preview",
            None,
            "",
            &state,
        )
        .await
        .is_ok_and(|response| response.status == "400 Bad Request")
    );
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "runtime-failure-and-timeout",
        runtime_versioned_preview.status == "503 Service Unavailable"
    );
    let _ = fs::remove_file(conflict_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create source-feed evidence directory");
    fs::write(
        evidence_dir.join("source_feed_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize source-feed ledger"),
    )
    .expect("write source-feed ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api source-feed-imports mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn distributed_runtime_propagates_branch_depth_capacity_and_disable() {
    let (state, mut session_commands) = test_state();
    let (parent, mut parent_messages) = mpsc::channel(8);
    let (alpha, mut alpha_messages) = mpsc::channel(8);
    let (beta, mut beta_messages) = mpsc::channel(8);
    {
        let mut runtime = state.distributed_network.write().await;
        runtime.parent = Some("parent".to_owned());
        runtime.parent_sender = Some(parent);
        runtime.branch_level = 4;
        runtime.branch_root = "root".to_owned();
        runtime.children.insert("alpha".to_owned(), alpha);
        runtime.children.insert("beta".to_owned(), beta);
        runtime.child_depths.insert("alpha".to_owned(), 2);
        runtime.child_depths.insert("beta".to_owned(), 5);
    }

    super::notify_distributed_branch(&state).await;
    for receiver in [&mut alpha_messages, &mut beta_messages] {
        assert_eq!(
            receiver.recv().await,
            Some(super::DistributedMessage::BranchLevel { level: 4 })
        );
        assert_eq!(
            receiver.recv().await,
            Some(super::DistributedMessage::BranchRoot {
                username: "root".to_owned(),
            })
        );
    }
    assert!(matches!(
        session_commands.recv().await,
        Some(super::SessionCommand::DistributedBranch {
            has_parent: true,
            accept_children: true,
            level: 4,
            root,
        }) if root == "root"
    ));

    super::handle_distributed_message(
        &state,
        "alpha",
        super::DistributedConnectionRole::Child,
        super::DistributedMessage::Ping,
    )
    .await;
    assert!(matches!(
        alpha_messages.recv().await,
        Some(super::DistributedMessage::PingResponse { token }) if token > 0
    ));

    super::notify_distributed_child_depth(&state).await;
    assert_eq!(
        parent_messages.recv().await,
        Some(super::DistributedMessage::ChildDepth { depth: 6 })
    );

    super::apply_distributed_settings(
        &state,
        super::config::SoulseekDistributedSettings {
            disabled: false,
            disable_children: false,
            child_limit: 1,
            logging: true,
        },
    )
    .await;
    {
        let runtime = state.distributed_network.read().await;
        assert_eq!(
            runtime.children.keys().cloned().collect::<Vec<_>>(),
            ["alpha"]
        );
        assert!(
            runtime.json(*state.soulseek_distributed_settings.read().await)["canAcceptChildren"]
                == false
        );
    }

    super::apply_distributed_settings(
        &state,
        super::config::SoulseekDistributedSettings {
            disabled: true,
            disable_children: false,
            child_limit: 1,
            logging: false,
        },
    )
    .await;
    let runtime = state.distributed_network.read().await;
    assert!(runtime.parent.is_none());
    assert!(runtime.parent_sender.is_none());
    assert!(runtime.children.is_empty());
    assert_eq!(runtime.branch_level, 0);
    assert_eq!(runtime.branch_root, runtime.local_username);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn embedded_distributed_search_is_unwrapped_and_broadcast_to_children() {
    let (state, _session_commands) = test_state();
    let (child_sender, mut child_messages) = mpsc::channel(4);
    {
        let mut runtime = state.distributed_network.write().await;
        runtime.parent = Some("parent".to_owned());
        runtime.children.insert("child".to_owned(), child_sender);
    }

    let search = super::DistributedSearch {
        identifier: 0,
        username: "requester".to_owned(),
        token: 77,
        query: "embedded-search-that-is-not-shared".to_owned(),
    };
    let search_frame = super::DistributedMessage::Search(search.clone())
        .encode()
        .expect("encode distributed search");

    super::handle_embedded_distributed_search(
        &state,
        Some(super::DistributedConnectionRole::Parent),
        Some("parent"),
        search_frame.code,
        &search_frame.payload,
    )
    .await;

    assert_eq!(
        child_messages.recv().await,
        Some(super::DistributedMessage::Search(search))
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn user_info_picture_is_read_at_response_time() {
    let (state, _session_commands) = test_state();
    let picture = state.config.state_dir.join("profile-picture.bin");
    std::fs::write(&picture, [0_u8, 1, 2, 255]).unwrap();
    *state
        .user_info_picture
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(picture.clone());

    assert_eq!(
        super::effective_user_info_picture(&state).await,
        Some(vec![0, 1, 2, 255])
    );
    std::fs::write(&picture, [9_u8, 8, 7]).unwrap();
    assert_eq!(
        super::effective_user_info_picture(&state).await,
        Some(vec![9, 8, 7])
    );
    std::fs::remove_file(picture).unwrap();
    assert_eq!(super::effective_user_info_picture(&state).await, None);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn soulseek_profile_and_distributed_yaml_validation_matches_target_profiles() {
    let cases = [
        (
            "soulseek:\n  diagnostic_level: trace\n",
            None,
            None,
        ),
        (
            "soulseek:\n  diagnostic_level: verbose\n",
            Some("Invalid configuration:\n  Soulseek:\n    The DiagnosticLevel field must be one of: None, Warning, Info, Debug, Trace. Case insensitive."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  picture: /tmp/slskr-picture-that-does-not-exist\n",
            Some("Invalid configuration:\n  Soulseek:\n    The Picture field specifies a non-existent file '/tmp/slskr-picture-that-does-not-exist'."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  distributed_network:\n    child_limit: 0\n",
            Some("Invalid configuration:\n  Soulseek:\n    DistributedNetwork:\n      The field ChildLimit must be between 1 and 2147483647."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  distributed_network:\n    child_limit: 2147483648\n",
            Some("Exception during deserialization: Arithmetic operation resulted in an overflow."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  distributed_network:\n    disabled: nope\n",
            Some("Exception during deserialization: The value \"nope\" is not a valid YAML Boolean"),
            Some("Invalid YAML configuration"),
        ),
    ];
    for (yaml, slskd, slskdn) in cases {
        let value = super::parse_controller_yaml(yaml).unwrap();
        assert_eq!(
            super::controller_yaml_target_validation_error(
                &value,
                super::ControllerProfile::Legacy,
            )
            .as_deref(),
            slskd,
            "slskd validation mismatch for {yaml:?}"
        );
        assert_eq!(
            super::controller_yaml_target_validation_error(
                &value,
                super::ControllerProfile::Native,
            )
            .as_deref(),
            slskdn,
            "slskdN validation mismatch for {yaml:?}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn distributed_child_socket_exchanges_branch_and_depth_then_closes_on_disable() {
    let (state, _session_commands) = test_state();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let connect = tokio::net::TcpStream::connect(address);
    let accept = listener.accept();
    let (client, accepted) = tokio::join!(connect, accept);
    let client = client.unwrap();
    let (server, _) = accepted.unwrap();
    super::register_distributed_child(Arc::clone(&state), "child".to_owned(), server, false)
        .await
        .unwrap();
    let mut peer = slskr_client::stream::DistributedConnection::new(client);
    assert_eq!(
        peer.receive().await.unwrap(),
        super::DistributedMessage::BranchLevel { level: 0 }
    );
    assert_eq!(
        peer.receive().await.unwrap(),
        super::DistributedMessage::BranchRoot {
            username: "tester".to_owned(),
        }
    );
    peer.send(&super::DistributedMessage::ChildDepth { depth: 3 })
        .await
        .unwrap();
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
    .unwrap();

    super::apply_distributed_settings(
        &state,
        super::config::SoulseekDistributedSettings {
            disabled: true,
            disable_children: false,
            child_limit: 25,
            logging: false,
        },
    )
    .await;
    assert!(tokio::time::timeout(Duration::from_secs(1), peer.receive())
        .await
        .unwrap()
        .is_err());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn soulseek_diagnostic_and_distributed_logging_filters_are_runtime_consumers() {
    assert!(!super::event_runtime::soulseek_diagnostic_level_allows(
        super::config::SoulseekDiagnosticLevel::None,
        super::logging::LogLevel::Error,
    ));
    assert!(!super::event_runtime::soulseek_diagnostic_level_allows(
        super::config::SoulseekDiagnosticLevel::Warning,
        super::logging::LogLevel::Info,
    ));
    assert!(super::event_runtime::soulseek_diagnostic_level_allows(
        super::config::SoulseekDiagnosticLevel::Warning,
        super::logging::LogLevel::Warn,
    ));
    assert!(super::event_runtime::soulseek_diagnostic_level_allows(
        super::config::SoulseekDiagnosticLevel::Trace,
        super::logging::LogLevel::Trace,
    ));

    let (state, _session_commands) = test_state();
    let before = state.events.read().await.records.len();
    super::record_soulseek_diagnostic(
        &state,
        super::logging::LogLevel::Info,
        "distributed",
        "hidden",
    )
    .await;
    assert_eq!(state.events.read().await.records.len(), before);
    state.soulseek_distributed_settings.write().await.logging = true;
    super::record_soulseek_diagnostic(
        &state,
        super::logging::LogLevel::Info,
        "distributed",
        "visible",
    )
    .await;
    assert_eq!(state.events.read().await.records.len(), before + 1);
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_file_transfer_and_room_contracts() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "peer=127.0.0.1:2234"),
    );
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($method:expr, $route:expr, $case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": true,
            }));
        };
    }

    let downloads_root = state.config.downloads_dir.clone();
    let incomplete_root = state.config.incomplete_dir.clone();
    std::fs::create_dir_all(downloads_root.join("Artist/Album")).unwrap();
    std::fs::write(downloads_root.join("Artist/Album/Track.flac"), b"track").unwrap();
    std::fs::create_dir_all(incomplete_root.join("Partial")).unwrap();
    std::fs::write(incomplete_root.join("Partial/Track.part"), b"partial").unwrap();

    for (path, route) in [
        (
            "/api/v0/files/downloads/directories?recursive=true",
            "/api/v0/files/downloads/directories",
        ),
        (
            "/api/v0/files/incomplete/directories?recursive=true",
            "/api/v0/files/incomplete/directories",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        assert_eq!(response.status, "200 OK", "GET {path}");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert!(json["directories"].is_array());
        assert!(json["files"].is_array());
        record_evidence!("GET", route, "nominal-status-headers-body");
        record_evidence!("GET", route, "populated-dynamic-state");
    }

    let nested = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories/QXJ0aXN0L0FsYnVt",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd nested directory listing");
    assert_eq!(nested.status, "200 OK");
    let nested_json = serde_json::from_str::<serde_json::Value>(&nested.body).unwrap();
    assert_eq!(nested_json["files"][0]["name"], "Track.flac");
    assert_eq!(nested_json["files"][0]["length"], 5);
    record_evidence!(
        "GET",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "populated-dynamic-state"
    );

    let missing_directory = super::route_http_request(
        "GET",
        "/api/v0/files/incomplete/directories/TWlzc2luZw==",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd directory");
    assert_eq!(missing_directory.status, "404 Not Found");
    record_evidence!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "missing-empty-or-conflict-state"
    );

    let file_cases = [
        (
            "downloads",
            "files",
            "Remote/Delete.mp3",
            downloads_root.join("Remote/Delete.mp3"),
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RemoveDownload",
            downloads_root.join("RemoveDownload/file.bin"),
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "Partial/Remove.part",
            incomplete_root.join("Partial/Remove.part"),
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RemovePartial",
            incomplete_root.join("RemovePartial/file.part"),
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ];
    for (storage, resource, relative, file_path, route) in file_cases {
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, b"delete me").unwrap();
        let response = super::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                super::STANDARD.encode(relative)
            ),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE {storage}/{resource}/{relative}: {error}"));
        assert_eq!(response.status, "204 No Content");
        assert!(
            !file_path.exists(),
            "deleted path remains: {}",
            file_path.display()
        );
        record_evidence!("DELETE", route, "nominal-status-headers-body");
        record_evidence!("DELETE", route, "mutation-side-effects-and-readback");
        let repeated = super::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                super::STANDARD.encode(relative)
            ),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("repeat DELETE {storage}/{resource}/{relative}: {error}"));
        record_evidence!("DELETE", route, "concurrency-and-idempotency");
        assert_eq!(
            repeated.status,
            if resource == "files" {
                "204 No Content"
            } else {
                "404 Not Found"
            }
        );
    }

    let missing_file = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/files/downloads/files/{}",
            super::STANDARD.encode("Missing.mp3")
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd file");
    assert_eq!(missing_file.status, "204 No Content");
    record_evidence!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "missing-empty-or-conflict-state"
    );

    let traversal = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/files/downloads/files/{}",
            super::STANDARD.encode("../secret")
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd traversal file");
    assert_eq!(traversal.status, "400 Bad Request");
    record_evidence!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "malformed-path-query-or-body"
    );

    let reset_state_dir = state.config.state_dir.display().to_string();
    let (reset_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_STATE_DIR", &reset_state_dir),
    );
    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "Remote/Delete.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RemoveDownload",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "Partial/Remove.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RemovePartial",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = super::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                super::STANDARD.encode(relative)
            ),
            None,
            "",
            &reset_state,
        )
        .await
        .unwrap_or_else(|error| panic!("reset DELETE {storage}/{resource}/{relative}: {error}"));
        assert_eq!(
            response.status,
            if resource == "files" {
                "204 No Content"
            } else {
                "404 Not Found"
            }
        );
        record_evidence!("DELETE", route, "restart-persistence-or-reset");
    }

    let downloads_conflict_root = std::env::temp_dir().join(format!(
        "slskr-download-delete-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let incomplete_conflict_root = std::env::temp_dir().join(format!(
        "slskr-incomplete-delete-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&downloads_conflict_root, b"downloads root is a file")
        .expect("create downloads delete conflict");
    fs::write(&incomplete_conflict_root, b"incomplete root is a file")
        .expect("create incomplete delete conflict");
    let (failure_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    *failure_state
        .downloads_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_conflict_root.clone();
    *failure_state
        .incomplete_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_conflict_root.clone();
    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "RuntimeFailure.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RuntimeFailureDirectory",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "RuntimeFailure.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RuntimeFailureDirectory",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = super::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                super::STANDARD.encode(relative)
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .unwrap_or_else(|error| panic!("runtime DELETE {storage}/{resource}/{relative}: {error}"));
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(response.body, r#"{"error":"file storage unavailable"}"#);
        assert!(!response.body.contains("root is a file"));
        record_evidence!("DELETE", route, "runtime-failure-and-timeout");
    }
    for (storage, relative, route) in [
        (
            "downloads",
            "RuntimeFailureDirectory",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "RuntimeFailureDirectory",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = super::route_http_request(
            "GET",
            &format!(
                "/api/v0/files/{storage}/directories/{}",
                super::STANDARD.encode(relative)
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .unwrap_or_else(|error| panic!("runtime GET {storage}/{relative}: {error}"));
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(response.body, r#"{"error":"file storage unavailable"}"#);
        assert!(!response.body.contains("root is a file"));
        record_evidence!("GET", route, "runtime-failure-and-timeout");
    }
    #[cfg(unix)]
    {
        let downloads_target = std::env::temp_dir().join(format!(
            "slskr-download-list-target-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let incomplete_target = std::env::temp_dir().join(format!(
            "slskr-incomplete-list-target-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let downloads_link = std::env::temp_dir().join(format!(
            "slskr-download-list-link-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let incomplete_link = std::env::temp_dir().join(format!(
            "slskr-incomplete-list-link-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&downloads_target).expect("create downloads list target");
        fs::create_dir_all(&incomplete_target).expect("create incomplete list target");
        std::os::unix::fs::symlink(&downloads_target, &downloads_link)
            .expect("create downloads list symlink");
        std::os::unix::fs::symlink(&incomplete_target, &incomplete_link)
            .expect("create incomplete list symlink");
        let (symlink_state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "legacy")
                .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
        );
        *symlink_state
            .downloads_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_link.clone();
        *symlink_state
            .incomplete_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_link.clone();
        for (storage, route) in [
            ("downloads", "/api/v0/files/downloads/directories"),
            ("incomplete", "/api/v0/files/incomplete/directories"),
        ] {
            let response = super::route_http_request(
                "GET",
                &format!("/api/v0/files/{storage}/directories"),
                None,
                "",
                &symlink_state,
            )
            .await
            .unwrap_or_else(|error| panic!("runtime root GET {storage}: {error}"));
            assert_eq!(response.status, "503 Service Unavailable");
            assert_eq!(response.body, r#"{"error":"file storage unavailable"}"#);
            record_evidence!("GET", route, "runtime-failure-and-timeout");
        }
        let _ = fs::remove_file(downloads_link);
        let _ = fs::remove_file(incomplete_link);
        let _ = fs::remove_dir_all(downloads_target);
        let _ = fs::remove_dir_all(incomplete_target);
    }
    let _ = fs::remove_file(downloads_conflict_root);
    let _ = fs::remove_file(incomplete_conflict_root);

    let enqueue = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        r#"{"files":[{"filename":"Remote/Queued.flac","size":99}]}"#,
        &state,
    )
    .await
    .expect("slskd transfer enqueue");
    assert_eq!(enqueue.status, "200 OK");
    let enqueue_json = serde_json::from_str::<serde_json::Value>(&enqueue.body).unwrap();
    assert_eq!(enqueue_json["queued"], 1);
    assert_eq!(enqueue_json["transfers"][0]["username"], "peer");
    let transfer_id = enqueue_json["transfers"][0]["id"]
        .as_u64()
        .or_else(|| {
            enqueue_json["transfers"][0]["id"]
                .as_str()
                .and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or_else(|| panic!("slskd transfer id missing from {}", enqueue.body));
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "mutation-side-effects-and-readback"
    );

    let transfer_list =
        super::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
            .await
            .expect("slskd transfer list");
    assert_eq!(transfer_list.status, "200 OK");
    let transfer_list_json =
        serde_json::from_str::<serde_json::Value>(&transfer_list.body).unwrap();
    assert_eq!(transfer_list_json[0]["username"], "peer");
    assert_eq!(
        transfer_list_json[0]["directories"][0]["directory"],
        "Remote"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads",
        "populated-dynamic-state"
    );

    let transfer_detail = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd transfer detail");
    assert_eq!(transfer_detail.status, "200 OK");
    let transfer_detail_json =
        serde_json::from_str::<serde_json::Value>(&transfer_detail.body).unwrap();
    assert!(
        transfer_detail_json["id"] == serde_json::json!(transfer_id)
            || transfer_detail_json["id"] == serde_json::json!(transfer_id.to_string()),
        "unexpected slskd transfer id: {}",
        transfer_detail.body
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "populated-dynamic-state"
    );

    let missing_transfer = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/other/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd transfer");
    assert_eq!(missing_transfer.status, "404 Not Found");
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "missing-empty-or-conflict-state"
    );

    let cancelled = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskd transfer");
    assert_eq!(cancelled.status, "204 No Content");
    record_evidence!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "mutation-side-effects-and-readback"
    );

    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let batch_request = format!(
        r#"{{"id":"{batch_id}","username":"peer","files":[{{"filename":"Music/A.flac","size":42}}]}}"#
    );
    let batch = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_request,
        &state,
    )
    .await
    .expect("slskd transfer batch");
    assert_eq!(batch.status, "201 Created");
    let batch_json = serde_json::from_str::<serde_json::Value>(&batch.body).unwrap();
    assert_eq!(batch_json["batch"]["id"], batch_id);
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "mutation-side-effects-and-readback"
    );

    let fetched_batch = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd fetched transfer batch");
    assert_eq!(fetched_batch.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&fetched_batch.body).unwrap()["id"],
        batch_id
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "populated-dynamic-state"
    );

    let duplicate_batch = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_request,
        &state,
    )
    .await
    .expect("duplicate slskd transfer batch");
    assert_eq!(duplicate_batch.status, "409 Conflict");
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "missing-empty-or-conflict-state"
    );

    state.session.write().await.state = "connected";
    for (method, path, route) in [
        ("GET", "/api/v0/rooms/joined", "/api/v0/rooms/joined"),
        ("GET", "/api/v0/rooms/available", "/api/v0/rooms/available"),
    ] {
        let response = super::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "200 OK");
        assert!(serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap()
            .is_array());
        record_evidence!(method, route, "nominal-status-headers-body");
    }

    for (method, path, route) in [
        (
            "GET",
            "/api/v0/rooms/joined/missing",
            "/api/v0/rooms/joined/{roomName}",
        ),
        (
            "GET",
            "/api/v0/rooms/joined/missing/users",
            "/api/v0/rooms/joined/{roomName}/users",
        ),
        (
            "GET",
            "/api/v0/rooms/joined/missing/messages",
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
        (
            "DELETE",
            "/api/v0/rooms/joined/missing",
            "/api/v0/rooms/joined/{roomName}",
        ),
        (
            "POST",
            "/api/v0/rooms/joined/missing/messages",
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
        (
            "POST",
            "/api/v0/rooms/joined/missing/ticker",
            "/api/v0/rooms/joined/{roomName}/ticker",
        ),
        (
            "POST",
            "/api/v0/rooms/joined/missing/members",
            "/api/v0/rooms/joined/{roomName}/members",
        ),
    ] {
        let response = super::route_http_request(method, path, None, r#""value""#, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
        record_evidence!(method, route, "missing-empty-or-conflict-state");
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("controller_file_transfer_room_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd controller ledger"),
    )
    .expect("write slskd controller ledger");
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_file_application_and_roster_edges() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let incomplete_root = state.config.incomplete_dir.clone();
    std::fs::create_dir_all(incomplete_root.join("Nested")).unwrap();
    std::fs::write(incomplete_root.join("Nested/Track.part"), b"partial").unwrap();
    let nested = super::route_http_request(
        "GET",
        "/api/v0/files/incomplete/directories/TmVzdGVk",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated incomplete directory");
    let nested_json = serde_json::from_str::<serde_json::Value>(&nested.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "nominal-status-headers-body",
        nested.status == "200 OK"
            && nested_json["files"]
                .as_array()
                .is_some_and(|files| { files.iter().any(|file| file["name"] == "Track.part") })
    );
    record!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "populated-dynamic-state",
        nested.status == "200 OK"
            && nested_json["files"]
                .as_array()
                .is_some_and(|files| !files.is_empty())
    );

    for (path, route) in [
        (
            "/api/v0/files/downloads/directories?recursive=not-a-boolean",
            "/api/v0/files/downloads/directories",
        ),
        (
            "/api/v0/files/downloads/directories/TmVzdGVk?recursive=not-a-boolean",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "/api/v0/files/incomplete/directories?recursive=not-a-boolean",
            "/api/v0/files/incomplete/directories",
        ),
        (
            "/api/v0/files/incomplete/directories/TmVzdGVk?recursive=not-a-boolean",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    let missing_download_directory = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories/Tm9TdWNoRGlyZWN0b3J5",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd missing downloads directory");
    record!(
        "GET",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "missing-empty-or-conflict-state",
        missing_download_directory.status == "404 Not Found"
    );

    for (resource, encoded, route) in [
        (
            "directories",
            "Tm9TdWNoRGlyZWN0b3J5",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "directories",
            "Tm9TdWNoRGlyZWN0b3J5",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
        (
            "files",
            "Tm9TdWNoRmlsZS5wYXJ0",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
    ] {
        let storage = if route.contains("/downloads/") {
            "downloads"
        } else {
            "incomplete"
        };
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/files/{storage}/{resource}/{encoded}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE {storage}/{resource}: {error}"));
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            response.status
                == if resource == "files" {
                    "204 No Content"
                } else {
                    "404 Not Found"
                }
        );
    }

    let version = super::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("slskd populated application version");
    record!(
        "GET",
        "/api/v0/application/version",
        "populated-dynamic-state",
        version.status == "200 OK" && !version.body.is_empty()
    );

    super::record_daemon_log(
        &state,
        super::logging::LogLevel::Info,
        "slskd.edge",
        "populated log entry",
    )
    .await;
    let logs = super::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("slskd populated logs");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/logs",
        "populated-dynamic-state",
        logs.status == "200 OK"
            && logs_json.as_array().is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry["category"] == "slskd.edge")
            })
    );

    {
        let mut session = state.session.write().await;
        session.state = "connected";
        session.username = Some("edge-user".to_owned());
    }
    let session = super::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("slskd populated session");
    let session_json = serde_json::from_str::<serde_json::Value>(&session.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "populated-dynamic-state",
        session.status == "200 OK" && session_json["state"] == "connected"
    );

    let (roster_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    roster_state.session.write().await.state = "connected";
    roster_state.session.write().await.username = Some("edge-self".to_owned());
    {
        let mut rooms = roster_state.rooms.write().await;
        let room = rooms.join("roster-edge".to_owned()).expect("room fixture");
        let stored = rooms
            .records
            .iter_mut()
            .find(|candidate| candidate.name == room.name)
            .expect("stored room fixture");
        stored.members = vec!["edge-self".to_owned(), "edge-peer".to_owned()];
        stored.roster = vec![
            super::RoomRosterEntry {
                username: "edge-self".to_owned(),
                status: 2,
                average_speed: 1_000,
                upload_count: 5,
                file_count: 42,
                directory_count: 3,
                slots_free: 1,
                country_code: "CA".to_owned(),
            },
            super::RoomRosterEntry {
                username: "edge-peer".to_owned(),
                status: 1,
                average_speed: 0,
                upload_count: 0,
                file_count: 0,
                directory_count: 0,
                slots_free: 0,
                country_code: String::new(),
            },
        ];
    }
    let roster = super::route_http_request(
        "GET",
        "/api/v0/rooms/joined/roster-edge/users",
        None,
        "",
        &roster_state,
    )
    .await
    .expect("slskd populated room roster");
    let roster_json = serde_json::from_str::<serde_json::Value>(&roster.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        "nominal-status-headers-body",
        roster.status == "200 OK" && roster_json.as_array().is_some_and(|users| users.len() == 2)
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        "populated-dynamic-state",
        roster.status == "200 OK" && roster_json[0]["username"] == "edge-self"
    );
    let available =
        super::route_http_request("GET", "/api/v0/rooms/available", None, "", &roster_state)
            .await
            .expect("slskd populated available rooms");
    let available_json =
        serde_json::from_str::<serde_json::Value>(&available.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/available",
        "populated-dynamic-state",
        available.status == "200 OK"
            && available_json
                .as_array()
                .is_some_and(|rooms| { rooms.iter().any(|room| room["name"] == "roster-edge") })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_file_application_roster_edges.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd edge ledger"),
    )
    .expect("write slskd edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd edge mismatches: {}",
        mismatches.len(),
        mismatches.join("; ")
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_share_and_relay_lifecycle() {
    let target = "slskd";
    let relay_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKD_RELAY", "true")
        .with("SLSKD_RELAY_MODE", "agent")
        .with("SLSKD_CONTROLLER_ADDRESS", "http://127.0.0.1:9")
        .with("SLSKD_CONTROLLER_API_KEY", "relay-api-key-123456")
        .with("SLSKD_CONTROLLER_SECRET", "relay-secret-123456");
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("relay lifecycle database");
    let (state, _receiver) = test_state_with_env_parts(
        relay_env.clone(),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let started = super::route_http_request("PUT", "/api/v0/relay/agent", None, "", &state)
        .await
        .expect("slskd relay-agent start");
    let started_row = db
        .get_runtime_compat_state()
        .await
        .expect("read relay-agent start")
        .expect("relay-agent runtime row");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "nominal-status-headers-body",
        started.status == "200 OK" && started.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "mutation-side-effects-and-readback",
        state.runtime.read().await.relay_agent_enabled
            && started_row.relay_agent_enabled
            && started_row.relay_enabled
    );

    let repeated_start = super::route_http_request("PUT", "/api/v0/relay/agent", None, "", &state)
        .await
        .expect("repeat slskd relay-agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "concurrency-and-idempotency",
        repeated_start.status == "200 OK" && repeated_start.body.is_empty()
    );

    let stopped = super::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &state)
        .await
        .expect("slskd relay-agent stop");
    let stopped_row = db
        .get_runtime_compat_state()
        .await
        .expect("read relay-agent stop")
        .expect("relay-agent stopped runtime row");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "nominal-status-headers-body",
        stopped.status == "204 No Content" && stopped.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "mutation-side-effects-and-readback",
        !state.runtime.read().await.relay_agent_enabled && !stopped_row.relay_agent_enabled
    );

    let repeated_stop =
        super::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &state)
            .await
            .expect("repeat slskd relay-agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "concurrency-and-idempotency",
        repeated_stop.status == "204 No Content" && repeated_stop.body.is_empty()
    );

    let (restarted_relay_state, _receiver) = test_state_with_env(relay_env.clone());
    let restarted_start = super::route_http_request(
        "PUT",
        "/api/v0/relay/agent",
        None,
        "",
        &restarted_relay_state,
    )
    .await
    .expect("restarted slskd relay-agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "restart-persistence-or-reset",
        restarted_start.status == "200 OK"
            && restarted_start.body.is_empty()
            && restarted_relay_state
                .runtime
                .read()
                .await
                .relay_agent_enabled
    );
    let restarted_stop = super::route_http_request(
        "DELETE",
        "/api/v0/relay/agent",
        None,
        "",
        &restarted_relay_state,
    )
    .await
    .expect("restarted slskd relay-agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "restart-persistence-or-reset",
        restarted_stop.status == "204 No Content"
            && restarted_stop.body.is_empty()
            && !restarted_relay_state
                .runtime
                .read()
                .await
                .relay_agent_enabled
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("relay failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        relay_env,
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_start =
        super::route_http_request("PUT", "/api/v0/relay/agent", None, "", &failure_state)
            .await
            .expect("failed relay-agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "runtime-failure-and-timeout",
        failed_start.status == "503 Service Unavailable"
            && !failure_state.runtime.read().await.relay_agent_enabled
    );
    let failed_stop =
        super::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &failure_state)
            .await
            .expect("failed relay-agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "runtime-failure-and-timeout",
        failed_stop.status == "503 Service Unavailable"
            && !failure_state.runtime.read().await.relay_agent_enabled
    );

    let (disabled_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    for method in ["PUT", "DELETE"] {
        let response =
            super::route_http_request(method, "/api/v0/relay/agent", None, "", &disabled_state)
                .await
                .unwrap_or_else(|error| panic!("disabled relay-agent {method}: {error}"));
        record!(
            method,
            "/api/v0/relay/agent",
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }

    let (share_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let scan_permit = share_state
        .share_scans
        .clone()
        .acquire_owned()
        .await
        .expect("occupy share scan permit");
    let cancelled = super::route_http_request("DELETE", "/api/v0/shares", None, "", &share_state)
        .await
        .expect("slskd share scan cancellation");
    record!(
        "DELETE",
        "/api/v0/shares",
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/shares",
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content" && share_state.share_scans.available_permits() == 1
    );
    let repeated_cancel =
        super::route_http_request("DELETE", "/api/v0/shares", None, "", &share_state)
            .await
            .expect("repeat slskd share scan cancellation");
    record!(
        "DELETE",
        "/api/v0/shares",
        "concurrency-and-idempotency",
        repeated_cancel.status == "404 Not Found"
    );
    drop(scan_permit);

    let (restarted_share_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let reset =
        super::route_http_request("DELETE", "/api/v0/shares", None, "", &restarted_share_state)
            .await
            .expect("slskd share scan restart state");
    record!(
        "DELETE",
        "/api/v0/shares",
        "restart-persistence-or-reset",
        reset.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_share_relay_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd share/relay ledger"),
    )
    .expect("write slskd share/relay ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd share/relay controller mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_fixed_route_malformed_paths() {
    let target = "slskd";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [malformed-path-query-or-body]",
                    $method, $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }};
    }

    for (method, route) in [
        ("DELETE", "/api/v0/application"),
        ("DELETE", "/api/v0/relay/agent"),
        ("DELETE", "/api/v0/server"),
        ("DELETE", "/api/v0/shares"),
        ("GET", "/api/v0/application"),
        ("GET", "/api/v0/application/dump"),
        ("GET", "/api/v0/application/version"),
        ("GET", "/api/v0/application/version/latest"),
        ("GET", "/api/v0/logs"),
        ("GET", "/api/v0/rooms/available"),
        ("GET", "/api/v0/server"),
        ("GET", "/api/v0/shares/contents"),
        ("GET", "/api/v0/telemetry/metrics"),
        ("GET", "/api/v0/telemetry/metrics/kpis"),
        ("GET", "/api/v0/telemetry/reports/transfers/summary"),
        ("POST", "/api/v0/application/gc"),
        ("POST", "/api/v0/application/loopback"),
        ("POST", "/api/v0/rooms/joined"),
        ("PUT", "/api/v0/application"),
        ("PUT", "/api/v0/options"),
        ("PUT", "/api/v0/server"),
        ("PUT", "/api/v0/shares"),
    ] {
        let malformed_path = format!("{route}/malformed");
        let response = super::route_http_request(method, &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {malformed_path}: {error}"));
        record!(method, route, response.status == "404 Not Found");
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_fixed_route_malformed_paths.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd malformed-path ledger"),
    )
    .expect("write slskd malformed-path ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd malformed-path mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_parameterized_malformed_paths() {
    let target = "slskd";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $path:expr, $expected:expr) => {{
            let response = super::route_http_request($method, $path, None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {error}", $method, $path));
            let pass = response.status == $expected;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} expected {}, got {}",
                    $method, $path, $expected, response.status
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }};
    }

    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/peer/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "/api/v0/files/downloads/directories/TmVzdGVk/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "/api/v0/files/incomplete/directories/TmVzdGVk/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/files/incomplete/files/{base64FileName}",
        "/api/v0/files/incomplete/files/VHJhY2sucGFydA==/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "/api/v0/rooms/joined/music/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "/api/v0/transfers/downloads/all/completed/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "/api/v0/transfers/uploads/all/completed/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "/api/v0/transfers/uploads/peer/1/extra",
        "404 Not Found"
    );

    record!(
        "GET",
        "/api/v0/conversations",
        "/api/v0/conversations?includeInactive=not-a-boolean",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/events",
        "/api/v0/events/malformed",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/options",
        "/api/v0/options/malformed",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "/api/v0/relay/controller/downloads/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined",
        "/api/v0/rooms/joined/malformed/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        "/api/v0/rooms/joined/music/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        "/api/v0/rooms/joined/music/messages/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        "/api/v0/rooms/joined/music/users/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/searches",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "/api/v0/searches/not-a-guid/extra/responses",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/session",
        "/api/v0/session/malformed",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares",
        "/api/v0/shares/extra/path",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "/api/v0/shares/extra/path",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "/api/v0/shares/extra/contents/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares/contents",
        "/api/v0/shares/contents/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "/api/v0/telemetry/reports/transfers/directories/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "/api/v0/telemetry/reports/transfers/users/peer/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads",
        "/api/v0/transfers/downloads/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "/api/v0/transfers/downloads/peer/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads",
        "/api/v0/transfers/uploads/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "/api/v0/transfers/uploads/peer/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "/api/v0/users/peer/browse/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "/api/v0/users/peer/browse/status/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "/api/v0/users/peer/endpoint/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "/api/v0/users/peer/info/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "/api/v0/users/peer/status/extra",
        "404 Not Found"
    );

    record!(
        "POST",
        "/api/v0/options",
        "/api/v0/options/malformed",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "/api/v0/relay/controller/files/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "/api/v0/relay/controller/shares/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/members",
        "/api/v0/rooms/joined/music/members/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "/api/v0/rooms/joined/music/messages/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/ticker",
        "/api/v0/rooms/joined/music/ticker/extra",
        "404 Not Found"
    );

    record!(
        "PUT",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/peer/extra/extra",
        "404 Not Found"
    );
    record!(
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "/api/v0/conversations/peer/1/extra",
        "404 Not Found"
    );
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "/api/v0/relay/agent/malformed",
        "404 Not Found"
    );
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_parameterized_malformed_paths.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize slskd parameterized malformed ledger"),
    )
    .expect("write slskd parameterized malformed ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd parameterized malformed-path mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_empty_and_missing_state() {
    let target = "slskd";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let (relay_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKD_RELAY", "true")
            .with("SLSKD_RELAY_MODE", "controller")
            .with("SLSKD_CONTROLLER_ADDRESS", "http://127.0.0.1:9")
            .with("SLSKD_CONTROLLER_API_KEY", "relay-api-key-123456")
            .with("SLSKD_CONTROLLER_SECRET", "relay-secret-123456"),
    );
    let (file_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let _ = std::fs::remove_dir_all(&file_state.config.downloads_dir);
    let _ = std::fs::remove_dir_all(&file_state.config.incomplete_dir);
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($state:expr, $method:expr, $route:expr, $path:expr, $body:expr, $expected:expr) => {{
            let response = super::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {error}", $method, $path));
            let pass = response.status == $expected;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} expected {}, got {}",
                    $method, $path, $expected, response.status
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "missing-empty-or-conflict-state",
                "pass": pass,
            }));
        }};
    }

    record!(
        &state,
        "GET",
        "/api/v0/application/version",
        "/api/v0/application/version",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/application/version/latest",
        "/api/v0/application/version/latest",
        "",
        "200 OK"
    );
    record!(&state, "GET", "/api/v0/logs", "/api/v0/logs", "", "200 OK");
    record!(
        &state,
        "GET",
        "/api/v0/options",
        "/api/v0/options",
        "",
        "200 OK"
    );
    state.session.write().await.state = "connected";
    record!(
        &state,
        "GET",
        "/api/v0/rooms/available",
        "/api/v0/rooms/available",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/rooms/joined",
        "/api/v0/rooms/joined",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/searches",
        "/api/v0/searches",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/conversations",
        "/api/v0/conversations",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/conversations/{username}/messages",
        "/api/v0/conversations/missing/messages",
        "",
        "404 Not Found"
    );
    record!(
        &file_state,
        "GET",
        "/api/v0/files/downloads/directories",
        "/api/v0/files/downloads/directories",
        "",
        "404 Not Found"
    );
    record!(
        &file_state,
        "GET",
        "/api/v0/files/incomplete/directories",
        "/api/v0/files/incomplete/directories",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "GET",
        "/api/v0/server",
        "/api/v0/server",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/session",
        "/api/v0/session",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/shares",
        "/api/v0/shares",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/shares/contents",
        "/api/v0/shares/contents",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "/api/v0/telemetry/reports/transfers/users/missing",
        "",
        "200 OK"
    );
    record!(
        &relay_state,
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "/api/v0/relay/controller/downloads/11111111-1111-4111-8111-111111111111",
        "",
        "401 Unauthorized"
    );

    record!(
        &state,
        "POST",
        "/api/v0/application/gc",
        "/api/v0/application/gc",
        "",
        "200 OK"
    );
    record!(
        &state,
        "POST",
        "/api/v0/application/loopback",
        "/api/v0/application/loopback",
        "",
        "400 Bad Request"
    );
    state.session.write().await.state = "connected";
    record!(
        &state,
        "POST",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/missing",
        "",
        "400 Bad Request"
    );
    record!(
        &state,
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "/api/v0/transfers/downloads/missing",
        "",
        "400 Bad Request"
    );
    record!(
        &state,
        "POST",
        "/api/v0/users/{username}/directory",
        "/api/v0/users/missing/directory",
        "",
        "400 Bad Request"
    );

    record!(
        &state,
        "PUT",
        "/api/v0/application",
        "/api/v0/application",
        "{}",
        "204 No Content"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/missing",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "/api/v0/conversations/missing/999",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/searches/{id}",
        "/api/v0/searches/ffffffff-ffff-4fff-8fff-ffffffffffff",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/server",
        "/api/v0/server",
        "",
        "205 Reset Content"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/shares",
        "/api/v0/shares",
        "",
        "200 OK"
    );
    record!(
        &state,
        "DELETE",
        "/api/v0/application",
        "/api/v0/application",
        "",
        "204 No Content"
    );
    record!(
        &state,
        "DELETE",
        "/api/v0/server",
        "/api/v0/server",
        "",
        "204 No Content"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_empty_and_missing_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd empty/missing state ledger"),
    )
    .expect("write slskd empty/missing state ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd empty/missing state mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_relay_controller_routes() {
    let target = "slskd";
    let (state, secret, now) = configured_relay_test_state().await;
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    async fn live_relay_download(
        state: Arc<super::AppState>,
        token: &str,
        credential: &str,
    ) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
        let request = format!(
            "GET /api/v0/relay/controller/downloads/{token} HTTP/1.1\r\n\
             Host: localhost\r\n\
             X-Relay-Credential: {credential}\r\n\
             Connection: close\r\n\r\n"
        );
        client
            .write_all(request.as_bytes())
            .await
            .expect("write relay download request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read relay download response");
        task.await
            .expect("relay download HTTP task")
            .expect("relay download HTTP response");
        response
    }

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

    let download_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_download_tokens("Relay/Agent.txt", now)
        .into_iter()
        .next()
        .map(|(_, token)| token)
        .expect("relay download token");
    let download_credential =
        super::relay::credential_for_test(secret, "edge-one", &download_token);
    let downloads_root = super::effective_downloads_dir(&state);
    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download root");
    fs::write(downloads_root.join("Relay/Agent.txt"), b"relay payload")
        .expect("relay download fixture");
    let download_headers = super::RequestSecurityHeaders {
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(download_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let download = Box::pin(super::route_http_request_with_headers(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &state,
        download_headers.clone(),
    ))
    .await
    .expect("relay controller download");
    let mut stream =
        super::open_relay_controller_download(&state, &download_token, &download_headers)
            .await
            .expect("open relay controller download");
    let mut payload = Vec::new();
    std::io::Read::read_to_end(&mut stream.file, &mut payload).expect("read relay download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "nominal-status-headers-body",
        download.status == "200 OK"
            && download.content_type == "application/octet-stream"
            && download.body.is_empty()
    );
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "populated-dynamic-state",
        payload == b"relay payload"
    );
    fs::remove_file(downloads_root.join("Relay/Agent.txt")).expect("remove relay download fixture");
    let runtime_download = live_relay_download(
        Arc::clone(&state),
        &download_token.to_string(),
        &download_credential,
    )
    .await;
    let runtime_download = String::from_utf8_lossy(&runtime_download);
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "runtime-failure-and-timeout",
        runtime_download.starts_with("HTTP/1.1 500 Internal Server Error")
            && runtime_download.contains("failed to open relay controller download")
            && !runtime_download.contains("Relay/Agent.txt")
    );

    let (upload_token, upload_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Upload.flac", 0, now)
        .expect("relay upload stream");
    let upload_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(super::relay::credential_for_test(
            secret,
            "edge-one",
            &upload_token.to_string(),
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let upload_body =
        "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Upload.flac\"\r\n\r\npayload\r\n--relay--\r\n";
    let upload = Box::pin(super::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay controller file upload");
    let uploaded = upload_receiver
        .await
        .expect("relay upload receiver")
        .expect("relay upload result");
    assert_eq!(uploaded.filename, "Upload.flac");
    let stored_upload = state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!("file-{}.part", upload_token.simple()));
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "nominal-status-headers-body",
        upload.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "mutation-side-effects-and-readback",
        fs::read(&stored_upload).ok().as_deref() == Some(b"payload".as_slice())
    );
    let replay = Box::pin(super::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay controller file replay");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "concurrency-and-idempotency",
        replay.status == "401 Unauthorized"
    );

    let missing_file_token = uuid::Uuid::new_v4();
    let missing_file_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(super::relay::credential_for_test(
            secret,
            "edge-one",
            &missing_file_token.to_string(),
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let missing_file = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{missing_file_token}"),
        upload_body.as_bytes(),
        &missing_file_headers,
        &state,
    )
    .await
    .expect("missing relay controller file token response");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "missing-empty-or-conflict-state",
        missing_file.status == "401 Unauthorized"
    );

    let (fresh_relay_state, _fresh_secret, _fresh_now) = configured_relay_test_state().await;
    let restart_file = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        upload_body.as_bytes(),
        &upload_headers,
        &fresh_relay_state,
    )
    .await
    .expect("reset relay controller file token response");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "restart-persistence-or-reset",
        restart_file.status == "401 Unauthorized"
    );

    let incoming_directory = state.config.state_dir.join("relay").join("incoming");
    fs::remove_dir_all(&incoming_directory).expect("remove relay upload directory");
    fs::write(&incoming_directory, b"relay upload directory is a file")
        .expect("create relay upload directory conflict");
    let (runtime_file_token, runtime_file_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Upload.flac", 0, now)
        .expect("runtime relay upload stream");
    let runtime_file_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(super::relay::credential_for_test(
            secret,
            "edge-one",
            &runtime_file_token.to_string(),
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let runtime_file = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{runtime_file_token}"),
        upload_body.as_bytes(),
        &runtime_file_headers,
        &state,
    )
    .await
    .expect("runtime relay controller file response");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "runtime-failure-and-timeout",
        runtime_file.status == "503 Service Unavailable"
            && !runtime_file
                .body
                .contains("relay upload directory is a file")
    );
    assert!(runtime_file_receiver
        .await
        .expect("runtime relay upload receiver")
        .is_err());
    fs::remove_file(&incoming_directory).expect("remove relay upload conflict");
    fs::create_dir_all(&incoming_directory).expect("restore relay upload directory");

    let share_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("relay share upload token");
    let share_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(super::relay::credential_for_test(
            secret,
            "edge-one",
            &share_token,
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let database_source = state.config.state_dir.join("relay-differential-source.db");
    super::relay::write_share_database(
        &database_source,
        super::ControllerProfile::Legacy,
        &[super::relay::RemoteShare {
            filename: "Remote/Agent.flac".to_owned(),
            size: 6,
        }],
    )
    .await
    .expect("relay differential share database");
    let database_bytes = fs::read(&database_source).expect("read relay differential database");
    let mut share_body = Vec::new();
    share_body.extend_from_slice(
        b"--relay\r\nContent-Disposition: form-data; name=\"shares\"\r\n\r\n[]\r\n--relay\r\nContent-Disposition: form-data; name=\"database\"; filename=\"shares.db\"\r\n\r\n",
    );
    share_body.extend_from_slice(&database_bytes);
    share_body.extend_from_slice(b"\r\n--relay--\r\n");
    let shares = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &state,
    )
    .await
    .expect("relay controller share upload");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "nominal-status-headers-body",
        shares.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "mutation-side-effects-and-readback",
        state
            .relay
            .read()
            .await
            .protocol
            .remote_file_for_agent("edge-one", "Remote/Agent.flac")
            .is_some()
    );
    let share_replay = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &state,
    )
    .await
    .expect("relay controller share replay");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "concurrency-and-idempotency",
        share_replay.status == "401 Unauthorized"
    );

    let missing_share_token = uuid::Uuid::new_v4().to_string();
    let missing_share_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(super::relay::credential_for_test(
            secret,
            "edge-one",
            &missing_share_token,
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let missing_share = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{missing_share_token}"),
        &share_body,
        &missing_share_headers,
        &state,
    )
    .await
    .expect("missing relay controller share token response");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "missing-empty-or-conflict-state",
        missing_share.status == "401 Unauthorized"
    );

    let restart_share = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &fresh_relay_state,
    )
    .await
    .expect("reset relay controller share token response");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "restart-persistence-or-reset",
        restart_share.status == "401 Unauthorized"
    );

    fs::remove_dir_all(&incoming_directory).expect("remove relay share upload directory");
    fs::write(
        &incoming_directory,
        b"relay share upload directory is a file",
    )
    .expect("create relay share upload directory conflict");
    let runtime_share_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("runtime relay share upload token");
    let runtime_share_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(super::relay::credential_for_test(
            secret,
            "edge-one",
            &runtime_share_token,
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let runtime_share = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{runtime_share_token}"),
        &share_body,
        &runtime_share_headers,
        &state,
    )
    .await
    .expect("runtime relay controller share response");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "runtime-failure-and-timeout",
        runtime_share.status == "503 Service Unavailable"
            && !runtime_share
                .body
                .contains("relay share upload directory is a file")
    );
    fs::remove_file(&incoming_directory).expect("remove relay share upload conflict");
    fs::create_dir_all(&incoming_directory).expect("restore relay share upload directory");
    let _ = fs::remove_file(database_source);
    let _ = fs::remove_dir_all(downloads_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_relay_controller_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd relay controller ledger"),
    )
    .expect("write slskd relay controller ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd relay controller mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_core_application_session_events_and_telemetry() {
    let target = "slskd";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
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

    let application = super::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("slskd application state");
    let application_json =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        "nominal-status-headers-body",
        application.status == "200 OK"
            && application.content_type == "application/json; charset=utf-8"
            && application_json["runtimeProfile"]
                == if target == "slskd" {
                    "legacy"
                } else {
                    "native"
                }
            && application_json["version"]["current"] == env!("CARGO_PKG_VERSION")
            && application_json["version"]["full"].is_string()
            && application_json["pendingReconnect"].is_boolean()
            && application_json["pendingRestart"] == false
            && application_json["server"].is_object()
            && application_json["connectionWatchdog"].is_object()
            && application_json["vpn"].is_object()
            && application_json["shares"].is_object()
            && application_json["rooms"].is_array()
            && application_json["users"].is_array()
            && application_json["vpn"].get("portForwards").is_none()
    );

    let restart = super::route_http_request("PUT", "/api/v0/application", None, "{}", &state)
        .await
        .expect("slskd application restart");
    record!(
        "PUT",
        "/api/v0/application",
        "nominal-status-headers-body",
        restart.status == "204 No Content" && restart.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        state.runtime.read().await.application_restart_requested
    );
    let populated_application =
        super::route_http_request("GET", "/api/v0/application", None, "", &state)
            .await
            .expect("populated slskd application state");
    let populated_application_json =
        serde_json::from_str::<serde_json::Value>(&populated_application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        "populated-dynamic-state",
        populated_application.status == "200 OK"
            && populated_application_json["pendingRestart"] == true
    );

    let version = super::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("slskd application version");
    record!(
        "GET",
        "/api/v0/application/version",
        "nominal-status-headers-body",
        version.status == "200 OK"
            && version.content_type == "application/json"
            && serde_json::from_str::<serde_json::Value>(&version.body).unwrap_or_default()
                == serde_json::json!(env!("CARGO_PKG_VERSION"))
    );

    let latest = super::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd latest version");
    let latest_json = serde_json::from_str::<serde_json::Value>(&latest.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "nominal-status-headers-body",
        latest.status == "200 OK"
            && latest_json["current"] == env!("CARGO_PKG_VERSION")
            && latest_json["isCanary"].is_boolean()
            && latest_json["isDevelopment"].is_boolean()
            && latest_json.get("latest").is_none()
    );
    {
        let mut version_state = state
            .controller_version
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        version_state.latest = Some("0.0.1".to_owned());
        version_state.latest_tag = Some("v0.0.1".to_owned());
        version_state.latest_url = Some("https://example.test/slskd/0.0.1".to_owned());
        version_state.checked_at = Some("2026-08-08T00:00:00Z".to_owned());
        version_state.is_update_available = Some(true);
    }
    let populated_latest = super::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("populated slskd latest version");
    let populated_latest_json =
        serde_json::from_str::<serde_json::Value>(&populated_latest.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "populated-dynamic-state",
        populated_latest.status == "200 OK"
            && populated_latest_json["latest"] == "0.0.1"
            && populated_latest_json["latestTag"] == "v0.0.1"
            && populated_latest_json["latestUrl"] == "https://example.test/slskd/0.0.1"
            && populated_latest_json["checkedAt"] == "2026-08-08T00:00:00Z"
            && populated_latest_json["isUpdateAvailable"] == true
    );

    let gc_before = state.runtime.read().await.gc_runs;
    let gc = super::route_http_request("POST", "/api/v0/application/gc", None, "", &state)
        .await
        .expect("slskd garbage collection");
    record!(
        "POST",
        "/api/v0/application/gc",
        "nominal-status-headers-body",
        gc.status == "200 OK" && gc.content_type.is_empty() && gc.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/application/gc",
        "mutation-side-effects-and-readback",
        state.runtime.read().await.gc_runs == gc_before.saturating_add(1)
    );

    let loopback = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"slskd"}"#,
        &state,
    )
    .await
    .expect("slskd loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "nominal-status-headers-body",
        loopback.status == "200 OK" && loopback.content_type.is_empty() && loopback.body.is_empty()
    );
    let loopback_logs = super::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("slskd loopback log readback");
    let loopback_logs_json =
        serde_json::from_str::<serde_json::Value>(&loopback_logs.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/application/loopback",
        "mutation-side-effects-and-readback",
        loopback_logs.status == "200 OK"
            && loopback_logs_json.as_array().is_some_and(|logs| {
                logs.iter().any(|log| {
                    log["category"] == "application"
                        && log["message"] == "Loopback POST: {\"probe\":\"slskd\"}"
                })
            })
    );
    let (fresh_loopback_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let fresh_loopback_logs =
        super::route_http_request("GET", "/api/v0/logs", None, "", &fresh_loopback_state)
            .await
            .expect("fresh slskd loopback logs");
    let fresh_loopback = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"slskd-fresh"}"#,
        &fresh_loopback_state,
    )
    .await
    .expect("fresh slskd loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "restart-persistence-or-reset",
        fresh_loopback_logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_loopback_logs.body)
                .is_ok_and(|logs| logs.as_array().is_some_and(Vec::is_empty))
            && fresh_loopback.status == "200 OK"
    );
    let loopback_missing =
        super::route_http_request("POST", "/api/v0/application/loopback", None, "null", &state)
            .await
            .expect("slskd missing loopback body");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "malformed-path-query-or-body",
        loopback_missing.status == "400 Bad Request"
    );
    let loopback_retries = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"slskd-retry-a"}"#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"slskd-retry-b"}"#,
            &state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/application/loopback",
        "concurrency-and-idempotency",
        loopback_retries.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty())
        }) && super::route_http_request("GET", "/api/v0/logs", None, "", &state)
            .await
            .ok()
            .and_then(|logs| serde_json::from_str::<serde_json::Value>(&logs.body).ok())
            .is_some_and(|logs| {
                [
                    "Loopback POST: {\"probe\":\"slskd-retry-a\"}",
                    "Loopback POST: {\"probe\":\"slskd-retry-b\"}",
                ]
                .iter()
                .all(|message| {
                    logs.as_array()
                        .is_some_and(|rows| rows.iter().any(|row| row["message"] == *message))
                })
            })
    );
    let gc_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("application GC differential database");
    let (gc_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(gc_db.clone()),
    );
    let gc_first = super::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
        .await
        .expect("first persisted slskd garbage collection");
    let gc_second =
        super::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
            .await
            .expect("repeated persisted slskd garbage collection");
    let persisted_gc = gc_db
        .get_runtime_compat_state()
        .await
        .expect("read persisted GC state")
        .expect("persisted GC row");
    let rehydrated_gc = super::RuntimeCompatState::from_persisted(&persisted_gc);
    record!(
        "POST",
        "/api/v0/application/gc",
        "restart-persistence-or-reset",
        gc_first.status == "200 OK"
            && gc_second.status == "200 OK"
            && persisted_gc.gc_runs >= 2
            && rehydrated_gc.gc_runs >= 2
    );
    record!(
        "POST",
        "/api/v0/application/gc",
        "concurrency-and-idempotency",
        gc_second.status == "200 OK" && persisted_gc.gc_runs == 2
    );

    let session = super::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("slskd session state");
    let session_json = serde_json::from_str::<serde_json::Value>(&session.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "nominal-status-headers-body",
        session.status == "200 OK"
            && session.content_type == "application/json"
            && session_json["state"].is_string()
    );

    let login_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "slskd-login-secret")
        .with("SLSKD_USERNAME", "admin")
        .with("SLSKD_PASSWORD", "slskd-login-secret");
    let (login_state, _receiver) = test_state_with_env(login_env.clone());
    let login = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"slskd-login-secret"}"#,
        &login_state,
    )
    .await
    .expect("slskd login");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/session",
        "nominal-status-headers-body",
        login.status == "200 OK"
            && login_json["name"] == "admin"
            && login_json["tokenType"] == "Bearer"
            && login_json["token"]
                .as_str()
                .is_some_and(|token| token.split('.').count() == 3)
    );
    let (invalid_login_state, _receiver) = test_state_with_env(login_env);
    let invalid_login =
        super::route_http_request("POST", "/api/v0/session", None, "{}", &invalid_login_state)
            .await
            .expect("slskd malformed login");
    record!(
        "POST",
        "/api/v0/session",
        "malformed-path-query-or-body",
        invalid_login.status == "400 Bad Request"
    );
    let unauthorized_login = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"wrong"}"#,
        &invalid_login_state,
    )
    .await
    .expect("slskd invalid login");
    record!(
        "POST",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        unauthorized_login.status == "401 Unauthorized"
    );

    let events_before = super::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("slskd events baseline");
    let events_before_json =
        serde_json::from_str::<serde_json::Value>(&events_before.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/events",
        "nominal-status-headers-body",
        events_before.status == "200 OK"
            && events_before_json
                .as_array()
                .is_some_and(|events| { events.iter().all(|event| event["type"] != "Noop") })
    );
    let events_before_count = events_before_json
        .as_array()
        .map_or(0, |events| events.len());
    let event = super::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""core-slice""#,
        &state,
    )
    .await
    .expect("slskd event injection");
    let event_json = serde_json::from_str::<serde_json::Value>(&event.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/events",
        "nominal-status-headers-body",
        event.status == "201 Created"
            && event_json["recorded"] == true
            && event_json["event"]["type"] == "Noop"
    );
    record!(
        "POST",
        "/api/v0/events",
        "mutation-side-effects-and-readback",
        event_json["count"].as_u64() == Some(events_before_count as u64 + 1)
    );
    let events_after = super::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("slskd populated events");
    let events_after_json =
        serde_json::from_str::<serde_json::Value>(&events_after.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/events",
        "populated-dynamic-state",
        events_after.status == "200 OK"
            && events_after_json.as_array().is_some_and(|events| {
                events.len() == events_before_count + 1
                    && events.iter().any(|event| event["type"] == "Noop")
            })
    );
    let unknown_event = super::route_http_request(
        "POST",
        "/api/v0/events/Unknown",
        None,
        r#""core-slice""#,
        &state,
    )
    .await
    .expect("slskd unknown event");
    record!(
        "POST",
        "/api/v0/events",
        "malformed-path-query-or-body",
        unknown_event.status == "400 Bad Request"
    );

    let metrics = super::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
        .await
        .expect("slskd metrics");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "nominal-status-headers-body",
        metrics.status == "200 OK"
            && metrics
                .content_type
                .starts_with("text/plain; version=0.0.4")
            && metrics.body.contains("# HELP slskr_telemetry_transfers")
            && metrics.body.contains("slskr_telemetry_transfers 0")
    );
    let kpis = super::route_http_request("GET", "/api/v0/telemetry/metrics/kpis", None, "", &state)
        .await
        .expect("slskd KPI metrics");
    let kpis_json = serde_json::from_str::<serde_json::Value>(&kpis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpis",
        "nominal-status-headers-body",
        kpis.status == "200 OK"
            && kpis.content_type == "application/json"
            && kpis_json["slskr_transfers"]["samples"].is_array()
            && kpis_json["slskr_searches"]["samples"].is_array()
    );

    let empty_summary = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty transfer summary");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        "nominal-status-headers-body",
        empty_summary.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&empty_summary.body).unwrap_or_default()
                == serde_json::json!({"Download": {}, "Upload": {}})
    );
    let empty_histogram = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty transfer histogram");
    let empty_histogram_json =
        serde_json::from_str::<serde_json::Value>(&empty_histogram.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram",
        "nominal-status-headers-body",
        empty_histogram.status == "200 OK"
            && empty_histogram_json["2100-01-01T00:00:00Z"].is_object()
    );

    {
        let mut transfers = state.transfers.write().await;
        let download = transfers.create(
            0,
            Some("telemetry peer".to_owned()),
            "Telemetry/Report.flac".to_owned(),
            None,
            Some(321),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == download.id)
            .expect("populated download transfer");
        entry.status = "succeeded".to_owned();
        entry.requested_at = 3_600;
        entry.started_at = Some(3_610);
        entry.bytes_transferred = 321;
        entry.updated_at = 3_630;
        entry.updated_at_ms = 3_630_000;

        let upload = transfers.create(
            1,
            Some("upload peer".to_owned()),
            "Albums/Release/Track.flac".to_owned(),
            None,
            Some(99),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == upload.id)
            .expect("populated upload transfer");
        entry.status = "succeeded".to_owned();
        entry.requested_at = 3_600;
        entry.started_at = Some(3_605);
        entry.bytes_transferred = 99;
        entry.updated_at = 3_620;
        entry.updated_at_ms = 3_620_000;
    }

    let populated_metrics =
        super::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
            .await
            .expect("slskd populated metrics");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "populated-dynamic-state",
        populated_metrics.status == "200 OK"
            && populated_metrics
                .body
                .contains("slskr_telemetry_transfers 2")
    );
    let populated_kpis =
        super::route_http_request("GET", "/api/v0/telemetry/metrics/kpis", None, "", &state)
            .await
            .expect("slskd populated KPI metrics");
    let populated_kpis_json =
        serde_json::from_str::<serde_json::Value>(&populated_kpis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpis",
        "populated-dynamic-state",
        populated_kpis_json["slskr_transfers"]["samples"][0]["value"] == 2.0
    );

    let summary = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary?start=3600&end=7200",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated transfer summary");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        "populated-dynamic-state",
        summary.status == "200 OK"
            && summary_json["Download"]["Succeeded"]["count"] == 1
            && summary_json["Download"]["Succeeded"]["totalBytes"] == 321
            && summary_json["Upload"]["Succeeded"]["count"] == 1
            && summary_json["Upload"]["Succeeded"]["totalBytes"] == 99
    );
    let histogram = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=3600&end=7200&interval=60",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated transfer histogram");
    let histogram_json =
        serde_json::from_str::<serde_json::Value>(&histogram.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram",
        "populated-dynamic-state",
        histogram.status == "200 OK"
            && histogram_json["1970-01-01T01:00:00Z"]["Download"]["Succeeded"]["count"] == 1
            && histogram_json["1970-01-01T01:00:00Z"]["Upload"]["Succeeded"]["count"] == 1
    );

    let leaderboard_missing_direction = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd leaderboard missing direction");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "malformed-path-query-or-body",
        leaderboard_missing_direction.status == "400 Bad Request"
    );
    let leaderboard_empty = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty leaderboard");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "nominal-status-headers-body",
        leaderboard_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&leaderboard_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let leaderboard = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&start=3600&end=7200",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated leaderboard");
    let leaderboard_json =
        serde_json::from_str::<serde_json::Value>(&leaderboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "populated-dynamic-state",
        leaderboard.status == "200 OK"
            && leaderboard_json[0]["username"] == "telemetry peer"
            && leaderboard_json[0]["count"] == 1
            && leaderboard_json[0]["totalBytes"] == 321
    );

    let user_empty = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/unknown",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty user report");
    let user_empty_json =
        serde_json::from_str::<serde_json::Value>(&user_empty.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "nominal-status-headers-body",
        user_empty.status == "200 OK"
            && user_empty_json["username"] == "unknown"
            && user_empty_json["count"] == 0
    );
    let user = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated user report");
    let user_json = serde_json::from_str::<serde_json::Value>(&user.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "populated-dynamic-state",
        user.status == "200 OK"
            && user_json["username"] == "telemetry peer"
            && user_json["count"] == 1
            && user_json["transfers"]
                .as_array()
                .is_some_and(|rows| rows.len() == 1)
    );

    {
        let mut transfers = state.transfers.write().await;
        let failed = transfers.create(
            0,
            Some("telemetry peer".to_owned()),
            "Telemetry/Failed.flac".to_owned(),
            None,
            Some(12),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == failed.id)
            .expect("populated failed transfer");
        entry.status = "failed".to_owned();
        entry.reason = Some("network: timeout".to_owned());
        entry.requested_at = 3_600;
        entry.started_at = Some(3_601);
        entry.updated_at = 3_602;
        entry.updated_at_ms = 3_602_000;
    }
    let exceptions_missing_direction = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd exceptions missing direction");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "malformed-path-query-or-body",
        exceptions_missing_direction.status == "400 Bad Request"
    );
    let exceptions_empty = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions?direction=Upload&username=none",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty exceptions");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "nominal-status-headers-body",
        exceptions_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&exceptions_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let exceptions = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated exceptions");
    let exceptions_json =
        serde_json::from_str::<serde_json::Value>(&exceptions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "populated-dynamic-state",
        exceptions.status == "200 OK"
            && exceptions_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["username"] == "telemetry peer"
                    && rows[0]["direction"] == "Download"
                    && rows[0]["state"] == "Failed"
            })
    );

    let pareto_missing_direction = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd pareto missing direction");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "malformed-path-query-or-body",
        pareto_missing_direction.status == "400 Bad Request"
    );
    let pareto_empty = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Upload&username=none",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty pareto");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "nominal-status-headers-body",
        pareto_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&pareto_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let pareto = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated pareto");
    let pareto_json = serde_json::from_str::<serde_json::Value>(&pareto.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "populated-dynamic-state",
        pareto.status == "200 OK"
            && pareto_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["exception"] == "transfer failed"
                    && rows[0]["count"] == 1
            })
    );

    let directories_empty = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories?username=none",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty transfer directories");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "nominal-status-headers-body",
        directories_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&directories_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let directories = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated transfer directories");
    let directories_json =
        serde_json::from_str::<serde_json::Value>(&directories.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "populated-dynamic-state",
        directories.status == "200 OK"
            && directories_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["path"] == "Albums/Release"
                    && rows[0]["count"] == 1
                    && rows[0]["distinctUsers"] == 1
            })
    );

    let shutdown = super::route_http_request("DELETE", "/api/v0/application", None, "", &state)
        .await
        .expect("slskd application shutdown");
    record!(
        "DELETE",
        "/api/v0/application",
        "nominal-status-headers-body",
        shutdown.status == "204 No Content" && shutdown.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        !state.runtime.read().await.application_restart_requested
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_core_application_session_events_telemetry.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd core ledger"),
    )
    .expect("write slskd core ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd core controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
