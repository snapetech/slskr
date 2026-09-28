//! Controller full media differential ownership.

use super::*;

/// Bulk differential proof crediting 14 mediacore routes' cases,
/// independently re-derived from `mediacore_mutations_match_native_
/// validation_and_result_dtos`'s real fuzzy-match/perceptual-hash/
/// portability/retrieval/stats contract checks. slskdN-only (confirmed
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
pub(super) async fn controller_api_differential_mediacore_mutations() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} POST {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    macro_rules! record_get {
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

    let fuzzy_text = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/text",
        None,
        r#"{"textA":"same","textB":"same"}"#,
        &state,
    )
    .await
    .expect("fuzzy text match");
    let fuzzy_text_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_text.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/fuzzymatch/text",
        "nominal-status-headers-body",
        fuzzy_text.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/fuzzymatch/text",
        "mutation-side-effects-and-readback",
        fuzzy_text_json["levenshteinSimilarity"] == 1.0
            && fuzzy_text_json["phoneticSimilarity"] == 1.0
            && fuzzy_text_json["combinedSimilarity"] == 1.0
    );

    let fuzzy_perceptual = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/perceptual",
        None,
        r#"{"contentIdA":"content:music:recording:a","contentIdB":"content:music:recording:b"}"#,
        &state,
    )
    .await
    .expect("fuzzy perceptual match");
    let fuzzy_perceptual_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_perceptual.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/fuzzymatch/perceptual",
        "nominal-status-headers-body",
        fuzzy_perceptual.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/fuzzymatch/perceptual",
        "mutation-side-effects-and-readback",
        fuzzy_perceptual_json["similarity"] == 0.0
            && fuzzy_perceptual_json["isSimilar"] == false
            && fuzzy_perceptual_json["threshold"] == 0.7
    );

    for (route, body) in [
        (
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[0.5],"sampleRate":1,"algorithm":"PHash"}"#,
        ),
        (
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":1,"height":1,"algorithm":"PHash"}"#,
        ),
    ] {
        let response = crate::route_http_request("POST", route, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            route,
            "mutation-side-effects-and-readback",
            value["algorithm"] == "PHash"
                && value["hex"] == "0000000000000000"
                && value["numericHash"] == 0
        );
    }

    let similarity = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/perceptualhash/similarity",
        None,
        r#"{"hashA":"0000000000000000","hashB":"ffffffffffffffff","threshold":0.8}"#,
        &state,
    )
    .await
    .expect("perceptual hash similarity");
    let similarity_json =
        serde_json::from_str::<serde_json::Value>(&similarity.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/perceptualhash/similarity",
        "nominal-status-headers-body",
        similarity.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/perceptualhash/similarity",
        "mutation-side-effects-and-readback",
        similarity_json["hammingDistance"] == 64
            && similarity_json["similarity"] == 0.0
            && similarity_json["areSimilar"] == false
    );

    for (route, body) in [
        (
            "/api/v0/mediacore/ipld/links/content:test:type:id",
            r#"{"links":[]}"#,
        ),
        (
            "/api/v0/mediacore/portability/export",
            r#"{"contentIds":[]}"#,
        ),
        (
            "/api/v0/mediacore/publish/republish",
            r#"{"contentIds":[]}"#,
        ),
        (
            "/api/v0/mediacore/publish/descriptor",
            r#"{"descriptor":{"contentId":"content:test:type:id"}}"#,
        ),
        (
            "/api/v0/mediacore/fuzzymatch/find/content:music:recording:missing",
            r#"{"minConfidence":2}"#,
        ),
        ("/api/v0/mediacore/fuzzymatch/text", r#"{"textA":"same"}"#),
        (
            "/api/v0/mediacore/fuzzymatch/perceptual",
            r#"{"contentIdA":"content:music:recording:a"}"#,
        ),
        (
            "/api/v0/mediacore/perceptualhash/similarity",
            r#"{"hashA":"not-hex","hashB":"0000000000000000"}"#,
        ),
        ("/api/v0/mediacore/perceptualhash/audio", r#"{}"#),
        (
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":0,"height":1}"#,
        ),
        ("/api/v0/mediacore/portability/analyze", r#"{}"#),
        ("/api/v0/mediacore/portability/import", r#"{}"#),
        ("/api/v0/mediacore/retrieve/verify", r#"{}"#),
    ] {
        let response = crate::route_http_request("POST", route, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let evidence_route = if route == "/api/v0/mediacore/ipld/links/content:test:type:id" {
            "/api/v0/mediacore/ipld/links/{*contentId}"
        } else if route.starts_with("/api/v0/mediacore/fuzzymatch/find/") {
            "/api/v0/mediacore/fuzzymatch/find/{*contentId}"
        } else {
            route
        };
        record!(
            evidence_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    let empty_package = r#"{"package":{"version":"1.0","exportedAt":"2026-01-01T00:00:00Z","source":"test","entries":[],"links":[],"metadata":{"totalEntries":0,"totalLinks":0,"entriesByDomain":{},"checksum":""}}}"#;
    let analysis = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/analyze",
        None,
        empty_package,
        &state,
    )
    .await
    .expect("portability analyze");
    record!(
        "/api/v0/mediacore/portability/analyze",
        "nominal-status-headers-body",
        analysis.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/portability/analyze",
        "mutation-side-effects-and-readback",
        analysis.body
            == r#"{"cleanEntries":0,"conflictingEntries":0,"conflicts":[],"recommendedStrategies":{"Merge":0,"Overwrite":0,"Skip":0},"totalEntries":0}"#
    );

    let imported = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/import",
        None,
        empty_package,
        &state,
    )
    .await
    .expect("portability import");
    let imported_json =
        serde_json::from_str::<serde_json::Value>(&imported.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/portability/import",
        "nominal-status-headers-body",
        imported.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/portability/import",
        "mutation-side-effects-and-readback",
        [
            "success",
            "entriesProcessed",
            "entriesImported",
            "entriesSkipped",
            "conflictsResolved",
            "conflicts",
            "errors",
            "duration",
        ]
        .iter()
        .all(|key| imported_json.get(*key).is_some())
    );

    let fuzzy_stats =
        crate::route_http_request("GET", "/api/v0/mediacore/stats/fuzzy", None, "", &state)
            .await
            .expect("fuzzy stats");
    let fuzzy_stats_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_stats.body).unwrap_or_default();
    record_get!(
        "/api/v0/mediacore/stats/fuzzy",
        "populated-dynamic-state",
        fuzzy_stats_json["totalMatches"] == 0
            && fuzzy_stats_json["successfulMatches"] == 0
            && fuzzy_stats_json["successRate"] == 0.0
    );

    let perceptual_stats = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/perceptual",
        None,
        "",
        &state,
    )
    .await
    .expect("perceptual stats");
    let perceptual_stats_json =
        serde_json::from_str::<serde_json::Value>(&perceptual_stats.body).unwrap_or_default();
    record_get!(
        "/api/v0/mediacore/stats/perceptual",
        "populated-dynamic-state",
        perceptual_stats_json["totalHashesComputed"] == 0
            && perceptual_stats_json["duplicateHashesDetected"] == 0
    );

    let portability_stats = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/portability",
        None,
        "",
        &state,
    )
    .await
    .expect("portability stats");
    let portability_stats_json =
        serde_json::from_str::<serde_json::Value>(&portability_stats.body).unwrap_or_default();
    record_get!(
        "/api/v0/mediacore/stats/portability",
        "populated-dynamic-state",
        portability_stats_json["totalExports"] == 0
            && portability_stats_json["totalImports"] == 0
            && portability_stats_json["successfulImports"] == 0
            && portability_stats_json["importSuccessRate"] == 0.0
    );

    let verify = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/verify",
        None,
        r#"{"descriptor":{"contentId":"content:test:type:id","hashes":[]}}"#,
        &state,
    )
    .await
    .expect("retrieve verify");
    let verify_json = serde_json::from_str::<serde_json::Value>(&verify.body).unwrap_or_default();
    record!(
        "/api/v0/mediacore/retrieve/verify",
        "nominal-status-headers-body",
        verify.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/retrieve/verify",
        "mutation-side-effects-and-readback",
        verify_json["isValid"] == false
            && verify_json["signatureValid"] == false
            && verify_json["freshnessValid"] == false
            && verify_json["validationError"].as_str().is_some()
    );

    let cache = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "",
        &state,
    )
    .await
    .expect("retrieve cache clear");
    record!(
        "/api/v0/mediacore/retrieve/cache/clear",
        "nominal-status-headers-body",
        cache.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/retrieve/cache/clear",
        "mutation-side-effects-and-readback",
        cache.body == r#"{"bytesFreed":0,"entriesCleared":0,"success":true}"#
    );

    let reset =
        crate::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .expect("stats reset");
    record!(
        "/api/v0/mediacore/stats/reset",
        "nominal-status-headers-body",
        reset.status == "200 OK"
    );
    record!(
        "/api/v0/mediacore/stats/reset",
        "mutation-side-effects-and-readback",
        reset.body == r#"{"message":"Statistics reset successfully"}"#
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mediacore mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting the remaining deterministic MediaCore
/// request-validation and static-list projections. The frozen controllers
/// reject missing request fields with 400 and return non-empty supported
/// algorithm/strategy lists. slskdN-only (confirmed against the frozen
/// registry).
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
pub(super) async fn controller_api_differential_mediacore_validation_tail() {
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
    let invalid_requests = [
        (
            "POST",
            "/api/v0/mediacore/contentid/register",
            "/api/v0/mediacore/contentid/register",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/ipld/links/content:audio:track:validation",
            "/api/v0/mediacore/ipld/links/{*contentId}",
            r#"{"links":[]}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/perceptual",
            "/api/v0/mediacore/fuzzymatch/perceptual",
            r#"{"contentIdA":"","contentIdB":"content:audio:track:b"}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/text",
            "/api/v0/mediacore/fuzzymatch/text",
            r#"{"textA":"","textB":"same"}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/audio",
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[],"sampleRate":1}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/image",
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"","width":1,"height":1}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/similarity",
            "/api/v0/mediacore/perceptualhash/similarity",
            r#"{"hashA":"","hashB":"0000000000000000"}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/export",
            "/api/v0/mediacore/portability/export",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/analyze",
            "/api/v0/mediacore/portability/analyze",
            r#"{"package":null}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/import",
            "/api/v0/mediacore/portability/import",
            r#"{"package":null}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/descriptor",
            "/api/v0/mediacore/publish/descriptor",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/batch",
            "/api/v0/mediacore/publish/batch",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/republish",
            "/api/v0/mediacore/publish/republish",
            r#"{"contentIds":[]}"#,
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/batch",
            "/api/v0/mediacore/retrieve/batch",
            "{}",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/verify",
            "/api/v0/mediacore/retrieve/verify",
            r#"{"descriptor":null}"#,
        ),
    ];

    for (method, path, route, body) in invalid_requests {
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }

    let fuzzy_find = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/find/content:audio:track:empty-request",
        None,
        "{}",
        &state,
    )
    .await
    .expect("fuzzy find empty request");
    let fuzzy_find_json =
        serde_json::from_str::<serde_json::Value>(&fuzzy_find.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/fuzzymatch/find/{*contentId}",
        "missing-empty-or-conflict-state",
        fuzzy_find.status == "200 OK"
            && fuzzy_find_json["targetContentId"] == "content:audio:track:empty-request"
            && fuzzy_find_json["searchParameters"]["minConfidence"] == 0.7
    );

    let cache_clear_malformed = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("cache clear malformed body");
    let cache_clear_malformed_json =
        serde_json::from_str::<serde_json::Value>(&cache_clear_malformed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        "malformed-path-query-or-body",
        cache_clear_malformed.status == "200 OK" && cache_clear_malformed_json["success"] == true
    );

    let cache_clear_empty = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "",
        &state,
    )
    .await
    .expect("cache clear empty request");
    let cache_clear_empty_json =
        serde_json::from_str::<serde_json::Value>(&cache_clear_empty.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        "missing-empty-or-conflict-state",
        cache_clear_empty.status == "200 OK" && cache_clear_empty_json["success"] == true
    );

    let stats_reset_malformed = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/stats/reset",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("stats reset malformed body");
    record!(
        "POST",
        "/api/v0/mediacore/stats/reset",
        "malformed-path-query-or-body",
        stats_reset_malformed.status == "200 OK"
            && stats_reset_malformed
                .body
                .contains("Statistics reset successfully")
    );

    let stats_reset_empty =
        crate::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .expect("stats reset empty request");
    record!(
        "POST",
        "/api/v0/mediacore/stats/reset",
        "missing-empty-or-conflict-state",
        stats_reset_empty.status == "200 OK"
            && stats_reset_empty
                .body
                .contains("Statistics reset successfully")
    );

    let batch_malformed = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/batch",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("publish batch malformed body");
    record!(
        "POST",
        "/api/v0/mediacore/publish/batch",
        "malformed-path-query-or-body",
        batch_malformed.status == "400 Bad Request"
    );

    let retrieve_batch_malformed = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("retrieve batch malformed body");
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        "malformed-path-query-or-body",
        retrieve_batch_malformed.status == "400 Bad Request"
    );

    let update_malformed = crate::route_http_request(
        "PUT",
        "/api/v0/mediacore/publish/descriptor/content:audio:track:missing-updates",
        None,
        "{}",
        &state,
    )
    .await
    .expect("descriptor update malformed body");
    record!(
        "PUT",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "malformed-path-query-or-body",
        update_malformed.status == "400 Bad Request"
    );

    let delete_empty_path = crate::route_http_request(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/",
        None,
        "",
        &state,
    )
    .await
    .expect("descriptor delete empty path");
    record!(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "malformed-path-query-or-body",
        delete_empty_path.status == "400 Bad Request"
    );

    let static_lists = [
        (
            "/api/v0/mediacore/perceptualhash/algorithms",
            "/api/v0/mediacore/perceptualhash/algorithms",
            "algorithms",
        ),
        (
            "/api/v0/mediacore/portability/merge-strategies",
            "/api/v0/mediacore/portability/merge-strategies",
            "strategies",
        ),
        (
            "/api/v0/mediacore/portability/strategies",
            "/api/v0/mediacore/portability/strategies",
            "strategies",
        ),
    ];
    for (path, route, key) in static_lists {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && value[key]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_validation_tail.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api MediaCore validation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
/// Bulk differential proof crediting the mediacore descriptor/retrieve/
/// stats lifecycle routes' cases, independently re-derived from
/// `mediacore_descriptor_updates_imports_and_stats_use_real_records`
/// and `mediacore_versioned_descriptor_delete_does_not_overflow_
/// worker_stack`'s real published/cached/statistics record checks.
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
pub(super) async fn controller_api_differential_mediacore_descriptor_lifecycle() {
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
    let content_id = "content:test:recording:differential";
    let descriptor = serde_json::json!({
        "contentId": content_id,
        "title": "before",
        "hashes": [{"algorithm": "sha256", "hex": "aaaa"}],
        "signature": {
            "publicKey": "key",
            "signature": "signature",
            "timestampUnixMs": crate::unix_timestamp_millis(),
        },
    });
    let published = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        None,
        &serde_json::json!({"descriptor": descriptor}).to_string(),
        &state,
    )
    .await
    .expect("publish descriptor");
    record!(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        "nominal-status-headers-body",
        published.status == "200 OK"
    );

    let updated = crate::route_http_request(
        "PUT",
        &format!("/api/v0/mediacore/publish/descriptor/{content_id}"),
        None,
        r#"{"updates":{"title":"after","genre":"ambient"}}"#,
        &state,
    )
    .await
    .expect("update descriptor");
    record!(
        "PUT",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "missing-empty-or-conflict-state",
        updated.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&updated.body).unwrap()["error"]
                == "Failed to update descriptor"
    );
    record!(
        "PUT",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "nominal-status-headers-body",
        updated.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&updated.body).unwrap()["error"]
                == "Failed to update descriptor"
    );
    let update_readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/mediacore/retrieve/descriptor/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read descriptor after failed update");
    record!(
        "PUT",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "mutation-side-effects-and-readback",
        update_readback.status == "200 OK"
            && update_readback.body.contains("\"title\":\"before\"")
            && !update_readback.body.contains("\"title\":\"after\"")
    );

    let retrieved = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        None,
        &serde_json::json!({"contentIds":[content_id,"content:test:recording:missing"]})
            .to_string(),
        &state,
    )
    .await
    .expect("retrieve batch");
    let retrieved_json =
        serde_json::from_str::<serde_json::Value>(&retrieved.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        "nominal-status-headers-body",
        retrieved.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/mediacore/retrieve/batch",
        "mutation-side-effects-and-readback",
        retrieved_json["requested"] == 2
            && retrieved_json["found"] == 1
            && retrieved_json["results"].as_array().map(Vec::len) == Some(2)
    );

    let stats =
        crate::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
            .await
            .expect("retrieve stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/retrieve/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/retrieve/stats",
        "populated-dynamic-state",
        stats_json["totalRetrievals"] == 3
            && stats_json["cacheHits"] == 1
            && stats_json["cacheMisses"] == 2
            && stats_json["activeCacheEntries"] == 1
    );

    let cached_route = format!("/api/v0/mediacore/retrieve/descriptor/{content_id}");
    let cached = crate::route_http_request("GET", &cached_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{cached_route}: {error}"));
    let cached_json = serde_json::from_str::<serde_json::Value>(&cached.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/retrieve/descriptor/{*contentId}",
        "nominal-status-headers-body",
        cached.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/retrieve/descriptor/{*contentId}",
        "populated-dynamic-state",
        cached_json["found"] == true && cached_json["fromCache"] == true
    );

    let query = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/retrieve/query/domain/test",
        None,
        "",
        &state,
    )
    .await
    .expect("query descriptors by domain");
    let query_json = serde_json::from_str::<serde_json::Value>(&query.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/retrieve/query/domain/{domain}",
        "nominal-status-headers-body",
        query.status == "200 OK"
            && query_json["domain"] == "test"
            && query_json["descriptors"].is_array()
    );
    record!(
        "GET",
        "/api/v0/mediacore/retrieve/query/domain/{domain}",
        "populated-dynamic-state",
        query_json["totalFound"] == 1
            && query_json["descriptors"]
                .as_array()
                .is_some_and(|descriptors| {
                    descriptors
                        .iter()
                        .any(|descriptor| descriptor["contentId"] == content_id)
                })
    );

    let descriptor_stats = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        None,
        "",
        &state,
    )
    .await
    .expect("descriptor stats");
    let descriptor_stats_json =
        serde_json::from_str::<serde_json::Value>(&descriptor_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        "populated-dynamic-state",
        descriptor_stats_json["totalRetrievals"] == 4
            && descriptor_stats_json["cacheHits"] == 2
            && descriptor_stats_json["cacheMisses"] == 2
            && descriptor_stats_json["activeCacheEntries"] == 1
    );

    let publishing = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/publishing",
        None,
        "",
        &state,
    )
    .await
    .expect("publishing stats");
    let publishing_json =
        serde_json::from_str::<serde_json::Value>(&publishing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/stats/publishing",
        "nominal-status-headers-body",
        publishing.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/stats/publishing",
        "populated-dynamic-state",
        publishing_json["totalPublished"] == 1
            && publishing_json["activePublications"] == 1
            && publishing_json["publicationsByDomain"]["test"] == 1
    );

    let publisher_stats =
        crate::route_http_request("GET", "/api/v0/mediacore/publish/stats", None, "", &state)
            .await
            .expect("publisher stats");
    let publisher_stats_json =
        serde_json::from_str::<serde_json::Value>(&publisher_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/publish/stats",
        "nominal-status-headers-body",
        publisher_stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/publish/stats",
        "populated-dynamic-state",
        publisher_stats_json["totalPublishedDescriptors"] == 1
            && publisher_stats_json["activePublications"] == 1
            && publisher_stats_json["publicationsByDomain"]["test"] == 1
            && publisher_stats_json["averageTtlHours"]
                .as_f64()
                .unwrap_or(0.0)
                > 0.0
    );

    let dashboard =
        crate::route_http_request("GET", "/api/v0/mediacore/stats/dashboard", None, "", &state)
            .await
            .expect("mediacore stats dashboard");
    let dashboard_json =
        serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/stats/dashboard",
        "populated-dynamic-state",
        dashboard_json["contentPublishing"]["totalPublished"] == 1
            && dashboard_json["contentPublishing"]["activePublications"] == 1
            && dashboard_json["descriptors"]["totalRetrievals"] == 4
            && dashboard_json["descriptors"]["cacheHits"] == 2
            && dashboard_json["descriptors"]["activeCacheEntries"] == 1
    );

    let batch_content_id = "content:test:recording:batch-differential";
    let batch_descriptor = serde_json::json!({
        "contentId": batch_content_id,
        "title": "batch",
        "hashes": [{"algorithm": "sha256", "hex": "bbbb"}],
        "signature": {
            "publicKey": "key",
            "signature": "signature",
            "timestampUnixMs": crate::unix_timestamp_millis(),
        },
    });
    let batch_published = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/batch",
        None,
        &serde_json::json!({"descriptors": [batch_descriptor]}).to_string(),
        &state,
    )
    .await
    .expect("publish descriptor batch");
    let batch_published_json =
        serde_json::from_str::<serde_json::Value>(&batch_published.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/publish/batch",
        "nominal-status-headers-body",
        batch_published.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/mediacore/publish/batch",
        "mutation-side-effects-and-readback",
        batch_published_json["totalRequested"] == 1
            && batch_published_json["successfullyPublished"] == 1
            && batch_published_json["results"]
                .as_array()
                .is_some_and(|results| results.len() == 1)
    );

    let republished = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/republish",
        None,
        &serde_json::json!({"contentIds": [content_id]}).to_string(),
        &state,
    )
    .await
    .expect("republish descriptor");
    let republished_json =
        serde_json::from_str::<serde_json::Value>(&republished.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/publish/republish",
        "nominal-status-headers-body",
        republished.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/mediacore/publish/republish",
        "mutation-side-effects-and-readback",
        republished_json["totalChecked"] == 1
            && republished_json["republished"] == 0
            && republished_json["stillValid"] == 1
    );

    let exported = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/export",
        None,
        &serde_json::json!({"contentIds": [content_id], "includeLinks": true}).to_string(),
        &state,
    )
    .await
    .expect("export descriptor metadata");
    let exported_json =
        serde_json::from_str::<serde_json::Value>(&exported.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/portability/export",
        "nominal-status-headers-body",
        exported.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/mediacore/portability/export",
        "mutation-side-effects-and-readback",
        exported_json["entries"]
            .as_array()
            .is_some_and(|entries| entries.len() == 1)
            && exported_json["metadata"]["totalEntries"] == 1
    );

    let reset =
        crate::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .expect("stats reset");
    let stats_after_reset =
        crate::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
            .await
            .expect("retrieve stats after reset");
    record!(
        "GET",
        "/api/v0/mediacore/retrieve/stats",
        "mutation-side-effects-and-readback",
        reset.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&stats_after_reset.body).unwrap()
                ["totalRetrievals"]
                == 0
    );

    let delete_route = "/api/v0/mediacore/publish/descriptor/content-id-differential";
    let deleted = crate::route_http_request("DELETE", delete_route, None, "", &state)
        .await
        .expect("delete descriptor");
    record!(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "nominal-status-headers-body",
        deleted.status == "200 OK"
    );
    record!(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/{*contentId}",
        "mutation-side-effects-and-readback",
        serde_json::from_str::<serde_json::Value>(&deleted.body).unwrap()
            == serde_json::json!({
                "contentId": "content-id-differential",
                "success": true,
                "wasPublished": false,
            })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_descriptor_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mediacore-descriptor-lifecycle mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the mediacore content-id/fuzzy-
/// match-find/IPLD-link routes' cases, independently re-derived from
/// `mediacore_fuzzy_matching_and_ipld_validation_use_persisted_state`'s
/// real registered-content-id and persisted-perceptual-hash link-graph
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
pub(super) async fn controller_api_differential_mediacore_ipld_and_fuzzy_find() {
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
    let target_content = "content:audio:track:differential-target";
    let candidate_content = "content:audio:track:differential-candidate";
    let mut register_pass = true;
    let mut publish_pass = true;
    for (external_id, content_id) in [
        ("differential-target", target_content),
        ("differential-candidate", candidate_content),
    ] {
        let registered = crate::route_http_request(
            "POST",
            "/api/v0/mediacore/contentid/register",
            None,
            &serde_json::json!({"externalId": external_id, "contentId": content_id}).to_string(),
            &state,
        )
        .await
        .expect("register content id");
        register_pass &= registered.status == "200 OK";

        let published = crate::route_http_request(
            "POST",
            "/api/v0/mediacore/publish/descriptor",
            None,
            &serde_json::json!({
                "descriptor": {
                    "contentId": content_id,
                    "hashes": [{"algorithm": "sha256", "hex": external_id}],
                    "perceptualHashes": [{
                        "algorithm": "Chromaprint",
                        "hex": "0000000000001234",
                        "numericHash": 0x1234_u64,
                    }],
                    "signature": {
                        "publicKey": "key",
                        "signature": "signature",
                        "timestampUnixMs": crate::unix_timestamp_millis(),
                    },
                },
            })
            .to_string(),
            &state,
        )
        .await
        .expect("publish descriptor");
        publish_pass &= published.status == "200 OK";
    }
    record!(
        "POST",
        "/api/v0/mediacore/contentid/register",
        "nominal-status-headers-body",
        register_pass
    );
    record!(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        "mutation-side-effects-and-readback",
        publish_pass
    );

    let registry_stats =
        crate::route_http_request("GET", "/api/v0/mediacore/stats/registry", None, "", &state)
            .await
            .expect("content registry stats");
    let registry_stats_json =
        serde_json::from_str::<serde_json::Value>(&registry_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/stats/registry",
        "populated-dynamic-state",
        registry_stats_json["totalMappings"] == 2
            && registry_stats_json["totalDomains"] == 1
            && registry_stats_json["mappingsByDomain"]["audio"] == 2
            && registry_stats_json["mappingsByType"]["track"] == 2
    );

    let matches = crate::route_http_request(
        "POST",
        &format!("/api/v0/mediacore/fuzzymatch/find/{target_content}"),
        None,
        r#"{"minConfidence":0.7,"maxCandidates":50,"maxResults":10}"#,
        &state,
    )
    .await
    .expect("fuzzy match find");
    let matches_json = serde_json::from_str::<serde_json::Value>(&matches.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mediacore/fuzzymatch/find/{*contentId}",
        "nominal-status-headers-body",
        matches.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/mediacore/fuzzymatch/find/{*contentId}",
        "mutation-side-effects-and-readback",
        matches_json["totalCandidates"] == 1
            && matches_json["matches"][0]["candidateContentId"] == candidate_content
            && matches_json["matches"][0]["reason"] == "PerceptualHash"
    );

    let links_route = format!("/api/v0/mediacore/ipld/links/{target_content}");
    let links = crate::route_http_request(
        "POST",
        &links_route,
        None,
        &serde_json::json!({"links": [
            {"name": "same", "target": candidate_content},
            {"name": "broken", "target": "content:audio:track:differential-missing"},
        ]})
        .to_string(),
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{links_route}: {error}"));
    record!(
        "POST",
        "/api/v0/mediacore/ipld/links/{*contentId}",
        "nominal-status-headers-body",
        links.status == "200 OK"
    );

    let orphan = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/ipld/links/content:audio:track:differential-orphan",
        None,
        &serde_json::json!({"links": [{"name": "same", "target": candidate_content}]}).to_string(),
        &state,
    )
    .await
    .expect("register orphan link");
    record!(
        "POST",
        "/api/v0/mediacore/ipld/links/{*contentId}",
        "mutation-side-effects-and-readback",
        orphan.status == "200 OK"
    );

    let ipld_stats =
        crate::route_http_request("GET", "/api/v0/mediacore/stats/ipld", None, "", &state)
            .await
            .expect("ipld stats");
    let ipld_stats_json =
        serde_json::from_str::<serde_json::Value>(&ipld_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/stats/ipld",
        "populated-dynamic-state",
        ipld_stats_json["totalLinks"] == 2
            && ipld_stats_json["totalNodes"] == 3
            && ipld_stats_json["totalGraphs"] == 2
            && ipld_stats_json["brokenLinksDetected"] == 1
            && ipld_stats_json["orphanedNodes"] == 0
            && ipld_stats_json["graphConnectivityRatio"] == 1.0
    );

    let validation =
        crate::route_http_request("GET", "/api/v0/mediacore/ipld/validate", None, "", &state)
            .await
            .expect("ipld validate");
    let validation_json =
        serde_json::from_str::<serde_json::Value>(&validation.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/ipld/validate",
        "nominal-status-headers-body",
        validation.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/ipld/validate",
        "populated-dynamic-state",
        validation_json["isValid"] == false
            && validation_json["totalLinksValidated"] == 2
            && validation_json["brokenLinks"].as_array().map(Vec::len) == Some(1)
            && validation_json["orphanedLinks"].as_array().map(Vec::len) == Some(1)
    );

    let inbound_route = format!("/api/v0/mediacore/ipld/inbound/{candidate_content}");
    let inbound = crate::route_http_request("GET", &inbound_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{inbound_route}: {error}"));
    let inbound_json = serde_json::from_str::<serde_json::Value>(&inbound.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
        "nominal-status-headers-body",
        inbound.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
        "populated-dynamic-state",
        inbound_json["inboundLinks"].as_array().map(Vec::len) == Some(2)
            && inbound_json["inboundLinks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|source| source == target_content)
    );

    let graph_route = format!("/api/v0/mediacore/ipld/graph/{target_content}");
    let graph = crate::route_http_request("GET", &graph_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{graph_route}: {error}"));
    let graph_json = serde_json::from_str::<serde_json::Value>(&graph.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/ipld/graph/{*contentId}",
        "nominal-status-headers-body",
        graph.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/ipld/graph/{*contentId}",
        "populated-dynamic-state",
        graph_json["rootContentId"] == target_content
            && graph_json["nodes"].as_array().is_some_and(|nodes| {
                nodes.iter().any(|node| node["contentId"] == target_content)
                    && nodes
                        .iter()
                        .any(|node| node["contentId"] == candidate_content)
                    && nodes
                        .iter()
                        .any(|node| node["contentId"] == "content:audio:track:differential-missing")
                    && nodes.iter().any(|node| {
                        node["contentId"] == target_content
                            && node["outgoingLinks"].as_array().map(Vec::len) == Some(2)
                    })
            })
            && graph_json["paths"].as_array().is_some_and(|paths| {
                paths.iter().any(|path| {
                    path["contentIds"] == serde_json::json!([target_content, candidate_content])
                })
            })
    );

    let traverse_route =
        format!("/api/v0/mediacore/ipld/traverse/{target_content}?linkName=same&maxDepth=3");
    let traverse = crate::route_http_request("GET", &traverse_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{traverse_route}: {error}"));
    let traverse_json =
        serde_json::from_str::<serde_json::Value>(&traverse.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/ipld/traverse/{*startContentId}",
        "nominal-status-headers-body",
        traverse.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/ipld/traverse/{*startContentId}",
        "populated-dynamic-state",
        traverse_json["startContentId"] == target_content
            && traverse_json["linkName"] == "same"
            && traverse_json["completedTraversal"] == true
            && traverse_json["visitedNodes"]
                .as_array()
                .is_some_and(|nodes| {
                    nodes.iter().any(|node| node["contentId"] == target_content)
                        && nodes
                            .iter()
                            .any(|node| node["contentId"] == candidate_content)
                })
            && traverse_json["paths"].as_array().is_some_and(|paths| {
                paths.iter().any(|path| {
                    path["contentIds"] == serde_json::json!([target_content, candidate_content])
                })
            })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_ipld_and_fuzzy_find.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mediacore-ipld-fuzzy-find mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting MediaCore statistics routes' tolerance of
/// an unknown query parameter. These actions bind no query parameters in
/// the frozen controllers, so the malformed projection preserves the
/// nominal 200 response and body shape. slskdN-only (confirmed against the
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
pub(super) async fn controller_api_differential_mediacore_stats_malformed_queries() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    let (state, _receiver) = test_state();
    let cases = [
        (
            "/api/v0/mediacore/contentid/stats?unexpected=not-a-number",
            "/api/v0/mediacore/contentid/stats",
            Some("\"totalMappings\":0"),
        ),
        (
            "/api/v0/mediacore/ipld/validate?unexpected=not-a-number",
            "/api/v0/mediacore/ipld/validate",
            Some("\"isValid\":true"),
        ),
        (
            "/api/v0/mediacore/publish/stats?unexpected=not-a-number",
            "/api/v0/mediacore/publish/stats",
            Some("\"totalPublishedDescriptors\":0"),
        ),
        (
            "/api/v0/mediacore/retrieve/stats?unexpected=not-a-number",
            "/api/v0/mediacore/retrieve/stats",
            Some("\"totalRetrievals\":0"),
        ),
        (
            "/api/v0/mediacore/stats/dashboard?unexpected=not-a-number",
            "/api/v0/mediacore/stats/dashboard",
            Some("\"contentRegistry\""),
        ),
        (
            "/api/v0/mediacore/stats/descriptors?unexpected=not-a-number",
            "/api/v0/mediacore/stats/descriptors",
            Some("\"totalRetrievals\":0"),
        ),
        (
            "/api/v0/mediacore/stats/fuzzy?unexpected=not-a-number",
            "/api/v0/mediacore/stats/fuzzy",
            Some("\"totalMatches\":0"),
        ),
        (
            "/api/v0/mediacore/stats/ipld?unexpected=not-a-number",
            "/api/v0/mediacore/stats/ipld",
            Some("\"totalLinks\":0"),
        ),
        (
            "/api/v0/mediacore/stats/perceptual?unexpected=not-a-number",
            "/api/v0/mediacore/stats/perceptual",
            Some("\"totalHashesComputed\":0"),
        ),
        (
            "/api/v0/mediacore/stats/portability?unexpected=not-a-number",
            "/api/v0/mediacore/stats/portability",
            Some("\"totalExports\":0"),
        ),
        (
            "/api/v0/mediacore/stats/publishing?unexpected=not-a-number",
            "/api/v0/mediacore/stats/publishing",
            Some("\"totalPublished\":0"),
        ),
        (
            "/api/v0/mediacore/stats/registry?unexpected=not-a-number",
            "/api/v0/mediacore/stats/registry",
            Some("\"totalMappings\":0"),
        ),
    ];

    for (path, route_template, expected_body) in cases {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let pass = response.status == "200 OK"
            && expected_body.is_none_or(|expected| response.body.contains(expected));
        if !pass {
            mismatches.push(format!(
                "{target} GET {route_template} [malformed-path-query-or-body]: {}",
                response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": route_template,
            "case": "malformed-path-query-or-body",
            "pass": pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_stats_malformed_queries.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api MediaCore stats malformed-query mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting the remaining simple MediaCore GET
/// routes' malformed-query projections. Unknown query parameters are
/// ignored by the frozen actions; traversal still rejects its required
/// link-name parameter, and missing resources retain their oracle 404
/// shapes. slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_mediacore_resource_malformed_queries() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    let (state, _receiver) = test_state();
    let cases = [
        (
            "/api/v0/mediacore/contentid/domain/music?unexpected=not-a-number",
            "/api/v0/mediacore/contentid/domain/{domain}",
            "200 OK",
            Some("\"contentIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording?unexpected=not-a-number",
            "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
            "200 OK",
            Some("\"normalizedType\":\"recording\""),
        ),
        (
            "/api/v0/mediacore/contentid/exists/missing?unexpected=not-a-number",
            "/api/v0/mediacore/contentid/exists/{externalId}",
            "200 OK",
            Some("\"exists\":false"),
        ),
        (
            "/api/v0/mediacore/contentid/external/missing?unexpected=not-a-number",
            "/api/v0/mediacore/contentid/external/{contentId}",
            "200 OK",
            Some("\"externalIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/resolve/missing?unexpected=not-a-number",
            "/api/v0/mediacore/contentid/resolve/{externalId}",
            "404 Not Found",
            Some("External ID not found"),
        ),
        (
            "/api/v0/mediacore/contentid/validate/not-a-content-id?unexpected=not-a-number",
            "/api/v0/mediacore/contentid/validate/{*contentId}",
            "200 OK",
            Some("\"isValid\":false"),
        ),
        (
            "/api/v0/mediacore/ipld/graph/missing?unexpected=not-a-number",
            "/api/v0/mediacore/ipld/graph/{*contentId}",
            "200 OK",
            Some("\"nodes\":["),
        ),
        (
            "/api/v0/mediacore/ipld/inbound/missing?unexpected=not-a-number",
            "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
            "200 OK",
            Some("\"inboundLinks\":[]"),
        ),
        (
            "/api/v0/mediacore/ipld/traverse/content:audio:track:missing?unexpected=not-a-number",
            "/api/v0/mediacore/ipld/traverse/{*startContentId}",
            "400 Bad Request",
            Some("Link name is required"),
        ),
        (
            "/api/v0/mediacore/retrieve/descriptor/missing?unexpected=not-a-number",
            "/api/v0/mediacore/retrieve/descriptor/{*contentId}",
            "404 Not Found",
            Some("\"found\":false"),
        ),
        (
            "/api/v0/mediacore/retrieve/query/domain/music?unexpected=not-a-number",
            "/api/v0/mediacore/retrieve/query/domain/{domain}",
            "200 OK",
            Some("\"descriptors\":[]"),
        ),
        (
            "/api/v0/mediacore/perceptualhash/algorithms?unexpected=not-a-number",
            "/api/v0/mediacore/perceptualhash/algorithms",
            "200 OK",
            Some("\"algorithms\":["),
        ),
        (
            "/api/v0/mediacore/portability/merge-strategies?unexpected=not-a-number",
            "/api/v0/mediacore/portability/merge-strategies",
            "200 OK",
            Some("\"strategies\":["),
        ),
        (
            "/api/v0/mediacore/portability/strategies?unexpected=not-a-number",
            "/api/v0/mediacore/portability/strategies",
            "200 OK",
            Some("\"strategies\":["),
        ),
    ];

    for (path, route_template, status, expected_body) in cases {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let pass = response.status == status
            && expected_body.is_none_or(|expected| response.body.contains(expected));
        if !pass {
            mismatches.push(format!(
                "{target} GET {route_template} [malformed-path-query-or-body]: {}",
                response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": route_template,
            "case": "malformed-path-query-or-body",
            "pass": pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_resource_malformed_queries.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api MediaCore resource malformed-query mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 5 SongID run-lifecycle routes'
/// cases, independently re-derived from `songid_run_creation_
/// requires_a_real_non_empty_source`, `songid_run_reports_its_real_
/// completed_status_not_stuck_at_queued`, and `songid_run_evidence_
/// package_reshapes_real_stored_run_fields`'s real synchronous-
/// analysis and stored-run-field checks. slskdN-only (confirmed
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
pub(super) async fn controller_api_differential_songid_run_lifecycle() {
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

    let mut invalid_pass = true;
    for body in ["{}", r#"{"source":""}"#, r#"{"source":"   "}"#] {
        let response = crate::route_http_request("POST", "/api/v0/songid/runs", None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{body}: {error}"));
        invalid_pass &= response.status == "400 Bad Request"
            && response.body.contains("SongID source is required.");
    }
    record!(
        "POST",
        "/api/v0/songid/runs",
        "malformed-path-query-or-body",
        invalid_pass
    );

    let before = crate::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
        .await
        .expect("runs before");
    let before_count = serde_json::from_str::<serde_json::Value>(&before.body)
        .unwrap_or_default()
        .as_array()
        .map(Vec::len)
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/songid/runs",
        "nominal-status-headers-body",
        before.status == "200 OK"
    );

    let created = crate::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"differential","query":"Evidence Package Differential"}"#,
        &state,
    )
    .await
    .expect("create run");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    let run_id = created_json["id"].as_str().unwrap_or_default().to_owned();
    record!(
        "POST",
        "/api/v0/songid/runs",
        "nominal-status-headers-body",
        created.status == "202 Accepted"
            && created_json["status"] == "completed"
            && created_json["currentStage"] == "completed"
            && created_json["percentComplete"] == 1.0
    );

    let after = crate::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
        .await
        .expect("runs after");
    let after_count = serde_json::from_str::<serde_json::Value>(&after.body)
        .unwrap_or_default()
        .as_array()
        .map(Vec::len)
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/songid/runs",
        "mutation-side-effects-and-readback",
        after_count == before_count + 1
    );

    let polled_route = format!("/api/v0/songid/runs/{run_id}");
    let polled = crate::route_http_request("GET", &polled_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{polled_route}: {error}"));
    let polled_json = serde_json::from_str::<serde_json::Value>(&polled.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/songid/runs/{id:guid}",
        "nominal-status-headers-body",
        polled.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/songid/runs/{id:guid}",
        "populated-dynamic-state",
        polled_json["status"] == "completed"
    );

    let queue = crate::route_http_request("GET", "/api/v0/songid/runs/queue", None, "", &state)
        .await
        .expect("queue summary");
    let queue_json = serde_json::from_str::<serde_json::Value>(&queue.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/songid/runs/queue",
        "nominal-status-headers-body",
        queue.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/songid/runs/queue",
        "populated-dynamic-state",
        queue_json["completedCount"].as_u64().unwrap_or_default() >= 1
            && queue_json["queuedCount"] == 0
            && queue_json["runningCount"] == 0
    );

    let package_route = format!("/api/v0/songid/runs/{run_id}/evidence-package");
    let package = crate::route_http_request("GET", &package_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{package_route}: {error}"));
    let package_json = serde_json::from_str::<serde_json::Value>(&package.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/songid/runs/{id:guid}/evidence-package",
        "nominal-status-headers-body",
        package.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/songid/runs/{id:guid}/evidence-package",
        "populated-dynamic-state",
        package_json["runId"] == run_id
            && package_json["status"] == "completed"
            && package_json["query"] == "Evidence Package Differential"
            && package_json["completedAt"] == package_json["createdAt"]
            && package_json["trackCandidates"] == serde_json::json!([])
            && package_json["artifacts"] == serde_json::json!([])
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("songid_run_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api songid-run-lifecycle mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the security/transports/status
/// and listening-party routes' cases, independently re-derived from
/// `native_versioned_extended_gets_match_empty_state_contracts`'s
/// real empty-state checks and `listening_party_requires_membership_
/// and_reports_a_real_event`'s real membership-gated, forgery-
/// resistant event lifecycle. slskdN-only (confirmed against the
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
pub(super) async fn controller_api_differential_listening_party_and_transports_status() {
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

    let (empty_state, _empty_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let transports = crate::route_http_request(
        "GET",
        "/api/v0/security/transports/status",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("transport selector status");
    let transports_json =
        serde_json::from_str::<serde_json::Value>(&transports.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/security/transports/status",
        "nominal-status-headers-body",
        transports.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/security/transports/status",
        "populated-dynamic-state",
        transports_json["selectedMode"] == "Direct"
            && transports_json["totalTransports"] == 1
            && transports_json["availableTransports"] == 0
            && transports_json["availableTransportTypes"] == serde_json::json!([])
            && transports_json["lastConnectivityTest"].is_string()
            && transports_json["primaryTransportAvailable"] == false
            && transports_json["fallbackAvailable"] == false
    );

    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-differential";
    let channel_id = "general";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Differential",
            }))
            .expect("deserialize pod record fixture"),
            "tester".to_owned(),
        )
        .expect("create pod (tester is the owner/member)");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    let outside_pod = "pod:listening-party-differential-outsider";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": outside_pod,
                "name": "Not Tester's Pod",
            }))
            .expect("deserialize pod record fixture"),
            "someone-else".to_owned(),
        )
        .expect("create outsider pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            outside_pod,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create outsider channel");

    let forbidden_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{outside_pod}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forbidden get");
    let forbidden_post = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{outside_pod}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:x"}"#,
        &state,
    )
    .await
    .expect("forbidden post");
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "missing-empty-or-conflict-state",
        forbidden_get.status == "403 Forbidden"
    );
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "missing-empty-or-conflict-state",
        forbidden_post.status == "403 Forbidden"
    );

    let invalid_action = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"resume","contentId":"content:audio:track:x"}"#,
        &state,
    )
    .await
    .expect("invalid action");
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "malformed-path-query-or-body",
        invalid_action.status == "400 Bad Request"
    );

    let before = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("no state yet");
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "nominal-status-headers-body",
        before.status == "204 No Content"
    );

    let played = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:x","title":"Track","artist":"Artist","hostPeerId":"forged-peer"}"#,
        &state,
    )
    .await
    .expect("play event");
    let played_json = serde_json::from_str::<serde_json::Value>(&played.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "nominal-status-headers-body",
        played.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "mutation-side-effects-and-readback",
        played_json["action"] == "play"
            && played_json["podId"] == pod_id
            && played_json["channelId"] == channel_id
            && played_json["hostPeerId"] == "tester"
            && played_json["kind"] == "slskdn.listenAlong.v1"
            && played_json["partyId"]
                .as_str()
                .is_some_and(|id| id.starts_with("party:"))
    );

    let polled = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("poll event");
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "populated-dynamic-state",
        polled.status == "200 OK" && polled.body == played.body
    );

    let stopped = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop event");
    let after_stop = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("state after stop");
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "mutation-side-effects-and-readback",
        stopped.status == "200 OK" && after_stop.status == "204 No Content"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("listening_party_and_transports_status.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api listening-party-transports mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
