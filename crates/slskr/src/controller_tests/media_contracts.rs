use super::test_state;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mediacore_mutations_match_native_validation_and_result_dtos() {
    let (state, _receiver) = test_state();

    let fuzzy_text = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/text",
        None,
        r#"{"textA":"same","textB":"same"}"#,
        &state,
    )
    .await
    .unwrap();
    let fuzzy_text = serde_json::from_str::<serde_json::Value>(&fuzzy_text.body).unwrap();
    assert_eq!(fuzzy_text["levenshteinSimilarity"], 1.0);
    assert_eq!(fuzzy_text["phoneticSimilarity"], 1.0);
    assert_eq!(fuzzy_text["combinedSimilarity"], 1.0);

    let fuzzy_perceptual = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/perceptual",
        None,
        r#"{"contentIdA":"content:music:recording:a","contentIdB":"content:music:recording:b"}"#,
        &state,
    )
    .await
    .unwrap();
    let fuzzy_perceptual =
        serde_json::from_str::<serde_json::Value>(&fuzzy_perceptual.body).unwrap();
    assert_eq!(fuzzy_perceptual["similarity"], 0.0);
    assert_eq!(fuzzy_perceptual["isSimilar"], false);
    assert_eq!(fuzzy_perceptual["threshold"], 0.7);

    for (path, body) in [
        (
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[0.5],"sampleRate":1,"algorithm":"PHash"}"#,
        ),
        (
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":1,"height":1,"algorithm":"PHash"}"#,
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .unwrap();
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
        let response = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(response["algorithm"], "PHash");
        assert_eq!(response["hex"], "0000000000000000");
        assert_eq!(response["numericHash"], 0);
    }
    let similarity = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/perceptualhash/similarity",
        None,
        r#"{"hashA":"0000000000000000","hashB":"ffffffffffffffff","threshold":0.8}"#,
        &state,
    )
    .await
    .unwrap();
    let similarity = serde_json::from_str::<serde_json::Value>(&similarity.body).unwrap();
    assert_eq!(similarity["hammingDistance"], 64);
    assert_eq!(similarity["similarity"], 0.0);
    assert_eq!(similarity["areSimilar"], false);

    for (path, body) in [
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
    ] {
        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .unwrap();
        assert_eq!(
            response.status, "400 Bad Request",
            "{path}: {}",
            response.body
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
    .unwrap();
    assert_eq!(
        analysis.body,
        r#"{"cleanEntries":0,"conflictingEntries":0,"conflicts":[],"recommendedStrategies":{"Merge":0,"Overwrite":0,"Skip":0},"totalEntries":0}"#
    );

    let imported = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/import",
        None,
        empty_package,
        &state,
    )
    .await
    .unwrap();
    let imported = serde_json::from_str::<serde_json::Value>(&imported.body).unwrap();
    for key in [
        "success",
        "entriesProcessed",
        "entriesImported",
        "entriesSkipped",
        "conflictsResolved",
        "conflicts",
        "errors",
        "duration",
    ] {
        assert!(
            imported.get(key).is_some(),
            "missing import result field {key}"
        );
    }

    let verify = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/verify",
        None,
        r#"{"descriptor":{"contentId":"content:test:type:id","hashes":[]}}"#,
        &state,
    )
    .await
    .unwrap();
    let verify = serde_json::from_str::<serde_json::Value>(&verify.body).unwrap();
    assert_eq!(verify["isValid"], false);
    assert_eq!(verify["signatureValid"], false);
    assert_eq!(verify["freshnessValid"], false);
    assert!(verify["validationError"].as_str().is_some());

    let cache = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/retrieve/cache/clear",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        cache.body,
        r#"{"bytesFreed":0,"entriesCleared":0,"success":true}"#
    );

    let reset =
        crate::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .unwrap();
    assert_eq!(reset.body, r#"{"message":"Statistics reset successfully"}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mediacore_versioned_descriptor_delete_does_not_overflow_worker_stack() {
    let (state, _receiver) = test_state();
    let response = crate::route_http_request(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/content-id",
        None,
        "",
        &state,
    )
    .await
    .expect("delete descriptor");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap(),
        serde_json::json!({
            "contentId": "content-id",
            "success": true,
            "wasPublished": false,
        })
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mediacore_descriptor_updates_imports_and_stats_use_real_records() {
    let (state, _receiver) = test_state();
    let content_id = "content:test:recording:real";
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
    .unwrap();
    assert_eq!(published.status, "200 OK", "{}", published.body);

    let updated = crate::route_http_request(
        "PUT",
        &format!("/api/v0/mediacore/publish/descriptor/{content_id}"),
        None,
        r#"{"updates":{"title":"after","genre":"ambient"}}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "400 Bad Request", "{}", updated.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.body).unwrap(),
        serde_json::json!({"error": "Failed to update descriptor"})
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
    .unwrap();
    assert_eq!(retrieved.status, "200 OK", "{}", retrieved.body);
    let retrieved_json = serde_json::from_str::<serde_json::Value>(&retrieved.body).unwrap();
    assert_eq!(retrieved_json["requested"], 2);
    assert_eq!(retrieved_json["found"], 1);
    assert_eq!(retrieved_json["results"].as_array().unwrap().len(), 2);

    let stats =
        crate::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
            .await
            .unwrap();
    let stats = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats["totalRetrievals"], 2);
    assert_eq!(stats["cacheHits"], 0);
    assert_eq!(stats["cacheMisses"], 2);
    assert_eq!(stats["activeCacheEntries"], 1);

    let cached = crate::route_http_request(
        "GET",
        &format!("/api/v0/mediacore/retrieve/descriptor/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let cached_json = serde_json::from_str::<serde_json::Value>(&cached.body).unwrap();
    assert_eq!(cached.status, "200 OK");
    assert_eq!(cached_json["found"], true);
    assert_eq!(cached_json["fromCache"], true);

    let imported = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/import",
        None,
        &serde_json::json!({
            "package": {
                "entries": [{
                    "contentId": "content:test:recording:imported",
                    "descriptor": {
                        "contentId": "content:test:recording:imported",
                        "title": "imported",
                        "hashes": [{"algorithm": "sha256", "hex": "bbbb"}]
                    }
                }]
            }
        })
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    let imported = serde_json::from_str::<serde_json::Value>(&imported.body).unwrap();
    assert_eq!(imported["success"], true);
    assert_eq!(imported["entriesProcessed"], 1);
    assert_eq!(imported["entriesImported"], 1);

    let publishing = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/publishing",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let publishing = serde_json::from_str::<serde_json::Value>(&publishing.body).unwrap();
    assert_eq!(publishing["totalPublished"], 1);
    assert_eq!(publishing["activePublications"], 1);
    assert_eq!(publishing["publicationsByDomain"]["test"], 1);

    let publisher_stats =
        crate::route_http_request("GET", "/api/v0/mediacore/publish/stats", None, "", &state)
            .await
            .unwrap();
    let publisher_stats = serde_json::from_str::<serde_json::Value>(&publisher_stats.body).unwrap();
    assert_eq!(publisher_stats["totalPublishedDescriptors"], 1);
    assert_eq!(publisher_stats["activePublications"], 1);
    assert_eq!(publisher_stats["publicationsByDomain"]["test"], 1);
    assert!(publisher_stats["averageTtlHours"].as_f64().unwrap() > 0.0);

    let reset =
        crate::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .unwrap();
    assert_eq!(reset.body, r#"{"message":"Statistics reset successfully"}"#);
    let stats =
        crate::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stats.body).unwrap()["totalRetrievals"],
        0
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mediacore_fuzzy_matching_and_ipld_validation_use_persisted_state() {
    let (state, _receiver) = test_state();
    let target = "content:audio:track:target";
    let candidate = "content:audio:track:candidate";
    for (external_id, content_id) in [("target", target), ("candidate", candidate)] {
        let response = crate::route_http_request(
            "POST",
            "/api/v0/mediacore/contentid/register",
            None,
            &serde_json::json!({"externalId": external_id, "contentId": content_id}).to_string(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "200 OK", "{}", response.body);
        let response = crate::route_http_request(
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
        .unwrap();
        assert_eq!(response.status, "200 OK", "{}", response.body);
    }

    let perceptual = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/fuzzymatch/perceptual",
        None,
        &serde_json::json!({"contentIdA": target, "contentIdB": candidate}).to_string(),
        &state,
    )
    .await
    .unwrap();
    let perceptual = serde_json::from_str::<serde_json::Value>(&perceptual.body).unwrap();
    assert_eq!(perceptual["similarity"], 1.0);
    assert_eq!(perceptual["isSimilar"], true);

    let matches = crate::route_http_request(
        "POST",
        &format!("/api/v0/mediacore/fuzzymatch/find/{target}"),
        None,
        r#"{"minConfidence":0.7,"maxCandidates":50,"maxResults":10}"#,
        &state,
    )
    .await
    .unwrap();
    let matches = serde_json::from_str::<serde_json::Value>(&matches.body).unwrap();
    assert_eq!(matches["totalCandidates"], 1);
    assert_eq!(matches["matches"][0]["candidateContentId"], candidate);
    assert_eq!(matches["matches"][0]["reason"], "PerceptualHash");

    let links = crate::route_http_request(
        "POST",
        &format!("/api/v0/mediacore/ipld/links/{target}"),
        None,
        &serde_json::json!({"links": [
            {"name": "same", "target": candidate},
            {"name": "broken", "target": "content:audio:track:missing"},
        ]})
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(links.status, "200 OK", "{}", links.body);
    let orphan = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/ipld/links/content:audio:track:orphan",
        None,
        &serde_json::json!({"links": [{"name": "same", "target": candidate}]}).to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(orphan.status, "200 OK", "{}", orphan.body);

    let validation =
        crate::route_http_request("GET", "/api/v0/mediacore/ipld/validate", None, "", &state)
            .await
            .unwrap();
    let validation = serde_json::from_str::<serde_json::Value>(&validation.body).unwrap();
    assert_eq!(validation["isValid"], false);
    assert_eq!(validation["totalLinksValidated"], 2);
    assert_eq!(validation["brokenLinks"].as_array().unwrap().len(), 1);
    assert_eq!(validation["orphanedLinks"].as_array().unwrap().len(), 1);

    let inbound = crate::route_http_request(
        "GET",
        &format!("/api/v0/mediacore/ipld/inbound/{candidate}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let inbound = serde_json::from_str::<serde_json::Value>(&inbound.body).unwrap();
    assert_eq!(inbound["inboundLinks"].as_array().unwrap().len(), 2);
    assert!(inbound["inboundLinks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source == target));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn fuzzy_edit_distance_uses_the_shorter_rolling_dimension() {
    let long = "a".repeat(2_048);
    assert_eq!(
        crate::mediacore_controller::levenshtein_similarity("b", &long),
        crate::mediacore_controller::levenshtein_similarity(&long, "b")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn fuzzy_edit_distance_trims_shared_affixes_without_changing_score() {
    assert_eq!(
        crate::mediacore_controller::levenshtein_similarity(
            "prefixkitten-suffix",
            "prefixsitting-suffix",
        ),
        0.85
    );
    let prefix = "a".repeat(20_000);
    assert_eq!(
        crate::mediacore_controller::levenshtein_similarity(
            &format!("{prefix}b"),
            &format!("{prefix}c"),
        ),
        1.0 - 1.0 / 20_001.0
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn analyzer_migration_requires_version_and_returns_exact_result_shape() {
    let (state, _receiver) = test_state();
    let unversioned =
        crate::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
            .await
            .unwrap();
    assert_eq!(unversioned.status, "400 Bad Request");

    let versioned =
        crate::route_http_request("POST", "/api/v0/audio/analyzers/migrate", None, "", &state)
            .await
            .unwrap();
    assert_eq!(versioned.status, "200 OK");
    assert_eq!(versioned.body, r#"{"updated":0}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn hashdb_optimize_profile_and_slow_queries_report_real_observed_data() {
    let (state, _receiver) = test_state();

    // Prime the store with a real hash entry so the profile/slow-query
    // endpoints have genuine data to report on, not an empty store.
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "route-audit-key".to_owned(),
                size: 123,
                file_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed hash entry");
    }

    let profile = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        r#"{"query":"SELECT * FROM hash_entries"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(profile.status, "200 OK");
    let profile = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap();
    assert_eq!(profile["query"], "SELECT * FROM hash_entries");
    assert_eq!(profile["rowsReturned"], 1);
    assert!(profile["executionTimeMs"].is_u64());
    assert!(
        profile["queryPlan"]
            .as_str()
            .unwrap()
            .contains("linear scan"),
        "{profile}"
    );

    let slow_queries = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(slow_queries.status, "200 OK");
    let slow_queries = serde_json::from_str::<serde_json::Value>(&slow_queries.body).unwrap();
    assert_eq!(slow_queries["totalQueries"], 1);
    let entries = slow_queries["slowQueries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["query"], "profile_query");
    assert_eq!(entries[0]["executionCount"], 1);
    assert_eq!(entries[0]["totalRowsReturned"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn hashdb_optimize_analyze_reports_real_observed_counts_and_thresholds() {
    let (state, _receiver) = test_state();

    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "analyze-audit-key".to_owned(),
                size: 456,
                file_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed hash entry");
        discovery
            .merge_shadow_records(vec![crate::content_discovery::ShadowIndexRecord {
                recording_id: "mbid-analyze-audit".to_owned(),
                peer_ids: vec!["peer-a".to_owned(), "peer-b".to_owned()],
                updated_at: 0,
            }])
            .expect("seed shadow record");
    }

    let analyze =
        crate::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
            .await
            .unwrap();
    assert_eq!(analyze.status, "200 OK");
    let analyze = serde_json::from_str::<serde_json::Value>(&analyze.body).unwrap();
    assert_eq!(analyze["hashDbEntryCount"], 1);
    assert_eq!(analyze["entries"], 1);
    assert_eq!(analyze["peerCount"], 2);
    assert!(analyze["databaseSizeBytes"].is_u64());
    // An in-memory store (no state file yet) has nothing on disk.
    assert_eq!(analyze["databaseSizeBytes"], 0);
    // Below both real thresholds, so no recommendations should fire.
    assert_eq!(analyze["recommendations"], serde_json::json!([]));
    assert_eq!(analyze["missingIndexes"], serde_json::json!([]));
}
