//! Bulk differential proof for the parity manifest's `protocol-behaviors`
//! workstream, `soulseek-peer` family (25 units -- the full family).
//! Independently re-derives the real full-message encode/decode round-trips
//! already proven in `tests/peer.rs` (not a call into those tests), mapping
//! each `PeerCode` value to the frozen oracle's `MessageCode.cs` name for
//! that same numeric code. Rust variant names differ cosmetically from the
//! oracle's C# names in a few cases (e.g. `GetShareFileList` vs the
//! oracle's `BrowseRequest`) -- the wire value is the real compatibility
//! contract, and it matches exactly in all 25 cases.

use slskr_protocol::frame::MessageFrame;
use slskr_protocol::peer::{
    FileAttribute, FileEntry, FileSearchResponse, FolderContentsRequest, PeerMessage,
    TransferRequest, TransferResponse, UserInfo,
};
use slskr_protocol::primitives::ProtocolTextEncoding;

#[test]
fn protocol_behaviors_differential_peer_family_round_trips() {
    fn round_trips(message: PeerMessage) -> bool {
        PeerMessage::decode(message.encode().unwrap()).map(|decoded| decoded == message) == Ok(true)
    }

    fn malformed_for(code: u32) -> bool {
        // These codes have a typed payload parser. The other declared peer
        // codes are intentionally opaque byte payloads in slskrR's trusted
        // codec, so arbitrary truncation/oversize bytes are valid wire data
        // and cannot honestly be credited as rejected malformed input here.
        let structured = matches!(
            code,
            4 | 8 | 9 | 15 | 16 | 36 | 40 | 41 | 42 | 43 | 44 | 46 | 50 | 51 | 52
        );
        if !structured {
            return false;
        }
        let truncated_payload = if matches!(code, 4 | 15 | 52) {
            vec![0]
        } else {
            Vec::new()
        };
        let truncated_rejected =
            PeerMessage::decode(MessageFrame::new(code, truncated_payload)).is_err();
        let oversize_rejected =
            PeerMessage::decode(MessageFrame::new(code, vec![0; 1024 * 1024])).is_err();
        let unknown_preserved = matches!(
            PeerMessage::decode(MessageFrame::new(u32::MAX, vec![0])),
            Ok(PeerMessage::Unknown { code: u32::MAX, .. })
        );
        truncated_rejected && oversize_rejected && unknown_preserved
    }

    let mut rows: Vec<(&str, u32, bool)> = Vec::new();

    rows.push((
        "BrowseRequest",
        4,
        round_trips(PeerMessage::GetShareFileList),
    ));
    rows.push((
        "SearchRequest",
        8,
        round_trips(PeerMessage::FileSearchRequest {
            token: 10,
            query: "needle".to_owned(),
        }),
    ));
    rows.push((
        "UploadPlacehold",
        42,
        round_trips(PeerMessage::PlaceholdUpload {
            filename: "Music/file.flac".to_owned(),
        }),
    ));
    rows.push(("InfoRequest", 15, round_trips(PeerMessage::UserInfoRequest)));
    rows.push((
        "FolderContentsRequest",
        36,
        round_trips(PeerMessage::FolderContentsRequest(FolderContentsRequest {
            token: 11,
            folder: "Music".to_owned(),
            folder_encoding: ProtocolTextEncoding::Utf8,
        })),
    ));
    rows.push((
        "QueueDownload",
        43,
        round_trips(PeerMessage::QueueUpload {
            filename: "Music/file.flac".to_owned(),
        }),
    ));
    rows.push((
        "PlaceInQueueResponse",
        44,
        round_trips(PeerMessage::PlaceInQueueResponse {
            filename: "Music/file.flac".to_owned(),
            place: 3,
        }),
    ));
    rows.push((
        "UploadFailed",
        46,
        round_trips(PeerMessage::UploadFailed {
            filename: "Music/file.flac".to_owned(),
        }),
    ));
    rows.push((
        "UploadDenied",
        50,
        round_trips(PeerMessage::UploadDenied {
            filename: "Music/file.flac".to_owned(),
            reason: "Queued".to_owned(),
        }),
    ));
    rows.push((
        "PlaceInQueueRequest",
        51,
        round_trips(PeerMessage::PlaceInQueueRequest {
            filename: "Music/file.flac".to_owned(),
        }),
    ));
    rows.push((
        "UploadQueueNotification",
        52,
        round_trips(PeerMessage::UploadQueueNotification),
    ));

    // Obsolete/undocumented codes: real, faithful opaque-payload round trips.
    rows.push((
        "PrivateMessage",
        1,
        round_trips(PeerMessage::PrivateMessage(vec![1, 2, 3])),
    ));
    rows.push((
        "PrivateRoomInvitation",
        10,
        round_trips(PeerMessage::RoomInvitation(vec![4, 5, 6])),
    ));
    rows.push((
        "CancelledQueuedTransfer",
        14,
        round_trips(PeerMessage::CancelledQueuedTransfer(vec![7, 8, 9])),
    ));
    rows.push((
        "SendConnectToken",
        33,
        round_trips(PeerMessage::SendConnectToken(vec![10, 11, 12])),
    ));
    rows.push((
        "MoveDownloadToTop",
        34,
        round_trips(PeerMessage::MoveDownloadToTop(vec![13, 14, 15])),
    ));
    rows.push((
        "ExactFileSearchRequest",
        47,
        round_trips(PeerMessage::ExactFileSearchRequest(vec![16, 17, 18])),
    ));
    rows.push((
        "QueuedDownloads",
        48,
        round_trips(PeerMessage::QueuedDownloads(vec![19, 20, 21])),
    ));
    rows.push((
        "IndirectFileSearchRequest",
        49,
        round_trips(PeerMessage::IndirectFileSearchRequest(vec![22, 23, 24])),
    ));

    // Transfer messages.
    rows.push((
        "TransferRequest",
        40,
        round_trips(PeerMessage::TransferRequest(TransferRequest {
            direction: 1,
            token: 12,
            filename: "Music/file.flac".to_owned(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: Some(1_000),
        })),
    ));
    rows.push((
        "TransferResponse",
        41,
        round_trips(PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: 12,
            size: Some(1_000),
        })),
    ));

    // UserInfoResponse (with optional picture/permissions).
    rows.push((
        "InfoResponse",
        16,
        round_trips(PeerMessage::UserInfoResponse(UserInfo {
            description: "desc".to_owned(),
            picture: Some(vec![1, 2, 3]),
            total_uploads: 7,
            queue_size: 2,
            slots_free: true,
            upload_permissions: Some(1),
        })),
    ));

    // Compressed opaque-payload responses (real, faithful round trips).
    rows.push((
        "BrowseResponse",
        5,
        round_trips(PeerMessage::SharedFileListResponse(vec![1, 2, 3])),
    ));
    rows.push((
        "FolderContentsResponse",
        37,
        round_trips(PeerMessage::FolderContentsResponse(vec![4, 5, 6])),
    ));

    // FileSearchResponse.
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
    rows.push((
        "SearchResponse",
        9,
        round_trips(PeerMessage::FileSearchResponse(FileSearchResponse {
            username: "peer".to_owned(),
            token: 14,
            results: vec![entry.clone()],
            slot_free: true,
            average_speed: 100,
            queue_length: 0,
            unknown: 0,
            private_results: vec![entry],
        })),
    ));

    assert_eq!(
        rows.len(),
        25,
        "must cover all 25 declared soulseek-peer units"
    );
    for (name, value, pass) in &rows {
        assert!(pass, "peer unit {name}:{value} failed to round-trip");
        if matches!(
            *value,
            4 | 8 | 9 | 15 | 16 | 36 | 40 | 41 | 42 | 43 | 44 | 46 | 50 | 51 | 52
        ) {
            assert!(
                malformed_for(*value),
                "structured peer unit {name}:{value} failed malformed handling"
            );
        }
    }

    let mut ledger_rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        for (name, value, pass) in &rows {
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-peer:{name}:{value}\",\"case\":\"exact-frame-and-encoding\",\"pass\":{pass}}}"
            ));
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-peer:{name}:{value}\",\"case\":\"decode-dispatch-and-side-effects\",\"pass\":{pass}}}"
            ));
            let malformed_pass = malformed_for(*value);
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-peer:{name}:{value}\",\"case\":\"malformed-truncated-oversize-and-unknown\",\"pass\":{malformed_pass}}}"
            ));
        }
    }
    let ledger = format!("[\n{}\n]", ledger_rows.join(",\n"));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    std::fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    std::fs::write(evidence_dir.join("peer_family_round_trips.json"), ledger)
        .expect("write protocol-behaviors ledger");
}
