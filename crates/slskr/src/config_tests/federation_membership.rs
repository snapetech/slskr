use super::*;

#[test]
fn gold_star_club_autojoin_is_profile_aware_and_configurable() {
    let parsed: crate::config::FileConfig =
        toml::from_str("[podcore.gold_star_club]\nautojoin = false\n")
            .expect("Gold Star Club TOML setting");
    assert_eq!(parsed.podcore.gold_star_club.autojoin, Some(false));

    let current = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
    )
    .expect("native/current Gold Star Club default");
    assert!(!current.advanced_networking.gold_star_club_autojoin);

    let current_enabled = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_POD_GOLD_STAR_CLUB_AUTOJOIN", "true"),
    )
    .expect("native/current Gold Star Club opt-in");
    assert!(current_enabled.advanced_networking.gold_star_club_autojoin);

    let frozen = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("frozen Gold Star Club default");
    assert!(frozen.advanced_networking.gold_star_club_autojoin);

    let file_disabled = crate::config::FileConfig {
        podcore: crate::config::PodCoreFileConfig {
            gold_star_club: crate::config::GoldStarClubFileConfig {
                autojoin: Some(false),
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let from_file = crate::config::AppConfig::from_layers(
        None,
        file_disabled,
        &MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("file-disabled Gold Star Club");
    assert!(!from_file.advanced_networking.gold_star_club_autojoin);

    let env_wins = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig {
            podcore: crate::config::PodCoreFileConfig {
                gold_star_club: crate::config::GoldStarClubFileConfig {
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
fn pod_join_signature_modes_are_validated() {
    let default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
    .expect("default pod signature config");
    assert_eq!(
        default.pod_join_signature_mode,
        crate::config::PodSignatureMode::Off
    );

    for (value, expected) in [
        ("warn", crate::config::PodSignatureMode::Warn),
        ("enforce", crate::config::PodSignatureMode::Enforce),
    ] {
        let config = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", value),
        )
        .expect("supported pod signature mode");
        assert_eq!(config.pod_join_signature_mode, expected);
    }

    let error = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", "accept-anything"),
    )
    .expect_err("invalid pod signature mode must fail");
    assert!(error.contains("off, warn, or enforce"), "{error}");
}

#[test]
fn virtual_soulfind_v2_defaults_disabled_and_honors_explicit_enable() {
    let default = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("default slskdN VirtualSoulfind v2 config");
    assert!(!default.virtual_soulfind_v2_enabled);
    assert!(default.acquisition_planning_enabled);

    let file_disabled = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig {
            virtual_soulfind_v2: crate::config::VirtualSoulfindV2FileConfig {
                enabled: Some(false),
            },
            ..crate::config::FileConfig::default()
        },
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"),
    )
    .expect("file-disabled VirtualSoulfind v2 config");
    assert!(!file_disabled.virtual_soulfind_v2_enabled);
    assert!(!file_disabled.acquisition_planning_enabled);

    let env_enabled = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig {
            virtual_soulfind_v2: crate::config::VirtualSoulfindV2FileConfig {
                enabled: Some(false),
            },
            ..crate::config::FileConfig::default()
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
    let defaults = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default(),
    )
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

    let file = crate::config::FileConfig {
        social_federation: crate::config::SocialFederationFileConfig {
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
        federation_publishing: crate::config::FederationPublishingFileConfig {
            enabled: Some(true),
            publishable_domains: vec!["music".to_owned(), "books".to_owned()],
            default_visibility: Some("circle".to_owned()),
            approved_circles: vec!["friends".to_owned()],
            require_moderation_approval: Some(false),
            include_external_links: Some(false),
            max_metadata_size_kb: Some(25),
        },
        ..crate::config::FileConfig::default()
    };
    let configured = crate::config::AppConfig::from_layers(
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
        let error = crate::config::AppConfig::from_layers(
            None,
            crate::config::FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("out-of-range federation setting must fail startup");
        assert!(error.contains("federation"), "{name}={value}: {error}");
    }
}
