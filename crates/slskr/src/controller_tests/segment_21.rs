#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_instance_name_uses_frozen_string_binding_and_restart_lifecycle() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let cli_environment = BTreeMap::from([(
        "SLSKR_STATE_DIR".to_owned(),
        state.config.state_dir.display().to_string(),
    )]);

    let numeric_yaml = "instance_name: 123\nflags:\n  no_connect: true\n";
    fs::write(state.config.state_dir.join("slskd.yml"), numeric_yaml).unwrap();
    super::apply_watched_controller_configuration(&state, Some(numeric_yaml), &cli_environment)
        .await;

    {
        let overlay = state.options_overlay.read().await;
        let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
            &state.config,
            &overlay,
            true,
        ))
        .unwrap();
        let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
            &state.config,
            &overlay,
            false,
        ))
        .unwrap();
        assert_eq!(current["instanceName"], "123");
        assert_eq!(startup["instanceName"], "default");
    }
    assert!(state.runtime.read().await.application_restart_requested);

    let empty_yaml = "instance_name: \"\"\nflags:\n  no_connect: true\n";
    fs::write(state.config.state_dir.join("slskd.yml"), empty_yaml).unwrap();
    super::apply_watched_controller_configuration(&state, Some(empty_yaml), &cli_environment).await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    assert_eq!(current["instanceName"], "default");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_completed_path_template_updates_projection_and_runtime_without_restart() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_CONTROLLER_PROFILE".to_owned(), "native".to_owned()),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let yaml =
        "transfers:\n  download:\n    completed_path_template: '{uploader}/{remote_folder}'\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    assert_eq!(
        super::effective_download_completed_path_template(&state),
        "{uploader}/{remote_folder}"
    );
    assert_eq!(
        super::file_transfer_runtime::render_completed_download_path(
            &super::effective_download_completed_path_template(&state),
            "friend",
            "Albums/Record/Song.flac",
            None,
            None,
            0,
        )
        .unwrap(),
        "friend/Albums/Record/Song.flac"
    );
    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(
        current["global"]["download"]["completedPathTemplate"],
        "{uploader}/{remote_folder}"
    );
    assert_eq!(startup["global"]["download"]["completedPathTemplate"], "");
    let debug = super::controller_options_debug_view(&state, &overlay);
    assert!(debug.contains(
        "completedpathtemplate={uploader}/{remote_folder} (YamlConfigurationProvider for 'slskd.yml' (Optional))"
    ));
    assert!(!state.runtime.read().await.application_restart_requested);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_private_message_auto_response_updates_actual_outbound_behavior() {
    use slskr_client::{
        protocol::server::{Direction, PrivateMessage, ServerMessage},
        server::ServerSession,
        stream::ServerConnection,
    };

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_CONTROLLER_PROFILE".to_owned(), "native".to_owned()),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let yaml = "soulseek:\n  private_message_auto_response:\n    enabled: true\n    message: Watched human response\n    cooldown_minutes: 15\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    super::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    let settings = state
        .private_message_auto_response_settings
        .read()
        .await
        .clone();
    assert!(settings.enabled);
    assert_eq!(settings.message, "Watched human response");
    assert_eq!(settings.cooldown_minutes, 15);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(address);
    let server = listener.accept();
    let (client, server) = tokio::join!(client, server);
    let (server, _) = server.unwrap();
    let mut session = ServerSession::new(ServerConnection::new(server));
    let mut fixture = ServerConnection::new(client.unwrap());

    crate::session_runtime::project_server_message(
        &state,
        &mut session,
        &ServerMessage::MessageUserResponse(PrivateMessage {
            id: 41,
            timestamp: 1,
            username: "FixturePeer".to_owned(),
            message: "Please prove you are human".to_owned(),
            is_new: true,
            was_replayed: false,
        }),
    )
    .await;

    assert_eq!(
        fixture
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::MessageUserRequest {
            username: "FixturePeer".to_owned(),
            message: "Watched human response".to_owned(),
        }
    );
    assert!(!state.runtime.read().await.application_restart_requested);

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(
        current["soulseek"]["privateMessageAutoResponse"],
        serde_json::json!({
            "enabled": true,
            "message": "Watched human response",
            "cooldownMinutes": 15,
        })
    );
    assert_ne!(
        startup["soulseek"]["privateMessageAutoResponse"],
        current["soulseek"]["privateMessageAutoResponse"]
    );
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
async fn file_lifecycle_differential_options_controller_rejects_backup_symlink() {
    use std::os::unix::fs::symlink;

    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let first = "soulseek:\n  description: first\n";
        let response = super::route_http_request(
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

        let response = super::route_http_request(
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
async fn file_lifecycle_differential_files_service_roots_and_metadata() {
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
            let response = super::route_http_request(
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
        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/v0/files/downloads/files/{encoded_managed}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete managed FileService file");
        let missing = super::route_http_request(
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
            let response = super::route_http_request(
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
async fn file_lifecycle_differential_download_service_path_and_retry() {
    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
        let rendered = super::render_configured_completed_download_path(
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

        let incomplete_root = super::effective_incomplete_dir(&state);
        let incomplete =
            super::safe_download_path(&incomplete_root, "FileService/friend/Song.flac")
                .expect("incomplete path");
        let incomplete = super::ensure_scoped_download_path(
            &incomplete_root,
            incomplete.to_string_lossy().as_ref(),
        )
        .expect("confined incomplete path");
        fs::write(&incomplete, b"abc").expect("existing partial download");

        let (mut resumed, resume_offset) =
            super::prepare_incomplete_download_file(&incomplete_root, &incomplete, "resume", 4)
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
            super::prepare_incomplete_download_file(&incomplete_root, &incomplete, "overwrite", 4)
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
async fn file_lifecycle_differential_relay_agent_download_cleanup_and_reload() {
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
        let root = super::effective_downloads_dir(&state);
        let filename = "Relay/Agent.flac";
        let destination =
            super::safe_download_path(&root, filename).expect("relay destination path");
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
        super::relay_agent::download_completed_file(
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
        super::relay_agent::download_completed_file(
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
        let confinement = super::relay_agent::download_completed_file(
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
        let partial_result = super::relay_agent::download_completed_file(
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
fn file_lifecycle_differential_secure_file_writer_download_open() {
    use std::io::Write;

    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let root = std::env::temp_dir().join(format!(
            "slskr-file-lifecycle-secure-writer-{target}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).expect("secure writer root");
        let destination = super::safe_download_path(&root, "Secure/Output.bin")
            .expect("secure writer destination");
        super::ensure_scoped_download_path(&root, destination.to_string_lossy().as_ref())
            .expect("secure writer parent");

        let mut first = super::file_transfer_runtime::open_download_file(&root, &destination)
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
            super::prepare_incomplete_download_file(&root, &destination, "overwrite", 32)
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
            super::file_transfer_runtime::open_download_file(&root, &linked).is_err()
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
async fn file_lifecycle_differential_dht_certificate_manager_identity_files() {
    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-dht-certificate-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("DHT certificate root");
    let first = super::private_gateway::Gateway::load_or_create_with_quic(
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

    let second = super::private_gateway::Gateway::load_or_create_with_quic(
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
        super::private_gateway::Gateway::load_or_create_with_quic(
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn options_debug_requires_both_debug_and_remote_configuration() {
    for (remote, debug, expected) in [
        (false, false, "403 Forbidden"),
        (true, false, "403 Forbidden"),
        (true, true, "200 OK"),
    ] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with(
                    "SLSKR_REMOTE_CONFIGURATION",
                    if remote { "true" } else { "false" },
                )
                .with("SLSKR_DEBUG", if debug { "true" } else { "false" }),
        );
        let response = super::route_http_request("GET", "/api/v0/options/debug", None, "", &state)
            .await
            .expect("options debug response");
        assert_eq!(response.status, expected, "remote={remote} debug={debug}");
        if expected == "200 OK" {
            let debug_view = serde_json::from_str::<String>(&response.body).unwrap();
            assert!(debug_view.starts_with("slskd:\n"));
            assert!(debug_view.contains("  debug=True (DefaultValueConfigurationProvider)"));
            assert!(debug_view.contains("urls=http://"));
            assert!(!debug_view.contains("api_token"));
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_debug_view_projects_frozen_default_authentication_values() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_CONFIGURATION", "true")
            .with("SLSKR_DEBUG", "true"),
    );
    let overlay = state.options_overlay.read().await;
    let debug = super::controller_options_debug_view(&state, &overlay);

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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_adversarial_yaml_updates_replace_the_existing_mapping() {
    let initial = "debug: true\nremote_configuration: true\n";
    let defaults = super::native_adversarial_yaml_update(initial, &serde_json::json!({})).unwrap();
    let custom = serde_json::json!({
        "enabled": true,
        "profile": "custom",
        "privacy": {
            "enabled": true,
            "padding": {
                "enabled": true,
                "bucketSizes": [256, 512]
            }
        }
    });
    let updated = super::native_adversarial_yaml_update(&defaults, &custom).unwrap();
    let direct = super::native_adversarial_yaml_update(initial, &custom).unwrap();

    assert_eq!(updated, direct);
    assert_eq!(updated.matches("\nsecurity:\n").count(), 1);
    assert!(updated.contains("    enabled: true\n    profile: Custom\n"));
    assert!(updated.contains(
        "        bucket_sizes:\n        - 256\n        - 512\n        use_random_fill: true\n"
    ));
    assert!(!updated.contains("        - 16384\n"));
    assert!(updated.ends_with("...\n"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_adversarial_yaml_update_preserves_security_siblings() {
    let current = "remote_configuration: true\nsecurity:\n  audit_enabled: true\n";
    let updated = super::native_adversarial_yaml_update(
        current,
        &serde_json::json!({"transport": {"webSocket": {"enabled": true}}}),
    )
    .unwrap();

    assert!(updated.starts_with(
        "remote_configuration: true\nsecurity:\n  audit_enabled: true\n  adversarial:\n"
    ));
    assert!(updated.contains("      web_socket:\n        enabled: true\n"));
    assert_eq!(updated.matches("  adversarial:\n").count(), 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_adversarial_validation_rejects_nonpositive_bucket_sizes() {
    assert_eq!(
        super::validate_native_adversarial_settings(&serde_json::json!({
            "privacy": {"padding": {"bucketSizes": [256, 0]}}
        })),
        Err("Bucket sizes must be positive")
    );
    assert!(
        super::validate_native_adversarial_settings(&serde_json::json!({
            "privacy": {"padding": {"bucketSizes": [256, 512]}}
        }))
        .is_ok()
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn native_adversarial_put_persists_and_accepts_target_yaml() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_REMOTE_CONFIGURATION", "true"),
    );
    fs::write(
        state.config.state_dir.join("slskd.yml"),
        "debug: true\nremote_configuration: true\n",
    )
    .unwrap();

    let response =
        super::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
            .await
            .expect("adversarial settings response");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    assert!(fs::read_to_string(state.config.state_dir.join("slskd.yml"))
        .unwrap()
        .contains("  adversarial:\n"));
    super::load_watched_controller_configuration(state.controller_cli_environment.clone())
        .expect("target adversarial YAML remains reloadable");
    let features = state.controller_features.read().await;
    let stored = features
        .get("security/profile/security/adversarial")
        .expect("stored adversarial settings");
    assert_eq!(stored["settings"], serde_json::json!({}));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn remote_options_overlay_and_yaml_are_effective_and_durable() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_CONFIGURATION", "true"));

    let options = super::route_http_request("GET", "/api/options", None, "", &state)
        .await
        .expect("options get");
    assert_eq!(options.status, "200 OK");
    let options_json = serde_json::from_str::<serde_json::Value>(&options.body).unwrap();
    assert_eq!(
        options_json["remoteConfiguration"], true,
        "{}",
        options.body
    );
    assert!(options_json.get("version").is_none());
    assert!(options_json.get("options").is_none());
    assert!(options_json.get("compatibility").is_none());
    assert!(options_json.get("nonPersistentMutationRoutes").is_none());

    let response = super::route_http_request(
        "PATCH",
        "/api/options",
        None,
        r#"{"soulseek":{"listenIpAddress":"127.0.0.1","listenPort":50300,"privateMessageAutoResponse":{"enabled":true}}}"#,
        &state,
    )
    .await
    .expect("options response");

    assert_eq!(response.status, "200 OK");
    let overlay = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(overlay["soulseek"]["listenIpAddress"], "127.0.0.1");
    assert_eq!(overlay["soulseek"]["listenPort"], 50300);
    assert_eq!(
        overlay["soulseek"]["privateMessageAutoResponse"]["enabled"],
        true
    );
    assert!(
        state
            .private_message_auto_response_settings
            .read()
            .await
            .enabled
    );

    let invalid = super::route_http_request("PATCH", "/api/options", None, "[]", &state)
        .await
        .expect("invalid options response");
    assert_eq!(invalid.status, "400 Bad Request");

    let location = super::route_http_request("GET", "/api/options/yaml/location", None, "", &state)
        .await
        .expect("options location");
    assert_eq!(location.status, "200 OK");
    let location_path = serde_json::from_str::<String>(&location.body).unwrap();
    assert_eq!(
        PathBuf::from(location_path),
        state.config.state_dir.join("slskd.yml")
    );

    let config_text = super::route_http_request("GET", "/api/options/yaml", None, "", &state)
        .await
        .expect("options config text");
    assert_eq!(config_text.status, "404 Not Found");

    let yaml = "soulseek:\n  description: test description\nweb:\n  authentication:\n    password: never-return-this\n";
    let uploaded = super::route_http_request(
        "PUT",
        "/api/options/yaml",
        None,
        &serde_json::to_string(yaml).unwrap(),
        &state,
    )
    .await
    .expect("options upload");
    assert_eq!(uploaded.status, "200 OK");
    assert!(uploaded.body.is_empty());
    assert_eq!(
        fs::read_to_string(state.config.state_dir.join("slskd.yml")).unwrap(),
        yaml
    );

    let config_text = super::route_http_request("GET", "/api/options/yaml", None, "", &state)
        .await
        .expect("options config text");
    assert_eq!(config_text.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<String>(&config_text.body).unwrap(),
        yaml
    );

    let effective = super::route_http_request("GET", "/api/options", None, "", &state)
        .await
        .expect("effective options");
    let effective = serde_json::from_str::<serde_json::Value>(&effective.body).unwrap();
    assert_eq!(effective["soulseek"]["description"], "test description");
    assert_eq!(effective["web"]["authentication"]["password"], "*****");
    assert_eq!(effective["soulseek"]["listenPort"], 50300);

    let validated = super::route_http_request(
        "POST",
        "/api/options/yaml/validate",
        None,
        &serde_json::to_string("app: {}\n").unwrap(),
        &state,
    )
    .await
    .expect("valid options validate");
    assert_eq!(validated.status, "200 OK");
    assert!(validated.content_type.is_empty());
    assert!(validated.body.is_empty());

    let invalid_upload = super::route_http_request("PUT", "/api/options/yaml", None, "{}", &state)
        .await
        .expect("invalid options upload");
    assert_eq!(invalid_upload.status, "400 Bad Request");

    let invalid_validate =
        super::route_http_request("POST", "/api/options/yaml/validate", None, "{}", &state)
            .await
            .expect("invalid options validate");
    assert_eq!(invalid_validate.status, "400 Bad Request");

    let invalid_yaml = super::route_http_request(
        "POST",
        "/api/options/yaml/validate",
        None,
        &serde_json::to_string("web: [unterminated").unwrap(),
        &state,
    )
    .await
    .expect("invalid YAML validation");
    assert_eq!(invalid_yaml.status, "200 OK");
    assert_eq!(invalid_yaml.body, r#""Invalid YAML configuration""#);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn options_config_location_is_the_confined_compatibility_file() {
    let (state, _receiver) = test_state();
    let mut config = state.config.clone();
    config.config_file = Some("/private/config/slskr-secret.toml".into());

    let location =
        serde_json::from_str::<String>(&super::controller_options_config_location_json(&config))
            .unwrap();
    assert_eq!(PathBuf::from(location), config.state_dir.join("slskd.yml"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_join_enforce_mode_verifies_ed25519_and_rejects_replay() {
    use ed25519_dalek::{Signer, SigningKey};

    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", "enforce"),
        super::SearchStore::new(),
        None,
    );
    state
        .rooms
        .write()
        .await
        .join("pod:ambient".to_owned())
        .expect("test pod");
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    let timestamp = super::unix_timestamp().saturating_mul(1_000);
    let payload = serde_json::to_string(&(
        1,
        "join-request",
        "pod:ambient",
        "peer-one",
        "member",
        timestamp,
        "Please add me",
        "nonce-one",
    ))
    .unwrap();
    let signature = base64::engine::general_purpose::STANDARD
        .encode(signing_key.sign(payload.as_bytes()).to_bytes());
    let public_key =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().to_bytes());
    let body = serde_json::json!({
        "podId": "pod:ambient",
        "peerId": "peer-one",
        "requestedRole": "member",
        "timestampUnixMs": timestamp,
        "message": "Please add me",
        "nonce": "nonce-one",
        "signature": format!("ed25519:{signature}"),
        "publicKey": public_key,
    })
    .to_string();

    let joined =
        super::route_http_request("POST", "/api/podcore/membership/join", None, &body, &state)
            .await
            .expect("verified pod join");
    assert_eq!(joined.status, "200 OK", "{}", joined.body);
    let joined_json = serde_json::from_str::<serde_json::Value>(&joined.body).unwrap();
    assert_eq!(joined_json["signatureMode"], "enforce");
    assert_eq!(joined_json["signatureVerified"], true);
    assert_eq!(joined_json["success"], true);
    assert_eq!(joined_json["joinRequest"]["peerId"], "peer-one");
    assert_eq!(state.rooms.read().await.records.len(), 1);
    assert!(state.rooms.read().await.records[0].members.is_empty());

    let replay =
        super::route_http_request("POST", "/api/podcore/membership/join", None, &body, &state)
            .await
            .expect("replayed pod join");
    assert_eq!(replay.status, "400 Bad Request");
    assert!(replay.body.contains("nonce has already been used"));
    assert_eq!(state.rooms.read().await.records.len(), 1);

    let mut tampered = serde_json::from_str::<serde_json::Value>(&body).unwrap();
    tampered["podId"] = serde_json::json!("pod:tampered");
    let rejected = super::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        &tampered.to_string(),
        &state,
    )
    .await
    .expect("tampered pod join");
    assert_eq!(rejected.status, "400 Bad Request");
    assert!(rejected.body.contains("signature is invalid"));
    assert_eq!(state.rooms.read().await.records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_join_warn_mode_accepts_legacy_but_rejects_invalid_ed25519() {
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", "warn"),
        super::SearchStore::new(),
        None,
    );
    {
        let mut rooms = state.rooms.write().await;
        rooms.join("legacy-pod".to_owned()).expect("legacy pod");
        rooms.join("invalid-pod".to_owned()).expect("invalid pod");
    }
    let legacy = super::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        r#"{"podId":"legacy-pod","peerId":"legacy-peer"}"#,
        &state,
    )
    .await
    .expect("legacy pod join");
    assert_eq!(legacy.status, "200 OK", "{}", legacy.body);
    let legacy_json = serde_json::from_str::<serde_json::Value>(&legacy.body).unwrap();
    assert_eq!(legacy_json["signatureMode"], "warn");
    assert_eq!(legacy_json["signatureVerified"], false);

    let invalid = super::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        &serde_json::json!({
            "podId": "invalid-pod",
            "peerId": "peer",
            "timestampUnixMs": super::unix_timestamp().saturating_mul(1_000),
            "signature": format!("ed25519:{}", base64::engine::general_purpose::STANDARD.encode([0_u8; 64])),
            "publicKey": base64::engine::general_purpose::STANDARD.encode([0_u8; 32]),
        }).to_string(),
        &state,
    )
    .await
    .expect("invalid signed pod join");
    assert_eq!(invalid.status, "400 Bad Request");
    assert_eq!(state.rooms.read().await.records.len(), 2);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_membership_workflow_queues_accepts_lists_leaves_and_cancels() {
    let (state, _receiver) = test_state();
    {
        let mut rooms = state.rooms.write().await;
        rooms.join("pod:workflow".to_owned()).expect("workflow pod");
        rooms
            .add_member("pod:workflow", "owner-peer".to_owned())
            .expect("owner capacity")
            .expect("workflow pod");
        rooms
            .add_member("pod:workflow", "ordinary-peer".to_owned())
            .expect("ordinary member capacity")
            .expect("workflow pod");
        rooms.records[0].operated = true;
    }
    state.pod_membership_workflow.write().await.set_role(
        "pod:workflow",
        "owner-peer",
        "owner".to_owned(),
    );

    let join = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","requestedRole":"moderator"}"#,
        &state,
    )
    .await
    .expect("join request");
    assert_eq!(join.status, "200 OK");
    assert!(!state.rooms.read().await.records[0]
        .members
        .iter()
        .any(|member| member == "applicant"));

    let pending = super::route_http_request(
        "GET",
        "/api/v0/podcore/membership/join/pending/pod%3Aworkflow",
        None,
        "",
        &state,
    )
    .await
    .expect("pending joins");
    assert_eq!(pending.status, "200 OK");
    let pending_json = serde_json::from_str::<serde_json::Value>(&pending.body).unwrap();
    assert_eq!(
        pending_json["pendingJoinRequests"][0]["peerId"],
        "applicant"
    );

    let unauthorized = super::route_http_request(
        "POST",
        "/api/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","acceptedRole":"moderator","acceptorPeerId":"ordinary-peer"}"#,
        &state,
    )
    .await
    .expect("unauthorized join acceptance");
    assert_eq!(unauthorized.status, "400 Bad Request");
    assert!(unauthorized.body.contains("does not have permission"));
    assert_eq!(
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_joins("pod:workflow")
            .len(),
        1
    );

    let accepted = super::route_http_request(
        "POST",
        "/api/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","acceptedRole":"moderator","acceptorPeerId":"owner-peer"}"#,
        &state,
    )
    .await
    .expect("join acceptance");
    assert_eq!(accepted.status, "200 OK");
    assert!(state.rooms.read().await.records[0]
        .members
        .iter()
        .any(|member| member == "applicant"));

    let leave = super::route_http_request(
        "POST",
        "/api/podcore/membership/leave",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant"}"#,
        &state,
    )
    .await
    .expect("leave request");
    assert_eq!(leave.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&leave.body).unwrap()["pending"],
        true
    );
    let pending_leave = super::route_http_request(
        "GET",
        "/api/podcore/membership/leave/pending/pod%3Aworkflow",
        None,
        "",
        &state,
    )
    .await
    .expect("pending leaves");
    assert!(pending_leave.body.contains("applicant"));

    let accepted_leave = super::route_http_request(
        "POST",
        "/api/podcore/membership/leave/accept",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","acceptorPeerId":"owner-peer"}"#,
        &state,
    )
    .await
    .expect("leave acceptance");
    assert_eq!(accepted_leave.status, "200 OK");
    assert!(!state.rooms.read().await.records[0]
        .members
        .iter()
        .any(|member| member == "applicant"));

    let second_join = super::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow","peerId":"peer-two"}"#,
        &state,
    )
    .await
    .expect("second join request");
    assert_eq!(second_join.status, "200 OK");
    let cancelled = super::route_http_request(
        "DELETE",
        "/api/v0/podcore/membership/join/pod%3Aworkflow/peer-two",
        None,
        "",
        &state,
    )
    .await
    .expect("cancel join");
    assert_eq!(cancelled.status, "200 OK");
    assert_eq!(cancelled.body, r#"{"cancelled":true}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn compatibility_noop_routes_advertise_supported_shape() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));

    let logs = super::route_http_request("GET", "/api/logs", None, "", &state)
        .await
        .expect("logs");
    assert_eq!(logs.status, "200 OK");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap();
    assert_eq!(logs_json["entries"].as_array().unwrap().len(), 0);
    assert_eq!(logs_json["level"], "Information");
    let compat_logs = super::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("compat logs");
    let compat_logs_json = serde_json::from_str::<serde_json::Value>(&compat_logs.body).unwrap();
    assert_eq!(compat_logs_json.as_array().unwrap().len(), 0);

    let bridge =
        super::route_http_request("PUT", "/api/v0/bridge/admin/config", None, "{}", &state)
            .await
            .expect("bridge config");
    assert_eq!(bridge.status, "200 OK");
    let bridge_json = serde_json::from_str::<serde_json::Value>(&bridge.body).unwrap();
    assert_eq!(bridge_json["persisted"], true);
    assert_eq!(bridge_json["restart_required"], true);
    assert_eq!(bridge_json["configUpdates"], 1);

    let username_ban = super::route_http_request(
        "POST",
        "/api/bans/username",
        None,
        r#"{"username":"peer1"}"#,
        &state,
    )
    .await
    .expect("username ban");
    let username_ban_json = serde_json::from_str::<serde_json::Value>(&username_ban.body).unwrap();
    assert_eq!(username_ban_json["banned"], true);
    assert_eq!(username_ban_json["persisted"], false);
    assert_eq!(username_ban_json["activeBans"], 1);
    let bans = super::route_http_request("GET", "/api/bans", None, "", &state)
        .await
        .expect("bans");
    let bans_json = serde_json::from_str::<serde_json::Value>(&bans.body).unwrap();
    assert_eq!(bans_json["count"], 1);
    assert_eq!(bans_json["bans"][0]["value"], "peer1");
    let unban = super::route_http_request("DELETE", "/api/bans/username/peer1", None, "", &state)
        .await
        .expect("username unban");
    let unban_json = serde_json::from_str::<serde_json::Value>(&unban.body).unwrap();
    assert_eq!(unban_json["removed"], true);
    assert_eq!(unban_json["persisted"], false);
    assert_eq!(unban_json["activeBans"], 0);

    let share_token = super::route_http_request(
        "POST",
        "/api/share-grants/grant-1/token",
        None,
        "{}",
        &state,
    )
    .await
    .expect("share grant token");
    let share_token_json = serde_json::from_str::<serde_json::Value>(&share_token.body).unwrap();
    assert_eq!(share_token_json["token"], serde_json::Value::Null);
    assert_eq!(share_token_json["created"], false);
    assert_eq!(share_token_json["persisted"], false);
    assert_eq!(share_token_json["status"], "compatibility_acknowledgement");

    let subscriptions = super::route_http_request(
        "GET",
        "/api/musicbrainz/release-radar/subscriptions",
        None,
        "",
        &state,
    )
    .await
    .expect("subscriptions");
    let subscriptions_json =
        serde_json::from_str::<serde_json::Value>(&subscriptions.body).unwrap();
    assert_eq!(subscriptions_json.as_array().unwrap().len(), 0);

    let created_subscription = super::route_http_request(
        "POST",
        "/api/musicbrainz/release-radar/subscriptions",
        None,
        "{}",
        &state,
    )
    .await
    .expect("subscription create");
    let created_subscription_json =
        serde_json::from_str::<serde_json::Value>(&created_subscription.body).unwrap();
    assert_eq!(created_subscription_json["created"], true);
    assert_eq!(created_subscription_json["persisted"], true);
    assert_eq!(created_subscription_json["status"], "local");
    assert_eq!(created_subscription_json["count"], 1);
    assert_eq!(
        created_subscription_json["subscriptions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn toml_config_sanitizes_secrets_and_storage_paths() {
    let music_path = std::env::temp_dir()
        .join("slskr-test-music")
        .display()
        .to_string()
        .replace('\\', "/");
    let file_config = toml::from_str::<FileConfig>(&format!(
        r#"
            [app]
            http_bind = "127.0.0.1:7788"
            state_dir = "/tmp/slskr-state"
            auto_connect = true
            reconnect = false
            reconnect_seconds = 7
            ping_seconds = 11

            [network]
            server_address = "example.invalid:2242"
            listen_port = 3333
            username = "alice"
            password = "secret-password"

            [network.obfuscation]
            enabled = true
            mode = "prefer"
            prefer_outbound = true

            [listeners]
            regular_bind = "0.0.0.0:3333"
            advertised_port = 4444
            obfuscated_bind = "0.0.0.0:3334"
            obfuscated_advertised_port = 4445
            overlay_bind = "0.0.0.0:50305"

            [dht]
            enabled = true
            port = 6881

            [profile]
            user_info_description = "custom daemon"

            [timeouts]
            peer_response_seconds = 9

            [shares]
            dirs = ['{}']
            fixture = "Virtual/Song.flac=42"
            follow_symlinks = true
            include_hidden = true
            scan_max_files = 123
            cache_tsv_enabled = false

            [transfers]
            history_limit = 12
            max_active = 2
            allow_inbound = false
            allow_outbound = false

            [auth]
            disabled = false
            api_token = "test-token"
            cookie_auth_enabled = true

            [integrations.external_visualizer]
            command = "projectm"
            launch_enabled = true
        "#,
        music_path,
    ))
    .unwrap();

    let config = super::AppConfig::from_layers(
        Some(PathBuf::from("/tmp/slskr/config.toml")),
        file_config,
        &MapEnv::default(),
    )
    .unwrap();

    assert_eq!(config.http_bind.to_string(), "127.0.0.1:7788");
    assert_eq!(config.server_address, "example.invalid:2242");
    assert_eq!(config.listen_port, 3333);
    assert_eq!(config.advertised_port, 4444);
    assert_eq!(config.obfuscated_advertised_port, Some(4445));
    assert_eq!(
        config.overlay_bind.map(|bind| bind.to_string()),
        Some("0.0.0.0:50305".to_owned())
    );
    assert!(config.dht_enabled);
    assert_eq!(config.dht_port, 6881);
    assert_eq!(config.obfuscation_mode.as_str(), "prefer");
    assert!(config.prefer_obfuscated_outbound());
    assert!(config.auto_connect);
    assert!(!config.reconnect);
    assert_eq!(config.reconnect_delay.as_secs(), 7);
    assert_eq!(config.ping_interval.as_secs(), 11);
    assert_eq!(config.user_info_description, "custom daemon");
    assert_eq!(config.peer_response_timeout.as_secs(), 9);
    assert_eq!(config.share_settings.roots, vec![PathBuf::from(music_path)]);
    assert_eq!(config.share_settings.fixture_entries.len(), 1);
    assert!(!config.share_settings.cache_tsv_enabled);
    assert_eq!(config.transfer_history_limit, 12);
    assert_eq!(config.transfer_max_active, 2);
    assert!(!config.transfer_allow_inbound);
    assert!(!config.transfer_allow_outbound);
    assert!(config.auth_required);
    assert_eq!(config.api_token.as_deref(), Some("test-token"));
    assert!(config.api_cookie_auth_enabled);
    assert_eq!(
        config.integrations.external_visualizer.command.as_deref(),
        Some("projectm")
    );
    assert!(config.integrations.external_visualizer.launch_enabled);

    let sanitized = config.sanitized_json();
    assert!(sanitized.contains("\"credentials_configured\":true"));
    assert!(sanitized.contains("\"overlay_bind\":\"0.0.0.0:50305\""));
    assert!(sanitized.contains("\"dht_enabled\":true"));
    assert!(sanitized.contains("\"dht_port\":6881"));
    assert!(sanitized.contains("\"transfer_max_active\":2"));
    assert!(sanitized.contains("\"transfer_allow_inbound\":false"));
    assert!(sanitized.contains("\"transfer_allow_outbound\":false"));
    assert!(sanitized.contains("\"api_token_configured\":true"));
    assert!(sanitized.contains("\"api_cookie_auth_enabled\":true"));
    assert!(sanitized.contains("\"share_cache_tsv_enabled\":false"));
    assert!(sanitized.contains("\"launch_enabled\":true"));
    assert!(sanitized.contains("\"command\":null"));
    assert!(sanitized.contains("a***e"));
    assert!(sanitized.contains("\"config_file\":\"config://file\""));
    assert!(sanitized.contains("\"state_dir\":\"state://configured\""));
    assert!(sanitized.contains("\"credential_file\":\"credential://configured\""));
    assert!(!sanitized.contains("/tmp/slskr"));
    assert!(!sanitized.contains("secret-password"));
    assert!(!sanitized.contains("test-token"));
    assert!(!sanitized.contains("projectm"));
    assert!(!sanitized.contains("\"alice\""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn lidarr_projections_redact_endpoint_and_errors() {
    let env = MapEnv::default()
        .with("SLSKR_LIDARR_ENABLED", "true")
        .with("SLSKR_LIDARR_URL", "http://lidarr.internal:8686/private")
        .with("SLSKR_LIDARR_API_KEY", "lidarr-secret")
        .with("SLSKR_LIDARR_DELETE_REJECTED_DOWNLOADS", "true")
        .with("SLSKR_LIDARR_BLACKLIST_REJECTED_DOWNLOADS", "true");
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("Lidarr config");
    let json = config.integrations.lidarr.sanitized_json();

    assert!(json.contains("\"url\":null"));
    assert!(json.contains("\"url_configured\":true"));
    assert!(!json.contains("lidarr.internal"));
    assert!(!json.contains("/private"));
    assert!(!json.contains("lidarr-secret"));
    assert!(json.contains("\"delete_rejected_downloads\":true"));
    assert!(json.contains("\"blacklist_rejected_downloads\":true"));
    assert_eq!(
        super::public_lidarr_error(Some(
            "request to http://lidarr.internal:8686/private failed"
        )),
        Some("Lidarr connection failed")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn lidarr_rejected_candidates_emit_distinct_portable_filenames() {
    let candidates = vec![
        serde_json::json!({
            "id": 1,
            "path": "/lidarr/Album/accepted.flac",
            "artist": {"id": 2},
            "album": {"id": 3},
            "albumReleaseId": 4,
            "tracks": [{"id": 5}],
            "quality": {"quality": {"id": 6}},
            "additionalFile": false,
            "rejections": []
        }),
        serde_json::json!({
            "path": r"C:\lidarr\Album\Rejected.FLAC",
            "rejections": [{"reason": "quality mismatch"}]
        }),
        serde_json::json!({
            "path": "/lidarr/Album/rejected.flac",
            "rejections": [{"reason": "quality mismatch"}]
        }),
    ];

    assert_eq!(
        super::lidarr_rejected_filenames(&candidates),
        vec!["Rejected.FLAC".to_owned()]
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn lidarr_import_mapping_preserves_native_windows_destination_separators() {
    assert_eq!(
        super::lidarr_map_import_path(
            "/music/downloaded/2 Chainz - T.R.U. REALigion (Anniversary Edition)",
            "/music/downloaded",
            r"D:\downloaded",
        ),
        r"D:\downloaded\2 Chainz - T.R.U. REALigion (Anniversary Edition)"
    );
    assert_eq!(
        super::lidarr_map_import_path("/downloads/Album", "/downloads", "/library",),
        "/library/Album"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn lidarr_rejected_file_deletion_stays_in_the_completed_directory() {
    let root = std::env::temp_dir().join(format!(
        "slskr-lidarr-rejected-files-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let inside = root.join("rejected.flac");
    let outside = root.parent().unwrap().join(format!(
        "slskr-lidarr-outside-{}-{}.flac",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&inside, b"rejected").unwrap();
    std::fs::write(&outside, b"protected").unwrap();

    let deleted = super::delete_lidarr_rejected_files(
        root.to_str().unwrap(),
        &[
            "rejected.flac".to_owned(),
            "../".to_owned() + outside.file_name().unwrap().to_str().unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(deleted, 1);
    assert!(!inside.exists());
    assert!(outside.exists());

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&outside);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn lidarr_rejection_policy_blacklists_wishlist_origin_and_deletes_files() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_LIDARR_DELETE_REJECTED_DOWNLOADS", "true")
            .with("SLSKR_LIDARR_BLACKLIST_REJECTED_DOWNLOADS", "true"),
    );
    let item = state
        .wishlist
        .write()
        .await
        .add_item("Artist".to_owned(), "Album".to_owned(), "Audio".to_owned())
        .unwrap();
    let root = std::env::temp_dir().join(format!(
        "slskr-lidarr-policy-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("track.flac");
    std::fs::write(&file, b"rejected").unwrap();
    let transfer = {
        let mut transfers = state.transfers.write().await;
        transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Remote/Album/track.flac".to_owned(),
            Some(file.display().to_string()),
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
        "rejectedFilenames": ["track.flac"]
    });

    super::apply_lidarr_rejection_policy(&state, &transfer, root.to_str().unwrap(), &result)
        .await
        .unwrap();

    assert!(!file.exists());
    let ignored = state
        .wishlist
        .read()
        .await
        .list_ignored_results(&item.id)
        .unwrap();
    assert_eq!(ignored.len(), 1);
    assert_eq!(ignored[0].username, "peer");
    assert_eq!(ignored[0].directory, "Remote/Album");

    let _ = std::fs::remove_dir_all(&root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn bridge_projections_redact_internal_endpoint() {
    let internal_host = "bridge.internal.private";
    let internal_port = "43127";
    let env = MapEnv::default()
        .with("SLSKR_BRIDGE_ENABLED", "true")
        .with("SLSKR_BRIDGE_HOST", internal_host)
        .with("SLSKR_BRIDGE_PORT", internal_port);
    let (state, _receiver) = test_state_with_env_parts(env, super::SearchStore::new(), None);

    let sanitized = state.config.integrations.bridge.sanitized_json();
    assert!(sanitized.contains("\"host\":null"));
    assert!(sanitized.contains("\"port\":null"));
    assert!(!sanitized.contains(internal_host));
    assert!(!sanitized.contains(internal_port));

    for path in [
        "/api/bridge/admin/config",
        "/api/bridge/admin/dashboard",
        "/api/bridge/status",
        "/api/application",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("bridge projection");
        assert_eq!(response.status, "200 OK", "{path}");
        assert!(!response.body.contains(internal_host), "{path}");
        assert!(!response.body.contains(internal_port), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spotify_diagnostic_projections_redact_redirect_uri() {
    let private_redirect = "https://spotify.internal/private/callback";
    let env = MapEnv::default()
        .with("SLSKR_SPOTIFY_ENABLED", "true")
        .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
        .with("SLSKR_SPOTIFY_REDIRECT_URI", private_redirect);
    let (state, _receiver) = test_state_with_env_parts(env, super::SearchStore::new(), None);

    let sanitized = state.config.integrations.spotify.sanitized_json();
    assert!(sanitized.contains("\"redirect_uri\":null"));
    assert!(sanitized.contains("\"redirect_uri_configured\":true"));
    assert!(!sanitized.contains("spotify.internal"));
    assert!(!sanitized.contains("/private/callback"));

    let status =
        super::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
            .await
            .expect("Spotify status");
    assert!(!status.body.contains(private_redirect));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&status.body).unwrap(),
        serde_json::json!({
            "configured": true,
            "connected": false,
            "displayName": "",
            "spotifyUserId": "",
            "scope": "",
        })
    );

    let authorize = super::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("Spotify authorize");
    assert!(authorize.body.contains("spotify.internal"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn non_loopback_bind_uses_controller_login_when_api_key_is_absent() {
    let env = MapEnv::default().with("SLSKR_HTTP_BIND", "0.0.0.0:5030");
    let config = super::AppConfig::from_layers(None, FileConfig::default(), &env)
        .expect("controller login credentials protect non-loopback binds");

    assert!(config.auth_required);
    assert!(config.api_token.is_none());
    assert_eq!(config.controller_web_auth_username, "slskr");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn peer_host_override_is_sanitized_config_only() {
    let env = MapEnv::default().with("SLSKR_PEER_HOST_OVERRIDE", "127.0.0.1");
    let config = super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    assert_eq!(
        config.peer_host_override,
        Some(std::net::Ipv4Addr::new(127, 0, 0, 1))
    );
    assert!(config
        .sanitized_json()
        .contains("\"peer_host_override\":\"127.0.0.1\""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scoped_file_storage_delete_rejects_traversal_and_deletes_only_under_root() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-file-delete-test-{}-{unique}",
        std::process::id()
    ));
    let nested = root.join("artist");
    std::fs::create_dir_all(&nested).expect("create nested dir");
    let file = nested.join("track.flac");
    std::fs::write(&file, b"fixture").expect("write file");

    let encoded_file = super::STANDARD.encode("artist/track.flac");
    assert_eq!(
        super::delete_scoped_file_storage_path(&root, &encoded_file, false),
        Ok(true)
    );
    assert!(!file.exists());

    let traversal = super::STANDARD.encode("../outside.flac");
    let error = super::delete_scoped_file_storage_path(&root, &traversal, false)
        .expect_err("traversal must fail");
    assert!(error.contains("relative"));

    let album = root.join("album/disc");
    std::fs::create_dir_all(&album).expect("create recursive directory");
    std::fs::write(album.join("song.flac"), b"fixture").expect("write recursive file");
    let encoded_album = super::STANDARD.encode("album");
    assert_eq!(
        super::delete_scoped_file_storage_path(&root, &encoded_album, true),
        Ok(true)
    );
    assert!(!root.join("album").exists());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scoped_storage_confined_delete_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-confined-delete-test-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-confined-delete-outside-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::create_dir_all(&outside).expect("create outside");
    let victim = outside.join("victim.flac");
    std::fs::write(&victim, b"keep").expect("write victim");
    symlink(&outside, root.join("linked")).expect("symlinked parent");

    assert!(super::controller_storage::delete_scoped_storage_path_unix(
        &root,
        &root.join("linked/victim.flac"),
        false
    )
    .is_err());
    assert_eq!(std::fs::read(&victim).unwrap(), b"keep");

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scoped_file_storage_delete_removes_dangling_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-dangling-delete-test-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create root");
    let link = root.join("dangling.flac");
    let missing_target = root.join("missing-target.flac");
    symlink(&missing_target, &link).expect("create dangling symlink");
    assert!(!link.exists(), "fixture must be dangling");
    assert!(link.symlink_metadata().is_ok(), "symlink must exist");

    let encoded = super::STANDARD.encode("dangling.flac");
    assert_eq!(
        super::delete_scoped_file_storage_path(&root, &encoded, false),
        Ok(true)
    );
    assert!(link.symlink_metadata().is_err());
    assert!(!missing_target.exists());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scoped_storage_delete_bounds_directory_depth() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-deep-delete-test-{}-{unique}",
        std::process::id()
    ));
    let tree = root.join("tree");
    let mut directory = tree.clone();
    for depth in 0..=super::controller_storage::SLSKD_STORAGE_MAX_DELETE_DEPTH {
        directory.push(format!("d{depth:02}"));
    }
    std::fs::create_dir_all(&directory).expect("create deep directory tree");

    let encoded = super::STANDARD.encode("tree");
    assert_eq!(
        super::delete_scoped_file_storage_path(&root, &encoded, true).unwrap_err(),
        super::STORAGE_DIRECTORY_DELETE_DEPTH_ERROR
    );
    assert!(tree.exists());
    assert!(directory.exists());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scoped_storage_delete_bounds_directory_width() {
    let mut scanned = super::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES - 1;
    let mut total = 0;
    super::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
        .expect("last delete scan slot");
    assert_eq!(scanned, super::SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES);
    assert_eq!(
        super::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
            .unwrap_err(),
        super::STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scoped_storage_delete_bounds_aggregate_entries() {
    let mut scanned = 0;
    let mut total = super::SLSKD_STORAGE_MAX_DELETE_TOTAL_ENTRIES - 1;
    super::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
        .expect("last aggregate delete slot");
    assert_eq!(total, super::SLSKD_STORAGE_MAX_DELETE_TOTAL_ENTRIES);
    assert_eq!(
        super::controller_storage::reserve_storage_delete_entry(&mut scanned, &mut total)
            .unwrap_err(),
        super::STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn configured_api_token_protects_api_routes() {
    std::thread::Builder::new()
        .name("configured-api-token-test".to_owned())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(configured_api_token_protects_api_routes_impl())
        })
        .unwrap()
        .join()
        .unwrap();
}

async fn configured_api_token_protects_api_routes_impl() {
    let env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            &std::env::temp_dir().display().to_string(),
        )
        .with("SLSKR_API_TOKEN", "route-token");
    let config =
        super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("auth config");
    let (sender, _receiver) = mpsc::channel(8);
    let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
    let rate_limiter = super::rate_limit::RateLimiter::new(super::rate_limit::RateLimitConfig {
        max_requests_anonymous: 1000,
        max_requests_authenticated: 5000,
        window_seconds: 60,
        enabled: true,
    });
    let share_index = super::build_share_index(&config);
    let share_lifecycle = super::ShareLifecycleState::from_snapshot(&share_index);

    let state = Arc::new(super::AppState {
        controller_version: std::sync::RwLock::new(super::ControllerVersionState::initial()),
        controller_cli_environment: BTreeMap::new(),
        log_level: RwLock::new(super::logging::LogLevel::Info),
        runtime_credentials: RwLock::new(None),
        configured_credentials: RwLock::new(config.credentials()),
        controller_web_auth_username: std::sync::RwLock::new(
            config.controller_web_auth_username.clone(),
        ),
        controller_web_auth_password: std::sync::RwLock::new(
            config.controller_web_auth_password.clone(),
        ),
        controller_web_jwt_key_current: std::sync::RwLock::new(
            config.controller_web_jwt_key.clone(),
        ),
        session: RwLock::new(super::SessionSnapshot::disconnected(&config)),
        server_address: std::sync::RwLock::new(config.server_address.clone()),
        connected_server_address: std::sync::RwLock::new(None),
        listeners: RwLock::new(super::ListenerSnapshot::new(&config)),
        distributed_network: RwLock::new(super::DistributedRuntime::new(
            config.username.as_deref(),
        )),
        distributed_persistence_snapshots: tokio::sync::watch::channel(
            super::DistributedRuntime::new(config.username.as_deref()).persistence_snapshot(),
        )
        .0,
        distributed_persistence_status: tokio::sync::watch::channel(
            super::DistributedPersistenceStatus {
                revision: 0,
                result: Ok(()),
            },
        )
        .1,
        soulseek_distributed_settings: RwLock::new(config.soulseek_distributed),
        shares: RwLock::new(share_index),
        share_settings: RwLock::new(config.share_settings.clone()),
        share_index_persistence_lock: tokio::sync::Mutex::new(()),
        share_settings_generation: std::sync::atomic::AtomicU64::new(0),
        core_workflow_settings: RwLock::new(config.core_workflow.clone()),
        advanced_networking: RwLock::new(config.advanced_networking.clone()),
        media_services: RwLock::new(config.media_services.clone()),
        share_lifecycle: RwLock::new(share_lifecycle),
        downloads_dir: std::sync::RwLock::new(config.downloads_dir.clone()),
        incomplete_dir: std::sync::RwLock::new(config.incomplete_dir.clone()),
        download_completed_path_template: std::sync::RwLock::new(
            config.download_completed_path_template.clone(),
        ),
        remote_file_management: std::sync::RwLock::new(config.remote_file_management),
        remote_configuration: std::sync::RwLock::new(config.remote_configuration),
        controller_no_config_watch: std::sync::RwLock::new(config.controller_no_config_watch),
        controller_case_sensitive_regex: std::sync::RwLock::new(
            config.controller_case_sensitive_regex,
        ),
        user_info_description: std::sync::RwLock::new(config.user_info_description.clone()),
        user_info_picture: std::sync::RwLock::new(config.user_info_picture.clone()),
        controller_options_validation_error: std::sync::RwLock::new(None),
        regular_listener_commands: None,
        obfuscated_listener_commands: None,
        advertised_port: std::sync::RwLock::new(config.advertised_port),
        obfuscated_advertised_port: std::sync::RwLock::new(config.obfuscated_advertised_port),
        searches: RwLock::new(super::SearchStore::new()),
        users: RwLock::new(super::UserStore::new()),
        user_persistence_lock: tokio::sync::Mutex::new(()),
        event_persistence_lock: tokio::sync::Mutex::new(()),
        mesh: RwLock::new(super::MeshState::new()),
        capability_signing_key: crate::controller_capabilities::new_capability_signing_key()
            .expect("capability signing key"),
        content_discovery: RwLock::new(super::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(super::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(super::BrowseStore::new()),
        browse_persistence_lock: tokio::sync::Mutex::new(()),
        remote_path_encodings: RwLock::new(super::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(super::MessageStore::new()),
        message_persistence_lock: tokio::sync::Mutex::new(()),
        managed_blacklist: RwLock::new(super::ManagedBlacklistRuntime::new(
            config.managed_blacklist.clone(),
            config.controller_profile,
            config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(
            super::compile_controller_regexes(
                &config.controller_search_request_filters,
                config.controller_case_sensitive_regex,
                config.controller_profile,
            )
            .unwrap(),
        ),
        integration_settings: RwLock::new(config.integrations.clone()),
        source_feed_import_history: RwLock::new(super::SourceFeedImportHistoryStore::default()),
        source_feed_import_history_persistence_lock: tokio::sync::Mutex::new(()),
        lidarr_sync_state: RwLock::new(super::LidarrSyncRuntimeState::new(
            &config.integrations.lidarr,
        )),
        lidarr_recent_imports: RwLock::new(BTreeMap::new()),
        lidarr_import_gate: tokio::sync::Semaphore::new(1),
        private_message_auto_response_settings: RwLock::new(
            config.private_message_auto_response.clone(),
        ),
        transfer_auto_retry_settings: RwLock::new(config.transfer_auto_retry.clone()),
        transfer_upload_settings: RwLock::new(config.transfer_upload.clone()),
        transfer_download_settings: RwLock::new(config.transfer_download.clone()),
        transfer_groups_settings: RwLock::new(config.transfer_groups.clone()),
        failed_upload_peer_cooldowns: RwLock::new(super::UploadPeerCooldowns::default()),
        private_message_auto_responses: RwLock::new(
            super::PrivateMessageAutoResponseTracker::default(),
        ),
        rooms: RwLock::new(super::RoomStore::new()),
        room_persistence_lock: tokio::sync::Mutex::new(()),
        pod_join_replays: RwLock::new(super::PodJoinReplayStore::default()),
        pod_membership_workflow: RwLock::new(super::PodMembershipWorkflowStore::default()),
        pod_channels: RwLock::new(super::pod_channels::PodChannelStore::empty(
            &config.state_dir,
        )),
        pods: RwLock::new(super::pods::PodStore::empty(&config.state_dir)),
        port_forwarding: super::port_forwarding::Manager::new(),
        private_gateway: None,
        dht: None,
        transfers: RwLock::new(super::TransferQueue::new(&config)),
        events: RwLock::new(super::EventStore::new(super::EVENT_HISTORY_LIMIT)),
        event_tx: event_tx.clone(),
        webhooks: Arc::new(RwLock::new(super::webhooks::WebhookManager::new())),
        webhook_deliveries: Arc::new(super::Semaphore::new(super::MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(super::Semaphore::new(super::MAX_SHARE_SCAN_TASKS)),
        share_scan_cancellation: Arc::new(std::sync::Mutex::new(None)),
        incoming_connections: Arc::new(super::Semaphore::new(super::MAX_INCOMING_CONNECTION_TASKS)),
        incoming_connection_ips: std::sync::Mutex::new(BTreeMap::new()),
        incoming_searches: Arc::new(super::Semaphore::new(
            config.core_workflow.incoming_search.concurrency,
        )),
        incoming_search_queue_depth: super::AtomicUsize::new(0),
        download_requests: Arc::new(super::Semaphore::new(2)),
        download_batch_requests: Arc::new(super::Semaphore::new(1)),
        websocket_connections: Arc::new(super::Semaphore::new(super::MAX_WEBSOCKET_CONNECTIONS)),
        external_visualizer_processes: Arc::new(super::Semaphore::new(
            super::MAX_EXTERNAL_VISUALIZER_PROCESSES,
        )),
        songid_run_slots: Arc::new(super::Semaphore::new(
            config.media_services.song_id_max_concurrent_runs,
        )),
        songid_jobs: None,
        collections: RwLock::new(super::CollectionStore::new()),
        collection_grant_persistence_lock: tokio::sync::Mutex::new(()),
        share_group_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist_search_persistence_lock: tokio::sync::Mutex::new(()),
        search_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist: RwLock::new(super::WishlistStore::new()),
        contact_persistence_lock: tokio::sync::Mutex::new(()),
        contacts: RwLock::new(super::ContactStore::new()),
        sharegroups: RwLock::new(super::ShareGroupStore::new()),
        user_note_persistence_lock: tokio::sync::Mutex::new(()),
        user_notes: RwLock::new(super::UserNoteStore::new()),
        interest_persistence_lock: tokio::sync::Mutex::new(()),
        interests: RwLock::new(super::InterestStore::new()),
        now_playing_persistence_lock: tokio::sync::Mutex::new(()),
        now_playing: RwLock::new(super::NowPlayingStore::new()),
        webhook_persistence_lock: Arc::new(tokio::sync::Mutex::new(())),
        relay: RwLock::new(super::RelayState::new()),
        runtime: RwLock::new(super::RuntimeCompatState::new()),
        runtime_persistence_lock: tokio::sync::Mutex::new(()),
        options_overlay: RwLock::new(super::ControllerOptionsOverlayState::default()),
        diagnostics_allow_memory_dump: RwLock::new(config.controller_diagnostics_allow_memory_dump),
        diagnostics_allow_remote_dump: RwLock::new(config.controller_diagnostics_allow_remote_dump),
        backfill: RwLock::new(super::BackfillState::default()),
        backfill_connections: Arc::new(super::Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security_ban_persistence_lock: tokio::sync::Mutex::new(()),
        security: RwLock::new(super::SecurityState::new()),
        share_grants: RwLock::new(super::ShareGrantStore::new()),
        share_access_tokens: RwLock::new(super::ShareAccessTokenStore::default()),
        incoming_shares: RwLock::new(super::IncomingShareStore::default()),
        library: RwLock::new(super::LibraryStore::new()),
        library_persistence_lock: tokio::sync::Mutex::new(()),
        virtual_soulfind_v2: Arc::new(RwLock::new(super::virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(super::SourceDiscoveryState::default()),
        destinations: RwLock::new(super::DestinationStore::new()),
        db: None,
        config,
        session_commands: sender.clone(),
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands: None,
        managed_background_tasks: super::ManagedTaskRegistry::default(),
        rate_limiter,
        soulseek_safety: super::rate_limit::SoulseekSafetyLimiter::new(
            super::rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(super::OAuthStateStore::default()),
        oauth_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection: RwLock::new(super::SpotifyConnectionStore::default()),
        spotify_connection_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection_generation: std::sync::atomic::AtomicU64::new(0),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(super::PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(super::multisource::SwarmStore::default())),
        controller_features: super::ControllerFeatureStore::new(
            super::ControllerFeatureState::in_memory(),
        ),
        peer_endpoints: RwLock::new(BTreeMap::new()),
        preview_streams: Arc::new(tokio::sync::Semaphore::new(super::MAX_PREVIEW_STREAMS)),
        listening_party_stream_limits: RwLock::new(super::ListeningPartyStreamLimits::default()),
        revoked_jwts: RwLock::new(super::RevokedJwtStore::default()),
        login_attempts: RwLock::new(super::LoginAttemptStore::default()),
        pod_signature_stats: super::PodSignatureStats::default(),
        pod_verification_stats: super::PodVerificationStats::default(),
        pod_dht_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_failed_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_publish_time_ms: std::sync::atomic::AtomicU64::new(0),
        podcore_runtime_stats: super::PodCoreRuntimeStats::default(),
    });
    let missing = super::route_http_request("GET", "/api/v0/config", None, "", &state)
        .await
        .unwrap();
    assert_eq!(missing.status, "401 Unauthorized");

    let wrong =
        super::route_http_request("GET", "/api/v0/config", Some("Bearer wrong"), "", &state)
            .await
            .unwrap();
    assert_eq!(wrong.status, "401 Unauthorized");

    let allowed = super::route_http_request(
        "GET",
        "/api/v0/config",
        Some("Bearer route-token"),
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(allowed.status, "200 OK");

    let x_api_key_allowed = super::route_http_request(
        "GET",
        "/api/v0/config",
        Some("ApiKey route-token"),
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(x_api_key_allowed.status, "200 OK");

    let cross_site = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        Some("Bearer route-token"),
        "",
        &state,
        super::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: Some("https://evil.example".to_string()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(cross_site.status, "202 Accepted");

    let same_origin = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        Some("Bearer route-token"),
        "",
        &state,
        super::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: Some("http://127.0.0.1:5030".to_string()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(same_origin.status, "202 Accepted");

    let cookie_rejected_by_default = super::route_http_request_with_headers(
        "GET",
        "/api/v0/config",
        None,
        "",
        &state,
        super::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: None,
            referer: None,
            cookie: Some("other=value; slskr.session=route-token".to_string()),
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(cookie_rejected_by_default.status, "401 Unauthorized");

    let cookie_enabled_env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            &std::env::temp_dir().display().to_string(),
        )
        .with("SLSKR_API_TOKEN", "route-token")
        .with("SLSKR_API_COOKIE_AUTH_ENABLED", "true");
    let cookie_enabled_config =
        super::AppConfig::from_layers(None, FileConfig::default(), &cookie_enabled_env)
            .expect("cookie auth config");
    let cookie_share_index = super::build_share_index(&cookie_enabled_config);
    let cookie_share_lifecycle = super::ShareLifecycleState::from_snapshot(&cookie_share_index);
    let cookie_enabled_state = super::AppState {
        controller_version: std::sync::RwLock::new(super::ControllerVersionState::initial()),
        controller_cli_environment: BTreeMap::new(),
        log_level: RwLock::new(super::logging::LogLevel::Info),
        runtime_credentials: RwLock::new(None),
        configured_credentials: RwLock::new(cookie_enabled_config.credentials()),
        controller_web_auth_username: std::sync::RwLock::new(
            cookie_enabled_config.controller_web_auth_username.clone(),
        ),
        controller_web_auth_password: std::sync::RwLock::new(
            cookie_enabled_config.controller_web_auth_password.clone(),
        ),
        controller_web_jwt_key_current: std::sync::RwLock::new(
            cookie_enabled_config.controller_web_jwt_key.clone(),
        ),
        session: RwLock::new(super::SessionSnapshot::disconnected(&cookie_enabled_config)),
        server_address: std::sync::RwLock::new(cookie_enabled_config.server_address.clone()),
        connected_server_address: std::sync::RwLock::new(None),
        listeners: RwLock::new(super::ListenerSnapshot::new(&cookie_enabled_config)),
        distributed_network: RwLock::new(super::DistributedRuntime::new(
            cookie_enabled_config.username.as_deref(),
        )),
        distributed_persistence_snapshots: tokio::sync::watch::channel(
            super::DistributedRuntime::new(cookie_enabled_config.username.as_deref())
                .persistence_snapshot(),
        )
        .0,
        distributed_persistence_status: tokio::sync::watch::channel(
            super::DistributedPersistenceStatus {
                revision: 0,
                result: Ok(()),
            },
        )
        .1,
        soulseek_distributed_settings: RwLock::new(cookie_enabled_config.soulseek_distributed),
        shares: RwLock::new(cookie_share_index),
        share_settings: RwLock::new(cookie_enabled_config.share_settings.clone()),
        share_index_persistence_lock: tokio::sync::Mutex::new(()),
        share_settings_generation: std::sync::atomic::AtomicU64::new(0),
        core_workflow_settings: RwLock::new(cookie_enabled_config.core_workflow.clone()),
        advanced_networking: RwLock::new(cookie_enabled_config.advanced_networking.clone()),
        media_services: RwLock::new(cookie_enabled_config.media_services.clone()),
        share_lifecycle: RwLock::new(cookie_share_lifecycle),
        downloads_dir: std::sync::RwLock::new(cookie_enabled_config.downloads_dir.clone()),
        incomplete_dir: std::sync::RwLock::new(cookie_enabled_config.incomplete_dir.clone()),
        download_completed_path_template: std::sync::RwLock::new(
            cookie_enabled_config
                .download_completed_path_template
                .clone(),
        ),
        remote_file_management: std::sync::RwLock::new(
            cookie_enabled_config.remote_file_management,
        ),
        remote_configuration: std::sync::RwLock::new(cookie_enabled_config.remote_configuration),
        controller_no_config_watch: std::sync::RwLock::new(
            cookie_enabled_config.controller_no_config_watch,
        ),
        controller_case_sensitive_regex: std::sync::RwLock::new(
            cookie_enabled_config.controller_case_sensitive_regex,
        ),
        user_info_description: std::sync::RwLock::new(
            cookie_enabled_config.user_info_description.clone(),
        ),
        user_info_picture: std::sync::RwLock::new(cookie_enabled_config.user_info_picture.clone()),
        controller_options_validation_error: std::sync::RwLock::new(None),
        regular_listener_commands: None,
        obfuscated_listener_commands: None,
        advertised_port: std::sync::RwLock::new(cookie_enabled_config.advertised_port),
        obfuscated_advertised_port: std::sync::RwLock::new(
            cookie_enabled_config.obfuscated_advertised_port,
        ),
        searches: RwLock::new(super::SearchStore::new()),
        users: RwLock::new(super::UserStore::new()),
        user_persistence_lock: tokio::sync::Mutex::new(()),
        event_persistence_lock: tokio::sync::Mutex::new(()),
        mesh: RwLock::new(super::MeshState::new()),
        capability_signing_key: crate::controller_capabilities::new_capability_signing_key()
            .expect("capability signing key"),
        content_discovery: RwLock::new(super::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(super::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(super::BrowseStore::new()),
        browse_persistence_lock: tokio::sync::Mutex::new(()),
        remote_path_encodings: RwLock::new(super::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(super::MessageStore::new()),
        message_persistence_lock: tokio::sync::Mutex::new(()),
        managed_blacklist: RwLock::new(super::ManagedBlacklistRuntime::new(
            cookie_enabled_config.managed_blacklist.clone(),
            cookie_enabled_config.controller_profile,
            cookie_enabled_config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(
            super::compile_controller_regexes(
                &cookie_enabled_config.controller_search_request_filters,
                cookie_enabled_config.controller_case_sensitive_regex,
                cookie_enabled_config.controller_profile,
            )
            .unwrap(),
        ),
        integration_settings: RwLock::new(cookie_enabled_config.integrations.clone()),
        source_feed_import_history: RwLock::new(super::SourceFeedImportHistoryStore::default()),
        source_feed_import_history_persistence_lock: tokio::sync::Mutex::new(()),
        lidarr_sync_state: RwLock::new(super::LidarrSyncRuntimeState::new(
            &cookie_enabled_config.integrations.lidarr,
        )),
        lidarr_recent_imports: RwLock::new(BTreeMap::new()),
        lidarr_import_gate: tokio::sync::Semaphore::new(1),
        private_message_auto_response_settings: RwLock::new(
            cookie_enabled_config.private_message_auto_response.clone(),
        ),
        transfer_auto_retry_settings: RwLock::new(
            cookie_enabled_config.transfer_auto_retry.clone(),
        ),
        transfer_upload_settings: RwLock::new(cookie_enabled_config.transfer_upload.clone()),
        transfer_download_settings: RwLock::new(cookie_enabled_config.transfer_download.clone()),
        transfer_groups_settings: RwLock::new(cookie_enabled_config.transfer_groups.clone()),
        failed_upload_peer_cooldowns: RwLock::new(super::UploadPeerCooldowns::default()),
        private_message_auto_responses: RwLock::new(
            super::PrivateMessageAutoResponseTracker::default(),
        ),
        rooms: RwLock::new(super::RoomStore::new()),
        room_persistence_lock: tokio::sync::Mutex::new(()),
        pod_join_replays: RwLock::new(super::PodJoinReplayStore::default()),
        pod_membership_workflow: RwLock::new(super::PodMembershipWorkflowStore::default()),
        pod_channels: RwLock::new(super::pod_channels::PodChannelStore::empty(
            &cookie_enabled_config.state_dir,
        )),
        pods: RwLock::new(super::pods::PodStore::empty(
            &cookie_enabled_config.state_dir,
        )),
        port_forwarding: super::port_forwarding::Manager::new(),
        private_gateway: None,
        dht: None,
        transfers: RwLock::new(super::TransferQueue::new(&cookie_enabled_config)),
        events: RwLock::new(super::EventStore::new(super::EVENT_HISTORY_LIMIT)),
        event_tx: event_tx.clone(),
        webhooks: Arc::new(RwLock::new(super::webhooks::WebhookManager::new())),
        webhook_deliveries: Arc::new(super::Semaphore::new(super::MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(super::Semaphore::new(super::MAX_SHARE_SCAN_TASKS)),
        share_scan_cancellation: Arc::new(std::sync::Mutex::new(None)),
        incoming_connections: Arc::new(super::Semaphore::new(super::MAX_INCOMING_CONNECTION_TASKS)),
        incoming_connection_ips: std::sync::Mutex::new(BTreeMap::new()),
        incoming_searches: Arc::new(super::Semaphore::new(
            cookie_enabled_config
                .core_workflow
                .incoming_search
                .concurrency,
        )),
        incoming_search_queue_depth: super::AtomicUsize::new(0),
        download_requests: Arc::new(super::Semaphore::new(2)),
        download_batch_requests: Arc::new(super::Semaphore::new(1)),
        websocket_connections: Arc::new(super::Semaphore::new(super::MAX_WEBSOCKET_CONNECTIONS)),
        external_visualizer_processes: Arc::new(super::Semaphore::new(
            super::MAX_EXTERNAL_VISUALIZER_PROCESSES,
        )),
        songid_run_slots: Arc::new(super::Semaphore::new(
            cookie_enabled_config
                .media_services
                .song_id_max_concurrent_runs,
        )),
        songid_jobs: None,
        collections: RwLock::new(super::CollectionStore::new()),
        collection_grant_persistence_lock: tokio::sync::Mutex::new(()),
        share_group_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist_search_persistence_lock: tokio::sync::Mutex::new(()),
        search_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist: RwLock::new(super::WishlistStore::new()),
        contact_persistence_lock: tokio::sync::Mutex::new(()),
        contacts: RwLock::new(super::ContactStore::new()),
        sharegroups: RwLock::new(super::ShareGroupStore::new()),
        user_note_persistence_lock: tokio::sync::Mutex::new(()),
        user_notes: RwLock::new(super::UserNoteStore::new()),
        interest_persistence_lock: tokio::sync::Mutex::new(()),
        interests: RwLock::new(super::InterestStore::new()),
        now_playing_persistence_lock: tokio::sync::Mutex::new(()),
        now_playing: RwLock::new(super::NowPlayingStore::new()),
        webhook_persistence_lock: Arc::new(tokio::sync::Mutex::new(())),
        relay: RwLock::new(super::RelayState::new()),
        runtime: RwLock::new(super::RuntimeCompatState::new()),
        runtime_persistence_lock: tokio::sync::Mutex::new(()),
        options_overlay: RwLock::new(super::ControllerOptionsOverlayState::default()),
        diagnostics_allow_memory_dump: RwLock::new(
            cookie_enabled_config.controller_diagnostics_allow_memory_dump,
        ),
        diagnostics_allow_remote_dump: RwLock::new(
            cookie_enabled_config.controller_diagnostics_allow_remote_dump,
        ),
        backfill: RwLock::new(super::BackfillState::default()),
        backfill_connections: Arc::new(super::Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security_ban_persistence_lock: tokio::sync::Mutex::new(()),
        security: RwLock::new(super::SecurityState::new()),
        share_grants: RwLock::new(super::ShareGrantStore::new()),
        share_access_tokens: RwLock::new(super::ShareAccessTokenStore::default()),
        incoming_shares: RwLock::new(super::IncomingShareStore::default()),
        library: RwLock::new(super::LibraryStore::new()),
        library_persistence_lock: tokio::sync::Mutex::new(()),
        virtual_soulfind_v2: Arc::new(RwLock::new(super::virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(super::SourceDiscoveryState::default()),
        destinations: RwLock::new(super::DestinationStore::new()),
        db: None,
        config: cookie_enabled_config,
        session_commands: sender.clone(),
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands: None,
        managed_background_tasks: super::ManagedTaskRegistry::default(),
        rate_limiter: super::rate_limit::RateLimiter::new(super::rate_limit::RateLimitConfig {
            max_requests_anonymous: 1000,
            max_requests_authenticated: 5000,
            window_seconds: 60,
            enabled: true,
        }),
        soulseek_safety: super::rate_limit::SoulseekSafetyLimiter::new(
            super::rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(super::OAuthStateStore::default()),
        oauth_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection: RwLock::new(super::SpotifyConnectionStore::default()),
        spotify_connection_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection_generation: std::sync::atomic::AtomicU64::new(0),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(super::PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(super::multisource::SwarmStore::default())),
        controller_features: super::ControllerFeatureStore::new(
            super::ControllerFeatureState::in_memory(),
        ),
        peer_endpoints: RwLock::new(BTreeMap::new()),
        preview_streams: Arc::new(tokio::sync::Semaphore::new(super::MAX_PREVIEW_STREAMS)),
        listening_party_stream_limits: RwLock::new(super::ListeningPartyStreamLimits::default()),
        revoked_jwts: RwLock::new(super::RevokedJwtStore::default()),
        login_attempts: RwLock::new(super::LoginAttemptStore::default()),
        pod_signature_stats: super::PodSignatureStats::default(),
        pod_verification_stats: super::PodVerificationStats::default(),
        pod_dht_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_failed_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_publish_time_ms: std::sync::atomic::AtomicU64::new(0),
        podcore_runtime_stats: super::PodCoreRuntimeStats::default(),
    };
    let cookie_allowed = super::route_http_request_with_headers(
        "GET",
        "/api/v0/config",
        None,
        "",
        &cookie_enabled_state,
        super::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: None,
            referer: None,
            cookie: Some("other=value; slskr.session=route-token".to_string()),
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(cookie_allowed.status, "200 OK");

    let health = super::route_http_request("GET", "/api/v0/health", None, "", &state)
        .await
        .unwrap();
    assert_eq!(health.status, "200 OK");

    let session_enabled =
        super::route_http_request("GET", "/api/v0/session/enabled", None, "", &state)
            .await
            .unwrap();
    assert_eq!(session_enabled.status, "200 OK");
    assert_eq!(session_enabled.body, "true");

    let capabilities = super::route_http_request("GET", "/api/v0/capabilities", None, "", &state)
        .await
        .unwrap();
    assert_eq!(capabilities.status, "401 Unauthorized");
    let public_capabilities =
        super::route_http_request("GET", "/api/capabilities", None, "", &state)
            .await
            .unwrap();
    assert_eq!(public_capabilities.status, "200 OK");
    assert!(public_capabilities.body.contains("\"room-list-sync\""));
    assert!(public_capabilities
        .body
        .contains("\"browser-session-auth\""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn environment_overrides_file_config() {
    let file_config = toml::from_str::<FileConfig>(
        r#"
            [network]
            listen_port = 3333

            [shares]
            scan_max_files = 123

            [transfers]
            max_active = 9
            allow_inbound = true
            allow_outbound = true
        "#,
    )
    .unwrap();
    let env = MapEnv::default()
        .with("SLSK_LISTEN_PORT", "4444")
        .with("SLSKR_SHARE_SCAN_MAX_FILES", "5")
        .with("SLSKR_TRANSFER_MAX_ACTIVE", "1")
        .with("SLSKR_TRANSFER_ALLOW_INBOUND", "false")
        .with("SLSKR_TRANSFER_ALLOW_OUTBOUND", "false");

    let config = super::AppConfig::from_layers(None, file_config, &env).unwrap();

    assert_eq!(config.listen_port, 4444);
    assert_eq!(config.share_settings.max_files, 5);
    assert_eq!(config.transfer_max_active, 1);
    assert!(!config.transfer_allow_inbound);
    assert!(!config.transfer_allow_outbound);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn overlay_bind_rejects_zero_port() {
    let error = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_OVERLAY_BIND", "127.0.0.1:0"),
    )
    .unwrap_err();
    assert_eq!(error, "SLSKR_OVERLAY_BIND port must be non-zero");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn capability_identity_is_stable_across_reloads() {
    let root = std::env::temp_dir().join(format!(
        "slskr-capability-identity-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let first = super::load_or_create_capability_signing_key(&root).unwrap();
    let second = super::load_or_create_capability_signing_key(&root).unwrap();
    assert_eq!(
        first.verifying_key().to_bytes(),
        second.verifying_key().to_bytes()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn capability_identity_rejects_symlinked_key() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "slskr-capability-symlink-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("target-key.bin");
    std::fs::write(&target, [7_u8; 32]).unwrap();
    symlink(&target, root.join("peer-capability-key.bin")).unwrap();
    let error = super::load_or_create_capability_signing_key(&root).unwrap_err();
    assert!(error.contains("must be a regular file"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn toml_config_rejects_unknown_fields() {
    let error = toml::from_str::<FileConfig>(
        r#"
            [app]
            surprise = true
        "#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("unknown field"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn scrubbed_socket_addr_hides_host() {
    let address = "192.0.2.10:2234".parse().unwrap();
    assert_eq!(super::scrub_socket_addr(address), "ipv4:2234");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn peer_message_names_are_stable() {
    assert_eq!(
        super::peer_message_name(&slskr_client::protocol::peer::PeerMessage::UserInfoRequest),
        "UserInfoRequest"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_active_statuses_include_peer_lifecycle() {
    assert!(super::is_active_transfer_status("in_progress"));
    assert!(super::is_active_transfer_status("peer_lookup"));
    assert!(super::is_active_transfer_status("peer_negotiating"));
    assert!(super::is_active_transfer_status("accepted"));
    assert!(super::is_active_transfer_status("indirect_pending"));
    assert!(!super::is_active_transfer_status("queued"));
    assert!(!super::is_active_transfer_status("failed"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_fixture_entries_are_searchable() {
    let entries = crate::config::parse_share_entries("Music/Artist - Song.flac=123").unwrap();
    assert_eq!(entries[0].extension, "flac");
    assert_eq!(super::search_shares(&entries, "artist song").len(), 1);
    assert!(super::search_shares(&entries, "missing").is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_scan_discovers_visible_files() {
    let root = std::env::temp_dir().join(format!("slskr-share-test-{}", std::process::id()));
    let artist = root.join("Artist");
    std::fs::create_dir_all(&artist).unwrap();
    std::fs::write(artist.join("Song.flac"), b"audio").unwrap();
    std::fs::create_dir_all(root.join(".hidden")).unwrap();
    std::fs::write(root.join(".hidden").join("Secret.mp3"), b"hidden").unwrap();

    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();
    let scan = super::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);

    assert_eq!(scan.entries.len(), 1);
    assert!(scan.entries[0].filename.ends_with("/Artist/Song.flac"));
    assert_eq!(scan.entries[0].size, 5);
    assert_eq!(scan.entries[0].extension, "flac");
    assert_eq!(scan.roots[0].files, 1);
    assert_eq!(scan.roots[0].bytes, 5);
    assert_eq!(scan.roots[0].extensions[0].extension, "flac");
    assert_eq!(scan.roots[0].extensions[0].files, 1);
    assert_eq!(scan.roots[0].extensions[0].bytes, 5);
    assert!(scan.roots[0].json().contains("\"bytes\":5"));

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_media_probe_emits_wav_protocol_attributes_only_when_enabled() {
    let root =
        std::env::temp_dir().join(format!("slskr-share-media-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&88_236_u32.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&44_100_u32.to_le_bytes());
    wav.extend_from_slice(&88_200_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&88_200_u32.to_le_bytes());
    wav.resize(88_244, 0);
    std::fs::write(root.join("tone.wav"), wav).unwrap();
    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();

    let probed = super::scan_share_dirs(&directory, false, false, 100, true, 1, &[]);
    assert_eq!(
        probed.entries[0].attributes,
        vec![
            super::FileAttribute {
                code: 0,
                value: 705
            },
            super::FileAttribute { code: 1, value: 1 },
            super::FileAttribute { code: 2, value: 0 },
            super::FileAttribute {
                code: 4,
                value: 44_100,
            },
            super::FileAttribute { code: 5, value: 16 },
        ]
    );
    let skipped = super::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);
    assert!(skipped.entries[0].attributes.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_cache_workers_scan_multiple_roots_deterministically() {
    let root =
        std::env::temp_dir().join(format!("slskr-share-workers-test-{}", uuid::Uuid::new_v4()));
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("a.flac"), b"a").unwrap();
    std::fs::write(second.join("b.flac"), b"b").unwrap();
    let directories = crate::config::parse_share_directories(&format!(
        "[First]{};[Second]{}",
        first.display(),
        second.display()
    ))
    .unwrap();
    let scan = super::scan_share_dirs(&directories, false, false, 100, false, 2, &[]);
    assert_eq!(scan.entries.len(), 2);
    assert_eq!(scan.roots.len(), 2);
    assert_eq!(scan.roots.iter().map(|root| root.files).sum::<usize>(), 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn incoming_search_response_honors_configured_file_limit() {
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT",
        "100",
    ));
    state.shares.write().await.entries = (0..150)
        .map(|index| super::FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: format!("Library/fixture-{index}.flac"),
            size: 1,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        })
        .collect();
    let response = super::build_file_search_response(&state, 1, "fixture")
        .await
        .expect("matching response");
    assert_eq!(response.results.len(), 100);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_scan_applies_alias_and_exclusion_before_indexing() {
    let root = std::env::temp_dir().join(format!(
        "slskr-share-exclusion-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let excluded = root.join("excluded");
    std::fs::create_dir_all(&excluded).unwrap();
    std::fs::write(root.join("visible.flac"), b"visible").unwrap();
    std::fs::write(excluded.join("secret.flac"), b"secret").unwrap();
    let directories = crate::config::parse_share_directories(&format!(
        "[Library]{};!{}",
        root.display(),
        excluded.display()
    ))
    .unwrap();

    let scan = super::scan_share_dirs(&directories, false, false, 100, false, 1, &[]);

    assert_eq!(scan.entries.len(), 1);
    assert_eq!(scan.entries[0].filename, "Library/visible.flac");
    assert_eq!(scan.roots.len(), 2);
    assert_eq!(scan.roots[0].label, "excluded");
    assert!(scan.roots[0].raw.starts_with('!'));
    assert_eq!(scan.roots[0].files, 0);
    assert_eq!(scan.roots[1].label, "Library");
    assert_eq!(scan.roots[1].files, 1);

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_scan_bounds_aggregate_directory_entries() {
    let mut scanned_entries = super::MAX_SHARE_SCAN_ENTRIES - 1;
    assert!(super::reserve_share_scan_entry(&mut scanned_entries));
    assert_eq!(scanned_entries, super::MAX_SHARE_SCAN_ENTRIES);
    assert!(!super::reserve_share_scan_entry(&mut scanned_entries));
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_scan_bounds_pending_directory_descriptors() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-scan-width-test-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    for index in 0..=super::MAX_SHARE_SCAN_PENDING_DIRECTORIES {
        std::fs::create_dir(root.join(format!("directory-{index:04}"))).unwrap();
    }

    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();
    let scan = super::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);

    assert!(scan.entries.is_empty());
    assert!(scan
        .errors
        .iter()
        .any(|error| error == super::SHARE_SCAN_PENDING_DIRECTORY_LIMIT_ERROR));
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_scan_does_not_follow_symlinked_directory() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-scan-symlink-test-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-share-scan-symlink-outside-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(root.join("visible.flac"), b"visible").unwrap();
    std::fs::write(outside.join("secret.flac"), b"secret").unwrap();
    symlink(&outside, root.join("linked")).unwrap();

    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();
    let scan = super::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);
    assert_eq!(scan.entries.len(), 1);
    assert!(scan.entries[0].filename.ends_with("/visible.flac"));
    assert!(!scan
        .entries
        .iter()
        .any(|entry| entry.filename.contains("secret.flac")));

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_cache_escapes_fields() {
    assert_eq!(super::escape_cache_field("a\tb\\c\n"), "a\\tb\\\\c\\n");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_snapshot_errors_redact_internal_details() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-share-error-redaction-test-{}",
        std::process::id()
    ));
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_SHARE_CACHE_TSV_ENABLED", "false");
    let config = super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");
    let mut snapshot = super::build_share_index(&config);
    snapshot.cache_error =
        Some("share cache write failed: /private/share-index.tsv denied".to_owned());

    let json = snapshot.json();
    assert!(json.contains("\"cache_error\":\"share cache unavailable\""));
    assert!(!json.contains("/private"));
    assert!(!json.contains("denied"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_cache_tsv_can_be_disabled_for_sqlite_primary_state() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let state_dir = std::env::temp_dir().join(format!("slskr-share-cache-disabled-{unique}"));
    std::fs::create_dir_all(&state_dir).unwrap();
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_SHARE_FIXTURE", "Virtual/Test.flac=42")
        .with("SLSKR_SHARE_CACHE_TSV_ENABLED", "false");
    let config = super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    let snapshot = super::build_share_index(&config);

    assert_eq!(snapshot.entries.len(), 1);
    assert!(!snapshot.cache_enabled);
    assert!(snapshot.cache_written_at.is_none());
    assert!(snapshot.cache_error.is_none());
    assert!(!super::share_cache_path(&state_dir).exists());
    assert!(snapshot.json().contains("\"cache_enabled\":false"));
    assert!(snapshot
        .json()
        .contains("\"cache_kind\":\"compatibility-debug\""));

    std::fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_queue_records_rejections_with_limit() {
    let mut queue = super::TransferQueue::new_in_memory(1);

    queue.record_rejected_request(1, 10, "a.flac".to_owned(), Some(5), "disabled".to_owned());
    queue.record_rejected_request(1, 11, "b.flac".to_owned(), Some(6), "disabled".to_owned());

    assert_eq!(queue.entries.len(), 1);
    assert_eq!(queue.entries[0].id, 2);
    assert_eq!(queue.entries[0].filename, "b.flac");
    assert_eq!(queue.entries[0].status, "rejected");

    let _ = std::fs::remove_file(queue.events_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn outbound_peer_dial_order_honors_compatibility_prefer_and_disabled_modes() {
    let compatibility =
        super::AppConfig::from_layers(None, FileConfig::default(), &MapEnv::default())
            .expect("compatibility config");
    assert_eq!(
        super::outbound_peer_dial_order(&compatibility, true, true),
        vec![super::OutboundPeerTransport::Regular]
    );

    let compatibility_with_obfuscation = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSK_OBFUSCATION", "true"),
    )
    .expect("compatibility obfuscation config");
    assert_eq!(
        super::outbound_peer_dial_order(&compatibility_with_obfuscation, false, true),
        vec![super::OutboundPeerTransport::Obfuscated]
    );

    let prefer = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
    )
    .expect("prefer config");
    assert_eq!(
        super::outbound_peer_dial_order(&prefer, true, true),
        vec![
            super::OutboundPeerTransport::Obfuscated,
            super::OutboundPeerTransport::Regular,
        ]
    );

    let disabled = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION", "false"),
    )
    .expect("disabled config");
    assert_eq!(
        super::outbound_peer_dial_order(&disabled, true, true),
        vec![super::OutboundPeerTransport::Regular]
    );

    let prefer_disabled = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSK_OBFUSCATION_MODE", "prefer")
            .with("SLSK_OBFUSCATION_PREFER_OUTBOUND", "false"),
    )
    .expect("prefer disabled config");
    assert_eq!(
        super::outbound_peer_dial_order(&prefer_disabled, true, true),
        vec![super::OutboundPeerTransport::Regular]
    );

    let invalid_frozen = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "false"),
    )
    .expect("frozen compatibility profile accepts the legacy option combination");
    let frozen_error = super::frozen_obfuscation_startup_error(&invalid_frozen)
        .expect("frozen compatibility profile must report its startup limitation");
    assert!(frozen_error.contains("regular peer port must be advertised"));

    let invalid_current = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "false"),
    )
    .expect_err("current behavior must reject hiding the regular port");
    assert!(invalid_current.contains("regular peer port must be advertised"));

    let valid_frozen = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("valid frozen obfuscation options");
    assert!(super::frozen_obfuscation_startup_error(&valid_frozen).is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn pod_join_signatures_match_frozen_unambiguous_canonical_payload() {
    use ed25519_dalek::{Signer as _, SigningKey};

    let now_millis = super::unix_timestamp().saturating_mul(1_000);
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let mut input = super::PodJoinSignatureInput {
        pod_id: "pod-alpha".to_owned(),
        peer_id: "peer".to_owned(),
        requested_role: "member".to_owned(),
        timestamp_unix_ms: now_millis,
        message: "hello world".to_owned(),
        nonce: "nonce-one".to_owned(),
        signature: String::new(),
        public_key: super::STANDARD.encode(signing_key.verifying_key().to_bytes()),
    };
    let signature = signing_key.sign(input.canonical_payload().as_bytes());
    input.signature = format!("ed25519:{}", super::STANDARD.encode(signature.to_bytes()));

    assert!(
        super::verify_pod_join_signature(super::PodSignatureMode::Enforce, &input, now_millis,)
            .unwrap()
    );

    assert_eq!(
        input.canonical_payload(),
        format!(
            "[1,\"join-request\",\"pod-alpha\",\"peer\",\"member\",{now_millis},\"hello world\",\"nonce-one\"]"
        )
    );
    input.message = "hello|world".to_owned();
    let delimiter_signature = signing_key.sign(input.canonical_payload().as_bytes());
    input.signature = format!(
        "ed25519:{}",
        super::STANDARD.encode(delimiter_signature.to_bytes())
    );
    assert!(
        super::verify_pod_join_signature(super::PodSignatureMode::Enforce, &input, now_millis,)
            .unwrap()
    );

    input.message = "hello".to_owned();
    input.nonce = "world|nonce-one".to_owned();
    assert!(
        super::verify_pod_join_signature(super::PodSignatureMode::Enforce, &input, now_millis,)
            .is_err()
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn auto_retry_tracker_prunes_evicted_and_expired_state() {
    let now = super::unix_timestamp();
    let mut queue = super::TransferQueue::new_in_memory(4);
    let live = queue.create(
        0,
        Some("live-peer".to_owned()),
        "Remote/Live.flac".to_owned(),
        None,
        Some(100),
    );
    let mut tracker = super::AutoRetryTracker::default();
    tracker.retried_ids.extend([live.id, live.id + 100]);
    tracker
        .alternate_search_requested_at
        .extend([(live.id, now), (live.id + 100, now)]);
    tracker.retry_counts.extend([
        (super::auto_retry_key("live-peer", &live.filename), 1),
        (super::auto_retry_key("evicted-peer", "Gone.mp3"), 2),
    ]);
    tracker.peer_retry_after.extend([
        ("live-peer".to_owned(), now + 30),
        ("expired".to_owned(), now),
    ]);

    super::prune_auto_retry_tracker(&mut tracker, &queue.entries, now);

    assert_eq!(tracker.retried_ids, HashSet::from([live.id]));
    assert_eq!(tracker.alternate_search_requested_at.len(), 1);
    assert!(tracker.alternate_search_requested_at.contains_key(&live.id));
    assert_eq!(tracker.retry_counts.len(), 1);
    assert!(tracker
        .retry_counts
        .contains_key(&super::auto_retry_key("live-peer", &live.filename)));
    assert_eq!(tracker.peer_retry_after.len(), 1);
    assert_eq!(tracker.peer_retry_after["live-peer"], now + 30);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn auto_retry_plan_is_bounded_latest_only_and_network_friendly() {
    assert!(super::alternate_size_is_eligible(Some(1_000), 1_050, 5.0));
    assert!(!super::alternate_size_is_eligible(Some(1_000), 1_051, 5.0));
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "10")
            .with("SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS", "1")
            .with(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
                "1",
            ),
    )
    .expect("auto retry config");
    let now = super::unix_timestamp();
    let mut queue = super::TransferQueue::new_in_memory(16);
    for (peer, filename) in [
        ("peer-a", "Remote/First.flac"),
        ("peer-a", "Remote/Second.mp3"),
        ("peer-b", "Remote/Notes.txt"),
    ] {
        let entry = queue.create(
            0,
            Some(peer.to_owned()),
            filename.to_owned(),
            None,
            Some(100),
        );
        queue.update_status(entry.id, "failed", None, Some("connection lost".to_owned()));
        queue
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap()
            .updated_at = now - 11;
    }
    let searches = super::SearchStore::new();
    let mut tracker = super::AutoRetryTracker::default();
    let plan = super::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    );
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].source.filename, "Remote/First.flac");

    tracker
        .retry_counts
        .insert(super::auto_retry_key("peer-a", "Remote/First.flac"), 1);
    let next = super::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    );
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].source.filename, "Remote/Second.mp3");
    tracker
        .retry_counts
        .insert(super::auto_retry_key("peer-a", "Remote/Second.mp3"), 1);
    assert!(super::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    )
    .is_empty());

    tracker.retry_counts.clear();
    tracker
        .peer_retry_after
        .insert("peer-a".to_owned(), now + 60);
    assert!(super::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    )
    .is_empty());

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn auto_retry_cycle_creates_a_metadata_preserving_attempt_once() {
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "10")
            .with(
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCES_ENABLED",
                "false",
            ),
        super::SearchStore::new(),
        None,
    );
    let now = super::unix_timestamp();
    let source = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("peer-a".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            Some("/tmp/Song.flac".to_owned()),
            Some(1_000),
            Some("batch-1".to_owned()),
            super::TransferRequestDetails {
                request_id: Some("request-1".to_owned()),
                request_name: Some("Song".to_owned()),
                artist: Some("Artist".to_owned()),
                ..Default::default()
            },
        );
        transfers.update_status(
            entry.id,
            "failed",
            Some(123),
            Some("connection lost".to_owned()),
        );
        let source = transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap();
        source.updated_at = now - 11;
        source.clone()
    };
    let mut tracker = super::AutoRetryTracker::default();

    assert_eq!(
        super::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("auto retry cycle"),
        1
    );
    let command = receiver.recv().await.expect("transfer dispatch");
    let super::SessionCommand::TransferPeer { id, username } = command else {
        panic!("unexpected command: {command:?}");
    };
    assert_eq!(username, "peer-a");
    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == source.id)
        .unwrap();
    let replacement = transfers
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .unwrap();
    assert_eq!(original.status, "failed");
    assert_eq!(replacement.status, "peer_lookup");
    assert_eq!(replacement.request_id.as_deref(), Some("request-1"));
    assert_eq!(replacement.batch_id.as_deref(), Some("batch-1"));
    assert_eq!(replacement.artist.as_deref(), Some("Artist"));
    assert_eq!(replacement.bytes_transferred, 0);
    drop(transfers);

    assert_eq!(
        super::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("deduplicated cycle"),
        0
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn auto_retry_searches_then_uses_a_verified_cached_alternate() {
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "10"),
        super::SearchStore::new(),
        None,
    );
    let now = super::unix_timestamp();
    let source_id = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("peer-a".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            None,
            Some(1_000),
        );
        transfers.update_status(entry.id, "failed", None, Some("timed out".to_owned()));
        transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap()
            .updated_at = now - 11;
        entry.id
    };
    let mut tracker = super::AutoRetryTracker::default();

    assert_eq!(
        super::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("discovery cycle"),
        0
    );
    let super::SessionCommand::Search { token, query, .. } =
        receiver.recv().await.expect("search dispatch")
    else {
        panic!("expected alternate-source search");
    };
    assert_eq!(query, "Song.flac");
    state
        .searches
        .write()
        .await
        .add_peer_response(&FileSearchResponse {
            username: "peer-b".to_owned(),
            token,
            results: vec![FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: "Other/Album/Song.flac".to_owned(),
                size: 1_040,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            }],
            slot_free: true,
            average_speed: 10_000,
            queue_length: 0,
            unknown: 0,
            private_results: Vec::new(),
        })
        .expect("search response");

    assert_eq!(
        super::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("alternate retry cycle"),
        1
    );
    let super::SessionCommand::TransferPeer { id, username } =
        receiver.recv().await.expect("alternate dispatch")
    else {
        panic!("expected transfer dispatch");
    };
    assert_eq!(username, "peer-b");
    let transfers = state.transfers.read().await;
    assert_eq!(
        transfers
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .unwrap()
            .filename,
        "Other/Album/Song.flac"
    );
    assert!(tracker.retried_ids.contains(&source_id));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn rescue_plan_detects_queue_throughput_and_stall_but_skips_sidecars() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "60")
            .with("SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS", "1")
            .with("SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS", "60")
            .with("SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS", "30")
            .with("SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE", "20"),
    )
    .expect("rescue config");
    let now = super::unix_timestamp();
    let mut queue = super::TransferQueue::new_in_memory(16);
    let queued_audio = queue.create(
        0,
        Some("slow-peer".to_owned()),
        "Remote/Queued.flac".to_owned(),
        None,
        Some(100),
    );
    let sidecar = queue.create(
        0,
        Some("slow-peer".to_owned()),
        "Remote/Cover.jpg".to_owned(),
        None,
        Some(100),
    );
    let stalled = queue.create(
        0,
        Some("stalled-peer".to_owned()),
        "Remote/Stalled.mp3".to_owned(),
        None,
        Some(2_000_000),
    );
    for id in [queued_audio.id, sidecar.id] {
        queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .unwrap()
            .requested_at = now - 61;
    }
    let stalled_entry = queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == stalled.id)
        .unwrap();
    stalled_entry.status = "in_progress".to_owned();
    stalled_entry.started_at = Some(now - 61);
    stalled_entry.bytes_transferred = 1_000_000;

    let searches = super::SearchStore::new();
    let mut tracker = super::RescueTracker::default();
    let initial = super::create_rescue_plan(
        &queue.entries,
        &searches,
        &mut tracker,
        &config.transfer_rescue,
        now,
    );
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].source.id, queued_audio.id);
    assert_eq!(
        initial[0].reason,
        super::UnderperformanceReason::QueuedTooLong
    );
    assert!(initial
        .iter()
        .all(|candidate| candidate.source.id != sidecar.id));

    tracker
        .retry_after
        .insert(queued_audio.id, now.saturating_add(300));
    let stalled_plan = super::create_rescue_plan(
        &queue.entries,
        &searches,
        &mut tracker,
        &config.transfer_rescue,
        now + 31,
    );
    assert_eq!(stalled_plan.len(), 1);
    assert_eq!(stalled_plan[0].source.id, stalled.id);
    assert_eq!(
        stalled_plan[0].reason,
        super::UnderperformanceReason::Stalled
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_changes_applies_completed_filter_only_to_initial_snapshot() {
    let (state, _receiver) = test_state();
    {
        let mut transfers = state.transfers.write().await;
        let active = transfers.create(
            0,
            Some("active-peer".to_owned()),
            "Remote/Active.flac".to_owned(),
            None,
            Some(10),
        );
        let completed = transfers.create(
            0,
            Some("completed-peer".to_owned()),
            "Remote/Completed.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_local_execution(completed.id, "succeeded", 10, Some(10), None)
            .expect("complete transfer");
        transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == active.id)
            .expect("active transfer")
            .updated_at_ms = 100;
        transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == completed.id)
            .expect("completed transfer")
            .updated_at_ms = 200;
    }

    let response = super::route_http_request(
        "GET",
        "/api/v0/transfers/changes?since=0&includeCompleted=false",
        None,
        "",
        &state,
    )
    .await
    .expect("transfer changes");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).expect("changes JSON");
    let rows = body["transfers"].as_array().expect("transfer rows");

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row["username"] == "active-peer"));
    assert!(rows.iter().any(|row| row["username"] == "completed-peer"));

    let future_since = 9_999_999_999_999_u64;
    let response = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/changes?since={future_since}"),
        None,
        "",
        &state,
    )
    .await
    .expect("future transfer cursor");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).expect("changes JSON");
    assert!(body["cursor"].as_u64().unwrap() < future_since);
    assert_eq!(body["transfers"].as_array().unwrap().len(), 0);

    let response = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/history?direction=download&asOf={future_since}"),
        None,
        "",
        &state,
    )
    .await
    .expect("future history snapshot");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).expect("history JSON");
    assert_eq!(body["asOf"], future_since);
    assert_eq!(body["transfers"].as_array().unwrap().len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn rescue_cycle_atomically_swaps_an_underperforming_audio_source() {
    let mut searches = super::SearchStore::new();
    let search = searches
        .create(
            None,
            "Song.flac".to_owned(),
            "global",
            None,
            Vec::new(),
            super::DEFAULT_SEARCH_TTL_SECONDS,
        )
        .expect("search")
        .record;
    searches
        .add_peer_response(&FileSearchResponse {
            username: "rescue-peer".to_owned(),
            token: search.token,
            results: vec![FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: "Alternate/Album/Song.flac".to_owned(),
                size: 1_020,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            }],
            slot_free: true,
            average_speed: 100_000,
            queue_length: 0,
            unknown: 0,
            private_results: Vec::new(),
        })
        .expect("search response");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "60")
            .with(
                "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                "5",
            ),
        searches,
        None,
    );
    let now = super::unix_timestamp();
    let source = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("original-peer".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            Some("downloads/Song.flac".to_owned()),
            Some(1_000),
            Some("batch-rescue".to_owned()),
            super::TransferRequestDetails {
                request_id: Some("request-rescue".to_owned()),
                artist: Some("Artist".to_owned()),
                ..Default::default()
            },
        );
        let source = transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap();
        source.requested_at = now - 61;
        source.clone()
    };
    let mut tracker = super::RescueTracker::default();

    assert_eq!(
        super::run_transfer_rescue_cycle(&state, &mut tracker)
            .await
            .expect("rescue cycle"),
        1
    );
    let super::SessionCommand::TransferPeer { id, username } =
        receiver.recv().await.expect("rescue dispatch")
    else {
        panic!("expected rescue transfer dispatch");
    };
    assert_eq!(username, "rescue-peer");
    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == source.id)
        .unwrap();
    let replacement = transfers
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .unwrap();
    assert_eq!(original.status, "cancelled");
    assert_eq!(replacement.status, "peer_lookup");
    assert_eq!(replacement.peer_username.as_deref(), Some("rescue-peer"));
    assert_eq!(replacement.request_id.as_deref(), Some("request-rescue"));
    assert_eq!(replacement.batch_id.as_deref(), Some("batch-rescue"));
    assert_eq!(replacement.artist.as_deref(), Some("Artist"));
    assert_eq!(replacement.local_path, source.local_path);
    drop(transfers);
    assert!(super::transfer_is_cancelled(&state, source.id).await);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn rescue_cycle_promotes_verified_mesh_swarm_without_overwriting_partial_file() {
    use sha2::{Digest, Sha256};

    let content = Arc::new(b"verified automatic rescue swarm".to_vec());
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));
    let (source_a, task_a) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let (source_b, task_b) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "60"),
        super::SearchStore::new(),
        None,
    );
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: super::content_discovery::generate_flac_key(
                    "Remote/Album/Song.flac",
                    content.len() as u64,
                ),
                size: content.len() as u64,
                full_file_hash: expected_hash.clone(),
                music_brainz_id: "recording-rescue".to_owned(),
                ..Default::default()
            }])
            .expect("merge rescue hash");
        discovery
            .merge_shadow_records(vec![super::content_discovery::ShadowIndexRecord {
                recording_id: "recording-rescue".to_owned(),
                peer_ids: vec!["rescue-peer-a".to_owned(), "rescue-peer-b".to_owned()],
                updated_at: 0,
            }])
            .expect("merge rescue shadow peers");
    }
    {
        let mut mesh = state.mesh.write().await;
        for (username, peer_id, address) in [
            ("mesh-a", "rescue-peer-a", source_a),
            ("mesh-b", "rescue-peer-b", source_b),
        ] {
            let mut descriptor = test_capability_descriptor(
                username,
                vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
            );
            descriptor.peer_id = peer_id.to_owned();
            descriptor.endpoints = vec![format!("http://{address}/content")];
            mesh.capability_records.push(descriptor);
        }
    }

    let partial_path = state.config.downloads_dir.join("Song.partial");
    std::fs::create_dir_all(partial_path.parent().unwrap()).expect("create download root");
    std::fs::write(&partial_path, b"keep partial until verified").expect("write partial");
    let now = super::unix_timestamp();
    let source = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("slow-peer".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            Some(partial_path.display().to_string()),
            Some(content.len() as u64),
            Some("batch-mesh-rescue".to_owned()),
            super::TransferRequestDetails {
                request_id: Some("request-mesh-rescue".to_owned()),
                artist: Some("Artist".to_owned()),
                ..Default::default()
            },
        );
        let source = transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap();
        source.requested_at = now - 61;
        source.clone()
    };
    let mut tracker = super::RescueTracker::default();

    assert_eq!(
        super::run_transfer_rescue_cycle(&state, &mut tracker)
            .await
            .expect("mesh rescue cycle"),
        1
    );
    assert!(receiver.try_recv().is_err());
    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == source.id)
        .expect("original transfer");
    let replacement = transfers
        .entries
        .iter()
        .find(|entry| entry.id != source.id)
        .expect("mesh replacement");
    assert_eq!(original.status, "cancelled");
    assert_eq!(replacement.status, "succeeded");
    assert_eq!(replacement.peer_username.as_deref(), Some("mesh-swarm"));
    assert_eq!(
        replacement.request_id.as_deref(),
        Some("request-mesh-rescue")
    );
    assert_eq!(replacement.batch_id.as_deref(), Some("batch-mesh-rescue"));
    assert_eq!(replacement.artist.as_deref(), Some("Artist"));
    assert_eq!(replacement.bytes_transferred, content.len() as u64);
    let replacement_path = replacement.local_path.clone().expect("rescue output path");
    assert_ne!(replacement_path, partial_path.display().to_string());
    drop(transfers);
    assert_eq!(
        std::fs::read(replacement_path).expect("read verified rescue output"),
        content.as_slice()
    );
    assert_eq!(
        std::fs::read(&partial_path).expect("read untouched partial"),
        b"keep partial until verified"
    );
    let jobs = state.multisource.read().await;
    assert_eq!(jobs.list().len(), 1);
    assert_eq!(jobs.list()[0].status, "completed");
    drop(jobs);

    task_a.abort();
    task_b.abort();
    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn cancelled_transfer_progress_cannot_revive_or_advance_the_attempt() {
    let (state, _receiver) =
        test_state_with_env_parts(MapEnv::default(), super::SearchStore::new(), None);
    let transfer = {
        let mut transfers = state.transfers.write().await;
        let transfer = transfers.create(
            0,
            Some("slow-peer".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some("downloads/Song.flac".to_owned()),
            Some(1_000),
        );
        transfers.update_status(transfer.id, "in_progress", Some(400), None);
        transfers.update_status(transfer.id, "cancelled", Some(400), None)
    }
    .expect("cancelled transfer");

    super::update_transfer_progress(&state, transfer.id, 900).await;

    let transfers = state.transfers.read().await;
    let current = transfers
        .entries
        .iter()
        .find(|entry| entry.id == transfer.id)
        .expect("transfer remains projected");
    assert_eq!(current.status, "cancelled");
    assert_eq!(current.bytes_transferred, 400);
    drop(transfers);

    let path = std::env::temp_dir().join(format!(
        "slskr-cancelled-chunk-{}-{}",
        std::process::id(),
        transfer.id
    ));
    let _ = std::fs::remove_file(&path);
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .expect("create cancelled transfer test file");
    let error = super::write_download_chunk_if_active(
        &state,
        transfer.id,
        &mut file,
        b"must-not-be-written",
        900,
    )
    .await
    .expect_err("cancelled download must reject a received chunk");
    assert_eq!(error, "transfer cancelled");
    assert_eq!(file.metadata().expect("test file metadata").len(), 0);
    drop(file);
    std::fs::remove_file(path).expect("remove cancelled transfer test file");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn auto_retry_rollback_preserves_concurrent_transfer_allocations() {
    let mut queue = super::TransferQueue::new_in_memory(16);
    let previous_next_id = queue.next_id;
    let previous_next_token = queue.next_token;
    let replacement = queue.create(
        0,
        Some("retry-peer".to_owned()),
        "Remote/Retry.flac".to_owned(),
        None,
        Some(100),
    );
    let replacement_next_id = queue.next_id;
    let replacement_next_token = queue.next_token;
    let concurrent = queue.create(
        0,
        Some("other-peer".to_owned()),
        "Remote/Other.flac".to_owned(),
        None,
        Some(200),
    );
    let concurrent_next_id = queue.next_id;
    let concurrent_next_token = queue.next_token;

    super::transfer_recovery_runtime::rollback_auto_retry_replacement(
        &mut queue,
        &replacement,
        previous_next_id,
        previous_next_token,
        replacement_next_id,
        replacement_next_token,
    );

    assert!(queue.entries.iter().all(|entry| entry.id != replacement.id));
    assert!(queue.entries.iter().any(|entry| entry.id == concurrent.id));
    assert_eq!(queue.next_id, concurrent_next_id);
    assert_eq!(queue.next_token, concurrent_next_token);
    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_queue_bounds_retained_text_and_normalizes_lookups() {
    let oversized_username = "é".repeat(super::MAX_TRANSFER_USERNAME_BYTES);
    let oversized_filename = "f".repeat(super::MAX_TRANSFER_FILENAME_BYTES + 1);
    let oversized_path = "p".repeat(super::MAX_TRANSFER_LOCAL_PATH_BYTES + 1);
    let oversized_batch = "b".repeat(super::MAX_TRANSFER_BATCH_ID_BYTES + 1);
    let oversized_reason = "r".repeat(super::MAX_TRANSFER_REASON_BYTES + 1);
    let mut queue = super::TransferQueue::new_in_memory(8);

    let entry = queue.create_with_batch(
        0,
        Some(oversized_username.clone()),
        oversized_filename,
        Some(oversized_path),
        Some(100),
        Some(oversized_batch),
    );
    assert!(entry.peer_username.unwrap().len() <= super::MAX_TRANSFER_USERNAME_BYTES);
    assert_eq!(entry.filename.len(), super::MAX_TRANSFER_FILENAME_BYTES);
    assert!(entry.local_path.is_none());
    assert_eq!(
        entry.batch_id.unwrap().len(),
        super::MAX_TRANSFER_BATCH_ID_BYTES
    );

    let updated = queue
        .update_status(entry.id, "peer_lookup", None, Some(oversized_reason))
        .unwrap();
    assert_eq!(
        updated.reason.unwrap().len(),
        super::MAX_TRANSFER_REASON_BYTES
    );
    assert!(queue.pending_peer_transfer(&oversized_username).is_some());

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_queue_records_progress_events_with_bytes() {
    let mut queue = super::TransferQueue::new_in_memory(8);
    let entry = queue.create(1, None, "Remote/Song.flac".to_owned(), None, Some(100));

    queue.update_status(entry.id, "in_progress", None, None);
    queue.update_progress(entry.id, 64);
    queue.update_local_execution(entry.id, "succeeded", 100, Some(100), None);

    let events = std::fs::read_to_string(&queue.events_path).expect("events");
    assert!(events.starts_with("slskr-transfer-events-v2\n"));
    assert!(events.contains("bytes_transferred"));
    assert!(events.contains("\t64\tin_progress\t\tRemote/Song.flac"));
    assert!(events.contains("\t100\tsucceeded\t\tRemote/Song.flac"));

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_projection_reports_elapsed_speed_and_remaining_time() {
    let mut queue = super::TransferQueue::new_in_memory(8);
    let entry = queue.create(
        0,
        Some("friend".to_owned()),
        "Remote/Song.flac".to_owned(),
        None,
        Some(325),
    );
    let entry = queue
        .entries
        .iter_mut()
        .find(|candidate| candidate.id == entry.id)
        .unwrap();
    entry.status = "succeeded".to_owned();
    entry.requested_at = 90;
    entry.started_at = Some(100);
    entry.start_offset = 25;
    entry.bytes_transferred = 225;
    entry.updated_at = 104;

    let projection = entry.controller_file_json();
    assert_eq!(projection["startOffset"], 25);
    assert_eq!(projection["startedAt"], "100");
    assert_eq!(projection["endedAt"], "104");
    assert_eq!(projection["averageSpeed"], 50.0);
    assert_eq!(projection["elapsedTime"], "00:00:04");
    assert_eq!(projection["remainingTime"], "");

    entry.status = "in_progress".to_owned();
    entry.bytes_transferred = 500;
    let overrun = entry.controller_file_json();
    assert_eq!(overrun["percentComplete"], 100.0);
    assert_eq!(overrun["bytesRemaining"], 0);
    assert_eq!(overrun["remainingTime"], "");

    let encoded = serde_json::to_value(&*entry).unwrap();
    let mut legacy = encoded.as_object().unwrap().clone();
    legacy.remove("started_at");
    legacy.remove("start_offset");
    let decoded = serde_json::from_value::<super::TransferEntry>(legacy.into()).unwrap();
    assert_eq!(decoded.started_at, None);
    assert_eq!(decoded.start_offset, 0);

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_reports_aggregate_measured_speeds() {
    let mut queue = super::TransferQueue::new_in_memory(8);
    for (direction, username, bytes, started_at, updated_at) in [
        (0, "friend", 200, 100, 104),
        (0, "friend", 300, 100, 103),
        (1, "uploader", 120, 100, 104),
    ] {
        let created = queue.create(
            direction,
            Some(username.to_owned()),
            format!("Remote/{bytes}.flac"),
            None,
            Some(bytes),
        );
        let entry = queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == created.id)
            .unwrap();
        entry.status = "succeeded".to_owned();
        entry.started_at = Some(started_at);
        entry.bytes_transferred = bytes;
        entry.updated_at = updated_at;
    }

    let summary = serde_json::from_str::<serde_json::Value>(
        &super::controller_transfer_summary_report(Some("direction=Download"), &queue),
    )
    .unwrap();
    assert_eq!(summary["averageSpeed"], 75.0);

    let leaderboard = serde_json::from_str::<serde_json::Value>(
        &super::controller_transfer_leaderboard_report(Some("direction=Download"), &queue)
            .expect("direction is present"),
    )
    .unwrap();
    assert_eq!(leaderboard[0]["username"], "friend");
    assert_eq!(leaderboard[0]["averageSpeed"], 75.0);

    let now = super::unix_timestamp();
    for entry in &mut queue.entries {
        entry.status = "in_progress".to_owned();
        entry.started_at = Some(now.saturating_sub(4));
        entry.start_offset = 0;
    }
    let speeds =
        serde_json::from_str::<serde_json::Value>(&super::controller_transfer_speeds_json(&queue))
            .unwrap();
    assert!(speeds["download"].as_f64().unwrap() > 0.0);
    assert!(speeds["upload"].as_f64().unwrap() > 0.0);
    assert_eq!(speeds["download"], speeds["downloadSpeed"]);
    assert_eq!(speeds["upload"], speeds["uploadSpeed"]);
    assert_eq!(speeds["total"], speeds["soulseek"]);
    assert_eq!(speeds["mesh"], 0.0);
    assert_eq!(speeds["sessionBytesDownloaded"], 500);
    assert_eq!(speeds["sessionBytesUploaded"], 120);
    assert_eq!(speeds["sessionBytesTotal"], 620);

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_leaderboard_excludes_incomplete_transfers_not_hardcoded_status() {
    let mut queue = super::TransferQueue::new_in_memory(8);
    // A failed download must never count toward the leaderboard --
    // matching the oracle's GetTransferLeaderboard, which filters
    // State = 48 (Completed | Succeeded) at the SQL layer.
    let failed = queue.create(
        0,
        Some("flaky".to_owned()),
        "Remote/Flaky.flac".to_owned(),
        None,
        Some(9999),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == failed.id)
        .unwrap()
        .status = "failed".to_owned();
    let succeeded = queue.create(
        0,
        Some("reliable".to_owned()),
        "Remote/Reliable.flac".to_owned(),
        None,
        Some(50),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == succeeded.id)
        .unwrap()
        .status = "succeeded".to_owned();

    let leaderboard = serde_json::from_str::<serde_json::Value>(
        &super::controller_transfer_leaderboard_report(Some("direction=Download"), &queue)
            .expect("direction is present"),
    )
    .unwrap();
    let rows = leaderboard.as_array().unwrap();
    assert!(
        rows.iter().all(|row| row["username"] != "flaky"),
        "{leaderboard}"
    );
    assert!(
        rows.iter().any(|row| row["username"] == "reliable"),
        "{leaderboard}"
    );

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_directories_report_is_completed_uploads_only() {
    let mut queue = super::TransferQueue::new_in_memory(8);
    // Matches the oracle's GetTransferDirectoryFrequency exactly: it
    // always reports on Uploads (what others downloaded from local
    // shares) and only completed ones -- never downloads, and never
    // queued/failed transfers.
    let download = queue.create(
        0,
        Some("downloader".to_owned()),
        "Shared/DownloadOnly/File.flac".to_owned(),
        None,
        Some(10),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == download.id)
        .unwrap()
        .status = "succeeded".to_owned();
    let incomplete_upload = queue.create(
        1,
        Some("stalled-peer".to_owned()),
        "Shared/StillQueued/File.flac".to_owned(),
        None,
        Some(10),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == incomplete_upload.id)
        .unwrap()
        .status = "queued".to_owned();
    let completed_upload = queue.create(
        1,
        Some("real-uploader".to_owned()),
        "Shared/Popular/File.flac".to_owned(),
        None,
        Some(10),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == completed_upload.id)
        .unwrap()
        .status = "succeeded".to_owned();

    let report = serde_json::from_str::<serde_json::Value>(
        &super::controller_transfer_directories_report(None, &queue),
    )
    .unwrap();
    let paths = report
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["path"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["Shared/Popular"], "{report}");

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_queue_persists_and_reloads_resume_state() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-state-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_AUTO_CONNECT", "false");
    let config = super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    {
        let mut queue = super::TransferQueue::new(&config);
        let entry = queue.create(
            0,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some(state_dir.join("Song.flac").display().to_string()),
            Some(100),
        );
        queue.update_status(entry.id, "in_progress", Some(40), None);
    }

    let events_path = super::transfer_events_path(&state_dir);
    let events_before_restart = std::fs::read_to_string(&events_path).expect("transfer events");
    assert!(events_before_restart.contains("\tqueued\t"));
    assert!(events_before_restart.contains("\tin_progress\t"));
    assert_eq!(events_before_restart.lines().count(), 4);

    let reloaded = super::TransferQueue::new(&config);
    let events_after_restart = std::fs::read_to_string(&events_path).expect("reloaded events");
    assert_eq!(events_after_restart, events_before_restart);
    let entry = reloaded.entries.first().expect("reloaded transfer");
    assert_eq!(entry.status, "queued");
    assert_eq!(entry.bytes_transferred, 40);
    assert_eq!(entry.reason.as_deref(), Some("resumed after restart"));
    assert_eq!(entry.started_at, None);
    assert_eq!(entry.start_offset, 40);
    assert_eq!(reloaded.next_id, 2);
    assert_eq!(reloaded.next_token, 2);

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_queue_rehydrates_from_sqlite_on_startup() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-sqlite-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_AUTO_CONNECT", "false")
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let config = super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    let db_path = state_dir.join("slskr.db");
    let db = crate::persistence::DatabaseManager::new(db_path.to_str().unwrap_or("slskr.db"))
        .await
        .expect("database");

    let record = crate::persistence::TransferRecord {
        id: "42".to_owned(),
        direction: "download".to_owned(),
        filename: "Remote/SQLite.flac".to_owned(),
        peer_username: "sqlite-peer".to_owned(),
        filesize: 2048,
        progress: 512,
        status: "queued".to_owned(),
        started_at: 1000,
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
    db.insert_transfer(&record).await.expect("insert transfer");

    let mut queue = super::TransferQueue::new(&config);
    assert!(queue.entries.is_empty(), "JSON queue should be empty");

    queue.rehydrate_from_database(&db).await;

    assert_eq!(
        queue.entries.len(),
        1,
        "should have rehydrated one transfer"
    );
    let entry = &queue.entries[0];
    assert_eq!(entry.id, 42);
    assert_eq!(entry.direction, 0);
    assert_eq!(entry.peer_username.as_deref(), Some("sqlite-peer"));
    assert_eq!(entry.filename, "Remote/SQLite.flac");
    assert_eq!(entry.size, Some(2048));
    assert_eq!(entry.bytes_transferred, 512);
    assert_eq!(entry.status, "queued");
    assert_eq!(queue.next_id, 43);
    assert_eq!(queue.next_token, 43);

    drop(db);
    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn dispatch_queued_downloads_after_login_sends_transfer_peer_commands() {
    let (state, mut receiver) = test_state();

    // Create a queued download
    {
        let mut queue = state.transfers.write().await;
        queue.create(
            0,
            Some("test-peer".to_owned()),
            "Remote/Test.flac".to_owned(),
            None,
            Some(1024),
        );
    }

    // Call dispatch function
    super::session_runtime::dispatch_queued_downloads_after_login(&state).await;

    // Verify TransferPeer command was sent
    let command = receiver.recv().await.expect("should receive command");
    match command {
        super::SessionCommand::TransferPeer { id, username } => {
            assert_eq!(id, 1);
            assert_eq!(username, "test-peer");
        }
        _ => panic!("Expected TransferPeer command, got {:?}", command),
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_state_loader_rejects_oversized_state_file() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-state-size-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let state_path = super::transfer_state_path(&state_dir);
    std::fs::write(
        &state_path,
        vec![b' '; (super::MAX_TRANSFER_STATE_BYTES as usize) + 1],
    )
    .expect("oversized state");

    let error = super::load_transfer_state(&state_path, 100).expect_err("oversized state");
    assert!(error.contains("transfer state file is too large"));

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_state_loader_rejects_fifo_without_blocking() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-state-fifo-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let state_path = super::transfer_state_path(&state_dir);
    let status = std::process::Command::new("mkfifo")
        .arg(&state_path)
        .status()
        .expect("run mkfifo");
    assert!(status.success());

    let error = super::load_transfer_state(&state_path, 100)
        .expect_err("FIFO transfer state path must be rejected");
    assert!(error.contains("must be a regular file"));

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn state_file_io_rejects_symlinks_without_touching_targets() {
    use std::os::unix::fs::symlink;

    let state_dir = std::env::temp_dir().join(format!(
        "slskr-state-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let target = state_dir.join("target.json");
    std::fs::write(&target, "keep").expect("target");

    let state_path = super::transfer_state_path(&state_dir);
    symlink(&target, &state_path).expect("state symlink");
    let error = super::load_transfer_state(&state_path, 100).expect_err("reject state symlink");
    assert!(error.contains("transfer state open failed"));

    let destination = state_dir.join("destination.json");
    let temporary = state_dir.join("temporary.json");
    symlink(&target, &temporary).expect("temporary symlink");
    let error =
        super::write_file_atomic_with_temp_path(&destination, &temporary, br#"{"version":1}"#)
            .expect_err("reject occupied temporary path");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        std::fs::read_to_string(&target).expect("target unchanged"),
        "keep"
    );
    assert!(!destination.exists());

    let events_path = super::transfer_events_path(&state_dir);
    symlink(&target, &events_path).expect("event symlink");
    let entry = super::TransferEntry {
        id: 1,
        direction: 0,
        token: 1,
        peer_username: None,
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
        size: Some(1),
        bytes_transferred: 0,
        status: "queued".to_owned(),
        reason: None,
        requested_at: 1,
        started_at: None,
        start_offset: 0,
        updated_at: 1,
        updated_at_ms: 1_000,
        previous_status: None,
    };
    let error =
        super::append_transfer_event(&events_path, &entry).expect_err("reject event symlink");
    assert!(error.contains("must be a regular file"));
    assert_eq!(
        std::fs::read_to_string(&target).expect("target still unchanged"),
        "keep"
    );

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn atomic_state_writer_replaces_existing_file() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-state-replace-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let destination = state_dir.join("state.json");

    super::write_file_atomic(&destination, b"first").expect("first state write");
    super::write_file_atomic(&destination, b"second").expect("replacement state write");

    assert_eq!(std::fs::read(&destination).expect("read state"), b"second");
    assert_eq!(
        std::fs::read_dir(&state_dir)
            .expect("list state dir")
            .count(),
        1
    );
    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-file-lifecycle-tests"
))]
fn file_lifecycle_differential_atomic_file_writer_common_cases() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-atomic-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&state_dir).expect("file lifecycle state dir");
    let destination = state_dir.join("state.bin");

    super::write_file_atomic(&destination, b"first").expect("nominal atomic write");
    assert_eq!(
        fs::read(&destination).expect("read nominal atomic write"),
        b"first"
    );
    assert_eq!(
        fs::metadata(&destination).expect("atomic metadata").len(),
        5
    );

    super::write_file_atomic(&destination, b"second").expect("atomic overwrite");
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
    super::write_file_atomic(&nested_destination, b"nested")
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
        super::write_file_atomic(&linked_destination, b"replacement")
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
    let partial_write = super::write_file_atomic(&blocked_parent.join("state.bin"), b"partial");
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
