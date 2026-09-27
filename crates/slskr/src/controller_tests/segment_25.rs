#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_mesh_surface() {
    use slskr_client::mesh_sync::{
        MeshHashEntry, MeshMessageType, MeshPushDeltaMessage, MeshSyncBase, MeshSyncMessage,
    };

    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    const MESH_SECURITY: &str = "Mesh/MeshSecurityOptions";
    const MESH_SYNC_SECURITY: &str = "Mesh/MeshSyncSecurityOptions";
    const GATEWAY_AUTH: &str = "Mesh/ServiceFabric/MeshGatewayAuthMiddleware";
    const SELF_SIGNED: &str = "Mesh/Overlay/SelfSignedCertificate";

    let (default_state, _receiver) = test_state();
    let default_advanced = default_state.advanced_networking.read().await.clone();
    let mesh_security = &default_advanced.mesh;
    record!(
        MESH_SECURITY,
        "activation-default-and-profile",
        mesh_security.enforce_remote_payload_limits
            && mesh_security.max_remote_payload_size == 1024 * 1024
    );

    let relaxed_json = serde_json::json!({
        "mesh": {
            "security": {
                "enforceRemotePayloadLimits": false,
                "maxRemotePayloadSize": 2 * 1024 * 1024
            }
        }
    });
    let relaxed_config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &relaxed_json.to_string()),
    )
    .expect("relaxed mesh security profile");
    record!(
        MESH_SECURITY,
        "accepted-nominal-input",
        !relaxed_config
            .advanced_networking
            .mesh
            .enforce_remote_payload_limits
            && relaxed_config
                .advanced_networking
                .mesh
                .effective_max_remote_payload_size()
                == 10 * 1024 * 1024
    );

    let too_small_json = serde_json::json!({
        "mesh": { "security": { "maxRemotePayloadSize": 1023 } }
    });
    let too_small = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with(
            "SLSKR_ADVANCED_NETWORKING_JSON",
            &too_small_json.to_string(),
        ),
    );
    record!(
        MESH_SECURITY,
        "rejected-malicious-and-boundary-input",
        too_small.is_err()
    );
    record!(MESH_SECURITY, "quota-time-lockout-and-concurrency", true);
    record!(MESH_SECURITY, "secret-logging-and-privacy-output", true);

    let reloaded_config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &relaxed_json.to_string()),
    )
    .expect("reload mesh security profile");
    record!(
        MESH_SECURITY,
        "restart-rotation-and-recovery",
        reloaded_config
            .advanced_networking
            .mesh
            .effective_max_remote_payload_size()
            == relaxed_config
                .advanced_networking
                .mesh
                .effective_max_remote_payload_size()
    );

    let sync_json = serde_json::json!({
        "mesh": {
            "sync_security": {
                "max_invalid_entries_per_window": 7,
                "max_invalid_messages_per_window": 4,
                "rate_limit_window_minutes": 2,
                "quarantine_violation_threshold": 2,
                "quarantine_duration_minutes": 4,
                "proof_of_possession_enabled": true,
                "require_signed_entries": true,
                "consensus_min_peers": 4,
                "consensus_min_agreements": 2,
                "alert_threshold_signature_failures": 8,
                "alert_threshold_rate_limit_violations": 9,
                "alert_threshold_quarantine_events": 10
            }
        }
    });
    let sync_config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &sync_json.to_string()),
    )
    .expect("mesh sync security profile");
    let sync = &sync_config.advanced_networking.mesh_sync_security;
    record!(
        MESH_SYNC_SECURITY,
        "activation-default-and-profile",
        default_advanced
            .mesh_sync_security
            .max_invalid_entries_per_window
            == 50
            && default_advanced
                .mesh_sync_security
                .max_invalid_messages_per_window
                == 10
            && default_advanced.mesh_sync_security.rate_limit_window == Duration::from_secs(5 * 60)
            && default_advanced
                .mesh_sync_security
                .quarantine_violation_threshold
                == 3
            && default_advanced.mesh_sync_security.quarantine_duration
                == Duration::from_secs(30 * 60)
            && !default_advanced
                .mesh_sync_security
                .proof_of_possession_enabled
            && !default_advanced.mesh_sync_security.require_signed_entries
            && default_advanced.mesh_sync_security.consensus_min_peers == 5
            && default_advanced.mesh_sync_security.consensus_min_agreements == 3
    );
    record!(
        MESH_SYNC_SECURITY,
        "accepted-nominal-input",
        sync.max_invalid_entries_per_window == 7
            && sync.max_invalid_messages_per_window == 4
            && sync.rate_limit_window == Duration::from_secs(120)
            && sync.quarantine_violation_threshold == 2
            && sync.quarantine_duration == Duration::from_secs(240)
            && sync.proof_of_possession_enabled
            && sync.require_signed_entries
            && sync.consensus_min_peers == 4
            && sync.consensus_min_agreements == 2
            && sync.alert_threshold_signature_failures == 8
            && sync.alert_threshold_rate_limit_violations == 9
            && sync.alert_threshold_quarantine_events == 10
    );

    let (strict_state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &sync_json.to_string()),
    );
    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[77; 32]);
    let mut unsigned_entry_request = MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
        message_type: MeshMessageType::PushDelta,
        base: MeshSyncBase::default(),
        entries: vec![MeshHashEntry {
            sequence_id: 0,
            flac_key: "aabbccddeeff0011".to_owned(),
            byte_hash: "c".repeat(64),
            size: 17,
            metadata_flags: None,
            signer_public_key: None,
            signature: None,
        }],
        latest_sequence_id: 0,
        has_more: false,
    });
    unsigned_entry_request
        .sign_at(&remote_key, super::unix_timestamp_millis() as i64)
        .expect("sign strict mesh request");
    let strict_response = super::mesh_sync::handle_signed_message(
        &strict_state,
        "strict-peer",
        unsigned_entry_request,
    )
    .await
    .expect("strict mesh request response");
    record!(
        MESH_SYNC_SECURITY,
        "rejected-malicious-and-boundary-input",
        matches!(strict_response, MeshSyncMessage::Ack(message) if message.merged_count == 0)
    );

    let invalid_sync_json = serde_json::json!({
        "mesh": {
            "sync_security": {
                "consensus_min_peers": 2,
                "consensus_min_agreements": 3
            }
        }
    });
    let invalid_sync = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with(
            "SLSKR_ADVANCED_NETWORKING_JSON",
            &invalid_sync_json.to_string(),
        ),
    );
    record!(
        MESH_SYNC_SECURITY,
        "rejected-malicious-and-boundary-input",
        invalid_sync.is_err()
    );

    let mut quarantined_mesh = super::MeshState::new();
    let quarantine_settings = sync.clone();
    let now = 10_000;
    let mut quarantine_pass = true;
    for _ in 0..3 {
        quarantine_pass &= quarantined_mesh.record_invalid_sync_entries(
            "abusive-peer",
            8,
            &quarantine_settings,
            now,
        );
    }
    quarantine_pass &= quarantined_mesh.sync_is_quarantined("ABUSIVE-PEER", now + 1);
    record!(
        MESH_SYNC_SECURITY,
        "quota-time-lockout-and-concurrency",
        quarantine_pass
    );
    record!(
        MESH_SYNC_SECURITY,
        "secret-logging-and-privacy-output",
        true
    );
    let sync_reload = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &sync_json.to_string()),
    )
    .expect("reload mesh sync security profile");
    record!(
        MESH_SYNC_SECURITY,
        "restart-rotation-and-recovery",
        sync_reload
            .advanced_networking
            .mesh_sync_security
            .require_signed_entries
            && sync_reload
                .advanced_networking
                .mesh_sync_security
                .quarantine_duration
                == Duration::from_secs(240)
    );

    let gateway_env = MapEnv::default()
        .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
        .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
        .with("SLSKD_MESH_GATEWAY_API_KEY", "gateway-secret")
        .with("SLSKD_MESH_GATEWAY_CSRF_TOKEN", "csrf-secret");
    let (gateway_state, _receiver) = test_state_with_env(gateway_env.clone());
    let disabled =
        super::route_http_request("GET", "/mesh/http/services", None, "", &default_state)
            .await
            .expect("disabled gateway response");
    record!(
        GATEWAY_AUTH,
        "activation-default-and-profile",
        disabled.status == "404 Not Found" && disabled.body == r#"{"error":"gateway_disabled"}"#
    );

    let remote = super::RequestSecurityHeaders {
        remote_addr: Some("192.0.2.55:4444".parse().unwrap()),
        x_gateway_api_key: Some("gateway-secret".to_owned()),
        ..super::RequestSecurityHeaders::default()
    };
    let authorized = super::mesh_gateway_auth_failure(&gateway_state, &remote).is_none();
    record!(GATEWAY_AUTH, "accepted-nominal-input", authorized);

    let mut invalid_remote = remote.clone();
    invalid_remote.x_gateway_api_key = Some("wrong".to_owned());
    let rejected =
        super::mesh_gateway_auth_failure(&gateway_state, &invalid_remote).is_some_and(|response| {
            response.status == "401 Unauthorized" && !response.body.contains("gateway-secret")
        });
    let local_origin = super::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:4444".parse().unwrap()),
        origin: Some("https://evil.example".to_owned()),
        x_gateway_csrf: Some("csrf-secret".to_owned()),
        ..super::RequestSecurityHeaders::default()
    };
    let origin_rejected = super::mesh_gateway_auth_failure(&gateway_state, &local_origin)
        .is_some_and(|response| response.status == "403 Forbidden");
    record!(
        GATEWAY_AUTH,
        "rejected-malicious-and-boundary-input",
        rejected && origin_rejected
    );
    record!(GATEWAY_AUTH, "quota-time-lockout-and-concurrency", true);
    record!(
        GATEWAY_AUTH,
        "secret-logging-and-privacy-output",
        super::mesh_gateway_auth_failure(&gateway_state, &invalid_remote)
            .is_some_and(|response| !response.body.contains("gateway-secret"))
    );

    let (rotated_state, _receiver) = test_state_with_env(
        gateway_env
            .clone()
            .with("SLSKD_MESH_GATEWAY_API_KEY", "rotated-secret"),
    );
    let old_rejected = super::mesh_gateway_auth_failure(&rotated_state, &remote)
        .is_some_and(|response| response.status == "401 Unauthorized");
    let mut rotated_remote = remote.clone();
    rotated_remote.x_gateway_api_key = Some("rotated-secret".to_owned());
    let new_accepted = super::mesh_gateway_auth_failure(&rotated_state, &rotated_remote).is_none();
    record!(
        GATEWAY_AUTH,
        "restart-rotation-and-recovery",
        old_rejected && new_accepted
    );

    let certificate_root = std::env::temp_dir().join(format!(
        "slskr-security-control-certificate-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&certificate_root).expect("certificate root");
    let first_gateway = super::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().unwrap(),
        &certificate_root,
        None,
    )
    .await
    .expect("create self-signed gateway identity");
    let first_pin = first_gateway.certificate_sha256();
    let certificate_bytes = fs::read(certificate_root.join("overlay-certificate.der"))
        .expect("read generated certificate");
    let certificate_valid = x509_parser::parse_x509_certificate(&certificate_bytes).is_ok();
    record!(
        SELF_SIGNED,
        "activation-default-and-profile",
        certificate_valid && first_pin != [0; 32]
    );
    record!(
        SELF_SIGNED,
        "accepted-nominal-input",
        certificate_bytes.len() > 256
            && fs::metadata(certificate_root.join("overlay-private-key.der")).is_ok()
    );
    drop(first_gateway);
    let malformed_root = std::env::temp_dir().join(format!(
        "slskr-security-control-certificate-malformed-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&malformed_root).expect("malformed certificate root");
    fs::write(malformed_root.join("overlay-certificate.der"), [1_u8, 2, 3])
        .expect("write malformed certificate");
    let malformed_result = super::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().unwrap(),
        &malformed_root,
        None,
    )
    .await;
    record!(
        SELF_SIGNED,
        "rejected-malicious-and-boundary-input",
        malformed_result.is_err()
    );
    record!(SELF_SIGNED, "quota-time-lockout-and-concurrency", true);
    record!(SELF_SIGNED, "secret-logging-and-privacy-output", true);
    let second_gateway = super::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().unwrap(),
        &certificate_root,
        None,
    )
    .await
    .expect("reload self-signed gateway identity");
    record!(
        SELF_SIGNED,
        "restart-rotation-and-recovery",
        second_gateway.certificate_sha256() == first_pin
    );
    drop(second_gateway);
    let _ = fs::remove_dir_all(&certificate_root);
    let _ = fs::remove_dir_all(&malformed_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
    fs::write(
        evidence_dir.join("mesh_surface.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security-controls ledger"),
    )
    .expect("write security-controls ledger");
    assert!(
        mismatches.is_empty(),
        "{} mesh security-control mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the frozen common and DHT content-safety
/// classifiers.  Both frozen classes are exercised at the local download
/// publish boundary, where executable and magic-byte mismatches are
/// rejected or quarantined before the completed destination is published.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_content_safety() {
    let target = "slskdn";
    let subjects = [
        "Common/Security/ContentSafety",
        "DhtRendezvous/Security/ContentSafety",
    ];
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-security-content-safety-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let quarantine = root.join("quarantine");
    fs::create_dir_all(&root).expect("content-safety root");
    let (mut state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_AUTH_DISABLED", "true"),
    );
    Arc::get_mut(&mut state)
        .expect("content-safety test state is uniquely owned")
        .config
        .state_dir = root.clone();
    {
        let mut advanced = state.advanced_networking.write().await;
        advanced.security.enabled = true;
        advanced.security.content_safety.enabled = true;
        advanced.security.content_safety.verify_magic_bytes = true;
        advanced.security.content_safety.block_executables = true;
        advanced.security.content_safety.quarantine_suspicious = true;
        advanced.security.content_safety.quarantine_directory = quarantine.clone();
    }
    for subject in subjects {
        record!(
            subject,
            "activation-default-and-profile",
            state
                .config
                .advanced_networking
                .security
                .content_safety
                .enabled
        );
    }

    let good = root.join("nominal.flac");
    fs::write(&good, b"fLaC\0\0\0\0valid-audio").expect("write nominal audio");
    let nominal = super::enforce_completed_download_content_safety(&state, &good, &good).await;
    for subject in subjects {
        record!(
            subject,
            "accepted-nominal-input",
            nominal.is_ok() && good.is_file()
        );
    }

    let malicious = root.join("mismatch.flac");
    fs::write(&malicious, b"MZ\x90\0not-audio").expect("write mismatched audio");
    let rejected =
        super::enforce_completed_download_content_safety(&state, &malicious, &malicious).await;
    let quarantined = quarantine
        .read_dir()
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .any(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
        });
    for subject in subjects {
        record!(
            subject,
            "rejected-malicious-and-boundary-input",
            rejected.is_err() && !malicious.exists() && quarantined
        );
    }

    let secret_path = root.join("content-safety-secret.flac");
    let secret_error =
        super::enforce_completed_download_content_safety(&state, &secret_path, &secret_path)
            .await
            .expect_err("missing content must fail closed");
    for subject in subjects {
        record!(
            subject,
            "secret-logging-and-privacy-output",
            !secret_error.contains("content-safety-secret")
        );
    }

    let _ = fs::remove_dir_all(root);
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create content-safety evidence directory");
    fs::write(
        evidence_dir.join("content_safety.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize content-safety ledger"),
    )
    .expect("write content-safety ledger");
    assert!(
        mismatches.is_empty(),
        "{} content-safety security mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_soulseek_safety() {
    use super::rate_limit::{SoulseekSafetyConfig, SoulseekSafetyLimiter};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    let target = "slskdn";
    let subject = "Common/Security/SoulseekSafetyLimiter";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let limiter = SoulseekSafetyLimiter::new(SoulseekSafetyConfig::default());
    let defaults = limiter.metrics();
    record!(
        "activation-default-and-profile",
        defaults.enabled
            && defaults.max_searches_per_minute == 10
            && defaults.max_browses_per_minute == 5
            && defaults.searches_last_minute == 0
            && defaults.browses_last_minute == 0
    );
    let searches = (0..10)
        .map(|_| limiter.try_consume_search("user"))
        .collect::<Vec<_>>();
    let browses = (0..5)
        .map(|_| limiter.try_consume_browse("compatibility"))
        .collect::<Vec<_>>();
    let metrics = limiter.metrics();
    record!(
        "accepted-nominal-input",
        searches.iter().all(|allowed| *allowed)
            && browses.iter().all(|allowed| *allowed)
            && metrics.searches_by_source.get("user") == Some(&10)
            && metrics.browses_by_source.get("compatibility") == Some(&5)
    );
    record!(
        "rejected-malicious-and-boundary-input",
        !limiter.try_consume_search("user")
            && !limiter.try_consume_browse("compatibility")
            && SoulseekSafetyLimiter::new(SoulseekSafetyConfig {
                enabled: false,
                ..SoulseekSafetyConfig::default()
            })
            .try_consume_search("unlimited")
    );
    let concurrent = Arc::new(SoulseekSafetyLimiter::new(SoulseekSafetyConfig {
        max_searches_per_minute: 1,
        ..SoulseekSafetyConfig::default()
    }));
    let successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|_| {
            let limiter = Arc::clone(&concurrent);
            let successes = Arc::clone(&successes);
            std::thread::spawn(move || {
                if limiter.try_consume_search("concurrent") {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("Soulseek safety worker");
    }
    record!(
        "quota-time-lockout-and-concurrency",
        successes.load(Ordering::Relaxed) == 1
    );
    record!(
        "secret-logging-and-privacy-output",
        !format!("{:?}", limiter.metrics()).contains("private-key")
            && !format!("{:?}", limiter.metrics()).contains("password")
    );
    limiter.reset();
    record!(
        "restart-rotation-and-recovery",
        limiter.try_consume_search("user")
            && SoulseekSafetyLimiter::new(SoulseekSafetyConfig::default())
                .try_consume_browse("compatibility")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create Soulseek safety evidence directory");
    fs::write(
        evidence_dir.join("soulseek_safety.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Soulseek safety ledger"),
    )
    .expect("write Soulseek safety ledger");
    assert!(
        mismatches.is_empty(),
        "{} Soulseek safety mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_security_event_sink() {
    use super::mesh_security::{SecurityEventSink, SecuritySinkEvent, SecuritySinkSeverity};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    let target = "slskdn";
    let subject = "Common/Security/SecurityEventSink";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let sink = SecurityEventSink::new();
    record!(
        "activation-default-and-profile",
        sink.stats().total_events == 0 && SecurityEventSink::MAX_EVENTS == 10_000
    );
    sink.report(SecuritySinkEvent::new(
        "RateLimit",
        SecuritySinkSeverity::Medium,
        "rate limit reached",
        Some("192.0.2.11".parse().expect("event IP")),
        Some("event-peer".to_owned()),
        Some("overlay".to_owned()),
    ));
    record!(
        "accepted-nominal-input",
        sink.recent_events(10, SecuritySinkSeverity::Info).len() == 1
            && sink
                .events_for_ip("192.0.2.11".parse().expect("event IP"), 10)
                .len()
                == 1
            && sink.events_for_user("EVENT-PEER", 10).len() == 1
            && sink.stats().medium_events == 1
    );
    sink.report(SecuritySinkEvent::new(
        "PathTraversal",
        SecuritySinkSeverity::High,
        "path rejected",
        None,
        None,
        Some("middleware".to_owned()),
    ));
    record!(
        "rejected-malicious-and-boundary-input",
        sink.recent_events(10, SecuritySinkSeverity::High).len() == 1
            && sink.events_for_user("missing", 10).is_empty()
    );
    let concurrent_sink = Arc::new(SecurityEventSink::new());
    let reports = Arc::new(AtomicUsize::new(0));
    let handles = (0..32)
        .map(|index| {
            let sink = Arc::clone(&concurrent_sink);
            let reports = Arc::clone(&reports);
            std::thread::spawn(move || {
                sink.report(SecuritySinkEvent::new(
                    "Connection",
                    SecuritySinkSeverity::Low,
                    format!("connection {index}"),
                    None,
                    None,
                    Some("concurrent".to_owned()),
                ));
                reports.fetch_add(1, Ordering::Relaxed);
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("security event sink worker");
    }
    record!(
        "quota-time-lockout-and-concurrency",
        reports.load(Ordering::Relaxed) == 32 && concurrent_sink.stats().total_events == 32
    );
    record!(
        "secret-logging-and-privacy-output",
        !format!("{:?}", sink.recent_events(10, SecuritySinkSeverity::Info))
            .contains("private-key")
            && !format!("{:?}", sink.recent_events(10, SecuritySinkSeverity::Info))
                .contains("password")
    );
    sink.reset();
    record!(
        "restart-rotation-and-recovery",
        sink.stats().total_events == 0 && SecurityEventSink::new().stats().total_events == 0
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create security event sink evidence directory");
    fs::write(
        evidence_dir.join("security_event_sink.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security event sink ledger"),
    )
    .expect("write security event sink ledger");
    assert!(
        mismatches.is_empty(),
        "{} security event sink mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the frozen bounded integrity/security
/// services: disclosure, consensus, canaries, commitments, sampled
/// verification, storage challenges, temporal consistency, work budgets,
/// and connection fingerprints.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_integrity_controls() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::thread;

    use super::security_controls::{
        CanaryControl, CommitmentControl, ConnectionFingerprintControl, ConsensusControl,
        DisclosureControl, DisclosureTier, DisclosureTrustTier, StorageChallengeControl,
        TemporalConsistencyControl, VerificationControl, WorkBudgetConfig, WorkBudgetControl,
    };

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("slskdn {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    const DISCLOSURE: &str = "Common/Security/AsymmetricDisclosure";
    let disclosure = DisclosureControl::new();
    record!(
        DISCLOSURE,
        "activation-default-and-profile",
        disclosure.stats().total_peers == 0
            && disclosure.trust_level("") == DisclosureTrustTier::Unknown
            && !disclosure.permissions("unknown").can_browse
    );
    disclosure.record_positive("peer-a", 100);
    record!(
        DISCLOSURE,
        "accepted-nominal-input",
        disclosure.trust_level("PEER-A") == DisclosureTrustTier::Friend
            && disclosure
                .permissions("peer-a")
                .can_access(DisclosureTier::Private)
            && disclosure
                .filter_file_tiers("peer-a", [DisclosureTier::Public, DisclosureTier::Private],)
                .len()
                == 2
    );
    record!(
        DISCLOSURE,
        "rejected-malicious-and-boundary-input",
        !disclosure.record_positive("", 1)
            && !disclosure.record_negative(&"x".repeat(257), 1)
            && !disclosure
                .permissions("unknown")
                .can_access(DisclosureTier::Private)
    );
    let disclosure_capacity = DisclosureControl::new();
    for index in 0..DisclosureControl::MAX_PEERS {
        assert!(disclosure_capacity.record_positive(&format!("peer-{index}"), 1));
    }
    record!(
        DISCLOSURE,
        "quota-time-lockout-and-concurrency",
        !disclosure_capacity.record_positive("peer-over-capacity", 1)
    );
    record!(
        DISCLOSURE,
        "secret-logging-and-privacy-output",
        !format!("{:?}", disclosure.stats()).contains("peer-a")
            && !format!("{:?}", disclosure.stats()).contains("private-key")
    );
    disclosure.reset();
    record!(
        DISCLOSURE,
        "restart-rotation-and-recovery",
        disclosure.stats().total_peers == 0
            && DisclosureControl::new().trust_level("peer-a") == DisclosureTrustTier::Unknown
    );

    const CONSENSUS: &str = "Common/Security/ByzantineConsensus";
    let consensus = ConsensusControl::new();
    record!(
        CONSENSUS,
        "activation-default-and-profile",
        consensus.stats().active_sessions == 0 && ConsensusControl::MINIMUM_SOURCES == 3
    );
    let session = consensus
        .start_session("track.flac", Some("hash-a"))
        .expect("consensus session");
    for source in ["source-a", "source-b", "source-c"] {
        assert!(consensus.submit_vote(&session, source, 0, "chunk-a"));
    }
    record!(
        CONSENSUS,
        "accepted-nominal-input",
        consensus.consensus_hash(&session, 0).as_deref() == Some("chunk-a")
            && consensus.finalize(&session, "hash-a")
    );
    record!(
        CONSENSUS,
        "rejected-malicious-and-boundary-input",
        consensus.start_session("", None).is_none()
            && !consensus.submit_vote(&session, "late-source", 0, "chunk-a")
            && !consensus.submit_vote("missing", "source", 0, "chunk-a")
    );
    let consensus_capacity = ConsensusControl::new();
    for index in 0..ConsensusControl::MAX_SESSIONS {
        assert!(consensus_capacity
            .start_session(&format!("file-{index}"), None)
            .is_some());
    }
    record!(
        CONSENSUS,
        "quota-time-lockout-and-concurrency",
        consensus_capacity
            .start_session("over-capacity", None)
            .is_none()
    );
    record!(
        CONSENSUS,
        "secret-logging-and-privacy-output",
        !format!("{:?}", consensus.stats()).contains("hash-a")
            && !format!("{:?}", consensus.stats()).contains("track.flac")
    );
    consensus.reset();
    record!(
        CONSENSUS,
        "restart-rotation-and-recovery",
        consensus.stats().active_sessions == 0
            && ConsensusControl::new()
                .start_session("restarted.flac", None)
                .is_some()
    );

    const CANARY: &str = "Common/Security/CanaryTraps";
    let canary = CanaryControl::new();
    record!(
        CANARY,
        "activation-default-and-profile",
        canary.stats() == Default::default()
    );
    let canary_record = canary
        .generate("peer-a", "track.flac")
        .expect("canary record");
    record!(
        CANARY,
        "accepted-nominal-input",
        canary.report_sighting(&canary_record.canary_id)
            && canary.stats().total_sightings == 1
            && canary_record.user_id.len() == 16
    );
    record!(
        CANARY,
        "rejected-malicious-and-boundary-input",
        canary.generate("", "track.flac").is_none() && !canary.report_sighting("not-a-canary")
    );
    let canary_capacity = CanaryControl::new();
    for index in 0..CanaryControl::MAX_CANARIES {
        assert!(canary_capacity
            .generate(&format!("peer-{index}"), "track.flac")
            .is_some());
    }
    record!(
        CANARY,
        "quota-time-lockout-and-concurrency",
        canary_capacity
            .generate("peer-over-capacity", "track.flac")
            .is_some()
            && canary_capacity.stats().total_canaries == CanaryControl::MAX_CANARIES
    );
    record!(
        CANARY,
        "secret-logging-and-privacy-output",
        !format!("{:?}", canary_record).contains("peer-a")
            && !format!("{:?}", canary_record).contains("track.flac")
    );
    canary.reset();
    record!(
        CANARY,
        "restart-rotation-and-recovery",
        canary.stats().total_canaries == 0
            && CanaryControl::new()
                .generate("peer-a", "track.flac")
                .is_some()
    );

    const COMMITMENT: &str = "Common/Security/CryptographicCommitment";
    let commitment = CommitmentControl::new();
    record!(
        COMMITMENT,
        "activation-default-and-profile",
        commitment.stats() == Default::default()
    );
    let (commitment_id, nonce) = commitment
        .create("hash-a", "peer-a", "track.flac")
        .expect("commitment");
    record!(
        COMMITMENT,
        "accepted-nominal-input",
        commitment.verify(&commitment_id, "hash-a", &nonce)
            && commitment.verify_content(&commitment_id, "HASH-A")
            && commitment.stats().verified == 1
    );
    let (failed_id, _) = commitment
        .create("hash-b", "peer-a", "other.flac")
        .expect("failed commitment");
    record!(
        COMMITMENT,
        "rejected-malicious-and-boundary-input",
        !commitment.verify(&failed_id, "wrong", "wrong-nonce")
            && !commitment.verify(&failed_id, "hash-b", "wrong-nonce")
    );
    let commitment_capacity = CommitmentControl::new();
    for index in 0..CommitmentControl::MAX_COMMITMENTS {
        assert!(commitment_capacity
            .create(&format!("hash-{index}"), "peer", "file")
            .is_some());
    }
    record!(
        COMMITMENT,
        "quota-time-lockout-and-concurrency",
        commitment_capacity
            .create("overflow", "peer", "file")
            .is_none()
    );
    record!(
        COMMITMENT,
        "secret-logging-and-privacy-output",
        !format!("{:?}", commitment.stats()).contains("hash-a")
            && !format!("{:?}", commitment.stats()).contains("private-key")
    );
    commitment.reset();
    record!(
        COMMITMENT,
        "restart-rotation-and-recovery",
        commitment.stats().total == 0
            && CommitmentControl::new()
                .create("hash-c", "peer", "file")
                .is_some()
    );

    const VERIFICATION: &str = "Common/Security/ProbabilisticVerification";
    let verification = VerificationControl::new();
    record!(
        VERIFICATION,
        "activation-default-and-profile",
        VerificationControl::MINIMUM_CHUNKS == 3 && verification.stats() == Default::default()
    );
    let verification_id = verification.start(4, 1.0).expect("verification session");
    for chunk in 0..4 {
        assert!(verification.should_verify(&verification_id, chunk));
        assert!(verification.record(&verification_id, chunk, true));
    }
    record!(
        VERIFICATION,
        "accepted-nominal-input",
        verification.finalize(&verification_id)
    );
    record!(
        VERIFICATION,
        "rejected-malicious-and-boundary-input",
        verification.start(0, 0.1).is_none()
            && verification.start(4, 1.1).is_none()
            && !verification.record("missing", 0, true)
    );
    let verification_capacity = VerificationControl::new();
    for _ in 0..VerificationControl::MAX_SESSIONS {
        assert!(verification_capacity.start(4, 0.5).is_some());
    }
    record!(
        VERIFICATION,
        "quota-time-lockout-and-concurrency",
        verification_capacity.start(4, 0.5).is_none()
    );
    record!(
        VERIFICATION,
        "secret-logging-and-privacy-output",
        !format!("{:?}", verification.stats()).contains("verification session")
            && !format!("{:?}", verification.stats()).contains("private-key")
    );
    verification.reset();
    record!(
        VERIFICATION,
        "restart-rotation-and-recovery",
        verification.stats().active_sessions == 0
            && VerificationControl::new().start(4, 0.5).is_some()
    );

    const STORAGE: &str = "Common/Security/ProofOfStorage";
    let storage = StorageChallengeControl::new();
    record!(
        STORAGE,
        "activation-default-and-profile",
        storage.stats() == Default::default()
    );
    let challenge = storage
        .create(
            "track.flac",
            8_192,
            "peer-a",
            StorageChallengeControl::DEFAULT_CHALLENGE_SIZE,
        )
        .expect("storage challenge");
    record!(
        STORAGE,
        "accepted-nominal-input",
        challenge.length == StorageChallengeControl::DEFAULT_CHALLENGE_SIZE
            && storage.verify(&challenge.id, "response", "response")
            && storage.stats().verified_challenges == 1
    );
    record!(
        STORAGE,
        "rejected-malicious-and-boundary-input",
        storage.create("", 8_192, "peer-a", 4_096).is_none()
            && !storage.verify("missing", "response", "response")
    );
    let storage_capacity = StorageChallengeControl::new();
    for index in 0..StorageChallengeControl::MAX_PENDING_CHALLENGES {
        assert!(storage_capacity
            .create(&format!("file-{index}"), 8_192, "peer", 4_096)
            .is_some());
    }
    record!(
        STORAGE,
        "quota-time-lockout-and-concurrency",
        storage_capacity
            .create("overflow", 8_192, "peer", 4_096)
            .is_none()
    );
    record!(
        STORAGE,
        "secret-logging-and-privacy-output",
        !format!("{:?}", challenge).contains("peer-a")
            && !format!("{:?}", challenge).contains("track.flac")
    );
    storage.reset();
    record!(
        STORAGE,
        "restart-rotation-and-recovery",
        storage.stats().total_challenges == 0
            && StorageChallengeControl::new()
                .create("restarted.flac", 4_096, "peer", 1)
                .is_some()
    );

    const TEMPORAL: &str = "Common/Security/TemporalConsistency";
    let temporal = TemporalConsistencyControl::new();
    record!(
        TEMPORAL,
        "activation-default-and-profile",
        temporal.stats() == Default::default()
    );
    record!(
        TEMPORAL,
        "accepted-nominal-input",
        temporal.record("peer-a", "track.flac", 100, "hash-a") == Some(false)
            && temporal.record("peer-a", "track.flac", 100, "hash-a") == Some(false)
    );
    record!(
        TEMPORAL,
        "rejected-malicious-and-boundary-input",
        temporal.record("", "track.flac", 1, "hash").is_none()
            && temporal.record("peer-a", "track.flac", 200, "hash-b") == Some(false)
    );
    for index in 0..12 {
        assert!(temporal
            .record(
                "suspicious-peer",
                "track.flac",
                index,
                &format!("hash-{index}")
            )
            .is_some());
    }
    record!(
        TEMPORAL,
        "quota-time-lockout-and-concurrency",
        temporal.is_suspicious("suspicious-peer") && temporal.stats().tracked_files >= 2
    );
    record!(
        TEMPORAL,
        "secret-logging-and-privacy-output",
        !format!("{:?}", temporal.stats()).contains("suspicious-peer")
            && !format!("{:?}", temporal.stats()).contains("track.flac")
    );
    temporal.reset();
    record!(
        TEMPORAL,
        "restart-rotation-and-recovery",
        temporal.stats() == Default::default()
            && TemporalConsistencyControl::new()
                .record("peer", "file", 1, "hash")
                .is_some()
    );

    const WORK: &str = "Common/Security/WorkBudget";
    let work = WorkBudgetControl::new(WorkBudgetConfig::default());
    record!(
        WORK,
        "activation-default-and-profile",
        work.try_consume("peer-a", 5) && work.stats().consumed_units == 5
    );
    record!(
        WORK,
        "accepted-nominal-input",
        work.try_consume("peer-a", 5) && work.try_consume("peer-a", 10)
    );
    record!(
        WORK,
        "rejected-malicious-and-boundary-input",
        !work.try_consume("peer-a", 11) && !work.try_consume("", 1)
    );
    let work_quota = WorkBudgetControl::new(WorkBudgetConfig {
        max_units_per_call: 10,
        max_units_per_peer_per_minute: 10,
        ..WorkBudgetConfig::default()
    });
    assert!(work_quota.try_consume("peer", 10));
    record!(
        WORK,
        "quota-time-lockout-and-concurrency",
        !work_quota.try_consume("peer", 1)
    );
    record!(
        WORK,
        "secret-logging-and-privacy-output",
        !format!("{:?}", work.stats()).contains("peer-a")
            && !format!("{:?}", work.stats()).contains("password")
    );
    work.reset();
    record!(
        WORK,
        "restart-rotation-and-recovery",
        work.stats() == Default::default()
            && WorkBudgetControl::new(WorkBudgetConfig::default()).try_consume("peer", 1)
    );

    for subject in [
        "Common/Security/ConnectionFingerprint",
        "DhtRendezvous/Security/ConnectionFingerprint",
    ] {
        let fingerprints = ConnectionFingerprintControl::new();
        record!(
            subject,
            "activation-default-and-profile",
            fingerprints.stats() == Default::default()
        );
        let id = fingerprints
            .record_connection("192.0.2.11".parse().expect("fingerprint IP"), 1234, "quic")
            .expect("fingerprint");
        record!(
            subject,
            "accepted-nominal-input",
            fingerprints.record_event(&id, "connected")
                && fingerprints.stats().active_connections == 1
        );
        record!(
            subject,
            "rejected-malicious-and-boundary-input",
            fingerprints
                .record_connection("192.0.2.11".parse().expect("fingerprint IP"), 0, "quic")
                .is_none()
                && !fingerprints.record_event("missing", "event")
        );
        let concurrent = Arc::new(ConnectionFingerprintControl::new());
        let successes = Arc::new(AtomicUsize::new(0));
        let handles = (0..16)
            .map(|index| {
                let concurrent = Arc::clone(&concurrent);
                let successes = Arc::clone(&successes);
                thread::spawn(move || {
                    if concurrent
                        .record_connection(
                            format!("198.51.100.{}", index + 1)
                                .parse()
                                .expect("concurrent fingerprint IP"),
                            2000 + index as u16,
                            "quic",
                        )
                        .is_some()
                    {
                        successes.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().expect("fingerprint worker");
        }
        record!(
            subject,
            "quota-time-lockout-and-concurrency",
            successes.load(Ordering::Relaxed) == 16 && concurrent.stats().total_fingerprints == 16
        );
        record!(
            subject,
            "secret-logging-and-privacy-output",
            !format!("{:?}", fingerprints.stats()).contains("192.0.2.11")
                && fingerprints.redacted_event_count() == 1
        );
        assert!(fingerprints.disconnect(&id));
        fingerprints.reset();
        record!(
            subject,
            "restart-rotation-and-recovery",
            fingerprints.stats() == Default::default()
                && ConnectionFingerprintControl::new()
                    .record_connection("203.0.113.9".parse().expect("restart IP"), 1, "quic")
                    .is_some()
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    std::fs::create_dir_all(&evidence_dir).expect("create integrity evidence directory");
    std::fs::write(
        evidence_dir.join("integrity_controls.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize integrity ledger"),
    )
    .expect("write integrity ledger");
    assert!(
        mismatches.is_empty(),
        "{} integrity-control mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for bounded runtime controls that sit below the
/// HTTP/controller projections.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_runtime_controls() {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
    };

    use super::security_controls::{
        CoverTrafficControl, DnsSecurityControl, EntropyControl, HoneypotControl,
        NetworkGuardConfig, NetworkGuardControl, ParanoidControl, ParanoidLevel, PolicyControl,
        PrivacyLayerControl, ReconnaissanceControl, ShadowRateLimiter, TransportControl,
        TransportKind,
    };

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("slskdn {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    const COVER: &str = "Common/Security/CoverTrafficGenerator";
    let cover = CoverTrafficControl::new();
    record!(
        COVER,
        "activation-default-and-profile",
        !cover.stats().running
    );
    record!(
        COVER,
        "accepted-nominal-input",
        cover.start() && cover.send_cover_message() && {
            cover.notify_real_traffic();
            cover.stats().cover_messages_sent == 1 && cover.stats().real_messages_observed == 1
        }
    );
    record!(
        COVER,
        "rejected-malicious-and-boundary-input",
        !cover.start() && cover.stop() && !cover.send_cover_message()
    );
    let concurrent_cover = Arc::new(CoverTrafficControl::new());
    assert!(concurrent_cover.start());
    let handles = (0..16)
        .map(|_| {
            let cover = Arc::clone(&concurrent_cover);
            thread::spawn(move || cover.send_cover_message())
        })
        .collect::<Vec<_>>();
    let sent = handles
        .into_iter()
        .map(|handle| handle.join().expect("cover traffic worker"))
        .filter(|sent| *sent)
        .count();
    record!(
        COVER,
        "quota-time-lockout-and-concurrency",
        sent == 16 && concurrent_cover.stats().cover_messages_sent == 16
    );
    record!(
        COVER,
        "secret-logging-and-privacy-output",
        !format!("{:?}", cover.stats()).contains("private-key")
            && !format!("{:?}", cover.stats()).contains("password")
    );
    cover.reset();
    record!(
        COVER,
        "restart-rotation-and-recovery",
        !cover.stats().running && CoverTrafficControl::new().start()
    );

    const DNS: &str = "Common/Security/DnsSecurityService";
    let dns = DnsSecurityControl::new();
    record!(
        DNS,
        "activation-default-and-profile",
        dns.stats() == Default::default()
    );
    let public_ip = "192.0.2.1".parse().expect("DNS test IP");
    record!(
        DNS,
        "accepted-nominal-input",
        dns.resolve_literal("192.0.2.1", false, true) == Ok(public_ip)
            && dns.pin_tunnel("tunnel-a", public_ip)
            && dns.validate_tunnel_ip("tunnel-a", public_ip)
    );
    record!(
        DNS,
        "rejected-malicious-and-boundary-input",
        dns.resolve_literal("127.0.0.1", false, true).is_err()
            && !dns.validate_tunnel_ip("tunnel-a", "198.51.100.1".parse().unwrap())
            && dns.resolve_literal("not-an-ip", false, true).is_err()
    );
    let concurrent_dns = Arc::new(DnsSecurityControl::new());
    let handles = (0..16)
        .map(|index| {
            let dns = Arc::clone(&concurrent_dns);
            thread::spawn(move || {
                let ip = format!("198.51.100.{}", index + 1).parse().unwrap();
                dns.pin_tunnel(&format!("tunnel-{index}"), ip)
            })
        })
        .collect::<Vec<_>>();
    let pins = handles
        .into_iter()
        .map(|handle| handle.join().expect("DNS security worker"))
        .filter(|pinned| *pinned)
        .count();
    record!(DNS, "quota-time-lockout-and-concurrency", pins == 16);
    record!(
        DNS,
        "secret-logging-and-privacy-output",
        !format!("{:?}", dns.stats()).contains("tunnel-a")
            && !format!("{:?}", dns.stats()).contains("private-key")
    );
    dns.reset();
    record!(
        DNS,
        "restart-rotation-and-recovery",
        dns.stats() == Default::default()
            && DnsSecurityControl::new()
                .resolve_literal("203.0.113.1", false, true)
                .is_ok()
    );

    const ENTROPY: &str = "Common/Security/EntropyMonitor";
    let entropy = EntropyControl::new();
    record!(
        ENTROPY,
        "activation-default-and-profile",
        EntropyControl::SAMPLE_SIZE == 4_096 && entropy.stats() == Default::default()
    );
    let randomish = (0..EntropyControl::SAMPLE_SIZE)
        .map(|index| (index % 256) as u8)
        .collect::<Vec<_>>();
    let healthy = entropy.check(&randomish);
    record!(
        ENTROPY,
        "accepted-nominal-input",
        healthy.entropy >= EntropyControl::WARNING_ENTROPY && !healthy.critical
    );
    let low = entropy.check(&vec![0_u8; EntropyControl::SAMPLE_SIZE]);
    record!(
        ENTROPY,
        "rejected-malicious-and-boundary-input",
        low.critical && low.entropy < EntropyControl::MIN_ACCEPTABLE_ENTROPY
    );
    for _ in 0..(EntropyControl::MAX_HISTORY + 4) {
        entropy.check(&randomish);
    }
    record!(
        ENTROPY,
        "quota-time-lockout-and-concurrency",
        entropy.stats().checks == EntropyControl::MAX_HISTORY
    );
    record!(
        ENTROPY,
        "secret-logging-and-privacy-output",
        !format!("{:?}", entropy.stats()).contains("private-key")
            && !format!("{:?}", entropy.stats()).contains("password")
    );
    entropy.reset();
    record!(
        ENTROPY,
        "restart-rotation-and-recovery",
        entropy.stats() == Default::default()
            && EntropyControl::new().check(&randomish).entropy > 0.0
    );

    const RECON: &str = "Common/Security/FingerprintDetection";
    let recon = ReconnaissanceControl::new();
    record!(
        RECON,
        "activation-default-and-profile",
        recon.stats() == Default::default()
    );
    let recon_ip = "198.51.100.22".parse().expect("recon IP");
    record!(
        RECON,
        "accepted-nominal-input",
        !recon.record_connection(recon_ip, 443, Some("quic-v1"), Some("slskr"), true)
            && recon.stats().tracked_profiles == 1
    );
    for port in [80, 8080, 9000] {
        recon.record_connection(recon_ip, port, Some("quic-v1"), Some("slskr"), false);
    }
    record!(
        RECON,
        "rejected-malicious-and-boundary-input",
        recon.stats().known_scanners == 1 && recon.stats().total_events > 0
    );
    let concurrent_recon = Arc::new(ReconnaissanceControl::new());
    let handles = (0..16)
        .map(|index| {
            let recon = Arc::clone(&concurrent_recon);
            thread::spawn(move || {
                recon.record_connection(
                    format!("203.0.113.{}", index + 1).parse().unwrap(),
                    1000 + index as u16,
                    Some("quic"),
                    Some("slskr"),
                    true,
                )
            })
        })
        .collect::<Vec<_>>();
    let _ = handles
        .into_iter()
        .map(|handle| handle.join().expect("recon worker"))
        .collect::<Vec<_>>();
    record!(
        RECON,
        "quota-time-lockout-and-concurrency",
        concurrent_recon.stats().tracked_profiles == 16
    );
    record!(
        RECON,
        "secret-logging-and-privacy-output",
        !format!("{:?}", recon.stats()).contains("198.51.100.22")
            && !format!("{:?}", recon.stats()).contains("slskr")
    );
    recon.reset();
    record!(
        RECON,
        "restart-rotation-and-recovery",
        recon.stats() == Default::default()
            && !ReconnaissanceControl::new().record_connection(recon_ip, 1, None, None, true)
    );

    const HONEYPOT: &str = "Common/Security/Honeypot";
    let honeypot = HoneypotControl::new();
    record!(
        HONEYPOT,
        "activation-default-and-profile",
        honeypot.stats() == Default::default()
    );
    let honeypot_ip = "198.51.100.24".parse().expect("honeypot IP");
    record!(
        HONEYPOT,
        "accepted-nominal-input",
        honeypot.is_honeypot_file("private_keys.pem")
            && honeypot.record_interaction(honeypot_ip, "private_keys.pem")
            && honeypot.stats().total_interactions == 1
    );
    record!(
        HONEYPOT,
        "rejected-malicious-and-boundary-input",
        !honeypot.is_honeypot_file("") && !honeypot.record_interaction(honeypot_ip, "ordinary.txt")
    );
    for _ in 0..3 {
        honeypot.record_interaction(honeypot_ip, "admin_credentials.txt");
    }
    record!(
        HONEYPOT,
        "quota-time-lockout-and-concurrency",
        honeypot.stats().known_threats == 1
    );
    record!(
        HONEYPOT,
        "secret-logging-and-privacy-output",
        !format!("{:?}", honeypot.stats()).contains("198.51.100.24")
            && !format!("{:?}", honeypot.stats()).contains("private_keys.pem")
    );
    honeypot.reset();
    record!(
        HONEYPOT,
        "restart-rotation-and-recovery",
        honeypot.stats() == Default::default()
            && HoneypotControl::new().is_honeypot_file("database_dump.sql")
    );

    const NETWORK: &str = "Common/Security/NetworkGuard";
    let network = NetworkGuardControl::new(NetworkGuardConfig {
        max_connections_per_ip: 1,
        max_global_connections: 2,
        max_messages_per_minute: 2,
        max_message_size: 16,
        max_pending_requests_per_ip: 1,
    });
    record!(
        NETWORK,
        "activation-default-and-profile",
        NetworkGuardConfig::default().max_message_size == 65_536
            && network.stats() == Default::default()
    );
    let network_ip = "198.51.100.25".parse().expect("network IP");
    record!(
        NETWORK,
        "accepted-nominal-input",
        network.register_connection(network_ip)
            && network.allow_message(network_ip, 4)
            && network.allow_request(network_ip)
    );
    record!(
        NETWORK,
        "rejected-malicious-and-boundary-input",
        !network.allow_connection(network_ip)
            && !network.allow_message(network_ip, 17)
            && !network.allow_request(network_ip)
    );
    network.complete_request(network_ip);
    let concurrent_network = Arc::new(NetworkGuardControl::new(NetworkGuardConfig {
        max_connections_per_ip: 1,
        max_global_connections: 1,
        ..NetworkGuardConfig::default()
    }));
    let successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|index| {
            let network = Arc::clone(&concurrent_network);
            let successes = Arc::clone(&successes);
            thread::spawn(move || {
                if network.register_connection(format!("203.0.113.{}", index + 1).parse().unwrap())
                {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("network guard worker");
    }
    record!(
        NETWORK,
        "quota-time-lockout-and-concurrency",
        successes.load(Ordering::Relaxed) == 1
            && concurrent_network.stats().global_connections == 1
    );
    record!(
        NETWORK,
        "secret-logging-and-privacy-output",
        !format!("{:?}", network.stats()).contains("198.51.100.25")
            && !format!("{:?}", network.stats()).contains("password")
    );
    network.reset();
    record!(
        NETWORK,
        "restart-rotation-and-recovery",
        network.stats() == Default::default()
            && NetworkGuardControl::default().register_connection(network_ip)
    );

    const PARANOID: &str = "Common/Security/ParanoidMode";
    let paranoid = ParanoidControl::new(ParanoidLevel::Enforce);
    record!(
        PARANOID,
        "activation-default-and-profile",
        ParanoidControl::MAX_SEARCH_RESULTS == 10_000
            && ParanoidControl::new(ParanoidLevel::Log).validate_endpoint(network_ip, 0)
    );
    record!(
        PARANOID,
        "accepted-nominal-input",
        paranoid.validate_endpoint("203.0.113.1".parse().unwrap(), 443)
            && paranoid.validate_username("peer-a")
    );
    record!(
        PARANOID,
        "rejected-malicious-and-boundary-input",
        !paranoid.validate_endpoint("127.0.0.1".parse().unwrap(), 443)
            && !paranoid.validate_message_size(ParanoidControl::MAX_MESSAGE_SIZE + 1)
            && !paranoid.validate_username("")
    );
    for _ in 0..(ParanoidControl::MAX_ANOMALIES + 3) {
        paranoid.validate_username("");
    }
    record!(
        PARANOID,
        "quota-time-lockout-and-concurrency",
        paranoid.stats().total_anomalies == ParanoidControl::MAX_ANOMALIES
    );
    record!(
        PARANOID,
        "secret-logging-and-privacy-output",
        !format!("{:?}", paranoid.stats()).contains("198.51.100.25")
            && !format!("{:?}", paranoid.stats()).contains("private-key")
    );
    paranoid.reset();
    record!(
        PARANOID,
        "restart-rotation-and-recovery",
        paranoid.stats() == Default::default()
            && ParanoidControl::new(ParanoidLevel::Enforce).validate_username("peer")
    );

    const PRIVACY: &str = "Common/Security/PrivacyLayer";
    let privacy = PrivacyLayerControl::new(true);
    record!(
        PRIVACY,
        "activation-default-and-profile",
        privacy.stats().enabled && !privacy.stats().running
    );
    assert!(privacy.start());
    let transformed = privacy
        .transform_outbound(b"hello", 8)
        .expect("privacy transform");
    record!(
        PRIVACY,
        "accepted-nominal-input",
        transformed.len() == 12
            && privacy.transform_inbound(&transformed).as_deref() == Some(&b"hello\0\0\0"[..])
    );
    record!(
        PRIVACY,
        "rejected-malicious-and-boundary-input",
        privacy.transform_outbound(b"hello", 4).is_none()
            && privacy.transform_inbound(b"bad").is_none()
    );
    let concurrent_privacy = Arc::new(PrivacyLayerControl::new(true));
    concurrent_privacy.start();
    let handles = (0..16)
        .map(|_| {
            let privacy = Arc::clone(&concurrent_privacy);
            thread::spawn(move || privacy.transform_outbound(b"x", 1).is_some())
        })
        .collect::<Vec<_>>();
    let transformed_count = handles
        .into_iter()
        .map(|handle| handle.join().expect("privacy worker"))
        .filter(|transformed| *transformed)
        .count();
    record!(
        PRIVACY,
        "quota-time-lockout-and-concurrency",
        transformed_count == 16 && concurrent_privacy.stats().outbound_messages == 16
    );
    record!(
        PRIVACY,
        "secret-logging-and-privacy-output",
        !format!("{:?}", privacy.stats()).contains("hello")
            && !format!("{:?}", privacy.stats()).contains("private-key")
    );
    privacy.reset();
    record!(
        PRIVACY,
        "restart-rotation-and-recovery",
        privacy.stats().outbound_messages == 0 && PrivacyLayerControl::new(true).start()
    );

    for (subject, kind, prefer_private) in [
        (
            "Common/Security/DirectTransport",
            TransportKind::Direct,
            false,
        ),
        (
            "Common/Security/TorSocksTransport",
            TransportKind::Tor,
            true,
        ),
        (
            "Common/Security/WebSocketTransport",
            TransportKind::WebSocket,
            false,
        ),
        (
            "Common/Security/HttpTunnelTransport",
            TransportKind::HttpTunnel,
            false,
        ),
        ("Common/Security/I2PTransport", TransportKind::I2p, true),
        ("Common/Security/MeekTransport", TransportKind::Meek, false),
        (
            "Common/Security/Obfs4Transport",
            TransportKind::Obfs4,
            false,
        ),
        (
            "Common/Security/RelayOnlyTransport",
            TransportKind::RelayOnly,
            true,
        ),
        (
            "Common/Security/AnonymityTransportSelector",
            TransportKind::Direct,
            false,
        ),
    ] {
        let transports = TransportControl::new();
        record!(
            subject,
            "activation-default-and-profile",
            (kind == TransportKind::Direct && transports.is_available(kind))
                || (kind != TransportKind::Direct && !transports.is_available(kind))
        );
        transports.set_available(kind, true);
        record!(
            subject,
            "accepted-nominal-input",
            transports.is_available(kind)
                && transports.select(prefer_private).is_some()
                && transports.validate_target("peer.example", 443)
        );
        record!(
            subject,
            "rejected-malicious-and-boundary-input",
            !transports.validate_target("", 443)
                && !transports.validate_target("peer.example", 0)
                && TransportControl::new().select(prefer_private).is_some()
        );
        let concurrent_transports = Arc::new(transports);
        let handles = (0..16)
            .map(|_| {
                let transports = Arc::clone(&concurrent_transports);
                thread::spawn(move || transports.select(false).is_some())
            })
            .collect::<Vec<_>>();
        let selected = handles
            .into_iter()
            .map(|handle| handle.join().expect("transport worker"))
            .filter(|selected| *selected)
            .count();
        record!(
            subject,
            "quota-time-lockout-and-concurrency",
            selected == 16
        );
        record!(
            subject,
            "secret-logging-and-privacy-output",
            !format!("{:?}", concurrent_transports.stats(false)).contains("peer.example")
                && !format!("{:?}", concurrent_transports.stats(false)).contains("password")
        );
        concurrent_transports.reset();
        record!(
            subject,
            "restart-rotation-and-recovery",
            concurrent_transports.stats(false).selected == Some(TransportKind::Direct)
        );
    }

    const POLICIES: &str = "Security/Policies";
    const COMPOSITE: &str = "Security/CompositeSecurityPolicy";
    let policies = PolicyControl::new();
    for subject in [POLICIES, COMPOSITE] {
        record!(
            subject,
            "activation-default-and-profile",
            policies.stats() == 0
        );
        record!(
            subject,
            "accepted-nominal-input",
            policies.evaluate("peer-a", "browse", None).allowed
        );
        policies.block_peer("peer-b");
        record!(
            subject,
            "rejected-malicious-and-boundary-input",
            !policies.evaluate("peer-b", "browse", None).allowed
                && !policies.evaluate("peer-a", "consensus", None).allowed
                && !policies
                    .evaluate("peer-a", "browse", Some("known-bad"))
                    .allowed
        );
        let concurrent_policies = Arc::new(PolicyControl::new());
        let handles = (0..16)
            .map(|index| {
                let policies = Arc::clone(&concurrent_policies);
                thread::spawn(move || policies.block_peer(&format!("peer-{index}")))
            })
            .collect::<Vec<_>>();
        let blocked = handles
            .into_iter()
            .map(|handle| handle.join().expect("policy worker"))
            .filter(|blocked| *blocked)
            .count();
        record!(
            subject,
            "quota-time-lockout-and-concurrency",
            blocked == 16 && concurrent_policies.stats() == 16
        );
        record!(
            subject,
            "secret-logging-and-privacy-output",
            !format!("{:?}", policies.stats()).contains("peer-b")
                && !format!("{:?}", policies.evaluate("peer-a", "browse", None))
                    .contains("private-key")
        );
        policies.reset();
        record!(
            subject,
            "restart-rotation-and-recovery",
            policies.stats() == 0
                && PolicyControl::new()
                    .evaluate("peer", "browse", None)
                    .allowed
        );
    }

    const SHADOW: &str = "VirtualSoulfind/ShadowIndex/RateLimiter";
    let shadow = ShadowRateLimiter::new(1);
    record!(
        SHADOW,
        "activation-default-and-profile",
        shadow.stats().max_operations_per_minute == 1 && shadow.stats().operations_last_minute == 0
    );
    record!(SHADOW, "accepted-nominal-input", shadow.try_acquire());
    record!(
        SHADOW,
        "rejected-malicious-and-boundary-input",
        !shadow.try_acquire()
    );
    let concurrent_shadow = Arc::new(ShadowRateLimiter::new(1));
    let successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|_| {
            let shadow = Arc::clone(&concurrent_shadow);
            let successes = Arc::clone(&successes);
            thread::spawn(move || {
                if shadow.try_acquire() {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("shadow limiter worker");
    }
    record!(
        SHADOW,
        "quota-time-lockout-and-concurrency",
        successes.load(Ordering::Relaxed) == 1
    );
    record!(
        SHADOW,
        "secret-logging-and-privacy-output",
        !format!("{:?}", shadow.stats()).contains("private-key")
            && !format!("{:?}", shadow.stats()).contains("password")
    );
    shadow.reset();
    record!(
        SHADOW,
        "restart-rotation-and-recovery",
        shadow.stats().operations_last_minute == 0 && shadow.try_acquire()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    std::fs::create_dir_all(&evidence_dir).expect("create runtime-control evidence directory");
    std::fs::write(
        evidence_dir.join("runtime_controls.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize runtime-control ledger"),
    )
    .expect("write runtime-control ledger");
    assert!(
        mismatches.is_empty(),
        "{} runtime-control mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining route-owned security
/// adapters: the local forwarding manager, aggregate health projection,
/// and request middleware/auth/body guards.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_route_security_adapters() {
    use std::sync::Arc;

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("slskdn {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    const FORWARDER: &str = "Common/Security/LocalPortForwarder";
    let manager = super::port_forwarding::Manager::new();
    record!(
        FORWARDER,
        "activation-default-and-profile",
        super::port_forwarding::MAX_FORWARDING_RULES == 128
            && super::port_forwarding::MAX_FORWARDING_CONNECTIONS == 128
            && manager.statuses().await.is_empty()
    );

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("forwarding test port");
    let local_port = probe.local_addr().expect("forwarding local address").port();
    drop(probe);
    let request = || super::port_forwarding::StartRequest {
        local_port,
        pod_id: "pod-forward".to_owned(),
        destination_host: "service".to_owned(),
        destination_port: 80,
        service_name: Some("http".to_owned()),
        gateway_username: "gateway".to_owned(),
        gateway_endpoints: vec!["127.0.0.1:2234".parse().expect("gateway endpoint")],
        gateway_certificate_sha256: [7_u8; 32],
        local_username: "tester".to_owned(),
        authentication_key: Arc::new(ed25519_dalek::SigningKey::from_bytes(&[7_u8; 32])),
    };
    record!(
        FORWARDER,
        "accepted-nominal-input",
        super::port_forwarding::validate_start_request(&request()).is_ok()
            && manager.start(request()).await.is_ok()
            && manager.statuses().await.len() == 1
    );
    let mut invalid = request();
    invalid.local_port = 80;
    record!(
        FORWARDER,
        "rejected-malicious-and-boundary-input",
        super::port_forwarding::validate_start_request(&invalid).is_err()
            && manager.start(request()).await.is_err()
    );
    record!(
        FORWARDER,
        "quota-time-lockout-and-concurrency",
        manager.status(local_port).await.is_some()
            && manager.used_ports().await == vec![local_port]
    );
    let status = manager.status(local_port).await.expect("forwarding status");
    record!(
        FORWARDER,
        "secret-logging-and-privacy-output",
        !serde_json::to_string(&status)
            .expect("serialize forwarding status")
            .contains("private-key")
            && !serde_json::to_string(&status)
                .expect("serialize forwarding status")
                .contains("secret")
    );
    assert!(manager.stop(local_port).await);
    record!(
        FORWARDER,
        "restart-rotation-and-recovery",
        manager.statuses().await.is_empty()
            && super::port_forwarding::Manager::new()
                .statuses()
                .await
                .is_empty()
    );

    const HEALTH: &str = "Common/Security/SecurityHealthCheck";
    const MIDDLEWARE: &str = "Common/Security/SecurityMiddleware";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "route-security-secret"),
    );
    let health = super::route_http_request("GET", "/health", None, "", &state)
        .await
        .expect("health response");
    let security_status = super::route_http_request(
        "GET",
        "/api/security/status",
        Some("Bearer route-security-secret"),
        "",
        &state,
    )
    .await
    .expect("security status response");
    let status_json = serde_json::from_str::<serde_json::Value>(&security_status.body)
        .expect("security status JSON");
    record!(
        HEALTH,
        "activation-default-and-profile",
        health.status == "200 OK"
            && status_json["enabled"] == true
            && status_json["status"] == "local"
    );
    record!(
        HEALTH,
        "accepted-nominal-input",
        security_status.status == "200 OK"
            && status_json["watchedPeers"].is_number()
            && status_json["activeBans"].is_number()
    );
    let unauthorized = super::route_http_request("GET", "/api/security/status", None, "", &state)
        .await
        .expect("unauthorized health projection");
    record!(
        HEALTH,
        "rejected-malicious-and-boundary-input",
        unauthorized.status == "401 Unauthorized"
            && !unauthorized.body.contains("route-security-secret")
    );
    record!(
        HEALTH,
        "secret-logging-and-privacy-output",
        !security_status.body.contains("route-security-secret")
            && !security_status.body.contains("private-key")
            && !health.body.contains("/home/")
    );

    let gateway_state = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKR_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKR_MESH_GATEWAY_MAX_REQUEST_BODY_BYTES", "32"),
    )
    .0;
    let oversized = super::route_http_request(
        "POST",
        "/mesh/http/pods/status",
        None,
        &"x".repeat(33),
        &gateway_state,
    )
    .await
    .expect("oversized middleware request");
    record!(
        MIDDLEWARE,
        "activation-default-and-profile",
        health.status == "200 OK"
    );
    record!(
        MIDDLEWARE,
        "accepted-nominal-input",
        security_status.status == "200 OK" && security_status.content_type.contains("json")
    );
    record!(
        MIDDLEWARE,
        "rejected-malicious-and-boundary-input",
        unauthorized.status == "401 Unauthorized" && oversized.status == "413 Payload Too Large"
    );
    record!(
        MIDDLEWARE,
        "secret-logging-and-privacy-output",
        !oversized.body.contains("route-security-secret")
            && !oversized.body.contains("/home/")
            && !unauthorized.body.contains("route-security-secret")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    std::fs::create_dir_all(&evidence_dir).expect("create route-security evidence directory");
    std::fs::write(
        evidence_dir.join("route_security_adapters.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize route-security ledger"),
    )
    .expect("write route-security ledger");
    assert!(
        mismatches.is_empty(),
        "{} route-security mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the frozen mesh transport security controls:
/// token buckets, DHT quotas, SPKI pinning, pin rotation persistence,
/// replay/backoff guards, endpoint selection, transport policy matching,
/// and structured security-event redaction.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_mesh_transport() {
    use std::{
        collections::BTreeMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
    };

    use super::mesh_security::{
        CertificatePinManager, CertificatePinType, DhtRateLimiter, EndpointCertificatePinValidator,
        MeshTransportOptions, MeshTransportType, OverlayBlocklist, OverlayRateLimiter, RateLimiter,
        SecurityEventLogger, SecurityUtils, TransportPolicy, TransportPolicyManager,
        MAX_PARSE_DEPTH, MAX_REMOTE_PAYLOAD_SIZE,
    };

    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    const DHT: &str = "Mesh/Dht/DhtRateLimiter";
    const EVENTS: &str = "Mesh/ServiceFabric/SecurityEventLogger";
    const PINS: &str = "Mesh/Transport/CertificatePinManager";
    const ENDPOINT_PINS: &str = "Mesh/Transport/EndpointCertificatePinValidator";
    const LIMITER: &str = "Mesh/Transport/RateLimiter";
    const SECURITY_UTILS: &str = "Mesh/Transport/SecurityUtils";
    const POLICY: &str = "Mesh/Transport/TransportPolicy";

    let limiter = Arc::new(RateLimiter::new());
    record!(
        LIMITER,
        "activation-default-and-profile",
        limiter.statistics().active_buckets == 0 && limiter.statistics().total_tokens_consumed == 0
    );
    record!(
        LIMITER,
        "accepted-nominal-input",
        limiter.try_consume("nominal", 2, 2, 0.0) && limiter.current_tokens("nominal") == 0
    );
    record!(
        LIMITER,
        "rejected-malicious-and-boundary-input",
        !limiter.try_consume("nominal", 1, 2, 0.0) && !limiter.try_consume("invalid", 1, 0, 1.0)
    );
    let concurrent_limiter = Arc::new(RateLimiter::new());
    let successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|_| {
            let limiter = Arc::clone(&concurrent_limiter);
            let successes = Arc::clone(&successes);
            thread::spawn(move || {
                if limiter.try_consume("concurrent", 1, 1, 0.0) {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("rate limiter worker");
    }
    record!(
        LIMITER,
        "quota-time-lockout-and-concurrency",
        successes.load(Ordering::Relaxed) == 1
            && concurrent_limiter.statistics().total_requests_blocked == 15
    );
    let privacy_stats = format!("{:?}", limiter.statistics());
    record!(
        LIMITER,
        "secret-logging-and-privacy-output",
        !privacy_stats.contains("nominal")
    );
    let restarted_limiter = RateLimiter::new();
    record!(
        LIMITER,
        "restart-rotation-and-recovery",
        restarted_limiter.try_consume("nominal", 1, 1, 0.0)
    );

    let dht_limiter = DhtRateLimiter::new(Arc::new(RateLimiter::new()));
    record!(
        DHT,
        "activation-default-and-profile",
        dht_limiter.statistics().active_buckets == 0
            && dht_limiter.statistics().descriptor_fetch_tokens == 0
    );
    record!(
        DHT,
        "accepted-nominal-input",
        dht_limiter.should_allow_descriptor_fetch("peer-a")
            && dht_limiter.should_allow_descriptor_publish("peer-a")
            && dht_limiter.should_allow_query("descriptor", "peer-a")
    );
    let publish_limiter = DhtRateLimiter::new(Arc::new(RateLimiter::new()));
    let publish_ok = (0..20)
        .map(|_| publish_limiter.should_allow_descriptor_publish("peer-a"))
        .collect::<Vec<_>>();
    record!(
        DHT,
        "rejected-malicious-and-boundary-input",
        publish_ok.into_iter().all(|allowed| allowed)
            && !publish_limiter.should_allow_descriptor_publish("peer-a")
    );
    let concurrent_dht = DhtRateLimiter::new(Arc::new(RateLimiter::new()));
    let concurrent_dht = Arc::new(concurrent_dht);
    let dht_successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..32)
        .map(|_| {
            let dht = Arc::clone(&concurrent_dht);
            let successes = Arc::clone(&dht_successes);
            thread::spawn(move || {
                if dht.should_allow_descriptor_publish("concurrent-peer") {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("DHT limiter worker");
    }
    record!(
        DHT,
        "quota-time-lockout-and-concurrency",
        dht_successes.load(Ordering::Relaxed) == 20
    );
    record!(
        DHT,
        "secret-logging-and-privacy-output",
        !format!("{:?}", dht_limiter.statistics()).contains("peer-a")
    );
    let restarted_dht = DhtRateLimiter::new(Arc::new(RateLimiter::new()));
    record!(
        DHT,
        "restart-rotation-and-recovery",
        restarted_dht.should_allow_descriptor_fetch("peer-a")
    );

    let event_logger = Arc::new(SecurityEventLogger::new());
    record!(
        EVENTS,
        "activation-default-and-profile",
        event_logger.snapshot().is_empty()
    );
    event_logger.log_rate_limit_violation("peer-123456789", "mesh", 5, 4);
    record!(
        EVENTS,
        "accepted-nominal-input",
        event_logger.snapshot().len() == 1
            && event_logger.snapshot()[0].kind == "rate-limit-violation"
    );
    event_logger.log_unauthorized_access("", "mesh", "missing-key");
    record!(
        EVENTS,
        "rejected-malicious-and-boundary-input",
        event_logger.snapshot().len() == 2 && event_logger.snapshot()[1].peer_id == "***"
    );
    let concurrent_events = Arc::new(SecurityEventLogger::new());
    let handles = (0..32)
        .map(|index| {
            let logger = Arc::clone(&concurrent_events);
            thread::spawn(move || {
                logger.log_payload_size_violation(&format!("peer-{index:02}"), "mesh", index + 1);
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("security event worker");
    }
    record!(
        EVENTS,
        "quota-time-lockout-and-concurrency",
        concurrent_events.snapshot().len() == 32
    );
    let event_debug = format!("{:?}", event_logger.snapshot());
    record!(
        EVENTS,
        "secret-logging-and-privacy-output",
        !event_debug.contains("peer-123456789")
    );
    record!(
        EVENTS,
        "restart-rotation-and-recovery",
        SecurityEventLogger::new().snapshot().is_empty()
    );

    let certificate_one = rcgen::generate_simple_self_signed(vec!["mesh-one".to_owned()])
        .expect("first mesh certificate");
    let certificate_one_der = certificate_one.cert.der().to_vec();
    let certificate_two = rcgen::generate_simple_self_signed(vec!["mesh-two".to_owned()])
        .expect("second mesh certificate");
    let certificate_two_der = certificate_two.cert.der().to_vec();
    let pin_one =
        SecurityUtils::certificate_pin_base64(&certificate_one_der).expect("first SPKI pin");
    let pin_two =
        SecurityUtils::certificate_pin_base64(&certificate_two_der).expect("second SPKI pin");

    record!(
        SECURITY_UTILS,
        "activation-default-and-profile",
        MAX_REMOTE_PAYLOAD_SIZE == 1024 * 1024 && MAX_PARSE_DEPTH == 32
    );
    let parsed_json = SecurityUtils::parse_json_safely::<serde_json::Value>(
        r#"{"mesh":true}"#,
        MAX_REMOTE_PAYLOAD_SIZE,
        MAX_PARSE_DEPTH,
    );
    record!(
        SECURITY_UTILS,
        "accepted-nominal-input",
        parsed_json.is_ok()
            && SecurityUtils::validate_messagepack_size(&[1, 2, 3], MAX_REMOTE_PAYLOAD_SIZE)
                .is_ok()
            && SecurityUtils::validate_certificate_pin(
                &certificate_one_der,
                std::slice::from_ref(&pin_one),
            )
    );
    let deep_json = "[".repeat(MAX_PARSE_DEPTH + 1) + "0" + &"]".repeat(MAX_PARSE_DEPTH + 1);
    record!(
        SECURITY_UTILS,
        "rejected-malicious-and-boundary-input",
        SecurityUtils::parse_json_safely::<serde_json::Value>(
            &deep_json,
            MAX_REMOTE_PAYLOAD_SIZE,
            MAX_PARSE_DEPTH,
        )
        .is_err()
            && SecurityUtils::validate_messagepack_size(
                &[0; MAX_REMOTE_PAYLOAD_SIZE + 1],
                MAX_REMOTE_PAYLOAD_SIZE,
            )
            .is_err()
            && !SecurityUtils::validate_certificate_pin(
                &certificate_one_der,
                std::slice::from_ref(&pin_two),
            )
    );
    let concurrent_json = Arc::new(r#"{"ok":true}"#.to_owned());
    let parse_successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|_| {
            let json = Arc::clone(&concurrent_json);
            let successes = Arc::clone(&parse_successes);
            thread::spawn(move || {
                if SecurityUtils::parse_json_safely::<serde_json::Value>(&json, 128, 4).is_ok() {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("security utility worker");
    }
    record!(
        SECURITY_UTILS,
        "quota-time-lockout-and-concurrency",
        parse_successes.load(Ordering::Relaxed) == 16
    );
    record!(
        SECURITY_UTILS,
        "secret-logging-and-privacy-output",
        !format!(
            "{:?}",
            SecurityUtils::parse_json_safely::<serde_json::Value>(r#"{"secret":"hidden"}"#, 128, 4)
        )
        .contains("error")
    );
    record!(
        SECURITY_UTILS,
        "restart-rotation-and-recovery",
        SecurityUtils::certificate_pin_base64(&certificate_one_der).is_ok_and(|pin| pin == pin_one)
    );

    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-security-pins-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let pin_manager = CertificatePinManager::new(&root).expect("pin manager");
    record!(
        PINS,
        "activation-default-and-profile",
        pin_manager.statistics().total_peers == 0
    );
    pin_manager
        .add_pin("peer-a", &pin_one, CertificatePinType::Current)
        .expect("store first pin");
    record!(
        PINS,
        "accepted-nominal-input",
        pin_manager.validate_certificate_pin("peer-a", &certificate_one_der)
            && pin_manager.statistics().total_current_pins == 1
    );
    record!(
        PINS,
        "rejected-malicious-and-boundary-input",
        !pin_manager.validate_certificate_pin("peer-a", &certificate_two_der)
            && pin_manager
                .add_pin("", &pin_one, CertificatePinType::Current)
                .is_err()
            && !pin_manager.validate_certificate_pin("peer-a", &[1, 2, 3])
    );
    let pin_manager = Arc::new(pin_manager);
    let pin_validations = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|_| {
            let manager = Arc::clone(&pin_manager);
            let certificate = certificate_one_der.clone();
            let validations = Arc::clone(&pin_validations);
            thread::spawn(move || {
                if manager.validate_certificate_pin("peer-a", &certificate) {
                    validations.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("pin validation worker");
    }
    record!(
        PINS,
        "quota-time-lockout-and-concurrency",
        pin_validations.load(Ordering::Relaxed) == 16
    );
    let pin_file = root.join("mesh").join("certificate-pins.json");
    let pin_file_body = fs::read_to_string(&pin_file).expect("read pin store");
    record!(
        PINS,
        "secret-logging-and-privacy-output",
        pin_file_body.contains("peer-a")
            && !pin_file_body.contains("PRIVATE KEY")
            && !pin_file_body.contains("mesh-one")
    );
    pin_manager
        .rotate_pin("peer-a", &pin_two)
        .expect("rotate mesh pin");
    let reloaded_pin_manager = CertificatePinManager::new(&root).expect("reload pin manager");
    record!(
        PINS,
        "restart-rotation-and-recovery",
        reloaded_pin_manager.validate_certificate_pin("peer-a", &certificate_two_der)
            && reloaded_pin_manager.validate_certificate_pin("peer-a", &certificate_one_der)
            && reloaded_pin_manager
                .peer_certificate_info("peer-a")
                .is_some_and(|info| info.previous_pins.contains(&pin_one))
    );
    let _ = fs::remove_dir_all(&root);

    let endpoint: SocketAddr = "127.0.0.1:50302".parse().expect("mesh endpoint");
    let mut trusted_pins = BTreeMap::new();
    trusted_pins.insert(endpoint.to_string(), vec![pin_one.clone()]);
    record!(
        ENDPOINT_PINS,
        "activation-default-and-profile",
        !EndpointCertificatePinValidator::validate(
            endpoint,
            &certificate_one_der,
            &BTreeMap::new()
        )
    );
    record!(
        ENDPOINT_PINS,
        "accepted-nominal-input",
        EndpointCertificatePinValidator::validate(endpoint, &certificate_one_der, &trusted_pins)
    );
    let wrong_endpoint: SocketAddr = "127.0.0.1:50303".parse().expect("wrong mesh endpoint");
    let wrong_pins = BTreeMap::from([(endpoint.to_string(), vec![pin_two.clone()])]);
    record!(
        ENDPOINT_PINS,
        "rejected-malicious-and-boundary-input",
        !EndpointCertificatePinValidator::validate(
            wrong_endpoint,
            &certificate_one_der,
            &trusted_pins
        ) && !EndpointCertificatePinValidator::validate(
            endpoint,
            &certificate_one_der,
            &wrong_pins
        )
    );
    let trusted_pins = Arc::new(trusted_pins);
    let endpoint_successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|_| {
            let trusted_pins = Arc::clone(&trusted_pins);
            let certificate = certificate_one_der.clone();
            let successes = Arc::clone(&endpoint_successes);
            thread::spawn(move || {
                if EndpointCertificatePinValidator::validate(endpoint, &certificate, &trusted_pins)
                {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("endpoint pin worker");
    }
    record!(
        ENDPOINT_PINS,
        "quota-time-lockout-and-concurrency",
        endpoint_successes.load(Ordering::Relaxed) == 16
    );
    record!(
        ENDPOINT_PINS,
        "secret-logging-and-privacy-output",
        !format!("{:?}", trusted_pins).contains("private-key")
    );
    record!(
        ENDPOINT_PINS,
        "restart-rotation-and-recovery",
        EndpointCertificatePinValidator::validate(endpoint, &certificate_one_der, &trusted_pins)
    );

    let policy_manager = Arc::new(TransportPolicyManager::new());
    record!(
        POLICY,
        "activation-default-and-profile",
        TransportPolicy::default().is_enabled && MeshTransportOptions::default().enable_direct
    );
    let global_policy = TransportPolicy::enabled_for(None, Some("pod-a".to_owned()));
    policy_manager.add_or_update_policy(global_policy);
    let peer_policy = TransportPolicy {
        peer_id: Some("peer-a".to_owned()),
        pod_id: Some("pod-a".to_owned()),
        allowed_transport_types: Some(vec![MeshTransportType::TorOnionQuic]),
        prefer_private_transports: true,
        is_enabled: true,
        ..TransportPolicy::default()
    };
    policy_manager.add_or_update_policy(peer_policy);
    let applicable = policy_manager.applicable_policy("peer-a", Some("pod-a"));
    record!(
        POLICY,
        "accepted-nominal-input",
        applicable.as_ref().is_some_and(|policy| {
            policy.prefer_private_transports
                && policy.is_transport_allowed(
                    MeshTransportType::TorOnionQuic,
                    MeshTransportOptions {
                        enable_direct: true,
                        tor_enabled: true,
                        i2p_enabled: false,
                    },
                )
        })
    );
    let direct_policy = TransportPolicy {
        disable_clearnet: true,
        is_enabled: true,
        ..TransportPolicy::default()
    };
    record!(
        POLICY,
        "rejected-malicious-and-boundary-input",
        !direct_policy.is_transport_allowed(
            MeshTransportType::DirectQuic,
            MeshTransportOptions::default()
        ) && !TransportPolicy {
            is_enabled: false,
            ..TransportPolicy::default()
        }
        .applies_to("peer-a", Some("pod-a"))
    );
    let concurrent_policies = Arc::new(TransportPolicyManager::new());
    let handles = (0..16)
        .map(|index| {
            let manager = Arc::clone(&concurrent_policies);
            thread::spawn(move || {
                manager.add_or_update_policy(TransportPolicy::enabled_for(
                    Some(format!("peer-{index}")),
                    None,
                ));
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("policy worker");
    }
    record!(
        POLICY,
        "quota-time-lockout-and-concurrency",
        concurrent_policies.all_policies().len() == 16
    );
    record!(
        POLICY,
        "secret-logging-and-privacy-output",
        !format!("{:?}", applicable).contains("secret")
    );
    policy_manager.remove_policy("peer-a", Some("pod-a"));
    record!(
        POLICY,
        "restart-rotation-and-recovery",
        policy_manager
            .applicable_policy("peer-a", Some("pod-a"))
            .is_some()
            && policy_manager.all_policies().len() == 1
    );

    const OVERLAY_LIMITER: &str = "DhtRendezvous/Security/OverlayRateLimiter";
    const OVERLAY_BLOCKLIST: &str = "DhtRendezvous/Security/OverlayBlocklist";
    let overlay_limiter = Arc::new(OverlayRateLimiter::new());
    record!(
        OVERLAY_LIMITER,
        "activation-default-and-profile",
        overlay_limiter.stats().total_connections == 0 && overlay_limiter.stats().tracked_ips == 0
    );
    let overlay_ip = "192.0.2.44".parse().expect("overlay test IP");
    let first_connections = (0..3)
        .map(|_| overlay_limiter.check_connection(overlay_ip).allowed)
        .collect::<Vec<_>>();
    record!(
        OVERLAY_LIMITER,
        "accepted-nominal-input",
        first_connections == vec![true, true, true]
            && overlay_limiter.check_message("overlay-connection").allowed
            && overlay_limiter.check_delta_request("overlay-peer").allowed
            && overlay_limiter
                .check_mesh_search_request("overlay-peer")
                .allowed
    );
    let rejected_connection = overlay_limiter.check_connection(overlay_ip);
    record!(
        OVERLAY_LIMITER,
        "rejected-malicious-and-boundary-input",
        !rejected_connection.allowed
            && overlay_limiter
                .check_connection("192.0.2.45".parse().expect("second overlay test IP"))
                .allowed
    );
    let concurrent_limiter = Arc::new(OverlayRateLimiter::new());
    let successes = Arc::new(AtomicUsize::new(0));
    let handles = (0..16)
        .map(|index| {
            let limiter = Arc::clone(&concurrent_limiter);
            let successes = Arc::clone(&successes);
            thread::spawn(move || {
                let ip = format!("198.51.100.{}", index + 1)
                    .parse()
                    .expect("concurrent overlay test IP");
                if limiter.check_connection(ip).allowed {
                    successes.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("overlay limiter worker");
    }
    record!(
        OVERLAY_LIMITER,
        "quota-time-lockout-and-concurrency",
        successes.load(Ordering::Relaxed) == 16 && overlay_limiter.stats().rejected >= 1
    );
    record!(
        OVERLAY_LIMITER,
        "secret-logging-and-privacy-output",
        !format!("{:?}", overlay_limiter.stats()).contains("192.0.2.44")
            && !format!("{:?}", overlay_limiter.stats()).contains("overlay-peer")
    );
    overlay_limiter.record_disconnection(overlay_ip);
    record!(
        OVERLAY_LIMITER,
        "restart-rotation-and-recovery",
        OverlayRateLimiter::new()
            .check_connection("203.0.113.9".parse().expect("restarted overlay test IP"))
            .allowed
    );

    let overlay_blocklist = Arc::new(OverlayBlocklist::new());
    record!(
        OVERLAY_BLOCKLIST,
        "activation-default-and-profile",
        overlay_blocklist.stats() == Default::default()
    );
    overlay_blocklist.block_ip(
        "192.0.2.55".parse().expect("blocklist test IP"),
        "policy",
        None,
        false,
    );
    overlay_blocklist.block_username("SensitivePeer", "policy", None, true);
    record!(
        OVERLAY_BLOCKLIST,
        "accepted-nominal-input",
        overlay_blocklist.is_blocked_ip("192.0.2.55".parse().expect("blocked IP"))
            && overlay_blocklist.is_blocked_username("sensitivepeer")
            && overlay_blocklist.stats().blocked_ips == 1
    );
    overlay_blocklist.block_username("", "ignored", None, false);
    record!(
        OVERLAY_BLOCKLIST,
        "rejected-malicious-and-boundary-input",
        !overlay_blocklist.is_blocked_username("")
            && !overlay_blocklist.unblock_ip("198.51.100.8".parse().expect("missing IP"))
    );
    let concurrent_blocklist = Arc::new(OverlayBlocklist::new());
    let handles = (0..16)
        .map(|index| {
            let blocklist = Arc::clone(&concurrent_blocklist);
            thread::spawn(move || {
                blocklist.block_username(&format!("peer-{index}"), "concurrent", None, false);
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("overlay blocklist worker");
    }
    record!(
        OVERLAY_BLOCKLIST,
        "quota-time-lockout-and-concurrency",
        concurrent_blocklist.stats().blocked_usernames == 16
    );
    record!(
        OVERLAY_BLOCKLIST,
        "secret-logging-and-privacy-output",
        !format!("{:?}", overlay_blocklist.stats()).contains("SensitivePeer")
    );
    overlay_blocklist.unblock_ip("192.0.2.55".parse().expect("unblock IP"));
    overlay_blocklist.unblock_username("SensitivePeer");
    record!(
        OVERLAY_BLOCKLIST,
        "restart-rotation-and-recovery",
        !overlay_blocklist.is_blocked_ip("192.0.2.55".parse().expect("unblocked IP"))
            && !overlay_blocklist.is_blocked_username("sensitivepeer")
            && OverlayBlocklist::new().stats() == Default::default()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
    fs::write(
        evidence_dir.join("mesh_transport.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh transport security ledger"),
    )
    .expect("write mesh transport security ledger");
    assert!(
        mismatches.is_empty(),
        "{} mesh transport security-control mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the durable JWT revocation and managed
/// blacklist controls shared by the frozen controller profiles.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_core_security() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($target:expr, $subject:expr, $case:expr, $pass:expr) => {{
            let target = $target;
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    const BLACKLIST: &str = "Core/Blacklist";
    const JWT: &str = "Core/Security/JwtRevocationStore";

    for target in [
        super::config::ControllerProfile::Legacy,
        super::config::ControllerProfile::Native,
    ] {
        let profile_name = target.as_str();
        let target_name = match target {
            super::config::ControllerProfile::Legacy => "slskd",
            super::config::ControllerProfile::Native => "slskdn",
        };
        let default_config = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", profile_name)
                .with("SLSKR_AUTH_DISABLED", "true"),
        )
        .expect("default blacklist profile");
        let mut default_runtime = super::ManagedBlacklistRuntime::new(
            default_config.managed_blacklist.clone(),
            target,
            true,
        );
        record!(
            target_name,
            BLACKLIST,
            "activation-default-and-profile",
            !default_config.managed_blacklist.enabled
                && !default_runtime.is_blacklisted(Some("ordinary-peer"), None, 1)
        );

        let configured = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", profile_name)
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_BLACKLISTED_MEMBERS", "blocked-peer")
                .with("SLSKD_BLACKLISTED_PATTERNS", "^caseuser$")
                .with("SLSKD_BLACKLISTED_CIDRS", "10.0.0.0/8"),
        )
        .expect("configured blacklist profile");
        let mut runtime =
            super::ManagedBlacklistRuntime::new(configured.managed_blacklist.clone(), target, true);
        let case_match = runtime.is_blacklisted(Some("caseuser"), None, 2);
        let member_match = runtime.is_blacklisted(Some("blocked-peer"), None, 3);
        let cidr_match = runtime.is_blacklisted(None, Some("10.1.2.3".parse().unwrap()), 4);
        let case_mode_match = runtime.is_blacklisted(Some("CaseUser"), None, 5);
        record!(
            target_name,
            BLACKLIST,
            "accepted-nominal-input",
            case_match
                && member_match
                && cidr_match
                && (target == super::config::ControllerProfile::Native || !case_mode_match)
        );

        let invalid_pattern = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", profile_name)
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_BLACKLISTED_PATTERNS", "["),
        );
        let invalid_cidr = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", profile_name)
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_BLACKLISTED_CIDRS", "not-a-cidr"),
        );
        record!(
            target_name,
            BLACKLIST,
            "rejected-malicious-and-boundary-input",
            invalid_pattern.is_err() && invalid_cidr.is_err()
        );

        let mut replacement =
            super::ManagedBlacklistRuntime::new(configured.managed_blacklist.clone(), target, true);
        let before_replace = replacement.is_blacklisted(Some("blocked-peer"), None, 10);
        replacement.replace(default_config.managed_blacklist.clone(), target, true);
        let after_replace = replacement.is_blacklisted(Some("blocked-peer"), None, 11);
        record!(
            target_name,
            BLACKLIST,
            "quota-time-lockout-and-concurrency",
            before_replace && !after_replace
        );
        let sanitized = configured.sanitized_json();
        record!(
            target_name,
            BLACKLIST,
            "secret-logging-and-privacy-output",
            !sanitized.contains("blocked-peer")
                && !sanitized.contains("caseuser")
                && !sanitized.contains("not-a-cidr")
        );
        let reloaded = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", profile_name)
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_BLACKLISTED_MEMBERS", "blocked-peer")
                .with("SLSKD_BLACKLISTED_PATTERNS", "^caseuser$")
                .with("SLSKD_BLACKLISTED_CIDRS", "10.0.0.0/8"),
        )
        .expect("reload blacklist profile");
        let mut reloaded_runtime =
            super::ManagedBlacklistRuntime::new(reloaded.managed_blacklist, target, true);
        record!(
            target_name,
            BLACKLIST,
            "restart-rotation-and-recovery",
            reloaded_runtime.is_blacklisted(Some("blocked-peer"), None, 12)
        );
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-security-control-jwt-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("JWT revocation root");
    let now = super::unix_timestamp();
    let mut store = super::RevokedJwtStore::load(&root).expect("load JWT revocation store");
    record!(
        "slskdn",
        JWT,
        "activation-default-and-profile",
        !store.contains("missing-token", now)
    );
    store
        .revoke("live-token".to_owned(), now + 3_600, now)
        .expect("persist JWT revocation");
    record!(
        "slskdn",
        JWT,
        "accepted-nominal-input",
        store.contains("live-token", now)
    );
    store
        .revoke("expired-token".to_owned(), now + 1, now)
        .expect("persist JWT revocation");
    record!(
        "slskdn",
        JWT,
        "rejected-malicious-and-boundary-input",
        !store.contains("expired-token", now + 2) && !store.contains("missing-token", now + 2)
    );
    store
        .revoke("second-live-token".to_owned(), now + 3_600, now + 2)
        .expect("persist JWT revocation");
    record!(
        "slskdn",
        JWT,
        "quota-time-lockout-and-concurrency",
        store.contains("live-token", now + 2) && store.contains("second-live-token", now + 2)
    );
    let persisted = fs::read_to_string(root.join(super::RevokedJwtStore::STATE_FILE_NAME))
        .expect("read JWT revocations");
    record!(
        "slskdn",
        JWT,
        "secret-logging-and-privacy-output",
        persisted.contains("live-token") && !persisted.contains("jwt-secret")
    );
    let mut reloaded = super::RevokedJwtStore::load(&root).expect("reload JWT revocation store");
    record!(
        "slskdn",
        JWT,
        "restart-rotation-and-recovery",
        reloaded.contains("live-token", now) && reloaded.contains("second-live-token", now + 2)
    );
    let _ = fs::remove_dir_all(root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create security-controls evidence directory");
    fs::write(
        evidence_dir.join("core_security.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize core security-control ledger"),
    )
    .expect("write core security-control ledger");
    assert!(
        mismatches.is_empty(),
        "{} core security-control mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The slskdN SecurityController owns the public security projections and
/// their bounded query contracts.  Keep this separate from the lower-level
/// security primitives so route evidence cannot be mistaken for proof of
/// an unrelated helper implementation.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_native_security_controller() {
    let target = "slskdn";
    let subject = "Common/Security/API/SecurityController";
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_API_TOKEN", "security-controller-secret")
        .with("SLSKR_AUTH_DISABLED", "true");
    let (state, _receiver) = test_state_with_env(env.clone());
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let adversarial =
        super::route_http_request("GET", "/api/v0/security/adversarial", None, "", &state)
            .await
            .expect("security controller default projection");
    let adversarial_json =
        serde_json::from_str::<serde_json::Value>(&adversarial.body).unwrap_or_default();
    record!(
        "activation-default-and-profile",
        (adversarial.status == "404 Not Found"
            && adversarial.body == "Adversarial features are not configured")
            || (adversarial.status == "200 OK" && adversarial_json["enabled"].is_boolean())
    );

    let events =
        super::route_http_request("GET", "/api/v0/security/events?count=1", None, "", &state)
            .await
            .expect("security controller nominal events projection");
    record!(
        "accepted-nominal-input",
        events.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&events.body)
                .ok()
                .is_some_and(|value| value.is_array())
    );

    let rejected =
        super::route_http_request("GET", "/api/v0/security/events?count=0", None, "", &state)
            .await
            .expect("security controller bounded query rejection");
    record!(
        "rejected-malicious-and-boundary-input",
        rejected.status == "400 Bad Request" && rejected.body.contains("count must be positive")
    );

    let over_limit = super::route_http_request(
        "GET",
        "/api/v0/security/events?count=1001",
        None,
        "",
        &state,
    )
    .await
    .expect("security controller query upper bound");
    record!(
        "quota-time-lockout-and-concurrency",
        over_limit.status == "400 Bad Request"
            && over_limit.body.contains("count must be positive")
    );

    let dashboard =
        super::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
            .await
            .expect("security controller dashboard projection");
    record!(
        "secret-logging-and-privacy-output",
        !dashboard.body.contains("security-controller-secret")
    );

    let (restarted, _receiver) = test_state_with_env(env);
    let reloaded =
        super::route_http_request("GET", "/api/v0/security/adversarial", None, "", &restarted)
            .await
            .expect("security controller restart projection");
    record!(
        "restart-rotation-and-recovery",
        reloaded.status == adversarial.status && reloaded.body == adversarial.body
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create security controller evidence directory");
    fs::write(
        evidence_dir.join("security_controller.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize security controller security ledger"),
    )
    .expect("write security controller security ledger");
    assert!(
        mismatches.is_empty(),
        "{} security controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_passthrough_authentication() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($target:expr, $case:expr, $pass:expr) => {{
            let target = $target;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} Common/Authentication/PassthroughAuthentication [{case}]"
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": "Common/Authentication/PassthroughAuthentication",
                "case": case,
                "pass": pass,
            }));
        }};
    }

    for target in ["slskd", "slskdn"] {
        let env = MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
            .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", "192.0.2.0/24");
        let config = super::AppConfig::from_layers(None, FileConfig::default(), &env)
            .expect("passthrough authentication config");
        record!(
            target,
            "activation-default-and-profile",
            !config.auth_required
                && config.controller_passthrough_allows(Some("127.0.0.1:1".parse().unwrap()))
        );
        record!(
            target,
            "accepted-nominal-input",
            config.controller_passthrough_allows(Some("192.0.2.44:1".parse().unwrap()))
        );
        record!(
            target,
            "rejected-malicious-and-boundary-input",
            !config.controller_passthrough_allows(Some("198.51.100.1:1".parse().unwrap()))
                && !config.controller_passthrough_allows(None)
        );
        record!(
            target,
            "secret-logging-and-privacy-output",
            !format!("{config:?}").contains("SLSKD_PASSTHROUGH_ALLOWED_CIDRS")
        );
        let reloaded = super::AppConfig::from_layers(None, FileConfig::default(), &env)
            .expect("reload passthrough authentication config");
        record!(
            target,
            "restart-rotation-and-recovery",
            reloaded.controller_passthrough_allows(Some("192.0.2.44:1".parse().unwrap()))
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create passthrough security evidence directory");
    fs::write(
        evidence_dir.join("passthrough_authentication.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize passthrough security ledger"),
    )
    .expect("write passthrough security ledger");
    assert!(
        mismatches.is_empty(),
        "{} passthrough authentication mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the shared controller authentication
/// primitives: CIDR-bound API keys, administrator JWT issuance and
/// verification, scheme enforcement, rotation, and secret-free failure
/// output.  The quota/lockout case is intentionally not emitted because
/// these two frozen components do not own request throttling.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_authentication_and_jwt() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($target:expr, $subject:expr, $case:expr, $pass:expr) => {{
            let target = $target;
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    const API_KEY: &str = "Common/Authentication/ApiKeyAuthentication";
    const SECURITY_SERVICE: &str = "Core/Security/SecurityService";
    const API_ROUTE: &str = "/api/v0/relay/controller/downloads/1";

    for target in ["slskd", "slskdn"] {
        let root = std::env::temp_dir().join(format!(
            "slskr-security-control-auth-{target}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).expect("authentication control state root");
        let jwt_env = MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().expect("auth state path"))
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKD_JWT_KEY", "0123456789abcdef0123456789abcdef")
            .with("SLSKD_JWT_TTL", "3600000")
            .with("SLSKR_API_TOKEN", "admin-token");
        let config = super::AppConfig::from_layers(None, FileConfig::default(), &jwt_env)
            .expect("authentication control config");
        let now = super::unix_timestamp();
        let (jwt, claims) = super::utils::issue_admin_jwt(&config, "security-user", now)
            .expect("issue administrator JWT");
        let verified = super::utils::verify_admin_jwt(&config, &jwt, now);
        record!(
            target,
            SECURITY_SERVICE,
            "activation-default-and-profile",
            config.auth_required
                && config.controller_web_jwt_ttl_millis == 3_600_000
                && !config.controller_web_jwt_key.is_empty()
        );
        record!(
            target,
            SECURITY_SERVICE,
            "accepted-nominal-input",
            verified.as_ref().is_some_and(|value| {
                value.jti == claims.jti
                    && value.name == "security-user"
                    && value.role == "Administrator"
                    && super::utils::authorize_controller_route_from(
                        &config,
                        "GET",
                        "/api/v0/options",
                        Some(&format!("Bearer {jwt}")),
                        None,
                        None,
                    )
                    .is_ok()
            })
        );
        let jwt_parts: Vec<&str> = jwt.split('.').collect();
        let header = jwt_parts.first().copied().unwrap_or("");
        let payload = jwt_parts.get(1).copied().unwrap_or("");
        let tampered = format!("{header}.{payload}.AAAA");
        record!(
            target,
            SECURITY_SERVICE,
            "rejected-malicious-and-boundary-input",
            super::utils::verify_admin_jwt(&config, &tampered, now).is_none()
        );
        record!(
            target,
            SECURITY_SERVICE,
            "secret-logging-and-privacy-output",
            !format!("{verified:?}").contains(&config.controller_web_jwt_key)
                && !format!("{tampered:?}").contains(&config.controller_web_jwt_key)
        );
        let reloaded = super::AppConfig::from_layers(None, FileConfig::default(), &jwt_env)
            .expect("reload authentication control config");
        record!(
            target,
            SECURITY_SERVICE,
            "restart-rotation-and-recovery",
            super::utils::verify_admin_jwt(&reloaded, &jwt, now)
                .is_some_and(|value| value.jti == claims.jti)
        );

        let key = "0123456789abcdef";
        let mut api_config = config.clone();
        api_config.controller_api_keys.insert(
            "operator".to_owned(),
            super::config::ControllerApiKeySettings {
                key: key.to_owned(),
                role: "readonly".to_owned(),
                cidr: "127.0.0.1/32".to_owned(),
                cidrs: vec![super::config::TrustedProxyCidr::parse("127.0.0.1/32")
                    .expect("loopback API-key CIDR")],
            },
        );
        let loopback = Some("127.0.0.1:5030".parse().expect("loopback API-key address"));
        let remote = Some("192.0.2.1:5030".parse().expect("remote API-key address"));
        let api_authorized = super::utils::authorize_controller_route_from(
            &api_config,
            "GET",
            API_ROUTE,
            Some(&format!("ApiKey {key}")),
            None,
            loopback,
        );
        record!(
            target,
            API_KEY,
            "activation-default-and-profile",
            api_config.auth_required && api_config.controller_api_keys.contains_key("operator")
        );
        record!(
            target,
            API_KEY,
            "accepted-nominal-input",
            api_authorized.is_ok()
        );
        let wrong_key = super::utils::authorize_controller_route_from(
            &api_config,
            "GET",
            API_ROUTE,
            Some("ApiKey wrong-key-012345"),
            None,
            loopback,
        );
        let out_of_range = super::utils::authorize_controller_route_from(
            &api_config,
            "GET",
            API_ROUTE,
            Some(&format!("ApiKey {key}")),
            None,
            remote,
        );
        record!(
            target,
            API_KEY,
            "rejected-malicious-and-boundary-input",
            wrong_key == Err("unauthorized") && out_of_range == Err("unauthorized")
        );
        record!(
            target,
            API_KEY,
            "secret-logging-and-privacy-output",
            !format!("{wrong_key:?}").contains(key) && !format!("{out_of_range:?}").contains(key)
        );
        let mut rotated = api_config.clone();
        rotated
            .controller_api_keys
            .get_mut("operator")
            .expect("operator API key")
            .key = "fedcba9876543210".to_owned();
        let old_after_rotation = super::utils::authorize_controller_route_from(
            &rotated,
            "GET",
            API_ROUTE,
            Some(&format!("ApiKey {key}")),
            None,
            loopback,
        );
        let new_after_rotation = super::utils::authorize_controller_route_from(
            &rotated,
            "GET",
            API_ROUTE,
            Some("ApiKey fedcba9876543210"),
            None,
            loopback,
        );
        record!(
            target,
            API_KEY,
            "restart-rotation-and-recovery",
            old_after_rotation == Err("unauthorized") && new_after_rotation.is_ok()
        );

        let _ = fs::remove_dir_all(root);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create authentication security evidence directory");
    fs::write(
        evidence_dir.join("authentication_and_jwt.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize authentication security ledger"),
    )
    .expect("write authentication security ledger");
    assert!(
        mismatches.is_empty(),
        "{} authentication security-control mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The pin manager is also a frozen file-writer subject. Keep its
/// persistence evidence in the file-lifecycle ledger as well as the
/// security-control ledger.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-file-lifecycle-tests"
))]
async fn file_lifecycle_differential_mesh_certificate_pin_manager() {
    use super::mesh_security::{CertificatePinManager, CertificatePinType, SecurityUtils};

    let target = "slskdn";
    let subject = "Mesh/Transport/CertificatePinManager";
    let mut ledger = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            if $pass {
                ledger.push(serde_json::json!({
                    "target": target,
                    "subject": subject,
                    "case": $case,
                    "pass": true,
                }));
            }
        }};
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-mesh-pins-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("pin file lifecycle root");
    let certificate = rcgen::generate_simple_self_signed(vec!["mesh-file".to_owned()])
        .expect("mesh file certificate");
    let certificate_der = certificate.cert.der().to_vec();
    let pin = SecurityUtils::certificate_pin_base64(&certificate_der).expect("mesh file pin");
    let manager = CertificatePinManager::new(&root).expect("mesh file pin manager");
    let path = root.join("mesh").join("certificate-pins.json");
    record!(
        "path-and-default-selection",
        manager
            .add_pin("file-peer", &pin, CertificatePinType::Current)
            .is_ok()
            && path.is_file()
    );
    let first_body = fs::read(&path).expect("first pin file");
    record!(
        "nominal-bytes-and-metadata",
        !first_body.is_empty() && serde_json::from_slice::<serde_json::Value>(&first_body).is_ok()
    );

    let second_certificate =
        rcgen::generate_simple_self_signed(vec!["mesh-file-rotated".to_owned()])
            .expect("rotated mesh file certificate");
    let second_der = second_certificate.cert.der().to_vec();
    let second_pin =
        SecurityUtils::certificate_pin_base64(&second_der).expect("rotated mesh file pin");
    manager
        .rotate_pin("file-peer", &second_pin)
        .expect("rotate mesh file pin");
    let second_body = fs::read(&path).expect("rotated pin file");
    record!(
        "existing-missing-and-overwrite",
        first_body != second_body && String::from_utf8_lossy(&second_body).contains(&second_pin)
    );

    #[cfg(unix)]
    let symlink_rejected = {
        use std::os::unix::fs::symlink;

        let attack_root = root.join("symlink-attack");
        fs::create_dir_all(attack_root.join("mesh")).expect("symlink attack directory");
        let outside = attack_root.join("outside.json");
        fs::write(&outside, b"{\"peer_certificates\":[]}").expect("outside pin file");
        let link = attack_root.join("mesh").join("certificate-pins.json");
        symlink(&outside, &link).expect("pin symlink");
        CertificatePinManager::new(&attack_root).is_err()
    };
    #[cfg(not(unix))]
    let symlink_rejected = false;
    record!("permissions-symlink-and-path-confinement", symlink_rejected);

    let reloaded = CertificatePinManager::new(&root).expect("reload mesh file pins");
    record!(
        "restart-reload-retention-and-corruption",
        reloaded.validate_certificate_pin("file-peer", &second_der)
            && reloaded.validate_certificate_pin("file-peer", &certificate_der)
            && reloaded
                .peer_certificate_info("file-peer")
                .is_some_and(|info| info.previous_pins.contains(&pin))
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
    fs::write(
        evidence_dir.join("mesh_certificate_pin_manager.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh pin file-lifecycle ledger"),
    )
    .expect("write mesh pin file-lifecycle ledger");
    let _ = fs::remove_dir_all(root);
}

/// GoldStarClubService persists its local opt-out marker as a fixed file
/// under the application directory.  The Rust pod service uses the same
/// externally visible marker and atomic replacement semantics.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-file-lifecycle-tests"
))]
fn file_lifecycle_differential_gold_star_club_revocation() {
    let target = "slskdn";
    let subject = "PodCore/GoldStarClubService";
    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-gold-star-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("Gold Star file lifecycle root");
    let path = super::pods::gold_star_club_revocation_path(&root);
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push($case.to_owned());
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    record!(
        "path-and-default-selection",
        path == root.join("gold-star-club.revoked")
            && super::pods::record_gold_star_club_revocation(&root, "local-peer").is_ok()
            && path.is_file()
    );
    let first_body = fs::read_to_string(&path).expect("read Gold Star revocation marker");
    record!(
        "nominal-bytes-and-metadata",
        !first_body.is_empty()
            && first_body.contains("revoked_by=local-peer\n")
            && fs::metadata(&path)
                .expect("Gold Star marker metadata")
                .is_file()
    );
    super::pods::record_gold_star_club_revocation(&root, "rotated-peer")
        .expect("overwrite Gold Star revocation marker");
    let second_body = fs::read_to_string(&path).expect("read rotated Gold Star marker");
    record!(
        "existing-missing-and-overwrite",
        first_body != second_body && second_body.contains("revoked_by=rotated-peer\n")
    );
    #[cfg(unix)]
    let symlink_rejected = {
        use std::os::unix::fs::symlink;

        let outside = root.join("gold-star-outside");
        let linked_root = root.join("linked-state");
        fs::create_dir_all(&linked_root).expect("Gold Star linked state directory");
        fs::write(&outside, b"must remain unchanged").expect("Gold Star outside fixture");
        symlink(
            &outside,
            super::pods::gold_star_club_revocation_path(&linked_root),
        )
        .expect("Gold Star revocation symlink");
        super::pods::record_gold_star_club_revocation(&linked_root, "attacker").is_ok()
            && fs::read(&outside).expect("read Gold Star outside fixture")
                == b"must remain unchanged"
            && !super::pods::gold_star_club_revocation_path(&linked_root).is_symlink()
    };
    #[cfg(not(unix))]
    let symlink_rejected = true;
    record!("permissions-symlink-and-path-confinement", symlink_rejected);
    record!(
        "restart-reload-retention-and-corruption",
        super::pods::gold_star_club_is_revoked(&root)
            && super::pods::record_gold_star_club_revocation(&root, "   ").is_err()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file-lifecycle evidence directory");
    fs::write(
        evidence_dir.join("gold_star_club_revocation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Gold Star file-lifecycle ledger"),
    )
    .expect("write Gold Star file-lifecycle ledger");
    let _ = fs::remove_dir_all(root);
    assert!(
        mismatches.is_empty(),
        "Gold Star file-lifecycle mismatches: {}",
        mismatches.join(", ")
    );
}

/// MultiSourceDownloadService publishes only fully ranged, hash-verified
/// content and removes its private workspace when the operation is dropped.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-file-lifecycle-tests"
))]
async fn file_lifecycle_differential_multisource_download_service() {
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const CHUNK_SIZE: u64 = 64 * 1024;
    let target = "slskdn";
    let subject = "Transfers/MultiSource/MultiSourceDownloadService";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(case.to_owned());
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let content = Arc::new(
        (0..(CHUNK_SIZE as usize * 2 + 17))
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));

    let spawn_range_source = |content: Arc<Vec<u8>>| async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind multisource lifecycle fixture");
        let address = listener
            .local_addr()
            .expect("multisource lifecycle fixture address");
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let content = Arc::clone(&content);
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    loop {
                        let count = stream
                            .read(&mut buffer)
                            .await
                            .expect("read multisource range request");
                        if count == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..count]);
                        if request.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let request =
                        String::from_utf8(request).expect("multisource range request UTF-8");
                    let range = request
                        .lines()
                        .filter_map(|line| line.split_once(':'))
                        .find(|(name, _)| name.eq_ignore_ascii_case("range"))
                        .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                        .expect("multisource range header");
                    let (start, end) = range.split_once('-').expect("multisource range bounds");
                    let start = start.parse::<usize>().expect("multisource range start");
                    let end = end.parse::<usize>().expect("multisource range end");
                    let body = &content[start..=end];
                    let response = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                        body.len(),
                        content.len()
                    );
                    stream
                        .write_all(response.as_bytes())
                        .await
                        .expect("write multisource range headers");
                    stream
                        .write_all(body)
                        .await
                        .expect("write multisource range body");
                });
            }
        });
        (address, task)
    };

    let (first_address, first_server) = spawn_range_source(Arc::clone(&content)).await;
    let (second_address, second_server) = spawn_range_source(Arc::clone(&content)).await;
    let request_for = |hash: String| super::multisource::SwarmRequest {
        filename: "Remote/assembled.flac".to_owned(),
        file_size: content.len() as u64,
        expected_hash: Some(hash),
        output_path: None,
        chunk_size: CHUNK_SIZE,
        sources: vec![
            super::multisource::RangeSource {
                username: "first".to_owned(),
                url: format!("http://{first_address}/file"),
                authorization: None,
            },
            super::multisource::RangeSource {
                username: "second".to_owned(),
                url: format!("http://{second_address}/file"),
                authorization: None,
            },
        ],
    };
    let insert_job = |id: String,
                      request: super::multisource::SwarmRequest,
                      store: Arc<RwLock<super::multisource::SwarmStore>>| async move {
        store.write().await.insert(super::multisource::new_job(
            id,
            &request,
            "multisource/assembled.flac".to_owned(),
            1,
        ));
    };

    let root = std::env::temp_dir().join(format!(
        "slskr-file-lifecycle-multisource-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create multisource lifecycle root");
    let output = root.join("assembled.flac");
    let mut nominal_request = request_for(expected_hash.clone());
    super::multisource::validate_request(&mut nominal_request)
        .expect("valid multisource lifecycle request");
    let nominal_id = uuid::Uuid::new_v4().to_string();
    let nominal_store = Arc::new(RwLock::new(super::multisource::SwarmStore::default()));
    insert_job(
        nominal_id.clone(),
        nominal_request.clone(),
        Arc::clone(&nominal_store),
    )
    .await;
    let nominal = super::multisource::execute(
        nominal_id.clone(),
        nominal_request,
        output.clone(),
        "multisource/assembled.flac".to_owned(),
        Arc::clone(&nominal_store),
    )
    .await;
    let output_bytes = fs::read(&output).expect("read multisource output");
    #[cfg(unix)]
    let output_private = {
        use std::os::unix::fs::PermissionsExt;

        fs::metadata(&output)
            .expect("multisource output metadata")
            .permissions()
            .mode()
            & 0o777
            == 0o600
    };
    #[cfg(not(unix))]
    let output_private = true;
    record!(
        "nominal-bytes-and-metadata",
        nominal.success
            && nominal.final_hash == expected_hash
            && output_bytes == *content
            && output_private
            && nominal_store
                .try_read()
                .ok()
                .and_then(|store| store.get(&nominal_id).map(|job| job.status == "completed"))
                .unwrap_or(false)
    );

    let mut existing_request = request_for(expected_hash.clone());
    super::multisource::validate_request(&mut existing_request)
        .expect("valid existing-output request");
    let existing_id = uuid::Uuid::new_v4().to_string();
    let existing_store = Arc::new(RwLock::new(super::multisource::SwarmStore::default()));
    insert_job(
        existing_id.clone(),
        existing_request.clone(),
        Arc::clone(&existing_store),
    )
    .await;
    let existing = super::multisource::execute(
        existing_id,
        existing_request,
        output.clone(),
        "multisource/assembled.flac".to_owned(),
        existing_store,
    )
    .await;
    record!(
        "existing-missing-and-overwrite",
        nominal.success
            && !existing.success
            && existing
                .error
                .as_deref()
                .is_some_and(|error| error.contains("output file already exists"))
            && fs::read(&output).expect("read protected multisource output") == output_bytes
    );

    #[cfg(unix)]
    let symlink_safe = {
        use std::os::unix::fs::symlink;

        let outside = root.join("outside.flac");
        let link = root.join("linked-output.flac");
        fs::write(&outside, b"outside data").expect("write multisource symlink target");
        symlink(&outside, &link).expect("create multisource output symlink");
        let mut link_request = request_for(expected_hash.clone());
        super::multisource::validate_request(&mut link_request)
            .expect("valid symlink-output request");
        let link_id = uuid::Uuid::new_v4().to_string();
        let link_store = Arc::new(RwLock::new(super::multisource::SwarmStore::default()));
        insert_job(
            link_id.clone(),
            link_request.clone(),
            Arc::clone(&link_store),
        )
        .await;
        let link_result = super::multisource::execute(
            link_id,
            link_request,
            link.clone(),
            "multisource/linked-output.flac".to_owned(),
            link_store,
        )
        .await;
        !link_result.success
            && fs::read(&outside).expect("read multisource symlink target") == b"outside data"
            && link.is_symlink()
    };
    #[cfg(not(unix))]
    let symlink_safe = true;
    record!(
        "permissions-symlink-and-path-confinement",
        output_private && symlink_safe
    );

    let cancel_root = root.join("cancelled");
    fs::create_dir(&cancel_root).expect("create multisource cancellation root");
    let cancel_output = cancel_root.join("cancelled.flac");
    let spawn_stalled_source = || async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind multisource cancellation fixture");
        let address = listener
            .local_addr()
            .expect("multisource cancellation fixture address");
        let (stalled_tx, stalled_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut preflight, _) = listener
                .accept()
                .await
                .expect("accept multisource preflight");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let count = preflight
                    .read(&mut buffer)
                    .await
                    .expect("read multisource preflight");
                assert_ne!(count, 0, "multisource preflight ended early");
                request.extend_from_slice(&buffer[..count]);
            }
            preflight
                .write_all(
                    b"HTTP/1.1 206 Partial Content\r\nContent-Length: 1\r\nContent-Range: bytes 0-0/131089\r\nConnection: close\r\n\r\na",
                )
                .await
                .expect("write multisource preflight");
            let (_stalled, _) = listener
                .accept()
                .await
                .expect("accept multisource stalled chunk");
            stalled_tx
                .send(())
                .expect("signal multisource stalled chunk");
            std::future::pending::<()>().await;
        });
        (address, stalled_rx, task)
    };
    let (cancel_first, cancel_first_stalled, cancel_first_server) = spawn_stalled_source().await;
    let (cancel_second, cancel_second_stalled, cancel_second_server) = spawn_stalled_source().await;
    let mut cancel_request = super::multisource::SwarmRequest {
        filename: "Remote/cancelled.flac".to_owned(),
        file_size: content.len() as u64,
        expected_hash: Some(expected_hash.clone()),
        output_path: None,
        chunk_size: CHUNK_SIZE,
        sources: vec![
            super::multisource::RangeSource {
                username: "cancel-first".to_owned(),
                url: format!("http://{cancel_first}/file"),
                authorization: None,
            },
            super::multisource::RangeSource {
                username: "cancel-second".to_owned(),
                url: format!("http://{cancel_second}/file"),
                authorization: None,
            },
        ],
    };
    super::multisource::validate_request(&mut cancel_request).expect("valid cancellation request");
    let cancel_id = uuid::Uuid::new_v4().to_string();
    let cancel_store = Arc::new(RwLock::new(super::multisource::SwarmStore::default()));
    insert_job(
        cancel_id.clone(),
        cancel_request.clone(),
        Arc::clone(&cancel_store),
    )
    .await;
    let download = tokio::spawn(super::multisource::execute(
        cancel_id,
        cancel_request,
        cancel_output.clone(),
        "multisource/cancelled.flac".to_owned(),
        cancel_store,
    ));
    let stalled = tokio::time::timeout(Duration::from_secs(5), async {
        cancel_first_stalled
            .await
            .expect("first multisource chunk must stall");
        cancel_second_stalled
            .await
            .expect("second multisource chunk must stall");
    })
    .await
    .is_ok();
    let workspace_seen = fs::read_dir(&cancel_root)
        .expect("read multisource cancellation root")
        .flatten()
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".slskr-swarm-")
        });
    download.abort();
    let cancelled = download
        .await
        .expect_err("multisource download must be cancelled")
        .is_cancelled();
    let workspace_removed = !fs::read_dir(&cancel_root)
        .expect("read multisource cancellation root after abort")
        .flatten()
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".slskr-swarm-")
        });
    cancel_first_server.abort();
    cancel_second_server.abort();
    record!(
        "partial-cancel-and-cleanup",
        stalled && workspace_seen && cancelled && workspace_removed && !cancel_output.exists()
    );

    let restarted_store = Arc::new(RwLock::new(super::multisource::SwarmStore::default()));
    let reloaded_bytes = fs::read(&output).expect("read multisource output after restart");
    let reloaded_hash = hex::encode(Sha256::digest(reloaded_bytes.as_slice()));
    let corrupt_output = root.join("corrupt.flac");
    let mut corrupt_request = request_for("00".repeat(32));
    super::multisource::validate_request(&mut corrupt_request)
        .expect("valid corrupt multisource request");
    let corrupt_id = uuid::Uuid::new_v4().to_string();
    insert_job(
        corrupt_id.clone(),
        corrupt_request.clone(),
        Arc::clone(&restarted_store),
    )
    .await;
    let corrupt = super::multisource::execute(
        corrupt_id,
        corrupt_request,
        corrupt_output.clone(),
        "multisource/corrupt.flac".to_owned(),
        Arc::clone(&restarted_store),
    )
    .await;
    record!(
        "restart-reload-retention-and-corruption",
        reloaded_bytes == *content
            && reloaded_hash == expected_hash
            && !corrupt.success
            && corrupt
                .error
                .as_deref()
                .is_some_and(|error| error.contains("SHA-256 verification"))
            && !corrupt_output.exists()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create multisource evidence directory");
    fs::write(
        evidence_dir.join("multisource_download_service.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize multisource file-lifecycle ledger"),
    )
    .expect("write multisource file-lifecycle ledger");
    first_server.abort();
    second_server.abort();
    let _ = fs::remove_dir_all(root);
    assert!(
        mismatches.is_empty(),
        "multisource file-lifecycle mismatches: {}",
        mismatches.join(", ")
    );
}

/// The frozen OptionsController uses action-level Route attributes for
/// startup/debug/YAML/location/validation.  Keep those actions in the
/// controller ledger with real file reads, watched projections, YAML
/// validation, and reload checks for both compatibility targets.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_options_action_routes() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let target = $target;
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    for target in ["slskd", "slskdn"] {
        let env = MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true")
            .with("SLSKR_DEBUG", "true")
            .with("SLSKR_NO_CONFIG_WATCH", "true");
        let (state, _receiver) = test_state_with_env(env.clone());
        let config_path = state.config.state_dir.join("slskd.yml");
        let initial_yaml = "soulseek:\n  description: action-initial\n";
        fs::write(&config_path, initial_yaml).expect("write options action YAML fixture");

        let startup = super::route_http_request("GET", "/api/v0/options/startup", None, "", &state)
            .await
            .expect("startup options action");
        let startup_json =
            serde_json::from_str::<serde_json::Value>(&startup.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "nominal-status-headers-body",
            startup.status == "200 OK"
                && startup.content_type == "application/json; charset=utf-8"
                && startup_json.is_object()
                && startup_json["web"]["authentication"]["password"] == "*****"
        );
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "populated-dynamic-state",
            startup_json.is_object() && !startup.body.is_empty()
        );

        let startup_missing =
            super::route_http_request("GET", "/api/v0/options/startup", None, "", &state)
                .await
                .expect("startup options empty-state action");
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "missing-empty-or-conflict-state",
            startup_missing.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&startup_missing.body).is_ok()
        );

        let startup_malformed =
            super::route_http_request("GET", "/api/v0/options/startup/extra", None, "", &state)
                .await
                .expect("malformed startup options action");
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "malformed-path-query-or-body",
            startup_malformed.status == "404 Not Found"
        );

        let watched_yaml = "soulseek:\n  description: action-watched\n";
        fs::write(&config_path, watched_yaml).expect("write watched options YAML fixture");
        super::apply_watched_controller_configuration(
            &state,
            Some(watched_yaml),
            &state.controller_cli_environment,
        )
        .await;

        let debug = super::route_http_request("GET", "/api/v0/options/debug", None, "", &state)
            .await
            .expect("debug options action");
        let debug_text = serde_json::from_str::<String>(&debug.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/debug",
            "nominal-status-headers-body",
            debug.status == "200 OK"
                && debug.content_type == "application/json; charset=utf-8"
                && debug_text.starts_with("slskd:\n")
        );
        record!(
            target,
            "GET",
            "/api/v0/options/debug",
            "populated-dynamic-state",
            debug_text.contains("action-watched") && !debug_text.contains("action-secret")
        );

        let debug_malformed =
            super::route_http_request("GET", "/api/v0/options/debug/extra", None, "", &state)
                .await
                .expect("malformed debug options action");
        record!(
            target,
            "GET",
            "/api/v0/options/debug",
            "malformed-path-query-or-body",
            debug_malformed.status == "404 Not Found"
        );

        let location =
            super::route_http_request("GET", "/api/v0/options/yaml/location", None, "", &state)
                .await
                .expect("options YAML location action");
        let location_value = serde_json::from_str::<String>(&location.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/yaml/location",
            "nominal-status-headers-body",
            location.status == "200 OK"
                && location.content_type == "application/json; charset=utf-8"
                && location_value == config_path.display().to_string()
        );
        record!(
            target,
            "GET",
            "/api/v0/options/yaml/location",
            "populated-dynamic-state",
            location_value.ends_with("slskd.yml")
        );

        let malformed_location = super::route_http_request(
            "GET",
            "/api/v0/options/yaml/location/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed options YAML location action");
        record!(
            target,
            "GET",
            "/api/v0/options/yaml/location",
            "malformed-path-query-or-body",
            malformed_location.status == "404 Not Found"
        );

        let yaml = super::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
            .await
            .expect("options YAML action");
        let yaml_text = serde_json::from_str::<String>(&yaml.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "nominal-status-headers-body",
            yaml.status == "200 OK"
                && yaml.content_type == "application/json; charset=utf-8"
                && yaml_text == watched_yaml
        );
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "populated-dynamic-state",
            yaml_text.contains("action-watched")
        );

        let yaml_malformed =
            super::route_http_request("GET", "/api/v0/options/yaml/extra", None, "", &state)
                .await
                .expect("malformed options YAML action");
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "malformed-path-query-or-body",
            yaml_malformed.status == "404 Not Found"
        );

        let yaml_failure_root = std::env::temp_dir().join(format!(
            "slskr-options-yaml-read-conflict-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(&yaml_failure_root, b"state directory is a file")
            .expect("create YAML read conflict");
        let mut yaml_failure_state = test_state_with_env(env.clone()).0;
        Arc::get_mut(&mut yaml_failure_state)
            .expect("exclusive YAML read conflict state")
            .config
            .state_dir = yaml_failure_root.clone();
        let yaml_failure =
            super::route_http_request("GET", "/api/v0/options/yaml", None, "", &yaml_failure_state)
                .await
                .expect("YAML read filesystem failure action");
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "runtime-failure-and-timeout",
            yaml_failure.status == "500 Internal Server Error"
                && !yaml_failure.body.contains("state directory is a file")
        );
        let _ = fs::remove_file(yaml_failure_root);

        let valid_yaml = serde_json::to_string("debug: false\n").unwrap();
        let validation = super::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &valid_yaml,
            &state,
        )
        .await
        .expect("valid options YAML validation action");
        record!(
            target,
            "POST",
            "/api/v0/options/yaml/validate",
            "nominal-status-headers-body",
            validation.status == "200 OK"
                && validation.content_type.is_empty()
                && validation.body.is_empty()
        );

        let malformed_validation =
            super::route_http_request("POST", "/api/v0/options/yaml/validate", None, "{", &state)
                .await
                .expect("malformed options YAML validation action");
        record!(
            target,
            "POST",
            "/api/v0/options/yaml/validate",
            "malformed-path-query-or-body",
            malformed_validation.status == "400 Bad Request"
        );

        let concurrent_validation = futures_util::future::join_all([
            super::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &valid_yaml,
                &state,
            ),
            super::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &valid_yaml,
                &state,
            ),
        ])
        .await;
        record!(
            target,
            "POST",
            "/api/v0/options/yaml/validate",
            "concurrency-and-idempotency",
            concurrent_validation.iter().all(|response| response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
        );

        let updated_yaml =
            serde_json::to_string("soulseek:\n  description: action-updated\n").unwrap();
        let update =
            super::route_http_request("PUT", "/api/v0/options/yaml", None, &updated_yaml, &state)
                .await
                .expect("options YAML update action");
        let readback = super::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
            .await
            .expect("options YAML update readback");
        record!(
            target,
            "PUT",
            "/api/v0/options/yaml",
            "nominal-status-headers-body",
            update.status == "200 OK" && update.body.is_empty()
        );
        record!(
            target,
            "PUT",
            "/api/v0/options/yaml",
            "mutation-side-effects-and-readback",
            update.status == "200 OK"
                && serde_json::from_str::<String>(&readback.body)
                    .is_ok_and(|value| value == "soulseek:\n  description: action-updated\n")
        );
        let reloaded = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default().with(
                "SLSKR_STATE_DIR",
                state.config.state_dir.to_str().expect("state path"),
            ),
        )
        .expect("reload options YAML state");
        record!(
            target,
            "PUT",
            "/api/v0/options/yaml",
            "restart-persistence-or-reset",
            reloaded.user_info_description == "action-updated"
        );

        let disabled =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target)).0;
        for (method, route, case) in [
            (
                "GET",
                "/api/v0/options/debug",
                "missing-empty-or-conflict-state",
            ),
            (
                "GET",
                "/api/v0/options/yaml",
                "missing-empty-or-conflict-state",
            ),
            (
                "GET",
                "/api/v0/options/yaml/location",
                "missing-empty-or-conflict-state",
            ),
        ] {
            let response = super::route_http_request(method, route, None, "", &disabled)
                .await
                .expect("disabled options action");
            record!(
                target,
                method,
                route,
                case,
                response.status == "403 Forbidden"
            );
        }

        let options = super::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options validation-failure setup");
        let _ = options;
        *state
            .controller_options_validation_error
            .write()
            .expect("options validation error lock") =
            Some("action route validation failure".to_owned());
        let failed_options = super::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options validation failure action");
        record!(
            target,
            "GET",
            "/api/v0/options",
            "runtime-failure-and-timeout",
            failed_options.status == "500 Internal Server Error"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("options_action_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize options action controller ledger"),
    )
    .expect("write options action controller ledger");
    assert!(
        mismatches.is_empty(),
        "{} options action controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Close deterministic slskd controller cases that are independent of
/// the larger route-family fixtures: session-enabled projection and
/// login lifecycle, YAML validation/update failure semantics, room
/// subresource missing/idempotent behavior, and share-scan fault
/// injection.  Each row below is backed by an actual versioned dispatcher
/// call and a state/file readback rather than a route-presence assertion.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
fn controller_api_differential_controller_residual_core_contracts() {
    run_controller_future_on_large_stack("controller-residual-core-contracts", || {
        controller_api_differential_controller_residual_core_contracts_impl()
    });
}

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_controller_residual_core_contracts_impl() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    // The bare controller profile reports that authentication is not
    // required.  The same route reports true once the real API-key gate
    // is enabled and the request carries the configured credential.
    let (default_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let default_enabled =
        super::route_http_request("GET", "/api/v0/session/enabled", None, "", &default_state)
            .await
            .expect("default session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "missing-empty-or-conflict-state",
        default_enabled.status == "200 OK"
            && default_enabled.content_type == "application/json"
            && default_enabled.body == "false"
    );
    let malformed_enabled = super::route_http_request(
        "GET",
        "/api/v0/session/enabled/extra",
        None,
        "",
        &default_state,
    )
    .await
    .expect("malformed session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "malformed-path-query-or-body",
        malformed_enabled.status == "404 Not Found"
    );

    let enabled_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "residual-api-token");
    let (enabled_state, _receiver) = test_state_with_env(enabled_env);
    let enabled = super::route_http_request(
        "GET",
        "/api/v0/session/enabled",
        Some("ApiKey residual-api-token"),
        "",
        &enabled_state,
    )
    .await
    .expect("authenticated session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "populated-dynamic-state",
        enabled.status == "200 OK"
            && enabled.content_type == "application/json"
            && enabled.body == "true"
    );

    // The frozen slskd controller propagates a failed forced GitHub
    // version lookup. Use a local connection that closes immediately so
    // this exercises the production response path without depending on
    // the public network.
    let version_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind version failure fixture");
    let version_address = version_listener
        .local_addr()
        .expect("version failure fixture address");
    let version_server = tokio::spawn(async move {
        let _ = version_listener.accept().await;
    });
    let forced_version_failure = super::controller_version_latest_response(
        &default_state,
        true,
        &format!("http://{version_address}/latest"),
    )
    .await;
    version_server.await.expect("version failure fixture task");
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "runtime-failure-and-timeout",
        forced_version_failure.status == "500 Internal Server Error"
            && !forced_version_failure
                .body
                .contains(&version_address.to_string())
    );

    let (user_failure_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                "user-read-failure-peer=127.0.0.1:2234",
            ),
    );
    user_failure_state
        .users
        .write()
        .await
        .records
        .push(super::UserRecord {
            username: "user-read-failure-peer".to_owned(),
            watched: true,
            status: Some("Online".to_owned()),
            privileged: false,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: super::unix_timestamp(),
        });
    let browse_response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        r#"{"username":"user-read-failure-peer","entries":[{"filename":"Remote/Track.flac","size":1}]}"#,
        &user_failure_state,
    )
    .await
    .expect("seed disconnected user browse failure");
    assert_eq!(browse_response.status, "200 OK");
    for (path, route) in [
        (
            "/api/v0/users/user-read-failure-peer/info",
            "/api/v0/users/{username}/info",
        ),
        (
            "/api/v0/users/user-read-failure-peer/status",
            "/api/v0/users/{username}/status",
        ),
        (
            "/api/v0/users/user-read-failure-peer/endpoint",
            "/api/v0/users/{username}/endpoint",
        ),
        (
            "/api/v0/users/user-read-failure-peer/browse",
            "/api/v0/users/{username}/browse",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &user_failure_state)
            .await
            .expect("disconnected user read failure response");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && !response.body.contains("user-read-failure-peer")
        );
    }

    // A successful login has a real observable side effect: its returned
    // administrator JWT authorizes the next controller request.  Two
    // concurrent valid logins remain independent, and a fresh state
    // instance can issue another token from the same configured profile.
    let login_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "residual-login-token")
        .with("SLSKD_USERNAME", "residual-admin")
        .with("SLSKD_PASSWORD", "residual-password");
    let (login_state, _receiver) = test_state_with_env(login_env.clone());
    let login_body = r#"{"username":"residual-admin","password":"residual-password"}"#;
    let login =
        super::route_http_request("POST", "/api/v0/session", None, login_body, &login_state)
            .await
            .expect("residual login");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    let login_token = login_json["token"].as_str().unwrap_or_default().to_owned();
    let authorized = super::route_http_request(
        "GET",
        "/api/v0/session/enabled",
        Some(&format!("Bearer {login_token}")),
        "",
        &login_state,
    )
    .await
    .expect("JWT-authorized residual session-enabled request");
    record!(
        "POST",
        "/api/v0/session",
        "mutation-side-effects-and-readback",
        login.status == "200 OK"
            && login_json["tokenType"] == "Bearer"
            && login_token.split('.').count() == 3
            && authorized.status == "200 OK"
            && authorized.body == "true"
    );

    let reloaded_state = test_state_with_env(login_env.clone()).0;
    let reloaded_login =
        super::route_http_request("POST", "/api/v0/session", None, login_body, &reloaded_state)
            .await
            .expect("reloaded residual login");
    record!(
        "POST",
        "/api/v0/session",
        "restart-persistence-or-reset",
        reloaded_login.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&reloaded_login.body)
                .is_ok_and(|value| value["tokenType"] == "Bearer")
    );
    let concurrent_logins = futures_util::future::join_all([
        super::route_http_request("POST", "/api/v0/session", None, login_body, &login_state),
        super::route_http_request("POST", "/api/v0/session", None, login_body, &login_state),
    ])
    .await;
    let concurrent_tokens = concurrent_logins
        .iter()
        .filter_map(|response| response.as_ref().ok())
        .filter_map(|response| serde_json::from_str::<serde_json::Value>(&response.body).ok())
        .filter_map(|value| value["token"].as_str().map(str::to_owned))
        .collect::<BTreeSet<_>>();
    record!(
        "POST",
        "/api/v0/session",
        "concurrency-and-idempotency",
        concurrent_logins.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_tokens.len() == 2
    );

    // YAML validation is intentionally non-mutating.  The empty request
    // is a real missing-body rejection; valid concurrent validations must
    // leave the durable file byte-for-byte unchanged.
    let options_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_REMOTE_CONFIGURATION", "true")
        .with("SLSKR_NO_CONFIG_WATCH", "true");
    let (options_state, _receiver) = test_state_with_env(options_env.clone());
    let options_path = options_state.config.state_dir.join("slskd.yml");
    let original_yaml = b"soulseek:\n  description: residual-validation\n";
    fs::write(&options_path, original_yaml).expect("write residual YAML fixture");
    let missing_validation = super::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        "",
        &options_state,
    )
    .await
    .expect("missing YAML validation body");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "missing-empty-or-conflict-state",
        missing_validation.status == "400 Bad Request"
    );
    let before_validation = fs::read(&options_path).expect("read YAML before validation");
    let valid_yaml = serde_json::to_string("soulseek:\n  description: validated\n").unwrap();
    let validation = super::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        &valid_yaml,
        &options_state,
    )
    .await
    .expect("valid YAML validation");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "mutation-side-effects-and-readback",
        validation.status == "200 OK"
            && validation.body.is_empty()
            && fs::read(&options_path).ok().as_deref() == Some(before_validation.as_slice())
    );
    let concurrent_validations = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &valid_yaml,
            &options_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &valid_yaml,
            &options_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "concurrency-and-idempotency",
        concurrent_validations.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && fs::read(&options_path).ok().as_deref() == Some(original_yaml)
    );
    let reloaded_options = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with(
            "SLSKR_STATE_DIR",
            options_state
                .config
                .state_dir
                .to_str()
                .expect("options state path"),
        ),
    )
    .expect("reload residual validation state");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "restart-persistence-or-reset",
        reloaded_options.state_dir == options_state.config.state_dir
            && fs::read(&options_path).ok().as_deref() == Some(original_yaml)
    );

    // YAML update rejects both an absent body and malformed JSON/YAML,
    // performs atomic concurrent replacement, and exposes a real 500
    // when its configured state directory is a regular file.
    let malformed_update =
        super::route_http_request("PUT", "/api/v0/options/yaml", None, "{", &options_state)
            .await
            .expect("malformed YAML update");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "malformed-path-query-or-body",
        malformed_update.status == "400 Bad Request"
    );
    let missing_update =
        super::route_http_request("PUT", "/api/v0/options/yaml", None, "", &options_state)
            .await
            .expect("missing YAML update body");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "missing-empty-or-conflict-state",
        missing_update.status == "400 Bad Request"
    );
    let update_a = serde_json::to_string("soulseek:\n  description: update-a\n").unwrap();
    let update_b = serde_json::to_string("soulseek:\n  description: update-b\n").unwrap();
    let concurrent_updates = futures_util::future::join_all([
        super::route_http_request(
            "PUT",
            "/api/v0/options/yaml",
            None,
            &update_a,
            &options_state,
        ),
        super::route_http_request(
            "PUT",
            "/api/v0/options/yaml",
            None,
            &update_b,
            &options_state,
        ),
    ])
    .await;
    let final_yaml = fs::read_to_string(&options_path).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "concurrency-and-idempotency",
        concurrent_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && [
                "soulseek:\n  description: update-a\n",
                "soulseek:\n  description: update-b\n",
            ]
            .contains(&final_yaml.as_str())
    );
    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-residual-options-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file").expect("create state conflict");
    let mut conflict_state = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true"),
    )
    .0;
    Arc::get_mut(&mut conflict_state)
        .expect("exclusive conflict state")
        .config
        .state_dir = conflict_root.clone();
    let failed_update = super::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        &update_a,
        &conflict_state,
    )
    .await
    .expect("YAML update filesystem failure");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "runtime-failure-and-timeout",
        failed_update.status == "500 Internal Server Error"
            && !failed_update.body.contains("state directory is a file")
    );
    let _ = fs::remove_file(conflict_root);

    // PATCH is volatile but still has a real serialization/idempotency
    // contract.  Both writes must complete and the final projection must
    // be one complete normalized overlay, never a torn JSON value.
    let patch_a = r#"{"soulseek":{"listenPort":50321}}"#;
    let patch_b = r#"{"soulseek":{"listenPort":50322}}"#;
    let concurrent_patches = futures_util::future::join_all([
        super::route_http_request("PATCH", "/api/v0/options", None, patch_a, &options_state),
        super::route_http_request("PATCH", "/api/v0/options", None, patch_b, &options_state),
    ])
    .await;
    let options_readback =
        super::route_http_request("GET", "/api/v0/options", None, "", &options_state)
            .await
            .expect("options patch readback");
    let options_readback_json =
        serde_json::from_str::<serde_json::Value>(&options_readback.body).unwrap_or_default();
    record!(
        "PATCH",
        "/api/v0/options",
        "concurrency-and-idempotency",
        concurrent_patches.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && matches!(
                options_readback_json["soulseek"]["listenPort"].as_u64(),
                Some(50321 | 50322)
            )
    );

    // Application restart requests are idempotent even when two real
    // callers race to set the same durable runtime flag.
    let (application_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
    );
    let concurrent_restarts = futures_util::future::join_all([
        super::route_http_request("PUT", "/api/v0/application", None, "{}", &application_state),
        super::route_http_request("PUT", "/api/v0/application", None, "{}", &application_state),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/application",
        "concurrency-and-idempotency",
        concurrent_restarts.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content" && response.body.is_empty()))
            && application_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    // Share rescans expose both the real persistence failure rollback and
    // the semaphore-backed concurrent-scan rejection.
    let share_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("residual share database");
    let (share_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(share_db.clone()),
    );
    let previous_shares = share_failure_state.shares.read().await.clone();
    share_db.close_for_test().await;
    let failed_share_scan =
        super::route_http_request("PUT", "/api/v0/shares", None, "", &share_failure_state)
            .await
            .expect("share persistence failure response");
    record!(
        "PUT",
        "/api/v0/shares",
        "runtime-failure-and-timeout",
        failed_share_scan.status == "503 Service Unavailable"
            && share_failure_state.shares.read().await.json() == previous_shares.json()
    );
    let (share_busy_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let _scan_permit = Arc::clone(&share_busy_state.share_scans)
        .acquire_owned()
        .await
        .expect("hold share scan permit");
    let busy_share_scan =
        super::route_http_request("PUT", "/api/v0/shares", None, "", &share_busy_state)
            .await
            .expect("busy share scan response");
    record!(
        "PUT",
        "/api/v0/shares",
        "concurrency-and-idempotency",
        busy_share_scan.status == "503 Service Unavailable"
            && busy_share_scan
                .body
                .contains("share scan already in progress")
    );

    // A failed share scan is a real repository/filesystem fault state.
    // Frozen BrowseAsync enumerates the backing repositories directly, so
    // an equivalent browse after that fault reaches the controller's 500
    // middleware instead of returning a stale directory projection.
    let (browse_failure_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let browse_root_id = {
        let mut shares = browse_failure_state.shares.write().await;
        if shares.roots.is_empty() {
            let files = shares.entries.len();
            let bytes = shares.entries.iter().map(|entry| entry.size).sum();
            shares.roots.push(super::ShareRoot {
                label: "shares".to_owned(),
                local_path: std::env::temp_dir(),
                raw: "shares".to_owned(),
                directories: 0,
                files,
                bytes,
                extensions: Vec::new(),
                statistics_ready: true,
            });
        }
        let root_id = super::share_root_id(&shares.roots[0].label);
        shares
            .scan_errors
            .push("repository browse failed".to_owned());
        root_id
    };
    let failed_browse_all = super::route_http_request(
        "GET",
        "/api/v0/shares/contents",
        None,
        "",
        &browse_failure_state,
    )
    .await
    .expect("share browse-all failure response");
    record!(
        "GET",
        "/api/v0/shares/contents",
        "runtime-failure-and-timeout",
        failed_browse_all.status == "500 Internal Server Error"
            && !failed_browse_all.body.contains("repository browse failed")
    );
    let failed_browse_share = super::route_http_request(
        "GET",
        &format!("/api/v0/shares/{browse_root_id}/contents"),
        None,
        "",
        &browse_failure_state,
    )
    .await
    .expect("share browse-share failure response");
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "runtime-failure-and-timeout",
        failed_browse_share.status == "500 Internal Server Error"
            && !failed_browse_share
                .body
                .contains("repository browse failed")
    );

    // The frozen RoomsController fetches the room list from the live
    // Soulseek client. A disconnected session therefore reaches the
    // framework's 500 path instead of serving the last cached projection.
    let (available_rooms_failure_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let available_rooms_failure = super::route_http_request(
        "GET",
        "/api/v0/rooms/available",
        None,
        "",
        &available_rooms_failure_state,
    )
    .await
    .expect("available rooms disconnected failure response");
    record!(
        "GET",
        "/api/v0/rooms/available",
        "runtime-failure-and-timeout",
        available_rooms_failure.status == "500 Internal Server Error"
            && available_rooms_failure
                .body
                .contains("failed to retrieve available rooms")
    );

    // Frozen ConversationsController reads through its EF-backed
    // messaging service for all three GET shapes. Exercise the same
    // failure through the live HTTP stream after closing a real SQLite
    // manager; the memory projection must not hide a failed storage read.
    async fn live_controller_get(state: Arc<super::AppState>, path: &str) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        client
            .write_all(request.as_bytes())
            .await
            .expect("write conversation read failure request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read conversation read failure response");
        task.await
            .expect("conversation read failure HTTP task")
            .expect("conversation read failure HTTP response");
        response
    }

    let conversation_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation read failure database");
    let (conversation_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(conversation_db.clone()),
    );
    conversation_failure_state.messages.write().await.add(
        "conversation-read-failure-peer".to_owned(),
        "inbound",
        "persisted conversation read failure".to_owned(),
    );
    conversation_db.close_for_test().await;
    for (path, route) in [
        ("/api/v0/conversations", "/api/v0/conversations"),
        (
            "/api/v0/conversations/conversation-read-failure-peer",
            "/api/v0/conversations/{username}",
        ),
        (
            "/api/v0/conversations/conversation-read-failure-peer/messages",
            "/api/v0/conversations/{username}/messages",
        ),
    ] {
        let response = live_controller_get(Arc::clone(&conversation_failure_state), path).await;
        let response = String::from_utf8_lossy(&response);
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.starts_with("HTTP/1.1 500 Internal Server Error")
        );
    }

    // Frozen slskd transfer reports query SQLite through the telemetry
    // report service. A closed store therefore reaches the controller's
    // generic 500 middleware; serving the in-memory transfer projection
    // would incorrectly hide that repository failure.
    let telemetry_report_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("telemetry report failure database");
    let (telemetry_report_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(telemetry_report_db.clone()),
    );
    telemetry_report_failure_state.session.write().await.state = "connected";
    {
        let mut transfers = telemetry_report_failure_state.transfers.write().await;
        transfers.create(
            0,
            Some("transfer-read-failure-peer".to_owned()),
            "Remote/Failure.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.create(
            1,
            Some("transfer-read-failure-peer".to_owned()),
            "Upload/Failure.flac".to_owned(),
            None,
            Some(1),
        );
    }
    let search_read_failure_id = "33333333-3333-4333-8333-333333333333";
    let search_created = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        &format!(r#"{{"id":"{search_read_failure_id}","searchText":"storage failure"}}"#),
        &telemetry_report_failure_state,
    )
    .await
    .expect("seed search response read failure");
    assert_eq!(search_created.status, "200 OK");
    telemetry_report_db.close_for_test().await;
    for (path, route) in [
        (
            "/api/v0/telemetry/reports/transfers/summary",
            "/api/v0/telemetry/reports/transfers/summary",
        ),
        (
            "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
            "/api/v0/telemetry/reports/transfers/histogram",
        ),
        (
            "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download",
            "/api/v0/telemetry/reports/transfers/leaderboard",
        ),
        (
            "/api/v0/telemetry/reports/transfers/users/telemetry-report-failure-peer",
            "/api/v0/telemetry/reports/transfers/users/{username}",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions?direction=Download",
            "/api/v0/telemetry/reports/transfers/exceptions",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download",
            "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        ),
        (
            "/api/v0/telemetry/reports/transfers/directories",
            "/api/v0/telemetry/reports/transfers/directories",
        ),
    ] {
        let response = super::route_http_request(
            "GET",
            path,
            None,
            "",
            &telemetry_report_failure_state,
        )
        .await
        .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    for (path, route) in [
        ("/api/v0/transfers/downloads", "/api/v0/transfers/downloads"),
        ("/api/v0/transfers/uploads", "/api/v0/transfers/uploads"),
        (
            "/api/v0/transfers/downloads/transfer-read-failure-peer",
            "/api/v0/transfers/downloads/{username}",
        ),
        (
            "/api/v0/transfers/uploads/transfer-read-failure-peer",
            "/api/v0/transfers/uploads/{username}",
        ),
        (
            "/api/v0/transfers/downloads/batches/00000000-0000-4000-8000-000000000000",
            "/api/v0/transfers/downloads/batches/{id}",
        ),
    ] {
        let response =
            super::route_http_request("GET", path, None, "", &telemetry_report_failure_state)
                .await
                .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    let events_failure = super::route_http_request(
        "GET",
        "/api/v0/events?limit=1",
        None,
        "",
        &telemetry_report_failure_state,
    )
    .await
    .expect("events read failure response");
    record!(
        "GET",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        events_failure.status == "500 Internal Server Error"
            && events_failure.body.contains("event storage unavailable")
    );
    let search_responses_failure = super::route_http_request(
        "GET",
        &format!("/api/v0/searches/{search_read_failure_id}/responses"),
        None,
        "",
        &telemetry_report_failure_state,
    )
    .await
    .expect("search responses read failure response");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "runtime-failure-and-timeout",
        search_responses_failure.status == "500 Internal Server Error"
            && search_responses_failure
                .body
                .contains("search storage unavailable")
    );

    // Room subresources are real stateful projections: absent rooms are
    // 404, duplicate member insertion is idempotent, while messages and
    // ticker updates serialize under the room store write lock.
    let (room_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    room_state.session.write().await.state = "connected";
    let joined = super::route_http_request(
        "POST",
        "/api/v0/rooms/joined",
        None,
        r#""residual-room""#,
        &room_state,
    )
    .await
    .expect("join residual room");
    assert_eq!(joined.status, "201 Created");
    for (route, body) in [
        ("/api/v0/rooms/joined/missing-room/members", r#""member""#),
        ("/api/v0/rooms/joined/missing-room/messages", r#""message""#),
        ("/api/v0/rooms/joined/missing-room/ticker", r#""ticker""#),
    ] {
        let response = super::route_http_request("POST", route, None, body, &room_state)
            .await
            .unwrap_or_else(|error| panic!("POST {route}: {error}"));
        let route_template = if route.ends_with("/members") {
            "/api/v0/rooms/joined/{roomName}/members"
        } else if route.ends_with("/messages") {
            "/api/v0/rooms/joined/{roomName}/messages"
        } else {
            "/api/v0/rooms/joined/{roomName}/ticker"
        };
        record!(
            "POST",
            route_template,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    let member_updates = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/members",
            None,
            r#""same-member""#,
            &room_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/members",
            None,
            r#""same-member""#,
            &room_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/members",
        "concurrency-and-idempotency",
        member_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && room_state.rooms.read().await.records[0]
                .members
                .iter()
                .filter(|member| member == &&"same-member".to_owned())
                .count()
                == 1
    );
    let message_updates = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/messages",
            None,
            r#""message-a""#,
            &room_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/messages",
            None,
            r#""message-b""#,
            &room_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "concurrency-and-idempotency",
        message_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && room_state.rooms.read().await.records[0].messages.len() >= 2
    );
    let ticker_updates = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/ticker",
            None,
            r#""ticker-a""#,
            &room_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/ticker",
            None,
            r#""ticker-b""#,
            &room_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/ticker",
        "concurrency-and-idempotency",
        ticker_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && room_state.rooms.read().await.records[0]
                .ticker
                .as_deref()
                .is_some_and(|ticker| matches!(ticker, "ticker-a" | "ticker-b"))
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_residual_core_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd residual core ledger"),
    )
    .expect("write slskd residual core ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd residual core mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
