use super::*;

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
    let config =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
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
        crate::config::SoulseekObfuscationMode::Compatibility
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
    let config =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKD_HTTP_PORT", "not-a-port"),
    )
    .expect_err("invalid frozen alias must fail");
    assert!(error.contains("SLSKD_HTTP_PORT"), "{error}");
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

    let yaml = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let environment = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let controller_error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let slskdn = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        crate::config::SoulseekObfuscationMode::Compatibility
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
        let config = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().unwrap()),
    )
    .expect("frozen long instance name");
    assert_eq!(config.instance_name, long_name);

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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    file.set_len(crate::config::MAX_CONFIG_FILE_BYTES + 1)
        .unwrap();

    let error =
        crate::config::read_file_config(&path).expect_err("oversized config should be rejected");
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

    let error = crate::config::read_file_config(&path).expect_err("directory should be rejected");
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

    let error = crate::config::read_file_config(&linked).expect_err("symlink should be rejected");
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

    let config = crate::config::read_file_config(&path).expect("small config parsed");
    assert_eq!(config.app.http_bind.as_deref(), Some("127.0.0.1:5555"));

    let _ = std::fs::remove_file(path);
}
