use slskr_client::{peer_cache::PeerConnectionCache, stream::PeerMessageConnection};
use slskr_protocol::peer::PeerMessage;
use tokio::io::duplex;

#[tokio::test]
async fn cache_tracks_insert_replace_remove() {
    let cache = PeerConnectionCache::new();
    let (a, _) = duplex(64);
    let (b, _) = duplex(64);

    assert!(cache.is_empty().await);
    assert!(cache
        .insert("peer", PeerMessageConnection::new(a))
        .await
        .unwrap()
        .is_none());
    assert!(cache.contains("peer").await);
    assert_eq!(cache.len().await, 1);

    let replaced = cache
        .insert("peer", PeerMessageConnection::new(b))
        .await
        .unwrap();
    assert!(replaced.is_some());
    assert_eq!(cache.len().await, 1);

    let removed = cache.remove("peer").await;
    assert!(removed.is_some());
    assert!(!cache.contains("peer").await);
}

#[tokio::test]
async fn cache_sends_to_existing_peer() {
    let cache = PeerConnectionCache::new();
    let (a, b) = duplex(256);
    let mut receiver = PeerMessageConnection::new(b);
    let message = PeerMessage::QueueUpload {
        filename: "Music/file.flac".to_owned(),
    };

    cache
        .insert("peer", PeerMessageConnection::new(a))
        .await
        .unwrap();
    assert!(cache.send_to("peer", &message).await.unwrap());
    assert_eq!(receiver.receive().await.unwrap(), message);
}

#[tokio::test]
async fn cache_reports_missing_peer_on_send() {
    let cache = PeerConnectionCache::<tokio::io::DuplexStream>::new();
    let message = PeerMessage::QueueUpload {
        filename: "Music/file.flac".to_owned(),
    };

    assert!(!cache.send_to("missing", &message).await.unwrap());
}

#[tokio::test]
async fn cache_receives_from_existing_peer() {
    let cache = PeerConnectionCache::new();
    let (a, b) = duplex(256);
    let mut sender = PeerMessageConnection::new(a);
    let message = PeerMessage::QueueUpload {
        filename: "Music/file.flac".to_owned(),
    };

    cache
        .insert("peer", PeerMessageConnection::new(b))
        .await
        .unwrap();
    sender.send(&message).await.unwrap();

    assert_eq!(cache.receive_from("peer").await.unwrap(), Some(message));
}

#[tokio::test]
async fn cache_rejects_new_peers_at_limit_but_allows_replacement() {
    let cache = PeerConnectionCache::with_max_connections(1);
    let (first, _) = duplex(64);
    cache
        .insert("first", PeerMessageConnection::new(first))
        .await
        .unwrap();

    let (replacement, _) = duplex(64);
    assert!(cache
        .insert("first", PeerMessageConnection::new(replacement))
        .await
        .unwrap()
        .is_some());

    let (second, _) = duplex(64);
    let error = cache
        .insert("second", PeerMessageConnection::new(second))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        slskr_client::ClientError::PeerConnectionCacheFull { max: 1 }
    ));
    assert_eq!(cache.len().await, 1);
    assert!(cache.contains("first").await);
}

#[tokio::test]
async fn cache_treats_username_casing_as_one_peer() {
    let cache = PeerConnectionCache::with_max_connections(1);
    let (first, _) = duplex(64);
    cache
        .insert("Alice", PeerMessageConnection::new(first))
        .await
        .unwrap();
    assert!(cache.contains("ALICE").await);

    let (replacement, wire) = duplex(256);
    assert!(cache
        .insert("alice", PeerMessageConnection::new(replacement))
        .await
        .unwrap()
        .is_some());
    assert_eq!(cache.len().await, 1);

    let mut receiver = PeerMessageConnection::new(wire);
    let message = PeerMessage::QueueUpload {
        filename: "Music/file.flac".to_owned(),
    };
    assert!(cache.send_to("aLiCe", &message).await.unwrap());
    assert_eq!(receiver.receive().await.unwrap(), message);
    assert!(cache.remove("ALICE").await.is_some());
    assert!(cache.is_empty().await);
}
