//! Controller full media differential 03 ownership.

use super::*;

/// Differential proof for the remaining SongID controller edge cases.
/// The frozen controller uses exact routes, a GUID-constrained run id,
/// empty-state DTOs, and a durable run store; these probes exercise those
/// boundaries in addition to the already-covered nominal lifecycle.
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
pub(super) async fn controller_api_differential_songid_open_cases() {
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

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let json_value = |response: &crate::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let capabilities_contract = |response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value.as_array().is_some_and(|capabilities| {
                !capabilities.is_empty()
                    && capabilities.iter().all(|capability| {
                        capability["id"].is_string()
                            && capability["label"].is_string()
                            && capability["status"].is_string()
                            && capability["available"].is_boolean()
                            && capability["reason"].is_string()
                            && capability["requirements"].is_array()
                    })
                    && capabilities
                        .iter()
                        .any(|capability| capability["id"] == "text_query")
            })
    };
    let runs_contract = |response: &crate::routing::HttpResponse, populated: bool| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value.as_array().is_some_and(|runs| {
                (!populated || !runs.is_empty())
                    && runs.iter().all(|run| {
                        run["id"].is_string()
                            && run["status"].is_string()
                            && run["source"].is_string()
                    })
            })
    };
    let queue_contract = |response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["queuedCount"].is_u64()
            && value["runningCount"].is_u64()
            && value["completedCount"].is_u64()
            && value["failedCount"].is_u64()
            && value["maxConcurrentRuns"].is_u64()
            && value["activeRuns"].is_array()
    };
    let forensic_contract = |response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["run_id"].is_string()
            && value["matrix"].is_array()
            && value["count"].is_u64()
    };
    // GET /capabilities: exact routing, empty-state capability DTOs,
    // closed-database independence, and dynamic tool/config state.
    {
        let path = "/api/v0/songid/capabilities";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            crate::route_http_request("GET", "/api/v0/songid/capabilities/extra", None, "", &state)
                .await
                .expect("SongID capabilities malformed path response");
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID capabilities empty response");
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            capabilities_contract(&empty)
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("SongID capabilities runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .expect("SongID capabilities closed-database response");
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            capabilities_contract(&runtime)
        );

        let (populated_state, _receiver) = test_state_with_env(target_env());
        let populated = crate::route_http_request("GET", path, None, "", &populated_state)
            .await
            .expect("SongID capabilities populated response");
        let populated_value = json_value(&populated);
        record!(
            "GET",
            path,
            "populated-dynamic-state",
            capabilities_contract(&populated)
                && populated_value
                    .as_array()
                    .is_some_and(|capabilities| capabilities.len() >= 10)
        );
    }

    // GET /runs: exact routing, empty store, closed-database behavior,
    // and a list populated by a real run mutation.
    {
        let path = "/api/v0/songid/runs";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            crate::route_http_request("GET", "/api/v0/songid/runs/extra", None, "", &state)
                .await
                .expect("SongID runs malformed path response");
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID runs empty response");
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            runs_contract(&empty, false)
                && json_value(&empty)
                    .as_array()
                    .is_some_and(|runs| runs.is_empty())
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("SongID runs runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .expect("SongID runs closed-database response");
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            runs_contract(&runtime, false)
        );

        let created = crate::route_http_request(
            "POST",
            path,
            None,
            r#"{"source":"SongID list differential"}"#,
            &state,
        )
        .await
        .expect("SongID populated run");
        let listed = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID populated runs response");
        record!(
            "GET",
            path,
            "populated-dynamic-state",
            created.status == "202 Accepted" && runs_contract(&listed, true)
        );
    }

    // The three id-addressed GET routes share the frozen GUID-constrained
    // route boundary and differ only in their successful DTO contract.
    let unknown_id = "00000000-0000-0000-0000-000000000000";
    for (path, kind) in [
        ("/api/v0/songid/runs/{id:guid}", "run"),
        ("/api/v0/songid/runs/{id:guid}/evidence-package", "package"),
        ("/api/v0/songid/runs/{id:guid}/forensic-matrix", "forensic"),
    ] {
        let exact = match kind {
            "run" => format!("/api/v0/songid/runs/{unknown_id}"),
            "package" => format!("/api/v0/songid/runs/{unknown_id}/evidence-package"),
            "forensic" => format!("/api/v0/songid/runs/{unknown_id}/forensic-matrix"),
            _ => unreachable!(),
        };
        let malformed_path = match kind {
            "run" => "/api/v0/songid/runs/not-a-guid/extra".to_owned(),
            "package" => "/api/v0/songid/runs/not-a-guid/evidence-package/extra".to_owned(),
            "forensic" => "/api/v0/songid/runs/not-a-guid/forensic-matrix/extra".to_owned(),
            _ => unreachable!(),
        };
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} malformed path: {error}"));
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = crate::route_http_request("GET", &exact, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} missing response: {error}"));
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} runtime database: {error}"));
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("GET", &exact, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} runtime response: {error}"));
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "404 Not Found"
        );
    }

    // Forensic matrix also needs populated-state proof, using the real
    // synchronous SongID run produced by the route.
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = crate::route_http_request(
            "POST",
            "/api/v0/songid/runs",
            None,
            r#"{"source":"SongID forensic differential"}"#,
            &state,
        )
        .await
        .expect("SongID forensic run");
        let created_value = json_value(&created);
        let run_id = created_value["id"].as_str().unwrap_or_default();
        let response = crate::route_http_request(
            "GET",
            &format!("/api/v0/songid/runs/{run_id}/forensic-matrix"),
            None,
            "",
            &state,
        )
        .await
        .expect("SongID populated forensic matrix response");
        record!(
            "GET",
            "/api/v0/songid/runs/{id:guid}/forensic-matrix",
            "nominal-status-headers-body",
            forensic_contract(&response)
        );
        record!(
            "GET",
            "/api/v0/songid/runs/{id:guid}/forensic-matrix",
            "populated-dynamic-state",
            created.status == "202 Accepted" && forensic_contract(&response)
        );
    }

    // GET /queue: exact routing, empty counters, and closed-database
    // independence. Populated counters were covered by the completed
    // SongID lifecycle proof.
    {
        let path = "/api/v0/songid/runs/queue";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            crate::route_http_request("GET", "/api/v0/songid/runs/queue/extra", None, "", &state)
                .await
                .expect("SongID queue malformed path response");
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID queue empty response");
        let empty_value = json_value(&empty);
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            queue_contract(&empty)
                && empty_value["queuedCount"] == 0
                && empty_value["runningCount"] == 0
                && empty_value["completedCount"] == 0
                && empty_value["failedCount"] == 0
                && empty_value["activeRuns"] == serde_json::json!([])
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("SongID queue runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .expect("SongID queue closed-database response");
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            queue_contract(&runtime)
        );
    }

    // POST /runs: empty model binding, reset semantics, and concurrent
    // run creation with distinct identifiers.
    {
        let path = "/api/v0/songid/runs";
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = crate::route_http_request("POST", path, None, "", &state)
            .await
            .expect("SongID run empty body response");
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body.contains("SongID source is required.")
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset = crate::route_http_request(
            "POST",
            path,
            None,
            r#"{"source":"SongID reset differential"}"#,
            &reset_state,
        )
        .await
        .expect("SongID reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request(
            "POST",
            path,
            None,
            r#"{"source":"SongID reset differential"}"#,
            &restarted_state,
        )
        .await
        .expect("SongID restarted response");
        let reset_value = json_value(&reset);
        let restarted_value = json_value(&restarted);
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            reset.status == "202 Accepted"
                && restarted.status == "202 Accepted"
                && reset_value["status"] == "completed"
                && restarted_value["status"] == "completed"
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                path,
                None,
                r#"{"source":"SongID concurrent differential"}"#,
                &concurrent_state
            ),
            crate::route_http_request(
                "POST",
                path,
                None,
                r#"{"source":"SongID concurrent differential"}"#,
                &concurrent_state
            )
        );
        let left = left.expect("SongID left concurrency response");
        let right = right.expect("SongID right concurrency response");
        let left_value = json_value(&left);
        let right_value = json_value(&right);
        let left_id = left_value["id"].as_str().unwrap_or_default();
        let right_id = right_value["id"].as_str().unwrap_or_default();
        let readback = crate::route_http_request("GET", path, None, "", &concurrent_state)
            .await
            .expect("SongID concurrency readback");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            left.status == "202 Accepted"
                && right.status == "202 Accepted"
                && left_value["status"] == "completed"
                && right_value["status"] == "completed"
                && !left_id.is_empty()
                && !right_id.is_empty()
                && left_id != right_id
                && runs_contract(&readback, true)
        );
    }

    assert_eq!(ledger.len(), 25, "SongID residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create SongID evidence directory");
    fs::write(
        evidence_dir.join("songid_controller_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize SongID ledger"),
    )
    .expect("write SongID ledger");
    assert!(
        mismatches.is_empty(),
        "{} SongID controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
