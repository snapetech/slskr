use std::collections::HashSet;

use flate2::{write::ZlibEncoder, Compression};
use slskr_protocol::{
    frame::MessageFrame,
    peer::{
        FileAttribute, FileEntry, FileSearchResponse, FolderContentsRequest, PeerCode, PeerMessage,
        TransferRequest, TransferResponse, UserInfo, MAX_FILE_ATTRIBUTES, MAX_FILE_SEARCH_RESULTS,
    },
    DecodeError, EncodeError, ProtocolTextEncoding,
};
use std::io::Write;

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
fn peer_code_inventory_is_complete_and_unique() {
    let mut seen = HashSet::new();

    for code in PeerCode::ALL {
        assert!(
            seen.insert(code.as_u32()),
            "duplicate peer code {}",
            code.as_u32()
        );
        assert_eq!(PeerCode::try_from(code.as_u32()), Ok(*code));
    }

    assert_eq!(PeerCode::ALL.len(), 25);
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
            folder_encoding: ProtocolTextEncoding::Utf8,
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
            filename_encoding: ProtocolTextEncoding::Utf8,
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
        filename_encoding: ProtocolTextEncoding::Utf8,
        size: 1_000,
        extension: String::new(),
        extension_encoding: ProtocolTextEncoding::Utf8,
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
fn legacy_peer_paths_preserve_windows_1251_bytes_across_round_trips() {
    let folder = PeerMessage::FolderContentsRequest(FolderContentsRequest {
        token: 21,
        folder: "Музыка".to_owned(),
        folder_encoding: ProtocolTextEncoding::Windows1251,
    });
    let folder_frame = folder.encode().unwrap();
    assert_eq!(
        &folder_frame.payload[8..],
        &[0xCC, 0xF3, 0xE7, 0xFB, 0xEA, 0xE0]
    );
    assert_eq!(PeerMessage::decode(folder_frame).unwrap(), folder);

    let transfer = PeerMessage::TransferRequest(TransferRequest {
        direction: 0,
        token: 22,
        filename: "Музыка\\песня.flac".to_owned(),
        filename_encoding: ProtocolTextEncoding::Windows1251,
        size: None,
    });
    let transfer_frame = transfer.encode().unwrap();
    assert!(transfer_frame
        .payload
        .windows(6)
        .any(|window| window == [0xCC, 0xF3, 0xE7, 0xFB, 0xEA, 0xE0]));
    assert_eq!(PeerMessage::decode(transfer_frame).unwrap(), transfer);

    let search = PeerMessage::FileSearchResponse(FileSearchResponse {
        username: "peer".to_owned(),
        token: 23,
        results: vec![FileEntry {
            code: 1,
            filename: "Музыка\\песня.flac".to_owned(),
            filename_encoding: ProtocolTextEncoding::Windows1251,
            size: 1_000,
            extension: "флак".to_owned(),
            extension_encoding: ProtocolTextEncoding::Windows1251,
            attributes: Vec::new(),
        }],
        slot_free: true,
        average_speed: 100,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    });
    let search_frame = search.encode().unwrap();
    let search_payload = zlib_decode(&search_frame.payload);
    assert!(search_payload
        .windows(6)
        .any(|window| window == [0xCC, 0xF3, 0xE7, 0xFB, 0xEA, 0xE0]));
    assert_eq!(PeerMessage::decode(search_frame).unwrap(), search);
}

#[test]
fn file_search_response_rejects_untrusted_count_without_preallocating() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&4_u32.to_le_bytes());
    payload.extend_from_slice(b"peer");
    payload.extend_from_slice(&14_u32.to_le_bytes());
    payload.extend_from_slice(&u32::MAX.to_le_bytes());

    let decoded = PeerMessage::decode(MessageFrame::new(
        PeerCode::FileSearchResponse.as_u32(),
        zlib(payload),
    ));
    assert!(decoded.is_err());
}

#[test]
fn file_search_response_bounds_total_result_entries() {
    let entry = FileEntry {
        code: 1,
        filename: String::new(),
        filename_encoding: ProtocolTextEncoding::Utf8,
        size: 0,
        extension: String::new(),
        extension_encoding: ProtocolTextEncoding::Utf8,
        attributes: Vec::new(),
    };
    let message = PeerMessage::FileSearchResponse(FileSearchResponse {
        username: "peer".to_owned(),
        token: 14,
        results: vec![entry.clone(); MAX_FILE_SEARCH_RESULTS],
        slot_free: true,
        average_speed: 0,
        queue_length: 0,
        unknown: 0,
        private_results: vec![entry],
    });
    assert!(matches!(
        message.encode().unwrap_err(),
        EncodeError::CountTooLarge {
            field: "file search results",
            count,
            maximum: MAX_FILE_SEARCH_RESULTS,
        } if count == MAX_FILE_SEARCH_RESULTS + 1
    ));

    let declared = MAX_FILE_SEARCH_RESULTS + 1;
    let mut payload = Vec::new();
    payload.extend_from_slice(&4_u32.to_le_bytes());
    payload.extend_from_slice(b"peer");
    payload.extend_from_slice(&14_u32.to_le_bytes());
    payload.extend_from_slice(&u32::try_from(declared).unwrap().to_le_bytes());
    payload.resize(payload.len() + declared * 21, 0);
    assert!(matches!(
        PeerMessage::decode(MessageFrame::new(
            PeerCode::FileSearchResponse.as_u32(),
            zlib(payload),
        )),
        Err(DecodeError::InvalidCount {
            field: "file search results",
            count,
            maximum: MAX_FILE_SEARCH_RESULTS,
        }) if count == declared
    ));
}

#[test]
fn file_search_response_rejects_untrusted_attribute_count_without_looping() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&4_u32.to_le_bytes());
    payload.extend_from_slice(b"peer");
    payload.extend_from_slice(&14_u32.to_le_bytes());
    payload.extend_from_slice(&1_u32.to_le_bytes());
    payload.push(1);
    payload.extend_from_slice(&4_u32.to_le_bytes());
    payload.extend_from_slice(b"song");
    payload.extend_from_slice(&123_u64.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&u32::MAX.to_le_bytes());

    let decoded = PeerMessage::decode(MessageFrame::new(
        PeerCode::FileSearchResponse.as_u32(),
        zlib(payload),
    ));
    assert!(decoded.is_err());
}

#[test]
fn file_search_response_bounds_attributes_per_file() {
    let attributes = (0..=MAX_FILE_ATTRIBUTES)
        .map(|code| FileAttribute {
            code: code as u32,
            value: 1,
        })
        .collect::<Vec<_>>();
    let message = PeerMessage::FileSearchResponse(FileSearchResponse {
        username: "peer".to_owned(),
        token: 14,
        results: vec![FileEntry {
            code: 1,
            filename: "song".to_owned(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: 123,
            extension: String::new(),
            extension_encoding: ProtocolTextEncoding::Utf8,
            attributes: attributes.clone(),
        }],
        slot_free: true,
        average_speed: 0,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    });
    assert!(matches!(
        message.encode().unwrap_err(),
        EncodeError::CountTooLarge {
            field: "file attributes",
            count,
            maximum: MAX_FILE_ATTRIBUTES,
        } if count == MAX_FILE_ATTRIBUTES + 1
    ));

    let mut payload = Vec::new();
    payload.extend_from_slice(&4_u32.to_le_bytes());
    payload.extend_from_slice(b"peer");
    payload.extend_from_slice(&14_u32.to_le_bytes());
    payload.extend_from_slice(&1_u32.to_le_bytes());
    payload.push(1);
    payload.extend_from_slice(&4_u32.to_le_bytes());
    payload.extend_from_slice(b"song");
    payload.extend_from_slice(&123_u64.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&u32::try_from(attributes.len()).unwrap().to_le_bytes());
    for attribute in attributes {
        payload.extend_from_slice(&attribute.code.to_le_bytes());
        payload.extend_from_slice(&attribute.value.to_le_bytes());
    }
    assert!(matches!(
        PeerMessage::decode(MessageFrame::new(
            PeerCode::FileSearchResponse.as_u32(),
            zlib(payload),
        )),
        Err(DecodeError::InvalidCount {
            field: "file attributes",
            count,
            maximum: MAX_FILE_ATTRIBUTES,
        }) if count == MAX_FILE_ATTRIBUTES + 1
    ));
}

#[test]
fn file_search_response_ignores_optional_trailing_fields() {
    let message = PeerMessage::FileSearchResponse(FileSearchResponse {
        username: "peer".to_owned(),
        token: 14,
        results: vec![FileEntry {
            code: 1,
            filename: "Music/file.flac".to_owned(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: 1_000,
            extension: String::new(),
            extension_encoding: ProtocolTextEncoding::Utf8,
            attributes: Vec::new(),
        }],
        slot_free: true,
        average_speed: 100,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    });
    let encoded = message.encode().unwrap();
    let mut payload = zlib_decode(&encoded.payload);
    payload.extend_from_slice(b"optional-search-response-tail");

    let decoded = PeerMessage::decode(MessageFrame::new(
        PeerCode::FileSearchResponse.as_u32(),
        zlib(payload),
    ))
    .unwrap();

    assert_eq!(decoded, message);
}

fn zlib(payload: Vec<u8>) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&payload).unwrap();
    encoder.finish().unwrap()
}

fn zlib_decode(payload: &[u8]) -> Vec<u8> {
    use std::io::Read;

    let mut decoder = flate2::read::ZlibDecoder::new(payload);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output).unwrap();
    output
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
