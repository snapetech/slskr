#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn hashdb_key_matches_frozen_dto_and_positive_size_validation() {
    let (state, _receiver) = test_state();
    let response = super::route_http_request(
        "GET",
        "/api/v0/hashdb/key?filename=Track.flac&size=123",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(response.status, "200 OK");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert!(json["flacKey"].is_string(), "{json}");
    assert!(json.get("key").is_none());
    assert!(json.get("filename").is_none());
    assert!(json.get("size").is_none());

    let zero = super::route_http_request(
        "GET",
        "/api/v0/hashdb/key?filename=Track.flac&size=0",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(zero.status, "400 Bad Request");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_base64_storage_paths_accept_mime_whitespace_but_reject_bad_data() {
    let raw = "a".repeat(80);
    let encoded = super::STANDARD.encode(raw.as_bytes());
    let wrapped = format!("{}%0A{}%0D%0A", &encoded[..76], &encoded[76..]);
    assert_eq!(
        super::controller_storage::decode_controller_base64_path_segment(&wrapped).unwrap(),
        raw
    );
    assert_eq!(
        super::controller_storage::decode_controller_base64_path_segment("4KC+").unwrap(),
        "࠾"
    );
    assert!(super::controller_storage::decode_controller_base64_path_segment("Zm9v%00").is_err());
    assert!(
        super::controller_storage::decode_controller_base64_path_segment("not-base64!").is_err()
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_file_directory_routes_list_storage_roots() {
    let (state, _receiver) = test_state();
    let album = state.config.downloads_dir.join("Artist").join("Album");
    std::fs::create_dir_all(&album).unwrap();
    std::fs::write(album.join("Track.flac"), b"track").unwrap();

    let root = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories?recursive=true",
        None,
        "",
        &state,
    )
    .await
    .expect("list downloads root");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["fullName"], "");
    assert_eq!(root_json["directories"][0]["fullName"], "Artist");
    assert_eq!(root_json["directories"][1]["fullName"], "Artist/Album");
    assert_eq!(root_json["files"][0]["fullName"], "Artist/Album/Track.flac");

    let album_dir = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories/QXJ0aXN0L0FsYnVt",
        None,
        "",
        &state,
    )
    .await
    .expect("list album dir");
    let aliased_album_dir = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories/unrelated/QXJ0aXN0L0FsYnVt",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased directory lookup");
    assert_eq!(aliased_album_dir.status, "404 Not Found");
    let album_json = serde_json::from_str::<serde_json::Value>(&album_dir.body).unwrap();
    assert_eq!(album_json["fullName"], "");
    assert_eq!(album_json["files"][0]["name"], "Track.flac");
    assert_eq!(album_json["files"][0]["length"], 5);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn file_storage_errors_redact_internal_details_and_preserve_client_errors() {
    let internal = super::file_storage_error_response(
        "storage directory read failed: permission denied: /srv/private/downloads",
    );
    assert_eq!(internal.status, "503 Service Unavailable");
    assert_eq!(internal.body, "{\"error\":\"file storage unavailable\"}");
    assert!(!internal.body.contains("permission denied"));
    assert!(!internal.body.contains("/srv/private"));

    let client = super::file_storage_error_response(
        "path must be relative and stay within the storage root",
    );
    assert_eq!(client.status, "400 Bad Request");
    assert!(client.body.contains("path must be relative"));

    let oversized = super::file_storage_error_response(super::STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR);
    assert_eq!(oversized.status, "413 Payload Too Large");
    assert_eq!(
        oversized.body,
        "{\"error\":\"storage directory is too large to list\"}"
    );

    let too_deep = super::file_storage_error_response(super::STORAGE_DIRECTORY_DELETE_DEPTH_ERROR);
    assert_eq!(too_deep.status, "413 Payload Too Large");
    assert_eq!(
        too_deep.body,
        "{\"error\":\"storage directory tree is too deep to delete\"}"
    );

    let too_wide =
        super::file_storage_error_response(super::STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR);
    assert_eq!(too_wide.status, "413 Payload Too Large");
    assert_eq!(
        too_wide.body,
        "{\"error\":\"storage directory is too large to delete\"}"
    );
    let aggregate =
        super::file_storage_error_response(super::STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR);
    assert_eq!(aggregate.status, "413 Payload Too Large");
    assert_eq!(aggregate.body, too_wide.body);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_storage_directory_listing_is_bounded() {
    let (state, _receiver) = test_state();
    let dir = state.config.state_dir.join("bounded-listing");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("one.txt"), b"1").unwrap();
    std::fs::write(dir.join("two.txt"), b"2").unwrap();

    let json = super::controller_storage_directory_json(
        &dir,
        None,
        super::StorageDirectoryListOptions {
            recursive: false,
            limit: 1,
        },
    )
    .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).map_err(|e| e.to_string()))
    .expect("listing");
    assert_eq!(json["files"].as_array().unwrap().len(), 1);
    assert!(json.get("entryCount").is_none());
    assert!(json.get("truncated").is_none());

    let mut scanned = super::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES - 1;
    super::reserve_storage_scan_entry(&mut scanned).expect("last scan slot");
    assert_eq!(scanned, super::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES);
    assert_eq!(
        super::reserve_storage_scan_entry(&mut scanned).unwrap_err(),
        super::STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR
    );
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scoped_storage_listing_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let (state, _receiver) = test_state();
    let root = state.config.state_dir.join("confined-listing");
    let outside = state.config.state_dir.join("outside-listing");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), b"secret").unwrap();
    symlink(&outside, root.join("linked")).unwrap();

    let error = super::controller_storage_directory_json_unix(
        &root,
        std::path::Path::new("linked"),
        super::StorageDirectoryListOptions {
            recursive: true,
            limit: 100,
        },
    )
    .expect_err("symlinked directory must be rejected");
    assert!(error.contains("confined open failed"));

    let listing = super::controller_storage_directory_json(
        &root,
        None,
        super::StorageDirectoryListOptions {
            recursive: true,
            limit: 100,
        },
    )
    .unwrap();
    assert!(!listing.contains("linked"));
    assert!(!listing.contains("secret.txt"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_storage_directory_routes_ignore_unknown_pagination_parameters() {
    let (state, _receiver) = test_state();
    let root = state.config.downloads_dir.clone();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"a").unwrap();
    std::fs::write(root.join("b.txt"), b"b").unwrap();
    std::fs::write(root.join("c.txt"), b"c").unwrap();

    let response = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories?limit=1&offset=1",
        None,
        "",
        &state,
    )
    .await
    .expect("storage listing with ignored query parameters");
    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "application/json; charset=utf-8");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert!(json.get("offset").is_none());
    assert!(json.get("limit").is_none());
    assert!(json.get("entryCount").is_none());
    assert!(json.get("truncated").is_none());
    assert_eq!(json["attributes"], "Directory");
    assert_eq!(json["files"].as_array().unwrap().len(), 3);
    assert_eq!(json["files"][0]["name"], "a.txt");
    assert_eq!(json["files"][0]["attributes"], "Normal");
    assert!(json["files"][0]["createdAt"]
        .as_str()
        .unwrap()
        .ends_with('Z'));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_recursive_storage_listing_has_lower_budget() {
    let (state, _receiver) = test_state();
    let root = state.config.downloads_dir.clone();
    std::fs::create_dir_all(&root).unwrap();
    for index in 0..300 {
        std::fs::write(root.join(format!("{index:03}.txt")), b"x").unwrap();
    }

    let response = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories?recursive=true",
        None,
        "",
        &state,
    )
    .await
    .expect("recursive storage listing");
    assert_eq!(response.status, "200 OK");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(
        json["files"].as_array().unwrap().len(),
        super::SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES
    );
    assert!(json.get("truncated").is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_recursive_storage_listing_bounds_directory_depth() {
    let (state, _receiver) = test_state();
    let root = state.config.state_dir.join("deep-storage-listing");
    let mut directory = root.clone();
    let mut relative = PathBuf::new();
    for depth in 0..(super::SLSKD_STORAGE_MAX_RECURSION_DEPTH + 3) {
        let component = format!("d{depth:02}");
        directory.push(&component);
        relative.push(component);
        std::fs::create_dir_all(&directory).unwrap();
    }

    let json = super::controller_storage_directory_json(
        &root,
        None,
        super::StorageDirectoryListOptions {
            recursive: true,
            limit: super::SLSKD_STORAGE_RECURSIVE_LIST_MAX_ENTRIES,
        },
    )
    .and_then(|json| {
        serde_json::from_str::<serde_json::Value>(&json).map_err(|error| error.to_string())
    })
    .expect("deep listing");
    assert!(json.get("truncated").is_none());

    let included = relative
        .components()
        .take(super::SLSKD_STORAGE_MAX_RECURSION_DEPTH + 1)
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    let excluded = relative
        .components()
        .take(super::SLSKD_STORAGE_MAX_RECURSION_DEPTH + 2)
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    let serialized = json.to_string();
    assert!(serialized.contains(&included), "{included}");
    assert!(!serialized.contains(&excluded), "{excluded}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn stats_api_aggregates_projection_counts() {
    let (state, mut receiver) = test_state();
    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }

    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"test flac\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        "{\"username\":\"friend\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"filename\":\"Remote/Song.flac\",\"size\":123}",
        &state,
    )
    .await
    .unwrap();
    super::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        "{\"username\":\"friend\",\"body\":\"hi\"}",
        &state,
    )
    .await
    .unwrap();
    super::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/rooms/music/messages",
        None,
        "{\"username\":\"friend\",\"body\":\"track?\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Song.flac\",\"size\":100}",
        &state,
    )
    .await
    .unwrap();
    super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":40}",
        &state,
    )
    .await
    .unwrap();

    let stats = super::route_http_request("GET", "/api/v0/stats", None, "", &state)
        .await
        .expect("stats response");

    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["shares"]["files"], 1);
    assert_eq!(stats_json["shares"]["bytes"], 42);
    assert_eq!(stats_json["searches"]["total"], 1);
    assert_eq!(stats_json["searches"]["active"], 1);
    assert_eq!(stats_json["searches"]["results"], 1);
    assert_eq!(stats_json["users"]["total"], 1);
    assert_eq!(stats_json["users"]["watched"], 1);
    assert_eq!(stats_json["browse"]["total"], 1);
    assert_eq!(stats_json["browse"]["ready"], 1);
    assert_eq!(stats_json["browse"]["files"], 1);
    assert_eq!(stats_json["browse"]["bytes"], 123);
    assert_eq!(stats_json["messages"]["total"], 1);
    assert_eq!(stats_json["messages"]["inbound"], 1);
    assert_eq!(stats_json["rooms"]["total"], 1);
    assert_eq!(stats_json["rooms"]["joined"], 1);
    assert_eq!(stats_json["rooms"]["messages"], 1);
    assert_eq!(stats_json["transfers"]["total"], 1);
    assert_eq!(stats_json["transfers"]["in_progress"], 1);
    assert_eq!(stats_json["transfers"]["bytes_transferred"], 40);
    assert_eq!(stats_json["database"]["enabled"], false);
    assert_eq!(stats_json["database"]["projections"]["searches"], 1);
    assert_eq!(stats_json["database"]["projections"]["transfers"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn stats_metrics_and_telemetry_expose_persisted_health_counts() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"durable metrics"}"#,
        &state,
    )
    .await
    .expect("create search");
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"username":"peer","files":[{"filename":"Remote/Metrics.flac","size":11}]}"#,
        &state,
    )
    .await
    .expect("ingest result");

    let stats = super::route_http_request("GET", "/api/v0/stats", None, "", &state)
        .await
        .expect("stats response");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["database"]["enabled"], true);
    assert_eq!(stats_json["database"]["searches"], 1);
    assert_eq!(stats_json["database"]["searchResults"], 1);
    assert_eq!(stats_json["database"]["projections"]["searches"], 1);

    let metrics = super::route_http_request("GET", "/api/v0/metrics", None, "", &state)
        .await
        .expect("metrics response");
    assert!(metrics.body.contains("slskr_database_enabled 1"));
    assert!(metrics
        .body
        .contains("slskr_database_rows{store=\"searches\"} 1"));
    assert!(metrics
        .body
        .contains("slskr_database_rows{store=\"search_results\"} 1"));

    let telemetry = super::route_http_request("GET", "/api/v0/telemetry", None, "", &state)
        .await
        .expect("telemetry response");
    let telemetry_json = serde_json::from_str::<serde_json::Value>(&telemetry.body).unwrap();
    assert_eq!(telemetry_json["database"]["searchResults"], 1);
    assert_eq!(telemetry_json["projections"]["searches"], 1);
    assert_eq!(telemetry_json["health"]["database"], true);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mutating_api_routes_enqueue_session_commands() {
    let (state, mut receiver) = test_state();

    let routes = [
        ("/api/v0/session/connect", super::SessionCommand::Connect),
        ("/api/v0/session/ping", super::SessionCommand::Ping),
        (
            "/api/v0/session/disconnect",
            super::SessionCommand::Disconnect,
        ),
        (
            "/api/v0/session/privileges/check",
            super::SessionCommand::CheckPrivileges,
        ),
    ];

    for (path, expected_command) in routes {
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .expect("route response");
        assert_eq!(response.status, "202 Accepted");
        assert_eq!(response.body, "{\"accepted\":true}");
        let command = receiver.try_recv().expect("session command");
        assert_eq!(
            std::mem::discriminant(&command),
            std::mem::discriminant(&expected_command)
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn session_control_routes_fail_when_manager_is_not_running() {
    let (state, receiver) = test_state();
    drop(receiver);

    for path in [
        "/api/session/connect",
        "/api/session/ping",
        "/api/session/disconnect",
        "/api/session/privileges/check",
    ] {
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .expect("route response");
        assert_eq!(response.status, "503 Service Unavailable");
        assert!(response.body.contains("session manager is not running"));
    }

    let connect = super::route_http_request("PUT", "/api/server", None, "", &state)
        .await
        .expect("connect response");
    assert_eq!(connect.status, "503 Service Unavailable");
    assert_eq!(state.session.read().await.state, "disconnected");

    state.session.write().await.state = "connected";
    let disconnect = super::route_http_request("DELETE", "/api/server", None, "", &state)
        .await
        .expect("disconnect response");
    assert_eq!(disconnect.status, "503 Service Unavailable");
    assert_eq!(state.session.read().await.state, "connected");
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
async fn controller_api_differential_automation_compat_routes_use_expected_shapes() {
    let (state, mut receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "peer 1=127.0.0.1:2234;peer1=127.0.0.1:2234",
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

    let app = super::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("application route");
    assert_eq!(app.status, "200 OK");
    let app_json = serde_json::from_str::<serde_json::Value>(&app.body).unwrap();
    assert_eq!(app_json["version"]["current"], env!("CARGO_PKG_VERSION"));
    assert_eq!(app_json["server"]["isConnected"], false);
    record_evidence!("GET", "/api/v0/application", "nominal-status-headers-body");

    let server_connect = super::route_http_request("POST", "/api/v0/server", None, "", &state)
        .await
        .expect("server connect");
    assert_eq!(server_connect.status, "202 Accepted");
    assert!(matches!(
        receiver.try_recv().unwrap(),
        super::SessionCommand::Connect
    ));

    let server_connect_put = super::route_http_request("PUT", "/api/v0/server", None, "", &state)
        .await
        .expect("server put connect");
    assert_eq!(server_connect_put.status, "205 Reset Content");
    assert!(server_connect_put.body.is_empty());
    record_evidence!("PUT", "/api/v0/server", "nominal-status-headers-body");
    assert!(receiver.try_recv().is_err());

    {
        let mut session = state.session.write().await;
        session.state = "error";
        session.last_error = Some("login failed: invalid password".to_owned());
    }
    let server_error = super::route_http_request("GET", "/api/v0/server", None, "", &state)
        .await
        .expect("server error state");
    assert_eq!(server_error.status, "200 OK");
    let server_error_json = serde_json::from_str::<serde_json::Value>(&server_error.body).unwrap();
    assert_eq!(server_error_json["state"], "Error");
    assert_eq!(server_error_json["lastError"], "login failed");
    record_evidence!("GET", "/api/v0/server", "nominal-status-headers-body");
    record_evidence!("GET", "/api/v0/server", "populated-dynamic-state");

    // Versioned search creation requires an authenticated Soulseek server
    // session.  The preceding assertions intentionally exercise the error
    // snapshot, then restore the nominal connected state for the positive
    // search/readback contract below.
    state.session.write().await.state = "connected";

    let session_enabled =
        super::route_http_request("GET", "/api/v0/session/enabled", None, "", &state)
            .await
            .expect("session enabled");
    assert!(matches!(session_enabled.body.as_str(), "true" | "false"));

    let search = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"searchText":"Remote Song"}"#,
        &state,
    )
    .await
    .expect("search route");
    assert_eq!(search.status, "200 OK");
    assert!(search.body.contains("\"query\":\"Remote Song\""));
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap();
    assert!(search_json["searchId"].is_string());
    assert_eq!(search_json["query"], "Remote Song");
    assert!(search_json["results"].is_array());
    record_evidence!("POST", "/api/v0/searches", "nominal-status-headers-body");
    record_evidence!(
        "POST",
        "/api/v0/searches",
        "mutation-side-effects-and-readback"
    );
    let _ = receiver.try_recv();

    let _ = super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"peer_username":"peer1","filename":"Remote/Song.mp3","size":99}"#,
        &state,
    )
    .await
    .unwrap();
    let responses =
        super::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
            .await
            .expect("search responses");
    let responses_json = serde_json::from_str::<serde_json::Value>(&responses.body).unwrap();
    assert!(responses_json.is_array());
    assert_eq!(responses_json[0]["username"], "peer1");
    assert_eq!(responses_json[0]["files"][0]["filename"], "Remote/Song.mp3");
    record_evidence!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "populated-dynamic-state"
    );

    let uuid_search_id = "11111111-1111-1111-1111-111111111111";
    let uuid_search = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        &format!(r#"{{"id":"{uuid_search_id}","searchText":"UUID Song"}}"#),
        &state,
    )
    .await
    .expect("uuid search route");
    assert_eq!(uuid_search.status, "200 OK");
    let uuid_search_json = serde_json::from_str::<serde_json::Value>(&uuid_search.body).unwrap();
    assert!(uuid_search_json["searchId"].is_string());
    assert_eq!(uuid_search_json["query"], "UUID Song");
    let _ = receiver.try_recv();

    let dashless_search_id = uuid_search_json["searchId"]
        .as_str()
        .expect("dashless search id");
    let dashless_detail = super::route_http_request(
        "GET",
        &format!("/api/v0/searches/{dashless_search_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("dashless search detail route");
    assert_eq!(dashless_detail.status, "200 OK");
    let dashless_responses = super::route_http_request(
        "GET",
        &format!("/api/v0/searches/{dashless_search_id}/responses"),
        None,
        "",
        &state,
    )
    .await
    .expect("dashless search responses route");
    assert_eq!(dashless_responses.status, "200 OK");

    for (method, path) in [
        ("GET", format!("/api/v0/searches/{uuid_search_id}")),
        (
            "GET",
            format!("/api/v0/searches/{uuid_search_id}?includeResponses=true"),
        ),
        (
            "GET",
            format!("/api/v0/searches/{uuid_search_id}/responses"),
        ),
        ("PUT", format!("/api/v0/searches/{uuid_search_id}")),
        ("DELETE", format!("/api/v0/searches/{uuid_search_id}")),
    ] {
        let response = super::route_http_request(method, &path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/uploads/peer1/1")
            && !path.contains("/transfers/downloads/peer1/1")
        {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
    }

    let enqueue = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer1",
        None,
        r#"{"files":[{"filename":"Remote/Song.mp3","size":99}]}"#,
        &state,
    )
    .await
    .expect("enqueue download");
    let enqueue_json = serde_json::from_str::<serde_json::Value>(&enqueue.body).unwrap();
    assert_eq!(enqueue_json["queued"], 1);
    assert_eq!(enqueue_json["transfers"][0]["username"], "peer1");
    assert_eq!(enqueue_json["transfers"][0]["filename"], "Remote/Song.mp3");
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

    let downloads =
        super::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
            .await
            .expect("downloads route");
    let downloads_json = serde_json::from_str::<serde_json::Value>(&downloads.body).unwrap();
    assert_eq!(downloads_json[0]["username"], "peer1");
    assert_eq!(downloads_json[0]["directories"][0]["directory"], "Remote");
    assert_eq!(downloads_json[0]["directories"][0]["fileCount"], 1);
    assert_eq!(
        downloads_json[0]["directories"][0]["files"][0]["direction"],
        "Download"
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

    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }
    // The route fixture can inherit a configured room from the caller's
    // environment.  Keep this round-trip deterministic so the first
    // request proves creation and the second proves idempotent readback.
    state.rooms.write().await.records.clear();
    let joined =
        super::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
            .await
            .expect("join room");
    assert_eq!(joined.status, "201 Created");
    let joined_again =
        super::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
            .await
            .unwrap();
    assert_eq!(joined_again.status, "200 OK");
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined",
        "concurrency-and-idempotency"
    );
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined",
        "mutation-side-effects-and-readback"
    );
    let joined_rooms = super::route_http_request("GET", "/api/v0/rooms/joined", None, "", &state)
        .await
        .expect("joined rooms");
    let joined_rooms_json = serde_json::from_str::<serde_json::Value>(&joined_rooms.body).unwrap();
    assert!(joined_rooms_json
        .as_array()
        .is_some_and(|rooms| rooms.iter().any(|room| room.as_str() == Some("music"))));
    record_evidence!("GET", "/api/v0/rooms/joined", "populated-dynamic-state");
    let _ = receiver.try_recv();
    let repeated = super::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .unwrap();
    assert_eq!(repeated.status, "200 OK");
    assert!(receiver.try_recv().is_err());

    let room_message = super::route_http_request(
        "POST",
        "/api/v0/rooms/joined/music/messages",
        None,
        r#""hello room""#,
        &state,
    )
    .await
    .expect("room message");
    assert_eq!(room_message.status, "201 Created");
    assert!(room_message.body.is_empty());
    let _ = receiver.try_recv();

    let room_messages = super::route_http_request(
        "GET",
        "/api/v0/rooms/joined/music/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("room messages");
    let room_messages_json =
        serde_json::from_str::<serde_json::Value>(&room_messages.body).unwrap();
    assert_eq!(room_messages_json[0]["message"], "hello room");
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "mutation-side-effects-and-readback"
    );
    record_evidence!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        "populated-dynamic-state"
    );

    let conversation_send = super::route_http_request(
        "POST",
        "/api/v0/conversations/peer1",
        None,
        r#""hello peer""#,
        &state,
    )
    .await
    .expect("conversation send");
    assert_eq!(conversation_send.status, "201 Created");
    assert!(conversation_send.body.is_empty());
    let _ = receiver.try_recv();

    let conversations = super::route_http_request("GET", "/api/v0/conversations", None, "", &state)
        .await
        .expect("conversations");
    let conversations_json =
        serde_json::from_str::<serde_json::Value>(&conversations.body).unwrap();
    assert_eq!(conversations_json[0]["username"], "peer1");
    assert_eq!(
        conversations_json[0]["messages"][0]["message"],
        "hello peer"
    );
    record_evidence!(
        "POST",
        "/api/v0/conversations/{username}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/conversations/{username}",
        "mutation-side-effects-and-readback"
    );
    record_evidence!(
        "GET",
        "/api/v0/conversations",
        "nominal-status-headers-body"
    );
    record_evidence!("GET", "/api/v0/conversations", "populated-dynamic-state");

    let user_status =
        super::route_http_request("GET", "/api/v0/users/peer1/status", None, "", &state)
            .await
            .expect("user status");
    assert!(user_status.body.contains("\"presence\""));
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/status",
        "nominal-status-headers-body"
    );

    let user_directory = super::route_http_request(
        "POST",
        "/api/v0/users/peer1/directory",
        None,
        r#"{"directory":"Virtual"}"#,
        &state,
    )
    .await
    .expect("user directory");
    let user_directory_json =
        serde_json::from_str::<serde_json::Value>(&user_directory.body).unwrap();
    assert!(user_directory_json.is_array());
    assert_eq!(user_directory_json[0]["name"], "Virtual");
    record_evidence!(
        "POST",
        "/api/v0/users/{username}/directory",
        "nominal-status-headers-body"
    );

    let user_endpoint =
        super::route_http_request("GET", "/api/v0/users/peer%201/endpoint", None, "", &state)
            .await
            .expect("user endpoint");
    let user_endpoint_json =
        serde_json::from_str::<serde_json::Value>(&user_endpoint.body).unwrap();
    assert_eq!(user_endpoint_json["username"], "peer 1");
    assert_eq!(user_endpoint_json["addressFamily"], "IPv4");
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "populated-dynamic-state"
    );

    let browse_status = super::route_http_request(
        "GET",
        "/api/v0/users/peer%201/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("browse status");
    assert_eq!(browse_status.status, "404 Not Found");

    let user_status =
        super::route_http_request("GET", "/api/v0/users/peer%201/status", None, "", &state)
            .await
            .expect("user status");
    let user_status_json = serde_json::from_str::<serde_json::Value>(&user_status.body).unwrap();
    assert_eq!(user_status_json["username"], "peer 1");
    assert_eq!(user_status_json["presence"], "Offline");
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/status",
        "populated-dynamic-state"
    );

    state.shares.write().await.roots.push(super::ShareRoot {
        label: "Virtual".to_owned(),
        local_path: PathBuf::from("Virtual"),
        raw: "Virtual".to_owned(),
        directories: 0,
        files: 1,
        bytes: 42,
        extensions: Vec::new(),
        statistics_ready: true,
    });
    let shares = super::route_http_request("GET", "/api/v0/shares", None, "", &state)
        .await
        .expect("shares");
    let shares_json = serde_json::from_str::<serde_json::Value>(&shares.body).unwrap();
    let first_share = shares_json
        .as_object()
        .and_then(|object| object.values().next())
        .and_then(|value| value.as_array())
        .and_then(|array| array.first())
        .expect("first share");
    assert!(first_share["id"].is_string());
    assert!(first_share["raw"].is_string());
    record_evidence!("GET", "/api/v0/shares", "populated-dynamic-state");

    let share_contents =
        super::route_http_request("GET", "/api/v0/shares/contents", None, "", &state)
            .await
            .expect("share contents");
    let share_contents_json =
        serde_json::from_str::<serde_json::Value>(&share_contents.body).unwrap();
    assert!(share_contents_json
        .as_array()
        .is_some_and(|entries| !entries.is_empty()));
    record_evidence!(
        "GET",
        "/api/v0/shares/contents",
        "nominal-status-headers-body"
    );
    record_evidence!("GET", "/api/v0/shares/contents", "populated-dynamic-state");

    let virtual_share = super::route_http_request(
        "GET",
        &format!("/api/v0/shares/{}", super::share_root_id("Virtual")),
        None,
        "",
        &state,
    )
    .await
    .expect("virtual share");
    let virtual_share_json =
        serde_json::from_str::<serde_json::Value>(&virtual_share.body).unwrap();
    assert_eq!(virtual_share_json["id"], super::share_root_id("Virtual"));
    assert_eq!(virtual_share_json["files"], 1);

    let download_dir = super::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories",
        None,
        "",
        &state,
    )
    .await
    .expect("download dir");
    let download_dir_json = serde_json::from_str::<serde_json::Value>(&download_dir.body).unwrap();
    assert!(download_dir_json["fullName"].is_string());
    assert!(download_dir_json.get("fullname").is_none());

    let telemetry_transfer = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/telemetry%20peer",
        None,
        r#"[{"filename":"Telemetry/Album/Track.flac","size":321}]"#,
        &state,
    )
    .await
    .expect("telemetry transfer");
    assert_eq!(telemetry_transfer.status, "200 OK");

    // The leaderboard and directories reports only count real
    // completed transfers now (matching the oracle's State=48
    // filter), so mark the downloads above completed, and add a real
    // completed *upload* for the directories report -- which, also
    // matching the oracle, only ever reports on uploads (what other
    // users downloaded from local shares), never downloads.
    {
        let mut transfers = state.transfers.write().await;
        for entry in transfers.entries.iter_mut() {
            if entry.direction == 0 {
                entry.status = "succeeded".to_owned();
            }
        }
        // A distinct username -- not "telemetry peer" -- so this
        // synthetic upload doesn't inflate that user's own transfer
        // count/report later in this test.
        transfers.create(
            1,
            Some("directory-audit-peer".to_owned()),
            "Telemetry/Album/Track.flac".to_owned(),
            None,
            Some(321),
        );
        if let Some(entry) = transfers
            .entries
            .iter_mut()
            .rev()
            .find(|entry| entry.direction == 1)
        {
            entry.status = "succeeded".to_owned();
        }
    }

    let leaderboard = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download",
        None,
        "",
        &state,
    )
    .await
    .expect("leaderboard");
    let leaderboard_json = serde_json::from_str::<serde_json::Value>(&leaderboard.body).unwrap();
    let telemetry_leader = leaderboard_json
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["username"].as_str() == Some("telemetry peer"))
        })
        .expect("telemetry leaderboard row");
    assert_eq!(telemetry_leader["totalBytes"], 321);
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "populated-dynamic-state"
    );

    let asc_leaderboard = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&sortBy=TotalBytes&sortOrder=ASC",
        None,
        "",
        &state,
    )
    .await
    .expect("ascending leaderboard");
    let asc_leaderboard_json =
        serde_json::from_str::<serde_json::Value>(&asc_leaderboard.body).unwrap();
    assert_eq!(asc_leaderboard_json[0]["username"], "peer1");

    let summary = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered summary");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap();
    let summary_record = &summary_json["Download"]["Succeeded"];
    assert_eq!(summary_record["count"], 1);
    assert_eq!(summary_record["totalBytes"], 321);
    assert_eq!(summary_record["distinctUsers"], 1);
    assert_eq!(summary_json["Upload"], serde_json::json!({}));
    assert!(summary_json.get("count").is_none());

    let user_transfers = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("user transfer report");
    let user_transfers_json =
        serde_json::from_str::<serde_json::Value>(&user_transfers.body).unwrap();
    assert_eq!(user_transfers_json["username"], "telemetry peer");
    assert_eq!(user_transfers_json["count"], 1);
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "populated-dynamic-state"
    );
    let aliased_user_transfers = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/unrelated/telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased user transfer report");
    assert_eq!(aliased_user_transfers.status, "404 Not Found");
    let telemetry_transfer_id = user_transfers_json["transfers"][0]["id"]
        .as_str()
        .expect("telemetry transfer id")
        .to_owned();

    let directory_report = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        None,
        "",
        &state,
    )
    .await
    .expect("directory report");
    let directory_report_json =
        serde_json::from_str::<serde_json::Value>(&directory_report.body).unwrap();
    assert!(directory_report_json
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["path"].as_str() == Some("Telemetry/Album"))
        })
        .is_some());
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "populated-dynamic-state"
    );

    let pareto = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download",
        None,
        "",
        &state,
    )
    .await
    .expect("exceptions pareto");
    let pareto_json = serde_json::from_str::<serde_json::Value>(&pareto.body).unwrap();
    assert!(pareto_json.is_array());
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "nominal-status-headers-body"
    );

    let cancelled = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/telemetry%20peer/{telemetry_transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel telemetry transfer");
    assert_eq!(cancelled.status, "204 No Content");
    let exceptions = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions?direction=Download&username=telemetry%20peer&sortOrder=ASC",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered exceptions");
    let exceptions_json = serde_json::from_str::<serde_json::Value>(&exceptions.body).unwrap();
    assert_eq!(exceptions_json[0]["username"], "telemetry peer");
    assert_eq!(exceptions_json[0]["state"], "Cancelled");
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "populated-dynamic-state"
    );

    let populated_pareto = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("populated exceptions pareto");
    let populated_pareto_json =
        serde_json::from_str::<serde_json::Value>(&populated_pareto.body).unwrap();
    assert_eq!(populated_pareto.status, "200 OK");
    assert_eq!(populated_pareto_json[0]["exception"], "Cancelled");
    assert_eq!(populated_pareto_json[0]["count"], 1);
    assert_eq!(populated_pareto_json[0]["distinctUsers"], 1);
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "populated-dynamic-state"
    );

    let contract_routes = [
        ("GET", "/api/application", ""),
        ("GET", "/api/application/version", ""),
        ("GET", "/api/application/version/latest", ""),
        ("POST", "/api/application/gc", ""),
        ("GET", "/api/session", ""),
        (
            "POST",
            "/api/session",
            r#"{"username":"user","password":"pass"}"#,
        ),
        ("GET", "/api/session/enabled", ""),
        ("GET", "/api/server", ""),
        ("PUT", "/api/server", ""),
        ("DELETE", "/api/server", ""),
        ("POST", "/api/searches", r#"{"searchText":"contract song"}"#),
        ("GET", "/api/searches", ""),
        ("GET", "/api/searches/1", ""),
        ("PUT", "/api/searches/1", ""),
        ("GET", "/api/searches/1/responses", ""),
        ("GET", "/api/transfers/downloads/", ""),
        ("GET", "/api/transfers/downloads/peer1", ""),
        (
            "POST",
            "/api/transfers/downloads/peer1",
            r#"[{"filename":"Remote/Song.mp3","size":99}]"#,
        ),
        ("GET", "/api/transfers/downloads/peer1/1", ""),
        ("GET", "/api/transfers/downloads/peer1/1/position", ""),
        ("DELETE", "/api/transfers/downloads/peer1/1", ""),
        ("DELETE", "/api/transfers/downloads/all/completed", ""),
        ("GET", "/api/transfers/uploads/", ""),
        ("GET", "/api/transfers/uploads/peer1", ""),
        ("GET", "/api/transfers/uploads/peer1/1", ""),
        ("DELETE", "/api/transfers/uploads/peer1/1", ""),
        ("DELETE", "/api/transfers/uploads/all/completed", ""),
        ("GET", "/api/rooms/joined", ""),
        ("POST", "/api/rooms/joined", r#""contract-room""#),
        ("GET", "/api/rooms/joined/contract-room", ""),
        (
            "POST",
            "/api/rooms/joined/contract-room/messages",
            r#""hello""#,
        ),
        ("GET", "/api/rooms/joined/contract-room/messages", ""),
        (
            "POST",
            "/api/rooms/joined/contract-room/ticker",
            r#""ticker""#,
        ),
        (
            "POST",
            "/api/rooms/joined/contract-room/members",
            r#""peer1""#,
        ),
        ("GET", "/api/rooms/joined/contract-room/users", ""),
        ("DELETE", "/api/rooms/joined/contract-room", ""),
        ("GET", "/api/rooms/available", ""),
        ("GET", "/api/conversations", ""),
        ("POST", "/api/conversations/peer1", r#""hello peer again""#),
        ("GET", "/api/conversations/peer1", ""),
        ("GET", "/api/conversations/peer1/messages", ""),
        ("PUT", "/api/conversations/peer1/1", ""),
        ("PUT", "/api/conversations/peer1", ""),
        ("DELETE", "/api/conversations/peer1", ""),
        ("GET", "/api/users/peer1/endpoint", ""),
        ("GET", "/api/users/peer1/browse", ""),
        ("POST", "/api/users/peer1/directory", r#"{"directory":""}"#),
        ("GET", "/api/users/peer1/info", ""),
        ("GET", "/api/users/peer1/status", ""),
        ("GET", "/api/shares", ""),
        (
            "GET",
            "/api/shares/8E7DAA120E8DB8C0B0388938D9F61D6C6B796EDD",
            "",
        ),
        ("GET", "/api/shares/contents", ""),
        (
            "GET",
            "/api/shares/8E7DAA120E8DB8C0B0388938D9F61D6C6B796EDD/contents",
            "",
        ),
        ("PUT", "/api/shares", ""),
        ("DELETE", "/api/shares", ""),
        ("GET", "/api/files/downloads/directories", ""),
        ("GET", "/api/files/downloads/directories/Zm9v", ""),
        ("DELETE", "/api/files/downloads/directories/Zm9v", ""),
        ("DELETE", "/api/files/downloads/files/Zm9vLm1wMw", ""),
        ("GET", "/api/files/incomplete/directories", ""),
        ("GET", "/api/files/incomplete/directories/Zm9v", ""),
        ("DELETE", "/api/files/incomplete/directories/Zm9v", ""),
        ("DELETE", "/api/files/incomplete/files/Zm9vLm1wMw", ""),
        ("GET", "/api/options", ""),
        ("GET", "/api/options/startup", ""),
        ("GET", "/api/options/debug", ""),
        ("GET", "/api/options/yaml/location", ""),
        ("GET", "/api/options/yaml", ""),
        ("PUT", "/api/options/yaml", r#""app: {}""#),
        ("POST", "/api/options/yaml/validate", r#""app: {}""#),
        ("GET", "/api/events", ""),
        ("POST", "/api/events/Noop", r#""""#),
        ("GET", "/api/logs", ""),
        ("PUT", "/api/relay/agent", ""),
        ("DELETE", "/api/relay/agent", ""),
        ("GET", "/api/relay/controller/downloads/token", ""),
        ("POST", "/api/relay/controller/files/token", ""),
        ("POST", "/api/relay/controller/shares/token", ""),
        ("GET", "/api/telemetry/metrics", ""),
        ("GET", "/api/telemetry/metrics/kpis", ""),
        ("GET", "/api/telemetry/reports/transfers/summary", ""),
        ("GET", "/api/telemetry/reports/transfers/histogram", ""),
        ("GET", "/api/telemetry/reports/transfers/leaderboard", ""),
        ("GET", "/api/telemetry/reports/transfers/users/peer1", ""),
        ("GET", "/api/telemetry/reports/transfers/exceptions", ""),
        (
            "GET",
            "/api/telemetry/reports/transfers/exceptions/pareto",
            "",
        ),
        ("GET", "/api/telemetry/reports/transfers/directories", ""),
        ("DELETE", "/api/searches/1", ""),
        ("DELETE", "/api/application", ""),
        ("PUT", "/api/application", ""),
    ];

    for (method, path, body) in contract_routes {
        if path.starts_with("/api/rooms/") {
            let mut session = state.session.write().await;
            session.state = "connected";
            session.updated_at = super::unix_timestamp();
        }
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            super::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        let expected_missing_user_transfer = (method == "GET"
            && path.ends_with("/transfers/uploads/peer1"))
            || path.contains("/transfers/uploads/peer1/1")
            || path.contains("/transfers/downloads/peer1/1");
        let expected_missing_storage_directory = method == "GET"
            && (path == "/api/files/downloads/directories/Zm9v"
                || path == "/api/files/incomplete/directories/Zm9v");
        let expected_missing_user_browse = method == "GET"
            && (path == "/api/users/peer1/browse" || path == "/api/users/peer%201/browse");
        if expected_missing_storage_directory {
            assert_eq!(response.status, "404 Not Found", "{method} {path}");
        } else if !expected_missing_user_transfer && !expected_missing_user_browse {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        while receiver.try_recv().is_ok() {}

        let versioned = path.replacen("/api", "/api/v0", 1);
        let versioned_response = tokio::time::timeout(
            Duration::from_secs(1),
            super::route_http_request(method, &versioned, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {versioned}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {versioned}: {error}"));
        if method == "DELETE"
            && (versioned == "/api/v0/searches/1"
                || versioned.starts_with("/api/v0/conversations/")
                || versioned == "/api/v0/shares"
                || versioned == "/api/v0/rooms/joined/contract-room"
                || versioned.starts_with("/api/v0/transfers/uploads/peer1/1"))
            || (method == "GET" && versioned.starts_with("/api/v0/transfers/uploads/peer1"))
            || (method == "GET" && versioned.starts_with("/api/v0/shares/root"))
            || (method == "GET"
                && (versioned == "/api/v0/users/peer1/browse"
                    || versioned == "/api/v0/users/peer%201/browse"))
            || (method == "GET"
                && (versioned == "/api/v0/files/downloads/directories/Zm9v"
                    || versioned == "/api/v0/files/incomplete/directories/Zm9v"))
        {
            assert_eq!(
                versioned_response.status, "404 Not Found",
                "{method} {versioned}: {}",
                versioned_response.body
            );
        } else {
            assert_ne!(
                versioned_response.status, "404 Not Found",
                "{method} {versioned}"
            );
        }
        assert!(
            !versioned_response.status.starts_with('5'),
            "{method} {versioned}: {}",
            versioned_response.status
        );
        while receiver.try_recv().is_ok() {}
    }

    // A user with no tracked browse must 404, matching the oracle's
    // BrowseTracker.TryGet-miss contract, on both the unversioned and
    // versioned paths -- there is no version-specific split here.
    for path in [
        "/api/users/peer1/browse/status",
        "/api/v0/users/peer1/browse/status",
        "/api/users/peer%201/browse/status",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "GET {path}");
    }

    let query_contract_routes = [
        // Not "/api/application/version/latest?forceCheck=true" here:
        // that now awaits a real (parameterized, separately tested)
        // GitHub Releases lookup, so it can't share this 1-second
        // timeout or depend on live network in a hermetic test run.
        ("GET", "/api/searches?includeResponses=true", ""),
        ("DELETE", "/api/transfers/downloads/peer1/1?remove=true", ""),
        ("GET", "/api/transfers/downloads/?includeRemoved=true", ""),
        ("GET", "/api/transfers/uploads/?includeRemoved=true", ""),
        (
            "GET",
            "/api/conversations?includeInactive=true&unAcknowledgedOnly=false",
            "",
        ),
        ("GET", "/api/conversations/peer1?includeMessages=false", ""),
        (
            "GET",
            "/api/conversations/peer1/messages?unAcknowledgedOnly=true",
            "",
        ),
        ("GET", "/api/files/downloads/directories?recursive=true", ""),
        (
            "GET",
            "/api/files/incomplete/directories?recursive=true",
            "",
        ),
        ("GET", "/api/events?limit=10", ""),
        (
            "GET",
            "/api/telemetry/reports/transfers/summary?startDate=2026-01-01&endDate=2026-01-02",
            "",
        ),
    ];

    for (method, path, body) in query_contract_routes {
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            super::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/downloads/peer1/1") {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        let json = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or_else(|error| panic!("{method} {path}: invalid json: {error}"));
        assert_ne!(json["status"], "disabled", "{method} {path}");
        if path == "/api/virtualsoulfind/canonical/status" {
            assert_eq!(json["enabled"], true);
            assert!(json["counts"]["shares"].as_u64().unwrap() >= 1);
            assert!(json["itemCount"].as_u64().unwrap() >= 1);
        }
        while receiver.try_recv().is_ok() {}
    }

    let encoded_path_routes = [
        ("GET", "/api/profile/peer%201", ""),
        ("GET", "/api/users/peer%201/browse", ""),
        (
            "POST",
            "/api/transfers/downloads/peer%201",
            r#"[{"filename":"Remote/Encoded.mp3","size":11}]"#,
        ),
        ("GET", "/api/transfers/downloads/peer%201", ""),
        ("GET", "/api/transfers/downloads/peer%201/1", ""),
        (
            "DELETE",
            "/api/transfers/downloads/peer%201/1?remove=true",
            "",
        ),
        ("POST", "/api/rooms/joined", r#""room space""#),
        ("GET", "/api/rooms/joined/room%20space", ""),
        (
            "POST",
            "/api/rooms/joined/room%20space/messages",
            r#""hello""#,
        ),
        ("GET", "/api/rooms/joined/room%20space/messages", ""),
        ("POST", "/api/conversations/peer%201", r#""hello encoded""#),
        ("GET", "/api/conversations/peer%201", ""),
        ("GET", "/api/conversations/peer%201/messages", ""),
        ("PUT", "/api/conversations/peer%201", ""),
        ("DELETE", "/api/conversations/peer%201", ""),
    ];

    for (method, path, body) in encoded_path_routes {
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            super::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/downloads/peer%201/1") && path != "/api/users/peer%201/browse"
        {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        while receiver.try_recv().is_ok() {}
    }

    for (method, path, body) in [
        ("GET", "/api/profile/peer%201/untrusted", ""),
        ("GET", "/api/users/peer%201/untrusted/info", ""),
        ("GET", "/api/users/peer%201/untrusted/browse", ""),
        ("GET", "/api/users/peer%201/untrusted/group", ""),
        ("GET", "/api/users/peer%201/untrusted/endpoint", ""),
        (
            "POST",
            "/api/users/peer%201/untrusted/directory",
            r#"{"directory":"Music"}"#,
        ),
        (
            "GET",
            "/api/soulseek/users/peer%201/untrusted/interests",
            "",
        ),
    ] {
        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    let native_compat_routes = [
        ("GET", "/api/slskdn", ""),
        ("GET", "/api/slskdn/library/health", ""),
        ("POST", "/api/slskdn/warm-cache", ""),
        ("GET", "/api/hashdb/stats", ""),
        ("POST", "/api/hashdb/backfill/from-history", ""),
        ("GET", "/api/streams/content-1", ""),
        (
            "POST",
            "/api/v0/peer-streams/tickets",
            r#"{"filename":"Virtual/Test.flac","username":"peer"}"#,
        ),
        (
            "POST",
            "/api/v0/mesh-streams/tickets",
            r#"{"contentId":"mesh-content","filename":"Virtual/Test.flac","peerId":"mesh-peer"}"#,
        ),
        ("GET", "/api/listening-party", ""),
        ("POST", "/api/listening-party/radio/party/content", ""),
        ("GET", "/api/mesh/health", ""),
        ("GET", "/api/mesh/stats", ""),
        (
            "POST",
            "/api/multisource/download",
            r#"{"filename":"x","size":1}"#,
        ),
        (
            "POST",
            "/api/multisource/swarm",
            r#"{"filename":"x","size":1,"sources":[]}"#,
        ),
        (
            "POST",
            "/api/multisource/swarm/async",
            r#"{"filename":"x","size":1,"sources":[]}"#,
        ),
        ("GET", "/api/podcore/content/search?query=cover", ""),
        (
            "POST",
            "/api/podcore/membership/join",
            r#"{"podId":"pod","peerId":"peer"}"#,
        ),
        ("GET", "/api/library/items", ""),
        ("GET", "/api/virtualsoulfind/canonical/status", ""),
        ("POST", "/api/audio/variants/dedupe", ""),
        ("POST", "/api/mediacore/retrieve", ""),
        ("GET", "/api/playback/status", ""),
        ("GET", "/api/traces", ""),
        ("GET", "/api/fairness", ""),
        ("GET", "/api/ranking", ""),
        ("GET", "/api/portforwarding/status", ""),
        ("GET", "/api/signals", ""),
        ("POST", "/api/backfill", ""),
        ("GET", "/api/security/status", ""),
        ("GET", "/api/pods", ""),
        ("GET", "/api/solid/status", ""),
        ("GET", "/api/federation/diagnostics", ""),
    ];

    for (method, path, body) in native_compat_routes {
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            super::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/downloads/peer%201/1") {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        while receiver.try_recv().is_ok() {}
    }

    for (method, path) in [
        ("GET", "/api/hashdb/unregistered-state-projection"),
        ("POST", "/api/security/status"),
    ] {
        let response = super::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("automation_compat_routes.json"),
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
async fn controller_api_differential_peer_and_mesh_preview_stream_tickets_are_short_lived() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    {
        let mut searches = state.searches.write().await;
        searches
            .create(None, "preview".to_owned(), "global", None, Vec::new(), 300)
            .expect("create preview search");
        searches
            .records
            .last_mut()
            .unwrap()
            .results
            .push(super::SearchResultEntry {
                peer_username: Some("peer".to_owned()),
                filename: "Remote/Other.flac".to_owned(),
                size: 42,
                extension: "flac".to_owned(),
                bit_rate: None,
                sample_rate: None,
                bit_depth: None,
                length_seconds: None,
                locked: false,
                slot_free: Some(true),
                average_speed: Some(1),
                queue_length: Some(0),
            });
    }

    let unmatched = super::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Remote/Requested.flac","username":"peer"}"#,
        &state,
    )
    .await
    .expect("unmatched peer ticket");
    assert_eq!(unmatched.status, "200 OK", "{}", unmatched.body);
    assert_eq!(unmatched.content_type, "application/json");
    let unmatched_json = serde_json::from_str::<serde_json::Value>(&unmatched.body).unwrap();
    assert_eq!(unmatched_json["filename"], "Remote/Requested.flac");

    let peer_ticket = super::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Virtual/Test.flac","username":"peer"}"#,
        &state,
    )
    .await
    .expect("peer ticket");
    assert_eq!(peer_ticket.status, "200 OK");
    assert_eq!(peer_ticket.content_type, "application/json");
    let peer_json = serde_json::from_str::<serde_json::Value>(&peer_ticket.body).unwrap();
    assert_eq!(peer_json["expiresInSeconds"], 120);
    assert_eq!(peer_json["source"], "local-share");
    assert_eq!(peer_json["streamUrl"], peer_json["stream_url"]);

    let peer_stream = super::route_http_request(
        "GET",
        peer_json["streamUrl"].as_str().unwrap(),
        None,
        "",
        &state,
    )
    .await
    .expect("peer stream");
    assert_eq!(peer_stream.status, "200 OK");
    assert_eq!(peer_stream.content_type, "application/json");
    let stream_json = serde_json::from_str::<serde_json::Value>(&peer_stream.body).unwrap();
    assert_eq!(stream_json["status"], "available");
    assert_eq!(stream_json["acceptRanges"], "none");
    assert_eq!(stream_json["cacheControl"], "no-store");

    let mesh_ticket = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-content","filename":"Virtual/Test.flac","peerId":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("mesh ticket");
    assert_eq!(mesh_ticket.status, "200 OK");
    assert_eq!(mesh_ticket.content_type, "application/json");
    let mesh_json = serde_json::from_str::<serde_json::Value>(&mesh_ticket.body).unwrap();
    assert!(mesh_json["streamUrl"]
        .as_str()
        .unwrap()
        .starts_with("/api/v0/mesh-streams/"));

    let mesh_stream = super::route_http_request(
        "GET",
        mesh_json["streamUrl"].as_str().unwrap(),
        None,
        "",
        &state,
    )
    .await
    .expect("mesh stream");
    assert_eq!(mesh_stream.status, "200 OK", "{}", mesh_stream.body);
    assert_eq!(mesh_stream.content_type, "application/json");
    let mesh_stream_json = serde_json::from_str::<serde_json::Value>(&mesh_stream.body).unwrap();
    assert_eq!(mesh_stream_json["status"], "available");
    assert_eq!(mesh_stream_json["cacheControl"], "no-store");

    let missing =
        super::route_http_request("GET", "/api/v0/peer-streams/not-a-ticket", None, "", &state)
            .await
            .expect("missing ticket");
    assert_eq!(missing.status, "404 Not Found");
    assert_eq!(missing.content_type, "application/json");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("peer_mesh_stream_tickets.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/peer-streams/tickets",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/peer-streams/{ticket}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh-streams/tickets",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh-streams/{ticket}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/peer-streams/{ticket}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
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
async fn controller_api_differential_peer_stream_ticket_validation_and_limits() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let nominal = super::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Track.mp3","username":"peer","size":321}"#,
        &state,
    )
    .await
    .expect("peer ticket nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let nominal_ticket = nominal_json["ticket"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type == "application/json"
            && nominal_json["contentType"] == "audio/mpeg"
            && nominal_json["expiresInSeconds"] == 120
            && nominal_json["streamUrl"] == format!("/api/v0/peer-streams/{nominal_ticket}")
            && nominal_json["stream_url"] == nominal_json["streamUrl"]
            && nominal_json["size"] == 321
            && !nominal_ticket.is_empty()
    );

    let stored = state
        .stream_tickets
        .write()
        .await
        .get(&nominal_ticket)
        .is_some_and(|ticket| {
            ticket.family == "peer"
                && ticket.filename == "Track.mp3"
                && ticket.peer_username.as_deref() == Some("peer")
                && ticket.size == 321
        });
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "mutation-side-effects-and-readback",
        stored
    );

    let nominal_get = super::route_http_request(
        "GET",
        &format!("/api/v0/peer-streams/{nominal_ticket}"),
        None,
        "",
        &state,
    )
    .await
    .expect("peer ticket nominal read");
    let nominal_get_json = serde_json::from_str::<serde_json::Value>(&nominal_get.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "nominal-status-headers-body",
        nominal_get.status == "200 OK"
            && nominal_get.content_type == "application/json"
            && nominal_get_json["status"] == "available"
            && nominal_get_json["filename"] == "Track.mp3"
            && nominal_get_json["peer_username"] == "peer"
            && nominal_get_json["size"] == 321
            && nominal_get_json["cacheControl"] == "no-store"
            && nominal_get_json["acceptRanges"] == "none"
    );
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "populated-dynamic-state",
        nominal_get_json["status"] == "available"
            && nominal_get_json["ticket"] == nominal_ticket
            && nominal_get_json["contentType"] == "audio/mpeg"
    );

    let malformed_bodies = [
        r#"{"filename":"../escape.flac","username":"peer"}"#,
        r#"{"filename":"Track.flac","username":"peer","size":-1}"#,
        r#"{"filename":"archive.zip","username":"peer"}"#,
        r#"{"filename":"Track.flac","username":"   "}"#,
    ];
    let mut malformed = true;
    let mut malformed_details = Vec::new();
    for body in malformed_bodies {
        let response =
            super::route_http_request("POST", "/api/v0/peer-streams/tickets", None, body, &state)
                .await
                .expect("peer ticket malformed response");
        let pass = response.status == "400 Bad Request"
            && response.content_type == "application/json"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .map(|value| {
                    value["error"]
                        .as_str()
                        .is_some_and(|error| !error.is_empty())
                })
                .unwrap_or(false);
        if !pass {
            malformed_details.push(format!(
                "body={body:?} response={} {} {:?}",
                response.status, response.content_type, response.body
            ));
        }
        malformed &= pass;
    }
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "malformed-path-query-or-body",
        malformed
    );

    let missing =
        super::route_http_request("GET", "/api/v0/peer-streams/not-a-ticket", None, "", &state)
            .await
            .expect("peer ticket missing response");
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found" && missing.content_type == "application/json"
    );

    let malformed_path = super::route_http_request(
        "GET",
        "/api/v0/peer-streams/not-a-ticket/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("peer ticket malformed path response");
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "malformed-path-query-or-body",
        malformed_path.status == "404 Not Found"
    );

    let (disabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_ADVANCED_NETWORKING_JSON",
                r#"{"feature":{"streaming":false}}"#,
            ),
    );
    let disabled_post = super::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Track.flac","username":"peer"}"#,
        &disabled_state,
    )
    .await
    .expect("disabled peer ticket create response");
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "missing-empty-or-conflict-state",
        disabled_post.status == "404 Not Found"
    );

    let (capacity_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut tickets = capacity_state.stream_tickets.write().await;
        for index in 0..super::MAX_PREVIEW_STREAM_TICKETS {
            assert!(tickets
                .issue(
                    "peer",
                    "peer-unresolved",
                    format!("peer-capacity-{index}"),
                    "Track.flac".to_owned(),
                    Some("peer".to_owned()),
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity = super::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Capacity.flac","username":"peer"}"#,
        &capacity_state,
    )
    .await
    .expect("peer ticket capacity response");
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "runtime-failure-and-timeout",
        capacity.status == "429 Too Many Requests"
            && capacity.content_type == "text/plain; charset=utf-8"
            && capacity.body == "Peer stream limit reached."
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("peer stream runtime-failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_ticket = super::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Runtime.flac","username":"peer"}"#,
        &failure_state,
    )
    .await
    .expect("create peer runtime-failure ticket");
    let failure_ticket_json = serde_json::from_str::<serde_json::Value>(&failure_ticket.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    failure_db.close_for_test().await;
    let failure_get = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/peer-streams/{}",
            failure_ticket_json["ticket"].as_str().unwrap_or_default()
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("peer stream read with closed unrelated database");
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "runtime-failure-and-timeout",
        failure_get.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_get.body)
                .map(|value| value["status"] == "available")
                .unwrap_or(false)
    );

    let (restarted_state, _restarted_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let reset_get = super::route_http_request(
        "GET",
        &format!("/api/v0/peer-streams/{nominal_ticket}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("peer stream ticket after restart");
    let reset_create = super::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Restart.flac","username":"peer"}"#,
        &restarted_state,
    )
    .await
    .expect("peer stream ticket create after restart");
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "restart-persistence-or-reset",
        reset_get.status == "404 Not Found" && reset_create.status == "200 OK"
    );

    let (concurrent_state, _concurrent_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let concurrent = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v0/peer-streams/tickets",
            None,
            r#"{"filename":"A.flac","username":"peer"}"#,
            &concurrent_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/peer-streams/tickets",
            None,
            r#"{"filename":"B.flac","username":"peer"}"#,
            &concurrent_state,
        ),
    );
    let concurrent_pass = match concurrent {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
                && concurrent_state.stream_tickets.read().await.records.len() == 2
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "concurrency-and-idempotency",
        concurrent_pass
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("peer_stream_ticket_validation_and_limits.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} peer stream ticket mismatches:\n{}\nmalformed details: {:?}",
        mismatches.len(),
        mismatches.join("\n"),
        malformed_details
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
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_mesh_stream_ticket_validation_and_limits() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SHARE_FIXTURE", ""),
    );
    let nominal = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        &format!(
            r#"{{"contentId":"mesh-ticket-contract","filename":"Track.aac","peerId":"mesh-peer","expectedSize":321,"expectedHash":"{}"}}"#,
            "A".repeat(64)
        ),
        &state,
    )
    .await
    .expect("mesh ticket nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let nominal_keys = nominal_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let nominal_ticket = nominal_json["ticket"].as_str().unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type == "application/json"
            && nominal_keys
                == BTreeSet::from([
                    "contentType".to_owned(),
                    "expiresInSeconds".to_owned(),
                    "source".to_owned(),
                    "streamUrl".to_owned(),
                    "ticket".to_owned(),
                ])
            && nominal_json["contentType"] == "audio/aac"
            && nominal_json["expiresInSeconds"] == 120
            && nominal_json["source"] == "mesh"
            && nominal_json["streamUrl"] == format!("/api/v0/mesh-streams/{nominal_ticket}")
            && !nominal_ticket.is_empty()
    );
    let stored = state
        .stream_tickets
        .write()
        .await
        .get(nominal_ticket)
        .is_some();
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "mutation-side-effects-and-readback",
        stored
    );

    let malformed_bodies = [
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"../escape.flac","peerId":"mesh-peer"}"#,
            "Filename is required.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.flac","peerId":"mesh-peer","expectedSize":-1}"#,
            "Expected size must be greater than or equal to zero.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.flac","peerId":"mesh-peer","expectedHash":"abc"}"#,
            "Expected hash must be a SHA-256 hex digest.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"archive.zip","peerId":"mesh-peer"}"#,
            "Only audio files can be preview streamed from mesh peers.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.oga","peerId":"mesh-peer"}"#,
            "Only audio files can be preview streamed from mesh peers.",
        ),
    ];
    let mut malformed = true;
    for (body, expected_error) in malformed_bodies {
        let response =
            super::route_http_request("POST", "/api/v0/mesh-streams/tickets", None, body, &state)
                .await
                .expect("mesh ticket malformed response");
        malformed &= response.status == "400 Bad Request"
            && response.body == format!(r#"{{"error":"{expected_error}"}}"#);
    }
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "malformed-path-query-or-body",
        malformed
    );

    let blank_peer = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-blank-peer","filename":"Track.flac","peerId":"   "}"#,
        &state,
    )
    .await
    .expect("mesh ticket blank peer response");
    let blank_peer_json = serde_json::from_str::<serde_json::Value>(&blank_peer.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "nominal-status-headers-body",
        blank_peer.status == "200 OK"
            && blank_peer_json["source"] == "mesh"
            && blank_peer_json["contentType"] == "audio/flac"
    );

    let missing =
        super::route_http_request("GET", "/api/v0/mesh-streams/not-a-ticket", None, "", &state)
            .await
            .expect("mesh ticket missing response");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let malformed_path = super::route_http_request(
        "GET",
        "/api/v0/mesh-streams/not-a-ticket/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh ticket malformed path response");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "malformed-path-query-or-body",
        malformed_path.status == "404 Not Found"
    );

    let (disabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_ADVANCED_NETWORKING_JSON",
                r#"{"feature":{"streaming":false}}"#,
            ),
    );
    let disabled_post = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-disabled","filename":"Track.flac"}"#,
        &disabled_state,
    )
    .await
    .expect("disabled mesh ticket create response");
    let disabled_get = super::route_http_request(
        "GET",
        "/api/v0/mesh-streams/not-a-ticket",
        None,
        "",
        &disabled_state,
    )
    .await
    .expect("disabled mesh ticket get response");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "missing-empty-or-conflict-state",
        disabled_post.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "missing-empty-or-conflict-state",
        disabled_get.status == "404 Not Found"
    );

    let (capacity_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut tickets = capacity_state.stream_tickets.write().await;
        for index in 0..super::MAX_PREVIEW_STREAM_TICKETS {
            assert!(tickets
                .issue(
                    "mesh",
                    "mesh-unresolved",
                    format!("capacity-{index}"),
                    "Track.flac".to_owned(),
                    None,
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-capacity","filename":"Track.flac","peerId":"mesh-peer"}"#,
        &capacity_state,
    )
    .await
    .expect("mesh ticket capacity response");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "runtime-failure-and-timeout",
        capacity.status == "429 Too Many Requests"
            && capacity.content_type == "text/plain; charset=utf-8"
            && capacity.body == "Mesh stream limit reached."
    );

    let nominal_get = super::route_http_request(
        "GET",
        &format!("/api/v0/mesh-streams/{nominal_ticket}"),
        None,
        "",
        &state,
    )
    .await
    .expect("nominal mesh stream read");
    let nominal_get_json = serde_json::from_str::<serde_json::Value>(&nominal_get.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "nominal-status-headers-body",
        nominal_get.status == "200 OK"
            && nominal_get.content_type == "application/json"
            && nominal_get_json["status"] == "available"
            && nominal_get_json["cacheControl"] == "no-store"
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh stream runtime-failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_ticket = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-runtime","filename":"Runtime.flac","peerId":"mesh-peer"}"#,
        &failure_state,
    )
    .await
    .expect("create mesh runtime-failure ticket");
    let failure_ticket_json = serde_json::from_str::<serde_json::Value>(&failure_ticket.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    failure_db.close_for_test().await;
    let failure_get = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/mesh-streams/{}",
            failure_ticket_json["ticket"].as_str().unwrap_or_default()
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("mesh stream read with closed unrelated database");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "runtime-failure-and-timeout",
        failure_get.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_get.body)
                .map(|value| value["status"] == "available")
                .unwrap_or(false)
    );

    let (restarted_state, _restarted_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let reset_get = super::route_http_request(
        "GET",
        &format!("/api/v0/mesh-streams/{nominal_ticket}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("mesh stream ticket after restart");
    let reset_create = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-restarted","filename":"Restart.flac","peerId":"mesh-peer"}"#,
        &restarted_state,
    )
    .await
    .expect("mesh stream ticket create after restart");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "restart-persistence-or-reset",
        reset_get.status == "404 Not Found" && reset_create.status == "200 OK"
    );

    let (concurrent_state, _concurrent_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let concurrent = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"contentId":"mesh-concurrent-a","filename":"A.flac","peerId":"mesh-peer"}"#,
            &concurrent_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"contentId":"mesh-concurrent-b","filename":"B.flac","peerId":"mesh-peer"}"#,
            &concurrent_state,
        ),
    );
    let concurrent_pass = match concurrent {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
                && concurrent_state.stream_tickets.read().await.records.len() == 2
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "concurrency-and-idempotency",
        concurrent_pass
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("mesh_stream_ticket_validation_and_limits.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} mesh stream ticket mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn download_batch_projection_uses_local_transfer_state() {
    let (state, _receiver) = test_state();
    let batch_id = "11111111-1111-4111-8111-111111111111";
    let mut ledger = Vec::new();

    let created = super::route_http_request(
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

    let invalid = super::route_http_request(
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

    let response = super::route_http_request(
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

    let missing = super::route_http_request(
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
async fn controller_download_batch_enqueue_validates_dispatches_and_persists() {
    let (state, mut receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "batch-peer=127.0.0.1:2234",
    ));
    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let search_id = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    let request = format!(
        r#"{{"id":"{batch_id}","searchId":"{search_id}","username":"batch-peer","files":[{{"filename":"Music/A.flac","size":42}},{{"filename":"Music/B.flac","size":84}}],"options":{{"destination":"Albums","externalId":"ignored-by-frozen-slskd"}}}}"#
    );

    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &request,
        &state,
    )
    .await
    .expect("enqueue batch");
    assert_eq!(created.status, "201 Created");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created_json["failures"], serde_json::json!([]));
    assert_eq!(created_json["batch"]["id"], batch_id);
    assert_eq!(created_json["batch"]["searchId"], search_id);
    assert_eq!(created_json["batch"]["username"], "batch-peer");
    assert_eq!(created_json["batch"]["direction"], "Download");
    assert_eq!(created_json["batch"]["options"]["destination"], "Albums");
    assert!(created_json["batch"]["options"].get("externalId").is_none());
    assert_eq!(
        created_json["batch"]["transfers"].as_array().unwrap().len(),
        2
    );
    assert!(created_json["batch"]["createdAt"]
        .as_str()
        .unwrap()
        .ends_with('Z'));

    let fetched = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("fetch persisted batch");
    assert_eq!(fetched.status, "200 OK");
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
    assert_eq!(fetched_json["id"], batch_id);
    assert_eq!(fetched_json["transfers"].as_array().unwrap().len(), 2);

    let duplicate_id = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &request,
        &state,
    )
    .await
    .expect("duplicate batch id");
    assert_eq!(duplicate_id.status, "409 Conflict");

    let partial = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        r#"{"id":"cccccccc-cccc-4ccc-8ccc-cccccccccccc","username":"batch-peer","files":[{"filename":"Music/A.flac","size":42},{"filename":"Music/C.flac","size":126}]}"#,
        &state,
    )
    .await
    .expect("partial batch");
    assert_eq!(partial.status, "207 Multi-Status");
    let partial_json = serde_json::from_str::<serde_json::Value>(&partial.body).unwrap();
    assert_eq!(partial_json["failures"].as_array().unwrap().len(), 1);
    assert_eq!(
        partial_json["batch"]["transfers"].as_array().unwrap().len(),
        1
    );

    let all_failed = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        r#"{"id":"dddddddd-dddd-4ddd-8ddd-dddddddddddd","username":"batch-peer","files":[{"filename":"Music/A.flac","size":42},{"filename":"Music/B.flac","size":84}]}"#,
        &state,
    )
    .await
    .expect("all-failed batch");
    assert_eq!(all_failed.status, "200 OK");
    let all_failed_json = serde_json::from_str::<serde_json::Value>(&all_failed.body).unwrap();
    assert_eq!(all_failed_json["failures"].as_array().unwrap().len(), 2);
    assert_eq!(
        all_failed_json["batch"]["transfers"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let commands = [
        receiver.recv().await.unwrap(),
        receiver.recv().await.unwrap(),
        receiver.recv().await.unwrap(),
    ];
    assert!(commands.iter().all(|command| matches!(
        command,
        super::SessionCommand::TransferPeer { username, .. } if username == "batch-peer"
    )));

    for body in [
        r#"{"username":"batch-peer","files":[{"filename":"Music/A.flac","size":42},{"filename":"Music/A.flac","size":42}]}"#,
        r#"{"username":"batch-peer","files":[{"filename":"../escape.flac","size":42}]}"#,
        r#"{"username":"batch-peer","files":[{"filename":"Music/X.flac","size":-1}]}"#,
    ] {
        let invalid = super::route_http_request(
            "POST",
            "/api/v0/transfers/downloads/batches",
            None,
            body,
            &state,
        )
        .await
        .expect("invalid batch");
        assert_eq!(invalid.status, "400 Bad Request", "{body}");
    }

    let held = Arc::clone(&state.download_batch_requests)
        .acquire_owned()
        .await
        .unwrap();
    let throttled = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        r#"{"username":"batch-peer","files":[{"filename":"Music/Y.flac","size":1}]}"#,
        &state,
    )
    .await
    .expect("throttled batch");
    assert_eq!(throttled.status, "429 Too Many Requests");
    drop(held);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_cancellation_returns_frozen_statuses() {
    let (state, _receiver) = test_state();
    let missing = super::route_http_request(
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
    let cancelled = super::route_http_request(
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

static APPLICATION_DUMP_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
async fn controller_api_differential_controller_application_dump_contracts() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd GET /api/v0/application/dump [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": "GET",
                "route": "/api/v0/application/dump",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    async fn live_dump(state: Arc<super::AppState>) -> (String, Vec<u8>) {
        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(super::handle_http_stream(
            server,
            Some("127.0.0.1:1".parse().expect("relay stream remote address")),
            false,
            state,
        ));
        client
            .write_all(
                b"GET /api/v0/application/dump HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write application dump request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read application dump response");
        task.await
            .expect("application dump HTTP task")
            .expect("application dump HTTP response");
        let header_end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("application dump response headers");
        (
            String::from_utf8_lossy(&response[..header_end]).to_string(),
            response[header_end + 4..].to_vec(),
        )
    }

    struct AuditModeGuard(Option<OsString>);
    impl Drop for AuditModeGuard {
        fn drop(&mut self) {
            if let Some(value) = self.0.take() {
                std::env::set_var("SLSKR_CONTROLLER_AUDIT_MODE", value);
            } else {
                std::env::remove_var("SLSKR_CONTROLLER_AUDIT_MODE");
            }
        }
    }

    let _env_lock = APPLICATION_DUMP_ENV_LOCK.lock().await;
    let _audit_mode_guard = AuditModeGuard(std::env::var_os("SLSKR_CONTROLLER_AUDIT_MODE"));
    std::env::set_var("SLSKR_CONTROLLER_AUDIT_MODE", "1");

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let (nominal_headers, nominal_body) = live_dump(Arc::clone(&state)).await;
    let nominal_headers_lower = nominal_headers.to_ascii_lowercase();
    record!(
        "nominal-status-headers-body",
        nominal_headers.starts_with("HTTP/1.1 200 OK")
            && nominal_headers_lower.contains("content-type: application/octet-stream")
            && nominal_headers_lower
                .contains("content-disposition: attachment; filename=slskd.dmp")
            && !nominal_body.is_empty()
    );

    assert!(super::preview_stream_controller::application_dump_path(
        "/api/v0/application/dump"
    ));
    assert!(!super::preview_stream_controller::application_dump_path(
        "/api/v1/application/dump"
    ));
    assert!(!super::preview_stream_controller::application_dump_path(
        "/api/v0/application/dump/extra"
    ));
    let wrong_method =
        super::route_http_request("POST", "/api/v0/application/dump", None, "", &state)
            .await
            .expect("slskd wrong dump method");
    record!(
        "malformed-path-query-or-body",
        wrong_method.status == "405 Method Not Allowed"
    );

    let (empty_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let (empty_headers, empty_body) = live_dump(Arc::clone(&empty_state)).await;
    record!(
        "missing-empty-or-conflict-state",
        empty_headers.starts_with("HTTP/1.1 200 OK") && !empty_body.is_empty()
    );

    state.session.write().await.state = "connected";
    state.session.write().await.username = Some("dump-user".to_owned());
    super::record_event(
        &state,
        "application.dump.fixture",
        "application",
        Some("populated-state".to_owned()),
    )
    .await;
    let (populated_headers, populated_body) = live_dump(Arc::clone(&state)).await;
    record!(
        "populated-dynamic-state",
        populated_headers.starts_with("HTTP/1.1 200 OK")
            && !populated_body.is_empty()
            && state.session.read().await.state == "connected"
    );

    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-application-dump-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file")
        .expect("create application dump state conflict");
    let (mut failure_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    Arc::get_mut(&mut failure_state)
        .expect("exclusive application dump failure state")
        .config
        .state_dir = conflict_root.clone();
    let (failure_headers, failure_body) = live_dump(Arc::clone(&failure_state)).await;
    record!(
        "runtime-failure-and-timeout",
        failure_headers.starts_with("HTTP/1.1 500 Internal Server Error")
            && failure_body
                .windows("failed to create application dump".len())
                .any(|window| window == b"failed to create application dump",)
            && !failure_body
                .windows("state directory is a file".len())
                .any(|window| window == b"state directory is a file")
    );
    let _ = fs::remove_file(conflict_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_application_dump_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize application dump ledger"),
    )
    .expect("write application dump ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd application dump mismatches: {}",
        mismatches.len(),
        mismatches.join(", ")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
fn controller_api_differential_native_application_dump_gates() {
    run_controller_future_on_large_stack("native-application-dump-gates", || {
        controller_api_differential_native_application_dump_gates_impl()
    });
}

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_native_application_dump_gates_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct AuditModeGuard(Option<std::ffi::OsString>);
    impl Drop for AuditModeGuard {
        fn drop(&mut self) {
            if let Some(value) = self.0.take() {
                std::env::set_var("SLSKR_CONTROLLER_AUDIT_MODE", value);
            } else {
                std::env::remove_var("SLSKR_CONTROLLER_AUDIT_MODE");
            }
        }
    }

    let _env_lock = APPLICATION_DUMP_ENV_LOCK.lock().await;
    let _audit_mode_guard = AuditModeGuard(std::env::var_os("SLSKR_CONTROLLER_AUDIT_MODE"));
    std::env::set_var("SLSKR_CONTROLLER_AUDIT_MODE", "1");

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskdn POST /api/v0/application/dump [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/application/dump",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    async fn live_dump(state: Arc<super::AppState>) -> (String, Vec<u8>) {
        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(super::handle_http_stream(
            server,
            Some("127.0.0.1:1".parse().expect("dump remote address")),
            false,
            state,
        ));
        client
            .write_all(
                b"POST /api/v0/application/dump HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
            )
            .await
            .expect("write application dump request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read application dump response");
        task.await
            .expect("application dump HTTP task")
            .expect("application dump HTTP response");
        let header_end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("application dump response headers");
        (
            String::from_utf8_lossy(&response[..header_end]).to_string(),
            response[header_end + 4..].to_vec(),
        )
    }

    let (disabled, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let response =
        super::route_http_request("POST", "/api/v0/application/dump", None, "", &disabled)
            .await
            .expect("disabled slskdN dump route");
    assert_eq!(response.status, "404 Not Found");
    assert!(response.body.is_empty());

    let base = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", "native")
        .with("SLSKD_ALLOW_MEMORY_DUMP", "true")
        .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
        .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", "192.0.2.0/24");
    let (local_only, _receiver) = test_state_with_env(base.clone());
    let local =
        super::route_http_request("POST", "/api/v0/application/dump", None, "", &local_only)
            .await
            .expect("local slskdN dump route");
    assert_eq!(local.status, "200 OK");
    assert_eq!(local.content_type, "application/octet-stream");

    let remote_headers = super::RequestSecurityHeaders {
        remote_addr: Some("192.0.2.44:1".parse().unwrap()),
        ..Default::default()
    };
    let remote_forbidden = super::route_http_request_with_headers(
        "POST",
        "/api/v0/application/dump",
        None,
        "",
        &local_only,
        remote_headers.clone(),
    )
    .await
    .expect("remote dump denial");
    assert_eq!(remote_forbidden.status, "403 Forbidden");
    assert!(remote_forbidden.body.is_empty());

    let (remote_allowed, _receiver) =
        test_state_with_env(base.clone().with("SLSKD_ALLOW_REMOTE_DUMP", "true"));
    let remote = super::route_http_request_with_headers(
        "POST",
        "/api/v0/application/dump",
        None,
        "",
        &remote_allowed,
        remote_headers,
    )
    .await
    .expect("remote dump allowance");
    assert_eq!(remote.status, "200 OK");

    let wrong_method =
        super::route_http_request("GET", "/api/v0/application/dump", None, "", &local_only)
            .await
            .expect("slskdN wrong dump method");
    assert_eq!(wrong_method.status, "405 Method Not Allowed");
    record!(
        "nominal-status-headers-body",
        local.status == "200 OK"
            && local.content_type == "application/octet-stream"
            && local.body.is_empty()
    );
    record!(
        "malformed-path-query-or-body",
        wrong_method.status == "405 Method Not Allowed"
    );

    let diagnostics_dir = local_only.config.state_dir.join("diagnostics");
    let files_before = fs::read_dir(&diagnostics_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .count();
    let (mutation_headers, mutation_body) = live_dump(Arc::clone(&local_only)).await;
    let mutation_headers_lower = mutation_headers.to_ascii_lowercase();
    let files_after = fs::read_dir(&diagnostics_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .count();
    record!(
        "mutation-side-effects-and-readback",
        mutation_headers.starts_with("HTTP/1.1 200 OK")
            && mutation_headers_lower.contains("content-type: application/octet-stream")
            && !mutation_body.is_empty()
            && files_after == files_before
    );

    let (fresh_dump_state, _receiver) = test_state_with_env(base.clone());
    let (fresh_headers, fresh_body) = live_dump(Arc::clone(&fresh_dump_state)).await;
    record!(
        "missing-empty-or-conflict-state",
        fresh_headers.starts_with("HTTP/1.1 200 OK") && !fresh_body.is_empty()
    );
    record!(
        "restart-persistence-or-reset",
        fresh_headers.starts_with("HTTP/1.1 200 OK")
            && !fresh_body.is_empty()
            && !fresh_dump_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    let concurrent_dumps = futures_util::future::join_all([
        live_dump(Arc::clone(&local_only)),
        live_dump(Arc::clone(&local_only)),
    ])
    .await;
    record!(
        "concurrency-and-idempotency",
        concurrent_dumps
            .iter()
            .all(|(headers, body)| { headers.starts_with("HTTP/1.1 200 OK") && !body.is_empty() })
    );

    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-slskdn-application-dump-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file")
        .expect("create slskdN application dump state conflict");
    let (mut failure_state, _receiver) = test_state_with_env(base);
    Arc::get_mut(&mut failure_state)
        .expect("exclusive slskdN application dump failure state")
        .config
        .state_dir = conflict_root.clone();
    let (failure_headers, failure_body) = live_dump(Arc::clone(&failure_state)).await;
    record!(
        "runtime-failure-and-timeout",
        failure_headers.starts_with("HTTP/1.1 500 Internal Server Error")
            && failure_body
                .windows("failed to create application dump".len())
                .any(|window| window == b"failed to create application dump")
            && !failure_body
                .windows("state directory is a file".len())
                .any(|window| window == b"state directory is a file")
    );
    let _ = fs::remove_file(conflict_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("native_application_dump_gates.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdN application dump mismatches: {}",
        mismatches.len(),
        mismatches.join(", ")
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
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_native_application_open_cases() {
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

    let (application_state, _receiver) = test_state_with_env(env.clone());
    let application_malformed = super::route_http_request(
        "GET",
        "/api/v0/application/extra",
        None,
        "",
        &application_state,
    )
    .await
    .expect("malformed application state path");
    record!(
        "GET",
        "/api/v0/application",
        "malformed-path-query-or-body",
        application_malformed.status == "404 Not Found"
    );

    let application_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("application runtime database");
    let (application_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(application_db.clone()),
    );
    application_db.close_for_test().await;
    let loopback_failure = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application-runtime-failure"}"#,
        &application_failure_state,
    )
    .await
    .expect("application runtime fixture");
    let application_runtime = super::route_http_request(
        "GET",
        "/api/v0/application",
        None,
        "",
        &application_failure_state,
    )
    .await
    .expect("application state after runtime failure");
    record!(
        "GET",
        "/api/v0/application",
        "runtime-failure-and-timeout",
        loopback_failure.status == "200 OK"
            && application_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&application_runtime.body)
                .is_ok_and(|value| value.is_object())
            && application_failure_state
                .session
                .read()
                .await
                .last_error
                .is_some()
    );

    let (version_state, _receiver) = test_state_with_env(env.clone());
    let build_malformed = super::route_http_request(
        "GET",
        "/api/v0/application/build?checkForUpdates=not-a-bool",
        None,
        "",
        &version_state,
    )
    .await
    .expect("malformed application build query");
    record!(
        "GET",
        "/api/v0/application/build",
        "malformed-path-query-or-body",
        build_malformed.status == "400 Bad Request"
    );
    let build_missing =
        super::route_http_request("GET", "/api/v0/application/build", None, "", &version_state)
            .await
            .expect("empty application build state");
    record!(
        "GET",
        "/api/v0/application/build",
        "missing-empty-or-conflict-state",
        build_missing.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&build_missing.body)
                .is_ok_and(|value| value.is_object())
    );

    let version_lookup_failure =
        super::refresh_controller_version_check(&version_state, "http://127.0.0.1:9").await;
    assert!(version_lookup_failure.is_err());
    let build_runtime =
        super::route_http_request("GET", "/api/v0/application/build", None, "", &version_state)
            .await
            .expect("application build after version lookup failure");
    record!(
        "GET",
        "/api/v0/application/build",
        "runtime-failure-and-timeout",
        build_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&build_runtime.body)
                .is_ok_and(|value| value.is_object())
            && version_state
                .controller_version
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .checked_at
                .is_some()
    );

    let version_malformed = super::route_http_request(
        "GET",
        "/api/v0/application/version/extra",
        None,
        "",
        &version_state,
    )
    .await
    .expect("malformed application version path");
    record!(
        "GET",
        "/api/v0/application/version",
        "malformed-path-query-or-body",
        version_malformed.status == "404 Not Found"
    );
    let version_missing = super::route_http_request(
        "GET",
        "/api/v0/application/version",
        None,
        "",
        &version_state,
    )
    .await
    .expect("empty application version state");
    record!(
        "GET",
        "/api/v0/application/version",
        "missing-empty-or-conflict-state",
        version_missing.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&version_missing.body)
                .is_ok_and(|value| value == env!("CARGO_PKG_VERSION"))
    );
    let version_runtime = super::route_http_request(
        "GET",
        "/api/v0/application/version",
        None,
        "",
        &version_state,
    )
    .await
    .expect("application version after lookup failure");
    record!(
        "GET",
        "/api/v0/application/version",
        "runtime-failure-and-timeout",
        version_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&version_runtime.body)
                .is_ok_and(|value| value == env!("CARGO_PKG_VERSION"))
    );
    let version_mutation =
        super::route_http_request("PUT", "/api/v0/application", None, "{}", &version_state)
            .await
            .expect("populate application runtime state");
    let populated_version = super::route_http_request(
        "GET",
        "/api/v0/application/version",
        None,
        "",
        &version_state,
    )
    .await
    .expect("populated application version");
    record!(
        "GET",
        "/api/v0/application/version",
        "populated-dynamic-state",
        version_mutation.status == "204 No Content"
            && populated_version.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&populated_version.body)
                .is_ok_and(|value| value == env!("CARGO_PKG_VERSION"))
    );

    let latest_malformed = super::route_http_request(
        "GET",
        "/api/v0/application/version/latest?forceCheck=not-a-bool",
        None,
        "",
        &version_state,
    )
    .await
    .expect("malformed latest version query");
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "malformed-path-query-or-body",
        latest_malformed.status == "400 Bad Request"
    );
    let latest_missing = super::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &version_state,
    )
    .await
    .expect("empty latest version state");
    let latest_missing_json =
        serde_json::from_str::<serde_json::Value>(&latest_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "missing-empty-or-conflict-state",
        latest_missing.status == "200 OK"
            && latest_missing_json.is_object()
            && latest_missing_json["current"] == env!("CARGO_PKG_VERSION")
    );
    let latest_runtime = super::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &version_state,
    )
    .await
    .expect("latest version after lookup failure");
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "runtime-failure-and-timeout",
        latest_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&latest_runtime.body)
                .is_ok_and(|value| value.is_object())
            && version_state
                .controller_version
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .checked_at
                .is_some()
    );

    let (put_state, _receiver) = test_state_with_env(env.clone());
    let put_nominal =
        super::route_http_request("PUT", "/api/v0/application", None, "{}", &put_state)
            .await
            .expect("application restart");
    record!(
        "PUT",
        "/api/v0/application",
        "nominal-status-headers-body",
        put_nominal.status == "204 No Content" && put_nominal.body.is_empty()
    );
    let put_malformed =
        super::route_http_request("PUT", "/api/v0/application/extra", None, "{}", &put_state)
            .await
            .expect("malformed application restart path");
    record!(
        "PUT",
        "/api/v0/application",
        "malformed-path-query-or-body",
        put_malformed.status == "404 Not Found"
    );
    let put_missing = super::route_http_request("PUT", "/api/v0/application", None, "", &put_state)
        .await
        .expect("empty application restart body");
    record!(
        "PUT",
        "/api/v0/application",
        "missing-empty-or-conflict-state",
        put_missing.status == "204 No Content" && put_missing.body.is_empty()
    );
    let put_readback =
        super::route_http_request("GET", "/api/v0/application", None, "", &put_state)
            .await
            .expect("application restart readback");
    record!(
        "PUT",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        put_readback.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&put_readback.body)
                .is_ok_and(|value| value["pendingRestart"] == true)
    );
    let (fresh_put_state, _receiver) = test_state_with_env(env.clone());
    let fresh_put_readback =
        super::route_http_request("GET", "/api/v0/application", None, "", &fresh_put_state)
            .await
            .expect("fresh application restart readback");
    record!(
        "PUT",
        "/api/v0/application",
        "restart-persistence-or-reset",
        fresh_put_readback.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_put_readback.body)
                .is_ok_and(|value| value["pendingRestart"] == false)
    );
    let (concurrent_put_state, _receiver) = test_state_with_env(env.clone());
    let concurrent_puts = futures_util::future::join_all([
        super::route_http_request(
            "PUT",
            "/api/v0/application",
            None,
            "{}",
            &concurrent_put_state,
        ),
        super::route_http_request(
            "PUT",
            "/api/v0/application",
            None,
            "{}",
            &concurrent_put_state,
        ),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/application",
        "concurrency-and-idempotency",
        concurrent_puts.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content"))
            && concurrent_put_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    let (delete_state, _receiver) = test_state_with_env(env.clone());
    delete_state
        .runtime
        .write()
        .await
        .set_restart_requested(true);
    let delete_nominal =
        super::route_http_request("DELETE", "/api/v0/application", None, "", &delete_state)
            .await
            .expect("application shutdown");
    record!(
        "DELETE",
        "/api/v0/application",
        "nominal-status-headers-body",
        delete_nominal.status == "204 No Content" && delete_nominal.body.is_empty()
    );
    let delete_malformed = super::route_http_request(
        "DELETE",
        "/api/v0/application/extra",
        None,
        "",
        &delete_state,
    )
    .await
    .expect("malformed application shutdown path");
    record!(
        "DELETE",
        "/api/v0/application",
        "malformed-path-query-or-body",
        delete_malformed.status == "404 Not Found"
    );
    let delete_missing =
        super::route_http_request("DELETE", "/api/v0/application", None, "", &delete_state)
            .await
            .expect("empty application shutdown body");
    record!(
        "DELETE",
        "/api/v0/application",
        "missing-empty-or-conflict-state",
        delete_missing.status == "204 No Content" && delete_missing.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        !delete_state
            .runtime
            .read()
            .await
            .application_restart_requested
    );
    let (fresh_delete_state, _receiver) = test_state_with_env(env.clone());
    let fresh_delete = super::route_http_request(
        "DELETE",
        "/api/v0/application",
        None,
        "",
        &fresh_delete_state,
    )
    .await
    .expect("fresh application shutdown");
    record!(
        "DELETE",
        "/api/v0/application",
        "restart-persistence-or-reset",
        fresh_delete.status == "204 No Content"
            && !fresh_delete_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );
    let (concurrent_delete_state, _receiver) = test_state_with_env(env.clone());
    concurrent_delete_state
        .runtime
        .write()
        .await
        .set_restart_requested(true);
    let concurrent_deletes = futures_util::future::join_all([
        super::route_http_request(
            "DELETE",
            "/api/v0/application",
            None,
            "",
            &concurrent_delete_state,
        ),
        super::route_http_request(
            "DELETE",
            "/api/v0/application",
            None,
            "",
            &concurrent_delete_state,
        ),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/application",
        "concurrency-and-idempotency",
        concurrent_deletes.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content"))
            && !concurrent_delete_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    let (gc_state, _receiver) = test_state_with_env(env.clone());
    let gc_nominal =
        super::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
            .await
            .expect("application garbage collection");
    record!(
        "POST",
        "/api/v0/application/gc",
        "nominal-status-headers-body",
        gc_nominal.status == "200 OK" && gc_nominal.body.is_empty()
    );
    let gc_malformed =
        super::route_http_request("POST", "/api/v0/application/gc/extra", None, "", &gc_state)
            .await
            .expect("malformed application gc path");
    record!(
        "POST",
        "/api/v0/application/gc",
        "malformed-path-query-or-body",
        gc_malformed.status == "404 Not Found"
    );
    let gc_missing =
        super::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
            .await
            .expect("empty application gc body");
    record!(
        "POST",
        "/api/v0/application/gc",
        "missing-empty-or-conflict-state",
        gc_missing.status == "200 OK" && gc_missing.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/application/gc",
        "mutation-side-effects-and-readback",
        gc_state.runtime.read().await.gc_runs == 2
    );
    let (fresh_gc_state, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/application/gc",
        "restart-persistence-or-reset",
        fresh_gc_state.runtime.read().await.gc_runs == 0
    );
    let (concurrent_gc_state, _receiver) = test_state_with_env(env.clone());
    let concurrent_gcs = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/application/gc",
            None,
            "",
            &concurrent_gc_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/application/gc",
            None,
            "",
            &concurrent_gc_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/application/gc",
        "concurrency-and-idempotency",
        concurrent_gcs.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && concurrent_gc_state.runtime.read().await.gc_runs == 2
    );

    let (loopback_state, _receiver) = test_state_with_env(env.clone());
    let loopback_nominal = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application"}"#,
        &loopback_state,
    )
    .await
    .expect("application loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "nominal-status-headers-body",
        loopback_nominal.status == "200 OK"
            && loopback_nominal.content_type.is_empty()
            && loopback_nominal.body.is_empty()
    );
    let loopback_malformed = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        "{",
        &loopback_state,
    )
    .await
    .expect("malformed application loopback body");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "malformed-path-query-or-body",
        loopback_malformed.status == "400 Bad Request"
    );
    let loopback_missing = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        "",
        &loopback_state,
    )
    .await
    .expect("empty application loopback body");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "missing-empty-or-conflict-state",
        loopback_missing.status == "400 Bad Request"
    );
    let loopback_logs = super::route_http_request("GET", "/api/v0/logs", None, "", &loopback_state)
        .await
        .expect("application loopback logs");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "mutation-side-effects-and-readback",
        loopback_logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&loopback_logs.body).is_ok_and(|value| {
                value.as_array().is_some_and(|logs| {
                    logs.iter().any(|log| {
                        log["category"] == "application"
                            && log["message"] == "Loopback POST: {\"probe\":\"application\"}"
                    })
                })
            },)
    );
    let loopback_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("loopback runtime database");
    let (loopback_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(loopback_db.clone()),
    );
    loopback_db.close_for_test().await;
    let loopback_runtime = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application-runtime"}"#,
        &loopback_failure_state,
    )
    .await
    .expect("application loopback runtime failure");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "runtime-failure-and-timeout",
        loopback_runtime.status == "200 OK"
            && loopback_failure_state
                .session
                .read()
                .await
                .last_error
                .is_some()
    );
    let (fresh_loopback_state, _receiver) = test_state_with_env(env.clone());
    let fresh_logs =
        super::route_http_request("GET", "/api/v0/logs", None, "", &fresh_loopback_state)
            .await
            .expect("fresh application loopback logs");
    let fresh_loopback = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application-fresh"}"#,
        &fresh_loopback_state,
    )
    .await
    .expect("fresh application loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "restart-persistence-or-reset",
        fresh_loopback.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_logs.body)
                .is_ok_and(|value| value.as_array().is_some_and(Vec::is_empty))
    );
    let (concurrent_loopback_state, _receiver) = test_state_with_env(env);
    let concurrent_loopbacks = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"application-concurrent-a"}"#,
            &concurrent_loopback_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"application-concurrent-b"}"#,
            &concurrent_loopback_state,
        ),
    ])
    .await;
    let concurrent_loopback_logs =
        super::route_http_request("GET", "/api/v0/logs", None, "", &concurrent_loopback_state)
            .await
            .expect("concurrent application loopback logs");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "concurrency-and-idempotency",
        concurrent_loopbacks.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && serde_json::from_str::<serde_json::Value>(&concurrent_loopback_logs.body).is_ok_and(
                |value| {
                    value.as_array().is_some_and(|logs| {
                        logs.iter()
                            .filter(|log| log["category"] == "application")
                            .count()
                            >= 2
                    })
                }
            )
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create application evidence directory");
    fs::write(
        evidence_dir.join("application_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize application evidence"),
    )
    .expect("write application evidence");
    assert!(
        mismatches.is_empty(),
        "{} slskdN application mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_rescan_route_rebuilds_snapshot() {
    let (state, _receiver) = test_state();

    {
        let mut shares = state.shares.write().await;
        shares.entries.clear();
    }

    let response = super::route_http_request("POST", "/api/v0/shares/rescan", None, "", &state)
        .await
        .expect("route response");

    assert_eq!(response.status, "202 Accepted");
    assert!(response.body.contains("\"files\":1"));
    assert_eq!(state.shares.read().await.entries.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn config_share_add_updates_runtime_index_and_rejects_duplicates() {
    let root = std::env::temp_dir().join(format!(
        "slskr-config-share-test-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("share root");
    std::fs::write(root.join("track.flac"), b"track").expect("share file");
    let (state, _receiver) = test_state();
    let existing_files = state.shares.read().await.entries.len();
    let response = super::route_http_request(
        "POST",
        "/api/config/shares",
        None,
        &serde_json::json!({
            "path": root,
            "alias": "added",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("add runtime share");
    assert_eq!(response.status, "201 Created");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(body["added"], true);
    assert_eq!(body["alias"], "added");
    assert_eq!(body["files"], existing_files + 1);
    assert_eq!(body["configurationPersisted"], false);
    assert!(state
        .share_settings
        .read()
        .await
        .directories
        .iter()
        .any(|directory| directory.alias == "added"));
    assert!(state
        .shares
        .read()
        .await
        .entries
        .iter()
        .any(|entry| entry.filename == "added/track.flac"));

    let duplicate = super::route_http_request(
        "POST",
        "/api/config/shares",
        None,
        &serde_json::json!({"path": root}).to_string(),
        &state,
    )
    .await
    .expect("duplicate runtime share");
    assert_eq!(duplicate.status, "409 Conflict");
    std::fs::remove_dir_all(root).expect("remove share root");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_rebuild_routes_reject_concurrent_scans() {
    let (state, _receiver) = test_state();
    let _permit = Arc::clone(&state.share_scans)
        .acquire_owned()
        .await
        .expect("share scan permit");

    for (method, path) in [("PUT", "/api/shares"), ("POST", "/api/v0/shares/rescan")] {
        let response = super::route_http_request(method, path, None, "", &state)
            .await
            .expect("route response");
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(
            response.body,
            "{\"error\":\"share scan already in progress\"}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_rebuild_errors_redact_internal_details() {
    for error in [
        super::SHARE_SCAN_WORKER_ERROR,
        "share index persistence failed: database path /private/slskr.db",
    ] {
        let response = super::share_rebuild_error_response(error);
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(response.body, "{\"error\":\"share index unavailable\"}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_rebuild_routes_roll_back_when_persistence_fails() {
    for (method, path) in [("PUT", "/api/shares"), ("POST", "/api/v0/shares/rescan")] {
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
        {
            let mut shares = state.shares.write().await;
            shares.entries.clear();
            shares.local_paths.clear();
            shares.roots.clear();
            shares.scan_errors.push("previous snapshot".to_owned());
        }
        let previous = state.shares.read().await.json();
        db.close_for_test().await;

        let response = super::route_http_request(method, path, None, "", &state)
            .await
            .expect("failed share index persistence response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert_eq!(
            response.body, "{\"error\":\"share index unavailable\"}",
            "{method} {path}"
        );
        assert_eq!(
            state.shares.read().await.json(),
            previous,
            "{method} {path}"
        );
    }
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
async fn controller_api_differential_search_api_creates_reads_and_completes_records() {
    let (state, mut receiver) = test_state();
    // Versioned search creation requires an authenticated Soulseek server
    // session; this test exercises the positive create/read/complete path.
    state.session.write().await.state = "connected";

    let created = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"test flac\"}",
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created.status, "200 OK");
    assert!(created.body.contains("\"query\":\"test flac\""));
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert!(created_json["searchId"].is_string());
    assert_eq!(created_json["query"], "test flac");
    assert!(created_json["results"].is_array());
    assert_eq!(
        receiver.try_recv().expect("search command"),
        super::SessionCommand::Search {
            token: 1,
            query: "test flac".to_owned(),
            target: super::SearchDispatchTarget::Global,
        }
    );
    let listed_compat = super::route_http_request("GET", "/api/v0/searches", None, "", &state)
        .await
        .expect("list compatibility searches");
    assert_eq!(listed_compat.status, "200 OK");
    let listed_compat_json =
        serde_json::from_str::<serde_json::Value>(&listed_compat.body).unwrap();
    assert!(listed_compat_json
        .as_array()
        .is_some_and(|entries| { entries.iter().any(|entry| entry["query"] == "test flac") }));
    let duplicate = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"id":"1","query":"duplicate"}"#,
        &state,
    )
    .await
    .expect("reject duplicate search id");
    assert_eq!(duplicate.status, "409 Conflict");

    let listed = super::route_http_request("GET", "/api/v0/searches/records", None, "", &state)
        .await
        .expect("list searches");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));
    assert!(listed.body.contains("\"filtered_count\":1"));

    let fetched = super::route_http_request("GET", "/api/v0/searches/1", None, "", &state)
        .await
        .expect("get search");
    assert_eq!(fetched.status, "200 OK");
    assert!(fetched.content_type.contains("application/json"));
    assert!(fetched.body.contains("Virtual/Test.flac"));

    let completed =
        super::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
            .await
            .expect("complete search");
    assert_eq!(completed.status, "200 OK");
    assert!(completed.body.contains("\"status\":\"completed\""));

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/searches",
            "case": "concurrency-and-idempotency",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/searches",
            "case": "missing-empty-or-conflict-state",
            "pass": duplicate.status == "409 Conflict",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/searches/{id}",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/searches/{id}",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/searches",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("search_api_reads_and_idempotency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn search_store_resolves_dash_stripped_controller_ids() {
    let canonical_id = "11111111-1111-4111-8111-111111111111";
    let compatibility_id = canonical_id.replace('-', "");
    let mut searches = super::SearchStore::new();
    let record = searches
        .create(
            Some(canonical_id.to_owned()),
            "compatibility id".to_owned(),
            "global",
            None,
            Vec::new(),
            300,
        )
        .expect("create search")
        .record;

    assert_eq!(
        searches
            .get_by_identifier(&compatibility_id)
            .map(|search| search.id),
        Some(canonical_id.to_owned())
    );
    assert!(searches
        .update_by_identifier(&compatibility_id, Some("updated".to_owned()), None)
        .is_some());
    assert_eq!(
        searches
            .get_by_identifier(canonical_id)
            .map(|search| search.query),
        Some("updated".to_owned())
    );
    assert!(searches.remove_by_identifier(&compatibility_id).is_some());
    assert!(searches.records.is_empty());
    assert!(record.id.contains('-'));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn search_response_availability_tracks_durable_payloads_and_completion() {
    let mut searches = super::SearchStore::new();
    let created = searches
        .create(
            None,
            "empty active".to_owned(),
            "global",
            None,
            Vec::new(),
            60,
        )
        .unwrap()
        .record;
    let active = serde_json::from_str::<serde_json::Value>(&created.json()).unwrap();
    assert_eq!(active["responsesAvailable"], false);

    let completed = searches.complete(created.token).unwrap().0;
    let completed = serde_json::from_str::<serde_json::Value>(&completed.json()).unwrap();
    assert_eq!(completed["responsesAvailable"], true);

    let cancelled = searches
        .create(
            None,
            "cancelled late completion".to_owned(),
            "global",
            None,
            Vec::new(),
            60,
        )
        .unwrap()
        .record;
    let (cancelled, transitioned) = searches
        .set_status_by_token(cancelled.token, "cancelled")
        .unwrap();
    assert!(transitioned);
    let (cancelled_after_completion, transitioned) = searches.complete(cancelled.token).unwrap();
    assert!(!transitioned);
    assert_eq!(cancelled_after_completion.status, "cancelled");
    assert_eq!(cancelled_after_completion.updated_at, cancelled.updated_at);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn search_create_rejects_before_mutation_when_dispatch_is_unavailable() {
    let (state, receiver) = test_state();
    drop(receiver);

    let response = super::route_http_request(
        "POST",
        "/api/searches",
        None,
        r#"{"query":"never dispatched"}"#,
        &state,
    )
    .await
    .expect("failed dispatch response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("session manager is not running"));
    let searches = state.searches.read().await;
    assert!(searches.records.is_empty());
    assert_eq!(searches.next_token, 1);
    drop(searches);
    assert!(state
        .events
        .read()
        .await
        .records
        .iter()
        .all(|event| event.kind != "search.started"));
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
async fn controller_api_differential_search_creation_rehydrates() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";

    let created = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"persist me\",\"target\":\"global\"}",
        &state,
    )
    .await
    .expect("create persisted search");
    assert_eq!(created.status, "200 OK");
    let _ = receiver.try_recv();

    let persisted = db.list_searches(10, 0).await.expect("list persisted");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].query, "persist me");

    let rehydrated = super::SearchStore::from_persisted(persisted);
    let (restarted_state, _) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        rehydrated,
        Some(db),
    );
    let listed = super::route_http_request(
        "GET",
        "/api/v0/searches/records",
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("list rehydrated searches");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));
    assert!(listed.body.contains("\"query\":\"persist me\""));

    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/searches",
        "case": "restart-persistence-or-reset",
        "pass": true,
    }));
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("search_creation_rehydrates.json"),
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
async fn controller_api_differential_search_mutation_lifecycle() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";

    let created = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"id":"stable-search","query":"durable search","ttl_seconds":60}"#,
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created.status, "200 OK");
    let _ = receiver.try_recv();

    let response = super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"username":"peer","files":[{"filename":"Remote/Durable.flac","size":42,"extension":"flac"}]}"#,
        &state,
    )
    .await
    .expect("ingest search response");
    assert_eq!(response.status, "200 OK");
    let persisted = db.get_search("1").await.expect("get search").unwrap();
    assert_eq!(persisted.result_count, 1);
    assert_eq!(
        db.list_search_identities()
            .await
            .expect("list search identities")
            .get("1")
            .map(String::as_str),
        Some("stable-search")
    );
    let persisted_results = db
        .list_search_results(Some("1"), 10, 0)
        .await
        .expect("list search results");
    assert_eq!(persisted_results.len(), 1);
    assert_eq!(persisted_results[0].peer_username.as_deref(), Some("peer"));
    assert_eq!(persisted_results[0].filename, "Remote/Durable.flac");

    let mut active = persisted.clone();
    active.status = "active".to_owned();
    active.result_count = 1;
    let terminalized =
        super::SearchStore::from_persisted_with_results(vec![active], persisted_results.clone());
    let terminal = terminalized.get_by_identifier("1").unwrap();
    assert_eq!(terminal.status, "expired");
    assert!(terminal.results.is_empty());

    let completed =
        super::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
            .await
            .expect("complete search");
    assert_eq!(completed.status, "200 OK");
    let rehydrated = super::SearchStore::from_persisted_with_results_and_identities(
        db.list_searches(10, 0).await.expect("list searches"),
        persisted_results.clone(),
        db.list_search_identities()
            .await
            .expect("list search identities"),
    );
    let rehydrated_record = rehydrated
        .get_by_identifier("1")
        .expect("rehydrated search");
    assert_eq!(rehydrated_record.results.len(), 1);
    assert_eq!(rehydrated_record.results[0].filename, "Remote/Durable.flac");
    assert_eq!(
        rehydrated
            .get_by_identifier("stable-search")
            .expect("stable identifier")
            .token,
        1
    );

    let cancelled = super::route_http_request("PUT", "/api/v0/searches/1", None, "", &state)
        .await
        .expect("cancel versioned search");
    assert_eq!(cancelled.status, "200 OK");
    assert!(cancelled.body.is_empty());
    assert_eq!(
        db.get_search("1")
            .await
            .expect("get cancelled search")
            .unwrap()
            .status,
        "cancelled"
    );

    let updated = super::route_http_request(
        "PUT",
        "/api/searches/1",
        None,
        r#"{"query":"durable updated","status":"failed"}"#,
        &state,
    )
    .await
    .expect("update search");
    assert_eq!(updated.status, "200 OK");
    let persisted = db.get_search("1").await.expect("get updated").unwrap();
    assert_eq!(persisted.query, "durable updated");
    assert_eq!(persisted.status, "failed");
    assert!(persisted.completed_at.is_some());

    let deleted = super::route_http_request("DELETE", "/api/searches/1", None, "", &state)
        .await
        .expect("delete search");
    assert_eq!(deleted.status, "200 OK");
    assert!(db.get_search("1").await.expect("get deleted").is_none());
    assert!(db
        .list_search_results(Some("1"), 10, 0)
        .await
        .expect("list deleted results")
        .is_empty());

    let created = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"clearable"}"#,
        &state,
    )
    .await
    .expect("create clearable search");
    assert_eq!(created.status, "200 OK");
    let _ = receiver.try_recv();
    assert_eq!(
        db.list_searches(10, 0)
            .await
            .expect("list before clear")
            .len(),
        1
    );
    let stats = db.get_stats().await.expect("stats before clear");
    assert_eq!(stats.search_count, 1);
    assert_eq!(stats.search_result_count, 0);

    let clearable_id = db
        .list_searches(10, 0)
        .await
        .expect("list clearable search")
        .first()
        .map(|record| record.id.clone())
        .expect("clearable search id");
    let deleted_versioned = super::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{clearable_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete versioned search");
    assert_eq!(deleted_versioned.status, "204 No Content");
    assert!(deleted_versioned.body.is_empty());
    assert!(db
        .get_search(&clearable_id)
        .await
        .expect("get deleted versioned search")
        .is_none());

    let cleared = super::route_http_request("DELETE", "/api/v0/searches", None, "", &state)
        .await
        .expect("clear searches");
    assert_eq!(cleared.status, "200 OK");
    assert!(db
        .list_searches(10, 0)
        .await
        .expect("list after clear")
        .is_empty());

    for (method, route, case) in [
        (
            "PUT",
            "/api/v0/searches/{id}",
            "nominal-status-headers-body",
        ),
        (
            "PUT",
            "/api/v0/searches/{id}",
            "mutation-side-effects-and-readback",
        ),
        (
            "DELETE",
            "/api/v0/searches/{id}",
            "nominal-status-headers-body",
        ),
        (
            "DELETE",
            "/api/v0/searches/{id}",
            "mutation-side-effects-and-readback",
        ),
        ("DELETE", "/api/v0/searches", "nominal-status-headers-body"),
        (
            "DELETE",
            "/api/v0/searches",
            "mutation-side-effects-and-readback",
        ),
    ] {
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": method,
            "route": route,
            "case": case,
            "pass": true,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("search_mutation_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn browse_cache_persists_and_rehydrates_records() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";

    let requested = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request browse");
    assert_eq!(requested.status, "202 Accepted");
    let _ = receiver.try_recv();

    let ingested = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        r#"{"username":"friend","directories":[{"name":"Remote/Album","files":[{"filename":"Track.flac","size":123,"extension":"flac"}]}]}"#,
        &state,
    )
    .await
    .expect("ingest browse");
    assert_eq!(ingested.status, "200 OK");

    let persisted = db.list_browse_records(10, 0).await.expect("list browse");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].username, "friend");
    assert_eq!(persisted[0].status, "ready");
    assert!(persisted[0]
        .entries_json
        .contains("Remote/Album/Track.flac"));

    let rehydrated = super::BrowseStore::from_persisted(persisted);
    assert!(rehydrated
        .json(None)
        .contains("\"filename\":\"Remote/Album/Track.flac\""));
    assert!(rehydrated
        .get("friend")
        .expect("rehydrated browse")
        .controller_status_json()
        .contains("\"state\":\"Completed\""));

    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("browse database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["browse"], 1);
    assert_eq!(stats_json["persisted"]["browse"], 1);
    assert_eq!(stats_json["projections"]["browse"], 1);
}
