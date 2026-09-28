use super::*;

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

    let yaml = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKD_WEB_MAX_REQUEST_BODY_SIZE", "6291456"),
    )
    .unwrap();
    assert_eq!(yaml.controller_web_max_request_body_size, 7 * 1024 * 1024);
    std::fs::remove_dir_all(root).unwrap();

    let slskdn = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
    .unwrap();
    assert_eq!(
        slskdn.controller_web_max_request_body_size,
        10 * 1024 * 1024
    );
    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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

    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKD_WEB_CORS_ENABLED", "false")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "https://ignored.example"),
    )
    .unwrap();
    assert_eq!(
        config.controller_web_cors,
        crate::config::ControllerWebCorsSettings {
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

    let defaults = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
    .unwrap();
    assert_eq!(
        defaults.controller_web_cors,
        crate::config::ControllerWebCorsSettings::default()
    );

    let unsafe_cors = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let enforced_explicit = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let nonloopback_config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let missing_cidrs_config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    assert!(config.controller_diagnostics_allow_memory_dump);
    assert!(config.controller_diagnostics_allow_remote_dump);
    let environment_override = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let environment_only = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let unsafe_dump = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let weak_metrics = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let hash_from_audio = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKD_WEB_API_PERMIT_LIMIT", "202"),
    )
    .unwrap();
    assert_eq!(
        config.controller_web_rate_limiting,
        crate::config::ControllerWebRateLimitingSettings {
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

    let slskdn = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
    .unwrap();
    assert_eq!(slskdn.controller_web_rate_limiting.api_permit_limit, 200);
    assert!(slskdn.controller_web_rate_limiting.enabled);
    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true"),
    )
    .unwrap();
    assert!(!slskd.controller_web_rate_limiting.enabled);

    let disabled_negative = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let enabled_zero = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
fn frozen_web_bind_profiles_preserve_multi_address_and_target_specific_names() {
    let controller_default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let controller_multi = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let slskdn = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.1,not-an-ip"),
    )
    .expect_err("invalid frozen slskd web IP list must fail");
    assert!(error.contains("SLSKD_HTTP_IP_ADDRESS"), "{error}");
}

#[test]
fn config_sensitive_value_detection_covers_secrets() {
    let empty = crate::config::FileConfig::default();
    assert!(!crate::config::config_contains_sensitive_values(&empty));

    let with_api_token = crate::config::FileConfig {
        auth: crate::config::AuthFileConfig {
            api_token: Some("token".to_owned()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(crate::config::config_contains_sensitive_values(
        &with_api_token
    ));

    let with_integration_secret = crate::config::FileConfig {
        integrations: crate::config::IntegrationsFileConfig {
            spotify: crate::config::SpotifyFileConfig {
                client_secret: Some("secret".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(crate::config::config_contains_sensitive_values(
        &with_integration_secret
    ));
}

#[test]
fn trusted_proxy_cidrs_parse_from_env_and_file() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32,::1/128");
    let config =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
            .expect("trusted proxy env config");
    assert_eq!(config.trusted_proxy_cidrs.len(), 2);
    assert!(config.trusted_proxy_cidrs[0].contains("127.0.0.1".parse().unwrap()));
    assert!(config.trusted_proxy_cidrs[1].contains("::1".parse().unwrap()));

    let file_config = crate::config::FileConfig {
        auth: crate::config::AuthFileConfig {
            trusted_proxy_cidrs: vec!["10.0.0.0/8".to_owned()],
            ..Default::default()
        },
        ..Default::default()
    };
    let config = crate::config::AppConfig::from_layers(None, file_config, &MapEnv::default())
        .expect("trusted proxy file config");
    assert!(config.trusted_proxy_cidrs[0].contains("10.1.2.3".parse().unwrap()));
}

#[test]
fn api_token_rejects_blank_env_and_file_values() {
    for token in ["", " \t\r\n"] {
        let env = MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", token);
        let error =
            crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
                .expect_err("blank environment API token must fail");
        assert!(error.contains("must not be empty"));

        let file_config = crate::config::FileConfig {
            auth: crate::config::AuthFileConfig {
                disabled: Some(false),
                api_token: Some(token.to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        let error = crate::config::AppConfig::from_layers(None, file_config, &MapEnv::default())
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
    let config =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
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

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_API_TOKEN", "duplicate-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "duplicate-token"),
    )
    .expect_err("role tokens must not alias");
    assert_eq!(error, "API tokens for different roles must be distinct");
}

#[test]
fn sanitized_config_is_valid_json() {
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("default config");

    serde_json::from_str::<serde_json::Value>(&config.sanitized_json())
        .expect("sanitized config must remain valid JSON");
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
        let error =
            crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
                .expect_err("unrepresentable environment API token must fail");
        assert!(
            error.contains("whitespace") || error.contains("control"),
            "{error}"
        );

        let file_config = crate::config::FileConfig {
            auth: crate::config::AuthFileConfig {
                disabled: Some(false),
                api_token: Some(token.to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        let error = crate::config::AppConfig::from_layers(None, file_config, &MapEnv::default())
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_API_TOKEN", &maximum),
    )
    .expect("maximum representable token");
    assert_eq!(config.api_token.as_deref(), Some(maximum.as_str()));

    let oversized = format!("{maximum}x");
    let env = MapEnv::default().with("SLSKR_API_TOKEN", &oversized);
    let error =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
            .expect_err("oversized environment token must fail");
    assert!(error.contains("maximum representable"), "{error}");

    let file_config = crate::config::FileConfig {
        auth: crate::config::AuthFileConfig {
            api_token: Some(oversized),
            ..Default::default()
        },
        ..Default::default()
    };
    let error = crate::config::AppConfig::from_layers(None, file_config, &MapEnv::default())
        .expect_err("oversized file token must fail");
    assert!(error.contains("maximum representable"), "{error}");
}

#[test]
fn trusted_proxy_cidrs_reject_invalid_prefixes() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/33");
    let error =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
            .expect_err("invalid trusted proxy prefix should fail");
    assert!(error.contains("prefix exceeds"));
}

#[test]
fn controller_swagger_defaults_split_by_target_and_honors_environment() {
    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd swagger default");
    assert!(!slskd.controller_swagger);

    let slskdn = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("slskdN swagger default");
    assert!(slskdn.controller_swagger);

    let disabled = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd metrics defaults");
    assert!(!slskd.controller_metrics_enabled);
    assert_eq!(slskd.controller_metrics_url, "/metrics");
    assert_eq!(slskd.controller_metrics_username, "slskd");
    assert!(slskd.controller_metrics_password.is_empty());

    let native = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("native disabled metrics defaults");
    assert!(!native.controller_metrics_enabled);
    assert_eq!(native.controller_metrics_username, "slskr");
    assert!(native.controller_metrics_password.is_empty());

    let missing_password = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_METRICS", "true"),
    );
    assert!(missing_password.is_err());

    let configured = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let controller_default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("slskd compatibility default authentication");
    assert_eq!(controller_default.controller_web_auth_password, "slskd");

    let native_default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("slskdN compatibility default authentication");
    assert_eq!(native_default.controller_web_auth_username, "slskr");
    assert_eq!(native_default.controller_web_auth_password, "slskr");

    let disabled = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("disabled web authentication does not need credentials");
    assert!(disabled.controller_web_auth_password.is_empty());

    let configured = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_USERNAME", "admin")
            .with("SLSKD_PASSWORD", "configured-secret"),
    )
    .expect("explicit web credentials");
    assert_eq!(configured.controller_web_auth_username, "admin");
    assert_eq!(configured.controller_web_auth_password, "configured-secret");
}
