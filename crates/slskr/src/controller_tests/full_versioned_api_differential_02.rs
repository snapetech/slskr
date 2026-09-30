//! Controller full versioned api differential 02 ownership.

use super::*;

/// Differential proof for the empty versioned capability-peer list. The
/// populated projection is covered separately; this closes its nominal
/// status/header/body case with the oracle's exact envelope.
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
pub(super) async fn controller_api_differential_versioned_capability_peers_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = crate::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &state)
        .await
        .expect("versioned capability peers response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "count": 0,
        "peers": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/capabilities/peers",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_capability_peers_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/capabilities/peers: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty/default versioned destination
/// projections. The legacy destination records retain their compatibility
/// fields, while slskdN returns the four-field `DestinationResponse` DTO.
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
pub(super) async fn controller_api_differential_versioned_destinations_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let expected_default = serde_json::json!({
        "name": "Downloads",
        "path": "/home/user/Downloads",
        "isDefault": true,
        "exists": std::path::Path::new("/home/user/Downloads").exists(),
    });
    let cases = [
        (
            "/api/v0/destinations",
            serde_json::json!([expected_default.clone()]),
        ),
        ("/api/v0/destinations/default", expected_default),
    ];
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for (path, expected) in cases {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned destinations response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == expected;
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": path,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_destinations_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the empty versioned hash-database backfill
/// candidate projection. The compatibility route retains its historical
/// `candidates` and `entries` aliases; slskdN exposes `count` and
/// `candidates`.
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
pub(super) async fn controller_api_differential_versioned_backfill_candidates_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        crate::route_http_request("GET", "/api/v0/backfill/candidates", None, "", &state)
            .await
            .expect("versioned backfill-candidates response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "count": 0,
        "candidates": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/backfill/candidates",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_backfill_candidates_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/backfill/candidates: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned multi-source job list.
/// This route already projects the oracle's `{ count, jobs }` envelope;
/// the missing piece was an explicit nominal ledger row.
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
pub(super) async fn controller_api_differential_versioned_multisource_jobs_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = crate::route_http_request("GET", "/api/v0/multisource/jobs", None, "", &state)
        .await
        .expect("versioned multisource-jobs response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "count": 0,
        "jobs": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/multisource/jobs",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_multisource_jobs_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/multisource/jobs: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned swarm-trends DTO.  The
/// oracle has no historical trend storage in this state, so the nominal
/// response is the exact six-array envelope already returned by slskR.
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
pub(super) async fn controller_api_differential_versioned_swarm_trends_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        crate::route_http_request("GET", "/api/v0/swarm/analytics/trends", None, "", &state)
            .await
            .expect("versioned swarm-trends response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "timePoints": [],
        "successRates": [],
        "averageSpeeds": [],
        "averageDurations": [],
        "averageSourcesUsed": [],
        "downloadCounts": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/swarm/analytics/trends",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_swarm_trends_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/swarm/analytics/trends: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned swarm-analytics dashboard.
/// The response is fully derived from an empty multi-source store and
/// matches the oracle's zero metrics plus deterministic recommendations.
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
pub(super) async fn controller_api_differential_versioned_swarm_dashboard_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        crate::route_http_request("GET", "/api/v0/swarm/analytics/dashboard", None, "", &state)
            .await
            .expect("versioned swarm-dashboard response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "performanceMetrics": {
            "totalDownloads": 0,
            "successfulDownloads": 0,
            "failedDownloads": 0,
            "successRate": 0.0,
            "averageDurationSeconds": 0.0,
            "averageSpeedBytesPerSecond": 0.0,
            "averageSourcesUsed": 0.0,
            "totalBytesDownloaded": 0,
            "totalChunksCompleted": 0,
            "chunkSuccessRate": 0.0,
            "timeWindow": "1.00:00:00",
        },
        "peerRankings": [],
        "efficiencyMetrics": {
            "chunkUtilization": 0.0,
            "peerUtilization": 0.0,
            "redundancyFactor": 0.0,
            "averageTimeToFirstByteMs": 0.0,
            "averageReassignmentRate": 0.0,
            "averageRescueRate": 0.0,
        },
        "recommendations": [
            {
                "type": "NetworkConfig",
                "priority": "High",
                "title": "Low Download Speed",
                "description": "Average download speed is 0.00 MB/s. This may indicate network or peer issues.",
                "action": "Check network connectivity, firewall settings, and consider using more sources per download.",
                "estimatedImpact": 0.4,
            },
            {
                "type": "PeerSelection",
                "priority": "High",
                "title": "Low Success Rate",
                "description": "Current success rate is 0.0%. Consider improving peer selection criteria.",
                "action": "Review peer reputation thresholds and increase minimum reputation score for peer selection.",
                "estimatedImpact": 0.3,
            },
            {
                "type": "ChunkSize",
                "priority": "Medium",
                "title": "High Chunk Failure Rate",
                "description": "Chunk success rate is 0.0%. Consider adjusting chunk size.",
                "action": "Try reducing chunk size to improve reliability, or increase timeout values.",
                "estimatedImpact": 0.2,
            },
            {
                "type": "SourceCount",
                "priority": "Low",
                "title": "Low Peer Utilization",
                "description": "Only 0.0% of available peers are being utilized.",
                "action": "Consider increasing the number of sources per download to improve redundancy.",
                "estimatedImpact": 0.15,
            },
        ],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/swarm/analytics/dashboard",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_swarm_dashboard_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/swarm/analytics/dashboard: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for an empty versioned transfer-history page.  An
/// explicit future watermark makes the nominal DTO deterministic while
/// exercising the real direction, cursor, and page-size validation path.
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
pub(super) async fn controller_api_differential_versioned_transfer_history_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = crate::route_http_request(
        "GET",
        "/api/v0/transfers/history?direction=download&asOf=4102444800000",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned transfer-history response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "asOf": 4_102_444_800_000_i64,
        "hasMore": false,
        "nextOffset": 0,
        "transfers": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/transfers/history",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_history_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/transfers/history: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the two empty list projections in the
/// versioned transfer controller.  Both are exact `[]` responses in the
/// oracle's fresh state and in slskR's state-backed queue.
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
pub(super) async fn controller_api_differential_versioned_transfer_empty_lists_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for (route, label) in [
        ("/api/v0/transfers/uploads", "versioned uploads"),
        (
            "/api/v0/transfers/downloads/stuck",
            "versioned stuck downloads",
        ),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .expect(label);
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == serde_json::json!([]);
        if !pass {
            mismatches.push(format!(
                "{target} GET {route}: got {} {} {}",
                response.status, response.content_type, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": route,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_empty_lists_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the versioned transfer-change snapshot.  The
/// cursor is server-generated, so this checks the exact response envelope,
/// zero direction counts, and empty transfer set rather than comparing the
/// timestamp to a second process's clock.
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
pub(super) async fn controller_api_differential_versioned_transfer_changes_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = crate::route_http_request("GET", "/api/v0/transfers/changes", None, "", &state)
        .await
        .expect("versioned transfer-changes response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value["cursor"].as_u64().is_some_and(|cursor| cursor > 0)
        && value["counts"] == serde_json::json!({"download": 0, "upload": 0})
        && value["transfers"] == serde_json::json!([]);
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/transfers/changes",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_changes_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/transfers/changes: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned transfer-summary DTO.  The
/// oracle returns a direction-keyed map whose state maps are empty when
/// the transfer store has no completed records; the legacy route retains
/// slskR's historical aggregate report.
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
pub(super) async fn controller_api_differential_versioned_transfer_summary_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned transfer-summary response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "Download": {},
        "Upload": {},
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/telemetry/reports/transfers/summary",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_summary_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/telemetry/reports/transfers/summary: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for a deterministic empty versioned transfer
/// histogram. The explicit future window avoids wall-clock-dependent
/// bucket keys while exercising the oracle's gapless interval projection.
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
pub(super) async fn controller_api_differential_versioned_transfer_histogram_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned transfer-histogram response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "2100-01-01T00:00:00Z": {
            "Download": {},
            "Upload": {},
        },
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/telemetry/reports/transfers/histogram",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_histogram_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/telemetry/reports/transfers/histogram: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for populated versioned transfer reports.  The
/// oracle groups completed records by direction and final state and
/// carries the measured summary fields into both the summary and histogram
/// projections.  Fixed Unix timestamps keep both responses deterministic.
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
pub(super) async fn controller_api_differential_versioned_transfer_reports_populated_state() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut transfers = state.transfers.write().await;
        let created = transfers.create(
            0,
            Some("telemetry peer".to_owned()),
            "Telemetry/Report.flac".to_owned(),
            None,
            Some(321),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == created.id)
            .expect("populated telemetry transfer");
        entry.status = "succeeded".to_owned();
        entry.requested_at = 3_700;
        entry.started_at = Some(3_710);
        entry.bytes_transferred = 321;
        entry.updated_at = 3_730;
        entry.updated_at_ms = 3_730_000;
    }

    let summary = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary?start=3600&end=7200&direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("populated versioned transfer-summary response");
    let summary_value =
        serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or(serde_json::Value::Null);
    let summary_record = &summary_value["Download"]["Succeeded"];
    let summary_pass = summary.status == "200 OK"
        && summary.content_type.starts_with("application/json")
        && summary_record["username"] == ""
        && summary_record["totalBytes"] == 321
        && summary_record["count"] == 1
        && summary_record["distinctUsers"] == 1
        && summary_record["averageWait"] == 10.0
        && summary_record["averageDuration"] == 20.0
        && summary_record["averageSpeed"]
            .as_f64()
            .is_some_and(|value| (value - 16.05).abs() < 0.000001)
        && summary_value["Upload"] == serde_json::json!({});

    let histogram = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=3600&end=7200&interval=60&direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("populated versioned transfer-histogram response");
    let histogram_value = serde_json::from_str::<serde_json::Value>(&histogram.body)
        .unwrap_or(serde_json::Value::Null);
    let histogram_record = &histogram_value["1970-01-01T01:00:00Z"]["Download"]["Succeeded"];
    let histogram_pass = histogram.status == "200 OK"
        && histogram.content_type.starts_with("application/json")
        && histogram_record["username"] == ""
        && histogram_record["totalBytes"] == 321
        && histogram_record["count"] == 1
        && histogram_record["distinctUsers"] == 1
        && histogram_value["1970-01-01T01:00:00Z"]["Upload"] == serde_json::json!({})
        && histogram_value
            .as_object()
            .is_some_and(|buckets| buckets.len() == 1);

    let ledger = vec![
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/telemetry/reports/transfers/summary",
            "case": "populated-dynamic-state",
            "pass": summary_pass,
        }),
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/telemetry/reports/transfers/histogram",
            "case": "populated-dynamic-state",
            "pass": histogram_pass,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_reports_populated.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        summary_pass,
        "{target} GET /api/v0/telemetry/reports/transfers/summary: got {} {} {}",
        summary.status, summary.content_type, summary.body
    );
    assert!(
        histogram_pass,
        "{target} GET /api/v0/telemetry/reports/transfers/histogram: got {} {} {}",
        histogram.status, histogram.content_type, histogram.body
    );
}

/// Bulk differential proof for the versioned destinations controller's
/// populated list and selected-default projections.  The fixture uses a
/// real directory so the response's existence flag is also exercised;
/// the in-memory destination store is the same state-backed projection
/// used by the HTTP handlers.  slskdN-only (confirmed against the frozen
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
pub(super) async fn controller_api_differential_versioned_destinations_populated_state() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [populated-dynamic-state]",
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "populated-dynamic-state",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let root = std::env::temp_dir().join(format!(
        "slskr-destination-populated-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let custom_path = root.join("Archive");
    fs::create_dir_all(&custom_path).expect("create destination fixture");
    {
        let mut destinations = state.destinations.write().await;
        destinations.records = vec![
            crate::destination_state::DestinationRecord {
                id: "default".to_owned(),
                name: "Downloads".to_owned(),
                path: root.join("Downloads").display().to_string(),
                is_default: false,
            },
            crate::destination_state::DestinationRecord {
                id: "configured-0".to_owned(),
                name: "Archive".to_owned(),
                path: custom_path.display().to_string(),
                is_default: true,
            },
        ];
    }

    let list = crate::route_http_request("GET", "/api/v0/destinations", None, "", &state)
        .await
        .expect("populated destinations response");
    let list_value =
        serde_json::from_str::<serde_json::Value>(&list.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/destinations",
        list.status == "200 OK"
            && list.content_type == "application/json"
            && list_value
                .as_array()
                .is_some_and(|records| records.len() == 2)
            && list_value
                .as_array()
                .is_some_and(|records| records.iter().any(|record| {
                    record["name"] == "Archive"
                        && record["path"] == custom_path.display().to_string()
                        && record["isDefault"] == true
                        && record["exists"] == true
                }))
    );

    let default =
        crate::route_http_request("GET", "/api/v0/destinations/default", None, "", &state)
            .await
            .expect("populated default destination response");
    let default_value =
        serde_json::from_str::<serde_json::Value>(&default.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/destinations/default",
        default.status == "200 OK"
            && default.content_type == "application/json"
            && default_value["name"] == "Archive"
            && default_value["path"] == custom_path.display().to_string()
            && default_value["isDefault"] == true
            && default_value["exists"] == true
    );

    let _ = fs::remove_dir_all(root);
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_destinations_populated_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api destinations mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
