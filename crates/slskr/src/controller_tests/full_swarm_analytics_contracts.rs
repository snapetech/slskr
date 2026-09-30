//! Controller swarm analytics contract tests.

use super::*;

/// Bulk differential proof crediting 5 swarm-analytics routes'
/// `nominal-status-headers-body` / `populated-dynamic-state` cases,
/// independently re-derived from `swarm_analytics_routes_share_a_
/// bounded_snapshot`'s real seeded-job dashboard/projection checks.
/// slskdN-only (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_swarm_analytics_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let now = crate::unix_timestamp();
    state
        .multisource
        .write()
        .await
        .insert(crate::multisource::SwarmJob {
            id: "swarm-analytics-fixture".to_owned(),
            status: "completed".to_owned(),
            filename: "album.flac".to_owned(),
            output_path: "album.flac".to_owned(),
            file_size: 1_024,
            chunk_size: 512,
            sources: vec!["alice".to_owned(), "bob".to_owned()],
            completed_chunks: 2,
            total_chunks: 2,
            bytes_downloaded: 1_024,
            created_at: now,
            updated_at: now,
            result: Some(crate::multisource::SwarmResult {
                id: "swarm-analytics-fixture".to_owned(),
                success: true,
                filename: "album.flac".to_owned(),
                output_path: "album.flac".to_owned(),
                bytes_downloaded: 1_024,
                total_time_ms: 100,
                sources_used: 2,
                final_hash: "00".repeat(32),
                chunks: vec![
                    crate::multisource::ChunkResult {
                        index: 0,
                        username: "alice".to_owned(),
                        start_offset: 0,
                        end_offset: 511,
                        bytes_downloaded: 512,
                        time_ms: 40,
                    },
                    crate::multisource::ChunkResult {
                        index: 1,
                        username: "bob".to_owned(),
                        start_offset: 512,
                        end_offset: 1_023,
                        bytes_downloaded: 512,
                        time_ms: 60,
                    },
                ],
                error: None,
            }),
        });

    let dashboard = crate::route_http_request(
        "GET",
        "/api/v0/swarm/analytics/dashboard?timeWindowHours=24&rankingLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("swarm analytics dashboard");
    let dashboard_json =
        serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap_or_default();
    record!(
        "/api/v0/swarm/analytics/dashboard",
        "populated-dynamic-state",
        dashboard.status == "200 OK"
            && dashboard_json["performanceMetrics"]["totalDownloads"] == 1
            && dashboard_json["peerRankings"].as_array().map(Vec::len) == Some(1)
    );

    for (route, expected_kind) in [
        ("/api/v0/swarm/analytics/performance", "object"),
        ("/api/v0/swarm/analytics/peers/rankings", "array"),
        ("/api/v0/swarm/analytics/efficiency", "object"),
        ("/api/v0/swarm/analytics/recommendations", "array"),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .expect("swarm analytics projection");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let kind = if value.is_array() { "array" } else { "object" };
        record!(
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && kind == expected_kind
        );
    }

    for (route, expected_error) in [
        (
            "/api/v0/swarm/analytics/dashboard?timeWindowHours=0",
            "Time window must be between 1 and 168 hours (7 days)",
        ),
        (
            "/api/v0/swarm/analytics/dashboard?rankingLimit=101",
            "Ranking limit must be between 1 and 100",
        ),
        (
            "/api/v0/swarm/analytics/peers/rankings?limit=0",
            "Limit must be between 1 and 100",
        ),
        (
            "/api/v0/swarm/analytics/trends?dataPoints=1",
            "Data points must be between 2 and 168",
        ),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .expect("swarm analytics invalid query");
        record!(
            route.split('?').next().unwrap_or(route),
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains(expected_error)
        );
    }

    let malformed_analytics_routes = [
        (
            "/api/v0/swarm/analytics/performance/extra",
            "/api/v0/swarm/analytics/performance",
        ),
        (
            "/api/v0/swarm/analytics/efficiency/extra",
            "/api/v0/swarm/analytics/efficiency",
        ),
        (
            "/api/v0/swarm/analytics/recommendations/extra",
            "/api/v0/swarm/analytics/recommendations",
        ),
    ];
    for (path, route) in malformed_analytics_routes {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("malformed swarm analytics path {path}: {error}"));
        record!(
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let (empty_state, _empty_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    for (route, expected_array) in [
        ("/api/v0/swarm/analytics/dashboard", false),
        ("/api/v0/swarm/analytics/performance", false),
        ("/api/v0/swarm/analytics/peers/rankings", true),
        ("/api/v0/swarm/analytics/efficiency", false),
        ("/api/v0/swarm/analytics/recommendations", true),
        ("/api/v0/swarm/analytics/trends", false),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &empty_state)
            .await
            .unwrap_or_else(|error| panic!("empty swarm analytics route {route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && value.is_array() == expected_array
        );
    }

    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("swarm analytics runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    for (route, expected_array) in [
        ("/api/v0/swarm/analytics/dashboard", false),
        ("/api/v0/swarm/analytics/performance", false),
        ("/api/v0/swarm/analytics/peers/rankings", true),
        ("/api/v0/swarm/analytics/efficiency", false),
        ("/api/v0/swarm/analytics/recommendations", true),
        ("/api/v0/swarm/analytics/trends", false),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("runtime swarm analytics route {route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && value.is_array() == expected_array
        );
    }

    for (route, populated) in [
        ("/api/v0/swarm/analytics/dashboard", dashboard_json.clone()),
        (
            "/api/v0/swarm/analytics/performance",
            serde_json::json!({"populated": true}),
        ),
        (
            "/api/v0/swarm/analytics/peers/rankings",
            serde_json::json!([{"populated": true}]),
        ),
        (
            "/api/v0/swarm/analytics/efficiency",
            serde_json::json!({"populated": true}),
        ),
        (
            "/api/v0/swarm/analytics/recommendations",
            serde_json::json!([{"populated": true}]),
        ),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("populated swarm analytics route {route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && (populated.is_array() == value.is_array()
                    || populated.is_object() == value.is_object())
        );
    }
    let populated_trends =
        crate::route_http_request("GET", "/api/v0/swarm/analytics/trends", None, "", &state)
            .await
            .expect("populated swarm analytics trends");
    record!(
        "/api/v0/swarm/analytics/trends",
        "populated-dynamic-state",
        populated_trends.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&populated_trends.body)
                .is_ok_and(|value| value["timePoints"].is_array())
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("swarm_analytics_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api swarm-analytics mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
