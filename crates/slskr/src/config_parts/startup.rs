use super::*;

impl AppConfig {
    pub fn from_layers<E: ConfigEnv>(
        config_file: Option<PathBuf>,
        file_config: FileConfig,
        base_env: &E,
    ) -> Result<Self, String> {
        let state_dir = optional_env_any(base_env, &["SLSKR_STATE_DIR", "SLSKD_APP_DIR"])
            .map(PathBuf::from)
            .or_else(|| file_config.app.state_dir.clone())
            .unwrap_or_else(default_state_dir);
        let profile_is_explicit = base_env.var("SLSKR_CONTROLLER_PROFILE").is_some()
            || file_config.compatibility.profile.is_some();
        let controller_profile = ControllerProfile::parse(
            base_env
                .var("SLSKR_CONTROLLER_PROFILE")
                .or_else(|| file_config.compatibility.profile.clone())
                .as_deref()
                .unwrap_or("native"),
        )?;
        let current_upstream_behavior = match base_env
            .var("SLSKR_PARITY_PROFILE")
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some("current") => true,
            Some("frozen") => false,
            Some(_) => return Err("SLSKR_PARITY_PROFILE must be current or frozen".to_owned()),
            None => !profile_is_explicit,
        };
        let controller_yaml =
            controller_yaml_environment(&state_dir.join("slskd.yml"), controller_profile)?;
        let layered_env = ControllerYamlEnv {
            base: base_env,
            yaml: controller_yaml,
        };
        let env = &layered_env;
        let auth_disabled = resolve_auth_disabled(env, file_config.auth.disabled.unwrap_or(false))?;
        let mut advanced_networking = AdvancedNetworkingSettings::from_layers(
            &file_config,
            env,
            controller_profile,
            current_upstream_behavior,
            &state_dir,
        )?;
        let mesh_gateway = MeshGatewaySettings::from_layers(&file_config.mesh_gateway, env)?;
        let media_services =
            MediaAdvancedServiceSettings::from_layers(&file_config, env, controller_profile)?;
        let social_federation =
            SocialFederationSettings::from_layers(file_config.social_federation, env)?;
        let federation_publishing =
            FederationPublishingSettings::from_layers(file_config.federation_publishing, env)?;
        let realm = RealmSettings::from_layers(file_config.realm, env)?;
        let controller_headless =
            env_bool_layer(env, "SLSKD_HEADLESS", file_config.headless.unwrap_or(false))?;
        let controller_swagger = env_bool_layer(
            env,
            "SLSKD_SWAGGER",
            file_config
                .feature
                .swagger
                .unwrap_or(controller_profile == ControllerProfile::Native),
        )?;
        let ControllerWebAuthSettings {
            controller_metrics_enabled,
            controller_metrics_url,
            controller_metrics_auth_disabled,
            controller_metrics_username,
            controller_metrics_password,
            controller_web_auth_username,
            controller_web_auth_password,
            controller_web_jwt_key,
            controller_web_jwt_key_configured,
            controller_web_jwt_ttl_millis,
        } = resolve_controller_web_auth(
            env,
            controller_profile,
            !auth_disabled,
            file_config.metrics.enabled,
            file_config.metrics.url,
            file_config.metrics.authentication.disabled,
            file_config.metrics.authentication.username,
            file_config.metrics.authentication.password,
            file_config.auth.username,
            file_config.auth.password,
            file_config.auth.jwt.key,
            file_config.auth.jwt.ttl,
        )?;
        let instance_name = optional_env_any(env, &["SLSKR_INSTANCE_NAME", "SLSKD_INSTANCE_NAME"])
            .unwrap_or_else(|| "default".to_owned());
        let configured_downloads_dir =
            optional_env_any(env, &["SLSKR_DOWNLOADS_DIR", "SLSKD_DOWNLOADS_DIR"]);
        let downloads_dir = configured_downloads_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| state_dir.join("downloads"));
        validate_controller_storage_directory(
            "Directories.Downloads",
            &downloads_dir,
            controller_profile,
            configured_downloads_dir.is_some(),
        )?;
        let configured_incomplete_dir =
            optional_env_any(env, &["SLSKR_INCOMPLETE_DIR", "SLSKD_INCOMPLETE_DIR"]);
        let incomplete_dir = configured_incomplete_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| state_dir.join("incomplete"));
        validate_controller_storage_directory(
            "Directories.Incomplete",
            &incomplete_dir,
            controller_profile,
            configured_incomplete_dir.is_some(),
        )?;
        let ControllerWebResolution {
            http_bind,
            http_binds,
            controller_http_address,
            controller_web,
        } = resolve_controller_web(
            env,
            controller_profile,
            file_config.app.http_bind,
            file_config.web.socket,
            file_config.web.url_base,
            file_config.web.content_path,
            file_config.web.logging,
            file_config.web.https,
        )?;
        let controller_api_keys =
            resolve_controller_api_keys(env, controller_profile, file_config.auth.api_keys)?;
        let SoulseekIdentity {
            server_address,
            listen_port,
            username,
            password,
            credential_store,
            credential_file,
            auto_connect,
        } = resolve_soulseek_identity(
            env,
            &state_dir,
            file_config.network.server_address,
            file_config.network.listen_port,
            file_config.network.username,
            file_config.network.password,
            file_config.network.credential_store,
            file_config.network.credential_file,
            file_config.app.auto_connect,
        )?;
        let reconnect = env_bool_layer(
            env,
            "SLSKR_RECONNECT",
            file_config.app.reconnect.unwrap_or(auto_connect),
        )?;
        let reconnect_delay = validated_runtime_interval(
            "SLSKR_RECONNECT_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_RECONNECT_SECONDS",
                file_config.app.reconnect_seconds,
                30,
            )?,
        )?;
        let ping_interval = validated_runtime_interval(
            "SLSKR_PING_SECONDS",
            env_parse_layer(env, "SLSKR_PING_SECONDS", file_config.app.ping_seconds, 300)?,
        )?;
        let log_level = env
            .var("SLSKR_LOG_LEVEL")
            .or(file_config.app.log_level)
            .or_else(|| env.var("RUST_LOG"))
            .unwrap_or_else(|| "info".to_owned());
        let daemon_flags = DaemonFlagsSettings::from_layers(
            file_config.flags.force_migrations,
            file_config.flags.legacy_windows_tcp_keepalive,
            file_config.flags.log_sql,
            file_config.flags.log_unobserved_exceptions,
            file_config.flags.optimistic_relay_file_info,
            file_config.flags.volatile,
            env,
        )?;
        let logger = LoggerSettings::from_layers(
            file_config.logger.disk,
            file_config.logger.loki,
            file_config.logger.no_color,
            env,
        )?;
        let permissions_file_mode = env
            .var("SLSKD_FILE_PERMISSION_MODE")
            .or(file_config.permissions.file.mode)
            .filter(|value| !value.is_empty());
        if permissions_file_mode.as_deref().is_some_and(|mode| {
            !(3..=4).contains(&mode.len())
                || !mode.bytes().all(|byte| (b'0'..=b'7').contains(&byte))
        }) {
            return Err(
                "permissions.file.mode must be a three- or four-character chmod value".to_owned(),
            );
        }
        if controller_profile == ControllerProfile::Legacy && permissions_file_mode.is_some() {
            return Err("The 'permissions' keys have been moved under a new 'destination' key under transfers -> download, and the behavior has changed.  See https://github.com/slskd/slskd/pull/1756 for details".to_owned());
        }
        let telemetry_tracing = TelemetryTracingSettings::from_layers(
            file_config.telemetry.tracing.enabled,
            file_config.telemetry.tracing.exporter,
            file_config.telemetry.tracing.jaeger_endpoint,
            file_config.telemetry.tracing.jaeger_port,
            file_config.telemetry.tracing.otlp_endpoint,
            env,
        )?;
        let retention = RetentionSettings::from_layers(
            file_config.retention.search,
            file_config.retention.logs,
            file_config.retention.files.complete,
            file_config.retention.files.incomplete,
            file_config.retention.transfers.upload,
            file_config.retention.transfers.download,
            env,
        )?;
        let search_retention = SearchRetentionSettings::from_layers(
            file_config
                .filters
                .search_retention
                .cleanup_interval_seconds,
            file_config.filters.search_retention.max_age_days,
            file_config.filters.search_retention.max_count,
            env,
        )?;
        let core_workflow = CoreWorkflowSettings::from_layers(env)?;
        let ListenerAndObfuscationResolution {
            listener_bind,
            advertised_port,
            obfuscated_listener_bind,
            obfuscated_advertised_port,
            overlay_bind,
            dht_enabled,
            mut dht_port,
            trusted_mesh_peers,
            obfuscation_enabled,
            obfuscation_mode,
            obfuscation_listen_port,
            obfuscation_advertise_regular_port,
            obfuscation_prefer_outbound,
        } = resolve_listener_and_obfuscation(
            env,
            controller_profile,
            current_upstream_behavior,
            &advanced_networking,
            listen_port,
            auto_connect,
            file_config.listeners.regular_bind,
            file_config.listeners.advertised_port,
            file_config.listeners.obfuscated_bind,
            file_config.listeners.obfuscated_advertised_port,
            file_config.listeners.overlay_bind,
            file_config.mesh.trusted_peers,
            file_config.network.obfuscation.enabled,
            file_config.network.obfuscation.mode,
            file_config.network.obfuscation.advertise_regular_port,
            file_config.network.obfuscation.prefer_outbound,
        )?;
        if controller_profile == ControllerProfile::Native && current_upstream_behavior {
            if let Some(bind) = listener_bind
                .as_deref()
                .and_then(|value| value.parse::<SocketAddr>().ok())
            {
                if advanced_networking.mesh.enabled
                    && advanced_networking.mesh.enable_overlay
                    && overlay_bind != Some(bind)
                {
                    return Err(
                        "native/current mesh overlay must share the Soulseek listener bind"
                            .to_owned(),
                    );
                }
                dht_port = bind.port();
                advanced_networking.dht.dht_port = bind.port();
                advanced_networking.dht.overlay_port = bind.port();
                advanced_networking.overlay.listen_port = bind.port();
                advanced_networking.overlay.quic_listen_port = bind.port();
                advanced_networking.overlay.share_quic_with_dht_port = true;
                advanced_networking.overlay_data.listen_port = bind.port();
                advanced_networking.overlay_data.share_with_dht_port = true;
            }
        }
        let PeerProfileSettings {
            peer_host_override,
            distributed_parent_override,
            test_user_endpoint_overrides,
            user_info_description,
            user_info_picture,
            soulseek_diagnostic_level,
        } = resolve_peer_profile(
            env,
            file_config.profile.user_info_description,
            file_config.profile.user_info_picture,
            file_config.profile.soulseek_diagnostic_level,
        )?;
        let soulseek_distributed = SoulseekDistributedSettings {
            disabled: env_bool_any_layer(
                env,
                &["SLSKR_SLSK_NO_DNET", "SLSKD_SLSK_NO_DNET", "SLSK_NO_DNET"],
                file_config
                    .network
                    .distributed_network
                    .disabled
                    .unwrap_or(false),
            )?,
            disable_children: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SLSK_DNET_NO_CHILDREN",
                    "SLSKD_SLSK_DNET_NO_CHILDREN",
                    "SLSK_DNET_NO_CHILDREN",
                ],
                file_config
                    .network
                    .distributed_network
                    .disable_children
                    .unwrap_or(false),
            )?,
            child_limit: bounded_config_value(
                "SLSK_DNET_CHILDREN",
                env_parse_any_layer(
                    env,
                    &[
                        "SLSKR_SLSK_DNET_CHILDREN",
                        "SLSKD_SLSK_DNET_CHILDREN",
                        "SLSK_DNET_CHILDREN",
                    ],
                    file_config.network.distributed_network.child_limit,
                    25_usize,
                )?,
                1,
                i32::MAX as usize,
            )?,
            logging: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SLSK_DNET_LOGGING",
                    "SLSKD_SLSK_DNET_LOGGING",
                    "SLSK_DNET_LOGGING",
                ],
                file_config
                    .network
                    .distributed_network
                    .logging
                    .unwrap_or(false),
            )?,
        };
        let peer_response_timeout_seconds = env_parse_layer(
            env,
            "SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS",
            file_config.timeouts.peer_response_seconds,
            5_u64,
        )?;
        let peer_response_timeout = validated_runtime_interval(
            "SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS",
            peer_response_timeout_seconds,
        )?;
        let soulseek_connection = SoulseekConnectionSettings::from_layers(
            file_config.network.connection,
            env,
            controller_profile,
        )?;
        let controller_case_sensitive_regex = env_bool_layer(
            env,
            "SLSKD_CASE_SENSITIVE_REGEX",
            file_config.flags.case_sensitive_reg_ex.unwrap_or(false),
        )?;
        let controller_search_request_filters = controller_string_array_layer(
            env,
            "SLSKD_SEARCH_REQUEST_FILTER",
            file_config.filters.search.request.clone(),
        );
        for filter in &controller_search_request_filters {
            crate::dotnet_regex::DotNetRegex::validate(filter).map_err(|_| {
                format!("Search request filter '{filter}' is not a valid regular expression")
            })?;
        }
        let download_filter = DownloadFilterSettings::from_layers(
            &file_config.filters.download,
            env,
            current_upstream_behavior,
        )?;
        let canonical_groups = file_config.transfers.groups.clone();
        let compatibility_groups = file_config.groups.clone();
        let user_blacklist_file_config = match env.var("SLSKR_FROZEN_TRANSFER_GROUPS_JSON") {
            Some(json) => {
                serde_json::from_str::<GroupsFileConfig>(&json)
                    .map_err(|error| format!("invalid transfer groups configuration: {error}"))?
                    .blacklisted
            }
            None if groups_file_config_is_empty(&canonical_groups) => {
                compatibility_groups.blacklisted.clone()
            }
            None => canonical_groups.blacklisted.clone(),
        };
        let share_settings =
            ShareSettings::from_layers(file_config.shares, env, controller_profile)?;
        let transfer_history_limit = env_parse_layer(
            env,
            "SLSKR_TRANSFER_HISTORY_LIMIT",
            file_config.transfers.history_limit,
            500_usize,
        )?;
        let transfer_max_active = env_parse_layer(
            env,
            "SLSKR_TRANSFER_MAX_ACTIVE",
            file_config.transfers.max_active,
            3_usize,
        )?;
        let transfer_allow_inbound = env_bool_layer(
            env,
            "SLSKR_TRANSFER_ALLOW_INBOUND",
            file_config.transfers.allow_inbound.unwrap_or(true),
        )?;
        let transfer_allow_outbound = env_bool_layer(
            env,
            "SLSKR_TRANSFER_ALLOW_OUTBOUND",
            file_config.transfers.allow_outbound.unwrap_or(true),
        )?;
        let transfer_upload =
            TransferUploadSettings::from_layers(file_config.transfers.upload, env)?;
        let transfer_download = TransferDownloadSettings::from_layers(
            file_config.transfers.download,
            file_config.auto_replace,
            env,
            controller_profile,
            current_upstream_behavior,
        )?;
        let transfer_groups = TransferGroupsSettings::from_layers(
            canonical_groups,
            compatibility_groups,
            env,
            controller_profile,
        )?;
        let transfer_auto_retry =
            TransferAutoRetrySettings::from_layers(file_config.transfers.auto_retry, env)?;
        let transfer_rescue =
            TransferRescueSettings::from_layers(file_config.transfers.rescue, env)?;
        let managed_blacklist = ManagedBlacklistSettings::from_layers(
            file_config.blacklist,
            &user_blacklist_file_config,
            env,
            controller_profile,
        )?;
        let download_completed_path_template = optional_env_any(
            env,
            &[
                "SLSKR_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
                "SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
            ],
        )
        .or(file_config.transfers.completed_path_template)
        .unwrap_or_default();
        if download_completed_path_template.len() > MAX_COMPLETED_PATH_TEMPLATE_BYTES {
            return Err(format!(
                "download completed path template exceeds {MAX_COMPLETED_PATH_TEMPLATE_BYTES} bytes"
            ));
        }
        if download_completed_path_template.contains('\0') {
            return Err("download completed path template contains a NUL byte".to_owned());
        }
        let private_message_auto_response = PrivateMessageAutoResponseSettings::from_layers(
            file_config.network.private_message_auto_response,
            env,
            "Hi, I'm human and testing an slskR client. Shares may be temporarily unavailable while I validate the client.",
        )?;
        let MiscControllerFlags {
            remote_configuration,
            remote_file_management,
            controller_debug,
            controller_no_config_watch,
            controller_no_logo,
            controller_no_start,
            controller_no_version_check,
            controller_experimental,
            controller_hash_from_audio_file_enabled,
            controller_no_share_scan,
            controller_force_share_scan,
        } = resolve_misc_controller_flags(
            env,
            file_config.compatibility.remote_configuration,
            file_config.compatibility.debug,
            file_config.compatibility.no_config_watch,
            file_config.flags.no_logo,
            file_config.flags.no_start,
            file_config.flags.no_version_check,
            file_config.flags.experimental,
            file_config.flags.hash_from_audio_file_enabled,
            file_config.flags.no_share_scan,
            file_config.flags.force_share_scan,
        )?;
        let ApiAndWebHardeningSettings {
            api_token,
            api_read_write_token,
            api_read_only_token,
            api_nowplaying_token,
            auth_required,
            api_cookie_auth_enabled,
            api_rate_limit_anonymous,
            api_rate_limit_authenticated,
            controller_web_max_request_body_size,
            controller_web_enforce_security,
            controller_web_allow_remote_no_auth,
            controller_web_passthrough_allowed_cidrs,
            controller_web_passthrough_cidrs,
            controller_diagnostics_allow_memory_dump,
            controller_diagnostics_allow_remote_dump,
            controller_web_cors,
            controller_web_rate_limiting,
        } = resolve_api_and_web_hardening(
            env,
            controller_profile,
            file_config.auth.api_token,
            file_config.auth.read_write_token,
            file_config.auth.read_only_token,
            file_config.auth.nowplaying_token,
            file_config.auth.disabled,
            file_config.auth.cookie_auth_enabled,
            file_config.auth.rate_limit_anonymous,
            file_config.auth.rate_limit_authenticated,
            file_config.web.max_request_body_size,
            file_config.web.enforce_security,
            file_config.web.allow_remote_no_auth,
            file_config.web.passthrough_allowed_cidrs,
            file_config.diagnostics.allow_memory_dump,
            file_config.diagnostics.allow_remote_dump,
            file_config.web.cors,
            file_config.web.rate_limiting,
        )?;
        let trusted_proxy_cidrs = trusted_proxy_cidrs_from_layers(
            env.var("SLSKR_TRUSTED_PROXY_CIDRS"),
            file_config.auth.trusted_proxy_cidrs,
        )?;
        let persistence_enabled = env_bool_layer(
            env,
            "SLSKR_PERSISTENCE_ENABLED",
            file_config.persistence.enabled.unwrap_or(false),
        )?;
        let pod_join_signature_mode = PodSignatureMode::parse(
            env.var("SLSKR_POD_JOIN_SIGNATURE_MODE")
                .or(file_config.podcore.join.signature_mode)
                .unwrap_or_else(|| "off".to_owned())
                .as_str(),
        )?;
        let virtual_soulfind_v2_explicit = file_config.virtual_soulfind_v2.enabled.is_some()
            || env.var("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED").is_some();
        let requested_virtual_soulfind_v2_enabled = env_bool_layer(
            env,
            "SLSKR_VIRTUAL_SOULFIND_V2_ENABLED",
            file_config.virtual_soulfind_v2.enabled.unwrap_or(false),
        )?;
        // The frozen native controller registers the v2 routes but keeps their
        // API option disabled by default. Its source-provider controller reads a
        // separate options type whose acquisition-planning option defaults to
        // enabled. Preserve both target defaults while allowing an explicit
        // file or environment value to control both contracts.
        let virtual_soulfind_v2_enabled = requested_virtual_soulfind_v2_enabled;
        let acquisition_planning_enabled = if virtual_soulfind_v2_explicit {
            requested_virtual_soulfind_v2_enabled
        } else {
            true
        };
        let mut integrations = IntegrationSettings::from_layers(
            file_config.integrations,
            env,
            current_upstream_behavior,
        )?;
        integrations.external_visualizer = media_services.external_visualizer.clone();

        Ok(Self {
            config_file,
            http_bind,
            http_binds,
            controller_http_address,
            state_dir,
            instance_name,
            downloads_dir,
            incomplete_dir,
            server_address,
            listen_port,
            username,
            password,
            credential_store,
            credential_file,
            auto_connect,
            reconnect,
            reconnect_delay,
            ping_interval,
            log_level,
            daemon_flags,
            logger,
            permissions_file_mode,
            telemetry_tracing,
            retention,
            search_retention,
            core_workflow,
            advanced_networking,
            mesh_gateway,
            media_services,
            social_federation,
            federation_publishing,
            realm,
            controller_web,
            controller_api_keys,
            listener_bind,
            advertised_port,
            obfuscated_listener_bind,
            obfuscated_advertised_port,
            overlay_bind,
            dht_enabled,
            dht_port,
            trusted_mesh_peers,
            obfuscation_enabled,
            obfuscation_mode,
            obfuscation_listen_port,
            obfuscation_advertise_regular_port,
            obfuscation_prefer_outbound,
            peer_host_override,
            distributed_parent_override,
            test_user_endpoint_overrides,
            user_info_description,
            user_info_picture,
            soulseek_diagnostic_level,
            soulseek_distributed,
            peer_response_timeout,
            soulseek_connection,
            share_settings,
            transfer_history_limit,
            transfer_max_active,
            transfer_allow_inbound,
            transfer_allow_outbound,
            transfer_upload,
            transfer_download,
            transfer_groups,
            transfer_auto_retry,
            transfer_rescue,
            managed_blacklist,
            download_completed_path_template,
            private_message_auto_response,
            pod_join_signature_mode,
            virtual_soulfind_v2_enabled,
            acquisition_planning_enabled,
            controller_profile,
            current_upstream_behavior,
            controller_headless,
            remote_configuration,
            remote_file_management,
            controller_debug,
            controller_no_config_watch,
            controller_no_logo,
            controller_no_start,
            controller_no_version_check,
            controller_experimental,
            controller_hash_from_audio_file_enabled,
            controller_case_sensitive_regex,
            controller_search_request_filters,
            download_filter,
            controller_no_share_scan,
            controller_force_share_scan,
            controller_swagger,
            controller_metrics_enabled,
            controller_metrics_url,
            controller_metrics_auth_disabled,
            controller_metrics_username,
            controller_metrics_password,
            controller_web_auth_username,
            controller_web_auth_password,
            controller_web_jwt_key,
            controller_web_jwt_key_configured,
            controller_web_jwt_ttl_millis,
            auth_required,
            api_token,
            api_read_write_token,
            api_read_only_token,
            api_nowplaying_token,
            api_cookie_auth_enabled,
            api_rate_limit_anonymous,
            api_rate_limit_authenticated,
            controller_web_enforce_security,
            controller_web_allow_remote_no_auth,
            controller_web_passthrough_allowed_cidrs,
            controller_web_passthrough_cidrs,
            controller_web_max_request_body_size,
            controller_web_cors,
            controller_web_rate_limiting,
            controller_diagnostics_allow_memory_dump,
            controller_diagnostics_allow_remote_dump,
            trusted_proxy_cidrs,
            persistence_enabled,
            integrations,
        })
    }

    pub fn credentials(&self) -> Option<LoginCredentials> {
        Some(LoginCredentials::default_client(
            self.username.clone()?,
            self.password.clone()?,
        ))
    }

    /// Current upstream-style native deployments put Soulseek peer traffic and
    /// the TLS mesh overlay on one public TCP listener. Require an explicitly
    /// configured overlay bind equal to the peer bind; frozen profiles retain
    /// their compatibility listener projection.
    pub fn shared_mesh_tcp(&self) -> bool {
        if self.controller_profile != ControllerProfile::Native
            || !self.current_upstream_behavior
            || !self.advanced_networking.mesh.enabled
            || !self.advanced_networking.mesh.enable_overlay
        {
            return false;
        }
        let Some(listener_bind) = self
            .listener_bind
            .as_deref()
            .and_then(|value| value.parse::<SocketAddr>().ok())
        else {
            return false;
        };
        self.overlay_bind == Some(listener_bind)
    }

    pub fn controller_passthrough_allows(&self, remote: Option<SocketAddr>) -> bool {
        let Some(remote) = remote else {
            return false;
        };
        let ip = match remote.ip() {
            IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4),
            ip => ip,
        };
        ip.is_loopback()
            || (self.controller_web_allow_remote_no_auth
                && self
                    .controller_web_passthrough_cidrs
                    .iter()
                    .any(|cidr| cidr.contains(ip)))
    }

    pub fn validate_controller_startup_hardening(&self) -> Result<(), String> {
        if self.controller_profile != ControllerProfile::Native {
            return Ok(());
        }

        let check = |condition: bool, rule: &str, message: &str| -> Result<(), String> {
            if !condition {
                return Ok(());
            }
            if self.controller_web_enforce_security {
                Err(format!("[{rule}] {message}"))
            } else {
                eprintln!("warning: [{rule}] {message}");
                Ok(())
            }
        };

        check(
            !self.auth_required
                && self.http_binds.iter().any(|address| !address.ip().is_loopback())
                && !self.controller_web_allow_remote_no_auth,
            "AuthDisabledNonLoopback",
            "Authentication is disabled and the application binds to a non-loopback address. Set Web.AllowRemoteNoAuth=true to allow, or bind to loopback only.",
        )?;
        check(
            !self.auth_required
                && self.controller_web_allow_remote_no_auth
                && self
                    .controller_web_passthrough_allowed_cidrs
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty()),
            "RemoteNoAuthWithoutCidrs",
            "Web.AllowRemoteNoAuth is enabled without Web.Authentication.Passthrough.AllowedCidrs. Remote no-auth access must be constrained to explicit CIDRs.",
        )?;
        check(
            self.controller_web_cors.enabled
                && self.controller_web_cors.allow_credentials
                && (self.controller_web_cors.allowed_origins.is_empty()
                    || self
                        .controller_web_cors
                        .allowed_origins
                        .iter()
                        .any(|origin| origin.eq_ignore_ascii_case("*"))),
            "CorsCredentialsWithWildcard",
            "CORS is configured with AllowCredentials and wildcard/any origin, which is unsafe. Use an explicit AllowedOrigins list and no wildcard.",
        )?;
        check(
            self.controller_diagnostics_allow_memory_dump && !self.auth_required,
            "MemoryDumpWithAuthDisabled",
            "Diagnostics.AllowMemoryDump is true while authentication is disabled. Enable authentication or set AllowMemoryDump=false.",
        )?;
        check(
            self.controller_metrics_enabled
                && !self.controller_metrics_auth_disabled
                && self.controller_metrics_password.trim().is_empty(),
            "WeakMetricsPassword",
            "Web.Authentication.Metrics.Password is empty. The Prometheus metrics endpoint will be protected with no password. Set a strong password via web.authentication.metrics.password or disable the metrics endpoint.",
        )?;
        if self.controller_hash_from_audio_file_enabled {
            return Err(
                "[HashFromAudioFileEnabled] Flags.HashFromAudioFileEnabled is true but audio hash from file requires unavailable PCM extraction support. Set it to false; this option is not supported in this build."
                    .to_owned(),
            );
        }
        Ok(())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"config_file\":{},\"http_bind\":\"{}\",\"state_dir\":\"{}\",\"server_address\":\"{}\",\"listen_port\":{},\"advertised_port\":{},\"listener_bind\":{},\"obfuscated_listener_bind\":{},\"obfuscated_advertised_port\":{},\"overlay_bind\":{},\"shared_mesh_tcp\":{},\"dht_enabled\":{},\"dht_port\":{},\"trusted_mesh_peers\":{},\"obfuscation\":{},\"peer_host_override\":{},\"test_user_endpoint_overrides\":{},\"username\":{},\"credentials_configured\":{},\"credential_store\":\"{}\",\"credential_file\":\"{}\",\"auto_connect\":{},\"reconnect\":{},\"reconnect_seconds\":{},\"ping_seconds\":{},\"log_level\":\"{}\",\"peer_response_timeout_seconds\":{},\"share_roots\":{},\"share_follow_symlinks\":{},\"share_include_hidden\":{},\"share_scan_max_files\":{},\"share_cache_tsv_enabled\":{},\"transfer_history_limit\":{},\"transfer_max_active\":{},\"transfer_allow_inbound\":{},\"transfer_allow_outbound\":{},\"transfer_auto_retry\":{},\"transfer_rescue\":{},\"download_completed_path_template_configured\":{},\"private_message_auto_response\":{},\"pod_join_signature_mode\":\"{}\",\"virtual_soulfind_v2_enabled\":{},\"controller_profile\":\"{}\",\"parity_profile\":\"{}\",\"remote_configuration\":{},\"auth_required\":{},\"api_token_configured\":{},\"api_read_write_token_configured\":{},\"api_read_only_token_configured\":{},\"api_nowplaying_token_configured\":{},\"api_cookie_auth_enabled\":{},\"trusted_proxy_cidrs\":{},\"persistence_enabled\":{},\"integrations\":{}}}",
            json_option(
                self.config_file
                    .as_ref()
                    .map(|_| "config://file".to_owned())
                    .as_deref()
            ),
            json_escape(&self.http_bind.to_string()),
            "state://configured",
            json_escape(&self.server_address),
            self.listen_port,
            self.advertised_port,
            json_option(self.listener_bind.as_deref()),
            json_option(self.obfuscated_listener_bind.as_deref()),
            json_u32_option(self.obfuscated_advertised_port),
            json_option(self.overlay_bind.map(|bind| bind.to_string()).as_deref()),
            self.shared_mesh_tcp(),
            self.dht_enabled,
            self.dht_port,
            self.trusted_mesh_peers.len(),
            format_args!(
                "{{\"enabled\":{},\"mode\":\"{}\",\"listen_port\":{},\"advertise_regular_port\":{},\"prefer_outbound\":{},\"effective_prefer_outbound\":{}}}",
                self.obfuscation_enabled,
                self.obfuscation_mode.as_str(),
                self.obfuscation_listen_port,
                self.obfuscation_advertise_regular_port,
                self.obfuscation_prefer_outbound,
                self.prefer_obfuscated_outbound(),
            ),
            json_option(self.peer_host_override.map(|ip| ip.to_string()).as_deref()),
            self.test_user_endpoint_overrides.len(),
            json_option(self.username.as_deref().map(redact_username).as_deref()),
            self.username.is_some() && self.password.is_some(),
            self.credential_store.as_str(),
            "credential://configured",
            self.auto_connect,
            self.reconnect,
            self.reconnect_delay.as_secs(),
            self.ping_interval.as_secs(),
            json_escape(&self.log_level),
            self.peer_response_timeout.as_secs(),
            self.share_settings.roots.len(),
            self.share_settings.follow_symlinks,
            self.share_settings.include_hidden,
            self.share_settings.max_files,
            self.share_settings.cache_tsv_enabled,
            self.transfer_history_limit,
            self.transfer_max_active,
            self.transfer_allow_inbound,
            self.transfer_allow_outbound,
            self.transfer_auto_retry.sanitized_json(),
            self.transfer_rescue.sanitized_json(),
            !self.download_completed_path_template.is_empty(),
            self.private_message_auto_response.sanitized_json(),
            self.pod_join_signature_mode.as_str(),
            self.virtual_soulfind_v2_enabled,
            self.controller_profile.as_str(),
            if self.current_upstream_behavior { "current" } else { "frozen" },
            self.remote_configuration,
            self.auth_required,
            self.api_token.is_some(),
            self.api_read_write_token.is_some(),
            self.api_read_only_token.is_some(),
            self.api_nowplaying_token.is_some(),
            self.api_cookie_auth_enabled,
            self.trusted_proxy_cidrs.len(),
            self.persistence_enabled,
            self.integrations.sanitized_json()
        )
    }

    pub fn prefer_obfuscated_outbound(&self) -> bool {
        self.obfuscation_enabled
            && self.obfuscation_mode == SoulseekObfuscationMode::Prefer
            && self.obfuscation_prefer_outbound
    }
}
