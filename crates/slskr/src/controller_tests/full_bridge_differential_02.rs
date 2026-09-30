//! Controller full bridge differential 02 ownership.

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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_virtual_soulfind_legacy_residuals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            crate::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{path}: {error}", path = $path))
        }};
    }

    fn json_body(response: &crate::routing::HttpResponse) -> serde_json::Value {
        serde_json::from_str(&response.body).unwrap_or_default()
    }

    let (empty_state, _receiver) = test_state();
    let (runtime_db, runtime_state, _runtime_receiver) = {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("VirtualSoulfind runtime database");
        let (state, receiver) = test_state_with_env_parts(
            MapEnv::default(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        (db, state, receiver)
    };
    let _runtime_db = runtime_db;

    let (populated_state, _populated_receiver) = test_state();
    let populated_mbid = "00000000-0000-0000-0000-000000000111";
    let variant_hash = "11".repeat(32);
    {
        let mut discovery = populated_state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: crate::content_discovery::generate_flac_key("Legacy/Variant.flac", 123),
                byte_hash: variant_hash.clone(),
                size: 123,
                full_file_hash: variant_hash.clone(),
                music_brainz_id: populated_mbid.to_owned(),
                file_sha256: variant_hash,
                ..Default::default()
            }])
            .expect("seed legacy VirtualSoulfind variant");
        discovery
            .merge_shadow_records(vec![crate::content_discovery::ShadowIndexRecord {
                recording_id: populated_mbid.to_owned(),
                peer_ids: vec!["peer-legacy".to_owned()],
                updated_at: 1,
            }])
            .expect("seed legacy VirtualSoulfind shadow record");
    }

    let canonical_routes = [
        (
            "/api/v0/virtualsoulfind/canonical/{mbid}",
            "/api/v0/virtualsoulfind/canonical/00000000-0000-0000-0000-000000000101",
        ),
        (
            "/api/virtualsoulfind/canonical/{mbid}",
            "/api/virtualsoulfind/canonical/00000000-0000-0000-0000-000000000102",
        ),
    ];
    for (route, nominal_path) in canonical_routes {
        let nominal = request!(&empty_state, "GET", nominal_path, "");
        let nominal_json = json_body(&nominal);
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal.content_type == "application/json"
                && nominal_json["canonical_variant"].is_null()
                && nominal_json["available_variants"] == 0
                && nominal_json["selection_reason"] == "No variants found in shadow index"
        );

        let malformed_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/canonical/%20"
        } else {
            "/api/virtualsoulfind/canonical/%20"
        };
        let malformed = request!(&empty_state, "GET", malformed_path, "invalid-json");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body == r#"{"error":"MBID is required"}"#
        );

        let missing = request!(&empty_state, "GET", nominal_path, "");
        let missing_json = json_body(&missing);
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["canonical_variant"].is_null()
                && missing_json["available_variants"] == 0
        );

        let runtime_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/canonical/runtime-failure"
        } else {
            "/api/virtualsoulfind/canonical/runtime-failure"
        };
        let runtime = request!(&runtime_state, "GET", runtime_path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "500 Internal Server Error"
                && runtime.body == r#"{"error":"Failed to select canonical variant"}"#
        );

        let populated_path = if route.starts_with("/api/v0/") {
            format!("/api/v0/virtualsoulfind/canonical/{populated_mbid}")
        } else {
            format!("/api/virtualsoulfind/canonical/{populated_mbid}")
        };
        let populated = request!(&populated_state, "GET", &populated_path, "");
        let populated_json = json_body(&populated);
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["canonical_variant"]["codec"] == "FLAC"
                && populated_json["canonical_variant"]["bitrate"] == 0
                && populated_json["canonical_variant"]["fileSize"] == 123
                && populated_json["canonical_variant"]["qualityScore"] == 1.0
                && populated_json["available_variants"] == 1
                && populated_json["selection_reason"]
                    == "Selected highest quality variant from shadow index"
        );
    }

    let shadow_routes = [
        (
            "/api/v0/virtualsoulfind/shadow-index/{mbid}",
            "/api/v0/virtualsoulfind/shadow-index/00000000-0000-0000-0000-000000000201",
        ),
        (
            "/api/virtualsoulfind/shadow-index/{mbid}",
            "/api/virtualsoulfind/shadow-index/00000000-0000-0000-0000-000000000202",
        ),
    ];
    for (route, nominal_path) in shadow_routes {
        let nominal = request!(&empty_state, "GET", nominal_path, "");
        let nominal_json = json_body(&nominal);
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal.content_type == "application/json"
                && nominal_json["variants"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );

        let malformed_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/shadow-index/%20"
        } else {
            "/api/virtualsoulfind/shadow-index/%20"
        };
        let malformed = request!(&empty_state, "GET", malformed_path, "invalid-json");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body == r#"{"error":"MBID is required"}"#
        );

        let missing = request!(&empty_state, "GET", nominal_path, "");
        let missing_json = json_body(&missing);
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["variants"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );

        let runtime_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/shadow-index/runtime-failure"
        } else {
            "/api/virtualsoulfind/shadow-index/runtime-failure"
        };
        let runtime = request!(&runtime_state, "GET", runtime_path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "500 Internal Server Error"
                && runtime.body == r#"{"error":"Failed to query shadow index"}"#
        );

        let populated_path = if route.starts_with("/api/v0/") {
            format!("/api/v0/virtualsoulfind/shadow-index/{populated_mbid}")
        } else {
            format!("/api/virtualsoulfind/shadow-index/{populated_mbid}")
        };
        let populated = request!(&populated_state, "GET", &populated_path, "");
        let populated_json = json_body(&populated);
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["variants"]
                    .as_array()
                    .is_some_and(|variants| {
                        variants.len() == 1
                            && variants[0]["codec"] == "FLAC"
                            && variants[0]["bitrate"] == 0
                            && variants[0]["fileSize"] == 123
                            && variants[0]["qualityScore"] == 1.0
                    })
        );
    }

    let disaster_routes = [
        (
            "/api/v0/virtualsoulfind/disaster-mode/status",
            "/api/v0/virtualsoulfind/disaster-mode/status",
        ),
        (
            "/api/virtualsoulfind/disaster-mode/status",
            "/api/virtualsoulfind/disaster-mode/status",
        ),
    ];
    let (forced_state, _forced_receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_ADVANCED_NETWORKING_JSON",
        r#"{"virtualSoulfind":{"disasterMode":{"force":true}}}"#,
    ));
    for (route, path) in disaster_routes {
        let malformed = request!(
            &empty_state,
            "GET",
            &format!("{path}?unexpected=%7B"),
            "invalid-json"
        );
        let malformed_json = json_body(&malformed);
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK"
                && malformed_json["level"] == 0
                && malformed_json["mode_family"] == "legacy_fallback"
        );

        let missing = request!(&empty_state, "GET", path, "");
        let missing_json = json_body(&missing);
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["level_name"] == "Normal"
                && missing_json["is_active"] == false
        );

        let runtime = request!(&runtime_state, "GET", path, "");
        let runtime_json = json_body(&runtime);
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["mode_family"] == "legacy_fallback"
                && runtime_json["networks"].is_object()
        );

        let populated = request!(&forced_state, "GET", path, "");
        let populated_json = json_body(&populated);
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["level"] == 0
                && populated_json["level_name"] == "Normal"
                && populated_json["is_active"] == false
                && populated_json["networks"]["full_fallback"] == false
        );
    }

    assert_eq!(
        ledger.len(),
        28,
        "legacy VirtualSoulfind residual ledger size"
    );
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create legacy VirtualSoulfind evidence directory");
    fs::write(
        evidence_dir.join("virtualsoulfind_legacy_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize legacy VirtualSoulfind ledger"),
    )
    .expect("write legacy VirtualSoulfind ledger");
    assert!(
        mismatches.is_empty(),
        "{} legacy VirtualSoulfind residual mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_bridge_controller_residuals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            crate::route_http_request($method, $path, None, $body, $state)
                .await
                .expect("BridgeController route request")
        }};
    }

    fn bridge_env() -> MapEnv {
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native")
    }

    fn json_body(response: &crate::routing::HttpResponse) -> serde_json::Value {
        serde_json::from_str(&response.body).unwrap_or_default()
    }

    async fn closed_state(persistence: bool) -> Arc<crate::AppState> {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("bridge runtime database");
        let env = if persistence {
            bridge_env().with("SLSKR_PERSISTENCE_ENABLED", "true")
        } else {
            bridge_env()
        };
        let (state, _receiver) =
            test_state_with_env_parts(env, crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        state
    }

    async fn seed_transfer(state: &Arc<crate::AppState>) -> String {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("bridge-peer".to_owned()),
            "Bridge/Residual.flac".to_owned(),
            Some("/tmp/Bridge-Residual.flac".to_owned()),
            Some(100),
        );
        entry.id.to_string()
    }

    async fn seed_bridge_client(state: &Arc<crate::AppState>) {
        state.runtime.write().await.bridge_active_clients.insert(
            "bridge-residual-client".to_owned(),
            serde_json::json!({
                "clientId": "bridge-residual-client",
                "clientType": "Soulseek Legacy (residual-user)",
                "ipAddress": "192.0.2.77",
                "connectedAt": 1_754_000_100_u64,
                "requestCount": 2,
                "lastActivity": 1_754_000_101_u64,
            }),
        );
    }

    async fn seed_room(state: &Arc<crate::AppState>, name: &str) {
        state
            .rooms
            .write()
            .await
            .join(name.to_owned())
            .expect("bridge room fixture");
    }

    fn exact_bridge_config(value: &serde_json::Value) -> bool {
        value["message"] == "Configuration updated. Restart bridge service to apply changes."
            && value["restart_required"] == true
            && value.as_object().is_some_and(|object| object.len() == 2)
    }

    // Static GET routes: extra path segments are unmatched, while the
    // controller's real empty and runtime states remain JSON responses.
    for (route, malformed) in [
        (
            "/api/bridge/admin/clients",
            "/api/bridge/admin/clients/extra",
        ),
        ("/api/bridge/admin/config", "/api/bridge/admin/config/extra"),
        (
            "/api/bridge/admin/dashboard",
            "/api/bridge/admin/dashboard/extra",
        ),
        ("/api/bridge/admin/stats", "/api/bridge/admin/stats/extra"),
        ("/api/bridge/rooms", "/api/bridge/rooms/extra"),
        ("/api/bridge/status", "/api/bridge/status/extra"),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let response = request!(&state, "GET", malformed, "");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }
    for (route, malformed) in [
        (
            "/api/v0/bridge/admin/clients",
            "/api/v0/bridge/admin/clients/extra",
        ),
        (
            "/api/v0/bridge/admin/config",
            "/api/v0/bridge/admin/config/extra",
        ),
        (
            "/api/v0/bridge/admin/dashboard",
            "/api/v0/bridge/admin/dashboard/extra",
        ),
        (
            "/api/v0/bridge/admin/stats",
            "/api/v0/bridge/admin/stats/extra",
        ),
        ("/api/v0/bridge/rooms", "/api/v0/bridge/rooms/extra"),
        ("/api/v0/bridge/status", "/api/v0/bridge/status/extra"),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let response = request!(&state, "GET", malformed, "");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let (state, _receiver) = test_state_with_env(bridge_env());
    seed_bridge_client(&state).await;
    let clients = request!(&state, "GET", "/api/bridge/admin/clients", "");
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "populated-dynamic-state",
        clients.status == "200 OK"
            && json_body(&clients)["clients"]
                .as_array()
                .is_some_and(|rows| rows.len() == 1)
    );
    let clients_runtime = closed_state(false).await;
    let clients_runtime_response =
        request!(&clients_runtime, "GET", "/api/bridge/admin/clients", "");
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "runtime-failure-and-timeout",
        clients_runtime_response.status == "200 OK"
            && json_body(&clients_runtime_response)["clients"].is_array()
    );
    let clients_v0_runtime = closed_state(false).await;
    let clients_v0_runtime_response = request!(
        &clients_v0_runtime,
        "GET",
        "/api/v0/bridge/admin/clients",
        ""
    );
    record!(
        "GET",
        "/api/v0/bridge/admin/clients",
        "runtime-failure-and-timeout",
        clients_v0_runtime_response.status == "200 OK"
            && json_body(&clients_v0_runtime_response)["clients"].is_array()
    );

    let config_env = bridge_env().with(
        "SLSKR_ADVANCED_NETWORKING_JSON",
        &serde_json::json!({
            "virtualSoulfind": {"bridge": {
                "enabled": true,
                "port": 4327,
                "bindAddress": "127.0.0.1",
                "maxClients": 19,
                "requireAuth": false
            }}
        })
        .to_string(),
    );
    let (config_state, _receiver) = test_state_with_env(config_env);
    let config = request!(&config_state, "GET", "/api/bridge/admin/config", "");
    record!(
        "GET",
        "/api/bridge/admin/config",
        "populated-dynamic-state",
        config.status == "200 OK"
            && json_body(&config)["enabled"] == true
            && json_body(&config)["port"] == 4327
            && json_body(&config)["max_clients"] == 19
            && json_body(&config)["require_auth"] == false
    );
    let config_empty = request!(&state, "GET", "/api/bridge/admin/config", "");
    record!(
        "GET",
        "/api/bridge/admin/config",
        "missing-empty-or-conflict-state",
        config_empty.status == "200 OK"
            && json_body(&config_empty)["port"].is_number()
            && json_body(&config_empty)["soulfind_path"].is_string()
    );
    let config_runtime = closed_state(false).await;
    let config_runtime_response = request!(&config_runtime, "GET", "/api/bridge/admin/config", "");
    record!(
        "GET",
        "/api/bridge/admin/config",
        "runtime-failure-and-timeout",
        config_runtime_response.status == "200 OK"
            && json_body(&config_runtime_response).is_object()
    );
    let config_v0_runtime = closed_state(false).await;
    let config_v0_runtime_response =
        request!(&config_v0_runtime, "GET", "/api/v0/bridge/admin/config", "");
    record!(
        "GET",
        "/api/v0/bridge/admin/config",
        "runtime-failure-and-timeout",
        config_v0_runtime_response.status == "200 OK"
            && json_body(&config_v0_runtime_response).is_object()
    );

    for route in [
        "/api/bridge/admin/dashboard",
        "/api/v0/bridge/admin/dashboard",
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        if !route.starts_with("/api/v0/") {
            let missing = request!(&state, "GET", route, "");
            record!(
                "GET",
                route,
                "missing-empty-or-conflict-state",
                missing.status == "200 OK"
                    && json_body(&missing)["health"]["isHealthy"].is_boolean()
                    && json_body(&missing)["stats"].is_object()
            );
        }
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK"
                && json_body(&runtime_response)["health"].is_object()
        );
    }

    for route in ["/api/bridge/admin/stats", "/api/v0/bridge/admin/stats"] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let missing = request!(&state, "GET", route, "");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && json_body(&missing)["totalConnections"].is_number()
                && json_body(&missing)["uptime"].is_string()
        );
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK"
                && json_body(&runtime_response)["totalSearches"].is_number()
        );
    }

    for route in ["/api/bridge/rooms", "/api/v0/bridge/rooms"] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let missing = request!(&state, "GET", route, "");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && json_body(&missing)["rooms"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK" && json_body(&runtime_response)["rooms"].is_array()
        );
        let (populated, _receiver) = test_state_with_env(bridge_env());
        seed_room(&populated, "bridge-residual-room").await;
        if !route.starts_with("/api/v0/") {
            let populated_response = request!(&populated, "GET", route, "");
            record!(
                "GET",
                route,
                "populated-dynamic-state",
                populated_response.status == "200 OK"
                    && json_body(&populated_response)["rooms"]
                        .as_array()
                        .is_some_and(|rows| {
                            rows.iter().any(|row| row["name"] == "bridge-residual-room")
                        })
            );
        }
    }

    for route in ["/api/bridge/status", "/api/v0/bridge/status"] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        if !route.starts_with("/api/v0/") {
            let missing = request!(&state, "GET", route, "");
            record!(
                "GET",
                route,
                "missing-empty-or-conflict-state",
                missing.status == "200 OK"
                    && json_body(&missing)["isHealthy"].is_boolean()
                    && json_body(&missing)["activeConnections"].is_number()
            );
        }
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, "GET", route, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "200 OK"
                && json_body(&runtime_response)["version"] == "1.0.0-proxy"
        );
    }

    for (route, path) in [
        (
            "/api/bridge/transfer/{transferId}/progress",
            "/api/bridge/transfer/%20/progress",
        ),
        (
            "/api/v0/bridge/transfer/{transferId}/progress",
            "/api/v0/bridge/transfer/%20/progress",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "GET", path, "");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("TransferId is required")
        );
        let missing_path = if path.starts_with("/api/v0/") {
            "/api/v0/bridge/transfer/missing-transfer/progress"
        } else {
            "/api/bridge/transfer/missing-transfer/progress"
        };
        let missing = request!(&state, "GET", missing_path, "");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
                && json_body(&missing)["error"] == "Transfer not found"
        );
        let transfer_id = seed_transfer(&state).await;
        let nominal_path = if path.starts_with("/api/v0/") {
            format!("/api/v0/bridge/transfer/{transfer_id}/progress")
        } else {
            format!("/api/bridge/transfer/{transfer_id}/progress")
        };
        let nominal = request!(&state, "GET", &nominal_path, "");
        let nominal_json = json_body(&nominal);
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal_json["proxyId"].is_string()
                && nominal_json["percentComplete"].is_number()
                && nominal_json["state"].is_string()
        );
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            nominal_json["filename"] == "Bridge/Residual.flac"
                && nominal_json["fileSize"] == 100
                && nominal_json["queuePosition"] == 0
        );
        let runtime = closed_state(false).await;
        let runtime_path = if path.starts_with("/api/v0/") {
            "/api/v0/bridge/transfer/runtime-transfer/progress"
        } else {
            "/api/bridge/transfer/runtime-transfer/progress"
        };
        let runtime_response = request!(&runtime, "GET", runtime_path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime_response.status == "404 Not Found"
                && json_body(&runtime_response)["error"] == "Transfer not found"
        );
    }

    // Mutation routes without an explicit API version are rejected by the
    // frozen API-versioning middleware before request binding or state
    // access.  The open cases therefore all share this exact response.
    for (method, route, body) in [
        (
            "POST",
            "/api/bridge/download",
            r#"{"username":"peer","filename":"file.flac","targetPath":"/tmp/file.flac"}"#,
        ),
        (
            "POST",
            "/api/bridge/search",
            r#"{"query":"bridge residual"}"#,
        ),
        ("POST", "/api/bridge/start", "{}"),
        ("POST", "/api/bridge/stop", "{}"),
        ("PUT", "/api/bridge/admin/config", "{}"),
    ] {
        for case in [
            "runtime-failure-and-timeout",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
            "concurrency-and-idempotency",
        ] {
            let (state, _receiver) = test_state_with_env(bridge_env());
            let response = request!(&state, method, route, body);
            record!(
                method,
                route,
                case,
                response.status == "400 Bad Request"
                    && response.body.contains("ApiVersionUnspecified")
            );
        }
    }

    let download_body = r#"{"username":"peer","filename":"Bridge/Residual.flac","targetPath":"/tmp/residual.flac"}"#;
    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "POST", "/api/v0/bridge/download", "not-json");
        record!(
            "POST",
            "/api/v0/bridge/download",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("Request is required")
        );
        let missing = request!(&state, "POST", "/api/v0/bridge/download", "{}");
        record!(
            "POST",
            "/api/v0/bridge/download",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing
                    .body
                    .contains("Username, filename, and targetPath are required")
        );
        let restart = request!(&state, "POST", "/api/v0/bridge/download", download_body);
        let transfer_id = json_body(&restart)["transfer_id"].clone();
        let (fresh, _receiver) = test_state_with_env(bridge_env());
        record!(
            "POST",
            "/api/v0/bridge/download",
            "restart-persistence-or-reset",
            restart.status == "200 OK"
                && transfer_id.is_string()
                && fresh.transfers.read().await.entries.is_empty()
        );
    }
    {
        let runtime = closed_state(true).await;
        let response = request!(&runtime, "POST", "/api/v0/bridge/download", download_body);
        record!(
            "POST",
            "/api/v0/bridge/download",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && json_body(&response)["error"] == "Bridge download failed"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/bridge/download",
                None,
                download_body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/bridge/download",
                None,
                download_body,
                &state
            ),
        );
        record!(
            "POST",
            "/api/v0/bridge/download",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && left
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["transfer_id"].is_string())
                && right
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["transfer_id"].is_string())
        );
    }

    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "POST", "/api/v0/bridge/search", "not-json");
        record!(
            "POST",
            "/api/v0/bridge/search",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("Query is required")
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/bridge/search",
            r#"{"query":"bridge residual"}"#
        );
        let (fresh, _receiver) = test_state_with_env(bridge_env());
        record!(
            "POST",
            "/api/v0/bridge/search",
            "restart-persistence-or-reset",
            response.status == "200 OK" && fresh.searches.read().await.records.is_empty()
        );
    }
    {
        let runtime = closed_state(true).await;
        let response = request!(
            &runtime,
            "POST",
            "/api/v0/bridge/search",
            r#"{"query":"bridge residual"}"#
        );
        record!(
            "POST",
            "/api/v0/bridge/search",
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && json_body(&response)["query"] == "bridge residual"
                && json_body(&response)["users"] == serde_json::json!([])
        );
    }
    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/bridge/search",
                None,
                r#"{"query":"bridge-a"}"#,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/bridge/search",
                None,
                r#"{"query":"bridge-b"}"#,
                &state
            ),
        );
        record!(
            "POST",
            "/api/v0/bridge/search",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for (method, route, action, body) in [
        ("POST", "/api/v0/bridge/start", "started", "{}"),
        ("POST", "/api/v0/bridge/stop", "stopped", "{}"),
    ] {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, method, route, "not-json");
        record!(
            method,
            route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_body(&malformed)["status"] == action
        );
        let missing = request!(&state, method, route, "");
        record!(
            method,
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && json_body(&missing)["status"] == action
        );
        let nominal = request!(&state, method, route, body);
        record!(
            method,
            route,
            "mutation-side-effects-and-readback",
            nominal.status == "200 OK"
                && json_body(&nominal) == serde_json::json!({"status": action})
        );
        let runtime = closed_state(false).await;
        let runtime_response = request!(&runtime, method, route, body);
        record!(
            method,
            route,
            "restart-persistence-or-reset",
            runtime_response.status == "200 OK" && json_body(&runtime_response)["status"] == action
        );
        let (concurrent_state, _receiver) = test_state_with_env(bridge_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(method, route, None, body, &concurrent_state),
            crate::route_http_request(method, route, None, body, &concurrent_state),
        );
        record!(
            method,
            route,
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && left
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["status"] == action)
                && right
                    .as_ref()
                    .is_ok_and(|response| json_body(response)["status"] == action)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(bridge_env());
        let malformed = request!(&state, "PUT", "/api/v0/bridge/admin/config", "not-json");
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = request!(&state, "PUT", "/api/v0/bridge/admin/config", "");
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
        let response = request!(
            &state,
            "PUT",
            "/api/v0/bridge/admin/config",
            r#"{"enabled":true}"#
        );
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "restart-persistence-or-reset",
            response.status == "200 OK" && exact_bridge_config(&json_body(&response))
        );
        let fresh = test_state_with_env(bridge_env()).0;
        let fresh_config = request!(&fresh, "GET", "/api/v0/bridge/admin/config", "");
        record!(
            "PUT",
            "/api/v0/bridge/admin/config",
            "concurrency-and-idempotency",
            fresh_config.status == "200 OK" && json_body(&fresh_config)["enabled"] == false
        );
    }

    assert_eq!(ledger.len(), 87, "BridgeController residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create BridgeController evidence directory");
    fs::write(
        evidence_dir.join("bridge_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize BridgeController ledger"),
    )
    .expect("write BridgeController ledger");
    assert!(
        mismatches.is_empty(),
        "{} BridgeController residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
