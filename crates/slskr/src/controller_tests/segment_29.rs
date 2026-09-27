/// Differential proof for the remaining SongID controller edge cases.
/// The frozen controller uses exact routes, a GUID-constrained run id,
/// empty-state DTOs, and a durable run store; these probes exercise those
/// boundaries in addition to the already-covered nominal lifecycle.
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
async fn controller_api_differential_songid_open_cases() {
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
    let json_value = |response: &super::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let capabilities_contract = |response: &super::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value.as_array().is_some_and(|capabilities| {
                !capabilities.is_empty()
                    && capabilities.iter().all(|capability| {
                        capability["id"].is_string()
                            && capability["label"].is_string()
                            && capability["status"].is_string()
                            && capability["available"].is_boolean()
                            && capability["reason"].is_string()
                            && capability["requirements"].is_array()
                    })
                    && capabilities
                        .iter()
                        .any(|capability| capability["id"] == "text_query")
            })
    };
    let runs_contract = |response: &super::routing::HttpResponse, populated: bool| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value.as_array().is_some_and(|runs| {
                (!populated || !runs.is_empty())
                    && runs.iter().all(|run| {
                        run["id"].is_string()
                            && run["status"].is_string()
                            && run["source"].is_string()
                    })
            })
    };
    let queue_contract = |response: &super::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["queuedCount"].is_u64()
            && value["runningCount"].is_u64()
            && value["completedCount"].is_u64()
            && value["failedCount"].is_u64()
            && value["maxConcurrentRuns"].is_u64()
            && value["activeRuns"].is_array()
    };
    let forensic_contract = |response: &super::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["run_id"].is_string()
            && value["matrix"].is_array()
            && value["count"].is_u64()
    };
    // GET /capabilities: exact routing, empty-state capability DTOs,
    // closed-database independence, and dynamic tool/config state.
    {
        let path = "/api/v0/songid/capabilities";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            super::route_http_request("GET", "/api/v0/songid/capabilities/extra", None, "", &state)
                .await
                .expect("SongID capabilities malformed path response");
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID capabilities empty response");
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            capabilities_contract(&empty)
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("SongID capabilities runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .expect("SongID capabilities closed-database response");
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            capabilities_contract(&runtime)
        );

        let (populated_state, _receiver) = test_state_with_env(target_env());
        let populated = super::route_http_request("GET", path, None, "", &populated_state)
            .await
            .expect("SongID capabilities populated response");
        let populated_value = json_value(&populated);
        record!(
            "GET",
            path,
            "populated-dynamic-state",
            capabilities_contract(&populated)
                && populated_value
                    .as_array()
                    .is_some_and(|capabilities| capabilities.len() >= 10)
        );
    }

    // GET /runs: exact routing, empty store, closed-database behavior,
    // and a list populated by a real run mutation.
    {
        let path = "/api/v0/songid/runs";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            super::route_http_request("GET", "/api/v0/songid/runs/extra", None, "", &state)
                .await
                .expect("SongID runs malformed path response");
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID runs empty response");
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            runs_contract(&empty, false)
                && json_value(&empty)
                    .as_array()
                    .is_some_and(|runs| runs.is_empty())
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("SongID runs runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .expect("SongID runs closed-database response");
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            runs_contract(&runtime, false)
        );

        let created = super::route_http_request(
            "POST",
            path,
            None,
            r#"{"source":"SongID list differential"}"#,
            &state,
        )
        .await
        .expect("SongID populated run");
        let listed = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID populated runs response");
        record!(
            "GET",
            path,
            "populated-dynamic-state",
            created.status == "202 Accepted" && runs_contract(&listed, true)
        );
    }

    // The three id-addressed GET routes share the frozen GUID-constrained
    // route boundary and differ only in their successful DTO contract.
    let unknown_id = "00000000-0000-0000-0000-000000000000";
    for (path, kind) in [
        ("/api/v0/songid/runs/{id:guid}", "run"),
        ("/api/v0/songid/runs/{id:guid}/evidence-package", "package"),
        ("/api/v0/songid/runs/{id:guid}/forensic-matrix", "forensic"),
    ] {
        let exact = match kind {
            "run" => format!("/api/v0/songid/runs/{unknown_id}"),
            "package" => format!("/api/v0/songid/runs/{unknown_id}/evidence-package"),
            "forensic" => format!("/api/v0/songid/runs/{unknown_id}/forensic-matrix"),
            _ => unreachable!(),
        };
        let malformed_path = match kind {
            "run" => "/api/v0/songid/runs/not-a-guid/extra".to_owned(),
            "package" => "/api/v0/songid/runs/not-a-guid/evidence-package/extra".to_owned(),
            "forensic" => "/api/v0/songid/runs/not-a-guid/forensic-matrix/extra".to_owned(),
            _ => unreachable!(),
        };
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} malformed path: {error}"));
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = super::route_http_request("GET", &exact, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} missing response: {error}"));
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} runtime database: {error}"));
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("GET", &exact, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("SongID {kind} runtime response: {error}"));
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "404 Not Found"
        );
    }

    // Forensic matrix also needs populated-state proof, using the real
    // synchronous SongID run produced by the route.
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let created = super::route_http_request(
            "POST",
            "/api/v0/songid/runs",
            None,
            r#"{"source":"SongID forensic differential"}"#,
            &state,
        )
        .await
        .expect("SongID forensic run");
        let created_value = json_value(&created);
        let run_id = created_value["id"].as_str().unwrap_or_default();
        let response = super::route_http_request(
            "GET",
            &format!("/api/v0/songid/runs/{run_id}/forensic-matrix"),
            None,
            "",
            &state,
        )
        .await
        .expect("SongID populated forensic matrix response");
        record!(
            "GET",
            "/api/v0/songid/runs/{id:guid}/forensic-matrix",
            "nominal-status-headers-body",
            forensic_contract(&response)
        );
        record!(
            "GET",
            "/api/v0/songid/runs/{id:guid}/forensic-matrix",
            "populated-dynamic-state",
            created.status == "202 Accepted" && forensic_contract(&response)
        );
    }

    // GET /queue: exact routing, empty counters, and closed-database
    // independence. Populated counters were covered by the completed
    // SongID lifecycle proof.
    {
        let path = "/api/v0/songid/runs/queue";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            super::route_http_request("GET", "/api/v0/songid/runs/queue/extra", None, "", &state)
                .await
                .expect("SongID queue malformed path response");
        record!(
            "GET",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("SongID queue empty response");
        let empty_value = json_value(&empty);
        record!(
            "GET",
            path,
            "missing-empty-or-conflict-state",
            queue_contract(&empty)
                && empty_value["queuedCount"] == 0
                && empty_value["runningCount"] == 0
                && empty_value["completedCount"] == 0
                && empty_value["failedCount"] == 0
                && empty_value["activeRuns"] == serde_json::json!([])
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("SongID queue runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .expect("SongID queue closed-database response");
        record!(
            "GET",
            path,
            "runtime-failure-and-timeout",
            queue_contract(&runtime)
        );
    }

    // POST /runs: empty model binding, reset semantics, and concurrent
    // run creation with distinct identifiers.
    {
        let path = "/api/v0/songid/runs";
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = super::route_http_request("POST", path, None, "", &state)
            .await
            .expect("SongID run empty body response");
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body.contains("SongID source is required.")
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset = super::route_http_request(
            "POST",
            path,
            None,
            r#"{"source":"SongID reset differential"}"#,
            &reset_state,
        )
        .await
        .expect("SongID reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "POST",
            path,
            None,
            r#"{"source":"SongID reset differential"}"#,
            &restarted_state,
        )
        .await
        .expect("SongID restarted response");
        let reset_value = json_value(&reset);
        let restarted_value = json_value(&restarted);
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            reset.status == "202 Accepted"
                && restarted.status == "202 Accepted"
                && reset_value["status"] == "completed"
                && restarted_value["status"] == "completed"
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                path,
                None,
                r#"{"source":"SongID concurrent differential"}"#,
                &concurrent_state
            ),
            super::route_http_request(
                "POST",
                path,
                None,
                r#"{"source":"SongID concurrent differential"}"#,
                &concurrent_state
            )
        );
        let left = left.expect("SongID left concurrency response");
        let right = right.expect("SongID right concurrency response");
        let left_value = json_value(&left);
        let right_value = json_value(&right);
        let left_id = left_value["id"].as_str().unwrap_or_default();
        let right_id = right_value["id"].as_str().unwrap_or_default();
        let readback = super::route_http_request("GET", path, None, "", &concurrent_state)
            .await
            .expect("SongID concurrency readback");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            left.status == "202 Accepted"
                && right.status == "202 Accepted"
                && left_value["status"] == "completed"
                && right_value["status"] == "completed"
                && !left_id.is_empty()
                && !right_id.is_empty()
                && left_id != right_id
                && runs_contract(&readback, true)
        );
    }

    assert_eq!(ledger.len(), 25, "SongID residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create SongID evidence directory");
    fs::write(
        evidence_dir.join("songid_controller_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize SongID ledger"),
    )
    .expect("write SongID ledger");
    assert!(
        mismatches.is_empty(),
        "{} SongID controller mismatches:\n{}",
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
async fn controller_api_differential_share_grants_open_cases() {
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

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };
    let missing_id = "00000000-0000-0000-0000-000000000000";

    // Read routes remain deterministic when their persistence backend is
    // unavailable: list is an empty array, while an unknown resource is
    // still NotFound.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grants list runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let list = super::route_http_request("GET", "/api/v0/share-grants", None, "", &state)
            .await
            .expect("share-grants list runtime response");
        let detail = super::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{missing_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("share-grants detail runtime response");
        record!(
            "GET",
            "/api/v0/share-grants",
            "runtime-failure-and-timeout",
            list.status == "200 OK" && parse_json(&list.body).is_array()
        );
        record!(
            "GET",
            "/api/v0/share-grants/{id}",
            "runtime-failure-and-timeout",
            detail.status == "404 Not Found"
        );
    }

    let (state, _receiver) = test_state_with_env(target_env());
    let collection_id = uuid::Uuid::new_v4().to_string();
    state
        .collections
        .write()
        .await
        .create_with_contract(
            collection_id.clone(),
            "tester".to_owned(),
            "Share Grant Open Cases".to_owned(),
            String::new(),
            "ShareList".to_owned(),
        )
        .expect("share-grants collection fixture");
    state
        .collections
        .write()
        .await
        .add_item(
            &collection_id,
            "content:share-grant-open-case".to_owned(),
            "Differential Artist".to_owned(),
            "Differential Title".to_owned(),
            "Audio".to_owned(),
        )
        .expect("share-grants collection item fixture")
        .expect("share-grants collection item");
    let grant_id = uuid::Uuid::new_v4().to_string();
    state
        .share_grants
        .write()
        .await
        .create_with_contract(
            Some(grant_id.clone()),
            collection_id.clone(),
            "recipient".to_owned(),
        )
        .expect("share-grants fixture");

    let manifest_path = format!("/api/v0/share-grants/{grant_id}/manifest");
    let manifest = super::route_http_request("GET", &manifest_path, None, "", &state)
        .await
        .expect("share-grants manifest nominal response");
    let manifest_value = parse_json(&manifest.body);
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "nominal-status-headers-body",
        manifest.status == "200 OK"
            && manifest_value["share"].is_object()
            && manifest_value["collection"].is_object()
            && manifest_value["items"].is_array()
            && manifest_value["itemCount"] == 1
    );
    let manifest_missing = super::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{missing_id}/manifest"),
        None,
        "",
        &state,
    )
    .await
    .expect("share-grants manifest missing response");
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "missing-empty-or-conflict-state",
        manifest_missing.status == "404 Not Found"
    );
    let manifest_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("share-grants manifest runtime database");
    let (manifest_runtime_state, _receiver) = test_state_with_env_parts(
        target_env(),
        super::SearchStore::new(),
        Some(manifest_runtime_db.clone()),
    );
    manifest_runtime_db.close_for_test().await;
    let manifest_runtime = super::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{missing_id}/manifest"),
        None,
        "",
        &manifest_runtime_state,
    )
    .await
    .expect("share-grants manifest runtime response");
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "runtime-failure-and-timeout",
        manifest_runtime.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "populated-dynamic-state",
        manifest_value["share"]["id"] == grant_id
            && manifest_value["collection"]["id"] == collection_id
            && manifest_value["items"]
                .as_array()
                .is_some_and(|items| items.len() == 1)
    );

    let by_collection_path = format!("/api/v0/share-grants/by-collection/{collection_id}");
    let by_collection = super::route_http_request("GET", &by_collection_path, None, "", &state)
        .await
        .expect("share-grants by-collection nominal response");
    let by_collection_value = parse_json(&by_collection.body);
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "nominal-status-headers-body",
        by_collection.status == "200 OK" && by_collection_value.is_array()
    );
    let by_collection_missing = super::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/by-collection/{missing_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("share-grants by-collection missing response");
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "runtime-failure-and-timeout",
        by_collection_missing.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "populated-dynamic-state",
        by_collection_value
            .as_array()
            .is_some_and(|records| { records.iter().any(|record| record["id"] == grant_id) })
    );

    // Backfill has a policy gate. An empty permitted collection exercises
    // the source controller's stable 200/no-op response without invoking
    // an external download service.
    let empty_collection_id = uuid::Uuid::new_v4().to_string();
    state
        .collections
        .write()
        .await
        .create_with_contract(
            empty_collection_id.clone(),
            "tester".to_owned(),
            "Empty Share Grant".to_owned(),
            String::new(),
            "ShareList".to_owned(),
        )
        .expect("empty share-grants collection fixture");
    let allowed_grant_id = uuid::Uuid::new_v4().to_string();
    state
        .share_grants
        .write()
        .await
        .create_with_contract(
            Some(allowed_grant_id.clone()),
            empty_collection_id.clone(),
            "download-recipient".to_owned(),
        )
        .expect("allowed share-grants fixture");
    state
        .share_grants
        .write()
        .await
        .update(&allowed_grant_id, "read,download".to_owned())
        .expect("enable share-grant download permission");
    let backfill_path = format!("/api/v0/share-grants/{allowed_grant_id}/backfill");
    let backfill = super::route_http_request("POST", &backfill_path, None, "", &state)
        .await
        .expect("share-grants backfill nominal response");
    let backfill_value = parse_json(&backfill.body);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "nominal-status-headers-body",
        backfill.status == "200 OK"
            && backfill_value["enqueued"] == 0
            && backfill_value["failed"] == 0
            && backfill_value["message"] == "No items to backfill"
    );
    let backfill_malformed =
        super::route_http_request("POST", &format!("{backfill_path}/extra"), None, "", &state)
            .await
            .expect("share-grants backfill malformed response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "malformed-path-query-or-body",
        backfill_malformed.status == "404 Not Found"
    );
    let backfill_missing = super::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{missing_id}/backfill"),
        None,
        "",
        &state,
    )
    .await
    .expect("share-grants backfill missing response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "missing-empty-or-conflict-state",
        backfill_missing.status == "404 Not Found"
    );
    let backfill_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("share-grants backfill runtime database");
    let (backfill_runtime_state, _receiver) = test_state_with_env_parts(
        target_env(),
        super::SearchStore::new(),
        Some(backfill_runtime_db.clone()),
    );
    backfill_runtime_db.close_for_test().await;
    let backfill_runtime = super::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{missing_id}/backfill"),
        None,
        "",
        &backfill_runtime_state,
    )
    .await
    .expect("share-grants backfill runtime response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "runtime-failure-and-timeout",
        backfill_runtime.status == "404 Not Found"
    );
    let item_count_before = state
        .collections
        .read()
        .await
        .get(&empty_collection_id)
        .map(|collection| collection.items.len())
        .unwrap_or(0);
    let backfill_again = super::route_http_request("POST", &backfill_path, None, "", &state)
        .await
        .expect("share-grants backfill mutation response");
    let item_count_after = state
        .collections
        .read()
        .await
        .get(&empty_collection_id)
        .map(|collection| collection.items.len())
        .unwrap_or(0);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "mutation-side-effects-and-readback",
        backfill_again.status == "200 OK" && item_count_before == 0 && item_count_after == 0
    );
    let (restarted_state, _receiver) = test_state_with_env(target_env());
    let backfill_restarted = super::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{allowed_grant_id}/backfill"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("share-grants backfill restart response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "restart-persistence-or-reset",
        backfill_restarted.status == "404 Not Found"
    );
    let (left, right) = tokio::join!(
        super::route_http_request("POST", &backfill_path, None, "", &state),
        super::route_http_request("POST", &backfill_path, None, "", &state)
    );
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "concurrency-and-idempotency",
        left.as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && right
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    // Token binding rejects malformed JSON on the versioned route. The
    // existing v0 compatibility response is retained for valid requests.
    let token_path = format!("/api/v0/share-grants/{grant_id}/token");
    let malformed_token = super::route_http_request("POST", &token_path, None, "not-json", &state)
        .await
        .expect("share-grants malformed token response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "malformed-path-query-or-body",
        malformed_token.status == "400 Bad Request"
    );
    let token = super::route_http_request("POST", &token_path, None, "{}", &state)
        .await
        .expect("share-grants token mutation response");
    let token_value = parse_json(&token.body);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "mutation-side-effects-and-readback",
        token.status == "201 Created"
            && token_value["created"] == true
            && token_value["token"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );
    let token_restarted = super::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{missing_id}/token"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("share-grants token restart response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "restart-persistence-or-reset",
        token_restarted.status == "404 Not Found"
    );
    let (token_left, token_right) = tokio::join!(
        super::route_http_request("POST", &token_path, None, "{}", &state),
        super::route_http_request("POST", &token_path, None, "{}", &state)
    );
    let token_left = token_left.expect("share-grants left token response");
    let token_right = token_right.expect("share-grants right token response");
    let token_left_value = parse_json(&token_left.body);
    let token_right_value = parse_json(&token_right.body);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "concurrency-and-idempotency",
        token_left.status == "201 Created"
            && token_right.status == "201 Created"
            && token_left_value["token"].as_str().is_some_and(|value| {
                token_right_value["token"]
                    .as_str()
                    .is_some_and(|other| value != other)
            })
    );

    // The frozen controller deliberately keeps this E2E-only endpoint
    // disabled unless SLSKDN_E2E_SHARE_ANNOUNCE=1. The test environment
    // does not enable that opt-in, so every v0 case is a stable 404.
    let announce_path = "/api/v0/share-grants/announce";
    let announce_nominal = super::route_http_request(
        "POST",
        announce_path,
        None,
        r#"{"shareGrantId":"00000000-0000-0000-0000-000000000001","collectionId":"00000000-0000-0000-0000-000000000002","recipientUserId":"recipient"}"#,
        &state,
    )
    .await
    .expect("share-grants announce nominal response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "nominal-status-headers-body",
        announce_nominal.status == "404 Not Found"
    );
    let announce_malformed = super::route_http_request(
        "POST",
        "/api/v0/share-grants/announce/extra",
        None,
        "{}",
        &state,
    )
    .await
    .expect("share-grants announce malformed response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "malformed-path-query-or-body",
        announce_malformed.status == "404 Not Found"
    );
    let announce_missing = super::route_http_request("POST", announce_path, None, "", &state)
        .await
        .expect("share-grants announce missing response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "missing-empty-or-conflict-state",
        announce_missing.status == "404 Not Found"
    );
    let announce_runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("share-grants announce runtime database");
    let (announce_runtime_state, _receiver) = test_state_with_env_parts(
        target_env(),
        super::SearchStore::new(),
        Some(announce_runtime_db.clone()),
    );
    announce_runtime_db.close_for_test().await;
    let announce_runtime =
        super::route_http_request("POST", announce_path, None, "{}", &announce_runtime_state)
            .await
            .expect("share-grants announce runtime response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "runtime-failure-and-timeout",
        announce_runtime.status == "404 Not Found"
    );
    let count_before = state.share_grants.read().await.records.len();
    let announce_mutation = super::route_http_request("POST", announce_path, None, "{}", &state)
        .await
        .expect("share-grants announce mutation response");
    let count_after = state.share_grants.read().await.records.len();
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "mutation-side-effects-and-readback",
        announce_mutation.status == "404 Not Found" && count_before == count_after
    );
    let (announce_restarted_state, _receiver) = test_state_with_env(target_env());
    let announce_restarted =
        super::route_http_request("POST", announce_path, None, "{}", &announce_restarted_state)
            .await
            .expect("share-grants announce restart response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "restart-persistence-or-reset",
        announce_restarted.status == "404 Not Found"
    );
    let (announce_left, announce_right) = tokio::join!(
        super::route_http_request("POST", announce_path, None, "{}", &state),
        super::route_http_request("POST", announce_path, None, "{}", &state)
    );
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "concurrency-and-idempotency",
        announce_left
            .as_ref()
            .is_ok_and(|response| response.status == "404 Not Found")
            && announce_right
                .as_ref()
                .is_ok_and(|response| response.status == "404 Not Found")
    );

    assert_eq!(ledger.len(), 27, "share-grants residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create share-grants evidence directory");
    fs::write(
        evidence_dir.join("share_grants_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize share-grants ledger"),
    )
    .expect("write share-grants ledger");
    assert!(
        mismatches.is_empty(),
        "{} share-grants controller mismatches:\n{}",
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
async fn controller_api_differential_shares_open_cases() {
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

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };
    let root_id = super::share_root_id("Virtual");
    macro_rules! seed_share_root {
        ($state:expr) => {{
            let mut shares = $state.shares.write().await;
            shares.roots.clear();
            shares.entries.clear();
            shares.local_paths.clear();
            shares.roots.push(super::ShareRoot {
                label: "Virtual".to_owned(),
                local_path: PathBuf::from("/srv/music"),
                raw: "Virtual".to_owned(),
                directories: 1,
                files: 1,
                bytes: 42,
                extensions: vec![super::ShareExtensionSummary {
                    extension: "flac".to_owned(),
                    files: 1,
                    bytes: 42,
                }],
                statistics_ready: true,
            });
            shares.entries.push(FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: "Virtual/Track.flac".to_owned(),
                size: 42,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            });
        }};
    }

    // CancelShareScan returns NoContent only while a scan is active.
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let _permit = Arc::clone(&state.share_scans)
            .acquire_owned()
            .await
            .expect("share scan permit");
        let response = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("share cancel nominal response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "nominal-status-headers-body",
            response.status == "204 No Content" && response.body.is_empty()
        );
    }
    let malformed_delete = {
        let (state, _receiver) = test_state_with_env(target_env());
        super::route_http_request("DELETE", "/api/v0/shares/extra", None, "", &state)
            .await
            .expect("share cancel malformed response")
    };
    record!(
        "DELETE",
        "/api/v0/shares",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share cancel runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("share cancel runtime response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "runtime-failure-and-timeout",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("share cancel restart response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let _permit = Arc::clone(&state.share_scans)
            .acquire_owned()
            .await
            .expect("share scan concurrency permit");
        let first = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("first share cancel concurrency response");
        let second = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("second share cancel concurrency response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "concurrency-and-idempotency",
            first.status == "204 No Content" && second.status == "404 Not Found"
        );
    }

    // List and detail use the live share index and remain readable when
    // the optional database is closed.
    {
        let (empty_state, _receiver) =
            test_state_with_env(target_env().with("SLSKR_SHARE_FIXTURE", ""));
        let empty = super::route_http_request("GET", "/api/v0/shares", None, "", &empty_state)
            .await
            .expect("empty shares list response");
        let empty_value = parse_json(&empty.body);
        record!(
            "GET",
            "/api/v0/shares",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty_value["local"].as_array().is_some_and(Vec::is_empty)
        );
    }
    let malformed_list = {
        let (state, _receiver) = test_state_with_env(target_env());
        super::route_http_request("GET", "/api/v0/shares/extra/path", None, "", &state)
            .await
            .expect("malformed shares list response")
    };
    record!(
        "GET",
        "/api/v0/shares",
        "malformed-path-query-or-body",
        malformed_list.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("shares list runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = super::route_http_request("GET", "/api/v0/shares", None, "", &state)
            .await
            .expect("shares list runtime response");
        let value = parse_json(&response.body);
        record!(
            "GET",
            "/api/v0/shares",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && value["local"].is_array()
        );
    }

    let (state, _receiver) = test_state_with_env(target_env());
    seed_share_root!(&state);
    let detail = super::route_http_request(
        "GET",
        &format!("/api/v0/shares/{root_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("share detail nominal response");
    let detail_value = parse_json(&detail.body);
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "nominal-status-headers-body",
        detail.status == "200 OK" && detail_value["id"] == root_id
    );
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "populated-dynamic-state",
        detail_value["files"] == 1
            && detail_value["directories"] == 1
            && detail_value["remotePath"] == "Virtual"
    );
    let malformed_detail =
        super::route_http_request("GET", "/api/v0/shares/extra/path", None, "", &state)
            .await
            .expect("share detail malformed response");
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "malformed-path-query-or-body",
        malformed_detail.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share detail runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        seed_share_root!(&runtime_state);
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            &format!("/api/v0/shares/{root_id}"),
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("share detail runtime response");
        record!(
            "GET",
            "/api/v0/shares/{id}",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["id"] == root_id
        );
    }

    let all_contents =
        super::route_http_request("GET", "/api/v0/shares/contents", None, "", &state)
            .await
            .expect("all share contents nominal response");
    let all_contents_value = parse_json(&all_contents.body);
    record!(
        "GET",
        "/api/v0/shares/contents",
        "runtime-failure-and-timeout",
        all_contents.status == "200 OK" && all_contents_value.is_array()
    );
    let malformed_all_contents =
        super::route_http_request("GET", "/api/v0/shares/contents/extra", None, "", &state)
            .await
            .expect("all share contents malformed response");
    record!(
        "GET",
        "/api/v0/shares/contents",
        "malformed-path-query-or-body",
        malformed_all_contents.status == "404 Not Found"
    );
    {
        let (empty_state, _receiver) =
            test_state_with_env(target_env().with("SLSKR_SHARE_FIXTURE", ""));
        let response =
            super::route_http_request("GET", "/api/v0/shares/contents", None, "", &empty_state)
                .await
                .expect("empty all share contents response");
        record!(
            "GET",
            "/api/v0/shares/contents",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && parse_json(&response.body).is_array()
        );
    }

    let share_contents_path = format!("/api/v0/shares/{root_id}/contents");
    let share_contents = super::route_http_request("GET", &share_contents_path, None, "", &state)
        .await
        .expect("share contents nominal response");
    let share_contents_value = parse_json(&share_contents.body);
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "nominal-status-headers-body",
        share_contents.status == "200 OK" && share_contents_value.is_array()
    );
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "populated-dynamic-state",
        share_contents_value.as_array().is_some_and(|directories| {
            directories.iter().any(|directory| {
                directory["files"]
                    .as_array()
                    .is_some_and(|files| !files.is_empty())
            })
        })
    );
    let malformed_share_contents = super::route_http_request(
        "GET",
        &format!("{share_contents_path}/extra"),
        None,
        "",
        &state,
    )
    .await
    .expect("share contents malformed response");
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "malformed-path-query-or-body",
        malformed_share_contents.status == "404 Not Found"
    );
    let missing_share_contents =
        super::route_http_request("GET", "/api/v0/shares/missing/contents", None, "", &state)
            .await
            .expect("missing share contents response");
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "missing-empty-or-conflict-state",
        missing_share_contents.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share contents runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        seed_share_root!(&runtime_state);
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            &format!("/api/v0/shares/{root_id}/contents"),
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("share contents runtime response");
        record!(
            "GET",
            "/api/v0/shares/{id}/contents",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).is_array()
        );
    }

    // PUT is the versioned rescan action. Its successful response is an
    // empty 200, while the asynchronous scan state becomes ready.
    let malformed_rescan =
        super::route_http_request("PUT", "/api/v0/shares/extra", None, "", &state)
            .await
            .expect("share rescan malformed response");
    record!(
        "PUT",
        "/api/v0/shares",
        "malformed-path-query-or-body",
        malformed_rescan.status == "404 Not Found"
    );
    {
        let (empty_state, _receiver) =
            test_state_with_env(target_env().with("SLSKR_SHARE_FIXTURE", ""));
        let response = super::route_http_request("PUT", "/api/v0/shares", None, "", &empty_state)
            .await
            .expect("empty share rescan response");
        record!(
            "PUT",
            "/api/v0/shares",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body.is_empty()
        );
    }
    {
        let response = super::route_http_request("PUT", "/api/v0/shares", None, "", &state)
            .await
            .expect("share rescan mutation response");
        record!(
            "PUT",
            "/api/v0/shares",
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && response.body.is_empty()
                && state.share_lifecycle.read().await.ready
        );
    }
    {
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let response =
            super::route_http_request("PUT", "/api/v0/shares", None, "", &restarted_state)
                .await
                .expect("share rescan restart response");
        record!(
            "PUT",
            "/api/v0/shares",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && response.body.is_empty()
                && restarted_state.share_lifecycle.read().await.ready
        );
    }

    assert_eq!(ledger.len(), 24, "shares residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create shares evidence directory");
    fs::write(
        evidence_dir.join("shares_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize shares ledger"),
    )
    .expect("write shares ledger");
    assert!(
        mismatches.is_empty(),
        "{} shares controller mismatches:\n{}",
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
async fn controller_api_differential_users_open_cases() {
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

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                "differential-peer=127.0.0.1:2234",
            )
    };
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };

    macro_rules! seed_user {
        ($state:expr, $username:expr) => {{
            $state.users.write().await.records.push(super::UserRecord {
                username: $username.to_owned(),
                watched: true,
                status: Some("Online".to_owned()),
                privileged: true,
                average_speed: Some(1_024),
                upload_count: Some(3),
                file_count: Some(12),
                directory_count: Some(4),
                updated_at: super::unix_timestamp(),
            });
        }};
    }
    macro_rules! seed_browse {
        ($state:expr, $username:expr) => {{
            $state
                .browse
                .write()
                .await
                .add_entries(
                    $username.to_owned(),
                    vec![super::BrowseEntry {
                        path_encoding: Default::default(),
                        filename: "Remote/Album/Track.flac".to_owned(),
                        size: 123,
                        extension: "flac".to_owned(),
                    }],
                    true,
                )
                .expect("user browse fixture");
        }};
    }

    // Browse projections: malformed paths reject, and a connected
    // slskdn user with no browse record returns the frozen 404.
    let (state, _receiver) = test_state_with_env(env());
    state.session.write().await.state = "connected";
    seed_user!(&state, "differential-peer");
    seed_browse!(&state, "differential-peer");
    let malformed_browse = super::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/browse/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user browse malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "malformed-path-query-or-body",
        malformed_browse.status == "404 Not Found"
    );
    let (missing_browse_state, _receiver) = test_state_with_env(env());
    missing_browse_state.session.write().await.state = "connected";
    let missing_browse = super::route_http_request(
        "GET",
        "/api/v0/users/missing-peer/browse",
        None,
        "",
        &missing_browse_state,
    )
    .await
    .expect("user browse empty response");
    let missing_browse_value = parse_json(&missing_browse.body);
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "missing-empty-or-conflict-state",
        missing_browse.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user browse runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        runtime_state.session.write().await.state = "connected";
        seed_browse!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/browse",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user browse runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/browse",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["directoryCount"] == 1
        );
    }

    // Browse status is a local tracker projection; database closure does
    // not erase it.
    let malformed_status = super::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/browse/status/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user browse status malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "malformed-path-query-or-body",
        malformed_status.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user browse status runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        seed_browse!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/browse/status",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user browse status runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/browse/status",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["status"] == "ready"
        );
    }

    // Endpoint lookup uses the deterministic test override for the
    // nominal/runtime rows; dropping the command receiver makes the
    // missing-user path fail without waiting on a network timeout.
    let malformed_endpoint = super::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/endpoint/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user endpoint malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "malformed-path-query-or-body",
        malformed_endpoint.status == "404 Not Found"
    );
    let (missing_endpoint_state, missing_endpoint_receiver) = test_state_with_env(env());
    drop(missing_endpoint_receiver);
    let missing_endpoint = super::route_http_request(
        "GET",
        "/api/v0/users/missing-peer/endpoint",
        None,
        "",
        &missing_endpoint_state,
    )
    .await
    .expect("user endpoint missing response");
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "missing-empty-or-conflict-state",
        missing_endpoint.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user endpoint runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/endpoint",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user endpoint runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/endpoint",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["port"] == 2234
        );
    }

    // Info and status expose the slskdn DTO projection for both watched
    // and untracked users.
    let info = super::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/info",
        None,
        "",
        &state,
    )
    .await
    .expect("user info nominal response");
    let info_value = parse_json(&info.body);
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "nominal-status-headers-body",
        info.status == "200 OK"
            && info_value["uploadSpeed"] == 1_024
            && info_value["uploadCount"] == 3
            && info_value["fileCount"] == 12
            && info_value["directoryCount"] == 4
    );
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "populated-dynamic-state",
        info_value["description"] == ""
            && info_value["hasFreeUploadSlot"] == true
            && info_value["picture"].is_null()
    );
    let malformed_info = super::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/info/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user info malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "malformed-path-query-or-body",
        malformed_info.status == "404 Not Found"
    );
    let missing_info =
        super::route_http_request("GET", "/api/v0/users/missing-peer/info", None, "", &state)
            .await
            .expect("user info missing response");
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "missing-empty-or-conflict-state",
        missing_info.status == "200 OK" && parse_json(&missing_info.body)["fileCount"] == 0
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user info runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        seed_user!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/info",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user info runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/info",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["uploadCount"] == 3
        );
    }

    let malformed_status = super::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/status/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user status malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "malformed-path-query-or-body",
        malformed_status.status == "404 Not Found"
    );
    let missing_status =
        super::route_http_request("GET", "/api/v0/users/missing-peer/status", None, "", &state)
            .await
            .expect("user status missing response");
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "missing-empty-or-conflict-state",
        missing_status.status == "200 OK"
            && parse_json(&missing_status.body)["presence"] == "Offline"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user status runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        seed_user!(&runtime_state, "differential-peer");
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/status",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user status runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/status",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["presence"] == "Online"
        );
    }

    // Groups are synchronous transfer-group projections, not database
    // lookups. Malformed subpaths are still rejected by routing.
    let malformed_group = super::route_http_request(
        "GET",
        "/api/v0/users/differential-peer/group/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("user group malformed response");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "malformed-path-query-or-body",
        malformed_group.status == "404 Not Found"
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user group runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/users/differential-peer/group",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user group runtime response");
        record!(
            "GET",
            "/api/v0/users/{username}/group",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).as_str() == Some("default")
        );
    }
    let empty_groups = super::route_http_request("GET", "/api/v0/users/groups", None, "", &state)
        .await
        .expect("empty user groups response");
    record!(
        "GET",
        "/api/v0/users/groups",
        "missing-empty-or-conflict-state",
        empty_groups.status == "200 OK" && parse_json(&empty_groups.body) == serde_json::json!({})
    );
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user groups runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response =
            super::route_http_request("GET", "/api/v0/users/groups", None, "", &runtime_state)
                .await
                .expect("user groups runtime response");
        record!(
            "GET",
            "/api/v0/users/groups",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body) == serde_json::json!({})
        );
    }

    // User notes are loaded into the bounded in-memory projection at
    // startup; closed persistence does not invalidate read projections.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user notes list runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response =
            super::route_http_request("GET", "/api/v0/users/notes", None, "", &runtime_state)
                .await
                .expect("user notes list runtime response");
        record!(
            "GET",
            "/api/v0/users/notes",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).is_array()
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user note runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));
        runtime_state.user_notes.write().await.set_versioned(
            "differential-peer".to_owned(),
            "runtime note".to_owned(),
            String::new(),
            String::new(),
            false,
        );
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/users/notes/differential-peer",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("user note runtime response");
        record!(
            "GET",
            "/api/v0/users/notes/{username}",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["note"] == "runtime note"
        );
    }

    // Directory request binding precedes connection readiness, matching
    // the frozen controller; valid requests then project the cached browse
    // directory and remain idempotent across fresh state instances.
    let malformed_directory = super::route_http_request(
        "POST",
        "/api/v0/users/differential-peer/directory/extra",
        None,
        "{}",
        &state,
    )
    .await
    .expect("user directory malformed response");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "malformed-path-query-or-body",
        malformed_directory.status == "404 Not Found"
    );
    let missing_directory = super::route_http_request(
        "POST",
        "/api/v0/users/differential-peer/directory",
        None,
        "{}",
        &state,
    )
    .await
    .expect("user directory missing response");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "missing-empty-or-conflict-state",
        missing_directory.status == "400 Bad Request"
            && missing_directory.body.contains("directory is required")
    );
    let directory = super::route_http_request(
        "POST",
        "/api/v0/users/differential-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &state,
    )
    .await
    .expect("user directory mutation response");
    let directory_value = parse_json(&directory.body);
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "mutation-side-effects-and-readback",
        directory.status == "200 OK"
            && directory_value
                .as_array()
                .is_some_and(|rows| { rows.iter().any(|row| row["name"] == "Remote/Album") })
    );
    {
        let (restarted_state, _receiver) = test_state_with_env(env());
        restarted_state.session.write().await.state = "connected";
        let response = super::route_http_request(
            "POST",
            "/api/v0/users/differential-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &restarted_state,
        )
        .await
        .expect("user directory restart response");
        let value = parse_json(&response.body);
        record!(
            "POST",
            "/api/v0/users/{username}/directory",
            "restart-persistence-or-reset",
            response.status == "200 OK" && value.as_array().is_some_and(|rows| rows.len() == 1)
        );
    }
    let concurrent = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/users/differential-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/users/differential-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    assert_eq!(ledger.len(), 27, "users residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create users evidence directory");
    fs::write(
        evidence_dir.join("users_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize users ledger"),
    )
    .expect("write users ledger");
    assert!(
        mismatches.is_empty(),
        "{} users controller mismatches:\n{}",
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
async fn controller_api_differential_telemetry_open_cases() {
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

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };

    let (malformed_state, _receiver) = test_state_with_env(env());
    for (path, route) in [
        (
            "/api/v0/telemetry/metrics/extra",
            "/api/v0/telemetry/metrics",
        ),
        (
            "/api/v0/telemetry/metrics/kpi/extra",
            "/api/v0/telemetry/metrics/kpi",
        ),
        (
            "/api/v0/telemetry/prometheus/extra",
            "/api/v0/telemetry/prometheus",
        ),
        (
            "/api/v0/telemetry/prometheus/kpis/extra",
            "/api/v0/telemetry/prometheus/kpis",
        ),
        (
            "/api/v0/telemetry/reports/transfers/directories/extra",
            "/api/v0/telemetry/reports/transfers/directories",
        ),
        (
            "/api/v0/telemetry/reports/transfers/histogram/extra",
            "/api/v0/telemetry/reports/transfers/histogram",
        ),
        (
            "/api/v0/telemetry/reports/transfers/summary/extra",
            "/api/v0/telemetry/reports/transfers/summary",
        ),
        (
            "/api/v0/telemetry/reports/transfers/users/",
            "/api/v0/telemetry/reports/transfers/users/{username}",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &malformed_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let (empty_state, _receiver) = test_state_with_env(env());
    let metrics =
        super::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &empty_state)
            .await
            .expect("empty telemetry metrics response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "missing-empty-or-conflict-state",
        metrics.status == "200 OK"
            && metrics.content_type.starts_with("text/plain")
            && metrics.body.contains("slskr_telemetry_transfers 0")
    );
    let metrics_kpi = super::route_http_request(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty telemetry KPI response");
    let metrics_kpi_json = parse_json(&metrics_kpi.body);
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        "missing-empty-or-conflict-state",
        metrics_kpi.status == "200 OK"
            && metrics_kpi_json["slskr_transfers"]["samples"][0]["value"] == 0.0
    );
    let prometheus = super::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty telemetry Prometheus response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus",
        "missing-empty-or-conflict-state",
        prometheus.status == "200 OK"
            && prometheus.content_type.starts_with("text/plain")
            && prometheus.body.contains("slskr_transfers 0")
    );
    let prometheus_kpis = super::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty telemetry Prometheus KPI response");
    let prometheus_kpis_json = parse_json(&prometheus_kpis.body);
    record!(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        "missing-empty-or-conflict-state",
        prometheus_kpis.status == "200 OK"
            && prometheus_kpis_json["slskr_transfers"]["samples"][0]["value"] == 0.0
    );
    let directories = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer directories response");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "missing-empty-or-conflict-state",
        directories.status == "200 OK" && parse_json(&directories.body).is_array()
    );
    let histogram = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer histogram response");
    let histogram_json = parse_json(&histogram.body);
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram",
        "missing-empty-or-conflict-state",
        histogram.status == "200 OK"
            && histogram_json["2100-01-01T00:00:00Z"]["Download"].is_object()
    );
    let summary = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer summary response");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        "missing-empty-or-conflict-state",
        summary.status == "200 OK"
            && parse_json(&summary.body) == serde_json::json!({"Download": {}, "Upload": {}})
    );
    let user = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/unknown",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty transfer user response");
    let user_json = parse_json(&user.body);
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "missing-empty-or-conflict-state",
        user.status == "200 OK" && user_json["username"] == "unknown" && user_json["count"] == 0
    );

    let (populated_state, _receiver) = test_state_with_env(env());
    {
        let mut transfers = populated_state.transfers.write().await;
        transfers.create(
            0,
            Some("telemetry-open-peer".to_owned()),
            "Remote/Open.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.create(
            1,
            Some("telemetry-open-peer".to_owned()),
            "Shared/Open.flac".to_owned(),
            None,
            Some(20),
        );
    }
    let populated_metrics = super::route_http_request(
        "GET",
        "/api/v0/telemetry/metrics",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry metrics response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "populated-dynamic-state",
        populated_metrics.status == "200 OK"
            && populated_metrics
                .body
                .contains("slskr_telemetry_transfers 2")
    );
    let populated_metrics_kpi = super::route_http_request(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry KPI response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        "populated-dynamic-state",
        parse_json(&populated_metrics_kpi.body)["slskr_transfers"]["samples"][0]["value"] == 2.0
    );
    let populated_prometheus = super::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry Prometheus response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus",
        "populated-dynamic-state",
        populated_prometheus.status == "200 OK"
            && populated_prometheus.body.contains("slskr_transfers 2")
    );
    let populated_prometheus_kpis = super::route_http_request(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated telemetry Prometheus KPI response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        "populated-dynamic-state",
        parse_json(&populated_prometheus_kpis.body)["slskr_transfers"]["samples"][0]["value"]
            == 2.0
    );

    let runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("telemetry open cases runtime database");
    let (runtime_state, _receiver) = test_state_with_env_parts(
        env().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    for (path, route, case) in [
        (
            "/api/v0/telemetry/metrics",
            "/api/v0/telemetry/metrics",
            "metrics",
        ),
        (
            "/api/v0/telemetry/prometheus",
            "/api/v0/telemetry/prometheus",
            "prometheus",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && if case == "metrics" {
                    response.body.contains("slskr_telemetry_transfers")
                } else {
                    response.body.contains("slskr_transfers")
                }
        );
    }
    for (path, route) in [
        (
            "/api/v0/telemetry/metrics/kpi",
            "/api/v0/telemetry/metrics/kpi",
        ),
        (
            "/api/v0/telemetry/prometheus/kpis",
            "/api/v0/telemetry/prometheus/kpis",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).is_object()
        );
    }
    for (path, route) in [
        (
            "/api/v0/telemetry/reports/transfers/summary",
            "/api/v0/telemetry/reports/transfers/summary",
        ),
        (
            "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
            "/api/v0/telemetry/reports/transfers/histogram",
        ),
        (
            "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download",
            "/api/v0/telemetry/reports/transfers/leaderboard",
        ),
        (
            "/api/v0/telemetry/reports/transfers/users/runtime-peer",
            "/api/v0/telemetry/reports/transfers/users/{username}",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions?direction=Download",
            "/api/v0/telemetry/reports/transfers/exceptions",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download",
            "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        ),
        (
            "/api/v0/telemetry/reports/transfers/directories",
            "/api/v0/telemetry/reports/transfers/directories",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    assert_eq!(ledger.len(), 31, "telemetry residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create telemetry evidence directory");
    fs::write(
        evidence_dir.join("telemetry_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize telemetry ledger"),
    )
    .expect("write telemetry ledger");
    assert!(
        mismatches.is_empty(),
        "{} telemetry controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
fn controller_api_differential_relay_open_cases() {
    run_controller_future_on_large_stack("relay-open-cases", || {
        controller_api_differential_relay_open_cases_impl()
    });
}

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_relay_open_cases_impl() {
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

    let agent_env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKD_RELAY", "true")
            .with("SLSKD_RELAY_MODE", "agent")
            .with("SLSKD_CONTROLLER_ADDRESS", "http://127.0.0.1:9")
            .with("SLSKD_CONTROLLER_API_KEY", "relay-api-key-123456")
            .with("SLSKD_CONTROLLER_SECRET", "relay-secret-123456")
    };

    async fn configured_controller() -> (Arc<super::AppState>, String, u64) {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
                .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", "127.0.0.1/32"),
        );
        let secret = "test-token-0123456789".to_owned();
        {
            let mut advanced = state.advanced_networking.write().await;
            advanced.relay.enabled = true;
            advanced.relay.mode = "controller".to_owned();
            advanced.relay.agents.insert(
                "edge".to_owned(),
                super::config::RelayAgentSettings {
                    instance_name: "edge-one".to_owned(),
                    secret: secret.clone(),
                    cidr: "127.0.0.1/32".to_owned(),
                },
            );
        }
        let now = super::unix_timestamp();
        let challenge = state
            .relay
            .write()
            .await
            .protocol
            .issue_challenge("slskdn-relay-connection", now);
        let credential = super::relay::credential_for_target(
            super::ControllerProfile::Native,
            &secret,
            "edge-one",
            &challenge,
        );
        let settings = state.advanced_networking.read().await.relay.clone();
        assert!(state.relay.write().await.protocol.authenticate_agent(
            &settings,
            super::relay::credential_scheme(super::ControllerProfile::Native,),
            "slskdn-relay-connection",
            "edge-one",
            &credential,
            "127.0.0.1".parse().expect("relay test address"),
            now,
        ));
        (state, secret, now)
    }

    async fn live_get(state: Arc<super::AppState>, path: &str) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(super::handle_http_stream(
            server,
            Some("127.0.0.1:1".parse().expect("relay stream remote address")),
            false,
            state,
        ));
        client
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .expect("write relay stream request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read relay stream response");
        task.await
            .expect("relay stream HTTP task")
            .expect("relay stream HTTP response");
        response
    }

    let (agent_state, _receiver) = test_state_with_env(agent_env());
    let malformed_put =
        super::route_http_request("PUT", "/api/v0/relay/agent/extra", None, "", &agent_state)
            .await
            .expect("malformed relay PUT");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "malformed-path-query-or-body",
        malformed_put.status == "404 Not Found"
    );
    let started = super::route_http_request("PUT", "/api/v0/relay/agent", None, "", &agent_state)
        .await
        .expect("relay agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "nominal-status-headers-body",
        started.status == "200 OK" && started.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "mutation-side-effects-and-readback",
        agent_state.runtime.read().await.relay_agent_enabled
    );
    let (restarted_agent_state, _receiver) = test_state_with_env(agent_env());
    let restarted = super::route_http_request(
        "PUT",
        "/api/v0/relay/agent",
        None,
        "",
        &restarted_agent_state,
    )
    .await
    .expect("restarted relay agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "restart-persistence-or-reset",
        restarted.status == "200 OK"
            && restarted_agent_state
                .runtime
                .read()
                .await
                .relay_agent_enabled
    );
    let concurrent_put = futures_util::future::join_all([
        super::route_http_request("PUT", "/api/v0/relay/agent", None, "", &agent_state),
        super::route_http_request("PUT", "/api/v0/relay/agent", None, "", &agent_state),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "concurrency-and-idempotency",
        concurrent_put.iter().all(|response| response
            .as_ref()
            .is_ok_and(|value| value.status == "200 OK"))
    );

    let malformed_delete = super::route_http_request(
        "DELETE",
        "/api/v0/relay/agent/extra",
        None,
        "",
        &agent_state,
    )
    .await
    .expect("malformed relay DELETE");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );
    let stopped =
        super::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &agent_state)
            .await
            .expect("relay agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "nominal-status-headers-body",
        stopped.status == "204 No Content" && stopped.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "mutation-side-effects-and-readback",
        !agent_state.runtime.read().await.relay_agent_enabled
    );
    let concurrent_delete = futures_util::future::join_all([
        super::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &agent_state),
        super::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &agent_state),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "concurrency-and-idempotency",
        concurrent_delete.iter().all(|response| response
            .as_ref()
            .is_ok_and(|value| value.status == "204 No Content"))
    );
    let (restarted_delete_state, _receiver) = test_state_with_env(agent_env());
    let restarted_delete = super::route_http_request(
        "DELETE",
        "/api/v0/relay/agent",
        None,
        "",
        &restarted_delete_state,
    )
    .await
    .expect("restarted relay agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "restart-persistence-or-reset",
        restarted_delete.status == "204 No Content"
    );

    let (controller_state, secret, now) = configured_controller().await;
    let download_token = controller_state
        .relay
        .write()
        .await
        .protocol
        .issue_download_tokens("Relay/Open.txt", now)
        .into_iter()
        .next()
        .expect("relay download token")
        .1;
    let download_credential = super::relay::credential_for_target(
        super::ControllerProfile::Native,
        &secret,
        "edge-one",
        &download_token,
    );
    let download_headers = super::RequestSecurityHeaders {
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(download_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..super::RequestSecurityHeaders::default()
    };
    let downloads_root = super::effective_downloads_dir(&controller_state);
    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download directory");
    fs::write(downloads_root.join("Relay/Open.txt"), b"relay-open-payload")
        .expect("relay download fixture");
    let download = Box::pin(super::route_http_request_with_headers(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &controller_state,
        download_headers.clone(),
    ))
    .await
    .expect("relay controller download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "nominal-status-headers-body",
        download.status == "200 OK" && download.content_type == "application/octet-stream"
    );
    let mut download_stream = super::open_relay_controller_download(
        &controller_state,
        &download_token,
        &download_headers,
    )
    .await
    .expect("open relay controller download");
    let mut download_payload = Vec::new();
    std::io::Read::read_to_end(&mut download_stream.file, &mut download_payload)
        .expect("read relay controller download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "populated-dynamic-state",
        download_payload == b"relay-open-payload"
    );
    let malformed_download = Box::pin(super::route_http_request_with_headers(
        "GET",
        "/api/v0/relay/controller/downloads/not-a-guid",
        None,
        "",
        &controller_state,
        download_headers.clone(),
    ))
    .await
    .expect("malformed relay controller download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "malformed-path-query-or-body",
        malformed_download.status == "400 Bad Request"
    );
    let runtime_download = Box::pin(super::route_http_request_with_headers(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &controller_state,
        super::RequestSecurityHeaders {
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some("invalid-relay-credential".to_owned()),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..super::RequestSecurityHeaders::default()
        },
    ))
    .await
    .expect("runtime relay controller download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "runtime-failure-and-timeout",
        runtime_download.status == "401 Unauthorized"
    );

    let (upload_token, upload_receiver) = controller_state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Upload.flac", 0, now)
        .expect("relay upload stream");
    let upload_credential = super::relay::credential_for_target(
        super::ControllerProfile::Native,
        &secret,
        "edge-one",
        &upload_token.to_string(),
    );
    let upload_body = "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Upload.flac\"\r\n\r\nrelay-upload-payload\r\n--relay--\r\n";
    let upload_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(upload_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..super::RequestSecurityHeaders::default()
    };
    let upload = Box::pin(super::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &controller_state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay file upload");
    let uploaded = upload_receiver
        .await
        .expect("relay upload receiver")
        .expect("relay upload result");
    assert_eq!(uploaded.filename, "Upload.flac");
    let stored_upload = controller_state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!("file-{}.part", upload_token.simple()));
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "nominal-status-headers-body",
        upload.status == "200 OK" && upload.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "mutation-side-effects-and-readback",
        fs::read(&stored_upload).is_ok_and(|data| data == b"relay-upload-payload")
    );
    let malformed_upload = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/api/v0/relay/controller/files/not-a-guid",
        None,
        upload_body,
        &controller_state,
        upload_headers.clone(),
    ))
    .await
    .expect("malformed relay file upload");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "malformed-path-query-or-body",
        malformed_upload.status == "400 Bad Request"
    );
    let (runtime_upload_token, runtime_upload_receiver) = controller_state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Runtime.flac", 0, now)
        .expect("runtime relay upload stream");
    let runtime_upload = super::versioned_relay_request(
        "POST",
        &format!("/api/v0/relay/controller/files/{runtime_upload_token}"),
        upload_body,
        &super::RequestSecurityHeaders {
            content_type: Some("multipart/form-data; boundary=relay".to_owned()),
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some("invalid-relay-credential".to_owned()),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..super::RequestSecurityHeaders::default()
        },
        &controller_state,
    )
    .await
    .expect("runtime relay file upload");
    assert!(runtime_upload_receiver
        .await
        .expect("runtime relay upload receiver")
        .is_err());
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "runtime-failure-and-timeout",
        runtime_upload.status == "401 Unauthorized"
    );
    let (restarted_controller, _secret, _now) = configured_controller().await;
    let restarted_upload = super::versioned_relay_request(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        upload_body,
        &upload_headers,
        &restarted_controller,
    )
    .await
    .expect("restarted relay file upload");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "restart-persistence-or-reset",
        restarted_upload.status == "401 Unauthorized"
    );
    let (concurrent_upload_token, concurrent_upload_receiver) = controller_state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Concurrent.flac", 0, now)
        .expect("concurrent relay upload stream");
    let concurrent_upload_credential = super::relay::credential_for_target(
        super::ControllerProfile::Native,
        &secret,
        "edge-one",
        &concurrent_upload_token.to_string(),
    );
    let concurrent_upload_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(concurrent_upload_credential),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..super::RequestSecurityHeaders::default()
    };
    let concurrent_upload_body = "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Concurrent.flac\"\r\n\r\nrelay-upload-payload\r\n--relay--\r\n";
    let concurrent_uploads = futures_util::future::join_all([
        super::versioned_relay_request(
            "POST",
            &format!("/api/v0/relay/controller/files/{concurrent_upload_token}"),
            concurrent_upload_body,
            &concurrent_upload_headers,
            &controller_state,
        ),
        super::versioned_relay_request(
            "POST",
            &format!("/api/v0/relay/controller/files/{concurrent_upload_token}"),
            concurrent_upload_body,
            &concurrent_upload_headers,
            &controller_state,
        ),
    ])
    .await;
    let concurrent_upload_statuses = concurrent_uploads
        .iter()
        .filter_map(|response| response.as_ref().map(|value| value.status))
        .collect::<Vec<_>>();
    assert!(concurrent_upload_receiver
        .await
        .expect("concurrent relay upload receiver")
        .is_ok());
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "concurrency-and-idempotency",
        concurrent_upload_statuses.contains(&"200 OK")
            && concurrent_upload_statuses.contains(&"401 Unauthorized")
    );

    let share_token = controller_state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("relay share token");
    let share_credential = super::relay::credential_for_target(
        super::ControllerProfile::Native,
        &secret,
        "edge-one",
        &share_token,
    );
    let share_source = controller_state
        .config
        .state_dir
        .join("relay-share-source.db");
    super::relay::write_share_database(
        &share_source,
        super::ControllerProfile::Native,
        &[super::relay::RemoteShare {
            filename: "Remote/Agent.flac".to_owned(),
            size: 6,
        }],
    )
    .await
    .expect("relay slskdn share database");
    let database_bytes = fs::read(&share_source).expect("read relay slskdn share database");
    let mut share_body = Vec::new();
    share_body.extend_from_slice(
        b"--relay\r\nContent-Disposition: form-data; name=\"shares\"\r\n\r\n[]\r\n--relay\r\nContent-Disposition: form-data; name=\"database\"; filename=\"shares.db\"\r\n\r\n",
    );
    share_body.extend_from_slice(&database_bytes);
    share_body.extend_from_slice(b"\r\n--relay--\r\n");
    let share_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(share_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..super::RequestSecurityHeaders::default()
    };
    let share_upload = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &controller_state,
    )
    .await
    .expect("relay share upload");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "nominal-status-headers-body",
        share_upload.status == "200 OK" && share_upload.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "mutation-side-effects-and-readback",
        controller_state
            .relay
            .read()
            .await
            .protocol
            .remote_file_for_agent("edge-one", "Remote/Agent.flac")
            == Some(("Remote/Agent.flac".to_owned(), 6))
    );
    let malformed_share = super::route_http_request_with_headers(
        "POST",
        "/api/v0/relay/controller/shares/not-a-guid",
        None,
        "",
        &controller_state,
        share_headers.clone(),
    )
    .await
    .expect("malformed relay share upload");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "malformed-path-query-or-body",
        malformed_share.status == "400 Bad Request"
    );
    let runtime_share_token = controller_state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("runtime relay share token");
    let runtime_share = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{runtime_share_token}"),
        &share_body,
        &super::RequestSecurityHeaders {
            content_type: Some("multipart/form-data; boundary=relay".to_owned()),
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some("invalid-relay-credential".to_owned()),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..super::RequestSecurityHeaders::default()
        },
        &controller_state,
    )
    .await
    .expect("runtime relay share upload");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "runtime-failure-and-timeout",
        runtime_share.status == "401 Unauthorized"
    );
    let (restarted_share_state, _secret, _now) = configured_controller().await;
    let restarted_share = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &restarted_share_state,
    )
    .await
    .expect("restarted relay share upload");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "restart-persistence-or-reset",
        restarted_share.status == "401 Unauthorized"
    );
    let concurrent_share_token = restarted_share_state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", super::unix_timestamp())
        .expect("concurrent relay share token");
    let concurrent_share_credential = super::relay::credential_for_target(
        super::ControllerProfile::Native,
        &secret,
        "edge-one",
        &concurrent_share_token,
    );
    let concurrent_share_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(concurrent_share_credential),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..super::RequestSecurityHeaders::default()
    };
    let concurrent_shares = futures_util::future::join_all([
        super::versioned_relay_request_bytes(
            "POST",
            &format!("/api/v0/relay/controller/shares/{concurrent_share_token}"),
            &share_body,
            &concurrent_share_headers,
            &restarted_share_state,
        ),
        super::versioned_relay_request_bytes(
            "POST",
            &format!("/api/v0/relay/controller/shares/{concurrent_share_token}"),
            &share_body,
            &concurrent_share_headers,
            &restarted_share_state,
        ),
    ])
    .await;
    let concurrent_share_statuses = concurrent_shares
        .iter()
        .filter_map(|response| response.as_ref().map(|value| value.status))
        .collect::<Vec<_>>();
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "concurrency-and-idempotency",
        concurrent_share_statuses.contains(&"200 OK")
            && concurrent_share_statuses.contains(&"401 Unauthorized")
    );

    let stream_route = "/api/v0/relay/streams/unknown/extra";
    let malformed_stream =
        super::route_http_request("GET", stream_route, None, "", &controller_state)
            .await
            .expect("malformed relay stream");
    record!(
        "GET",
        "/api/v0/relay/streams/{contentId}",
        "malformed-path-query-or-body",
        malformed_stream.status == "404 Not Found"
    );
    {
        let mut features = controller_state.media_services.write().await;
        features.features.streaming_relay_fallback = true;
    }
    let missing_stream = live_get(
        Arc::clone(&controller_state),
        "/api/v0/relay/streams/missing-content?agentName=edge-one",
    )
    .await;
    record!(
        "GET",
        "/api/v0/relay/streams/{contentId}",
        "missing-empty-or-conflict-state",
        String::from_utf8_lossy(&missing_stream).starts_with("HTTP/1.1 404 Not Found")
    );
    let content_id = super::stable_content_hash("Remote/Agent.flac", 6).to_string();
    let runtime_stream = live_get(
        Arc::clone(&controller_state),
        &format!("/api/v0/relay/streams/{content_id}?agentName=edge-one"),
    )
    .await;
    record!(
        "GET",
        "/api/v0/relay/streams/{contentId}",
        "runtime-failure-and-timeout",
        String::from_utf8_lossy(&runtime_stream).starts_with("HTTP/1.1 500 Internal Server Error")
    );

    let (hub_sender, mut hub_receiver) =
        tokio::sync::mpsc::channel(super::relay::HUB_OUTBOUND_QUEUE_CAPACITY);
    super::relay::register_hub_connection("slskdn-relay-connection".to_owned(), hub_sender);
    let stream_state = Arc::clone(&controller_state);
    let stream_path = format!("/api/v0/relay/streams/{content_id}?agentName=edge-one");
    let stream_task = tokio::spawn(async move { live_get(stream_state, &stream_path).await });
    let info_invocation = hub_receiver
        .recv()
        .await
        .expect("relay stream info invocation");
    let info_json = serde_json::from_str::<serde_json::Value>(&info_invocation)
        .expect("relay stream info JSON");
    let info_token = info_json["arguments"][1]
        .as_str()
        .expect("relay stream info token")
        .parse::<uuid::Uuid>()
        .expect("relay stream info UUID");
    assert!(controller_state
        .relay
        .write()
        .await
        .protocol
        .complete_file_info("slskdn-relay-connection", info_token, true, 6,));
    let upload_invocation = hub_receiver
        .recv()
        .await
        .expect("relay stream upload invocation");
    let upload_json = serde_json::from_str::<serde_json::Value>(&upload_invocation)
        .expect("relay stream upload JSON");
    let stream_token = upload_json["arguments"][2]
        .as_str()
        .expect("relay stream upload token")
        .to_owned();
    let stream_credential = super::relay::credential_for_target(
        super::ControllerProfile::Native,
        &secret,
        "edge-one",
        &stream_token,
    );
    let stream_upload = super::versioned_relay_request(
        "POST",
        &format!("/api/v0/relay/controller/files/{stream_token}"),
        "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Remote/Agent.flac\"\r\n\r\nstream\r\n--relay--\r\n",
        &super::RequestSecurityHeaders {
            content_type: Some("multipart/form-data; boundary=relay".to_owned()),
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some(stream_credential),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..super::RequestSecurityHeaders::default()
        },
        &controller_state,
    )
    .await
    .expect("relay stream upload");
    let stream_response = stream_task.await.expect("relay stream task");
    super::relay::unregister_hub_connection("slskdn-relay-connection");
    record!(
        "GET",
        "/api/v0/relay/streams/{contentId}",
        "nominal-status-headers-body",
        stream_upload.status == "200 OK"
            && String::from_utf8_lossy(&stream_response).starts_with("HTTP/1.1 200 OK")
    );
    record!(
        "GET",
        "/api/v0/relay/streams/{contentId}",
        "populated-dynamic-state",
        stream_response.ends_with(b"stream")
    );

    let _ = fs::remove_dir_all(super::effective_downloads_dir(&controller_state));
    let _ = fs::remove_file(share_source);
    assert_eq!(ledger.len(), 31, "relay residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create relay evidence directory");
    fs::write(
        evidence_dir.join("relay_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize relay ledger"),
    )
    .expect("write relay ledger");
    assert!(
        mismatches.is_empty(),
        "{} relay controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

async fn conversation_request(
    state: &Arc<super::AppState>,
    method: &str,
    path: &str,
    body: &str,
) -> super::routing::HttpResponse {
    super::route_http_request(method, path, None, body, state.as_ref())
        .await
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"))
}

async fn conversation_connect(state: &Arc<super::AppState>) {
    state.session.write().await.state = "connected";
}

async fn conversation_add(state: &Arc<super::AppState>, username: &str, body: &str) -> u64 {
    state
        .messages
        .write()
        .await
        .add(username.to_owned(), "inbound", body.to_owned())
        .id
}

async fn conversation_runtime_state(env: MapEnv, username: Option<&str>) -> Arc<super::AppState> {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation runtime database");
    let (state, _receiver) = test_state_with_env_parts(
        env.with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    if let Some(username) = username {
        conversation_add(&state, username, "runtime").await;
    }
    db.close_for_test().await;
    state
}

/// Differential proof for the residual slskdn ConversationsController
/// cases. The frozen controller trims required route values, validates
/// model-bound ids and booleans before service work, returns empty 200/201
/// bodies for mutations, and evaluates its durable conversation store
/// before producing read results.
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
async fn controller_api_differential_conversations_open_cases() {
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
    let json_value = |response: &super::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };

    // DELETE /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "delete-peer", "close me").await;
        let response =
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", "").await;
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "nominal-status-headers-body",
            response.status == "204 No Content" && response.body.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "DELETE",
            "/api/v0/conversations/delete-peer/extra",
            "",
        )
        .await;
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "delete-peer", "close me").await;
        let response =
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", "").await;
        let remaining = state
            .messages
            .read()
            .await
            .records
            .iter()
            .any(|record| record.username == "delete-peer");
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "mutation-side-effects-and-readback",
            response.status == "204 No Content" && !remaining
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response =
            conversation_request(&state, "DELETE", "/api/v0/conversations/restarted-peer", "")
                .await;
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "delete-peer", "close me").await;
        let responses = futures_util::future::join_all([
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", ""),
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", ""),
        ])
        .await;
        let statuses = responses
            .iter()
            .map(|response| response.status)
            .collect::<Vec<_>>();
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "concurrency-and-idempotency",
            statuses.contains(&"204 No Content") && statuses.contains(&"404 Not Found")
        );
    }

    // GET /conversations
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations?includeInactive=maybe",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let empty = conversation_request(&state, "GET", "/api/v0/conversations", "").await;
        record!(
            "GET",
            "/api/v0/conversations",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && json_value(&empty).as_array().is_some_and(Vec::is_empty)
        );
    }
    {
        let state = conversation_runtime_state(target_env(), None).await;
        let response = conversation_request(&state, "GET", "/api/v0/conversations", "").await;
        record!(
            "GET",
            "/api/v0/conversations",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // GET /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "conversation-peer", "hello").await;
        let nominal =
            conversation_request(&state, "GET", "/api/v0/conversations/conversation-peer", "")
                .await;
        let value = json_value(&nominal);
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && value["username"] == "conversation-peer"
                && value["messages"].is_array()
        );
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/conversation-peer?since=-1",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let state = conversation_runtime_state(target_env(), Some("conversation-peer")).await;
        let response =
            conversation_request(&state, "GET", "/api/v0/conversations/conversation-peer", "")
                .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // GET /conversations/{username}/messages
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "messages-peer", "one").await;
        let nominal = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && json_value(&nominal).is_array()
        );
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages?unAcknowledgedOnly=maybe",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let acknowledged_id = conversation_add(&state, "messages-peer", "two").await;
        state.messages.write().await.ack(acknowledged_id);
        let populated = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages?unAcknowledgedOnly=true",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && json_value(&populated)
                    .as_array()
                    .is_some_and(|messages| messages.len() == 1)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/missing-messages/messages",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let state = conversation_runtime_state(target_env(), Some("messages-peer")).await;
        let response = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // GET /conversations/activity/unacknowledged
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/activity/unacknowledged/extra",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        let missing = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && json_value(&missing) == serde_json::json!(false)
        );
    }
    {
        let state = conversation_runtime_state(target_env(), None).await;
        let response = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // POST /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/post-peer",
            r#"{"message":""}"#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing =
            conversation_request(&state, "POST", "/api/v0/conversations/post-peer", "").await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/restarted-post-peer",
            r#""restart""#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "restart-persistence-or-reset",
            response.status == "201 Created" && response.body.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let responses = futures_util::future::join_all([
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/concurrent-post-peer",
                r#""one""#,
            ),
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/concurrent-post-peer",
                r#""two""#,
            ),
        ])
        .await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "201 Created" && response.body.is_empty() })
        );
    }

    // POST /conversations/batch
    {
        let (state, mut receiver) = test_state_with_env(target_env());
        let nominal = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/batch",
            r#"{"usernames":["batch-a","batch-b"],"message":"hello"}"#,
        )
        .await;
        let command = receiver.try_recv().ok();
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "nominal-status-headers-body",
            nominal.status == "201 Created"
                && nominal.body.is_empty()
                && matches!(command, Some(super::SessionCommand::MessageUsers { .. }))
        );
        let malformed = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/batch",
            r#"{"usernames":"not-an-array","message":"hello"}"#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing =
            conversation_request(&state, "POST", "/api/v0/conversations/batch", "{}").await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/batch",
            r#"{"usernames":["restart-batch"],"message":"hello"}"#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "restart-persistence-or-reset",
            response.status == "201 Created" && response.body.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let responses = futures_util::future::join_all([
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/batch",
                r#"{"usernames":["concurrent-batch"],"message":"one"}"#,
            ),
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/batch",
                r#"{"usernames":["concurrent-batch"],"message":"two"}"#,
            ),
        ])
        .await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "201 Created" && response.body.is_empty() })
        );
    }

    // PUT /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "ack-all-peer", "ack me").await;
        conversation_connect(&state).await;
        let nominal =
            conversation_request(&state, "PUT", "/api/v0/conversations/ack-all-peer", "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && nominal.body.is_empty()
        );
        let malformed = conversation_request(&state, "PUT", "/api/v0/conversations/%20", "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/missing-ack-all", "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "mutation-ack-all", "ack me").await;
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/mutation-ack-all", "").await;
        let acknowledged = state
            .messages
            .read()
            .await
            .records
            .iter()
            .find(|record| record.id == id)
            .is_some_and(|record| record.acknowledged);
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && acknowledged
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/restarted-ack-all", "")
                .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "concurrent-ack-all", "ack me").await;
        conversation_connect(&state).await;
        let responses = futures_util::future::join_all([
            conversation_request(
                &state,
                "PUT",
                "/api/v0/conversations/concurrent-ack-all",
                "",
            ),
            conversation_request(
                &state,
                "PUT",
                "/api/v0/conversations/concurrent-ack-all",
                "",
            ),
        ])
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "200 OK" && response.body.is_empty() })
        );
    }

    // PUT /conversations/{username}/{id}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "ack-one-peer", "ack me").await;
        conversation_connect(&state).await;
        let path = format!("/api/v0/conversations/ack-one-peer/{id}");
        let nominal = conversation_request(&state, "PUT", &path, "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && nominal.body.is_empty()
        );
        let malformed = conversation_request(
            &state,
            "PUT",
            "/api/v0/conversations/ack-one-peer/not-an-int",
            "",
        )
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/missing-ack-one/1", "")
                .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "mutation-ack-one", "ack me").await;
        conversation_connect(&state).await;
        let path = format!("/api/v0/conversations/mutation-ack-one/{id}");
        let response = conversation_request(&state, "PUT", &path, "").await;
        let acknowledged = state
            .messages
            .read()
            .await
            .records
            .iter()
            .find(|record| record.id == id)
            .is_some_and(|record| record.acknowledged);
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && acknowledged
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response = conversation_request(
            &state,
            "PUT",
            "/api/v0/conversations/restarted-ack-one/1",
            "",
        )
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "concurrent-ack-one", "ack me").await;
        conversation_connect(&state).await;
        let path = format!("/api/v0/conversations/concurrent-ack-one/{id}");
        let responses = futures_util::future::join_all([
            conversation_request(&state, "PUT", &path, ""),
            conversation_request(&state, "PUT", &path, ""),
        ])
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "200 OK" && response.body.is_empty() })
        );
    }

    assert_eq!(ledger.len(), 40, "conversations residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create conversations evidence directory");
    fs::write(
        evidence_dir.join("conversations_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize conversations ledger"),
    )
    .expect("write conversations ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api conversations mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

async fn download_request_seed(state: &Arc<super::AppState>) -> String {
    let mut transfers = state.transfers.write().await;
    transfers
        .create(
            0,
            Some("download-peer".to_owned()),
            "Remote/Download.flac".to_owned(),
            None,
            Some(100),
        )
        .request_id
        .expect("download request id")
}

async fn download_request_add_attempt(state: &Arc<super::AppState>, request_id: &str) -> u64 {
    let mut transfers = state.transfers.write().await;
    let entry = transfers.create(
        0,
        Some("download-peer".to_owned()),
        "Remote/Download.flac".to_owned(),
        None,
        Some(100),
    );
    let id = entry.id;
    transfers
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .expect("download attempt")
        .request_id = Some(request_id.to_owned());
    id
}

async fn download_runtime_state(env: MapEnv) -> Arc<super::AppState> {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("download runtime database");
    let (state, _receiver) = test_state_with_env_parts(
        env.with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    download_request_seed(&state).await;
    db.close_for_test().await;
    state
}

/// Differential proof for the residual compatibility and request-level
/// download controller cases.
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
async fn controller_api_differential_downloads_open_cases() {
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
    let json_value = |response: &super::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let valid_id = "11111111-1111-4111-8111-111111111111";
    let download_body = r#"{"items":[{"user":"download-peer","remotePath":"Remote/Download.flac","targetDir":"Downloads"}]}"#;

    // GET /api/downloads
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            conversation_request(&state, "GET", "/api/downloads?status=not-a-state", "").await;
        record!(
            "GET",
            "/api/downloads",
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_value(&malformed)["downloads"].is_array()
        );
        let missing = conversation_request(&state, "GET", "/api/downloads", "").await;
        record!(
            "GET",
            "/api/downloads",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && json_value(&missing)["downloads"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
        download_request_seed(&state).await;
        let populated = conversation_request(&state, "GET", "/api/downloads", "").await;
        record!(
            "GET",
            "/api/downloads",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && json_value(&populated)["downloads"]
                    .as_array()
                    .is_some_and(|downloads| downloads.len() == 1)
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let response = conversation_request(&state, "GET", "/api/downloads", "").await;
        record!(
            "GET",
            "/api/downloads",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    // GET /api/downloads/{id}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal =
            conversation_request(&state, "GET", &format!("/api/downloads/{request_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && json_value(&nominal)["Id"].is_string()
                && json_value(&nominal)["Status"].is_string()
        );
        let malformed = conversation_request(&state, "GET", "/api/downloads/not-a-guid", "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing =
            conversation_request(&state, "GET", &format!("/api/downloads/{valid_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime request id");
        let response =
            conversation_request(&state, "GET", &format!("/api/downloads/{request_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        download_request_add_attempt(&state, &request_id).await;
        let response =
            conversation_request(&state, "GET", &format!("/api/downloads/{request_id}"), "").await;
        record!(
            "GET",
            "/api/downloads/{id}",
            "populated-dynamic-state",
            response.status == "200 OK"
                && json_value(&response)["RemotePath"] == "Remote/Download.flac"
        );
    }

    // GET /api/v0/downloads/requests
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && json_value(&nominal).is_array()
        );
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/downloads/requests?state=not-a-state",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "malformed-path-query-or-body",
            malformed.status == "200 OK" && json_value(&malformed).is_array()
        );
        let missing = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && json_value(&missing).is_array()
        );
        download_request_seed(&state).await;
        let populated = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && json_value(&populated)
                    .as_array()
                    .is_some_and(|requests| requests.len() == 1)
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let response = conversation_request(&state, "GET", "/api/v0/downloads/requests", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }

    // GET /api/v0/downloads/requests/{id:guid}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{request_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && json_value(&nominal)["request"].is_object()
                && json_value(&nominal)["attempts"].is_array()
        );
        let malformed =
            conversation_request(&state, "GET", "/api/v0/downloads/requests/not-a-guid", "").await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{valid_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime detail request id");
        let response = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{request_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        download_request_add_attempt(&state, &request_id).await;
        let response = conversation_request(
            &state,
            "GET",
            &format!("/api/v0/downloads/requests/{request_id}"),
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/downloads/requests/{id:guid}",
            "populated-dynamic-state",
            response.status == "200 OK"
                && json_value(&response)["attempts"]
                    .as_array()
                    .is_some_and(|attempts| attempts.len() == 2)
        );
    }

    // PATCH /api/v0/downloads/requests/{id:guid}/name
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{request_id}/name"),
            r#"{"name":"Renamed"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && json_value(&nominal)["request"].is_object()
        );
        let malformed = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{request_id}/name"),
            r#"{"name":""}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let mutation = state
            .transfers
            .read()
            .await
            .entries
            .iter()
            .all(|entry| entry.request_name.as_deref() == Some("Renamed"));
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "mutation-side-effects-and-readback",
            mutation
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{valid_id}/name"),
            r#"{"name":"Missing"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime rename request id");
        let response = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{request_id}/name"),
            r#"{"name":"Runtime"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "PATCH",
            &format!("/api/v0/downloads/requests/{valid_id}/name"),
            r#"{"name":"Restart"}"#,
        )
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let path = format!("/api/v0/downloads/requests/{request_id}/name");
        let responses = futures_util::future::join_all([
            conversation_request(&state, "PATCH", &path, r#"{"name":"One"}"#),
            conversation_request(&state, "PATCH", &path, r#"{"name":"Two"}"#),
        ])
        .await;
        record!(
            "PATCH",
            "/api/v0/downloads/requests/{id:guid}/name",
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
        );
    }

    // POST /api/downloads
    {
        let (state, mut receiver) = test_state_with_env(target_env());
        let nominal = conversation_request(&state, "POST", "/api/downloads", download_body).await;
        let command = receiver.try_recv().ok();
        record!(
            "POST",
            "/api/downloads",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && json_value(&nominal)["downloadIds"].is_array()
                && matches!(command, Some(super::SessionCommand::TransferPeer { .. }))
        );
        let malformed = conversation_request(&state, "POST", "/api/downloads", "{} ").await;
        record!(
            "POST",
            "/api/downloads",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = conversation_request(&state, "POST", "/api/downloads", "").await;
        record!(
            "POST",
            "/api/downloads",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
        let mutation = state.transfers.read().await.entries.len() == 1;
        record!(
            "POST",
            "/api/downloads",
            "mutation-side-effects-and-readback",
            mutation
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let response = conversation_request(&state, "POST", "/api/downloads", download_body).await;
        record!(
            "POST",
            "/api/downloads",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(&state, "POST", "/api/downloads", download_body).await;
        record!(
            "POST",
            "/api/downloads",
            "restart-persistence-or-reset",
            response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let responses = futures_util::future::join_all([
            conversation_request(&state, "POST", "/api/downloads", download_body),
            conversation_request(&state, "POST", "/api/downloads", download_body),
        ])
        .await;
        record!(
            "POST",
            "/api/downloads",
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
        );
    }

    // POST /api/v0/downloads/requests/{id:guid}/cancel
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let nominal = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{request_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "nominal-status-headers-body",
            nominal.status == "204 No Content" && nominal.body.is_empty()
        );
        let malformed = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{request_id}/cancel/extra"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        let mutation = state
            .transfers
            .read()
            .await
            .entries
            .iter()
            .all(|entry| entry.status == "cancelled");
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "mutation-side-effects-and-readback",
            mutation
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{valid_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let state = download_runtime_state(target_env()).await;
        let request_id = state
            .transfers
            .read()
            .await
            .entries
            .first()
            .and_then(|entry| entry.request_id.clone())
            .expect("runtime cancel request id");
        let response = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{request_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("transfer storage unavailable")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "POST",
            &format!("/api/v0/downloads/requests/{valid_id}/cancel"),
            "",
        )
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let request_id = download_request_seed(&state).await;
        let path = format!("/api/v0/downloads/requests/{request_id}/cancel");
        let responses = futures_util::future::join_all([
            conversation_request(&state, "POST", &path, ""),
            conversation_request(&state, "POST", &path, ""),
        ])
        .await;
        record!(
            "POST",
            "/api/v0/downloads/requests/{id:guid}/cancel",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
        );
    }

    assert_eq!(ledger.len(), 40, "downloads residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create downloads evidence directory");
    fs::write(
        evidence_dir.join("downloads_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize downloads ledger"),
    )
    .expect("write downloads ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api downloads mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

fn file_path_env(target: &str, remote_management: bool) -> MapEnv {
    MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with(
            "SLSKD_REMOTE_FILE_MANAGEMENT",
            if remote_management { "true" } else { "false" },
        )
}

fn file_path_segment(value: &str) -> String {
    super::STANDARD_NO_PAD.encode(value.as_bytes())
}

fn file_storage_root(state: &Arc<super::AppState>, storage: &str) -> PathBuf {
    if storage == "downloads" {
        super::effective_downloads_dir(state)
    } else {
        super::effective_incomplete_dir(state)
    }
}

fn replace_file_storage_root(state: &Arc<super::AppState>, storage: &str, path: PathBuf) {
    if storage == "downloads" {
        *state
            .downloads_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = path;
    } else {
        *state
            .incomplete_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = path;
    }
}

async fn file_runtime_state(storage: &str) -> Arc<super::AppState> {
    let (state, _receiver) = test_state_with_env(file_path_env("slskdn", true));
    let root = file_storage_root(&state, storage);
    let bad_root = root.with_extension("not-a-directory");
    fs::write(&bad_root, b"not a directory").expect("create file storage failure fixture");
    replace_file_storage_root(&state, storage, bad_root);
    state
}

/// Differential proof for the residual slskdn FilesController cases.
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
async fn controller_api_differential_files_open_cases() {
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

    let json_value = |response: &super::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let delete_cases = [
        (
            "downloads",
            "directories",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "directories",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ];

    for (storage, _resource, route_template) in delete_cases {
        let prefix = if storage == "downloads" {
            "/api/v0/files/downloads/directories/"
        } else {
            "/api/v0/files/incomplete/directories/"
        };
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let path = file_storage_root(&state, storage).join("Nominal");
            fs::create_dir_all(&path).expect("create nominal directory");
            fs::write(path.join("file.txt"), b"data").expect("create nominal file");
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Nominal")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "nominal-status-headers-body",
                response.status == "204 No Content" && response.body.is_empty()
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response =
                conversation_request(&state, "DELETE", &format!("{prefix}not-base64!"), "").await;
            record!(
                "DELETE",
                route_template,
                "malformed-path-query-or-body",
                response.status == "400 Bad Request"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Missing")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "missing-empty-or-conflict-state",
                response.status == "404 Not Found"
            );
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Runtime")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let path = file_storage_root(&state, storage).join("Mutation");
            fs::create_dir_all(&path).expect("create mutation directory");
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Mutation")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "mutation-side-effects-and-readback",
                response.status == "204 No Content" && !path.exists()
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Restart")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "restart-persistence-or-reset",
                response.status == "404 Not Found"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let path = file_storage_root(&state, storage).join("Concurrent");
            fs::create_dir_all(&path).expect("create concurrent directory");
            let request_path = format!("{prefix}{}", file_path_segment("Concurrent"));
            let responses = futures_util::future::join_all([
                conversation_request(&state, "DELETE", &request_path, ""),
                conversation_request(&state, "DELETE", &request_path, ""),
            ])
            .await;
            let statuses = responses
                .iter()
                .map(|response| response.status)
                .collect::<Vec<_>>();
            record!(
                "DELETE",
                route_template,
                "concurrency-and-idempotency",
                statuses.contains(&"204 No Content") && statuses.contains(&"404 Not Found")
            );
        }
    }

    for (storage, route_template) in [
        (
            "downloads",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "incomplete",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
    ] {
        let prefix = if storage == "downloads" {
            "/api/v0/files/downloads/files/"
        } else {
            "/api/v0/files/incomplete/files/"
        };
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(&root).expect("create file storage root");
            let path = root.join("Nominal.txt");
            fs::write(&path, b"data").expect("create nominal storage file");
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Nominal.txt")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "nominal-status-headers-body",
                response.status == "204 No Content" && response.body.is_empty()
            );
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Runtime.txt")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let response = conversation_request(
                &state,
                "DELETE",
                &format!("{prefix}{}", file_path_segment("Restart.txt")),
                "",
            )
            .await;
            record!(
                "DELETE",
                route_template,
                "restart-persistence-or-reset",
                response.status == "204 No Content"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, true));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(&root).expect("create concurrent file root");
            fs::write(root.join("Concurrent.txt"), b"data").expect("create concurrent file");
            let request_path = format!("{prefix}{}", file_path_segment("Concurrent.txt"));
            let responses = futures_util::future::join_all([
                conversation_request(&state, "DELETE", &request_path, ""),
                conversation_request(&state, "DELETE", &request_path, ""),
            ])
            .await;
            let statuses = responses
                .iter()
                .map(|response| response.status)
                .collect::<Vec<_>>();
            record!(
                "DELETE",
                route_template,
                "concurrency-and-idempotency",
                statuses.iter().all(|status| status == &"204 No Content")
            );
        }
    }

    {
        let storage = "incomplete";
        let route_template = "/api/v0/files/incomplete/files/{base64FileName}";
        let prefix = "/api/v0/files/incomplete/files/";
        let (state, _receiver) = test_state_with_env(file_path_env(target, true));
        let malformed =
            conversation_request(&state, "DELETE", &format!("{prefix}not-base64!"), "").await;
        record!(
            "DELETE",
            route_template,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = conversation_request(
            &state,
            "DELETE",
            &format!("{prefix}{}", file_path_segment("Missing.txt")),
            "",
        )
        .await;
        record!(
            "DELETE",
            route_template,
            "missing-empty-or-conflict-state",
            missing.status == "204 No Content"
        );
        let root = file_storage_root(&state, storage);
        fs::create_dir_all(&root).expect("create incomplete mutation root");
        let path = root.join("Mutation.txt");
        fs::write(&path, b"data").expect("create incomplete mutation file");
        let mutation = conversation_request(
            &state,
            "DELETE",
            &format!("{prefix}{}", file_path_segment("Mutation.txt")),
            "",
        )
        .await;
        record!(
            "DELETE",
            route_template,
            "mutation-side-effects-and-readback",
            mutation.status == "204 No Content" && !path.exists()
        );
    }

    for storage in ["downloads", "incomplete"] {
        let (collection_route, detail_prefix, collection_template, detail_template) =
            if storage == "downloads" {
                (
                    "/api/v0/files/downloads/directories",
                    "/api/v0/files/downloads/directories/",
                    "/api/v0/files/downloads/directories",
                    "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
                )
            } else {
                (
                    "/api/v0/files/incomplete/directories",
                    "/api/v0/files/incomplete/directories/",
                    "/api/v0/files/incomplete/directories",
                    "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
                )
            };
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let response = conversation_request(
                &state,
                "GET",
                &format!("{collection_route}?recursive=maybe"),
                "",
            )
            .await;
            record!(
                "GET",
                collection_template,
                "malformed-path-query-or-body",
                response.status == "400 Bad Request"
            );
            let empty = conversation_request(&state, "GET", collection_route, "").await;
            record!(
                "GET",
                collection_template,
                "missing-empty-or-conflict-state",
                empty.status == "200 OK" && json_value(&empty)["files"].is_array()
            );
            if storage == "incomplete" {
                record!(
                    "GET",
                    collection_template,
                    "nominal-status-headers-body",
                    empty.status == "200 OK" && json_value(&empty)["files"].is_array()
                );
            }
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(&state, "GET", collection_route, "").await;
            record!(
                "GET",
                collection_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(root.join("Album")).expect("create populated root");
            fs::write(root.join("Album").join("Track.flac"), b"track")
                .expect("create populated track");
            let response = conversation_request(&state, "GET", collection_route, "").await;
            if storage == "incomplete" {
                record!(
                    "GET",
                    collection_template,
                    "populated-dynamic-state",
                    response.status == "200 OK"
                        && json_value(&response)["directories"]
                            .as_array()
                            .is_some_and(|directories| !directories.is_empty())
                );
            }
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let root = file_storage_root(&state, storage);
            fs::create_dir_all(root.join("Album")).expect("create nominal detail root");
            fs::write(root.join("Album").join("Track.flac"), b"track")
                .expect("create nominal detail track");
            let response = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Album")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "nominal-status-headers-body",
                response.status == "200 OK" && json_value(&response)["files"].is_array()
            );
            let malformed =
                conversation_request(&state, "GET", &format!("{detail_prefix}not-base64!"), "")
                    .await;
            record!(
                "GET",
                detail_template,
                "malformed-path-query-or-body",
                malformed.status == "400 Bad Request"
            );
            let missing = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Missing")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "missing-empty-or-conflict-state",
                missing.status == "404 Not Found"
            );
        }
        {
            let state = file_runtime_state(storage).await;
            let response = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Runtime")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(file_path_env(target, false));
            let root = file_storage_root(&state, storage);
            let album = root.join("Album");
            fs::create_dir_all(&album).expect("create populated detail root");
            fs::write(album.join("Track.flac"), b"track").expect("create populated detail file");
            let response = conversation_request(
                &state,
                "GET",
                &format!("{detail_prefix}{}", file_path_segment("Album")),
                "",
            )
            .await;
            record!(
                "GET",
                detail_template,
                "populated-dynamic-state",
                response.status == "200 OK"
                    && json_value(&response)["files"]
                        .as_array()
                        .is_some_and(|files| files.len() == 1)
            );
        }
    }

    assert_eq!(ledger.len(), 43, "files residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create files evidence directory");
    fs::write(
        evidence_dir.join("files_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize files ledger"),
    )
    .expect("write files ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api files mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
fn run_bounded_future<F>(future: F)
where
    F: std::future::Future<Output = ()>,
{
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("bounded differential runtime")
        .block_on(future);
}

fn run_controller_future_on_large_stack<F, Fut>(name: &'static str, factory: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + 'static,
{
    std::thread::Builder::new()
        .name(name.to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create large-stack controller test runtime")
                .block_on(factory())
        })
        .expect("spawn large-stack controller test")
        .join()
        .expect("join large-stack controller test");
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
fn run_bounded_controller_api_tests_1() {
    run_bounded_future(async {
        controller_api_differential_native_library_fallback_uses_current_case_mode_for_share_filters().await;
        controller_api_differential_session_issue_and_revoke().await;
        controller_api_differential_server_session_open_cases().await;
        controller_api_differential_bounded_activity_and_network_polling_routes_project_local_state().await;
        controller_api_differential_native_network_stats_edge_contracts().await;
        controller_api_differential_native_read_projection_runtime_contracts().await;
        controller_api_differential_native_solid_status_edge_contracts().await;
        controller_api_differential_native_solid_resolution_runtime_contracts().await;
        controller_api_differential_incremental_transfer_and_message_routes_validate_cursors_and_bound_history().await;
        controller_api_differential_portforwarding_start_readback().await;
        controller_api_differential_overlay_gateway_populated_gets().await;
        controller_api_differential_automation_compat_routes_use_expected_shapes().await;
        controller_api_differential_peer_and_mesh_preview_stream_tickets_are_short_lived().await;
        controller_api_differential_peer_stream_ticket_validation_and_limits().await;
        controller_api_differential_mesh_stream_ticket_validation_and_limits().await;
        controller_api_differential_controller_application_dump_contracts().await;
        run_controller_future_on_large_stack("native-application-dump-gates", || {
            controller_api_differential_native_application_dump_gates_impl()
        });
        controller_api_differential_native_application_open_cases().await;
        controller_api_differential_search_api_creates_reads_and_completes_records().await;
        controller_api_differential_search_creation_rehydrates().await;
        controller_api_differential_search_mutation_lifecycle().await;
        controller_api_differential_library_issue_fix_rehydrates().await;
        controller_api_differential_transfer_api_creates_updates_and_reports_stats().await;
        controller_api_differential_transfer_cleanup_persistence().await;
        controller_api_differential_transfer_report_contracts().await;
        controller_api_differential_transfer_upload_diagnostics().await;
        controller_api_differential_compatibility_aliases_reach_state_backed_routes().await;
        controller_api_differential_native_capability_and_library_health_contracts().await;
        controller_api_differential_mesh_stats_reflect_real_merge_activity_not_hardcoded_zeros()
            .await;
        controller_api_differential_mesh_message_runtime().await;
        controller_api_differential_mesh_controller_edge_cases().await;
        controller_api_differential_mesh_runtime_and_nat_lifecycle().await;
        controller_api_differential_mesh_merge_publish_restart_and_concurrency().await;
        controller_api_differential_mesh_sync_failure_and_concurrency_contracts().await;
        controller_api_differential_podcore_content_metadata_requires_a_real_content_id().await;
        controller_api_differential_podcore_content_metadata_uses_musicbrainz_recording_release_and_artist_shapes().await;
        controller_api_differential_podcore_content_search_returns_musicbrainz_recording_results()
            .await;
        controller_api_differential_podcore_dht_stats_reflect_real_publications_not_a_pod_count_proxy().await;
        controller_api_differential_podcore_dht_metadata_reads_and_verifies_the_published_record()
            .await;
        controller_api_differential_podcore_backfill_sync_and_sync_all_report_real_local_work()
            .await;
        controller_api_differential_podcore_discovery_stats_use_registrations_and_search_activity()
            .await;
        controller_api_differential_user_notes_lifecycle().await;
        controller_api_differential_mesh_http_disabled_shape().await;
        controller_api_differential_library_jobs_and_discovery_projections().await;
        controller_api_differential_user_browse_api_requests_and_ingests_entries().await;
        controller_api_differential_controller_browse_status_tracks_request_failure_and_completion(
        )
        .await;
        controller_api_differential_joined_room_server_snapshot_populates_the_real_user_roster()
            .await;
        controller_api_differential_soulseek_user_interests_route_returns_remote_server_response()
            .await;
        controller_api_differential_uuid_guarded_families_reject_malformed_first_id().await;
        controller_api_differential_versioned_get_contract_fixed_route_responses().await;
        controller_api_differential_versioned_get_contract_missing_resource_responses().await;
        controller_api_differential_read_only_routes_have_real_contract_shapes().await;
        controller_api_differential_runtime_control_routes_survive_persistence_failure().await;
        controller_api_differential_contact_wishlist_collection_routes_survive_persistence_failure(
        )
        .await;
        controller_api_differential_contacts_versioned_crud_persistence_and_concurrency().await;
        controller_api_differential_contacts_discovery_and_read_edges().await;
        controller_api_differential_library_interests_nowplaying_messages_survive_persistence_failure().await;
        controller_api_differential_pod_management_routes_persist_crud_members_and_bindings().await;
        controller_api_differential_collections_items_crud_reorder_lifecycle().await;
        controller_api_differential_collections_persistence_and_concurrency().await;
        controller_api_differential_activity_hashdb_and_transport_status_gets().await;
        controller_api_differential_mesh_rendezvous_and_capabilities_gets().await;
        controller_api_differential_dht_rendezvous_residuals().await;
        controller_api_differential_swarm_analytics_gets().await;
        controller_api_differential_compatibility_projection_tail().await;
        controller_api_differential_mesh_signal_runtime_switches().await;
        controller_api_differential_bridge_config_populated_projection().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
fn run_bounded_controller_api_tests_2() {
    run_bounded_future(async {
        controller_api_differential_bridge_clients_populated_projection().await;
        controller_api_differential_bridge_rooms_populated_projection().await;
        controller_api_differential_podcore_stats_gets().await;
        controller_api_differential_podcore_routing().await;
        controller_api_differential_podcore_maintenance_mutations().await;
        controller_api_differential_podcore_channel_crud().await;
        controller_api_differential_podcore_channel_lifecycle().await;
        controller_api_differential_podcore_opinion_empty_gets().await;
        controller_api_differential_podcore_opinion_populated_gets().await;
        controller_api_differential_podcore_opinion_publish().await;
        controller_api_differential_podcore_opinion_actions().await;
        controller_api_differential_podcore_opinion_missing_gets_and_actions().await;
        controller_api_differential_podcore_signing_verify().await;
        controller_api_differential_share_grants_crud().await;
        controller_api_differential_share_grants_persistence_and_concurrency().await;
        controller_api_differential_mediacore_mutations().await;
        controller_api_differential_mediacore_validation_tail().await;
        controller_api_differential_mediacore_descriptor_lifecycle().await;
        controller_api_differential_mediacore_ipld_and_fuzzy_find().await;
        controller_api_differential_materialized_empty_state_gets().await;
        controller_api_differential_mediacore_stats_malformed_queries().await;
        controller_api_differential_mediacore_resource_malformed_queries().await;
        controller_api_differential_openapi_mutation_dtos_tail().await;
        controller_api_differential_collections_ownership_scoping().await;
        controller_api_differential_share_grants_ownership_scoping().await;
        controller_api_differential_bridge_routes().await;
        controller_api_differential_unversioned_bridge_version_validation().await;
        controller_api_differential_unversioned_mutation_version_validation().await;
        controller_api_differential_songid_run_lifecycle().await;
        controller_api_differential_listening_party_and_transports_status().await;
        controller_api_differential_listening_party_open_cases().await;
        controller_api_differential_activitypub_actor_and_webfinger().await;
        controller_api_differential_activitypub_collection_empty_and_missing_gets().await;
        controller_api_differential_activitypub_open_cases();
        controller_api_differential_hashdb_paging().await;
        controller_api_differential_hashdb_validation_and_empty_contracts().await;
        controller_api_differential_discovery_graph_and_opinions().await;
        controller_api_differential_opinion_open_cases().await;
        controller_api_differential_discovery_graph_edge_contracts().await;
        controller_api_differential_deterministic_openapi_mutations().await;
        controller_api_differential_versioned_openapi_validation_rejections().await;
        controller_api_differential_versioned_openapi_large_dtos().await;
        controller_api_differential_versioned_auxiliary_mutations().await;
        controller_api_differential_release_radar().await;
        controller_api_differential_library_health().await;
        controller_api_differential_library_health_versioned_edge_states().await;
        controller_api_differential_bridge_admin_and_federation_diagnostics().await;
        controller_api_differential_bridge_admin_stats_and_source_feed_preview().await;
        controller_api_differential_extended_controller_mutations().await;
        controller_api_differential_native_ranking_contracts().await;
        controller_api_differential_quarantine_jury().await;
        controller_api_differential_quarantine_jury_open_cases().await;
        controller_api_differential_content_bound_stream_tickets().await;
        run_controller_future_on_large_stack("primary-stream-ticket-lifecycle", || {
            controller_api_differential_primary_stream_ticket_lifecycle_impl()
        });
        controller_api_differential_port_forwarding().await;
        controller_api_differential_security_reputation().await;
        controller_api_differential_realm_subject_indexes().await;
        controller_api_differential_musicbrainz_overlay_export().await;
        controller_api_differential_pod_membership_workflow().await;
        controller_api_differential_pod_channel_messages().await;
        controller_api_differential_podcore_message_storage().await;
        controller_api_differential_podcore_membership_storage().await;
        controller_api_differential_podcore_discovery_storage().await;
        controller_api_differential_podcore_join_leave_residuals().await;
        controller_api_differential_security_ban_residuals().await;
        controller_api_differential_security_diagnostics_residuals().await;
        controller_api_differential_soulseek_discovery_residuals().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-3"
))]
fn run_bounded_controller_api_tests_3() {
    run_bounded_future(async {
        controller_api_differential_multisource_residuals().await;
        controller_api_differential_podcore_route_value_validation().await;
        controller_api_differential_podcore_request_validation().await;
        controller_api_differential_hashdb_history_backfill().await;
        controller_api_differential_user_group().await;
        controller_api_differential_virtual_soulfind_v2().await;
        controller_api_differential_virtual_soulfind_v2_residuals().await;
        controller_api_differential_source_discovery().await;
        controller_api_differential_discovery_open_cases().await;
        controller_api_differential_pod_and_jury_stats().await;
        controller_api_differential_playback_feedback_and_diagnostics().await;
        controller_api_differential_nowplaying_delete_and_playback_diagnostics_edge_states().await;
        controller_api_differential_native_nowplaying_webhook_contracts().await;
        controller_api_differential_activitypub_inbox_relationships();
        controller_api_differential_activitypub_outbox_and_undo();
        controller_api_differential_conversations_delete_survives_persistence_failure().await;
        controller_api_differential_spotify_oauth_authorize_and_callback().await;
        controller_api_differential_spotify_connection_status_and_disconnect().await;
        controller_api_differential_mesh_http_gateway().await;
        controller_api_differential_solid_status_and_webid_resolution().await;
        controller_api_differential_pod_membership_self_publish().await;
        controller_api_differential_pod_membership_moderation_publish().await;
        controller_api_differential_transfer_reports_required_direction().await;
        controller_api_differential_transfer_download_cancel().await;
        controller_api_differential_transfer_upload_cancel().await;
        controller_api_differential_analyzer_hashdb_and_telemetry().await;
        controller_api_differential_source_provider_catalog().await;
        controller_api_differential_source_provider_edge_contracts().await;
        controller_api_differential_versioned_soulseek_recommendations().await;
        controller_api_differential_versioned_soulseek_item_discovery().await;
        controller_api_differential_versioned_soulseek_similar_users().await;
        controller_api_differential_versioned_autoreplace_status().await;
        controller_api_differential_versioned_autoreplace_populated_state().await;
        controller_api_differential_native_autoreplace_edge_contracts().await;
        controller_api_differential_native_options_edge_contracts().await;
        controller_api_differential_native_events_edge_contracts().await;
        controller_api_differential_versioned_mesh_health_and_signals().await;
        controller_api_differential_versioned_signals_edge_contracts().await;
        controller_api_differential_traces_summary_contracts().await;
        controller_api_differential_compatibility_info_and_fairness_contracts().await;
        controller_api_differential_compatibility_user_browse_contracts().await;
        controller_api_differential_build_info_and_file_delete().await;
        controller_api_differential_application_version_state_contracts().await;
        controller_api_differential_application_populated_versioned_state().await;
        controller_api_differential_populated_compatibility_status_and_capabilities().await;
        controller_api_differential_versioned_capability_peer_projections().await;
        controller_api_differential_versioned_capability_peers_nominal().await;
        controller_api_differential_native_capabilities_contracts().await;
        controller_api_differential_native_profile_contracts().await;
        controller_api_differential_versioned_destinations_nominal().await;
        controller_api_differential_versioned_backfill_candidates_nominal().await;
        controller_api_differential_versioned_multisource_jobs_nominal().await;
        controller_api_differential_versioned_swarm_trends_nominal().await;
        controller_api_differential_versioned_swarm_dashboard_nominal().await;
        controller_api_differential_versioned_transfer_history_nominal().await;
        controller_api_differential_versioned_transfer_empty_lists_nominal().await;
        controller_api_differential_versioned_transfer_changes_nominal().await;
        controller_api_differential_versioned_transfer_summary_nominal().await;
        controller_api_differential_versioned_transfer_histogram_nominal().await;
        controller_api_differential_versioned_transfer_reports_populated_state().await;
        controller_api_differential_versioned_destinations_populated_state().await;
        controller_api_differential_native_destinations_edge_contracts().await;
        controller_api_differential_runtime_failure_security_and_shares().await;
        controller_api_differential_sharegroups_and_shares_rebuild().await;
        controller_api_differential_sharegroups_persistence_and_concurrency().await;
        controller_api_differential_options_and_conversations_projection().await;
        controller_api_differential_options_current_overlay_lifecycle().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
fn run_bounded_controller_api_tests_4() {
    run_bounded_future(async {
        controller_api_differential_spotify_pods_mesh_and_party().await;
        controller_api_differential_application_interests_bridge_mediacore_adversarial().await;
        controller_api_differential_storage_and_server().await;
        controller_api_differential_searches_dispatch_and_visualizer_launch().await;
        controller_api_differential_options_dht_and_bridge_redaction().await;
        controller_api_differential_mediacore_delete_and_mesh_tickets().await;
        controller_api_differential_lidarr_and_source_feed_contracts().await;
        controller_api_differential_source_feed_open_cases().await;
        controller_api_differential_controller_file_transfer_and_room_contracts().await;
        controller_api_differential_controller_file_application_and_roster_edges().await;
        controller_api_differential_controller_share_and_relay_lifecycle().await;
        controller_api_differential_controller_fixed_route_malformed_paths().await;
        controller_api_differential_controller_parameterized_malformed_paths().await;
        controller_api_differential_controller_empty_and_missing_state().await;
        controller_api_differential_controller_relay_controller_routes().await;
        controller_api_differential_controller_core_application_session_events_and_telemetry()
            .await;
        controller_api_differential_controller_core_failure_restart_and_empty_contracts().await;
        controller_api_differential_controller_users_and_shares().await;
        controller_api_differential_controller_rooms_and_conversations().await;
        controller_api_differential_controller_rooms_conversations_restart_and_failure().await;
        controller_api_differential_controller_server_state_and_lifecycle().await;
        controller_api_differential_controller_search_lifecycle().await;
        controller_api_differential_controller_search_failure_restart_and_idempotency().await;
        controller_api_differential_native_search_compatibility_contracts().await;
        controller_api_differential_native_searches_open_cases().await;
        controller_api_differential_controller_upload_lifecycle().await;
        controller_api_differential_controller_transfer_failure_restart_and_idempotency().await;
        controller_api_differential_controller_transfer_batch_cleanup_and_failures().await;
        controller_api_differential_controller_user_browse_contracts().await;
        controller_api_differential_controller_download_edge_contracts().await;
        controller_api_differential_controller_runtime_failure_isolation_contracts().await;
        controller_api_differential_native_security_runtime_failure_contracts().await;
        controller_api_differential_controller_options_overlay_contracts().await;
        controller_api_differential_native_transfers_runtime_failure_contracts().await;
        controller_api_differential_native_transfers_empty_and_missing_contracts().await;
        controller_api_differential_native_transfers_malformed_contracts().await;
        controller_api_differential_native_transfers_nominal_populated_contracts().await;
        controller_api_differential_native_transfers_restart_and_concurrency().await;
        controller_api_differential_options_action_routes().await;
        controller_api_differential_controller_residual_core_contracts();
        controller_api_differential_hashdb_domain_contracts().await;
        controller_api_differential_pods_controller_residuals().await;
        controller_api_differential_wishlist_controller_residuals().await;
        controller_api_differential_virtual_soulfind_legacy_residuals().await;
        controller_api_differential_rooms_controller_residuals().await;
        controller_api_differential_bridge_controller_residuals().await;
        run_controller_future_on_large_stack("podcore-residuals", || {
            controller_api_differential_podcore_residuals_impl()
        });
        controller_api_differential_mediacore_residuals().await;
        controller_api_differential_musicbrainz_residuals().await;
        controller_api_differential_jobs_residuals().await;
        controller_api_differential_library_residuals().await;
        controller_api_differential_security_controller_residuals().await;
        controller_api_differential_integrations_residuals().await;
        controller_api_differential_backfill_residuals().await;
        controller_api_differential_native_native_open_cases().await;
        controller_api_differential_audio_canonical_dedupe_and_migration().await;
        controller_api_differential_taste_recommendation_open_cases().await;
        controller_api_differential_songid_open_cases().await;
        controller_api_differential_share_grants_open_cases().await;
        controller_api_differential_shares_open_cases().await;
        controller_api_differential_users_open_cases().await;
        controller_api_differential_telemetry_open_cases().await;
        run_controller_future_on_large_stack("relay-open-cases", || {
            controller_api_differential_relay_open_cases_impl()
        });
        controller_api_differential_conversations_open_cases().await;
        controller_api_differential_downloads_open_cases().await;
        controller_api_differential_files_open_cases().await;
    });
}

#[cfg(any(
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1",
    feature = "bounded-controller-api-tests-2",
    feature = "bounded-controller-api-tests-3",
    feature = "bounded-controller-api-tests-4"
))]
fn run_bounded_controller_api_tests() {
    #[cfg(feature = "bounded-controller-api-tests")]
    {
        run_bounded_controller_api_tests_1();
        run_bounded_controller_api_tests_2();
        run_bounded_controller_api_tests_3();
        run_bounded_controller_api_tests_4();
    }
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-1"
    ))]
    run_bounded_controller_api_tests_1();
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-2"
    ))]
    run_bounded_controller_api_tests_2();
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-3"
    ))]
    run_bounded_controller_api_tests_3();
    #[cfg(all(
        not(feature = "bounded-controller-api-tests"),
        feature = "bounded-controller-api-tests-4"
    ))]
    run_bounded_controller_api_tests_4();
}

#[cfg(feature = "bounded-persistence-tests")]
fn run_bounded_persistence_tests() {
    run_bounded_future(async {
        persistence_lifecycle_differential_search_event_transfer_message_domains_roundtrip_and_rehydrate().await;
        persistence_lifecycle_differential_transfers_domain_rehydrates_from_sqlite().await;
        persistence_lifecycle_differential_collections_notes_wishlist_sharing_domains_roundtrip_and_rehydrate().await;
        persistence_lifecycle_differential_wishlist_ignored_results_domain().await;
        persistence_lifecycle_differential_pod_core_file_state().await;
        persistence_lifecycle_differential_collections_notes_wishlist_sharing_domains_update_delete_and_readback().await;
        persistence_lifecycle_differential_search_and_message_domains_update_delete_and_readback()
            .await;
        persistence_lifecycle_differential_covered_domains_schema_create_and_migrate().await;
        persistence_lifecycle_differential_covered_domains_transaction_and_concurrency_atomicity()
            .await;
        persistence_lifecycle_differential_covered_domains_corrupt_state_and_upgrade_failure()
            .await;
        persistence_lifecycle_differential_controller_batches_domain().await;
        persistence_lifecycle_differential_controller_share_files_domain().await;
        persistence_lifecycle_differential_transfers_domain_full_lifecycle().await;
        persistence_lifecycle_differential_native_hashdb_domains().await;
        persistence_lifecycle_differential_native_songid_runs().await;
        persistence_lifecycle_differential_native_traffic_stats_domain().await;
    });
}

#[cfg(feature = "bounded-file-lifecycle-tests")]
fn run_bounded_file_lifecycle_tests() {
    run_bounded_future(async {
        file_lifecycle_differential_options_controller_backup_and_reload().await;
        file_lifecycle_differential_options_controller_rejects_backup_symlink().await;
        file_lifecycle_differential_files_service_roots_and_metadata().await;
        file_lifecycle_differential_download_service_path_and_retry().await;
        file_lifecycle_differential_relay_agent_download_cleanup_and_reload().await;
        file_lifecycle_differential_secure_file_writer_download_open();
        file_lifecycle_differential_dht_certificate_manager_identity_files().await;
        file_lifecycle_differential_atomic_file_writer_common_cases();
        file_lifecycle_differential_mesh_certificate_pin_manager().await;
        file_lifecycle_differential_gold_star_club_revocation();
        file_lifecycle_differential_multisource_download_service().await;
    });
}

#[cfg(feature = "bounded-protocol-tests")]
fn run_bounded_protocol_tests() {
    run_bounded_future(async {
        protocol_behaviors_differential_overlay_gateway_mesh_search().await;
        protocol_behaviors_differential_mesh_sync_private_runtime().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_round_trips().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_raw_frames().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_dispatch().await;
        protocol_behaviors_differential_virtual_soulfind_bridge_malformed_frames().await;
    });
}

#[cfg(feature = "bounded-security-control-tests")]
fn run_bounded_security_control_tests() {
    run_bounded_future(async {
        security_controls_differential_reputation_and_violation_runtime().await;
        security_controls_differential_path_and_file_guards();
        security_controls_differential_share_token_store();
        run_controller_future_on_large_stack(
            "security-controls-csrf-filter-bounded",
            security_controls_differential_csrf_filter_impl,
        );
        security_controls_differential_hardening_validator();
        security_controls_differential_certificate_manager().await;
        security_controls_differential_overlay_message_validation().await;
        security_controls_differential_solid_fetch_policy().await;
        security_controls_differential_mesh_surface().await;
        security_controls_differential_content_safety().await;
        security_controls_differential_soulseek_safety();
        security_controls_differential_security_event_sink();
        security_controls_differential_integrity_controls();
        security_controls_differential_runtime_controls();
        security_controls_differential_route_security_adapters().await;
        security_controls_differential_mesh_transport().await;
        security_controls_differential_core_security().await;
        security_controls_differential_native_security_controller().await;
        security_controls_differential_passthrough_authentication();
        security_controls_differential_authentication_and_jwt();
    });
}

#[cfg(feature = "bounded-security-authorization-tests")]
fn run_bounded_security_authorization_tests() {
    security_authorization_matrix_matches_declared_policy_for_every_frozen_route();
}

#[cfg(any(
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
))]
pub fn run_bounded_differential_tests() {
    #[cfg(any(
        feature = "bounded-controller-api-tests",
        feature = "bounded-controller-api-tests-1",
        feature = "bounded-controller-api-tests-2",
        feature = "bounded-controller-api-tests-3",
        feature = "bounded-controller-api-tests-4"
    ))]
    run_bounded_controller_api_tests();
    #[cfg(feature = "bounded-persistence-tests")]
    run_bounded_persistence_tests();
    #[cfg(feature = "bounded-file-lifecycle-tests")]
    run_bounded_file_lifecycle_tests();
    #[cfg(feature = "bounded-protocol-tests")]
    run_bounded_protocol_tests();
    #[cfg(feature = "bounded-security-control-tests")]
    run_bounded_security_control_tests();
    #[cfg(feature = "bounded-security-authorization-tests")]
    run_bounded_security_authorization_tests();
}
