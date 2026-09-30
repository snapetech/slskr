//! Controller full configuration contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn default_controller_options_omit_unconfigured_optional_values() {
    for target in ["slskd", "slskdn"] {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-default-options-{target}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&state_dir).unwrap();
        let config = crate::config::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", state_dir.to_str().unwrap())
                .with("SLSKR_CONTROLLER_PROFILE", target),
        )
        .unwrap();
        let overlay = crate::ControllerOptionsOverlayState::load(&config).unwrap();
        let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
            &config, &overlay, true,
        ))
        .unwrap();

        assert!(options["retention"].get("search").is_none());
        assert_eq!(options["retention"]["files"], serde_json::json!({}));
        assert_eq!(
            options["retention"]["transfers"]["upload"],
            serde_json::json!({})
        );
        assert_eq!(
            options["retention"]["transfers"]["download"],
            serde_json::json!({})
        );
        assert!(options["shares"]["cache"].get("retention").is_none());
        if target == "slskd" {
            assert!(options["web"].get("socket").is_none());
        } else {
            assert_eq!(options["web"]["socket"], "");
            assert!(options["telemetry"]["tracing"]
                .get("jaegerEndpoint")
                .is_none());
            assert!(options["telemetry"]["tracing"].get("jaegerPort").is_none());
            assert!(options["telemetry"]["tracing"]
                .get("otlpEndpoint")
                .is_none());
        }
        fs::remove_dir_all(state_dir).unwrap();
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn serve_arguments_reject_unknown_or_duplicate_options() {
    assert_eq!(
        crate::parse_serve_args(&[OsString::from("serve")]),
        Ok(Some(crate::ServeInvocation::default()))
    );
    assert_eq!(
        crate::parse_serve_args(&[OsString::from("serve"), OsString::from("--once")]),
        Ok(Some(crate::ServeInvocation {
            once: true,
            ..Default::default()
        }))
    );
    assert!(
        crate::parse_serve_args(&[OsString::from("serve"), OsString::from("--ocne"),]).is_err()
    );
    assert!(crate::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--once"),
        OsString::from("--once"),
    ])
    .is_err());
    assert_eq!(
        crate::parse_serve_args(&[OsString::from("probe")]),
        Ok(None)
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn regex_filter_arguments_preserve_frozen_multi_value_semantics() {
    let invocation = crate::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--case-sensitive-regex"),
        OsString::from("--share-filter=first"),
        OsString::from("--share-filter"),
        OsString::from("second"),
        OsString::from("--search-request-filter=short"),
        OsString::from("--search-request-filter"),
        OsString::from("private"),
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_CASE_SENSITIVE_REGEX"),
        Some(&"true".to_owned())
    );
    assert_eq!(
        invocation.config_environment.get("SLSKD_SHARE_FILTER"),
        Some(&"first;second".to_owned())
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SEARCH_REQUEST_FILTER"),
        Some(&"short;private".to_owned())
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn frozen_controller_command_line_options_map_to_exact_startup_names() {
    let invocation = crate::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--debug"),
        OsString::from("--headless"),
        OsString::from("--remote-configuration"),
        OsString::from("--remote-file-management"),
        OsString::from("--no-connect"),
        OsString::from("-n"),
        OsString::from("-x"),
        OsString::from("--no-version-check"),
        OsString::from("--experimental"),
        OsString::from("--no-share-scan"),
        OsString::from("--force-share-scan"),
        OsString::from("--enable-blacklist"),
        OsString::from("--blacklist-file=/etc/slskd/blacklist.txt"),
        OsString::from("--swagger"),
        OsString::from("--metrics"),
        OsString::from("--metrics-url=prometheus"),
        OsString::from("--metrics-no-auth"),
        OsString::from("--metrics-username"),
        OsString::from("metrics-user"),
        OsString::from("--metrics-password=metrics-pass"),
        OsString::from("--http-ip-address=127.0.0.2"),
        OsString::from("--http-address=127.0.0.3"),
        OsString::from("--http-port"),
        OsString::from("55030"),
        OsString::from("--instance-name"),
        OsString::from("frozen-instance"),
        OsString::from("--downloads"),
        OsString::from("/srv/slskd/downloads"),
        OsString::from("--incomplete=/srv/slskd/incomplete"),
        OsString::from("--shared=/srv/slskd/music"),
        OsString::from("--slsk-address"),
        OsString::from("soulseek.example"),
        OsString::from("--slsk-port=2271"),
        OsString::from("--slsk-listen-port"),
        OsString::from("55031"),
        OsString::from("--slsk-obfuscation-mode=prefer"),
        OsString::from("--slsk-obfuscation-advertise-regular-port"),
        OsString::from("--download-completed-path-template"),
        OsString::from("{uploader}/{remote_folder}"),
    ])
    .expect("valid frozen controller flags")
    .expect("serve invocation");

    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_DEBUG")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HEADLESS")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_REMOTE_CONFIGURATION")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_REMOTE_FILE_MANAGEMENT")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_CONNECT")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_START")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_LOGO")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_NO_VERSION_CHECK")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_EXPERIMENTAL")
            .map(String::as_str),
        Some("true")
    );
    for name in ["SLSKD_NO_SHARE_SCAN", "SLSKD_FORCE_SHARE_SCAN"] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some("true")
        );
    }
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_BLACKLIST")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_BLACKLIST_FILE")
            .map(String::as_str),
        Some("/etc/slskd/blacklist.txt")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SWAGGER")
            .map(String::as_str),
        Some("true")
    );
    for (name, expected) in [
        ("SLSKD_METRICS", "true"),
        ("SLSKD_METRICS_URL", "prometheus"),
        ("SLSKD_METRICS_NO_AUTH", "true"),
        ("SLSKD_METRICS_USERNAME", "metrics-user"),
        ("SLSKD_METRICS_PASSWORD", "metrics-pass"),
    ] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some(expected)
        );
    }
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HTTP_IP_ADDRESS")
            .map(String::as_str),
        Some("127.0.0.2")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HTTP_PORT")
            .map(String::as_str),
        Some("55030")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_HTTP_ADDRESS")
            .map(String::as_str),
        Some("127.0.0.3")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_INSTANCE_NAME")
            .map(String::as_str),
        Some("frozen-instance")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_DOWNLOADS_DIR")
            .map(String::as_str),
        Some("/srv/slskd/downloads")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_INCOMPLETE_DIR")
            .map(String::as_str),
        Some("/srv/slskd/incomplete")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SHARED_DIR")
            .map(String::as_str),
        Some("/srv/slskd/music")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SLSK_OBFUSCATION_MODE")
            .map(String::as_str),
        Some("prefer")
    );
    assert_eq!(
        invocation
            .config_environment
            .get("SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT")
            .map(String::as_str),
        Some("true")
    );

    let direct = crate::parse_serve_args(&[
        OsString::from("--slsk-address"),
        OsString::from("direct.example"),
    ])
    .expect("direct upstream invocation")
    .expect("direct invocation");
    assert_eq!(
        direct
            .config_environment
            .get("SLSKD_SLSK_ADDRESS")
            .map(String::as_str),
        Some("direct.example")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn frozen_ftp_command_line_options_map_to_exact_startup_names() {
    let invocation = crate::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--ftp"),
        OsString::from("--ftp-address=ftp.example"),
        OsString::from("--ftp-port=2121"),
        OsString::from("--ftp-encryption-mode=Explicit"),
        OsString::from("--ftp-ignore-certificate-errors"),
        OsString::from("--ftp-username=fixture-user"),
        OsString::from("--ftp-password=fixture-password"),
        OsString::from("--ftp-remote-path=/incoming"),
        OsString::from("--ftp-overwrite-existing"),
        OsString::from("--ftp-connection-timeout=4321"),
        OsString::from("--ftp-retry-attempts=5"),
    ])
    .unwrap()
    .unwrap();
    for (name, expected) in [
        ("SLSKD_FTP", "true"),
        ("SLSKD_FTP_ADDRESS", "ftp.example"),
        ("SLSKD_FTP_PORT", "2121"),
        ("SLSKD_FTP_ENCRYPTION_MODE", "Explicit"),
        ("SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS", "true"),
        ("SLSKD_FTP_USERNAME", "fixture-user"),
        ("SLSKD_FTP_PASSWORD", "fixture-password"),
        ("SLSKD_FTP_REMOTE_PATH", "/incoming"),
        ("SLSKD_FTP_OVERWRITE_EXISTING", "true"),
        ("SLSKD_FTP_CONNECTION_TIMEOUT", "4321"),
        ("SLSKD_FTP_RETRY_ATTEMPTS", "5"),
    ] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some(expected),
            "{name}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn frozen_relay_command_line_options_map_to_exact_startup_names() {
    let invocation = crate::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--relay"),
        OsString::from("--relay-mode=agent"),
        OsString::from("--controller-address=https://controller.example"),
        OsString::from("--controller-ignore-certificate-errors"),
        OsString::from("--controller-pinned-spki=pin-a,pin-b"),
        OsString::from("--controller-api-key=0123456789abcdef"),
        OsString::from("--controller-secret=abcdef0123456789"),
        OsString::from("--controller-downloads"),
    ])
    .unwrap()
    .unwrap();
    for (name, expected) in [
        ("SLSKD_RELAY", "true"),
        ("SLSKD_RELAY_MODE", "agent"),
        ("SLSKD_CONTROLLER_ADDRESS", "https://controller.example"),
        ("SLSKD_CONTROLLER_IGNORE_CERTIFICATE_ERRORS", "true"),
        ("SLSKD_CONTROLLER_PINNED_SPKI", "pin-a,pin-b"),
        ("SLSKD_CONTROLLER_API_KEY", "0123456789abcdef"),
        ("SLSKD_CONTROLLER_SECRET", "abcdef0123456789"),
        ("SLSKD_CONTROLLER_DOWNLOADS", "true"),
    ] {
        assert_eq!(
            invocation.config_environment.get(name).map(String::as_str),
            Some(expected),
            "{name}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn startup_flag_command_line_projection_overrides_controller_yaml() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-startup-flags-cli-test-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "flags:\n  no_logo: false\n  no_start: false\n  no_version_check: false\n  experimental: false\nweb:\n  authentication:\n    passthrough:\n      allowed_cidrs: 127.0.0.1/32\n",
    )
    .unwrap();
    let invocation = crate::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--app-dir"),
        state_dir.clone().into_os_string(),
        OsString::from("--no-logo"),
        OsString::from("--no-start"),
        OsString::from("--no-version-check"),
        OsString::from("--experimental"),
        OsString::from("--no-auth"),
        OsString::from("--enforce-security"),
        OsString::from("--allow-remote-no-auth"),
    ])
    .unwrap()
    .unwrap();
    let config = crate::config::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &crate::ControllerCliEnv {
            values: &invocation.config_environment,
        },
    )
    .unwrap();
    assert!(config.controller_no_logo);
    assert!(config.controller_no_start);
    assert!(config.controller_no_version_check);
    assert!(config.controller_experimental);
    assert!(!config.auth_required);
    assert!(config.controller_web_enforce_security);
    assert!(config.controller_web_allow_remote_no_auth);
    let mut overlay = crate::ControllerOptionsOverlayState::load(&config).unwrap();
    overlay.command_line_environment = invocation.config_environment;
    let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(options["flags"]["noLogo"], true);
    assert_eq!(options["flags"]["noStart"], true);
    assert_eq!(options["flags"]["noVersionCheck"], true);
    assert_eq!(options["flags"]["experimental"], true);
    assert_eq!(options["web"]["authentication"]["disabled"], true);
    assert_eq!(options["web"]["enforceSecurity"], true);
    assert_eq!(options["web"]["allowRemoteNoAuth"], true);
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn frozen_controller_rate_limit_policies_match_path_and_auth_precedence() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_WEB_API_PERMIT_LIMIT", "17")
            .with("SLSKD_WEB_API_WINDOW_SECONDS", "0")
            .with("SLSKD_WEB_FEDERATION_PERMIT_LIMIT", "3")
            .with("SLSKD_WEB_MESH_GATEWAY_PERMIT_LIMIT", "4"),
    )
    .expect("slskdN rate-limit config");
    let remote = Some("192.0.2.30:1234".parse().unwrap());

    let anonymous =
        crate::controller_rate_limit_policy(&config, "GET", "/API/v0/searches", None, remote)
            .unwrap();
    assert_eq!(anonymous.partition, "api");
    assert_eq!(anonymous.max_requests, 17);
    assert_eq!(anonymous.window_seconds, 60);
    assert!(crate::controller_rate_limit_policy(
        &config,
        "GET",
        "/api/v0/searches",
        Some("principal"),
        remote,
    )
    .is_none());
    assert!(crate::controller_rate_limit_policy(&config, "GET", "/", None, remote,).is_none());

    let mesh = crate::controller_rate_limit_policy(
        &config,
        "GET",
        "/MESH/gateway",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!((mesh.partition.as_str(), mesh.max_requests), ("mesh", 4));
    let inbox = crate::controller_rate_limit_policy(
        &config,
        "POST",
        "/actors/alice/INBOX",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!((inbox.partition.as_str(), inbox.max_requests), ("fed", 3));
    let event = crate::controller_rate_limit_policy(
        &config,
        "POST",
        "/api/v0/events/inject",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!(event.max_requests, 10);
    let warm = crate::controller_rate_limit_policy(
        &config,
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        Some("principal"),
        remote,
    )
    .unwrap();
    assert_eq!(warm.partition, "warm-cache:principal");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn frozen_webhook_configuration_projects_dynamic_names_and_redacts_headers() {
    let state_dir =
        std::env::temp_dir().join(format!("slskr-webhook-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(state_dir.join("slskd.yml"), "flags:\n  no_connect: true\nintegrations:\n  webhooks:\n    my_webhook:\n      on: [Any, PrivateMessageReceived]\n      call:\n        url: https://example.com/hook\n        headers:\n          - name: Authorization\n            value: secret-value\n        ignore_certificate_errors: false\n      timeout: 1234\n      retry:\n        attempts: 2\n").unwrap();
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_APP_DIR", state_dir.to_str().unwrap()),
    )
    .unwrap();
    let hook = &config.integrations.frozen_webhooks["my_webhook"];
    assert_eq!(hook.on, ["Any", "PrivateMessageReceived"]);
    assert_eq!(hook.call.headers[0].value, "secret-value");
    assert_eq!(hook.timeout, 1234);
    assert_eq!(hook.retry.attempts, 2);
    let overlay = crate::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["integration"]["webhooks"]["mywebhook"]["call"]["headers"][0]["value"],
        "*****"
    );
    assert!(!options.to_string().contains("secret-value"));
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn frozen_script_configuration_validates_and_projects_all_execution_modes() {
    let state_dir =
        std::env::temp_dir().join(format!("slskr-script-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "integrations:\n  scripts:\n    command_mode:\n      on: [DownloadFileComplete]\n      run:\n        command: echo fixture\n    args_mode:\n      on: [Any]\n      run:\n        executable: /bin/sh\n        args: '-c \"echo fixture\"'\n    arglist_mode:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        args_list: [-c, 'echo fixture']\n",
    )
    .unwrap();
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", state_dir.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    assert_eq!(config.integrations.scripts.len(), 3);
    assert_eq!(
        config.integrations.scripts["arglist_mode"].run.arglist,
        Some(vec!["-c".to_owned(), "echo fixture".to_owned()])
    );
    let overlay = crate::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["integration"]["scripts"]["commandmode"]["run"]["command"],
        "echo fixture"
    );
    assert_eq!(
        options["integration"]["scripts"]["argsmode"]["run"]["executable"],
        "/bin/sh"
    );
    assert_eq!(
        options["integration"]["scripts"]["arglistmode"]["run"]["arglist"],
        serde_json::json!(["-c", "echo fixture"])
    );

    for (yaml, expected) in [
        (
            "integrations:\n  scripts:\n    bad:\n      on: [NotAnEvent]\n      run:\n        command: echo\n",
            "invalid event",
        ),
        (
            "integrations:\n  scripts:\n    bad:\n      on: [Any]\n      run: {}\n",
            "One and only one",
        ),
        (
            "integrations:\n  scripts:\n    bad:\n      on: [Any]\n      run:\n        command: echo\n        executable: /bin/sh\n",
            "One and only one",
        ),
        (
            "integrations:\n  scripts:\n    bad:\n      on: [Any]\n      run:\n        executable: /bin/sh\n        args: -c echo\n        arglist: [-c, echo]\n",
            "Only one",
        ),
    ] {
        fs::write(state_dir.join("slskd.yml"), yaml).unwrap();
        let error = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default().with("SLSKD_APP_DIR", state_dir.to_str().unwrap()),
        )
        .expect_err("invalid script configuration must fail");
        assert!(error.contains(expected), "{error}");
    }
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn config_share_add_updates_runtime_index_and_rejects_duplicates() {
    let root = std::env::temp_dir().join(format!(
        "slskr-config-share-test-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("share root");
    std::fs::write(root.join("track.flac"), b"track").expect("share file");
    let (state, _receiver) = test_state();
    let existing_files = state.shares.read().await.entries.len();
    let response = crate::route_http_request(
        "POST",
        "/api/config/shares",
        None,
        &serde_json::json!({
            "path": root,
            "alias": "added",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("add runtime share");
    assert_eq!(response.status, "201 Created");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(body["added"], true);
    assert_eq!(body["alias"], "added");
    assert_eq!(body["files"], existing_files + 1);
    assert_eq!(body["configurationPersisted"], false);
    assert!(state
        .share_settings
        .read()
        .await
        .directories
        .iter()
        .any(|directory| directory.alias == "added"));
    assert!(state
        .shares
        .read()
        .await
        .entries
        .iter()
        .any(|entry| entry.filename == "added/track.flac"));

    let duplicate = crate::route_http_request(
        "POST",
        "/api/config/shares",
        None,
        &serde_json::json!({"path": root}).to_string(),
        &state,
    )
    .await
    .expect("duplicate runtime share");
    assert_eq!(duplicate.status, "409 Conflict");
    std::fs::remove_dir_all(root).expect("remove share root");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn yaml_validation_matches_target_specific_error_contracts() {
    let controller_error = "No node deserializer was able to deserialize the node into type slskd.Options+WebOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null";
    for (target, expected_error) in [
        ("slskd", controller_error),
        ("slskdn", "Invalid YAML configuration"),
    ] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        let valid = crate::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("debug: false\n").unwrap(),
            &state,
        )
        .await
        .expect("valid YAML response");
        assert_eq!(valid.status, "200 OK", "{target}");
        assert!(valid.content_type.is_empty(), "{target}");
        assert!(valid.body.is_empty(), "{target}");

        let invalid = crate::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("web: [unterminated").unwrap(),
            &state,
        )
        .await
        .expect("invalid YAML response");
        assert_eq!(invalid.status, "200 OK", "{target}");
        assert_eq!(invalid.content_type, "application/json; charset=utf-8");
        assert_eq!(
            serde_json::from_str::<String>(&invalid.body).unwrap(),
            expected_error,
            "{target}"
        );

        let invalid_port = crate::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("soulseek:\n  port: 1023\n").unwrap(),
            &state,
        )
        .await
        .expect("invalid server port response");
        assert_eq!(invalid_port.status, "200 OK", "{target}");
        assert_eq!(
            invalid_port.content_type, "application/json; charset=utf-8",
            "{target}"
        );
        assert_eq!(
            serde_json::from_str::<String>(&invalid_port.body).unwrap(),
            if target == "slskd" {
                "Invalid configuration:\n  Soulseek:\n    The field Port must be between 1024 and 65535."
            } else {
                "Invalid YAML configuration"
            },
            "{target}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn options_overlay_sets_target_specific_reconnect_and_redacts_secrets() {
    for (target, reconnect) in [("slskd", false), ("slskdn", true)] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        state.session.write().await.state = "connected";
        let response = crate::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50301,"privateMessageAutoResponse":{"enabled":true}},"integration":{"spotify":{"clientSecret":"do-not-return"}}}"#,
            &state,
        )
        .await
        .expect("target overlay response");
        assert_eq!(response.status, "200 OK", "{target}");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(value["soulseek"]["listenPort"], 50301, "{target}");
        assert_eq!(
            state.runtime.read().await.application_reconnect_pending,
            reconnect,
            "{target}"
        );
        if target == "slskdn" {
            assert_eq!(
                value["soulseek"]["privateMessageAutoResponse"]["enabled"],
                true
            );
            assert_eq!(value["integration"]["spotify"]["clientSecret"], "*****");
            assert!(!response.body.contains("do-not-return"));
        } else {
            assert!(value["soulseek"]
                .get("privateMessageAutoResponse")
                .is_none());
            assert!(value.get("integration").is_none());
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn syntactically_invalid_watched_reload_clears_current_options_projection() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let valid = "shares:\n  directories:\n    - '[Valid]/tmp/valid-share'\n";
    fs::write(state.config.state_dir.join("slskd.yml"), valid).unwrap();
    crate::apply_watched_controller_configuration(
        &state,
        Some(valid),
        &state.controller_cli_environment,
    )
    .await;

    let invalid = "soulseek: [\n";
    fs::write(state.config.state_dir.join("slskd.yml"), invalid).unwrap();
    crate::apply_watched_controller_configuration(
        &state,
        Some(invalid),
        &state.controller_cli_environment,
    )
    .await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    assert_eq!(current["shares"]["directories"], serde_json::json!([]));
    assert!(state
        .controller_options_validation_error
        .read()
        .unwrap()
        .is_none());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn config_reload_joins_newly_configured_rooms_while_connected() {
    // Matches the oracle's real OptionsMonitor_OnChange, which calls
    // RoomService.TryJoinAsync(newOptions.Rooms) on every config
    // reload -- previously config-reload only updated local
    // bookkeeping (RoomStore::merge_configured), so a room added
    // while already connected was marked "joined" in the API
    // immediately but the server was never actually told to join it.
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let yaml = "rooms: [music]\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    assert_eq!(
        receiver
            .try_recv()
            .expect("real join dispatched for the newly configured room"),
        crate::SessionCommand::JoinRoom("music".to_owned())
    );
    assert!(state
        .rooms
        .read()
        .await
        .records
        .iter()
        .any(|record| record.name == "music" && record.joined));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn options_debug_requires_both_debug_and_remote_configuration() {
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
        let response = crate::route_http_request("GET", "/api/v0/options/debug", None, "", &state)
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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn options_config_location_is_the_confined_compatibility_file() {
    let (state, _receiver) = test_state();
    let mut config = state.config.clone();
    config.config_file = Some("/private/config/slskr-secret.toml".into());

    let location =
        serde_json::from_str::<String>(&crate::controller_options_config_location_json(&config))
            .unwrap();
    assert_eq!(PathBuf::from(location), config.state_dir.join("slskd.yml"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn toml_config_sanitizes_secrets_and_storage_paths() {
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

    let config = crate::AppConfig::from_layers(
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
pub(super) fn non_loopback_bind_uses_controller_login_when_api_key_is_absent() {
    let env = MapEnv::default().with("SLSKR_HTTP_BIND", "0.0.0.0:5030");
    let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env)
        .expect("controller login credentials protect non-loopback binds");

    assert!(config.auth_required);
    assert!(config.api_token.is_none());
    assert_eq!(config.controller_web_auth_username, "slskr");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn environment_overrides_file_config() {
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

    let config = crate::AppConfig::from_layers(None, file_config, &env).unwrap();

    assert_eq!(config.listen_port, 4444);
    assert_eq!(config.share_settings.max_files, 5);
    assert_eq!(config.transfer_max_active, 1);
    assert!(!config.transfer_allow_inbound);
    assert!(!config.transfer_allow_outbound);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn toml_config_rejects_unknown_fields() {
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
pub(super) fn scrubbed_socket_addr_hides_host() {
    let address = "192.0.2.10:2234".parse().unwrap();
    assert_eq!(crate::scrub_socket_addr(address), "ipv4:2234");
}
