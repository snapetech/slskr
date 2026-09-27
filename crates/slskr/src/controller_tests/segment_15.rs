/// Differential proof for the residual quarantine-jury controller cases.
/// These cases exercise empty and missing reads, malformed route tails,
/// real persisted request/verdict/acceptance/route mutations, and
/// concurrent idempotent operations against the same feature-state store.
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
async fn controller_api_differential_quarantine_jury_open_cases() {
    let target = "slskdn";
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

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let json_value = |response: &super::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let request_body = |request_id: &str, jurors: &[&str], min_votes: u64| {
        serde_json::json!({
            "requestId": request_id,
            "localReason": "residual differential quarantine",
            "jurors": jurors,
            "evidence": [{"type": "hash", "reference": format!("opaque-{request_id}")}],
            "minJurorVotes": min_votes,
        })
        .to_string()
    };

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/audit/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("quarantine audit malformed response");
        record!(
            "GET",
            "/api/v0/quarantine-jury/audit",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty =
            super::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
                .await
                .expect("quarantine audit empty response");
        let empty_json = json_value(&empty);
        record!(
            "GET",
            "/api/v0/quarantine-jury/audit",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && empty_json["requestCount"] == 0
                && empty_json["entries"].as_array().is_some_and(Vec::is_empty)
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/audit?staleAfterHours=1",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("quarantine audit runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/quarantine-jury/audit",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["requestCount"] == 0
                && runtime_json["entries"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("quarantine requests malformed response");
        // The collection route is exact; a single path segment is the
        // request-detail route, so this tail is a missing request rather
        // than a collection match.
        record!(
            "GET",
            "/api/v0/quarantine-jury/requests",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty =
            super::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
                .await
                .expect("quarantine requests empty response");
        let empty_json = json_value(&empty);
        record!(
            "GET",
            "/api/v0/quarantine-jury/requests",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty_json.as_array().is_some_and(Vec::is_empty)
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("quarantine requests runtime response");
        record!(
            "GET",
            "/api/v0/quarantine-jury/requests",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && json_value(&runtime).as_array().is_some_and(Vec::is_empty)
        );
    }

    for (route, suffix) in [
        (
            "/api/v0/quarantine-jury/requests/{requestId}",
            "requests/open-differential",
        ),
        (
            "/api/v0/quarantine-jury/requests/{requestId}/aggregate",
            "requests/open-differential/aggregate",
        ),
        (
            "/api/v0/quarantine-jury/requests/{requestId}/review",
            "requests/open-differential/review",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed_path = format!("/api/v0/quarantine-jury/{suffix}/extra");
        let malformed = super::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .expect("quarantine detail malformed response");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing_path = format!("/api/v0/quarantine-jury/{suffix}");
        let missing = super::route_http_request("GET", &missing_path, None, "", &state)
            .await
            .expect("quarantine detail missing response");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );

        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request("GET", &missing_path, None, "", &runtime_state)
            .await
            .expect("quarantine detail runtime response");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests/open-differential/release-package/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("quarantine release package malformed response");
        record!(
            "GET",
            "/api/v0/quarantine-jury/requests/{requestId}/release-package",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests/open-differential/release-package",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("quarantine release package runtime response");
        record!(
            "GET",
            "/api/v0/quarantine-jury/requests/{requestId}/release-package",
            "runtime-failure-and-timeout",
            runtime.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests/open-differential/routes/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("quarantine routes malformed response");
        record!(
            "GET",
            "/api/v0/quarantine-jury/requests/{requestId}/routes",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests/open-differential/routes",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("quarantine routes runtime response");
        record!(
            "GET",
            "/api/v0/quarantine-jury/requests/{requestId}/routes",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && json_value(&runtime).as_array().is_some_and(Vec::is_empty)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing =
            super::route_http_request("POST", "/api/v0/quarantine-jury/requests", None, "", &state)
                .await
                .expect("quarantine request missing response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-runtime-request", &["runtime-juror"], 1),
            &runtime_state,
        )
        .await
        .expect("quarantine request runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["isValid"] == true
                && runtime_json["request"]["requestId"] == "open-runtime-request"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-mutation-request", &["mutation-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine request mutation response");
        let listed =
            super::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
                .await
                .expect("quarantine request mutation readback");
        let listed_json = json_value(&listed);
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests",
            "mutation-side-effects-and-readback",
            created.status == "200 OK"
                && listed_json.as_array().is_some_and(|items| {
                    items
                        .iter()
                        .any(|item| item["requestId"] == "open-mutation-request")
                })
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-reset-request", &["reset-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine request reset seed");
        let (restarted_state, _restarted_receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("quarantine request reset response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests",
            "restart-persistence-or-reset",
            created.status == "200 OK"
                && json_value(&restarted).as_array().is_some_and(Vec::is_empty)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let concurrent_a = request_body("open-concurrent-a", &["concurrent-a"], 1);
        let concurrent_b = request_body("open-concurrent-b", &["concurrent-b"], 1);
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/quarantine-jury/requests",
                None,
                &concurrent_a,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/quarantine-jury/requests",
                None,
                &concurrent_b,
                &state
            )
        );
        let left = left.expect("left quarantine request concurrency response");
        let right = right.expect("right quarantine request concurrency response");
        let listed =
            super::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
                .await
                .expect("quarantine request concurrency readback");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests",
            "concurrency-and-idempotency",
            left.status == "200 OK"
                && right.status == "200 OK"
                && json_value(&listed)
                    .as_array()
                    .is_some_and(|items| items.len() == 2)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests/open-accept/accept-release-candidate/extra",
            None,
            "{}",
            &state,
        )
        .await
        .expect("quarantine accept malformed response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests/open-accept/accept-release-candidate",
            None,
            "{}",
            &runtime_state,
        )
        .await
        .expect("quarantine accept runtime response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
            "runtime-failure-and-timeout",
            runtime.status == "404 Not Found"
        );

        let (restarted_state, _restarted_receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests/open-accept/accept-release-candidate",
            None,
            "{}",
            &restarted_state,
        )
        .await
        .expect("quarantine accept restarted response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
            "restart-persistence-or-reset",
            restarted.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-accept-concurrent", &["accept-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine accept request seed");
        let verdict = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(
                "open-accept-concurrent",
                "accept-juror",
                "ReleaseCandidate",
            )
            .to_string(),
            &state,
        )
        .await
        .expect("quarantine accept verdict seed");
        let accept_path =
            "/api/v0/quarantine-jury/requests/open-accept-concurrent/accept-release-candidate";
        let (left, right) = tokio::join!(
            super::route_http_request("POST", accept_path, None, "{}", &state),
            super::route_http_request("POST", accept_path, None, "{}", &state)
        );
        let left = left.expect("left quarantine accept concurrency response");
        let right = right.expect("right quarantine accept concurrency response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
            "concurrency-and-idempotency",
            created.status == "200 OK"
                && verdict.status == "200 OK"
                && left.status == "200 OK"
                && right.status == "200 OK"
                && json_value(&left)["isAccepted"] == true
                && json_value(&right)["isAccepted"] == true
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-route-nominal", &["route-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine route request seed");
        let route = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests/open-route-nominal/routes",
            None,
            "{}",
            &state,
        )
        .await
        .expect("quarantine route nominal response");
        let route_json = json_value(&route);
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests/{requestId}/routes",
            "nominal-status-headers-body",
            created.status == "200 OK"
                && route.status == "400 Bad Request"
                && route_json["success"] == false
                && route_json["errorMessage"] == "Routing backend is not available."
        );

        let (restarted_state, _restarted_receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests/open-route-nominal/routes",
            None,
            "{}",
            &restarted_state,
        )
        .await
        .expect("quarantine route restarted response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests/{requestId}/routes",
            "restart-persistence-or-reset",
            restarted.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body(
                "open-route-concurrent",
                &["route-concurrent-a", "route-concurrent-b"],
                2,
            ),
            &state,
        )
        .await
        .expect("quarantine concurrent route request seed");
        let route_path = "/api/v0/quarantine-jury/requests/open-route-concurrent/routes";
        let route_body = r#"{"targetJurors":["route-concurrent-a"]}"#;
        let (left, right) = tokio::join!(
            super::route_http_request("POST", route_path, None, route_body, &state),
            super::route_http_request("POST", route_path, None, route_body, &state)
        );
        let left = left.expect("left quarantine route concurrency response");
        let right = right.expect("right quarantine route concurrency response");
        let routes = super::route_http_request("GET", route_path, None, "", &state)
            .await
            .expect("quarantine route concurrency readback");
        record!(
            "POST",
            "/api/v0/quarantine-jury/requests/{requestId}/routes",
            "concurrency-and-idempotency",
            created.status == "200 OK"
                && left.status == "400 Bad Request"
                && right.status == "400 Bad Request"
                && json_value(&routes)
                    .as_array()
                    .is_some_and(|items| items.len() == 2)
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-verdict-runtime", &["verdict-runtime"], 1),
            &runtime_state,
        )
        .await
        .expect("quarantine verdict runtime request seed");
        let verdict = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(
                "open-verdict-runtime",
                "verdict-runtime",
                "ReleaseCandidate",
            )
            .to_string(),
            &runtime_state,
        )
        .await
        .expect("quarantine verdict runtime response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            "runtime-failure-and-timeout",
            created.status == "200 OK" && verdict.status == "200 OK"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-verdict-mutation", &["verdict-mutation"], 1),
            &state,
        )
        .await
        .expect("quarantine verdict mutation request seed");
        let verdict = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(
                "open-verdict-mutation",
                "verdict-mutation",
                "UpholdQuarantine",
            )
            .to_string(),
            &state,
        )
        .await
        .expect("quarantine verdict mutation response");
        let aggregate = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests/open-verdict-mutation/aggregate",
            None,
            "",
            &state,
        )
        .await
        .expect("quarantine verdict mutation readback");
        let aggregate_json = json_value(&aggregate);
        record!(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            "mutation-side-effects-and-readback",
            created.status == "200 OK"
                && verdict.status == "200 OK"
                && aggregate_json["totalVerdicts"] == 1
                && aggregate_json["quorumReached"] == true
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(
                "open-verdict-restarted",
                "restarted-juror",
                "ReleaseCandidate",
            )
            .to_string(),
            &state,
        )
        .await
        .expect("quarantine verdict restarted response");
        record!(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            "restart-persistence-or-reset",
            missing.status == "400 Bad Request"
                && json_value(&missing)["errors"]
                    .as_array()
                    .is_some_and(|errors| errors.iter().any(|error| error == "Request not found."))
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body(
                "open-verdict-concurrent",
                &["verdict-concurrent-a", "verdict-concurrent-b"],
                2,
            ),
            &state,
        )
        .await
        .expect("quarantine concurrent verdict request seed");
        let concurrent_a = quarantine_signed_verdict_json(
            "open-verdict-concurrent",
            "verdict-concurrent-a",
            "ReleaseCandidate",
        )
        .to_string();
        let concurrent_b = quarantine_signed_verdict_json(
            "open-verdict-concurrent",
            "verdict-concurrent-b",
            "ReleaseCandidate",
        )
        .to_string();
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/quarantine-jury/verdicts",
                None,
                &concurrent_a,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/quarantine-jury/verdicts",
                None,
                &concurrent_b,
                &state
            )
        );
        let left = left.expect("left quarantine verdict concurrency response");
        let right = right.expect("right quarantine verdict concurrency response");
        let aggregate = super::route_http_request(
            "GET",
            "/api/v0/quarantine-jury/requests/open-verdict-concurrent/aggregate",
            None,
            "",
            &state,
        )
        .await
        .expect("quarantine verdict concurrency readback");
        let aggregate_json = json_value(&aggregate);
        record!(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            "concurrency-and-idempotency",
            created.status == "200 OK"
                && left.status == "200 OK"
                && right.status == "200 OK"
                && aggregate_json["totalVerdicts"] == 2
                && aggregate_json["quorumReached"] == true
        );
    }

    assert_eq!(ledger.len(), 35, "quarantine-jury residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create quarantine-jury evidence directory");
    fs::write(
        evidence_dir.join("quarantine_jury_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize quarantine-jury ledger"),
    )
    .expect("write quarantine-jury ledger");
    assert!(
        mismatches.is_empty(),
        "{} quarantine-jury controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 2 content-streaming routes'
/// cases, independently re-derived from `share_tokens_use_headers_
/// and_content_bound_stream_tickets`'s real checks that share
/// tokens/tickets are rejected in the query string (header-only),
/// are content-bound (a ticket minted for one content id 401s for a
/// different one), and are revoked when the owning grant is deleted.
/// `/api/collections`/`/api/share-grants` fixture setup uses the
/// original test's bare (non-`/api/v0/`) paths -- neither is
/// registered in either target, used purely to seed real state, not
/// credited. slskdN-only (confirmed against the frozen registry).
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
async fn controller_api_differential_content_bound_stream_tickets() {
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
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "differential-route-token"),
    );
    let api_authorization = Some("Bearer differential-route-token");

    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        api_authorization,
        r#"{"name":"Private Differential"}"#,
        &state,
    )
    .await
    .expect("create collection fixture");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        api_authorization,
        r#"{"content_id":"content/differential-one","title":"track.flac","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create collection item fixture");
    let grant = super::route_http_request(
        "POST",
        "/api/share-grants",
        api_authorization,
        &format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}"),
        &state,
    )
    .await
    .expect("create share grant fixture");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let issued = super::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        api_authorization,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .expect("issue share token fixture");
    let token = serde_json::from_str::<serde_json::Value>(&issued.body).unwrap_or_default()
        ["token"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let query_stream = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fdifferential-one?token={}",
            super::url_encode(&token)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("query-string token rejected");
    record!(
        "GET",
        "/api/v0/streams/{contentId}",
        "malformed-path-query-or-body",
        query_stream.status == "400 Bad Request"
    );

    let share_headers = super::RequestSecurityHeaders {
        x_share_token: Some(token.clone()),
        ..Default::default()
    };
    let ticket_route = "/api/v0/streams/content%2Fdifferential-one/share-ticket";
    let ticket_response = super::route_http_request_with_headers(
        "POST",
        ticket_route,
        None,
        "",
        &state,
        share_headers,
    )
    .await
    .unwrap_or_else(|error| panic!("{ticket_route}: {error}"));
    let ticket_json =
        serde_json::from_str::<serde_json::Value>(&ticket_response.body).unwrap_or_default();
    let ticket = ticket_json["ticket"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "nominal-status-headers-body",
        ticket_response.status == "200 OK" && !ticket_response.body.contains(&token)
    );

    let stream = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fdifferential-one?ticket={}",
            super::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("stream with valid ticket");
    record!(
        "GET",
        "/api/v0/streams/{contentId}",
        "nominal-status-headers-body",
        stream.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&stream.body).unwrap_or_default()
                ["status"]
                == "available"
    );
    let stream_json = serde_json::from_str::<serde_json::Value>(&stream.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/streams/{contentId}",
        "populated-dynamic-state",
        stream.status == "200 OK"
            && stream_json["id"] == "content/differential-one"
            && stream_json["status"] == "available"
            && stream_json["ticket"] == "accepted"
    );

    let wrong_content = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/differential-different?ticket={}",
            super::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("ticket rejected for wrong content id");
    record!(
        "GET",
        "/api/v0/streams/{contentId}",
        "missing-empty-or-conflict-state",
        wrong_content.status == "401 Unauthorized"
    );

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        api_authorization,
        "",
        &state,
    )
    .await
    .expect("delete share grant");
    let revoked_stream = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fdifferential-one?ticket={}",
            super::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("revoked ticket rejected");
    record!(
        "GET",
        "/api/v0/streams/{contentId}",
        "mutation-side-effects-and-readback",
        deleted.status == "200 OK" && revoked_stream.status == "401 Unauthorized"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("content_bound_stream_tickets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api content-bound-stream-tickets mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
fn controller_api_differential_primary_stream_ticket_lifecycle() {
    run_controller_future_on_large_stack("primary-stream-ticket-lifecycle", || {
        controller_api_differential_primary_stream_ticket_lifecycle_impl()
    });
}

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
async fn controller_api_differential_primary_stream_ticket_lifecycle_impl() {
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

    let content_id = "Virtual/Test.flac";
    let encoded_content_id = super::url_encode(content_id);
    let normal_route = format!("/api/v0/streams/{encoded_content_id}/ticket");
    let stream_route = format!("/api/v0/streams/{encoded_content_id}");
    let normal_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "differential-route-token");
    let normal_authorization = Some("Bearer differential-route-token");
    let (state, _receiver) = test_state_with_env(normal_env.clone());

    let normal_ticket_response =
        super::route_http_request("POST", &normal_route, normal_authorization, "", &state)
            .await
            .expect("normal stream ticket response");
    let normal_ticket_json =
        serde_json::from_str::<serde_json::Value>(&normal_ticket_response.body)
            .unwrap_or_else(|_| serde_json::json!({}));
    let normal_ticket = normal_ticket_json["ticket"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "nominal-status-headers-body",
        normal_ticket_response.status == "200 OK"
            && normal_ticket_response.content_type == "application/json"
            && normal_ticket_json["expiresInSeconds"] == 120
            && !normal_ticket.is_empty()
    );

    let normal_stream = super::route_http_request(
        "GET",
        &format!(
            "{stream_route}?ticket={}",
            super::url_encode(&normal_ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("normal stream ticket readback");
    let normal_stream_json = serde_json::from_str::<serde_json::Value>(&normal_stream.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let normal_stored = state
        .stream_tickets
        .write()
        .await
        .get(&normal_ticket)
        .is_some_and(|ticket| {
            ticket.family == "share"
                && ticket.content_id == content_id
                && ticket.filename == content_id
        });
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "mutation-side-effects-and-readback",
        normal_stored
            && normal_stream.status == "200 OK"
            && normal_stream_json["status"] == "available"
            && normal_stream_json["ticket"] == "accepted"
    );

    let malformed_normal = super::route_http_request(
        "POST",
        "/api/v0/streams/%20/ticket",
        normal_authorization,
        "",
        &state,
    )
    .await
    .expect("normal stream ticket malformed content id");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "malformed-path-query-or-body",
        malformed_normal.status == "400 Bad Request"
            && malformed_normal.body.contains("ContentId is required")
    );

    let missing_normal = super::route_http_request(
        "POST",
        "/api/v0/streams/content%3Amusic%3Arecording%3Amissing/ticket",
        normal_authorization,
        "",
        &state,
    )
    .await
    .expect("normal stream ticket missing content");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "missing-empty-or-conflict-state",
        missing_normal.status == "404 Not Found"
    );

    let (capacity_state, _receiver) = test_state_with_env(normal_env.clone());
    {
        let mut tickets = capacity_state.stream_tickets.write().await;
        while tickets.records.len() < super::MAX_PREVIEW_STREAM_TICKETS {
            let index = tickets.records.len();
            assert!(tickets
                .issue(
                    "share",
                    "local-share",
                    format!("stream-capacity-{index}"),
                    "Track.flac".to_owned(),
                    None,
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity_normal = super::route_http_request(
        "POST",
        &normal_route,
        normal_authorization,
        "",
        &capacity_state,
    )
    .await
    .expect("normal stream ticket capacity response");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "runtime-failure-and-timeout",
        capacity_normal.status == "503 Service Unavailable"
            && capacity_normal
                .body
                .contains("stream ticket capacity is full")
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("normal stream runtime-failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        normal_env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_ticket = super::route_http_request(
        "POST",
        &normal_route,
        normal_authorization,
        "",
        &failure_state,
    )
    .await
    .expect("create normal runtime-failure ticket");
    let failure_ticket_json = serde_json::from_str::<serde_json::Value>(&failure_ticket.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    failure_db.close_for_test().await;
    let failure_stream = super::route_http_request(
        "GET",
        &format!(
            "{stream_route}?ticket={}",
            super::url_encode(failure_ticket_json["ticket"].as_str().unwrap_or_default())
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("normal stream read with closed unrelated database");
    record!(
        "GET",
        "/api/v0/streams/{contentId}",
        "runtime-failure-and-timeout",
        failure_stream.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_stream.body)
                .map(|value| value["status"] == "available")
                .unwrap_or(false)
    );

    let (restarted_state, _restarted_receiver) = test_state_with_env(normal_env.clone());
    let reset_get = super::route_http_request(
        "GET",
        &format!(
            "{stream_route}?ticket={}",
            super::url_encode(&normal_ticket)
        ),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("normal stream ticket after restart");
    let reset_create = super::route_http_request(
        "POST",
        &normal_route,
        normal_authorization,
        "",
        &restarted_state,
    )
    .await
    .expect("normal stream ticket create after restart");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "restart-persistence-or-reset",
        reset_get.status == "401 Unauthorized" && reset_create.status == "200 OK"
    );

    let (concurrent_state, _concurrent_receiver) = test_state_with_env(normal_env.clone());
    let concurrent_normal = tokio::join!(
        super::route_http_request(
            "POST",
            &normal_route,
            normal_authorization,
            "",
            &concurrent_state,
        ),
        super::route_http_request(
            "POST",
            &normal_route,
            normal_authorization,
            "",
            &concurrent_state,
        ),
    );
    let concurrent_normal_pass = match concurrent_normal {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
                && concurrent_state.stream_tickets.read().await.records.len() == 2
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "concurrency-and-idempotency",
        concurrent_normal_pass
    );

    let api_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "differential-route-token");
    let (share_state, _share_receiver) = test_state_with_env(api_env.clone());
    let api_authorization = Some("Bearer differential-route-token");
    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        api_authorization,
        r#"{"name":"Primary stream residual"}"#,
        &share_state,
    )
    .await
    .expect("create stream share collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let share_content_id = "content/stream-residual";
    super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        api_authorization,
        &format!(r#"{{"content_id":"{share_content_id}","title":"Residual.flac","kind":"Audio"}}"#),
        &share_state,
    )
    .await
    .expect("create stream share item");
    let grant = super::route_http_request(
        "POST",
        "/api/share-grants",
        api_authorization,
        &format!(r#"{{"collection_id":"{collection_id}","username":"friend"}}"#),
        &share_state,
    )
    .await
    .expect("create stream share grant");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let issued = super::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        api_authorization,
        r#"{"expiresInSeconds":600}"#,
        &share_state,
    )
    .await
    .expect("issue stream share token");
    let share_token = serde_json::from_str::<serde_json::Value>(&issued.body).unwrap_or_default()
        ["token"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let share_headers = super::RequestSecurityHeaders {
        x_share_token: Some(share_token.clone()),
        ..Default::default()
    };
    let share_encoded_content_id = super::url_encode(share_content_id);
    let share_route = format!("/api/v0/streams/{share_encoded_content_id}/share-ticket");

    let share_ticket_response = super::route_http_request_with_headers(
        "POST",
        &share_route,
        None,
        "",
        &share_state,
        share_headers.clone(),
    )
    .await
    .expect("share stream ticket response");
    let share_ticket_json = serde_json::from_str::<serde_json::Value>(&share_ticket_response.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let share_ticket = share_ticket_json["ticket"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "nominal-status-headers-body",
        share_ticket_response.status == "200 OK"
            && share_ticket_response.content_type == "application/json"
            && share_ticket_json["expiresInSeconds"] == 120
            && !share_ticket.is_empty()
            && !share_ticket_response.body.contains(&share_token)
    );
    let share_stored = share_state
        .stream_tickets
        .write()
        .await
        .get(&share_ticket)
        .is_some_and(|ticket| ticket.family == "share" && ticket.content_id == share_content_id);
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "mutation-side-effects-and-readback",
        share_stored
    );

    let malformed_share = super::route_http_request_with_headers(
        "POST",
        &format!("{share_route}/extra"),
        api_authorization,
        "",
        &share_state,
        share_headers.clone(),
    )
    .await
    .expect("malformed share stream ticket path");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "malformed-path-query-or-body",
        malformed_share.status == "404 Not Found"
    );

    let missing_share = super::route_http_request("POST", &share_route, None, "", &share_state)
        .await
        .expect("missing share stream token");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "missing-empty-or-conflict-state",
        missing_share.status == "401 Unauthorized"
    );

    let concurrent_share = tokio::join!(
        super::route_http_request_with_headers(
            "POST",
            &share_route,
            None,
            "",
            &share_state,
            share_headers.clone(),
        ),
        super::route_http_request_with_headers(
            "POST",
            &share_route,
            None,
            "",
            &share_state,
            share_headers.clone(),
        ),
    );
    let concurrent_share_pass = match concurrent_share {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "concurrency-and-idempotency",
        concurrent_share_pass
    );

    {
        let mut tickets = share_state.stream_tickets.write().await;
        while tickets.records.len() < super::MAX_PREVIEW_STREAM_TICKETS {
            let index = tickets.records.len();
            assert!(tickets
                .issue(
                    "share",
                    "share:capacity",
                    format!("share-capacity-{index}"),
                    "Track.flac".to_owned(),
                    Some("friend".to_owned()),
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity_share = super::route_http_request_with_headers(
        "POST",
        &share_route,
        None,
        "",
        &share_state,
        share_headers.clone(),
    )
    .await
    .expect("share stream ticket capacity response");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "runtime-failure-and-timeout",
        capacity_share.status == "503 Service Unavailable"
            && capacity_share
                .body
                .contains("share stream ticket capacity is full")
    );

    let (restarted_share_state, _restarted_share_receiver) = test_state_with_env(api_env);
    let reset_share = super::route_http_request_with_headers(
        "POST",
        &share_route,
        None,
        "",
        &restarted_share_state,
        share_headers,
    )
    .await
    .expect("share stream ticket after restart");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "restart-persistence-or-reset",
        reset_share.status == "401 Unauthorized"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("primary_stream_ticket_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} primary stream ticket mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 6 port-forwarding routes' cases,
/// independently re-derived from `port_forwarding_reads_are_bounded_
/// and_start_requires_an_authorized_pinned_gateway`'s real bounded
/// port-listing and gateway-pinning security checks (start is
/// forbidden without a real, authorized, certificate-pinned gateway
/// pod, and succeeds once one genuinely exists). The registry lists
/// these routes as `/api/v0/portforwarding/...` (no hyphen);
/// `normalize_api_path` maps that to the hyphenated `/api/port-
/// forwarding/...` internal form the source test calls directly --
/// both resolve to the same handler (see the `normalize_api_path`
/// unit test asserting this exact mapping), so either request path
/// exercises the same registered route. slskdN-only (confirmed
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
async fn controller_api_differential_port_forwarding() {
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

    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));

    let status =
        super::route_http_request("GET", "/api/v0/portforwarding/status", None, "", &state)
            .await
            .expect("port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "nominal-status-headers-body",
        status.status == "200 OK" && status.body == "[]"
    );
    let malformed_status_collection = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/extra/more",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding status collection");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "malformed-path-query-or-body",
        malformed_status_collection.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "missing-empty-or-conflict-state",
        status.status == "200 OK" && status.body == "[]"
    );

    let available = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/available-ports?startPort=2000&endPort=2010&limit=3",
        None,
        "",
        &state,
    )
    .await
    .expect("available port page");
    let available_json =
        serde_json::from_str::<serde_json::Value>(&available.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "nominal-status-headers-body",
        available.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "populated-dynamic-state",
        available_json["availablePortCount"] == 11
            && available_json["usedPortCount"] == 0
            && available_json["availablePorts"] == serde_json::json!([2000, 2001, 2002])
    );
    let available_default = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/available-ports",
        None,
        "",
        &state,
    )
    .await
    .expect("default available ports");
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "missing-empty-or-conflict-state",
        available_default.status == "200 OK"
    );

    let mut invalid_query_pass = true;
    for path in [
        "/api/v0/portforwarding/available-ports?startPort=0",
        "/api/v0/portforwarding/available-ports?startPort=3000&endPort=2000",
        "/api/v0/portforwarding/available-ports?limit=0",
        "/api/v0/portforwarding/available-ports?limit=1001",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        invalid_query_pass &= response.status == "400 Bad Request";
    }
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "malformed-path-query-or-body",
        invalid_query_pass
    );

    let stats = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        None,
        "",
        &state,
    )
    .await
    .expect("port forwarding stream stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "populated-dynamic-state",
        stats_json["totalForwardingRules"] == 0 && stats_json["rules"] == serde_json::json!([])
    );
    let malformed_stats = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/stream-stats/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding stream stats");
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "malformed-path-query-or-body",
        malformed_stats.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "missing-empty-or-conflict-state",
        stats.status == "200 OK" && stats_json["totalForwardingRules"] == 0
    );

    let missing = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/2000",
        None,
        "",
        &state,
    )
    .await
    .expect("missing port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );
    let malformed_status = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/not-a-port",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "malformed-path-query-or-body",
        malformed_status.status == "404 Not Found"
    );

    let malformed_start = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start/extra",
        None,
        "{}",
        &state,
    )
    .await
    .expect("malformed port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "malformed-path-query-or-body",
        malformed_start.status == "404 Not Found"
    );

    let start_unpinned = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        r#"{"localPort":2000,"podId":"pod-differential-unpinned","destinationHost":"service","destinationPort":80}"#,
        &state,
    )
    .await
    .expect("unauthorized port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "missing-empty-or-conflict-state",
        start_unpinned.status == "403 Forbidden"
    );

    let pin = "07".repeat(32);
    let create = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-differential-forward","name":"Forward Differential","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &state,
    )
    .await
    .expect("create gateway pod fixture");
    assert_eq!(create.status, "201 Created", "{}", create.body);

    let mut gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    gateway.peer_id = "tester".to_owned();
    state.mesh.write().await.capability_records.push(gateway);

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);

    let start = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-differential-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start port forwarding");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "nominal-status-headers-body",
        start.status == "200 OK"
    );
    let repeated_start = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-differential-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("repeated port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "concurrency-and-idempotency",
        start.status == "200 OK" && repeated_start.status == "409 Conflict"
    );

    let status_route = format!("/api/v0/portforwarding/status/{local_port}");
    let running_status = super::route_http_request("GET", &status_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{status_route}: {error}"));
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "nominal-status-headers-body",
        running_status.status == "200 OK"
            && running_status.body.contains("pod-differential-forward")
    );
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "populated-dynamic-state",
        running_status.status == "200 OK"
            && running_status.body.contains("pod-differential-forward")
    );
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "mutation-side-effects-and-readback",
        start.status == "200 OK"
            && running_status.status == "200 OK"
            && running_status.body.contains("pod-differential-forward")
    );

    let populated_status =
        super::route_http_request("GET", "/api/v0/portforwarding/status", None, "", &state)
            .await
            .expect("populated port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "populated-dynamic-state",
        populated_status.status == "200 OK"
            && populated_status.body.contains("pod-differential-forward")
    );

    let stop_route = format!("/api/v0/portforwarding/stop/{local_port}");
    let stop = super::route_http_request("POST", &stop_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{stop_route}: {error}"));
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "mutation-side-effects-and-readback",
        stop.status == "200 OK"
    );
    let stop_missing = super::route_http_request("POST", &stop_route, None, "", &state)
        .await
        .expect("missing port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "missing-empty-or-conflict-state",
        stop_missing.status == "200 OK"
    );
    let malformed_stop = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/stop/not-a-port",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "malformed-path-query-or-body",
        malformed_stop.status == "404 Not Found"
    );

    let occupied = tokio::net::TcpListener::bind(("127.0.0.1", local_port))
        .await
        .expect("occupy forwarding local port");
    let runtime_start = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-differential-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("runtime port forwarding start");
    drop(occupied);
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "runtime-failure-and-timeout",
        runtime_start.status == "500 Internal Server Error"
            && runtime_start
                .body
                .contains("Failed to start port forwarding")
    );

    let (restart_state, _restart_receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));
    let restart_pod = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-differential-forward-restart","name":"Forward Restart","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &restart_state,
    )
    .await
    .expect("create restarted gateway pod");
    assert_eq!(restart_pod.status, "201 Created", "{}", restart_pod.body);
    let mut restart_gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    restart_gateway.peer_id = "tester".to_owned();
    restart_state
        .mesh
        .write()
        .await
        .capability_records
        .push(restart_gateway);
    let restart_probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("restarted port probe");
    let restart_port = restart_probe.local_addr().unwrap().port();
    drop(restart_probe);
    let restart_start = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{restart_port},"podId":"pod-differential-forward-restart","destinationHost":"service","destinationPort":80}}"#
        ),
        &restart_state,
    )
    .await
    .expect("restarted port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "restart-persistence-or-reset",
        restart_start.status == "200 OK"
    );
    let _ = restart_state.port_forwarding.stop(restart_port).await;

    let (stop_restart_state, _stop_restart_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let stop_restart = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/stop/29999",
        None,
        "",
        &stop_restart_state,
    )
    .await
    .expect("restarted port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "restart-persistence-or-reset",
        stop_restart.status == "200 OK"
    );
    let stop_concurrent = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/portforwarding/stop/29998",
            None,
            "",
            &stop_restart_state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/portforwarding/stop/29998",
            None,
            "",
            &stop_restart_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "concurrency-and-idempotency",
        stop_concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    let runtime_read_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("port forwarding runtime read database");
    let (runtime_read_state, _runtime_read_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(runtime_read_db.clone()),
    );
    runtime_read_db.close_for_test().await;
    let runtime_available = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/available-ports",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime available ports");
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "runtime-failure-and-timeout",
        runtime_available.status == "200 OK"
    );
    let runtime_status = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/status",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "runtime-failure-and-timeout",
        runtime_status.status == "200 OK" && runtime_status.body == "[]"
    );
    let runtime_dynamic_status = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/29997",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime dynamic port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "runtime-failure-and-timeout",
        runtime_dynamic_status.status == "404 Not Found"
    );
    let runtime_stats = super::route_http_request(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime port forwarding stream stats");
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "runtime-failure-and-timeout",
        runtime_stats.status == "200 OK"
    );
    let runtime_stop = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/stop/29996",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "runtime-failure-and-timeout",
        runtime_stop.status == "200 OK"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("port_forwarding.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api port-forwarding mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

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
async fn controller_api_differential_security_reputation() {
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
        let empty = super::route_http_request("GET", route, None, "", &state)
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
    let clean = super::route_http_request("GET", clean_route, None, "", &state)
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
    let bad = super::route_http_request("GET", bad_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{bad_route}: {error}"));
    let bad_json = serde_json::from_str::<serde_json::Value>(&bad.body).unwrap_or_default();

    let suspicious = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
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

    let trusted = super::route_http_request(
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

    let invalid = super::route_http_request("PUT", bad_route, None, r#"{"score":150}"#, &state)
        .await
        .unwrap_or_else(|error| panic!("{bad_route}: {error}"));
    record!(
        "PUT",
        "/api/v0/security/reputation/{username}",
        "malformed-path-query-or-body",
        invalid.status == "400 Bad Request"
    );

    let overridden = super::route_http_request(
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

    let after_override = super::route_http_request("GET", bad_route, None, "", &state)
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

/// Bulk differential proof crediting 5 realm-subject-index routes'
/// cases, independently re-derived from `realm_subject_indexes_
/// persist_authority_and_compute_conflicts`'s real conflict-
/// detection (external-id, recording-subject, workref-identity,
/// alias-subject) and authority-decision-disables-conflicts checks.
/// Uses the same `compute_payload_hash`-based fixture-building
/// closure pattern as the source test (computes a real hash over its
/// own content, so index content can be freely varied per fixture
/// without breaking signature validation -- unlike the earlier
/// session bug with a copy-pasted fixed hash literal). slskdN-only
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
async fn controller_api_differential_realm_subject_indexes() {
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
    let index = |id: &str, subject: &str, title: &str, discogs: &str| {
        let mut index = serde_json::json!({
            "id": id,
            "realmId": super::realm_subject_index::DEFAULT_REALM_ID,
            "subjectNamespace": "music",
            "revision": 1,
            "publishedAt": "2026-08-01T00:00:00Z",
            "entries": [{
                "subjectId": subject,
                "workRef": {
                    "domain": "music",
                    "title": title,
                    "creator": "Artist",
                },
                "externalIds": {
                    "musicbrainz:recording": "recording-differential",
                    "discogs": discogs,
                },
                "aliases": ["shared-alias-differential"],
            }],
            "signature": {
                "signer": super::realm_subject_index::DEFAULT_GOVERNANCE_ROOT,
                "algorithm": "realm-governance-sha256",
                "payloadHash": "",
                "value": "signature",
            },
        });
        index["signature"]["payloadHash"] =
            serde_json::json!(super::realm_subject_index::compute_payload_hash(&index));
        index
    };

    let merged = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"recording-differential","peerIds":["peer-a"],"updatedAt":1}],
            "realmIndexes": [
                index("index-differential-a", "subject-a", "Title A", "discogs-a"),
                index("index-differential-b", "subject-b", "Title B", "discogs-b"),
                index("index-differential-c", "subject-a", "Title C", "discogs-c"),
            ],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("register realm index fixtures");
    assert_eq!(merged.status, "200 OK", "{}", merged.body);

    let indexes = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &state,
    )
    .await
    .expect("list realm indexes");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "nominal-status-headers-body",
        indexes.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "populated-dynamic-state",
        serde_json::from_str::<serde_json::Value>(&indexes.body)
            .unwrap_or_default()
            .as_array()
            .map(Vec::len)
            == Some(3)
    );

    let conflicts_route = "/api/v0/realm-subject-indexes/default-realm/conflicts";
    let conflicts = super::route_http_request("GET", conflicts_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{conflicts_route}: {error}"));
    let conflicts_json =
        serde_json::from_str::<serde_json::Value>(&conflicts.body).unwrap_or_default();
    let conflict_types = conflicts_json["conflicts"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|entry| entry["type"].as_str().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "nominal-status-headers-body",
        conflicts.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "populated-dynamic-state",
        conflicts_json["indexCount"] == 3
            && conflicts_json["entryCount"] == 3
            && conflicts_json["hasConflicts"] == true
            && conflict_types.contains(&"external-id".to_owned())
            && conflict_types.contains(&"recording-subject".to_owned())
            && conflict_types.contains(&"workref-identity".to_owned())
            && conflict_types.contains(&"alias-subject".to_owned())
    );

    let resolutions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-differential/resolutions",
        None,
        "",
        &state,
    )
    .await
    .expect("recording resolutions");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "nominal-status-headers-body",
        resolutions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "populated-dynamic-state",
        serde_json::from_str::<serde_json::Value>(&resolutions.body)
            .unwrap_or_default()
            .as_array()
            .map(Vec::len)
            == Some(3)
    );

    for index_id in ["index-differential-b", "index-differential-c"] {
        let route =
            format!("/api/v0/realm-subject-indexes/default-realm/{index_id}/authority-decision");
        let disabled = super::route_http_request(
            "POST",
            &route,
            None,
            r#"{"enabled":false,"decidedBy":"differential-operator","note":"conflicting authority"}"#,
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route}: {error}"));
        assert_eq!(disabled.status, "200 OK", "{}", disabled.body);
    }
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "mutation-side-effects-and-readback",
        true
    );

    let decisions_route = "/api/v0/realm-subject-indexes/default-realm/authority-decisions";
    let decisions = super::route_http_request("GET", decisions_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{decisions_route}: {error}"));
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "nominal-status-headers-body",
        decisions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "populated-dynamic-state",
        serde_json::from_str::<serde_json::Value>(&decisions.body)
            .unwrap_or_default()
            .as_array()
            .map(Vec::len)
            == Some(2)
    );

    let after_disable = super::route_http_request("GET", conflicts_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{conflicts_route}: {error}"));
    let after_disable_json =
        serde_json::from_str::<serde_json::Value>(&after_disable.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "mutation-side-effects-and-readback",
        after_disable_json["disabledAuthorityCount"] == 2
            && after_disable_json["hasConflicts"] == false
    );

    let missing_route =
        "/api/v0/realm-subject-indexes/default-realm/missing-differential/authority-decision";
    let missing_decision = super::route_http_request(
        "POST",
        missing_route,
        None,
        r#"{"enabled":true,"decidedBy":"differential-operator"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{missing_route}: {error}"));
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "missing-empty-or-conflict-state",
        missing_decision.status == "400 Bad Request"
            && missing_decision
                .body
                .contains("Index authority was not found")
    );

    let (empty_state, _empty_receiver) = test_state();
    let malformed_indexes = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed realm indexes path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "malformed-path-query-or-body",
        malformed_indexes.status == "404 Not Found"
    );
    let malformed_authority_decisions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed authority decisions path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "malformed-path-query-or-body",
        malformed_authority_decisions.status == "404 Not Found"
    );
    let malformed_conflicts = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed realm conflicts path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "malformed-path-query-or-body",
        malformed_conflicts.status == "404 Not Found"
    );
    let malformed_resolutions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-differential/resolutions/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed recording resolutions path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "malformed-path-query-or-body",
        malformed_resolutions.status == "404 Not Found"
    );

    let empty_indexes = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty realm indexes");
    let empty_indexes_json =
        serde_json::from_str::<serde_json::Value>(&empty_indexes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "missing-empty-or-conflict-state",
        empty_indexes.status == "200 OK" && empty_indexes_json == serde_json::json!([])
    );
    let empty_authority_decisions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty authority decisions");
    let empty_authority_decisions_json =
        serde_json::from_str::<serde_json::Value>(&empty_authority_decisions.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "missing-empty-or-conflict-state",
        empty_authority_decisions.status == "200 OK"
            && empty_authority_decisions_json == serde_json::json!([])
    );
    let empty_conflicts = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty realm conflicts");
    let empty_conflicts_json =
        serde_json::from_str::<serde_json::Value>(&empty_conflicts.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "missing-empty-or-conflict-state",
        empty_conflicts.status == "200 OK"
            && empty_conflicts_json["indexCount"] == 0
            && empty_conflicts_json["disabledAuthorityCount"] == 0
            && empty_conflicts_json["entryCount"] == 0
            && empty_conflicts_json["hasConflicts"] == false
            && empty_conflicts_json["conflicts"] == serde_json::json!([])
    );
    let empty_resolutions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-empty/resolutions",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty recording resolutions");
    let empty_resolutions_json =
        serde_json::from_str::<serde_json::Value>(&empty_resolutions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "missing-empty-or-conflict-state",
        empty_resolutions.status == "200 OK" && empty_resolutions_json == serde_json::json!([])
    );

    let runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("realm subject-index runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    let runtime_indexes = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime realm indexes");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "runtime-failure-and-timeout",
        runtime_indexes.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_indexes.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_authority_decisions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime authority decisions");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "runtime-failure-and-timeout",
        runtime_authority_decisions.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_authority_decisions.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_conflicts = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime realm conflicts");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "runtime-failure-and-timeout",
        runtime_conflicts.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_conflicts.body)
                .is_ok_and(|value| value.is_object())
    );
    let runtime_resolutions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-runtime/resolutions",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime recording resolutions");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "runtime-failure-and-timeout",
        runtime_resolutions.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_resolutions.body)
                .is_ok_and(|value| value.is_array())
    );

    let runtime_index = index(
        "index-runtime-differential",
        "subject-runtime-differential",
        "Runtime Title",
        "discogs-runtime-differential",
    );
    runtime_state
        .realm_subject_indexes
        .write()
        .await
        .merge_indexes(vec![runtime_index])
        .expect("seed runtime authority index");
    let runtime_decision = super::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-runtime-differential/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"runtime-operator"}"#,
        &runtime_state,
    )
    .await
    .expect("runtime authority decision");
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "runtime-failure-and-timeout",
        runtime_decision.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_decision.body)
                .is_ok_and(|value| value["isAccepted"] == true)
    );

    let persisted_state_dir = state.config.state_dir.clone();
    *state.realm_subject_indexes.write().await =
        super::realm_subject_index::Store::load_with_identity(
            &persisted_state_dir,
            super::realm_subject_index::DEFAULT_REALM_ID,
            [super::realm_subject_index::DEFAULT_GOVERNANCE_ROOT],
        )
        .expect("load persistent realm subject-index store");
    let persisted_index = index(
        "index-restart-differential",
        "subject-restart-differential",
        "Restart Title",
        "discogs-restart-differential",
    );
    state
        .realm_subject_indexes
        .write()
        .await
        .merge_indexes(vec![persisted_index])
        .expect("persist restart authority index");
    let persisted_decision = super::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-restart-differential/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"restart-operator"}"#,
        &state,
    )
    .await
    .expect("persist restart authority decision");
    let (restarted_state, _restarted_receiver) = test_state();
    *restarted_state.realm_subject_indexes.write().await =
        super::realm_subject_index::Store::load_with_identity(
            &persisted_state_dir,
            super::realm_subject_index::DEFAULT_REALM_ID,
            [super::realm_subject_index::DEFAULT_GOVERNANCE_ROOT],
        )
        .expect("reload persistent realm subject-index store");
    let restarted_decisions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("read restarted authority decisions");
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "restart-persistence-or-reset",
        persisted_decision.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&restarted_decisions.body).is_ok_and(
                |value| {
                    value.as_array().is_some_and(|decisions| {
                        decisions.iter().any(|decision| {
                            decision["indexId"] == "index-restart-differential"
                                && decision["enabled"] == false
                        })
                    })
                }
            )
    );

    let concurrent_bodies: Vec<String> = (0..4)
        .map(|index| {
            format!(
                r#"{{"enabled":{},"decidedBy":"concurrent-operator-{index}"}}"#,
                index % 2 == 0
            )
        })
        .collect();
    let concurrent_posts = futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
        let path =
            "/api/v0/realm-subject-indexes/default-realm/index-restart-differential/authority-decision"
                .to_owned();
        let body = body.clone();
        let state = Arc::clone(&state);
        async move { super::route_http_request("POST", &path, None, &body, &state).await }
    }))
    .await;
    let concurrent_decisions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &state,
    )
    .await
    .expect("read concurrent authority decisions");
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "concurrency-and-idempotency",
        concurrent_posts.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && serde_json::from_str::<serde_json::Value>(&concurrent_decisions.body).is_ok_and(
            |value| {
                value.as_array().is_some_and(|decisions| {
                    decisions
                        .iter()
                        .any(|decision| decision["indexId"] == "index-restart-differential")
                })
            }
        )
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("realm_subject_indexes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api realm-subject-indexes mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 3 musicbrainz-overlay routes'
/// cases, independently re-derived from `musicbrainz_overlay_export_
/// review_and_approval_match_real_oracle_gating`'s real exportable-
/// type gate, unsafe-approvedBy rejection, and idempotent-approval
/// (a repeat approval never overwrites the original decision)
/// checks. slskdN-only (confirmed against the frozen registry).
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
async fn controller_api_differential_musicbrainz_overlay_export() {
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
    state.library.write().await.create(
        "Differential Overlay Artist".to_owned(),
        "Differential Overlay Release".to_owned(),
        "Audio".to_owned(),
    );
    let release_graph = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/overlays/artist/Differential%20Overlay%20Artist/release-graph",
        None,
        "",
        &state,
    )
    .await
    .expect("read overlay release graph");
    let release_graph_json =
        serde_json::from_str::<serde_json::Value>(&release_graph.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
        "nominal-status-headers-body",
        release_graph.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
        "populated-dynamic-state",
        release_graph_json["artistId"] == "Differential Overlay Artist"
            && release_graph_json["releases"]
                .as_array()
                .is_some_and(|releases| {
                    releases
                        .iter()
                        .any(|release| release["title"] == "Differential Overlay Release")
                })
            && release_graph_json["edges"].is_array()
    );

    let non_exportable = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"Other","targetType":"Recording","targetId":"rec-differential-1","field":"title","value":"New Title","evidence":[{"type":"WorkRef","reference":"opaque-ref-differential"}]}"#,
        &state,
    )
    .await
    .expect("create non-exportable edit");
    let non_exportable_id = serde_json::from_str::<serde_json::Value>(&non_exportable.body)
        .unwrap_or_default()["edit"]["editId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        "nominal-status-headers-body",
        non_exportable.status == "200 OK" && !non_exportable_id.is_empty()
    );

    let route_route = format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/routes");
    let empty_route = super::route_http_request("POST", &route_route, None, "{}", &state)
        .await
        .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let empty_route_json =
        serde_json::from_str::<serde_json::Value>(&empty_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "malformed-path-query-or-body",
        empty_route.status == "400 Bad Request"
            && empty_route_json["editId"] == non_exportable_id
            && empty_route_json["success"] == false
            && empty_route_json["errorMessage"] == "At least one target peer is required."
            && empty_route_json["targetPeerIds"] == serde_json::json!([])
    );

    let unavailable_route = super::route_http_request(
        "POST",
        &route_route,
        None,
        r#"{"targetPeerIds":["actor:differential-route-peer"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let unavailable_route_json =
        serde_json::from_str::<serde_json::Value>(&unavailable_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "runtime-failure-and-timeout",
        unavailable_route.status == "400 Bad Request"
            && unavailable_route_json["editId"] == non_exportable_id
            && unavailable_route_json["success"] == false
            && unavailable_route_json["errorMessage"] == "Routing backend is not available."
            && unavailable_route_json["targetPeerIds"]
                == serde_json::json!(["actor:differential-route-peer"])
    );

    let route_attempts = super::route_http_request("GET", &route_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let route_attempts_json =
        serde_json::from_str::<serde_json::Value>(&route_attempts.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "nominal-status-headers-body",
        route_attempts.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "mutation-side-effects-and-readback",
        route_attempts_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["id"] == unavailable_route_json["id"]
                    && attempt["editId"] == non_exportable_id
                    && attempt["errorMessage"] == "Routing backend is not available."
            })
        })
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "populated-dynamic-state",
        route_attempts_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["id"] == unavailable_route_json["id"]
                    && attempt["editId"] == non_exportable_id
                    && attempt["success"] == false
            })
        })
    );

    let review_route =
        format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/export-review");
    let review = super::route_http_request("GET", &review_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{review_route}: {error}"));
    let review_json = serde_json::from_str::<serde_json::Value>(&review.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
        "nominal-status-headers-body",
        review.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
        "populated-dynamic-state",
        review_json["upstreamTarget"] == "Recording:rec-differential-1"
            && review_json["proposedChange"] == "title => New Title"
            && review_json["canApproveExport"] == false
            && review_json["reviewReason"] == "Overlay edit type is not exportable."
            && review_json["decision"].is_null()
    );

    let rejected_route =
        format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/approve-export");
    let rejected_approval = super::route_http_request("POST", &rejected_route, None, "{}", &state)
        .await
        .unwrap_or_else(|error| panic!("{rejected_route}: {error}"));
    let rejected_json =
        serde_json::from_str::<serde_json::Value>(&rejected_approval.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "malformed-path-query-or-body",
        rejected_approval.status == "400 Bad Request"
            && rejected_json["errors"]
                .as_array()
                .is_some_and(|errors| errors
                    .iter()
                    .any(|error| error == "Overlay edit type is not exportable."))
            && rejected_json["decision"].is_null()
    );

    let unsafe_edit = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"TitleCorrection","targetType":"Recording","targetId":"rec-differential-2","field":"title","value":"Corrected","evidence":[{"type":"WorkRef","reference":"opaque-ref-differential"}]}"#,
        &state,
    )
    .await
    .expect("create exportable edit for unsafe-approver check");
    let unsafe_edit_id = serde_json::from_str::<serde_json::Value>(&unsafe_edit.body)
        .unwrap_or_default()["edit"]["editId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let unsafe_route =
        format!("/api/v0/musicbrainz/overlays/edits/{unsafe_edit_id}/approve-export");
    let unsafe_approval = super::route_http_request(
        "POST",
        &unsafe_route,
        None,
        r#"{"approvedBy":"/etc/passwd"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{unsafe_route}: {error}"));
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "missing-empty-or-conflict-state",
        unsafe_approval.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&unsafe_approval.body).unwrap_or_default()
                ["errors"]
                .as_array()
                .is_some_and(|errors| errors
                    .iter()
                    .any(|error| error == "Approved-by identifier must be opaque and safe."))
    );

    let exportable = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"TitleCorrection","targetType":"Recording","targetId":"rec-differential-3","field":"title","value":"Corrected Title","evidence":[{"type":"WorkRef","reference":"opaque-ref-differential"}]}"#,
        &state,
    )
    .await
    .expect("create exportable edit");
    let exportable_id = serde_json::from_str::<serde_json::Value>(&exportable.body)
        .unwrap_or_default()["edit"]["editId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let approval_route =
        format!("/api/v0/musicbrainz/overlays/edits/{exportable_id}/approve-export");
    let approval = super::route_http_request(
        "POST",
        &approval_route,
        None,
        r#"{"approvedBy":"differential-reviewer","note":"looks good"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{approval_route}: {error}"));
    let approval_json =
        serde_json::from_str::<serde_json::Value>(&approval.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "nominal-status-headers-body",
        approval.status == "200 OK"
            && approval_json["errors"] == serde_json::json!([])
            && approval_json["decision"]["approvedBy"] == "differential-reviewer"
            && approval_json["decision"]["note"] == "looks good"
            && approval_json["decision"]["editId"] == exportable_id
            && approval_json["decision"]["upstreamTarget"] == "Recording:rec-differential-3"
            && approval_json["decision"]["proposedChange"] == "title => Corrected Title"
            && approval_json["decision"]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("musicbrainz-overlay-export:"))
    );

    let post_approval_review_route =
        format!("/api/v0/musicbrainz/overlays/edits/{exportable_id}/export-review");
    let post_approval_review =
        super::route_http_request("GET", &post_approval_review_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{post_approval_review_route}: {error}"));
    let post_approval_review_json =
        serde_json::from_str::<serde_json::Value>(&post_approval_review.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
        "mutation-side-effects-and-readback",
        post_approval_review_json["canApproveExport"] == false
            && post_approval_review_json["reviewReason"]
                == "Upstream export has already been approved locally."
            && post_approval_review_json["decision"]["approvedBy"] == "differential-reviewer"
    );

    let repeat_approval = super::route_http_request(
        "POST",
        &approval_route,
        None,
        r#"{"approvedBy":"different-differential-reviewer"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{approval_route}: {error}"));
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "mutation-side-effects-and-readback",
        repeat_approval.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&repeat_approval.body).unwrap_or_default()
                ["decision"]["approvedBy"]
                == "differential-reviewer"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("musicbrainz_overlay_export.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api musicbrainz-overlay-export mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 7 pod-membership-workflow
/// routes' cases, independently re-derived from `pod_membership_
/// workflow_queues_accepts_lists_leaves_and_cancels`'s real
/// queued-request lifecycle: joining queues a pending request rather
/// than adding the member immediately, only an authorized acceptor
/// (a real pod moderator/owner, not any member) can accept it, and
/// leave/cancel follow the same real queued pattern. slskdN-only
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
async fn controller_api_differential_pod_membership_workflow() {
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
    {
        let mut rooms = state.rooms.write().await;
        rooms
            .join("pod:differential-workflow".to_owned())
            .expect("workflow pod");
        rooms
            .add_member(
                "pod:differential-workflow",
                "differential-owner-peer".to_owned(),
            )
            .expect("owner capacity")
            .expect("workflow pod");
        rooms
            .add_member(
                "pod:differential-workflow",
                "differential-ordinary-peer".to_owned(),
            )
            .expect("ordinary member capacity")
            .expect("workflow pod");
        rooms.records[0].operated = true;
    }
    state.pod_membership_workflow.write().await.set_role(
        "pod:differential-workflow",
        "differential-owner-peer",
        "owner".to_owned(),
    );

    let join = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","requestedRole":"moderator"}"#,
        &state,
    )
    .await
    .expect("join request");
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "nominal-status-headers-body",
        join.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "mutation-side-effects-and-readback",
        !state.rooms.read().await.records[0]
            .members
            .iter()
            .any(|member| member == "differential-applicant")
    );

    let pending_route = "/api/v0/podcore/membership/join/pending/pod%3Adifferential-workflow";
    let pending = super::route_http_request("GET", pending_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{pending_route}: {error}"));
    let pending_json = serde_json::from_str::<serde_json::Value>(&pending.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/join/pending/{podId}",
        "nominal-status-headers-body",
        pending.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/membership/join/pending/{podId}",
        "populated-dynamic-state",
        pending_json["pendingJoinRequests"][0]["peerId"] == "differential-applicant"
    );

    let unauthorized = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptedRole":"moderator","acceptorPeerId":"differential-ordinary-peer"}"#,
        &state,
    )
    .await
    .expect("unauthorized join acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        "missing-empty-or-conflict-state",
        unauthorized.status == "400 Bad Request"
            && unauthorized
                .body
                .contains("Join acceptance could not be processed")
            && state
                .pod_membership_workflow
                .read()
                .await
                .pending_joins("pod:differential-workflow")
                .len()
                == 1
    );

    let accepted = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptedRole":"moderator","acceptorPeerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("join acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        "nominal-status-headers-body",
        accepted.status == "200 OK"
            && state.rooms.read().await.records[0]
                .members
                .iter()
                .any(|member| member == "differential-applicant")
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        "mutation-side-effects-and-readback",
        accepted.status == "200 OK"
            && accepted.body.contains("differential-applicant")
            && state
                .pod_membership_workflow
                .read()
                .await
                .pending_joins("pod:differential-workflow")
                .is_empty()
            && state.rooms.read().await.records[0]
                .members
                .iter()
                .any(|member| member == "differential-applicant")
    );

    let leave = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant"}"#,
        &state,
    )
    .await
    .expect("leave request");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave",
        "nominal-status-headers-body",
        leave.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&leave.body).unwrap_or_default()
                ["pending"]
                == true
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/leave",
        "mutation-side-effects-and-readback",
        leave.status == "200 OK"
            && leave.body.contains("differential-applicant")
            && state
                .pod_membership_workflow
                .read()
                .await
                .pending_leaves("pod:differential-workflow")
                .iter()
                .any(|request| request.peer_id == "differential-applicant")
    );

    let pending_leave_route =
        "/api/v0/podcore/membership/leave/pending/pod%3Adifferential-workflow";
    let pending_leave = super::route_http_request("GET", pending_leave_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{pending_leave_route}: {error}"));
    record!(
        "GET",
        "/api/v0/podcore/membership/leave/pending/{podId}",
        "nominal-status-headers-body",
        pending_leave.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/membership/leave/pending/{podId}",
        "populated-dynamic-state",
        pending_leave.body.contains("differential-applicant")
    );

    let missing_pending_join = super::route_http_request(
        "GET",
        "/api/v0/podcore/membership/join/pending/pod%3Amissing-workflow",
        None,
        "",
        &state,
    )
    .await
    .expect("missing pending join requests");
    let missing_pending_join_json =
        serde_json::from_str::<serde_json::Value>(&missing_pending_join.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/join/pending/{podId}",
        "missing-empty-or-conflict-state",
        missing_pending_join.status == "200 OK"
            && missing_pending_join_json["pendingJoinRequests"] == serde_json::json!([])
    );

    let missing_pending_leave = super::route_http_request(
        "GET",
        "/api/v0/podcore/membership/leave/pending/pod%3Amissing-workflow",
        None,
        "",
        &state,
    )
    .await
    .expect("missing pending leave requests");
    let missing_pending_leave_json =
        serde_json::from_str::<serde_json::Value>(&missing_pending_leave.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/leave/pending/{podId}",
        "missing-empty-or-conflict-state",
        missing_pending_leave.status == "200 OK"
            && missing_pending_leave_json["pendingLeaveRequests"] == serde_json::json!([])
    );

    let accepted_leave = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptorPeerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("leave acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        "nominal-status-headers-body",
        accepted_leave.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        "mutation-side-effects-and-readback",
        !state.rooms.read().await.records[0]
            .members
            .iter()
            .any(|member| member == "differential-applicant")
    );

    let missing_leave = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant"}"#,
        &state,
    )
    .await
    .expect("missing leave request");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave",
        "missing-empty-or-conflict-state",
        missing_leave.status == "400 Bad Request"
    );

    let missing_leave_accept = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptorPeerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("missing leave acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        "missing-empty-or-conflict-state",
        missing_leave_accept.status == "400 Bad Request"
    );

    let second_join = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-peer-two"}"#,
        &state,
    )
    .await
    .expect("second join request");
    assert_eq!(second_join.status, "200 OK");
    let duplicate_join = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-peer-two"}"#,
        &state,
    )
    .await
    .expect("duplicate join request");
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "missing-empty-or-conflict-state",
        duplicate_join.status == "400 Bad Request"
    );

    let cancel_route =
        "/api/v0/podcore/membership/join/pod%3Adifferential-workflow/differential-peer-two";
    let cancelled = super::route_http_request("DELETE", cancel_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/join/{podId}/{peerId}",
        "nominal-status-headers-body",
        cancelled.status == "200 OK" && cancelled.body == r#"{"cancelled":true}"#
    );
    record!(
        "DELETE",
        "/api/v0/podcore/membership/join/{podId}/{peerId}",
        "mutation-side-effects-and-readback",
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_joins("pod:differential-workflow")
            .is_empty()
    );
    let repeated_cancel = super::route_http_request("DELETE", cancel_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/join/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        repeated_cancel.status == "404 Not Found"
    );

    // The frozen PodJoinLeaveController uses the same pending-request
    // contract for leave cancellation: a privileged member creates a
    // pending leave, DELETE consumes it with the exact cancelled body,
    // and a repeat DELETE is a real not-found branch.
    let leave_request = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("leave request for cancellation");
    assert_eq!(leave_request.status, "200 OK", "{}", leave_request.body);
    let leave_cancel_route =
        "/api/v0/podcore/membership/leave/pod%3Adifferential-workflow/differential-owner-peer";
    let cancelled_leave = super::route_http_request("DELETE", leave_cancel_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{leave_cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/leave/{podId}/{peerId}",
        "nominal-status-headers-body",
        cancelled_leave.status == "200 OK" && cancelled_leave.body == r#"{"cancelled":true}"#
    );
    record!(
        "DELETE",
        "/api/v0/podcore/membership/leave/{podId}/{peerId}",
        "mutation-side-effects-and-readback",
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_leaves("pod:differential-workflow")
            .is_empty()
    );
    let repeated_leave_cancel =
        super::route_http_request("DELETE", leave_cancel_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{leave_cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/leave/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        repeated_leave_cancel.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_membership_workflow.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-membership-workflow mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the pod-channel-messages route's
/// cases, independently re-derived from `pod_channel_messages_are_
/// durable_shaped_and_incremental`'s real sender-identity-spoofing
/// rejection (a caller cannot post as a different `senderPeerId`),
/// membership-required history reads (a non-member 403s even for a
/// public pod), and `?since=` incremental-cursor pagination.
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
async fn controller_api_differential_pod_channel_messages() {
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
    let path = "/api/v0/pods/pod-differential-1/channels/general/messages";
    let created = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-differential-1","name":"Pod Differential One","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"}]},"requestingPeerId":"differential-peer-1"}"#,
        &state,
    )
    .await
    .expect("create pod for messages");
    assert_eq!(created.status, "201 Created");

    state
        .pods
        .write()
        .await
        .join("pod-differential-1", "differential-peer-1".to_owned())
        .expect("join pod as fixture member");

    // PodMessageStorageController exposes count and search as separate
    // read contracts from channel history. The frozen oracle returns a
    // numeric zero / empty array for a member pod with no matching
    // messages, then returns the durable message rows after publish.
    let empty_count = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/general/count",
        None,
        "",
        &state,
    )
    .await
    .expect("empty message count");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/{channelId}/count",
        "nominal-status-headers-body",
        empty_count.status == "200 OK" && empty_count.body == "0"
    );
    let empty_search = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=first",
        None,
        "",
        &state,
    )
    .await
    .expect("empty message search");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "nominal-status-headers-body",
        empty_search.status == "200 OK" && empty_search.body == "[]"
    );

    let spoofed = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"spoofed","senderPeerId":"differential-peer-1"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "missing-empty-or-conflict-state",
        spoofed.status == "403 Forbidden"
    );

    *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
        "differential-public-intruder",
        "secret",
    ));
    let forbidden = super::route_http_request("GET", path, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{path}: {error}"));
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );
    *state.runtime_credentials.write().await = None;

    let first = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"first","senderPeerId":"tester","signature":"sig"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "nominal-status-headers-body",
        first.status == "200 OK"
            && first_json["sent"] == true
            && first_json["messageId"].as_str().map(str::len) == Some(32)
    );

    let initial = super::route_http_request("GET", path, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{path}: {error}"));
    let initial_json = serde_json::from_str::<serde_json::Value>(&initial.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "nominal-status-headers-body",
        initial.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "populated-dynamic-state",
        initial_json.as_array().map(Vec::len) == Some(1)
            && initial_json[0]["podId"] == "pod-differential-1"
            && initial_json[0]["channelId"] == "general"
            && initial_json[0]["senderPeerId"] == "tester"
            && initial_json[0]["body"] == "first"
            && initial_json[0]["sigVersion"] == 1
    );
    let populated_count = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/general/count",
        None,
        "",
        &state,
    )
    .await
    .expect("populated message count");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/{channelId}/count",
        "populated-dynamic-state",
        populated_count.status == "200 OK" && populated_count.body == "1"
    );
    let populated_search = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=first",
        None,
        "",
        &state,
    )
    .await
    .expect("populated message search");
    let populated_search_json =
        serde_json::from_str::<serde_json::Value>(&populated_search.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "populated-dynamic-state",
        populated_search.status == "200 OK"
            && populated_search_json.as_array().is_some_and(|entries| {
                entries.len() == 1
                    && entries[0]["podId"] == "pod-differential-1"
                    && entries[0]["channelId"] == "general"
                    && entries[0]["body"] == "first"
            })
    );
    let no_result_search = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=not-present",
        None,
        "",
        &state,
    )
    .await
    .expect("no-result message search");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "missing-empty-or-conflict-state",
        no_result_search.status == "200 OK" && no_result_search.body == "[]"
    );
    let malformed_search = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed message search");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "malformed-path-query-or-body",
        malformed_search.status == "400 Bad Request"
    );
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "mutation-side-effects-and-readback",
        first.status == "200 OK"
            && first_json["sent"] == true
            && initial_json.as_array().map(Vec::len) == Some(1)
            && initial_json[0]["body"] == "first"
            && initial_json[0]["senderPeerId"] == "tester"
    );
    let cursor = initial_json[0]["timestampUnixMs"]
        .as_u64()
        .unwrap_or_default();

    let second = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"second","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    assert_eq!(second.status, "200 OK");
    {
        let mut channels = state.pod_channels.write().await;
        let latest = channels
            .list("pod-differential-1", "general", None)
            .pop()
            .expect("at least one message");
        if latest.timestamp_unix_ms == cursor {
            channels
                .append(
                    "pod-differential-1".to_owned(),
                    "general".to_owned(),
                    "tester".to_owned(),
                    "third".to_owned(),
                    String::new(),
                    cursor + 1,
                )
                .expect("append disambiguating message");
        }
    }
    let incremental_route = format!("{path}?since={cursor}");
    let incremental = super::route_http_request("GET", &incremental_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{incremental_route}: {error}"));
    let incremental_json =
        serde_json::from_str::<serde_json::Value>(&incremental.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "mutation-side-effects-and-readback",
        incremental.status == "200 OK"
            && incremental_json.as_array().is_some_and(|entries| {
                !entries.is_empty()
                    && entries.iter().all(|message| {
                        message["timestampUnixMs"].as_u64().unwrap_or_default() > cursor
                    })
            })
    );

    let invalid_cursor_route = format!("{path}?since=-1");
    let invalid_cursor = super::route_http_request("GET", &invalid_cursor_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{invalid_cursor_route}: {error}"));
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "malformed-path-query-or-body",
        invalid_cursor.status == "400 Bad Request"
    );

    let invalid_body = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "malformed-path-query-or-body",
        invalid_body.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_channel_messages.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-channel-messages mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the PodMessageStorageController cleanup,
/// projection, rebuild-index, and vacuum contracts. The fixtures exercise
/// durable state, reloads, concurrent idempotent mutations, and confined
/// persisted-file failures instead of crediting static success responses.
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
async fn controller_api_differential_podcore_message_storage() {
    let target = "slskdn";
    let global_cleanup_route = "/api/v0/podcore/messages/cleanup";
    let channel_cleanup_route = "/api/v0/podcore/messages/{podId}/{channelId}/cleanup";
    let stats_route = "/api/v0/podcore/messages/stats";
    let search_route = "/api/v0/podcore/messages/{podId}/search";
    let count_route = "/api/v0/podcore/messages/{podId}/{channelId}/count";
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
        ($state:ident, $pod_id:expr) => {{
            $state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                        "podId": $pod_id,
                        "name": "Message storage differential",
                        "isPublic": true,
                        "channels": [{
                            "channelId": "general",
                            "kind": 0,
                            "name": "General"
                        }]
                    }))
                    .expect("deserialize message storage pod"),
                    "message-owner".to_owned(),
                )
                .expect("create message storage pod");
        }};
    }

    macro_rules! seed_messages {
        ($state:ident, $pod_id:expr, $old_id:expr, $new_id:expr) => {{
            let mut messages = $state.pod_channels.write().await;
            messages
                .append_with_id(
                    $old_id.to_owned(),
                    $pod_id.to_owned(),
                    "general".to_owned(),
                    "message-owner".to_owned(),
                    "old message".to_owned(),
                    String::new(),
                    1,
                    1,
                )
                .expect("seed old message");
            messages
                .append_with_id(
                    $new_id.to_owned(),
                    $pod_id.to_owned(),
                    "general".to_owned(),
                    "message-owner".to_owned(),
                    "new message".to_owned(),
                    String::new(),
                    100,
                    1,
                )
                .expect("seed new message");
        }};
    }

    macro_rules! block_message_path {
        ($state:ident) => {{
            let path = $state.config.state_dir.join("pod-channel-messages.json");
            fs::remove_file(&path).expect("remove message storage file");
            fs::create_dir(&path).expect("block message storage path");
            path
        }};
    }

    let message_env = || {
        MapEnv::default()
            .with("SLSK_USERNAME", "message-owner")
            .with("SLSK_PASSWORD", "test-secret")
    };

    // Global cleanup returns the frozen zero-count result on an empty
    // store, including when the persisted file does not yet exist.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=10",
            None,
            "",
            &state,
        )
        .await
        .expect("empty global message cleanup");
        record!(
            "DELETE",
            global_cleanup_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "0"
        );
    }

    // Global cleanup removes only messages older than the requested
    // cursor and leaves the newer row in the live store.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-mutation",
            "00000000-0000-4000-8000-000000000101",
            "00000000-0000-4000-8000-000000000102"
        );
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("mutating global message cleanup");
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            global_cleanup_route,
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && response.body == "1" && remaining == 1
        );
    }

    // A cleanup result remains present after reconstructing the message
    // store from its persisted state file.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-restart",
            "00000000-0000-4000-8000-000000000201",
            "00000000-0000-4000-8000-000000000202"
        );
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("restart global message cleanup");
        let loaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
            .expect("reload global message cleanup");
        record!(
            "DELETE",
            global_cleanup_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && response.body == "1"
                && loaded.stats().total_messages == 1
        );
    }

    // Two concurrent global cleanup requests are serialized by the store:
    // exactly one removes the old row and the other is an idempotent zero.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-concurrent",
            "00000000-0000-4000-8000-000000000301",
            "00000000-0000-4000-8000-000000000302"
        );
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "DELETE",
                    "/api/v0/podcore/messages/cleanup?olderThan=50",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let removed_one = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "1")
        });
        let removed_zero = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "0")
        });
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            global_cleanup_route,
            "concurrency-and-idempotency",
            removed_one && removed_zero && remaining == 1
        );
    }

    // A persisted-file failure returns the frozen 500 body and does not
    // silently report a successful cleanup.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-runtime",
            "00000000-0000-4000-8000-000000000401",
            "00000000-0000-4000-8000-000000000402"
        );
        let messages_path = block_message_path!(state);
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime global message cleanup");
        record!(
            "DELETE",
            global_cleanup_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while cleaning up messages")
        );
        fs::remove_dir(&messages_path).expect("remove blocked global message path");
    }

    // Channel-scoped cleanup has the same durable and idempotent contract
    // while retaining the pod/channel filter.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-empty");
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-empty/general/cleanup?olderThan=10",
            None,
            "",
            &state,
        )
        .await
        .expect("empty channel message cleanup");
        record!(
            "DELETE",
            channel_cleanup_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "0"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-mutation");
        seed_messages!(
            state,
            "pod:message-channel-mutation",
            "00000000-0000-4000-8000-000000000501",
            "00000000-0000-4000-8000-000000000502"
        );
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-mutation/general/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("mutating channel message cleanup");
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            channel_cleanup_route,
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && response.body == "1" && remaining == 1
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-restart");
        seed_messages!(
            state,
            "pod:message-channel-restart",
            "00000000-0000-4000-8000-000000000601",
            "00000000-0000-4000-8000-000000000602"
        );
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-restart/general/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("restart channel message cleanup");
        let loaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
            .expect("reload channel message cleanup");
        record!(
            "DELETE",
            channel_cleanup_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && response.body == "1"
                && loaded.stats().total_messages == 1
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-concurrent");
        seed_messages!(
            state,
            "pod:message-channel-concurrent",
            "00000000-0000-4000-8000-000000000701",
            "00000000-0000-4000-8000-000000000702"
        );
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "DELETE",
                    "/api/v0/podcore/messages/pod:message-channel-concurrent/general/cleanup?olderThan=50",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let removed_one = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "1")
        });
        let removed_zero = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "0")
        });
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            channel_cleanup_route,
            "concurrency-and-idempotency",
            removed_one && removed_zero && remaining == 1
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-runtime");
        seed_messages!(
            state,
            "pod:message-channel-runtime",
            "00000000-0000-4000-8000-000000000801",
            "00000000-0000-4000-8000-000000000802"
        );
        let messages_path = block_message_path!(state);
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-runtime/general/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime channel message cleanup");
        record!(
            "DELETE",
            channel_cleanup_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while cleaning up channel messages")
        );
        fs::remove_dir(&messages_path).expect("remove blocked channel message path");
    }

    // Read projections surface storage failures using their frozen
    // controller-specific messages.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-stats-runtime",
            "00000000-0000-4000-8000-000000000901",
            "00000000-0000-4000-8000-000000000902"
        );
        let messages_path = block_message_path!(state);
        let response = super::route_http_request("GET", stats_route, None, "", &state)
            .await
            .expect("runtime message stats");
        record!(
            "GET",
            stats_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting storage statistics")
        );
        fs::remove_dir(&messages_path).expect("remove blocked stats message path");
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-search-runtime");
        seed_messages!(
            state,
            "pod:message-search-runtime",
            "00000000-0000-4000-8000-000000001001",
            "00000000-0000-4000-8000-000000001002"
        );
        let messages_path = block_message_path!(state);
        let response = super::route_http_request(
            "GET",
            "/api/v0/podcore/messages/pod:message-search-runtime/search?query=message",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime message search");
        record!(
            "GET",
            search_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while searching messages")
        );
        fs::remove_dir(&messages_path).expect("remove blocked search message path");
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-count-empty");
        let response = super::route_http_request(
            "GET",
            "/api/v0/podcore/messages/pod:message-count-empty/general/count",
            None,
            "",
            &state,
        )
        .await
        .expect("empty message count");
        record!(
            "GET",
            count_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "0"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-count-runtime");
        seed_messages!(
            state,
            "pod:message-count-runtime",
            "00000000-0000-4000-8000-000000001101",
            "00000000-0000-4000-8000-000000001102"
        );
        let messages_path = block_message_path!(state);
        let response = super::route_http_request(
            "GET",
            "/api/v0/podcore/messages/pod:message-count-runtime/general/count",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime message count");
        record!(
            "GET",
            count_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting message count")
        );
        fs::remove_dir(&messages_path).expect("remove blocked count message path");
    }

    // Both maintenance actions retain their frozen true result on normal
    // paths and return their separate 500 contracts on storage failure.
    for (action, route, error_message) in [
        (
            "rebuild-index",
            "/api/v0/podcore/messages/rebuild-index",
            "An error occurred while rebuilding search index",
        ),
        (
            "vacuum",
            "/api/v0/podcore/messages/vacuum",
            "An error occurred while vacuuming database",
        ),
    ] {
        let action_path = format!("/api/v0/podcore/messages/{action}");

        {
            let (state, _receiver) = test_state_with_env(message_env());
            let responses =
                futures_util::future::join_all((0..2).map(|_| {
                    let state = Arc::clone(&state);
                    let action_path = action_path.clone();
                    async move {
                        super::route_http_request("POST", &action_path, None, "", &state).await
                    }
                }))
                .await;
            record!(
                "POST",
                route,
                "concurrency-and-idempotency",
                responses.iter().all(|response| {
                    response.as_ref().is_ok_and(|response| {
                        response.status == "200 OK" && response.body == "true"
                    })
                })
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            let response =
                super::route_http_request("POST", &action_path, None, "not-json", &state)
                    .await
                    .expect("malformed maintenance body");
            record!(
                "POST",
                route,
                "malformed-path-query-or-body",
                response.status == "200 OK" && response.body == "true"
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            let response = super::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("empty maintenance action");
            let loaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
                .expect("reload empty maintenance action");
            record!(
                "POST",
                route,
                "missing-empty-or-conflict-state",
                response.status == "200 OK"
                    && response.body == "true"
                    && loaded.stats().total_messages == 0
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            seed_messages!(
                state,
                "pod:message-maintenance-mutation",
                "00000000-0000-4000-8000-000000001201",
                "00000000-0000-4000-8000-000000001202"
            );
            let response = super::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("mutating maintenance action");
            let loaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
                .expect("reload mutating maintenance action");
            record!(
                "POST",
                route,
                "mutation-side-effects-and-readback",
                response.status == "200 OK"
                    && response.body == "true"
                    && loaded.stats().total_messages == 2
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            seed_messages!(
                state,
                "pod:message-maintenance-restart",
                "00000000-0000-4000-8000-000000001301",
                "00000000-0000-4000-8000-000000001302"
            );
            let response = super::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("restart maintenance action");
            let loaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
                .expect("reload restart maintenance action");
            record!(
                "POST",
                route,
                "restart-persistence-or-reset",
                response.status == "200 OK"
                    && response.body == "true"
                    && loaded.stats().total_messages == 2
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            seed_messages!(
                state,
                "pod:message-maintenance-runtime",
                "00000000-0000-4000-8000-000000001401",
                "00000000-0000-4000-8000-000000001402"
            );
            let messages_path = block_message_path!(state);
            let response = super::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("runtime maintenance action");
            record!(
                "POST",
                route,
                "runtime-failure-and-timeout",
                response.status == "500 Internal Server Error"
                    && response.body.contains(error_message)
            );
            fs::remove_dir(&messages_path).expect("remove blocked maintenance path");
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_message_storage.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-message-storage mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
