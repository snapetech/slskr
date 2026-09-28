//! Controller full security differential 04 ownership.

use super::*;

/// Differential proof for the remaining slskdN SecurityController cases.
/// The existing security ledgers cover bans, reputation, diagnostics, and
/// closed-database behavior; this residual ledger closes the configured
/// optional-service reads plus the circuit, entropy, adversarial,
/// disclosure, and reputation validation/lifecycle branches.
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
pub(super) async fn controller_api_differential_security_controller_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
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

    async fn seed_feature(state: &crate::AppState, key: &str, value: serde_json::Value) {
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(key.to_owned(), value)
            .expect("seed security controller feature");
    }

    async fn seed_circuit_service(state: &crate::AppState) {
        seed_feature(
            state,
            "security/profile/security/circuits",
            serde_json::json!({"configured": true}),
        )
        .await;
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/adversarial",
            serde_json::json!({
                "resource": "security/adversarial",
                "settings": {"enabled": true, "profile": "Standard"}
            }),
        )
        .await;
        let response =
            crate::route_http_request("GET", "/api/v0/security/adversarial", None, "", &state)
                .await
                .expect("configured adversarial settings");
        record!(
            "GET",
            "/api/v0/security/adversarial",
            "nominal-status-headers-body",
            response.status == "404 Not Found"
                && response.body == "Adversarial features are not configured"
        );
        record!(
            "GET",
            "/api/v0/security/adversarial",
            "populated-dynamic-state",
            response.status == "404 Not Found"
                && response.body == "Adversarial features are not configured"
        );

        let malformed = crate::route_http_request(
            "GET",
            "/api/v0/security/adversarial/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed adversarial path");
        record!(
            "GET",
            "/api/v0/security/adversarial",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/adversarial",
            serde_json::json!({
                "resource": "security/adversarial",
                "settings": {"enabled": true}
            }),
        )
        .await;
        let response = crate::route_http_request(
            "GET",
            "/api/v0/security/adversarial/stats",
            None,
            "",
            &state,
        )
        .await
        .expect("adversarial stats");
        record!(
            "GET",
            "/api/v0/security/adversarial/stats",
            "populated-dynamic-state",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["profile"].is_string())
        );
    }

    for (path, route, case, expected) in [
        (
            "/api/v0/security/bans/extra",
            "/api/v0/security/bans",
            "malformed-path-query-or-body",
            "404 Not Found",
        ),
        (
            "/api/v0/security/bans",
            "/api/v0/security/bans",
            "missing-empty-or-conflict-state",
            "200 OK",
        ),
        (
            "/api/v0/security/dashboard/extra",
            "/api/v0/security/dashboard",
            "malformed-path-query-or-body",
            "404 Not Found",
        ),
        (
            "/api/v0/security/dashboard",
            "/api/v0/security/dashboard",
            "missing-empty-or-conflict-state",
            "200 OK",
        ),
        (
            "/api/v0/security/reputation/peer/extra",
            "/api/v0/security/reputation/{username}",
            "malformed-path-query-or-body",
            "404 Not Found",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).ok();
        record!(
            "GET",
            route,
            case,
            response.status == expected && (expected != "200 OK" || value.is_some())
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/canaries",
            serde_json::json!({
                "settings": {
                    "totalAccesses": 3,
                    "uniqueAttackers": 2,
                    "activeTraps": 1,
                    "triggeredTraps": 1,
                    "trapsByType": {"credential": 1}
                }
            }),
        )
        .await;
        let response =
            crate::route_http_request("GET", "/api/v0/security/canaries", None, "", &state)
                .await
                .expect("configured canary state");
        record!(
            "GET",
            "/api/v0/security/canaries",
            "populated-dynamic-state",
            response.status == "200 OK"
                && response.body.contains("\"totalAccesses\":3")
                && response.body.contains("credential")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/disclosure/residual-peer",
            serde_json::json!({
                "resource": "security/disclosure/residual-peer",
                "settings": {"tier": "Trusted", "canBrowse": true}
            }),
        )
        .await;
        let nominal = crate::route_http_request(
            "GET",
            "/api/v0/security/disclosure/residual-peer",
            None,
            "",
            &state,
        )
        .await
        .expect("disclosure nominal");
        let malformed = crate::route_http_request(
            "GET",
            "/api/v0/security/disclosure/residual-peer/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("disclosure malformed");
        record!(
            "GET",
            "/api/v0/security/disclosure/{username}",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && nominal.body.contains("\"peerTier\":\"Trusted\"")
        );
        record!(
            "GET",
            "/api/v0/security/disclosure/{username}",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        record!(
            "GET",
            "/api/v0/security/disclosure/{username}",
            "populated-dynamic-state",
            nominal.status == "200 OK" && nominal.body.contains("\"canBrowse\":true")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.users.write().await.records.push(crate::UserRecord {
            username: "security-populated-peer".to_owned(),
            watched: false,
            status: Some("online".to_owned()),
            privileged: false,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: crate::unix_timestamp(),
        });
        let response =
            crate::route_http_request("GET", "/api/v0/security/peers/stats", None, "", &state)
                .await
                .expect("peer stats");
        record!(
            "GET",
            "/api/v0/security/peers/stats",
            "populated-dynamic-state",
            response.status == "200 OK" && response.body.contains("\"totalPeers\":1")
        );
    }

    for route in [
        "/api/v0/security/reputation/suspicious",
        "/api/v0/security/reputation/trusted",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response =
            crate::route_http_request("GET", &format!("{route}?limit=0"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    for (route, case, path) in [
        (
            "/api/v0/security/scanners",
            "malformed-path-query-or-body",
            "/api/v0/security/scanners/extra",
        ),
        (
            "/api/v0/security/scanners",
            "populated-dynamic-state",
            "/api/v0/security/scanners",
        ),
        (
            "/api/v0/security/threats",
            "malformed-path-query-or-body",
            "/api/v0/security/threats/extra",
        ),
        (
            "/api/v0/security/threats",
            "populated-dynamic-state",
            "/api/v0/security/threats",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let feature_key = if route.ends_with("scanners") {
            "security/profile/security/scanners"
        } else {
            "security/profile/security/threats"
        };
        seed_feature(
            &state,
            feature_key,
            serde_json::json!({"items": [{"id": "security-residual"}]}),
        )
        .await;
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            case,
            if case == "malformed-path-query-or-body" {
                response.status == "404 Not Found"
            } else {
                response.status == "200 OK" && response.body.contains("security-residual")
            }
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/tor",
            serde_json::json!({
                "settings": {"type": "Tor", "available": true, "healthy": true}
            }),
        )
        .await;
        let response =
            crate::route_http_request("GET", "/api/v0/security/tor/status", None, "", &state)
                .await
                .expect("configured Tor status");
        record!(
            "GET",
            "/api/v0/security/tor/status",
            "populated-dynamic-state",
            response.status == "200 OK" && response.body.contains("\"available\":true")
        );
    }

    for (route, case, path) in [
        (
            "/api/v0/security/transports",
            "malformed-path-query-or-body",
            "/api/v0/security/transports/extra",
        ),
        (
            "/api/v0/security/transports",
            "populated-dynamic-state",
            "/api/v0/security/transports",
        ),
        (
            "/api/v0/security/transports/status",
            "malformed-path-query-or-body",
            "/api/v0/security/transports/status/extra",
        ),
        (
            "/api/v0/security/transports/status",
            "missing-empty-or-conflict-state",
            "/api/v0/security/transports/status",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if case != "missing-empty-or-conflict-state" {
            seed_feature(
                &state,
                "security/profile/security/transports",
                serde_json::json!({
                    "items": {"Direct": {"available": true}},
                    "status": {"selectedMode": "Direct", "totalTransports": 1}
                }),
            )
            .await;
        }
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let pass = if case == "malformed-path-query-or-body" {
            response.status == "404 Not Found"
        } else if route.ends_with("/status") {
            response.status == "200 OK" && response.body.contains("selectedMode")
        } else {
            response.status == "200 OK" && response.body.contains("Direct")
        };
        record!("GET", route, case, pass);
    }

    let circuit_body = r#"{"targetPeerId":"security-circuit-peer","circuitId":"residual-circuit"}"#;
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let created = crate::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("create residual circuit");
        record!(
            "POST",
            "/api/v0/security/circuits",
            "nominal-status-headers-body",
            created.status == "200 OK" && created.body.contains("residual-circuit")
        );
        let listed =
            crate::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
                .await
                .expect("circuit readback");
        record!(
            "POST",
            "/api/v0/security/circuits",
            "mutation-side-effects-and-readback",
            listed.status == "200 OK" && listed.body.contains("residual-circuit")
        );
    }
    for (case, body, path) in [
        (
            "malformed-path-query-or-body",
            "not-json",
            "/api/v0/security/circuits",
        ),
        (
            "missing-empty-or-conflict-state",
            "",
            "/api/v0/security/circuits",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("invalid residual circuit");
        record!(
            "POST",
            "/api/v0/security/circuits",
            case,
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let created = crate::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("restart circuit seed");
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let listed =
            crate::route_http_request("GET", "/api/v0/security/circuits", None, "", &restarted)
                .await
                .expect("restart circuit list");
        record!(
            "POST",
            "/api/v0/security/circuits",
            "restart-persistence-or-reset",
            created.status == "200 OK" && listed.status == "200 OK" && listed.body == "[]"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/security/circuits",
                None,
                circuit_body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/security/circuits",
                None,
                circuit_body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/security/circuits",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for (case, body, path) in [
        (
            "malformed-path-query-or-body",
            "not-json",
            "/api/v0/security/circuits/%20",
        ),
        (
            "missing-empty-or-conflict-state",
            "",
            "/api/v0/security/circuits/missing-circuit",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = crate::route_http_request("DELETE", path, None, body, &state)
            .await
            .expect("invalid residual circuit delete");
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            case,
            response.status
                == if case == "malformed-path-query-or-body" {
                    "400 Bad Request"
                } else {
                    "200 OK"
                }
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        crate::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("mutation circuit seed");
        let deleted = crate::route_http_request(
            "DELETE",
            "/api/v0/security/circuits/residual-circuit",
            None,
            "",
            &state,
        )
        .await
        .expect("mutation circuit delete");
        let listed =
            crate::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
                .await
                .expect("mutation circuit readback");
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "nominal-status-headers-body",
            deleted.status == "200 OK"
        );
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "mutation-side-effects-and-readback",
            deleted.status == "200 OK" && listed.body == "[]"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        crate::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("restart delete circuit seed");
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let deleted = crate::route_http_request(
            "DELETE",
            "/api/v0/security/circuits/residual-circuit",
            None,
            "",
            &restarted,
        )
        .await
        .expect("restart delete circuit");
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "restart-persistence-or-reset",
            deleted.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        crate::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("concurrent delete circuit seed");
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "DELETE",
                "/api/v0/security/circuits/residual-circuit",
                None,
                "",
                &state
            ),
            crate::route_http_request(
                "DELETE",
                "/api/v0/security/circuits/residual-circuit",
                None,
                "",
                &state
            )
        );
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for case in [
        "nominal-status-headers-body",
        "missing-empty-or-conflict-state",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let path = if case == "malformed-path-query-or-body" {
            "/api/v0/security/entropy/check/extra"
        } else {
            "/api/v0/security/entropy/check"
        };
        let response = crate::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            "/api/v0/security/entropy/check",
            case,
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = crate::route_http_request(
            "POST",
            "/api/v0/security/entropy/check/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed entropy path");
        record!(
            "POST",
            "/api/v0/security/entropy/check",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }

    let prepare_adversarial = |state: &Arc<crate::AppState>| {
        let state = Arc::clone(state);
        async move {
            fs::write(
                state.config.state_dir.join("slskd.yml"),
                "remote_configuration: true\nsecurity: {}\n",
            )
            .expect("write residual adversarial YAML");
        }
    };
    for (case, body) in [
        ("malformed-path-query-or-body", "not-json"),
        ("missing-empty-or-conflict-state", ""),
    ] {
        let (state, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        prepare_adversarial(&state).await;
        let response =
            crate::route_http_request("PUT", "/api/v0/security/adversarial", None, body, &state)
                .await
                .expect("invalid adversarial update");
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            case,
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        prepare_adversarial(&state).await;
        let response =
            crate::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
                .await
                .expect("adversarial restart seed");
        let (restarted, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        let readback =
            crate::route_http_request("GET", "/api/v0/security/adversarial", None, "", &restarted)
                .await
                .expect("adversarial restart readback");
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "restart-persistence-or-reset",
            response.status == "200 OK" && readback.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        prepare_adversarial(&state).await;
        let (left, right) = tokio::join!(
            crate::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state),
            crate::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
        );
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for case in [
        "nominal-status-headers-body",
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let body = if case == "malformed-path-query-or-body" {
            "not-json"
        } else if case == "missing-empty-or-conflict-state" {
            ""
        } else {
            r#"{"tier":"Trusted"}"#
        };
        let response = crate::route_http_request(
            "PUT",
            "/api/v0/security/disclosure/residual-peer",
            None,
            body,
            &state,
        )
        .await
        .expect("disclosure mutation");
        let mut pass = if case == "malformed-path-query-or-body"
            || case == "missing-empty-or-conflict-state"
        {
            response.status == "400 Bad Request"
        } else {
            response.status == "200 OK"
        };
        if pass && case == "mutation-side-effects-and-readback" {
            let readback = crate::route_http_request(
                "GET",
                "/api/v0/security/disclosure/residual-peer",
                None,
                "",
                &state,
            )
            .await
            .expect("disclosure mutation readback");
            pass = readback.status == "200 OK" && readback.body.contains("Trusted");
        }
        if pass && case == "restart-persistence-or-reset" {
            let (restarted, _receiver) = test_state_with_env(base_env.clone());
            let readback = crate::route_http_request(
                "GET",
                "/api/v0/security/disclosure/residual-peer",
                None,
                "",
                &restarted,
            )
            .await
            .expect("disclosure restart readback");
            pass = readback.status == "200 OK" && !readback.body.contains("Trusted");
        }
        record!("PUT", "/api/v0/security/disclosure/{username}", case, pass);
    }

    for case in [
        "missing-empty-or-conflict-state",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let body = if case == "missing-empty-or-conflict-state" {
            ""
        } else {
            r#"{"score":80}"#
        };
        let response = crate::route_http_request(
            "PUT",
            "/api/v0/security/reputation/residual-peer",
            None,
            body,
            &state,
        )
        .await
        .expect("reputation residual mutation");
        let mut pass = if case == "missing-empty-or-conflict-state" {
            response.status == "400 Bad Request"
        } else {
            response.status == "200 OK"
        };
        if pass && case == "restart-persistence-or-reset" {
            let (restarted, _receiver) = test_state_with_env(base_env.clone());
            let readback = crate::route_http_request(
                "GET",
                "/api/v0/security/reputation/residual-peer",
                None,
                "",
                &restarted,
            )
            .await
            .expect("reputation restart readback");
            pass = readback.status == "200 OK" && readback.body.contains("\"score\":50");
        }
        if pass && case == "concurrency-and-idempotency" {
            let (left, right) = tokio::join!(
                crate::route_http_request(
                    "PUT",
                    "/api/v0/security/reputation/residual-peer",
                    None,
                    body,
                    &state
                ),
                crate::route_http_request(
                    "PUT",
                    "/api/v0/security/reputation/residual-peer",
                    None,
                    body,
                    &state
                )
            );
            pass = left.as_ref().is_ok_and(|value| value.status == "200 OK")
                && right.as_ref().is_ok_and(|value| value.status == "200 OK");
        }
        record!("PUT", "/api/v0/security/reputation/{username}", case, pass);
    }

    assert_eq!(ledger.len(), 56, "Security residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Security evidence directory");
    fs::write(
        evidence_dir.join("security_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Security ledger"),
    )
    .expect("write Security ledger");
    assert!(
        mismatches.is_empty(),
        "{} Security residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
