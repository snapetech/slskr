//! Controller full security contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn secure_oauth_state_fails_closed_when_randomness_is_unavailable() {
    assert!(crate::secure_oauth_state_with(|_| false).is_none());

    let token = crate::secure_oauth_state_with(|bytes| {
        bytes.fill(0xab);
        true
    })
    .expect("deterministic randomness fixture");
    assert_eq!(token, format!("slskr-{}", "ab".repeat(32)));

    assert!(crate::secure_share_grant_token_with(|_| false).is_none());
    let share_token = crate::secure_share_grant_token_with(|bytes| {
        bytes.fill(0xcd);
        true
    })
    .expect("deterministic randomness fixture");
    assert_eq!(share_token, format!("share-{}", "cd".repeat(32)));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn managed_blacklist_runtime_caches_by_username_and_clears_on_replace() {
    let root = std::env::temp_dir().join(format!(
        "slskr-managed-blacklist-runtime-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("blacklist.txt");
    fs::write(&path, "127.0.0.1/32\n").unwrap();
    let enabled = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_BLACKLIST", "true")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    )
    .expect("enabled managed blacklist");
    let disabled = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_BLACKLIST", "false")
            .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
    )
    .expect("disabled managed blacklist");

    let mut runtime = crate::ManagedBlacklistRuntime::new(
        enabled.managed_blacklist,
        enabled.controller_profile,
        enabled.controller_case_sensitive_regex,
    );
    assert!(runtime.is_blacklisted(Some("Peer"), Some("127.0.0.1".parse().unwrap()), 100));
    assert!(!runtime.is_blacklisted(Some("peer"), None, 101));
    assert!(!runtime.is_blacklisted(Some("other"), Some("127.0.0.2".parse().unwrap()), 101));

    runtime.replace(
        disabled.managed_blacklist,
        disabled.controller_profile,
        disabled.controller_case_sensitive_regex,
    );
    assert!(!runtime.is_blacklisted(Some("peer"), Some("127.0.0.1".parse().unwrap()), 102));
    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn revoked_jwt_store_persists_and_reloads_across_restart() {
    let root = std::env::temp_dir().join(format!(
        "slskr-revoked-jwt-store-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let now = crate::unix_timestamp();

    {
        let mut store = crate::RevokedJwtStore::load(&root).expect("load JWT revocation store");
        assert!(!store.contains("token-1", now), "fresh store must be empty");
        store
            .revoke("token-1".to_owned(), now + 3_600, now)
            .expect("persist JWT revocation");
        assert!(store.contains("token-1", now));
    }

    // Simulate a restart: a fresh store loaded from the same state
    // directory must still honor the revocation.
    let mut reloaded = crate::RevokedJwtStore::load(&root).expect("reload JWT revocation store");
    assert!(
        reloaded.contains("token-1", now),
        "revocation must survive a restart"
    );

    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn revoked_jwt_store_prunes_expired_entries_on_reload() {
    let root = std::env::temp_dir().join(format!(
        "slskr-revoked-jwt-store-prune-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let now = crate::unix_timestamp();

    {
        let mut store = crate::RevokedJwtStore::load(&root).expect("load JWT revocation store");
        store
            .revoke("short-lived-token".to_owned(), now + 1, now)
            .expect("persist JWT revocation");
    }

    // Reload well after expiry: the entry must be pruned, not resurrected.
    let mut reloaded = crate::RevokedJwtStore::load(&root).expect("reload JWT revocation store");
    assert!(!reloaded.contains("short-lived-token", now + 100));

    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn blacklist_pattern_case_mode_applies_to_current_and_frozen_profiles() {
    let slskd = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true")
            .with("SLSKD_BLACKLISTED_PATTERNS", "^caseuser$"),
    )
    .unwrap();
    let mut controller_runtime = crate::ManagedBlacklistRuntime::new(
        slskd.managed_blacklist,
        slskd.controller_profile,
        slskd.controller_case_sensitive_regex,
    );
    assert!(!controller_runtime.is_blacklisted(Some("CaseUser"), None, 1));
    assert!(controller_runtime.is_blacklisted(Some("caseuser"), None, 1));

    let slskdn = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true")
            .with("SLSKD_BLACKLISTED_PATTERNS", "^caseuser$"),
    )
    .unwrap();
    let mut native_runtime = crate::ManagedBlacklistRuntime::new(
        slskdn.managed_blacklist,
        slskdn.controller_profile,
        slskdn.controller_case_sensitive_regex,
    );
    assert!(!native_runtime.is_blacklisted(Some("CaseUser"), None, 1));
    assert!(native_runtime.is_blacklisted(Some("caseuser"), None, 1));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn managed_blacklist_empty_peer_payloads_match_the_frozen_wire_shape() {
    let browse = crate::build_empty_browse_payload().expect("empty browse payload");
    assert_eq!(
        crate::decompress_zlib_payload(&browse).unwrap(),
        vec![0; 12]
    );

    let folder = crate::build_folder_contents_payload(&[], 124, "share", Default::default())
        .expect("empty folder payload");
    let decoded = crate::decompress_zlib_payload(&folder).unwrap();
    let mut reader = slskr_client::protocol::Reader::new(&decoded);
    assert_eq!(reader.read_u32_le().unwrap(), 124);
    assert_eq!(reader.read_string().unwrap(), "share");
    assert_eq!(reader.read_u32_le().unwrap(), 1);
    assert_eq!(reader.read_string().unwrap(), "share");
    assert_eq!(reader.read_u32_le().unwrap(), 0);
    assert!(reader.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn role_denials_are_distinguishable_from_csrf_rejections() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "read-token"),
    );
    state.session.write().await.state = "connected";
    // Authenticated, but with a role too low for this route -- must not
    // be reported as a CSRF rejection.
    let role_denied = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        Some("Bearer read-token"),
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(role_denied.status, "403 Forbidden");
    assert_eq!(
        role_denied.body,
        "{\"error\":\"insufficient permissions for this route\"}"
    );

    // Bearer credentials bypass cookie CSRF validation, matching the
    // frozen ValidateCsrfForCookiesOnly policy.
    let csrf_denied = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/searches",
        Some("Bearer write-token"),
        r#"{"searchText":"csrf-audit"}"#,
        &state,
        crate::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: Some("https://evil.example".to_string()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(csrf_denied.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn no_auth_mode_uses_native_passthrough_peer_identity() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "")
            .with("SLSK_PASSWORD", "")
            .with("SLSKR_AUTH_DISABLED", "true"),
    );

    assert_eq!(
        crate::pod_request_peer_id(&state).await.as_deref(),
        Some("Anonymous")
    );

    let created = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-passthrough","name":"Passthrough","isPublic":true}}"#,
        &state,
    )
    .await
    .expect("create pod with passthrough identity");
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let members = crate::route_http_request(
        "GET",
        "/api/v0/pods/pod-passthrough/members",
        None,
        "",
        &state,
    )
    .await
    .expect("list passthrough pod members");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&members.body).unwrap()[0]["peerId"],
        "Anonymous"
    );

    let content_pod = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"Content Pod","contentId":"content:music:recording:passthrough"}"#,
        &state,
    )
    .await
    .expect("create content pod with passthrough identity");
    assert_eq!(content_pod.status, "201 Created", "{}", content_pod.body);
    assert!(state
        .pods
        .read()
        .await
        .can_moderate("pod:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "Anonymous"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn trusted_proxy_rate_limit_addr_uses_forwarded_headers_only_from_allowlist() {
    let trusted_env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32");
    let trusted_config = crate::AppConfig::from_layers(None, FileConfig::default(), &trusted_env)
        .expect("trusted proxy config");
    let untrusted_config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &MapEnv::default())
            .expect("default config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = crate::http_server::HttpHeaders {
        x_forwarded_for: Some("198.51.100.24, 127.0.0.1".to_owned()),
        ..Default::default()
    };

    let trusted_addr = crate::rate_limit_remote_addr(&trusted_config, proxy, &headers)
        .expect("trusted forwarded address");
    assert_eq!(
        trusted_addr.ip(),
        "198.51.100.24".parse::<IpAddr>().unwrap()
    );

    let untrusted_addr = crate::rate_limit_remote_addr(&untrusted_config, proxy, &headers)
        .expect("raw peer address");
    assert_eq!(untrusted_addr.ip(), "127.0.0.1".parse::<IpAddr>().unwrap());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn authenticated_rate_limit_key_uses_verified_credential_identity() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_API_TOKEN", "route-token")
            .with("SLSKR_API_COOKIE_AUTH_ENABLED", "true"),
    )
    .expect("cookie auth config");
    let cookie = Some("slskr.session=route-token");
    let first = crate::authenticated_rate_limit_user_key(&config, None, cookie, None)
        .expect("cookie-authenticated key");
    let second =
        crate::authenticated_rate_limit_user_key(&config, Some("Bearer route-token"), cookie, None)
            .expect("matching credentials key");

    assert_eq!(first, second);
    assert_eq!(first, crate::rate_limit_user_key("route-token"));
    let key = first;
    assert!(!key.contains("route-token"));
    assert!(crate::authenticated_rate_limit_user_key(
        &config,
        Some("Bearer attacker-controlled"),
        cookie,
        None,
    )
    .is_none());
    assert!(crate::authenticated_rate_limit_user_key(
        &config,
        Some("Basic route-token"),
        cookie,
        None,
    )
    .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn authenticated_api_key_rate_limit_key_honors_cidr_from_request_peer() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKD_API_KEYS_JSON",
                r#"{"operator":{"key":"0123456789abcdef","role":"readonly","cidr":"127.0.0.1/32"}}"#,
            ),
    )
    .expect("API key config");
    let key = crate::authenticated_rate_limit_user_key(
        &config,
        Some("ApiKey 0123456789abcdef"),
        None,
        Some("127.0.0.1:8080".parse().unwrap()),
    )
    .expect("CIDR-authorized API key must bypass the anonymous partition");
    assert_eq!(key, crate::rate_limit_user_key("0123456789abcdef"));
    assert!(crate::authenticated_rate_limit_user_key(
        &config,
        Some("ApiKey 0123456789abcdef"),
        None,
        Some("192.0.2.20:8080".parse().unwrap()),
    )
    .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn cookie_auth_rejects_noncanonical_token_encodings() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_API_TOKEN", "token-�")
            .with("SLSKR_API_COOKIE_AUTH_ENABLED", "true"),
    )
    .expect("cookie auth config");

    assert!(crate::is_authorized(
        &config,
        None,
        Some("slskr.session=token-%EF%BF%BD")
    ));
    for cookie in [
        "slskr.session=token-%FF",
        "slskr.session=token-%",
        "slskr.session=token-%G0",
        "slskr.session=token-%FF; slskr.session=token-%EF%BF%BD",
        "slskr.session=token-%EF%BF%BD; slskr.session=token-%EF%BF%BD",
        "slskr.session=wrong; slskr.session=token-%EF%BF%BD",
    ] {
        assert!(
            !crate::is_authorized(&config, None, Some(cookie)),
            "{cookie}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn trusted_proxy_rate_limit_addr_parses_forwarded_header_ipv6() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "::1/128");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("[::1]:5000".parse::<SocketAddr>().unwrap());
    let headers = crate::http_server::HttpHeaders {
        forwarded: Some(r#"for="[2001:db8::42]:1234";proto=https"#.to_owned()),
        ..Default::default()
    };

    let addr =
        crate::rate_limit_remote_addr(&config, proxy, &headers).expect("trusted forwarded address");
    assert_eq!(addr.ip(), "2001:db8::42".parse::<IpAddr>().unwrap());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn forwarded_ip_parser_rejects_malformed_authorities() {
    for malformed in [
        "\"\"198.51.100.24\"\"",
        "\"198.51.100.24",
        "198.51.100.24\"",
        "[2001:db8::42]garbage",
        "[2001:db8::42]:garbage",
        "[2001:db8::42]:65536",
        "[198.51.100.24]:80",
        "198.51.100.24:65536",
        "198.51.100.24:80:90",
    ] {
        assert_eq!(
            crate::parse_forwarded_ip_token(malformed),
            None,
            "accepted {malformed}"
        );
    }
    assert_eq!(
        crate::parse_forwarded_ip_token("\"[2001:db8::42]:443\""),
        Some("2001:db8::42".parse::<IpAddr>().unwrap())
    );
    assert_eq!(
        crate::parse_forwarded_ip_token("198.51.100.24:443"),
        Some("198.51.100.24".parse::<IpAddr>().unwrap())
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn forwarded_elements_require_one_valid_for_parameter() {
    let expected = "198.51.100.24".parse::<IpAddr>().unwrap();
    assert_eq!(
        crate::parse_forwarded_element_ip("proto=https; for=198.51.100.24; by=10.0.0.2"),
        Some(expected)
    );
    for malformed in [
        "proto=https",
        "for=unknown;for=198.51.100.24",
        "for=198.51.100.24;for=198.51.100.24",
        "for=198.51.100.24;for=203.0.113.9",
        "for=198.51.100.24;broken",
    ] {
        assert_eq!(
            crate::parse_forwarded_element_ip(malformed),
            None,
            "accepted {malformed:?}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn trusted_proxy_rate_limit_addr_rejects_spoofed_leftmost_hop() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32, 10.0.0.0/8");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = crate::http_server::HttpHeaders {
        x_forwarded_for: Some("203.0.113.99, 198.51.100.24, 10.0.0.2".to_owned()),
        ..Default::default()
    };

    let addr =
        crate::rate_limit_remote_addr(&config, proxy, &headers).expect("forwarded client address");
    assert_eq!(
        addr.ip(),
        "198.51.100.24".parse::<IpAddr>().unwrap(),
        "the first untrusted hop from the proxy boundary is the client"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn trusted_proxy_rate_limit_addr_fails_closed_on_malformed_chain() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = crate::http_server::HttpHeaders {
        x_forwarded_for: Some("203.0.113.99, not-an-ip".to_owned()),
        ..Default::default()
    };

    let addr =
        crate::rate_limit_remote_addr(&config, proxy, &headers).expect("socket peer fallback");
    assert_eq!(addr.ip(), "127.0.0.1".parse::<IpAddr>().unwrap());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn trusted_proxy_rate_limit_addr_does_not_fallback_from_invalid_forwarded_header() {
    let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("proxy config");
    let proxy = Some("127.0.0.1:5000".parse::<SocketAddr>().unwrap());
    let headers = crate::http_server::HttpHeaders {
        forwarded: Some("for=unknown".to_owned()),
        x_forwarded_for: Some("203.0.113.99".to_owned()),
        ..Default::default()
    };

    let addr =
        crate::rate_limit_remote_addr(&config, proxy, &headers).expect("socket peer fallback");
    assert_eq!(addr.ip(), "127.0.0.1".parse::<IpAddr>().unwrap());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn configured_mesh_sync_and_violation_thresholds_enforce_quarantine_and_bans() {
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    let mut sync = config.advanced_networking.mesh_sync_security.clone();
    sync.max_invalid_entries_per_window = 2;
    sync.quarantine_violation_threshold = 2;
    sync.rate_limit_window = std::time::Duration::from_secs(60);
    sync.quarantine_duration = std::time::Duration::from_secs(120);
    let mut mesh = crate::MeshState::new();
    assert!(mesh.record_invalid_sync_entries("Peer", 3, &sync, 1_000));
    assert!(!mesh.sync_is_quarantined("peer", 1_000));
    assert!(mesh.record_invalid_sync_entries("PEER", 1, &sync, 1_001));
    assert!(mesh.sync_is_quarantined("peer", 1_001));
    assert_eq!(mesh.sync_rejected_messages, 2);
    assert_eq!(mesh.sync_quarantine_events, 1);
    assert!(!mesh.sync_is_quarantined("peer", 1_122));

    let mut security_settings = config.advanced_networking.security.clone();
    security_settings.enabled = true;
    security_settings.violation_tracker.enabled = true;
    security_settings
        .violation_tracker
        .violations_before_auto_ban = 2;
    security_settings.violation_tracker.base_ban_duration = std::time::Duration::from_secs(900);
    let mut security = crate::SecurityState::new();
    assert!(!security.record_peer_violation("BadPeer", &security_settings));
    assert_eq!(security.reputation["badpeer"], 35);
    assert!(security.record_peer_violation("badpeer", &security_settings));
    assert_eq!(security.reputation["badpeer"], 20);
    assert_eq!(security.active_bans(), 1);
    assert_eq!(security.bans[0].value, "badpeer");
    assert_eq!(security.bans[0].reason, "Automatic security-policy ban");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn security_bans_normalize_and_reject_malformed_persisted_values() {
    let oversized_username = format!("  {}  ", "é".repeat(crate::MAX_USER_USERNAME_BYTES));
    let mut security = crate::SecurityState::new();
    let username = security
        .ban("username", oversized_username)
        .expect("valid username ban");
    assert!(username.value.len() <= crate::MAX_SECURITY_BAN_USERNAME_BYTES);
    assert!(!username.value.starts_with(char::is_whitespace));
    assert!(!username.value.ends_with(char::is_whitespace));
    assert!(security
        .ban(
            "username",
            format!(" {} ", username.value.to_ascii_uppercase())
        )
        .is_some());
    assert_eq!(security.active_bans(), 1);
    assert!(security.ban("ip", "not-an-ip".to_owned()).is_none());
    assert!(security.ban("unknown", "value".to_owned()).is_none());

    let hydrated = crate::SecurityState::from_persisted(vec![
        crate::persistence::SecurityBanRecord {
            kind: "ip".to_owned(),
            value: "2001:0db8:0:0:0:0:0:1".to_owned(),
            created_at: 1,
            reason: "Manual ban".to_owned(),
            expires_at: 3_601,
            is_permanent: false,
        },
        crate::persistence::SecurityBanRecord {
            kind: "ip".to_owned(),
            value: "2001:db8::1".to_owned(),
            created_at: 2,
            reason: "Manual ban".to_owned(),
            expires_at: 3_602,
            is_permanent: false,
        },
        crate::persistence::SecurityBanRecord {
            kind: "ip".to_owned(),
            value: "malformed".to_owned(),
            created_at: 3,
            reason: "Manual ban".to_owned(),
            expires_at: 3_603,
            is_permanent: false,
        },
        crate::persistence::SecurityBanRecord {
            kind: "other".to_owned(),
            value: "ignored".to_owned(),
            created_at: 4,
            reason: "Manual ban".to_owned(),
            expires_at: 3_604,
            is_permanent: false,
        },
    ]);
    assert_eq!(hydrated.active_bans(), 1);
    assert_eq!(hydrated.bans[0].value, "2001:db8::1");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn security_reputation_routes_reflect_real_score_and_violations() {
    // Matches the oracle's real PeerReputation-backed
    // SecurityController: reputation endpoints previously read/wrote
    // a disconnected, admin-only settings blob that never reflected
    // real peer behavior (violations always showed as 0, and
    // suspicious/trusted lists were derived from watch/online status
    // instead of reputation at all).
    let (state, _receiver) = test_state();

    // The profile counters must reconcile the real retained transfer
    // history rather than remain at an unconditional zero.
    {
        let mut transfers = state.transfers.write().await;
        let successful = transfers.create(
            0,
            Some("cleanpeer".to_owned()),
            "Music/clean.flac".to_owned(),
            None,
            Some(100),
        );
        transfers.update_status(successful.id, "succeeded", Some(100), None);
        let failed = transfers.create(
            0,
            Some("cleanpeer".to_owned()),
            "Music/failed.flac".to_owned(),
            None,
            Some(20),
        );
        transfers.update_status(
            failed.id,
            "failed",
            Some(20),
            Some("content hash mismatch".to_owned()),
        );
        let aborted = transfers.create(
            0,
            Some("cleanpeer".to_owned()),
            "Music/aborted.flac".to_owned(),
            None,
            Some(3),
        );
        transfers.update_status(aborted.id, "cancelled", Some(3), None);
    }

    // A peer with no recorded violations reports the real,
    // never-decremented default score.
    let clean = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/cleanpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("clean peer reputation");
    let clean_json = serde_json::from_str::<serde_json::Value>(&clean.body).unwrap();
    assert_eq!(clean_json["score"], 50);
    assert_eq!(clean_json["protocolViolations"], 0);
    assert_eq!(clean_json["successfulTransfers"], 1);
    assert_eq!(clean_json["failedTransfers"], 1);
    assert_eq!(clean_json["abortedTransfers"], 1);
    assert_eq!(clean_json["totalBytesTransferred"], 123);
    assert_eq!(clean_json["contentMismatches"], 1);
    assert_eq!(clean_json["successRate"], 1.0 / 3.0);
    assert_eq!(clean_json["trustLevel"], "Neutral");

    // Seed a real violation-driven score drop via the same
    // SecurityState used by the real violation tracker.
    {
        let mut security = state.security.write().await;
        security.reputation.insert("badpeer".to_owned(), 15);
        security.violations.insert("badpeer".to_owned(), 3);
        security.reputation.insert("neutral-low".to_owned(), 30);
    }
    let bad = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/badpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("bad peer reputation");
    let bad_json = serde_json::from_str::<serde_json::Value>(&bad.body).unwrap();
    assert_eq!(bad_json["score"], 15);
    assert_eq!(bad_json["protocolViolations"], 3);
    assert_eq!(bad_json["trustLevel"], "Untrusted");

    // The suspicious list reflects the real low score, not
    // watch/online status.
    let suspicious = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/suspicious",
        None,
        "",
        &state,
    )
    .await
    .expect("suspicious peers");
    let suspicious_json = serde_json::from_str::<serde_json::Value>(&suspicious.body).unwrap();
    let suspicious_usernames = suspicious_json
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["username"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(suspicious_usernames, vec!["badpeer", "neutral-low"]);
    assert_eq!(suspicious_json[0]["firstSeen"].as_str().unwrap().len(), 20);
    assert_eq!(suspicious_json[0]["lastSeen"].as_str().unwrap().len(), 20);
    assert_eq!(suspicious_json[0]["successfulTransfers"], 0);
    assert_eq!(suspicious_json[0]["failedTransfers"], 0);
    assert_eq!(suspicious_json[0]["successRate"], 0.5);
    assert_eq!(suspicious_json[0]["trustLevel"], "Untrusted");

    let dashboard =
        crate::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
            .await
            .expect("security dashboard");
    let dashboard_json = serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap();
    assert_eq!(dashboard_json["reputationStats"]["totalPeers"], 3);
    assert_eq!(dashboard_json["reputationStats"]["untrustedPeers"], 1);
    let average_score = dashboard_json["reputationStats"]["averageScore"]
        .as_f64()
        .unwrap();
    assert!((average_score - (95.0 / 3.0)).abs() < 1e-12);
    assert_eq!(
        dashboard_json["reputationStats"]["totalProtocolViolations"],
        3
    );

    // The trusted list never includes the suspicious peer.
    let trusted = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/trusted",
        None,
        "",
        &state,
    )
    .await
    .expect("trusted peers");
    let trusted_json = serde_json::from_str::<serde_json::Value>(&trusted.body).unwrap();
    assert!(trusted_json
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["username"] != "badpeer"));

    // A manual PUT override rejects out-of-range scores...
    let invalid = crate::route_http_request(
        "PUT",
        "/api/v0/security/reputation/badpeer",
        None,
        r#"{"score":150}"#,
        &state,
    )
    .await
    .expect("reject out-of-range score");
    assert_eq!(invalid.status, "400 Bad Request");

    // ...and a valid override writes directly into the same real
    // score the automatic violation tracker reads and adjusts.
    let overridden = crate::route_http_request(
        "PUT",
        "/api/v0/security/reputation/badpeer",
        None,
        r#"{"score":80,"reason":"manual review"}"#,
        &state,
    )
    .await
    .expect("override reputation score");
    assert_eq!(overridden.status, "200 OK");
    assert_eq!(state.security.read().await.reputation["badpeer"], 80);
    let after_override = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/badpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("reputation after override");
    let after_override_json =
        serde_json::from_str::<serde_json::Value>(&after_override.body).unwrap();
    assert_eq!(after_override_json["score"], 80);
    assert_eq!(after_override_json["trustLevel"], "Trusted");

    let first_seen = bad_json["firstSeen"].as_str().unwrap().to_owned();
    let reread = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/badpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("re-read peer reputation");
    let reread_json = serde_json::from_str::<serde_json::Value>(&reread.body).unwrap();
    assert_eq!(reread_json["firstSeen"], first_seen);
    assert_eq!(reread_json["protocolViolations"], 3);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn security_ban_routes_reject_invalid_ips_and_canonicalize_values() {
    let (state, _receiver) = test_state();
    let invalid = crate::route_http_request(
        "POST",
        "/api/security/bans/ip",
        None,
        r#"{"ip":"attacker-controlled"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid.status, "400 Bad Request");
    assert!(state.security.read().await.bans.is_empty());

    let ip = crate::route_http_request(
        "POST",
        "/api/security/bans/ip",
        None,
        r#"{"ip":" 2001:0db8:0:0:0:0:0:1 "}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(ip.status, "200 OK");
    let ip_json = serde_json::from_str::<serde_json::Value>(&ip.body).unwrap();
    assert_eq!(ip_json["ip"], "2001:db8::1");
    assert_eq!(ip_json["persisted"], false);

    let username = crate::route_http_request(
        "POST",
        "/api/security/bans/username",
        None,
        r#"{"username":"  Peer One  "}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(username.status, "200 OK");
    let username_json = serde_json::from_str::<serde_json::Value>(&username.body).unwrap();
    assert_eq!(username_json["username"], "Peer One");
    assert_eq!(username_json["persisted"], false);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn security_ban_rolls_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;

    let response = crate::route_http_request(
        "POST",
        "/api/security/bans/username",
        None,
        r#"{"username":"must-persist"}"#,
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("security ban persistence failed"));
    assert!(state.security.read().await.bans.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn security_ban_routes_require_exact_documented_segments() {
    let (state, _receiver) = test_state();
    let security_body = state.security.read().await.json_value().to_string();
    for path in [
        "/not-api/bans",
        "/api/x/y/bans",
        "/api/security/bans/username",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap();
        assert_ne!(response.body, security_body, "unexpected GET match: {path}");
    }
    for path in [
        "/api/security/bans/ip/extra",
        "/api/x/y/bans/ip",
        "/not-api/bans/ip",
    ] {
        crate::route_http_request("POST", path, None, r#"{"ip":"192.0.2.1"}"#, &state)
            .await
            .unwrap();
    }
    assert!(state.security.read().await.bans.is_empty());

    let root = crate::route_http_request("GET", "/api/bans", None, "", &state)
        .await
        .unwrap();
    assert_eq!(root.body, security_body);
    let namespaced = crate::route_http_request("GET", "/api/security/bans", None, "", &state)
        .await
        .unwrap();
    assert_eq!(namespaced.body, security_body);
    let versioned = crate::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
        .await
        .unwrap();
    assert_eq!(versioned.body, "[]");

    assert_eq!(crate::security_ban_route_tail("/api/bans"), Some(vec![]));
    assert_eq!(
        crate::security_ban_route_tail("/api/security/bans/ip/192.0.2.1"),
        Some(vec!["ip", "192.0.2.1"])
    );
    assert_eq!(crate::security_ban_route_tail("/not-api/bans"), None);
    assert_eq!(crate::security_ban_route_tail("/api/x/y/bans"), None);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn security_unban_removes_legacy_noncanonical_persistence_keys() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    db.upsert_security_ban(&crate::persistence::SecurityBanRecord {
        kind: "username".to_owned(),
        value: "  Peer One  ".to_owned(),
        created_at: 1,
        reason: "Manual ban".to_owned(),
        expires_at: 3_601,
        is_permanent: false,
    })
    .await
    .unwrap();
    db.upsert_security_ban(&crate::persistence::SecurityBanRecord {
        kind: "ip".to_owned(),
        value: "2001:0db8:0:0:0:0:0:1".to_owned(),
        created_at: 1,
        reason: "Manual ban".to_owned(),
        expires_at: 3_601,
        is_permanent: false,
    })
    .await
    .unwrap();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    assert!(
        crate::persist_security_unban(&state, "username", "peer one")
            .await
            .unwrap()
    );
    assert!(crate::persist_security_unban(&state, "ip", "2001:db8::1")
        .await
        .unwrap());
    assert!(db.list_security_bans().await.unwrap().is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn security_unban_rolls_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .security
        .write()
        .await
        .ban("username", "must-remain".to_owned())
        .expect("ban");
    db.close_for_test().await;

    let response = crate::route_http_request(
        "DELETE",
        "/api/security/bans/username/must-remain",
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("security unban persistence failed"));
    assert_eq!(state.security.read().await.active_bans(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn trusted_mesh_preview_fetches_frozen_overlay_content_ranges() {
    use sha2::{Digest, Sha256};
    use std::io::Read as _;

    let root = std::env::temp_dir().join(format!(
        "slskr-trusted-mesh-preview-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("trusted mesh preview root");
    let content = b"trusted frozen overlay mesh preview";
    let content_path = root.join("trusted.flac");
    std::fs::write(&content_path, content).expect("write trusted mesh content");

    let (remote_state, _remote_receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "member=127.0.0.1:1"),
    );
    add_test_share(
        &remote_state,
        "Virtual/Trusted.flac",
        &content_path,
        content.len() as u64,
    )
    .await;
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("trusted mesh gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    let trusted_peers = serde_json::json!([{
        "peerId": "remote-peer",
        "username": "tester",
        "overlayEndpoint": endpoint.to_string(),
        "certificateSha256": hex::encode(certificate_pin)
    }]);
    let (local_state, _local_receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "member")
            .with("SLSK_PASSWORD", "secret")
            .with("SLSKR_TRUSTED_MESH_PEERS", &trusted_peers.to_string()),
    );
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "member",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        Vec::new(),
        std::time::Duration::from_secs(300),
        &local_state.capability_signing_key,
        std::time::SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.sign(&local_state.capability_signing_key))
    .expect("local trusted mesh capability");
    remote_state
        .mesh
        .write()
        .await
        .update_capability(descriptor)
        .expect("register trusted mesh caller capability");
    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&remote_state)));

    let expected_hash = hex::encode(Sha256::digest(content));
    let ticket_response = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        &format!(
            r#"{{"contentId":"Virtual/Trusted.flac","filename":"Remote/Trusted.flac","peerId":"remote-peer","size":{},"expectedHash":"{expected_hash}"}}"#,
            content.len()
        ),
        &local_state,
    )
    .await
    .expect("create trusted overlay mesh ticket");
    assert_eq!(ticket_response.status, "200 OK", "{}", ticket_response.body);
    let ticket = serde_json::from_str::<serde_json::Value>(&ticket_response.body).unwrap()
        ["ticket"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut preview = crate::open_remote_mesh_preview_file(&local_state, "mesh", &ticket)
        .await
        .expect("fetch trusted overlay mesh content")
        .expect("trusted overlay preview file");
    let mut received = Vec::new();
    preview
        .file
        .read_to_end(&mut received)
        .expect("read trusted overlay preview");
    assert_eq!(received, content);
    assert_eq!(preview.length, content.len() as u64);
    let cleanup = preview.cleanup_path.take().expect("preview cleanup path");
    drop(preview);
    std::fs::remove_file(cleanup).expect("remove trusted preview staging file");

    gateway_server.abort();
    let _ = std::fs::remove_dir_all(&remote_state.config.state_dir);
    let _ = std::fs::remove_dir_all(&local_state.config.state_dir);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn websocket_origin_must_match_host_when_present() {
    let headers = crate::RequestSecurityHeaders {
        host: Some("127.0.0.1:5030".to_owned()),
        origin: Some("https://evil.example".to_owned()),
        referer: None,
        cookie: None,
        content_type: None,
        x_share_token: None,
        x_gateway_api_key: None,
        x_gateway_csrf: None,
        x_relay_agent: None,
        x_relay_credential: None,
        remote_addr: None,
        date: None,
        digest: None,
        signature: None,
    };
    assert!(!crate::request_origin_matches_host(
        &headers,
        "127.0.0.1:5030"
    ));

    let headers = crate::RequestSecurityHeaders {
        host: Some("127.0.0.1:5030".to_owned()),
        origin: Some("http://127.0.0.1:5030".to_owned()),
        referer: None,
        cookie: None,
        content_type: None,
        x_share_token: None,
        x_gateway_api_key: None,
        x_gateway_csrf: None,
        x_relay_agent: None,
        x_relay_credential: None,
        remote_addr: None,
        date: None,
        digest: None,
        signature: None,
    };
    assert!(crate::request_origin_matches_host(
        &headers,
        "127.0.0.1:5030"
    ));

    let headers = crate::RequestSecurityHeaders {
        host: Some("[::1]:5030".to_owned()),
        origin: Some("http://[::1]:5030".to_owned()),
        referer: None,
        cookie: None,
        content_type: None,
        x_share_token: None,
        x_gateway_api_key: None,
        x_gateway_csrf: None,
        x_relay_agent: None,
        x_relay_credential: None,
        remote_addr: None,
        date: None,
        digest: None,
        signature: None,
    };
    assert!(crate::request_origin_matches_host(&headers, "[::1]:5030"));

    for malformed_origin in [
        "127.0.0.1:5030",
        "null",
        "ftp://127.0.0.1:5030",
        "http://user@127.0.0.1:5030",
        "http://127.0.0.1:5030@evil.example",
        "http://127.0.0.1:5030:80",
    ] {
        let headers = crate::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_owned()),
            origin: Some(malformed_origin.to_owned()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        };
        assert!(!crate::request_origin_matches_host(
            &headers,
            "127.0.0.1:5030"
        ));
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn websocket_origin_normalizes_default_ports_and_dns_case() {
    for (host, origin) in [
        ("example.com", "http://EXAMPLE.com:80"),
        ("example.com:443", "https://example.com"),
        ("example.com.", "http://example.com"),
    ] {
        let headers = crate::RequestSecurityHeaders {
            host: Some(host.to_owned()),
            origin: Some(origin.to_owned()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        };
        assert!(crate::request_origin_matches_host(&headers, "localhost"));
    }
    assert!(!crate::same_origin_host("bad:port", "also:bad"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn websocket_connection_pool_reserves_http_capacity() {
    let semaphore = Arc::new(crate::Semaphore::new(crate::MAX_WEBSOCKET_CONNECTIONS));
    let permits = (0..crate::MAX_WEBSOCKET_CONNECTIONS)
        .map(|_| {
            Arc::clone(&semaphore)
                .try_acquire_owned()
                .expect("configured websocket permit")
        })
        .collect::<Vec<_>>();
    assert!(Arc::clone(&semaphore).try_acquire_owned().is_err());
    drop(permits);
    assert_eq!(
        semaphore.available_permits(),
        crate::MAX_WEBSOCKET_CONNECTIONS
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn websocket_auth_subprotocol_builds_bearer_authorization() {
    let header = "chat, slskr.api-token.route%2Dtoken%2Fwith%20space";
    assert_eq!(
        crate::websocket_auth_protocol(Some(header)),
        Some("slskr.api-token.route%2Dtoken%2Fwith%20space")
    );
    assert_eq!(
        crate::websocket_protocol_authorization(Some(header)).as_deref(),
        Some("Bearer route-token/with space")
    );
    assert_eq!(crate::websocket_auth_protocol(Some("chat")), None);
    assert_eq!(
        crate::websocket_protocol_authorization(Some("slskr.api-token.")),
        None
    );
    for malformed in [
        "slskr.api-token.route/token",
        "slskr.api-token.route token",
        "slskr.api-token.route\"token",
        "slskr.api-token.route%",
        "slskr.api-token.route%GG",
        "slskr.api-token.%FF",
        "slskr.api-token.%00",
        "slskr.api-token.tokén",
    ] {
        assert_eq!(crate::websocket_auth_protocol(Some(malformed)), None);
        assert_eq!(
            crate::websocket_protocol_authorization(Some(malformed)),
            None
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn websocket_auth_subprotocol_rejects_header_credentials() {
    let websocket_auth = Some("Bearer route-token");
    for headers in [
        crate::http_server::HttpHeaders {
            authorization: Some("Bearer route-token".to_owned()),
            ..Default::default()
        },
        crate::http_server::HttpHeaders {
            x_api_key: Some("route-token".to_owned()),
            ..Default::default()
        },
    ] {
        assert!(crate::mixed_websocket_auth_credentials(
            &headers,
            websocket_auth
        ));
    }
    assert!(!crate::mixed_websocket_auth_credentials(
        &crate::http_server::HttpHeaders::default(),
        websocket_auth
    ));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn websocket_auth_subprotocol_rejects_multiple_token_credentials() {
    let headers = crate::http_server::HttpHeaders {
        sec_websocket_protocol: Some(
            "chat, slskr.api-token.first, slskr.api-token.second".to_owned(),
        ),
        ..Default::default()
    };
    assert!(crate::mixed_websocket_auth_credentials(
        &headers,
        Some("Bearer first")
    ));

    let headers = crate::http_server::HttpHeaders {
        sec_websocket_protocol: Some("chat, slskr.api-token.only, telemetry".to_owned()),
        ..Default::default()
    };
    assert!(!crate::mixed_websocket_auth_credentials(
        &headers,
        Some("Bearer only")
    ));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn websocket_subprotocol_authorizes_event_feed_route() {
    let env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            &std::env::temp_dir().display().to_string(),
        )
        .with("SLSKR_API_TOKEN", "route-token");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("auth config");
    let headers = crate::RequestSecurityHeaders {
        host: Some("127.0.0.1:5030".to_string()),
        origin: Some("http://127.0.0.1:5030".to_string()),
        referer: None,
        cookie: None,
        content_type: None,
        x_share_token: None,
        x_gateway_api_key: None,
        x_gateway_csrf: None,
        x_relay_agent: None,
        x_relay_credential: None,
        remote_addr: None,
        date: None,
        digest: None,
        signature: None,
    };
    let auth = crate::websocket_protocol_authorization(Some("slskr.api-token.route%2Dtoken"));

    assert_eq!(auth.as_deref(), Some("Bearer route-token"));
    assert!(crate::routing::check_route_auth(
        &config,
        "GET",
        "/api/events/ws",
        auth.as_deref(),
        &headers,
    )
    .is_ok());
    assert_eq!(
        crate::routing::check_route_auth(&config, "GET", "/api/events/ws", None, &headers,),
        Err("unauthorized")
    );
}

/// Exhaustive in-process differential proof for the manifest's
/// `security-authorization` workstream (`scripts/audit-parity-manifest.py`
/// `api_entries()`): every declared rule in both frozen controller
/// auth-policy registries, against every one of the manifest's 10
/// credential profiles, through the exact same `check_route_auth` gate
/// the live HTTP server calls before any handler dispatches. Writes a
/// machine-readable ledger the manifest script reads to move cases from
/// `needs-proof` to `complete` -- this is real, executable evidence, not
/// a route-presence check.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-authorization-tests"
))]
pub(super) fn security_authorization_matrix_matches_declared_policy_for_every_frozen_route() {
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
        access: String,
        scheme: String,
        scopes: Vec<String>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Outcome {
        Allowed,
        Unauthorized,
        Forbidden,
    }

    #[derive(Clone, Copy, Debug)]
    enum Profile {
        Anonymous,
        BasicReadOnly,
        BasicReadWrite,
        BasicAdministrator,
        BearerReadOnly,
        BearerReadWrite,
        BearerAdministrator,
        InvalidOrExpiredCredential,
        MissingRequiredScope,
        WrongAuthenticationScheme,
    }

    const PROFILES: [Profile; 10] = [
        Profile::Anonymous,
        Profile::BasicReadOnly,
        Profile::BasicReadWrite,
        Profile::BasicAdministrator,
        Profile::BearerReadOnly,
        Profile::BearerReadWrite,
        Profile::BearerAdministrator,
        Profile::InvalidOrExpiredCredential,
        Profile::MissingRequiredScope,
        Profile::WrongAuthenticationScheme,
    ];

    fn profile_case(profile: Profile) -> &'static str {
        match profile {
            Profile::Anonymous => "anonymous",
            Profile::BasicReadOnly => "basic-readonly",
            Profile::BasicReadWrite => "basic-readwrite",
            Profile::BasicAdministrator => "basic-administrator",
            Profile::BearerReadOnly => "bearer-readonly",
            Profile::BearerReadWrite => "bearer-readwrite",
            Profile::BearerAdministrator => "bearer-administrator",
            Profile::InvalidOrExpiredCredential => "invalid-or-expired-credential",
            Profile::MissingRequiredScope => "missing-required-scope",
            Profile::WrongAuthenticationScheme => "wrong-authentication-scheme",
        }
    }

    // (Authorization header value, credential-as-derived-by-`api_credential`:
    // access rank 0=authenticated/1=read_write/2=administrator, scheme,
    // nowplaying-only) -- `None` credential means the gate sees nobody
    // recognized at all (anonymous or an invalid/unrecognized token).
    fn profile_header_and_credential(
        profile: Profile,
        rule_scheme: &str,
    ) -> (Option<&'static str>, Option<(u8, &'static str, bool)>) {
        match profile {
            Profile::Anonymous => (None, None),
            Profile::BasicReadOnly => (Some("ApiKey read-token"), Some((0, "api_key", false))),
            Profile::BasicReadWrite => (Some("ApiKey write-token"), Some((1, "api_key", false))),
            Profile::BasicAdministrator => {
                (Some("ApiKey admin-token"), Some((2, "api_key", false)))
            }
            Profile::BearerReadOnly => (Some("Bearer read-token"), Some((0, "jwt", false))),
            Profile::BearerReadWrite => (Some("Bearer write-token"), Some((1, "jwt", false))),
            Profile::BearerAdministrator => (Some("Bearer admin-token"), Some((2, "jwt", false))),
            Profile::InvalidOrExpiredCredential => {
                (Some("Bearer not-a-real-differential-token"), None)
            }
            Profile::MissingRequiredScope => {
                (Some("ApiKey nowplaying-token"), Some((1, "api_key", true)))
            }
            Profile::WrongAuthenticationScheme => {
                if rule_scheme == "jwt" {
                    (Some("ApiKey admin-token"), Some((2, "api_key", false)))
                } else {
                    (Some("Bearer admin-token"), Some((2, "jwt", false)))
                }
            }
        }
    }

    fn required_access_rank(access: &str) -> Option<u8> {
        match access {
            "anonymous" | "delegated" => None,
            "administrator" => Some(2),
            "read_write" => Some(1),
            _ => Some(0),
        }
    }

    // Independently reconstructs the expected outcome from the
    // *declared* rule data (not by calling the production decision
    // function itself, which would make this circular) -- mirrors
    // `authorize_controller_route_from`'s real precedence: access rank,
    // then scheme, then the nowplaying-only scope restriction.
    fn expected_outcome(rule: &AuthPolicyRow, profile: Profile) -> Outcome {
        let Some(required) = required_access_rank(&rule.access) else {
            return Outcome::Allowed;
        };
        let (_, credential) = profile_header_and_credential(profile, &rule.scheme);
        let Some((cred_rank, cred_scheme, nowplaying_only)) = credential else {
            return Outcome::Unauthorized;
        };
        if cred_rank < required {
            return Outcome::Forbidden;
        }
        if rule.scheme.as_str() != "any" && cred_scheme != rule.scheme.as_str() {
            return Outcome::Forbidden;
        }
        let requires_nowplaying = rule.scopes.iter().any(|scope| scope == "nowplaying");
        if nowplaying_only && !requires_nowplaying {
            return Outcome::Forbidden;
        }
        Outcome::Allowed
    }

    // Auth is checked at route-template level before any handler touches
    // real data, so a fixed placeholder is a faithful stand-in for every
    // `{param}` segment; an unusual literal minimizes any chance of
    // colliding with a real literal segment used by a different rule.
    fn placeholder_path(route: &str) -> String {
        let mut segments: Vec<String> = route
            .trim_matches('/')
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') && segment.ends_with('}') {
                    "differential-fixture-value".to_owned()
                } else {
                    segment.to_owned()
                }
            })
            .collect();
        if route.contains("{*") {
            segments.push("differential-fixture-tail".to_owned());
        }
        format!("/{}", segments.join("/"))
    }

    let headers = crate::RequestSecurityHeaders::default();
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    for (target, source) in [
        (
            "slskd",
            include_str!("../../data/legacy-controller-auth-policy.json"),
        ),
        (
            "slskdn",
            include_str!("../../data/native-controller-auth-policy.json"),
        ),
    ] {
        let rules: Vec<AuthPolicyRow> =
            serde_json::from_str(source).expect("checked controller auth policy registry");
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-security-auth-differential-{target}-{}",
            uuid::Uuid::new_v4()
        ));
        let config = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", state_dir.to_str().unwrap())
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_API_TOKEN", "admin-token")
                .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
                .with("SLSKR_API_READ_ONLY_TOKEN", "read-token")
                .with("SLSKR_API_NOWPLAYING_TOKEN", "nowplaying-token"),
        )
        .expect("hermetic auth-policy differential config");

        for rule in &rules {
            let path = placeholder_path(&rule.route);
            for profile in PROFILES {
                let (header, _) = profile_header_and_credential(profile, &rule.scheme);
                let expected = expected_outcome(rule, profile);
                let actual = match crate::routing::check_route_auth(
                    &config,
                    &rule.method,
                    &path,
                    header,
                    &headers,
                ) {
                    Ok(()) => Outcome::Allowed,
                    Err("unauthorized") => Outcome::Unauthorized,
                    Err("forbidden") => Outcome::Forbidden,
                    Err(other) => panic!(
                        "unexpected auth-gate outcome {other:?} for {target} {} {}",
                        rule.method, rule.route
                    ),
                };
                let pass = actual == expected;
                if !pass {
                    mismatches.push(format!(
                        "{target} {} {} [{}]: expected {:?}, got {:?}",
                        rule.method,
                        rule.route,
                        profile_case(profile),
                        expected,
                        actual
                    ));
                }
                ledger.push(serde_json::json!({
                    "target": target,
                    "method": rule.method,
                    "route": rule.route,
                    "case": profile_case(profile),
                    "pass": pass,
                    "expected": format!("{expected:?}"),
                    "actual": format!("{actual:?}"),
                }));
            }
        }
    }

    let evidence_dir = std::env::temp_dir().join("slskr-parity-evidence");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("security-authorization.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security-authorization ledger"),
    )
    .expect("write security-authorization ledger");

    assert!(
        mismatches.is_empty(),
        "{} security-authorization mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn cors_headers_reject_control_characters() {
    assert_eq!(
        crate::cors_headers(Some("https://example.test\r\nX-Bad: yes"), &["*"]),
        ""
    );
    let headers = crate::cors_headers(Some("https://example.test"), &["*"]);
    assert!(headers.contains("Access-Control-Allow-Origin: https://example.test"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn protected_api_cache_control_is_no_store() {
    assert_eq!(
        crate::cache_control_header("GET", "application/json", "/api/shares/catalog"),
        Some("Cache-Control: no-store\r\n".to_string())
    );
    assert_eq!(
        crate::cache_control_header("GET", "application/json", "/api/metrics"),
        Some("Cache-Control: no-store\r\n".to_string())
    );
    assert_eq!(
        crate::cache_control_header("GET", "application/json", "/api/capabilities"),
        Some("Cache-Control: public, max-age=3600\r\n".to_string())
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn configured_api_token_protects_api_routes() {
    std::thread::Builder::new()
        .name("configured-api-token-test".to_owned())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(configured_api_token_protects_api_routes_impl())
        })
        .unwrap()
        .join()
        .unwrap();
}
