use slskr_client::{file_transfer::FileTransferConnection, ClientError};
use tokio::io::duplex;

#[tokio::test]
async fn token_round_trips() {
    let (a, b) = duplex(16);
    let mut a = FileTransferConnection::new(a);
    let mut b = FileTransferConnection::new(b);

    a.send_token(42).await.unwrap();
    assert_eq!(b.receive_token().await.unwrap(), 42);
}

#[tokio::test]
async fn offset_round_trips() {
    let (a, b) = duplex(16);
    let mut a = FileTransferConnection::new(a);
    let mut b = FileTransferConnection::new(b);

    a.send_offset(1_234_567_890).await.unwrap();
    assert_eq!(b.receive_offset().await.unwrap(), 1_234_567_890);
}

#[tokio::test]
async fn chunks_round_trip() {
    let (a, b) = duplex(16);
    let mut a = FileTransferConnection::new(a);
    let mut b = FileTransferConnection::new(b);

    a.write_chunk(&[1, 2, 3, 4]).await.unwrap();
    assert_eq!(b.read_chunk(4).await.unwrap(), vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn oversized_chunk_is_rejected_before_allocation() {
    let (_a, b) = duplex(8);
    let mut b = FileTransferConnection::new(b);

    let error = b.read_chunk_with_max(9, 8).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::FrameTooLarge { length: 9, max: 8 }
    ));
}

#[tokio::test]
async fn token_offset_then_chunk_preserve_order() {
    let (a, b) = duplex(64);
    let mut a = FileTransferConnection::new(a);
    let mut b = FileTransferConnection::new(b);

    a.send_token(99).await.unwrap();
    a.send_offset(10).await.unwrap();
    a.write_chunk(&[7, 8, 9]).await.unwrap();

    assert_eq!(b.receive_token().await.unwrap(), 99);
    assert_eq!(b.receive_offset().await.unwrap(), 10);
    assert_eq!(b.read_chunk(3).await.unwrap(), vec![7, 8, 9]);
}
