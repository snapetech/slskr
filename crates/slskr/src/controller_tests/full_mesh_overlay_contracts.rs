//! Controller overlay gateway contract tests.

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
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_overlay_gateway_populated_gets() {
    use sha2::Digest as _;
    use slskr_client::overlay::{
        CloseTunnelRequest, GetTunnelDataRequest, MeshHello, MeshServiceCall, OpenTunnelRequest,
        OpenTunnelResponse, TunnelDataRequest, TunnelDataResponse, FEATURE_MESH_SERVICE,
    };

    let root = std::env::temp_dir().join(format!(
        "slskr-private-gateway-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("gateway state directory");
    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": true,
            "enableDht": true,
        },
        "feature": {
            "mesh": true,
            "pods": true,
            "virtualSoulfind": true,
        }
    });
    let (mut state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string())
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "member=127.0.0.1:1"),
    );
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    Arc::get_mut(&mut state)
        .expect("unshared state")
        .private_gateway = Some(gateway.clone());
    Arc::get_mut(&mut state)
        .expect("unshared state")
        .config
        .trusted_mesh_peers
        .push(crate::TrustedMeshPeer {
            peer_id: "tester".to_owned(),
            username: "tester".to_owned(),
            overlay_endpoint: endpoint,
            certificate_sha256: certificate_pin,
            range_endpoint: None,
        });
    let mesh_content = b"frozen mesh content bytes";
    let mesh_content_path = root.join("mesh-content.flac");
    std::fs::write(&mesh_content_path, mesh_content).expect("write mesh content fixture");
    add_test_share(
        &state,
        "Virtual/MeshContent.flac",
        &mesh_content_path,
        mesh_content.len() as u64,
    )
    .await;
    let mesh_content_hash = hex::encode(sha2::Sha256::digest(mesh_content));
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "mesh-content-key".to_owned(),
                file_sha256: mesh_content_hash,
                size: mesh_content.len() as u64,
                music_brainz_id: "recording-1".to_owned(),
                ..Default::default()
            }])
            .expect("merge mesh content hash");
        discovery
            .merge_shadow_records(vec![crate::content_discovery::ShadowIndexRecord {
                recording_id: "recording-1".to_owned(),
                peer_ids: vec!["peer-hint".to_owned()],
                updated_at: 0,
            }])
            .expect("merge shadow index fixture");
    }

    let echo_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("echo listener");
    let echo_port = echo_listener.local_addr().unwrap().port();
    let echo = tokio::spawn(async move {
        let (mut stream, _) = echo_listener.accept().await.expect("echo accept");
        let mut request = [0_u8; 4];
        tokio::io::AsyncReadExt::read_exact(&mut stream, &mut request)
            .await
            .expect("echo read");
        assert_eq!(&request, b"ping");
        tokio::io::AsyncWriteExt::write_all(&mut stream, b"pong")
            .await
            .expect("echo write");
        let _ = echo_listener.accept().await.expect("legacy echo accept");
    });

    let create = crate::route_http_request(
        "POST",
        "/api/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-gateway","name":"Gateway","isPublic":true,"requireApproval":false,"channels":[{{"channelId":"general","kind":0,"name":"General"}}],"capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"127.0.0.1","port":{},"protocol":"tcp","allowPublic":false}}]}}}}}}"#,
            hex::encode(certificate_pin),
            echo_port
        ),
        &state,
    )
    .await
    .expect("create gateway pod");
    assert_eq!(create.status, "201 Created", "{}", create.body);
    *state.runtime_credentials.write().await =
        Some(crate::LoginCredentials::default_client("member", "secret"));
    let join = crate::route_http_request("POST", "/api/pods/pod-gateway/join", None, "{}", &state)
        .await
        .expect("join gateway pod");
    assert_eq!(join.status, "200 OK", "{}", join.body);
    *state.runtime_credentials.write().await = None;

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "member",
        vec!["mesh_sync".to_owned()],
        Vec::new(),
        std::time::Duration::from_secs(300),
        &remote_key,
        std::time::SystemTime::now(),
    )
    .map(|descriptor| descriptor.with_overlay_port(Some(endpoint.port())))
    .and_then(|descriptor| descriptor.sign(&remote_key))
    .expect("member capability");
    state
        .mesh
        .write()
        .await
        .update_capability(descriptor)
        .expect("register member capability");
    let local_descriptor = crate::local_capability_descriptor(&state)
        .await
        .expect("local capability descriptor");
    state
        .mesh
        .write()
        .await
        .update_capability(local_descriptor)
        .expect("register local capability");
    crate::remember_peer_endpoint(
        &state,
        crate::PeerAddress {
            username: "tester".to_owned(),
            ip: std::net::Ipv4Addr::LOCALHOST,
            port: 1,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
    let routed_message_id = crate::route_pod_message_to_peer(
        &state,
        &serde_json::json!({
            "messageId": "routed-message",
            "podId": "pod-gateway",
            "channelId": "general",
            "body": "routed through trusted mesh",
            "signature": "",
        }),
        "tester",
    )
    .await
    .expect("trusted mesh pod route");
    assert!(!routed_message_id.is_empty());

    let mut hello = MeshHello::new(
        "member",
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        "gateway-test-nonce",
    )
    .expect("hello");
    hello
        .authenticate(&remote_key, &certificate_pin)
        .expect("authenticate hello");
    let mut client = slskr_client::overlay::connect_tls_overlay(endpoint, certificate_pin, hello)
        .await
        .expect("connect gateway");
    assert_eq!(client.remote_username, "tester");

    let pods_reply = client
        .call(&MeshServiceCall::new("pods-list", "pods", "List", Vec::new()).unwrap())
        .await
        .expect("pods list reply");
    assert_eq!(pods_reply.status_code, 0, "{:?}", pods_reply.error_message);
    let pod_list = serde_json::from_slice::<serde_json::Value>(&pods_reply.payload).unwrap();
    assert_eq!(pod_list[0]["podId"], "pod-gateway");

    let post_reply = client
        .call(
            &MeshServiceCall::new(
                "pods-post",
                "pods",
                "PostMessage",
                br#"{"PodId":"pod-gateway","ChannelId":"general","Body":"mesh hello"}"#.to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("pods post reply");
    assert_eq!(post_reply.status_code, 0, "{:?}", post_reply.error_message);
    let messages_reply = client
        .call(
            &MeshServiceCall::new(
                "pods-messages",
                "pods",
                "GetMessages",
                br#"{"PodId":"pod-gateway","ChannelId":"general"}"#.to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("pods messages reply");
    assert_eq!(
        messages_reply.status_code, 0,
        "{:?}",
        messages_reply.error_message
    );
    let messages = serde_json::from_slice::<serde_json::Value>(&messages_reply.payload).unwrap();
    assert!(messages
        .as_array()
        .unwrap()
        .iter()
        .any(|message| message["senderPeerId"] == "member" && message["body"] == "mesh hello"));
    assert!(messages
        .as_array()
        .unwrap()
        .iter()
        .any(|message| message["senderPeerId"] == "tester"
            && message["body"] == "routed through trusted mesh"));

    let shadow_reply = client
        .call(
            &MeshServiceCall::new(
                "shadow-query",
                "shadow-index",
                "QueryByMbid",
                br#"{"MBID":"recording-1"}"#.to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("shadow-index reply");
    assert_eq!(
        shadow_reply.status_code, 0,
        "{:?}",
        shadow_reply.error_message
    );
    let shadow = serde_json::from_slice::<serde_json::Value>(&shadow_reply.payload).unwrap();
    assert_eq!(shadow["MBID"], "recording-1");
    assert_eq!(shadow["PeerCount"], 1);
    assert_eq!(
        shadow["CanonicalVariants"][0]["SizeBytes"],
        mesh_content.len()
    );

    let content_reply = client
        .call(
            &MeshServiceCall::new(
                "content-range",
                "MeshContent",
                "GetByContentId",
                br#"{"contentId":"Virtual/MeshContent.flac","range":{"offset":7,"length":4}}"#
                    .to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("mesh content reply");
    assert_eq!(
        content_reply.status_code, 0,
        "{:?}",
        content_reply.error_message
    );
    assert_eq!(content_reply.payload, b"mesh");

    let open = OpenTunnelRequest::new(
        "pod-gateway",
        "127.0.0.1",
        echo_port,
        None,
        "open-tunnel-nonce",
    )
    .expect("open request");
    let reply = client
        .call(
            &MeshServiceCall::new(
                "open",
                "private-gateway",
                "OpenTunnel",
                serde_json::to_vec(&open).unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("open reply");
    assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);
    let opened: OpenTunnelResponse = serde_json::from_slice(&reply.payload).unwrap();

    // The frozen MeshController transport projection reads active DHT
    // and overlay session counts from live services.  This open tunnel
    // is the real populated overlay state used by the following checks.
    let mesh_transport =
        crate::route_http_request("GET", "/api/v0/mesh/transport", None, "", &state)
            .await
            .expect("mesh transport with an open overlay session");
    let mesh_transport_json =
        serde_json::from_str::<serde_json::Value>(&mesh_transport.body).unwrap();
    let mesh_transport_keys = mesh_transport_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    assert_eq!(mesh_transport.status, "200 OK");
    assert_eq!(mesh_transport.content_type, "application/json");
    assert_eq!(
        mesh_transport_keys,
        BTreeSet::from(["dht".to_owned(), "natType".to_owned(), "overlay".to_owned(),])
    );
    assert_eq!(mesh_transport_json["dht"], 0);
    assert_eq!(mesh_transport_json["overlay"], 1);
    assert_eq!(mesh_transport_json["natType"], "Unknown");

    // Matches the oracle's real ServerStatsResponse.ActiveConnections:
    // must reflect this real, currently-open tunnel, not a hardcoded
    // 0 regardless of live connection state.
    let overlay_stats = crate::route_http_request("GET", "/api/v0/overlay/stats", None, "", &state)
        .await
        .expect("overlay stats with an open tunnel");
    let overlay_stats_json =
        serde_json::from_str::<serde_json::Value>(&overlay_stats.body).unwrap();
    assert_eq!(
        overlay_stats_json["activeConnections"], 1,
        "{overlay_stats_json}"
    );
    assert_eq!(
        overlay_stats_json["server"]["activeConnections"], 1,
        "{overlay_stats_json}"
    );

    // Matches the oracle's real overlay/mesh session list -- report the
    // authenticated TLS session, not tunnel ownership state.
    let overlay_connections =
        crate::route_http_request("GET", "/api/v0/overlay/connections", None, "", &state)
            .await
            .expect("overlay connections with an open tunnel");
    let overlay_connections_json =
        serde_json::from_str::<serde_json::Value>(&overlay_connections.body).unwrap();
    assert_eq!(overlay_connections_json.as_array().unwrap().len(), 1);
    let connection = &overlay_connections_json[0];
    assert_eq!(connection["username"], "member");
    assert_eq!(connection["address"], "127.0.0.1");
    assert!(connection["port"].as_u64().is_some_and(|port| port > 0));
    assert_eq!(
        connection["features"],
        serde_json::json!([FEATURE_MESH_SERVICE])
    );
    assert!(connection["connectedAt"]
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some());
    assert!(connection["lastActivity"]
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some());
    assert!(connection["certificateThumbprint"].is_null());
    assert_eq!(connection["version"], 1);
    assert_eq!(connection["isOutbound"], false);

    let reply = client
        .call(
            &MeshServiceCall::new(
                "send",
                "private-gateway",
                "TunnelData",
                serde_json::to_vec(&TunnelDataRequest {
                    tunnel_id: opened.tunnel_id.clone(),
                    data: b"ping".to_vec(),
                })
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("send reply");
    assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);

    let received = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let reply = client
                .call(
                    &MeshServiceCall::new(
                        uuid::Uuid::new_v4().to_string(),
                        "private-gateway",
                        "GetTunnelData",
                        serde_json::to_vec(&GetTunnelDataRequest {
                            tunnel_id: opened.tunnel_id.clone(),
                        })
                        .unwrap(),
                    )
                    .unwrap(),
                )
                .await
                .expect("receive reply");
            assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);
            let response: TunnelDataResponse = serde_json::from_slice(&reply.payload).unwrap();
            if !response.data.is_empty() {
                break response.data;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("receive timeout");
    assert_eq!(received, b"pong");

    let reply = client
        .call(
            &MeshServiceCall::new(
                "close",
                "private-gateway",
                "CloseTunnel",
                serde_json::to_vec(&CloseTunnelRequest {
                    tunnel_id: opened.tunnel_id,
                })
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("close reply");
    assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);

    let overlay_stats_after_close =
        crate::route_http_request("GET", "/api/v0/overlay/stats", None, "", &state)
            .await
            .expect("overlay stats after closing the tunnel");
    let overlay_stats_after_close_json =
        serde_json::from_str::<serde_json::Value>(&overlay_stats_after_close.body).unwrap();
    assert_eq!(
        overlay_stats_after_close_json["activeConnections"], 0,
        "{overlay_stats_after_close_json}"
    );
    let overlay_connections_after_close =
        crate::route_http_request("GET", "/api/v0/overlay/connections", None, "", &state)
            .await
            .expect("overlay connections after closing the tunnel");
    let overlay_connections_after_close_json =
        serde_json::from_str::<serde_json::Value>(&overlay_connections_after_close.body).unwrap();
    assert_eq!(
        overlay_connections_after_close_json[0]["username"], "member",
        "closing a tunnel must not remove its still-open overlay session"
    );

    let mut client_stream = client.into_inner();
    tokio::io::AsyncWriteExt::shutdown(&mut client_stream)
        .await
        .expect("close overlay client connection");
    let overlay_connections_after_client_close =
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let response = crate::route_http_request(
                    "GET",
                    "/api/v0/overlay/connections",
                    None,
                    "",
                    &state,
                )
                .await
                .expect("overlay connections after client close");
                let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
                if value.as_array().is_some_and(Vec::is_empty) {
                    break value;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("overlay connection cleanup timeout");
    assert_eq!(
        overlay_connections_after_client_close,
        serde_json::json!([])
    );

    let mut second_hello = MeshHello::new(
        "member",
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        "second-gateway-test-nonce",
    )
    .expect("second hello");
    second_hello
        .authenticate(&remote_key, &certificate_pin)
        .expect("authenticate second hello");
    let mut second_client =
        slskr_client::overlay::connect_tls_overlay(endpoint, certificate_pin, second_hello)
            .await
            .expect("connect second authenticated gateway client");
    let second_reply = second_client
        .call(
            &MeshServiceCall::new(
                "legacy-open",
                "private-gateway",
                "OpenTunnel",
                serde_json::to_vec(
                    &OpenTunnelRequest::new(
                        "pod-gateway",
                        "127.0.0.1",
                        echo_port,
                        None,
                        "legacy-open-nonce",
                    )
                    .unwrap(),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("second gateway reply");
    assert_eq!(
        second_reply.status_code, 0,
        "{:?}",
        second_reply.error_message
    );
    echo.await.expect("echo task");

    let ledger = [
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/overlay/stats",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/overlay/connections",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/mesh/transport",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("overlay_gateway_populated_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    gateway_server.abort();
    let _ = std::fs::remove_dir_all(root);
}
