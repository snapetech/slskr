//! Controller full native api differential 02 ownership.

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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_native_autoreplace_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    fn status_matches(response: &crate::HttpResponse, enabled: bool) -> bool {
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| {
                    value["enabled"] == enabled
                        && value["lastRunAt"].is_null()
                        && value["lastRunProcessedCount"] == 0
                        && value["lastRunReplacedCount"] == 0
                        && value["intervalSeconds"] == 300
                })
    }

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());
    let malformed_status = crate::route_http_request(
        "GET",
        "/api/v0/autoreplace?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed auto-replace status query");
    record!(
        "GET",
        "/api/v0/autoreplace",
        "malformed-path-query-or-body",
        status_matches(&malformed_status, false)
    );
    let empty_status = crate::route_http_request("GET", "/api/v0/autoreplace", None, "", &state)
        .await
        .expect("empty auto-replace status");
    record!(
        "GET",
        "/api/v0/autoreplace",
        "missing-empty-or-conflict-state",
        status_matches(&empty_status, false)
    );

    let read_failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace read-failure database");
    let (read_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        crate::SearchStore::new(),
        Some(read_failure_db.clone()),
    );
    read_failure_db.close_for_test().await;
    let read_failure_status =
        crate::route_http_request("GET", "/api/v0/autoreplace", None, "", &read_failure_state)
            .await
            .expect("auto-replace status after database failure");
    record!(
        "GET",
        "/api/v0/autoreplace",
        "runtime-failure-and-timeout",
        status_matches(&read_failure_status, false)
    );

    let (enable_state, _receiver) = test_state_with_env(env());
    let malformed_enable = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable?unexpected=not-a-number",
        None,
        "not-json",
        &enable_state,
    )
    .await
    .expect("malformed auto-replace enable query");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "malformed-path-query-or-body",
        status_matches(&malformed_enable, true)
    );
    let enabled =
        crate::route_http_request("PUT", "/api/v0/autoreplace/enable", None, "", &enable_state)
            .await
            .expect("enable auto-replace");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "missing-empty-or-conflict-state",
        status_matches(&enabled, true)
    );

    let enable_failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace enable-failure database");
    let (enable_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        crate::SearchStore::new(),
        Some(enable_failure_db.clone()),
    );
    enable_failure_db.close_for_test().await;
    let failed_enable = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_failure_state,
    )
    .await
    .expect("auto-replace enable after database failure");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "runtime-failure-and-timeout",
        status_matches(&failed_enable, true)
    );

    let enable_restart_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace enable restart database");
    let (enable_restart_state, _receiver) = test_state_with_env_parts(
        env(),
        crate::SearchStore::new(),
        Some(enable_restart_db.clone()),
    );
    let enabled_for_restart = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_restart_state,
    )
    .await
    .expect("persist auto-replace enable");
    let enable_record = enable_restart_db
        .get_runtime_compat_state()
        .await
        .expect("read auto-replace enable state")
        .expect("auto-replace enable state record");
    let rehydrated_enable = crate::RuntimeCompatState::from_persisted(&enable_record);
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "restart-persistence-or-reset",
        status_matches(&enabled_for_restart, true) && rehydrated_enable.autoreplace_enabled
    );

    let (enable_concurrent_state, _receiver) = test_state_with_env(env());
    let enable_first = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_concurrent_state,
    )
    .await
    .expect("first auto-replace enable");
    let enable_second = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_concurrent_state,
    )
    .await
    .expect("repeat auto-replace enable");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "concurrency-and-idempotency",
        status_matches(&enable_first, true)
            && status_matches(&enable_second, true)
            && enable_concurrent_state
                .runtime
                .read()
                .await
                .autoreplace_enabled
    );

    let (disable_state, _receiver) = test_state_with_env(env());
    let malformed_disable = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable?unexpected=not-a-number",
        None,
        "not-json",
        &disable_state,
    )
    .await
    .expect("malformed auto-replace disable query");
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "malformed-path-query-or-body",
        status_matches(&malformed_disable, false)
    );
    let disabled = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_state,
    )
    .await
    .expect("disable auto-replace");
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "missing-empty-or-conflict-state",
        status_matches(&disabled, false)
    );

    let disable_restart_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace disable restart database");
    let (disable_restart_state, _receiver) = test_state_with_env_parts(
        env(),
        crate::SearchStore::new(),
        Some(disable_restart_db.clone()),
    );
    let enabled_before_disable = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &disable_restart_state,
    )
    .await
    .expect("enable before auto-replace disable");
    let disabled_for_restart = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_restart_state,
    )
    .await
    .expect("persist auto-replace disable");
    let disable_record = disable_restart_db
        .get_runtime_compat_state()
        .await
        .expect("read auto-replace disable state")
        .expect("auto-replace disable state record");
    let rehydrated_disable = crate::RuntimeCompatState::from_persisted(&disable_record);
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "restart-persistence-or-reset",
        status_matches(&enabled_before_disable, true)
            && status_matches(&disabled_for_restart, false)
            && !rehydrated_disable.autoreplace_enabled
    );

    let (disable_concurrent_state, _receiver) = test_state_with_env(env());
    let disable_first = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_concurrent_state,
    )
    .await
    .expect("first auto-replace disable");
    let disable_second = crate::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_concurrent_state,
    )
    .await
    .expect("repeat auto-replace disable");
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "concurrency-and-idempotency",
        status_matches(&disable_first, false)
            && status_matches(&disable_second, false)
            && !disable_concurrent_state
                .runtime
                .read()
                .await
                .autoreplace_enabled
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_autoreplace_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn auto-replace edge ledger"),
    )
    .expect("write slskdn auto-replace edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn auto-replace edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining slskdN OptionsController
/// edges.  These cases cover the controller's invalid-options failure
/// projection, startup/config-file isolation from SQLite, validation's
/// deliberately non-mutating lifecycle, and concurrent YAML replacement.
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
pub(super) async fn controller_api_differential_native_options_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    fn problem_response(response: &crate::HttpResponse) -> bool {
        response.status == "500 Internal Server Error"
            && response.content_type == "application/problem+json"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| {
                    value["title"] == "Internal Server Error"
                        && value["status"] == 500
                        && value["detail"] == "An unexpected error occurred."
                })
    }

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true")
            .with("SLSKR_DEBUG", "true")
            .with("SLSKR_NO_CONFIG_WATCH", "true")
    };

    let (state, _receiver) = test_state_with_env(env());
    let malformed_current =
        crate::route_http_request("GET", "/api/v0/options/extra", None, "", &state)
            .await
            .expect("malformed current options response");
    record!(
        "GET",
        "/api/v0/options",
        "malformed-path-query-or-body",
        malformed_current.status == "404 Not Found"
    );

    *state
        .controller_options_validation_error
        .write()
        .expect("current options validation error lock") =
        Some("differential options failure".to_owned());
    let invalid_current = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
        .await
        .expect("invalid current options response");
    record!(
        "GET",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        problem_response(&invalid_current)
    );

    let invalid_debug = crate::route_http_request("GET", "/api/v0/options/debug", None, "", &state)
        .await
        .expect("invalid debug options response");
    record!(
        "GET",
        "/api/v0/options/debug",
        "runtime-failure-and-timeout",
        problem_response(&invalid_debug)
    );

    let patch_failure = crate::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50331}}"#,
        &state,
    )
    .await
    .expect("invalid options patch response");
    record!(
        "PATCH",
        "/api/v0/options",
        "runtime-failure-and-timeout",
        problem_response(&patch_failure)
    );

    let database = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("options read-only failure database");
    let (database_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(database.clone()));
    database.close_for_test().await;
    let startup_after_database_failure =
        crate::route_http_request("GET", "/api/v0/options/startup", None, "", &database_state)
            .await
            .expect("startup options after database failure");
    record!(
        "GET",
        "/api/v0/options/startup",
        "runtime-failure-and-timeout",
        startup_after_database_failure.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&startup_after_database_failure.body,)
                .is_ok_and(|value| value.is_object())
    );

    let location_after_database_failure = crate::route_http_request(
        "GET",
        "/api/v0/options/yaml/location",
        None,
        "",
        &database_state,
    )
    .await
    .expect("options location after database failure");
    record!(
        "GET",
        "/api/v0/options/yaml/location",
        "runtime-failure-and-timeout",
        location_after_database_failure.status == "200 OK"
            && serde_json::from_str::<String>(&location_after_database_failure.body)
                .is_ok_and(|value| value.ends_with("slskd.yml"))
    );

    let (missing_validation_state, _receiver) = test_state_with_env(env());
    let missing_validation = crate::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        "",
        &missing_validation_state,
    )
    .await
    .expect("missing YAML validation response");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "missing-empty-or-conflict-state",
        missing_validation.status == "400 Bad Request"
    );

    let valid_yaml = serde_json::to_string("soulseek:\n  description: validation-only\n")
        .expect("serialize validation YAML");
    let validation_state = test_state_with_env(env()).0;
    let validation_path = validation_state.config.state_dir.join("slskd.yml");
    let _ = fs::remove_file(&validation_path);
    let before_description = validation_state
        .user_info_description
        .read()
        .expect("validation description lock")
        .clone();
    let validation = crate::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        &valid_yaml,
        &validation_state,
    )
    .await
    .expect("valid YAML validation response");
    let after_description = validation_state
        .user_info_description
        .read()
        .expect("validation description readback lock")
        .clone();
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "mutation-side-effects-and-readback",
        validation.status == "200 OK"
            && validation.body.is_empty()
            && !validation_path.exists()
            && after_description == before_description
    );

    let reload_env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            validation_state.config.state_dir.to_str().unwrap(),
        )
        .with("SLSKR_CONTROLLER_PROFILE", target);
    let reloaded = crate::AppConfig::from_layers(None, FileConfig::default(), &reload_env)
        .expect("reload options validation state");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "restart-persistence-or-reset",
        reloaded.user_info_description != "validation-only" && !validation_path.exists()
    );

    let (failed_validation_state, _receiver) = test_state_with_env(env());
    *failed_validation_state
        .controller_options_validation_error
        .write()
        .expect("validation failure lock") = Some("differential options failure".to_owned());
    let failed_validation = crate::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        &valid_yaml,
        &failed_validation_state,
    )
    .await
    .expect("runtime YAML validation response");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "runtime-failure-and-timeout",
        problem_response(&failed_validation)
    );

    let (malformed_update_state, _receiver) = test_state_with_env(env());
    let malformed_update = crate::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        "not-json",
        &malformed_update_state,
    )
    .await
    .expect("malformed YAML update response");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "malformed-path-query-or-body",
        malformed_update.status == "400 Bad Request"
    );

    let missing_update = crate::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        "",
        &malformed_update_state,
    )
    .await
    .expect("missing YAML update response");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "missing-empty-or-conflict-state",
        missing_update.status == "400 Bad Request"
    );

    let (failed_update_state, _receiver) = test_state_with_env(env());
    *failed_update_state
        .controller_options_validation_error
        .write()
        .expect("YAML update failure lock") = Some("differential options failure".to_owned());
    let failed_update = crate::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        &valid_yaml,
        &failed_update_state,
    )
    .await
    .expect("runtime YAML update response");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "runtime-failure-and-timeout",
        problem_response(&failed_update)
    );

    let (concurrent_state, _receiver) = test_state_with_env(env());
    let concurrent_path = concurrent_state.config.state_dir.join("slskd.yml");
    fs::write(
        &concurrent_path,
        "soulseek:\n  description: options-initial\n",
    )
    .expect("write concurrent YAML baseline");
    let concurrent_bodies = [
        serde_json::to_string("soulseek:\n  description: options-a\n")
            .expect("serialize concurrent YAML A"),
        serde_json::to_string("soulseek:\n  description: options-b\n")
            .expect("serialize concurrent YAML B"),
    ];
    let concurrent_responses =
        futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
            crate::route_http_request("PUT", "/api/v0/options/yaml", None, body, &concurrent_state)
        }))
        .await;
    let concurrent_readback = fs::read_to_string(&concurrent_path).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty())
        }) && [
            "soulseek:\n  description: options-a\n",
            "soulseek:\n  description: options-b\n",
        ]
        .contains(&concurrent_readback.as_str())
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_options_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn options edge ledger"),
    )
    .expect("write slskdn options edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn options edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the frozen slskdN EventsController.  The
/// native controller reads and writes the durable event service, rejects
/// invalid paging and synthetic-event requests, rolls back failed writes,
/// and preserves records through rehydration and concurrent injection.
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
pub(super) async fn controller_api_differential_native_events_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
    };
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("events differential database");
    let (state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));

    let empty =
        crate::route_http_request("GET", "/api/v0/events?offset=0&limit=100", None, "", &state)
            .await
            .expect("empty slskdn events response");
    let empty_json =
        serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        empty.status == "200 OK"
            && empty.content_type == "application/json"
            && empty_json.as_array().is_some_and(Vec::is_empty)
    );

    let malformed_path = crate::route_http_request("GET", "/api/v0/events/extra", None, "", &state)
        .await
        .expect("malformed events path response");
    let malformed_query =
        crate::route_http_request("GET", "/api/v0/events?offset=-1", None, "", &state)
            .await
            .expect("malformed events query response");
    record!(
        "GET",
        "/api/v0/events",
        "malformed-path-query-or-body",
        malformed_path.status == "404 Not Found"
            && malformed_query.status == "400 Bad Request"
            && malformed_query.body.contains("Offset must be greater")
    );

    let raised =
        crate::route_http_request("POST", "/api/v0/events/Noop", None, r#""event-a""#, &state)
            .await
            .expect("nominal slskdn event response");
    let raised_json = serde_json::from_str::<serde_json::Value>(&raised.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/events",
        "nominal-status-headers-body",
        raised.status == "201 Created"
            && raised.content_type == "application/json"
            && raised_json["recorded"] == true
            && raised_json["event"]["type"] == "Noop"
    );

    let populated = crate::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("populated slskdn events response");
    let populated_json = serde_json::from_str::<serde_json::Value>(&populated.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/events",
        "populated-dynamic-state",
        populated.status == "200 OK"
            && populated_json.as_array().is_some_and(|events| {
                events
                    .iter()
                    .any(|event| event["type"] == "Noop" && event["detail"] == "event-a")
            })
    );
    record!(
        "POST",
        "/api/v0/events",
        "mutation-side-effects-and-readback",
        raised.status == "201 Created"
            && populated_json
                .as_array()
                .is_some_and(|events| { events.iter().any(|event| event["detail"] == "event-a") })
    );

    let unknown = crate::route_http_request(
        "POST",
        "/api/v0/events/Unknown",
        None,
        r#""event-unknown""#,
        &state,
    )
    .await
    .expect("unknown slskdn event response");
    record!(
        "POST",
        "/api/v0/events",
        "malformed-path-query-or-body",
        unknown.status == "400 Bad Request" && unknown.body.contains("Unknown event type")
    );

    let missing_body = crate::route_http_request("POST", "/api/v0/events/Noop", None, "{", &state)
        .await
        .expect("missing slskdn event body response");
    record!(
        "POST",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        missing_body.status == "400 Bad Request"
    );

    let persisted = db.list_events(20, 0).await.expect("list persisted events");
    let rehydrated =
        crate::EventStore::from_persisted(persisted.clone(), crate::EVENT_HISTORY_LIMIT);
    record!(
        "POST",
        "/api/v0/events",
        "restart-persistence-or-reset",
        persisted
            .iter()
            .any(|event| event.detail.as_deref() == Some("event-a"))
            && rehydrated.controller_json(None).contains("event-a")
    );

    let concurrent_responses = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-a""#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-b""#,
            &state,
        ),
    ])
    .await;
    let concurrent_events = db.list_events(20, 0).await.expect("list concurrent events");
    record!(
        "POST",
        "/api/v0/events",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
        }) && concurrent_events
            .iter()
            .any(|event| { event.detail.as_deref() == Some("concurrent-a") })
            && concurrent_events
                .iter()
                .any(|event| { event.detail.as_deref() == Some("concurrent-b") })
    );

    let read_failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("events read-failure database");
    let (read_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        crate::SearchStore::new(),
        Some(read_failure_db.clone()),
    );
    read_failure_db.close_for_test().await;
    let read_failure =
        crate::route_http_request("GET", "/api/v0/events", None, "", &read_failure_state)
            .await
            .expect("events read failure response");
    record!(
        "GET",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        read_failure.status == "500 Internal Server Error"
            && read_failure.body.contains("Failed to list events")
    );

    let write_failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("events write-failure database");
    let (write_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        crate::SearchStore::new(),
        Some(write_failure_db.clone()),
    );
    write_failure_db.close_for_test().await;
    let write_failure = crate::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""failure""#,
        &write_failure_state,
    )
    .await
    .expect("events write failure response");
    record!(
        "POST",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        write_failure.status == "500 Internal Server Error"
            && write_failure.body.contains("Failed to raise event")
            && write_failure_state.events.read().await.records.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_events_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn events edge ledger"),
    )
    .expect("write slskdn events edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn events edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining versioned CapabilitiesController
/// cases.  The frozen controller is backed by process-local capability
/// service state, so a closed SQLite pool must not change its projections;
/// parsing likewise has no durable side effects to restore on restart.
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
pub(super) async fn controller_api_differential_native_capabilities_contracts() {
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

    let valid_root = |response: &crate::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let capability_file = value["json"]
            .as_str()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["version"] == "slskdn/1.0.0+dht+mesh+swarm"
            && value["tag"] == "slskdn_caps:v1;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1"
            && capability_file["client"] == "slskdn"
            && capability_file["version"] == "1.0.0"
            && capability_file["capabilities"] == 63
    };

    let empty_peer_list = |response: &crate::routing::HttpResponse| {
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && serde_json::from_str::<serde_json::Value>(&response.body).ok()
                == Some(serde_json::json!({"count": 0, "peers": []}))
    };

    let (root_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let malformed_root =
        crate::route_http_request("GET", "/api/v0/capabilities/extra", None, "", &root_state)
            .await
            .expect("malformed versioned capabilities path");
    record!(
        "GET",
        "/api/v0/capabilities",
        "malformed-path-query-or-body",
        malformed_root.status == "404 Not Found"
    );

    let empty_root =
        crate::route_http_request("GET", "/api/v0/capabilities", None, "", &root_state)
            .await
            .expect("empty versioned capabilities response");
    record!(
        "GET",
        "/api/v0/capabilities",
        "missing-empty-or-conflict-state",
        valid_root(&empty_root)
    );

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("capabilities runtime database");
    let (runtime_root_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_root =
        crate::route_http_request("GET", "/api/v0/capabilities", None, "", &runtime_root_state)
            .await
            .expect("versioned capabilities response with closed database");
    record!(
        "GET",
        "/api/v0/capabilities",
        "runtime-failure-and-timeout",
        valid_root(&runtime_root)
    );

    let (populated_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    populated_state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            "capabilities-root-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let populated_root =
        crate::route_http_request("GET", "/api/v0/capabilities", None, "", &populated_state)
            .await
            .expect("populated versioned capabilities response");
    record!(
        "GET",
        "/api/v0/capabilities",
        "populated-dynamic-state",
        valid_root(&populated_root)
    );

    let malformed_mesh = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/mesh-peers/extra",
        None,
        "",
        &root_state,
    )
    .await
    .expect("malformed mesh-peer path");
    record!(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        "malformed-path-query-or-body",
        malformed_mesh.status == "404 Not Found"
    );
    let empty_mesh = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        None,
        "",
        &root_state,
    )
    .await
    .expect("empty mesh-peer response");
    record!(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        "missing-empty-or-conflict-state",
        empty_peer_list(&empty_mesh)
    );

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh-peer runtime database");
    let (runtime_mesh_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_mesh = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        None,
        "",
        &runtime_mesh_state,
    )
    .await
    .expect("mesh-peer response with closed database");
    record!(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        "runtime-failure-and-timeout",
        empty_peer_list(&runtime_mesh)
    );

    let malformed_peers = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/peers/extra",
        None,
        "",
        &root_state,
    )
    .await
    .expect("malformed capability-peer list path");
    record!(
        "GET",
        "/api/v0/capabilities/peers",
        "malformed-path-query-or-body",
        malformed_peers.status == "404 Not Found"
    );
    let empty_peers =
        crate::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &root_state)
            .await
            .expect("empty capability-peer response");
    record!(
        "GET",
        "/api/v0/capabilities/peers",
        "missing-empty-or-conflict-state",
        empty_peer_list(&empty_peers)
    );

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("capability-peer runtime database");
    let (runtime_peers_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_peers = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/peers",
        None,
        "",
        &runtime_peers_state,
    )
    .await
    .expect("capability-peer response with closed database");
    record!(
        "GET",
        "/api/v0/capabilities/peers",
        "runtime-failure-and-timeout",
        empty_peer_list(&runtime_peers)
    );

    let peer_username = "capabilities-detail-peer";
    populated_state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            peer_username,
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let peer = crate::route_http_request(
        "GET",
        &format!("/api/v0/capabilities/peers/{peer_username}"),
        None,
        "",
        &populated_state,
    )
    .await
    .expect("known capability-peer detail");
    let valid_peer = |response: &crate::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["username"] == peer_username
            && value["protocolVersion"] == 1
            && value["flagsValue"] == 8
            && value["canMeshSync"] == true
    };
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "nominal-status-headers-body",
        valid_peer(&peer)
    );
    let malformed_peer = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/peers/%20",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("blank capability-peer username");
    let malformed_peer_value = serde_json::from_str::<serde_json::Value>(&malformed_peer.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "malformed-path-query-or-body",
        malformed_peer.status == "400 Bad Request"
            && malformed_peer_value["error"] == "Username is required"
    );
    let missing_peer = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/peers/unknown-capability-peer",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("unknown capability-peer detail");
    let missing_peer_value = serde_json::from_str::<serde_json::Value>(&missing_peer.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "missing-empty-or-conflict-state",
        missing_peer.status == "404 Not Found"
            && missing_peer_value["error"] == "No capabilities known for peer"
    );

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("capability-peer detail runtime database");
    let (runtime_peer_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    runtime_peer_state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            peer_username,
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    db.close_for_test().await;
    let runtime_peer = crate::route_http_request(
        "GET",
        &format!("/api/v0/capabilities/peers/{peer_username}"),
        None,
        "",
        &runtime_peer_state,
    )
    .await
    .expect("capability-peer detail with closed database");
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "runtime-failure-and-timeout",
        valid_peer(&runtime_peer)
    );

    let parse_body = r#"{"description":" slskr_caps:v7;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1 "}"#;
    let parsed = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &root_state,
    )
    .await
    .expect("nominal capabilities parse");
    let parsed_value =
        serde_json::from_str::<serde_json::Value>(&parsed.body).unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "nominal-status-headers-body",
        parsed.status == "200 OK"
            && parsed.content_type.starts_with("application/json")
            && parsed_value["isSlskdn"] == true
            && parsed_value["flagsValue"] == 59
            && parsed_value["protocolVersion"] == 7
            && parsed_value["canMeshSync"] == true
            && parsed_value["canSwarm"] == true
    );
    let malformed_parse = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        "not-json",
        &root_state,
    )
    .await
    .expect("malformed capabilities parse");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "malformed-path-query-or-body",
        malformed_parse.status == "400 Bad Request"
    );
    let empty_parse = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        "{}",
        &root_state,
    )
    .await
    .expect("empty capabilities parse");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "missing-empty-or-conflict-state",
        empty_parse.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&empty_parse.body).ok()
                == Some(serde_json::json!({"isSlskdn": false}))
    );
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("capabilities parse runtime database");
    let (runtime_parse_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_parse = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &runtime_parse_state,
    )
    .await
    .expect("capabilities parse with closed database");
    let runtime_parse_value = serde_json::from_str::<serde_json::Value>(&runtime_parse.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "runtime-failure-and-timeout",
        runtime_parse.status == "200 OK"
            && runtime_parse_value["isSlskdn"] == true
            && runtime_parse_value["flagsValue"] == 59
    );

    let before =
        crate::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &root_state)
            .await
            .expect("capability peers before parse mutation check");
    let mutation_parse = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &root_state,
    )
    .await
    .expect("capability parse mutation check");
    let after =
        crate::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &root_state)
            .await
            .expect("capability peers after parse mutation check");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "mutation-side-effects-and-readback",
        mutation_parse.status == "200 OK" && empty_peer_list(&before) && empty_peer_list(&after)
    );

    let (restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let restart_parse = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &restart_state,
    )
    .await
    .expect("capability parse before restart check");
    let (restarted_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let restarted_peers = crate::route_http_request(
        "GET",
        "/api/v0/capabilities/peers",
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("capability peers after restart check");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "restart-persistence-or-reset",
        restart_parse.status == "200 OK" && empty_peer_list(&restarted_peers)
    );

    let concurrent = futures_util::future::join_all((0..8).map(|_| {
        crate::route_http_request(
            "POST",
            "/api/v0/capabilities/parse",
            None,
            parse_body,
            &root_state,
        )
    }))
    .await;
    let first_body = concurrent
        .first()
        .and_then(|response| response.as_ref().ok())
        .map(|response| response.body.clone());
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response.as_ref().is_ok_and(|response| {
                response.status == "200 OK" && first_body.as_deref() == Some(response.body.as_str())
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_capabilities_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize capabilities ledger"),
    )
    .expect("write capabilities ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api capabilities mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the versioned Identity/Profile controller.  The
/// versioned surface is deliberately kept separate from slskR's legacy
/// username-oriented `/api/profile/*` compatibility shell.
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
pub(super) async fn controller_api_differential_native_profile_contracts() {
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

    let valid_profile = |response: &crate::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["peerId"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["publicKey"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["displayName"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["capabilities"].is_i64()
            && value["endpoints"].is_array()
            && value["createdAt"].is_string()
            && value["expiresAt"].is_string()
            && value["signature"].is_string()
    };
    let valid_lookup = |response: &crate::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["peerId"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["displayName"].is_string()
            && value["capabilities"].is_i64()
            && value["endpoints"].is_array()
            && value.get("publicKey").is_none()
            && value.get("signature").is_none()
    };
    let valid_invite = |response: &crate::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let friend_code = value["friendCode"].as_str().unwrap_or_default();
        let code_parts = friend_code.split('-').collect::<Vec<_>>();
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["inviteLink"]
                .as_str()
                .is_some_and(|value| value.starts_with("slskdn://invite/"))
            && code_parts.len() == 4
            && code_parts[0].len() == 5
            && code_parts[1].len() == 4
            && code_parts[2].len() == 4
            && code_parts[3].len() == 3
    };
    let update_body = r#"{"displayName":" Updated Profile ","avatar":" avatar.png ","capabilities":7,"endpoints":[{"type":"Direct","address":"https://profile.example","priority":1},{"type":" ","address":"","priority":2}]}"#;

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let me = crate::route_http_request("GET", "/api/v0/profile/me", None, "", &state)
        .await
        .expect("versioned own profile");
    record!(
        "GET",
        "/api/v0/profile/me",
        "missing-empty-or-conflict-state",
        valid_profile(&me)
    );
    let me_value = serde_json::from_str::<serde_json::Value>(&me.body).expect("own profile JSON");
    let peer_id = me_value["peerId"]
        .as_str()
        .expect("own profile peer ID")
        .to_owned();

    let malformed_me =
        crate::route_http_request("GET", "/api/v0/profile/me/extra", None, "", &state)
            .await
            .expect("malformed own profile path");
    record!(
        "GET",
        "/api/v0/profile/me",
        "malformed-path-query-or-body",
        malformed_me.status == "404 Not Found"
    );
    let known = crate::route_http_request(
        "GET",
        &format!("/api/v0/profile/{peer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("known profile lookup");
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "nominal-status-headers-body",
        valid_lookup(&known)
    );
    let malformed_peer = crate::route_http_request("GET", "/api/v0/profile/%20", None, "", &state)
        .await
        .expect("blank profile peer ID");
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "malformed-path-query-or-body",
        malformed_peer.status == "400 Bad Request"
    );

    let update = crate::route_http_request("PUT", "/api/v0/profile/me", None, update_body, &state)
        .await
        .expect("populated profile update");
    let updated_value =
        serde_json::from_str::<serde_json::Value>(&update.body).unwrap_or(serde_json::Value::Null);
    let populated_me = crate::route_http_request("GET", "/api/v0/profile/me", None, "", &state)
        .await
        .expect("populated own profile");
    let populated_peer = crate::route_http_request(
        "GET",
        &format!("/api/v0/profile/{peer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated profile lookup");
    record!(
        "GET",
        "/api/v0/profile/me",
        "populated-dynamic-state",
        valid_profile(&populated_me)
            && serde_json::from_str::<serde_json::Value>(&populated_me.body)
                .ok()
                .is_some_and(|value| value["displayName"] == "Updated Profile")
    );
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "populated-dynamic-state",
        valid_lookup(&populated_peer)
            && serde_json::from_str::<serde_json::Value>(&populated_peer.body)
                .ok()
                .is_some_and(|value| value["displayName"] == "Updated Profile")
    );
    let _ = updated_value;

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("profile runtime database");
    let (runtime_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_me =
        crate::route_http_request("GET", "/api/v0/profile/me", None, "", &runtime_state)
            .await
            .expect("own profile with closed database");
    record!(
        "GET",
        "/api/v0/profile/me",
        "runtime-failure-and-timeout",
        valid_profile(&runtime_me)
    );
    let runtime_me_value = serde_json::from_str::<serde_json::Value>(&runtime_me.body)
        .expect("runtime own profile JSON");
    let runtime_peer_id = runtime_me_value["peerId"]
        .as_str()
        .expect("runtime own profile peer ID");
    let runtime_peer = crate::route_http_request(
        "GET",
        &format!("/api/v0/profile/{runtime_peer_id}"),
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("profile lookup with closed database");
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "runtime-failure-and-timeout",
        valid_lookup(&runtime_peer)
    );

    let malformed_invite =
        crate::route_http_request("POST", "/api/v0/profile/invite", None, "null", &state)
            .await
            .expect("malformed profile invite");
    record!(
        "POST",
        "/api/v0/profile/invite",
        "malformed-path-query-or-body",
        malformed_invite.status == "400 Bad Request"
    );
    let empty_invite =
        crate::route_http_request("POST", "/api/v0/profile/invite", None, "{}", &state)
            .await
            .expect("empty profile invite");
    record!(
        "POST",
        "/api/v0/profile/invite",
        "missing-empty-or-conflict-state",
        valid_invite(&empty_invite)
    );
    let (restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let restarted_invite = crate::route_http_request(
        "POST",
        "/api/v0/profile/invite",
        None,
        r#"{"expiresInHours":2}"#,
        &restart_state,
    )
    .await
    .expect("profile invite after restart");
    record!(
        "POST",
        "/api/v0/profile/invite",
        "restart-persistence-or-reset",
        valid_invite(&restarted_invite)
    );
    let concurrent_invites = futures_util::future::join_all((0..8).map(|_| {
        crate::route_http_request(
            "POST",
            "/api/v0/profile/invite",
            None,
            r#"{"expiresInHours":1}"#,
            &state,
        )
    }))
    .await;
    record!(
        "POST",
        "/api/v0/profile/invite",
        "concurrency-and-idempotency",
        concurrent_invites
            .iter()
            .all(|response| { response.as_ref().is_ok_and(valid_invite) })
    );

    let malformed_update =
        crate::route_http_request("PUT", "/api/v0/profile/me", None, "null", &state)
            .await
            .expect("malformed profile update");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "malformed-path-query-or-body",
        malformed_update.status == "400 Bad Request"
    );
    let empty_update = crate::route_http_request("PUT", "/api/v0/profile/me", None, "{}", &state)
        .await
        .expect("empty profile update");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "missing-empty-or-conflict-state",
        empty_update.status == "400 Bad Request"
    );
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("profile update runtime database");
    let (runtime_update_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_update = crate::route_http_request(
        "PUT",
        "/api/v0/profile/me",
        None,
        update_body,
        &runtime_update_state,
    )
    .await
    .expect("profile update with closed database");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "runtime-failure-and-timeout",
        valid_profile(&runtime_update)
    );
    let restart_update = crate::route_http_request(
        "PUT",
        "/api/v0/profile/me",
        None,
        r#"{"displayName":"Restart Profile"}"#,
        &restart_state,
    )
    .await
    .expect("profile update after restart");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "restart-persistence-or-reset",
        valid_profile(&restart_update)
    );
    let concurrent_updates =
        futures_util::future::join_all((0..8).map(|index| {
            let body = format!(r#"{{"displayName":"Concurrent {index}"}}"#);
            let state = state.clone();
            async move {
                crate::route_http_request("PUT", "/api/v0/profile/me", None, &body, &state).await
            }
        }))
        .await;
    record!(
        "PUT",
        "/api/v0/profile/me",
        "concurrency-and-idempotency",
        concurrent_updates
            .iter()
            .all(|response| { response.as_ref().is_ok_and(valid_profile) })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_profile_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize profile ledger"),
    )
    .expect("write profile ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api profile mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining slskdN DestinationsController
/// edges.  The native controller binds only the case-insensitive `Path`
/// property, rejects null/empty/malformed bodies, derives its default
/// destination from configuration even if the backing state is empty, and
/// performs validation without persistence or database coupling.
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
pub(super) async fn controller_api_differential_native_destinations_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
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
                "pass": pass,
            }));
        }};
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-destination-edge-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create destination edge root");
    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_DOWNLOADS_DIR", &root.display().to_string())
    };
    let (state, _receiver) = test_state_with_env(env());
    {
        let mut destinations = state.destinations.write().await;
        destinations.records.clear();
    }

    let expected_default = serde_json::json!({
        "name": "Downloads",
        "path": root.display().to_string(),
        "isDefault": true,
        "exists": true,
    });
    let empty_list = crate::route_http_request("GET", "/api/v0/destinations", None, "", &state)
        .await
        .expect("empty slskdn destinations list");
    let empty_list_json =
        serde_json::from_str::<serde_json::Value>(&empty_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/destinations",
        "missing-empty-or-conflict-state",
        empty_list.status == "200 OK"
            && empty_list.content_type.starts_with("application/json")
            && empty_list_json == serde_json::json!([expected_default.clone()])
    );

    let empty_default =
        crate::route_http_request("GET", "/api/v0/destinations/default", None, "", &state)
            .await
            .expect("empty slskdn default destination");
    let empty_default_json =
        serde_json::from_str::<serde_json::Value>(&empty_default.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/destinations/default",
        "missing-empty-or-conflict-state",
        empty_default.status == "200 OK"
            && empty_default.content_type.starts_with("application/json")
            && empty_default_json == expected_default
    );

    let malformed_list = crate::route_http_request(
        "GET",
        "/api/v0/destinations/extra?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskdn destinations list path");
    record!(
        "GET",
        "/api/v0/destinations",
        "malformed-path-query-or-body",
        malformed_list.status == "404 Not Found"
    );

    let malformed_default = crate::route_http_request(
        "GET",
        "/api/v0/destinations/default/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskdn default destination path");
    record!(
        "GET",
        "/api/v0/destinations/default",
        "malformed-path-query-or-body",
        malformed_default.status == "404 Not Found"
    );

    {
        let mut destinations = state.destinations.write().await;
        *destinations = crate::DestinationStore::from_config(
            &state.config.downloads_dir,
            &state.config.core_workflow.destinations,
        );
    }
    let invalid_field = crate::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        &serde_json::json!({"destination": root.display().to_string()}).to_string(),
        &state,
    )
    .await
    .expect("invalid slskdn destination field");
    let empty_body =
        crate::route_http_request("POST", "/api/v0/destinations/validate", None, "", &state)
            .await
            .expect("empty slskdn destination body");
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "malformed-path-query-or-body",
        invalid_field.status == "400 Bad Request"
            && invalid_field.body.contains("Path is required")
    );
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "missing-empty-or-conflict-state",
        empty_body.status == "400 Bad Request" && empty_body.body.contains("Path is required")
    );

    let valid_body = serde_json::json!({"Path": root.display().to_string()}).to_string();
    let before_validation = state.destinations.read().await.list();
    let valid = crate::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        &valid_body,
        &state,
    )
    .await
    .expect("valid slskdn destination validation");
    let valid_json = serde_json::from_str::<serde_json::Value>(&valid.body).unwrap_or_default();
    let after_validation = state.destinations.read().await.list();
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "restart-persistence-or-reset",
        valid.status == "200 OK"
            && valid_json["path"] == root.display().to_string()
            && valid_json["exists"] == true
            && valid_json["writable"] == true
            && before_validation == after_validation
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("destinations failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(failure_db.clone()));
    {
        let mut destinations = failure_state.destinations.write().await;
        *destinations = crate::DestinationStore::from_config(
            &failure_state.config.downloads_dir,
            &failure_state.config.core_workflow.destinations,
        );
    }
    failure_db.close_for_test().await;
    let failure_list =
        crate::route_http_request("GET", "/api/v0/destinations", None, "", &failure_state)
            .await
            .expect("destinations list after database close");
    let failure_default = crate::route_http_request(
        "GET",
        "/api/v0/destinations/default",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("destination default after database close");
    let failure_validate = crate::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        &valid_body,
        &failure_state,
    )
    .await
    .expect("destination validation after database close");
    let failure_validate_json =
        serde_json::from_str::<serde_json::Value>(&failure_validate.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/destinations",
        "runtime-failure-and-timeout",
        failure_list.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_list.body)
                .ok()
                .and_then(|value| value.as_array().map(|records| records.len()))
                == Some(1)
    );
    record!(
        "GET",
        "/api/v0/destinations/default",
        "runtime-failure-and-timeout",
        failure_default.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_default.body)
                .ok()
                .is_some_and(|value| value["name"] == "Downloads")
    );
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "runtime-failure-and-timeout",
        failure_validate.status == "200 OK"
            && failure_validate_json["path"] == root.display().to_string()
    );

    let concurrent_before = state.destinations.read().await.list();
    let concurrent_responses = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/destinations/validate",
            None,
            &valid_body,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/destinations/validate",
            None,
            &valid_body,
            &state,
        ),
    ])
    .await;
    let concurrent_after = state.destinations.read().await.list();
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_before == concurrent_after
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_destinations_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn destinations edge ledger"),
    )
    .expect("write slskdn destinations edge ledger");
    let _ = fs::remove_dir_all(root);
    assert!(
        mismatches.is_empty(),
        "{} slskdn destinations edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
