/// Bulk differential proof for compatibility-tail controller projections
/// that already have direct contract tests but were not yet linked to the
/// behavioral ledger: profile projection, share-grant token acknowledgement,
/// and the default-disabled relay, mesh-rendezvous, and STUN guards.
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
async fn controller_api_differential_compatibility_projection_tail() {
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

    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": false,
            "enableDht": false,
            "enableStun": false
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );

    let profile = super::route_http_request("GET", "/api/v0/profile/me", None, "", &state)
        .await
        .expect("versioned profile projection");
    let profile_json =
        serde_json::from_str::<serde_json::Value>(&profile.body).unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/profile/me",
        "nominal-status-headers-body",
        profile.status == "200 OK"
            && profile.content_type.starts_with("application/json")
            && profile_json["peerId"]
                .as_str()
                .is_some_and(|peer_id| !peer_id.is_empty())
            && profile_json["displayName"].is_string()
            && profile_json["capabilities"].is_i64()
            && profile_json["endpoints"].is_array()
    );

    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Compatibility Tail"}"#,
        &state,
    )
    .await
    .expect("compatibility-tail collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .unwrap_or(serde_json::Value::Null)["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let grant = super::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"tail-peer"}}"#),
        &state,
    )
    .await
    .expect("compatibility-tail share grant");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body)
        .unwrap_or(serde_json::Value::Null)["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let share_token = super::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{grant_id}/token"),
        None,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .expect("versioned share-grant token acknowledgement");
    let share_token_json = serde_json::from_str::<serde_json::Value>(&share_token.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "nominal-status-headers-body",
        share_token.status == "201 Created"
            && share_token_json["token"].is_string()
            && share_token_json["created"] == true
            && share_token_json["persisted"] == false
            && share_token_json["status"] == "ephemeral_compatibility_token"
    );

    let relay_download = super::route_http_request(
        "GET",
        "/api/v0/relay/controller/downloads/token",
        None,
        "",
        &state,
    )
    .await
    .expect("disabled relay controller download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "missing-empty-or-conflict-state",
        relay_download.status == "403 Forbidden"
            && relay_download.body.contains("feature is disabled")
    );

    let mesh_users = super::route_http_request(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/users",
        None,
        "",
        &state,
    )
    .await
    .expect("disabled mesh-rendezvous users");
    record!(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/users",
        "missing-empty-or-conflict-state",
        mesh_users.status == "403 Forbidden" && mesh_users.body.contains("feature is disabled")
    );

    let nat_detect = super::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &state)
        .await
        .expect("disabled mesh NAT detection");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "missing-empty-or-conflict-state",
        nat_detect.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("compatibility_projection_tail.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api compatibility-tail mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the versioned runtime-switch projections
/// already exercised by `mesh_and_signal_routes_honor_configured_runtime_
/// switches`: configured SignalSystem DTOs retain their non-default
/// fields, while mesh and DHT routes fail closed when those subsystems
/// are disabled. This records only the still-open populated/missing
/// manifest cases for the slskdN versioned aliases.
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
async fn controller_api_differential_mesh_signal_runtime_switches() {
    let target = "slskdn";
    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": false,
            "enableDht": false,
            "enableStun": false
        },
        "SignalSystem": {
            "enabled": false,
            "deduplicationCacheSize": 2048,
            "defaultTtl": "00:07:30",
            "meshChannel": {
                "enabled": false,
                "priority": 3,
                "requireActiveSession": true
            },
            "btExtensionChannel": {
                "enabled": true,
                "priority": 4,
                "requireActiveSession": false
            }
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SIGNAL_SYSTEM_ENABLED", "true")
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );
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

    let signal_config =
        super::route_http_request("GET", "/api/v0/signals/config", None, "", &state)
            .await
            .expect("versioned signal config response");
    let signal_config_json = serde_json::from_str::<serde_json::Value>(&signal_config.body)
        .unwrap_or(serde_json::Value::Null);
    let signal_config_pass = signal_config.status == "200 OK"
        && signal_config.content_type.starts_with("application/json")
        && signal_config_json["enabled"] == true
        && signal_config_json["deduplication_cache_size"] == 2_048
        && signal_config_json["default_ttl_seconds"] == 450
        && signal_config_json["mesh_channel"]["enabled"] == false
        && signal_config_json["bt_extension_channel"]["priority"] == 4;
    if !signal_config_pass {
        mismatches.push(format!(
            "{target} signal config actual: {} {} {}",
            signal_config.status, signal_config.content_type, signal_config.body
        ));
    }
    record!(
        "GET",
        "/api/v0/signals/config",
        "populated-dynamic-state",
        signal_config_pass
    );

    let signal_status =
        super::route_http_request("GET", "/api/v0/signals/status", None, "", &state)
            .await
            .expect("versioned signal status response");
    let signal_status_json = serde_json::from_str::<serde_json::Value>(&signal_status.body)
        .unwrap_or(serde_json::Value::Null);
    let signal_status_pass = signal_status.status == "200 OK"
        && signal_status.content_type.starts_with("application/json")
        && signal_status_json["enabled"] == true
        && signal_status_json["active_channels"] == serde_json::json!(["bt_extension"]);
    if !signal_status_pass {
        mismatches.push(format!(
            "{target} signal status actual: {} {} {}",
            signal_status.status, signal_status.content_type, signal_status.body
        ));
    }
    record!(
        "GET",
        "/api/v0/signals/status",
        "populated-dynamic-state",
        signal_status_pass
    );

    let mesh_stats = super::route_http_request("GET", "/api/v0/mesh/stats", None, "", &state)
        .await
        .expect("disabled versioned mesh stats response");
    record!(
        "GET",
        "/api/v0/mesh/stats",
        "missing-empty-or-conflict-state",
        mesh_stats.status == "404 Not Found"
    );

    for route in [
        "/api/v0/mesh/delta",
        "/api/v0/mesh/hello",
        "/api/v0/mesh/lookup/disabled-mesh-key",
        "/api/v0/mesh/peers",
        "/api/v0/mesh/transport",
    ] {
        let response = super::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("disabled {route}: {error}"));
        let ledger_route = if route.starts_with("/api/v0/mesh/lookup/") {
            "/api/v0/mesh/lookup/{flacKey}"
        } else {
            route
        };
        record!(
            "GET",
            ledger_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    let dht_peers = super::route_http_request("GET", "/api/v0/dht/peers", None, "", &state)
        .await
        .expect("disabled versioned DHT peers response");
    record!(
        "GET",
        "/api/v0/dht/peers",
        "missing-empty-or-conflict-state",
        dht_peers.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("mesh_signal_runtime_switches.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh-signal mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

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
async fn controller_api_differential_bridge_config_populated_projection() {
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
        super::route_http_request("GET", "/api/v0/bridge/admin/config", None, "", &state)
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
async fn controller_api_differential_bridge_clients_populated_projection() {
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
        super::route_http_request("GET", "/api/v0/bridge/admin/clients", None, "", &state)
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
async fn controller_api_differential_bridge_rooms_populated_projection() {
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

    let response = super::route_http_request("GET", "/api/v0/bridge/rooms", None, "", &state)
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

/// Bulk differential proof crediting PodCore stats routes' nominal,
/// empty-state, and populated-state cases, independently re-derived
/// from real seeded pod/member/channel-message aggregation and runtime
/// counter checks. slskdN-only (confirmed against the frozen registry).
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
async fn controller_api_differential_podcore_stats_gets() {
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

    let empty_membership =
        super::route_http_request("GET", "/api/v0/podcore/membership/stats", None, "", &state)
            .await
            .expect("empty membership stats");
    let empty_membership_json =
        serde_json::from_str::<serde_json::Value>(&empty_membership.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/membership/stats",
        "missing-empty-or-conflict-state",
        empty_membership.status == "200 OK"
            && empty_membership_json["totalMemberships"] == 0
            && empty_membership_json["activeMemberships"] == 0
            && empty_membership_json["membershipsByRole"] == serde_json::json!({})
            && empty_membership_json["membershipsByPod"] == serde_json::json!({})
    );
    let malformed_membership = super::route_http_request(
        "GET",
        "/api/v0/podcore/membership/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed membership stats query");
    let malformed_membership_json =
        serde_json::from_str::<serde_json::Value>(&malformed_membership.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/membership/stats",
        "malformed-path-query-or-body",
        malformed_membership.status == "200 OK"
            && malformed_membership_json["totalMemberships"] == 0
            && malformed_membership_json["activeMemberships"] == 0
    );

    let empty_messages =
        super::route_http_request("GET", "/api/v0/podcore/messages/stats", None, "", &state)
            .await
            .expect("empty message stats");
    let empty_messages_json =
        serde_json::from_str::<serde_json::Value>(&empty_messages.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/messages/stats",
        "missing-empty-or-conflict-state",
        empty_messages.status == "200 OK"
            && empty_messages_json["totalMessages"] == 0
            && empty_messages_json["totalSizeBytes"] == 0
            && empty_messages_json["messagesPerPod"] == serde_json::json!({})
            && empty_messages_json["messagesPerChannel"] == serde_json::json!({})
            && empty_messages_json.get("oldestMessage").is_none()
            && empty_messages_json.get("newestMessage").is_none()
    );
    let malformed_messages = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed message stats query");
    let malformed_messages_json =
        serde_json::from_str::<serde_json::Value>(&malformed_messages.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/messages/stats",
        "malformed-path-query-or-body",
        malformed_messages.status == "200 OK"
            && malformed_messages_json["totalMessages"] == 0
            && malformed_messages_json["totalSizeBytes"] == 0
    );

    let empty_routing =
        super::route_http_request("GET", "/api/v0/podcore/routing/stats", None, "", &state)
            .await
            .expect("empty routing stats");
    let empty_routing_json =
        serde_json::from_str::<serde_json::Value>(&empty_routing.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/routing/stats",
        "missing-empty-or-conflict-state",
        empty_routing.status == "200 OK"
            && empty_routing_json["totalMessagesRouted"] == 0
            && empty_routing_json["totalRoutingAttempts"] == 0
            && empty_routing_json["successfulRoutingCount"] == 0
            && empty_routing_json["failedRoutingCount"] == 0
            && empty_routing_json["activeDeduplicationItems"] == 0
            && empty_routing_json["routingStatsByPod"] == serde_json::json!({})
    );
    let malformed_routing = super::route_http_request(
        "GET",
        "/api/v0/podcore/routing/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed routing stats query");
    let malformed_routing_json =
        serde_json::from_str::<serde_json::Value>(&malformed_routing.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/routing/stats",
        "malformed-path-query-or-body",
        malformed_routing.status == "200 OK"
            && malformed_routing_json["totalMessagesRouted"] == 0
            && malformed_routing_json["totalRoutingAttempts"] == 0
    );

    let empty_verification = super::route_http_request(
        "GET",
        "/api/v0/podcore/verification/stats",
        None,
        "",
        &state,
    )
    .await
    .expect("empty verification stats");
    let empty_verification_json =
        serde_json::from_str::<serde_json::Value>(&empty_verification.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/verification/stats",
        "missing-empty-or-conflict-state",
        empty_verification.status == "200 OK"
            && empty_verification_json["totalVerifications"] == 0
            && empty_verification_json["successfulVerifications"] == 0
            && empty_verification_json["failedMembershipChecks"] == 0
            && empty_verification_json["failedSignatureChecks"] == 0
            && empty_verification_json["bannedMemberRejections"] == 0
    );
    let pod_id = "pod:stats-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Stats differential",
                "isPublic": true,
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize stats pod"),
            "owner-peer".to_owned(),
        )
        .expect("create stats pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "moderator-peer".to_owned(),
                role: "mod".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: Some("2026-01-01T00:00:00+00:00".to_owned()),
                last_seen: Some("2026-01-02T00:00:00+00:00".to_owned()),
            },
        )
        .expect("add stats member");
    {
        let mut channels = state.pod_channels.write().await;
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "owner-peer".to_owned(),
                "one".to_owned(),
                String::new(),
                1_000,
            )
            .expect("append first stats message");
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "moderator-peer".to_owned(),
                "two".to_owned(),
                String::new(),
                2_000,
            )
            .expect("append second stats message");
    }

    let membership =
        super::route_http_request("GET", "/api/v0/podcore/membership/stats", None, "", &state)
            .await
            .expect("membership stats");
    let membership_json =
        serde_json::from_str::<serde_json::Value>(&membership.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/membership/stats",
        "nominal-status-headers-body",
        membership.status == "200 OK"
    );
    record!(
        "/api/v0/podcore/membership/stats",
        "populated-dynamic-state",
        membership_json["totalMemberships"] == 2
            && membership_json["activeMemberships"] == 2
            && membership_json["membershipsByRole"]["owner"] == 1
            && membership_json["membershipsByRole"]["mod"] == 1
            && membership_json["membershipsByPod"][pod_id] == 2
    );

    let messages =
        super::route_http_request("GET", "/api/v0/podcore/messages/stats", None, "", &state)
            .await
            .expect("message stats");
    let messages_json =
        serde_json::from_str::<serde_json::Value>(&messages.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/messages/stats",
        "nominal-status-headers-body",
        messages.status == "200 OK"
    );
    record!(
        "/api/v0/podcore/messages/stats",
        "populated-dynamic-state",
        messages_json["totalMessages"] == 2
            && messages_json["totalSizeBytes"] == 400
            && messages_json["messagesPerPod"][pod_id] == 2
            && messages_json["messagesPerChannel"]["general"] == 2
            && messages_json["oldestMessage"] == "1970-01-01T00:00:01+00:00"
            && messages_json["newestMessage"] == "1970-01-01T00:00:02+00:00"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_stats_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-stats mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 3 podcore routing routes' cases,
/// independently re-derived from `podcore_routing_matches_message_
/// router_contract_and_updates_real_stats`'s real bloom-filter
/// deduplication, banned-member filtering, and validation checks.
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
async fn controller_api_differential_podcore_routing() {
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
    let pod_id = "routing-differential-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Routing differential",
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize routing pod"),
            "sender-peer".to_owned(),
        )
        .expect("create routing pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "target-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add routing target");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "banned-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: true,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add banned routing peer");

    let unseen = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/routing/seen/routing-unseen/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("check unseen routing message");
    record!(
        "GET",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "nominal-status-headers-body",
        unseen.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&unseen.body).unwrap_or_default()
                == serde_json::json!({"isSeen": false})
    );

    let missing_unseen = super::route_http_request(
        "GET",
        "/api/v0/podcore/routing/seen/routing-unseen/pod%3Amissing-routing",
        None,
        "",
        &state,
    )
    .await
    .expect("check unseen routing message in missing pod");
    record!(
        "GET",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "missing-empty-or-conflict-state",
        missing_unseen.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&missing_unseen.body).unwrap_or_default()
                == serde_json::json!({"isSeen": false})
    );

    let nominal_pod_id = "routing-nominal-pod";
    let (nominal_state, _nominal_receiver) = test_state();
    nominal_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": nominal_pod_id,
                "name": "Routing nominal differential",
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize nominal routing pod"),
            "nominal-sender".to_owned(),
        )
        .expect("create nominal routing pod");
    let nominal_route = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-nominal-1",
            "podId": nominal_pod_id,
            "channelId": "general",
            "senderPeerId": "nominal-sender",
            "body": "no eligible peers",
        })
        .to_string(),
        &nominal_state,
    )
    .await
    .expect("nominal route message");
    let nominal_route_json =
        serde_json::from_str::<serde_json::Value>(&nominal_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "nominal-status-headers-body",
        nominal_route.status == "200 OK"
            && nominal_route_json["success"] == true
            && nominal_route_json["messageId"] == "routing-nominal-1"
            && nominal_route_json["targetPeerCount"] == 0
            && nominal_route_json["successfullyRoutedCount"] == 0
            && nominal_route_json["failedRoutingCount"] == 0
    );

    let routed = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-differential-1",
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "sender-peer",
            "body": "hello",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route message");
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "runtime-failure-and-timeout",
        routed.status == "500 Internal Server Error"
            && serde_json::from_str::<serde_json::Value>(&routed.body).unwrap()["error"]
                == "Failed to route message"
    );

    let seen_after_route = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/routing/seen/routing-differential-1/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("check routed message state");
    record!(
        "GET",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "populated-dynamic-state",
        seen_after_route.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&seen_after_route.body)
                .unwrap_or_default()
                == serde_json::json!({"isSeen": true})
    );

    let direct = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        None,
        &serde_json::json!({
            "message": {
                "messageId": "routing-differential-2",
                "podId": pod_id,
                "channelId": "general",
                "senderPeerId": "sender-peer",
            },
            "targetPeerIds": [" target-peer ", "target-peer"],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route message to peers");
    let direct_json = serde_json::from_str::<serde_json::Value>(&direct.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        "nominal-status-headers-body",
        direct.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        "mutation-side-effects-and-readback",
        direct_json["success"] == false
            && direct_json["targetPeerCount"] == 1
            && direct_json["successfullyRoutedCount"] == 0
            && direct_json["failedRoutingCount"] == 1
            && direct_json["failedPeerIds"] == serde_json::json!(["target-peer"])
    );

    let stats = super::route_http_request("GET", "/api/v0/podcore/routing/stats", None, "", &state)
        .await
        .expect("routing stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/routing/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/routing/stats",
        "populated-dynamic-state",
        stats_json["totalMessagesRouted"] == 1
            && stats_json["totalRoutingAttempts"] == 1
            && stats_json["successfulRoutingCount"] == 0
            && stats_json["failedRoutingCount"] == 1
            && stats_json["activeDeduplicationItems"] == 1
            && stats_json["routingStatsByPod"][pod_id] == 1
    );

    let duplicate = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-differential-1",
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "sender-peer",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route duplicate message");
    let duplicate_json =
        serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "mutation-side-effects-and-readback",
        duplicate.status == "200 OK"
            && duplicate_json["targetPeerCount"] == 0
            && duplicate_json["errorMessage"] == "Message already routed (duplicate)"
    );

    let missing_channel = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-differential-invalid",
            "podId": pod_id,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("validate route message");
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "malformed-path-query-or-body",
        missing_channel.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_routing.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-routing mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 13 podcore maintenance-mutation
/// routes' nominal, mutation, malformed, and missing-resource cases,
/// independently re-derived from the real DHT/discovery/membership/
/// routing/messages/backfill result-contract checks. slskdN-only
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
async fn controller_api_differential_podcore_maintenance_mutations() {
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
    let pod_id = "pod:00000000000000000000000000000002";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Maintenance differential",
                "visibility": "Listed",
                "isPublic": true,
            }))
            .expect("deserialize maintenance pod fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create maintenance pod");

    for (action, fields) in [
        (
            "publish",
            vec!["success", "podId", "dhtKey", "publishedAt", "expiresAt"],
        ),
        (
            "update",
            vec!["success", "podId", "dhtKey", "publishedAt", "expiresAt"],
        ),
    ] {
        let route = format!("/api/v0/podcore/dht/{action}");
        let malformed = super::route_http_request("POST", &route, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{route} malformed: {error}"));
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = super::route_http_request(
            "POST",
            &route,
            None,
            &serde_json::json!({
                "pod": {
                    "podId": "pod:00000000000000000000000000000003",
                    "name": "Missing maintenance pod"
                }
            })
            .to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route} missing: {error}"));
        record!(
            "POST",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
        let response = super::route_http_request(
            "POST",
            &route,
            None,
            &serde_json::json!({"pod": {"podId": pod_id, "name": "Maintenance differential"}})
                .to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "POST",
            route,
            "mutation-side-effects-and-readback",
            fields.iter().all(|key| value.get(*key).is_some()) && value["podId"] == pod_id
        );
    }

    for (action, fields) in [
        (
            "register",
            vec![
                "success",
                "podId",
                "discoveryKeys",
                "registeredAt",
                "expiresAt",
            ],
        ),
        (
            "update",
            vec![
                "success",
                "podId",
                "discoveryKeys",
                "registeredAt",
                "expiresAt",
            ],
        ),
    ] {
        let route = format!("/api/v0/podcore/discovery/{action}");
        let malformed = super::route_http_request("POST", &route, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{route} malformed: {error}"));
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let response = super::route_http_request(
            "POST",
            &route,
            None,
            &serde_json::json!({
                "podId": pod_id,
                "name": "Maintenance differential",
                "visibility": "Listed",
            })
            .to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "POST",
            route,
            "mutation-side-effects-and-readback",
            fields.iter().all(|key| value.get(*key).is_some())
        );
    }

    let refresh = super::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/refresh",
        None,
        "",
        &state,
    )
    .await
    .expect("discovery refresh");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/discovery/refresh",
        "nominal-status-headers-body",
        refresh.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/discovery/refresh",
        "mutation-side-effects-and-readback",
        ["success", "podId", "wasRepublished", "nextRefresh"]
            .iter()
            .all(|key| refresh_json.get(*key).is_some())
    );

    for (route, fields) in [
        (
            "/api/v0/podcore/membership/cleanup",
            vec!["recordsCleaned", "errorsEncountered", "completedAt"],
        ),
        (
            "/api/v0/podcore/routing/cleanup",
            vec![
                "messagesCleaned",
                "messagesRetained",
                "cleanupDuration",
                "completedAt",
            ],
        ),
    ] {
        let response = super::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "POST",
            route,
            "mutation-side-effects-and-readback",
            fields.iter().all(|key| value.get(*key).is_some())
        );
    }

    let malformed_membership_cleanup = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("malformed membership cleanup body");
    let malformed_membership_cleanup_json =
        serde_json::from_str::<serde_json::Value>(&malformed_membership_cleanup.body)
            .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        "malformed-path-query-or-body",
        malformed_membership_cleanup.status == "200 OK"
            && malformed_membership_cleanup_json
                .get("recordsCleaned")
                .is_some()
            && malformed_membership_cleanup_json
                .get("errorsEncountered")
                .is_some()
    );
    let missing_membership_cleanup = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        None,
        "",
        &state,
    )
    .await
    .expect("empty membership cleanup request");
    let missing_membership_cleanup_json =
        serde_json::from_str::<serde_json::Value>(&missing_membership_cleanup.body)
            .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        "missing-empty-or-conflict-state",
        missing_membership_cleanup.status == "200 OK"
            && missing_membership_cleanup_json
                .get("recordsCleaned")
                .is_some()
            && missing_membership_cleanup_json
                .get("errorsEncountered")
                .is_some()
    );

    let (concurrent_cleanup_one, concurrent_cleanup_two) = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v0/podcore/membership/cleanup",
            None,
            "",
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/podcore/membership/cleanup",
            None,
            "",
            &state,
        )
    );
    let concurrent_cleanup_one =
        concurrent_cleanup_one.expect("first concurrent membership cleanup");
    let concurrent_cleanup_two =
        concurrent_cleanup_two.expect("second concurrent membership cleanup");
    let concurrent_cleanup_one_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_cleanup_one.body).unwrap_or_default();
    let concurrent_cleanup_two_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_cleanup_two.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        "concurrency-and-idempotency",
        concurrent_cleanup_one.status == "200 OK"
            && concurrent_cleanup_two.status == "200 OK"
            && concurrent_cleanup_one_json.get("recordsCleaned").is_some()
            && concurrent_cleanup_two_json.get("recordsCleaned").is_some()
            && concurrent_cleanup_one_json["errorsEncountered"] == 0
            && concurrent_cleanup_two_json["errorsEncountered"] == 0
    );

    let seen_route = format!("/api/v0/podcore/routing/seen/maintenance-message-1/{pod_id}");
    let seen = super::route_http_request("POST", &seen_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{seen_route}: {error}"));
    let seen_json = serde_json::from_str::<serde_json::Value>(&seen.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "nominal-status-headers-body",
        seen.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "mutation-side-effects-and-readback",
        seen_json["wasNewlyRegistered"] == true
    );

    for route in [
        "/api/v0/podcore/messages/rebuild-index",
        "/api/v0/podcore/messages/vacuum",
    ] {
        let response = super::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && response.body == "true"
        );
    }

    let cleanup = super::route_http_request(
        "DELETE",
        "/api/v0/podcore/messages/cleanup?olderThan=9999999999999",
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup pod messages");
    record!(
        "DELETE",
        "/api/v0/podcore/messages/cleanup",
        "nominal-status-headers-body",
        cleanup.status == "200 OK" && cleanup.body == "0"
    );

    let channel_cleanup = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/messages/{pod_id}/general/cleanup?olderThan=9999999999999"),
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup pod channel messages");
    record!(
        "DELETE",
        "/api/v0/podcore/messages/{podId}/{channelId}/cleanup",
        "nominal-status-headers-body",
        channel_cleanup.status == "200 OK" && channel_cleanup.body == "0"
    );

    let sync_all = super::route_http_request(
        "POST",
        "/api/v0/podcore/backfill/sync-all",
        None,
        "",
        &state,
    )
    .await
    .expect("backfill sync-all");
    record!(
        "POST",
        "/api/v0/podcore/backfill/sync-all",
        "nominal-status-headers-body",
        sync_all.status == "200 OK" && sync_all.body == "[]"
    );

    for (section, action) in [("dht", "unpublish"), ("discovery", "unregister")] {
        let route = format!("/api/v0/podcore/{section}/{action}/{pod_id}");
        let response = super::route_http_request("DELETE", &route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let route_template = match section {
            "dht" => "/api/v0/podcore/dht/unpublish/{*podId}",
            "discovery" => "/api/v0/podcore/discovery/unregister/{podId}",
            _ => unreachable!("known podcore delete section"),
        };
        record!(
            "DELETE",
            route_template,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "DELETE",
            route_template,
            "mutation-side-effects-and-readback",
            value["success"] == true
        );
        let missing_route =
            format!("/api/v0/podcore/{section}/{action}/pod:00000000000000000000000000000003");
        let missing = super::route_http_request("DELETE", &missing_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{missing_route}: {error}"));
        let missing_json =
            serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
        record!(
            "DELETE",
            route_template,
            "missing-empty-or-conflict-state",
            match section {
                "dht" => missing.status == "200 OK" && missing_json["success"] == true,
                "discovery" => missing.status == "500 Internal Server Error",
                _ => false,
            }
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_maintenance_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-maintenance mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting podcore channel CRUD contracts,
/// independently re-derived from the frozen PodChannelController status,
/// validation, system-channel, and readback behavior. slskdN-only
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
async fn controller_api_differential_podcore_channel_crud() {
    let target = "slskdn";
    let channels_route = "/api/v0/podcore/{podId}/channels";
    let channel_route = "/api/v0/podcore/{podId}/channels/{channelId}";
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "channel-owner")
            .with("SLSK_PASSWORD", "test-secret"),
    );
    let pod_id = "pod:channel-crud-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Channel CRUD differential",
                "isPublic": true,
                "channels": [
                    {"channelId": "general", "kind": 0, "name": "general"},
                    {"channelId": "extra", "kind": 0, "name": "Extra"}
                ]
            }))
            .expect("deserialize channel CRUD pod"),
            "channel-owner".to_owned(),
        )
        .expect("create channel CRUD pod");

    let empty_channels_pod_id = "pod:channel-empty-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": empty_channels_pod_id,
                "name": "Empty channel differential",
                "isPublic": true,
                "channels": [],
            }))
            .expect("deserialize empty channel pod"),
            "channel-owner".to_owned(),
        )
        .expect("create empty channel pod");
    let empty_channels = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{empty_channels_pod_id}/channels"),
        None,
        "",
        &state,
    )
    .await
    .expect("list empty pod channels");
    let empty_channels_json =
        serde_json::from_str::<serde_json::Value>(&empty_channels.body).unwrap_or_default();
    record!(
        "GET",
        channels_route,
        "missing-empty-or-conflict-state",
        empty_channels.status == "200 OK"
            && empty_channels.content_type == "application/json"
            && empty_channels_json == serde_json::json!([])
    );

    let missing_channel_id = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        r#"{"name":"Missing ID"}"#,
        &state,
    )
    .await
    .expect("create channel without ID");
    record!(
        "POST",
        channels_route,
        "malformed-path-query-or-body",
        missing_channel_id.status == "400 Bad Request"
    );

    let missing_channel_name = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        r#"{"channelId":"missing-name"}"#,
        &state,
    )
    .await
    .expect("create channel without name");
    record!(
        "POST",
        channels_route,
        "malformed-path-query-or-body",
        missing_channel_name.status == "400 Bad Request"
    );

    let created = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        r#"{"channelId":"created","name":"Created"}"#,
        &state,
    )
    .await
    .expect("create channel");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    record!(
        "POST",
        channels_route,
        "nominal-status-headers-body",
        created.status == "201 Created"
            && created.content_type == "application/json"
            && created_json["channelId"] == "created"
            && created_json["name"] == "Created"
    );
    let channels_after_create = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        "",
        &state,
    )
    .await
    .expect("list channels after create");
    let channels_after_create_json =
        serde_json::from_str::<serde_json::Value>(&channels_after_create.body).unwrap_or_default();
    record!(
        "POST",
        channels_route,
        "mutation-side-effects-and-readback",
        channels_after_create.status == "200 OK"
            && channels_after_create_json
                .as_array()
                .is_some_and(|channels| {
                    channels.iter().any(|channel| {
                        channel["channelId"] == "created" && channel["name"] == "Created"
                    })
                })
    );

    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/general"),
        None,
        "",
        &state,
    )
    .await
    .expect("get channel");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
    record!(
        "GET",
        channel_route,
        "nominal-status-headers-body",
        detail.status == "200 OK"
            && detail.content_type == "application/json"
            && detail_json["channelId"] == "general"
            && detail_json["name"] == "general"
    );
    record!(
        "GET",
        channel_route,
        "populated-dynamic-state",
        detail_json["channelId"] == "general" && detail_json["kind"] == 0
    );

    let missing_detail = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/does-not-exist"),
        None,
        "",
        &state,
    )
    .await
    .expect("get missing channel");
    record!(
        "GET",
        channel_route,
        "missing-empty-or-conflict-state",
        missing_detail.status == "404 Not Found"
    );

    let updated = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/extra"),
        None,
        r#"{"name":"Extra renamed"}"#,
        &state,
    )
    .await
    .expect("update channel");
    record!(
        "PUT",
        channel_route,
        "nominal-status-headers-body",
        updated.status == "200 OK" && updated.body.is_empty()
    );
    let updated_readback = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/extra"),
        None,
        "",
        &state,
    )
    .await
    .expect("read updated channel");
    let updated_readback_json =
        serde_json::from_str::<serde_json::Value>(&updated_readback.body).unwrap_or_default();
    record!(
        "PUT",
        channel_route,
        "mutation-side-effects-and-readback",
        updated_readback.status == "200 OK"
            && updated_readback_json["channelId"] == "extra"
            && updated_readback_json["name"] == "Extra renamed"
    );

    let update_missing_name = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/extra"),
        None,
        r#"{"channelId":"extra"}"#,
        &state,
    )
    .await
    .expect("update channel without name");
    record!(
        "PUT",
        channel_route,
        "malformed-path-query-or-body",
        update_missing_name.status == "400 Bad Request"
    );

    let update_missing_channel = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/does-not-exist"),
        None,
        r#"{"name":"Missing"}"#,
        &state,
    )
    .await
    .expect("update missing channel");
    record!(
        "PUT",
        channel_route,
        "missing-empty-or-conflict-state",
        update_missing_channel.status == "404 Not Found"
    );

    let update_system_channel = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/general"),
        None,
        r#"{"name":"General renamed"}"#,
        &state,
    )
    .await
    .expect("update system channel");
    record!(
        "PUT",
        channel_route,
        "missing-empty-or-conflict-state",
        update_system_channel.status == "400 Bad Request"
    );

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/{pod_id}/channels/created"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete channel");
    record!(
        "DELETE",
        channel_route,
        "nominal-status-headers-body",
        deleted.status == "200 OK" && deleted.body.is_empty()
    );
    let deleted_readback = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/created"),
        None,
        "",
        &state,
    )
    .await
    .expect("read deleted channel");
    record!(
        "DELETE",
        channel_route,
        "mutation-side-effects-and-readback",
        deleted_readback.status == "404 Not Found"
    );

    let delete_missing = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/{pod_id}/channels/does-not-exist"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete missing channel");
    record!(
        "DELETE",
        channel_route,
        "missing-empty-or-conflict-state",
        delete_missing.status == "404 Not Found"
    );

    let delete_system_channel = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/{pod_id}/channels/general"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete system channel");
    record!(
        "DELETE",
        channel_route,
        "missing-empty-or-conflict-state",
        delete_system_channel.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_channel_crud.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-channel mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the PodChannelController's storage failure,
/// reload, and concurrent mutation behavior. The frozen controller
/// surfaces storage exceptions as 500 responses and the pod service
/// preserves successful channel mutations across a fresh service load.
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
async fn controller_api_differential_podcore_channel_lifecycle() {
    let target = "slskdn";
    let channels_route = "/api/v0/podcore/{podId}/channels";
    let channel_route = "/api/v0/podcore/{podId}/channels/{channelId}";
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

    macro_rules! seed_pod {
        ($state:expr, $pod_id:expr, $channels:expr) => {{
            $state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                        "podId": $pod_id,
                        "name": "Channel lifecycle differential",
                        "isPublic": true,
                        "channels": $channels,
                    }))
                    .expect("deserialize channel lifecycle pod"),
                    "channel-owner".to_owned(),
                )
                .expect("create channel lifecycle pod");
        }};
    }

    let channel_env = || {
        MapEnv::default()
            .with("SLSK_USERNAME", "channel-owner")
            .with("SLSK_PASSWORD", "test-secret")
    };

    // A valid create against a missing pod is rejected by the frozen
    // controller's access check before its service lookup.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let response = super::route_http_request(
            "POST",
            "/api/v0/podcore/pod:missing-channel-lifecycle/channels",
            None,
            r#"{"channelId":"missing","name":"Missing"}"#,
            &state,
        )
        .await
        .expect("missing channel-lifecycle pod");
        record!(
            "POST",
            channels_route,
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }

    // Failed durable writes must return 500 and leave the in-memory pod
    // unchanged. A directory at the fixed pods.json path is a confined,
    // deterministic write failure in the temporary test state.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-create-runtime";
        seed_pod!(&state, pod_id, serde_json::json!([]));
        let pods_path = state.config.state_dir.join("pods.json");
        fs::remove_file(&pods_path).expect("remove channel create state file");
        fs::create_dir(&pods_path).expect("block channel create state path");
        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/podcore/{pod_id}/channels"),
            None,
            r#"{"channelId":"runtime-create","name":"Runtime create"}"#,
            &state,
        )
        .await
        .expect("channel create runtime failure");
        let unchanged = state.pods.read().await.get(pod_id).is_some_and(|pod| {
            !pod.channels
                .iter()
                .any(|channel| channel.channel_id == "runtime-create")
        });
        record!(
            "POST",
            channels_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while creating the channel")
                && unchanged
        );
        fs::remove_dir(&pods_path).expect("remove blocked channel create state path");
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-create-restart";
        seed_pod!(&state, pod_id, serde_json::json!([]));
        let created = super::route_http_request(
            "POST",
            &format!("/api/v0/podcore/{pod_id}/channels"),
            None,
            r#"{"channelId":"restart-create","name":"Restart create"}"#,
            &state,
        )
        .await
        .expect("channel create restart fixture");
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload channel create state");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            pod.channels.iter().any(|channel| {
                channel.channel_id == "restart-create" && channel.name == "Restart create"
            })
        });
        record!(
            "POST",
            channels_route,
            "restart-persistence-or-reset",
            created.status == "201 Created" && persisted
        );
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-create-concurrent";
        seed_pod!(&state, pod_id, serde_json::json!([]));
        let bodies = (0..4)
            .map(|index| {
                format!(
                    r#"{{"channelId":"concurrent-create-{index}","name":"Concurrent create {index}"}}"#
                )
            })
            .collect::<Vec<_>>();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            let path = format!("/api/v0/podcore/{pod_id}/channels");
            let body = body.clone();
            let state = Arc::clone(&state);
            async move { super::route_http_request("POST", &path, None, &body, &state).await }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent channel creates");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            (0..4).all(|index| {
                pod.channels
                    .iter()
                    .any(|channel| channel.channel_id == format!("concurrent-create-{index}"))
            })
        });
        record!(
            "POST",
            channels_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && persisted
        );
    }

    // Update failure, reload, and concurrent updates use pre-existing
    // channels so the authorization and existence checks are identical
    // to the frozen controller's path.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-update-runtime";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"runtime-update","kind":0,"name":"Before"}])
        );
        let pods_path = state.config.state_dir.join("pods.json");
        fs::remove_file(&pods_path).expect("remove channel update state file");
        fs::create_dir(&pods_path).expect("block channel update state path");
        let response = super::route_http_request(
            "PUT",
            &format!("/api/v0/podcore/{pod_id}/channels/runtime-update"),
            None,
            r#"{"name":"After"}"#,
            &state,
        )
        .await
        .expect("channel update runtime failure");
        let unchanged = state
            .pods
            .read()
            .await
            .get(pod_id)
            .and_then(|pod| {
                pod.channels
                    .iter()
                    .find(|channel| channel.channel_id == "runtime-update")
                    .map(|channel| channel.name == "Before")
            })
            .unwrap_or(false);
        record!(
            "PUT",
            channel_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while updating the channel")
                && unchanged
        );
        fs::remove_dir(&pods_path).expect("remove blocked channel update state path");
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-update-restart";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"restart-update","kind":0,"name":"Before"}])
        );
        let updated = super::route_http_request(
            "PUT",
            &format!("/api/v0/podcore/{pod_id}/channels/restart-update"),
            None,
            r#"{"name":"Restarted update"}"#,
            &state,
        )
        .await
        .expect("channel update restart fixture");
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload channel update state");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            pod.channels.iter().any(|channel| {
                channel.channel_id == "restart-update" && channel.name == "Restarted update"
            })
        });
        record!(
            "PUT",
            channel_route,
            "restart-persistence-or-reset",
            updated.status == "200 OK" && persisted
        );
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-update-concurrent";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([
                {"channelId":"concurrent-update-0","kind":0,"name":"Before 0"},
                {"channelId":"concurrent-update-1","kind":0,"name":"Before 1"},
                {"channelId":"concurrent-update-2","kind":0,"name":"Before 2"},
                {"channelId":"concurrent-update-3","kind":0,"name":"Before 3"}
            ])
        );
        let responses = futures_util::future::join_all((0..4).map(|index| {
            let path = format!("/api/v0/podcore/{pod_id}/channels/concurrent-update-{index}");
            let body = format!(r#"{{"name":"After {index}"}}"#);
            let state = Arc::clone(&state);
            async move { super::route_http_request("PUT", &path, None, &body, &state).await }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent channel updates");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            (0..4).all(|index| {
                pod.channels.iter().any(|channel| {
                    channel.channel_id == format!("concurrent-update-{index}")
                        && channel.name == format!("After {index}")
                })
            })
        });
        record!(
            "PUT",
            channel_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && persisted
        );
    }

    // Delete failure, reload, and concurrent deletion mirror the update
    // checks and ensure a failed write does not remove the live channel.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-delete-runtime";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"runtime-delete","kind":0,"name":"Keep"}])
        );
        let pods_path = state.config.state_dir.join("pods.json");
        fs::remove_file(&pods_path).expect("remove channel delete state file");
        fs::create_dir(&pods_path).expect("block channel delete state path");
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/{pod_id}/channels/runtime-delete"),
            None,
            "",
            &state,
        )
        .await
        .expect("channel delete runtime failure");
        let unchanged = state.pods.read().await.get(pod_id).is_some_and(|pod| {
            pod.channels
                .iter()
                .any(|channel| channel.channel_id == "runtime-delete")
        });
        record!(
            "DELETE",
            channel_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while deleting the channel")
                && unchanged
        );
        fs::remove_dir(&pods_path).expect("remove blocked channel delete state path");
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-delete-restart";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"restart-delete","kind":0,"name":"Delete"}])
        );
        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/{pod_id}/channels/restart-delete"),
            None,
            "",
            &state,
        )
        .await
        .expect("channel delete restart fixture");
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload channel delete state");
        let removed = loaded.get(pod_id).is_some_and(|pod| {
            !pod.channels
                .iter()
                .any(|channel| channel.channel_id == "restart-delete")
        });
        record!(
            "DELETE",
            channel_route,
            "restart-persistence-or-reset",
            deleted.status == "200 OK" && removed
        );
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-delete-concurrent";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([
                {"channelId":"concurrent-delete-0","kind":0,"name":"Delete 0"},
                {"channelId":"concurrent-delete-1","kind":0,"name":"Delete 1"},
                {"channelId":"concurrent-delete-2","kind":0,"name":"Delete 2"},
                {"channelId":"concurrent-delete-3","kind":0,"name":"Delete 3"}
            ])
        );
        let responses = futures_util::future::join_all((0..4).map(|index| {
            let path = format!("/api/v0/podcore/{pod_id}/channels/concurrent-delete-{index}");
            let state = Arc::clone(&state);
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent channel deletes");
        let removed = loaded.get(pod_id).is_some_and(|pod| {
            (0..4).all(|index| {
                !pod.channels
                    .iter()
                    .any(|channel| channel.channel_id == format!("concurrent-delete-{index}"))
            })
        });
        record!(
            "DELETE",
            channel_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && removed
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_channel_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-channel-lifecycle mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting podcore opinion/affinity empty
/// getter contracts. The selected routes are limited to shapes that are
/// exact in the frozen oracle and slskR for an empty store: arrays for
/// opinion/recommendation reads and an object for member affinities.
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
async fn controller_api_differential_podcore_opinion_empty_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [nominal]", $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "nominal-status-headers-body",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-empty-differential";
    let content_id = "content-empty-differential";

    let opinions = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty content opinions");
    let opinions_json =
        serde_json::from_str::<serde_json::Value>(&opinions.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}",
        opinions.status == "200 OK"
            && opinions.content_type == "application/json"
            && opinions_json == serde_json::json!([])
    );

    let recommendations = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/recommendations"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty opinion recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
        recommendations.status == "200 OK"
            && recommendations.content_type == "application/json"
            && recommendations_json == serde_json::json!([])
    );

    let variant = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/variant/variant-empty"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty variant opinions");
    let variant_json = serde_json::from_str::<serde_json::Value>(&variant.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
        variant.status == "200 OK"
            && variant.content_type == "application/json"
            && variant_json == serde_json::json!([])
    );

    let aggregated = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/aggregated"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty aggregated opinions");
    let aggregated_json =
        serde_json::from_str::<serde_json::Value>(&aggregated.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
        aggregated.status == "200 OK"
            && aggregated.content_type == "application/json"
            && aggregated_json["podId"] == pod_id
            && aggregated_json["contentId"] == content_id
            && aggregated_json["weightedAverageScore"] == 0.0
            && aggregated_json["unweightedAverageScore"] == 0.0
            && aggregated_json["totalOpinions"] == 0
            && aggregated_json["uniqueVariants"] == 0
            && aggregated_json["contributingMembers"] == 0
            && aggregated_json["consensusStrength"] == 0.0
            && aggregated_json["variantAggregates"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && aggregated_json["memberContributions"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
            && aggregated_json["lastUpdated"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let stats = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/stats"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty opinion statistics");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
        stats.status == "200 OK"
            && stats.content_type == "application/json"
            && stats_json["podId"] == pod_id
            && stats_json["contentId"] == content_id
            && stats_json["totalOpinions"] == 0
            && stats_json["uniqueVariants"] == 0
            && stats_json["averageScore"] == 0.0
            && stats_json["minScore"] == 0.0
            && stats_json["maxScore"] == 0.0
            && stats_json["scoreDistribution"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
            && stats_json["lastUpdated"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let affinities = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty member affinities");
    let affinities_json =
        serde_json::from_str::<serde_json::Value>(&affinities.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/members/affinity",
        affinities.status == "200 OK"
            && affinities.content_type == "application/json"
            && affinities_json == serde_json::json!({})
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_empty_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for populated PodCore opinion/affinity getter
/// contracts. The fixture contains a real stored opinion, a pod member,
/// a content-discovery variant, and a shadow-index recommendation peer;
/// the assertions therefore cover non-empty opinion, variant, aggregate,
/// statistics, recommendation, and affinity projections rather than
/// only checking status codes. slskdN-only (confirmed against the frozen
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
    feature = "bounded-controller-api-tests-2"
))]
async fn controller_api_differential_podcore_opinion_populated_gets() {
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

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-populated-differential";
    let content_id = "content-populated-differential";
    let variant_hash = "variant-populated-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Opinion populated differential",
                "isPublic": true,
            }))
            .expect("deserialize populated opinion pod"),
            "opinion-member".to_owned(),
        )
        .expect("create populated opinion pod");
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "opinion-populated-key".to_owned(),
                size: 321,
                music_brainz_id: content_id.to_owned(),
                file_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed populated opinion content");
        discovery
            .merge_shadow_records(vec![super::content_discovery::ShadowIndexRecord {
                recording_id: content_id.to_owned(),
                peer_ids: vec!["recommendation-peer".to_owned()],
                updated_at: 1,
            }])
            .expect("seed populated opinion recommendation");
    }
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            format!("pod/opinion/{pod_id}/{content_id}/opinion-populated"),
            serde_json::json!({
                "id": "opinion-populated",
                "podId": pod_id,
                "contentId": content_id,
                "variantHash": variant_hash,
                "score": 0.8,
                "note": "populated differential opinion",
                "senderPeerId": "opinion-member",
                "signature": "differential-signature",
            }),
        )
        .expect("seed populated opinion");

    let opinions = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated content opinions");
    let opinions_json =
        serde_json::from_str::<serde_json::Value>(&opinions.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}",
        opinions.status == "200 OK"
            && opinions_json.as_array().is_some_and(|rows| rows.len() == 1)
            && opinions_json[0]["variantHash"] == variant_hash
            && opinions_json[0]["score"] == 0.8
    );

    let variant = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/variant/{variant_hash}"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated variant opinions");
    let variant_json = serde_json::from_str::<serde_json::Value>(&variant.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
        variant.status == "200 OK"
            && variant_json.as_array().is_some_and(|rows| rows.len() == 1)
            && variant_json[0]["senderPeerId"] == "opinion-member"
    );

    let aggregated = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/aggregated"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated aggregated opinions");
    let aggregated_json =
        serde_json::from_str::<serde_json::Value>(&aggregated.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
        aggregated.status == "200 OK"
            && aggregated_json["totalOpinions"] == 1
            && aggregated_json["uniqueVariants"] == 1
            && aggregated_json["contributingMembers"] == 1
            && aggregated_json["weightedAverageScore"]
                .as_f64()
                .is_some_and(|value| (value - 0.8).abs() < 1e-9)
            && aggregated_json["variantAggregates"]
                .as_array()
                .is_some_and(|rows| rows.len() == 1)
            && aggregated_json["memberContributions"]
                .as_object()
                .is_some_and(|members| members.contains_key("opinion-member"))
            && aggregated_json["consensusStrength"]
                .as_f64()
                .is_some_and(|value| value > 0.0)
    );

    let recommendations = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/recommendations"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated opinion recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
        recommendations.status == "200 OK"
            && recommendations_json == serde_json::json!(["recommendation-peer"])
    );

    let stats = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/stats"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated opinion statistics");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
        stats.status == "200 OK"
            && stats_json["totalOpinions"] == 1
            && stats_json["uniqueVariants"] == 1
            && stats_json["averageScore"].as_f64() == Some(0.8)
            && stats_json["minScore"].as_f64() == Some(0.8)
            && stats_json["maxScore"].as_f64() == Some(0.8)
            && stats_json["scoreDistribution"] == serde_json::json!({"0": 1})
    );

    let affinities = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated member affinities");
    let affinities_json =
        serde_json::from_str::<serde_json::Value>(&affinities.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/members/affinity",
        affinities.status == "200 OK"
            && affinities_json["opinion-member"]["peerId"] == "opinion-member"
            && affinities_json["opinion-member"]["affinityScore"]
                .as_f64()
                .is_some_and(|value| value > 0.0)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_populated_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion-populated mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the PodCore opinion publication contract.
/// The publication response is the oracle's result DTO, and the
/// subsequent content read proves that a successful publication is stored
/// and observable through the sibling getter. slskdN-only (confirmed
/// against the frozen registry).
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
async fn controller_api_differential_podcore_opinion_publish() {
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

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-publish-differential";
    let content_id = "content-publish-differential";
    let variant_hash = "variant-publish-differential";
    let opinion_body = serde_json::json!({
        "contentId": content_id,
        "variantHash": variant_hash,
        "score": 0.9,
        "note": "published differential opinion",
        "senderPeerId": "opinion-publisher",
        "signature": format!(
            "ed25519:{}",
            base64::engine::general_purpose::STANDARD.encode([0_u8; 64])
        ),
    })
    .to_string();

    let published = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions"),
        None,
        &opinion_body,
        &state,
    )
    .await
    .expect("publish opinion");
    let published_json =
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions",
        "nominal-status-headers-body",
        published.status == "200 OK"
            && published.content_type == "application/json"
            && published_json["success"] == true
            && published_json["podId"] == pod_id
            && published_json["contentId"] == content_id
            && published_json["variantHash"] == variant_hash
            && published_json["publishedOpinion"]["contentId"] == content_id
            && published_json["publishedOpinion"]["variantHash"] == variant_hash
            && published_json["publishedOpinion"]["senderPeerId"] == "opinion-publisher"
    );

    let readback = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read published opinion");
    let readback_json =
        serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions",
        "mutation-side-effects-and-readback",
        readback.status == "200 OK"
            && readback_json.as_array().is_some_and(|rows| rows.len() == 1)
            && readback_json[0]["contentId"] == content_id
            && readback_json[0]["variantHash"] == variant_hash
            && readback_json[0]["score"] == 0.9
    );

    let missing = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("publish opinion with missing fields");
    let missing_json = serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions",
        "missing-empty-or-conflict-state",
        missing.status == "400 Bad Request"
            && missing.content_type == "application/json"
            && missing_json["error"] == "Content ID and variant hash are required"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_publish.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion-publish mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the empty-state opinion actions.
/// The frozen services return successful zero-count result records when
/// there is no cached opinion or membership state for a valid route.
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
async fn controller_api_differential_podcore_opinion_actions() {
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

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-actions-empty";

    let refresh = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/refresh"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty opinion refresh");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/refresh",
        "nominal-status-headers-body",
        refresh.status == "200 OK"
            && refresh.content_type == "application/json"
            && refresh_json["success"] == true
            && refresh_json["podId"] == pod_id
            && refresh_json["opinionsRefreshed"] == 0
            && refresh_json["newOpinions"] == 0
            && refresh_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let update = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity/update"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty affinity update");
    let update_json = serde_json::from_str::<serde_json::Value>(&update.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/members/affinity/update",
        "nominal-status-headers-body",
        update.status == "200 OK"
            && update.content_type == "application/json"
            && update_json["success"] == true
            && update_json["podId"] == pod_id
            && update_json["membersUpdated"] == 0
            && update_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let (populated_state, _receiver) = test_state();
    let populated_pod_id = "pod:opinion-actions-populated";
    populated_state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            format!("pod/opinion/{populated_pod_id}/content-actions/opinion-refresh"),
            serde_json::json!({
                "id": "opinion-refresh",
                "podId": populated_pod_id,
                "contentId": "content-actions",
                "variantHash": "variant-actions",
                "score": 0.7,
                "senderPeerId": "affinity-member",
                "signature": "action-differential-signature",
            }),
        )
        .expect("seed populated opinion action");
    let refreshed = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{populated_pod_id}/opinions/refresh"),
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated opinion refresh");
    let refreshed_json =
        serde_json::from_str::<serde_json::Value>(&refreshed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/refresh",
        "mutation-side-effects-and-readback",
        refreshed.status == "200 OK"
            && refreshed.content_type == "application/json"
            && refreshed_json["success"] == true
            && refreshed_json["podId"] == populated_pod_id
            && refreshed_json["opinionsRefreshed"] == 1
            && refreshed_json["newOpinions"] == 1
            && refreshed_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    populated_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": populated_pod_id,
                "name": "Opinion action differential",
                "isPublic": true,
            }))
            .expect("deserialize populated opinion action pod"),
            "affinity-member".to_owned(),
        )
        .expect("create populated opinion action pod");
    let populated_update = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{populated_pod_id}/opinions/members/affinity/update"),
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated affinity update");
    let populated_update_json =
        serde_json::from_str::<serde_json::Value>(&populated_update.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/members/affinity/update",
        "mutation-side-effects-and-readback",
        populated_update.status == "200 OK"
            && populated_update.content_type == "application/json"
            && populated_update_json["success"] == true
            && populated_update_json["podId"] == populated_pod_id
            && populated_update_json["membersUpdated"] == 1
            && populated_update_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_actions.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion action mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting PodCore opinion routes when the pod
/// has no cached opinion or membership state. The frozen services expose
/// successful zero/empty DTOs for these valid routes rather than a 404.
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
async fn controller_api_differential_podcore_opinion_missing_gets_and_actions() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} {} {} [missing-empty-or-conflict-state]",
                    $method,
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "missing-empty-or-conflict-state",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-missing-differential";
    let content_id = "content-missing-differential";

    let opinions = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state content opinions");
    let opinions_json =
        serde_json::from_str::<serde_json::Value>(&opinions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}",
        opinions.status == "200 OK" && opinions_json == serde_json::json!([])
    );

    let aggregated = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/aggregated"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state aggregated opinions");
    let aggregated_json =
        serde_json::from_str::<serde_json::Value>(&aggregated.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
        aggregated.status == "200 OK"
            && aggregated_json["podId"] == pod_id
            && aggregated_json["contentId"] == content_id
            && aggregated_json["totalOpinions"] == 0
            && aggregated_json["variantAggregates"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && aggregated_json["memberContributions"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
    );

    let recommendations = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/recommendations"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state opinion recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
        recommendations.status == "200 OK" && recommendations_json == serde_json::json!([])
    );

    let stats = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/stats"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state opinion statistics");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
        stats.status == "200 OK"
            && stats_json["podId"] == pod_id
            && stats_json["contentId"] == content_id
            && stats_json["totalOpinions"] == 0
            && stats_json["uniqueVariants"] == 0
            && stats_json["scoreDistribution"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
    );

    let variant = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/variant/variant-missing"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state variant opinions");
    let variant_json = serde_json::from_str::<serde_json::Value>(&variant.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
        variant.status == "200 OK" && variant_json == serde_json::json!([])
    );

    let refresh = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/refresh"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state opinion refresh");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/refresh",
        refresh.status == "200 OK"
            && refresh_json["success"] == true
            && refresh_json["podId"] == pod_id
            && refresh_json["opinionsRefreshed"] == 0
            && refresh_json["newOpinions"] == 0
    );

    let update = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity/update"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state affinity update");
    let update_json = serde_json::from_str::<serde_json::Value>(&update.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/members/affinity/update",
        update.status == "200 OK"
            && update_json["success"] == true
            && update_json["podId"] == pod_id
            && update_json["membersUpdated"] == 0
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_missing_gets_and_actions.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion missing-state mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof that PodCore message-signature verification updates
/// its observable counters while preserving the oracle's boolean result
/// DTO for a legacy signature in the default non-enforced mode. slskdN-only
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
async fn controller_api_differential_podcore_signing_verify() {
    let target = "slskdn";
    let route = "/api/v0/podcore/signing/verify";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} POST {route} [mutation-side-effects-and-readback]"
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": route,
                "case": "mutation-side-effects-and-readback",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let stats_before =
        super::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
            .await
            .expect("read signing stats before verification");
    let stats_before_json =
        serde_json::from_str::<serde_json::Value>(&stats_before.body).unwrap_or_default();
    let verified = super::route_http_request(
        "POST",
        route,
        None,
        r#"{"messageId":"signing-differential","podId":"pod:signing-differential","channelId":"general","senderPeerId":"signing-peer","body":"signing differential","timestampUnixMs":1,"signature":"legacy","sigVersion":1}"#,
        &state,
    )
    .await
    .expect("verify signing differential message");
    let verified_json =
        serde_json::from_str::<serde_json::Value>(&verified.body).unwrap_or_default();
    let stats_after =
        super::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
            .await
            .expect("read signing stats after verification");
    let stats_after_json =
        serde_json::from_str::<serde_json::Value>(&stats_after.body).unwrap_or_default();
    let before_count = stats_before_json["totalSignaturesVerified"]
        .as_u64()
        .unwrap_or(0);
    let after_count = stats_after_json["totalSignaturesVerified"]
        .as_u64()
        .unwrap_or(0);
    let before_successes = stats_before_json["successfulVerifications"]
        .as_u64()
        .unwrap_or(0);
    let after_successes = stats_after_json["successfulVerifications"]
        .as_u64()
        .unwrap_or(0);
    record!(
        verified.status == "200 OK"
            && verified.content_type == "application/json"
            && verified_json == serde_json::json!({"isValid": true})
            && stats_after.status == "200 OK"
            && after_count == before_count + 1
            && after_successes == before_successes + 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_signing_verify.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-signing mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the share-grants CRUD routes'
/// cases, independently re-derived from `v0_share_grants_get_real_
/// uuid_ids_usable_on_versioned_routes`'s real UUID-id and full
/// create/read/update/delete lifecycle checks. slskdN-only (confirmed
/// against the frozen registry: absent from the slskd policy file).
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
async fn controller_api_differential_share_grants_crud() {
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
    let empty_list = super::route_http_request("GET", "/api/v0/share-grants", None, "", &state)
        .await
        .expect("empty share grants list");
    record!(
        "GET",
        "/api/v0/share-grants",
        "missing-empty-or-conflict-state",
        empty_list.status == "200 OK" && empty_list.body == "[]"
    );
    let malformed_create =
        super::route_http_request("POST", "/api/v0/share-grants", None, "{}", &state)
            .await
            .expect("malformed share grant create");
    record!(
        "POST",
        "/api/v0/share-grants",
        "malformed-path-query-or-body",
        malformed_create.status == "409 Conflict"
    );
    let missing_create = super::route_http_request(
        "POST",
        "/api/v0/share-grants",
        None,
        r#"{"collection_id":"missing-collection","username":"friend"}"#,
        &state,
    )
    .await
    .expect("missing share grant collection");
    record!(
        "POST",
        "/api/v0/share-grants",
        "missing-empty-or-conflict-state",
        missing_create.status == "404 Not Found"
    );
    let collection = super::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Share Grant Differential"}"#,
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
    let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default()
        ["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/share-grants",
        "nominal-status-headers-body",
        created.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v0/share-grants",
        "mutation-side-effects-and-readback",
        uuid::Uuid::parse_str(&grant_id).is_ok()
    );

    let populated_list = super::route_http_request("GET", "/api/v0/share-grants", None, "", &state)
        .await
        .expect("populated share grants list");
    let populated_list_json =
        serde_json::from_str::<serde_json::Value>(&populated_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/share-grants",
        "populated-dynamic-state",
        populated_list.status == "200 OK"
            && populated_list_json
                .as_array()
                .is_some_and(|records| { records.iter().any(|record| record["id"] == grant_id) })
    );

    let get_route = format!("/api/v0/share-grants/{grant_id}");
    let get = super::route_http_request("GET", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    let get_json = serde_json::from_str::<serde_json::Value>(&get.body).unwrap_or_default();
    let malformed_get = super::route_http_request("GET", "/api/v0/share-grants/", None, "", &state)
        .await
        .expect("malformed share grant get");
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "malformed-path-query-or-body",
        malformed_get.status == "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/share-grants",
        "malformed-path-query-or-body",
        malformed_get.status == "400 Bad Request"
    );
    let missing_get = super::route_http_request(
        "GET",
        "/api/v0/share-grants/00000000-0000-0000-0000-000000000000",
        None,
        "",
        &state,
    )
    .await
    .expect("missing share grant get");
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        missing_get.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "nominal-status-headers-body",
        get.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "populated-dynamic-state",
        get_json["id"] == grant_id
            && get_json["collection_id"] == collection_id
            && get_json["username"] == "friend"
            && get_json["permissions"].is_string()
    );

    let malformed_update =
        super::route_http_request("PUT", "/api/v0/share-grants/", None, "{}", &state)
            .await
            .expect("malformed share grant update");
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "malformed-path-query-or-body",
        malformed_update.status == "404 Not Found"
    );

    let update = super::route_http_request(
        "PUT",
        &get_route,
        None,
        r#"{"permissions":"read,download"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "nominal-status-headers-body",
        update.status == "200 OK"
    );

    let readback = super::route_http_request("GET", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    let readback_json =
        serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "mutation-side-effects-and-readback",
        readback_json["permissions"] == "read,download"
    );

    let missing_update = super::route_http_request(
        "PUT",
        "/api/v0/share-grants/00000000-0000-0000-0000-000000000000",
        None,
        r#"{"permissions":"read"}"#,
        &state,
    )
    .await
    .expect("missing share grant update");
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        missing_update.status == "404 Not Found"
    );

    let malformed_delete =
        super::route_http_request("DELETE", "/api/v0/share-grants/", None, "", &state)
            .await
            .expect("malformed share grant delete");
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );

    let delete = super::route_http_request("DELETE", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "nominal-status-headers-body",
        delete.status == "200 OK"
    );

    let after_delete = super::route_http_request("GET", &get_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{get_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "mutation-side-effects-and-readback",
        after_delete.status == "404 Not Found"
    );

    let missing_delete = super::route_http_request(
        "DELETE",
        "/api/v0/share-grants/00000000-0000-0000-0000-000000000000",
        None,
        "",
        &state,
    )
    .await
    .expect("missing share grant delete");
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        missing_delete.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("share_grants_crud.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api share-grants-crud mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-2"
))]
async fn controller_api_differential_share_grants_persistence_and_concurrency() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    let persistence_env = || {
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target)
    };

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

    async fn create_share_grant_collection(state: &super::AppState, title: &str) -> String {
        let body = serde_json::json!({"title": title}).to_string();
        let response = super::route_http_request("POST", "/api/v0/collections", None, &body, state)
            .await
            .expect("persist share-grant collection");
        assert_eq!(response.status, "201 Created", "{}", response.body);
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap()["id"]
            .as_str()
            .expect("collection id")
            .to_owned()
    }

    // Versioned grant creation rolls back when persistence fails.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant create runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id = create_share_grant_collection(&state, "Runtime Grant Collection").await;
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"runtime"}}"#),
            &state,
        )
        .await
        .unwrap();
        let pass = response.status == "503 Service Unavailable"
            && state.share_grants.read().await.records.is_empty();
        record!(
            "POST",
            "/api/v0/share-grants",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Versioned grant creation survives rebuilding the grant store.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id = create_share_grant_collection(&state, "Restart Grant Collection").await;
        let created = super::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"restart"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let persisted = db.list_share_grants(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.share_grants.write().await =
            super::ShareGrantStore::from_persisted(persisted.clone());
        let fetched = super::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "POST",
            "/api/v0/share-grants",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && persisted.len() == 1
                && fetched.status == "200 OK"
                && fetched_json["id"] == grant_id
                && fetched_json["username"] == "restart"
        );
    }

    // Distinct versioned grants can be created concurrently and all persist.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Concurrent Grant Collection").await;
        let bodies: Vec<String> = (0..4)
            .map(|index| {
                format!(r#"{{"collection_id":"{collection_id}","username":"concurrent-{index}"}}"#)
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            super::route_http_request("POST", "/api/v0/share-grants", None, body, &state)
        }))
        .await;
        let persisted = db.list_share_grants(10, 0).await.unwrap_or_default();
        let usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|grant| grant.username.clone())
            .collect();
        let expected: std::collections::BTreeSet<String> =
            (0..4).map(|index| format!("concurrent-{index}")).collect();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
        }) && persisted.len() == 4
            && usernames == expected;
        record!(
            "POST",
            "/api/v0/share-grants",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Versioned grant updates restore the prior state when persistence fails.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant update runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Runtime Update Grant Collection").await;
        let created = super::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"update-runtime"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        db.close_for_test().await;
        let response = super::route_http_request(
            "PUT",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            r#"{"permissions":"read,download"}"#,
            &state,
        )
        .await
        .unwrap();
        let grant = state.share_grants.read().await.get(&grant_id);
        let pass = response.status == "503 Service Unavailable"
            && grant.is_some_and(|grant| grant.permissions == "download,stream");
        record!(
            "PUT",
            "/api/v0/share-grants/{id}",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Versioned grant updates survive rebuilding the grant store.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant update restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Restart Update Grant Collection").await;
        let created = super::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"update-restart"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let updated = super::route_http_request(
            "PUT",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            r#"{"permissions":"read,download"}"#,
            &state,
        )
        .await
        .unwrap();
        let persisted = db.list_share_grants(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.share_grants.write().await =
            super::ShareGrantStore::from_persisted(persisted.clone());
        let fetched = super::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "PUT",
            "/api/v0/share-grants/{id}",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && persisted.len() == 1
                && fetched.status == "200 OK"
                && fetched_json["permissions"] == "read,download"
        );
    }

    // Distinct versioned grant updates complete concurrently and persist.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant update concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Concurrent Update Grant Collection").await;
        let mut grant_ids = Vec::new();
        for index in 0..4 {
            let created = super::route_http_request(
                "POST",
                "/api/v0/share-grants",
                None,
                &format!(r#"{{"collection_id":"{collection_id}","username":"update-{index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            grant_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(grant_ids.iter().enumerate().map(
            |(index, grant_id)| {
                let path = format!("/api/v0/share-grants/{grant_id}");
                let body = format!(r#"{{"permissions":"read,slot-{index}"}}"#);
                let state = Arc::clone(&state);
                async move { super::route_http_request("PUT", &path, None, &body, &state).await }
            },
        ))
        .await;
        let persisted = db.list_share_grants(10, 0).await.unwrap_or_default();
        let permissions: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|grant| grant.permissions.clone())
            .collect();
        let expected: std::collections::BTreeSet<String> =
            (0..4).map(|index| format!("read,slot-{index}")).collect();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.len() == 4
            && permissions == expected;
        record!(
            "PUT",
            "/api/v0/share-grants/{id}",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Versioned grant deletion survives rebuilding the grant store.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Restart Delete Grant Collection").await;
        let created = super::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"delete-restart"}}"#),
            &state,
        )
        .await
        .unwrap();
        let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let persisted = db.list_share_grants(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.share_grants.write().await =
            super::ShareGrantStore::from_persisted(persisted.clone());
        let fetched = super::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{grant_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/share-grants/{id}",
            "restart-persistence-or-reset",
            deleted.status == "200 OK" && persisted.is_empty() && fetched.status == "404 Not Found"
        );
    }

    // Distinct versioned grant deletions complete concurrently.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grant delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id =
            create_share_grant_collection(&state, "Concurrent Delete Grant Collection").await;
        let mut grant_ids = Vec::new();
        for index in 0..4 {
            let created = super::route_http_request(
                "POST",
                "/api/v0/share-grants",
                None,
                &format!(r#"{{"collection_id":"{collection_id}","username":"delete-{index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            grant_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(grant_ids.iter().map(|grant_id| {
            let path = format!("/api/v0/share-grants/{grant_id}");
            let state = Arc::clone(&state);
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let persisted = db.list_share_grants(10, 0).await.unwrap_or_default();
        let in_memory_empty = state.share_grants.read().await.records.is_empty();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.is_empty()
            && in_memory_empty;
        record!(
            "DELETE",
            "/api/v0/share-grants/{id}",
            "concurrency-and-idempotency",
            pass
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("share_grants_persistence_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api share-grant persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 14 mediacore routes' cases,
/// independently re-derived from `mediacore_mutations_match_native_
/// validation_and_result_dtos`'s real fuzzy-match/perceptual-hash/
/// portability/retrieval/stats contract checks. slskdN-only (confirmed
/// against the frozen registry).
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
async fn controller_api_differential_mediacore_mutations() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} POST {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    macro_rules! record_get {
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

    let fuzzy_text = super::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/text",
        None,
        r#"{"textA":"same","textB":"same"}"#,
        &state,
    )
    .await
    .expect("fuzzy text match");
    let fuzzy_text_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_text.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/fuzzymatch/text",
        "nominal-status-headers-body",
        fuzzy_text.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/fuzzymatch/text",
        "mutation-side-effects-and-readback",
        fuzzy_text_json["levenshteinSimilarity"] == 1.0
            && fuzzy_text_json["phoneticSimilarity"] == 1.0
            && fuzzy_text_json["combinedSimilarity"] == 1.0
    );

    let fuzzy_perceptual = super::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/perceptual",
        None,
        r#"{"contentIdA":"content:music:recording:a","contentIdB":"content:music:recording:b"}"#,
        &state,
    )
    .await
    .expect("fuzzy perceptual match");
    let fuzzy_perceptual_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_perceptual.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/fuzzymatch/perceptual",
        "nominal-status-headers-body",
        fuzzy_perceptual.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/fuzzymatch/perceptual",
        "mutation-side-effects-and-readback",
        fuzzy_perceptual_json["similarity"] == 0.0
            && fuzzy_perceptual_json["isSimilar"] == false
            && fuzzy_perceptual_json["threshold"] == 0.7
    );

    for (route, body) in [
        (
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[0.5],"sampleRate":1,"algorithm":"PHash"}"#,
        ),
        (
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":1,"height":1,"algorithm":"PHash"}"#,
        ),
    ] {
        let response = super::route_http_request("POST", route, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            route,
            "mutation-side-effects-and-readback",
            value["algorithm"] == "PHash"
                && value["hex"] == "0000000000000000"
                && value["numericHash"] == 0
        );
    }

    let similarity = super::route_http_request(
        "POST",
        "/api/v0/mediacore/perceptualhash/similarity",
        None,
        r#"{"hashA":"0000000000000000","hashB":"ffffffffffffffff","threshold":0.8}"#,
        &state,
    )
    .await
    .expect("perceptual hash similarity");
    let similarity_json =
        serde_json::from_str::<serde_json::Value>(&similarity.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/perceptualhash/similarity",
        "nominal-status-headers-body",
        similarity.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/perceptualhash/similarity",
        "mutation-side-effects-and-readback",
        similarity_json["hammingDistance"] == 64
            && similarity_json["similarity"] == 0.0
            && similarity_json["areSimilar"] == false
    );

    for (route, body) in [
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
        (
            "/api/v0/mediacore/fuzzymatch/find/content:music:recording:missing",
            r#"{"minConfidence":2}"#,
        ),
        ("/api/v0/mediacore/fuzzymatch/text", r#"{"textA":"same"}"#),
        (
            "/api/v0/mediacore/fuzzymatch/perceptual",
            r#"{"contentIdA":"content:music:recording:a"}"#,
        ),
        (
            "/api/v0/mediacore/perceptualhash/similarity",
            r#"{"hashA":"not-hex","hashB":"0000000000000000"}"#,
        ),
        ("/api/v0/mediacore/perceptualhash/audio", r#"{}"#),
        (
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":0,"height":1}"#,
        ),
        ("/api/v0/mediacore/portability/analyze", r#"{}"#),
        ("/api/v0/mediacore/portability/import", r#"{}"#),
        ("/api/v0/mediacore/retrieve/verify", r#"{}"#),
    ] {
        let response = super::route_http_request("POST", route, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let evidence_route = if route == "/api/v0/mediacore/ipld/links/content:test:type:id" {
            "/api/v0/mediacore/ipld/links/{*contentId}"
        } else if route.starts_with("/api/v0/mediacore/fuzzymatch/find/") {
            "/api/v0/mediacore/fuzzymatch/find/{*contentId}"
        } else {
            route
        };
        record!(
            evidence_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
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
    .expect("portability analyze");
    record!(
        "/api/v0/mediacore/portability/analyze",
        "nominal-status-headers-body",
        analysis.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/portability/analyze",
        "mutation-side-effects-and-readback",
        analysis.body
            == r#"{"cleanEntries":0,"conflictingEntries":0,"conflicts":[],"recommendedStrategies":{"Merge":0,"Overwrite":0,"Skip":0},"totalEntries":0}"#
    );

    let imported = super::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/import",
        None,
        empty_package,
        &state,
    )
    .await
    .expect("portability import");
    let imported_json =
        serde_json::from_str::<serde_json::Value>(&imported.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/portability/import",
        "nominal-status-headers-body",
        imported.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/portability/import",
        "mutation-side-effects-and-readback",
        [
            "success",
            "entriesProcessed",
            "entriesImported",
            "entriesSkipped",
            "conflictsResolved",
            "conflicts",
            "errors",
            "duration",
        ]
        .iter()
        .all(|key| imported_json.get(*key).is_some())
    );

    let fuzzy_stats =
        super::route_http_request("GET", "/api/v0/mediacore/stats/fuzzy", None, "", &state)
            .await
            .expect("fuzzy stats");
    let fuzzy_stats_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_stats.body).unwrap_or_default();
    record_get!(
        "/api/v0/mediacore/stats/fuzzy",
        "populated-dynamic-state",
        fuzzy_stats_json["totalMatches"] == 0
            && fuzzy_stats_json["successfulMatches"] == 0
            && fuzzy_stats_json["successRate"] == 0.0
    );

    let perceptual_stats = super::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/perceptual",
        None,
        "",
        &state,
    )
    .await
    .expect("perceptual stats");
    let perceptual_stats_json =
        serde_json::from_str::<serde_json::Value>(&perceptual_stats.body).unwrap_or_default();
    record_get!(
        "/api/v0/mediacore/stats/perceptual",
        "populated-dynamic-state",
        perceptual_stats_json["totalHashesComputed"] == 0
            && perceptual_stats_json["duplicateHashesDetected"] == 0
    );

    let portability_stats = super::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/portability",
        None,
        "",
        &state,
    )
    .await
    .expect("portability stats");
    let portability_stats_json =
        serde_json::from_str::<serde_json::Value>(&portability_stats.body).unwrap_or_default();
    record_get!(
        "/api/v0/mediacore/stats/portability",
        "populated-dynamic-state",
        portability_stats_json["totalExports"] == 0
            && portability_stats_json["totalImports"] == 0
            && portability_stats_json["successfulImports"] == 0
            && portability_stats_json["importSuccessRate"] == 0.0
    );

    let verify = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/verify",
        None,
        r#"{"descriptor":{"contentId":"content:test:type:id","hashes":[]}}"#,
        &state,
    )
    .await
    .expect("retrieve verify");
    let verify_json = serde_json::from_str::<serde_json::Value>(&verify.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/retrieve/verify",
        "nominal-status-headers-body",
        verify.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/retrieve/verify",
        "mutation-side-effects-and-readback",
        verify_json["isValid"] == false
            && verify_json["signatureValid"] == false
            && verify_json["freshnessValid"] == false
            && verify_json["validationError"].as_str().is_some()
    );

    let cache = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "",
        &state,
    )
    .await
    .expect("retrieve cache clear");
    record!(
        "/api/v0/mediacore/retrieve/cache/clear",
        "nominal-status-headers-body",
        cache.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/retrieve/cache/clear",
        "mutation-side-effects-and-readback",
        cache.body == r#"{"bytesFreed":0,"entriesCleared":0,"success":true}"#
    );

    let reset =
        super::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .expect("stats reset");
    record!(
        "/api/v0/mediacore/stats/reset",
        "nominal-status-headers-body",
        reset.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/stats/reset",
        "mutation-side-effects-and-readback",
        reset.body == r#"{"message":"Statistics reset successfully"}"#
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mediacore mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting the remaining deterministic MediaCore
/// request-validation and static-list projections. The frozen controllers
/// reject missing request fields with 400 and return non-empty supported
/// algorithm/strategy lists. slskdN-only (confirmed against the frozen
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
    feature = "bounded-controller-api-tests-2"
))]
async fn controller_api_differential_mediacore_validation_tail() {
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
    let invalid_requests = [
        (
            "POST",
            "/api/v0/mediacore/contentid/register",
            "/api/v0/mediacore/contentid/register",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/ipld/links/content:audio:track:validation",
            "/api/v0/mediacore/ipld/links/{*contentId}",
            r#"{"links":[]}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/perceptual",
            "/api/v0/mediacore/fuzzymatch/perceptual",
            r#"{"contentIdA":"","contentIdB":"content:audio:track:b"}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/text",
            "/api/v0/mediacore/fuzzymatch/text",
            r#"{"textA":"","textB":"same"}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/audio",
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[],"sampleRate":1}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/image",
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"","width":1,"height":1}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/similarity",
            "/api/v0/mediacore/perceptualhash/similarity",
            r#"{"hashA":"","hashB":"0000000000000000"}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/export",
            "/api/v0/mediacore/portability/export",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/analyze",
            "/api/v0/mediacore/portability/analyze",
            r#"{"package":null}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/import",
            "/api/v0/mediacore/portability/import",
            r#"{"package":null}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/descriptor",
            "/api/v0/mediacore/publish/descriptor",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/batch",
            "/api/v0/mediacore/publish/batch",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/republish",
            "/api/v0/mediacore/publish/republish",
            r#"{"contentIds":[]}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/batch",
            "/api/v0/mediacore/retrieve/batch",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/verify",
            "/api/v0/mediacore/retrieve/verify",
            r#"{"descriptor":null}"#,
        ),
    ];

    for (method, path, route, body) in invalid_requests {
        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }

    let fuzzy_find = super::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/find/content:audio:track:empty-request",
        None,
        "{}",
        &state,
    )
    .await
    .expect("fuzzy find empty request");
    let fuzzy_find_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_find.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/fuzzymatch/find/{*contentId}",
        "missing-empty-or-conflict-state",
        fuzzy_find.status == "200 OK"
            && fuzzy_find_json["targetContentId"] == "content:audio:track:empty-request"
            && fuzzy_find_json["searchParameters"]["minConfidence"] == 0.7
    );

    let cache_clear_malformed = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("cache clear malformed body");
    let cache_clear_malformed_json =
        serde_json::from_str::<serde_json::Value>(&cache_clear_malformed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        "malformed-path-query-or-body",
        cache_clear_malformed.status == "200 OK" && cache_clear_malformed_json["success"] == true
    );

    let cache_clear_empty = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "",
        &state,
    )
    .await
    .expect("cache clear empty request");
    let cache_clear_empty_json =
        serde_json::from_str::<serde_json::Value>(&cache_clear_empty.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        "missing-empty-or-conflict-state",
        cache_clear_empty.status == "200 OK" && cache_clear_empty_json["success"] == true
    );

    let stats_reset_malformed = super::route_http_request(
        "POST",
        "/api/v0/mediacore/stats/reset",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("stats reset malformed body");
    record!(
        "POST",
        "/api/v0/mediacore/stats/reset",
        "malformed-path-query-or-body",
        stats_reset_malformed.status == "200 OK"
            && stats_reset_malformed
                .body
                .contains("Statistics reset successfully")
    );

    let stats_reset_empty =
        super::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .expect("stats reset empty request");
    record!(
        "POST",
        "/api/v0/mediacore/stats/reset",
        "missing-empty-or-conflict-state",
        stats_reset_empty.status == "200 OK"
            && stats_reset_empty
                .body
                .contains("Statistics reset successfully")
    );

    let batch_malformed = super::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/batch",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("publish batch malformed body");
    record!(
        "POST",
        "/api/v0/mediacore/publish/batch",
        "malformed-path-query-or-body",
        batch_malformed.status == "400 Bad Request"
    );

    let retrieve_batch_malformed = super::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("retrieve batch malformed body");
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        "malformed-path-query-or-body",
        retrieve_batch_malformed.status == "400 Bad Request"
    );

    let update_malformed = super::route_http_request(
        "PUT",
        "/api/v0/mediacore/publish/descriptor/content:audio:track:missing-updates",
        None,
        "{}",
        &state,
    )
    .await
    .expect("descriptor update malformed body");
    record!(
        "PUT",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "malformed-path-query-or-body",
        update_malformed.status == "400 Bad Request"
    );

    let delete_empty_path = super::route_http_request(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/",
        None,
        "",
        &state,
    )
    .await
    .expect("descriptor delete empty path");
    record!(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "malformed-path-query-or-body",
        delete_empty_path.status == "400 Bad Request"
    );

    let static_lists = [
        (
            "/api/v0/mediacore/perceptualhash/algorithms",
            "/api/v0/mediacore/perceptualhash/algorithms",
            "algorithms",
        ),
        (
            "/api/v0/mediacore/portability/merge-strategies",
            "/api/v0/mediacore/portability/merge-strategies",
            "strategies",
        ),
        (
            "/api/v0/mediacore/portability/strategies",
            "/api/v0/mediacore/portability/strategies",
            "strategies",
        ),
    ];
    for (path, route, key) in static_lists {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && value[key]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_validation_tail.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api MediaCore validation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
