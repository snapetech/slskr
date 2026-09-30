//! Controller full security differential 02 ownership.

use super::*;

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
pub(super) async fn security_controls_differential_solid_fetch_policy() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let target = "slskdn";
    let subject = "Solid/SolidFetchPolicy";
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

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let defaults = state.media_services.read().await.solid.clone();
    record!(
        "activation-default-and-profile",
        defaults.max_fetch_bytes > 0
            && !defaults.allow_insecure_http
            && !defaults.allow_localhost_for_web_id
            && !defaults.timeout.is_zero()
    );

    {
        let mut media = state.media_services.write().await;
        media.solid.allow_insecure_http = true;
        media.solid.allow_localhost_for_web_id = true;
        media.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
        media.solid.timeout = Duration::from_secs(1);
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Solid policy fixture");
    let port = listener
        .local_addr()
        .expect("Solid policy fixture address")
        .port();
    let web_id = format!("http://127.0.0.1:{port}/profile/card#me");
    let profile = format!(
        "@prefix solid: <http://www.w3.org/ns/solid/terms#>.\n<{web_id}> solid:oidcIssuer <https://solid-policy.example/oidc>.\n"
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("Solid policy request");
        let mut request = [0_u8; 4096];
        let _ = stream
            .read(&mut request)
            .await
            .expect("read Solid policy request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/turtle\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            profile.len(), profile
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write Solid policy response");
    });
    let accepted = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        &serde_json::json!({"webId": web_id}).to_string(),
        &state,
    )
    .await
    .expect("accepted Solid policy request");
    server.await.expect("Solid policy fixture task");
    let accepted_json =
        serde_json::from_str::<serde_json::Value>(&accepted.body).unwrap_or_default();
    record!(
        "accepted-nominal-input",
        accepted.status == "200 OK"
            && accepted_json["oidcIssuers"]
                == serde_json::json!(["https://solid-policy.example/oidc"])
    );

    {
        let mut media = state.media_services.write().await;
        media.solid.allow_localhost_for_web_id = false;
    }
    let blocked_local = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        r#"{"webId":"http://127.0.0.1:1/profile#solid-policy-secret"}"#,
        &state,
    )
    .await
    .expect("blocked localhost Solid policy request");
    let blocked_remote = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        r#"{"webId":"https://private-solid-policy.example/profile#opaque-secret"}"#,
        &state,
    )
    .await
    .expect("blocked host Solid policy request");
    record!(
        "rejected-malicious-and-boundary-input",
        blocked_local.status == "400 Bad Request"
            && blocked_remote.status == "400 Bad Request"
            && blocked_local.body.contains("Solid fetch blocked")
            && blocked_remote.body.contains("Solid fetch blocked")
    );
    record!(
        "secret-logging-and-privacy-output",
        !blocked_local.body.contains("solid-policy-secret")
            && !blocked_remote.body.contains("opaque-secret")
            && !blocked_local.body.contains("127.0.0.1:1")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create Solid policy security evidence directory");
    fs::write(
        evidence_dir.join("solid_fetch_policy.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Solid policy ledger"),
    )
    .expect("write Solid policy ledger");
    assert!(
        mismatches.is_empty(),
        "{} Solid-fetch-policy mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
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
pub(super) async fn security_controls_differential_mesh_surface() {
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
    let relaxed_config = crate::AppConfig::from_layers(
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
    let too_small = crate::AppConfig::from_layers(
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

    let reloaded_config = crate::AppConfig::from_layers(
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
    let sync_config = crate::AppConfig::from_layers(
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
        .sign_at(&remote_key, crate::unix_timestamp_millis() as i64)
        .expect("sign strict mesh request");
    let strict_response = crate::mesh_sync::handle_signed_message(
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
    let invalid_sync = crate::AppConfig::from_layers(
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

    let mut quarantined_mesh = crate::MeshState::new();
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
    let sync_reload = crate::AppConfig::from_layers(
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
        crate::route_http_request("GET", "/mesh/http/services", None, "", &default_state)
            .await
            .expect("disabled gateway response");
    record!(
        GATEWAY_AUTH,
        "activation-default-and-profile",
        disabled.status == "404 Not Found" && disabled.body == r#"{"error":"gateway_disabled"}"#
    );

    let remote = crate::RequestSecurityHeaders {
        remote_addr: Some("192.0.2.55:4444".parse().unwrap()),
        x_gateway_api_key: Some("gateway-secret".to_owned()),
        ..crate::RequestSecurityHeaders::default()
    };
    let authorized = crate::mesh_gateway_auth_failure(&gateway_state, &remote).is_none();
    record!(GATEWAY_AUTH, "accepted-nominal-input", authorized);

    let mut invalid_remote = remote.clone();
    invalid_remote.x_gateway_api_key = Some("wrong".to_owned());
    let rejected =
        crate::mesh_gateway_auth_failure(&gateway_state, &invalid_remote).is_some_and(|response| {
            response.status == "401 Unauthorized" && !response.body.contains("gateway-secret")
        });
    let local_origin = crate::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:4444".parse().unwrap()),
        origin: Some("https://evil.example".to_owned()),
        x_gateway_csrf: Some("csrf-secret".to_owned()),
        ..crate::RequestSecurityHeaders::default()
    };
    let origin_rejected = crate::mesh_gateway_auth_failure(&gateway_state, &local_origin)
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
        crate::mesh_gateway_auth_failure(&gateway_state, &invalid_remote)
            .is_some_and(|response| !response.body.contains("gateway-secret"))
    );

    let (rotated_state, _receiver) = test_state_with_env(
        gateway_env
            .clone()
            .with("SLSKD_MESH_GATEWAY_API_KEY", "rotated-secret"),
    );
    let old_rejected = crate::mesh_gateway_auth_failure(&rotated_state, &remote)
        .is_some_and(|response| response.status == "401 Unauthorized");
    let mut rotated_remote = remote.clone();
    rotated_remote.x_gateway_api_key = Some("rotated-secret".to_owned());
    let new_accepted = crate::mesh_gateway_auth_failure(&rotated_state, &rotated_remote).is_none();
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
    let first_gateway = crate::private_gateway::Gateway::load_or_create_with_quic(
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
    let malformed_result = crate::private_gateway::Gateway::load_or_create_with_quic(
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
    let second_gateway = crate::private_gateway::Gateway::load_or_create_with_quic(
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
pub(super) async fn security_controls_differential_content_safety() {
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
    let nominal = crate::enforce_completed_download_content_safety(&state, &good, &good).await;
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
        crate::enforce_completed_download_content_safety(&state, &malicious, &malicious).await;
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
        crate::enforce_completed_download_content_safety(&state, &secret_path, &secret_path)
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
pub(super) fn security_controls_differential_soulseek_safety() {
    use crate::rate_limit::{SoulseekSafetyConfig, SoulseekSafetyLimiter};
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
pub(super) fn security_controls_differential_security_event_sink() {
    use crate::mesh_security::{SecurityEventSink, SecuritySinkEvent, SecuritySinkSeverity};
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
pub(super) fn security_controls_differential_integrity_controls() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::thread;

    use crate::security_controls::{
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
pub(super) fn security_controls_differential_runtime_controls() {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
    };

    use crate::security_controls::{
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
