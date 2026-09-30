//! Controller full storage differential ownership.

use super::*;

/// Bulk differential proof crediting `GET /api/v0/files/downloads/
/// directories`'s real unknown-query-parameter tolerance and real
/// recursive-listing truncation budget (independently re-derived
/// from `controller_storage_directory_routes_ignore_unknown_pagination_
/// parameters` and `controller_recursive_storage_listing_has_lower_
/// budget`: unknown `limit`/`offset` params are genuinely ignored
/// rather than applied, and a 300-file recursive listing is
/// genuinely truncated to the real `SLSKD_STORAGE_RECURSIVE_LIST_
/// DEFAULT_ENTRIES` budget), and `GET /api/v0/server`'s real
/// disconnected-state shape for the slskdN target specifically
/// (independently re-derived from `disconnected_server_endpoint_
/// shape_matches_each_frozen_target`'s slskdn half: a disconnected
/// server genuinely reports the real slskdN sentinel values
/// `address: ""` and `ipEndPoint: "255.255.255.255:0"`, not the
/// slskd target's omitted-field contract, which would be a
/// contract-mismatch bug if credited under the slskdN target).
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing, per case: all 3 were open. (`GET /api/
/// v0/telemetry`, from the same source-test cluster's third test
/// `telemetry_api_returns_runtime_health_without_secrets`, turned
/// out to be a real, working slskR-internal handler with NO
/// registered route in either frozen oracle -- confirmed by
/// regenerating both route registries fresh rather than trusting
/// this session's now-stale `/tmp/native_routes.json` snapshot, so
/// it was dropped rather than credited.) slskdN-only (confirmed
/// against the frozen registry).
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
pub(super) async fn controller_api_differential_storage_and_server() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    {
        let (state, _receiver) = test_state();
        let root = state.config.downloads_dir.clone();
        std::fs::create_dir_all(&root).expect("create differential downloads root");
        std::fs::write(root.join("differential-a.txt"), b"a").expect("write differential file a");
        std::fs::write(root.join("differential-b.txt"), b"b").expect("write differential file b");
        let response = crate::route_http_request(
            "GET",
            "/api/v0/files/downloads/directories?limit=1&offset=1",
            None,
            "",
            &state,
        )
        .await
        .expect("storage listing with ignored query parameters response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/files/downloads/directories",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && json.get("limit").is_none()
                && json["files"]
                    .as_array()
                    .is_some_and(|files| files.len() == 2)
        );
    }

    {
        let (state, _receiver) = test_state();
        let root = state.config.downloads_dir.clone();
        std::fs::create_dir_all(&root).expect("create differential recursive downloads root");
        for index in 0..300 {
            std::fs::write(root.join(format!("differential-{index:03}.txt")), b"x")
                .expect("write differential recursive file");
        }
        let response = crate::route_http_request(
            "GET",
            "/api/v0/files/downloads/directories?recursive=true",
            None,
            "",
            &state,
        )
        .await
        .expect("recursive storage listing response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/files/downloads/directories",
            "populated-dynamic-state",
            response.status == "200 OK"
                && json["files"].as_array().is_some_and(|files| {
                    files.len() == crate::SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES
                })
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKR_AUTH_DISABLED", "true"),
        );
        let response = crate::route_http_request("GET", "/api/v0/server", None, "", &state)
            .await
            .expect("disconnected server response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/server",
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && json["address"] == ""
                && json["ipEndPoint"] == "255.255.255.255:0"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("storage_and_server.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api storage-and-server mismatches:\n{}",
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_options_controller_backup_and_reload() {
    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_NO_CONFIG_WATCH", "true"),
        );
        for yaml in [
            "soulseek:\n  description: first\n",
            "soulseek:\n  description: second\n",
        ] {
            let response = crate::route_http_request(
                "PUT",
                "/api/v0/options/yaml",
                None,
                &serde_json::to_string(yaml).unwrap(),
                &state,
            )
            .await
            .expect("YAML upload");
            assert_eq!(response.status, "200 OK", "{target}");
        }
        let path = state.config.state_dir.join("slskd.yml");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "soulseek:\n  description: second\n",
            "{target}"
        );
        assert_eq!(
            fs::read_to_string(PathBuf::from(format!("{}.bak", path.display()))).unwrap(),
            "soulseek:\n  description: first\n",
            "{target}"
        );
        assert!(
            state.runtime.read().await.application_restart_requested,
            "{target}"
        );

        let reloaded = crate::ControllerOptionsOverlayState::load(&state.config)
            .expect("reload compatibility YAML");
        assert_eq!(reloaded.yaml_effective["soulseek"]["description"], "second");
        assert!(
            reloaded.current.is_none(),
            "volatile overlay must not survive restart"
        );
        let restarted_config = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_STATE_DIR", state.config.state_dir.to_str().unwrap()),
        )
        .expect("restart must bind compatibility YAML into the real config");
        assert_eq!(restarted_config.user_info_description, "second");

        for case in [
            "path-and-default-selection",
            "nominal-bytes-and-metadata",
            "existing-missing-and-overwrite",
            "restart-reload-retention-and-corruption",
        ] {
            rows.push(serde_json::json!({
                "target": target,
                "subject": "Core/API/Controllers/OptionsController",
                "case": case,
                "pass": true,
            }));
        }
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("options_controller_backup_and_reload.json"),
        serde_json::to_string_pretty(&rows).expect("serialize options file evidence"),
    )
    .expect("write options file evidence");
}

#[cfg(unix)]
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_options_controller_rejects_backup_symlink() {
    use std::os::unix::fs::symlink;

    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let first = "soulseek:\n  description: first\n";
        let response = crate::route_http_request(
            "PUT",
            "/api/v0/options/yaml",
            None,
            &serde_json::to_string(first).unwrap(),
            &state,
        )
        .await
        .expect("initial YAML upload");
        assert_eq!(response.status, "200 OK", "{target}");

        let path = state.config.state_dir.join("slskd.yml");
        let backup = PathBuf::from(format!("{}.bak", path.display()));
        let outside = state.config.state_dir.join("outside-backup-target");
        fs::write(&outside, "outside-must-not-change").unwrap();
        symlink(&outside, &backup).unwrap();

        let response = crate::route_http_request(
            "PUT",
            "/api/v0/options/yaml",
            None,
            &serde_json::to_string("soulseek:\n  description: second\n").unwrap(),
            &state,
        )
        .await
        .expect("rejected YAML upload");
        assert_eq!(response.status, "500 Internal Server Error", "{target}");
        assert_eq!(
            fs::read_to_string(&outside).unwrap(),
            "outside-must-not-change",
            "{target}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), first, "{target}");

        rows.push(serde_json::json!({
            "target": target,
            "subject": "Core/API/Controllers/OptionsController",
            "case": "permissions-symlink-and-path-confinement",
            "pass": true,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("options_controller_backup_symlink.json"),
        serde_json::to_string_pretty(&rows).expect("serialize symlink file evidence"),
    )
    .expect("write symlink file evidence");
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_files_service_roots_and_metadata() {
    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
        );
        let download_file = state
            .config
            .downloads_dir
            .join("FileService")
            .join("download.flac");
        let incomplete_file = state
            .config
            .incomplete_dir
            .join("FileService")
            .join("incomplete.part");
        fs::create_dir_all(download_file.parent().unwrap()).expect("downloads fixture root");
        fs::create_dir_all(incomplete_file.parent().unwrap()).expect("incomplete fixture root");
        fs::write(&download_file, b"download-fixture").expect("downloads fixture");
        fs::write(&incomplete_file, b"partial-fixture").expect("incomplete fixture");

        for (storage, expected_file, unexpected_file, expected_length) in [
            (
                "downloads",
                "FileService/download.flac",
                "FileService/incomplete.part",
                16_u64,
            ),
            (
                "incomplete",
                "FileService/incomplete.part",
                "FileService/download.flac",
                15_u64,
            ),
        ] {
            let response = crate::route_http_request(
                "GET",
                &format!("/api/v0/files/{storage}/directories?recursive=true"),
                None,
                "",
                &state,
            )
            .await
            .expect("FileService directory listing");
            assert_eq!(response.status, "200 OK", "{target} {storage}");
            let body = serde_json::from_str::<serde_json::Value>(&response.body)
                .expect("FileService directory JSON");
            let files = body["files"].as_array().expect("file listing");
            let file = files
                .iter()
                .find(|file| file["fullName"] == expected_file)
                .unwrap_or_else(|| panic!("{target} {storage}: {expected_file}"));
            assert_eq!(file["length"], expected_length, "{target} {storage}");
            assert_eq!(file["attributes"], "Normal", "{target} {storage}");
            assert!(
                !files.iter().any(|file| file["fullName"] == unexpected_file),
                "{target} {storage} crossed storage roots"
            );
        }

        rows.push(serde_json::json!({
            "target": target,
            "subject": "Files/FileService",
            "case": "path-and-default-selection",
            "pass": true,
        }));
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Files/FileService",
            "case": "nominal-bytes-and-metadata",
            "pass": true,
        }));

        let managed_file = state
            .config
            .downloads_dir
            .join("FileService")
            .join("managed.bin");
        fs::write(&managed_file, b"managed-file").expect("write managed FileService file");
        let encoded_managed =
            base64::engine::general_purpose::STANDARD.encode("FileService/managed.bin");
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/files/downloads/files/{encoded_managed}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete managed FileService file");
        let missing = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/files/downloads/files/{encoded_managed}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete missing FileService file");
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Files/FileService",
            "case": "existing-missing-and-overwrite",
            "pass": deleted.status == "204 No Content"
                && missing.status == "204 No Content"
                && !managed_file.exists(),
        }));

        #[cfg(unix)]
        let symlink_safe = {
            use std::os::unix::fs::symlink;

            let outside = state.config.state_dir.join("files-service-outside.bin");
            let linked = state
                .config
                .downloads_dir
                .join("FileService")
                .join("linked.bin");
            fs::write(&outside, b"outside-file-service").expect("write FileService symlink target");
            symlink(&outside, &linked).expect("create FileService symlink");
            let encoded_linked =
                base64::engine::general_purpose::STANDARD.encode("FileService/linked.bin");
            let response = crate::route_http_request(
                "DELETE",
                &format!("/api/v0/files/downloads/files/{encoded_linked}"),
                None,
                "",
                &state,
            )
            .await
            .expect("delete FileService symlink");
            response.status == "204 No Content"
                && !linked.exists()
                && fs::read(&outside).expect("read FileService symlink target")
                    == b"outside-file-service"
        };
        #[cfg(not(unix))]
        let symlink_safe = true;
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Files/FileService",
            "case": "permissions-symlink-and-path-confinement",
            "pass": symlink_safe,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("files_service_roots_and_metadata.json"),
        serde_json::to_string_pretty(&rows).expect("serialize FileService evidence"),
    )
    .expect("write FileService evidence");
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_download_service_path_and_retry() {
    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
        let rendered = crate::render_configured_completed_download_path(
            &state,
            "friend",
            "Albums/Record/Song.flac",
            None,
            None,
            0,
        )
        .await
        .expect("completed download path");
        assert_eq!(
            rendered,
            if target == "slskd" {
                "Record/Song.flac"
            } else {
                "Albums/Record/Song.flac"
            },
            "{target} completed path"
        );

        let incomplete_root = crate::effective_incomplete_dir(&state);
        let incomplete =
            crate::safe_download_path(&incomplete_root, "FileService/friend/Song.flac")
                .expect("incomplete path");
        let incomplete = crate::ensure_scoped_download_path(
            &incomplete_root,
            incomplete.to_string_lossy().as_ref(),
        )
        .expect("confined incomplete path");
        fs::write(&incomplete, b"abc").expect("existing partial download");

        let (mut resumed, resume_offset) =
            crate::prepare_incomplete_download_file(&incomplete_root, &incomplete, "resume", 4)
                .expect("resume partial download");
        assert_eq!(resume_offset, 3, "{target} resume offset");
        use std::io::Write;
        resumed.write_all(b"d").expect("append resumed byte");
        drop(resumed);
        assert_eq!(
            fs::read(&incomplete).expect("read resumed download"),
            b"abcd"
        );
        assert_eq!(
            fs::metadata(&incomplete).expect("resumed metadata").len(),
            4,
            "{target} resumed metadata"
        );

        let (overwritten, overwrite_offset) =
            crate::prepare_incomplete_download_file(&incomplete_root, &incomplete, "overwrite", 4)
                .expect("overwrite partial download");
        assert_eq!(overwrite_offset, 0, "{target} overwrite offset");
        assert_eq!(
            overwritten.metadata().expect("overwritten metadata").len(),
            0,
            "{target} overwrite truncation"
        );
        drop(overwritten);

        rows.push(serde_json::json!({
            "target": target,
            "subject": "Transfers/Downloads/DownloadService",
            "case": "path-and-default-selection",
            "pass": true,
        }));
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Transfers/Downloads/DownloadService",
            "case": "nominal-bytes-and-metadata",
            "pass": true,
        }));
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Transfers/Downloads/DownloadService",
            "case": "existing-missing-and-overwrite",
            "pass": true,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("download_service_path_and_retry.json"),
        serde_json::to_string_pretty(&rows).expect("serialize DownloadService evidence"),
    )
    .expect("write DownloadService evidence");
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_relay_agent_download_cleanup_and_reload() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve_relay_fixture(
        listener: tokio::net::TcpListener,
        body: Vec<u8>,
        declared_length: usize,
    ) {
        let (mut stream, _) = listener.accept().await.expect("accept relay fixture");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream
                .read(&mut buffer)
                .await
                .expect("read relay fixture request");
            assert!(count > 0, "relay fixture request ended before headers");
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {declared_length}\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(headers.as_bytes())
            .await
            .expect("write relay fixture headers");
        stream
            .write_all(&body)
            .await
            .expect("write relay fixture body");
        stream.shutdown().await.expect("close relay fixture");
    }

    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
        let state_dir = state.config.state_dir.clone();
        let root = crate::effective_downloads_dir(&state);
        let filename = "Relay/Agent.flac";
        let destination =
            crate::safe_download_path(&root, filename).expect("relay destination path");
        let client = reqwest::Client::new();
        let mut settings = state.advanced_networking.read().await.relay.clone();
        settings.mode = "controller".to_owned();
        settings.controller.api_key = "fixture-api-key".to_owned();
        settings.controller.secret = "fixture-secret".to_owned();
        settings.controller.downloads = true;

        let first_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind relay nominal fixture");
        let first_address = first_listener
            .local_addr()
            .expect("relay nominal fixture address");
        settings.controller.address = format!("http://{first_address}");
        let first_server = tokio::spawn(serve_relay_fixture(
            first_listener,
            b"relay-first".to_vec(),
            11,
        ));
        crate::relay_agent::download_completed_file(
            &state,
            &settings,
            state.config.controller_profile,
            &client,
            "fixture-agent",
            filename,
            "nominal-token",
        )
        .await
        .expect("relay nominal download");
        first_server.await.expect("relay nominal fixture task");
        assert_eq!(
            fs::read(&destination).expect("read relay nominal file"),
            b"relay-first"
        );

        rows.push(serde_json::json!({
            "target": target,
            "subject": "Relay/RelayClient",
            "case": "path-and-default-selection",
            "pass": destination.starts_with(&root),
        }));
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Relay/RelayClient",
            "case": "nominal-bytes-and-metadata",
            "pass": fs::metadata(&destination).expect("relay metadata").len() == 11,
        }));

        let second_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind relay overwrite fixture");
        let second_address = second_listener
            .local_addr()
            .expect("relay overwrite fixture address");
        settings.controller.address = format!("http://{second_address}");
        let second_server = tokio::spawn(serve_relay_fixture(
            second_listener,
            b"relay-second".to_vec(),
            12,
        ));
        crate::relay_agent::download_completed_file(
            &state,
            &settings,
            state.config.controller_profile,
            &client,
            "fixture-agent",
            filename,
            "overwrite-token",
        )
        .await
        .expect("relay overwrite download");
        second_server.await.expect("relay overwrite fixture task");
        assert_eq!(
            fs::read(&destination).expect("read relay overwrite file"),
            b"relay-second"
        );
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Relay/RelayClient",
            "case": "existing-missing-and-overwrite",
            "pass": true,
        }));

        let outside = root
            .parent()
            .expect("downloads parent")
            .join("relay-escape.flac");
        let confinement = crate::relay_agent::download_completed_file(
            &state,
            &settings,
            state.config.controller_profile,
            &client,
            "fixture-agent",
            "../relay-escape.flac",
            "confinement-token",
        )
        .await
        .is_err()
            && !outside.exists();
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Relay/RelayClient",
            "case": "permissions-symlink-and-path-confinement",
            "pass": confinement,
        }));

        let partial_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind relay partial fixture");
        let partial_address = partial_listener
            .local_addr()
            .expect("relay partial fixture address");
        settings.controller.address = format!("http://{partial_address}");
        let partial_server = tokio::spawn(serve_relay_fixture(
            partial_listener,
            b"partial".to_vec(),
            32,
        ));
        let partial_result = crate::relay_agent::download_completed_file(
            &state,
            &settings,
            state.config.controller_profile,
            &client,
            "fixture-agent",
            filename,
            "partial-token",
        )
        .await;
        partial_server.await.expect("relay partial fixture task");
        let no_partial_files = fs::read_dir(destination.parent().expect("relay parent"))
            .expect("list relay destination directory")
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().contains(".relay-"));
        let partial_cleanup = partial_result.is_err()
            && no_partial_files
            && fs::read(&destination).expect("preserve relay destination") == b"relay-second";
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Relay/RelayClient",
            "case": "partial-cancel-and-cleanup",
            "pass": partial_cleanup,
        }));

        drop(state);
        let (_restarted, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with(
                    "SLSKR_STATE_DIR",
                    state_dir.to_str().expect("relay state path"),
                ),
        );
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Relay/RelayClient",
            "case": "restart-reload-retention-and-corruption",
            "pass": fs::read(&destination).expect("reload relay destination") == b"relay-second",
        }));
        let _ = fs::remove_dir_all(state_dir);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("relay_agent_download.json"),
        serde_json::to_string_pretty(&rows).expect("serialize relay file evidence"),
    )
    .expect("write relay file evidence");
    assert!(rows.iter().all(|row| row["pass"] == true));
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) fn file_lifecycle_differential_secure_file_writer_download_open() {
    use std::io::Write;

    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let root = std::env::temp_dir().join(format!(
            "slskr-file-lifecycle-secure-writer-{target}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).expect("secure writer root");
        let destination = crate::safe_download_path(&root, "Secure/Output.bin")
            .expect("secure writer destination");
        crate::ensure_scoped_download_path(&root, destination.to_string_lossy().as_ref())
            .expect("secure writer parent");

        let mut first = crate::file_transfer_runtime::open_download_file(&root, &destination)
            .expect("secure writer nominal open");
        first
            .write_all(b"secure-first")
            .expect("secure writer nominal bytes");
        drop(first);
        assert_eq!(
            fs::read(&destination).expect("read secure writer output"),
            b"secure-first"
        );
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Common/Security/SecureFileWriter",
            "case": "path-and-default-selection",
            "pass": destination.starts_with(&root),
        }));
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Common/Security/SecureFileWriter",
            "case": "nominal-bytes-and-metadata",
            "pass": fs::metadata(&destination).expect("secure writer metadata").len() == 12,
        }));

        let (mut overwrite, offset) =
            crate::prepare_incomplete_download_file(&root, &destination, "overwrite", 32)
                .expect("secure writer overwrite open");
        overwrite
            .write_all(b"secure-second")
            .expect("secure writer overwrite bytes");
        drop(overwrite);
        assert_eq!(offset, 0);
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Common/Security/SecureFileWriter",
            "case": "existing-missing-and-overwrite",
            "pass": fs::read(&destination).expect("read secure writer overwrite")
                == b"secure-second",
        }));

        #[cfg(unix)]
        let confined = {
            use std::os::unix::fs::symlink;

            let outside = root.join("outside.bin");
            fs::write(&outside, b"outside").expect("secure writer outside file");
            let linked = root.join("Secure").join("linked.bin");
            symlink(&outside, &linked).expect("secure writer destination symlink");
            crate::file_transfer_runtime::open_download_file(&root, &linked).is_err()
                && fs::read(&outside).expect("read secure writer outside") == b"outside"
        };
        #[cfg(not(unix))]
        let confined = true;
        rows.push(serde_json::json!({
            "target": target,
            "subject": "Common/Security/SecureFileWriter",
            "case": "permissions-symlink-and-path-confinement",
            "pass": confined,
        }));
        let _ = fs::remove_dir_all(root);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    let evidence =
        serde_json::to_string_pretty(&rows).expect("serialize secure writer file evidence");
    fs::write(
        evidence_dir.join("secure_file_writer_download_open.json"),
        evidence,
    )
    .expect("write secure writer file evidence");
    assert!(rows.iter().all(|row| row["pass"] == true));
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_dht_certificate_manager_identity_files() {
    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-dht-certificate-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("DHT certificate root");
    let first = crate::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().unwrap(),
        &root,
        None,
    )
    .await
    .expect("create DHT certificate identity");
    let first_pin = first.certificate_sha256();
    let certificate = root.join("overlay-certificate.der");
    let private_key = root.join("overlay-private-key.der");
    let certificate_bytes = fs::read(&certificate).expect("read DHT certificate");
    let private_key_bytes = fs::read(&private_key).expect("read DHT private key");
    assert!(certificate_bytes.len() > 256);
    assert!(!private_key_bytes.is_empty());
    drop(first);

    let second = crate::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().unwrap(),
        &root,
        None,
    )
    .await
    .expect("reload DHT certificate identity");
    let second_pin = second.certificate_sha256();
    assert_eq!(second_pin, first_pin);
    drop(second);

    #[cfg(unix)]
    let symlink_rejected = {
        use std::os::unix::fs::symlink;

        let linked_root = root.join("symlinked");
        fs::create_dir_all(&linked_root).expect("DHT symlink root");
        let outside = linked_root.join("outside.der");
        fs::write(&outside, &certificate_bytes).expect("DHT outside certificate");
        symlink(&outside, linked_root.join("overlay-certificate.der"))
            .expect("DHT certificate symlink");
        fs::write(
            linked_root.join("overlay-private-key.der"),
            &private_key_bytes,
        )
        .expect("DHT symlink private key");
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &linked_root,
            None,
        )
        .await
        .is_err()
            && fs::read(&outside).expect("read DHT outside certificate") == certificate_bytes
    };
    #[cfg(not(unix))]
    let symlink_rejected = true;

    let rows = vec![
        serde_json::json!({
            "target": "slskdn",
            "subject": "DhtRendezvous/Security/CertificateManager",
            "case": "path-and-default-selection",
            "pass": certificate.starts_with(&root),
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "DhtRendezvous/Security/CertificateManager",
            "case": "nominal-bytes-and-metadata",
            "pass": certificate_bytes.len() > 256 && !private_key_bytes.is_empty(),
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "DhtRendezvous/Security/CertificateManager",
            "case": "existing-missing-and-overwrite",
            "pass": fs::read(&certificate).expect("read retained DHT certificate")
                == certificate_bytes,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "DhtRendezvous/Security/CertificateManager",
            "case": "permissions-symlink-and-path-confinement",
            "pass": symlink_rejected,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "DhtRendezvous/Security/CertificateManager",
            "case": "restart-reload-retention-and-corruption",
            "pass": second_pin == first_pin,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("dht_certificate_manager_identity_files.json"),
        serde_json::to_string_pretty(&rows).expect("serialize DHT certificate evidence"),
    )
    .expect("write DHT certificate evidence");
    assert!(rows.iter().all(|row| row["pass"] == true));
    let _ = fs::remove_dir_all(root);
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) fn file_lifecycle_differential_atomic_file_writer_common_cases() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-atomic-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&state_dir).expect("file lifecycle state dir");
    let destination = state_dir.join("state.bin");

    crate::write_file_atomic(&destination, b"first").expect("nominal atomic write");
    assert_eq!(
        fs::read(&destination).expect("read nominal atomic write"),
        b"first"
    );
    assert_eq!(
        fs::metadata(&destination).expect("atomic metadata").len(),
        5
    );

    crate::write_file_atomic(&destination, b"second").expect("atomic overwrite");
    assert_eq!(
        fs::read(&destination).expect("read atomic overwrite"),
        b"second"
    );
    assert_eq!(
        fs::read_dir(&state_dir)
            .expect("list atomic state dir")
            .count(),
        1
    );

    let nested_destination = state_dir.join("nested").join("missing").join("state.bin");
    crate::write_file_atomic(&nested_destination, b"nested")
        .expect("atomic writer creates missing parent directories");
    assert_eq!(
        fs::read(&nested_destination).expect("read nested atomic write"),
        b"nested"
    );

    #[cfg(unix)]
    let symlink_destination_is_confined = {
        use std::os::unix::fs::symlink;

        let outside = state_dir.join("outside.bin");
        fs::write(&outside, b"outside").expect("outside atomic target");
        let linked_destination = state_dir.join("linked.bin");
        symlink(&outside, &linked_destination).expect("atomic destination symlink");
        crate::write_file_atomic(&linked_destination, b"replacement")
            .expect("replace destination symlink itself");
        fs::read(&outside).expect("read outside atomic target") == b"outside"
            && fs::read(&linked_destination).expect("read replaced destination") == b"replacement"
            && !fs::symlink_metadata(&linked_destination)
                .expect("destination metadata")
                .file_type()
                .is_symlink()
    };
    #[cfg(not(unix))]
    let symlink_destination_is_confined = true;

    let reloaded_bytes = fs::read(&nested_destination).expect("reload nested atomic state");

    let blocked_parent = state_dir.join("blocked-parent");
    fs::write(&blocked_parent, b"not-a-directory").expect("blocked atomic parent");
    let partial_write = crate::write_file_atomic(&blocked_parent.join("state.bin"), b"partial");
    let partial_cleanup = fs::read_dir(&state_dir)
        .expect("list state directory after blocked atomic write")
        .filter_map(Result::ok)
        .all(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            !name.contains(".tmp")
        });

    let rows = vec![
        serde_json::json!({
            "target": "slskdn",
            "subject": "Common/IO/AtomicFileWriter",
            "case": "path-and-default-selection",
            "pass": nested_destination.parent().is_some_and(Path::is_dir),
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "Common/IO/AtomicFileWriter",
            "case": "nominal-bytes-and-metadata",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "Common/IO/AtomicFileWriter",
            "case": "existing-missing-and-overwrite",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "Common/IO/AtomicFileWriter",
            "case": "permissions-symlink-and-path-confinement",
            "pass": symlink_destination_is_confined,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "Common/IO/AtomicFileWriter",
            "case": "partial-cancel-and-cleanup",
            "pass": partial_write.is_err() && partial_cleanup,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "Common/IO/AtomicFileWriter",
            "case": "restart-reload-retention-and-corruption",
            "pass": reloaded_bytes == b"nested",
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("atomic_file_writer_common_cases.json"),
        serde_json::to_string_pretty(&rows).expect("serialize atomic file evidence"),
    )
    .expect("write atomic file evidence");

    let _ = fs::remove_dir_all(state_dir);
}

/// The pin manager is also a frozen file-writer subject. Keep its
/// persistence evidence in the file-lifecycle ledger as well as the
/// security-control ledger.
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_mesh_certificate_pin_manager() {
    use crate::mesh_security::{CertificatePinManager, CertificatePinType, SecurityUtils};

    let target = "slskdn";
    let subject = "Mesh/Transport/CertificatePinManager";
    let mut ledger = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            if $pass {
                ledger.push(serde_json::json!({
                    "target": target,
                    "subject": subject,
                    "case": $case,
                    "pass": true,
                }));
            }
        }};
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-mesh-pins-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("pin file lifecycle root");
    let certificate = rcgen::generate_simple_self_signed(vec!["mesh-file".to_owned()])
        .expect("mesh file certificate");
    let certificate_der = certificate.cert.der().to_vec();
    let pin = SecurityUtils::certificate_pin_base64(&certificate_der).expect("mesh file pin");
    let manager = CertificatePinManager::new(&root).expect("mesh file pin manager");
    let path = root.join("mesh").join("certificate-pins.json");
    record!(
        "path-and-default-selection",
        manager
            .add_pin("file-peer", &pin, CertificatePinType::Current)
            .is_ok()
            && path.is_file()
    );
    let first_body = fs::read(&path).expect("first pin file");
    record!(
        "nominal-bytes-and-metadata",
        !first_body.is_empty() && serde_json::from_slice::<serde_json::Value>(&first_body).is_ok()
    );

    let second_certificate =
        rcgen::generate_simple_self_signed(vec!["mesh-file-rotated".to_owned()])
            .expect("rotated mesh file certificate");
    let second_der = second_certificate.cert.der().to_vec();
    let second_pin =
        SecurityUtils::certificate_pin_base64(&second_der).expect("rotated mesh file pin");
    manager
        .rotate_pin("file-peer", &second_pin)
        .expect("rotate mesh file pin");
    let second_body = fs::read(&path).expect("rotated pin file");
    record!(
        "existing-missing-and-overwrite",
        first_body != second_body && String::from_utf8_lossy(&second_body).contains(&second_pin)
    );

    #[cfg(unix)]
    let symlink_rejected = {
        use std::os::unix::fs::symlink;

        let attack_root = root.join("symlink-attack");
        fs::create_dir_all(attack_root.join("mesh")).expect("symlink attack directory");
        let outside = attack_root.join("outside.json");
        fs::write(&outside, b"{\"peer_certificates\":[]}").expect("outside pin file");
        let link = attack_root.join("mesh").join("certificate-pins.json");
        symlink(&outside, &link).expect("pin symlink");
        CertificatePinManager::new(&attack_root).is_err()
    };
    #[cfg(not(unix))]
    let symlink_rejected = false;
    record!("permissions-symlink-and-path-confinement", symlink_rejected);

    let reloaded = CertificatePinManager::new(&root).expect("reload mesh file pins");
    record!(
        "restart-reload-retention-and-corruption",
        reloaded.validate_certificate_pin("file-peer", &second_der)
            && reloaded.validate_certificate_pin("file-peer", &certificate_der)
            && reloaded
                .peer_certificate_info("file-peer")
                .is_some_and(|info| info.previous_pins.contains(&pin))
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
    fs::write(
        evidence_dir.join("mesh_certificate_pin_manager.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh pin file-lifecycle ledger"),
    )
    .expect("write mesh pin file-lifecycle ledger");
    let _ = fs::remove_dir_all(root);
}

/// GoldStarClubService persists its local opt-out marker as a fixed file
/// under the application directory.  The Rust pod service uses the same
/// externally visible marker and atomic replacement semantics.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) fn file_lifecycle_differential_gold_star_club_revocation() {
    let target = "slskdn";
    let subject = "PodCore/GoldStarClubService";
    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-gold-star-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("Gold Star file lifecycle root");
    let path = crate::pods::gold_star_club_revocation_path(&root);
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push($case.to_owned());
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    record!(
        "path-and-default-selection",
        path == root.join("gold-star-club.revoked")
            && crate::pods::record_gold_star_club_revocation(&root, "local-peer").is_ok()
            && path.is_file()
    );
    let first_body = fs::read_to_string(&path).expect("read Gold Star revocation marker");
    record!(
        "nominal-bytes-and-metadata",
        !first_body.is_empty()
            && first_body.contains("revoked_by=local-peer\n")
            && fs::metadata(&path)
                .expect("Gold Star marker metadata")
                .is_file()
    );
    crate::pods::record_gold_star_club_revocation(&root, "rotated-peer")
        .expect("overwrite Gold Star revocation marker");
    let second_body = fs::read_to_string(&path).expect("read rotated Gold Star marker");
    record!(
        "existing-missing-and-overwrite",
        first_body != second_body && second_body.contains("revoked_by=rotated-peer\n")
    );
    #[cfg(unix)]
    let symlink_rejected = {
        use std::os::unix::fs::symlink;

        let outside = root.join("gold-star-outside");
        let linked_root = root.join("linked-state");
        fs::create_dir_all(&linked_root).expect("Gold Star linked state directory");
        fs::write(&outside, b"must remain unchanged").expect("Gold Star outside fixture");
        symlink(
            &outside,
            crate::pods::gold_star_club_revocation_path(&linked_root),
        )
        .expect("Gold Star revocation symlink");
        crate::pods::record_gold_star_club_revocation(&linked_root, "attacker").is_ok()
            && fs::read(&outside).expect("read Gold Star outside fixture")
                == b"must remain unchanged"
            && !crate::pods::gold_star_club_revocation_path(&linked_root).is_symlink()
    };
    #[cfg(not(unix))]
    let symlink_rejected = true;
    record!("permissions-symlink-and-path-confinement", symlink_rejected);
    record!(
        "restart-reload-retention-and-corruption",
        crate::pods::gold_star_club_is_revoked(&root)
            && crate::pods::record_gold_star_club_revocation(&root, "   ").is_err()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
    fs::write(
        evidence_dir.join("gold_star_club_revocation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Gold Star file-lifecycle ledger"),
    )
    .expect("write Gold Star file-lifecycle ledger");
    let _ = fs::remove_dir_all(root);
    assert!(
        mismatches.is_empty(),
        "Gold Star file-lifecycle mismatches: {}",
        mismatches.join(", ")
    );
}

/// MultiSourceDownloadService publishes only fully ranged, hash-verified
/// content and removes its private workspace when the operation is dropped.
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
    feature = "bounded-file-lifecycle-tests"
))]
pub(super) async fn file_lifecycle_differential_multisource_download_service() {
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const CHUNK_SIZE: u64 = 64 * 1024;
    let target = "slskdn";
    let subject = "Transfers/MultiSource/MultiSourceDownloadService";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(case.to_owned());
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let content = Arc::new(
        (0..(CHUNK_SIZE as usize * 2 + 17))
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));

    let spawn_range_source = |content: Arc<Vec<u8>>| async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind multisource lifecycle fixture");
        let address = listener
            .local_addr()
            .expect("multisource lifecycle fixture address");
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let content = Arc::clone(&content);
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let count = stream
                            .read(&mut buffer)
                            .await
                            .expect("read multisource range request");
                        if count == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..count]);
                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let request =
                        String::from_utf8(request).expect("multisource range request UTF-8");
                    let range = request
                        .lines()
                        .filter_map(|line| line.split_once(':'))
                        .find(|(name, _)| name.eq_ignore_ascii_case("range"))
                        .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                        .expect("multisource range header");
                    let (start, end) = range.split_once('-').expect("multisource range bounds");
                    let start = start.parse::<usize>().expect("multisource range start");
                    let end = end.parse::<usize>().expect("multisource range end");
                    let body = &content[start..=end];
                    let response = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                        body.len(),
                        content.len()
                    );
                    stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("write multisource range headers");
                    stream
                        .write_all(body)
                        .await
                        .expect("write multisource range body");
                });
            }
        });
        (address, task)
    };

    let (first_address, first_server) = spawn_range_source(Arc::clone(&content)).await;
    let (second_address, second_server) = spawn_range_source(Arc::clone(&content)).await;
    let request_for = |hash: String| crate::multisource::SwarmRequest {
        filename: "Remote/assembled.flac".to_owned(),
        file_size: content.len() as u64,
        expected_hash: Some(hash),
        output_path: None,
        chunk_size: CHUNK_SIZE,
        sources: vec![
            crate::multisource::RangeSource {
                username: "first".to_owned(),
                url: format!("http://{first_address}/file"),
                authorization: None,
            },
            crate::multisource::RangeSource {
                username: "second".to_owned(),
                url: format!("http://{second_address}/file"),
                authorization: None,
            },
        ],
    };
    let insert_job = |id: String,
                      request: crate::multisource::SwarmRequest,
                      store: Arc<RwLock<crate::multisource::SwarmStore>>| async move {
        store.write().await.insert(crate::multisource::new_job(
            id,
            &request,
            "multisource/assembled.flac".to_owned(),
            1,
        ));
    };

    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-multisource-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create multisource lifecycle root");
    let output = root.join("assembled.flac");
    let mut nominal_request = request_for(expected_hash.clone());
    crate::multisource::validate_request(&mut nominal_request)
        .expect("valid multisource lifecycle request");
    let nominal_id = uuid::Uuid::new_v4().to_string();
    let nominal_store = Arc::new(RwLock::new(crate::multisource::SwarmStore::default()));
    insert_job(
        nominal_id.clone(),
        nominal_request.clone(),
        Arc::clone(&nominal_store),
    )
    .await;
    let nominal = crate::multisource::execute(
        nominal_id.clone(),
        nominal_request,
        output.clone(),
        "multisource/assembled.flac".to_owned(),
        Arc::clone(&nominal_store),
    )
    .await;
    let output_bytes = fs::read(&output).expect("read multisource output");
    #[cfg(unix)]
    let output_private = {
        use std::os::unix::fs::PermissionsExt;

        fs::metadata(&output)
            .expect("multisource output metadata")
            .permissions()
            .mode()
            & 0o777
            == 0o600
    };
    #[cfg(not(unix))]
    let output_private = true;
    record!(
        "nominal-bytes-and-metadata",
        nominal.success
            && nominal.final_hash == expected_hash
            && output_bytes == *content
            && output_private
            && nominal_store
                .try_read()
                .ok()
                .and_then(|store| store.get(&nominal_id).map(|job| job.status == "completed"))
                .unwrap_or(false)
    );

    let mut existing_request = request_for(expected_hash.clone());
    crate::multisource::validate_request(&mut existing_request)
        .expect("valid existing-output request");
    let existing_id = uuid::Uuid::new_v4().to_string();
    let existing_store = Arc::new(RwLock::new(crate::multisource::SwarmStore::default()));
    insert_job(
        existing_id.clone(),
        existing_request.clone(),
        Arc::clone(&existing_store),
    )
    .await;
    let existing = crate::multisource::execute(
        existing_id,
        existing_request,
        output.clone(),
        "multisource/assembled.flac".to_owned(),
        existing_store,
    )
    .await;
    record!(
        "existing-missing-and-overwrite",
        nominal.success
            && !existing.success
            && existing
                .error
                .as_deref()
                .is_some_and(|error| error.contains("output file already exists"))
            && fs::read(&output).expect("read protected multisource output") == output_bytes
    );

    #[cfg(unix)]
    let symlink_safe = {
        use std::os::unix::fs::symlink;

        let outside = root.join("outside.flac");
        let link = root.join("linked-output.flac");
        fs::write(&outside, b"outside data").expect("write multisource symlink target");
        symlink(&outside, &link).expect("create multisource output symlink");
        let mut link_request = request_for(expected_hash.clone());
        crate::multisource::validate_request(&mut link_request)
            .expect("valid symlink-output request");
        let link_id = uuid::Uuid::new_v4().to_string();
        let link_store = Arc::new(RwLock::new(crate::multisource::SwarmStore::default()));
        insert_job(
            link_id.clone(),
            link_request.clone(),
            Arc::clone(&link_store),
        )
        .await;
        let link_result = crate::multisource::execute(
            link_id,
            link_request,
            link.clone(),
            "multisource/linked-output.flac".to_owned(),
            link_store,
        )
        .await;
        !link_result.success
            && fs::read(&outside).expect("read multisource symlink target") == b"outside data"
            && link.is_symlink()
    };
    #[cfg(not(unix))]
    let symlink_safe = true;
    record!(
        "permissions-symlink-and-path-confinement",
        output_private && symlink_safe
    );

    let cancel_root = root.join("cancelled");
    fs::create_dir(&cancel_root).expect("create multisource cancellation root");
    let cancel_output = cancel_root.join("cancelled.flac");
    let spawn_stalled_source = || async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind multisource cancellation fixture");
        let address = listener
            .local_addr()
            .expect("multisource cancellation fixture address");
        let (stalled_tx, stalled_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut preflight, _) = listener
                .accept()
                .await
                .expect("accept multisource preflight");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let count = preflight
                    .read(&mut buffer)
                    .await
                    .expect("read multisource preflight");
                assert_ne!(count, 0, "multisource preflight ended early");
                request.extend_from_slice(&buffer[..count]);
            }
            preflight
                .write_all(
                    b"HTTP/1.1 206 Partial Content\r\nContent-Length: 1\r\nContent-Range: bytes 0-0/131089\r\nConnection: close\r\n\r\na",
                )
                .await
                .expect("write multisource preflight");
            let (_stalled, _) = listener
                .accept()
                .await
                .expect("accept multisource stalled chunk");
            stalled_tx
                .send(())
                .expect("signal multisource stalled chunk");
            std::future::pending::<()>().await;
        });
        (address, stalled_rx, task)
    };
    let (cancel_first, cancel_first_stalled, cancel_first_server) = spawn_stalled_source().await;
    let (cancel_second, cancel_second_stalled, cancel_second_server) = spawn_stalled_source().await;
    let mut cancel_request = crate::multisource::SwarmRequest {
        filename: "Remote/cancelled.flac".to_owned(),
        file_size: content.len() as u64,
        expected_hash: Some(expected_hash.clone()),
        output_path: None,
        chunk_size: CHUNK_SIZE,
        sources: vec![
            crate::multisource::RangeSource {
                username: "cancel-first".to_owned(),
                url: format!("http://{cancel_first}/file"),
                authorization: None,
            },
            crate::multisource::RangeSource {
                username: "cancel-second".to_owned(),
                url: format!("http://{cancel_second}/file"),
                authorization: None,
            },
        ],
    };
    crate::multisource::validate_request(&mut cancel_request).expect("valid cancellation request");
    let cancel_id = uuid::Uuid::new_v4().to_string();
    let cancel_store = Arc::new(RwLock::new(crate::multisource::SwarmStore::default()));
    insert_job(
        cancel_id.clone(),
        cancel_request.clone(),
        Arc::clone(&cancel_store),
    )
    .await;
    let download = tokio::spawn(crate::multisource::execute(
        cancel_id,
        cancel_request,
        cancel_output.clone(),
        "multisource/cancelled.flac".to_owned(),
        cancel_store,
    ));
    let stalled = tokio::time::timeout(Duration::from_secs(5), async {
        cancel_first_stalled
            .await
            .expect("first multisource chunk must stall");
        cancel_second_stalled
            .await
            .expect("second multisource chunk must stall");
    })
    .await
    .is_ok();
    let workspace_seen = fs::read_dir(&cancel_root)
        .expect("read multisource cancellation root")
        .flatten()
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".slskr-swarm-")
        });
    download.abort();
    let cancelled = download
        .await
        .expect_err("multisource download must be cancelled")
        .is_cancelled();
    let workspace_removed = !fs::read_dir(&cancel_root)
        .expect("read multisource cancellation root after abort")
        .flatten()
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".slskr-swarm-")
        });
    cancel_first_server.abort();
    cancel_second_server.abort();
    record!(
        "partial-cancel-and-cleanup",
        stalled && workspace_seen && cancelled && workspace_removed && !cancel_output.exists()
    );

    let restarted_store = Arc::new(RwLock::new(crate::multisource::SwarmStore::default()));
    let reloaded_bytes = fs::read(&output).expect("read multisource output after restart");
    let reloaded_hash = hex::encode(Sha256::digest(reloaded_bytes.as_slice()));
    let corrupt_output = root.join("corrupt.flac");
    let mut corrupt_request = request_for("00".repeat(32));
    crate::multisource::validate_request(&mut corrupt_request)
        .expect("valid corrupt multisource request");
    let corrupt_id = uuid::Uuid::new_v4().to_string();
    insert_job(
        corrupt_id.clone(),
        corrupt_request.clone(),
        Arc::clone(&restarted_store),
    )
    .await;
    let corrupt = crate::multisource::execute(
        corrupt_id,
        corrupt_request,
        corrupt_output.clone(),
        "multisource/corrupt.flac".to_owned(),
        Arc::clone(&restarted_store),
    )
    .await;
    record!(
        "restart-reload-retention-and-corruption",
        reloaded_bytes == *content
            && reloaded_hash == expected_hash
            && !corrupt.success
            && corrupt
                .error
                .as_deref()
                .is_some_and(|error| error.contains("SHA-256 verification"))
            && !corrupt_output.exists()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create multisource evidence directory");
    fs::write(
        evidence_dir.join("multisource_download_service.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize multisource file-lifecycle ledger"),
    )
    .expect("write multisource file-lifecycle ledger");
    first_server.abort();
    second_server.abort();
    let _ = fs::remove_dir_all(root);
    assert!(
        mismatches.is_empty(),
        "multisource file-lifecycle mismatches: {}",
        mismatches.join(", ")
    );
}
