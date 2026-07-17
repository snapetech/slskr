use slskr_client::{
    connection::ConnectionKind,
    file_transfer::FileTransferConnection,
    listener::{demux_incoming, IncomingConnection},
    peer_cache::MAX_PEER_USERNAME_BYTES,
    peer_connect::{
        connect_peer_messages_with_timeout, send_obfuscated_peer_init, send_peer_init,
        send_pierce_firewall, IndirectPeerRequest,
    },
    stream::{DistributedConnection, PeerMessageConnection},
    ClientError,
};
use slskr_protocol::server::{ConnectToPeerRequest, ServerMessage};
use tokio::{
    io::{duplex, AsyncReadExt, DuplexStream},
    net::TcpListener,
    time::{timeout, Duration},
};

#[tokio::test]
async fn send_peer_init_writes_requested_connection_type() {
    let (client, server) = duplex(256);
    let client_task = tokio::spawn(async move {
        send_peer_init(client, " local ", ConnectionKind::Distributed)
            .await
            .unwrap()
    });

    let incoming = demux_incoming(server).await.unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        ..
    } = incoming
    else {
        panic!("expected peer init");
    };
    assert_eq!(username, "local");
    assert_eq!(kind, ConnectionKind::Distributed);
    assert_eq!(token, 0);

    client_task.await.unwrap();
}

#[tokio::test]
async fn blank_peer_init_is_rejected_before_regular_or_obfuscated_io() {
    let (regular_client, mut regular_server) = duplex(256);
    assert!(matches!(
        send_peer_init(regular_client, "  ", ConnectionKind::PeerMessages)
            .await
            .unwrap_err(),
        ClientError::BlankPeerUsername
    ));
    let mut byte = [0];
    assert_eq!(regular_server.read(&mut byte).await.unwrap(), 0);

    let (obfuscated_client, mut obfuscated_server) = duplex(256);
    assert!(matches!(
        send_obfuscated_peer_init(obfuscated_client, "  ", ConnectionKind::PeerMessages)
            .await
            .unwrap_err(),
        ClientError::BlankPeerUsername
    ));
    assert_eq!(obfuscated_server.read(&mut byte).await.unwrap(), 0);
}

#[tokio::test]
async fn oversized_peer_init_is_rejected_before_regular_or_obfuscated_io() {
    let username = "x".repeat(MAX_PEER_USERNAME_BYTES + 1);
    let (regular_client, mut regular_server) = duplex(256);
    assert!(matches!(
        send_peer_init(regular_client, &username, ConnectionKind::PeerMessages)
            .await
            .unwrap_err(),
        ClientError::PeerUsernameTooLong { length, max }
            if length == MAX_PEER_USERNAME_BYTES + 1 && max == MAX_PEER_USERNAME_BYTES
    ));
    let mut byte = [0];
    assert_eq!(regular_server.read(&mut byte).await.unwrap(), 0);

    let (obfuscated_client, mut obfuscated_server) = duplex(256);
    assert!(matches!(
        send_obfuscated_peer_init(obfuscated_client, &username, ConnectionKind::PeerMessages)
            .await
            .unwrap_err(),
        ClientError::PeerUsernameTooLong { length, max }
            if length == MAX_PEER_USERNAME_BYTES + 1 && max == MAX_PEER_USERNAME_BYTES
    ));
    assert_eq!(obfuscated_server.read(&mut byte).await.unwrap(), 0);
}

#[tokio::test]
async fn blank_peer_identity_is_rejected_before_tcp_connect() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let error = connect_peer_messages_with_timeout(
        listener.local_addr().unwrap(),
        "  ",
        Duration::from_secs(1),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, ClientError::BlankPeerUsername));
    assert!(timeout(Duration::from_millis(25), listener.accept())
        .await
        .is_err());
}

#[tokio::test]
async fn send_pierce_firewall_writes_token() {
    let (client, server) = duplex(256);
    let client_task = tokio::spawn(async move { send_pierce_firewall(client, 99).await.unwrap() });

    let incoming = demux_incoming(server).await.unwrap();
    let IncomingConnection::PierceFirewall { token, .. } = incoming else {
        panic!("expected pierce firewall");
    };
    assert_eq!(token, 99);

    client_task.await.unwrap();
}

#[test]
fn indirect_request_builds_server_connect_to_peer_message() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);

    assert_eq!(
        request.server_message(),
        ServerMessage::ConnectToPeerRequest(ConnectToPeerRequest {
            token: 42,
            username: "peer".to_owned(),
            connection_type: "P".to_owned(),
        })
    );
}

#[test]
fn indirect_request_accepts_matching_peer_init() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let completed = request.complete(IncomingConnection::PeerInit {
        username: "peer".to_owned(),
        kind: ConnectionKind::PeerMessages,
        token: 42,
        stream,
    });

    assert!(completed.is_ok());
}

#[test]
fn indirect_request_accepts_matching_peer_init_with_different_username_casing() {
    let request = IndirectPeerRequest::new(42, "Alice", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let completed = request.complete(IncomingConnection::PeerInit {
        username: "ALICE".to_owned(),
        kind: ConnectionKind::PeerMessages,
        token: 42,
        stream,
    });

    assert!(completed.is_ok());
}

#[test]
fn indirect_request_accepts_matching_pierce_firewall() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let completed = request.complete(IncomingConnection::PierceFirewall { token: 42, stream });

    assert!(completed.is_ok());
}

#[test]
fn indirect_request_rejects_untokened_tagged_connection() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let error = request
        .complete(IncomingConnection::PeerMessages(
            PeerMessageConnection::new(stream),
        ))
        .unwrap_err();

    assert!(matches!(error, ClientError::IndirectInitRequired));
}

#[test]
fn indirect_request_rejects_wrong_token() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let error = request
        .complete(IncomingConnection::PierceFirewall { token: 99, stream })
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::IndirectTokenMismatch {
            expected: 42,
            received: 99
        }
    ));
}

#[test]
fn indirect_request_rejects_wrong_username() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let error = request
        .complete(IncomingConnection::PeerInit {
            username: "other".to_owned(),
            kind: ConnectionKind::PeerMessages,
            token: 42,
            stream,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::IndirectUsernameMismatch { expected, received }
            if expected == "peer" && received == "other"
    ));
}

#[test]
fn indirect_request_rejects_wrong_connection_kind() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::Distributed);
    let (stream, _) = duplex(64);
    let incoming: IncomingConnection<DuplexStream> = IncomingConnection::PeerInit {
        username: "peer".to_owned(),
        kind: ConnectionKind::PeerMessages,
        token: 42,
        stream,
    };

    let error = request.complete(incoming).unwrap_err();

    assert!(matches!(
        error,
        ClientError::IndirectKindMismatch {
            expected: ConnectionKind::Distributed,
            received: ConnectionKind::PeerMessages
        }
    ));
}

#[test]
fn indirect_request_rejects_distributed_tagged_connection_without_token() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::Distributed);
    let (stream, _) = duplex(64);
    let incoming: IncomingConnection<DuplexStream> =
        IncomingConnection::Distributed(DistributedConnection::new(stream));

    assert!(matches!(
        request.complete(incoming).unwrap_err(),
        ClientError::IndirectInitRequired
    ));
}

#[test]
fn indirect_request_rejects_file_transfer_tagged_connection_without_token() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::FileTransfer);
    let (stream, _) = duplex(64);
    let incoming: IncomingConnection<DuplexStream> =
        IncomingConnection::FileTransfer(FileTransferConnection::new(stream));

    assert!(matches!(
        request.complete(incoming).unwrap_err(),
        ClientError::IndirectInitRequired
    ));
}
