//! Controller full quarantine differential ownership.

use super::*;

/// Bulk differential proof crediting 7 quarantine-jury routes' cases,
/// independently re-derived from `quarantine_jury_requires_real_
/// quorum_before_accepting_or_releasing`'s real signed-verdict
/// validation, quorum enforcement, and idempotent-acceptance checks
/// (reuses the shared `quarantine_signed_verdict_json` fixture
/// builder, not the original test function). slskdN-only (confirmed
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
pub(super) async fn controller_api_differential_quarantine_jury() {
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

    let created = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"differential","jurors":["juror-x","juror-y","juror-z"],"evidence":[{"type":"hash","reference":"opaque-ref-differential"}],"minJurorVotes":2}"#,
        &state,
    )
    .await
    .expect("create quarantine request");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    let request_id = created_json["request"]["requestId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests",
        "nominal-status-headers-body",
        created.status == "200 OK" && !request_id.is_empty()
    );

    let empty_routes = crate::route_http_request(
        "GET",
        "/api/v0/quarantine-jury/requests/empty-differential/routes",
        None,
        "",
        &state,
    )
    .await
    .expect("empty quarantine route attempts");
    let empty_routes_json =
        serde_json::from_str::<serde_json::Value>(&empty_routes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "missing-empty-or-conflict-state",
        empty_routes.status == "200 OK" && empty_routes_json == serde_json::json!([])
    );

    let missing_route = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests/missing-differential/routes",
        None,
        "{}",
        &state,
    )
    .await
    .expect("missing quarantine request route");
    let missing_route_json =
        serde_json::from_str::<serde_json::Value>(&missing_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "missing-empty-or-conflict-state",
        missing_route.status == "404 Not Found"
            && missing_route_json["requestId"] == "missing-differential"
            && missing_route_json["success"] == false
            && missing_route_json["errorMessage"] == "Request not found."
    );

    let requests =
        crate::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
            .await
            .expect("list quarantine requests");
    let requests_json =
        serde_json::from_str::<serde_json::Value>(&requests.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests",
        "populated-dynamic-state",
        requests.status == "200 OK"
            && requests_json
                .as_array()
                .is_some_and(|items| { items.iter().any(|item| item["requestId"] == request_id) })
    );

    let request_detail_route = format!("/api/v0/quarantine-jury/requests/{request_id}");
    let request_detail = crate::route_http_request("GET", &request_detail_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{request_detail_route}: {error}"));
    let request_detail_json =
        serde_json::from_str::<serde_json::Value>(&request_detail.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}",
        "nominal-status-headers-body",
        request_detail.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}",
        "populated-dynamic-state",
        request_detail_json["requestId"] == request_id
            && request_detail_json["localReason"] == "differential"
            && request_detail_json["status"] == "Pending"
    );

    let unlisted = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &quarantine_signed_verdict_json(
            &request_id,
            "not-a-juror-differential",
            "ReleaseCandidate",
        )
        .to_string(),
        &state,
    )
    .await
    .expect("unlisted juror verdict");
    let mut verdicts_malformed_pass = unlisted.status == "400 Bad Request";

    let unsigned = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &format!(
            r#"{{"requestId":"{request_id}","juror":"juror-x","verdict":"ReleaseCandidate"}}"#
        ),
        &state,
    )
    .await
    .expect("unsigned verdict");
    let unsigned_json =
        serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap_or_default();
    verdicts_malformed_pass &= unsigned.status == "400 Bad Request"
        && unsigned_json["errors"].as_array().is_some_and(|errors| {
            errors
                .iter()
                .any(|error| error == "Signed juror verdict is required.")
        });

    let mut tampered = quarantine_signed_verdict_json(&request_id, "juror-x", "ReleaseCandidate");
    tampered["verdict"] = serde_json::json!("UpholdQuarantine");
    let tampered_result = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &tampered.to_string(),
        &state,
    )
    .await
    .expect("tampered verdict");
    let tampered_json =
        serde_json::from_str::<serde_json::Value>(&tampered_result.body).unwrap_or_default();
    verdicts_malformed_pass &= tampered_result.status == "400 Bad Request"
        && tampered_json["errors"].as_array().is_some_and(|errors| {
            errors
                .iter()
                .any(|error| error == "Signature payload hash does not match verdict contents.")
        });
    record!(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        "malformed-path-query-or-body",
        verdicts_malformed_pass
    );

    let too_early = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("accept too early");
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
        "missing-empty-or-conflict-state",
        too_early.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&too_early.body).unwrap_or_default()
                ["isAccepted"]
                == false
    );

    let package_too_early = crate::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/release-package"),
        None,
        "",
        &state,
    )
    .await
    .expect("release package too early");
    let package_too_early_json =
        serde_json::from_str::<serde_json::Value>(&package_too_early.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/release-package",
        "missing-empty-or-conflict-state",
        package_too_early.status == "400 Bad Request"
            && package_too_early_json["isReady"] == false
            && package_too_early_json["errors"][0]
                .as_str()
                .is_some_and(|error| error.contains("has not been accepted"))
    );

    let mut verdicts_nominal_pass = true;
    for juror in ["juror-x", "juror-y"] {
        let verdict = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(&request_id, juror, "ReleaseCandidate").to_string(),
            &state,
        )
        .await
        .expect("real signed verdict");
        verdicts_nominal_pass &= verdict.status == "200 OK";
    }
    record!(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        "nominal-status-headers-body",
        verdicts_nominal_pass
    );

    let aggregate_route = format!("/api/v0/quarantine-jury/requests/{request_id}/aggregate");
    let aggregate = crate::route_http_request("GET", &aggregate_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{aggregate_route}: {error}"));
    let aggregate_json =
        serde_json::from_str::<serde_json::Value>(&aggregate.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/aggregate",
        "nominal-status-headers-body",
        aggregate.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/aggregate",
        "populated-dynamic-state",
        aggregate_json["recommendation"] == "ReleaseCandidate"
            && aggregate_json["totalVerdicts"] == 2
            && aggregate_json["requiredVotes"] == 2
            && aggregate_json["quorumReached"] == true
    );

    let review_route = format!("/api/v0/quarantine-jury/requests/{request_id}/review");
    let review = crate::route_http_request("GET", &review_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{review_route}: {error}"));
    let review_json = serde_json::from_str::<serde_json::Value>(&review.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/review",
        "nominal-status-headers-body",
        review.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/review",
        "populated-dynamic-state",
        review_json["canAcceptReleaseCandidate"] == true
            && review_json["verdicts"].as_array().map(Vec::len) == Some(2)
    );

    let accept_route =
        format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate");
    let accepted = crate::route_http_request(
        "POST",
        &accept_route,
        None,
        r#"{"acceptedBy":"differential-operator"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{accept_route}: {error}"));
    let accepted_json =
        serde_json::from_str::<serde_json::Value>(&accepted.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
        "nominal-status-headers-body",
        accepted.status == "200 OK"
            && accepted_json["isAccepted"] == true
            && accepted_json["decision"]["acceptedBy"] == "differential-operator"
    );

    let reaccepted = crate::route_http_request("POST", &accept_route, None, "{}", &state)
        .await
        .unwrap_or_else(|error| panic!("{accept_route}: {error}"));
    let reaccepted_json =
        serde_json::from_str::<serde_json::Value>(&reaccepted.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
        "mutation-side-effects-and-readback",
        reaccepted.status == "200 OK"
            && reaccepted_json["decision"]["id"] == accepted_json["decision"]["id"]
    );

    let package_route = format!("/api/v0/quarantine-jury/requests/{request_id}/release-package");
    let package = crate::route_http_request("GET", &package_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{package_route}: {error}"));
    let package_json = serde_json::from_str::<serde_json::Value>(&package.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/release-package",
        "nominal-status-headers-body",
        package.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/release-package",
        "populated-dynamic-state",
        package_json["isReady"] == true
            && package_json["package"]["requestId"] == request_id
            && package_json["package"]["currentAggregate"]["recommendation"] == "ReleaseCandidate"
            && package_json["package"]["verdicts"].as_array().map(Vec::len) == Some(2)
    );

    let routes_route = format!("/api/v0/quarantine-jury/requests/{request_id}/routes");
    let bad_route = crate::route_http_request(
        "POST",
        &routes_route,
        None,
        r#"{"targetJurors":["not-a-juror-differential"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{routes_route}: {error}"));
    let bad_route_json =
        serde_json::from_str::<serde_json::Value>(&bad_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "malformed-path-query-or-body",
        bad_route.status == "400 Bad Request"
            && bad_route_json["success"] == false
            && bad_route_json["errorMessage"]
                .as_str()
                .is_some_and(|error| error.contains("safe jurors"))
    );

    let real_route = crate::route_http_request(
        "POST",
        &routes_route,
        None,
        r#"{"targetJurors":["juror-x"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{routes_route}: {error}"));
    let real_route_json =
        serde_json::from_str::<serde_json::Value>(&real_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "runtime-failure-and-timeout",
        real_route.status == "400 Bad Request"
            && real_route_json["success"] == false
            && real_route_json["errorMessage"] == "Routing backend is not available."
    );

    let routes = crate::route_http_request("GET", &routes_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{routes_route}: {error}"));
    let routes_json = serde_json::from_str::<serde_json::Value>(&routes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "nominal-status-headers-body",
        routes.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "populated-dynamic-state",
        routes_json.as_array().is_some_and(|attempts| {
            attempts
                .iter()
                .any(|attempt| attempt["requestId"] == request_id)
        })
    );
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "mutation-side-effects-and-readback",
        routes_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["requestId"] == request_id
                    && attempt["success"] == false
                    && attempt["errorMessage"] == "Routing backend is not available."
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("quarantine_jury.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api quarantine-jury mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
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
pub(super) async fn controller_api_differential_quarantine_jury_open_cases() {
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
    let json_value = |response: &crate::routing::HttpResponse| {
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
        let malformed = crate::route_http_request(
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
            crate::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
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
        let runtime = crate::route_http_request(
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
        let malformed = crate::route_http_request(
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
            crate::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
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
        let runtime = crate::route_http_request(
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
        let malformed = crate::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .expect("quarantine detail malformed response");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing_path = format!("/api/v0/quarantine-jury/{suffix}");
        let missing = crate::route_http_request("GET", &missing_path, None, "", &state)
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
        let runtime = crate::route_http_request("GET", &missing_path, None, "", &runtime_state)
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
        let malformed = crate::route_http_request(
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
        let runtime = crate::route_http_request(
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
        let malformed = crate::route_http_request(
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
        let runtime = crate::route_http_request(
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
            crate::route_http_request("POST", "/api/v0/quarantine-jury/requests", None, "", &state)
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
        let runtime = crate::route_http_request(
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-mutation-request", &["mutation-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine request mutation response");
        let listed =
            crate::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-reset-request", &["reset-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine request reset seed");
        let (restarted_state, _restarted_receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request(
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
            crate::route_http_request(
                "POST",
                "/api/v0/quarantine-jury/requests",
                None,
                &concurrent_a,
                &state
            ),
            crate::route_http_request(
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
            crate::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
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
        let malformed = crate::route_http_request(
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
        let runtime = crate::route_http_request(
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
        let restarted = crate::route_http_request(
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-accept-concurrent", &["accept-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine accept request seed");
        let verdict = crate::route_http_request(
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
            crate::route_http_request("POST", accept_path, None, "{}", &state),
            crate::route_http_request("POST", accept_path, None, "{}", &state)
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-route-nominal", &["route-juror"], 1),
            &state,
        )
        .await
        .expect("quarantine route request seed");
        let route = crate::route_http_request(
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
        let restarted = crate::route_http_request(
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
        let created = crate::route_http_request(
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
            crate::route_http_request("POST", route_path, None, route_body, &state),
            crate::route_http_request("POST", route_path, None, route_body, &state)
        );
        let left = left.expect("left quarantine route concurrency response");
        let right = right.expect("right quarantine route concurrency response");
        let routes = crate::route_http_request("GET", route_path, None, "", &state)
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-verdict-runtime", &["verdict-runtime"], 1),
            &runtime_state,
        )
        .await
        .expect("quarantine verdict runtime request seed");
        let verdict = crate::route_http_request(
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/requests",
            None,
            &request_body("open-verdict-mutation", &["verdict-mutation"], 1),
            &state,
        )
        .await
        .expect("quarantine verdict mutation request seed");
        let verdict = crate::route_http_request(
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
        let aggregate = crate::route_http_request(
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
        let missing = crate::route_http_request(
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
        let created = crate::route_http_request(
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
            crate::route_http_request(
                "POST",
                "/api/v0/quarantine-jury/verdicts",
                None,
                &concurrent_a,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/quarantine-jury/verdicts",
                None,
                &concurrent_b,
                &state
            )
        );
        let left = left.expect("left quarantine verdict concurrency response");
        let right = right.expect("right quarantine verdict concurrency response");
        let aggregate = crate::route_http_request(
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
