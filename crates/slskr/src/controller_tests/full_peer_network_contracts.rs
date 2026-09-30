//! Controller full peer network contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn soulseek_connector_uses_authenticated_socks5_proxy() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind SOCKS5 fixture");
    let port = listener.local_addr().expect("SOCKS5 address").port();
    let proxy = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept SOCKS5 client");
        let mut greeting = [0_u8; 4];
        stream.read_exact(&mut greeting).await.expect("greeting");
        assert_eq!(greeting, [0x05, 0x02, 0x00, 0x02]);
        stream.write_all(&[0x05, 0x02]).await.expect("method");

        let mut auth_header = [0_u8; 2];
        stream
            .read_exact(&mut auth_header)
            .await
            .expect("auth header");
        assert_eq!(auth_header, [0x01, 5]);
        let mut username = [0_u8; 5];
        stream.read_exact(&mut username).await.expect("username");
        assert_eq!(&username, b"alice");
        let mut password_len = [0_u8; 1];
        stream
            .read_exact(&mut password_len)
            .await
            .expect("password length");
        assert_eq!(password_len[0], 6);
        let mut password = [0_u8; 6];
        stream.read_exact(&mut password).await.expect("password");
        assert_eq!(&password, b"secret");
        stream
            .write_all(&[0x01, 0x00])
            .await
            .expect("auth response");

        let mut request_header = [0_u8; 5];
        stream
            .read_exact(&mut request_header)
            .await
            .expect("connect header");
        assert_eq!(request_header, [0x05, 0x01, 0x00, 0x03, 12]);
        let mut target = [0_u8; 12];
        stream.read_exact(&mut target).await.expect("target");
        assert_eq!(&target, b"example.test");
        let mut target_port = [0_u8; 2];
        stream
            .read_exact(&mut target_port)
            .await
            .expect("target port");
        assert_eq!(u16::from_be_bytes(target_port), 4242);
        stream
            .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
            .await
            .expect("connect response");
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SLSK_PROXY_ENABLED", "true")
            .with("SLSKR_SLSK_PROXY_ADDRESS", "127.0.0.1")
            .with("SLSKR_SLSK_PROXY_PORT", &port.to_string())
            .with("SLSKR_SLSK_PROXY_USERNAME", "alice")
            .with("SLSKR_SLSK_PROXY_PASSWORD", "secret"),
    );
    let stream = crate::connect_soulseek_tcp(
        &state,
        "example.test:4242",
        crate::SoulseekSocketClass::Control,
    )
    .await
    .expect("connect through SOCKS5");
    assert_eq!(stream.peer_addr().expect("proxy peer").port(), port);
    proxy.await.expect("SOCKS5 fixture task");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn soulseek_connector_uses_no_auth_socks5_and_transfer_buffers() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind SOCKS5 fixture");
    let port = listener.local_addr().expect("SOCKS5 address").port();
    let proxy = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept SOCKS5 client");
        let mut greeting = [0_u8; 3];
        stream.read_exact(&mut greeting).await.expect("greeting");
        assert_eq!(greeting, [0x05, 0x01, 0x00]);
        stream.write_all(&[0x05, 0x00]).await.expect("method");
        let mut request = [0_u8; 10];
        stream
            .read_exact(&mut request)
            .await
            .expect("connect request");
        assert_eq!(request, [0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 8, 223]);
        stream
            .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
            .await
            .expect("connect response");
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SLSK_PROXY_ENABLED", "true")
            .with("SLSKR_SLSK_PROXY_ADDRESS", "127.0.0.1")
            .with("SLSKR_SLSK_PROXY_PORT", &port.to_string())
            .with("SLSKR_SLSK_TRANSFER_BUFFER", "81920"),
    );
    crate::connect_soulseek_tcp(
        &state,
        "127.0.0.1:2271",
        crate::SoulseekSocketClass::Transfer,
    )
    .await
    .expect("connect through no-auth SOCKS5");
    proxy.await.expect("SOCKS5 fixture task");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn port_forwarding_reads_are_bounded_and_start_requires_an_authorized_pinned_gateway(
) {
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));

    let status =
        crate::route_http_request("GET", "/api/v0/port-forwarding/status", None, "", &state)
            .await
            .expect("port forwarding status");
    assert_eq!(status.status, "200 OK");
    assert_eq!(status.body, "[]");

    let available = crate::route_http_request(
        "GET",
        "/api/v0/port-forwarding/available-ports?startPort=2000&endPort=2010&limit=3",
        None,
        "",
        &state,
    )
    .await
    .expect("available port page");
    let available = serde_json::from_str::<serde_json::Value>(&available.body).unwrap();
    assert_eq!(available["availablePortCount"], 11);
    assert_eq!(available["usedPortCount"], 0);
    assert_eq!(
        available["availablePorts"],
        serde_json::json!([2000, 2001, 2002])
    );

    let stats =
        crate::route_http_request("GET", "/api/port-forwarding/stream-stats", None, "", &state)
            .await
            .expect("port forwarding stream stats");
    let stats = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats["totalForwardingRules"], 0);
    assert_eq!(stats["rules"], serde_json::json!([]));

    let default_page = crate::route_http_request(
        "GET",
        "/api/port-forwarding/available-ports",
        None,
        "",
        &state,
    )
    .await
    .expect("default available port page");
    let default_page = serde_json::from_str::<serde_json::Value>(&default_page.body).unwrap();
    assert_eq!(
        default_page["availablePorts"].as_array().unwrap().len(),
        1_000
    );

    for path in [
        "/api/port-forwarding/available-ports?startPort=0",
        "/api/port-forwarding/available-ports?startPort=3000&endPort=2000",
        "/api/port-forwarding/available-ports?limit=0",
        "/api/port-forwarding/available-ports?limit=1001",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("invalid port forwarding query");
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }

    let missing =
        crate::route_http_request("GET", "/api/port-forwarding/status/2000", None, "", &state)
            .await
            .expect("missing port forwarding status");
    assert_eq!(missing.status, "404 Not Found");

    let start = crate::route_http_request(
        "POST",
        "/api/v0/port-forwarding/start",
        None,
        r#"{"localPort":2000,"podId":"pod-1","destinationHost":"service","destinationPort":80}"#,
        &state,
    )
    .await
    .expect("unsupported port forwarding start");
    assert_eq!(start.status, "403 Forbidden");

    let pin = "07".repeat(32);
    let create = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-forward","name":"Forward","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &state,
    )
    .await
    .expect("create gateway pod");
    assert_eq!(create.status, "201 Created", "{}", create.body);
    let mut gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    gateway.peer_id = "tester".to_owned();
    state.mesh.write().await.capability_records.push(gateway);
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);
    let start = crate::route_http_request(
        "POST",
        "/api/v0/port-forwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start port forwarding");
    assert_eq!(start.status, "200 OK", "{}", start.body);
    let status = crate::route_http_request(
        "GET",
        &format!("/api/v0/port-forwarding/status/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forwarding status");
    assert_eq!(status.status, "200 OK");
    assert!(status.body.contains("pod-forward"));

    let stop = crate::route_http_request(
        "POST",
        &format!("/api/port-forwarding/stop/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("port forwarding stop");
    assert_eq!(stop.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn port_forwarding_uses_operator_pinned_gateway_when_frozen_pod_omits_pin() {
    let trusted_peers = serde_json::json!([{
        "peerId": "tester",
        "username": "gateway",
        "overlayEndpoint": "127.0.0.1:50305",
        "certificateSha256": "07".repeat(32)
    }]);
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &trusted_peers.to_string()),
    );
    let create = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-trusted-forward","name":"Trusted Forward","capabilities":[0],"privateServicePolicy":{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","registeredServices":[],"allowedDestinations":[{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}]}}}"#,
        &state,
    )
    .await
    .expect("create frozen gateway pod without a certificate field");
    assert_eq!(create.status, "201 Created", "{}", create.body);

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);
    let start = crate::route_http_request(
        "POST",
        "/api/v0/port-forwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-trusted-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start operator-pinned port forwarding");
    assert_eq!(start.status, "200 OK", "{}", start.body);

    let stop = crate::route_http_request(
        "POST",
        &format!("/api/port-forwarding/stop/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("stop operator-pinned port forwarding");
    assert_eq!(stop.status, "200 OK");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn stun_response_parsing_decodes_the_xor_mapped_address() {
    let mapped = "203.0.113.5:51820".parse::<SocketAddr>().unwrap();
    let response = build_stun_success_response([7_u8; 12], mapped);
    assert_eq!(crate::parse_stun_mapped_address(&response), Some(mapped));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn stun_probe_resolves_the_mapped_address_from_a_real_udp_round_trip() {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind STUN fixture");
    let address = socket.local_addr().expect("STUN fixture address");
    let mapped = "198.51.100.9:4000".parse::<SocketAddr>().unwrap();
    let server = tokio::spawn(async move { serve_one_stun_response(&socket, mapped).await });
    let result = crate::stun_probe(&address.to_string())
        .await
        .expect("STUN probe");
    server.await.expect("STUN fixture task");
    assert_eq!(result.mapped, mapped);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn detect_nat_type_reports_symmetric_when_the_mapping_changes_between_probes() {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind STUN fixture");
    let address = socket.local_addr().expect("STUN fixture address");
    let server = tokio::spawn(async move {
        serve_one_stun_response(&socket, "198.51.100.9:4000".parse().unwrap()).await;
        serve_one_stun_response(&socket, "198.51.100.9:4001".parse().unwrap()).await;
    });
    let server_addr = address.to_string();
    let (nat_type, detected) = crate::detect_nat_type(&[&server_addr]).await;
    server.await.expect("STUN fixture task");
    assert_eq!(nat_type, "symmetric");
    assert!(detected);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn detect_nat_type_reports_restricted_when_the_mapping_is_stable() {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind STUN fixture");
    let address = socket.local_addr().expect("STUN fixture address");
    let mapped: SocketAddr = "198.51.100.9:4000".parse().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..3 {
            serve_one_stun_response(&socket, mapped).await;
        }
    });
    let server_addr = address.to_string();
    let (nat_type, detected) = crate::detect_nat_type(&[&server_addr, &server_addr]).await;
    server.await.expect("STUN fixture task");
    assert_eq!(nat_type, "restricted");
    assert!(detected);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn detect_nat_type_reports_unknown_when_no_server_responds() {
    let (nat_type, detected) = crate::detect_nat_type(&["127.0.0.1:1"]).await;
    assert_eq!(nat_type, "unknown");
    assert!(!detected);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn listener_errors_redact_internal_details() {
    let (state, _receiver) = test_state();
    state.listeners.write().await.last_error =
        Some("regular listener bind failed at 10.0.0.8:2234: permission denied".to_owned());

    let response = crate::route_http_request("GET", "/api/v0/listeners", None, "", &state)
        .await
        .expect("listener response");
    assert_eq!(response.status, "200 OK");
    assert!(response
        .body
        .contains("\"last_error\":\"listener unavailable\""));
    assert!(!response.body.contains("10.0.0.8"));
    assert!(!response.body.contains("permission denied"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn peer_search_responses_suppress_only_matching_wishlist_peer_folder() {
    let mut store = crate::SearchStore::new();
    let record = store
        .create_scheduled_wishlist_for_item(
            "artist album".to_owned(),
            Some("wish-1".to_owned()),
            300,
        )
        .unwrap()
        .record;
    let response = FileSearchResponse {
        username: "PeerOne".to_owned(),
        token: record.token,
        results: ["Remote/Album/Blocked.flac", "Remote/Other/Allowed.flac"]
            .into_iter()
            .map(|filename| FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: filename.to_owned(),
                size: 1,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            })
            .collect(),
        slot_free: true,
        average_speed: 0,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    };
    let ignored = crate::WishlistIgnoredResult {
        id: "ignored-1".to_owned(),
        wishlist_item_id: "wish-1".to_owned(),
        username: "peerone".to_owned(),
        directory: "Remote/Album".to_owned(),
        created_at: 1,
    };

    let (updated, appended) = store
        .add_peer_response_filtered(&response, &[ignored], None)
        .unwrap();

    assert_eq!(updated.results.len(), 1);
    assert_eq!(updated.results[0].filename, "Remote/Other/Allowed.flac");
    assert_eq!(appended.len(), 1);
    assert_eq!(appended[0].filename, "Remote/Other/Allowed.flac");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_transfer_actions_reject_before_mutation_when_dispatch_is_unavailable() {
    for (action, initial_status, initial_reason) in [
        ("start", "queued", None),
        ("retry", "failed", Some("peer offline")),
    ] {
        let (state, receiver) = test_state();
        crate::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            r#"{"filename":"Remote/Song.flac","peer_username":"friend","size":10}"#,
            &state,
        )
        .await
        .expect("create transfer");
        if action == "retry" {
            state.transfers.write().await.update_status(
                1,
                initial_status,
                Some(4),
                initial_reason.map(str::to_owned),
            );
        }
        drop(receiver);

        let response = crate::route_http_request(
            "POST",
            &format!("/api/v0/transfers/1/{action}"),
            None,
            "",
            &state,
        )
        .await
        .expect("unavailable transfer action response");
        assert_eq!(response.status, "503 Service Unavailable", "{action}");
        assert!(
            response.body.contains("session manager is not running"),
            "{action}"
        );
        let transfers = state.transfers.read().await;
        assert_eq!(transfers.entries[0].status, initial_status, "{action}");
        assert_eq!(
            transfers.entries[0].reason.as_deref(),
            initial_reason,
            "{action}"
        );
        assert_eq!(
            transfers.entries[0].bytes_transferred,
            if action == "retry" { 4 } else { 0 },
            "{action}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_negotiates_pending_transfer() {
    let (state, _receiver) = test_state();
    let token = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            None,
            Some(4),
        );
        let token = entry.token;
        transfers.update_status(entry.id, "peer_lookup", None, None);
        token
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token,
                filename: "Remote/Song.flac".to_owned(),
                size: Some(4),
            })
        );
        peer.send(&crate::PeerMessage::TransferResponse(
            crate::TransferResponse::Allowed {
                token,
                size: Some(4),
            },
        ))
        .await
        .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_transfer_response(&state, &address).await;
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "accepted");
    assert_eq!(record.size, Some(4));
    assert_eq!(record.reason, None);
    assert!(transfers.stats_json().contains("\"in_progress\":1"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_transfer_request_reuses_remembered_legacy_encoding() {
    let (state, _receiver) = test_state();
    let filename = "Музыка/песня.flac";
    let token = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            filename.to_owned(),
            None,
            Some(4),
        );
        transfers.update_status(entry.id, "peer_lookup", None, None);
        entry.token
    };
    state.remote_path_encodings.write().await.remember(
        "friend",
        filename,
        crate::ProtocolTextEncoding::Windows1251,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        init.receive().await.expect("init");
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: crate::ProtocolTextEncoding::Windows1251,
                direction: 0,
                token,
                filename: filename.to_owned(),
                size: None,
            })
        );
        peer.send(&crate::PeerMessage::TransferResponse(
            crate::TransferResponse::Allowed {
                token,
                size: Some(4),
            },
        ))
        .await
        .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_transfer_response(&state, &address).await;
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    assert_eq!(transfers.entries.first().unwrap().status, "accepted");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_uploads_accepted_local_file_transfer() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write upload file");
    add_test_share(&state, "Remote/Song.flac", &path, 4).await;
    let token = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some(path.display().to_string()),
            Some(4),
        );
        let token = entry.token;
        transfers.update_status(entry.id, "peer_lookup", None, None);
        token
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept peer-message");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("peer-message init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token,
                filename: "Remote/Song.flac".to_owned(),
                size: Some(4),
            })
        );
        peer.send(&crate::PeerMessage::TransferResponse(
            crate::TransferResponse::Allowed {
                token,
                size: Some(4),
            },
        ))
        .await
        .expect("transfer response");

        let (stream, _) = listener.accept().await.expect("accept file-transfer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("file init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        assert_eq!(file.receive_token().await.expect("token"), token);
        file.send_offset(1).await.expect("offset");
        file.read_chunk(3).await.expect("chunk")
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_transfer_response(&state, &address).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![2, 3, 4]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded", "reason={:?}", record.reason);
    assert_eq!(record.bytes_transferred, 4);
    assert_eq!(record.size, Some(4));
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_downloads_accepted_file_transfer_with_resume() {
    let (state, _receiver) = test_state();
    let downloads_dir = state.config.downloads_dir.display().to_string();
    if let Some(destination) = state
        .destinations
        .write()
        .await
        .records
        .iter_mut()
        .find(|destination| destination.is_default)
    {
        destination.path = downloads_dir;
    }
    let path = crate::safe_download_path(&state.config.downloads_dir, "Remote/Song.flac")
        .expect("download path");
    std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
    std::fs::write(&path, *b"f").expect("write partial download file");
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some(path.display().to_string()),
            Some(4),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "peer_lookup", Some(1), None);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept peer-message");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("peer-message init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: Default::default(),
                direction: 0,
                token: 1,
                filename: "Remote/Song.flac".to_owned(),
                size: None,
            })
        );
        peer.send(&crate::PeerMessage::TransferResponse(
            crate::TransferResponse::Allowed {
                token: 1,
                size: Some(4),
            },
        ))
        .await
        .expect("transfer response");

        let (stream, _) = listener.accept().await.expect("accept file-transfer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("file init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        file.send_token(1).await.expect("token");
        assert_eq!(file.receive_offset().await.expect("offset"), 1);
        file.write_chunk(b"LaC").await.expect("chunk");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_transfer_response(&state, &address).await;
    server.await.expect("server task");

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded", "reason={:?}", record.reason);
    assert_eq!(record.bytes_transferred, 4);
    assert_eq!(record.size, Some(4));
    assert_eq!(std::fs::read(&path).expect("download file"), b"fLaC");
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_falls_back_to_plain_file_transfer_when_obfuscated_fails()
{
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
        crate::SearchStore::new(),
        None,
    );
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-obfuscated-fallback-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [8_u8, 9]).expect("write upload file");
    add_test_share(&state, "Remote/Fallback.flac", &path, 2).await;
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Fallback.flac".to_owned(),
            Some(path.display().to_string()),
            Some(2),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "peer_lookup", None, None);
    }

    let unused_obfuscated = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("unused listener");
        listener.local_addr().expect("unused addr").port()
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("plain listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept p");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("peer init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token: 1,
                filename: "Remote/Fallback.flac".to_owned(),
                size: Some(2),
            })
        );
        peer.send(&crate::PeerMessage::TransferResponse(
            crate::TransferResponse::Allowed {
                token: 1,
                size: Some(2),
            },
        ))
        .await
        .expect("transfer response");

        let (stream, _) = listener.accept().await.expect("accept f");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("file init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        assert_eq!(file.receive_token().await.expect("token"), 1);
        file.send_offset(0).await.expect("offset");
        file.read_chunk(2).await.expect("chunk")
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: crate::ROTATED_OBFUSCATION_TYPE,
        obfuscated_port: unused_obfuscated,
    };

    crate::project_peer_transfer_response(&state, &address).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![8, 9]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded");
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn indirect_transfer_command_requests_connect_to_peer() {
    let (state, mut receiver) = test_state();
    crate::session_runtime::try_send_session_command(
        &state,
        crate::SessionCommand::IndirectTransfer {
            id: 7,
            username: "friend".to_owned(),
            token: 42,
        },
    )
    .expect("queue indirect transfer command");

    assert_eq!(
        receiver.try_recv().expect("indirect command"),
        crate::SessionCommand::IndirectTransfer {
            id: 7,
            username: "friend".to_owned(),
            token: 42,
        }
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn connect_to_peer_response_executes_indirect_file_upload() {
    let (state, _receiver) = test_state();
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-indirect-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [8_u8, 9, 10]).expect("write upload file");
    add_test_share(&state, "Remote/Indirect.flac", &path, 3).await;
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Indirect.flac".to_owned(),
            Some(path.display().to_string()),
            Some(3),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "indirect_pending", None, None);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept indirect");
        let incoming = slskr_client::listener::demux_incoming(stream)
            .await
            .expect("demux indirect");
        let slskr_client::listener::IncomingConnection::PierceFirewall { token, stream } = incoming
        else {
            panic!("expected pierce firewall");
        };
        assert_eq!(token, 1);
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(stream);
        assert_eq!(file.receive_token().await.expect("token"), 1);
        file.send_offset(1).await.expect("offset");
        file.read_chunk(2).await.expect("chunk")
    });
    let response = crate::ConnectToPeerResponse {
        username: "friend".to_owned(),
        connection_type: "F".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        token: 1,
        privileged: false,
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_indirect_transfer_response(&state, &response).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![9, 10]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded");
    assert_eq!(record.bytes_transferred, 3);
    assert_eq!(record.size, Some(3));
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn connect_to_peer_response_executes_indirect_browse() {
    let (state, _receiver) = test_state();
    {
        let mut browse = state.browse.write().await;
        browse.request("friend".to_owned());
        assert_eq!(
            browse.mark_indirect_pending("friend", "direct failed".to_owned()),
            Some(1)
        );
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept indirect");
        let incoming = slskr_client::listener::demux_incoming(stream)
            .await
            .expect("demux indirect");
        let slskr_client::listener::IncomingConnection::PierceFirewall { token, stream } = incoming
        else {
            panic!("expected pierce firewall");
        };
        assert_eq!(token, 1);
        let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
        assert_eq!(
            peer.receive().await.expect("browse request"),
            crate::PeerMessage::GetShareFileList
        );
        let entries =
            crate::config::parse_share_entries("Remote/Indirect.flac=55").expect("entries");
        let payload = crate::build_shared_file_list_payload(&entries).expect("payload");
        peer.send(&crate::PeerMessage::SharedFileListResponse(payload))
            .await
            .expect("response");
    });
    let response = crate::ConnectToPeerResponse {
        username: "friend".to_owned(),
        connection_type: "P".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        token: 1,
        privileged: false,
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_indirect_browse_response(&state, &response).await;
    server.await.expect("server task");

    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "ready");
    assert_eq!(record.indirect_token, None);
    assert_eq!(record.entries.len(), 1);
    assert_eq!(record.entries[0].filename, "Remote/Indirect.flac");
    assert_eq!(record.entries[0].size, 55);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_endpoint_cache_does_not_alias_case_distinct_usernames() {
    let (state, _receiver) = test_state();
    crate::remember_peer_endpoint(
        &state,
        slskr_client::protocol::server::PeerAddress {
            username: "CasePeer".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: 40_001,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    assert!(crate::cached_peer_endpoint(&state, "CasePeer")
        .await
        .is_some());
    assert!(crate::cached_peer_endpoint(&state, "casepeer")
        .await
        .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn peer_preview_ticket_streams_remote_soulseek_bytes_without_transfer_record() {
    run_controller_future_on_large_stack("peer-preview-ticket-stream", || {
        peer_preview_ticket_streams_remote_soulseek_bytes_without_transfer_record_impl()
    });
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_fetches_pending_browse_from_plain_peer() {
    let (state, _receiver) = test_state();
    {
        let mut browse = state.browse.write().await;
        browse.request("friend".to_owned());
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("browse request"),
            crate::PeerMessage::GetShareFileList
        );
        let entries = crate::config::parse_share_entries("Remote/Song.flac=321").expect("entries");
        let payload = crate::build_shared_file_list_payload(&entries).expect("payload");
        peer.send(&crate::PeerMessage::SharedFileListResponse(payload))
            .await
            .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_browse_response(&state, &address).await;
    server.await.expect("server task");

    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "ready");
    assert_eq!(record.entries.len(), 1);
    assert_eq!(record.entries[0].filename, "Remote/Song.flac");
    assert_eq!(record.entries[0].size, 321);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_falls_back_to_plain_browse_when_obfuscated_fails() {
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
        crate::SearchStore::new(),
        None,
    );
    {
        let mut browse = state.browse.write().await;
        browse.request("friend".to_owned());
    }

    let unused_obfuscated = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("unused listener");
        listener.local_addr().expect("unused addr").port()
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("plain listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("browse request"),
            crate::PeerMessage::GetShareFileList
        );
        let entries =
            crate::config::parse_share_entries("Remote/Fallback.flac=222").expect("entries");
        let payload = crate::build_shared_file_list_payload(&entries).expect("payload");
        peer.send(&crate::PeerMessage::SharedFileListResponse(payload))
            .await
            .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: crate::ROTATED_OBFUSCATION_TYPE,
        obfuscated_port: unused_obfuscated,
    };

    crate::project_peer_browse_response(&state, &address).await;
    server.await.expect("server task");

    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "ready");
    assert_eq!(record.entries.len(), 1);
    assert_eq!(record.entries[0].filename, "Remote/Fallback.flac");
    assert_eq!(record.entries[0].size, 222);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_falls_back_to_indirect_browse() {
    let (state, mut receiver) = test_state();
    {
        let mut browse = state.browse.write().await;
        browse.request("friend".to_owned());
    }
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: 0,
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_browse_response(&state, &address).await;

    assert_eq!(
        receiver.try_recv().expect("indirect browse command"),
        crate::SessionCommand::IndirectBrowse {
            username: "friend".to_owned(),
            token: 1,
        }
    );
    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "indirect_pending");
    assert_eq!(record.indirect_token, Some(1));
    assert!(record
        .reason
        .as_deref()
        .unwrap_or_default()
        .contains("direct browse failed"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn indirect_browse_dispatch_failure_does_not_stay_pending() {
    let (state, _receiver) = test_state();
    for _ in 0..8 {
        crate::session_runtime::try_send_session_command(&state, crate::SessionCommand::Ping)
            .expect("fill command queue");
    }
    state.browse.write().await.request("friend".to_owned());
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: 0,
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_browse_response(&state, &address).await;

    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "failed");
    assert_eq!(record.indirect_token, None);
    assert!(record
        .reason
        .as_deref()
        .unwrap_or_default()
        .contains("session command queue rejected request"));
    drop(browse);
    assert!(state
        .session
        .read()
        .await
        .last_error
        .as_deref()
        .unwrap_or_default()
        .contains("indirect browse f***d dispatch failed"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_address_response_fetches_pending_browse_folder_from_plain_peer() {
    let (state, _receiver) = test_state();
    {
        let mut browse = state.browse.write().await;
        browse.request_folder("friend".to_owned(), "Remote/Album".to_owned());
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        let init_message = init.receive().await.expect("init");
        assert_eq!(
            init_message,
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("folder request"),
            crate::PeerMessage::FolderContentsRequest(crate::FolderContentsRequest {
                folder_encoding: Default::default(),
                token: 0,
                folder: "Remote/Album".to_owned()
            })
        );
        let entries =
            crate::config::parse_share_entries("Remote/Album/Song.flac=321").expect("entries");
        let payload =
            crate::build_folder_contents_payload(&entries, 0, "Remote/Album", Default::default())
                .expect("payload");
        peer.send(&crate::PeerMessage::FolderContentsResponse(payload))
            .await
            .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_browse_response(&state, &address).await;
    server.await.expect("server task");

    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "ready");
    assert_eq!(record.folder.as_deref(), Some("Remote/Album"));
    assert_eq!(record.entries.len(), 1);
    assert_eq!(record.entries[0].filename, "Remote/Album/Song.flac");
    assert_eq!(record.entries[0].size, 321);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_folder_request_reuses_remembered_legacy_encoding() {
    let (state, _receiver) = test_state();
    state
        .browse
        .write()
        .await
        .request_folder("friend".to_owned(), "Музыка".to_owned());
    state.remote_path_encodings.write().await.remember(
        "friend",
        "Музыка",
        crate::ProtocolTextEncoding::Windows1251,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        init.receive().await.expect("init");
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("folder request"),
            crate::PeerMessage::FolderContentsRequest(crate::FolderContentsRequest {
                folder_encoding: crate::ProtocolTextEncoding::Windows1251,
                token: 0,
                folder: "Музыка".to_owned(),
            })
        );
        let mut writer = crate::Writer::new();
        writer.write_u32_le(0);
        let payload = crate::compress_zlib_payload(&writer.into_inner()).unwrap();
        peer.send(&crate::PeerMessage::FolderContentsResponse(payload))
            .await
            .expect("response");
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(local_addr.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    };

    crate::project_peer_browse_response(&state, &address).await;
    server.await.expect("server task");

    let browse = state.browse.read().await;
    let record = browse.get("friend").expect("browse record");
    assert_eq!(record.status, "ready");
    assert_eq!(record.folder.as_deref(), Some("Музыка"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn peer_host_override_is_sanitized_config_only() {
    let env = MapEnv::default().with("SLSKR_PEER_HOST_OVERRIDE", "127.0.0.1");
    let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    assert_eq!(
        config.peer_host_override,
        Some(std::net::Ipv4Addr::new(127, 0, 0, 1))
    );
    assert!(config
        .sanitized_json()
        .contains("\"peer_host_override\":\"127.0.0.1\""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn peer_message_names_are_stable() {
    assert_eq!(
        crate::peer_message_name(&slskr_client::protocol::peer::PeerMessage::UserInfoRequest),
        "UserInfoRequest"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn distributed_runtime_propagates_branch_depth_capacity_and_disable() {
    let (state, mut session_commands) = test_state();
    let (parent, mut parent_messages) = mpsc::channel(8);
    let (alpha, mut alpha_messages) = mpsc::channel(8);
    let (beta, mut beta_messages) = mpsc::channel(8);
    {
        let mut runtime = state.distributed_network.write().await;
        runtime.parent = Some("parent".to_owned());
        runtime.parent_sender = Some(parent);
        runtime.branch_level = 4;
        runtime.branch_root = "root".to_owned();
        runtime.children.insert("alpha".to_owned(), alpha);
        runtime.children.insert("beta".to_owned(), beta);
        runtime.child_depths.insert("alpha".to_owned(), 2);
        runtime.child_depths.insert("beta".to_owned(), 5);
    }

    crate::notify_distributed_branch(&state).await;
    for receiver in [&mut alpha_messages, &mut beta_messages] {
        assert_eq!(
            receiver.recv().await,
            Some(crate::DistributedMessage::BranchLevel { level: 4 })
        );
        assert_eq!(
            receiver.recv().await,
            Some(crate::DistributedMessage::BranchRoot {
                username: "root".to_owned(),
            })
        );
    }
    assert!(matches!(
        session_commands.recv().await,
        Some(crate::SessionCommand::DistributedBranch {
            has_parent: true,
            accept_children: true,
            level: 4,
            root,
        }) if root == "root"
    ));

    crate::handle_distributed_message(
        &state,
        "alpha",
        crate::DistributedConnectionRole::Child,
        crate::DistributedMessage::Ping,
    )
    .await;
    assert!(matches!(
        alpha_messages.recv().await,
        Some(crate::DistributedMessage::PingResponse { token }) if token > 0
    ));

    crate::notify_distributed_child_depth(&state).await;
    assert_eq!(
        parent_messages.recv().await,
        Some(crate::DistributedMessage::ChildDepth { depth: 6 })
    );

    crate::apply_distributed_settings(
        &state,
        crate::config::SoulseekDistributedSettings {
            disabled: false,
            disable_children: false,
            child_limit: 1,
            logging: true,
        },
    )
    .await;
    {
        let runtime = state.distributed_network.read().await;
        assert_eq!(
            runtime.children.keys().cloned().collect::<Vec<_>>(),
            ["alpha"]
        );
        assert!(
            runtime.json(*state.soulseek_distributed_settings.read().await)["canAcceptChildren"]
                == false
        );
    }

    crate::apply_distributed_settings(
        &state,
        crate::config::SoulseekDistributedSettings {
            disabled: true,
            disable_children: false,
            child_limit: 1,
            logging: false,
        },
    )
    .await;
    let runtime = state.distributed_network.read().await;
    assert!(runtime.parent.is_none());
    assert!(runtime.parent_sender.is_none());
    assert!(runtime.children.is_empty());
    assert_eq!(runtime.branch_level, 0);
    assert_eq!(runtime.branch_root, runtime.local_username);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn soulseek_profile_and_distributed_yaml_validation_matches_target_profiles() {
    let cases = [
        (
            "soulseek:\n  diagnostic_level: trace\n",
            None,
            None,
        ),
        (
            "soulseek:\n  diagnostic_level: verbose\n",
            Some("Invalid configuration:\n  Soulseek:\n    The DiagnosticLevel field must be one of: None, Warning, Info, Debug, Trace. Case insensitive."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  picture: /tmp/slskr-picture-that-does-not-exist\n",
            Some("Invalid configuration:\n  Soulseek:\n    The Picture field specifies a non-existent file '/tmp/slskr-picture-that-does-not-exist'."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  distributed_network:\n    child_limit: 0\n",
            Some("Invalid configuration:\n  Soulseek:\n    DistributedNetwork:\n      The field ChildLimit must be between 1 and 2147483647."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  distributed_network:\n    child_limit: 2147483648\n",
            Some("Exception during deserialization: Arithmetic operation resulted in an overflow."),
            Some("Invalid YAML configuration"),
        ),
        (
            "soulseek:\n  distributed_network:\n    disabled: nope\n",
            Some("Exception during deserialization: The value \"nope\" is not a valid YAML Boolean"),
            Some("Invalid YAML configuration"),
        ),
    ];
    for (yaml, slskd, slskdn) in cases {
        let value = crate::parse_controller_yaml(yaml).unwrap();
        assert_eq!(
            crate::controller_yaml_target_validation_error(
                &value,
                crate::ControllerProfile::Legacy,
            )
            .as_deref(),
            slskd,
            "slskd validation mismatch for {yaml:?}"
        );
        assert_eq!(
            crate::controller_yaml_target_validation_error(
                &value,
                crate::ControllerProfile::Native,
            )
            .as_deref(),
            slskdn,
            "slskdN validation mismatch for {yaml:?}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn distributed_child_socket_exchanges_branch_and_depth_then_closes_on_disable() {
    let (state, _session_commands) = test_state();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let connect = tokio::net::TcpStream::connect(address);
    let accept = listener.accept();
    let (client, accepted) = tokio::join!(connect, accept);
    let client = client.unwrap();
    let (server, _) = accepted.unwrap();
    crate::register_distributed_child(Arc::clone(&state), "child".to_owned(), server, false)
        .await
        .unwrap();
    let mut peer = slskr_client::stream::DistributedConnection::new(client);
    assert_eq!(
        peer.receive().await.unwrap(),
        crate::DistributedMessage::BranchLevel { level: 0 }
    );
    assert_eq!(
        peer.receive().await.unwrap(),
        crate::DistributedMessage::BranchRoot {
            username: "tester".to_owned(),
        }
    );
    peer.send(&crate::DistributedMessage::ChildDepth { depth: 3 })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state
                .distributed_network
                .read()
                .await
                .child_depths
                .get("child")
                == Some(&3)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    crate::apply_distributed_settings(
        &state,
        crate::config::SoulseekDistributedSettings {
            disabled: true,
            disable_children: false,
            child_limit: 25,
            logging: false,
        },
    )
    .await;
    assert!(tokio::time::timeout(Duration::from_secs(1), peer.receive())
        .await
        .unwrap()
        .is_err());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn soulseek_diagnostic_and_distributed_logging_filters_are_runtime_consumers() {
    assert!(!crate::event_runtime::soulseek_diagnostic_level_allows(
        crate::config::SoulseekDiagnosticLevel::None,
        crate::logging::LogLevel::Error,
    ));
    assert!(!crate::event_runtime::soulseek_diagnostic_level_allows(
        crate::config::SoulseekDiagnosticLevel::Warning,
        crate::logging::LogLevel::Info,
    ));
    assert!(crate::event_runtime::soulseek_diagnostic_level_allows(
        crate::config::SoulseekDiagnosticLevel::Warning,
        crate::logging::LogLevel::Warn,
    ));
    assert!(crate::event_runtime::soulseek_diagnostic_level_allows(
        crate::config::SoulseekDiagnosticLevel::Trace,
        crate::logging::LogLevel::Trace,
    ));

    let (state, _session_commands) = test_state();
    let before = state.events.read().await.records.len();
    crate::record_soulseek_diagnostic(
        &state,
        crate::logging::LogLevel::Info,
        "distributed",
        "hidden",
    )
    .await;
    assert_eq!(state.events.read().await.records.len(), before);
    state.soulseek_distributed_settings.write().await.logging = true;
    crate::record_soulseek_diagnostic(
        &state,
        crate::logging::LogLevel::Info,
        "distributed",
        "visible",
    )
    .await;
    assert_eq!(state.events.read().await.records.len(), before + 1);
}
