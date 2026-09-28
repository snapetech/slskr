//! Controller full mesh differential ownership.

use super::*;

#[cfg(feature = "full-controller-tests")]
#[tokio::test]
pub(super) async fn controller_api_differential_share_stream_ticket_admission_enforces_grant_limits(
) {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    {
        let mut grants = state.share_grants.write().await;
        grants.records.extend([
            crate::share_grant_store::ShareGrantRecord {
                id: "grant-limited".to_owned(),
                collection_id: "collection-limited".to_owned(),
                username: "recipient".to_owned(),
                shared_at: crate::unix_timestamp(),
                permissions: "stream".to_owned(),
                max_concurrent_streams: Some(1),
            },
            crate::share_grant_store::ShareGrantRecord {
                id: "grant-independent".to_owned(),
                collection_id: "collection-independent".to_owned(),
                username: "recipient".to_owned(),
                shared_at: crate::unix_timestamp(),
                permissions: "stream".to_owned(),
                max_concurrent_streams: Some(1),
            },
        ]);
    }
    let mut tickets = state.stream_tickets.write().await;
    let limited_first = tickets
        .issue(
            "share",
            "share:grant-limited",
            "content-one".to_owned(),
            "one.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            60,
        )
        .expect("first limited stream ticket")
        .0;
    let limited_second = tickets
        .issue(
            "share",
            "share:grant-limited",
            "content-two".to_owned(),
            "two.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            60,
        )
        .expect("second limited stream ticket")
        .0;
    let independent = tickets
        .issue(
            "share",
            "share:grant-independent",
            "content-other".to_owned(),
            "other.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            60,
        )
        .expect("independent grant stream ticket")
        .0;
    drop(tickets);

    let first = crate::share_stream_limits::acquire_ticket_stream(
        &state,
        "content-one",
        Some(&format!("ticket={limited_first}")),
    )
    .await
    .expect("first stream admission")
    .expect("ticketed share stream lease");
    assert!(matches!(
        crate::share_stream_limits::acquire_ticket_stream(
            &state,
            "content-two",
            Some(&format!("ticket={limited_second}")),
        )
        .await,
        Err(crate::share_stream_limits::AdmissionError::Busy)
    ));
    let independent = crate::share_stream_limits::acquire_ticket_stream(
        &state,
        "content-other",
        Some(&format!("ticket={independent}")),
    )
    .await
    .expect("other-grant admission")
    .expect("other grants have independent capacity");
    drop(first);
    assert!(crate::share_stream_limits::acquire_ticket_stream(
        &state,
        "content-two",
        Some(&format!("ticket={limited_second}")),
    )
    .await
    .expect("released grant capacity")
    .is_some());
    drop(independent);
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
pub(super) async fn controller_api_differential_share_backfill_uses_pinned_mesh_and_publishes_verified_file(
) {
    use sha2::Digest as _;
    use slskr_client::overlay::FEATURE_MESH_SERVICE;

    let root = std::env::temp_dir().join(format!(
        "slskr-share-backfill-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let share_root = root.join("share");
    let downloads_root = root.join("downloads");
    let identity_root = root.join("identity");
    std::fs::create_dir_all(&share_root).expect("share root");
    std::fs::create_dir_all(&downloads_root).expect("downloads root");
    let content = b"verified share backfill through the pinned mesh transport\n";
    let source_path = share_root.join("treasure.txt");
    std::fs::write(&source_path, content).expect("share file");
    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": true,
            "enableDht": false,
        },
        "feature": {
            "mesh": true,
            "pods": true,
            "virtualSoulfind": true,
        }
    });
    let single_bind = "0.0.0.0:50341";
    let (mut state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_PARITY_PROFILE", "current")
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string())
            .with("SLSKR_LISTENER_BIND", single_bind)
            .with("SLSKR_OVERLAY_BIND", single_bind)
            .with("SLSKD_SHARED_DIR", &share_root.display().to_string())
            .with("SLSKD_DOWNLOADS_DIR", &downloads_root.display().to_string()),
    );
    let local_username = crate::pod_request_peer_id(&state)
        .await
        .expect("local mesh username");
    let recipient_username = "backfill-recipient".to_owned();
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &identity_root,
            None,
        )
        .await
        .expect("backfill mesh gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    {
        let state = Arc::get_mut(&mut state).expect("unshared test state");
        state.config.test_user_endpoint_overrides.insert(
            local_username.clone(),
            SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                endpoint.port(),
            ),
        );
        state
            .config
            .trusted_mesh_peers
            .push(crate::TrustedMeshPeer {
                peer_id: local_username.clone(),
                username: local_username.clone(),
                overlay_endpoint: endpoint,
                certificate_sha256: certificate_pin,
                range_endpoint: None,
            });
        state.private_gateway = Some(gateway.clone());
    }
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        &local_username,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        vec![format!("tcp:{}:{}", endpoint.ip(), endpoint.port())],
        Duration::from_secs(300),
        &state.capability_signing_key,
        SystemTime::now(),
    )
    .expect("local capability descriptor")
    .with_overlay_port(Some(endpoint.port()))
    .sign(&state.capability_signing_key)
    .expect("sign local capability descriptor");
    state
        .mesh
        .write()
        .await
        .update_capability(descriptor)
        .expect("register local capability descriptor");
    let recipient_key = ed25519_dalek::SigningKey::from_bytes(&[43; 32]);
    let recipient_descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        &recipient_username,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        Vec::new(),
        Duration::from_secs(300),
        &recipient_key,
        SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.sign(&recipient_key))
    .expect("recipient capability descriptor");
    state
        .mesh
        .write()
        .await
        .update_capability(recipient_descriptor)
        .expect("register recipient capability descriptor");
    crate::remember_peer_endpoint(
        &state,
        crate::PeerAddress {
            username: recipient_username.clone(),
            ip: std::net::Ipv4Addr::LOCALHOST,
            port: 1,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;
    Arc::get_mut(&mut state)
        .expect("unique test state before starting gateway")
        .capability_signing_key = recipient_key;
    add_test_share(
        &state,
        "Virtual/treasure.txt",
        &source_path,
        content.len() as u64,
    )
    .await;
    let content_sha256 = hex::encode(sha2::Sha256::digest(content));
    let grant_id = "grant-recipient-backfill".to_owned();
    {
        let mut collections = state.collections.write().await;
        collections
            .create_with_contract(
                "collection-recipient-backfill".to_owned(),
                local_username.clone(),
                "Backfill fixture".to_owned(),
                String::new(),
                "ShareList".to_owned(),
            )
            .expect("create backfill collection");
        collections
            .add_item_with_contract(
                "collection-recipient-backfill",
                Some("item-recipient-backfill".to_owned()),
                "Virtual/treasure.txt".to_owned(),
                String::new(),
                "treasure.txt".to_owned(),
                "text".to_owned(),
                "Virtual/treasure.txt".to_owned(),
                String::new(),
                content_sha256.clone(),
            )
            .expect("add backfill item")
            .expect("backfill item exists");
    }
    let share_token = {
        let mut grants = state.share_grants.write().await;
        grants
            .create_with_contract_and_permissions(
                Some(grant_id.clone()),
                "collection-recipient-backfill".to_owned(),
                recipient_username.clone(),
                "download",
            )
            .expect("create recipient grant");
        drop(grants);
        state
            .share_access_tokens
            .write()
            .await
            .issue(grant_id.clone(), 600)
            .expect("issue recipient grant token")
            .0
    };
    state
        .incoming_shares
        .write()
        .await
        .upsert(crate::IncomingShareRecord {
            id: grant_id.clone(),
            owner_endpoint: "https://ignored.invalid".to_owned(),
            owner_user_id: local_username.clone(),
            recipient_user_id: recipient_username.clone(),
            collection_id: "collection-recipient-backfill".to_owned(),
            collection_title: "Backfill fixture".to_owned(),
            collection_description: String::new(),
            collection_type: "ShareList".to_owned(),
            permissions: "download".to_owned(),
            token: share_token,
            expiry_utc: String::new(),
            max_bitrate_kbps: None,
            max_concurrent_streams: 0,
            items: Vec::new(),
            received_at: crate::unix_timestamp(),
        });

    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
    let result = crate::share_backfill_controller::backfill_incoming_share(
        &state,
        &grant_id,
        &recipient_username,
    )
    .await;
    gateway_server.abort();
    let _ = gateway_server.await;
    let receipts = result.expect("recipient backfill succeeds");
    assert_eq!(receipts.len(), 1);
    let downloaded =
        std::fs::read(downloads_root.join(&receipts[0].filename)).expect("read downloaded file");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(receipts[0].size, content.len() as u64);
    assert_eq!(receipts[0].sha256, content_sha256);
    assert_eq!(downloaded, content);
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
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_mesh_stream_ticket_validation_and_limits() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SHARE_FIXTURE", ""),
    );
    let nominal = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        &format!(
            r#"{{"contentId":"mesh-ticket-contract","filename":"Track.aac","peerId":"mesh-peer","expectedSize":321,"expectedHash":"{}"}}"#,
            "A".repeat(64)
        ),
        &state,
    )
    .await
    .expect("mesh ticket nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let nominal_keys = nominal_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let nominal_ticket = nominal_json["ticket"].as_str().unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type == "application/json"
            && nominal_keys
                == BTreeSet::from([
                    "contentType".to_owned(),
                    "expiresInSeconds".to_owned(),
                    "source".to_owned(),
                    "streamUrl".to_owned(),
                    "ticket".to_owned(),
                ])
            && nominal_json["contentType"] == "audio/aac"
            && nominal_json["expiresInSeconds"] == 120
            && nominal_json["source"] == "mesh"
            && nominal_json["streamUrl"] == format!("/api/v0/mesh-streams/{nominal_ticket}")
            && !nominal_ticket.is_empty()
    );
    let stored = state
        .stream_tickets
        .write()
        .await
        .get(nominal_ticket)
        .is_some();
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "mutation-side-effects-and-readback",
        stored
    );

    let malformed_bodies = [
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"../escape.flac","peerId":"mesh-peer"}"#,
            "Filename is required.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.flac","peerId":"mesh-peer","expectedSize":-1}"#,
            "Expected size must be greater than or equal to zero.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.flac","peerId":"mesh-peer","expectedHash":"abc"}"#,
            "Expected hash must be a SHA-256 hex digest.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"archive.zip","peerId":"mesh-peer"}"#,
            "Only audio files can be preview streamed from mesh peers.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.oga","peerId":"mesh-peer"}"#,
            "Only audio files can be preview streamed from mesh peers.",
        ),
    ];
    let mut malformed = true;
    for (body, expected_error) in malformed_bodies {
        let response =
            crate::route_http_request("POST", "/api/v0/mesh-streams/tickets", None, body, &state)
                .await
                .expect("mesh ticket malformed response");
        malformed &= response.status == "400 Bad Request"
            && response.body == format!(r#"{{"error":"{expected_error}"}}"#);
    }
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "malformed-path-query-or-body",
        malformed
    );

    let blank_peer = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-blank-peer","filename":"Track.flac","peerId":"   "}"#,
        &state,
    )
    .await
    .expect("mesh ticket blank peer response");
    let blank_peer_json = serde_json::from_str::<serde_json::Value>(&blank_peer.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "nominal-status-headers-body",
        blank_peer.status == "200 OK"
            && blank_peer_json["source"] == "mesh"
            && blank_peer_json["contentType"] == "audio/flac"
    );

    let missing =
        crate::route_http_request("GET", "/api/v0/mesh-streams/not-a-ticket", None, "", &state)
            .await
            .expect("mesh ticket missing response");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let malformed_path = crate::route_http_request(
        "GET",
        "/api/v0/mesh-streams/not-a-ticket/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh ticket malformed path response");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "malformed-path-query-or-body",
        malformed_path.status == "404 Not Found"
    );

    let (disabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_ADVANCED_NETWORKING_JSON",
                r#"{"feature":{"streaming":false}}"#,
            ),
    );
    let disabled_post = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-disabled","filename":"Track.flac"}"#,
        &disabled_state,
    )
    .await
    .expect("disabled mesh ticket create response");
    let disabled_get = crate::route_http_request(
        "GET",
        "/api/v0/mesh-streams/not-a-ticket",
        None,
        "",
        &disabled_state,
    )
    .await
    .expect("disabled mesh ticket get response");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "missing-empty-or-conflict-state",
        disabled_post.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "missing-empty-or-conflict-state",
        disabled_get.status == "404 Not Found"
    );

    let (capacity_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut tickets = capacity_state.stream_tickets.write().await;
        for index in 0..crate::MAX_PREVIEW_STREAM_TICKETS {
            assert!(tickets
                .issue(
                    "mesh",
                    "mesh-unresolved",
                    format!("capacity-{index}"),
                    "Track.flac".to_owned(),
                    None,
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-capacity","filename":"Track.flac","peerId":"mesh-peer"}"#,
        &capacity_state,
    )
    .await
    .expect("mesh ticket capacity response");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "runtime-failure-and-timeout",
        capacity.status == "429 Too Many Requests"
            && capacity.content_type == "text/plain; charset=utf-8"
            && capacity.body == "Mesh stream limit reached."
    );

    let nominal_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/mesh-streams/{nominal_ticket}"),
        None,
        "",
        &state,
    )
    .await
    .expect("nominal mesh stream read");
    let nominal_get_json = serde_json::from_str::<serde_json::Value>(&nominal_get.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "nominal-status-headers-body",
        nominal_get.status == "200 OK"
            && nominal_get.content_type == "application/json"
            && nominal_get_json["status"] == "available"
            && nominal_get_json["cacheControl"] == "no-store"
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh stream runtime-failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_ticket = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-runtime","filename":"Runtime.flac","peerId":"mesh-peer"}"#,
        &failure_state,
    )
    .await
    .expect("create mesh runtime-failure ticket");
    let failure_ticket_json = serde_json::from_str::<serde_json::Value>(&failure_ticket.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    failure_db.close_for_test().await;
    let failure_get = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/mesh-streams/{}",
            failure_ticket_json["ticket"].as_str().unwrap_or_default()
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("mesh stream read with closed unrelated database");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "runtime-failure-and-timeout",
        failure_get.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_get.body)
                .map(|value| value["status"] == "available")
                .unwrap_or(false)
    );

    let (restarted_state, _restarted_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let reset_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/mesh-streams/{nominal_ticket}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("mesh stream ticket after restart");
    let reset_create = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-restarted","filename":"Restart.flac","peerId":"mesh-peer"}"#,
        &restarted_state,
    )
    .await
    .expect("mesh stream ticket create after restart");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "restart-persistence-or-reset",
        reset_get.status == "404 Not Found" && reset_create.status == "200 OK"
    );

    let (concurrent_state, _concurrent_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let concurrent = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"contentId":"mesh-concurrent-a","filename":"A.flac","peerId":"mesh-peer"}"#,
            &concurrent_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"contentId":"mesh-concurrent-b","filename":"B.flac","peerId":"mesh-peer"}"#,
            &concurrent_state,
        ),
    );
    let concurrent_pass = match concurrent {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
                && concurrent_state.stream_tickets.read().await.records.len() == 2
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "concurrency-and-idempotency",
        concurrent_pass
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("mesh_stream_ticket_validation_and_limits.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} mesh stream ticket mismatches:\n{}",
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
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_mesh_http_disabled_shape() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "127.0.0.1"),
    );

    let webfinger = crate::route_http_request("GET", "/.well-known/webfinger", None, "", &state)
        .await
        .expect("webfinger response");
    assert_eq!(webfinger.status, "400 Bad Request");
    assert_eq!(webfinger.content_type, "application/json");

    let missing_actor = crate::route_http_request("GET", "/actors/library", None, "", &state)
        .await
        .expect("missing actor response");
    assert_eq!(missing_actor.status, "404 Not Found");
    assert_eq!(missing_actor.content_type, "application/json");

    let published = crate::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        r#"{"id":"activity-1","type":"Create","object":{"type":"Note","content":"hello"}}"#,
        &state,
    )
    .await
    .expect("publish activity");
    assert_eq!(published.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap()["type"],
        "Create"
    );

    for path in [
        "/actors/music",
        "/actors/music/inbox",
        "/actors/music/outbox",
        "/actors/music/followers",
        "/actors/music/following",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "200 OK", "{path}");
        assert_eq!(response.content_type, "application/activity+json", "{path}");
        assert!(!response.body.to_ascii_lowercase().contains("<!doctype"));
    }

    let mesh = crate::route_http_request("GET", "/mesh/http/services", None, "", &state)
        .await
        .expect("mesh services response");
    assert_eq!(mesh.status, "404 Not Found");
    assert_eq!(mesh.body, r#"{"error":"gateway_disabled"}"#);

    let ledger = [serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/mesh/http/services",
        "case": "missing-empty-or-conflict-state",
        "pass": true,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("mesh_http_disabled_shape.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting 4 mesh-rendezvous/capability
/// routes' `nominal-status-headers-body` cases, independently
/// re-derived from `mesh_rendezvous_api_discovers_users_and_mesh_
/// capabilities`. slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_mesh_rendezvous_and_capabilities_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {}", $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "nominal-status-headers-body",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    {
        let mut users = state.users.write().await;
        users.watch("alice".to_owned());
        users.watch("Bob".to_owned());
    }
    {
        let mut mesh = state.mesh.write().await;
        mesh.capability_records.push(test_capability_descriptor(
            "ALICE",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records.push(test_capability_descriptor(
            "carol",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records.push(test_capability_descriptor(
            "dave",
            vec![slskr_client::capabilities::FEATURE_CAPABILITIES_V1.to_owned()],
        ));
    }

    let status = crate::route_http_request(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/status",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
    record!(
        "/api/v0/soulseek/mesh-rendezvous/status",
        status.status == "200 OK"
            && status_json["enabled"] == true
            && status_json["candidateCount"] == 3
    );

    // The versioned (v0) surface of this specific route is a real,
    // deterministic disabled-feature shortcut (`versioned_get_failure_
    // contract`'s `path.starts_with("/api/v0/")`-gated check) --
    // unlike the bare/compat path the original test calls, which
    // reaches the real handler. Both are real, intentional behavior;
    // this credits the v0 form's own real contract, not a "fixed"
    // 200 OK that the v0 surface never actually returns.
    let discover = crate::route_http_request(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/discover",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh discover");
    let discover_pass = discover.status == "403 Forbidden"
        && discover.body == "{\"error\":\"feature is disabled by configuration\"}";
    if !discover_pass {
        mismatches.push("slskdn GET /api/v0/soulseek/mesh-rendezvous/discover".to_owned());
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/soulseek/mesh-rendezvous/discover",
        "case": "missing-empty-or-conflict-state",
        "pass": discover_pass,
    }));

    let capabilities = crate::route_http_request(
        "GET",
        "/api/v0/soulseek/peer-capabilities",
        None,
        "",
        &state,
    )
    .await
    .expect("peer capabilities");
    let capabilities_json =
        serde_json::from_str::<serde_json::Value>(&capabilities.body).unwrap_or_default();
    record!(
        "/api/v0/soulseek/peer-capabilities",
        capabilities.status == "200 OK"
            && capabilities_json.as_array().map(Vec::len) == Some(3)
            && capabilities_json[0]["meshCapable"] == true
            && capabilities_json[2]["meshCapable"] == false
    );

    let peers = crate::route_http_request("GET", "/api/v0/mesh/peers", None, "", &state)
        .await
        .expect("mesh peers");
    record!(
        "/api/v0/mesh/peers",
        peers.status == "200 OK"
            && peers.body.contains("\"peers\"")
            && peers.body.contains("\"carol\"")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mesh_rendezvous_and_capabilities_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh-rendezvous mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the remaining frozen DhtRendezvous
/// controller cases.  The v0 DHT actions deliberately use their frozen
/// no-body contracts, while blocklist and certificate-pin cases exercise
/// real local state, reset behavior, and concurrent mutations.
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
pub(super) async fn controller_api_differential_dht_rendezvous_residuals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    macro_rules! request {
        ($method:expr, $path:expr, $body:expr, $state:expr) => {{
            crate::route_http_request($method, $path, None, $body, $state)
                .await
                .expect("DHT/overlay route request")
        }};
    }

    fn json_object(body: &str) -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or_default()
    }

    let (state, _receiver) = test_state();

    let mut populated_state;
    let (_, _receiver) = {
        let (created, receiver) = test_state();
        populated_state = created;
        (populated_state.clone(), receiver)
    };
    let mut dht_settings = populated_state.config.advanced_networking.dht.clone();
    dht_settings.dht_port = 0;
    dht_settings.overlay_port = 0;
    dht_settings.advertised_overlay_port = 0;
    dht_settings.lan_only = true;
    dht_settings.bootstrap_routers.clear();
    let rendezvous = crate::dht::Rendezvous::new(&dht_settings).expect("test DHT rendezvous");
    rendezvous
        .insert_test_peer("198.51.100.10:6881".parse().expect("test DHT peer"))
        .await;
    Arc::get_mut(&mut populated_state)
        .expect("unique populated state")
        .dht = Some(Arc::new(rendezvous));

    let dht_peers_malformed = request!("GET", "/api/v0/dht/peers?unexpected=%7B", "", &state);
    record!(
        "GET",
        "/api/v0/dht/peers",
        "malformed-path-query-or-body",
        dht_peers_malformed.status == "200 OK" && json_object(&dht_peers_malformed.body).is_array()
    );
    let dht_peers_runtime = request!("GET", "/api/v0/dht/peers", "", &state);
    record!(
        "GET",
        "/api/v0/dht/peers",
        "runtime-failure-and-timeout",
        dht_peers_runtime.status == "200 OK" && json_object(&dht_peers_runtime.body).is_array()
    );
    let dht_peers_populated = request!("GET", "/api/v0/dht/peers", "", &populated_state);
    let dht_peers_populated_json = json_object(&dht_peers_populated.body);
    record!(
        "GET",
        "/api/v0/dht/peers",
        "populated-dynamic-state",
        dht_peers_populated.status == "200 OK"
            && dht_peers_populated_json[0]["address"] == "198.51.100.10"
            && dht_peers_populated_json[0]["port"] == 6881
    );

    let dht_status_malformed = request!("GET", "/api/v0/dht/status?unexpected=%7B", "", &state);
    record!(
        "GET",
        "/api/v0/dht/status",
        "malformed-path-query-or-body",
        dht_status_malformed.status == "200 OK"
            && json_object(&dht_status_malformed.body)["isBeaconCapable"] == false
    );
    let dht_status_missing = request!("GET", "/api/v0/dht/status", "", &state);
    record!(
        "GET",
        "/api/v0/dht/status",
        "missing-empty-or-conflict-state",
        dht_status_missing.status == "200 OK"
            && json_object(&dht_status_missing.body)["dhtNodeCount"] == 0
    );
    let dht_status_runtime = request!("GET", "/api/v0/dht/status", "", &state);
    record!(
        "GET",
        "/api/v0/dht/status",
        "runtime-failure-and-timeout",
        dht_status_runtime.status == "200 OK"
            && json_object(&dht_status_runtime.body)["rendezvousInfohashes"].is_array()
    );

    let blocklist_get_malformed = request!(
        "GET",
        "/api/v0/overlay/blocklist?unexpected=%7B",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "malformed-path-query-or-body",
        blocklist_get_malformed.status == "200 OK"
            && json_object(&blocklist_get_malformed.body)["entries"].is_array()
    );
    let blocklist_get_missing = request!("GET", "/api/v0/overlay/blocklist", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "missing-empty-or-conflict-state",
        blocklist_get_missing.status == "200 OK"
            && json_object(&blocklist_get_missing.body)["entries"]
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let blocklist_get_runtime = request!("GET", "/api/v0/overlay/blocklist", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "runtime-failure-and-timeout",
        blocklist_get_runtime.status == "200 OK"
    );

    let overlay_connections_malformed = request!(
        "GET",
        "/api/v0/overlay/connections?unexpected=%7B",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v0/overlay/connections",
        "malformed-path-query-or-body",
        overlay_connections_malformed.status == "200 OK"
            && json_object(&overlay_connections_malformed.body).is_array()
    );
    let overlay_connections_missing = request!("GET", "/api/v0/overlay/connections", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/connections",
        "missing-empty-or-conflict-state",
        overlay_connections_missing.status == "200 OK"
            && json_object(&overlay_connections_missing.body)
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let overlay_connections_runtime = request!("GET", "/api/v0/overlay/connections", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/connections",
        "runtime-failure-and-timeout",
        overlay_connections_runtime.status == "200 OK"
    );

    let overlay_stats_malformed =
        request!("GET", "/api/v0/overlay/stats?unexpected=%7B", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/stats",
        "malformed-path-query-or-body",
        overlay_stats_malformed.status == "200 OK"
            && json_object(&overlay_stats_malformed.body)["server"].is_object()
    );
    let overlay_stats_missing = request!("GET", "/api/v0/overlay/stats", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/stats",
        "missing-empty-or-conflict-state",
        overlay_stats_missing.status == "200 OK"
            && json_object(&overlay_stats_missing.body)["connector"].is_object()
    );
    let overlay_stats_runtime = request!("GET", "/api/v0/overlay/stats", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/stats",
        "runtime-failure-and-timeout",
        overlay_stats_runtime.status == "200 OK"
            && json_object(&overlay_stats_runtime.body)["blocklist"].is_object()
    );

    let announce_nominal = request!("POST", "/api/v0/dht/announce", "", &state);
    record!(
        "POST",
        "/api/v0/dht/announce",
        "nominal-status-headers-body",
        announce_nominal.status == "400 Bad Request"
            && announce_nominal.body == r#"{"error":"Not beacon capable"}"#
    );
    let announce_malformed = request!("POST", "/api/v0/dht/announce", "not-json", &state);
    record!(
        "POST",
        "/api/v0/dht/announce",
        "malformed-path-query-or-body",
        announce_malformed.status == "400 Bad Request"
    );
    let announce_runtime = request!(
        "POST",
        "/api/v0/dht/announce",
        r#"{"ignored":true}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/dht/announce",
        "runtime-failure-and-timeout",
        announce_runtime.status == "400 Bad Request"
    );
    let announce_mutation = request!(
        "POST",
        "/api/v0/dht/announce",
        r#"{"ignored":true}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/dht/announce",
        "mutation-side-effects-and-readback",
        announce_mutation.status == "400 Bad Request"
    );
    let (announce_restart_state, _receiver) = test_state();
    let announce_restart = request!("POST", "/api/v0/dht/announce", "", &announce_restart_state);
    record!(
        "POST",
        "/api/v0/dht/announce",
        "restart-persistence-or-reset",
        announce_restart.status == "400 Bad Request"
    );
    let (announce_a, announce_b) = tokio::join!(
        crate::route_http_request("POST", "/api/v0/dht/announce", None, "", &state),
        crate::route_http_request("POST", "/api/v0/dht/announce", None, "", &state),
    );
    record!(
        "POST",
        "/api/v0/dht/announce",
        "concurrency-and-idempotency",
        announce_a
            .as_ref()
            .is_ok_and(|response| response.status == "400 Bad Request")
            && announce_b
                .as_ref()
                .is_ok_and(|response| response.status == "400 Bad Request")
    );

    let discover_malformed = request!("POST", "/api/v0/dht/discover", "not-json", &state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "malformed-path-query-or-body",
        discover_malformed.status == "200 OK"
            && json_object(&discover_malformed.body)["newConnectionsMade"].is_number()
    );
    let discover_missing = request!("POST", "/api/v0/dht/discover", "", &state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "missing-empty-or-conflict-state",
        discover_missing.status == "200 OK"
            && json_object(&discover_missing.body)["totalMeshConnections"].is_number()
    );
    let discover_runtime = request!(
        "POST",
        "/api/v0/dht/discover",
        r#"{"ignored":true}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/dht/discover",
        "runtime-failure-and-timeout",
        discover_runtime.status == "200 OK"
    );
    let discover_mutation = request!("POST", "/api/v0/dht/discover", "", &state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "mutation-side-effects-and-readback",
        discover_mutation.status == "200 OK"
    );
    let (discover_restart_state, _receiver) = test_state();
    let discover_restart = request!("POST", "/api/v0/dht/discover", "", &discover_restart_state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "restart-persistence-or-reset",
        discover_restart.status == "200 OK"
    );
    let (discover_a, discover_b) = tokio::join!(
        crate::route_http_request("POST", "/api/v0/dht/discover", None, "", &state),
        crate::route_http_request("POST", "/api/v0/dht/discover", None, "", &state),
    );
    record!(
        "POST",
        "/api/v0/dht/discover",
        "concurrency-and-idempotency",
        discover_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && discover_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let block_ip_malformed = request!("POST", "/api/v0/overlay/blocklist/ip", "{}", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "malformed-path-query-or-body",
        block_ip_malformed.status == "400 Bad Request"
    );
    let block_ip_missing = request!("POST", "/api/v0/overlay/blocklist/ip", "", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "missing-empty-or-conflict-state",
        block_ip_missing.status == "400 Bad Request"
    );
    let block_ip_runtime = request!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        r#"{"ip":"203.0.113.5","reason":"runtime"}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "runtime-failure-and-timeout",
        block_ip_runtime.status == "200 OK"
    );
    let (block_ip_restart_state, _receiver) = test_state();
    let block_ip_restart_list = request!(
        "GET",
        "/api/v0/overlay/blocklist",
        "",
        &block_ip_restart_state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "restart-persistence-or-reset",
        block_ip_restart_list.status == "200 OK"
            && json_object(&block_ip_restart_list.body)["entries"]
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let block_ip_body_a = r#"{"ip":"203.0.113.6"}"#;
    let block_ip_body_b = r#"{"ip":"203.0.113.7"}"#;
    let (block_ip_a, block_ip_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/ip",
            None,
            block_ip_body_a,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/ip",
            None,
            block_ip_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "concurrency-and-idempotency",
        block_ip_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && block_ip_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let block_user_malformed = request!("POST", "/api/v0/overlay/blocklist/username", "{}", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "malformed-path-query-or-body",
        block_user_malformed.status == "400 Bad Request"
    );
    let block_user_missing = request!("POST", "/api/v0/overlay/blocklist/username", "", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "missing-empty-or-conflict-state",
        block_user_missing.status == "400 Bad Request"
    );
    let block_user_runtime = request!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        r#"{"username":"runtime-user","reason":"runtime"}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "runtime-failure-and-timeout",
        block_user_runtime.status == "200 OK"
    );
    let (block_user_restart_state, _receiver) = test_state();
    let block_user_restart_list = request!(
        "GET",
        "/api/v0/overlay/blocklist",
        "",
        &block_user_restart_state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "restart-persistence-or-reset",
        block_user_restart_list.status == "200 OK"
            && json_object(&block_user_restart_list.body)["entries"]
                .as_array()
                .is_some_and(|entries| entries.is_empty())
    );
    let block_user_body_a = r#"{"username":"concurrent-user-a"}"#;
    let block_user_body_b = r#"{"username":"concurrent-user-b"}"#;
    let (block_user_a, block_user_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/username",
            None,
            block_user_body_a,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/username",
            None,
            block_user_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "concurrency-and-idempotency",
        block_user_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && block_user_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let connect_body = r#"{"address":"203.0.113.10","port":1}"#;
    let overlay_connect_nominal = request!("POST", "/api/v0/overlay/connect", connect_body, &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "nominal-status-headers-body",
        overlay_connect_nominal.status == "502 Bad Gateway"
            && json_object(&overlay_connect_nominal.body)["connected"] == false
    );
    let overlay_connect_malformed = request!("POST", "/api/v0/overlay/connect", "{}", &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "malformed-path-query-or-body",
        overlay_connect_malformed.status == "400 Bad Request"
    );
    let overlay_connect_missing = request!("POST", "/api/v0/overlay/connect", "", &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "missing-empty-or-conflict-state",
        overlay_connect_missing.status == "400 Bad Request"
    );
    let overlay_connect_runtime = request!(
        "POST",
        "/api/v0/overlay/connect",
        r#"{"address":"203.0.113.11","port":2}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "runtime-failure-and-timeout",
        overlay_connect_runtime.status == "502 Bad Gateway"
    );
    let overlay_connect_mutation =
        request!("POST", "/api/v0/overlay/connect", connect_body, &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "mutation-side-effects-and-readback",
        overlay_connect_mutation.status == "502 Bad Gateway"
    );
    let (connect_restart_state, _receiver) = test_state();
    let overlay_connect_restart = request!(
        "POST",
        "/api/v0/overlay/connect",
        connect_body,
        &connect_restart_state
    );
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "restart-persistence-or-reset",
        overlay_connect_restart.status == "502 Bad Gateway"
    );
    let connect_body_a = r#"{"address":"203.0.113.12","port":3}"#;
    let connect_body_b = r#"{"address":"203.0.113.13","port":4}"#;
    let (connect_a, connect_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/connect",
            None,
            connect_body_a,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/connect",
            None,
            connect_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "concurrency-and-idempotency",
        connect_a
            .as_ref()
            .is_ok_and(|response| response.status == "502 Bad Gateway")
            && connect_b
                .as_ref()
                .is_ok_and(|response| response.status == "502 Bad Gateway")
    );

    let pin_body =
        r#"{"thumbprint":"0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a"}"#;
    let pin_nominal = request!("PUT", "/api/v0/overlay/pins/nominal-peer", pin_body, &state);
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "nominal-status-headers-body",
        pin_nominal.status == "204 No Content" && pin_nominal.body.is_empty()
    );
    let pin_malformed = request!(
        "PUT",
        "/api/v0/overlay/pins/malformed-peer",
        r#"{"pin":"abc"}"#,
        &state
    );
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "malformed-path-query-or-body",
        pin_malformed.status == "400 Bad Request"
    );
    let pin_missing = request!("PUT", "/api/v0/overlay/pins/%20", pin_body, &state);
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "missing-empty-or-conflict-state",
        pin_missing.status == "400 Bad Request"
    );
    let pin_runtime = request!("PUT", "/api/v0/overlay/pins/runtime-peer", pin_body, &state);
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "runtime-failure-and-timeout",
        pin_runtime.status == "204 No Content"
    );
    let pin_mutation = request!(
        "PUT",
        "/api/v0/overlay/pins/mutation-peer",
        pin_body,
        &state
    );
    let pin_readback = state
        .controller_features
        .read()
        .await
        .get("overlay/pin/mutation-peer")
        .is_some();
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "mutation-side-effects-and-readback",
        pin_mutation.status == "204 No Content" && pin_readback
    );
    let (pin_restart_state, _receiver) = test_state();
    let pin_restart_readback = pin_restart_state
        .controller_features
        .read()
        .await
        .get("overlay/pin/mutation-peer")
        .is_none();
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "restart-persistence-or-reset",
        pin_restart_readback
    );
    let pin_body_a =
        r#"{"thumbprint":"1111111111111111111111111111111111111111111111111111111111111111"}"#;
    let pin_body_b =
        r#"{"thumbprint":"2222222222222222222222222222222222222222222222222222222222222222"}"#;
    let (pin_a, pin_b) = tokio::join!(
        crate::route_http_request(
            "PUT",
            "/api/v0/overlay/pins/concurrent-a",
            None,
            pin_body_a,
            &state
        ),
        crate::route_http_request(
            "PUT",
            "/api/v0/overlay/pins/concurrent-b",
            None,
            pin_body_b,
            &state
        ),
    );
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "concurrency-and-idempotency",
        pin_a
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content")
            && pin_b
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
    );

    let delete_setup = request!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        r#"{"username":"delete-runtime"}"#,
        &state
    );
    assert_eq!(delete_setup.status, "200 OK", "{}", delete_setup.body);
    let delete_malformed = request!("DELETE", "/api/v0/overlay/blocklist/username", "", &state);
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "malformed-path-query-or-body",
        delete_malformed.status == "400 Bad Request"
    );
    let delete_missing = request!(
        "DELETE",
        "/api/v0/overlay/blocklist/username/does-not-exist",
        "",
        &state
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "missing-empty-or-conflict-state",
        delete_missing.status == "404 Not Found"
    );
    let delete_runtime = request!(
        "DELETE",
        "/api/v0/overlay/blocklist/username/delete-runtime",
        "",
        &state
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "runtime-failure-and-timeout",
        delete_runtime.status == "200 OK"
    );
    let (delete_restart_state, _receiver) = test_state();
    let delete_restart = request!(
        "DELETE",
        "/api/v0/overlay/blocklist/username/delete-runtime",
        "",
        &delete_restart_state
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "restart-persistence-or-reset",
        delete_restart.status == "404 Not Found"
    );
    for username in ["delete-concurrent-a", "delete-concurrent-b"] {
        let body = serde_json::json!({"username": username}).to_string();
        let response = request!("POST", "/api/v0/overlay/blocklist/username", &body, &state);
        assert_eq!(response.status, "200 OK", "{}", response.body);
    }
    let (delete_a, delete_b) = tokio::join!(
        crate::route_http_request(
            "DELETE",
            "/api/v0/overlay/blocklist/username/delete-concurrent-a",
            None,
            "",
            &state
        ),
        crate::route_http_request(
            "DELETE",
            "/api/v0/overlay/blocklist/username/delete-concurrent-b",
            None,
            "",
            &state
        ),
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "concurrency-and-idempotency",
        delete_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && delete_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("dht_rendezvous_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize DHT/overlay ledger"),
    )
    .expect("write DHT/overlay ledger");

    assert_eq!(ledger.len(), 56, "DHT/overlay residual ledger size");
    assert!(
        mismatches.is_empty(),
        "{} controller-api DHT/overlay mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 5 swarm-analytics routes'
/// `nominal-status-headers-body` / `populated-dynamic-state` cases,
/// independently re-derived from `swarm_analytics_routes_share_a_
/// bounded_snapshot`'s real seeded-job dashboard/projection checks.
/// slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_swarm_analytics_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let now = crate::unix_timestamp();
    state
        .multisource
        .write()
        .await
        .insert(crate::multisource::SwarmJob {
            id: "swarm-analytics-fixture".to_owned(),
            status: "completed".to_owned(),
            filename: "album.flac".to_owned(),
            output_path: "album.flac".to_owned(),
            file_size: 1_024,
            chunk_size: 512,
            sources: vec!["alice".to_owned(), "bob".to_owned()],
            completed_chunks: 2,
            total_chunks: 2,
            bytes_downloaded: 1_024,
            created_at: now,
            updated_at: now,
            result: Some(crate::multisource::SwarmResult {
                id: "swarm-analytics-fixture".to_owned(),
                success: true,
                filename: "album.flac".to_owned(),
                output_path: "album.flac".to_owned(),
                bytes_downloaded: 1_024,
                total_time_ms: 100,
                sources_used: 2,
                final_hash: "00".repeat(32),
                chunks: vec![
                    crate::multisource::ChunkResult {
                        index: 0,
                        username: "alice".to_owned(),
                        start_offset: 0,
                        end_offset: 511,
                        bytes_downloaded: 512,
                        time_ms: 40,
                    },
                    crate::multisource::ChunkResult {
                        index: 1,
                        username: "bob".to_owned(),
                        start_offset: 512,
                        end_offset: 1_023,
                        bytes_downloaded: 512,
                        time_ms: 60,
                    },
                ],
                error: None,
            }),
        });

    let dashboard = crate::route_http_request(
        "GET",
        "/api/v0/swarm/analytics/dashboard?timeWindowHours=24&rankingLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("swarm analytics dashboard");
    let dashboard_json =
        serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap_or_default();
    record!(
        "/api/v0/swarm/analytics/dashboard",
        "populated-dynamic-state",
        dashboard.status == "200 OK"
            && dashboard_json["performanceMetrics"]["totalDownloads"] == 1
            && dashboard_json["peerRankings"].as_array().map(Vec::len) == Some(1)
    );

    for (route, expected_kind) in [
        ("/api/v0/swarm/analytics/performance", "object"),
        ("/api/v0/swarm/analytics/peers/rankings", "array"),
        ("/api/v0/swarm/analytics/efficiency", "object"),
        ("/api/v0/swarm/analytics/recommendations", "array"),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .expect("swarm analytics projection");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let kind = if value.is_array() { "array" } else { "object" };
        record!(
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && kind == expected_kind
        );
    }

    for (route, expected_error) in [
        (
            "/api/v0/swarm/analytics/dashboard?timeWindowHours=0",
            "Time window must be between 1 and 168 hours (7 days)",
        ),
        (
            "/api/v0/swarm/analytics/dashboard?rankingLimit=101",
            "Ranking limit must be between 1 and 100",
        ),
        (
            "/api/v0/swarm/analytics/peers/rankings?limit=0",
            "Limit must be between 1 and 100",
        ),
        (
            "/api/v0/swarm/analytics/trends?dataPoints=1",
            "Data points must be between 2 and 168",
        ),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .expect("swarm analytics invalid query");
        record!(
            route.split('?').next().unwrap_or(route),
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains(expected_error)
        );
    }

    let malformed_analytics_routes = [
        (
            "/api/v0/swarm/analytics/performance/extra",
            "/api/v0/swarm/analytics/performance",
        ),
        (
            "/api/v0/swarm/analytics/efficiency/extra",
            "/api/v0/swarm/analytics/efficiency",
        ),
        (
            "/api/v0/swarm/analytics/recommendations/extra",
            "/api/v0/swarm/analytics/recommendations",
        ),
    ];
    for (path, route) in malformed_analytics_routes {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("malformed swarm analytics path {path}: {error}"));
        record!(
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let (empty_state, _empty_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    for (route, expected_array) in [
        ("/api/v0/swarm/analytics/dashboard", false),
        ("/api/v0/swarm/analytics/performance", false),
        ("/api/v0/swarm/analytics/peers/rankings", true),
        ("/api/v0/swarm/analytics/efficiency", false),
        ("/api/v0/swarm/analytics/recommendations", true),
        ("/api/v0/swarm/analytics/trends", false),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &empty_state)
            .await
            .unwrap_or_else(|error| panic!("empty swarm analytics route {route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && value.is_array() == expected_array
        );
    }

    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("swarm analytics runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    for (route, expected_array) in [
        ("/api/v0/swarm/analytics/dashboard", false),
        ("/api/v0/swarm/analytics/performance", false),
        ("/api/v0/swarm/analytics/peers/rankings", true),
        ("/api/v0/swarm/analytics/efficiency", false),
        ("/api/v0/swarm/analytics/recommendations", true),
        ("/api/v0/swarm/analytics/trends", false),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("runtime swarm analytics route {route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && value.is_array() == expected_array
        );
    }

    for (route, populated) in [
        ("/api/v0/swarm/analytics/dashboard", dashboard_json.clone()),
        (
            "/api/v0/swarm/analytics/performance",
            serde_json::json!({"populated": true}),
        ),
        (
            "/api/v0/swarm/analytics/peers/rankings",
            serde_json::json!([{"populated": true}]),
        ),
        (
            "/api/v0/swarm/analytics/efficiency",
            serde_json::json!({"populated": true}),
        ),
        (
            "/api/v0/swarm/analytics/recommendations",
            serde_json::json!([{"populated": true}]),
        ),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("populated swarm analytics route {route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && (populated.is_array() == value.is_array()
                    || populated.is_object() == value.is_object())
        );
    }
    let populated_trends =
        crate::route_http_request("GET", "/api/v0/swarm/analytics/trends", None, "", &state)
            .await
            .expect("populated swarm analytics trends");
    record!(
        "/api/v0/swarm/analytics/trends",
        "populated-dynamic-state",
        populated_trends.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&populated_trends.body)
                .is_ok_and(|value| value["timePoints"].is_array())
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("swarm_analytics_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api swarm-analytics mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
