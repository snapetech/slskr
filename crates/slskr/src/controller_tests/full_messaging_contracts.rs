//! Controller full messaging contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn private_message_auto_response_command_line_overrides_controller_yaml() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-auto-response-cli-test-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "soulseek:\n  private_message_auto_response:\n    enabled: false\n    message: yaml response\n    cooldown_minutes: 15\n",
    )
    .unwrap();
    let invocation = crate::parse_serve_args(&[
        OsString::from("serve"),
        OsString::from("--app-dir"),
        state_dir.clone().into_os_string(),
        OsString::from("--slsk-private-message-auto-response"),
        OsString::from("--slsk-private-message-auto-response-message"),
        OsString::from("cli response"),
        OsString::from("--slsk-private-message-auto-response-cooldown-minutes"),
        OsString::from("45"),
    ])
    .unwrap()
    .unwrap();
    let config = crate::config::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &crate::ControllerCliEnv {
            values: &invocation.config_environment,
        },
    )
    .unwrap();
    assert!(config.private_message_auto_response.enabled);
    assert_eq!(config.private_message_auto_response.message, "cli response");
    assert_eq!(config.private_message_auto_response.cooldown_minutes, 45);
    let mut overlay = crate::ControllerOptionsOverlayState::load(&config).unwrap();
    overlay.command_line_environment = invocation.config_environment;
    let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["soulseek"]["privateMessageAutoResponse"],
        serde_json::json!({
            "enabled": true,
            "message": "cli response",
            "cooldownMinutes": 45,
        })
    );
    let scalar_projection = crate::controller_yaml_api_projection(serde_json::json!({
        "soulseek": {
            "private_message_auto_response": {
                "enabled": "true",
                "message": 123,
                "cooldown_minutes": "15",
            }
        }
    }));
    assert_eq!(
        scalar_projection["soulseek"]["privateMessageAutoResponse"],
        serde_json::json!({
            "enabled": true,
            "message": "123",
            "cooldownMinutes": 15,
        })
    );
    let auto_retry_projection = crate::controller_yaml_api_projection(serde_json::json!({
        "transfers": {
            "download": {
                "auto_retry": {
                    "enabled": "false",
                    "retry_delay_seconds": "10",
                    "check_interval_seconds": null,
                    "max_attempts": "0",
                    "max_files_per_cycle": "2",
                    "max_files_per_peer_per_cycle": "2",
                    "peer_cooldown_seconds": "60",
                    "alternate_sources_enabled": "true",
                    "max_alternate_source_searches_per_cycle": null,
                    "alternate_source_size_tolerance_percent": "5.5",
                }
            }
        }
    }));
    assert_eq!(
        auto_retry_projection["global"]["download"]["autoRetry"],
        serde_json::json!({
            "enabled": false,
            "retryDelaySeconds": 10,
            "checkIntervalSeconds": 300,
            "maxAttempts": 0,
            "maxFilesPerCycle": 2,
            "maxFilesPerPeerPerCycle": 2,
            "peerCooldownSeconds": 60,
            "alternateSourcesEnabled": true,
            "maxAlternateSourceSearchesPerCycle": 1,
            "alternateSourceSizeTolerancePercent": 5.5,
        })
    );
    let _ = fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn private_pod_message_history_requires_membership() {
    let (state, _receiver) = test_state();
    let path = "/api/v0/pods/private-pod/channels/general/messages";
    let created = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"private-pod","name":"Private","isPublic":false,"channels":[{"channelId":"general","kind":0,"name":"General"}]}}"#,
        &state,
    )
    .await
    .expect("create private pod");
    assert_eq!(created.status, "201 Created");

    *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
        "intruder", "secret",
    ));
    let forbidden = crate::route_http_request("GET", path, None, "", &state)
        .await
        .expect("private pod history");
    assert_eq!(forbidden.status, "403 Forbidden");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn messages_and_rooms_persist_and_rehydrate_records() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let outbound = crate::route_http_request(
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

    let inbound = crate::route_http_request(
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

    let acked = crate::route_http_request("POST", "/api/v0/messages/1/ack", None, "", &state)
        .await
        .expect("ack persisted message");
    assert_eq!(acked.status, "200 OK");
    let _ = receiver.try_recv();

    {
        let mut session = state.session.write().await;
        session.state = "connected";
        session.updated_at = crate::unix_timestamp();
    }
    let joined = crate::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
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

    let rehydrated_messages = crate::MessageStore::from_persisted(persisted_messages);
    let rehydrated_rooms = crate::RoomStore::from_persisted(persisted_rooms);
    let (restarted_state, _) = test_state_with_env_parts_full(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        rehydrated_messages,
        rehydrated_rooms,
        Some(db),
    );

    let listed_messages =
        crate::route_http_request("GET", "/api/v0/messages/friend", None, "", &restarted_state)
            .await
            .expect("list rehydrated messages");
    assert_eq!(listed_messages.status, "200 OK");
    assert!(listed_messages.body.contains("\"count\":2"));
    assert!(listed_messages.body.contains("\"body\":\"persisted hi\""));
    assert!(listed_messages.body.contains("\"acknowledged\":true"));
    assert!(listed_messages.body.contains("\"body\":\"persisted back\""));

    let listed_rooms =
        crate::route_http_request("GET", "/api/v0/rooms", None, "", &restarted_state)
            .await
            .expect("list rehydrated rooms");
    assert_eq!(listed_rooms.status, "200 OK");
    assert!(listed_rooms.body.contains("\"name\":\"music\""));
    assert!(listed_rooms.body.contains("\"joined\":true"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn conversation_delete_removes_persisted_history() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    for (username, message) in [("friend", "private"), ("other", "retained")] {
        let response = crate::route_http_request(
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
        crate::route_http_request("DELETE", "/api/conversations/friend", None, "", &state)
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

    let rehydrated = crate::MessageStore::from_persisted(persisted);
    assert!(rehydrated
        .records
        .iter()
        .all(|message| message.username != "friend"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn joined_room_delete_requires_exact_path_and_persists_leave() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";
    let joined =
        crate::route_http_request("POST", "/api/rooms/joined", None, r#""room space""#, &state)
            .await
            .unwrap();
    assert_eq!(joined.status, "201 Created");
    assert_eq!(db.list_subscribed_rooms().await.unwrap().len(), 1);

    crate::route_http_request(
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
        crate::route_http_request("DELETE", "/api/rooms/joined/room%20space", None, "", &state)
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
pub(super) async fn joined_room_subresources_require_exact_room_segments() {
    let (state, _receiver) = test_state();
    crate::route_http_request(
        "POST",
        "/api/rooms/joined",
        None,
        r#""private room""#,
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request(
        "POST",
        "/api/rooms/joined/private%20room/messages",
        None,
        r#""secret message""#,
        &state,
    )
    .await
    .unwrap();

    let exact = crate::route_http_request(
        "GET",
        "/api/rooms/joined/private%20room/messages",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let malformed = crate::route_http_request(
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

    crate::route_http_request(
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
        crate::joined_room_subresource("/api/rooms/joined/private%20room/messages", "/messages"),
        Some("private room".to_owned())
    );
    assert_eq!(
        crate::joined_room_subresource(
            "/api/rooms/joined/private%20room/extra/messages",
            "/messages"
        ),
        None
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn social_and_security_state_persist_and_rehydrate_records() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let note = crate::route_http_request(
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

    let updated_note = crate::route_http_request(
        "PUT",
        &format!("/api/users/notes/{note_id}"),
        None,
        r#"{"note":"trusted peer updated"}"#,
        &state,
    )
    .await
    .expect("update user note");
    assert_eq!(updated_note.status, "200 OK");

    let liked = crate::route_http_request(
        "POST",
        "/api/soulseek/interests",
        None,
        r#"{"name":"jazz"}"#,
        &state,
    )
    .await
    .expect("create liked interest");
    assert_eq!(liked.status, "201 Created");
    let duplicate_liked = crate::route_http_request(
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

    let hated = crate::route_http_request(
        "POST",
        "/api/soulseek/hated-interests",
        None,
        r#"{"name":"low bitrate"}"#,
        &state,
    )
    .await
    .expect("create hated interest");
    assert_eq!(hated.status, "201 Created");

    let ban = crate::route_http_request(
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
        crate::route_http_request("POST", "/api/security/bans/username", None, "{}", &state)
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

    let rehydrated_notes = crate::UserNoteStore::from_persisted(persisted_notes);
    let rehydrated_interests = crate::InterestStore::from_persisted(persisted_interests);
    let rehydrated_security = crate::SecurityState::from_persisted(persisted_bans);
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

    let delete_note = crate::route_http_request(
        "DELETE",
        &format!("/api/users/notes/{note_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete user note");
    assert_eq!(delete_note.status, "200 OK");
    let unban = crate::route_http_request(
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
pub(super) async fn user_note_routes_roll_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let created = crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
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
            crate::route_http_request(method, "/api/users/notes/note-1", None, body, &state)
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
pub(super) async fn interest_routes_roll_back_when_persistence_fails() {
    for path in ["/api/soulseek/interests", "/api/soulseek/hated-interests"] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;

        let response = crate::route_http_request("POST", path, None, r#"{"name":"jazz"}"#, &state)
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
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

        let response = crate::route_http_request("DELETE", path, None, "", &state)
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
pub(super) async fn contact_routes_roll_back_when_persistence_fails() {
    for path in [
        "/api/contacts",
        "/api/contacts/from-discovery",
        "/api/contacts/from-invite",
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;

        let response =
            crate::route_http_request("POST", path, None, r#"{"username":"friend"}"#, &state)
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
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
            crate::route_http_request(method, "/api/contacts/contact-1", None, body, &state)
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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn user_store_rejects_new_records_at_limit_but_updates_existing_users() {
    let mut users = crate::UserStore::with_max_records(1);
    users.watch("alice".to_owned()).unwrap();

    assert!(users.watch("bob".to_owned()).is_none());
    let updated = users
        .apply_status(&crate::UserStatus {
            username: "alice".to_owned(),
            status: 2,
            privileged: false,
        })
        .unwrap();
    assert_eq!(updated.status.as_deref(), Some("2"));
    assert!(users
        .apply_status(&crate::UserStatus {
            username: "remote-unique".to_owned(),
            status: 1,
            privileged: false,
        })
        .is_none());
    assert_eq!(users.records.len(), 1);
    assert_eq!(users.records[0].username, "alice");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn user_store_bounds_and_normalizes_retained_usernames() {
    let oversized_username = "é".repeat(crate::MAX_USER_USERNAME_BYTES);
    let mut users = crate::UserStore::with_max_records(2);
    let watched = users.watch(oversized_username.clone()).unwrap();
    assert!(watched.username.len() <= crate::MAX_USER_USERNAME_BYTES);

    let updated = users
        .apply_status(&crate::UserStatus {
            username: oversized_username.clone(),
            status: 2,
            privileged: false,
        })
        .unwrap();
    assert_eq!(updated.status.as_deref(), Some("2"));
    assert_eq!(users.records.len(), 1);

    let unwatched = users.unwatch(&oversized_username).unwrap();
    assert!(!unwatched.watched);
    assert_eq!(users.records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn users_api_watches_lists_and_unwatches_users() {
    let (state, mut receiver) = test_state();

    let watched = crate::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        "{\"username\":\"friend\"}",
        &state,
    )
    .await
    .expect("watch user");
    assert_eq!(watched.status, "201 Created");
    assert!(watched.body.contains("\"username\":\"friend\""));
    assert!(watched.body.contains("\"watched\":true"));
    assert_eq!(
        receiver.try_recv().expect("watch command"),
        crate::SessionCommand::WatchUser("friend".to_owned())
    );

    let listed = crate::route_http_request("GET", "/api/v0/users", None, "", &state)
        .await
        .expect("list users");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));

    let stats_request = crate::route_http_request(
        "POST",
        "/api/v1/users/friend/stats/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request user stats");
    assert_eq!(stats_request.status, "202 Accepted");
    assert_eq!(
        receiver.try_recv().expect("stats command"),
        crate::SessionCommand::RequestUserStats("friend".to_owned())
    );

    {
        let mut users = state.users.write().await;
        users.apply_stats(
            "friend".to_owned(),
            &crate::UserStats {
                average_speed: 1234,
                upload_count: 5,
                unknown: 0,
                file_count: 42,
                directory_count: 7,
            },
        );
    }
    let listed = crate::route_http_request("GET", "/api/v0/users", None, "", &state)
        .await
        .expect("list users with stats");
    assert!(listed.body.contains("\"average_speed\":1234"));
    assert!(listed.body.contains("\"upload_count\":5"));
    assert!(listed.body.contains("\"file_count\":42"));
    assert!(listed.body.contains("\"directory_count\":7"));

    let unwatched =
        crate::route_http_request("DELETE", "/api/v2/users/friend/watch", None, "", &state)
            .await
            .expect("unwatch user");
    assert_eq!(unwatched.status, "200 OK");
    assert!(unwatched.body.contains("\"watched\":false"));
    assert_eq!(
        receiver.try_recv().expect("unwatch command"),
        crate::SessionCommand::UnwatchUser("friend".to_owned())
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn user_watch_routes_reject_before_mutation_when_dispatch_is_unavailable() {
    let (state, receiver) = test_state();
    drop(receiver);
    let watched = crate::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("unavailable watch response");
    assert_eq!(watched.status, "503 Service Unavailable");
    assert!(watched.body.contains("session manager is not running"));
    assert!(state.users.read().await.records.is_empty());

    let (state, receiver) = test_state();
    state
        .users
        .write()
        .await
        .watch("friend".to_owned())
        .unwrap();
    drop(receiver);
    let unwatched =
        crate::route_http_request("DELETE", "/api/v2/users/friend/watch", None, "", &state)
            .await
            .expect("unavailable unwatch response");
    assert_eq!(unwatched.status, "503 Service Unavailable");
    assert!(unwatched.body.contains("session manager is not running"));
    let users = state.users.read().await;
    assert_eq!(users.records.len(), 1);
    assert!(users.records[0].watched);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn user_watch_routes_roll_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let watched = crate::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("failed watch persistence response");
    assert_eq!(watched.status, "503 Service Unavailable");
    assert!(watched.body.contains("user projection persistence failed"));
    assert!(state.users.read().await.records.is_empty());
    assert!(receiver.try_recv().is_err());

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .users
        .write()
        .await
        .watch("friend".to_owned())
        .unwrap();
    db.close_for_test().await;
    let unwatched =
        crate::route_http_request("DELETE", "/api/v2/users/friend/watch", None, "", &state)
            .await
            .expect("failed unwatch persistence response");
    assert_eq!(unwatched.status, "503 Service Unavailable");
    assert!(unwatched
        .body
        .contains("user projection persistence failed"));
    assert!(state.users.read().await.records[0].watched);
    assert!(receiver.try_recv().is_err());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn user_status_routes_require_exact_user_segments() {
    let (state, _receiver) = test_state();
    {
        let mut users = state.users.write().await;
        let record = users.watch("friend".to_owned()).unwrap();
        users
            .records
            .iter_mut()
            .find(|candidate| candidate.username == record.username)
            .unwrap()
            .status = Some("Online".to_owned());
    }
    state
        .browse
        .write()
        .await
        .request("friend".to_owned())
        .unwrap();

    let exact_status =
        crate::route_http_request("GET", "/api/users/friend/status", None, "", &state)
            .await
            .unwrap();
    let malformed_status =
        crate::route_http_request("GET", "/api/users/friend/extra/status", None, "", &state)
            .await
            .unwrap();
    assert!(exact_status.body.contains("Online"));
    assert_ne!(malformed_status.body, exact_status.body);

    let exact_browse =
        crate::route_http_request("GET", "/api/users/friend/browse/status", None, "", &state)
            .await
            .unwrap();
    let malformed_browse = crate::route_http_request(
        "GET",
        "/api/users/friend/extra/browse/status",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert!(exact_browse.body.contains("\"username\":\"friend\""));
    assert_ne!(malformed_browse.body, exact_browse.body);
    assert_eq!(
        crate::user_route_username("/api/users/friend/status", "/status"),
        Some("friend".to_owned())
    );
    assert_eq!(
        crate::user_route_username("/api/users/friend/extra/status", "/status"),
        None
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn contacts_are_bounded_deduplicated_and_report_discovery_truthfully() {
    let mut store = crate::ContactStore::with_max_records(1);
    let (_, created) = store.create("Alice".to_owned()).unwrap();
    assert!(created);
    let (existing, created) = store.create("alice".to_owned()).unwrap();
    assert!(!created);
    assert_eq!(existing.username, "Alice");
    assert!(store.create("Bob".to_owned()).is_err());
    assert_eq!(store.records.len(), 1);

    let (state, _receiver) = test_state();
    let empty =
        crate::route_http_request("POST", "/api/contacts/from-discovery", None, "{}", &state)
            .await
            .expect("empty discovery response");
    assert_eq!(empty.status, "400 Bad Request");

    let first = crate::route_http_request(
        "POST",
        "/api/contacts/from-discovery",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("first discovery response");
    assert_eq!(first.status, "201 Created");
    assert!(first.body.contains("\"added\":true"));

    let duplicate = crate::route_http_request(
        "POST",
        "/api/contacts/from-discovery",
        None,
        r#"{"username":"FRIEND"}"#,
        &state,
    )
    .await
    .expect("duplicate discovery response");
    assert_eq!(duplicate.status, "200 OK");
    assert!(duplicate.body.contains("\"added\":false"));
    assert_eq!(state.contacts.read().await.records.len(), 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn contacts_and_share_groups_bound_text_and_aggregate_members() {
    let oversized_username = "é".repeat(crate::MAX_USER_USERNAME_BYTES);
    let mut contacts = crate::ContactStore::with_max_records(2);
    let (contact, created) = contacts.create(oversized_username.clone()).unwrap();
    assert!(created);
    assert!(contact.username.len() <= crate::MAX_USER_USERNAME_BYTES);
    let (_, duplicate) = contacts.create(oversized_username.clone()).unwrap();
    assert!(!duplicate);

    let mut groups = crate::ShareGroupStore::with_limits(14, crate::MAX_SHARE_GROUP_MEMBERS + 1);
    let first = groups
        .create(
            "n".repeat(crate::MAX_LIST_NAME_BYTES + 1),
            "d".repeat(crate::MAX_LIST_DESCRIPTION_BYTES + 1),
        )
        .unwrap();
    assert_eq!(first.name.len(), crate::MAX_LIST_NAME_BYTES);
    assert_eq!(first.description.len(), crate::MAX_LIST_DESCRIPTION_BYTES);
    let (_, added) = groups
        .add_member(&first.id, oversized_username.clone())
        .unwrap()
        .unwrap();
    assert!(added);
    assert!(groups.records[0].members[0].username.len() <= crate::MAX_USER_USERNAME_BYTES);

    groups.records[0].members.clear();
    while groups.records.len() < 13 {
        groups.create("group".to_owned(), String::new()).unwrap();
    }
    let template = crate::ShareGroupMember {
        username: "peer".to_owned(),
        added_at: 0,
    };
    for record in groups.records.iter_mut().take(12) {
        record.members = vec![template.clone(); crate::MAX_SHARE_GROUP_MEMBERS];
    }
    groups.records[12].members = vec![
        template;
        crate::MAX_TOTAL_SHARE_GROUP_MEMBERS
            - (12 * crate::MAX_SHARE_GROUP_MEMBERS)
    ];
    let last = groups.create("last".to_owned(), String::new()).unwrap();
    assert!(groups.add_member(&last.id, "overflow".to_owned()).is_err());
    assert_eq!(groups.total_members(), crate::MAX_TOTAL_SHARE_GROUP_MEMBERS);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn contact_update_rejects_duplicate_username() {
    let (state, _receiver) = test_state();
    let first = crate::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"Alice"}"#,
        &state,
    )
    .await
    .unwrap();
    let first_id = serde_json::from_str::<serde_json::Value>(&first.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = crate::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"Bob"}"#,
        &state,
    )
    .await
    .unwrap();
    let second_id = serde_json::from_str::<serde_json::Value>(&second.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let duplicate = crate::route_http_request(
        "PUT",
        &format!("/api/contacts/{first_id}"),
        None,
        r#"{"username":"bOB"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(duplicate.status, "409 Conflict");
    assert_eq!(
        duplicate.body,
        "{\"error\":\"contact username already exists\"}"
    );
    let contacts = state.contacts.read().await;
    assert_eq!(contacts.get(&first_id).unwrap().username, "Alice");
    assert_eq!(contacts.get(&second_id).unwrap().username, "Bob");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn notes_and_interests_bound_growth_and_ids() {
    let mut notes = crate::UserNoteStore::new();
    let bounded_note = notes
        .create(
            "é".repeat(crate::MAX_USER_USERNAME_BYTES),
            "n".repeat(crate::MAX_USER_NOTE_BYTES + 1),
        )
        .unwrap();
    assert!(bounded_note.username.len() <= crate::MAX_USER_USERNAME_BYTES);
    assert_eq!(bounded_note.note.len(), crate::MAX_USER_NOTE_BYTES);
    notes.records.clear();
    for index in 0..crate::MAX_USER_NOTES {
        notes
            .create(format!("user-{index}"), "note".to_owned())
            .unwrap();
    }
    assert!(notes.create("overflow".to_owned(), String::new()).is_none());
    let mut exhausted_notes = crate::UserNoteStore::new();
    exhausted_notes.next_id = u64::MAX;
    assert_eq!(
        exhausted_notes
            .create("user".to_owned(), String::new())
            .unwrap()
            .id,
        format!("note-{}", u64::MAX)
    );

    let mut interests = crate::InterestStore::new();
    let (bounded_interest, created) = interests
        .add_liked("i".repeat(crate::MAX_INTEREST_NAME_BYTES + 1))
        .unwrap();
    assert!(created);
    assert_eq!(bounded_interest.name.len(), crate::MAX_INTEREST_NAME_BYTES);
    interests.liked.clear();
    interests.next_id = 1;
    let (liked, created) = interests.add_liked("Ambient".to_owned()).unwrap();
    assert!(created);
    let (duplicate_liked, created) = interests.add_liked("ambient".to_owned()).unwrap();
    assert!(!created);
    assert_eq!(duplicate_liked.id, liked.id);
    let (hated, created) = interests.add_hated("Noise".to_owned()).unwrap();
    assert!(created);
    let (duplicate_hated, created) = interests.add_hated("NOISE".to_owned()).unwrap();
    assert!(!created);
    assert_eq!(duplicate_hated.id, hated.id);
    assert_eq!(interests.liked.len(), 1);
    assert_eq!(interests.hated.len(), 1);
    interests.next_id = u64::MAX;
    assert_eq!(
        interests.add_liked("Jazz".to_owned()).unwrap().0.id,
        format!("liked-{}", u64::MAX)
    );
    assert_eq!(
        interests.add_hated("Pop".to_owned()).unwrap().0.id,
        "hated-3"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn private_message_auto_response_classifier_and_cooldown_are_bounded() {
    for candidate in [
        "Are you human?",
        "Please prove you are not a bot",
        "Human verification challenge",
        "You are not sharing anything; share more files",
    ] {
        assert!(
            crate::is_private_message_auto_response_candidate(candidate),
            "{candidate}"
        );
    }
    for ordinary in [
        "hello",
        "what music do you like?",
        "the robots album is good",
    ] {
        assert!(
            !crate::is_private_message_auto_response_candidate(ordinary),
            "{ordinary}"
        );
    }
    assert!(!crate::is_private_message_auto_response_candidate(
        &"x".repeat(crate::MAX_MESSAGE_BODY_BYTES + 1)
    ));

    let mut tracker = crate::PrivateMessageAutoResponseTracker::default();
    assert!(tracker.should_respond("Peer", 1_000, 600));
    assert!(!tracker.should_respond("peer", 1_599, 600));
    tracker.release(" PEER ");
    assert!(tracker.should_respond("peer", 1_599, 600));
    assert!(!tracker.should_respond("PEER", 1_600, 600));
    assert!(tracker.should_respond("PEER", 2_199, 600));
    for index in 0..=crate::MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS {
        assert!(tracker.should_respond(&format!("peer-{index}"), 2_000, 600));
    }
    assert_eq!(
        tracker.len(),
        crate::MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn private_message_auto_response_settings_are_runtime_mutable_and_redacted() {
    let (state, _receiver) = test_state();
    let initial = crate::route_http_request(
        "GET",
        "/api/private-message-auto-response",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(initial.status, "200 OK");
    assert!(initial.body.contains(r#""enabled":false"#));
    assert!(!initial.body.contains("temporarily unavailable"));

    let updated = crate::route_http_request(
        "PUT",
        "/api/private-message-auto-response",
        None,
        r#"{"enabled":true,"message":"Runtime human check","cooldownMinutes":15}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "200 OK");
    assert!(updated.body.contains(r#""enabled":true"#));
    assert!(updated.body.contains(r#""cooldown_minutes":15"#));
    assert!(!updated.body.contains("Runtime human check"));
    let settings = state.private_message_auto_response_settings.read().await;
    assert!(settings.enabled);
    assert_eq!(settings.message, "Runtime human check");
    assert_eq!(settings.cooldown_minutes, 15);
    drop(settings);

    assert!(state
        .private_message_auto_responses
        .write()
        .await
        .should_respond("peer", crate::unix_timestamp(), 60));
    let disabled = crate::route_http_request(
        "PUT",
        "/api/private-message-auto-response",
        None,
        r#"{"enabled":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(disabled.status, "200 OK");
    assert!(state.private_message_auto_responses.read().await.is_empty());

    let invalid = crate::route_http_request(
        "PUT",
        "/api/private-message-auto-response",
        None,
        r#"{"cooldownMinutes":0}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn users_api_rejects_missing_username() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("POST", "/api/v0/users/watch", None, "{}", &state)
        .await
        .expect("bad user watch");

    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(response.body, "{\"error\":\"username is required\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn user_projection_state_persists_and_rehydrates_records() {
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
        "/api/v0/users/watch",
        None,
        "{\"username\":\"friend\"}",
        &state,
    )
    .await
    .expect("watch user");
    assert_eq!(created.status, "201 Created");
    assert_eq!(
        receiver.try_recv().expect("watch command"),
        crate::SessionCommand::WatchUser("friend".to_owned())
    );

    let mut records = db
        .list_user_projections(10, 0)
        .await
        .expect("list persisted users");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].username, "friend");
    assert!(records[0].watched);
    records[0].status = Some("Online".to_owned());
    records[0].average_speed = Some(2048);
    records[0].upload_count = Some(7);
    records[0].file_count = Some(123);
    records[0].directory_count = Some(4);
    db.upsert_user_projection(&records[0])
        .await
        .expect("update persisted user");

    let rehydrated = crate::UserStore::from_persisted(
        db.list_user_projections(10, 0)
            .await
            .expect("reload persisted users"),
    );
    assert_eq!(rehydrated.records.len(), 1);
    assert_eq!(rehydrated.records[0].username, "friend");
    assert_eq!(rehydrated.records[0].status.as_deref(), Some("Online"));
    assert_eq!(rehydrated.records[0].file_count, Some(123));
    assert!(rehydrated.json().contains("\"watched\":true"));

    let stats = crate::database_stats_value(&state).await;
    assert_eq!(stats["users"], 1);
    assert_eq!(stats["persisted"]["users"], 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn message_store_evicts_oldest_records_at_limit() {
    let mut messages = crate::MessageStore::with_max_records(2);
    messages.add("alice".to_owned(), "inbound", "first".to_owned());
    let second = messages.add("bob".to_owned(), "outbound", "second".to_owned());
    let third = messages.add("carol".to_owned(), "inbound", "third".to_owned());

    assert_eq!(messages.records.len(), 2);
    assert_eq!(messages.records[0].id, second.id);
    assert_eq!(messages.records[1].id, third.id);
    assert!(messages.ack(second.id).is_some());
    assert!(messages.ack(1).is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn message_store_bounds_live_and_rehydrated_text_fields() {
    let oversized_username = "é".repeat(crate::MAX_MESSAGE_USERNAME_BYTES);
    let oversized_body = "b".repeat(crate::MAX_MESSAGE_BODY_BYTES + 1);
    let mut messages = crate::MessageStore::with_max_records(2);
    let live = messages.add(
        oversized_username.clone(),
        "inbound",
        oversized_body.clone(),
    );
    assert!(live.username.len() <= crate::MAX_MESSAGE_USERNAME_BYTES);
    assert!(live.username.is_char_boundary(live.username.len()));
    assert_eq!(live.body.len(), crate::MAX_MESSAGE_BODY_BYTES);

    let rehydrated = crate::MessageStore::from_persisted(vec![
        crate::persistence::MessageRecord {
            id: "1".to_owned(),
            username: oversized_username.clone(),
            content: oversized_body.clone(),
            direction: "incoming".to_owned(),
            read: false,
            created_at: 1,
            source_id: None,
            source_timestamp: None,
            was_replayed: false,
        },
        crate::persistence::MessageRecord {
            id: "2".to_owned(),
            username: oversized_username,
            content: oversized_body,
            direction: "incoming".to_owned(),
            read: false,
            created_at: 1,
            source_id: None,
            source_timestamp: None,
            was_replayed: false,
        },
    ]);
    assert!(rehydrated.records[0].username.len() <= crate::MAX_MESSAGE_USERNAME_BYTES);
    assert_eq!(
        rehydrated.records[0].body.len(),
        crate::MAX_MESSAGE_BODY_BYTES
    );
    assert!(rehydrated.records[1].created_at_ms > rehydrated.records[0].created_at_ms);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn messages_api_records_lists_and_acks_messages() {
    let (state, mut receiver) = test_state();

    let outbound = crate::route_http_request(
        "POST",
        "/api/v0/messages",
        None,
        "{\"username\":\"friend\",\"body\":\"hello\"}",
        &state,
    )
    .await
    .expect("outbound message");
    assert_eq!(outbound.status, "201 Created");
    assert!(outbound.body.contains("\"direction\":\"outbound\""));
    assert!(outbound.body.contains("\"acknowledged\":false"));
    assert_eq!(
        receiver.try_recv().expect("message command"),
        crate::SessionCommand::MessageUser {
            username: "friend".to_owned(),
            body: "hello".to_owned(),
        }
    );

    let inbound = crate::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        "{\"username\":\"friend\",\"body\":\"hi\"}",
        &state,
    )
    .await
    .expect("inbound message");
    assert_eq!(inbound.status, "201 Created");
    assert!(inbound.body.contains("\"direction\":\"inbound\""));

    let listed = crate::route_http_request("GET", "/api/v0/messages/friend", None, "", &state)
        .await
        .expect("list messages");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"body\":\"hello\""));
    assert!(listed.body.contains("\"body\":\"hi\""));

    let filtered = crate::route_http_request(
        "GET",
        "/api/v0/messages?username=friend&direction=inbound&q=hi&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered messages");
    assert_eq!(filtered.status, "200 OK");
    assert!(filtered.body.contains("\"filtered_count\":1"));
    assert!(filtered.body.contains("\"direction\":\"inbound\""));
    assert!(!filtered.body.contains("\"body\":\"hello\""));

    let acked = crate::route_http_request("POST", "/api/v0/messages/1/ack", None, "", &state)
        .await
        .expect("ack message");
    assert_eq!(acked.status, "200 OK");
    assert!(acked.body.contains("\"acknowledged\":true"));
    assert_eq!(
        receiver.try_recv().expect("ack command"),
        crate::SessionCommand::MessageAcked { id: 1 }
    );

    let oversized_ack =
        crate::route_http_request("POST", "/api/v0/messages/4294967296/ack", None, "", &state)
            .await
            .expect("oversized ack id");
    assert_eq!(oversized_ack.status, "400 Bad Request");
    assert_eq!(
        oversized_ack.body,
        "{\"error\":\"message id exceeds u32 range\"}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn message_ack_routes_reject_before_mutation_when_dispatch_is_unavailable() {
    for method in ["POST", "PUT"] {
        let (state, receiver) = test_state();
        state.messages.write().await.add(
            "friend".to_owned(),
            "inbound",
            "not acknowledged".to_owned(),
        );
        drop(receiver);

        let response =
            crate::route_http_request(method, "/api/v0/messages/1/ack", None, "", &state)
                .await
                .expect("unavailable ack response");
        assert_eq!(response.status, "503 Service Unavailable", "{method}");
        assert!(
            response.body.contains("session manager is not running"),
            "{method}"
        );
        assert!(!state.messages.read().await.records[0].acknowledged);
        assert!(
            state
                .events
                .read()
                .await
                .records
                .iter()
                .all(|event| event.kind != "message.acked"),
            "{method}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn conversations_batch_sends_multi_user_message() {
    let (state, mut receiver) = test_state();

    let response = crate::route_http_request(
        "POST",
        "/api/conversations/batch",
        None,
        r#"{"usernames":["friend","Friend"," peer "],"body":"hello all"}"#,
        &state,
    )
    .await
    .expect("batch message");

    assert_eq!(response.status, "201 Created");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["count"], 2);
    assert_eq!(json["usernames"][0], "friend");
    assert_eq!(json["usernames"][1], "peer");
    assert_eq!(
        receiver.try_recv().expect("batch message command"),
        crate::SessionCommand::MessageUsers {
            usernames: vec!["friend".to_owned(), "peer".to_owned()],
            body: "hello all".to_owned(),
        }
    );

    let friend_messages =
        crate::route_http_request("GET", "/api/v0/messages/friend", None, "", &state)
            .await
            .expect("friend messages");
    assert!(friend_messages.body.contains("\"body\":\"hello all\""));

    let peer_messages = crate::route_http_request("GET", "/api/v0/messages/peer", None, "", &state)
        .await
        .expect("peer messages");
    assert!(peer_messages.body.contains("\"body\":\"hello all\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn message_routes_roll_back_when_persistence_fails() {
    for (path, body) in [
        ("/api/messages", r#"{"username":"friend","body":"direct"}"#),
        ("/api/conversations/friend", r#"{"body":"conversation"}"#),
        (
            "/api/conversations/batch",
            r#"{"usernames":["friend","peer"],"body":"batch"}"#,
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.messages.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("failed message persistence response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("message persistence failed"),
            "{path}"
        );
        assert_eq!(*state.messages.read().await, previous, "{path}");
        assert!(receiver.try_recv().is_err(), "{path}");
    }

    for (method, path, expected_error) in [
        (
            "POST",
            "/api/v0/messages/1/ack",
            "message acknowledgement persistence failed",
        ),
        (
            "PUT",
            "/api/v0/messages/1/ack",
            "message acknowledgement persistence failed",
        ),
        (
            "PUT",
            "/api/conversations/friend/1",
            "message acknowledgement persistence failed",
        ),
        (
            "PUT",
            "/api/conversations/friend",
            "message acknowledgement persistence failed",
        ),
        (
            "DELETE",
            "/api/conversations/friend",
            "conversation deletion persistence failed",
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .messages
            .write()
            .await
            .add("friend".to_owned(), "inbound", "message".to_owned());
        let previous = state.messages.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .expect("failed message mutation persistence response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(response.body.contains(expected_error), "{method} {path}");
        assert_eq!(*state.messages.read().await, previous, "{method} {path}");
        assert!(receiver.try_recv().is_err(), "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn message_batch_database_write_is_atomic() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let original = crate::persistence::MessageRecord {
        id: "1".to_owned(),
        username: "friend".to_owned(),
        content: "original".to_owned(),
        direction: "outbound".to_owned(),
        read: false,
        created_at: 1,
        source_id: None,
        source_timestamp: None,
        was_replayed: false,
    };
    db.insert_message(&original).await.unwrap();
    let new_record = crate::persistence::MessageRecord {
        id: "2".to_owned(),
        content: "new".to_owned(),
        ..original.clone()
    };
    assert!(db
        .insert_messages(&[new_record, original.clone()])
        .await
        .is_err());
    let persisted = db.list_messages(10, 0).await.unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].id, "1");
    assert_eq!(persisted[0].content, "original");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn conversations_batch_persists_each_outbound_message() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let response = crate::route_http_request(
        "POST",
        "/api/conversations/batch",
        None,
        r#"{"usernames":["friend","peer"],"body":"persisted batch"}"#,
        &state,
    )
    .await
    .expect("batch message");
    assert_eq!(response.status, "201 Created");
    assert!(matches!(
        receiver.try_recv(),
        Ok(crate::SessionCommand::MessageUsers { .. })
    ));

    let mut persisted = db.list_messages(10, 0).await.expect("list messages");
    persisted.sort_by(|left, right| left.username.cmp(&right.username));
    assert_eq!(persisted.len(), 2);
    assert_eq!(persisted[0].username, "friend");
    assert_eq!(persisted[0].content, "persisted batch");
    assert_eq!(persisted[0].direction, "outbound");
    assert_eq!(persisted[1].username, "peer");
    assert_eq!(persisted[1].content, "persisted batch");
    assert_eq!(persisted[1].direction, "outbound");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn conversations_batch_rejects_invalid_recipients() {
    let (state, mut receiver) = test_state();

    let blank = crate::route_http_request(
        "POST",
        "/api/conversations/batch",
        None,
        r#"{"recipients":["friend"," "],"message":"hello"}"#,
        &state,
    )
    .await
    .expect("blank recipient");
    assert_eq!(blank.status, "400 Bad Request");
    assert!(blank
        .body
        .contains("private message recipient must not be blank"));
    assert!(receiver.try_recv().is_err());

    let missing = crate::route_http_request(
        "POST",
        "/api/conversations/batch",
        None,
        r#"{"body":"hello"}"#,
        &state,
    )
    .await
    .expect("missing recipients");
    assert_eq!(missing.status, "400 Bad Request");
    assert_eq!(
        missing.body,
        "{\"error\":\"usernames/recipients array is required\"}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn rooms_api_joins_and_records_messages() {
    let (state, mut receiver) = test_state();
    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }

    let refresh = crate::route_http_request("POST", "/api/v0/rooms/refresh", None, "", &state)
        .await
        .expect("room refresh");
    assert_eq!(refresh.status, "202 Accepted");
    assert_eq!(
        receiver.try_recv().expect("room refresh command"),
        crate::SessionCommand::RefreshRooms
    );

    let joined = crate::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .expect("join room");
    assert_eq!(joined.status, "201 Created");
    assert!(joined.body.contains("\"name\":\"music\""));
    assert!(joined.body.contains("\"joined\":true"));
    assert_eq!(
        receiver.try_recv().expect("join command"),
        crate::SessionCommand::JoinRoom("music".to_owned())
    );

    let message = crate::route_http_request(
        "POST",
        "/api/v0/rooms/music/messages",
        None,
        "{\"username\":\"friend\",\"body\":\"track?\"}",
        &state,
    )
    .await
    .expect("room message");
    assert_eq!(message.status, "200 OK");
    assert!(message.body.contains("\"message_count\":1"));
    assert!(message.body.contains("\"body\":\"track?\""));
    assert_eq!(
        receiver.try_recv().expect("room message command"),
        crate::SessionCommand::SayRoom {
            room: "music".to_owned(),
            body: "track?".to_owned(),
        }
    );

    let ticker = crate::route_http_request(
        "POST",
        "/api/rooms/joined/music/ticker",
        None,
        r#""now playing""#,
        &state,
    )
    .await
    .expect("room ticker");
    assert_eq!(ticker.status, "200 OK");
    let ticker_json = serde_json::from_str::<serde_json::Value>(&ticker.body).unwrap();
    assert_eq!(ticker_json["updated"], true);
    assert_eq!(ticker_json["room"]["ticker"], "now playing");
    assert_eq!(
        receiver.try_recv().expect("room ticker command"),
        crate::SessionCommand::SetRoomTicker {
            room: "music".to_owned(),
            ticker: "now playing".to_owned(),
        }
    );

    let member = crate::route_http_request(
        "POST",
        "/api/rooms/joined/music/members",
        None,
        r#""friend""#,
        &state,
    )
    .await
    .expect("room member");
    assert_eq!(member.status, "200 OK");
    let member_json = serde_json::from_str::<serde_json::Value>(&member.body).unwrap();
    assert_eq!(member_json["updated"], true);
    assert_eq!(member_json["userCount"], 1);
    assert_eq!(member_json["room"]["users"][0], "friend");
    assert_eq!(
        receiver.try_recv().expect("room member command"),
        crate::SessionCommand::AddRoomMember {
            room: "music".to_owned(),
            username: "friend".to_owned(),
        }
    );
    let users =
        crate::route_http_request("GET", "/api/v0/rooms/joined/music/users", None, "", &state)
            .await
            .expect("room member roster");
    assert_eq!(users.status, "200 OK");
    let users_json = serde_json::from_str::<serde_json::Value>(&users.body).unwrap();
    assert_eq!(users_json[0]["username"], "friend");
    assert_eq!(users_json[0]["status"], "Offline");

    let rooms = crate::route_http_request("GET", "/api/v0/rooms", None, "", &state)
        .await
        .expect("list rooms");
    assert_eq!(rooms.status, "200 OK");
    assert!(rooms.body.contains("\"count\":1"));

    let filtered =
        crate::route_http_request("GET", "/api/v0/rooms?joined=true&q=music", None, "", &state)
            .await
            .expect("filtered rooms");
    assert_eq!(filtered.status, "200 OK");
    assert!(filtered.body.contains("\"filtered_count\":1"));
    assert!(filtered.body.contains("\"name\":\"music\""));

    let left = crate::route_http_request("DELETE", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .expect("leave room");
    assert_eq!(left.status, "200 OK");
    assert!(left.body.contains("\"joined\":false"));
    assert_eq!(
        receiver.try_recv().expect("leave room command"),
        crate::SessionCommand::LeaveRoom("music".to_owned())
    );

    let joined_filter =
        crate::route_http_request("GET", "/api/v0/rooms?joined=true&q=music", None, "", &state)
            .await
            .expect("joined room filter");
    assert!(joined_filter.body.contains("\"filtered_count\":0"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn room_subscription_routes_roll_back_when_persistence_fails() {
    for (method, path, body, seeded, expected_error) in [
        (
            "POST",
            "/api/v0/rooms/music/join",
            "",
            false,
            "room subscription persistence failed",
        ),
        (
            "POST",
            "/api/rooms/joined",
            r#"{"room":"music"}"#,
            false,
            "room subscription persistence failed",
        ),
        (
            "DELETE",
            "/api/v0/rooms/music/join",
            "",
            true,
            "room unsubscription persistence failed",
        ),
        (
            "DELETE",
            "/api/rooms/joined/music",
            "",
            true,
            "room unsubscription persistence failed",
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with(
                    "SLSKR_CONTROLLER_PROFILE",
                    if path.starts_with("/api/v0/") {
                        "legacy"
                    } else {
                        "native"
                    },
                )
                .with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        if seeded {
            state.rooms.write().await.join("music".to_owned()).unwrap();
        }
        let previous = state.rooms.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed room persistence response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(response.body.contains(expected_error), "{method} {path}");
        assert_eq!(*state.rooms.read().await, previous, "{method} {path}");
        assert!(receiver.try_recv().is_err(), "{method} {path}");
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn room_message_history_evicts_oldest_entries_at_limit() {
    let mut rooms = crate::RoomStore::new();
    rooms.join("music".to_owned()).unwrap();

    for index in 0..(crate::room_store::MAX_ROOM_MESSAGES_PER_ROOM + 5) {
        rooms
            .add_message("music", "friend".to_owned(), format!("message-{index}"))
            .expect("joined room");
    }

    let room = rooms
        .records
        .iter()
        .find(|record| record.name == "music")
        .expect("music room");
    assert_eq!(
        room.messages.len(),
        crate::room_store::MAX_ROOM_MESSAGES_PER_ROOM
    );
    assert_eq!(room.messages.first().unwrap().body, "message-5");
    assert_eq!(
        room.messages.last().unwrap().body,
        format!(
            "message-{}",
            crate::room_store::MAX_ROOM_MESSAGES_PER_ROOM + 4
        )
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn room_store_bounds_text_and_aggregate_retention() {
    let oversized_room = "é".repeat(crate::room_store::MAX_ROOM_NAME_BYTES);
    let oversized_username = "u".repeat(crate::MAX_ROOM_USERNAME_BYTES + 1);
    let oversized_body = "b".repeat(crate::room_store::MAX_ROOM_MESSAGE_BODY_BYTES + 1);
    let oversized_ticker = "t".repeat(crate::room_store::MAX_ROOM_TICKER_BYTES + 1);
    let mut rooms = crate::RoomStore::with_limits(3, crate::room_store::MAX_TOTAL_ROOM_MEMBERS + 1);
    let joined = rooms.join(oversized_room.clone()).unwrap();
    assert!(joined.name.len() <= crate::room_store::MAX_ROOM_NAME_BYTES);
    rooms.join("other".to_owned()).unwrap();

    let message = rooms
        .add_message(&oversized_room, oversized_username.clone(), oversized_body)
        .unwrap();
    assert_eq!(
        message.messages[0].username.len(),
        crate::MAX_ROOM_USERNAME_BYTES
    );
    assert_eq!(
        message.messages[0].body.len(),
        crate::room_store::MAX_ROOM_MESSAGE_BODY_BYTES
    );
    let ticker = rooms.set_ticker(&oversized_room, oversized_ticker).unwrap();
    assert_eq!(
        ticker.ticker.unwrap().len(),
        crate::room_store::MAX_ROOM_TICKER_BYTES
    );

    rooms.records[0].messages = (0..crate::room_store::MAX_TOTAL_ROOM_MESSAGES)
        .map(|created_at| crate::RoomMessageRecord {
            id: created_at as u64 + 1,
            username: "peer".to_owned(),
            body: "message".to_owned(),
            created_at: created_at as u64,
            created_at_ms: created_at as u64 * 1_000,
        })
        .collect();
    let _ = rooms.add_message("other", "peer".to_owned(), "new".to_owned());
    assert_eq!(
        rooms.total_messages(),
        crate::room_store::MAX_TOTAL_ROOM_MESSAGES
    );
    assert_eq!(rooms.records[0].messages.first().unwrap().created_at, 1);

    rooms.records[0].members = (0..crate::room_store::MAX_TOTAL_ROOM_MEMBERS)
        .map(|index| format!("peer-{index}"))
        .collect();
    assert!(rooms.add_member("other", "new-peer".to_owned()).is_err());
    let existing = rooms.records[0].members[0].clone();
    assert!(rooms.add_member(&oversized_room, existing).is_ok());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn room_store_rejects_new_records_at_limit_but_updates_existing_rooms() {
    let mut rooms = crate::RoomStore::with_limits(2, 2);
    rooms.join("one".to_owned()).unwrap();
    rooms.join("two".to_owned()).unwrap();

    assert!(rooms.join("three".to_owned()).is_none());
    assert!(rooms.join("one".to_owned()).unwrap().joined);
    rooms.apply_room_list(&crate::RoomList {
        public_rooms: vec![
            crate::RoomListEntry {
                name: "one".to_owned(),
                user_count: 12,
            },
            crate::RoomListEntry {
                name: "remote-unique".to_owned(),
                user_count: 1,
            },
        ],
        owned_private_rooms: Vec::new(),
        private_rooms: Vec::new(),
        operated_private_rooms: vec!["remote-operated".to_owned()],
    });

    assert_eq!(rooms.records.len(), 2);
    assert_eq!(rooms.records[0].user_count, Some(12));
    assert!(!rooms
        .records
        .iter()
        .any(|room| room.name == "remote-unique"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn room_store_rejects_new_members_at_limit_but_accepts_duplicates() {
    let mut rooms = crate::RoomStore::with_limits(1, 2);
    rooms.join("music".to_owned()).unwrap();
    rooms.add_member("music", "alice".to_owned()).unwrap();
    rooms.add_member("music", "bob".to_owned()).unwrap();

    assert!(rooms.add_member("music", "carol".to_owned()).is_err());
    assert!(rooms.add_member("music", "ALICE".to_owned()).is_ok());
    assert_eq!(rooms.records[0].members, ["alice", "bob"]);
    assert_eq!(
        rooms.records[0]
            .roster
            .iter()
            .map(|user| user.username.as_str())
            .collect::<Vec<_>>(),
        ["alice", "bob"]
    );
    assert_eq!(rooms.records[0].user_count, Some(2));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn room_join_records_local_projection_while_disconnected_or_reconnecting() {
    let (state, mut receiver) = test_state();

    let disconnected =
        crate::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
            .await
            .expect("join while disconnected");
    assert_eq!(disconnected.status, "201 Created");
    assert!(disconnected.body.contains("\"name\":\"music\""));
    assert!(disconnected.body.contains("\"joined\":true"));
    assert!(receiver.try_recv().is_err());

    {
        let mut session = state.session.write().await;
        session.state = "error";
        session.last_error = Some("server receive failed".to_owned());
    }
    let reconnecting =
        crate::route_http_request("POST", "/api/rooms/joined", None, r#""music""#, &state)
            .await
            .expect("join while reconnecting");
    assert_eq!(reconnecting.status, "201 Created");
    assert!(reconnecting.body.contains("\"name\":\"music\""));
    assert!(reconnecting.body.contains("\"messages\":[]"));
    assert!(receiver.try_recv().is_err());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn room_list_projection_tracks_server_metadata() {
    let mut rooms = crate::RoomStore::new();
    rooms.join("public".to_owned());
    rooms.apply_room_list(&crate::RoomList {
        public_rooms: vec![crate::RoomListEntry {
            name: "public".to_owned(),
            user_count: 12,
        }],
        owned_private_rooms: vec![crate::RoomListEntry {
            name: "owned".to_owned(),
            user_count: 2,
        }],
        private_rooms: vec![crate::RoomListEntry {
            name: "private".to_owned(),
            user_count: 3,
        }],
        operated_private_rooms: vec!["private".to_owned(), "orphan-operated".to_owned()],
    });

    let json = rooms.json(None);
    assert!(json.contains("\"name\":\"public\""));
    assert!(json.contains("\"joined\":true"));
    assert!(json.contains("\"kind\":\"public\""));
    assert!(json.contains("\"user_count\":12"));
    assert!(json.contains("\"name\":\"owned\""));
    assert!(json.contains("\"kind\":\"owned_private\""));
    assert!(json.contains("\"operated\":true"));
    assert!(json.contains("\"name\":\"private\""));
    assert!(json.contains("\"kind\":\"private\""));
    assert!(json.contains("\"user_count\":3"));
    assert!(json.contains("\"name\":\"orphan-operated\""));
    assert!(json.contains("\"kind\":\"operated_private\""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn room_join_failure_projection_clears_optimistic_join() {
    let mut rooms = crate::RoomStore::new();
    rooms.join("denied".to_owned());

    let failed = rooms
        .fail_join("denied", "server reported cant-create-room".to_owned())
        .unwrap();

    assert!(!failed.joined);
    assert_eq!(
        failed.last_error.as_deref(),
        Some("server reported cant-create-room")
    );
    assert!(failed.json().contains("\"joined\":false"));
    assert!(failed
        .json()
        .contains("\"last_error\":\"server reported cant-create-room\""));

    let joined = rooms.join("denied".to_owned()).unwrap();
    assert!(joined.joined);
    assert_eq!(joined.last_error, None);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn usernames_are_redacted() {
    assert_eq!(redact_username("tester"), "t***r");
    assert_eq!(redact_username("xy"), "**");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn user_info_picture_is_read_at_response_time() {
    let (state, _session_commands) = test_state();
    let picture = state.config.state_dir.join("profile-picture.bin");
    std::fs::write(&picture, [0_u8, 1, 2, 255]).unwrap();
    *state
        .user_info_picture
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(picture.clone());

    assert_eq!(
        crate::effective_user_info_picture(&state).await,
        Some(vec![0, 1, 2, 255])
    );
    std::fs::write(&picture, [9_u8, 8, 7]).unwrap();
    assert_eq!(
        crate::effective_user_info_picture(&state).await,
        Some(vec![9, 8, 7])
    );
    std::fs::remove_file(picture).unwrap();
    assert_eq!(crate::effective_user_info_picture(&state).await, None);
}
