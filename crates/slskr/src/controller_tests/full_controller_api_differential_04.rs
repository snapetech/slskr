//! Controller full controller api differential 04 ownership.

use super::*;

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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_controller_rooms_and_conversations() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    state.session.write().await.state = "connected";
    state.rooms.write().await.records.clear();
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let joined =
        crate::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
            .await
            .expect("slskd room join");
    let joined_json = serde_json::from_str::<serde_json::Value>(&joined.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "nominal-status-headers-body",
        joined.status == "201 Created"
            && joined_json["name"] == "music"
            && joined_json["users"].is_array()
            && joined_json["messages"].is_array()
    );
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "mutation-side-effects-and-readback",
        joined.status == "201 Created"
            && state
                .rooms
                .read()
                .await
                .records
                .iter()
                .any(|room| room.name == "music")
    );

    let repeated =
        crate::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
            .await
            .expect("slskd repeated room join");
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "concurrency-and-idempotency",
        repeated.status == "200 OK" && repeated.body.is_empty()
    );

    let joined_rooms = crate::route_http_request("GET", "/api/v0/rooms/joined", None, "", &state)
        .await
        .expect("slskd joined rooms");
    let joined_rooms_json =
        serde_json::from_str::<serde_json::Value>(&joined_rooms.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/joined",
        "nominal-status-headers-body",
        joined_rooms.status == "200 OK" && joined_rooms_json.is_array()
    );
    record!(
        "GET",
        "/api/v0/rooms/joined",
        "populated-dynamic-state",
        joined_rooms_json
            .as_array()
            .is_some_and(|rooms| rooms.iter().any(|room| room == "music"))
    );

    let room = crate::route_http_request("GET", "/api/v0/rooms/joined/music", None, "", &state)
        .await
        .expect("slskd room detail");
    let room_json = serde_json::from_str::<serde_json::Value>(&room.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        "nominal-status-headers-body",
        room.status == "200 OK"
            && room_json["name"] == "music"
            && room_json["users"].is_array()
            && room_json["messages"].is_array()
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        "populated-dynamic-state",
        room.status == "200 OK" && room_json["name"] == "music"
    );

    for (path, route) in [
        (
            "/api/v0/rooms/joined/music/users",
            "/api/v0/rooms/joined/{roomName}/users",
        ),
        (
            "/api/v0/rooms/joined/music/messages",
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && json.is_array()
        );
    }

    let room_message = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined/music/messages",
        None,
        r#""hello room""#,
        &state,
    )
    .await
    .expect("slskd room message");
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "nominal-status-headers-body",
        room_message.status == "201 Created" && room_message.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "mutation-side-effects-and-readback",
        room_message.status == "201 Created"
            && state.rooms.read().await.records[0]
                .messages
                .iter()
                .any(|message| { message.body == "hello room" })
    );
    let room_messages = crate::route_http_request(
        "GET",
        "/api/v0/rooms/joined/music/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated room messages");
    let room_messages_json =
        serde_json::from_str::<serde_json::Value>(&room_messages.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        "populated-dynamic-state",
        room_messages.status == "200 OK" && room_messages_json[0]["message"] == "hello room"
    );

    for (path, route) in [
        (
            "/api/v0/rooms/joined/music/ticker",
            "/api/v0/rooms/joined/{roomName}/ticker",
        ),
        (
            "/api/v0/rooms/joined/music/members",
            "/api/v0/rooms/joined/{roomName}/members",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, r#""value""#, &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "201 Created" && response.body.is_empty()
        );
        record!(
            "POST",
            route,
            "mutation-side-effects-and-readback",
            response.status == "201 Created"
        );
    }

    let leave = crate::route_http_request("DELETE", "/api/v0/rooms/joined/music", None, "", &state)
        .await
        .expect("slskd room leave");
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "nominal-status-headers-body",
        leave.status == "204 No Content" && leave.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "mutation-side-effects-and-readback",
        leave.status == "204 No Content"
            && state
                .rooms
                .read()
                .await
                .records
                .iter()
                .all(|room| !room.joined)
    );
    let missing_room =
        crate::route_http_request("GET", "/api/v0/rooms/joined/music", None, "", &state)
            .await
            .expect("slskd missing room");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        "missing-empty-or-conflict-state",
        missing_room.status == "404 Not Found"
    );

    let sent = crate::route_http_request(
        "POST",
        "/api/v0/conversations/peer",
        None,
        r#""hello peer""#,
        &state,
    )
    .await
    .expect("slskd conversation send");
    record!(
        "POST",
        "/api/v0/conversations/{username}",
        "nominal-status-headers-body",
        sent.status == "201 Created" && sent.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/conversations/{username}",
        "mutation-side-effects-and-readback",
        sent.status == "201 Created"
            && state
                .messages
                .read()
                .await
                .records
                .iter()
                .any(|message| { message.username == "peer" && message.body == "hello peer" })
    );

    let conversations = crate::route_http_request("GET", "/api/v0/conversations", None, "", &state)
        .await
        .expect("slskd conversations");
    let conversations_json =
        serde_json::from_str::<serde_json::Value>(&conversations.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/conversations",
        "nominal-status-headers-body",
        conversations.status == "200 OK" && conversations_json.is_array()
    );
    record!(
        "GET",
        "/api/v0/conversations",
        "populated-dynamic-state",
        conversations_json.as_array().is_some_and(|rows| {
            rows.iter().any(|row| {
                row["username"] == "peer"
                    && row["messages"].as_array().is_some_and(|messages| {
                        messages
                            .iter()
                            .any(|message| message["message"] == "hello peer")
                    })
            })
        })
    );

    let conversation =
        crate::route_http_request("GET", "/api/v0/conversations/peer", None, "", &state)
            .await
            .expect("slskd conversation detail");
    let conversation_json =
        serde_json::from_str::<serde_json::Value>(&conversation.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/conversations/{username}",
        "nominal-status-headers-body",
        conversation.status == "200 OK" && conversation_json["username"] == "peer"
    );
    record!(
        "GET",
        "/api/v0/conversations/{username}",
        "populated-dynamic-state",
        conversation.status == "200 OK"
            && conversation_json["messages"]
                .as_array()
                .is_some_and(|messages| {
                    messages
                        .iter()
                        .any(|message| message["message"] == "hello peer")
                })
    );

    let messages = crate::route_http_request(
        "GET",
        "/api/v0/conversations/peer/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd conversation messages");
    let messages_json =
        serde_json::from_str::<serde_json::Value>(&messages.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/conversations/{username}/messages",
        "nominal-status-headers-body",
        messages.status == "200 OK" && messages_json.is_array()
    );
    record!(
        "GET",
        "/api/v0/conversations/{username}/messages",
        "populated-dynamic-state",
        messages_json.as_array().is_some_and(|rows| {
            rows.iter()
                .any(|message| message["message"] == "hello peer")
        })
    );

    let message_id = state
        .messages
        .read()
        .await
        .records
        .iter()
        .find(|message| message.username == "peer")
        .map(|message| message.id)
        .expect("conversation message id");
    let acknowledge = crate::route_http_request(
        "PUT",
        &format!("/api/v0/conversations/peer/{message_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd acknowledge message");
    record!(
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "nominal-status-headers-body",
        acknowledge.status == "200 OK" && acknowledge.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "mutation-side-effects-and-readback",
        acknowledge.status == "200 OK"
            && state
                .messages
                .read()
                .await
                .records
                .iter()
                .find(|message| message.id == message_id)
                .is_some_and(|message| message.acknowledged)
    );

    let acknowledge_all =
        crate::route_http_request("PUT", "/api/v0/conversations/peer", None, "", &state)
            .await
            .expect("slskd acknowledge conversation");
    record!(
        "PUT",
        "/api/v0/conversations/{username}",
        "nominal-status-headers-body",
        acknowledge_all.status == "200 OK" && acknowledge_all.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/conversations/{username}",
        "mutation-side-effects-and-readback",
        acknowledge_all.status == "200 OK"
    );

    let malformed_send =
        crate::route_http_request("POST", "/api/v0/conversations/peer", None, "", &state)
            .await
            .expect("slskd malformed conversation");
    record!(
        "POST",
        "/api/v0/conversations/{username}",
        "malformed-path-query-or-body",
        malformed_send.status == "400 Bad Request"
    );

    let deleted =
        crate::route_http_request("DELETE", "/api/v0/conversations/peer", None, "", &state)
            .await
            .expect("slskd conversation delete");
    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "nominal-status-headers-body",
        deleted.status == "204 No Content" && deleted.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "mutation-side-effects-and-readback",
        deleted.status == "204 No Content"
            && state
                .messages
                .read()
                .await
                .records
                .iter()
                .all(|message| { message.username != "peer" })
    );
    let missing_conversation =
        crate::route_http_request("GET", "/api/v0/conversations/peer", None, "", &state)
            .await
            .expect("slskd missing conversation");
    record!(
        "GET",
        "/api/v0/conversations/{username}",
        "missing-empty-or-conflict-state",
        missing_conversation.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_rooms_conversations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd rooms/conversations ledger"),
    )
    .expect("write slskd rooms/conversations ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd rooms/conversations controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_controller_rooms_conversations_restart_and_failure()
{
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("rooms/conversations database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
    state.session.write().await.state = "connected";

    let joined = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined",
        None,
        r#""persist-room""#,
        &state,
    )
    .await
    .expect("persist room join");
    let persisted_rooms = db
        .list_subscribed_rooms()
        .await
        .expect("list persisted rooms");
    let rehydrated_rooms = crate::RoomStore::from_persisted(persisted_rooms);
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "restart-persistence-or-reset",
        joined.status == "201 Created"
            && rehydrated_rooms
                .records
                .iter()
                .any(|room| room.name == "persist-room" && room.joined)
    );
    let _ = receiver.try_recv();
    let repeated_join = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined",
        None,
        r#""persist-room""#,
        &state,
    )
    .await
    .expect("repeat room join");
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "concurrency-and-idempotency",
        repeated_join.status == "200 OK" && repeated_join.body.is_empty()
    );

    let left = crate::route_http_request(
        "DELETE",
        "/api/v0/rooms/joined/persist-room",
        None,
        "",
        &state,
    )
    .await
    .expect("persist room leave");
    let rooms_after_leave = db
        .list_subscribed_rooms()
        .await
        .expect("list rooms after leave");
    let rehydrated_after_leave = crate::RoomStore::from_persisted(rooms_after_leave);
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "restart-persistence-or-reset",
        left.status == "204 No Content"
            && rehydrated_after_leave
                .records
                .iter()
                .all(|room| room.name != "persist-room" || !room.joined)
    );
    let repeated_leave = crate::route_http_request(
        "DELETE",
        "/api/v0/rooms/joined/persist-room",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat room leave");
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "missing-empty-or-conflict-state",
        repeated_leave.status == "204 No Content" && repeated_leave.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "concurrency-and-idempotency",
        left.status == "204 No Content" && repeated_leave.status == "204 No Content"
    );

    let malformed_join =
        crate::route_http_request("POST", "/api/v0/rooms/joined", None, "{}", &state)
            .await
            .expect("malformed room join");
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "malformed-path-query-or-body",
        malformed_join.status == "400 Bad Request"
    );
    let empty_join = crate::route_http_request("POST", "/api/v0/rooms/joined", None, "{}", &state)
        .await
        .expect("empty room join");
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "missing-empty-or-conflict-state",
        empty_join.status == "400 Bad Request"
    );

    // Room message, ticker, and private-member writes are runtime
    // commands to the Soulseek client.  Their tracker projections are
    // deliberately reset on a fresh rehydrate; only the room
    // subscription itself is durable, matching the frozen controller's
    // in-memory IRoomTracker lifecycle.
    while receiver.try_recv().is_ok() {}
    let action_join = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined",
        None,
        r#""room-actions""#,
        &state,
    )
    .await
    .expect("join room for subresource actions");
    let action_join_command = receiver.try_recv().ok();
    let action_message = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined/room-actions/messages",
        None,
        r#""message before reset""#,
        &state,
    )
    .await
    .expect("room message before reset");
    let action_message_command = receiver.try_recv().ok();
    let action_ticker = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined/room-actions/ticker",
        None,
        r#""ticker before reset""#,
        &state,
    )
    .await
    .expect("room ticker before reset");
    let action_ticker_command = receiver.try_recv().ok();
    let action_member = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined/room-actions/members",
        None,
        r#""member before reset""#,
        &state,
    )
    .await
    .expect("room member before reset");
    let action_member_command = receiver.try_recv().ok();
    let rehydrated_action_rooms = crate::RoomStore::from_persisted(
        db.list_subscribed_rooms()
            .await
            .expect("list room subresource subscriptions"),
    );
    let rehydrated_action_room = rehydrated_action_rooms
        .records
        .iter()
        .find(|room| room.name == "room-actions");
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "restart-persistence-or-reset",
        action_join.status == "201 Created"
            && action_join_command
                == Some(crate::SessionCommand::JoinRoom("room-actions".to_owned()))
            && action_message.status == "201 Created"
            && action_message_command
                == Some(crate::SessionCommand::SayRoom {
                    room: "room-actions".to_owned(),
                    body: "message before reset".to_owned(),
                })
            && rehydrated_action_room.is_some_and(|room| {
                room.joined && room.messages.is_empty() && room.ticker.is_none()
            })
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/ticker",
        "restart-persistence-or-reset",
        action_ticker.status == "201 Created"
            && action_ticker_command
                == Some(crate::SessionCommand::SetRoomTicker {
                    room: "room-actions".to_owned(),
                    ticker: "ticker before reset".to_owned(),
                })
            && rehydrated_action_room.is_some_and(|room| {
                room.joined && room.ticker.is_none() && room.members.is_empty()
            })
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/members",
        "restart-persistence-or-reset",
        action_member.status == "201 Created"
            && action_member_command
                == Some(crate::SessionCommand::AddRoomMember {
                    room: "room-actions".to_owned(),
                    username: "member before reset".to_owned(),
                })
            && rehydrated_action_room.is_some_and(|room| room.joined && room.members.is_empty())
    );

    let (failure_state, failure_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    failure_state
        .rooms
        .write()
        .await
        .join("room-runtime-failure".to_owned())
        .expect("runtime failure room");
    let before_room_failure = failure_state.rooms.read().await.clone();
    drop(failure_receiver);
    for (path, body, route) in [
        (
            "/api/v0/rooms/joined/room-runtime-failure/messages",
            r#""message""#,
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
        (
            "/api/v0/rooms/joined/room-runtime-failure/ticker",
            r#""ticker""#,
            "/api/v0/rooms/joined/{roomName}/ticker",
        ),
        (
            "/api/v0/rooms/joined/room-runtime-failure/members",
            r#""member""#,
            "/api/v0/rooms/joined/{roomName}/members",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, body, &failure_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            route,
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && failure_state.rooms.read().await.clone() == before_room_failure
        );
    }

    let sent = crate::route_http_request(
        "POST",
        "/api/v0/conversations/persist-peer",
        None,
        r#""persisted conversation""#,
        &state,
    )
    .await
    .expect("persist conversation message");
    let message_id = state
        .messages
        .read()
        .await
        .records
        .iter()
        .find(|message| message.username == "persist-peer")
        .map(|message| message.id)
        .expect("persisted message id");
    let persisted_messages = db.list_messages(20, 0).await.expect("list messages");
    let rehydrated_messages = crate::MessageStore::from_persisted(persisted_messages);
    record!(
        "POST",
        "/api/v0/conversations/{username}",
        "restart-persistence-or-reset",
        sent.status == "201 Created"
            && rehydrated_messages.records.iter().any(|message| {
                message.username == "persist-peer" && message.body == "persisted conversation"
            })
    );
    let _ = receiver.try_recv();

    let concurrent_messages = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/conversations/concurrent-a",
            None,
            r#""parallel-a""#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/conversations/concurrent-b",
            None,
            r#""parallel-b""#,
            &state,
        ),
    ])
    .await;
    let persisted_concurrent = db
        .list_messages(20, 0)
        .await
        .expect("list concurrent messages");
    record!(
        "POST",
        "/api/v0/conversations/{username}",
        "concurrency-and-idempotency",
        concurrent_messages.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && persisted_concurrent.iter().any(|message| {
                message.username == "concurrent-a" && message.content == "parallel-a"
            })
            && persisted_concurrent.iter().any(|message| {
                message.username == "concurrent-b" && message.content == "parallel-b"
            })
    );

    let ack = crate::route_http_request(
        "PUT",
        &format!("/api/v0/conversations/persist-peer/{message_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("ack persisted conversation");
    let persisted_after_ack = db.list_messages(20, 0).await.expect("list acked messages");
    let rehydrated_after_ack = crate::MessageStore::from_persisted(persisted_after_ack.clone());
    record!(
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "restart-persistence-or-reset",
        ack.status == "200 OK"
            && persisted_after_ack
                .iter()
                .any(|message| message.id == message_id.to_string() && message.read)
            && rehydrated_after_ack
                .records
                .iter()
                .any(|message| message.id == message_id && message.acknowledged)
    );
    let repeated_ack = crate::route_http_request(
        "PUT",
        &format!("/api/v0/conversations/persist-peer/{message_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat conversation ack");
    record!(
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "concurrency-and-idempotency",
        repeated_ack.status == "200 OK"
    );

    let second_message = crate::route_http_request(
        "POST",
        "/api/v0/conversations/persist-peer",
        None,
        r#""ack all""#,
        &state,
    )
    .await
    .expect("create second conversation message");
    assert_eq!(second_message.status, "201 Created");
    let ack_all = crate::route_http_request(
        "PUT",
        "/api/v0/conversations/persist-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("ack all conversation messages");
    let persisted_after_ack_all = db
        .list_messages(20, 0)
        .await
        .expect("list all acked messages");
    record!(
        "PUT",
        "/api/v0/conversations/{username}",
        "restart-persistence-or-reset",
        ack_all.status == "200 OK"
            && persisted_after_ack_all
                .iter()
                .filter(|message| message.username == "persist-peer")
                .all(|message| message.read)
    );
    let repeated_ack_all = crate::route_http_request(
        "PUT",
        "/api/v0/conversations/persist-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat ack all conversation messages");
    record!(
        "PUT",
        "/api/v0/conversations/{username}",
        "concurrency-and-idempotency",
        repeated_ack_all.status == "200 OK"
    );

    let deleted = crate::route_http_request(
        "DELETE",
        "/api/v0/conversations/persist-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("delete persisted conversation");
    let persisted_after_delete = db
        .list_messages(20, 0)
        .await
        .expect("list after conversation delete");
    let rehydrated_after_delete =
        crate::MessageStore::from_persisted(persisted_after_delete.clone());
    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "restart-persistence-or-reset",
        deleted.status == "204 No Content"
            && persisted_after_delete
                .iter()
                .all(|message| message.username != "persist-peer")
            && rehydrated_after_delete
                .records
                .iter()
                .all(|message| message.username != "persist-peer")
    );
    let repeated_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/conversations/persist-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat conversation delete");
    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "missing-empty-or-conflict-state",
        repeated_delete.status == "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "concurrency-and-idempotency",
        deleted.status == "204 No Content" && repeated_delete.status == "404 Not Found"
    );

    let malformed_conversation = crate::route_http_request(
        "GET",
        "/api/v0/conversations/concurrent-a?since=-1",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed conversation timestamp");
    record!(
        "GET",
        "/api/v0/conversations/{username}",
        "malformed-path-query-or-body",
        malformed_conversation.status == "400 Bad Request"
    );
    let malformed_messages = crate::route_http_request(
        "GET",
        "/api/v0/conversations/concurrent-a/messages?unAcknowledgedOnly=maybe",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed conversation messages timestamp");
    record!(
        "GET",
        "/api/v0/conversations/{username}/messages",
        "malformed-path-query-or-body",
        malformed_messages.status == "400 Bad Request"
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("room join failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_db.close_for_test().await;
    let failed_join = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined",
        None,
        r#""failed-room""#,
        &failure_state,
    )
    .await
    .expect("room join persistence failure response");
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "runtime-failure-and-timeout",
        failed_join.status == "503 Service Unavailable"
            && failure_state
                .rooms
                .read()
                .await
                .records
                .iter()
                .all(|room| room.name != "failed-room")
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("room leave failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state
        .rooms
        .write()
        .await
        .join("failed-leave".to_owned())
        .expect("seed room leave");
    let previous_rooms = failure_state.rooms.read().await.clone();
    failure_db.close_for_test().await;
    let failed_leave = crate::route_http_request(
        "DELETE",
        "/api/v0/rooms/joined/failed-leave",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("room leave persistence failure response");
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "runtime-failure-and-timeout",
        failed_leave.status == "503 Service Unavailable"
            && *failure_state.rooms.read().await == previous_rooms
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation send failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_db.close_for_test().await;
    let failed_send = crate::route_http_request(
        "POST",
        "/api/v0/conversations/failure-peer",
        None,
        r#""failed send""#,
        &failure_state,
    )
    .await
    .expect("conversation send persistence failure response");
    record!(
        "POST",
        "/api/v0/conversations/{username}",
        "runtime-failure-and-timeout",
        failed_send.status == "503 Service Unavailable"
            && failure_state.messages.read().await.records.is_empty()
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation acknowledgement failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    let seeded_id = failure_state
        .messages
        .write()
        .await
        .add(
            "failure-peer".to_owned(),
            "inbound",
            "unacknowledged".to_owned(),
        )
        .id;
    failure_db.close_for_test().await;
    let failed_ack = crate::route_http_request(
        "PUT",
        &format!("/api/v0/conversations/failure-peer/{seeded_id}"),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("conversation ack persistence failure response");
    record!(
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "runtime-failure-and-timeout",
        failed_ack.status == "503 Service Unavailable"
            && failure_state
                .messages
                .read()
                .await
                .records
                .iter()
                .any(|message| message.id == seeded_id && !message.acknowledged)
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation acknowledge-all failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_state.messages.write().await.add(
        "failure-peer".to_owned(),
        "inbound",
        "unacknowledged".to_owned(),
    );
    failure_db.close_for_test().await;
    let failed_ack_all = crate::route_http_request(
        "PUT",
        "/api/v0/conversations/failure-peer",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("conversation acknowledge-all failure response");
    record!(
        "PUT",
        "/api/v0/conversations/{username}",
        "runtime-failure-and-timeout",
        failed_ack_all.status == "503 Service Unavailable"
            && failure_state
                .messages
                .read()
                .await
                .records
                .iter()
                .all(|message| !message.acknowledged)
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation delete failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(failure_db.clone()));
    failure_state.messages.write().await.add(
        "failure-peer".to_owned(),
        "inbound",
        "retained".to_owned(),
    );
    let previous_messages = failure_state.messages.read().await.clone();
    failure_db.close_for_test().await;
    let failed_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/conversations/failure-peer",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("conversation delete persistence failure response");
    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "runtime-failure-and-timeout",
        failed_delete.status == "503 Service Unavailable"
            && *failure_state.messages.read().await == previous_messages
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_rooms_conversations_restart_and_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd rooms/conversations ledger"),
    )
    .expect("write slskd rooms/conversations ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd rooms/conversations restart/failure mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_controller_server_state_and_lifecycle() {
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let initial = crate::route_http_request("GET", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd initial server state");
    let initial_json = serde_json::from_str::<serde_json::Value>(&initial.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/server",
        "nominal-status-headers-body",
        initial.status == "200 OK"
            && initial_json.as_object().is_some_and(|object| {
                object.len() == 4
                    && object["state"] == "Disconnected"
                    && object["isConnected"] == false
                    && object["isLoggedIn"] == false
                    && object["isTransitioning"] == false
            })
    );

    {
        let mut session = state.session.write().await;
        session.state = "connected";
        session.username = Some("tester".to_owned());
    }
    *state
        .connected_server_address
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some("127.0.0.1:2242".to_owned());
    let connected = crate::route_http_request("GET", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd connected server state");
    let connected_json =
        serde_json::from_str::<serde_json::Value>(&connected.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/server",
        "populated-dynamic-state",
        connected.status == "200 OK"
            && connected_json["state"] == "Connected, LoggedIn"
            && connected_json["isConnected"] == true
            && connected_json["isLoggedIn"] == true
            && connected_json["isTransitioning"] == false
            && connected_json["address"] == "127.0.0.1"
            && connected_json["ipEndPoint"] == "127.0.0.1:2242"
    );

    {
        let mut session = state.session.write().await;
        session.state = "disconnected";
        session.username = None;
    }
    let connect = crate::route_http_request("PUT", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "nominal-status-headers-body",
        connect.status == "200 OK" && connect.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/server",
        "mutation-side-effects-and-readback",
        connect.status == "200 OK"
            && state.session.read().await.state == "connecting"
            && matches!(receiver.try_recv(), Ok(crate::SessionCommand::Connect))
    );

    let repeated_connect = crate::route_http_request("PUT", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd repeated server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "concurrency-and-idempotency",
        repeated_connect.status == "205 Reset Content" && repeated_connect.body.is_empty()
    );

    let disconnect = crate::route_http_request("DELETE", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "nominal-status-headers-body",
        disconnect.status == "204 No Content" && disconnect.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/server",
        "mutation-side-effects-and-readback",
        disconnect.status == "204 No Content"
            && state.session.read().await.state == "disconnecting"
            && matches!(receiver.try_recv(), Ok(crate::SessionCommand::Disconnect))
    );

    let repeated_disconnect =
        crate::route_http_request("DELETE", "/api/v0/server", None, "", &state)
            .await
            .expect("slskd repeated server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "concurrency-and-idempotency",
        repeated_disconnect.status == "204 No Content" && repeated_disconnect.body.is_empty()
    );

    let malformed_disconnect =
        crate::route_http_request("DELETE", "/api/v0/server", None, "{", &state)
            .await
            .expect("slskd malformed server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "malformed-path-query-or-body",
        malformed_disconnect.status == "400 Bad Request"
    );

    let (restarted_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let reset_disconnect =
        crate::route_http_request("DELETE", "/api/v0/server", None, "", &restarted_state)
            .await
            .expect("slskd server disconnect after restart");
    record!(
        "DELETE",
        "/api/v0/server",
        "restart-persistence-or-reset",
        reset_disconnect.status == "204 No Content" && reset_disconnect.body.is_empty()
    );

    let (put_restart_state, mut put_restart_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let reset_connect =
        crate::route_http_request("PUT", "/api/v0/server", None, "", &put_restart_state)
            .await
            .expect("slskd server connect after restart");
    record!(
        "PUT",
        "/api/v0/server",
        "restart-persistence-or-reset",
        reset_connect.status == "200 OK"
            && reset_connect.body.is_empty()
            && put_restart_state.session.read().await.state == "connecting"
            && matches!(
                put_restart_receiver.try_recv(),
                Ok(crate::SessionCommand::Connect)
            )
    );

    let (put_failure_state, put_failure_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    drop(put_failure_receiver);
    let put_failure =
        crate::route_http_request("PUT", "/api/v0/server", None, "", &put_failure_state)
            .await
            .expect("slskd server connect failure");
    record!(
        "PUT",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        put_failure.status == "503 Service Unavailable"
            && put_failure_state.session.read().await.state == "disconnected"
    );

    let (delete_failure_state, delete_failure_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    delete_failure_state.session.write().await.state = "connected";
    drop(delete_failure_receiver);
    let delete_failure =
        crate::route_http_request("DELETE", "/api/v0/server", None, "", &delete_failure_state)
            .await
            .expect("slskd server disconnect failure");
    record!(
        "DELETE",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        delete_failure.status == "503 Service Unavailable"
            && delete_failure_state.session.read().await.state == "connected"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_server_state_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd server state ledger"),
    )
    .expect("write slskd server state ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd server controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_controller_search_lifecycle() {
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    state.session.write().await.state = "connected";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    state.shares.write().await.entries.push(FileEntry {
        filename_encoding: Default::default(),
        extension_encoding: Default::default(),
        code: 1,
        filename: "Remote/Search.flac".to_owned(),
        size: 321,
        extension: "flac".to_owned(),
        attributes: Vec::new(),
    });
    let search_id = "22222222-2222-4222-8222-222222222222";
    let create_body = format!(r#"{{"id":"{search_id}","searchText":"Remote Search"}}"#);
    let created = crate::route_http_request("POST", "/api/v0/searches", None, &create_body, &state)
        .await
        .expect("slskd search create");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/searches",
        "nominal-status-headers-body",
        created.status == "200 OK"
            && created_json["searchId"] == search_id.replace('-', "")
            && created_json["query"] == "Remote Search"
            && created_json["results"].is_array()
    );
    record!(
        "POST",
        "/api/v0/searches",
        "mutation-side-effects-and-readback",
        state
            .searches
            .read()
            .await
            .get_by_identifier(search_id)
            .is_some_and(|record| record.query == "Remote Search")
            && matches!(
                receiver.try_recv(),
                Ok(crate::SessionCommand::Search { .. })
            )
    );

    let duplicate =
        crate::route_http_request("POST", "/api/v0/searches", None, &create_body, &state)
            .await
            .expect("duplicate slskd search");
    record!(
        "POST",
        "/api/v0/searches",
        "missing-empty-or-conflict-state",
        duplicate.status == "409 Conflict"
    );

    let malformed = crate::route_http_request("POST", "/api/v0/searches", None, "{}", &state)
        .await
        .expect("malformed slskd search");
    record!(
        "POST",
        "/api/v0/searches",
        "malformed-path-query-or-body",
        malformed.status == "400 Bad Request"
    );

    let token = state
        .searches
        .read()
        .await
        .get_by_identifier(search_id)
        .map(|record| record.token)
        .expect("slskd search token");
    let response_ingest = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        &format!(
            r#"{{"token":{token},"peer_username":"peer","filename":"Remote/Peer.flac","size":99}}"#
        ),
        &state,
    )
    .await
    .expect("slskd search response ingest");
    assert_eq!(response_ingest.status, "200 OK");

    let listed = crate::route_http_request("GET", "/api/v0/searches", None, "", &state)
        .await
        .expect("slskd search list");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/searches",
        "nominal-status-headers-body",
        listed.status == "200 OK" && listed_json.is_array()
    );
    record!(
        "GET",
        "/api/v0/searches",
        "populated-dynamic-state",
        listed.status == "200 OK"
            && listed_json.as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["id"] == search_id
                        && row["results"]
                            .as_array()
                            .is_some_and(|results| !results.is_empty())
                })
            })
    );

    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{search_id}?includeResponses=true"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd search detail");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "nominal-status-headers-body",
        detail.status == "200 OK" && detail_json["id"] == search_id
    );
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "populated-dynamic-state",
        detail.status == "200 OK"
            && detail_json["results"]
                .as_array()
                .is_some_and(|results| !results.is_empty())
    );

    let responses = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{search_id}/responses"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd search responses");
    let responses_json =
        serde_json::from_str::<serde_json::Value>(&responses.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "nominal-status-headers-body",
        responses.status == "200 OK" && responses_json.is_array()
    );
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "populated-dynamic-state",
        responses.status == "200 OK"
            && responses_json.as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["username"] == "peer"
                        && row["files"]
                            .as_array()
                            .is_some_and(|files| !files.is_empty())
                })
            })
    );

    let cancelled = crate::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{search_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd search cancel");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "nominal-status-headers-body",
        cancelled.status == "200 OK" && cancelled.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "mutation-side-effects-and-readback",
        cancelled.status == "200 OK"
            && state
                .searches
                .read()
                .await
                .get_by_identifier(search_id)
                .is_some_and(|record| record.status == "cancelled")
    );

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{search_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd search delete");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "nominal-status-headers-body",
        deleted.status == "204 No Content" && deleted.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "mutation-side-effects-and-readback",
        deleted.status == "204 No Content"
            && state
                .searches
                .read()
                .await
                .get_by_identifier(search_id)
                .is_none()
    );
    let missing = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{search_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd search");
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_search_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd search ledger"),
    )
    .expect("write slskd search ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd search controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_controller_search_failure_restart_and_idempotency()
{
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    async fn live_search_get(state: Arc<crate::AppState>, path: &str) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(crate::handle_http_stream(server, None, false, state));
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        client
            .write_all(request.as_bytes())
            .await
            .expect("write search failure request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read search failure response");
        task.await
            .expect("search failure HTTP task")
            .expect("search failure HTTP response");
        response
    }

    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search lifecycle database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
    state.session.write().await.state = "connected";

    let created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"restart search","target":"global"}"#,
        &state,
    )
    .await
    .expect("create restart search");
    let created_json =
        serde_json::from_str::<serde_json::Value>(&created.body).expect("search response json");
    let search_id = created_json["searchId"]
        .as_str()
        .expect("search id")
        .to_owned();
    let _ = receiver.try_recv();
    let persisted = db
        .list_searches(10, 0)
        .await
        .expect("list persisted searches");
    let rehydrated = crate::SearchStore::from_persisted(persisted.clone());
    record!(
        "POST",
        "/api/v0/searches",
        "restart-persistence-or-reset",
        created.status == "200 OK"
            && persisted.len() == 1
            && persisted[0].query == "restart search"
            && rehydrated.get_by_identifier(&search_id).is_some()
    );

    let duplicate_body = r#"{"id":"44444444-4444-4444-8444-444444444444","query":"idempotent"}"#;
    let first_duplicate =
        crate::route_http_request("POST", "/api/v0/searches", None, duplicate_body, &state)
            .await
            .expect("first idempotent search");
    let second_duplicate =
        crate::route_http_request("POST", "/api/v0/searches", None, duplicate_body, &state)
            .await
            .expect("duplicate idempotent search");
    record!(
        "POST",
        "/api/v0/searches",
        "concurrency-and-idempotency",
        first_duplicate.status == "200 OK" && second_duplicate.status == "409 Conflict"
    );

    let delete_created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"delete restart search"}"#,
        &state,
    )
    .await
    .expect("create delete search");
    let delete_id = serde_json::from_str::<serde_json::Value>(&delete_created.body)
        .expect("delete search json")["searchId"]
        .as_str()
        .expect("delete search id")
        .to_owned();
    let _ = receiver.try_recv();
    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{delete_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete persisted search");
    let persisted_after_delete = db.list_searches(10, 0).await.expect("list after delete");
    let rehydrated_after_delete = crate::SearchStore::from_persisted(persisted_after_delete);
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "restart-persistence-or-reset",
        deleted.status == "204 No Content"
            && !state
                .searches
                .read()
                .await
                .records
                .iter()
                .any(|record| record.id == delete_id)
            && rehydrated_after_delete
                .get_by_identifier(&delete_id)
                .is_none()
    );
    let repeated_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{delete_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat delete search");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        repeated_delete.status == "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "concurrency-and-idempotency",
        deleted.status == "204 No Content" && repeated_delete.status == "404 Not Found"
    );

    let put_created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"put restart search"}"#,
        &state,
    )
    .await
    .expect("create put search");
    let put_id = serde_json::from_str::<serde_json::Value>(&put_created.body)
        .expect("put search json")["searchId"]
        .as_str()
        .expect("put search id")
        .to_owned();
    let _ = receiver.try_recv();
    let put = crate::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{put_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel persisted search");
    let persisted_put = db
        .get_search(&put_id)
        .await
        .expect("read cancelled search")
        .expect("cancelled search row");
    let rehydrated_put = crate::SearchStore::from_persisted(vec![persisted_put.clone()]);
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "restart-persistence-or-reset",
        put.status == "200 OK"
            && persisted_put.status == "cancelled"
            && rehydrated_put
                .get_by_identifier(&put_id)
                .is_some_and(|record| record.status == "cancelled")
    );
    let repeated_put = crate::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{put_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cancel search");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "concurrency-and-idempotency",
        repeated_put.status == "200 OK"
            && db
                .get_search(&put_id)
                .await
                .expect("read repeated cancellation")
                .is_some_and(|record| record.status == "cancelled")
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search creation failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_db.close_for_test().await;
    let failed_create = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"failed search"}"#,
        &failure_state,
    )
    .await
    .expect("search creation failure response");
    record!(
        "POST",
        "/api/v0/searches",
        "runtime-failure-and-timeout",
        failed_create.status == "503 Service Unavailable"
            && failure_state.searches.read().await.records.is_empty()
            && failure_state.searches.read().await.next_token == 1
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search update failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    {
        let mut searches = failure_state.searches.write().await;
        searches
            .create(
                None,
                "update failure".to_owned(),
                "global",
                None,
                Vec::new(),
                60,
            )
            .expect("seed update failure search");
    }
    failure_db.close_for_test().await;
    let failed_update =
        crate::route_http_request("PUT", "/api/v0/searches/1", None, "", &failure_state)
            .await
            .expect("search update failure response");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        failed_update.status == "503 Service Unavailable"
            && failure_state
                .searches
                .read()
                .await
                .get_by_identifier("1")
                .is_some_and(|record| record.status == "active")
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search delete failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    {
        let mut searches = failure_state.searches.write().await;
        searches
            .create(
                None,
                "delete failure".to_owned(),
                "global",
                None,
                Vec::new(),
                60,
            )
            .expect("seed delete failure search");
    }
    failure_db.close_for_test().await;
    let failed_delete =
        crate::route_http_request("DELETE", "/api/v0/searches/1", None, "", &failure_state)
            .await
            .expect("search delete failure response");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        failed_delete.status == "503 Service Unavailable"
            && failure_state
                .searches
                .read()
                .await
                .get_by_identifier("1")
                .is_some()
    );

    let expired_list_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search list expiry database");
    let (expired_list_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(expired_list_db.clone()),
    );
    {
        let mut searches = expired_list_state.searches.write().await;
        searches
            .create(
                None,
                "expired list".to_owned(),
                "global",
                None,
                Vec::new(),
                0,
            )
            .expect("seed expired list search");
    }
    expired_list_db.close_for_test().await;
    let live_failed_list =
        live_search_get(Arc::clone(&expired_list_state), "/api/v0/searches").await;
    record!(
        "GET",
        "/api/v0/searches",
        "runtime-failure-and-timeout",
        String::from_utf8_lossy(&live_failed_list)
            .starts_with("HTTP/1.1 500 Internal Server Error")
            && String::from_utf8_lossy(&live_failed_list).contains("failed to persist search")
    );

    let expired_detail_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search detail expiry database");
    let (expired_detail_state, _receiver) = test_state_with_env_parts(
        env,
        crate::SearchStore::new(),
        Some(expired_detail_db.clone()),
    );
    {
        let mut searches = expired_detail_state.searches.write().await;
        searches
            .create(
                None,
                "expired detail".to_owned(),
                "global",
                None,
                Vec::new(),
                0,
            )
            .expect("seed expired detail search");
    }
    expired_detail_db.close_for_test().await;
    let live_failed_detail =
        live_search_get(Arc::clone(&expired_detail_state), "/api/v0/searches/1").await;
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        String::from_utf8_lossy(&live_failed_detail)
            .starts_with("HTTP/1.1 500 Internal Server Error")
            && String::from_utf8_lossy(&live_failed_detail).contains("failed to persist search")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_search_failure_restart_and_idempotency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd search failure ledger"),
    )
    .expect("write slskd search failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd search failure/restart mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
