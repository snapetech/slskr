use slskr_client::stream::{
    DistributedConnection, InitConnection, ObfuscatedInitConnection,
    ObfuscatedPeerMessageConnection, PeerMessageConnection, ServerConnection,
};
use slskr_protocol::{
    distributed::{DistributedMessage, DistributedSearch},
    init::InitMessage,
    peer::PeerMessage,
    server::{Direction, LoginRequest, ServerMessage},
};
use tokio::{
    io::{duplex, AsyncWriteExt},
    time::{timeout, Duration},
};

#[tokio::test]
async fn server_connection_sends_typed_messages() {
    let (client, server) = duplex(256);
    let mut client = ServerConnection::new(client);
    let mut server = ServerConnection::new(server);
    let message = ServerMessage::LoginRequest(LoginRequest {
        username: "user".to_owned(),
        password: "pass".to_owned(),
        major_version: 175,
        hash: "hash".to_owned(),
        minor_version: 1,
    });

    client.send(&message).await.unwrap();
    let received = server
        .receive_with_direction(Direction::ClientToServer)
        .await
        .unwrap();
    assert_eq!(received, message);
}

#[tokio::test]
async fn server_connection_sends_batches_without_combining_frames() {
    let (client, server) = duplex(256);
    let mut client = ServerConnection::new(client);
    let mut server = ServerConnection::new(server);
    let messages = [ServerMessage::ServerPing, ServerMessage::ServerPing];

    client.send_batch(&messages).await.unwrap();

    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::ServerPing
    );
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::ServerPing
    );
}

#[tokio::test]
async fn server_connection_preserves_partial_frame_after_receive_timeout() {
    let (client, mut server) = duplex(256);
    let mut client = ServerConnection::new(client);
    let encoded = ServerMessage::ServerPing
        .encode()
        .unwrap()
        .encode()
        .unwrap();

    server.write_all(&encoded[..2]).await.unwrap();
    assert!(timeout(Duration::from_millis(20), client.receive())
        .await
        .is_err());

    server.write_all(&encoded[2..]).await.unwrap();
    assert_eq!(client.receive().await.unwrap(), ServerMessage::ServerPing);
}

#[tokio::test]
async fn peer_connection_round_trips_typed_messages() {
    let (a, b) = duplex(256);
    let mut a = PeerMessageConnection::new(a);
    let mut b = PeerMessageConnection::new(b);
    let message = PeerMessage::QueueUpload {
        filename: "Music/file.flac".to_owned(),
    };

    a.send(&message).await.unwrap();
    assert_eq!(b.receive().await.unwrap(), message);
}

#[tokio::test]
async fn obfuscated_peer_connection_round_trips_typed_messages() {
    let (a, b) = duplex(256);
    let mut a = ObfuscatedPeerMessageConnection::new(a);
    let mut b = ObfuscatedPeerMessageConnection::new(b);
    let message = PeerMessage::UserInfoRequest;

    a.send(&message).await.unwrap();
    assert_eq!(b.receive().await.unwrap(), message);
}

#[tokio::test]
async fn distributed_connection_round_trips_typed_messages() {
    let (a, b) = duplex(256);
    let mut a = DistributedConnection::new(a);
    let mut b = DistributedConnection::new(b);
    let message = DistributedMessage::Search(DistributedSearch {
        identifier: 49,
        username: "peer".to_owned(),
        token: 7,
        query: "query".to_owned(),
    });

    a.send(&message).await.unwrap();
    assert_eq!(b.receive().await.unwrap(), message);
}

#[tokio::test]
async fn init_connection_round_trips_typed_messages() {
    let (a, b) = duplex(256);
    let mut a = InitConnection::new(a);
    let mut b = InitConnection::new(b);
    let message = InitMessage::PeerInit {
        username: "user".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };

    a.send(&message).await.unwrap();
    assert_eq!(b.receive().await.unwrap(), message);
}

#[tokio::test]
async fn obfuscated_init_connection_round_trips_typed_messages() {
    let (a, b) = duplex(256);
    let mut a = ObfuscatedInitConnection::new(a);
    let mut b = ObfuscatedInitConnection::new(b);
    let message = InitMessage::PeerInit {
        username: "user".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };

    a.send(&message).await.unwrap();
    assert_eq!(b.receive().await.unwrap(), message);
}
