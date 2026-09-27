use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Default)]
struct MapEnv {
    values: BTreeMap<String, String>,
}

impl MapEnv {
    fn with(mut self, name: &str, value: &str) -> Self {
        self.values.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl super::ConfigEnv for MapEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

#[test]
fn mesh_gateway_settings_match_frozen_defaults_and_validation() {
    let disabled =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .unwrap();
    assert!(!disabled.mesh_gateway.enabled);
    assert!(disabled.mesh_gateway.allowed_services.is_empty());
    assert_eq!(disabled.mesh_gateway.max_request_body_bytes, 1_048_576);
    assert_eq!(disabled.mesh_gateway.request_timeout_seconds, 30);

    let enabled = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods; shadow-index")
            .with("SLSKD_MESH_GATEWAY_MAX_REQUEST_BODY_BYTES", "4096")
            .with("SLSKD_MESH_GATEWAY_REQUEST_TIMEOUT_SECONDS", "7"),
    )
    .unwrap();
    assert!(enabled.mesh_gateway.enabled);
    assert_eq!(
        enabled.mesh_gateway.allowed_services,
        vec!["pods", "shadow-index"]
    );
    assert_eq!(enabled.mesh_gateway.max_request_body_bytes, 4096);
    assert_eq!(enabled.mesh_gateway.request_timeout_seconds, 7);

    let file = serde_yaml::from_str::<super::FileConfig>(
        "MeshGateway:\n  Enabled: true\n  AllowedServices: [pods]\n  MaxRequestBodyBytes: 8192\n",
    )
    .unwrap();
    let from_file = super::AppConfig::from_layers(None, file, &MapEnv::default()).unwrap();
    assert!(from_file.mesh_gateway.enabled);
    assert_eq!(from_file.mesh_gateway.allowed_services, vec!["pods"]);
    assert_eq!(from_file.mesh_gateway.max_request_body_bytes, 8192);

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKD_MESH_GATEWAY_ENABLED", "true"),
    )
    .expect_err("enabled gateway without services must fail validation");
    assert!(error.contains("AllowedServices"), "{error}");

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKD_MESH_GATEWAY_BIND_ADDRESS", "0.0.0.0"),
    )
    .expect_err("remote gateway without an API key must fail validation");
    assert!(error.contains("ApiKey"), "{error}");

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKD_MESH_GATEWAY_BIND_ADDRESS", "0.0.0.0")
            .with("SLSKD_MESH_GATEWAY_API_KEY", "gateway-key"),
    )
    .expect_err("remote gateway without risk acknowledgment must fail validation");
    assert!(error.contains("IUnderstandTheRisk"), "{error}");

    let remote = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKD_MESH_GATEWAY_BIND_ADDRESS", "0.0.0.0")
            .with("SLSKD_MESH_GATEWAY_API_KEY", "gateway-key")
            .with("SLSKD_MESH_GATEWAY_I_UNDERSTAND_THE_RISK", "true"),
    )
    .expect("remote gateway with required security settings");
    assert_eq!(remote.mesh_gateway.api_key.as_deref(), Some("gateway-key"));
    assert!(remote.mesh_gateway.i_understand_the_risk);
}

#[test]
fn controller_web_max_request_body_size_matches_native_layers_and_bounds() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-body-limit-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        "web:\n  max_request_body_size: 7340032\n",
    )
    .unwrap();

    let yaml = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKD_WEB_MAX_REQUEST_BODY_SIZE", "6291456"),
    )
    .unwrap();
    assert_eq!(yaml.controller_web_max_request_body_size, 7 * 1024 * 1024);
    std::fs::remove_dir_all(root).unwrap();

    let slskdn =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .unwrap();
    assert_eq!(
        slskdn.controller_web_max_request_body_size,
        10 * 1024 * 1024
    );
    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .unwrap();
    assert_eq!(
        slskd.controller_web_max_request_body_size,
        crate::http_server::BODY_SIZE_LIMIT
    );

    for invalid in ["0", "-1", "2147483648"] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKD_WEB_MAX_REQUEST_BODY_SIZE", invalid),
        )
        .expect_err("out-of-range request body limit must fail startup");
        assert!(error.contains("web.max_request_body_size"), "{error}");
    }
}

#[test]
fn controller_web_cors_reads_frozen_yaml_and_defaults() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-cors-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            "web:\n  cors:\n    enabled: true\n    allow_credentials: true\n    allowed_origins: [https://one.example, https://two.example]\n    allowed_headers: [X-One, X-Two]\n    allowed_methods: [GET, POST]\n",
        )
        .unwrap();

    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKD_WEB_CORS_ENABLED", "false")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "https://ignored.example"),
    )
    .unwrap();
    assert_eq!(
        config.controller_web_cors,
        super::ControllerWebCorsSettings {
            enabled: true,
            allow_credentials: true,
            allowed_origins: vec![
                "https://one.example".to_owned(),
                "https://two.example".to_owned(),
            ],
            allowed_headers: vec!["X-One".to_owned(), "X-Two".to_owned()],
            allowed_methods: vec!["GET".to_owned(), "POST".to_owned()],
        }
    );
    std::fs::remove_dir_all(root).unwrap();

    let defaults =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .unwrap();
    assert_eq!(
        defaults.controller_web_cors,
        super::ControllerWebCorsSettings::default()
    );

    let unsafe_cors = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_ENFORCE_SECURITY", "true")
            .with("SLSKD_WEB_CORS_ENABLED", "true")
            .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "*"),
    )
    .unwrap();
    let error = unsafe_cors
        .validate_controller_startup_hardening()
        .expect_err("enforced credentialed wildcard CORS must fail startup");
    assert!(error.contains("CorsCredentialsWithWildcard"), "{error}");

    let enforced_explicit = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_ENFORCE_SECURITY", "true")
            .with("SLSKD_WEB_CORS_ENABLED", "true")
            .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "https://allowed.example"),
    )
    .expect("explicit credentialed CORS is valid under enforcement");
    assert!(enforced_explicit.controller_web_enforce_security);
}

#[test]
fn controller_no_auth_passthrough_reads_yaml_and_enforces_remote_cidrs() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-passthrough-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            "web:\n  allow_remote_no_auth: true\n  authentication:\n    disabled: true\n    passthrough:\n      allowed_cidrs: 192.0.2.0/24,invalid\n",
        )
        .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
    )
    .unwrap();
    assert!(!config.auth_required);
    assert!(config.controller_web_allow_remote_no_auth);
    assert_eq!(
        config.controller_web_passthrough_allowed_cidrs.as_deref(),
        Some("192.0.2.0/24,invalid")
    );
    assert!(config.controller_passthrough_allows(Some("127.0.0.1:1".parse().unwrap())));
    assert!(config.controller_passthrough_allows(Some("192.0.2.44:1".parse().unwrap())));
    assert!(!config.controller_passthrough_allows(Some("198.51.100.1:1".parse().unwrap())));
    assert!(!config.controller_passthrough_allows(None));
    std::fs::remove_dir_all(root).unwrap();

    let nonloopback_config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ENFORCE_SECURITY", "true"),
    )
    .unwrap();
    let nonloopback = nonloopback_config
        .validate_controller_startup_hardening()
        .expect_err("enforced non-loopback no-auth bind must fail");
    assert!(
        nonloopback.contains("AuthDisabledNonLoopback"),
        "{nonloopback}"
    );

    let missing_cidrs_config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ENFORCE_SECURITY", "true")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true"),
    )
    .unwrap();
    let missing_cidrs = missing_cidrs_config
        .validate_controller_startup_hardening()
        .expect_err("enforced remote no-auth without CIDRs must fail");
    assert!(
        missing_cidrs.contains("RemoteNoAuthWithoutCidrs"),
        "{missing_cidrs}"
    );
}

#[test]
fn controller_diagnostics_dump_reads_yaml_and_enforces_no_auth_hardening() {
    let root = std::env::temp_dir().join(format!(
        "slskr-diagnostics-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        "diagnostics:\n  allow_memory_dump: true\n  allow_remote_dump: true\n",
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    assert!(config.controller_diagnostics_allow_memory_dump);
    assert!(config.controller_diagnostics_allow_remote_dump);
    let environment_override = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_ALLOW_MEMORY_DUMP", "false")
            .with("SLSKD_ALLOW_REMOTE_DUMP", "false"),
    )
    .unwrap();
    assert!(environment_override.controller_diagnostics_allow_memory_dump);
    assert!(environment_override.controller_diagnostics_allow_remote_dump);
    std::fs::remove_file(root.join("slskd.yml")).unwrap();
    let environment_only = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_ALLOW_MEMORY_DUMP", "true")
            .with("SLSKD_ALLOW_REMOTE_DUMP", "true"),
    )
    .unwrap();
    assert!(environment_only.controller_diagnostics_allow_memory_dump);
    assert!(environment_only.controller_diagnostics_allow_remote_dump);
    std::fs::remove_dir_all(root).unwrap();

    let unsafe_dump = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ENFORCE_SECURITY", "true")
            .with("SLSKD_ALLOW_MEMORY_DUMP", "true"),
    )
    .unwrap();
    let error = unsafe_dump
        .validate_controller_startup_hardening()
        .expect_err("enforced memory dump with disabled authentication must fail");
    assert!(error.contains("MemoryDumpWithAuthDisabled"), "{error}");
}

#[test]
fn controller_remaining_hardening_rules_match_frozen_startup_policy() {
    let weak_metrics = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_ENFORCE_SECURITY", "true")
            .with("SLSKD_METRICS", "true")
            .with("SLSKD_METRICS_USERNAME", "slskd")
            .with("SLSKD_METRICS_PASSWORD", " "),
    )
    .expect_err("whitespace metrics password must fail options validation");
    assert!(
        weak_metrics.contains("metrics authentication password must be configured"),
        "{weak_metrics}"
    );

    let root = std::env::temp_dir().join(format!(
        "slskr-hash-from-audio-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        "flags:\n  hash_from_audio_file_enabled: true\n",
    )
    .unwrap();
    let hash_from_audio = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    assert!(hash_from_audio.controller_hash_from_audio_file_enabled);
    let error = hash_from_audio
        .validate_controller_startup_hardening()
        .expect_err("unsupported audio hash flag must always fail startup");
    assert!(error.contains("HashFromAudioFileEnabled"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn controller_web_rate_limiting_reads_frozen_yaml_and_profile_defaults() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-rate-limit-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            "web:\n  rate_limiting:\n    enabled: false\n    api_permit_limit: 201\n    api_window_seconds: 0\n    federation_permit_limit: 31\n    federation_window_seconds: 61\n    mesh_gateway_permit_limit: 62\n    mesh_gateway_window_seconds: 63\n",
        )
        .unwrap();

    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKD_WEB_API_PERMIT_LIMIT", "202"),
    )
    .unwrap();
    assert_eq!(
        config.controller_web_rate_limiting,
        super::ControllerWebRateLimitingSettings {
            enabled: false,
            api_permit_limit: 201,
            api_window_seconds: 0,
            federation_permit_limit: 31,
            federation_window_seconds: 61,
            mesh_gateway_permit_limit: 62,
            mesh_gateway_window_seconds: 63,
        }
    );
    std::fs::remove_dir_all(root).unwrap();

    let slskdn =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .unwrap();
    assert_eq!(slskdn.controller_web_rate_limiting.api_permit_limit, 200);
    assert!(slskdn.controller_web_rate_limiting.enabled);
    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .unwrap();
    assert!(!slskd.controller_web_rate_limiting.enabled);

    let disabled_negative = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_WEB_RATE_LIMITING", "false")
            .with("SLSKD_WEB_API_PERMIT_LIMIT", "-1")
            .with("SLSKD_WEB_API_WINDOW_SECONDS", "-2"),
    )
    .unwrap();
    assert_eq!(
        disabled_negative
            .controller_web_rate_limiting
            .api_permit_limit,
        -1
    );
    assert_eq!(
        disabled_negative
            .controller_web_rate_limiting
            .api_window_seconds,
        -2
    );

    let enabled_zero = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKD_WEB_API_PERMIT_LIMIT", "0"),
    )
    .expect("frozen slskdN accepts zero permit limits until first policy use");
    assert_eq!(
        enabled_zero.controller_web_rate_limiting.api_permit_limit,
        0
    );
}

#[test]
fn controller_surfaces_accept_dotnet_backtracking_regex_syntax() {
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_SHARE_FILTER", r"(?<=/)private(?=\.flac$)")
            .with("SLSKD_SEARCH_REQUEST_FILTER", r"^(secret)\1$")
            .with("SLSKD_BLACKLISTED_PATTERNS", r"^(?<stem>blocked)\k<stem>$"),
    )
    .expect("valid .NET lookaround and backreference syntax");

    assert_eq!(
        config.share_settings.filters,
        vec![r"(?<=/)private(?=\.flac$)"]
    );
    assert_eq!(
        config.controller_search_request_filters,
        vec![r"^(secret)\1$"]
    );
    assert_eq!(
        config.managed_blacklist.patterns,
        vec![r"^(?<stem>blocked)\k<stem>$"]
    );
}

#[test]
fn current_profile_consumes_canonical_auto_replace_settings() {
    let root = std::env::temp_dir().join(format!(
        "slskr-auto-replace-current-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        "auto_replace:\n  interval_seconds: 180\n  size_threshold_percent: 2.5\n  max_retries: 4\n",
    )
    .unwrap();

    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("current canonical auto_replace settings");
    assert!(config.current_upstream_behavior);
    assert_eq!(
        config.transfer_download.auto_replace_interval.as_secs(),
        180
    );
    assert_eq!(config.transfer_download.auto_replace_threshold_percent, 2.5);
    assert_eq!(config.transfer_download.auto_replace_max_retries, Some(4));

    let defaults = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_PARITY_PROFILE", "current"),
    )
    .expect("current auto_replace defaults");
    assert_eq!(
        defaults.transfer_download.auto_replace_interval.as_secs(),
        180
    );

    std::fs::remove_file(root.join("slskd.yml")).unwrap();
    let defaults = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_PARITY_PROFILE", "current"),
    )
    .expect("current auto_replace defaults without YAML");
    assert_eq!(
        defaults.transfer_download.auto_replace_interval.as_secs(),
        300
    );
    assert_eq!(
        defaults.transfer_download.auto_replace_threshold_percent,
        0.0
    );
    assert_eq!(defaults.transfer_download.auto_replace_max_retries, Some(3));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn current_profile_accepts_upstream_filter_and_lidarr_names() {
    let current = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("DOWNLOAD_FILTER_EXCLUDE", "sample;private")
            .with("LIDARR_IMPORT_DELAY", "9")
            .with("LIDARR_IMPORT_RETRY_MAX_ATTEMPTS", "4")
            .with("LIDARR_IMPORT_RETRY_DELAY", "17")
            .with("LIDARR_SKIP_ALREADY_OWNED_ALBUMS", "false")
            .with("LIDARR_EDITION_MATCH_MODE", "prefer"),
    )
    .expect("current upstream filter and Lidarr environment names");
    assert!(current.current_upstream_behavior);
    assert_eq!(current.download_filter.exclude, ["sample", "private"]);
    assert_eq!(current.integrations.lidarr.import_delay_seconds, 9);
    assert_eq!(current.integrations.lidarr.import_retry_max_attempts, 4);
    assert_eq!(current.integrations.lidarr.import_retry_delay_seconds, 17);
    assert!(!current.integrations.lidarr.skip_already_owned_albums);
    assert_eq!(current.integrations.lidarr.edition_match_mode, "prefer");

    let frozen = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("LIDARR_IMPORT_DELAY", "19"),
    )
    .expect("frozen profile ignores current-only environment names");
    assert!(!frozen.current_upstream_behavior);
    assert_eq!(frozen.integrations.lidarr.import_delay_seconds, 0);
}

#[test]
fn frozen_targets_read_blacklisted_groups_from_their_distinct_yaml_paths() {
    for (target, expected) in [
        ("slskd", "^transfers-path$"),
        ("slskdn", "^top-level-path$"),
    ] {
        let root = std::env::temp_dir().join(format!(
            "slskr-blacklist-yaml-path-{target}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
                root.join("slskd.yml"),
                "transfers:\n  groups:\n    blacklisted:\n      patterns: ['^transfers-path$']\ngroups:\n  blacklisted:\n    patterns: ['^top-level-path$']\n",
            )
            .unwrap();

        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        )
        .unwrap();
        assert_eq!(config.managed_blacklist.patterns, vec![expected]);
        std::fs::remove_dir_all(root).unwrap();
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-blacklist-yaml-path-slskdn-transfers-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        "transfers:\n  groups:\n    blacklisted:\n      patterns: ['^documented-slskdn-path$']\n",
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    assert_eq!(
        config.managed_blacklist.patterns,
        vec!["^documented-slskdn-path$"]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn frozen_transfer_download_settings_match_both_target_profiles() {
    let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "legacy")
                .with(
                    "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                    r#"{"slots":4,"speed_limit":777,"retry":{"partial":"overwrite","attempts":4,"delay":1200,"max_delay":31000},"destination":{"subdirectory":"Music/${SOURCE_USERNAME}","exists":"overwrite","permissions":{"mode":"0750"}}}"#,
                ),
        )
        .unwrap();
    assert_eq!(slskd.transfer_download.slots, 4);
    assert_eq!(slskd.transfer_download.speed_limit_kib, 777);
    assert_eq!(slskd.transfer_download.retry.incomplete, "overwrite");
    assert_eq!(slskd.transfer_download.retry.attempts, 4);
    assert_eq!(slskd.transfer_download.retry.delay.as_millis(), 1200);
    assert_eq!(
        slskd.transfer_download.destination.subdirectory.as_deref(),
        Some("Music/${SOURCE_USERNAME}")
    );
    assert_eq!(
        slskd
            .transfer_download
            .destination
            .permissions_mode
            .as_deref(),
        Some("0750")
    );

    let native = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with(
                    "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                    r#"{"slots":5,"speed_limit":888,"retry":{"incomplete":"overwrite","attempts":5,"delay":1300,"max_delay":32000},"completed_layout":"uploader_folder","auto_replace_stuck":true,"auto_replace_threshold":7.5,"auto_replace_interval":90}"#,
                )
                .with("SLSKD_DOWNLOAD_SLOTS", "6")
                .with("SLSKD_AUTO_REPLACE_INTERVAL", "91"),
        )
        .unwrap();
    assert_eq!(native.transfer_download.slots, 6);
    assert_eq!(native.transfer_download.speed_limit_kib, 888);
    assert_eq!(native.transfer_download.retry.incomplete, "overwrite");
    assert_eq!(native.transfer_download.completed_layout, "uploader_folder");
    assert!(native.transfer_download.auto_replace_stuck);
    assert_eq!(native.transfer_download.auto_replace_threshold_percent, 7.5);
    assert_eq!(native.transfer_download.auto_replace_interval.as_secs(), 91);

    for (target, json, expected) in [
        ("slskd", r#"{"retry":{"attempts":0}}"#, "attempts"),
        (
            "slskd",
            r#"{"destination":{"permissions":{"mode":"999"}}}"#,
            "permissions",
        ),
        (
            "slskdn",
            r#"{"auto_replace_threshold":0.0}"#,
            "AUTO_REPLACE_THRESHOLD",
        ),
    ] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON", json),
        )
        .expect_err("invalid frozen transfer download setting must fail");
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn frozen_transfer_groups_and_upload_settings_load_validate_and_preserve_null_windows() {
    let root = std::env::temp_dir().join(format!(
        "slskr-transfer-groups-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            "transfers:\n  upload:\n    slots: 20\n    speed_limit: 1200\n    limits:\n      queued:\n        files: 50\n        megabytes: 500\n      daily: null\n      weekly:\n        failures: 8\n  groups:\n    default:\n      upload:\n        priority: 20\n        strategy: firstinfirstout\n        slots: 9\n    leechers:\n      thresholds:\n        files: 4\n        directories: 2\n      upload:\n        priority: 90\n        slots: 1\n        speed_limit: 100\n    user_defined:\n      friends:\n        upload:\n          priority: 5\n          slots: 7\n          limits:\n            queued:\n              files: 100\n        members: [alice, bob]\n",
        )
        .unwrap();

    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKD_APP_DIR", root.to_str().unwrap()),
    )
    .unwrap();
    assert_eq!(config.transfer_upload.slots, 20);
    assert_eq!(config.transfer_upload.speed_limit_kib, 1200);
    assert_eq!(
        config.transfer_upload.limits.queued.as_ref().unwrap().files,
        Some(50)
    );
    assert_eq!(
        config.transfer_upload.limits.daily,
        Some(super::TransferLimitSettings::default())
    );
    assert_eq!(
        config
            .transfer_upload
            .limits
            .weekly
            .as_ref()
            .unwrap()
            .failures,
        Some(8)
    );
    assert_eq!(config.transfer_groups.default.upload.priority, 20);
    assert_eq!(
        config.transfer_groups.default.upload.strategy,
        super::TransferQueueStrategy::FirstInFirstOut
    );
    assert_eq!(config.transfer_groups.leechers.threshold_files, 4);
    assert_eq!(config.transfer_groups.leechers.threshold_directories, 2);
    assert_eq!(config.transfer_groups.leechers.upload.speed_limit_kib, 100);
    assert_eq!(
        config.transfer_groups.user_defined["friends"].members,
        vec!["alice", "bob"]
    );
    assert_eq!(
        config.transfer_groups.user_defined["friends"]
            .upload
            .limits
            .queued
            .as_ref()
            .unwrap()
            .files,
        Some(100)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn frozen_transfer_group_validation_is_target_specific() {
    let duplicate = r#"{
            "user_defined": {
                "first": {"members": ["alice"]},
                "second": {"members": ["alice"]}
            }
        }"#;
    let native = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", duplicate),
    )
    .expect_err("slskdN rejects duplicate explicit group membership");
    assert!(native.contains("multiple groups"), "{native}");

    let blacklist_duplicate = r#"{
            "blacklisted": {"members": ["alice"]},
            "user_defined": {"first": {"members": ["ALICE"]}}
        }"#;
    let native_blacklist = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", blacklist_duplicate),
    )
    .expect_err("slskdN rejects blacklisted/user-defined duplicate membership");
    assert!(
        native_blacklist.contains("multiple groups"),
        "{native_blacklist}"
    );

    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", duplicate),
    )
    .expect("slskd resolves duplicate memberships by group priority");
    assert_eq!(slskd.transfer_groups.user_defined.len(), 2);

    for json in [
        r#"{"default":{"upload":{"priority":0}}}"#,
        r#"{"default":{"upload":{"slots":0}}}"#,
        r#"{"leechers":{"thresholds":{"files":0}}}"#,
        r#"{"default":{"upload":{"strategy":"invalid"}}}"#,
        r#"{"default":{"upload":{"limits":{"queued":{"files":0}}}}}"#,
    ] {
        assert!(
            super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", json),
            )
            .is_err(),
            "accepted invalid groups JSON: {json}"
        );
    }
}

#[test]
fn frozen_controller_startup_aliases_drive_core_runtime_configuration() {
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", "legacy")
        .with("SLSKD_APP_DIR", "/tmp/slskd-compatible-state")
        .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.2")
        .with("SLSKD_HTTP_PORT", "55030")
        .with("SLSKD_SLSK_ADDRESS", "soulseek.example")
        .with("SLSKD_SLSK_PORT", "2271")
        .with("SLSKD_SLSK_USERNAME", "upstream-user")
        .with("SLSKD_SLSK_PASSWORD", "upstream-password")
        .with("SLSKD_SLSK_LISTEN_IP_ADDRESS", "0.0.0.0")
        .with("SLSKD_SLSK_LISTEN_PORT", "55031")
        .with("SLSKD_SLSK_DESCRIPTION", "upstream description")
        .with(
            "SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
            "{uploader}/{remote_folder}",
        )
        .with("SLSKD_NO_CONNECT", "true")
        .with("SLSKD_REMOTE_CONFIGURATION", "true")
        .with("SLSKD_DEBUG", "true")
        .with("SLSKD_NO_CONFIG_WATCH", "true");
    let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect("frozen slskd aliases");

    assert_eq!(
        config.state_dir,
        std::path::PathBuf::from("/tmp/slskd-compatible-state")
    );
    assert_eq!(config.http_bind, "127.0.0.2:55030".parse().unwrap());
    assert_eq!(config.server_address, "soulseek.example:2271");
    assert_eq!(config.username.as_deref(), Some("upstream-user"));
    assert_eq!(config.password.as_deref(), Some("upstream-password"));
    assert_eq!(config.listen_port, 55031);
    assert_eq!(config.listener_bind.as_deref(), Some("0.0.0.0:55031"));
    assert!(!config.obfuscation_enabled);
    assert!(config.obfuscated_listener_bind.is_none());
    assert!(config.obfuscated_advertised_port.is_none());
    assert_eq!(config.obfuscation_listen_port, 0);
    assert!(config.obfuscation_advertise_regular_port);
    assert_eq!(
        config.obfuscation_mode,
        super::SoulseekObfuscationMode::Compatibility
    );
    assert!(!config.obfuscation_prefer_outbound);
    assert_eq!(
        config.download_completed_path_template,
        "{uploader}/{remote_folder}"
    );
    assert_eq!(config.user_info_description, "upstream description");
    assert!(!config.auto_connect);
    assert!(config.remote_configuration);
    assert!(config.controller_debug);
    assert!(config.controller_no_config_watch);
}

#[test]
fn frozen_listener_defaults_bind_the_projected_unspecified_address_and_port() {
    for target in ["slskd", "slskdn"] {
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKD_NO_CONNECT", "true")
                .with("SLSKR_AUTH_DISABLED", "true"),
        )
        .expect("frozen listener defaults");

        assert_eq!(config.listen_port, 50300);
        assert_eq!(config.listener_bind.as_deref(), Some("0.0.0.0:50300"));
    }
}

#[test]
fn current_native_networking_defaults_consolidate_the_public_tcp_endpoint() {
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", "native")
        .with("SLSKR_PARITY_PROFILE", "current")
        .with("SLSKD_NO_CONNECT", "true")
        .with("SLSKR_AUTH_DISABLED", "true");
    let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect("current native networking defaults");

    assert!(config.current_upstream_behavior);
    assert_eq!(config.listen_port, 50_300);
    assert_eq!(config.listener_bind.as_deref(), Some("0.0.0.0:50300"));
    assert_eq!(config.dht_port, 50_300);
    assert_eq!(config.advanced_networking.dht.overlay_port, 50_300);
    assert_eq!(config.advanced_networking.overlay.listen_port, 50_300);
    assert_eq!(config.advanced_networking.overlay.quic_listen_port, 50_300);
    assert_eq!(config.overlay_bind, Some("0.0.0.0:50300".parse().unwrap()));
    assert!(config.shared_mesh_tcp());
    assert!(config.obfuscated_listener_bind.is_none());
    assert_eq!(config.obfuscated_advertised_port, Some(50_300));
    assert_eq!(config.obfuscation_listen_port, 0);

    let frozen = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &env.clone().with("SLSKR_PARITY_PROFILE", "frozen"),
    )
    .expect("frozen native networking defaults");
    assert!(!frozen.current_upstream_behavior);
    assert_eq!(
        frozen.obfuscated_listener_bind.as_deref(),
        Some("0.0.0.0:50301")
    );
    assert_eq!(frozen.obfuscated_advertised_port, Some(50_301));
    assert_eq!(frozen.obfuscation_listen_port, 0);

    let dedicated = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &env.clone().with("SLSKR_OVERLAY_BIND", "127.0.0.1:50305"),
    )
    .expect("explicit dedicated overlay bind");
    assert_eq!(
        dedicated.overlay_bind,
        Some("127.0.0.1:50305".parse().unwrap())
    );
    assert_eq!(dedicated.advanced_networking.dht.overlay_port, 50_305);
    assert!(!dedicated.shared_mesh_tcp());

    let custom_tcp = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &env.with("SLSK_LISTEN_PORT", "51000"),
    )
    .expect("custom current Soulseek TCP port");
    assert_eq!(custom_tcp.listen_port, 51_000);
    assert_eq!(custom_tcp.listener_bind.as_deref(), Some("0.0.0.0:51000"));
    assert_eq!(custom_tcp.advanced_networking.dht.dht_port, 50_300);
    assert_eq!(custom_tcp.advanced_networking.dht.overlay_port, 51_000);
    assert_eq!(custom_tcp.advanced_networking.overlay.listen_port, 50_300);
    assert!(custom_tcp.shared_mesh_tcp());
}

#[test]
fn native_startup_names_take_precedence_over_frozen_controller_aliases() {
    let env = MapEnv::default()
        .with("SLSKR_HTTP_BIND", "127.0.0.1:51000")
        .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.2")
        .with("SLSKD_HTTP_PORT", "52000")
        .with("SLSK_LISTEN_PORT", "51001")
        .with("SLSKD_SLSK_LISTEN_PORT", "52001")
        .with("SLSK_USERNAME", "native-user")
        .with("SLSKD_SLSK_USERNAME", "upstream-user")
        .with("SLSKR_AUTO_CONNECT", "true")
        .with("SLSKD_NO_CONNECT", "true")
        .with("SLSKR_REMOTE_CONFIGURATION", "false")
        .with("SLSKD_REMOTE_CONFIGURATION", "true")
        .with("SLSKR_DEBUG", "false")
        .with("SLSKD_DEBUG", "true")
        .with("SLSKR_NO_CONFIG_WATCH", "false")
        .with("SLSKD_NO_CONFIG_WATCH", "true");
    let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect("native precedence");

    assert_eq!(config.http_bind, "127.0.0.1:51000".parse().unwrap());
    assert_eq!(config.listen_port, 51001);
    assert_eq!(config.username.as_deref(), Some("native-user"));
    assert!(config.auto_connect);
    assert!(!config.remote_configuration);
    assert!(!config.controller_debug);
    assert!(!config.controller_no_config_watch);
}

#[test]
fn invalid_frozen_controller_alias_reports_the_exact_name() {
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKD_HTTP_PORT", "not-a-port"),
    )
    .expect_err("invalid frozen alias must fail");
    assert!(error.contains("SLSKD_HTTP_PORT"), "{error}");
}

#[test]
fn frozen_web_bind_profiles_preserve_multi_address_and_target_specific_names() {
    let controller_default = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("frozen slskd default web bind");
    assert_eq!(controller_default.controller_http_address, None);
    assert_eq!(
        controller_default.http_binds,
        vec!["[::]:5030".parse().unwrap()]
    );

    let controller_multi = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.1, ::1")
            .with("SLSKD_HTTP_PORT", "55440"),
    )
    .expect("frozen slskd comma-separated web binds");
    assert_eq!(
        controller_multi.controller_http_address.as_deref(),
        Some("127.0.0.1, ::1")
    );
    assert_eq!(
        controller_multi.http_binds,
        vec![
            "127.0.0.1:55440".parse().unwrap(),
            "[::1]:55440".parse().unwrap()
        ]
    );

    let slskdn = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.2")
            .with("SLSKD_HTTP_ADDRESS", "*")
            .with("SLSKD_HTTP_PORT", "55441"),
    )
    .expect("frozen slskdN address web bind");
    assert_eq!(slskdn.controller_http_address.as_deref(), Some("*"));
    assert_eq!(slskdn.http_binds, vec!["0.0.0.0:55441".parse().unwrap()]);

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.1,not-an-ip"),
    )
    .expect_err("invalid frozen slskd web IP list must fail");
    assert!(error.contains("SLSKD_HTTP_IP_ADDRESS"), "{error}");
}

#[test]
fn frozen_directory_yaml_environment_and_target_validation_drive_storage_roots() {
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-directories-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let yaml_downloads = root.join("yaml-downloads");
    let yaml_incomplete = root.join("yaml-incomplete");
    let yaml_share_a = root.join("yaml-share-a");
    let yaml_share_b = root.join("yaml-share-b");
    let env_downloads = root.join("env-downloads");
    std::fs::create_dir_all(&yaml_downloads).unwrap();
    std::fs::create_dir_all(&yaml_incomplete).unwrap();
    std::fs::create_dir_all(&yaml_share_a).unwrap();
    std::fs::create_dir_all(&yaml_share_b).unwrap();
    std::fs::create_dir_all(&env_downloads).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            format!(
                "directories:\n  downloads: '{}'\n  incomplete: '{}'\nshares:\n  directories:\n    - '{}'\n    - '{}'\n",
                yaml_downloads.display(),
                yaml_incomplete.display(),
                yaml_share_a.display(),
                yaml_share_b.display()
            ),
        )
        .unwrap();

    let yaml = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("directory YAML provider");
    assert_eq!(yaml.downloads_dir, yaml_downloads);
    assert_eq!(yaml.incomplete_dir, yaml_incomplete);
    assert_eq!(
        yaml.share_settings.roots,
        vec![yaml_share_a.clone(), yaml_share_b.clone()]
    );

    let environment = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKD_DOWNLOADS_DIR", env_downloads.to_str().unwrap()),
    )
    .expect("directory YAML precedence over frozen environment alias");
    assert_eq!(environment.downloads_dir, yaml_downloads);
    assert_eq!(environment.incomplete_dir, yaml_incomplete);

    let missing = root.join("missing");
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_DOWNLOADS_DIR", missing.to_str().unwrap()),
    )
    .expect_err("missing configured directory must fail");
    assert!(error.contains("non-existent directory"), "{error}");

    let current = std::env::current_dir().unwrap();
    let relative_root = current.join(format!(
        ".slskr-relative-directory-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&relative_root).unwrap();
    let relative = relative_root
        .strip_prefix(&current)
        .unwrap()
        .to_str()
        .unwrap();
    let controller_error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_DOWNLOADS_DIR", relative),
    )
    .expect_err("slskd rejects relative download directories");
    assert!(
        controller_error.contains("absolute path"),
        "{controller_error}"
    );
    let slskdn = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_DOWNLOADS_DIR", relative),
    )
    .expect("slskdN accepts an existing relative download directory");
    assert_eq!(slskdn.downloads_dir, std::path::PathBuf::from(relative));

    std::fs::remove_dir_all(relative_root).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_rejects_loopback_listener_only_when_connecting() {
    let root = std::env::temp_dir().join(format!(
        "slskr-no-connect-validation-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let base = MapEnv::default()
        .with("SLSKR_STATE_DIR", root.to_str().unwrap())
        .with("SLSKR_AUTH_DISABLED", "true")
        .with("SLSKD_SLSK_USERNAME", "fixture-user")
        .with("SLSKD_SLSK_PASSWORD", "fixture-password")
        .with("SLSKD_SLSK_LISTEN_IP_ADDRESS", "127.0.0.1")
        .with("SLSKD_SLSK_LISTEN_PORT", "55091");

    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &base
            .clone()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKD_NO_CONNECT", "false"),
    )
    .expect("frozen slskd permits a loopback listener while connecting");
    assert!(slskd.auto_connect);

    let disconnected = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &base
            .clone()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_NO_CONNECT", "true"),
    )
    .expect("frozen slskdN permits a loopback listener when no-connect is set");
    assert!(!disconnected.auto_connect);

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &base
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_NO_CONNECT", "false"),
    )
    .expect_err("frozen slskdN rejects a loopback listener while connecting");
    assert_eq!(
            error,
            "Soulseek.ListenIpAddress must not be a loopback address when the client is connecting. Use 0.0.0.0 or a reachable LAN/VPN interface instead."
        );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn controller_yaml_is_a_real_startup_provider_for_core_settings() {
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-yaml-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            "instance_name: yaml-instance\ndebug: true\nremote_configuration: true\nremote_file_management: true\nflags:\n  no_connect: true\n  no_config_watch: true\ndht:\n  enabled: false\n  dht_port: 55200\nsoulseek:\n  address: yaml.example\n  port: 2271\n  username: yaml-user\n  password: yaml-password\n  description: yaml description\n  listen_ip_address: 0.0.0.0\n  listen_port: 55100\n  obfuscation:\n    enabled: true\n    mode: prefer\n    listen_port: 55101\n    prefer_outbound: false\n  private_message_auto_response:\n    enabled: true\n    message: yaml auto response\n    cooldown_minutes: 15\nintegrations:\n  spotify:\n    enabled: true\n    client_id: yaml-client\n    client_secret: yaml-client-secret\n    redirect_uri: https://localhost/callback\n    market: CA\n  lidarr:\n    enabled: true\n    url: https://lidarr.example\n    api_key: yaml-lidarr-key\n    timeout_seconds: 30\ntransfers:\n  download:\n    completed_path_template: '{uploader}/{remote_folder}'\n    auto_retry:\n      enabled: false\n      retry_delay_seconds: 1200\n      check_interval_seconds: 120\n      max_attempts: 7\n      max_files_per_cycle: 8\n      max_files_per_peer_per_cycle: 2\n      peer_cooldown_seconds: 600\n      alternate_sources_enabled: false\n      max_alternate_source_searches_per_cycle: 2\n      alternate_source_size_tolerance_percent: 7\nweb:\n  ip_address: 127.0.0.3\n  port: 55102\n",
        )
        .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("controller YAML startup provider");

    assert!(config.controller_debug);
    assert_eq!(config.instance_name, "yaml-instance");
    assert!(config.remote_configuration);
    assert!(config.remote_file_management);
    assert!(config.controller_no_config_watch);
    assert!(!config.auto_connect);
    assert_eq!(config.server_address, "yaml.example:2271");
    assert_eq!(config.username.as_deref(), Some("yaml-user"));
    assert_eq!(config.password.as_deref(), Some("yaml-password"));
    assert_eq!(config.user_info_description, "yaml description");
    assert_eq!(config.listen_port, 55100);
    assert_eq!(config.listener_bind.as_deref(), Some("0.0.0.0:55100"));
    // The frozen slskd profile does not expose the slskdN-only Soulseek
    // type-1 obfuscation listener or parse its obfuscation settings, so
    // the YAML `soulseek.obfuscation` block above is ignored for this
    // target. See release-notes/20260817-slskd-obfuscation-profile.md
    // and controller_profile_does_not_expose_native_type1_obfuscation_layers.
    assert!(config.obfuscated_listener_bind.is_none());
    assert_eq!(config.http_bind, "127.0.0.3:55102".parse().unwrap());
    assert!(!config.dht_enabled);
    assert_eq!(config.dht_port, 55200);
    assert_eq!(
        config.obfuscation_mode,
        super::SoulseekObfuscationMode::Compatibility
    );
    assert_eq!(config.obfuscation_listen_port, 0);
    assert!(config.obfuscation_advertise_regular_port);
    assert!(!config.obfuscation_prefer_outbound);
    assert_eq!(
        config.download_completed_path_template,
        "{uploader}/{remote_folder}"
    );
    assert!(config.private_message_auto_response.enabled);
    assert_eq!(
        config.private_message_auto_response.message,
        "yaml auto response"
    );
    assert_eq!(config.private_message_auto_response.cooldown_minutes, 15);
    assert!(config.integrations.spotify.enabled);
    assert_eq!(
        config.integrations.spotify.client_id.as_deref(),
        Some("yaml-client")
    );
    assert_eq!(config.integrations.spotify.market, "CA");
    assert!(config.integrations.lidarr.enabled);
    assert_eq!(
        config.integrations.lidarr.url.as_deref(),
        Some("https://lidarr.example")
    );
    assert_eq!(config.integrations.lidarr.timeout_seconds, 30);
    assert!(!config.transfer_auto_retry.enabled);
    assert_eq!(config.transfer_auto_retry.retry_delay.as_secs(), 1200);
    assert_eq!(config.transfer_auto_retry.check_interval.as_secs(), 120);
    assert_eq!(config.transfer_auto_retry.max_attempts, 7);
    assert_eq!(config.transfer_auto_retry.max_files_per_cycle, 8);
    assert_eq!(config.transfer_auto_retry.max_files_per_peer_per_cycle, 2);
    assert_eq!(config.transfer_auto_retry.peer_cooldown.as_secs(), 600);
    assert!(!config.transfer_auto_retry.alternate_sources_enabled);
    assert_eq!(
        config
            .transfer_auto_retry
            .max_alternate_source_searches_per_cycle,
        2
    );
    assert_eq!(
        config
            .transfer_auto_retry
            .alternate_source_size_tolerance_percent,
        7.0
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn controller_yaml_instance_name_matches_frozen_unvalidated_string_binding() {
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-instance-name-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let long_name = "a".repeat(300);
    let cases = [
        ("instance_name: null\n", "default".to_owned()),
        ("instance_name: \"\"\n", "default".to_owned()),
        ("instance_name: 123\n", "123".to_owned()),
        ("instance_name: true\n", "true".to_owned()),
        (
            "instance_name: \"line one\\nline two\"\n",
            "line one\nline two".to_owned(),
        ),
    ];

    for (yaml, expected) in cases {
        std::fs::write(root.join("slskd.yml"), yaml).unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().unwrap()),
        )
        .expect("frozen instance-name scalar binding");
        assert_eq!(config.instance_name, expected);
    }

    std::fs::write(
        root.join("slskd.yml"),
        format!("instance_name: \"{long_name}\"\n"),
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().unwrap()),
    )
    .expect("frozen long instance name");
    assert_eq!(config.instance_name, long_name);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn musicbrainz_yaml_projection_matches_frozen_integration_options() {
    let root = std::env::temp_dir().join(format!(
        "slskr-musicbrainz-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            "integrations:\n  musicbrainz:\n    baseUrl: http://musicbrainz.fixture/ws/2\n    userAgent: fixture-agent/1.0\n    timeoutSeconds: 7.5\n    retryAttempts: 3\n",
        )
        .unwrap();

    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("MusicBrainz YAML options");
    assert_eq!(
        config.integrations.musicbrainz.base_url,
        "http://musicbrainz.fixture/ws/2"
    );
    assert_eq!(
        config.integrations.musicbrainz.user_agent,
        "fixture-agent/1.0"
    );
    assert_eq!(config.integrations.musicbrainz.timeout_seconds, 7.5);
    assert_eq!(config.integrations.musicbrainz.retry_attempts, 3);

    std::fs::write(
        root.join("slskd.yml"),
        "integrations:\n  musicbrainz:\n    timeoutSeconds: 0\n",
    )
    .unwrap();
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect_err("non-positive MusicBrainz timeout");
    assert!(error.contains("TimeoutSeconds"), "{error}");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn chromaprint_and_acoustid_settings_match_frozen_integration_options() {
    let root = std::env::temp_dir().join(format!(
        "slskr-audio-integrations-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let file = serde_yaml::from_str::<super::FileConfig>(
            "integration:\n  chromaprint:\n    enabled: true\n    algorithm: 1\n    ffmpegPath: /usr/bin/ffmpeg\n    sampleRate: 22050\n    channels: 1\n    durationSeconds: 45\n  acoustid:\n    enabled: true\n    clientId: fixture-client\n    baseUrl: http://127.0.0.1:39001/v2\n",
        )
        .expect("target integration alias parses");
    let config = super::AppConfig::from_layers(
        None,
        file,
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("audio integration settings");
    assert!(config.integrations.chromaprint.enabled);
    assert_eq!(config.integrations.chromaprint.algorithm, 1);
    assert_eq!(
        config.integrations.chromaprint.ffmpeg_path,
        "/usr/bin/ffmpeg"
    );
    assert_eq!(config.integrations.chromaprint.sample_rate, 22_050);
    assert_eq!(config.integrations.chromaprint.channels, 1);
    assert_eq!(config.integrations.chromaprint.duration_seconds, 45);
    assert!(config.integrations.acoustid.enabled);
    assert_eq!(
        config.integrations.acoustid.client_id.as_deref(),
        Some("fixture-client")
    );
    assert_eq!(
        config.integrations.acoustid.base_url,
        "http://127.0.0.1:39001/v2"
    );
    let sanitized = config.integrations.sanitized_json();
    assert!(sanitized.contains("\"chromaprint\""));
    assert!(sanitized.contains("\"acoustid\""));
    assert!(sanitized.contains("\"client_id_configured\":true"));
    assert!(!sanitized.contains("fixture-client"));

    let error = super::AppConfig::from_layers(
        None,
        serde_yaml::from_str::<super::FileConfig>("integration:\n  acoustid:\n    enabled: true\n")
            .unwrap(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect_err("enabled AcoustID requires a client id");
    assert!(error.contains("client id"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn frozen_share_directories_preserve_aliases_exclusions_and_raw_values() {
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-share-aliases-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let excluded = root.join("excluded");
    std::fs::create_dir_all(&excluded).unwrap();
    let value = format!("[Library]{};!{}", root.display(), excluded.display());
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKD_SHARED_DIR", &value),
    )
    .expect("aliased and excluded frozen shares");

    assert_eq!(config.share_settings.directories.len(), 2);
    assert_eq!(config.share_settings.directories[0].alias, "Library");
    assert!(!config.share_settings.directories[0].is_excluded);
    assert_eq!(config.share_settings.directories[1].alias, "excluded");
    assert!(config.share_settings.directories[1].is_excluded);
    assert_eq!(config.share_settings.roots, vec![root.clone()]);
    assert_eq!(
        config.share_settings.directories[0].raw,
        format!("[Library]{}", root.display())
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn controller_yaml_precedes_frozen_environment_aliases_while_native_names_win() {
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-yaml-precedence-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
            root.join("slskd.yml"),
            "debug: true\nremote_configuration: true\nweb:\n  port: 55102\nsoulseek:\n  listen_port: 55100\n",
        )
        .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKD_DEBUG", "false")
            .with("SLSKR_REMOTE_CONFIGURATION", "false")
            .with("SLSKD_HTTP_PORT", "55202")
            .with("SLSK_LISTEN_PORT", "55200"),
    )
    .expect("controller precedence");

    assert!(config.controller_debug);
    assert!(!config.remote_configuration);
    assert_eq!(config.http_bind.port(), 55102);
    assert_eq!(config.listen_port, 55200);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn controller_yaml_startup_provider_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "slskr-controller-yaml-link-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let outside = root.with_extension("outside.yml");
    std::fs::write(&outside, "debug: true\n").unwrap();
    symlink(&outside, root.join("slskd.yml")).unwrap();
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().unwrap()),
    )
    .expect_err("controller YAML symlink must fail");
    assert_eq!(error, "controller YAML must be a regular file");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_file(outside);
}

#[test]
fn config_file_reader_rejects_oversized_files() {
    let path = std::env::temp_dir().join(format!(
        "slskr-config-large-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(super::MAX_CONFIG_FILE_BYTES + 1).unwrap();

    let error = super::read_file_config(&path).expect_err("oversized config should be rejected");
    assert!(error.contains("config file"));
    assert!(error.contains("too large"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn config_file_reader_rejects_non_regular_paths() {
    let path = std::env::temp_dir().join(format!(
        "slskr-config-dir-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&path).unwrap();

    let error = super::read_file_config(&path).expect_err("directory should be rejected");
    assert!(error.contains("config"));
    assert!(error.contains(&path.display().to_string()));

    let _ = std::fs::remove_dir(path);
}

#[cfg(unix)]
#[test]
fn config_file_reader_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "slskr-config-symlink-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("target.toml");
    let linked = root.join("config.toml");
    std::fs::write(&target, "[auth]\napi_token = \"outside-secret\"\n").unwrap();
    symlink(&target, &linked).unwrap();

    let error = super::read_file_config(&linked).expect_err("symlink should be rejected");
    assert!(error.contains("failed to read config file"));
    assert!(!error.contains("outside-secret"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn config_file_reader_parses_small_files() {
    let path = std::env::temp_dir().join(format!(
        "slskr-config-small-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&path, "[app]\nhttp_bind = \"127.0.0.1:5555\"\n").unwrap();

    let config = super::read_file_config(&path).expect("small config parsed");
    assert_eq!(config.app.http_bind.as_deref(), Some("127.0.0.1:5555"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn config_sensitive_value_detection_covers_secrets() {
    let empty = super::FileConfig::default();
    assert!(!super::config_contains_sensitive_values(&empty));

    let with_api_token = super::FileConfig {
        auth: super::AuthFileConfig {
            api_token: Some("token".to_owned()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(super::config_contains_sensitive_values(&with_api_token));

    let with_integration_secret = super::FileConfig {
        integrations: super::IntegrationsFileConfig {
            spotify: super::SpotifyFileConfig {
                client_secret: Some("secret".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(super::config_contains_sensitive_values(
        &with_integration_secret
    ));
}

#[test]
fn lidarr_url_rejects_embedded_credentials_before_projection() {
    let env = MapEnv::default()
        .with("SLSKR_LIDARR_URL", "https://operator:secret@example.com")
        .with("SLSKR_LIDARR_API_KEY", "api-key");
    let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect_err("credential-bearing Lidarr URL should be rejected");
    assert_eq!(error, "Lidarr URL must not contain embedded credentials");
    assert!(!error.contains("operator"));
    assert!(!error.contains("secret"));

    for url in [
        "ftp://example.com/lidarr",
        "https://example.com/lidarr?api_key=secret",
        "https://example.com/lidarr#ignored-api-path",
    ] {
        let env = MapEnv::default().with("SLSKR_LIDARR_URL", url);
        let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect_err("non-base Lidarr URL should be rejected");
        assert!(!error.contains("api_key=secret"), "{error}");
    }
}

#[test]
fn trusted_proxy_cidrs_parse_from_env_and_file() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32,::1/128");
    let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect("trusted proxy env config");
    assert_eq!(config.trusted_proxy_cidrs.len(), 2);
    assert!(config.trusted_proxy_cidrs[0].contains("127.0.0.1".parse().unwrap()));
    assert!(config.trusted_proxy_cidrs[1].contains("::1".parse().unwrap()));

    let file_config = super::FileConfig {
        auth: super::AuthFileConfig {
            trusted_proxy_cidrs: vec!["10.0.0.0/8".to_owned()],
            ..Default::default()
        },
        ..Default::default()
    };
    let config = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
        .expect("trusted proxy file config");
    assert!(config.trusted_proxy_cidrs[0].contains("10.1.2.3".parse().unwrap()));
}

#[test]
fn api_token_rejects_blank_env_and_file_values() {
    for token in ["", " \t\r\n"] {
        let env = MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", token);
        let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect_err("blank environment API token must fail");
        assert!(error.contains("must not be empty"));

        let file_config = super::FileConfig {
            auth: super::AuthFileConfig {
                disabled: Some(false),
                api_token: Some(token.to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        let error = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
            .expect_err("blank file API token must fail");
        assert!(error.contains("must not be empty"));
    }
}

#[test]
fn role_api_tokens_are_distinct_and_never_serialized() {
    let env = MapEnv::default()
        .with("SLSKR_API_TOKEN", "admin-token")
        .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
        .with("SLSKR_API_READ_ONLY_TOKEN", "read-token")
        .with("SLSKR_API_NOWPLAYING_TOKEN", "nowplaying-token");
    let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect("distinct role tokens");
    let sanitized = config.sanitized_json();
    for token in [
        "admin-token",
        "write-token",
        "read-token",
        "nowplaying-token",
    ] {
        assert!(!sanitized.contains(token));
    }
    assert!(sanitized.contains("\"api_read_write_token_configured\":true"));
    assert!(sanitized.contains("\"api_read_only_token_configured\":true"));
    assert!(sanitized.contains("\"api_nowplaying_token_configured\":true"));

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_API_TOKEN", "duplicate-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "duplicate-token"),
    )
    .expect_err("role tokens must not alias");
    assert_eq!(error, "API tokens for different roles must be distinct");
}

#[test]
fn sanitized_config_is_valid_json() {
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("default config");

    serde_json::from_str::<serde_json::Value>(&config.sanitized_json())
        .expect("sanitized config must remain valid JSON");
}

#[test]
fn controller_profile_is_explicit_bounded_and_projected() {
    let default = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("default controller compatibility target");
    assert_eq!(default.controller_profile, super::ControllerProfile::Native);

    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd controller compatibility target");
    assert_eq!(slskd.controller_profile, super::ControllerProfile::Legacy);
    assert!(slskd
        .sanitized_json()
        .contains("\"controller_profile\":\"legacy\""));

    let file = super::FileConfig {
        compatibility: super::CompatibilityFileConfig {
            profile: Some("slskd".to_owned()),
            ..Default::default()
        },
        ..Default::default()
    };
    let from_file = super::AppConfig::from_layers(
        None,
        file,
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("file controller compatibility target");
    assert_eq!(
        from_file.controller_profile,
        super::ControllerProfile::Legacy
    );

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "auto"),
    )
    .expect_err("ambiguous compatibility target must fail");
    assert!(error.contains("must be legacy or native"));
}

#[test]
fn gold_star_club_autojoin_is_profile_aware_and_configurable() {
    let parsed: super::FileConfig = toml::from_str("[podcore.gold_star_club]\nautojoin = false\n")
        .expect("Gold Star Club TOML setting");
    assert_eq!(parsed.podcore.gold_star_club.autojoin, Some(false));

    let current = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("native/current Gold Star Club default");
    assert!(!current.advanced_networking.gold_star_club_autojoin);

    let current_enabled = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_POD_GOLD_STAR_CLUB_AUTOJOIN", "true"),
    )
    .expect("native/current Gold Star Club opt-in");
    assert!(current_enabled.advanced_networking.gold_star_club_autojoin);

    let frozen = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("frozen Gold Star Club default");
    assert!(frozen.advanced_networking.gold_star_club_autojoin);

    let file_disabled = super::FileConfig {
        podcore: super::PodCoreFileConfig {
            gold_star_club: super::GoldStarClubFileConfig {
                autojoin: Some(false),
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let from_file = super::AppConfig::from_layers(
        None,
        file_disabled,
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("file-disabled Gold Star Club");
    assert!(!from_file.advanced_networking.gold_star_club_autojoin);

    let env_wins = super::AppConfig::from_layers(
        None,
        super::FileConfig {
            podcore: super::PodCoreFileConfig {
                gold_star_club: super::GoldStarClubFileConfig {
                    autojoin: Some(false),
                },
                ..Default::default()
            },
            ..Default::default()
        },
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_POD_GOLD_STAR_CLUB_AUTOJOIN", "true"),
    )
    .expect("environment-enabled Gold Star Club");
    assert!(env_wins.advanced_networking.gold_star_club_autojoin);
}

#[test]
fn api_token_rejects_unrepresentable_env_and_file_values() {
    for token in [
        " leading",
        "trailing ",
        "token\tvalue",
        "token\nvalue",
        "token\u{7f}value",
    ] {
        let env = MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", token);
        let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect_err("unrepresentable environment API token must fail");
        assert!(
            error.contains("whitespace") || error.contains("control"),
            "{error}"
        );

        let file_config = super::FileConfig {
            auth: super::AuthFileConfig {
                disabled: Some(false),
                api_token: Some(token.to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        let error = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
            .expect_err("unrepresentable file API token must fail");
        assert!(
            error.contains("whitespace") || error.contains("control"),
            "{error}"
        );
    }
}

#[test]
fn api_token_length_matches_http_header_capacity() {
    let maximum = "x".repeat(crate::http_server::MAX_API_TOKEN_BYTES);
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_API_TOKEN", &maximum),
    )
    .expect("maximum representable token");
    assert_eq!(config.api_token.as_deref(), Some(maximum.as_str()));

    let oversized = format!("{maximum}x");
    let env = MapEnv::default().with("SLSKR_API_TOKEN", &oversized);
    let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect_err("oversized environment token must fail");
    assert!(error.contains("maximum representable"), "{error}");

    let file_config = super::FileConfig {
        auth: super::AuthFileConfig {
            api_token: Some(oversized),
            ..Default::default()
        },
        ..Default::default()
    };
    let error = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
        .expect_err("oversized file token must fail");
    assert!(error.contains("maximum representable"), "{error}");
}

#[test]
fn trusted_proxy_cidrs_reject_invalid_prefixes() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/33");
    let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect_err("invalid trusted proxy prefix should fail");
    assert!(error.contains("prefix exceeds"));
}

#[test]
fn peer_response_timeout_rejects_zero() {
    let env = MapEnv::default().with("SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS", "0");
    let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect_err("zero peer timeout should fail");
    assert!(error.contains("must be greater than zero"), "{error}");
}

#[test]
fn soulseek_connection_defaults_bounds_and_target_difference_are_exact() {
    let slskdn = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("slskdN connection defaults");
    let connection = &slskdn.soulseek_connection;
    assert_eq!(connection.buffer_read, 16_384);
    assert_eq!(connection.buffer_write, 16_384);
    assert_eq!(connection.buffer_transfer, 262_144);
    assert_eq!(connection.buffer_write_queue, 50);
    assert_eq!(connection.timeout_connect, Duration::from_millis(10_000));
    assert_eq!(connection.timeout_inactivity, Duration::from_millis(60_000));
    assert_eq!(connection.timeout_transfer, Duration::from_millis(60_000));
    assert!(!connection.proxy.enabled);
    assert_eq!(connection.proxy.port, None);

    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd connection defaults");
    assert_eq!(
        slskd.soulseek_connection.timeout_inactivity,
        Duration::from_millis(15_000)
    );

    for (name, value) in [
        ("SLSKD_SLSK_READ_BUFFER", "1023"),
        ("SLSKD_SLSK_WRITE_BUFFER", "1023"),
        ("SLSKD_SLSK_TRANSFER_BUFFER", "81919"),
        ("SLSKD_SLSK_WRITE_QUEUE", "4"),
        ("SLSKD_SLSK_CONNECTION_TIMEOUT", "999"),
        ("SLSKD_SLSK_INACTIVITY_TIMEOUT", "999"),
        ("SLSKD_SLSK_TRANSFER_TIMEOUT", "29999"),
    ] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with(name, value),
        )
        .expect_err("out-of-range Soulseek connection value");
        assert!(error.contains("must be between"), "{name}: {error}");
    }

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_SLSK_PROXY_ENABLED", "true"),
    )
    .expect_err("enabled proxy needs endpoint");
    assert!(error.contains("no address"), "{error}");
}

#[test]
fn private_message_auto_response_is_opt_in_bounded_and_redacted() {
    let env = MapEnv::default()
        .with("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE", "true")
        .with(
            "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
            "human response",
        )
        .with("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES", "15");
    let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
        .expect("auto-response config");
    assert!(config.private_message_auto_response.enabled);
    assert_eq!(
        config.private_message_auto_response.message,
        "human response"
    );
    assert_eq!(config.private_message_auto_response.cooldown_minutes, 15);
    let sanitized = config.sanitized_json();
    assert!(sanitized.contains("private_message_auto_response"));
    assert!(!sanitized.contains("human response"));

    let blank = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE", ""),
    )
    .expect("blank frozen slskdN auto-response message");
    assert!(blank.private_message_auto_response.message.is_empty());

    for (name, value) in [
        ("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES", "0"),
        (
            "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
            "1441",
        ),
    ] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("invalid auto-response config");
        assert!(error.contains("auto response") || error.contains("auto-response"));
    }
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
            let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
                .expect_err("invalid runtime interval must fail at startup");
            assert!(
                error.contains("greater than zero") || error.contains("timer range"),
                "{name}={value}: {error}"
            );
        }
    }
}

#[test]
fn transfer_auto_retry_defaults_match_the_frozen_client_policy() {
    let config =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .expect("default auto-retry config");
    let retry = &config.transfer_auto_retry;
    assert!(retry.enabled);
    assert_eq!(retry.retry_delay.as_secs(), 1800);
    assert_eq!(retry.check_interval.as_secs(), 300);
    assert_eq!(retry.max_attempts, 5);
    assert_eq!(retry.max_files_per_cycle, 10);
    assert_eq!(retry.max_files_per_peer_per_cycle, 1);
    assert_eq!(retry.peer_cooldown.as_secs(), 900);
    assert!(retry.alternate_sources_enabled);
    assert_eq!(retry.max_alternate_source_searches_per_cycle, 1);
    assert_eq!(retry.alternate_source_size_tolerance_percent, 5.0);
    assert!(config.sanitized_json().contains("\"transfer_auto_retry\""));
}

#[test]
fn transfer_auto_retry_bounds_are_enforced_at_startup() {
    for (name, value) in [
        ("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "9"),
        ("SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS", "3601"),
        ("SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS", "101"),
        ("SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE", "0"),
        (
            "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
            "21",
        ),
        ("SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS", "59"),
        (
            "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
            "11",
        ),
        (
            "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
            "101",
        ),
    ] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("out-of-range auto-retry config must fail");
        assert!(error.contains("must be between"), "{name}={value}: {error}");
    }
}

#[test]
fn transfer_auto_retry_preserves_fractional_tolerance_and_frozen_boundary_rounding() {
    for (value, expected) in [("5.5", 5.5), ("-0.5", -0.5), ("100.5", 100.5)] {
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with(
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                value,
            ),
        )
        .expect("frozen slskdN tolerance must bind");
        assert_eq!(
            config
                .transfer_auto_retry
                .alternate_source_size_tolerance_percent,
            expected
        );
    }
    for value in ["-0.5001", "100.5001"] {
        assert!(super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with(
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                value,
            ),
        )
        .is_err());
    }
}

#[test]
fn managed_blacklist_parses_cidr_p2p_and_dat_ranges() {
    let root = std::env::temp_dir().join(format!(
        "slskr-managed-blacklist-formats-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();

    let cases = [
        ("cidr.txt", "127.0.0.0/24\n"),
        ("p2p.txt", "loopback:127.0.0.1-127.0.0.2\n"),
        (
            "dat.txt",
            "127.000.000.001 - 127.000.000.002 , 000 , local\n",
        ),
    ];
    for (name, body) in cases {
        let path = root.join(name);
        std::fs::write(&path, body).unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKD_BLACKLIST", "true")
                .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
        )
        .expect("managed blacklist format");

        assert!(config
            .managed_blacklist
            .contains("127.0.0.1".parse().unwrap()));
        assert!(config
            .managed_blacklist
            .contains("::ffff:127.0.0.2".parse().unwrap()));
        assert!(!config
            .managed_blacklist
            .contains("127.0.1.1".parse().unwrap()));
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn managed_blacklist_preserves_target_specific_p2p_colon_handling() {
    let root = std::env::temp_dir().join(format!(
        "slskr-managed-blacklist-p2p-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("colon.p2p");
    std::fs::write(&path, "category:label:127.0.0.1-127.0.0.1\n").unwrap();

    let slskdn = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_BLACKLIST", "true")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    )
    .expect("slskdN uses the final P2P colon");
    assert!(slskdn
        .managed_blacklist
        .contains("127.0.0.1".parse().unwrap()));

    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKD_BLACKLIST", "true")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    );
    assert!(slskd.is_err(), "slskd uses the first P2P colon");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn controller_swagger_defaults_split_by_target_and_honors_environment() {
    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd swagger default");
    assert!(!slskd.controller_swagger);

    let slskdn = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("slskdN swagger default");
    assert!(slskdn.controller_swagger);

    let disabled = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_SWAGGER", "false"),
    )
    .expect("slskdN swagger environment override");
    assert!(!disabled.controller_swagger);
}

#[test]
fn controller_metrics_defaults_split_by_target_and_enforce_credentials() {
    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd metrics defaults");
    assert!(!slskd.controller_metrics_enabled);
    assert_eq!(slskd.controller_metrics_url, "/metrics");
    assert_eq!(slskd.controller_metrics_username, "slskd");
    assert!(slskd.controller_metrics_password.is_empty());

    let native = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("native disabled metrics defaults");
    assert!(!native.controller_metrics_enabled);
    assert_eq!(native.controller_metrics_username, "slskr");
    assert!(native.controller_metrics_password.is_empty());

    let missing_password = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_METRICS", "true"),
    );
    assert!(missing_password.is_err());

    let configured = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_METRICS", "true")
            .with("SLSKD_METRICS_URL", "prometheus")
            .with("SLSKD_METRICS_USERNAME", "metrics-user")
            .with("SLSKD_METRICS_PASSWORD", "metrics-pass"),
    )
    .expect("slskdN configured metrics auth");
    assert!(configured.controller_metrics_enabled);
    assert_eq!(configured.controller_metrics_url, "prometheus");
    assert_eq!(configured.controller_metrics_username, "metrics-user");
    assert_eq!(configured.controller_metrics_password, "metrics-pass");
}

#[test]
fn controller_web_auth_defaults_match_frozen_profiles() {
    let controller_default = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd compatibility default authentication");
    assert_eq!(controller_default.controller_web_auth_password, "slskd");

    let native_default = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("slskdN compatibility default authentication");
    assert_eq!(native_default.controller_web_auth_username, "slskr");
    assert_eq!(native_default.controller_web_auth_password, "slskr");

    let disabled = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("disabled web authentication does not need credentials");
    assert!(disabled.controller_web_auth_password.is_empty());

    let configured = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_USERNAME", "admin")
            .with("SLSKD_PASSWORD", "configured-secret"),
    )
    .expect("explicit web credentials");
    assert_eq!(configured.controller_web_auth_username, "admin");
    assert_eq!(configured.controller_web_auth_password, "configured-secret");
}

#[test]
fn controller_headless_defaults_false_and_honors_environment() {
    let default = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("headless default");
    assert!(!default.controller_headless);

    let headless = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
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
    let default = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
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

    let configured = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
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
fn transfer_rescue_defaults_match_the_frozen_client_policy() {
    let config =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .expect("default rescue config");
    let rescue = &config.transfer_rescue;
    assert!(rescue.enabled);
    assert_eq!(rescue.max_queue_time.as_secs(), 1_800);
    assert_eq!(rescue.min_throughput_bytes_per_second, 10 * 1_024);
    assert_eq!(rescue.min_duration.as_secs(), 300);
    assert_eq!(rescue.stalled_timeout.as_secs(), 120);
    assert_eq!(rescue.check_interval.as_secs(), 45);
    assert_eq!(rescue.retry_cooldown.as_secs(), 1_800);
    assert_eq!(rescue.max_files_per_cycle, 2);
    assert_eq!(rescue.alternate_source_size_tolerance_percent, 5);
    assert!(config.sanitized_json().contains("\"transfer_rescue\""));
}

#[test]
fn transfer_rescue_bounds_are_enforced_at_startup() {
    for (name, value) in [
        ("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "59"),
        ("SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS", "0"),
        ("SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS", "59"),
        ("SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS", "29"),
        ("SLSKR_TRANSFER_RESCUE_CHECK_INTERVAL_SECONDS", "14"),
        ("SLSKR_TRANSFER_RESCUE_RETRY_COOLDOWN_SECONDS", "59"),
        ("SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE", "0"),
        (
            "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
            "101",
        ),
    ] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("out-of-range rescue config must fail");
        assert!(error.contains("must be between"), "{name}={value}: {error}");
    }
}

#[test]
fn soulseek_obfuscation_defaults_to_regular_first_compatibility() {
    let compatibility =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .expect("default obfuscation config");
    assert!(compatibility.obfuscation_enabled);
    assert_eq!(
        compatibility.obfuscation_mode,
        super::SoulseekObfuscationMode::Compatibility
    );
    assert!(compatibility.obfuscation_prefer_outbound);
    assert!(compatibility.obfuscation_advertise_regular_port);
    assert!(!compatibility.prefer_obfuscated_outbound());

    let prefer = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
    )
    .expect("prefer obfuscation config");
    assert!(prefer.prefer_obfuscated_outbound());

    for value in ["only", "unknown"] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_OBFUSCATION_MODE", value),
        )
        .expect_err("unsupported obfuscation mode must fail");
        assert!(
            error.to_ascii_lowercase().contains("obfuscation"),
            "{error}"
        );
    }

    let missing_regular_advertisement = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "false"),
    )
    .expect_err("enabled obfuscation must keep the regular port advertised");
    assert!(missing_regular_advertisement.contains("regular peer port"));
}

#[test]
fn controller_profile_does_not_expose_native_type1_obfuscation_layers() {
    let slskd = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            // These are deliberately malformed slskdN-only layers. The
            // frozen slskd profile does not parse or expose them.
            .with("SLSKD_SLSK_OBFUSCATION", "not-a-boolean")
            .with("SLSKD_SLSK_OBFUSCATION_MODE", "not-a-mode")
            .with("SLSKD_SLSK_OBFUSCATION_LISTEN_PORT", "not-a-port")
            .with("SLSKR_OBFUSCATED_LISTENER_BIND", "127.0.0.1:50101")
            .with("SLSKR_OBFUSCATED_ADVERTISED_PORT", "not-a-port"),
    )
    .expect("slskd-only profile must ignore slskdN obfuscation layers");

    assert!(!slskd.obfuscation_enabled);
    assert_eq!(
        slskd.obfuscation_mode,
        super::SoulseekObfuscationMode::Compatibility
    );
    assert_eq!(slskd.obfuscation_listen_port, 0);
    assert!(slskd.obfuscation_advertise_regular_port);
    assert!(!slskd.obfuscation_prefer_outbound);
    assert!(slskd.obfuscated_listener_bind.is_none());
    assert!(slskd.obfuscated_advertised_port.is_none());
}

#[test]
fn pod_join_signature_modes_are_validated() {
    let default =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .expect("default pod signature config");
    assert_eq!(
        default.pod_join_signature_mode,
        super::PodSignatureMode::Off
    );

    for (value, expected) in [
        ("warn", super::PodSignatureMode::Warn),
        ("enforce", super::PodSignatureMode::Enforce),
    ] {
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", value),
        )
        .expect("supported pod signature mode");
        assert_eq!(config.pod_join_signature_mode, expected);
    }

    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", "accept-anything"),
    )
    .expect_err("invalid pod signature mode must fail");
    assert!(error.contains("off, warn, or enforce"), "{error}");
}

#[test]
fn virtual_soulfind_v2_defaults_disabled_and_honors_explicit_enable() {
    let default = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("default slskdN VirtualSoulfind v2 config");
    assert!(!default.virtual_soulfind_v2_enabled);
    assert!(default.acquisition_planning_enabled);

    let file_disabled = super::AppConfig::from_layers(
        None,
        super::FileConfig {
            virtual_soulfind_v2: super::VirtualSoulfindV2FileConfig {
                enabled: Some(false),
            },
            ..super::FileConfig::default()
        },
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("file-disabled VirtualSoulfind v2 config");
    assert!(!file_disabled.virtual_soulfind_v2_enabled);
    assert!(!file_disabled.acquisition_planning_enabled);

    let env_enabled = super::AppConfig::from_layers(
        None,
        super::FileConfig {
            virtual_soulfind_v2: super::VirtualSoulfindV2FileConfig {
                enabled: Some(false),
            },
            ..super::FileConfig::default()
        },
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"),
    )
    .expect("environment-enabled VirtualSoulfind v2 config");
    assert!(env_enabled.virtual_soulfind_v2_enabled);
    assert!(env_enabled.acquisition_planning_enabled);
    assert!(env_enabled
        .sanitized_json()
        .contains("\"virtual_soulfind_v2_enabled\":true"));
}

#[test]
fn federation_settings_match_target_defaults_file_layers_and_bounds() {
    let defaults =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .expect("default federation settings");
    assert!(!defaults.social_federation.enabled);
    assert_eq!(defaults.social_federation.mode, "Hermit");
    assert_eq!(defaults.social_federation.outbox_max_activities, 100);
    assert_eq!(defaults.social_federation.page_size, 20);
    assert!(defaults.social_federation.verify_signatures);
    assert_eq!(defaults.social_federation.http_timeout_seconds, 30);
    assert!(!defaults.federation_publishing.enabled);
    assert_eq!(
        defaults.federation_publishing.publishable_domains,
        ["music"]
    );
    assert_eq!(defaults.federation_publishing.default_visibility, "public");
    assert!(defaults.federation_publishing.require_moderation_approval);
    assert!(defaults.federation_publishing.include_external_links);
    assert_eq!(defaults.federation_publishing.max_metadata_size_kb, 10);

    let file = super::FileConfig {
        social_federation: super::SocialFederationFileConfig {
            enabled: Some(true),
            mode: Some("FriendsOnly".to_owned()),
            domain: Some("social.example".to_owned()),
            base_url: Some("https://social.example".to_owned()),
            approved_peers: vec!["peer-a".to_owned()],
            outbox_max_activities: Some(250),
            page_size: Some(40),
            verify_signatures: Some(false),
            http_timeout_seconds: Some(45),
        },
        federation_publishing: super::FederationPublishingFileConfig {
            enabled: Some(true),
            publishable_domains: vec!["music".to_owned(), "books".to_owned()],
            default_visibility: Some("circle".to_owned()),
            approved_circles: vec!["friends".to_owned()],
            require_moderation_approval: Some(false),
            include_external_links: Some(false),
            max_metadata_size_kb: Some(25),
        },
        ..super::FileConfig::default()
    };
    let configured = super::AppConfig::from_layers(
        None,
        file,
        &MapEnv::default()
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_PAGE_SIZE", "50")
            .with("FEDERATION_PUBLISHING_PUBLISHABLE_DOMAINS", "music;video")
            .with("SLSKR_FEDERATION_PUBLISHING_MAX_METADATA_SIZE_KB", "30"),
    )
    .expect("layered federation settings");
    assert!(configured.social_federation.enabled);
    assert_eq!(configured.social_federation.mode, "Public");
    assert_eq!(configured.social_federation.page_size, 50);
    assert_eq!(configured.social_federation.outbox_max_activities, 250);
    assert_eq!(configured.social_federation.approved_peers, ["peer-a"]);
    assert_eq!(
        configured.federation_publishing.publishable_domains,
        ["music", "video"]
    );
    assert_eq!(configured.federation_publishing.max_metadata_size_kb, 30);

    for (name, value) in [
        ("FEDERATION_PAGE_SIZE", "9"),
        ("FEDERATION_OUTBOX_MAX_ACTIVITIES", "1001"),
        ("FEDERATION_HTTP_TIMEOUT_SECONDS", "121"),
        ("FEDERATION_PUBLISHING_MAX_METADATA_SIZE_KB", "0"),
    ] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("out-of-range federation setting must fail startup");
        assert!(error.contains("federation"), "{name}={value}: {error}");
    }
}

#[test]
fn trusted_mesh_peers_are_bounded_pinned_and_redacted() {
    let value = serde_json::json!([{
        "peerId": "peer-a",
        "username": "mesh-user",
        "overlayEndpoint": "127.0.0.1:50305",
        "certificateSha256": "11".repeat(32),
        "rangeEndpoint": "https://mesh.example/content/{sha256}?ignored"
    }]);
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
    )
    .expect_err("query-bearing endpoint must fail");
    assert!(error.contains("query or fragment"), "{error}");

    let value = serde_json::json!([{
        "peerId": "peer-a",
        "username": "mesh-user",
        "overlayEndpoint": "127.0.0.1:50305",
        "certificateSha256": "11".repeat(32),
        "rangeEndpoint": "https://{recordingId}.example/content/{sha256}"
    }]);
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
    )
    .expect_err("authority placeholder must fail");
    assert!(error.contains("only in the path"), "{error}");

    let value = serde_json::json!([{
        "peerId": "peer-a",
        "username": "mesh-user",
        "overlayEndpoint": "127.0.0.1:50305",
        "certificateSha256": "11".repeat(32),
        "rangeEndpoint": "https://mesh.example/content/{sha256}/{size}/{recordingId}"
    }]);
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
    )
    .expect("trusted mesh config");
    let peer = &config.trusted_mesh_peers[0];
    assert!(peer.matches("PEER-A"));
    assert!(peer.matches("MESH-USER"));
    assert_eq!(peer.certificate_sha256, [0x11; 32]);
    assert_eq!(
        peer.range_url(&"a".repeat(64), 42, Some("recording-1")),
        Some(format!(
            "https://mesh.example/content/{}/42/recording-1",
            "a".repeat(64)
        ))
    );
    assert_eq!(
        peer.range_url(&"a".repeat(64), 42, Some("recording/../?next=1")),
        Some(format!(
            "https://mesh.example/content/{}/42/recording%2F..%2F%3Fnext%3D1",
            "a".repeat(64)
        ))
    );
    assert!(peer.range_url("not-a-sha256", 42, None).is_none());
    let sanitized = config.sanitized_json();
    assert!(sanitized.contains("\"trusted_mesh_peers\":1"));
    assert!(!sanitized.contains("mesh.example"));
    assert!(!sanitized.contains("mesh-user"));
    assert!(!sanitized.contains(&"11".repeat(32)));
}

#[test]
fn trusted_mesh_peer_config_rejects_ambiguous_or_unpinned_identities() {
    for value in [
        serde_json::json!([{
            "peerId": "peer-a",
            "username": "mesh-user",
            "overlayEndpoint": "127.0.0.1:0",
            "certificateSha256": "11".repeat(32)
        }]),
        serde_json::json!([{
            "peerId": "peer-a",
            "username": "mesh-user",
            "overlayEndpoint": "127.0.0.1:50305",
            "certificateSha256": "00".repeat(32)
        }]),
        serde_json::json!([
            {
                "peerId": "peer-a",
                "username": "mesh-user",
                "overlayEndpoint": "127.0.0.1:50305",
                "certificateSha256": "11".repeat(32)
            },
            {
                "peerId": "MESH-USER",
                "username": "other-user",
                "overlayEndpoint": "127.0.0.1:50306",
                "certificateSha256": "22".repeat(32)
            }
        ]),
    ] {
        super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
        )
        .expect_err("invalid trusted mesh peer must fail");
    }
}

#[test]
fn soulseek_profile_and_distributed_defaults_match_frozen_targets() {
    let config =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
            .unwrap();

    assert_eq!(config.user_info_picture, None);
    assert_eq!(
        config.soulseek_diagnostic_level,
        super::SoulseekDiagnosticLevel::Info
    );
    assert_eq!(
        config.soulseek_distributed,
        super::SoulseekDistributedSettings {
            disabled: false,
            disable_children: false,
            child_limit: 25,
            logging: false,
        }
    );
}

#[test]
fn soulseek_profile_and_distributed_layers_apply_in_one_contract() {
    let root = std::env::temp_dir().join(format!(
        "slskr-soulseek-profile-config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let picture = root.join("picture.bin");
    std::fs::write(&picture, [0_u8, 1, 2, 255]).unwrap();
    let file: super::FileConfig = serde_yaml::from_str(&format!(
            "profile:\n  user_info_picture: {}\n  soulseek_diagnostic_level: warning\nnetwork:\n  distributed_network:\n    disabled: true\n    disable_children: true\n    child_limit: 7\n    logging: true\n",
            picture.display()
        ))
        .unwrap();
    let file_config = super::AppConfig::from_layers(None, file, &MapEnv::default()).unwrap();
    assert_eq!(
        file_config.user_info_picture.as_deref(),
        Some(picture.as_path())
    );
    assert_eq!(
        file_config.soulseek_diagnostic_level,
        super::SoulseekDiagnosticLevel::Warning
    );
    assert_eq!(
        file_config.soulseek_distributed,
        super::SoulseekDistributedSettings {
            disabled: true,
            disable_children: true,
            child_limit: 7,
            logging: true,
        }
    );

    let environment = MapEnv::default()
        .with("SLSK_PICTURE", picture.to_str().unwrap())
        .with("SLSK_DIAG_LEVEL", "debug")
        .with("SLSK_NO_DNET", "false")
        .with("SLSK_DNET_NO_CHILDREN", "false")
        .with("SLSK_DNET_CHILDREN", "31")
        .with("SLSK_DNET_LOGGING", "false");
    let environment_config =
        super::AppConfig::from_layers(None, super::FileConfig::default(), &environment).unwrap();
    assert_eq!(
        environment_config.soulseek_diagnostic_level,
        super::SoulseekDiagnosticLevel::Debug
    );
    assert_eq!(
        environment_config.soulseek_distributed,
        super::SoulseekDistributedSettings {
            disabled: false,
            disable_children: false,
            child_limit: 31,
            logging: false,
        }
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn soulseek_profile_and_distributed_validation_is_bounded() {
    for level in ["", "INFOO"] {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_DIAG_LEVEL", level),
        )
        .expect_err("invalid diagnostic level must fail");
        assert!(error.contains("diagnostic"), "{error}");
    }
    let trace = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSK_DIAG_LEVEL", "trace"),
    )
    .expect("the frozen runtime enum accepts trace");
    assert_eq!(
        trace.soulseek_diagnostic_level,
        super::SoulseekDiagnosticLevel::Trace
    );
    for limit in ["0".to_owned(), (i64::from(i32::MAX) + 1).to_string()] {
        super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_DNET_CHILDREN", &limit),
        )
        .expect_err("invalid distributed child limit must fail");
    }

    let missing =
        std::env::temp_dir().join(format!("slskr-missing-picture-{}", std::process::id()));
    super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSK_PICTURE", missing.to_str().unwrap()),
    )
    .expect_err("missing picture must fail");
    super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default().with("SLSK_PICTURE", std::env::temp_dir().to_str().unwrap()),
    )
    .expect_err("picture directory must fail");
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
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
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
        super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
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
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
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
        super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("invalid core workflow setting must fail startup");
    }
}

#[test]
fn advanced_networking_security_contracts_load_as_one_runtime_policy() {
    let root = std::env::temp_dir().join(format!(
        "slskr-advanced-networking-config-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        r#"dht:
  enabled: true
  dht_port: 51001
  overlay_port: 51002
  advertised_overlay_port: 51003
  vpn_port_sync: target_port
  bootstrap_routers: [router.example:6881]
  announce_interval_seconds: 120
  discovery_interval_seconds: 90
  min_neighbors: 7
  bootstrap_timeout_seconds: 20
  cold_bootstrap_timeout_seconds: 30
  lan_only_bootstrap_timeout_seconds: 10
  lan_only: true
  enable_upnp: true
  enable_stun: false
mesh:
  enabled: true
  EnableOverlay: false
  EnableDht: false
  EnableStun: false
  enable_soulseek_capability_handshake: false
  enable_soulseek_rendezvous: true
  probe_soulseek_rendezvous_capabilities: false
  dht: { bootstrap_nodes: 17 }
  overlay: { udp_port: 51004, quic_port: 51005 }
  security: { enforceRemotePayloadLimits: true, maxRemotePayloadSize: 262144 }
  sync_security:
    max_invalid_entries_per_window: 8
    max_invalid_messages_per_window: 4
    rate_limit_window_minutes: 2
    quarantine_violation_threshold: 2
    quarantine_duration_minutes: 11
    proof_of_possession_enabled: true
    consensus_min_peers: 4
    consensus_min_agreements: 2
    alert_threshold_signature_failures: 9
    alert_threshold_rate_limit_violations: 8
    alert_threshold_quarantine_events: 7
SignalSystem:
  Enabled: false
  DeduplicationCacheSize: 2048
  DefaultTtl: "00:07:30"
  MeshChannel:
    Enabled: false
    Priority: 3
    RequireActiveSession: true
  BtExtensionChannel:
    Enabled: true
    Priority: 4
    RequireActiveSession: false
PodCore:
  Join: { SignatureMode: warn }
  Security: { SignatureMode: enforce }
overlay:
  enable: true
  listen_port: 51006
  enable_quic: true
  quic_listen_port: 51007
  share_quic_with_dht_port: false
  quic_backend_listen_port: 51008
  trusted_certificate_pins: { "127.0.0.1:51007": [pin-value] }
overlay_data:
  enable: true
  listen_port: 51009
  share_with_dht_port: false
  backend_listen_port: 51010
  max_concurrent_streams: 7
  relay_authentication_token: overlay-token
  allowed_relay_destinations: ["8.8.8.8:443"]
  max_concurrent_relays: 3
  max_relay_bytes_per_direction: 123456
  max_relay_duration_seconds: 45
  trusted_certificate_pins: { "127.0.0.1:51009": [data-pin] }
relay:
  enabled: true
  mode: controller
  controller:
    address: https://controller.example
    ignore_certificate_errors: true
    api_key: 1234567890abcdef
    secret: abcdef1234567890
    downloads: true
  agents:
    edge:
      instance_name: edge-one
      secret: 0123456789abcdef
      cidr: 127.0.0.1/32
security:
  enabled: true
  profile: Custom
  network_guard:
    enabled: true
    max_connections_per_ip: 12
    max_global_connections: 345
    max_messages_per_minute: 67
    max_message_size: 8192
  path_guard: { enabled: true, max_path_length: 333, max_path_depth: 13 }
  content_safety:
    enabled: true
    verify_magic_bytes: false
    quarantine_suspicious: false
    quarantine_directory: /tmp/quarantine
    block_executables: false
  peer_reputation: { enabled: true, trusted_threshold: 80, untrusted_threshold: 10 }
  violation_tracker: { enabled: true, violations_before_auto_ban: 3, base_ban_duration_minutes: 15 }
  adversarial:
    privacy: { padding: { max_unpadded_bytes: 1024, max_padded_bytes: 2048 } }
    anonymity:
      relay_only:
        relay_peer_data_endpoints: ["8.8.4.4:443"]
        relay_authentication_token: anonymity-token
"#,
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    let advanced = &config.advanced_networking;
    assert_eq!(advanced.dht.dht_port, 51_001);
    assert_eq!(advanced.dht.effective_overlay_port(), 51_003);
    assert_eq!(advanced.dht.vpn_port_sync, "target_port");
    assert!(advanced.dht.lan_only);
    assert_eq!(advanced.mesh.dht_bootstrap_nodes, 17);
    assert!(!advanced.mesh.enable_overlay);
    assert!(!advanced.mesh.enable_dht);
    assert!(!advanced.mesh.enable_stun);
    assert!(!advanced.mesh.enable_soulseek_capability_handshake);
    assert_eq!(advanced.mesh.max_remote_payload_size, 262_144);
    assert!(!advanced.signal_system.enabled);
    assert_eq!(advanced.signal_system.deduplication_cache_size, 2_048);
    assert_eq!(advanced.signal_system.default_ttl.as_secs(), 450);
    assert!(!advanced.signal_system.mesh_channel.enabled);
    assert_eq!(advanced.signal_system.mesh_channel.priority, 3);
    assert!(advanced.signal_system.mesh_channel.require_active_session);
    assert!(advanced.signal_system.bt_extension_channel.enabled);
    assert_eq!(advanced.signal_system.bt_extension_channel.priority, 4);
    assert!(
        !advanced
            .signal_system
            .bt_extension_channel
            .require_active_session
    );
    assert_eq!(advanced.mesh_sync_security.consensus_min_agreements, 2);
    assert_eq!(
        advanced.pod_join_signature_mode,
        super::PodSignatureMode::Warn
    );
    assert_eq!(
        advanced.pod_security_signature_mode,
        super::PodSignatureMode::Enforce
    );
    assert_eq!(advanced.overlay.quic_backend_listen_port, 51_008);
    assert!(!advanced.overlay_data.share_with_dht_port);
    assert_eq!(advanced.overlay_data.backend_listen_port, 51_010);
    assert_eq!(advanced.overlay_data.max_concurrent_streams, 7);
    assert_eq!(advanced.overlay_data.max_concurrent_relays, 3);
    assert!(advanced.relay.enabled);
    assert_eq!(advanced.relay.agents["edge"].instance_name, "edge-one");
    assert_eq!(advanced.security.network_guard.max_global_connections, 345);
    assert_eq!(advanced.security.path_guard.max_path_depth, 13);
    assert_eq!(advanced.security.peer_reputation.trusted_threshold, 80);
    assert_eq!(advanced.security.adversarial.max_padded_bytes, 2048);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn advanced_networking_validation_rejects_inconsistent_security_limits() {
    let root = std::env::temp_dir().join(format!(
        "slskr-advanced-networking-invalid-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        "Mesh:\n  sync_security:\n    consensus_min_peers: 2\n    consensus_min_agreements: 3\n",
    )
    .unwrap();
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect_err("inconsistent consensus must fail");
    assert!(error.contains("sync_security"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn signal_system_environment_layers_and_bounds_match_slskdn() {
    let file = super::FileConfig {
        signal_system: super::SignalSystemFileConfig {
            enabled: Some(false),
            deduplication_cache_size: Some(2_000),
            default_ttl: Some(super::SignalDurationFileValue::Text("00:06:00".to_owned())),
            mesh_channel: super::SignalChannelFileConfig {
                enabled: Some(false),
                priority: Some(4),
                require_active_session: Some(true),
            },
            bt_extension_channel: super::SignalChannelFileConfig {
                enabled: Some(false),
                priority: Some(5),
                require_active_session: Some(false),
            },
        },
        ..super::FileConfig::default()
    };
    let config = super::AppConfig::from_layers(
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
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with(name, value),
        )
        .expect_err("invalid SignalSystem setting must fail startup");
        assert!(error.contains("SignalSystem"), "{name}={value}: {error}");
    }
}

#[test]
fn media_advanced_service_contracts_load_as_one_runtime_policy() {
    let root = std::env::temp_dir().join(format!(
        "slskr-media-advanced-service-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        r#"feature:
  collectionsSharing: false
  streaming: false
  streamingRelayFallback: false
  meshParallelSearch: false
  meshPublishAvailability: false
  identityFriends: false
  solid: false
  scenePodBridge: true
  scenePodBridgeOptions:
    proxyTransfers: true
    exportPodAvailability: true
  songId: false
  mesh: false
  dht: false
  pods: false
  socialFederation: false
  virtualSoulfind: false
  multiSourceDownloads: false
player:
  external_visualizer:
    enabled: true
    path: /bin/echo
    arguments: [visualizer, --fixture]
    working_directory: /tmp
    name: Fixture Visualizer
solid:
  allowInsecureHttp: true
  allowLocalhostForWebId: true
  maxFetchBytes: 7654321
  timeoutSeconds: 23
  allowedHosts: [Pod.Example., identity.example]
  clientIdUrl: https://solid.example/clientid.jsonld
  redirectPath: /fixture/callback
song_id:
  max_concurrent_runs: 7
virtualSoulfind:
  bridge:
    enabled: true
    port: 4322
    bindAddress: 127.0.0.2
    maxClients: 17
    requireAuth: true
    password: fixture-secret
    maxRequestsPerMinute: 71
    maxTransfersPerSession: 19
  disasterMode:
    auto: true
    force: true
    unavailableThresholdMinutes: 13
    enableGracefulDegradation: false
    recoveryCheckIntervalMinutes: 11
    recoveryHealthyChecksRequired: 5
"#,
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    let media = &config.media_services;
    let feature = &media.features;
    assert!(!feature.collections_sharing);
    assert!(!feature.streaming);
    assert!(!feature.streaming_relay_fallback);
    assert!(!feature.mesh_parallel_search);
    assert!(!feature.mesh_publish_availability);
    assert!(!feature.identity_friends);
    assert!(!feature.solid);
    assert!(feature.scene_pod_bridge);
    assert!(feature.scene_pod_bridge_proxy_transfers);
    assert!(feature.scene_pod_bridge_export_pod_availability);
    assert!(!feature.song_id);
    assert!(!feature.mesh);
    assert!(!feature.dht);
    assert!(!feature.pods);
    assert!(!feature.social_federation);
    assert!(!feature.virtual_soulfind);
    assert!(!feature.multi_source_downloads);

    let visualizer = &media.external_visualizer;
    assert!(visualizer.launch_enabled);
    assert_eq!(visualizer.command.as_deref(), Some("/bin/echo"));
    assert_eq!(visualizer.arguments, ["visualizer", "--fixture"]);
    assert_eq!(
        visualizer.working_directory.as_deref(),
        Some(std::path::Path::new("/tmp"))
    );
    assert_eq!(visualizer.name, "Fixture Visualizer");

    assert!(media.solid.allow_insecure_http);
    assert!(media.solid.allow_localhost_for_web_id);
    assert_eq!(media.solid.max_fetch_bytes, 7_654_321);
    assert_eq!(media.solid.timeout.as_secs(), 23);
    assert_eq!(
        media.solid.allowed_hosts,
        ["pod.example", "identity.example"]
    );
    assert_eq!(media.solid.redirect_path, "/fixture/callback");
    assert_eq!(
        media.solid.client_id_url.as_deref(),
        Some("https://solid.example/clientid.jsonld")
    );
    assert_eq!(media.song_id_max_concurrent_runs, 7);

    let bridge = &media.virtual_soulfind.bridge;
    assert!(bridge.enabled);
    assert_eq!(bridge.port, 4322);
    assert_eq!(
        bridge.bind_address,
        "127.0.0.2".parse::<std::net::IpAddr>().unwrap()
    );
    assert_eq!(bridge.max_clients, 17);
    assert!(bridge.require_auth);
    assert_eq!(bridge.password, "fixture-secret");
    assert_eq!(bridge.max_requests_per_minute, 71);
    assert_eq!(bridge.max_transfers_per_session, 19);

    let disaster = &media.virtual_soulfind.disaster_mode;
    assert!(disaster.auto);
    assert!(disaster.force);
    assert_eq!(disaster.unavailable_threshold.as_secs(), 13 * 60);
    assert!(!disaster.enable_graceful_degradation);
    assert_eq!(disaster.recovery_check_interval.as_secs(), 11 * 60);
    assert_eq!(disaster.recovery_healthy_checks_required, 5);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_advanced_service_validation_rejects_unsafe_values() {
    let root = std::env::temp_dir().join(format!(
        "slskr-media-advanced-service-invalid-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("slskd.yml"),
        "virtualSoulfind:\n  bridge:\n    enabled: true\n    requireAuth: true\n    password: ''\n",
    )
    .unwrap();
    let error = super::AppConfig::from_layers(
        None,
        super::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect_err("an enabled authenticated bridge requires a password");
    assert!(error.contains("virtualSoulfind.bridge"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}
