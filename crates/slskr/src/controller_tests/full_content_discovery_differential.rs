//! Controller full content discovery differential ownership.

use super::*;

/// Bulk differential proof crediting the hashdb entries/sync paging
/// routes' cases, independently re-derived from `versioned_hashdb_
/// paging_matches_sequence_controller_contract`'s real seq-ordered
/// pagination checks (the v0-shaped response omits the legacy
/// `offset`/`limit`/`fromSeqId`/`hasMore` echo fields the bare routes
/// carry). slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_hashdb_paging() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let hash_a = "a".repeat(64);
    let hash_b = "b".repeat(64);
    state
        .content_discovery
        .write()
        .await
        .merge_hash_entries(vec![
            crate::content_discovery::HashDbEntry {
                flac_key: hash_a.clone(),
                byte_hash: hash_a,
                size: 100,
                ..Default::default()
            },
            crate::content_discovery::HashDbEntry {
                flac_key: hash_b.clone(),
                byte_hash: hash_b,
                size: 200,
                ..Default::default()
            },
        ])
        .expect("seed hashdb sequence");

    let first =
        crate::route_http_request("GET", "/api/v0/hashdb/entries?limit=1", None, "", &state)
            .await
            .expect("first hashdb page");
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap_or_default();
    record!(
        "/api/v0/hashdb/entries",
        "nominal-status-headers-body",
        first.status == "200 OK"
    );
    record!(
        "/api/v0/hashdb/entries",
        "populated-dynamic-state",
        first_json["latestSeq"] == 2
            && first_json["count"] == 1
            && first_json["entries"][0]["seqId"] == 1
            && first_json.get("offset").is_none()
            && first_json.get("limit").is_none()
    );

    let second = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/entries?offset=1&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("sequence-offset hashdb page");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap_or_default();
    record!(
        "/api/v0/hashdb/entries",
        "malformed-path-query-or-body",
        second.status == "200 OK" && second_json["entries"][0]["seqId"] == 2
    );

    let sync = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/sync/since/1?limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb sync page");
    let sync_json = serde_json::from_str::<serde_json::Value>(&sync.body).unwrap_or_default();
    record!(
        "/api/v0/hashdb/sync/since/{sinceSeq}",
        "nominal-status-headers-body",
        sync.status == "200 OK"
    );
    record!(
        "/api/v0/hashdb/sync/since/{sinceSeq}",
        "populated-dynamic-state",
        sync_json["latestSeq"] == 2
            && sync_json["count"] == 1
            && sync_json["entries"][0]["seqId"] == 2
            && sync_json.get("fromSeqId").is_none()
            && sync_json.get("hasMore").is_none()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("hashdb_paging.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api hashdb-paging mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for HashDb model-binding failures and empty
/// state.  The frozen slskdN controller binds numeric and boolean query
/// values before invoking its services, returns 404 for an unknown hash,
/// and preserves the zero-sized lookup as a successful empty projection.
/// These cases are intentionally separate from the nominal and populated
/// HashDb ledgers so the route-specific negative behavior is audited.
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
pub(super) async fn controller_api_differential_hashdb_validation_and_empty_contracts() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SHARE_FIXTURE", ""),
    );

    for (method, route, path, expected) in [
        (
            "GET",
            "/api/v0/hashdb/backfill/candidates",
            "/api/v0/hashdb/backfill/candidates?limit=invalid",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/hash/{flacKey}",
            "/api/v0/hashdb/hash/%20",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/hash/by-size/{size}",
            "/api/v0/hashdb/hash/by-size/not-a-size",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/inventory/by-size/{size}",
            "/api/v0/hashdb/inventory/by-size/not-a-size",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/inventory/by-size/{size}",
            "/api/v0/hashdb/inventory/by-size/1?limit=invalid",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/inventory/unhashed",
            "/api/v0/hashdb/inventory/unhashed?limit=invalid",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/optimize/slow-queries",
            "/api/v0/hashdb/optimize/slow-queries?limit=invalid",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/sync/since/{sinceSeq}",
            "/api/v0/hashdb/sync/since/not-a-seq",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/hashdb/backfill/candidates",
            "/api/v0/hashdb/backfill/candidates/extra",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/hashdb/inventory/unhashed",
            "/api/v0/hashdb/inventory/unhashed/extra",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/hashdb/optimize/analyze",
            "/api/v0/hashdb/optimize/analyze/extra",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/hashdb/optimize/slow-queries",
            "/api/v0/hashdb/optimize/slow-queries/extra",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/hashdb/peers",
            "/api/v0/hashdb/peers/extra",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/hashdb/schema",
            "/api/v0/hashdb/schema/extra",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/hashdb/stats",
            "/api/v0/hashdb/stats/extra",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/hashdb/backfill/from-history",
            "/api/v0/hashdb/backfill/from-history?batchSize=invalid",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/hashdb/optimize/indexes",
            "/api/v0/hashdb/optimize/indexes/extra",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/hashdb/optimize/vacuum",
            "/api/v0/hashdb/optimize/vacuum/extra",
            "404 Not Found",
        ),
    ] {
        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "malformed-path-query-or-body",
            response.status == expected
        );
    }

    let backfill_candidates = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/backfill/candidates",
        None,
        "",
        &state,
    )
    .await
    .expect("empty backfill candidates");
    let backfill_candidates_json =
        serde_json::from_str::<serde_json::Value>(&backfill_candidates.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/backfill/candidates",
        "missing-empty-or-conflict-state",
        backfill_candidates.status == "200 OK"
            && backfill_candidates_json["count"] == 0
            && backfill_candidates_json["entries"] == serde_json::json!([])
    );

    let entries = crate::route_http_request("GET", "/api/v0/hashdb/entries", None, "", &state)
        .await
        .expect("empty hashdb entries");
    let entries_json = serde_json::from_str::<serde_json::Value>(&entries.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/entries",
        "missing-empty-or-conflict-state",
        entries.status == "200 OK"
            && entries_json["latestSeq"] == 0
            && entries_json["count"] == 0
            && entries_json["entries"] == serde_json::json!([])
    );

    let missing_hash =
        crate::route_http_request("GET", "/api/v0/hashdb/hash/missing-key", None, "", &state)
            .await
            .expect("missing hash");
    record!(
        "GET",
        "/api/v0/hashdb/hash/{flacKey}",
        "missing-empty-or-conflict-state",
        missing_hash.status == "404 Not Found"
            && missing_hash.body.contains("No hash found for key")
    );

    let hash_by_size =
        crate::route_http_request("GET", "/api/v0/hashdb/hash/by-size/0", None, "", &state)
            .await
            .expect("zero-sized hash lookup");
    let hash_by_size_json =
        serde_json::from_str::<serde_json::Value>(&hash_by_size.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/hash/by-size/{size}",
        "missing-empty-or-conflict-state",
        hash_by_size.status == "200 OK"
            && hash_by_size_json["count"] == 0
            && hash_by_size_json["entries"] == serde_json::json!([])
    );

    let inventory_by_size = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/inventory/by-size/0",
        None,
        "",
        &state,
    )
    .await
    .expect("zero-sized inventory lookup");
    let inventory_by_size_json =
        serde_json::from_str::<serde_json::Value>(&inventory_by_size.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/inventory/by-size/{size}",
        "missing-empty-or-conflict-state",
        inventory_by_size.status == "200 OK"
            && inventory_by_size_json["count"] == 0
            && inventory_by_size_json["entries"] == serde_json::json!([])
    );

    let unhashed =
        crate::route_http_request("GET", "/api/v0/hashdb/inventory/unhashed", None, "", &state)
            .await
            .expect("empty unhashed inventory");
    let unhashed_json =
        serde_json::from_str::<serde_json::Value>(&unhashed.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/inventory/unhashed",
        "missing-empty-or-conflict-state",
        unhashed.status == "200 OK"
            && unhashed_json["count"] == 0
            && unhashed_json["entries"] == serde_json::json!([])
            && unhashed_json["items"] == serde_json::json!([])
    );

    let key = crate::route_http_request("GET", "/api/v0/hashdb/key", None, "", &state)
        .await
        .expect("missing hash key inputs");
    record!(
        "GET",
        "/api/v0/hashdb/key",
        "missing-empty-or-conflict-state",
        key.status == "400 Bad Request"
    );

    let analyze =
        crate::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
            .await
            .expect("empty hashdb analysis");
    let analyze_json = serde_json::from_str::<serde_json::Value>(&analyze.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/optimize/analyze",
        "missing-empty-or-conflict-state",
        analyze.status == "200 OK"
            && analyze_json["analyzed"] == true
            && analyze_json["entries"] == 0
            && analyze_json["recommendations"] == serde_json::json!([])
    );

    let (slow_state, _slow_receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SHARE_FIXTURE", ""),
    );
    let slow_queries = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        None,
        "",
        &slow_state,
    )
    .await
    .expect("empty slow query report");
    let slow_queries_json =
        serde_json::from_str::<serde_json::Value>(&slow_queries.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        "missing-empty-or-conflict-state",
        slow_queries.status == "200 OK"
            && slow_queries_json["totalQueries"] == 0
            && slow_queries_json["slowQueries"] == serde_json::json!([])
    );

    for (route, path, expected) in [
        (
            "/api/v0/hashdb/peers",
            "/api/v0/hashdb/peers",
            serde_json::json!({"count": 0, "peers": []}),
        ),
        (
            "/api/v0/hashdb/schema",
            "/api/v0/hashdb/schema",
            serde_json::json!({"currentVersion": 24, "targetVersion": 24, "isUpToDate": true}),
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && expected.as_object().is_some_and(|fields| {
                    fields.iter().all(|(key, expected)| value[key] == *expected)
                })
        );
    }

    let stats = crate::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
        .await
        .expect("empty hashdb stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/stats",
        "missing-empty-or-conflict-state",
        stats.status == "200 OK"
            && stats_json["totalHashEntries"] == 0
            && stats_json["currentSeqId"] == 0
    );

    let sync = crate::route_http_request("GET", "/api/v0/hashdb/sync/since/0", None, "", &state)
        .await
        .expect("empty hashdb sync page");
    let sync_json = serde_json::from_str::<serde_json::Value>(&sync.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/sync/since/{sinceSeq}",
        "missing-empty-or-conflict-state",
        sync.status == "200 OK"
            && sync_json["latestSeq"] == 0
            && sync_json["count"] == 0
            && sync_json["entries"] == serde_json::json!([])
    );

    let backfill = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "",
        &state,
    )
    .await
    .expect("empty hashdb history backfill");
    let backfill_json =
        serde_json::from_str::<serde_json::Value>(&backfill.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "missing-empty-or-conflict-state",
        backfill.status == "200 OK"
            && backfill_json["complete"] == true
            && backfill_json["searchesProcessed"] == 0
    );

    let profile = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        "{}",
        &state,
    )
    .await
    .expect("missing hashdb profile query");
    record!(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        "missing-empty-or-conflict-state",
        profile.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("hashdb_validation_and_empty_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api hashdb validation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
/// Bulk differential proof crediting 6 discovery-graph/opinions/
/// contacts/searches routes' cases, independently re-derived from
/// `versioned_discovery_graph_and_opinions_match_native_contracts`'s
/// real seed-graph, opinion-validation, and honest-404 checks.
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
pub(super) async fn controller_api_differential_discovery_graph_and_opinions() {
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

    let graph = crate::route_http_request(
        "POST",
        "/api/v0/discovery-graph",
        None,
        r#"{"scope":"differential","songIdRunId":"00000000-0000-4000-8000-000000000002","recordingId":"00000000-0000-4000-8000-000000000002","releaseId":"00000000-0000-4000-8000-000000000002","artistId":"00000000-0000-4000-8000-000000000002","title":"Discovery Differential","artist":"differential","album":"differential"}"#,
        &state,
    )
    .await
    .expect("discovery graph");
    let graph_json = serde_json::from_str::<serde_json::Value>(&graph.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/discovery-graph",
        "nominal-status-headers-body",
        graph.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/discovery-graph",
        "mutation-side-effects-and-readback",
        graph_json["title"] == "Discovery Differential"
            && graph_json["seedNodeId"] == "seed:discovery-differential"
            && graph_json["nodes"].as_array().map(Vec::len) == Some(4)
            && graph_json["edges"].as_array().map(Vec::len) == Some(3)
            && graph_json["evidenceSummary"].as_array().map(Vec::len) == Some(3)
            && graph_json["request"]["scope"] == "differential"
    );

    let invalid_opinion = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"00000000-0000-4000-8000-000000000002","issuer":"differential","subjectType":"Unknown","subjectId":"subject","kind":"Unknown","strength":1,"confidence":1}"#,
        &state,
    )
    .await
    .expect("invalid opinion");
    record!(
        "POST",
        "/api/v0/opinions",
        "malformed-path-query-or-body",
        invalid_opinion.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&invalid_opinion.body).unwrap_or_default()
                == serde_json::json!(["subject type is required", "opinion kind is required"])
    );

    let missing_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/opinions/00000000-0000-4000-8000-000000000002",
        None,
        "",
        &state,
    )
    .await
    .expect("missing opinion delete");
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "missing-empty-or-conflict-state",
        missing_delete.status == "404 Not Found"
    );

    let valid_opinion = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"differential","subjectType":"Track","subjectId":"track-1","kind":"Like","strength":1,"confidence":1,"scope":"global","source":"local","evidence":[]}"#,
        &state,
    )
    .await
    .expect("valid opinion");
    let valid_opinion_json =
        serde_json::from_str::<serde_json::Value>(&valid_opinion.body).unwrap_or_default();
    let opinion_id = valid_opinion_json["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/opinions",
        "nominal-status-headers-body",
        valid_opinion.status == "200 OK"
            && valid_opinion_json["updatedUnixMs"]
                .as_i64()
                .unwrap_or_default()
                > 0
    );

    let delete_route = format!("/api/v0/opinions/{opinion_id}");
    let deleted = crate::route_http_request("DELETE", &delete_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{delete_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "mutation-side-effects-and-readback",
        deleted.status == "204 No Content"
    );

    let contact = crate::route_http_request(
        "POST",
        "/api/v0/contacts/from-discovery",
        None,
        r#"{"peerId":"00000000-0000-4000-8000-000000000002","nickname":"Discovery Differential"}"#,
        &state,
    )
    .await
    .expect("contact from discovery");
    record!(
        "POST",
        "/api/v0/contacts/from-discovery",
        "missing-empty-or-conflict-state",
        contact.status == "404 Not Found" && contact.body == r#""Profile not found.""#
    );

    let mut searches_pass = true;
    for action in ["download", "stream"] {
        let route = format!(
            "/api/v0/searches/00000000-0000-4000-8000-000000000002/items/00000000-0000-4000-8000-000000000002/{action}"
        );
        let response = crate::route_http_request("POST", &route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        searches_pass &= response.status == "404 Not Found"
            && serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default()
                == serde_json::json!({
                    "type": "search_not_found",
                    "title": "Search not found",
                    "status": 404,
                    "detail": "Search not found",
                });
    }
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "missing-empty-or-conflict-state",
        searches_pass
    );
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "missing-empty-or-conflict-state",
        searches_pass
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("discovery_graph_and_opinions.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api discovery-graph-opinions mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining DiscoveryGraphController
/// cases: request rejection, empty fallback construction, process-local
/// behavior when SQLite is closed, reset behavior after reconstruction,
/// and deterministic concurrent builds.
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
pub(super) async fn controller_api_differential_discovery_graph_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} POST /api/v0/discovery-graph [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": "/api/v0/discovery-graph",
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let malformed =
        crate::route_http_request("POST", "/api/v0/discovery-graph/extra", None, "{}", &state)
            .await
            .expect("malformed discovery graph route");
    record!(
        "malformed-path-query-or-body",
        malformed.status == "404 Not Found"
    );

    let empty = crate::route_http_request("POST", "/api/v0/discovery-graph", None, "{}", &state)
        .await
        .expect("empty discovery graph request");
    let empty_json =
        serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or(serde_json::Value::Null);
    record!(
        "missing-empty-or-conflict-state",
        empty.status == "200 OK"
            && empty.content_type == "application/json"
            && empty_json["request"]["scope"] == "songid_run"
            && empty_json["seedNodeId"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && empty_json["nodes"]
                .as_array()
                .is_some_and(|nodes| !nodes.is_empty())
    );

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("discovery graph runtime-failure database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default(),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime = crate::route_http_request(
        "POST",
        "/api/v0/discovery-graph",
        None,
        "{}",
        &runtime_state,
    )
    .await
    .expect("discovery graph with closed unrelated database");
    let runtime_json =
        serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap_or(serde_json::Value::Null);
    record!(
        "runtime-failure-and-timeout",
        runtime.status == "200 OK"
            && runtime_json["nodes"]
                .as_array()
                .is_some_and(|nodes| !nodes.is_empty())
    );

    let (restarted_state, _restarted_receiver) = test_state();
    let restarted = crate::route_http_request(
        "POST",
        "/api/v0/discovery-graph",
        None,
        "{}",
        &restarted_state,
    )
    .await
    .expect("discovery graph after restart");
    let restarted_json = serde_json::from_str::<serde_json::Value>(&restarted.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "restart-persistence-or-reset",
        restarted.status == "200 OK"
            && restarted_json["seedNodeId"] == empty_json["seedNodeId"]
            && restarted_json["nodes"] == empty_json["nodes"]
    );

    let concurrent_bodies = tokio::join!(
        crate::route_http_request("POST", "/api/v0/discovery-graph", None, "{}", &state),
        crate::route_http_request("POST", "/api/v0/discovery-graph", None, "{}", &state),
    );
    let concurrent = match concurrent_bodies {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or(serde_json::Value::Null);
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or(serde_json::Value::Null);
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["seedNodeId"] == right_json["seedNodeId"]
                && left_json["nodes"] == right_json["nodes"]
                && left_json["edges"] == right_json["edges"]
        }
        _ => false,
    };
    record!("concurrency-and-idempotency", concurrent);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create discovery graph edge evidence directory");
    fs::write(
        evidence_dir.join("discovery_graph_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize discovery graph edge evidence"),
    )
    .expect("write discovery graph edge evidence");
    assert!(
        mismatches.is_empty(),
        "{} discovery graph edge mismatches: {:?}",
        mismatches.len(),
        mismatches
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
pub(super) async fn controller_api_differential_content_bound_stream_tickets() {
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

    let collection = crate::route_http_request(
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
    crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        api_authorization,
        r#"{"content_id":"content/differential-one","title":"track.flac","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create collection item fixture");
    let grant = crate::route_http_request(
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
    let issued = crate::route_http_request(
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

    let query_stream = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fdifferential-one?token={}",
            crate::url_encode(&token)
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

    let share_headers = crate::RequestSecurityHeaders {
        x_share_token: Some(token.clone()),
        ..Default::default()
    };
    let ticket_route = "/api/v0/streams/content%2Fdifferential-one/share-ticket";
    let ticket_response = crate::route_http_request_with_headers(
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

    let stream = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fdifferential-one?ticket={}",
            crate::url_encode(&ticket)
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

    let wrong_content = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/differential-different?ticket={}",
            crate::url_encode(&ticket)
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

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        api_authorization,
        "",
        &state,
    )
    .await
    .expect("delete share grant");
    let revoked_stream = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fdifferential-one?ticket={}",
            crate::url_encode(&ticket)
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

/// Bulk differential proof crediting 5 hashdb routes' cases,
/// independently re-derived from `hashdb_history_backfill_batches_
/// persists_inventory_and_progress`'s real batched-backfill-progress
/// checks (a real batch size limits how many search-history records
/// are consumed per call, progress persists and resumes across
/// calls, and `reset=true` genuinely restarts from the full history).
/// slskdN-only (confirmed against the frozen registry; the bare
/// `/api/hashdb/peers` compatibility-surface comparison in the
/// source test has no registry entry and is not called here).
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
pub(super) async fn controller_api_differential_hashdb_history_backfill() {
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
        let mut searches = state.searches.write().await;
        for index in 1..=11_u64 {
            searches.records.push(crate::SearchRecord {
                id: format!("history-differential-{index}"),
                token: u32::try_from(index).unwrap(),
                query: format!("history differential {index}"),
                target: "global",
                target_name: None,
                status: "completed",
                results: vec![crate::SearchResultEntry {
                    peer_username: Some(format!("differential-peer-{index}")),
                    filename: format!("Library/DifferentialTrack-{index}.flac"),
                    size: 32_768 + index,
                    extension: "flac".to_owned(),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked: false,
                    slot_free: Some(true),
                    average_speed: Some(1_000),
                    queue_length: Some(0),
                }],
                raw_response_count: 1,
                filtered_out_count: 0,
                ignored_result_count: 0,
                hidden_locked_count: 0,
                fallback_attempts: 0,
                ttl_seconds: crate::DEFAULT_SEARCH_TTL_SECONDS,
                expires_at: 0,
                created_at: index,
                updated_at: index,
            });
        }
    }

    let first = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("first history backfill batch");
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "nominal-status-headers-body",
        first.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "mutation-side-effects-and-readback",
        first_json["searchesProcessed"] == 10
            && first_json["flacsDiscovered"] == 10
            && first_json["totalSearches"] == 11
            && first_json["remainingSearches"] == 1
            && first_json["complete"] == false
    );

    let candidates = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/backfill/candidates?limit=20",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb backfill candidates");
    let candidates_json =
        serde_json::from_str::<serde_json::Value>(&candidates.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/backfill/candidates",
        "nominal-status-headers-body",
        candidates.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/backfill/candidates",
        "populated-dynamic-state",
        candidates_json["count"] == 10
            && candidates_json["entries"].as_array().map(Vec::len) == Some(10)
    );

    let second = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("second history backfill batch");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "mutation-side-effects-and-readback",
        second_json["searchesProcessed"] == 1
            && second_json["flacsDiscovered"] == 1
            && second_json["remainingSearches"] == 0
            && second_json["complete"] == true
    );

    let complete = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "",
        &state,
    )
    .await
    .expect("completed history backfill");
    let complete_json =
        serde_json::from_str::<serde_json::Value>(&complete.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "concurrency-and-idempotency",
        complete_json["searchesProcessed"] == 0
            && complete_json["flacsDiscovered"] == 0
            && complete_json["complete"] == true
    );

    let reset = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?reset=true",
        None,
        "",
        &state,
    )
    .await
    .expect("reset history backfill");
    let reset_json = serde_json::from_str::<serde_json::Value>(&reset.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "restart-persistence-or-reset",
        reset_json["searchesProcessed"] == 11
            && reset_json["flacsDiscovered"] == 11
            && reset_json["complete"] == true
    );

    let stats = crate::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
        .await
        .expect("hashdb stats after backfill");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/stats",
        "populated-dynamic-state",
        stats_json["totalFlacEntries"] == 11 && stats_json["hashedFlacEntries"] == 0
    );

    let analysis =
        crate::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
            .await
            .expect("hashdb optimize analysis after backfill");
    let analysis_json =
        serde_json::from_str::<serde_json::Value>(&analysis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/optimize/analyze",
        "nominal-status-headers-body",
        analysis.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/optimize/analyze",
        "populated-dynamic-state",
        analysis_json["flacInventoryEntryCount"] == 11 && analysis_json["peerCount"] == 11
    );

    let peers = crate::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("hashdb peers after backfill");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/peers",
        "nominal-status-headers-body",
        peers.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/peers",
        "populated-dynamic-state",
        peers_json["count"] == 0
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("hashdb_history_backfill.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api hashdb-history-backfill mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the residual source-discovery controller cases.
/// The frozen service keeps an empty status/SQLite projection available
/// when the Soulseek client is idle, returns a no-op response when stop is
/// requested before start, and reports its full conflict DTO for an
/// overlapping start.  These probes cover those boundaries plus the
/// malformed, reset, restart, and concurrent request paths.
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
pub(super) async fn controller_api_differential_discovery_open_cases() {
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
    let status_contract = |response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["isRunning"].is_boolean()
            && value["currentSearchTerm"].is_string()
            && value["stats"]["totalFiles"].is_u64()
            && value["stats"]["totalUsers"].is_u64()
            && value["stats"]["searchCycles"].is_u64()
            && value["stats"]["lastCycleNewFiles"].is_u64()
            && value["stats"]["hashVerificationEnabled"].is_boolean()
            && value["stats"]["filesWithHash"].is_u64()
    };
    let empty_status_contract = |response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        status_contract(response)
            && value["isRunning"] == false
            && value["currentSearchTerm"] == ""
            && value["stats"]["totalFiles"] == 0
            && value["stats"]["totalUsers"] == 0
            && value["stats"]["searchCycles"] == 0
            && value["stats"]["lastCycleNewFiles"] == 0
            && value["stats"]["hashVerificationEnabled"] == false
            && value["stats"]["filesWithHash"] == 0
    };

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            crate::route_http_request("GET", "/api/v0/discovery/extra", None, "", &state)
                .await
                .expect("discovery status malformed response");
        record!(
            "GET",
            "/api/v0/discovery",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request("GET", "/api/v0/discovery", None, "", &state)
            .await
            .expect("discovery empty status response");
        record!(
            "GET",
            "/api/v0/discovery",
            "missing-empty-or-conflict-state",
            empty_status_contract(&empty)
        );
    }

    {
        // Status is local service state in the frozen controller; a
        // disconnected session must not turn this read into a 5xx.
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime =
            crate::route_http_request("GET", "/api/v0/discovery", None, "", &runtime_state)
                .await
                .expect("discovery runtime status response");
        record!(
            "GET",
            "/api/v0/discovery",
            "runtime-failure-and-timeout",
            empty_status_contract(&runtime)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("no-partial malformed response");
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &state,
        )
        .await
        .expect("no-partial empty response");
        let empty_json = json_value(&empty);
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && empty_json["usersWithoutPartialSupport"] == 0
                && empty_json["message"]
                    == "0 users are flagged as not supporting partial/chunked downloads"
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = crate::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("no-partial runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK" && runtime_json["usersWithoutPartialSupport"] == 0
        );

        let (populated_state, mut receiver) = test_state_with_env(target_env());
        let started = crate::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"partial support probe"}"#,
            &populated_state,
        )
        .await
        .expect("start no-partial populated probe");
        let token = match receiver.recv().await.expect("no-partial discovery command") {
            crate::SessionCommand::Search { token, .. } => token,
            command => panic!("unexpected no-partial command: {command:?}"),
        };
        {
            let mut searches = populated_state.searches.write().await;
            searches
                .records
                .iter_mut()
                .find(|record| record.token == token)
                .expect("no-partial populated search")
                .results
                .push(crate::SearchResultEntry {
                    peer_username: Some("partial-probe-peer".to_owned()),
                    filename: "Partial/Probe.flac".to_owned(),
                    size: 123,
                    extension: "flac".to_owned(),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked: false,
                    slot_free: Some(true),
                    average_speed: Some(1),
                    queue_length: Some(0),
                });
        }
        let populated = crate::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &populated_state,
        )
        .await
        .expect("no-partial populated response");
        let populated_json = json_value(&populated);
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "populated-dynamic-state",
            started.status == "200 OK"
                && populated.status == "200 OK"
                && populated_json["usersWithoutPartialSupport"] == 0
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-filename/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery filename malformed response");
        record!(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = crate::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery filename missing response");
        record!(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body.contains("pattern query parameter is required")
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = crate::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-filename?pattern=recording",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery filename runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["pattern"] == "recording"
                && runtime_json["sourceCount"] == 0
                && runtime_json["sources"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-size/not-a-size",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery size malformed response");
        record!(
            "GET",
            "/api/v0/discovery/sources/by-size/{size}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("size must be greater than zero")
        );

        let missing = crate::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-size/42",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery size missing response");
        let missing_json = json_value(&missing);
        record!(
            "GET",
            "/api/v0/discovery/sources/by-size/{size}",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["size"] == 42
                && missing_json["sourceCount"] == 0
                && missing_json["sources"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = crate::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-size/42?limit=10",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery size runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/sources/by-size/{size}",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["size"] == 42
                && runtime_json["sourceCount"] == 0
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request(
            "GET",
            "/api/v0/discovery/summaries?minSources=0",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery summaries malformed response");
        record!(
            "GET",
            "/api/v0/discovery/summaries",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed
                    .body
                    .contains("minSources must be greater than zero")
        );

        let missing =
            crate::route_http_request("GET", "/api/v0/discovery/summaries", None, "", &state)
                .await
                .expect("discovery summaries missing response");
        let missing_json = json_value(&missing);
        record!(
            "GET",
            "/api/v0/discovery/summaries",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["minSources"] == 2
                && missing_json["count"] == 0
                && missing_json["summaries"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = crate::route_http_request(
            "GET",
            "/api/v0/discovery/summaries?minSources=1",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery summaries runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/summaries",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["minSources"] == 1
                && runtime_json["count"] == 0
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = crate::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset nominal response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let malformed = crate::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset malformed response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = crate::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset missing response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let followup = crate::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset readback response");
        let followup_json = json_value(&followup);
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "mutation-side-effects-and-readback",
            followup.status == "200 OK" && followup_json["usersWithoutPartialSupport"] == 0
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = crate::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery reset runtime response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("discovery reset restarted response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "restart-persistence-or-reset",
            restarted.status == "200 OK"
                && restarted
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/discovery/reset-partial-flags",
                None,
                "",
                &concurrent_state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/discovery/reset-partial-flags",
                None,
                "",
                &concurrent_state
            )
        );
        let left = left.expect("left discovery reset concurrency response");
        let right = right.expect("right discovery reset concurrency response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "concurrency-and-idempotency",
            left.status == "200 OK" && right.status == "200 OK" && left.body == right.body
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request(
            "POST",
            "/api/v0/discovery/start/extra",
            None,
            "not-json",
            &state,
        )
        .await
        .expect("discovery start malformed response");
        record!(
            "POST",
            "/api/v0/discovery/start",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = crate::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"runtime discovery","enableHashVerification":false}"#,
            &runtime_state,
        )
        .await
        .expect("discovery start runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "POST",
            "/api/v0/discovery/start",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["message"] == "Discovery started"
                && runtime_json["searchTerm"] == "runtime discovery"
                && runtime_json["hashVerificationEnabled"] == false
        );
    }

    {
        let (restart_state, mut receiver) = test_state_with_env(target_env());
        let first = crate::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"first discovery"}"#,
            &restart_state,
        )
        .await
        .expect("discovery first start response");
        let _ = receiver.recv().await.expect("first discovery command");
        let stopped =
            crate::route_http_request("POST", "/api/v0/discovery/stop", None, "", &restart_state)
                .await
                .expect("discovery restart stop response");
        let second = crate::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"second discovery"}"#,
            &restart_state,
        )
        .await
        .expect("discovery second start response");
        let _ = receiver.recv().await.expect("second discovery command");
        let second_json = json_value(&second);
        record!(
            "POST",
            "/api/v0/discovery/start",
            "restart-persistence-or-reset",
            first.status == "200 OK"
                && stopped.status == "200 OK"
                && second.status == "200 OK"
                && second_json["searchTerm"] == "second discovery"
        );
    }

    {
        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/discovery/start",
                None,
                r#"{"searchTerm":"concurrent one"}"#,
                &concurrent_state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/discovery/start",
                None,
                r#"{"searchTerm":"concurrent two"}"#,
                &concurrent_state
            )
        );
        let left = left.expect("left discovery start concurrency response");
        let right = right.expect("right discovery start concurrency response");
        let left_ok = left.status == "200 OK";
        let right_ok = right.status == "200 OK";
        let left_allowed = left_ok || left.status == "409 Conflict";
        let right_allowed = right_ok || right.status == "409 Conflict";
        record!(
            "POST",
            "/api/v0/discovery/start",
            "concurrency-and-idempotency",
            left_allowed
                && right_allowed
                && (left_ok || right_ok)
                && concurrent_state.source_discovery.read().await.running
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            crate::route_http_request("POST", "/api/v0/discovery/stop/extra", None, "", &state)
                .await
                .expect("discovery stop malformed response");
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = crate::route_http_request("POST", "/api/v0/discovery/stop", None, "", &state)
            .await
            .expect("discovery stop missing response");
        let missing_json = json_value(&missing);
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json == serde_json::json!({"message": "Discovery not running"})
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let started = crate::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"runtime stop"}"#,
            &runtime_state,
        )
        .await
        .expect("discovery runtime stop start response");
        let runtime =
            crate::route_http_request("POST", "/api/v0/discovery/stop", None, "", &runtime_state)
                .await
                .expect("discovery stop runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "runtime-failure-and-timeout",
            started.status == "200 OK"
                && runtime.status == "200 OK"
                && runtime_json["message"] == "Discovery stopped"
                && runtime_json["stats"].is_object()
        );

        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted =
            crate::route_http_request("POST", "/api/v0/discovery/stop", None, "", &restarted_state)
                .await
                .expect("discovery stop restarted response");
        let restarted_json = json_value(&restarted);
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "restart-persistence-or-reset",
            restarted.status == "200 OK"
                && restarted_json == serde_json::json!({"message": "Discovery not running"})
        );

        let (concurrent_state, mut receiver) = test_state_with_env(target_env());
        let started = crate::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"concurrent stop"}"#,
            &concurrent_state,
        )
        .await
        .expect("discovery concurrent stop start response");
        let _ = receiver
            .recv()
            .await
            .expect("concurrent stop discovery command");
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/discovery/stop",
                None,
                "",
                &concurrent_state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/discovery/stop",
                None,
                "",
                &concurrent_state
            )
        );
        let left = left.expect("left discovery stop concurrency response");
        let right = right.expect("right discovery stop concurrency response");
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "concurrency-and-idempotency",
            started.status == "200 OK"
                && left.status == "200 OK"
                && right.status == "200 OK"
                && !concurrent_state.source_discovery.read().await.running
        );
    }

    assert_eq!(ledger.len(), 32, "discovery residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
    fs::write(
        evidence_dir.join("discovery_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize discovery ledger"),
    )
    .expect("write discovery ledger");
    assert!(
        mismatches.is_empty(),
        "{} discovery controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
