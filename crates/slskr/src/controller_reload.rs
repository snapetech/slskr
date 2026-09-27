use super::*;

pub(super) fn controller_compatibility_config_path(config: &AppConfig) -> PathBuf {
    config.state_dir.join("slskd.yml")
}

pub(super) fn controller_options_config_location_json(config: &AppConfig) -> String {
    serde_json::Value::String(
        controller_compatibility_config_path(config)
            .display()
            .to_string(),
    )
    .to_string()
}

pub(super) fn read_controller_compatibility_yaml(
    config: &AppConfig,
) -> Result<Option<String>, String> {
    use std::io::Read;

    let path = controller_compatibility_config_path(config);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("configuration file metadata failed: {error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("configuration file must be a regular file".to_owned());
    }
    if metadata.len() > MAX_CONTROLLER_YAML_BYTES as u64 {
        return Err("configuration file is too large".to_owned());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(&path)
        .map_err(|error| format!("configuration file read failed: {error}"))?;
    let mut text = String::new();
    file.by_ref()
        .take(MAX_CONTROLLER_YAML_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|error| format!("configuration file read failed: {error}"))?;
    if text.len() > MAX_CONTROLLER_YAML_BYTES {
        return Err("configuration file is too large".to_owned());
    }
    Ok(Some(text))
}

pub(super) fn spawn_controller_config_watcher(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let mut previous = read_controller_compatibility_yaml(&state.config)
            .ok()
            .flatten();
        let mut interval = time::interval(Duration::from_millis(200));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let current = match read_controller_compatibility_yaml(&state.config) {
                Ok(current) => current,
                Err(error) => {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "configuration",
                        format!("configuration watch read failed: {error}"),
                    )
                    .await;
                    continue;
                }
            };
            if current == previous {
                continue;
            }
            previous = current.clone();
            apply_watched_controller_configuration(
                &state,
                current.as_deref(),
                &state.controller_cli_environment,
            )
            .await;
        }
    });
}

pub(super) fn load_watched_controller_configuration(
    cli_environment: BTreeMap<String, String>,
) -> Result<Box<AppConfig>, String> {
    std::thread::Builder::new()
        .name("slskr-config-reload".to_owned())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let (config_file, file_config) = config::load_file_config()?;
            AppConfig::from_layers(
                config_file,
                file_config,
                &ControllerCliEnv {
                    values: &cli_environment,
                },
            )
            .map(Box::new)
        })
        .map_err(|error| format!("failed to start configuration reload: {error}"))?
        .join()
        .map_err(|_| "configuration reload panicked".to_owned())?
}

pub(super) fn apply_watched_controller_configuration<'a>(
    state: &'a AppState,
    text: Option<&'a str>,
    cli_environment: &'a BTreeMap<String, String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        let parsed = text.and_then(|text| parse_controller_yaml(text).ok());
        let reloaded = match load_watched_controller_configuration(cli_environment.clone()) {
            Ok(config) => config,
            Err(error) => {
                let exposed_validation_error = parsed.as_ref().and_then(|value| {
                    controller_yaml_target_validation_error(value, state.config.controller_profile)
                });
                if text.is_some() && parsed.is_none() {
                    // A syntactically invalid watched document must never leave
                    // stale configuration visible through the current options
                    // projection. Keep the already-loaded runtime/share index
                    // until the next valid reload, but make the invalid state
                    // explicit to API consumers for every controller profile.
                    let mut overlay = state.options_overlay.write().await;
                    overlay.watched_yaml_effective = Some(serde_json::Value::Null);
                    overlay.watched_share_directories = Some(Vec::new());
                    overlay.watched_instance_name = Some(state.config.instance_name.clone());
                    overlay.watched_controller_swagger = Some(state.config.controller_swagger);
                    overlay.watched_dht = Some(state.config.advanced_networking.dht.clone());
                }
                record_daemon_log(
                    state,
                    logging::LogLevel::Warn,
                    "configuration",
                    format!("configuration watch validation failed: {error}"),
                )
                .await;
                *state
                    .controller_options_validation_error
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = exposed_validation_error;
                return;
            }
        };
        let previous_obfuscation = state
            .options_overlay
            .read()
            .await
            .watched_obfuscation
            .unwrap_or_else(|| ObfuscationReloadState::from_config(&state.config));
        let reloaded_obfuscation = ObfuscationReloadState::from_config(&reloaded);
        let obfuscation_changed = previous_obfuscation != reloaded_obfuscation;
        let previous_download_exclusions = state
            .options_overlay
            .read()
            .await
            .watched_download_exclusions
            .clone()
            .unwrap_or_else(|| state.config.download_filter.exclude.clone());
        let download_exclusions_changed =
            previous_download_exclusions != reloaded.download_filter.exclude;
        let previous_restart_fingerprint = state
            .options_overlay
            .read()
            .await
            .watched_restart_fingerprint
            .clone()
            .unwrap_or_else(|| restart_reload_fingerprint(&state.config));
        let reloaded_restart_fingerprint = restart_reload_fingerprint(&reloaded);
        let restart_changed = previous_restart_fingerprint != reloaded_restart_fingerprint;
        {
            // Keep the last validated projection visible when a watched reload is
            // rejected.  This is the same retain-last-valid contract as the
            // upstream options monitor; applying the parsed YAML before loading
            // the full configuration briefly exposed an invalid or incomplete
            // overlay even though runtime consumers correctly kept old settings.
            let mut overlay = state.options_overlay.write().await;
            overlay.apply_yaml(
                parsed
                    .clone()
                    .map(controller_yaml_api_projection)
                    .unwrap_or(serde_json::Value::Null),
            );
            overlay.watched_download_exclusions = Some(reloaded.download_filter.exclude.clone());
            overlay.watched_obfuscation = Some(reloaded_obfuscation);
            overlay.watched_restart_fingerprint = Some(reloaded_restart_fingerprint);
            overlay.watched_share_directories = Some(
                parsed
                    .as_ref()
                    .and_then(|value| value.pointer("/shares/directories"))
                    .and_then(serde_json::Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            );
        }
        *state
            .controller_options_validation_error
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        let new_directories = reloaded
            .share_settings
            .directories
            .iter()
            .map(|directory| directory.raw.clone())
            .collect::<Vec<_>>();
        {
            let mut overlay = state.options_overlay.write().await;
            overlay.watched_share_directories = Some(new_directories);
            overlay.watched_instance_name = Some(reloaded.instance_name.clone());
            overlay.watched_controller_swagger = Some(reloaded.controller_swagger);
            overlay.watched_dht = Some(reloaded.advanced_networking.dht.clone());
        }
        if restart_changed {
            mutate_runtime_compat_state_in_memory(state, |runtime| {
                runtime.set_restart_requested(true);
            })
            .await;
        }
        *state
            .downloads_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = reloaded.downloads_dir.clone();
        *state
            .incomplete_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = reloaded.incomplete_dir.clone();
        *state
            .download_completed_path_template
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            reloaded.download_completed_path_template.clone();
        *state
            .remote_file_management
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = reloaded.remote_file_management;
        *state
            .remote_configuration
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = reloaded.remote_configuration;
        *state
            .controller_no_config_watch
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            reloaded.controller_no_config_watch;
        let previous_case_sensitive_regex = *state
            .controller_case_sensitive_regex
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *state
            .controller_case_sensitive_regex
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            reloaded.controller_case_sensitive_regex;
        *state
            .controller_web_auth_username
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            reloaded.controller_web_auth_username.clone();
        *state
            .controller_web_auth_password
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            reloaded.controller_web_auth_password.clone();
        *state
            .controller_web_jwt_key_current
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            if reloaded.controller_web_jwt_key_configured {
                reloaded.controller_web_jwt_key.clone()
            } else {
                state.config.controller_web_jwt_key.clone()
            };
        *state.private_message_auto_response_settings.write().await =
            reloaded.private_message_auto_response.clone();
        *state.integration_settings.write().await = reloaded.integrations.clone();
        *state.transfer_auto_retry_settings.write().await = reloaded.transfer_auto_retry.clone();
        *state.transfer_upload_settings.write().await = reloaded.transfer_upload.clone();
        *state.transfer_download_settings.write().await = reloaded.transfer_download.clone();
        *state.transfer_groups_settings.write().await = reloaded.transfer_groups.clone();
        *state.core_workflow_settings.write().await = reloaded.core_workflow.clone();
        *state.advanced_networking.write().await = reloaded.advanced_networking.clone();
        *state.media_services.write().await = reloaded.media_services.clone();
        if download_exclusions_changed {
            cancel_downloads_blocked_by_policy(state, &reloaded.download_filter.exclude).await;
        }
        state
            .rooms
            .write()
            .await
            .merge_configured(&reloaded.core_workflow.rooms);
        // Matches the oracle's real OptionsMonitor_OnChange, which calls
        // RoomService.TryJoinAsync(newOptions.Rooms) on every config reload,
        // issuing a real join for each configured room over the live
        // session (idempotent for rooms already joined) -- previously
        // config-reload only updated local bookkeeping, so a room added to
        // an already-connected instance was marked "joined" in the API
        // immediately but the server was never actually told to join it
        // until the next reconnect.
        for room in &reloaded.core_workflow.rooms {
            send_room_join_if_connected(state, room.clone()).await;
        }
        state.interests.write().await.merge_configured(
            &reloaded.core_workflow.liked_interests,
            &reloaded.core_workflow.hated_interests,
        );
        if reloaded.advanced_networking.mesh.enabled
            && reloaded.advanced_networking.mesh.enable_soulseek_rendezvous
        {
            let _ = state
                .interests
                .write()
                .await
                .add_liked(MESH_RENDEZVOUS_INTEREST_TAG.to_owned());
        }
        if obfuscation_changed && state.session.read().await.state == "connected" {
            state.runtime.write().await.set_reconnect_pending(true);
        }
        *state.destinations.write().await = DestinationStore::from_config(
            &reloaded.downloads_dir,
            &reloaded.core_workflow.destinations,
        );
        *state.diagnostics_allow_memory_dump.write().await =
            reloaded.controller_diagnostics_allow_memory_dump;
        *state.diagnostics_allow_remote_dump.write().await =
            reloaded.controller_diagnostics_allow_remote_dump;
        state.managed_blacklist.write().await.replace(
            reloaded.managed_blacklist.clone(),
            reloaded.controller_profile,
            reloaded.controller_case_sensitive_regex,
        );
        let search_filters_changed = {
            let current = state.search_request_filters.read().await;
            previous_case_sensitive_regex != reloaded.controller_case_sensitive_regex
                || current
                    .iter()
                    .map(|filter| filter.expression.as_str())
                    .ne(reloaded
                        .controller_search_request_filters
                        .iter()
                        .map(String::as_str))
        };
        if search_filters_changed {
            *state.search_request_filters.write().await = compile_controller_regexes(
                &reloaded.controller_search_request_filters,
                reloaded.controller_case_sensitive_regex,
                reloaded.controller_profile,
            )
            .expect("validated search request filters must compile");
        }
        let server_address_changed = effective_server_address(state) != reloaded.server_address;
        if server_address_changed {
            *state
                .server_address
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                reloaded.server_address.clone();
            if state.session.read().await.state == "connected" {
                state.runtime.write().await.set_reconnect_pending(true);
            }
        }
        let reloaded_credentials = reloaded.credentials();
        let credentials_changed =
            *state.configured_credentials.read().await != reloaded_credentials;
        if credentials_changed {
            *state.configured_credentials.write().await = reloaded_credentials;
            if state.session.read().await.state == "connected" {
                state.runtime.write().await.set_reconnect_pending(true);
            }
        }
        *state
            .user_info_description
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            reloaded.user_info_description.clone();
        *state
            .user_info_picture
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            reloaded.user_info_picture.clone();
        apply_distributed_settings(state, reloaded.soulseek_distributed).await;
        let regular_listener_result =
            reconfigure_regular_listener(state, reloaded.listener_bind.clone()).await;
        if state.config.shared_mesh_tcp()
            && reloaded.shared_mesh_tcp()
            && regular_listener_result.is_ok()
        {
            if let (Some(gateway), Some(bind)) = (
                state.private_gateway.as_ref(),
                reloaded
                    .listener_bind
                    .as_deref()
                    .and_then(|value| value.parse::<SocketAddr>().ok()),
            ) {
                gateway.set_bind(bind);
            }
        }
        let defer_obfuscated_listener = !reloaded.current_upstream_behavior
            && reloaded.controller_profile == ControllerProfile::Native;
        if defer_obfuscated_listener
            && matches!(regular_listener_result.as_ref(), Ok(true))
            && state.session.read().await.state == "connected"
        {
            state.runtime.write().await.set_reconnect_pending(true);
        }
        let obfuscated_bind = reloaded
            .obfuscation_enabled
            .then(|| reloaded.obfuscated_listener_bind.clone())
            .flatten();
        // native profile projects watched obfuscation changes immediately but keeps
        // the running obfuscation listener and its advertised port until the
        // reconnect/restart boundary.  Rebinding here would close an active
        // socket when obfuscation is disabled and would open a new socket
        // when it is enabled, both of which diverge from OptionsMonitor's
        // live-reload behavior.
        let obfuscated_listener_result = if defer_obfuscated_listener {
            Ok(false)
        } else {
            reconfigure_obfuscated_listener(state, obfuscated_bind).await
        };
        match (regular_listener_result, obfuscated_listener_result) {
            (Ok(regular_changed), Ok(obfuscated_changed))
                if regular_changed || obfuscated_changed || obfuscation_changed =>
            {
                *state
                    .advertised_port
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = reloaded.advertised_port;
                if !defer_obfuscated_listener {
                    *state
                        .obfuscated_advertised_port
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = reloaded
                        .obfuscation_enabled
                        .then_some(reloaded.obfuscated_advertised_port)
                        .flatten();
                }
                if let Err(error) = send_session_command(
                    state,
                    SessionCommand::SetWaitPort {
                        port: reloaded.advertised_port,
                        obfuscated_port: effective_obfuscated_advertised_port(state),
                    },
                )
                .await
                {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Warn,
                        "configuration",
                        format!("failed to publish reloaded Soulseek listener port: {error}"),
                    )
                    .await;
                }
            }
            (Ok(_), Ok(_)) => {}
            (regular, obfuscated) => {
                let error = [regular.err(), obfuscated.err()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join("; ");
                record_daemon_log(
                    state,
                    logging::LogLevel::Error,
                    "configuration",
                    format!("failed to apply Soulseek listener update: {error}"),
                )
                .await;
            }
        }
        let _share_index_turn = state.share_index_persistence_lock.lock().await;
        let changed = {
            let current = state.share_settings.read().await;
            current.fixture_entries != reloaded.share_settings.fixture_entries
                || current.directories != reloaded.share_settings.directories
                || current.follow_symlinks != reloaded.share_settings.follow_symlinks
                || current.include_hidden != reloaded.share_settings.include_hidden
                || current.max_files != reloaded.share_settings.max_files
                || current.cache_tsv_enabled != reloaded.share_settings.cache_tsv_enabled
                || current.filters != reloaded.share_settings.filters
                || previous_case_sensitive_regex != reloaded.controller_case_sensitive_regex
                || current.cache_storage_mode != reloaded.share_settings.cache_storage_mode
                || current.cache_workers != reloaded.share_settings.cache_workers
                || current.cache_retention != reloaded.share_settings.cache_retention
                || current.probe_media_attributes != reloaded.share_settings.probe_media_attributes
        };
        *state.share_settings.write().await = reloaded.share_settings.clone();
        if !changed {
            return;
        }
        cancel_active_share_scan(state);
        state
            .share_settings_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        state.shares.write().await.roots = pending_share_roots(&reloaded.share_settings);
        state.share_lifecycle.write().await.scan_pending = true;
        drop(_share_index_turn);
        record_event(
            state,
            "options.shares.changed",
            "shares",
            Some("scan_pending=true".to_owned()),
        )
        .await;
    })
}

pub(super) fn write_controller_compatibility_yaml(
    config: &AppConfig,
    text: &str,
) -> Result<(), String> {
    let path = controller_compatibility_config_path(config);
    let parent = path
        .parent()
        .ok_or_else(|| "configuration file parent is unavailable".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("configuration directory create failed: {error}"))?;
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("configuration file must be a regular file".to_owned());
        }
        let backup = PathBuf::from(format!("{}.bak", path.display()));
        if let Ok(metadata) = fs::symlink_metadata(&backup) {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err("configuration backup must be a regular file".to_owned());
            }
        }
        let previous = read_controller_compatibility_yaml(config)?
            .ok_or_else(|| "configuration file disappeared during backup".to_owned())?;
        write_file_atomic(&backup, previous.as_bytes())
            .map_err(|error| format!("configuration backup failed: {error}"))?;
    }
    write_file_atomic(&path, text.as_bytes())
        .map_err(|error| format!("configuration file write failed: {error}"))
}

pub(super) const NATIVE_DEFAULT_ADVERSARIAL_YAML: &str = r#"security:
  adversarial:
    enabled: false
    profile: Disabled
    privacy:
      enabled: false
      padding:
        enabled: false
        bucket_sizes:
        - 512
        - 1024
        - 2048
        - 4096
        - 8192
        - 16384
        use_random_fill: true
        max_overhead_percent: 50
        max_unpadded_bytes: 0
        max_padded_bytes: 0
      timing:
        enabled: false
        jitter_ms: 100
        jitter_all_messages: true
      batching:
        enabled: false
        batch_window_ms: 1000
        max_batch_size: 10
      cover_traffic:
        enabled: false
        interval_seconds: 300
        only_when_idle: true
    anonymity: &o0
      enabled: false
      mode: Direct
      tor:
        socks_address: 127.0.0.1:9050
        isolate_streams: true
        control_port: __NATIVE_EMPTY__
        verify_connectivity: true
      i2_p:
        sam_address: 127.0.0.1:7656
        verify_connectivity: true
      relay_only:
        relay_authentication_token: ''
        trusted_relay_peers: []
        relay_peer_data_endpoints: []
        max_chain_length: 3
        require_encryption: true
    transport: &o1
      enabled: false
      mode: Direct
      primary_transport: Direct
      fallback_transports: []
      web_socket:
        enabled: false
        server_url: ''
        use_wss: true
        ignore_certificate_errors: false
        sub_protocol: __NATIVE_EMPTY__
        custom_headers: __NATIVE_EMPTY__
        max_pooled_connections: 10
        headers: {}
      http_tunnel:
        enabled: false
        server_url: __NATIVE_EMPTY__
        proxy_url: ''
        use_https: true
        method: POST
        custom_headers: __NATIVE_EMPTY__
        user_agent: __NATIVE_EMPTY__
      obfs4:
        enabled: false
        bridge_lines: []
        obfs4_proxy_path: ''
        verify_bridges: true
      meek:
        enabled: false
        bridge_url: ''
        front_domain: ''
        verify_front_domain: true
        custom_headers: __NATIVE_EMPTY__
        user_agent: __NATIVE_EMPTY__
    onion_routing:
      enabled: false
      circuit_rotation_minutes: 10
      max_circuit_length: 5
      use_diverse_relays: true
      bandwidth:
        enabled: true
        max_contributed_bandwidth_mbps: 10
        accounting_window_hours: 24
    censorship_resistance:
      enabled: false
      bridge_discovery:
        enabled: false
        request_email: ''
        custom_bridges: []
        health_check_interval_minutes: 60
      domain_fronting:
        enabled: false
        front_domain: ''
        real_domain: ''
        verify_front_domain: true
      steganography:
        enabled: false
        max_image_size_mb: 5
        verify_extracted_bridges: true
    plausible_deniability:
      enabled: false
      hidden_volumes:
        enabled: false
        max_volume_size_gb: 100
        use_argon2: true
        argon2_iterations: 3
      decoy_pods:
        enabled: false
        decoy_pod_count: 5
        activity_interval_minutes: 60
        auto_join_decoy_pods: true
    anonymity_layer: *o0
    obfuscated_transports: *o1
    mesh_transport_options:
      enable_direct: true
      tor:
        enabled: false
        socks_host: 127.0.0.1
        socks_port: 9050
        advertise_onion: false
        onion_port: __NATIVE_EMPTY__
        onion_address: __NATIVE_EMPTY__
        privacy_mode_no_clearnet_advertise: false
        allow_data_over_tor: false
        connection_timeout: 00:00:30
        max_concurrent_connections: 10
        enable_stream_isolation: true
        isolation_method: SocksAuth
      i2_p:
        enabled: false
        socks_host: 127.0.0.1
        socks_port: 4447
        advertise_i2_p: false
        destination_address: __NATIVE_EMPTY__
        allow_data_over_i2p: false
        connection_timeout: 00:00:45
        max_concurrent_connections: 5
      preference_order:
      - DirectQuic
      - TorOnionQuic
      - I2PQuic
...
"#;
