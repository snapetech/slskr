//! Controller full protocol differential ownership.

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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
pub(super) async fn protocol_behaviors_differential_overlay_gateway_mesh_search() {
    use slskr_client::overlay::{
        MeshHello, MeshSearchRequestMessage, FEATURE_MESH_SEARCH, FEATURE_MESH_SERVICE,
    };

    let root = std::env::temp_dir().join(format!(
        "slskr-overlay-search-differential-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create overlay search state directory");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "member=127.0.0.1:1"),
    );
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("load overlay gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    *state.runtime_credentials.write().await =
        Some(crate::LoginCredentials::default_client("slskR", "secret"));

    add_test_share(
        &state,
        "Virtual/Zed.flac",
        Path::new("/nonexistent/virtual-zed.flac"),
        9,
    )
    .await;
    add_test_share(
        &state,
        "Virtual/Alpha.flac",
        Path::new("/nonexistent/virtual-alpha.flac"),
        7,
    )
    .await;
    add_test_share(
        &state,
        "Virtual/Mid.mp3",
        Path::new("/nonexistent/virtual-mid.mp3"),
        8,
    )
    .await;

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "member",
        vec![FEATURE_MESH_SEARCH.to_owned()],
        Vec::new(),
        Duration::from_secs(300),
        &remote_key,
        SystemTime::now(),
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
    crate::remember_peer_endpoint(
        &state,
        crate::PeerAddress {
            username: "member".to_owned(),
            ip: std::net::Ipv4Addr::LOCALHOST,
            port: 1,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
    let mut hello = MeshHello::new(
        "member",
        vec![
            FEATURE_MESH_SERVICE.to_owned(),
            FEATURE_MESH_SEARCH.to_owned(),
        ],
        None,
        None,
        "mesh-search-differential-nonce",
    )
    .expect("mesh search hello");
    hello
        .authenticate(&remote_key, &certificate_pin)
        .expect("authenticate mesh search hello");
    let mut client = slskr_client::overlay::connect_tls_overlay(endpoint, certificate_pin, hello)
        .await
        .expect("connect mesh search gateway");

    let request_id = uuid::Uuid::new_v4().to_string();
    let request = MeshSearchRequestMessage::new(request_id.clone(), "virtual", 2, None)
        .expect("mesh search request");
    let response = client.search(&request).await.expect("mesh search response");
    assert_eq!(response.request_id, request_id);
    assert!(response.truncated);
    assert_eq!(response.files.len(), 2);
    assert_eq!(response.files[0].filename, "Virtual/Alpha.flac");
    assert_eq!(response.files[0].size, 7);
    assert_eq!(response.files[0].codec.as_deref(), Some("FLAC"));
    assert_eq!(
        response.files[0].media_kinds,
        Some(vec!["Music".to_owned()])
    );
    assert_eq!(response.files[1].filename, "Virtual/Mid.mp3");
    assert_eq!(response.files[1].size, 8);
    assert_eq!(response.files[1].codec.as_deref(), Some("MP3"));

    let rows = [
        serde_json::json!({
            "target": "slskdn",
            "subject": "rendezvous-overlay:MeshSearchReq:mesh_search_req",
            "case": "live-bidirectional-exchange",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "rendezvous-overlay:MeshSearchResp:mesh_search_resp",
            "case": "live-bidirectional-exchange",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create overlay protocol evidence directory");
    fs::write(
        evidence_dir.join("overlay_server_mesh_search.json"),
        serde_json::to_string_pretty(&rows).expect("serialize overlay protocol evidence"),
    )
    .expect("write overlay protocol evidence");

    drop(client);
    gateway_server.abort();
    let _ = gateway_server.await;
    let _ = fs::remove_dir_all(root);
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
pub(super) async fn protocol_behaviors_differential_mesh_sync_private_runtime() {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use slskr_client::{
        mesh_sync::{
            DhtStoreMessage, MeshAckMessage, MeshHashEntry, MeshHelloMessage, MeshMessageType,
            MeshPushDeltaMessage, MeshReqChunkMessage, MeshReqDeltaMessage, MeshReqKeyMessage,
            MeshRespChunkMessage, MeshRespKeyMessage, MeshSyncBase, MeshSyncMessage,
        },
        server::ServerSession,
        stream::ServerConnection,
    };

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh-sync fixture");
    let address = listener.local_addr().expect("mesh-sync fixture address");
    let client = tokio::net::TcpStream::connect(address);
    let server = listener.accept();
    let (client, server) = tokio::join!(client, server);
    let (server, _) = server.expect("accept mesh-sync fixture");
    let mut session = ServerSession::new(ServerConnection::new(server));
    let mut fixture = ServerConnection::new(client.expect("mesh-sync client fixture"));

    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-sync-runtime-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create mesh-sync fixture directory");
    let local_path = root.join("fixture.flac");
    let local_bytes = b"mesh-sync-proof-fixture";
    fs::write(&local_path, local_bytes).expect("write mesh-sync fixture");
    let filename = "Virtual/fixture.flac";
    add_test_share(&state, filename, &local_path, local_bytes.len() as u64).await;
    let local_key = crate::content_discovery::generate_flac_key(filename, local_bytes.len() as u64);
    state
        .content_discovery
        .write()
        .await
        .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
            flac_key: local_key.clone(),
            byte_hash: "a".repeat(64),
            size: local_bytes.len() as u64,
            ..crate::content_discovery::HashDbEntry::default()
        }])
        .expect("seed mesh-sync hash database");

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[53; 32]);
    let now = crate::unix_timestamp_millis() as i64;
    let hello = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::Hello(MeshHelloMessage {
            message_type: MeshMessageType::Hello,
            base: MeshSyncBase::default(),
            client_id: "mesh-peer".to_owned(),
            client_version: "fixture".to_owned(),
            latest_sequence_id: 0,
            hash_count: 0,
        }),
    )
    .await;
    let hello = MeshSyncMessage::decode_private_message(&hello).expect("decode hello response");
    assert!(matches!(hello, MeshSyncMessage::Hello(_)));
    hello.verify_signature().expect("verify hello response");

    let delta = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::ReqDelta(MeshReqDeltaMessage {
            message_type: MeshMessageType::ReqDelta,
            base: MeshSyncBase::default(),
            since_sequence_id: 0,
            max_entries: 10,
        }),
    )
    .await;
    let delta = MeshSyncMessage::decode_private_message(&delta).expect("decode delta response");
    delta.verify_signature().expect("verify delta response");
    assert!(matches!(delta, MeshSyncMessage::PushDelta(_)));
    if let MeshSyncMessage::PushDelta(message) = &delta {
        assert_eq!(message.entries.len(), 1);
        for entry in &message.entries {
            slskr_client::mesh_sync::verify_mesh_hash_entry_signature(entry)
                .expect("verify delta entry signature");
        }
    }

    let key_response = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::ReqKey(MeshReqKeyMessage {
            message_type: MeshMessageType::ReqKey,
            base: MeshSyncBase::default(),
            flac_key: local_key.clone(),
        }),
    )
    .await;
    let key_response =
        MeshSyncMessage::decode_private_message(&key_response).expect("decode key response");
    key_response
        .verify_signature()
        .expect("verify key response");
    assert!(matches!(
        key_response,
        MeshSyncMessage::RespKey(MeshRespKeyMessage { found: true, .. })
    ));

    let chunk_response = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
            message_type: MeshMessageType::ReqChunk,
            base: MeshSyncBase::default(),
            flac_key: local_key,
            offset: 0,
            length: 8,
        }),
    )
    .await;
    let chunk_response =
        MeshSyncMessage::decode_private_message(&chunk_response).expect("decode chunk response");
    chunk_response
        .verify_signature()
        .expect("verify chunk response");
    match chunk_response {
        MeshSyncMessage::RespChunk(MeshRespChunkMessage {
            success: true,
            data_base64,
            ..
        }) => assert_eq!(
            BASE64.decode(data_base64).expect("decode chunk"),
            &local_bytes[..8]
        ),
        other => panic!("unexpected chunk response: {other:?}"),
    }

    let pushed_key = "1122334455667788".to_owned();
    let pushed = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
            message_type: MeshMessageType::PushDelta,
            base: MeshSyncBase::default(),
            entries: vec![MeshHashEntry {
                sequence_id: 7,
                flac_key: pushed_key.clone(),
                byte_hash: "b".repeat(64),
                size: 12,
                metadata_flags: None,
                signer_public_key: None,
                signature: None,
            }],
            latest_sequence_id: 7,
            has_more: false,
        }),
    )
    .await;
    let pushed =
        MeshSyncMessage::decode_private_message(&pushed).expect("decode push acknowledgement");
    pushed
        .verify_signature()
        .expect("verify push acknowledgement");
    assert!(matches!(
        pushed,
        MeshSyncMessage::Ack(MeshAckMessage {
            merged_count: 1,
            ..
        })
    ));
    assert!(state
        .content_discovery
        .read()
        .await
        .lookup_hash(&pushed_key)
        .is_some());

    for message in [
        MeshSyncMessage::RespKey(MeshRespKeyMessage {
            message_type: MeshMessageType::RespKey,
            base: MeshSyncBase::default(),
            flac_key: pushed_key.clone(),
            found: false,
            entry: None,
        }),
        MeshSyncMessage::Ack(MeshAckMessage {
            message_type: MeshMessageType::Ack,
            base: MeshSyncBase::default(),
            merged_count: 0,
            latest_sequence_id: 0,
        }),
        MeshSyncMessage::RespChunk(MeshRespChunkMessage {
            message_type: MeshMessageType::RespChunk,
            base: MeshSyncBase::default(),
            flac_key: pushed_key.clone(),
            offset: 0,
            data_base64: String::new(),
            success: false,
        }),
        MeshSyncMessage::DhtStore(DhtStoreMessage {
            message_type: MeshMessageType::DhtStore,
            base: MeshSyncBase::default(),
            key: BASE64.encode(b"key"),
            value: BASE64.encode(b"value"),
            requester_id: "mesh-peer".to_owned(),
            ttl_seconds: 60,
        }),
    ] {
        send_mesh_sync_fixture_without_response(
            &state,
            &mut session,
            &mut fixture,
            &remote_key,
            now,
            message,
        )
        .await;
    }

    let subjects = [
        ("Hello", 1),
        ("ReqDelta", 2),
        ("PushDelta", 3),
        ("ReqKey", 4),
        ("RespKey", 5),
        ("Ack", 6),
        ("ReqChunk", 7),
        ("RespChunk", 8),
    ];
    let mut rows = Vec::new();
    for (name, value) in subjects {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("mesh-sync:{name}:{value}"),
            "case": "live-bidirectional-exchange",
            "pass": true,
        }));
    }
    for (name, value) in [
        ("Hello", 1),
        ("ReqDelta", 2),
        ("PushDelta", 3),
        ("ReqKey", 4),
        ("ReqChunk", 7),
        ("RespKey", 5),
        ("Ack", 6),
        ("RespChunk", 8),
        ("DhtStore", 9),
    ] {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("mesh-sync:{name}:{value}"),
            "case": "decode-dispatch-and-side-effects",
            "pass": true,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create mesh-sync evidence directory");
    fs::write(
        evidence_dir.join("mesh_sync_private_runtime.json"),
        serde_json::to_string_pretty(&rows).expect("serialize mesh-sync evidence"),
    )
    .expect("write mesh-sync evidence");
    let _ = fs::remove_dir_all(root);
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
pub(super) async fn protocol_behaviors_differential_virtual_soulfind_bridge_round_trips() {
    // These fields mirror BinaryReader/BinaryWriter in the frozen
    // SoulseekProtocolParser.cs.  Build the expected payloads separately
    // from slskR's bridge helpers so this test catches layout drift, not
    // merely helper-to-helper agreement.
    fn oracle_string(payload: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        payload.extend_from_slice(&(i32::try_from(bytes.len()).unwrap()).to_le_bytes());
        payload.extend_from_slice(bytes);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge differential listener");
    let address = listener.local_addr().expect("bridge differential address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept bridge differential");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read login request")
                .expect("login request frame");
        let mut expected = Vec::new();
        oracle_string(&mut expected, "legacy-client");
        oracle_string(&mut expected, "secret");
        assert_eq!(message_type, crate::BRIDGE_LOGIN);
        assert_eq!(payload, expected);
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            crate::BRIDGE_LOGIN_RESPONSE,
            &crate::bridge_login_response(true, "Login successful"),
        )
        .await
        .expect("write login response");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read search request")
                .expect("search request frame");
        let mut expected = Vec::new();
        oracle_string(&mut expected, "ambient");
        expected.extend_from_slice(&41_i32.to_le_bytes());
        assert_eq!(message_type, crate::BRIDGE_SEARCH_REQUEST);
        assert_eq!(payload, expected);
        let mut response = Vec::new();
        response.extend_from_slice(&41_i32.to_le_bytes());
        response.extend_from_slice(&1_i32.to_le_bytes());
        oracle_string(&mut response, "slskR");
        oracle_string(&mut response, "Ambient/Track.flac");
        response.extend_from_slice(&4_096_i64.to_le_bytes());
        response.extend_from_slice(&0_i32.to_le_bytes());
        oracle_string(&mut response, "flac");
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            crate::BRIDGE_SEARCH_RESPONSE,
            &response,
        )
        .await
        .expect("write search response");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read download request")
                .expect("download request frame");
        let mut expected = Vec::new();
        oracle_string(&mut expected, "peer");
        oracle_string(&mut expected, "Ambient/Track.flac");
        expected.extend_from_slice(&42_i32.to_le_bytes());
        assert_eq!(message_type, crate::BRIDGE_DOWNLOAD_REQUEST);
        assert_eq!(payload, expected);
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            crate::BRIDGE_DOWNLOAD_RESPONSE,
            &crate::bridge_download_wire_response(true, "transfer-id", 42),
        )
        .await
        .expect("write download response");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read room list request")
                .expect("room list request frame");
        assert_eq!(message_type, crate::BRIDGE_ROOM_LIST_REQUEST);
        assert!(payload.is_empty());
        let mut response = Vec::new();
        response.extend_from_slice(&1_i32.to_le_bytes());
        oracle_string(&mut response, "ambient");
        response.extend_from_slice(&7_i32.to_le_bytes());
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            crate::BRIDGE_ROOM_LIST_RESPONSE,
            &response,
        )
        .await
        .expect("write room list response");
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect bridge differential client");
    let mut rows = Vec::new();
    macro_rules! record {
        ($name:literal, $value:literal) => {
            rows.push(serde_json::json!({
                "target": "slskdn",
                "subject": concat!("virtual-soulfind-bridge:", $name, ":", $value),
                "case": "exact-frame-and-encoding",
                "pass": true,
            }));
            rows.push(serde_json::json!({
                "target": "slskdn",
                "subject": concat!("virtual-soulfind-bridge:", $name, ":", $value),
                "case": "live-bidirectional-exchange",
                "pass": true,
            }));
        };
    }

    let mut login = Vec::new();
    oracle_string(&mut login, "legacy-client");
    oracle_string(&mut login, "secret");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, crate::BRIDGE_LOGIN, &login)
        .await
        .expect("send login request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read login response")
        .expect("login response frame");
    assert_eq!(message_type, crate::BRIDGE_LOGIN_RESPONSE);
    let mut expected = vec![1_u8];
    oracle_string(&mut expected, "Login successful");
    assert_eq!(payload, expected);
    record!("Login", "1");
    record!("LoginResponse", "2");

    let mut search = Vec::new();
    oracle_string(&mut search, "ambient");
    search.extend_from_slice(&41_i32.to_le_bytes());
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_SEARCH_REQUEST,
        &search,
    )
    .await
    .expect("send search request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read search response")
        .expect("search response frame");
    assert_eq!(message_type, crate::BRIDGE_SEARCH_RESPONSE);
    let mut expected = Vec::new();
    expected.extend_from_slice(&41_i32.to_le_bytes());
    expected.extend_from_slice(&1_i32.to_le_bytes());
    oracle_string(&mut expected, "slskR");
    oracle_string(&mut expected, "Ambient/Track.flac");
    expected.extend_from_slice(&4_096_i64.to_le_bytes());
    expected.extend_from_slice(&0_i32.to_le_bytes());
    oracle_string(&mut expected, "flac");
    assert_eq!(payload, expected);
    record!("SearchRequest", "3");
    record!("SearchResponse", "4");

    let mut download = Vec::new();
    oracle_string(&mut download, "peer");
    oracle_string(&mut download, "Ambient/Track.flac");
    download.extend_from_slice(&42_i32.to_le_bytes());
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_DOWNLOAD_REQUEST,
        &download,
    )
    .await
    .expect("send download request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read download response")
        .expect("download response frame");
    assert_eq!(message_type, crate::BRIDGE_DOWNLOAD_RESPONSE);
    let mut expected = vec![1_u8];
    oracle_string(&mut expected, "transfer-id");
    expected.extend_from_slice(&42_i32.to_le_bytes());
    assert_eq!(payload, expected);
    record!("DownloadRequest", "5");
    record!("DownloadResponse", "6");

    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_ROOM_LIST_REQUEST,
        &[],
    )
    .await
    .expect("send room list request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read room list response")
        .expect("room list response frame");
    assert_eq!(message_type, crate::BRIDGE_ROOM_LIST_RESPONSE);
    let mut expected = Vec::new();
    expected.extend_from_slice(&1_i32.to_le_bytes());
    oracle_string(&mut expected, "ambient");
    expected.extend_from_slice(&7_i32.to_le_bytes());
    assert_eq!(payload, expected);
    record!("RoomListRequest", "7");
    record!("RoomListResponse", "8");

    server.await.expect("bridge differential server task");

    for (name, message_type) in [
        ("Login", crate::BRIDGE_LOGIN),
        ("LoginResponse", crate::BRIDGE_LOGIN_RESPONSE),
        ("SearchRequest", crate::BRIDGE_SEARCH_REQUEST),
        ("SearchResponse", crate::BRIDGE_SEARCH_RESPONSE),
        ("DownloadRequest", crate::BRIDGE_DOWNLOAD_REQUEST),
        ("DownloadResponse", crate::BRIDGE_DOWNLOAD_RESPONSE),
        ("RoomListRequest", crate::BRIDGE_ROOM_LIST_REQUEST),
        ("RoomListResponse", crate::BRIDGE_ROOM_LIST_RESPONSE),
    ] {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "timeout-cancel-reconnect-and-failure",
            "pass": virtual_soulfind_bridge_timeout_and_reconnect(
                message_type,
                vec![0xB2, message_type as u8],
            )
            .await,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_round_trips.json"),
        serde_json::to_string_pretty(&rows).expect("serialize bridge protocol evidence"),
    )
    .expect("write bridge protocol evidence");
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
pub(super) async fn protocol_behaviors_differential_virtual_soulfind_bridge_raw_frames() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    // The frozen bridge parser accepts message types 9..15 at the generic
    // frame boundary, while its handler intentionally dispatches only
    // types 1..8.  Prove the wire framing for these opaque messages, but
    // do not claim payload decoding or handler side effects that neither
    // runtime implements.
    const CASES: [(&str, i32, &[u8]); 7] = [
        ("RoomJoinRequest", 9, b"room-join"),
        ("RoomJoinResponse", 10, b"room-joined"),
        ("RoomLeaveRequest", 11, b"room-leave"),
        ("RoomMessage", 12, b"room-message"),
        ("UserStatus", 13, b"user-status"),
        ("PeerInfo", 14, b"peer-info"),
        ("FileTransfer", 15, b"file-transfer"),
    ];

    fn oracle_frame(message_type: i32, payload: &[u8]) -> Vec<u8> {
        let length = u32::try_from(4_usize + payload.len()).expect("fixture frame length");
        let mut frame = Vec::with_capacity(4 + length as usize);
        frame.extend_from_slice(&length.to_le_bytes());
        frame.extend_from_slice(&message_type.to_le_bytes());
        frame.extend_from_slice(payload);
        frame
    }

    async fn truncated_frame_is_rejected(message_type: i32) -> bool {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind truncated bridge frame listener");
        let address = listener
            .local_addr()
            .expect("truncated bridge frame listener address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("accept truncated bridge frame client");
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .is_err()
        });
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect truncated bridge frame client");
        client
            .write_all(&7_u32.to_le_bytes())
            .await
            .expect("write truncated bridge frame length");
        client
            .write_all(&message_type.to_le_bytes())
            .await
            .expect("write truncated bridge frame type");
        client
            .write_all(&[0xA0, 0xB0])
            .await
            .expect("write truncated bridge frame payload");
        drop(client);
        server.await.expect("truncated bridge frame server task")
    }

    async fn oversized_frame_is_rejected(message_type: i32) -> bool {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind oversized bridge frame listener");
        let address = listener
            .local_addr()
            .expect("oversized bridge frame listener address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("accept oversized bridge frame client");
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .is_err()
        });
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect oversized bridge frame client");
        let length = u32::try_from(crate::BRIDGE_MAX_FRAME_BYTES + 1)
            .expect("oversized bridge frame length");
        client
            .write_all(&length.to_le_bytes())
            .await
            .expect("write oversized bridge frame length");
        client
            .write_all(&message_type.to_le_bytes())
            .await
            .expect("write oversized bridge frame type");
        server.await.expect("oversized bridge frame server task")
    }

    async fn unknown_type_is_preserved() -> bool {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind unknown bridge frame listener");
        let address = listener
            .local_addr()
            .expect("unknown bridge frame listener address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("accept unknown bridge frame client");
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream).await
                == Ok(Some((999, vec![0xCC])))
        });
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect unknown bridge frame client");
        client
            .write_all(&5_u32.to_le_bytes())
            .await
            .expect("write unknown bridge frame length");
        client
            .write_all(&999_i32.to_le_bytes())
            .await
            .expect("write unknown bridge frame type");
        client
            .write_all(&[0xCC])
            .await
            .expect("write unknown bridge frame payload");
        server.await.expect("unknown bridge frame server task")
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind raw bridge frame listener");
    let address = listener
        .local_addr()
        .expect("raw bridge frame listener address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener
            .accept()
            .await
            .expect("accept raw bridge frame client");
        for (_, message_type, payload) in CASES {
            let expected = oracle_frame(message_type, payload);
            let mut actual = vec![0_u8; expected.len()];
            stream
                .read_exact(&mut actual)
                .await
                .expect("read raw bridge request frame");
            assert_eq!(actual, expected, "raw bridge request frame");
            stream
                .write_all(&expected)
                .await
                .expect("write raw bridge response frame");
        }
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect raw bridge frame client");
    let mut rows = Vec::new();
    for (name, message_type, payload) in CASES {
        crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, message_type, payload)
            .await
            .expect("write raw bridge request frame");
        let (actual_type, actual_payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
                .await
                .expect("read raw bridge response frame")
                .expect("raw bridge response frame");
        assert_eq!(actual_type, message_type, "raw bridge response type");
        assert_eq!(actual_payload, payload, "raw bridge response payload");
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "exact-frame-and-encoding",
            "pass": true,
        }));
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "live-bidirectional-exchange",
            "pass": true,
        }));
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "timeout-cancel-reconnect-and-failure",
            "pass": virtual_soulfind_bridge_timeout_and_reconnect(
                message_type,
                payload.to_vec(),
            )
            .await,
        }));
    }

    server.await.expect("raw bridge frame server task");

    const ALL_TYPES: [(&str, i32); 15] = [
        ("Login", 1),
        ("LoginResponse", 2),
        ("SearchRequest", 3),
        ("SearchResponse", 4),
        ("DownloadRequest", 5),
        ("DownloadResponse", 6),
        ("RoomListRequest", 7),
        ("RoomListResponse", 8),
        ("RoomJoinRequest", 9),
        ("RoomJoinResponse", 10),
        ("RoomLeaveRequest", 11),
        ("RoomMessage", 12),
        ("UserStatus", 13),
        ("PeerInfo", 14),
        ("FileTransfer", 15),
    ];
    let unknown_type_pass = unknown_type_is_preserved().await;
    for (name, message_type) in ALL_TYPES {
        let malformed_pass = truncated_frame_is_rejected(message_type).await
            && oversized_frame_is_rejected(message_type).await
            && unknown_type_pass;
        assert!(
            malformed_pass,
            "bridge frame boundary handling failed for {name}:{message_type}"
        );
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "malformed-truncated-oversize-and-unknown",
            "pass": malformed_pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_raw_frames.json"),
        serde_json::to_string_pretty(&rows).expect("serialize raw bridge frame evidence"),
    )
    .expect("write raw bridge frame evidence");
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
pub(super) async fn protocol_behaviors_differential_virtual_soulfind_bridge_dispatch() {
    let (mut state, mut receiver) = test_state();
    let expected_username = state
        .config
        .username
        .clone()
        .unwrap_or_else(|| "slskR".to_owned());
    Arc::get_mut(&mut state)
        .expect("test state has one owner")
        .config
        .media_services
        .virtual_soulfind
        .bridge
        .require_auth = false;
    add_test_share(
        &state,
        "Ambient/Track.flac",
        Path::new("/nonexistent/ambient-track.flac"),
        4_096,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge dispatch listener");
    let address = listener.local_addr().expect("bridge dispatch address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept bridge dispatch client");
        crate::bridge_handle_client(
            "protocol-differential-client".to_owned(),
            stream,
            server_state,
        )
        .await;
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect bridge dispatch client");
    let mut rows = Vec::new();
    let mut record = |name: &str, value: i32, pass: bool| {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{value}"),
            "case": "decode-dispatch-and-side-effects",
            "pass": pass,
        }));
    };

    let mut login = Vec::new();
    crate::bridge_write_string(&mut login, "legacy-client");
    crate::bridge_write_string(&mut login, "ignored-with-auth-disabled");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, crate::BRIDGE_LOGIN, &login)
        .await
        .expect("send bridge dispatch login");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch login response")
        .expect("bridge dispatch login response frame");
    let mut cursor = 1;
    let login_pass = message_type == crate::BRIDGE_LOGIN_RESPONSE
        && payload.first() == Some(&1)
        && crate::bridge_read_string(&payload, &mut cursor).as_deref() == Some("Login successful");

    let mut search = Vec::new();
    crate::bridge_write_string(&mut search, "ambient");
    crate::bridge_write_i32(&mut search, 41);
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_SEARCH_REQUEST,
        &search,
    )
    .await
    .expect("send bridge dispatch search");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch search response")
        .expect("bridge dispatch search response frame");
    let mut cursor = 0;
    let search_pass = message_type == crate::BRIDGE_SEARCH_RESPONSE
        && crate::bridge_read_i32(&payload, &mut cursor) == Some(41)
        && crate::bridge_read_i32(&payload, &mut cursor) == Some(1)
        && crate::bridge_read_string(&payload, &mut cursor).as_deref()
            == Some(expected_username.as_str())
        && crate::bridge_read_string(&payload, &mut cursor).as_deref()
            == Some("Ambient/Track.flac");

    let mut download = Vec::new();
    crate::bridge_write_string(&mut download, "peer");
    crate::bridge_write_string(&mut download, "Ambient/Track.flac");
    crate::bridge_write_i32(&mut download, 42);
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_DOWNLOAD_REQUEST,
        &download,
    )
    .await
    .expect("send bridge dispatch download");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch download response")
        .expect("bridge dispatch download response frame");
    let mut cursor = 1;
    let transfer_id = crate::bridge_read_string(&payload, &mut cursor).unwrap_or_default();
    let download_pass = message_type == crate::BRIDGE_DOWNLOAD_RESPONSE
        && payload.first() == Some(&1)
        && !transfer_id.is_empty()
        && crate::bridge_read_i32(&payload, &mut cursor) == Some(42);
    let queued_download = matches!(
        receiver.recv().await,
        Some(crate::SessionCommand::TransferPeer { username, .. }) if username == "peer"
    );
    let download_pass = download_pass && queued_download;

    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_ROOM_LIST_REQUEST,
        &[],
    )
    .await
    .expect("send bridge dispatch room list");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch room list response")
        .expect("bridge dispatch room list response frame");
    let mut cursor = 0;
    let room_list_pass = message_type == crate::BRIDGE_ROOM_LIST_RESPONSE
        && crate::bridge_read_i32(&payload, &mut cursor) == Some(0);

    // The frozen bridge parser declares these message types, but the
    // frozen proxy has no handlers for them.  Its observable contract is
    // therefore the generic LoginResponse-shaped error, not a fabricated
    // typed payload or side effect.
    for (name, value) in [
        ("RoomJoinRequest", 9),
        ("RoomJoinResponse", 10),
        ("RoomLeaveRequest", 11),
        ("RoomMessage", 12),
        ("UserStatus", 13),
        ("PeerInfo", 14),
        ("FileTransfer", 15),
    ] {
        crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, value, &[0x01, 0x02, 0x03])
            .await
            .expect("send unsupported bridge message type");
        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
                .await
                .expect("read unsupported bridge error")
                .expect("unsupported bridge error frame");
        let mut cursor = 1;
        let unsupported_pass = message_type == crate::BRIDGE_LOGIN_RESPONSE
            && payload.first() == Some(&0)
            && crate::bridge_read_string(&payload, &mut cursor).as_deref()
                == Some("Unknown message type");
        record(name, value, unsupported_pass);
    }

    record("Login", 1, login_pass);
    record("LoginResponse", 2, login_pass);
    record("SearchRequest", 3, search_pass);
    record("SearchResponse", 4, search_pass);
    record("DownloadRequest", 5, download_pass);
    record("DownloadResponse", 6, download_pass);
    record("RoomListRequest", 7, room_list_pass);
    record("RoomListResponse", 8, room_list_pass);

    assert!(rows.iter().all(|row| row["pass"] == true), "{rows:?}");
    drop(client);
    server.await.expect("bridge dispatch server task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create bridge dispatch evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_dispatch.json"),
        serde_json::to_string_pretty(&rows).expect("serialize bridge dispatch evidence"),
    )
    .expect("write bridge dispatch evidence");
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
pub(super) async fn protocol_behaviors_differential_virtual_soulfind_bridge_malformed_frames() {
    let (mut state, _receiver) = test_state();
    Arc::get_mut(&mut state)
        .expect("test state has one owner")
        .config
        .media_services
        .virtual_soulfind
        .bridge
        .require_auth = false;

    let mut rows = Vec::new();
    let mut record = |name: &str, value: i32, pass: bool| {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{value}"),
            "case": "malformed-truncated-oversize-and-unknown",
            "pass": pass,
        }));
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("malformed login listener");
    let address = listener.local_addr().expect("malformed login address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept malformed login client");
        crate::bridge_handle_client("malformed-login".to_owned(), stream, server_state).await;
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect malformed login client");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, crate::BRIDGE_LOGIN, &[])
        .await
        .expect("send malformed login");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read malformed login response")
        .expect("malformed login response frame");
    let mut cursor = 1;
    let login_pass = message_type == crate::BRIDGE_LOGIN_RESPONSE
        && payload.first() == Some(&0)
        && crate::bridge_read_string(&payload, &mut cursor).is_some();
    drop(client);
    server.await.expect("malformed login server task");
    record("Login", 1, login_pass);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("malformed search listener");
    let address = listener.local_addr().expect("malformed search address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept malformed search client");
        crate::bridge_handle_client("malformed-search".to_owned(), stream, server_state).await;
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect malformed search client");
    let mut login = Vec::new();
    crate::bridge_write_string(&mut login, "legacy-client");
    crate::bridge_write_string(&mut login, "ignored");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, crate::BRIDGE_LOGIN, &login)
        .await
        .expect("send valid login before malformed search");
    let _ = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read valid login response");
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_SEARCH_REQUEST,
        &[],
    )
    .await
    .expect("send malformed search");
    let search_pass = matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        )
        .await,
        Ok(Ok(None))
    );
    drop(client);
    server.await.expect("malformed search server task");
    record("SearchRequest", 3, search_pass);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("malformed download listener");
    let address = listener.local_addr().expect("malformed download address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept malformed download client");
        crate::bridge_handle_client("malformed-download".to_owned(), stream, server_state).await;
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect malformed download client");
    let mut login = Vec::new();
    crate::bridge_write_string(&mut login, "legacy-client");
    crate::bridge_write_string(&mut login, "ignored");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, crate::BRIDGE_LOGIN, &login)
        .await
        .expect("send valid login before malformed download");
    let _ = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read valid login response");
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        crate::BRIDGE_DOWNLOAD_REQUEST,
        &[],
    )
    .await
    .expect("send malformed download");
    let download_pass = matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        )
        .await,
        Ok(Ok(None))
    );
    drop(client);
    server.await.expect("malformed download server task");
    record("DownloadRequest", 5, download_pass);

    assert!(rows.iter().all(|row| row["pass"] == true), "{rows:?}");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create bridge malformed evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_malformed.json"),
        serde_json::to_string_pretty(&rows).expect("serialize bridge malformed evidence"),
    )
    .expect("write bridge malformed evidence");
}
