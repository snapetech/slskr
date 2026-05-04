use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use slskr_client::{
    connection::ConnectionKind,
    file_transfer::FileTransferConnection,
    listener::IncomingConnection,
    manager::{ConnectionManager, PeerConnector, TokenGenerator},
    peer_cache::PeerConnectionCache,
    peer_connect::IndirectPeerRequest,
    server::ServerSession,
    stream::{DistributedConnection, PeerMessageConnection, ServerConnection},
};
use slskr_protocol::server::Direction;
use tokio::io::{duplex, DuplexStream};

#[test]
fn token_generator_wraps() {
    let mut tokens = TokenGenerator::new(u32::MAX);

    assert_eq!(tokens.next_token(), u32::MAX);
    assert_eq!(tokens.next_token(), 0);
}

#[tokio::test]
async fn ensure_peer_messages_reuses_cached_connection() {
    let manager = manager_with_connector(|_| {
        let (stream, _) = duplex(64);
        PeerMessageConnection::new(stream)
    });

    assert!(manager.ensure_peer_messages("peer").await.unwrap());
    assert!(!manager.ensure_peer_messages("peer").await.unwrap());
    assert!(manager.peer_cache().contains("peer").await);
}

#[tokio::test]
async fn ensure_peer_messages_calls_connector_once_for_cached_peer() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_connector = Arc::clone(&calls);
    let manager = manager_with_connector(move |_| {
        calls_for_connector.fetch_add(1, Ordering::SeqCst);
        let (stream, _) = duplex(64);
        PeerMessageConnection::new(stream)
    });

    manager.ensure_peer_messages("peer").await.unwrap();
    manager.ensure_peer_messages("peer").await.unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn request_indirect_peer_messages_sends_server_connect_message() {
    let (client, server) = duplex(512);
    let manager = ConnectionManager::new(
        ServerSession::new(ServerConnection::new(client)),
        PeerConnectionCache::new(),
        connector(|_| {
            let (stream, _) = duplex(64);
            PeerMessageConnection::new(stream)
        }),
    )
    .with_token_seed(700);
    let mut server = ServerConnection::new(server);

    let request = manager
        .request_indirect_peer_messages("peer")
        .await
        .unwrap();
    assert_eq!(
        request,
        IndirectPeerRequest::new(700, "peer", ConnectionKind::PeerMessages)
    );
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        request.server_message()
    );
}

#[tokio::test]
async fn request_indirect_supports_all_connection_kinds() {
    let (client, server) = duplex(1024);
    let manager = ConnectionManager::new(
        ServerSession::new(ServerConnection::new(client)),
        PeerConnectionCache::new(),
        connector(|_| {
            let (stream, _) = duplex(64);
            PeerMessageConnection::new(stream)
        }),
    )
    .with_token_seed(900);
    let mut server = ServerConnection::new(server);

    let distributed = manager.request_indirect_distributed("peer").await.unwrap();
    let file = manager
        .request_indirect_file_transfer("peer")
        .await
        .unwrap();

    assert_eq!(
        distributed,
        IndirectPeerRequest::new(900, "peer", ConnectionKind::Distributed)
    );
    assert_eq!(
        file,
        IndirectPeerRequest::new(901, "peer", ConnectionKind::FileTransfer)
    );
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        distributed.server_message()
    );
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        file.server_message()
    );
}

#[tokio::test]
async fn complete_inbound_peer_messages_inserts_cache_entry() {
    let manager = manager_with_connector(|_| {
        let (stream, _) = duplex(64);
        PeerMessageConnection::new(stream)
    });
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::PeerMessages);
    let (stream, _) = duplex(64);

    manager
        .complete_inbound_peer_messages(
            &request,
            IncomingConnection::PeerInit {
                username: "peer".to_owned(),
                kind: ConnectionKind::PeerMessages,
                token: 42,
                stream,
            },
        )
        .await
        .unwrap();

    assert!(manager.peer_cache().contains("peer").await);
}

#[test]
fn complete_inbound_distributed_returns_typed_connection() {
    let manager = manager_with_connector(|_| {
        let (stream, _) = duplex(64);
        PeerMessageConnection::new(stream)
    });
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::Distributed);
    let (stream, _) = duplex(64);

    let connection = manager
        .complete_inbound_distributed(
            &request,
            IncomingConnection::Distributed(DistributedConnection::new(stream)),
        )
        .unwrap();

    let _stream = connection.into_inner();
}

#[test]
fn complete_inbound_file_transfer_returns_typed_connection() {
    let manager = manager_with_connector(|_| {
        let (stream, _) = duplex(64);
        PeerMessageConnection::new(stream)
    });
    let request = IndirectPeerRequest::new(42, "peer", ConnectionKind::FileTransfer);
    let (stream, _) = duplex(64);

    let connection = manager
        .complete_inbound_file_transfer(
            &request,
            IncomingConnection::FileTransfer(FileTransferConnection::new(stream)),
        )
        .unwrap();

    let _stream = connection.into_inner();
}

fn manager_with_connector<F>(factory: F) -> ConnectionManager<DuplexStream, DuplexStream>
where
    F: Fn(String) -> PeerMessageConnection<DuplexStream> + Send + Sync + 'static,
{
    let (client, _) = duplex(512);
    ConnectionManager::new(
        ServerSession::new(ServerConnection::new(client)),
        PeerConnectionCache::new(),
        connector(factory),
    )
}

fn connector<F>(factory: F) -> PeerConnector<DuplexStream>
where
    F: Fn(String) -> PeerMessageConnection<DuplexStream> + Send + Sync + 'static,
{
    Arc::new(move |username| {
        let connection = factory(username);
        Box::pin(async move { Ok(connection) })
    })
}
