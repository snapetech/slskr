#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn bridge_admin_clients_never_leaks_unrelated_peer_activity() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    // Real, unrelated peer activity: an online watched user and a
    // real peer capability record. Neither is a legacy client
    // connected to the embedded Soulfind bridge listener, so neither
    // must appear in the bridge client list.
    {
        let mut users = state.users.write().await;
        users.watch("online-peer".to_owned());
        if let Some(record) = users
            .records
            .iter_mut()
            .find(|record| record.username == "online-peer")
        {
            record.status = Some("online".to_owned());
        }
    }
    let clients = super::route_http_request("GET", "/api/bridge/admin/clients", None, "", &state)
        .await
        .expect("bridge clients");
    assert_eq!(clients.status, "200 OK", "{}", clients.body);
    let clients_json = serde_json::from_str::<serde_json::Value>(&clients.body).unwrap();
    assert_eq!(
        clients_json,
        serde_json::json!({"clients": [], "count": 0, "status": "disabled", "ready": false}),
        "{clients_json}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn bridge_search_and_download_use_real_oracle_shapes() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    add_test_share(
        &state,
        "Virtual/Bridge Track.flac",
        Path::new("/nonexistent/bridge-track.flac"),
        4096,
    )
    .await;

    // Matches the oracle's real BridgeSearchResult{Query,
    // Users[{PeerId, Username, Files[...]}]} contract, not the
    // generic /api/search record shape.
    let search = super::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"Bridge Track"}"#,
        &state,
    )
    .await
    .expect("bridge search");
    assert_eq!(search.status, "201 Created", "{}", search.body);
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap();
    assert_eq!(search_json["query"], "Bridge Track");
    let users = search_json["users"].as_array().unwrap();
    assert_eq!(users.len(), 1, "{search_json}");
    assert!(users[0].get("peerId").is_some(), "{search_json}");
    assert!(users[0].get("username").is_some(), "{search_json}");
    let files = users[0]["files"].as_array().unwrap();
    assert_eq!(files.len(), 1, "{search_json}");
    assert_eq!(files[0]["path"], "Virtual/Bridge Track.flac");
    assert_eq!(files[0]["sizeBytes"], 4096);
    assert_eq!(files[0]["codec"], "flac");

    // A query with no matches must report a real empty users list,
    // not a synthetic empty-file peer entry.
    let empty_search = super::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"nothing-matches-this"}"#,
        &state,
    )
    .await
    .expect("bridge search with no matches");
    let empty_search_json = serde_json::from_str::<serde_json::Value>(&empty_search.body).unwrap();
    assert_eq!(empty_search_json["users"], serde_json::json!([]));

    // Matches the oracle's real single-item BridgeDownloadRequest /
    // {transfer_id} contract, not the generic /api/downloads
    // batch shape.
    let download = super::route_http_request(
        "POST",
        "/api/v0/bridge/download",
        None,
        r#"{"username":"peer","filename":"Virtual/Bridge Track.flac","targetPath":"/tmp/out.flac"}"#,
        &state,
    )
    .await
    .expect("bridge download");
    assert_eq!(download.status, "200 OK", "{}", download.body);
    let download_json = serde_json::from_str::<serde_json::Value>(&download.body).unwrap();
    assert!(download_json["transfer_id"].is_string(), "{download_json}");
    assert!(
        download_json.get("downloadIds").is_none(),
        "{download_json}"
    );
    assert!(download_json.get("enqueued").is_none(), "{download_json}");
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
async fn mediacore_mutations_match_native_validation_and_result_dtos() {
    let (state, _receiver) = test_state();

    let fuzzy_text = super::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/text",
        None,
        r#"{"textA":"same","textB":"same"}"#,
        &state,
    )
    .await
    .unwrap();
    let fuzzy_text = serde_json::from_str::<serde_json::Value>(&fuzzy_text.body).unwrap();
    assert_eq!(fuzzy_text["levenshteinSimilarity"], 1.0);
    assert_eq!(fuzzy_text["phoneticSimilarity"], 1.0);
    assert_eq!(fuzzy_text["combinedSimilarity"], 1.0);

    let fuzzy_perceptual = super::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/perceptual",
        None,
        r#"{"contentIdA":"content:music:recording:a","contentIdB":"content:music:recording:b"}"#,
        &state,
    )
    .await
    .unwrap();
    let fuzzy_perceptual =
        serde_json::from_str::<serde_json::Value>(&fuzzy_perceptual.body).unwrap();
    assert_eq!(fuzzy_perceptual["similarity"], 0.0);
    assert_eq!(fuzzy_perceptual["isSimilar"], false);
    assert_eq!(fuzzy_perceptual["threshold"], 0.7);

    for (path, body) in [
        (
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[0.5],"sampleRate":1,"algorithm":"PHash"}"#,
        ),
        (
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":1,"height":1,"algorithm":"PHash"}"#,
        ),
    ] {
        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .unwrap();
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
        let response = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(response["algorithm"], "PHash");
        assert_eq!(response["hex"], "0000000000000000");
        assert_eq!(response["numericHash"], 0);
    }
    let similarity = super::route_http_request(
        "POST",
        "/api/v0/mediacore/perceptualhash/similarity",
        None,
        r#"{"hashA":"0000000000000000","hashB":"ffffffffffffffff","threshold":0.8}"#,
        &state,
    )
    .await
    .unwrap();
    let similarity = serde_json::from_str::<serde_json::Value>(&similarity.body).unwrap();
    assert_eq!(similarity["hammingDistance"], 64);
    assert_eq!(similarity["similarity"], 0.0);
    assert_eq!(similarity["areSimilar"], false);

    for (path, body) in [
        (
            "/api/v0/mediacore/ipld/links/content:test:type:id",
            r#"{"links":[]}"#,
        ),
        (
            "/api/v0/mediacore/portability/export",
            r#"{"contentIds":[]}"#,
        ),
        (
            "/api/v0/mediacore/publish/republish",
            r#"{"contentIds":[]}"#,
        ),
        (
            "/api/v0/mediacore/publish/descriptor",
            r#"{"descriptor":{"contentId":"content:test:type:id"}}"#,
        ),
    ] {
        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .unwrap();
        assert_eq!(
            response.status, "400 Bad Request",
            "{path}: {}",
            response.body
        );
    }

    let empty_package = r#"{"package":{"version":"1.0","exportedAt":"2026-01-01T00:00:00Z","source":"test","entries":[],"links":[],"metadata":{"totalEntries":0,"totalLinks":0,"entriesByDomain":{},"checksum":""}}}"#;
    let analysis = super::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/analyze",
        None,
        empty_package,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        analysis.body,
        r#"{"cleanEntries":0,"conflictingEntries":0,"conflicts":[],"recommendedStrategies":{"Merge":0,"Overwrite":0,"Skip":0},"totalEntries":0}"#
    );

    let imported = super::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/import",
        None,
        empty_package,
        &state,
    )
    .await
    .unwrap();
    let imported = serde_json::from_str::<serde_json::Value>(&imported.body).unwrap();
    for key in [
        "success",
        "entriesProcessed",
        "entriesImported",
        "entriesSkipped",
        "conflictsResolved",
        "conflicts",
        "errors",
        "duration",
    ] {
        assert!(
            imported.get(key).is_some(),
            "missing import result field {key}"
        );
    }

    let verify = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/verify",
        None,
        r#"{"descriptor":{"contentId":"content:test:type:id","hashes":[]}}"#,
        &state,
    )
    .await
    .unwrap();
    let verify = serde_json::from_str::<serde_json::Value>(&verify.body).unwrap();
    assert_eq!(verify["isValid"], false);
    assert_eq!(verify["signatureValid"], false);
    assert_eq!(verify["freshnessValid"], false);
    assert!(verify["validationError"].as_str().is_some());

    let cache = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        cache.body,
        r#"{"bytesFreed":0,"entriesCleared":0,"success":true}"#
    );

    let reset =
        super::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .unwrap();
    assert_eq!(reset.body, r#"{"message":"Statistics reset successfully"}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mediacore_versioned_descriptor_delete_does_not_overflow_worker_stack() {
    let (state, _receiver) = test_state();
    let response = super::route_http_request(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/content-id",
        None,
        "",
        &state,
    )
    .await
    .expect("delete descriptor");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap(),
        serde_json::json!({
            "contentId": "content-id",
            "success": true,
            "wasPublished": false,
        })
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mediacore_descriptor_updates_imports_and_stats_use_real_records() {
    let (state, _receiver) = test_state();
    let content_id = "content:test:recording:real";
    let descriptor = serde_json::json!({
        "contentId": content_id,
        "title": "before",
        "hashes": [{"algorithm": "sha256", "hex": "aaaa"}],
        "signature": {
            "publicKey": "key",
            "signature": "signature",
            "timestampUnixMs": super::unix_timestamp_millis(),
        },
    });
    let published = super::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        None,
        &serde_json::json!({"descriptor": descriptor}).to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(published.status, "200 OK", "{}", published.body);

    let updated = super::route_http_request(
        "PUT",
        &format!("/api/v0/mediacore/publish/descriptor/{content_id}"),
        None,
        r#"{"updates":{"title":"after","genre":"ambient"}}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "400 Bad Request", "{}", updated.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.body).unwrap(),
        serde_json::json!({"error": "Failed to update descriptor"})
    );

    let retrieved = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        None,
        &serde_json::json!({"contentIds":[content_id,"content:test:recording:missing"]})
            .to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(retrieved.status, "200 OK", "{}", retrieved.body);
    let retrieved_json = serde_json::from_str::<serde_json::Value>(&retrieved.body).unwrap();
    assert_eq!(retrieved_json["requested"], 2);
    assert_eq!(retrieved_json["found"], 1);
    assert_eq!(retrieved_json["results"].as_array().unwrap().len(), 2);

    let stats =
        super::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
            .await
            .unwrap();
    let stats = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats["totalRetrievals"], 2);
    assert_eq!(stats["cacheHits"], 0);
    assert_eq!(stats["cacheMisses"], 2);
    assert_eq!(stats["activeCacheEntries"], 1);

    let cached = super::route_http_request(
        "GET",
        &format!("/api/v0/mediacore/retrieve/descriptor/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let cached_json = serde_json::from_str::<serde_json::Value>(&cached.body).unwrap();
    assert_eq!(cached.status, "200 OK");
    assert_eq!(cached_json["found"], true);
    assert_eq!(cached_json["fromCache"], true);

    let imported = super::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/import",
        None,
        &serde_json::json!({
            "package": {
                "entries": [{
                    "contentId": "content:test:recording:imported",
                    "descriptor": {
                        "contentId": "content:test:recording:imported",
                        "title": "imported",
                        "hashes": [{"algorithm": "sha256", "hex": "bbbb"}]
                    }
                }]
            }
        })
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    let imported = serde_json::from_str::<serde_json::Value>(&imported.body).unwrap();
    assert_eq!(imported["success"], true);
    assert_eq!(imported["entriesProcessed"], 1);
    assert_eq!(imported["entriesImported"], 1);

    let publishing = super::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/publishing",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let publishing = serde_json::from_str::<serde_json::Value>(&publishing.body).unwrap();
    assert_eq!(publishing["totalPublished"], 1);
    assert_eq!(publishing["activePublications"], 1);
    assert_eq!(publishing["publicationsByDomain"]["test"], 1);

    let publisher_stats =
        super::route_http_request("GET", "/api/v0/mediacore/publish/stats", None, "", &state)
            .await
            .unwrap();
    let publisher_stats = serde_json::from_str::<serde_json::Value>(&publisher_stats.body).unwrap();
    assert_eq!(publisher_stats["totalPublishedDescriptors"], 1);
    assert_eq!(publisher_stats["activePublications"], 1);
    assert_eq!(publisher_stats["publicationsByDomain"]["test"], 1);
    assert!(publisher_stats["averageTtlHours"].as_f64().unwrap() > 0.0);

    let reset =
        super::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .unwrap();
    assert_eq!(reset.body, r#"{"message":"Statistics reset successfully"}"#);
    let stats =
        super::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stats.body).unwrap()["totalRetrievals"],
        0
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mediacore_fuzzy_matching_and_ipld_validation_use_persisted_state() {
    let (state, _receiver) = test_state();
    let target = "content:audio:track:target";
    let candidate = "content:audio:track:candidate";
    for (external_id, content_id) in [("target", target), ("candidate", candidate)] {
        let response = super::route_http_request(
            "POST",
            "/api/v0/mediacore/contentid/register",
            None,
            &serde_json::json!({"externalId": external_id, "contentId": content_id}).to_string(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "200 OK", "{}", response.body);
        let response = super::route_http_request(
            "POST",
            "/api/v0/mediacore/publish/descriptor",
            None,
            &serde_json::json!({
                "descriptor": {
                    "contentId": content_id,
                    "hashes": [{"algorithm": "sha256", "hex": external_id}],
                    "perceptualHashes": [{
                        "algorithm": "Chromaprint",
                        "hex": "0000000000001234",
                        "numericHash": 0x1234_u64,
                    }],
                    "signature": {
                        "publicKey": "key",
                        "signature": "signature",
                        "timestampUnixMs": super::unix_timestamp_millis(),
                    },
                },
            })
            .to_string(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "200 OK", "{}", response.body);
    }

    let perceptual = super::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/perceptual",
        None,
        &serde_json::json!({"contentIdA": target, "contentIdB": candidate}).to_string(),
        &state,
    )
    .await
    .unwrap();
    let perceptual = serde_json::from_str::<serde_json::Value>(&perceptual.body).unwrap();
    assert_eq!(perceptual["similarity"], 1.0);
    assert_eq!(perceptual["isSimilar"], true);

    let matches = super::route_http_request(
        "POST",
        &format!("/api/v0/mediacore/fuzzymatch/find/{target}"),
        None,
        r#"{"minConfidence":0.7,"maxCandidates":50,"maxResults":10}"#,
        &state,
    )
    .await
    .unwrap();
    let matches = serde_json::from_str::<serde_json::Value>(&matches.body).unwrap();
    assert_eq!(matches["totalCandidates"], 1);
    assert_eq!(matches["matches"][0]["candidateContentId"], candidate);
    assert_eq!(matches["matches"][0]["reason"], "PerceptualHash");

    let links = super::route_http_request(
        "POST",
        &format!("/api/v0/mediacore/ipld/links/{target}"),
        None,
        &serde_json::json!({"links": [
            {"name": "same", "target": candidate},
            {"name": "broken", "target": "content:audio:track:missing"},
        ]})
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(links.status, "200 OK", "{}", links.body);
    let orphan = super::route_http_request(
        "POST",
        "/api/v0/mediacore/ipld/links/content:audio:track:orphan",
        None,
        &serde_json::json!({"links": [{"name": "same", "target": candidate}]}).to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(orphan.status, "200 OK", "{}", orphan.body);

    let validation =
        super::route_http_request("GET", "/api/v0/mediacore/ipld/validate", None, "", &state)
            .await
            .unwrap();
    let validation = serde_json::from_str::<serde_json::Value>(&validation.body).unwrap();
    assert_eq!(validation["isValid"], false);
    assert_eq!(validation["totalLinksValidated"], 2);
    assert_eq!(validation["brokenLinks"].as_array().unwrap().len(), 1);
    assert_eq!(validation["orphanedLinks"].as_array().unwrap().len(), 1);

    let inbound = super::route_http_request(
        "GET",
        &format!("/api/v0/mediacore/ipld/inbound/{candidate}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let inbound = serde_json::from_str::<serde_json::Value>(&inbound.body).unwrap();
    assert_eq!(inbound["inboundLinks"].as_array().unwrap().len(), 2);
    assert!(inbound["inboundLinks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source == target));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn fuzzy_edit_distance_uses_the_shorter_rolling_dimension() {
    let long = "a".repeat(2_048);
    assert_eq!(
        super::mediacore_controller::levenshtein_similarity("b", &long),
        super::mediacore_controller::levenshtein_similarity(&long, "b")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn fuzzy_edit_distance_trims_shared_affixes_without_changing_score() {
    assert_eq!(
        super::mediacore_controller::levenshtein_similarity(
            "prefixkitten-suffix",
            "prefixsitting-suffix",
        ),
        0.85
    );
    let prefix = "a".repeat(20_000);
    assert_eq!(
        super::mediacore_controller::levenshtein_similarity(
            &format!("{prefix}b"),
            &format!("{prefix}c"),
        ),
        1.0 - 1.0 / 20_001.0
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn analyzer_migration_requires_version_and_returns_exact_result_shape() {
    let (state, _receiver) = test_state();
    let unversioned =
        super::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
            .await
            .unwrap();
    assert_eq!(unversioned.status, "400 Bad Request");

    let versioned =
        super::route_http_request("POST", "/api/v0/audio/analyzers/migrate", None, "", &state)
            .await
            .unwrap();
    assert_eq!(versioned.status, "200 OK");
    assert_eq!(versioned.body, r#"{"updated":0}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn hashdb_optimize_profile_and_slow_queries_report_real_observed_data() {
    let (state, _receiver) = test_state();

    // Prime the store with a real hash entry so the profile/slow-query
    // endpoints have genuine data to report on, not an empty store.
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "route-audit-key".to_owned(),
                size: 123,
                file_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed hash entry");
    }

    let profile = super::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        r#"{"query":"SELECT * FROM hash_entries"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(profile.status, "200 OK");
    let profile = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap();
    assert_eq!(profile["query"], "SELECT * FROM hash_entries");
    assert_eq!(profile["rowsReturned"], 1);
    assert!(profile["executionTimeMs"].is_u64());
    assert!(
        profile["queryPlan"]
            .as_str()
            .unwrap()
            .contains("linear scan"),
        "{profile}"
    );

    let slow_queries = super::route_http_request(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(slow_queries.status, "200 OK");
    let slow_queries = serde_json::from_str::<serde_json::Value>(&slow_queries.body).unwrap();
    assert_eq!(slow_queries["totalQueries"], 1);
    let entries = slow_queries["slowQueries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["query"], "profile_query");
    assert_eq!(entries[0]["executionCount"], 1);
    assert_eq!(entries[0]["totalRowsReturned"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn hashdb_optimize_analyze_reports_real_observed_counts_and_thresholds() {
    let (state, _receiver) = test_state();

    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "analyze-audit-key".to_owned(),
                size: 456,
                file_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed hash entry");
        discovery
            .merge_shadow_records(vec![super::content_discovery::ShadowIndexRecord {
                recording_id: "mbid-analyze-audit".to_owned(),
                peer_ids: vec!["peer-a".to_owned(), "peer-b".to_owned()],
                updated_at: 0,
            }])
            .expect("seed shadow record");
    }

    let analyze =
        super::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
            .await
            .unwrap();
    assert_eq!(analyze.status, "200 OK");
    let analyze = serde_json::from_str::<serde_json::Value>(&analyze.body).unwrap();
    assert_eq!(analyze["hashDbEntryCount"], 1);
    assert_eq!(analyze["entries"], 1);
    assert_eq!(analyze["peerCount"], 2);
    assert!(analyze["databaseSizeBytes"].is_u64());
    // An in-memory store (no state file yet) has nothing on disk.
    assert_eq!(analyze["databaseSizeBytes"], 0);
    // Below both real thresholds, so no recommendations should fire.
    assert_eq!(analyze["recommendations"], serde_json::json!([]));
    assert_eq!(analyze["missingIndexes"], serde_json::json!([]));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn songid_capabilities_report_real_tool_presence_not_a_fake_string_array() {
    let capabilities = super::songid_capabilities_json(None);
    let capabilities = capabilities.as_array().expect("capabilities array");
    // Matches the oracle's full SongIdCapabilityReporter id set, not
    // the old three-string placeholder.
    assert_eq!(capabilities.len(), 18);

    let by_id = |id: &str| {
        capabilities
            .iter()
            .find(|capability| capability["id"] == id)
            .unwrap_or_else(|| panic!("missing capability {id}"))
    };

    // An absent integration configuration is reported as unavailable;
    // this is a configuration state, not a fabricated implementation
    // status.
    for id in ["chromaprint_fingerprint", "acoustid_lookup"] {
        let capability = by_id(id);
        assert_eq!(capability["available"], false, "{id}");
        assert!(
            capability["reason"].as_str().unwrap().contains("disabled")
                || capability["reason"].as_str().unwrap().contains("Requires"),
            "{id}: {}",
            capability["reason"]
        );
    }
    assert_eq!(by_id("musicbrainz_lookup")["available"], false);
    assert!(by_id("musicbrainz_lookup")["reason"]
        .as_str()
        .unwrap()
        .contains("base URL"));
    assert_eq!(by_id("text_query")["status"], "stable");
    assert_eq!(by_id("text_query")["available"], true);
    assert_eq!(by_id("url_parsing")["available"], true);
    assert_eq!(by_id("local_file_intake")["available"], true);
    assert_eq!(by_id("spotify_page_metadata")["available"], true);

    // Tool-gated capabilities must reflect real PATH state, not a
    // hardcoded value.
    let youtube_metadata = by_id("youtube_metadata");
    assert_eq!(
        youtube_metadata["available"],
        super::command_exists_on_path("yt-dlp")
    );
    assert_eq!(
        youtube_metadata["requirements"],
        serde_json::json!(["yt-dlp"])
    );

    let youtube_audio = by_id("youtube_audio");
    assert_eq!(
        youtube_audio["available"],
        super::command_exists_on_path("yt-dlp") && super::command_exists_on_path("ffmpeg")
    );

    let hash_flag = by_id("hash_from_audio_file_flag");
    assert_eq!(hash_flag["status"], "broken");
    assert_eq!(hash_flag["available"], false);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn songid_source_classification_and_local_root_guard_match_target() {
    assert_eq!(super::songid_source_type("Artist - Track"), "text_query");
    assert_eq!(
        super::songid_source_type("https://www.youtube.com/watch?v=track"),
        "youtube_url"
    );
    assert_eq!(
        super::songid_source_type("https://open.spotify.com/track/abc"),
        "spotify_url"
    );
    assert_eq!(
        super::songid_source_type("https://example.test/audio"),
        "url"
    );
    let spotify_metadata = super::songid_spotify_metadata_from_html(
        "https://open.spotify.com/track/abc123?si=fixture",
        r#"<html><head>
            <meta property="og:title" content="Track &amp; More">
            <meta property="og:description" content="Artist · Album">
            <meta property="og:audio" content="https://cdn.example/preview.mp3">
        </head></html>"#,
    );
    assert_eq!(spotify_metadata["query"], "Artist - Track & More");
    assert_eq!(spotify_metadata["metadata"]["artist"], "Artist");
    assert_eq!(spotify_metadata["metadata"]["album"], "Album");
    assert_eq!(spotify_metadata["metadata"]["spotifyTrackId"], "abc123");
    assert_eq!(
        spotify_metadata["metadata"]["previewUrl"],
        "https://cdn.example/preview.mp3"
    );

    let state_dir = std::env::temp_dir().join(format!("slskr-songid-{}", uuid::Uuid::new_v4()));
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_STATE_DIR", state_dir.to_str().unwrap()),
    )
    .expect("SongID test config");
    fs::create_dir_all(&config.downloads_dir).unwrap();
    let allowed = config.downloads_dir.join("allowed.flac");
    fs::write(&allowed, b"fixture").unwrap();
    assert!(super::songid_local_file_is_allowed(
        &config,
        allowed.to_str().unwrap()
    ));

    let outside_dir = state_dir.join("outside");
    fs::create_dir_all(&outside_dir).unwrap();
    let outside = outside_dir.join("outside.flac");
    fs::write(&outside, b"fixture").unwrap();
    assert!(!super::songid_local_file_is_allowed(
        &config,
        outside.to_str().unwrap()
    ));
    assert_eq!(
        super::songid_fallback_query(allowed.to_str().unwrap(), "local_file"),
        "allowed"
    );
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_chromaprint_path_uses_configured_ffmpeg_and_fpcalc() {
    if !super::configured_command_exists("ffmpeg") || !super::command_exists_on_path("fpcalc") {
        return;
    }
    let source = std::env::temp_dir().join(format!(
        "slskr-songid-chromaprint-{}.wav",
        uuid::Uuid::new_v4().simple()
    ));
    let generated = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=10",
            "-ar",
            "44100",
            "-ac",
            "2",
        ])
        .arg(&source)
        .status()
        .expect("ffmpeg test fixture");
    assert!(generated.success());
    let settings = super::ChromaprintIntegrationSettings {
        enabled: true,
        algorithm: 1,
        ffmpeg_path: "ffmpeg".to_owned(),
        sample_rate: 44_100,
        channels: 2,
        duration_seconds: 10,
    };
    let fingerprint = super::songid_extract_chromaprint(source.to_str().unwrap(), &settings)
        .await
        .expect("Chromaprint fingerprint");
    assert!(fingerprint.len() > 20);
    let _ = fs::remove_file(source);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_acoustid_lookup_matches_target_form_contract() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind AcoustID fixture");
    let address = listener.local_addr().expect("AcoustID fixture address");
    let server = tokio::spawn(async move {
        serve_json_fixture(
            &listener,
            serde_json::json!({
                "status": "ok",
                "results": [{
                    "id": "acoustid-audit",
                    "score": 0.97,
                    "recordings": [{
                        "id": "recording-audit",
                        "title": "Audit Track",
                        "artists": [{"name": "Audit Artist"}]
                    }]
                }]
            }),
        )
        .await
    });
    let settings = super::AcoustIdIntegrationSettings {
        enabled: true,
        client_id: Some("fixture-client".to_owned()),
        base_url: format!("http://{address}/v2"),
    };
    let result = super::songid_acoustid_lookup("fixture-fingerprint", &settings, 44_100, 120)
        .await
        .expect("AcoustID fixture lookup")
        .expect("AcoustID result");
    assert_eq!(result["id"], "acoustid-audit");
    let request = server.await.expect("AcoustID fixture task");
    assert!(request.starts_with("POST /v2/lookup HTTP/1.1"), "{request}");
    assert!(request.contains("client=fixture-client"), "{request}");
    assert!(
        request.contains("fingerprint=fixture-fingerprint"),
        "{request}"
    );
    assert!(request.contains("duration=120"), "{request}");
    assert!(request.contains("sample_rate=44100"), "{request}");
    assert!(request.contains("meta=recordings"), "{request}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_run_creation_requires_a_real_non_empty_source() {
    let (state, _receiver) = test_state();

    // Matches the oracle's SongIdController.CreateRun: an empty (or
    // missing) source is rejected before a run is ever queued, not
    // silently accepted as an empty-source run.
    for body in ["{}", r#"{"source":""}"#, r#"{"source":"   "}"#] {
        let response = super::route_http_request("POST", "/api/v0/songid/runs", None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{body}: {error}"));
        assert_eq!(response.status, "400 Bad Request", "{body}");
        assert!(
            response.body.contains("SongID source is required."),
            "{body}: {}",
            response.body
        );
    }

    let before = super::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
        .await
        .expect("runs before");
    let before_count = serde_json::from_str::<serde_json::Value>(&before.body)
        .unwrap()
        .as_array()
        .unwrap()
        .len();

    let valid = super::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("valid run");
    assert_eq!(valid.status, "202 Accepted", "{}", valid.body);

    let after = super::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
        .await
        .expect("runs after");
    let after_count = serde_json::from_str::<serde_json::Value>(&after.body)
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    assert_eq!(after_count, before_count + 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_run_reports_its_real_completed_status_not_stuck_at_queued() {
    let (state, _receiver) = test_state();

    // slskR analyzes synchronously (unlike the oracle's real async
    // queue+worker pipeline), so by the time the create response is
    // built the run has already really finished -- it must report
    // that real status immediately, not a fake "queued" placeholder
    // that nothing would ever advance past.
    let created = super::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("create run");
    assert_eq!(created.status, "202 Accepted", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created_json["status"], "completed", "{created_json}");
    assert_eq!(created_json["currentStage"], "completed", "{created_json}");
    assert_eq!(created_json["percentComplete"], 1.0, "{created_json}");
    let run_id = created_json["id"].as_str().unwrap().to_owned();

    // Polling the run afterward must reflect the same real, final
    // status -- not revert to a stale "queued" snapshot.
    let polled = super::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("poll run");
    let polled_json = serde_json::from_str::<serde_json::Value>(&polled.body).unwrap();
    assert_eq!(polled_json["status"], "completed", "{polled_json}");

    // The queue summary must count this as a real completion, never
    // as perpetually queued/running.
    let queue = super::route_http_request("GET", "/api/v0/songid/runs/queue", None, "", &state)
        .await
        .expect("queue summary");
    let queue_json = serde_json::from_str::<serde_json::Value>(&queue.body).unwrap();
    assert!(
        queue_json["completedCount"].as_u64().unwrap() >= 1,
        "{queue_json}"
    );
    assert_eq!(queue_json["queuedCount"], 0, "{queue_json}");
    assert_eq!(queue_json["runningCount"], 0, "{queue_json}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_run_evidence_package_reshapes_real_stored_run_fields() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"route-audit","query":"Evidence Package Audit"}"#,
        &state,
    )
    .await
    .expect("create run");
    assert_eq!(created.status, "202 Accepted", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let run_id = created_json["id"].as_str().unwrap().to_owned();

    // Matches the oracle's real SongIdRunEvidencePackage contract,
    // reshaped from the same stored run fields -- not a missing
    // endpoint or an invented shape.
    let package = super::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}/evidence-package"),
        None,
        "",
        &state,
    )
    .await
    .expect("evidence package");
    assert_eq!(package.status, "200 OK", "{}", package.body);
    let package_json = serde_json::from_str::<serde_json::Value>(&package.body).unwrap();
    assert_eq!(package_json["runId"], run_id, "{package_json}");
    assert_eq!(package_json["status"], "completed", "{package_json}");
    assert_eq!(
        package_json["query"], "Evidence Package Audit",
        "{package_json}"
    );
    // slskR's synchronous analysis completes instantly, so
    // completedAt is real (equal to createdAt), not fabricated.
    assert_eq!(
        package_json["completedAt"], package_json["createdAt"],
        "{package_json}"
    );
    assert_eq!(package_json["trackCandidates"], serde_json::json!([]));
    assert_eq!(package_json["artifacts"], serde_json::json!([]));
    // Honest warnings reflecting the real (empty) analysis state,
    // not fabricated ones.
    let warnings = package_json["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "No track candidates were produced."),
        "{package_json}"
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "No recognizer hits were recorded."),
        "{package_json}"
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "No forensic matrix was generated."),
        "{package_json}"
    );

    let missing = super::route_http_request(
        "GET",
        "/api/v0/songid/runs/songid-does-not-exist/evidence-package",
        None,
        "",
        &state,
    )
    .await
    .expect("missing run evidence package");
    assert_eq!(missing.status, "404 Not Found");

    // The sibling run-detail and forensic-matrix routes must still
    // work unaffected by the new routing guard.
    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("run detail");
    assert_eq!(detail.status, "200 OK");
    let matrix = super::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}/forensic-matrix"),
        None,
        "",
        &state,
    )
    .await
    .expect("forensic matrix");
    assert_eq!(matrix.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_runs_respect_a_real_limit_and_newest_first_order() {
    let (state, _receiver) = test_state();
    for index in 0..3 {
        let response = super::route_http_request(
            "POST",
            "/api/v0/songid/runs",
            None,
            &format!(r#"{{"source":"route-audit-{index}"}}"#),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("create run {index}: {error}"));
        assert_eq!(response.status, "202 Accepted");
    }

    // Matches the oracle's real ListRuns: newest-first, bounded by
    // the real `limit` query param -- not the full, unbounded,
    // creation-order history.
    let limited = super::route_http_request("GET", "/api/v0/songid/runs?limit=2", None, "", &state)
        .await
        .expect("limited runs");
    let limited_json = serde_json::from_str::<serde_json::Value>(&limited.body).unwrap();
    let limited_runs = limited_json.as_array().unwrap();
    assert_eq!(limited_runs.len(), 2, "{limited_json}");
    assert_eq!(
        limited_runs[0]["source"], "route-audit-2",
        "newest run must come first: {limited_json}"
    );
    assert_eq!(limited_runs[1]["source"], "route-audit-1", "{limited_json}");

    // A non-positive limit falls back to the oracle's default of 10,
    // not an empty or unbounded result.
    let zero_limit =
        super::route_http_request("GET", "/api/v0/songid/runs?limit=0", None, "", &state)
            .await
            .expect("zero limit runs");
    let zero_limit_json = serde_json::from_str::<serde_json::Value>(&zero_limit.body).unwrap();
    assert_eq!(
        zero_limit_json.as_array().unwrap().len(),
        3,
        "{zero_limit_json}"
    );
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
async fn controller_api_differential_mesh_stats_reflect_real_merge_activity_not_hardcoded_zeros() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let baseline = super::route_http_request("GET", "/api/v0/mesh/stats", None, "", &state)
        .await
        .expect("baseline mesh stats");
    assert_eq!(baseline.status, "200 OK", "{}", baseline.body);
    assert_eq!(baseline.content_type, "application/json");
    let baseline_json = serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap();
    assert_eq!(baseline_json["totalSyncs"], 0, "{baseline_json}");
    assert_eq!(baseline_json["totalEntriesSent"], 0, "{baseline_json}");
    assert_eq!(baseline_json["currentSeqId"], 0, "{baseline_json}");

    let valid_entry = serde_json::json!({
        "flacKey": "mesh-stats-audit-key",
        "size": 1234,
        "byteHash": "a".repeat(64),
    });
    let merged = super::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-peer",
        None,
        &serde_json::json!({"entries": [valid_entry]}).to_string(),
        &state,
    )
    .await
    .expect("merge real entry");
    assert_eq!(merged.status, "200 OK", "{}", merged.body);
    assert_eq!(merged.content_type, "application/json");
    let merged_json = serde_json::from_str::<serde_json::Value>(&merged.body).unwrap();
    assert_eq!(merged_json["merged"], 1, "{merged_json}");
    let real_seq_id = merged_json["latestSeqId"].as_u64().unwrap();
    assert!(real_seq_id > 0, "{merged_json}");

    let delta = super::route_http_request(
        "GET",
        "/api/v0/mesh/delta?sinceSeq=0&maxEntries=1",
        None,
        "",
        &state,
    )
    .await
    .expect("generate a real mesh delta");
    assert_eq!(delta.status, "200 OK", "{}", delta.body);
    assert_eq!(delta.content_type, "application/json");
    let delta_json = serde_json::from_str::<serde_json::Value>(&delta.body).unwrap();
    assert_eq!(delta_json["type"], 3, "{delta_json}");
    assert_eq!(
        delta_json["entries"].as_array().unwrap().len(),
        1,
        "{delta_json}"
    );
    assert_eq!(
        delta_json["entries"][0]["seq_id"], real_seq_id,
        "{delta_json}"
    );
    assert!(
        delta_json["entries"][0].get("flacKey").is_none(),
        "{delta_json}"
    );
    assert_eq!(delta_json["has_more"], false, "{delta_json}");
    assert_eq!(delta_json["latest_seq_id"], real_seq_id, "{delta_json}");

    // A structurally usable but malformed hash is retained through the
    // controller's request normalization and then skipped individually,
    // matching MeshSyncService.MergeEntriesAsync.
    let invalid_entry = serde_json::json!({
        "flacKey": "mesh-stats-audit-invalid",
        "byteHash": "not-a-sha256",
        "size": 1234
    });
    let failed = super::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-peer",
        None,
        &serde_json::json!({"entries": [invalid_entry]}).to_string(),
        &state,
    )
    .await
    .expect("merge invalid entry");
    assert_eq!(failed.status, "200 OK", "{}", failed.body);
    assert_eq!(failed.content_type, "application/json");
    let failed_json = serde_json::from_str::<serde_json::Value>(&failed.body).unwrap();
    assert_eq!(failed_json["merged"], 0, "{failed_json}");
    assert_eq!(failed_json["skipped"], 1, "{failed_json}");

    let stats = super::route_http_request("GET", "/api/v0/mesh/stats", None, "", &state)
        .await
        .expect("mesh stats after activity");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalSyncs"], 2, "{stats_json}");
    assert_eq!(stats_json["successfulSyncs"], 2, "{stats_json}");
    assert_eq!(stats_json["failedSyncs"], 0, "{stats_json}");
    assert_eq!(stats_json["totalEntriesReceived"], 2, "{stats_json}");
    assert_eq!(stats_json["totalEntriesSent"], 1, "{stats_json}");
    assert_eq!(stats_json["skippedEntries"], 1, "{stats_json}");
    assert_eq!(stats_json["totalEntriesMerged"], 1, "{stats_json}");
    assert_eq!(stats_json["currentSeqId"], real_seq_id, "{stats_json}");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("mesh_stats_and_merge.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh/stats",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/merge",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/merge",
                "case": "mutation-side-effects-and-readback",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh/delta",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_hello_matches_frozen_message_dto() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKD_SLSK_USERNAME", "mesh-user"));

    let hello = super::route_http_request("GET", "/api/v0/mesh/hello", None, "", &state)
        .await
        .expect("mesh hello");
    assert_eq!(hello.status, "200 OK", "{}", hello.body);
    let json = serde_json::from_str::<serde_json::Value>(&hello.body).unwrap();
    assert_eq!(json["type"], 1, "{json}");
    assert_eq!(json["proto_version"], 1, "{json}");
    assert_eq!(json["client_id"], "tester", "{json}");
    assert_eq!(json["client_version"], super::APP_VERSION, "{json}");
    assert_eq!(json["latest_seq_id"], 0, "{json}");
    assert_eq!(json["hash_count"], 0, "{json}");
    assert_eq!(json["public_key"], "", "{json}");
    assert_eq!(json["signature"], "", "{json}");
    assert_eq!(json["timestamp_ms"], 0, "{json}");
    assert!(json.get("peerId").is_none(), "{json}");
    assert!(json.get("capabilities").is_none(), "{json}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_mesh_message_matches_typed_controller_validation() {
    let (state, _receiver) = test_state();

    let missing_peer = super::route_http_request(
        "POST",
        "/api/v0/mesh/message",
        None,
        r#"{"type":1}"#,
        &state,
    )
    .await
    .expect("missing fromUser response");
    assert_eq!(missing_peer.status, "400 Bad Request");
    assert!(missing_peer.body.contains("fromUser"));

    let missing_type = super::route_http_request(
        "POST",
        "/api/v0/mesh/message?fromUser=mesh-peer",
        None,
        "{}",
        &state,
    )
    .await
    .expect("missing type response");
    assert_eq!(missing_type.status, "400 Bad Request");
    assert!(missing_type.body.contains("type"));

    let unsupported = super::route_http_request(
        "POST",
        "/api/v0/mesh/message?fromUser=mesh-peer",
        None,
        r#"{"type":6}"#,
        &state,
    )
    .await
    .expect("unsupported type response");
    assert_eq!(unsupported.status, "400 Bad Request");

    let unsigned = super::route_http_request(
        "POST",
        "/api/v0/mesh/message?fromUser=mesh-peer",
        None,
        r#"{"type":1}"#,
        &state,
    )
    .await
    .expect("unsigned envelope response");
    assert_eq!(unsigned.status, "200 OK", "{}", unsigned.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap(),
        serde_json::json!({"handled": true, "response": null})
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
async fn controller_api_differential_mesh_message_runtime() {
    use slskr_client::mesh_sync::{
        MeshHashEntry, MeshHelloMessage, MeshMessageType, MeshPushDeltaMessage,
        MeshReqChunkMessage, MeshReqDeltaMessage, MeshReqKeyMessage, MeshSyncBase, MeshSyncMessage,
    };

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-mesh-message-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create mesh controller fixture directory");
    let local_path = root.join("fixture.flac");
    let local_bytes = b"mesh-controller-message-fixture";
    fs::write(&local_path, local_bytes).expect("write mesh controller fixture");
    let filename = "Virtual/mesh-controller.flac";
    add_test_share(&state, filename, &local_path, local_bytes.len() as u64).await;
    let local_key = super::content_discovery::generate_flac_key(filename, local_bytes.len() as u64);
    state
        .content_discovery
        .write()
        .await
        .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
            flac_key: local_key.clone(),
            byte_hash: "a".repeat(64),
            size: local_bytes.len() as u64,
            ..super::content_discovery::HashDbEntry::default()
        }])
        .expect("seed mesh controller hash database");

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[61; 32]);
    let sign_request = |mut message: MeshSyncMessage| -> String {
        message
            .sign_at(&remote_key, super::unix_timestamp_millis() as i64)
            .expect("sign mesh controller request");
        String::from_utf8(
            message
                .encode_json()
                .expect("encode mesh controller request"),
        )
        .expect("mesh controller request is UTF-8")
    };
    let decode_response = |response: &super::HttpResponse| {
        assert_eq!(response.status, "200 OK", "{}", response.body);
        assert_eq!(response.content_type, "application/json");
        let outer = serde_json::from_str::<serde_json::Value>(&response.body)
            .expect("decode mesh controller response envelope");
        assert_eq!(outer["handled"], true, "{outer}");
        let encoded = serde_json::to_vec(&outer["response"])
            .expect("encode mesh controller response payload");
        let message =
            MeshSyncMessage::decode_json(&encoded).expect("decode typed mesh controller response");
        message
            .verify_signature()
            .expect("verify typed mesh controller response");
        (outer, message)
    };
    let route = "/api/v0/mesh/message?fromUser=mesh-peer";

    let hello_body = sign_request(MeshSyncMessage::Hello(MeshHelloMessage {
        message_type: MeshMessageType::Hello,
        base: MeshSyncBase::default(),
        client_id: "mesh-peer".to_owned(),
        client_version: "fixture".to_owned(),
        latest_sequence_id: 0,
        hash_count: 0,
    }));
    let hello_response = super::route_http_request("POST", route, None, &hello_body, &state)
        .await
        .expect("mesh controller hello");
    let (hello_outer, hello) = decode_response(&hello_response);
    assert_eq!(hello_outer["response"]["type"], 1, "{hello_outer}");
    assert!(matches!(hello, MeshSyncMessage::Hello(_)));

    let delta_body = sign_request(MeshSyncMessage::ReqDelta(MeshReqDeltaMessage {
        message_type: MeshMessageType::ReqDelta,
        base: MeshSyncBase::default(),
        since_sequence_id: 0,
        max_entries: 10,
    }));
    let delta_response = super::route_http_request("POST", route, None, &delta_body, &state)
        .await
        .expect("mesh controller delta");
    let (delta_outer, delta) = decode_response(&delta_response);
    assert_eq!(delta_outer["response"]["type"], 3, "{delta_outer}");
    match delta {
        MeshSyncMessage::PushDelta(message) => assert_eq!(message.entries.len(), 1),
        other => panic!("expected PushDelta response, got {other:?}"),
    }

    let key_body = sign_request(MeshSyncMessage::ReqKey(MeshReqKeyMessage {
        message_type: MeshMessageType::ReqKey,
        base: MeshSyncBase::default(),
        flac_key: local_key.clone(),
    }));
    let key_response = super::route_http_request("POST", route, None, &key_body, &state)
        .await
        .expect("mesh controller key lookup");
    let (key_outer, key) = decode_response(&key_response);
    assert_eq!(key_outer["response"]["type"], 5, "{key_outer}");
    match key {
        MeshSyncMessage::RespKey(message) => assert!(message.found),
        other => panic!("expected RespKey response, got {other:?}"),
    }

    let chunk_body = sign_request(MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key: local_key.clone(),
        offset: 5,
        length: 7,
    }));
    let chunk_response = super::route_http_request("POST", route, None, &chunk_body, &state)
        .await
        .expect("mesh controller chunk request");
    let (chunk_outer, chunk) = decode_response(&chunk_response);
    assert_eq!(chunk_outer["response"]["type"], 8, "{chunk_outer}");
    match chunk {
        MeshSyncMessage::RespChunk(message) => {
            assert!(message.success);
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(message.data_base64)
                    .expect("decode mesh chunk"),
                &local_bytes[5..12]
            );
        }
        other => panic!("expected RespChunk response, got {other:?}"),
    }

    let pushed_key = "1122334455667788".to_owned();
    let push_body = sign_request(MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
        message_type: MeshMessageType::PushDelta,
        base: MeshSyncBase::default(),
        entries: vec![MeshHashEntry {
            sequence_id: 0,
            flac_key: pushed_key.clone(),
            byte_hash: "b".repeat(64),
            size: 17,
            metadata_flags: None,
            signer_public_key: None,
            signature: None,
        }],
        latest_sequence_id: 0,
        has_more: false,
    }));
    let push_response = super::route_http_request("POST", route, None, &push_body, &state)
        .await
        .expect("mesh controller delta merge");
    let (push_outer, push) = decode_response(&push_response);
    assert_eq!(push_outer["response"]["type"], 6, "{push_outer}");
    match push {
        MeshSyncMessage::Ack(message) => assert_eq!(message.merged_count, 1),
        other => panic!("expected Ack response, got {other:?}"),
    }

    let pushed_key_body = sign_request(MeshSyncMessage::ReqKey(MeshReqKeyMessage {
        message_type: MeshMessageType::ReqKey,
        base: MeshSyncBase::default(),
        flac_key: pushed_key,
    }));
    let pushed_key_response =
        super::route_http_request("POST", route, None, &pushed_key_body, &state)
            .await
            .expect("mesh controller merged key readback");
    let (_, pushed_key) = decode_response(&pushed_key_response);
    assert!(matches!(
        pushed_key,
        MeshSyncMessage::RespKey(message) if message.found
    ));

    let unsupported = super::route_http_request("POST", route, None, r#"{"type":6}"#, &state)
        .await
        .expect("mesh controller unsupported type");
    assert_eq!(unsupported.status, "400 Bad Request");
    let unsigned = super::route_http_request("POST", route, None, r#"{"type":1}"#, &state)
        .await
        .expect("mesh controller unsigned message");
    assert_eq!(unsigned.status, "200 OK", "{}", unsigned.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap(),
        serde_json::json!({"handled": true, "response": null})
    );

    let missing_from_user = super::route_http_request(
        "POST",
        "/api/v0/mesh/message",
        None,
        r#"{"type":1}"#,
        &state,
    )
    .await
    .expect("mesh controller missing fromUser");
    let missing_type = super::route_http_request("POST", route, None, "{}", &state)
        .await
        .expect("mesh controller missing type");
    let missing_case = missing_from_user.status == "400 Bad Request"
        && missing_from_user.content_type == "application/json"
        && missing_from_user.body.contains("fromUser")
        && missing_type.status == "400 Bad Request"
        && missing_type.content_type == "application/json"
        && missing_type.body.contains("type");

    let missing_chunk_body = sign_request(MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key: "deadbeefdeadbeef".to_owned(),
        offset: 0,
        length: 4,
    }));
    let missing_chunk_response =
        super::route_http_request("POST", route, None, &missing_chunk_body, &state)
            .await
            .expect("mesh controller missing chunk response");
    let (missing_chunk_outer, missing_chunk) = decode_response(&missing_chunk_response);
    let runtime_failure_case = missing_chunk_outer["response"]["type"] == 8
        && matches!(
            missing_chunk,
            MeshSyncMessage::RespChunk(message) if !message.success
        );

    let concurrent_bodies = (0..6)
        .map(|index| {
            sign_request(MeshSyncMessage::Hello(MeshHelloMessage {
                message_type: MeshMessageType::Hello,
                base: MeshSyncBase::default(),
                client_id: format!("mesh-concurrent-message-{index}"),
                client_version: "fixture".to_owned(),
                latest_sequence_id: 0,
                hash_count: 0,
            }))
        })
        .collect::<Vec<_>>();
    let concurrent_responses =
        futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
            let state = Arc::clone(&state);
            async move { super::route_http_request("POST", route, None, body, &state).await }
        }))
        .await;
    let concurrency_case = concurrent_responses.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            if response.status != "200 OK" || response.content_type != "application/json" {
                return false;
            }
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            value["handled"] == true && value["response"]["type"] == 1
        })
    });

    let state_dir = state.config.state_dir.clone();
    let persistent_store = super::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("create path-backed mesh message store");
    *state.content_discovery.write().await = persistent_store;
    let restart_key = "cafebabecafebabe".to_owned();
    let restart_body = sign_request(MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
        message_type: MeshMessageType::PushDelta,
        base: MeshSyncBase::default(),
        entries: vec![MeshHashEntry {
            sequence_id: 0,
            flac_key: restart_key.clone(),
            byte_hash: "c".repeat(64),
            size: 8001,
            metadata_flags: None,
            signer_public_key: None,
            signature: None,
        }],
        latest_sequence_id: 0,
        has_more: false,
    }));
    let restart_response = super::route_http_request("POST", route, None, &restart_body, &state)
        .await
        .expect("persist mesh message delta");
    let (restart_outer, restart_message) = decode_response(&restart_response);
    let reloaded = super::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("reload path-backed mesh message store");
    let restart_case = restart_outer["response"]["type"] == 6
        && matches!(
            restart_message,
            MeshSyncMessage::Ack(message) if message.merged_count == 1
        )
        && reloaded.lookup_hash(&restart_key).is_some();

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh controller evidence directory");
    fs::write(
        evidence_dir.join("mesh_message_runtime.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "mutation-side-effects-and-readback",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "missing-empty-or-conflict-state",
                "pass": missing_case,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "runtime-failure-and-timeout",
                "pass": runtime_failure_case,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "restart-persistence-or-reset",
                "pass": restart_case,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "concurrency-and-idempotency",
                "pass": concurrency_case,
            }),
        ])
        .expect("serialize mesh controller evidence"),
    )
    .expect("write mesh controller evidence");
    assert!(
        missing_case && runtime_failure_case && restart_case && concurrency_case,
        "mesh message differential cases failed: missing={missing_case}, runtime={runtime_failure_case}, restart={restart_case}, concurrency={concurrency_case}"
    );
    fs::remove_dir_all(state_dir).expect("remove mesh message test state directory");
    fs::remove_dir_all(root).expect("remove mesh controller fixture directory");
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
async fn controller_api_differential_mesh_controller_edge_cases() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SHARE_FIXTURE", ""),
    );

    // MeshController's [ApiController] binding rejects malformed long/int
    // query values instead of silently substituting the defaults.
    let malformed_delta = super::route_http_request(
        "GET",
        "/api/v0/mesh/delta?sinceSeq=not-an-integer",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed mesh delta response");
    let malformed_max_entries = super::route_http_request(
        "GET",
        "/api/v0/mesh/delta?maxEntries=not-an-integer",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed mesh delta maxEntries response");
    record!(
        "GET",
        "/api/v0/mesh/delta",
        "malformed-path-query-or-body",
        malformed_delta.status == "400 Bad Request"
            && malformed_max_entries.status == "400 Bad Request"
    );

    for (request_path, ledger_route) in [
        ("/api/v0/mesh/health/extra", "/api/v0/mesh/health"),
        ("/api/v0/mesh/hello/extra", "/api/v0/mesh/hello"),
        ("/api/v0/mesh/peers/extra", "/api/v0/mesh/peers"),
        ("/api/v0/mesh/stats/extra", "/api/v0/mesh/stats"),
        ("/api/v0/mesh/transport/extra", "/api/v0/mesh/transport"),
    ] {
        let response = super::route_http_request("GET", request_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("malformed {request_path}: {error}"));
        record!(
            "GET",
            ledger_route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let malformed_nat_detect =
        super::route_http_request("POST", "/api/v0/mesh/nat/detect/extra", None, "", &state)
            .await
            .expect("malformed versioned mesh NAT detection path response");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "malformed-path-query-or-body",
        malformed_nat_detect.status == "404 Not Found"
    );

    // The frozen controller always returns a two-field JSON result for
    // a completed best-effort STUN probe. A no-response environment is
    // a normal unknown result, not an HTTP failure or an exception
    // shape; successful probes use one of the same four lower-case NAT
    // labels and set detected accordingly.
    let nat_detect = super::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &state)
        .await
        .expect("versioned mesh NAT detection response");
    let nat_detect_json =
        serde_json::from_str::<serde_json::Value>(&nat_detect.body).unwrap_or_default();
    let nat_detect_keys = nat_detect_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let nat_type = nat_detect_json["type"].as_str().unwrap_or_default();
    let nat_detected = nat_detect_json["detected"].as_bool();
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "nominal-status-headers-body",
        nat_detect.status == "200 OK"
            && nat_detect.content_type.starts_with("application/json")
            && nat_detect_keys == BTreeSet::from(["detected".to_owned(), "type".to_owned()])
            && matches!(nat_type, "unknown" | "direct" | "symmetric" | "restricted")
            && nat_detected.is_some_and(|detected| detected == (nat_type != "unknown"))
    );

    let missing_lookup = super::route_http_request(
        "GET",
        "/api/v0/mesh/lookup/edge-case-missing",
        None,
        "",
        &state,
    )
    .await
    .expect("missing mesh lookup response");
    let missing_lookup_json =
        serde_json::from_str::<serde_json::Value>(&missing_lookup.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "missing-empty-or-conflict-state",
        missing_lookup.status == "404 Not Found" && missing_lookup_json["found"] == false
    );

    let malformed_lookup =
        super::route_http_request("GET", "/api/v0/mesh/lookup/", None, "", &state)
            .await
            .expect("malformed mesh lookup path response");
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "malformed-path-query-or-body",
        malformed_lookup.status == "404 Not Found"
    );

    let empty_health = super::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("empty mesh health response");
    let empty_health_json =
        serde_json::from_str::<serde_json::Value>(&empty_health.body).unwrap_or_default();
    let empty_health_keys = empty_health_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/health",
        "missing-empty-or-conflict-state",
        empty_health.status == "200 OK"
            && empty_health.content_type == "application/json"
            && empty_health_keys
                == BTreeSet::from([
                    "contentPeerHints".to_owned(),
                    "generatedAt".to_owned(),
                    "routingNodes".to_owned(),
                    "storedKeys".to_owned(),
                ])
            && empty_health_json["routingNodes"] == 0
            && empty_health_json["storedKeys"] == 0
            && empty_health_json["contentPeerHints"] == 0
            && empty_health_json["generatedAt"].is_string()
    );

    let published_key = "edge-case-published";
    let published = super::route_http_request(
        "POST",
        "/api/v0/mesh/publish",
        None,
        &serde_json::json!({
            "flacKey": published_key,
            "byteHash": "b".repeat(64),
            "size": 8192,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish versioned mesh hash");
    let published_json =
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap_or_default();
    let publish_pass = published.status == "200 OK" && published_json["published"] == true;
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "nominal-status-headers-body",
        publish_pass
    );
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "mutation-side-effects-and-readback",
        publish_pass
    );

    let lookup = super::route_http_request(
        "GET",
        &format!("/api/v0/mesh/lookup/{published_key}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read back versioned mesh hash");
    let lookup_json =
        serde_json::from_str::<serde_json::Value>(&lookup.body).unwrap_or(serde_json::Value::Null);
    let lookup_pass = lookup.status == "200 OK"
        && lookup_json["found"] == true
        && lookup_json["entry"]["flacKey"] == published_key
        && lookup_json["entry"]["size"] == 8192;
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "nominal-status-headers-body",
        lookup_pass
    );
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "populated-dynamic-state",
        lookup_pass
    );

    let health = super::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("populated versioned mesh health response");
    let health_json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/health",
        "populated-dynamic-state",
        health.status == "200 OK"
            && health_json["storedKeys"]
                .as_u64()
                .is_some_and(|count| count >= 1)
    );

    state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            "edge-case-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let peers = super::route_http_request("GET", "/api/v0/mesh/peers", None, "", &state)
        .await
        .expect("populated versioned mesh peers response");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/peers",
        "populated-dynamic-state",
        peers.status == "200 OK"
            && peers_json["peers"]
                .as_array()
                .is_some_and(|rows| { rows.iter().any(|row| row["username"] == "edge-case-peer") })
    );

    let malformed_publish =
        super::route_http_request("POST", "/api/v0/mesh/publish", None, "not-json", &state)
            .await
            .expect("malformed versioned mesh publish response");
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "malformed-path-query-or-body",
        malformed_publish.status == "400 Bad Request"
    );

    let missing_publish =
        super::route_http_request("POST", "/api/v0/mesh/publish", None, "{}", &state)
            .await
            .expect("missing versioned mesh publish fields response");
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "missing-empty-or-conflict-state",
        missing_publish.status == "400 Bad Request" && missing_publish.body.contains("flacKey")
    );

    let valid_merge = serde_json::json!({
        "entries": [{
            "flacKey": "edge-case-merge",
            "byteHash": "c".repeat(64),
            "size": 4097,
        }]
    })
    .to_string();
    let malformed_merge = super::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=edge-case-peer",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("malformed versioned mesh merge response");
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "malformed-path-query-or-body",
        malformed_merge.status == "400 Bad Request"
    );

    let missing_merge =
        super::route_http_request("POST", "/api/v0/mesh/merge", None, &valid_merge, &state)
            .await
            .expect("missing versioned mesh merge peer response");
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "missing-empty-or-conflict-state",
        missing_merge.status == "400 Bad Request" && missing_merge.body.contains("fromUser")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh edge-case evidence directory");
    fs::write(
        evidence_dir.join("mesh_controller_edge_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh edge-case ledger"),
    )
    .expect("write mesh edge-case ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh edge-case mismatches:\n{}",
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
async fn controller_api_differential_mesh_runtime_and_nat_lifecycle() {
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

    let runtime_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        runtime_env.clone(),
        super::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;

    for (path, route) in [
        ("/api/v0/mesh/delta", "/api/v0/mesh/delta"),
        ("/api/v0/mesh/health", "/api/v0/mesh/health"),
        ("/api/v0/mesh/hello", "/api/v0/mesh/hello"),
        (
            "/api/v0/mesh/lookup/mesh-runtime-missing",
            "/api/v0/mesh/lookup/{flacKey}",
        ),
        ("/api/v0/mesh/peers", "/api/v0/mesh/peers"),
        ("/api/v0/mesh/stats", "/api/v0/mesh/stats"),
        ("/api/v0/mesh/transport", "/api/v0/mesh/transport"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("runtime mesh GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                || (route == "/api/v0/mesh/lookup/{flacKey}"
                    && response.status == "404 Not Found"
                    && response.body.contains("\"found\":false"))
        );
    }

    let gateway_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh gateway runtime database");
    let gateway_env = runtime_env
        .clone()
        .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
        .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods");
    let (gateway_state, _gateway_receiver) = test_state_with_env_parts(
        gateway_env,
        super::SearchStore::new(),
        Some(gateway_db.clone()),
    );
    gateway_db.close_for_test().await;
    let runtime_services =
        super::route_http_request("GET", "/mesh/http/services", None, "", &gateway_state)
            .await
            .expect("runtime mesh gateway services");
    let runtime_services_json =
        serde_json::from_str::<serde_json::Value>(&runtime_services.body).unwrap_or_default();
    record!(
        "GET",
        "/mesh/http/services",
        "runtime-failure-and-timeout",
        runtime_services.status == "200 OK" && runtime_services_json["gateway"]["enabled"] == true
    );

    let merge_body = serde_json::json!({
        "entries": [{
            "flacKey": "mesh-runtime-merge",
            "byteHash": "1".repeat(64),
            "size": 4096,
        }]
    })
    .to_string();
    let runtime_merge = super::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-runtime-peer",
        None,
        &merge_body,
        &runtime_state,
    )
    .await
    .expect("runtime mesh merge");
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "runtime-failure-and-timeout",
        runtime_merge.status == "200 OK"
    );

    let runtime_publish = super::route_http_request(
        "POST",
        "/api/v0/mesh/publish",
        None,
        &serde_json::json!({
            "flacKey": "mesh-runtime-publish",
            "byteHash": "2".repeat(64),
            "size": 4097,
        })
        .to_string(),
        &runtime_state,
    )
    .await
    .expect("runtime mesh publish");
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "runtime-failure-and-timeout",
        runtime_publish.status == "200 OK"
    );

    let runtime_nat =
        super::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &runtime_state)
            .await
            .expect("runtime mesh NAT detection");
    let nat_shape = |response: &super::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        response.status == "200 OK"
            && value["type"].as_str().is_some_and(|value| {
                matches!(value, "unknown" | "direct" | "symmetric" | "restricted")
            })
            && value["detected"].is_boolean()
    };
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "runtime-failure-and-timeout",
        nat_shape(&runtime_nat)
    );

    let (nat_state, _nat_receiver) = test_state_with_env(runtime_env.clone());
    let nat_mutation =
        super::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &nat_state)
            .await
            .expect("mesh NAT mutation");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "mutation-side-effects-and-readback",
        nat_shape(&nat_mutation)
    );

    let (nat_restart_state, _nat_restart_receiver) = test_state_with_env(runtime_env.clone());
    let nat_restart = super::route_http_request(
        "POST",
        "/api/v0/mesh/nat/detect",
        None,
        "",
        &nat_restart_state,
    )
    .await
    .expect("mesh NAT restart");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "restart-persistence-or-reset",
        nat_shape(&nat_restart)
    );

    let nat_concurrent = futures_util::future::join_all([
        super::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &nat_state),
        super::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &nat_state),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "concurrency-and-idempotency",
        nat_concurrent
            .iter()
            .all(|response| response.as_ref().is_ok_and(nat_shape))
    );

    let sync_shape = |response: &super::HttpResponse| {
        response.status == "400 Bad Request"
            && response.body == r#"{"error":"Failed to sync with peer"}"#
    };
    let sync_mutation = super::route_http_request(
        "POST",
        "/api/v0/mesh/sync/mesh-runtime-peer",
        None,
        "{}",
        &runtime_state,
    )
    .await
    .expect("runtime mesh sync mutation");
    record!(
        "POST",
        "/api/v0/mesh/sync/{username}",
        "mutation-side-effects-and-readback",
        sync_shape(&sync_mutation)
    );
    let (sync_restart_state, _sync_restart_receiver) = test_state_with_env(runtime_env);
    let sync_restart = super::route_http_request(
        "POST",
        "/api/v0/mesh/sync/mesh-runtime-peer",
        None,
        "{}",
        &sync_restart_state,
    )
    .await
    .expect("runtime mesh sync restart");
    record!(
        "POST",
        "/api/v0/mesh/sync/{username}",
        "restart-persistence-or-reset",
        sync_shape(&sync_restart)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh runtime evidence directory");
    fs::write(
        evidence_dir.join("mesh_runtime_and_nat_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh runtime evidence"),
    )
    .expect("write mesh runtime evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh runtime mismatches:\n{}",
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
async fn controller_api_differential_mesh_merge_publish_restart_and_concurrency() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
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
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    let idempotent_body = serde_json::json!({
        "entries": [{
            "flacKey": "mesh-idempotent-entry",
            "byteHash": "d".repeat(64),
            "size": 4096,
        }]
    })
    .to_string();
    let first_merge = super::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-idempotency-peer",
        None,
        &idempotent_body,
        &state,
    )
    .await
    .expect("first idempotent mesh merge");
    let second_merge = super::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-idempotency-peer",
        None,
        &idempotent_body,
        &state,
    )
    .await
    .expect("second idempotent mesh merge");
    let first_merge_json =
        serde_json::from_str::<serde_json::Value>(&first_merge.body).unwrap_or_default();
    let second_merge_json =
        serde_json::from_str::<serde_json::Value>(&second_merge.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "concurrency-and-idempotency",
        first_merge.status == "200 OK"
            && second_merge.status == "200 OK"
            && first_merge_json["merged"] == 1
            && second_merge_json["merged"] == 0
    );

    let merge_bodies = (0..6)
        .map(|index| {
            serde_json::json!({
                "entries": [{
                    "flacKey": format!("mesh-concurrent-entry-{index}"),
                    "byteHash": format!("{:064x}", index + 1),
                    "size": 5000 + index,
                }]
            })
            .to_string()
        })
        .collect::<Vec<_>>();
    let merge_responses = futures_util::future::join_all(merge_bodies.iter().map(|body| {
        super::route_http_request(
            "POST",
            "/api/v0/mesh/merge?fromUser=mesh-concurrency-peer",
            None,
            body,
            &state,
        )
    }))
    .await;
    let concurrent_merges_ok = merge_responses.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["merged"] == 1)
        })
    });
    let concurrent_lookups_ok = futures_util::future::join_all((0..6).map(|index| {
        let state = Arc::clone(&state);
        async move {
            let response = super::route_http_request(
                "GET",
                &format!("/api/v0/mesh/lookup/mesh-concurrent-entry-{index}"),
                None,
                "",
                &state,
            )
            .await
            .expect("read concurrent mesh merge");
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            response.status == "200 OK" && value["found"] == true
        }
    }))
    .await
    .into_iter()
    .all(|pass| pass);
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "concurrency-and-idempotency",
        concurrent_merges_ok && concurrent_lookups_ok
    );

    let publish_bodies = (0..6)
        .map(|index| {
            serde_json::json!({
                "flacKey": format!("mesh-publish-concurrent-{index}"),
                "byteHash": format!("{:064x}", index + 101),
                "size": 6000 + index,
            })
            .to_string()
        })
        .collect::<Vec<_>>();
    let publish_responses =
        futures_util::future::join_all(publish_bodies.iter().map(|body| {
            super::route_http_request("POST", "/api/v0/mesh/publish", None, body, &state)
        }))
        .await;
    let publish_ok = publish_responses.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["published"] == true)
        })
    });
    let published_lookups_ok = futures_util::future::join_all((0..6).map(|index| {
        let state = Arc::clone(&state);
        async move {
            let response = super::route_http_request(
                "GET",
                &format!("/api/v0/mesh/lookup/mesh-publish-concurrent-{index}"),
                None,
                "",
                &state,
            )
            .await
            .expect("read concurrent mesh publish");
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            response.status == "200 OK" && value["found"] == true
        }
    }))
    .await
    .into_iter()
    .all(|pass| pass);
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "concurrency-and-idempotency",
        publish_ok && published_lookups_ok
    );

    // Replace the test-only in-memory store with the same path-backed
    // store production startup loads. The route mutation must then
    // survive a fresh store load, not merely remain in the live lock.
    let state_dir = state.config.state_dir.clone();
    let persistent_store = super::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("create path-backed mesh discovery store");
    *state.content_discovery.write().await = persistent_store;
    let restart_merge = super::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-restart-peer",
        None,
        r#"{"entries":[{"flacKey":"mesh-restart-merge","byteHash":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","size":7001}]} "#,
        &state,
    )
    .await
    .expect("persist mesh merge");
    let restart_publish = super::route_http_request(
        "POST",
        "/api/v0/mesh/publish",
        None,
        r#"{"flacKey":"mesh-restart-publish","byteHash":"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","size":7002}"#,
        &state,
    )
    .await
    .expect("persist mesh publish");
    let reloaded = super::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("reload path-backed mesh discovery store");
    let restart_merge_json =
        serde_json::from_str::<serde_json::Value>(&restart_merge.body).unwrap_or_default();
    let restart_publish_json =
        serde_json::from_str::<serde_json::Value>(&restart_publish.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "restart-persistence-or-reset",
        restart_merge.status == "200 OK"
            && restart_merge_json["merged"] == 1
            && reloaded.lookup_hash("mesh-restart-merge").is_some()
    );
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "restart-persistence-or-reset",
        restart_publish.status == "200 OK"
            && restart_publish_json["published"] == true
            && reloaded.lookup_hash("mesh-restart-publish").is_some()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh merge/publish evidence directory");
    fs::write(
        evidence_dir.join("mesh_merge_publish_restart_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh merge/publish evidence"),
    )
    .expect("write mesh merge/publish evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh merge/publish mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    fs::remove_dir_all(state_dir).expect("remove mesh merge/publish test state directory");
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
async fn controller_api_differential_mesh_sync_failure_and_concurrency_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($case:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} POST /api/v0/mesh/sync/{{username}} [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": "/api/v0/mesh/sync/{username}",
                "case": $case,
                "pass": pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let route = "/api/v0/mesh/sync/mesh-unsupported-peer";
    let response = super::route_http_request("POST", route, None, "{}", &state)
        .await
        .expect("unsupported mesh sync response");
    let expected_body = r#"{"error":"Failed to sync with peer"}"#;
    let failure_shape = response.status == "400 Bad Request"
        && response.content_type == "application/json"
        && response.body == expected_body;
    record!("nominal-status-headers-body", failure_shape);
    record!("missing-empty-or-conflict-state", failure_shape);

    // A capability-present peer reaches the frozen service's next
    // failure boundary: the pinned build has no usable mesh transport
    // and still returns the same generic controller error.
    state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            "mesh-transport-unavailable-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let transport_failure = super::route_http_request(
        "POST",
        "/api/v0/mesh/sync/mesh-transport-unavailable-peer",
        None,
        "{}",
        &state,
    )
    .await
    .expect("mesh transport failure response");
    record!(
        "runtime-failure-and-timeout",
        transport_failure.status == "400 Bad Request"
            && transport_failure.content_type == "application/json"
            && transport_failure.body == expected_body
    );

    let concurrent = futures_util::future::join_all((0..6).map(|_| {
        let state = Arc::clone(&state);
        async move {
            super::route_http_request(
                "POST",
                "/api/v0/mesh/sync/mesh-unsupported-peer",
                None,
                "{}",
                &state,
            )
            .await
        }
    }))
    .await;
    record!(
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response.as_ref().is_ok_and(|response| {
                response.status == "400 Bad Request"
                    && response.content_type == "application/json"
                    && response.body == expected_body
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh sync evidence directory");
    fs::write(
        evidence_dir.join("mesh_sync_failure_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh sync evidence"),
    )
    .expect("write mesh sync evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh sync mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
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
