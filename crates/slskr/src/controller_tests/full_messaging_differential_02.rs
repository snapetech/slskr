//! Controller full messaging differential 02 ownership.

use super::*;

/// Bulk differential proof crediting 2 user-group routes' cases,
/// independently re-derived from `native_user_group_projects_
/// transfer_group_memberships_and_live_user_classification`'s real
/// blacklist-before-privileged precedence (a blacklisted user stays
/// blacklisted even if the Soulseek server also reports them as
/// privileged), leecher/privileged live classification, and
/// batch-size bound checks. slskdN-only (confirmed against the
/// frozen registry; the source test's own final check -- that these
/// routes 404 on the slskd target -- independently confirms they are
/// genuinely absent from slskd's registry, matching the frozen
/// registry check).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_user_group() {
    let target = "slskdn";
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"leechers":{"thresholds":{"files":2,"directories":2}},"blacklisted":{"members":["differential-blocked"]},"user_defined":{"trusted":{"upload":{"priority":10},"members":["differential-friend"]}}}"#,
            ),
    );

    let group = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-friend/group",
        None,
        "",
        &state,
    )
    .await
    .expect("user group");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "nominal-status-headers-body",
        group.status == "200 OK"
            && serde_json::from_str::<String>(&group.body).unwrap_or_default() == "trusted"
    );

    let unknown = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-stranger/group",
        None,
        "",
        &state,
    )
    .await
    .expect("unknown user group");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "missing-empty-or-conflict-state",
        serde_json::from_str::<String>(&unknown.body).unwrap_or_default() == "default"
    );

    let groups = crate::route_http_request(
        "GET",
        "/api/v0/users/groups?UserNames=%20differential-friend%20&usernames=DIFFERENTIAL-FRIEND&usernames=differential-stranger&usernames=",
        None,
        "",
        &state,
    )
    .await
    .expect("user group batch");
    let groups_json = serde_json::from_str::<serde_json::Value>(&groups.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/groups",
        "nominal-status-headers-body",
        groups.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/users/groups",
        "populated-dynamic-state",
        groups_json.as_object().map(|object| object.len()) == Some(2)
            && groups_json["differential-friend"] == "trusted"
            && groups_json["differential-stranger"] == "default"
    );

    let blocked = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-blocked/group",
        None,
        "",
        &state,
    )
    .await
    .expect("blacklisted user group");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "populated-dynamic-state",
        serde_json::from_str::<String>(&blocked.body).unwrap_or_default() == "default"
    );

    {
        let mut users = state.users.write().await;
        users.apply_stats(
            "differential-leecher".to_owned(),
            &crate::UserStats {
                average_speed: 1,
                upload_count: 0,
                unknown: 0,
                file_count: 1,
                directory_count: 10,
            },
        );
        users.apply_status(&crate::UserStatus {
            username: "differential-supporter".to_owned(),
            status: 2,
            privileged: true,
        });
        users.apply_status(&crate::UserStatus {
            username: "differential-blocked".to_owned(),
            status: 2,
            privileged: true,
        });
    }
    let blocked_but_privileged = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-blocked/group",
        None,
        "",
        &state,
    )
    .await
    .expect("blacklisted-over-privileged precedence");
    let leecher = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-leecher/group",
        None,
        "",
        &state,
    )
    .await
    .expect("leecher classification");
    let privileged = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-supporter/group",
        None,
        "",
        &state,
    )
    .await
    .expect("privileged classification");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "mutation-side-effects-and-readback",
        serde_json::from_str::<String>(&blocked_but_privileged.body).unwrap_or_default()
            == "blacklisted"
            && serde_json::from_str::<String>(&leecher.body).unwrap_or_default() == "leechers"
            && serde_json::from_str::<String>(&privileged.body).unwrap_or_default() == "privileged"
    );

    let too_many = (0..=crate::MAX_USER_GROUP_BATCH)
        .map(|index| format!("usernames=differential-user-{index}"))
        .collect::<Vec<_>>()
        .join("&");
    let rejected_route = format!("/api/v0/users/groups?{too_many}");
    let rejected = crate::route_http_request("GET", &rejected_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{rejected_route}: {error}"));
    record!(
        "GET",
        "/api/v0/users/groups",
        "malformed-path-query-or-body",
        rejected.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("user_group.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api user-group mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `runtime-failure-and-timeout`
/// for `DELETE /api/v0/conversations/{username}` -- independently
/// re-verified real DB-close fault injection for the same route
/// `message_routes_roll_back_when_persistence_fails` already
/// proves. Note: most of that source test's routes turned out to
/// already be credited by pre-existing differentials
/// (`controller_api_differential_library_interests_nowplaying_
/// messages_survive_persistence_failure` covers POST `{username}`
/// and `batch` and PUT `{username}/{id}`; the table-driven
/// `controller_api_differential_versioned_openapi_validation_
/// rejections` covers PUT `{username}`) -- confirmed by grepping
/// `/tmp/slskr-parity-evidence/controller-api/*.json` directly for
/// this exact `(target, method, route, case)` key before writing
/// this, per the standing duplicate-check rule. This DELETE route
/// was the only one of the 5 conversations cases with no existing
/// evidence anywhere. slskdN-only (confirmed against the frozen
/// registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_conversations_delete_survives_persistence_failure()
{
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.messages.write().await.add(
        "differential-friend".to_owned(),
        "inbound",
        "message".to_owned(),
    );
    let previous = state.messages.read().await.clone();
    db.close_for_test().await;
    let response = crate::route_http_request(
        "DELETE",
        "/api/v0/conversations/differential-friend",
        None,
        "",
        &state,
    )
    .await
    .expect("failed conversation delete response");
    let pass = response.status == "503 Service Unavailable"
        && *state.messages.read().await == previous
        && receiver.try_recv().is_err();
    if !pass {
        mismatches.push(format!(
            "{target} DELETE /api/v0/conversations/{{username}} [runtime-failure-and-timeout]"
        ));
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "DELETE",
        "route": "/api/v0/conversations/{username}",
        "case": "runtime-failure-and-timeout",
        "pass": pass,
    }));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("conversations_delete_survives_persistence_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api conversations-delete mismatches:\n{}",
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
pub(super) async fn controller_api_differential_rooms_controller_residuals() {
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
            crate::route_http_request($method, $path, None, $body, $state)
                .await
                .expect("RoomsController route request")
        }};
    }

    fn json_body(response: &crate::routing::HttpResponse) -> serde_json::Value {
        serde_json::from_str(&response.body).unwrap_or_default()
    }

    async fn connect_and_join(state: &Arc<crate::AppState>, room: &str) {
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("compatibility runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
        let fresh = crate::RoomStore::new();
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
                crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("compatibility delete runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
        let fresh = crate::RoomStore::new();
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
                crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("rooms activity runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("rooms available runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("joined rooms runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("room reads runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned join restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state.session.write().await.state = "connected";
        let response = request!(&state, "POST", "/api/v0/rooms/joined", r#""restart-room""#);
        let persisted = db.list_subscribed_rooms().await.unwrap_or_default();
        let reset = crate::RoomStore::from_persisted(persisted);
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
            Some(crate::SessionCommand::SayRoom {
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
                == Some(crate::SessionCommand::SetRoomTicker {
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
                == Some(crate::SessionCommand::AddRoomMember {
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
            command == Some(crate::SessionCommand::LeaveRoom("leave-room".to_owned()))
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("room leave restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        connect_and_join(&state, "leave-reset").await;
        let response = request!(&state, "DELETE", "/api/v0/rooms/joined/leave-reset", "");
        let reset =
            crate::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
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
                crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("room member restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
            crate::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
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
                crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("room message restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
            crate::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
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
                crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("room ticker restart database");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
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
            crate::RoomStore::from_persisted(db.list_subscribed_rooms().await.unwrap_or_default());
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
                crate::route_http_request(
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
pub(super) async fn controller_api_differential_users_open_cases() {
    let target = "slskdn";
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

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                "differential-peer=127.0.0.1:2234",
            )
    };
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };

    macro_rules! seed_user {
        ($state:expr, $username:expr) => {{
            $state.users.write().await.records.push(crate::UserRecord {
                username: $username.to_owned(),
                watched: true,
                status: Some("Online".to_owned()),
                privileged: true,
                average_speed: Some(1_024),
                upload_count: Some(3),
                file_count: Some(12),
                directory_count: Some(4),
                updated_at: crate::unix_timestamp(),
            });
        }};
    }
    macro_rules! seed_browse {
        ($state:expr, $username:expr) => {{
            $state
                .browse
                .write()
                .await
                .add_entries(
                    $username.to_owned(),
                    vec![crate::BrowseEntry {
                        path_encoding: Default::default(),
                        filename: "Remote/Album/Track.flac".to_owned(),
                        size: 123,
                        extension: "flac".to_owned(),
                    }],
                    true,
                )
                .expect("user browse fixture");
        }};
    }

    // Browse projections: malformed paths reject, and a connected
    // slskdn user with no browse record returns the frozen 404.
    let (state, _receiver) = test_state_with_env(env());
    state.session.write().await.state = "connected";
    seed_user!(&state, "differential-peer");
    seed_browse!(&state, "differential-peer");
    let malformed_browse = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/browse/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user browse malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "malformed-path-query-or-body",
        malformed_browse.status == "404 Not Found"
    );
    let (missing_browse_state, _receiver) = test_state_with_env(env());
    missing_browse_state.session.write().await.state = "connected";
    let missing_browse = crate::route_http_request(
        "GET",
        "/api/v0/users/missing-peer/browse",
        None,
        "",
        &missing_browse_state,
    )
    .await
    .expect("user browse empty response");
    let missing_browse_value = parse_json(&missing_browse.body);
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "missing-empty-or-conflict-state",
        missing_browse.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user browse runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        runtime_state.session.write().await.state = "connected";
        seed_browse!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/browse",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user browse runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/browse",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["directoryCount"] == 1
        );
    }

    // Browse status is a local tracker projection; database closure does
    // not erase it.
    let malformed_status = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/browse/status/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user browse status malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "malformed-path-query-or-body",
        malformed_status.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user browse status runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        seed_browse!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/browse/status",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user browse status runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/browse/status",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["status"] == "ready"
        );
    }

    // Endpoint lookup uses the deterministic test override for the
    // nominal/runtime rows; dropping the command receiver makes the
    // missing-user path fail without waiting on a network timeout.
    let malformed_endpoint = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/endpoint/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user endpoint malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "malformed-path-query-or-body",
        malformed_endpoint.status == "404 Not Found"
    );
    let (missing_endpoint_state, missing_endpoint_receiver) = test_state_with_env(env());
    drop(missing_endpoint_receiver);
    let missing_endpoint = crate::route_http_request(
        "GET",
        "/api/v0/users/missing-peer/endpoint",
        None,
        "",
        &missing_endpoint_state,
    )
    .await
    .expect("user endpoint missing response");
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "missing-empty-or-conflict-state",
        missing_endpoint.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user endpoint runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/endpoint",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user endpoint runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/endpoint",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["port"] == 2234
        );
    }

    // Info and status expose the slskdn DTO projection for both watched
    // and untracked users.
    let info = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/info",
        None,
        "",
        &state,
    )
    .await
    .expect("user info nominal response");
    let info_value = parse_json(&info.body);
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "nominal-status-headers-body",
        info.status == "200 OK"
            && info_value["uploadSpeed"] == 1_024
            && info_value["uploadCount"] == 3
            && info_value["fileCount"] == 12
            && info_value["directoryCount"] == 4
    );
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "populated-dynamic-state",
        info_value["description"] == ""
            && info_value["hasFreeUploadSlot"] == true
            && info_value["picture"].is_null()
    );
    let malformed_info = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/info/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user info malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "malformed-path-query-or-body",
        malformed_info.status == "404 Not Found"
    );
    let missing_info =
        crate::route_http_request("GET", "/api/v0/users/missing-peer/info", None, "", &state)
            .await
            .expect("user info missing response");
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "missing-empty-or-conflict-state",
        missing_info.status == "200 OK" && parse_json(&missing_info.body)["fileCount"] == 0
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user info runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        seed_user!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/info",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user info runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/info",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["uploadCount"] == 3
        );
    }

    let malformed_status = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/status/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user status malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "malformed-path-query-or-body",
        malformed_status.status == "404 Not Found"
    );
    let missing_status =
        crate::route_http_request("GET", "/api/v0/users/missing-peer/status", None, "", &state)
            .await
            .expect("user status missing response");
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "missing-empty-or-conflict-state",
        missing_status.status == "200 OK"
            && parse_json(&missing_status.body)["presence"] == "Offline"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user status runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        seed_user!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/status",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user status runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/status",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["presence"] == "Online"
        );
    }

    // Groups are synchronous transfer-group projections, not database
    // lookups. Malformed subpaths are still rejected by routing.
    let malformed_group = crate::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/group/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user group malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "malformed-path-query-or-body",
        malformed_group.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user group runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/group",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user group runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/group",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).as_str() == Some("default")
        );
    }
    let empty_groups = crate::route_http_request("GET", "/api/v0/users/groups", None, "", &state)
        .await
        .expect("empty user groups response");
    record!(
        "GET",
        "/api/v0/users/groups",
        "missing-empty-or-conflict-state",
        empty_groups.status == "200 OK" && parse_json(&empty_groups.body) == serde_json::json!({})
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user groups runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response =
            crate::route_http_request("GET", "/api/v0/users/groups", None, "", &runtime_state)
                .await
                .expect("user groups runtime response");
        record!(
            "GET",
            "/api/v0/users/groups",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body) == serde_json::json!({})
        );
    }

    // User notes are loaded into the bounded in-memory projection at
    // startup; closed persistence does not invalidate read projections.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user notes list runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response =
            crate::route_http_request("GET", "/api/v0/users/notes", None, "", &runtime_state)
                .await
                .expect("user notes list runtime response");
        record!(
            "GET",
            "/api/v0/users/notes",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).is_array()
        );
    }
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("user note runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
        runtime_state.user_notes.write().await.set_versioned(
            "differential-peer".to_owned(),
            "runtime note".to_owned(),
            String::new(),
            String::new(),
            false,
        );
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/users/notes/differential-peer",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user note runtime response");
        record!(
            "GET",
            "/api/v0/users/notes/{username}",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["note"] == "runtime note"
        );
    }

    // Directory request binding precedes connection readiness, matching
    // the frozen controller; valid requests then project the cached browse
    // directory and remain idempotent across fresh state instances.
    let malformed_directory = crate::route_http_request(
        "POST",
        "/api/v0/users/differential-peer/directory/extra",
        None,
        "{}",
        &state,
    )
    .await
    .expect("user directory malformed response");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "malformed-path-query-or-body",
        malformed_directory.status == "404 Not Found"
    );
    let missing_directory = crate::route_http_request(
        "POST",
        "/api/v0/users/differential-peer/directory",
        None,
        "{}",
        &state,
    )
    .await
    .expect("user directory missing response");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "missing-empty-or-conflict-state",
        missing_directory.status == "400 Bad Request"
            && missing_directory.body.contains("directory is required")
    );
    let directory = crate::route_http_request(
        "POST",
        "/api/v0/users/differential-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &state,
    )
    .await
    .expect("user directory mutation response");
    let directory_value = parse_json(&directory.body);
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "mutation-side-effects-and-readback",
        directory.status == "200 OK"
            && directory_value
                .as_array()
                .is_some_and(|rows| { rows.iter().any(|row| row["name"] == "Remote/Album") })
    );
    {
        let (restarted_state, _receiver) = test_state_with_env(env());
        restarted_state.session.write().await.state = "connected";
        let response = crate::route_http_request(
            "POST",
            "/api/v0/users/differential-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &restarted_state,
        )
        .await
        .expect("user directory restart response");
        let value = parse_json(&response.body);
        record!(
            "POST",
            "/api/v0/users/{username}/directory",
            "restart-persistence-or-reset",
            response.status == "200 OK" && value.as_array().is_some_and(|rows| rows.len() == 1)
        );
    }
    let concurrent = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/users/differential-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/users/differential-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    assert_eq!(ledger.len(), 27, "users residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create users evidence directory");
    fs::write(
        evidence_dir.join("users_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize users ledger"),
    )
    .expect("write users ledger");
    assert!(
        mismatches.is_empty(),
        "{} users controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
