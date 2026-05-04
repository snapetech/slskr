use slskr_protocol::{
    frame::MessageFrame,
    peer::{
        FileAttribute, FileEntry, FileSearchResponse, FolderContentsRequest, PeerCode, PeerMessage,
        TransferRequest, TransferResponse, UserInfo,
    },
};

#[test]
fn peer_codes_map_known_values() {
    assert_eq!(PeerCode::try_from(4), Ok(PeerCode::GetShareFileList));
    assert_eq!(
        PeerCode::try_from(52),
        Ok(PeerCode::UploadQueueNotification)
    );
    assert_eq!(PeerCode::try_from(99), Err(99));
}

#[test]
fn peer_core_messages_round_trip() {
    let messages = [
        PeerMessage::GetShareFileList,
        PeerMessage::FileSearchRequest {
            token: 10,
            query: "needle".to_owned(),
        },
        PeerMessage::PlaceholdUpload {
            filename: "Music/file.flac".to_owned(),
        },
        PeerMessage::UserInfoRequest,
        PeerMessage::FolderContentsRequest(FolderContentsRequest {
            token: 11,
            folder: "Music".to_owned(),
        }),
        PeerMessage::QueueUpload {
            filename: "Music/file.flac".to_owned(),
        },
        PeerMessage::PlaceInQueueResponse {
            filename: "Music/file.flac".to_owned(),
            place: 3,
        },
        PeerMessage::UploadFailed {
            filename: "Music/file.flac".to_owned(),
        },
        PeerMessage::UploadDenied {
            filename: "Music/file.flac".to_owned(),
            reason: "Queued".to_owned(),
        },
        PeerMessage::PlaceInQueueRequest {
            filename: "Music/file.flac".to_owned(),
        },
        PeerMessage::UploadQueueNotification,
    ];

    for message in messages {
        let decoded = PeerMessage::decode(message.encode().unwrap()).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn obsolete_or_undocumented_known_peer_codes_preserve_payload_under_named_variants() {
    let messages = [
        PeerMessage::PrivateMessage(vec![1, 2, 3]),
        PeerMessage::RoomInvitation(vec![4, 5, 6]),
        PeerMessage::CancelledQueuedTransfer(vec![7, 8, 9]),
        PeerMessage::SendConnectToken(vec![10, 11, 12]),
        PeerMessage::MoveDownloadToTop(vec![13, 14, 15]),
        PeerMessage::ExactFileSearchRequest(vec![16, 17, 18]),
        PeerMessage::QueuedDownloads(vec![19, 20, 21]),
        PeerMessage::IndirectFileSearchRequest(vec![22, 23, 24]),
    ];

    for message in messages {
        let decoded = PeerMessage::decode(message.encode().unwrap()).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn transfer_messages_round_trip() {
    let messages = [
        PeerMessage::TransferRequest(TransferRequest {
            direction: 1,
            token: 12,
            filename: "Music/file.flac".to_owned(),
            size: Some(1_000),
        }),
        PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 12,
            size: Some(1_000),
        }),
        PeerMessage::TransferResponse(TransferResponse::Rejected {
            token: 13,
            reason: "Queued".to_owned(),
        }),
    ];

    for message in messages {
        let decoded = PeerMessage::decode(message.encode().unwrap()).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn user_info_round_trips_with_optional_picture_and_permissions() {
    let message = PeerMessage::UserInfoResponse(UserInfo {
        description: "desc".to_owned(),
        picture: Some(vec![1, 2, 3]),
        total_uploads: 7,
        queue_size: 2,
        slots_free: true,
        upload_permissions: Some(1),
    });

    let decoded = PeerMessage::decode(message.encode().unwrap()).unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn file_search_response_round_trips() {
    let entry = FileEntry {
        code: 1,
        filename: "Music/file.flac".to_owned(),
        size: 1_000,
        extension: String::new(),
        attributes: vec![FileAttribute {
            code: 1,
            value: 320,
        }],
    };
    let message = PeerMessage::FileSearchResponse(FileSearchResponse {
        username: "peer".to_owned(),
        token: 14,
        results: vec![entry.clone()],
        slot_free: true,
        average_speed: 100,
        queue_length: 0,
        unknown: 0,
        private_results: vec![entry],
    });

    let decoded = PeerMessage::decode(message.encode().unwrap()).unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn compressed_peer_payloads_are_preserved() {
    for message in [
        PeerMessage::SharedFileListResponse(vec![1, 2, 3]),
        PeerMessage::FolderContentsResponse(vec![4, 5, 6]),
    ] {
        let decoded = PeerMessage::decode(message.encode().unwrap()).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn unknown_peer_messages_preserve_payload() {
    let message = PeerMessage::decode(MessageFrame::new(99, [1, 2, 3])).unwrap();

    assert_eq!(
        message,
        PeerMessage::Unknown {
            code: 99,
            payload: vec![1, 2, 3]
        }
    );
}
