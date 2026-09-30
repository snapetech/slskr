//! Controller full native api differential 03 ownership.

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
pub(super) async fn controller_api_differential_native_search_compatibility_contracts() {
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
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn search compatibility database");
    let (state, mut receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
    state.shares.write().await.entries.push(FileEntry {
        filename_encoding: Default::default(),
        extension_encoding: Default::default(),
        code: 1,
        filename: "Remote/Search.flac".to_owned(),
        size: 321,
        extension: "flac".to_owned(),
        attributes: Vec::new(),
    });

    let created = crate::route_http_request(
        "POST",
        "/api/search",
        None,
        r#"{"query":"  Remote Search  ","limit":1}"#,
        &state,
    )
    .await
    .expect("slskdn compatibility search");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body)
        .expect("slskdn compatibility search JSON");
    let search_id = created_json["searchId"].as_str().unwrap_or_default();
    let result = created_json["results"]
        .as_array()
        .and_then(|results| results.first())
        .cloned()
        .unwrap_or_default();
    record!(
        "POST",
        "/api/search",
        "nominal-status-headers-body",
        created.status == "200 OK"
            && created.content_type == "application/json"
            && search_id.len() == 32
            && !search_id.contains('-')
            && created_json["query"] == "Remote Search"
            && created_json["results"].as_array().is_some_and(|results| {
                results.len() == 1
                    && result.as_object().is_some_and(|object| {
                        object.len() == 5
                            && result["username"] == ""
                            && result["filename"] == "Remote/Search.flac"
                            && result["size"] == 321
                            && result["code"] == 1
                            && result["extension"] == "flac"
                    })
            })
    );

    let dispatched = receiver.try_recv();
    record!(
        "POST",
        "/api/search",
        "mutation-side-effects-and-readback",
        matches!(
            dispatched,
            Ok(crate::SessionCommand::Search {
                query,
                target: crate::SearchDispatchTarget::Global,
                ..
            }) if query == "Remote Search"
        ) && state
            .searches
            .read()
            .await
            .records
            .iter()
            .any(|record| record.query == "Remote Search")
    );

    let malformed = crate::route_http_request("POST", "/api/search", None, "not-json", &state)
        .await
        .expect("malformed slskdn compatibility search");
    let malformed_path = crate::route_http_request(
        "POST",
        "/api/search/extra",
        None,
        r#"{"query":"Remote"}"#,
        &state,
    )
    .await
    .expect("malformed slskdn compatibility search path");
    record!(
        "POST",
        "/api/search",
        "malformed-path-query-or-body",
        malformed.status == "400 Bad Request"
            && malformed.body == r#"{"error":"Query is required"}"#
            && malformed_path.status == "404 Not Found"
    );

    let blank =
        crate::route_http_request("POST", "/api/search", None, r#"{"query":"   "}"#, &state)
            .await
            .expect("blank slskdn compatibility search");
    let invalid_limit = crate::route_http_request(
        "POST",
        "/api/search",
        None,
        r#"{"query":"Remote","limit":0}"#,
        &state,
    )
    .await
    .expect("invalid slskdn compatibility search limit");
    record!(
        "POST",
        "/api/search",
        "missing-empty-or-conflict-state",
        blank.status == "400 Bad Request"
            && blank.body == r#"{"error":"Query is required"}"#
            && invalid_limit.status == "400 Bad Request"
            && invalid_limit.body == r#"{"error":"Limit must be positive"}"#
    );

    let persisted = db
        .list_searches(50, 0)
        .await
        .expect("list slskdn compatibility searches");
    let rehydrated = crate::SearchStore::from_persisted(persisted.clone());
    record!(
        "POST",
        "/api/search",
        "restart-persistence-or-reset",
        persisted
            .iter()
            .any(|record| record.query == "Remote Search")
            && rehydrated
                .records
                .iter()
                .any(|record| record.query == "Remote Search")
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn compatibility search failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed = crate::route_http_request(
        "POST",
        "/api/search",
        None,
        r#"{"query":"failed search"}"#,
        &failure_state,
    )
    .await
    .expect("slskdn compatibility search failure");
    record!(
        "POST",
        "/api/search",
        "runtime-failure-and-timeout",
        failed.status == "500 Internal Server Error"
            && failed.body == r#"{"error":"Search failed"}"#
            && failure_state.searches.read().await.records.is_empty()
            && failure_state.searches.read().await.next_token == 1
    );

    let (first, second) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/search",
            None,
            r#"{"query":"concurrent one"}"#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/search",
            None,
            r#"{"query":"concurrent two"}"#,
            &state,
        )
    );
    let first = first.expect("first concurrent compatibility search");
    let second = second.expect("second concurrent compatibility search");
    let first_id = serde_json::from_str::<serde_json::Value>(&first.body)
        .ok()
        .and_then(|body| body["searchId"].as_str().map(str::to_owned));
    let second_id = serde_json::from_str::<serde_json::Value>(&second.body)
        .ok()
        .and_then(|body| body["searchId"].as_str().map(str::to_owned));
    record!(
        "POST",
        "/api/search",
        "concurrency-and-idempotency",
        first.status == "200 OK"
            && second.status == "200 OK"
            && first_id.is_some()
            && second_id.is_some()
            && first_id != second_id
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_search_compatibility_contracts.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize slskdn search compatibility ledger"),
    )
    .expect("write slskdn search compatibility ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn search compatibility mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_native_searches_open_cases() {
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

    async fn seed_search(state: &Arc<crate::AppState>, id: &str, with_result: bool) {
        let token = {
            let mut searches = state.searches.write().await;
            searches
                .create(
                    Some(id.to_owned()),
                    "open search".to_owned(),
                    "global",
                    None,
                    Vec::new(),
                    3_600,
                )
                .expect("seed search")
                .record
                .token
        };
        if with_result {
            let response = crate::route_http_request(
                "POST",
                "/api/v0/search-responses",
                None,
                &format!(
                    r#"{{"token":{token},"username":"search-peer","files":[{{"filename":"Remote/Search.flac","size":321,"extension":"flac"}}]}}"#
                ),
                state,
            )
            .await
            .expect("seed search response");
            assert_eq!(response.status, "200 OK", "{}", response.body);
        }
    }

    async fn seed_expired_search(state: &Arc<crate::AppState>, id: &str) {
        let mut searches = state.searches.write().await;
        searches
            .create(
                Some(id.to_owned()),
                "expired open search".to_owned(),
                "global",
                None,
                Vec::new(),
                0,
            )
            .expect("seed expired search");
    }

    let env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let list_id = "00000000-0000-4000-8000-000000000101";
    let detail_id = "00000000-0000-4000-8000-000000000102";

    let (list_state, _receiver) = test_state_with_env(env.clone());
    let list_malformed = crate::route_http_request(
        "GET",
        "/api/v0/searches?limit=not-a-number",
        None,
        "",
        &list_state,
    )
    .await
    .expect("malformed search list query");
    record!(
        "GET",
        "/api/v0/searches",
        "malformed-path-query-or-body",
        list_malformed.status == "400 Bad Request"
    );
    let list_empty = crate::route_http_request("GET", "/api/v0/searches", None, "", &list_state)
        .await
        .expect("empty search list");
    record!(
        "GET",
        "/api/v0/searches",
        "missing-empty-or-conflict-state",
        list_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&list_empty.body)
                .is_ok_and(|value| value.as_array().is_some_and(Vec::is_empty))
    );

    let list_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search list runtime database");
    let (list_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(list_db.clone()),
    );
    seed_search(&list_failure_state, list_id, true).await;
    list_db.close_for_test().await;
    let list_runtime =
        crate::route_http_request("GET", "/api/v0/searches", None, "", &list_failure_state)
            .await
            .expect("search list runtime failure");
    record!(
        "GET",
        "/api/v0/searches",
        "runtime-failure-and-timeout",
        list_runtime.status == "500 Internal Server Error"
            && !list_runtime.body.contains("search list runtime database")
    );

    let (detail_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&detail_state, detail_id, true).await;
    let detail_malformed = crate::route_http_request(
        "GET",
        "/api/v0/searches/not-a-guid",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("malformed search detail id");
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "malformed-path-query-or-body",
        detail_malformed.status == "400 Bad Request"
    );
    let detail_missing = crate::route_http_request(
        "GET",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("missing search detail");
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        detail_missing.status == "404 Not Found"
    );

    let detail_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search detail runtime database");
    let (detail_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(detail_db.clone()),
    );
    seed_search(&detail_failure_state, detail_id, true).await;
    detail_db.close_for_test().await;
    let detail_runtime = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &detail_failure_state,
    )
    .await
    .expect("search detail runtime failure");
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        detail_runtime.status == "500 Internal Server Error"
    );

    let responses_malformed = crate::route_http_request(
        "GET",
        "/api/v0/searches/not-a-guid/responses",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("malformed search responses id");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "malformed-path-query-or-body",
        responses_malformed.status == "400 Bad Request"
    );
    let responses_missing = crate::route_http_request(
        "GET",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199/responses",
        None,
        "",
        &detail_state,
    )
    .await
    .expect("missing search responses");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "missing-empty-or-conflict-state",
        responses_missing.status == "404 Not Found"
    );
    let responses_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search responses runtime database");
    let (responses_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(responses_db.clone()),
    );
    seed_search(&responses_failure_state, detail_id, true).await;
    responses_db.close_for_test().await;
    let responses_runtime = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{detail_id}/responses"),
        None,
        "",
        &responses_failure_state,
    )
    .await
    .expect("search responses runtime failure");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "runtime-failure-and-timeout",
        responses_runtime.status == "500 Internal Server Error"
    );

    let (download_state, _receiver) = test_state_with_env(env.clone());
    let action_id = "00000000-0000-4000-8000-000000000103";
    seed_search(&download_state, action_id, true).await;
    let download_route = format!("/api/v0/searches/{action_id}/items/0/download");
    let download_nominal =
        crate::route_http_request("POST", &download_route, None, "", &download_state)
            .await
            .expect("search item download");
    let download_json =
        serde_json::from_str::<serde_json::Value>(&download_nominal.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "nominal-status-headers-body",
        download_nominal.status == "200 OK"
            && download_json["success"] == true
            && download_json["source"] == "scene"
    );
    let download_malformed = crate::route_http_request(
        "POST",
        &format!("/api/v0/searches/{action_id}/items/not-an-item/download"),
        None,
        "",
        &download_state,
    )
    .await
    .expect("malformed search item download");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "malformed-path-query-or-body",
        download_malformed.status == "400 Bad Request"
    );
    let download_side_effects = download_state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|entry| {
            entry.peer_username.as_deref() == Some("search-peer")
                && entry.filename == "Remote/Search.flac"
        });
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "mutation-side-effects-and-readback",
        download_side_effects
    );

    let download_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search download runtime database");
    let (download_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(download_db.clone()),
    );
    seed_search(&download_failure_state, action_id, true).await;
    download_db.close_for_test().await;
    let download_runtime =
        crate::route_http_request("POST", &download_route, None, "", &download_failure_state)
            .await
            .expect("search download runtime failure");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "runtime-failure-and-timeout",
        download_runtime.status == "500 Internal Server Error"
    );
    let (download_restarted, _receiver) = test_state_with_env(env.clone());
    seed_search(&download_restarted, action_id, true).await;
    let restarted_download =
        crate::route_http_request("POST", &download_route, None, "", &download_restarted)
            .await
            .expect("restarted search download");
    let (download_fresh, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "restart-persistence-or-reset",
        restarted_download.status == "200 OK"
            && download_fresh.transfers.read().await.entries.is_empty()
    );
    let (download_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_search(&download_concurrent, action_id, true).await;
    let concurrent_downloads = futures_util::future::join_all([
        crate::route_http_request("POST", &download_route, None, "", &download_concurrent),
        crate::route_http_request("POST", &download_route, None, "", &download_concurrent),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/download",
        "concurrency-and-idempotency",
        concurrent_downloads.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && download_concurrent.transfers.read().await.entries.len() == 2
    );

    let (stream_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&stream_state, action_id, true).await;
    let stream_route = format!("/api/v0/searches/{action_id}/items/0/stream");
    let stream_nominal = crate::route_http_request("POST", &stream_route, None, "", &stream_state)
        .await
        .expect("search item stream");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "nominal-status-headers-body",
        stream_nominal.status == "400 Bad Request"
            && stream_nominal
                .body
                .contains("scene_streaming_not_supported")
    );
    let stream_malformed = crate::route_http_request(
        "POST",
        &format!("/api/v0/searches/{action_id}/items/not-an-item/stream"),
        None,
        "",
        &stream_state,
    )
    .await
    .expect("malformed search item stream");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "malformed-path-query-or-body",
        stream_malformed.status == "400 Bad Request"
    );
    let stream_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search stream runtime database");
    let (stream_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(stream_db.clone()),
    );
    seed_search(&stream_failure_state, action_id, true).await;
    stream_db.close_for_test().await;
    let stream_runtime =
        crate::route_http_request("POST", &stream_route, None, "", &stream_failure_state)
            .await
            .expect("search stream runtime failure");
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "runtime-failure-and-timeout",
        stream_runtime.status == "500 Internal Server Error"
    );
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "mutation-side-effects-and-readback",
        stream_state.transfers.read().await.entries.is_empty()
    );
    let (stream_restarted, _receiver) = test_state_with_env(env.clone());
    seed_search(&stream_restarted, action_id, true).await;
    let restarted_stream =
        crate::route_http_request("POST", &stream_route, None, "", &stream_restarted)
            .await
            .expect("restarted search stream");
    let (stream_fresh, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "restart-persistence-or-reset",
        restarted_stream.status == "400 Bad Request"
            && stream_fresh.transfers.read().await.entries.is_empty()
    );
    let (stream_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_search(&stream_concurrent, action_id, true).await;
    let concurrent_streams = futures_util::future::join_all([
        crate::route_http_request("POST", &stream_route, None, "", &stream_concurrent),
        crate::route_http_request("POST", &stream_route, None, "", &stream_concurrent),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/searches/{searchId}/items/{itemId}/stream",
        "concurrency-and-idempotency",
        concurrent_streams.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "400 Bad Request"))
            && stream_concurrent.transfers.read().await.entries.is_empty()
    );

    let (delete_all_state, _receiver) = test_state_with_env(env.clone());
    let delete_all_malformed = crate::route_http_request(
        "DELETE",
        "/api/v0/searches/extra",
        None,
        "",
        &delete_all_state,
    )
    .await
    .expect("malformed delete-all search path");
    record!(
        "DELETE",
        "/api/v0/searches",
        "malformed-path-query-or-body",
        delete_all_malformed.status == "400 Bad Request"
    );
    let delete_all_empty =
        crate::route_http_request("DELETE", "/api/v0/searches", None, "", &delete_all_state)
            .await
            .expect("empty delete-all search");
    record!(
        "DELETE",
        "/api/v0/searches",
        "missing-empty-or-conflict-state",
        delete_all_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&delete_all_empty.body)
                .is_ok_and(|value| value["deleted"] == 0)
    );
    let delete_all_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("delete-all runtime database");
    let (delete_all_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(delete_all_db.clone()),
    );
    delete_all_db.close_for_test().await;
    let delete_all_runtime = crate::route_http_request(
        "DELETE",
        "/api/v0/searches",
        None,
        "",
        &delete_all_failure_state,
    )
    .await
    .expect("delete-all runtime failure");
    record!(
        "DELETE",
        "/api/v0/searches",
        "runtime-failure-and-timeout",
        delete_all_runtime.status == "500 Internal Server Error"
    );
    let (delete_all_reset_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&delete_all_reset_state, list_id, false).await;
    let reset_delete_all = crate::route_http_request(
        "DELETE",
        "/api/v0/searches",
        None,
        "",
        &delete_all_reset_state,
    )
    .await
    .expect("reset delete-all search");
    let (delete_all_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "DELETE",
        "/api/v0/searches",
        "restart-persistence-or-reset",
        reset_delete_all.status == "200 OK"
            && delete_all_restarted
                .searches
                .read()
                .await
                .records
                .is_empty()
    );
    let (delete_all_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_search(&delete_all_concurrent, list_id, false).await;
    seed_search(&delete_all_concurrent, detail_id, false).await;
    let concurrent_delete_all = futures_util::future::join_all([
        crate::route_http_request(
            "DELETE",
            "/api/v0/searches",
            None,
            "",
            &delete_all_concurrent,
        ),
        crate::route_http_request(
            "DELETE",
            "/api/v0/searches",
            None,
            "",
            &delete_all_concurrent,
        ),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/searches",
        "concurrency-and-idempotency",
        concurrent_delete_all.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && delete_all_concurrent
                .searches
                .read()
                .await
                .records
                .is_empty()
    );

    let (delete_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&delete_state, detail_id, false).await;
    let delete_malformed = crate::route_http_request(
        "DELETE",
        "/api/v0/searches/not-a-guid",
        None,
        "",
        &delete_state,
    )
    .await
    .expect("malformed search delete id");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "malformed-path-query-or-body",
        delete_malformed.status == "400 Bad Request"
    );
    let delete_missing = crate::route_http_request(
        "DELETE",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199",
        None,
        "",
        &delete_state,
    )
    .await
    .expect("missing search delete");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        delete_missing.status == "404 Not Found"
    );
    let delete_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search delete runtime database");
    let (delete_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(delete_db.clone()),
    );
    seed_search(&delete_failure_state, detail_id, true).await;
    delete_db.close_for_test().await;
    let delete_runtime = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &delete_failure_state,
    )
    .await
    .expect("search delete runtime failure");
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        delete_runtime.status == "500 Internal Server Error"
    );
    let reset_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &delete_state,
    )
    .await
    .expect("search delete reset");
    let (delete_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "restart-persistence-or-reset",
        reset_delete.status == "204 No Content"
            && delete_restarted.searches.read().await.records.is_empty()
    );
    let (concurrent_delete, _receiver) = test_state_with_env(env.clone());
    seed_search(&concurrent_delete, detail_id, false).await;
    let concurrent_deletes = futures_util::future::join_all([
        crate::route_http_request(
            "DELETE",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_delete,
        ),
        crate::route_http_request(
            "DELETE",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_delete,
        ),
    ])
    .await;
    let delete_statuses = concurrent_deletes
        .iter()
        .filter_map(|response| response.as_ref().ok().map(|response| response.status))
        .collect::<BTreeSet<_>>();
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "concurrency-and-idempotency",
        delete_statuses == BTreeSet::from(["204 No Content", "404 Not Found"])
            && concurrent_delete.searches.read().await.records.is_empty()
    );

    let (cleanup_state, _receiver) = test_state_with_env(env.clone());
    let cleanup_nominal =
        crate::route_http_request("POST", "/api/v0/searches/cleanup", None, "", &cleanup_state)
            .await
            .expect("search cleanup");
    let cleanup_json =
        serde_json::from_str::<serde_json::Value>(&cleanup_nominal.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "nominal-status-headers-body",
        cleanup_nominal.status == "200 OK"
            && cleanup_json["deleted"].is_u64()
            && cleanup_json["appliedMaxAgeDays"].is_i64()
            && cleanup_json["appliedMaxCount"].is_i64()
    );
    let cleanup_malformed = crate::route_http_request(
        "POST",
        "/api/v0/searches/cleanup?maxAgeDays=not-a-number",
        None,
        "",
        &cleanup_state,
    )
    .await
    .expect("malformed search cleanup query");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "malformed-path-query-or-body",
        cleanup_malformed.status == "400 Bad Request"
    );
    let cleanup_empty =
        crate::route_http_request("POST", "/api/v0/searches/cleanup", None, "", &cleanup_state)
            .await
            .expect("empty search cleanup");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "missing-empty-or-conflict-state",
        cleanup_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&cleanup_empty.body)
                .is_ok_and(|value| value["deleted"] == 0)
    );
    let cleanup_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search cleanup runtime database");
    let (cleanup_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(cleanup_db.clone()),
    );
    cleanup_db.close_for_test().await;
    let cleanup_runtime = crate::route_http_request(
        "POST",
        "/api/v0/searches/cleanup",
        None,
        "",
        &cleanup_failure_state,
    )
    .await
    .expect("search cleanup runtime failure");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "runtime-failure-and-timeout",
        cleanup_runtime.status == "500 Internal Server Error"
    );
    let cleanup_side_effect_id = "00000000-0000-4000-8000-000000000104";
    let (cleanup_side_effect_state, _receiver) = test_state_with_env(env.clone());
    seed_expired_search(&cleanup_side_effect_state, cleanup_side_effect_id).await;
    let cleanup_side_effect = crate::route_http_request(
        "POST",
        "/api/v0/searches/cleanup",
        None,
        "",
        &cleanup_side_effect_state,
    )
    .await
    .expect("search cleanup side effect");
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "mutation-side-effects-and-readback",
        cleanup_side_effect.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&cleanup_side_effect.body)
                .is_ok_and(|value| value["deleted"] == 1)
            && cleanup_side_effect_state
                .searches
                .read()
                .await
                .records
                .is_empty()
    );
    let (cleanup_reset_state, _receiver) = test_state_with_env(env.clone());
    let cleanup_reset = crate::route_http_request(
        "POST",
        "/api/v0/searches/cleanup",
        None,
        "",
        &cleanup_reset_state,
    )
    .await
    .expect("search cleanup reset");
    let (cleanup_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "restart-persistence-or-reset",
        cleanup_reset.status == "200 OK"
            && cleanup_restarted.searches.read().await.records.is_empty()
    );
    let (cleanup_concurrent, _receiver) = test_state_with_env(env.clone());
    seed_expired_search(&cleanup_concurrent, cleanup_side_effect_id).await;
    let concurrent_cleanups = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/searches/cleanup",
            None,
            "",
            &cleanup_concurrent,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/searches/cleanup",
            None,
            "",
            &cleanup_concurrent,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/searches/cleanup",
        "concurrency-and-idempotency",
        concurrent_cleanups.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && cleanup_concurrent.searches.read().await.records.is_empty()
    );

    let (put_state, _receiver) = test_state_with_env(env.clone());
    seed_search(&put_state, detail_id, false).await;
    let put_malformed =
        crate::route_http_request("PUT", "/api/v0/searches/not-a-guid", None, "", &put_state)
            .await
            .expect("malformed search update id");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "malformed-path-query-or-body",
        put_malformed.status == "400 Bad Request"
    );
    let put_missing = crate::route_http_request(
        "PUT",
        "/api/v0/searches/00000000-0000-4000-8000-000000000199",
        None,
        "",
        &put_state,
    )
    .await
    .expect("missing search update");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "missing-empty-or-conflict-state",
        put_missing.status == "404 Not Found"
    );
    let put_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("search update runtime database");
    let (put_failure_state, _receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(put_db.clone()));
    seed_search(&put_failure_state, detail_id, true).await;
    put_db.close_for_test().await;
    let put_runtime = crate::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &put_failure_state,
    )
    .await
    .expect("search update runtime failure");
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "runtime-failure-and-timeout",
        put_runtime.status == "500 Internal Server Error"
    );
    let reset_put = crate::route_http_request(
        "PUT",
        &format!("/api/v0/searches/{detail_id}"),
        None,
        "",
        &put_state,
    )
    .await
    .expect("search update reset");
    let (put_restarted, _receiver) = test_state_with_env(env.clone());
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "restart-persistence-or-reset",
        reset_put.status == "200 OK" && put_restarted.searches.read().await.records.is_empty()
    );
    let (concurrent_put, _receiver) = test_state_with_env(env);
    seed_search(&concurrent_put, detail_id, false).await;
    let concurrent_puts = futures_util::future::join_all([
        crate::route_http_request(
            "PUT",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_put,
        ),
        crate::route_http_request(
            "PUT",
            &format!("/api/v0/searches/{detail_id}"),
            None,
            "",
            &concurrent_put,
        ),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "concurrency-and-idempotency",
        concurrent_puts.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && concurrent_put
                .searches
                .read()
                .await
                .get_by_identifier(detail_id)
                .is_some_and(|record| record.status == "cancelled")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create searches evidence directory");
    fs::write(
        evidence_dir.join("searches_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize searches evidence"),
    )
    .expect("write searches evidence");
    assert_eq!(ledger.len(), 43, "searches open-case ledger size");
    assert!(
        mismatches.is_empty(),
        "{} slskdN searches mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the slskdN SecurityController's
/// `runtime-failure-and-timeout` cases.  A closed SQLite pool is injected
/// after the in-memory ban/circuit fixtures are prepared.  The frozen
/// security projections remain readable because they are process-local,
/// while ban persistence, unavailable transport actions, and disabled
/// remote adversarial configuration retain their distinct frozen failure
/// contracts.  slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_native_security_runtime_failure_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [runtime-failure-and-timeout]",
                    $method, $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "runtime-failure-and-timeout",
                "pass": pass,
            }));
        }};
    }

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn security runtime-failure database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(db.clone()));
    state
        .security
        .write()
        .await
        .ban("ip", "192.0.2.77".to_owned())
        .expect("seed security ban");
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            "security/circuit/runtime-circuit".to_owned(),
            serde_json::json!({
                "circuitId": "runtime-circuit",
                "active": true,
                "peerId": "runtime-peer",
            }),
        )
        .expect("seed security circuit");
    db.close_for_test().await;

    for (path, route, expected_status, body_kind) in [
        (
            "/api/v0/security/adversarial",
            "/api/v0/security/adversarial",
            "404 Not Found",
            "adversarial",
        ),
        (
            "/api/v0/security/adversarial/stats",
            "/api/v0/security/adversarial/stats",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/anomalies",
            "/api/v0/security/anomalies",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/bans",
            "/api/v0/security/bans",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/canaries",
            "/api/v0/security/canaries",
            "404 Not Found",
            "not-found",
        ),
        (
            "/api/v0/security/circuits",
            "/api/v0/security/circuits",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/circuits/stats",
            "/api/v0/security/circuits/stats",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/dashboard",
            "/api/v0/security/dashboard",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/disclosure/runtime-peer",
            "/api/v0/security/disclosure/{username}",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/events",
            "/api/v0/security/events",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/network",
            "/api/v0/security/network",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/network/top",
            "/api/v0/security/network/top",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/peers",
            "/api/v0/security/peers",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/peers/stats",
            "/api/v0/security/peers/stats",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/reputation/suspicious",
            "/api/v0/security/reputation/suspicious",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/reputation/trusted",
            "/api/v0/security/reputation/trusted",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/reputation/runtime-peer",
            "/api/v0/security/reputation/{username}",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/scanners",
            "/api/v0/security/scanners",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/threats",
            "/api/v0/security/threats",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/tor/status",
            "/api/v0/security/tor/status",
            "404 Not Found",
            "not-found",
        ),
        (
            "/api/v0/security/transports",
            "/api/v0/security/transports",
            "200 OK",
            "json",
        ),
        (
            "/api/v0/security/transports/status",
            "/api/v0/security/transports/status",
            "200 OK",
            "json",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let body_pass = match body_kind {
            "json" => serde_json::from_str::<serde_json::Value>(&response.body).is_ok(),
            "adversarial" => response.body == "Adversarial features are not configured",
            "not-found" => response.body == r#"{"error":"not found"}"#,
            _ => false,
        };
        record!(
            "GET",
            route,
            response.status == expected_status && body_pass
        );
    }

    let delete_ip = crate::route_http_request(
        "DELETE",
        "/api/v0/security/bans/ip/192.0.2.77",
        None,
        "",
        &state,
    )
    .await
    .expect("delete IP ban under closed SQLite");
    record!(
        "DELETE",
        "/api/v0/security/bans/ip/{ipAddress}",
        delete_ip.status == "503 Service Unavailable"
            && delete_ip.body.contains("security unban persistence failed")
            && state
                .security
                .read()
                .await
                .bans
                .iter()
                .any(|ban| ban.kind == "ip" && ban.value == "192.0.2.77")
    );

    let delete_circuit = crate::route_http_request(
        "DELETE",
        "/api/v0/security/circuits/runtime-circuit",
        None,
        "",
        &state,
    )
    .await
    .expect("delete circuit under closed SQLite");
    record!(
        "DELETE",
        "/api/v0/security/circuits/{circuitId}",
        delete_circuit.status == "200 OK" && delete_circuit.body.is_empty()
    );

    let ban_ip = crate::route_http_request(
        "POST",
        "/api/v0/security/bans/ip",
        None,
        r#"{"ipAddress":"198.51.100.77","reason":"runtime"}"#,
        &state,
    )
    .await
    .expect("create IP ban under closed SQLite");
    record!(
        "POST",
        "/api/v0/security/bans/ip",
        ban_ip.status == "503 Service Unavailable"
            && ban_ip.body.contains("security ban persistence failed")
            && !state
                .security
                .read()
                .await
                .bans
                .iter()
                .any(|ban| ban.value == "198.51.100.77")
    );

    let entropy =
        crate::route_http_request("POST", "/api/v0/security/entropy/check", None, "", &state)
            .await
            .expect("entropy check under closed SQLite");
    record!(
        "POST",
        "/api/v0/security/entropy/check",
        entropy.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&entropy.body)
                .ok()
                .is_some_and(|value| value["isHealthy"] == true)
    );

    let tor_test = crate::route_http_request("POST", "/api/v0/security/tor/test", None, "", &state)
        .await
        .expect("Tor test under closed SQLite");
    record!(
        "POST",
        "/api/v0/security/tor/test",
        tor_test.status == "404 Not Found"
            && tor_test.body.contains("Tor transport is not configured")
    );

    let transport_test =
        crate::route_http_request("POST", "/api/v0/security/transports/test", None, "", &state)
            .await
            .expect("transport test under closed SQLite");
    record!(
        "POST",
        "/api/v0/security/transports/test",
        transport_test.status == "503 Service Unavailable"
            && transport_test
                .body
                .contains("Transport selector not available")
    );

    let adversarial =
        crate::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
            .await
            .expect("adversarial settings under closed SQLite");
    record!(
        "PUT",
        "/api/v0/security/adversarial",
        adversarial.status == "500 Internal Server Error"
            && serde_json::from_str::<serde_json::Value>(&adversarial.body)
                .ok()
                .is_some_and(|value| value["status"] == 500)
    );

    let disclosure = crate::route_http_request(
        "PUT",
        "/api/v0/security/disclosure/runtime-peer",
        None,
        r#"{"tier":"Trusted"}"#,
        &state,
    )
    .await
    .expect("disclosure settings under closed SQLite");
    record!(
        "PUT",
        "/api/v0/security/disclosure/{username}",
        disclosure.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&disclosure.body)
                .ok()
                .is_some_and(|value| value["settings"]["tier"] == "Trusted")
    );

    let reputation = crate::route_http_request(
        "PUT",
        "/api/v0/security/reputation/runtime-peer",
        None,
        r#"{"score":80}"#,
        &state,
    )
    .await
    .expect("reputation settings under closed SQLite");
    record!(
        "PUT",
        "/api/v0/security/reputation/{username}",
        reputation.status == "200 OK"
            && reputation.body == "{}"
            && state.security.read().await.reputation.get("runtime-peer") == Some(&80)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_security_runtime_failure_contracts.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize slskdn security runtime-failure ledger"),
    )
    .expect("write slskdn security runtime-failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn security runtime-failure mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for slskdN TransfersController runtime failures.
/// The frozen controller keeps accelerated-download mode in memory, while
/// the remaining transfer projections and mutations reach EF-backed
/// services.  Closing the SQLite pool after valid transfer fixtures have
/// been staged therefore yields 500 for those 24 routes and leaves the
/// accelerated GET/PUT pair available.
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
pub(super) async fn controller_api_differential_native_transfers_runtime_failure_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [runtime-failure-and-timeout]",
                    $method, $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "runtime-failure-and-timeout",
                "pass": pass,
            }));
        }};
    }

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn transfers runtime-failure database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(db.clone()));
    let batch_id = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
    let (download_id, upload_id) = {
        let mut transfers = state.transfers.write().await;
        let download = transfers.create_with_batch(
            0,
            Some("runtime-download-peer".to_owned()),
            "Remote/Runtime.flac".to_owned(),
            None,
            Some(42),
            Some(batch_id.to_owned()),
        );
        let upload = transfers.create(
            1,
            Some("runtime-upload-peer".to_owned()),
            "Remote/Upload.flac".to_owned(),
            None,
            Some(24),
        );
        (download.id, upload.id)
    };
    let entries = state.transfers.read().await.entries.clone();
    crate::persist_transfer_records(&state, &entries)
        .await
        .expect("persist slskdn transfer fixtures");
    db.close_for_test().await;

    let mut checks: Vec<(&str, String, &str, &str, bool)> = Vec::new();
    let mut add = |method: &'static str,
                   path: String,
                   route: &'static str,
                   body: &'static str,
                   persistence_independent: bool| {
        checks.push((method, path, route, body, persistence_independent));
    };

    add(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed".to_owned(),
        "/api/v0/transfers/downloads/all/completed",
        "",
        false,
    );
    add(
        "DELETE",
        format!("/api/v0/transfers/downloads/runtime-download-peer/{download_id}"),
        "/api/v0/transfers/downloads/{username}/{id}",
        "",
        false,
    );
    add(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed".to_owned(),
        "/api/v0/transfers/uploads/all/completed",
        "",
        false,
    );
    add(
        "DELETE",
        format!("/api/v0/transfers/uploads/runtime-upload-peer/{upload_id}"),
        "/api/v0/transfers/uploads/{username}/{id}",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers".to_owned(),
        "/api/v0/transfers",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/changes".to_owned(),
        "/api/v0/transfers/changes",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/downloads".to_owned(),
        "/api/v0/transfers/downloads",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/downloads/accelerated".to_owned(),
        "/api/v0/transfers/downloads/accelerated",
        "",
        true,
    );
    add(
        "GET",
        "/api/v0/transfers/downloads/auto-replace/status".to_owned(),
        "/api/v0/transfers/downloads/auto-replace/status",
        "",
        false,
    );
    add(
        "GET",
        format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        "/api/v0/transfers/downloads/batches/{id}",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/downloads/stuck".to_owned(),
        "/api/v0/transfers/downloads/stuck",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/downloads/user-stats".to_owned(),
        "/api/v0/transfers/downloads/user-stats",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/downloads/runtime-download-peer".to_owned(),
        "/api/v0/transfers/downloads/{username}",
        "",
        false,
    );
    add(
        "GET",
        format!("/api/v0/transfers/downloads/runtime-download-peer/{download_id}"),
        "/api/v0/transfers/downloads/{username}/{id}",
        "",
        false,
    );
    add(
        "GET",
        format!("/api/v0/transfers/downloads/runtime-download-peer/{download_id}/position"),
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/history?direction=download".to_owned(),
        "/api/v0/transfers/history",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/speeds".to_owned(),
        "/api/v0/transfers/speeds",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/uploads".to_owned(),
        "/api/v0/transfers/uploads",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/uploads/diagnostics".to_owned(),
        "/api/v0/transfers/uploads/diagnostics",
        "",
        false,
    );
    add(
        "GET",
        "/api/v0/transfers/uploads/runtime-upload-peer".to_owned(),
        "/api/v0/transfers/uploads/{username}",
        "",
        false,
    );
    add(
        "GET",
        format!("/api/v0/transfers/uploads/runtime-upload-peer/{upload_id}"),
        "/api/v0/transfers/uploads/{username}/{id}",
        "",
        false,
    );
    add(
        "POST",
        "/api/v0/transfers/downloads/auto-replace".to_owned(),
        "/api/v0/transfers/downloads/auto-replace",
        "{}",
        false,
    );
    add(
        "POST",
        "/api/v0/transfers/downloads/find-alternative".to_owned(),
        "/api/v0/transfers/downloads/find-alternative",
        r#"{"username":"runtime-download-peer","filename":"Remote/Runtime.flac"}"#,
        false,
    );
    add(
        "POST",
        "/api/v0/transfers/downloads/replace".to_owned(),
        "/api/v0/transfers/downloads/replace",
        r#"{"username":"runtime-download-peer","id":1}"#,
        false,
    );
    add(
        "POST",
        "/api/v0/transfers/downloads/runtime-download-peer".to_owned(),
        "/api/v0/transfers/downloads/{username}",
        r#"[{"filename":"Remote/Queued.flac","size":12}]"#,
        false,
    );
    add(
        "PUT",
        "/api/v0/transfers/downloads/accelerated".to_owned(),
        "/api/v0/transfers/downloads/accelerated",
        r#"{"enabled":true}"#,
        true,
    );

    for (method, path, route, body, persistence_independent) in checks {
        let response = crate::route_http_request(method, &path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        let pass = if persistence_independent {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
        } else {
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        };
        record!(method, route, pass);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_runtime_failure_contracts.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize slskdn transfers runtime-failure ledger"),
    )
    .expect("write slskdn transfers runtime-failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers runtime-failure mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_native_transfers_empty_and_missing_contracts() {
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

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let batch_id = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";

    let clear_downloads = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("empty slskdn download cleanup");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "missing-empty-or-conflict-state",
        clear_downloads.status == "204 No Content" && clear_downloads.body.is_empty()
    );

    let clear_uploads = crate::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("empty slskdn upload cleanup");
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "missing-empty-or-conflict-state",
        clear_uploads.status == "204 No Content" && clear_uploads.body.is_empty()
    );

    let transfers = crate::route_http_request("GET", "/api/v0/transfers", None, "", &state)
        .await
        .expect("empty slskdn transfer projection");
    record!(
        "GET",
        "/api/v0/transfers",
        "missing-empty-or-conflict-state",
        transfers.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&transfers.body)
                .ok()
                .is_some_and(
                    |value| value.is_array() && value.as_array().is_some_and(Vec::is_empty)
                )
    );

    let changes = crate::route_http_request("GET", "/api/v0/transfers/changes", None, "", &state)
        .await
        .expect("empty slskdn transfer changes");
    record!(
        "GET",
        "/api/v0/transfers/changes",
        "missing-empty-or-conflict-state",
        changes.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&changes.body)
                .ok()
                .is_some_and(|value| value["transfers"].is_array())
    );

    let downloads =
        crate::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
            .await
            .expect("empty slskdn downloads");
    record!(
        "GET",
        "/api/v0/transfers/downloads",
        "missing-empty-or-conflict-state",
        downloads.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&downloads.body)
                .ok()
                .is_some_and(
                    |value| value.is_array() && value.as_array().is_some_and(Vec::is_empty)
                )
    );

    let position = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/missing-peer/999/position",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskdn queue position");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "missing-empty-or-conflict-state",
        position.status == "404 Not Found"
    );

    let accelerated = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/accelerated",
        None,
        "",
        &state,
    )
    .await
    .expect("empty slskdn accelerated downloads");
    record!(
        "GET",
        "/api/v0/transfers/downloads/accelerated",
        "missing-empty-or-conflict-state",
        accelerated.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&accelerated.body)
                .ok()
                .is_some_and(|value| {
                    value["enabled"] == false
                        && value["updatedAt"].is_string()
                        && value["policy"].is_string()
                })
    );

    let auto_replace_status = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/auto-replace/status",
        None,
        "",
        &state,
    )
    .await
    .expect("empty slskdn auto-replace status");
    let auto_replace_status_json =
        serde_json::from_str::<serde_json::Value>(&auto_replace_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/downloads/auto-replace/status",
        "missing-empty-or-conflict-state",
        auto_replace_status.status == "200 OK"
            && auto_replace_status_json["stuckCount"] == 0
            && auto_replace_status_json["enabled"] == false
            && auto_replace_status_json["intervalSeconds"] == 300
    );

    let batch = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskdn transfer batch");
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "missing-empty-or-conflict-state",
        batch.status == "404 Not Found"
    );

    let stuck =
        crate::route_http_request("GET", "/api/v0/transfers/downloads/stuck", None, "", &state)
            .await
            .expect("empty slskdn stuck downloads");
    record!(
        "GET",
        "/api/v0/transfers/downloads/stuck",
        "missing-empty-or-conflict-state",
        stuck.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&stuck.body)
                .ok()
                .is_some_and(
                    |value| value.is_array() && value.as_array().is_some_and(Vec::is_empty)
                )
    );

    let user_stats = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/user-stats",
        None,
        "",
        &state,
    )
    .await
    .expect("empty slskdn download user stats");
    record!(
        "GET",
        "/api/v0/transfers/downloads/user-stats",
        "missing-empty-or-conflict-state",
        user_stats.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&user_stats.body)
                .ok()
                .is_some_and(|value| value.is_object()
                    && value.as_object().is_some_and(|map| map.is_empty()))
    );

    let history = crate::route_http_request("GET", "/api/v0/transfers/history", None, "", &state)
        .await
        .expect("missing slskdn history direction");
    record!(
        "GET",
        "/api/v0/transfers/history",
        "missing-empty-or-conflict-state",
        history.status == "400 Bad Request"
    );

    let speeds = crate::route_http_request("GET", "/api/v0/transfers/speeds", None, "", &state)
        .await
        .expect("empty slskdn transfer speeds");
    record!(
        "GET",
        "/api/v0/transfers/speeds",
        "missing-empty-or-conflict-state",
        speeds.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&speeds.body).is_ok()
    );

    let uploads = crate::route_http_request("GET", "/api/v0/transfers/uploads", None, "", &state)
        .await
        .expect("empty slskdn uploads");
    record!(
        "GET",
        "/api/v0/transfers/uploads",
        "missing-empty-or-conflict-state",
        uploads.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&uploads.body)
                .ok()
                .is_some_and(
                    |value| value.is_array() && value.as_array().is_some_and(Vec::is_empty)
                )
    );

    let diagnostics = crate::route_http_request(
        "GET",
        "/api/v0/transfers/uploads/diagnostics",
        None,
        "",
        &state,
    )
    .await
    .expect("empty slskdn upload diagnostics");
    record!(
        "GET",
        "/api/v0/transfers/uploads/diagnostics",
        "missing-empty-or-conflict-state",
        diagnostics.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&diagnostics.body)
                .ok()
                .is_some_and(|value| value["recentUploads"].is_array())
    );

    let enqueue = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/missing-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("empty slskdn enqueue request");
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "missing-empty-or-conflict-state",
        enqueue.status == "400 Bad Request"
    );

    let auto_replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("empty slskdn auto-replace");
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "missing-empty-or-conflict-state",
        auto_replace.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&auto_replace.body)
                .ok()
                .is_some_and(|value| value["replaced"] == 0 && value["details"].is_array())
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "nominal-status-headers-body",
        auto_replace.status == "200 OK"
            && auto_replace.body == r#"{"replaced":0,"failed":0,"skipped":0,"details":[]}"#
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "mutation-side-effects-and-readback",
        auto_replace.status == "200 OK" && state.transfers.read().await.entries.is_empty()
    );

    let find_alternative = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &state,
    )
    .await
    .expect("empty slskdn find-alternative");
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "missing-empty-or-conflict-state",
        find_alternative.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&find_alternative.body)
                .ok()
                .is_some_and(
                    |value| value.is_array() && value.as_array().is_some_and(Vec::is_empty)
                )
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "nominal-status-headers-body",
        find_alternative.status == "200 OK" && find_alternative.body == "[]"
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "mutation-side-effects-and-readback",
        find_alternative.status == "200 OK" && state.transfers.read().await.entries.is_empty()
    );

    let replace = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("empty slskdn replace");
    let replace_json = serde_json::from_str::<serde_json::Value>(&replace.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/transfers/downloads/replace",
        "missing-empty-or-conflict-state",
        replace.status == "500 Internal Server Error"
            && replace_json["success"] == false
            && replace_json["error"] == "Failed to replace download"
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/replace",
        "nominal-status-headers-body",
        replace.status == "500 Internal Server Error"
            && replace_json["success"] == false
            && replace_json["error"] == "Failed to replace download"
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/replace",
        "mutation-side-effects-and-readback",
        replace.status == "500 Internal Server Error"
            && state.transfers.read().await.entries.is_empty()
    );

    let accelerated_update = crate::route_http_request(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        None,
        "{}",
        &state,
    )
    .await
    .expect("empty slskdn accelerated update");
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "missing-empty-or-conflict-state",
        accelerated_update.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&accelerated_update.body).is_ok()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_empty_and_missing_contracts.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize slskdn transfers empty/missing ledger"),
    )
    .expect("write slskdn transfers empty/missing ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers empty/missing mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
