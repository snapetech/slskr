use slskr_client::{
    peer_cache::MAX_PEER_USERNAME_BYTES,
    search::{
        InMemoryShareIndex, SearchDispatcher, SearchRequestHandle, SearchResponder, SearchResults,
        SearchTarget, ShareIndex, TimedSearchResults, WishlistSearchScheduler,
        WishlistSearchSchedulerOptions, MAX_OUTBOUND_SEARCH_FIELD_BYTES,
        MAX_SEARCH_RESPONSES_PER_TOKEN, MAX_SEARCH_RESPONSES_TOTAL,
        MAX_SEARCH_RESULT_FILES_PER_TOKEN, MAX_SEARCH_RESULT_FILES_TOTAL,
        MAX_SEARCH_RESULT_TEXT_BYTES_PER_TOKEN, MAX_TRACKED_SEARCH_RESULT_TOKENS,
        MAX_WISHLIST_SEARCH_TERMS, MAX_WISHLIST_SEARCH_TERM_BYTES,
    },
    server::ServerSession,
    stream::ServerConnection,
    ClientError,
};
use slskr_protocol::{
    distributed::DistributedSearch,
    peer::{FileEntry, FileSearchResponse, PeerMessage},
    server::{Direction, SearchRequest, ServerMessage, TargetedSearchRequest},
    ProtocolTextEncoding,
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

#[tokio::test]
async fn outbound_searches_reject_oversized_fields_before_consuming_a_token() {
    let (client, server) = duplex(512);
    let mut dispatcher =
        SearchDispatcher::new(ServerSession::new(ServerConnection::new(client)), 300);
    let oversized = "x".repeat(MAX_OUTBOUND_SEARCH_FIELD_BYTES + 1);

    for (result, expected_field) in [
        (dispatcher.search_global(&oversized).await, "query"),
        (
            dispatcher.search_user(&oversized, "query").await,
            "username",
        ),
        (dispatcher.search_user("alice", &oversized).await, "query"),
        (dispatcher.search_room(&oversized, "query").await, "room"),
        (dispatcher.search_room("room", &oversized).await, "query"),
        (dispatcher.search_wishlist(&oversized).await, "query"),
    ] {
        assert!(matches!(
            result,
            Err(ClientError::SearchFieldTooLong { field, length, max })
                if field == expected_field
                    && length == MAX_OUTBOUND_SEARCH_FIELD_BYTES + 1
                    && max == MAX_OUTBOUND_SEARCH_FIELD_BYTES
        ));
    }

    let handle = dispatcher.search_global("valid").await.unwrap();
    assert_eq!(handle.token, 300);
    let mut server = ServerConnection::new(server);
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::FileSearchRequest(SearchRequest {
            token: 300,
            query: "valid".to_owned(),
        })
    );
}

#[tokio::test]
async fn outbound_searches_reject_blank_fields_before_consuming_a_token() {
    let (client, server) = duplex(512);
    let mut dispatcher =
        SearchDispatcher::new(ServerSession::new(ServerConnection::new(client)), 400);

    for (result, expected_field) in [
        (dispatcher.search_global("  ").await, "query"),
        (dispatcher.search_user("\t", "query").await, "username"),
        (dispatcher.search_user("alice", "\n").await, "query"),
        (dispatcher.search_room(" ", "query").await, "room"),
        (dispatcher.search_room("room", "  ").await, "query"),
        (dispatcher.search_wishlist("").await, "query"),
    ] {
        assert!(matches!(
            result,
            Err(ClientError::BlankSearchField { field }) if field == expected_field
        ));
    }

    let handle = dispatcher.search_global(" valid ").await.unwrap();
    assert_eq!(handle.token, 400);
    assert_eq!(handle.query, "valid");
    let mut server = ServerConnection::new(server);
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::FileSearchRequest(SearchRequest {
            token: 400,
            query: "valid".to_owned(),
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
fn search_results_ignore_exact_response_replays() {
    let mut results = SearchResults::new();
    let mut original = response("alice", 10);
    original.results.push(entry("file.flac"));

    for _ in 0..3 {
        assert!(results
            .accept_peer_message(PeerMessage::FileSearchResponse(original.clone()))
            .unwrap());
    }

    assert_eq!(results.responses_for(10), &[original]);
    assert_eq!(results.len_for(10), 1);
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
fn search_results_bound_responses_and_files_per_token() {
    let mut results = SearchResults::new();
    let mut oversized = response("alice", 10);
    oversized.results = (0..(MAX_SEARCH_RESULT_FILES_PER_TOKEN + 5))
        .map(|index| entry(&format!("file-{index}")))
        .collect();
    results
        .accept_peer_message(PeerMessage::FileSearchResponse(oversized))
        .unwrap();
    assert_eq!(
        results.responses_for(10)[0].results.len(),
        MAX_SEARCH_RESULT_FILES_PER_TOKEN
    );

    for index in 1..(MAX_SEARCH_RESPONSES_PER_TOKEN + 5) {
        results
            .accept_peer_message(PeerMessage::FileSearchResponse(response(
                &format!("peer-{index}"),
                10,
            )))
            .unwrap();
    }
    assert_eq!(
        results.responses_for(10).len(),
        MAX_SEARCH_RESPONSES_PER_TOKEN
    );
}

#[test]
fn search_results_deduplicate_replayed_responses_after_per_response_truncation() {
    let mut results = SearchResults::new();
    let mut oversized = response("alice", 10);
    oversized.results = (0..(MAX_SEARCH_RESULT_FILES_PER_TOKEN + 5))
        .map(|index| entry(&format!("file-{index}")))
        .collect();

    for _ in 0..3 {
        results
            .accept_peer_message(PeerMessage::FileSearchResponse(oversized.clone()))
            .unwrap();
    }

    assert_eq!(results.len_for(10), 1);
    assert_eq!(results.stored_responses_len(), 1);
}

#[test]
fn search_results_bound_attacker_controlled_token_keys() {
    let mut results = SearchResults::new();
    for token in 0..u32::try_from(MAX_TRACKED_SEARCH_RESULT_TOKENS).unwrap() {
        results
            .accept_peer_message(PeerMessage::FileSearchResponse(response("peer", token)))
            .unwrap();
    }

    assert_eq!(
        results.tracked_tokens_len(),
        MAX_TRACKED_SEARCH_RESULT_TOKENS
    );
    assert!(results
        .accept_peer_message(PeerMessage::FileSearchResponse(response(
            "ignored",
            u32::MAX,
        )))
        .unwrap());
    assert_eq!(
        results.tracked_tokens_len(),
        MAX_TRACKED_SEARCH_RESULT_TOKENS
    );
    assert!(results.responses_for(u32::MAX).is_empty());

    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response("second", 0)))
        .unwrap();
    assert_eq!(results.len_for(0), 2);
}

#[test]
fn search_results_bound_retained_text_bytes_per_token() {
    let mut results = SearchResults::new();
    let oversized = FileSearchResponse {
        username: "x".repeat(MAX_SEARCH_RESULT_TEXT_BYTES_PER_TOKEN + 1),
        ..response("peer", 10)
    };

    assert!(results
        .accept_peer_message(PeerMessage::FileSearchResponse(oversized))
        .unwrap());
    assert!(results.responses_for(10).is_empty());
    assert_eq!(results.tracked_tokens_len(), 0);

    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response("peer", 10)))
        .unwrap();
    assert_eq!(results.len_for(10), 1);
    assert_eq!(results.take(10).len(), 1);
    assert!(results.responses_for(10).is_empty());
}

#[test]
fn search_results_bound_aggregate_files_across_tokens_and_release_budget() {
    let mut results = SearchResults::new();
    for token in 0..u32::try_from(MAX_SEARCH_RESULT_FILES_TOTAL / 1_000).unwrap() {
        let mut batch = response("peer", token);
        batch.results = (0..1_000)
            .map(|index| entry(&format!("file-{token}-{index}")))
            .collect();
        results
            .accept_peer_message(PeerMessage::FileSearchResponse(batch))
            .unwrap();
    }
    assert_eq!(results.stored_files_len(), MAX_SEARCH_RESULT_FILES_TOTAL);

    let mut rejected = response("peer", u32::MAX);
    rejected.results.push(entry("over-budget"));
    results
        .accept_peer_message(PeerMessage::FileSearchResponse(rejected))
        .unwrap();
    assert_eq!(results.stored_files_len(), MAX_SEARCH_RESULT_FILES_TOTAL);
    assert_eq!(results.len_for(u32::MAX), 1);
    assert!(results.responses_for(u32::MAX)[0].results.is_empty());

    let _ = results.take(0);
    assert_eq!(
        results.stored_files_len(),
        MAX_SEARCH_RESULT_FILES_TOTAL - 1_000
    );
}

#[test]
fn search_results_bound_aggregate_responses_and_release_budget() {
    let mut results = SearchResults::new();
    for index in 0..MAX_SEARCH_RESPONSES_TOTAL {
        let token = u32::try_from(index / 100).unwrap();
        results
            .accept_peer_message(PeerMessage::FileSearchResponse(response(
                &format!("peer-{index}"),
                token,
            )))
            .unwrap();
    }
    assert_eq!(results.stored_responses_len(), MAX_SEARCH_RESPONSES_TOTAL);

    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response(
            "rejected",
            u32::MAX,
        )))
        .unwrap();
    assert_eq!(results.stored_responses_len(), MAX_SEARCH_RESPONSES_TOTAL);
    assert!(results.responses_for(u32::MAX).is_empty());

    assert_eq!(results.take(0).len(), 100);
    assert_eq!(
        results.stored_responses_len(),
        MAX_SEARCH_RESPONSES_TOTAL - 100
    );
}

#[test]
fn wishlist_scheduler_uses_server_interval_with_positive_guards() {
    assert!(matches!(
        WishlistSearchSchedulerOptions::new(Duration::ZERO, None),
        Err(ClientError::InvalidInterval {
            field: "minimum_interval"
        })
    ));
    assert!(matches!(
        WishlistSearchSchedulerOptions::new(Duration::from_secs(1), Some(Duration::ZERO)),
        Err(ClientError::InvalidInterval {
            field: "override_interval"
        })
    ));

    let options = WishlistSearchSchedulerOptions::new(Duration::from_secs(30), None).unwrap();
    let mut scheduler = WishlistSearchScheduler::new(
        [" rare ".to_owned(), "".to_owned(), "mix".to_owned()],
        options,
    )
    .unwrap();

    assert_eq!(scheduler.interval(), Duration::from_secs(30));
    assert!(scheduler.apply_server_message(&ServerMessage::WishlistInterval { seconds: 120 }));
    assert_eq!(scheduler.interval(), Duration::from_secs(120));
    assert!(!scheduler.apply_server_message(&ServerMessage::ServerPing));
    assert_eq!(
        scheduler.next_search_message(7),
        Some(ServerMessage::WishlistSearch(SearchRequest {
            token: 7,
            query: "rare".to_owned(),
        }))
    );
    assert_eq!(
        scheduler.next_search_message(8),
        Some(ServerMessage::WishlistSearch(SearchRequest {
            token: 8,
            query: "mix".to_owned(),
        }))
    );
}

#[test]
fn wishlist_scheduler_override_wins_but_respects_minimum() {
    let options =
        WishlistSearchSchedulerOptions::new(Duration::from_secs(30), Some(Duration::from_secs(5)))
            .unwrap();
    let mut scheduler = WishlistSearchScheduler::new(["rare".to_owned()], options).unwrap();

    scheduler.apply_server_message(&ServerMessage::WishlistInterval { seconds: 120 });
    assert_eq!(scheduler.interval(), Duration::from_secs(30));
}

#[test]
fn wishlist_scheduler_replaces_terms_without_losing_server_interval() {
    let options = WishlistSearchSchedulerOptions::new(Duration::from_secs(30), None).unwrap();
    let mut scheduler =
        WishlistSearchScheduler::new(["first".to_owned(), "second".to_owned()], options).unwrap();
    scheduler.apply_server_message(&ServerMessage::WishlistInterval { seconds: 120 });

    assert_eq!(
        scheduler.next_search_message(1),
        Some(ServerMessage::WishlistSearch(SearchRequest {
            token: 1,
            query: "first".to_owned(),
        }))
    );

    scheduler.replace_terms(["third".to_owned()]);
    assert_eq!(scheduler.interval(), Duration::from_secs(120));
    assert_eq!(
        scheduler.next_search_message(2),
        Some(ServerMessage::WishlistSearch(SearchRequest {
            token: 2,
            query: "third".to_owned(),
        }))
    );
}

#[test]
fn wishlist_scheduler_bounds_deduplicates_and_truncates_terms() {
    let options = WishlistSearchSchedulerOptions::new(Duration::from_secs(30), None).unwrap();
    let oversized = "é".repeat(MAX_WISHLIST_SEARCH_TERM_BYTES);
    let terms = std::iter::once(oversized)
        .chain([" Rare ".to_owned(), "rare".to_owned()])
        .chain((0..MAX_WISHLIST_SEARCH_TERMS + 5).map(|index| format!("term-{index}")));
    let mut scheduler = WishlistSearchScheduler::new(terms, options).unwrap();

    let ServerMessage::WishlistSearch(first) = scheduler.next_search_message(1).unwrap() else {
        panic!("expected wishlist search");
    };
    assert!(first.query.len() <= MAX_WISHLIST_SEARCH_TERM_BYTES);
    let mut emitted = vec![first.query];
    for token in 2..=u32::try_from(MAX_WISHLIST_SEARCH_TERMS).unwrap() {
        let ServerMessage::WishlistSearch(search) = scheduler.next_search_message(token).unwrap()
        else {
            panic!("expected wishlist search");
        };
        emitted.push(search.query);
    }

    assert_eq!(emitted.len(), MAX_WISHLIST_SEARCH_TERMS);
    assert_eq!(
        emitted
            .iter()
            .filter(|term| term.eq_ignore_ascii_case("rare"))
            .count(),
        1
    );
    assert_eq!(
        scheduler.next_search_message(u32::MAX),
        Some(ServerMessage::WishlistSearch(SearchRequest {
            token: u32::MAX,
            query: emitted[0].clone(),
        }))
    );
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
fn timed_search_token_reuse_discards_stale_responses() {
    let now = Instant::now();
    let mut results = TimedSearchResults::new(Duration::from_secs(5));
    let original = handle(10);
    results.track(original.clone(), now);
    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response("alice", 10)))
        .unwrap();
    assert_eq!(results.len_for(10), 1);

    let mut replacement = handle(10);
    replacement.query = "replacement query".to_owned();
    assert_eq!(
        results.track(replacement.clone(), now + Duration::from_secs(1)),
        Some(slskr_client::search::TimedSearch {
            handle: original,
            created_at: now,
            expires_at: now + Duration::from_secs(5),
        })
    );
    assert!(results.responses_for(10).is_empty());

    let (search, responses) = results.finish(10).unwrap();
    assert_eq!(search.handle, replacement);
    assert!(responses.is_empty());
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
fn timed_search_window_overflow_saturates_instead_of_panicking() {
    let now = Instant::now();
    let mut results = TimedSearchResults::new(Duration::MAX);

    assert!(results.track(handle(10), now).is_none());
    assert!(results.is_active(10));
    assert!(results.drain_expired(now).is_empty());
}

#[test]
fn timed_search_tracking_evicts_oldest_state_at_token_limit() {
    let now = Instant::now();
    let mut results = TimedSearchResults::new(Duration::from_secs(60));
    results.track(handle(0), now);
    results
        .accept_peer_message(PeerMessage::FileSearchResponse(response("alice", 0)))
        .unwrap();

    for token in 1..=u32::try_from(MAX_TRACKED_SEARCH_RESULT_TOKENS).unwrap() {
        results.track(handle(token), now + Duration::from_nanos(u64::from(token)));
    }

    assert_eq!(results.active_len(), MAX_TRACKED_SEARCH_RESULT_TOKENS);
    assert!(!results.is_active(0));
    assert!(results.responses_for(0).is_empty());
    assert!(results.is_active(u32::try_from(MAX_TRACKED_SEARCH_RESULT_TOKENS).unwrap()));
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
    .unwrap()
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
    )
    .unwrap();

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
    )
    .unwrap();

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
    .unwrap()
    .with_excluded_phrases(vec!["rare".to_owned()]);

    assert!(responder
        .respond_to_server_search(&ServerMessage::FileSearchIncoming {
            username: "remote".to_owned(),
            token: 55,
            query: "rare track".to_owned(),
        })
        .is_none());
}

#[test]
fn responder_bounds_files_in_a_single_search_response() {
    let entries = (0..(MAX_SEARCH_RESULT_FILES_PER_TOKEN + 1))
        .map(|index| entry(&format!("Music/match-{index}.flac")))
        .collect();
    let responder = SearchResponder::new("local", InMemoryShareIndex::new(entries)).unwrap();

    let message = responder
        .respond_to_server_search(&ServerMessage::FileSearchIncoming {
            username: "remote".to_owned(),
            token: 55,
            query: "match".to_owned(),
        })
        .unwrap();

    let PeerMessage::FileSearchResponse(response) = message else {
        panic!("expected search response");
    };
    assert_eq!(response.results.len(), MAX_SEARCH_RESULT_FILES_PER_TOKEN);

    let encoded = PeerMessage::FileSearchResponse(response).encode().unwrap();
    assert!(matches!(
        PeerMessage::decode(encoded).unwrap(),
        PeerMessage::FileSearchResponse(_)
    ));
}

#[test]
fn responder_rejects_malformed_local_identity() {
    assert!(matches!(
        SearchResponder::new("   ", InMemoryShareIndex::new(Vec::new())).unwrap_err(),
        ClientError::BlankPeerUsername
    ));
    assert!(matches!(
        SearchResponder::new(
            "x".repeat(MAX_PEER_USERNAME_BYTES + 1),
            InMemoryShareIndex::new(Vec::new()),
        )
        .unwrap_err(),
        ClientError::PeerUsernameTooLong { length, max }
            if length == MAX_PEER_USERNAME_BYTES + 1 && max == MAX_PEER_USERNAME_BYTES
    ));

    let responder =
        SearchResponder::new(" local ", InMemoryShareIndex::new(vec![entry("x")])).unwrap();
    let PeerMessage::FileSearchResponse(response) = responder
        .respond_to_distributed_search(&DistributedSearch {
            identifier: 1,
            username: "remote".to_owned(),
            token: 2,
            query: "x".to_owned(),
        })
        .unwrap()
    else {
        panic!("expected search response");
    };
    assert_eq!(response.username, "local");
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
        filename_encoding: ProtocolTextEncoding::Utf8,
        size: 100,
        extension: String::new(),
        extension_encoding: ProtocolTextEncoding::Utf8,
        attributes: Vec::new(),
    }
}
