use std::{collections::HashSet, net::Ipv4Addr};

use slskr_protocol::{
    frame::MessageFrame,
    server::{
        Direction, LoginRequest, LoginResponse, ObfuscatedPort, PeerAddress, RoomList,
        RoomListEntry, ServerCode, ServerMessage, TargetedSearchRequest, UserStats, WaitPort,
    },
};

#[test]
fn server_code_table_maps_known_codes() {
    assert_eq!(ServerCode::try_from(1), Ok(ServerCode::Login));
    assert_eq!(
        ServerCode::try_from(160),
        Ok(ServerCode::ExcludedSearchPhrases)
    );
    assert_eq!(ServerCode::try_from(1003), Ok(ServerCode::CantCreateRoom));
    assert_eq!(ServerCode::try_from(4), Err(4));
    assert_eq!(ServerCode::Login.name(), "Login");
}

#[test]
fn server_code_inventory_is_complete_and_unique() {
    let mut seen = HashSet::new();

    for code in ServerCode::ALL {
        assert!(
            seen.insert(code.as_u32()),
            "duplicate server code {}",
            code.as_u32()
        );
        assert_eq!(ServerCode::try_from(code.as_u32()), Ok(*code));
        assert!(!code.name().is_empty());
    }

    assert_eq!(ServerCode::ALL.len(), 102);
}

#[test]
fn login_request_matches_protocol_example() {
    let message = ServerMessage::LoginRequest(LoginRequest {
        username: "username".to_owned(),
        password: "password".to_owned(),
        major_version: 175,
        hash: "d51c9a7e9353746a6020f9602d452929".to_owned(),
        minor_version: 1,
    });

    let frame = message.encode().unwrap();
    let encoded = frame.encode().unwrap();

    assert_eq!(
        encoded,
        [
            0x48, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x75, 0x73,
            0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65, 0x08, 0x00, 0x00, 0x00, 0x70, 0x61, 0x73, 0x73,
            0x77, 0x6f, 0x72, 0x64, 0xaf, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x64, 0x35,
            0x31, 0x63, 0x39, 0x61, 0x37, 0x65, 0x39, 0x33, 0x35, 0x33, 0x37, 0x34, 0x36, 0x61,
            0x36, 0x30, 0x32, 0x30, 0x66, 0x39, 0x36, 0x30, 0x32, 0x64, 0x34, 0x35, 0x32, 0x39,
            0x32, 0x39, 0x01, 0x00, 0x00, 0x00,
        ]
    );

    let decoded = ServerMessage::decode(
        MessageFrame::decode(&encoded).unwrap(),
        Direction::ClientToServer,
    )
    .unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn login_responses_round_trip() {
    let success = ServerMessage::LoginResponse(LoginResponse::Success {
        greet: "hello".to_owned(),
        ip: Ipv4Addr::new(127, 0, 0, 1),
        hash: "abc123".to_owned(),
        is_supporter: true,
    });

    let decoded =
        ServerMessage::decode(success.encode().unwrap(), Direction::ServerToClient).unwrap();
    assert_eq!(decoded, success);

    let failure = ServerMessage::LoginResponse(LoginResponse::Failure {
        reason: "INVALIDUSERNAME".to_owned(),
        detail: Some("Nick empty.".to_owned()),
    });

    let decoded =
        ServerMessage::decode(failure.encode().unwrap(), Direction::ServerToClient).unwrap();
    assert_eq!(decoded, failure);
}

#[test]
fn peer_address_response_round_trips() {
    let message = ServerMessage::GetPeerAddressResponse(PeerAddress {
        username: "peer".to_owned(),
        ip: Ipv4Addr::new(10, 0, 0, 7),
        port: 2234,
        obfuscation_type: 1,
        obfuscated_port: 2235,
    });

    let decoded =
        ServerMessage::decode(message.encode().unwrap(), Direction::ServerToClient).unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn wait_port_supports_optional_obfuscated_port() {
    let message = ServerMessage::SetWaitPort(WaitPort {
        port: 2234,
        obfuscation: Some(ObfuscatedPort {
            kind: 1,
            port: 2235,
        }),
    });

    let decoded =
        ServerMessage::decode(message.encode().unwrap(), Direction::ClientToServer).unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn common_user_messages_round_trip() {
    let stats = UserStats {
        average_speed: 100,
        upload_count: 2,
        unknown: 0,
        file_count: 1000,
        directory_count: 50,
    };

    let messages = [
        ServerMessage::WatchUserRequest {
            username: "alice".to_owned(),
        },
        ServerMessage::GetUserStatusResponse(slskr_protocol::server::UserStatus {
            username: "alice".to_owned(),
            status: 2,
            privileged: true,
        }),
        ServerMessage::GetUserStats {
            username: "alice".to_owned(),
            stats,
        },
        ServerMessage::SayChatroomRequest {
            room: "room".to_owned(),
            message: "hello".to_owned(),
        },
        ServerMessage::ServerPing,
        ServerMessage::Relogged,
    ];

    for message in messages {
        let direction = match message {
            ServerMessage::WatchUserRequest { .. } | ServerMessage::SayChatroomRequest { .. } => {
                Direction::ClientToServer
            }
            _ => Direction::ServerToClient,
        };
        let decoded = ServerMessage::decode(message.encode().unwrap(), direction).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn room_list_round_trips() {
    let message = ServerMessage::RoomList(RoomList {
        public_rooms: vec![
            RoomListEntry {
                name: "public".to_owned(),
                user_count: 10,
            },
            RoomListEntry {
                name: "music".to_owned(),
                user_count: 20,
            },
        ],
        owned_private_rooms: vec![RoomListEntry {
            name: "owned".to_owned(),
            user_count: 2,
        }],
        private_rooms: vec![RoomListEntry {
            name: "private".to_owned(),
            user_count: 3,
        }],
        operated_private_rooms: vec!["operated".to_owned()],
    });

    let decoded =
        ServerMessage::decode(message.encode().unwrap(), Direction::ServerToClient).unwrap();
    assert_eq!(decoded, message);

    let request = ServerMessage::RoomListRequest;
    let decoded =
        ServerMessage::decode(request.encode().unwrap(), Direction::ClientToServer).unwrap();
    assert_eq!(decoded, request);
}

#[test]
fn connection_and_search_messages_round_trip() {
    let messages = [
        (
            ServerMessage::ConnectToPeerRequest(slskr_protocol::server::ConnectToPeerRequest {
                token: 42,
                username: "peer".to_owned(),
                connection_type: "P".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::ConnectToPeerResponse(slskr_protocol::server::ConnectToPeerResponse {
                username: "peer".to_owned(),
                connection_type: "P".to_owned(),
                ip: Ipv4Addr::new(192, 0, 2, 10),
                port: 2234,
                token: 42,
                privileged: false,
                obfuscation_type: 1,
                obfuscated_port: 2235,
            }),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::FileSearchRequest(slskr_protocol::server::SearchRequest {
                token: 100,
                query: "artist title".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::FileSearchIncoming {
                username: "peer".to_owned(),
                token: 100,
                query: "artist title".to_owned(),
            },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::UserSearch(TargetedSearchRequest {
                target: "peer".to_owned(),
                token: 101,
                query: "album".to_owned(),
            }),
            Direction::ClientToServer,
        ),
        (
            ServerMessage::JoinRoom {
                room: "room".to_owned(),
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
    ];

    for (message, direction) in messages {
        let decoded = ServerMessage::decode(message.encode().unwrap(), direction).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn distributed_and_filter_messages_round_trip() {
    let messages = [
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
            ServerMessage::AcceptChildren { accept: true },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::PossibleParents(vec![slskr_protocol::server::PossibleParent {
                username: "parent".to_owned(),
                ip: Ipv4Addr::new(203, 0, 113, 1),
                port: 2234,
            }]),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::BranchLevel { level: 2 },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::BranchRoot {
                username: "root".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (ServerMessage::ResetDistributed, Direction::ServerToClient),
        (
            ServerMessage::ExcludedSearchPhrases(vec!["bad phrase".to_owned()]),
            Direction::ServerToClient,
        ),
    ];

    for (message, direction) in messages {
        let decoded = ServerMessage::decode(message.encode().unwrap(), direction).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn private_message_and_error_messages_round_trip() {
    let messages = [
        (
            ServerMessage::MessageUserRequest {
                username: "peer".to_owned(),
                message: "hello".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::MessageUserResponse(slskr_protocol::server::PrivateMessage {
                id: 7,
                timestamp: 1_780_000_000,
                username: "peer".to_owned(),
                message: "hello".to_owned(),
                is_new: true,
            }),
            Direction::ServerToClient,
        ),
        (
            ServerMessage::MessageAcked { id: 7 },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::MessageUsers {
                usernames: vec!["alice".to_owned(), "bob".to_owned()],
                message: "hello group".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::CantConnectToPeerRequest {
                token: 42,
                username: "peer".to_owned(),
            },
            Direction::ClientToServer,
        ),
        (
            ServerMessage::CantConnectToPeerResponse { token: 42 },
            Direction::ServerToClient,
        ),
        (
            ServerMessage::CantCreateRoom {
                room: "room".to_owned(),
            },
            Direction::ServerToClient,
        ),
    ];

    for (message, direction) in messages {
        let decoded = ServerMessage::decode(message.encode().unwrap(), direction).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn possible_parents_rejects_untrusted_count_without_preallocating() {
    let frame = MessageFrame::new(ServerCode::PossibleParents.as_u32(), u32::MAX.to_le_bytes());
    let decoded = ServerMessage::decode(frame, Direction::ServerToClient);
    assert!(decoded.is_err());
}

#[test]
fn string_vec_rejects_untrusted_count_without_looping() {
    let frame = MessageFrame::new(ServerCode::PrivilegedUsers.as_u32(), u32::MAX.to_le_bytes());
    let decoded = ServerMessage::decode(frame, Direction::ServerToClient);
    assert!(decoded.is_err());
}

#[test]
fn unknown_server_codes_preserve_payload() {
    let frame = MessageFrame::new(4, [1, 2, 3]);

    let decoded = ServerMessage::decode(frame, Direction::ServerToClient).unwrap();
    assert_eq!(
        decoded,
        ServerMessage::Unknown {
            code: 4,
            payload: vec![1, 2, 3]
        }
    );
}
