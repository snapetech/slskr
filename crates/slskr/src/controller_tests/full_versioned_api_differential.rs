//! Controller full versioned api differential ownership.

use super::*;

/// Bulk differential proof for two more real, deterministic (no
/// fixture data needed) branches of the shared production contract
/// `versioned_get_failure_contract` (near line 16817, called
/// for every GET request before its own handler): a fixed list of
/// routes that require a query value and 400 without one, and 2 routes
/// that are unconditionally not-found/not-configured for the slskdN
/// compatibility profile. Credits `malformed-path-query-or-body` (the
/// missing-required-query routes) and `missing-empty-or-conflict-state`
/// (the always-404 routes) manifest cases.
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
pub(super) async fn controller_api_differential_versioned_get_contract_fixed_route_responses() {
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
        scheme: String,
    }

    fn credential_header(scheme: &str) -> &'static str {
        if scheme == "api_key" {
            "ApiKey admin-token"
        } else {
            "Bearer admin-token"
        }
    }

    // (path, manifest case this proves)
    const MISSING_REQUIRED_QUERY_ROUTES: [&str; 9] = [
        "/api/v0/library/health/issues/by-type",
        "/api/v0/multisource/search",
        "/api/v0/multisource/users",
        "/api/v0/opinions/summary",
        "/api/v0/podcore/content/metadata",
        "/api/v0/podcore/content/search",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "/api/v0/telemetry/reports/transfers/leaderboard",
    ];
    const ALWAYS_NOT_FOUND_ROUTES: [&str; 2] =
        ["/api/v0/security/canaries", "/api/v0/security/tor/status"];

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
        // Skip targets that don't declare any of these routes at all --
        // no manifest denominator exists there, so there's nothing to
        // credit and no reason to spin up a hermetic state.
        let declares_any = MISSING_REQUIRED_QUERY_ROUTES
            .iter()
            .chain(ALWAYS_NOT_FOUND_ROUTES.iter())
            .any(|route| scheme_for(route).is_some());
        if !declares_any {
            continue;
        }

        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_API_TOKEN", "admin-token"),
        );

        for route in MISSING_REQUIRED_QUERY_ROUTES {
            let Some(scheme) = scheme_for(route) else {
                continue;
            };
            let response = crate::route_http_request(
                "GET",
                route,
                Some(credential_header(scheme)),
                "",
                &state,
            )
            .await
            .expect("route response");
            let pass = response.status == "400 Bad Request"
                && response.body == "{\"error\":\"A required query value is missing\"}";
            if !pass {
                mismatches.push(format!(
                    "{target} GET {route} (missing query): got {} {}",
                    response.status, response.body
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }

        for route in ALWAYS_NOT_FOUND_ROUTES {
            let Some(scheme) = scheme_for(route) else {
                continue;
            };
            let response = crate::route_http_request(
                "GET",
                route,
                Some(credential_header(scheme)),
                "",
                &state,
            )
            .await
            .expect("route response");
            let pass = response.status == "404 Not Found";
            if !pass {
                mismatches.push(format!(
                    "{target} GET {route} (always-not-found): got {} {}",
                    response.status, response.body
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": route,
                "case": "missing-empty-or-conflict-state",
                "pass": pass,
            }));
        }

        if target == "slskdn" {
            let response = crate::route_http_request(
                "GET",
                "/api/v0/security/adversarial",
                Some("Bearer admin-token"),
                "",
                &state,
            )
            .await
            .expect("route response");
            let pass = response.status == "404 Not Found"
                && response.body == "Adversarial features are not configured";
            if !pass {
                mismatches.push(format!(
                    "{target} GET /api/v0/security/adversarial: got {} {}",
                    response.status, response.body
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": "/api/v0/security/adversarial",
                "case": "missing-empty-or-conflict-state",
                "pass": pass,
            }));
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("versioned_get_contract_fixed_route_responses.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api fixed-route mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the "referenced resource does not exist
/// yet" family of branches in `versioned_get_failure_contract` (near
/// near line 16817): conversations, jobs, search responses,
/// listening-party radio, MusicBrainz artist lookups, profile, shares,
/// transfer entries, browse status, and multisource search results all
/// real-check against slskR's own live stores (not a stub) before
/// falling through to a real handler. Proven here against a fresh,
/// empty hermetic state -- no fixture rows are needed to prove the
/// "does not exist" branch, only that a real lookup happens and the
/// real oracle-matching negative response comes back. Credits
/// `missing-empty-or-conflict-state` manifest cases.
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
pub(super) async fn controller_api_differential_versioned_get_contract_missing_resource_responses()
{
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
        scheme: String,
    }

    #[derive(Clone, Copy)]
    enum Expected {
        StandardNotFound,
        EmptyBodyNotFound,
    }

    const CHECKS: [(&str, &str, Expected); 11] = [
        (
            "/api/v0/conversations/{username}",
            "/api/v0/conversations/no-such-user",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/jobs/{id}",
            "/api/v0/jobs/no-such-job",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/searches/{id}/responses",
            "/api/v0/searches/no-such-search/responses",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/listening-party/radio/{partyId}/{contentId}",
            "/api/v0/listening-party/radio/no-such-party/no-such-content",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "/api/v0/musicbrainz/artist/definitely-no-such-artist/discography-coverage",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
            "/api/v0/musicbrainz/overlays/artist/definitely-no-such-artist/release-graph",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/profile/{peerId}",
            "/api/v0/profile/no-such-peer",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/shares/{id}",
            "/api/v0/shares/no-such-share",
            Expected::EmptyBodyNotFound,
        ),
        (
            "/api/v0/transfers/downloads/{username}",
            "/api/v0/transfers/downloads/no-such-user",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/transfers/uploads/{username}",
            "/api/v0/transfers/uploads/no-such-user",
            Expected::StandardNotFound,
        ),
        (
            "/api/v0/users/{username}/browse/status",
            "/api/v0/users/no-such-user/browse/status",
            Expected::StandardNotFound,
        ),
    ];
    const SEARCH_RESULTS_REQUIRED: (&str, &str) = (
        "/api/v0/multisource/users/{username}/files",
        "/api/v0/multisource/users/no-such-user/files",
    );

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

        for (route_template, concrete_path, expected) in CHECKS {
            let header = credential_header(scheme_for(route_template).unwrap_or("any"));
            let response =
                crate::route_http_request("GET", concrete_path, Some(header), "", &state)
                    .await
                    .expect("route response");
            let native_search_response_id =
                target == "slskdn" && route_template == "/api/v0/searches/{id}/responses";
            let pass = if native_search_response_id {
                // SearchResponsesController binds {id} as Guid, so an
                // invalid value is model-validation 400 rather than a
                // missing-record 404. slskd retains its legacy text id.
                response.status == "400 Bad Request"
                    && response.body == "{\"error\":\"The request is invalid\"}"
            } else {
                match expected {
                    Expected::StandardNotFound => {
                        response.status == "404 Not Found"
                            && response.body == "{\"error\":\"not found\"}"
                    }
                    Expected::EmptyBodyNotFound => {
                        response.status == "404 Not Found" && response.body.is_empty()
                    }
                }
            };
            if !pass {
                mismatches.push(format!(
                    "{target} GET {concrete_path}: got {} {}",
                    response.status, response.body
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": route_template,
                "case": if native_search_response_id {
                    "malformed-path-query-or-body"
                } else {
                    "missing-empty-or-conflict-state"
                },
                "pass": pass,
            }));
        }

        let (route_template, concrete_path) = SEARCH_RESULTS_REQUIRED;
        let header = credential_header(scheme_for(route_template).unwrap_or("any"));
        let response = crate::route_http_request("GET", concrete_path, Some(header), "", &state)
            .await
            .expect("route response");
        let pass = response.status == "400 Bad Request"
            && response.body
                == "{\"error\":\"No search results. Call /users?searchText=... first\"}";
        if !pass {
            mismatches.push(format!(
                "{target} GET {concrete_path}: got {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": route_template,
            "case": "missing-empty-or-conflict-state",
            "pass": pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("versioned_get_contract_missing_resource_responses.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api missing-resource mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the large table-driven validation
/// block of `versioned_openapi_validation_and_large_dtos_match_
/// native_contracts` (26 routes' rejection-path contracts: malformed
/// body, missing/conflicting resource state, and dependency-
/// unavailable runtime failures). Independently re-derived from the
/// same real request/response pairs. The remainder of that source
/// test (multisource/musicbrainz/songid/taste/portforwarding/
/// podcore large-DTO success-path checks) is a separate, still-open
/// batch -- see session memory. slskdN-only (confirmed against the
/// frozen registry route-by-route).
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
pub(super) async fn controller_api_differential_versioned_openapi_validation_rejections() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    let (state, _receiver) = test_state();

    let cases: Vec<(&str, &str, &str, &str, &str)> = vec![
        (
            "POST",
            "/api/v0/searches",
            "/api/v0/searches",
            r#"{"searchText":"differential","acquisitionProfile":"differential"}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/mesh/sync/differential-peer",
            "/api/v0/mesh/sync/{username}",
            "{}",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/mesh/message",
            "/api/v0/mesh/message",
            "",
            "415 Unsupported Media Type",
        ),
        (
            "POST",
            "/api/v0/multisource/download",
            "/api/v0/multisource/download",
            r#"{"filename":"Differential.flac","fileSize":1,"sources":[]}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets",
            "/api/v0/musicbrainz/targets",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000004"}"#,
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits",
            "/api/v0/musicbrainz/overlays/edits",
            r#"{"id":"00000000-0000-4000-8000-000000000004","evidence":[]}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/00000000-0000-4000-8000-000000000004/approve-export",
            "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
            "{}",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/00000000-0000-4000-8000-000000000004/routes",
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "{}",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/share-grants",
            "/api/v0/share-grants",
            r#"{"collectionId":"00000000-0000-4000-8000-000000000004"}"#,
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/users/differential-peer/directory",
            "/api/v0/users/{username}/directory",
            r#"{"directory":"/tmp/slskdn-differential"}"#,
            "503 Service Unavailable",
        ),
        (
            "PUT",
            "/api/v0/conversations/differential-peer/1",
            "/api/v0/conversations/{username}/{id}",
            "",
            "503 Service Unavailable",
        ),
        (
            "PUT",
            "/api/v0/conversations/differential-peer",
            "/api/v0/conversations/{username}",
            "",
            "503 Service Unavailable",
        ),
        (
            "DELETE",
            "/api/v0/conversations/differential-peer",
            "/api/v0/conversations/{username}",
            "",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/conversations/differential-peer",
            "/api/v0/conversations/{username}",
            r#""differential""#,
            "503 Service Unavailable",
        ),
        (
            "POST",
            "/api/v0/rooms/joined",
            "/api/v0/rooms/joined",
            r#""differential""#,
            "503 Service Unavailable",
        ),
        (
            "POST",
            "/api/v0/session",
            "/api/v0/session",
            r#"{"username":"differential-peer","password":"differential"}"#,
            "401 Unauthorized",
        ),
        (
            "POST",
            "/api/v0/pods/pod%3A00000000000000000000000000000004/channels/general/bind",
            "/api/v0/pods/{podId}/channels/{channelId}/bind",
            r#"{"roomName":"Differential","mode":"differential"}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/pods/pod%3A00000000000000000000000000000004/channels/general/unbind",
            "/api/v0/pods/{podId}/channels/{channelId}/unbind",
            "",
            "404 Not Found",
        ),
        ("PATCH", "/api/v0/options", "/api/v0/options", "{}", "403 Forbidden"),
        (
            "PUT",
            "/api/v0/relay/agent",
            "/api/v0/relay/agent",
            "",
            "403 Forbidden",
        ),
        (
            "DELETE",
            "/api/v0/relay/agent",
            "/api/v0/relay/agent",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/relay/controller/files/differential",
            "/api/v0/relay/controller/files/{token}",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/relay/controller/shares/differential",
            "/api/v0/relay/controller/shares/{token}",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/soulseek/mesh-rendezvous/interest",
            "/api/v0/soulseek/mesh-rendezvous/interest",
            "",
            "403 Forbidden",
        ),
        (
            "DELETE",
            "/api/v0/soulseek/mesh-rendezvous/interest",
            "/api/v0/soulseek/mesh-rendezvous/interest",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/streams/content%3Amusic%3Arecording%3Amissing-differential/ticket",
            "/api/v0/streams/{contentId}/ticket",
            "{}",
            "404 Not Found",
        ),
    ];

    for (method, path, route, body, expected_status) in cases {
        let case = match expected_status {
            "400 Bad Request" | "415 Unsupported Media Type" => "malformed-path-query-or-body",
            "503 Service Unavailable" => "runtime-failure-and-timeout",
            _ => "missing-empty-or-conflict-state",
        };
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        let pass = response.status == expected_status;
        if !pass {
            mismatches.push(format!(
                "{target} {method} {route} [{case}]: expected {expected_status}, got {}",
                response.status
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": method,
            "route": route,
            "case": case,
            "pass": pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("versioned_openapi_validation_rejections.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned-openapi-validation-rejections mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the large-DTO success-path
/// remainder of `versioned_openapi_validation_and_large_dtos_match_
/// native_contracts` (everything after the rejection-path table
/// credited above): multisource/test, musicbrainz library-bloom
/// preview, SongID run's full ~28-field DTO, taste-recommendations
/// (+ its 3 sub-route validation guards), portforwarding start/stop
/// (start's real Pod-membership guard was previously shadowed by a
/// routing-table typo mapping to the wrong internal route -- fixed,
/// see the original test's comment), realm-subject-indexes authority-
/// decision (real `IsSafeOpaqueReference` validation, not an
/// unconditional accept/reject), and a podcore membership/backfill/
/// signing/verification/opinions/membership-removal sequence.
/// slskdN-only (confirmed against the frozen registry route-by-
/// route; `virtualsoulfind/shadow-index/sync/merge` has no registry
/// entry in either target and is skipped as genuinely unwireable).
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
pub(super) async fn controller_api_differential_versioned_openapi_large_dtos() {
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

    let multisource_test = crate::route_http_request(
        "POST",
        "/api/v0/multisource/test",
        None,
        r#"{"searchText":"differential"}"#,
        &state,
    )
    .await
    .expect("multisource test");
    let multisource_test_json =
        serde_json::from_str::<serde_json::Value>(&multisource_test.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/multisource/test",
        "nominal-status-headers-body",
        multisource_test.status == "200 OK"
            && multisource_test_json["searchText"] == "differential"
    );
    record!(
        "POST",
        "/api/v0/multisource/test",
        "mutation-side-effects-and-readback",
        [
            "downloadSuccess",
            "downloadTimeMs",
            "bytesDownloaded",
            "sourcesUsed",
            "outputPath",
            "finalHash",
            "averageSpeedMBps",
        ]
        .iter()
        .all(|key| multisource_test_json.get(*key).is_some())
    );

    let bloom = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"expectedItems":1,"falsePositiveRate":1,"saltId":"differential","rotatesAt":"2026-01-01T00:00:00Z"}"#,
        &state,
    )
    .await
    .expect("bloom preview");
    let bloom_json = serde_json::from_str::<serde_json::Value>(&bloom.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        "nominal-status-headers-body",
        bloom.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        "mutation-side-effects-and-readback",
        [
            "snapshotId",
            "scope",
            "saltId",
            "createdAt",
            "rotatesAt",
            "expectedItems",
            "falsePositiveRate",
            "bitSize",
            "hashFunctionCount",
            "itemCount",
            "fillRatio",
            "bitsBase64",
            "namespaceItemCounts",
            "privacyNotes",
        ]
        .iter()
        .all(|key| bloom_json.get(*key).is_some())
    );

    let songid = crate::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"differential"}"#,
        &state,
    )
    .await
    .expect("songid run large dto");
    let songid_json = serde_json::from_str::<serde_json::Value>(&songid.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/songid/runs",
        "mutation-side-effects-and-readback",
        songid_json["source"] == "differential"
            && [
                "sourceType",
                "query",
                "createdAt",
                "summary",
                "currentStage",
                "percentComplete",
                "artifactDirectory",
                "evidence",
                "tracks",
                "albums",
                "artists",
                "plans",
                "options",
                "scorecard",
                "assessment",
                "metadata",
                "provenance",
                "perturbations",
                "stems",
                "corpusMatches",
                "clips",
                "transcripts",
                "ocr",
                "comments",
                "chapters",
                "segments",
                "mixGroups",
                "identityAssessment",
                "syntheticAssessment",
            ]
            .iter()
            .all(|key| songid_json.get(*key).is_some())
    );

    let taste = crate::route_http_request(
        "POST",
        "/api/v0/taste-recommendations",
        None,
        r#"{"minimumTrustedSources":1}"#,
        &state,
    )
    .await
    .expect("taste recommendations");
    let taste_json = serde_json::from_str::<serde_json::Value>(&taste.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/taste-recommendations",
        "nominal-status-headers-body",
        taste.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/taste-recommendations",
        "mutation-side-effects-and-readback",
        [
            "minimumTrustedSources",
            "trustedActorCount",
            "candidateCount",
            "recommendations"
        ]
        .iter()
        .all(|key| taste_json.get(*key).is_some())
    );

    let invalid_work_ref =
        r#"{"workRef":{"@context":null,"domain":"music","title":"Differential"}}"#;
    for (path, route) in [
        (
            "/api/v0/taste-recommendations/wishlist",
            "/api/v0/taste-recommendations/wishlist",
        ),
        (
            "/api/v0/taste-recommendations/release-radar",
            "/api/v0/taste-recommendations/release-radar",
        ),
        (
            "/api/v0/taste-recommendations/graph-preview",
            "/api/v0/taste-recommendations/graph-preview",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, invalid_work_ref, &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    let start = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        r#"{"localPort":1024,"podId":"pod:differential","destinationHost":"example.invalid","destinationPort":1}"#,
        &state,
    )
    .await
    .expect("portforwarding start without membership");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "missing-empty-or-conflict-state",
        start.status == "403 Forbidden"
    );

    let stop = crate::route_http_request("POST", "/api/v0/portforwarding/stop/1", None, "", &state)
        .await
        .expect("portforwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "nominal-status-headers-body",
        stop.body == r#"{"message":"Port forwarding stopped"}"#
    );

    // Not creditable itself (no registry entry in either frozen
    // target), but required fixture setup: this registers the
    // "index" index in the "default-realm" realm that the
    // authority-decision calls below require to exist. The
    // `payloadHash` is a real signature check over these exact
    // field values (see `compute_payload_hash` in
    // realm_subject_index.rs) -- it must match this literal content
    // verbatim, not a "differential"-renamed variant, or the merge
    // is rejected as a signature mismatch and the index is never
    // registered.
    let index_sync_fixture = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        r#"{"records":[{"recordingId":"route-audit","peerIds":["peer-a"],"updatedAt":1}],"realmIndexes":[{"id":"index","realmId":"default-realm","subjectNamespace":"music","revision":1,"entries":[{"subjectId":"route-audit","workRef":{"domain":"music","title":"Route Audit","externalIds":{"musicbrainz:recording":"route-audit"}},"externalIds":{},"aliases":[]}],"signature":{"signer":"default-governance","value":"signature","payloadHash":"a890273abd9ae483659d6b08c9bc83dd82cda93bdefc9c940412c91f2ccddcc6"}}]}"#,
        &state,
    )
    .await
    .expect("register realm index fixture");
    assert_eq!(
        index_sync_fixture.status, "200 OK",
        "realm index fixture setup failed: {}",
        index_sync_fixture.body
    );

    let unsafe_decision = crate::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index/authority-decision",
        None,
        r#"{"enabled":true,"decidedBy":"/etc/passwd","note":"differential"}"#,
        &state,
    )
    .await
    .expect("unsafe authority decision");
    let unsafe_decision_json =
        serde_json::from_str::<serde_json::Value>(&unsafe_decision.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "malformed-path-query-or-body",
        unsafe_decision.status == "400 Bad Request"
            && unsafe_decision_json["isAccepted"] == false
            && unsafe_decision_json["errors"][0]
                .as_str()
                .is_some_and(|error| error.contains("opaque and safe"))
    );

    let decision = crate::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index/authority-decision",
        None,
        r#"{"enabled":true,"decidedBy":"differential","note":"differential"}"#,
        &state,
    )
    .await
    .expect("safe authority decision");
    let decision_json =
        serde_json::from_str::<serde_json::Value>(&decision.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "nominal-status-headers-body",
        decision.status == "200 OK"
            && decision_json["isAccepted"] == true
            && decision_json["enabled"] == true
            && decision_json["errors"] == serde_json::json!([])
    );

    let pod_id = "pod:00000000000000000000000000000005";
    crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        &format!(
            r#"{{"podId":"{pod_id}","name":"Differential","visibility":"Listed","contentId":"content:music:recording:differential-large-dto","tags":[],"channels":[],"externalBindings":[]}}"#
        ),
        &state,
    )
    .await
    .expect("create pod for large-dto sequence");

    let joined = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        &format!(
            r#"{{"podId":"{pod_id}","peerId":"00000000-0000-4000-8000-000000000005","requestedRole":"differential","publicKey":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","timestampUnixMs":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","message":"differential","nonce":"differential"}}"#
        ),
        &state,
    )
    .await
    .expect("join pod membership");
    let joined_json = serde_json::from_str::<serde_json::Value>(&joined.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "nominal-status-headers-body",
        joined.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "mutation-side-effects-and-readback",
        joined_json["podId"] == pod_id
    );

    let empty_backfill_route = format!("/api/v0/podcore/backfill/{pod_id}/sync");
    let empty_backfill =
        crate::route_http_request("POST", &empty_backfill_route, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{empty_backfill_route}: {error}"));
    record!(
        "POST",
        "/api/v0/podcore/backfill/{podId}/sync",
        "malformed-path-query-or-body",
        empty_backfill.status == "400 Bad Request"
    );

    let last_seen_route = format!("/api/v0/podcore/backfill/{pod_id}/general/last-seen");
    let last_seen = crate::route_http_request("PUT", &last_seen_route, None, "1", &state)
        .await
        .unwrap_or_else(|error| panic!("{last_seen_route}: {error}"));
    record!(
        "PUT",
        "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
        "nominal-status-headers-body",
        last_seen.status == "200 OK" && last_seen.body.is_empty()
    );

    let verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &format!(
            r#"{{"messageId":"message","podId":"{pod_id}","channelId":"00000000-0000-4000-8000-000000000005","senderPeerId":"00000000-0000-4000-8000-000000000005","body":"differential","timestampUnixMs":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","sigVersion":1}}"#
        ),
        &state,
    )
    .await
    .expect("signing verify");
    record!(
        "POST",
        "/api/v0/podcore/signing/verify",
        "nominal-status-headers-body",
        verified.body == r#"{"isValid":true}"#
    );

    let evidence = crate::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &format!(
            r#"{{"messageId":"message","podId":"{pod_id}","channelId":"00000000-0000-4000-8000-000000000005","senderPeerId":"00000000-0000-4000-8000-000000000005","body":"differential","timestampUnixMs":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","sigVersion":1}}"#
        ),
        &state,
    )
    .await
    .expect("verification message");
    let evidence_json =
        serde_json::from_str::<serde_json::Value>(&evidence.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/verification/message",
        "nominal-status-headers-body",
        evidence.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/verification/message",
        "mutation-side-effects-and-readback",
        [
            "isValid",
            "isFromValidMember",
            "hasValidSignature",
            "isNotBanned",
            "errorMessage"
        ]
        .iter()
        .all(|key| evidence_json.get(*key).is_some())
    );

    let opinion_route = format!("/api/v0/podcore/{pod_id}/opinions");
    let opinion = crate::route_http_request(
        "POST",
        &opinion_route,
        None,
        r#"{"contentId":"content:music:recording:differential-large-dto","score":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{opinion_route}: {error}"));
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions",
        "malformed-path-query-or-body",
        opinion.status == "400 Bad Request"
    );

    let removed_route =
        format!("/api/v0/podcore/membership/{pod_id}/00000000-0000-4000-8000-000000000005");
    let removed = crate::route_http_request("DELETE", &removed_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{removed_route}: {error}"));
    let removed_json = serde_json::from_str::<serde_json::Value>(&removed.body).unwrap_or_default();
    record!(
        "DELETE",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "nominal-status-headers-body",
        removed.status == "200 OK"
    );
    record!(
        "DELETE",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "mutation-side-effects-and-readback",
        removed_json["success"] == true && removed_json.get("dhtKey").is_some()
    );
    let repeated_removed = crate::route_http_request("DELETE", &removed_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{removed_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        repeated_removed.status == "200 OK"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("versioned_openapi_large_dtos.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned-openapi-large-dtos mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 10 auxiliary mutation routes'
/// cases, independently re-derived from `versioned_auxiliary_
/// mutations_match_native_status_and_dto_contracts`'s real status/DTO
/// checks and `enabled_warm_cache_hints_normalize_persist_and_bound_
/// popularity`'s real persisted-popularity-counter and input-bounds
/// checks. slskdN-only (confirmed against the frozen registry;
/// `/api/v0/events/{eventType}` has no registry entry in either
/// target and is skipped as genuinely unwireable).
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
pub(super) async fn controller_api_differential_versioned_auxiliary_mutations() {
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

    for path in [
        "/api/slskdn/warm-cache/hints",
        "/api/v0/slskdn/warm-cache/hints",
    ] {
        let response = crate::route_http_request(
            "POST",
            path,
            None,
            r#"{"mb_release_ids":[],"mb_artist_ids":[],"mb_label_ids":[]}"#,
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "POST",
            "/api/v0/slskdn/warm-cache/hints",
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
                && response.body == r#"{"error":"Warm cache not enabled"}"#
        );
    }

    let shares_delete_before =
        crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("no scan to cancel");
    record!(
        "DELETE",
        "/api/v0/shares",
        "missing-empty-or-conflict-state",
        shares_delete_before.status == "404 Not Found"
    );

    let scan = crate::route_http_request("PUT", "/api/v0/shares", None, "", &state)
        .await
        .expect("start share scan");
    record!(
        "PUT",
        "/api/v0/shares",
        "nominal-status-headers-body",
        scan.status == "200 OK"
    );

    let cancelled = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .expect("cancel completed scan");
    record!(
        "DELETE",
        "/api/v0/shares",
        "mutation-side-effects-and-readback",
        cancelled.status == "404 Not Found"
    );

    let invite = crate::route_http_request(
        "POST",
        "/api/v0/profile/invite",
        None,
        r#"{"expiresInHours":1}"#,
        &state,
    )
    .await
    .expect("profile invite");
    let invite_json = serde_json::from_str::<serde_json::Value>(&invite.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/profile/invite",
        "nominal-status-headers-body",
        invite.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/profile/invite",
        "mutation-side-effects-and-readback",
        invite_json["inviteLink"]
            .as_str()
            .is_some_and(|link| link.starts_with("slskdn://invite/"))
            && invite_json["friendCode"].as_str().is_some_and(|code| code
                .split('-')
                .map(str::len)
                .collect::<Vec<_>>()
                == vec![5, 4, 4, 3])
    );

    let csv_body = r#"{"csvText":"Artist,Track Title,Album\nDifferential Auxiliary,Parity Track,Contract Album","filter":"differential-auxiliary","enabled":true,"autoDownload":true,"maxResults":1,"includeAlbum":true}"#;
    let imported = crate::route_http_request(
        "POST",
        "/api/v0/wishlist/import/csv",
        None,
        csv_body,
        &state,
    )
    .await
    .expect("import csv");
    let imported_json =
        serde_json::from_str::<serde_json::Value>(&imported.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/wishlist/import/csv",
        "nominal-status-headers-body",
        imported.status == "200 OK"
            && imported_json["totalRows"] == 1
            && imported_json["createdCount"] == 1
            && imported_json["duplicateCount"] == 0
            && imported_json["skippedCount"] == 0
            && imported_json["createdItems"][0]["searchText"]
                == "Differential Auxiliary Contract Album"
            && imported_json["createdItems"][0]["maxResults"] == 1
            && imported_json["createdItems"][0].get("artist").is_none()
    );

    let duplicate = crate::route_http_request(
        "POST",
        "/api/v0/wishlist/import/csv",
        None,
        csv_body,
        &state,
    )
    .await
    .expect("import duplicate csv");
    let duplicate_json =
        serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/wishlist/import/csv",
        "mutation-side-effects-and-readback",
        duplicate_json["createdCount"] == 0 && duplicate_json["duplicateCount"] == 1
    );
    record!(
        "POST",
        "/api/v0/wishlist/import/csv",
        "concurrency-and-idempotency",
        duplicate_json["createdCount"] == 0 && duplicate_json["duplicateCount"] == 1
    );

    let invalid_content = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/validate",
        None,
        r#""differential""#,
        &state,
    )
    .await
    .expect("validate invalid content id");
    let invalid_content_json =
        serde_json::from_str::<serde_json::Value>(&invalid_content.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/content/validate",
        "malformed-path-query-or-body",
        invalid_content_json["isValid"] == false
            && invalid_content_json["contentId"] == "differential"
            && invalid_content_json["errorMessage"]
                == "Invalid content ID format. Expected: content:<domain>:<type>:<id>"
    );

    let valid_content = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/validate",
        None,
        r#""content:music:recording:differential""#,
        &state,
    )
    .await
    .expect("validate valid content id");
    let valid_content_json =
        serde_json::from_str::<serde_json::Value>(&valid_content.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/content/validate",
        "nominal-status-headers-body",
        valid_content_json["isValid"] == true
            && valid_content_json["metadata"]["domain"] == "music"
            && valid_content_json["metadata"]["type"] == "recording"
    );

    let group = crate::route_http_request(
        "POST",
        "/api/v0/sharegroups",
        None,
        r#"{"name":"Differential Group"}"#,
        &state,
    )
    .await
    .expect("create sharegroup");
    let group_json = serde_json::from_str::<serde_json::Value>(&group.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/sharegroups",
        "nominal-status-headers-body",
        group.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v0/sharegroups",
        "mutation-side-effects-and-readback",
        uuid::Uuid::parse_str(group_json["id"].as_str().unwrap_or_default()).is_ok()
            && group_json["name"] == "Differential Group"
            && group_json["ownerUserId"] == "Anonymous"
            && group_json["createdAt"].as_str().is_some()
            && group_json.get("members").is_none()
    );

    let profile = crate::route_http_request(
        "PUT",
        "/api/v0/profile/me",
        None,
        r#"{"displayName":"Differential","avatar":"differential","capabilities":1,"endpoints":[]}"#,
        &state,
    )
    .await
    .expect("update profile");
    let profile_json = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/profile/me",
        "nominal-status-headers-body",
        profile.status == "200 OK"
    );
    record!(
        "PUT",
        "/api/v0/profile/me",
        "mutation-side-effects-and-readback",
        [
            "peerId",
            "publicKey",
            "displayName",
            "avatar",
            "capabilities",
            "endpoints",
            "createdAt",
            "expiresAt",
            "signature",
        ]
        .iter()
        .all(|key| profile_json.get(*key).is_some())
            && profile_json["displayName"] == "Differential"
            && profile_json["capabilities"] == 1
    );

    let verdict = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        r#"{"requestId":"00000000-0000-4000-8000-000000000006","juror":"differential","verdict":"NeedsManualReview"}"#,
        &state,
    )
    .await
    .expect("submit verdict for missing request");
    record!(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        "missing-empty-or-conflict-state",
        verdict.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&verdict.body).unwrap_or_default()
                == serde_json::json!({"isValid": false, "errors": ["Request not found."]})
    );

    let (warm_state, _warm_receiver) = test_state();
    std::fs::write(
        warm_state.config.state_dir.join("slskd.yml"),
        "warmCache:\n  enabled: true\n",
    )
    .expect("write warm-cache config");
    let accepted = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[" rel-1 ","REL-1"],"mb_artist_ids":["artist-1"],"mb_label_ids":[]}"#,
        &warm_state,
    )
    .await
    .expect("accept warm-cache hints");
    record!(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        "nominal-status-headers-body",
        accepted.status == "200 OK" && accepted.body == r#"{"accepted":true}"#
    );

    let features = warm_state.controller_features.read().await;
    let popularity_pass = features
        .get("warm-cache/popularity/mb:release:rel-1")
        .is_some_and(|value| value["hits"] == 1)
        && features
            .get("warm-cache/popularity/mb:artist:artist-1")
            .is_some_and(|value| value["hits"] == 1);
    drop(features);
    record!(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        "mutation-side-effects-and-readback",
        popularity_pass
    );

    let invalid_type = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[42]}"#,
        &warm_state,
    )
    .await
    .expect("reject non-string release id");
    let oversized = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        &serde_json::json!({"mb_release_ids": ["x".repeat(129)]}).to_string(),
        &warm_state,
    )
    .await
    .expect("reject oversized release id");
    record!(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        "malformed-path-query-or-body",
        invalid_type.status == "400 Bad Request" && oversized.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("versioned_auxiliary_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned-auxiliary-mutations mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the empty slskdN Soulseek recommendation
/// DTOs. The versioned controller returns the protocol-shaped pair of
/// arrays, even when there are no local or global recommendations.
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
pub(super) async fn controller_api_differential_versioned_soulseek_recommendations() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let expected = serde_json::json!({
        "recommendations": [],
        "unrecommendations": [],
    });
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for path in [
        "/api/v0/soulseek/recommendations",
        "/api/v0/soulseek/recommendations/global",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned Soulseek recommendations response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == expected;
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
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
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_soulseek_recommendations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the versioned Soulseek item-discovery DTOs.
/// The legacy handlers retain their compatibility envelopes, while the
/// versioned routes expose the slskdN `ItemRecommendations` and
/// `ItemSimilarUsers` property shapes.
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
pub(super) async fn controller_api_differential_versioned_soulseek_item_discovery() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let cases = [
        (
            "/api/v0/soulseek/items/ambient/recommendations",
            "/api/v0/soulseek/items/{item}/recommendations",
            serde_json::json!({
                "item": "ambient",
                "recommendations": [],
            }),
        ),
        (
            "/api/v0/soulseek/items/ambient/similar-users",
            "/api/v0/soulseek/items/{item}/similar-users",
            serde_json::json!({
                "item": "ambient",
                "usernames": [],
            }),
        ),
    ];
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for (path, ledger_route, expected) in cases {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned Soulseek item-discovery response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == expected;
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": ledger_route,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_soulseek_item_discovery.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the empty versioned Soulseek similar-user
/// collection. The legacy route keeps its mesh envelope; slskdN exposes
/// the underlying `IReadOnlyCollection<SimilarUser>` directly.
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
pub(super) async fn controller_api_differential_versioned_soulseek_similar_users() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        crate::route_http_request("GET", "/api/v0/soulseek/users/similar", None, "", &state)
            .await
            .expect("versioned Soulseek similar-users response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == serde_json::json!([]);
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/soulseek/users/similar",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_soulseek_similar_users.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/soulseek/users/similar: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Bulk differential proof for the slskdN auto-replace status DTO.
/// The versioned controller exposes only the five fields below; the
/// legacy compatibility route intentionally keeps its older projection.
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
pub(super) async fn controller_api_differential_versioned_autoreplace_status() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = crate::route_http_request("GET", "/api/v0/autoreplace", None, "", &state)
        .await
        .expect("versioned auto-replace status response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "enabled": false,
        "lastRunAt": null,
        "lastRunProcessedCount": 0,
        "lastRunReplacedCount": 0,
        "intervalSeconds": 300,
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/autoreplace",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_autoreplace_status.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/autoreplace: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the populated slskdN auto-replace status.  The
/// controller must reflect the real enable mutation instead of returning
/// the disabled baseline on every GET.
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
pub(super) async fn controller_api_differential_versioned_autoreplace_populated_state() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let enabled = crate::route_http_request("PUT", "/api/v0/autoreplace/enable", None, "", &state)
        .await
        .expect("enable versioned auto-replace");
    assert_eq!(enabled.status, "200 OK", "{}", enabled.body);

    let response = crate::route_http_request("GET", "/api/v0/autoreplace", None, "", &state)
        .await
        .expect("populated versioned auto-replace status response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value["enabled"] == true
        && value["lastRunAt"].is_null()
        && value["lastRunProcessedCount"] == 0
        && value["lastRunReplacedCount"] == 0
        && value["intervalSeconds"] == 300;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/autoreplace",
        "case": "populated-dynamic-state",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_autoreplace_populated_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/autoreplace: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Bulk differential proof for the three fixed-version slskdN native
/// projections that were previously rejected by the shared slskd
/// unsupported-version guard: mesh-health and the signal-system
/// configuration/status DTOs.  The assertions use the frozen controller
/// property names (`routingNodes`, `storedKeys`, `active_channels`, and
/// nested `statistics`) so the rows cannot be credited by a merely
/// successful status code or by the legacy unversioned compatibility
/// shapes.
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
pub(super) async fn controller_api_differential_versioned_mesh_health_and_signals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [nominal-status-headers-body]",
                    $route
                ));
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

    let advanced = serde_json::json!({
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
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );

    let health = crate::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("versioned mesh health response");
    let health_json =
        serde_json::from_str::<serde_json::Value>(&health.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/mesh/health",
        health.status == "200 OK"
            && health.content_type.starts_with("application/json")
            && health_json["routingNodes"] == 0
            && health_json["storedKeys"] == 0
            && health_json["contentPeerHints"] == 0
            && health_json["generatedAt"].as_str().is_some()
            && health_json.get("status").is_none()
    );

    let config = crate::route_http_request("GET", "/api/v0/signals/config", None, "", &state)
        .await
        .expect("versioned signal configuration response");
    let config_json =
        serde_json::from_str::<serde_json::Value>(&config.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/signals/config",
        config.status == "200 OK"
            && config.content_type.starts_with("application/json")
            && config_json["enabled"] == false
            && config_json["deduplication_cache_size"] == 2048
            && config_json["default_ttl_seconds"] == 450
            && config_json["mesh_channel"]["enabled"] == false
            && config_json["mesh_channel"]["priority"] == 3
            && config_json["mesh_channel"]["require_active_session"] == true
            && config_json["bt_extension_channel"]["enabled"] == true
            && config_json["bt_extension_channel"]["priority"] == 4
            && config_json["bt_extension_channel"]["require_active_session"] == false
            && config_json.get("meshChannel").is_none()
    );

    let status = crate::route_http_request("GET", "/api/v0/signals/status", None, "", &state)
        .await
        .expect("versioned signal status response");
    let status_json =
        serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/signals/status",
        status.status == "200 OK"
            && status.content_type.starts_with("application/json")
            && status_json["enabled"] == false
            && status_json["active_channels"] == serde_json::json!([])
            && status_json["statistics"]["signals_sent"] == 0
            && status_json["statistics"]["signals_received"] == 0
            && status_json["statistics"]["duplicate_signals_dropped"] == 0
            && status_json["statistics"]["expired_signals_dropped"] == 0
            && status_json.get("activeChannels").is_none()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("versioned_mesh_health_and_signals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned mesh-health/signals mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining fixed-version SignalSystem
/// edge rows.  Both DTOs are configuration/process projections, so their
/// empty and closed-SQLite responses remain successful while malformed
/// extra path segments are rejected by routing.
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
pub(super) async fn controller_api_differential_versioned_signals_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} GET {} [{}]",
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());
    for route in ["/api/v0/signals/config", "/api/v0/signals/status"] {
        let malformed =
            crate::route_http_request("GET", &format!("{route}/extra"), None, "", &state)
                .await
                .expect("malformed signal path");
        record!(
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request("GET", route, None, "", &state)
            .await
            .expect("empty signal projection");
        let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or_default();
        let shape = if route.ends_with("/config") {
            empty_json.get("enabled").is_some()
                && empty_json.get("deduplication_cache_size").is_some()
        } else {
            empty_json.get("enabled").is_some()
                && empty_json.get("active_channels").is_some()
                && empty_json.get("statistics").is_some()
        };
        record!(
            route,
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty.content_type.starts_with("application/json") && shape
        );
    }

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("signal failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    for route in ["/api/v0/signals/config", "/api/v0/signals/status"] {
        let response = crate::route_http_request("GET", route, None, "", &failure_state)
            .await
            .expect("signal projection after database close");
        record!(
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && response.content_type.starts_with("application/json")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_signals_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize signal edge ledger"),
    )
    .expect("write signal edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} signal edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the frozen versioned capability peer
/// projections: all known peers, mesh-capable peers, and one peer's
/// detail.  The rows are populated from slskR's real signed-capability
/// store and assert the controller's public DTO fields, not merely a
/// non-404 response.  slskdN-only (confirmed against the registry).
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
pub(super) async fn controller_api_differential_versioned_capability_peer_projections() {
    let target = "slskdn";
    let username = "versioned-capability-peer";
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

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            username,
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));

    let peers = crate::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &state)
        .await
        .expect("versioned capability peers response");
    let peers_value =
        serde_json::from_str::<serde_json::Value>(&peers.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/capabilities/peers",
        peers.status == "200 OK"
            && peers.content_type == "application/json"
            && peers_value["count"] == 1
            && peers_value["peers"][0]["username"] == username
            && peers_value["peers"][0]["protocolVersion"] == 1
            && peers_value["peers"][0]["flagsValue"] == 8
            && peers_value["peers"][0]["canMeshSync"] == true
    );

    let mesh_peers =
        crate::route_http_request("GET", "/api/v0/capabilities/mesh-peers", None, "", &state)
            .await
            .expect("versioned mesh capability peers response");
    let mesh_peers_value = serde_json::from_str::<serde_json::Value>(&mesh_peers.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/capabilities/mesh-peers",
        mesh_peers.status == "200 OK"
            && mesh_peers.content_type == "application/json"
            && mesh_peers_value["count"] == 1
            && mesh_peers_value["peers"][0]["username"] == username
            && mesh_peers_value["peers"][0]["meshSeqId"] == 0
    );

    let peer = crate::route_http_request(
        "GET",
        &format!("/api/v0/capabilities/peers/{username}"),
        None,
        "",
        &state,
    )
    .await
    .expect("versioned capability peer detail response");
    let peer_value =
        serde_json::from_str::<serde_json::Value>(&peer.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/capabilities/peers/{username}",
        peer.status == "200 OK"
            && peer.content_type == "application/json"
            && peer_value["username"] == username
            && peer_value["protocolVersion"] == 1
            && peer_value["flagsValue"] == 8
            && peer_value["canMeshSync"] == true
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_capability_peer_projections.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned capability mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
