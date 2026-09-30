//! Controller full session contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn reconnect_backoff_is_interrupted_by_session_commands() {
    let (sender, mut receiver) = mpsc::channel(1);
    let waiter = tokio::spawn(async move {
        crate::wait_for_reconnect_or_command(&mut receiver, Duration::from_secs(3_600)).await
    });
    tokio::task::yield_now().await;
    sender
        .send(crate::SessionCommand::Disconnect)
        .await
        .expect("send disconnect");

    let wake = tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .expect("command must interrupt reconnect delay")
        .expect("wait task");
    assert!(matches!(
        wake,
        crate::ReconnectWake::Command(crate::SessionCommand::Disconnect)
    ));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn session_create_does_not_echo_api_token() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "secret-token")
            .with("SLSKD_USERNAME", "admin")
            .with("SLSKD_PASSWORD", "secret-token"),
    );
    let response = crate::route_http_request(
        "POST",
        "/api/session",
        Some("Bearer secret-token"),
        r#"{"username":"user"}"#,
        &state,
    )
    .await
    .expect("session response");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["token"], "");
    assert!(json["issued"].is_number());
    assert!(json["notBefore"].is_number());
    assert!(json["expires"].is_number());
    assert_eq!(json["tokenConfigured"], true);
    assert!(!response.body.contains("secret-token"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn force_checking_the_latest_version_awaits_a_real_github_lookup() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind version fixture");
    let address = listener.local_addr().expect("version fixture address");
    let server = tokio::spawn(async move {
        serve_json_fixture(
            &listener,
            serde_json::json!({
                "tag_name": "v9.9.9-slskdn.20260101120000",
                "html_url": "https://github.com/snapetech/slskdn/releases/tag/v9.9.9-slskdn.20260101120000",
            }),
        )
        .await
    });
    let (state, _receiver) = test_state();
    crate::refresh_controller_version_check(&state, &format!("http://{address}"))
        .await
        .expect("version fixture lookup");
    server.await.expect("version fixture task");

    let version = state
        .controller_version
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        version.latest.as_deref(),
        Some("9.9.9-slskdn.20260101120000")
    );
    assert!(version.latest_tag.is_some());
    assert!(version.checked_at.is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn session_errors_redact_internal_details() {
    let (state, _receiver) = test_state();
    state.session.write().await.last_error =
        Some("server receive failed from 10.0.0.9:2242: /private/session.db denied".to_owned());

    for path in ["/api/v0/server"] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("session response");
        assert_eq!(response.status, "200 OK", "{path}");
        assert!(response.body.contains("session operation failed"), "{path}");
        assert!(!response.body.contains("10.0.0.9"), "{path}");
        assert!(!response.body.contains("/private"), "{path}");
        assert!(!response.body.contains("denied"), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn session_control_routes_fail_when_manager_is_not_running() {
    let (state, receiver) = test_state();
    drop(receiver);

    for path in [
        "/api/session/connect",
        "/api/session/ping",
        "/api/session/disconnect",
        "/api/session/privileges/check",
    ] {
        let response = crate::route_http_request("POST", path, None, "", &state)
            .await
            .expect("route response");
        assert_eq!(response.status, "503 Service Unavailable");
        assert!(response.body.contains("session manager is not running"));
    }

    let connect = crate::route_http_request("PUT", "/api/server", None, "", &state)
        .await
        .expect("connect response");
    assert_eq!(connect.status, "503 Service Unavailable");
    assert_eq!(state.session.read().await.state, "disconnected");

    state.session.write().await.state = "connected";
    let disconnect = crate::route_http_request("DELETE", "/api/server", None, "", &state)
        .await
        .expect("disconnect response");
    assert_eq!(disconnect.status, "503 Service Unavailable");
    assert_eq!(state.session.read().await.state, "connected");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn daemon_outbound_capability_probe_requires_matching_signed_acknowledgement() {
    use ed25519_dalek::SigningKey;

    let (state, _receiver) = test_state();
    state
        .advanced_networking
        .write()
        .await
        .mesh
        .enable_soulseek_rendezvous = true;
    let remote_key = SigningKey::from_bytes(&[9_u8; 32]);
    let remote_descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "remote-peer",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        Vec::new(),
        std::time::Duration::from_secs(300),
        &remote_key,
        std::time::SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.with_overlay_port(Some(50_305)).sign(&remote_key))
    .expect("signed remote descriptor");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local address");
    let server_descriptor = remote_descriptor.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
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
        let hello = peer.receive().await.expect("capability hello");
        let hello = slskr_client::capabilities::decode_peer_capability_message(&hello)
            .expect("decode hello")
            .expect("capability envelope");
        assert_eq!(
            hello.message_type,
            slskr_client::capabilities::PeerCapabilityMessageType::Hello
        );
        hello
            .descriptor
            .verify(std::time::SystemTime::now())
            .expect("verify local descriptor");
        let acknowledgement = slskr_client::capabilities::peer_capability_message(
            &slskr_client::capabilities::PeerCapabilityEnvelope::new(
                slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge,
                hello.nonce,
                server_descriptor,
            ),
        )
        .expect("acknowledgement");
        peer.send(&acknowledgement)
            .await
            .expect("send acknowledgement");
    });
    crate::remember_peer_endpoint(
        &state,
        slskr_client::protocol::server::PeerAddress {
            username: "remote-peer".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    let observed = crate::probe_peer_capability(&state, "remote-peer")
        .await
        .expect("capability probe");
    server.await.expect("server task");
    assert_eq!(observed.username, "remote-peer");
    assert_eq!(observed.peer_id, remote_descriptor.peer_id);
    assert_eq!(observed.overlay_port, Some(50_305));
    assert_eq!(state.mesh.read().await.capability_records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn daemon_ingests_verified_peer_capabilities_and_acknowledges_hello() {
    use ed25519_dalek::SigningKey;

    let (state, _receiver) = test_state();
    let signing_key = SigningKey::from_bytes(&[8_u8; 32]);
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "mesh-source",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        vec!["https://mesh.example/content".to_owned()],
        std::time::Duration::from_secs(300),
        &signing_key,
        std::time::SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.sign(&signing_key))
    .expect("signed capability descriptor");
    let nonce = "03030303030303030303030303030303";
    let hello = slskr_client::capabilities::peer_capability_message(
        &slskr_client::capabilities::PeerCapabilityEnvelope::new(
            slskr_client::capabilities::PeerCapabilityMessageType::Hello,
            nonce,
            descriptor.clone(),
        ),
    )
    .expect("capability hello");
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();

    crate::handle_peer_message(&state, hello, Some("mesh-source"), |response| async move {
        response_tx
            .send(response)
            .map_err(|_| "capability response receiver closed".to_owned())
    })
    .await
    .expect("ingest capability hello");

    let response = response_rx.await.expect("capability acknowledgement");
    let acknowledgement = slskr_client::capabilities::decode_peer_capability_message(&response)
        .expect("decode acknowledgement")
        .expect("capability envelope");
    assert_eq!(
        acknowledgement.message_type,
        slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge
    );
    assert_eq!(acknowledgement.nonce, nonce);
    acknowledgement
        .descriptor
        .verify(std::time::SystemTime::now())
        .expect("verify local acknowledgement descriptor");
    let mesh = state.mesh.read().await;
    assert_eq!(mesh.capability_records.len(), 1);
    assert_eq!(mesh.capability_records[0].username, "mesh-source");
    assert_eq!(mesh.capability_records[0].peer_id, descriptor.peer_id);
    assert_eq!(mesh.capability_records[0].features, descriptor.features);
    assert!(
        mesh.capability_records[0].expires_at_unix
            <= crate::unix_timestamp() + crate::PEER_CAPABILITY_LEASE_SECONDS
    );
    drop(mesh);

    let replay = slskr_client::capabilities::peer_capability_message(
        &slskr_client::capabilities::PeerCapabilityEnvelope::new(
            slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge,
            "replayed-under-another-user",
            descriptor.clone(),
        ),
    )
    .expect("replayed capability message");
    let replay_error =
        crate::handle_peer_message(&state, replay, Some("attacker"), |_| async { Ok(()) })
            .await
            .expect_err("reject capability peer ID alias");
    assert!(replay_error.contains("already registered to another username"));

    let mut tampered = descriptor;
    tampered.signature = Some([0_u8; 64]);
    let invalid = slskr_client::capabilities::peer_capability_message(
        &slskr_client::capabilities::PeerCapabilityEnvelope::new(
            slskr_client::capabilities::PeerCapabilityMessageType::Acknowledge,
            "04040404040404040404040404040404",
            tampered,
        ),
    )
    .expect("tampered capability message");
    let error =
        crate::handle_peer_message(&state, invalid, Some("mesh-source"), |_| async { Ok(()) })
            .await
            .expect_err("reject invalid descriptor");
    assert!(error.contains("signature is invalid"));
    assert_eq!(state.mesh.read().await.capability_records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn disconnected_server_endpoint_shape_matches_each_frozen_target() {
    for (target, expected_address, expected_endpoint) in [
        ("slskd", None, None),
        ("slskdn", Some(""), Some("255.255.255.255:0")),
    ] {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
        for path in ["/api/v0/server", "/api/v0/application"] {
            let response = crate::route_http_request("GET", path, None, "", &state)
                .await
                .expect("server state response");
            assert_eq!(response.status, "200 OK", "{target} {path}");
            assert_eq!(
                response.content_type, "application/json; charset=utf-8",
                "{target} {path}"
            );
            let body = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
            let server = if path.ends_with("/application") {
                &body["server"]
            } else {
                &body
            };
            assert_eq!(
                server.get("address").and_then(serde_json::Value::as_str),
                expected_address,
                "{target} {path}"
            );
            assert_eq!(
                server.get("ipEndPoint").and_then(serde_json::Value::as_str),
                expected_endpoint,
                "{target} {path}"
            );
        }
    }
}
