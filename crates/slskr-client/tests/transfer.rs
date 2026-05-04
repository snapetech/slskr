use slskr_client::{
    file_transfer::FileTransferConnection,
    transfer::{DownloadState, DownloadTransfer, UploadState, UploadTransfer},
    ClientError,
};
use slskr_protocol::peer::{PeerMessage, TransferRequest, TransferResponse};
use tokio::io::duplex;

#[test]
fn queue_upload_message_marks_transfer_queued() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    assert_eq!(
        transfer.queue_upload_message(),
        PeerMessage::QueueUpload {
            filename: "Music/file.flac".to_owned(),
        }
    );
    assert_eq!(transfer.state, DownloadState::Queued);
}

#[test]
fn place_in_queue_response_updates_state() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    transfer
        .handle_peer_message(PeerMessage::PlaceInQueueResponse {
            filename: "Music/file.flac".to_owned(),
            place: 3,
        })
        .unwrap();

    assert_eq!(transfer.state, DownloadState::PlaceInQueue(3));
}

#[test]
fn transfer_request_updates_state() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    transfer
        .handle_peer_message(PeerMessage::TransferRequest(TransferRequest {
            direction: 1,
            token: 7,
            filename: "Music/file.flac".to_owned(),
            size: Some(100),
        }))
        .unwrap();

    assert_eq!(transfer.state, DownloadState::Requested { size: Some(100) });
}

#[test]
fn transfer_response_updates_state() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(100),
        }))
        .unwrap();

    assert_eq!(transfer.state, DownloadState::Accepted { size: Some(100) });
}

#[test]
fn reject_response_message_updates_state_and_payload() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    assert_eq!(
        transfer.reject_upload_response_message("Queued"),
        PeerMessage::TransferResponse(TransferResponse::Rejected {
            token: 7,
            reason: "Queued".to_owned(),
        })
    );
    assert_eq!(
        transfer.state,
        DownloadState::Rejected {
            reason: "Queued".to_owned()
        }
    );
}

#[test]
fn wrong_token_is_rejected() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    let error = transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 8,
            size: Some(100),
        }))
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransferTokenMismatch {
            expected: 7,
            received: 8
        }
    ));
}

#[test]
fn wrong_filename_is_rejected() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    let error = transfer
        .handle_peer_message(PeerMessage::PlaceInQueueResponse {
            filename: "other.flac".to_owned(),
            place: 1,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransferFilenameMismatch { expected, received }
            if expected == "Music/file.flac" && received == "other.flac"
    ));
}

#[tokio::test]
async fn receive_file_validates_token_sends_offset_and_reads_bytes() {
    let (uploader, downloader) = duplex(64);
    let mut uploader = FileTransferConnection::new(uploader);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    let uploader_task = tokio::spawn(async move {
        uploader.send_token(7).await.unwrap();
        assert_eq!(uploader.receive_offset().await.unwrap(), 5);
        uploader.write_chunk(&[1, 2, 3]).await.unwrap();
    });

    let bytes = transfer
        .receive_file_from(&mut downloader, 5, 3)
        .await
        .unwrap();

    assert_eq!(bytes, vec![1, 2, 3]);
    assert_eq!(transfer.state, DownloadState::Completed);
    uploader_task.await.unwrap();
}

#[test]
fn upload_transfer_request_marks_requested() {
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 100);

    assert_eq!(
        transfer.transfer_request_message(),
        PeerMessage::TransferRequest(TransferRequest {
            direction: 1,
            token: 7,
            filename: "Music/file.flac".to_owned(),
            size: Some(100),
        })
    );
    assert_eq!(transfer.state, UploadState::Requested);
}

#[test]
fn upload_transfer_response_updates_state() {
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 100);

    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();
    assert_eq!(transfer.state, UploadState::Accepted);

    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Rejected {
            token: 7,
            reason: "Queued".to_owned(),
        }))
        .unwrap();
    assert_eq!(
        transfer.state,
        UploadState::Rejected {
            reason: "Queued".to_owned()
        }
    );
}

#[tokio::test]
async fn send_file_uses_requested_offset_and_marks_completed() {
    let (uploader, downloader) = duplex(64);
    let mut uploader = FileTransferConnection::new(uploader);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 5);

    let downloader_task = tokio::spawn(async move {
        assert_eq!(downloader.receive_token().await.unwrap(), 7);
        downloader.send_offset(2).await.unwrap();
        downloader.read_chunk(3).await.unwrap()
    });

    let offset = transfer
        .send_file_to(&mut uploader, &[1, 2, 3, 4, 5])
        .await
        .unwrap();

    assert_eq!(offset, 2);
    assert_eq!(transfer.state, UploadState::Completed);
    assert_eq!(downloader_task.await.unwrap(), vec![3, 4, 5]);
}

#[tokio::test]
async fn send_file_rejects_offset_past_end() {
    let (uploader, downloader) = duplex(64);
    let mut uploader = FileTransferConnection::new(uploader);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 5);

    let downloader_task = tokio::spawn(async move {
        assert_eq!(downloader.receive_token().await.unwrap(), 7);
        downloader.send_offset(9).await.unwrap();
    });

    let error = transfer
        .send_file_to(&mut uploader, &[1, 2, 3, 4, 5])
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransferOffsetOutOfRange { offset: 9, size: 5 }
    ));
    downloader_task.await.unwrap();
}
