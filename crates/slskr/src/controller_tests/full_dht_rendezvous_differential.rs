//! Controller DHT rendezvous differential contract tests.

use super::*;

/// Bulk differential proof for the remaining frozen DhtRendezvous
/// controller cases.  The v0 DHT actions deliberately use their frozen
/// no-body contracts, while blocklist and certificate-pin cases exercise
/// real local state, reset behavior, and concurrent mutations.
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
pub(super) async fn controller_api_differential_dht_rendezvous_residuals() {
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
                .expect("DHT/overlay route request")
        }};
    }

    fn json_object(body: &str) -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or_default()
    }

    let (state, _receiver) = test_state();

    let mut populated_state;
    let (_, _receiver) = {
        let (created, receiver) = test_state();
        populated_state = created;
        (populated_state.clone(), receiver)
    };
    let mut dht_settings = populated_state.config.advanced_networking.dht.clone();
    dht_settings.dht_port = 0;
    dht_settings.overlay_port = 0;
    dht_settings.advertised_overlay_port = 0;
    dht_settings.lan_only = true;
    dht_settings.bootstrap_routers.clear();
    let rendezvous = crate::dht::Rendezvous::new(&dht_settings).expect("test DHT rendezvous");
    rendezvous
        .insert_test_peer("198.51.100.10:6881".parse().expect("test DHT peer"))
        .await;
    Arc::get_mut(&mut populated_state)
        .expect("unique populated state")
        .dht = Some(Arc::new(rendezvous));

    let dht_peers_malformed = request!("GET", "/api/v0/dht/peers?unexpected=%7B", "", &state);
    record!(
        "GET",
        "/api/v0/dht/peers",
        "malformed-path-query-or-body",
        dht_peers_malformed.status == "200 OK" && json_object(&dht_peers_malformed.body).is_array()
    );
    let dht_peers_runtime = request!("GET", "/api/v0/dht/peers", "", &state);
    record!(
        "GET",
        "/api/v0/dht/peers",
        "runtime-failure-and-timeout",
        dht_peers_runtime.status == "200 OK" && json_object(&dht_peers_runtime.body).is_array()
    );
    let dht_peers_populated = request!("GET", "/api/v0/dht/peers", "", &populated_state);
    let dht_peers_populated_json = json_object(&dht_peers_populated.body);
    record!(
        "GET",
        "/api/v0/dht/peers",
        "populated-dynamic-state",
        dht_peers_populated.status == "200 OK"
            && dht_peers_populated_json[0]["address"] == "198.51.100.10"
            && dht_peers_populated_json[0]["port"] == 6881
    );

    let dht_status_malformed = request!("GET", "/api/v0/dht/status?unexpected=%7B", "", &state);
    record!(
        "GET",
        "/api/v0/dht/status",
        "malformed-path-query-or-body",
        dht_status_malformed.status == "200 OK"
            && json_object(&dht_status_malformed.body)["isBeaconCapable"] == false
    );
    let dht_status_missing = request!("GET", "/api/v0/dht/status", "", &state);
    record!(
        "GET",
        "/api/v0/dht/status",
        "missing-empty-or-conflict-state",
        dht_status_missing.status == "200 OK"
            && json_object(&dht_status_missing.body)["dhtNodeCount"] == 0
    );
    let dht_status_runtime = request!("GET", "/api/v0/dht/status", "", &state);
    record!(
        "GET",
        "/api/v0/dht/status",
        "runtime-failure-and-timeout",
        dht_status_runtime.status == "200 OK"
            && json_object(&dht_status_runtime.body)["rendezvousInfohashes"].is_array()
    );

    let blocklist_get_malformed = request!(
        "GET",
        "/api/v0/overlay/blocklist?unexpected=%7B",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "malformed-path-query-or-body",
        blocklist_get_malformed.status == "200 OK"
            && json_object(&blocklist_get_malformed.body)["entries"].is_array()
    );
    let blocklist_get_missing = request!("GET", "/api/v0/overlay/blocklist", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "missing-empty-or-conflict-state",
        blocklist_get_missing.status == "200 OK"
            && json_object(&blocklist_get_missing.body)["entries"]
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let blocklist_get_runtime = request!("GET", "/api/v0/overlay/blocklist", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "runtime-failure-and-timeout",
        blocklist_get_runtime.status == "200 OK"
    );

    let overlay_connections_malformed = request!(
        "GET",
        "/api/v0/overlay/connections?unexpected=%7B",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v0/overlay/connections",
        "malformed-path-query-or-body",
        overlay_connections_malformed.status == "200 OK"
            && json_object(&overlay_connections_malformed.body).is_array()
    );
    let overlay_connections_missing = request!("GET", "/api/v0/overlay/connections", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/connections",
        "missing-empty-or-conflict-state",
        overlay_connections_missing.status == "200 OK"
            && json_object(&overlay_connections_missing.body)
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let overlay_connections_runtime = request!("GET", "/api/v0/overlay/connections", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/connections",
        "runtime-failure-and-timeout",
        overlay_connections_runtime.status == "200 OK"
    );

    let overlay_stats_malformed =
        request!("GET", "/api/v0/overlay/stats?unexpected=%7B", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/stats",
        "malformed-path-query-or-body",
        overlay_stats_malformed.status == "200 OK"
            && json_object(&overlay_stats_malformed.body)["server"].is_object()
    );
    let overlay_stats_missing = request!("GET", "/api/v0/overlay/stats", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/stats",
        "missing-empty-or-conflict-state",
        overlay_stats_missing.status == "200 OK"
            && json_object(&overlay_stats_missing.body)["connector"].is_object()
    );
    let overlay_stats_runtime = request!("GET", "/api/v0/overlay/stats", "", &state);
    record!(
        "GET",
        "/api/v0/overlay/stats",
        "runtime-failure-and-timeout",
        overlay_stats_runtime.status == "200 OK"
            && json_object(&overlay_stats_runtime.body)["blocklist"].is_object()
    );

    let announce_nominal = request!("POST", "/api/v0/dht/announce", "", &state);
    record!(
        "POST",
        "/api/v0/dht/announce",
        "nominal-status-headers-body",
        announce_nominal.status == "400 Bad Request"
            && announce_nominal.body == r#"{"error":"Not beacon capable"}"#
    );
    let announce_malformed = request!("POST", "/api/v0/dht/announce", "not-json", &state);
    record!(
        "POST",
        "/api/v0/dht/announce",
        "malformed-path-query-or-body",
        announce_malformed.status == "400 Bad Request"
    );
    let announce_runtime = request!(
        "POST",
        "/api/v0/dht/announce",
        r#"{"ignored":true}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/dht/announce",
        "runtime-failure-and-timeout",
        announce_runtime.status == "400 Bad Request"
    );
    let announce_mutation = request!(
        "POST",
        "/api/v0/dht/announce",
        r#"{"ignored":true}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/dht/announce",
        "mutation-side-effects-and-readback",
        announce_mutation.status == "400 Bad Request"
    );
    let (announce_restart_state, _receiver) = test_state();
    let announce_restart = request!("POST", "/api/v0/dht/announce", "", &announce_restart_state);
    record!(
        "POST",
        "/api/v0/dht/announce",
        "restart-persistence-or-reset",
        announce_restart.status == "400 Bad Request"
    );
    let (announce_a, announce_b) = tokio::join!(
        crate::route_http_request("POST", "/api/v0/dht/announce", None, "", &state),
        crate::route_http_request("POST", "/api/v0/dht/announce", None, "", &state),
    );
    record!(
        "POST",
        "/api/v0/dht/announce",
        "concurrency-and-idempotency",
        announce_a
            .as_ref()
            .is_ok_and(|response| response.status == "400 Bad Request")
            && announce_b
                .as_ref()
                .is_ok_and(|response| response.status == "400 Bad Request")
    );

    let discover_malformed = request!("POST", "/api/v0/dht/discover", "not-json", &state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "malformed-path-query-or-body",
        discover_malformed.status == "200 OK"
            && json_object(&discover_malformed.body)["newConnectionsMade"].is_number()
    );
    let discover_missing = request!("POST", "/api/v0/dht/discover", "", &state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "missing-empty-or-conflict-state",
        discover_missing.status == "200 OK"
            && json_object(&discover_missing.body)["totalMeshConnections"].is_number()
    );
    let discover_runtime = request!(
        "POST",
        "/api/v0/dht/discover",
        r#"{"ignored":true}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/dht/discover",
        "runtime-failure-and-timeout",
        discover_runtime.status == "200 OK"
    );
    let discover_mutation = request!("POST", "/api/v0/dht/discover", "", &state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "mutation-side-effects-and-readback",
        discover_mutation.status == "200 OK"
    );
    let (discover_restart_state, _receiver) = test_state();
    let discover_restart = request!("POST", "/api/v0/dht/discover", "", &discover_restart_state);
    record!(
        "POST",
        "/api/v0/dht/discover",
        "restart-persistence-or-reset",
        discover_restart.status == "200 OK"
    );
    let (discover_a, discover_b) = tokio::join!(
        crate::route_http_request("POST", "/api/v0/dht/discover", None, "", &state),
        crate::route_http_request("POST", "/api/v0/dht/discover", None, "", &state),
    );
    record!(
        "POST",
        "/api/v0/dht/discover",
        "concurrency-and-idempotency",
        discover_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && discover_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let block_ip_malformed = request!("POST", "/api/v0/overlay/blocklist/ip", "{}", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "malformed-path-query-or-body",
        block_ip_malformed.status == "400 Bad Request"
    );
    let block_ip_missing = request!("POST", "/api/v0/overlay/blocklist/ip", "", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "missing-empty-or-conflict-state",
        block_ip_missing.status == "400 Bad Request"
    );
    let block_ip_runtime = request!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        r#"{"ip":"203.0.113.5","reason":"runtime"}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "runtime-failure-and-timeout",
        block_ip_runtime.status == "200 OK"
    );
    let (block_ip_restart_state, _receiver) = test_state();
    let block_ip_restart_list = request!(
        "GET",
        "/api/v0/overlay/blocklist",
        "",
        &block_ip_restart_state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "restart-persistence-or-reset",
        block_ip_restart_list.status == "200 OK"
            && json_object(&block_ip_restart_list.body)["entries"]
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let block_ip_body_a = r#"{"ip":"203.0.113.6"}"#;
    let block_ip_body_b = r#"{"ip":"203.0.113.7"}"#;
    let (block_ip_a, block_ip_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/ip",
            None,
            block_ip_body_a,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/ip",
            None,
            block_ip_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "concurrency-and-idempotency",
        block_ip_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && block_ip_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let block_user_malformed = request!("POST", "/api/v0/overlay/blocklist/username", "{}", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "malformed-path-query-or-body",
        block_user_malformed.status == "400 Bad Request"
    );
    let block_user_missing = request!("POST", "/api/v0/overlay/blocklist/username", "", &state);
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "missing-empty-or-conflict-state",
        block_user_missing.status == "400 Bad Request"
    );
    let block_user_runtime = request!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        r#"{"username":"runtime-user","reason":"runtime"}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "runtime-failure-and-timeout",
        block_user_runtime.status == "200 OK"
    );
    let (block_user_restart_state, _receiver) = test_state();
    let block_user_restart_list = request!(
        "GET",
        "/api/v0/overlay/blocklist",
        "",
        &block_user_restart_state
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "restart-persistence-or-reset",
        block_user_restart_list.status == "200 OK"
            && json_object(&block_user_restart_list.body)["entries"]
                .as_array()
                .is_some_and(|entries| entries.is_empty())
    );
    let block_user_body_a = r#"{"username":"concurrent-user-a"}"#;
    let block_user_body_b = r#"{"username":"concurrent-user-b"}"#;
    let (block_user_a, block_user_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/username",
            None,
            block_user_body_a,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/blocklist/username",
            None,
            block_user_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "concurrency-and-idempotency",
        block_user_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && block_user_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let connect_body = r#"{"address":"203.0.113.10","port":1}"#;
    let overlay_connect_nominal = request!("POST", "/api/v0/overlay/connect", connect_body, &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "nominal-status-headers-body",
        overlay_connect_nominal.status == "502 Bad Gateway"
            && json_object(&overlay_connect_nominal.body)["connected"] == false
    );
    let overlay_connect_malformed = request!("POST", "/api/v0/overlay/connect", "{}", &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "malformed-path-query-or-body",
        overlay_connect_malformed.status == "400 Bad Request"
    );
    let overlay_connect_missing = request!("POST", "/api/v0/overlay/connect", "", &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "missing-empty-or-conflict-state",
        overlay_connect_missing.status == "400 Bad Request"
    );
    let overlay_connect_runtime = request!(
        "POST",
        "/api/v0/overlay/connect",
        r#"{"address":"203.0.113.11","port":2}"#,
        &state
    );
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "runtime-failure-and-timeout",
        overlay_connect_runtime.status == "502 Bad Gateway"
    );
    let overlay_connect_mutation =
        request!("POST", "/api/v0/overlay/connect", connect_body, &state);
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "mutation-side-effects-and-readback",
        overlay_connect_mutation.status == "502 Bad Gateway"
    );
    let (connect_restart_state, _receiver) = test_state();
    let overlay_connect_restart = request!(
        "POST",
        "/api/v0/overlay/connect",
        connect_body,
        &connect_restart_state
    );
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "restart-persistence-or-reset",
        overlay_connect_restart.status == "502 Bad Gateway"
    );
    let connect_body_a = r#"{"address":"203.0.113.12","port":3}"#;
    let connect_body_b = r#"{"address":"203.0.113.13","port":4}"#;
    let (connect_a, connect_b) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/connect",
            None,
            connect_body_a,
            &state
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/overlay/connect",
            None,
            connect_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v0/overlay/connect",
        "concurrency-and-idempotency",
        connect_a
            .as_ref()
            .is_ok_and(|response| response.status == "502 Bad Gateway")
            && connect_b
                .as_ref()
                .is_ok_and(|response| response.status == "502 Bad Gateway")
    );

    let pin_body =
        r#"{"thumbprint":"0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a"}"#;
    let pin_nominal = request!("PUT", "/api/v0/overlay/pins/nominal-peer", pin_body, &state);
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "nominal-status-headers-body",
        pin_nominal.status == "204 No Content" && pin_nominal.body.is_empty()
    );
    let pin_malformed = request!(
        "PUT",
        "/api/v0/overlay/pins/malformed-peer",
        r#"{"pin":"abc"}"#,
        &state
    );
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "malformed-path-query-or-body",
        pin_malformed.status == "400 Bad Request"
    );
    let pin_missing = request!("PUT", "/api/v0/overlay/pins/%20", pin_body, &state);
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "missing-empty-or-conflict-state",
        pin_missing.status == "400 Bad Request"
    );
    let pin_runtime = request!("PUT", "/api/v0/overlay/pins/runtime-peer", pin_body, &state);
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "runtime-failure-and-timeout",
        pin_runtime.status == "204 No Content"
    );
    let pin_mutation = request!(
        "PUT",
        "/api/v0/overlay/pins/mutation-peer",
        pin_body,
        &state
    );
    let pin_readback = state
        .controller_features
        .read()
        .await
        .get("overlay/pin/mutation-peer")
        .is_some();
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "mutation-side-effects-and-readback",
        pin_mutation.status == "204 No Content" && pin_readback
    );
    let (pin_restart_state, _receiver) = test_state();
    let pin_restart_readback = pin_restart_state
        .controller_features
        .read()
        .await
        .get("overlay/pin/mutation-peer")
        .is_none();
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "restart-persistence-or-reset",
        pin_restart_readback
    );
    let pin_body_a =
        r#"{"thumbprint":"1111111111111111111111111111111111111111111111111111111111111111"}"#;
    let pin_body_b =
        r#"{"thumbprint":"2222222222222222222222222222222222222222222222222222222222222222"}"#;
    let (pin_a, pin_b) = tokio::join!(
        crate::route_http_request(
            "PUT",
            "/api/v0/overlay/pins/concurrent-a",
            None,
            pin_body_a,
            &state
        ),
        crate::route_http_request(
            "PUT",
            "/api/v0/overlay/pins/concurrent-b",
            None,
            pin_body_b,
            &state
        ),
    );
    record!(
        "PUT",
        "/api/v0/overlay/pins/{username}",
        "concurrency-and-idempotency",
        pin_a
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content")
            && pin_b
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
    );

    let delete_setup = request!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        r#"{"username":"delete-runtime"}"#,
        &state
    );
    assert_eq!(delete_setup.status, "200 OK", "{}", delete_setup.body);
    let delete_malformed = request!("DELETE", "/api/v0/overlay/blocklist/username", "", &state);
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "malformed-path-query-or-body",
        delete_malformed.status == "400 Bad Request"
    );
    let delete_missing = request!(
        "DELETE",
        "/api/v0/overlay/blocklist/username/does-not-exist",
        "",
        &state
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "missing-empty-or-conflict-state",
        delete_missing.status == "404 Not Found"
    );
    let delete_runtime = request!(
        "DELETE",
        "/api/v0/overlay/blocklist/username/delete-runtime",
        "",
        &state
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "runtime-failure-and-timeout",
        delete_runtime.status == "200 OK"
    );
    let (delete_restart_state, _receiver) = test_state();
    let delete_restart = request!(
        "DELETE",
        "/api/v0/overlay/blocklist/username/delete-runtime",
        "",
        &delete_restart_state
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "restart-persistence-or-reset",
        delete_restart.status == "404 Not Found"
    );
    for username in ["delete-concurrent-a", "delete-concurrent-b"] {
        let body = serde_json::json!({"username": username}).to_string();
        let response = request!("POST", "/api/v0/overlay/blocklist/username", &body, &state);
        assert_eq!(response.status, "200 OK", "{}", response.body);
    }
    let (delete_a, delete_b) = tokio::join!(
        crate::route_http_request(
            "DELETE",
            "/api/v0/overlay/blocklist/username/delete-concurrent-a",
            None,
            "",
            &state
        ),
        crate::route_http_request(
            "DELETE",
            "/api/v0/overlay/blocklist/username/delete-concurrent-b",
            None,
            "",
            &state
        ),
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "concurrency-and-idempotency",
        delete_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && delete_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("dht_rendezvous_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize DHT/overlay ledger"),
    )
    .expect("write DHT/overlay ledger");

    assert_eq!(ledger.len(), 56, "DHT/overlay residual ledger size");
    assert!(
        mismatches.is_empty(),
        "{} controller-api DHT/overlay mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
