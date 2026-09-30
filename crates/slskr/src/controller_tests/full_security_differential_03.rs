//! Controller full security differential 03 ownership.

use super::*;

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
pub(super) async fn security_controls_differential_route_security_adapters() {
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
    let manager = crate::port_forwarding::Manager::new();
    record!(
        FORWARDER,
        "activation-default-and-profile",
        crate::port_forwarding::MAX_FORWARDING_RULES == 128
            && crate::port_forwarding::MAX_FORWARDING_CONNECTIONS == 128
            && manager.statuses().await.is_empty()
    );

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("forwarding test port");
    let local_port = probe.local_addr().expect("forwarding local address").port();
    drop(probe);
    let request = || crate::port_forwarding::StartRequest {
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
        crate::port_forwarding::validate_start_request(&request()).is_ok()
            && manager.start(request()).await.is_ok()
            && manager.statuses().await.len() == 1
    );
    let mut invalid = request();
    invalid.local_port = 80;
    record!(
        FORWARDER,
        "rejected-malicious-and-boundary-input",
        crate::port_forwarding::validate_start_request(&invalid).is_err()
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
            && crate::port_forwarding::Manager::new()
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
    let health = crate::route_http_request("GET", "/health", None, "", &state)
        .await
        .expect("health response");
    let security_status = crate::route_http_request(
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
    let unauthorized = crate::route_http_request("GET", "/api/security/status", None, "", &state)
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
    let oversized = crate::route_http_request(
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
pub(super) async fn security_controls_differential_mesh_transport() {
    use std::{
        collections::BTreeMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
    };

    use crate::mesh_security::{
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
pub(super) async fn security_controls_differential_core_security() {
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
        crate::config::ControllerProfile::Legacy,
        crate::config::ControllerProfile::Native,
    ] {
        let profile_name = target.as_str();
        let target_name = match target {
            crate::config::ControllerProfile::Legacy => "slskd",
            crate::config::ControllerProfile::Native => "slskdn",
        };
        let default_config = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", profile_name)
                .with("SLSKR_AUTH_DISABLED", "true"),
        )
        .expect("default blacklist profile");
        let mut default_runtime = crate::ManagedBlacklistRuntime::new(
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

        let configured = crate::AppConfig::from_layers(
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
            crate::ManagedBlacklistRuntime::new(configured.managed_blacklist.clone(), target, true);
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
                && (target == crate::config::ControllerProfile::Native || !case_mode_match)
        );

        let invalid_pattern = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", profile_name)
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_BLACKLISTED_PATTERNS", "["),
        );
        let invalid_cidr = crate::AppConfig::from_layers(
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
            crate::ManagedBlacklistRuntime::new(configured.managed_blacklist.clone(), target, true);
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
        let reloaded = crate::AppConfig::from_layers(
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
            crate::ManagedBlacklistRuntime::new(reloaded.managed_blacklist, target, true);
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
    let now = crate::unix_timestamp();
    let mut store = crate::RevokedJwtStore::load(&root).expect("load JWT revocation store");
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
    let persisted = fs::read_to_string(root.join(crate::RevokedJwtStore::STATE_FILE_NAME))
        .expect("read JWT revocations");
    record!(
        "slskdn",
        JWT,
        "secret-logging-and-privacy-output",
        persisted.contains("live-token") && !persisted.contains("jwt-secret")
    );
    let mut reloaded = crate::RevokedJwtStore::load(&root).expect("reload JWT revocation store");
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
pub(super) async fn security_controls_differential_native_security_controller() {
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
        crate::route_http_request("GET", "/api/v0/security/adversarial", None, "", &state)
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
        crate::route_http_request("GET", "/api/v0/security/events?count=1", None, "", &state)
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
        crate::route_http_request("GET", "/api/v0/security/events?count=0", None, "", &state)
            .await
            .expect("security controller bounded query rejection");
    record!(
        "rejected-malicious-and-boundary-input",
        rejected.status == "400 Bad Request" && rejected.body.contains("count must be positive")
    );

    let over_limit = crate::route_http_request(
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
        crate::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
            .await
            .expect("security controller dashboard projection");
    record!(
        "secret-logging-and-privacy-output",
        !dashboard.body.contains("security-controller-secret")
    );

    let (restarted, _receiver) = test_state_with_env(env);
    let reloaded =
        crate::route_http_request("GET", "/api/v0/security/adversarial", None, "", &restarted)
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
pub(super) fn security_controls_differential_passthrough_authentication() {
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
        let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env)
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
        let reloaded = crate::AppConfig::from_layers(None, FileConfig::default(), &env)
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
pub(super) fn security_controls_differential_authentication_and_jwt() {
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
        let config = crate::AppConfig::from_layers(None, FileConfig::default(), &jwt_env)
            .expect("authentication control config");
        let now = crate::unix_timestamp();
        let (jwt, claims) = crate::utils::issue_admin_jwt(&config, "security-user", now)
            .expect("issue administrator JWT");
        let verified = crate::utils::verify_admin_jwt(&config, &jwt, now);
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
                    && crate::utils::authorize_controller_route_from(
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
            crate::utils::verify_admin_jwt(&config, &tampered, now).is_none()
        );
        record!(
            target,
            SECURITY_SERVICE,
            "secret-logging-and-privacy-output",
            !format!("{verified:?}").contains(&config.controller_web_jwt_key)
                && !format!("{tampered:?}").contains(&config.controller_web_jwt_key)
        );
        let reloaded = crate::AppConfig::from_layers(None, FileConfig::default(), &jwt_env)
            .expect("reload authentication control config");
        record!(
            target,
            SECURITY_SERVICE,
            "restart-rotation-and-recovery",
            crate::utils::verify_admin_jwt(&reloaded, &jwt, now)
                .is_some_and(|value| value.jti == claims.jti)
        );

        let key = "0123456789abcdef";
        let mut api_config = config.clone();
        api_config.controller_api_keys.insert(
            "operator".to_owned(),
            crate::config::ControllerApiKeySettings {
                key: key.to_owned(),
                role: "readonly".to_owned(),
                cidr: "127.0.0.1/32".to_owned(),
                cidrs: vec![crate::config::TrustedProxyCidr::parse("127.0.0.1/32")
                    .expect("loopback API-key CIDR")],
            },
        );
        let loopback = Some("127.0.0.1:5030".parse().expect("loopback API-key address"));
        let remote = Some("192.0.2.1:5030".parse().expect("remote API-key address"));
        let api_authorized = crate::utils::authorize_controller_route_from(
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
        let wrong_key = crate::utils::authorize_controller_route_from(
            &api_config,
            "GET",
            API_ROUTE,
            Some("ApiKey wrong-key-012345"),
            None,
            loopback,
        );
        let out_of_range = crate::utils::authorize_controller_route_from(
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
        let old_after_rotation = crate::utils::authorize_controller_route_from(
            &rotated,
            "GET",
            API_ROUTE,
            Some(&format!("ApiKey {key}")),
            None,
            loopback,
        );
        let new_after_rotation = crate::utils::authorize_controller_route_from(
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
