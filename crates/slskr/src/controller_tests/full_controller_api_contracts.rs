//! Controller full controller api contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_regex_supports_dotnet_backtracking_constructs() {
    let cases = [
        (r"foo(?=bar)", "foobar", true),
        (r"foo(?!bar)", "foobaz", true),
        (r"(?<=foo)bar", "foobar", true),
        (r"(?<!foo)bar", "bazbar", true),
        (r"^(a+)\1$", "aaaa", true),
        (r"^(?<word>\w+)\s+\k<word>$", "same same", true),
        (r"^(?>a|ab)c$", "abc", false),
        (r"^(a)?b(?(1)c|d)$", "abc", true),
        (r"^(a)?b(?(1)c|d)$", "bd", true),
    ];

    for (expression, value, expected) in cases {
        let matcher = crate::ControllerRegex::compile_with_timeout(expression, true, None)
            .unwrap_or_else(|error| panic!("failed to compile {expression:?}: {error}"));
        assert_eq!(
            matcher.is_match(value),
            expected,
            "unexpected .NET-compatible match for {expression:?} against {value:?}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_regex_applies_global_case_mode_to_backreferences() {
    let insensitive =
        crate::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", false, None)
            .expect("case-insensitive named backreference");
    let sensitive =
        crate::ControllerRegex::compile_with_timeout(r"^(?<word>abc)\k<word>$", true, None)
            .expect("case-sensitive named backreference");

    assert!(insensitive.is_match("abcABC"));
    assert!(!sensitive.is_match("abcABC"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_swagger_routes_follow_the_target_specific_startup_default() {
    let (slskd, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let disabled = crate::route_http_request("GET", "/swagger/v0/swagger.json", None, "", &slskd)
        .await
        .expect("disabled slskd swagger route");
    assert_eq!(disabled.status, "404 Not Found");
    assert!(disabled.content_type.is_empty());
    assert!(disabled.body.is_empty());

    let (slskdn, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let enabled = crate::route_http_request("GET", "/swagger/v0/swagger.json", None, "", &slskdn)
        .await
        .expect("enabled slskdN swagger route");
    assert_eq!(enabled.status, "200 OK");
    let spec: serde_json::Value = serde_json::from_str(&enabled.body).unwrap();
    assert_eq!(spec["openapi"], "3.0.4");
    assert_eq!(spec["info"]["title"], "slskr API");
    assert!(spec["paths"]
        .as_object()
        .is_some_and(|paths| !paths.is_empty()));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_metrics_route_uses_its_own_basic_authentication() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_METRICS", "true")
            .with("SLSKD_METRICS_URL", "prometheus")
            .with("SLSKD_METRICS_USERNAME", "metrics-user")
            .with("SLSKD_METRICS_PASSWORD", "metrics-pass"),
    );
    let missing = crate::route_http_request("GET", "/prometheus", None, "", &state)
        .await
        .expect("missing metrics auth response");
    assert_eq!(missing.status, "401 Unauthorized");
    assert!(missing.body.is_empty());

    let malformed = crate::route_http_request("GET", "/prometheus", Some("Basic !!!"), "", &state)
        .await
        .expect("malformed metrics auth response");
    assert_eq!(malformed.status, "401 Unauthorized");

    let authorized = crate::route_http_request(
        "GET",
        "/prometheus",
        Some("Basic bWV0cmljcy11c2VyOm1ldHJpY3MtcGFzcw=="),
        "",
        &state,
    )
    .await
    .expect("authorized metrics response");
    assert_eq!(authorized.status, "200 OK");
    assert_eq!(
        authorized.content_type,
        "text/plain; version=0.0.4; charset=utf-8"
    );
    assert!(authorized.body.contains("# HELP slskr_session_connected"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_headless_suppresses_ui_but_retains_api_routes() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_HEADLESS", "true"),
    );
    for path in ["/", "/missing-client-route"] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("headless UI response");
        assert_eq!(response.status, "404 Not Found");
        assert!(response.body.is_empty());
    }
    let application = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("headless API response");
    assert_eq!(application.status, "200 OK");

    let login = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"slskd","password":"slskd"}"#,
        &state,
    )
    .await
    .expect("headless login response");
    assert_eq!(login.status, "403 Forbidden");
    assert!(login.body.is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_profile_has_no_native_global_rate_limiter() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_WEB_RATE_LIMITING", "true"),
    )
    .expect("slskd config");
    assert!(crate::controller_rate_limit_policy(
        &config,
        "GET",
        "/api/v0/searches",
        None,
        Some("192.0.2.31:1234".parse().unwrap()),
    )
    .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_release_comparison_matches_frozen_native_rules() {
    use crate::{is_newer_controller_release_available, normalize_controller_release_version};

    assert_eq!(
        normalize_controller_release_version(" refs/tags/V1.2.3+build.4 "),
        "1.2.3"
    );
    assert_eq!(
        normalize_controller_release_version("BUILD-DEV-20260717-slskdn.2"),
        "20260717-slskdn.2"
    );
    assert!(!is_newer_controller_release_available("1.2.3", "v1.2.3"));
    assert!(is_newer_controller_release_available("1.2.3", "1.2.4"));
    assert!(!is_newer_controller_release_available("1.2.4", "1.2.3"));
    assert!(is_newer_controller_release_available(
        "20260717-slskdn.1",
        "20260717-slskdn.2"
    ));
    assert!(!is_newer_controller_release_available(
        "20260718-slskdn.1",
        "20260717-slskdn.99"
    ));
    assert!(is_newer_controller_release_available("manual", "release"));
    assert!(!is_newer_controller_release_available("manual", ""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_file_delete_routes_are_forbidden_by_default() {
    let (state, _receiver) = test_state();
    let response = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("default remote file management policy");
    assert_eq!(response.status, "403 Forbidden");
    assert!(response.content_type.is_empty());
    assert!(response.body.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_file_delete_routes_are_scoped_to_storage_roots() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let download_file = state.config.downloads_dir.join("Remote").join("Song.mp3");
    std::fs::create_dir_all(download_file.parent().unwrap()).unwrap();
    std::fs::write(&download_file, b"song").unwrap();

    let aliased_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/unrelated/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased file delete");
    assert_eq!(aliased_delete.status, "404 Not Found");
    assert!(download_file.exists());

    let deleted = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("delete downloaded file");
    assert_eq!(deleted.status, "204 No Content");
    assert!(deleted.body.is_empty());
    assert!(!download_file.exists());

    let missing = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("delete missing downloaded file");
    assert_eq!(missing.status, "204 No Content");

    let (controller_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let controller_missing = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &controller_state,
    )
    .await
    .expect("slskd repeated file delete");
    assert_eq!(controller_missing.status, "204 No Content");

    let traversal = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/Li4vc2VjcmV0",
        None,
        "",
        &state,
    )
    .await
    .expect("delete traversal path");
    assert_eq!(traversal.status, "400 Bad Request");

    let newline_encoded_missing = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/directories/Wm05dg==%0A",
        None,
        "",
        &state,
    )
    .await
    .expect("delete slskd encoded directory");
    assert_eq!(newline_encoded_missing.status, "404 Not Found");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_base64_storage_paths_accept_mime_whitespace_but_reject_bad_data() {
    let raw = "a".repeat(80);
    let encoded = crate::STANDARD.encode(raw.as_bytes());
    let wrapped = format!("{}%0A{}%0D%0A", &encoded[..76], &encoded[76..]);
    assert_eq!(
        crate::controller_storage::decode_controller_base64_path_segment(&wrapped).unwrap(),
        raw
    );
    assert_eq!(
        crate::controller_storage::decode_controller_base64_path_segment("4KC+").unwrap(),
        "࠾"
    );
    assert!(crate::controller_storage::decode_controller_base64_path_segment("Zm9v%00").is_err());
    assert!(
        crate::controller_storage::decode_controller_base64_path_segment("not-base64!").is_err()
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_file_directory_routes_list_storage_roots() {
    let (state, _receiver) = test_state();
    let album = state.config.downloads_dir.join("Artist").join("Album");
    std::fs::create_dir_all(&album).unwrap();
    std::fs::write(album.join("Track.flac"), b"track").unwrap();

    let root = crate::route_http_request(
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

    let album_dir = crate::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories/QXJ0aXN0L0FsYnVt",
        None,
        "",
        &state,
    )
    .await
    .expect("list album dir");
    let aliased_album_dir = crate::route_http_request(
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
pub(super) fn controller_storage_directory_listing_is_bounded() {
    let (state, _receiver) = test_state();
    let dir = state.config.state_dir.join("bounded-listing");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("one.txt"), b"1").unwrap();
    std::fs::write(dir.join("two.txt"), b"2").unwrap();

    let json = crate::controller_storage_directory_json(
        &dir,
        None,
        crate::StorageDirectoryListOptions {
            recursive: false,
            limit: 1,
        },
    )
    .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).map_err(|e| e.to_string()))
    .expect("listing");
    assert_eq!(json["files"].as_array().unwrap().len(), 1);
    assert!(json.get("entryCount").is_none());
    assert!(json.get("truncated").is_none());

    let mut scanned = crate::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES - 1;
    crate::reserve_storage_scan_entry(&mut scanned).expect("last scan slot");
    assert_eq!(scanned, crate::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES);
    assert_eq!(
        crate::reserve_storage_scan_entry(&mut scanned).unwrap_err(),
        crate::STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_storage_directory_routes_ignore_unknown_pagination_parameters() {
    let (state, _receiver) = test_state();
    let root = state.config.downloads_dir.clone();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"a").unwrap();
    std::fs::write(root.join("b.txt"), b"b").unwrap();
    std::fs::write(root.join("c.txt"), b"c").unwrap();

    let response = crate::route_http_request(
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
pub(super) async fn controller_recursive_storage_listing_has_lower_budget() {
    let (state, _receiver) = test_state();
    let root = state.config.downloads_dir.clone();
    std::fs::create_dir_all(&root).unwrap();
    for index in 0..300 {
        std::fs::write(root.join(format!("{index:03}.txt")), b"x").unwrap();
    }

    let response = crate::route_http_request(
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
        crate::SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES
    );
    assert!(json.get("truncated").is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_recursive_storage_listing_bounds_directory_depth() {
    let (state, _receiver) = test_state();
    let root = state.config.state_dir.join("deep-storage-listing");
    let mut directory = root.clone();
    let mut relative = PathBuf::new();
    for depth in 0..(crate::SLSKD_STORAGE_MAX_RECURSION_DEPTH + 3) {
        let component = format!("d{depth:02}");
        directory.push(&component);
        relative.push(component);
        std::fs::create_dir_all(&directory).unwrap();
    }

    let json = crate::controller_storage_directory_json(
        &root,
        None,
        crate::StorageDirectoryListOptions {
            recursive: true,
            limit: crate::SLSKD_STORAGE_RECURSIVE_LIST_MAX_ENTRIES,
        },
    )
    .and_then(|json| {
        serde_json::from_str::<serde_json::Value>(&json).map_err(|error| error.to_string())
    })
    .expect("deep listing");
    assert!(json.get("truncated").is_none());

    let included = relative
        .components()
        .take(crate::SLSKD_STORAGE_MAX_RECURSION_DEPTH + 1)
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    let excluded = relative
        .components()
        .take(crate::SLSKD_STORAGE_MAX_RECURSION_DEPTH + 2)
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    let serialized = json.to_string();
    assert!(serialized.contains(&included), "{included}");
    assert!(!serialized.contains(&excluded), "{excluded}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mutating_api_routes_enqueue_session_commands() {
    let (state, mut receiver) = test_state();

    let routes = [
        ("/api/v0/session/connect", crate::SessionCommand::Connect),
        ("/api/v0/session/ping", crate::SessionCommand::Ping),
        (
            "/api/v0/session/disconnect",
            crate::SessionCommand::Disconnect,
        ),
        (
            "/api/v0/session/privileges/check",
            crate::SessionCommand::CheckPrivileges,
        ),
    ];

    for (path, expected_command) in routes {
        let response = crate::route_http_request("POST", path, None, "", &state)
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
pub(super) async fn controller_download_batch_enqueue_validates_dispatches_and_persists() {
    let (state, mut receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "batch-peer=127.0.0.1:2234",
    ));
    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let search_id = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    let request = format!(
        r#"{{"id":"{batch_id}","searchId":"{search_id}","username":"batch-peer","files":[{{"filename":"Music/A.flac","size":42}},{{"filename":"Music/B.flac","size":84}}],"options":{{"destination":"Albums","externalId":"ignored-by-frozen-slskd"}}}}"#
    );

    let created = crate::route_http_request(
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

    let fetched = crate::route_http_request(
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

    let duplicate_id = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &request,
        &state,
    )
    .await
    .expect("duplicate batch id");
    assert_eq!(duplicate_id.status, "409 Conflict");

    let partial = crate::route_http_request(
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

    let all_failed = crate::route_http_request(
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
        crate::SessionCommand::TransferPeer { username, .. } if username == "batch-peer"
    )));

    for body in [
        r#"{"username":"batch-peer","files":[{"filename":"Music/A.flac","size":42},{"filename":"Music/A.flac","size":42}]}"#,
        r#"{"username":"batch-peer","files":[{"filename":"../escape.flac","size":42}]}"#,
        r#"{"username":"batch-peer","files":[{"filename":"Music/X.flac","size":-1}]}"#,
    ] {
        let invalid = crate::route_http_request(
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
    let throttled = crate::route_http_request(
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
pub(super) async fn nested_resource_paths_cannot_mutate_flat_record_ids() {
    let (state, _receiver) = test_state();
    let now = crate::unix_timestamp();
    state
        .user_notes
        .write()
        .await
        .records
        .push(crate::user_note_store::UserNoteRecord {
            id: "note/nested".to_owned(),
            username: "friend".to_owned(),
            note: "keep".to_owned(),
            color: String::new(),
            icon: String::new(),
            is_high_priority: false,
            created_at: now,
            updated_at: now,
        });
    state
        .interests
        .write()
        .await
        .liked
        .push(crate::interest_store::InterestRecord {
            id: "liked/nested".to_owned(),
            name: "jazz".to_owned(),
            kind: "liked".to_owned(),
            created_at: now,
        });
    state
        .library
        .write()
        .await
        .records
        .push(crate::LibraryItemRecord {
            id: "lib/nested".to_owned(),
            artist: "artist".to_owned(),
            title: "title".to_owned(),
            kind: "Audio".to_owned(),
            created_at: now,
        });

    for path in [
        "/api/users/notes/note/nested",
        "/api/soulseek/interests/liked/nested",
        "/api/library/items/lib/nested",
    ] {
        let response = crate::route_http_request("DELETE", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(response.status, "404 Not Found", "{path}");
    }
    let update = crate::route_http_request(
        "PUT",
        "/api/users/notes/note/nested",
        None,
        r#"{"note":"changed"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(update.status, "404 Not Found");

    assert_eq!(
        state.user_notes.read().await.records[0].note,
        "keep",
        "nested PUT must not reach the user-note store"
    );
    assert_eq!(state.interests.read().await.liked.len(), 1);
    assert_eq!(state.library.read().await.records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_transfer_position_requests_and_returns_the_remote_queue_place() {
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/transfers/downloads/{username}/{id}/position",
                "case": $case,
                "pass": true,
            }));
        };
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind queue position fixture");
    let local_addr = listener
        .local_addr()
        .expect("queue position fixture address");
    let server = tokio::spawn(async move {
        for (expected_filename, expected_place) in
            [("Remote/Two.flac", 4_u32), ("Remote/One.flac", 7_u32)]
        {
            let (stream, _) = listener.accept().await.expect("accept queue position");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            assert_eq!(
                init.receive().await.expect("queue position init"),
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "tester".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("queue position request"),
                crate::PeerMessage::PlaceInQueueRequest {
                    filename: expected_filename.to_owned(),
                }
            );
            peer.send(&crate::PeerMessage::PlaceInQueueResponse {
                filename: expected_filename.to_owned(),
                place: expected_place,
            })
            .await
            .expect("queue position response");
        }
    });
    let endpoint = format!("friend={local_addr}");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", &endpoint),
    );
    state.session.write().await.state = "connected";
    let (first_id, second_id) = {
        let mut transfers = state.transfers.write().await;
        let first = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/One.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.update_status(first.id, "completed", Some(1), None);
        let second = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Two.flac".to_owned(),
            None,
            Some(2),
        );
        (first.id, second.id)
    };

    let active = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/friend/{second_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("active position");
    assert_eq!(active.status, "200 OK", "{}", active.body);
    assert_eq!(active.body, "4");
    record_evidence!("nominal-status-headers-body");
    record_evidence!("populated-dynamic-state");

    let completed = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/friend/{first_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("completed position");
    assert_eq!(completed.status, "200 OK");
    assert_eq!(completed.body, "7");

    // An id that isn't a real download for this username must 404,
    // not silently report a position.
    let unknown_id = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/friend/999999/position",
        None,
        "",
        &state,
    )
    .await
    .expect("unknown id");
    assert_eq!(unknown_id.status, "404 Not Found");

    let wrong_username = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/other/{second_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("wrong username");
    assert_eq!(wrong_username.status, "404 Not Found");
    record_evidence!("missing-empty-or-conflict-state");
    server.await.expect("queue position fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("transfer_position_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn enabled_warm_cache_hints_normalize_persist_and_bound_popularity() {
    let (state, _receiver) = test_state();
    std::fs::write(
        state.config.state_dir.join("slskd.yml"),
        "warmCache:\n  enabled: true\n",
    )
    .unwrap();

    let accepted = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[" rel-1 ","REL-1"],"mb_artist_ids":["artist-1"],"mb_label_ids":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(accepted.status, "200 OK", "{}", accepted.body);
    assert_eq!(accepted.body, r#"{"accepted":true}"#);

    let features = state.controller_features.read().await;
    assert_eq!(
        features
            .get("warm-cache/popularity/mb:release:rel-1")
            .unwrap()["hits"],
        1
    );
    assert_eq!(
        features
            .get("warm-cache/popularity/mb:artist:artist-1")
            .unwrap()["hits"],
        1
    );
    drop(features);

    let invalid = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[42]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid.status, "400 Bad Request");

    let oversized = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        &serde_json::json!({"mb_release_ids": ["x".repeat(129)]}).to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(oversized.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn extended_controller_mutations_are_stateful_and_domain_backed() {
    let (state, _receiver) = test_state();

    let opinion = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"stateful-test","subjectType":"Track","subjectId":"recording-1","kind":"Like","strength":0.75,"confidence":1,"comment":"good"}"#,
        &state,
    )
    .await
    .expect("create opinion");
    assert_eq!(opinion.status, "200 OK");
    let opinion_json = serde_json::from_str::<serde_json::Value>(&opinion.body).unwrap();
    let opinion_id = opinion_json["id"].as_str().unwrap();
    let opinions = crate::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .expect("list opinions");
    assert!(opinions.body.contains("recording-1"));

    let circuit = crate::route_http_request(
        "POST",
        "/api/v0/security/circuits",
        None,
        r#"{"id":"circuit-1","peerId":"peer-1","active":true}"#,
        &state,
    )
    .await
    .expect("create circuit");
    assert_eq!(circuit.status, "400 Bad Request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&circuit.body).unwrap(),
        serde_json::json!({"error": "Circuit building failed"})
    );
    let circuits = crate::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
        .await
        .expect("list circuits");
    assert_eq!(circuits.body, "[]");

    let descriptor = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        None,
        &serde_json::json!({
            "descriptor": {
                "contentId": "cid-1",
                "hashes": [{"algorithm": "sha256", "hex": "0123456789abcdef"}],
                "signature": {
                    "publicKey": "key",
                    "signature": "0123456789abcdef",
                    "timestampUnixMs": crate::unix_timestamp_millis(),
                },
            },
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish descriptor");
    assert_eq!(descriptor.status, "200 OK");
    let descriptor_stats = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        None,
        "",
        &state,
    )
    .await
    .expect("descriptor stats");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&descriptor_stats.body).unwrap()
            ["activeCacheEntries"],
        0
    );

    let created_pod = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-controller","name":"Controller Pod","isPublic":true}}"#,
        &state,
    )
    .await
    .expect("create pod");
    assert_eq!(created_pod.status, "201 Created");
    let channel = crate::route_http_request(
        "POST",
        "/api/v0/podcore/pod-controller/channels",
        None,
        r#"{"channelId":"general","name":"General"}"#,
        &state,
    )
    .await
    .expect("create pod channel");
    assert_eq!(channel.status, "201 Created");
    let channels = crate::route_http_request(
        "GET",
        "/api/v0/podcore/pod-controller/channels",
        None,
        "",
        &state,
    )
    .await
    .expect("list pod channels");
    assert!(channels.body.contains("general"));

    let keypair = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate pod signing keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    // Verification resolves the sender's public key from real pod
    // membership, matching the oracle -- not from a client-supplied
    // field, which would let anyone "verify" a self-made signature
    // against a self-made key. Register "tester" as a real member
    // with the generated public key so verification has a real key
    // to check against.
    state
        .pods
        .write()
        .await
        .upsert_member(
            "pod-controller",
            crate::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");
    let signed = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId":"message-1",
                "podId":"pod-controller",
                "senderPeerId":"tester",
                "body":"hello",
                "timestampUnixMs": crate::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign pod message");
    assert_eq!(signed.status, "200 OK");
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap();
    assert!(
        signed_json["signature"]
            .as_str()
            .unwrap()
            .starts_with("ed25519:"),
        "{signed_json}"
    );
    let verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &signed.body,
        &state,
    )
    .await
    .expect("verify pod message");
    assert_eq!(verified.body, r#"{"isValid":true}"#, "{}", verified.body);

    // A signature that doesn't match the sender's real registered
    // public key must fail -- not a fake "isValid: true" for whatever
    // key the caller happens to supply.
    let mut forged = signed_json.clone();
    forged["message"]["senderPeerId"] = serde_json::json!("someone-else");
    let forged_verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &state,
    )
    .await
    .expect("verify forged sender");
    assert_eq!(forged_verified.body, r#"{"isValid":false}"#);

    let ranked = crate::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":"slow","filename":"x.flac","uploadSpeed":10},{"username":"fast","filename":"x.flac","uploadSpeed":10000,"hasFreeUploadSlot":true}]"#,
        &state,
    )
    .await
    .expect("rank sources");
    let ranked_json = serde_json::from_str::<serde_json::Value>(&ranked.body).unwrap();
    assert_eq!(ranked_json[0]["username"], "fast");

    let removed_opinion = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/opinions/{opinion_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("remove opinion");
    assert_eq!(removed_opinion.status, "204 No Content");
    let removed_descriptor = crate::route_http_request(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/cid-1",
        None,
        "",
        &state,
    )
    .await
    .expect("remove descriptor");
    assert_eq!(removed_descriptor.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&removed_descriptor.body).unwrap()
            ["wasPublished"],
        true
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_feature_state_persists_bounded_records() {
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-feature-state-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut state = crate::ControllerFeatureState::load(&root).unwrap();
    state
        .upsert(
            "opinion/one".to_owned(),
            serde_json::json!({"id":"one","score":1.0}),
        )
        .unwrap();
    drop(state);
    let reloaded = crate::ControllerFeatureState::load(&root).unwrap();
    assert_eq!(reloaded.get("opinion/one").unwrap()["score"], 1.0);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_user_browse_routes_page_directories_and_directory_files() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/One.flac\",\"size\":1},{\"filename\":\"Remote/Album/Two.flac\",\"size\":2},{\"filename\":\"Remote/Album/Three.flac\",\"size\":3},{\"filename\":\"Remote/Other/Four.flac\",\"size\":4}]}",
        &state,
    )
    .await
    .expect("browse ingest");

    let root = crate::route_http_request(
        "GET",
        "/api/users/friend/browse?offset=1&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("paged root browse");
    assert_eq!(root.status, "200 OK");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["directoryCount"], 2);
    assert_eq!(root_json["filteredDirectoryCount"], 2);
    assert_eq!(root_json["fileCount"], 4);
    assert_eq!(root_json["filteredFileCount"], 4);
    assert_eq!(root_json["totalBytes"], 10);
    assert_eq!(root_json["offset"], 1);
    assert_eq!(root_json["limit"], 1);
    assert_eq!(root_json["directories"].as_array().unwrap().len(), 1);
    assert_eq!(root_json["directories"][0]["name"], "Remote/Other");
    assert_eq!(root_json["directories"][0]["filteredFileCount"], 1);
    assert_eq!(root_json["directories"][0]["totalBytes"], 4);

    let directory = crate::route_http_request(
        "POST",
        "/api/users/friend/directory?offset=1&limit=1",
        None,
        "{\"directory\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("paged directory browse");
    assert_eq!(directory.status, "200 OK");
    let directory_json = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap();
    assert_eq!(directory_json[0]["name"], "Remote/Album");
    assert_eq!(directory_json[0]["fileCount"], 3);
    assert_eq!(directory_json[0]["filteredFileCount"], 3);
    assert_eq!(directory_json[0]["totalBytes"], 6);
    assert_eq!(directory_json[0]["offset"], 1);
    assert_eq!(directory_json[0]["limit"], 1);
    assert_eq!(directory_json[0]["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        directory_json[0]["files"][0]["filename"],
        "Remote/Album/Two.flac"
    );
}
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_user_browse_routes_filter_directories_and_files() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/One.flac\",\"size\":1},{\"filename\":\"Remote/Album/Two.mp3\",\"size\":2},{\"filename\":\"Remote/Other/Four.flac\",\"size\":4},{\"filename\":\"Remote/Special/Needle.wav\",\"size\":5}]}",
        &state,
    )
    .await
    .expect("browse ingest");

    let root = crate::route_http_request(
        "GET",
        "/api/users/friend/browse?q=special",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered root browse");
    assert_eq!(root.status, "200 OK");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["directoryCount"], 3);
    assert_eq!(root_json["filteredDirectoryCount"], 1);
    assert_eq!(root_json["fileCount"], 4);
    assert_eq!(root_json["filteredFileCount"], 1);
    assert_eq!(root_json["totalBytes"], 5);
    assert_eq!(root_json["directories"].as_array().unwrap().len(), 1);
    assert_eq!(root_json["directories"][0]["name"], "Remote/Special");
    assert_eq!(root_json["directories"][0]["filteredFileCount"], 1);
    assert_eq!(root_json["directories"][0]["totalBytes"], 5);

    let directory = crate::route_http_request(
        "POST",
        "/api/users/friend/directory?q=two",
        None,
        "{\"directory\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("filtered directory browse");
    assert_eq!(directory.status, "200 OK");
    let directory_json = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap();
    assert_eq!(directory_json[0]["fileCount"], 2);
    assert_eq!(directory_json[0]["filteredFileCount"], 1);
    assert_eq!(directory_json[0]["totalBytes"], 2);
    assert_eq!(directory_json[0]["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        directory_json[0]["files"][0]["filename"],
        "Remote/Album/Two.mp3"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_auth_enforces_native_roles_schemes_scopes_and_anonymous_routes() {
    let state_dir =
        std::env::temp_dir().join(format!("slskr-slskdn-auth-test-{}", uuid::Uuid::new_v4()));
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", state_dir.to_str().unwrap())
            .with("SLSKR_API_TOKEN", "admin-token")
            .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "read-token")
            .with("SLSKR_API_NOWPLAYING_TOKEN", "nowplaying-token"),
    )
    .expect("role token config");
    let headers = crate::RequestSecurityHeaders::default();
    let check = |method, path, authorization| {
        crate::routing::check_route_auth(&config, method, path, authorization, &headers)
    };

    assert_eq!(check("GET", "/api/v0/session", None), Err("unauthorized"));
    assert!(check("GET", "/api/v0/transfers", Some("Bearer read-token")).is_ok());
    assert!(check("GET", "/api/v0/transfers", Some("bEaReR read-token")).is_ok());
    assert_eq!(
        check(
            "POST",
            "/api/v0/transfers/downloads/peer",
            Some("Bearer read-token")
        ),
        Err("forbidden")
    );
    assert!(check(
        "POST",
        "/api/v0/transfers/downloads/peer",
        Some("Bearer write-token")
    )
    .is_ok());
    assert_eq!(
        check("GET", "/api/v0/security/status", Some("Bearer write-token")),
        Err("forbidden")
    );
    assert!(check("GET", "/api/v0/security/status", Some("Bearer admin-token")).is_ok());
    assert_eq!(
        check("PUT", "/api/v0/application", Some("ApiKey admin-token")),
        Err("forbidden")
    );
    assert!(check("PUT", "/api/v0/application", Some("Bearer admin-token")).is_ok());
    assert!(check(
        "POST",
        "/api/v0/nowplaying/webhook",
        Some("ApiKey nowplaying-token")
    )
    .is_ok());
    assert_eq!(
        check("GET", "/api/v0/transfers", Some("ApiKey nowplaying-token")),
        Err("forbidden")
    );

    let delegated_headers = crate::RequestSecurityHeaders {
        origin: Some("https://recipient.example".to_owned()),
        host: Some("owner.example".to_owned()),
        x_share_token: Some("share-token".to_owned()),
        ..Default::default()
    };
    assert!(crate::routing::check_route_auth(
        &config,
        "POST",
        "/api/v0/share-grants/grant-1/backfill",
        None,
        &delegated_headers,
    )
    .is_ok());
    assert_eq!(
        check("POST", "/api/v0/share-grants/grant-1/backfill", None,),
        Err("unauthorized")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn controller_auth_selects_the_frozen_controller_policy_registry() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-controller-auth-test-{}",
        uuid::Uuid::new_v4()
    ));
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", state_dir.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_API_TOKEN", "admin-token")
            .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "read-token"),
    )
    .expect("slskd auth profile");
    let headers = crate::RequestSecurityHeaders::default();
    let check = |method, path, authorization| {
        crate::routing::check_route_auth(&config, method, path, authorization, &headers)
    };

    for (method, path) in [
        ("GET", "/api/v0/logs"),
        ("POST", "/api/v0/searches"),
        ("GET", "/api/v0/application/dump"),
        ("POST", "/api/v0/transfers/downloads/batches"),
    ] {
        assert_eq!(check(method, path, None), Err("unauthorized"), "{path}");
        assert!(
            check(method, path, Some("Bearer read-token")).is_ok(),
            "{path}"
        );
    }

    let slskdn = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", state_dir.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_API_TOKEN", "admin-token")
            .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "read-token"),
    )
    .expect("slskdN auth profile");
    assert_eq!(
        crate::routing::check_route_auth(
            &slskdn,
            "GET",
            "/api/v0/logs",
            Some("Bearer read-token"),
            &headers,
        ),
        Err("forbidden")
    );
    assert_eq!(
        crate::routing::check_route_auth(
            &slskdn,
            "POST",
            "/api/v0/searches",
            Some("Bearer read-token"),
            &headers,
        ),
        Err("forbidden")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn controller_debug_view_projects_frozen_default_authentication_values() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_CONFIGURATION", "true")
            .with("SLSKR_DEBUG", "true"),
    );
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
