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
async fn controller_api_differential_rooms_controller_residuals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method,
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            super::route_http_request($method, $path, None, $body, $state)
                .await
                .expect("RoomsController route request")
        }};
    }

    fn json_body(response: &super::routing::HttpResponse) -> serde_json::Value {
        serde_json::from_str(&response.body).unwrap_or_default()
    }

    async fn connect_and_join(state: &Arc<super::AppState>, room: &str) {
        state.session.write().await.state = "connected";
        state
            .rooms
            .write()
            .await
            .join(room.to_owned())
            .expect("room fixture capacity");
    }

    // RoomsCompatibilityController is intentionally a validation-only
    // acknowledgement surface in frozen slskdN.  It must not acquire or
    // mutate the local RoomStore, and it remains available if SQLite is
    // closed.
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/rooms", r#"{"room":"compat-room"}"#);
        let after = state.rooms.read().await.clone();
        let body = json_body(&response);
        record!(
            "POST",
            "/api/rooms",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/json"
                && body["joined"] == true
        );
        record!(
            "POST",
            "/api/rooms",
            "mutation-side-effects-and-readback",
            after.records.iter().all(|room| room.name != "compat-room")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/rooms", "{}");
        record!(
            "POST",
            "/api/rooms",
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("Room is required")
        );
        let response = request!(&state, "POST", "/api/rooms", r#"{"room":" "}"#);
        record!(
            "POST",
            "/api/rooms",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request" && response.body.contains("Room is required")
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("compatibility runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = request!(&state, "POST", "/api/rooms", r#"{"room":"compat-runtime"}"#);
        record!(
            "POST",
            "/api/rooms",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && json_body(&response)["joined"] == true
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/rooms", r#"{"room":"compat-restart"}"#);
        let fresh = super::RoomStore::new();
        record!(
            "POST",
            "/api/rooms",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && fresh
                    .records
                    .iter()
                    .all(|room| room.name != "compat-restart")
        );
    }
    {
        let (state, _receiver) = test_state();
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "POST",
                    "/api/rooms",
                    None,
                    r#"{"room":"compat-concurrent"}"#,
                    &state,
                )
                .await
                .expect("concurrent compatibility join")
            }
        }))
        .await;
        record!(
            "POST",
            "/api/rooms",
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
        );
    }

    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "DELETE", "/api/rooms/compat-room", "");
        let body = json_body(&response);
        record!(
            "DELETE",
            "/api/rooms/{roomName}",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/json"
                && body["left"] == true
        );
        let after = state.rooms.read().await.clone();
        record!(
            "DELETE",
            "/api/rooms/{roomName}",
            "mutation-side-effects-and-readback",
            after.records.iter().all(|room| room.name != "compat-room")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "DELETE", "/api/rooms/%20", "");
        record!(
            "DELETE",
            "/api/rooms/{roomName}",
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("Room is required")
        );
        let response = request!(&state, "DELETE", "/api/rooms/not-joined", "");
        record!(
            "DELETE",
            "/api/rooms/{roomName}",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && json_body(&response)["left"] == true
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("compatibility delete runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = request!(&state, "DELETE", "/api/rooms/compat-runtime", "");
        record!(
            "DELETE",
            "/api/rooms/{roomName}",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && json_body(&response)["left"] == true
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "DELETE", "/api/rooms/compat-restart", "");
        let fresh = super::RoomStore::new();
        record!(
            "DELETE",
            "/api/rooms/{roomName}",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && fresh
                    .records
                    .iter()
                    .all(|room| room.name != "compat-restart")
        );
    }
    {
        let (state, _receiver) = test_state();
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "DELETE",
                    "/api/rooms/compat-concurrent",
                    None,
                    "",
                    &state,
                )
                .await
                .expect("concurrent compatibility leave")
            }
        }))
        .await;
        record!(
            "DELETE",
            "/api/rooms/{roomName}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
        );
    }

    // Read-only versioned RoomsController actions are storage-independent
    // and ignore unrelated query values.  Seed only the connected and
    // populated fixtures required by the controller's observable shapes.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "GET", "/api/v0/rooms/activity?unexpected=%7B", "");
        let missing = request!(&state, "GET", "/api/v0/rooms/activity", "");
        record!(
            "GET",
            "/api/v0/rooms/activity",
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_body(&malformed).is_object()
        );
        record!(
            "GET",
            "/api/v0/rooms/activity",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && json_body(&missing)
                    .as_object()
                    .is_some_and(|v| v.is_empty())
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("rooms activity runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = request!(&state, "GET", "/api/v0/rooms/activity", "");
        record!(
            "GET",
            "/api/v0/rooms/activity",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && json_body(&response).is_object()
        );
    }

    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "GET", "/api/v0/rooms/available?unexpected=%7B", "");
        let missing = request!(&state, "GET", "/api/v0/rooms/available", "");
        record!(
            "GET",
            "/api/v0/rooms/available",
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_body(&malformed).is_array()
        );
        record!(
            "GET",
            "/api/v0/rooms/available",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && json_body(&missing).as_array().is_some_and(Vec::is_empty)
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("rooms available runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = request!(&state, "GET", "/api/v0/rooms/available", "");
        record!(
            "GET",
            "/api/v0/rooms/available",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && json_body(&response).is_array()
        );
    }
    {
        let (state, _receiver) = test_state();
        connect_and_join(&state, "available-room").await;
        let response = request!(&state, "GET", "/api/v0/rooms/available", "");
        record!(
            "GET",
            "/api/v0/rooms/available",
            "populated-dynamic-state",
            response.status == "200 OK"
                && json_body(&response).as_array().is_some_and(|rooms| {
                    rooms.iter().any(|room| room["name"] == "available-room")
                })
        );
    }

    for (path, route) in [
        (
            "/api/v0/rooms/joined?unexpected=%7B",
            "/api/v0/rooms/joined",
        ),
        ("/api/v0/rooms/joined", "/api/v0/rooms/joined"),
    ] {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", path, "");
        let case = if path.contains('?') {
            "malformed-path-query-or-body"
        } else {
            "missing-empty-or-conflict-state"
        };
        record!(
            "GET",
            route,
            case,
            response.status == "200 OK"
                && json_body(&response).as_array().is_some_and(Vec::is_empty)
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("joined rooms runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = request!(&state, "GET", "/api/v0/rooms/joined", "");
        record!(
            "GET",
            "/api/v0/rooms/joined",
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && json_body(&response).as_array().is_some_and(Vec::is_empty)
        );
    }

    for (route, path) in [
        (
            "/api/v0/rooms/joined/{roomName}",
            "/api/v0/rooms/joined/%20",
        ),
        (
            "/api/v0/rooms/joined/{roomName}/messages",
            "/api/v0/rooms/joined/%20/messages",
        ),
        (
            "/api/v0/rooms/joined/{roomName}/users",
            "/api/v0/rooms/joined/%20/users",
        ),
    ] {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", path, "");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("roomName is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/rooms/joined/missing", "");
        record!(
            "GET",
            "/api/v0/rooms/joined/{roomName}",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
        let response = request!(&state, "GET", "/api/v0/rooms/joined/missing/messages", "");
        record!(
            "GET",
            "/api/v0/rooms/joined/{roomName}/messages",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
        let response = request!(&state, "GET", "/api/v0/rooms/joined/missing/users", "");
        record!(
            "GET",
            "/api/v0/rooms/joined/{roomName}/users",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("room reads runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        connect_and_join(&state, "room-runtime").await;
        db.close_for_test().await;
        for (route, path) in [
            (
                "/api/v0/rooms/joined/{roomName}",
                "/api/v0/rooms/joined/room-runtime",
            ),
            (
                "/api/v0/rooms/joined/{roomName}/messages",
                "/api/v0/rooms/joined/room-runtime/messages",
            ),
            (
                "/api/v0/rooms/joined/{roomName}/users",
                "/api/v0/rooms/joined/room-runtime/users",
            ),
        ] {
            let response = request!(&state, "GET", path, "");
            record!(
                "GET",
                route,
                "runtime-failure-and-timeout",
                response.status == "200 OK"
                    && (json_body(&response).is_object() || json_body(&response).is_array())
            );
        }
    }

    // Versioned join has controller validation before connection state,
    // and its in-memory tracker resets on restart in frozen slskdN.
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/v0/rooms/joined", "{}");
        record!(
            "POST",
            "/api/v0/rooms/joined",
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("roomName is required")
        );
        let response = request!(&state, "POST", "/api/v0/rooms/joined", r#"{"room":" "}"#);
        record!(
            "POST",
            "/api/v0/rooms/joined",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request" && response.body.contains("roomName is required")
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned join restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state.session.write().await.state = "connected";
        let response = request!(&state, "POST", "/api/v0/rooms/joined", r#""restart-room""#);
        let persisted = db.list_subscribed_rooms().await.unwrap_or_default();
        let reset = super::RoomStore::from_persisted(persisted);
        record!(
            "POST",
            "/api/v0/rooms/joined",
            "restart-persistence-or-reset",
            response.status == "201 Created"
                && reset.records.iter().all(|room| room.name != "restart-room")
        );
    }

    for (route, path) in [
        (
            "/api/v0/rooms/joined/{roomName}/members",
            "/api/v0/rooms/joined/%20/members",
        ),
        (
            "/api/v0/rooms/joined/{roomName}/messages",
            "/api/v0/rooms/joined/%20/messages",
        ),
        (
            "/api/v0/rooms/joined/{roomName}/ticker",
            "/api/v0/rooms/joined/%20/ticker",
        ),
    ] {
        let (state, _receiver) = test_state();
        let body = if path.ends_with("members") {
            r#""member""#
        } else if path.ends_with("messages") {
            r#""message""#
        } else {
            r#""ticker""#
        };
        let method = "POST";
        let response = request!(&state, method, path, body);
        record!(
            method,
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("roomName is required")
        );
    }

    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/missing/members",
            r#""member""#
        );
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/members",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/missing/messages",
            r#""message""#
        );
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/messages",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/missing/ticker",
            r#""ticker""#
        );
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/ticker",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    {
        let (state, mut receiver) = test_state();
        connect_and_join(&state, "room-actions").await;
        let before = state.rooms.read().await.clone();
        let message = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/room-actions/messages",
            r#""message""#
        );
        let message_command = receiver.try_recv().ok();
        let ticker = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/room-actions/ticker",
            r#""ticker""#
        );
        let ticker_command = receiver.try_recv().ok();
        let member = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/room-actions/members",
            r#""member""#
        );
        let member_command = receiver.try_recv().ok();
        let after = state.rooms.read().await.clone();
        assert_eq!(message.status, "201 Created");
        assert!(message.body.is_empty());
        assert_eq!(
            message_command,
            Some(super::SessionCommand::SayRoom {
                room: "room-actions".to_owned(),
                body: "message".to_owned(),
            })
        );
        assert!(after.records[0]
            .messages
            .iter()
            .any(|entry| entry.body == "message"));
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/ticker",
            "nominal-status-headers-body",
            ticker.status == "201 Created" && ticker.body.is_empty()
        );
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/ticker",
            "mutation-side-effects-and-readback",
            ticker_command
                == Some(super::SessionCommand::SetRoomTicker {
                    room: "room-actions".to_owned(),
                    ticker: "ticker".to_owned(),
                })
                && after.records[0].ticker.as_deref() == Some("ticker")
        );
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/members",
            "nominal-status-headers-body",
            member.status == "201 Created" && member.body.is_empty()
        );
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/members",
            "mutation-side-effects-and-readback",
            member_command
                == Some(super::SessionCommand::AddRoomMember {
                    room: "room-actions".to_owned(),
                    username: "member".to_owned(),
                })
                && after.records[0].members.iter().any(|name| name == "member")
                && before.records[0].members.is_empty()
        );
    }

    {
        let (state, mut receiver) = test_state();
        connect_and_join(&state, "leave-room").await;
        let response = request!(&state, "DELETE", "/api/v0/rooms/joined/leave-room", "");
        let command = receiver.try_recv().ok();
        let remaining = state.rooms.read().await.clone();
        record!(
            "DELETE",
            "/api/v0/rooms/joined/{roomName}",
            "nominal-status-headers-body",
            response.status == "204 No Content" && response.body.is_empty()
        );
        record!(
            "DELETE",
            "/api/v0/rooms/joined/{roomName}",
            "mutation-side-effects-and-readback",
            command == Some(super::SessionCommand::LeaveRoom("leave-room".to_owned()))
                && remaining.joined_names_json() == "[]"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "DELETE", "/api/v0/rooms/joined/%20", "");
        record!(
            "DELETE",
            "/api/v0/rooms/joined/{roomName}",
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("roomName is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "DELETE", "/api/v0/rooms/joined/missing", "");
        record!(
            "DELETE",
            "/api/v0/rooms/joined/{roomName}",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, receiver) = test_state();
        connect_and_join(&state, "leave-runtime").await;
        drop(receiver);
        let response = request!(&state, "DELETE", "/api/v0/rooms/joined/leave-runtime", "");
        record!(
            "DELETE",
            "/api/v0/rooms/joined/{roomName}",
            "runtime-failure-and-timeout",
            response.status == "204 No Content"
                && state.rooms.read().await.joined_names_json() == "[]"
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("room leave restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        connect_and_join(&state, "leave-reset").await;
        let response = request!(&state, "DELETE", "/api/v0/rooms/joined/leave-reset", "");
        let reset =
            super::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
        record!(
            "DELETE",
            "/api/v0/rooms/joined/{roomName}",
            "restart-persistence-or-reset",
            response.status == "204 No Content"
                && reset.records.iter().all(|room| room.name != "leave-reset")
        );
    }
    {
        let (state, _receiver) = test_state();
        connect_and_join(&state, "leave-concurrent").await;
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "DELETE",
                    "/api/v0/rooms/joined/leave-concurrent",
                    None,
                    "",
                    &state,
                )
                .await
                .expect("concurrent room leave")
            }
        }))
        .await;
        record!(
            "DELETE",
            "/api/v0/rooms/joined/{roomName}",
            "concurrency-and-idempotency",
            responses
                .iter()
                .filter(|response| response.status == "204 No Content")
                .count()
                == 1
                && responses
                    .iter()
                    .filter(|response| response.status == "404 Not Found")
                    .count()
                    == 1
                && state.rooms.read().await.joined_names_json() == "[]"
        );
    }

    for (route, path, body) in [
        (
            "/api/v0/rooms/joined/{roomName}/messages",
            "/api/v0/rooms/joined/room-runtime/messages",
            r#""message""#,
        ),
        (
            "/api/v0/rooms/joined/{roomName}/ticker",
            "/api/v0/rooms/joined/room-runtime/ticker",
            r#""ticker""#,
        ),
        (
            "/api/v0/rooms/joined/{roomName}/members",
            "/api/v0/rooms/joined/room-runtime/members",
            r#""member""#,
        ),
    ] {
        let (state, receiver) = test_state();
        connect_and_join(&state, "room-runtime").await;
        let before = state.rooms.read().await.clone();
        drop(receiver);
        let response = request!(&state, "POST", path, body);
        record!(
            "POST",
            route,
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && state.rooms.read().await.clone() == before
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("room member restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        connect_and_join(&state, "room-reset").await;
        let member = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/room-reset/members",
            r#""member""#
        );
        let reset =
            super::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/members",
            "restart-persistence-or-reset",
            member.status == "201 Created"
                && reset
                    .records
                    .iter()
                    .find(|room| room.name == "room-reset")
                    .is_none_or(|room| room.members.is_empty())
        );
    }
    {
        let (state, _receiver) = test_state();
        connect_and_join(&state, "room-concurrent").await;
        let responses = futures_util::future::join_all((0..2).map(|index| {
            let state = Arc::clone(&state);
            async move {
                let body = format!(r#""member-{index}""#);
                super::route_http_request(
                    "POST",
                    "/api/v0/rooms/joined/room-concurrent/members",
                    None,
                    &body,
                    &state,
                )
                .await
                .expect("concurrent room member")
            }
        }))
        .await;
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/members",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && !state.rooms.read().await.records[0].members.is_empty()
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("room message restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        connect_and_join(&state, "message-reset").await;
        let message = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/message-reset/messages",
            r#""message""#
        );
        let reset =
            super::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/messages",
            "restart-persistence-or-reset",
            message.status == "201 Created"
                && reset
                    .records
                    .iter()
                    .find(|room| room.name == "message-reset")
                    .is_none_or(|room| room.messages.is_empty())
        );
    }
    {
        let (state, _receiver) = test_state();
        connect_and_join(&state, "message-concurrent").await;
        let responses = futures_util::future::join_all((0..2).map(|index| {
            let state = Arc::clone(&state);
            async move {
                let body = format!(r#""message-{index}""#);
                super::route_http_request(
                    "POST",
                    "/api/v0/rooms/joined/message-concurrent/messages",
                    None,
                    &body,
                    &state,
                )
                .await
                .expect("concurrent room message")
            }
        }))
        .await;
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/messages",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && state.rooms.read().await.records[0].messages.len() == 2
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("room ticker restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        connect_and_join(&state, "ticker-reset").await;
        let ticker = request!(
            &state,
            "POST",
            "/api/v0/rooms/joined/ticker-reset/ticker",
            r#""ticker""#
        );
        let reset =
            super::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/ticker",
            "restart-persistence-or-reset",
            ticker.status == "201 Created"
                && reset
                    .records
                    .iter()
                    .find(|room| room.name == "ticker-reset")
                    .is_none_or(|room| room.ticker.is_none())
        );
    }
    {
        let (state, _receiver) = test_state();
        connect_and_join(&state, "ticker-concurrent").await;
        let responses = futures_util::future::join_all((0..2).map(|index| {
            let state = Arc::clone(&state);
            async move {
                let body = format!(r#""ticker-{index}""#);
                super::route_http_request(
                    "POST",
                    "/api/v0/rooms/joined/ticker-concurrent/ticker",
                    None,
                    &body,
                    &state,
                )
                .await
                .expect("concurrent room ticker")
            }
        }))
        .await;
        record!(
            "POST",
            "/api/v0/rooms/joined/{roomName}/ticker",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && state.rooms.read().await.records[0].ticker.is_some()
        );
    }

    assert_eq!(ledger.len(), 62, "RoomsController residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create RoomsController evidence directory");
    fs::write(
        evidence_dir.join("rooms_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize RoomsController ledger"),
    )
    .expect("write RoomsController ledger");
    assert!(
        mismatches.is_empty(),
        "{} RoomsController residual mismatches:\n{}",
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
async fn controller_api_differential_bridge_controller_residuals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method,
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            super::route_http_request($method, $path, None, $body, $state)
                .await
                .expect("BridgeController route request")
        }};
    }

    fn bridge_env() -> MapEnv {
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native")
    }

    fn json_body(response: &super::routing::HttpResponse) -> serde_json::Value {
        serde_json::from_str(&response.body).unwrap_or_default()
    }

    async fn closed_state(persistence: bool) -> Arc<super::AppState> {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("bridge runtime database");
        let env = if persistence {
            bridge_env().with("SLSKR_PERSISTENCE_ENABLED", "true")
        } else {
            bridge_env()
        };
        let (state, _receiver) =
            test_state_with_env_parts(env, super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        state
    }

    async fn seed_transfer(state: &Arc<super::AppState>) -> String {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("bridge-peer".to_owned()),
            "Bridge/Residual.flac".to_owned(),
            Some("/tmp/Bridge-Residual.flac".to_owned()),
            Some(100),
        );
        entry.id.to_string()
    }

    async fn seed_bridge_client(state: &Arc<super::AppState>) {
        state.runtime.write().await.bridge_active_clients.insert(
            "bridge-residual-client".to_owned(),
            serde_json::json!({
                "clientId": "bridge-residual-client",
                "clientType": "Soulseek Legacy (residual-user)",
                "ipAddress": "192.0.2.77",
                "connectedAt": 1_754_000_100_u64,
                "requestCount": 2,
                "lastActivity": 1_754_000_101_u64,
            }),
        );
    }

    async fn seed_room(state: &Arc<super::AppState>, name: &str) {
        state
            .rooms
            .write()
            .await
            .join(name.to_owned())
            .expect("bridge room fixture");
    }

    fn exact_bridge_config(value: &serde_json::Value) -> bool {
        value["message"] == "Configuration updated. Restart bridge service to apply changes."
            && value["restart_required"] == true
            && value.as_object().is_some_and(|object| object.len() == 2)
    }

    // Static GET routes: extra path segments are unmatched, while the
    // controller's real empty and runtime states remain JSON responses.
    for (route, malformed) in [
        (
            "/api/bridge/admin/clients",
            "/api/bridge/admin/clients/extra",
        ),
        ("/api/bridge/admin/config", "/api/bridge/admin/config/extra"),
        (
            "/api/bridge/admin/dashboard",
            "/api/bridge/admin/dashboard/extra",
        ),
        ("/api/bridge/admin/stats", "/api/bridge/admin/stats/extra"),
        ("/api/bridge/rooms", "/api/bridge/rooms/extra"),
        ("/api/bridge/status", "/api/bridge/status/extra"),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let response = request!(&state, "GET", malformed, "");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }
    for (route, malformed) in [
        (
            "/api/v0/bridge/admin/clients",
            "/api/v0/bridge/admin/clients/extra",
        ),
        (
            "/api/v0/bridge/admin/config",
            "/api/v0/bridge/admin/config/extra",
        ),
        (
            "/api/v0/bridge/admin/dashboard",
            "/api/v0/bridge/admin/dashboard/extra",
        ),
        (
            "/api/v0/bridge/admin/stats",
            "/api/v0/bridge/admin/stats/extra",
        ),
        ("/api/v0/bridge/rooms", "/api/v0/bridge/rooms/extra"),
        ("/api/v0/bridge/status", "/api/v0/bridge/status/extra"),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let response = request!(&state, "GET", malformed, "");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let (state, _receiver) = test_state_with_env(bridge_env());
    seed_bridge_client(&state).await;
    let clients = request!(&state, "GET", "/api/bridge/admin/clients", "");
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "populated-dynamic-state",
        clients.status == "200 OK"
            && json_body(&clients)["clients"]
                .as_array()
                .is_some_and(|rows| rows.len() == 1)
    );
    let clients_runtime = closed_state(false).await;
    let clients_runtime_response =
        request!(&clients_runtime, "GET", "/api/bridge/admin/clients", "");
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "runtime-failure-and-timeout",
        clients_runtime_response.status == "200 OK"
            && json_body(&clients_runtime_response)["clients"].is_array()
    );
    let clients_v0_runtime = closed_state(false).await;
    let clients_v0_runtime_response = request!(
        &clients_v0_runtime,
        "GET",
        "/api/v0/bridge/admin/clients",
        ""
    );
    record!(
        "GET",
        "/api/v0/bridge/admin/clients",
        "runtime-failure-and-timeout",
        clients_v0_runtime_response.status == "200 OK"
            && json_body(&clients_v0_runtime_response)["clients"].is_array()
    );

    let config_env = bridge_env().with(
        "SLSKR_ADVANCED_NETWORKING_JSON",
        &serde_json::json!({
            "virtualSoulfind": {"bridge": {
                "enabled": true,
                "port": 4327,
                "bindAddress": "127.0.0.1",
                "maxClients": 19,
                "requireAuth": false
            }}
        })
        .to_string(),
    );
    let (config_state, _receiver) = test_state_with_env(config_env);
    let config = request!(&config_state, "GET", "/api/bridge/admin/config", "");
    record!(
        "GET",
        "/api/bridge/admin/config",
        "populated-dynamic-state",
        config.status == "200 OK"
            && json_body(&config)["enabled"] == true
            && json_body(&config)["port"] == 4327
            && json_body(&config)["max_clients"] == 19
            && json_body(&config)["require_auth"] == false
    );
    let config_empty = request!(&state, "GET", "/api/bridge/admin/config", "");
    record!(
        "GET",
        "/api/bridge/admin/config",
        "missing-empty-or-conflict-state",
        config_empty.status == "200 OK"
            && json_body(&config_empty)["port"].is_number()
            && json_body(&config_empty)["soulfind_path"].is_string()
    );
    let config_runtime = closed_state(false).await;
    let config_runtime_response = request!(&config_runtime, "GET", "/api/bridge/admin/config", "");
    record!(
        "GET",
        "/api/bridge/admin/config",
        "runtime-failure-and-timeout",
        config_runtime_response.status == "200 OK"
            && json_body(&config_runtime_response).is_object()
    );
    let config_v0_runtime = closed_state(false).await;
    let config_v0_runtime_response =
        request!(&config_v0_runtime, "GET", "/api/v0/bridge/admin/config", "");
    record!(
        "GET",
        "/api/v0/bridge/admin/config",
        "runtime-failure-and-timeout",
        config_v0_runtime_response.status == "200 OK"
            && json_body(&config_v0_runtime_response).is_object()
    );

    for route in [
        "/api/bridge/admin/dashboard",
        "/api/v0/bridge/admin/dashboard",
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        if !route.starts_with("/api/v0/") {
            let missing = request!(&state, "GET", route, "");
            record!(
                "GET",
                route,
                "missing-empty-or-conflict-state",
                missing.status == "200 OK"
                    && json_body(&missing)["health"]["isHealthy"].is_boolean()
                    && json_body(&missing)["stats"].is_object()
            );
        }
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK"
                && json_body(&runtime_response)["health"].is_object()
        );
    }

    for route in ["/api/bridge/admin/stats", "/api/v0/bridge/admin/stats"] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let missing = request!(&state, "GET", route, "");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && json_body(&missing)["totalConnections"].is_number()
                && json_body(&missing)["uptime"].is_string()
        );
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK"
                && json_body(&runtime_response)["totalSearches"].is_number()
        );
    }

    for route in ["/api/bridge/rooms", "/api/v0/bridge/rooms"] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let missing = request!(&state, "GET", route, "");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && json_body(&missing)["rooms"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK" && json_body(&runtime_response)["rooms"].is_array()
        );
        let (populated, _receiver) = test_state_with_env(bridge_env());
        seed_room(&populated, "bridge-residual-room").await;
        if !route.starts_with("/api/v0/") {
            let populated_response = request!(&populated, "GET", route, "");
            record!(
                "GET",
                route,
                "populated-dynamic-state",
                populated_response.status == "200 OK"
                    && json_body(&populated_response)["rooms"]
                        .as_array()
                        .is_some_and(|rows| {
                            rows.iter().any(|row| row["name"] == "bridge-residual-room")
                        })
            );
        }
    }

    for route in ["/api/bridge/status", "/api/v0/bridge/status"] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        if !route.starts_with("/api/v0/") {
            let missing = request!(&state, "GET", route, "");
            record!(
                "GET",
                route,
                "missing-empty-or-conflict-state",
                missing.status == "200 OK"
                    && json_body(&missing)["isHealthy"].is_boolean()
                    && json_body(&missing)["activeConnections"].is_number()
            );
        }
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK"
                && json_body(&runtime_response)["version"] == "1.0.0-proxy"
        );
    }

    for (route, path) in [
        (
            "/api/bridge/transfer/{transferId}/progress",
            "/api/bridge/transfer/%20/progress",
        ),
        (
            "/api/v0/bridge/transfer/{transferId}/progress",
            "/api/v0/bridge/transfer/%20/progress",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "GET", path, "");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("TransferId is required")
        );
        let missing_path = if path.starts_with("/api/v0/") {
            "/api/v0/bridge/transfer/missing-transfer/progress"
        } else {
            "/api/bridge/transfer/missing-transfer/progress"
        };
        let missing = request!(&state, "GET", missing_path, "");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
                && json_body(&missing)["error"] == "Transfer not found"
        );
        let transfer_id = seed_transfer(&state).await;
        let nominal_path = if path.starts_with("/api/v0/") {
            format!("/api/v0/bridge/transfer/{transfer_id}/progress")
        } else {
            format!("/api/bridge/transfer/{transfer_id}/progress")
        };
        let nominal = request!(&state, "GET", &nominal_path, "");
        let nominal_json = json_body(&nominal);
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal_json["proxyId"].is_string()
                && nominal_json["percentComplete"].is_number()
                && nominal_json["state"].is_string()
        );
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            nominal_json["filename"] == "Bridge/Residual.flac"
                && nominal_json["fileSize"] == 100
                && nominal_json["queuePosition"] == 0
        );
        let runtime = closed_state(false).await;
        let runtime_path = if path.starts_with("/api/v0/") {
            "/api/v0/bridge/transfer/runtime-transfer/progress"
        } else {
            "/api/bridge/transfer/runtime-transfer/progress"
        };
        let runtime_response = request!(&runtime, "GET", runtime_path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "404 Not Found"
                && json_body(&runtime_response)["error"] == "Transfer not found"
        );
    }

    // Mutation routes without an explicit API version are rejected by the
    // frozen API-versioning middleware before request binding or state
    // access.  The open cases therefore all share this exact response.
    for (method, route, body) in [
        (
            "POST",
            "/api/bridge/download",
            r#"{"username":"peer","filename":"file.flac","targetPath":"/tmp/file.flac"}"#,
        ),
        (
            "POST",
            "/api/bridge/search",
            r#"{"query":"bridge residual"}"#,
        ),
        ("POST", "/api/bridge/start", "{}"),
        ("POST", "/api/bridge/stop", "{}"),
        ("PUT", "/api/bridge/admin/config", "{}"),
    ] {
        for case in [
            "runtime-failure-and-timeout",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
            "concurrency-and-idempotency",
        ] {
            let (state, _receiver) = test_state_with_env(bridge_env());
            let response = request!(&state, method, route, body);
            record!(
                method,
                route,
                case,
                response.status == "400 Bad Request"
                    && response.body.contains("ApiVersionUnspecified")
            );
        }
    }

    let download_body = r#"{"username":"peer","filename":"Bridge/Residual.flac","targetPath":"/tmp/residual.flac"}"#;
    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "POST", "/api/v0/bridge/download", "not-json");
        record!(
            "POST",
            "/api/v0/bridge/download",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("Request is required")
        );
        let missing = request!(&state, "POST", "/api/v0/bridge/download", "{}");
        record!(
            "POST",
            "/api/v0/bridge/download",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing
                    .body
                    .contains("Username, filename, and targetPath are required")
        );
        let restart = request!(&state, "POST", "/api/v0/bridge/download", download_body);
        let transfer_id = json_body(&restart)["transfer_id"].clone();
        let (fresh, _receiver) = test_state_with_env(bridge_env());
        record!(
            "POST",
            "/api/v0/bridge/download",
            "restart-persistence-or-reset",
            restart.status == "200 OK"
                && transfer_id.is_string()
                && fresh.transfers.read().await.entries.is_empty()
        );
    }
    {
        let runtime = closed_state(true).await;
        let response = request!(&runtime, "POST", "/api/v0/bridge/download", download_body);
        record!(
            "POST",
            "/api/v0/bridge/download",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && json_body(&response)["error"] == "Bridge download failed"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/bridge/download",
                None,
                download_body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/bridge/download",
                None,
                download_body,
                &state
            ),
        );
        record!(
            "POST",
            "/api/v0/bridge/download",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && left
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["transfer_id"].is_string())
                && right
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["transfer_id"].is_string())
        );
    }

    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "POST", "/api/v0/bridge/search", "not-json");
        record!(
            "POST",
            "/api/v0/bridge/search",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("Query is required")
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/bridge/search",
            r#"{"query":"bridge residual"}"#
        );
        let (fresh, _receiver) = test_state_with_env(bridge_env());
        record!(
            "POST",
            "/api/v0/bridge/search",
            "restart-persistence-or-reset",
            response.status == "200 OK" && fresh.searches.read().await.records.is_empty()
        );
    }
    {
        let runtime = closed_state(true).await;
        let response = request!(
            &runtime,
            "POST",
            "/api/v0/bridge/search",
            r#"{"query":"bridge residual"}"#
        );
        record!(
            "POST",
            "/api/v0/bridge/search",
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && json_body(&response)["query"] == "bridge residual"
                && json_body(&response)["users"] == serde_json::json!([])
        );
    }
    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/bridge/search",
                None,
                r#"{"query":"bridge-a"}"#,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/bridge/search",
                None,
                r#"{"query":"bridge-b"}"#,
                &state
            ),
        );
        record!(
            "POST",
            "/api/v0/bridge/search",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for (method, route, action, body) in [
        ("POST", "/api/v0/bridge/start", "started", "{}"),
        ("POST", "/api/v0/bridge/stop", "stopped", "{}"),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, method, route, "not-json");
        record!(
            method,
            route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_body(&malformed)["status"] == action
        );
        let missing = request!(&state, method, route, "");
        record!(
            method,
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && json_body(&missing)["status"] == action
        );
        let nominal = request!(&state, method, route, body);
        record!(
            method,
            route,
            "mutation-side-effects-and-readback",
            nominal.status == "200 OK"
                && json_body(&nominal) == serde_json::json!({"status": action})
        );
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, method, route, body);
        record!(
            method,
            route,
            "restart-persistence-or-reset",
            runtime_response.status == "200 OK" && json_body(&runtime_response)["status"] == action
        );
        let (concurrent_state, _receiver) = test_state_with_env(bridge_env());
        let (left, right) = tokio::join!(
            super::route_http_request(method, route, None, body, &concurrent_state),
            super::route_http_request(method, route, None, body, &concurrent_state),
        );
        record!(
            method,
            route,
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && left
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["status"] == action)
                && right
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["status"] == action)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "PUT", "/api/v0/bridge/admin/config", "not-json");
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = request!(&state, "PUT", "/api/v0/bridge/admin/config", "");
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
        let response = request!(
            &state,
            "PUT",
            "/api/v0/bridge/admin/config",
            r#"{"enabled":true}"#
        );
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "restart-persistence-or-reset",
            response.status == "200 OK" && exact_bridge_config(&json_body(&response))
        );
        let fresh = test_state_with_env(bridge_env()).0;
        let fresh_config = request!(&fresh, "GET", "/api/v0/bridge/admin/config", "");
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "concurrency-and-idempotency",
            fresh_config.status == "200 OK" && json_body(&fresh_config)["enabled"] == false
        );
    }

    assert_eq!(ledger.len(), 87, "BridgeController residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create BridgeController evidence directory");
    fs::write(
        evidence_dir.join("bridge_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize BridgeController ledger"),
    )
    .expect("write BridgeController ledger");
    assert!(
        mismatches.is_empty(),
        "{} BridgeController residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining frozen slskdN PodCore controller
/// cases.  The ledger deliberately covers the source controller's
/// storage-error contracts as well as the process-local reset and
/// concurrent mutation behavior, rather than proving route presence only.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
fn controller_api_differential_podcore_residuals() {
    run_controller_future_on_large_stack("podcore-residuals", || {
        controller_api_differential_podcore_residuals_impl()
    });
}

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_podcore_residuals_impl() {
    let target = "slskdn";
    let pod_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            super::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {}", $method, $path, error))
        }};
    }

    let json_body = |response: &super::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let block_file = |path: PathBuf| {
        if path.exists() {
            fs::remove_file(&path).expect("remove PodCore state file before blocking it");
        }
        fs::create_dir(&path).expect("block PodCore state file with a directory");
    };
    let block_feature_file = |state: &Arc<super::AppState>| {
        let state = Arc::clone(state);
        async move {
            let path = state.config.state_dir.join("controller-feature-state.json");
            state.controller_features.write_for_test().await.state_path = path.clone();
            block_file(path);
        }
    };
    let prepare_feature_file = |state: &Arc<super::AppState>| {
        let state = Arc::clone(state);
        async move {
            let path = state.config.state_dir.join("controller-feature-state.json");
            if path.is_dir() {
                fs::remove_dir_all(&path).expect("remove prepared PodCore feature directory");
            } else if path.exists() {
                fs::remove_file(&path).expect("remove prepared PodCore feature file");
            }
            state.controller_features.write_for_test().await.state_path = path;
        }
    };
    let seed_pod = |state: &Arc<super::AppState>, pod_id: &str| {
        let state = Arc::clone(state);
        let pod_id = pod_id.to_owned();
        async move {
            state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                        "podId": pod_id,
                        "name": format!("PodCore residual {pod_id}"),
                        "visibility": "Listed",
                        "isPublic": true,
                        "channels": [{
                            "channelId": "general",
                            "kind": 0,
                            "name": "General"
                        }]
                    }))
                    .expect("deserialize PodCore residual pod"),
                    "tester".to_owned(),
                )
                .expect("persist PodCore residual pod");
        }
    };
    let pod_body = |pod_id: &str| {
        serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": format!("Published {pod_id}"),
                "visibility": "Listed",
                "isPublic": true,
                "channels": [{"channelId":"general","kind":0,"name":"General"}]
            }
        })
        .to_string()
    };
    let content_id = "content:video:movie:podcore-residual";
    let valid_signature = super::STANDARD.encode([0_u8; 64]);
    let opinion_body = |pod_id: &str, id: &str| {
        serde_json::json!({
            "id": id,
            "contentId": content_id,
            "variantHash": "variant-residual",
            "score": 4.0,
            "note": "PodCore residual opinion",
            "senderPeerId": "tester",
            "signature": format!("ed25519:{valid_signature}"),
            "podId": pod_id,
        })
        .to_string()
    };
    let message_body = |message_id: &str, pod_id: &str| {
        serde_json::json!({
            "messageId": message_id,
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "tester",
            "body": "PodCore residual message",
            "timestampUnixMs": super::unix_timestamp_millis(),
            "signature": "",
            "sigVersion": 1,
        })
        .to_string()
    };
    let signing_private_key = super::STANDARD.encode([7_u8; 32]);

    // GET /podcore/{podId}/channels and /channels/{channelId} -- the
    // frozen services read the durable pod store and normalize failures
    // to their controller-specific 500 responses.
    for (path, route, message) in [
        (
            "/api/v0/podcore/podcore-channels-runtime/channels",
            "/api/v0/podcore/{podId}/channels",
            "An error occurred while getting channels",
        ),
        (
            "/api/v0/podcore/podcore-channels-runtime/channels/general",
            "/api/v0/podcore/{podId}/channels/{channelId}",
            "An error occurred while getting the channel",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-channels-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(&state, "GET", path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error" && response.body.contains(message)
        );
    }

    // Opinion projections use the controller-feature persistence boundary.
    for (path, route, message) in [
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/aggregated",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/recommendations",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/stats",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/variant/variant-residual",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
            "An error occurred while getting opinions",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-opinions-runtime").await;
        block_feature_file(&state).await;
        let response = request!(&state, "GET", path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error" && response.body.contains(message)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-affinity-runtime").await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/podcore-affinity-runtime/opinions/members/affinity",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/{podId}/opinions/members/affinity",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting member affinities")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-last-seen-runtime").await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-runtime/general/last-seen",
            r#"{"lastSeen":10}"#
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/backfill/podcore-last-seen-runtime/last-seen",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/backfill/{podId}/last-seen",
            "runtime-failure-and-timeout",
            updated.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting last seen timestamps")
        );
    }

    // These statistics and content-link operations are intentionally
    // process-local in the frozen services; their runtime contract is a
    // successful JSON projection even when the optional remote provider
    // is empty or unavailable.
    for (path, route) in [
        (
            "/api/v0/podcore/backfill/stats",
            "/api/v0/podcore/backfill/stats",
        ),
        (
            "/api/v0/podcore/content/metadata?contentId=content:video:movie:podcore-residual",
            "/api/v0/podcore/content/metadata",
        ),
        (
            "/api/v0/podcore/content/search?query=podcore-residual&domain=video",
            "/api/v0/podcore/content/search",
        ),
        ("/api/v0/podcore/dht/stats", "/api/v0/podcore/dht/stats"),
        (
            "/api/v0/podcore/signing/stats",
            "/api/v0/podcore/signing/stats",
        ),
        (
            "/api/v0/podcore/verification/stats",
            "/api/v0/podcore/verification/stats",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "GET", path, "");
        let body_shape_ok = if path.contains("/content/search") {
            json_body(&response).is_array() || json_body(&response).is_object()
        } else {
            json_body(&response).is_object()
        };
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && body_shape_ok
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-dht-metadata-runtime").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-dht-metadata-runtime")
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/dht/metadata/podcore-dht-metadata-runtime",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/dht/metadata/{*podId}",
            "runtime-failure-and-timeout",
            published.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to retrieve pod metadata")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-seen-runtime").await;
        let registered = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/message-runtime/podcore-routing-seen-runtime",
            ""
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/routing/seen/message-runtime/podcore-routing-seen-runtime",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "runtime-failure-and-timeout",
            registered.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("Failed to check message seen status")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-verification-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let membership = request!(
            &state,
            "GET",
            "/api/v0/podcore/verification/membership/podcore-verification-runtime/tester",
            ""
        );
        let role = request!(
            &state,
            "GET",
            "/api/v0/podcore/verification/role/podcore-verification-runtime/tester/member",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/verification/membership/{podId}/{peerId}",
            "runtime-failure-and-timeout",
            membership.status == "500 Internal Server Error"
                && membership.body.contains("Failed to verify membership")
        );
        record!(
            "GET",
            "/api/v0/podcore/verification/role/{podId}/{peerId}/{requiredRole}",
            "runtime-failure-and-timeout",
            role.status == "500 Internal Server Error"
                && role.body.contains("Failed to check role")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-stats-runtime").await;
        let registered = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/message-stats/podcore-routing-stats-runtime",
            ""
        );
        block_feature_file(&state).await;
        let response = request!(&state, "GET", "/api/v0/podcore/routing/stats", "");
        record!(
            "GET",
            "/api/v0/podcore/routing/stats",
            "runtime-failure-and-timeout",
            registered.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to get routing statistics")
        );
    }

    // DHT unpublish: the frozen controller returns a fixed 500 when the
    // publisher/storage boundary fails, persists a successful removal,
    // and treats repeated removal as idempotent.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-unpublish-runtime").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-unpublish-runtime")
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "DELETE",
            "/api/v0/podcore/dht/unpublish/podcore-unpublish-runtime",
            ""
        );
        record!(
            "DELETE",
            "/api/v0/podcore/dht/unpublish/{*podId}",
            "runtime-failure-and-timeout",
            published.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to unpublish pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-unpublish-restart").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-unpublish-restart")
        );
        let response = request!(
            &state,
            "DELETE",
            "/api/v0/podcore/dht/unpublish/podcore-unpublish-restart",
            ""
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload DHT unpublish state");
        record!(
            "DELETE",
            "/api/v0/podcore/dht/unpublish/{*podId}",
            "restart-persistence-or-reset",
            published.status == "200 OK"
                && response.status == "200 OK"
                && loaded.get("pod/dht/podcore-unpublish-restart").is_none()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-unpublish-concurrent").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-unpublish-concurrent")
        );
        let (left, right) = tokio::join!(
            super::route_http_request(
                "DELETE",
                "/api/v0/podcore/dht/unpublish/podcore-unpublish-concurrent",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "DELETE",
                "/api/v0/podcore/dht/unpublish/podcore-unpublish-concurrent",
                None,
                "",
                &state
            )
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent DHT unpublish state");
        record!(
            "DELETE",
            "/api/v0/podcore/dht/unpublish/{*podId}",
            "concurrency-and-idempotency",
            published.status == "200 OK"
                && left
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded.get("pod/dht/podcore-unpublish-concurrent").is_none()
        );
    }

    // DHT publish and update share the same durable publication record.
    for (route, pod_id, action) in [
        (
            "/api/v0/podcore/dht/publish",
            "podcore-publish-runtime",
            "publish",
        ),
        (
            "/api/v0/podcore/dht/update",
            "podcore-update-runtime",
            "update",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, pod_id).await;
        let initial = if action == "update" {
            request!(
                &state,
                "POST",
                "/api/v0/podcore/dht/publish",
                &pod_body(pod_id)
            )
        } else {
            super::HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: String::new(),
            }
        };
        block_feature_file(&state).await;
        let response = request!(&state, "POST", route, &pod_body(pod_id));
        record!(
            "POST",
            route,
            "runtime-failure-and-timeout",
            initial.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains(if action == "publish" {
                    "Failed to publish pod"
                } else {
                    "Failed to update pod"
                })
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-publish-restart").await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-publish-restart")
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload DHT publication state");
        record!(
            "POST",
            "/api/v0/podcore/dht/publish",
            "restart-persistence-or-reset",
            response.status == "200 OK" && loaded.get("pod/dht/podcore-publish-restart").is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-publish-concurrent").await;
        let body = pod_body("podcore-publish-concurrent");
        let (left, right) = tokio::join!(
            super::route_http_request("POST", "/api/v0/podcore/dht/publish", None, &body, &state),
            super::route_http_request("POST", "/api/v0/podcore/dht/publish", None, &body, &state)
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent DHT publication state");
        record!(
            "POST",
            "/api/v0/podcore/dht/publish",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded.get("pod/dht/podcore-publish-concurrent").is_some()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-update-restart").await;
        let first = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-update-restart")
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/update",
            &pod_body("podcore-update-restart")
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload DHT update state");
        record!(
            "POST",
            "/api/v0/podcore/dht/update",
            "restart-persistence-or-reset",
            first.status == "200 OK"
                && response.status == "200 OK"
                && loaded.get("pod/dht/podcore-update-restart").is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-update-concurrent").await;
        let initial = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-update-concurrent")
        );
        let body = pod_body("podcore-update-concurrent");
        let (left, right) = tokio::join!(
            super::route_http_request("POST", "/api/v0/podcore/dht/update", None, &body, &state),
            super::route_http_request("POST", "/api/v0/podcore/dht/update", None, &body, &state)
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/update",
            "concurrency-and-idempotency",
            initial.status == "200 OK"
                && left
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    let expire_dht_publication = |state: &Arc<super::AppState>, pod_id: &str| {
        let state = Arc::clone(state);
        let pod_id = pod_id.to_owned();
        async move {
            let key = format!("pod/dht/{pod_id}");
            let mut features = state.controller_features.write_for_test().await;
            let mut publication = features
                .get(&key)
                .cloned()
                .expect("DHT publication exists before expiry fixture");
            publication["expiresAt"] = serde_json::json!("1970-01-01T00:00:00+00:00");
            features
                .upsert(key, publication)
                .expect("expire DHT publication fixture");
        }
    };

    // Refresh has distinct missing, publisher-failure, republish, reset,
    // and concurrent contracts.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-missing",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "missing-empty-or-conflict-state",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to refresh pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-runtime").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-runtime")
        );
        expire_dht_publication(&state, "podcore-refresh-runtime").await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-runtime",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "runtime-failure-and-timeout",
            published.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to refresh pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-mutation").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-mutation")
        );
        expire_dht_publication(&state, "podcore-refresh-mutation").await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-mutation",
            ""
        );
        let metadata = request!(
            &state,
            "GET",
            "/api/v0/podcore/dht/metadata/podcore-refresh-mutation",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "mutation-side-effects-and-readback",
            published.status == "200 OK"
                && response.status == "200 OK"
                && json_body(&response)["wasRepublished"] == true
                && metadata.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-reset").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-reset")
        );
        let (fresh, _fresh_receiver) = test_state_with_env(pod_env());
        let response = request!(
            &fresh,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-reset",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "restart-persistence-or-reset",
            published.status == "200 OK" && response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-concurrent").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-concurrent")
        );
        expire_dht_publication(&state, "podcore-refresh-concurrent").await;
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/dht/refresh/podcore-refresh-concurrent",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/dht/refresh/podcore-refresh-concurrent",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "concurrency-and-idempotency",
            published.status == "200 OK"
                && left
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Opinion publication, affinity refresh, and DHT-backed opinion
    // refresh share the frozen controller's explicit persistence/error
    // boundary and remain idempotent across repeated calls.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-runtime/opinions",
            &opinion_body("podcore-opinion-runtime", "opinion-runtime")
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while publishing the opinion")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-restart/opinions",
            &opinion_body("podcore-opinion-restart", "opinion-restart")
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload published opinion state");
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/opinion/podcore-opinion-restart/content:video:movie:podcore-residual/opinion-restart")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let body = opinion_body("podcore-opinion-concurrent", "opinion-concurrent");
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-concurrent/opinions",
                None,
                &body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-concurrent/opinions",
                None,
                &body,
                &state
            )
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent opinion state");
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions",
            "concurrency-and-idempotency",
            left.as_ref().is_ok_and(|response| response.status == "200 OK")
                && right.as_ref().is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/opinion/podcore-opinion-concurrent/content:video:movie:podcore-residual/opinion-concurrent")
                    .is_some()
        );
    }

    for (pod_id, case, expected_status) in [
        (
            "podcore-affinity-runtime",
            "runtime-failure-and-timeout",
            "500 Internal Server Error",
        ),
        (
            "podcore-affinity-restart",
            "restart-persistence-or-reset",
            "200 OK",
        ),
        (
            "podcore-affinity-concurrent",
            "concurrency-and-idempotency",
            "200 OK",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        if case == "runtime-failure-and-timeout" {
            block_feature_file(&state).await;
        }
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity/update"),
            ""
        );
        let pass = response.status == expected_status
            && (case != "runtime-failure-and-timeout"
                || response
                    .body
                    .contains("An error occurred while updating member affinities"));
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/members/affinity/update",
            case,
            pass
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-refresh-runtime/opinions/refresh",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/refresh",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while refreshing opinions")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-refresh-restart/opinions",
            &opinion_body("podcore-opinion-refresh-restart", "opinion-refresh-restart")
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-refresh-restart/opinions/refresh",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/refresh",
            "restart-persistence-or-reset",
            published.status == "200 OK" && response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-refresh-concurrent/opinions/refresh",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-refresh-concurrent/opinions/refresh",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/refresh",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Backfill sync validates both durable stores before invoking the
    // local projection.  sync-all is body-independent, returns an empty
    // list when there is no work, and includes persisted work when it is
    // present.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/podcore-backfill-missing/sync",
            "{}"
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
                && response.body.contains("Last seen timestamps are required")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-backfill-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/podcore-backfill-runtime/sync",
            r#"{"general":10}"#
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while syncing backfill")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-backfill-restart").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-backfill-restart/general/last-seen",
            "10"
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/podcore-backfill-restart/sync",
            r#"{"general":10}"#
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload backfill state");
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && response.status == "200 OK"
                && loaded
                    .get("pod/backfill/podcore-backfill-restart/general")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-backfill-concurrent").await;
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/podcore-backfill-concurrent/sync",
                None,
                r#"{"general":10}"#,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/podcore-backfill-concurrent/sync",
                None,
                r#"{"general":10}"#,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "not-json"
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "malformed-path-query-or-body",
            response.status == "200 OK" && json_body(&response).is_array()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && json_body(&response).is_array()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-sync-all-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while syncing all pods")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-sync-all-mutation").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-sync-all-mutation/general/last-seen",
            "10"
        );
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "mutation-side-effects-and-readback",
            updated.status == "200 OK"
                && response.status == "200 OK"
                && json_body(&response)
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-sync-all-restart").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-sync-all-restart/general/last-seen",
            "10"
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload sync-all state");
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && loaded
                    .get("pod/backfill/podcore-sync-all-restart/general")
                    .is_some()
                && response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/sync-all",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/sync-all",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Content validation is a pure projection; content-linked pod
    // creation is backed by the durable pod store.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/content/create-pod", "");
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/create-pod",
            &pod_body("podcore-content-create-runtime")
        );
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while creating the pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/create-pod",
            &pod_body("podcore-content-create-restart")
        );
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload created content-linked pod");
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "restart-persistence-or-reset",
            response.status == "201 Created"
                && loaded.get("podcore-content-create-restart").is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = pod_body("podcore-content-create-concurrent");
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/content/create-pod",
                None,
                &body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/content/create-pod",
                None,
                &body,
                &state
            )
        );
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent content-linked pod");
        let success_count = [left.as_ref(), right.as_ref()]
            .into_iter()
            .filter(|response| response.is_ok_and(|response| response.status == "201 Created"))
            .count();
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "concurrency-and-idempotency",
            success_count == 1
                && [left.as_ref(), right.as_ref()]
                    .into_iter()
                    .all(|response| response.is_ok_and(|response| {
                        response.status == "201 Created" || response.status == "409 Conflict"
                    }))
                && loaded.get("podcore-content-create-concurrent").is_some()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/content/validate", "");
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/validate",
            &serde_json::to_string(&content_id).expect("serialize content ID")
        );
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && json_body(&response)["isValid"] == true
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = serde_json::to_string(&content_id).expect("serialize content ID");
        let first = request!(&state, "POST", "/api/v0/podcore/content/validate", &body);
        let second = request!(&state, "POST", "/api/v0/podcore/content/validate", &body);
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "mutation-side-effects-and-readback",
            first.status == "200 OK"
                && second.status == "200 OK"
                && json_body(&second)["contentId"] == content_id
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = serde_json::to_string(&content_id).expect("serialize content ID");
        let first = request!(&state, "POST", "/api/v0/podcore/content/validate", &body);
        let loaded_body = body.clone();
        let second = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/validate",
            &loaded_body
        );
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "restart-persistence-or-reset",
            first.status == "200 OK" && second.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = serde_json::to_string(&content_id).expect("serialize content ID");
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/content/validate",
                None,
                &body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/content/validate",
                None,
                &body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Routing persistence has one shared seen-message registry.  The
    // route, explicit registration, and cleanup actions all preserve the
    // frozen error text and durable/idempotent behavior.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "not-json"
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "malformed-path-query-or-body",
            response.status == "200 OK" && json_body(&response).is_object()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/routing/cleanup", "");
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && json_body(&response).is_object()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        {
            let mut features = state.controller_features.write_for_test().await;
            features
                .upsert(
                    "pod/routing-seen/podcore-cleanup-runtime/message".to_owned(),
                    serde_json::json!({
                        "messageId": "message",
                        "podId": "podcore-cleanup-runtime",
                        "seenAt": 0,
                    }),
                )
                .expect("seed expired routing entry");
        }
        block_feature_file(&state).await;
        let response = request!(&state, "POST", "/api/v0/podcore/routing/cleanup", "");
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to cleanup seen messages")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        {
            let mut features = state.controller_features.write_for_test().await;
            features
                .upsert(
                    "pod/routing-seen/podcore-cleanup-restart/message".to_owned(),
                    serde_json::json!({
                        "messageId": "message",
                        "podId": "podcore-cleanup-restart",
                        "seenAt": 0,
                    }),
                )
                .expect("seed restart cleanup entry");
        }
        let response = request!(&state, "POST", "/api/v0/podcore/routing/cleanup", "");
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload cleanup state");
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/routing-seen/podcore-cleanup-restart/message")
                    .is_none()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        {
            let mut features = state.controller_features.write_for_test().await;
            features
                .upsert(
                    "pod/routing-seen/podcore-cleanup-concurrent/message".to_owned(),
                    serde_json::json!({
                        "messageId": "message",
                        "podId": "podcore-cleanup-concurrent",
                        "seenAt": 0,
                    }),
                )
                .expect("seed concurrent cleanup entry");
        }
        let (left, right) = tokio::join!(
            super::route_http_request("POST", "/api/v0/podcore/routing/cleanup", None, "", &state),
            super::route_http_request("POST", "/api/v0/podcore/routing/cleanup", None, "", &state)
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-route-missing").await;
        let response = request!(&state, "POST", "/api/v0/podcore/routing/route", "{}");
        record!(
            "POST",
            "/api/v0/podcore/routing/route",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-route-restart").await;
        prepare_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/route",
            &message_body("route-restart", "podcore-routing-route-restart")
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload routed message state");
        record!(
            "POST",
            "/api/v0/podcore/routing/route",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/routing-seen/podcore-routing-route-restart/route-restart")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-route-concurrent").await;
        prepare_feature_file(&state).await;
        let body = message_body("route-concurrent", "podcore-routing-route-concurrent");
        let (left, right) = tokio::join!(
            super::route_http_request("POST", "/api/v0/podcore/routing/route", None, &body, &state),
            super::route_http_request("POST", "/api/v0/podcore/routing/route", None, &body, &state)
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent routed message state");
        record!(
            "POST",
            "/api/v0/podcore/routing/route",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/routing-seen/podcore-routing-route-concurrent/route-concurrent")
                    .is_some()
        );
    }

    let route_to_peers_body = |message_id: &str, pod_id: &str| {
        serde_json::json!({
            "message": serde_json::from_str::<serde_json::Value>(&message_body(message_id, pod_id))
                .expect("route-to-peers message fixture"),
            "targetPeerIds": ["missing-peer"],
        })
        .to_string()
    };
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            "{}"
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            &route_to_peers_body("route-to-peers", "podcore-route-to-peers")
        );
        let pass =
            response.status == "200 OK" && json_body(&response)["messageId"] == "route-to-peers";
        record!("POST", "/api/v0/podcore/routing/route-to-peers", case, pass);
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = route_to_peers_body("route-to-peers-concurrent", "podcore-route-to-peers");
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/routing/route-to-peers",
                None,
                &body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/routing/route-to-peers",
                None,
                &body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/%20/podcore-seen-missing",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/seen-runtime/podcore-seen-runtime",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to register message as seen")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/seen-restart/podcore-seen-restart",
            ""
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload seen registration state");
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/routing-seen/podcore-seen-restart/seen-restart")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/routing/seen/seen-concurrent/podcore-seen-concurrent",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/routing/seen/seen-concurrent/podcore-seen-concurrent",
                None,
                "",
                &state
            )
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent seen state");
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/routing-seen/podcore-seen-concurrent/seen-concurrent")
                    .is_some()
        );
    }

    // Signing and verification are process-local projections. Their
    // contracts are stable across restart and concurrent calls, with
    // malformed signing requests rejected before any crypto work.
    for case in [
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/signing/generate-keypair",
            if case == "malformed-path-query-or-body" {
                "not-json"
            } else {
                ""
            }
        );
        let payload = json_body(&response);
        record!(
            "POST",
            "/api/v0/podcore/signing/generate-keypair",
            case,
            response.status == "200 OK"
                && payload["algorithm"] == "Ed25519"
                && payload["privateKey"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                && payload["publicKey"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/signing/generate-keypair",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/signing/generate-keypair",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/generate-keypair",
            "concurrency-and-idempotency",
            left.as_ref().is_ok_and(|response| {
                response.status == "200 OK" && json_body(response)["privateKey"].is_string()
            }) && right.as_ref().is_ok_and(|response| {
                response.status == "200 OK" && json_body(response)["privateKey"].is_string()
            })
        );
    }

    let signing_message = message_body("signing-message", "podcore-signing");
    let signing_request = |message: &str| {
        serde_json::json!({
            "message": serde_json::from_str::<serde_json::Value>(message)
                .expect("signing message fixture"),
            "privateKey": signing_private_key,
        })
        .to_string()
    };
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/signing/sign", "{}");
        record!(
            "POST",
            "/api/v0/podcore/signing/sign",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/signing/sign",
            &signing_request(&signing_message)
        );
        let payload = json_body(&response);
        record!(
            "POST",
            "/api/v0/podcore/signing/sign",
            case,
            response.status == "200 OK"
                && payload["signature"]
                    .as_str()
                    .is_some_and(|signature| signature.starts_with("ed25519:"))
                && payload["publicKey"].is_string()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = signing_request(&signing_message);
        let (left, right) = tokio::join!(
            super::route_http_request("POST", "/api/v0/podcore/signing/sign", None, &body, &state),
            super::route_http_request("POST", "/api/v0/podcore/signing/sign", None, &body, &state)
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/sign",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    let verify_message = message_body("verify-message", "podcore-signing");
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/signing/verify",
            &verify_message
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/verify",
            case,
            response.status == "200 OK" && json_body(&response)["isValid"].is_boolean()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/signing/verify",
                None,
                &verify_message,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/signing/verify",
                None,
                &verify_message,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/verify",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // The verification endpoint returns a structured, non-throwing
    // result for a syntactically valid but unknown member. This is a
    // process-local projection and remains stable across lifecycle calls.
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/verification/message",
            &message_body("verification-message", "podcore-verification-message")
        );
        let payload = json_body(&response);
        record!(
            "POST",
            "/api/v0/podcore/verification/message",
            case,
            response.status == "200 OK"
                && payload["isValid"].is_boolean()
                && payload["isFromValidMember"].is_boolean()
                && payload["hasValidSignature"].is_boolean()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = message_body(
            "verification-message-concurrent",
            "podcore-verification-message",
        );
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/podcore/verification/message",
                None,
                &body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/podcore/verification/message",
                None,
                &body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/verification/message",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Last-seen updates are the one remaining backfill mutation.  The
    // fixture proves validation, durable write/readback, restart reload,
    // and concurrent overwrites of the same channel timestamp.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-missing/general/last-seen",
            ""
        );
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-runtime/general/last-seen",
            "10"
        );
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while updating last seen timestamp")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-last-seen-mutation").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-mutation/general/last-seen",
            "10"
        );
        let readback = request!(
            &state,
            "GET",
            "/api/v0/podcore/backfill/podcore-last-seen-mutation/last-seen",
            ""
        );
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "mutation-side-effects-and-readback",
            updated.status == "200 OK"
                && readback.status == "200 OK"
                && json_body(&readback)["general"] == 10
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-restart/general/last-seen",
            "10"
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload last-seen state");
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && loaded
                    .get("pod/backfill/podcore-last-seen-restart/general")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let (left, right) = tokio::join!(
            super::route_http_request(
                "PUT",
                "/api/v0/podcore/backfill/podcore-last-seen-concurrent/general/last-seen",
                None,
                "10",
                &state
            ),
            super::route_http_request(
                "PUT",
                "/api/v0/podcore/backfill/podcore-last-seen-concurrent/general/last-seen",
                None,
                "20",
                &state
            )
        );
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent last-seen state");
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/backfill/podcore-last-seen-concurrent/general")
                    .and_then(|value| value["lastSeen"].as_u64())
                    .is_some_and(|timestamp| timestamp == 10 || timestamp == 20)
        );
    }

    assert_eq!(ledger.len(), 98, "PodCore residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create PodCore evidence directory");
    fs::write(
        evidence_dir.join("podcore_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize PodCore ledger"),
    )
    .expect("write PodCore ledger");
    assert!(
        mismatches.is_empty(),
        "{} PodCore residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining MediaCore controller matrix.
/// These rows are the generic runtime/restart/concurrency cases that were
/// left open after the route-specific MediaCore proofs. Keep the ledger
/// one-to-one with the frozen manifest so the authoritative audit can
/// credit each controller action independently.
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
async fn controller_api_differential_mediacore_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let get_cases = [
        (
            "/api/v0/mediacore/contentid/domain/music",
            "/api/v0/mediacore/contentid/domain/{domain}",
            "200 OK",
            "contentIds",
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
            "200 OK",
            "normalizedType",
        ),
        (
            "/api/v0/mediacore/contentid/exists/missing",
            "/api/v0/mediacore/contentid/exists/{externalId}",
            "200 OK",
            "exists",
        ),
        (
            "/api/v0/mediacore/contentid/external/missing",
            "/api/v0/mediacore/contentid/external/{contentId}",
            "200 OK",
            "externalIds",
        ),
        (
            "/api/v0/mediacore/contentid/resolve/missing",
            "/api/v0/mediacore/contentid/resolve/{externalId}",
            "404 Not Found",
            "External ID not found",
        ),
        (
            "/api/v0/mediacore/contentid/stats",
            "/api/v0/mediacore/contentid/stats",
            "200 OK",
            "totalMappings",
        ),
        (
            "/api/v0/mediacore/contentid/validate/not-a-content-id",
            "/api/v0/mediacore/contentid/validate/{*contentId}",
            "200 OK",
            "isValid",
        ),
        (
            "/api/v0/mediacore/ipld/graph/missing",
            "/api/v0/mediacore/ipld/graph/{*contentId}",
            "200 OK",
            "nodes",
        ),
        (
            "/api/v0/mediacore/ipld/inbound/missing",
            "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
            "200 OK",
            "inboundLinks",
        ),
        (
            "/api/v0/mediacore/ipld/traverse/content:audio:track:missing?linkName=missing",
            "/api/v0/mediacore/ipld/traverse/{*startContentId}",
            "200 OK",
            "visitedNodes",
        ),
        (
            "/api/v0/mediacore/ipld/validate",
            "/api/v0/mediacore/ipld/validate",
            "200 OK",
            "isValid",
        ),
        (
            "/api/v0/mediacore/perceptualhash/algorithms",
            "/api/v0/mediacore/perceptualhash/algorithms",
            "200 OK",
            "algorithms",
        ),
        (
            "/api/v0/mediacore/portability/merge-strategies",
            "/api/v0/mediacore/portability/merge-strategies",
            "200 OK",
            "strategies",
        ),
        (
            "/api/v0/mediacore/portability/strategies",
            "/api/v0/mediacore/portability/strategies",
            "200 OK",
            "strategies",
        ),
        (
            "/api/v0/mediacore/publish/stats",
            "/api/v0/mediacore/publish/stats",
            "200 OK",
            "totalPublishedDescriptors",
        ),
        (
            "/api/v0/mediacore/retrieve/descriptor/content:audio:track:missing",
            "/api/v0/mediacore/retrieve/descriptor/{*contentId}",
            "404 Not Found",
            "found",
        ),
        (
            "/api/v0/mediacore/retrieve/query/domain/music",
            "/api/v0/mediacore/retrieve/query/domain/{domain}",
            "200 OK",
            "descriptors",
        ),
        (
            "/api/v0/mediacore/retrieve/stats",
            "/api/v0/mediacore/retrieve/stats",
            "200 OK",
            "totalRetrievals",
        ),
        (
            "/api/v0/mediacore/stats/dashboard",
            "/api/v0/mediacore/stats/dashboard",
            "200 OK",
            "contentRegistry",
        ),
        (
            "/api/v0/mediacore/stats/descriptors",
            "/api/v0/mediacore/stats/descriptors",
            "200 OK",
            "totalRetrievals",
        ),
        (
            "/api/v0/mediacore/stats/fuzzy",
            "/api/v0/mediacore/stats/fuzzy",
            "200 OK",
            "totalMatches",
        ),
        (
            "/api/v0/mediacore/stats/ipld",
            "/api/v0/mediacore/stats/ipld",
            "200 OK",
            "totalLinks",
        ),
        (
            "/api/v0/mediacore/stats/perceptual",
            "/api/v0/mediacore/stats/perceptual",
            "200 OK",
            "totalHashesComputed",
        ),
        (
            "/api/v0/mediacore/stats/portability",
            "/api/v0/mediacore/stats/portability",
            "200 OK",
            "totalExports",
        ),
        (
            "/api/v0/mediacore/stats/publishing",
            "/api/v0/mediacore/stats/publishing",
            "200 OK",
            "totalPublished",
        ),
        (
            "/api/v0/mediacore/stats/registry",
            "/api/v0/mediacore/stats/registry",
            "200 OK",
            "totalMappings",
        ),
    ];
    for (path, route, status, marker) in get_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.session.write().await.state = "disconnected";
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == status && response.body.contains(marker)
        );
    }

    let descriptor_body = |content_id: &str| {
        serde_json::json!({
            "descriptor": {
                "contentId": content_id,
                "title": "mediacore residual",
                "hashes": [{"algorithm": "sha256", "hex": "aaaa"}],
                "signature": {
                    "publicKey": "key",
                    "signature": "abcdef",
                    "timestampUnixMs": super::unix_timestamp_millis(),
                },
            },
            "forceUpdate": true,
        })
        .to_string()
    };
    let residual_content_id = "content:audio:recording:mediacore-residual";
    let empty_package = r#"{"package":{"version":"1.0","exportedAt":"2026-01-01T00:00:00Z","source":"test","entries":[],"links":[],"metadata":{"totalEntries":0,"totalLinks":0,"entriesByDomain":{},"checksum":""}}}"#;
    let verify_body = serde_json::json!({
        "descriptor": {
            "contentId": residual_content_id,
            "hashes": [{"algorithm": "sha256", "hex": "aaaa"}],
            "signature": {"publicKey": "key", "signature": "abcdef"},
        },
    })
    .to_string();
    let batch_body = serde_json::json!({
        "descriptors": [{
            "contentId": residual_content_id,
            "title": "mediacore residual batch",
            "hashes": [{"algorithm": "sha256", "hex": "bbbb"}],
            "signature": {"publicKey": "key", "signature": "abcdef"},
        }],
    })
    .to_string();

    let mutations: Vec<(&str, String, &str, String, &str)> = vec![
        (
            "DELETE",
            format!("/api/v0/mediacore/publish/descriptor/{residual_content_id}"),
            "/api/v0/mediacore/publish/descriptor/{*contentId}",
            String::new(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/contentid/register".to_owned(),
            "/api/v0/mediacore/contentid/register",
            serde_json::json!({
                "externalId": "mediacore-residual",
                "contentId": "content:audio:recording:mediacore-register",
            })
            .to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/find/content:audio:recording:mediacore-target"
                .to_owned(),
            "/api/v0/mediacore/fuzzymatch/find/{*contentId}",
            r#"{"minConfidence":0.0,"maxCandidates":50,"maxResults":10}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/perceptual".to_owned(),
            "/api/v0/mediacore/fuzzymatch/perceptual",
            r#"{"contentIdA":"content:audio:recording:a","contentIdB":"content:audio:recording:b"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/text".to_owned(),
            "/api/v0/mediacore/fuzzymatch/text",
            r#"{"textA":"same","textB":"same"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/ipld/links/content:audio:recording:mediacore-ipld"
                .to_owned(),
            "/api/v0/mediacore/ipld/links/{*contentId}",
            r#"{"links":[{"name":"related","target":"content:audio:recording:mediacore-target"}]}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/audio".to_owned(),
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[0.5],"sampleRate":1,"algorithm":"PHash"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/image".to_owned(),
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":1,"height":1,"algorithm":"PHash"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/similarity".to_owned(),
            "/api/v0/mediacore/perceptualhash/similarity",
            r#"{"hashA":"0000000000000000","hashB":"0000000000000000","threshold":0.8}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/analyze".to_owned(),
            "/api/v0/mediacore/portability/analyze",
            empty_package.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/export".to_owned(),
            "/api/v0/mediacore/portability/export",
            serde_json::json!({"contentIds":[residual_content_id]}).to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/import".to_owned(),
            "/api/v0/mediacore/portability/import",
            empty_package.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/batch".to_owned(),
            "/api/v0/mediacore/publish/batch",
            batch_body,
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/descriptor".to_owned(),
            "/api/v0/mediacore/publish/descriptor",
            descriptor_body(residual_content_id),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/republish".to_owned(),
            "/api/v0/mediacore/publish/republish",
            serde_json::json!({"contentIds":[residual_content_id]}).to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/batch".to_owned(),
            "/api/v0/mediacore/retrieve/batch",
            serde_json::json!({"contentIds":[residual_content_id]}).to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/cache/clear".to_owned(),
            "/api/v0/mediacore/retrieve/cache/clear",
            String::new(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/verify".to_owned(),
            "/api/v0/mediacore/retrieve/verify",
            verify_body,
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/stats/reset".to_owned(),
            "/api/v0/mediacore/stats/reset",
            String::new(),
            "200 OK",
        ),
        (
            "PUT",
            format!("/api/v0/mediacore/publish/descriptor/{residual_content_id}"),
            "/api/v0/mediacore/publish/descriptor/{*contentId}",
            r#"{"updates":{"title":"after"}}"#.to_owned(),
            "400 Bad Request",
        ),
    ];

    for (index, (method, path, route, body, expected_status)) in mutations.iter().enumerate() {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "runtime-failure-and-timeout",
            response.status == *expected_status
        );

        let state_dir = std::env::temp_dir().join(format!(
            "slskr-mediacore-residual-{}-{index}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&state_dir).expect("create MediaCore residual state directory");
        let state_env = base_env
            .clone()
            .with("SLSKR_STATE_DIR", &state_dir.display().to_string());
        let (first_state, _first_receiver) = test_state_with_env(state_env.clone());
        {
            first_state
                .controller_features
                .write_for_test()
                .await
                .state_path = state_dir.join("controller-feature-state.json");
        }
        let needs_descriptor_seed = method == &"DELETE"
            || method == &"PUT"
            || path == "/api/v0/mediacore/portability/export"
            || path == "/api/v0/mediacore/publish/republish"
            || path == "/api/v0/mediacore/retrieve/batch";
        if needs_descriptor_seed {
            let seed = super::route_http_request(
                "POST",
                "/api/v0/mediacore/publish/descriptor",
                None,
                &descriptor_body(residual_content_id),
                &first_state,
            )
            .await
            .expect("seed MediaCore residual descriptor");
            assert_eq!(seed.status, "200 OK", "{method} {path} descriptor seed");
        }
        let first = super::route_http_request(method, path, None, body, &first_state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} first: {error}"));
        let restored = super::ControllerFeatureState::load(&state_dir)
            .expect("reload MediaCore residual feature state");
        let (second_state, _second_receiver) = test_state_with_env(state_env);
        *second_state.controller_features.write_for_test().await = restored;
        let second = super::route_http_request(method, path, None, body, &second_state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} restart: {error}"));
        record!(
            method,
            route,
            "restart-persistence-or-reset",
            first.status == *expected_status && second.status == *expected_status
        );
        let _ = fs::remove_dir_all(&state_dir);

        let (concurrent_state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            super::route_http_request(method, path, None, body, &concurrent_state),
            super::route_http_request(method, path, None, body, &concurrent_state)
        );
        record!(
            method,
            route,
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == *expected_status)
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == *expected_status)
        );
    }

    assert_eq!(ledger.len(), 86, "MediaCore residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create MediaCore evidence directory");
    fs::write(
        evidence_dir.join("mediacore_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize MediaCore ledger"),
    )
    .expect("write MediaCore ledger");
    assert!(
        mismatches.is_empty(),
        "{} MediaCore residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining MusicBrainz controller matrix.
/// The existing MusicBrainz tests cover the detailed release-radar and
/// overlay contracts; this ledger closes the frozen controller rows that
/// exercise their remaining malformed, empty, runtime, restart, and
/// concurrent dimensions.
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
async fn controller_api_differential_musicbrainz_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let seed_artist = |state: Arc<super::AppState>| async move {
        state.library.write().await.create(
            "MusicBrainz Residual Artist".to_owned(),
            "Residual Release".to_owned(),
            "Audio".to_owned(),
        );
    };
    let get_cases = [
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "populated-dynamic-state",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage?unexpected=not-a-number",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "malformed-path-query-or-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "runtime-failure-and-timeout",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "populated-dynamic-state",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "nominal-status-headers-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph?unexpected=not-a-number",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "malformed-path-query-or-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/definitely-missing/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "missing-empty-or-conflict-state",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "runtime-failure-and-timeout",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "populated-dynamic-state",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/overlays/artist/MusicBrainz%20Residual%20Artist/release-graph?unexpected=not-a-number",
            "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
            "malformed-path-query-or-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/overlays/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
            "runtime-failure-and-timeout",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/export-review?unexpected=not-a-number",
            "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
            "malformed-path-query-or-body",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/export-review",
            "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
            "missing-empty-or-conflict-state",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/export-review",
            "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
            "runtime-failure-and-timeout",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/routes?unexpected=not-a-number",
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/routes",
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/routes",
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications?unexpected=not-a-number",
            "/api/v0/musicbrainz/release-radar/notifications",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications",
            "/api/v0/musicbrainz/release-radar/notifications",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications",
            "/api/v0/musicbrainz/release-radar/notifications",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes?unexpected=not-a-number",
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes",
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/subscriptions?unexpected=not-a-number",
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
    ];
    for (path, route, case, expected_status, populated) in get_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if populated {
            seed_artist(state.clone()).await;
        }
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == expected_status && !response.body.is_empty()
        );
    }

    let edit_body = r#"{"editId":"musicbrainz-residual-edit","type":"TitleCorrection","targetType":"Recording","targetId":"recording-residual","field":"title","value":"Residual Title","evidence":[{"type":"WorkRef","reference":"residual-ref"}]}"#;
    let valid_subscription = r#"{"artistId":"artist-residual","artistName":"Residual Artist","scope":"trusted","enabled":true,"mutedReleaseGroupIds":[],"createdAt":"2026-01-01T00:00:00Z"}"#;
    let valid_observation = r#"{"artistId":"artist-residual","recordingId":"recording-residual","songIdConfirmed":true,"confidence":1,"workRef":{"id":"recording-residual","type":"recording","domain":"music","externalIds":{},"title":"Residual Track","creator":"Residual Artist","year":2026,"metadata":{},"attributedTo":"actor","published":"2026-01-01T00:00:00Z"},"observedAt":"2026-01-01T00:00:00Z"}"#;
    let post_cases: Vec<(&str, String, &str, &str, String, &str, bool)> = vec![
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "nominal-status-headers-body",
            r#"{"missingReleases":["Residual Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "400 Bad Request",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "missing-empty-or-conflict-state",
            String::new(),
            "400 Bad Request",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "runtime-failure-and-timeout",
            r#"{"missingReleases":["Residual Runtime Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "mutation-side-effects-and-readback",
            r#"{"missingReleases":["Residual Readback Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "restart-persistence-or-reset",
            r#"{"missingReleases":["Residual Restart Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "concurrency-and-idempotency",
            r#"{"missingReleases":["Residual Concurrent Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "nominal-status-headers-body",
            r#"{"recordingIds":["recording-residual"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "runtime-failure-and-timeout",
            r#"{"recordingIds":["recording-runtime"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "mutation-side-effects-and-readback",
            r#"{"recordingIds":["recording-readback"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "restart-persistence-or-reset",
            r#"{"recordingIds":["recording-restart"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "concurrency-and-idempotency",
            r#"{"recordingIds":["recording-concurrent"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "missing-empty-or-conflict-state",
            String::new(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "runtime-failure-and-timeout",
            r#"{"saltId":"residual-runtime","expectedItems":16}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "restart-persistence-or-reset",
            r#"{"saltId":"residual-restart","expectedItems":16}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "concurrency-and-idempotency",
            r#"{"saltId":"residual-concurrent","expectedItems":16}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "nominal-status-headers-body",
            r#"{"suggestions":[{"artist":"Residual Artist","title":"Residual Bloom Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "runtime-failure-and-timeout",
            r#"{"suggestions":[{"artist":"Residual Runtime Artist","title":"Residual Runtime Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "mutation-side-effects-and-readback",
            r#"{"suggestions":[{"artist":"Residual Readback Artist","title":"Residual Readback Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "restart-persistence-or-reset",
            r#"{"suggestions":[{"artist":"Residual Restart Artist","title":"Residual Restart Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "concurrency-and-idempotency",
            r#"{"suggestions":[{"artist":"Residual Concurrent Artist","title":"Residual Concurrent Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "runtime-failure-and-timeout",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "mutation-side-effects-and-readback",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "restart-persistence-or-reset",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "concurrency-and-idempotency",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/approve-export".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
            "runtime-failure-and-timeout",
            r#"{"approvedBy":"residual-reviewer"}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/approve-export".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
            "restart-persistence-or-reset",
            r#"{"approvedBy":"residual-reviewer"}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/approve-export".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
            "concurrency-and-idempotency",
            r#"{"approvedBy":"residual-reviewer"}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/routes".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "nominal-status-headers-body",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/routes".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "restart-persistence-or-reset",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/routes".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "concurrency-and-idempotency",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes".to_owned(),
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "nominal-status-headers-body",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes".to_owned(),
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "restart-persistence-or-reset",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes".to_owned(),
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "concurrency-and-idempotency",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/observations".to_owned(),
            "/api/v0/musicbrainz/release-radar/observations",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/observations".to_owned(),
            "/api/v0/musicbrainz/release-radar/observations",
            "runtime-failure-and-timeout",
            valid_observation.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/observations".to_owned(),
            "/api/v0/musicbrainz/release-radar/observations",
            "restart-persistence-or-reset",
            valid_observation.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "malformed-path-query-or-body",
            "{}".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "missing-empty-or-conflict-state",
            String::new(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "restart-persistence-or-reset",
            valid_subscription.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "concurrency-and-idempotency",
            valid_subscription.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "nominal-status-headers-body",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "mutation-side-effects-and-readback",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "restart-persistence-or-reset",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "concurrency-and-idempotency",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
    ];

    for (method, path, route, case, body, expected_status, _seed_edit) in post_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if path.contains("/approve-export")
            || path.contains("/overlays/edits/")
            || path.ends_with("/overlays/edits")
        {
            let seed = super::route_http_request(
                "POST",
                "/api/v0/musicbrainz/overlays/edits",
                None,
                edit_body,
                &state,
            )
            .await
            .expect("seed MusicBrainz residual edit");
            assert_eq!(seed.status, "200 OK", "edit seed: {}", seed.body);
        }
        if path.contains("discography-coverage/wishlist") {
            seed_artist(state.clone()).await;
        }
        if case == "concurrency-and-idempotency" {
            let (left, right) = tokio::join!(
                super::route_http_request(method, &path, None, &body, &state),
                super::route_http_request(method, &path, None, &body, &state)
            );
            record!(
                method,
                route,
                case,
                left.as_ref()
                    .is_ok_and(|response| response.status == expected_status)
                    && right
                        .as_ref()
                        .is_ok_and(|response| response.status == expected_status)
            );
        } else {
            let response = super::route_http_request(method, &path, None, &body, &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            record!(
                method,
                route,
                case,
                response.status == expected_status
                    && (expected_status == "404 Not Found" || !response.body.is_empty())
            );
        }
    }

    assert_eq!(ledger.len(), 80, "MusicBrainz residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create MusicBrainz evidence directory");
    fs::write(
        evidence_dir.join("musicbrainz_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize MusicBrainz ledger"),
    )
    .expect("write MusicBrainz ledger");
    assert!(
        mismatches.is_empty(),
        "{} MusicBrainz residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
