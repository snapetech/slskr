use std::time::Duration;

use slskr_client::{
    file_transfer::FileTransferConnection,
    transfer::{
        DownloadState, DownloadTransfer, UploadState, UploadTransfer, MAX_TRANSFER_REASON_BYTES,
    },
    ClientError,
};
use slskr_protocol::peer::{PeerMessage, TransferRequest, TransferResponse};
use slskr_protocol::ProtocolTextEncoding;
use tokio::io::duplex;

#[test]
fn queue_upload_message_marks_transfer_queued() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    assert_eq!(
        transfer.queue_upload_message().unwrap(),
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
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: Some(100),
        }))
        .unwrap();

    assert_eq!(transfer.state, DownloadState::Requested { size: Some(100) });
}

#[test]
fn download_rejects_wrong_transfer_direction_without_changing_state() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    let error = transfer
        .handle_peer_message(PeerMessage::TransferRequest(TransferRequest {
            direction: 0,
            token: 7,
            filename: "Music/file.flac".to_owned(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: Some(100),
        }))
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransferDirectionMismatch {
            expected: 1,
            received: 0
        }
    ));
    assert_eq!(transfer.state, DownloadState::New);
}

#[test]
fn transfer_response_updates_state() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, Some(100));

    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(100),
        }))
        .unwrap();

    assert_eq!(transfer.state, DownloadState::Accepted { size: Some(100) });
}

#[test]
fn download_preserves_advertised_size_and_rejects_conflicts() {
    let mut unspecified = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut unspecified, Some(100));
    unspecified
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();
    assert_eq!(
        unspecified.state,
        DownloadState::Accepted { size: Some(100) }
    );

    let mut conflicting = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut conflicting, Some(100));
    assert!(matches!(
        conflicting.handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(99),
        },)),
        Err(ClientError::TransferSizeMismatch {
            expected: 100,
            actual: 99,
        })
    ));
    assert_eq!(
        conflicting.state,
        DownloadState::Requested { size: Some(100) }
    );

    assert!(matches!(
        conflicting.accept_upload_response_message(99),
        Err(ClientError::TransferSizeMismatch {
            expected: 100,
            actual: 99,
        })
    ));
    assert_eq!(
        conflicting.state,
        DownloadState::Requested { size: Some(100) }
    );
}

#[test]
fn reject_response_message_updates_state_and_payload() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, Some(100));

    assert_eq!(
        transfer.reject_upload_response_message("Queued").unwrap(),
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
fn transfer_rejection_reasons_are_bounded_and_control_safe() {
    for reason in [
        "forged\r\nlog entry".to_owned(),
        "x".repeat(MAX_TRANSFER_REASON_BYTES + 1),
    ] {
        let mut download = DownloadTransfer::new("peer", "Music/file.flac", 7);
        request_download(&mut download, Some(100));
        assert!(matches!(
            download.handle_peer_message(PeerMessage::TransferResponse(
                TransferResponse::Rejected {
                    token: 7,
                    reason: reason.clone(),
                },
            )),
            Err(ClientError::InvalidTransferReason {
                max: MAX_TRANSFER_REASON_BYTES,
            })
        ));
        assert_eq!(download.state, DownloadState::Requested { size: Some(100) });
        assert!(matches!(
            download.reject_upload_response_message(reason.clone()),
            Err(ClientError::InvalidTransferReason {
                max: MAX_TRANSFER_REASON_BYTES,
            })
        ));

        let mut upload = UploadTransfer::new("peer", "Music/file.flac", 7, 100);
        upload.transfer_request_message().unwrap();
        assert!(matches!(
            upload.handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Rejected {
                token: 7,
                reason
            },)),
            Err(ClientError::InvalidTransferReason {
                max: MAX_TRANSFER_REASON_BYTES,
            })
        ));
        assert_eq!(upload.state, UploadState::Requested);
    }
}

#[test]
fn wrong_token_is_rejected() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, Some(100));

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
    request_download(&mut transfer, Some(8));
    transfer.accept_upload_response_message(8).unwrap();

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

#[tokio::test]
async fn receive_file_timeout_marks_download_failed() {
    let (_silent_uploader, downloader) = duplex(64);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, Some(1));
    transfer.accept_upload_response_message(1).unwrap();

    assert!(matches!(
        transfer
            .receive_file_from_with_timeout(&mut downloader, 0, 1, Duration::from_millis(10),)
            .await,
        Err(ClientError::TimedOut {
            operation: "download transfer I/O",
        })
    ));
    assert_eq!(
        transfer.state,
        DownloadState::Failed {
            reason: "transfer I/O timed out".to_owned(),
        }
    );
}

#[tokio::test]
async fn receive_file_rejects_oversized_remaining_before_io() {
    let (mut uploader, downloader) = duplex(64);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, None);
    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();

    let error = tokio::time::timeout(
        Duration::from_millis(10),
        transfer.receive_file_from(&mut downloader, 5, usize::MAX),
    )
    .await
    .expect("local validation must not wait for the peer")
    .unwrap_err();

    assert!(matches!(
        error,
        ClientError::FrameTooLarge {
            length: usize::MAX,
            ..
        }
    ));
    use tokio::io::AsyncReadExt as _;
    let mut byte = [0];
    assert!(
        tokio::time::timeout(Duration::from_millis(10), uploader.read(&mut byte))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn receive_file_rejects_range_that_does_not_complete_negotiated_size() {
    let (downloader, _uploader) = duplex(64);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, Some(5));
    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(5),
        }))
        .unwrap();

    let error = transfer
        .receive_file_from(&mut downloader, 2, 2)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransferSizeMismatch {
            expected: 5,
            actual: 4
        }
    ));
    assert_eq!(transfer.state, DownloadState::Accepted { size: Some(5) });
}

#[tokio::test]
async fn receive_file_rejects_overflowing_range_before_io() {
    let (downloader, _uploader) = duplex(64);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, Some(u64::MAX));
    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(u64::MAX),
        }))
        .unwrap();

    let error = transfer
        .receive_file_from(&mut downloader, u64::MAX - 1, 2)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransferOffsetOutOfRange {
            offset,
            size: u64::MAX,
        } if offset == u64::MAX - 1
    ));
    assert_eq!(
        transfer.state,
        DownloadState::Accepted {
            size: Some(u64::MAX)
        }
    );
}

#[test]
fn upload_transfer_request_marks_requested() {
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 100);

    assert_eq!(
        transfer.transfer_request_message().unwrap(),
        PeerMessage::TransferRequest(TransferRequest {
            direction: 1,
            token: 7,
            filename: "Music/file.flac".to_owned(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: Some(100),
        })
    );
    assert_eq!(transfer.state, UploadState::Requested);
}

#[test]
fn upload_transfer_response_updates_state() {
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 100);
    transfer.transfer_request_message().unwrap();

    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();
    assert_eq!(transfer.state, UploadState::Accepted);

    let mut rejected = UploadTransfer::new("peer", "Music/file.flac", 7, 100);
    rejected.transfer_request_message().unwrap();
    rejected
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Rejected {
            token: 7,
            reason: "Queued".to_owned(),
        }))
        .unwrap();
    assert_eq!(
        rejected.state,
        UploadState::Rejected {
            reason: "Queued".to_owned()
        }
    );
}

#[test]
fn upload_rejects_conflicting_acknowledgement_size() {
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 100);
    transfer.transfer_request_message().unwrap();

    assert!(matches!(
        transfer.handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(99),
        })),
        Err(ClientError::TransferSizeMismatch {
            expected: 100,
            actual: 99,
        })
    ));
    assert_eq!(transfer.state, UploadState::Requested);

    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(100),
        }))
        .unwrap();
    assert_eq!(transfer.state, UploadState::Accepted);
}

#[test]
fn upload_rejects_unsolicited_control_messages_before_request() {
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 5);

    assert!(matches!(
        transfer.handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        })),
        Err(ClientError::InvalidTransferState {
            operation: "handle transfer response",
            state: "new",
        })
    ));
    assert_eq!(transfer.state, UploadState::New);

    assert!(matches!(
        transfer.handle_peer_message(PeerMessage::UploadFailed {
            filename: "Music/file.flac".to_owned(),
        }),
        Err(ClientError::InvalidTransferState {
            operation: "handle upload failure",
            state: "new",
        })
    ));
    assert_eq!(transfer.state, UploadState::New);
}

#[test]
fn download_rejects_unsolicited_control_messages_before_request() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);

    assert!(matches!(
        transfer.handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: Some(5),
        })),
        Err(ClientError::InvalidTransferState {
            operation: "handle transfer response",
            state: "new",
        })
    ));
    assert_eq!(transfer.state, DownloadState::New);

    for (message, operation) in [
        (
            PeerMessage::UploadFailed {
                filename: "Music/file.flac".to_owned(),
            },
            "handle upload failure",
        ),
        (
            PeerMessage::UploadDenied {
                filename: "Music/file.flac".to_owned(),
                reason: "denied".to_owned(),
            },
            "handle upload denial",
        ),
    ] {
        assert!(matches!(
            transfer.handle_peer_message(message),
            Err(ClientError::InvalidTransferState { operation: actual, state: "new" })
                if actual == operation
        ));
        assert_eq!(transfer.state, DownloadState::New);
    }
}

#[test]
fn terminal_transfers_reject_replayed_peer_control_messages() {
    let response = PeerMessage::TransferResponse(TransferResponse::Allowed {
        token: 7,
        size: Some(5),
    });

    for (terminal, expected_name) in [
        (
            DownloadState::Rejected {
                reason: "denied".to_owned(),
            },
            "rejected",
        ),
        (
            DownloadState::Failed {
                reason: "failed".to_owned(),
            },
            "failed",
        ),
        (DownloadState::Completed, "completed"),
    ] {
        let mut download = DownloadTransfer::new("peer", "Music/file.flac", 7);
        download.state = terminal.clone();
        assert!(matches!(
            download.handle_peer_message(response.clone()),
            Err(ClientError::InvalidTransferState {
                operation: "handle peer message",
                state,
            }) if state == expected_name
        ));
        assert_eq!(download.state, terminal);
    }

    for (terminal, expected_name) in [
        (
            UploadState::Rejected {
                reason: "denied".to_owned(),
            },
            "rejected",
        ),
        (
            UploadState::Failed {
                reason: "failed".to_owned(),
            },
            "failed",
        ),
        (UploadState::Completed, "completed"),
    ] {
        let mut upload = UploadTransfer::new("peer", "Music/file.flac", 7, 5);
        upload.state = terminal.clone();
        assert!(matches!(
            upload.handle_peer_message(response.clone()),
            Err(ClientError::InvalidTransferState {
                operation: "handle peer message",
                state,
            }) if state == expected_name
        ));
        assert_eq!(upload.state, terminal);
    }
}

#[test]
fn accepted_download_rejects_stale_queue_and_request_messages() {
    let mut transfer = DownloadTransfer::new("peer", "Music/file.flac", 7);
    request_download(&mut transfer, Some(5));
    transfer.accept_upload_response_message(5).unwrap();

    for (message, operation) in [
        (
            PeerMessage::PlaceInQueueResponse {
                filename: "Music/file.flac".to_owned(),
                place: 1,
            },
            "handle queue position",
        ),
        (
            PeerMessage::TransferRequest(TransferRequest {
                direction: 1,
                token: 7,
                filename: "Music/file.flac".to_owned(),
                filename_encoding: ProtocolTextEncoding::Utf8,
                size: Some(6),
            }),
            "handle transfer request",
        ),
    ] {
        assert!(matches!(
            transfer.handle_peer_message(message),
            Err(ClientError::InvalidTransferState {
                operation: actual,
                state: "accepted",
            }) if actual == operation
        ));
        assert_eq!(transfer.state, DownloadState::Accepted { size: Some(5) });
    }
}

#[test]
fn local_commands_cannot_reopen_or_skip_transfer_states() {
    let mut download = DownloadTransfer::new("peer", "Music/file.flac", 7);
    download.state = DownloadState::Completed;
    assert!(matches!(
        download.queue_upload_message(),
        Err(ClientError::InvalidTransferState {
            operation: "queue download",
            state: "completed",
        })
    ));
    assert!(matches!(
        download.accept_upload_response_message(5),
        Err(ClientError::InvalidTransferState {
            operation: "accept download",
            state: "completed",
        })
    ));
    assert!(matches!(
        download.reject_upload_response_message("denied"),
        Err(ClientError::InvalidTransferState {
            operation: "reject download",
            state: "completed",
        })
    ));
    assert_eq!(download.state, DownloadState::Completed);

    let mut upload = UploadTransfer::new("peer", "Music/file.flac", 7, 5);
    upload.state = UploadState::Accepted;
    assert!(matches!(
        upload.transfer_request_message(),
        Err(ClientError::InvalidTransferState {
            operation: "request upload",
            state: "accepted",
        })
    ));
    assert_eq!(upload.state, UploadState::Accepted);
}

#[tokio::test]
async fn send_file_uses_requested_offset_and_marks_completed() {
    let (uploader, downloader) = duplex(64);
    let mut uploader = FileTransferConnection::new(uploader);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 5);
    transfer.transfer_request_message().unwrap();
    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();

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
async fn send_file_timeout_marks_upload_failed() {
    let (uploader, _silent_downloader) = duplex(64);
    let mut uploader = FileTransferConnection::new(uploader);
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 1);
    transfer.transfer_request_message().unwrap();
    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();

    assert!(matches!(
        transfer
            .send_file_to_with_timeout(&mut uploader, &[1], Duration::from_millis(10))
            .await,
        Err(ClientError::TimedOut {
            operation: "upload transfer I/O",
        })
    ));
    assert_eq!(
        transfer.state,
        UploadState::Failed {
            reason: "transfer I/O timed out".to_owned(),
        }
    );
}

#[tokio::test]
async fn send_file_rejects_offset_past_end() {
    let (uploader, downloader) = duplex(64);
    let mut uploader = FileTransferConnection::new(uploader);
    let mut downloader = FileTransferConnection::new(downloader);
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 5);
    transfer.transfer_request_message().unwrap();
    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();

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

#[tokio::test]
async fn send_file_rejects_payload_that_differs_from_advertised_size() {
    let (uploader, _downloader) = duplex(64);
    let mut uploader = FileTransferConnection::new(uploader);
    let mut transfer = UploadTransfer::new("peer", "Music/file.flac", 7, 3);
    transfer.transfer_request_message().unwrap();
    transfer
        .handle_peer_message(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 7,
            size: None,
        }))
        .unwrap();

    let error = transfer
        .send_file_to(&mut uploader, &[1, 2, 3, 4, 5])
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransferSizeMismatch {
            expected: 3,
            actual: 5
        }
    ));
    assert_eq!(transfer.state, UploadState::Accepted);
}

#[tokio::test]
async fn file_io_rejects_unaccepted_transfer_states_before_network_io() {
    let (download_stream, _) = duplex(64);
    let mut download_connection = FileTransferConnection::new(download_stream);
    let mut download = DownloadTransfer::new("peer", "Music/file.flac", 7);
    assert!(matches!(
        download
            .receive_file_from(&mut download_connection, 0, 1)
            .await,
        Err(ClientError::InvalidTransferState {
            operation: "receive file",
            state: "new",
        })
    ));
    assert_eq!(download.state, DownloadState::New);

    let (upload_stream, _) = duplex(64);
    let mut upload_connection = FileTransferConnection::new(upload_stream);
    let mut upload = UploadTransfer::new("peer", "Music/file.flac", 7, 1);
    assert!(matches!(
        upload.send_file_to(&mut upload_connection, &[1]).await,
        Err(ClientError::InvalidTransferState {
            operation: "send file",
            state: "new",
        })
    ));
    assert_eq!(upload.state, UploadState::New);
}

fn request_download(transfer: &mut DownloadTransfer, size: Option<u64>) {
    transfer
        .handle_peer_message(PeerMessage::TransferRequest(TransferRequest {
            direction: 1,
            token: transfer.token,
            filename: transfer.filename.clone(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size,
        }))
        .unwrap();
}
