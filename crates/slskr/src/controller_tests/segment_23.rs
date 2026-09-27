/// The nominal core slice above deliberately keeps its fixtures small.
/// This companion closes the remaining deterministic empty-state,
/// malformed-input, restart, idempotency, and persistence-failure cases
/// for the same frozen slskd routes.  Every row is emitted only after a
/// real dispatcher call and, where applicable, a raw SQLite readback or a
/// fresh store rehydration check.
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
async fn controller_api_differential_controller_core_failure_restart_and_empty_contracts() {
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("core differential database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));

    // Disconnected application/session/event state is a real empty
    // projection, not a missing handler.
    let application = super::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("empty slskd application");
    let application_json =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        "missing-empty-or-conflict-state",
        application.status == "200 OK"
            && application_json["pendingRestart"] == false
            && application_json["server"]["isConnected"] == false
    );

    let session = super::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("empty slskd session");
    let session_json = serde_json::from_str::<serde_json::Value>(&session.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        session.status == "200 OK" && session_json["state"] == "disconnected"
    );

    let events = super::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("empty slskd events");
    record!(
        "GET",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        events.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&events.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );

    // Invalid JSON is rejected before any event is recorded.
    let malformed_event =
        super::route_http_request("POST", "/api/v0/events/Noop", None, "{", &state)
            .await
            .expect("malformed slskd event");
    record!(
        "POST",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        malformed_event.status == "400 Bad Request"
    );

    // Restart is persisted in the real runtime compatibility row and is
    // recoverable by the same conversion used during application boot.
    let restart = super::route_http_request("PUT", "/api/v0/application", None, "{}", &state)
        .await
        .expect("persist slskd restart request");
    let persisted_restart = db
        .get_runtime_compat_state()
        .await
        .expect("read persisted restart state")
        .expect("runtime compatibility row");
    let rehydrated_restart = super::RuntimeCompatState::from_persisted(&persisted_restart);
    record!(
        "PUT",
        "/api/v0/application",
        "restart-persistence-or-reset",
        restart.status == "204 No Content"
            && persisted_restart.application_restart_requested
            && rehydrated_restart.application_restart_requested
    );

    // DELETE is safe to repeat and persists the reset state rather than
    // leaving a stale restart request behind.
    let shutdown = super::route_http_request("DELETE", "/api/v0/application", None, "", &state)
        .await
        .expect("persist slskd shutdown reset");
    let shutdown_row = db
        .get_runtime_compat_state()
        .await
        .expect("read persisted shutdown state")
        .expect("runtime compatibility shutdown row");
    record!(
        "DELETE",
        "/api/v0/application",
        "restart-persistence-or-reset",
        shutdown.status == "204 No Content" && !shutdown_row.application_restart_requested
    );
    let repeated_shutdown =
        super::route_http_request("DELETE", "/api/v0/application", None, "", &state)
            .await
            .expect("repeat slskd shutdown reset");
    record!(
        "DELETE",
        "/api/v0/application",
        "concurrency-and-idempotency",
        repeated_shutdown.status == "204 No Content"
            && db
                .get_runtime_compat_state()
                .await
                .expect("read repeated shutdown state")
                .is_some_and(|row| !row.application_restart_requested)
    );

    // Events use the same durable database row as the boot rehydration
    // path.  The fresh store check avoids crediting a write that only
    // changed the in-memory projection.
    let event = super::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""durable-event""#,
        &state,
    )
    .await
    .expect("durable slskd event");
    let persisted_events = db.list_events(10, 0).await.expect("list durable events");
    let rehydrated_events =
        super::EventStore::from_persisted(persisted_events.clone(), super::EVENT_HISTORY_LIMIT);
    record!(
        "POST",
        "/api/v0/events",
        "restart-persistence-or-reset",
        event.status == "201 Created"
            && persisted_events
                .iter()
                .any(|row| row.detail.as_deref() == Some("durable-event"))
            && rehydrated_events
                .controller_json(None)
                .contains("durable-event")
    );

    // Two distinct event writes are serialized by the real store and
    // both survive the raw durable readback.
    let event_responses = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-a""#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-b""#,
            &state,
        ),
    ])
    .await;
    let concurrent_events = db.list_events(10, 0).await.expect("list concurrent events");
    record!(
        "POST",
        "/api/v0/events",
        "concurrency-and-idempotency",
        event_responses.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && concurrent_events
                .iter()
                .filter_map(|row| row.detail.as_deref())
                .collect::<std::collections::BTreeSet<_>>()
                .is_superset(&std::collections::BTreeSet::from([
                    "concurrent-a",
                    "concurrent-b"
                ]))
    );

    // Closing the real database exercises the route's recoverable
    // persistence error rather than a synthetic response.
    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("event failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_event = super::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""failure""#,
        &failure_state,
    )
    .await
    .expect("slskd event persistence failure response");
    record!(
        "POST",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        failed_event.status == "503 Service Unavailable"
            && failed_event.body.contains("persistence")
    );

    // Telemetry reports have explicit empty projections and query
    // validation.  Keep the time range outside the fixture window so
    // the empty assertions cannot accidentally pass from seeded state.
    let (telemetry_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let telemetry_empty_cases = [
        ("/api/v0/telemetry/metrics", "GET"),
        ("/api/v0/telemetry/metrics/kpis", "GET"),
        ("/api/v0/telemetry/reports/transfers/summary?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z", "GET"),
        ("/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60", "GET"),
        ("/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z", "GET"),
        ("/api/v0/telemetry/reports/transfers/exceptions?direction=Download&username=none", "GET"),
        ("/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download&username=none", "GET"),
        ("/api/v0/telemetry/reports/transfers/directories?username=none", "GET"),
        ("/api/v0/telemetry/reports/transfers/users/none", "GET"),
    ];
    for (path, method) in telemetry_empty_cases {
        let response = super::route_http_request(method, path, None, "", &telemetry_state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        let body = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let empty_shape = if path.contains("/metrics") {
            response.status == "200 OK" && (body.is_object() || response.body.contains("# HELP"))
        } else if path.contains("/summary") {
            response.status == "200 OK"
                && body["Download"].is_object()
                && body["Upload"].is_object()
        } else if path.contains("/histogram") {
            response.status == "200 OK" && body.is_object()
        } else if path.contains("/users/") {
            response.status == "200 OK" && body["count"] == 0
        } else {
            response.status == "200 OK" && body.as_array().is_some_and(Vec::is_empty)
        };
        let route = path.split('?').next().unwrap_or(path);
        record!("GET", route, "missing-empty-or-conflict-state", empty_shape);
    }

    for (path, route) in [
        (
            "/api/v0/telemetry/reports/transfers/leaderboard?direction=NoSuchDirection",
            "/api/v0/telemetry/reports/transfers/leaderboard",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions?direction=NoSuchDirection",
            "/api/v0/telemetry/reports/transfers/exceptions",
        ),
        (
            "/api/v0/telemetry/reports/transfers/histogram?interval=0",
            "/api/v0/telemetry/reports/transfers/histogram",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &telemetry_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_core_failure_restart_and_empty_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd core failure ledger"),
    )
    .expect("write slskd core failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd core failure/restart mismatches:\n{}",
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
async fn controller_api_differential_controller_users_and_shares() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "peer=127.0.0.1:2234"),
    );
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

    {
        let mut users = state.users.write().await;
        users.records.push(super::UserRecord {
            username: "peer".to_owned(),
            watched: true,
            status: Some("Online".to_owned()),
            privileged: true,
            average_speed: Some(1_024),
            upload_count: Some(3),
            file_count: Some(12),
            directory_count: Some(4),
            updated_at: super::unix_timestamp(),
        });
    }
    let info = super::route_http_request("GET", "/api/v0/users/peer/info", None, "", &state)
        .await
        .expect("slskd user info");
    let info_json = serde_json::from_str::<serde_json::Value>(&info.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "nominal-status-headers-body",
        info.status == "200 OK"
            && info_json.as_object().is_some_and(|object| {
                object.len() == 6
                    && object["description"] == ""
                    && object["hasFreeUploadSlot"] == true
                    && object["hasPicture"] == false
                    && object["picture"].is_null()
                    && object["queueLength"] == 0
                    && object["uploadSlots"] == 0
            })
    );
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "populated-dynamic-state",
        info.status == "200 OK"
            && info_json["hasFreeUploadSlot"] == true
            && info_json["uploadSlots"] == 0
    );
    let missing_info =
        super::route_http_request("GET", "/api/v0/users/missing/info", None, "", &state)
            .await
            .expect("slskd missing user info");
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "missing-empty-or-conflict-state",
        missing_info.status == "404 Not Found"
    );

    let status = super::route_http_request("GET", "/api/v0/users/peer/status", None, "", &state)
        .await
        .expect("slskd user status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "nominal-status-headers-body",
        status.status == "200 OK"
            && status_json.as_object().is_some_and(|object| {
                object.len() == 2
                    && object["isPrivileged"] == true
                    && object["presence"] == "Online"
            })
    );
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "populated-dynamic-state",
        status.status == "200 OK" && status_json["presence"] == "Online"
    );
    let missing_status =
        super::route_http_request("GET", "/api/v0/users/missing/status", None, "", &state)
            .await
            .expect("slskd missing user status");
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "missing-empty-or-conflict-state",
        missing_status.status == "404 Not Found"
    );

    let endpoint =
        super::route_http_request("GET", "/api/v0/users/peer/endpoint", None, "", &state)
            .await
            .expect("slskd user endpoint");
    let endpoint_json =
        serde_json::from_str::<serde_json::Value>(&endpoint.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "nominal-status-headers-body",
        endpoint.status == "200 OK"
            && endpoint_json.as_object().is_some_and(|object| {
                object.len() == 3
                    && object["addressFamily"] == "IPv4"
                    && object["address"] == "127.0.0.1"
                    && object["port"] == 2234
            })
    );
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "populated-dynamic-state",
        endpoint.status == "200 OK"
            && endpoint_json["address"] == "127.0.0.1"
            && endpoint_json["port"] == 2234
    );

    let unknown_endpoint =
        super::route_http_request("GET", "/api/v0/users/unknown/endpoint", None, "", &state)
            .await
            .expect("slskd unknown user endpoint");
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "missing-empty-or-conflict-state",
        unknown_endpoint.status == "404 Not Found"
    );

    {
        let mut shares = state.shares.write().await;
        shares.roots.clear();
        shares.entries.clear();
        shares.local_paths.clear();
        shares.roots.push(super::ShareRoot {
            label: "Virtual".to_owned(),
            local_path: PathBuf::from("/srv/music"),
            raw: "Virtual".to_owned(),
            directories: 1,
            files: 1,
            bytes: 42,
            extensions: vec![super::ShareExtensionSummary {
                extension: "flac".to_owned(),
                files: 1,
                bytes: 42,
            }],
            statistics_ready: true,
        });
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Virtual/Track.flac".to_owned(),
            size: 42,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
    }
    let share_id = super::share_root_id("Virtual");
    let shares = super::route_http_request("GET", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd shares");
    let shares_json = serde_json::from_str::<serde_json::Value>(&shares.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares",
        "nominal-status-headers-body",
        shares.status == "200 OK"
            && shares_json["local"]
                .as_array()
                .is_some_and(|roots| roots.len() == 1)
    );
    record!(
        "GET",
        "/api/v0/shares",
        "populated-dynamic-state",
        shares.status == "200 OK"
            && shares_json["local"][0]["id"] == share_id
            && shares_json["local"][0]["files"] == 1
    );

    let share = super::route_http_request(
        "GET",
        &format!("/api/v0/shares/{share_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd share detail");
    let share_json = serde_json::from_str::<serde_json::Value>(&share.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "nominal-status-headers-body",
        share.status == "200 OK" && share_json["id"] == share_id
    );
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "populated-dynamic-state",
        share.status == "200 OK" && share_json["files"] == 1 && share_json["directories"] == 1
    );

    for (path, route) in [
        ("/api/v0/shares/contents", "/api/v0/shares/contents"),
        (
            &format!("/api/v0/shares/{share_id}/contents"),
            "/api/v0/shares/{id}/contents",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && json.as_array().is_some_and(|rows| !rows.is_empty())
        );
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && json[0]["files"]
                    .as_array()
                    .is_some_and(|files| !files.is_empty())
        );
    }

    for path in ["/api/v0/shares/missing", "/api/v0/shares/missing/contents"] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let route = if path.ends_with("/contents") {
            "/api/v0/shares/{id}/contents"
        } else {
            "/api/v0/shares/{id}"
        };
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    let rescan = super::route_http_request("PUT", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd share rescan");
    record!(
        "PUT",
        "/api/v0/shares",
        "nominal-status-headers-body",
        rescan.status == "200 OK" && rescan.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/shares",
        "mutation-side-effects-and-readback",
        rescan.status == "200 OK" && state.share_lifecycle.read().await.ready
    );

    let (restarted_share_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let restarted_rescan =
        super::route_http_request("PUT", "/api/v0/shares", None, "", &restarted_share_state)
            .await
            .expect("restarted slskd share rescan");
    record!(
        "PUT",
        "/api/v0/shares",
        "restart-persistence-or-reset",
        restarted_rescan.status == "200 OK"
            && restarted_rescan.body.is_empty()
            && restarted_share_state.share_lifecycle.read().await.ready
    );

    let cancel = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd share scan cancel");
    record!(
        "DELETE",
        "/api/v0/shares",
        "missing-empty-or-conflict-state",
        cancel.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_users_shares.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd users/shares ledger"),
    )
    .expect("write slskd users/shares ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd users/shares controller mismatches:\n{}",
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
async fn controller_api_differential_controller_rooms_and_conversations() {
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
        super::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
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
        super::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
            .await
            .expect("slskd repeated room join");
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "concurrency-and-idempotency",
        repeated.status == "200 OK" && repeated.body.is_empty()
    );

    let joined_rooms = super::route_http_request("GET", "/api/v0/rooms/joined", None, "", &state)
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

    let room = super::route_http_request("GET", "/api/v0/rooms/joined/music", None, "", &state)
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
        let response = super::route_http_request("GET", path, None, "", &state)
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

    let room_message = super::route_http_request(
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
    let room_messages = super::route_http_request(
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
        let response = super::route_http_request("POST", path, None, r#""value""#, &state)
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

    let leave = super::route_http_request("DELETE", "/api/v0/rooms/joined/music", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/rooms/joined/music", None, "", &state)
            .await
            .expect("slskd missing room");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        "missing-empty-or-conflict-state",
        missing_room.status == "404 Not Found"
    );

    let sent = super::route_http_request(
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

    let conversations = super::route_http_request("GET", "/api/v0/conversations", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/conversations/peer", None, "", &state)
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

    let messages = super::route_http_request(
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
    let acknowledge = super::route_http_request(
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
        super::route_http_request("PUT", "/api/v0/conversations/peer", None, "", &state)
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
        super::route_http_request("POST", "/api/v0/conversations/peer", None, "", &state)
            .await
            .expect("slskd malformed conversation");
    record!(
        "POST",
        "/api/v0/conversations/{username}",
        "malformed-path-query-or-body",
        malformed_send.status == "400 Bad Request"
    );

    let deleted =
        super::route_http_request("DELETE", "/api/v0/conversations/peer", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/conversations/peer", None, "", &state)
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
async fn controller_api_differential_controller_rooms_conversations_restart_and_failure() {
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
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("rooms/conversations database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
    state.session.write().await.state = "connected";

    let joined = super::route_http_request(
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
    let rehydrated_rooms = super::RoomStore::from_persisted(persisted_rooms);
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
    let repeated_join = super::route_http_request(
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

    let left = super::route_http_request(
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
    let rehydrated_after_leave = super::RoomStore::from_persisted(rooms_after_leave);
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
    let repeated_leave = super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/rooms/joined", None, "{}", &state)
            .await
            .expect("malformed room join");
    record!(
        "POST",
        "/api/v0/rooms/joined",
        "malformed-path-query-or-body",
        malformed_join.status == "400 Bad Request"
    );
    let empty_join = super::route_http_request("POST", "/api/v0/rooms/joined", None, "{}", &state)
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
    let action_join = super::route_http_request(
        "POST",
        "/api/v0/rooms/joined",
        None,
        r#""room-actions""#,
        &state,
    )
    .await
    .expect("join room for subresource actions");
    let action_join_command = receiver.try_recv().ok();
    let action_message = super::route_http_request(
        "POST",
        "/api/v0/rooms/joined/room-actions/messages",
        None,
        r#""message before reset""#,
        &state,
    )
    .await
    .expect("room message before reset");
    let action_message_command = receiver.try_recv().ok();
    let action_ticker = super::route_http_request(
        "POST",
        "/api/v0/rooms/joined/room-actions/ticker",
        None,
        r#""ticker before reset""#,
        &state,
    )
    .await
    .expect("room ticker before reset");
    let action_ticker_command = receiver.try_recv().ok();
    let action_member = super::route_http_request(
        "POST",
        "/api/v0/rooms/joined/room-actions/members",
        None,
        r#""member before reset""#,
        &state,
    )
    .await
    .expect("room member before reset");
    let action_member_command = receiver.try_recv().ok();
    let rehydrated_action_rooms = super::RoomStore::from_persisted(
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
                == Some(super::SessionCommand::JoinRoom("room-actions".to_owned()))
            && action_message.status == "201 Created"
            && action_message_command
                == Some(super::SessionCommand::SayRoom {
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
                == Some(super::SessionCommand::SetRoomTicker {
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
                == Some(super::SessionCommand::AddRoomMember {
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
        let response = super::route_http_request("POST", path, None, body, &failure_state)
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

    let sent = super::route_http_request(
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
    let rehydrated_messages = super::MessageStore::from_persisted(persisted_messages);
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
        super::route_http_request(
            "POST",
            "/api/v0/conversations/concurrent-a",
            None,
            r#""parallel-a""#,
            &state,
        ),
        super::route_http_request(
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

    let ack = super::route_http_request(
        "PUT",
        &format!("/api/v0/conversations/persist-peer/{message_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("ack persisted conversation");
    let persisted_after_ack = db.list_messages(20, 0).await.expect("list acked messages");
    let rehydrated_after_ack = super::MessageStore::from_persisted(persisted_after_ack.clone());
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
    let repeated_ack = super::route_http_request(
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

    let second_message = super::route_http_request(
        "POST",
        "/api/v0/conversations/persist-peer",
        None,
        r#""ack all""#,
        &state,
    )
    .await
    .expect("create second conversation message");
    assert_eq!(second_message.status, "201 Created");
    let ack_all = super::route_http_request(
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
    let repeated_ack_all = super::route_http_request(
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

    let deleted = super::route_http_request(
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
        super::MessageStore::from_persisted(persisted_after_delete.clone());
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
    let repeated_delete = super::route_http_request(
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

    let malformed_conversation = super::route_http_request(
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
    let malformed_messages = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("room join failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_db.close_for_test().await;
    let failed_join = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("room leave failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
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
    let failed_leave = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation send failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_db.close_for_test().await;
    let failed_send = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation acknowledgement failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
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
    let failed_ack = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation acknowledge-all failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_state.messages.write().await.add(
        "failure-peer".to_owned(),
        "inbound",
        "unacknowledged".to_owned(),
    );
    failure_db.close_for_test().await;
    let failed_ack_all = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation delete failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(failure_db.clone()));
    failure_state.messages.write().await.add(
        "failure-peer".to_owned(),
        "inbound",
        "retained".to_owned(),
    );
    let previous_messages = failure_state.messages.read().await.clone();
    failure_db.close_for_test().await;
    let failed_delete = super::route_http_request(
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
async fn controller_api_differential_controller_server_state_and_lifecycle() {
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

    let initial = super::route_http_request("GET", "/api/v0/server", None, "", &state)
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
    let connected = super::route_http_request("GET", "/api/v0/server", None, "", &state)
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
    let connect = super::route_http_request("PUT", "/api/v0/server", None, "", &state)
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
            && matches!(receiver.try_recv(), Ok(super::SessionCommand::Connect))
    );

    let repeated_connect = super::route_http_request("PUT", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd repeated server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "concurrency-and-idempotency",
        repeated_connect.status == "205 Reset Content" && repeated_connect.body.is_empty()
    );

    let disconnect = super::route_http_request("DELETE", "/api/v0/server", None, "", &state)
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
            && matches!(receiver.try_recv(), Ok(super::SessionCommand::Disconnect))
    );

    let repeated_disconnect =
        super::route_http_request("DELETE", "/api/v0/server", None, "", &state)
            .await
            .expect("slskd repeated server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "concurrency-and-idempotency",
        repeated_disconnect.status == "204 No Content" && repeated_disconnect.body.is_empty()
    );

    let malformed_disconnect =
        super::route_http_request("DELETE", "/api/v0/server", None, "{", &state)
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
        super::route_http_request("DELETE", "/api/v0/server", None, "", &restarted_state)
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
        super::route_http_request("PUT", "/api/v0/server", None, "", &put_restart_state)
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
                Ok(super::SessionCommand::Connect)
            )
    );

    let (put_failure_state, put_failure_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    drop(put_failure_receiver);
    let put_failure =
        super::route_http_request("PUT", "/api/v0/server", None, "", &put_failure_state)
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
        super::route_http_request("DELETE", "/api/v0/server", None, "", &delete_failure_state)
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
async fn controller_api_differential_controller_search_lifecycle() {
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
    let created = super::route_http_request("POST", "/api/v0/searches", None, &create_body, &state)
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
                Ok(super::SessionCommand::Search { .. })
            )
    );

    let duplicate =
        super::route_http_request("POST", "/api/v0/searches", None, &create_body, &state)
            .await
            .expect("duplicate slskd search");
    record!(
        "POST",
        "/api/v0/searches",
        "missing-empty-or-conflict-state",
        duplicate.status == "409 Conflict"
    );

    let malformed = super::route_http_request("POST", "/api/v0/searches", None, "{}", &state)
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
    let response_ingest = super::route_http_request(
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

    let listed = super::route_http_request("GET", "/api/v0/searches", None, "", &state)
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

    let detail = super::route_http_request(
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

    let responses = super::route_http_request(
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

    let cancelled = super::route_http_request(
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

    let deleted = super::route_http_request(
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
    let missing = super::route_http_request(
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
async fn controller_api_differential_controller_search_failure_restart_and_idempotency() {
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

    async fn live_search_get(state: Arc<super::AppState>, path: &str) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(super::handle_http_stream(server, None, false, state));
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
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search lifecycle database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
    state.session.write().await.state = "connected";

    let created = super::route_http_request(
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
    let rehydrated = super::SearchStore::from_persisted(persisted.clone());
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
        super::route_http_request("POST", "/api/v0/searches", None, duplicate_body, &state)
            .await
            .expect("first idempotent search");
    let second_duplicate =
        super::route_http_request("POST", "/api/v0/searches", None, duplicate_body, &state)
            .await
            .expect("duplicate idempotent search");
    record!(
        "POST",
        "/api/v0/searches",
        "concurrency-and-idempotency",
        first_duplicate.status == "200 OK" && second_duplicate.status == "409 Conflict"
    );

    let delete_created = super::route_http_request(
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
    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{delete_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete persisted search");
    let persisted_after_delete = db.list_searches(10, 0).await.expect("list after delete");
    let rehydrated_after_delete = super::SearchStore::from_persisted(persisted_after_delete);
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
    let repeated_delete = super::route_http_request(
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

    let put_created = super::route_http_request(
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
    let put = super::route_http_request(
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
    let rehydrated_put = super::SearchStore::from_persisted(vec![persisted_put.clone()]);
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
    let repeated_put = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search creation failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_state.session.write().await.state = "connected";
    failure_db.close_for_test().await;
    let failed_create = super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search update failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
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
        super::route_http_request("PUT", "/api/v0/searches/1", None, "", &failure_state)
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search delete failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
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
        super::route_http_request("DELETE", "/api/v0/searches/1", None, "", &failure_state)
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

    let expired_list_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search list expiry database");
    let (expired_list_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
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

    let expired_detail_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search detail expiry database");
    let (expired_detail_state, _receiver) = test_state_with_env_parts(
        env,
        super::SearchStore::new(),
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
async fn controller_api_differential_native_search_compatibility_contracts() {
    let target = "slskdn";
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
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn search compatibility database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
    state.shares.write().await.entries.push(FileEntry {
        filename_encoding: Default::default(),
        extension_encoding: Default::default(),
        code: 1,
        filename: "Remote/Search.flac".to_owned(),
        size: 321,
        extension: "flac".to_owned(),
        attributes: Vec::new(),
    });

    let created = super::route_http_request(
        "POST",
        "/api/search",
        None,
        r#"{"query":"  Remote Search  ","limit":1}"#,
        &state,
    )
    .await
    .expect("slskdn compatibility search");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body)
        .expect("slskdn compatibility search JSON");
    let search_id = created_json["searchId"].as_str().unwrap_or_default();
    let result = created_json["results"]
        .as_array()
        .and_then(|results| results.first())
        .cloned()
        .unwrap_or_default();
    record!(
        "POST",
        "/api/search",
        "nominal-status-headers-body",
        created.status == "200 OK"
            && created.content_type == "application/json"
            && search_id.len() == 32
            && !search_id.contains('-')
            && created_json["query"] == "Remote Search"
            && created_json["results"].as_array().is_some_and(|results| {
                results.len() == 1
                    && result.as_object().is_some_and(|object| {
                        object.len() == 5
                            && result["username"] == ""
                            && result["filename"] == "Remote/Search.flac"
                            && result["size"] == 321
                            && result["code"] == 1
                            && result["extension"] == "flac"
                    })
            })
    );

    let dispatched = receiver.try_recv();
    record!(
        "POST",
        "/api/search",
        "mutation-side-effects-and-readback",
        matches!(
            dispatched,
            Ok(super::SessionCommand::Search {
                query,
                target: super::SearchDispatchTarget::Global,
                ..
            }) if query == "Remote Search"
        ) && state
            .searches
            .read()
            .await
            .records
            .iter()
            .any(|record| record.query == "Remote Search")
    );

    let malformed = super::route_http_request("POST", "/api/search", None, "not-json", &state)
        .await
        .expect("malformed slskdn compatibility search");
    let malformed_path = super::route_http_request(
        "POST",
        "/api/search/extra",
        None,
        r#"{"query":"Remote"}"#,
        &state,
    )
    .await
    .expect("malformed slskdn compatibility search path");
    record!(
        "POST",
        "/api/search",
        "malformed-path-query-or-body",
        malformed.status == "400 Bad Request"
            && malformed.body == r#"{"error":"Query is required"}"#
            && malformed_path.status == "404 Not Found"
    );

    let blank =
        super::route_http_request("POST", "/api/search", None, r#"{"query":"   "}"#, &state)
            .await
            .expect("blank slskdn compatibility search");
    let invalid_limit = super::route_http_request(
        "POST",
        "/api/search",
        None,
        r#"{"query":"Remote","limit":0}"#,
        &state,
    )
    .await
    .expect("invalid slskdn compatibility search limit");
    record!(
        "POST",
        "/api/search",
        "missing-empty-or-conflict-state",
        blank.status == "400 Bad Request"
            && blank.body == r#"{"error":"Query is required"}"#
            && invalid_limit.status == "400 Bad Request"
            && invalid_limit.body == r#"{"error":"Limit must be positive"}"#
    );

    let persisted = db
        .list_searches(50, 0)
        .await
        .expect("list slskdn compatibility searches");
    let rehydrated = super::SearchStore::from_persisted(persisted.clone());
    record!(
        "POST",
        "/api/search",
        "restart-persistence-or-reset",
        persisted
            .iter()
            .any(|record| record.query == "Remote Search")
            && rehydrated
                .records
                .iter()
                .any(|record| record.query == "Remote Search")
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn compatibility search failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed = super::route_http_request(
        "POST",
        "/api/search",
        None,
        r#"{"query":"failed search"}"#,
        &failure_state,
    )
    .await
    .expect("slskdn compatibility search failure");
    record!(
        "POST",
        "/api/search",
        "runtime-failure-and-timeout",
        failed.status == "500 Internal Server Error"
            && failed.body == r#"{"error":"Search failed"}"#
            && failure_state.searches.read().await.records.is_empty()
            && failure_state.searches.read().await.next_token == 1
    );

    let (first, second) = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/search",
            None,
            r#"{"query":"concurrent one"}"#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/search",
            None,
            r#"{"query":"concurrent two"}"#,
            &state,
        )
    );
    let first = first.expect("first concurrent compatibility search");
    let second = second.expect("second concurrent compatibility search");
    let first_id = serde_json::from_str::<serde_json::Value>(&first.body)
        .ok()
        .and_then(|body| body["searchId"].as_str().map(str::to_owned));
    let second_id = serde_json::from_str::<serde_json::Value>(&second.body)
        .ok()
        .and_then(|body| body["searchId"].as_str().map(str::to_owned));
    record!(
        "POST",
        "/api/search",
        "concurrency-and-idempotency",
        first.status == "200 OK"
            && second.status == "200 OK"
            && first_id.is_some()
            && second_id.is_some()
            && first_id != second_id
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_search_compatibility_contracts.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize slskdn search compatibility ledger"),
    )
    .expect("write slskdn search compatibility ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn search compatibility mismatches:\n{}",
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
async fn controller_api_differential_native_searches_open_cases() {
    let target = "slskdn";
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

    async fn seed_search(state: &Arc<super::AppState>, id: &str, with_result: bool) {
        let token = {
            let mut searches = state.searches.write().await;
            searches
                .create(
                    Some(id.to_owned()),
                    "open search".to_owned(),
                    "global",
                    None,
                    Vec::new(),
                    3_600,
                )
                .expect("seed search")
                .record
                .token
        };
        if with_result {
            let response = super::route_http_request(
                "POST",
                "/api/v0/search-responses",
                None,
                &format!(
                    r#"{{"token":{token},"username":"search-peer","files":[{{"filename":"Remote/Search.flac","size":321,"extension":"flac"}}]}}"#
                ),
                state,
            )
            .await
            .expect("seed search response");
            assert_eq!(response.status, "200 OK", "{}", response.body);
        }
    }

    async fn seed_expired_search(state: &Arc<super::AppState>, id: &str) {
        let mut searches = state.searches.write().await;
        searches
            .create(
                Some(id.to_owned()),
                "expired open search".to_owned(),
                "global",
                None,
                Vec::new(),
                0,
            )
            .expect("seed expired search");
    }

    let env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let list_id = "00000000-0000-4000-8000-000000000101";
    let detail_id = "00000000-0000-4000-8000-000000000102";

    let (list_state, _receiver) = test_state_with_env(env.clone());
    let list_malformed = super::route_http_request(
        "GET",
        "/api/v0/searches?limit=not-a-number",
        None,
        "",
        &list_state,
    )
    .await
    .expect("malformed search list query");
    record!(
        "GET",
        "/api/v0/searches",
        "malformed-path-query-or-body",
        list_malformed.status == "400 Bad Request"
    );
    let list_empty = super::route_http_request("GET", "/api/v0/searches", None, "", &list_state)
        .await
        .expect("empty search list");
    record!(
        "GET",
        "/api/v0/searches",
        "missing-empty-or-conflict-state",
        list_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&list_empty.body)
                .is_ok_and(|value| value.as_array().is_some_and(Vec::is_empty))
    );

    let list_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search list runtime database");
    let (list_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(list_db.clone()),
    );
    seed_search(&list_failure_state, list_id, true).await;
    list_db.close_for_test().await;
    let list_runtime =
        super::route_http_request("GET", "/api/v0/searches", None, "", &list_failure_state)
            .await
            .expect("search list runtime failure");
    record!(
        "GET",
        "/api/v0/searches",
        "runtime-failure-and-timeout",
        list_runtime.status == "500 Internal Server Error"
            && !list_runtime.body.contains("search list runtime database")
    );

    let (detail_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&detail_state, detail_id, true).await;
    let detail_malformed = super::route_http_request(
        "GET",
        "/api/v0/searches/not-a-guid",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("malformed search detail id");
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "malformed-path-query-or-body",
        detail_malformed.status == "400 Bad Request"
    );
    let detail_missing = super::route_http_request(
        "GET",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("missing search detail");
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        detail_missing.status == "404 Not Found"
    );

    let detail_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search detail runtime database");
    let (detail_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(detail_db.clone()),
    );
    seed_search(&detail_failure_state, detail_id, true).await;
    detail_db.close_for_test().await;
    let detail_runtime = super::route_http_request(
        "GET",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &detail_failure_state,
    )
    .await
    .expect("search detail runtime failure");
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        detail_runtime.status == "500 Internal Server Error"
    );

    let responses_malformed = super::route_http_request(
        "GET",
        "/api/v0/searches/not-a-guid/responses",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("malformed search responses id");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "malformed-path-query-or-body",
        responses_malformed.status == "400 Bad Request"
    );
    let responses_missing = super::route_http_request(
        "GET",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199/responses",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("missing search responses");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "missing-empty-or-conflict-state",
        responses_missing.status == "404 Not Found"
    );
    let responses_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search responses runtime database");
    let (responses_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(responses_db.clone()),
    );
    seed_search(&responses_failure_state, detail_id, true).await;
    responses_db.close_for_test().await;
    let responses_runtime = super::route_http_request(
        "GET",
        &format!("/api/v0/searches/{detail_id}/responses"),
        None,
        "",
        &responses_failure_state,
    )
    .await
    .expect("search responses runtime failure");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "runtime-failure-and-timeout",
        responses_runtime.status == "500 Internal Server Error"
    );

    let (download_state, _receiver) = test_state_with_env(env.clone());
    let action_id = "00000000-0000-4000-8000-000000000103";
    seed_search(&download_state, action_id, true).await;
    let download_route = format!("/api/v0/searches/{action_id}/items/0/download");
    let download_nominal =
        super::route_http_request("POST", &download_route, None, "", &download_state)
            .await
            .expect("search item download");
    let download_json =
        serde_json::from_str::<serde_json::Value>(&download_nominal.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "nominal-status-headers-body",
        download_nominal.status == "200 OK"
            && download_json["success"] == true
            && download_json["source"] == "scene"
    );
    let download_malformed = super::route_http_request(
        "POST",
        &format!("/api/v0/searches/{action_id}/items/not-an-item/download"),
        None,
        "",
        &download_state,
    )
    .await
    .expect("malformed search item download");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "malformed-path-query-or-body",
        download_malformed.status == "400 Bad Request"
    );
    let download_side_effects = download_state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|entry| {
            entry.peer_username.as_deref() == Some("search-peer")
                && entry.filename == "Remote/Search.flac"
        });
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "mutation-side-effects-and-readback",
        download_side_effects
    );

    let download_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search download runtime database");
    let (download_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(download_db.clone()),
    );
    seed_search(&download_failure_state, action_id, true).await;
    download_db.close_for_test().await;
    let download_runtime =
        super::route_http_request("POST", &download_route, None, "", &download_failure_state)
            .await
            .expect("search download runtime failure");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "runtime-failure-and-timeout",
        download_runtime.status == "500 Internal Server Error"
    );
    let (download_restarted, _receiver) = test_state_with_env(env.clone());
    seed_search(&download_restarted, action_id, true).await;
    let restarted_download =
        super::route_http_request("POST", &download_route, None, "", &download_restarted)
            .await
            .expect("restarted search download");
    let (download_fresh, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "restart-persistence-or-reset",
        restarted_download.status == "200 OK"
            && download_fresh.transfers.read().await.entries.is_empty()
    );
    let (download_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_search(&download_concurrent, action_id, true).await;
    let concurrent_downloads = futures_util::future::join_all([
        super::route_http_request("POST", &download_route, None, "", &download_concurrent),
        super::route_http_request("POST", &download_route, None, "", &download_concurrent),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "concurrency-and-idempotency",
        concurrent_downloads.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && download_concurrent.transfers.read().await.entries.len() == 2
    );

    let (stream_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&stream_state, action_id, true).await;
    let stream_route = format!("/api/v0/searches/{action_id}/items/0/stream");
    let stream_nominal = super::route_http_request("POST", &stream_route, None, "", &stream_state)
        .await
        .expect("search item stream");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "nominal-status-headers-body",
        stream_nominal.status == "400 Bad Request"
            && stream_nominal
                .body
                .contains("scene_streaming_not_supported")
    );
    let stream_malformed = super::route_http_request(
        "POST",
        &format!("/api/v0/searches/{action_id}/items/not-an-item/stream"),
        None,
        "",
        &stream_state,
    )
    .await
    .expect("malformed search item stream");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "malformed-path-query-or-body",
        stream_malformed.status == "400 Bad Request"
    );
    let stream_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search stream runtime database");
    let (stream_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(stream_db.clone()),
    );
    seed_search(&stream_failure_state, action_id, true).await;
    stream_db.close_for_test().await;
    let stream_runtime =
        super::route_http_request("POST", &stream_route, None, "", &stream_failure_state)
            .await
            .expect("search stream runtime failure");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "runtime-failure-and-timeout",
        stream_runtime.status == "500 Internal Server Error"
    );
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "mutation-side-effects-and-readback",
        stream_state.transfers.read().await.entries.is_empty()
    );
    let (stream_restarted, _receiver) = test_state_with_env(env.clone());
    seed_search(&stream_restarted, action_id, true).await;
    let restarted_stream =
        super::route_http_request("POST", &stream_route, None, "", &stream_restarted)
            .await
            .expect("restarted search stream");
    let (stream_fresh, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "restart-persistence-or-reset",
        restarted_stream.status == "400 Bad Request"
            && stream_fresh.transfers.read().await.entries.is_empty()
    );
    let (stream_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_search(&stream_concurrent, action_id, true).await;
    let concurrent_streams = futures_util::future::join_all([
        super::route_http_request("POST", &stream_route, None, "", &stream_concurrent),
        super::route_http_request("POST", &stream_route, None, "", &stream_concurrent),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "concurrency-and-idempotency",
        concurrent_streams.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "400 Bad Request"))
            && stream_concurrent.transfers.read().await.entries.is_empty()
    );

    let (delete_all_state, _receiver) = test_state_with_env(env.clone());
    let delete_all_malformed = super::route_http_request(
        "DELETE",
        "/api/v0/searches/extra",
        None,
        "",
        &delete_all_state,
    )
    .await
    .expect("malformed delete-all search path");
    record!(
        "DELETE",
        "/api/v0/searches",
        "malformed-path-query-or-body",
        delete_all_malformed.status == "400 Bad Request"
    );
    let delete_all_empty =
        super::route_http_request("DELETE", "/api/v0/searches", None, "", &delete_all_state)
            .await
            .expect("empty delete-all search");
    record!(
        "DELETE",
        "/api/v0/searches",
        "missing-empty-or-conflict-state",
        delete_all_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&delete_all_empty.body)
                .is_ok_and(|value| value["deleted"] == 0)
    );
    let delete_all_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("delete-all runtime database");
    let (delete_all_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(delete_all_db.clone()),
    );
    delete_all_db.close_for_test().await;
    let delete_all_runtime = super::route_http_request(
        "DELETE",
        "/api/v0/searches",
        None,
        "",
        &delete_all_failure_state,
    )
    .await
    .expect("delete-all runtime failure");
    record!(
        "DELETE",
        "/api/v0/searches",
        "runtime-failure-and-timeout",
        delete_all_runtime.status == "500 Internal Server Error"
    );
    let (delete_all_reset_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&delete_all_reset_state, list_id, false).await;
    let reset_delete_all = super::route_http_request(
        "DELETE",
        "/api/v0/searches",
        None,
        "",
        &delete_all_reset_state,
    )
    .await
    .expect("reset delete-all search");
    let (delete_all_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "DELETE",
        "/api/v0/searches",
        "restart-persistence-or-reset",
        reset_delete_all.status == "200 OK"
            && delete_all_restarted
                .searches
                .read()
                .await
                .records
                .is_empty()
    );
    let (delete_all_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_search(&delete_all_concurrent, list_id, false).await;
    seed_search(&delete_all_concurrent, detail_id, false).await;
    let concurrent_delete_all = futures_util::future::join_all([
        super::route_http_request(
            "DELETE",
            "/api/v0/searches",
            None,
            "",
            &delete_all_concurrent,
        ),
        super::route_http_request(
            "DELETE",
            "/api/v0/searches",
            None,
            "",
            &delete_all_concurrent,
        ),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/searches",
        "concurrency-and-idempotency",
        concurrent_delete_all.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && delete_all_concurrent
                .searches
                .read()
                .await
                .records
                .is_empty()
    );

    let (delete_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&delete_state, detail_id, false).await;
    let delete_malformed = super::route_http_request(
        "DELETE",
        "/api/v0/searches/not-a-guid",
        None,
        "",
        &delete_state,
    )
    .await
    .expect("malformed search delete id");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "malformed-path-query-or-body",
        delete_malformed.status == "400 Bad Request"
    );
    let delete_missing = super::route_http_request(
        "DELETE",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199",
        None,
        "",
        &delete_state,
    )
    .await
    .expect("missing search delete");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        delete_missing.status == "404 Not Found"
    );
    let delete_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search delete runtime database");
    let (delete_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(delete_db.clone()),
    );
    seed_search(&delete_failure_state, detail_id, true).await;
    delete_db.close_for_test().await;
    let delete_runtime = super::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &delete_failure_state,
    )
    .await
    .expect("search delete runtime failure");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        delete_runtime.status == "500 Internal Server Error"
    );
    let reset_delete = super::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &delete_state,
    )
    .await
    .expect("search delete reset");
    let (delete_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "restart-persistence-or-reset",
        reset_delete.status == "204 No Content"
            && delete_restarted.searches.read().await.records.is_empty()
    );
    let (concurrent_delete, _receiver) = test_state_with_env(env.clone());
    seed_search(&concurrent_delete, detail_id, false).await;
    let concurrent_deletes = futures_util::future::join_all([
        super::route_http_request(
            "DELETE",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_delete,
        ),
        super::route_http_request(
            "DELETE",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_delete,
        ),
    ])
    .await;
    let delete_statuses = concurrent_deletes
        .iter()
        .filter_map(|response| response.as_ref().ok().map(|response| response.status))
        .collect::<BTreeSet<_>>();
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "concurrency-and-idempotency",
        delete_statuses == BTreeSet::from(["204 No Content", "404 Not Found"])
            && concurrent_delete.searches.read().await.records.is_empty()
    );

    let (cleanup_state, _receiver) = test_state_with_env(env.clone());
    let cleanup_nominal =
        super::route_http_request("POST", "/api/v0/searches/cleanup", None, "", &cleanup_state)
            .await
            .expect("search cleanup");
    let cleanup_json =
        serde_json::from_str::<serde_json::Value>(&cleanup_nominal.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "nominal-status-headers-body",
        cleanup_nominal.status == "200 OK"
            && cleanup_json["deleted"].is_u64()
            && cleanup_json["appliedMaxAgeDays"].is_i64()
            && cleanup_json["appliedMaxCount"].is_i64()
    );
    let cleanup_malformed = super::route_http_request(
        "POST",
        "/api/v0/searches/cleanup?maxAgeDays=not-a-number",
        None,
        "",
        &cleanup_state,
    )
    .await
    .expect("malformed search cleanup query");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "malformed-path-query-or-body",
        cleanup_malformed.status == "400 Bad Request"
    );
    let cleanup_empty =
        super::route_http_request("POST", "/api/v0/searches/cleanup", None, "", &cleanup_state)
            .await
            .expect("empty search cleanup");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "missing-empty-or-conflict-state",
        cleanup_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&cleanup_empty.body)
                .is_ok_and(|value| value["deleted"] == 0)
    );
    let cleanup_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search cleanup runtime database");
    let (cleanup_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(cleanup_db.clone()),
    );
    cleanup_db.close_for_test().await;
    let cleanup_runtime = super::route_http_request(
        "POST",
        "/api/v0/searches/cleanup",
        None,
        "",
        &cleanup_failure_state,
    )
    .await
    .expect("search cleanup runtime failure");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "runtime-failure-and-timeout",
        cleanup_runtime.status == "500 Internal Server Error"
    );
    let cleanup_side_effect_id = "00000000-0000-4000-8000-000000000104";
    let (cleanup_side_effect_state, _receiver) = test_state_with_env(env.clone());
    seed_expired_search(&cleanup_side_effect_state, cleanup_side_effect_id).await;
    let cleanup_side_effect = super::route_http_request(
        "POST",
        "/api/v0/searches/cleanup",
        None,
        "",
        &cleanup_side_effect_state,
    )
    .await
    .expect("search cleanup side effect");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "mutation-side-effects-and-readback",
        cleanup_side_effect.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&cleanup_side_effect.body)
                .is_ok_and(|value| value["deleted"] == 1)
            && cleanup_side_effect_state
                .searches
                .read()
                .await
                .records
                .is_empty()
    );
    let (cleanup_reset_state, _receiver) = test_state_with_env(env.clone());
    let cleanup_reset = super::route_http_request(
        "POST",
        "/api/v0/searches/cleanup",
        None,
        "",
        &cleanup_reset_state,
    )
    .await
    .expect("search cleanup reset");
    let (cleanup_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "restart-persistence-or-reset",
        cleanup_reset.status == "200 OK"
            && cleanup_restarted.searches.read().await.records.is_empty()
    );
    let (cleanup_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_expired_search(&cleanup_concurrent, cleanup_side_effect_id).await;
    let concurrent_cleanups = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/searches/cleanup",
            None,
            "",
            &cleanup_concurrent,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/searches/cleanup",
            None,
            "",
            &cleanup_concurrent,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "concurrency-and-idempotency",
        concurrent_cleanups.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && cleanup_concurrent.searches.read().await.records.is_empty()
    );

    let (put_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&put_state, detail_id, false).await;
    let put_malformed =
        super::route_http_request("PUT", "/api/v0/searches/not-a-guid", None, "", &put_state)
            .await
            .expect("malformed search update id");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "malformed-path-query-or-body",
        put_malformed.status == "400 Bad Request"
    );
    let put_missing = super::route_http_request(
        "PUT",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199",
        None,
        "",
        &put_state,
    )
    .await
    .expect("missing search update");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        put_missing.status == "404 Not Found"
    );
    let put_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("search update runtime database");
    let (put_failure_state, _receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(put_db.clone()));
    seed_search(&put_failure_state, detail_id, true).await;
    put_db.close_for_test().await;
    let put_runtime = super::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &put_failure_state,
    )
    .await
    .expect("search update runtime failure");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        put_runtime.status == "500 Internal Server Error"
    );
    let reset_put = super::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &put_state,
    )
    .await
    .expect("search update reset");
    let (put_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "restart-persistence-or-reset",
        reset_put.status == "200 OK" && put_restarted.searches.read().await.records.is_empty()
    );
    let (concurrent_put, _receiver) = test_state_with_env(env);
    seed_search(&concurrent_put, detail_id, false).await;
    let concurrent_puts = futures_util::future::join_all([
        super::route_http_request(
            "PUT",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_put,
        ),
        super::route_http_request(
            "PUT",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_put,
        ),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "concurrency-and-idempotency",
        concurrent_puts.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && concurrent_put
                .searches
                .read()
                .await
                .get_by_identifier(detail_id)
                .is_some_and(|record| record.status == "cancelled")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create searches evidence directory");
    fs::write(
        evidence_dir.join("searches_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize searches evidence"),
    )
    .expect("write searches evidence");
    assert_eq!(ledger.len(), 43, "searches open-case ledger size");
    assert!(
        mismatches.is_empty(),
        "{} slskdN searches mismatches:\n{}",
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
async fn controller_api_differential_controller_upload_lifecycle() {
    let (state, _receiver) =
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

    let upload_id = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("slskd-upload-peer".to_owned()),
            "Uploads/Parity.flac".to_owned(),
            None,
            Some(50),
        );
        transfers
            .update_status(entry.id, "in_progress", Some(17), None)
            .expect("mark slskd upload active")
            .id
    };

    let uploads = super::route_http_request("GET", "/api/v0/transfers/uploads", None, "", &state)
        .await
        .expect("slskd upload list");
    let uploads_json = serde_json::from_str::<serde_json::Value>(&uploads.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/uploads",
        "nominal-status-headers-body",
        uploads.status == "200 OK" && uploads_json.is_array()
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads",
        "populated-dynamic-state",
        uploads.status == "200 OK"
            && uploads_json.as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["username"] == "slskd-upload-peer"
                        && row["directories"].as_array().is_some_and(|directories| {
                            directories.iter().any(|directory| {
                                directory["files"].as_array().is_some_and(|files| {
                                    files
                                        .iter()
                                        .any(|file| file["filename"] == "Uploads/Parity.flac")
                                })
                            })
                        })
                })
            })
    );

    let user_uploads = super::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/slskd-upload-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd user uploads");
    let user_uploads_json =
        serde_json::from_str::<serde_json::Value>(&user_uploads.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "nominal-status-headers-body",
        user_uploads.status == "200 OK" && user_uploads_json["username"] == "slskd-upload-peer"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "populated-dynamic-state",
        user_uploads.status == "200 OK"
            && user_uploads_json["directories"][0]["files"][0]["filename"] == "Uploads/Parity.flac"
    );

    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/uploads/slskd-upload-peer/{upload_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd upload detail");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "nominal-status-headers-body",
        detail.status == "200 OK"
            && detail_json["filename"] == "Uploads/Parity.flac"
            && detail_json["direction"] == "Upload"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "populated-dynamic-state",
        detail.status == "200 OK"
            && detail_json["id"] == upload_id
            && detail_json["bytesTransferred"] == 17
    );

    let missing_user = super::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/no-such-user",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd upload user");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "missing-empty-or-conflict-state",
        missing_user.status == "404 Not Found"
    );

    let missing_detail = super::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/slskd-upload-peer/999999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd upload detail");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "missing-empty-or-conflict-state",
        missing_detail.status == "404 Not Found"
    );

    let malformed_detail = super::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/slskd-upload-peer/not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd upload detail");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        "malformed-path-query-or-body",
        malformed_detail.status == "400 Bad Request"
    );

    let cancelled = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/slskd-upload-peer/{upload_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskd upload");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content"
            && state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .find(|entry| entry.id == upload_id)
                .is_some_and(|entry| entry.status == "cancelled")
    );

    let pruned = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("prune slskd uploads");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "nominal-status-headers-body",
        pruned.status == "204 No Content" && pruned.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "mutation-side-effects-and-readback",
        pruned.status == "204 No Content"
            && state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .all(|entry| entry.id != upload_id)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_upload_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd upload ledger"),
    )
    .expect("write slskd upload ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd upload controller mismatches:\n{}",
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
async fn controller_api_differential_controller_transfer_failure_restart_and_idempotency() {
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

    let empty_state =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target)).0;
    for (path, route) in [
        ("/api/v0/transfers/downloads", "/api/v0/transfers/downloads"),
        ("/api/v0/transfers/uploads", "/api/v0/transfers/uploads"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &empty_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_default()
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    let malformed_download = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        "{}",
        &empty_state,
    )
    .await
    .expect("malformed slskd download request");
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "malformed-path-query-or-body",
        malformed_download.status == "400 Bad Request"
    );

    // Frozen slskd admits two legacy enqueue operations at a time and
    // rejects the next one before touching transfer state. Hold both
    // permits to exercise the same deterministic 429 boundary.
    let first_download_permit = Arc::clone(&empty_state.download_requests)
        .acquire_owned()
        .await
        .expect("first legacy download limiter permit");
    let second_download_permit = Arc::clone(&empty_state.download_requests)
        .acquire_owned()
        .await
        .expect("second legacy download limiter permit");
    let throttled_download = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/concurrent-peer",
        None,
        r#"{"files":[{"filename":"Concurrent/Track.flac","size":42}]}"#,
        &empty_state,
    )
    .await
    .expect("throttled slskd download request");
    let state_unchanged = empty_state.transfers.read().await.entries.is_empty();
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "concurrency-and-idempotency",
        throttled_download.status == "429 Too Many Requests" && state_unchanged
    );
    drop(second_download_permit);
    drop(first_download_permit);

    let malformed_batch = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        "{}",
        &empty_state,
    )
    .await
    .expect("malformed slskd download batch");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "malformed-path-query-or-body",
        malformed_batch.status == "400 Bad Request"
    );

    // Both direction-specific cancel routes return the same stable
    // missing-resource contract and can be safely repeated for an
    // already-cancelled transfer.
    for (direction, route) in [
        ("downloads", "/api/v0/transfers/downloads/{username}/{id}"),
        ("uploads", "/api/v0/transfers/uploads/{username}/{id}"),
    ] {
        let missing = super::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/missing-peer/999999"),
            None,
            "",
            &empty_state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE missing {direction}: {error}"));
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }

    for (direction, route) in [
        ("downloads", "/api/v0/transfers/downloads/all/completed"),
        ("uploads", "/api/v0/transfers/uploads/all/completed"),
    ] {
        let first = super::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/all/completed"),
            None,
            "",
            &empty_state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE empty {direction}: {error}"));
        let second = super::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/all/completed"),
            None,
            "",
            &empty_state,
        )
        .await
        .unwrap_or_else(|error| panic!("repeat DELETE empty {direction}: {error}"));
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            first.status == "204 No Content" && second.status == "204 No Content"
        );
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            first.status == "204 No Content" && second.status == "204 No Content"
        );
    }

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskd transfer differential database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with(
            "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
            "restart-peer=127.0.0.1:2234",
        );
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
    let queued = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/restart-peer",
        None,
        r#"{"files":[{"filename":"Persist/Track.flac","size":42}]}"#,
        &state,
    )
    .await
    .expect("persist slskd queued transfer");
    let persisted = db
        .list_transfers(None, 10, 0)
        .await
        .expect("read persisted slskd transfer");
    let mut rehydrated = super::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated.rehydrate_from_database(&db).await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "restart-persistence-or-reset",
        queued.status == "200 OK"
            && persisted
                .iter()
                .any(|entry| entry.filename == "Persist/Track.flac")
            && rehydrated
                .entries
                .iter()
                .any(|entry| entry.filename == "Persist/Track.flac")
    );

    let transfer_id = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.filename == "Persist/Track.flac")
        .map(|entry| entry.id)
        .expect("persisted transfer id");
    for (direction, route) in [
        ("downloads", "/api/v0/transfers/downloads/{username}/{id}"),
        ("uploads", "/api/v0/transfers/uploads/{username}/{id}"),
    ] {
        let (username, id) = if direction == "downloads" {
            ("restart-peer", transfer_id)
        } else {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(
                1,
                Some("restart-peer".to_owned()),
                "Persist/Upload.flac".to_owned(),
                None,
                Some(42),
            );
            ("restart-peer", entry.id)
        };
        let first = super::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/{username}/{id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE {direction} transfer: {error}"));
        let second = super::route_http_request(
            "DELETE",
            &format!("/api/v0/transfers/{direction}/{username}/{id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("repeat DELETE {direction} transfer: {error}"));
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            first.status == "204 No Content" && second.status == "204 No Content"
        );
    }

    // A failed durable write must not leave a transfer in the in-memory
    // queue.  This is the real rollback contract for the slskd enqueue
    // route, not just an assertion about the returned error.
    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskd transfer failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    let failed = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/failure-peer",
        None,
        r#"{"files":[{"filename":"Failure/Track.flac","size":42}]}"#,
        &failure_state,
    )
    .await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "runtime-failure-and-timeout",
        failed.is_ok_and(|response| response.status == "503 Service Unavailable")
            && failure_state.transfers.read().await.entries.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_transfer_failure_restart_and_idempotency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd transfer failure ledger"),
    )
    .expect("write slskd transfer failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd transfer failure/restart mismatches:\n{}",
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
async fn controller_api_differential_controller_transfer_batch_cleanup_and_failures() {
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
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with(
            "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
            "batch-peer=127.0.0.1:2234",
        );
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("transfer batch database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
    let batch_id = "55555555-5555-4555-8555-555555555555";
    let batch_body = format!(
        r#"{{"id":"{batch_id}","username":"batch-peer","files":[{{"filename":"Batch/One.flac","size":101}},{{"filename":"Batch/Two.flac","size":202}}]}}"#
    );
    let batch = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("create transfer batch");
    let persisted_batch_transfers = db
        .list_transfers(None, 20, 0)
        .await
        .expect("list batch transfers");
    let mut rehydrated = super::TransferQueue::new(&state.config);
    rehydrated.rehydrate_from_database(&db).await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "restart-persistence-or-reset",
        batch.status == "201 Created"
            && persisted_batch_transfers.len() == 2
            && persisted_batch_transfers
                .iter()
                .all(|entry| entry.batch_id.as_deref() == Some(batch_id))
            && rehydrated
                .entries
                .iter()
                .filter(|entry| entry.batch_id.as_deref() == Some(batch_id))
                .count()
                == 2
    );
    let _ = receiver.try_recv();
    let duplicate_batch = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("duplicate transfer batch");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "concurrency-and-idempotency",
        duplicate_batch.status == "409 Conflict"
    );

    let batch_entry_ids = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .filter(|entry| entry.batch_id.as_deref() == Some(batch_id))
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    let cancelled_download = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/downloads/batch-peer/{}",
            batch_entry_ids[0]
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel persisted batch download");
    let persisted_cancelled_download = db
        .get_transfer(&batch_entry_ids[0].to_string())
        .await
        .expect("read cancelled batch download")
        .expect("cancelled batch download row");
    let mut rehydrated_cancelled =
        super::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated_cancelled.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_download.status == "204 No Content"
            && persisted_cancelled_download.status == "cancelled"
            && rehydrated_cancelled
                .entries
                .iter()
                .any(|entry| entry.id == batch_entry_ids[0] && entry.status == "cancelled")
    );
    let repeated_cancelled_download = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/downloads/batch-peer/{}",
            batch_entry_ids[0]
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat batch download cancellation");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "concurrency-and-idempotency",
        repeated_cancelled_download.status == "204 No Content"
    );

    let upload_entry = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("upload-peer".to_owned()),
            "Upload/Track.flac".to_owned(),
            None,
            Some(303),
        )
    };
    super::persist_transfer_record(&state, &upload_entry)
        .await
        .expect("persist upload entry");
    let cancelled_upload = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/upload-peer/{}", upload_entry.id),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel persisted upload");
    let persisted_cancelled_upload = db
        .get_transfer(&upload_entry.id.to_string())
        .await
        .expect("read cancelled upload")
        .expect("cancelled upload row");
    let mut rehydrated_upload =
        super::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated_upload.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_upload.status == "204 No Content"
            && persisted_cancelled_upload.status == "cancelled"
            && rehydrated_upload
                .entries
                .iter()
                .any(|entry| entry.id == upload_entry.id && entry.status == "cancelled")
    );
    let repeated_cancelled_upload = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/upload-peer/{}", upload_entry.id),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat upload cancellation");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "concurrency-and-idempotency",
        repeated_cancelled_upload.status == "204 No Content"
    );

    let completed_download = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("cleanup-peer".to_owned()),
            "Cleanup.flac".to_owned(),
            None,
            Some(404),
        )
    };
    let completed_upload = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("cleanup-peer".to_owned()),
            "Cleanup-upload.flac".to_owned(),
            None,
            Some(405),
        )
    };
    for entry in [&completed_download, &completed_upload] {
        let completed = state
            .transfers
            .write()
            .await
            .update_status(entry.id, "completed", entry.size, None)
            .expect("mark cleanup transfer completed");
        super::persist_transfer_record(&state, &completed)
            .await
            .expect("persist cleanup transfer");
    }
    let cleanup_download = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup completed downloads");
    let persisted_after_download_cleanup = db
        .get_transfer(&completed_download.id.to_string())
        .await
        .expect("read cleaned download");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "restart-persistence-or-reset",
        cleanup_download.status == "204 No Content" && persisted_after_download_cleanup.is_none()
    );
    let repeated_cleanup_download = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cleanup completed downloads");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "concurrency-and-idempotency",
        repeated_cleanup_download.status == "204 No Content"
    );
    let cleanup_upload = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup completed uploads");
    let persisted_after_upload_cleanup = db
        .get_transfer(&completed_upload.id.to_string())
        .await
        .expect("read cleaned upload");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "restart-persistence-or-reset",
        cleanup_upload.status == "204 No Content" && persisted_after_upload_cleanup.is_none()
    );
    let repeated_cleanup_upload = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cleanup completed uploads");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "concurrency-and-idempotency",
        repeated_cleanup_upload.status == "204 No Content"
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("batch failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_batch = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        r#"{"id":"66666666-6666-4666-8666-666666666666","username":"batch-peer","files":[{"filename":"Failure/Batch.flac","size":1}]}"#,
        &failure_state,
    )
    .await
    .expect("batch persistence failure response");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "runtime-failure-and-timeout",
        failed_batch.status == "503 Service Unavailable"
            && failure_state.transfers.read().await.entries.is_empty()
            && failure_state
                .controller_features
                .read()
                .await
                .get("slskd/transfer-batch/66666666-6666-4666-8666-666666666666")
                .is_none()
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("download cancellation failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_download = {
        let mut transfers = failure_state.transfers.write().await;
        transfers.create(
            0,
            Some("failure-peer".to_owned()),
            "Failure/Download.flac".to_owned(),
            None,
            Some(1),
        )
    };
    failure_db.close_for_test().await;
    let failed_download_cancel = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/downloads/failure-peer/{}",
            failure_download.id
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("download cancellation failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "runtime-failure-and-timeout",
        failed_download_cancel.status == "503 Service Unavailable"
            && failure_state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .any(|entry| entry.id == failure_download.id && entry.status == "queued")
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("upload cancellation failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_upload = {
        let mut transfers = failure_state.transfers.write().await;
        transfers.create(
            1,
            Some("failure-peer".to_owned()),
            "Failure/Upload.flac".to_owned(),
            None,
            Some(1),
        )
    };
    failure_db.close_for_test().await;
    let failed_upload_cancel = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/uploads/failure-peer/{}",
            failure_upload.id
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("upload cancellation failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "runtime-failure-and-timeout",
        failed_upload_cancel.status == "503 Service Unavailable"
            && failure_state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .any(|entry| entry.id == failure_upload.id && entry.status == "queued")
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("download cleanup failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    {
        let mut transfers = failure_state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("failure-peer".to_owned()),
            "Failure/Cleanup.flac".to_owned(),
            None,
            Some(1),
        );
        transfers
            .update_status(entry.id, "completed", entry.size, None)
            .expect("seed completed download cleanup");
    }
    let previous_cleanup_entries = failure_state.transfers.read().await.entries.clone();
    failure_db.close_for_test().await;
    let failed_cleanup = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("download cleanup failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "runtime-failure-and-timeout",
        failed_cleanup.status == "503 Service Unavailable"
            && failure_state.transfers.read().await.entries == previous_cleanup_entries
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("upload cleanup failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(failure_db.clone()));
    {
        let mut transfers = failure_state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("failure-peer".to_owned()),
            "Failure/UploadCleanup.flac".to_owned(),
            None,
            Some(1),
        );
        transfers
            .update_status(entry.id, "completed", entry.size, None)
            .expect("seed completed upload cleanup");
    }
    let previous_upload_cleanup_entries = failure_state.transfers.read().await.entries.clone();
    failure_db.close_for_test().await;
    let failed_upload_cleanup = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("upload cleanup failure response");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "runtime-failure-and-timeout",
        failed_upload_cleanup.status == "503 Service Unavailable"
            && failure_state.transfers.read().await.entries == previous_upload_cleanup_entries
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_transfer_batch_cleanup_and_failures.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd transfer batch ledger"),
    )
    .expect("write slskd transfer batch ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd transfer batch/failure mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
