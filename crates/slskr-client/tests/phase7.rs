use slskr_client::{
    events::{trace_distributed_message, trace_peer_message, trace_server_message},
    filters::ExcludedPhraseFilter,
    share_payload::{
        compress_zlib_payload, decompress_peer_share_payload, decompress_zlib_payload,
        decompress_zlib_payload_with_limit,
    },
    social::{
        private_message_users_command, PrivateMessageInbox, RoomState, UserWatchState,
        MAX_PRIVATE_MESSAGE_RECIPIENTS, MAX_STORED_PRIVATE_MESSAGES, MAX_STORED_ROOM_MESSAGES,
    },
    ClientError,
};
use slskr_protocol::{
    distributed::DistributedMessage,
    peer::PeerMessage,
    server::{PrivateMessage, ServerMessage, UserStats, UserStatus, WatchedUser},
};

#[test]
fn zlib_share_payload_helpers_round_trip_peer_payloads() {
    let raw = b"encoded share tree bytes";
    let compressed = compress_zlib_payload(raw).unwrap();

    assert_ne!(compressed, raw);
    assert_eq!(decompress_zlib_payload(&compressed).unwrap(), raw);
    assert_eq!(
        decompress_peer_share_payload(&PeerMessage::SharedFileListResponse(compressed.clone()))
            .unwrap()
            .unwrap(),
        raw
    );
    assert_eq!(
        decompress_peer_share_payload(&PeerMessage::FolderContentsResponse(compressed))
            .unwrap()
            .unwrap(),
        raw
    );
    assert!(decompress_peer_share_payload(&PeerMessage::GetShareFileList).is_none());
}

#[test]
fn zlib_share_payload_decompression_is_bounded() {
    let raw = vec![b'x'; 1024];
    let compressed = compress_zlib_payload(&raw).unwrap();
    let error = decompress_zlib_payload_with_limit(&compressed, 128).unwrap_err();
    assert!(matches!(error, ClientError::PayloadTooLarge { max: 128 }));
}

#[test]
fn excluded_phrase_filter_tracks_server_updates_and_matches_case_insensitively() {
    let filter =
        ExcludedPhraseFilter::from_server_message(&ServerMessage::ExcludedSearchPhrases(vec![
            "bad phrase".to_owned(),
            "  ".to_owned(),
            "Leak".to_owned(),
        ]))
        .unwrap();

    assert_eq!(filter.phrases(), &["bad phrase", "leak"]);
    assert!(!filter.allows_query("contains BAD PHRASE now"));
    assert!(!filter.allows_query("album leak"));
    assert!(filter.allows_query("public domain album"));
    assert!(ExcludedPhraseFilter::from_server_message(&ServerMessage::ServerPing).is_none());
}

#[test]
fn user_watch_state_builds_requests_and_applies_watch_and_status_updates() {
    let mut state = UserWatchState::new();

    assert_eq!(
        UserWatchState::watch_message("alice"),
        ServerMessage::WatchUserRequest {
            username: "alice".to_owned(),
        }
    );
    assert_eq!(
        UserWatchState::unwatch_message("alice"),
        ServerMessage::UnwatchUser {
            username: "alice".to_owned(),
        }
    );

    assert!(
        state.apply_server_message(&ServerMessage::WatchUserResponse(WatchedUser {
            username: "alice".to_owned(),
            exists: true,
            status: Some(2),
            stats: Some(UserStats {
                average_speed: 10,
                upload_count: 1,
                unknown: 0,
                file_count: 100,
                directory_count: 3,
            }),
            country_code: Some("US".to_owned()),
        }))
    );
    assert!(
        state.apply_server_message(&ServerMessage::GetUserStatusResponse(UserStatus {
            username: "alice".to_owned(),
            status: 1,
            privileged: true,
        }))
    );

    assert_eq!(state.watched("alice").unwrap().status, Some(2));
    assert_eq!(state.status("alice").unwrap().status, 1);
    assert!(state.apply_server_message(&ServerMessage::UnwatchUser {
        username: "alice".to_owned(),
    }));
    assert!(state.watched("alice").is_none());
    assert!(state.status("alice").is_none());
}

#[test]
fn room_state_tracks_global_messages_and_leave_requests() {
    let mut state = RoomState::new();

    assert_eq!(
        RoomState::join_global_message(),
        ServerMessage::JoinGlobalRoom
    );
    assert_eq!(
        RoomState::leave_global_message(),
        ServerMessage::LeaveGlobalRoom
    );
    assert_eq!(
        RoomState::leave_room_message("lobby"),
        ServerMessage::LeaveRoom {
            room: "lobby".to_owned(),
        }
    );

    assert!(
        state.apply_server_message(&ServerMessage::GlobalRoomMessage {
            room: "lobby".to_owned(),
            username: "alice".to_owned(),
            message: "hello".to_owned(),
        })
    );
    assert!(state.is_joined("lobby"));
    assert_eq!(state.messages()[0].message, "hello");

    assert!(state.apply_server_message(&ServerMessage::LeaveRoom {
        room: "lobby".to_owned(),
    }));
    assert!(!state.is_joined("lobby"));
}

#[test]
fn private_message_inbox_returns_ack_messages() {
    let mut inbox = PrivateMessageInbox::new();

    let ack = inbox
        .apply_server_message(&ServerMessage::MessageUserResponse(PrivateMessage {
            id: 42,
            timestamp: 123,
            username: "alice".to_owned(),
            message: "hi".to_owned(),
            is_new: true,
        }))
        .unwrap();

    assert_eq!(ack, ServerMessage::MessageAcked { id: 42 });
    assert_eq!(inbox.messages()[0].message, "hi");
    assert!(inbox
        .apply_server_message(&ServerMessage::ServerPing)
        .is_none());
}

#[test]
fn social_message_histories_evict_oldest_entries_at_limits() {
    let mut rooms = RoomState::new();
    for index in 0..(MAX_STORED_ROOM_MESSAGES + 5) {
        assert!(
            rooms.apply_server_message(&ServerMessage::GlobalRoomMessage {
                room: "lobby".to_owned(),
                username: "alice".to_owned(),
                message: format!("room-{index}"),
            })
        );
    }
    assert_eq!(rooms.messages().len(), MAX_STORED_ROOM_MESSAGES);
    assert_eq!(rooms.messages().first().unwrap().message, "room-5");

    let mut inbox = PrivateMessageInbox::new();
    for index in 0..(MAX_STORED_PRIVATE_MESSAGES + 5) {
        let ack = inbox.apply_server_message(&ServerMessage::MessageUserResponse(PrivateMessage {
            id: index as u32,
            timestamp: 123,
            username: "alice".to_owned(),
            message: format!("private-{index}"),
            is_new: true,
        }));
        assert_eq!(ack, Some(ServerMessage::MessageAcked { id: index as u32 }));
    }
    assert_eq!(inbox.messages().len(), MAX_STORED_PRIVATE_MESSAGES);
    assert_eq!(inbox.messages().first().unwrap().message, "private-5");
}

#[test]
fn multi_user_private_message_command_dedupes_and_validates_recipients() {
    let command = private_message_users_command(["Alice", "alice", " Bob "], "hello").unwrap();

    assert_eq!(
        command,
        ServerMessage::MessageUsers {
            usernames: vec!["Alice".to_owned(), "Bob".to_owned()],
            message: "hello".to_owned(),
        }
    );

    assert!(matches!(
        private_message_users_command(Vec::<String>::new(), "hello"),
        Err(ClientError::EmptyMessageRecipients)
    ));
    assert!(matches!(
        private_message_users_command(["alice", " "], "hello"),
        Err(ClientError::BlankMessageRecipient)
    ));

    let too_many = (0..=MAX_PRIVATE_MESSAGE_RECIPIENTS)
        .map(|index| format!("user-{index}"))
        .collect::<Vec<_>>();
    assert!(matches!(
        private_message_users_command(too_many, "hello"),
        Err(ClientError::TooManyMessageRecipients { count, max })
            if count == MAX_PRIVATE_MESSAGE_RECIPIENTS + 1
                && max == MAX_PRIVATE_MESSAGE_RECIPIENTS
    ));
}

#[test]
fn tracing_hooks_are_noop_without_a_subscriber() {
    trace_server_message("in", &ServerMessage::ServerPing);
    trace_peer_message("alice", "out", &PeerMessage::GetShareFileList);
    trace_distributed_message("parent", "in", &DistributedMessage::Ping);
}
