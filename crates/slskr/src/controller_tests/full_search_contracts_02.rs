//! Controller full search contracts 02 ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_auto_retry_configuration_changes_the_live_retry_cycle_without_restart()
{
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let now = crate::unix_timestamp();
    {
        let mut transfers = state.transfers.write().await;
        let source = transfers.create(
            0,
            Some("watched-peer".to_owned()),
            "Remote/Watched.flac".to_owned(),
            None,
            Some(1_000),
        );
        transfers.update_status(source.id, "failed", None, Some("timed out".to_owned()));
        transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == source.id)
            .unwrap()
            .updated_at = now - 11;
    }
    let mut tracker = crate::AutoRetryTracker::default();
    assert_eq!(
        crate::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .unwrap(),
        0,
        "the startup 1800-second delay must not retry an 11-second-old failure"
    );

    let yaml = "transfers:\n  download:\n    auto_retry:\n      enabled: true\n      retry_delay_seconds: 10\n      check_interval_seconds: 20\n      max_attempts: 7\n      max_files_per_cycle: 8\n      max_files_per_peer_per_cycle: 2\n      peer_cooldown_seconds: 60\n      alternate_sources_enabled: false\n      max_alternate_source_searches_per_cycle: 2\n      alternate_source_size_tolerance_percent: 5.5\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    let settings = state.transfer_auto_retry_settings.read().await.clone();
    assert!(settings.enabled);
    assert_eq!(settings.retry_delay.as_secs(), 10);
    assert_eq!(settings.check_interval.as_secs(), 20);
    assert_eq!(settings.max_attempts, 7);
    assert_eq!(settings.max_files_per_cycle, 8);
    assert_eq!(settings.max_files_per_peer_per_cycle, 2);
    assert_eq!(settings.peer_cooldown.as_secs(), 60);
    assert!(!settings.alternate_sources_enabled);
    assert_eq!(settings.max_alternate_source_searches_per_cycle, 2);
    assert_eq!(settings.alternate_source_size_tolerance_percent, 5.5);
    assert!(!state.runtime.read().await.application_restart_requested);

    assert_eq!(
        crate::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .unwrap(),
        1
    );
    assert!(matches!(
        receiver.recv().await,
        Some(crate::SessionCommand::TransferPeer { username, .. }) if username == "watched-peer"
    ));

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(
        current["global"]["download"]["autoRetry"]["alternateSourceSizeTolerancePercent"],
        5.5
    );
    assert_eq!(
        startup["global"]["download"]["autoRetry"]["alternateSourceSizeTolerancePercent"],
        5
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_share_configuration_marks_pending_and_rescan_uses_new_roots() {
    let (state, _receiver) = test_state();
    let new_root = state.config.state_dir.join("watched-share");
    let new_downloads = state.config.state_dir.join("watched-downloads");
    let new_incomplete = state.config.state_dir.join("watched-incomplete");
    fs::create_dir_all(&new_root).unwrap();
    fs::create_dir_all(&new_downloads).unwrap();
    fs::create_dir_all(&new_incomplete).unwrap();
    fs::write(new_root.join("new.flac"), b"new").unwrap();
    fs::write(new_downloads.join("guarded.flac"), b"guarded").unwrap();
    let denied = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/Z3VhcmRlZC5mbGFj",
        None,
        "",
        &state,
    )
    .await
    .expect("remote file management disabled response");
    assert_eq!(denied.status, "403 Forbidden");
    let yaml = format!(
        "remote_configuration: true\nremote_file_management: true\ndirectories:\n  downloads: '{}'\n  incomplete: '{}'\nshares:\n  directories:\n    - '[Watched]{}'\n",
        new_downloads.display(),
        new_incomplete.display(),
        new_root.display(),
    );
    fs::write(state.config.state_dir.join("slskd.yml"), &yaml).unwrap();
    let mut cli_environment = state.controller_cli_environment.clone();
    cli_environment.remove("SLSKR_SHARE_FIXTURE");

    crate::apply_watched_controller_configuration(&state, Some(&yaml), &cli_environment).await;

    assert_eq!(
        state.share_settings.read().await.directories[0].alias,
        "Watched"
    );
    let shares = state.shares.read().await;
    assert_eq!(shares.roots[0].label, "Watched");
    assert!(!shares.roots[0].statistics_ready);
    drop(shares);
    assert!(state.share_lifecycle.read().await.scan_pending);
    assert!(state.runtime.read().await.application_restart_requested);
    assert_eq!(crate::effective_downloads_dir(&state), new_downloads);
    assert_eq!(crate::effective_incomplete_dir(&state), new_incomplete);
    assert!(crate::effective_remote_file_management(&state));
    assert!(crate::effective_remote_configuration(&state));
    {
        let overlay = state.options_overlay.read().await;
        let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
            &state.config,
            &overlay,
            true,
        ))
        .unwrap();
        let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
            &state.config,
            &overlay,
            false,
        ))
        .unwrap();
        assert_eq!(current["remoteConfiguration"], true);
        assert_eq!(startup["remoteConfiguration"], false);
    }
    let yaml_response = crate::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
        .await
        .expect("watched remote configuration response");
    assert_eq!(yaml_response.status, "200 OK");
    let deleted = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/Z3VhcmRlZC5mbGFj",
        None,
        "",
        &state,
    )
    .await
    .expect("watched remote file management response");
    assert_eq!(deleted.status, "204 No Content");
    assert!(!new_downloads.join("guarded.flac").exists());
    let local_path = crate::prepare_transfer_local_path(
        &state,
        0,
        Some("peer"),
        "Remote/Song.flac",
        None,
        &crate::TransferRequestDetails::default(),
        None,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(Path::new(&local_path).starts_with(crate::effective_downloads_dir(&state)));
    let overlay = state.options_overlay.read().await;
    let options = crate::controller_options_json(&state.config, &overlay, true);
    drop(overlay);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&options).unwrap()["shares"]["directories"][0],
        format!("[Watched]{}", new_root.display())
    );

    let snapshot = crate::rebuild_share_index(&state).await.unwrap();
    assert_eq!(snapshot.entries[0].filename, "Watched/new.flac");
    assert!(snapshot.roots[0].statistics_ready);
    assert!(!state.share_lifecycle.read().await.scan_pending);

    let disabled_yaml = yaml.replacen(
        "remote_configuration: true",
        "remote_configuration: false",
        1,
    );
    fs::write(state.config.state_dir.join("slskd.yml"), &disabled_yaml).unwrap();
    crate::apply_watched_controller_configuration(&state, Some(&disabled_yaml), &cli_environment)
        .await;
    assert!(!crate::effective_remote_configuration(&state));
    let forbidden = crate::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
        .await
        .expect("self-disabled remote configuration response");
    assert_eq!(forbidden.status, "403 Forbidden");
    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(current["remoteConfiguration"], false);
    assert_eq!(startup["remoteConfiguration"], false);
}
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_instance_name_uses_frozen_string_binding_and_restart_lifecycle() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let cli_environment = BTreeMap::from([(
        "SLSKR_STATE_DIR".to_owned(),
        state.config.state_dir.display().to_string(),
    )]);

    let numeric_yaml = "instance_name: 123\nflags:\n  no_connect: true\n";
    fs::write(state.config.state_dir.join("slskd.yml"), numeric_yaml).unwrap();
    crate::apply_watched_controller_configuration(&state, Some(numeric_yaml), &cli_environment)
        .await;

    {
        let overlay = state.options_overlay.read().await;
        let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
            &state.config,
            &overlay,
            true,
        ))
        .unwrap();
        let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
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
    crate::apply_watched_controller_configuration(&state, Some(empty_yaml), &cli_environment).await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    assert_eq!(current["instanceName"], "default");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_completed_path_template_updates_projection_and_runtime_without_restart()
{
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

    crate::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    assert_eq!(
        crate::effective_download_completed_path_template(&state),
        "{uploader}/{remote_folder}"
    );
    assert_eq!(
        crate::file_transfer_runtime::render_completed_download_path(
            &crate::effective_download_completed_path_template(&state),
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
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
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
    let debug = crate::controller_options_debug_view(&state, &overlay);
    assert!(debug.contains(
        "completedpathtemplate={uploader}/{remote_folder} (YamlConfigurationProvider for 'slskd.yml' (Optional))"
    ));
    assert!(!state.runtime.read().await.application_restart_requested);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_private_message_auto_response_updates_actual_outbound_behavior() {
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
    crate::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

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
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn incoming_search_response_honors_configured_file_limit() {
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT",
        "100",
    ));
    state.shares.write().await.entries = (0..150)
        .map(|index| crate::FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: format!("Library/fixture-{index}.flac"),
            size: 1,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        })
        .collect();
    let response = crate::build_file_search_response(&state, 1, "fixture")
        .await
        .expect("matching response");
    assert_eq!(response.results.len(), 100);
}
