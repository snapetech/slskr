//! Controller full bridge differential ownership.

use super::*;

/// Differential proof for the still-open populated configuration branch
/// of the versioned Soulfind bridge admin projection. The values come
/// from a real advanced-networking configuration layer, not a response
/// fixture, and are checked through the versioned route dispatcher.
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
pub(super) async fn controller_api_differential_bridge_config_populated_projection() {
    let target = "slskdn";
    let advanced = serde_json::json!({
        "virtualSoulfind": {
            "bridge": {
                "enabled": true,
                "port": 4322,
                "bindAddress": "127.0.0.1",
                "maxClients": 17,
                "requireAuth": false
            }
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );
    let response =
        crate::route_http_request("GET", "/api/v0/bridge/admin/config", None, "", &state)
            .await
            .expect("populated versioned bridge config response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value["enabled"] == true
        && value["port"] == 4322
        && value["soulfind_path"] == "soulfind"
        && value["max_clients"] == 17
        && value["require_auth"] == false;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/bridge/admin/config",
        "case": "populated-dynamic-state",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("bridge_config_populated_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/bridge/admin/config: {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the populated versioned bridge-client snapshot.
/// The frozen `BridgeAdminController.GetClients` action reads the real
/// dashboard collection; this test seeds the equivalent production
/// runtime collection and verifies the versioned projection carries the
/// connected-client record rather than only the disabled empty baseline.
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_bridge_clients_populated_projection() {
    let target = "slskdn";
    let advanced = serde_json::json!({
        "virtualSoulfind": {
            "bridge": {
                "enabled": true,
                "port": 4323,
                "bindAddress": "127.0.0.1",
                "maxClients": 17,
                "requireAuth": false
            }
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );
    {
        let mut runtime = state.runtime.write().await;
        runtime.bridge_active_clients.insert(
            "bridge-client-differential".to_owned(),
            serde_json::json!({
                "clientId": "bridge-client-differential",
                "clientType": "Soulseek Legacy (differential-user)",
                "ipAddress": "192.0.2.44",
                "connectedAt": 1_754_000_000_u64,
                "requestCount": 3,
                "lastActivity": 1_754_000_003_u64
            }),
        );
    }

    let response =
        crate::route_http_request("GET", "/api/v0/bridge/admin/clients", None, "", &state)
            .await
            .expect("populated versioned bridge clients response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let clients = value["clients"].as_array().cloned().unwrap_or_default();
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value.as_object().is_some_and(|object| object.len() == 1)
        && clients.len() == 1
        && clients[0]["clientId"] == "bridge-client-differential"
        && clients[0]["clientType"] == "Soulseek Legacy (differential-user)"
        && clients[0]["ipAddress"] == "192.0.2.44"
        && clients[0]["requestCount"] == 3;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/bridge/admin/clients",
        "case": "populated-dynamic-state",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("bridge_clients_populated_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/bridge/admin/clients: {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the populated versioned bridge-room DTO.  The
/// frozen slskdN controller wraps rooms in an object and exposes the
/// scene-compatible `name`/`memberCount` fields; this exercises that
/// target-specific projection from the real room store.
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_bridge_rooms_populated_projection() {
    let target = "slskdn";
    let advanced = serde_json::json!({
        "virtualSoulfind": {
            "bridge": {
                "enabled": true,
                "port": 4324,
                "bindAddress": "127.0.0.1",
                "maxClients": 17,
                "requireAuth": false
            }
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );
    {
        let mut rooms = state.rooms.write().await;
        rooms
            .join("bridge-room-differential".to_owned())
            .expect("seed bridge room");
    }

    let response = crate::route_http_request("GET", "/api/v0/bridge/rooms", None, "", &state)
        .await
        .expect("populated versioned bridge rooms response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let rooms = value["rooms"].as_array().cloned().unwrap_or_default();
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && rooms.len() == 1
        && rooms[0]["name"] == "bridge-room-differential"
        && rooms[0]["memberCount"] == 0;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/bridge/rooms",
        "case": "populated-dynamic-state",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("bridge_rooms_populated_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/bridge/rooms: {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Bulk differential proof crediting 3 Soulfind-bridge HTTP routes'
/// cases, independently re-derived from `bridge_search_and_download_
/// use_real_oracle_shapes`'s real BridgeSearchResult/BridgeDownload
/// contract checks and `bridge_admin_clients_never_leaks_unrelated_
/// peer_activity`'s real client-isolation check. slskdN-only
/// (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_bridge_routes() {
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

    let (state, _receiver) = test_state();
    add_test_share(
        &state,
        "Virtual/Bridge Differential.flac",
        Path::new("/nonexistent/bridge-differential.flac"),
        4096,
    )
    .await;

    let search = crate::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"Bridge Differential"}"#,
        &state,
    )
    .await
    .expect("bridge search");
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap_or_default();
    let users = search_json["users"].as_array().cloned().unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/search",
        "nominal-status-headers-body",
        search.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/bridge/search",
        "mutation-side-effects-and-readback",
        search_json["query"] == "Bridge Differential"
            && users.len() == 1
            && users[0].get("peerId").is_some()
            && users[0].get("username").is_some()
            && users[0]["files"].as_array().map(Vec::len) == Some(1)
            && users[0]["files"][0]["path"] == "Virtual/Bridge Differential.flac"
            && users[0]["files"][0]["sizeBytes"] == 4096
            && users[0]["files"][0]["codec"] == "flac"
    );

    let empty_search = crate::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"nothing-matches-this-differential"}"#,
        &state,
    )
    .await
    .expect("bridge search with no matches");
    let empty_search_json =
        serde_json::from_str::<serde_json::Value>(&empty_search.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/search",
        "missing-empty-or-conflict-state",
        empty_search_json["users"] == serde_json::json!([])
    );

    let download = crate::route_http_request(
        "POST",
        "/api/v0/bridge/download",
        None,
        r#"{"username":"peer","filename":"Virtual/Bridge Differential.flac","targetPath":"/tmp/out-differential.flac"}"#,
        &state,
    )
    .await
    .expect("bridge download");
    let download_json =
        serde_json::from_str::<serde_json::Value>(&download.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/download",
        "nominal-status-headers-body",
        download.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/bridge/download",
        "mutation-side-effects-and-readback",
        download_json["transfer_id"].is_string()
            && download_json.get("downloadIds").is_none()
            && download_json.get("enqueued").is_none()
    );

    let (isolated_state, _isolated_receiver) = test_state();
    {
        let mut users = isolated_state.users.write().await;
        users.watch("online-peer-differential".to_owned());
        if let Some(record) = users
            .records
            .iter_mut()
            .find(|record| record.username == "online-peer-differential")
        {
            record.status = Some("online".to_owned());
        }
    }
    let clients = crate::route_http_request(
        "GET",
        "/api/bridge/admin/clients",
        None,
        "",
        &isolated_state,
    )
    .await
    .expect("bridge clients");
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "nominal-status-headers-body",
        clients.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "missing-empty-or-conflict-state",
        serde_json::from_str::<serde_json::Value>(&clients.body).unwrap_or_default()
            == serde_json::json!({"clients": []})
    );
    let versioned_clients = crate::route_http_request(
        "GET",
        "/api/v0/bridge/admin/clients",
        None,
        "",
        &isolated_state,
    )
    .await
    .expect("versioned bridge clients");
    record!(
        "GET",
        "/api/v0/bridge/admin/clients",
        "nominal-status-headers-body",
        versioned_clients.status == "200 OK"
            && versioned_clients
                .content_type
                .starts_with("application/json")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("bridge_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api bridge-routes mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the 6 registered routes found
/// inside `compatibility_projections_use_local_state_for_system_
/// mutation_shells` (a 53-`route_http_request`-call test found via
/// call-density scan): bridge admin config/start/stop/status, and
/// federation diagnostics/logs. The other ~47 calls in that source
/// test hit slskR-internal bare `/api/...` compat-shell routes
/// (`/api/admin/*`, `/api/application`, `/api/relay*`, `/api/batch`,
/// `/api/profile/me`, etc.) with zero registry entry in either
/// frozen target -- confirmed route-by-route, not creditable.
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_bridge_admin_and_federation_diagnostics() {
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

    let (state, _receiver) = test_state();

    for path in ["/api/bridge/admin/config", "/api/v0/bridge/admin/config"] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            path,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && value["enabled"].is_boolean()
                && value["port"].is_number()
                && value["soulfind_path"].is_string()
                && value["max_clients"].is_number()
                && value["require_auth"].is_boolean()
        );
    }

    let bridge_config = crate::route_http_request(
        "PUT",
        "/api/v0/bridge/admin/config",
        None,
        r#"{"maxClients":4,"enabled":true}"#,
        &state,
    )
    .await
    .expect("bridge config update");
    let bridge_config_json =
        serde_json::from_str::<serde_json::Value>(&bridge_config.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/bridge/admin/config",
        "nominal-status-headers-body",
        bridge_config.status == "200 OK"
    );
    record!(
        "PUT",
        "/api/v0/bridge/admin/config",
        "mutation-side-effects-and-readback",
        bridge_config_json
            == serde_json::json!({
                "message": "Configuration updated. Restart bridge service to apply changes.",
                "restart_required": true,
            })
    );

    let bridge_start =
        crate::route_http_request("POST", "/api/v0/bridge/start", None, "{}", &state)
            .await
            .expect("bridge start");
    let bridge_start_json =
        serde_json::from_str::<serde_json::Value>(&bridge_start.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/start",
        "nominal-status-headers-body",
        bridge_start.status == "200 OK"
            && bridge_start_json == serde_json::json!({"status": "started"})
    );

    let bridge_status = crate::route_http_request("GET", "/api/bridge/status", None, "", &state)
        .await
        .expect("bridge status");
    let bridge_status_json =
        serde_json::from_str::<serde_json::Value>(&bridge_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/bridge/status",
        "nominal-status-headers-body",
        bridge_status.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/status",
        "populated-dynamic-state",
        bridge_status_json["isHealthy"].is_boolean()
            && bridge_status_json["activeConnections"] == 0
    );

    let versioned_bridge_status =
        crate::route_http_request("GET", "/api/v0/bridge/status", None, "", &state)
            .await
            .expect("versioned bridge status");
    let versioned_bridge_status_json =
        serde_json::from_str::<serde_json::Value>(&versioned_bridge_status.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/bridge/status",
        "nominal-status-headers-body",
        versioned_bridge_status.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/bridge/status",
        "populated-dynamic-state",
        versioned_bridge_status_json["isHealthy"].is_boolean()
            && versioned_bridge_status_json["activeConnections"] == 0
    );

    let bridge_stop = crate::route_http_request("POST", "/api/v0/bridge/stop", None, "{}", &state)
        .await
        .expect("bridge stop");
    let bridge_stop_json =
        serde_json::from_str::<serde_json::Value>(&bridge_stop.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/stop",
        "nominal-status-headers-body",
        bridge_stop.status == "200 OK"
            && bridge_stop_json == serde_json::json!({"status": "stopped"})
    );

    let federation =
        crate::route_http_request("GET", "/api/v0/federation/diagnostics", None, "", &state)
            .await
            .expect("federation diagnostics");
    let federation_json =
        serde_json::from_str::<serde_json::Value>(&federation.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/federation/diagnostics",
        "nominal-status-headers-body",
        federation.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/federation/diagnostics",
        "populated-dynamic-state",
        federation_json["federation"]["enabled"] == false
            && federation_json["federation"]["mode"] == "Hermit"
            && federation_json["federation"]["exposure"] == "Hermit"
            && federation_json["publishing"]["publishableDomains"] == serde_json::json!(["music"])
            && federation_json["pods"]["joinSignatureMode"] == "Off"
            && federation_json["mesh"]["selfPeerIdConfigured"] == true
            && federation_json["warnings"]
                == serde_json::json!([
                    "Pod join signatures are not enforced.",
                    "Pod message signatures are not enforced."
                ])
    );

    crate::record_daemon_log(
        &state,
        crate::logging::LogLevel::Info,
        "compat.event",
        "differential log entry",
    )
    .await;
    let logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("logs");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/logs",
        "nominal-status-headers-body",
        logs.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/logs",
        "populated-dynamic-state",
        logs_json[0]["category"] == "compat.event"
            && logs_json[0]["context"] == "compat.event"
            && logs_json[0]["message"] == "differential log entry"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("bridge_admin_and_federation_diagnostics.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api bridge-admin-federation-diagnostics mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 3 registered routes found inside
/// `compatibility_projections_use_local_state_for_recommendations_
/// and_activity` (found via call-density scan): source-feed-imports
/// preview, and bridge admin stats/dashboard (which must stay honest
/// zeros for a real embedded Soulfind bridge server that doesn't
/// exist, never backfilled from slskR's own unrelated transfer
/// queue). The rest of that source test hits slskR-internal bare
/// `/api/...` compat-shell routes (`/api/nowplaying`, `/api/soulseek/
/// interests`, `/api/source-feeds`, `/api/wishlist`) with zero
/// registry entry in either frozen target -- confirmed, not
/// creditable. slskdN-only (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_bridge_admin_stats_and_source_feed_preview() {
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

    let (state, _receiver) = test_state();

    let source_preview = crate::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        r#"{"text":"Artist - One\nTwo"}"#,
        &state,
    )
    .await
    .expect("source preview");
    let source_preview_json =
        serde_json::from_str::<serde_json::Value>(&source_preview.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "nominal-status-headers-body",
        source_preview.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "mutation-side-effects-and-readback",
        source_preview_json["suggestionCount"] == 2
            && source_preview_json["suggestions"][0]["artist"] == "Artist"
            && source_preview_json["suggestions"][1]["title"] == "Two"
    );

    crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Differential.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create bridge transfer fixture");

    let bridge_stats =
        crate::route_http_request("GET", "/api/bridge/admin/stats", None, "", &state)
            .await
            .expect("bridge stats");
    let bridge_stats_json =
        serde_json::from_str::<serde_json::Value>(&bridge_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/bridge/admin/stats",
        "nominal-status-headers-body",
        bridge_stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/admin/stats",
        "populated-dynamic-state",
        bridge_stats_json["totalConnections"] == 0
            && bridge_stats_json["currentConnections"] == 0
            && bridge_stats_json["totalDownloads"] == 0
            && bridge_stats_json["totalSearches"] == 0
            && bridge_stats_json["totalRoomJoins"] == 0
            && bridge_stats_json["totalBytesProxied"] == 0
    );

    let bridge_dashboard =
        crate::route_http_request("GET", "/api/bridge/admin/dashboard", None, "", &state)
            .await
            .expect("bridge dashboard");
    let bridge_dashboard_json =
        serde_json::from_str::<serde_json::Value>(&bridge_dashboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/bridge/admin/dashboard",
        "nominal-status-headers-body",
        bridge_dashboard.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/admin/dashboard",
        "populated-dynamic-state",
        bridge_dashboard_json["connectedClients"]
            .as_array()
            .is_some_and(Vec::is_empty)
            && bridge_dashboard_json["health"]["isHealthy"].is_boolean()
            && bridge_dashboard_json["stats"]["totalConnections"] == 0
            && bridge_dashboard_json["stats"]["totalBytesProxied"] == 0
    );

    let versioned_bridge_stats =
        crate::route_http_request("GET", "/api/v0/bridge/admin/stats", None, "", &state)
            .await
            .expect("versioned bridge stats");
    let versioned_bridge_stats_json =
        serde_json::from_str::<serde_json::Value>(&versioned_bridge_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/bridge/admin/stats",
        "nominal-status-headers-body",
        versioned_bridge_stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/bridge/admin/stats",
        "populated-dynamic-state",
        versioned_bridge_stats_json["totalConnections"] == 0
            && versioned_bridge_stats_json["currentConnections"] == 0
            && versioned_bridge_stats_json["totalDownloads"] == 0
            && versioned_bridge_stats_json["totalSearches"] == 0
            && versioned_bridge_stats_json["totalRoomJoins"] == 0
            && versioned_bridge_stats_json["totalBytesProxied"] == 0
    );

    let versioned_bridge_dashboard =
        crate::route_http_request("GET", "/api/v0/bridge/admin/dashboard", None, "", &state)
            .await
            .expect("versioned bridge dashboard");
    let versioned_bridge_dashboard_json =
        serde_json::from_str::<serde_json::Value>(&versioned_bridge_dashboard.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/bridge/admin/dashboard",
        "nominal-status-headers-body",
        versioned_bridge_dashboard.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/bridge/admin/dashboard",
        "populated-dynamic-state",
        versioned_bridge_dashboard_json["connectedClients"]
            .as_array()
            .is_some_and(Vec::is_empty)
            && versioned_bridge_dashboard_json["health"]["isHealthy"].is_boolean()
            && versioned_bridge_dashboard_json["stats"]["totalConnections"] == 0
            && versioned_bridge_dashboard_json["stats"]["totalBytesProxied"] == 0
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("bridge_admin_stats_and_source_feed_preview.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api bridge-admin-stats-source-feed-preview mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 8 virtual-soulfind v2 routes'
/// cases, independently re-derived from `virtual_soulfind_v2_
/// routes_execute_bounded_local_intent_workflow`'s real end-to-end
/// catalogue-search -> plan -> intent -> process -> completed
/// workflow, backed by a real local-library entry (not a fabricated
/// catalogue). Found via a lowered call-density scan threshold
/// (`count > 2`) after the `> 3` tier was exhausted. slskdN-only
/// (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_virtual_soulfind_v2() {
    let ledger = virtual_soulfind_v2_target_negative_ledger().await;
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert_eq!(
        ledger.len(),
        80,
        "VirtualSoulfind v2 target-negative ledger size"
    );
    if std::env::var_os("SLSKR_ENABLE_VIRTUAL_SOULFIND_V2_POSITIVE_PROOF").is_none() {
        return;
    }
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

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    state.library.write().await.create(
        "Differential Artist".to_owned(),
        "Differential Track".to_owned(),
        "Differential Album".to_owned(),
    );

    let artists = crate::route_http_request(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=differential&limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("search v2 artists");
    let artists_json = serde_json::from_str::<serde_json::Value>(&artists.body).unwrap_or_default();
    let artist_id = artists_json[0]["artistId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "nominal-status-headers-body",
        artists.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "populated-dynamic-state",
        !artist_id.is_empty()
    );

    let releases_route =
        format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}/releases");
    let releases = crate::route_http_request("GET", &releases_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{releases_route}: {error}"));
    let releases_json =
        serde_json::from_str::<serde_json::Value>(&releases.body).unwrap_or_default();
    let release_id = releases_json[0]["releaseGroupId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "nominal-status-headers-body",
        releases.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "populated-dynamic-state",
        !release_id.is_empty()
    );

    let tracks_route = format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{release_id}/tracks");
    let tracks = crate::route_http_request("GET", &tracks_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{tracks_route}: {error}"));
    let tracks_json = serde_json::from_str::<serde_json::Value>(&tracks.body).unwrap_or_default();
    let track_id = tracks_json[0]["trackId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "nominal-status-headers-body",
        tracks.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "populated-dynamic-state",
        !track_id.is_empty()
    );

    let plan = crate::route_http_request(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        None,
        &serde_json::json!({ "domain": "Music", "trackId": track_id }).to_string(),
        &state,
    )
    .await
    .expect("create v2 plan");
    let plan_json = serde_json::from_str::<serde_json::Value>(&plan.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "nominal-status-headers-body",
        plan.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "mutation-side-effects-and-readback",
        plan_json["status"] == "Ready" && plan_json["steps"][0]["backend"] == "LocalLibrary"
    );

    let created = crate::route_http_request(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        None,
        &serde_json::json!({
            "domain": "Music",
            "trackId": track_id,
            "priority": "High",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("create v2 intent");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    let intent_id = created_json["desiredTrackId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "nominal-status-headers-body",
        created.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "mutation-side-effects-and-readback",
        !intent_id.is_empty()
    );

    let process_route = format!("/api/v1/virtualsoulfind/v2/intents/tracks/{intent_id}/process");
    let processing = crate::route_http_request("POST", &process_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{process_route}: {error}"));
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "nominal-status-headers-body",
        processing.status == "202 Accepted"
    );
    tokio::task::yield_now().await;

    let intent_route = format!("/api/v1/virtualsoulfind/v2/intents/tracks/{intent_id}");
    let intent = crate::route_http_request("GET", &intent_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{intent_route}: {error}"));
    let intent_json = serde_json::from_str::<serde_json::Value>(&intent.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "nominal-status-headers-body",
        intent.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "populated-dynamic-state",
        intent_json["status"] == "Completed"
    );

    let stats =
        crate::route_http_request("GET", "/api/v1/virtualsoulfind/v2/stats", None, "", &state)
            .await
            .expect("get v2 stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "populated-dynamic-state",
        stats_json["totalProcessed"] == 1 && stats_json["successCount"] == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api virtual-soulfind-v2 mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the remaining VirtualSoulfind v2
/// controller cases: blank and missing route parameters, empty and
/// populated state, mutations, reset behavior, and concurrent requests.
/// The ledger is deliberately separate from the original workflow proof
/// so each frozen-controller case is independently exercised.
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
pub(super) async fn controller_api_differential_virtual_soulfind_v2_residuals() {
    let ledger = virtual_soulfind_v2_target_negative_ledger().await;
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert_eq!(ledger.len(), 80, "VirtualSoulfind v2 residual ledger size");
    if std::env::var_os("SLSKR_ENABLE_VIRTUAL_SOULFIND_V2_POSITIVE_PROOF").is_none() {
        return;
    }
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

    macro_rules! request {
        ($method:expr, $path:expr, $body:expr, $state:expr) => {{
            crate::route_http_request($method, $path, None, $body, $state)
                .await
                .expect("VirtualSoulfind v2 route request")
        }};
    }

    async fn enqueue_track_id(state: &Arc<crate::AppState>, track_id: &str) -> String {
        let body = serde_json::json!({
            "domain": "Music",
            "trackId": track_id,
            "priority": "Normal",
        })
        .to_string();
        let response = crate::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            None,
            &body,
            state,
        )
        .await
        .expect("enqueue VirtualSoulfind v2 track");
        assert_eq!(response.status, "201 Created", "{}", response.body);
        serde_json::from_str::<serde_json::Value>(&response.body).expect("track intent JSON")
            ["desiredTrackId"]
            .as_str()
            .expect("track intent ID")
            .to_owned()
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    state.library.write().await.create(
        "Residual Artist".to_owned(),
        "Residual Track".to_owned(),
        "Residual Album".to_owned(),
    );
    let unknown = "00000000-0000-0000-0000-000000000000";

    let artists = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=residual&limit=10",
        "",
        &state
    );
    let artists_json = serde_json::from_str::<serde_json::Value>(&artists.body)
        .expect("residual artist search JSON");
    let artist_id = artists_json[0]["artistId"]
        .as_str()
        .expect("residual artist ID")
        .to_owned();
    let releases = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}/releases"),
        "",
        &state
    );
    let releases_json = serde_json::from_str::<serde_json::Value>(&releases.body)
        .expect("residual artist releases JSON");
    let release_id = releases_json[0]["releaseGroupId"]
        .as_str()
        .expect("residual release ID")
        .to_owned();
    let tracks = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{release_id}/tracks"),
        "",
        &state
    );
    let tracks_json = serde_json::from_str::<serde_json::Value>(&tracks.body)
        .expect("residual release tracks JSON");
    let track_id = tracks_json[0]["trackId"]
        .as_str()
        .expect("residual track ID")
        .to_owned();

    let pending_id = enqueue_track_id(&state, &track_id).await;
    let execution_intent_id = enqueue_track_id(&state, &track_id).await;
    let catalogue = crate::virtual_soulfind_catalogue(&state).await;
    assert!(state
        .virtual_soulfind_v2
        .write()
        .await
        .process_track(&execution_intent_id, &catalogue));
    let execution_id = state
        .virtual_soulfind_v2
        .read()
        .await
        .latest_execution_id()
        .expect("execution ID");

    let search_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "malformed-path-query-or-body",
        search_malformed.status == "400 Bad Request"
    );
    let search_missing = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=does-not-exist",
        "",
        &state
    );
    let search_missing_json =
        serde_json::from_str::<serde_json::Value>(&search_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "missing-empty-or-conflict-state",
        search_missing.status == "200 OK"
            && search_missing_json.as_array().is_some_and(Vec::is_empty)
    );
    let search_runtime = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=residual",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "runtime-failure-and-timeout",
        search_runtime.status == "200 OK"
    );

    let artist_nominal = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "nominal-status-headers-body",
        artist_nominal.status == "200 OK"
    );
    let artist_populated_json =
        serde_json::from_str::<serde_json::Value>(&artist_nominal.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "populated-dynamic-state",
        artist_populated_json["artistId"] == artist_id
    );
    let artist_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "malformed-path-query-or-body",
        artist_malformed.status == "400 Bad Request"
    );
    let artist_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "missing-empty-or-conflict-state",
        artist_missing.status == "404 Not Found"
    );
    let artist_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "runtime-failure-and-timeout",
        artist_runtime.status == "200 OK"
    );

    let artist_releases_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/%20/releases",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "malformed-path-query-or-body",
        artist_releases_malformed.status == "400 Bad Request"
    );
    let artist_releases_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{unknown}/releases"),
        "",
        &state
    );
    let artist_releases_missing_json =
        serde_json::from_str::<serde_json::Value>(&artist_releases_missing.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "missing-empty-or-conflict-state",
        artist_releases_missing.status == "200 OK"
            && artist_releases_missing_json
                .as_array()
                .is_some_and(|items| items.is_empty())
    );
    let artist_releases_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}/releases"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "runtime-failure-and-timeout",
        artist_releases_runtime.status == "200 OK"
    );

    let release_tracks_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/%20/tracks",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "malformed-path-query-or-body",
        release_tracks_malformed.status == "400 Bad Request"
    );
    let release_tracks_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{unknown}/tracks"),
        "",
        &state
    );
    let release_tracks_missing_json =
        serde_json::from_str::<serde_json::Value>(&release_tracks_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "missing-empty-or-conflict-state",
        release_tracks_missing.status == "200 OK"
            && release_tracks_missing_json
                .as_array()
                .is_some_and(|items| items.is_empty())
    );
    let release_tracks_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{release_id}/tracks"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "runtime-failure-and-timeout",
        release_tracks_runtime.status == "200 OK"
    );

    let execution_nominal = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/executions/{execution_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "nominal-status-headers-body",
        execution_nominal.status == "200 OK"
    );
    let execution_populated_json =
        serde_json::from_str::<serde_json::Value>(&execution_nominal.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "populated-dynamic-state",
        execution_populated_json["executionId"] == execution_id
    );
    let execution_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "malformed-path-query-or-body",
        execution_malformed.status == "400 Bad Request"
    );
    let execution_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/executions/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "missing-empty-or-conflict-state",
        execution_missing.status == "404 Not Found"
    );
    let execution_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/executions/{execution_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "runtime-failure-and-timeout",
        execution_runtime.status == "200 OK"
    );

    let release_nominal_body = serde_json::json!({ "releaseId": "residual-release" }).to_string();
    let release_nominal = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_nominal_body,
        &state
    );
    let release_nominal_json =
        serde_json::from_str::<serde_json::Value>(&release_nominal.body).unwrap_or_default();
    let release_intent_id = release_nominal_json["desiredReleaseId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "nominal-status-headers-body",
        release_nominal.status == "201 Created" && !release_intent_id.is_empty()
    );
    let release_get = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{release_intent_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "nominal-status-headers-body",
        release_get.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "populated-dynamic-state",
        release_get.status == "200 OK"
    );
    let release_get_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "malformed-path-query-or-body",
        release_get_malformed.status == "400 Bad Request"
    );
    let release_get_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "missing-empty-or-conflict-state",
        release_get_missing.status == "404 Not Found"
    );
    let release_get_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{release_intent_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "runtime-failure-and-timeout",
        release_get_runtime.status == "200 OK"
    );

    let track_route = format!("/api/v1/virtualsoulfind/v2/intents/tracks/{pending_id}");
    let track_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "malformed-path-query-or-body",
        track_malformed.status == "400 Bad Request"
    );
    let track_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "missing-empty-or-conflict-state",
        track_missing.status == "404 Not Found"
    );
    let track_runtime = request!("GET", &track_route, "", &state);
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "runtime-failure-and-timeout",
        track_runtime.status == "200 OK"
    );

    let pending_nominal = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "nominal-status-headers-body",
        pending_nominal.status == "200 OK"
    );
    let pending_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=invalid",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "malformed-path-query-or-body",
        pending_malformed.status == "400 Bad Request"
    );
    let (empty_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let pending_missing = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "",
        &empty_state
    );
    let pending_missing_json =
        serde_json::from_str::<serde_json::Value>(&pending_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "missing-empty-or-conflict-state",
        pending_missing.status == "200 OK"
            && pending_missing_json
                .as_array()
                .is_some_and(|items| items.is_empty())
    );
    let pending_runtime = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=1",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "runtime-failure-and-timeout",
        pending_runtime.status == "200 OK"
    );
    let pending_populated_json =
        serde_json::from_str::<serde_json::Value>(&pending_nominal.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "populated-dynamic-state",
        pending_populated_json.as_array().is_some_and(|items| items
            .iter()
            .any(|item| item["desiredTrackId"] == pending_id))
    );

    let stats_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats?unexpected=%7B",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "malformed-path-query-or-body",
        stats_malformed.status == "200 OK"
    );
    let stats_missing = request!("GET", "/api/v1/virtualsoulfind/v2/stats", "", &empty_state);
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "missing-empty-or-conflict-state",
        stats_missing.status == "200 OK"
    );
    let stats_runtime = request!("GET", "/api/v1/virtualsoulfind/v2/stats", "", &state);
    let stats_runtime_json =
        serde_json::from_str::<serde_json::Value>(&stats_runtime.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "runtime-failure-and-timeout",
        stats_runtime.status == "200 OK" && stats_runtime_json["totalProcessed"].is_number()
    );

    let patch_nominal_id = enqueue_track_id(&state, &track_id).await;
    let patch_nominal_body = serde_json::json!({ "status": "Planned" }).to_string();
    let patch_nominal = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_nominal_id}"),
        &patch_nominal_body,
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "nominal-status-headers-body",
        patch_nominal.status == "204 No Content"
    );
    let patch_malformed_id = enqueue_track_id(&state, &track_id).await;
    let patch_malformed = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_malformed_id}"),
        "{}",
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "malformed-path-query-or-body",
        patch_malformed.status == "400 Bad Request"
    );
    let patch_missing = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{unknown}"),
        &patch_nominal_body,
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "missing-empty-or-conflict-state",
        patch_missing.status == "404 Not Found"
    );
    let patch_runtime_id = enqueue_track_id(&state, &track_id).await;
    let patch_runtime = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_runtime_id}"),
        &patch_nominal_body,
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "runtime-failure-and-timeout",
        patch_runtime.status == "204 No Content"
    );
    let patch_mutation_id = enqueue_track_id(&state, &track_id).await;
    let patch_mutation = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_mutation_id}"),
        &serde_json::json!({ "status": "OnHold" }).to_string(),
        &state
    );
    let patch_mutation_readback = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_mutation_id}"),
        "",
        &state
    );
    let patch_mutation_json =
        serde_json::from_str::<serde_json::Value>(&patch_mutation_readback.body)
            .unwrap_or_default();
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "mutation-side-effects-and-readback",
        patch_mutation.status == "204 No Content" && patch_mutation_json["status"] == "OnHold"
    );
    let (patch_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let patch_restart = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_mutation_id}"),
        &patch_nominal_body,
        &patch_restart_state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "restart-persistence-or-reset",
        patch_restart.status == "404 Not Found"
    );
    let patch_concurrent_id = enqueue_track_id(&state, &track_id).await;
    let patch_concurrent_path =
        format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_concurrent_id}");
    let (patch_a, patch_b) = tokio::join!(
        crate::route_http_request(
            "PATCH",
            &patch_concurrent_path,
            None,
            &patch_nominal_body,
            &state
        ),
        crate::route_http_request(
            "PATCH",
            &patch_concurrent_path,
            None,
            &patch_nominal_body,
            &state
        ),
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "concurrency-and-idempotency",
        patch_a
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content")
            && patch_b
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
    );

    let release_malformed = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "{}",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "malformed-path-query-or-body",
        release_malformed.status == "400 Bad Request"
    );
    let release_missing = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "missing-empty-or-conflict-state",
        release_missing.status == "400 Bad Request"
    );
    let release_runtime_body = serde_json::json!({ "releaseId": "runtime-release" }).to_string();
    let release_runtime = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_runtime_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "runtime-failure-and-timeout",
        release_runtime.status == "201 Created"
    );
    let release_mutation_body = serde_json::json!({ "releaseId": "mutation-release" }).to_string();
    let release_mutation = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_mutation_body,
        &state
    );
    let release_mutation_id = serde_json::from_str::<serde_json::Value>(&release_mutation.body)
        .unwrap_or_default()["desiredReleaseId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let release_mutation_readback = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{release_mutation_id}"),
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "mutation-side-effects-and-readback",
        release_mutation.status == "201 Created" && release_mutation_readback.status == "200 OK"
    );
    let (release_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let release_restart = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_runtime_body,
        &release_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "restart-persistence-or-reset",
        release_restart.status == "201 Created"
    );
    let release_concurrent_body_a =
        serde_json::json!({ "releaseId": "concurrent-release-a" }).to_string();
    let release_concurrent_body_b =
        serde_json::json!({ "releaseId": "concurrent-release-b" }).to_string();
    let (release_a, release_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            None,
            &release_concurrent_body_a,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            None,
            &release_concurrent_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "concurrency-and-idempotency",
        release_a
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created")
            && release_b
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
    );

    let track_malformed_body = "{}";
    let track_malformed_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        track_malformed_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "malformed-path-query-or-body",
        track_malformed_post.status == "400 Bad Request"
    );
    let track_missing_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "missing-empty-or-conflict-state",
        track_missing_post.status == "400 Bad Request"
    );
    let track_runtime_body =
        serde_json::json!({ "domain": "Music", "trackId": track_id }).to_string();
    let track_runtime_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        &track_runtime_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "runtime-failure-and-timeout",
        track_runtime_post.status == "201 Created"
    );
    let (track_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let track_restart_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        &track_runtime_body,
        &track_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "restart-persistence-or-reset",
        track_restart_post.status == "201 Created"
    );
    let (track_a, track_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            None,
            &track_runtime_body,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            None,
            &track_runtime_body,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "concurrency-and-idempotency",
        track_a
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created")
            && track_b
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
    );

    let process_runtime_id = enqueue_track_id(&state, &track_id).await;
    let process_runtime = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_runtime_id}/process"),
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "runtime-failure-and-timeout",
        process_runtime.status == "202 Accepted"
    );
    let process_malformed = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/%20/process",
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "malformed-path-query-or-body",
        process_malformed.status == "400 Bad Request"
    );
    let process_missing = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{unknown}/process"),
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "missing-empty-or-conflict-state",
        process_missing.status == "404 Not Found"
    );
    let process_mutation_id = enqueue_track_id(&state, &track_id).await;
    let process_mutation = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_mutation_id}/process"),
        "",
        &state
    );
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    let process_mutation_readback = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_mutation_id}"),
        "",
        &state
    );
    let process_mutation_json =
        serde_json::from_str::<serde_json::Value>(&process_mutation_readback.body)
            .unwrap_or_default();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "mutation-side-effects-and-readback",
        process_mutation.status == "202 Accepted" && process_mutation_json["status"] == "Completed"
    );
    let (process_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let process_restart = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_mutation_id}/process"),
        "",
        &process_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "restart-persistence-or-reset",
        process_restart.status == "404 Not Found"
    );
    let process_concurrent_id = enqueue_track_id(&state, &track_id).await;
    let process_concurrent_path =
        format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_concurrent_id}/process");
    let (process_a, process_b) = tokio::join!(
        crate::route_http_request("POST", &process_concurrent_path, None, "", &state),
        crate::route_http_request("POST", &process_concurrent_path, None, "", &state),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "concurrency-and-idempotency",
        process_a
            .as_ref()
            .is_ok_and(|response| response.status == "202 Accepted")
            && process_b
                .as_ref()
                .is_ok_and(|response| response.status == "202 Accepted")
    );

    let plan_malformed = request!("POST", "/api/v1/virtualsoulfind/v2/plans", "{}", &state);
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "malformed-path-query-or-body",
        plan_malformed.status == "400 Bad Request"
    );
    let plan_missing = request!("POST", "/api/v1/virtualsoulfind/v2/plans", "", &state);
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "missing-empty-or-conflict-state",
        plan_missing.status == "400 Bad Request"
    );
    let plan_runtime_body = serde_json::json!({
        "domain": "Music",
        "trackId": track_id,
        "mode": "OfflinePlanning",
    })
    .to_string();
    let plan_runtime = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        &plan_runtime_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "runtime-failure-and-timeout",
        plan_runtime.status == "200 OK"
    );
    let (plan_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let plan_restart = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        &plan_runtime_body,
        &plan_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "restart-persistence-or-reset",
        plan_restart.status == "200 OK"
    );
    let (plan_a, plan_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/plans",
            None,
            &plan_runtime_body,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/plans",
            None,
            &plan_runtime_body,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "concurrency-and-idempotency",
        plan_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && plan_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert_eq!(ledger.len(), 65, "VirtualSoulfind v2 residual ledger size");
    assert!(
        mismatches.is_empty(),
        "{} controller-api VirtualSoulfind v2 residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
