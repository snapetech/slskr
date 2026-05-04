use slskr_client::{
    connection::ConnectionKind,
    file_transfer::FileTransferConnection,
    listener::{demux_incoming, IncomingConnection},
    peer_connect::{send_peer_init, send_pierce_firewall, IndirectPeerRequest},
    stream::{DistributedConnection, PeerMessageConnection},
    ClientError,
};
use slskr_protocol::server::{ConnectToPeerRequest, ServerMessage};
use tokio::io::{duplex, DuplexStream};

#[tokio::test]
async fn send_peer_init_writes_requested_connection_type() {
    let (client, server) = duplex(256);
    let client_task = tokio::spawn(async move {
        send_peer_init(client, "local", ConnectionKind::Distributed)
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
fn indirect_request_accepts_matching_pierce_firewall() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let completed = request.complete(IncomingConnection::PierceFirewall { token: 42, stream });

    assert!(completed.is_ok());
}

#[test]
fn indirect_request_accepts_matching_tagged_connection() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    let completed = request.complete(IncomingConnection::PeerMessages(
        PeerMessageConnection::new(stream),
    ));

    assert!(completed.is_ok());
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
    let incoming: IncomingConnection<DuplexStream> =
        IncomingConnection::PeerMessages(PeerMessageConnection::new(stream));

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
fn indirect_request_accepts_distributed_tagged_connection() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::Distributed);
    let (stream, _) = duplex(64);
    let incoming: IncomingConnection<DuplexStream> =
        IncomingConnection::Distributed(DistributedConnection::new(stream));

    assert!(request.complete(incoming).is_ok());
}

#[test]
fn indirect_request_accepts_file_transfer_tagged_connection() {
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::FileTransfer);
    let (stream, _) = duplex(64);
    let incoming: IncomingConnection<DuplexStream> =
        IncomingConnection::FileTransfer(FileTransferConnection::new(stream));

    assert!(request.complete(incoming).is_ok());
}
