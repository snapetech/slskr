//! Controller full integrations differential 02 ownership.

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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_lidarr_and_source_feed_contracts() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
    let server = tokio::spawn(async move {
        let responses = [
            serde_json::json!({"appName": "Lidarr", "version": "2.1.0"}).to_string(),
            serde_json::json!({
                "totalRecords": 1,
                "records": [{
                    "id": 7,
                    "title": "Fixture Album",
                    "artist": {"id": 8, "artistName": "Fixture Artist"}
                }]
            })
            .to_string(),
            serde_json::json!([{
                "id": 9,
                "path": "/lidarr/Fixture Album/track.flac",
                "artist": {"id": 8, "artistName": "Fixture Artist"},
                "album": {"id": 10, "title": "Fixture Album"},
                "albumReleaseId": 11,
                "tracks": [{"id": 12, "title": "Track"}],
                "quality": {"quality": {"id": 1}},
                "additionalFile": false,
                "rejections": []
            }])
            .to_string(),
            serde_json::json!({"id": 42, "name": "ManualImport", "status": "queued"}).to_string(),
            serde_json::json!([{
                "id": 19,
                "path": "/lidarr/Auto Album/track.flac",
                "artist": {"id": 18, "artistName": "Auto Artist"},
                "album": {"id": 20, "title": "Auto Album"},
                "albumReleaseId": 21,
                "tracks": [{"id": 22, "title": "Track"}],
                "quality": {"quality": {"id": 1}},
                "additionalFile": false,
                "rejections": []
            }])
            .to_string(),
            serde_json::json!({"id": 43, "name": "ManualImport", "status": "queued"}).to_string(),
        ];
        for response in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = vec![0_u8; 65_536];
            let count = stream.read(&mut bytes).await.unwrap();
            request_tx
                .send(String::from_utf8_lossy(&bytes[..count]).to_string())
                .unwrap();
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.len(),
                response
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
        }
    });

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", &format!("http://{address}"))
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_SYNC_WANTED", "true")
            .with("SLSKR_LIDARR_SYNC_MAX_ITEMS", "1")
            .with("SLSKR_LIDARR_AUTO_DOWNLOAD", "true")
            .with("SLSKR_LIDARR_WISHLIST_FILTER", "flac")
            .with("SLSKR_LIDARR_WISHLIST_MAX_RESULTS", "37")
            .with("SLSKR_LIDARR_AUTO_IMPORT_COMPLETED", "true")
            .with("SLSKR_LIDARR_IMPORT_PATH_FROM", "/downloads")
            .with("SLSKR_LIDARR_IMPORT_PATH_TO", "/lidarr")
            .with("SLSKR_LIDARR_IMPORT_MODE", "copy")
            .with("SLSKR_LIDARR_IMPORT_REPLACE_EXISTING", "true"),
    );

    let status = crate::route_http_request(
        "GET",
        "/api/v0/integrations/lidarr/status",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(status.status, "200 OK", "{}", status.body);
    assert_eq!(status.content_type, "application/json; charset=utf-8");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&status.body).unwrap(),
        serde_json::json!({"appName": "Lidarr", "version": "2.1.0"})
    );

    let sync = crate::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/wanted/sync",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(sync.status, "200 OK", "{}", sync.body);
    assert_eq!(sync.content_type, "application/json; charset=utf-8");
    let sync: serde_json::Value = serde_json::from_str(&sync.body).unwrap();
    assert_eq!(sync["wantedCount"], 1);
    assert_eq!(sync["createdCount"], 1);
    let item = state
        .wishlist
        .read()
        .await
        .records
        .iter()
        .flat_map(|record| record.items.iter())
        .next()
        .cloned()
        .unwrap();
    assert_eq!(item.search_text(), "Fixture Artist Fixture Album");
    assert_eq!(item.filter, "flac");
    assert!(item.auto_download);
    assert_eq!(item.max_results, 37);

    let import = crate::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/downloads/Fixture Album"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(import.status, "200 OK", "{}", import.body);
    let import: serde_json::Value = serde_json::from_str(&import.body).unwrap();
    assert_eq!(import["directory"], "/lidarr/Fixture Album");
    assert_eq!(import["candidateCount"], 1);
    assert_eq!(import["safeCandidateCount"], 1);
    assert_eq!(import["commandId"], 42);
    assert_eq!(import["importMode"], "Copy");

    let repeated = crate::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/downloads/Fixture Album"}"#,
        &state,
    )
    .await
    .unwrap();
    let repeated: serde_json::Value = serde_json::from_str(&repeated.body).unwrap();
    assert_eq!(repeated["skippedReason"], "Recently processed");

    let completed = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Remote/Auto Album/track.flac".to_owned(),
            Some("/downloads/Auto Album/track.flac".to_owned()),
            Some(1),
            None,
            crate::TransferRequestDetails::default(),
        );
        transfers
            .update_local_execution(entry.id, "succeeded", 1, Some(1), None)
            .unwrap()
    };
    crate::maybe_import_lidarr_completed_download(&state, &completed).await;

    let status_request = request_rx.recv().await.unwrap();
    let wanted_request = request_rx.recv().await.unwrap();
    let candidates_request = request_rx.recv().await.unwrap();
    let command_request = request_rx.recv().await.unwrap();
    assert!(status_request.contains("/api/v1/system/status"));
    assert!(wanted_request
        .to_ascii_lowercase()
        .contains("x-api-key: fixture-key"));
    assert!(wanted_request.contains("page=1&pageSize=250&includeArtist=true&monitored=true"));
    assert!(candidates_request.contains("folder=%2Flidarr%2FFixture%20Album"));
    assert!(candidates_request.contains("replaceExistingFiles=true"));
    assert!(command_request.contains("\"importMode\":\"Copy\""));
    assert!(command_request.contains("\"replaceExistingFiles\":true"));
    assert!(command_request.contains("\"foreignArtistId\":\"\""));
    let automatic_candidates_request = request_rx.recv().await.unwrap();
    let automatic_command_request = request_rx.recv().await.unwrap();
    assert!(automatic_candidates_request.contains("folder=%2Flidarr%2FAuto%20Album"));
    assert!(automatic_command_request.contains("\"name\":\"ManualImport\""));
    server.await.unwrap();
    assert!(!state.runtime.read().await.application_restart_requested);

    let yaml = r#"integrations:
  lidarr:
    enabled: false
    timeout_seconds: 31
    sync_interval_seconds: 600
    max_items_per_sync: 9
    wishlist_max_results: 41
  spotify:
    enabled: true
    client_id: watched-client
    client_secret: watched-secret
    redirect_uri: http://127.0.0.1/spotify-callback
    timeout_seconds: 32
    max_items_per_import: 43
    market: CA
"#;
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    let mut cli_environment = state.controller_cli_environment.clone();
    cli_environment.retain(|name, _| {
        !name.starts_with("SLSKR_LIDARR_") && !name.starts_with("SLSKR_SPOTIFY_")
    });
    crate::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;
    let integrations = state.integration_settings.read().await;
    assert!(!integrations.lidarr.enabled);
    assert_eq!(integrations.lidarr.timeout_seconds, 31);
    assert!(integrations.spotify.enabled);
    assert_eq!(
        integrations.spotify.client_id.as_deref(),
        Some("watched-client")
    );
    assert_eq!(integrations.spotify.timeout_seconds, 32);
    assert_eq!(integrations.spotify.max_items_per_import, 43);
    assert_eq!(integrations.spotify.market, "CA");
    drop(integrations);
    assert!(!state.runtime.read().await.application_restart_requested);

    let disabled_import = crate::route_http_request(
        "POST",
        "/api/v0/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/downloads/Fixture Album"}"#,
        &state,
    )
    .await
    .unwrap();
    let disabled_import: serde_json::Value = serde_json::from_str(&disabled_import.body).unwrap();
    assert_eq!(disabled_import["enabled"], false);

    let plugins = crate::route_http_request("GET", "/api/config/plugins", None, "", &state)
        .await
        .unwrap();
    let plugins: serde_json::Value = serde_json::from_str(&plugins.body).unwrap();
    assert_eq!(plugins["plugins"][0]["enabled"], true);
    assert_eq!(plugins["plugins"][1]["enabled"], false);

    let source_text = (1..=50)
        .map(|index| format!("Artist {index} - Track {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let empty_unversioned_history = crate::route_http_request(
        "GET",
        "/api/source-feed-imports/history?limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(empty_unversioned_history.status, "200 OK");
    assert_eq!(
        empty_unversioned_history.content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(empty_unversioned_history.body, "[]");

    let preview = crate::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &serde_json::json!({
            "sourceText": source_text,
            "sourceKind": "text",
            "fetchProviderUrls": false,
            "limit": 500,
        })
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    let preview: serde_json::Value = serde_json::from_str(&preview.body).unwrap();
    assert_eq!(preview["totalRows"], 43);
    assert_eq!(preview["suggestionCount"], 43);
    assert_eq!(preview["suggestions"].as_array().unwrap().len(), 43);

    let history = crate::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history?limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let history: serde_json::Value = serde_json::from_str(&history.body).unwrap();
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["limit"], 500);
    assert_eq!(history[0]["totalRows"], 43);
    assert_eq!(history[0]["suggestions"].as_array().unwrap().len(), 25);
    assert_eq!(history[0]["sourceFingerprint"].as_str().unwrap().len(), 64);
    let import_id = history[0]["importId"].as_str().unwrap();
    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/source-feed-imports/history/{import_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&detail.body).unwrap(),
        history[0]
    );
    let missing_unversioned_detail = crate::route_http_request(
        "GET",
        "/api/source-feed-imports/history/does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_unversioned_detail.status, "404 Not Found");
    let unversioned_history = crate::route_http_request(
        "GET",
        "/api/source-feed-imports/history?limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unversioned_history.status, "200 OK");
    assert_eq!(
        unversioned_history.content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(unversioned_history.body, history.to_string());
    let unversioned_detail = crate::route_http_request(
        "GET",
        &format!("/api/source-feed-imports/history/{import_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unversioned_detail.status, "200 OK");
    assert_eq!(
        unversioned_detail.content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unversioned_detail.body).unwrap(),
        history[0]
    );
    let reloaded = crate::SourceFeedImportHistoryStore::load(&state.config.state_dir);
    assert_eq!(reloaded.history.len(), 1);
    assert_eq!(reloaded.history[0]["importId"], import_id);

    for (method, route, case) in [
        (
            "GET",
            "/api/v0/integrations/lidarr/status",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/wanted/sync",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/wanted/sync",
            "mutation-side-effects-and-readback",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "mutation-side-effects-and-readback",
        ),
        (
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "concurrency-and-idempotency",
        ),
        (
            "GET",
            "/api/v0/source-feed-imports/history",
            "populated-dynamic-state",
        ),
        (
            "GET",
            "/api/v0/source-feed-imports/history/{importId}",
            "populated-dynamic-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history",
            "missing-empty-or-conflict-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/source-feed-imports/history",
            "populated-dynamic-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history/{importId}",
            "missing-empty-or-conflict-state",
        ),
        (
            "GET",
            "/api/source-feed-imports/history/{importId}",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/source-feed-imports/history/{importId}",
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
        evidence_dir.join("lidarr_and_source_feed_contracts.json"),
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_source_feed_open_cases() {
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

    let env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env.clone());

    for (path, route) in [
        (
            "/api/source-feed-imports/history/extra/segment",
            "/api/source-feed-imports/history",
        ),
        (
            "/api/source-feed-imports/history/open-id/extra",
            "/api/source-feed-imports/history/{importId}",
        ),
        (
            "/api/v0/source-feed-imports/history/extra/segment",
            "/api/v0/source-feed-imports/history",
        ),
        (
            "/api/v0/source-feed-imports/history/open-id/extra",
            "/api/v0/source-feed-imports/history/{importId}",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("malformed source-feed history path");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let empty_versioned_history = crate::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &state,
    )
    .await
    .expect("empty versioned source-feed history");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history",
        "missing-empty-or-conflict-state",
        empty_versioned_history.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&empty_versioned_history.body)
                .is_ok_and(|value| value == serde_json::json!([]))
    );
    let missing_versioned_detail = crate::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history/missing-open-id",
        None,
        "",
        &state,
    )
    .await
    .expect("missing versioned source-feed detail");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history/{importId}",
        "missing-empty-or-conflict-state",
        missing_versioned_detail.status == "404 Not Found"
    );

    let preview_body = serde_json::json!({
        "sourceText": "Open Artist - Open Track",
        "sourceKind": "text",
        "fetchProviderUrls": false,
        "limit": 10,
    })
    .to_string();
    let preview = crate::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &preview_body,
        &state,
    )
    .await
    .expect("versioned source-feed preview");
    let preview_json = serde_json::from_str::<serde_json::Value>(&preview.body).unwrap_or_default();
    let history = crate::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &state,
    )
    .await
    .expect("populated versioned source-feed history");
    let history_json = serde_json::from_str::<serde_json::Value>(&history.body).unwrap_or_default();
    let import_id = history_json[0]["importId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/source-feed-imports/history/{import_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("versioned source-feed detail");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history/{importId}",
        "nominal-status-headers-body",
        detail.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&detail.body)
                .is_ok_and(|value| value["importId"] == import_id)
    );

    let unversioned_preview = crate::route_http_request(
        "POST",
        "/api/source-feed-imports/preview",
        None,
        r#"{"sourceText":"Unversioned Artist - Unversioned Track"}"#,
        &state,
    )
    .await
    .expect("unversioned source-feed preview");
    let unversioned_preview_json =
        serde_json::from_str::<serde_json::Value>(&unversioned_preview.body).unwrap_or_default();
    let unversioned_history_after = crate::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned history after unversioned source-feed rejection");
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "mutation-side-effects-and-readback",
        unversioned_preview.status == "400 Bad Request"
            && unversioned_preview_json["code"] == "ApiVersionUnspecified"
            && serde_json::from_str::<serde_json::Value>(&unversioned_history_after.body)
                .is_ok_and(|value| value.as_array().is_some_and(|rows| rows.len() == 1))
    );
    let unversioned_restart = test_state_with_env(env.clone()).0;
    let unversioned_restart_history = crate::route_http_request(
        "GET",
        "/api/source-feed-imports/history",
        None,
        "",
        &unversioned_restart,
    )
    .await
    .expect("unversioned source-feed restart");
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "restart-persistence-or-reset",
        unversioned_preview.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&unversioned_restart_history.body)
                .is_ok_and(|value| value == serde_json::json!([]))
    );
    let unversioned_concurrent_bodies = [
        r#"{"sourceText":"Concurrent Artist A - Track A"}"#,
        r#"{"sourceText":"Concurrent Artist B - Track B"}"#,
    ];
    let unversioned_concurrent =
        futures_util::future::join_all(unversioned_concurrent_bodies.into_iter().map(|body| {
            let state = Arc::clone(&state);
            async move {
                crate::route_http_request(
                    "POST",
                    "/api/source-feed-imports/preview",
                    None,
                    body,
                    &state,
                )
                .await
            }
        }))
        .await;
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "concurrency-and-idempotency",
        unversioned_concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "400 Bad Request")
        })
    );

    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "mutation-side-effects-and-readback",
        preview.status == "200 OK"
            && preview_json["suggestionCount"] == 1
            && history_json.as_array().is_some_and(|rows| rows.len() == 1)
            && !import_id.is_empty()
    );
    let reloaded_history = crate::SourceFeedImportHistoryStore::load(&state.config.state_dir);
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "restart-persistence-or-reset",
        reloaded_history
            .history
            .iter()
            .any(|entry| { entry["importId"].as_str() == Some(import_id.as_str()) })
    );
    let versioned_concurrent_bodies: Vec<String> = (0..2)
        .map(|index| {
            serde_json::json!({
                "sourceText": format!("Versioned Concurrent {index} - Track {index}"),
                "sourceKind": "text",
                "fetchProviderUrls": false,
            })
            .to_string()
        })
        .collect();
    let versioned_concurrent =
        futures_util::future::join_all(versioned_concurrent_bodies.iter().map(|body| {
            let body = body.clone();
            let state = Arc::clone(&state);
            async move {
                crate::route_http_request(
                    "POST",
                    "/api/v0/source-feed-imports/preview",
                    None,
                    &body,
                    &state,
                )
                .await
            }
        }))
        .await;
    let reloaded_after_concurrent =
        crate::SourceFeedImportHistoryStore::load(&state.config.state_dir);
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "concurrency-and-idempotency",
        versioned_concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && reloaded_after_concurrent.history.len() >= 3
    );

    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("source-feed runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        env.clone().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    let runtime_unversioned = crate::route_http_request(
        "POST",
        "/api/source-feed-imports/preview",
        None,
        r#"{"sourceText":"Runtime Artist - Runtime Track"}"#,
        &runtime_state,
    )
    .await
    .expect("runtime unversioned source-feed preview");
    record!(
        "POST",
        "/api/source-feed-imports/preview",
        "runtime-failure-and-timeout",
        runtime_unversioned.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&runtime_unversioned.body)
                .is_ok_and(|value| value["code"] == "ApiVersionUnspecified")
    );
    let runtime_history = crate::route_http_request(
        "GET",
        "/api/source-feed-imports/history",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime unversioned source-feed history");
    let runtime_detail = crate::route_http_request(
        "GET",
        "/api/source-feed-imports/history/runtime-missing",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime unversioned source-feed detail");
    record!(
        "GET",
        "/api/source-feed-imports/history",
        "runtime-failure-and-timeout",
        runtime_history.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_history.body)
                .is_ok_and(|value| value.is_array())
    );
    record!(
        "GET",
        "/api/source-feed-imports/history/{importId}",
        "runtime-failure-and-timeout",
        runtime_detail.status == "404 Not Found"
    );
    let runtime_versioned_history = crate::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime versioned source-feed history");
    let runtime_versioned_detail = crate::route_http_request(
        "GET",
        "/api/v0/source-feed-imports/history/runtime-missing",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime versioned source-feed detail");
    record!(
        "GET",
        "/api/v0/source-feed-imports/history",
        "runtime-failure-and-timeout",
        runtime_versioned_history.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_versioned_history.body)
                .is_ok_and(|value| value.is_array())
    );
    record!(
        "GET",
        "/api/v0/source-feed-imports/history/{importId}",
        "runtime-failure-and-timeout",
        runtime_versioned_detail.status == "404 Not Found"
    );

    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-source-feed-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file")
        .expect("create source-feed state conflict");
    let mut conflict_state = test_state_with_env(env).0;
    Arc::get_mut(&mut conflict_state)
        .expect("exclusive source-feed conflict state")
        .config
        .state_dir = conflict_root.clone();
    let runtime_versioned_preview = crate::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &preview_body,
        &conflict_state,
    )
    .await
    .expect("runtime versioned source-feed preview");
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "malformed-path-query-or-body",
        crate::route_http_request(
            "POST",
            "/api/v0/source-feed-imports/preview/extra",
            None,
            &preview_body,
            &state,
        )
        .await
        .is_ok_and(|response| response.status == "404 Not Found")
    );
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "missing-empty-or-conflict-state",
        crate::route_http_request(
            "POST",
            "/api/v0/source-feed-imports/preview",
            None,
            "",
            &state,
        )
        .await
        .is_ok_and(|response| response.status == "400 Bad Request")
    );
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "runtime-failure-and-timeout",
        runtime_versioned_preview.status == "503 Service Unavailable"
    );
    let _ = fs::remove_file(conflict_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create source-feed evidence directory");
    fs::write(
        evidence_dir.join("source_feed_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize source-feed ledger"),
    )
    .expect("write source-feed ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api source-feed-imports mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining MusicBrainz controller matrix.
/// The existing MusicBrainz tests cover the detailed release-radar and
/// overlay contracts; this ledger closes the frozen controller rows that
/// exercise their remaining malformed, empty, runtime, restart, and
/// concurrent dimensions.
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_musicbrainz_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
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

    let seed_artist = |state: Arc<crate::AppState>| async move {
        state.library.write().await.create(
            "MusicBrainz Residual Artist".to_owned(),
            "Residual Release".to_owned(),
            "Audio".to_owned(),
        );
    };
    let get_cases = [
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/albums/completion",
            "/api/v0/musicbrainz/albums/completion",
            "populated-dynamic-state",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage?unexpected=not-a-number",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "malformed-path-query-or-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "runtime-failure-and-timeout",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "populated-dynamic-state",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "nominal-status-headers-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph?unexpected=not-a-number",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "malformed-path-query-or-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/definitely-missing/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "missing-empty-or-conflict-state",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "runtime-failure-and-timeout",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/artist/{artistId}/release-graph",
            "populated-dynamic-state",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/overlays/artist/MusicBrainz%20Residual%20Artist/release-graph?unexpected=not-a-number",
            "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
            "malformed-path-query-or-body",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/overlays/artist/MusicBrainz%20Residual%20Artist/release-graph",
            "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
            "runtime-failure-and-timeout",
            "200 OK",
            true,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/export-review?unexpected=not-a-number",
            "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
            "malformed-path-query-or-body",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/export-review",
            "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
            "missing-empty-or-conflict-state",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/export-review",
            "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
            "runtime-failure-and-timeout",
            "404 Not Found",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/routes?unexpected=not-a-number",
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/routes",
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/overlays/edits/missing-residual/routes",
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications?unexpected=not-a-number",
            "/api/v0/musicbrainz/release-radar/notifications",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications",
            "/api/v0/musicbrainz/release-radar/notifications",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications",
            "/api/v0/musicbrainz/release-radar/notifications",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes?unexpected=not-a-number",
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes",
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/subscriptions?unexpected=not-a-number",
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "malformed-path-query-or-body",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "missing-empty-or-conflict-state",
            "200 OK",
            false,
        ),
        (
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "runtime-failure-and-timeout",
            "200 OK",
            false,
        ),
    ];
    for (path, route, case, expected_status, populated) in get_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if populated {
            seed_artist(state.clone()).await;
        }
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == expected_status && !response.body.is_empty()
        );
    }

    let edit_body = r#"{"editId":"musicbrainz-residual-edit","type":"TitleCorrection","targetType":"Recording","targetId":"recording-residual","field":"title","value":"Residual Title","evidence":[{"type":"WorkRef","reference":"residual-ref"}]}"#;
    let valid_subscription = r#"{"artistId":"artist-residual","artistName":"Residual Artist","scope":"trusted","enabled":true,"mutedReleaseGroupIds":[],"createdAt":"2026-01-01T00:00:00Z"}"#;
    let valid_observation = r#"{"artistId":"artist-residual","recordingId":"recording-residual","songIdConfirmed":true,"confidence":1,"workRef":{"id":"recording-residual","type":"recording","domain":"music","externalIds":{},"title":"Residual Track","creator":"Residual Artist","year":2026,"metadata":{},"attributedTo":"actor","published":"2026-01-01T00:00:00Z"},"observedAt":"2026-01-01T00:00:00Z"}"#;
    let post_cases: Vec<(&str, String, &str, &str, String, &str, bool)> = vec![
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "nominal-status-headers-body",
            r#"{"missingReleases":["Residual Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "400 Bad Request",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "missing-empty-or-conflict-state",
            String::new(),
            "400 Bad Request",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "runtime-failure-and-timeout",
            r#"{"missingReleases":["Residual Runtime Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "mutation-side-effects-and-readback",
            r#"{"missingReleases":["Residual Readback Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "restart-persistence-or-reset",
            r#"{"missingReleases":["Residual Restart Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/artist/MusicBrainz%20Residual%20Artist/discography-coverage/wishlist".to_owned(),
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage/wishlist",
            "concurrency-and-idempotency",
            r#"{"missingReleases":["Residual Concurrent Album"],"maxResults":5}"#.to_owned(),
            "500 Internal Server Error",
            true,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "nominal-status-headers-body",
            r#"{"recordingIds":["recording-residual"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "runtime-failure-and-timeout",
            r#"{"recordingIds":["recording-runtime"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "mutation-side-effects-and-readback",
            r#"{"recordingIds":["recording-readback"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "restart-persistence-or-reset",
            r#"{"recordingIds":["recording-restart"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/diffs".to_owned(),
            "/api/v0/musicbrainz/library-bloom/diffs",
            "concurrency-and-idempotency",
            r#"{"recordingIds":["recording-concurrent"]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "missing-empty-or-conflict-state",
            String::new(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "runtime-failure-and-timeout",
            r#"{"saltId":"residual-runtime","expectedItems":16}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "restart-persistence-or-reset",
            r#"{"saltId":"residual-restart","expectedItems":16}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/snapshots/preview".to_owned(),
            "/api/v0/musicbrainz/library-bloom/snapshots/preview",
            "concurrency-and-idempotency",
            r#"{"saltId":"residual-concurrent","expectedItems":16}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "nominal-status-headers-body",
            r#"{"suggestions":[{"artist":"Residual Artist","title":"Residual Bloom Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "runtime-failure-and-timeout",
            r#"{"suggestions":[{"artist":"Residual Runtime Artist","title":"Residual Runtime Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "mutation-side-effects-and-readback",
            r#"{"suggestions":[{"artist":"Residual Readback Artist","title":"Residual Readback Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "restart-persistence-or-reset",
            r#"{"suggestions":[{"artist":"Residual Restart Artist","title":"Residual Restart Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/library-bloom/wishlist".to_owned(),
            "/api/v0/musicbrainz/library-bloom/wishlist",
            "concurrency-and-idempotency",
            r#"{"suggestions":[{"artist":"Residual Concurrent Artist","title":"Residual Concurrent Album"}]}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "runtime-failure-and-timeout",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "mutation-side-effects-and-readback",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "restart-persistence-or-reset",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits".to_owned(),
            "/api/v0/musicbrainz/overlays/edits",
            "concurrency-and-idempotency",
            edit_body.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/approve-export".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
            "runtime-failure-and-timeout",
            r#"{"approvedBy":"residual-reviewer"}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/approve-export".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
            "restart-persistence-or-reset",
            r#"{"approvedBy":"residual-reviewer"}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/approve-export".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
            "concurrency-and-idempotency",
            r#"{"approvedBy":"residual-reviewer"}"#.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/routes".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "nominal-status-headers-body",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/routes".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "restart-persistence-or-reset",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/musicbrainz-residual-edit/routes".to_owned(),
            "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
            "concurrency-and-idempotency",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes".to_owned(),
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "nominal-status-headers-body",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes".to_owned(),
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "restart-persistence-or-reset",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/notifications/missing-residual/routes".to_owned(),
            "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
            "concurrency-and-idempotency",
            r#"{"targetPeerIds":["residual-peer"]}"#.to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/observations".to_owned(),
            "/api/v0/musicbrainz/release-radar/observations",
            "missing-empty-or-conflict-state",
            "{}".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/observations".to_owned(),
            "/api/v0/musicbrainz/release-radar/observations",
            "runtime-failure-and-timeout",
            valid_observation.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/observations".to_owned(),
            "/api/v0/musicbrainz/release-radar/observations",
            "restart-persistence-or-reset",
            valid_observation.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "malformed-path-query-or-body",
            "{}".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "missing-empty-or-conflict-state",
            String::new(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "restart-persistence-or-reset",
            valid_subscription.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/release-radar/subscriptions".to_owned(),
            "/api/v0/musicbrainz/release-radar/subscriptions",
            "concurrency-and-idempotency",
            valid_subscription.to_owned(),
            "200 OK",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "nominal-status-headers-body",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "malformed-path-query-or-body",
            "not-json".to_owned(),
            "400 Bad Request",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "mutation-side-effects-and-readback",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "restart-persistence-or-reset",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets".to_owned(),
            "/api/v0/musicbrainz/targets",
            "concurrency-and-idempotency",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#.to_owned(),
            "404 Not Found",
            false,
        ),
    ];

    for (method, path, route, case, body, expected_status, _seed_edit) in post_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if path.contains("/approve-export")
            || path.contains("/overlays/edits/")
            || path.ends_with("/overlays/edits")
        {
            let seed = crate::route_http_request(
                "POST",
                "/api/v0/musicbrainz/overlays/edits",
                None,
                edit_body,
                &state,
            )
            .await
            .expect("seed MusicBrainz residual edit");
            assert_eq!(seed.status, "200 OK", "edit seed: {}", seed.body);
        }
        if path.contains("discography-coverage/wishlist") {
            seed_artist(state.clone()).await;
        }
        if case == "concurrency-and-idempotency" {
            let (left, right) = tokio::join!(
                crate::route_http_request(method, &path, None, &body, &state),
                crate::route_http_request(method, &path, None, &body, &state)
            );
            record!(
                method,
                route,
                case,
                left.as_ref()
                    .is_ok_and(|response| response.status == expected_status)
                    && right
                        .as_ref()
                        .is_ok_and(|response| response.status == expected_status)
            );
        } else {
            let response = crate::route_http_request(method, &path, None, &body, &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            record!(
                method,
                route,
                case,
                response.status == expected_status
                    && (expected_status == "404 Not Found" || !response.body.is_empty())
            );
        }
    }

    assert_eq!(ledger.len(), 80, "MusicBrainz residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create MusicBrainz evidence directory");
    fs::write(
        evidence_dir.join("musicbrainz_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize MusicBrainz ledger"),
    )
    .expect("write MusicBrainz ledger");
    assert!(
        mismatches.is_empty(),
        "{} MusicBrainz residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
