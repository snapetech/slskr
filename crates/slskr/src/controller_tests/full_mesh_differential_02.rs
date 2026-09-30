//! Controller full mesh differential 02 ownership.

use super::*;

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
pub(super) async fn controller_api_differential_mesh_signal_runtime_switches() {
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
        crate::route_http_request("GET", "/api/v0/signals/config", None, "", &state)
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
        crate::route_http_request("GET", "/api/v0/signals/status", None, "", &state)
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

    let mesh_stats = crate::route_http_request("GET", "/api/v0/mesh/stats", None, "", &state)
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
        let response = crate::route_http_request("GET", route, None, "", &state)
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

    let dht_peers = crate::route_http_request("GET", "/api/v0/dht/peers", None, "", &state)
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

/// Bulk differential proof for the real slskdN mesh HTTP gateway
/// proxy route (`POST /mesh/http/{serviceName}/{method}`): a
/// disallowed service is rejected before any dispatch is attempted
/// (real config-driven allowlist, not a hardcoded set), a
/// permitted-but-unrouted service reports the same real "no
/// providers" state the gateway would report to a genuine caller,
/// and a real local `private_gateway::Gateway` instance is used to
/// prove the nominal dispatch path actually reaches a real service
/// handler rather than a stub -- independently re-derived from
/// `mesh_gateway_enabled_enforces_allowlist_and_provider_discovery`
/// and `mesh_gateway_enabled_dispatches_to_real_local_service_
/// handlers` with fresh fixture data. Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before
/// writing: this route had no prior case credited. slskdN-only
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
pub(super) async fn controller_api_differential_mesh_http_gateway() {
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
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods"),
    );

    let disallowed = crate::route_http_request(
        "POST",
        "/mesh/http/differential-service/List",
        None,
        "{}",
        &state,
    )
    .await
    .expect("disallowed service response");
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "missing-empty-or-conflict-state",
        disallowed.status == "403 Forbidden"
            && serde_json::from_str::<serde_json::Value>(&disallowed.body).unwrap_or_default()
                ["error"]
                == "service_not_allowed"
    );

    let no_provider = crate::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &state)
        .await
        .expect("no-provider response");
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "runtime-failure-and-timeout",
        no_provider.status == "503 Service Unavailable"
            && serde_json::from_str::<serde_json::Value>(&no_provider.body).unwrap_or_default()
                ["error"]
                == "service_unavailable"
    );

    let services = crate::route_http_request("GET", "/mesh/http/services", None, "", &state)
        .await
        .expect("mesh gateway services response");
    let services_json =
        serde_json::from_str::<serde_json::Value>(&services.body).unwrap_or_default();
    record!(
        "GET",
        "/mesh/http/services",
        "nominal-status-headers-body",
        services.status == "200 OK"
            && services_json["gateway"]["enabled"] == true
            && services_json["services"].as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["serviceName"] == "pods"
                        && row["providerCount"] == 0
                        && row["available"] == false
                })
            })
    );

    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-http-gateway-differential-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("mesh gateway differential state directory");
    let (mut dispatch_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods"),
    );
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("mesh gateway differential fixture"),
    );
    Arc::get_mut(&mut dispatch_state)
        .expect("unshared differential test state")
        .private_gateway = Some(gateway);
    let populated_services =
        crate::route_http_request("GET", "/mesh/http/services", None, "", &dispatch_state)
            .await
            .expect("populated mesh gateway services response");
    let populated_services_json =
        serde_json::from_str::<serde_json::Value>(&populated_services.body).unwrap_or_default();
    record!(
        "GET",
        "/mesh/http/services",
        "populated-dynamic-state",
        populated_services.status == "200 OK"
            && populated_services_json["services"]
                .as_array()
                .is_some_and(|rows| {
                    rows.iter().any(|row| {
                        row["serviceName"] == "pods"
                            && row["providerCount"] == 1
                            && row["available"] == true
                    })
                })
    );
    let dispatched =
        crate::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &dispatch_state)
            .await
            .expect("real local service dispatch response");
    let dispatched_json =
        serde_json::from_str::<serde_json::Value>(&dispatched.body).unwrap_or_default();
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "nominal-status-headers-body",
        dispatched.status == "200 OK" && dispatched_json == serde_json::json!([])
    );

    let malformed_services_path =
        crate::route_http_request("GET", "/mesh/http/services/extra", None, "", &state)
            .await
            .expect("malformed mesh gateway services path response");
    record!(
        "GET",
        "/mesh/http/services",
        "malformed-path-query-or-body",
        malformed_services_path.status == "404 Not Found"
    );
    let malformed_service_path = crate::route_http_request(
        "POST",
        "/mesh/http/pods/List/extra",
        None,
        "{}",
        &dispatch_state,
    )
    .await
    .expect("malformed mesh gateway service path response");
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "malformed-path-query-or-body",
        malformed_service_path.status == "404 Not Found"
    );

    let mutation_pod_id = "pod:mesh-http-gateway-mutation";
    let gateway_username = dispatch_state
        .config
        .username
        .as_deref()
        .filter(|username| !username.trim().is_empty())
        .unwrap_or("slskr")
        .to_owned();
    dispatch_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": mutation_pod_id,
                "name": "Mesh HTTP Gateway Mutation",
                "isPublic": true,
            }))
            .expect("deserialize mesh gateway pod fixture"),
            "mesh-gateway-owner".to_owned(),
        )
        .expect("create mesh gateway mutation pod");
    let joined = crate::route_http_request(
        "POST",
        "/mesh/http/pods/Join",
        None,
        &serde_json::json!({"PodId": mutation_pod_id}).to_string(),
        &dispatch_state,
    )
    .await
    .expect("mesh gateway mutation response");
    let joined_json = serde_json::from_str::<serde_json::Value>(&joined.body).unwrap_or_default();
    let mutation_case = joined.status == "200 OK"
        && joined_json["Success"] == true
        && dispatch_state
            .pods
            .read()
            .await
            .is_member(mutation_pod_id, &gateway_username);
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "mutation-side-effects-and-readback",
        mutation_case
    );

    let concurrent_pod_ids = (0..6)
        .map(|index| format!("pod:mesh-http-gateway-concurrent-{index}"))
        .collect::<Vec<_>>();
    {
        let mut pods = dispatch_state.pods.write().await;
        for pod_id in &concurrent_pod_ids {
            pods.create(
                serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                    "podId": pod_id,
                    "name": pod_id,
                    "isPublic": true,
                }))
                .expect("deserialize concurrent mesh gateway pod"),
                "mesh-gateway-owner".to_owned(),
            )
            .expect("create concurrent mesh gateway pod");
        }
    }
    let concurrent_joins =
        futures_util::future::join_all(concurrent_pod_ids.iter().map(|pod_id| {
            let state = Arc::clone(&dispatch_state);
            let body = serde_json::json!({"PodId": pod_id}).to_string();
            async move {
                crate::route_http_request("POST", "/mesh/http/pods/Join", None, &body, &state).await
            }
        }))
        .await;
    let concurrent_responses_positive = concurrent_joins.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["Success"] == true)
        })
    });
    let all_concurrent_members = {
        let pods = dispatch_state.pods.read().await;
        concurrent_pod_ids
            .iter()
            .all(|pod_id| pods.is_member(pod_id, &gateway_username))
    };
    let concurrency_case = concurrent_responses_positive && all_concurrent_members;
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "concurrency-and-idempotency",
        concurrency_case
    );

    let state_dir = dispatch_state.config.state_dir.clone();
    let reloaded_pods =
        crate::pods::PodStore::load(&state_dir).expect("reload mesh gateway pod store");
    let restart_case = reloaded_pods.is_member(mutation_pod_id, &gateway_username)
        && concurrent_pod_ids
            .iter()
            .all(|pod_id| reloaded_pods.is_member(pod_id, &gateway_username));
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "restart-persistence-or-reset",
        restart_case
    );
    std::fs::remove_dir_all(root).expect("remove mesh gateway differential state directory");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mesh_http_gateway.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh-http-gateway mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
