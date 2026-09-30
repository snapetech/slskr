use super::*;

#[test]
fn controller_profile_is_explicit_bounded_and_projected() {
    let default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("default controller compatibility target");
    assert_eq!(
        default.controller_profile,
        crate::config::ControllerProfile::Native
    );

    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd controller compatibility target");
    assert_eq!(
        slskd.controller_profile,
        crate::config::ControllerProfile::Legacy
    );
    assert!(slskd
        .sanitized_json()
        .contains("\"controller_profile\":\"legacy\""));

    let file = crate::config::FileConfig {
        compatibility: crate::config::CompatibilityFileConfig {
            profile: Some("slskd".to_owned()),
            ..Default::default()
        },
        ..Default::default()
    };
    let from_file = crate::config::AppConfig::from_layers(
        None,
        file,
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("file controller compatibility target");
    assert_eq!(
        from_file.controller_profile,
        crate::config::ControllerProfile::Legacy
    );

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "auto"),
    )
    .expect_err("ambiguous compatibility target must fail");
    assert!(error.contains("must be legacy or native"));
}

#[test]
fn runtime_intervals_reject_zero_and_unrepresentable_values() {
    for name in [
        "SLSKR_RECONNECT_SECONDS",
        "SLSKR_PING_SECONDS",
        "SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS",
    ] {
        for value in ["0", &u64::MAX.to_string()] {
            let env = MapEnv::default().with(name, value);
            let error = crate::config::AppConfig::from_layers(
                None,
                crate::config::FileConfig::default(),
                &env,
            )
            .expect_err("invalid runtime interval must fail at startup");
            assert!(
                error.contains("greater than zero") || error.contains("timer range"),
                "{name}={value}: {error}"
            );
        }
    }
}

#[test]
fn controller_headless_defaults_false_and_honors_environment() {
    let default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("headless default");
    assert!(!default.controller_headless);

    let headless = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_HEADLESS", "true"),
    )
    .expect("headless environment override");
    assert!(headless.controller_headless);
}

#[test]
fn controller_startup_flags_default_false_and_honor_environment() {
    let default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("startup flag defaults");
    assert!(!default.controller_no_logo);
    assert!(!default.controller_no_start);
    assert!(!default.controller_no_version_check);
    assert!(!default.controller_experimental);
    assert!(!default.controller_case_sensitive_regex);
    assert!(default.controller_search_request_filters.is_empty());
    assert!(default.share_settings.filters.is_empty());
    assert!(!default.controller_no_share_scan);
    assert!(!default.controller_force_share_scan);

    let configured = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_NO_LOGO", "true")
            .with("SLSKD_NO_START", "true")
            .with("SLSKD_NO_VERSION_CHECK", "true")
            .with("SLSKD_EXPERIMENTAL", "true")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true")
            .with("SLSKD_SEARCH_REQUEST_FILTER", "first;second")
            .with("SLSKD_SHARE_FILTER", "secret;private")
            .with("SLSKD_NO_SHARE_SCAN", "true")
            .with("SLSKD_FORCE_SHARE_SCAN", "true"),
    )
    .expect("startup environment flags");
    assert!(configured.controller_no_logo);
    assert!(configured.controller_no_start);
    assert!(configured.controller_no_version_check);
    assert!(configured.controller_experimental);
    assert!(configured.controller_case_sensitive_regex);
    assert_eq!(
        configured.controller_search_request_filters,
        ["first", "second"]
    );
    assert_eq!(configured.share_settings.filters, ["secret", "private"]);
    assert!(configured.controller_no_share_scan);
    assert!(configured.controller_force_share_scan);
}

#[test]
fn daemon_foundation_contracts_load_as_one_runtime_policy() {
    let root = std::env::temp_dir().join(format!(
        "slskr-daemon-foundation-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let content_name = format!("slskr-foundation-wwwroot-{}", std::process::id());
    let content = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(&content_name);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&content).unwrap();
    let socket = root.join("slskr.sock");
    let socket_yaml = socket.display().to_string().replace('\\', "/");
    std::fs::write(
        root.join("slskd.yml"),
        format!(
            r#"flags:
  force_migrations: true
  legacy_windows_tcp_keepalive: true
  log_sql: true
  log_unobserved_exceptions: true
  optimistic_relay_file_info: true
  volatile: true
logger:
  disk: true
  loki: https://loki.example
  no_color: true
permissions:
  file:
    mode: "0640"
telemetry:
  tracing:
    enabled: true
    exporter: jaeger
    jaeger_endpoint: collector.example
    jaeger_port: 4318
    otlp_endpoint: https://otlp.example
retention:
  search: 10
  logs: 9
  files:
    complete: 30
    incomplete: 31
  transfers:
    upload: {{succeeded: 5, errored: 6, cancelled: 7, failed: 8}}
    download: {{succeeded: 9, errored: 10, cancelled: 11, failed: 12}}
filters:
  search_retention:
    max_age_days: 4
    max_count: 77
    cleanup_interval_seconds: 3600
web:
  socket: '{}'
  url_base: /slsk
  content_path: "{}"
  logging: true
  https:
    disabled: true
    port: 5443
    force: true
  authentication:
    api_keys:
      operator:
        key: 0123456789abcdef
        role: readwrite
        cidr: 127.0.0.1/32
"#,
            socket_yaml, content_name,
        ),
    )
    .unwrap();
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKD_APP_DIR", root.to_str().unwrap()),
    )
    .unwrap();
    assert!(config.daemon_flags.force_migrations);
    assert!(config.daemon_flags.legacy_windows_tcp_keepalive);
    assert!(config.daemon_flags.log_sql);
    assert!(config.daemon_flags.log_unobserved_exceptions);
    assert!(config.daemon_flags.optimistic_relay_file_info);
    assert!(config.daemon_flags.volatile);
    assert_eq!(config.logger.loki.as_deref(), Some("https://loki.example"));
    assert!(config.logger.disk && config.logger.no_color);
    assert_eq!(config.permissions_file_mode.as_deref(), Some("0640"));
    assert!(config.telemetry_tracing.enabled);
    assert_eq!(config.telemetry_tracing.exporter, "jaeger");
    assert_eq!(config.telemetry_tracing.jaeger_port, Some(4318));
    assert_eq!(config.retention.search_minutes, Some(10));
    assert_eq!(config.retention.download.failed_minutes, Some(12));
    assert_eq!(config.search_retention.max_count, 77);
    assert_eq!(config.search_retention.cleanup_interval.as_secs(), 3600);
    assert_eq!(
        config.controller_web.socket.as_deref(),
        Some(socket.as_path())
    );
    assert_eq!(config.controller_web.url_base, "/slsk");
    assert_eq!(config.controller_web.content_path, content);
    assert_eq!(config.controller_web.content_path_display, content_name);
    assert!(config.controller_web.logging);
    assert!(config.controller_web.https.disabled);
    assert!(config.controller_web.https.force);
    assert_eq!(config.controller_web.https.binds[0].port(), 5443);
    let key = &config.controller_api_keys["operator"];
    assert_eq!(key.role, "readwrite");
    assert!(key.cidrs[0].contains("127.0.0.1".parse().unwrap()));
    std::fs::remove_dir_all(&content).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn daemon_foundation_validation_rejects_unsafe_values() {
    for (name, value) in [
        ("SLSKD_FILE_PERMISSION_MODE", "888"),
        ("SLSKR_RETENTION_LOGS", "0"),
        ("SLSKR_RETENTION_SEARCH", "4"),
        ("SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL", "3599"),
        ("SLSKD_TELEMETRY_TRACING_EXPORTER", "zipkin"),
    ] {
        crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("invalid daemon foundation setting must fail startup");
    }
}

#[test]
fn core_workflow_contracts_load_as_one_runtime_policy() {
    let root = std::env::temp_dir().join(format!(
        "slskr-core-workflow-config-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let destination_path = root.join("downloads").join("music");
    let destination_yaml = destination_path.display().to_string().replace('\\', "/");
    std::fs::write(
        root.join("slskd.yml"),
        format!(
            r#"rooms: [Ambient, Jazz, ambient]
soulseek:
  liked_interests: [Aphex Twin, Ambient]
  hated_interests: [spam]
destinations:
  folders:
    - name: Music
      path: '{}'
      default: true
shares:
  cache:
    storage_mode: disk
    workers: 3
    retention: 120
  probe_media_attributes: false
wishlist:
  enabled: false
  interval_seconds: 600
  auto_download: true
  max_results: 250
throttling:
  search:
    incoming:
      concurrency: 4
      circuit_breaker: 600
      response_file_limit: 700
"#,
            destination_yaml,
        ),
    )
    .unwrap();
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    assert_eq!(config.core_workflow.rooms, ["Ambient", "Jazz"]);
    assert_eq!(
        config.core_workflow.liked_interests,
        ["Aphex Twin", "Ambient"]
    );
    assert_eq!(config.core_workflow.hated_interests, ["spam"]);
    assert_eq!(config.core_workflow.destinations.len(), 1);
    assert!(config.core_workflow.destinations[0].default);
    assert_eq!(config.core_workflow.wishlist.interval.as_secs(), 600);
    assert!(config.core_workflow.wishlist.auto_download);
    assert_eq!(config.core_workflow.wishlist.max_results, 250);
    assert_eq!(config.core_workflow.incoming_search.concurrency, 4);
    assert_eq!(config.core_workflow.incoming_search.circuit_breaker, 600);
    assert_eq!(
        config.core_workflow.incoming_search.response_file_limit,
        700
    );
    assert_eq!(config.share_settings.cache_storage_mode, "disk");
    assert_eq!(config.share_settings.cache_workers, 3);
    assert_eq!(
        config.share_settings.cache_retention.unwrap().as_secs(),
        7_200
    );
    assert!(!config.share_settings.probe_media_attributes);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn core_workflow_validation_rejects_out_of_range_values() {
    for (name, value) in [
        ("SLSKD_SHARE_CACHE_WORKERS", "0"),
        ("SLSKD_SHARE_CACHE_RETENTION", "59"),
        ("SLSKD_WISHLIST_INTERVAL", "299"),
        ("SLSKD_WISHLIST_MAX_RESULTS", "9"),
        ("SLSKD_THROTTLING_SEARCH_INCOMING_CONCURRENCY", "0"),
        ("SLSKD_THROTTLING_SEARCH_INCOMING_CIRCUIT_BREAKER", "99"),
        ("SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT", "99"),
    ] {
        crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("invalid core workflow setting must fail startup");
    }
}

#[test]
fn signal_system_environment_layers_and_bounds_match_slskdn() {
    let file = crate::config::FileConfig {
        signal_system: crate::config::SignalSystemFileConfig {
            enabled: Some(false),
            deduplication_cache_size: Some(2_000),
            default_ttl: Some(crate::config::SignalDurationFileValue::Text(
                "00:06:00".to_owned(),
            )),
            mesh_channel: crate::config::SignalChannelFileConfig {
                enabled: Some(false),
                priority: Some(4),
                require_active_session: Some(true),
            },
            bt_extension_channel: crate::config::SignalChannelFileConfig {
                enabled: Some(false),
                priority: Some(5),
                require_active_session: Some(false),
            },
        },
        ..crate::config::FileConfig::default()
    };
    let config = crate::config::AppConfig::from_layers(
        None,
        file,
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_SIGNALSYSTEM_ENABLED", "true")
            .with("SLSKD_SIGNALSYSTEM_DEDUPLICATIONCACHESIZE", "4096")
            .with("SLSKD_SIGNALSYSTEM_DEFAULTTTL", "00:08:30")
            .with("SLSKD_SIGNALSYSTEM_MESHCHANNEL_ENABLED", "true")
            .with("SLSKD_SIGNALSYSTEM_MESHCHANNEL_PRIORITY", "2")
            .with("SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_ENABLED", "true"),
    )
    .expect("SignalSystem environment overrides");
    let signal = &config.advanced_networking.signal_system;
    assert!(signal.enabled);
    assert_eq!(signal.deduplication_cache_size, 4_096);
    assert_eq!(signal.default_ttl.as_secs(), 510);
    assert!(signal.mesh_channel.enabled);
    assert_eq!(signal.mesh_channel.priority, 2);
    assert!(signal.mesh_channel.require_active_session);
    assert!(signal.bt_extension_channel.enabled);
    assert_eq!(signal.bt_extension_channel.priority, 5);

    for (name, value) in [
        ("SLSKD_SIGNALSYSTEM_DEDUPLICATIONCACHESIZE", "99"),
        ("SLSKD_SIGNALSYSTEM_DEFAULTTTL", "00:00:00"),
        ("SLSKD_SIGNALSYSTEM_MESHCHANNEL_PRIORITY", "11"),
        ("SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_PRIORITY", "0"),
    ] {
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with(name, value),
        )
        .expect_err("invalid SignalSystem setting must fail startup");
        assert!(error.contains("SignalSystem"), "{name}={value}: {error}");
    }
}
