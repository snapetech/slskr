//! Controller full native api contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_controller_regex_timeout_is_fail_closed() {
    let matcher = crate::ControllerRegex::compile_with_timeout(
        r"^(?=(a+)+$).*$",
        true,
        Some(crate::NATIVE_REGEX_MATCH_TIMEOUT),
    )
    .expect("pathological regex");
    let hostile_username = format!("{}!", "a".repeat(255));
    let started = Instant::now();

    assert!(!matcher.is_match(&hostile_username));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "regex timeout guard exceeded the bounded execution window"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_no_auth_passthrough_is_loopback_or_explicit_cidr_only() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
            .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", "192.0.2.0/24"),
    )
    .expect("no-auth passthrough config");
    let check = |address: &str, path: &str| {
        crate::routing::check_route_auth(
            &config,
            "GET",
            path,
            None,
            &crate::RequestSecurityHeaders {
                remote_addr: Some(address.parse().unwrap()),
                ..Default::default()
            },
        )
    };

    assert_eq!(check("127.0.0.1:1", "/api/v0/application"), Ok(()));
    assert_eq!(check("192.0.2.25:1", "/api/v0/application"), Ok(()));
    assert_eq!(
        check("198.51.100.25:1", "/api/v0/application"),
        Err("unauthorized")
    );
    assert_eq!(check("198.51.100.25:1", "/api/health"), Ok(()));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_request_body_limit_validation_and_projection_match_frozen_shape() {
    for invalid in [
        serde_json::json!({"web": {"max_request_body_size": 0}}),
        serde_json::json!({"web": {"max_request_body_size": -1}}),
        serde_json::json!({"web": {"max_request_body_size": 2147483648_i64}}),
        serde_json::json!({"web": {"max_request_body_size": []}}),
        serde_json::json!({"web": []}),
    ] {
        assert_eq!(
            crate::controller_yaml_target_validation_error(
                &invalid,
                crate::ControllerProfile::Native,
            ),
            Some("Invalid YAML configuration".to_owned())
        );
    }
    for valid in [
        serde_json::json!({"web": {"max_request_body_size": null}}),
        serde_json::json!({"web": {"max_request_body_size": 1}}),
        serde_json::json!({"web": {"max_request_body_size": "2147483647"}}),
    ] {
        assert_eq!(
            crate::controller_yaml_target_validation_error(
                &valid,
                crate::ControllerProfile::Native,
            ),
            None
        );
    }

    let projected = crate::controller_yaml_api_projection(serde_json::json!({
        "web": {"max_request_body_size": "7340032"}
    }));
    assert_eq!(projected["web"]["maxRequestBodySize"], 7 * 1024 * 1024);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_cors_validation_and_projection_match_frozen_shape() {
    let valid = serde_json::json!({
        "diagnostics": {
            "allow_memory_dump": "true",
            "allow_remote_dump": false
        },
        "web": {
            "enforce_security": "true",
            "allow_remote_no_auth": "false",
            "authentication": {
                "disabled": "true",
                "passthrough": {"allowed_cidrs": 123}
            },
            "cors": {
                "enabled": "true",
                "allow_credentials": false,
                "allowed_origins": ["https://allowed.example"],
                "allowed_headers": ["X-Custom"],
                "allowed_methods": null
            }
        }
    });
    assert_eq!(
        crate::controller_yaml_target_validation_error(&valid, crate::ControllerProfile::Native,),
        None
    );
    for invalid in [
        serde_json::json!({"web": {"cors": []}}),
        serde_json::json!({"web": {"cors": {"enabled": "yes"}}}),
        serde_json::json!({"web": {"cors": {"allow_credentials": 1}}}),
        serde_json::json!({"web": {"cors": {"allowed_origins": "*"}}}),
        serde_json::json!({"web": {"cors": {"allowed_methods": {}}}}),
        serde_json::json!({"web": {"enforce_security": "yes"}}),
        serde_json::json!({"web": {"allow_remote_no_auth": 1}}),
        serde_json::json!({"web": {"authentication": []}}),
        serde_json::json!({"web": {"authentication": {"disabled": "yes"}}}),
        serde_json::json!({"web": {"authentication": {"passthrough": []}}}),
        serde_json::json!({"web": {"authentication": {"passthrough": {"allowed_cidrs": []}}}}),
        serde_json::json!({"diagnostics": []}),
        serde_json::json!({"diagnostics": {"allow_memory_dump": "yes"}}),
        serde_json::json!({"diagnostics": {"allow_remote_dump": 1}}),
    ] {
        assert_eq!(
            crate::controller_yaml_target_validation_error(
                &invalid,
                crate::ControllerProfile::Native,
            ),
            Some("Invalid YAML configuration".to_owned())
        );
    }

    let projected = crate::controller_yaml_api_projection(valid);
    assert_eq!(projected["diagnostics"]["allowMemoryDump"], true);
    assert_eq!(projected["diagnostics"]["allowRemoteDump"], false);
    assert_eq!(projected["web"]["enforceSecurity"], true);
    assert_eq!(projected["web"]["allowRemoteNoAuth"], false);
    assert_eq!(projected["web"]["authentication"]["disabled"], true);
    assert_eq!(
        projected["web"]["authentication"]["passthrough"]["allowedCidrs"],
        "123"
    );
    assert_eq!(projected["web"]["cors"]["enabled"], true);
    assert_eq!(projected["web"]["cors"]["allowCredentials"], false);
    assert_eq!(
        projected["web"]["cors"]["allowedOrigins"],
        serde_json::json!(["https://allowed.example"])
    );
    assert!(projected["web"]["cors"].get("allowedMethods").is_none());

    let scalar_array = serde_json::json!({
        "web": {"cors": {"allowed_headers": ["X-Good", 1, true]}}
    });
    assert_eq!(
        crate::controller_yaml_target_validation_error(
            &scalar_array,
            crate::ControllerProfile::Native,
        ),
        None
    );
    let scalar_array = crate::controller_yaml_api_projection(scalar_array);
    assert_eq!(
        scalar_array["web"]["cors"]["allowedHeaders"],
        serde_json::json!(["X-Good", "1", "true"])
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_rate_limit_validation_projection_and_nonpositive_runtime_match_frozen() {
    let valid = serde_json::json!({
        "web": {
            "rate_limiting": {
                "enabled": "true",
                "api_permit_limit": 0,
                "api_window_seconds": -1,
                "federation_permit_limit": "-2",
                "federation_window_seconds": null,
                "mesh_gateway_permit_limit": 2147483647_i64,
                "mesh_gateway_window_seconds": -2147483648_i64
            }
        }
    });
    assert_eq!(
        crate::controller_yaml_target_validation_error(&valid, crate::ControllerProfile::Native,),
        None
    );
    for invalid in [
        serde_json::json!({"web": {"rate_limiting": []}}),
        serde_json::json!({"web": {"rate_limiting": {"enabled": "nope"}}}),
        serde_json::json!({"web": {"rate_limiting": {"api_permit_limit": 1.5}}}),
        serde_json::json!({"web": {"rate_limiting": {"api_window_seconds": 2147483648_i64}}}),
        serde_json::json!({"web": {"rate_limiting": {"federation_permit_limit": []}}}),
    ] {
        assert_eq!(
            crate::controller_yaml_target_validation_error(
                &invalid,
                crate::ControllerProfile::Native,
            ),
            Some("Invalid YAML configuration".to_owned())
        );
    }

    let projected = crate::controller_yaml_api_projection(valid);
    assert_eq!(projected["web"]["rateLimiting"]["enabled"], true);
    assert_eq!(projected["web"]["rateLimiting"]["apiPermitLimit"], 0);
    assert_eq!(
        projected["web"]["rateLimiting"]["federationPermitLimit"],
        -2
    );

    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_WEB_API_PERMIT_LIMIT", "0"),
    )
    .expect("frozen non-positive permit value remains startup-valid");
    let policy = crate::controller_rate_limit_policy(
        &config,
        "GET",
        "/api/v0/options",
        None,
        Some("192.0.2.33:1234".parse().unwrap()),
    )
    .unwrap();
    assert_eq!(policy.max_requests, 0);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn native_rate_limit_watch_projects_current_values_and_requests_restart() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let yaml = "web:\n  max_request_body_size: 7340032\n  cors:\n    enabled: true\n    allow_credentials: true\n    allowed_origins: [https://allowed.example]\n    allowed_headers: [X-Custom]\n    allowed_methods: [GET, POST]\n  rate_limiting:\n    enabled: true\n    api_permit_limit: 9\n    api_window_seconds: -1\n    federation_permit_limit: 8\n    federation_window_seconds: 7\n    mesh_gateway_permit_limit: 6\n    mesh_gateway_window_seconds: 5\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(state.runtime.read().await.application_restart_requested);
    assert_eq!(
        state.config.controller_web_rate_limiting.api_permit_limit,
        200
    );
    assert_eq!(
        state.config.controller_web_max_request_body_size,
        10 * 1024 * 1024
    );
    let overlay = state.options_overlay.read().await;
    let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    assert_eq!(
        options["web"]["rateLimiting"],
        serde_json::json!({
            "enabled": true,
            "apiPermitLimit": 9,
            "apiWindowSeconds": -1,
            "federationPermitLimit": 8,
            "federationWindowSeconds": 7,
            "meshGatewayPermitLimit": 6,
            "meshGatewayWindowSeconds": 5,
        })
    );
    assert_eq!(options["web"]["maxRequestBodySize"], 7 * 1024 * 1024);
    assert_eq!(
        options["web"]["cors"],
        serde_json::json!({
            "enabled": true,
            "allowCredentials": true,
            "allowedOrigins": ["https://allowed.example"],
            "allowedHeaders": ["X-Custom"],
            "allowedMethods": ["GET", "POST"],
        })
    );
    assert!(!state.config.controller_web_cors.enabled);

    let policy = crate::controller_rate_limit_policy(
        &state.config,
        "GET",
        "/api/v0/session",
        None,
        Some("192.0.2.32:1234".parse().unwrap()),
    )
    .unwrap();
    assert_eq!(policy.max_requests, 200);
    assert_eq!(policy.window_seconds, 60);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn native_user_group_projects_transfer_group_memberships_and_live_user_classification(
) {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"leechers":{"thresholds":{"files":2,"directories":2}},"blacklisted":{"members":["blocked"]},"user_defined":{"trusted":{"upload":{"priority":10},"members":["friend"]}}}"#,
            ),
    );

    let group = crate::route_http_request("GET", "/api/users/friend/group", None, "", &state)
        .await
        .expect("user group");
    assert_eq!(group.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<String>(&group.body).unwrap(),
        "trusted"
    );

    let unknown =
        crate::route_http_request("GET", "/api/v0/users/stranger/group", None, "", &state)
            .await
            .expect("unknown user group");
    assert_eq!(
        serde_json::from_str::<String>(&unknown.body).unwrap(),
        "default"
    );

    let groups = crate::route_http_request(
        "GET",
        "/api/v0/users/groups?UserNames=%20friend%20&usernames=FRIEND&usernames=stranger&usernames=",
        None,
        "",
        &state,
    )
    .await
    .expect("user group batch");
    assert_eq!(groups.status, "200 OK");
    let groups_json = serde_json::from_str::<serde_json::Value>(&groups.body).unwrap();
    assert_eq!(groups_json.as_object().unwrap().len(), 2);
    assert_eq!(groups_json["friend"], "trusted");
    assert_eq!(groups_json["stranger"], "default");

    // Matches the oracle's cache-only UserService.GetGroup: an unknown
    // username remains in the default group until its user record exists.
    let blocked = crate::route_http_request("GET", "/api/v0/users/blocked/group", None, "", &state)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&blocked.body).unwrap(),
        "default"
    );

    {
        let mut users = state.users.write().await;
        users.apply_stats(
            "leecher".to_owned(),
            &crate::UserStats {
                average_speed: 1,
                upload_count: 0,
                unknown: 0,
                file_count: 1,
                directory_count: 10,
            },
        );
        users.apply_status(&crate::UserStatus {
            username: "supporter".to_owned(),
            status: 2,
            privileged: true,
        });
        // Matches the oracle's real precedence: IsBlacklisted is
        // checked before privileged status, so a blacklisted user
        // stays blacklisted even if the Soulseek server also reports
        // them as privileged.
        users.apply_status(&crate::UserStatus {
            username: "blocked".to_owned(),
            status: 2,
            privileged: true,
        });
    }
    let blocked_but_privileged =
        crate::route_http_request("GET", "/api/v0/users/blocked/group", None, "", &state)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&blocked_but_privileged.body).unwrap(),
        "blacklisted"
    );
    let leecher = crate::route_http_request("GET", "/api/v0/users/leecher/group", None, "", &state)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&leecher.body).unwrap(),
        "leechers"
    );
    let privileged =
        crate::route_http_request("GET", "/api/v0/users/supporter/group", None, "", &state)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<String>(&privileged.body).unwrap(),
        "privileged"
    );

    let too_many = (0..=crate::MAX_USER_GROUP_BATCH)
        .map(|index| format!("usernames=user-{index}"))
        .collect::<Vec<_>>()
        .join("&");
    let rejected = crate::route_http_request(
        "GET",
        &format!("/api/v0/users/groups?{too_many}"),
        None,
        "",
        &state,
    )
    .await
    .expect("bounded user group batch");
    assert_eq!(rejected.status, "400 Bad Request");

    let (controller_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    for path in [
        "/api/v0/users/friend/group",
        "/api/v0/users/groups?usernames=friend",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &controller_state)
            .await
            .unwrap();
        assert_eq!(response.status, "404 Not Found", "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn native_relay_credential_profile_authenticates_agent() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let secret = "test-token-0123456789";
    {
        let mut advanced = state.advanced_networking.write().await;
        advanced.relay.enabled = true;
        advanced.relay.mode = "controller".to_owned();
        advanced.relay.agents.insert(
            "edge".to_owned(),
            crate::config::RelayAgentSettings {
                instance_name: "edge-one".to_owned(),
                secret: secret.to_owned(),
                cidr: "127.0.0.1/32".to_owned(),
            },
        );
    }
    let now = crate::unix_timestamp();
    let challenge = state
        .relay
        .write()
        .await
        .protocol
        .issue_challenge("slskdn-connection", now);
    let credential = crate::relay::credential_for_target(
        crate::config::ControllerProfile::Native,
        secret,
        "edge-one",
        &challenge,
    );
    let settings = state.advanced_networking.read().await.relay.clone();
    assert!(state.relay.write().await.protocol.authenticate_agent(
        &settings,
        crate::relay::credential_scheme(crate::config::ControllerProfile::Native,),
        "slskdn-connection",
        "edge-one",
        &credential,
        "127.0.0.1".parse().unwrap(),
        now,
    ));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_configured_cors_matches_preflight_and_response_contracts() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_WEB_CORS_ENABLED", "true")
            .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
            .with(
                "SLSKD_WEB_CORS_ALLOWED_ORIGINS",
                "https://allowed.example;https://other.example",
            )
            .with("SLSKD_WEB_CORS_ALLOWED_HEADERS", "X-Custom;Content-Type")
            .with("SLSKD_WEB_CORS_ALLOWED_METHODS", "GET;POST"),
    )
    .expect("slskdN CORS config");
    let headers = crate::http_server::HttpHeaders {
        origin: Some("https://allowed.example".to_owned()),
        access_control_request_method: Some("DELETE".to_owned()),
        access_control_request_headers: Some("X-Bad".to_owned()),
        ..Default::default()
    };

    let preflight = crate::controller_cors_headers(&config, &headers, "127.0.0.1:5030", true);
    assert!(preflight.contains("Access-Control-Allow-Origin: https://allowed.example\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Credentials: true\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Headers: X-Custom,Content-Type\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Methods: GET,POST\r\n"));
    assert!(preflight.contains("Access-Control-Max-Age: 3600\r\n"));
    assert!(preflight.contains("Vary: Origin\r\n"));

    let response = crate::controller_cors_headers(&config, &headers, "127.0.0.1:5030", false);
    assert!(response.contains("Access-Control-Expose-Headers: X-URL-Base,X-Total-Count\r\n"));
    assert!(!response.contains("Access-Control-Allow-Methods"));

    let disallowed = crate::http_server::HttpHeaders {
        origin: Some("https://evil.example".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        crate::controller_cors_headers(&config, &disallowed, "127.0.0.1:5030", false),
        ""
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_wildcard_cors_echoes_any_requested_method_and_headers() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_WEB_CORS_ENABLED", "true")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "*"),
    )
    .expect("slskdN wildcard CORS config");
    let headers = crate::http_server::HttpHeaders {
        origin: Some("https://any.example".to_owned()),
        access_control_request_method: Some("PATCH".to_owned()),
        access_control_request_headers: Some("X-One, X-Two".to_owned()),
        ..Default::default()
    };

    let preflight = crate::controller_cors_headers(&config, &headers, "127.0.0.1:5030", true);
    assert!(preflight.contains("Access-Control-Allow-Origin: *\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Methods: PATCH\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Headers: X-One,X-Two\r\n"));
    assert!(!preflight.contains("Vary: Origin"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn native_adversarial_yaml_updates_replace_the_existing_mapping() {
    let initial = "debug: true\nremote_configuration: true\n";
    let defaults = crate::native_adversarial_yaml_update(initial, &serde_json::json!({})).unwrap();
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
    let updated = crate::native_adversarial_yaml_update(&defaults, &custom).unwrap();
    let direct = crate::native_adversarial_yaml_update(initial, &custom).unwrap();

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
pub(super) fn native_adversarial_yaml_update_preserves_security_siblings() {
    let current = "remote_configuration: true\nsecurity:\n  audit_enabled: true\n";
    let updated = crate::native_adversarial_yaml_update(
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
pub(super) fn native_adversarial_validation_rejects_nonpositive_bucket_sizes() {
    assert_eq!(
        crate::validate_native_adversarial_settings(&serde_json::json!({
            "privacy": {"padding": {"bucketSizes": [256, 0]}}
        })),
        Err("Bucket sizes must be positive")
    );
    assert!(
        crate::validate_native_adversarial_settings(&serde_json::json!({
            "privacy": {"padding": {"bucketSizes": [256, 512]}}
        }))
        .is_ok()
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn native_adversarial_put_persists_and_accepts_target_yaml() {
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
        crate::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
            .await
            .expect("adversarial settings response");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    assert!(fs::read_to_string(state.config.state_dir.join("slskd.yml"))
        .unwrap()
        .contains("  adversarial:\n"));
    crate::load_watched_controller_configuration(state.controller_cli_environment.clone())
        .expect("target adversarial YAML remains reloadable");
    let features = state.controller_features.read().await;
    let stored = features
        .get("security/profile/security/adversarial")
        .expect("stored adversarial settings");
    assert_eq!(stored["settings"], serde_json::json!({}));
}
