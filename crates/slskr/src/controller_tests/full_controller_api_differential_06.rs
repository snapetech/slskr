//! Controller full controller api differential 06 ownership.

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
pub(super) async fn controller_api_differential_controller_options_overlay_contracts() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
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
                "pass": pass,
            }));
        }};
    }

    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_REMOTE_CONFIGURATION", "true");
    let (state, _receiver) = test_state_with_env(env.clone());
    let current = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
        .await
        .expect("slskd current options projection");
    let current_json = serde_json::from_str::<serde_json::Value>(&current.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/options",
        "nominal-status-headers-body",
        current.status == "200 OK"
            && current.content_type == "application/json; charset=utf-8"
            && current_json["remoteConfiguration"] == true
            && current_json["web"]["authentication"]["password"] == "*****"
    );

    let patched = crate::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50317}}"#,
        &state,
    )
    .await
    .expect("slskd options overlay");
    let patched_json = serde_json::from_str::<serde_json::Value>(&patched.body).unwrap_or_default();
    record!(
        "PATCH",
        "/api/v0/options",
        "nominal-status-headers-body",
        patched.status == "200 OK"
            && patched.content_type == "application/json; charset=utf-8"
            && patched_json["soulseek"]["listenPort"] == 50317
    );

    let readback = crate::route_http_request("GET", "/api/v0/options", None, "", &state)
        .await
        .expect("slskd options overlay readback");
    let readback_json =
        serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/options",
        "populated-dynamic-state",
        readback.status == "200 OK" && readback_json["soulseek"]["listenPort"] == 50317
    );
    record!(
        "PATCH",
        "/api/v0/options",
        "mutation-side-effects-and-readback",
        patched.status == "200 OK" && readback_json["soulseek"]["listenPort"] == 50317
    );

    let null_overlay = crate::route_http_request("PATCH", "/api/v0/options", None, "null", &state)
        .await
        .expect("slskd null options overlay");
    record!(
        "PATCH",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        null_overlay.status == "204 No Content" && null_overlay.body.is_empty()
    );

    let malformed_overlay = crate::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":80}}"#,
        &state,
    )
    .await
    .expect("slskd malformed options overlay");
    record!(
        "PATCH",
        "/api/v0/options",
        "malformed-path-query-or-body",
        malformed_overlay.status == "400 Bad Request"
    );

    let (disabled, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let forbidden = crate::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50318}}"#,
        &disabled,
    )
    .await
    .expect("slskd disabled options overlay");
    record!(
        "PATCH",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );

    let restarted = test_state_with_env(env).0;
    let after_restart = crate::route_http_request("GET", "/api/v0/options", None, "", &restarted)
        .await
        .expect("slskd options after restart");
    let after_restart_json =
        serde_json::from_str::<serde_json::Value>(&after_restart.body).unwrap_or_default();
    record!(
        "PATCH",
        "/api/v0/options",
        "restart-persistence-or-reset",
        after_restart.status == "200 OK" && after_restart_json["soulseek"]["listenPort"] != 50317
    );

    let (failed_options, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true"),
    );
    *failed_options
        .controller_options_validation_error
        .write()
        .expect("slskd options validation error lock") =
        Some("differential invalid options".into());
    let failed_patch = crate::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50319}}"#,
        &failed_options,
    )
    .await
    .expect("slskd options runtime failure response");
    record!(
        "PATCH",
        "/api/v0/options",
        "runtime-failure-and-timeout",
        failed_patch.status == "500 Internal Server Error"
            && failed_patch.content_type == "application/json; charset=utf-8"
            && failed_patch.body == r#""A validation error has occurred.""#
    );

    let failed_yaml = crate::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        r#""debug: true\n""#,
        &failed_options,
    )
    .await
    .expect("slskd yaml validation runtime failure response");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "runtime-failure-and-timeout",
        failed_yaml.status == "500 Internal Server Error"
            && failed_yaml.content_type == "application/json; charset=utf-8"
            && failed_yaml.body == r#""A validation error has occurred.""#
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_options_overlay_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd options ledger"),
    )
    .expect("write slskd options ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd options mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Close deterministic slskd controller cases that are independent of
/// the larger route-family fixtures: session-enabled projection and
/// login lifecycle, YAML validation/update failure semantics, room
/// subresource missing/idempotent behavior, and share-scan fault
/// injection.  Each row below is backed by an actual versioned dispatcher
/// call and a state/file readback rather than a route-presence assertion.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) fn controller_api_differential_controller_residual_core_contracts() {
    run_controller_future_on_large_stack("controller-residual-core-contracts", || {
        controller_api_differential_controller_residual_core_contracts_impl()
    });
}
