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
async fn controller_api_differential_compatibility_aliases_reach_state_backed_routes() {
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
        let response = super::route_http_request("GET", path, None, "", &state)
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
        super::route_http_request("GET", "/api/v0/bridge/rooms", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/backfill/stats", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/backfill/config", None, "", &state)
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
        let response = super::route_http_request("GET", route, None, "", &state)
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
    let status = super::route_http_request("GET", "/api/server/status", None, "", &state)
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
async fn controller_api_differential_native_capability_and_library_health_contracts() {
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
        super::route_http_request("GET", "/api/slskdn/capabilities", None, "", &state)
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

    let health = super::route_http_request(
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

    let invalid = super::route_http_request(
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

    let transport = super::route_http_request("GET", "/api/v0/mesh/transport", None, "", &state)
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

    let peers = super::route_http_request("GET", "/api/v0/mesh/peers", None, "", &state)
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

    let hashdb = super::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
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
        assert!(super::extended_controller_dynamic_get_route(path), "{path}");
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
        let response = super::route_http_request("GET", path, None, "", &state)
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

    let collection = super::route_http_request(
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
    let collection_item = super::route_http_request(
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

    let updated_collection = super::route_http_request(
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
    let updated_item = super::route_http_request(
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

    let reordered = super::route_http_request(
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

    let collection_items = super::route_http_request(
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

    let removed_item = super::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(removed_item.status, "204 No Content");
    let removed_collection = super::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(removed_collection.status, "204 No Content");

    let invalid_content_id = super::route_http_request(
        "POST",
        "/api/v0/mediacore/contentid/register",
        None,
        r#"{"externalId":"mbid","contentId":"invalid"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid_content_id.status, "400 Bad Request");
    let valid_content_id = super::route_http_request(
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
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let now_playing = super::route_http_request(
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

    let invalid_content_pod = super::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:invalid","name":"Route Audit","visibility":"Listed","contentId":"content:music:recording:route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid_content_pod.status, "400 Bad Request");
    let content_pod = super::route_http_request(
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

    let wishlist_item = super::route_http_request(
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
    let updated_wishlist = super::route_http_request(
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

    let ignored_result = super::route_http_request(
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
    let deleted_ignored = super::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results/{ignored_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted_ignored.status, "204 No Content");
    let marked_viewed = super::route_http_request(
        "POST",
        &format!("/api/v0/wishlist/{wishlist_id}/mark-viewed"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(marked_viewed.status, "204 No Content");
    let deleted_wishlist = super::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted_wishlist.status, "204 No Content");

    let block = super::route_http_request(
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

    let invalid_jury = super::route_http_request(
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

    let security_ban = super::route_http_request(
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
    let security_bans = super::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn collections_are_scoped_to_the_real_authenticated_caller_identity() {
    // Matches the oracle's real AuthenticatedWebUserId-based
    // ownership (CollectionsController.cs): a collection created by
    // one authenticated identity must not be visible to, or mutable
    // by, a different one. Previously `owner_user_id` was always a
    // hardcoded "Anonymous"/empty placeholder, never checked on any
    // read or write -- any caller could view/edit/delete any other
    // caller's collections.
    let keys = serde_json::json!({
        "alice": {"key": "alice-key-0123456789", "role": "readwrite", "cidr": ""},
        "bob": {"key": "bob-key-00123456789ab", "role": "readwrite", "cidr": ""},
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKD_API_KEYS_JSON", &keys.to_string()),
    );
    let alice = Some("ApiKey alice-key-0123456789");
    let bob = Some("ApiKey bob-key-00123456789ab");

    let created = super::route_http_request(
        "POST",
        "/api/v0/collections",
        alice,
        r#"{"title":"Alice Collection"}"#,
        &state,
    )
    .await
    .expect("alice creates a collection");
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created_json["ownerUserId"], "alice");
    let collection_id = created_json["id"].as_str().unwrap().to_owned();

    for (method, path, body) in [
        ("GET", format!("/api/v0/collections/{collection_id}"), ""),
        (
            "PUT",
            format!("/api/v0/collections/{collection_id}"),
            r#"{"title":"Hijacked"}"#,
        ),
        (
            "GET",
            format!("/api/v0/collections/{collection_id}/items"),
            "",
        ),
        (
            "POST",
            format!("/api/v0/collections/{collection_id}/items"),
            r#"{"contentId":"track-1"}"#,
        ),
    ] {
        let response = super::route_http_request(method, &path, bob, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    // Alice can still read/mutate her own collection.
    let alice_get = super::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice reads her own collection");
    assert_eq!(alice_get.status, "200 OK");

    // Bob's own collection list never includes Alice's collection,
    // and vice versa.
    let bob_created = super::route_http_request(
        "POST",
        "/api/v0/collections",
        bob,
        r#"{"title":"Bob Collection"}"#,
        &state,
    )
    .await
    .expect("bob creates his own collection");
    assert_eq!(bob_created.status, "201 Created");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&bob_created.body).unwrap()["ownerUserId"],
        "bob"
    );

    let alice_list = super::route_http_request("GET", "/api/v0/collections", alice, "", &state)
        .await
        .expect("alice lists collections");
    let alice_list_json = serde_json::from_str::<serde_json::Value>(&alice_list.body).unwrap();
    let alice_titles = alice_list_json
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["title"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(alice_titles, vec!["Alice Collection"]);

    let bob_list = super::route_http_request("GET", "/api/v0/collections", bob, "", &state)
        .await
        .expect("bob lists collections");
    let bob_list_json = serde_json::from_str::<serde_json::Value>(&bob_list.body).unwrap();
    let bob_titles = bob_list_json
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["title"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(bob_titles, vec!["Bob Collection"]);

    // Bob cannot delete Alice's collection.
    let bob_delete = super::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        bob,
        "",
        &state,
    )
    .await
    .expect("bob attempts to delete alice's collection");
    assert_eq!(bob_delete.status, "404 Not Found");
    let still_there = super::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice's collection still exists");
    assert_eq!(still_there.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_grants_are_scoped_to_the_real_collection_owner() {
    // Matches the oracle's real Share-Grants ownership gate
    // (SharesController.cs): a grant is owned transitively through
    // its collection, so every action 404s unless
    // `collection.OwnerUserId == currentUserId`. Previously there was
    // no ownership check anywhere -- any authenticated caller could
    // view, mutate, delete, or mint access tokens for any other
    // caller's share grants.
    let keys = serde_json::json!({
        "alice": {"key": "alice-key-0123456789", "role": "administrator", "cidr": ""},
        "bob": {"key": "bob-key-00123456789ab", "role": "administrator", "cidr": ""},
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKD_API_KEYS_JSON", &keys.to_string()),
    );
    let alice = Some("ApiKey alice-key-0123456789");
    let bob = Some("ApiKey bob-key-00123456789ab");

    let collection = super::route_http_request(
        "POST",
        "/api/v0/collections",
        alice,
        r#"{"title":"Alice Private"}"#,
        &state,
    )
    .await
    .expect("alice creates a collection");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Bob cannot create a grant against a collection he doesn't own.
    let bob_create = super::route_http_request(
        "POST",
        "/api/v0/share-grants",
        bob,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("bob attempts to grant alice's collection");
    assert_eq!(bob_create.status, "404 Not Found");

    let granted = super::route_http_request(
        "POST",
        "/api/v0/share-grants",
        alice,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("alice grants her own collection");
    assert_eq!(granted.status, "201 Created", "{}", granted.body);
    let grant_id = serde_json::from_str::<serde_json::Value>(&granted.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    for (method, path, body) in [
        ("GET", format!("/api/share-grants/{grant_id}"), ""),
        (
            "PUT",
            format!("/api/v0/share-grants/{grant_id}"),
            r#"{"permissions":"read,download"}"#,
        ),
        (
            "GET",
            format!("/api/v0/share-grants/by-collection/{collection_id}"),
            "",
        ),
        (
            "POST",
            format!("/api/v0/share-grants/{grant_id}/token"),
            "{}",
        ),
    ] {
        let response = super::route_http_request(method, &path, bob, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    // Bob's own grant list never includes Alice's grant.
    let bob_list = super::route_http_request("GET", "/api/v0/share-grants", bob, "", &state)
        .await
        .expect("bob lists share grants");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&bob_list.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Alice can still manage her own grant.
    let alice_get = super::route_http_request(
        "GET",
        &format!("/api/share-grants/{grant_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice reads her own grant");
    assert_eq!(alice_get.status, "200 OK");

    let alice_token = super::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{grant_id}/token"),
        alice,
        "{}",
        &state,
    )
    .await
    .expect("alice mints a token for her own grant");
    assert_eq!(alice_token.status, "201 Created", "{}", alice_token.body);

    // Bob cannot delete Alice's grant, either.
    let bob_delete = super::route_http_request(
        "DELETE",
        &format!("/api/v0/share-grants/{grant_id}"),
        bob,
        "",
        &state,
    )
    .await
    .expect("bob attempts to delete alice's grant");
    assert_eq!(bob_delete.status, "404 Not Found");
    let still_there = super::route_http_request(
        "GET",
        &format!("/api/share-grants/{grant_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice's grant still exists");
    assert_eq!(still_there.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn v0_share_grants_get_real_uuid_ids_usable_on_versioned_routes() {
    // Matches the oracle's real ShareGrant.Id (a Guid), and the
    // existing `versioned_get_failure_contract` UUID-format guard
    // that v0 share-grant routes already enforced -- previously
    // ShareGrantStore always minted sequential "grant-N" ids
    // regardless of API version (unlike CollectionStore, which
    // already mints a real UUID for v0 requests), so any v0
    // GET/PUT/DELETE by id always 400'd against that same guard for
    // a real grant, no matter what id was passed.
    let (state, _receiver) = test_state();
    let collection = super::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Versioned Grants"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let created = super::route_http_request(
        "POST",
        "/api/v0/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"friend"}}"#),
        &state,
    )
    .await
    .expect("create share grant via the v0 route");
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        uuid::Uuid::parse_str(&grant_id).is_ok(),
        "v0-created share grant id must be a real UUID: {grant_id}"
    );

    let get = super::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("get share grant via the v0 route");
    assert_eq!(get.status, "200 OK", "{}", get.body);

    let update = super::route_http_request(
        "PUT",
        &format!("/api/v0/share-grants/{grant_id}"),
        None,
        r#"{"permissions":"read,download"}"#,
        &state,
    )
    .await
    .expect("update share grant via the v0 route");
    assert_eq!(update.status, "200 OK", "{}", update.body);

    let delete = super::route_http_request(
        "DELETE",
        &format!("/api/v0/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete share grant via the v0 route");
    assert_eq!(delete.status, "200 OK", "{}", delete.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_reports_require_a_real_direction_not_a_silent_default() {
    let (state, _receiver) = test_state();
    for path in [
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
    ] {
        // Present but nonsense: matches the oracle's real
        // Enum.TryParse<TransferDirection> failure path -- must be a
        // real 400, not silently treated as "no filter" and mixing
        // Upload/Download rows into one report.
        let invalid = super::route_http_request(
            "GET",
            &format!("{path}?direction=sideways"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(invalid.status, "400 Bad Request", "{path}");
        assert!(
            invalid.body.contains("Invalid direction"),
            "{path}: {}",
            invalid.body
        );

        // Absent, but with an unrelated query param present (so the
        // generic "no query at all" gate doesn't short-circuit it):
        // still a real 400, matching the oracle's required Direction.
        let missing =
            super::route_http_request("GET", &format!("{path}?limit=5"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(missing.status, "400 Bad Request", "{path}");
        assert!(
            missing.body.contains("Direction is required"),
            "{path}: {}",
            missing.body
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn telemetry_kpis_are_always_json_unlike_the_base_prometheus_route() {
    let (state, _receiver) = test_state();

    let base = super::route_http_request("GET", "/api/telemetry/prometheus", None, "", &state)
        .await
        .expect("base prometheus route");
    assert!(base.content_type.starts_with("text/plain"), "{base:?}");
    assert!(base.body.contains("slskr_transfers"));

    let kpis = super::route_http_request("GET", "/api/telemetry/prometheus/kpis", None, "", &state)
        .await
        .expect("kpis route");
    // Matches the oracle's GetKpis: always application/json, a
    // dictionary keyed by metric name -- never the base route's
    // text/plain Prometheus exposition format.
    assert!(
        kpis.content_type.starts_with("application/json"),
        "{kpis:?}"
    );
    let kpis_json = serde_json::from_str::<serde_json::Value>(&kpis.body).unwrap();
    assert_eq!(kpis_json["slskr_transfers"]["type"], "gauge");
    assert_eq!(kpis_json["slskr_transfers"]["samples"][0]["value"], 0.0);
    assert_eq!(kpis_json["slskr_searches"]["type"], "gauge");
}

/// Builds a verdict body with a real, self-consistent signature --
/// the oracle's own check is content-integrity (the submitted
/// payloadHash must match a hash of the verdict's own fields), not
/// full cryptographic authentication, so a real "signer" key isn't
/// needed here, only a hash that genuinely matches.
fn quarantine_signed_verdict_json(
    request_id: &str,
    juror: &str,
    verdict: &str,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "requestId": request_id,
        "juror": juror,
        "verdict": verdict,
    });
    let payload_hash = crate::quarantine_controller::quarantine_verdict_payload_hash(&value);
    value["signature"] = serde_json::json!({
        "signer": juror,
        "payloadHash": payload_hash,
        "value": "test-signature",
    });
    value
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn quarantine_jury_requires_real_quorum_before_accepting_or_releasing() {
    let (state, _receiver) = test_state();

    let created = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"route-audit","jurors":["juror-a","juror-b","juror-c"],"evidence":[{"type":"hash","reference":"opaque-ref"}],"minJurorVotes":2}"#,
        &state,
    )
    .await
    .expect("create quarantine request");
    assert_eq!(created.status, "200 OK", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let request_id = created_json["request"]["requestId"]
        .as_str()
        .unwrap()
        .to_owned();

    // A verdict from a juror not on the request must be rejected.
    let unlisted = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &quarantine_signed_verdict_json(&request_id, "not-a-juror", "ReleaseCandidate").to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unlisted.status, "400 Bad Request");

    // A verdict without a real signature (or with a payload hash that
    // doesn't match its own contents) must be rejected too -- not
    // silently accepted the way the old handler took any verdict
    // shape it was given.
    let unsigned = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &format!(
            r#"{{"requestId":"{request_id}","juror":"juror-a","verdict":"ReleaseCandidate"}}"#
        ),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unsigned.status, "400 Bad Request");
    let unsigned_json = serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap();
    assert!(
        unsigned_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Signed juror verdict is required."),
        "{unsigned_json}"
    );

    let mut tampered = quarantine_signed_verdict_json(&request_id, "juror-a", "ReleaseCandidate");
    tampered["verdict"] = serde_json::json!("UpholdQuarantine");
    let tampered_result = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &tampered.to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(tampered_result.status, "400 Bad Request");
    let tampered_json = serde_json::from_str::<serde_json::Value>(&tampered_result.body).unwrap();
    assert!(
        tampered_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Signature payload hash does not match verdict contents."),
        "{tampered_json}"
    );

    // Before quorum (minJurorVotes=2), acceptance and release must be
    // denied -- not silently accepted, as the old fake handlers did.
    let too_early = super::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(too_early.status, "400 Bad Request", "{}", too_early.body);
    let too_early_json = serde_json::from_str::<serde_json::Value>(&too_early.body).unwrap();
    assert_eq!(too_early_json["isAccepted"], false);

    let package_too_early = super::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/release-package"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(package_too_early.status, "400 Bad Request");
    let package_too_early_json =
        serde_json::from_str::<serde_json::Value>(&package_too_early.body).unwrap();
    assert_eq!(package_too_early_json["isReady"], false);
    assert!(package_too_early_json["errors"][0]
        .as_str()
        .unwrap()
        .contains("has not been accepted"));

    // Two of three jurors vote ReleaseCandidate -- quorum (2) reached
    // and a real 2/3 supermajority.
    for juror in ["juror-a", "juror-b"] {
        let verdict = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(&request_id, juror, "ReleaseCandidate").to_string(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(verdict.status, "200 OK", "{}", verdict.body);
    }

    let aggregate = super::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/aggregate"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(aggregate.status, "200 OK");
    let aggregate_json = serde_json::from_str::<serde_json::Value>(&aggregate.body).unwrap();
    assert_eq!(aggregate_json["recommendation"], "ReleaseCandidate");
    assert_eq!(aggregate_json["totalVerdicts"], 2);
    assert_eq!(aggregate_json["requiredVotes"], 2);
    assert_eq!(aggregate_json["quorumReached"], true);

    let review = super::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/review"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let review_json = serde_json::from_str::<serde_json::Value>(&review.body).unwrap();
    assert_eq!(review_json["canAcceptReleaseCandidate"], true);
    assert_eq!(review_json["verdicts"].as_array().unwrap().len(), 2);

    // Real quorum reached: acceptance must now succeed.
    let accepted = super::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        r#"{"acceptedBy":"route-audit-operator"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(accepted.status, "200 OK", "{}", accepted.body);
    let accepted_json = serde_json::from_str::<serde_json::Value>(&accepted.body).unwrap();
    assert_eq!(accepted_json["isAccepted"], true);
    assert_eq!(
        accepted_json["decision"]["acceptedBy"],
        "route-audit-operator"
    );

    // Re-accepting is idempotent, not a second decision.
    let reaccepted = super::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(reaccepted.status, "200 OK");
    let reaccepted_json = serde_json::from_str::<serde_json::Value>(&reaccepted.body).unwrap();
    assert_eq!(
        reaccepted_json["decision"]["id"],
        accepted_json["decision"]["id"]
    );

    // Now that a real acceptance decision exists, the release package
    // must be built from real stored data.
    let package = super::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/release-package"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(package.status, "200 OK", "{}", package.body);
    let package_json = serde_json::from_str::<serde_json::Value>(&package.body).unwrap();
    assert_eq!(package_json["isReady"], true);
    assert_eq!(package_json["package"]["requestId"], request_id);
    assert_eq!(
        package_json["package"]["currentAggregate"]["recommendation"],
        "ReleaseCandidate"
    );
    assert_eq!(
        package_json["package"]["verdicts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Routing to a juror not on the request must be rejected before
    // ever reaching the (currently unwired) routing backend.
    let bad_route = super::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/routes"),
        None,
        r#"{"targetJurors":["not-a-juror"]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(bad_route.status, "400 Bad Request");
    let bad_route_json = serde_json::from_str::<serde_json::Value>(&bad_route.body).unwrap();
    assert_eq!(bad_route_json["success"], false);
    assert!(bad_route_json["errorMessage"]
        .as_str()
        .unwrap()
        .contains("safe jurors"));

    // Valid jurors pass validation but hit the same "routing backend
    // unavailable" outcome the oracle itself reports when its router
    // isn't configured -- a real code path, not a fabricated success.
    let real_route = super::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/routes"),
        None,
        r#"{"targetJurors":["juror-a"]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(real_route.status, "400 Bad Request");
    let real_route_json = serde_json::from_str::<serde_json::Value>(&real_route.body).unwrap();
    assert_eq!(real_route_json["success"], false);
    assert_eq!(
        real_route_json["errorMessage"],
        "Routing backend is not available."
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn native_versioned_extended_gets_match_empty_state_contracts() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let metadata = super::route_http_request(
        "GET",
        "/api/v0/hashdb/metadata-processing?limit=0",
        None,
        "",
        &state,
    )
    .await
    .expect("metadata processing status");
    assert_eq!(metadata.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&metadata.body).unwrap(),
        serde_json::json!({"active": [], "history": []})
    );

    let transports = super::route_http_request(
        "GET",
        "/api/v0/security/transports/status",
        None,
        "",
        &state,
    )
    .await
    .expect("transport selector status");
    assert_eq!(transports.status, "200 OK");
    let transports = serde_json::from_str::<serde_json::Value>(&transports.body).unwrap();
    assert_eq!(transports["selectedMode"], "Direct");
    assert_eq!(transports["totalTransports"], 1);
    assert_eq!(transports["availableTransports"], 0);
    assert_eq!(transports["availableTransportTypes"], serde_json::json!([]));
    assert!(transports["lastConnectivityTest"].is_string());
    assert_eq!(transports["primaryTransportAvailable"], false);
    assert_eq!(transports["fallbackAvailable"], false);

    let listening_party = super::route_http_request(
        "GET",
        "/api/v0/listening-party/pod:route-audit/route-audit-channel",
        None,
        "",
        &state,
    )
    .await
    .expect("empty listening-party state");
    assert_eq!(listening_party.status, "204 No Content");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn listening_party_requires_membership_and_reports_a_real_event() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-audit";
    let channel_id = "general";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Audit",
            }))
            .expect("deserialize pod record fixture"),
            "tester".to_owned(),
        )
        .expect("create pod (tester is the owner/member)");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    // A pod that exists, where "tester" (the fixture's default
    // configured identity) is not a member, must reject both GET and
    // POST with a real 403 -- never leak state or accept an event.
    let outside_pod = "pod:listening-party-outsider";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": outside_pod,
                "name": "Not Tester's Pod",
            }))
            .expect("deserialize pod record fixture"),
            "someone-else".to_owned(),
        )
        .expect("create outsider pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            outside_pod,
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create outsider channel");
    let forbidden_get = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{outside_pod}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forbidden get");
    assert_eq!(
        forbidden_get.status, "403 Forbidden",
        "{}",
        forbidden_get.body
    );
    let forbidden_post = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{outside_pod}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:x"}"#,
        &state,
    )
    .await
    .expect("forbidden post");
    assert_eq!(
        forbidden_post.status, "403 Forbidden",
        "{}",
        forbidden_post.body
    );

    // An invalid action is a real 400, matching the oracle's real
    // play|pause|seek|stop vocabulary -- not slskR's old invented
    // {kind, action} pair.
    let invalid_action = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"resume","contentId":"content:audio:track:x"}"#,
        &state,
    )
    .await
    .expect("invalid action");
    assert_eq!(invalid_action.status, "400 Bad Request");

    // No prior state: a real 204, not a fabricated pod+chat-history
    // payload.
    let before = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("no state yet");
    assert_eq!(before.status, "204 No Content", "{}", before.body);

    let played = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:x","title":"Track","artist":"Artist","hostPeerId":"forged-peer"}"#,
        &state,
    )
    .await
    .expect("play event");
    assert_eq!(played.status, "200 OK", "{}", played.body);
    let played_json = serde_json::from_str::<serde_json::Value>(&played.body).unwrap();
    assert_eq!(played_json["action"], "play");
    assert_eq!(played_json["podId"], pod_id);
    assert_eq!(played_json["channelId"], channel_id);
    // hostPeerId is always the authenticated local identity, never
    // the client-supplied value -- a forged value must not survive.
    assert_eq!(played_json["hostPeerId"], "tester");
    assert_eq!(played_json["kind"], "slskdn.listenAlong.v1");
    assert!(played_json["partyId"]
        .as_str()
        .unwrap()
        .starts_with("party:"));

    // GET must now return the exact same real, persisted event.
    let polled = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("poll event");
    assert_eq!(polled.status, "200 OK", "{}", polled.body);
    assert_eq!(polled.body, played.body);

    // A "stop" event clears the stored state entirely.
    let stopped = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop event");
    assert_eq!(stopped.status, "200 OK", "{}", stopped.body);
    let after_stop = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("state after stop");
    assert_eq!(after_stop.status, "204 No Content", "{}", after_stop.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn listening_party_directory_reflects_real_listed_events() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-directory-audit";
    let channel_id = "general";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Directory Audit",
            }))
            .expect("deserialize pod record fixture"),
            "tester".to_owned(),
        )
        .expect("create pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    // An unlisted event must never appear in the public directory.
    let unlisted = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:unlisted","listed":false}"#,
        &state,
    )
    .await
    .expect("unlisted play event");
    assert_eq!(unlisted.status, "200 OK", "{}", unlisted.body);
    let after_unlisted =
        super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .expect("directory after unlisted event");
    assert_eq!(
        after_unlisted.body, "[]",
        "an unlisted event must not appear in the directory"
    );

    // A listed event must appear with the real event's data.
    let listed = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:listed","title":"Directory Track","artist":"Directory Artist","listed":true,"allowMeshStreaming":true}"#,
        &state,
    )
    .await
    .expect("listed play event");
    assert_eq!(listed.status, "200 OK", "{}", listed.body);
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    let party_id = listed_json["partyId"].as_str().unwrap().to_owned();

    let directory = super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
        .await
        .expect("directory after listed event");
    let directory_json = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap();
    let entries = directory_json.as_array().unwrap();
    assert_eq!(entries.len(), 1, "{directory_json}");
    assert_eq!(entries[0]["partyId"], party_id);
    assert_eq!(entries[0]["podId"], pod_id);
    assert_eq!(entries[0]["channelId"], channel_id);
    assert_eq!(entries[0]["hostPeerId"], "tester");
    assert_eq!(entries[0]["title"], "Directory Track");
    assert_eq!(entries[0]["artist"], "Directory Artist");
    assert_eq!(entries[0]["contentId"], "content:audio:track:listed");
    assert_eq!(entries[0]["allowMeshStreaming"], true);
    assert_eq!(entries[0]["kind"], "slskdn.listeningParty.announce.v1");
    assert!(entries[0]["streamPath"].as_str().is_some_and(|path| path
        .starts_with("/api/v0/listening-party/radio/")
        && path.contains("?ticket=")));
    assert!(
        entries[0]["expiresAtUnixMs"].as_u64().unwrap()
            > entries[0]["startedAtUnixMs"].as_u64().unwrap()
    );
    assert_eq!(
        entries[0]["lastSeenUnixMs"], entries[0]["startedAtUnixMs"],
        "directory reads must not refresh a persisted party"
    );

    // A persisted announcement must age out from its event timestamp. If
    // the directory used the request time as last-seen, this stale record
    // would incorrectly appear beside the live party.
    let stale_timestamp = super::unix_timestamp_millis().saturating_sub(
        super::extended_controller::LISTENING_PARTY_ANNOUNCEMENT_TTL_MS.saturating_add(1),
    );
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            "listening-party/stale-pod/stale-channel".to_owned(),
            serde_json::json!({
                "partyId": "party:stale",
                "podId": "pod:stale",
                "channelId": "stale-channel",
                "hostPeerId": "stale-peer",
                "action": "play",
                "contentId": "stale-content",
                "serverTimeUnixMs": stale_timestamp,
                "listed": true,
                "allowMeshStreaming": false,
            }),
        )
        .expect("persist stale listening-party fixture");
    let with_stale = super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
        .await
        .expect("directory with stale event");
    let with_stale_json = serde_json::from_str::<serde_json::Value>(&with_stale.body).unwrap();
    let with_stale_entries = with_stale_json.as_array().unwrap();
    assert_eq!(with_stale_entries.len(), 1, "{with_stale_json}");
    assert_eq!(with_stale_entries[0]["partyId"], party_id);

    // Stopping the party removes it from the directory entirely.
    let stopped = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop event");
    assert_eq!(stopped.status, "200 OK", "{}", stopped.body);
    let after_stop_directory =
        super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .expect("directory after stop");
    assert_eq!(after_stop_directory.body, "[]");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn listening_party_radio_requires_a_real_ticket_matching_the_content_id() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-radio-audit";
    let channel_id = "general";
    let content_id = "radio-audit-content";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Radio Audit",
            }))
            .expect("deserialize pod record fixture"),
            "tester".to_owned(),
        )
        .expect("create pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    // A real, listed, mesh-streaming-enabled party actually playing
    // this exact contentId -- matches the oracle's real StreamListedParty
    // party-state gate, which is evaluated before any ticket at all.
    let played = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        &format!(
            r#"{{"action":"play","contentId":"{content_id}","title":"Radio Track","artist":"Radio Artist","listed":true,"allowMeshStreaming":true}}"#
        ),
        &state,
    )
    .await
    .expect("play event");
    assert_eq!(played.status, "200 OK", "{}", played.body);
    let party_id = serde_json::from_str::<serde_json::Value>(&played.body).unwrap()["partyId"]
        .as_str()
        .unwrap()
        .to_owned();

    // No ticket at all -- must not leak availability/peer data to an
    // unauthenticated probe.
    let no_ticket = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("no ticket request");
    assert_eq!(no_ticket.status, "401 Unauthorized", "{}", no_ticket.body);

    // A real ticket, but issued for a different piece of content --
    // must not authorize access to this contentId.
    let mismatched_ticket_response = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"some-other-content","filename":"Other.flac","peerId":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("create mismatched ticket");
    assert_eq!(mismatched_ticket_response.status, "200 OK");
    let mismatched_ticket =
        serde_json::from_str::<serde_json::Value>(&mismatched_ticket_response.body).unwrap()
            ["ticket"]
            .as_str()
            .unwrap()
            .to_owned();
    let mismatched = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={mismatched_ticket}"
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("mismatched ticket request");
    assert_eq!(mismatched.status, "401 Unauthorized", "{}", mismatched.body);

    // The frozen directory service issues a ticket owned by this exact
    // party. That ticket authorizes the exact contentId, and the
    // response reflects the real content-discovery shadow record (not a
    // fake/empty stub).
    state
        .content_discovery
        .write()
        .await
        .merge_shadow_records(vec![super::content_discovery::ShadowIndexRecord {
            recording_id: content_id.to_owned(),
            peer_ids: vec!["peer-a".to_owned(), "peer-b".to_owned()],
            updated_at: 0,
        }])
        .expect("seed shadow record");
    let valid_ticket = super::issue_listening_party_stream_ticket(&state, &party_id, content_id)
        .await
        .expect("create valid listening-party ticket");
    let authorized = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={valid_ticket}"),
        None,
        "",
        &state,
    )
    .await
    .expect("authorized request");
    assert_eq!(authorized.status, "200 OK", "{}", authorized.body);
    let authorized_json = serde_json::from_str::<serde_json::Value>(&authorized.body).unwrap();
    assert_eq!(authorized_json["partyId"], party_id);
    assert_eq!(authorized_json["contentId"], content_id);
    assert_eq!(authorized_json["available"], true);
    assert_eq!(
        authorized_json["peerIds"],
        serde_json::json!(["peer-a", "peer-b"])
    );

    let stopped = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop party and revoke ticket");
    assert_eq!(stopped.status, "200 OK", "{}", stopped.body);
    let mut tickets = state.stream_tickets.write().await;
    assert!(
        tickets.get(&valid_ticket).is_none(),
        "stopping a party must revoke tickets owned by its stored party id"
    );
}
