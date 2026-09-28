//! Controller full storage differential 02 ownership.

use super::*;

/// Differential proof for the remaining slskdN BackfillController cases.
/// The scheduler is process-local except for its HashDb-backed candidate
/// and file operations, so the ledger separates closed-storage failures
/// from the successful config, state, reset, and concurrency contracts.
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
pub(super) async fn controller_api_differential_backfill_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_TRANSFER_ALLOW_OUTBOUND", "false");
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

    macro_rules! seed_candidates {
        ($state:expr) => {{
            let state = &$state;
            state
                .searches
                .write()
                .await
                .records
                .push(crate::SearchRecord {
                    id: "backfill-residual-search".to_owned(),
                    token: 1,
                    query: "backfill residual".to_owned(),
                    target: "global",
                    target_name: None,
                    status: "completed",
                    results: vec![crate::SearchResultEntry {
                        peer_username: Some("backfill-residual-peer".to_owned()),
                        filename: "Library/BackfillResidual.flac".to_owned(),
                        size: 98_765,
                        extension: "flac".to_owned(),
                        bit_rate: None,
                        sample_rate: None,
                        bit_depth: None,
                        length_seconds: None,
                        locked: false,
                        slot_free: Some(true),
                        average_speed: Some(1_000),
                        queue_length: Some(0),
                    }],
                    raw_response_count: 1,
                    filtered_out_count: 0,
                    ignored_result_count: 0,
                    hidden_locked_count: 0,
                    fallback_attempts: 0,
                    ttl_seconds: crate::DEFAULT_SEARCH_TTL_SECONDS,
                    expires_at: 0,
                    created_at: 1,
                    updated_at: 1,
                });
            state.users.write().await.records.push(crate::UserRecord {
                username: "backfill-residual-peer".to_owned(),
                watched: false,
                status: Some("online".to_owned()),
                privileged: false,
                average_speed: None,
                upload_count: None,
                file_count: None,
                directory_count: None,
                updated_at: crate::unix_timestamp(),
            });
            state
                .mesh
                .write()
                .await
                .capability_records
                .push(test_capability_descriptor(
                    "backfill-residual-peer",
                    vec![slskr_client::capabilities::FEATURE_CAPABILITIES_V1.to_owned()],
                ));
        }};
    }

    async fn closed_db_state(env: &MapEnv) -> Arc<crate::AppState> {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("backfill residual database");
        let (state, _receiver) =
            test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        state
    }

    async fn response(
        method: &str,
        path: &str,
        body: &str,
        state: &Arc<crate::AppState>,
    ) -> crate::HttpResponse {
        crate::route_http_request(method, path, None, body, state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"))
    }

    // GET /candidates: invalid binding, empty state, closed HashDb, and
    // a populated candidate with the frozen DTO field names and types.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let invalid = response(
            "GET",
            "/api/v0/backfill/candidates?limit=invalid",
            "",
            &state,
        )
        .await;
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "malformed-path-query-or-body",
            invalid.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("GET", "/api/v0/backfill/candidates", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&empty.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && value == serde_json::json!({"count": 0, "candidates": []})
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let failed = response("GET", "/api/v0/backfill/candidates", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "runtime-failure-and-timeout",
            failed.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_candidates!(state);
        let populated = response("GET", "/api/v0/backfill/candidates", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&populated.body)
            .unwrap_or(serde_json::Value::Null);
        let candidate = value
            .get("candidates")
            .and_then(serde_json::Value::as_array)
            .and_then(|entries| entries.first())
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && value["count"] == 1
                && candidate["fileId"]
                    == crate::content_discovery::generate_flac_key(
                        "Library/BackfillResidual.flac",
                        98_765,
                    )
                && candidate["peerId"] == "backfill-residual-peer"
                && candidate["path"] == "Library/BackfillResidual.flac"
                && candidate["size"] == 98_765
                && candidate["discoveredAt"].is_string()
                && candidate["peerBackfillsToday"] == 0
                && candidate["isPeerOnline"] == true
                && candidate["isPeerSlskdn"] == true
        );
    }

    // GET /config: exact default DTO, malformed extra segment, closed
    // storage independence, and a live enabled-state projection.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("GET", "/api/v0/backfill/config/extra", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/config",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("GET", "/api/v0/backfill/config", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&empty.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/config",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && value
                    == serde_json::json!({
                        "maxGlobalConnections": 2,
                        "maxPerPeerPerDay": 10,
                        "maxHeaderBytes": 65_536,
                        "minIdleTimeSeconds": 300,
                        "runIntervalSeconds": 600,
                        "transferTimeoutSeconds": 30,
                        "enabled": true,
                    })
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("GET", "/api/v0/backfill/config", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/config",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body).is_ok()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.backfill.write().await.enabled = false;
        let populated = response("GET", "/api/v0/backfill/config", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&populated.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/config",
            "populated-dynamic-state",
            populated.status == "200 OK" && value["enabled"] == false
        );
    }

    // GET /stats: empty/default, malformed path, closed-storage
    // independence, and populated counters with .NET-compatible time
    // representations.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("GET", "/api/v0/backfill/stats/extra", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("GET", "/api/v0/backfill/stats", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&empty.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && value["totalAttempts"] == 0
                && value["successful"] == 0
                && value["failed"] == 0
                && value["rateLimited"] == 0
                && value["active"] == 0
                && value["hashesDiscovered"] == 0
                && value["isIdle"] == false
                && value["lastCycleTime"].is_null()
                && value["nextCycleTime"].is_null()
                && value["idleDuration"].is_null()
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("GET", "/api/v0/backfill/stats", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body).is_ok()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        {
            let mut backfill = state.backfill.write().await;
            backfill.total_attempts = 3;
            backfill.successful = 2;
            backfill.failed = 1;
            backfill.rate_limited = 4;
            backfill.active = 1;
            backfill.hashes_discovered = 2;
            backfill.last_cycle_time = Some(1);
            backfill.next_cycle_time = Some(2);
            backfill.is_idle = true;
            backfill.idle_since = Some(1);
        }
        let populated = response("GET", "/api/v0/backfill/stats", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&populated.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && value["totalAttempts"] == 3
                && value["successful"] == 2
                && value["failed"] == 1
                && value["rateLimited"] == 4
                && value["active"] == 1
                && value["hashesDiscovered"] == 2
                && value["isIdle"] == true
                && value["lastCycleTime"].is_string()
                && value["nextCycleTime"].is_string()
                && value["idleDuration"].is_string()
        );
    }

    // POST /enable.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let nominal = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&nominal.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && value["enabled"] == false
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response(
            "POST",
            "/api/v0/backfill/enable?enabled=invalid",
            "",
            &state,
        )
        .await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("POST", "/api/v0/backfill/enable", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&empty.body)
                    .is_ok_and(|value| value["enabled"] == true)
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body)
                    .is_ok_and(|value| value["enabled"] == false)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let changed = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        let config = response("GET", "/api/v0/backfill/config", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "mutation-side-effects-and-readback",
            changed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&config.body)
                    .is_ok_and(|value| value["enabled"] == false)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let changed = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let config = response("GET", "/api/v0/backfill/config", "", &restarted).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "restart-persistence-or-reset",
            changed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&config.body)
                    .is_ok_and(|value| value["enabled"] == true)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            response("POST", "/api/v0/backfill/enable?enabled=false", "", &state),
            response("POST", "/api/v0/backfill/enable?enabled=false", "", &state)
        );
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "concurrency-and-idempotency",
            left.status == "200 OK"
                && right.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&left.body)
                    .is_ok_and(|value| value["enabled"] == false)
                && serde_json::from_str::<serde_json::Value>(&right.body)
                    .is_ok_and(|value| value["enabled"] == false)
        );
    }

    // POST /idle and /busy share process-local state semantics.
    for (route, set_idle, expected) in [
        ("/api/v0/backfill/idle", true, true),
        ("/api/v0/backfill/busy", false, false),
    ] {
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let nominal = response("POST", route, "", &state).await;
            record!(
                "POST",
                route,
                "nominal-status-headers-body",
                nominal.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&nominal.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let malformed = response("POST", &format!("{route}/extra"), "", &state).await;
            record!(
                "POST",
                route,
                "malformed-path-query-or-body",
                malformed.status == "404 Not Found"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let empty = response("POST", route, "", &state).await;
            record!(
                "POST",
                route,
                "missing-empty-or-conflict-state",
                empty.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&empty.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
        {
            let state = closed_db_state(&base_env).await;
            let closed = response("POST", route, "", &state).await;
            record!(
                "POST",
                route,
                "runtime-failure-and-timeout",
                closed.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&closed.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let first = response(
                "POST",
                if set_idle {
                    "/api/v0/backfill/idle"
                } else {
                    "/api/v0/backfill/busy"
                },
                "",
                &state,
            )
            .await;
            let second = response(
                "POST",
                if set_idle {
                    "/api/v0/backfill/busy"
                } else {
                    "/api/v0/backfill/idle"
                },
                "",
                &state,
            )
            .await;
            let stats = response("GET", "/api/v0/backfill/stats", "", &state).await;
            record!(
                "POST",
                route,
                "mutation-side-effects-and-readback",
                first.status == "200 OK"
                    && second.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&stats.body)
                        .is_ok_and(|value| value["isIdle"] == !set_idle)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let changed = response("POST", route, "", &state).await;
            let (restarted, _receiver) = test_state_with_env(base_env.clone());
            let stats = response("GET", "/api/v0/backfill/stats", "", &restarted).await;
            record!(
                "POST",
                route,
                "restart-persistence-or-reset",
                changed.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&stats.body)
                        .is_ok_and(|value| value["isIdle"] == false)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let (left, right) = tokio::join!(
                response("POST", route, "", &state),
                response("POST", route, "", &state)
            );
            record!(
                "POST",
                route,
                "concurrency-and-idempotency",
                left.status == "200 OK"
                    && right.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&left.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
                    && serde_json::from_str::<serde_json::Value>(&right.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
    }

    // POST /file: validation, process-local attempt accounting, reset,
    // closed HashDb failure, and concurrent valid requests.
    let file_body = r#"{"peerId":"backfill-residual-peer","path":"Library/BackfillResidual.flac","size":98765}"#;
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let nominal = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&nominal.body)
                    .is_ok_and(|value| value["peerId"] == "backfill-residual-peer")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("POST", "/api/v0/backfill/file", "not-json", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let missing = response("POST", "/api/v0/backfill/file", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let failed = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "runtime-failure-and-timeout",
            failed.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        let stats = response("GET", "/api/v0/backfill/stats", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "mutation-side-effects-and-readback",
            mutation.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&stats.body)
                    .is_ok_and(|value| value["totalAttempts"] == 1)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let stats = response("GET", "/api/v0/backfill/stats", "", &restarted).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "restart-persistence-or-reset",
            mutation.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&stats.body)
                    .is_ok_and(|value| value["totalAttempts"] == 0)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            response("POST", "/api/v0/backfill/file", file_body, &state),
            response("POST", "/api/v0/backfill/file", file_body, &state)
        );
        record!(
            "POST",
            "/api/v0/backfill/file",
            "concurrency-and-idempotency",
            left.status == "200 OK" && right.status == "200 OK"
        );
    }

    // POST /trigger: the scheduler returns a result envelope even when
    // the candidate set is empty or HashDb throws; the state mutation is
    // visible in stats and is reset on a fresh process.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let nominal = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&nominal.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && value["candidatesEvaluated"] == 0
                && value["backfillsAttempted"] == 0
                && value["results"].is_array()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("POST", "/api/v0/backfill/trigger/extra", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&empty.body)
                    .is_ok_and(|value| value["candidatesEvaluated"] == 0)
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body)
                    .is_ok_and(|value| value["candidatesEvaluated"] == 0)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        let stats = response("GET", "/api/v0/backfill/stats", "", &state).await;
        let stats = serde_json::from_str::<serde_json::Value>(&stats.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "mutation-side-effects-and-readback",
            mutation.status == "200 OK"
                && stats["lastCycleTime"].is_string()
                && stats["nextCycleTime"].is_string()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let stats = response("GET", "/api/v0/backfill/stats", "", &restarted).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "restart-persistence-or-reset",
            mutation.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&stats.body)
                    .is_ok_and(|value| value["lastCycleTime"].is_null())
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            response("POST", "/api/v0/backfill/trigger", "", &state),
            response("POST", "/api/v0/backfill/trigger", "", &state)
        );
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "concurrency-and-idempotency",
            left.status == "200 OK"
                && right.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&left.body).is_ok()
                && serde_json::from_str::<serde_json::Value>(&right.body).is_ok()
        );
    }

    assert_eq!(ledger.len(), 47, "Backfill residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Backfill evidence directory");
    fs::write(
        evidence_dir.join("backfill_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Backfill ledger"),
    )
    .expect("write Backfill ledger");
    assert!(
        mismatches.is_empty(),
        "{} Backfill residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the residual slskdn FilesController cases.
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
pub(super) async fn controller_api_differential_files_open_cases() {
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

    let json_value = |response: &crate::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let delete_cases = [
        (
            "downloads",
            "directories",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "directories",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ];

    for (storage, _resource, route_template) in delete_cases {
        let prefix = if storage == "downloads" {
            "/api/v0/files/downloads/directories/"
        } else {
            "/api/v0/files/incomplete/directories/"
        };
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let path = file_storage_root(&state, storage).join("Nominal");
            fs::create_dir_all(&path).expect("create nominal directory");
            fs::write(path.join("file.txt"), b"data").expect("create nominal file");
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Nominal")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "nominal-status-headers-body",
                response.status == "204 No Content" && response.body.is_empty()
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response =
                conversation_request(&state, "DELETE", &format!("{prefix}not-base64!"), "").await;
            record!(
                "DELETE",
                route_template,
                "malformed-path-query-or-body",
                response.status == "400 Bad Request"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Missing")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "missing-empty-or-conflict-state",
                response.status == "404 Not Found"
            );
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Runtime")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let path = file_storage_root(&state, storage).join("Mutation");
            fs::create_dir_all(&path).expect("create mutation directory");
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Mutation")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "mutation-side-effects-and-readback",
                response.status == "204 No Content" && !path.exists()
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Restart")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "restart-persistence-or-reset",
                response.status == "404 Not Found"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let path = file_storage_root(&state, storage).join("Concurrent");
            fs::create_dir_all(&path).expect("create concurrent directory");
            let request_path = format!("{prefix}{}", file_path_segment("Concurrent"));
            let responses = futures_util::future::join_all([
                conversation_request(&state, "DELETE", &request_path, ""),
                conversation_request(&state, "DELETE", &request_path, ""),
            ])
            .await;
            let statuses = responses
                .iter()
                .map(|response| response.status)
                .collect::<Vec<_>>();
            record!(
                "DELETE",
                route_template,
                "concurrency-and-idempotency",
                statuses.contains(&"204 No Content") && statuses.contains(&"404 Not Found")
            );
        }
    }

    for (storage, route_template) in [
        (
            "downloads",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "incomplete",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
    ] {
        let prefix = if storage == "downloads" {
            "/api/v0/files/downloads/files/"
        } else {
            "/api/v0/files/incomplete/files/"
        };
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(&root).expect("create file storage root");
            let path = root.join("Nominal.txt");
            fs::write(&path, b"data").expect("create nominal storage file");
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Nominal.txt")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "nominal-status-headers-body",
                response.status == "204 No Content" && response.body.is_empty()
            );
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Runtime.txt")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Restart.txt")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "restart-persistence-or-reset",
                response.status == "204 No Content"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(&root).expect("create concurrent file root");
            fs::write(root.join("Concurrent.txt"), b"data").expect("create concurrent file");
            let request_path = format!("{prefix}{}", file_path_segment("Concurrent.txt"));
            let responses = futures_util::future::join_all([
                conversation_request(&state, "DELETE", &request_path, ""),
                conversation_request(&state, "DELETE", &request_path, ""),
            ])
            .await;
            let statuses = responses
                .iter()
                .map(|response| response.status)
                .collect::<Vec<_>>();
            record!(
                "DELETE",
                route_template,
                "concurrency-and-idempotency",
                statuses.iter().all(|status| status == &"204 No Content")
            );
        }
    }

    {
        let storage = "incomplete";
        let route_template = "/api/v0/files/incomplete/files/{base64FileName}";
        let prefix = "/api/v0/files/incomplete/files/";
        let (state, _receiver) = test_state_with_env(file_path_env(target, true));
        let malformed =
            conversation_request(&state, "DELETE", &format!("{prefix}not-base64!"), "").await;
        record!(
            "DELETE",
            route_template,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = conversation_request(
            &state,
            "DELETE",
            &format!("{prefix}{}", file_path_segment("Missing.txt")),
            "",
        )
        .await;
        record!(
            "DELETE",
            route_template,
            "missing-empty-or-conflict-state",
            missing.status == "204 No Content"
        );
        let root = file_storage_root(&state, storage);
        fs::create_dir_all(&root).expect("create incomplete mutation root");
        let path = root.join("Mutation.txt");
        fs::write(&path, b"data").expect("create incomplete mutation file");
        let mutation = conversation_request(
            &state,
            "DELETE",
            &format!("{prefix}{}", file_path_segment("Mutation.txt")),
            "",
        )
        .await;
        record!(
            "DELETE",
            route_template,
            "mutation-side-effects-and-readback",
            mutation.status == "204 No Content" && !path.exists()
        );
    }

    for storage in ["downloads", "incomplete"] {
        let (collection_route, detail_prefix, collection_template, detail_template) =
            if storage == "downloads" {
                (
                    "/api/v0/files/downloads/directories",
                    "/api/v0/files/downloads/directories/",
                    "/api/v0/files/downloads/directories",
                    "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
                )
            } else {
                (
                    "/api/v0/files/incomplete/directories",
                    "/api/v0/files/incomplete/directories/",
                    "/api/v0/files/incomplete/directories",
                    "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
                )
            };
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let response = conversation_request(
                &state,
                "GET",
                &format!("{collection_route}?recursive=maybe"),
                "",
            )
            .await;
            record!(
                "GET",
                collection_template,
                "malformed-path-query-or-body",
                response.status == "400 Bad Request"
            );
            let empty = conversation_request(&state, "GET", collection_route, "").await;
            record!(
                "GET",
                collection_template,
                "missing-empty-or-conflict-state",
                empty.status == "200 OK" && json_value(&empty)["files"].is_array()
            );
            if storage == "incomplete" {
                record!(
                    "GET",
                    collection_template,
                    "nominal-status-headers-body",
                    empty.status == "200 OK" && json_value(&empty)["files"].is_array()
                );
            }
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(&state, "GET", collection_route, "").await;
            record!(
                "GET",
                collection_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(root.join("Album")).expect("create populated root");
            fs::write(root.join("Album").join("Track.flac"), b"track")
                .expect("create populated track");
            let response = conversation_request(&state, "GET", collection_route, "").await;
            if storage == "incomplete" {
                record!(
                    "GET",
                    collection_template,
                    "populated-dynamic-state",
                    response.status == "200 OK"
                        && json_value(&response)["directories"]
                            .as_array()
                            .is_some_and(|directories| !directories.is_empty())
                );
            }
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(root.join("Album")).expect("create nominal detail root");
            fs::write(root.join("Album").join("Track.flac"), b"track")
                .expect("create nominal detail track");
            let response = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Album")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "nominal-status-headers-body",
                response.status == "200 OK" && json_value(&response)["files"].is_array()
            );
            let malformed =
                conversation_request(&state, "GET", &format!("{detail_prefix}not-base64!"), "")
                    .await;
            record!(
                "GET",
                detail_template,
                "malformed-path-query-or-body",
                malformed.status == "400 Bad Request"
            );
            let missing = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Missing")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "missing-empty-or-conflict-state",
                missing.status == "404 Not Found"
            );
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Runtime")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let root = file_storage_root(&state, storage);
            let album = root.join("Album");
            fs::create_dir_all(&album).expect("create populated detail root");
            fs::write(album.join("Track.flac"), b"track").expect("create populated detail file");
            let response = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Album")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "populated-dynamic-state",
                response.status == "200 OK"
                    && json_value(&response)["files"]
                        .as_array()
                        .is_some_and(|files| files.len() == 1)
            );
        }
    }

    assert_eq!(ledger.len(), 43, "files residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create files evidence directory");
    fs::write(
        evidence_dir.join("files_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize files ledger"),
    )
    .expect("write files ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api files mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
