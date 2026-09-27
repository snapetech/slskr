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
async fn controller_api_differential_mediacore_descriptor_lifecycle() {
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
            "timestampUnixMs": super::unix_timestamp_millis(),
        },
    });
    let published = super::route_http_request(
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

    let updated = super::route_http_request(
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
    let update_readback = super::route_http_request(
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

    let retrieved = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
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
    let cached = super::route_http_request("GET", &cached_route, None, "", &state)
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

    let query = super::route_http_request(
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

    let descriptor_stats = super::route_http_request(
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

    let publishing = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/mediacore/publish/stats", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/mediacore/stats/dashboard", None, "", &state)
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
            "timestampUnixMs": super::unix_timestamp_millis(),
        },
    });
    let batch_published = super::route_http_request(
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

    let republished = super::route_http_request(
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

    let exported = super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/mediacore/stats/reset", None, "", &state)
            .await
            .expect("stats reset");
    let stats_after_reset =
        super::route_http_request("GET", "/api/v0/mediacore/retrieve/stats", None, "", &state)
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
    let deleted = super::route_http_request("DELETE", delete_route, None, "", &state)
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
async fn controller_api_differential_mediacore_ipld_and_fuzzy_find() {
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
        let registered = super::route_http_request(
            "POST",
            "/api/v0/mediacore/contentid/register",
            None,
            &serde_json::json!({"externalId": external_id, "contentId": content_id}).to_string(),
            &state,
        )
        .await
        .expect("register content id");
        register_pass &= registered.status == "200 OK";

        let published = super::route_http_request(
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
                        "timestampUnixMs": super::unix_timestamp_millis(),
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
        super::route_http_request("GET", "/api/v0/mediacore/stats/registry", None, "", &state)
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

    let matches = super::route_http_request(
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
    let links = super::route_http_request(
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

    let orphan = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/mediacore/stats/ipld", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/mediacore/ipld/validate", None, "", &state)
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
    let inbound = super::route_http_request("GET", &inbound_route, None, "", &state)
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
    let graph = super::route_http_request("GET", &graph_route, None, "", &state)
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
    let traverse = super::route_http_request("GET", &traverse_route, None, "", &state)
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

/// Bulk differential proof crediting 34 GET routes'
/// `missing-empty-or-conflict-state` cases, independently re-derived
/// from `materialized_controller_gets_match_native_empty_state_
/// contracts`'s real empty-state response-shape checks for
/// nonexistent resources. slskdN-only (confirmed against the frozen
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
async fn controller_api_differential_materialized_empty_state_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    let (state, _receiver) = test_state();
    let cases = [
        (
            "/api/v0/nowplaying",
            "/api/v0/nowplaying",
            "204 No Content",
            None,
        ),
        (
            "/api/v0/listening-party/missing-pod/missing-channel",
            "/api/v0/listening-party/{podId}/{channelId}",
            "404 Not Found",
            Some("\"error\":\"not found\""),
        ),
        (
            "/api/v0/mediacore/contentid/domain/music",
            "/api/v0/mediacore/contentid/domain/{domain}",
            "200 OK",
            Some("\"contentIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
            "200 OK",
            Some("\"normalizedType\":\"recording\""),
        ),
        (
            "/api/v0/mediacore/contentid/stats",
            "/api/v0/mediacore/contentid/stats",
            "200 OK",
            Some("\"totalMappings\":0"),
        ),
        (
            "/api/v0/mediacore/contentid/exists/missing",
            "/api/v0/mediacore/contentid/exists/{externalId}",
            "200 OK",
            Some("\"exists\":false"),
        ),
        (
            "/api/v0/mediacore/contentid/external/missing",
            "/api/v0/mediacore/contentid/external/{contentId}",
            "200 OK",
            Some("\"externalIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/validate/not-a-content-id",
            "/api/v0/mediacore/contentid/validate/{*contentId}",
            "200 OK",
            Some("\"isValid\":false"),
        ),
        (
            "/api/v0/mediacore/ipld/graph/missing",
            "/api/v0/mediacore/ipld/graph/{*contentId}",
            "200 OK",
            Some("\"nodes\":["),
        ),
        (
            "/api/v0/mediacore/ipld/inbound/missing",
            "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
            "200 OK",
            Some("\"inboundLinks\":[]"),
        ),
        (
            "/api/v0/mediacore/ipld/traverse/content:audio:track:missing?linkName=missing",
            "/api/v0/mediacore/ipld/traverse/{*startContentId}",
            "200 OK",
            Some("\"visitedNodes\":["),
        ),
        (
            "/api/v0/mediacore/ipld/validate",
            "/api/v0/mediacore/ipld/validate",
            "200 OK",
            Some("\"isValid\":true"),
        ),
        (
            "/api/v0/mediacore/retrieve/query/domain/music",
            "/api/v0/mediacore/retrieve/query/domain/{domain}",
            "200 OK",
            Some("\"descriptors\":[]"),
        ),
        (
            "/api/v0/mediacore/perceptualhash/algorithms",
            "/api/v0/mediacore/perceptualhash/algorithms",
            "200 OK",
            Some("\"algorithms\":["),
        ),
        (
            "/api/v0/mediacore/portability/merge-strategies",
            "/api/v0/mediacore/portability/merge-strategies",
            "200 OK",
            Some("\"strategies\":["),
        ),
        (
            "/api/v0/mediacore/portability/strategies",
            "/api/v0/mediacore/portability/strategies",
            "200 OK",
            Some("\"strategies\":["),
        ),
        (
            "/api/v0/mediacore/retrieve/stats",
            "/api/v0/mediacore/retrieve/stats",
            "200 OK",
            Some("\"totalRetrievals\":0"),
        ),
        (
            "/api/v0/mediacore/publish/stats",
            "/api/v0/mediacore/publish/stats",
            "200 OK",
            Some("\"totalPublishedDescriptors\":0"),
        ),
        (
            "/api/v0/mediacore/stats/dashboard",
            "/api/v0/mediacore/stats/dashboard",
            "200 OK",
            Some("\"contentRegistry\""),
        ),
        (
            "/api/v0/mediacore/stats/descriptors",
            "/api/v0/mediacore/stats/descriptors",
            "200 OK",
            Some("\"totalRetrievals\":0"),
        ),
        (
            "/api/v0/mediacore/stats/fuzzy",
            "/api/v0/mediacore/stats/fuzzy",
            "200 OK",
            Some("\"totalMatches\":0"),
        ),
        (
            "/api/v0/mediacore/stats/ipld",
            "/api/v0/mediacore/stats/ipld",
            "200 OK",
            Some("\"totalLinks\":0"),
        ),
        (
            "/api/v0/mediacore/stats/perceptual",
            "/api/v0/mediacore/stats/perceptual",
            "200 OK",
            Some("\"totalHashesComputed\":0"),
        ),
        (
            "/api/v0/mediacore/stats/portability",
            "/api/v0/mediacore/stats/portability",
            "200 OK",
            Some("\"totalExports\":0"),
        ),
        (
            "/api/v0/mediacore/stats/publishing",
            "/api/v0/mediacore/stats/publishing",
            "200 OK",
            Some("\"totalPublished\":0"),
        ),
        (
            "/api/v0/mediacore/stats/registry",
            "/api/v0/mediacore/stats/registry",
            "200 OK",
            Some("\"totalMappings\":0"),
        ),
        (
            "/api/v0/mediacore/retrieve/descriptor/content:audio:track:missing",
            "/api/v0/mediacore/retrieve/descriptor/{*contentId}",
            "404 Not Found",
            Some("\"found\":false"),
        ),
        (
            "/api/v0/podcore/missing/opinions/members/affinity",
            "/api/v0/podcore/{podId}/opinions/members/affinity",
            "200 OK",
            Some("{}"),
        ),
        (
            "/api/v0/podcore/backfill/missing/last-seen",
            "/api/v0/podcore/backfill/{podId}/last-seen",
            "200 OK",
            Some("{}"),
        ),
        (
            "/api/v0/pods/missing/channels/missing/messages",
            "/api/v0/pods/{podId}/channels/{channelId}/messages",
            "200 OK",
            Some("[]"),
        ),
        (
            "/api/v0/quarantine-jury/requests/missing/routes",
            "/api/v0/quarantine-jury/requests/{requestId}/routes",
            "200 OK",
            Some("[]"),
        ),
        (
            "/api/v0/security/disclosure/missing",
            "/api/v0/security/disclosure/{username}",
            "200 OK",
            Some("\"peerTier\":\"Unknown\""),
        ),
        (
            "/api/v0/security/reputation/missing",
            "/api/v0/security/reputation/{username}",
            "200 OK",
            Some("\"score\":50"),
        ),
        (
            "/api/v0/traces/missing/summary",
            "/api/v0/traces/{jobId}/summary",
            "200 OK",
            Some("\"totalEvents\":0"),
        ),
    ];

    for (path, route_template, status, expected_body) in cases {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let pass = response.status == status
            && match expected_body {
                Some(expected) => response.body.contains(expected),
                None => response.body.is_empty(),
            };
        if !pass {
            mismatches.push(format!(
                "{target} GET {route_template} [missing-empty-or-conflict-state]: {}",
                response.body
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
        evidence_dir.join("materialized_empty_state_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api materialized-empty-state mismatches:\n{}",
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
async fn controller_api_differential_mediacore_stats_malformed_queries() {
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
        let response = super::route_http_request("GET", path, None, "", &state)
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
async fn controller_api_differential_mediacore_resource_malformed_queries() {
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
        let response = super::route_http_request("GET", path, None, "", &state)
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

/// Bulk differential proof crediting the remaining tail of
/// `openapi_mutation_dtos_match_native_status_and_field_contracts`
/// (everything after the Collections/CollectionItems lifecycle, which
/// `controller_api_differential_collections_items_crud_reorder_
/// lifecycle` already credits): mediacore content-id registration
/// validation, nowplaying, podcore content-pod creation, the full
/// wishlist lifecycle, overlay IP blocklisting, quarantine-jury
/// request validation, and security IP bans. slskdN-only (confirmed
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
async fn controller_api_differential_openapi_mutation_dtos_tail() {
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

    let invalid_content_id = super::route_http_request(
        "POST",
        "/api/v0/mediacore/contentid/register",
        None,
        r#"{"externalId":"mbid","contentId":"invalid"}"#,
        &state,
    )
    .await
    .expect("register invalid content id");
    record!(
        "POST",
        "/api/v0/mediacore/contentid/register",
        "malformed-path-query-or-body",
        invalid_content_id.status == "400 Bad Request"
    );

    let valid_content_id = super::route_http_request(
        "POST",
        "/api/v0/mediacore/contentid/register",
        None,
        r#"{"externalId":"mbid","contentId":"content:music:recording:mbid"}"#,
        &state,
    )
    .await
    .expect("register valid content id");
    record!(
        "POST",
        "/api/v0/mediacore/contentid/register",
        "mutation-side-effects-and-readback",
        valid_content_id.status == "200 OK"
            && valid_content_id
                .body
                .contains("mapping registered successfully")
    );

    let mut content_id_gets_pass = true;
    for (path, expected) in [
        (
            "/api/v0/mediacore/contentid/exists/mbid",
            r#"{"exists":true}"#,
        ),
        (
            "/api/v0/mediacore/contentid/external/content%3Amusic%3Arecording%3Ambid",
            r#"{"externalIds":["mbid"]}"#,
        ),
        (
            "/api/v0/mediacore/contentid/domain/music",
            r#""contentIds":["content:music:recording:mbid"]"#,
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            r#""contentIds":["content:music:recording:mbid"]"#,
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        content_id_gets_pass &= response.status == "200 OK" && response.body.contains(expected);
    }
    record!(
        "GET",
        "/api/v0/mediacore/contentid/exists/{externalId}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/external/{contentId}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/exists/{externalId}",
        "populated-dynamic-state",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/external/{contentId}",
        "populated-dynamic-state",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}",
        "populated-dynamic-state",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
        "populated-dynamic-state",
        content_id_gets_pass
    );

    let resolved_content_id = super::route_http_request(
        "GET",
        "/api/v0/mediacore/contentid/resolve/mbid",
        None,
        "",
        &state,
    )
    .await
    .expect("resolve content id");
    let resolved_content_id_json =
        serde_json::from_str::<serde_json::Value>(&resolved_content_id.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/contentid/resolve/{externalId}",
        "nominal-status-headers-body",
        resolved_content_id.status == "200 OK" && !resolved_content_id.body.is_empty()
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/resolve/{externalId}",
        "populated-dynamic-state",
        resolved_content_id.status == "200 OK"
            && resolved_content_id_json["contentId"] == "content:music:recording:mbid"
    );

    let unresolved_content_id = super::route_http_request(
        "GET",
        "/api/v0/mediacore/contentid/resolve/missing",
        None,
        "",
        &state,
    )
    .await
    .expect("resolve missing content id");
    record!(
        "GET",
        "/api/v0/mediacore/contentid/resolve/{externalId}",
        "missing-empty-or-conflict-state",
        unresolved_content_id.status == "404 Not Found"
            && unresolved_content_id.body.contains("External ID not found")
    );

    let content_id_stats =
        super::route_http_request("GET", "/api/v0/mediacore/contentid/stats", None, "", &state)
            .await
            .expect("content id stats");
    let content_id_stats_json =
        serde_json::from_str::<serde_json::Value>(&content_id_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/contentid/stats",
        "populated-dynamic-state",
        content_id_stats.status == "200 OK"
            && content_id_stats_json["totalMappings"] == 1
            && content_id_stats_json["totalDomains"] == 1
            && content_id_stats_json["mappingsByDomain"]["music"] == 1
    );

    let valid_content_id_readback = super::route_http_request(
        "GET",
        "/api/v0/mediacore/contentid/validate/content%3Amusic%3Arecording%3Ambid",
        None,
        "",
        &state,
    )
    .await
    .expect("validate content id");
    let valid_content_id_json =
        serde_json::from_str::<serde_json::Value>(&valid_content_id_readback.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/contentid/validate/{*contentId}",
        "nominal-status-headers-body",
        valid_content_id_readback.status == "200 OK" && !valid_content_id_readback.body.is_empty()
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/validate/{*contentId}",
        "populated-dynamic-state",
        valid_content_id_readback.status == "200 OK"
            && valid_content_id_json["contentId"] == "content:music:recording:mbid"
            && valid_content_id_json["isValid"] == true
            && valid_content_id_json["domain"] == "music"
            && valid_content_id_json["type"] == "recording"
            && valid_content_id_json["id"] == "mbid"
    );

    let now_playing = super::route_http_request(
        "PUT",
        "/api/v0/nowplaying",
        None,
        r#"{"artist":"Artist","title":"Track","album":"Album"}"#,
        &state,
    )
    .await
    .expect("set now playing");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "nominal-status-headers-body",
        now_playing.status == "204 No Content" && now_playing.body.is_empty()
    );
    let now_playing_readback =
        super::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("get now playing");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "mutation-side-effects-and-readback",
        now_playing.status == "204 No Content"
            && now_playing_readback.status == "200 OK"
            && now_playing_readback.body.contains("Artist")
            && now_playing_readback.body.contains("Track")
    );
    record!(
        "GET",
        "/api/v0/nowplaying",
        "nominal-status-headers-body",
        now_playing_readback.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/nowplaying",
        "populated-dynamic-state",
        now_playing_readback.status == "200 OK"
            && now_playing_readback.body.contains("Artist")
            && now_playing_readback.body.contains("Track")
    );

    let invalid_content_pod = super::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:invalid","name":"Differential","visibility":"Listed","contentId":"content:music:recording:differential"}"#,
        &state,
    )
    .await
    .expect("create invalid content pod");
    record!(
        "POST",
        "/api/v0/podcore/content/create-pod",
        "malformed-path-query-or-body",
        invalid_content_pod.status == "400 Bad Request"
    );

    let content_pod = super::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:00000000000000000000000000000003","name":"Differential","visibility":"Listed","contentId":"content:music:recording:differential","tags":[],"channels":[],"externalBindings":[]}"#,
        &state,
    )
    .await
    .expect("create valid content pod");
    let content_pod_json =
        serde_json::from_str::<serde_json::Value>(&content_pod.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/content/create-pod",
        "nominal-status-headers-body",
        content_pod.status == "201 Created"
            && content_pod_json["focusContentId"] == "content:music:recording:differential"
    );
    let content_pod_readback = super::route_http_request(
        "GET",
        "/api/v0/pods/pod%3A00000000000000000000000000000003",
        None,
        "",
        &state,
    )
    .await
    .expect("read content-linked pod");
    record!(
        "POST",
        "/api/v0/podcore/content/create-pod",
        "mutation-side-effects-and-readback",
        content_pod.status == "201 Created"
            && content_pod_json["focusContentId"] == "content:music:recording:differential"
            && content_pod_readback.status == "200 OK"
            && content_pod_readback
                .body
                .contains("content:music:recording:differential")
    );

    let wishlist_item = super::route_http_request(
        "POST",
        "/api/v0/wishlist",
        None,
        r#"{"searchText":"differential","filter":"differential","enabled":true,"autoDownload":true,"maxResults":1,"maxDownloads":1}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    let wishlist_json =
        serde_json::from_str::<serde_json::Value>(&wishlist_item.body).unwrap_or_default();
    let wishlist_id = wishlist_json["id"].as_str().unwrap_or_default().to_owned();
    record!(
        "POST",
        "/api/v0/wishlist",
        "nominal-status-headers-body",
        wishlist_item.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v0/wishlist",
        "mutation-side-effects-and-readback",
        uuid::Uuid::parse_str(&wishlist_id).is_ok()
            && wishlist_json["searchText"] == "differential"
            && wishlist_json["maxResults"] == 1
            && wishlist_json.get("artist").is_none()
    );

    let updated_wishlist = super::route_http_request(
        "PUT",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        r#"{"searchText":"updated-differential","filter":"lossless","enabled":true,"autoDownload":false,"maxResults":5,"maxDownloads":1}"#,
        &state,
    )
    .await
    .expect("update wishlist item");
    let updated_wishlist_json =
        serde_json::from_str::<serde_json::Value>(&updated_wishlist.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/wishlist/{id}",
        "mutation-side-effects-and-readback",
        updated_wishlist_json["searchText"] == "updated-differential"
            && updated_wishlist_json["maxResults"] == 5
    );

    let ignored_result = super::route_http_request(
        "POST",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results"),
        None,
        r#"{"username":"differential-peer","directory":"/tmp/slskdn-differential"}"#,
        &state,
    )
    .await
    .expect("add ignored result");
    let ignored_json =
        serde_json::from_str::<serde_json::Value>(&ignored_result.body).unwrap_or_default();
    let ignored_id = ignored_json["id"].as_str().unwrap_or_default().to_owned();
    record!(
        "POST",
        "/api/v0/wishlist/{id}/ignored-results",
        "nominal-status-headers-body",
        ignored_result.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v0/wishlist/{id}/ignored-results",
        "mutation-side-effects-and-readback",
        ignored_json["wishlistItemId"] == wishlist_id
            && ignored_json["directory"] == "/tmp/slskdn-differential"
    );
    let ignored_list = super::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results"),
        None,
        "",
        &state,
    )
    .await
    .expect("list ignored results");
    let ignored_list_json =
        serde_json::from_str::<serde_json::Value>(&ignored_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/wishlist/{id}/ignored-results",
        "nominal-status-headers-body",
        ignored_list.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/wishlist/{id}/ignored-results",
        "populated-dynamic-state",
        ignored_list.status == "200 OK"
            && ignored_list_json.as_array().is_some_and(|entries| {
                entries.len() == 1
                    && entries[0]["wishlistItemId"] == wishlist_id
                    && entries[0]["directory"] == "/tmp/slskdn-differential"
            })
    );

    let deleted_ignored = super::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results/{ignored_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete ignored result");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}/ignored-results/{ignoredResultId}",
        "nominal-status-headers-body",
        deleted_ignored.status == "204 No Content"
    );
    let ignored_after_delete = super::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results"),
        None,
        "",
        &state,
    )
    .await
    .expect("read deleted ignored result");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}/ignored-results/{ignoredResultId}",
        "mutation-side-effects-and-readback",
        deleted_ignored.status == "204 No Content"
            && ignored_after_delete.status == "200 OK"
            && ignored_after_delete.body == "[]"
    );

    let wishlist_before_view = super::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read wishlist before marking viewed");
    let wishlist_before_view_json =
        serde_json::from_str::<serde_json::Value>(&wishlist_before_view.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/wishlist/{id}",
        "nominal-status-headers-body",
        wishlist_before_view.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/wishlist/{id}",
        "populated-dynamic-state",
        wishlist_before_view.status == "200 OK"
            && wishlist_before_view_json["id"] == wishlist_id
            && wishlist_before_view_json["searchText"] == "updated-differential"
    );

    let marked_viewed = super::route_http_request(
        "POST",
        &format!("/api/v0/wishlist/{wishlist_id}/mark-viewed"),
        None,
        "",
        &state,
    )
    .await
    .expect("mark wishlist viewed");
    let wishlist_after_view = super::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read wishlist after marking viewed");
    record!(
        "POST",
        "/api/v0/wishlist/{id}/mark-viewed",
        "nominal-status-headers-body",
        marked_viewed.status == "204 No Content"
    );
    let wishlist_after_view_json =
        serde_json::from_str::<serde_json::Value>(&wishlist_after_view.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/wishlist/{id}/mark-viewed",
        "mutation-side-effects-and-readback",
        marked_viewed.status == "204 No Content"
            && wishlist_after_view.status == "200 OK"
            && wishlist_before_view_json["lastViewedAt"].is_null()
            && wishlist_after_view_json["lastViewedAt"].is_string()
    );

    let deleted_wishlist = super::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete wishlist item");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}",
        "nominal-status-headers-body",
        deleted_wishlist.status == "204 No Content"
    );
    let deleted_wishlist_readback = super::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read deleted wishlist item");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}",
        "mutation-side-effects-and-readback",
        deleted_wishlist.status == "204 No Content"
            && deleted_wishlist_readback.status == "404 Not Found"
    );

    let block = super::route_http_request(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        None,
        r#"{"ip":"192.0.2.9","reason":"differential"}"#,
        &state,
    )
    .await
    .expect("block ip");
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "nominal-status-headers-body",
        block.status == "200 OK" && block.body == r#"{"message":"IP address blocked"}"#
    );
    let blocklist = super::route_http_request("GET", "/api/v0/overlay/blocklist", None, "", &state)
        .await
        .expect("list overlay blocklist");
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "mutation-side-effects-and-readback",
        block.status == "200 OK"
            && blocklist.status == "200 OK"
            && blocklist.body.contains("192.0.2.9")
    );
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "populated-dynamic-state",
        blocklist.status == "200 OK" && blocklist.body.contains("192.0.2.9")
    );

    let invalid_jury = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"evidence":[],"jurors":[],"minJurorVotes":1}"#,
        &state,
    )
    .await
    .expect("validate invalid jury request");
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests",
        "malformed-path-query-or-body",
        invalid_jury.status == "400 Bad Request"
            && invalid_jury.body.contains("trusted juror")
            && invalid_jury.body.contains("minimal evidence")
    );

    let security_ban = super::route_http_request(
        "POST",
        "/api/v0/security/bans/ip",
        None,
        r#"{"ipAddress":"198.51.100.9","reason":"differential"}"#,
        &state,
    )
    .await
    .expect("ban ip");
    record!(
        "POST",
        "/api/v0/security/bans/ip",
        "nominal-status-headers-body",
        security_ban.status == "200 OK" && security_ban.body.is_empty()
    );

    let security_bans = super::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
        .await
        .expect("list security bans");
    let security_bans_json =
        serde_json::from_str::<serde_json::Value>(&security_bans.body).unwrap_or_default();
    let security_record = security_bans_json
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record["key"] == "IP:198.51.100.9")
        })
        .cloned()
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/security/bans",
        "populated-dynamic-state",
        security_bans.status == "200 OK"
            && security_record["reason"] == "differential"
            && security_record["isPermanent"] == false
            && security_record["timeRemaining"].as_str().is_some()
    );
    record!(
        "POST",
        "/api/v0/security/bans/ip",
        "mutation-side-effects-and-readback",
        security_ban.status == "200 OK"
            && security_ban.body.is_empty()
            && security_record["key"] == "IP:198.51.100.9"
            && security_record["reason"] == "differential"
            && security_record["isPermanent"] == false
            && security_record["timeRemaining"].as_str().is_some()
    );

    let username_ban = super::route_http_request(
        "POST",
        "/api/v0/security/bans/username",
        None,
        r#"{"username":"Differential-User","reason":"username differential"}"#,
        &state,
    )
    .await
    .expect("ban username");
    record!(
        "POST",
        "/api/v0/security/bans/username",
        "nominal-status-headers-body",
        username_ban.status == "200 OK" && username_ban.body.is_empty()
    );

    let username_bans = super::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
        .await
        .expect("list username ban");
    let username_bans_json =
        serde_json::from_str::<serde_json::Value>(&username_bans.body).unwrap_or_default();
    let username_record = username_bans_json
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record["key"] == "User:differential-user")
        })
        .cloned()
        .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/security/bans/username",
        "mutation-side-effects-and-readback",
        username_ban.status == "200 OK"
            && username_ban.body.is_empty()
            && username_bans.status == "200 OK"
            && username_record["reason"] == "username differential"
            && username_record["isPermanent"] == false
            && username_record["timeRemaining"].as_str().is_some()
    );

    let username_unban = super::route_http_request(
        "DELETE",
        "/api/v0/security/bans/username/differential-user",
        None,
        "",
        &state,
    )
    .await
    .expect("unban username");
    record!(
        "DELETE",
        "/api/v0/security/bans/username/{username}",
        "nominal-status-headers-body",
        username_unban.status == "200 OK" && username_unban.body.is_empty()
    );

    let username_bans_after_delete =
        super::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
            .await
            .expect("list bans after username unban");
    let username_bans_after_delete_json =
        serde_json::from_str::<serde_json::Value>(&username_bans_after_delete.body)
            .unwrap_or_default();
    let username_removed = username_bans_after_delete_json
        .as_array()
        .is_some_and(|records| {
            !records
                .iter()
                .any(|record| record["key"] == "User:differential-user")
        });
    record!(
        "DELETE",
        "/api/v0/security/bans/username/{username}",
        "mutation-side-effects-and-readback",
        username_unban.status == "200 OK"
            && username_unban.body.is_empty()
            && username_bans_after_delete.status == "200 OK"
            && username_removed
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("openapi_mutation_dtos_tail.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api openapi-mutation-dtos-tail mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the Collections routes' real
/// per-caller ownership enforcement, independently re-derived from
/// `collections_are_scoped_to_the_real_authenticated_caller_identity`'s
/// checks that a collection created by one authenticated identity is
/// invisible to, and immutable by, a different one. Credits
/// `missing-empty-or-conflict-state` (cross-user 404) for the mutating
/// routes already covered by other cases, plus first-time coverage of
/// the single-collection GET and list-collections GET. slskdN-only
/// (confirmed against the frozen registry: slskd has no Collections
/// routes at all).
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
async fn controller_api_differential_collections_ownership_scoping() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{} {} {} [{}]", $target, $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    for target in ["slskdn"] {
        let keys = serde_json::json!({
            "alice": {"key": "alice-key-0123456789", "role": "readwrite", "cidr": ""},
            "bob": {"key": "bob-key-00123456789ab", "role": "readwrite", "cidr": ""},
        });
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKD_API_KEYS_JSON", &keys.to_string()),
        );
        let alice = Some("ApiKey alice-key-0123456789");
        let bob = Some("ApiKey bob-key-00123456789ab");

        let created = super::route_http_request(
            "POST",
            "/api/v0/collections",
            alice,
            r#"{"title":"Ownership Differential"}"#,
            &state,
        )
        .await
        .expect("alice creates a collection");
        let created_json =
            serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
        let collection_id = created_json["id"].as_str().unwrap_or_default().to_owned();
        record!(
            target,
            "POST",
            "/api/v0/collections",
            "mutation-side-effects-and-readback",
            created.status == "201 Created" && created_json["ownerUserId"] == "alice"
        );

        let mut cross_user_pass = true;
        for (method, path, body) in [
            ("GET", format!("/api/v0/collections/{collection_id}"), ""),
            (
                "PUT",
                format!("/api/v0/collections/{collection_id}"),
                r#"{"title":"Hijacked"}"#,
            ),
            (
                "GET",
                format!("/api/v0/collections/{collection_id}/items"),
                "",
            ),
            (
                "POST",
                format!("/api/v0/collections/{collection_id}/items"),
                r#"{"contentId":"track-1"}"#,
            ),
        ] {
            let response = super::route_http_request(method, &path, bob, body, &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            cross_user_pass &= response.status == "404 Not Found";
        }
        record!(
            target,
            "GET",
            "/api/v0/collections/{id}",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );
        record!(
            target,
            "PUT",
            "/api/v0/collections/{id}",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );
        record!(
            target,
            "GET",
            "/api/v0/collections/{id}/items",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );
        record!(
            target,
            "POST",
            "/api/v0/collections/{id}/items",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );

        let alice_get = super::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}"),
            alice,
            "",
            &state,
        )
        .await
        .expect("alice reads her own collection");
        record!(
            target,
            "GET",
            "/api/v0/collections/{id}",
            "nominal-status-headers-body",
            alice_get.status == "200 OK"
        );

        let bob_created = super::route_http_request(
            "POST",
            "/api/v0/collections",
            bob,
            r#"{"title":"Bob Ownership Differential"}"#,
            &state,
        )
        .await
        .expect("bob creates his own collection");
        let bob_created_json =
            serde_json::from_str::<serde_json::Value>(&bob_created.body).unwrap_or_default();
        let bob_owned =
            bob_created.status == "201 Created" && bob_created_json["ownerUserId"] == "bob";

        let alice_list = super::route_http_request("GET", "/api/v0/collections", alice, "", &state)
            .await
            .expect("alice lists collections");
        let alice_titles = serde_json::from_str::<serde_json::Value>(&alice_list.body)
            .unwrap_or_default()
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|record| record["title"].as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        let bob_list = super::route_http_request("GET", "/api/v0/collections", bob, "", &state)
            .await
            .expect("bob lists collections");
        let bob_titles = serde_json::from_str::<serde_json::Value>(&bob_list.body)
            .unwrap_or_default()
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|record| record["title"].as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        record!(
            target,
            "GET",
            "/api/v0/collections",
            "nominal-status-headers-body",
            alice_list.status == "200 OK" && bob_list.status == "200 OK"
        );
        record!(
            target,
            "GET",
            "/api/v0/collections",
            "populated-dynamic-state",
            bob_owned
                && alice_titles == vec!["Ownership Differential".to_owned()]
                && bob_titles == vec!["Bob Ownership Differential".to_owned()]
        );

        let bob_delete = super::route_http_request(
            "DELETE",
            &format!("/api/v0/collections/{collection_id}"),
            bob,
            "",
            &state,
        )
        .await
        .expect("bob attempts to delete alice's collection");
        let still_there = super::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}"),
            alice,
            "",
            &state,
        )
        .await
        .expect("alice's collection still exists");
        record!(
            target,
            "DELETE",
            "/api/v0/collections/{id}",
            "missing-empty-or-conflict-state",
            bob_delete.status == "404 Not Found" && still_there.status == "200 OK"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("collections_ownership_scoping.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "controller-api collections-ownership-scoping mismatches:\n{}",
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the share-grants routes' real
/// transitive ownership enforcement, independently re-derived from
/// `share_grants_are_scoped_to_the_real_collection_owner`'s checks
/// that a grant is owned through its collection (`collection.
/// OwnerUserId == currentUserId`), so every action 404s for a
/// different caller. Uses the `/api/v0/` routes throughout (the
/// original test also exercises a legacy `/api/share-grants/{id}`
/// alias that has no manifest registry entry in either frozen
/// target -- not creditable, matching the documented pattern for
/// slskR-internal aliases). slskdN-only (confirmed against the
/// frozen registry: absent from the slskd policy file).
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
async fn controller_api_differential_share_grants_ownership_scoping() {
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

    let keys = serde_json::json!({
        "alice": {"key": "alice-key-0123456789", "role": "administrator", "cidr": ""},
        "bob": {"key": "bob-key-00123456789ab", "role": "administrator", "cidr": ""},
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKD_API_KEYS_JSON", &keys.to_string()),
    );
    let alice = Some("ApiKey alice-key-0123456789");
    let bob = Some("ApiKey bob-key-00123456789ab");

    let collection = super::route_http_request(
        "POST",
        "/api/v0/collections",
        alice,
        r#"{"title":"Alice Grant Ownership"}"#,
        &state,
    )
    .await
    .expect("alice creates a collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let bob_create = super::route_http_request(
        "POST",
        "/api/v0/share-grants",
        bob,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("bob attempts to grant alice's collection");
    record!(
        "POST",
        "/api/v0/share-grants",
        "missing-empty-or-conflict-state",
        bob_create.status == "404 Not Found"
    );

    let granted = super::route_http_request(
        "POST",
        "/api/v0/share-grants",
        alice,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("alice grants her own collection");
    let grant_id = serde_json::from_str::<serde_json::Value>(&granted.body).unwrap_or_default()
        ["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let mut cross_user_pass = true;
    for (method, path, body) in [
        (
            "GET",
            format!("/api/v0/share-grants/{grant_id}"),
            String::new(),
        ),
        (
            "PUT",
            format!("/api/v0/share-grants/{grant_id}"),
            r#"{"permissions":"read,download"}"#.to_owned(),
        ),
        (
            "GET",
            format!("/api/v0/share-grants/by-collection/{collection_id}"),
            String::new(),
        ),
        (
            "POST",
            format!("/api/v0/share-grants/{grant_id}/token"),
            "{}".to_owned(),
        ),
    ] {
        let response = super::route_http_request(method, &path, bob, &body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        cross_user_pass &= response.status == "404 Not Found";
    }
    record!(
        "GET",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );
    record!(
        "PUT",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "missing-empty-or-conflict-state",
        cross_user_pass
    );

    let bob_list = super::route_http_request("GET", "/api/v0/share-grants", bob, "", &state)
        .await
        .expect("bob lists share grants");
    let bob_list_empty = serde_json::from_str::<serde_json::Value>(&bob_list.body)
        .unwrap_or_default()
        .as_array()
        .map(Vec::len)
        == Some(0);
    record!(
        "GET",
        "/api/v0/share-grants",
        "nominal-status-headers-body",
        bob_list.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/share-grants",
        "populated-dynamic-state",
        bob_list_empty
    );

    let alice_token = super::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{grant_id}/token"),
        alice,
        "{}",
        &state,
    )
    .await
    .expect("alice mints a token for her own grant");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "nominal-status-headers-body",
        alice_token.status == "201 Created"
    );

    let bob_delete = super::route_http_request(
        "DELETE",
        &format!("/api/v0/share-grants/{grant_id}"),
        bob,
        "",
        &state,
    )
    .await
    .expect("bob attempts to delete alice's grant");
    let still_there = super::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{grant_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice's grant still exists");
    record!(
        "DELETE",
        "/api/v0/share-grants/{id}",
        "missing-empty-or-conflict-state",
        bob_delete.status == "404 Not Found" && still_there.status == "200 OK"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("share_grants_ownership_scoping.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api share-grants-ownership-scoping mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 3 Soulfind-bridge HTTP routes'
/// cases, independently re-derived from `bridge_search_and_download_
/// use_real_oracle_shapes`'s real BridgeSearchResult/BridgeDownload
/// contract checks and `bridge_admin_clients_never_leaks_unrelated_
/// peer_activity`'s real client-isolation check. slskdN-only
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
async fn controller_api_differential_bridge_routes() {
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
    add_test_share(
        &state,
        "Virtual/Bridge Differential.flac",
        Path::new("/nonexistent/bridge-differential.flac"),
        4096,
    )
    .await;

    let search = super::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"Bridge Differential"}"#,
        &state,
    )
    .await
    .expect("bridge search");
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap_or_default();
    let users = search_json["users"].as_array().cloned().unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/search",
        "nominal-status-headers-body",
        search.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/bridge/search",
        "mutation-side-effects-and-readback",
        search_json["query"] == "Bridge Differential"
            && users.len() == 1
            && users[0].get("peerId").is_some()
            && users[0].get("username").is_some()
            && users[0]["files"].as_array().map(Vec::len) == Some(1)
            && users[0]["files"][0]["path"] == "Virtual/Bridge Differential.flac"
            && users[0]["files"][0]["sizeBytes"] == 4096
            && users[0]["files"][0]["codec"] == "flac"
    );

    let empty_search = super::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"nothing-matches-this-differential"}"#,
        &state,
    )
    .await
    .expect("bridge search with no matches");
    let empty_search_json =
        serde_json::from_str::<serde_json::Value>(&empty_search.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/search",
        "missing-empty-or-conflict-state",
        empty_search_json["users"] == serde_json::json!([])
    );

    let download = super::route_http_request(
        "POST",
        "/api/v0/bridge/download",
        None,
        r#"{"username":"peer","filename":"Virtual/Bridge Differential.flac","targetPath":"/tmp/out-differential.flac"}"#,
        &state,
    )
    .await
    .expect("bridge download");
    let download_json =
        serde_json::from_str::<serde_json::Value>(&download.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/download",
        "nominal-status-headers-body",
        download.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/bridge/download",
        "mutation-side-effects-and-readback",
        download_json["transfer_id"].is_string()
            && download_json.get("downloadIds").is_none()
            && download_json.get("enqueued").is_none()
    );

    let (isolated_state, _isolated_receiver) = test_state();
    {
        let mut users = isolated_state.users.write().await;
        users.watch("online-peer-differential".to_owned());
        if let Some(record) = users
            .records
            .iter_mut()
            .find(|record| record.username == "online-peer-differential")
        {
            record.status = Some("online".to_owned());
        }
    }
    let clients = super::route_http_request(
        "GET",
        "/api/bridge/admin/clients",
        None,
        "",
        &isolated_state,
    )
    .await
    .expect("bridge clients");
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "nominal-status-headers-body",
        clients.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/admin/clients",
        "missing-empty-or-conflict-state",
        serde_json::from_str::<serde_json::Value>(&clients.body).unwrap_or_default()
            == serde_json::json!({"clients": []})
    );
    let versioned_clients = super::route_http_request(
        "GET",
        "/api/v0/bridge/admin/clients",
        None,
        "",
        &isolated_state,
    )
    .await
    .expect("versioned bridge clients");
    record!(
        "GET",
        "/api/v0/bridge/admin/clients",
        "nominal-status-headers-body",
        versioned_clients.status == "200 OK"
            && versioned_clients
                .content_type
                .starts_with("application/json")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("bridge_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api bridge-routes mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting the unversioned bridge mutation routes'
/// API-version negotiation contract. The frozen controller exposes the
/// compatibility routes but ASP.NET API versioning rejects POST requests
/// without an explicit version before binding the request body. slskdN-only
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
async fn controller_api_differential_unversioned_bridge_version_validation() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} POST {} [{}]",
                    $route, $case
                ));
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

    let (state, _receiver) = test_state();
    let routes = [
        ("/api/bridge/search", r#"{"query":"unversioned bridge"}"#),
        (
            "/api/bridge/download",
            r#"{"username":"peer","filename":"file.flac","targetPath":"/tmp/file.flac"}"#,
        ),
        ("/api/bridge/start", "{}"),
        ("/api/bridge/stop", "{}"),
    ];
    for (route, nominal_body) in routes {
        let nominal = super::route_http_request("POST", route, None, nominal_body, &state)
            .await
            .unwrap_or_else(|error| panic!("POST {route}: {error}"));
        record!(
            route,
            "nominal-status-headers-body",
            nominal.status == "400 Bad Request" && nominal.body.contains("ApiVersionUnspecified")
        );

        let malformed = super::route_http_request("POST", route, None, "not-json", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {route} malformed: {error}"));
        record!(
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("ApiVersionUnspecified")
        );

        let empty = super::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {route} empty: {error}"));
        record!(
            route,
            "missing-empty-or-conflict-state",
            empty.status == "400 Bad Request" && empty.body.contains("ApiVersionUnspecified")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("unversioned_bridge_version_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api unversioned bridge version mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting the remaining unversioned mutating
/// controller routes' API-version negotiation contract. These routes are
/// exposed for compatibility, but the frozen API-versioning middleware
/// rejects them before request binding when no version is supplied.
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
async fn controller_api_differential_unversioned_mutation_version_validation() {
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
    let routes = [
        (
            "POST",
            "/api/audio/analyzers/migrate",
            "/api/audio/analyzers/migrate",
            "{}",
        ),
        ("POST", "/api/jobs/mb-release", "/api/jobs/mb-release", "{}"),
        (
            "POST",
            "/api/jobs/discography",
            "/api/jobs/discography",
            "{}",
        ),
        (
            "POST",
            "/api/jobs/label-crate",
            "/api/jobs/label-crate",
            "{}",
        ),
        (
            "POST",
            "/api/library/health/scans",
            "/api/library/health/scans",
            "{}",
        ),
        (
            "POST",
            "/api/library/health/issues/fix",
            "/api/library/health/issues/fix",
            "{}",
        ),
        (
            "POST",
            "/api/source-feed-imports/preview",
            "/api/source-feed-imports/preview",
            "{}",
        ),
        (
            "POST",
            "/api/integrations/spotify/authorize",
            "/api/integrations/spotify/authorize",
            "{}",
        ),
        (
            "DELETE",
            "/api/integrations/spotify",
            "/api/integrations/spotify",
            "",
        ),
        (
            "PUT",
            "/api/bridge/admin/config",
            "/api/bridge/admin/config",
            "{}",
        ),
        (
            "PATCH",
            "/api/library/health/issues/issue-id",
            "/api/library/health/issues/{issueId}",
            "{}",
        ),
    ];
    for (method, path, route, nominal_body) in routes {
        let nominal = super::route_http_request(method, path, None, nominal_body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "nominal-status-headers-body",
            nominal.status == "400 Bad Request" && nominal.body.contains("ApiVersionUnspecified")
        );

        let malformed = super::route_http_request(method, path, None, "not-json", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} malformed: {error}"));
        record!(
            method,
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("ApiVersionUnspecified")
        );

        let empty = super::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} empty: {error}"));
        record!(
            method,
            route,
            "missing-empty-or-conflict-state",
            empty.status == "400 Bad Request" && empty.body.contains("ApiVersionUnspecified")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("unversioned_mutation_version_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api unversioned mutation version mismatches:\n{}",
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
async fn controller_api_differential_songid_run_lifecycle() {
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
        let response = super::route_http_request("POST", "/api/v0/songid/runs", None, body, &state)
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

    let before = super::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
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

    let created = super::route_http_request(
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

    let after = super::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
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
    let polled = super::route_http_request("GET", &polled_route, None, "", &state)
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

    let queue = super::route_http_request("GET", "/api/v0/songid/runs/queue", None, "", &state)
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
    let package = super::route_http_request("GET", &package_route, None, "", &state)
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
async fn controller_api_differential_listening_party_and_transports_status() {
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
    let transports = super::route_http_request(
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
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
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
            super::pods::PodChannel {
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
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
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
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create outsider channel");

    let forbidden_get = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{outside_pod}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forbidden get");
    let forbidden_post = super::route_http_request(
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

    let invalid_action = super::route_http_request(
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

    let before = super::route_http_request(
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

    let played = super::route_http_request(
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

    let polled = super::route_http_request(
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

    let stopped = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop event");
    let after_stop = super::route_http_request(
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
async fn controller_api_differential_listening_party_open_cases() {
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
    let pod_id = "pod:listening-party-open-cases";
    let channel_id = "general";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Open Cases",
            }))
            .expect("deserialize listening-party pod fixture"),
            "tester".to_owned(),
        )
        .expect("create listening-party pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create listening-party channel");

    let malformed_directory =
        super::route_http_request("GET", "/api/v0/listening-party/extra", None, "", &state)
            .await
            .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party",
        "malformed-path-query-or-body",
        malformed_directory.status == "404 Not Found"
    );

    let empty_directory =
        super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .unwrap();
    let empty_directory_json =
        serde_json::from_str::<serde_json::Value>(&empty_directory.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/listening-party",
        "missing-empty-or-conflict-state",
        empty_directory.status == "200 OK" && empty_directory_json == serde_json::json!([])
    );

    let malformed_state = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "malformed-path-query-or-body",
        malformed_state.status == "404 Not Found"
    );

    let empty_state = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "runtime-failure-and-timeout",
        empty_state.status == "204 No Content"
    );

    let played = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"open-cases-content","title":"Open Cases","listed":true,"allowMeshStreaming":true}"#,
        &state,
    )
    .await
    .unwrap();
    let played_json = serde_json::from_str::<serde_json::Value>(&played.body).unwrap_or_default();
    let party_id = played_json["partyId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let content_id = played_json["contentId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let ticket = super::issue_listening_party_stream_ticket(&state, &party_id, &content_id)
        .await
        .expect("issue listening-party ticket");

    let radio = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={ticket}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "nominal-status-headers-body",
        radio.status == "200 OK"
    );

    let radio_json = serde_json::from_str::<serde_json::Value>(&radio.body).unwrap_or_default();
    state
        .content_discovery
        .write()
        .await
        .merge_shadow_records(vec![super::content_discovery::ShadowIndexRecord {
            recording_id: content_id.clone(),
            peer_ids: vec!["party-peer".to_owned()],
            updated_at: 0,
        }])
        .expect("seed listening-party content");
    let populated_radio = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={ticket}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let populated_radio_json =
        serde_json::from_str::<serde_json::Value>(&populated_radio.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "populated-dynamic-state",
        radio_json["available"] == false
            && populated_radio.status == "200 OK"
            && populated_radio_json["available"] == true
            && populated_radio_json["peerIds"] == serde_json::json!(["party-peer"])
    );

    let malformed_radio = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "malformed-path-query-or-body",
        malformed_radio.status == "404 Not Found"
    );

    let runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("listening-party runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    let runtime_directory =
        super::route_http_request("GET", "/api/v0/listening-party", None, "", &runtime_state)
            .await
            .unwrap();
    let runtime_directory_json =
        serde_json::from_str::<serde_json::Value>(&runtime_directory.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/listening-party",
        "runtime-failure-and-timeout",
        runtime_directory.status == "200 OK" && runtime_directory_json.is_array()
    );

    let runtime_get = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &runtime_state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "runtime-failure-and-timeout",
        runtime_get.status == "204 No Content"
    );

    let radio_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("listening-party radio runtime database");
    let (radio_runtime_state, _radio_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(radio_runtime_db.clone()),
    );
    radio_runtime_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Radio Runtime",
            }))
            .expect("deserialize radio runtime pod fixture"),
            "tester".to_owned(),
        )
        .expect("create radio runtime pod");
    radio_runtime_state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create radio runtime channel");
    let radio_runtime_play = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"radio-runtime-content","listed":true,"allowMeshStreaming":true}"#,
        &radio_runtime_state,
    )
    .await
    .unwrap();
    let radio_runtime_json =
        serde_json::from_str::<serde_json::Value>(&radio_runtime_play.body).unwrap_or_default();
    let radio_runtime_party = radio_runtime_json["partyId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let radio_runtime_content = radio_runtime_json["contentId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let radio_runtime_ticket = super::issue_listening_party_stream_ticket(
        &radio_runtime_state,
        &radio_runtime_party,
        &radio_runtime_content,
    )
    .await
    .expect("issue radio runtime ticket");
    radio_runtime_db.close_for_test().await;
    let radio_runtime = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/listening-party/radio/{radio_runtime_party}/{radio_runtime_content}?ticket={radio_runtime_ticket}"
        ),
        None,
        "",
        &radio_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "runtime-failure-and-timeout",
        radio_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&radio_runtime.body).is_ok()
    );

    let post_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("listening-party post runtime database");
    let (post_runtime_state, _post_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(post_runtime_db.clone()),
    );
    post_runtime_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Runtime",
            }))
            .expect("deserialize runtime pod fixture"),
            "tester".to_owned(),
        )
        .expect("create runtime pod");
    post_runtime_state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            super::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create runtime channel");
    post_runtime_db.close_for_test().await;
    let post_runtime = super::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"runtime-post-content"}"#,
        &post_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "runtime-failure-and-timeout",
        post_runtime.status == "200 OK"
    );

    let (restarted_state, _restarted_receiver) = test_state();
    let reset = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "restart-persistence-or-reset",
        reset.status == "204 No Content"
    );

    let concurrent_bodies: Vec<String> = (0..4)
        .map(|index| {
            format!(r#"{{"action":"play","contentId":"concurrent-party-content-{index}"}}"#)
        })
        .collect();
    let concurrent = futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
        let path = format!("/api/v0/listening-party/{pod_id}/{channel_id}");
        let body = body.clone();
        let state = Arc::clone(&state);
        async move { super::route_http_request("POST", &path, None, &body, &state).await }
    }))
    .await;
    let final_state = super::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && final_state.status == "200 OK"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("listening_party_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api listening-party open-case mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the ActivityPub music-actor and
/// WebFinger discovery routes' cases, independently re-derived from
/// `activitypub_music_actor_and_webfinger_match_target_discovery_
/// contract`'s real actor/webfinger response-shape and content-type
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
async fn controller_api_differential_activitypub_actor_and_webfinger() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "differential.example")
            .with("FEDERATION_BASE_URL", "https://differential.example/")
            .with("FEDERATION_PAGE_SIZE", "10"),
    );

    let actor = super::route_http_request("GET", "/actors/music", None, "", &state)
        .await
        .expect("music actor response");
    let actor_json = serde_json::from_str::<serde_json::Value>(&actor.body).unwrap_or_default();
    record!(
        "/actors/{actorName}",
        "nominal-status-headers-body",
        actor.status == "200 OK" && actor.content_type == "application/activity+json"
    );
    record!(
        "/actors/{actorName}",
        "populated-dynamic-state",
        actor_json["id"] == "https://differential.example/actors/music"
            && actor_json["type"] == "Service"
            && actor_json["preferredUsername"] == "music"
            && actor_json["name"] == "Music Library"
            && actor_json["publicKey"]["publicKeyPem"]
                .as_str()
                .is_some_and(|key| key.starts_with("-----BEGIN PUBLIC KEY-----"))
    );

    let generic = super::route_http_request("GET", "/actors/books", None, "", &state)
        .await
        .expect("generic actor response");
    record!(
        "/actors/{actorName}",
        "missing-empty-or-conflict-state",
        generic.status == "404 Not Found"
    );

    let acct = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40differential.example",
        None,
        "",
        &state,
    )
    .await
    .expect("acct WebFinger response");
    let acct_json = serde_json::from_str::<serde_json::Value>(&acct.body).unwrap_or_default();
    record!(
        "/.well-known/webfinger",
        "nominal-status-headers-body",
        acct.status == "200 OK" && acct.content_type == "application/jrd+json"
    );
    record!(
        "/.well-known/webfinger",
        "populated-dynamic-state",
        acct_json["subject"] == "acct:music@differential.example"
            && acct_json["links"].as_array().map(Vec::len) == Some(2)
    );

    let https_resource = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=https%3A%2F%2Fdifferential.example%2F%40music",
        None,
        "",
        &state,
    )
    .await
    .expect("https WebFinger response");
    let https_json =
        serde_json::from_str::<serde_json::Value>(&https_resource.body).unwrap_or_default();
    record!(
        "/.well-known/webfinger",
        "mutation-side-effects-and-readback",
        https_resource.status == "200 OK"
            && https_json["subject"] == "https://differential.example/@music"
            && https_json["links"].as_array().map(Vec::len) == Some(2)
    );

    let filtered = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40differential.example&rel=self",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered WebFinger response");
    let filtered_json =
        serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap_or_default();
    record!(
        "/.well-known/webfinger",
        "malformed-path-query-or-body",
        filtered.status == "200 OK" && filtered_json["links"].as_array().map(Vec::len) == Some(1)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("activitypub_actor_and_webfinger.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api activitypub-actor-webfinger mismatches:\n{}",
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
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
async fn controller_api_differential_activitypub_collection_empty_and_missing_gets() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "collections.example")
            .with("FEDERATION_BASE_URL", "https://collections.example/")
            .with("FEDERATION_PAGE_SIZE", "10"),
    );

    for (path, route, collection) in [
        ("/actors/music/inbox", "/actors/{actorName}/inbox", "inbox"),
        (
            "/actors/music/outbox",
            "/actors/{actorName}/outbox",
            "outbox",
        ),
        (
            "/actors/music/followers",
            "/actors/{actorName}/followers",
            "followers",
        ),
        (
            "/actors/music/following",
            "/actors/{actorName}/following",
            "following",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/activity+json"
                && response_json["type"] == "OrderedCollection"
                && response_json["totalItems"].as_u64().is_some()
                && response_json["orderedItems"].is_array()
                && (collection == "outbox"
                    || (response_json["totalItems"] == 0
                        && response_json["orderedItems"] == serde_json::json!([])))
                && response_json["id"]
                    .as_str()
                    .is_some_and(|id| id.ends_with(&format!("/{collection}")))
        );
    }

    let malformed_outbox = super::route_http_request(
        "GET",
        "/actors/music/outbox?page=not-an-integer",
        None,
        "",
        &state,
    )
    .await
    .expect("reject malformed outbox page");
    record!(
        "/actors/{actorName}/outbox",
        "malformed-path-query-or-body",
        malformed_outbox.status == "400 Bad Request"
    );

    {
        let mut features = state.controller_features.write_for_test().await;
        for collection in ["inbox", "outbox"] {
            features
                .upsert(
                    format!("activitypub/music/{collection}/collection-seed"),
                    serde_json::json!({
                        "activity": {
                            "id": format!("https://collections.example/activities/{collection}-seed"),
                            "type": "Create",
                            "actor": "https://collections.example/actors/music",
                        }
                    }),
                )
                .expect("seed ActivityPub collection activity");
        }
    }
    for (path, route) in [
        ("/actors/music/inbox", "/actors/{actorName}/inbox"),
        ("/actors/music/outbox", "/actors/{actorName}/outbox"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && response_json["totalItems"]
                    .as_u64()
                    .is_some_and(|count| count >= 1)
                && response_json["orderedItems"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
        );
    }

    for (path, route) in [
        ("/actors/unknown/inbox", "/actors/{actorName}/inbox"),
        ("/actors/unknown/outbox", "/actors/{actorName}/outbox"),
        ("/actors/unknown/following", "/actors/{actorName}/following"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    let unknown_webfinger = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Aunknown%40collections.example",
        None,
        "",
        &state,
    )
    .await
    .expect("unknown WebFinger resource");
    record!(
        "/.well-known/webfinger",
        "missing-empty-or-conflict-state",
        unknown_webfinger.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("activitypub_collection_empty_and_missing_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api activitypub collection mismatches:\n{}",
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
fn controller_api_differential_activitypub_open_cases() {
    std::thread::Builder::new()
        .name("activitypub-open-cases-test".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Runtime::new()
                .expect("create ActivityPub open-case runtime")
                .block_on(controller_api_differential_activitypub_open_cases_impl())
        })
        .expect("spawn ActivityPub open-case test")
        .join()
        .expect("join ActivityPub open-case test");
}

async fn controller_api_differential_activitypub_open_cases_impl() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    let fixture = ActivityPubSignatureFixture::spawn().await;
    let runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("ActivityPub runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "social.example")
            .with("FEDERATION_BASE_URL", "https://social.example/")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;

    for (path, route) in [
        ("/actors/music/extra", "/actors/{actorName}"),
        (
            "/actors/music/followers/extra",
            "/actors/{actorName}/followers",
        ),
        (
            "/actors/music/following/extra",
            "/actors/{actorName}/following",
        ),
        ("/actors/music/inbox/extra", "/actors/{actorName}/inbox"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("malformed ActivityPub path {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    for (path, route) in [
        ("/actors/music", "/actors/{actorName}"),
        ("/actors/music/followers", "/actors/{actorName}/followers"),
        ("/actors/music/following", "/actors/{actorName}/following"),
        ("/actors/music/inbox", "/actors/{actorName}/inbox"),
        ("/actors/music/outbox", "/actors/{actorName}/outbox"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("runtime ActivityPub path {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
        );
    }

    let created = super::unix_timestamp();
    let inbox_signature = fixture.sign(created);
    let inbox_headers = fixture.headers(&inbox_signature, created);
    let inbox_runtime = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &runtime_state,
        inbox_headers,
    )
    .await
    .expect("runtime ActivityPub inbox");
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "runtime-failure-and-timeout",
        inbox_runtime.status == "202 Accepted"
    );

    let (restart_state, _restart_receiver) = fixture.state();
    let restart_created = super::unix_timestamp();
    let restart_signature = fixture.sign(restart_created);
    let inbox_restart = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &restart_state,
        fixture.headers(&restart_signature, restart_created),
    )
    .await
    .expect("restart ActivityPub inbox");
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "restart-persistence-or-reset",
        inbox_restart.status == "202 Accepted"
    );

    let malformed_outbox = super::route_http_request(
        "POST",
        "/actors/music/outbox/extra",
        None,
        "{}",
        &runtime_state,
    )
    .await
    .expect("malformed ActivityPub outbox");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "malformed-path-query-or-body",
        malformed_outbox.status == "404 Not Found"
    );
    let missing_outbox =
        super::route_http_request("POST", "/actors/music/outbox", None, "{}", &runtime_state)
            .await
            .expect("missing ActivityPub outbox body");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "missing-empty-or-conflict-state",
        missing_outbox.status == "400 Bad Request"
    );

    let outbox_body = serde_json::json!({
        "id": "https://social.example/activities/open-outbox",
        "type": "Create",
        "actor": "https://social.example/actors/music",
        "object": {"type": "Note", "content": "runtime"},
    })
    .to_string();
    let outbox_runtime = super::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        &outbox_body,
        &runtime_state,
    )
    .await
    .expect("runtime ActivityPub outbox");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "runtime-failure-and-timeout",
        outbox_runtime.status == "200 OK"
    );

    let (outbox_restart_state, _outbox_restart_receiver) = fixture.state();
    let outbox_restart = super::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        &outbox_body,
        &outbox_restart_state,
    )
    .await
    .expect("restart ActivityPub outbox");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "restart-persistence-or-reset",
        outbox_restart.status == "200 OK"
    );

    let concurrent_bodies = [
        serde_json::json!({
            "id": "https://social.example/activities/open-concurrent-one",
            "type": "Create",
            "actor": "https://social.example/actors/music",
        })
        .to_string(),
        serde_json::json!({
            "id": "https://social.example/activities/open-concurrent-two",
            "type": "Create",
            "actor": "https://social.example/actors/music",
        })
        .to_string(),
    ];
    let concurrent_outbox =
        futures_util::future::join_all(concurrent_bodies.into_iter().map(|body| {
            let state = Arc::clone(&runtime_state);
            async move {
                super::route_http_request("POST", "/actors/music/outbox", None, &body, &state).await
            }
        }))
        .await;
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "concurrency-and-idempotency",
        concurrent_outbox.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create ActivityPub open-case evidence directory");
    fs::write(
        evidence_dir.join("activitypub_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize ActivityPub open-case ledger"),
    )
    .expect("write ActivityPub open-case evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api ActivityPub mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

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
async fn controller_api_differential_hashdb_paging() {
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
            super::content_discovery::HashDbEntry {
                flac_key: hash_a.clone(),
                byte_hash: hash_a,
                size: 100,
                ..Default::default()
            },
            super::content_discovery::HashDbEntry {
                flac_key: hash_b.clone(),
                byte_hash: hash_b,
                size: 200,
                ..Default::default()
            },
        ])
        .expect("seed hashdb sequence");

    let first =
        super::route_http_request("GET", "/api/v0/hashdb/entries?limit=1", None, "", &state)
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

    let second = super::route_http_request(
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

    let sync = super::route_http_request(
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
async fn controller_api_differential_hashdb_validation_and_empty_contracts() {
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
        let response = super::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "malformed-path-query-or-body",
            response.status == expected
        );
    }

    let backfill_candidates = super::route_http_request(
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

    let entries = super::route_http_request("GET", "/api/v0/hashdb/entries", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/hashdb/hash/missing-key", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/hashdb/hash/by-size/0", None, "", &state)
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

    let inventory_by_size = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/hashdb/inventory/unhashed", None, "", &state)
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

    let key = super::route_http_request("GET", "/api/v0/hashdb/key", None, "", &state)
        .await
        .expect("missing hash key inputs");
    record!(
        "GET",
        "/api/v0/hashdb/key",
        "missing-empty-or-conflict-state",
        key.status == "400 Bad Request"
    );

    let analyze =
        super::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
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
    let slow_queries = super::route_http_request(
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
        let response = super::route_http_request("GET", path, None, "", &state)
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

    let stats = super::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
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

    let sync = super::route_http_request("GET", "/api/v0/hashdb/sync/since/0", None, "", &state)
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

    let backfill = super::route_http_request(
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

    let profile = super::route_http_request(
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
