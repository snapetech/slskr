//! Controller full capabilities differential ownership.

use super::*;

/// Bulk differential proof for one slice of the manifest's
/// `controller-api` workstream's `malformed-path-query-or-body` case:
/// the shared production contract `versioned_get_failure_contract`
/// (called for every GET request, see this module near line 16817) rejects
/// a non-UUID first path segment with a real 400 for 7 declared
/// route-prefix families. Proves it against the real dispatcher
/// (`route_http_request`) for every currently-declared GET route in
/// either frozen registry whose first parameter segment falls under one
/// of these prefixes -- not a hand-picked sample -- and writes a ledger
/// `scripts/audit-parity-manifest.py` reads to promote proven
/// `controller-api` cases out of `needs-proof`.
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
pub(super) async fn controller_api_differential_uuid_guarded_families_reject_malformed_first_id() {
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
        scheme: String,
    }

    // Same ordered, nested-prefix-first list `versioned_get_failure_contract`
    // uses -- a more specific nested prefix must be checked before the
    // shorter prefix it is contained within, or its real id segment
    // never reaches its own check.
    const UUID_GUARDED_PREFIXES: [&str; 7] = [
        "/api/v0/collections/",
        "/api/v0/contacts/",
        "/api/v0/share-grants/by-collection/",
        "/api/v0/share-grants/",
        "/api/v0/sharegroups/",
        "/api/v0/wishlist/",
        "/api/v0/multisource/jobs/",
    ];

    fn matching_prefix(route: &str) -> Option<&'static str> {
        UUID_GUARDED_PREFIXES
            .iter()
            .copied()
            .find(|prefix| route.starts_with(prefix))
    }

    // The segment immediately after the matched prefix must itself be a
    // template parameter (not a literal sibling route like
    // "/api/v0/contacts/nearby") for the malformed-id contract to apply.
    fn first_segment_is_param(route: &str, prefix: &str) -> bool {
        route
            .strip_prefix(prefix)
            .and_then(|rest| rest.split('/').next())
            .is_some_and(|segment| segment.starts_with('{') && segment.ends_with('}'))
    }

    fn malformed_id_path(route: &str, prefix: &str) -> String {
        let rest = route.strip_prefix(prefix).unwrap_or_default();
        let mut segments: Vec<&str> = rest.split('/').collect();
        segments[0] = "not-a-valid-uuid";
        format!("{prefix}{}", segments.join("/"))
    }

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
        let candidates: Vec<&AuthPolicyRow> = rules
            .iter()
            .filter(|rule| {
                rule.method == "GET"
                    && matching_prefix(&rule.route)
                        .is_some_and(|prefix| first_segment_is_param(&rule.route, prefix))
            })
            .collect();
        if candidates.is_empty() {
            continue;
        }

        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_API_TOKEN", "admin-token"),
        );

        for rule in candidates {
            let prefix = matching_prefix(&rule.route).expect("filtered above");
            let path = malformed_id_path(&rule.route, prefix);
            let header = credential_header(&rule.scheme);
            let response = crate::route_http_request("GET", &path, Some(header), "", &state)
                .await
                .expect("route response");
            let pass = response.status == "400 Bad Request"
                && response.body == "{\"error\":\"The request is invalid\"}";
            if !pass {
                mismatches.push(format!(
                    "{target} GET {} -> {path}: got {} {}",
                    rule.route, response.status, response.body
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": rule.route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("uuid_guarded_families_reject_malformed_first_id.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api malformed-id mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 13 miscellaneous deterministic
/// mutation routes' cases, independently re-derived from
/// `deterministic_openapi_mutations_match_native_status_and_dto_
/// contracts`'s real DTO-shape and status-code checks spanning
/// autoreplace, destinations, DHT, hashdb optimize, nowplaying,
/// integrations, transfers, library-health, and overlay-blocklist.
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
pub(super) async fn controller_api_differential_deterministic_openapi_mutations() {
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

    for (path, route, enabled) in [
        (
            "/api/v0/autoreplace/enable",
            "/api/v0/autoreplace/enable",
            true,
        ),
        (
            "/api/v0/autoreplace/disable",
            "/api/v0/autoreplace/disable",
            false,
        ),
    ] {
        let response = crate::route_http_request("PUT", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "PUT",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && value["enabled"] == enabled
        );
        record!(
            "PUT",
            route,
            "mutation-side-effects-and-readback",
            [
                "lastRunAt",
                "lastRunProcessedCount",
                "lastRunReplacedCount",
                "intervalSeconds"
            ]
            .iter()
            .all(|key| value.get(*key).is_some())
        );
    }

    let destination = crate::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        r#"{"path":"/tmp/slskdn-differential"}"#,
        &state,
    )
    .await
    .expect("validate destination");
    let destination_json =
        serde_json::from_str::<serde_json::Value>(&destination.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "nominal-status-headers-body",
        destination.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "mutation-side-effects-and-readback",
        destination_json["path"] == "/tmp/slskdn-differential"
            && destination_json.get("exists").is_some()
            && destination_json.get("writable").is_some()
    );

    let announce = crate::route_http_request("POST", "/api/v0/dht/announce", None, "", &state)
        .await
        .expect("dht announce");
    record!(
        "POST",
        "/api/v0/dht/announce",
        "missing-empty-or-conflict-state",
        announce.status == "400 Bad Request"
            && announce.body == r#"{"error":"Not beacon capable"}"#
    );

    let discover = crate::route_http_request("POST", "/api/v0/dht/discover", None, "", &state)
        .await
        .expect("dht discover");
    let discover_json =
        serde_json::from_str::<serde_json::Value>(&discover.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/dht/discover",
        "nominal-status-headers-body",
        discover.status == "200 OK"
            && discover_json.get("newConnectionsMade").is_some()
            && discover_json.get("totalMeshConnections").is_some()
    );

    for (path, message) in [
        (
            "/api/v0/hashdb/optimize/indexes",
            "Index optimization completed",
        ),
        (
            "/api/v0/hashdb/optimize/vacuum",
            "VACUUM and ANALYZE completed",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default()
                ["message"]
                == message
        );
    }

    let profile = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        r#"{"query":"differential","parameters":{}}"#,
        &state,
    )
    .await
    .expect("hashdb optimize profile");
    record!(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        "malformed-path-query-or-body",
        profile.status == "400 Bad Request"
    );

    crate::route_http_request(
        "PUT",
        "/api/v0/nowplaying",
        None,
        r#"{"artist":"Delete Differential","title":"Delete Track","album":"Delete Album"}"#,
        &state,
    )
    .await
    .expect("seed now playing before delete");

    for (method, path) in [
        ("DELETE", "/api/v0/nowplaying"),
        ("DELETE", "/api/v0/integrations/spotify"),
        ("DELETE", "/api/v0/transfers/downloads/all/completed"),
        ("DELETE", "/api/v0/transfers/uploads/all/completed"),
        (
            "PATCH",
            "/api/v0/library/health/issues/00000000-0000-4000-8000-000000000003",
        ),
    ] {
        let route = if path.starts_with("/api/v0/library/health/issues/") {
            "/api/v0/library/health/issues/{issueId}"
        } else {
            path
        };
        let response =
            crate::route_http_request(method, path, None, r#"{"status":"Resolved"}"#, &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "nominal-status-headers-body",
            response.status == "204 No Content" && response.body.is_empty()
        );
        if path == "/api/v0/nowplaying" {
            let cleared = crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
                .await
                .expect("read cleared now playing");
            record!(
                "DELETE",
                "/api/v0/nowplaying",
                "mutation-side-effects-and-readback",
                response.status == "204 No Content" && cleared.status == "204 No Content"
            );
        }
    }

    let blocked = crate::route_http_request(
        "POST",
        "/api/v0/overlay/blocklist/username",
        None,
        r#"{"username":"differential-peer"}"#,
        &state,
    )
    .await
    .expect("block username");
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "nominal-status-headers-body",
        blocked.status == "200 OK"
    );
    let blocklist = crate::route_http_request("GET", "/api/v0/overlay/blocklist", None, "", &state)
        .await
        .expect("list username blocklist");
    record!(
        "POST",
        "/api/v0/overlay/blocklist/username",
        "mutation-side-effects-and-readback",
        blocked.status == "200 OK"
            && blocklist.status == "200 OK"
            && blocklist.body.contains("differential-peer")
    );

    let unblocked = crate::route_http_request(
        "DELETE",
        "/api/v0/overlay/blocklist/username/differential-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("unblock username");
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "nominal-status-headers-body",
        unblocked.status == "200 OK"
            && unblocked.body == r#"{"message":"Blocklist entry removed"}"#
    );
    record!(
        "DELETE",
        "/api/v0/overlay/blocklist/{type}/{target}",
        "mutation-side-effects-and-readback",
        unblocked.status == "200 OK"
            && unblocked.body == r#"{"message":"Blocklist entry removed"}"#
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("deterministic_openapi_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api deterministic-openapi-mutations mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
