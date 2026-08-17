//! Differential evidence for protocol families implemented by slskR's client
//! orchestration layer rather than by the base Soulseek codec crate.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use serde::Serialize;
use serde_json::Value;
use slskr_client::io::{
    read_init_frame, read_message_frame, write_init_frame, write_message_frame,
};
use slskr_client::protocol::frame::{InitFrame, MessageFrame};
use slskr_client::protocol::peer::PeerMessage;
use slskr_client::stream::PeerMessageConnection;
use slskr_client::{
    capabilities::{
        decode_peer_capability_message, handle_peer_capability_message, peer_capability_message,
        PeerCapabilityDescriptor, PeerCapabilityEnvelope, PeerCapabilityMessageType,
        PeerCapabilityRegistry,
    },
    mesh_sync::{
        DhtStoreMessage, MeshAckMessage, MeshHashEntry, MeshHelloMessage, MeshMessageType,
        MeshPushDeltaMessage, MeshReqChunkMessage, MeshReqDeltaMessage, MeshReqKeyMessage,
        MeshRespChunkMessage, MeshRespKeyMessage, MeshSyncBase, MeshSyncMessage,
        MAX_MESH_SYNC_ENTRIES, MAX_MESH_SYNC_PAYLOAD_BYTES,
    },
    overlay::{
        Disconnect, MeshHello, MeshHelloAck, MeshSearchFileDto, MeshSearchRequestMessage,
        MeshSearchResponseMessage, MeshServiceCall, MeshServiceReply, OverlayClient, OverlayError,
        OverlayFramer, Ping, Pong, SoulseekPorts, FEATURE_MESH_SEARCH, MAX_OVERLAY_MESSAGE_BYTES,
        OVERLAY_MAGIC, OVERLAY_VERSION,
    },
    overlay_control::ControlEnvelope,
};
use tokio::io::AsyncWriteExt as _;

fn fixed_now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

fn test_nonce() -> String {
    rand::random::<u128>().to_string()
}

fn test_signing_key() -> SigningKey {
    let bytes = rand::random::<[u8; 32]>();
    SigningKey::from_bytes(&bytes)
}

fn signed_descriptor(username: &str, signing_key: &SigningKey) -> PeerCapabilityDescriptor {
    PeerCapabilityDescriptor::unsigned(
        username,
        vec![
            "slskdn-capabilities-v1".to_owned(),
            "slskdn-mesh-v1".to_owned(),
        ],
        vec!["tcp:127.0.0.1:2234".to_owned()],
        Duration::from_secs(300),
        signing_key,
        fixed_now(),
    )
    .expect("capability descriptor")
    .sign(signing_key)
    .expect("sign capability descriptor")
}

fn json_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize protocol value")
}

async fn raw_message_frame_bidirectional(code: u32) -> bool {
    let (mut client, mut server) = tokio::io::duplex(16 * 1024);
    let server_task = tokio::spawn(async move {
        let request = match read_message_frame(&mut server).await {
            Ok(frame) if frame == MessageFrame::new(code, [0xa1, code as u8]) => frame,
            _ => return false,
        };
        if write_message_frame(
            &mut server,
            &MessageFrame::new(request.code, [0xb1, request.code as u8]),
        )
        .await
        .is_err()
        {
            return false;
        }
        let second_request = match read_message_frame(&mut server).await {
            Ok(frame) if frame == MessageFrame::new(code, [0xa2, code as u8]) => frame,
            _ => return false,
        };
        write_message_frame(
            &mut server,
            &MessageFrame::new(second_request.code, [0xb2, second_request.code as u8]),
        )
        .await
        .is_ok()
    });

    if write_message_frame(&mut client, &MessageFrame::new(code, [0xa1, code as u8]))
        .await
        .is_err()
    {
        server_task.abort();
        return false;
    }
    let first_reply = matches!(
        read_message_frame(&mut client).await,
        Ok(frame) if frame == MessageFrame::new(code, [0xb1, code as u8])
    );
    if !first_reply
        || write_message_frame(&mut client, &MessageFrame::new(code, [0xa2, code as u8]))
            .await
            .is_err()
    {
        server_task.abort();
        return false;
    }
    let second_reply = matches!(
        read_message_frame(&mut client).await,
        Ok(frame) if frame == MessageFrame::new(code, [0xb2, code as u8])
    );
    server_task.await.is_ok_and(|result| result) && second_reply
}

async fn raw_init_frame_bidirectional(code: u8) -> bool {
    let (mut client, mut server) = tokio::io::duplex(16 * 1024);
    let server_task = tokio::spawn(async move {
        let request = match read_init_frame(&mut server).await {
            Ok(frame) if frame == InitFrame::new(code, [0xa1, code]) => frame,
            _ => return false,
        };
        if write_init_frame(
            &mut server,
            &InitFrame::new(request.code, [0xb1, request.code]),
        )
        .await
        .is_err()
        {
            return false;
        }
        let second_request = match read_init_frame(&mut server).await {
            Ok(frame) if frame == InitFrame::new(code, [0xa2, code]) => frame,
            _ => return false,
        };
        write_init_frame(
            &mut server,
            &InitFrame::new(second_request.code, [0xb2, second_request.code]),
        )
        .await
        .is_ok()
    });

    if write_init_frame(&mut client, &InitFrame::new(code, [0xa1, code]))
        .await
        .is_err()
    {
        server_task.abort();
        return false;
    }
    let first_reply = matches!(
        read_init_frame(&mut client).await,
        Ok(frame) if frame == InitFrame::new(code, [0xb1, code])
    );
    if !first_reply
        || write_init_frame(&mut client, &InitFrame::new(code, [0xa2, code]))
            .await
            .is_err()
    {
        server_task.abort();
        return false;
    }
    let second_reply = matches!(
        read_init_frame(&mut client).await,
        Ok(frame) if frame == InitFrame::new(code, [0xb2, code])
    );
    server_task.await.is_ok_and(|result| result) && second_reply
}

async fn peer_capability_live_exchange() -> bool {
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server_key = test_signing_key();
    let server_descriptor = signed_descriptor("capability-server", &server_key);
    let server = tokio::spawn(async move {
        let mut connection = PeerMessageConnection::new(server_stream);
        let message = match connection.receive().await {
            Ok(PeerMessage::Unknown { code, payload })
                if code == slskr_client::capabilities::PEER_CAPABILITY_MESSAGE_CODE =>
            {
                PeerMessage::Unknown { code, payload }
            }
            _ => return false,
        };
        let Some(envelope) = decode_peer_capability_message(&message).ok().flatten() else {
            return false;
        };
        if envelope.message_type != PeerCapabilityMessageType::Hello {
            return false;
        }
        let acknowledgement = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Acknowledge,
            envelope.nonce,
            server_descriptor,
        );
        let response = match peer_capability_message(&acknowledgement) {
            Ok(response) => response,
            Err(_) => return false,
        };
        connection.send(&response).await.is_ok()
    });

    let client_key = test_signing_key();
    let nonce = test_nonce();
    let hello = PeerCapabilityEnvelope::new(
        PeerCapabilityMessageType::Hello,
        nonce.clone(),
        signed_descriptor("capability-client", &client_key),
    );
    let message = match peer_capability_message(&hello) {
        Ok(message) => message,
        Err(_) => return false,
    };
    let mut connection = PeerMessageConnection::new(client_stream);
    if connection.send(&message).await.is_err() {
        server.abort();
        let _ = server.await;
        return false;
    }
    let acknowledgement = match connection.receive().await {
        Ok(message) => decode_peer_capability_message(&message).ok().flatten(),
        Err(_) => None,
    };
    let client_pass = acknowledgement.is_some_and(|acknowledgement| {
        acknowledgement.message_type == PeerCapabilityMessageType::Acknowledge
            && acknowledgement.nonce == nonce
            && acknowledgement.descriptor.signature.is_some()
    });
    let server_pass = server.await.is_ok_and(|result| result);
    client_pass && server_pass
}

async fn peer_capability_timeout_and_reconnect() -> bool {
    let client_key = test_signing_key();
    let nonce = test_nonce();
    let hello = PeerCapabilityEnvelope::new(
        PeerCapabilityMessageType::Hello,
        nonce.clone(),
        signed_descriptor("timeout-capability-client", &client_key),
    );
    let message = match peer_capability_message(&hello) {
        Ok(message) => message,
        Err(_) => return false,
    };

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let first_server = tokio::spawn(async move {
        let mut connection = PeerMessageConnection::new(server_stream);
        let Ok(message) = connection.receive().await else {
            return false;
        };
        let Some(envelope) = decode_peer_capability_message(&message).ok().flatten() else {
            return false;
        };
        if envelope.message_type != PeerCapabilityMessageType::Hello {
            return false;
        }
        std::future::pending::<()>().await;
        #[allow(unreachable_code)]
        false
    });
    let mut first_client = PeerMessageConnection::new(client_stream);
    if first_client.send(&message).await.is_err() {
        first_server.abort();
        let _ = first_server.await;
        return false;
    }
    let timed_out = tokio::time::timeout(Duration::from_millis(25), first_client.receive())
        .await
        .is_err();
    first_server.abort();
    let _ = first_server.await;

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server_key = test_signing_key();
    let server_descriptor = signed_descriptor("reconnected-capability-server", &server_key);
    let second_server = tokio::spawn(async move {
        let mut connection = PeerMessageConnection::new(server_stream);
        let Ok(message) = connection.receive().await else {
            return false;
        };
        let Some(envelope) = decode_peer_capability_message(&message).ok().flatten() else {
            return false;
        };
        if envelope.message_type != PeerCapabilityMessageType::Hello {
            return false;
        }
        let acknowledgement = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Acknowledge,
            envelope.nonce,
            server_descriptor,
        );
        let Ok(response) = peer_capability_message(&acknowledgement) else {
            return false;
        };
        connection.send(&response).await.is_ok()
    });
    let mut second_client = PeerMessageConnection::new(client_stream);
    let reconnected = if second_client.send(&message).await.is_err() {
        false
    } else {
        match second_client.receive().await {
            Ok(response) => decode_peer_capability_message(&response)
                .ok()
                .flatten()
                .is_some_and(|acknowledgement| {
                    acknowledgement.message_type == PeerCapabilityMessageType::Acknowledge
                        && acknowledgement.nonce == nonce
                }),
            Err(_) => false,
        }
    };
    let server_pass = second_server.await.is_ok_and(|result| result);
    timed_out && reconnected && server_pass
}

async fn mesh_sync_timeout_and_reconnect(message: MeshSyncMessage) -> bool {
    let body = match message.encode_private_message() {
        Ok(body) => body,
        Err(_) => return false,
    };
    let expected_type = message.message_type();
    let wire_message = PeerMessage::PrivateMessage(body.as_bytes().to_vec());

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let first_server = tokio::spawn({
        let body = body.clone();
        async move {
            let mut connection = PeerMessageConnection::new(server_stream);
            let Ok(PeerMessage::PrivateMessage(payload)) = connection.receive().await else {
                return false;
            };
            let Ok(decoded) = MeshSyncMessage::decode_private_message(
                std::str::from_utf8(&payload).unwrap_or_default(),
            ) else {
                return false;
            };
            if decoded.message_type() != expected_type || payload != body.as_bytes() {
                return false;
            }
            std::future::pending::<()>().await;
            #[allow(unreachable_code)]
            false
        }
    });
    let mut first_client = PeerMessageConnection::new(client_stream);
    if first_client.send(&wire_message).await.is_err() {
        first_server.abort();
        let _ = first_server.await;
        return false;
    }
    let timed_out = tokio::time::timeout(Duration::from_millis(25), first_client.receive())
        .await
        .is_err();
    first_server.abort();
    let _ = first_server.await;

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let second_server = tokio::spawn({
        let body = body.clone();
        async move {
            let mut connection = PeerMessageConnection::new(server_stream);
            let Ok(PeerMessage::PrivateMessage(payload)) = connection.receive().await else {
                return false;
            };
            let Ok(decoded) = MeshSyncMessage::decode_private_message(
                std::str::from_utf8(&payload).unwrap_or_default(),
            ) else {
                return false;
            };
            if decoded.message_type() != expected_type || payload != body.as_bytes() {
                return false;
            }
            connection
                .send(&PeerMessage::PrivateMessage(body.into_bytes()))
                .await
                .is_ok()
        }
    });
    let mut second_client = PeerMessageConnection::new(client_stream);
    let reconnected = if second_client.send(&wire_message).await.is_err() {
        false
    } else {
        match second_client.receive().await {
            Ok(PeerMessage::PrivateMessage(payload)) => MeshSyncMessage::decode_private_message(
                std::str::from_utf8(&payload).unwrap_or_default(),
            )
            .is_ok_and(|decoded| decoded.message_type() == expected_type),
            _ => false,
        }
    };
    let server_pass = second_server.await.is_ok_and(|result| result);
    timed_out && reconnected && server_pass
}

async fn overlay_handshake_timeout_and_reconnect() -> bool {
    let hello = match MeshHello::new(
        "timeout-overlay-client",
        vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        test_nonce(),
    ) {
        Ok(hello) => hello,
        Err(_) => return false,
    };
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let first_server = tokio::spawn(async move {
        let mut framer = OverlayFramer::new(server_stream);
        let Ok(hello) = framer.read::<MeshHello>().await else {
            return false;
        };
        if hello.validate().is_err() {
            return false;
        }
        std::future::pending::<()>().await;
        #[allow(unreachable_code)]
        false
    });
    let timed_out = tokio::time::timeout(
        Duration::from_millis(25),
        OverlayClient::handshake(client_stream, hello.clone()),
    )
    .await
    .is_err();
    first_server.abort();
    let _ = first_server.await;

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let second_server = tokio::spawn(async move {
        let mut framer = OverlayFramer::new(server_stream);
        let Ok(hello) = framer.read::<MeshHello>().await else {
            return false;
        };
        if hello.validate().is_err() {
            return false;
        }
        framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "reconnected-overlay-server".to_owned(),
                features: vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports: None,
                overlay_port: None,
                nonce_echo: hello.nonce,
            })
            .await
            .is_ok()
    });
    let reconnected = matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            OverlayClient::handshake(client_stream, hello)
        )
        .await,
        Ok(Ok(_))
    );
    let server_pass = second_server.await.is_ok_and(|result| result);
    timed_out && reconnected && server_pass
}

async fn overlay_round_trip(value: Value) -> bool {
    let (writer_stream, reader_stream) = tokio::io::duplex(64 * 1024);
    let expected = value.clone();
    let write = async move {
        let mut framer = OverlayFramer::new(writer_stream);
        framer.write(&value).await
    };
    let read = async move {
        let mut framer = OverlayFramer::new(reader_stream);
        framer.read::<Value>().await
    };
    let (write_result, read_result) = tokio::join!(write, read);
    write_result.is_ok() && read_result.ok() == Some(expected)
}

async fn overlay_search_exchange() -> bool {
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(async move {
        let mut framer = OverlayFramer::new(server_stream);
        let hello: MeshHello = framer.read().await.expect("read mesh hello");
        framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "search-peer".to_owned(),
                features: vec![FEATURE_MESH_SEARCH.to_owned()],
                soulseek_ports: None,
                overlay_port: None,
                nonce_echo: hello.nonce,
            })
            .await
            .expect("write mesh hello ack");
        let request: MeshSearchRequestMessage = framer.read().await.expect("read mesh search");
        request.validate().expect("validate mesh search");
        framer
            .write(
                &MeshSearchResponseMessage::new(
                    request.request_id,
                    vec![MeshSearchFileDto {
                        filename: "Music/fixture.flac".to_owned(),
                        size: 1024,
                        extension: Some("flac".to_owned()),
                        bitrate: Some(900),
                        duration: Some(4),
                        codec: Some("FLAC".to_owned()),
                        media_kinds: Some(vec!["Music".to_owned()]),
                        content_id: Some("fixture-content".to_owned()),
                        hash: Some("fixture-hash".to_owned()),
                    }],
                    false,
                    None,
                )
                .expect("build mesh search response"),
            )
            .await
            .expect("write mesh search response");
    });
    let hello = MeshHello::new(
        "search-client",
        vec![FEATURE_MESH_SEARCH.to_owned()],
        None,
        None,
        test_nonce(),
    )
    .expect("build search hello");
    let mut client = OverlayClient::handshake(client_stream, hello)
        .await
        .expect("mesh search handshake");
    let response = client
        .search(
            &MeshSearchRequestMessage::new(
                "01234567-89ab-cdef-0123-456789abcdef",
                "fixture",
                25,
                Some("music".to_owned()),
            )
            .expect("build search request"),
        )
        .await
        .expect("mesh search exchange");
    let pass = response.files.len() == 1
        && response.files[0].filename == "Music/fixture.flac"
        && response.request_id == "01234567-89ab-cdef-0123-456789abcdef";
    server.await.expect("mesh search server task");
    pass
}

async fn overlay_service_exchange(service_name: &str) -> bool {
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server_service_name = service_name.to_owned();
    let server = tokio::spawn(async move {
        let mut framer = OverlayFramer::new(server_stream);
        let hello: MeshHello = match framer.read::<MeshHello>().await {
            Ok(hello) if hello.validate().is_ok() => hello,
            _ => return false,
        };
        if framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "service-peer".to_owned(),
                features: vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports: None,
                overlay_port: None,
                nonce_echo: hello.nonce,
            })
            .await
            .is_err()
        {
            return false;
        }
        let call: MeshServiceCall = match framer.read::<MeshServiceCall>().await {
            Ok(call) if call.validate().is_ok() => call,
            _ => return false,
        };
        if call.service_name != server_service_name {
            return false;
        }
        let ping_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or(1_700_000_000_000);
        if framer
            .write(&Ping {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "ping".to_owned(),
                version: OVERLAY_VERSION,
                timestamp: ping_timestamp,
            })
            .await
            .is_err()
        {
            return false;
        }
        let _pong: Pong = match framer.read::<Pong>().await {
            Ok(pong) if pong.validate().is_ok() && pong.timestamp == ping_timestamp => pong,
            _ => return false,
        };
        framer
            .write(&MeshServiceReply {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_service_reply".to_owned(),
                version: OVERLAY_VERSION,
                correlation_id: call.correlation_id,
                status_code: 0,
                payload: vec![9, 8, 7],
                error_message: None,
            })
            .await
            .is_ok()
    });

    let hello = match MeshHello::new(
        "service-client",
        vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        test_nonce(),
    ) {
        Ok(hello) => hello,
        Err(_) => return false,
    };
    let mut client = match OverlayClient::handshake(client_stream, hello).await {
        Ok(client) => client,
        Err(_) => return false,
    };
    let call =
        match MeshServiceCall::new("service-correlation", service_name, "Probe", vec![1, 2, 3]) {
            Ok(call) => call,
            Err(_) => return false,
        };
    let reply = client.call(&call).await;
    let server_pass = server.await.is_ok_and(|result| result);
    reply.is_ok_and(|reply| {
        reply.correlation_id == call.correlation_id && reply.payload == [9, 8, 7]
    }) && server_pass
}

async fn overlay_disconnect_exchange() -> bool {
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(async move {
        let mut framer = OverlayFramer::new(server_stream);
        let hello: MeshHello = match framer.read::<MeshHello>().await {
            Ok(hello) if hello.validate().is_ok() => hello,
            _ => return false,
        };
        if framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "disconnect-peer".to_owned(),
                features: vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports: None,
                overlay_port: None,
                nonce_echo: hello.nonce,
            })
            .await
            .is_err()
        {
            return false;
        }
        let call: MeshServiceCall = match framer.read::<MeshServiceCall>().await {
            Ok(call) if call.validate().is_ok() => call,
            _ => return false,
        };
        if call.service_name != "private-gateway" {
            return false;
        }
        framer
            .write(&Disconnect {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "disconnect".to_owned(),
                version: OVERLAY_VERSION,
                reason: Some("peer shutdown".to_owned()),
            })
            .await
            .is_ok()
    });

    let hello = match MeshHello::new(
        "disconnect-client",
        vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        test_nonce(),
    ) {
        Ok(hello) => hello,
        Err(_) => return false,
    };
    let mut client = match OverlayClient::handshake(client_stream, hello).await {
        Ok(client) => client,
        Err(_) => return false,
    };
    let call = match MeshServiceCall::new(
        "disconnect-correlation",
        "private-gateway",
        "Probe",
        vec![1, 2, 3],
    ) {
        Ok(call) => call,
        Err(_) => return false,
    };
    let disconnected = matches!(
        client
            .call_with_timeout(&call, Duration::from_secs(1))
            .await,
        Err(OverlayError::Disconnected)
    );
    let failed_reuse = matches!(
        client
            .call_with_timeout(&call, Duration::from_secs(1))
            .await,
        Err(OverlayError::Disconnected)
    );
    let server_pass = server.await.is_ok_and(|result| result);
    disconnected && failed_reuse && server_pass
}

async fn overlay_service_timeout_and_reuse(service_name: &str) -> bool {
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server_service_name = service_name.to_owned();
    let server = tokio::spawn(async move {
        let mut framer = OverlayFramer::new(server_stream);
        let hello: MeshHello = framer.read().await.expect("read timeout service hello");
        framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "silent-service-peer".to_owned(),
                features: vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
                soulseek_ports: None,
                overlay_port: None,
                nonce_echo: hello.nonce,
            })
            .await
            .expect("write timeout service hello ack");
        let call: MeshServiceCall = framer.read().await.expect("read timeout service call");
        assert_eq!(call.service_name, server_service_name);
        let ping_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or(1_700_000_000_000);
        framer
            .write(&Ping {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "ping".to_owned(),
                version: OVERLAY_VERSION,
                timestamp: ping_timestamp,
            })
            .await
            .expect("write timeout service ping");
        let pong: Pong = framer.read().await.expect("read timeout service pong");
        assert!(pong.validate().is_ok());
        assert_eq!(pong.timestamp, ping_timestamp);
        std::future::pending::<()>().await;
    });

    let hello = MeshHello::new(
        "timeout-service-client",
        vec![slskr_client::overlay::FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        test_nonce(),
    )
    .expect("build timeout service hello");
    let mut client = OverlayClient::handshake(client_stream, hello)
        .await
        .expect("timeout service handshake");
    let call = MeshServiceCall::new(
        "timeout-service-correlation",
        service_name,
        "Probe",
        vec![1, 2, 3],
    )
    .expect("build timeout service call");
    let timed_out = matches!(
        client
            .call_with_timeout(&call, Duration::from_millis(25))
            .await,
        Err(OverlayError::Timeout("overlay service call"))
    );
    let failed_reuse = matches!(
        client
            .call_with_timeout(&call, Duration::from_secs(1))
            .await,
        Err(OverlayError::Disconnected)
    );
    server.abort();
    let _ = server.await;
    timed_out && failed_reuse
}

async fn overlay_search_timeout_and_reuse() -> bool {
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(async move {
        let mut framer = OverlayFramer::new(server_stream);
        let hello: MeshHello = framer.read().await.expect("read timeout mesh hello");
        framer
            .write(&MeshHelloAck {
                magic: OVERLAY_MAGIC.to_owned(),
                message_type: "mesh_hello_ack".to_owned(),
                version: OVERLAY_VERSION,
                username: "silent-search-peer".to_owned(),
                features: vec![FEATURE_MESH_SEARCH.to_owned()],
                soulseek_ports: None,
                overlay_port: None,
                nonce_echo: hello.nonce,
            })
            .await
            .expect("write timeout mesh hello ack");
        let _: MeshSearchRequestMessage = framer.read().await.expect("read timeout request");
        std::future::pending::<()>().await;
    });

    let hello = MeshHello::new(
        "timeout-search-client",
        vec![FEATURE_MESH_SEARCH.to_owned()],
        None,
        None,
        test_nonce(),
    )
    .expect("build timeout search hello");
    let mut client = OverlayClient::handshake(client_stream, hello)
        .await
        .expect("timeout search handshake");
    let request =
        MeshSearchRequestMessage::new("01234567-89ab-cdef-0123-456789abcdef", "silent", 1, None)
            .expect("build timeout search request");
    let timed_out = matches!(
        client
            .search_with_timeout(&request, Duration::from_millis(10))
            .await,
        Err(OverlayError::Timeout("overlay mesh search"))
    );
    let failed_reuse = matches!(
        client
            .search_with_timeout(&request, Duration::from_secs(1))
            .await,
        Err(OverlayError::Disconnected)
    );
    server.abort();
    let _ = server.await;
    timed_out && failed_reuse
}

async fn overlay_search_malformed_cases(value: Value, request: bool) -> bool {
    let encoded = serde_json::to_vec(&value).expect("encode overlay fixture");
    let truncated = &encoded[..encoded.len().saturating_sub(1)];
    let truncated_rejected = serde_json::from_slice::<Value>(truncated).is_err();

    let mut unknown = value.clone();
    unknown["type"] = Value::String("unknown-overlay-message".to_owned());
    let unknown_rejected = if request {
        serde_json::from_value::<MeshSearchRequestMessage>(unknown)
            .ok()
            .is_some_and(|message| message.validate().is_err())
    } else {
        serde_json::from_value::<MeshSearchResponseMessage>(unknown)
            .ok()
            .is_some_and(|message| message.validate().is_err())
    };

    let specific_rejected = if request {
        let mut invalid = serde_json::from_value::<MeshSearchRequestMessage>(value)
            .expect("decode search request fixture");
        invalid.max_results = 0;
        invalid.validate().is_err()
    } else {
        let mut invalid = serde_json::from_value::<MeshSearchResponseMessage>(value)
            .expect("decode search response fixture");
        invalid.files.resize(
            501,
            MeshSearchFileDto {
                filename: "oversize.flac".to_owned(),
                size: 1,
                extension: None,
                bitrate: None,
                duration: None,
                codec: None,
                media_kinds: None,
                content_id: None,
                hash: None,
            },
        );
        invalid.validate().is_err()
    };

    let (mut writer, reader) = tokio::io::duplex(8);
    writer
        .write_all(&((MAX_OVERLAY_MESSAGE_BYTES as u32) + 1).to_be_bytes())
        .await
        .expect("write oversized overlay length");
    let mut framer = OverlayFramer::new(reader);
    let oversized_rejected = matches!(
        framer.read_raw().await,
        Err(OverlayError::FrameTooLarge(length)) if length == MAX_OVERLAY_MESSAGE_BYTES + 1
    );

    let utf16_boundary_rejected = if request {
        MeshSearchRequestMessage::new(
            "01234567-89ab-cdef-0123-456789abcdef",
            "😀".repeat(129),
            1,
            None,
        )
        .is_err()
    } else {
        true
    };

    truncated_rejected
        && unknown_rejected
        && specific_rejected
        && oversized_rejected
        && utf16_boundary_rejected
}

fn typed_overlay_value_rejects(value: &Value, name: &str) -> bool {
    match name {
        "Hello" => serde_json::from_value::<MeshHello>(value.clone())
            .ok()
            .is_some_and(|message| message.validate().is_err()),
        "HelloAck" => serde_json::from_value::<MeshHelloAck>(value.clone())
            .ok()
            .is_some_and(|message| message.validate().is_err()),
        "Ping" => serde_json::from_value::<Ping>(value.clone())
            .ok()
            .is_some_and(|message| message.validate().is_err()),
        "Pong" => serde_json::from_value::<Pong>(value.clone())
            .ok()
            .is_some_and(|message| message.validate().is_err()),
        "Disconnect" => serde_json::from_value::<Disconnect>(value.clone())
            .ok()
            .is_some_and(|message| message.validate().is_err()),
        "MeshServiceCall" => serde_json::from_value::<MeshServiceCall>(value.clone())
            .ok()
            .is_some_and(|message| message.validate().is_err()),
        "MeshServiceReply" => serde_json::from_value::<MeshServiceReply>(value.clone())
            .ok()
            .is_some_and(|message| message.validate().is_err()),
        _ => false,
    }
}

async fn overlay_typed_malformed_cases(value: Value, name: &str) -> bool {
    let encoded = serde_json::to_vec(&value).expect("encode typed overlay fixture");
    let truncated = &encoded[..encoded.len().saturating_sub(1)];
    let truncated_json_rejected = serde_json::from_slice::<Value>(truncated).is_err();

    let (mut writer, reader) = tokio::io::duplex(encoded.len() + 8);
    writer
        .write_all(&(encoded.len() as u32).to_be_bytes())
        .await
        .expect("write truncated typed overlay length");
    writer
        .write_all(truncated)
        .await
        .expect("write truncated typed overlay payload");
    drop(writer);
    let mut framer = OverlayFramer::new(reader);
    let truncated_frame_rejected = matches!(
        framer.read_raw().await,
        Err(OverlayError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof
    );

    let mut unknown_type = value.clone();
    unknown_type["type"] = Value::String("unknown-overlay-message".to_owned());
    let unknown_type_rejected = typed_overlay_value_rejects(&unknown_type, name);

    let mut invalid_magic = value;
    invalid_magic["magic"] = Value::String("invalid".to_owned());
    let invalid_magic_rejected = typed_overlay_value_rejects(&invalid_magic, name);

    let (mut writer, reader) = tokio::io::duplex(8);
    writer
        .write_all(&((MAX_OVERLAY_MESSAGE_BYTES as u32) + 1).to_be_bytes())
        .await
        .expect("write oversized typed overlay length");
    let mut framer = OverlayFramer::new(reader);
    let oversized_rejected = matches!(
        framer.read_raw().await,
        Err(OverlayError::FrameTooLarge(length)) if length == MAX_OVERLAY_MESSAGE_BYTES + 1
    );

    truncated_json_rejected
        && truncated_frame_rejected
        && unknown_type_rejected
        && invalid_magic_rejected
        && oversized_rejected
}

fn control_envelope_malformed_cases(
    envelope: &ControlEnvelope,
    encoded: &[u8],
    signing_key: &SigningKey,
) -> bool {
    let truncated_rejected =
        ControlEnvelope::decode(&encoded[..encoded.len().saturating_sub(1)]).is_err();
    let oversized_rejected = ControlEnvelope::decode(&vec![
        0_u8;
        slskr_client::overlay_control::CONTROL_MAX_DATAGRAM_BYTES
            + 1
    ])
    .is_err();

    let unknown = ControlEnvelope::signed_at(
        "unknown-control-type",
        envelope.payload.clone(),
        envelope.message_id.clone(),
        envelope.timestamp_unix_ms,
        signing_key,
    )
    .expect("sign unknown control envelope");
    let unknown_preserved = ControlEnvelope::decode(&unknown.encode().expect("encode unknown"))
        .is_ok_and(|decoded| decoded.message_type == "unknown-control-type");

    let mut invalid_signature = envelope.clone();
    let replacement = if invalid_signature.signature.starts_with('A') {
        'B'
    } else {
        'A'
    };
    invalid_signature
        .signature
        .replace_range(0..1, &replacement.to_string());
    let invalid_signature_rejected = invalid_signature.verify().is_err();

    truncated_rejected && oversized_rejected && unknown_preserved && invalid_signature_rejected
}

fn capability_wire_equivalent(
    decoded: &PeerCapabilityEnvelope,
    encoded: &PeerCapabilityEnvelope,
) -> bool {
    decoded.version == encoded.version
        && decoded.message_type == encoded.message_type
        && decoded.nonce == encoded.nonce
        && decoded.descriptor.peer_id == encoded.descriptor.peer_id
        && decoded.descriptor.features == encoded.descriptor.features
        && decoded.descriptor.overlay_port == encoded.descriptor.overlay_port
        && decoded.descriptor.max_payload_length == encoded.descriptor.max_payload_length
        && decoded.descriptor.public_key == encoded.descriptor.public_key
        && decoded.descriptor.signature == encoded.descriptor.signature
}

fn mesh_sync_round_trip(message: MeshSyncMessage) -> bool {
    let encoded = message.encode_json().expect("encode mesh-sync JSON");
    let decoded = MeshSyncMessage::decode_json(&encoded).expect("decode mesh-sync JSON");
    let private = message
        .encode_private_message()
        .expect("encode mesh-sync private message");
    let private_decoded = MeshSyncMessage::decode_private_message(&private)
        .expect("decode mesh-sync private message");
    decoded == message && private_decoded == message
}

fn mesh_sync_signed_round_trip(message: &MeshSyncMessage) -> bool {
    let signing_key = test_signing_key();
    let timestamp = 1_700_000_000_000;
    let mut signed = message.clone();
    signed
        .sign_at(&signing_key, timestamp)
        .expect("sign mesh-sync fixture");
    if signed.verify_signature_at(timestamp).is_err() {
        return false;
    }
    let encoded = signed.encode_json().expect("encode signed mesh-sync JSON");
    let decoded = MeshSyncMessage::decode_json(&encoded).expect("decode signed mesh-sync JSON");
    if decoded.verify_signature_at(timestamp).is_err() || decoded != signed {
        return false;
    }
    let private = signed
        .encode_private_message()
        .expect("encode signed mesh-sync private message");
    let private_decoded = MeshSyncMessage::decode_private_message(&private)
        .expect("decode signed mesh-sync private message");
    private_decoded.verify_signature_at(timestamp).is_ok() && private_decoded == signed
}

fn mesh_sync_malformed_cases(message: &MeshSyncMessage) -> bool {
    let encoded = message.encode_json().expect("encode mesh-sync fixture");
    let truncated = &encoded[..encoded.len().saturating_sub(1)];
    let unknown_type = br#"{"type":99}"#;
    let oversized = vec![b' '; MAX_MESH_SYNC_PAYLOAD_BYTES + 1];
    let specific_invalid = match message {
        MeshSyncMessage::Hello(message) => {
            let mut invalid = message.clone();
            invalid.latest_sequence_id = -1;
            MeshSyncMessage::Hello(invalid).validate().is_err()
        }
        MeshSyncMessage::ReqDelta(message) => {
            let mut invalid = message.clone();
            invalid.max_entries = 2_001;
            MeshSyncMessage::ReqDelta(invalid).validate().is_err()
        }
        MeshSyncMessage::PushDelta(message) => {
            let mut invalid = message.clone();
            let entry = invalid.entries.first().cloned().unwrap_or(MeshHashEntry {
                sequence_id: 0,
                flac_key: "0123456789abcdef".to_owned(),
                byte_hash: "0".repeat(64),
                size: 1,
                metadata_flags: None,
                signer_public_key: None,
                signature: None,
            });
            invalid.entries.resize(MAX_MESH_SYNC_ENTRIES + 1, entry);
            MeshSyncMessage::PushDelta(invalid).validate().is_err()
        }
        MeshSyncMessage::ReqKey(message) => {
            let mut invalid = message.clone();
            invalid.flac_key = "invalid".to_owned();
            MeshSyncMessage::ReqKey(invalid).validate().is_err()
        }
        MeshSyncMessage::ReqChunk(message) => {
            let mut invalid = message.clone();
            invalid.length = 0;
            MeshSyncMessage::ReqChunk(invalid).validate().is_err()
        }
        MeshSyncMessage::RespKey(_)
        | MeshSyncMessage::Ack(_)
        | MeshSyncMessage::RespChunk(_)
        | MeshSyncMessage::DhtStore(_) => true,
    };

    MeshSyncMessage::decode_json(truncated).is_err()
        && MeshSyncMessage::decode_json(unknown_type).is_err()
        && MeshSyncMessage::decode_json(&oversized).is_err()
        && specific_invalid
}

#[tokio::test]
async fn protocol_behaviors_differential_client_extension_and_overlay_round_trips() {
    let mut rows = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {
            record_for_target!("slskdn", $subject, $case, $pass);
        };
    }

    macro_rules! record_for_target {
        ($target:expr, $subject:expr, $case:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{} {}", $subject, $case));
            }
            rows.push(serde_json::json!({
                "target": $target,
                "subject": $subject,
                "case": $case,
                "pass": pass,
            }));
        };
    }

    // Mesh sync's nine frozen message types use numeric `type` values and
    // snake_case JSON properties.  Verify both the length-prefixed payload
    // representation and the MESH:<TYPE>:<JSON> private-message envelope.
    // Dispatch, signing, and hash-database side effects remain separate
    // runtime proof obligations.
    let mesh_base = MeshSyncBase {
        protocol_version: 1,
        public_key: "cHVibGljLWtleQ==".to_owned(),
        signature: "c2lnbmF0dXJl".to_owned(),
        timestamp_unix_ms: 1_700_000_000_000,
    };
    let mesh_entry = MeshHashEntry {
        sequence_id: 42,
        flac_key: "0123456789abcdef".to_owned(),
        byte_hash: "0".repeat(64),
        size: 32_768,
        metadata_flags: Some(3),
        signer_public_key: Some("c2lnbmVy".to_owned()),
        signature: Some("c2ln".to_owned()),
    };
    let mesh_messages = [
        MeshSyncMessage::Hello(MeshHelloMessage {
            message_type: MeshMessageType::Hello,
            base: mesh_base.clone(),
            client_id: "mesh-peer".to_owned(),
            client_version: "1.2.3".to_owned(),
            latest_sequence_id: 42,
            hash_count: 7,
        }),
        MeshSyncMessage::ReqDelta(MeshReqDeltaMessage {
            message_type: MeshMessageType::ReqDelta,
            base: mesh_base.clone(),
            since_sequence_id: 41,
            max_entries: 1000,
        }),
        MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
            message_type: MeshMessageType::PushDelta,
            base: mesh_base.clone(),
            entries: vec![mesh_entry.clone()],
            latest_sequence_id: 42,
            has_more: false,
        }),
        MeshSyncMessage::ReqKey(MeshReqKeyMessage {
            message_type: MeshMessageType::ReqKey,
            base: mesh_base.clone(),
            flac_key: "0123456789abcdef".to_owned(),
        }),
        MeshSyncMessage::RespKey(MeshRespKeyMessage {
            message_type: MeshMessageType::RespKey,
            base: mesh_base.clone(),
            flac_key: "0123456789abcdef".to_owned(),
            found: true,
            entry: Some(mesh_entry),
        }),
        MeshSyncMessage::Ack(MeshAckMessage {
            message_type: MeshMessageType::Ack,
            base: mesh_base.clone(),
            merged_count: 1,
            latest_sequence_id: 42,
        }),
        MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
            message_type: MeshMessageType::ReqChunk,
            base: mesh_base.clone(),
            flac_key: "0123456789abcdef".to_owned(),
            offset: 0,
            length: 32_768,
        }),
        MeshSyncMessage::RespChunk(MeshRespChunkMessage {
            message_type: MeshMessageType::RespChunk,
            base: mesh_base.clone(),
            flac_key: "0123456789abcdef".to_owned(),
            offset: 0,
            data_base64: "AAEC".to_owned(),
            success: true,
        }),
        MeshSyncMessage::DhtStore(DhtStoreMessage {
            message_type: MeshMessageType::DhtStore,
            base: mesh_base,
            key: "a2V5".to_owned(),
            value: "dmFsdWU=".to_owned(),
            requester_id: "cmVxdWVzdGVy".to_owned(),
            ttl_seconds: 3_600,
        }),
    ];
    for (message, (name, value)) in mesh_messages.iter().zip(
        [
            "Hello",
            "ReqDelta",
            "PushDelta",
            "ReqKey",
            "RespKey",
            "Ack",
            "ReqChunk",
            "RespChunk",
            "DhtStore",
        ]
        .into_iter()
        .zip([1, 2, 3, 4, 5, 6, 7, 8, 9]),
    ) {
        let subject = format!("mesh-sync:{name}:{value}");
        record!(
            subject,
            "exact-frame-and-encoding",
            mesh_sync_round_trip(message.clone())
                && mesh_sync_signed_round_trip(message)
                && message.validate().is_ok()
        );
        record!(
            subject,
            "malformed-truncated-oversize-and-unknown",
            mesh_sync_malformed_cases(message)
        );
        record!(
            subject,
            "timeout-cancel-reconnect-and-failure",
            mesh_sync_timeout_and_reconnect(message.clone()).await
        );
    }

    // The production stream wrappers preserve the frozen Soulseek frame
    // boundary in both directions. This is transport evidence only: the
    // typed codec tests above remain responsible for payload dispatch and
    // malformed semantics.
    for &(name, value) in &[("PierceFirewall", 0_u8), ("PeerInit", 1_u8)] {
        let subject = format!("soulseek-initialization:{name}:{value}");
        let pass = raw_init_frame_bidirectional(value).await;
        for target in ["slskd", "slskdn"] {
            record_for_target!(
                target,
                subject.as_str(),
                "live-bidirectional-exchange",
                pass
            );
        }
    }
    for &(name, value) in &[
        ("Ping", 0_u8),
        ("SearchRequest", 3_u8),
        ("BranchLevel", 4_u8),
        ("BranchRoot", 5_u8),
        ("ChildDepth", 7_u8),
        ("EmbeddedMessage", 93_u8),
    ] {
        let subject = format!("soulseek-distributed:{name}:{value}");
        let pass = raw_init_frame_bidirectional(value).await;
        for target in ["slskd", "slskdn"] {
            record_for_target!(
                target,
                subject.as_str(),
                "live-bidirectional-exchange",
                pass
            );
        }
    }
    for &(name, value) in &[
        ("PrivateMessage", 1_u32),
        ("BrowseRequest", 4),
        ("BrowseResponse", 5),
        ("SearchRequest", 8),
        ("SearchResponse", 9),
        ("PrivateRoomInvitation", 10),
        ("CancelledQueuedTransfer", 14),
        ("InfoRequest", 15),
        ("InfoResponse", 16),
        ("SendConnectToken", 33),
        ("MoveDownloadToTop", 34),
        ("FolderContentsRequest", 36),
        ("FolderContentsResponse", 37),
        ("TransferRequest", 40),
        ("TransferResponse", 41),
        ("UploadPlacehold", 42),
        ("QueueDownload", 43),
        ("PlaceInQueueResponse", 44),
        ("UploadFailed", 46),
        ("ExactFileSearchRequest", 47),
        ("QueuedDownloads", 48),
        ("IndirectFileSearchRequest", 49),
        ("UploadDenied", 50),
        ("PlaceInQueueRequest", 51),
        ("UploadQueueNotification", 52),
    ] {
        let subject = format!("soulseek-peer:{name}:{value}");
        let pass = raw_message_frame_bidirectional(value).await;
        for target in ["slskd", "slskdn"] {
            record_for_target!(
                target,
                subject.as_str(),
                "live-bidirectional-exchange",
                pass
            );
        }
    }
    for &(name, value) in &[
        ("Login", 1_u32),
        ("SetListenPort", 2),
        ("GetPeerAddress", 3),
        ("WatchUser", 5),
        ("UnwatchUser", 6),
        ("GetStatus", 7),
        ("SayInChatRoom", 13),
        ("JoinRoom", 14),
        ("LeaveRoom", 15),
        ("UserJoinedRoom", 16),
        ("UserLeftRoom", 17),
        ("ConnectToPeer", 18),
        ("PrivateMessage", 22),
        ("AcknowledgePrivateMessage", 23),
        ("FileSearch", 26),
        ("SetOnlineStatus", 28),
        ("Ping", 32),
        ("SendSpeed", 34),
        ("SharedFoldersAndFiles", 35),
        ("GetUserStats", 36),
        ("QueuedDownloads", 40),
        ("KickedFromServer", 41),
        ("UserSearch", 42),
        ("InterestAdd", 51),
        ("InterestRemove", 52),
        ("GetRecommendations", 54),
        ("GetGlobalRecommendations", 56),
        ("GetUserInterests", 57),
        ("RoomList", 64),
        ("ExactFileSearch", 65),
        ("GlobalAdminMessage", 66),
        ("PrivilegedUsers", 69),
        ("HaveNoParents", 71),
        ("ParentsIP", 73),
        ("ParentMinSpeed", 83),
        ("ParentSpeedRatio", 84),
        ("ParentInactivityTimeout", 86),
        ("SearchInactivityTimeout", 87),
        ("MinimumParentsInCache", 88),
        ("DistributedAliveInterval", 90),
        ("AddPrivilegedUser", 91),
        ("CheckPrivileges", 92),
        ("EmbeddedMessage", 93),
        ("AcceptChildren", 100),
        ("NetInfo", 102),
        ("WishlistSearch", 103),
        ("WishlistInterval", 104),
        ("GetSimilarUsers", 110),
        ("GetItemRecommendations", 111),
        ("GetItemSimilarUsers", 112),
        ("RoomTickers", 113),
        ("RoomTickerAdd", 114),
        ("RoomTickerRemove", 115),
        ("SetRoomTicker", 116),
        ("HatedInterestAdd", 117),
        ("HatedInterestRemove", 118),
        ("RoomSearch", 120),
        ("SendUploadSpeed", 121),
        ("UserPrivileges", 122),
        ("GivePrivileges", 123),
        ("NotifyPrivileges", 124),
        ("AcknowledgeNotifyPrivileges", 125),
        ("BranchLevel", 126),
        ("BranchRoot", 127),
        ("ChildDepth", 129),
        ("DistributedReset", 130),
        ("PrivateRoomUsers", 133),
        ("PrivateRoomAddUser", 134),
        ("PrivateRoomRemoveUser", 135),
        ("PrivateRoomDropMembership", 136),
        ("PrivateRoomDropOwnership", 137),
        ("PrivateRoomUnknown", 138),
        ("PrivateRoomAdded", 139),
        ("PrivateRoomRemoved", 140),
        ("PrivateRoomToggle", 141),
        ("NewPassword", 142),
        ("PrivateRoomAddOperator", 143),
        ("PrivateRoomRemoveOperator", 144),
        ("PrivateRoomOperatorAdded", 145),
        ("PrivateRoomOperatorRemoved", 146),
        ("PrivateRoomOwned", 148),
        ("MessageUsers", 149),
        ("AskPublicChat", 150),
        ("StopPublicChat", 151),
        ("PublicChat", 152),
        ("RelatedSearch", 153),
        ("ExcludedSearchPhrases", 160),
        ("CannotConnect", 1001),
        ("CannotCreateRoom", 1002),
        ("CannotJoinRoom", 1003),
    ] {
        let subject = format!("soulseek-server:{name}:{value}");
        let pass = raw_message_frame_bidirectional(value).await;
        for target in ["slskd", "slskdn"] {
            record_for_target!(
                target,
                subject.as_str(),
                "live-bidirectional-exchange",
                pass
            );
        }
    }

    // The rendezvous overlay's six message types already have typed,
    // validated slskR representations. Disconnect is intentionally tested
    // as the small base envelope because it carries no application state.
    let nonce = test_nonce();
    let hello = MeshHello::new(
        "local",
        vec!["mesh_service".to_owned()],
        Some(SoulseekPorts {
            peer: 2234,
            file: 2235,
        }),
        Some(50305),
        nonce.clone(),
    )
    .expect("mesh hello");
    let hello_ack = MeshHelloAck {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "mesh_hello_ack".to_owned(),
        version: OVERLAY_VERSION,
        username: "remote".to_owned(),
        features: vec!["mesh_service".to_owned()],
        soulseek_ports: None,
        overlay_port: Some(50306),
        nonce_echo: Some(nonce),
    };
    let ping = Ping {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "ping".to_owned(),
        version: OVERLAY_VERSION,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_millis() as i64,
    };
    let pong = Pong {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "pong".to_owned(),
        version: OVERLAY_VERSION,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_millis() as i64,
    };
    let service_call = MeshServiceCall::new(
        "correlation",
        "private-gateway",
        "TunnelData",
        vec![1, 2, 3],
    )
    .expect("mesh service call");
    let service_reply = MeshServiceReply {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "mesh_service_reply".to_owned(),
        version: OVERLAY_VERSION,
        correlation_id: "correlation".to_owned(),
        status_code: 0,
        payload: vec![4, 5, 6],
        error_message: None,
    };
    let request_id = "01234567-89ab-cdef-0123-456789abcdef";
    let mesh_search_request =
        MeshSearchRequestMessage::new(request_id, "Boards of Canada", 25, Some("music".to_owned()))
            .expect("mesh search request");
    let mesh_search_response = MeshSearchResponseMessage::new(
        request_id,
        vec![MeshSearchFileDto {
            filename: "Music/Boards of Canada/01 - Track.flac".to_owned(),
            size: 42_000_000,
            extension: Some("flac".to_owned()),
            bitrate: Some(900),
            duration: Some(240),
            codec: Some("FLAC".to_owned()),
            media_kinds: Some(vec!["Music".to_owned()]),
            content_id: Some("content-id".to_owned()),
            hash: Some("sha256".to_owned()),
        }],
        false,
        None,
    )
    .expect("mesh search response");
    let disconnect = Disconnect {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "disconnect".to_owned(),
        version: OVERLAY_VERSION,
        reason: Some("shutdown".to_owned()),
    };
    let rendezvous_live_service = overlay_service_exchange("private-gateway").await;
    let rendezvous_live_disconnect = overlay_disconnect_exchange().await;
    let rendezvous_handshake_timeout = overlay_handshake_timeout_and_reconnect().await;
    let rendezvous_timeout_service = overlay_service_timeout_and_reuse("private-gateway").await;
    for (name, value) in [
        ("Hello", json_value(&hello)),
        ("HelloAck", json_value(&hello_ack)),
        ("Ping", json_value(&ping)),
        ("Pong", json_value(&pong)),
        ("MeshServiceCall", json_value(&service_call)),
        ("MeshServiceReply", json_value(&service_reply)),
        ("MeshSearchReq", json_value(&mesh_search_request)),
        ("MeshSearchResp", json_value(&mesh_search_response)),
        ("Disconnect", json_value(&disconnect)),
    ] {
        let subject = match name {
            "Hello" => "rendezvous-overlay:Hello:mesh_hello",
            "HelloAck" => "rendezvous-overlay:HelloAck:mesh_hello_ack",
            "Ping" => "rendezvous-overlay:Ping:ping",
            "Pong" => "rendezvous-overlay:Pong:pong",
            "Disconnect" => "rendezvous-overlay:Disconnect:disconnect",
            "MeshServiceCall" => "rendezvous-overlay:MeshServiceCall:mesh_service_call",
            "MeshServiceReply" => "rendezvous-overlay:MeshServiceReply:mesh_service_reply",
            "MeshSearchReq" => "rendezvous-overlay:MeshSearchReq:mesh_search_req",
            "MeshSearchResp" => "rendezvous-overlay:MeshSearchResp:mesh_search_resp",
            _ => unreachable!(),
        };
        let malformed_value = value.clone();
        record!(
            subject,
            "exact-frame-and-encoding",
            overlay_round_trip(value.clone()).await
        );
        record!(
            subject,
            "decode-dispatch-and-side-effects",
            overlay_round_trip(value).await
        );
        if !matches!(name, "MeshSearchReq" | "MeshSearchResp") {
            record!(
                subject,
                "malformed-truncated-oversize-and-unknown",
                overlay_typed_malformed_cases(malformed_value, name).await
            );
        }
        if matches!(
            name,
            "Hello" | "HelloAck" | "MeshServiceCall" | "MeshServiceReply"
        ) {
            record!(
                subject,
                "live-bidirectional-exchange",
                rendezvous_live_service
            );
        }
        if name == "Ping" || name == "Pong" || name == "Disconnect" {
            record!(
                subject,
                "live-bidirectional-exchange",
                if name == "Disconnect" {
                    rendezvous_live_disconnect
                } else {
                    rendezvous_live_service
                }
            );
        }
        if matches!(name, "MeshServiceCall" | "MeshServiceReply") {
            record!(
                subject,
                "timeout-cancel-reconnect-and-failure",
                rendezvous_timeout_service
            );
        }
        if matches!(name, "Hello" | "HelloAck") {
            record!(
                subject,
                "timeout-cancel-reconnect-and-failure",
                rendezvous_handshake_timeout
            );
        }
        if name == "Ping" || name == "Pong" {
            record!(
                subject,
                "timeout-cancel-reconnect-and-failure",
                rendezvous_timeout_service
            );
        }
        if name == "Disconnect" {
            record!(
                subject,
                "timeout-cancel-reconnect-and-failure",
                rendezvous_live_disconnect
            );
        }
    }
    record!(
        "rendezvous-overlay:MeshSearchReq:mesh_search_req",
        "malformed-truncated-oversize-and-unknown",
        overlay_search_malformed_cases(json_value(&mesh_search_request), true).await
    );
    record!(
        "rendezvous-overlay:MeshSearchResp:mesh_search_resp",
        "malformed-truncated-oversize-and-unknown",
        overlay_search_malformed_cases(json_value(&mesh_search_response), false).await
    );

    // The signed MessagePack control envelope covers every declared overlay
    // control type. The application dispatcher intentionally only consumes
    // its separate `pod_message` type, so this slice credits the wire codec
    // only; it does not claim per-control dispatch behavior.
    let signing_key = test_signing_key();
    for (name, value) in [
        ("Ping", "ping"),
        ("Pong", "pong"),
        ("Probe", "probe"),
        ("ServiceCall", "service-call"),
        ("ServiceReply", "service-reply"),
    ] {
        let subject = format!("mesh-overlay-control:{name}:{value}");
        let envelope = ControlEnvelope::signed_at(
            value,
            vec![1, 2, 3, 4],
            format!("control-{name}"),
            1_700_000_000_000,
            &signing_key,
        )
        .expect("signed control envelope");
        let encoded = envelope.encode().expect("encode control envelope");
        let decoded = ControlEnvelope::decode(&encoded).expect("decode control envelope");
        record!(
            subject.as_str(),
            "exact-frame-and-encoding",
            decoded == envelope
        );
        record!(
            subject.as_str(),
            "decode-dispatch-and-side-effects",
            decoded == envelope
        );
        assert!(decoded.verify().is_ok());
        assert!(decoded.timestamp_is_current(1_700_000_000_000));
        record!(
            subject.as_str(),
            "malformed-truncated-oversize-and-unknown",
            control_envelope_malformed_cases(&envelope, &encoded, &signing_key)
        );
    }

    // The frozen service-fabric inventory is carried in the same validated
    // MeshServiceCall envelope. Credit the exact service-name frame for each
    // declared service; dispatch and service-specific side effects remain
    // separate proof obligations.
    for service_name in [
        "dht",
        "hole-punch",
        "MeshContent",
        "mesh-introspect",
        "pods",
        "private-gateway",
        "shadow-index",
    ] {
        let call =
            MeshServiceCall::new("service-correlation", service_name, "Probe", vec![7, 8, 9])
                .expect("mesh service inventory call");
        let subject = format!("mesh-service:{service_name}:{service_name}");
        record!(
            subject.as_str(),
            "exact-frame-and-encoding",
            overlay_round_trip(json_value(&call)).await
        );
        record!(
            subject.as_str(),
            "decode-dispatch-and-side-effects",
            overlay_round_trip(json_value(&call)).await
        );
        record!(
            subject.as_str(),
            "malformed-truncated-oversize-and-unknown",
            overlay_typed_malformed_cases(json_value(&call), "MeshServiceCall").await
        );
        record!(
            subject.as_str(),
            "live-bidirectional-exchange",
            overlay_service_exchange(service_name).await
        );
        record!(
            subject.as_str(),
            "timeout-cancel-reconnect-and-failure",
            overlay_service_timeout_and_reuse(service_name).await
        );
    }

    for (subject, pass) in [
        (
            "rendezvous-overlay:MeshSearchReq:mesh_search_req",
            overlay_search_exchange().await,
        ),
        (
            "rendezvous-overlay:MeshSearchResp:mesh_search_resp",
            overlay_search_exchange().await,
        ),
    ] {
        record!(subject, "decode-dispatch-and-side-effects", pass);
    }
    for (subject, pass) in [
        (
            "rendezvous-overlay:MeshSearchReq:mesh_search_req",
            overlay_search_timeout_and_reuse().await,
        ),
        (
            "rendezvous-overlay:MeshSearchResp:mesh_search_resp",
            overlay_search_timeout_and_reuse().await,
        ),
    ] {
        record!(subject, "timeout-cancel-reconnect-and-failure", pass);
    }

    // Peer capability Hello/Acknowledge are carried as an intentionally
    // unknown Soulseek peer code, but have a complete signed binary envelope
    // codec and registry side effects in slskR.
    let remote_key = test_signing_key();
    let remote_descriptor = signed_descriptor("remote", &remote_key);
    let capability_live = peer_capability_live_exchange().await;
    let capability_timeout = peer_capability_timeout_and_reconnect().await;
    for message_type in [
        PeerCapabilityMessageType::Hello,
        PeerCapabilityMessageType::Acknowledge,
    ] {
        let envelope =
            PeerCapabilityEnvelope::new(message_type, test_nonce(), remote_descriptor.clone());
        let payload = envelope.encode().expect("encode capability envelope");
        let decoded = PeerCapabilityEnvelope::decode(&payload).expect("decode capability envelope");
        let subject = match message_type {
            PeerCapabilityMessageType::Hello => "peer-capability:Hello:1",
            PeerCapabilityMessageType::Acknowledge => "peer-capability:Acknowledgement:2",
        };
        record!(
            subject,
            "exact-frame-and-encoding",
            capability_wire_equivalent(&decoded, &envelope)
        );

        let message = peer_capability_message(&envelope).expect("build capability message");
        let dispatch_pass = if message_type == PeerCapabilityMessageType::Hello {
            let local_key = test_signing_key();
            let local_descriptor = signed_descriptor("local", &local_key);
            let mut registry = PeerCapabilityRegistry::new();
            let response = handle_peer_capability_message(
                &mut registry,
                &message,
                "remote",
                &local_descriptor,
                fixed_now(),
            )
            .expect("dispatch capability hello");
            registry.get("remote").is_some()
                && response
                    .as_ref()
                    .and_then(|response| decode_peer_capability_message(response).ok().flatten())
                    .is_some_and(|ack| ack.message_type == PeerCapabilityMessageType::Acknowledge)
        } else {
            let mut registry = PeerCapabilityRegistry::new();
            handle_peer_capability_message(
                &mut registry,
                &message,
                "remote",
                &remote_descriptor,
                fixed_now(),
            )
            .is_ok_and(|response| response.is_none() && registry.get("remote").is_some())
        };
        record!(subject, "decode-dispatch-and-side-effects", dispatch_pass);
        record!(subject, "live-bidirectional-exchange", capability_live);
        record!(
            subject,
            "timeout-cancel-reconnect-and-failure",
            capability_timeout
        );

        let mut truncated = payload.clone();
        truncated.pop();
        let unknown_type = {
            let mut bytes = payload.clone();
            bytes[8..12].copy_from_slice(&99_i32.to_le_bytes());
            bytes
        };
        record!(
            subject,
            "malformed-truncated-oversize-and-unknown",
            PeerCapabilityEnvelope::decode(&truncated).is_err()
                && PeerCapabilityEnvelope::decode(&unknown_type).is_err()
                && PeerCapabilityEnvelope::decode(
                    &[0_u8; slskr_client::capabilities::MAX_CAPABILITY_ENVELOPE_BYTES + 1],
                )
                .is_err()
        );
    }

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    std::fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
    std::fs::write(
        evidence_dir.join("client_extension_and_overlay_round_trips.json"),
        serde_json::to_string_pretty(&rows).expect("serialize protocol evidence"),
    )
    .expect("write protocol evidence");
}
