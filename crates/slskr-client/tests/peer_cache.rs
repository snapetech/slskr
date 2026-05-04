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
        .is_none());
    assert!(cache.contains("peer").await);
    assert_eq!(cache.len().await, 1);

    let replaced = cache.insert("peer", PeerMessageConnection::new(b)).await;
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

    cache.insert("peer", PeerMessageConnection::new(a)).await;
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

    cache.insert("peer", PeerMessageConnection::new(b)).await;
    sender.send(&message).await.unwrap();

    assert_eq!(cache.receive_from("peer").await.unwrap(), Some(message));
}
