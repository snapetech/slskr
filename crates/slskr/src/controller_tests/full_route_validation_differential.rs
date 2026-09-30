//! Controller full route validation differential ownership.

use super::*;

/// Bulk differential proof crediting `nominal-status-headers-body` for
/// 20 read-only routes: independently re-verifies real status,
/// content-type, and body-shape (not just "200 OK") against the real
/// dispatcher, plus a secrets-leak guard, mirroring the same routes
/// `read_only_api_routes_return_contract_shapes` already proves --
/// written as its own independent differential (not a call into that
/// test) so this evidence is genuinely re-derived, not just re-exported.
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
pub(super) async fn controller_api_differential_read_only_routes_have_real_contract_shapes() {
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
        scheme: String,
    }

    // `/api/v0/capabilities`'s version string and `/api/v0/session/enabled`
    // are compatibility-target/credential-configuration-sensitive, unlike
    // the other fixed shapes below -- checked separately, per target,
    // rather than with one fragment assumed to hold for both profiles.
    const CASES: [(&str, &str); 18] = [
        ("/api/v0/health", "\"status\":\"ok\""),
        ("/api/v0/version", "\"name\":\"slskr\""),
        ("/api/v0/config", "\"credentials_configured\":true"),
        ("/api/v0/stats", "\"session\":"),
        ("/api/v0/telemetry", "\"health\":"),
        ("/api/v0/events", "[]"),
        ("/api/v0/events/records", "\"entries\":"),
        ("/api/v0/logs", "[]"),
        ("/api/v0/listeners", "\"regular_accepts\":0"),
        ("/api/v0/users", "\"count\":0"),
        ("/api/v0/rooms", "\"count\":0"),
        ("/api/v0/shares", "\"files\":1"),
        ("/api/v0/shares/catalog", "\"total_bytes\":42"),
        ("/api/v0/searches", "[]"),
        ("/api/v0/searches/records", "\"count\":0"),
        ("/api/v0/transfers", "[]"),
        ("/api/v0/transfers/stats", "\"total\":0"),
        ("/api/v0/session", "\"state\":"),
    ];

    fn credential_header(scheme: &str) -> &'static str {
        if scheme == "api_key" {
            "ApiKey admin-token"
        } else {
            "Bearer admin-token"
        }
    }

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    for (target, source) in [
        (
            "slskd",
            include_str!("../../data/legacy-controller-auth-policy.json"),
        ),
        (
            "slskdn",
            include_str!("../../data/native-controller-auth-policy.json"),
        ),
    ] {
        let rules: Vec<AuthPolicyRow> =
            serde_json::from_str(source).expect("checked controller auth policy registry");
        let scheme_for = |route: &str| {
            rules
                .iter()
                .find(|rule| rule.method == "GET" && rule.route == route)
                .map(|rule| rule.scheme.as_str())
        };

        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_API_TOKEN", "admin-token"),
        );

        for (path, expected_fragment) in CASES {
            let header = credential_header(scheme_for(path).unwrap_or("any"));
            let response = crate::route_http_request("GET", path, Some(header), "", &state)
                .await
                .expect("route response");
            let expected_content_type = if path == "/api/v0/shares" {
                "application/json; charset=utf-8"
            } else {
                "application/json"
            };
            let no_secrets_leaked = !response.body.contains("test-password")
                && !response.body.contains("api-token")
                && !response.body.contains("client-secret");
            let pass = response.status == "200 OK"
                && response.content_type == expected_content_type
                && response.body.contains(expected_fragment)
                && no_secrets_leaked;
            if !pass {
                mismatches.push(format!(
                    "{target} GET {path}: got {} {} {}",
                    response.status, response.content_type, response.body
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": path,
                "case": "nominal-status-headers-body",
                "pass": pass,
            }));
        }

        let capabilities_header =
            credential_header(scheme_for("/api/v0/capabilities").unwrap_or("any"));
        let capabilities = crate::route_http_request(
            "GET",
            "/api/v0/capabilities",
            Some(capabilities_header),
            "",
            &state,
        )
        .await
        .expect("route response");
        let expected_version = if target == "slskdn" {
            "\"version\":\"slskdn/1.0.0+dht+mesh+swarm\""
        } else {
            "\"version\":\"slskr/0.0.0+dht+mesh+swarm\""
        };
        let capabilities_pass = capabilities.status == "200 OK"
            && capabilities.content_type == "application/json"
            && capabilities.body.contains(expected_version);
        if !capabilities_pass {
            mismatches.push(format!(
                "{target} GET /api/v0/capabilities: got {} {} {}",
                capabilities.status, capabilities.content_type, capabilities.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/capabilities",
            "case": "nominal-status-headers-body",
            "pass": capabilities_pass,
        }));

        // This differential configures SLSKR_API_TOKEN, so "session
        // enabled" (credentials configured) is genuinely `true` here --
        // unlike the bare-default fixture `read_only_api_routes_return_
        // contract_shapes` uses, which configures no API token at all
        // and correctly sees `false`. Both are real, config-dependent
        // outcomes, not a contradiction.
        let session_enabled_header =
            credential_header(scheme_for("/api/v0/session/enabled").unwrap_or("any"));
        let session_enabled = crate::route_http_request(
            "GET",
            "/api/v0/session/enabled",
            Some(session_enabled_header),
            "",
            &state,
        )
        .await
        .expect("route response");
        let session_enabled_pass = session_enabled.status == "200 OK"
            && session_enabled.content_type == "application/json"
            && session_enabled.body == "true";
        if !session_enabled_pass {
            mismatches.push(format!(
                "{target} GET /api/v0/session/enabled: got {} {} {}",
                session_enabled.status, session_enabled.content_type, session_enabled.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/session/enabled",
            "case": "nominal-status-headers-body",
            "pass": session_enabled_pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("read_only_routes_have_real_contract_shapes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api nominal-shape mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
