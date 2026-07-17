use slskr_client::{
    connection::ConnectionKind,
    io::{
        write_connection_kind, write_init_frame, write_obfuscated_init_frame,
        write_obfuscated_init_frame_with_key,
    },
    listener::{demux_incoming, demux_obfuscated_incoming, IncomingConnection, Listener},
    peer_cache::MAX_PEER_USERNAME_BYTES,
    ClientError,
};
use slskr_protocol::{distributed::DistributedMessage, init::InitMessage, peer::PeerMessage};
use tokio::io::duplex;
use tokio::net::TcpStream;
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
    } = incoming
    else {
        panic!("expected peer init");
    };
    assert_eq!(username, "peer");
    assert_eq!(kind, ConnectionKind::PeerMessages);
    assert_eq!(token, 0);

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
    slskr_client::stream::DistributedConnection::new(client)
        .send(&DistributedMessage::Ping)
        .await
        .unwrap();

    let incoming = demux_obfuscated_incoming(server).await.unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        stream,
    } = incoming
    else {
        panic!("expected distributed peer init");
    };
    assert_eq!(username, "peer");
    assert_eq!(kind, ConnectionKind::Distributed);
    assert_eq!(token, 0);

    let mut distributed = slskr_client::stream::DistributedConnection::new(stream);
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
    slskr_client::file_transfer::FileTransferConnection::new(client)
        .send_token(123)
        .await
        .unwrap();

    let incoming = demux_obfuscated_incoming(server).await.unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        stream,
    } = incoming
    else {
        panic!("expected file-transfer peer init");
    };
    assert_eq!(username, "peer");
    assert_eq!(kind, ConnectionKind::FileTransfer);
    assert_eq!(token, 0);

    let mut file = slskr_client::file_transfer::FileTransferConnection::new(stream);
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
