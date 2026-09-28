use super::*;

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

        let config = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let slskd = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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

    let native = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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

    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
        Some(crate::config::TransferLimitSettings::default())
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
        crate::config::TransferQueueStrategy::FirstInFirstOut
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
    let native = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", duplicate),
    )
    .expect_err("slskdN rejects duplicate explicit group membership");
    assert!(native.contains("multiple groups"), "{native}");

    let blacklist_duplicate = r#"{
            "blacklisted": {"members": ["alice"]},
            "user_defined": {"first": {"members": ["ALICE"]}}
        }"#;
    let native_blacklist = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", blacklist_duplicate),
    )
    .expect_err("slskdN rejects blacklisted/user-defined duplicate membership");
    assert!(
        native_blacklist.contains("multiple groups"),
        "{native_blacklist}"
    );

    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
            crate::config::AppConfig::from_layers(
                None,
                crate::config::FileConfig::default(),
                &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", json),
            )
            .is_err(),
            "accepted invalid groups JSON: {json}"
        );
    }
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
fn transfer_auto_retry_defaults_match_the_frozen_client_policy() {
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("out-of-range auto-retry config must fail");
        assert!(error.contains("must be between"), "{name}={value}: {error}");
    }
}

#[test]
fn transfer_auto_retry_preserves_fractional_tolerance_and_frozen_boundary_rounding() {
    for (value, expected) in [("5.5", 5.5), ("-0.5", -0.5), ("100.5", 100.5)] {
        let config = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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
        assert!(crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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
        let config = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
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

    let slskdn = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_BLACKLIST", "true")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    )
    .expect("slskdN uses the final P2P colon");
    assert!(slskdn
        .managed_blacklist
        .contains("127.0.0.1".parse().unwrap()));

    let slskd = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKD_BLACKLIST", "true")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    );
    assert!(slskd.is_err(), "slskd uses the first P2P colon");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn transfer_rescue_defaults_match_the_frozen_client_policy() {
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("out-of-range rescue config must fail");
        assert!(error.contains("must be between"), "{name}={value}: {error}");
    }
}
