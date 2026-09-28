use slskr_client::{
    connection::ConnectionKind,
    io::{
        write_connection_kind, write_init_frame, write_obfuscated_init_frame,
        write_obfuscated_init_frame_with_key,
    },
    listener::{
        demux_incoming, demux_obfuscated_incoming, demux_shared_incoming, IncomingConnection,
        Listener, SharedIncomingConnection,
    },
    peer_cache::MAX_PEER_USERNAME_BYTES,
    ClientError,
};
use slskr_protocol::{
    decode_rotated,
    distributed::DistributedMessage,
    encode_rotated,
    init::{InitCode, InitMessage, MAX_PEER_INIT_FRAME_LEN},
    peer::PeerMessage,
    InitFrame,
};
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Duration;

#[tokio::test]
async fn demuxes_tagged_peer_message_connection() {
    let (mut client, server) = duplex(128);
    write_connection_kind(&mut client, ConnectionKind::PeerMessages)
        .await
        .unwrap();

    let incoming = demux_incoming(server).await.unwrap();
    assert!(matches!(incoming, IncomingConnection::PeerMessages(_)));
}

#[tokio::test]
async fn demuxes_tagged_file_transfer_connection() {
    let (mut client, server) = duplex(128);
    write_connection_kind(&mut client, ConnectionKind::FileTransfer)
        .await
        .unwrap();

    let incoming = demux_incoming(server).await.unwrap();
    let IncomingConnection::FileTransfer(mut connection) = incoming else {
        panic!("expected file transfer");
    };

    slskr_client::file_transfer::FileTransferConnection::new(client)
        .send_token(123)
        .await
        .unwrap();
    assert_eq!(connection.receive_token().await.unwrap(), 123);
}

#[tokio::test]
async fn demuxes_tagged_distributed_connection() {
    let (mut client, server) = duplex(128);
    write_connection_kind(&mut client, ConnectionKind::Distributed)
        .await
        .unwrap();

    let incoming = demux_incoming(server).await.unwrap();
    assert!(matches!(incoming, IncomingConnection::Distributed(_)));
}

#[tokio::test]
async fn demuxes_peer_init_and_leaves_stream_after_init_frame() {
    let (mut client, server) = duplex(512);
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };
    write_init_frame(&mut client, &init.encode().unwrap())
        .await
        .unwrap();

    let message = PeerMessage::QueueUpload {
        filename: "Music/file.flac".to_owned(),
    };
    slskr_client::io::write_message_frame(&mut client, &message.encode().unwrap())
        .await
        .unwrap();

    let incoming = demux_incoming(server).await.unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        stream,
        obfuscated,
    } = incoming
    else {
        panic!("expected peer init");
    };
    assert_eq!(username, "peer");
    assert_eq!(kind, ConnectionKind::PeerMessages);
    assert_eq!(token, 0);
    assert!(!obfuscated);

    let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
    assert_eq!(peer.receive().await.unwrap(), message);
}

#[tokio::test]
async fn demux_rejects_malformed_plain_peer_init_identities() {
    for (username, expected_oversized) in [
        ("   ".to_owned(), false),
        ("x".repeat(MAX_PEER_USERNAME_BYTES + 1), true),
    ] {
        let (mut client, server) = duplex(8192);
        let init = InitMessage::PeerInit {
            username,
            connection_type: "P".to_owned(),
            token: 0,
        };
        write_init_frame(&mut client, &init.encode().unwrap())
            .await
            .unwrap();

        let error = demux_incoming(server).await.unwrap_err();
        assert!(if expected_oversized {
            matches!(error, ClientError::PeerUsernameTooLong { .. })
        } else {
            matches!(error, ClientError::BlankPeerUsername)
        });
    }
}

#[tokio::test]
async fn demuxes_obfuscated_peer_message_connection() {
    let (mut client, server) = duplex(512);
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };
    write_obfuscated_init_frame(&mut client, &init.encode().unwrap())
        .await
        .unwrap();

    let message = PeerMessage::UserInfoRequest;
    slskr_client::io::write_obfuscated_message_frame(&mut client, &message.encode().unwrap())
        .await
        .unwrap();

    let incoming = demux_obfuscated_incoming(server).await.unwrap();
    let IncomingConnection::ObfuscatedPeerMessages(mut peer) = incoming else {
        panic!("expected obfuscated peer messages");
    };
    assert_eq!(peer.receive().await.unwrap(), message);
}

#[tokio::test]
async fn shared_demux_accepts_plain_and_obfuscated_initialization() {
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };

    let (mut plain_client, plain_server) = tcp_pair().await;
    write_init_frame(&mut plain_client, &init.encode().unwrap())
        .await
        .unwrap();
    let plain = demux_shared_incoming(plain_server).await.unwrap();
    assert!(matches!(
        plain,
        IncomingConnection::PeerInit {
            obfuscated: false,
            kind: ConnectionKind::PeerMessages,
            ..
        }
    ));

    let (mut obfuscated_client, obfuscated_server) = tcp_pair().await;
    write_obfuscated_init_frame_with_key(
        &mut obfuscated_client,
        &init.encode().unwrap(),
        0x4aee_9414,
    )
    .await
    .unwrap();
    let obfuscated = demux_shared_incoming(obfuscated_server).await.unwrap();
    assert!(matches!(
        obfuscated,
        IncomingConnection::ObfuscatedPeerMessages(_)
    ));
}

#[test]
fn shared_wire_bytes_can_form_valid_plain_and_obfuscated_init_frames() {
    let obfuscated_message = InitMessage::PeerInit {
        username: "a".repeat(252),
        connection_type: "P".to_owned(),
        token: 0,
    };
    let obfuscated_frame = obfuscated_message.encode().unwrap().encode().unwrap();
    let wire = encode_rotated(&obfuscated_frame, 5);

    assert_eq!(wire.len(), 274);

    let plain_frame = InitFrame::decode(&wire[..9]).unwrap();
    assert_eq!(
        InitMessage::decode(plain_frame).unwrap(),
        InitMessage::PierceFirewall { token: 0x1500_0001 }
    );

    let decoded_obfuscated_frame = decode_rotated(&wire).unwrap();
    let decoded_obfuscated_frame = InitFrame::decode(&decoded_obfuscated_frame).unwrap();
    assert_eq!(
        InitMessage::decode(decoded_obfuscated_frame).unwrap(),
        obfuscated_message
    );
}

#[tokio::test]
async fn shared_demux_rejects_dual_valid_init_prefix() {
    let obfuscated_message = InitMessage::PeerInit {
        username: "a".repeat(252),
        connection_type: "P".to_owned(),
        token: 0,
    };
    let wire = encode_rotated(&obfuscated_message.encode().unwrap().encode().unwrap(), 5);
    let (mut client, server) = tcp_pair().await;
    client.write_all(&wire).await.unwrap();

    let error = demux_shared_incoming(server).await.unwrap_err();
    assert!(matches!(
        error,
        slskr_client::ClientError::AmbiguousInitFrame
    ));
}

#[tokio::test]
async fn shared_demux_rejects_nested_collision_before_reading_body() {
    let message = InitMessage::PeerInit {
        username: "a".repeat(248),
        connection_type: "F".to_owned(),
        token: 0,
    };
    let inner = message.encode().unwrap().encode().unwrap();
    let nested = InitFrame::new(inner[0], inner[1..].to_vec());
    let wire = encode_rotated(&nested.encode().unwrap(), 5);
    assert!(matches!(
        InitMessage::decode(InitFrame::decode(&wire[..9]).unwrap()).unwrap(),
        InitMessage::PierceFirewall { .. }
    ));

    // The same complete wire is a supported nested obfuscated file init.
    let (mut writer, reader) = duplex(512);
    writer.write_all(&wire).await.unwrap();
    assert!(matches!(
        demux_obfuscated_incoming(reader).await.unwrap(),
        IncomingConnection::PeerInit {
            token: 0,
            kind: ConnectionKind::FileTransfer,
            obfuscated: true,
            ..
        }
    ));

    // Keep the sender open: reject at nine bytes, without waiting for a body.
    let (mut writer, reader) = duplex(64);
    writer.write_all(&wire[..9]).await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(1), demux_shared_incoming(reader))
        .await
        .expect("ambiguous header must not wait for its body");
    assert!(matches!(result, Err(ClientError::AmbiguousInitFrame)));
}

#[tokio::test]
async fn shared_demux_preserves_unambiguous_nested_file_init_and_following_bytes() {
    let message = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "F".to_owned(),
        token: 42,
    };
    let inner = message.encode().unwrap().encode().unwrap();
    let nested = InitFrame::new(inner[0], inner[1..].to_vec());
    let wire = encode_rotated(&nested.encode().unwrap(), 0x8000_0000);
    let (mut writer, reader) = duplex(512);
    writer.write_all(&wire).await.unwrap();
    writer.write_all(b"sentinel").await.unwrap();
    let IncomingConnection::PeerInit {
        token,
        mut stream,
        kind: ConnectionKind::FileTransfer,
        obfuscated: true,
        ..
    } = demux_shared_incoming(reader).await.unwrap()
    else {
        panic!("expected nested obfuscated file init");
    };
    assert_eq!(token, 42);
    let mut following = [0u8; 8];
    stream.read_exact(&mut following).await.unwrap();
    assert_eq!(&following, b"sentinel");
}

#[tokio::test]
async fn obfuscated_init_writer_avoids_plain_length_and_tls_prefixes() {
    let frame = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    }
    .encode()
    .unwrap();

    for _ in 0..32 {
        let (mut writer, mut reader) = duplex(128);
        write_obfuscated_init_frame(&mut writer, &frame)
            .await
            .unwrap();
        let key = reader.read_u32_le().await.unwrap();
        assert!(key as usize > slskr_client::io::DEFAULT_MAX_FRAME_LEN);
        assert_ne!(key.to_le_bytes()[..2], [0x16, 0x03]);
    }
}

#[tokio::test]
async fn dedicated_listener_accepts_raw_connection_kind() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client_task = tokio::spawn(async move {
        let mut stream = TcpStream::connect(address).await.unwrap();
        write_connection_kind(&mut stream, ConnectionKind::Distributed)
            .await
            .unwrap();
    });

    let (incoming, _) = listener.accept().await.unwrap();
    assert!(matches!(incoming, IncomingConnection::Distributed(_)));
    client_task.await.unwrap();
}

#[tokio::test]
async fn shared_demux_accepts_obfuscated_keys_starting_with_connection_kind_bytes() {
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };

    for first in [b'P', b'F', b'D'] {
        let key = u32::from_le_bytes([first, 0, 0, 0]);
        let (mut client, server) = tcp_pair().await;
        write_obfuscated_init_frame_with_key(&mut client, &init.encode().unwrap(), key)
            .await
            .unwrap();

        let incoming = demux_shared_incoming(server).await.unwrap();
        assert!(matches!(
            incoming,
            IncomingConnection::ObfuscatedPeerMessages(_)
        ));
    }
}

#[tokio::test]
async fn shared_demux_accepts_plain_init_lengths_starting_with_connection_kind_bytes() {
    for (first, username_len) in [(b'P', 66), (b'F', 56), (b'D', 54)] {
        let init = InitMessage::PeerInit {
            username: "a".repeat(username_len),
            connection_type: "P".to_owned(),
            token: 0,
        };
        let encoded = init.encode().unwrap();
        let wire = encoded.encode().unwrap();
        assert_eq!(wire[0], first);

        let (mut client, server) = tcp_pair().await;
        write_init_frame(&mut client, &encoded).await.unwrap();

        let incoming = demux_shared_incoming(server).await.unwrap();
        let IncomingConnection::PeerInit {
            username,
            kind,
            obfuscated,
            ..
        } = incoming
        else {
            panic!("expected a plain peer init");
        };
        assert_eq!(username.len(), username_len);
        assert_eq!(kind, ConnectionKind::PeerMessages);
        assert!(!obfuscated);
    }
}

#[tokio::test]
async fn shared_demux_rejects_oversized_peer_init_before_buffering_body() {
    let (mut client, server) = duplex(64);
    client
        .write_u32_le(slskr_client::io::DEFAULT_MAX_FRAME_LEN as u32)
        .await
        .unwrap();
    client.write_u8(InitCode::PeerInit.as_u8()).await.unwrap();
    client.write_all(&[0, 0, 0]).await.unwrap();

    let error = tokio::time::timeout(Duration::from_secs(1), demux_shared_incoming(server))
        .await
        .expect("shared demux should reject the bounded header without waiting")
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::FrameTooLarge {
            length,
            max: MAX_PEER_INIT_FRAME_LEN,
        } if length == slskr_client::io::DEFAULT_MAX_FRAME_LEN
    ));
}

#[tokio::test]
async fn shared_mesh_listener_routes_tls_without_consuming_record_bytes() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client_task = tokio::spawn(async move {
        let mut stream = TcpStream::connect(address).await.unwrap();
        stream
            .write_all(&[0x16, 0x03, 0x01, 0x00, 0x04, 0x01])
            .await
            .unwrap();
        stream
    });

    let (incoming, remote_addr) = listener.accept_shared_mesh().await.unwrap();
    assert!(remote_addr.ip().is_loopback());
    let SharedIncomingConnection::MeshOverlay(mut stream) = incoming else {
        panic!("expected mesh overlay connection");
    };
    let mut prefix = [0_u8; 6];
    stream.read_exact(&mut prefix).await.unwrap();
    assert_eq!(prefix, [0x16, 0x03, 0x01, 0x00, 0x04, 0x01]);
    drop(stream);
    client_task.await.unwrap();
}

#[tokio::test]
async fn shared_mesh_listener_preserves_soulseek_demux() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client_task = tokio::spawn(async move {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let init = InitMessage::PeerInit {
            username: "peer".to_owned(),
            connection_type: "P".to_owned(),
            token: 0,
        };
        write_init_frame(&mut stream, &init.encode().unwrap())
            .await
            .unwrap();
    });

    let (incoming, _) = listener.accept_shared_mesh().await.unwrap();
    assert!(matches!(
        incoming,
        SharedIncomingConnection::Soulseek(IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            obfuscated: false,
            ..
        })
    ));
    client_task.await.unwrap();
}

#[tokio::test]
async fn shared_mesh_listener_accepts_plain_frame_with_tls_like_first_two_bytes() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let username = "p".repeat(776);
    let expected_username = username.clone();
    let client_task = tokio::spawn(async move {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let init = InitMessage::PeerInit {
            username,
            connection_type: "P".to_owned(),
            token: 0,
        };
        let frame = init.encode().unwrap();
        assert_eq!(&frame.encode().unwrap()[..2], &[0x16, 0x03]);
        write_init_frame(&mut stream, &frame).await.unwrap();
    });

    let (incoming, _) = listener.accept_shared_mesh().await.unwrap();
    let SharedIncomingConnection::Soulseek(IncomingConnection::PeerInit {
        username,
        kind: ConnectionKind::PeerMessages,
        obfuscated: false,
        ..
    }) = incoming
    else {
        panic!("expected plain peer init on the shared listener");
    };
    assert_eq!(username, expected_username);
    client_task.await.unwrap();
}

#[tokio::test]
async fn shared_mesh_listener_rejects_a_stalled_tls_prefix() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client_task = tokio::spawn(async move {
        let mut stream = TcpStream::connect(address).await.unwrap();
        stream.write_all(&[0x16]).await.unwrap();
        tokio::time::sleep(Duration::from_millis(400)).await;
    });

    let error = listener
        .accept_shared_mesh_with_timeout(Duration::from_secs(1))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::TimedOut {
            operation: "shared Soulseek/mesh TCP classification"
        }
    ));
    client_task.await.unwrap();
}

#[tokio::test]
async fn demux_rejects_malformed_obfuscated_peer_init_identities() {
    for (username, expected_oversized) in [
        ("   ".to_owned(), false),
        ("x".repeat(MAX_PEER_USERNAME_BYTES + 1), true),
    ] {
        let (mut client, server) = duplex(8192);
        let init = InitMessage::PeerInit {
            username,
            connection_type: "P".to_owned(),
            token: 0,
        };
        write_obfuscated_init_frame(&mut client, &init.encode().unwrap())
            .await
            .unwrap();

        let error = demux_obfuscated_incoming(server).await.unwrap_err();
        assert!(if expected_oversized {
            matches!(error, ClientError::PeerUsernameTooLong { .. })
        } else {
            matches!(error, ClientError::BlankPeerUsername)
        });
    }
}

#[tokio::test]
async fn demuxes_obfuscated_distributed_peer_init_and_preserves_stream() {
    let (mut client, server) = duplex(512);
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "D".to_owned(),
        token: 0,
    };
    write_obfuscated_init_frame(&mut client, &init.encode().unwrap())
        .await
        .unwrap();
    slskr_client::stream::DistributedConnection::new_obfuscated(client)
        .send(&DistributedMessage::Ping)
        .await
        .unwrap();

    let incoming = demux_obfuscated_incoming(server).await.unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        stream,
        obfuscated,
    } = incoming
    else {
        panic!("expected distributed peer init");
    };
    assert_eq!(username, "peer");
    assert_eq!(kind, ConnectionKind::Distributed);
    assert_eq!(token, 0);
    assert!(obfuscated);

    let mut distributed = slskr_client::stream::DistributedConnection::new_obfuscated(stream);
    assert_eq!(
        distributed.receive().await.unwrap(),
        DistributedMessage::Ping
    );
}

#[tokio::test]
async fn demuxes_obfuscated_file_transfer_peer_init_and_preserves_stream() {
    let (mut client, server) = duplex(512);
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "F".to_owned(),
        token: 0,
    };
    write_obfuscated_init_frame(&mut client, &init.encode().unwrap())
        .await
        .unwrap();
    slskr_client::file_transfer::FileTransferConnection::new_obfuscated(client)
        .send_token(123)
        .await
        .unwrap();

    let incoming = demux_obfuscated_incoming(server).await.unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        stream,
        obfuscated,
    } = incoming
    else {
        panic!("expected file-transfer peer init");
    };
    assert_eq!(username, "peer");
    assert_eq!(kind, ConnectionKind::FileTransfer);
    assert_eq!(token, 0);
    assert!(obfuscated);

    let mut file = slskr_client::file_transfer::FileTransferConnection::new_obfuscated(stream);
    assert_eq!(file.receive_token().await.unwrap(), 123);
}

#[tokio::test]
async fn demuxes_slskdn_nested_obfuscated_file_transfer_init() {
    let (mut client, server) = duplex(512);
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "F".to_owned(),
        token: 0,
    };
    let encoded_init = init.encode().unwrap().encode().unwrap();
    let nested = InitFrame::new(encoded_init[0], encoded_init[1..].to_vec());
    write_obfuscated_init_frame(&mut client, &nested)
        .await
        .unwrap();
    slskr_client::file_transfer::FileTransferConnection::new_obfuscated(client)
        .send_token(123)
        .await
        .unwrap();

    let incoming = demux_obfuscated_incoming(server).await.unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        stream,
        obfuscated,
    } = incoming
    else {
        panic!("expected nested file-transfer peer init");
    };
    assert_eq!(username, "peer");
    assert_eq!(kind, ConnectionKind::FileTransfer);
    assert_eq!(token, 0);
    assert!(obfuscated);

    let mut file = slskr_client::file_transfer::FileTransferConnection::new_obfuscated(stream);
    assert_eq!(file.receive_token().await.unwrap(), 123);
}

#[tokio::test]
async fn obfuscated_demux_rejects_plain_peer_init() {
    let (mut client, server) = duplex(512);
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };
    write_init_frame(&mut client, &init.encode().unwrap())
        .await
        .unwrap();
    drop(client);

    assert!(demux_obfuscated_incoming(server).await.is_err());
}

#[tokio::test]
async fn plain_demux_rejects_obfuscated_peer_init() {
    let (mut client, server) = duplex(512);
    let init = InitMessage::PeerInit {
        username: "peer".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };
    write_obfuscated_init_frame_with_key(&mut client, &init.encode().unwrap(), 0x4aee_9414)
        .await
        .unwrap();
    drop(client);

    assert!(demux_incoming(server).await.is_err());
}

#[tokio::test]
async fn demuxes_pierce_firewall() {
    let (mut client, server) = duplex(128);
    let init = InitMessage::PierceFirewall { token: 42 };
    write_init_frame(&mut client, &init.encode().unwrap())
        .await
        .unwrap();

    let incoming = demux_incoming(server).await.unwrap();
    let IncomingConnection::PierceFirewall { token, .. } = incoming else {
        panic!("expected pierce firewall");
    };
    assert_eq!(token, 42);
}

#[tokio::test]
async fn listener_accepts_and_demuxes_tcp_connection() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let client_task = tokio::spawn(async move {
        let mut stream = TcpStream::connect(address).await.unwrap();
        write_connection_kind(&mut stream, ConnectionKind::PeerMessages)
            .await
            .unwrap();
    });

    let (incoming, remote_addr) = listener.accept().await.unwrap();
    assert!(remote_addr.ip().is_loopback());
    assert!(matches!(incoming, IncomingConnection::PeerMessages(_)));
    client_task.await.unwrap();
}

#[tokio::test]
async fn listener_times_out_silent_initialization_handshake() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let _silent_peer = TcpStream::connect(address).await.unwrap();

    let error = listener
        .accept_with_timeout(Duration::from_millis(10))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        slskr_client::ClientError::TimedOut {
            operation: "peer initialization handshake"
        }
    ));
}

#[tokio::test]
async fn listener_timeout_covers_waiting_for_a_connection() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();

    assert!(matches!(
        listener
            .accept_with_timeout(Duration::from_millis(10))
            .await,
        Err(ClientError::TimedOut {
            operation: "peer initialization handshake",
        })
    ));
}

#[tokio::test]
async fn obfuscated_listener_times_out_silent_initialization_handshake() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let _silent_peer = TcpStream::connect(address).await.unwrap();

    let error = listener
        .accept_obfuscated_with_timeout(Duration::from_millis(10))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        slskr_client::ClientError::TimedOut {
            operation: "obfuscated peer initialization handshake"
        }
    ));
}

#[tokio::test]
async fn obfuscated_listener_timeout_covers_waiting_for_a_connection() {
    let listener = Listener::bind("127.0.0.1:0").await.unwrap();

    assert!(matches!(
        listener
            .accept_obfuscated_with_timeout(Duration::from_millis(10))
            .await,
        Err(ClientError::TimedOut {
            operation: "obfuscated peer initialization handshake",
        })
    ));
}

async fn tcp_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (client, accepted) = tokio::join!(TcpStream::connect(address), listener.accept());
    (client.unwrap(), accepted.unwrap().0)
}

#[tokio::test]
async fn shared_demux_rejects_unknown_fallback_after_consuming_alternate_frame() {
    let unknown = InitMessage::Unknown {
        code: 0x42,
        payload: vec![0x55; 6],
    };
    let wire = encode_rotated(&unknown.encode().unwrap().encode().unwrap(), 5);
    // Both interpretations are valid unknown init frames of different lengths.
    let plain = InitFrame::decode(&wire[..9]).unwrap();
    assert!(matches!(
        InitMessage::decode(plain).unwrap(),
        InitMessage::Unknown { .. }
    ));
    let obfuscated = InitFrame::decode(&decode_rotated(&wire).unwrap()).unwrap();
    assert_eq!(InitMessage::decode(obfuscated).unwrap(), unknown);
    let (mut client, server) = tcp_pair().await;
    client.write_all(&wire).await.unwrap();
    let error = demux_shared_incoming(server).await.unwrap_err();
    assert!(matches!(error, ClientError::AmbiguousInitFrame));
}

#[tokio::test]
async fn shared_demux_preserves_unambiguous_unknown_frame_and_following_bytes() {
    let unknown = InitMessage::Unknown {
        code: 0x42,
        payload: vec![0x55; 6],
    };
    let wire = encode_rotated(&unknown.encode().unwrap().encode().unwrap(), 0x8000_0000);
    let (mut client, server) = tcp_pair().await;
    client.write_all(&wire).await.unwrap();
    client.write_all(b"sentinel").await.unwrap();
    let IncomingConnection::UnknownInit {
        code,
        payload,
        mut stream,
    } = demux_shared_incoming(server).await.unwrap()
    else {
        panic!("expected unambiguous unknown init");
    };
    assert_eq!(code, 0x42);
    assert_eq!(payload, vec![0x55; 6]);
    let mut following = [0u8; 8];
    stream.read_exact(&mut following).await.unwrap();
    assert_eq!(&following, b"sentinel");
}

#[tokio::test]
async fn shared_demux_rejects_oversized_firewall_init_before_buffering_body() {
    let length = slskr_client::io::DEFAULT_MAX_FRAME_LEN;
    let mut plain_header = (length as u32).to_le_bytes().to_vec();
    plain_header.extend_from_slice(&[InitCode::PierceFirewall.as_u8(), 0, 0, 0]);
    let obfuscated_header = encode_rotated(&plain_header[..5], 0x8000_0000);
    for header in [plain_header, obfuscated_header] {
        let (mut client, server) = duplex(64);
        client.write_all(&header).await.unwrap();
        // Keep the sender open with no body: rejection must use only the header.
        let error = tokio::time::timeout(Duration::from_secs(1), demux_shared_incoming(server))
            .await
            .expect("reject firewall header without waiting for body")
            .unwrap_err();
        assert!(
            matches!(error, ClientError::FrameTooLarge { length: actual, max: 5 } if actual == length)
        );
    }
}

#[tokio::test]
async fn shared_demux_bounds_unknown_initialization_before_buffering_body() {
    let length = slskr_client::io::DEFAULT_MAX_FRAME_LEN;
    let mut plain_header = (length as u32).to_le_bytes().to_vec();
    plain_header.extend_from_slice(&[0x42, 0, 0, 0]);
    let obfuscated_header = encode_rotated(&plain_header[..5], 0x8000_0000);
    for header in [plain_header, obfuscated_header] {
        let (mut client, server) = duplex(64);
        client.write_all(&header).await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(1), demux_shared_incoming(server))
            .await
            .expect("reject unknown init header without reading body")
            .unwrap_err();
        assert!(
            matches!(error, ClientError::FrameTooLarge { length: actual, max: MAX_PEER_INIT_FRAME_LEN } if actual == length)
        );
    }
}

#[tokio::test]
async fn shared_demux_accepts_unknown_extensions_at_initialization_bound() {
    let unknown = InitMessage::Unknown {
        code: 0x42,
        payload: vec![0x55; MAX_PEER_INIT_FRAME_LEN - 1],
    };
    let plain = unknown.encode().unwrap().encode().unwrap();
    let obfuscated = encode_rotated(&plain, 0x8000_0000);
    for wire in [plain, obfuscated] {
        let (mut client, server) = tcp_pair().await;
        client.write_all(&wire).await.unwrap();
        client.write_all(b"sentinel").await.unwrap();
        let IncomingConnection::UnknownInit {
            code,
            payload,
            mut stream,
        } = demux_shared_incoming(server).await.unwrap()
        else {
            panic!("expected bounded unknown initialization");
        };
        assert_eq!(code, 0x42);
        assert_eq!(payload.len(), MAX_PEER_INIT_FRAME_LEN - 1);
        let mut following = [0; 8];
        stream.read_exact(&mut following).await.unwrap();
        assert_eq!(&following, b"sentinel");
    }
}
