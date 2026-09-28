use super::*;
#[test]
fn mesh_gateway_settings_match_frozen_defaults_and_validation() {
    let disabled = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
    .unwrap();
    assert!(!disabled.mesh_gateway.enabled);
    assert!(disabled.mesh_gateway.allowed_services.is_empty());
    assert_eq!(disabled.mesh_gateway.max_request_body_bytes, 1_048_576);
    assert_eq!(disabled.mesh_gateway.request_timeout_seconds, 30);

    let enabled = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let file = serde_yaml::from_str::<crate::config::FileConfig>(
        "MeshGateway:\n  Enabled: true\n  AllowedServices: [pods]\n  MaxRequestBodyBytes: 8192\n",
    )
    .unwrap();
    let from_file = crate::config::AppConfig::from_layers(None, file, &MapEnv::default()).unwrap();
    assert!(from_file.mesh_gateway.enabled);
    assert_eq!(from_file.mesh_gateway.allowed_services, vec!["pods"]);
    assert_eq!(from_file.mesh_gateway.max_request_body_bytes, 8192);

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKD_MESH_GATEWAY_ENABLED", "true"),
    )
    .expect_err("enabled gateway without services must fail validation");
    assert!(error.contains("AllowedServices"), "{error}");

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKD_MESH_GATEWAY_BIND_ADDRESS", "0.0.0.0"),
    )
    .expect_err("remote gateway without an API key must fail validation");
    assert!(error.contains("ApiKey"), "{error}");

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKD_MESH_GATEWAY_BIND_ADDRESS", "0.0.0.0")
            .with("SLSKD_MESH_GATEWAY_API_KEY", "gateway-key"),
    )
    .expect_err("remote gateway without risk acknowledgment must fail validation");
    assert!(error.contains("IUnderstandTheRisk"), "{error}");

    let remote = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
fn frozen_listener_defaults_bind_the_projected_unspecified_address_and_port() {
    for target in ["slskd", "slskdn"] {
        let config = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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
    let config =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
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

    let frozen = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let dedicated = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &env.clone().with("SLSKR_OVERLAY_BIND", "127.0.0.1:50305"),
    )
    .expect("explicit dedicated overlay bind");
    assert_eq!(
        dedicated.overlay_bind,
        Some("127.0.0.1:50305".parse().unwrap())
    );
    assert_eq!(dedicated.advanced_networking.dht.overlay_port, 50_305);
    assert!(!dedicated.shared_mesh_tcp());

    let custom_tcp = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &base
            .clone()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKD_NO_CONNECT", "false"),
    )
    .expect("frozen slskd permits a loopback listener while connecting");
    assert!(slskd.auto_connect);

    let disconnected = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &base
            .clone()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_NO_CONNECT", "true"),
    )
    .expect("frozen slskdN permits a loopback listener when no-connect is set");
    assert!(!disconnected.auto_connect);

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
fn peer_response_timeout_rejects_zero() {
    let env = MapEnv::default().with("SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS", "0");
    let error =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
            .expect_err("zero peer timeout should fail");
    assert!(error.contains("must be greater than zero"), "{error}");
}

#[test]
fn soulseek_connection_defaults_bounds_and_target_difference_are_exact() {
    let slskdn = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with(name, value),
        )
        .expect_err("out-of-range Soulseek connection value");
        assert!(error.contains("must be between"), "{name}: {error}");
    }

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let config =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
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

    let blank = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("invalid auto-response config");
        assert!(error.contains("auto response") || error.contains("auto-response"));
    }
}

#[test]
fn soulseek_obfuscation_defaults_to_regular_first_compatibility() {
    let compatibility = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
    .expect("default obfuscation config");
    assert!(compatibility.obfuscation_enabled);
    assert_eq!(
        compatibility.obfuscation_mode,
        crate::config::SoulseekObfuscationMode::Compatibility
    );
    assert!(compatibility.obfuscation_prefer_outbound);
    assert!(compatibility.obfuscation_advertise_regular_port);
    assert!(!compatibility.prefer_obfuscated_outbound());

    let prefer = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
    )
    .expect("prefer obfuscation config");
    assert!(prefer.prefer_obfuscated_outbound());

    for value in ["only", "unknown"] {
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with("SLSK_OBFUSCATION_MODE", value),
        )
        .expect_err("unsupported obfuscation mode must fail");
        assert!(
            error.to_ascii_lowercase().contains("obfuscation"),
            "{error}"
        );
    }

    let missing_regular_advertisement = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "false"),
    )
    .expect_err("enabled obfuscation must keep the regular port advertised");
    assert!(missing_regular_advertisement.contains("regular peer port"));
}

#[test]
fn controller_profile_does_not_expose_native_type1_obfuscation_layers() {
    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        crate::config::SoulseekObfuscationMode::Compatibility
    );
    assert_eq!(slskd.obfuscation_listen_port, 0);
    assert!(slskd.obfuscation_advertise_regular_port);
    assert!(!slskd.obfuscation_prefer_outbound);
    assert!(slskd.obfuscated_listener_bind.is_none());
    assert!(slskd.obfuscated_advertised_port.is_none());
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
        )
        .expect_err("invalid trusted mesh peer must fail");
    }
}

#[test]
fn soulseek_profile_and_distributed_defaults_match_frozen_targets() {
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
    .unwrap();

    assert_eq!(config.user_info_picture, None);
    assert_eq!(
        config.soulseek_diagnostic_level,
        crate::config::SoulseekDiagnosticLevel::Info
    );
    assert_eq!(
        config.soulseek_distributed,
        crate::config::SoulseekDistributedSettings {
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
    let file: crate::config::FileConfig = serde_yaml::from_str(&format!(
            "profile:\n  user_info_picture: {}\n  soulseek_diagnostic_level: warning\nnetwork:\n  distributed_network:\n    disabled: true\n    disable_children: true\n    child_limit: 7\n    logging: true\n",
            picture.display()
        ))
        .unwrap();
    let file_config =
        crate::config::AppConfig::from_layers(None, file, &MapEnv::default()).unwrap();
    assert_eq!(
        file_config.user_info_picture.as_deref(),
        Some(picture.as_path())
    );
    assert_eq!(
        file_config.soulseek_diagnostic_level,
        crate::config::SoulseekDiagnosticLevel::Warning
    );
    assert_eq!(
        file_config.soulseek_distributed,
        crate::config::SoulseekDistributedSettings {
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
    let environment_config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &environment,
    )
    .unwrap();
    assert_eq!(
        environment_config.soulseek_diagnostic_level,
        crate::config::SoulseekDiagnosticLevel::Debug
    );
    assert_eq!(
        environment_config.soulseek_distributed,
        crate::config::SoulseekDistributedSettings {
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with("SLSK_DIAG_LEVEL", level),
        )
        .expect_err("invalid diagnostic level must fail");
        assert!(error.contains("diagnostic"), "{error}");
    }
    let trace = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSK_DIAG_LEVEL", "trace"),
    )
    .expect("the frozen runtime enum accepts trace");
    assert_eq!(
        trace.soulseek_diagnostic_level,
        crate::config::SoulseekDiagnosticLevel::Trace
    );
    for limit in ["0".to_owned(), (i64::from(i32::MAX) + 1).to_string()] {
        crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with("SLSK_DNET_CHILDREN", &limit),
        )
        .expect_err("invalid distributed child limit must fail");
    }

    let missing =
        std::env::temp_dir().join(format!("slskr-missing-picture-{}", std::process::id()));
    crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSK_PICTURE", missing.to_str().unwrap()),
    )
    .expect_err("missing picture must fail");
    crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSK_PICTURE", std::env::temp_dir().to_str().unwrap()),
    )
    .expect_err("picture directory must fail");
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        crate::config::PodSignatureMode::Warn
    );
    assert_eq!(
        advanced.pod_security_signature_mode,
        crate::config::PodSignatureMode::Enforce
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect_err("inconsistent consensus must fail");
    assert!(error.contains("sync_security"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}
