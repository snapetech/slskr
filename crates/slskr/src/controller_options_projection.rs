use super::*;

pub(super) fn controller_options_json(
    config: &AppConfig,
    overlay: &ControllerOptionsOverlayState,
    include_volatile_overlay: bool,
) -> String {
    let defaults = match config.controller_profile {
        ControllerProfile::Legacy => {
            include_str!("../data/legacy-options-default.json")
        }
        ControllerProfile::Native => {
            include_str!("../data/native-options-default.json")
        }
    };
    let mut response = serde_json::from_str::<serde_json::Value>(defaults)
        .expect("frozen controller options fixture must be valid JSON");

    response["debug"] = serde_json::json!(config.controller_debug);
    response["headless"] = serde_json::json!(config.controller_headless);
    response["remoteConfiguration"] = serde_json::json!(config.remote_configuration);
    response["remoteFileManagement"] = serde_json::json!(config.remote_file_management);
    response["instanceName"] = serde_json::json!(config.instance_name);
    response["flags"]["noConfigWatch"] = serde_json::json!(config.controller_no_config_watch);
    response["flags"]["noConnect"] = serde_json::json!(!config.auto_connect);
    response["flags"]["noLogo"] = serde_json::json!(config.controller_no_logo);
    response["flags"]["noStart"] = serde_json::json!(config.controller_no_start);
    response["flags"]["noVersionCheck"] = serde_json::json!(config.controller_no_version_check);
    response["flags"]["experimental"] = serde_json::json!(config.controller_experimental);
    if config.controller_profile == ControllerProfile::Native {
        response["flags"]["hashFromAudioFileEnabled"] =
            serde_json::json!(config.controller_hash_from_audio_file_enabled);
    }
    response["flags"]["caseSensitiveRegEx"] =
        serde_json::json!(config.controller_case_sensitive_regex);
    response["flags"]["noShareScan"] = serde_json::json!(config.controller_no_share_scan);
    response["flags"]["forceShareScan"] = serde_json::json!(config.controller_force_share_scan);
    response["blacklist"]["enabled"] = serde_json::json!(config.managed_blacklist.enabled);
    if let Some(file) = config.managed_blacklist.file.as_deref() {
        response["blacklist"]["file"] = serde_json::json!(file.display().to_string());
    } else if config.controller_profile == ControllerProfile::Legacy {
        response["blacklist"]
            .as_object_mut()
            .expect("slskd blacklist defaults must be an object")
            .remove("file");
    }
    response["feature"]["swagger"] = serde_json::json!(config.controller_swagger);
    if config.current_upstream_behavior && config.controller_profile == ControllerProfile::Native {
        response["autoReplace"] = serde_json::json!({
            "intervalSeconds": config.transfer_download.auto_replace_interval.as_secs(),
            "maxRetries": config
                .transfer_download
                .auto_replace_max_retries
                .unwrap_or(3),
            "sizeThresholdPercent": config.transfer_download.auto_replace_threshold_percent,
        });
    }
    response["flags"]["forceMigrations"] = serde_json::json!(config.daemon_flags.force_migrations);
    response["flags"]["legacyWindowsTcpKeepalive"] =
        serde_json::json!(config.daemon_flags.legacy_windows_tcp_keepalive);
    response["flags"]["logSQL"] = serde_json::json!(config.daemon_flags.log_sql);
    response["flags"]["logUnobservedExceptions"] =
        serde_json::json!(config.daemon_flags.log_unobserved_exceptions);
    response["flags"]["optimisticRelayFileInfo"] =
        serde_json::json!(config.daemon_flags.optimistic_relay_file_info);
    response["flags"]["volatile"] = serde_json::json!(config.daemon_flags.volatile);
    response["logger"]["disk"] = serde_json::json!(config.logger.disk);
    if let Some(value) = config.logger.loki.as_ref() {
        response["logger"]["loki"] = serde_json::json!(value);
    } else if config.controller_profile == ControllerProfile::Native {
        response["logger"]["loki"] = serde_json::json!("");
    } else if let Some(logger) = response["logger"].as_object_mut() {
        logger.remove("loki");
    }
    response["logger"]["noColor"] = serde_json::json!(config.logger.no_color);
    if config.controller_profile == ControllerProfile::Native {
        response["permissions"]["file"]["mode"] = config
            .permissions_file_mode
            .as_ref()
            .map_or_else(|| serde_json::json!(""), |value| serde_json::json!(value));
    }
    if config.controller_profile == ControllerProfile::Native {
        response["telemetry"]["tracing"]["enabled"] =
            serde_json::json!(config.telemetry_tracing.enabled);
        response["telemetry"]["tracing"]["exporter"] =
            serde_json::json!(config.telemetry_tracing.exporter);
        let tracing = response["telemetry"]["tracing"]
            .as_object_mut()
            .expect("frozen tracing defaults must be an object");
        for (key, value) in [
            (
                "jaegerEndpoint",
                config.telemetry_tracing.jaeger_endpoint.as_ref(),
            ),
            (
                "otlpEndpoint",
                config.telemetry_tracing.otlp_endpoint.as_ref(),
            ),
        ] {
            if let Some(value) = value {
                tracing.insert(key.to_owned(), serde_json::json!(value));
            } else {
                tracing.remove(key);
            }
        }
        if let Some(value) = config.telemetry_tracing.jaeger_port {
            tracing.insert("jaegerPort".to_owned(), serde_json::json!(value));
        } else {
            tracing.remove("jaegerPort");
        }
    }
    let retention = response["retention"]
        .as_object_mut()
        .expect("frozen retention defaults must be an object");
    if let Some(value) = config.retention.search_minutes {
        retention.insert("search".to_owned(), serde_json::json!(value));
    } else {
        retention.remove("search");
    }
    response["retention"]["logs"] = serde_json::json!(config.retention.logs_days);
    let files = response["retention"]["files"]
        .as_object_mut()
        .expect("frozen file retention defaults must be an object");
    for (key, value) in [
        ("complete", config.retention.files_complete_minutes),
        ("incomplete", config.retention.files_incomplete_minutes),
    ] {
        if let Some(value) = value {
            files.insert(key.to_owned(), serde_json::json!(value));
        } else {
            files.remove(key);
        }
    }
    for (direction, retention) in [
        ("upload", config.retention.upload),
        ("download", config.retention.download),
    ] {
        let direction = response["retention"]["transfers"][direction]
            .as_object_mut()
            .expect("frozen transfer retention defaults must be an object");
        for (key, value) in [
            ("succeeded", retention.succeeded_minutes),
            ("errored", retention.errored_minutes),
            ("cancelled", retention.cancelled_minutes),
            ("failed", retention.failed_minutes),
        ] {
            if let Some(value) = value {
                direction.insert(key.to_owned(), serde_json::json!(value));
            } else {
                direction.remove(key);
            }
        }
    }
    if config.controller_profile == ControllerProfile::Native {
        response["filters"]["searchRetention"] = serde_json::json!({
            "maxAgeDays": config.search_retention.max_age_days,
            "maxCount": config.search_retention.max_count,
            "cleanupIntervalSeconds": config.search_retention.cleanup_interval.as_secs(),
        });
    }
    response["metrics"]["enabled"] = serde_json::json!(config.controller_metrics_enabled);
    response["metrics"]["url"] = serde_json::json!(config.controller_metrics_url);
    response["metrics"]["authentication"]["disabled"] =
        serde_json::json!(config.controller_metrics_auth_disabled);
    response["metrics"]["authentication"]["username"] =
        serde_json::json!(config.controller_metrics_username);
    response["metrics"]["authentication"]["password"] = serde_json::json!("*****");
    if config.controller_profile == ControllerProfile::Native {
        response["diagnostics"]["allowMemoryDump"] =
            serde_json::json!(config.controller_diagnostics_allow_memory_dump);
        response["diagnostics"]["allowRemoteDump"] =
            serde_json::json!(config.controller_diagnostics_allow_remote_dump);
    }
    response["directories"]["downloads"] =
        serde_json::json!(config.downloads_dir.display().to_string());
    response["directories"]["incomplete"] =
        serde_json::json!(config.incomplete_dir.display().to_string());
    let web_address_key = if config.controller_profile == ControllerProfile::Native {
        "address"
    } else {
        "ipAddress"
    };
    response["web"][web_address_key] = config
        .controller_http_address
        .as_ref()
        .map_or(serde_json::Value::Null, |address| {
            serde_json::Value::String(address.clone())
        });
    response["web"]["port"] = serde_json::json!(config.http_bind.port());
    if let Some(path) = config.controller_web.socket.as_ref() {
        response["web"]["socket"] = serde_json::json!(path.display().to_string());
    } else if config.controller_profile == ControllerProfile::Native {
        response["web"]["socket"] = serde_json::json!("");
    } else if let Some(web) = response["web"].as_object_mut() {
        web.remove("socket");
    }
    response["web"]["urlBase"] = serde_json::json!(config.controller_web.url_base);
    response["web"]["contentPath"] = serde_json::json!(config.controller_web.content_path_display);
    response["web"]["logging"] = serde_json::json!(config.controller_web.logging);
    response["web"]["https"]["disabled"] = serde_json::json!(config.controller_web.https.disabled);
    response["web"]["https"]["port"] = serde_json::json!(config
        .controller_web
        .https
        .binds
        .first()
        .map_or(5031, SocketAddr::port));
    if config.controller_profile == ControllerProfile::Legacy {
        if let Some(value) = config.controller_web.https.configured_ip_address.as_ref() {
            response["web"]["https"]["ipAddress"] = serde_json::json!(value);
        } else if let Some(https) = response["web"]["https"].as_object_mut() {
            https.remove("ipAddress");
        }
    }
    response["web"]["https"]["force"] = serde_json::json!(config.controller_web.https.force);
    response["web"]["https"]["certificate"]["pfx"] = config
        .controller_web
        .https
        .certificate_pfx
        .as_ref()
        .map_or_else(
            || {
                if config.controller_profile == ControllerProfile::Native {
                    serde_json::json!("")
                } else {
                    serde_json::Value::Null
                }
            },
            |path| serde_json::json!(path.display().to_string()),
        );
    response["web"]["https"]["certificate"]["password"] = serde_json::json!("*****");
    if config.controller_profile == ControllerProfile::Legacy
        && config
            .controller_web
            .https
            .certificate_pfx
            .as_ref()
            .is_none()
    {
        if let Some(certificate) = response["web"]["https"]["certificate"].as_object_mut() {
            certificate.remove("pfx");
            certificate.remove("password");
        }
    }
    response["web"]["authentication"]["apiKeys"] = serde_json::Value::Object(
        config
            .controller_api_keys
            .iter()
            .map(|(name, key)| {
                let mut value = serde_json::json!({
                    "key": "*****",
                    "role": key.role,
                    "cidr": key.cidr,
                });
                if config.controller_profile == ControllerProfile::Native {
                    value["scopes"] = serde_json::json!("*");
                }
                (name.clone(), value)
            })
            .collect(),
    );
    response["web"]["authentication"]["disabled"] = serde_json::json!(!config.auth_required);
    response["web"]["authentication"]["username"] =
        serde_json::json!(config.controller_web_auth_username);
    response["web"]["authentication"]["password"] = serde_json::json!("*****");
    response["web"]["authentication"]["jwt"]["key"] = serde_json::json!("*****");
    response["web"]["authentication"]["jwt"]["ttl"] =
        serde_json::json!(config.controller_web_jwt_ttl_millis);
    if config.controller_profile == ControllerProfile::Native {
        response["web"]["enforceSecurity"] =
            serde_json::json!(config.controller_web_enforce_security);
        response["web"]["allowRemoteNoAuth"] =
            serde_json::json!(config.controller_web_allow_remote_no_auth);
        if let Some(value) = config.controller_web_passthrough_allowed_cidrs.as_ref() {
            response["web"]["authentication"]["passthrough"]["allowedCidrs"] =
                serde_json::Value::String(value.clone());
        }
        response["web"]["maxRequestBodySize"] =
            serde_json::json!(config.controller_web_max_request_body_size);
        let cors = &config.controller_web_cors;
        response["web"]["cors"] = serde_json::json!({
            "enabled": cors.enabled,
            "allowCredentials": cors.allow_credentials,
            "allowedOrigins": cors.allowed_origins,
            "allowedHeaders": cors.allowed_headers,
            "allowedMethods": cors.allowed_methods,
        });
        let rate_limiting = config.controller_web_rate_limiting;
        response["web"]["rateLimiting"] = serde_json::json!({
            "enabled": rate_limiting.enabled,
            "apiPermitLimit": rate_limiting.api_permit_limit,
            "apiWindowSeconds": rate_limiting.api_window_seconds,
            "federationPermitLimit": rate_limiting.federation_permit_limit,
            "federationWindowSeconds": rate_limiting.federation_window_seconds,
            "meshGatewayPermitLimit": rate_limiting.mesh_gateway_permit_limit,
            "meshGatewayWindowSeconds": rate_limiting.mesh_gateway_window_seconds,
        });
    }
    response["soulseek"]["description"] = serde_json::json!(config.user_info_description);
    response["rooms"] = serde_json::json!(config.core_workflow.rooms);
    if let Some(picture) = config.user_info_picture.as_deref() {
        response["soulseek"]["picture"] = serde_json::json!(picture.display().to_string());
    } else if config.controller_profile == ControllerProfile::Native {
        response["soulseek"]["picture"] = serde_json::json!("");
    } else if let Some(soulseek) = response["soulseek"].as_object_mut() {
        soulseek.remove("picture");
    }
    response["soulseek"]["diagnosticLevel"] =
        serde_json::json!(config.soulseek_diagnostic_level.as_str());
    response["soulseek"]["distributedNetwork"] = serde_json::json!({
        "disabled": config.soulseek_distributed.disabled,
        "disableChildren": config.soulseek_distributed.disable_children,
        "childLimit": config.soulseek_distributed.child_limit,
        "logging": config.soulseek_distributed.logging,
    });
    if config.controller_profile == ControllerProfile::Native {
        response["soulseek"]["likedInterests"] =
            serde_json::json!(config.core_workflow.liked_interests);
        response["soulseek"]["hatedInterests"] =
            serde_json::json!(config.core_workflow.hated_interests);
    }
    response["soulseek"]["listenPort"] = serde_json::json!(config.listen_port);
    let connection = &config.soulseek_connection;
    response["soulseek"]["connection"]["buffer"]["read"] =
        serde_json::json!(connection.buffer_read);
    response["soulseek"]["connection"]["buffer"]["write"] =
        serde_json::json!(connection.buffer_write);
    response["soulseek"]["connection"]["buffer"]["transfer"] =
        serde_json::json!(connection.buffer_transfer);
    response["soulseek"]["connection"]["buffer"]["writeQueue"] =
        serde_json::json!(connection.buffer_write_queue);
    response["soulseek"]["connection"]["timeout"]["connect"] =
        serde_json::json!(connection.timeout_connect.as_millis());
    response["soulseek"]["connection"]["timeout"]["inactivity"] =
        serde_json::json!(connection.timeout_inactivity.as_millis());
    response["soulseek"]["connection"]["timeout"]["transfer"] =
        serde_json::json!(connection.timeout_transfer.as_millis());
    response["soulseek"]["connection"]["proxy"]["enabled"] =
        serde_json::json!(connection.proxy.enabled);
    let proxy = response["soulseek"]["connection"]["proxy"]
        .as_object_mut()
        .expect("frozen Soulseek proxy defaults must be an object");
    if config.controller_profile == ControllerProfile::Native
        || !connection.proxy.address.is_empty()
    {
        proxy.insert(
            "address".to_owned(),
            serde_json::json!(connection.proxy.address),
        );
    } else {
        proxy.remove("address");
    }
    if let Some(port) = connection.proxy.port {
        proxy.insert("port".to_owned(), serde_json::json!(port));
    } else {
        proxy.remove("port");
    }
    if config.controller_profile == ControllerProfile::Native
        || !connection.proxy.username.is_empty()
    {
        proxy.insert(
            "username".to_owned(),
            serde_json::json!(connection.proxy.username),
        );
    } else {
        proxy.remove("username");
    }
    if config.controller_profile == ControllerProfile::Native
        || !connection.proxy.password.is_empty()
    {
        proxy.insert("password".to_owned(), serde_json::json!("*****"));
    } else {
        proxy.remove("password");
    }
    response["soulseek"]["listenIpAddress"] = serde_json::json!(config
        .listener_bind
        .as_deref()
        .and_then(|value| value.parse::<SocketAddr>().ok())
        .map_or_else(|| "0.0.0.0".to_owned(), |address| address.ip().to_string()));
    if let Some((address, port)) = config
        .server_address
        .rsplit_once(':')
        .and_then(|(address, port)| port.parse::<u16>().ok().map(|port| (address, port)))
    {
        response["soulseek"]["address"] = serde_json::json!(address.trim_matches(['[', ']']));
        response["soulseek"]["port"] = serde_json::json!(port);
    }
    response["shares"]["directories"] = serde_json::json!(config
        .share_settings
        .directories
        .iter()
        .map(|directory| directory.raw.clone())
        .collect::<Vec<_>>());
    response["shares"]["filters"] = serde_json::json!(config.share_settings.filters);
    response["shares"]["cache"]["storageMode"] =
        serde_json::json!(config.share_settings.cache_storage_mode);
    response["shares"]["cache"]["workers"] = serde_json::json!(config.share_settings.cache_workers);
    if let Some(value) = config.share_settings.cache_retention {
        response["shares"]["cache"]["retention"] = serde_json::json!(value.as_secs() / 60);
    } else if let Some(cache) = response["shares"]["cache"].as_object_mut() {
        cache.remove("retention");
    }
    if config.controller_profile == ControllerProfile::Native {
        response["shares"]["probeMediaAttributes"] =
            serde_json::json!(config.share_settings.probe_media_attributes);
        response["wishlist"] = serde_json::json!({
            "enabled": config.core_workflow.wishlist.enabled,
            "intervalSeconds": config.core_workflow.wishlist.interval.as_secs(),
            "autoDownload": config.core_workflow.wishlist.auto_download,
            "maxResults": config.core_workflow.wishlist.max_results,
        });
        response["destinations"]["folders"] = serde_json::Value::Array(
            config
                .core_workflow
                .destinations
                .iter()
                .map(|destination| {
                    serde_json::json!({
                        "name": destination.name,
                        "path": destination.path.display().to_string(),
                        "default": destination.default,
                    })
                })
                .collect(),
        );
    }
    response["throttling"]["search"]["incoming"] = serde_json::json!({
        "concurrency": config.core_workflow.incoming_search.concurrency,
        "circuitBreaker": config.core_workflow.incoming_search.circuit_breaker,
        "responseFileLimit": config.core_workflow.incoming_search.response_file_limit,
    });
    response["filters"]["search"]["request"] =
        serde_json::json!(config.controller_search_request_filters);
    response["filters"]["download"]["exclude"] = serde_json::json!(config.download_filter.exclude);
    let blacklisted = serde_json::json!({
        "members": config.managed_blacklist.members,
        "patterns": config.managed_blacklist.patterns,
        "cidrs": config.managed_blacklist.cidr_values,
    });
    match config.controller_profile {
        ControllerProfile::Legacy => {
            response["transfers"]["upload"]["slots"] =
                serde_json::json!(config.transfer_upload.slots);
            response["transfers"]["upload"]["speedLimit"] =
                serde_json::json!(config.transfer_upload.speed_limit_kib);
            response["transfers"]["upload"]["limits"] =
                transfer_limits_options_json(&config.transfer_upload.limits);
            response["transfers"]["groups"] = transfer_groups_options_json(config, false);
            response["transfers"]["groups"]["blacklisted"] = blacklisted;
            response["transfers"]["download"] = transfer_download_options_json(config, false);
        }
        ControllerProfile::Native => {
            response["global"]["upload"]["slots"] = serde_json::json!(config.transfer_upload.slots);
            response["global"]["upload"]["speedLimit"] =
                serde_json::json!(config.transfer_upload.speed_limit_kib);
            response["global"]["limits"] =
                transfer_limits_options_json(&config.transfer_upload.limits);
            response["groups"] = transfer_groups_options_json(config, true);
            response["groups"]["blacklisted"] = blacklisted;
            let preserved_scheduled_limits =
                response["global"]["download"]["scheduledLimits"].clone();
            let preserved_cost_based =
                response["global"]["download"]["costBasedScheduling"].clone();
            response["global"]["download"] = transfer_download_options_json(config, true);
            response["global"]["download"]["scheduledLimits"] = preserved_scheduled_limits;
            response["global"]["download"]["costBasedScheduling"] = preserved_cost_based;
        }
    }
    let ftp_key = if config.controller_profile == ControllerProfile::Native {
        "integration"
    } else {
        "integrations"
    };
    response[ftp_key]["ftp"]["enabled"] = serde_json::json!(config.integrations.ftp.enabled);
    response[ftp_key]["ftp"]["port"] = serde_json::json!(config.integrations.ftp.port);
    response[ftp_key]["ftp"]["encryptionMode"] =
        serde_json::json!(config.integrations.ftp.encryption_mode);
    response[ftp_key]["ftp"]["ignoreCertificateErrors"] =
        serde_json::json!(config.integrations.ftp.ignore_certificate_errors);
    response[ftp_key]["ftp"]["remotePath"] = serde_json::json!(config.integrations.ftp.remote_path);
    response[ftp_key]["ftp"]["overwriteExisting"] =
        serde_json::json!(config.integrations.ftp.overwrite_existing);
    response[ftp_key]["ftp"]["connectionTimeout"] =
        serde_json::json!(config.integrations.ftp.connection_timeout);
    response[ftp_key]["ftp"]["retryAttempts"] =
        serde_json::json!(config.integrations.ftp.retry_attempts);
    if config.controller_profile == ControllerProfile::Native
        || !config.integrations.ftp.address.is_empty()
    {
        response[ftp_key]["ftp"]["address"] = serde_json::json!(config.integrations.ftp.address);
    }
    if config.controller_profile == ControllerProfile::Native
        || !config.integrations.ftp.username.is_empty()
    {
        response[ftp_key]["ftp"]["username"] = serde_json::json!(config.integrations.ftp.username);
    }
    if config.controller_profile == ControllerProfile::Native
        || !config.integrations.ftp.password.is_empty()
    {
        response[ftp_key]["ftp"]["password"] = serde_json::json!("*****");
    }
    response[ftp_key]["vpn"]["enabled"] = serde_json::json!(config.integrations.vpn.enabled);
    response[ftp_key]["vpn"]["portForwarding"] =
        serde_json::json!(config.integrations.vpn.port_forwarding);
    if let Some(vpn) = response[ftp_key]["vpn"].as_object_mut() {
        vpn.remove("selfHostedRelay");
    }
    response[ftp_key]["vpn"]["pollingInterval"] =
        serde_json::json!(config.integrations.vpn.polling_interval);
    response[ftp_key]["vpn"]["gluetun"]["version"] = serde_json::json!(1);
    response[ftp_key]["vpn"]["gluetun"]["timeout"] =
        serde_json::json!(config.integrations.vpn.gluetun.timeout);
    for (key, value) in [
        ("url", config.integrations.vpn.gluetun.url.as_str()),
        (
            "username",
            config.integrations.vpn.gluetun.username.as_str(),
        ),
    ] {
        if config.controller_profile == ControllerProfile::Native || !value.is_empty() {
            response[ftp_key]["vpn"]["gluetun"][key] = serde_json::json!(value);
        }
    }
    for (key, configured) in [
        (
            "password",
            !config.integrations.vpn.gluetun.password.is_empty(),
        ),
        (
            "apiKey",
            !config.integrations.vpn.gluetun.api_key.is_empty(),
        ),
    ] {
        if config.controller_profile == ControllerProfile::Native || configured {
            response[ftp_key]["vpn"]["gluetun"][key] = serde_json::json!("*****");
        }
    }
    response[ftp_key]["scripts"] =
        serde_json::to_value(&config.integrations.scripts).expect("script options must serialize");

    if config.controller_profile == ControllerProfile::Native {
        let advanced = &config.advanced_networking;
        let dht = if include_volatile_overlay {
            overlay.watched_dht.as_ref().unwrap_or(&advanced.dht)
        } else {
            &advanced.dht
        };
        let media = &config.media_services;
        response["feature"]["collectionsSharing"] =
            serde_json::json!(media.features.collections_sharing);
        response["feature"]["streaming"] = serde_json::json!(media.features.streaming);
        response["feature"]["streamingRelayFallback"] =
            serde_json::json!(media.features.streaming_relay_fallback);
        response["feature"]["meshParallelSearch"] =
            serde_json::json!(media.features.mesh_parallel_search);
        response["feature"]["meshPublishAvailability"] =
            serde_json::json!(media.features.mesh_publish_availability);
        response["feature"]["identityFriends"] = serde_json::json!(media.features.identity_friends);
        response["feature"]["solid"] = serde_json::json!(media.features.solid);
        response["feature"]["scenePodBridge"] = serde_json::json!(media.features.scene_pod_bridge);
        response["feature"]["scenePodBridgeOptions"]["proxyTransfers"] =
            serde_json::json!(media.features.scene_pod_bridge_proxy_transfers);
        response["feature"]["scenePodBridgeOptions"]["exportPodAvailability"] =
            serde_json::json!(media.features.scene_pod_bridge_export_pod_availability);
        response["feature"]["songId"] = serde_json::json!(media.features.song_id);
        response["feature"]["mesh"] = serde_json::json!(media.features.mesh);
        response["feature"]["dht"] = serde_json::json!(media.features.dht);
        response["feature"]["pods"] = serde_json::json!(media.features.pods);
        response["feature"]["socialFederation"] =
            serde_json::json!(media.features.social_federation);
        response["feature"]["virtualSoulfind"] = serde_json::json!(media.features.virtual_soulfind);
        response["feature"]["multiSourceDownloads"] =
            serde_json::json!(media.features.multi_source_downloads);
        response["player"]["externalVisualizer"]["enabled"] =
            serde_json::json!(media.external_visualizer.launch_enabled);
        response["player"]["externalVisualizer"]["arguments"] =
            serde_json::json!(media.external_visualizer.arguments);
        response["player"]["externalVisualizer"]["name"] =
            serde_json::json!(media.external_visualizer.name);
        if let Some(path) = media.external_visualizer.command.as_ref() {
            response["player"]["externalVisualizer"]["path"] = serde_json::json!(path);
        } else if let Some(player) = response["player"]["externalVisualizer"].as_object_mut() {
            player.remove("path");
        }
        if let Some(directory) = media.external_visualizer.working_directory.as_ref() {
            response["player"]["externalVisualizer"]["workingDirectory"] =
                serde_json::json!(directory);
        } else if let Some(player) = response["player"]["externalVisualizer"].as_object_mut() {
            player.remove("workingDirectory");
        }
        response["solid"]["allowInsecureHttp"] = serde_json::json!(media.solid.allow_insecure_http);
        response["solid"]["allowLocalhostForWebId"] =
            serde_json::json!(media.solid.allow_localhost_for_web_id);
        response["solid"]["maxFetchBytes"] = serde_json::json!(media.solid.max_fetch_bytes);
        response["solid"]["timeoutSeconds"] = serde_json::json!(media.solid.timeout.as_secs());
        response["solid"]["allowedHosts"] = serde_json::json!(media.solid.allowed_hosts);
        if let Some(client_id_url) = media.solid.client_id_url.clone() {
            response["solid"]["clientIdUrl"] = serde_json::Value::String(client_id_url);
        } else if let Some(solid) = response["solid"].as_object_mut() {
            solid.remove("clientIdUrl");
        }
        response["solid"]["redirectPath"] = serde_json::json!(media.solid.redirect_path);
        response["songId"]["maxConcurrentRuns"] =
            serde_json::json!(media.song_id_max_concurrent_runs);
        let bridge = &media.virtual_soulfind.bridge;
        response["virtualSoulfind"]["bridge"]["enabled"] = serde_json::json!(bridge.enabled);
        response["virtualSoulfind"]["bridge"]["port"] = serde_json::json!(bridge.port);
        response["virtualSoulfind"]["bridge"]["bindAddress"] =
            serde_json::json!(bridge.bind_address);
        response["virtualSoulfind"]["bridge"]["maxClients"] = serde_json::json!(bridge.max_clients);
        response["virtualSoulfind"]["bridge"]["requireAuth"] =
            serde_json::json!(bridge.require_auth);
        response["virtualSoulfind"]["bridge"]["maxRequestsPerMinute"] =
            serde_json::json!(bridge.max_requests_per_minute);
        response["virtualSoulfind"]["bridge"]["maxTransfersPerSession"] =
            serde_json::json!(bridge.max_transfers_per_session);
        if !bridge.password.is_empty() {
            response["virtualSoulfind"]["bridge"]["password"] = serde_json::json!("*****");
        }
        let disaster = &media.virtual_soulfind.disaster_mode;
        response["virtualSoulfind"]["disasterMode"]["auto"] = serde_json::json!(disaster.auto);
        response["virtualSoulfind"]["disasterMode"]["force"] = serde_json::json!(disaster.force);
        response["virtualSoulfind"]["disasterMode"]["unavailableThresholdMinutes"] =
            serde_json::json!(disaster.unavailable_threshold.as_secs() / 60);
        response["virtualSoulfind"]["disasterMode"]["enableGracefulDegradation"] =
            serde_json::json!(disaster.enable_graceful_degradation);
        response["virtualSoulfind"]["disasterMode"]["recoveryCheckIntervalMinutes"] =
            serde_json::json!(disaster.recovery_check_interval.as_secs() / 60);
        response["virtualSoulfind"]["disasterMode"]["recoveryHealthyChecksRequired"] =
            serde_json::json!(disaster.recovery_healthy_checks_required);
        response["dhtRendezvous"] = serde_json::json!({
            "advertisedOverlayPort": dht.advertised_overlay_port,
            "announceIntervalSeconds": dht.announce_interval.as_secs(),
            "bootstrapRouters": dht.bootstrap_routers,
            "bootstrapTimeoutSeconds": dht.bootstrap_timeout.as_secs(),
            "coldBootstrapTimeoutSeconds": dht.cold_bootstrap_timeout.as_secs(),
            "dhtPort": dht.dht_port,
            "discoveryIntervalSeconds": dht.discovery_interval.as_secs(),
            "effectiveOverlayPort": dht.effective_overlay_port(),
            "enablePeerDiversity": true,
            "enableStun": dht.enable_stun,
            "enableUpnp": dht.enable_upnp,
            "enableUsernameVerification": false,
            "enabled": dht.enabled,
            "lanOnly": dht.lan_only,
            "lanOnlyBootstrapTimeoutSeconds": dht.lan_only_bootstrap_timeout.as_secs(),
            "minNeighbors": dht.min_neighbors,
            "overlayPort": dht.overlay_port,
            "vpnPortSync": dht.vpn_port_sync,
            "vpnPortSyncMode": match dht.vpn_port_sync.as_str() {
                "primary" => "Primary",
                "target_port" => "TargetPort",
                _ => "Disabled",
            },
        });
        let signal_system = &advanced.signal_system;
        let format_ttl = |ttl: Duration| {
            let total_seconds = ttl.as_secs();
            let hours = total_seconds / 3_600;
            let minutes = (total_seconds % 3_600) / 60;
            let seconds = total_seconds % 60;
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        };
        response["signalSystem"] = serde_json::json!({
            "enabled": signal_system.enabled,
            "deduplicationCacheSize": signal_system.deduplication_cache_size,
            "defaultTtl": format_ttl(signal_system.default_ttl),
            "meshChannel": {
                "enabled": signal_system.mesh_channel.enabled,
                "priority": signal_system.mesh_channel.priority,
                "requireActiveSession": signal_system.mesh_channel.require_active_session,
            },
            "btExtensionChannel": {
                "enabled": signal_system.bt_extension_channel.enabled,
                "priority": signal_system.bt_extension_channel.priority,
                "requireActiveSession": signal_system.bt_extension_channel.require_active_session,
            },
        });
        response["relay"] = serde_json::json!({
            "enabled": advanced.relay.enabled,
            "mode": advanced.relay.mode,
            "controller": {
                "address": advanced.relay.controller.address,
                "ignoreCertificateErrors": advanced.relay.controller.ignore_certificate_errors,
                "pinnedSpki": advanced.relay.controller.pinned_spki,
                "apiKey": "*****",
                "secret": "*****",
                "downloads": advanced.relay.controller.downloads,
            },
            "agents": advanced.relay.agents.iter().map(|(name, agent)| {
                (name.clone(), serde_json::json!({
                    "instanceName": agent.instance_name,
                    "secret": "*****",
                    "cidr": agent.cidr,
                }))
            }).collect::<serde_json::Map<String, serde_json::Value>>(),
        });
        response["security"]["enabled"] = serde_json::json!(advanced.security.enabled);
        response["security"]["profile"] = serde_json::json!(advanced.security.profile);
        response["security"]["networkGuard"] = serde_json::json!({
            "enabled": advanced.security.network_guard.enabled,
            "maxConnectionsPerIp": advanced.security.network_guard.max_connections_per_ip,
            "maxGlobalConnections": advanced.security.network_guard.max_global_connections,
            "maxMessagesPerMinute": advanced.security.network_guard.max_messages_per_minute,
            "maxMessageSize": advanced.security.network_guard.max_message_size,
            "maxPendingRequestsPerIp": 10,
        });
        response["security"]["pathGuard"]["enabled"] =
            serde_json::json!(advanced.security.path_guard.enabled);
        response["security"]["pathGuard"]["maxPathLength"] =
            serde_json::json!(advanced.security.path_guard.max_path_length);
        response["security"]["pathGuard"]["maxPathDepth"] =
            serde_json::json!(advanced.security.path_guard.max_path_depth);
        response["security"]["contentSafety"] = serde_json::json!({
            "enabled": advanced.security.content_safety.enabled,
            "verifyMagicBytes": advanced.security.content_safety.verify_magic_bytes,
            "quarantineSuspicious": advanced.security.content_safety.quarantine_suspicious,
            "quarantineDirectory": advanced.security.content_safety.quarantine_directory.display().to_string(),
            "blockExecutables": advanced.security.content_safety.block_executables,
        });
        response["security"]["peerReputation"]["enabled"] =
            serde_json::json!(advanced.security.peer_reputation.enabled);
        response["security"]["peerReputation"]["trustedThreshold"] =
            serde_json::json!(advanced.security.peer_reputation.trusted_threshold);
        response["security"]["peerReputation"]["untrustedThreshold"] =
            serde_json::json!(advanced.security.peer_reputation.untrusted_threshold);
        response["security"]["violationTracker"]["enabled"] =
            serde_json::json!(advanced.security.violation_tracker.enabled);
        response["security"]["violationTracker"]["violationsBeforeAutoBan"] = serde_json::json!(
            advanced
                .security
                .violation_tracker
                .violations_before_auto_ban
        );
        response["security"]["violationTracker"]["baseBanDurationMinutes"] = serde_json::json!(
            advanced
                .security
                .violation_tracker
                .base_ban_duration
                .as_secs()
                / 60
        );
        response["global"]["download"]["completedPathTemplate"] =
            serde_json::json!(config.download_completed_path_template);
        response["soulseek"]["username"] =
            serde_json::json!(config.username.as_deref().unwrap_or_default());
        response["soulseek"]["password"] = serde_json::json!("*****");
        response["soulseek"]["obfuscation"]["enabled"] =
            serde_json::json!(config.obfuscation_enabled);
        response["soulseek"]["obfuscation"]["mode"] =
            serde_json::json!(config.obfuscation_mode.as_str());
        response["soulseek"]["obfuscation"]["listenPort"] =
            serde_json::json!(config.obfuscation_listen_port);
        response["soulseek"]["obfuscation"]["advertiseRegularPort"] =
            serde_json::json!(config.obfuscation_advertise_regular_port);
        response["soulseek"]["obfuscation"]["preferOutbound"] =
            serde_json::json!(config.obfuscation_prefer_outbound);
        response["soulseek"]["privateMessageAutoResponse"]["enabled"] =
            serde_json::json!(config.private_message_auto_response.enabled);
        response["soulseek"]["privateMessageAutoResponse"]["message"] =
            serde_json::json!(config.private_message_auto_response.message);
        response["soulseek"]["privateMessageAutoResponse"]["cooldownMinutes"] =
            serde_json::json!(config.private_message_auto_response.cooldown_minutes);
        response["integration"]["spotify"]["enabled"] =
            serde_json::json!(config.integrations.spotify.enabled);
        response["integration"]["spotify"]["clientId"] = serde_json::json!("*****");
        response["integration"]["spotify"]["clientSecret"] = serde_json::json!("*****");
        response["integration"]["spotify"]["redirectUri"] = serde_json::json!(config
            .integrations
            .spotify
            .redirect_uri
            .as_deref()
            .unwrap_or_default());
        response["integration"]["spotify"]["timeoutSeconds"] =
            serde_json::json!(config.integrations.spotify.timeout_seconds);
        response["integration"]["spotify"]["maxItemsPerImport"] =
            serde_json::json!(config.integrations.spotify.max_items_per_import);
        response["integration"]["spotify"]["market"] =
            serde_json::json!(config.integrations.spotify.market);
        response["integration"]["youTube"]["enabled"] =
            serde_json::json!(config.integrations.youtube.enabled);
        response["integration"]["youTube"]["apiKey"] = serde_json::json!("*****");
        response["integration"]["lastFm"]["enabled"] =
            serde_json::json!(config.integrations.lastfm.enabled);
        response["integration"]["lastFm"]["apiKey"] = serde_json::json!("*****");
        let musicbrainz_timeout_seconds = config.integrations.musicbrainz.timeout_seconds;
        let musicbrainz_timeout_seconds = if musicbrainz_timeout_seconds.fract() == 0.0 {
            serde_json::json!(musicbrainz_timeout_seconds as i64)
        } else {
            serde_json::json!(musicbrainz_timeout_seconds)
        };
        response["integration"]["musicBrainz"] = serde_json::json!({
            "baseUrl": config.integrations.musicbrainz.base_url,
            "retryAttempts": config.integrations.musicbrainz.retry_attempts,
            "timeout": format_timespan_hms(config.integrations.musicbrainz.timeout_seconds as i64),
            "timeoutSeconds": musicbrainz_timeout_seconds,
            "userAgent": config.integrations.musicbrainz.user_agent,
        });
        response["integration"]["ntfy"] = serde_json::json!({
            "enabled": config.integrations.ntfy.enabled,
            "url": config.integrations.ntfy.url,
            "accessToken": "*****",
            "notificationPrefix": config.integrations.ntfy.notification_prefix,
            "notifyOnPrivateMessage": config.integrations.ntfy.notify_on_private_message,
            "notifyOnRoomMention": config.integrations.ntfy.notify_on_room_mention,
        });
        response["integration"]["pushover"] = serde_json::json!({
            "enabled": config.integrations.pushover.enabled,
            "userKey": "*****",
            "token": "*****",
            "notificationPrefix": config.integrations.pushover.notification_prefix,
            "notifyOnPrivateMessage": config.integrations.pushover.notify_on_private_message,
            "notifyOnRoomMention": config.integrations.pushover.notify_on_room_mention,
        });
        response["integration"]["pushbullet"] = serde_json::json!({
            "enabled": config.integrations.pushbullet.enabled,
            "accessToken": "*****",
            "notificationPrefix": config.integrations.pushbullet.notification_prefix,
            "notifyOnPrivateMessage": config.integrations.pushbullet.notify_on_private_message,
            "notifyOnRoomMention": config.integrations.pushbullet.notify_on_room_mention,
            "retryAttempts": config.integrations.pushbullet.retry_attempts,
            "cooldownTime": config.integrations.pushbullet.cooldown_time,
        });
        response["integration"]["webhooks"] = serde_json::Value::Object(
            config.integrations.frozen_webhooks.iter().map(|(name, hook)| {
                (name.clone(), serde_json::json!({
                    "on": hook.on,
                    "call": {
                        "url": hook.call.url,
                        "headers": hook.call.headers.iter().map(|header| serde_json::json!({"name": header.name, "value": "*****"})).collect::<Vec<_>>(),
                        "ignoreCertificateErrors": hook.call.ignore_certificate_errors,
                    },
                    "timeout": hook.timeout,
                    "retry": {"attempts": hook.retry.attempts},
                }))
            }).collect()
        );
        response["integration"]["lidarr"]["enabled"] =
            serde_json::json!(config.integrations.lidarr.enabled);
        response["integration"]["lidarr"]["url"] = serde_json::json!(config
            .integrations
            .lidarr
            .url
            .as_deref()
            .unwrap_or_default());
        response["integration"]["lidarr"]["apiKey"] = serde_json::json!("*****");
        response["integration"]["lidarr"]["timeoutSeconds"] =
            serde_json::json!(config.integrations.lidarr.timeout_seconds);
        response["integration"]["lidarr"]["syncWantedToWishlist"] =
            serde_json::json!(config.integrations.lidarr.sync_wanted_to_wishlist);
        response["integration"]["lidarr"]["syncIntervalSeconds"] =
            serde_json::json!(config.integrations.lidarr.sync_interval_seconds);
        response["integration"]["lidarr"]["maxItemsPerSync"] =
            serde_json::json!(config.integrations.lidarr.max_items_per_sync);
        response["integration"]["lidarr"]["autoDownload"] =
            serde_json::json!(config.integrations.lidarr.auto_download);
        response["integration"]["lidarr"]["wishlistFilter"] =
            serde_json::json!(config.integrations.lidarr.wishlist_filter);
        response["integration"]["lidarr"]["wishlistMaxResults"] =
            serde_json::json!(config.integrations.lidarr.wishlist_max_results);
        response["integration"]["lidarr"]["autoImportCompleted"] =
            serde_json::json!(config.integrations.lidarr.auto_import_completed);
        response["integration"]["lidarr"]["importPathFrom"] =
            serde_json::json!(config.integrations.lidarr.import_path_from);
        response["integration"]["lidarr"]["importPathTo"] =
            serde_json::json!(config.integrations.lidarr.import_path_to);
        response["integration"]["lidarr"]["importMode"] =
            serde_json::json!(config.integrations.lidarr.import_mode);
        response["integration"]["lidarr"]["importReplaceExistingFiles"] =
            serde_json::json!(config.integrations.lidarr.import_replace_existing_files);
        if let Some(lidarr) = response["integration"]["lidarr"].as_object_mut() {
            lidarr.remove("deleteRejectedDownloads");
            lidarr.remove("blacklistRejectedDownloads");
        }
    } else if let Some(username) = config.username.as_deref() {
        response["soulseek"]["username"] = serde_json::json!(username);
        response["soulseek"]["password"] = serde_json::json!("*****");
    }

    let command_line_projection = response.clone();
    if include_volatile_overlay {
        if let Some(instance_name) = overlay.watched_instance_name.as_ref() {
            response["instanceName"] = serde_json::json!(instance_name);
        }
    }
    let yaml_effective = if include_volatile_overlay {
        overlay
            .watched_yaml_effective
            .as_ref()
            .unwrap_or(&overlay.yaml_effective)
    } else {
        &overlay.yaml_effective
    };
    if yaml_effective.is_object() {
        if config.controller_profile == ControllerProfile::Native {
            // These sections bind to dedicated runtime option types in
            // native profile; they are not arbitrary members of the main Options API.
            // The typed projection above owns the main-Options subsets, while
            // mesh/overlay/PodCore remain runtime-only configuration trees.
            let mut projected = yaml_effective.clone();
            if let Some(object) = projected.as_object_mut() {
                for key in [
                    "dhtRendezvous",
                    "relay",
                    "security",
                    "mesh",
                    "Mesh",
                    "overlay",
                    "overlayData",
                    "podCore",
                    "PodCore",
                    "feature",
                    "player",
                    "solid",
                    "songId",
                    "song_id",
                    "virtualSoulfind",
                ] {
                    object.remove(key);
                }
            }
            merge_json_objects(&mut response, &projected);
        } else {
            merge_json_objects(&mut response, yaml_effective);
        }
    }
    normalize_frozen_transfer_options_shape(&mut response, config.controller_profile);
    if config.controller_profile == ControllerProfile::Native {
        if let Some(integration) = response["integration"].as_object_mut() {
            integration.remove("youtube");
            integration.remove("lastfm");
            if let Some(webhooks) = integration
                .get_mut("webhooks")
                .and_then(serde_json::Value::as_object_mut)
            {
                let projected = std::mem::take(webhooks);
                for (name, mut webhook) in projected {
                    if let Some(call) = webhook
                        .get_mut("call")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        if let Some(value) = call.remove("ignore_certificate_errors") {
                            call.insert("ignoreCertificateErrors".to_owned(), value);
                        }
                        if let Some(headers) = call
                            .get_mut("headers")
                            .and_then(serde_json::Value::as_array_mut)
                        {
                            for header in headers {
                                header["value"] = serde_json::json!("*****");
                            }
                        }
                    }
                    let canonical_name = name
                        .chars()
                        .filter(|ch| ch.is_alphanumeric())
                        .flat_map(char::to_lowercase)
                        .collect::<String>();
                    webhooks.insert(canonical_name, webhook);
                }
            }
        }
    }
    if let Some(scripts) = response[ftp_key]["scripts"].as_object_mut() {
        let projected = std::mem::take(scripts);
        for (name, mut script) in projected {
            let canonical_name = name
                .chars()
                .filter(|ch| ch.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>();
            if config.controller_profile == ControllerProfile::Legacy {
                if let Some(run) = script
                    .get_mut("run")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    for key in ["command", "executable", "args"] {
                        if run.get(key).and_then(serde_json::Value::as_str) == Some("") {
                            run.remove(key);
                        }
                    }
                    if run.get("arglist").is_some_and(serde_json::Value::is_null) {
                        run.remove("arglist");
                    }
                }
            }
            scripts.insert(canonical_name, script);
        }
    }
    for (environment_name, pointer) in [
        ("SLSKD_BLACKLIST", "/blacklist/enabled"),
        ("SLSKD_BLACKLIST_FILE", "/blacklist/file"),
        ("SLSKD_SWAGGER", "/feature/swagger"),
        ("SLSKD_METRICS", "/metrics/enabled"),
        ("SLSKD_METRICS_URL", "/metrics/url"),
        ("SLSKD_METRICS_NO_AUTH", "/metrics/authentication/disabled"),
        ("SLSKD_METRICS_USERNAME", "/metrics/authentication/username"),
        ("SLSKD_METRICS_PASSWORD", "/metrics/authentication/password"),
        ("SLSKD_ALLOW_MEMORY_DUMP", "/diagnostics/allowMemoryDump"),
        ("SLSKD_ALLOW_REMOTE_DUMP", "/diagnostics/allowRemoteDump"),
        ("SLSKD_ENFORCE_SECURITY", "/web/enforceSecurity"),
        ("SLSKD_ALLOW_REMOTE_NO_AUTH", "/web/allowRemoteNoAuth"),
        ("SLSKR_AUTH_DISABLED", "/web/authentication/disabled"),
        ("SLSKD_USERNAME", "/web/authentication/username"),
        ("SLSKD_PASSWORD", "/web/authentication/password"),
        ("SLSKD_JWT_KEY", "/web/authentication/jwt/key"),
        ("SLSKD_JWT_TTL", "/web/authentication/jwt/ttl"),
        ("SLSKD_DEBUG", "/debug"),
        ("SLSKD_HEADLESS", "/headless"),
        ("SLSKD_REMOTE_CONFIGURATION", "/remoteConfiguration"),
        ("SLSKD_REMOTE_FILE_MANAGEMENT", "/remoteFileManagement"),
        ("SLSKD_INSTANCE_NAME", "/instanceName"),
        ("SLSKD_NO_CONFIG_WATCH", "/flags/noConfigWatch"),
        ("SLSKD_NO_CONNECT", "/flags/noConnect"),
        ("SLSKD_NO_LOGO", "/flags/noLogo"),
        ("SLSKD_NO_START", "/flags/noStart"),
        ("SLSKD_NO_VERSION_CHECK", "/flags/noVersionCheck"),
        ("SLSKD_EXPERIMENTAL", "/flags/experimental"),
        ("SLSKD_CASE_SENSITIVE_REGEX", "/flags/caseSensitiveRegEx"),
        ("SLSKD_NO_SHARE_SCAN", "/flags/noShareScan"),
        ("SLSKD_FORCE_SHARE_SCAN", "/flags/forceShareScan"),
        ("SLSKD_SHARED_DIR", "/shares/directories"),
        ("SLSKD_SHARE_FILTER", "/shares/filters"),
        ("SLSKD_SEARCH_REQUEST_FILTER", "/filters/search/request"),
        ("SLSKD_DOWNLOADS_DIR", "/directories/downloads"),
        ("SLSKD_INCOMPLETE_DIR", "/directories/incomplete"),
        ("SLSKD_HTTP_PORT", "/web/port"),
        ("SLSKD_HTTP_IP_ADDRESS", "/web/ipAddress"),
        ("SLSKD_HTTP_ADDRESS", "/web/address"),
        ("SLSKD_SLSK_ADDRESS", "/soulseek/address"),
        ("SLSKD_SLSK_PORT", "/soulseek/port"),
        ("SLSKD_SLSK_USERNAME", "/soulseek/username"),
        ("SLSKD_SLSK_PASSWORD", "/soulseek/password"),
        ("SLSKD_SLSK_LISTEN_IP_ADDRESS", "/soulseek/listenIpAddress"),
        ("SLSKD_SLSK_LISTEN_PORT", "/soulseek/listenPort"),
        ("SLSKD_SLSK_DESCRIPTION", "/soulseek/description"),
        ("SLSKD_SLSK_OBFUSCATION", "/soulseek/obfuscation/enabled"),
        ("SLSKD_SLSK_OBFUSCATION_MODE", "/soulseek/obfuscation/mode"),
        (
            "SLSKD_SLSK_OBFUSCATION_LISTEN_PORT",
            "/soulseek/obfuscation/listenPort",
        ),
        (
            "SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
            "/soulseek/obfuscation/advertiseRegularPort",
        ),
        (
            "SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND",
            "/soulseek/obfuscation/preferOutbound",
        ),
        (
            "SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
            "/global/download/completedPathTemplate",
        ),
        (
            "SLSKD_DOWNLOAD_COMPLETED_LAYOUT",
            "/global/download/completedLayout",
        ),
        (
            "SLSKD_DOWNLOAD_SLOTS",
            if config.controller_profile == ControllerProfile::Native {
                "/global/download/slots"
            } else {
                "/transfers/download/slots"
            },
        ),
        (
            "SLSKD_DOWNLOAD_SPEED_LIMIT",
            if config.controller_profile == ControllerProfile::Native {
                "/global/download/speedLimit"
            } else {
                "/transfers/download/speedLimit"
            },
        ),
        (
            "SLSKD_AUTO_REPLACE_STUCK",
            "/global/download/autoReplaceStuck",
        ),
        (
            "SLSKD_AUTO_REPLACE_THRESHOLD",
            "/global/download/autoReplaceThreshold",
        ),
        (
            "SLSKD_AUTO_REPLACE_INTERVAL",
            "/global/download/autoReplaceInterval",
        ),
        (
            "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
            "/soulseek/privateMessageAutoResponse/enabled",
        ),
        (
            "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
            "/soulseek/privateMessageAutoResponse/message",
        ),
        (
            "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
            "/soulseek/privateMessageAutoResponse/cooldownMinutes",
        ),
        ("SLSKD_VPN", "/integration/vpn/enabled"),
        (
            "SLSKD_VPN_PORT_FORWARDING",
            "/integration/vpn/portForwarding",
        ),
        (
            "SLSKD_VPN_SELF_HOSTED_RELAY",
            "/integration/vpn/selfHostedRelay",
        ),
        (
            "SLSKD_VPN_POLLING_INTERVAL",
            "/integration/vpn/pollingInterval",
        ),
        ("SLSKD_VPN_GLUETUN_URL", "/integration/vpn/gluetun/url"),
        (
            "SLSKD_VPN_GLUETUN_TIMEOUT",
            "/integration/vpn/gluetun/timeout",
        ),
        (
            "SLSKD_VPN_GLUETUN_USERNAME",
            "/integration/vpn/gluetun/username",
        ),
        (
            "SLSKD_VPN_GLUETUN_PASSWORD",
            "/integration/vpn/gluetun/password",
        ),
        (
            "SLSKD_VPN_GLUETUN_API_KEY",
            "/integration/vpn/gluetun/apiKey",
        ),
        ("SLSKD_VPN", "/integrations/vpn/enabled"),
        (
            "SLSKD_VPN_PORT_FORWARDING",
            "/integrations/vpn/portForwarding",
        ),
        (
            "SLSKD_VPN_SELF_HOSTED_RELAY",
            "/integrations/vpn/selfHostedRelay",
        ),
        (
            "SLSKD_VPN_POLLING_INTERVAL",
            "/integrations/vpn/pollingInterval",
        ),
        ("SLSKD_VPN_GLUETUN_URL", "/integrations/vpn/gluetun/url"),
        (
            "SLSKD_VPN_GLUETUN_TIMEOUT",
            "/integrations/vpn/gluetun/timeout",
        ),
        (
            "SLSKD_VPN_GLUETUN_USERNAME",
            "/integrations/vpn/gluetun/username",
        ),
        (
            "SLSKD_VPN_GLUETUN_PASSWORD",
            "/integrations/vpn/gluetun/password",
        ),
        (
            "SLSKD_VPN_GLUETUN_API_KEY",
            "/integrations/vpn/gluetun/apiKey",
        ),
        ("SLSKD_SPOTIFY", "/integration/spotify/enabled"),
        ("SLSKD_SPOTIFY_CLIENT_ID", "/integration/spotify/clientId"),
        (
            "SLSKD_SPOTIFY_CLIENT_SECRET",
            "/integration/spotify/clientSecret",
        ),
        (
            "SLSKD_SPOTIFY_REDIRECT_URI",
            "/integration/spotify/redirectUri",
        ),
        (
            "SLSKD_SPOTIFY_TIMEOUT",
            "/integration/spotify/timeoutSeconds",
        ),
        (
            "SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT",
            "/integration/spotify/maxItemsPerImport",
        ),
        ("SLSKD_SPOTIFY_MARKET", "/integration/spotify/market"),
        ("SLSKD_LIDARR", "/integration/lidarr/enabled"),
        ("SLSKD_LIDARR_URL", "/integration/lidarr/url"),
        ("SLSKD_LIDARR_API_KEY", "/integration/lidarr/apiKey"),
        ("SLSKD_LIDARR_TIMEOUT", "/integration/lidarr/timeoutSeconds"),
        (
            "SLSKD_LIDARR_SYNC_WANTED",
            "/integration/lidarr/syncWantedToWishlist",
        ),
        (
            "SLSKD_LIDARR_SYNC_INTERVAL",
            "/integration/lidarr/syncIntervalSeconds",
        ),
        (
            "SLSKD_LIDARR_SYNC_MAX_ITEMS",
            "/integration/lidarr/maxItemsPerSync",
        ),
        (
            "SLSKD_LIDARR_AUTO_DOWNLOAD",
            "/integration/lidarr/autoDownload",
        ),
        (
            "SLSKD_LIDARR_WISHLIST_FILTER",
            "/integration/lidarr/wishlistFilter",
        ),
        (
            "SLSKD_LIDARR_WISHLIST_MAX_RESULTS",
            "/integration/lidarr/wishlistMaxResults",
        ),
        (
            "SLSKD_LIDARR_AUTO_IMPORT_COMPLETED",
            "/integration/lidarr/autoImportCompleted",
        ),
        (
            "SLSKD_LIDARR_IMPORT_PATH_FROM",
            "/integration/lidarr/importPathFrom",
        ),
        (
            "SLSKD_LIDARR_IMPORT_PATH_TO",
            "/integration/lidarr/importPathTo",
        ),
        ("SLSKD_LIDARR_IMPORT_MODE", "/integration/lidarr/importMode"),
        (
            "SLSKD_LIDARR_IMPORT_REPLACE_EXISTING",
            "/integration/lidarr/importReplaceExistingFiles",
        ),
        (
            "SLSKD_LIDARR_DELETE_REJECTED_DOWNLOADS",
            "/integration/lidarr/deleteRejectedDownloads",
        ),
        (
            "SLSKD_LIDARR_BLACKLIST_REJECTED_DOWNLOADS",
            "/integration/lidarr/blacklistRejectedDownloads",
        ),
        (
            "SLSKD_UPLOAD_SLOTS",
            if config.controller_profile == ControllerProfile::Native {
                "/global/upload/slots"
            } else {
                "/transfers/upload/slots"
            },
        ),
        (
            "SLSKD_UPLOAD_SPEED_LIMIT",
            if config.controller_profile == ControllerProfile::Native {
                "/global/upload/speedLimit"
            } else {
                "/transfers/upload/speedLimit"
            },
        ),
    ] {
        if overlay
            .command_line_environment
            .contains_key(environment_name)
        {
            if let (Some(source), Some(destination)) = (
                command_line_projection.pointer(pointer),
                response.pointer_mut(pointer),
            ) {
                *destination = source.clone();
            }
        }
    }
    if include_volatile_overlay && overlay.effective.is_object() {
        merge_json_objects(&mut response, &overlay.effective);
    }
    if include_volatile_overlay {
        if let Some(swagger) = overlay.watched_controller_swagger {
            response["feature"]["swagger"] = serde_json::json!(swagger);
        }
    }
    if let Some(directories) = overlay.watched_share_directories.as_ref() {
        response["shares"]["directories"] = serde_json::json!(directories);
    }
    if let Some(gluetun) = response[ftp_key]["vpn"]["gluetun"].as_object_mut() {
        gluetun.remove("auth");
    }
    for pointer in [
        "/web/authentication/password",
        "/web/authentication/jwt/key",
        "/web/https/certificate/password",
        "/metrics/authentication/password",
        "/soulseek/password",
    ] {
        if let Some(secret) = response.pointer_mut(pointer) {
            *secret = serde_json::json!("*****");
        }
    }
    if let Some(api_keys) = response
        .pointer_mut("/web/authentication/apiKeys")
        .and_then(serde_json::Value::as_object_mut)
    {
        for api_key in api_keys.values_mut() {
            api_key["key"] = serde_json::json!("*****");
        }
    }
    response.to_string()
}
