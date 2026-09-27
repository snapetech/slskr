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
async fn controller_api_differential_discovery_graph_and_opinions() {
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

    let graph = super::route_http_request(
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

    let invalid_opinion = super::route_http_request(
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

    let missing_delete = super::route_http_request(
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

    let valid_opinion = super::route_http_request(
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
    let deleted = super::route_http_request("DELETE", &delete_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{delete_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "mutation-side-effects-and-readback",
        deleted.status == "204 No Content"
    );

    let contact = super::route_http_request(
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
        let response = super::route_http_request("POST", &route, None, "", &state)
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
async fn controller_api_differential_opinion_open_cases() {
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

    let malformed_list = super::route_http_request(
        "GET",
        "/api/v0/opinions?limit=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/opinions",
        "malformed-path-query-or-body",
        malformed_list.status == "400 Bad Request"
    );

    let empty_list = super::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .unwrap();
    let empty_list_json =
        serde_json::from_str::<serde_json::Value>(&empty_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/opinions",
        "missing-empty-or-conflict-state",
        empty_list.status == "200 OK" && empty_list_json == serde_json::json!([])
    );

    let summary = super::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track&subjectId=empty-track",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "nominal-status-headers-body",
        summary.status == "200 OK"
            && summary_json["subjectType"] == "Track"
            && summary_json["subjectId"] == "empty-track"
    );

    let missing_summary =
        super::route_http_request("GET", "/api/v0/opinions/summary", None, "", &state)
            .await
            .unwrap();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "missing-empty-or-conflict-state",
        missing_summary.status == "400 Bad Request"
    );

    let posted = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"opinion-open-populated","issuer":"open-cases","subjectType":"Track","subjectId":"summary-track","kind":"Like","strength":0.8,"confidence":0.75}"#,
        &state,
    )
    .await
    .unwrap();
    let posted_json = serde_json::from_str::<serde_json::Value>(&posted.body).unwrap_or_default();
    let populated_summary = super::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track&subjectId=summary-track",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let populated_summary_json =
        serde_json::from_str::<serde_json::Value>(&populated_summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "populated-dynamic-state",
        posted.status == "200 OK"
            && posted_json["id"] == "opinion-open-populated"
            && populated_summary.status == "200 OK"
            && populated_summary_json["total"] == 1
            && populated_summary_json["positive"] == 1
            && populated_summary_json["opinions"].as_array().map(Vec::len) == Some(1)
    );

    let summary_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion summary runtime database");
    let (summary_runtime_state, _summary_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(summary_db.clone()),
    );
    summary_db.close_for_test().await;
    let summary_runtime = super::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track&subjectId=runtime-track",
        None,
        "",
        &summary_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "runtime-failure-and-timeout",
        summary_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&summary_runtime.body).is_ok()
    );

    let list_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion list runtime database");
    let (list_runtime_state, _list_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(list_db.clone()),
    );
    list_db.close_for_test().await;
    let list_runtime =
        super::route_http_request("GET", "/api/v0/opinions", None, "", &list_runtime_state)
            .await
            .unwrap();
    record!(
        "GET",
        "/api/v0/opinions",
        "runtime-failure-and-timeout",
        list_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&list_runtime.body)
                .is_ok_and(|value| value.is_array())
    );

    let missing_post = super::route_http_request("POST", "/api/v0/opinions", None, "{}", &state)
        .await
        .unwrap();
    record!(
        "POST",
        "/api/v0/opinions",
        "missing-empty-or-conflict-state",
        missing_post.status == "400 Bad Request"
    );

    let post_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion post runtime database");
    let (post_runtime_state, _post_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(post_db.clone()),
    );
    post_db.close_for_test().await;
    let post_runtime = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"opinion-runtime","issuer":"runtime","subjectType":"Track","subjectId":"runtime-track","kind":"Like","strength":1,"confidence":1}"#,
        &post_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/opinions",
        "runtime-failure-and-timeout",
        post_runtime.status == "200 OK"
    );

    let (post_restart_state, _post_restart_receiver) = test_state();
    let post_restart =
        super::route_http_request("GET", "/api/v0/opinions", None, "", &post_restart_state)
            .await
            .unwrap();
    let post_restart_json =
        serde_json::from_str::<serde_json::Value>(&post_restart.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/opinions",
        "restart-persistence-or-reset",
        post_restart.status == "200 OK" && post_restart_json == serde_json::json!([])
    );

    let concurrent_bodies: Vec<String> = (0..4)
        .map(|index| {
            format!(
                r#"{{"id":"opinion-concurrent-{index}","issuer":"concurrent","subjectType":"Track","subjectId":"concurrent-track-{index}","kind":"Like","strength":1,"confidence":1}}"#
            )
        })
        .collect();
    let concurrent_posts = futures_util::future::join_all(
        concurrent_bodies
            .iter()
            .map(|body| super::route_http_request("POST", "/api/v0/opinions", None, body, &state)),
    )
    .await;
    let concurrent_list = super::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .unwrap();
    let concurrent_list_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_list.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/opinions",
        "concurrency-and-idempotency",
        concurrent_posts.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_list_json
            .as_array()
            .is_some_and(|opinions| opinions.len() == 5)
    );

    let malformed_delete = super::route_http_request(
        "DELETE",
        "/api/v0/opinions/opinion-open-populated/extra",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );

    let delete_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion delete runtime database");
    let (delete_runtime_state, _delete_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(delete_db.clone()),
    );
    let delete_runtime_post = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"opinion-delete-runtime","issuer":"runtime","subjectType":"Track","subjectId":"delete-runtime","kind":"Like","strength":1,"confidence":1}"#,
        &delete_runtime_state,
    )
    .await
    .unwrap();
    delete_db.close_for_test().await;
    let delete_runtime = super::route_http_request(
        "DELETE",
        "/api/v0/opinions/opinion-delete-runtime",
        None,
        "",
        &delete_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "runtime-failure-and-timeout",
        delete_runtime_post.status == "200 OK" && delete_runtime.status == "204 No Content"
    );

    let (delete_restart_state, _delete_restart_receiver) = test_state();
    let delete_restart = super::route_http_request(
        "DELETE",
        "/api/v0/opinions/opinion-open-populated",
        None,
        "",
        &delete_restart_state,
    )
    .await
    .unwrap();
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "restart-persistence-or-reset",
        delete_restart.status == "404 Not Found"
    );

    let delete_ids: Vec<String> = (0..4)
        .map(|index| format!("opinion-delete-concurrent-{index}"))
        .collect();
    for id in &delete_ids {
        let body = format!(
            r#"{{"id":"{id}","issuer":"delete-concurrent","subjectType":"Track","subjectId":"{id}","kind":"Like","strength":1,"confidence":1}}"#
        );
        super::route_http_request("POST", "/api/v0/opinions", None, &body, &state)
            .await
            .unwrap();
    }
    let concurrent_deletes = futures_util::future::join_all(delete_ids.iter().map(|id| {
        let path = format!("/api/v0/opinions/{id}");
        let state = Arc::clone(&state);
        async move { super::route_http_request("DELETE", &path, None, "", &state).await }
    }))
    .await;
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "concurrency-and-idempotency",
        concurrent_deletes.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("opinion_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api opinion open-case mismatches:\n{}",
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
async fn controller_api_differential_discovery_graph_edge_contracts() {
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
        super::route_http_request("POST", "/api/v0/discovery-graph/extra", None, "{}", &state)
            .await
            .expect("malformed discovery graph route");
    record!(
        "malformed-path-query-or-body",
        malformed.status == "404 Not Found"
    );

    let empty = super::route_http_request("POST", "/api/v0/discovery-graph", None, "{}", &state)
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("discovery graph runtime-failure database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default(),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime = super::route_http_request(
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
    let restarted = super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/discovery-graph", None, "{}", &state),
        super::route_http_request("POST", "/api/v0/discovery-graph", None, "{}", &state),
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
async fn controller_api_differential_deterministic_openapi_mutations() {
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
        let response = super::route_http_request("PUT", path, None, "", &state)
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

    let destination = super::route_http_request(
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

    let announce = super::route_http_request("POST", "/api/v0/dht/announce", None, "", &state)
        .await
        .expect("dht announce");
    record!(
        "POST",
        "/api/v0/dht/announce",
        "missing-empty-or-conflict-state",
        announce.status == "400 Bad Request"
            && announce.body == r#"{"error":"Not beacon capable"}"#
    );

    let discover = super::route_http_request("POST", "/api/v0/dht/discover", None, "", &state)
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
        let response = super::route_http_request("POST", path, None, "", &state)
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

    let profile = super::route_http_request(
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

    super::route_http_request(
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
            super::route_http_request(method, path, None, r#"{"status":"Resolved"}"#, &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "nominal-status-headers-body",
            response.status == "204 No Content" && response.body.is_empty()
        );
        if path == "/api/v0/nowplaying" {
            let cleared = super::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
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

    let blocked = super::route_http_request(
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
    let blocklist = super::route_http_request("GET", "/api/v0/overlay/blocklist", None, "", &state)
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

    let unblocked = super::route_http_request(
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
async fn controller_api_differential_versioned_openapi_validation_rejections() {
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
        let response = super::route_http_request(method, path, None, body, &state)
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
async fn controller_api_differential_versioned_openapi_large_dtos() {
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

    let multisource_test = super::route_http_request(
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

    let bloom = super::route_http_request(
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

    let songid = super::route_http_request(
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

    let taste = super::route_http_request(
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
        let response = super::route_http_request("POST", path, None, invalid_work_ref, &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    let start = super::route_http_request(
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

    let stop = super::route_http_request("POST", "/api/v0/portforwarding/stop/1", None, "", &state)
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
    let index_sync_fixture = super::route_http_request(
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

    let unsafe_decision = super::route_http_request(
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

    let decision = super::route_http_request(
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
    super::route_http_request(
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

    let joined = super::route_http_request(
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
        super::route_http_request("POST", &empty_backfill_route, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{empty_backfill_route}: {error}"));
    record!(
        "POST",
        "/api/v0/podcore/backfill/{podId}/sync",
        "malformed-path-query-or-body",
        empty_backfill.status == "400 Bad Request"
    );

    let last_seen_route = format!("/api/v0/podcore/backfill/{pod_id}/general/last-seen");
    let last_seen = super::route_http_request("PUT", &last_seen_route, None, "1", &state)
        .await
        .unwrap_or_else(|error| panic!("{last_seen_route}: {error}"));
    record!(
        "PUT",
        "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
        "nominal-status-headers-body",
        last_seen.status == "200 OK" && last_seen.body.is_empty()
    );

    let verified = super::route_http_request(
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

    let evidence = super::route_http_request(
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
    let opinion = super::route_http_request(
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
    let removed = super::route_http_request("DELETE", &removed_route, None, "", &state)
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
    let repeated_removed = super::route_http_request("DELETE", &removed_route, None, "", &state)
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
async fn controller_api_differential_versioned_auxiliary_mutations() {
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
        let response = super::route_http_request(
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
        super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("no scan to cancel");
    record!(
        "DELETE",
        "/api/v0/shares",
        "missing-empty-or-conflict-state",
        shares_delete_before.status == "404 Not Found"
    );

    let scan = super::route_http_request("PUT", "/api/v0/shares", None, "", &state)
        .await
        .expect("start share scan");
    record!(
        "PUT",
        "/api/v0/shares",
        "nominal-status-headers-body",
        scan.status == "200 OK"
    );

    let cancelled = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .expect("cancel completed scan");
    record!(
        "DELETE",
        "/api/v0/shares",
        "mutation-side-effects-and-readback",
        cancelled.status == "404 Not Found"
    );

    let invite = super::route_http_request(
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
    let imported = super::route_http_request(
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

    let duplicate = super::route_http_request(
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

    let invalid_content = super::route_http_request(
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

    let valid_content = super::route_http_request(
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

    let group = super::route_http_request(
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

    let profile = super::route_http_request(
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

    let verdict = super::route_http_request(
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
    let accepted = super::route_http_request(
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

    let invalid_type = super::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[42]}"#,
        &warm_state,
    )
    .await
    .expect("reject non-string release id");
    let oversized = super::route_http_request(
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

/// Bulk differential proof crediting 5 release-radar routes' cases,
/// independently re-derived from `versioned_release_radar_matches_
/// native_state_and_result_contracts`'s real SongID-confirmation
/// gate, deduplication, and notification-routing checks. slskdN-only
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
async fn controller_api_differential_release_radar() {
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
    let artist_id = "00000000-0000-4000-8000-000000000201";

    let subscription = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        &format!(
            r#"{{"artistId":"{artist_id}","artistName":"Differential Artist","scope":"trusted","enabled":true,"mutedReleaseGroupIds":[],"createdAt":"2026-01-01T00:00:00Z"}}"#
        ),
        &state,
    )
    .await
    .expect("create subscription");
    let subscription_json =
        serde_json::from_str::<serde_json::Value>(&subscription.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "nominal-status-headers-body",
        subscription.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "mutation-side-effects-and-readback",
        subscription_json["id"] == format!("artist-radar:{artist_id}")
            && subscription_json["artistName"] == "Differential Artist"
            && subscription_json["createdAt"] == "2026-01-01T00:00:00+00:00"
    );

    let subscriptions = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        "",
        &state,
    )
    .await
    .expect("list subscriptions");
    let subscriptions_json =
        serde_json::from_str::<serde_json::Value>(&subscriptions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "nominal-status-headers-body",
        subscriptions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "populated-dynamic-state",
        subscriptions_json == serde_json::json!([subscription_json])
    );

    let rejected = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject unconfirmed observation");
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "malformed-path-query-or-body",
        rejected.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&rejected.body).unwrap_or_default()
                == serde_json::json!({
                    "accepted": false,
                    "notifications": [],
                    "rejectionReason": "Observation is not SongID-confirmed.",
                })
    );

    let observation_body = format!(
        r#"{{"artistId":"{artist_id}","recordingId":"00000000-0000-4000-8000-000000000202","releaseId":"00000000-0000-4000-8000-000000000203","releaseGroupId":"00000000-0000-4000-8000-000000000204","sourceRealm":"realm","sourceActor":"actor","songIdConfirmed":true,"confidence":1,"workRef":{{"id":"00000000-0000-4000-8000-000000000202","type":"recording","domain":"music","externalIds":{{}},"title":"Track","creator":"Artist","year":2026,"metadata":{{}},"attributedTo":"actor","published":"2026-01-01T00:00:00Z"}},"observedAt":"2026-01-01T00:00:00Z"}}"#
    );
    let observation = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        &observation_body,
        &state,
    )
    .await
    .expect("accept confirmed observation");
    let observation_json =
        serde_json::from_str::<serde_json::Value>(&observation.body).unwrap_or_default();
    let notification = observation_json["notifications"][0].clone();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "nominal-status-headers-body",
        observation.status == "200 OK" && observation_json["accepted"] == true
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "mutation-side-effects-and-readback",
        notification["subscriptionId"] == format!("artist-radar:{artist_id}")
            && notification["artistId"] == artist_id
            && notification["firstSeenAt"] == "2026-01-01T00:00:00+00:00"
            && notification["read"] == false
            && notification["workRef"]["@context"]
                == serde_json::json!([
                    "https://www.w3.org/ns/activitystreams",
                    "https://w3id.org/federation/workref#"
                ])
    );

    let duplicate = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        &observation_body,
        &state,
    )
    .await
    .expect("deduplicate observation");
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "concurrency-and-idempotency",
        serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap_or_default()
            == serde_json::json!({"accepted": true, "notifications": []})
    );

    let notification_id = notification["id"].as_str().unwrap_or_default().to_owned();
    let notifications = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        None,
        "",
        &state,
    )
    .await
    .expect("list release-radar notifications");
    let notifications_json =
        serde_json::from_str::<serde_json::Value>(&notifications.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        "nominal-status-headers-body",
        notifications.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        "populated-dynamic-state",
        notifications_json
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["id"] == notification_id) })
    );
    let route_route =
        format!("/api/v0/musicbrainz/release-radar/notifications/{notification_id}/routes");
    let route = super::route_http_request(
        "POST",
        &route_route,
        None,
        r#"{"targetPeerIds":[],"podId":"pod","channelId":"channel","senderPeerId":"sender"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let route_json = serde_json::from_str::<serde_json::Value>(&route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "malformed-path-query-or-body",
        route.status == "400 Bad Request"
            && route_json["notificationId"] == notification_id
            && route_json["success"] == false
            && route_json["errorMessage"] == "At least one target peer is required."
            && route_json["targetPeerIds"] == serde_json::json!([])
    );

    let missing_route = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/missing-differential/routes",
        None,
        "{}",
        &state,
    )
    .await
    .expect("missing release-radar notification route");
    let missing_route_json =
        serde_json::from_str::<serde_json::Value>(&missing_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "missing-empty-or-conflict-state",
        missing_route.status == "400 Bad Request"
            && missing_route_json["notificationId"] == "missing-differential"
            && missing_route_json["success"] == false
            && missing_route_json["errorMessage"] == "Notification not found."
    );

    let unavailable_route = super::route_http_request(
        "POST",
        &route_route,
        None,
        r#"{"targetPeerIds":["actor:differential-radar-peer"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let unavailable_route_json =
        serde_json::from_str::<serde_json::Value>(&unavailable_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "runtime-failure-and-timeout",
        unavailable_route.status == "400 Bad Request"
            && unavailable_route_json["notificationId"] == notification_id
            && unavailable_route_json["success"] == false
            && unavailable_route_json["errorMessage"] == "Routing backend is not available."
            && unavailable_route_json["targetPeerIds"]
                == serde_json::json!(["actor:differential-radar-peer"])
    );

    let missing_routes = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/empty-differential/routes",
        None,
        "",
        &state,
    )
    .await
    .expect("empty routes for missing release-radar notification");
    let missing_routes_json =
        serde_json::from_str::<serde_json::Value>(&missing_routes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "missing-empty-or-conflict-state",
        missing_routes.status == "200 OK" && missing_routes_json == serde_json::json!([])
    );

    let routes_list = super::route_http_request("GET", &route_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let routes_list_json =
        serde_json::from_str::<serde_json::Value>(&routes_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "nominal-status-headers-body",
        routes_list.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "populated-dynamic-state",
        routes_list_json.as_array().is_some_and(|attempts| {
            attempts.len() == 2
                && attempts
                    .iter()
                    .any(|attempt| attempt["id"] == route_json["id"])
                && attempts
                    .iter()
                    .any(|attempt| attempt["id"] == unavailable_route_json["id"])
        })
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "mutation-side-effects-and-readback",
        routes_list_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["id"] == unavailable_route_json["id"]
                    && attempt["notificationId"] == notification_id
                    && attempt["errorMessage"] == "Routing backend is not available."
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("release_radar.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api release-radar mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the library-health portion of
/// `compatibility_projections_use_local_state_for_library_jobs_and_
/// discovery` (10 registered routes; the source test's remaining
/// lidarr/musicbrainz/wishlist/discovery-graph/listening-party
/// sections mostly hit slskR-internal bare `/api/...` compat-shell
/// routes with no registry entry in either target, or routes already
/// credited via other differentials -- not revisited here).
/// Independently re-derived from the same real seeded-library-item
/// health/dashboard/scan/remediation checks. slskdN-only (confirmed
/// against the frozen registry route-by-route; `/api/library/items`
/// itself has no registry entry and is used only as fixture setup).
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
async fn controller_api_differential_library_health() {
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

    super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Untitled","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create incomplete library item fixture");
    super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Known","title":"Release","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create complete library item fixture");

    let health = super::route_http_request("GET", "/api/library/health/issues", None, "", &state)
        .await
        .expect("library issues");
    let health_json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues",
        "nominal-status-headers-body",
        health.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues",
        "populated-dynamic-state",
        health_json["totalCount"] == 1
            && health_json["issues"][0]["type"] == "MissingMetadata"
            && health_json["issues"][0]["metadata"]["missingField"] == "missing_artist"
            && health_json["filter"]["limit"] == 100
            && health_json["filter"]["offset"] == 0
    );

    let by_artist = super::route_http_request(
        "GET",
        "/api/library/health/issues/by-artist",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by artist");
    let by_artist_json =
        serde_json::from_str::<serde_json::Value>(&by_artist.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-artist",
        "nominal-status-headers-body",
        by_artist.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-artist",
        "populated-dynamic-state",
        by_artist_json["groups"]
            .as_array()
            .is_some_and(Vec::is_empty)
            && by_artist_json["totalArtists"] == 0
    );

    let summary = super::route_http_request(
        "GET",
        "/api/library/health/summary?LibraryPath=%2Fmusic",
        None,
        "",
        &state,
    )
    .await
    .expect("library health summary");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/summary",
        "nominal-status-headers-body",
        summary.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/summary",
        "populated-dynamic-state",
        summary_json["libraryPath"] == "/music"
            && summary_json["totalIssues"] == 1
            && summary_json["issuesOpen"] == 1
    );

    let dashboard = super::route_http_request(
        "GET",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=1&issueLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("library health dashboard");
    let dashboard_json =
        serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/dashboard",
        "nominal-status-headers-body",
        dashboard.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/dashboard",
        "populated-dynamic-state",
        dashboard_json["summary"]["libraryPath"] == "/music"
            && dashboard_json["issuesByType"][0]["type"] == "MissingMetadata"
            && dashboard_json["issuesByArtist"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && dashboard_json["issues"].as_array().map(Vec::len) == Some(1)
            && dashboard_json["totalIssues"] == 1
    );

    let by_codec = super::route_http_request(
        "GET",
        "/api/library/health/issues/by-codec",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by codec");
    let by_codec_json =
        serde_json::from_str::<serde_json::Value>(&by_codec.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-codec",
        "nominal-status-headers-body",
        by_codec.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-codec",
        "populated-dynamic-state",
        by_codec_json["groups"].as_array().map(Vec::len) == Some(1)
            && by_codec_json["groups"][0]["codec"] == "UNKNOWN"
            && by_codec_json["groups"][0]["count"] == 1
            && by_codec_json["groups"][0]["transcodeSuspect"] == 0
            && by_codec_json["totalIssues"] == 1
    );

    let filtered = super::route_http_request(
        "GET",
        "/api/library/health/issues?LibraryPath=%2Fmusic&types=CorruptedFile&severities=Medium&statuses=Detected&Limit=2&Offset=999999",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered library issues");
    let filtered_json =
        serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues",
        "malformed-path-query-or-body",
        filtered.status == "200 OK"
            && filtered_json["totalCount"] == 0
            && filtered_json["filter"]["libraryPath"] == "/music"
            && filtered_json["filter"]["types"][0] == "CorruptedFile"
            && filtered_json["filter"]["severities"][0] == "Medium"
            && filtered_json["filter"]["statuses"][0] == "Detected"
            && filtered_json["filter"]["limit"] == 2
            && filtered_json["filter"]["offset"] == 999999
    );

    let by_type = super::route_http_request(
        "GET",
        "/api/library/health/issues/by-type",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by type");
    let by_type_json = serde_json::from_str::<serde_json::Value>(&by_type.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-type",
        "nominal-status-headers-body",
        by_type.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-type",
        "populated-dynamic-state",
        by_type_json["groups"][0]["type"] == "MissingMetadata" && by_type_json["totalIssues"] == 1
    );

    // The versioned controller exposes the same library-health actions
    // through the frozen v0 route table.  Exercise those actual aliases
    // as separate requests: route-presence alone does not prove that the
    // versioned dispatch reaches the state-backed implementation.
    for (path, route, populated) in [
        (
            "/api/v0/library/health/issues",
            "/api/v0/library/health/issues",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-artist",
            "/api/v0/library/health/issues/by-artist",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-release",
            "/api/v0/library/health/issues/by-release",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-codec",
            "/api/v0/library/health/issues/by-codec",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-type?libraryPath=%2Fmusic",
            "/api/v0/library/health/issues/by-type",
            true,
        ),
        (
            "/api/v0/library/health/summary?LibraryPath=%2Fmusic",
            "/api/v0/library/health/summary",
            true,
        ),
        (
            "/api/v0/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=1&issueLimit=1",
            "/api/v0/library/health/dashboard",
            true,
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        let populated_pass = match route {
            "/api/v0/library/health/issues" => {
                value["totalCount"] == 1
                    && value["issues"][0]["metadata"]["missingField"] == "missing_artist"
            }
            "/api/v0/library/health/issues/by-artist" => {
                value["groups"].as_array().is_some() && value["totalArtists"].as_u64().is_some()
            }
            "/api/v0/library/health/issues/by-release" => {
                value["groups"].as_array().is_some() && value["totalReleases"].as_u64().is_some()
            }
            "/api/v0/library/health/issues/by-codec" => {
                value["groups"].as_array().is_some() && value["totalIssues"] == 1
            }
            "/api/v0/library/health/issues/by-type" => {
                value["groups"].as_array().is_some() && value["totalIssues"] == 1
            }
            "/api/v0/library/health/summary" => {
                value["libraryPath"] == "/music"
                    && value["totalIssues"] == 1
                    && value["issuesOpen"] == 1
            }
            "/api/v0/library/health/dashboard" => {
                value["summary"]["libraryPath"] == "/music"
                    && value["issues"]
                        .as_array()
                        .is_some_and(|issues| issues.len() == 1)
                    && value["totalIssues"] == 1
            }
            _ => false,
        };
        if populated {
            record!("GET", route, "populated-dynamic-state", populated_pass);
        }
    }

    let by_release = super::route_http_request(
        "GET",
        "/api/library/health/issues/by-release",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by release");
    let by_release_json =
        serde_json::from_str::<serde_json::Value>(&by_release.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-release",
        "nominal-status-headers-body",
        by_release.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-release",
        "populated-dynamic-state",
        by_release_json["groups"].as_array().is_some()
            && by_release_json["totalReleases"].as_u64().is_some()
    );

    let mut bad_query_pass = true;
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
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        bad_query_pass &= response.status == "400 Bad Request";
    }
    record!(
        "GET",
        "/api/library/health/summary",
        "malformed-path-query-or-body",
        bad_query_pass
    );
    record!(
        "GET",
        "/api/library/health/dashboard",
        "malformed-path-query-or-body",
        bad_query_pass
    );
    record!(
        "GET",
        "/api/library/health/issues/by-artist",
        "malformed-path-query-or-body",
        bad_query_pass
    );
    record!(
        "GET",
        "/api/library/health/issues/by-release",
        "malformed-path-query-or-body",
        bad_query_pass
    );

    let patched_route = "/api/v0/library/health/issues/lib-1-missing-artist";
    let patched_issue = super::route_http_request(
        "PATCH",
        patched_route,
        None,
        r#"{"artist":"Recovered Artist"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{patched_route}: {error}"));
    record!(
        "PATCH",
        "/api/v0/library/health/issues/{issueId}",
        "nominal-status-headers-body",
        patched_issue.status == "204 No Content"
    );
    record!(
        "PATCH",
        "/api/v0/library/health/issues/{issueId}",
        "mutation-side-effects-and-readback",
        patched_issue.status == "204 No Content"
            && state
                .library
                .read()
                .await
                .get("lib-1")
                .is_some_and(|record| { record.artist == "Recovered Artist" })
    );

    super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Fixable","title":"Kindless","kind":""}"#,
        &state,
    )
    .await
    .expect("create fixable library item fixture");

    let scan = super::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("library scan");
    let scan_json = serde_json::from_str::<serde_json::Value>(&scan.body).unwrap_or_default();
    let active_id = scan_json["scanId"].as_str().unwrap_or_default().to_owned();
    record!(
        "POST",
        "/api/v0/library/health/scans",
        "nominal-status-headers-body",
        scan.status == "200 OK"
            && scan_json["scanId"].as_str().is_some()
            && scan_json["message"] == "Scan started successfully"
    );

    let second_scan = super::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("second concurrent library scan");
    record!(
        "POST",
        "/api/v0/library/health/scans",
        "missing-empty-or-conflict-state",
        second_scan.status == "409 Conflict" && second_scan.body.contains(&active_id)
    );

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let completed_route = format!("/api/library/health/scans/{active_id}");
    let completed = super::route_http_request("GET", &completed_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{completed_route}: {error}"));
    record!(
        "GET",
        "/api/library/health/scans/{scanId}",
        "nominal-status-headers-body",
        serde_json::from_str::<serde_json::Value>(&completed.body).unwrap_or_default()["status"]
            == "completed"
    );

    let completed_versioned_route = format!("/api/v0/library/health/scans/{active_id}");
    let completed_versioned =
        super::route_http_request("GET", &completed_versioned_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{completed_versioned_route}: {error}"));
    let completed_versioned_json =
        serde_json::from_str::<serde_json::Value>(&completed_versioned.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/library/health/scans/{scanId}",
        "nominal-status-headers-body",
        completed_versioned.status == "200 OK" && completed_versioned_json["status"] == "completed"
    );
    record!(
        "GET",
        "/api/v0/library/health/scans/{scanId}",
        "populated-dynamic-state",
        completed_versioned_json["status"] == "completed"
            && completed_versioned_json["issues_found"].as_u64().is_some()
    );
    record!(
        "POST",
        "/api/v0/library/health/scans",
        "mutation-side-effects-and-readback",
        scan.status == "200 OK"
            && completed_versioned_json["status"] == "completed"
            && completed_versioned_json["issues_found"].as_u64().is_some()
    );

    let missing_scan = super::route_http_request(
        "GET",
        "/api/library/health/scans/scan-does-not-exist-differential",
        None,
        "",
        &state,
    )
    .await
    .expect("missing library scan");
    record!(
        "GET",
        "/api/library/health/scans/{scanId}",
        "missing-empty-or-conflict-state",
        missing_scan.status == "404 Not Found"
    );

    let fixed = super::route_http_request(
        "POST",
        "/api/v0/slskdn/library/remediate",
        None,
        r#"{"issue_ids":["lib-3-missing-kind"]}"#,
        &state,
    )
    .await
    .expect("fix library issues");
    let fixed_json = serde_json::from_str::<serde_json::Value>(&fixed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/slskdn/library/remediate",
        "nominal-status-headers-body",
        fixed.status == "200 OK" && fixed_json["id"].as_str().is_some()
    );
    record!(
        "POST",
        "/api/v0/slskdn/library/remediate",
        "mutation-side-effects-and-readback",
        fixed_json["kind"] == "library_remediation"
            && fixed_json["status"] == "completed"
            && fixed_json["fixedCount"] == 1
            && fixed_json["issueIds"] == serde_json::json!(["lib-3-missing-kind"])
            && fixed_json["remaining"] == serde_json::Value::Null
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("library_health.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api library-health mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The library-health differential above already covers populated and
/// nominal projections.  This companion closes the remaining deterministic
/// empty/error branches for the same real handlers, including the frozen
/// `/api/v0/` aliases and the native `/api/v0/slskdn/` alias.  No database
/// fault or external scan is used here; runtime-failure cases remain open.
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
async fn controller_api_differential_library_health_versioned_edge_states() {
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

    macro_rules! get_case {
        ($state:expr, $path:expr, $route:expr, $case:expr, $check:expr) => {{
            let response = super::route_http_request("GET", $path, None, "", $state)
                .await
                .unwrap_or_else(|error| panic!("GET {}: {error}", $path));
            let value = serde_json::from_str::<serde_json::Value>(&response.body)
                .unwrap_or(serde_json::Value::Null);
            let check: fn(&super::routing::HttpResponse, &serde_json::Value) -> bool = $check;
            let pass = check(&response, &value);
            record!("GET", $route, $case, pass);
        }};
    }

    let (empty_state, _receiver) = test_state();

    // The unversioned compatibility routes have real empty-state
    // contracts already implemented; exercise the branches that the
    // populated library-health test intentionally leaves open.
    get_case!(
        &empty_state,
        "/api/library/health/summary",
        "/api/library/health/summary",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/library/health/dashboard",
        "/api/library/health/dashboard",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-artist",
        "/api/library/health/issues/by-artist",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalArtists"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-codec",
        "/api/library/health/issues/by-codec",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalIssues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-release",
        "/api/library/health/issues/by-release",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalReleases"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-type",
        "/api/library/health/issues/by-type",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalIssues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/slskdn/library/health",
        "/api/slskdn/library/health",
        "missing-empty-or-conflict-state",
        |response, value| { response.status == "200 OK" && value["summary"]["total_issues"] == 0 }
    );
    get_case!(
        &empty_state,
        "/api/library/health/scans/",
        "/api/library/health/scans/{scanId}",
        "malformed-path-query-or-body",
        |response, _| response.status == "404 Not Found"
    );

    // Versioned library-health projections: empty results are real
    // state-backed responses, while missing required values and malformed
    // values follow the controller's BadRequest/NotFound contracts.
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues?limit=0",
        "/api/v0/library/health/issues",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues",
        "/api/v0/library/health/issues",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["issues"].as_array().is_some_and(Vec::is_empty)
                && value["totalCount"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-artist?limit=101",
        "/api/v0/library/health/issues/by-artist",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-artist",
        "/api/v0/library/health/issues/by-artist",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalArtists"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-codec/",
        "/api/v0/library/health/issues/by-codec",
        "malformed-path-query-or-body",
        |response, _| response.status == "404 Not Found"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-codec",
        "/api/v0/library/health/issues/by-codec",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalIssues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-release?limit=0",
        "/api/v0/library/health/issues/by-release",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-release",
        "/api/v0/library/health/issues/by-release",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalReleases"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-type",
        "/api/v0/library/health/issues/by-type",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/summary?libraryPath=%20",
        "/api/v0/library/health/summary",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/summary",
        "/api/v0/library/health/summary",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=0",
        "/api/v0/library/health/dashboard",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/dashboard",
        "/api/v0/library/health/dashboard",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/scans/",
        "/api/v0/library/health/scans/{scanId}",
        "malformed-path-query-or-body",
        |response, _| response.status == "404 Not Found"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/scans/no-such-scan",
        "/api/v0/library/health/scans/{scanId}",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "404 Not Found"
    );

    // The native versioned alias also has a populated branch backed by the
    // same library store, not a hardcoded fixture response.
    get_case!(
        &empty_state,
        "/api/v0/slskdn/library/health?limit=1",
        "/api/v0/slskdn/library/health",
        "nominal-status-headers-body",
        |response, value| {
            response.status == "200 OK"
                && value["path"] == "(all)"
                && value["summary"]["total_issues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/slskdn/library/health?limit=251",
        "/api/v0/slskdn/library/health",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );

    let (populated_state, _receiver) = test_state();
    let seed = super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Versioned health","kind":"Audio"}"#,
        &populated_state,
    )
    .await
    .expect("seed versioned library-health issue");
    assert_eq!(seed.status, "201 Created", "{}", seed.body);
    get_case!(
        &populated_state,
        "/api/v0/slskdn/library/health?limit=1",
        "/api/v0/slskdn/library/health",
        "populated-dynamic-state",
        |response, value| {
            response.status == "200 OK"
                && value["summary"]["total_issues"]
                    .as_u64()
                    .is_some_and(|count| count >= 1)
                && value["issues"]
                    .as_array()
                    .is_some_and(|issues| !issues.is_empty())
        }
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("library_health_versioned_edge_states.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned library-health mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the 6 registered routes found
/// inside `compatibility_projections_use_local_state_for_system_
/// mutation_shells` (a 53-`route_http_request`-call test found via
/// call-density scan): bridge admin config/start/stop/status, and
/// federation diagnostics/logs. The other ~47 calls in that source
/// test hit slskR-internal bare `/api/...` compat-shell routes
/// (`/api/admin/*`, `/api/application`, `/api/relay*`, `/api/batch`,
/// `/api/profile/me`, etc.) with zero registry entry in either
/// frozen target -- confirmed route-by-route, not creditable.
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
async fn controller_api_differential_bridge_admin_and_federation_diagnostics() {
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

    for path in ["/api/bridge/admin/config", "/api/v0/bridge/admin/config"] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            path,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && value["enabled"].is_boolean()
                && value["port"].is_number()
                && value["soulfind_path"].is_string()
                && value["max_clients"].is_number()
                && value["require_auth"].is_boolean()
        );
    }

    let bridge_config = super::route_http_request(
        "PUT",
        "/api/v0/bridge/admin/config",
        None,
        r#"{"maxClients":4,"enabled":true}"#,
        &state,
    )
    .await
    .expect("bridge config update");
    let bridge_config_json =
        serde_json::from_str::<serde_json::Value>(&bridge_config.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/bridge/admin/config",
        "nominal-status-headers-body",
        bridge_config.status == "200 OK"
    );
    record!(
        "PUT",
        "/api/v0/bridge/admin/config",
        "mutation-side-effects-and-readback",
        bridge_config_json
            == serde_json::json!({
                "message": "Configuration updated. Restart bridge service to apply changes.",
                "restart_required": true,
            })
    );

    let bridge_start =
        super::route_http_request("POST", "/api/v0/bridge/start", None, "{}", &state)
            .await
            .expect("bridge start");
    let bridge_start_json =
        serde_json::from_str::<serde_json::Value>(&bridge_start.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/start",
        "nominal-status-headers-body",
        bridge_start.status == "200 OK"
            && bridge_start_json == serde_json::json!({"status": "started"})
    );

    let bridge_status = super::route_http_request("GET", "/api/bridge/status", None, "", &state)
        .await
        .expect("bridge status");
    let bridge_status_json =
        serde_json::from_str::<serde_json::Value>(&bridge_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/bridge/status",
        "nominal-status-headers-body",
        bridge_status.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/status",
        "populated-dynamic-state",
        bridge_status_json["isHealthy"].is_boolean()
            && bridge_status_json["activeConnections"] == 0
    );

    let versioned_bridge_status =
        super::route_http_request("GET", "/api/v0/bridge/status", None, "", &state)
            .await
            .expect("versioned bridge status");
    let versioned_bridge_status_json =
        serde_json::from_str::<serde_json::Value>(&versioned_bridge_status.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/bridge/status",
        "nominal-status-headers-body",
        versioned_bridge_status.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/bridge/status",
        "populated-dynamic-state",
        versioned_bridge_status_json["isHealthy"].is_boolean()
            && versioned_bridge_status_json["activeConnections"] == 0
    );

    let bridge_stop = super::route_http_request("POST", "/api/v0/bridge/stop", None, "{}", &state)
        .await
        .expect("bridge stop");
    let bridge_stop_json =
        serde_json::from_str::<serde_json::Value>(&bridge_stop.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/bridge/stop",
        "nominal-status-headers-body",
        bridge_stop.status == "200 OK"
            && bridge_stop_json == serde_json::json!({"status": "stopped"})
    );

    let federation =
        super::route_http_request("GET", "/api/v0/federation/diagnostics", None, "", &state)
            .await
            .expect("federation diagnostics");
    let federation_json =
        serde_json::from_str::<serde_json::Value>(&federation.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/federation/diagnostics",
        "nominal-status-headers-body",
        federation.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/federation/diagnostics",
        "populated-dynamic-state",
        federation_json["federation"]["enabled"] == false
            && federation_json["federation"]["mode"] == "Hermit"
            && federation_json["federation"]["exposure"] == "Hermit"
            && federation_json["publishing"]["publishableDomains"] == serde_json::json!(["music"])
            && federation_json["pods"]["joinSignatureMode"] == "Off"
            && federation_json["mesh"]["selfPeerIdConfigured"] == true
            && federation_json["warnings"]
                == serde_json::json!([
                    "Pod join signatures are not enforced.",
                    "Pod message signatures are not enforced."
                ])
    );

    super::record_daemon_log(
        &state,
        super::logging::LogLevel::Info,
        "compat.event",
        "differential log entry",
    )
    .await;
    let logs = super::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("logs");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/logs",
        "nominal-status-headers-body",
        logs.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/logs",
        "populated-dynamic-state",
        logs_json[0]["category"] == "compat.event"
            && logs_json[0]["context"] == "compat.event"
            && logs_json[0]["message"] == "differential log entry"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("bridge_admin_and_federation_diagnostics.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api bridge-admin-federation-diagnostics mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 3 registered routes found inside
/// `compatibility_projections_use_local_state_for_recommendations_
/// and_activity` (found via call-density scan): source-feed-imports
/// preview, and bridge admin stats/dashboard (which must stay honest
/// zeros for a real embedded Soulfind bridge server that doesn't
/// exist, never backfilled from slskR's own unrelated transfer
/// queue). The rest of that source test hits slskR-internal bare
/// `/api/...` compat-shell routes (`/api/nowplaying`, `/api/soulseek/
/// interests`, `/api/source-feeds`, `/api/wishlist`) with zero
/// registry entry in either frozen target -- confirmed, not
/// creditable. slskdN-only (confirmed against the frozen registry).
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
async fn controller_api_differential_bridge_admin_stats_and_source_feed_preview() {
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

    let source_preview = super::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        r#"{"text":"Artist - One\nTwo"}"#,
        &state,
    )
    .await
    .expect("source preview");
    let source_preview_json =
        serde_json::from_str::<serde_json::Value>(&source_preview.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "nominal-status-headers-body",
        source_preview.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/source-feed-imports/preview",
        "mutation-side-effects-and-readback",
        source_preview_json["suggestionCount"] == 2
            && source_preview_json["suggestions"][0]["artist"] == "Artist"
            && source_preview_json["suggestions"][1]["title"] == "Two"
    );

    super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Differential.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create bridge transfer fixture");

    let bridge_stats =
        super::route_http_request("GET", "/api/bridge/admin/stats", None, "", &state)
            .await
            .expect("bridge stats");
    let bridge_stats_json =
        serde_json::from_str::<serde_json::Value>(&bridge_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/bridge/admin/stats",
        "nominal-status-headers-body",
        bridge_stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/admin/stats",
        "populated-dynamic-state",
        bridge_stats_json["totalConnections"] == 0
            && bridge_stats_json["currentConnections"] == 0
            && bridge_stats_json["totalDownloads"] == 0
            && bridge_stats_json["totalSearches"] == 0
            && bridge_stats_json["totalRoomJoins"] == 0
            && bridge_stats_json["totalBytesProxied"] == 0
    );

    let bridge_dashboard =
        super::route_http_request("GET", "/api/bridge/admin/dashboard", None, "", &state)
            .await
            .expect("bridge dashboard");
    let bridge_dashboard_json =
        serde_json::from_str::<serde_json::Value>(&bridge_dashboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/bridge/admin/dashboard",
        "nominal-status-headers-body",
        bridge_dashboard.status == "200 OK"
    );
    record!(
        "GET",
        "/api/bridge/admin/dashboard",
        "populated-dynamic-state",
        bridge_dashboard_json["connectedClients"]
            .as_array()
            .is_some_and(Vec::is_empty)
            && bridge_dashboard_json["health"]["isHealthy"].is_boolean()
            && bridge_dashboard_json["stats"]["totalConnections"] == 0
            && bridge_dashboard_json["stats"]["totalBytesProxied"] == 0
    );

    let versioned_bridge_stats =
        super::route_http_request("GET", "/api/v0/bridge/admin/stats", None, "", &state)
            .await
            .expect("versioned bridge stats");
    let versioned_bridge_stats_json =
        serde_json::from_str::<serde_json::Value>(&versioned_bridge_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/bridge/admin/stats",
        "nominal-status-headers-body",
        versioned_bridge_stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/bridge/admin/stats",
        "populated-dynamic-state",
        versioned_bridge_stats_json["totalConnections"] == 0
            && versioned_bridge_stats_json["currentConnections"] == 0
            && versioned_bridge_stats_json["totalDownloads"] == 0
            && versioned_bridge_stats_json["totalSearches"] == 0
            && versioned_bridge_stats_json["totalRoomJoins"] == 0
            && versioned_bridge_stats_json["totalBytesProxied"] == 0
    );

    let versioned_bridge_dashboard =
        super::route_http_request("GET", "/api/v0/bridge/admin/dashboard", None, "", &state)
            .await
            .expect("versioned bridge dashboard");
    let versioned_bridge_dashboard_json =
        serde_json::from_str::<serde_json::Value>(&versioned_bridge_dashboard.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/bridge/admin/dashboard",
        "nominal-status-headers-body",
        versioned_bridge_dashboard.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/bridge/admin/dashboard",
        "populated-dynamic-state",
        versioned_bridge_dashboard_json["connectedClients"]
            .as_array()
            .is_some_and(Vec::is_empty)
            && versioned_bridge_dashboard_json["health"]["isHealthy"].is_boolean()
            && versioned_bridge_dashboard_json["stats"]["totalConnections"] == 0
            && versioned_bridge_dashboard_json["stats"]["totalBytesProxied"] == 0
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("bridge_admin_stats_and_source_feed_preview.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api bridge-admin-stats-source-feed-preview mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the remaining registered routes
/// of `extended_controller_mutations_are_stateful_and_domain_backed`
/// (found via call-density scan) not already credited elsewhere:
/// opinions GET/DELETE, security circuits (real Tor-circuit-building
/// failure, no invented success), mediacore descriptor stats, pods
/// creation, podcore channels, and the pod signing keypair/sign/
/// verify pipeline (real ed25519 crypto resolved from actual pod
/// membership, not a client-supplied key -- a forged sender is
/// genuinely rejected). slskdN-only (confirmed against the frozen
/// registry; the literal-pod-id-embedded `/api/v0/podcore/pod-
/// controller/channels` path in the source test maps to the real
/// templated route `/api/v0/podcore/{podId}/channels`, not a
/// separate unregistered path).
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
async fn controller_api_differential_extended_controller_mutations() {
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

    let opinion = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"differential-test","subjectType":"Track","subjectId":"recording-differential","kind":"Like","strength":0.75,"confidence":1,"comment":"good"}"#,
        &state,
    )
    .await
    .expect("create opinion");
    let opinion_json = serde_json::from_str::<serde_json::Value>(&opinion.body).unwrap_or_default();
    let opinion_id = opinion_json["id"].as_str().unwrap_or_default().to_owned();

    let opinions = super::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .expect("list opinions");
    record!(
        "GET",
        "/api/v0/opinions",
        "nominal-status-headers-body",
        opinions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/opinions",
        "populated-dynamic-state",
        opinions.body.contains("recording-differential")
    );
    record!(
        "POST",
        "/api/v0/opinions",
        "mutation-side-effects-and-readback",
        opinion.status == "200 OK"
            && !opinion_id.is_empty()
            && opinion_json["subjectId"] == "recording-differential"
            && opinions.body.contains(&opinion_id)
    );

    let circuit = super::route_http_request(
        "POST",
        "/api/v0/security/circuits",
        None,
        r#"{"id":"circuit-differential","peerId":"peer-differential","active":true}"#,
        &state,
    )
    .await
    .expect("create circuit");
    record!(
        "POST",
        "/api/v0/security/circuits",
        "runtime-failure-and-timeout",
        circuit.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&circuit.body).unwrap_or_default()
                == serde_json::json!({"error": "Circuit building failed"})
    );

    let circuits = super::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
        .await
        .expect("list circuits");
    record!(
        "GET",
        "/api/v0/security/circuits",
        "nominal-status-headers-body",
        circuits.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/security/circuits",
        "populated-dynamic-state",
        circuits.body == "[]"
    );

    super::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        None,
        &serde_json::json!({
            "descriptor": {
                "contentId": "cid-differential",
                "hashes": [{"algorithm": "sha256", "hex": "0123456789abcdef"}],
                "signature": {
                    "publicKey": "key",
                    "signature": "0123456789abcdef",
                    "timestampUnixMs": super::unix_timestamp_millis(),
                },
            },
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish descriptor fixture");

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
        "nominal-status-headers-body",
        descriptor_stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        "populated-dynamic-state",
        descriptor_stats_json["activeCacheEntries"] == 0
    );

    let created_pod = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-controller-differential","name":"Controller Pod Differential","isPublic":true}}"#,
        &state,
    )
    .await
    .expect("create pod");
    record!(
        "POST",
        "/api/v0/pods",
        "nominal-status-headers-body",
        created_pod.status == "201 Created"
    );

    let channel = super::route_http_request(
        "POST",
        "/api/v0/podcore/pod-controller-differential/channels",
        None,
        r#"{"channelId":"general","name":"General"}"#,
        &state,
    )
    .await
    .expect("create pod channel");
    record!(
        "POST",
        "/api/v0/podcore/{podId}/channels",
        "nominal-status-headers-body",
        channel.status == "201 Created"
    );

    let channels = super::route_http_request(
        "GET",
        "/api/v0/podcore/pod-controller-differential/channels",
        None,
        "",
        &state,
    )
    .await
    .expect("list pod channels");
    record!(
        "GET",
        "/api/v0/podcore/{podId}/channels",
        "nominal-status-headers-body",
        channels.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/{podId}/channels",
        "populated-dynamic-state",
        channels.body.contains("general")
    );
    record!(
        "POST",
        "/api/v0/podcore/{podId}/channels",
        "mutation-side-effects-and-readback",
        channel.status == "201 Created" && channels.body.contains("general")
    );

    let keypair = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate pod signing keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        "nominal-status-headers-body",
        keypair.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        "mutation-side-effects-and-readback",
        keys["publicKey"].as_str().is_some() && keys["privateKey"].as_str().is_some()
    );

    state
        .pods
        .write()
        .await
        .upsert_member(
            "pod-controller-differential",
            super::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let signed = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId":"message-differential",
                "podId":"pod-controller-differential",
                "senderPeerId":"tester",
                "body":"hello",
                "timestampUnixMs": super::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign pod message");
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/signing/sign",
        "nominal-status-headers-body",
        signed.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/signing/sign",
        "mutation-side-effects-and-readback",
        signed_json["signature"]
            .as_str()
            .is_some_and(|signature| signature.starts_with("ed25519:"))
    );

    let mut forged = signed_json.clone();
    forged["message"]["senderPeerId"] = serde_json::json!("someone-else");
    let forged_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &state,
    )
    .await
    .expect("verify forged sender");
    record!(
        "POST",
        "/api/v0/podcore/signing/verify",
        "missing-empty-or-conflict-state",
        forged_verified.body == r#"{"isValid":false}"#
    );

    let ranked = super::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":"slow","filename":"x.flac","uploadSpeed":10},{"username":"fast","filename":"x.flac","uploadSpeed":10000,"hasFreeUploadSlot":true}]"#,
        &state,
    )
    .await
    .expect("rank sources");
    let ranked_json = serde_json::from_str::<serde_json::Value>(&ranked.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "nominal-status-headers-body",
        ranked.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "mutation-side-effects-and-readback",
        ranked_json[0]["username"] == "fast"
    );

    let opinion_delete_route = format!("/api/v0/opinions/{opinion_id}");
    let removed_opinion =
        super::route_http_request("DELETE", &opinion_delete_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{opinion_delete_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "nominal-status-headers-body",
        removed_opinion.status == "204 No Content"
    );
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "mutation-side-effects-and-readback",
        removed_opinion.status == "204 No Content"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("extended_controller_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api extended-controller-mutations mismatches:\n{}",
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
async fn controller_api_differential_native_ranking_contracts() {
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

    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn ranking database");
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));

    let empty_history =
        super::route_http_request("GET", "/api/v0/ranking/history/rank-peer", None, "", &state)
            .await
            .expect("empty ranking history");
    let empty_history_json =
        serde_json::from_str::<serde_json::Value>(&empty_history.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/ranking/history/{username}",
        "nominal-status-headers-body",
        empty_history.status == "200 OK"
            && empty_history.content_type == "application/json"
            && empty_history_json["username"] == "rank-peer"
            && empty_history_json["successes"] == 0
            && empty_history_json["failures"] == 0
            && empty_history_json["successRate"] == 0.5
            && empty_history_json
                .as_object()
                .is_some_and(|object| object.len() == 4)
    );

    let malformed_history =
        super::route_http_request("GET", "/api/v0/ranking/history/%20", None, "", &state)
            .await
            .expect("malformed ranking history path");
    record!(
        "GET",
        "/api/v0/ranking/history/{username}",
        "malformed-path-query-or-body",
        malformed_history.status == "400 Bad Request"
            && malformed_history.body == "Username is required"
    );

    let missing_history = super::route_http_request(
        "GET",
        "/api/v0/ranking/history/unknown-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("missing ranking history");
    let missing_history_json =
        serde_json::from_str::<serde_json::Value>(&missing_history.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/ranking/history/{username}",
        "missing-empty-or-conflict-state",
        missing_history.status == "200 OK"
            && missing_history_json["username"] == "unknown-peer"
            && missing_history_json["successRate"] == 0.5
    );

    let success_id = {
        let mut transfers = state.transfers.write().await;
        let success = transfers.create(
            0,
            Some("rank-peer".to_owned()),
            "rank/success.flac".to_owned(),
            None,
            Some(100),
        );
        let failure = transfers.create(
            0,
            Some("rank-peer".to_owned()),
            "rank/failure.flac".to_owned(),
            None,
            Some(100),
        );
        for entry in &mut transfers.entries {
            if entry.id == success.id {
                entry.status = "completed".to_owned();
            } else if entry.id == failure.id {
                entry.status = "failed".to_owned();
            }
        }
        success.id
    };
    let seeded_entries = state.transfers.read().await.entries.clone();
    super::persist_transfer_records(&state, &seeded_entries)
        .await
        .expect("persist ranking transfer history");
    let _ = success_id;

    let populated_history =
        super::route_http_request("GET", "/api/v0/ranking/history/rank-peer", None, "", &state)
            .await
            .expect("populated ranking history");
    let populated_history_json =
        serde_json::from_str::<serde_json::Value>(&populated_history.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/ranking/history/{username}",
        "populated-dynamic-state",
        populated_history.status == "200 OK"
            && populated_history_json["successes"] == 1
            && populated_history_json["failures"] == 1
            && populated_history_json["successRate"] == 0.5
    );

    let history_body = r#"[" rank-peer ","rank-peer","other-peer"]"#;
    let histories = super::route_http_request(
        "POST",
        "/api/v0/ranking/history",
        None,
        history_body,
        &state,
    )
    .await
    .expect("ranking histories");
    let histories_json =
        serde_json::from_str::<serde_json::Value>(&histories.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/ranking/history",
        "nominal-status-headers-body",
        histories.status == "200 OK"
            && histories.content_type == "application/json"
            && histories_json["rank-peer"]["successes"] == 1
            && histories_json["other-peer"]["successRate"] == 0.5
            && histories_json
                .as_object()
                .is_some_and(|object| object.len() == 2)
    );
    record!(
        "POST",
        "/api/v0/ranking/history",
        "mutation-side-effects-and-readback",
        histories_json.get("rank-peer").is_some()
            && histories_json.get("other-peer").is_some()
            && histories_json.get("rank-peer ").is_none()
    );

    let malformed_histories =
        super::route_http_request("POST", "/api/v0/ranking/history", None, "not-json", &state)
            .await
            .expect("malformed ranking histories");
    record!(
        "POST",
        "/api/v0/ranking/history",
        "malformed-path-query-or-body",
        malformed_histories.status == "400 Bad Request"
            && malformed_histories.body == "At least one username is required"
    );

    let empty_histories =
        super::route_http_request("POST", "/api/v0/ranking/history", None, "[]", &state)
            .await
            .expect("empty ranking histories");
    let blank_histories = super::route_http_request(
        "POST",
        "/api/v0/ranking/history",
        None,
        r#"[" ",""]"#,
        &state,
    )
    .await
    .expect("blank ranking histories");
    record!(
        "POST",
        "/api/v0/ranking/history",
        "missing-empty-or-conflict-state",
        empty_histories.status == "400 Bad Request"
            && blank_histories.status == "400 Bad Request"
            && blank_histories.body == "Each username must be non-empty"
    );

    let restart_state =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone())).0;
    restart_state
        .transfers
        .write()
        .await
        .rehydrate_from_database(&db)
        .await;
    let restarted_histories = super::route_http_request(
        "POST",
        "/api/v0/ranking/history",
        None,
        r#"["rank-peer"]"#,
        &restart_state,
    )
    .await
    .expect("ranking histories after restart");
    let restarted_histories_json =
        serde_json::from_str::<serde_json::Value>(&restarted_histories.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/ranking/history",
        "restart-persistence-or-reset",
        restarted_histories.status == "200 OK"
            && restarted_histories_json["rank-peer"]["successes"] == 1
            && restarted_histories_json["rank-peer"]["failures"] == 1
    );

    let (first_histories, second_histories) = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v0/ranking/history",
            None,
            r#"["rank-peer","other-peer"]"#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/ranking/history",
            None,
            r#"["rank-peer","other-peer"]"#,
            &state,
        )
    );
    let first_histories = first_histories.expect("first concurrent ranking history");
    let second_histories = second_histories.expect("second concurrent ranking history");
    record!(
        "POST",
        "/api/v0/ranking/history",
        "concurrency-and-idempotency",
        first_histories.status == "200 OK"
            && second_histories.status == "200 OK"
            && first_histories.body == second_histories.body
    );

    let rank_body = r#"[
        {"username":" slow ","filename":" x.flac ","size":100,"uploadSpeed":10},
        {"username":"fast","filename":"x.flac","size":100,"uploadSpeed":10000000,"hasFreeUploadSlot":true}
    ]"#;
    let ranked = super::route_http_request("POST", "/api/v0/ranking/rank", None, rank_body, &state)
        .await
        .expect("rank sources");
    let ranked_json = serde_json::from_str::<serde_json::Value>(&ranked.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "nominal-status-headers-body",
        ranked.status == "200 OK"
            && ranked_json.as_array().is_some_and(|rows| rows.len() == 2)
            && ranked_json[0]["username"] == "fast"
            && ranked_json[0]["filename"] == "x.flac"
            && ranked_json[0]["smartScore"].as_f64().is_some()
            && ranked_json[0]["speedScore"] == 40.0
            && ranked_json[0]["freeSlotScore"] == 15.0
    );
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "mutation-side-effects-and-readback",
        ranked_json[0]["username"] == "fast"
            && ranked_json[1]["username"] == "slow"
            && ranked_json[0]["username"] != ranked_json[1]["username"]
    );

    let malformed_rank =
        super::route_http_request("POST", "/api/v0/ranking/rank", None, "{}", &state)
            .await
            .expect("malformed rank sources");
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "malformed-path-query-or-body",
        malformed_rank.status == "400 Bad Request"
            && malformed_rank.body == "At least one source candidate is required"
    );

    let empty_rank = super::route_http_request("POST", "/api/v0/ranking/rank", None, "[]", &state)
        .await
        .expect("empty rank sources");
    let invalid_rank = super::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":" ","filename":"x.flac"}]"#,
        &state,
    )
    .await
    .expect("invalid rank sources");
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "missing-empty-or-conflict-state",
        empty_rank.status == "400 Bad Request"
            && invalid_rank.status == "400 Bad Request"
            && invalid_rank.body
                == "Each source candidate requires a non-empty username and filename"
    );

    let restarted_rank = super::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":"rank-peer","filename":"x.flac","size":100}]"#,
        &restart_state,
    )
    .await
    .expect("rank sources after restart");
    let restarted_rank_json =
        serde_json::from_str::<serde_json::Value>(&restarted_rank.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "restart-persistence-or-reset",
        restarted_rank.status == "200 OK"
            && restarted_rank_json[0]["username"] == "rank-peer"
            && restarted_rank_json[0]["historyScore"] == 0.0
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("ranking failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        env.clone(),
        super::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_rank = super::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":"rank-peer","filename":"x.flac"}]"#,
        &failure_state,
    )
    .await
    .expect("ranking failure");
    let failed_history = super::route_http_request(
        "GET",
        "/api/v0/ranking/history/rank-peer",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("ranking history failure");
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "runtime-failure-and-timeout",
        failed_rank.status == "500 Internal Server Error"
    );
    record!(
        "GET",
        "/api/v0/ranking/history/{username}",
        "runtime-failure-and-timeout",
        failed_history.status == "500 Internal Server Error"
    );
    record!(
        "POST",
        "/api/v0/ranking/history",
        "runtime-failure-and-timeout",
        super::route_http_request(
            "POST",
            "/api/v0/ranking/history",
            None,
            r#"["rank-peer"]"#,
            &failure_state,
        )
        .await
        .expect("ranking histories failure")
        .status
            == "500 Internal Server Error"
    );

    let (first_rank, second_rank) = tokio::join!(
        super::route_http_request("POST", "/api/v0/ranking/rank", None, rank_body, &state),
        super::route_http_request("POST", "/api/v0/ranking/rank", None, rank_body, &state)
    );
    let first_rank = first_rank.expect("first concurrent rank");
    let second_rank = second_rank.expect("second concurrent rank");
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "concurrency-and-idempotency",
        first_rank.status == "200 OK"
            && second_rank.status == "200 OK"
            && first_rank.body == second_rank.body
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_ranking_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize ranking ledger"),
    )
    .expect("write ranking ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn ranking mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 7 quarantine-jury routes' cases,
/// independently re-derived from `quarantine_jury_requires_real_
/// quorum_before_accepting_or_releasing`'s real signed-verdict
/// validation, quorum enforcement, and idempotent-acceptance checks
/// (reuses the shared `quarantine_signed_verdict_json` fixture
/// builder, not the original test function). slskdN-only (confirmed
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
async fn controller_api_differential_quarantine_jury() {
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

    let created = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"differential","jurors":["juror-x","juror-y","juror-z"],"evidence":[{"type":"hash","reference":"opaque-ref-differential"}],"minJurorVotes":2}"#,
        &state,
    )
    .await
    .expect("create quarantine request");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    let request_id = created_json["request"]["requestId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests",
        "nominal-status-headers-body",
        created.status == "200 OK" && !request_id.is_empty()
    );

    let empty_routes = super::route_http_request(
        "GET",
        "/api/v0/quarantine-jury/requests/empty-differential/routes",
        None,
        "",
        &state,
    )
    .await
    .expect("empty quarantine route attempts");
    let empty_routes_json =
        serde_json::from_str::<serde_json::Value>(&empty_routes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "missing-empty-or-conflict-state",
        empty_routes.status == "200 OK" && empty_routes_json == serde_json::json!([])
    );

    let missing_route = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests/missing-differential/routes",
        None,
        "{}",
        &state,
    )
    .await
    .expect("missing quarantine request route");
    let missing_route_json =
        serde_json::from_str::<serde_json::Value>(&missing_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "missing-empty-or-conflict-state",
        missing_route.status == "404 Not Found"
            && missing_route_json["requestId"] == "missing-differential"
            && missing_route_json["success"] == false
            && missing_route_json["errorMessage"] == "Request not found."
    );

    let requests =
        super::route_http_request("GET", "/api/v0/quarantine-jury/requests", None, "", &state)
            .await
            .expect("list quarantine requests");
    let requests_json =
        serde_json::from_str::<serde_json::Value>(&requests.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests",
        "populated-dynamic-state",
        requests.status == "200 OK"
            && requests_json
                .as_array()
                .is_some_and(|items| { items.iter().any(|item| item["requestId"] == request_id) })
    );

    let request_detail_route = format!("/api/v0/quarantine-jury/requests/{request_id}");
    let request_detail = super::route_http_request("GET", &request_detail_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{request_detail_route}: {error}"));
    let request_detail_json =
        serde_json::from_str::<serde_json::Value>(&request_detail.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}",
        "nominal-status-headers-body",
        request_detail.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}",
        "populated-dynamic-state",
        request_detail_json["requestId"] == request_id
            && request_detail_json["localReason"] == "differential"
            && request_detail_json["status"] == "Pending"
    );

    let unlisted = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &quarantine_signed_verdict_json(
            &request_id,
            "not-a-juror-differential",
            "ReleaseCandidate",
        )
        .to_string(),
        &state,
    )
    .await
    .expect("unlisted juror verdict");
    let mut verdicts_malformed_pass = unlisted.status == "400 Bad Request";

    let unsigned = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &format!(
            r#"{{"requestId":"{request_id}","juror":"juror-x","verdict":"ReleaseCandidate"}}"#
        ),
        &state,
    )
    .await
    .expect("unsigned verdict");
    let unsigned_json =
        serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap_or_default();
    verdicts_malformed_pass &= unsigned.status == "400 Bad Request"
        && unsigned_json["errors"].as_array().is_some_and(|errors| {
            errors
                .iter()
                .any(|error| error == "Signed juror verdict is required.")
        });

    let mut tampered = quarantine_signed_verdict_json(&request_id, "juror-x", "ReleaseCandidate");
    tampered["verdict"] = serde_json::json!("UpholdQuarantine");
    let tampered_result = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &tampered.to_string(),
        &state,
    )
    .await
    .expect("tampered verdict");
    let tampered_json =
        serde_json::from_str::<serde_json::Value>(&tampered_result.body).unwrap_or_default();
    verdicts_malformed_pass &= tampered_result.status == "400 Bad Request"
        && tampered_json["errors"].as_array().is_some_and(|errors| {
            errors
                .iter()
                .any(|error| error == "Signature payload hash does not match verdict contents.")
        });
    record!(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        "malformed-path-query-or-body",
        verdicts_malformed_pass
    );

    let too_early = super::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("accept too early");
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
        "missing-empty-or-conflict-state",
        too_early.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&too_early.body).unwrap_or_default()
                ["isAccepted"]
                == false
    );

    let package_too_early = super::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/release-package"),
        None,
        "",
        &state,
    )
    .await
    .expect("release package too early");
    let package_too_early_json =
        serde_json::from_str::<serde_json::Value>(&package_too_early.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/release-package",
        "missing-empty-or-conflict-state",
        package_too_early.status == "400 Bad Request"
            && package_too_early_json["isReady"] == false
            && package_too_early_json["errors"][0]
                .as_str()
                .is_some_and(|error| error.contains("has not been accepted"))
    );

    let mut verdicts_nominal_pass = true;
    for juror in ["juror-x", "juror-y"] {
        let verdict = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(&request_id, juror, "ReleaseCandidate").to_string(),
            &state,
        )
        .await
        .expect("real signed verdict");
        verdicts_nominal_pass &= verdict.status == "200 OK";
    }
    record!(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        "nominal-status-headers-body",
        verdicts_nominal_pass
    );

    let aggregate_route = format!("/api/v0/quarantine-jury/requests/{request_id}/aggregate");
    let aggregate = super::route_http_request("GET", &aggregate_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{aggregate_route}: {error}"));
    let aggregate_json =
        serde_json::from_str::<serde_json::Value>(&aggregate.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/aggregate",
        "nominal-status-headers-body",
        aggregate.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/aggregate",
        "populated-dynamic-state",
        aggregate_json["recommendation"] == "ReleaseCandidate"
            && aggregate_json["totalVerdicts"] == 2
            && aggregate_json["requiredVotes"] == 2
            && aggregate_json["quorumReached"] == true
    );

    let review_route = format!("/api/v0/quarantine-jury/requests/{request_id}/review");
    let review = super::route_http_request("GET", &review_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{review_route}: {error}"));
    let review_json = serde_json::from_str::<serde_json::Value>(&review.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/review",
        "nominal-status-headers-body",
        review.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/review",
        "populated-dynamic-state",
        review_json["canAcceptReleaseCandidate"] == true
            && review_json["verdicts"].as_array().map(Vec::len) == Some(2)
    );

    let accept_route =
        format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate");
    let accepted = super::route_http_request(
        "POST",
        &accept_route,
        None,
        r#"{"acceptedBy":"differential-operator"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{accept_route}: {error}"));
    let accepted_json =
        serde_json::from_str::<serde_json::Value>(&accepted.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
        "nominal-status-headers-body",
        accepted.status == "200 OK"
            && accepted_json["isAccepted"] == true
            && accepted_json["decision"]["acceptedBy"] == "differential-operator"
    );

    let reaccepted = super::route_http_request("POST", &accept_route, None, "{}", &state)
        .await
        .unwrap_or_else(|error| panic!("{accept_route}: {error}"));
    let reaccepted_json =
        serde_json::from_str::<serde_json::Value>(&reaccepted.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/accept-release-candidate",
        "mutation-side-effects-and-readback",
        reaccepted.status == "200 OK"
            && reaccepted_json["decision"]["id"] == accepted_json["decision"]["id"]
    );

    let package_route = format!("/api/v0/quarantine-jury/requests/{request_id}/release-package");
    let package = super::route_http_request("GET", &package_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{package_route}: {error}"));
    let package_json = serde_json::from_str::<serde_json::Value>(&package.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/release-package",
        "nominal-status-headers-body",
        package.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/release-package",
        "populated-dynamic-state",
        package_json["isReady"] == true
            && package_json["package"]["requestId"] == request_id
            && package_json["package"]["currentAggregate"]["recommendation"] == "ReleaseCandidate"
            && package_json["package"]["verdicts"].as_array().map(Vec::len) == Some(2)
    );

    let routes_route = format!("/api/v0/quarantine-jury/requests/{request_id}/routes");
    let bad_route = super::route_http_request(
        "POST",
        &routes_route,
        None,
        r#"{"targetJurors":["not-a-juror-differential"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{routes_route}: {error}"));
    let bad_route_json =
        serde_json::from_str::<serde_json::Value>(&bad_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "malformed-path-query-or-body",
        bad_route.status == "400 Bad Request"
            && bad_route_json["success"] == false
            && bad_route_json["errorMessage"]
                .as_str()
                .is_some_and(|error| error.contains("safe jurors"))
    );

    let real_route = super::route_http_request(
        "POST",
        &routes_route,
        None,
        r#"{"targetJurors":["juror-x"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{routes_route}: {error}"));
    let real_route_json =
        serde_json::from_str::<serde_json::Value>(&real_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "runtime-failure-and-timeout",
        real_route.status == "400 Bad Request"
            && real_route_json["success"] == false
            && real_route_json["errorMessage"] == "Routing backend is not available."
    );

    let routes = super::route_http_request("GET", &routes_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{routes_route}: {error}"));
    let routes_json = serde_json::from_str::<serde_json::Value>(&routes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "nominal-status-headers-body",
        routes.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "populated-dynamic-state",
        routes_json.as_array().is_some_and(|attempts| {
            attempts
                .iter()
                .any(|attempt| attempt["requestId"] == request_id)
        })
    );
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests/{requestId}/routes",
        "mutation-side-effects-and-readback",
        routes_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["requestId"] == request_id
                    && attempt["success"] == false
                    && attempt["errorMessage"] == "Routing backend is not available."
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("quarantine_jury.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api quarantine-jury mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
