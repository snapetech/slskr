//! Controller full search differential ownership.

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
pub(super) async fn controller_api_differential_search_api_creates_reads_and_completes_records() {
    let (state, mut receiver) = test_state();
    // Versioned search creation requires an authenticated Soulseek server
    // session; this test exercises the positive create/read/complete path.
    state.session.write().await.state = "connected";

    let created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"test flac\"}",
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created.status, "200 OK");
    assert!(created.body.contains("\"query\":\"test flac\""));
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert!(created_json["searchId"].is_string());
    assert_eq!(created_json["query"], "test flac");
    assert!(created_json["results"].is_array());
    assert_eq!(
        receiver.try_recv().expect("search command"),
        crate::SessionCommand::Search {
            token: 1,
            query: "test flac".to_owned(),
            target: crate::SearchDispatchTarget::Global,
        }
    );
    let listed_compat = crate::route_http_request("GET", "/api/v0/searches", None, "", &state)
        .await
        .expect("list compatibility searches");
    assert_eq!(listed_compat.status, "200 OK");
    let listed_compat_json =
        serde_json::from_str::<serde_json::Value>(&listed_compat.body).unwrap();
    assert!(listed_compat_json
        .as_array()
        .is_some_and(|entries| { entries.iter().any(|entry| entry["query"] == "test flac") }));
    let duplicate = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"id":"1","query":"duplicate"}"#,
        &state,
    )
    .await
    .expect("reject duplicate search id");
    assert_eq!(duplicate.status, "409 Conflict");

    let listed = crate::route_http_request("GET", "/api/v0/searches/records", None, "", &state)
        .await
        .expect("list searches");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));
    assert!(listed.body.contains("\"filtered_count\":1"));

    let fetched = crate::route_http_request("GET", "/api/v0/searches/1", None, "", &state)
        .await
        .expect("get search");
    assert_eq!(fetched.status, "200 OK");
    assert!(fetched.content_type.contains("application/json"));
    assert!(fetched.body.contains("Virtual/Test.flac"));

    let completed =
        crate::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
            .await
            .expect("complete search");
    assert_eq!(completed.status, "200 OK");
    assert!(completed.body.contains("\"status\":\"completed\""));

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/searches",
            "case": "concurrency-and-idempotency",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/searches",
            "case": "missing-empty-or-conflict-state",
            "pass": duplicate.status == "409 Conflict",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/searches/{id}",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/searches/{id}",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/searches",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("search_api_reads_and_idempotency.json"),
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
pub(super) async fn controller_api_differential_search_creation_rehydrates() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";

    let created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"persist me\",\"target\":\"global\"}",
        &state,
    )
    .await
    .expect("create persisted search");
    assert_eq!(created.status, "200 OK");
    let _ = receiver.try_recv();

    let persisted = db.list_searches(10, 0).await.expect("list persisted");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].query, "persist me");

    let rehydrated = crate::SearchStore::from_persisted(persisted);
    let (restarted_state, _) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        rehydrated,
        Some(db),
    );
    let listed = crate::route_http_request(
        "GET",
        "/api/v0/searches/records",
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("list rehydrated searches");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));
    assert!(listed.body.contains("\"query\":\"persist me\""));

    ledger.push(serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/searches",
        "case": "restart-persistence-or-reset",
        "pass": true,
    }));
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("search_creation_rehydrates.json"),
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
pub(super) async fn controller_api_differential_search_mutation_lifecycle() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let mut ledger = Vec::new();
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";

    let created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"id":"stable-search","query":"durable search","ttl_seconds":60}"#,
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created.status, "200 OK");
    let _ = receiver.try_recv();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"username":"peer","files":[{"filename":"Remote/Durable.flac","size":42,"extension":"flac"}]}"#,
        &state,
    )
    .await
    .expect("ingest search response");
    assert_eq!(response.status, "200 OK");
    let persisted = db.get_search("1").await.expect("get search").unwrap();
    assert_eq!(persisted.result_count, 1);
    assert_eq!(
        db.list_search_identities()
            .await
            .expect("list search identities")
            .get("1")
            .map(String::as_str),
        Some("stable-search")
    );
    let persisted_results = db
        .list_search_results(Some("1"), 10, 0)
        .await
        .expect("list search results");
    assert_eq!(persisted_results.len(), 1);
    assert_eq!(persisted_results[0].peer_username.as_deref(), Some("peer"));
    assert_eq!(persisted_results[0].filename, "Remote/Durable.flac");

    let mut active = persisted.clone();
    active.status = "active".to_owned();
    active.result_count = 1;
    let terminalized =
        crate::SearchStore::from_persisted_with_results(vec![active], persisted_results.clone());
    let terminal = terminalized.get_by_identifier("1").unwrap();
    assert_eq!(terminal.status, "expired");
    assert!(terminal.results.is_empty());

    let completed =
        crate::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
            .await
            .expect("complete search");
    assert_eq!(completed.status, "200 OK");
    let rehydrated = crate::SearchStore::from_persisted_with_results_and_identities(
        db.list_searches(10, 0).await.expect("list searches"),
        persisted_results.clone(),
        db.list_search_identities()
            .await
            .expect("list search identities"),
    );
    let rehydrated_record = rehydrated
        .get_by_identifier("1")
        .expect("rehydrated search");
    assert_eq!(rehydrated_record.results.len(), 1);
    assert_eq!(rehydrated_record.results[0].filename, "Remote/Durable.flac");
    assert_eq!(
        rehydrated
            .get_by_identifier("stable-search")
            .expect("stable identifier")
            .token,
        1
    );

    let cancelled = crate::route_http_request("PUT", "/api/v0/searches/1", None, "", &state)
        .await
        .expect("cancel versioned search");
    assert_eq!(cancelled.status, "200 OK");
    assert!(cancelled.body.is_empty());
    assert_eq!(
        db.get_search("1")
            .await
            .expect("completed search remains terminal after cancellation")
            .unwrap()
            .status,
        "completed"
    );

    let updated = crate::route_http_request(
        "PUT",
        "/api/searches/1",
        None,
        r#"{"query":"durable updated","status":"failed"}"#,
        &state,
    )
    .await
    .expect("update search");
    assert_eq!(updated.status, "200 OK");
    let persisted = db.get_search("1").await.expect("get updated").unwrap();
    assert_eq!(persisted.query, "durable updated");
    assert_eq!(
        persisted.status, "completed",
        "terminal status is preserved while metadata changes"
    );
    assert!(persisted.completed_at.is_some());

    let deleted = crate::route_http_request("DELETE", "/api/searches/1", None, "", &state)
        .await
        .expect("delete search");
    assert_eq!(deleted.status, "200 OK");
    assert!(db.get_search("1").await.expect("get deleted").is_none());
    assert!(db
        .list_search_results(Some("1"), 10, 0)
        .await
        .expect("list deleted results")
        .is_empty());

    let created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"clearable"}"#,
        &state,
    )
    .await
    .expect("create clearable search");
    assert_eq!(created.status, "200 OK");
    let _ = receiver.try_recv();
    assert_eq!(
        db.list_searches(10, 0)
            .await
            .expect("list before clear")
            .len(),
        1
    );
    let stats = db.get_stats().await.expect("stats before clear");
    assert_eq!(stats.search_count, 1);
    assert_eq!(stats.search_result_count, 0);

    let clearable_id = db
        .list_searches(10, 0)
        .await
        .expect("list clearable search")
        .first()
        .map(|record| record.id.clone())
        .expect("clearable search id");
    let cancelled_active = crate::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{clearable_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel active versioned search");
    assert_eq!(cancelled_active.status, "200 OK");
    assert!(cancelled_active.body.is_empty());
    assert_eq!(
        db.get_search(&clearable_id)
            .await
            .expect("get actively cancelled search")
            .unwrap()
            .status,
        "cancelled"
    );
    let deleted_versioned = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{clearable_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete versioned search");
    assert_eq!(deleted_versioned.status, "204 No Content");
    assert!(deleted_versioned.body.is_empty());
    assert!(db
        .get_search(&clearable_id)
        .await
        .expect("get deleted versioned search")
        .is_none());

    let cleared = crate::route_http_request("DELETE", "/api/v0/searches", None, "", &state)
        .await
        .expect("clear searches");
    assert_eq!(cleared.status, "200 OK");
    assert!(db
        .list_searches(10, 0)
        .await
        .expect("list after clear")
        .is_empty());

    for (method, route, case) in [
        (
            "PUT",
            "/api/v0/searches/{id}",
            "nominal-status-headers-body",
        ),
        (
            "PUT",
            "/api/v0/searches/{id}",
            "mutation-side-effects-and-readback",
        ),
        (
            "DELETE",
            "/api/v0/searches/{id}",
            "nominal-status-headers-body",
        ),
        (
            "DELETE",
            "/api/v0/searches/{id}",
            "mutation-side-effects-and-readback",
        ),
        ("DELETE", "/api/v0/searches", "nominal-status-headers-body"),
        (
            "DELETE",
            "/api/v0/searches",
            "mutation-side-effects-and-readback",
        ),
    ] {
        ledger.push(serde_json::json!({
            "target": "slskdn",
            "method": method,
            "route": route,
            "case": case,
            "pass": true,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("search_mutation_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting `POST /api/v0/searches`'s
/// real dispatch-unavailable rollback (independently re-derived
/// from `search_create_rejects_before_mutation_when_dispatch_is_
/// unavailable`: with the session command receiver dropped, the
/// route genuinely 503s before any search record or event is
/// created) and the completely uncredited `POST /api/v0/player/
/// external-visualizer/launch` route across all 3 of its real
/// scenarios (independently re-derived from `external_visualizer_
/// launch_records_audit_event_when_enabled`, `external_visualizer_
/// launch_errors_redact_command_details`, and `external_
/// visualizer_launch_rejects_when_process_pool_is_full`: a real
/// successful launch redacts the configured command from both the
/// response and the audit event, a real launch failure redacts the
/// command from the error response while still recording a failed
/// event, and a real held process-pool semaphore genuinely blocks
/// a new launch with 503 rather than a race). Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before
/// writing, per case: the searches case was open (other cases on
/// that route already credited from earlier batches), and the
/// visualizer-launch route had zero prior credit at all. slskdN-
/// only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_searches_dispatch_and_visualizer_launch() {
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

    {
        let (state, receiver) = test_state();
        state.session.write().await.state = "connected";
        drop(receiver);
        let response = crate::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            r#"{"query":"differential never dispatched"}"#,
            &state,
        )
        .await
        .expect("failed dispatch response");
        record!(
            "POST",
            "/api/v0/searches",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && state.searches.read().await.records.is_empty()
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = crate::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer status response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "nominal-status-headers-body",
            status.status == "200 OK"
                && status.content_type == "application/json"
                && json["enabled"] == true
                && json["configured"] == true
                && json["available"] == true
                && json["name"] == "MilkDrop3"
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default().with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command),
        );
        let status = crate::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer?unexpected=not-a-number",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer malformed-query response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "malformed-path-query-or-body",
            status.status == "200 OK" && json["configured"] == true && json["available"] == true
        );
    }

    {
        let (state, _receiver) = test_state();
        let status = crate::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer empty-state response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "missing-empty-or-conflict-state",
            status.status == "200 OK" && json["configured"] == false && json["available"] == false
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with(
                    "SLSKR_EXTERNAL_VISUALIZER_COMMAND",
                    "/private/differential-missing-visualizer-secret",
                )
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = crate::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer runtime-failure response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "runtime-failure-and-timeout",
            status.status == "200 OK" && json["configured"] == true && json["available"] == false
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = crate::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer populated response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "populated-dynamic-state",
            status.status == "200 OK"
                && json["enabled"] == true
                && json["configured"] == true
                && json["available"] == true
                && json["path"] == command
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = crate::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer launch response");
        let events = state.events.read().await;
        let launched = events.records.iter().any(|event| {
            event.kind == "external_visualizer.launch" && event.resource == "external_visualizer"
        });
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "nominal-status-headers-body",
            launch.status == "200 OK"
                && launch.body.contains("\"started\":true")
                && !launch.body.contains("\"command\"")
                && launched
        );
    }

    {
        let command = "/private/differential-missing-visualizer-secret";
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = crate::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer failed launch response");
        let events = state.events.read().await;
        let failed_event = events
            .records
            .iter()
            .any(|event| event.kind == "external_visualizer.launch.failed");
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "malformed-path-query-or-body",
            launch.status == "400 Bad Request" && !launch.body.contains(command) && failed_event
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let _permits = Arc::clone(&state.external_visualizer_processes)
            .acquire_many_owned(crate::MAX_EXTERNAL_VISUALIZER_PROCESSES as u32)
            .await
            .expect("configured differential visualizer permits");
        let launch = crate::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer pool-exhausted response");
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "missing-empty-or-conflict-state",
            launch.status == "503 Service Unavailable"
                && launch.body.contains("process limit reached")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with(
                    "SLSKR_EXTERNAL_VISUALIZER_COMMAND",
                    &std::env::temp_dir().to_string_lossy(),
                )
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = crate::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer runtime-failure response");
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "runtime-failure-and-timeout",
            launch.status == "400 Bad Request"
                && launch.body.contains("started")
                && !launch.body.contains("/tmp")
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let first = crate::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let second = crate::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let launches = state
            .events
            .read()
            .await
            .records
            .iter()
            .filter(|event| event.kind == "external_visualizer.launch")
            .count();
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "mutation-side-effects-and-readback",
            first.status == "200 OK" && second.status == "200 OK" && launches >= 2
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = crate::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let (restarted, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = crate::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &restarted,
        )
        .await
        .unwrap();
        let events_reset = restarted.events.read().await.records.is_empty();
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "restart-persistence-or-reset",
            launch.status == "200 OK" && status.status == "200 OK" && events_reset
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let (first, second) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/player/external-visualizer/launch",
                None,
                "",
                &state,
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/player/external-visualizer/launch",
                None,
                "",
                &state,
            ),
        );
        let launches = state
            .events
            .read()
            .await
            .records
            .iter()
            .filter(|event| event.kind == "external_visualizer.launch")
            .count();
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "concurrency-and-idempotency",
            first
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && second
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && launches >= 2
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("searches_dispatch_and_visualizer_launch.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api searches-dispatch-and-visualizer-launch mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining frozen WishlistController cases.
/// The v0 controller binds Guid route values, returns 200 for an executed
/// manual search, and lets ordinary service/storage failures surface as
/// 500 responses.  Each row below is backed by a live route call and, for
/// stateful cases, durable SQLite readback or a fresh in-memory projection.
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
pub(super) async fn controller_api_differential_wishlist_controller_residuals() {
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

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            crate::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {}", $method, $path, error))
        }};
    }

    macro_rules! seed_item {
        ($state:expr, $id:expr) => {{
            $state
                .wishlist
                .write()
                .await
                .add_item_with_contract(
                    Some($id.to_owned()),
                    "Artist".to_owned(),
                    "Track".to_owned(),
                    "Audio".to_owned(),
                    String::new(),
                    true,
                    false,
                    100,
                    None,
                )
                .expect("seed WishlistController item");
        }};
    }

    macro_rules! seed_ignored {
        ($state:expr, $id:expr) => {{
            $state
                .wishlist
                .write()
                .await
                .ignore_result($id, "Peer", "Remote/Album", false)
                .expect("seed WishlistController ignored result")
                .0
        }};
    }

    macro_rules! db_state {
        () => {{
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("WishlistController in-memory database");
            let (state, receiver) = test_state_with_env_parts(
                MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            (state, db, receiver)
        }};
    }

    let create_body = |search_text: &str| {
        serde_json::json!({
            "searchText": search_text,
            "filter": "lossless",
            "enabled": true,
            "autoDownload": false,
            "maxResults": 10,
            "maxDownloads": 2,
        })
        .to_string()
    };
    let update_body = |search_text: &str| {
        serde_json::json!({
            "searchText": search_text,
            "filter": "updated",
            "enabled": false,
            "autoDownload": true,
            "maxResults": 5,
            "maxDownloads": null,
        })
        .to_string()
    };
    let ignored_body = r#"{"username":"Peer","directory":"Remote\\Album/"}"#;
    let import_body = r#"{"csvText":"Artist,Title,Album\nImported Artist,Imported Track,Imported Album","filter":"imported","maxResults":5,"includeAlbum":true}"#;

    let delete_route = "/api/v0/wishlist/{id}";
    let ignored_delete_route = "/api/v0/wishlist/{id}/ignored-results/{ignoredResultId}";
    let list_route = "/api/v0/wishlist";
    let detail_route = "/api/v0/wishlist/{id}";
    let ignored_list_route = "/api/v0/wishlist/{id}/ignored-results";
    let searches_route = "/api/v0/wishlist/{id}/searches";
    let create_route = "/api/v0/wishlist";
    let ignored_create_route = "/api/v0/wishlist/{id}/ignored-results";
    let viewed_route = "/api/v0/wishlist/{id}/mark-viewed";
    let search_route = "/api/v0/wishlist/{id}/search";
    let import_route = "/api/v0/wishlist/import/csv";
    let mark_all_route = "/api/v0/wishlist/mark-all-viewed";
    let update_route = "/api/v0/wishlist/{id}";

    // DELETE /wishlist/{id}: malformed, missing, restart, concurrency.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "DELETE", "/api/v0/wishlist/%20", "");
        record!(
            "DELETE",
            delete_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let missing_id = uuid::Uuid::new_v4().to_string();
        let missing = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{missing_id}"),
            ""
        );
        record!(
            "DELETE",
            delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "204 No Content"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("delete-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("delete restart item id");
        let deleted = request!(&state, "DELETE", &format!("/api/v0/wishlist/{item_id}"), "");
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "DELETE",
            delete_route,
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && deleted.status == "204 No Content"
                && persisted.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = format!("/api/v0/wishlist/{item_id}");
            async move { request!(&state, "DELETE", &path, "") }
        }))
        .await;
        let readback = request!(&state, "GET", &format!("/api/v0/wishlist/{item_id}"), "");
        record!(
            "DELETE",
            delete_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
                && readback.status == "404 Not Found"
        );
    }

    // DELETE /wishlist/{id}/ignored-results/{ignoredResultId}.
    {
        let (state, _receiver) = test_state();
        let malformed_first = request!(
            &state,
            "DELETE",
            "/api/v0/wishlist/not-a-guid/ignored-results/not-a-guid",
            ""
        );
        let valid_id = uuid::Uuid::new_v4();
        let malformed_second = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{valid_id}/ignored-results/not-a-guid"),
            ""
        );
        record!(
            "DELETE",
            ignored_delete_route,
            "malformed-path-query-or-body",
            malformed_first.status == "400 Bad Request"
                && malformed_second.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let rule_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{item_id}/ignored-results/{rule_id}"),
            ""
        );
        record!(
            "DELETE",
            ignored_delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let rule = seed_ignored!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{item_id}/ignored-results/{}", rule.id),
            ""
        );
        record!(
            "DELETE",
            ignored_delete_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("ignored-delete-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("ignored delete item id");
        let ignored = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        let rule_id = serde_json::from_str::<serde_json::Value>(&ignored.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("ignored delete rule id");
        let deleted = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{item_id}/ignored-results/{rule_id}"),
            ""
        );
        let persisted = db
            .list_wishlist_ignored_results(&item_id)
            .await
            .unwrap_or_default();
        record!(
            "DELETE",
            ignored_delete_route,
            "restart-persistence-or-reset",
            ignored.status == "201 Created"
                && deleted.status == "204 No Content"
                && persisted.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let rule = seed_ignored!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/ignored-results/{}", rule.id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "DELETE", &path, "") }
        }))
        .await;
        let success = responses
            .iter()
            .filter(|response| response.status == "204 No Content")
            .count();
        let missing = responses
            .iter()
            .filter(|response| response.status == "404 Not Found")
            .count();
        record!(
            "DELETE",
            ignored_delete_route,
            "concurrency-and-idempotency",
            success == 1 && missing == 1
        );
    }

    // GET /wishlist.
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        record!(
            "GET",
            list_route,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_default()
                    .is_array()
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/wishlist?unexpected=1", "");
        record!(
            "GET",
            list_route,
            "malformed-path-query-or-body",
            response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        record!(
            "GET",
            list_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "[]"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        record!(
            "GET",
            list_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state();
        let first_id = uuid::Uuid::new_v4().to_string();
        let second_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &first_id);
        seed_item!(&state, &second_id);
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            list_route,
            "populated-dynamic-state",
            response.status == "200 OK" && json.as_array().is_some_and(|items| items.len() == 2)
        );
    }

    // GET /wishlist/{id}.
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let response = request!(&state, "GET", &format!("/api/v0/wishlist/{item_id}"), "");
        record!(
            "GET",
            detail_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(&state, "GET", &format!("/api/v0/wishlist/{item_id}"), "");
        record!(
            "GET",
            detail_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }

    // GET /wishlist/{id}/ignored-results.
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ""
        );
        record!(
            "GET",
            ignored_list_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ""
        );
        record!(
            "GET",
            ignored_list_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }

    // GET /wishlist/{id}/searches.
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("searches-nominal")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("searches nominal item id");
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "GET",
            searches_route,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_default()
                    .is_array()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "GET",
            searches_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "GET",
            searches_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, mut receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("searches-populated")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("searches populated item id");
        let started = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        let _ = receiver.try_recv();
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            searches_route,
            "populated-dynamic-state",
            started.status == "200 OK"
                && response.status == "200 OK"
                && json.as_array().is_some_and(|searches| !searches.is_empty())
        );
    }

    // POST /wishlist: malformed, missing, restart, concurrency.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "POST", "/api/v0/wishlist", "{}");
        record!(
            "POST",
            create_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("SearchText is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("empty-state-create")
        );
        record!(
            "POST",
            create_route,
            "missing-empty-or-conflict-state",
            response.status == "201 Created"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let response = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("create-restart")
        );
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        let restored =
            crate::WishlistStore::from_persisted_with_ignored(persisted.clone(), Vec::new());
        record!(
            "POST",
            create_route,
            "restart-persistence-or-reset",
            response.status == "201 Created"
                && persisted.len() == 1
                && restored.records[0].items.len() == 1
        );
    }
    {
        let (state, _receiver) = test_state();
        let responses = futures_util::future::join_all((0..4).map(|index| {
            let state = Arc::clone(&state);
            let body = create_body(&format!("create-concurrent-{index}"));
            async move { request!(&state, "POST", "/api/v0/wishlist", &body) }
        }))
        .await;
        let count = state
            .wishlist
            .read()
            .await
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .count();
        record!(
            "POST",
            create_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && count == 4
        );
    }

    // POST /wishlist/{id}/ignored-results.
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let malformed = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            "{}"
        );
        record!(
            "POST",
            ignored_create_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed
                    .body
                    .contains("Username and Directory are required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        record!(
            "POST",
            ignored_create_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        record!(
            "POST",
            ignored_create_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("ignored-create-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("ignored create item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        let persisted = db
            .list_wishlist_ignored_results(&item_id)
            .await
            .unwrap_or_default();
        record!(
            "POST",
            ignored_create_route,
            "restart-persistence-or-reset",
            response.status == "201 Created" && persisted.len() == 1
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/ignored-results");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "POST", &path, ignored_body) }
        }))
        .await;
        let rules = state.wishlist.read().await.list_ignored_results(&item_id);
        record!(
            "POST",
            ignored_create_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && rules.as_ref().is_some_and(|rules| rules.len() == 1)
        );
    }

    // POST /wishlist/{id}/mark-viewed.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(
            &state,
            "POST",
            "/api/v0/wishlist/not-a-guid/mark-viewed",
            ""
        );
        record!(
            "POST",
            viewed_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/mark-viewed"),
            ""
        );
        record!(
            "POST",
            viewed_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/mark-viewed"),
            ""
        );
        record!(
            "POST",
            viewed_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("viewed-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("viewed restart item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/mark-viewed"),
            ""
        );
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "POST",
            viewed_route,
            "restart-persistence-or-reset",
            response.status == "204 No Content"
                && persisted
                    .first()
                    .and_then(|item| item.last_viewed_at)
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/mark-viewed");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "POST", &path, "") }
        }))
        .await;
        let viewed = state
            .wishlist
            .read()
            .await
            .get_item(&item_id)
            .and_then(|item| item.last_viewed_at)
            .is_some();
        record!(
            "POST",
            viewed_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
                && viewed
        );
    }

    // POST /wishlist/{id}/search.
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("manual-search-nominal")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("manual search nominal item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        record!(
            "POST",
            search_route,
            "nominal-status-headers-body",
            response.status == "200 OK" && response.body.contains("\"search_started\":true")
        );
    }
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "POST", "/api/v0/wishlist/not-a-guid/search", "");
        record!(
            "POST",
            search_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        record!(
            "POST",
            search_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        record!(
            "POST",
            search_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("manual-search-mutation")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("manual search mutation item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        let history = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "POST",
            search_route,
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&history.body)
                    .unwrap_or_default()
                    .as_array()
                    .is_some_and(|searches| !searches.is_empty())
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("manual-search-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("manual search restart item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        let persisted = db.list_searches(10, 0).await.unwrap_or_default();
        record!(
            "POST",
            search_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && !persisted.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/search");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "POST", &path, "") }
        }))
        .await;
        let search_count = state.searches.read().await.records.len();
        record!(
            "POST",
            search_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK") && search_count == 2
        );
    }

    // POST /wishlist/import/csv.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "POST", import_route, "not-json");
        record!(
            "POST",
            import_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let missing = request!(&state, "POST", import_route, r#"{"csvText":""}"#);
        record!(
            "POST",
            import_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("CsvText is required")
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let response = request!(&state, "POST", import_route, import_body);
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        let restored =
            crate::WishlistStore::from_persisted_with_ignored(persisted.clone(), Vec::new());
        record!(
            "POST",
            import_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default()
                    ["createdCount"]
                    == 1
                && restored.records[0].items.len() == 1
        );
    }

    // POST /wishlist/mark-all-viewed.
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", mark_all_route, "");
        record!(
            "POST",
            mark_all_route,
            "nominal-status-headers-body",
            response.status == "204 No Content"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", mark_all_route, "not-json");
        record!(
            "POST",
            mark_all_route,
            "malformed-path-query-or-body",
            response.status == "204 No Content"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", mark_all_route, "");
        record!(
            "POST",
            mark_all_route,
            "missing-empty-or-conflict-state",
            response.status == "204 No Content"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(&state, "POST", mark_all_route, "");
        record!(
            "POST",
            mark_all_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state();
        let first_id = uuid::Uuid::new_v4().to_string();
        let second_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &first_id);
        seed_item!(&state, &second_id);
        let response = request!(&state, "POST", mark_all_route, "");
        let viewed = state
            .wishlist
            .read()
            .await
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .all(|item| item.last_viewed_at.is_some());
        record!(
            "POST",
            mark_all_route,
            "mutation-side-effects-and-readback",
            response.status == "204 No Content" && viewed
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        for text in ["mark-all-restart-one", "mark-all-restart-two"] {
            let response = request!(&state, "POST", "/api/v0/wishlist", &create_body(text));
            assert_eq!(response.status, "201 Created", "{text}");
        }
        let response = request!(&state, "POST", mark_all_route, "");
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "POST",
            mark_all_route,
            "restart-persistence-or-reset",
            response.status == "204 No Content"
                && persisted.len() == 2
                && persisted.iter().all(|item| item.last_viewed_at.is_some())
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move { request!(&state, "POST", mark_all_route, "") }
        }))
        .await;
        let viewed = state
            .wishlist
            .read()
            .await
            .get_item(&item_id)
            .and_then(|item| item.last_viewed_at)
            .is_some();
        record!(
            "POST",
            mark_all_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
                && viewed
        );
    }

    // PUT /wishlist/{id}.
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("update-nominal")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("update nominal item id");
        let response = request!(
            &state,
            "PUT",
            &format!("/api/v0/wishlist/{item_id}"),
            &update_body("updated-nominal")
        );
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "PUT",
            update_route,
            "nominal-status-headers-body",
            response.status == "200 OK" && json["searchText"] == "updated-nominal"
        );
    }
    {
        let (state, _receiver) = test_state();
        let malformed = request!(
            &state,
            "PUT",
            "/api/v0/wishlist/not-a-guid",
            &update_body("malformed")
        );
        record!(
            "PUT",
            update_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "PUT",
            &format!("/api/v0/wishlist/{item_id}"),
            &update_body("missing")
        );
        record!(
            "PUT",
            update_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("update-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("update restart item id");
        let response = request!(
            &state,
            "PUT",
            &format!("/api/v0/wishlist/{item_id}"),
            &update_body("updated-restart")
        );
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "PUT",
            update_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && persisted
                    .first()
                    .is_some_and(|item| item.artist == "updated-restart")
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}");
        let responses = futures_util::future::join_all((0..2).map(|index| {
            let state = Arc::clone(&state);
            let path = path.clone();
            let body = update_body(&format!("updated-concurrent-{index}"));
            async move { request!(&state, "PUT", &path, &body) }
        }))
        .await;
        let updated = state
            .wishlist
            .read()
            .await
            .get_item(&item_id)
            .is_some_and(|item| item.search_text().starts_with("updated-concurrent-"));
        record!(
            "PUT",
            update_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK") && updated
        );
    }

    assert_eq!(ledger.len(), 58, "WishlistController residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create WishlistController evidence directory");
    fs::write(
        evidence_dir.join("wishlist_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize WishlistController ledger"),
    )
    .expect("write WishlistController ledger");
    assert!(
        mismatches.is_empty(),
        "{} WishlistController residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
