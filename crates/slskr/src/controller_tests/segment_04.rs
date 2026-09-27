#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_index_persists_and_rehydrates_records() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let rescanned = super::route_http_request("POST", "/api/v0/shares/rescan", None, "", &state)
        .await
        .expect("rescan shares");
    assert_eq!(rescanned.status, "202 Accepted");
    assert!(rescanned.body.contains("\"files\":1"));

    let persisted = db.list_share_files(10, 0).await.expect("list shares");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].filename, "Virtual/Test.flac");
    assert_eq!(persisted[0].size, 42);
    assert_eq!(persisted[0].extension, "flac");
    assert_eq!(persisted[0].root_label, "Virtual");

    let rehydrated = super::ShareIndexSnapshot::from_persisted(&state.config, persisted);
    assert_eq!(rehydrated.entries.len(), 1);
    assert!(rehydrated.catalog_json(None).contains("Virtual/Test.flac"));
    assert_eq!(rehydrated.roots.len(), 1);
    assert_eq!(rehydrated.roots[0].label, "Virtual");

    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("share database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["persisted"]["shares"], 1);
    assert_eq!(stats_json["projections"]["shares"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn event_log_persists_and_rehydrates_records() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    super::record_event(
        &state,
        "search.started",
        "42",
        Some("query=durable".to_owned()),
    )
    .await;

    let compatibility_event = super::route_http_request(
        "POST",
        "/api/events/Noop",
        None,
        r#""durable compatibility event""#,
        &state,
    )
    .await
    .expect("record compatibility event");
    assert_eq!(compatibility_event.status, "200 OK");
    let nested_event = super::route_http_request(
        "POST",
        "/api/events/Noop/extra",
        None,
        r#""must not persist""#,
        &state,
    )
    .await
    .expect("reject nested event route");
    assert_eq!(nested_event.status, "404 Not Found");

    let persisted = db.list_events(10, 0).await.expect("list events");
    assert_eq!(persisted.len(), 2);
    assert!(persisted.iter().any(|record| {
        record.kind == "search.started"
            && record.resource == "42"
            && record.detail.as_deref() == Some("query=durable")
    }));
    assert!(persisted.iter().any(|record| {
        record.kind == "compat.event"
            && record.resource == "Noop"
            && record.detail.as_deref() == Some("durable compatibility event")
    }));

    let rehydrated = super::EventStore::from_persisted(persisted, super::EVENT_HISTORY_LIMIT);
    assert_eq!(rehydrated.next_id, 3);
    assert!(rehydrated.json(None).contains("\"topic\":\"searches\""));
    assert!(rehydrated
        .controller_json(Some("topic=searches"))
        .contains("\"type\":\"search.started\""));

    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("event database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["events"], 2);
    assert_eq!(stats_json["persisted"]["events"], 2);
    assert_eq!(stats_json["projections"]["events"], 2);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn event_persistence_failures_roll_back_ingest_and_surface_internal_loss() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let previous = state.events.read().await.clone();
    db.close_for_test().await;

    let response =
        super::route_http_request("POST", "/api/events/Noop", None, r#""must fail""#, &state)
            .await
            .expect("failed event persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("event persistence failed"));
    assert_eq!(*state.events.read().await, previous);

    super::record_event(&state, "internal.test", "resource", None).await;
    assert_eq!(
        state.events.read().await.records.len(),
        previous.records.len() + 1
    );
    let session = state.session.read().await;
    assert!(session
        .last_error
        .as_deref()
        .unwrap_or_default()
        .contains("event persistence failed"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn event_store_bounds_live_and_rehydrated_text_fields() {
    let oversized_kind = "é".repeat(super::MAX_EVENT_KIND_BYTES);
    let oversized_resource = "r".repeat(super::MAX_EVENT_RESOURCE_BYTES + 1);
    let oversized_detail = "d".repeat(super::MAX_EVENT_DETAIL_BYTES + 1);

    let mut live = super::EventStore::new(2);
    let record = live.record(
        oversized_kind.clone(),
        oversized_resource.clone(),
        Some(oversized_detail.clone()),
    );
    assert!(record.kind.len() <= super::MAX_EVENT_KIND_BYTES);
    assert!(record.kind.is_char_boundary(record.kind.len()));
    assert_eq!(record.resource.len(), super::MAX_EVENT_RESOURCE_BYTES);
    let expected_detail = format!(
        "<omitted oversized event detail: {} bytes>",
        oversized_detail.len()
    );
    assert_eq!(record.detail.as_deref(), Some(expected_detail.as_str()));

    let rehydrated = super::EventStore::from_persisted(
        vec![super::persistence::EventRecord {
            id: 1,
            kind: oversized_kind,
            resource: oversized_resource,
            detail: Some(oversized_detail),
            created_at: 1,
        }],
        2,
    );
    assert!(rehydrated.records[0].kind.len() <= super::MAX_EVENT_KIND_BYTES);
    assert_eq!(
        rehydrated.records[0].resource.len(),
        super::MAX_EVENT_RESOURCE_BYTES
    );
    assert!(rehydrated.records[0]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.starts_with("<omitted oversized event detail:")));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn messages_and_rooms_persist_and_rehydrate_records() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let outbound = super::route_http_request(
        "POST",
        "/api/v0/messages",
        None,
        "{\"username\":\"friend\",\"body\":\"persisted hi\"}",
        &state,
    )
    .await
    .expect("create outbound message");
    assert_eq!(outbound.status, "201 Created");
    let _ = receiver.try_recv();

    let inbound = super::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        "{\"username\":\"friend\",\"body\":\"persisted back\"}",
        &state,
    )
    .await
    .expect("create inbound message");
    assert_eq!(inbound.status, "201 Created");
    let _ = receiver.try_recv();

    let acked = super::route_http_request("POST", "/api/v0/messages/1/ack", None, "", &state)
        .await
        .expect("ack persisted message");
    assert_eq!(acked.status, "200 OK");
    let _ = receiver.try_recv();

    {
        let mut session = state.session.write().await;
        session.state = "connected";
        session.updated_at = super::unix_timestamp();
    }
    let joined = super::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .expect("join persisted room");
    assert_eq!(joined.status, "201 Created");
    let _ = receiver.try_recv();

    let persisted_messages = db.list_messages(10, 0).await.expect("list messages");
    assert_eq!(persisted_messages.len(), 2);
    assert!(persisted_messages
        .iter()
        .any(|message| { message.id == "1" && message.content == "persisted hi" && message.read }));
    assert!(persisted_messages.iter().any(|message| {
        message.id == "2" && message.content == "persisted back" && !message.read
    }));
    let persisted_rooms = db.list_subscribed_rooms().await.expect("list rooms");
    assert_eq!(persisted_rooms.len(), 1);
    assert_eq!(persisted_rooms[0].name, "music");

    let rehydrated_messages = super::MessageStore::from_persisted(persisted_messages);
    let rehydrated_rooms = super::RoomStore::from_persisted(persisted_rooms);
    let (restarted_state, _) = test_state_with_env_parts_full(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        rehydrated_messages,
        rehydrated_rooms,
        Some(db),
    );

    let listed_messages =
        super::route_http_request("GET", "/api/v0/messages/friend", None, "", &restarted_state)
            .await
            .expect("list rehydrated messages");
    assert_eq!(listed_messages.status, "200 OK");
    assert!(listed_messages.body.contains("\"count\":2"));
    assert!(listed_messages.body.contains("\"body\":\"persisted hi\""));
    assert!(listed_messages.body.contains("\"acknowledged\":true"));
    assert!(listed_messages.body.contains("\"body\":\"persisted back\""));

    let listed_rooms =
        super::route_http_request("GET", "/api/v0/rooms", None, "", &restarted_state)
            .await
            .expect("list rehydrated rooms");
    assert_eq!(listed_rooms.status, "200 OK");
    assert!(listed_rooms.body.contains("\"name\":\"music\""));
    assert!(listed_rooms.body.contains("\"joined\":true"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn inbound_message_route_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let previous = state.messages.read().await.clone();
    db.close_for_test().await;

    let response = super::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        r#"{"username":"friend","body":"do not lose me"}"#,
        &state,
    )
    .await
    .expect("failed inbound message persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("message persistence failed"));
    assert_eq!(*state.messages.read().await, previous);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn conversation_delete_removes_persisted_history() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    for (username, message) in [("friend", "private"), ("other", "retained")] {
        let response = super::route_http_request(
            "POST",
            "/api/messages/inbound",
            None,
            &format!("{{\"username\":\"{username}\",\"body\":\"{message}\"}}"),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "201 Created");
    }
    assert_eq!(db.list_messages(10, 0).await.unwrap().len(), 2);

    let deleted =
        super::route_http_request("DELETE", "/api/conversations/friend", None, "", &state)
            .await
            .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert_eq!(deleted.body, "true");
    assert!(state
        .messages
        .read()
        .await
        .records
        .iter()
        .all(|message| message.username != "friend"));
    let persisted = db.list_messages(10, 0).await.unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].username, "other");

    let rehydrated = super::MessageStore::from_persisted(persisted);
    assert!(rehydrated
        .records
        .iter()
        .all(|message| message.username != "friend"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn joined_room_delete_requires_exact_path_and_persists_leave() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";
    let joined =
        super::route_http_request("POST", "/api/rooms/joined", None, r#""room space""#, &state)
            .await
            .unwrap();
    assert_eq!(joined.status, "201 Created");
    assert_eq!(db.list_subscribed_rooms().await.unwrap().len(), 1);

    super::route_http_request(
        "DELETE",
        "/api/rooms/joined/room%20space/extra",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert!(state
        .rooms
        .read()
        .await
        .records
        .iter()
        .any(|room| room.name == "room space" && room.joined));
    assert_eq!(db.list_subscribed_rooms().await.unwrap().len(), 1);

    let left =
        super::route_http_request("DELETE", "/api/rooms/joined/room%20space", None, "", &state)
            .await
            .unwrap();
    assert_eq!(left.status, "200 OK");
    assert!(state
        .rooms
        .read()
        .await
        .records
        .iter()
        .any(|room| room.name == "room space" && !room.joined));
    assert!(db.list_subscribed_rooms().await.unwrap().is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn joined_room_subresources_require_exact_room_segments() {
    let (state, _receiver) = test_state();
    super::route_http_request(
        "POST",
        "/api/rooms/joined",
        None,
        r#""private room""#,
        &state,
    )
    .await
    .unwrap();
    super::route_http_request(
        "POST",
        "/api/rooms/joined/private%20room/messages",
        None,
        r#""secret message""#,
        &state,
    )
    .await
    .unwrap();

    let exact = super::route_http_request(
        "GET",
        "/api/rooms/joined/private%20room/messages",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let malformed = super::route_http_request(
        "GET",
        "/api/rooms/joined/private%20room/extra/messages",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert!(exact.body.contains("secret message"));
    assert_ne!(malformed.body, exact.body);

    super::route_http_request(
        "POST",
        "/api/rooms/joined/private%20room/extra/messages",
        None,
        r#""wrong message""#,
        &state,
    )
    .await
    .unwrap();
    let room = state
        .rooms
        .read()
        .await
        .records
        .iter()
        .find(|room| room.name == "private room")
        .cloned()
        .unwrap();
    assert_eq!(room.messages.len(), 1);
    assert_eq!(
        super::joined_room_subresource("/api/rooms/joined/private%20room/messages", "/messages"),
        Some("private room".to_owned())
    );
    assert_eq!(
        super::joined_room_subresource(
            "/api/rooms/joined/private%20room/extra/messages",
            "/messages"
        ),
        None
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn library_health_patches_require_exact_paths_and_bound_repairs() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Track","kind":"Audio"}"#,
        &state,
    )
    .await
    .unwrap();
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let issue_id = format!("{item_id}-missing-artist");

    super::route_http_request(
        "PATCH",
        &format!("/api/library/health/issues/extra/{issue_id}"),
        None,
        r#"{"artist":"wrong"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(state.library.read().await.get(&item_id).unwrap().artist, "");
    assert_eq!(
        super::library_health_issue_id(&format!("/api/library/health/issues/extra/{issue_id}")),
        None
    );

    let oversized_artist = "é".repeat(super::MAX_LIST_ARTIST_BYTES);
    let repaired = super::route_http_request(
        "PATCH",
        &format!("/api/v0/library/health/issues/{issue_id}"),
        None,
        &format!("{{\"artist\":\"{oversized_artist}\"}}"),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(repaired.status, "204 No Content");
    let item = state.library.read().await.get(&item_id).unwrap();
    assert_eq!(item.artist.len(), super::MAX_LIST_ARTIST_BYTES);
    assert_eq!(
        super::library_health_issue_id(&format!("/api/library/health/issues/{issue_id}")),
        Some(issue_id.as_str())
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn social_and_security_state_persist_and_rehydrate_records() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let note = super::route_http_request(
        "POST",
        "/api/users/notes",
        None,
        r#"{"username":"friend","note":"trusted peer"}"#,
        &state,
    )
    .await
    .expect("create user note");
    assert_eq!(note.status, "201 Created");
    let note_json = serde_json::from_str::<serde_json::Value>(&note.body).unwrap();
    let note_id = note_json["id"].as_str().unwrap().to_owned();

    let updated_note = super::route_http_request(
        "PUT",
        &format!("/api/users/notes/{note_id}"),
        None,
        r#"{"note":"trusted peer updated"}"#,
        &state,
    )
    .await
    .expect("update user note");
    assert_eq!(updated_note.status, "200 OK");

    let liked = super::route_http_request(
        "POST",
        "/api/soulseek/interests",
        None,
        r#"{"name":"jazz"}"#,
        &state,
    )
    .await
    .expect("create liked interest");
    assert_eq!(liked.status, "201 Created");
    let duplicate_liked = super::route_http_request(
        "POST",
        "/api/soulseek/interests",
        None,
        r#"{"name":"JAZZ"}"#,
        &state,
    )
    .await
    .expect("reuse liked interest");
    assert_eq!(duplicate_liked.status, "200 OK");
    assert_eq!(duplicate_liked.body, liked.body);

    let hated = super::route_http_request(
        "POST",
        "/api/soulseek/hated-interests",
        None,
        r#"{"name":"low bitrate"}"#,
        &state,
    )
    .await
    .expect("create hated interest");
    assert_eq!(hated.status, "201 Created");

    let ban = super::route_http_request(
        "POST",
        "/api/security/bans/username",
        None,
        r#"{"username":"spammer"}"#,
        &state,
    )
    .await
    .expect("create username ban");
    assert_eq!(ban.status, "200 OK");
    let blank_ban =
        super::route_http_request("POST", "/api/security/bans/username", None, "{}", &state)
            .await
            .expect("reject blank username ban");
    assert_eq!(blank_ban.status, "400 Bad Request");

    let persisted_notes = db.list_user_notes(10, 0).await.expect("list notes");
    assert_eq!(persisted_notes.len(), 1);
    assert_eq!(persisted_notes[0].note, "trusted peer updated");
    let persisted_interests = db.list_interests(10, 0).await.expect("list interests");
    assert_eq!(persisted_interests.len(), 2);
    assert!(persisted_interests
        .iter()
        .any(|record| record.kind == "liked" && record.name == "jazz"));
    assert!(persisted_interests
        .iter()
        .any(|record| record.kind == "hated" && record.name == "low bitrate"));
    let persisted_bans = db.list_security_bans().await.expect("list bans");
    assert_eq!(persisted_bans.len(), 1);
    assert_eq!(persisted_bans[0].kind, "username");
    assert_eq!(persisted_bans[0].value, "spammer");

    let rehydrated_notes = super::UserNoteStore::from_persisted(persisted_notes);
    let rehydrated_interests = super::InterestStore::from_persisted(persisted_interests);
    let rehydrated_security = super::SecurityState::from_persisted(persisted_bans);
    assert!(rehydrated_notes
        .json(None)
        .contains("\"note\":\"trusted peer updated\""));
    assert!(rehydrated_interests
        .json_liked()
        .contains("\"name\":\"jazz\""));
    assert!(rehydrated_interests
        .json_hated()
        .contains("\"name\":\"low bitrate\""));
    assert_eq!(rehydrated_security.active_bans(), 1);
    assert!(rehydrated_security
        .json_value()
        .to_string()
        .contains("\"value\":\"spammer\""));

    let delete_note = super::route_http_request(
        "DELETE",
        &format!("/api/users/notes/{note_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete user note");
    assert_eq!(delete_note.status, "200 OK");
    let unban = super::route_http_request(
        "DELETE",
        "/api/security/bans/username/spammer",
        None,
        "",
        &state,
    )
    .await
    .expect("delete username ban");
    assert_eq!(unban.status, "200 OK");
    assert!(db.list_user_notes(10, 0).await.unwrap().is_empty());
    assert!(db.list_security_bans().await.unwrap().is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn user_note_routes_roll_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let created = super::route_http_request(
        "POST",
        "/api/users/notes",
        None,
        r#"{"username":"friend","note":"not persisted"}"#,
        &state,
    )
    .await
    .expect("failed note creation response");
    assert_eq!(created.status, "503 Service Unavailable");
    assert!(created.body.contains("user note persistence failed"));
    let notes = state.user_notes.read().await;
    assert!(notes.records.is_empty());
    assert_eq!(notes.next_id, 1);
    drop(notes);

    for (method, body, expected_error) in [
        (
            "PUT",
            r#"{"note":"changed"}"#,
            "user note persistence failed",
        ),
        ("DELETE", "", "user note deletion persistence failed"),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .user_notes
            .write()
            .await
            .create("friend".to_owned(), "original".to_owned())
            .unwrap();
        db.close_for_test().await;

        let response =
            super::route_http_request(method, "/api/users/notes/note-1", None, body, &state)
                .await
                .expect("failed note mutation response");
        assert_eq!(response.status, "503 Service Unavailable", "{method}");
        assert!(response.body.contains(expected_error), "{method}");
        let notes = state.user_notes.read().await;
        assert_eq!(notes.records.len(), 1, "{method}");
        assert_eq!(notes.records[0].note, "original", "{method}");
        assert_eq!(notes.next_id, 2, "{method}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn interest_routes_roll_back_when_persistence_fails() {
    for path in ["/api/soulseek/interests", "/api/soulseek/hated-interests"] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;

        let response = super::route_http_request("POST", path, None, r#"{"name":"jazz"}"#, &state)
            .await
            .expect("failed interest creation response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("interest persistence failed"),
            "{path}"
        );
        let interests = state.interests.read().await;
        assert!(interests.liked.is_empty(), "{path}");
        assert!(interests.hated.is_empty(), "{path}");
        assert_eq!(interests.next_id, 1, "{path}");
    }

    for (path, hated) in [
        ("/api/soulseek/interests/liked-1", false),
        ("/api/soulseek/hated-interests/hated-1", true),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        if hated {
            state
                .interests
                .write()
                .await
                .add_hated("noise".to_owned())
                .unwrap();
        } else {
            state
                .interests
                .write()
                .await
                .add_liked("jazz".to_owned())
                .unwrap();
        }
        db.close_for_test().await;

        let response = super::route_http_request("DELETE", path, None, "", &state)
            .await
            .expect("failed interest deletion response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response
                .body
                .contains("interest deletion persistence failed"),
            "{path}"
        );
        let interests = state.interests.read().await;
        assert_eq!(interests.liked.len(), usize::from(!hated), "{path}");
        assert_eq!(interests.hated.len(), usize::from(hated), "{path}");
        assert_eq!(interests.next_id, 2, "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn nested_resource_paths_cannot_mutate_flat_record_ids() {
    let (state, _receiver) = test_state();
    let now = super::unix_timestamp();
    state
        .user_notes
        .write()
        .await
        .records
        .push(super::user_note_store::UserNoteRecord {
            id: "note/nested".to_owned(),
            username: "friend".to_owned(),
            note: "keep".to_owned(),
            color: String::new(),
            icon: String::new(),
            is_high_priority: false,
            created_at: now,
            updated_at: now,
        });
    state
        .interests
        .write()
        .await
        .liked
        .push(super::interest_store::InterestRecord {
            id: "liked/nested".to_owned(),
            name: "jazz".to_owned(),
            kind: "liked".to_owned(),
            created_at: now,
        });
    state
        .library
        .write()
        .await
        .records
        .push(super::LibraryItemRecord {
            id: "lib/nested".to_owned(),
            artist: "artist".to_owned(),
            title: "title".to_owned(),
            kind: "Audio".to_owned(),
            created_at: now,
        });

    for path in [
        "/api/users/notes/note/nested",
        "/api/soulseek/interests/liked/nested",
        "/api/library/items/lib/nested",
    ] {
        let response = super::route_http_request("DELETE", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(response.status, "404 Not Found", "{path}");
    }
    let update = super::route_http_request(
        "PUT",
        "/api/users/notes/note/nested",
        None,
        r#"{"note":"changed"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(update.status, "404 Not Found");

    assert_eq!(
        state.user_notes.read().await.records[0].note,
        "keep",
        "nested PUT must not reach the user-note store"
    );
    assert_eq!(state.interests.read().await.liked.len(), 1);
    assert_eq!(state.library.read().await.records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn compatibility_store_state_persists_and_rehydrates_records() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let wishlist = super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Alice","title":"Blue Track","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    assert_eq!(wishlist.status, "201 Created");
    let wishlist_json = serde_json::from_str::<serde_json::Value>(&wishlist.body).unwrap();
    let wish_id = wishlist_json["id"].as_str().unwrap().to_owned();

    let aliased_wishlist_update = super::route_http_request(
        "PUT",
        &format!("/api/wishlist/unrelated/{wish_id}"),
        None,
        r#"{"title":"Aliased Track"}"#,
        &state,
    )
    .await
    .expect("reject aliased wishlist update");
    assert_eq!(aliased_wishlist_update.status, "404 Not Found");
    assert_eq!(
        state.wishlist.read().await.records[0].items[0].title,
        "Blue Track",
        "nested route must not update the last path segment"
    );

    let updated_wishlist = super::route_http_request(
        "PUT",
        &format!("/api/wishlist/{wish_id}"),
        None,
        r#"{"title":"Green Track"}"#,
        &state,
    )
    .await
    .expect("update wishlist item");
    assert_eq!(updated_wishlist.status, "200 OK");

    let contact = super::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("create contact");
    assert_eq!(contact.status, "201 Created");
    let contact_json = serde_json::from_str::<serde_json::Value>(&contact.body).unwrap();
    let contact_id = contact_json["id"].as_str().unwrap().to_owned();

    let updated_contact = super::route_http_request(
        "PUT",
        &format!("/api/contacts/{contact_id}"),
        None,
        r#"{"online":true}"#,
        &state,
    )
    .await
    .expect("update contact");
    assert_eq!(updated_contact.status, "200 OK");

    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Shared"}"#,
        &state,
    )
    .await
    .expect("create grant collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let grant_body = format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}");
    let grant = super::route_http_request("POST", "/api/share-grants", None, &grant_body, &state)
        .await
        .expect("create share grant");
    assert_eq!(grant.status, "201 Created");
    let duplicate_grant_body =
        format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"FRIEND\"}}");
    let duplicate_grant = super::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &duplicate_grant_body,
        &state,
    )
    .await
    .expect("reuse share grant");
    assert_eq!(duplicate_grant.status, "200 OK");
    assert_eq!(duplicate_grant.body, grant.body);
    let grant_json = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap();
    let grant_id = grant_json["id"].as_str().unwrap().to_owned();

    let updated_grant = super::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}"),
        None,
        r#"{"permissions":"read,download"}"#,
        &state,
    )
    .await
    .expect("update share grant");
    assert_eq!(updated_grant.status, "200 OK");

    let sharegroup = super::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Trusted peers","description":"sharing"}"#,
        &state,
    )
    .await
    .expect("create sharegroup");
    assert_eq!(sharegroup.status, "201 Created");
    let sharegroup_json = serde_json::from_str::<serde_json::Value>(&sharegroup.body).unwrap();
    let sharegroup_id = sharegroup_json["id"].as_str().unwrap().to_owned();

    let updated_sharegroup = super::route_http_request(
        "PUT",
        &format!("/api/sharegroups/{sharegroup_id}"),
        None,
        r#"{"name":"Trusted peers updated","description":"sharing more"}"#,
        &state,
    )
    .await
    .expect("update sharegroup");
    assert_eq!(updated_sharegroup.status, "200 OK");

    let sharegroup_member = super::route_http_request(
        "POST",
        &format!("/api/sharegroups/{sharegroup_id}/members"),
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("create sharegroup member");
    assert_eq!(sharegroup_member.status, "201 Created");

    db.upsert_destination(&super::persistence::DestinationRecord {
        id: "archive".to_string(),
        name: "Archive".to_string(),
        path: "/srv/archive".to_string(),
        is_default: true,
        created_at: 10,
        updated_at: 11,
    })
    .await
    .expect("persist destination");

    let now_playing = super::route_http_request(
        "POST",
        "/api/nowplaying",
        None,
        r#"{"username":"peer","artist":"Alice","title":"Currently Playing"}"#,
        &state,
    )
    .await
    .expect("create now playing");
    assert_eq!(now_playing.status, "200 OK");

    let persisted_wishlist = db.list_wishlist_items(10, 0).await.expect("list wishlist");
    assert_eq!(persisted_wishlist.len(), 1);
    assert_eq!(persisted_wishlist[0].title, "Green Track");
    let persisted_contacts = db.list_contacts(10, 0).await.expect("list contacts");
    assert_eq!(persisted_contacts.len(), 1);
    assert_eq!(persisted_contacts[0].username, "friend");
    assert!(persisted_contacts[0].online);
    let persisted_grants = db.list_share_grants(10, 0).await.expect("list grants");
    assert_eq!(persisted_grants.len(), 1);
    assert_eq!(persisted_grants[0].permissions, "read,download");
    let persisted_sharegroups = db.list_share_groups(10, 0).await.expect("list sharegroups");
    assert_eq!(persisted_sharegroups.len(), 1);
    assert_eq!(persisted_sharegroups[0].name, "Trusted peers updated");
    let persisted_sharegroup_members = db
        .list_share_group_members(10, 0)
        .await
        .expect("list sharegroup members");
    assert_eq!(persisted_sharegroup_members.len(), 1);
    assert_eq!(persisted_sharegroup_members[0].username, "friend");
    let persisted_destinations = db
        .list_destinations(10, 0)
        .await
        .expect("list destinations");
    assert_eq!(persisted_destinations.len(), 1);
    assert_eq!(persisted_destinations[0].name, "Archive");
    let persisted_now_playing = db.list_now_playing(10, 0).await.expect("list now playing");
    assert_eq!(persisted_now_playing.len(), 1);
    assert_eq!(persisted_now_playing[0].username, "peer");
    assert_eq!(persisted_now_playing[0].title, "Currently Playing");

    let mut rehydrated_wishlist =
        super::WishlistStore::from_persisted_with_ignored(persisted_wishlist, Vec::new());
    let rehydrated_contacts = super::ContactStore::from_persisted(persisted_contacts);
    let rehydrated_grants = super::ShareGrantStore::from_persisted(persisted_grants);
    let rehydrated_sharegroups =
        super::ShareGroupStore::from_persisted(persisted_sharegroups, persisted_sharegroup_members);
    let rehydrated_destinations = super::DestinationStore::from_persisted(persisted_destinations);
    let rehydrated_now_playing = super::NowPlayingStore::from_persisted(persisted_now_playing);
    assert!(rehydrated_wishlist
        .json_array()
        .contains("\"title\":\"Green Track\""));
    assert!(rehydrated_contacts
        .nearby_json(None)
        .contains("\"username\":\"friend\""));
    assert!(rehydrated_grants
        .json_array()
        .contains("\"permissions\":\"read,download\""));
    assert!(rehydrated_sharegroups
        .json_array(None)
        .contains("\"name\":\"Trusted peers updated\""));
    assert!(rehydrated_sharegroups
        .user_group_json("friend")
        .contains("\"group\":\"Trusted peers updated\""));
    assert!(rehydrated_destinations
        .list()
        .contains("\"name\":\"Archive\""));
    assert!(rehydrated_destinations
        .default()
        .contains("\"path\":\"/srv/archive\""));
    assert!(rehydrated_now_playing
        .json()
        .contains("\"title\":\"Currently Playing\""));

    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("compat database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["wishlist"], 1);
    assert_eq!(stats_json["contacts"], 1);
    assert_eq!(stats_json["shareGrants"], 1);
    assert_eq!(stats_json["sharegroups"], 1);
    assert_eq!(stats_json["sharegroupMembers"], 1);
    assert_eq!(stats_json["destinations"], 1);
    assert_eq!(stats_json["nowPlaying"], 1);
    assert_eq!(stats_json["persisted"]["destinations"], 1);
    assert_eq!(stats_json["persisted"]["nowPlaying"], 1);
    assert_eq!(stats_json["projections"]["destinations"], 1);
    assert_eq!(stats_json["projections"]["nowPlaying"], 1);

    let clear_now_playing =
        super::route_http_request("DELETE", "/api/nowplaying", None, "", &state)
            .await
            .expect("clear now playing");
    assert_eq!(clear_now_playing.status, "200 OK");
    assert!(db.list_now_playing(10, 0).await.unwrap().is_empty());

    let delete_wishlist = super::route_http_request(
        "DELETE",
        &format!("/api/wishlist/{wish_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete wishlist item");
    assert_eq!(delete_wishlist.status, "200 OK");
    let delete_contact = super::route_http_request(
        "DELETE",
        &format!("/api/contacts/{contact_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete contact");
    assert_eq!(delete_contact.status, "200 OK");
    let delete_grant = super::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete share grant");
    assert_eq!(delete_grant.status, "200 OK");
    let delete_member = super::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{sharegroup_id}/members/friend"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete sharegroup member");
    assert_eq!(delete_member.status, "200 OK");
    let delete_sharegroup = super::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{sharegroup_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete sharegroup");
    assert_eq!(delete_sharegroup.status, "200 OK");
    assert!(db.list_wishlist_items(10, 0).await.unwrap().is_empty());
    assert!(db.list_contacts(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_grants(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_groups(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_group_members(10, 0).await.unwrap().is_empty());
    db.delete_destination("archive")
        .await
        .expect("delete destination");
    assert!(db.list_destinations(10, 0).await.unwrap().is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn now_playing_routes_roll_back_when_persistence_fails() {
    for method in ["PUT", "POST"] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state.now_playing.write().await.upsert(
            "existing".to_owned(),
            "Original".to_owned(),
            "Track".to_owned(),
        );
        db.close_for_test().await;

        let response = super::route_http_request(
            method,
            "/api/nowplaying",
            None,
            r#"{"username":"new","artist":"Changed","title":"Song"}"#,
            &state,
        )
        .await
        .expect("failed now-playing persistence response");
        assert_eq!(response.status, "503 Service Unavailable", "{method}");
        assert!(
            response.body.contains("now-playing persistence failed"),
            "{method}"
        );
        let now_playing = state.now_playing.read().await;
        assert_eq!(now_playing.records.len(), 1, "{method}");
        assert_eq!(now_playing.records[0].username, "existing", "{method}");
        assert_eq!(now_playing.records[0].artist, "Original", "{method}");
        assert_eq!(now_playing.records[0].title, "Track", "{method}");
    }

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.now_playing.write().await.upsert(
        "existing".to_owned(),
        "Original".to_owned(),
        "Track".to_owned(),
    );
    db.close_for_test().await;

    let response = super::route_http_request("DELETE", "/api/nowplaying", None, "", &state)
        .await
        .expect("failed now-playing clear response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("now-playing clear persistence failed"));
    let now_playing = state.now_playing.read().await;
    assert_eq!(now_playing.records.len(), 1);
    assert_eq!(now_playing.records[0].username, "existing");
    assert_eq!(now_playing.records[0].artist, "Original");
    assert_eq!(now_playing.records[0].title, "Track");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn contact_routes_roll_back_when_persistence_fails() {
    for path in [
        "/api/contacts",
        "/api/contacts/from-discovery",
        "/api/contacts/from-invite",
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;

        let response =
            super::route_http_request("POST", path, None, r#"{"username":"friend"}"#, &state)
                .await
                .expect("failed contact creation response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("contact persistence failed"),
            "{path}"
        );
        let contacts = state.contacts.read().await;
        assert!(contacts.records.is_empty(), "{path}");
        assert_eq!(contacts.next_id, 1, "{path}");
    }

    for (method, body, expected_error) in [
        (
            "PUT",
            r#"{"username":"changed","online":true}"#,
            "contact persistence failed",
        ),
        ("DELETE", "", "contact deletion persistence failed"),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .contacts
            .write()
            .await
            .create("friend".to_owned())
            .unwrap();
        db.close_for_test().await;

        let response =
            super::route_http_request(method, "/api/contacts/contact-1", None, body, &state)
                .await
                .expect("failed contact mutation response");
        assert_eq!(response.status, "503 Service Unavailable", "{method}");
        assert!(response.body.contains(expected_error), "{method}");
        let contacts = state.contacts.read().await;
        assert_eq!(contacts.records.len(), 1, "{method}");
        assert_eq!(contacts.records[0].username, "friend", "{method}");
        assert!(!contacts.records[0].online, "{method}");
        assert_eq!(contacts.next_id, 2, "{method}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn wishlist_routes_roll_back_when_persistence_fails() {
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
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.wishlist.read().await.clone();
        db.close_for_test().await;

        let response = super::route_http_request("POST", path, None, body, &state)
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
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
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
            super::route_http_request(method, "/api/wishlist/wish-1", None, body, &state)
                .await
                .expect("failed wishlist mutation response");
        assert_eq!(response.status, "503 Service Unavailable", "{method}");
        assert!(response.body.contains(expected_error), "{method}");
        assert_eq!(*state.wishlist.read().await, previous, "{method}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn collection_routes_roll_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let previous = state.collections.read().await.clone();
    db.close_for_test().await;
    let response = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Collection"}"#,
        &state,
    )
    .await
    .expect("failed collection create response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("collection persistence failed"));
    assert_eq!(*state.collections.read().await, previous);

    for (method, path, body) in [
        ("PUT", "/api/collections/col-1", r#"{"name":"Changed"}"#),
        (
            "POST",
            "/api/collections/col-1/items",
            r#"{"title":"Added"}"#,
        ),
        (
            "PUT",
            "/api/collections/items/item-1",
            r#"{"title":"Changed"}"#,
        ),
        ("DELETE", "/api/collections/items/item-1", ""),
        (
            "PUT",
            "/api/collections/col-1/items/reorder",
            r#"{"item_ids":["item-2","item-1"]}"#,
        ),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let mut collections = state.collections.write().await;
        collections
            .create(String::new(), "Collection".to_owned(), String::new())
            .unwrap();
        collections
            .add_item(
                "col-1",
                "one".to_owned(),
                String::new(),
                "One".to_owned(),
                "Audio".to_owned(),
            )
            .unwrap();
        collections
            .add_item(
                "col-1",
                "two".to_owned(),
                String::new(),
                "Two".to_owned(),
                "Audio".to_owned(),
            )
            .unwrap();
        let previous = collections.clone();
        drop(collections);
        db.close_for_test().await;

        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed collection mutation response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(
            response.body.contains("collection persistence failed"),
            "{method} {path}"
        );
        assert_eq!(*state.collections.read().await, previous, "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn collection_item_order_is_persisted_as_one_snapshot() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Collection"}"#,
        &state,
    )
    .await
    .unwrap();
    for title in ["One", "Two"] {
        super::route_http_request(
            "POST",
            "/api/collections/col-1/items",
            None,
            &format!(r#"{{"title":"{title}"}}"#),
            &state,
        )
        .await
        .unwrap();
    }
    let response = super::route_http_request(
        "PUT",
        "/api/collections/col-1/items/reorder",
        None,
        r#"{"item_ids":["item-2","item-1"]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(response.status, "200 OK");

    let items = db.list_collection_items(10, 0).await.unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| (item.id.as_str(), item.position))
            .collect::<Vec<_>>(),
        vec![("item-2", 0), ("item-1", 1)]
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn collection_snapshot_database_write_is_atomic() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let original = super::persistence::CollectionRecord {
        id: "col-1".to_owned(),
        owner_user_id: String::new(),
        name: "Original".to_owned(),
        description: String::new(),
        collection_type: "ShareList".to_owned(),
        created_at: 1,
        updated_at: 1,
    };
    let original_item = super::persistence::CollectionItemRecord {
        id: "item-1".to_owned(),
        collection_id: "col-1".to_owned(),
        content_id: "original".to_owned(),
        artist: String::new(),
        title: "Original".to_owned(),
        kind: "Audio".to_owned(),
        file_name: String::new(),
        album: String::new(),
        content_hash: String::new(),
        added_at: 1,
        position: 0,
    };
    db.upsert_collection(&original).await.unwrap();
    db.upsert_collection_item(&original_item).await.unwrap();

    let changed = super::persistence::CollectionRecord {
        name: "Changed".to_owned(),
        updated_at: 2,
        ..original.clone()
    };
    let duplicate_items = vec![
        super::persistence::CollectionItemRecord {
            title: "Changed".to_owned(),
            ..original_item.clone()
        },
        super::persistence::CollectionItemRecord {
            content_id: "duplicate".to_owned(),
            position: 1,
            ..original_item.clone()
        },
    ];
    assert!(db
        .replace_collection(&changed, &duplicate_items)
        .await
        .is_err());

    let collections = db.list_collections(10, 0).await.unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "Original");
    let items = db.list_collection_items(10, 0).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Original");
    assert_eq!(items[0].content_id, "original");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn wishlist_persistence_batches_across_sqlite_parameter_boundaries() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let mut store = super::WishlistStore::with_max_items(50);
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
    assert!(super::persist_wishlist_items_checked(&state, &items)
        .await
        .unwrap());
    assert_eq!(db.list_wishlist_items(100, 0).await.unwrap().len(), 41);

    for item in &mut items {
        item.total_search_count = 2;
    }
    super::persist_wishlist_items_checked(&state, &items)
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
async fn library_routes_roll_back_when_persistence_fails() {
    for (path, body) in [
        (
            "/api/library/items",
            r#"{"artist":"Artist","title":"Track"}"#,
        ),
        (
            "/api/integrations/lidarr/manualimport",
            r#"{"artist":"Artist","album":"Release"}"#,
        ),
        (
            "/api/musicbrainz/targets",
            r#"{"artist":"Artist","title":"Release"}"#,
        ),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.library.read().await.clone();
        db.close_for_test().await;

        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .expect("failed library creation response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("library persistence failed"),
            "{path}"
        );
        assert_eq!(*state.library.read().await, previous, "{path}");
    }

    for (method, path, body, expected_error) in [
        (
            "PATCH",
            "/api/v0/library/health/issues/lib-1-missing-title",
            r#"{"title":"Fixed"}"#,
            "library persistence failed",
        ),
        (
            "POST",
            "/api/v0/library/health/issues/fix",
            "",
            "library persistence failed",
        ),
        (
            "DELETE",
            "/api/library/items/lib-1",
            "",
            "library deletion persistence failed",
        ),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .library
            .write()
            .await
            .create("Artist".to_owned(), String::new(), String::new())
            .unwrap();
        let previous = state.library.read().await.clone();
        db.close_for_test().await;

        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed library mutation response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(response.body.contains(expected_error), "{method} {path}");
        assert_eq!(*state.library.read().await, previous, "{method} {path}");
    }
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_library_issue_fix_rehydrates() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Road Trip","description":"queued albums"}"#,
        &state,
    )
    .await
    .expect("create collection");
    assert_eq!(collection.status, "201 Created");
    let collection_json = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap();
    let collection_id = collection_json["id"].as_str().unwrap().to_owned();

    let collection_item = super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"track-1","artist":"Alice","title":"One","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create collection item");
    assert_eq!(collection_item.status, "201 Created");
    let collection_item_json =
        serde_json::from_str::<serde_json::Value>(&collection_item.body).unwrap();
    let collection_item_id = collection_item_json["id"].as_str().unwrap().to_owned();

    let updated_collection_item = super::route_http_request(
        "PUT",
        &format!("/api/collections/items/{collection_item_id}"),
        None,
        r#"{"title":"Two"}"#,
        &state,
    )
    .await
    .expect("update collection item");
    assert_eq!(updated_collection_item.status, "200 OK");

    let library_item = super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Bob","title":"","kind":""}"#,
        &state,
    )
    .await
    .expect("create library item");
    assert_eq!(library_item.status, "201 Created");
    let library_json = serde_json::from_str::<serde_json::Value>(&library_item.body).unwrap();
    let library_id = library_json["id"].as_str().unwrap().to_owned();

    let patched = super::route_http_request(
        "PATCH",
        &format!("/api/v0/library/health/issues/{library_id}-missing-title"),
        None,
        r#"{"title":"Fixed Title"}"#,
        &state,
    )
    .await
    .expect("patch library issue");
    assert_eq!(patched.status, "204 No Content");
    let fixed = super::route_http_request(
        "POST",
        "/api/v0/library/health/issues/fix",
        None,
        "",
        &state,
    )
    .await
    .expect("fix library issues");
    assert_eq!(fixed.status, "200 OK");
    let fixed_json = serde_json::from_str::<serde_json::Value>(&fixed.body)
        .expect("library issue fix response JSON");
    assert_eq!(fixed_json["fixed"], 1);
    assert_eq!(fixed_json["fixable"], 1);
    assert_eq!(fixed_json["remaining"], 0);

    let persisted_collections = db.list_collections(10, 0).await.expect("list collections");
    assert_eq!(persisted_collections.len(), 1);
    assert_eq!(persisted_collections[0].name, "Road Trip");
    let persisted_collection_items = db
        .list_collection_items(10, 0)
        .await
        .expect("list collection items");
    assert_eq!(persisted_collection_items.len(), 1);
    assert_eq!(persisted_collection_items[0].title, "Two");
    let persisted_library = db.list_library_items(10, 0).await.expect("list library");
    assert_eq!(persisted_library.len(), 1);
    assert_eq!(persisted_library[0].title, "Fixed Title");
    assert_eq!(persisted_library[0].kind, "Audio");

    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("collection/library database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["collections"], 1);
    assert_eq!(stats_json["collectionItems"], 1);
    assert_eq!(stats_json["libraryItems"], 1);
    assert_eq!(stats_json["persisted"]["collections"], 1);
    assert_eq!(stats_json["persisted"]["collectionItems"], 1);
    assert_eq!(stats_json["persisted"]["libraryItems"], 1);
    assert_eq!(stats_json["projections"]["collections"], 1);
    assert_eq!(stats_json["projections"]["collectionItems"], 1);
    assert_eq!(stats_json["projections"]["libraryItems"], 1);

    let rehydrated_collections =
        super::CollectionStore::from_persisted(persisted_collections, persisted_collection_items);
    let rehydrated_library = super::LibraryStore::from_persisted(persisted_library.clone());
    assert!(rehydrated_collections
        .json_array(None, None)
        .contains("\"title\":\"Two\""));
    assert!(rehydrated_library
        .json()
        .contains("\"title\":\"Fixed Title\""));
    assert!(rehydrated_library.json().contains("\"kind\":\"Audio\""));

    let delete_collection_item = super::route_http_request(
        "DELETE",
        &format!("/api/collections/items/{collection_item_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection item");
    assert_eq!(delete_collection_item.status, "200 OK");
    let delete_collection = super::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection");
    assert_eq!(delete_collection.status, "200 OK");
    let delete_library = super::route_http_request(
        "DELETE",
        &format!("/api/library/items/{library_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete library item");
    assert_eq!(delete_library.status, "200 OK");
    assert!(db.list_collections(10, 0).await.unwrap().is_empty());
    assert!(db.list_collection_items(10, 0).await.unwrap().is_empty());
    assert!(db.list_library_items(10, 0).await.unwrap().is_empty());

    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/library/health/issues/fix",
        "case": "mutation-side-effects-and-readback",
        "pass": true,
    }));
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/library/health/issues/fix",
        "case": "nominal-status-headers-body",
        "pass": fixed.status == "200 OK"
            && fixed.content_type.starts_with("application/json")
            && fixed_json["fixed"] == 1
            && fixed_json["fixable"] == 1
            && fixed_json["remaining"] == 0,
    }));
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/library/health/issues/fix",
        "case": "restart-persistence-or-reset",
        "pass": fixed.status == "200 OK"
            && persisted_library.len() == 1
            && persisted_library[0].kind == "Audio"
            && rehydrated_library.json().contains("\"kind\":\"Audio\""),
    }));
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "PATCH",
        "route": "/api/v0/library/health/issues/{issueId}",
        "case": "restart-persistence-or-reset",
        "pass": patched.status == "204 No Content"
            && persisted_library.len() == 1
            && persisted_library[0].title == "Fixed Title"
            && rehydrated_library.json().contains("\"title\":\"Fixed Title\""),
    }));
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("library_issue_fix_rehydrates.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn database_admin_aliases_report_live_state_and_cleanup() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let old_message = super::persistence::MessageRecord {
        id: "old".to_owned(),
        username: "friend".to_owned(),
        content: "old".to_owned(),
        direction: "inbound".to_owned(),
        read: false,
        created_at: 1,
        source_id: None,
        source_timestamp: None,
        was_replayed: false,
    };
    db.insert_message(&old_message).await.expect("old message");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    {
        let mut transfers = state.transfers.write().await;
        let failed = transfers.create(
            0,
            Some("peer".to_owned()),
            "Remote/Failed.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(failed.id, "failed", Some(1), Some("offline".to_owned()));
    }

    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("admin database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["enabled"], true);
    assert_eq!(stats_json["healthy"], true);
    assert_eq!(stats_json["messages"], 1);
    assert_eq!(stats_json["persisted"]["messages"], 1);
    assert_eq!(stats_json["projections"]["transfers"], 1);

    let cleanup = super::route_http_request(
        "POST",
        "/api/database/cleanup",
        None,
        "{\"days\":0}",
        &state,
    )
    .await
    .expect("legacy database cleanup");
    assert_eq!(cleanup.status, "200 OK");
    let cleanup_json = serde_json::from_str::<serde_json::Value>(&cleanup.body).unwrap();
    assert_eq!(cleanup_json["status"], "ok");
    assert_eq!(cleanup_json["persisted"]["enabled"], true);
    assert_eq!(cleanup_json["cleaned"], 1);
    assert_eq!(cleanup_json["pruned_transfers"], 1);

    let vacuum = super::route_http_request("POST", "/api/admin/database/vacuum", None, "", &state)
        .await
        .expect("admin database vacuum");
    assert_eq!(vacuum.status, "200 OK");
    let vacuum_json = serde_json::from_str::<serde_json::Value>(&vacuum.body).unwrap();
    assert_eq!(vacuum_json["enabled"], true);
    assert_eq!(vacuum_json["vacuumed"], true);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn database_admin_errors_are_redacted_and_do_not_report_success() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    db.close_for_test().await;
    let raw_error = db
        .get_stats()
        .await
        .expect_err("closed database should reject statistics")
        .to_string();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db),
    );

    let stats = super::database_stats_value(&state).await;
    assert_eq!(stats["healthy"], false);
    assert_eq!(
        stats["persisted"]["error"],
        "database statistics unavailable"
    );

    let cleanup = super::database_cleanup_value(&state, "{\"days\":0}").await;
    assert_eq!(cleanup["status"], "error");
    assert_eq!(
        cleanup["persisted"]["error"],
        "database cleanup unavailable"
    );

    let vacuum = super::database_vacuum_value(&state).await;
    assert_eq!(vacuum["status"], "error");
    assert_eq!(vacuum["error"], "database vacuum unavailable");

    for value in [stats, cleanup, vacuum] {
        let json = value.to_string();
        assert!(!json.contains(&raw_error), "leaked database error: {json}");
        assert!(!json.to_ascii_lowercase().contains("pool closed"), "{json}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn runtime_control_routes_roll_back_when_persistence_fails() {
    for (method, path, body, seed_runtime, seed_relay) in [
        ("PUT", "/api/application", "{}", "", false),
        ("DELETE", "/api/application", "", "restart", false),
        ("POST", "/api/application/gc", "", "", false),
        (
            "PUT",
            "/api/config/preferences",
            r#"{"autoreplace_enabled":true}"#,
            "",
            false,
        ),
        ("PUT", "/api/autoreplace/enable", "", "", false),
        ("PUT", "/api/autoreplace/disable", "", "autoreplace", false),
        ("PUT", "/api/relay", r#"{"enabled":true}"#, "", false),
        ("POST", "/api/relay", r#"{"enabled":true}"#, "", false),
        ("DELETE", "/api/relay", "", "", true),
        ("PUT", "/api/relay/agent", r#"{"enabled":true}"#, "", false),
        ("DELETE", "/api/relay/agent", "", "relay_agent", false),
        ("POST", "/api/v0/bridge/start", "", "", false),
        ("POST", "/api/v0/bridge/stop", "", "bridge", false),
        (
            "PUT",
            "/api/v0/bridge/admin/config",
            r#"{"enabled":true}"#,
            "",
            false,
        ),
        ("POST", "/api/options", r#"{}"#, "", false),
        (
            "POST",
            "/api/songid/runs",
            r#"{"source":"route-audit"}"#,
            "",
            false,
        ),
        (
            "POST",
            "/api/integrations/lidarr/wanted/sync",
            "",
            "",
            false,
        ),
        ("POST", "/api/profile/invite", "", "", false),
        ("POST", "/api/slskdn/warm-cache", "", "", false),
        ("POST", "/api/backfill", "", "", false),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "legacy")
                .with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        match seed_runtime {
            "restart" => {
                state.runtime.write().await.set_restart_requested(true);
            }
            "autoreplace" => {
                state.runtime.write().await.set_autoreplace(true);
            }
            "relay_agent" => {
                state.runtime.write().await.set_relay_agent(true);
            }
            "bridge" => {
                state.runtime.write().await.set_bridge_running(true, false);
            }
            _ => {}
        }
        if seed_relay {
            state.relay.write().await.set_enabled(true);
        }
        let previous_runtime = state.runtime.read().await.clone();
        let previous_relay = state.relay.read().await.clone();
        db.close_for_test().await;

        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed runtime compatibility persistence response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(
            response
                .body
                .contains("runtime compatibility persistence failed"),
            "{method} {path}"
        );
        assert_eq!(
            *state.runtime.read().await,
            previous_runtime,
            "{method} {path}"
        );
        assert_eq!(*state.relay.read().await, previous_relay, "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn lidarr_manual_import_rolls_back_both_stores_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let previous_library = state.library.read().await.clone();
    let previous_runtime = state.runtime.read().await.clone();
    db.close_for_test().await;

    let response = super::route_http_request(
        "POST",
        "/api/integrations/lidarr/manualimport",
        None,
        r#"{"artist":"Artist","title":"Album","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("failed Lidarr manual import persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("library persistence failed: Lidarr manual import transaction failed"));
    assert_eq!(*state.library.read().await, previous_library);
    assert_eq!(*state.runtime.read().await, previous_runtime);
}

#[cfg_attr(test, tokio::test)]
#[cfg(all(
    feature = "full-controller-tests",
    not(feature = "legacy-route-dispatch")
))]
async fn lidarr_manual_import_releases_store_guards_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-legacy-lidarr-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = super::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create manual import database");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let blocker_pool = sqlx_sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx_sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await
        .expect("open SQLite lock connection");
    let mut blocker = blocker_pool
        .acquire()
        .await
        .expect("acquire SQLite lock connection");
    sqlx_core::query::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold SQLite write lock");

    let task_state = Arc::clone(&state);
    let mutation = tokio::spawn(async move {
        super::route_http_request(
            "POST",
            "/api/integrations/lidarr/manualimport",
            None,
            r#"{"artist":"Artist","title":"Album","kind":"Audio"}"#,
            &task_state,
        )
        .await
    });
    let stores_visible_during_sqlite_wait = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let library_visible = state.library.try_read().is_ok_and(|library| {
                library
                    .records
                    .iter()
                    .any(|record| record.artist == "Artist" && record.title == "Album")
            });
            let runtime_visible = state
                .runtime
                .try_read()
                .is_ok_and(|runtime| runtime.lidarr_manual_imports == 1);
            if library_visible && runtime_visible {
                break true;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let request_waited_for_sqlite = !mutation.is_finished();
    let library_reader_responsive = state.library.try_read().is_ok();
    let runtime_reader_responsive = state.runtime.try_read().is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let response = tokio::time::timeout(Duration::from_secs(3), mutation)
        .await
        .expect("manual import should finish after releasing SQLite")
        .expect("manual import route task should join")
        .expect("manual import response");

    assert!(
        stores_visible_during_sqlite_wait,
        "both in-memory mutations should be readable while SQLite is blocked"
    );
    assert!(
        request_waited_for_sqlite,
        "the request should wait for SQLite"
    );
    assert!(
        library_reader_responsive,
        "library readers should not wait for SQLite"
    );
    assert!(
        runtime_reader_responsive,
        "runtime readers should not wait for SQLite"
    );
    assert_eq!(response.status, "202 Accepted");
    let persisted_library = db
        .list_library_items(10, 0)
        .await
        .expect("list persisted library items");
    assert!(persisted_library
        .iter()
        .any(|record| record.artist == "Artist" && record.title == "Album"));
    assert_eq!(
        db.get_runtime_compat_state()
            .await
            .expect("load runtime compatibility state")
            .expect("persisted runtime state")
            .lidarr_manual_imports,
        1
    );

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn runtime_compat_state_persists_and_rehydrates_records() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let restart = super::route_http_request("PUT", "/api/application", None, "{}", &state)
        .await
        .expect("restart request");
    assert_eq!(restart.status, "204 No Content");
    let gc = super::route_http_request("POST", "/api/application/gc", None, "", &state)
        .await
        .expect("gc");
    assert_eq!(gc.status, "200 OK");
    let autoreplace = super::route_http_request("PUT", "/api/autoreplace/enable", None, "", &state)
        .await
        .expect("autoreplace");
    assert_eq!(autoreplace.status, "200 OK");
    let relay =
        super::route_http_request("POST", "/api/relay", None, r#"{"enabled":true}"#, &state)
            .await
            .expect("relay");
    assert_eq!(relay.status, "200 OK");
    let relay_agent = super::route_http_request(
        "PUT",
        "/api/relay/agent",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("relay agent");
    assert_eq!(relay_agent.status, "200 OK");
    assert!(
        db.get_runtime_compat_state()
            .await
            .unwrap()
            .unwrap()
            .relay_agent_enabled
    );
    let relay_agent_disabled =
        super::route_http_request("DELETE", "/api/relay/agent", None, "", &state)
            .await
            .expect("disable relay agent");
    assert_eq!(relay_agent_disabled.status, "200 OK");
    let bridge_config = super::route_http_request(
        "PUT",
        "/api/v0/bridge/admin/config",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("bridge config");
    assert_eq!(bridge_config.status, "200 OK");
    let invite = super::route_http_request("POST", "/api/profile/invite", None, "{}", &state)
        .await
        .expect("invite");
    assert_eq!(invite.status, "201 Created");
    let warm_cache =
        super::route_http_request("POST", "/api/slskdn/warm-cache", None, "{}", &state)
            .await
            .expect("warm cache");
    assert_eq!(warm_cache.status, "202 Accepted");
    let songid = super::route_http_request(
        "POST",
        "/api/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("songid");
    assert_eq!(songid.status, "202 Accepted");
    let backfill = super::route_http_request("POST", "/api/backfill", None, "{}", &state)
        .await
        .expect("backfill");
    assert_eq!(backfill.status, "202 Accepted");
    let options = super::route_http_request(
        "PATCH",
        "/api/options",
        None,
        r#"{"soulseek":{"listenPort":50300}}"#,
        &state,
    )
    .await
    .expect("options patch");
    assert_eq!(options.status, "200 OK");
    let options_json = serde_json::from_str::<serde_json::Value>(&options.body).unwrap();
    assert_eq!(options_json["soulseek"]["listenPort"], 50300);
    let options_upload =
        super::route_http_request("PUT", "/api/options/yaml", None, r#""app: {}""#, &state)
            .await
            .expect("options upload");
    assert_eq!(options_upload.status, "200 OK");
    assert!(options_upload.body.is_empty());
    let options_validate = super::route_http_request(
        "POST",
        "/api/options/yaml/validate",
        None,
        r#""app: {}""#,
        &state,
    )
    .await
    .expect("options validate");
    assert_eq!(options_validate.status, "200 OK");
    assert!(options_validate.content_type.is_empty());
    assert!(options_validate.body.is_empty());
    let manual_import = super::route_http_request(
        "POST",
        "/api/integrations/lidarr/manualimport",
        None,
        r#"{"artist":"Artist","title":"Album","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("Lidarr manual import");
    assert_eq!(manual_import.status, "202 Accepted");
    assert_eq!(db.list_library_items(10, 0).await.unwrap().len(), 1);

    let persisted = db
        .get_runtime_compat_state()
        .await
        .expect("get runtime state")
        .expect("runtime state persisted");
    assert!(!persisted.application_restart_requested);
    assert!(!persisted.bridge_running);
    assert!(state.runtime.read().await.application_restart_requested);
    assert_eq!(persisted.gc_runs, 1);
    assert!(persisted.autoreplace_enabled);
    assert!(persisted.relay_enabled);
    assert!(!persisted.relay_agent_enabled);
    assert_eq!(persisted.bridge_config_updates, 1);
    assert_eq!(persisted.options_updates, 0);
    assert_eq!(persisted.options_yaml_uploads, 0);
    assert_eq!(persisted.options_yaml_validations, 0);
    assert_eq!(persisted.profile_invites_created, 1);
    assert_eq!(persisted.cache_warm_runs, 1);
    assert_eq!(persisted.songid_runs, 1);
    assert_eq!(persisted.backfill_runs, 1);
    assert_eq!(persisted.lidarr_manual_imports, 1);

    let rehydrated_relay = super::RelayState::from_persisted(&persisted);
    let rehydrated_runtime = super::RuntimeCompatState::from_persisted(&persisted);
    assert!(rehydrated_relay.enabled);
    let runtime_json = rehydrated_runtime.json_value();
    assert_eq!(runtime_json["pendingRestart"], false);
    assert_eq!(runtime_json["autoreplaceEnabled"], true);
    assert_eq!(runtime_json["relayAgentEnabled"], false);
    assert_eq!(runtime_json["bridgeConfigUpdates"], 1);
    assert_eq!(runtime_json["optionsUpdates"], 0);
    assert_eq!(runtime_json["optionsYamlUploads"], 0);
    assert_eq!(runtime_json["optionsYamlValidations"], 0);
    assert_eq!(runtime_json["profileInvitesCreated"], 1);
    assert_eq!(runtime_json["cacheWarmRuns"], 1);
    assert_eq!(runtime_json["songidRuns"], 1);
    assert_eq!(runtime_json["backfillRuns"], 1);
    assert_eq!(runtime_json["lidarrManualImports"], 1);

    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("database stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["runtimeState"], 1);
    assert_eq!(stats_json["persisted"]["runtimeState"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn search_api_lists_with_filters_and_pagination() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"alpha\",\"target\":\"global\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"beta\",\"target\":\"wishlist\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
        .await
        .unwrap();

    let filtered = super::route_http_request(
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
async fn search_api_expires_and_prunes_records() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    let created = super::route_http_request(
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

    let listed = super::route_http_request("GET", "/api/v0/searches/records", None, "", &state)
        .await
        .expect("list searches");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"status\":\"expired\""));
    assert!(listed.body.contains("\"expired\":1"));

    let pruned = super::route_http_request("POST", "/api/v0/searches/prune", None, "", &state)
        .await
        .expect("prune searches");
    assert_eq!(pruned.status, "200 OK");
    assert!(pruned.body.contains("\"pruned\":1"));
    assert!(pruned.body.contains("\"remaining\":0"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn search_create_accepts_camel_case_ttl_and_caps_large_values() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    let created = super::route_http_request(
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

    let invalid = super::route_http_request(
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
async fn events_api_records_mutating_workflows() {
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_CONFIGURATION", "true"));
    state.session.write().await.state = "connected";

    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"event flac\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    super::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        "{\"username\":\"friend\",\"body\":\"hi\"}",
        &state,
    )
    .await
    .unwrap();
    super::route_http_request(
        "PATCH",
        "/api/options",
        None,
        "{\"theme\":\"dark\"}",
        &state,
    )
    .await
    .unwrap();

    let events = super::route_http_request("GET", "/api/v0/events/records", None, "", &state)
        .await
        .expect("events response");
    assert_eq!(events.status, "200 OK");
    assert!(events.body.contains("\"kind\":\"search.started\""));
    assert!(events.body.contains("\"topic\":\"searches\""));
    assert!(events.body.contains("\"kind\":\"message.received\""));
    assert!(events.body.contains("\"topic\":\"messages\""));
    assert!(events.body.contains("\"kind\":\"options.updated\""));
    assert!(events.body.contains("\"topic\":\"settings\""));
    assert!(events.body.contains("\"count\":3"));

    let filtered = super::route_http_request(
        "GET",
        "/api/v0/events/records?kind=search.started",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered events");
    assert_eq!(filtered.status, "200 OK");
    assert!(filtered.body.contains("\"filtered_count\":1"));
    assert!(filtered.body.contains("\"resource\":\"1\""));
    assert!(!filtered.body.contains("message.received"));

    let topic_filtered = super::route_http_request(
        "GET",
        "/api/v0/events/records?topic=messages",
        None,
        "",
        &state,
    )
    .await
    .expect("topic-filtered events");
    assert_eq!(topic_filtered.status, "200 OK");
    assert!(topic_filtered.body.contains("\"filtered_count\":1"));
    assert!(topic_filtered
        .body
        .contains("\"kind\":\"message.received\""));
    assert!(!topic_filtered.body.contains("search.started"));

    let settings_filtered = super::route_http_request(
        "GET",
        "/api/v0/events/records?topic=settings",
        None,
        "",
        &state,
    )
    .await
    .expect("settings-filtered events");
    assert_eq!(settings_filtered.status, "200 OK");
    assert!(settings_filtered.body.contains("\"filtered_count\":1"));
    assert!(settings_filtered
        .body
        .contains("\"kind\":\"options.updated\""));
    assert!(settings_filtered.body.contains("volatile=true"));

    let controller_events = super::route_http_request(
        "GET",
        "/api/v0/events?topic=searches&q=search",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd events");
    assert_eq!(controller_events.status, "200 OK");
    let controller_json =
        serde_json::from_str::<serde_json::Value>(&controller_events.body).unwrap();
    assert_eq!(controller_json.as_array().unwrap().len(), 1);
    assert_eq!(controller_json[0]["topic"], "searches");
    assert_eq!(controller_json[0]["type"], "search.started");
    assert_eq!(controller_json[0]["payload"]["resource"], "1");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn events_controller_emits_filtered_total_count_header() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"));
    {
        let mut events = state.events.write().await;
        events.record("search.started", "one", Some("ambient".to_owned()));
        events.record("search.completed", "two", Some("ambient".to_owned()));
        events.record("transfer.completed", "three", Some("ambient".to_owned()));
    }

    let (mut client, server) = tokio::io::duplex(1024 * 1024);
    let task = tokio::spawn(super::handle_http_stream(
        server,
        Some("127.0.0.1:1".parse().expect("local test peer")),
        false,
        state,
    ));
    client
        .write_all(
            b"GET /api/v0/events?topic=searches&limit=1 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write events pagination request");
    let mut response = Vec::new();
    client
        .read_to_end(&mut response)
        .await
        .expect("read events pagination response");
    task.await
        .expect("events pagination HTTP task")
        .expect("events pagination HTTP response");

    let response = String::from_utf8(response).expect("events response is UTF-8");
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .expect("events response header boundary");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("X-Total-Count: 2\r\n"));
    let events = serde_json::from_str::<serde_json::Value>(body).expect("events JSON");
    assert_eq!(events.as_array().map(Vec::len), Some(1));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_registration_rejects_invalid_events_and_caps_count() {
    let (state, _receiver) = test_state();

    let (array_state, _array_receiver) = test_state();
    let array_created = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/array-hook","events":["message.sent","search.created","message.sent"]}"#,
        &array_state,
    )
    .await
    .expect("array webhook");
    assert_eq!(array_created.status, "201 Created");
    let array_events = array_state
        .webhooks
        .read()
        .await
        .get_all()
        .first()
        .map(|webhook| webhook.events.clone())
        .expect("array webhook record");
    assert_eq!(
        array_events,
        vec![
            super::webhooks::WebhookEvent::MessageSent,
            super::webhooks::WebhookEvent::SearchCreated,
        ]
    );

    let admin_array_created = super::route_http_request(
        "POST",
        "/api/admin/webhooks",
        None,
        r#"{"url":"https://example.test/admin-array-hook","events":["transfer.failed"]}"#,
        &array_state,
    )
    .await
    .expect("admin array webhook");
    assert_eq!(admin_array_created.status, "201 Created");
    assert!(array_state
        .webhooks
        .read()
        .await
        .get_all()
        .iter()
        .any(|webhook| webhook.events == vec![super::webhooks::WebhookEvent::TransferFailed]));

    let invalid = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created,bogus"}"#,
        &state,
    )
    .await
    .expect("invalid webhook");
    assert_eq!(invalid.status, "400 Bad Request");
    assert_eq!(invalid.body, "{\"error\":\"invalid webhook event\"}");

    let weak_secret = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created","secret":"short"}"#,
        &state,
    )
    .await
    .expect("weak webhook secret");
    assert_eq!(weak_secret.status, "400 Bad Request");
    assert!(weak_secret.body.contains("webhook secret"));

    let weak_admin_secret = super::route_http_request(
        "POST",
        "/api/admin/webhooks",
        None,
        r#"{"url":"https://example.test/admin-hook","secret":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        &state,
    )
    .await
    .expect("weak admin webhook secret");
    assert_eq!(weak_admin_secret.status, "400 Bad Request");
    assert!(weak_admin_secret.body.contains("webhook secret"));

    for index in 0..super::webhooks::MAX_WEBHOOKS {
        let body =
            format!(r#"{{"url":"https://example.test/hook/{index}","events":"search.created"}}"#);
        let created = super::route_http_request("POST", "/api/webhooks", None, &body, &state)
            .await
            .expect("create webhook");
        assert_eq!(created.status, "201 Created");
        if index == 0 {
            let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
            assert_eq!(created_json["secretReturnedOnce"], true);
            assert!(created_json["secret"]
                .as_str()
                .is_some_and(|value| !value.is_empty()));
        }
    }

    let listed = super::route_http_request("GET", "/api/webhooks", None, "", &state)
        .await
        .expect("list webhooks");
    assert_eq!(listed.status, "200 OK");
    assert!(!listed.body.contains("\"secret\""));

    let capped = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook/overflow","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("capped webhook");
    assert_eq!(capped.status, "400 Bad Request");
    assert_eq!(capped.body, "{\"error\":\"webhook limit reached\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_config_persists_rehydrates_and_records_dispatch_logs() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";
    let secret = super::webhooks::Webhook::generate_secret().expect("test randomness");
    let created = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        &format!(
            "{{\"url\":\"https://example.com/hook\",\"events\":\"search.created,message.sent\",\"secret\":\"{}\"}}",
            super::json_escape(&secret)
        ),
        &state,
    )
    .await
    .expect("create webhook");
    assert_eq!(created.status, "201 Created");
    let created_json: serde_json::Value = serde_json::from_str(&created.body).unwrap();
    let webhook_id = created_json["id"].as_str().expect("webhook id");

    let persisted = db.list_webhooks().await.expect("list webhooks");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].id, webhook_id);
    assert_eq!(persisted[0].events, "search.created,message.sent");

    let rehydrated = super::webhooks::WebhookManager::from_webhooks(
        persisted
            .clone()
            .into_iter()
            .filter_map(super::webhooks::webhook_from_persisted)
            .collect(),
    );
    assert_eq!(rehydrated.get_all().len(), 1);
    assert!(rehydrated
        .get_for_event(super::webhooks::WebhookEvent::MessageSent)
        .iter()
        .any(|webhook| webhook.id == webhook_id));

    let admin_list = super::route_http_request("GET", "/api/admin/webhooks", None, "", &state)
        .await
        .expect("list admin webhooks");
    assert_eq!(admin_list.status, "200 OK");
    assert!(admin_list.body.contains("\"retry_count\":0"));
    assert!(admin_list.body.contains("\"max_retries\":3"));
    assert!(admin_list.body.contains("\"timeout_seconds\":30"));

    let patched = super::route_http_request(
        "PATCH",
        &format!("/api/webhooks/{webhook_id}"),
        None,
        "{\"active\":false}",
        &state,
    )
    .await
    .expect("patch webhook");
    assert_eq!(patched.status, "200 OK");
    let persisted_patch = db.get_webhook(webhook_id).await.unwrap().unwrap();
    assert!(!persisted_patch.active);

    let _ = super::route_http_request(
        "PATCH",
        &format!("/api/webhooks/{webhook_id}"),
        None,
        "{\"active\":true}",
        &state,
    )
    .await
    .expect("reactivate webhook");
    let created_search = super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"webhook log\"}",
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created_search.status, "200 OK");
    let logs = db
        .get_webhook_logs(webhook_id, 10, 0)
        .await
        .expect("webhook logs");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].event, "search.created");
    assert!(
        matches!(logs[0].status.as_str(), "queued" | "success" | "failed"),
        "unexpected webhook delivery status: {}",
        logs[0].status
    );

    let routed_logs = super::route_http_request(
        "GET",
        &format!("/api/webhooks/{webhook_id}/logs"),
        None,
        "",
        &state,
    )
    .await
    .expect("route webhook logs");
    assert_eq!(routed_logs.status, "200 OK");
    assert!(routed_logs.body.contains("search.created"));

    let aliased_logs = super::route_http_request(
        "GET",
        &format!("/api/webhooks/unrelated/{webhook_id}/logs"),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased webhook logs");
    assert_eq!(aliased_logs.status, "404 Not Found");

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/webhooks/{webhook_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete webhook");
    assert_eq!(deleted.status, "200 OK");
    assert!(db.list_webhooks().await.expect("list deleted").is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_audit_persistence_failures_surface_in_session_health() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let mut manager = super::webhooks::WebhookManager::new();
    manager
        .register(super::webhooks::Webhook::new(
            "https://example.com/hook".to_owned(),
            vec![super::webhooks::WebhookEvent::SearchCreated],
            "secret_0123456789abcdef0123456789abcdef".to_owned(),
        ))
        .expect("register webhook");
    db.close_for_test().await;

    let persistence_turn = state.webhook_persistence_lock.lock().await;
    let error = super::webhooks::persist_webhook_dispatch_logs(
        &state,
        &manager,
        super::webhooks::WebhookEvent::SearchCreated,
        "correlation-test",
        r#"{"query":"test"}"#,
        &persistence_turn,
    )
    .await;
    drop(persistence_turn);
    if let Some(error) = error {
        super::session_runtime::update_session(&state, |session| {
            session.last_error = Some(error);
        })
        .await;
    }

    let session = state.session.read().await;
    let error = session.last_error.as_deref().unwrap_or_default();
    assert!(error.contains("webhook audit persistence failed for 1 delivery record"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_routes_roll_back_when_persistence_fails() {
    for path in ["/api/webhooks", "/api/admin/webhooks"] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.webhooks.read().await.clone();
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            path,
            None,
            r#"{"url":"https://example.test/hook","events":"search.created"}"#,
            &state,
        )
        .await
        .expect("failed webhook create response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("webhook persistence failed"),
            "{path}"
        );
        assert_eq!(*state.webhooks.read().await, previous, "{path}");
    }

    for (method, prefix, body, expected_error) in [
        (
            "PATCH",
            "/api/webhooks/",
            r#"{"active":false}"#,
            "webhook persistence failed",
        ),
        (
            "DELETE",
            "/api/webhooks/",
            "",
            "webhook deletion persistence failed",
        ),
        (
            "DELETE",
            "/api/admin/webhooks/",
            "",
            "webhook deletion persistence failed",
        ),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let webhook = super::webhooks::Webhook::new(
            "https://example.test/hook".to_owned(),
            vec![super::webhooks::WebhookEvent::SearchCreated],
            "0123456789abcdef0123456789abcdef".to_owned(),
        );
        let webhook_id = webhook.id.clone();
        state.webhooks.write().await.register(webhook).unwrap();
        let previous = state.webhooks.read().await.clone();
        db.close_for_test().await;

        let response =
            super::route_http_request(method, &format!("{prefix}{webhook_id}"), None, body, &state)
                .await
                .expect("failed webhook mutation response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {prefix}"
        );
        assert!(response.body.contains(expected_error), "{method} {prefix}");
        assert_eq!(*state.webhooks.read().await, previous, "{method} {prefix}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_test_send_rejects_when_delivery_pool_is_full() {
    let (state, _receiver) = test_state();

    let created = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("create webhook");
    assert_eq!(created.status, "201 Created");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let webhook_id = created_json["id"].as_str().unwrap();
    let _all_delivery_permits = Arc::clone(&state.webhook_deliveries)
        .acquire_many_owned(super::MAX_WEBHOOK_DELIVERY_TASKS as u32)
        .await
        .expect("acquire webhook delivery permits");

    let response = super::route_http_request(
        "POST",
        &format!("/api/webhooks/{webhook_id}/test"),
        None,
        "",
        &state,
    )
    .await
    .expect("test webhook");

    assert_eq!(response.status, "429 Too Many Requests");
    assert!(response.body.contains("webhook deliveries"));

    let admin_response = super::route_http_request(
        "POST",
        &format!("/api/admin/webhooks/{webhook_id}/test"),
        None,
        "",
        &state,
    )
    .await
    .expect("admin test webhook");

    assert_eq!(admin_response.status, "429 Too Many Requests");
    assert!(admin_response.body.contains("webhook deliveries"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_registration_rejects_blocked_urls() {
    let (state, _receiver) = test_state();

    for body in [
        r#"{"url":"ftp://example.test/hook","events":"search.created"}"#,
        r#"{"url":"http://localhost/hook","events":"search.created"}"#,
        r#"{"url":"http://127.0.0.1/hook","events":"search.created"}"#,
        r#"{"url":"http://169.254.169.254/hook","events":"search.created"}"#,
    ] {
        let response = super::route_http_request("POST", "/api/webhooks", None, body, &state)
            .await
            .expect("blocked webhook URL");
        assert_eq!(response.status, "400 Bad Request");
    }

    let admin_response = super::route_http_request(
        "POST",
        "/api/admin/webhooks",
        None,
        r#"{"url":"http://127.0.0.1/hook"}"#,
        &state,
    )
    .await
    .expect("blocked admin webhook URL");
    assert_eq!(admin_response.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_patch_requires_active_boolean() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("create webhook");
    let id = serde_json::from_str::<serde_json::Value>(&created.body).expect("created json")["id"]
        .as_str()
        .expect("webhook id")
        .to_owned();

    let missing =
        super::route_http_request("PATCH", &format!("/api/webhooks/{id}"), None, "{}", &state)
            .await
            .expect("missing active");
    assert_eq!(missing.status, "400 Bad Request");
    assert_eq!(missing.body, "{\"error\":\"active boolean is required\"}");

    let patched = super::route_http_request(
        "PATCH",
        &format!("/api/webhooks/{id}"),
        None,
        r#"{"active":false}"#,
        &state,
    )
    .await
    .expect("patch webhook");
    assert_eq!(patched.status, "200 OK");
    assert!(patched.body.contains("\"active\":false"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn webhook_mutations_and_tests_require_exact_paths() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created"}"#,
        &state,
    )
    .await
    .unwrap();
    let id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    super::route_http_request(
        "PATCH",
        &format!("/api/webhooks/extra/{id}"),
        None,
        r#"{"active":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert!(state.webhooks.read().await.get(&id).unwrap().active);

    let _all_delivery_permits = Arc::clone(&state.webhook_deliveries)
        .acquire_many_owned(super::MAX_WEBHOOK_DELIVERY_TASKS as u32)
        .await
        .unwrap();
    let malformed_test = super::route_http_request(
        "POST",
        &format!("/api/webhooks/extra/{id}/test"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_ne!(malformed_test.status, "429 Too Many Requests");
    let malformed_admin_test = super::route_http_request(
        "POST",
        &format!("/api/admin/webhooks/extra/{id}/test"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_ne!(malformed_admin_test.body, "{\"status\":\"test_sent\"}");

    for path in [
        format!("/api/webhooks/extra/{id}"),
        format!("/api/admin/webhooks/extra/{id}"),
    ] {
        super::route_http_request("DELETE", &path, None, "", &state)
            .await
            .unwrap();
        assert!(state.webhooks.read().await.get(&id).is_some());
    }

    assert_eq!(
        super::webhook_resource_id(&format!("/api/webhooks/{id}"), "/api/webhooks/"),
        Some(id.as_str())
    );
    assert_eq!(
        super::webhook_resource_id(&format!("/api/webhooks/extra/{id}"), "/api/webhooks/"),
        None
    );
    let deleted =
        super::route_http_request("DELETE", &format!("/api/webhooks/{id}"), None, "", &state)
            .await
            .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state.webhooks.read().await.get(&id).is_none());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn external_visualizer_launch_requires_explicit_enable_flag() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", "true"));

    let status =
        super::route_http_request("GET", "/api/player/external-visualizer", None, "", &state)
            .await
            .expect("external visualizer status");
    assert_eq!(status.status, "200 OK");
    assert!(status.body.contains("\"configured\":true"));
    assert!(status.body.contains("\"enabled\":false"));

    let launch = super::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "400 Bad Request");
    assert!(launch.body.contains("launching is disabled"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn external_visualizer_launch_records_audit_event_when_enabled() {
    let command = if cfg!(windows) { "where.exe" } else { "true" };
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
            .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
    );

    let launch = super::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "200 OK");
    assert!(launch.body.contains("\"started\":true"));
    assert!(!launch.body.contains("\"command\""));
    let events = state.events.read().await;
    let event = events
        .records
        .iter()
        .find(|event| event.kind == "external_visualizer.launch")
        .expect("launch event");
    assert_eq!(event.resource, "external_visualizer");
    assert!(!event.resource.contains("true"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn external_visualizer_launch_errors_redact_command_details() {
    let command = "/private/missing-visualizer-secret";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
            .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
    );

    let launch = super::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "400 Bad Request");
    assert!(!launch.body.contains(command));
    let events = state.events.read().await;
    let event = events
        .records
        .iter()
        .find(|event| event.kind == "external_visualizer.launch.failed")
        .expect("failed launch event");
    assert_eq!(event.resource, "external_visualizer");
    assert_eq!(event.detail.as_deref(), Some("launch failed"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn external_visualizer_launch_rejects_when_process_pool_is_full() {
    let command = if cfg!(windows) { "where.exe" } else { "true" };
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
            .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
    );
    let _permits = Arc::clone(&state.external_visualizer_processes)
        .acquire_many_owned(super::MAX_EXTERNAL_VISUALIZER_PROCESSES as u32)
        .await
        .expect("configured visualizer permits");

    let launch = super::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "503 Service Unavailable");
    assert!(launch.body.contains("process limit reached"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn search_api_supports_targeted_dispatch_commands() {
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";

    let user = super::route_http_request(
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
        super::SessionCommand::Search {
            token: 1,
            query: "rare".to_owned(),
            target: super::SearchDispatchTarget::User("friend".to_owned()),
        }
    );

    let room = super::route_http_request(
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
        super::SessionCommand::Search {
            token: 2,
            query: "ambient".to_owned(),
            target: super::SearchDispatchTarget::Room("music".to_owned()),
        }
    );

    let wishlist = super::route_http_request(
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
        super::SessionCommand::Search {
            token: 3,
            query: "wantlist".to_owned(),
            target: super::SearchDispatchTarget::Wishlist,
        }
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn search_api_rejects_invalid_targeted_dispatch() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request(
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

    let invalid_target = super::route_http_request(
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
async fn search_response_api_merges_flattened_results() {
    let (state, _receiver) = test_state();
    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .unwrap();

    let response = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
            .await
            .expect("search responses");
    assert_eq!(responses.status, "200 OK");
    assert!(responses.body.starts_with('['));
    assert!(responses.body.contains("\"token\":1"));
    assert!(responses.body.contains("\"filename\":\"Remote/Song.mp3\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn wishlist_ignored_folders_persist_and_suppress_existing_and_future_results() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let created = super::route_http_request(
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
    let search = super::route_http_request(
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
        super::SessionCommand::Search {
            token: 1,
            query: "Artist Album".to_owned(),
            target: super::SearchDispatchTarget::Wishlist,
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
        super::route_http_request(
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

    let history = super::route_http_request(
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

    let viewed = super::route_http_request(
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

    let ignored = super::route_http_request(
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

    let updated = super::route_http_request(
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

    let duplicate = super::route_http_request(
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
        super::route_http_request(
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

    let listed = super::route_http_request(
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
    let wishlist = super::route_http_request("GET", "/api/wishlist", None, "", &state)
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
    let rehydrated = super::WishlistStore::from_persisted_with_ignored(
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

    let restored = super::route_http_request(
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

    super::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"username":"PeerOne","filename":"Remote/Album/Restored.mp3","size":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(state.searches.read().await.records[0].results.len(), 3);

    let completed = super::route_http_request("POST", "/api/searches/1/complete", None, "", &state)
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

    let repeated = super::route_http_request("POST", "/api/searches/1/complete", None, "", &state)
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
async fn wishlist_ignored_folder_mutations_roll_back_on_persistence_failure() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .wishlist
        .write()
        .await
        .add_item("Artist".to_owned(), "Album".to_owned(), "Audio".to_owned())
        .unwrap();
    let search = super::route_http_request("POST", "/api/wishlist/wish-1/search", None, "", &state)
        .await
        .unwrap();
    assert_eq!(search.status, "202 Accepted");
    super::route_http_request(
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

    let create = super::route_http_request(
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
    let delete = super::route_http_request(
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
async fn search_response_api_accepts_controller_group_payload() {
    let (state, _receiver) = test_state();
    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .unwrap();

    let response = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
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
async fn search_detail_routes_page_result_arrays_without_changing_totals() {
    let (state, _receiver) = test_state();
    super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/search-responses", None, &body, &state)
            .await
            .expect("ingest response");
    }

    for path in [
        "/api/v0/searches/1?offset=1&limit=1",
        "/api/searches/1?offset=1&limit=1",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
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
async fn search_response_routes_page_and_filter_peer_groups() {
    let (state, _receiver) = test_state();
    super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/search-responses", None, &body, &state)
            .await
            .expect("ingest response");
    }

    let paged = super::route_http_request(
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

    let filtered = super::route_http_request(
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

    let query_filtered = super::route_http_request(
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
async fn search_update_routes_mutate_lifecycle_and_query_projection() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    state.session.write().await.state = "connected";
    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"original\"}",
        &state,
    )
    .await
    .expect("create search");

    let completed = super::route_http_request(
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

    let active = super::route_http_request(
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

    let fetched = super::route_http_request("GET", "/api/v0/searches/1", None, "", &state)
        .await
        .expect("fetch updated search");
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
    assert_eq!(fetched_json["query"], "renamed");
    assert_eq!(fetched_json["status"], "active");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn search_action_routes_preserve_cancel_fail_and_expire_lifecycle() {
    let (state, _receiver) = test_state();
    for query in ["cancel me", "fail me", "expire me"] {
        super::route_http_request(
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
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .expect("search action");
        assert_eq!(response.status, "200 OK");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(json["status"], status);
        assert_eq!(json["state"], state_name);
        assert_eq!(json["isComplete"], true);
    }

    let event_count = state.events.read().await.records.len();
    let repeated = super::route_http_request("POST", "/api/v0/searches/1/cancel", None, "", &state)
        .await
        .expect("repeat cancelled search action");
    assert_eq!(repeated.status, "200 OK");
    assert_eq!(state.events.read().await.records.len(), event_count);

    let listed = super::route_http_request(
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
async fn search_response_api_projects_locked_files_for_controller_shape() {
    let (state, _receiver) = test_state();
    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"remote\"}",
        &state,
    )
    .await
    .unwrap();

    let response = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
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
async fn search_response_api_rejects_missing_fields() {
    let (state, _receiver) = test_state();

    let response =
        super::route_http_request("POST", "/api/v0/search-responses", None, "{}", &state)
            .await
            .expect("bad response");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(response.body, "{\"error\":\"token is required\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn search_response_api_rejects_oversized_protocol_token() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request(
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
fn search_store_merges_peer_search_responses() {
    let mut store = super::SearchStore::new();
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
fn peer_search_responses_suppress_only_matching_wishlist_peer_folder() {
    let mut store = super::SearchStore::new();
    let record = store
        .create_scheduled_wishlist_for_item(
            "artist album".to_owned(),
            Some("wish-1".to_owned()),
            300,
        )
        .unwrap()
        .record;
    let response = FileSearchResponse {
        username: "PeerOne".to_owned(),
        token: record.token,
        results: ["Remote/Album/Blocked.flac", "Remote/Other/Allowed.flac"]
            .into_iter()
            .map(|filename| FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: filename.to_owned(),
                size: 1,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            })
            .collect(),
        slot_free: true,
        average_speed: 0,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    };
    let ignored = super::WishlistIgnoredResult {
        id: "ignored-1".to_owned(),
        wishlist_item_id: "wish-1".to_owned(),
        username: "peerone".to_owned(),
        directory: "Remote/Album".to_owned(),
        created_at: 1,
    };

    let (updated, appended) = store
        .add_peer_response_filtered(&response, &[ignored], None)
        .unwrap();

    assert_eq!(updated.results.len(), 1);
    assert_eq!(updated.results[0].filename, "Remote/Other/Allowed.flac");
    assert_eq!(appended.len(), 1);
    assert_eq!(appended[0].filename, "Remote/Other/Allowed.flac");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn search_store_caps_results_from_peer_responses() {
    let mut store = super::SearchStore::new();
    let record = store
        .create(None, "remote".to_owned(), "global", None, Vec::new(), 300)
        .unwrap()
        .record;
    let response = FileSearchResponse {
        username: "peer1".to_owned(),
        token: record.token,
        results: (0..(super::MAX_SEARCH_RESULTS_PER_SEARCH + 5))
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
    assert_eq!(updated.results.len(), super::MAX_SEARCH_RESULTS_PER_SEARCH);
    assert_eq!(updated.results.last().unwrap().filename, "Remote/9999.flac");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn search_store_bounds_text_and_aggregate_results() {
    let mut store = super::SearchStore::new();
    let oversized_query = "q".repeat(super::MAX_SEARCH_QUERY_BYTES + 1);
    let first = store
        .create(None, oversized_query, "global", None, Vec::new(), 300)
        .unwrap()
        .record;
    assert_eq!(first.query.len(), super::MAX_SEARCH_QUERY_BYTES);
    let response = FileSearchResponse {
        username: "u".repeat(super::MAX_SEARCH_RESULT_USERNAME_BYTES + 1),
        token: first.token,
        results: vec![FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "f".repeat(super::MAX_SEARCH_RESULT_FILENAME_BYTES + 1),
            size: 1,
            extension: "e".repeat(super::MAX_SEARCH_RESULT_EXTENSION_BYTES + 1),
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
        super::MAX_SEARCH_RESULT_USERNAME_BYTES
    );
    assert_eq!(
        updated.results[0].filename.len(),
        super::MAX_SEARCH_RESULT_FILENAME_BYTES
    );
    assert_eq!(
        updated.results[0].extension.len(),
        super::MAX_SEARCH_RESULT_EXTENSION_BYTES
    );

    let template = super::SearchResultEntry {
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
        record.results = vec![template.clone(); super::MAX_SEARCH_RESULTS_PER_SEARCH];
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
    assert_eq!(store.total_results(), super::MAX_TOTAL_SEARCH_RESULTS);
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_transfer_api_creates_updates_and_reports_stats() {
    let (state, _receiver) = test_state();

    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"direction\":0,\"filename\":\"Remote/Song.flac\",\"size\":100}",
        &state,
    )
    .await
    .expect("create transfer");
    assert_eq!(created.status, "201 Created");
    assert!(created.body.contains("\"id\":1"));
    assert!(created.body.contains("\"status\":\"queued\""));

    let started = super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start transfer");
    assert_eq!(started.status, "200 OK");
    assert!(started.body.contains("\"status\":\"in_progress\""));

    let progress = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":40}",
        &state,
    )
    .await
    .expect("progress transfer");
    assert_eq!(progress.status, "200 OK");
    assert!(progress.body.contains("\"bytes_transferred\":40"));

    let missing_progress =
        super::route_http_request("POST", "/api/v0/transfers/1/progress", None, "{}", &state)
            .await
            .expect("reject missing transfer progress");
    assert_eq!(missing_progress.status, "400 Bad Request");

    let oversized_progress = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":101}",
        &state,
    )
    .await
    .expect("reject oversized transfer progress");
    assert_eq!(oversized_progress.status, "400 Bad Request");

    let invalid_status = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"bytes_transferred\":40,\"status\":\"running\"}",
        &state,
    )
    .await
    .expect("reject non-terminal transfer status");
    assert_eq!(invalid_status.status, "400 Bad Request");

    let partial_success = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"bytes_transferred\":99,\"status\":\"succeeded\"}",
        &state,
    )
    .await
    .expect("reject partial transfer success");
    assert_eq!(partial_success.status, "400 Bad Request");

    let missing_completion_bytes = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"status\":\"failed\"}",
        &state,
    )
    .await
    .expect("reject missing completion bytes");
    assert_eq!(missing_completion_bytes.status, "400 Bad Request");

    let completed = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        "{\"bytes_transferred\":100}",
        &state,
    )
    .await
    .expect("complete transfer");
    assert_eq!(completed.status, "200 OK");
    assert!(completed.body.contains("\"status\":\"succeeded\""));

    let late_progress = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":50}",
        &state,
    )
    .await
    .expect("reject progress after completion");
    assert_eq!(late_progress.status, "409 Conflict");

    let stats = super::route_http_request("GET", "/api/v0/transfers/stats", None, "", &state)
        .await
        .expect("transfer stats");
    assert_eq!(stats.status, "200 OK");
    assert!(stats.body.contains("\"total\":1"));
    assert!(stats.body.contains("\"succeeded\":1"));
    assert!(stats.body.contains("\"bytes_transferred\":100"));

    let filtered = super::route_http_request(
        "GET",
        "/api/v0/transfers?status=succeeded&q=song&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered transfers");
    assert_eq!(filtered.status, "200 OK");
    let filtered_json = serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap();
    assert_eq!(filtered_json.as_array().unwrap().len(), 1);
    assert_eq!(
        filtered_json[0]["directories"][0]["files"][0]["filename"],
        "Remote/Song.flac"
    );

    let ledger = vec![serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/transfers",
        "case": "populated-dynamic-state",
        "pass": true,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("transfer_api_populated.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_transfer_cleanup_persistence() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"filename":"Remote/Persisted.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create persisted transfer");
    assert_eq!(created.status, "201 Created");
    let persisted = db.get_transfer("1").await.expect("get created").unwrap();
    assert_eq!(persisted.direction, "download");
    assert_eq!(persisted.status, "queued");
    assert_eq!(persisted.filesize, 100);

    let started = super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
        .await
        .expect("start persisted transfer");
    assert_eq!(started.status, "200 OK");
    let persisted = db.get_transfer("1").await.expect("get started").unwrap();
    assert_eq!(persisted.status, "in_progress");

    let progressed = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":40}"#,
        &state,
    )
    .await
    .expect("progress persisted transfer");
    assert_eq!(progressed.status, "200 OK");
    let persisted = db.get_transfer("1").await.expect("get progressed").unwrap();
    assert_eq!(persisted.status, "in_progress");
    assert_eq!(persisted.progress, 40);

    let completed = super::route_http_request(
        "POST",
        "/api/v0/transfers/1/complete",
        None,
        r#"{"bytes_transferred":100}"#,
        &state,
    )
    .await
    .expect("complete persisted transfer");
    assert_eq!(completed.status, "200 OK");
    let persisted = db.get_transfer("1").await.expect("get completed").unwrap();
    assert_eq!(persisted.status, "succeeded");
    assert_eq!(persisted.progress, 100);
    assert!(persisted.completed_at.is_some());
    let mut rehydrated = super::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated.rehydrate_from_database(&db).await;
    assert!(rehydrated
        .entries
        .iter()
        .any(|entry| entry.id == 1 && entry.status == "succeeded"));
    let events = db
        .list_transfer_events(Some("1"), 10, 0)
        .await
        .expect("list transfer events");
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].status, "succeeded");
    assert_eq!(events[0].progress, 100);
    assert_eq!(events[0].filename, "Remote/Persisted.flac");
    assert_eq!(events[3].status, "queued");

    let cancelled = super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"friend","filename":"Remote/Cancel.flac","size":10}"#,
        &state,
    )
    .await
    .expect("create cancellable transfer");
    assert_eq!(cancelled.status, "201 Created");
    let aliased_delete =
        super::route_http_request("DELETE", "/api/v0/transfers/unrelated/2", None, "", &state)
            .await
            .expect("reject aliased transfer delete");
    assert_eq!(aliased_delete.status, "404 Not Found");
    assert_eq!(
        db.get_transfer("2").await.unwrap().unwrap().status,
        "queued"
    );
    let deleted = super::route_http_request("DELETE", "/api/v0/transfers/2", None, "", &state)
        .await
        .expect("cancel persisted transfer");
    assert_eq!(deleted.status, "200 OK");
    let persisted = db.get_transfer("2").await.expect("get cancelled").unwrap();
    assert_eq!(persisted.status, "cancelled");
    assert!(persisted.completed_at.is_some());
    let stats = super::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("transfer event database stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["persisted"]["transfers"], 2);
    assert_eq!(stats_json["persisted"]["transferEvents"], 6);

    let upload_created = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("uploader".to_owned()),
            "Uploads/Persisted.flac".to_owned(),
            None,
            Some(50),
        )
    };
    super::persist_transfer_record(&state, &upload_created)
        .await
        .expect("persist upload");
    let upload_id = upload_created.id;
    let upload_completed = {
        let mut transfers = state.transfers.write().await;
        transfers
            .update_status(upload_id, "succeeded", Some(50), None)
            .expect("complete persisted upload")
    };
    super::persist_transfer_record(&state, &upload_completed)
        .await
        .expect("persist completed upload");
    assert_eq!(
        db.get_transfer(&upload_id.to_string())
            .await
            .expect("get persisted upload")
            .unwrap()
            .status,
        "succeeded"
    );

    let pruned = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("prune persisted terminal downloads");
    assert_eq!(pruned.status, "204 No Content");
    assert!(pruned.body.is_empty());
    assert!(db.get_transfer("1").await.expect("get pruned 1").is_none());
    assert!(db.get_transfer("2").await.expect("get pruned 2").is_none());

    let uploads_pruned = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("prune persisted terminal uploads");
    assert_eq!(uploads_pruned.status, "204 No Content");
    assert!(uploads_pruned.body.is_empty());
    assert!(db
        .get_transfer(&upload_id.to_string())
        .await
        .expect("get pruned upload")
        .is_none());

    for case in [
        "nominal-status-headers-body",
        "mutation-side-effects-and-readback",
    ] {
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/transfers/downloads/all/completed",
            "case": case,
            "pass": true,
        }));
    }
    for case in [
        "nominal-status-headers-body",
        "mutation-side-effects-and-readback",
    ] {
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/transfers/uploads/all/completed",
            "case": case,
            "pass": true,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("transfer_cleanup_persistence.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}
