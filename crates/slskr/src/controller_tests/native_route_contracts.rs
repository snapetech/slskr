use std::fs;

use super::test_state;

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
pub(super) async fn controller_api_differential_compatibility_aliases_reach_state_backed_routes() {
    let (state, _receiver) = test_state();
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for path in [
        "/api/info",
        "/api/downloads",
        "/api/server/status",
        "/api/slskdn/capabilities",
        "/api/v0/slskdn/capabilities",
        "/api/v0/capabilities/mesh-peers",
        "/api/v0/dht/peers",
        "/api/v0/fairness/summary",
        "/api/v0/hashdb/backfill/candidates",
        "/api/v0/integrations/lidarr/sync/status",
        "/api/v0/songid/runs/queue",
        "/api/v0/transfers/downloads/auto-replace/status",
        "/api/v0/portforwarding/available-ports?limit=1",
        "/api/v0/portforwarding/stream-stats",
        "/api/v0/security/adversarial/stats",
        "/api/v0/security/anomalies",
        "/api/v0/security/bans",
        "/api/v0/security/circuits",
        "/api/v0/security/circuits/stats",
        "/api/v0/security/events",
        "/api/v0/security/network",
        "/api/v0/security/network/top",
        "/api/v0/security/peers",
        "/api/v0/security/peers/stats",
        "/api/v0/security/reputation/suspicious",
        "/api/v0/security/reputation/trusted",
        "/api/v0/security/scanners",
        "/api/v0/security/threats",
        "/api/v0/security/transports",
        "/api/v0/security/transports/status",
        "/api/v0/mediacore/contentid/stats",
        "/api/v0/mediacore/ipld/validate",
        "/api/v0/mediacore/perceptualhash/algorithms",
        "/api/v0/mediacore/portability/merge-strategies",
        "/api/v0/mediacore/portability/strategies",
        "/api/v0/mediacore/publish/stats",
        "/api/v0/mediacore/retrieve/stats",
        "/api/v0/mediacore/stats/dashboard",
        "/api/v0/mediacore/stats/descriptors",
        "/api/v0/mediacore/stats/fuzzy",
        "/api/v0/mediacore/stats/ipld",
        "/api/v0/mediacore/stats/perceptual",
        "/api/v0/mediacore/stats/portability",
        "/api/v0/mediacore/stats/publishing",
        "/api/v0/mediacore/stats/registry",
        "/api/bridge/rooms",
        "/api/source-feed-imports/history",
        "/api/v0/hashdb/inventory/unhashed",
        "/api/v0/hashdb/key",
        "/api/v0/hashdb/optimize/analyze",
        "/api/v0/hashdb/optimize/slow-queries",
        "/api/v0/hashdb/peers",
        "/api/v0/hashdb/schema",
        "/api/v0/mesh/delta",
        "/api/v0/mesh/hello",
        "/api/v0/multisource/search",
        "/api/v0/multisource/users",
        "/api/v0/opinions",
        "/api/v0/opinions/summary",
        "/api/v0/overlay/blocklist",
        "/api/v0/overlay/connections",
        "/api/v0/overlay/stats",
        "/api/v0/podcore/backfill/stats",
        "/api/v0/podcore/content/metadata",
        "/api/v0/podcore/dht/stats",
        "/api/v0/podcore/discovery/all",
        "/api/v0/podcore/discovery/stats",
        "/api/v0/podcore/membership/stats",
        "/api/v0/podcore/messages/stats",
        "/api/v0/podcore/routing/stats",
        "/api/v0/podcore/signing/stats",
        "/api/v0/podcore/verification/stats",
        "/api/v0/quarantine-jury/audit",
        "/api/v0/quarantine-jury/requests",
        "/api/v0/songid/capabilities",
        "/api/v0/source-feed-imports/history",
        "/api/v0/telemetry/prometheus",
        "/api/v0/telemetry/prometheus/kpis",
        "/api/v0/virtualsoulfind/disaster-mode/status",
        "/api/virtualsoulfind/disaster-mode/status",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_ne!(response.status, "404 Not Found", "{path}");
        // Matches the oracle's MetricsController: the base prometheus
        // route defaults to text/plain, but /kpis is always a JSON
        // dictionary -- it has no text/plain response type at all.
        if path.ends_with("telemetry/prometheus") {
            assert!(response.content_type.starts_with("text/plain"), "{path}");
        } else {
            assert!(
                response.content_type.starts_with("application/json"),
                "{path}"
            );
        }
        let nominal = response.status.starts_with('2')
            && !response.body.is_empty()
            && if path.ends_with("telemetry/prometheus") {
                response.content_type.starts_with("text/plain")
            } else {
                response.content_type.starts_with("application/json")
            };
        let route = path.split('?').next().unwrap_or(path);
        let expected_validation_error = match route {
            "/api/v0/hashdb/key" => Some("filename and positive size are required"),
            "/api/v0/multisource/search"
            | "/api/v0/multisource/users"
            | "/api/v0/opinions/summary"
            | "/api/v0/podcore/content/metadata" => Some("A required query value is missing"),
            _ => None,
        };
        let validation = expected_validation_error.is_some_and(|message| {
            response.status == "400 Bad Request" && response.body.contains(message)
        });
        let (case, pass) = if expected_validation_error.is_some() {
            ("malformed-path-query-or-body", validation)
        } else {
            ("nominal-status-headers-body", nominal)
        };
        if !pass {
            mismatches.push(format!(
                "slskdn GET {route}: got {} {} {}",
                response.status, response.content_type, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": route,
            "case": case,
            "pass": pass,
        }));
    }
    let versioned_bridge_rooms =
        crate::route_http_request("GET", "/api/v0/bridge/rooms", None, "", &state)
            .await
            .expect("versioned bridge rooms");
    let versioned_bridge_rooms_pass = versioned_bridge_rooms.status == "200 OK"
        && versioned_bridge_rooms
            .content_type
            .starts_with("application/json");
    if !versioned_bridge_rooms_pass {
        mismatches.push(format!(
            "slskdn GET /api/v0/bridge/rooms: got {} {} {}",
            versioned_bridge_rooms.status,
            versioned_bridge_rooms.content_type,
            versioned_bridge_rooms.body
        ));
    }
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/bridge/rooms",
        "case": "nominal-status-headers-body",
        "pass": versioned_bridge_rooms_pass,
    }));

    // These versioned controller routes normalize to the same local
    // state-backed handlers as their compatibility siblings, but their
    // frozen slskdN subjects are versioned.  Assert the oracle DTO fields
    // and empty-directory shapes explicitly before crediting them.
    let versioned_backfill_stats =
        crate::route_http_request("GET", "/api/v0/backfill/stats", None, "", &state)
            .await
            .expect("versioned backfill stats");
    let versioned_backfill_stats_json =
        serde_json::from_str::<serde_json::Value>(&versioned_backfill_stats.body)
            .expect("versioned backfill stats JSON");
    let versioned_backfill_stats_pass = versioned_backfill_stats.status == "200 OK"
        && versioned_backfill_stats.content_type == "application/json"
        && versioned_backfill_stats_json["totalAttempts"] == 0
        && versioned_backfill_stats_json["successful"] == 0
        && versioned_backfill_stats_json["failed"] == 0
        && versioned_backfill_stats_json["rateLimited"] == 0
        && versioned_backfill_stats_json["active"] == 0
        && versioned_backfill_stats_json["hashesDiscovered"] == 0
        && versioned_backfill_stats_json["isIdle"] == false
        && versioned_backfill_stats_json.get("lastCycleTime").is_some()
        && versioned_backfill_stats_json.get("nextCycleTime").is_some()
        && versioned_backfill_stats_json.get("idleDuration").is_some();
    if !versioned_backfill_stats_pass {
        mismatches.push(format!(
            "slskdn GET /api/v0/backfill/stats: got {} {} {}",
            versioned_backfill_stats.status,
            versioned_backfill_stats.content_type,
            versioned_backfill_stats.body
        ));
    }
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/backfill/stats",
        "case": "nominal-status-headers-body",
        "pass": versioned_backfill_stats_pass,
    }));

    let versioned_backfill_config =
        crate::route_http_request("GET", "/api/v0/backfill/config", None, "", &state)
            .await
            .expect("versioned backfill config");
    let versioned_backfill_config_json =
        serde_json::from_str::<serde_json::Value>(&versioned_backfill_config.body)
            .expect("versioned backfill config JSON");
    let versioned_backfill_config_pass = versioned_backfill_config.status == "200 OK"
        && versioned_backfill_config.content_type == "application/json"
        && versioned_backfill_config_json["maxGlobalConnections"] == 2
        && versioned_backfill_config_json["maxPerPeerPerDay"] == 10
        && versioned_backfill_config_json["maxHeaderBytes"] == 65_536
        && versioned_backfill_config_json["minIdleTimeSeconds"] == 300
        && versioned_backfill_config_json["runIntervalSeconds"] == 600
        && versioned_backfill_config_json["transferTimeoutSeconds"] == 30
        && versioned_backfill_config_json["enabled"] == true;
    if !versioned_backfill_config_pass {
        mismatches.push(format!(
            "slskdn GET /api/v0/backfill/config: got {} {} {}",
            versioned_backfill_config.status,
            versioned_backfill_config.content_type,
            versioned_backfill_config.body
        ));
    }
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/api/v0/backfill/config",
        "case": "nominal-status-headers-body",
        "pass": versioned_backfill_config_pass,
    }));

    for (route, label) in [
        ("/api/v0/rooms/available", "versioned available rooms"),
        ("/api/v0/rooms/joined", "versioned joined rooms"),
    ] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or_else(|error| panic!("{label} JSON: {error}"));
        let pass = response.status == "200 OK"
            && response.content_type == "application/json"
            && value.as_array().is_some_and(|entries| entries.is_empty());
        if !pass {
            mismatches.push(format!(
                "slskdn GET {route}: got {} {} {}",
                response.status, response.content_type, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": route,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let status = crate::route_http_request("GET", "/api/server/status", None, "", &state)
        .await
        .unwrap();
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["connected"], false);
    assert_eq!(status_json["state"], "disconnected");
    assert_eq!(status_json["username"], "");
    assert!(status_json.get("credentialStore").is_none());

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("compatibility_aliases_state_backed.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api compatibility-alias mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_native_capability_and_library_health_contracts() {
    let (state, _receiver) = test_state();
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("slskdn {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let capabilities =
        crate::route_http_request("GET", "/api/slskdn/capabilities", None, "", &state)
            .await
            .unwrap();
    assert_eq!(capabilities.status, "200 OK", "{}", capabilities.body);
    let capabilities = serde_json::from_str::<serde_json::Value>(&capabilities.body).unwrap();
    assert_eq!(capabilities["impl"], "slskdn");
    assert_eq!(capabilities["compat"], "slskd");
    assert!(capabilities["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature == "library_health"));
    assert_eq!(capabilities["obfuscation"]["type"], 1);
    assert_eq!(
        capabilities["obfuscation"]["supportedConnectionTypes"],
        serde_json::json!(["P", "D", "F"])
    );
    record!(
        "GET",
        "/api/slskdn/capabilities",
        "nominal-status-headers-body",
        capabilities["impl"] == "slskdn"
            && capabilities["compat"] == "slskd"
            && capabilities["features"].is_array()
    );

    let health = crate::route_http_request(
        "GET",
        "/api/slskdn/library/health?limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(health.status, "200 OK", "{}", health.body);
    let health = serde_json::from_str::<serde_json::Value>(&health.body).unwrap();
    assert_eq!(health["path"], "(all)");
    assert_eq!(
        health["summary"],
        serde_json::json!({
            "total_issues": 0,
            "issues_open": 0,
            "issues_resolved": 0,
        })
    );
    assert!(health.get("issueCount").is_none());
    record!(
        "GET",
        "/api/slskdn/library/health",
        "nominal-status-headers-body",
        health["path"] == "(all)"
            && health["summary"]["total_issues"] == 0
            && health["summary"]["issues_open"] == 0
            && health["summary"]["issues_resolved"] == 0
    );

    let invalid = crate::route_http_request(
        "GET",
        "/api/slskdn/library/health?limit=251",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid.status, "400 Bad Request");
    record!(
        "GET",
        "/api/slskdn/library/health",
        "malformed-path-query-or-body",
        invalid.status == "400 Bad Request"
    );

    let transport = crate::route_http_request("GET", "/api/v0/mesh/transport", None, "", &state)
        .await
        .unwrap();
    let transport = serde_json::from_str::<serde_json::Value>(&transport.body).unwrap();
    assert!(transport["dht"].is_number());
    assert!(transport["overlay"].is_number());
    assert_eq!(transport["natType"], "Unknown");
    record!(
        "GET",
        "/api/v0/mesh/transport",
        "nominal-status-headers-body",
        transport["dht"].is_number()
            && transport["overlay"].is_number()
            && transport["natType"] == "Unknown"
    );

    let peers = crate::route_http_request("GET", "/api/v0/mesh/peers", None, "", &state)
        .await
        .unwrap();
    let peers = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap();
    assert!(peers["count"].is_number());
    assert!(peers["peers"].is_array());
    assert!(peers["overlay"].is_array());
    record!(
        "GET",
        "/api/v0/mesh/peers",
        "nominal-status-headers-body",
        peers["count"].is_number() && peers["peers"].is_array() && peers["overlay"].is_array()
    );

    let hashdb = crate::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
        .await
        .unwrap();
    let hashdb = serde_json::from_str::<serde_json::Value>(&hashdb.body).unwrap();
    for key in [
        "totalPeers",
        "capabilityPeers",
        "totalFlacEntries",
        "hashedFlacEntries",
        "totalHashEntries",
        "currentSeqId",
        "databaseSizeBytes",
    ] {
        assert!(hashdb.get(key).is_some(), "missing {key}: {hashdb}");
    }
    assert!(hashdb.get("totalEntries").is_none());
    assert!(hashdb.get("projectedShareEntries").is_none());
    record!(
        "GET",
        "/api/v0/hashdb/stats",
        "nominal-status-headers-body",
        hashdb["totalPeers"].is_number()
            && hashdb["totalFlacEntries"].is_number()
            && hashdb["currentSeqId"].is_number()
            && hashdb["databaseSizeBytes"].is_number()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("native_capability_and_library_health.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api native capability mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn dynamic_controller_route_patterns_cover_materialized_controller_paths() {
    for path in [
        "/api/audio/canonical/recording-id",
        "/api/compatibility/users/peer/browse",
        "/api/hashdb/inventory/by-size/1",
        "/api/listening-party/pod/channel",
        "/api/podcore/pod/channels",
        "/api/podcore/messages/pod/channel/count",
        "/api/realm-subject-indexes/realm",
        "/api/virtualsoulfind/canonical/recording-id",
    ] {
        assert!(crate::extended_controller_dynamic_get_route(path), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn materialized_controller_gets_match_native_empty_state_contracts() {
    let (state, _receiver) = test_state();
    let cases = [
        ("/api/v0/nowplaying", "204 No Content", None),
        // A truly nonexistent pod/channel is a real 404, not a
        // fake 204 -- matching the sibling pod-channel-messages
        // endpoint's convention for the identical "doesn't exist"
        // check.
        (
            "/api/v0/listening-party/missing-pod/missing-channel",
            "404 Not Found",
            Some("\"error\":\"not found\""),
        ),
        (
            "/api/v0/mediacore/contentid/domain/music",
            "200 OK",
            Some("\"contentIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            "200 OK",
            Some("\"normalizedType\":\"recording\""),
        ),
        (
            "/api/v0/mediacore/contentid/exists/missing",
            "200 OK",
            Some("\"exists\":false"),
        ),
        (
            "/api/v0/mediacore/contentid/external/missing",
            "200 OK",
            Some("\"externalIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/validate/not-a-content-id",
            "200 OK",
            Some("\"isValid\":false"),
        ),
        (
            "/api/v0/mediacore/ipld/graph/missing",
            "200 OK",
            Some("\"nodes\":["),
        ),
        (
            "/api/v0/mediacore/ipld/inbound/missing",
            "200 OK",
            Some("\"inboundLinks\":[]"),
        ),
        (
            "/api/v0/mediacore/retrieve/query/domain/music",
            "200 OK",
            Some("\"descriptors\":[]"),
        ),
        (
            "/api/v0/podcore/missing/opinions/members/affinity",
            "200 OK",
            Some("{}"),
        ),
        (
            "/api/v0/podcore/backfill/missing/last-seen",
            "200 OK",
            Some("{}"),
        ),
        (
            "/api/v0/pods/missing/channels/missing/messages",
            "200 OK",
            Some("[]"),
        ),
        (
            "/api/v0/quarantine-jury/requests/missing/routes",
            "200 OK",
            Some("[]"),
        ),
        (
            "/api/v0/security/disclosure/missing",
            "200 OK",
            Some("\"peerTier\":\"Unknown\""),
        ),
        (
            // Matches slskR's real reputation default: a peer with no
            // recorded violations has never been decremented from its
            // starting score.
            "/api/v0/security/reputation/missing",
            "200 OK",
            Some("\"score\":50"),
        ),
        (
            "/api/v0/traces/missing/summary",
            "200 OK",
            Some("\"totalEvents\":0"),
        ),
    ];

    for (path, status, expected_body) in cases {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, status, "{path}: {}", response.body);
        if let Some(expected_body) = expected_body {
            assert!(
                response.body.contains(expected_body),
                "{path}: {}",
                response.body
            );
        } else {
            assert!(response.body.is_empty(), "{path}: {}", response.body);
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn openapi_mutation_dtos_match_native_status_and_field_contracts() {
    let (state, _receiver) = test_state();

    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Route Audit","description":"contract"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(collection.status, "201 Created");
    let collection_json = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap();
    assert_eq!(collection_json["title"], "Route Audit");
    assert_eq!(collection_json["type"], "ShareList");
    // Matches the real AuthenticatedWebUserId.Resolve semantics: with
    // no resolvable per-caller identity (this test uses no
    // authentication at all), ownerUserId is honestly empty rather
    // than a fabricated "Anonymous" placeholder.
    assert_eq!(collection_json["ownerUserId"], "");
    assert!(uuid::Uuid::parse_str(collection_json["id"].as_str().unwrap()).is_ok());
    assert!(
        chrono::DateTime::parse_from_rfc3339(collection_json["createdAt"].as_str().unwrap())
            .is_ok()
    );
    assert!(collection_json.get("items").is_none());

    let collection_id = collection_json["id"].as_str().unwrap();
    let collection_item = crate::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items"),
        None,
        r#"{"contentId":"content:music:recording:route-audit","mediaKind":"Music","contentHash":"abc123","fileName":"Route Audit.flac","title":"Route Audit","artist":"Artist","album":"Album"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(collection_item.status, "201 Created");
    let item_json = serde_json::from_str::<serde_json::Value>(&collection_item.body).unwrap();
    assert!(uuid::Uuid::parse_str(item_json["id"].as_str().unwrap()).is_ok());
    assert_eq!(item_json["collectionId"], collection_id);
    assert_eq!(item_json["ordinal"], 0);
    assert_eq!(
        item_json["contentId"],
        "content:music:recording:route-audit"
    );
    assert_eq!(item_json["fileName"], "Route Audit.flac");
    assert_eq!(item_json["album"], "Album");
    assert_eq!(item_json["contentHash"], "abc123");
    assert!(item_json.get("addedAt").is_none());

    let updated_collection = crate::route_http_request(
        "PUT",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        r#"{"title":"Updated Route Audit","type":"Playlist"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated_collection.status, "200 OK");
    let updated_collection_json =
        serde_json::from_str::<serde_json::Value>(&updated_collection.body).unwrap();
    assert_eq!(updated_collection_json["title"], "Updated Route Audit");
    assert_eq!(updated_collection_json["description"], "contract");
    assert_eq!(updated_collection_json["type"], "Playlist");

    let item_id = item_json["id"].as_str().unwrap();
    let updated_item = crate::route_http_request(
        "PUT",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
        None,
        r#"{"title":"Updated Item","album":"Updated Album","sha256":"def456"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated_item.status, "200 OK");
    let updated_item_json = serde_json::from_str::<serde_json::Value>(&updated_item.body).unwrap();
    assert_eq!(updated_item_json["ordinal"], 0);
    assert_eq!(updated_item_json["title"], "Updated Item");
    assert_eq!(updated_item_json["album"], "Updated Album");
    assert_eq!(updated_item_json["contentHash"], "def456");

    let reordered = crate::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items/reorder"),
        None,
        &format!(r#"{{"itemIds":["{item_id}"]}}"#),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(reordered.status, "204 No Content");
    assert!(reordered.body.is_empty());

    let collection_items = crate::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}/items"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let collection_items_json =
        serde_json::from_str::<serde_json::Value>(&collection_items.body).unwrap();
    assert_eq!(collection_items_json[0]["id"], item_id);
    assert!(collection_items_json[0].get("addedAt").is_none());

    let removed_item = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(removed_item.status, "204 No Content");
    let removed_collection = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(removed_collection.status, "204 No Content");

    let invalid_content_id = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/contentid/register",
        None,
        r#"{"externalId":"mbid","contentId":"invalid"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid_content_id.status, "400 Bad Request");
    let valid_content_id = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/contentid/register",
        None,
        r#"{"externalId":"mbid","contentId":"content:music:recording:mbid"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(valid_content_id.status, "200 OK");
    assert!(valid_content_id
        .body
        .contains("mapping registered successfully"));
    for (path, expected) in [
        (
            "/api/v0/mediacore/contentid/exists/mbid",
            r#"{"exists":true}"#,
        ),
        (
            "/api/v0/mediacore/contentid/external/content%3Amusic%3Arecording%3Ambid",
            r#"{"externalIds":["mbid"]}"#,
        ),
        (
            "/api/v0/mediacore/contentid/domain/music",
            r#""contentIds":["content:music:recording:mbid"]"#,
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            r#""contentIds":["content:music:recording:mbid"]"#,
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let now_playing = crate::route_http_request(
        "PUT",
        "/api/v0/nowplaying",
        None,
        r#"{"artist":"Artist","title":"Track","album":"Album"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(now_playing.status, "204 No Content");
    assert!(now_playing.body.is_empty());

    let invalid_content_pod = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:invalid","name":"Route Audit","visibility":"Listed","contentId":"content:music:recording:route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid_content_pod.status, "400 Bad Request");
    let content_pod = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:00000000000000000000000000000001","name":"Route Audit","visibility":"Listed","contentId":"content:music:recording:route-audit","tags":[],"channels":[],"externalBindings":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(content_pod.status, "201 Created");
    let content_pod_json = serde_json::from_str::<serde_json::Value>(&content_pod.body).unwrap();
    assert_eq!(
        content_pod_json["focusContentId"],
        "content:music:recording:route-audit"
    );

    let wishlist_item = crate::route_http_request(
        "POST",
        "/api/v0/wishlist",
        None,
        r#"{"searchText":"route-audit","filter":"route-audit","enabled":true,"autoDownload":true,"maxResults":1,"maxDownloads":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(wishlist_item.status, "201 Created");
    let wishlist_json = serde_json::from_str::<serde_json::Value>(&wishlist_item.body).unwrap();
    assert!(uuid::Uuid::parse_str(wishlist_json["id"].as_str().unwrap()).is_ok());
    assert_eq!(wishlist_json["searchText"], "route-audit");
    assert_eq!(wishlist_json["maxResults"], 1);
    assert!(
        chrono::DateTime::parse_from_rfc3339(wishlist_json["createdAt"].as_str().unwrap()).is_ok()
    );
    assert!(wishlist_json.get("artist").is_none());

    let wishlist_id = wishlist_json["id"].as_str().unwrap();
    let updated_wishlist = crate::route_http_request(
        "PUT",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        r#"{"searchText":"updated-route-audit","filter":"lossless","enabled":true,"autoDownload":false,"maxResults":5,"maxDownloads":1}"#,
        &state,
    )
    .await
    .unwrap();
    let updated_wishlist_json =
        serde_json::from_str::<serde_json::Value>(&updated_wishlist.body).unwrap();
    assert_eq!(updated_wishlist_json["searchText"], "updated-route-audit");
    assert_eq!(updated_wishlist_json["maxResults"], 5);

    let ignored_result = crate::route_http_request(
        "POST",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results"),
        None,
        r#"{"username":"route-audit-peer","directory":"/tmp/slskdn-route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(ignored_result.status, "201 Created");
    let ignored_json = serde_json::from_str::<serde_json::Value>(&ignored_result.body).unwrap();
    assert_eq!(ignored_json["wishlistItemId"], wishlist_id);
    assert_eq!(ignored_json["directory"], "/tmp/slskdn-route-audit");
    assert!(
        chrono::DateTime::parse_from_rfc3339(ignored_json["createdAt"].as_str().unwrap()).is_ok()
    );
    let ignored_id = ignored_json["id"].as_str().unwrap();
    let deleted_ignored = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results/{ignored_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted_ignored.status, "204 No Content");
    let marked_viewed = crate::route_http_request(
        "POST",
        &format!("/api/v0/wishlist/{wishlist_id}/mark-viewed"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(marked_viewed.status, "204 No Content");
    let deleted_wishlist = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted_wishlist.status, "204 No Content");

    let block = crate::route_http_request(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        None,
        r#"{"ip":"192.0.2.1","reason":"contract"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(block.status, "200 OK");
    assert_eq!(block.body, r#"{"message":"IP address blocked"}"#);

    let invalid_jury = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"evidence":[],"jurors":[],"minJurorVotes":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid_jury.status, "400 Bad Request");
    assert!(invalid_jury.body.contains("trusted juror"));
    assert!(invalid_jury.body.contains("minimal evidence"));

    let security_ban = crate::route_http_request(
        "POST",
        "/api/v0/security/bans/ip",
        None,
        r#"{"ipAddress":"198.51.100.1","reason":"contract"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(security_ban.status, "200 OK");
    assert!(security_ban.body.is_empty());
    let security_bans = crate::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
        .await
        .unwrap();
    let security_bans_json =
        serde_json::from_str::<serde_json::Value>(&security_bans.body).unwrap();
    let security_record = security_bans_json
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["key"] == "IP:198.51.100.1")
        .unwrap();
    assert_eq!(security_record["reason"], "contract");
    assert_eq!(security_record["isPermanent"], false);
    assert!(
        chrono::DateTime::parse_from_rfc3339(security_record["expiresAt"].as_str().unwrap())
            .is_ok()
    );
    assert!(security_record["timeRemaining"].as_str().is_some());
}
