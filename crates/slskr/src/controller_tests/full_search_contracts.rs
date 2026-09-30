//! Controller full search contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn incoming_search_filters_honor_startup_case_mode() {
    let (insensitive, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKD_SEARCH_REQUEST_FILTER", "secret"),
    );
    assert!(crate::build_file_search_response(&insensitive, 1, "SECRET")
        .await
        .is_none());

    let (sensitive, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKD_SEARCH_REQUEST_FILTER", "secret")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    );
    assert!(crate::build_file_search_response(&sensitive, 1, "SECRET")
        .await
        .is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn incoming_public_search_sends_response_over_peer_wire() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind incoming-search peer fixture");
    let peer_address = listener
        .local_addr()
        .expect("incoming-search peer fixture address");
    let peer_task = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept incoming-search peer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("incoming-search peer init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        let response = peer.receive().await.expect("incoming-search peer response");
        let crate::PeerMessage::FileSearchResponse(response) = response else {
            panic!("incoming-search peer received the wrong message");
        };
        assert_eq!(response.username, "tester");
        assert_eq!(response.token, 19);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].filename, "Virtual/SECRET.flac");
    });

    let endpoint = format!("searcher={peer_address}");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", &endpoint),
    );

    let session_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind incoming-search session fixture");
    let session_address = session_listener
        .local_addr()
        .expect("incoming-search session fixture address");
    let session_client = tokio::net::TcpStream::connect(session_address);
    let session_server = session_listener.accept();
    let (session_client, session_server) = tokio::join!(session_client, session_server);
    let _session_client = session_client.expect("connect incoming-search session fixture");
    let (session_server, _) = session_server.expect("accept incoming-search session fixture");
    let mut session = slskr_client::server::ServerSession::new(
        slskr_client::stream::ServerConnection::new(session_server),
    );

    crate::session_runtime::project_server_message(
        &state,
        &mut session,
        &crate::ServerMessage::FileSearchIncoming {
            username: "searcher".to_owned(),
            token: 19,
            query: "SECRET".to_owned(),
        },
    )
    .await;

    tokio::time::timeout(Duration::from_secs(2), peer_task)
        .await
        .expect("incoming-search peer response timed out")
        .expect("incoming-search peer task failed");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn raw_peer_search_request_responds_without_peer_username() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind raw-search peer fixture");
    let peer_address = listener
        .local_addr()
        .expect("raw-search peer fixture address");
    let peer_task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept raw-search peer");
        tokio::io::AsyncWriteExt::write_u8(&mut stream, b'P')
            .await
            .expect("write raw peer kind");
        let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
        peer.send(&crate::PeerMessage::FileSearchRequest {
            token: 23,
            query: "SECRET".to_owned(),
        })
        .await
        .expect("send raw-search request");
        let response = tokio::time::timeout(Duration::from_secs(2), peer.receive())
            .await
            .expect("raw-search response timed out")
            .expect("raw-search response failed");
        let crate::PeerMessage::FileSearchResponse(response) = response else {
            panic!("raw-search peer received the wrong message");
        };
        assert_eq!(response.token, 23);
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].filename, "Virtual/SECRET.flac");
    });

    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42"),
    );
    let stream = tokio::net::TcpStream::connect(peer_address)
        .await
        .expect("connect raw-search peer fixture");
    let remote_address = stream
        .peer_addr()
        .expect("raw-search peer fixture remote address");
    let incoming = slskr_client::listener::demux_incoming(stream)
        .await
        .expect("demux raw-search peer");
    let crate::IncomingConnection::PeerMessages(peer) = incoming else {
        panic!("expected raw peer-message connection");
    };
    crate::handle_plain_peer_messages_with_address(&state, peer, None, Some(remote_address.ip()))
        .await
        .expect("handle raw-search peer");
    peer_task.await.expect("raw-search peer task failed");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_search_filters_preserve_reloaded_case_mode() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_SHARE_FIXTURE", "Virtual/SECRET.flac=42")
            .with("SLSKD_SEARCH_REQUEST_FILTER", "secret")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    );
    assert!(state.config.controller_case_sensitive_regex);
    assert!(crate::build_file_search_response(&state, 1, "SECRET")
        .await
        .is_some());

    let yaml = "flags:\n  case_sensitive_reg_ex: false\nfilters:\n  search:\n    request:\n      - secret|other\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
        ("SLSKR_CONTROLLER_PROFILE".to_owned(), "slskdn".to_owned()),
    ]);
    crate::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    assert!(crate::build_file_search_response(&state, 1, "SECRET")
        .await
        .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn search_store_resolves_dash_stripped_controller_ids() {
    let canonical_id = "11111111-1111-4111-8111-111111111111";
    let compatibility_id = canonical_id.replace('-', "");
    let mut searches = crate::SearchStore::new();
    let record = searches
        .create(
            Some(canonical_id.to_owned()),
            "compatibility id".to_owned(),
            "global",
            None,
            Vec::new(),
            300,
        )
        .expect("create search")
        .record;

    assert_eq!(
        searches
            .get_by_identifier(&compatibility_id)
            .map(|search| search.id),
        Some(canonical_id.to_owned())
    );
    assert!(searches
        .update_by_identifier(&compatibility_id, Some("updated".to_owned()), None)
        .is_some());
    assert_eq!(
        searches
            .get_by_identifier(canonical_id)
            .map(|search| search.query),
        Some("updated".to_owned())
    );
    assert!(searches.remove_by_identifier(&compatibility_id).is_some());
    assert!(searches.records.is_empty());
    assert!(record.id.contains('-'));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn search_response_availability_tracks_durable_payloads_and_completion() {
    let mut searches = crate::SearchStore::new();
    let created = searches
        .create(
            None,
            "empty active".to_owned(),
            "global",
            None,
            Vec::new(),
            60,
        )
        .unwrap()
        .record;
    let active = serde_json::from_str::<serde_json::Value>(&created.json()).unwrap();
    assert_eq!(active["responsesAvailable"], false);

    let completed = searches.complete(created.token).unwrap().0;
    let completed = serde_json::from_str::<serde_json::Value>(&completed.json()).unwrap();
    assert_eq!(completed["responsesAvailable"], true);

    let cancelled = searches
        .create(
            None,
            "cancelled late completion".to_owned(),
            "global",
            None,
            Vec::new(),
            60,
        )
        .unwrap()
        .record;
    let (cancelled, transitioned) = searches
        .set_status_by_token(cancelled.token, "cancelled")
        .unwrap();
    assert!(transitioned);
    let (cancelled_after_completion, transitioned) = searches.complete(cancelled.token).unwrap();
    assert!(!transitioned);
    assert_eq!(cancelled_after_completion.status, "cancelled");
    assert_eq!(cancelled_after_completion.updated_at, cancelled.updated_at);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_create_rejects_before_mutation_when_dispatch_is_unavailable() {
    let (state, receiver) = test_state();
    drop(receiver);

    let response = crate::route_http_request(
        "POST",
        "/api/searches",
        None,
        r#"{"query":"never dispatched"}"#,
        &state,
    )
    .await
    .expect("failed dispatch response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("session manager is not running"));
    let searches = state.searches.read().await;
    assert!(searches.records.is_empty());
    assert_eq!(searches.next_token, 1);
    drop(searches);
    assert!(state
        .events
        .read()
        .await
        .records
        .iter()
        .all(|event| event.kind != "search.started"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn wishlist_routes_roll_back_when_persistence_fails() {
    for (path, body) in [
        ("/api/wishlist", r#"{"artist":"Artist","title":"Track"}"#),
        (
            "/api/source-feeds",
            r#"{"name":"feed","text":"Artist - One\nArtist - Two"}"#,
        ),
        (
            "/api/musicbrainz/release-radar/subscriptions",
            r#"{"artist":"Artist","title":"Release"}"#,
        ),
        (
            "/api/wishlist/import/csv",
            r#"{"csv":"artist,title\nArtist,One\nArtist,Two"}"#,
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.wishlist.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("failed wishlist creation response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("wishlist persistence failed"),
            "{path}"
        );
        assert_eq!(*state.wishlist.read().await, previous, "{path}");
    }

    for (method, body, expected_error) in [
        (
            "PUT",
            r#"{"artist":"Changed","title":"Changed"}"#,
            "wishlist persistence failed",
        ),
        ("DELETE", "", "wishlist deletion persistence failed"),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .wishlist
            .write()
            .await
            .add_item("Artist".to_owned(), "Track".to_owned(), "Audio".to_owned())
            .unwrap();
        let previous = state.wishlist.read().await.clone();
        db.close_for_test().await;

        let response =
            crate::route_http_request(method, "/api/wishlist/wish-1", None, body, &state)
                .await
                .expect("failed wishlist mutation response");
        assert_eq!(response.status, "503 Service Unavailable", "{method}");
        assert!(response.body.contains(expected_error), "{method}");
        assert_eq!(*state.wishlist.read().await, previous, "{method}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn wishlist_persistence_batches_across_sqlite_parameter_boundaries() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let mut store = crate::WishlistStore::with_max_items(50);
    for index in 0..41 {
        store
            .add_item(
                format!("Artist {index}"),
                format!("Track {index}"),
                "Audio".to_owned(),
            )
            .unwrap();
    }
    let mut items = store.records[0].items.clone();
    assert!(crate::persist_wishlist_items_checked(&state, &items)
        .await
        .unwrap());
    assert_eq!(db.list_wishlist_items(100, 0).await.unwrap().len(), 41);

    for item in &mut items {
        item.total_search_count = 2;
    }
    crate::persist_wishlist_items_checked(&state, &items)
        .await
        .unwrap();
    assert!(db
        .list_wishlist_items(100, 0)
        .await
        .unwrap()
        .iter()
        .all(|item| item.total_search_count == 2));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_api_lists_with_filters_and_pagination() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"alpha\",\"target\":\"global\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"beta\",\"target\":\"wishlist\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
        .await
        .unwrap();

    let filtered = crate::route_http_request(
        "GET",
        "/api/v0/searches/records?status=active&target=wishlist&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered searches");

    assert_eq!(filtered.status, "200 OK");
    assert!(filtered.body.contains("\"count\":2"));
    assert!(filtered.body.contains("\"filtered_count\":1"));
    assert!(filtered.body.contains("\"limit\":1"));
    assert!(filtered.body.contains("\"query\":\"beta\""));
    assert!(!filtered.body.contains("\"query\":\"alpha\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_api_expires_and_prunes_records() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    let created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"short lived\",\"ttl_seconds\":1}",
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created.status, "200 OK");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert!(created_json["searchId"].is_string());
    assert_eq!(created_json["query"], "short lived");
    let _ = receiver.try_recv();

    {
        let mut searches = state.searches.write().await;
        let record = searches
            .records
            .iter_mut()
            .find(|record| record.token == 1)
            .unwrap();
        record.expires_at = 0;
    }

    let listed = crate::route_http_request("GET", "/api/v0/searches/records", None, "", &state)
        .await
        .expect("list searches");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"status\":\"expired\""));
    assert!(listed.body.contains("\"expired\":1"));

    let pruned = crate::route_http_request("POST", "/api/v0/searches/prune", None, "", &state)
        .await
        .expect("prune searches");
    assert_eq!(pruned.status, "200 OK");
    assert!(pruned.body.contains("\"pruned\":1"));
    assert!(pruned.body.contains("\"remaining\":0"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_create_accepts_camel_case_ttl_and_caps_large_values() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    let created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"bounded\",\"ttlSeconds\":999999}",
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created.status, "200 OK");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert!(created_json["searchId"].is_string());
    assert_eq!(created_json["query"], "bounded");
    let _ = receiver.try_recv();

    let invalid = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"bad ttl\",\"ttl_seconds\":0}",
        &state,
    )
    .await
    .expect("invalid ttl");
    assert_eq!(invalid.status, "400 Bad Request");
    assert_eq!(
        invalid.body,
        "{\"error\":\"search ttl_seconds must be greater than zero\"}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_api_supports_targeted_dispatch_commands() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    let user = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"rare\",\"target\":\"user\",\"username\":\"friend\"}",
        &state,
    )
    .await
    .expect("user search");
    assert_eq!(user.status, "200 OK");
    let user_json = serde_json::from_str::<serde_json::Value>(&user.body).unwrap();
    assert!(user_json["searchId"].is_string());
    assert_eq!(user_json["query"], "rare");
    assert_eq!(
        receiver.try_recv().expect("user search command"),
        crate::SessionCommand::Search {
            token: 1,
            query: "rare".to_owned(),
            target: crate::SearchDispatchTarget::User("friend".to_owned()),
        }
    );

    let room = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"ambient\",\"target\":\"room\",\"room\":\"music\"}",
        &state,
    )
    .await
    .expect("room search");
    assert_eq!(room.status, "200 OK");
    let room_json = serde_json::from_str::<serde_json::Value>(&room.body).unwrap();
    assert!(room_json["searchId"].is_string());
    assert_eq!(room_json["query"], "ambient");
    assert_eq!(
        receiver.try_recv().expect("room search command"),
        crate::SessionCommand::Search {
            token: 2,
            query: "ambient".to_owned(),
            target: crate::SearchDispatchTarget::Room("music".to_owned()),
        }
    );

    let wishlist = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"wantlist\",\"target\":\"wishlist\"}",
        &state,
    )
    .await
    .expect("wishlist search");
    assert_eq!(wishlist.status, "200 OK");
    let wishlist_json = serde_json::from_str::<serde_json::Value>(&wishlist.body).unwrap();
    assert!(wishlist_json["searchId"].is_string());
    assert_eq!(wishlist_json["query"], "wantlist");
    assert_eq!(
        receiver.try_recv().expect("wishlist search command"),
        crate::SessionCommand::Search {
            token: 3,
            query: "wantlist".to_owned(),
            target: crate::SearchDispatchTarget::Wishlist,
        }
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_api_rejects_invalid_targeted_dispatch() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"rare\",\"target\":\"user\"}",
        &state,
    )
    .await
    .expect("bad user search");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(
        response.body,
        "{\"error\":\"username is required for user search\"}"
    );

    let invalid_target = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"rare\",\"target\":\"bogus\"}",
        &state,
    )
    .await
    .expect("bad target search");

    assert_eq!(invalid_target.status, "400 Bad Request");
    assert_eq!(invalid_target.body, "{\"error\":\"invalid search target\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_response_api_merges_flattened_results() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .unwrap();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":1,\"peer_username\":\"peer1\",\"filename\":\"Remote/Song.mp3\",\"size\":99,\"slot_free\":false,\"average_speed\":12,\"queue_length\":3}",
        &state,
    )
    .await
    .expect("ingest response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"result_count\":1"));
    assert!(response.body.contains("\"peer_username\":\"peer1\""));
    assert!(response.body.contains("\"filename\":\"Remote/Song.mp3\""));
    assert!(response.body.contains("\"extension\":\"mp3\""));
    assert!(response.body.contains("\"locked\":false"));
    assert!(response.body.contains("\"slot_free\":false"));
    assert!(response.body.contains("\"average_speed\":12"));
    assert!(response.body.contains("\"queue_length\":3"));

    let responses =
        crate::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
            .await
            .expect("search responses");
    assert_eq!(responses.status, "200 OK");
    assert!(responses.body.starts_with('['));
    assert!(responses.body.contains("\"token\":1"));
    assert!(responses.body.contains("\"filename\":\"Remote/Song.mp3\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn wishlist_ignored_folders_persist_and_suppress_existing_and_future_results() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let created = crate::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Album","filter":"flac","enabled":true,"autoDownload":true,"maxResults":25,"maxDownloads":3}"#,
        &state,
    )
    .await
    .unwrap();
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let item_id = created_json["id"].as_str().unwrap().to_owned();
    assert_eq!(created_json["filter"], "flac");
    assert_eq!(created_json["autoDownload"], true);
    assert_eq!(created_json["maxResults"], 25);
    assert_eq!(created_json["maxDownloads"], 3);
    let search = crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(search.status, "202 Accepted");
    assert_eq!(
        receiver.try_recv().expect("wishlist search dispatch"),
        crate::SessionCommand::Search {
            token: 1,
            query: "Artist Album".to_owned(),
            target: crate::SearchDispatchTarget::Wishlist,
        }
    );
    assert_eq!(
        state.searches.read().await.records[0].wishlist_item_id(),
        Some(item_id.as_str())
    );

    for (username, filename) in [
        ("PeerOne", "Remote/Album/One.flac"),
        ("PeerTwo", "Remote/Album/Two.flac"),
    ] {
        crate::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            &format!(r#"{{"token":1,"username":"{username}","filename":"{filename}","size":1}}"#),
            &state,
        )
        .await
        .unwrap();
    }
    assert_eq!(state.searches.read().await.records[0].results.len(), 2);

    let history = crate::route_http_request(
        "GET",
        &format!("/api/wishlist/{item_id}/searches?limit=1"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(history.status, "200 OK");
    let history_json = serde_json::from_str::<serde_json::Value>(&history.body).unwrap();
    assert_eq!(history_json.as_array().unwrap().len(), 1);
    assert_eq!(history_json[0]["wishlistItemId"], item_id);
    assert_eq!(history_json[0]["result_count"], 2);
    assert!(history_json[0].get("results").is_none());
    assert!(history_json[0].get("responses").is_none());

    let viewed = crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/mark-viewed"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(viewed.status, "204 No Content");
    assert!(state
        .wishlist
        .read()
        .await
        .get_item(&item_id)
        .unwrap()
        .last_viewed_at
        .is_some());

    let ignored = crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/ignored-results"),
        None,
        r#"{"username":"PeerOne","directory":"/Remote\\Album/"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(ignored.status, "201 Created");
    let ignored_json = serde_json::from_str::<serde_json::Value>(&ignored.body).unwrap();
    let rule_id = ignored_json["id"].as_str().unwrap().to_owned();
    assert_eq!(ignored_json["directory"], "Remote/Album");
    assert_eq!(state.searches.read().await.records[0].results.len(), 1);

    let updated = crate::route_http_request(
        "PUT",
        &format!("/api/wishlist/{item_id}"),
        None,
        r#"{"title":"Updated Album","filter":"mp3","enabled":false,"autoDownload":false,"maxResults":10,"maxDownloads":null}"#,
        &state,
    )
    .await
    .unwrap();
    let updated_json = serde_json::from_str::<serde_json::Value>(&updated.body).unwrap();
    assert_eq!(updated_json["title"], "Updated Album");
    assert_eq!(updated_json["filter"], "mp3");
    assert_eq!(updated_json["enabled"], false);
    assert_eq!(updated_json["autoDownload"], false);
    assert_eq!(updated_json["maxResults"], 10);
    assert!(updated_json["maxDownloads"].is_null());
    assert_eq!(
        db.list_all_wishlist_ignored_results().await.unwrap().len(),
        1
    );

    let duplicate = crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/ignored-results"),
        None,
        r#"{"username":"peerone","directory":"remote/album"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(duplicate.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap()["id"],
        rule_id
    );

    for filename in ["Remote/Album/Again.mp3", "Remote/Other/Allowed.mp3"] {
        crate::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            &format!(r#"{{"token":1,"username":"PEERONE","filename":"{filename}","size":1}}"#),
            &state,
        )
        .await
        .unwrap();
    }
    let search = state.searches.read().await.records[0].clone();
    assert_eq!(search.results.len(), 2);
    assert!(search
        .results
        .iter()
        .any(|result| result.filename == "Remote/Other/Allowed.mp3"));

    let listed = crate::route_http_request(
        "GET",
        &format!("/api/wishlist/{item_id}/ignored-results"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    assert_eq!(listed_json.as_array().unwrap().len(), 1);
    let wishlist = crate::route_http_request("GET", "/api/wishlist", None, "", &state)
        .await
        .unwrap();
    let wishlist_json = serde_json::from_str::<serde_json::Value>(&wishlist.body).unwrap();
    assert_eq!(wishlist_json[0]["ignoredResultCount"], 1);
    assert_eq!(wishlist_json[0]["filter"], "mp3");
    assert_eq!(wishlist_json[0]["enabled"], false);
    assert_eq!(wishlist_json[0]["maxResults"], 10);
    assert!(wishlist_json[0]["maxDownloads"].is_null());
    assert_eq!(wishlist_json[0]["ignoredResults"][0]["id"], rule_id);
    assert_eq!(
        wishlist_json[0]["ignoredResults"][0]["directory"],
        "Remote/Album"
    );
    let persisted = db.list_all_wishlist_ignored_results().await.unwrap();
    assert_eq!(persisted.len(), 1);
    let rehydrated = crate::WishlistStore::from_persisted_with_ignored(
        db.list_wishlist_items(10, 0).await.unwrap(),
        persisted,
    );
    assert_eq!(rehydrated.list_ignored_results(&item_id).unwrap().len(), 1);
    let rehydrated_item = rehydrated.get_item(&item_id).unwrap();
    assert_eq!(rehydrated_item.title, "Updated Album");
    assert_eq!(rehydrated_item.filter, "mp3");
    assert!(!rehydrated_item.enabled);
    assert_eq!(rehydrated_item.max_results, 10);
    assert_eq!(rehydrated_item.max_downloads, None);

    let restored = crate::route_http_request(
        "DELETE",
        &format!("/api/wishlist/{item_id}/ignored-results/{rule_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(restored.status, "204 No Content");
    assert!(db
        .list_all_wishlist_ignored_results()
        .await
        .unwrap()
        .is_empty());

    crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"username":"PeerOne","filename":"Remote/Album/Restored.mp3","size":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(state.searches.read().await.records[0].results.len(), 3);

    let completed = crate::route_http_request("POST", "/api/searches/1/complete", None, "", &state)
        .await
        .unwrap();
    assert_eq!(completed.status, "200 OK");
    let item = state.wishlist.read().await.get_item(&item_id).unwrap();
    assert_eq!(item.last_visible_hit_count, 3);
    assert_eq!(item.last_match_count, 3);
    assert_eq!(item.last_ignored_result_hit_count, 1);
    assert_eq!(item.last_filtered_out_hit_count, 0);
    assert_eq!(item.last_hidden_locked_hit_count, 0);
    assert_eq!(item.last_response_count, 5);
    assert_eq!(item.total_search_count, 1);
    assert_eq!(item.last_search_id.as_deref(), Some("1"));
    assert!(item.last_searched_at.is_some());
    let persisted_item = db.list_wishlist_items(10, 0).await.unwrap().remove(0);
    assert_eq!(persisted_item.last_visible_hit_count, 3);
    assert_eq!(persisted_item.last_ignored_result_hit_count, 1);
    assert_eq!(persisted_item.total_search_count, 1);

    let repeated = crate::route_http_request("POST", "/api/searches/1/complete", None, "", &state)
        .await
        .unwrap();
    assert_eq!(repeated.status, "200 OK");
    assert_eq!(
        state
            .wishlist
            .read()
            .await
            .get_item(&item_id)
            .unwrap()
            .total_search_count,
        1
    );
    assert_eq!(
        db.list_wishlist_items(10, 0)
            .await
            .unwrap()
            .remove(0)
            .total_search_count,
        1
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn wishlist_ignored_folder_mutations_roll_back_on_persistence_failure() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .wishlist
        .write()
        .await
        .add_item("Artist".to_owned(), "Album".to_owned(), "Audio".to_owned())
        .unwrap();
    let search = crate::route_http_request("POST", "/api/wishlist/wish-1/search", None, "", &state)
        .await
        .unwrap();
    assert_eq!(search.status, "202 Accepted");
    crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"username":"friend","filename":"Remote/Album/One.flac","size":1}"#,
        &state,
    )
    .await
    .unwrap();
    let searches_before_failure = state.searches.read().await.clone();
    db.close_for_test().await;

    let create = crate::route_http_request(
        "POST",
        "/api/wishlist/wish-1/ignored-results",
        None,
        r#"{"username":"friend","directory":"Remote/Album"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(create.status, "503 Service Unavailable");
    assert!(state.wishlist.read().await.ignored_results.is_empty());
    assert_eq!(*state.searches.read().await, searches_before_failure);

    let rule = state
        .wishlist
        .write()
        .await
        .ignore_result("wish-1", "friend", "Remote/Album", false)
        .unwrap()
        .0;
    let delete = crate::route_http_request(
        "DELETE",
        &format!("/api/wishlist/wish-1/ignored-results/{}", rule.id),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(delete.status, "503 Service Unavailable");
    assert_eq!(state.wishlist.read().await.ignored_results, vec![rule]);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_response_api_accepts_controller_group_payload() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .unwrap();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":1,\"username\":\"peer1\",\"hasFreeUploadSlot\":false,\"uploadSpeed\":88,\"queueLength\":5,\"files\":[{\"filename\":\"Public/One.flac\",\"size\":11}],\"lockedFiles\":[{\"filename\":\"Private/Two.mp3\",\"size\":22}]}",
        &state,
    )
    .await
    .expect("slskd search response");

    assert_eq!(response.status, "200 OK");
    let search_json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(search_json["result_count"], 2);
    assert_eq!(search_json["lockedFileCount"], 1);
    assert_eq!(search_json["results"][0]["peer_username"], "peer1");
    assert_eq!(search_json["results"][0]["slot_free"], false);
    assert_eq!(search_json["results"][0]["average_speed"], 88);
    assert_eq!(search_json["results"][0]["queue_length"], 5);

    let responses =
        crate::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
            .await
            .expect("search responses");
    let responses_json = serde_json::from_str::<serde_json::Value>(&responses.body).unwrap();
    assert_eq!(responses_json[0]["username"], "peer1");
    assert_eq!(responses_json[0]["fileCount"], 1);
    assert_eq!(responses_json[0]["lockedFileCount"], 1);
    assert_eq!(responses_json[0]["files"][0]["filename"], "Public/One.flac");
    assert_eq!(responses_json[0]["files"][0]["resultIndex"], 0);
    assert_eq!(
        responses_json[0]["lockedFiles"][0]["filename"],
        "Private/Two.mp3"
    );
    assert_eq!(responses_json[0]["lockedFiles"][0]["resultIndex"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_detail_routes_page_result_arrays_without_changing_totals() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .unwrap();

    for filename in ["Remote/One.flac", "Remote/Two.flac", "Remote/Three.flac"] {
        let body = format!(
            "{{\"token\":1,\"peer_username\":\"peer1\",\"filename\":\"{}\",\"size\":99}}",
            filename
        );
        crate::route_http_request("POST", "/api/v0/search-responses", None, &body, &state)
            .await
            .expect("ingest response");
    }

    for path in [
        "/api/v0/searches/1?offset=1&limit=1",
        "/api/searches/1?offset=1&limit=1",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("paged search detail");
        assert_eq!(response.status, "200 OK");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(json["result_count"], 3);
        assert_eq!(json["fileCount"], 3);
        assert_eq!(json["resultOffset"], 1);
        assert_eq!(json["resultLimit"], 1);
        assert_eq!(json["results"].as_array().unwrap().len(), 1);
        assert_eq!(json["results"][0]["filename"], "Remote/Two.flac");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_response_routes_page_and_filter_peer_groups() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .expect("create search");

    for (peer, filename) in [
        ("alpha", "Remote/Alpha.flac"),
        ("beta", "Remote/Beta.flac"),
        ("gamma", "Remote/Gamma.flac"),
        ("gamma", "Remote/Gamma-Alt.flac"),
    ] {
        let body = format!(
            "{{\"token\":1,\"peer_username\":\"{}\",\"filename\":\"{}\",\"size\":99}}",
            peer, filename
        );
        crate::route_http_request("POST", "/api/v0/search-responses", None, &body, &state)
            .await
            .expect("ingest response");
    }

    let paged = crate::route_http_request(
        "GET",
        "/api/v0/searches/1/responses?offset=1&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("paged responses");
    assert_eq!(paged.status, "200 OK");
    let paged_json = serde_json::from_str::<serde_json::Value>(&paged.body).unwrap();
    assert_eq!(paged_json.as_array().unwrap().len(), 1);
    assert_eq!(paged_json[0]["username"], "beta");
    assert_eq!(paged_json[0]["fileCount"], 1);

    let filtered = crate::route_http_request(
        "GET",
        "/api/searches/1/responses?username=gamma&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered responses");
    assert_eq!(filtered.status, "200 OK");
    let filtered_json = serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap();
    assert_eq!(filtered_json.as_array().unwrap().len(), 1);
    assert_eq!(filtered_json[0]["username"], "gamma");
    assert_eq!(filtered_json[0]["fileCount"], 2);
    assert_eq!(filtered_json[0]["files"].as_array().unwrap().len(), 2);

    let query_filtered = crate::route_http_request(
        "GET",
        "/api/v0/searches/1/responses?q=beta",
        None,
        "",
        &state,
    )
    .await
    .expect("query filtered responses");
    let query_json = serde_json::from_str::<serde_json::Value>(&query_filtered.body).unwrap();
    assert_eq!(query_json.as_array().unwrap().len(), 1);
    assert_eq!(query_json[0]["username"], "beta");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_update_routes_mutate_lifecycle_and_query_projection() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    state.session.write().await.state = "connected";
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"original\"}",
        &state,
    )
    .await
    .expect("create search");

    let completed = crate::route_http_request(
        "PUT",
        "/api/searches/1",
        None,
        "{\"state\":\"Completed\",\"searchText\":\"renamed\"}",
        &state,
    )
    .await
    .expect("update search");
    assert_eq!(completed.status, "200 OK");
    let completed_json = serde_json::from_str::<serde_json::Value>(&completed.body).unwrap();
    assert_eq!(completed_json["updated"], true);
    assert_eq!(completed_json["query"], "renamed");
    assert_eq!(completed_json["status"], "completed");
    assert_eq!(completed_json["state"], "Completed");
    assert_eq!(completed_json["isComplete"], true);

    let active = crate::route_http_request(
        "PUT",
        "/api/searches/1",
        None,
        "{\"isComplete\":false}",
        &state,
    )
    .await
    .expect("reactivate search");
    assert_eq!(active.status, "200 OK");
    let active_json = serde_json::from_str::<serde_json::Value>(&active.body).unwrap();
    assert_eq!(active_json["updated"], true);
    assert_eq!(active_json["status"], "active");
    assert_eq!(active_json["state"], "InProgress");
    assert_eq!(active_json["isComplete"], false);

    let fetched = crate::route_http_request("GET", "/api/v0/searches/1", None, "", &state)
        .await
        .expect("fetch updated search");
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
    assert_eq!(fetched_json["query"], "renamed");
    assert_eq!(fetched_json["status"], "active");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_action_routes_preserve_cancel_fail_and_expire_lifecycle() {
    let (state, _receiver) = test_state();
    for query in ["cancel me", "fail me", "expire me"] {
        crate::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            &format!(r#"{{"query":"{query}"}}"#),
            &state,
        )
        .await
        .expect("create search");
    }

    for (path, status, state_name) in [
        ("/api/v0/searches/1/cancel", "cancelled", "Cancelled"),
        ("/api/searches/2/fail", "failed", "Failed"),
        ("/api/v0/searches/3/expire", "expired", "Expired"),
    ] {
        let response = crate::route_http_request("POST", path, None, "", &state)
            .await
            .expect("search action");
        assert_eq!(response.status, "200 OK");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(json["status"], status);
        assert_eq!(json["state"], state_name);
        assert_eq!(json["isComplete"], true);
    }

    let event_count = state.events.read().await.records.len();
    let repeated = crate::route_http_request("POST", "/api/v0/searches/1/cancel", None, "", &state)
        .await
        .expect("repeat cancelled search action");
    assert_eq!(repeated.status, "200 OK");
    assert_eq!(state.events.read().await.records.len(), event_count);

    let listed = crate::route_http_request(
        "GET",
        "/api/v0/searches/records?status=cancelled",
        None,
        "",
        &state,
    )
    .await
    .expect("list cancelled searches");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    assert_eq!(listed_json["filtered_count"], 1);
    assert_eq!(listed_json["entries"][0]["state"], "Cancelled");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_response_api_projects_locked_files_for_controller_shape() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .unwrap();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":1,\"peer_username\":\"peer1\",\"filename\":\"Private/Locked.flac\",\"size\":321,\"isLocked\":true,\"slot_free\":true}",
        &state,
    )
    .await
    .expect("ingest locked response");

    assert_eq!(response.status, "200 OK");
    let search_json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(search_json["lockedFileCount"], 1);
    assert_eq!(search_json["results"][0]["locked"], true);

    let responses =
        crate::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
            .await
            .expect("search responses");
    let responses_json = serde_json::from_str::<serde_json::Value>(&responses.body).unwrap();
    assert_eq!(responses_json[0]["fileCount"], 0);
    assert_eq!(responses_json[0]["lockedFileCount"], 1);
    assert_eq!(
        responses_json[0]["lockedFiles"][0]["filename"],
        "Private/Locked.flac"
    );
    assert_eq!(responses_json[0]["lockedFiles"][0]["isLocked"], true);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_response_api_rejects_missing_fields() {
    let (state, _receiver) = test_state();

    let response =
        crate::route_http_request("POST", "/api/v0/search-responses", None, "{}", &state)
            .await
            .expect("bad response");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(response.body, "{\"error\":\"token is required\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_response_api_rejects_oversized_protocol_token() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        "{\"token\":4294967296,\"peer_username\":\"peer1\",\"filename\":\"Remote/Song.mp3\"}",
        &state,
    )
    .await
    .expect("oversized response token");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(response.body, "{\"error\":\"token exceeds u32 range\"}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn search_store_merges_peer_search_responses() {
    let mut store = crate::SearchStore::new();
    let record = store
        .create(None, "remote".to_owned(), "global", None, Vec::new(), 300)
        .unwrap()
        .record;
    let response = FileSearchResponse {
        username: "peer1".to_owned(),
        token: record.token,
        results: vec![FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Remote/Song.flac".to_owned(),
            size: 123,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        }],
        slot_free: false,
        average_speed: 42,
        queue_length: 7,
        unknown: 0,
        private_results: vec![FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Private/Locked.flac".to_owned(),
            size: 456,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        }],
    };

    let updated = store
        .add_peer_response(&response)
        .expect("peer response accepted");

    assert_eq!(updated.results.len(), 2);
    assert_eq!(updated.results[0].peer_username.as_deref(), Some("peer1"));
    assert!(!updated.results[0].locked);
    assert_eq!(updated.results[0].slot_free, Some(false));
    assert_eq!(updated.results[0].average_speed, Some(42));
    assert_eq!(updated.results[0].queue_length, Some(7));
    assert_eq!(updated.results[1].filename, "Private/Locked.flac");
    assert!(updated.results[1].locked);
    assert_eq!(
        updated.expires_at,
        updated.updated_at.saturating_add(updated.ttl_seconds)
    );
    let responses = serde_json::from_str::<serde_json::Value>(&updated.controller_responses_json())
        .expect("response json");
    assert_eq!(responses[0]["fileCount"], 1);
    assert_eq!(responses[0]["lockedFileCount"], 1);
    assert_eq!(responses[0]["files"][0]["filename"], "Remote/Song.flac");
    assert_eq!(
        responses[0]["lockedFiles"][0]["filename"],
        "Private/Locked.flac"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn search_store_caps_results_from_peer_responses() {
    let mut store = crate::SearchStore::new();
    let record = store
        .create(None, "remote".to_owned(), "global", None, Vec::new(), 300)
        .unwrap()
        .record;
    let response = FileSearchResponse {
        username: "peer1".to_owned(),
        token: record.token,
        results: (0..(crate::MAX_SEARCH_RESULTS_PER_SEARCH + 5))
            .map(|index| FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: format!("Remote/{index}.flac"),
                size: index as u64,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            })
            .collect(),
        slot_free: true,
        average_speed: 42,
        queue_length: 0,
        unknown: 0,
        private_results: vec![FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Private/overflow.flac".to_owned(),
            size: 1,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        }],
    };

    let updated = store.add_peer_response(&response).expect("search exists");
    assert_eq!(updated.results.len(), crate::MAX_SEARCH_RESULTS_PER_SEARCH);
    assert_eq!(updated.results.last().unwrap().filename, "Remote/9999.flac");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn search_store_bounds_text_and_aggregate_results() {
    let mut store = crate::SearchStore::new();
    let oversized_query = "q".repeat(crate::MAX_SEARCH_QUERY_BYTES + 1);
    let first = store
        .create(None, oversized_query, "global", None, Vec::new(), 300)
        .unwrap()
        .record;
    assert_eq!(first.query.len(), crate::MAX_SEARCH_QUERY_BYTES);
    let response = FileSearchResponse {
        username: "u".repeat(crate::MAX_SEARCH_RESULT_USERNAME_BYTES + 1),
        token: first.token,
        results: vec![FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "f".repeat(crate::MAX_SEARCH_RESULT_FILENAME_BYTES + 1),
            size: 1,
            extension: "e".repeat(crate::MAX_SEARCH_RESULT_EXTENSION_BYTES + 1),
            attributes: Vec::new(),
        }],
        slot_free: true,
        average_speed: 1,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    };
    let updated = store.add_peer_response(&response).unwrap();
    assert_eq!(
        updated.results[0].peer_username.as_ref().unwrap().len(),
        crate::MAX_SEARCH_RESULT_USERNAME_BYTES
    );
    assert_eq!(
        updated.results[0].filename.len(),
        crate::MAX_SEARCH_RESULT_FILENAME_BYTES
    );
    assert_eq!(
        updated.results[0].extension.len(),
        crate::MAX_SEARCH_RESULT_EXTENSION_BYTES
    );

    let template = crate::SearchResultEntry {
        peer_username: Some("peer".to_owned()),
        filename: "file".to_owned(),
        size: 1,
        extension: String::new(),
        bit_rate: None,
        sample_rate: None,
        bit_depth: None,
        length_seconds: None,
        locked: false,
        slot_free: Some(true),
        average_speed: Some(1),
        queue_length: Some(0),
    };
    store.records[0].results.clear();
    while store.records.len() < 5 {
        store
            .create(None, "query".to_owned(), "global", None, Vec::new(), 300)
            .unwrap();
    }
    for record in &mut store.records {
        record.results = vec![template.clone(); crate::MAX_SEARCH_RESULTS_PER_SEARCH];
    }
    let empty = store
        .create(None, "last".to_owned(), "global", None, Vec::new(), 300)
        .unwrap()
        .record;
    let response = FileSearchResponse {
        username: "peer".to_owned(),
        token: empty.token,
        results: vec![FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "overflow".to_owned(),
            size: 1,
            extension: String::new(),
            attributes: Vec::new(),
        }],
        slot_free: true,
        average_speed: 1,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    };
    let updated = store.add_peer_response(&response).unwrap();
    assert!(updated.results.is_empty());
    assert_eq!(store.total_results(), crate::MAX_TOTAL_SEARCH_RESULTS);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn prefer_mode_uses_obfuscated_file_transfer_when_available() {
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
        crate::SearchStore::new(),
        None,
    );
    let path = std::env::temp_dir().join(format!(
        "slskr-transfer-f-{}-obfuscated-upload.bin",
        std::process::id()
    ));
    std::fs::write(&path, [5_u8, 6, 7]).expect("write upload file");
    add_test_share(&state, "Remote/Obfuscated.flac", &path, 3).await;
    {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("friend".to_owned()),
            "Remote/Obfuscated.flac".to_owned(),
            Some(path.display().to_string()),
            Some(3),
        );
        assert_eq!(entry.token, 1);
        transfers.update_status(entry.id, "peer_lookup", None, None);
    }

    let listener = slskr_client::listener::Listener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let local_addr = listener.local_addr().expect("local addr");
    let unused_regular = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("unused regular listener");
        listener.local_addr().expect("unused regular addr").port()
    };
    let server = tokio::spawn(async move {
        let (incoming, _) = listener.accept_obfuscated().await.expect("accept p");
        let slskr_client::listener::IncomingConnection::ObfuscatedPeerMessages(mut peer) = incoming
        else {
            panic!("expected obfuscated peer messages");
        };
        assert_eq!(
            peer.receive().await.expect("transfer request"),
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: Default::default(),
                direction: 1,
                token: 1,
                filename: "Remote/Obfuscated.flac".to_owned(),
                size: Some(3),
            })
        );
        peer.send(&crate::PeerMessage::TransferResponse(
            crate::TransferResponse::Allowed {
                token: 1,
                size: Some(3),
            },
        ))
        .await
        .expect("transfer response");

        let (incoming, _) = listener.accept_obfuscated().await.expect("accept f");
        let slskr_client::listener::IncomingConnection::PeerInit {
            username,
            kind,
            token,
            stream,
            obfuscated,
        } = incoming
        else {
            panic!("expected obfuscated file-transfer peer init");
        };
        assert_eq!(username, "tester");
        assert_eq!(kind, crate::ConnectionKind::FileTransfer);
        assert_eq!(token, 0);
        assert!(obfuscated);
        let mut file = slskr_client::file_transfer::FileTransferConnection::new_obfuscated(stream);
        assert_eq!(file.receive_token().await.expect("token"), 1);
        file.send_offset(0).await.expect("offset");
        file.read_chunk(3).await.expect("chunk")
    });
    let address = slskr_client::protocol::server::PeerAddress {
        username: "friend".to_owned(),
        ip: "127.0.0.1".parse().unwrap(),
        port: u32::from(unused_regular),
        obfuscation_type: crate::ROTATED_OBFUSCATION_TYPE,
        obfuscated_port: local_addr.port(),
    };

    crate::project_peer_transfer_response(&state, &address).await;
    let uploaded = server.await.expect("server task");
    assert_eq!(uploaded, vec![5, 6, 7]);

    let transfers = state.transfers.read().await;
    let record = transfers.entries.first().expect("transfer");
    assert_eq!(record.status, "succeeded");
    assert_eq!(record.bytes_transferred, 3);
    assert_eq!(record.size, Some(3));
    assert_eq!(record.reason, None);
    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn wishlist_item_search_requires_exact_action_path() {
    let (state, _receiver) = test_state();
    let wish = crate::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Artist","title":"Track"}"#,
        &state,
    )
    .await
    .unwrap();
    let item_id = serde_json::from_str::<serde_json::Value>(&wish.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/extra/search"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert!(state.searches.read().await.records.is_empty());
    assert_eq!(
        crate::wishlist_search_item_id(&format!("/api/wishlist/{item_id}/extra/search")),
        None
    );

    let exact = crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(exact.status, "202 Accepted");
    assert_eq!(
        crate::wishlist_search_item_id(&format!("/api/wishlist/{item_id}/search")),
        Some(item_id.as_str())
    );
    let searches = state.searches.read().await;
    assert_eq!(searches.records.len(), 1);
    assert_eq!(searches.records[0].query, "Artist Track");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn wishlist_bounds_items_and_allocates_unique_ids() {
    let mut wishlist = crate::WishlistStore::with_max_items(2);
    let first = wishlist
        .add_item("Artist".to_owned(), "First".to_owned(), "Audio".to_owned())
        .unwrap();
    let second = wishlist
        .add_item("Artist".to_owned(), "Second".to_owned(), "Audio".to_owned())
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(wishlist.remaining_capacity(), 0);
    assert!(wishlist
        .add_item(String::new(), "Overflow".to_owned(), "Audio".to_owned())
        .is_err());
    wishlist.next_item_id = u64::MAX;
    assert!(!wishlist.can_add_items(1));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn searches_bound_active_records_and_avoid_identity_collisions() {
    let mut searches = crate::SearchStore::new();
    for index in 0..crate::MAX_SEARCH_RECORDS {
        searches
            .create(
                None,
                format!("query-{index}"),
                "global",
                None,
                Vec::new(),
                crate::MAX_SEARCH_TTL_SECONDS,
            )
            .unwrap();
    }
    assert_eq!(
        searches
            .create(
                None,
                "overflow".to_owned(),
                "global",
                None,
                Vec::new(),
                crate::MAX_SEARCH_TTL_SECONDS,
            )
            .unwrap_err(),
        crate::SearchCreateError::CapacityFull
    );
    searches.records[0].status = "completed";
    let evicted = searches
        .create(
            None,
            "replacement".to_owned(),
            "global",
            None,
            Vec::new(),
            crate::MAX_SEARCH_TTL_SECONDS,
        )
        .unwrap();
    assert_eq!(evicted.evicted.len(), 1);
    assert_eq!(searches.records.len(), crate::MAX_SEARCH_RECORDS);

    let mut wrapped = crate::SearchStore::new();
    wrapped.next_token = u32::MAX;
    let external = wrapped
        .create(
            Some("2".to_owned()),
            "external".to_owned(),
            "global",
            None,
            Vec::new(),
            300,
        )
        .unwrap();
    assert_eq!(external.record.token, u32::MAX);
    let after_wrap = wrapped
        .create(None, "wrapped".to_owned(), "global", None, Vec::new(), 300)
        .unwrap();
    assert_eq!(after_wrap.record.token, 1);
    let skips_string_id = wrapped
        .create(None, "skip".to_owned(), "global", None, Vec::new(), 300)
        .unwrap();
    assert_eq!(skips_string_id.record.token, 3);
    assert_eq!(
        wrapped
            .create(
                Some("2".to_owned()),
                "duplicate".to_owned(),
                "global",
                None,
                Vec::new(),
                300,
            )
            .unwrap_err(),
        crate::SearchCreateError::DuplicateId
    );

    let mut persisted = (1..=crate::MAX_SEARCH_RECORDS + 1)
        .map(|index| crate::persistence::SearchRecord {
            id: index.to_string(),
            query: format!("persisted-{index}"),
            status: "completed".to_owned(),
            result_count: 0,
            created_at: index as i64,
            completed_at: Some(index as i64),
            room: None,
            target: Some("global".to_owned()),
            fallback_attempts: 0,
        })
        .collect::<Vec<_>>();
    persisted.push(crate::persistence::SearchRecord {
        id: "1".to_owned(),
        query: "duplicate".to_owned(),
        status: "completed".to_owned(),
        result_count: 0,
        created_at: 0,
        completed_at: Some(0),
        room: None,
        target: Some("global".to_owned()),
        fallback_attempts: 0,
    });
    let hydrated = crate::SearchStore::from_persisted(persisted);
    assert_eq!(hydrated.records.len(), crate::MAX_SEARCH_RECORDS);
    assert_eq!(
        hydrated
            .records
            .iter()
            .filter(|record| record.id == "1")
            .count(),
        1
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn wishlist_store_terms_feed_scheduled_search_records() {
    let mut wishlist = crate::WishlistStore::new();
    wishlist
        .add_item("Artist".to_owned(), "Title".to_owned(), "Audio".to_owned())
        .unwrap();
    wishlist
        .add_item(String::new(), "Rare Track".to_owned(), "Audio".to_owned())
        .unwrap();

    assert_eq!(
        wishlist.search_terms(),
        vec!["Artist Title".to_owned(), "Rare Track".to_owned()]
    );

    let mut searches = crate::SearchStore::new();
    let record = searches
        .create_scheduled_wishlist("Artist Title".to_owned(), 300)
        .unwrap()
        .record;
    assert_eq!(record.token, 1);
    assert_eq!(record.target, "wishlist");
    assert_eq!(record.query, "Artist Title");
    assert_eq!(searches.summary_json(), "{\"total\":1,\"active\":1,\"completed\":0,\"expired\":0,\"results\":0,\"global\":0,\"user\":0,\"room\":0,\"wishlist\":1,\"next_token\":2}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn wishlist_result_filter_is_literal_case_insensitive_and_bounded_by_policy() {
    let filter = crate::WishlistResultFilter::parse(r#"flac OR "studio mix" -live -.cue"#);
    assert!(filter.matches("Artist/STUDIO MIX.FLAC"));
    assert!(filter.matches("Artist/Album/Track.flac"));
    assert!(!filter.matches("Artist/Live/Track.flac"));
    assert!(!filter.matches("Artist/Album/disc.cue"));
    assert!(!filter.matches("Artist/Album/Track.mp3"));

    let literal = crate::WishlistResultFilter::parse("[a-z]+.flac");
    assert!(literal.matches("Remote/[a-z]+.flac"));
    assert!(!literal.matches("Remote/track.flac"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn wishlist_auto_download_enqueues_best_folder_and_applies_one_shot_limit() {
    let (state, mut receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"searchText":"rare album","filter":"flac","autoDownload":true,"maxResults":10,"maxDownloads":null}"#,
        &state,
    )
    .await
    .unwrap();
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let search = crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{item_id}/search"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(search.status, "202 Accepted");
    assert!(matches!(
        receiver.try_recv().unwrap(),
        crate::SessionCommand::Search { token: 1, .. }
    ));

    for filename in ["Remote/Album/One.flac", "Remote/Album/Two.flac"] {
        crate::route_http_request(
            "POST",
            "/api/search-responses",
            None,
            &format!(
                r#"{{"token":1,"username":"fast-peer","filename":"{filename}","size":12,"slot_free":true,"average_speed":100,"queue_length":0}}"#
            ),
            &state,
        )
        .await
        .unwrap();
    }
    crate::route_http_request(
        "POST",
        "/api/search-responses",
        None,
        r#"{"token":1,"username":"slow-peer","filename":"Other/Album/Track.flac","size":12,"slot_free":false,"average_speed":1,"queue_length":9}"#,
        &state,
    )
    .await
    .unwrap();

    let completed = crate::route_http_request("POST", "/api/searches/1/complete", None, "", &state)
        .await
        .unwrap();
    assert_eq!(completed.status, "200 OK");

    let first = receiver.try_recv().expect("first automatic download");
    let second = receiver.try_recv().expect("second automatic download");
    assert!(matches!(
        first,
        crate::SessionCommand::TransferPeer {
            username,
            ..
        } if username == "fast-peer"
    ));
    assert!(matches!(
        second,
        crate::SessionCommand::TransferPeer {
            username,
            ..
        } if username == "fast-peer"
    ));
    assert!(receiver.try_recv().is_err());

    let transfers = state.transfers.read().await;
    assert_eq!(transfers.entries.len(), 2);
    assert!(transfers.entries.iter().all(|entry| {
        entry.status == "peer_lookup"
            && entry.batch_id.is_some()
            && entry.wishlist_item_id.as_deref() == Some(item_id.as_str())
    }));
    drop(transfers);
    let item = state.wishlist.read().await.get_item(&item_id).unwrap();
    assert_eq!(item.total_download_count, 2);
    assert!(!item.enabled);
    assert_eq!(item.total_search_count, 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn search_api_rejects_missing_query() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("POST", "/api/v0/searches", None, "{}", &state)
        .await
        .expect("bad search");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(
        response.body,
        "{\"error\":\"query/searchText is required\"}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_native_swagger_updates_current_options_but_not_startup_options() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    assert!(state.config.controller_swagger);
    let yaml = "feature:\n  swagger: false\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(current["feature"]["swagger"], false);
    assert_eq!(startup["feature"]["swagger"], true);
    assert!(state.runtime.read().await.application_restart_requested);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_cors_changes_do_not_mark_restart_required() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let yaml = "web:\n  cors:\n    enabled: true\n    allowed_origins: [https://allowed.example]\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(!state.runtime.read().await.application_restart_requested);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_native_listener_change_requires_reconnect() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    state.session.write().await.state = "connected";
    let yaml = "soulseek:\n  listen_port: 50301\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(state.runtime.read().await.application_reconnect_pending);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_native_obfuscated_listener_change_waits_for_restart() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let previous_port = crate::effective_obfuscated_advertised_port(&state);
    let yaml = "soulseek:\n  obfuscation:\n    listen_port: 50302\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    let reloaded =
        crate::load_watched_controller_configuration(state.controller_cli_environment.clone())
            .expect("obfuscated listener config reload");
    assert_eq!(
        reloaded.obfuscated_listener_bind.as_deref(),
        Some("0.0.0.0:50302")
    );
    assert_eq!(reloaded.obfuscated_advertised_port, Some(50_302));

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert_eq!(
        crate::effective_obfuscated_advertised_port(&state),
        previous_port
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_download_policy_updates_runtime_and_cancels_blocked_downloads() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_CONFIGURATION", "true"));
    let entry = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("peer".to_owned()),
            "Music/blocked.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_status(entry.id, "peer_lookup", None, None)
            .expect("active download")
    };
    let yaml = "filters:\n  download:\n    exclude:\n      - blocked\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert_eq!(
        crate::effective_download_exclusions(&state).await,
        vec!["blocked".to_owned()]
    );
    let transfer = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|candidate| candidate.id == entry.id)
        .cloned()
        .expect("reloaded transfer");
    assert_eq!(transfer.status, "cancelled");
    assert!(transfer
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("blocked by download exclusion")));
    let config = crate::route_http_request("GET", "/api/config/download-filter", None, "", &state)
        .await
        .expect("download policy response");
    assert_eq!(config.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&config.body).unwrap()["exclude"],
        serde_json::json!(["blocked"])
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_native_dht_updates_current_options_but_retains_startup_socket_settings()
{
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    assert!(state.config.advanced_networking.dht.enabled);
    let yaml = "dht:\n  enabled: false\n  dht_port: 51002\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(current["dhtRendezvous"]["enabled"], false);
    assert_eq!(current["dhtRendezvous"]["dhtPort"], 51_002);
    assert_eq!(startup["dhtRendezvous"]["enabled"], true);
    assert_eq!(startup["dhtRendezvous"]["dhtPort"], 50_305);
    assert!(!state.advanced_networking.read().await.dht.enabled);
    assert!(!state.runtime.read().await.application_restart_requested);
    let status = crate::route_http_request("GET", "/api/v0/dht/status", None, "", &state)
        .await
        .expect("watched DHT status");
    assert_eq!(status.status, "200 OK");
    let status = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status["isEnabled"], false);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_credentials_update_configured_login_without_overwriting_runtime_credentials(
) {
    let (state, _receiver) = test_state();
    state.session.write().await.state = "connected";
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let yaml = "remote_configuration: true\nsoulseek:\n  username: watched-user\n  password: watched-password\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    crate::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    let configured = state
        .configured_credentials
        .read()
        .await
        .clone()
        .expect("watched credentials");
    assert_eq!(configured.username, "watched-user");
    assert_eq!(configured.password, "watched-password");
    assert!(state.runtime.read().await.application_reconnect_pending);
    assert_eq!(
        crate::pod_request_peer_id(&state).await.as_deref(),
        Some("watched-user")
    );

    *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
        "runtime-user",
        "runtime-password",
    ));
    assert_eq!(
        crate::pod_request_peer_id(&state).await.as_deref(),
        Some("runtime-user")
    );
    assert_eq!(
        state
            .configured_credentials
            .read()
            .await
            .as_ref()
            .map(|credentials| credentials.username.as_str()),
        Some("watched-user")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn watched_server_endpoint_marks_reconnect_only_while_connected() {
    let (state, _receiver) = test_state();
    let original_endpoint = crate::effective_server_address(&state);
    state.session.write().await.state = "connected";
    *state
        .connected_server_address
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(original_endpoint.clone());
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let first = "soulseek:\n  address: 127.0.0.2\n  port: 34567\n";
    fs::write(state.config.state_dir.join("slskd.yml"), first).unwrap();

    crate::apply_watched_controller_configuration(&state, Some(first), &cli_environment).await;

    assert_eq!(crate::effective_server_address(&state), "127.0.0.2:34567");
    assert_eq!(
        crate::connected_server_address(&state).as_deref(),
        Some(original_endpoint.as_str())
    );
    assert!(state.runtime.read().await.application_reconnect_pending);

    let mut session = None;
    let mut next_ping = tokio::time::Instant::now();
    let mut reconnect_requested = true;
    crate::handle_session_command(
        &state,
        crate::SessionCommand::Disconnect,
        &mut session,
        &mut next_ping,
        &mut reconnect_requested,
    )
    .await;
    assert!(!reconnect_requested);
    assert!(crate::connected_server_address(&state).is_none());
    assert!(!state.runtime.read().await.application_reconnect_pending);

    let second = "soulseek:\n  address: 127.0.0.3\n  port: 34568\n";
    fs::write(state.config.state_dir.join("slskd.yml"), second).unwrap();
    crate::apply_watched_controller_configuration(&state, Some(second), &cli_environment).await;
    assert_eq!(crate::effective_server_address(&state), "127.0.0.3:34568");
    assert!(!state.runtime.read().await.application_reconnect_pending);
}
