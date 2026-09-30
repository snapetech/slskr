use super::*;
use rcgen::generate_simple_self_signed;
use sha2::{Digest, Sha256};
use slskr_client::overlay::{
    MeshHelloAck, MeshServiceReply, OverlayFramer, OVERLAY_MAGIC, OVERLAY_VERSION,
};
use tokio_rustls::{
    rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig},
    TlsAcceptor,
};

fn request(port: u16) -> StartRequest {
    StartRequest {
        local_port: port,
        pod_id: "pod:test".to_owned(),
        destination_host: "service".to_owned(),
        destination_port: 80,
        service_name: None,
        gateway_username: "gateway".to_owned(),
        gateway_endpoints: vec!["127.0.0.1:50305".parse().unwrap()],
        gateway_certificate_sha256: [7; 32],
        local_username: "local".to_owned(),
        authentication_key: Arc::new(SigningKey::from_bytes(&[9; 32])),
    }
}

#[test]
fn performance_computes_real_oracle_derived_metrics() {
    let idle = Performance::new(0, 0);
    assert_eq!(idle.average_bytes_per_connection, 0);
    assert!(!idle.is_high_throughput);
    assert_eq!(idle.efficiency_rating, 0.0);

    let active = Performance::new(4, 8_000);
    assert_eq!(active.average_bytes_per_connection, 2_000);
    assert!(!active.is_high_throughput);
    assert_eq!(active.efficiency_rating, 2.0);

    let high_throughput = Performance::new(2, 2 * 1024 * 1024);
    assert!(high_throughput.is_high_throughput);
}

#[tokio::test]
async fn manager_binds_reports_and_stops_local_listener() {
    let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let manager = Manager::new();

    let status = manager.start(request(port)).await.unwrap();
    assert_eq!(status.local_port, port);
    assert!(status.is_active);
    assert!(status.started_at > 0);
    assert_eq!(status.last_activity, status.started_at);
    assert!(TcpStream::connect(("127.0.0.1", port)).await.is_ok());
    assert_eq!(manager.statuses().await.len(), 1);
    assert!(manager.stop(port).await);
    assert!(!manager.stop(port).await);
    assert!(manager.statuses().await.is_empty());
}

#[tokio::test]
async fn dropping_manager_releases_local_listener() {
    let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let manager = Manager::new();
    manager.start(request(port)).await.unwrap();

    drop(manager);

    timeout(Duration::from_secs(1), async {
        loop {
            if TcpListener::bind(("127.0.0.1", port)).await.is_ok() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("dropping the manager must release its listeners");
}

#[tokio::test]
async fn invalid_overlay_fields_are_rejected_before_binding() {
    let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let manager = Manager::new();

    for mutate in [
        |request: &mut StartRequest| request.local_username = "bad username".to_owned(),
        |request: &mut StartRequest| request.gateway_username = "x".repeat(65),
        |request: &mut StartRequest| request.pod_id = "x".repeat(513),
        |request: &mut StartRequest| request.destination_host = "x".repeat(256),
        |request: &mut StartRequest| request.service_name = Some("x".repeat(129)),
    ] {
        let mut invalid = request(port);
        mutate(&mut invalid);
        assert!(manager.start(invalid).await.is_err());
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .expect("invalid rule must not bind its local port");
        drop(listener);
    }
}

#[tokio::test]
async fn manager_rejects_duplicate_and_occupied_ports() {
    let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    let manager = Manager::new();
    assert!(manager.start(request(port)).await.is_err());
    drop(probe);
    manager.start(request(port)).await.unwrap();
    assert!(manager.start(request(port)).await.is_err());
    assert!(manager.stop(port).await);
}

#[tokio::test]
async fn stopping_rule_cancels_stalled_gateway_handshake_and_releases_permit() {
    let gateway_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_endpoint = gateway_listener.local_addr().unwrap();
    let stalled_gateway = tokio::spawn(async move {
        let (_stream, _) = gateway_listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let local_probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_port = local_probe.local_addr().unwrap().port();
    drop(local_probe);
    let manager = Manager::new();
    let mut request = request(local_port);
    request.gateway_endpoints = vec![gateway_endpoint];
    manager.start(request).await.unwrap();
    let rule = Arc::clone(manager.rules.read().await.get(&local_port).unwrap());
    let local = TcpStream::connect(("127.0.0.1", local_port)).await.unwrap();

    timeout(Duration::from_secs(2), async {
        while rule.active_connections.load(Ordering::Relaxed) == 0 {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        manager.connection_permits.available_permits(),
        MAX_FORWARDING_CONNECTIONS - 1
    );

    assert!(manager.stop(local_port).await);
    timeout(Duration::from_secs(2), async {
        while rule.active_connections.load(Ordering::Relaxed) != 0 {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("stalled gateway setup should be cancelled promptly");
    assert_eq!(
        manager.connection_permits.available_permits(),
        MAX_FORWARDING_CONNECTIONS
    );
    drop(local);
    stalled_gateway.abort();
    let _ = stalled_gateway.await;
}

#[test]
fn gateway_tunnel_responses_enforce_integrity_and_size_contracts() {
    let accepted = serde_json::to_vec(&OpenTunnelResponse {
        tunnel_id: "tunnel-1".to_owned(),
        accepted: true,
    })
    .unwrap();
    assert_eq!(parse_open_tunnel_response(&accepted).unwrap(), "tunnel-1");

    let oversized_id = serde_json::to_vec(&OpenTunnelResponse {
        tunnel_id: "x".repeat(MAX_TUNNEL_ID_BYTES + 1),
        accepted: true,
    })
    .unwrap();
    assert!(parse_open_tunnel_response(&oversized_id).is_err());

    validate_tunnel_data_acknowledgement(br#"{"Sent":5}"#, 5).unwrap();
    assert!(validate_tunnel_data_acknowledgement(br#"{"Sent":4}"#, 5).is_err());
    assert!(validate_tunnel_data_acknowledgement(br#"{}"#, 5).is_err());

    let valid_data = serde_json::to_vec(&TunnelDataResponse {
        data: vec![7; TUNNEL_CHUNK_BYTES],
        bytes_received: TUNNEL_CHUNK_BYTES,
    })
    .unwrap();
    assert_eq!(
        parse_tunnel_data_response(&valid_data).unwrap().len(),
        TUNNEL_CHUNK_BYTES
    );

    let oversized_data = serde_json::to_vec(&TunnelDataResponse {
        data: vec![7; TUNNEL_CHUNK_BYTES + 1],
        bytes_received: TUNNEL_CHUNK_BYTES + 1,
    })
    .unwrap();
    assert!(parse_tunnel_data_response(&oversized_data).is_err());
}

#[tokio::test]
async fn manager_forwards_bytes_through_tls_private_gateway_calls() {
    let certified = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let certificate = certified.cert.der().clone();
    let certificate_sha256 = Sha256::digest(certificate.as_ref()).into();
    let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let config =
        ServerConfig::builder_with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
            .with_no_client_auth()
            .with_single_cert(vec![certificate], private_key.into())
            .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let gateway_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_endpoint = gateway_listener.local_addr().unwrap();
    let gateway = tokio::spawn(async move {
        let (tcp, _) = gateway_listener.accept().await.unwrap();
        let tls = acceptor.accept(tcp).await.unwrap();
        let mut framer = OverlayFramer::new(tls);
        let hello: slskr_client::overlay::MeshHello = framer.read().await.unwrap();
        framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "gateway".to_owned(),
                features: vec![FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports: None,
                overlay_port: Some(gateway_endpoint.port()),
                nonce_echo: hello.nonce,
            })
            .await
            .unwrap();
        let mut buffered = Vec::new();
        loop {
            let call: MeshServiceCall = framer.read().await.unwrap();
            let (status_code, payload, done) = match call.method.as_str() {
                "OpenTunnel" => {
                    let request: OpenTunnelRequest = serde_json::from_slice(&call.payload).unwrap();
                    assert_eq!(request.pod_id, "pod:test");
                    (
                        0,
                        serde_json::to_vec(&OpenTunnelResponse {
                            tunnel_id: "tunnel-1".to_owned(),
                            accepted: true,
                        })
                        .unwrap(),
                        false,
                    )
                }
                "TunnelData" => {
                    let request: TunnelDataRequest = serde_json::from_slice(&call.payload).unwrap();
                    buffered.extend_from_slice(&request.data);
                    (0, br#"{"Sent":5}"#.to_vec(), false)
                }
                "GetTunnelData" => {
                    let data = std::mem::take(&mut buffered);
                    (
                        0,
                        serde_json::to_vec(&TunnelDataResponse {
                            bytes_received: data.len(),
                            data,
                        })
                        .unwrap(),
                        false,
                    )
                }
                "CloseTunnel" => (0, br#"{"Closed":true}"#.to_vec(), true),
                other => panic!("unexpected gateway method {other}"),
            };
            framer
                .write(&MeshServiceReply {
                    magic: OVERLAY_MAGIC.to_owned(),
                    message_type: "mesh_service_reply".to_owned(),
                    version: OVERLAY_VERSION,
                    correlation_id: call.correlation_id,
                    status_code,
                    payload,
                    error_message: None,
                })
                .await
                .unwrap();
            if done {
                break;
            }
        }
    });

    let local_probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_port = local_probe.local_addr().unwrap().port();
    drop(local_probe);
    let unavailable_probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let unavailable_endpoint = unavailable_probe.local_addr().unwrap();
    drop(unavailable_probe);
    let manager = Manager::new();
    let mut request = request(local_port);
    request.gateway_endpoints = vec![unavailable_endpoint, gateway_endpoint];
    request.gateway_certificate_sha256 = certificate_sha256;
    manager.start(request).await.unwrap();
    let mut local = TcpStream::connect(("127.0.0.1", local_port)).await.unwrap();
    local.write_all(b"hello").await.unwrap();
    let mut echoed = [0_u8; 5];
    timeout(Duration::from_secs(5), local.read_exact(&mut echoed))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&echoed, b"hello");
    timeout(Duration::from_secs(2), async {
        loop {
            let status = manager.status(local_port).await.unwrap();
            if status.bytes_forwarded == 10 {
                assert_eq!(status.bytes_in, 5);
                assert_eq!(status.bytes_out, 5);
                assert!(status.last_activity >= status.started_at);
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("forwarding totals should update while the connection remains active");
    assert!(manager.stop(local_port).await);
    timeout(Duration::from_secs(5), gateway)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn shutdown_and_forced_listener_abort_join_stalled_children_and_release_capacity() {
    for forced in [false, true] {
        let gateway_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = local_probe.local_addr().unwrap().port();
        drop(local_probe);
        let manager = Manager::new();
        let mut config = request(port);
        config.gateway_endpoints = vec![gateway_listener.local_addr().unwrap()];
        manager.start(config).await.unwrap();
        let rule = manager.rules.read().await.get(&port).unwrap().clone();
        let mut local = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let (mut gateway, _) = timeout(Duration::from_secs(2), gateway_listener.accept())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(rule.active_connections.load(Ordering::Relaxed), 1);
        if forced {
            let mut task = rule.listener_task.lock().await.take().unwrap();
            task.0.abort();
            let _ = (&mut task.0).await;
        } else {
            manager.shutdown().await;
            assert!(manager.statuses().await.is_empty());
            assert!(manager
                .start(request(port))
                .await
                .unwrap_err()
                .contains("shut down"));
        }
        let mut buffer = [0; 4096];
        timeout(Duration::from_secs(2), async {
            loop {
                match gateway.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
            assert_eq!(local.read(&mut buffer).await.unwrap(), 0);
            while rule.active_connections.load(Ordering::Relaxed) != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("forwarding child sockets and activity must close");
        assert_eq!(
            manager.connection_permits.available_permits(),
            MAX_FORWARDING_CONNECTIONS
        );
        let rebound = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        drop(rebound);
    }
}
