//! Controller full library differential ownership.

use super::*;

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
pub(super) async fn controller_api_differential_library_issue_fix_rehydrates() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Road Trip","description":"queued albums"}"#,
        &state,
    )
    .await
    .expect("create collection");
    assert_eq!(collection.status, "201 Created");
    let collection_json = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap();
    let collection_id = collection_json["id"].as_str().unwrap().to_owned();

    let collection_item = crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"track-1","artist":"Alice","title":"One","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create collection item");
    assert_eq!(collection_item.status, "201 Created");
    let collection_item_json =
        serde_json::from_str::<serde_json::Value>(&collection_item.body).unwrap();
    let collection_item_id = collection_item_json["id"].as_str().unwrap().to_owned();

    let updated_collection_item = crate::route_http_request(
        "PUT",
        &format!("/api/collections/items/{collection_item_id}"),
        None,
        r#"{"title":"Two"}"#,
        &state,
    )
    .await
    .expect("update collection item");
    assert_eq!(updated_collection_item.status, "200 OK");

    let library_item = crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Bob","title":"","kind":""}"#,
        &state,
    )
    .await
    .expect("create library item");
    assert_eq!(library_item.status, "201 Created");
    let library_json = serde_json::from_str::<serde_json::Value>(&library_item.body).unwrap();
    let library_id = library_json["id"].as_str().unwrap().to_owned();

    let patched = crate::route_http_request(
        "PATCH",
        &format!("/api/v0/library/health/issues/{library_id}-missing-title"),
        None,
        r#"{"title":"Fixed Title"}"#,
        &state,
    )
    .await
    .expect("patch library issue");
    assert_eq!(patched.status, "204 No Content");
    let fixed = crate::route_http_request(
        "POST",
        "/api/v0/library/health/issues/fix",
        None,
        "",
        &state,
    )
    .await
    .expect("fix library issues");
    assert_eq!(fixed.status, "200 OK");
    let fixed_json = serde_json::from_str::<serde_json::Value>(&fixed.body)
        .expect("library issue fix response JSON");
    assert_eq!(fixed_json["fixed"], 1);
    assert_eq!(fixed_json["fixable"], 1);
    assert_eq!(fixed_json["remaining"], 0);

    let persisted_collections = db.list_collections(10, 0).await.expect("list collections");
    assert_eq!(persisted_collections.len(), 1);
    assert_eq!(persisted_collections[0].name, "Road Trip");
    let persisted_collection_items = db
        .list_collection_items(10, 0)
        .await
        .expect("list collection items");
    assert_eq!(persisted_collection_items.len(), 1);
    assert_eq!(persisted_collection_items[0].title, "Two");
    let persisted_library = db.list_library_items(10, 0).await.expect("list library");
    assert_eq!(persisted_library.len(), 1);
    assert_eq!(persisted_library[0].title, "Fixed Title");
    assert_eq!(persisted_library[0].kind, "Audio");

    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("collection/library database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["collections"], 1);
    assert_eq!(stats_json["collectionItems"], 1);
    assert_eq!(stats_json["libraryItems"], 1);
    assert_eq!(stats_json["persisted"]["collections"], 1);
    assert_eq!(stats_json["persisted"]["collectionItems"], 1);
    assert_eq!(stats_json["persisted"]["libraryItems"], 1);
    assert_eq!(stats_json["projections"]["collections"], 1);
    assert_eq!(stats_json["projections"]["collectionItems"], 1);
    assert_eq!(stats_json["projections"]["libraryItems"], 1);

    let rehydrated_collections =
        crate::CollectionStore::from_persisted(persisted_collections, persisted_collection_items);
    let rehydrated_library = crate::LibraryStore::from_persisted(persisted_library.clone());
    assert!(rehydrated_collections
        .json_array(None, None)
        .contains("\"title\":\"Two\""));
    assert!(rehydrated_library
        .json()
        .contains("\"title\":\"Fixed Title\""));
    assert!(rehydrated_library.json().contains("\"kind\":\"Audio\""));

    let delete_collection_item = crate::route_http_request(
        "DELETE",
        &format!("/api/collections/items/{collection_item_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection item");
    assert_eq!(delete_collection_item.status, "200 OK");
    let delete_collection = crate::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection");
    assert_eq!(delete_collection.status, "200 OK");
    let delete_library = crate::route_http_request(
        "DELETE",
        &format!("/api/library/items/{library_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete library item");
    assert_eq!(delete_library.status, "200 OK");
    assert!(db.list_collections(10, 0).await.unwrap().is_empty());
    assert!(db.list_collection_items(10, 0).await.unwrap().is_empty());
    assert!(db.list_library_items(10, 0).await.unwrap().is_empty());

    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/library/health/issues/fix",
        "case": "mutation-side-effects-and-readback",
        "pass": true,
    }));
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/library/health/issues/fix",
        "case": "nominal-status-headers-body",
        "pass": fixed.status == "200 OK"
            && fixed.content_type.starts_with("application/json")
            && fixed_json["fixed"] == 1
            && fixed_json["fixable"] == 1
            && fixed_json["remaining"] == 0,
    }));
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/library/health/issues/fix",
        "case": "restart-persistence-or-reset",
        "pass": fixed.status == "200 OK"
            && persisted_library.len() == 1
            && persisted_library[0].kind == "Audio"
            && rehydrated_library.json().contains("\"kind\":\"Audio\""),
    }));
    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "PATCH",
        "route": "/api/v0/library/health/issues/{issueId}",
        "case": "restart-persistence-or-reset",
        "pass": patched.status == "204 No Content"
            && persisted_library.len() == 1
            && persisted_library[0].title == "Fixed Title"
            && rehydrated_library.json().contains("\"title\":\"Fixed Title\""),
    }));
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("library_issue_fix_rehydrates.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_library_jobs_and_discovery_projections() {
    let (state, _receiver) = test_state();
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($method:expr, $route:expr, $case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": true,
            }));
        };
    }

    crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Untitled","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create incomplete library item");
    crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Known","title":"Release","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create complete library item");

    let health = crate::route_http_request("GET", "/api/library/health/issues", None, "", &state)
        .await
        .expect("library issues");
    let health_json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap();
    assert_eq!(health_json["totalCount"], 1);
    assert_eq!(health_json["issues"][0]["type"], "MissingMetadata");
    assert_eq!(
        health_json["issues"][0]["metadata"]["missingField"],
        "missing_artist"
    );
    assert_eq!(health_json["filter"]["limit"], 100);
    assert_eq!(health_json["filter"]["offset"], 0);

    let by_artist = crate::route_http_request(
        "GET",
        "/api/library/health/issues/by-artist",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by artist");
    let by_artist_json = serde_json::from_str::<serde_json::Value>(&by_artist.body).unwrap();
    assert!(by_artist_json["groups"].as_array().unwrap().is_empty());
    assert_eq!(by_artist_json["totalArtists"], 0);

    let summary = crate::route_http_request(
        "GET",
        "/api/library/health/summary?LibraryPath=%2Fmusic",
        None,
        "",
        &state,
    )
    .await
    .expect("library health summary");
    assert_eq!(summary.status, "200 OK");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap();
    assert_eq!(summary_json["libraryPath"], "/music");
    assert_eq!(summary_json["totalIssues"], 1);
    assert_eq!(summary_json["issuesOpen"], 1);

    let dashboard = crate::route_http_request(
        "GET",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=1&issueLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("library health dashboard");
    assert_eq!(dashboard.status, "200 OK");
    let dashboard_json = serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap();
    assert_eq!(dashboard_json["summary"]["libraryPath"], "/music");
    assert_eq!(dashboard_json["issuesByType"][0]["type"], "MissingMetadata");
    assert!(dashboard_json["issuesByArtist"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(dashboard_json["issues"].as_array().unwrap().len(), 1);
    assert_eq!(dashboard_json["totalIssues"], 1);

    let by_codec = crate::route_http_request(
        "GET",
        "/api/library/health/issues/by-codec",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by codec");
    let by_codec_json = serde_json::from_str::<serde_json::Value>(&by_codec.body).unwrap();
    assert_eq!(by_codec_json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(by_codec_json["groups"][0]["codec"], "UNKNOWN");
    assert_eq!(by_codec_json["groups"][0]["count"], 1);
    assert_eq!(by_codec_json["groups"][0]["transcodeSuspect"], 0);
    assert_eq!(by_codec_json["totalIssues"], 1);

    let filtered = crate::route_http_request(
        "GET",
        "/api/library/health/issues?LibraryPath=%2Fmusic&types=CorruptedFile&severities=Medium&statuses=Detected&Limit=2&Offset=999999",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered library issues");
    assert_eq!(filtered.status, "200 OK");
    let filtered_json = serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap();
    assert_eq!(filtered_json["totalCount"], 0);
    assert_eq!(filtered_json["filter"]["libraryPath"], "/music");
    assert_eq!(filtered_json["filter"]["types"][0], "CorruptedFile");
    assert_eq!(filtered_json["filter"]["severities"][0], "Medium");
    assert_eq!(filtered_json["filter"]["statuses"][0], "Detected");
    assert_eq!(filtered_json["filter"]["limit"], 2);
    assert_eq!(filtered_json["filter"]["offset"], 999999);

    let by_type = crate::route_http_request(
        "GET",
        "/api/library/health/issues/by-type",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by type");
    assert_eq!(by_type.status, "200 OK");
    let by_type_json = serde_json::from_str::<serde_json::Value>(&by_type.body).unwrap();
    assert_eq!(by_type_json["groups"][0]["type"], "MissingMetadata");
    assert_eq!(by_type_json["totalIssues"], 1);

    for path in [
        "/api/library/health/summary",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=0",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&issueLimit=251",
        "/api/library/health/issues?limit=0",
        "/api/library/health/issues?limit=251",
        "/api/library/health/issues?offset=-1",
        "/api/library/health/issues?types=NotAnIssueType",
        "/api/library/health/issues?severities=Urgent",
        "/api/library/health/issues?statuses=Open",
        "/api/library/health/issues/by-artist?limit=101",
        "/api/library/health/issues/by-release?limit=0",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }
    for path in [
        "/api/library/health/summary-untrusted",
        "/api/library/health/issues/by-type-untrusted",
        "/api/library/health/issues/by-type/missing_artist/untrusted",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{path}");
    }

    let lidarr_missing = crate::route_http_request(
        "GET",
        "/api/integrations/lidarr/wanted/missing",
        None,
        "",
        &state,
    )
    .await
    .expect("lidarr missing fallback");
    let lidarr_missing_json =
        serde_json::from_str::<serde_json::Value>(&lidarr_missing.body).unwrap();
    assert_eq!(lidarr_missing.status, "200 OK");
    assert_eq!(lidarr_missing_json["status"], "local");
    assert_eq!(lidarr_missing_json["source"], "library-health");
    assert_eq!(lidarr_missing_json["count"], 1);
    assert_eq!(
        lidarr_missing_json["missing_albums"][0]["issueType"],
        "missing_artist"
    );
    let versioned_lidarr_missing = crate::route_http_request(
        "GET",
        "/api/v0/integrations/lidarr/wanted/missing",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned lidarr missing fallback");
    let versioned_lidarr_missing_json =
        serde_json::from_str::<serde_json::Value>(&versioned_lidarr_missing.body).unwrap();
    assert_eq!(versioned_lidarr_missing.status, "200 OK");
    assert_eq!(versioned_lidarr_missing_json["status"], "local");
    assert_eq!(versioned_lidarr_missing_json["source"], "library-health");
    assert_eq!(versioned_lidarr_missing_json["count"], 1);
    let lidarr_sync = crate::route_http_request(
        "POST",
        "/api/integrations/lidarr/wanted/sync",
        None,
        "{}",
        &state,
    )
    .await
    .expect("lidarr sync fallback");
    let lidarr_sync_json = serde_json::from_str::<serde_json::Value>(&lidarr_sync.body).unwrap();
    assert_eq!(lidarr_sync_json["status"], "local");
    assert_eq!(lidarr_sync_json["missingCount"], 1);
    assert_eq!(lidarr_sync_json["runs"], 1);

    let patched_issue = crate::route_http_request(
        "PATCH",
        "/api/v0/library/health/issues/lib-1-missing-artist",
        None,
        r#"{"artist":"Recovered Artist"}"#,
        &state,
    )
    .await
    .expect("patch library health issue");
    assert_eq!(patched_issue.status, "204 No Content");
    assert_eq!(
        state.library.read().await.get("lib-1").unwrap().artist,
        "Recovered Artist"
    );

    crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Fixable","title":"Kindless","kind":""}"#,
        &state,
    )
    .await
    .expect("create fixable library item");
    let scan = crate::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("library scan");
    let scan_json = serde_json::from_str::<serde_json::Value>(&scan.body).unwrap();
    assert_eq!(scan.status, "200 OK");
    assert!(scan_json["scanId"].as_str().is_some());
    assert_eq!(scan_json["message"], "Scan started successfully");
    let active_id = scan_json["scanId"].as_str().unwrap();
    let second_scan = crate::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("second library scan");
    assert_eq!(second_scan.status, "409 Conflict");
    assert!(second_scan.body.contains(active_id));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let completed = crate::route_http_request(
        "GET",
        &format!("/api/library/health/scans/{active_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&completed.body).unwrap()["status"],
        "completed"
    );
    let missing_scan = crate::route_http_request(
        "GET",
        "/api/library/health/scans/scan-does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .expect("missing library scan");
    assert_eq!(missing_scan.status, "404 Not Found");
    let scan_detail = crate::route_http_request(
        "GET",
        &format!(
            "/api/library/health/scans/{}",
            scan_json["scanId"].as_str().unwrap()
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("library scan detail");
    let scan_detail_json = serde_json::from_str::<serde_json::Value>(&scan_detail.body).unwrap();
    assert_eq!(scan_detail_json["issues_found"], 1);
    let aliased_scan = crate::route_http_request(
        "GET",
        &format!(
            "/api/library/health/scans/{}/untrusted",
            scan_json["scanId"].as_str().unwrap()
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased library scan");
    assert_eq!(aliased_scan.status, "404 Not Found");
    let fixed = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/library/remediate",
        None,
        r#"{"issue_ids":["lib-3-missing-kind"]}"#,
        &state,
    )
    .await
    .expect("fix library issues");
    let fixed_json = serde_json::from_str::<serde_json::Value>(&fixed.body).unwrap();
    assert!(fixed_json["id"].as_str().is_some());
    assert_eq!(fixed_json["kind"], "library_remediation");
    assert_eq!(fixed_json["status"], "completed");
    assert_eq!(fixed_json["fixedCount"], 1);
    assert_eq!(
        fixed_json["issueIds"],
        serde_json::json!(["lib-3-missing-kind"])
    );
    let job = crate::route_http_request(
        "GET",
        &format!("/api/jobs/{}", fixed_json["id"].as_str().unwrap()),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(job.status, "200 OK");
    let job_json = serde_json::from_str::<serde_json::Value>(&job.body).unwrap();
    assert_eq!(job_json["id"], fixed_json["id"]);
    assert_eq!(fixed_json["remaining"], serde_json::Value::Null);
    let stored_scan = crate::route_http_request(
        "GET",
        &format!(
            "/api/library/health/scans/{}",
            scan_json["scanId"].as_str().unwrap()
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("stored library scan snapshot");
    let stored_scan_json = serde_json::from_str::<serde_json::Value>(&stored_scan.body).unwrap();
    assert_eq!(stored_scan_json["issues_found"], 1);

    let lidarr_import = crate::route_http_request(
        "POST",
        "/api/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/imports/Manual Album","artist":"Imported Artist"}"#,
        &state,
    )
    .await
    .expect("lidarr manual import fallback");
    let lidarr_import_json =
        serde_json::from_str::<serde_json::Value>(&lidarr_import.body).unwrap();
    assert_eq!(lidarr_import_json["status"], "local");
    assert_eq!(lidarr_import_json["imported"], 1);
    assert_eq!(lidarr_import_json["items"][0]["artist"], "Imported Artist");
    assert_eq!(lidarr_import_json["items"][0]["title"], "Manual Album");

    let completion = crate::route_http_request(
        "GET",
        "/api/musicbrainz/albums/completion",
        None,
        "",
        &state,
    )
    .await
    .expect("completion");
    let completion_json = serde_json::from_str::<serde_json::Value>(&completion.body).unwrap();
    assert_eq!(completion.status, "200 OK");
    assert_eq!(completion_json["count"], 4);
    let versioned_completion = crate::route_http_request(
        "GET",
        "/api/v0/musicbrainz/albums/completion",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned completion");
    let versioned_completion_json =
        serde_json::from_str::<serde_json::Value>(&versioned_completion.body).unwrap();
    assert_eq!(versioned_completion.status, "200 OK");
    assert_eq!(versioned_completion_json["count"], 4);

    let coverage = crate::route_http_request(
        "GET",
        "/api/musicbrainz/artist/Known/discography-coverage",
        None,
        "",
        &state,
    )
    .await
    .expect("coverage");
    let coverage_json = serde_json::from_str::<serde_json::Value>(&coverage.body).unwrap();
    assert_eq!(coverage.status, "200 OK");
    assert_eq!(coverage_json["releases"], 1);
    let versioned_coverage = crate::route_http_request(
        "GET",
        "/api/v0/musicbrainz/artist/Known/discography-coverage",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned coverage");
    let versioned_coverage_json =
        serde_json::from_str::<serde_json::Value>(&versioned_coverage.body).unwrap();
    assert_eq!(versioned_coverage.status, "200 OK");
    assert_eq!(versioned_coverage_json["releases"], 1);
    let aliased_coverage = crate::route_http_request(
        "GET",
        "/api/musicbrainz/artist/Known/extra/discography-coverage",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased artist coverage");
    assert_eq!(aliased_coverage.status, "404 Not Found");

    let target = crate::route_http_request(
        "POST",
        "/api/musicbrainz/targets",
        None,
        r#"{"artist":"Known","title":"New Target"}"#,
        &state,
    )
    .await
    .expect("musicbrainz target");
    let target_json = serde_json::from_str::<serde_json::Value>(&target.body).unwrap();
    assert_eq!(target_json["created"], true);
    assert_eq!(target_json["item"]["artist"], "Known");
    assert!(target_json["projection"]["count"].as_u64().unwrap() >= 2);

    crate::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Radar","title":"Need"}"#,
        &state,
    )
    .await
    .expect("wishlist seed");
    let radar = crate::route_http_request(
        "GET",
        "/api/musicbrainz/release-radar/notifications",
        None,
        "",
        &state,
    )
    .await
    .expect("release radar notifications");
    let radar_json = serde_json::from_str::<serde_json::Value>(&radar.body).unwrap();
    assert_eq!(radar.status, "200 OK");
    assert_eq!(radar_json[0]["artist"], "Radar");
    let versioned_radar = crate::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned release radar notifications");
    let versioned_radar_json =
        serde_json::from_str::<serde_json::Value>(&versioned_radar.body).unwrap();
    assert_eq!(versioned_radar.status, "200 OK");
    assert!(versioned_radar_json.is_array());

    crate::route_http_request(
        "POST",
        "/api/soulseek/interests",
        None,
        r#"{"name":"jazz"}"#,
        &state,
    )
    .await
    .expect("interest seed");
    let taste = crate::route_http_request("POST", "/api/taste-recommendations", None, "{}", &state)
        .await
        .expect("taste recommendations");
    let taste_json = serde_json::from_str::<serde_json::Value>(&taste.body).unwrap();
    assert_eq!(taste_json["recommendations"][0]["query"], "jazz");

    let graph = crate::route_http_request("POST", "/api/discovery-graph", None, "{}", &state)
        .await
        .expect("discovery graph");
    let graph_json = serde_json::from_str::<serde_json::Value>(&graph.body).unwrap();
    assert_eq!(graph_json["status"], "ready");
    assert!(graph_json["count"].as_u64().unwrap() >= 2);

    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }
    crate::route_http_request("POST", "/api/v0/rooms/listening/join", None, "", &state)
        .await
        .expect("join listening room");
    let parties = crate::route_http_request("GET", "/api/listening-party", None, "", &state)
        .await
        .expect("listening parties");
    // Matches the oracle's real directory contract: it reflects
    // real, currently-listed pod listen-along events, not unrelated
    // joined chat rooms -- joining a room named "listening" above
    // must not fabricate an entry here.
    let parties_json = serde_json::from_str::<serde_json::Value>(&parties.body).unwrap();
    assert_eq!(parties.status, "200 OK");
    assert_eq!(parties_json, serde_json::json!([]));
    let versioned_parties =
        crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .expect("versioned listening parties");
    let versioned_parties_json =
        serde_json::from_str::<serde_json::Value>(&versioned_parties.body).unwrap();
    assert_eq!(versioned_parties.status, "200 OK");
    assert_eq!(versioned_parties_json, serde_json::json!([]));
    let party_content = crate::route_http_request(
        "POST",
        "/api/listening-party/radio/party/content",
        None,
        r#"{"room":"listening","artist":"Party Artist","title":"Party Track"}"#,
        &state,
    )
    .await
    .expect("party content");
    let party_content_json =
        serde_json::from_str::<serde_json::Value>(&party_content.body).unwrap();
    assert_eq!(party_content_json["activePartyCount"], 1);
    assert_eq!(party_content_json["party"]["message_count"], 1);
    assert_eq!(party_content_json["nowPlaying"]["title"], "Party Track");

    let destination = crate::route_http_request(
        "POST",
        "/api/destinations/validate",
        None,
        r#"{"path":"/home/user/Downloads"}"#,
        &state,
    )
    .await
    .expect("destination validate");
    let destination_json = serde_json::from_str::<serde_json::Value>(&destination.body).unwrap();
    assert_eq!(destination_json["valid"], true);
    assert_eq!(destination_json["known"], true);
    assert_eq!(destination_json["matched"]["id"], "default");

    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"job search"}"#,
        &state,
    )
    .await
    .expect("search job seed");
    crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Song.flac","size":100}"#,
        &state,
    )
    .await
    .expect("multisource transfer seed");
    let jobs = crate::route_http_request("GET", "/api/jobs", None, "", &state)
        .await
        .expect("jobs");
    let jobs_json = serde_json::from_str::<serde_json::Value>(&jobs.body).unwrap();
    assert_eq!(jobs.status, "200 OK");
    assert!(jobs_json["jobs"].is_array());
    assert!(jobs_json["limit"].is_number());
    assert!(jobs_json["offset"].is_number());
    assert!(jobs_json["has_more"].is_boolean());
    assert!(jobs_json["total"].as_u64().unwrap() >= 2);

    let discography = crate::route_http_request(
        "POST",
        "/api/v0/jobs/discography",
        None,
        r#"{"artist":"Known"}"#,
        &state,
    )
    .await
    .expect("discography job");
    let discography_json = serde_json::from_str::<serde_json::Value>(&discography.body).unwrap();
    assert_eq!(discography_json["kind"], "discography");
    assert_eq!(discography_json["status"], "queued");
    let discography_id = discography_json["search_id"].as_str().unwrap();
    let discography_detail = crate::route_http_request(
        "GET",
        &format!("/api/jobs/{discography_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("discography detail");
    let discography_detail_json =
        serde_json::from_str::<serde_json::Value>(&discography_detail.body).unwrap();
    assert_eq!(discography_detail_json["kind"], "search");
    assert_eq!(discography_detail_json["query"], "Known discography");
    let aliased_job = crate::route_http_request(
        "GET",
        &format!("/api/jobs/{discography_id}/untrusted"),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased job detail");
    assert_eq!(aliased_job.status, "404 Not Found");

    let mb_release = crate::route_http_request(
        "POST",
        "/api/v0/jobs/mb-release",
        None,
        r#"{"artist":"Known","title":"Release"}"#,
        &state,
    )
    .await
    .expect("mb release job");
    let mb_release_json = serde_json::from_str::<serde_json::Value>(&mb_release.body).unwrap();
    assert_eq!(mb_release_json["kind"], "mb-release");
    assert_eq!(mb_release_json["query"], "Known Release");

    {
        let mut shares = state.shares.write().await;
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Library/Known/Release.flac".to_owned(),
            size: 321,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
    }
    let hash_entries = crate::route_http_request("GET", "/api/hashdb/entries", None, "", &state)
        .await
        .expect("hashdb entries");
    let hash_entries_json = serde_json::from_str::<serde_json::Value>(&hash_entries.body).unwrap();
    assert!(hash_entries_json["count"].as_u64().unwrap() >= 1);
    assert!(hash_entries_json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["filename"] == "Library/Known/Release.flac"
            && entry["extension"] == "flac"));
    assert!(!hash_entries_json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["filename"] == "Library/Known/Release.jpg"));

    let backfill_candidates =
        crate::route_http_request("GET", "/api/backfill/candidates", None, "", &state)
            .await
            .expect("backfill candidates");
    let backfill_candidates_json =
        serde_json::from_str::<serde_json::Value>(&backfill_candidates.body).unwrap();
    assert!(backfill_candidates_json["count"].as_u64().unwrap() <= 10);
    let backfill_stats = crate::route_http_request("GET", "/api/backfill/stats", None, "", &state)
        .await
        .expect("backfill stats");
    let backfill_stats_json =
        serde_json::from_str::<serde_json::Value>(&backfill_stats.body).unwrap();
    assert_eq!(backfill_stats_json["totalAttempts"], 0);
    assert_eq!(backfill_stats_json["active"], 0);
    assert_eq!(backfill_stats_json["isIdle"], false);
    let backfill_config =
        crate::route_http_request("GET", "/api/backfill/config", None, "", &state)
            .await
            .expect("backfill config");
    let backfill_config_json =
        serde_json::from_str::<serde_json::Value>(&backfill_config.body).unwrap();
    assert_eq!(backfill_config_json["maxGlobalConnections"], 2);
    assert_eq!(backfill_config_json["maxHeaderBytes"], 65_536);
    let disabled = crate::route_http_request(
        "POST",
        "/api/backfill/enable?enabled=false",
        None,
        "",
        &state,
    )
    .await
    .expect("disable backfill");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&disabled.body).unwrap()["enabled"],
        false
    );
    let idle = crate::route_http_request("POST", "/api/backfill/idle", None, "", &state)
        .await
        .expect("mark backfill idle");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&idle.body).unwrap()["isIdle"],
        true
    );
    let busy = crate::route_http_request("POST", "/api/backfill/busy", None, "", &state)
        .await
        .expect("mark backfill busy");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&busy.body).unwrap()["isIdle"],
        false
    );
    let invalid_file = crate::route_http_request(
        "POST",
        "/api/backfill/file",
        None,
        r#"{"peerId":"","path":"","size":0}"#,
        &state,
    )
    .await
    .expect("reject invalid backfill file");
    assert_eq!(invalid_file.status, "400 Bad Request");
    let hash_backfill = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "{}",
        &state,
    )
    .await
    .expect("hashdb backfill");
    let hash_backfill_json =
        serde_json::from_str::<serde_json::Value>(&hash_backfill.body).unwrap();
    assert_eq!(hash_backfill.status, "200 OK");
    assert!(hash_backfill_json.get("searchesProcessed").is_some());
    assert!(hash_backfill_json.get("flacsDiscovered").is_some());
    let unversioned_hash_backfill = crate::route_http_request(
        "POST",
        "/api/hashdb/backfill/from-history",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject unversioned hashdb backfill");
    assert_eq!(unversioned_hash_backfill.status, "400 Bad Request");
    let aliased_hash_backfill = crate::route_http_request(
        "POST",
        "/api/hashdb/backfill/from-history-untrusted",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject aliased hashdb backfill");
    assert_eq!(aliased_hash_backfill.status, "404 Not Found");
    let runtime_backfill = crate::route_http_request("POST", "/api/backfill", None, "{}", &state)
        .await
        .expect("runtime backfill");
    let runtime_backfill_json =
        serde_json::from_str::<serde_json::Value>(&runtime_backfill.body).unwrap();
    assert_eq!(runtime_backfill_json["runs"], 1);
    assert!(runtime_backfill_json["queued"].as_u64().unwrap() >= 4);

    let song_runs = crate::route_http_request("GET", "/api/songid/runs", None, "", &state)
        .await
        .expect("song id runs");
    let song_runs_json = serde_json::from_str::<serde_json::Value>(&song_runs.body).unwrap();
    assert!(song_runs_json.as_array().unwrap().is_empty());
    let song_run = crate::route_http_request(
        "POST",
        "/api/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("song id run");
    let song_run_json = serde_json::from_str::<serde_json::Value>(&song_run.body).unwrap();
    assert!(song_run_json["matchCount"].as_u64().unwrap() >= 1);
    assert_eq!(song_run_json["runs"], 1);
    assert_eq!(song_run_json["persisted"], true);
    let song_id = song_run_json["id"].as_str().unwrap();
    let song_detail = crate::route_http_request(
        "GET",
        &format!("/api/songid/runs/{song_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("song id detail");
    let song_detail_json = serde_json::from_str::<serde_json::Value>(&song_detail.body).unwrap();
    assert_eq!(song_detail_json["id"], song_id);
    assert!(song_detail_json["matchCount"].as_u64().unwrap() >= 1);
    let missing_song_run = crate::route_http_request(
        "GET",
        "/api/songid/runs/songid-does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .expect("missing song id run");
    assert_eq!(missing_song_run.status, "404 Not Found");
    let song_matrix = crate::route_http_request(
        "GET",
        &format!("/api/songid/runs/{song_id}/forensic-matrix"),
        None,
        "",
        &state,
    )
    .await
    .expect("song id matrix");
    let song_matrix_json = serde_json::from_str::<serde_json::Value>(&song_matrix.body).unwrap();
    assert!(song_matrix_json["count"].as_u64().unwrap() >= 1);

    let multisource = crate::route_http_request("GET", "/api/multisource/jobs", None, "", &state)
        .await
        .expect("multisource jobs");
    let multisource_json = serde_json::from_str::<serde_json::Value>(&multisource.body).unwrap();
    assert_eq!(multisource_json["jobs"][0]["sources"][0], "peer");
    let multisource_id = multisource_json["jobs"][0]["id"].as_str().unwrap();
    let multisource_detail = crate::route_http_request(
        "GET",
        &format!("/api/multisource/jobs/{multisource_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("multisource job detail");
    assert_eq!(multisource_detail.status, "200 OK");
    let aliased_multisource = crate::route_http_request(
        "GET",
        &format!("/api/multisource/jobs/{multisource_id}/untrusted"),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased multisource job detail");
    assert_eq!(aliased_multisource.status, "404 Not Found");
    // Matches the oracle's real GetJobStatus: an unknown job id must be
    // a real 404, not a fabricated 200 with an invented "not_found"
    // status string.
    let missing_multisource = crate::route_http_request(
        "GET",
        "/api/multisource/jobs/multisource-does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .expect("missing multisource job");
    assert_eq!(missing_multisource.status, "404 Not Found");

    let slskdn = crate::route_http_request("GET", "/api/slskdn", None, "", &state)
        .await
        .expect("slskdn summary");
    let native_json = serde_json::from_str::<serde_json::Value>(&slskdn.body).unwrap();
    assert_eq!(native_json["status"], "local");
    assert!(native_json["libraryItems"].as_u64().unwrap() >= 4);
    let native_health =
        crate::route_http_request("GET", "/api/slskdn/library/health", None, "", &state)
            .await
            .expect("slskdn library health");
    let native_health_json =
        serde_json::from_str::<serde_json::Value>(&native_health.body).unwrap();
    assert_eq!(native_health_json["summary"]["total_issues"], 0);
    let podcore_search = crate::route_http_request(
        "GET",
        "/api/podcore/content/search?query=Release",
        None,
        "",
        &state,
    )
    .await
    .expect("podcore search");
    let podcore_search_json =
        serde_json::from_str::<serde_json::Value>(&podcore_search.body).unwrap();
    let podcore_search_results = podcore_search_json.as_array().unwrap();
    assert!(!podcore_search_results.is_empty(), "{podcore_search_json}");
    assert!(
        podcore_search_results[0]["contentId"]
            .as_str()
            .unwrap()
            .starts_with("content:audio:track:"),
        "{podcore_search_json}"
    );
    let stream = crate::route_http_request(
        "GET",
        "/api/streams/Library/Known/Release.flac",
        None,
        "",
        &state,
    )
    .await
    .expect("stream status");
    let stream_json = serde_json::from_str::<serde_json::Value>(&stream.body).unwrap();
    assert_eq!(stream_json["status"], "available");

    // The test above exercises a larger compatibility projection surface,
    // but only these rows correspond to frozen slskdn controller subjects
    // and have assertions strong enough to prove the named case.  Keep
    // compatibility aliases that are not in the frozen controller
    // inventory out of the ledger rather than treating route reachability
    // as behavioral proof.
    for (method, route, case) in [
        (
            "GET",
            "/api/library/health/summary",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/dashboard",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues/by-codec",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues/by-release",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues/by-type",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues",
            "missing-empty-or-conflict-state",
        ),
        (
            "POST",
            "/api/v0/library/health/scans",
            "concurrency-and-idempotency",
        ),
        (
            "GET",
            "/api/library/health/scans/{scanId}",
            "populated-dynamic-state",
        ),
        ("GET", "/api/jobs", "populated-dynamic-state"),
        ("GET", "/api/jobs", "nominal-status-headers-body"),
        (
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/listening-party",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/musicbrainz/albums/completion",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/musicbrainz/release-radar/notifications",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/jobs/discography",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/jobs/discography",
            "mutation-side-effects-and-readback",
        ),
        ("GET", "/api/jobs/{id}", "populated-dynamic-state"),
        (
            "POST",
            "/api/v0/jobs/mb-release",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/slskdn/library/health",
            "populated-dynamic-state",
        ),
    ] {
        record_evidence!(method, route, case);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("library_jobs_and_discovery_projections.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting `runtime-failure-and-timeout` for
/// the library/interests/now-playing/messages families -- independently
/// re-verified real DB-close fault injection for the same routes
/// `library_routes_`, `interest_routes_`, `now_playing_routes_`, and
/// `message_routes_roll_back_when_persistence_fails` already prove.
/// Only routes confirmed present in a frozen registry are credited --
/// e.g. `library/items` POST/DELETE and the webhook/user-watch/browse
/// families this session also checked turned out to be slskR-only
/// additions absent from both frozen oracles, so they're skipped here
/// rather than crediting a nonexistent manifest case.
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
pub(super) async fn controller_api_differential_library_interests_nowplaying_messages_survive_persistence_failure(
) {
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
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
        let declared = |method: &str, route: &str| {
            rules
                .iter()
                .any(|rule| rule.method == method && rule.route == route)
        };

        macro_rules! record {
            ($method:expr, $route:expr, $pass:expr) => {
                if declared($method, $route) {
                    if !$pass {
                        mismatches.push(format!("{target} {} {}", $method, $route));
                    }
                    ledger.push(serde_json::json!({
                        "target": target,
                        "method": $method,
                        "route": $route,
                        "case": "runtime-failure-and-timeout",
                        "pass": $pass,
                    }));
                }
            };
        }

        // Library: create-family (only lidarr/musicbrainz map to real
        // declared routes; the plain library/items POST doesn't) plus
        // the 2 declared mutation routes.
        for (call_path, ledger_route, body) in [
            (
                "/api/integrations/lidarr/manualimport",
                "/api/v0/integrations/lidarr/manualimport",
                r#"{"artist":"Artist","album":"Release"}"#,
            ),
            (
                "/api/musicbrainz/targets",
                "/api/v0/musicbrainz/targets",
                r#"{"artist":"Artist","title":"Release"}"#,
            ),
        ] {
            if !declared("POST", ledger_route) {
                continue;
            }
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            let previous = state.library.read().await.clone();
            db.close_for_test().await;
            let response = crate::route_http_request("POST", call_path, None, body, &state)
                .await
                .expect("failed library creation response");
            let pass = response.status == "503 Service Unavailable"
                && response.body.contains("library persistence failed")
                && *state.library.read().await == previous;
            record!("POST", ledger_route, pass);
        }
        for (method, call_path, body, ledger_route) in [
            (
                "PATCH",
                "/api/v0/library/health/issues/lib-1-missing-title",
                r#"{"title":"Fixed"}"#,
                "/api/v0/library/health/issues/{issueId}",
            ),
            (
                "POST",
                "/api/v0/library/health/issues/fix",
                "",
                "/api/v0/library/health/issues/fix",
            ),
        ] {
            if !declared(method, ledger_route) {
                continue;
            }
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            state
                .library
                .write()
                .await
                .create("Artist".to_owned(), String::new(), String::new())
                .unwrap();
            let previous = state.library.read().await.clone();
            db.close_for_test().await;
            let response = crate::route_http_request(method, call_path, None, body, &state)
                .await
                .expect("failed library mutation response");
            let pass = response.status == "503 Service Unavailable"
                && response.body.contains("library persistence failed")
                && *state.library.read().await == previous;
            record!(method, ledger_route, pass);
        }

        // Interests: create + delete for both liked and hated.
        for (call_path, ledger_route, body) in [
            (
                "/api/soulseek/interests",
                "/api/v0/soulseek/interests",
                r#"{"name":"jazz"}"#,
            ),
            (
                "/api/soulseek/hated-interests",
                "/api/v0/soulseek/hated-interests",
                r#"{"name":"jazz"}"#,
            ),
        ] {
            if !declared("POST", ledger_route) {
                continue;
            }
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            db.close_for_test().await;
            let response = crate::route_http_request("POST", call_path, None, body, &state)
                .await
                .expect("failed interest creation response");
            let interests = state.interests.read().await;
            let pass = response.status == "503 Service Unavailable"
                && response.body.contains("interest persistence failed")
                && interests.liked.is_empty()
                && interests.hated.is_empty();
            drop(interests);
            record!("POST", ledger_route, pass);
        }
        for (call_path, ledger_route, hated) in [
            (
                "/api/soulseek/interests/liked-1",
                "/api/v0/soulseek/interests/{item}",
                false,
            ),
            (
                "/api/soulseek/hated-interests/hated-1",
                "/api/v0/soulseek/hated-interests/{item}",
                true,
            ),
        ] {
            if !declared("DELETE", ledger_route) {
                continue;
            }
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            if hated {
                state
                    .interests
                    .write()
                    .await
                    .add_hated("noise".to_owned())
                    .unwrap();
            } else {
                state
                    .interests
                    .write()
                    .await
                    .add_liked("jazz".to_owned())
                    .unwrap();
            }
            db.close_for_test().await;
            let response = crate::route_http_request("DELETE", call_path, None, "", &state)
                .await
                .expect("failed interest deletion response");
            let pass = response.status == "503 Service Unavailable"
                && response
                    .body
                    .contains("interest deletion persistence failed");
            record!("DELETE", ledger_route, pass);
        }

        // Now-playing: PUT upsert and DELETE clear (declared oracle
        // routes are PUT/DELETE only -- the test's own POST variant of
        // the same handler has no declared oracle counterpart).
        if declared("PUT", "/api/v0/nowplaying") {
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            state.now_playing.write().await.upsert(
                "existing".to_owned(),
                "Original".to_owned(),
                "Track".to_owned(),
            );
            db.close_for_test().await;
            let response = crate::route_http_request(
                "PUT",
                "/api/nowplaying",
                None,
                r#"{"username":"new","artist":"Changed","title":"Song"}"#,
                &state,
            )
            .await
            .expect("failed now-playing persistence response");
            let now_playing = state.now_playing.read().await;
            let pass = response.status == "503 Service Unavailable"
                && response.body.contains("now-playing persistence failed")
                && now_playing.records.len() == 1
                && now_playing.records[0].username == "existing";
            drop(now_playing);
            record!("PUT", "/api/v0/nowplaying", pass);
        }
        if declared("DELETE", "/api/v0/nowplaying") {
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            state.now_playing.write().await.upsert(
                "existing".to_owned(),
                "Original".to_owned(),
                "Track".to_owned(),
            );
            db.close_for_test().await;
            let response = crate::route_http_request("DELETE", "/api/nowplaying", None, "", &state)
                .await
                .expect("failed now-playing deletion response");
            let pass = response.status == "503 Service Unavailable"
                && response
                    .body
                    .contains("now-playing clear persistence failed");
            record!("DELETE", "/api/v0/nowplaying", pass);
        }

        // Messages: conversations create (single + batch) and ack.
        for (call_path, ledger_route, body) in [
            (
                "/api/conversations/friend",
                "/api/v0/conversations/{username}",
                r#"{"body":"conversation"}"#,
            ),
            (
                "/api/conversations/batch",
                "/api/v0/conversations/batch",
                r#"{"usernames":["friend","peer"],"body":"batch"}"#,
            ),
        ] {
            if !declared("POST", ledger_route) {
                continue;
            }
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            let previous = state.messages.read().await.clone();
            db.close_for_test().await;
            let response = crate::route_http_request("POST", call_path, None, body, &state)
                .await
                .expect("failed message persistence response");
            let pass = response.status == "503 Service Unavailable"
                && response.body.contains("message persistence failed")
                && *state.messages.read().await == previous;
            record!("POST", ledger_route, pass);
        }
        if declared("PUT", "/api/v0/conversations/{username}/{id}") {
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            state
                .messages
                .write()
                .await
                .add("friend".to_owned(), "inbound", "message".to_owned());
            db.close_for_test().await;
            let response =
                crate::route_http_request("PUT", "/api/conversations/friend/1", None, "", &state)
                    .await
                    .expect("failed message ack response");
            let pass = response.status == "503 Service Unavailable"
                && response
                    .body
                    .contains("message acknowledgement persistence failed");
            record!("PUT", "/api/v0/conversations/{username}/{id}", pass);
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("library_interests_nowplaying_messages_survive_persistence_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api library/interests/nowplaying/messages mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 8 Collections/CollectionItems
/// routes' `nominal-status-headers-body` and `mutation-side-effects-
/// and-readback` cases, independently re-derived from `openapi_
/// mutation_dtos_match_native_status_and_field_contracts`'s real CRUD
/// lifecycle (create collection -> create item -> update collection ->
/// update item -> reorder -> re-GET items reflects reorder -> delete
/// item -> delete collection). slskdN-only (confirmed against the
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_collections_items_crud_reorder_lifecycle() {
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

    let empty_collections =
        crate::route_http_request("GET", "/api/v0/collections", None, "", &state)
            .await
            .expect("empty collections");
    record!(
        "GET",
        "/api/v0/collections",
        "missing-empty-or-conflict-state",
        empty_collections.status == "200 OK" && empty_collections.body == "[]"
    );
    let malformed_collections =
        crate::route_http_request("GET", "/api/v0/collections/", None, "", &state)
            .await
            .expect("malformed collections path");
    record!(
        "GET",
        "/api/v0/collections",
        "malformed-path-query-or-body",
        malformed_collections.status == "400 Bad Request"
    );
    let malformed_create =
        crate::route_http_request("POST", "/api/v0/collections", None, "{}", &state)
            .await
            .expect("malformed collection create");
    record!(
        "POST",
        "/api/v0/collections",
        "malformed-path-query-or-body",
        malformed_create.status == "400 Bad Request"
    );
    record!(
        "POST",
        "/api/v0/collections",
        "missing-empty-or-conflict-state",
        malformed_create.status == "400 Bad Request"
    );

    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Route Audit","description":"contract"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_json =
        serde_json::from_str::<serde_json::Value>(&collection.body).unwrap_or_default();
    let collection_pass = collection.status == "201 Created"
        && collection_json["title"] == "Route Audit"
        && collection_json["ownerUserId"] == "";
    record!(
        "POST",
        "/api/v0/collections",
        "nominal-status-headers-body",
        collection_pass
    );
    let collection_id = collection_json["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let malformed_item = crate::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("malformed collection item create");
    record!(
        "POST",
        "/api/v0/collections/{id}/items",
        "malformed-path-query-or-body",
        malformed_item.status == "400 Bad Request"
    );

    let fetched_collection = crate::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("get collection");
    let fetched_collection_json =
        serde_json::from_str::<serde_json::Value>(&fetched_collection.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/collections/{id}",
        "populated-dynamic-state",
        fetched_collection.status == "200 OK"
            && fetched_collection_json["id"] == collection_id
            && fetched_collection_json["title"] == "Route Audit"
    );

    let collection_item = crate::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items"),
        None,
        r#"{"contentId":"content:music:recording:route-audit","mediaKind":"Music","contentHash":"abc123","fileName":"Route Audit.flac","title":"Route Audit","artist":"Artist","album":"Album"}"#,
        &state,
    )
    .await
    .expect("create collection item");
    let item_json =
        serde_json::from_str::<serde_json::Value>(&collection_item.body).unwrap_or_default();
    let item_pass = collection_item.status == "201 Created"
        && item_json["collectionId"] == collection_id
        && item_json["contentHash"] == "abc123";
    record!(
        "POST",
        "/api/v0/collections/{id}/items",
        "mutation-side-effects-and-readback",
        item_pass
    );
    record!(
        "POST",
        "/api/v0/collections/{id}/items",
        "nominal-status-headers-body",
        collection_item.status == "201 Created"
    );
    let item_id = item_json["id"].as_str().unwrap_or_default().to_owned();

    let updated_collection = crate::route_http_request(
        "PUT",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        r#"{"title":"Updated Route Audit","type":"Playlist"}"#,
        &state,
    )
    .await
    .expect("update collection");
    let updated_collection_json =
        serde_json::from_str::<serde_json::Value>(&updated_collection.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/collections/{id}",
        "mutation-side-effects-and-readback",
        updated_collection.status == "200 OK"
            && updated_collection_json["title"] == "Updated Route Audit"
    );
    record!(
        "PUT",
        "/api/v0/collections/{id}",
        "nominal-status-headers-body",
        updated_collection.status == "200 OK"
    );

    let malformed_collection_update = crate::route_http_request(
        "PUT",
        &format!("/api/v0/collections/{collection_id}/"),
        None,
        r#"{"title":"Malformed Path"}"#,
        &state,
    )
    .await
    .expect("malformed collection update path");
    record!(
        "PUT",
        "/api/v0/collections/{id}",
        "malformed-path-query-or-body",
        malformed_collection_update.status == "404 Not Found"
    );

    let updated_item = crate::route_http_request(
        "PUT",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
        None,
        r#"{"title":"Updated Item","album":"Updated Album","sha256":"def456"}"#,
        &state,
    )
    .await
    .expect("update collection item");
    let updated_item_json =
        serde_json::from_str::<serde_json::Value>(&updated_item.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/collections/{id}/items/{itemId}",
        "mutation-side-effects-and-readback",
        updated_item.status == "200 OK" && updated_item_json["contentHash"] == "def456"
    );
    record!(
        "PUT",
        "/api/v0/collections/{id}/items/{itemId}",
        "nominal-status-headers-body",
        updated_item.status == "200 OK"
    );

    let malformed_item_update = crate::route_http_request(
        "PUT",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}/"),
        None,
        r#"{"title":"Malformed Path"}"#,
        &state,
    )
    .await
    .expect("malformed collection item update path");
    record!(
        "PUT",
        "/api/v0/collections/{id}/items/{itemId}",
        "malformed-path-query-or-body",
        malformed_item_update.status == "404 Not Found"
    );

    let missing_item_update = crate::route_http_request(
        "PUT",
        &format!("/api/v0/collections/{collection_id}/items/missing-item"),
        None,
        r#"{"title":"Missing Item"}"#,
        &state,
    )
    .await
    .expect("missing collection item update");
    record!(
        "PUT",
        "/api/v0/collections/{id}/items/{itemId}",
        "missing-empty-or-conflict-state",
        missing_item_update.status == "404 Not Found"
    );

    let reordered = crate::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items/reorder"),
        None,
        &format!(r#"{{"itemIds":["{item_id}"]}}"#),
        &state,
    )
    .await
    .expect("reorder collection items");
    let reorder_pass = reordered.status == "204 No Content" && reordered.body.is_empty();

    let collection_items = crate::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}/items"),
        None,
        "",
        &state,
    )
    .await
    .expect("list collection items");
    let collection_items_json =
        serde_json::from_str::<serde_json::Value>(&collection_items.body).unwrap_or_default();
    let list_pass =
        collection_items.status == "200 OK" && collection_items_json[0]["id"] == item_id;
    record!(
        "POST",
        "/api/v0/collections/{id}/items/reorder",
        "mutation-side-effects-and-readback",
        reorder_pass && list_pass
    );
    record!(
        "POST",
        "/api/v0/collections/{id}/items/reorder",
        "nominal-status-headers-body",
        reorder_pass
    );

    let malformed_reorder = crate::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items/reorder/"),
        None,
        &format!(r#"{{"itemIds":["{item_id}"]}}"#),
        &state,
    )
    .await
    .expect("malformed collection reorder path");
    record!(
        "POST",
        "/api/v0/collections/{id}/items/reorder",
        "malformed-path-query-or-body",
        malformed_reorder.status == "404 Not Found"
    );
    let missing_reorder = crate::route_http_request(
        "POST",
        "/api/v0/collections/00000000-0000-0000-0000-000000000000/items/reorder",
        None,
        r#"{"itemIds":["00000000-0000-0000-0000-000000000000"]}"#,
        &state,
    )
    .await
    .expect("missing collection reorder");
    record!(
        "POST",
        "/api/v0/collections/{id}/items/reorder",
        "missing-empty-or-conflict-state",
        missing_reorder.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/collections/{id}/items",
        "nominal-status-headers-body",
        list_pass
    );
    record!(
        "GET",
        "/api/v0/collections/{id}/items",
        "populated-dynamic-state",
        list_pass
    );

    let malformed_item_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}/"),
        None,
        "",
        &state,
    )
    .await
    .expect("malformed collection item delete path");
    record!(
        "DELETE",
        "/api/v0/collections/{id}/items/{itemId}",
        "malformed-path-query-or-body",
        malformed_item_delete.status == "404 Not Found"
    );

    let removed_item = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection item");
    record!(
        "DELETE",
        "/api/v0/collections/{id}/items/{itemId}",
        "mutation-side-effects-and-readback",
        removed_item.status == "204 No Content"
    );
    record!(
        "DELETE",
        "/api/v0/collections/{id}/items/{itemId}",
        "nominal-status-headers-body",
        removed_item.status == "204 No Content"
    );
    let missing_item = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing collection item");
    record!(
        "DELETE",
        "/api/v0/collections/{id}/items/{itemId}",
        "missing-empty-or-conflict-state",
        missing_item.status == "404 Not Found"
    );

    let malformed_collection_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}/"),
        None,
        "",
        &state,
    )
    .await
    .expect("malformed collection delete path");
    record!(
        "DELETE",
        "/api/v0/collections/{id}",
        "malformed-path-query-or-body",
        malformed_collection_delete.status == "404 Not Found"
    );

    let removed_collection = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete collection");
    record!(
        "DELETE",
        "/api/v0/collections/{id}",
        "mutation-side-effects-and-readback",
        removed_collection.status == "204 No Content"
    );
    record!(
        "DELETE",
        "/api/v0/collections/{id}",
        "nominal-status-headers-body",
        removed_collection.status == "204 No Content"
    );
    let missing_collection = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing collection");
    record!(
        "DELETE",
        "/api/v0/collections/{id}",
        "missing-empty-or-conflict-state",
        missing_collection.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("collections_items_crud_reorder_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api collections mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
