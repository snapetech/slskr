use super::fixtures::*;

#[tokio::test]
async fn room_join_fast_path_does_not_return_stale_duplicate_projection() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    state.session.write().await.state = "connected";

    let mut rooms = state.rooms.write().await;
    let healthy = rooms.join("music".to_owned()).expect("healthy room");
    rooms.records[0].joined = false;
    rooms.records[0].last_error = Some("stale join failure".to_owned());
    rooms.records.push(healthy);
    drop(rooms);

    let response = crate::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .expect("room join response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"lastError\":null"));
    assert!(!response.body.contains("stale join failure"));
}

#[tokio::test]
async fn contact_update_and_delete_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create contact ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("create contact");
    assert_eq!(created.status, "201 Created");
    let contact_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("contact id")
        .to_owned();

    let persistence_turn = state.contact_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/contacts/{contact_id}");
    let mut update = tokio::spawn(async move {
        crate::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"username":"updated","online":true}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/contacts/{contact_id}");
    let mut delete = tokio::spawn(async move {
        crate::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = crate::route_http_request(
        "GET",
        &format!("/api/contacts/{contact_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("contact read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["username"],
        "friend"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("contact update completes")
        .expect("update contact");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("contact delete completes")
        .expect("delete contact");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.contacts.read().await.get(&contact_id).is_none());
    assert!(!db
        .list_contacts(10, 0)
        .await
        .expect("read final contact rows")
        .iter()
        .any(|contact| contact.id == contact_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn user_note_update_and_delete_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create user-note ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
        "POST",
        "/api/users/notes",
        None,
        r#"{"username":"friend","note":"Original"}"#,
        &state,
    )
    .await
    .expect("create user note");
    assert_eq!(created.status, "201 Created");
    let note_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("user-note id")
        .to_owned();

    let persistence_turn = state.user_note_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let update_path = format!("/api/users/notes/{note_id}");
    let mut update = tokio::spawn(async move {
        crate::route_http_request(
            "PUT",
            &update_path,
            None,
            r#"{"note":"Updated"}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/users/notes/{note_id}");
    let mut delete = tokio::spawn(async move {
        crate::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = crate::route_http_request(
        "GET",
        &format!("/api/users/notes/{note_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("user-note read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["note"],
        "Original"
    );

    drop(persistence_turn);
    let updated = update
        .await
        .expect("user-note update completes")
        .expect("update user note");
    assert_eq!(updated.status, "200 OK");
    let deleted = delete
        .await
        .expect("user-note delete completes")
        .expect("delete user note");
    assert_eq!(deleted.status, "200 OK");
    assert!(state.user_notes.read().await.get(&note_id).is_none());
    assert!(!db
        .list_user_notes(10, 0)
        .await
        .expect("read final user-note rows")
        .iter()
        .any(|note| note.id == note_id));

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}
#[tokio::test]
async fn interest_mutations_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create interest ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.interest_persistence_lock.lock().await;
    let add_state = Arc::clone(&state);
    let mut add = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/soulseek/interests",
            None,
            r#"{"name":"Jazz"}"#,
            &add_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut add)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let mut delete = tokio::spawn(async move {
        crate::route_http_request(
            "DELETE",
            "/api/soulseek/interests/Jazz",
            None,
            "",
            &delete_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let hate_state = Arc::clone(&state);
    let mut hate = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/soulseek/hated-interests",
            None,
            r#"{"name":"Noise"}"#,
            &hate_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut hate)
        .await
        .is_err());

    let liked_read = crate::route_http_request("GET", "/api/soulseek/interests", None, "", &state)
        .await
        .expect("liked-interest read remains available while writes wait");
    assert_eq!(liked_read.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&liked_read.body).unwrap()["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let hated_read =
        crate::route_http_request("GET", "/api/soulseek/hated-interests", None, "", &state)
            .await
            .expect("hated-interest read remains available while writes wait");
    assert_eq!(hated_read.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&hated_read.body).unwrap()["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        add.await
            .expect("interest creation completes")
            .expect("create liked interest")
            .status,
        "201 Created"
    );
    assert_eq!(
        delete
            .await
            .expect("interest deletion completes")
            .expect("delete liked interest")
            .status,
        "200 OK"
    );
    assert_eq!(
        hate.await
            .expect("hated-interest creation completes")
            .expect("create hated interest")
            .status,
        "201 Created"
    );

    let interests = state.interests.read().await;
    assert!(interests.liked.is_empty());
    assert_eq!(interests.hated.len(), 1);
    assert_eq!(interests.hated[0].name, "Noise");
    drop(interests);
    let persisted = db
        .list_interests(10, 0)
        .await
        .expect("read final interest rows");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].kind, "hated");
    assert_eq!(persisted[0].name, "Noise");

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn mesh_interest_mutations_roll_back_when_persistence_fails() {
    let post_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create mesh-interest creation database");
    let (post_state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        post_db.clone(),
    );
    post_db.close_for_test().await;
    let headers = crate::RequestSecurityHeaders::default();
    let created = crate::extended_controller_mutation_response(
        "POST",
        "/api/soulseek/mesh-rendezvous/interest",
        None,
        "",
        &post_state,
        true,
        &headers,
    )
    .await;
    assert_eq!(created.status, "503 Service Unavailable");
    assert!(post_state.interests.read().await.liked.is_empty());

    let delete_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create mesh-interest deletion database");
    let (delete_state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        delete_db.clone(),
    );
    delete_state
        .interests
        .write()
        .await
        .add_liked(crate::MESH_RENDEZVOUS_INTEREST_TAG.to_owned())
        .expect("seed mesh interest");
    delete_db.close_for_test().await;
    let deleted = crate::extended_controller_mutation_response(
        "DELETE",
        "/api/soulseek/mesh-rendezvous/interest",
        None,
        "",
        &delete_state,
        true,
        &headers,
    )
    .await;
    assert_eq!(deleted.status, "503 Service Unavailable");
    let interests = delete_state.interests.read().await;
    assert_eq!(interests.liked.len(), 1);
    assert_eq!(interests.liked[0].name, crate::MESH_RENDEZVOUS_INTEREST_TAG);
}

#[tokio::test]
async fn now_playing_update_and_clear_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create now-playing ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
        "POST",
        "/api/nowplaying",
        None,
        r#"{"username":"friend","artist":"Artist","title":"Original"}"#,
        &state,
    )
    .await
    .expect("create now-playing record");
    assert_eq!(created.status, "200 OK");

    let persistence_turn = state.now_playing_persistence_lock.lock().await;
    let update_state = Arc::clone(&state);
    let mut update = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/nowplaying",
            None,
            r#"{"username":"friend","artist":"Artist","title":"Updated"}"#,
            &update_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut update)
        .await
        .is_err());

    let clear_state = Arc::clone(&state);
    let mut clear = tokio::spawn(async move {
        crate::route_http_request("DELETE", "/api/nowplaying", None, "", &clear_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut clear)
        .await
        .is_err());

    let readable = crate::route_http_request("GET", "/api/nowplaying", None, "", &state)
        .await
        .expect("now-playing read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    let readable_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    let records = readable_json["now_playing"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["title"], "Original");

    drop(persistence_turn);
    assert_eq!(
        update
            .await
            .expect("now-playing update completes")
            .expect("update now-playing record")
            .status,
        "200 OK"
    );
    assert_eq!(
        clear
            .await
            .expect("now-playing clear completes")
            .expect("clear now-playing records")
            .status,
        "200 OK"
    );
    assert!(state.now_playing.read().await.records.is_empty());
    assert!(db
        .list_now_playing(10, 0)
        .await
        .expect("read final now-playing rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn message_create_and_conversation_delete_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create message ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.message_persistence_lock.lock().await;
    let create_state = Arc::clone(&state);
    let mut create = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/conversations/friend",
            None,
            r#"{"message":"queued message"}"#,
            &create_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut create)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let mut delete = tokio::spawn(async move {
        crate::route_http_request(
            "DELETE",
            "/api/conversations/friend",
            None,
            "",
            &delete_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = crate::route_http_request("GET", "/api/conversations/friend", None, "", &state)
        .await
        .expect("conversation read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["messages"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        create
            .await
            .expect("conversation message creation completes")
            .expect("create conversation message")
            .status,
        "200 OK"
    );
    assert_eq!(
        delete
            .await
            .expect("conversation deletion completes")
            .expect("delete conversation")
            .status,
        "200 OK"
    );
    assert!(state.messages.read().await.records.is_empty());
    assert!(db
        .list_messages(10, 0)
        .await
        .expect("read final message rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn room_subscription_join_and_leave_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create room subscription ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.room_persistence_lock.lock().await;
    let join_state = Arc::clone(&state);
    let mut join = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/rooms/joined",
            None,
            r#"{"room":"music"}"#,
            &join_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut join)
        .await
        .is_err());

    let leave_state = Arc::clone(&state);
    let mut leave = tokio::spawn(async move {
        crate::route_http_request("DELETE", "/api/rooms/joined/music", None, "", &leave_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut leave)
        .await
        .is_err());

    let readable = crate::route_http_request("GET", "/api/rooms/joined", None, "", &state)
        .await
        .expect("room read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(!readable.body.contains("music"));

    drop(persistence_turn);
    assert_eq!(
        join.await
            .expect("room join completes")
            .expect("join room")
            .status,
        "201 Created"
    );
    assert_eq!(
        leave
            .await
            .expect("room leave completes")
            .expect("leave room")
            .status,
        "200 OK"
    );
    assert!(!state
        .rooms
        .read()
        .await
        .records
        .iter()
        .any(|room| room.name == "music" && room.joined));
    assert!(db
        .list_subscribed_rooms()
        .await
        .expect("read final subscribed rooms")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn compatibility_message_ack_rolls_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create compatibility message database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let record =
        state
            .messages
            .write()
            .await
            .add("friend".to_owned(), "inbound", "hello".to_owned());
    let path = format!("/api/conversations/friend/{}", record.id);
    db.close_for_test().await;

    let response = crate::extended_controller_mutation_response(
        "PUT",
        &path,
        None,
        "",
        &state,
        true,
        &crate::RequestSecurityHeaders::default(),
    )
    .await;
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(!state.messages.read().await.records[0].acknowledged);
}

#[tokio::test]
async fn compatibility_message_ack_wrong_username_does_not_mutate_message() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create compatibility message database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let record =
        state
            .messages
            .write()
            .await
            .add("friend".to_owned(), "inbound", "hello".to_owned());
    let path = format!("/api/conversations/stranger/{}", record.id);

    let response = crate::extended_controller_mutation_response(
        "PUT",
        &path,
        None,
        "",
        &state,
        true,
        &crate::RequestSecurityHeaders::default(),
    )
    .await;

    assert_eq!(response.status, "404 Not Found");
    assert!(!state.messages.read().await.records[0].acknowledged);
}

#[tokio::test]
async fn pod_mutations_leave_pod_reads_available_while_waiting_for_channel_store() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    for pod_id in ["pod-lock-put", "pod-lock-delete"] {
        let create = serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": "Before",
                "isPublic": true,
                "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
            }
        })
        .to_string();
        let response = crate::route_http_request("POST", "/api/v0/pods", None, &create, &state)
            .await
            .expect("create pod for lock-order regression");
        assert_eq!(response.status, "201 Created");
    }

    let update_path = "/api/v0/pods/pod-lock-put";
    let update_body = serde_json::json!({
        "pod": {
            "podId": "pod-lock-put",
            "name": "After",
            "isPublic": true,
            "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
        }
    })
    .to_string();
    let channel_guard = state.pod_channels.write().await;
    let update = crate::route_http_request("PUT", update_path, None, &update_body, &state);
    tokio::pin!(update);
    tokio::select! {
        biased;
        _ = &mut update => panic!("pod update should wait for the held channel store"),
        _ = tokio::task::yield_now() => {}
    }
    let readable = tokio::time::timeout(
        Duration::from_secs(1),
        crate::route_http_request("GET", update_path, None, "", &state),
    )
    .await
    .expect("pod read should not wait behind a channel-store waiter")
    .expect("read pod during update wait");
    assert_eq!(readable.status, "200 OK");
    drop(channel_guard);
    let updated = tokio::time::timeout(Duration::from_secs(1), &mut update)
        .await
        .expect("pod update should finish after the channel store is released")
        .expect("update pod after channel store is released");
    assert_eq!(updated.status, "200 OK");

    let delete_path = "/api/v0/pods/pod-lock-delete";
    let channel_guard = state.pod_channels.write().await;
    let delete = crate::route_http_request("DELETE", delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("pod delete should wait for the held channel store"),
        _ = tokio::task::yield_now() => {}
    }
    let readable = tokio::time::timeout(
        Duration::from_secs(1),
        crate::route_http_request("GET", delete_path, None, "", &state),
    )
    .await
    .expect("pod read should not wait behind a channel-store waiter")
    .expect("read pod during delete wait");
    assert_eq!(readable.status, "200 OK");
    drop(channel_guard);
    let deleted = tokio::time::timeout(Duration::from_secs(1), &mut delete)
        .await
        .expect("pod delete should finish after the channel store is released")
        .expect("delete pod after channel store is released");
    assert_eq!(deleted.status, "204 No Content");

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn pod_membership_acceptance_keeps_pending_reads_available_while_room_store_waits() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    state
        .rooms
        .write()
        .await
        .join("pod:workflow-lock".to_owned())
        .expect("create room for membership workflow lock regression");
    state.pod_membership_workflow.write().await.set_role(
        "pod:workflow-lock",
        "owner-peer",
        "owner".to_owned(),
    );

    let join = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"applicant"}"#,
        &state,
    )
    .await
    .expect("create pending join request");
    assert_eq!(join.status, "200 OK");

    let room_guard = state.rooms.write().await;
    let acceptance = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"applicant","acceptedRole":"moderator","acceptorPeerId":"owner-peer"}"#,
        &state,
    );
    tokio::pin!(acceptance);
    tokio::select! {
        biased;
        _ = &mut acceptance => panic!("membership acceptance should wait for the held room store"),
        _ = tokio::task::yield_now() => {}
    }

    let pending = tokio::time::timeout(
        Duration::from_secs(1),
        crate::route_http_request(
            "GET",
            "/api/v0/podcore/membership/join/pending/pod%3Aworkflow-lock",
            None,
            "",
            &state,
        ),
    )
    .await
    .expect("pending request read must remain available while acceptance waits")
    .expect("read pending join request");
    assert_eq!(pending.status, "200 OK");
    assert!(pending.body.contains("applicant"));

    drop(room_guard);
    let accepted = tokio::time::timeout(Duration::from_secs(1), &mut acceptance)
        .await
        .expect("acceptance should finish once the room store is released")
        .expect("accept pending join request");
    assert_eq!(accepted.status, "200 OK");
    assert!(state
        .pod_membership_workflow
        .read()
        .await
        .pending_joins("pod:workflow-lock")
        .is_empty());
    assert!(state
        .rooms
        .read()
        .await
        .records
        .iter()
        .find(|room| room.name == "pod:workflow-lock")
        .is_some_and(|room| room.members.iter().any(|member| member == "applicant")));

    let second_join = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"second-applicant"}"#,
        &state,
    )
    .await
    .expect("create second pending join request");
    assert_eq!(second_join.status, "200 OK");
    state.pod_membership_workflow.write().await.set_role(
        "pod:workflow-lock",
        "owner-peer",
        "owner".to_owned(),
    );

    let room_guard = state.rooms.write().await;
    let revoked_acceptance = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow-lock","peerId":"second-applicant","acceptedRole":"moderator","acceptorPeerId":"owner-peer"}"#,
        &state,
    );
    tokio::pin!(revoked_acceptance);
    tokio::select! {
        biased;
        _ = &mut revoked_acceptance => panic!("membership acceptance should wait for the held room store"),
        _ = tokio::task::yield_now() => {}
    }
    state
        .pod_membership_workflow
        .write()
        .await
        .remove_role("pod:workflow-lock", "owner-peer");
    drop(room_guard);
    let rejected = tokio::time::timeout(Duration::from_secs(1), &mut revoked_acceptance)
        .await
        .expect("revoked acceptance should finish after the room store is released")
        .expect("revoked acceptance response");
    assert_eq!(rejected.status, "400 Bad Request");
    assert_eq!(
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_joins("pod:workflow-lock")
            .len(),
        1
    );
    assert!(!state
        .rooms
        .read()
        .await
        .records
        .iter()
        .find(|room| room.name == "pod:workflow-lock")
        .is_some_and(|room| room
            .members
            .iter()
            .any(|member| member == "second-applicant")));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn pod_channel_append_queued_after_channel_removal_is_rejected() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let create = serde_json::json!({
        "pod": {
            "podId": "pod:append-race",
            "name": "Before",
            "isPublic": true,
            "channels": [
                {"channelId": "general", "kind": 0, "name": "General"},
                {"channelId": "removed", "kind": 1, "name": "Removed"}
            ]
        }
    })
    .to_string();
    let created = crate::route_http_request("POST", "/api/v0/pods", None, &create, &state)
        .await
        .expect("create pod for queued append regression");
    assert_eq!(created.status, "201 Created");

    let channel_guard = state.pod_channels.write().await;
    let update = serde_json::json!({
        "pod": {
            "podId": "pod:append-race",
            "name": "After",
            "isPublic": true,
            "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
        }
    })
    .to_string();
    let update =
        crate::route_http_request("PUT", "/api/v0/pods/pod:append-race", None, &update, &state);
    tokio::pin!(update);
    tokio::select! {
        biased;
        _ = &mut update => panic!("pod update should wait for the held channel store"),
        _ = tokio::task::yield_now() => {}
    }

    let append_body = serde_json::json!({
        "senderPeerId": "tester",
        "body": "must not outlive the removed channel"
    })
    .to_string();
    let append = crate::route_http_request(
        "POST",
        "/api/v0/pods/pod:append-race/channels/removed/messages",
        None,
        &append_body,
        &state,
    );
    tokio::pin!(append);
    tokio::select! {
        biased;
        _ = &mut append => panic!("pod message append should queue behind the update"),
        _ = tokio::task::yield_now() => {}
    }

    drop(channel_guard);
    let updated = tokio::time::timeout(Duration::from_secs(1), &mut update)
        .await
        .expect("pod update should complete before queued append")
        .expect("update pod after channel store is released");
    assert_eq!(updated.status, "200 OK");
    let appended = tokio::time::timeout(Duration::from_secs(1), &mut append)
        .await
        .expect("queued append should finish after the update")
        .expect("append route should return a response");
    assert_eq!(appended.status, "404 Not Found");
    assert!(state
        .pod_channels
        .read()
        .await
        .list("pod:append-race", "removed", None)
        .is_empty());
    let reloaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
        .expect("reload channel state after queued append");
    assert!(reloaded.list("pod:append-race", "removed", None).is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn pod_update_restores_parent_when_channel_cleanup_fails() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let create = serde_json::json!({
        "pod": {
            "podId": "pod:cleanup-failure",
            "name": "Before",
            "isPublic": true,
            "channels": [
                {"channelId": "general", "kind": 0, "name": "General"},
                {"channelId": "removed", "kind": 1, "name": "Removed"}
            ]
        }
    })
    .to_string();
    let created = crate::route_http_request("POST", "/api/v0/pods", None, &create, &state)
        .await
        .expect("create pod for rollback regression");
    assert_eq!(created.status, "201 Created");
    state
        .pod_channels
        .write()
        .await
        .append(
            "pod:cleanup-failure".to_owned(),
            "removed".to_owned(),
            "peer".to_owned(),
            "message".to_owned(),
            "signature".to_owned(),
            1,
        )
        .expect("persist message in channel that will be removed");

    let channel_path = state.config.state_dir.join("pod-channel-messages.json");
    let preserved_channel_path = state
        .config
        .state_dir
        .join("pod-channel-messages.json.saved");
    fs::rename(&channel_path, &preserved_channel_path)
        .expect("move channel file before forcing cleanup failure");
    fs::create_dir(&channel_path).expect("make channel state path unwritable");
    let update = serde_json::json!({
        "pod": {
            "podId": "pod:cleanup-failure",
            "name": "After",
            "isPublic": true,
            "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
        }
    })
    .to_string();
    let response = crate::route_http_request(
        "PUT",
        "/api/v0/pods/pod:cleanup-failure",
        None,
        &update,
        &state,
    )
    .await
    .expect("failed channel cleanup response");
    assert_eq!(response.status, "500 Internal Server Error");
    fs::remove_dir(&channel_path).expect("remove directory that blocked channel persistence");
    fs::rename(&preserved_channel_path, &channel_path)
        .expect("restore channel file after rollback regression");

    let pod = state
        .pods
        .read()
        .await
        .get("pod:cleanup-failure")
        .expect("pod remains in memory after channel cleanup failure");
    assert_eq!(pod.name, "Before");
    assert_eq!(pod.channels.len(), 2);
    assert_eq!(
        state
            .pod_channels
            .read()
            .await
            .list("pod:cleanup-failure", "removed", None)
            .len(),
        1
    );
    let reloaded =
        crate::pods::PodStore::load(&state.config.state_dir).expect("reload restored parent state");
    assert_eq!(
        reloaded
            .get("pod:cleanup-failure")
            .expect("persisted pod remains after rollback")
            .channels
            .len(),
        2
    );
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[test]
fn pod_startup_prunes_channel_messages_left_by_interrupted_parent_commit() {
    let root = std::env::temp_dir().join(format!(
        "slskr-pod-cross-file-recovery-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create pod recovery state directory");
    let mut pods = crate::pods::PodStore::empty(&root);
    let mut channels = crate::pod_channels::PodChannelStore::empty(&root);

    let mut update_pod = serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
        "podId": "pod:recovery-update",
        "name": "Before update",
        "channels": [
            {"channelId": "general", "kind": 0, "name": "General"},
            {"channelId": "removed", "kind": 1, "name": "Removed"}
        ]
    }))
    .expect("deserialize update recovery pod");
    pods.create(update_pod.clone(), "owner".to_owned())
        .expect("create update recovery pod");
    channels
        .append(
            "pod:recovery-update".to_owned(),
            "removed".to_owned(),
            "peer".to_owned(),
            "message".to_owned(),
            "signature".to_owned(),
            1,
        )
        .expect("persist channel message removed by update");
    update_pod
        .channels
        .retain(|channel| channel.channel_id == "general");
    pods.update("pod:recovery-update", update_pod)
        .expect("persist parent update before channel cleanup");

    let deleted_pod = serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
        "podId": "pod:recovery-delete",
        "name": "Deleted pod",
        "channels": [{"channelId": "general", "kind": 0, "name": "General"}]
    }))
    .expect("deserialize delete recovery pod");
    pods.create(deleted_pod, "owner".to_owned())
        .expect("create delete recovery pod");
    channels
        .append(
            "pod:recovery-delete".to_owned(),
            "general".to_owned(),
            "peer".to_owned(),
            "message".to_owned(),
            "signature".to_owned(),
            2,
        )
        .expect("persist channel message removed by pod deletion");
    pods.delete("pod:recovery-delete")
        .expect("persist parent deletion before channel cleanup");
    drop(channels);
    drop(pods);

    let (channels, pods) = crate::daemon_serve::load_pod_stores(&root, false)
        .expect("recover pod/channel state during startup");
    assert_eq!(
        pods.get("pod:recovery-update")
            .expect("updated pod survives recovery")
            .channels
            .len(),
        1
    );
    assert!(pods.get("pod:recovery-delete").is_none());
    assert!(channels
        .list("pod:recovery-update", "removed", None)
        .is_empty());
    assert!(channels
        .list("pod:recovery-delete", "general", None)
        .is_empty());
    drop(channels);
    drop(pods);

    let persisted_channels =
        crate::pod_channels::PodChannelStore::load(&root).expect("reload repaired channel store");
    assert!(persisted_channels
        .list("pod:recovery-update", "removed", None)
        .is_empty());
    assert!(persisted_channels
        .list("pod:recovery-delete", "general", None)
        .is_empty());
    fs::remove_dir_all(root).expect("remove pod recovery state directory");
}

#[tokio::test]
async fn share_grant_creation_rechecks_collection_after_waiting_for_grant_store() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-grant-parent-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create share grant parent directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create share grant parent database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Concurrent grant collection"}"#,
        &state,
    )
    .await
    .expect("create persisted collection");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();

    let grant_store = state.share_grants.write().await;
    let delete_path = format!("/api/v0/collections/{collection_id}");
    let delete = crate::route_http_request("DELETE", &delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("collection delete should wait for the held grant store"),
        _ = tokio::task::yield_now() => {}
    }

    let grant_body = serde_json::json!({
        "collection_id": collection_id,
        "username": "recipient",
        "permissions": "read"
    })
    .to_string();
    let create_grant =
        crate::route_http_request("POST", "/api/v0/share-grants", None, &grant_body, &state);
    tokio::pin!(create_grant);
    tokio::select! {
        biased;
        _ = &mut create_grant => panic!("share grant creation should wait for the held grant store"),
        _ = tokio::task::yield_now() => {}
    }

    drop(grant_store);
    let (deleted, created) = tokio::join!(&mut delete, &mut create_grant);
    let deleted = deleted.expect("delete collection after grant-store release");
    let created = created.expect("finish grant creation after collection delete");
    assert_eq!(deleted.status, "204 No Content");
    assert_eq!(created.status, "404 Not Found");
    assert!(state.share_grants.read().await.records.is_empty());
    assert!(db
        .list_collections(100, 0)
        .await
        .expect("list persisted collections")
        .is_empty());
    assert!(db
        .list_share_grants(100, 0)
        .await
        .expect("list persisted grants")
        .is_empty());

    let orphan = crate::persistence::ShareGrantRecord {
        id: "orphan-grant".to_owned(),
        collection_id,
        username: "recipient".to_owned(),
        shared_at: 1,
        permissions: "read".to_owned(),
    };
    assert!(db.upsert_share_grant(&orphan).await.is_err());
    assert!(db
        .list_share_grants(100, 0)
        .await
        .expect("confirm rejected orphan grant")
        .is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove share grant parent directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn stale_collection_snapshot_cannot_resurrect_deleted_parent() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-collection-parent-consistency-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create collection parent directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create collection parent database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let created = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Persisted collection"}"#,
        &state,
    )
    .await
    .expect("create collection through production route");
    assert_eq!(created.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();
    let stale_record = state
        .collections
        .read()
        .await
        .get(&collection_id)
        .expect("created collection snapshot");

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection through production route");
    assert_eq!(deleted.status, "204 No Content");
    assert!(db
        .list_collections(100, 0)
        .await
        .expect("confirm collection deletion")
        .is_empty());

    assert!(
        crate::collection_store::persist_collection_checked(&state, &stale_record)
            .await
            .is_err()
    );
    assert!(db
        .list_collections(100, 0)
        .await
        .expect("confirm stale snapshot did not recreate collection")
        .is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove collection parent directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn collection_delete_waits_for_persistence_turn_without_blocking_reads() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-collection-persistence-order-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create collection persistence directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create collection persistence database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let created = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Persistence order"}"#,
        &state,
    )
    .await
    .expect("create collection");
    assert_eq!(created.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();

    let persistence_turn = state.collection_grant_persistence_lock.lock().await;
    let delete_path = format!("/api/v0/collections/{collection_id}");
    let delete = crate::route_http_request("DELETE", &delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("collection delete must wait for its persistence turn"),
        _ = tokio::task::yield_now() => {}
    }

    let readable = crate::route_http_request("GET", &delete_path, None, "", &state)
        .await
        .expect("read collection while delete waits for persistence turn");
    assert_eq!(readable.status, "200 OK");
    assert_eq!(db.list_collections(100, 0).await.unwrap().len(), 1);

    drop(persistence_turn);
    let deleted = tokio::time::timeout(Duration::from_secs(1), &mut delete)
        .await
        .expect("delete completes after persistence turn is released")
        .expect("delete collection");
    assert_eq!(deleted.status, "204 No Content");
    assert!(db.list_collections(100, 0).await.unwrap().is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove collection persistence directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn share_group_delete_precedes_queued_member_add_without_blocking_reads() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create share-group persistence database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());
    let created = crate::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Trusted peers"}"#,
        &state,
    )
    .await
    .expect("create share group");
    assert_eq!(created.status, "201 Created");
    let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("share-group id")
        .to_owned();
    let member_path = format!("/api/sharegroups/{group_id}/members");
    let member = crate::route_http_request(
        "POST",
        &member_path,
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("add initial share-group member");
    assert_eq!(member.status, "201 Created");

    let persistence_turn = state.share_group_persistence_lock.lock().await;
    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/sharegroups/{group_id}");
    let delete_task_path = delete_path.clone();
    let mut delete = tokio::spawn(async move {
        crate::route_http_request("DELETE", &delete_task_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let add_state = Arc::clone(&state);
    let add_path = member_path.clone();
    let mut add = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            &add_path,
            None,
            r#"{"username":"late"}"#,
            &add_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut add)
        .await
        .is_err());

    let readable = crate::route_http_request("GET", &delete_path, None, "", &state)
        .await
        .expect("read share group while mutations wait for persistence turn");
    assert_eq!(readable.status, "200 OK");
    let readable_members = crate::route_http_request("GET", &member_path, None, "", &state)
        .await
        .expect("read share-group members while mutations wait");
    assert_eq!(readable_members.status, "200 OK");
    assert!(readable_members.body.contains("friend"));
    assert_eq!(db.list_share_groups(10, 0).await.unwrap().len(), 1);

    drop(persistence_turn);
    let deleted = tokio::time::timeout(Duration::from_secs(1), &mut delete)
        .await
        .expect("delete completes after persistence turn is released")
        .expect("join delete route")
        .expect("delete response");
    assert_eq!(deleted.status, "200 OK");
    let added = tokio::time::timeout(Duration::from_secs(1), &mut add)
        .await
        .expect("queued member request completes after delete")
        .expect("join member route")
        .expect("member response");
    assert_eq!(added.status, "404 Not Found");
    assert!(db.list_share_groups(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_group_members(10, 0).await.unwrap().is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn collection_delete_precedes_queued_share_token_creation() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-collection-token-order-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create collection token-order directory");
    let db_path = root.join("slskr.db");
    let db_path_string = db_path.to_str().expect("UTF-8 database path");
    let db = crate::persistence::DatabaseManager::new(db_path_string)
        .await
        .expect("create collection token-order database");
    let (state, _receiver) = test_state_with_db(MapEnv::default(), db.clone());

    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Token order"}"#,
        &state,
    )
    .await
    .expect("create collection for token-order regression");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .expect("collection id")
        .to_owned();
    let grant = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        None,
        &serde_json::json!({
            "collection_id": collection_id,
            "username": "recipient",
            "permissions": "read,stream"
        })
        .to_string(),
        &state,
    )
    .await
    .expect("create grant for token-order regression");
    assert_eq!(grant.status, "201 Created");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap()["id"]
        .as_str()
        .expect("grant id")
        .to_owned();

    let persistence_turn = state.collection_grant_persistence_lock.lock().await;
    let delete_path = format!("/api/v0/collections/{collection_id}");
    let delete = crate::route_http_request("DELETE", &delete_path, None, "", &state);
    tokio::pin!(delete);
    tokio::select! {
        biased;
        _ = &mut delete => panic!("collection delete must wait for persistence turn"),
        _ = tokio::task::yield_now() => {}
    }
    let token_path = format!("/api/v0/share-grants/{grant_id}/token");
    let create_token = crate::route_http_request("POST", &token_path, None, "", &state);
    tokio::pin!(create_token);
    tokio::select! {
        biased;
        _ = &mut create_token => panic!("share-token creation must wait behind collection delete"),
        _ = tokio::task::yield_now() => {}
    }

    drop(persistence_turn);
    let (deleted, token) = tokio::join!(&mut delete, &mut create_token);
    let deleted = deleted.expect("delete collection before queued token request");
    let token = token.expect("finish queued token request");
    assert_eq!(deleted.status, "204 No Content");
    assert_eq!(token.status, "404 Not Found");
    assert!(state.share_access_tokens.read().await.records.is_empty());
    assert!(db
        .list_share_access_tokens(0, 100, 0)
        .await
        .expect("list persisted tokens after collection delete")
        .is_empty());

    db.close_for_test().await;
    fs::remove_dir_all(root).expect("remove collection token-order directory");
    let _ = fs::remove_dir_all(&state.config.state_dir);
}
