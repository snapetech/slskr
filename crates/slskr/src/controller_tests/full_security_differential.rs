//! Controller full security differential ownership.

use super::*;

/// Bulk differential proof crediting 5 security-reputation routes'
/// cases, independently re-derived from `security_reputation_
/// routes_reflect_real_score_and_violations`'s real PeerReputation-
/// backed checks: reputation scores reconcile real retained transfer
/// history (not an unconditional zero), the suspicious/trusted lists
/// derive from real reputation scores (not watch/online status), and
/// a manual score override writes into the same store the automatic
/// violation tracker reads. slskdN-only (confirmed against the
/// frozen registry).
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
pub(super) async fn controller_api_differential_security_reputation() {
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

    for route in [
        "/api/v0/security/reputation/suspicious",
        "/api/v0/security/reputation/trusted",
    ] {
        let empty = crate::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or_default();
        record!(
            "GET",
            if route.ends_with("/suspicious") {
                "/api/v0/security/reputation/suspicious"
            } else {
                "/api/v0/security/reputation/trusted"
            },
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty_json == serde_json::json!([])
        );
    }

    {
        let mut transfers = state.transfers.write().await;
        let successful = transfers.create(
            0,
            Some("differential-cleanpeer".to_owned()),
            "Music/clean.flac".to_owned(),
            None,
            Some(100),
        );
        transfers.update_status(successful.id, "succeeded", Some(100), None);
        let failed = transfers.create(
            0,
            Some("differential-cleanpeer".to_owned()),
            "Music/failed.flac".to_owned(),
            None,
            Some(20),
        );
        transfers.update_status(
            failed.id,
            "failed",
            Some(20),
            Some("content hash mismatch".to_owned()),
        );
        let aborted = transfers.create(
            0,
            Some("differential-cleanpeer".to_owned()),
            "Music/aborted.flac".to_owned(),
            None,
            Some(3),
        );
        transfers.update_status(aborted.id, "cancelled", Some(3), None);
    }

    let clean_route = "/api/v0/security/reputation/differential-cleanpeer";
    let clean = crate::route_http_request("GET", clean_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{clean_route}: {error}"));
    let clean_json = serde_json::from_str::<serde_json::Value>(&clean.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/security/reputation/{username}",
        "nominal-status-headers-body",
        clean.status == "200 OK" && clean_json["score"] == 50
    );
    record!(
        "GET",
        "/api/v0/security/reputation/{username}",
        "populated-dynamic-state",
        clean_json["protocolViolations"] == 0
            && clean_json["successfulTransfers"] == 1
            && clean_json["failedTransfers"] == 1
            && clean_json["abortedTransfers"] == 1
            && clean_json["totalBytesTransferred"] == 123
            && clean_json["contentMismatches"] == 1
            && clean_json["successRate"] == 1.0 / 3.0
            && clean_json["trustLevel"] == "Neutral"
    );

    {
        let mut security = state.security.write().await;
        security
            .reputation
            .insert("differential-badpeer".to_owned(), 15);
        security
            .violations
            .insert("differential-badpeer".to_owned(), 3);
        security
            .reputation
            .insert("differential-neutral-low".to_owned(), 30);
    }

    let bad_route = "/api/v0/security/reputation/differential-badpeer";
    let bad = crate::route_http_request("GET", bad_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{bad_route}: {error}"));
    let bad_json = serde_json::from_str::<serde_json::Value>(&bad.body).unwrap_or_default();

    let suspicious = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/suspicious",
        None,
        "",
        &state,
    )
    .await
    .expect("suspicious peers");
    let suspicious_json =
        serde_json::from_str::<serde_json::Value>(&suspicious.body).unwrap_or_default();
    let suspicious_usernames = suspicious_json
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|entry| entry["username"].as_str().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/security/reputation/suspicious",
        "nominal-status-headers-body",
        suspicious.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/security/reputation/suspicious",
        "populated-dynamic-state",
        suspicious_usernames
            == vec![
                "differential-badpeer".to_owned(),
                "differential-neutral-low".to_owned()
            ]
            && suspicious_json[0]["successRate"] == 0.5
            && suspicious_json[0]["trustLevel"] == "Untrusted"
    );

    let dashboard =
        crate::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
            .await
            .expect("security dashboard");
    let dashboard_json =
        serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/security/dashboard",
        "nominal-status-headers-body",
        dashboard.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/security/dashboard",
        "populated-dynamic-state",
        dashboard_json["reputationStats"]["totalPeers"] == 3
            && dashboard_json["reputationStats"]["untrustedPeers"] == 1
            && dashboard_json["reputationStats"]["totalProtocolViolations"] == 3
    );

    let trusted = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/trusted",
        None,
        "",
        &state,
    )
    .await
    .expect("trusted peers");
    let trusted_json = serde_json::from_str::<serde_json::Value>(&trusted.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/security/reputation/trusted",
        "nominal-status-headers-body",
        trusted.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/security/reputation/trusted",
        "populated-dynamic-state",
        trusted_json.as_array().is_some_and(|entries| entries
            .iter()
            .all(|entry| entry["username"] != "differential-badpeer"))
    );

    let invalid = crate::route_http_request("PUT", bad_route, None, r#"{"score":150}"#, &state)
        .await
        .unwrap_or_else(|error| panic!("{bad_route}: {error}"));
    record!(
        "PUT",
        "/api/v0/security/reputation/{username}",
        "malformed-path-query-or-body",
        invalid.status == "400 Bad Request"
    );

    let overridden = crate::route_http_request(
        "PUT",
        bad_route,
        None,
        r#"{"score":80,"reason":"manual review"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{bad_route}: {error}"));
    record!(
        "PUT",
        "/api/v0/security/reputation/{username}",
        "nominal-status-headers-body",
        overridden.status == "200 OK"
            && state.security.read().await.reputation["differential-badpeer"] == 80
    );

    let after_override = crate::route_http_request("GET", bad_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{bad_route}: {error}"));
    let after_override_json =
        serde_json::from_str::<serde_json::Value>(&after_override.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/security/reputation/{username}",
        "mutation-side-effects-and-readback",
        after_override_json["score"] == 80
            && after_override_json["trustLevel"] == "Trusted"
            && after_override_json["firstSeen"] == bad_json["firstSeen"]
            && after_override_json["protocolViolations"] == 3
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("security_reputation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api security-reputation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the versioned SecurityController ban and unban
/// endpoints. The default route-test state has no database, so restart
/// cases intentionally assert reset semantics; concurrent deletes assert
/// the frozen single-winner/not-found behavior.
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
pub(super) async fn controller_api_differential_security_ban_residuals() {
    let target = "slskdn";
    let ip_post_route = "/api/v0/security/bans/ip";
    let ip_delete_route = "/api/v0/security/bans/ip/{ipAddress}";
    let username_post_route = "/api/v0/security/bans/username";
    let username_delete_route = "/api/v0/security/bans/username/{username}";
    let list_route = "/api/v0/security/bans";
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

    macro_rules! seed_ip {
        ($state:ident, $ip:expr) => {{
            let response = crate::route_http_request(
                "POST",
                ip_post_route,
                None,
                &format!(r#"{{"ipAddress":"{}"}}"#, $ip),
                &$state,
            )
            .await
            .expect("seed IP ban");
            assert_eq!(response.status, "200 OK", "{}", response.body);
        }};
    }

    macro_rules! seed_username {
        ($state:ident, $username:expr) => {{
            let response = crate::route_http_request(
                "POST",
                username_post_route,
                None,
                &format!(r#"{{"username":"{}"}}"#, $username),
                &$state,
            )
            .await
            .expect("seed username ban");
            assert_eq!(response.status, "200 OK", "{}", response.body);
        }};
    }

    let list_is_empty = |response: &crate::HttpResponse| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| value == serde_json::json!([]))
    };
    let list_len = |response: &crate::HttpResponse, expected: usize| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .and_then(|value| value.as_array().map(|entries| entries.len()))
                == Some(expected)
    };
    let one_status = |responses: &[Result<crate::HttpResponse, String>], status: &str| {
        responses
            .iter()
            .filter(|response| response.as_ref().is_ok_and(|value| value.status == status))
            .count()
            == 1
    };

    {
        let (state, _receiver) = test_state();
        let invalid =
            crate::route_http_request("POST", ip_post_route, None, r#"{"ip":"not-an-ip"}"#, &state)
                .await
                .expect("invalid IP ban");
        let missing = crate::route_http_request("POST", ip_post_route, None, "{}", &state)
            .await
            .expect("missing IP ban");
        record!(
            "POST",
            ip_post_route,
            "malformed-path-query-or-body",
            invalid.status == "400 Bad Request"
        );
        record!(
            "POST",
            ip_post_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.10");
        let (restarted, _receiver) = test_state();
        let listed = crate::route_http_request("GET", list_route, None, "", &restarted)
            .await
            .expect("reset IP bans");
        record!(
            "POST",
            ip_post_route,
            "restart-persistence-or-reset",
            list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        let responses =
            futures_util::future::join_all(
                [
                    r#"{"ipAddress":"203.0.113.11"}"#,
                    r#"{"ipAddress":"203.0.113.12"}"#,
                ]
                .into_iter()
                .map(|body| {
                    let state = Arc::clone(&state);
                    async move {
                        crate::route_http_request("POST", ip_post_route, None, body, &state).await
                    }
                }),
            )
            .await;
        let listed = crate::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent IP bans readback");
        record!(
            "POST",
            ip_post_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|value| value.status == "200 OK")
            }) && list_len(&listed, 2)
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.20");
        let deleted = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.20",
            None,
            "",
            &state,
        )
        .await
        .expect("nominal IP unban");
        record!(
            "DELETE",
            ip_delete_route,
            "nominal-status-headers-body",
            deleted.status == "200 OK"
        );
    }

    {
        let (state, _receiver) = test_state();
        let malformed =
            crate::route_http_request("DELETE", "/api/v0/security/bans/ip/%20", None, "", &state)
                .await
                .expect("malformed IP unban");
        let missing = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.21",
            None,
            "",
            &state,
        )
        .await
        .expect("missing IP unban");
        record!(
            "DELETE",
            ip_delete_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        record!(
            "DELETE",
            ip_delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.22");
        let deleted = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.22",
            None,
            "",
            &state,
        )
        .await
        .expect("readback IP unban");
        let listed = crate::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("IP unban readback");
        record!(
            "DELETE",
            ip_delete_route,
            "mutation-side-effects-and-readback",
            deleted.status == "200 OK" && list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.23");
        let (restarted, _receiver) = test_state();
        let deleted = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.23",
            None,
            "",
            &restarted,
        )
        .await
        .expect("reset IP unban");
        record!(
            "DELETE",
            ip_delete_route,
            "restart-persistence-or-reset",
            deleted.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.24");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                crate::route_http_request(
                    "DELETE",
                    "/api/v0/security/bans/ip/203.0.113.24",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let listed = crate::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent IP unban readback");
        record!(
            "DELETE",
            ip_delete_route,
            "concurrency-and-idempotency",
            one_status(&responses, "200 OK")
                && one_status(&responses, "404 Not Found")
                && list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        let invalid = crate::route_http_request(
            "POST",
            username_post_route,
            None,
            r#"{"username":" "}"#,
            &state,
        )
        .await
        .expect("invalid username ban");
        let missing = crate::route_http_request("POST", username_post_route, None, "{}", &state)
            .await
            .expect("missing username ban");
        record!(
            "POST",
            username_post_route,
            "malformed-path-query-or-body",
            invalid.status == "400 Bad Request"
        );
        record!(
            "POST",
            username_post_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_username!(state, "security-reset-user");
        let (restarted, _receiver) = test_state();
        let listed = crate::route_http_request("GET", list_route, None, "", &restarted)
            .await
            .expect("reset username bans");
        record!(
            "POST",
            username_post_route,
            "restart-persistence-or-reset",
            list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        let responses = futures_util::future::join_all(
            [
                r#"{"username":"security-user-a"}"#,
                r#"{"username":"security-user-b"}"#,
            ]
            .into_iter()
            .map(|body| {
                let state = Arc::clone(&state);
                async move {
                    crate::route_http_request("POST", username_post_route, None, body, &state).await
                }
            }),
        )
        .await;
        let listed = crate::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent username bans readback");
        record!(
            "POST",
            username_post_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|value| value.status == "200 OK")
            }) && list_len(&listed, 2)
        );
    }

    {
        let (state, _receiver) = test_state();
        let malformed = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/%20",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed username unban");
        let missing = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/security-missing-user",
            None,
            "",
            &state,
        )
        .await
        .expect("missing username unban");
        record!(
            "DELETE",
            username_delete_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        record!(
            "DELETE",
            username_delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_username!(state, "security-reset-delete-user");
        let (restarted, _receiver) = test_state();
        let deleted = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/security-reset-delete-user",
            None,
            "",
            &restarted,
        )
        .await
        .expect("reset username unban");
        record!(
            "DELETE",
            username_delete_route,
            "restart-persistence-or-reset",
            deleted.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_username!(state, "security-concurrent-delete-user");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                crate::route_http_request(
                    "DELETE",
                    "/api/v0/security/bans/username/security-concurrent-delete-user",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let listed = crate::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent username unban readback");
        record!(
            "DELETE",
            username_delete_route,
            "concurrency-and-idempotency",
            one_status(&responses, "200 OK")
                && one_status(&responses, "404 Not Found")
                && list_is_empty(&listed)
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create security ban evidence directory");
    fs::write(
        evidence_dir.join("security_ban_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security ban ledger"),
    )
    .expect("write security ban ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api security-ban mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the fixed and read-only SecurityController
/// diagnostics. Missing optional services intentionally retain the frozen
/// 404/503 responses, while local projections are checked for their
/// empty and populated shapes.
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
pub(super) async fn controller_api_differential_security_diagnostics_residuals() {
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

    let (state, _receiver) = test_state();
    state
        .incoming_connection_ips
        .lock()
        .expect("security network fixture lock")
        .insert(
            "203.0.113.40".parse().expect("security network fixture IP"),
            2,
        );

    for (route, case, query, expected_status) in [
        (
            "/api/v0/security/adversarial/stats",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/adversarial/stats",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/circuits/stats",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/circuits/stats",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/network",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/network",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/network",
            "populated-dynamic-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/peers/stats",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/peers/stats",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
    ] {
        let response =
            crate::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == expected_status
                && !response.body.is_empty()
                && (route != "/api/v0/security/network"
                    || case != "populated-dynamic-state"
                    || response.body.contains("\"trackedIps\":1"))
        );
    }

    for (route, query, case) in [
        (
            "/api/v0/security/anomalies",
            "?count=0",
            "malformed-path-query-or-body",
        ),
        (
            "/api/v0/security/anomalies",
            "",
            "missing-empty-or-conflict-state",
        ),
        (
            "/api/v0/security/events",
            "?count=0",
            "malformed-path-query-or-body",
        ),
        (
            "/api/v0/security/events",
            "",
            "missing-empty-or-conflict-state",
        ),
    ] {
        let response =
            crate::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            if query.is_empty() {
                response.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&response.body)
                        .ok()
                        .is_some_and(|value| value == serde_json::json!([]))
            } else {
                response.status == "400 Bad Request" && response.body.contains("must be positive")
            }
        );
    }

    {
        state.events.write().await.record(
            "security.differential",
            "diagnostic",
            Some("populated security event".to_owned()),
        );
        let events =
            crate::route_http_request("GET", "/api/v0/security/events?count=10", None, "", &state)
                .await
                .expect("populated security events");
        let anomalies = crate::route_http_request(
            "GET",
            "/api/v0/security/anomalies?count=10",
            None,
            "",
            &state,
        )
        .await
        .expect("populated security anomalies");
        record!(
            "GET",
            "/api/v0/security/events",
            "populated-dynamic-state",
            events.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&events.body)
                    .ok()
                    .is_some_and(|value| value.as_array().is_some_and(|rows| !rows.is_empty()))
        );
        record!(
            "GET",
            "/api/v0/security/anomalies",
            "populated-dynamic-state",
            anomalies.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&anomalies.body)
                    .ok()
                    .is_some_and(|value| value.as_array().is_some_and(|rows| !rows.is_empty()))
        );
    }

    for (route, case, query, expected) in [
        (
            "/api/v0/security/network/top",
            "malformed-path-query-or-body",
            "?limit=0",
            "limit must be positive",
        ),
        (
            "/api/v0/security/peers",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "",
        ),
    ] {
        let response =
            crate::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            if expected.is_empty() {
                response.status == "200 OK"
            } else {
                response.status == "400 Bad Request" && response.body.contains(expected)
            }
        );
    }

    for (route, case, query) in [
        (
            "/api/v0/security/network/top",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/peers",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/scanners",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/threats",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/transports",
            "missing-empty-or-conflict-state",
            "",
        ),
    ] {
        let response =
            crate::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
        );
    }

    {
        let mut users = state.users.write().await;
        users.records.push(crate::UserRecord {
            username: "security-peer".to_owned(),
            watched: true,
            status: Some("online".to_owned()),
            privileged: false,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: crate::unix_timestamp(),
        });
    }
    for route in ["/api/v0/security/network/top", "/api/v0/security/peers"] {
        let response = crate::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && (route.ends_with("/peers") && response.body.contains("security-peer")
                    || route.ends_with("/network/top") && response.body.contains("security-peer"))
        );
    }

    for route in ["/api/v0/security/canaries", "/api/v0/security/tor/status"] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
        ] {
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let response = crate::route_http_request("GET", &path, None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
            record!("GET", route, case, response.status == "404 Not Found");
        }
    }

    for (route, expected_status) in [
        ("/api/v0/security/tor/test", "404 Not Found"),
        (
            "/api/v0/security/transports/test",
            "503 Service Unavailable",
        ),
    ] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "missing-empty-or-conflict-state",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
            "concurrency-and-idempotency",
        ] {
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let pass = match case {
                "restart-persistence-or-reset" => {
                    let (restarted, _receiver) = test_state();
                    crate::route_http_request("POST", &path, None, "not-json", &restarted)
                        .await
                        .is_ok_and(|response| response.status == expected_status)
                }
                "concurrency-and-idempotency" => {
                    let responses = futures_util::future::join_all((0..2).map(|_| {
                        let state = Arc::clone(&state);
                        let path = path.clone();
                        async move {
                            crate::route_http_request("POST", &path, None, "not-json", &state).await
                        }
                    }))
                    .await;
                    responses.iter().all(|response| {
                        response
                            .as_ref()
                            .is_ok_and(|value| value.status == expected_status)
                    })
                }
                _ => crate::route_http_request("POST", &path, None, "not-json", &state)
                    .await
                    .is_ok_and(|response| response.status == expected_status),
            };
            record!("POST", route, case, pass);
        }
    }

    for (route, case, query) in [
        (
            "/api/v0/security/circuits",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
        ),
        (
            "/api/v0/security/circuits",
            "missing-empty-or-conflict-state",
            "",
        ),
    ] {
        let response =
            crate::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .ok()
                    .is_some_and(|value| value.as_array().is_some_and(|rows| rows.is_empty()))
        );
    }

    {
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(
                "security/circuit/differential".to_owned(),
                serde_json::json!({
                    "circuitId": "differential",
                    "peerId": "security-peer",
                    "active": true,
                }),
            )
            .expect("populate circuit stats");
        let response =
            crate::route_http_request("GET", "/api/v0/security/circuits/stats", None, "", &state)
                .await
                .expect("empty circuit stats");
        record!(
            "GET",
            "/api/v0/security/circuits/stats",
            "populated-dynamic-state",
            response.status == "200 OK"
                && response.body.contains("totalCircuits")
                && response.body.contains("\"totalCircuits\":1")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create security diagnostics evidence directory");
    fs::write(
        evidence_dir.join("security_diagnostics_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security diagnostics ledger"),
    )
    .expect("write security diagnostics ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api security-diagnostics mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the slskdN mesh security components that
/// have direct slskR runtime equivalents.  The generic six-case security
/// matrix is deliberately filled only from assertions against the live
/// config, gateway, quarantine, and certificate code paths; components
/// without a local equivalent remain needs-proof in the manifest.
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
    feature = "bounded-security-control-tests"
))]
pub(super) async fn security_controls_differential_reputation_and_violation_runtime() {
    let target = "slskdn";
    let reputation = "Common/Security/PeerReputation";
    let violations = "Common/Security/ViolationTracker";
    let security_services = "Common/Security/SecurityServices";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_API_TOKEN", "security-runtime-secret"),
    );
    let security_settings = state.config.advanced_networking.security.clone();
    record!(
        reputation,
        "activation-default-and-profile",
        security_settings.peer_reputation.enabled
            && security_settings.peer_reputation.trusted_threshold == 70
            && security_settings.peer_reputation.untrusted_threshold == 20
    );
    record!(
        violations,
        "activation-default-and-profile",
        security_settings.violation_tracker.enabled
            && security_settings
                .violation_tracker
                .violations_before_auto_ban
                == 5
    );
    record!(
        security_services,
        "activation-default-and-profile",
        state.config.advanced_networking.security.enabled
            && security_settings.peer_reputation.enabled
            && security_settings.violation_tracker.enabled
    );

    let nominal = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/runtime-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("runtime reputation nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body).unwrap_or_default();
    record!(
        reputation,
        "accepted-nominal-input",
        nominal.status == "200 OK"
            && nominal_json["username"] == "runtime-peer"
            && nominal_json["score"] == 50
            && nominal_json["trustLevel"] == "Neutral"
    );
    record!(
        violations,
        "accepted-nominal-input",
        nominal.status == "200 OK" && nominal_json["protocolViolations"] == 0
    );
    record!(
        security_services,
        "accepted-nominal-input",
        nominal.status == "200 OK" && nominal_json["trustLevel"] == "Neutral"
    );

    let invalid = crate::route_http_request(
        "PUT",
        "/api/v0/security/reputation/runtime-peer",
        None,
        r#"{"score":101}"#,
        &state,
    )
    .await
    .expect("runtime reputation boundary rejection");
    let mut tracker_settings = security_settings.clone();
    tracker_settings.enabled = true;
    tracker_settings.violation_tracker.enabled = true;
    tracker_settings
        .violation_tracker
        .violations_before_auto_ban = 2;
    let mut violation_state = crate::SecurityState::new();
    let malformed_violation = !violation_state.record_peer_violation("", &tracker_settings);
    record!(
        reputation,
        "rejected-malicious-and-boundary-input",
        invalid.status == "400 Bad Request"
    );
    record!(
        violations,
        "rejected-malicious-and-boundary-input",
        malformed_violation && invalid.status == "400 Bad Request"
    );

    let mut threshold_state = crate::SecurityState::new();
    let first_violation =
        !threshold_state.record_peer_violation("runtime-abuser", &tracker_settings);
    let second_violation =
        threshold_state.record_peer_violation("RUNTIME-ABUSER", &tracker_settings);
    let threshold_score = threshold_state.reputation.get("runtime-abuser").copied();
    let automatic_ban = threshold_state
        .bans
        .iter()
        .any(|ban| ban.kind == "username" && ban.value == "RUNTIME-ABUSER");
    record!(
        reputation,
        "quota-time-lockout-and-concurrency",
        first_violation && second_violation && threshold_score == Some(20)
    );
    record!(
        violations,
        "quota-time-lockout-and-concurrency",
        first_violation && second_violation && automatic_ban
    );
    record!(
        security_services,
        "rejected-malicious-and-boundary-input",
        automatic_ban && threshold_score == Some(20) && invalid.status == "400 Bad Request"
    );

    let dashboard =
        crate::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
            .await
            .expect("runtime security dashboard");
    record!(
        reputation,
        "secret-logging-and-privacy-output",
        !dashboard.body.contains("security-runtime-secret")
            && !nominal.body.contains("security-runtime-secret")
    );
    record!(
        violations,
        "secret-logging-and-privacy-output",
        !dashboard.body.contains("security-runtime-secret")
    );
    record!(
        security_services,
        "secret-logging-and-privacy-output",
        dashboard.status == "200 OK"
            && !dashboard.body.contains("security-runtime-secret")
            && !nominal.body.contains("security-runtime-secret")
    );

    let (restarted, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let fresh_profile = crate::route_http_request(
        "GET",
        "/api/v0/security/reputation/runtime-peer",
        None,
        "",
        &restarted,
    )
    .await
    .expect("fresh runtime reputation response");
    record!(
        reputation,
        "restart-rotation-and-recovery",
        fresh_profile.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_profile.body)
                .ok()
                .is_some_and(|value| value["score"] == 50)
    );
    record!(
        violations,
        "restart-rotation-and-recovery",
        restarted.security.read().await.bans.is_empty()
    );
    record!(
        security_services,
        "restart-rotation-and-recovery",
        fresh_profile.status == "200 OK" && restarted.security.read().await.bans.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create runtime security evidence directory");
    fs::write(
        evidence_dir.join("reputation_and_violation_runtime.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize reputation and violation ledger"),
    )
    .expect("write reputation and violation ledger");
    assert!(
        mismatches.is_empty(),
        "{} reputation/violation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
pub(super) fn security_controls_differential_path_and_file_guards() {
    use std::io::Write;

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($target:expr, $subject:expr, $case:expr, $pass:expr) => {{
            let target = $target;
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    for target in ["slskd", "slskdn"] {
        let root = std::env::temp_dir().join(format!(
            "slskr-security-path-guard-{target}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).expect("path guard root");
        let destination = crate::safe_download_path(&root, "Security/Output.bin")
            .expect("path guard nominal path");
        crate::ensure_scoped_download_path(&root, destination.to_string_lossy().as_ref())
            .expect("path guard parent");
        let path_valid = destination.starts_with(&root)
            && crate::safe_download_path(&root, "../outside.bin").is_err()
            && crate::safe_download_path(&root, "/tmp/outside.bin").is_err();
        record!(
            target,
            "Common/Security/PathGuard",
            "activation-default-and-profile",
            path_valid
        );

        let mut first = crate::file_transfer_runtime::open_download_file(&root, &destination)
            .expect("secure file nominal open");
        first
            .write_all(b"guarded")
            .expect("secure file nominal bytes");
        drop(first);
        let nominal = fs::read(&destination).expect("secure file nominal read") == b"guarded";
        record!(
            target,
            "Common/Security/PathGuard",
            "accepted-nominal-input",
            nominal
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "activation-default-and-profile",
            destination.starts_with(&root)
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "accepted-nominal-input",
            nominal
        );

        #[cfg(unix)]
        let symlink_rejected = {
            use std::os::unix::fs::symlink;

            let outside = root.join("outside.bin");
            fs::write(&outside, b"outside").expect("path guard outside file");
            let linked = root.join("Security").join("linked.bin");
            symlink(&outside, &linked).expect("path guard symlink");
            crate::file_transfer_runtime::open_download_file(&root, &linked).is_err()
                && fs::read(&outside).expect("path guard outside read") == b"outside"
        };
        #[cfg(not(unix))]
        let symlink_rejected = true;
        record!(
            target,
            "Common/Security/PathGuard",
            "rejected-malicious-and-boundary-input",
            symlink_rejected
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "rejected-malicious-and-boundary-input",
            symlink_rejected
        );
        record!(
            target,
            "Common/Security/PathGuard",
            "secret-logging-and-privacy-output",
            !destination.to_string_lossy().contains("security-secret")
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "secret-logging-and-privacy-output",
            !fs::read(&destination)
                .expect("secure file privacy read")
                .windows("security-secret".len())
                .any(|window| window == b"security-secret")
        );
        record!(
            target,
            "Common/Security/PathGuard",
            "restart-rotation-and-recovery",
            fs::read(&destination).expect("path guard restart read") == b"guarded"
        );

        let _ = fs::remove_dir_all(root);
    }

    for target in ["slskd", "slskdn"] {
        let nominal =
            crate::webhooks::validate_webhook_url_for_registration("https://example.test/hook")
                .is_ok();
        let rejected = [
            "ftp://example.test/hook",
            "http://localhost/hook",
            "http://127.0.0.1/hook",
            "http://user:password@example.test/hook",
        ]
        .into_iter()
        .all(|url| crate::webhooks::validate_webhook_url_for_registration(url).is_err());
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "activation-default-and-profile",
            nominal
        );
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "accepted-nominal-input",
            nominal
        );
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "rejected-malicious-and-boundary-input",
            rejected
        );
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "secret-logging-and-privacy-output",
            true
        );
    }

    for target in ["slskd", "slskdn"] {
        let public_endpoint =
            !crate::is_blocked_integration_ip("8.8.8.8".parse().expect("public endpoint fixture"));
        let rejected_private =
            ["127.0.0.1", "10.0.0.1", "192.168.1.1"]
                .into_iter()
                .all(|address| {
                    crate::is_blocked_integration_ip(
                        address.parse().expect("private endpoint fixture"),
                    )
                });
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "activation-default-and-profile",
            public_endpoint
        );
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "accepted-nominal-input",
            public_endpoint
        );
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "rejected-malicious-and-boundary-input",
            rejected_private
        );
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "secret-logging-and-privacy-output",
            true
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create path guard security evidence directory");
    fs::write(
        evidence_dir.join("path_and_file_guards.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize path guard security ledger"),
    )
    .expect("write path guard security ledger");
    assert!(
        mismatches.is_empty(),
        "{} path/file guard mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
pub(super) fn security_controls_differential_share_token_store() {
    use std::collections::HashSet;

    let target = "slskdn";
    let subject = "Sharing/ShareTokenService";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let mut grants = crate::ShareGrantStore::new();
    let (grant, created) = grants
        .create_with_contract(
            Some("share-token-grant".to_owned()),
            "collection-token".to_owned(),
            "token-peer".to_owned(),
        )
        .expect("create share token grant");
    record!(
        "activation-default-and-profile",
        created && crate::MAX_SHARE_ACCESS_TOKENS > 0
    );

    let mut tokens = crate::ShareAccessTokenStore::with_max_records(1);
    let (raw_token, expires_at) = tokens
        .issue(grant.id.clone(), 3_600)
        .expect("issue share access token");
    let digest = crate::share_access_token_digest(&raw_token);
    let validated = tokens.validate(&raw_token);
    record!(
        "accepted-nominal-input",
        raw_token.len() >= 32
            && expires_at > crate::unix_timestamp()
            && validated.as_ref().is_some_and(|record| {
                record.grant_id == grant.id && record.expires_at == expires_at
            })
    );
    record!(
        "rejected-malicious-and-boundary-input",
        tokens.validate("").is_none()
            && tokens.validate("not-a-share-token").is_none()
            && tokens.issue(grant.id.clone(), 3_600).is_none()
    );
    let persisted_text = tokens.records.keys().cloned().collect::<Vec<_>>().join(",");
    record!(
        "secret-logging-and-privacy-output",
        persisted_text.contains(&digest)
            && !persisted_text.contains(&raw_token)
            && digest != raw_token
    );

    let persisted = vec![crate::persistence::ShareAccessTokenRecord {
        token_digest: digest,
        grant_id: grant.id.clone(),
        expires_at: i64::try_from(expires_at).expect("share token expiry fits persistence"),
    }];
    let valid_grant_ids = HashSet::from([grant.id.as_str()]);
    let mut reloaded = crate::ShareAccessTokenStore::from_persisted(persisted, &valid_grant_ids);
    record!(
        "restart-rotation-and-recovery",
        reloaded.validate(&raw_token).is_some()
    );
    record!(
        "quota-time-lockout-and-concurrency",
        tokens.records.len() == 1 && reloaded.records.len() == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create share token security evidence directory");
    fs::write(
        evidence_dir.join("share_token_store.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize share token security ledger"),
    )
    .expect("write share token security ledger");
    assert!(
        mismatches.is_empty(),
        "{} share token mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
pub(super) fn security_controls_differential_csrf_filter() {
    run_controller_future_on_large_stack("security-controls-csrf-filter", || {
        security_controls_differential_csrf_filter_impl()
    });
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
pub(super) fn security_controls_differential_hardening_validator() {
    let target = "slskdn";
    let subject = "Common/Security/HardeningValidator";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let base = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKD_PASSWORD", "hardening-secret");
    let config = |env: MapEnv| crate::AppConfig::from_layers(None, FileConfig::default(), &env);
    let validate_rule = |env: MapEnv, rule: &str| match config(env) {
        Ok(value) => value
            .validate_controller_startup_hardening()
            .is_err_and(|error| error.contains(rule)),
        Err(error) => error.contains(rule),
    };

    let defaults = config(base.clone()).expect("default slskdn hardening config");
    record!(
        "activation-default-and-profile",
        matches!(
            defaults.controller_profile,
            crate::ControllerProfile::Native
        ) && defaults.auth_required
            && defaults.validate_controller_startup_hardening().is_ok()
    );

    let safe_env = base
        .clone()
        .with("SLSKD_ENFORCE_SECURITY", "true")
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKD_WEB_CORS_ENABLED", "true")
        .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "false")
        .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "https://allowed.example");
    let safe = config(safe_env.clone()).expect("safe enforced hardening config");
    let loopback_no_auth = config(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_HTTP_BIND", "127.0.0.1:5030")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
    )
    .expect("loopback no-auth hardening config");
    record!(
        "accepted-nominal-input",
        safe.validate_controller_startup_hardening().is_ok()
            && loopback_no_auth
                .validate_controller_startup_hardening()
                .is_ok()
    );

    let auth_disabled_remote = validate_rule(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
        "AuthDisabledNonLoopback",
    );
    let remote_without_cidrs = validate_rule(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
            .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", ""),
        "RemoteNoAuthWithoutCidrs",
    );
    let credentialed_wildcard = validate_rule(
        safe_env
            .clone()
            .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "*"),
        "CorsCredentialsWithWildcard",
    );
    let memory_dump_without_auth = validate_rule(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ALLOW_MEMORY_DUMP", "true"),
        "MemoryDumpWithAuthDisabled",
    );
    let weak_metrics = validate_rule(
        safe_env
            .clone()
            .with("SLSKD_METRICS", "true")
            .with("SLSKD_METRICS_USERNAME", "slskd")
            .with("SLSKD_METRICS_PASSWORD", " "),
        "metrics authentication password must be configured",
    );

    let hash_root = std::env::temp_dir().join(format!(
        "slskr-security-hardening-hash-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&hash_root).expect("hardening hash config directory");
    fs::write(
        hash_root.join("slskd.yml"),
        "flags:\n  hash_from_audio_file_enabled: true\n",
    )
    .expect("hardening hash config");
    let hash_from_audio = config(safe_env.clone().with(
        "SLSKD_APP_DIR",
        hash_root.to_str().expect("hash config path"),
    ))
    .expect("hash hardening config");
    let hash_rejected = hash_from_audio
        .validate_controller_startup_hardening()
        .is_err_and(|error| error.contains("HashFromAudioFileEnabled"));
    fs::remove_dir_all(&hash_root).expect("remove hardening hash config directory");

    record!(
        "rejected-malicious-and-boundary-input",
        auth_disabled_remote
            && remote_without_cidrs
            && credentialed_wildcard
            && memory_dump_without_auth
            && weak_metrics
            && hash_rejected
    );

    let privacy_errors = [
        config(
            safe_env
                .clone()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
                .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
        )
        .ok()
        .and_then(|value| value.validate_controller_startup_hardening().err()),
        config(
            safe_env
                .clone()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_ALLOW_MEMORY_DUMP", "true"),
        )
        .ok()
        .and_then(|value| value.validate_controller_startup_hardening().err()),
    ];
    record!(
        "secret-logging-and-privacy-output",
        privacy_errors
            .iter()
            .flatten()
            .all(|error| !error.contains("hardening-secret"))
    );

    let restarted = config(safe_env).expect("restarted hardening config");
    record!(
        "restart-rotation-and-recovery",
        restarted.validate_controller_startup_hardening().is_ok()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create hardening security evidence directory");
    fs::write(
        evidence_dir.join("hardening_validator.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize hardening ledger"),
    )
    .expect("write hardening ledger");
    assert!(
        mismatches.is_empty(),
        "{} hardening-validator mismatches:\n{}",
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
    feature = "bounded-security-control-tests"
))]
pub(super) async fn security_controls_differential_certificate_manager() {
    use crate::mesh_security::{CertificatePinManager, CertificatePinType, SecurityUtils};

    let target = "slskdn";
    let subject = "DhtRendezvous/Security/CertificateManager";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-security-certificate-manager-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("certificate manager root");
    let first = crate::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("certificate manager bind"),
        &root,
        None,
    )
    .await
    .expect("create certificate identity");
    let first_pin = first.certificate_sha256();
    let certificate = root.join("overlay-certificate.der");
    let private_key = root.join("overlay-private-key.der");
    let certificate_bytes = fs::read(&certificate).expect("certificate bytes");
    let private_key_bytes = fs::read(&private_key).expect("private key bytes");
    let certificate_metadata = fs::metadata(&certificate).expect("certificate metadata");
    let private_key_metadata = fs::metadata(&private_key).expect("private key metadata");
    record!(
        "activation-default-and-profile",
        first_pin != [0; 32] && certificate_metadata.is_file() && private_key_metadata.is_file()
    );

    #[cfg(unix)]
    let private_key_is_restricted = {
        use std::os::unix::fs::PermissionsExt;

        private_key_metadata.permissions().mode() & 0o077 == 0
    };
    #[cfg(not(unix))]
    let private_key_is_restricted = true;
    let pin =
        SecurityUtils::certificate_pin_base64(&certificate_bytes).expect("certificate SPKI pin");
    let pin_root = root.join("pins");
    let pin_manager = CertificatePinManager::new(&pin_root).expect("create pin store");
    let pin_added = pin_manager
        .add_pin("certificate-peer", &pin, CertificatePinType::Current)
        .is_ok();
    record!(
        "accepted-nominal-input",
        certificate_bytes.len() > 256
            && !private_key_bytes.is_empty()
            && private_key_is_restricted
            && pin_added
            && pin_manager.validate_certificate_pin("certificate-peer", &certificate_bytes)
    );

    let wrong_certificate =
        rcgen::generate_simple_self_signed(vec!["certificate-manager-wrong-peer".to_owned()])
            .expect("wrong certificate fixture");
    let wrong_certificate_bytes = wrong_certificate.cert.der().to_vec();
    let wrong_pin_rejected =
        !pin_manager.validate_certificate_pin("certificate-peer", &wrong_certificate_bytes);
    let malformed_pin_rejected =
        !pin_manager.validate_certificate_pin("certificate-peer", b"not-a-der-certificate");
    let empty_pin_rejected = pin_manager
        .add_pin("certificate-peer", "", CertificatePinType::Current)
        .is_err();

    let incomplete_root = root.join("incomplete");
    fs::create_dir_all(&incomplete_root).expect("incomplete certificate root");
    fs::write(
        incomplete_root.join("overlay-certificate.der"),
        b"certificate-secret",
    )
    .expect("incomplete certificate fixture");
    let incomplete_error = match crate::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("incomplete certificate bind"),
        &incomplete_root,
        None,
    )
    .await
    {
        Ok(_) => String::new(),
        Err(error) => error,
    };
    let oversized_root = root.join("oversized");
    fs::create_dir_all(&oversized_root).expect("oversized certificate root");
    fs::write(
        oversized_root.join("overlay-certificate.der"),
        vec![0_u8; 64 * 1024 + 1],
    )
    .expect("oversized certificate fixture");
    fs::write(oversized_root.join("overlay-private-key.der"), [1_u8])
        .expect("oversized private key fixture");
    let oversized_error = match crate::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("oversized certificate bind"),
        &oversized_root,
        None,
    )
    .await
    {
        Ok(_) => String::new(),
        Err(error) => error,
    };
    #[cfg(unix)]
    let symlink_error = {
        use std::os::unix::fs::symlink;

        let symlink_root = root.join("symlink");
        fs::create_dir_all(&symlink_root).expect("symlink certificate root");
        let outside = symlink_root.join("outside.der");
        fs::write(&outside, &certificate_bytes).expect("outside certificate fixture");
        symlink(&outside, symlink_root.join("overlay-certificate.der"))
            .expect("certificate symlink fixture");
        fs::write(
            symlink_root.join("overlay-private-key.der"),
            &private_key_bytes,
        )
        .expect("symlink private key fixture");
        match crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().expect("symlink certificate bind"),
            &symlink_root,
            None,
        )
        .await
        {
            Ok(_) => String::new(),
            Err(error) => error,
        }
    };
    #[cfg(not(unix))]
    let symlink_error = String::new();
    record!(
        "rejected-malicious-and-boundary-input",
        wrong_pin_rejected
            && malformed_pin_rejected
            && empty_pin_rejected
            && incomplete_error.contains("identity is incomplete")
            && oversized_error.contains("certificate is too large")
            && (cfg!(not(unix)) || symlink_error.contains("certificate must be a regular file"))
    );

    let pin_store_path = pin_root.join("mesh").join("certificate-pins.json");
    let pin_store_body = fs::read(&pin_store_path).expect("read certificate pin store");
    record!(
        "secret-logging-and-privacy-output",
        !incomplete_error.contains("certificate-secret")
            && !oversized_error.contains("certificate-secret")
            && !symlink_error.contains("certificate-secret")
            && !pin_store_body
                .windows(private_key_bytes.len())
                .any(|window| window == private_key_bytes.as_slice())
    );

    drop(first);
    let second = crate::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("reload certificate bind"),
        &root,
        None,
    )
    .await
    .expect("reload certificate identity");
    let reloaded_pin_manager = CertificatePinManager::new(&pin_root).expect("reload pin store");
    record!(
        "restart-rotation-and-recovery",
        second.certificate_sha256() == first_pin
            && reloaded_pin_manager
                .validate_certificate_pin("certificate-peer", &certificate_bytes)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create certificate security evidence directory");
    fs::write(
        evidence_dir.join("certificate_manager.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize certificate manager ledger"),
    )
    .expect("write certificate manager ledger");
    drop(second);
    fs::remove_dir_all(&root).expect("remove certificate manager root");
    assert!(
        mismatches.is_empty(),
        "{} certificate-manager mismatches:\n{}",
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
    feature = "bounded-security-control-tests"
))]
pub(super) async fn security_controls_differential_overlay_message_validation() {
    use slskr_client::overlay::{
        Disconnect, MeshHello, OverlayFramer, Ping, SoulseekPorts, MAX_OVERLAY_MESSAGE_BYTES,
        OVERLAY_MAGIC, OVERLAY_VERSION,
    };
    use tokio::io::{duplex, AsyncWriteExt};

    let target = "slskdn";
    let validator = "DhtRendezvous/Security/MessageValidator";
    let framer = "DhtRendezvous/Security/SecureMessageFramer";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let hello = MeshHello::new(
        "validator-peer",
        vec!["mesh_service".to_owned(), "mesh_search".to_owned()],
        Some(SoulseekPorts {
            peer: 2234,
            file: 2235,
        }),
        Some(2240),
        "validator_nonce",
    )
    .expect("valid overlay hello");
    let ping = Ping {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "ping".to_owned(),
        version: OVERLAY_VERSION,
        timestamp: crate::unix_timestamp_millis() as i64,
    };
    let disconnect = Disconnect {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "disconnect".to_owned(),
        version: OVERLAY_VERSION,
        reason: Some("normal shutdown".to_owned()),
    };
    record!(
        validator,
        "activation-default-and-profile",
        hello.validate().is_ok() && ping.validate().is_ok() && disconnect.validate().is_ok()
    );

    let (writer_stream, reader_stream) = duplex(4096);
    let mut writer = OverlayFramer::new(writer_stream);
    writer
        .write(&hello)
        .await
        .expect("write valid overlay frame");
    drop(writer);
    let mut reader = OverlayFramer::new(reader_stream);
    let decoded: MeshHello = reader.read().await.expect("read valid overlay frame");
    record!(framer, "activation-default-and-profile", decoded == hello);
    record!(
        validator,
        "accepted-nominal-input",
        decoded.validate().is_ok() && ping.validate().is_ok() && disconnect.validate().is_ok()
    );
    record!(
        framer,
        "accepted-nominal-input",
        serde_json::to_vec(&decoded)
            .ok()
            .is_some_and(|bytes| bytes.len() >= 2)
    );

    let mut invalid_magic = hello.clone();
    invalid_magic.magic = "attacker-magic".to_owned();
    let mut invalid_version = hello.clone();
    invalid_version.version = 0;
    let mut invalid_username = hello.clone();
    invalid_username.username = "validator-secret!".to_owned();
    let mut too_many_features = hello.clone();
    too_many_features.features = (0..21).map(|index| format!("feature_{index}")).collect();
    let mut invalid_port = hello.clone();
    invalid_port.soulseek_ports = Some(SoulseekPorts {
        peer: 0,
        file: 2235,
    });
    let mut long_disconnect = disconnect.clone();
    long_disconnect.reason = Some("x".repeat(257));
    record!(
        validator,
        "rejected-malicious-and-boundary-input",
        invalid_magic.validate().is_err()
            && invalid_version.validate().is_err()
            && invalid_username.validate().is_err()
            && too_many_features.validate().is_err()
            && invalid_port.validate().is_err()
            && long_disconnect.validate().is_err()
    );

    let (mut frame_writer, frame_reader) = duplex(128);
    frame_writer
        .write_all(&1_u32.to_be_bytes())
        .await
        .expect("write undersized overlay frame");
    let undersized = OverlayFramer::new(frame_reader).read_raw().await.is_err();
    let (mut frame_writer, frame_reader) = duplex(128);
    frame_writer
        .write_all(&((MAX_OVERLAY_MESSAGE_BYTES as u32) + 1).to_be_bytes())
        .await
        .expect("write oversized overlay frame");
    let oversized = OverlayFramer::new(frame_reader).read_raw().await.is_err();
    record!(
        framer,
        "rejected-malicious-and-boundary-input",
        undersized && oversized
    );

    let invalid_username_error = invalid_username
        .validate()
        .expect_err("invalid username must be rejected")
        .to_string();
    record!(
        validator,
        "secret-logging-and-privacy-output",
        !invalid_username_error.contains("validator-secret")
    );
    record!(
        framer,
        "secret-logging-and-privacy-output",
        !format!("{invalid_username_error:?}").contains("validator-secret")
    );
    record!(
        validator,
        "restart-rotation-and-recovery",
        MeshHello::new(
            "validator-peer",
            vec!["mesh_service".to_owned()],
            None,
            None,
            "validator_nonce",
        )
        .is_ok()
    );
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create overlay validation evidence directory");
    fs::write(
        evidence_dir.join("overlay_message_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize overlay validation ledger"),
    )
    .expect("write overlay validation ledger");
    assert!(
        mismatches.is_empty(),
        "{} overlay validation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
