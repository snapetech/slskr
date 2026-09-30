//! Controller full telemetry differential ownership.

use super::*;

/// Differential evidence for the slskdN compatibility user-browse route.
/// The route projects the real browse store into the legacy directory/file
/// DTO, rejects malformed route shapes, returns not-found for absent users,
/// and remains process-local when SQLite is unavailable.
/// Differential evidence for the slskdN swarm trace summary projection.
/// The route is process-local/file-backed in the frozen controller, so a
/// closed unrelated SQLite pool must not change its empty response.
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
pub(super) async fn controller_api_differential_traces_summary_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET /api/v0/traces/{{jobId}}/summary [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": "/api/v0/traces/{jobId}/summary",
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let malformed = crate::route_http_request("GET", "/api/v0/traces//summary", None, "", &state)
        .await
        .expect("malformed trace summary route");
    record!(
        "malformed-path-query-or-body",
        malformed.status == "404 Not Found"
    );

    let nominal = crate::route_http_request(
        "GET",
        "/api/v0/traces/trace-empty/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("nominal empty trace summary route");
    let nominal_json =
        serde_json::from_str::<serde_json::Value>(&nominal.body).unwrap_or(serde_json::Value::Null);
    record!(
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type == "application/json"
            && nominal_json["jobId"] == "trace-empty"
            && nominal_json["totalEvents"] == 0
            && nominal_json["eventCounts"].is_object()
            && nominal_json["bytesBySource"].is_object()
            && nominal_json["bytesByBackend"].is_object()
            && nominal_json["peers"].as_array().is_some_and(Vec::is_empty)
            && nominal_json["rescueInvoked"] == false
    );

    let now = crate::unix_timestamp();
    state
        .multisource
        .write()
        .await
        .insert(crate::multisource::SwarmJob {
            id: "trace-populated".to_owned(),
            status: "completed".to_owned(),
            filename: "trace.flac".to_owned(),
            output_path: "trace.flac".to_owned(),
            file_size: 1_024,
            chunk_size: 512,
            sources: vec!["trace-peer".to_owned()],
            completed_chunks: 2,
            total_chunks: 2,
            bytes_downloaded: 1_024,
            created_at: now,
            updated_at: now + 1,
            result: Some(crate::multisource::SwarmResult {
                id: "trace-populated".to_owned(),
                success: true,
                filename: "trace.flac".to_owned(),
                output_path: "trace.flac".to_owned(),
                bytes_downloaded: 1_024,
                total_time_ms: 100,
                sources_used: 1,
                final_hash: "11".repeat(32),
                chunks: vec![
                    crate::multisource::ChunkResult {
                        index: 0,
                        username: "trace-peer".to_owned(),
                        start_offset: 0,
                        end_offset: 511,
                        bytes_downloaded: 512,
                        time_ms: 40,
                    },
                    crate::multisource::ChunkResult {
                        index: 1,
                        username: "trace-peer".to_owned(),
                        start_offset: 512,
                        end_offset: 1_023,
                        bytes_downloaded: 512,
                        time_ms: 60,
                    },
                ],
                error: None,
            }),
        });
    let populated = crate::route_http_request(
        "GET",
        "/api/v0/traces/trace-populated/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("populated trace summary route");
    let populated_json = serde_json::from_str::<serde_json::Value>(&populated.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "populated-dynamic-state",
        populated.status == "200 OK"
            && populated_json["jobId"] == "trace-populated"
            && populated_json["firstEventAt"] == now
            && populated_json["lastEventAt"] == now + 1
            && populated_json["duration"] == 1
            && populated_json["totalEvents"] == 2
    );

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("trace runtime-failure database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default(),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime = crate::route_http_request(
        "GET",
        "/api/v0/traces/runtime-missing/summary",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("trace summary with closed unrelated database");
    record!(
        "runtime-failure-and-timeout",
        runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime.body)
                .map(|value| value["totalEvents"] == 0)
                .unwrap_or(false)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create trace evidence directory");
    fs::write(
        evidence_dir.join("traces_summary_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize trace evidence"),
    )
    .expect("write trace evidence");
    assert!(
        mismatches.is_empty(),
        "{} trace summary mismatches: {:?}",
        mismatches.len(),
        mismatches
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
pub(super) async fn controller_api_differential_telemetry_open_cases() {
    let target = "slskdn";
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

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };

    let (malformed_state, _receiver) = test_state_with_env(env());
    for (path, route) in [
        (
            "/api/v0/telemetry/metrics/extra",
            "/api/v0/telemetry/metrics",
        ),
        (
            "/api/v0/telemetry/metrics/kpi/extra",
            "/api/v0/telemetry/metrics/kpi",
        ),
        (
            "/api/v0/telemetry/prometheus/extra",
            "/api/v0/telemetry/prometheus",
        ),
        (
            "/api/v0/telemetry/prometheus/kpis/extra",
            "/api/v0/telemetry/prometheus/kpis",
        ),
        (
            "/api/v0/telemetry/reports/transfers/directories/extra",
            "/api/v0/telemetry/reports/transfers/directories",
        ),
        (
            "/api/v0/telemetry/reports/transfers/histogram/extra",
            "/api/v0/telemetry/reports/transfers/histogram",
        ),
        (
            "/api/v0/telemetry/reports/transfers/summary/extra",
            "/api/v0/telemetry/reports/transfers/summary",
        ),
        (
            "/api/v0/telemetry/reports/transfers/users/",
            "/api/v0/telemetry/reports/transfers/users/{username}",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &malformed_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let (empty_state, _receiver) = test_state_with_env(env());
    let metrics =
        crate::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &empty_state)
            .await
            .expect("empty telemetry metrics response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "missing-empty-or-conflict-state",
        metrics.status == "200 OK"
            && metrics.content_type.starts_with("text/plain")
            && metrics.body.contains("slskr_telemetry_transfers 0")
    );
    let metrics_kpi = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty telemetry KPI response");
    let metrics_kpi_json = parse_json(&metrics_kpi.body);
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        "missing-empty-or-conflict-state",
        metrics_kpi.status == "200 OK"
            && metrics_kpi_json["slskr_transfers"]["samples"][0]["value"] == 0.0
    );
    let prometheus = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty telemetry Prometheus response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus",
        "missing-empty-or-conflict-state",
        prometheus.status == "200 OK"
            && prometheus.content_type.starts_with("text/plain")
            && prometheus.body.contains("slskr_transfers 0")
    );
    let prometheus_kpis = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty telemetry Prometheus KPI response");
    let prometheus_kpis_json = parse_json(&prometheus_kpis.body);
    record!(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        "missing-empty-or-conflict-state",
        prometheus_kpis.status == "200 OK"
            && prometheus_kpis_json["slskr_transfers"]["samples"][0]["value"] == 0.0
    );
    let directories = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer directories response");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "missing-empty-or-conflict-state",
        directories.status == "200 OK" && parse_json(&directories.body).is_array()
    );
    let histogram = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer histogram response");
    let histogram_json = parse_json(&histogram.body);
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram",
        "missing-empty-or-conflict-state",
        histogram.status == "200 OK"
            && histogram_json["2100-01-01T00:00:00Z"]["Download"].is_object()
    );
    let summary = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer summary response");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        "missing-empty-or-conflict-state",
        summary.status == "200 OK"
            && parse_json(&summary.body) == serde_json::json!({"Download": {}, "Upload": {}})
    );
    let user = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/unknown",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer user response");
    let user_json = parse_json(&user.body);
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "missing-empty-or-conflict-state",
        user.status == "200 OK" && user_json["username"] == "unknown" && user_json["count"] == 0
    );

    let (populated_state, _receiver) = test_state_with_env(env());
    {
        let mut transfers = populated_state.transfers.write().await;
        transfers.create(
            0,
            Some("telemetry-open-peer".to_owned()),
            "Remote/Open.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.create(
            1,
            Some("telemetry-open-peer".to_owned()),
            "Shared/Open.flac".to_owned(),
            None,
            Some(20),
        );
    }
    let populated_metrics = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/metrics",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry metrics response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "populated-dynamic-state",
        populated_metrics.status == "200 OK"
            && populated_metrics
                .body
                .contains("slskr_telemetry_transfers 2")
    );
    let populated_metrics_kpi = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry KPI response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        "populated-dynamic-state",
        parse_json(&populated_metrics_kpi.body)["slskr_transfers"]["samples"][0]["value"] == 2.0
    );
    let populated_prometheus = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry Prometheus response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus",
        "populated-dynamic-state",
        populated_prometheus.status == "200 OK"
            && populated_prometheus.body.contains("slskr_transfers 2")
    );
    let populated_prometheus_kpis = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry Prometheus KPI response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        "populated-dynamic-state",
        parse_json(&populated_prometheus_kpis.body)["slskr_transfers"]["samples"][0]["value"]
            == 2.0
    );

    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("telemetry open cases runtime database");
    let (runtime_state, _receiver) = test_state_with_env_parts(
        env().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    for (path, route, case) in [
        (
            "/api/v0/telemetry/metrics",
            "/api/v0/telemetry/metrics",
            "metrics",
        ),
        (
            "/api/v0/telemetry/prometheus",
            "/api/v0/telemetry/prometheus",
            "prometheus",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && if case == "metrics" {
                    response.body.contains("slskr_telemetry_transfers")
                } else {
                    response.body.contains("slskr_transfers")
                }
        );
    }
    for (path, route) in [
        (
            "/api/v0/telemetry/metrics/kpi",
            "/api/v0/telemetry/metrics/kpi",
        ),
        (
            "/api/v0/telemetry/prometheus/kpis",
            "/api/v0/telemetry/prometheus/kpis",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).is_object()
        );
    }
    for (path, route) in [
        (
            "/api/v0/telemetry/reports/transfers/summary",
            "/api/v0/telemetry/reports/transfers/summary",
        ),
        (
            "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
            "/api/v0/telemetry/reports/transfers/histogram",
        ),
        (
            "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download",
            "/api/v0/telemetry/reports/transfers/leaderboard",
        ),
        (
            "/api/v0/telemetry/reports/transfers/users/runtime-peer",
            "/api/v0/telemetry/reports/transfers/users/{username}",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions?direction=Download",
            "/api/v0/telemetry/reports/transfers/exceptions",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download",
            "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        ),
        (
            "/api/v0/telemetry/reports/transfers/directories",
            "/api/v0/telemetry/reports/transfers/directories",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    assert_eq!(ledger.len(), 31, "telemetry residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create telemetry evidence directory");
    fs::write(
        evidence_dir.join("telemetry_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize telemetry ledger"),
    )
    .expect("write telemetry ledger");
    assert!(
        mismatches.is_empty(),
        "{} telemetry controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
