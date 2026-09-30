use super::*;

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

    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let defaults = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let defaults = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let current = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let frozen = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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

    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let file = serde_yaml::from_str::<crate::config::FileConfig>(
            "integration:\n  chromaprint:\n    enabled: true\n    algorithm: 1\n    ffmpegPath: /usr/bin/ffmpeg\n    sampleRate: 22050\n    channels: 1\n    durationSeconds: 45\n  acoustid:\n    enabled: true\n    clientId: fixture-client\n    baseUrl: http://127.0.0.1:39001/v2\n",
        )
        .expect("target integration alias parses");
    let config = crate::config::AppConfig::from_layers(
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

    let error = crate::config::AppConfig::from_layers(
        None,
        serde_yaml::from_str::<crate::config::FileConfig>(
            "integration:\n  acoustid:\n    enabled: true\n",
        )
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
fn lidarr_url_rejects_embedded_credentials_before_projection() {
    let env = MapEnv::default()
        .with("SLSKR_LIDARR_URL", "https://operator:secret@example.com")
        .with("SLSKR_LIDARR_API_KEY", "api-key");
    let error =
        crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
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
        let error =
            crate::config::AppConfig::from_layers(None, crate::config::FileConfig::default(), &env)
                .expect_err("non-base Lidarr URL should be rejected");
        assert!(!error.contains("api_key=secret"), "{error}");
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
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
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
    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", root.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect_err("an enabled authenticated bridge requires a password");
    assert!(error.contains("virtualSoulfind.bridge"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}
