use super::*;
use crate::mesh_security::OVERLAY_MAX_MESSAGES_PER_SECOND;

fn temporary_directory(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "slskr-{label}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn gateway_destinations_are_confined_to_private_networks() {
    assert!(valid_destination_ip("127.0.0.1".parse().unwrap()));
    assert!(valid_destination_ip("10.0.0.1".parse().unwrap()));
    assert!(valid_destination_ip("fd00::1".parse().unwrap()));
    assert!(!valid_destination_ip("8.8.8.8".parse().unwrap()));
    assert!(!valid_destination_ip(
        "2001:4860:4860::8888".parse().unwrap()
    ));
    assert!(!valid_destination_ip("0.0.0.0".parse().unwrap()));
}

#[tokio::test]
async fn dropping_tunnel_aborts_idle_destination_reader() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (destination, _) = listener.accept().await.unwrap();
    let (reader, writer) = destination.into_split();
    let (_client_reader, _client_writer) = client.into_split();
    let (_incoming_tx, incoming_rx) = mpsc::channel(1);
    let reader_task = tokio::spawn(async move {
        let mut reader = reader;
        let mut buffer = [0_u8; 1];
        let _ = reader.read(&mut buffer).await;
    });
    let tunnel = Tunnel {
        owner: "owner".to_owned(),
        connection_id: "connection".to_owned(),
        pod_id: "pod".to_owned(),
        writer: Mutex::new(writer),
        incoming: Mutex::new(incoming_rx),
        reader_abort: reader_task.abort_handle(),
    };

    drop(tunnel);

    let result = timeout(Duration::from_secs(1), reader_task)
        .await
        .expect("dropping the tunnel must stop its reader");
    assert!(result
        .expect_err("reader task must be aborted")
        .is_cancelled());
}

#[test]
fn quic_relay_destinations_are_confined_to_public_addresses() {
    assert!(valid_public_relay_ip("8.8.8.8".parse().unwrap()));
    assert!(valid_public_relay_ip(
        "2001:4860:4860::8888".parse().unwrap()
    ));
    for address in [
        "0.0.0.0",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.1.1",
        "172.16.0.1",
        "192.168.1.1",
        "224.0.0.1",
        "fc00::1",
        "fe80::1",
    ] {
        assert!(
            !valid_public_relay_ip(address.parse().unwrap()),
            "{address}"
        );
    }
    for address in [
        "192.0.0.1",
        "192.0.2.1",
        "192.31.196.1",
        "192.52.193.1",
        "192.88.99.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "::ffff:10.0.0.1",
        "::ffff:192.0.2.1",
        "2001:db8::1",
        "2001:2::1",
        "2001:20::1",
        "100::1",
        "64:ff9b::a00:1",
        "3fff::1",
    ] {
        assert!(
            !valid_public_relay_ip(address.parse().unwrap()),
            "{address}"
        );
    }
}

#[test]
fn quic_relay_authentication_uses_the_configured_token_bytes() {
    let encoded = BASE64.encode("relay-token");
    assert!(relay_authentication_valid(
        &format!("AUTH {encoded}"),
        "relay-token"
    ));
    assert!(!relay_authentication_valid(
        &format!("AUTH {encoded}"),
        "other-token"
    ));
    assert!(!relay_authentication_valid(
        "AUTH not-base64",
        "relay-token"
    ));
}

#[test]
fn overlay_services_follow_selected_profile_feature_gates() {
    let mut features = crate::config::FeatureGateSettings::default();
    assert!(!overlay_service_enabled(
        "private-gateway",
        &features,
        crate::config::ControllerProfile::Legacy
    ));
    assert!(!overlay_service_enabled(
        "MeshContent",
        &features,
        crate::config::ControllerProfile::Legacy
    ));
    assert!(!overlay_service_enabled(
        "pods",
        &features,
        crate::config::ControllerProfile::Legacy
    ));
    assert!(!overlay_service_enabled(
        "shadow-index",
        &features,
        crate::config::ControllerProfile::Legacy
    ));
    assert!(overlay_service_enabled(
        "dht",
        &features,
        crate::config::ControllerProfile::Legacy
    ));

    features.mesh = true;
    features.pods = true;
    features.virtual_soulfind = true;
    assert!(overlay_service_enabled(
        "private-gateway",
        &features,
        crate::config::ControllerProfile::Legacy
    ));
    assert!(overlay_service_enabled(
        "MeshContent",
        &features,
        crate::config::ControllerProfile::Legacy
    ));
    assert!(overlay_service_enabled(
        "pods",
        &features,
        crate::config::ControllerProfile::Legacy
    ));
    assert!(overlay_service_enabled(
        "shadow-index",
        &features,
        crate::config::ControllerProfile::Legacy
    ));

    assert!(!overlay_service_enabled(
        "private-gateway",
        &features,
        crate::config::ControllerProfile::Native
    ));
    assert!(!overlay_service_enabled(
        "pods",
        &features,
        crate::config::ControllerProfile::Native
    ));
    assert!(!overlay_service_enabled(
        "shadow-index",
        &features,
        crate::config::ControllerProfile::Native
    ));
    assert!(overlay_service_enabled(
        "MeshContent",
        &features,
        crate::config::ControllerProfile::Native
    ));
}

#[test]
fn inbound_service_calls_use_complete_overlay_validation() {
    let mut call = MeshServiceCall::new("correlation", "pods", "List", Vec::new()).unwrap();
    assert!(valid_service_call(&call));

    call.method = "\n".to_owned();
    assert!(!valid_service_call(&call));

    call.method = "List".to_owned();
    call.payload = vec![0; MAX_OVERLAY_MESSAGE_BYTES + 1];
    assert!(!valid_service_call(&call));
}

#[test]
fn quic_proxy_admission_matches_frozen_global_and_prefix_limits() {
    let gate = QuicProxyAdmissionGate::default();
    let first_prefix = [
        "198.51.100.1:50305",
        "198.51.100.2:50305",
        "198.51.100.3:50305",
        "198.51.100.4:50305",
    ];
    let leases = first_prefix
        .into_iter()
        .map(|address| gate.try_acquire(address.parse().unwrap()))
        .collect::<Option<Vec<_>>>();
    assert!(leases.is_some());
    assert!(gate
        .try_acquire("198.51.100.5:50305".parse().unwrap())
        .is_none());
    drop(leases);

    let global_gate = QuicProxyAdmissionGate::default();
    let mut global_leases = Vec::new();
    for third_octet in 0..64_u8 {
        let address = SocketAddr::from(([203, 0, third_octet, 1], 50_305));
        global_leases.push(
            global_gate
                .try_acquire(address)
                .expect("global admission slot"),
        );
    }
    assert!(global_gate
        .try_acquire("203.0.64.1:50305".parse().unwrap())
        .is_none());
}

#[test]
fn overlay_udp_limiter_identity_is_scoped_to_source_ip() {
    let limiter = OverlayRateLimiter::new();
    for port in 1..=OVERLAY_MAX_MESSAGES_PER_SECOND {
        assert!(
            limiter
                .check_message(&super::overlay_datagram_limiter_id(SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::LOCALHOST),
                    port as u16,
                )))
                .allowed,
            "source-port rotation must not bypass the UDP message budget"
        );
    }
    assert!(
        !limiter
            .check_message(&super::overlay_datagram_limiter_id(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                65_535,
            )))
            .allowed
    );
    assert_eq!(
        super::overlay_datagram_limiter_id(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1,)),
        super::overlay_datagram_limiter_id(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 2,))
    );
}

#[test]
fn quic_packet_classifier_accepts_only_frozen_initial_shapes() {
    let mut packet = vec![0_u8; 1_200];
    packet[0] = 0xc0;
    packet[1..5].copy_from_slice(&1_u32.to_be_bytes());
    assert!(is_quic_initial_packet(&packet));
    packet[1..5].copy_from_slice(&0x6b33_43cf_u32.to_be_bytes());
    packet[0] = 0xd0;
    assert!(is_quic_initial_packet(&packet));
    packet[0] = 0x40;
    assert!(!is_quic_initial_packet(&packet));
    assert!(is_dht_packet(b"d1:ad2:id20:01234567890123456789ee"));
    assert!(!is_dht_packet(b"\xc0"));
}

#[tokio::test]
async fn shared_dht_response_forwarder_returns_packets_from_public_source() {
    let public_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let forward_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let backend_peer = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let public_address = public_socket.local_addr().unwrap();
    let forward_address = forward_socket.local_addr().unwrap();
    let forwarder = tokio::spawn(forward_dht_responses(
        Arc::clone(&forward_socket),
        Arc::clone(&public_socket),
    ));

    let packet = b"d1:ad2:id20:01234567890123456789ee";
    backend_peer.send_to(packet, forward_address).await.unwrap();
    let mut received = [0_u8; 128];
    let (size, source) = tokio::time::timeout(
        Duration::from_secs(1),
        backend_peer.recv_from(&mut received),
    )
    .await
    .expect("DHT response should be forwarded")
    .unwrap();
    assert_eq!(&received[..size], packet);
    assert_eq!(source, public_address);

    forwarder.abort();
}

#[test]
fn overlay_keepalive_and_control_validation_match_the_frozen_lifecycle() {
    assert_eq!(OVERLAY_MESSAGE_READ_TIMEOUT, Duration::from_secs(30));
    assert_eq!(OVERLAY_KEEPALIVE_INTERVAL, Duration::from_secs(120));
    assert_eq!(OVERLAY_IDLE_TIMEOUT, Duration::from_secs(300));

    let now = i64::try_from(super::super::unix_timestamp_millis()).unwrap();
    assert!(Ping {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "ping".to_owned(),
        version: OVERLAY_VERSION,
        timestamp: now,
    }
    .validate()
    .is_ok());

    let mut liveness = OverlayLiveness {
        last_inbound: Instant::now() - OVERLAY_IDLE_TIMEOUT,
        last_ping: Instant::now() - OVERLAY_KEEPALIVE_INTERVAL,
    };
    assert!(liveness.is_idle());
    liveness.record_ping();
    assert!(liveness.is_idle(), "outbound pings are not peer activity");
    liveness.record_inbound();
    assert!(!liveness.is_idle());
}

#[test]
fn gateway_tunnel_request_fields_are_bounded_before_replay_caching() {
    let request = OpenTunnelRequest {
        pod_id: "pod".to_owned(),
        destination_host: "service.local".to_owned(),
        destination_port: 80,
        service_name: None,
        request_nonce: "n".repeat(MAX_REQUEST_NONCE_BYTES),
        request_timestamp: 1,
    };
    assert!(valid_open_tunnel_request(&request));

    let mut oversized = request.clone();
    oversized.request_nonce.push('n');
    assert!(!valid_open_tunnel_request(&oversized));
    oversized = request.clone();
    oversized.destination_host = "h".repeat(MAX_DESTINATION_HOST_BYTES + 1);
    assert!(!valid_open_tunnel_request(&oversized));
    oversized = request;
    oversized.service_name = Some(String::new());
    assert!(!valid_open_tunnel_request(&oversized));
}

#[test]
fn gateway_replay_nonce_identity_is_case_insensitive() {
    let mut nonces = BTreeMap::new();
    nonces.insert(gateway_replay_nonce_key("Peer-One", "nonce"), 1);

    assert!(nonces.contains_key(&gateway_replay_nonce_key("peer-one", "nonce")));
    assert!(!nonces.contains_key(&gateway_replay_nonce_key("peer-two", "nonce")));
}

#[test]
fn gateway_requires_capability_key_authentication() {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[17; 32]);
    let public_key = signing_key.verifying_key().to_bytes();
    let certificate_sha256 = [23; 32];
    let mut hello = MeshHello::new(
        "peer",
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        Some(443),
        "nonce",
    )
    .unwrap();

    assert!(verify_overlay_peer_authentication(&hello, &public_key, &certificate_sha256).is_err());

    hello
        .authenticate(&signing_key, &certificate_sha256)
        .unwrap();
    assert!(verify_overlay_peer_authentication(&hello, &public_key, &certificate_sha256).is_ok());
}

#[tokio::test]
async fn gateway_certificate_identity_is_durable() {
    let root = temporary_directory("gateway-identity");
    let first = Gateway::load_or_create_with_quic("127.0.0.1:0".parse().unwrap(), &root, None)
        .await
        .unwrap();
    let second = Gateway::load_or_create_with_quic("127.0.0.1:0".parse().unwrap(), &root, None)
        .await
        .unwrap();
    assert_eq!(first.certificate_sha256(), second.certificate_sha256());
    drop((first, second));
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn shared_tcp_gateway_does_not_bind_a_second_public_tcp_socket() {
    let root = temporary_directory("gateway-shared-tcp");
    let public_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let public_address = public_listener.local_addr().unwrap();
    let gateway = Gateway::load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket_with_data_share_shared_tcp(
            public_address,
            &root,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            8,
            false,
        )
        .await
        .unwrap();

    assert_eq!(gateway.bind(), public_address);
    assert!(gateway.listener.lock().await.is_none());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn outbound_overlay_metadata_is_removed_when_guard_drops() {
    let root = temporary_directory("gateway-outbound-session");
    let gateway = Arc::new(
        Gateway::load_or_create_with_quic("127.0.0.1:0".parse().unwrap(), &root, None)
            .await
            .unwrap(),
    );
    let guard = gateway
        .register_outbound_guard(
            "remote".to_owned(),
            "192.0.2.10:2234".parse().unwrap(),
            vec![FEATURE_MESH_SERVICE.to_owned()],
            OVERLAY_VERSION,
            None,
        )
        .await
        .unwrap();
    let connections = gateway.active_overlay_connections().await;
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].username, "remote");
    assert_eq!(connections[0].address, "192.0.2.10");
    assert_eq!(connections[0].port, 2234);
    assert!(connections[0].is_outbound);
    drop(guard);
    tokio::task::yield_now().await;
    assert!(gateway.active_overlay_connections().await.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn outbound_overlay_metadata_is_validated_and_capacity_bounded() {
    let root = temporary_directory("gateway-outbound-capacity");
    let gateway = Gateway::load_or_create_with_quic("127.0.0.1:0".parse().unwrap(), &root, None)
        .await
        .unwrap();
    assert!(gateway
        .register_outbound_overlay(
            "x".repeat(MAX_OVERLAY_METADATA_USERNAME_BYTES + 1),
            "192.0.2.10:2234".parse().unwrap(),
            Vec::new(),
            OVERLAY_VERSION,
            None,
        )
        .await
        .is_err());
    for index in 0..MAX_GATEWAY_CONNECTIONS {
        assert!(gateway
            .register_outbound_overlay(
                format!("remote-{index}"),
                "192.0.2.10:2234".parse().unwrap(),
                vec![FEATURE_MESH_SERVICE.to_owned()],
                OVERLAY_VERSION,
                None,
            )
            .await
            .is_ok());
    }
    assert!(gateway
        .register_outbound_overlay(
            "capacity-full".to_owned(),
            "192.0.2.10:2234".parse().unwrap(),
            Vec::new(),
            OVERLAY_VERSION,
            None,
        )
        .await
        .is_err());
    drop(gateway);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_gateway_identity_is_rejected() {
    let root = temporary_directory("gateway-incomplete-identity");
    fs::write(root.join("overlay-certificate.der"), [1_u8]).unwrap();
    let error = load_or_create_certificate(&root).unwrap_err();
    assert!(error.contains("identity is incomplete"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn oversized_gateway_identity_is_rejected_before_parsing() {
    let root = temporary_directory("gateway-oversized-identity");
    fs::write(
        root.join("overlay-certificate.der"),
        vec![1_u8; MAX_CERTIFICATE_BYTES as usize + 1],
    )
    .unwrap();
    fs::write(root.join("overlay-private-key.der"), [1_u8]).unwrap();
    let error = load_or_create_certificate(&root).unwrap_err();
    assert!(error.contains("certificate is too large"));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_gateway_identity_is_rejected() {
    use std::os::unix::fs::symlink;

    let root = temporary_directory("gateway-symlinked-identity");
    let certificate_target = root.join("certificate-target.der");
    fs::write(&certificate_target, [1_u8]).unwrap();
    symlink(&certificate_target, root.join("overlay-certificate.der")).unwrap();
    fs::write(root.join("overlay-private-key.der"), [1_u8]).unwrap();
    let error = load_or_create_certificate(&root).unwrap_err();
    assert!(error.contains("certificate must be a regular file"));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn exposed_gateway_private_key_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let root = temporary_directory("gateway-exposed-private-key");
    let path = root.join("overlay-private-key.der");
    fs::write(&path, [1_u8]).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

    let error = read_identity_file(&path, "private key", MAX_PRIVATE_KEY_BYTES, true)
        .expect_err("reject exposed private key");
    assert!(error.contains("must not be accessible by group or other users"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_identity_publish_removes_temporary_secret() {
    let root = temporary_directory("gateway-failed-secret-publish");
    let destination = root.join("overlay-private-key.der");
    fs::create_dir(&destination).unwrap();

    let error = write_secret(&destination, b"private-key").unwrap_err();
    assert!(error.contains("publish failed"));
    let names = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert_eq!(names, vec![destination.file_name().unwrap()]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_private_key_publish_rolls_back_new_certificate() {
    let root = temporary_directory("gateway-identity-rollback");
    let certificate = root.join("overlay-certificate.der");
    let private_key = root.join("overlay-private-key.der");
    fs::create_dir(&private_key).unwrap();

    let error =
        write_new_identity(&certificate, &private_key, b"certificate", b"private-key").unwrap_err();
    assert!(error.contains("publish failed"));
    assert!(!certificate.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn secret_publish_never_replaces_existing_identity() {
    let root = temporary_directory("gateway-existing-identity");
    let path = root.join("overlay-private-key.der");
    fs::write(&path, b"existing-key").unwrap();

    let error = write_secret(&path, b"replacement-key").unwrap_err();

    assert!(error.contains("publish failed"), "{error}");
    assert_eq!(fs::read(&path).unwrap(), b"existing-key");
    let temporary_files = fs::read_dir(&root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != "overlay-private-key.der")
        .count();
    assert_eq!(temporary_files, 0);
    fs::remove_dir_all(root).unwrap();
}
