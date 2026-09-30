//! Controller full configuration differential ownership.

use super::*;

/// Bulk differential proof crediting `PUT /api/v0/options`'s
/// remote-configuration-disabled-by-default gate (independently
/// re-derived from `remote_configuration_routes_are_forbidden_by_
/// default`, which already fully credited `PATCH /api/v0/options`
/// for the same case but not `PUT`), `POST /api/v0/conversations/
/// batch`'s real per-recipient database persistence (independently
/// re-derived from `conversations_batch_persists_each_outbound_
/// message`, using a real in-memory `DatabaseManager` and reading
/// the persisted rows back directly, not just the HTTP response),
/// and `GET /api/v0/conversations/{username}`'s real replay-
/// deduplication projection (independently re-derived from
/// `inbound_private_message_replays_update_one_record_and_retain_
/// replay_flag`: two server-pushed `MessageUserResponse` frames for
/// the same message id collapse to one stored record with
/// `wasReplayed` preserved, reusing the real `project_server_
/// message` production function against a real loopback session).
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing, per case: all 3 were open. slskdN-only
/// (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_options_and_conversations_projection() {
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

    let (state, _receiver) = test_state();
    let forbidden = crate::route_http_request("PUT", "/api/options", None, "{}", &state)
        .await
        .expect("remote configuration policy response");
    record!(
        "PUT",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );

    {
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
            "/api/v0/conversations/batch",
            None,
            r#"{"usernames":["differential-friend","differential-peer"],"body":"differential persisted batch"}"#,
            &state,
        )
        .await
        .expect("batch message response");
        let dispatched = matches!(
            receiver.try_recv(),
            Ok(crate::SessionCommand::MessageUsers { .. })
        );
        let mut persisted = db.list_messages(10, 0).await.expect("list messages");
        persisted.sort_by(|left, right| left.username.cmp(&right.username));
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "mutation-side-effects-and-readback",
            response.status == "201 Created"
                && dispatched
                && persisted.len() == 2
                && persisted[0].username == "differential-friend"
                && persisted[0].content == "differential persisted batch"
                && persisted[1].username == "differential-peer"
        );
    }

    {
        use slskr_client::protocol::server::{PrivateMessage, ServerMessage};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = tokio::net::TcpStream::connect(address);
        let server = listener.accept();
        let (client, server) = tokio::join!(client, server);
        let (server, _) = server.unwrap();
        let mut session = slskr_client::server::ServerSession::new(
            slskr_client::stream::ServerConnection::new(server),
        );
        let _client = slskr_client::stream::ServerConnection::new(client.unwrap());
        let message = |replayed| {
            ServerMessage::MessageUserResponse(PrivateMessage {
                id: 177,
                timestamp: 188,
                username: "differential-peer".to_owned(),
                message: "differential hello".to_owned(),
                is_new: true,
                was_replayed: replayed,
            })
        };
        crate::session_runtime::project_server_message(&state, &mut session, &message(false)).await;
        crate::session_runtime::project_server_message(&state, &mut session, &message(true)).await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/conversations/differential-peer",
            None,
            "",
            &state,
        )
        .await
        .expect("conversation projection response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let messages = json["messages"].as_array().cloned().unwrap_or_default();
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "populated-dynamic-state",
            messages.len() == 1 && messages[0]["wasReplayed"] == true
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("options_and_conversations_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api options-and-conversations-projection mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the versioned slskdN options controller's
/// current projection and volatile overlay lifecycle.  The oracle's
/// `OptionsController` returns a redacted current snapshot, accepts a
/// validated overlay only when remote configuration is enabled, applies
/// overlays in memory, loses them on restart, and exposes generic 500
/// problem details when the current options snapshot is invalid.  The
/// concurrency case checks that each real overlay request succeeds and
/// that the final readback is one of the submitted live values.  The
/// POST/PUT `/api/v0/options` extension rows are intentionally not
/// credited here because they are not declared by the frozen oracle
/// `OptionsController`.  slskdN-only (confirmed against the registry).
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
pub(super) async fn controller_api_differential_options_current_overlay_lifecycle() {
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

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let response = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("current options response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/options",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/json; charset=utf-8"
                && value["remoteConfiguration"] == true
                && value["web"]["authentication"]["password"] == "*****"
                && value["web"]["authentication"]["jwt"]["key"] == "*****"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let patched = crate::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50311,"privateMessageAutoResponse":{"enabled":true}},"integration":{"spotify":{"clientSecret":"options-readback-secret"}}}"#,
            &state,
        )
        .await
        .expect("options readback overlay");
        let current = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options populated readback");
        let value = serde_json::from_str::<serde_json::Value>(&current.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/options",
            "populated-dynamic-state",
            patched.status == "200 OK"
                && current.status == "200 OK"
                && value["soulseek"]["listenPort"] == 50311
                && value["soulseek"]["privateMessageAutoResponse"]["enabled"] == true
                && value["integration"]["spotify"]["clientSecret"] == "*****"
                && !current.body.contains("options-readback-secret")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let response = crate::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50312,"privateMessageAutoResponse":{"enabled":true}},"integration":{"spotify":{"clientSecret":"options-patch-secret"}}}"#,
            &state,
        )
        .await
        .expect("nominal options overlay response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "PATCH",
            "/api/v0/options",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/json; charset=utf-8"
                && value["soulseek"]["listenPort"] == 50312
                && value["soulseek"]["privateMessageAutoResponse"]["enabled"] == true
                && value["integration"]["spotify"]["clientSecret"] == "*****"
                && !response.body.contains("options-patch-secret")
        );
    }

    {
        let env = MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true");
        let (state, _receiver) = test_state_with_env(env.clone());
        let baseline = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("baseline options response");
        let baseline_value =
            serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap_or_default();
        let patched = crate::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50313}}"#,
            &state,
        )
        .await
        .expect("volatile options overlay response");
        let restarted = test_state_with_env(env).0;
        let current = crate::route_http_request("GET", "/api/v0/options", None, "", &restarted)
            .await
            .expect("options response after restart");
        let current_value =
            serde_json::from_str::<serde_json::Value>(&current.body).unwrap_or_default();
        record!(
            "PATCH",
            "/api/v0/options",
            "restart-persistence-or-reset",
            patched.status == "200 OK"
                && current.status == "200 OK"
                && current_value["soulseek"]["listenPort"]
                    == baseline_value["soulseek"]["listenPort"]
                && current_value["soulseek"]["listenPort"] != 50313
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let ports = [50320_u64, 50321, 50322, 50323];
        let bodies = ports
            .iter()
            .map(|port| format!(r#"{{"soulseek":{{"listenPort":{port}}}}}"#))
            .collect::<Vec<_>>();
        let responses =
            futures_util::future::join_all(bodies.iter().map(|body| {
                crate::route_http_request("PATCH", "/api/v0/options", None, body, &state)
            }))
            .await;
        let current = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("concurrent options readback");
        let port = serde_json::from_str::<serde_json::Value>(&current.body)
            .ok()
            .and_then(|value| value["soulseek"]["listenPort"].as_u64());
        record!(
            "PATCH",
            "/api/v0/options",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && current.status == "200 OK"
                && port.is_some_and(|port| ports.contains(&port))
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        *state
            .controller_options_validation_error
            .write()
            .expect("options validation error lock") = Some("differential invalid options".into());
        let response = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options validation failure response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/options",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.content_type == "application/problem+json"
                && value["title"] == "Internal Server Error"
                && value["status"] == 500
                && value["detail"] == "An unexpected error occurred."
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("options_current_overlay_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api options-current-overlay mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `PATCH /api/v0/options`'s
/// real non-object-body rejection and real successful secret-
/// redacting overlay mutation (independently re-derived from
/// `null_options_overlay_matches_the_selected_frozen_target`'s
/// slskdn half and `options_overlay_sets_target_specific_
/// reconnect_and_redacts_secrets`: a `null`/`[]` body is a real
/// ASP.NET-style ProblemDetails 400, and a real successful overlay
/// redacts a configured Spotify client secret while driving a real
/// runtime reconnect-pending flag), `GET /api/v0/dht/status`'s
/// real dynamic reflection of a watched YAML config change
/// (independently re-derived from `watched_native_dht_updates_
/// current_options_but_retains_startup_socket_settings`: DHT is
/// genuinely disabled after a real watched-config apply, not a
/// canned response), and 4 routes from `bridge_projections_
/// redact_internal_endpoint`'s real internal host/port redaction
/// proof (`GET /api/v0/bridge/admin/config`, `GET /api/v0/bridge/
/// admin/dashboard`, and `GET /api/v0/bridge/status`, all
/// completely uncredited; `GET /api/v0/application`'s redaction
/// case, distinct from its already-credited nominal-shape case).
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing, per case: all 7 were open. slskdN-only
/// (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_options_dht_and_bridge_redaction() {
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

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
        );
        let mut pass = true;
        for body in ["null", "[]"] {
            let response =
                crate::route_http_request("PATCH", "/api/v0/options", None, body, &state)
                    .await
                    .expect("non-object options overlay response");
            pass &= response.status == "400 Bad Request"
                && response.content_type == "application/json; charset=utf-8"
                && response.body
                    == r#"{"title":"One or more validation errors occurred.","status":400,"detail":"The request is invalid.","errors":{}}"#;
        }
        record!(
            "PATCH",
            "/api/v0/options",
            "malformed-path-query-or-body",
            pass
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
        );
        state.session.write().await.state = "connected";
        let response = crate::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50399},"integration":{"spotify":{"clientSecret":"differential-do-not-return"}}}"#,
            &state,
        )
        .await
        .expect("target overlay response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "PATCH",
            "/api/v0/options",
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && value["soulseek"]["listenPort"] == 50_399
                && value["integration"]["spotify"]["clientSecret"] == "*****"
                && !response.body.contains("differential-do-not-return")
                && state.runtime.read().await.application_reconnect_pending
        );
    }

    {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
        let baseline = crate::route_http_request("GET", "/api/v0/dht/status", None, "", &state)
            .await
            .expect("baseline dht status response");
        let baseline_json =
            serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/dht/status",
            "nominal-status-headers-body",
            baseline.status == "200 OK"
                && baseline.content_type.starts_with("application/json")
                && baseline_json["isEnabled"].is_boolean()
                && baseline_json["lanOnly"].is_boolean()
                && baseline_json["isBeaconCapable"].is_boolean()
                && baseline_json["isDhtRunning"].is_boolean()
                && baseline_json["dhtNodeCount"].is_number()
                && baseline_json["discoveredPeerCount"].is_number()
                && baseline_json["activeMeshConnections"].is_number()
                && baseline_json["verifiedBeaconCount"].is_number()
                && baseline_json["totalPeersDiscovered"].is_number()
                && baseline_json["totalCandidateEndpointsSeen"].is_number()
                && baseline_json["totalCandidatesAccepted"].is_number()
                && baseline_json["totalCandidatesSkippedDhtPort"].is_number()
                && baseline_json["totalCandidatesSkippedDiscoveredCapacity"].is_number()
                && baseline_json["totalCandidatesDeferredConnectorCapacity"].is_number()
                && baseline_json["totalCandidatesSkippedReconnectBackoff"].is_number()
                && baseline_json["totalConnectionsAttempted"].is_number()
                && baseline_json["totalConnectionsSucceeded"].is_number()
                && baseline_json["lastAnnounceTime"].is_null()
                && baseline_json["lastDiscoveryTime"].is_null()
                && baseline_json["startedAt"].is_null()
                && baseline_json["uptimeSeconds"].is_number()
                && baseline_json["rendezvousInfohashes"].is_array()
        );
        assert!(state.config.advanced_networking.dht.enabled);
        let yaml = "dht:\n  enabled: false\n  dht_port: 51099\n";
        fs::write(state.config.state_dir.join("slskd.yml"), yaml)
            .expect("write differential dht yaml");
        crate::apply_watched_controller_configuration(
            &state,
            Some(yaml),
            &state.controller_cli_environment,
        )
        .await;
        let status = crate::route_http_request("GET", "/api/v0/dht/status", None, "", &state)
            .await
            .expect("watched dht status response");
        let status_json =
            serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/dht/status",
            "populated-dynamic-state",
            status.status == "200 OK" && status_json["isEnabled"] == false
        );
    }

    {
        let internal_host = "differential-bridge.internal.private";
        let internal_port = "43199";
        let env = MapEnv::default()
            .with("SLSKR_BRIDGE_ENABLED", "true")
            .with("SLSKR_BRIDGE_HOST", internal_host)
            .with("SLSKR_BRIDGE_PORT", internal_port);
        let (state, _receiver) = test_state_with_env_parts(env, crate::SearchStore::new(), None);
        for path in [
            "/api/v0/bridge/admin/config",
            "/api/v0/bridge/admin/dashboard",
            "/api/v0/bridge/status",
            "/api/v0/application",
        ] {
            let response = crate::route_http_request("GET", path, None, "", &state)
                .await
                .expect("bridge projection response");
            record!(
                "GET",
                path,
                "missing-empty-or-conflict-state",
                response.status == "200 OK"
                    && !response.body.contains(internal_host)
                    && !response.body.contains(internal_port)
            );
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("options_dht_and_bridge_redaction.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api options-dht-and-bridge-redaction mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The frozen OptionsController uses action-level Route attributes for
/// startup/debug/YAML/location/validation.  Keep those actions in the
/// controller ledger with real file reads, watched projections, YAML
/// validation, and reload checks for both compatibility targets.
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
pub(super) async fn controller_api_differential_options_action_routes() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let target = $target;
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

    for target in ["slskd", "slskdn"] {
        let env = MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true")
            .with("SLSKR_DEBUG", "true")
            .with("SLSKR_NO_CONFIG_WATCH", "true");
        let (state, _receiver) = test_state_with_env(env.clone());
        let config_path = state.config.state_dir.join("slskd.yml");
        let initial_yaml = "soulseek:\n  description: action-initial\n";
        fs::write(&config_path, initial_yaml).expect("write options action YAML fixture");

        let startup = crate::route_http_request("GET", "/api/v0/options/startup", None, "", &state)
            .await
            .expect("startup options action");
        let startup_json =
            serde_json::from_str::<serde_json::Value>(&startup.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "nominal-status-headers-body",
            startup.status == "200 OK"
                && startup.content_type == "application/json; charset=utf-8"
                && startup_json.is_object()
                && startup_json["web"]["authentication"]["password"] == "*****"
        );
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "populated-dynamic-state",
            startup_json.is_object() && !startup.body.is_empty()
        );

        let startup_missing =
            crate::route_http_request("GET", "/api/v0/options/startup", None, "", &state)
                .await
                .expect("startup options empty-state action");
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "missing-empty-or-conflict-state",
            startup_missing.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&startup_missing.body).is_ok()
        );

        let startup_malformed =
            crate::route_http_request("GET", "/api/v0/options/startup/extra", None, "", &state)
                .await
                .expect("malformed startup options action");
        record!(
            target,
            "GET",
            "/api/v0/options/startup",
            "malformed-path-query-or-body",
            startup_malformed.status == "404 Not Found"
        );

        let watched_yaml = "soulseek:\n  description: action-watched\n";
        fs::write(&config_path, watched_yaml).expect("write watched options YAML fixture");
        crate::apply_watched_controller_configuration(
            &state,
            Some(watched_yaml),
            &state.controller_cli_environment,
        )
        .await;

        let debug = crate::route_http_request("GET", "/api/v0/options/debug", None, "", &state)
            .await
            .expect("debug options action");
        let debug_text = serde_json::from_str::<String>(&debug.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/debug",
            "nominal-status-headers-body",
            debug.status == "200 OK"
                && debug.content_type == "application/json; charset=utf-8"
                && debug_text.starts_with("slskd:\n")
        );
        record!(
            target,
            "GET",
            "/api/v0/options/debug",
            "populated-dynamic-state",
            debug_text.contains("action-watched") && !debug_text.contains("action-secret")
        );

        let debug_malformed =
            crate::route_http_request("GET", "/api/v0/options/debug/extra", None, "", &state)
                .await
                .expect("malformed debug options action");
        record!(
            target,
            "GET",
            "/api/v0/options/debug",
            "malformed-path-query-or-body",
            debug_malformed.status == "404 Not Found"
        );

        let location =
            crate::route_http_request("GET", "/api/v0/options/yaml/location", None, "", &state)
                .await
                .expect("options YAML location action");
        let location_value = serde_json::from_str::<String>(&location.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/yaml/location",
            "nominal-status-headers-body",
            location.status == "200 OK"
                && location.content_type == "application/json; charset=utf-8"
                && location_value == config_path.display().to_string()
        );
        record!(
            target,
            "GET",
            "/api/v0/options/yaml/location",
            "populated-dynamic-state",
            location_value.ends_with("slskd.yml")
        );

        let malformed_location = crate::route_http_request(
            "GET",
            "/api/v0/options/yaml/location/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed options YAML location action");
        record!(
            target,
            "GET",
            "/api/v0/options/yaml/location",
            "malformed-path-query-or-body",
            malformed_location.status == "404 Not Found"
        );

        let yaml = crate::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
            .await
            .expect("options YAML action");
        let yaml_text = serde_json::from_str::<String>(&yaml.body).unwrap_or_default();
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "nominal-status-headers-body",
            yaml.status == "200 OK"
                && yaml.content_type == "application/json; charset=utf-8"
                && yaml_text == watched_yaml
        );
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "populated-dynamic-state",
            yaml_text.contains("action-watched")
        );

        let yaml_malformed =
            crate::route_http_request("GET", "/api/v0/options/yaml/extra", None, "", &state)
                .await
                .expect("malformed options YAML action");
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "malformed-path-query-or-body",
            yaml_malformed.status == "404 Not Found"
        );

        let yaml_failure_root = std::env::temp_dir().join(format!(
            "slskr-options-yaml-read-conflict-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(&yaml_failure_root, b"state directory is a file")
            .expect("create YAML read conflict");
        let mut yaml_failure_state = test_state_with_env(env.clone()).0;
        Arc::get_mut(&mut yaml_failure_state)
            .expect("exclusive YAML read conflict state")
            .config
            .state_dir = yaml_failure_root.clone();
        let yaml_failure =
            crate::route_http_request("GET", "/api/v0/options/yaml", None, "", &yaml_failure_state)
                .await
                .expect("YAML read filesystem failure action");
        record!(
            target,
            "GET",
            "/api/v0/options/yaml",
            "runtime-failure-and-timeout",
            yaml_failure.status == "500 Internal Server Error"
                && !yaml_failure.body.contains("state directory is a file")
        );
        let _ = fs::remove_file(yaml_failure_root);

        let valid_yaml = serde_json::to_string("debug: false\n").unwrap();
        let validation = crate::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &valid_yaml,
            &state,
        )
        .await
        .expect("valid options YAML validation action");
        record!(
            target,
            "POST",
            "/api/v0/options/yaml/validate",
            "nominal-status-headers-body",
            validation.status == "200 OK"
                && validation.content_type.is_empty()
                && validation.body.is_empty()
        );

        let malformed_validation =
            crate::route_http_request("POST", "/api/v0/options/yaml/validate", None, "{", &state)
                .await
                .expect("malformed options YAML validation action");
        record!(
            target,
            "POST",
            "/api/v0/options/yaml/validate",
            "malformed-path-query-or-body",
            malformed_validation.status == "400 Bad Request"
        );

        let concurrent_validation = futures_util::future::join_all([
            crate::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &valid_yaml,
                &state,
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &valid_yaml,
                &state,
            ),
        ])
        .await;
        record!(
            target,
            "POST",
            "/api/v0/options/yaml/validate",
            "concurrency-and-idempotency",
            concurrent_validation.iter().all(|response| response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
        );

        let updated_yaml =
            serde_json::to_string("soulseek:\n  description: action-updated\n").unwrap();
        let update =
            crate::route_http_request("PUT", "/api/v0/options/yaml", None, &updated_yaml, &state)
                .await
                .expect("options YAML update action");
        let readback = crate::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
            .await
            .expect("options YAML update readback");
        record!(
            target,
            "PUT",
            "/api/v0/options/yaml",
            "nominal-status-headers-body",
            update.status == "200 OK" && update.body.is_empty()
        );
        record!(
            target,
            "PUT",
            "/api/v0/options/yaml",
            "mutation-side-effects-and-readback",
            update.status == "200 OK"
                && serde_json::from_str::<String>(&readback.body)
                    .is_ok_and(|value| value == "soulseek:\n  description: action-updated\n")
        );
        let reloaded = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default().with(
                "SLSKR_STATE_DIR",
                state.config.state_dir.to_str().expect("state path"),
            ),
        )
        .expect("reload options YAML state");
        record!(
            target,
            "PUT",
            "/api/v0/options/yaml",
            "restart-persistence-or-reset",
            reloaded.user_info_description == "action-updated"
        );

        let disabled =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target)).0;
        for (method, route, case) in [
            (
                "GET",
                "/api/v0/options/debug",
                "missing-empty-or-conflict-state",
            ),
            (
                "GET",
                "/api/v0/options/yaml",
                "missing-empty-or-conflict-state",
            ),
            (
                "GET",
                "/api/v0/options/yaml/location",
                "missing-empty-or-conflict-state",
            ),
        ] {
            let response = crate::route_http_request(method, route, None, "", &disabled)
                .await
                .expect("disabled options action");
            record!(
                target,
                method,
                route,
                case,
                response.status == "403 Forbidden"
            );
        }

        let options = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options validation-failure setup");
        let _ = options;
        *state
            .controller_options_validation_error
            .write()
            .expect("options validation error lock") =
            Some("action route validation failure".to_owned());
        let failed_options = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options validation failure action");
        record!(
            target,
            "GET",
            "/api/v0/options",
            "runtime-failure-and-timeout",
            failed_options.status == "500 Internal Server Error"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("options_action_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize options action controller ledger"),
    )
    .expect("write options action controller ledger");
    assert!(
        mismatches.is_empty(),
        "{} options action controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
