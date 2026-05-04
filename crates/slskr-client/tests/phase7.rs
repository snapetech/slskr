use slskr_client::{
    events::{trace_distributed_message, trace_peer_message, trace_server_message},
    filters::ExcludedPhraseFilter,
    share_payload::{
        compress_zlib_payload, decompress_peer_share_payload, decompress_zlib_payload,
    },
    social::{PrivateMessageInbox, RoomState, UserWatchState},
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
fn tracing_hooks_are_noop_without_a_subscriber() {
    trace_server_message("in", &ServerMessage::ServerPing);
    trace_peer_message("alice", "out", &PeerMessage::GetShareFileList);
    trace_distributed_message("parent", "in", &DistributedMessage::Ping);
}
