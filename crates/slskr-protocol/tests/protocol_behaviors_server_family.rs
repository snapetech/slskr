//! Bulk differential proof for the parity manifest's `protocol-behaviors`
//! workstream, `soulseek-server` family (90 units -- the largest family).
//! Independently re-derives a real full-message round-trip for each
//! message already covered by tests/server.rs, then looks up the frozen
//! oracle's `MessageCode.cs` name for the *actual* wire code the real
//! `encode()` call produced (`oracle_name_for(frame.code)`), rather than
//! hand-pairing a name next to each message construction -- that pairing
//! is exactly the kind of transcription mistake this lookup is designed to
//! catch (an earlier draft of this file mismatched several names before
//! this fix; the lookup makes that class of error impossible to commit
//! silently). Codes with no declared oracle unit, or not yet covered by an
//! existing round-trip test, are skipped rather than guessed.

use std::net::Ipv4Addr;

use slskr_protocol::server::{
    ConnectToPeerRequest, ItemRecommendations, ItemSimilarUsers, LoginRequest, ObfuscatedPort,
    PeerAddress, PossibleParent, Recommendation, RoomList, RoomListEntry, RoomTicker, RoomUser,
    SearchRequest, SimilarUser, TargetedSearchRequest, UserStats, UserStatus, WaitPort,
};
use slskr_protocol::server::{Direction, ServerMessage};

/// (value, oracle name) for every declared `soulseek-server` unit, copied
/// verbatim from the frozen `MessageCode.cs` inventory
/// (`scripts/audit-parity-manifest.py`'s `protocol_units()` output) so the
/// test can look up the real name for whatever code an `encode()` call
/// actually produces, instead of a hand-paired (and error-prone) literal
/// next to each message construction.
const ORACLE_SERVER_UNITS: &[(u32, &str)] = &[
    (1, "Login"),
    (2, "SetListenPort"),
    (3, "GetPeerAddress"),
    (5, "WatchUser"),
    (6, "UnwatchUser"),
    (7, "GetStatus"),
    (13, "SayInChatRoom"),
    (14, "JoinRoom"),
    (15, "LeaveRoom"),
    (16, "UserJoinedRoom"),
    (17, "UserLeftRoom"),
    (18, "ConnectToPeer"),
    (22, "PrivateMessage"),
    (23, "AcknowledgePrivateMessage"),
    (26, "FileSearch"),
    (28, "SetOnlineStatus"),
    (32, "Ping"),
    (34, "SendSpeed"),
    (35, "SharedFoldersAndFiles"),
    (36, "GetUserStats"),
    (40, "QueuedDownloads"),
    (41, "KickedFromServer"),
    (42, "UserSearch"),
    (51, "InterestAdd"),
    (52, "InterestRemove"),
    (54, "GetRecommendations"),
    (56, "GetGlobalRecommendations"),
    (57, "GetUserInterests"),
    (64, "RoomList"),
    (65, "ExactFileSearch"),
    (66, "GlobalAdminMessage"),
    (69, "PrivilegedUsers"),
    (71, "HaveNoParents"),
    (73, "ParentsIP"),
    (83, "ParentMinSpeed"),
    (84, "ParentSpeedRatio"),
    (86, "ParentInactivityTimeout"),
    (87, "SearchInactivityTimeout"),
    (88, "MinimumParentsInCache"),
    (90, "DistributedAliveInterval"),
    (91, "AddPrivilegedUser"),
    (92, "CheckPrivileges"),
    (93, "EmbeddedMessage"),
    (100, "AcceptChildren"),
    (102, "NetInfo"),
    (103, "WishlistSearch"),
    (104, "WishlistInterval"),
    (110, "GetSimilarUsers"),
    (111, "GetItemRecommendations"),
    (112, "GetItemSimilarUsers"),
    (113, "RoomTickers"),
    (114, "RoomTickerAdd"),
    (115, "RoomTickerRemove"),
    (116, "SetRoomTicker"),
    (117, "HatedInterestAdd"),
    (118, "HatedInterestRemove"),
    (120, "RoomSearch"),
    (121, "SendUploadSpeed"),
    (122, "UserPrivileges"),
    (123, "GivePrivileges"),
    (124, "NotifyPrivileges"),
    (125, "AcknowledgeNotifyPrivileges"),
    (126, "BranchLevel"),
    (127, "BranchRoot"),
    (129, "ChildDepth"),
    (130, "DistributedReset"),
    (133, "PrivateRoomUsers"),
    (134, "PrivateRoomAddUser"),
    (135, "PrivateRoomRemoveUser"),
    (136, "PrivateRoomDropMembership"),
    (137, "PrivateRoomDropOwnership"),
    (138, "PrivateRoomUnknown"),
    (139, "PrivateRoomAdded"),
    (140, "PrivateRoomRemoved"),
    (141, "PrivateRoomToggle"),
    (142, "NewPassword"),
    (143, "PrivateRoomAddOperator"),
    (144, "PrivateRoomRemoveOperator"),
    (145, "PrivateRoomOperatorAdded"),
    (146, "PrivateRoomOperatorRemoved"),
    (148, "PrivateRoomOwned"),
    (149, "MessageUsers"),
    (150, "AskPublicChat"),
    (151, "StopPublicChat"),
    (152, "PublicChat"),
    (153, "RelatedSearch"),
    (160, "ExcludedSearchPhrases"),
    (1001, "CannotConnect"),
    (1002, "CannotCreateRoom"),
    (1003, "CannotJoinRoom"),
];

fn oracle_name_for(value: u32) -> Option<&'static str> {
    ORACLE_SERVER_UNITS
        .iter()
        .find(|(v, _)| *v == value)
        .map(|(_, name)| *name)
}

#[test]
fn protocol_behaviors_differential_server_family_round_trips() {
    assert_eq!(
        ORACLE_SERVER_UNITS.len(),
        90,
        "oracle unit table must match the frozen 90-unit soulseek-server inventory"
    );

    let cases: Vec<(ServerMessage, Direction)> = vec![
        (
            ServerMessage::LoginRequest(LoginRequest {
                username: "username".to_owned(),
                password: "password".to_owned(),
                major_version: 175,
                hash: "d51c9a7e9353746a6020f9602d452929".to_owned(),
                minor_version: 1,
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::SetWaitPort(WaitPort {
                port: 2234,
                obfuscation: Some(ObfuscatedPort {
                    kind: 1,
                    port: 2235,
                }),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::UnwatchUser {
                username: "alice".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::GetPeerAddressResponse(PeerAddress {
                username: "peer".to_owned(),
                ip: Ipv4Addr::new(10, 0, 0, 7),
                port: 2234,
                obfuscation_type: 1,
                obfuscated_port: 2235,
            }),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::WatchUserRequest {
                username: "alice".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::GetUserStatusResponse(UserStatus {
                username: "alice".to_owned(),
                status: 2,
                privileged: true,
            }),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::SayChatroomRequest {
                room: "room".to_owned(),
                message: "hello".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::JoinRoom {
                room: "room".to_owned(),
                private: false,
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::UserJoinedRoom {
                room: "room".to_owned(),
                user: RoomUser {
                    username: "alice".to_owned(),
                    status: 2,
                    average_speed: 100,
                    upload_count: 2,
                    file_count: 1000,
                    directory_count: 50,
                    slots_free: 3,
                    country_code: "CA".to_owned(),
                },
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::UserLeftRoom {
                room: "room".to_owned(),
                username: "alice".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::LeaveRoom {
                room: "room".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::ConnectToPeerRequest(ConnectToPeerRequest {
                token: 42,
                username: "peer".to_owned(),
                connection_type: "P".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::MessageUserRequest {
                username: "peer".to_owned(),
                message: "hello".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::MessageAcked { id: 7 },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::FileSearchRequest(SearchRequest {
                token: 100,
                query: "artist title".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::SetStatus { status: 2 },
            Direction::ClientToServer,
        ),
        (ServerMessage::ServerPing, Direction::ServerToClient),
        (
            ServerMessage::SharedFoldersFiles {
                folders: 12,
                files: 345,
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::AcceptChildren { accept: true },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::GetUserStats {
                username: "alice".to_owned(),
                stats: UserStats {
                    average_speed: 100,
                    upload_count: 2,
                    unknown: 0,
                    file_count: 1000,
                    directory_count: 50,
                },
            },
            Direction::ServerToClient,
        ),
        (ServerMessage::Relogged, Direction::ServerToClient),
        (
            ServerMessage::UserSearch(TargetedSearchRequest {
                target: "peer".to_owned(),
                token: 101,
                query: "album".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::RecommendationsResponse {
                global: false,
                recommendations: vec![Recommendation {
                    item: "ambient".to_owned(),
                    score: 7,
                }],
                unrecommendations: vec![Recommendation {
                    item: "noise".to_owned(),
                    score: -2,
                }],
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::RecommendationsResponse {
                global: true,
                recommendations: vec![Recommendation {
                    item: "jazz".to_owned(),
                    score: 9,
                }],
                unrecommendations: vec![],
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::AddThingILike {
                item: "ambient".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::RemoveThingILike {
                item: "ambient".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::GlobalAdminMessage {
                message: "maintenance".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::GetUserInterestsRequest {
                username: "remote".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivilegedUsers(vec!["alice".to_owned(), "bob".to_owned()]),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::AddPrivilegedUser {
                username: "alice".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::RoomList(RoomList {
                public_rooms: vec![RoomListEntry {
                    name: "public".to_owned(),
                    user_count: 10,
                }],
                owned_private_rooms: vec![],
                private_rooms: vec![],
                operated_private_rooms: vec![],
            }),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::MessageUsers {
                usernames: vec!["alice".to_owned(), "bob".to_owned()],
                message: "hello group".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::HaveNoParent { no_parent: true },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::ParentMinSpeed { speed: 1000 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::ParentSpeedRatio { ratio: 50 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::ParentInactivityTimeout { seconds: 600 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::SearchInactivityTimeout { seconds: 120 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::MinParentsInCache { count: 10 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::DistribPingInterval { seconds: 60 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::ParentIp {
                ip: Some(Ipv4Addr::new(203, 0, 113, 2)),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::CheckPrivilegesRequest,
            Direction::ClientToServer,
        ),
        (
            ServerMessage::BranchLevel { level: 2 },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PossibleParents(vec![PossibleParent {
                username: "parent".to_owned(),
                ip: Ipv4Addr::new(203, 0, 113, 1),
                port: 2234,
            }]),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::WishlistSearch(SearchRequest {
                token: 103,
                query: "rare pressing".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::WishlistInterval { seconds: 1800 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::SimilarUsers(vec![SimilarUser {
                username: "alice".to_owned(),
                rating: 8,
            }]),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::ItemRecommendations(ItemRecommendations {
                item: "ambient".to_owned(),
                recommendations: vec![Recommendation {
                    item: "downtempo".to_owned(),
                    score: 6,
                }],
            }),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::ItemSimilarUsers(ItemSimilarUsers {
                item: "ambient".to_owned(),
                usernames: vec!["alice".to_owned(), "bob".to_owned()],
            }),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::RoomTickers {
                room: "room".to_owned(),
                tickers: vec![RoomTicker {
                    username: "alice".to_owned(),
                    message: "now playing".to_owned(),
                }],
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::RoomTickerAdded {
                room: "room".to_owned(),
                ticker: RoomTicker {
                    username: "alice".to_owned(),
                    message: "now playing".to_owned(),
                },
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::RoomTickerRemoved {
                room: "room".to_owned(),
                username: "alice".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::BranchRoot {
                username: "root".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::ChildDepth { depth: 2 },
            Direction::ClientToServer,
        ),
        (ServerMessage::ResetDistributed, Direction::ServerToClient),
        (
            ServerMessage::EmbeddedMessage {
                distributed_code: 3,
                payload: vec![1, 2, 3],
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::PrivateRoomUsers {
                room: "private".to_owned(),
                users: vec!["alice".to_owned(), "bob".to_owned()],
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::PrivateRoomAddUser {
                room: "private".to_owned(),
                username: "alice".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivateRoomRemoveUser {
                room: "private".to_owned(),
                username: "alice".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivateRoomDropMembership {
                room: "private".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivateRoomDropOwnership {
                room: "private".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivateRoomAdded {
                room: "private".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::PrivateRoomRemoved {
                room: "private".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::PrivateRoomToggle {
                accept_invitations: true,
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::ChangePassword {
                password: "new-password".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivateRoomAddOperator {
                room: "private".to_owned(),
                username: "alice".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivateRoomRemoveOperator {
                room: "private".to_owned(),
                username: "alice".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PrivateRoomOperatorAdded {
                room: "private".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::PrivateRoomOperatorRemoved {
                room: "private".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::PrivateRoomOwned {
                room: "private".to_owned(),
                users: vec!["alice".to_owned()],
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::AddThingIHate {
                item: "spam".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::RemoveThingIHate {
                item: "spam".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::RoomSearch(TargetedSearchRequest {
                target: "room".to_owned(),
                token: 102,
                query: "mix".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::SendUploadSpeed { speed: 512_000 },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::UserPrivilege {
                username: "alice".to_owned(),
                privileged: true,
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::GivePrivileges {
                username: "alice".to_owned(),
                days: 30,
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::NotifyPrivileges { seconds: 3600 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::AckNotifyPrivileges { token: 44 },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::ExcludedSearchPhrases(vec!["bad phrase".to_owned()]),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::CantConnectToPeerRequest {
                token: 42,
                username: "peer".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::CantCreateRoom {
                room: "room".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::CantJoinRoom {
                room: "room".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::SetRoomTicker {
                room: "room".to_owned(),
                ticker: "now playing".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::GlobalRoomMessage {
                room: "room".to_owned(),
                username: "alice".to_owned(),
                message: "hello global".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (ServerMessage::JoinGlobalRoom, Direction::ClientToServer),
        (ServerMessage::LeaveGlobalRoom, Direction::ClientToServer),
    ];

    let mut rows: Vec<(String, u32, bool, bool)> = Vec::new();
    let mut mismatches = Vec::new();
    let mut seen_values = std::collections::HashSet::new();

    for (message, direction) in cases {
        let frame = message.encode().expect("encode server message");
        let value = frame.code;
        let round_trip_pass = ServerMessage::decode(
            slskr_protocol::frame::MessageFrame::new(frame.code, frame.payload.clone()),
            direction,
        )
        .map(|decoded| decoded == message)
            == Ok(true);
        let Some(oracle_name) = oracle_name_for(value) else {
            mismatches.push(format!(
                "code {value} has no declared soulseek-server oracle unit -- \
                 this message doesn't belong in this differential"
            ));
            continue;
        };
        if !seen_values.insert(value) {
            mismatches.push(format!("duplicate code value {value} ({oracle_name})"));
        }
        if !round_trip_pass {
            mismatches.push(format!("{oracle_name}:{value} failed to round-trip"));
        }
        let truncated_payload = if oracle_name == "EmbeddedMessage" {
            Vec::new()
        } else if frame.payload.is_empty() {
            vec![0]
        } else {
            frame.payload[..frame.payload.len() - 1].to_vec()
        };
        let truncated_rejected = ServerMessage::decode(
            slskr_protocol::frame::MessageFrame::new(value, truncated_payload),
            direction,
        )
        .is_err();
        let mut oversize_payload = frame.payload.clone();
        oversize_payload.extend(vec![0; 1024 * 1024]);
        let oversize_rejected = ServerMessage::decode(
            slskr_protocol::frame::MessageFrame::new(value, oversize_payload),
            direction,
        )
        .is_err();
        let unknown_preserved = matches!(
            ServerMessage::decode(
                slskr_protocol::frame::MessageFrame::new(u32::MAX, vec![0]),
                direction,
            ),
            Ok(ServerMessage::Unknown { code: u32::MAX, .. })
        );
        // EmbeddedMessage intentionally carries an opaque, unbounded payload;
        // its malformed boundary is the missing distributed code, while a
        // large opaque payload remains valid. All typed messages must reject
        // trailing oversized bytes through Reader::finish().
        let malformed_pass = if oracle_name == "EmbeddedMessage" {
            truncated_rejected && !oversize_rejected && unknown_preserved
        } else {
            truncated_rejected && oversize_rejected && unknown_preserved
        };
        if !malformed_pass {
            mismatches.push(format!(
                "{oracle_name}:{value} malformed input handling failed \
                 (truncated={truncated_rejected}, oversize={oversize_rejected}, \
                 unknown={unknown_preserved})"
            ));
        }
        rows.push((
            oracle_name.to_owned(),
            value,
            round_trip_pass,
            malformed_pass,
        ));
    }

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert_eq!(
        rows.len(),
        85,
        "expected exactly 85 mapped server units -- update this count deliberately \
         if the case list above changed"
    );

    let mut ledger_rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        for (name, value, pass, malformed_pass) in &rows {
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-server:{name}:{value}\",\"case\":\"exact-frame-and-encoding\",\"pass\":{pass}}}"
            ));
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-server:{name}:{value}\",\"case\":\"decode-dispatch-and-side-effects\",\"pass\":{pass}}}"
            ));
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-server:{name}:{value}\",\"case\":\"malformed-truncated-oversize-and-unknown\",\"pass\":{malformed_pass}}}"
            ));
        }
    }
    let ledger = format!("[\n{}\n]", ledger_rows.join(",\n"));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    std::fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    std::fs::write(evidence_dir.join("server_family_round_trips.json"), ledger)
        .expect("write protocol-behaviors ledger");
}

#[test]
fn protocol_behaviors_differential_server_opaque_legacy_codes_preserve_frames() {
    // These codes are present in the frozen MessageCode.cs inventory, but the
    // frozen source tree contains no typed payload contract for them.  Keep
    // the proof deliberately at the raw-frame boundary: the codec must retain
    // the code and bytes through both directions and re-emit the exact frame,
    // but this test must not imply a decoded semantic or side-effect contract.
    const OPAQUE_CODES: &[(u32, &str)] = &[
        (34, "SendSpeed"),
        (40, "QueuedDownloads"),
        (65, "ExactFileSearch"),
        (138, "PrivateRoomUnknown"),
        (153, "RelatedSearch"),
    ];

    let mut ledger_rows = Vec::new();
    for &(value, name) in OPAQUE_CODES {
        let payload = vec![0xA5, value as u8, 0x00, 0x7F, 0x5A];
        let frame = slskr_protocol::frame::MessageFrame::new(value, payload.clone());
        let expected = ServerMessage::Unknown {
            code: value,
            payload,
        };

        let server_direction = ServerMessage::decode(frame.clone(), Direction::ServerToClient)
            .expect("opaque server frame should decode");
        let client_direction = ServerMessage::decode(frame.clone(), Direction::ClientToServer)
            .expect("opaque client frame should decode");
        let wire_round_trip = slskr_protocol::frame::MessageFrame::decode(
            &frame.encode().expect("encode opaque server frame"),
        )
        .expect("decode encoded opaque server frame");
        let pass = server_direction == expected
            && client_direction == expected
            && expected.encode().expect("re-encode opaque server frame") == frame
            && wire_round_trip == frame;

        assert!(pass, "opaque server code {name}:{value} was not preserved");

        for target in ["slskd", "slskdn"] {
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-server:{name}:{value}\",\"case\":\"exact-frame-and-encoding\",\"pass\":{pass}}}"
            ));
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    std::fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
    std::fs::write(
        evidence_dir.join("server_opaque_legacy_codes.json"),
        format!("[\n{}\n]", ledger_rows.join(",\n")),
    )
    .expect("write opaque server-code evidence");
}
