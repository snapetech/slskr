use slskr_client::{
    search::{
        InMemoryShareIndex, SearchDispatcher, SearchRequestHandle, SearchResponder, SearchResults,
        SearchTarget, ShareIndex, TimedSearchResults,
    },
    server::ServerSession,
    stream::ServerConnection,
    ClientError,
};
use slskr_protocol::{
    distributed::DistributedSearch,
    peer::{FileEntry, FileSearchResponse, PeerMessage},
    server::{Direction, SearchRequest, ServerMessage, TargetedSearchRequest},
};
use std::time::{Duration, Instant};
use tokio::io::duplex;

#[tokio::test]
async fn dispatches_global_search_with_token() {
    let (client, server) = duplex(512);
    let mut dispatcher =
        SearchDispatcher::new(ServerSession::new(ServerConnection::new(client)), 100);
    let mut server = ServerConnection::new(server);

    let handle = dispatcher.search_global("needle").await.unwrap();

    assert_eq!(handle.token, 100);
    assert_eq!(handle.query, "needle");
    assert_eq!(handle.target, SearchTarget::Global);
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::FileSearchRequest(SearchRequest {
            token: 100,
            query: "needle".to_owned(),
        })
    );
}

#[tokio::test]
async fn dispatches_targeted_searches_with_incrementing_tokens() {
    let (client, server) = duplex(1024);
    let mut dispatcher =
        SearchDispatcher::new(ServerSession::new(ServerConnection::new(client)), 200);
    let mut server = ServerConnection::new(server);

    let user = dispatcher.search_user("peer", "album").await.unwrap();
    let room = dispatcher.search_room("room", "mix").await.unwrap();
    let wishlist = dispatcher.search_wishlist("rare").await.unwrap();

    assert_eq!(user.token, 200);
    assert_eq!(room.token, 201);
    assert_eq!(wishlist.token, 202);
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::UserSearch(TargetedSearchRequest {
            target: "peer".to_owned(),
            token: 200,
            query: "album".to_owned(),
        })
    );
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::RoomSearch(TargetedSearchRequest {
            target: "room".to_owned(),
            token: 201,
            query: "mix".to_owned(),
        })
    );
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::WishlistSearch(SearchRequest {
            token: 202,
            query: "rare".to_owned(),
        })
    );
}

#[test]
fn search_results_collect_by_token() {
    let mut results = SearchResults::new();
    let first = response("alice", 10);
    let second = response("bob", 10);
    let other = response("carol", 11);

    assert!(results
        .accept_peer_message(PeerMessage::FileSearchResponse(first.clone()))
        .unwrap());
    assert!(results
        .accept_peer_message(PeerMessage::FileSearchResponse(second.clone()))
        .unwrap());
    assert!(results
        .accept_peer_message(PeerMessage::FileSearchResponse(other.clone()))
        .unwrap());

    assert_eq!(results.responses_for(10), &[first, second]);
    assert_eq!(results.responses_for(11), &[other]);
    assert_eq!(results.len_for(10), 2);
}

#[test]
fn search_results_take_removes_token() {
    let mut results = SearchResults::new();
    let response = response("alice", 10);
    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response.clone()))
        .unwrap();

    assert_eq!(results.take(10), vec![response]);
    assert!(results.responses_for(10).is_empty());
}

#[test]
fn search_results_reject_non_search_message() {
    let mut results = SearchResults::new();

    let error = results
        .accept_peer_message(PeerMessage::QueueUpload {
            filename: "file.flac".to_owned(),
        })
        .unwrap_err();

    assert!(matches!(error, ClientError::UnexpectedSearchMessage(_)));
}

#[test]
fn timed_search_results_track_active_windows_and_accept_matching_tokens() {
    let now = Instant::now();
    let mut results = TimedSearchResults::new(Duration::from_secs(5));
    results.track(handle(10), now);

    assert!(results.is_active(10));
    assert_eq!(results.active_len(), 1);
    assert!(results
        .accept_peer_message(PeerMessage::FileSearchResponse(response("alice", 10)))
        .unwrap());
    assert!(!results
        .accept_peer_message(PeerMessage::FileSearchResponse(response("bob", 11)))
        .unwrap());
    assert_eq!(results.len_for(10), 1);
    assert!(results.responses_for(11).is_empty());
}

#[test]
fn timed_search_results_finish_returns_responses() {
    let now = Instant::now();
    let mut results = TimedSearchResults::new(Duration::from_secs(5));
    results.track(handle(10), now);
    let response = response("alice", 10);
    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response.clone()))
        .unwrap();

    let (search, responses) = results.finish(10).unwrap();

    assert_eq!(search.handle.token, 10);
    assert_eq!(responses, vec![response]);
    assert!(!results.is_active(10));
}

#[test]
fn timed_search_results_drain_expired() {
    let now = Instant::now();
    let mut results = TimedSearchResults::new(Duration::from_secs(5));
    results.track(handle(10), now);
    results.track(handle(11), now + Duration::from_secs(3));
    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response("alice", 10)))
        .unwrap();

    let expired = results.drain_expired(now + Duration::from_secs(5));

    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].0.handle.token, 10);
    assert_eq!(expired[0].1, vec![response("alice", 10)]);
    assert!(!results.is_active(10));
    assert!(results.is_active(11));
}

#[test]
fn in_memory_share_index_matches_all_terms_case_insensitively() {
    let index = InMemoryShareIndex::new(vec![
        entry("Music/Artist - Rare Track.flac"),
        entry("Music/Artist - Common Track.flac"),
    ]);

    assert_eq!(
        index.search("artist rare"),
        vec![entry("Music/Artist - Rare Track.flac")]
    );
    assert!(index.search("missing").is_empty());
}

#[test]
fn responder_builds_file_search_response_for_server_search() {
    let responder = SearchResponder::new(
        "local",
        InMemoryShareIndex::new(vec![entry("Music/Artist - Rare Track.flac")]),
    )
    .with_stats(1000, 2, 0);

    let message = responder
        .respond_to_server_search(&ServerMessage::FileSearchIncoming {
            username: "remote".to_owned(),
            token: 55,
            query: "rare".to_owned(),
        })
        .unwrap();

    let PeerMessage::FileSearchResponse(response) = message else {
        panic!("expected search response");
    };
    assert_eq!(response.username, "local");
    assert_eq!(response.token, 55);
    assert_eq!(
        response.results,
        vec![entry("Music/Artist - Rare Track.flac")]
    );
    assert_eq!(response.average_speed, 1000);
    assert_eq!(response.queue_length, 2);
}

#[test]
fn responder_builds_file_search_response_for_distributed_search() {
    let responder = SearchResponder::new(
        "local",
        InMemoryShareIndex::new(vec![entry("Music/Artist - Rare Track.flac")]),
    );

    let message = responder
        .respond_to_distributed_search(&DistributedSearch {
            identifier: 49,
            username: "remote".to_owned(),
            token: 56,
            query: "rare".to_owned(),
        })
        .unwrap();

    let PeerMessage::FileSearchResponse(response) = message else {
        panic!("expected search response");
    };
    assert_eq!(response.token, 56);
    assert_eq!(
        response.results,
        vec![entry("Music/Artist - Rare Track.flac")]
    );
}

#[test]
fn responder_returns_none_without_matches() {
    let responder = SearchResponder::new(
        "local",
        InMemoryShareIndex::new(vec![entry("Music/Artist - Rare Track.flac")]),
    );

    assert!(responder
        .respond_to_server_search(&ServerMessage::FileSearchIncoming {
            username: "remote".to_owned(),
            token: 55,
            query: "missing".to_owned(),
        })
        .is_none());
}

#[test]
fn responder_suppresses_excluded_search_phrases() {
    let responder = SearchResponder::new(
        "local",
        InMemoryShareIndex::new(vec![entry("Music/Artist - Rare Track.flac")]),
    )
    .with_excluded_phrases(vec!["rare".to_owned()]);

    assert!(responder
        .respond_to_server_search(&ServerMessage::FileSearchIncoming {
            username: "remote".to_owned(),
            token: 55,
            query: "rare track".to_owned(),
        })
        .is_none());
}

fn response(username: &str, token: u32) -> FileSearchResponse {
    FileSearchResponse {
        username: username.to_owned(),
        token,
        results: Vec::new(),
        slot_free: true,
        average_speed: 100,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    }
}

fn handle(token: u32) -> SearchRequestHandle {
    SearchRequestHandle {
        token,
        query: "needle".to_owned(),
        target: SearchTarget::Global,
    }
}

fn entry(filename: &str) -> FileEntry {
    FileEntry {
        code: 1,
        filename: filename.to_owned(),
        size: 100,
        extension: String::new(),
        attributes: Vec::new(),
    }
}
