//! Controller full telemetry contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn health_warns_when_auth_disabled_on_non_loopback() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
            .with("SLSKR_AUTH_DISABLED", "true"),
    );

    let response = crate::route_http_request("GET", "/api/health", None, "", &state)
        .await
        .expect("health response");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["warnings"][0], "auth_disabled_non_loopback");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn metrics_api_returns_scrapable_counters() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("GET", "/api/v0/metrics", None, "", &state)
        .await
        .expect("metrics response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(
        response.content_type,
        "text/plain; version=0.0.4; charset=utf-8"
    );
    assert!(response.body.contains("slskr_session_connected 0"));
    assert!(response.body.contains("slskr_shares_files 1"));
    assert!(response.body.contains("slskr_shares_bytes 42"));
    assert!(response.body.contains("slskr_transfers{state=\"total\"} 0"));
    assert!(response
        .body
        .contains("slskr_transfers{state=\"active\"} 0"));
    assert!(response.body.contains("slskr_events_total 0"));
    assert!(response.body.contains("slskr_database_enabled 0"));
    assert!(response.body.contains("slskr_database_stats_available 0"));
    assert!(response
        .body
        .contains("slskr_database_rows{store=\"searches\"} 0"));
    assert!(response
        .body
        .contains("slskr_runtime_operations_total{operation=\"backfill\"} 0"));
    assert!(!response.body.contains("secret"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn metrics_distinguish_database_stats_failure_from_empty_database() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let healthy = crate::route_http_request("GET", "/api/metrics", None, "", &state)
        .await
        .expect("healthy metrics response");
    assert!(healthy.body.contains("slskr_database_enabled 1"));
    assert!(healthy.body.contains("slskr_database_stats_available 1"));

    db.close_for_test().await;
    let failed = crate::route_http_request("GET", "/api/metrics", None, "", &state)
        .await
        .expect("failed database metrics response");
    assert!(failed.body.contains("slskr_database_enabled 1"));
    assert!(failed.body.contains("slskr_database_stats_available 0"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn telemetry_api_returns_runtime_health_without_secrets() {
    let (state, _receiver) = test_state();
    state.shares.write().await.cache_error =
        Some("share cache write failed: /private/share-index.tsv denied".to_owned());
    {
        let mut transfers = state.transfers.write().await;
        transfers.state_error =
            Some("transfer state parse failed at /private/transfer-state.json".to_owned());
        transfers.events_error =
            Some("transfer event open failed at /private/transfer-events.tsv".to_owned());
    }

    let response = crate::route_http_request("GET", "/api/v0/telemetry", None, "", &state)
        .await
        .expect("telemetry response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "application/json");
    assert!(response.body.contains("\"service\":{\"name\":\"slskr\""));
    assert!(response.body.contains("\"health\":{"));
    assert!(response.body.contains("\"connected\":false"));
    assert!(response
        .body
        .contains("\"share_cache_file\":\"share-index.tsv\""));
    assert!(response
        .body
        .contains("\"share_cache_kind\":\"compatibility-debug\""));
    assert!(response
        .body
        .contains("\"transfer_events_file\":\"transfer-events.tsv\""));
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["shares"]["roots"], 0);
    assert_eq!(json["shares"]["files"], 1);
    assert_eq!(json["storage"]["share_cache_enabled"], true);
    assert_eq!(
        json["storage"]["share_cache_error"],
        "share cache unavailable"
    );
    assert!(!response.body.contains("/private"));
    assert!(!response.body.contains("denied"));
    assert_eq!(
        json["storage"]["transfer_state_error"],
        "transfer state unavailable"
    );
    assert_eq!(
        json["storage"]["transfer_events_error"],
        "transfer events unavailable"
    );
    assert_eq!(json["health"]["transferState"], false);
    assert_eq!(json["health"]["transferEvents"], false);
    assert_eq!(json["shares"]["cache_enabled"], true);
    assert_eq!(json["database"]["enabled"], false);
    assert_eq!(json["health"]["database"], true);
    assert_eq!(json["events"]["total"], 0);
    assert_eq!(json["messages"]["total"], 0);
    assert_eq!(json["rooms"]["joined"], 0);
    assert_eq!(json["transfers"]["total"], 0);
    assert_eq!(json["runtime"]["backfillRuns"], 0);
    assert!(!response.body.contains("secret"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn stats_api_aggregates_projection_counts() {
    let (state, mut receiver) = test_state();
    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }

    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"test flac\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/users/watch",
        None,
        "{\"username\":\"friend\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"filename\":\"Remote/Song.flac\",\"size\":123}",
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        "{\"username\":\"friend\",\"body\":\"hi\"}",
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/rooms/music/messages",
        None,
        "{\"username\":\"friend\",\"body\":\"track?\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        "{\"filename\":\"Remote/Song.flac\",\"size\":100}",
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        "{\"bytes_transferred\":40}",
        &state,
    )
    .await
    .unwrap();

    let stats = crate::route_http_request("GET", "/api/v0/stats", None, "", &state)
        .await
        .expect("stats response");

    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["shares"]["files"], 1);
    assert_eq!(stats_json["shares"]["bytes"], 42);
    assert_eq!(stats_json["searches"]["total"], 1);
    assert_eq!(stats_json["searches"]["active"], 1);
    assert_eq!(stats_json["searches"]["results"], 1);
    assert_eq!(stats_json["users"]["total"], 1);
    assert_eq!(stats_json["users"]["watched"], 1);
    assert_eq!(stats_json["browse"]["total"], 1);
    assert_eq!(stats_json["browse"]["ready"], 1);
    assert_eq!(stats_json["browse"]["files"], 1);
    assert_eq!(stats_json["browse"]["bytes"], 123);
    assert_eq!(stats_json["messages"]["total"], 1);
    assert_eq!(stats_json["messages"]["inbound"], 1);
    assert_eq!(stats_json["rooms"]["total"], 1);
    assert_eq!(stats_json["rooms"]["joined"], 1);
    assert_eq!(stats_json["rooms"]["messages"], 1);
    assert_eq!(stats_json["transfers"]["total"], 1);
    assert_eq!(stats_json["transfers"]["in_progress"], 1);
    assert_eq!(stats_json["transfers"]["bytes_transferred"], 40);
    assert_eq!(stats_json["database"]["enabled"], false);
    assert_eq!(stats_json["database"]["projections"]["searches"], 1);
    assert_eq!(stats_json["database"]["projections"]["transfers"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn stats_metrics_and_telemetry_expose_persisted_health_counts() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"durable metrics"}"#,
        &state,
    )
    .await
    .expect("create search");
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"username":"peer","files":[{"filename":"Remote/Metrics.flac","size":11}]}"#,
        &state,
    )
    .await
    .expect("ingest result");

    let stats = crate::route_http_request("GET", "/api/v0/stats", None, "", &state)
        .await
        .expect("stats response");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["database"]["enabled"], true);
    assert_eq!(stats_json["database"]["searches"], 1);
    assert_eq!(stats_json["database"]["searchResults"], 1);
    assert_eq!(stats_json["database"]["projections"]["searches"], 1);

    let metrics = crate::route_http_request("GET", "/api/v0/metrics", None, "", &state)
        .await
        .expect("metrics response");
    assert!(metrics.body.contains("slskr_database_enabled 1"));
    assert!(metrics
        .body
        .contains("slskr_database_rows{store=\"searches\"} 1"));
    assert!(metrics
        .body
        .contains("slskr_database_rows{store=\"search_results\"} 1"));

    let telemetry = crate::route_http_request("GET", "/api/v0/telemetry", None, "", &state)
        .await
        .expect("telemetry response");
    let telemetry_json = serde_json::from_str::<serde_json::Value>(&telemetry.body).unwrap();
    assert_eq!(telemetry_json["database"]["searchResults"], 1);
    assert_eq!(telemetry_json["projections"]["searches"], 1);
    assert_eq!(telemetry_json["health"]["database"], true);
}
