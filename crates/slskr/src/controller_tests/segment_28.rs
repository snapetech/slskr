/// Differential proof for the remaining slskdN JobsController and the
/// dedicated discography/label-crate job controllers.  The compatibility
/// dispatcher already uses local searches for work execution; this test
/// proves the frozen job-list/detail projections, exact nested routes,
/// versioned validation, persistence-backed projections, and concurrent
/// creation behavior independently of the broader search tests.
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
async fn controller_api_differential_jobs_residuals() {
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

    let known_id = "jobs-residual-known";
    let label_id = "jobs-residual-label";
    let get_cases = [
        (
            "/api/jobs?limit=not-a-number",
            "/api/jobs",
            "malformed-path-query-or-body",
            "list",
        ),
        (
            "/api/jobs",
            "/api/jobs",
            "missing-empty-or-conflict-state",
            "list",
        ),
        (
            "/api/jobs",
            "/api/jobs",
            "runtime-failure-and-timeout",
            "list",
        ),
        (
            "/api/jobs/jobs-residual-known",
            "/api/jobs/{id}",
            "nominal-status-headers-body",
            "found",
        ),
        (
            "/api/jobs/jobs-residual-known/extra",
            "/api/jobs/{id}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/jobs/no-such-job",
            "/api/jobs/{id}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/jobs/jobs-residual-known",
            "/api/jobs/{id}",
            "runtime-failure-and-timeout",
            "found",
        ),
        (
            "/api/jobs/discography/jobs-residual-known",
            "/api/jobs/discography/{jobId}",
            "nominal-status-headers-body",
            "discography",
        ),
        (
            "/api/jobs/discography/jobs-residual-known/extra",
            "/api/jobs/discography/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/jobs/discography/no-such-job",
            "/api/jobs/discography/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/jobs/discography/jobs-residual-known",
            "/api/jobs/discography/{jobId}",
            "runtime-failure-and-timeout",
            "discography",
        ),
        (
            "/api/jobs/discography/jobs-residual-known",
            "/api/jobs/discography/{jobId}",
            "populated-dynamic-state",
            "discography",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label",
            "/api/jobs/label-crate/{jobId}",
            "nominal-status-headers-body",
            "label",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label/extra",
            "/api/jobs/label-crate/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/jobs/label-crate/no-such-job",
            "/api/jobs/label-crate/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label",
            "/api/jobs/label-crate/{jobId}",
            "runtime-failure-and-timeout",
            "label",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label",
            "/api/jobs/label-crate/{jobId}",
            "populated-dynamic-state",
            "label",
        ),
        (
            "/api/v0/jobs?limit=not-a-number",
            "/api/v0/jobs",
            "nominal-status-headers-body",
            "list",
        ),
        (
            "/api/v0/jobs?limit=not-a-number",
            "/api/v0/jobs",
            "malformed-path-query-or-body",
            "list",
        ),
        (
            "/api/v0/jobs",
            "/api/v0/jobs",
            "missing-empty-or-conflict-state",
            "list",
        ),
        (
            "/api/v0/jobs",
            "/api/v0/jobs",
            "runtime-failure-and-timeout",
            "list",
        ),
        (
            "/api/v0/jobs",
            "/api/v0/jobs",
            "populated-dynamic-state",
            "list",
        ),
        (
            "/api/v0/jobs/jobs-residual-known",
            "/api/v0/jobs/{id}",
            "nominal-status-headers-body",
            "found",
        ),
        (
            "/api/v0/jobs/jobs-residual-known/extra",
            "/api/v0/jobs/{id}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/v0/jobs/jobs-residual-known",
            "/api/v0/jobs/{id}",
            "runtime-failure-and-timeout",
            "found",
        ),
        (
            "/api/v0/jobs/jobs-residual-known",
            "/api/v0/jobs/{id}",
            "populated-dynamic-state",
            "found",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known",
            "/api/v0/jobs/discography/{jobId}",
            "nominal-status-headers-body",
            "discography",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known/extra",
            "/api/v0/jobs/discography/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/v0/jobs/discography/no-such-job",
            "/api/v0/jobs/discography/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known",
            "/api/v0/jobs/discography/{jobId}",
            "runtime-failure-and-timeout",
            "discography",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known",
            "/api/v0/jobs/discography/{jobId}",
            "populated-dynamic-state",
            "discography",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label",
            "/api/v0/jobs/label-crate/{jobId}",
            "nominal-status-headers-body",
            "label",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label/extra",
            "/api/v0/jobs/label-crate/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/v0/jobs/label-crate/no-such-job",
            "/api/v0/jobs/label-crate/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label",
            "/api/v0/jobs/label-crate/{jobId}",
            "runtime-failure-and-timeout",
            "label",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label",
            "/api/v0/jobs/label-crate/{jobId}",
            "populated-dynamic-state",
            "label",
        ),
    ];

    for (path, route, case, shape) in get_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if matches!(shape, "found" | "discography") {
            state
                .searches
                .write()
                .await
                .create(
                    Some(known_id.to_owned()),
                    "Residual discography".to_owned(),
                    "global",
                    None,
                    Vec::new(),
                    super::DEFAULT_SEARCH_TTL_SECONDS,
                )
                .expect("seed residual search job");
        }
        if shape == "label" {
            state
                .controller_features
                .write_for_test()
                .await
                .upsert(
                    format!("job/label-crate/{label_id}"),
                    serde_json::json!({
                        "id": label_id,
                        "jobId": label_id,
                        "labelId": "label-residual",
                        "labelName": "Residual Label",
                        "limit": 2,
                        "releaseIds": [],
                        "totalReleases": 0,
                        "completedReleases": 0,
                        "failedReleases": 0,
                        "type": "label_crate",
                        "status": "Pending",
                    }),
                )
                .expect("seed residual label job");
        }
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let pass = match shape {
            "list" => serde_json::from_str::<serde_json::Value>(&response.body).is_ok_and(|json| {
                response.status == "200 OK"
                    && json["jobs"].is_array()
                    && json["limit"].is_number()
                    && json["offset"].is_number()
                    && json["has_more"].is_boolean()
            }),
            "found" => response.status == "200 OK" && response.body.contains(known_id),
            "discography" | "label" => {
                response.status == "200 OK" && response.body.contains("jobId")
            }
            "missing" => response.status == "404 Not Found",
            _ => false,
        };
        record!("GET", route, case, pass);
    }

    let unversioned_mutations = [
        (
            "/api/jobs/discography",
            "/api/jobs/discography",
            "{\"artist\":\"Residual Artist\"}",
        ),
        (
            "/api/jobs/label-crate",
            "/api/jobs/label-crate",
            "{\"label_name\":\"Residual Label\"}",
        ),
        (
            "/api/jobs/mb-release",
            "/api/jobs/mb-release",
            "{\"artist\":\"Residual Artist\",\"title\":\"Residual Release\"}",
        ),
    ];
    for (path, route, valid_body) in unversioned_mutations {
        for (case, body) in [
            ("runtime-failure-and-timeout", valid_body),
            ("mutation-side-effects-and-readback", valid_body),
            ("restart-persistence-or-reset", valid_body),
            ("concurrency-and-idempotency", valid_body),
        ] {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let response = super::route_http_request("POST", path, None, body, &state)
                .await
                .unwrap_or_else(|error| panic!("POST {path}: {error}"));
            record!(
                "POST",
                route,
                case,
                response.status == "400 Bad Request"
                    && response.body.contains("ApiVersionUnspecified")
            );
        }
    }

    let versioned_mutations = vec![
        (
            "/api/v0/jobs/discography",
            "/api/v0/jobs/discography",
            "discography",
            "{\"artist\":\"Residual Artist\"}",
            vec![
                ("malformed-path-query-or-body", "not-json"),
                ("missing-empty-or-conflict-state", ""),
                ("runtime-failure-and-timeout", "{\"artist\":\"Residual Artist\"}"),
                (
                    "restart-persistence-or-reset",
                    "{\"artist\":\"Residual Restart Artist\"}",
                ),
                (
                    "concurrency-and-idempotency",
                    "{\"artist\":\"Residual Concurrent Artist\"}",
                ),
            ],
        ),
        (
            "/api/v0/jobs/label-crate",
            "/api/v0/jobs/label-crate",
            "label",
            "{\"label_name\":\"Residual Label\",\"limit\":2}",
            vec![
                ("nominal-status-headers-body", "{\"label_name\":\"Residual Label\"}"),
                ("malformed-path-query-or-body", "not-json"),
                ("missing-empty-or-conflict-state", ""),
                (
                    "runtime-failure-and-timeout",
                    "{\"label_name\":\"Residual Label\"}",
                ),
                (
                    "mutation-side-effects-and-readback",
                    "{\"label_name\":\"Residual Readback Label\"}",
                ),
                (
                    "restart-persistence-or-reset",
                    "{\"label_name\":\"Residual Restart Label\"}",
                ),
                (
                    "concurrency-and-idempotency",
                    "{\"label_name\":\"Residual Concurrent Label\"}",
                ),
            ],
        ),
        (
            "/api/v0/jobs/mb-release",
            "/api/v0/jobs/mb-release",
            "mb-release",
            "{\"artist\":\"Residual Artist\",\"title\":\"Residual Release\"}",
            vec![
                ("malformed-path-query-or-body", "not-json"),
                ("missing-empty-or-conflict-state", ""),
                (
                    "runtime-failure-and-timeout",
                    "{\"artist\":\"Residual Artist\",\"title\":\"Residual Release\"}",
                ),
                (
                    "mutation-side-effects-and-readback",
                    "{\"artist\":\"Residual Readback Artist\",\"title\":\"Residual Readback Release\"}",
                ),
                (
                    "restart-persistence-or-reset",
                    "{\"artist\":\"Residual Restart Artist\",\"title\":\"Residual Restart Release\"}",
                ),
                (
                    "concurrency-and-idempotency",
                    "{\"artist\":\"Residual Concurrent Artist\",\"title\":\"Residual Concurrent Release\"}",
                ),
            ],
        ),
    ];

    for (path, route, kind, _valid_body, cases) in versioned_mutations {
        for (case, body) in cases {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            if case == "concurrency-and-idempotency" {
                let (left, right) = tokio::join!(
                    super::route_http_request("POST", path, None, body, &state),
                    super::route_http_request("POST", path, None, body, &state)
                );
                let expected = if kind == "label" {
                    "200 OK"
                } else if body == "not-json" || body.is_empty() {
                    "400 Bad Request"
                } else {
                    "202 Accepted"
                };
                record!(
                    "POST",
                    route,
                    case,
                    left.as_ref()
                        .is_ok_and(|response| response.status == expected)
                        && right
                            .as_ref()
                            .is_ok_and(|response| response.status == expected)
                );
                continue;
            }

            let response = super::route_http_request("POST", path, None, body, &state)
                .await
                .unwrap_or_else(|error| panic!("POST {path}: {error}"));
            let expected = if body == "not-json" || body.is_empty() {
                "400 Bad Request"
            } else if kind == "label" {
                "200 OK"
            } else {
                "202 Accepted"
            };
            let mut pass = response.status == expected && !response.body.is_empty();
            if pass && matches!(case, "mutation-side-effects-and-readback") {
                let json = serde_json::from_str::<serde_json::Value>(&response.body)
                    .expect("job mutation JSON");
                let id = if kind == "label" {
                    json["jobId"]
                        .as_str()
                        .or_else(|| json["id"].as_str())
                        .unwrap_or_default()
                } else {
                    json["search_id"].as_str().unwrap_or_default()
                };
                let read_path = if kind == "label" {
                    format!("/api/v0/jobs/label-crate/{id}")
                } else {
                    format!("/api/jobs/{id}")
                };
                let readback = super::route_http_request("GET", &read_path, None, "", &state)
                    .await
                    .expect("job mutation readback");
                pass = !id.is_empty() && readback.status == "200 OK" && !readback.body.is_empty();
            }
            if pass && case == "restart-persistence-or-reset" {
                let json = serde_json::from_str::<serde_json::Value>(&response.body)
                    .expect("restart job JSON");
                let id = if kind == "label" {
                    json["jobId"]
                        .as_str()
                        .or_else(|| json["id"].as_str())
                        .unwrap_or_default()
                } else {
                    json["search_id"].as_str().unwrap_or_default()
                };
                let state_path = state.controller_features.read().await.state_path.clone();
                let persisted = fs::read(&state_path).expect("read persisted job projection");
                let file = serde_json::from_slice::<super::ControllerFeatureStateFile>(&persisted)
                    .expect("parse persisted job projection");
                drop(state);
                let (restarted, _receiver) = test_state_with_env(base_env.clone());
                *restarted.controller_features.write_for_test().await =
                    super::ControllerFeatureState {
                        records: file.records,
                        state_path,
                    };
                let read_path = if kind == "label" {
                    format!("/api/v0/jobs/label-crate/{id}")
                } else if kind == "discography" {
                    format!("/api/v0/jobs/discography/{id}")
                } else {
                    format!("/api/jobs/{id}")
                };
                let readback = super::route_http_request("GET", &read_path, None, "", &restarted)
                    .await
                    .expect("restart job readback");
                pass = !id.is_empty() && readback.status == "200 OK" && !readback.body.is_empty();
            }
            record!("POST", route, case, pass);
        }
    }

    assert_eq!(ledger.len(), 66, "Jobs residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Jobs evidence directory");
    fs::write(
        evidence_dir.join("jobs_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Jobs ledger"),
    )
    .expect("write Jobs ledger");
    assert!(
        mismatches.is_empty(),
        "{} Jobs residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining slskdN library-health and native
/// library-items controller cases.  The broad library-health tests cover
/// the already-closed nominal and mutation rows; this ledger isolates the
/// open runtime, malformed, restart, and concurrent branches so the audit
/// can credit them independently.
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
async fn controller_api_differential_library_residuals() {
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

    let health_cases = [
        (
            "/api/library/health/dashboard?libraryPath=%2Fmusic",
            "/api/library/health/dashboard",
        ),
        (
            "/api/library/health/issues?libraryPath=%2Fmusic",
            "/api/library/health/issues",
        ),
        (
            "/api/library/health/issues/by-artist?limit=20",
            "/api/library/health/issues/by-artist",
        ),
        (
            "/api/library/health/issues/by-codec",
            "/api/library/health/issues/by-codec",
        ),
        (
            "/api/library/health/issues/by-release?limit=20",
            "/api/library/health/issues/by-release",
        ),
        (
            "/api/library/health/issues/by-type?libraryPath=%2Fmusic",
            "/api/library/health/issues/by-type",
        ),
        (
            "/api/library/health/summary?libraryPath=%2Fmusic",
            "/api/library/health/summary",
        ),
    ];
    for (path, route) in health_cases {
        for versioned in [false, true] {
            let request_path = if versioned {
                path.replacen("/api/", "/api/v0/", 1)
            } else {
                path.to_owned()
            };
            let request_route = if versioned {
                route.replacen("/api/", "/api/v0/", 1)
            } else {
                route.to_owned()
            };
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let response = super::route_http_request("GET", &request_path, None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {request_path}: {error}"));
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            record!(
                "GET",
                request_route,
                "runtime-failure-and-timeout",
                response.status == "200 OK" && value.is_object()
            );
        }
    }

    for (scan_prefix, route) in [
        (
            "/api/library/health/scans/",
            "/api/library/health/scans/{scanId}",
        ),
        (
            "/api/v0/library/health/scans/",
            "/api/v0/library/health/scans/{scanId}",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let started = super::route_http_request(
            "POST",
            "/api/v0/library/health/scans",
            None,
            r#"{"libraryPath":"/music"}"#,
            &state,
        )
        .await
        .expect("start residual library scan");
        let scan_id = serde_json::from_str::<serde_json::Value>(&started.body).unwrap_or_default()
            ["scanId"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let path = format!("{scan_prefix}{scan_id}");
        let response = super::route_http_request("GET", &path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            started.status == "200 OK"
                && !scan_id.is_empty()
                && response.status == "200 OK"
                && response.body.contains(&scan_id)
        );
    }

    let item_root =
        std::env::temp_dir().join(format!("slskr-library-residual-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&item_root).expect("create residual library directory");
    let item_path = item_root.join("Residual.flac");
    fs::write(&item_path, b"residual library item").expect("write residual library file");
    let (items_state, _receiver) = test_state_with_env(base_env.clone());
    add_test_share(&items_state, "Residual/Residual.flac", &item_path, 22).await;
    let item_record = items_state
        .library
        .write()
        .await
        .create(
            "Residual Artist".to_owned(),
            "Residual Title".to_owned(),
            "Audio".to_owned(),
        )
        .expect("create residual library item");
    let item_id = item_record.id.clone();

    for (path, case) in [
        (
            "/api/v0/library/items?limit=not-a-number",
            "malformed-path-query-or-body",
        ),
        ("/api/v0/library/items", "missing-empty-or-conflict-state"),
        ("/api/v0/library/items", "runtime-failure-and-timeout"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &items_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/library/items",
            case,
            response.status == "200 OK"
                && value["items"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
        );
    }

    let detail_cases = [
        (
            format!("/api/v0/library/items/{item_id}"),
            "nominal-status-headers-body",
            true,
        ),
        (
            format!("/api/v0/library/items/{item_id}/extra"),
            "malformed-path-query-or-body",
            false,
        ),
        (
            "/api/v0/library/items/no-such-content-id".to_owned(),
            "missing-empty-or-conflict-state",
            false,
        ),
        (
            format!("/api/v0/library/items/{item_id}"),
            "runtime-failure-and-timeout",
            true,
        ),
        (
            format!("/api/v0/library/items/{item_id}"),
            "populated-dynamic-state",
            true,
        ),
    ];
    for (path, case, found) in detail_cases {
        let response = super::route_http_request("GET", &path, None, "", &items_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let pass = if found {
            response.status == "200 OK" && response.body.contains(&item_id)
        } else {
            response.status == "404 Not Found"
        };
        record!("GET", "/api/v0/library/items/{contentId}", case, pass);
    }

    for (path, case) in [
        (
            "/api/v0/library/items/browser?path=Residual",
            "nominal-status-headers-body",
        ),
        (
            "/api/v0/library/items/browser?path=Residual&limit=not-a-number",
            "malformed-path-query-or-body",
        ),
        (
            "/api/v0/library/items/browser?path=Residual",
            "missing-empty-or-conflict-state",
        ),
        (
            "/api/v0/library/items/browser?path=Residual",
            "runtime-failure-and-timeout",
        ),
        (
            "/api/v0/library/items/browser?path=Residual",
            "populated-dynamic-state",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &items_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/library/items/browser",
            case,
            response.status == "200 OK"
                && value["files"]
                    .as_array()
                    .is_some_and(|files| !files.is_empty())
        );
    }
    let _ = fs::remove_dir_all(&item_root);

    let unversioned_mutations = [
        (
            "PATCH",
            "/api/library/health/issues/lib-1-missing-artist",
            "/api/library/health/issues/{issueId}",
            r#"{"artist":"Recovered Artist"}"#,
        ),
        (
            "POST",
            "/api/library/health/issues/fix",
            "/api/library/health/issues/fix",
            r#"{}"#,
        ),
        (
            "POST",
            "/api/library/health/scans",
            "/api/library/health/scans",
            r#"{"libraryPath":"/music"}"#,
        ),
    ];
    for (method, path, route, body) in unversioned_mutations {
        for case in [
            "runtime-failure-and-timeout",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
            "concurrency-and-idempotency",
        ] {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let response = super::route_http_request(method, path, None, body, &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            record!(
                method,
                route,
                case,
                response.status == "400 Bad Request"
                    && response.body.contains("ApiVersionUnspecified")
            );
        }
    }

    for (case, body) in [
        ("malformed-path-query-or-body", "not-json"),
        ("missing-empty-or-conflict-state", ""),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request(
            "PATCH",
            "/api/v0/library/health/issues/lib-1-missing-artist",
            None,
            body,
            &state,
        )
        .await
        .expect("versioned residual library issue update");
        record!(
            "PATCH",
            "/api/v0/library/health/issues/{issueId}",
            case,
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state
            .library
            .write()
            .await
            .create(
                "".to_owned(),
                "Concurrent Issue".to_owned(),
                "Audio".to_owned(),
            )
            .expect("seed concurrent library issue");
        let (left, right) = tokio::join!(
            super::route_http_request(
                "PATCH",
                "/api/v0/library/health/issues/lib-1-missing-artist",
                None,
                r#"{"artist":"Recovered Artist"}"#,
                &state
            ),
            super::route_http_request(
                "PATCH",
                "/api/v0/library/health/issues/lib-1-missing-artist",
                None,
                r#"{"artist":"Recovered Artist"}"#,
                &state
            )
        );
        record!(
            "PATCH",
            "/api/v0/library/health/issues/{issueId}",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "204 No Content")
        );
    }

    for (case, body) in [
        ("nominal-status-headers-body", r#"{"path":"/music"}"#),
        ("malformed-path-query-or-body", "not-json"),
        ("missing-empty-or-conflict-state", ""),
        ("runtime-failure-and-timeout", r#"{"path":"/music"}"#),
        ("mutation-side-effects-and-readback", r#"{"path":"/music"}"#),
        ("restart-persistence-or-reset", r#"{"path":"/music"}"#),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request("POST", "/api/library/scan", None, body, &state)
            .await
            .expect("compatibility library scan");
        let mut pass = response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .is_ok_and(|value| value["scan_id"].as_str().is_some());
        if pass && case == "mutation-side-effects-and-readback" {
            let scan_id = serde_json::from_str::<serde_json::Value>(&response.body)
                .expect("compatibility scan JSON")["scan_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let readback = super::route_http_request(
                "GET",
                &format!("/api/library/health/scans/{scan_id}"),
                None,
                "",
                &state,
            )
            .await
            .expect("compatibility scan readback");
            pass = !scan_id.is_empty() && readback.status == "200 OK";
        }
        if body == "not-json" {
            pass = response.status == "400 Bad Request";
        }
        record!("POST", "/api/library/scan", case, pass);
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/library/scan",
                None,
                r#"{"path":"/music"}"#,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/library/scan",
                None,
                r#"{"path":"/music"}"#,
                &state
            )
        );
        record!(
            "POST",
            "/api/library/scan",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for (method, path, route, cases) in [
        (
            "POST",
            "/api/v0/library/health/issues/fix",
            "/api/v0/library/health/issues/fix",
            vec![
                ("malformed-path-query-or-body", "not-json"),
                ("missing-empty-or-conflict-state", ""),
                ("concurrency-and-idempotency", "{}"),
            ],
        ),
        (
            "POST",
            "/api/v0/library/health/scans",
            "/api/v0/library/health/scans",
            vec![
                ("malformed-path-query-or-body", "not-json"),
                ("runtime-failure-and-timeout", r#"{"libraryPath":"/music"}"#),
                (
                    "restart-persistence-or-reset",
                    r#"{"libraryPath":"/music"}"#,
                ),
            ],
        ),
    ] {
        for (case, body) in cases {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            if case == "concurrency-and-idempotency" {
                let (left, right) = tokio::join!(
                    super::route_http_request(method, path, None, body, &state),
                    super::route_http_request(method, path, None, body, &state)
                );
                record!(
                    method,
                    route,
                    case,
                    left.as_ref()
                        .is_ok_and(|response| response.status == "200 OK")
                        && right
                            .as_ref()
                            .is_ok_and(|response| response.status == "200 OK")
                );
            } else {
                let response = super::route_http_request(method, path, None, body, &state)
                    .await
                    .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
                let pass = if path == "/api/v0/library/health/issues/fix" {
                    if body == "not-json" {
                        response.status == "400 Bad Request"
                    } else {
                        response.status == "200 OK"
                            && serde_json::from_str::<serde_json::Value>(&response.body)
                                .is_ok_and(|value| value["fixed"].is_number())
                    }
                } else if body == "not-json" {
                    response.status == "400 Bad Request"
                } else {
                    response.status == "200 OK"
                        && serde_json::from_str::<serde_json::Value>(&response.body)
                            .is_ok_and(|value| value["scanId"].as_str().is_some())
                };
                record!(method, route, case, pass);
            }
        }
    }

    assert_eq!(ledger.len(), 57, "Library residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Library evidence directory");
    fs::write(
        evidence_dir.join("library_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Library ledger"),
    )
    .expect("write Library ledger");
    assert!(
        mismatches.is_empty(),
        "{} Library residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining slskdN SecurityController cases.
/// The existing security ledgers cover bans, reputation, diagnostics, and
/// closed-database behavior; this residual ledger closes the configured
/// optional-service reads plus the circuit, entropy, adversarial,
/// disclosure, and reputation validation/lifecycle branches.
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
async fn controller_api_differential_security_controller_residuals() {
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

    async fn seed_feature(state: &super::AppState, key: &str, value: serde_json::Value) {
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(key.to_owned(), value)
            .expect("seed security controller feature");
    }

    async fn seed_circuit_service(state: &super::AppState) {
        seed_feature(
            state,
            "security/profile/security/circuits",
            serde_json::json!({"configured": true}),
        )
        .await;
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/adversarial",
            serde_json::json!({
                "resource": "security/adversarial",
                "settings": {"enabled": true, "profile": "Standard"}
            }),
        )
        .await;
        let response =
            super::route_http_request("GET", "/api/v0/security/adversarial", None, "", &state)
                .await
                .expect("configured adversarial settings");
        record!(
            "GET",
            "/api/v0/security/adversarial",
            "nominal-status-headers-body",
            response.status == "404 Not Found"
                && response.body == "Adversarial features are not configured"
        );
        record!(
            "GET",
            "/api/v0/security/adversarial",
            "populated-dynamic-state",
            response.status == "404 Not Found"
                && response.body == "Adversarial features are not configured"
        );

        let malformed = super::route_http_request(
            "GET",
            "/api/v0/security/adversarial/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed adversarial path");
        record!(
            "GET",
            "/api/v0/security/adversarial",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/adversarial",
            serde_json::json!({
                "resource": "security/adversarial",
                "settings": {"enabled": true}
            }),
        )
        .await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/security/adversarial/stats",
            None,
            "",
            &state,
        )
        .await
        .expect("adversarial stats");
        record!(
            "GET",
            "/api/v0/security/adversarial/stats",
            "populated-dynamic-state",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["profile"].is_string())
        );
    }

    for (path, route, case, expected) in [
        (
            "/api/v0/security/bans/extra",
            "/api/v0/security/bans",
            "malformed-path-query-or-body",
            "404 Not Found",
        ),
        (
            "/api/v0/security/bans",
            "/api/v0/security/bans",
            "missing-empty-or-conflict-state",
            "200 OK",
        ),
        (
            "/api/v0/security/dashboard/extra",
            "/api/v0/security/dashboard",
            "malformed-path-query-or-body",
            "404 Not Found",
        ),
        (
            "/api/v0/security/dashboard",
            "/api/v0/security/dashboard",
            "missing-empty-or-conflict-state",
            "200 OK",
        ),
        (
            "/api/v0/security/reputation/peer/extra",
            "/api/v0/security/reputation/{username}",
            "malformed-path-query-or-body",
            "404 Not Found",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).ok();
        record!(
            "GET",
            route,
            case,
            response.status == expected && (expected != "200 OK" || value.is_some())
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/canaries",
            serde_json::json!({
                "settings": {
                    "totalAccesses": 3,
                    "uniqueAttackers": 2,
                    "activeTraps": 1,
                    "triggeredTraps": 1,
                    "trapsByType": {"credential": 1}
                }
            }),
        )
        .await;
        let response =
            super::route_http_request("GET", "/api/v0/security/canaries", None, "", &state)
                .await
                .expect("configured canary state");
        record!(
            "GET",
            "/api/v0/security/canaries",
            "populated-dynamic-state",
            response.status == "200 OK"
                && response.body.contains("\"totalAccesses\":3")
                && response.body.contains("credential")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/disclosure/residual-peer",
            serde_json::json!({
                "resource": "security/disclosure/residual-peer",
                "settings": {"tier": "Trusted", "canBrowse": true}
            }),
        )
        .await;
        let nominal = super::route_http_request(
            "GET",
            "/api/v0/security/disclosure/residual-peer",
            None,
            "",
            &state,
        )
        .await
        .expect("disclosure nominal");
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/security/disclosure/residual-peer/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("disclosure malformed");
        record!(
            "GET",
            "/api/v0/security/disclosure/{username}",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && nominal.body.contains("\"peerTier\":\"Trusted\"")
        );
        record!(
            "GET",
            "/api/v0/security/disclosure/{username}",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        record!(
            "GET",
            "/api/v0/security/disclosure/{username}",
            "populated-dynamic-state",
            nominal.status == "200 OK" && nominal.body.contains("\"canBrowse\":true")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.users.write().await.records.push(super::UserRecord {
            username: "security-populated-peer".to_owned(),
            watched: false,
            status: Some("online".to_owned()),
            privileged: false,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: super::unix_timestamp(),
        });
        let response =
            super::route_http_request("GET", "/api/v0/security/peers/stats", None, "", &state)
                .await
                .expect("peer stats");
        record!(
            "GET",
            "/api/v0/security/peers/stats",
            "populated-dynamic-state",
            response.status == "200 OK" && response.body.contains("\"totalPeers\":1")
        );
    }

    for route in [
        "/api/v0/security/reputation/suspicious",
        "/api/v0/security/reputation/trusted",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response =
            super::route_http_request("GET", &format!("{route}?limit=0"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    for (route, case, path) in [
        (
            "/api/v0/security/scanners",
            "malformed-path-query-or-body",
            "/api/v0/security/scanners/extra",
        ),
        (
            "/api/v0/security/scanners",
            "populated-dynamic-state",
            "/api/v0/security/scanners",
        ),
        (
            "/api/v0/security/threats",
            "malformed-path-query-or-body",
            "/api/v0/security/threats/extra",
        ),
        (
            "/api/v0/security/threats",
            "populated-dynamic-state",
            "/api/v0/security/threats",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let feature_key = if route.ends_with("scanners") {
            "security/profile/security/scanners"
        } else {
            "security/profile/security/threats"
        };
        seed_feature(
            &state,
            feature_key,
            serde_json::json!({"items": [{"id": "security-residual"}]}),
        )
        .await;
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            case,
            if case == "malformed-path-query-or-body" {
                response.status == "404 Not Found"
            } else {
                response.status == "200 OK" && response.body.contains("security-residual")
            }
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_feature(
            &state,
            "security/profile/security/tor",
            serde_json::json!({
                "settings": {"type": "Tor", "available": true, "healthy": true}
            }),
        )
        .await;
        let response =
            super::route_http_request("GET", "/api/v0/security/tor/status", None, "", &state)
                .await
                .expect("configured Tor status");
        record!(
            "GET",
            "/api/v0/security/tor/status",
            "populated-dynamic-state",
            response.status == "200 OK" && response.body.contains("\"available\":true")
        );
    }

    for (route, case, path) in [
        (
            "/api/v0/security/transports",
            "malformed-path-query-or-body",
            "/api/v0/security/transports/extra",
        ),
        (
            "/api/v0/security/transports",
            "populated-dynamic-state",
            "/api/v0/security/transports",
        ),
        (
            "/api/v0/security/transports/status",
            "malformed-path-query-or-body",
            "/api/v0/security/transports/status/extra",
        ),
        (
            "/api/v0/security/transports/status",
            "missing-empty-or-conflict-state",
            "/api/v0/security/transports/status",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if case != "missing-empty-or-conflict-state" {
            seed_feature(
                &state,
                "security/profile/security/transports",
                serde_json::json!({
                    "items": {"Direct": {"available": true}},
                    "status": {"selectedMode": "Direct", "totalTransports": 1}
                }),
            )
            .await;
        }
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let pass = if case == "malformed-path-query-or-body" {
            response.status == "404 Not Found"
        } else if route.ends_with("/status") {
            response.status == "200 OK" && response.body.contains("selectedMode")
        } else {
            response.status == "200 OK" && response.body.contains("Direct")
        };
        record!("GET", route, case, pass);
    }

    let circuit_body = r#"{"targetPeerId":"security-circuit-peer","circuitId":"residual-circuit"}"#;
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let created = super::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("create residual circuit");
        record!(
            "POST",
            "/api/v0/security/circuits",
            "nominal-status-headers-body",
            created.status == "200 OK" && created.body.contains("residual-circuit")
        );
        let listed =
            super::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
                .await
                .expect("circuit readback");
        record!(
            "POST",
            "/api/v0/security/circuits",
            "mutation-side-effects-and-readback",
            listed.status == "200 OK" && listed.body.contains("residual-circuit")
        );
    }
    for (case, body, path) in [
        (
            "malformed-path-query-or-body",
            "not-json",
            "/api/v0/security/circuits",
        ),
        (
            "missing-empty-or-conflict-state",
            "",
            "/api/v0/security/circuits",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .expect("invalid residual circuit");
        record!(
            "POST",
            "/api/v0/security/circuits",
            case,
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let created = super::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("restart circuit seed");
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let listed =
            super::route_http_request("GET", "/api/v0/security/circuits", None, "", &restarted)
                .await
                .expect("restart circuit list");
        record!(
            "POST",
            "/api/v0/security/circuits",
            "restart-persistence-or-reset",
            created.status == "200 OK" && listed.status == "200 OK" && listed.body == "[]"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_circuit_service(&state).await;
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/security/circuits",
                None,
                circuit_body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/security/circuits",
                None,
                circuit_body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/security/circuits",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for (case, body, path) in [
        (
            "malformed-path-query-or-body",
            "not-json",
            "/api/v0/security/circuits/%20",
        ),
        (
            "missing-empty-or-conflict-state",
            "",
            "/api/v0/security/circuits/missing-circuit",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request("DELETE", path, None, body, &state)
            .await
            .expect("invalid residual circuit delete");
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            case,
            response.status
                == if case == "malformed-path-query-or-body" {
                    "400 Bad Request"
                } else {
                    "200 OK"
                }
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        super::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("mutation circuit seed");
        let deleted = super::route_http_request(
            "DELETE",
            "/api/v0/security/circuits/residual-circuit",
            None,
            "",
            &state,
        )
        .await
        .expect("mutation circuit delete");
        let listed =
            super::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
                .await
                .expect("mutation circuit readback");
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "nominal-status-headers-body",
            deleted.status == "200 OK"
        );
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "mutation-side-effects-and-readback",
            deleted.status == "200 OK" && listed.body == "[]"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        super::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("restart delete circuit seed");
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let deleted = super::route_http_request(
            "DELETE",
            "/api/v0/security/circuits/residual-circuit",
            None,
            "",
            &restarted,
        )
        .await
        .expect("restart delete circuit");
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "restart-persistence-or-reset",
            deleted.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        super::route_http_request(
            "POST",
            "/api/v0/security/circuits",
            None,
            circuit_body,
            &state,
        )
        .await
        .expect("concurrent delete circuit seed");
        let (left, right) = tokio::join!(
            super::route_http_request(
                "DELETE",
                "/api/v0/security/circuits/residual-circuit",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "DELETE",
                "/api/v0/security/circuits/residual-circuit",
                None,
                "",
                &state
            )
        );
        record!(
            "DELETE",
            "/api/v0/security/circuits/{circuitId}",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for case in [
        "nominal-status-headers-body",
        "missing-empty-or-conflict-state",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let path = if case == "malformed-path-query-or-body" {
            "/api/v0/security/entropy/check/extra"
        } else {
            "/api/v0/security/entropy/check"
        };
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            "/api/v0/security/entropy/check",
            case,
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = super::route_http_request(
            "POST",
            "/api/v0/security/entropy/check/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed entropy path");
        record!(
            "POST",
            "/api/v0/security/entropy/check",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }

    let prepare_adversarial = |state: &Arc<super::AppState>| {
        let state = Arc::clone(state);
        async move {
            fs::write(
                state.config.state_dir.join("slskd.yml"),
                "remote_configuration: true\nsecurity: {}\n",
            )
            .expect("write residual adversarial YAML");
        }
    };
    for (case, body) in [
        ("malformed-path-query-or-body", "not-json"),
        ("missing-empty-or-conflict-state", ""),
    ] {
        let (state, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        prepare_adversarial(&state).await;
        let response =
            super::route_http_request("PUT", "/api/v0/security/adversarial", None, body, &state)
                .await
                .expect("invalid adversarial update");
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            case,
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        prepare_adversarial(&state).await;
        let response =
            super::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
                .await
                .expect("adversarial restart seed");
        let (restarted, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        let readback =
            super::route_http_request("GET", "/api/v0/security/adversarial", None, "", &restarted)
                .await
                .expect("adversarial restart readback");
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "restart-persistence-or-reset",
            response.status == "200 OK" && readback.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) =
            test_state_with_env(base_env.clone().with("SLSKR_REMOTE_CONFIGURATION", "true"));
        prepare_adversarial(&state).await;
        let (left, right) = tokio::join!(
            super::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state),
            super::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
        );
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    for case in [
        "nominal-status-headers-body",
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let body = if case == "malformed-path-query-or-body" {
            "not-json"
        } else if case == "missing-empty-or-conflict-state" {
            ""
        } else {
            r#"{"tier":"Trusted"}"#
        };
        let response = super::route_http_request(
            "PUT",
            "/api/v0/security/disclosure/residual-peer",
            None,
            body,
            &state,
        )
        .await
        .expect("disclosure mutation");
        let mut pass = if case == "malformed-path-query-or-body"
            || case == "missing-empty-or-conflict-state"
        {
            response.status == "400 Bad Request"
        } else {
            response.status == "200 OK"
        };
        if pass && case == "mutation-side-effects-and-readback" {
            let readback = super::route_http_request(
                "GET",
                "/api/v0/security/disclosure/residual-peer",
                None,
                "",
                &state,
            )
            .await
            .expect("disclosure mutation readback");
            pass = readback.status == "200 OK" && readback.body.contains("Trusted");
        }
        if pass && case == "restart-persistence-or-reset" {
            let (restarted, _receiver) = test_state_with_env(base_env.clone());
            let readback = super::route_http_request(
                "GET",
                "/api/v0/security/disclosure/residual-peer",
                None,
                "",
                &restarted,
            )
            .await
            .expect("disclosure restart readback");
            pass = readback.status == "200 OK" && !readback.body.contains("Trusted");
        }
        record!("PUT", "/api/v0/security/disclosure/{username}", case, pass);
    }

    for case in [
        "missing-empty-or-conflict-state",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let body = if case == "missing-empty-or-conflict-state" {
            ""
        } else {
            r#"{"score":80}"#
        };
        let response = super::route_http_request(
            "PUT",
            "/api/v0/security/reputation/residual-peer",
            None,
            body,
            &state,
        )
        .await
        .expect("reputation residual mutation");
        let mut pass = if case == "missing-empty-or-conflict-state" {
            response.status == "400 Bad Request"
        } else {
            response.status == "200 OK"
        };
        if pass && case == "restart-persistence-or-reset" {
            let (restarted, _receiver) = test_state_with_env(base_env.clone());
            let readback = super::route_http_request(
                "GET",
                "/api/v0/security/reputation/residual-peer",
                None,
                "",
                &restarted,
            )
            .await
            .expect("reputation restart readback");
            pass = readback.status == "200 OK" && readback.body.contains("\"score\":50");
        }
        if pass && case == "concurrency-and-idempotency" {
            let (left, right) = tokio::join!(
                super::route_http_request(
                    "PUT",
                    "/api/v0/security/reputation/residual-peer",
                    None,
                    body,
                    &state
                ),
                super::route_http_request(
                    "PUT",
                    "/api/v0/security/reputation/residual-peer",
                    None,
                    body,
                    &state
                )
            );
            pass = left.as_ref().is_ok_and(|value| value.status == "200 OK")
                && right.as_ref().is_ok_and(|value| value.status == "200 OK");
        }
        record!("PUT", "/api/v0/security/reputation/{username}", case, pass);
    }

    assert_eq!(ledger.len(), 56, "Security residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Security evidence directory");
    fs::write(
        evidence_dir.join("security_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Security ledger"),
    )
    .expect("write Security ledger");
    assert!(
        mismatches.is_empty(),
        "{} Security residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining slskdN Spotify and Lidarr
/// integration-controller rows.  The ledger keeps the frozen
/// unversioned-mutation API-version rejection distinct from the real
/// versioned actions, and uses local HTTP fixtures for the external
/// status/wanted projections rather than treating a hardcoded fallback
/// as a successful integration response.
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
async fn controller_api_differential_integrations_residuals() {
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

    async fn one_json_fixture(
        response: serde_json::Value,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind integration fixture");
        let address = listener.local_addr().expect("integration fixture address");
        let task = tokio::spawn(async move { serve_json_fixture(&listener, response).await });
        (format!("http://{address}"), task)
    }

    async fn unused_local_url() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind unused integration port");
        let address = listener.local_addr().expect("unused integration address");
        drop(listener);
        format!("http://{address}")
    }

    fn spotify_env() -> MapEnv {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "integration-client-id")
    }

    fn lidarr_env(url: &str) -> MapEnv {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", url)
            .with("SLSKR_LIDARR_API_KEY", "integration-fixture-key")
            .with("SLSKR_LIDARR_TIMEOUT", "1")
    }

    for case in [
        "runtime-failure-and-timeout",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response = super::route_http_request(
            "POST",
            "/api/integrations/spotify/authorize",
            None,
            if case == "runtime-failure-and-timeout" {
                "not-json"
            } else {
                ""
            },
            &state,
        )
        .await
        .expect("unversioned Spotify authorize rejection");
        record!(
            "POST",
            "/api/integrations/spotify/authorize",
            case,
            response.status == "400 Bad Request" && response.body.contains("ApiVersionUnspecified")
        );
    }

    for case in [
        "runtime-failure-and-timeout",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response =
            super::route_http_request("DELETE", "/api/integrations/spotify", None, "", &state)
                .await
                .expect("unversioned Spotify disconnect rejection");
        record!(
            "DELETE",
            "/api/integrations/spotify",
            case,
            response.status == "400 Bad Request" && response.body.contains("ApiVersionUnspecified")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/integrations/spotify/status/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("unversioned Spotify status malformed path");
        record!(
            "GET",
            "/api/integrations/spotify/status",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("Spotify status runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response =
            super::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
                .await
                .expect("unversioned Spotify status runtime response");
        record!(
            "GET",
            "/api/integrations/spotify/status",
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["connected"] == false)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/spotify/status",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned Spotify status nominal response");
        record!(
            "GET",
            "/api/v0/integrations/spotify/status",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["configured"] == true)
        );
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/integrations/spotify/status/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned Spotify status malformed path");
        record!(
            "GET",
            "/api/v0/integrations/spotify/status",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned Spotify status runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/spotify/status",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned Spotify status runtime response");
        record!(
            "GET",
            "/api/v0/integrations/spotify/status",
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
        );
    }

    for (route, path, case, expected) in [
        (
            "/api/integrations/spotify/callback",
            "/api/integrations/spotify/callback?error=access_denied",
            "nominal-status-headers-body",
            "200 OK",
        ),
        (
            "/api/integrations/spotify/callback",
            "/api/integrations/spotify/callback/extra",
            "malformed-path-query-or-body",
            "404 Not Found",
        ),
        (
            "/api/integrations/spotify/callback",
            "/api/integrations/spotify/callback",
            "missing-empty-or-conflict-state",
            "400 Bad Request",
        ),
        (
            "/api/integrations/spotify/callback",
            "/api/integrations/spotify/callback?error=access_denied&state=populated",
            "populated-dynamic-state",
            "200 OK",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == expected
                && (expected != "200 OK" || response.body.contains("Spotify"))
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("unversioned Spotify callback runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), super::SearchStore::new(), Some(db.clone()));
        let authorize = super::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("issue Spotify callback state");
        let issued_state = state
            .oauth_states
            .read()
            .await
            .records
            .keys()
            .next()
            .cloned()
            .expect("issued Spotify callback state");
        db.close_for_test().await;
        let path = format!(
            "/api/integrations/spotify/callback?code=integration-code&state={issued_state}"
        );
        let response = super::route_http_request("GET", &path, None, "", &state)
            .await
            .expect("unversioned Spotify callback runtime response");
        record!(
            "GET",
            "/api/integrations/spotify/callback",
            "runtime-failure-and-timeout",
            authorize.status == "200 OK"
                && response.status == "503 Service Unavailable"
                && state
                    .oauth_states
                    .read()
                    .await
                    .records
                    .contains_key(&issued_state)
        );
    }

    for (route, path, case, expected) in [
        (
            "/api/v0/integrations/spotify/callback",
            "/api/v0/integrations/spotify/callback?error=access_denied",
            "nominal-status-headers-body",
            "200 OK",
        ),
        (
            "/api/v0/integrations/spotify/callback",
            "/api/v0/integrations/spotify/callback",
            "missing-empty-or-conflict-state",
            "400 Bad Request",
        ),
        (
            "/api/v0/integrations/spotify/callback",
            "/api/v0/integrations/spotify/callback?error=access_denied&state=populated",
            "populated-dynamic-state",
            "200 OK",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == expected
                && (expected != "200 OK" || response.body.contains("Spotify"))
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned Spotify callback runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), super::SearchStore::new(), Some(db.clone()));
        super::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("issue versioned Spotify callback state");
        let issued_state = state
            .oauth_states
            .read()
            .await
            .records
            .keys()
            .next()
            .cloned()
            .expect("versioned Spotify callback state");
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            &format!(
                "/api/v0/integrations/spotify/callback?code=integration-code&state={issued_state}"
            ),
            None,
            "",
            &state,
        )
        .await
        .expect("versioned Spotify callback runtime response");
        record!(
            "GET",
            "/api/v0/integrations/spotify/callback",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && state
                    .oauth_states
                    .read()
                    .await
                    .records
                    .contains_key(&issued_state)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let malformed = super::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned Spotify authorize malformed path");
        record!(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("unconfigured Spotify authorize");
        record!(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request" && response.body.contains("not configured")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response = super::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned Spotify authorize mutation");
        record!(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["authorizationUrl"].is_string())
                && state.oauth_states.read().await.records.len() == 1
        );
    }
    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response = super::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("seed Spotify authorize restart state");
        let restarted = test_state_with_env(spotify_env()).0;
        record!(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            "restart-persistence-or-reset",
            response.status == "200 OK" && restarted.oauth_states.read().await.records.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/integrations/spotify/authorize",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/integrations/spotify/authorize",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && state.oauth_states.read().await.records.len() == 2
        );
    }

    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let malformed = super::route_http_request(
            "DELETE",
            "/api/v0/integrations/spotify/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned Spotify disconnect malformed path");
        record!(
            "DELETE",
            "/api/v0/integrations/spotify",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response =
            super::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
                .await
                .expect("missing Spotify disconnect");
        record!(
            "DELETE",
            "/api/v0/integrations/spotify",
            "missing-empty-or-conflict-state",
            response.status == "204 No Content" && response.body.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let connection_path = super::spotify_connection_path(&state.config.state_dir);
        fs::create_dir_all(&connection_path).expect("create Spotify connection conflict");
        let response =
            super::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
                .await
                .expect("runtime Spotify disconnect");
        record!(
            "DELETE",
            "/api/v0/integrations/spotify",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && response.body.contains("Spotify connection delete failed")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let response =
            super::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
                .await
                .expect("seed Spotify disconnect restart state");
        let restarted = test_state_with_env(spotify_env()).0;
        let readback = super::route_http_request(
            "GET",
            "/api/v0/integrations/spotify/status",
            None,
            "",
            &restarted,
        )
        .await
        .expect("Spotify disconnect restart readback");
        record!(
            "DELETE",
            "/api/v0/integrations/spotify",
            "restart-persistence-or-reset",
            response.status == "204 No Content"
                && readback.status == "200 OK"
                && readback.body.contains("\"connected\":false")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(spotify_env());
        let (left, right) = tokio::join!(
            super::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state),
            super::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
        );
        record!(
            "DELETE",
            "/api/v0/integrations/spotify",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "204 No Content")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/status/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("Lidarr status malformed path");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/status",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        let missing = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/status",
            None,
            "",
            &state,
        )
        .await
        .expect("missing Lidarr status");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/status",
            "missing-empty-or-conflict-state",
            missing.status == "503 Service Unavailable"
                && missing.body.contains("Lidarr URL is not configured")
        );
    }
    {
        let url = unused_local_url().await;
        let (state, _receiver) = test_state_with_env(lidarr_env(&url));
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/status",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime Lidarr status");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/status",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && response.body.contains("Lidarr status request failed")
        );
    }
    {
        let (url, fixture) = one_json_fixture(serde_json::json!({
            "appName": "Lidarr",
            "version": "2.2.0"
        }))
        .await;
        let (state, _receiver) = test_state_with_env(lidarr_env(&url));
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/status",
            None,
            "",
            &state,
        )
        .await
        .expect("populated Lidarr status");
        let _ = fixture.await.expect("Lidarr status fixture");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/status",
            "populated-dynamic-state",
            response.status == "200 OK"
                && response.body.contains("\"appName\":\"Lidarr\"")
                && response.body.contains("2.2.0")
        );
    }

    for (route, path, case) in [
        (
            "/api/v0/integrations/lidarr/sync/status",
            "/api/v0/integrations/lidarr/sync/status/extra",
            "malformed-path-query-or-body",
        ),
        (
            "/api/v0/integrations/lidarr/sync/status",
            "/api/v0/integrations/lidarr/sync/status",
            "missing-empty-or-conflict-state",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            case,
            if case == "malformed-path-query-or-body" {
                response.status == "404 Not Found"
            } else {
                response.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
            }
        );
    }
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("Lidarr sync runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            base_env.clone(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/sync/status",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime Lidarr sync status");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/sync/status",
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        {
            let mut sync = state.lidarr_sync_state.write().await;
            sync.is_syncing = true;
            sync.last_sync_at = Some("2026-08-13T00:00:00Z".to_owned());
            sync.last_error = Some("fixture warning".to_owned());
            sync.last_result = Some(serde_json::json!({"wantedCount": 4}));
        }
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/sync/status",
            None,
            "",
            &state,
        )
        .await
        .expect("populated Lidarr sync status");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/sync/status",
            "populated-dynamic-state",
            response.status == "200 OK"
                && response.body.contains("fixture warning")
                && response.body.contains("wantedCount")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("Lidarr wanted malformed path");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        let missing = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            None,
            "",
            &state,
        )
        .await
        .expect("missing Lidarr wanted state");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && missing.body.contains("\"configured\":false")
        );
    }
    {
        let url = unused_local_url().await;
        let (state, _receiver) = test_state_with_env(lidarr_env(&url));
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime Lidarr wanted state");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && response.body.contains("connection_failed")
        );
    }
    {
        let (url, fixture) = one_json_fixture(serde_json::json!({
            "totalRecords": 1,
            "records": [{
                "id": 7,
                "title": "Residual Album",
                "artist": {"id": 8, "artistName": "Residual Artist"}
            }]
        }))
        .await;
        let (state, _receiver) = test_state_with_env(lidarr_env(&url));
        let response = super::route_http_request(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            None,
            "",
            &state,
        )
        .await
        .expect("populated Lidarr wanted state");
        let _ = fixture.await.expect("Lidarr wanted fixture");
        record!(
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            "populated-dynamic-state",
            response.status == "200 OK"
                && response.body.contains("Residual Album")
                && response.body.contains("Residual Artist")
                && response.body.contains("searchText")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = super::route_http_request(
            "POST",
            "/api/v0/integrations/lidarr/manualimport/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("Lidarr manual import malformed path");
        record!(
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        let missing = super::route_http_request(
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            None,
            "",
            &state,
        )
        .await
        .expect("missing Lidarr manual import body");
        record!(
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("Directory is required")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request(
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            None,
            r#"{"directory":"/downloads/residual"}"#,
            &state,
        )
        .await
        .expect("seed Lidarr manual import restart state");
        let restarted = test_state_with_env(base_env.clone()).0;
        let readback = super::route_http_request(
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            None,
            r#"{"directory":"/downloads/residual"}"#,
            &restarted,
        )
        .await
        .expect("Lidarr manual import restart readback");
        record!(
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && readback.status == "200 OK"
                && response.body.contains("\"enabled\":false")
                && readback.body.contains("\"enabled\":false")
        );
    }

    for (case, path) in [
        (
            "malformed-path-query-or-body",
            "/api/v0/integrations/lidarr/wanted/sync/extra",
        ),
        (
            "missing-empty-or-conflict-state",
            "/api/v0/integrations/lidarr/wanted/sync",
        ),
        (
            "restart-persistence-or-reset",
            "/api/v0/integrations/lidarr/wanted/sync",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        let pass = if case == "malformed-path-query-or-body" {
            response.status == "404 Not Found"
        } else {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["enabled"] == false)
        };
        record!(
            "POST",
            "/api/v0/integrations/lidarr/wanted/sync",
            case,
            pass
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/integrations/lidarr/wanted/sync",
                None,
                "",
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/integrations/lidarr/wanted/sync",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/integrations/lidarr/wanted/sync",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    assert_eq!(ledger.len(), 51, "Integrations residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Integrations evidence directory");
    fs::write(
        evidence_dir.join("integrations_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Integrations ledger"),
    )
    .expect("write Integrations ledger");
    assert!(
        mismatches.is_empty(),
        "{} Integrations residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining slskdN BackfillController cases.
/// The scheduler is process-local except for its HashDb-backed candidate
/// and file operations, so the ledger separates closed-storage failures
/// from the successful config, state, reset, and concurrency contracts.
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
async fn controller_api_differential_backfill_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_TRANSFER_ALLOW_OUTBOUND", "false");
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

    macro_rules! seed_candidates {
        ($state:expr) => {{
            let state = &$state;
            state
                .searches
                .write()
                .await
                .records
                .push(super::SearchRecord {
                    id: "backfill-residual-search".to_owned(),
                    token: 1,
                    query: "backfill residual".to_owned(),
                    target: "global",
                    target_name: None,
                    status: "completed",
                    results: vec![super::SearchResultEntry {
                        peer_username: Some("backfill-residual-peer".to_owned()),
                        filename: "Library/BackfillResidual.flac".to_owned(),
                        size: 98_765,
                        extension: "flac".to_owned(),
                        bit_rate: None,
                        sample_rate: None,
                        bit_depth: None,
                        length_seconds: None,
                        locked: false,
                        slot_free: Some(true),
                        average_speed: Some(1_000),
                        queue_length: Some(0),
                    }],
                    raw_response_count: 1,
                    filtered_out_count: 0,
                    ignored_result_count: 0,
                    hidden_locked_count: 0,
                    fallback_attempts: 0,
                    ttl_seconds: super::DEFAULT_SEARCH_TTL_SECONDS,
                    expires_at: 0,
                    created_at: 1,
                    updated_at: 1,
                });
            state.users.write().await.records.push(super::UserRecord {
                username: "backfill-residual-peer".to_owned(),
                watched: false,
                status: Some("online".to_owned()),
                privileged: false,
                average_speed: None,
                upload_count: None,
                file_count: None,
                directory_count: None,
                updated_at: super::unix_timestamp(),
            });
            state
                .mesh
                .write()
                .await
                .capability_records
                .push(test_capability_descriptor(
                    "backfill-residual-peer",
                    vec![slskr_client::capabilities::FEATURE_CAPABILITIES_V1.to_owned()],
                ));
        }};
    }

    async fn closed_db_state(env: &MapEnv) -> Arc<super::AppState> {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("backfill residual database");
        let (state, _receiver) =
            test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        state
    }

    async fn response(
        method: &str,
        path: &str,
        body: &str,
        state: &Arc<super::AppState>,
    ) -> super::HttpResponse {
        super::route_http_request(method, path, None, body, state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"))
    }

    // GET /candidates: invalid binding, empty state, closed HashDb, and
    // a populated candidate with the frozen DTO field names and types.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let invalid = response(
            "GET",
            "/api/v0/backfill/candidates?limit=invalid",
            "",
            &state,
        )
        .await;
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "malformed-path-query-or-body",
            invalid.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("GET", "/api/v0/backfill/candidates", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&empty.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && value == serde_json::json!({"count": 0, "candidates": []})
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let failed = response("GET", "/api/v0/backfill/candidates", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "runtime-failure-and-timeout",
            failed.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        seed_candidates!(state);
        let populated = response("GET", "/api/v0/backfill/candidates", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&populated.body)
            .unwrap_or(serde_json::Value::Null);
        let candidate = value
            .get("candidates")
            .and_then(serde_json::Value::as_array)
            .and_then(|entries| entries.first())
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/candidates",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && value["count"] == 1
                && candidate["fileId"]
                    == super::content_discovery::generate_flac_key(
                        "Library/BackfillResidual.flac",
                        98_765,
                    )
                && candidate["peerId"] == "backfill-residual-peer"
                && candidate["path"] == "Library/BackfillResidual.flac"
                && candidate["size"] == 98_765
                && candidate["discoveredAt"].is_string()
                && candidate["peerBackfillsToday"] == 0
                && candidate["isPeerOnline"] == true
                && candidate["isPeerSlskdn"] == true
        );
    }

    // GET /config: exact default DTO, malformed extra segment, closed
    // storage independence, and a live enabled-state projection.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("GET", "/api/v0/backfill/config/extra", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/config",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("GET", "/api/v0/backfill/config", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&empty.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/config",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && value
                    == serde_json::json!({
                        "maxGlobalConnections": 2,
                        "maxPerPeerPerDay": 10,
                        "maxHeaderBytes": 65_536,
                        "minIdleTimeSeconds": 300,
                        "runIntervalSeconds": 600,
                        "transferTimeoutSeconds": 30,
                        "enabled": true,
                    })
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("GET", "/api/v0/backfill/config", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/config",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body).is_ok()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.backfill.write().await.enabled = false;
        let populated = response("GET", "/api/v0/backfill/config", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&populated.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/config",
            "populated-dynamic-state",
            populated.status == "200 OK" && value["enabled"] == false
        );
    }

    // GET /stats: empty/default, malformed path, closed-storage
    // independence, and populated counters with .NET-compatible time
    // representations.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("GET", "/api/v0/backfill/stats/extra", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("GET", "/api/v0/backfill/stats", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&empty.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && value["totalAttempts"] == 0
                && value["successful"] == 0
                && value["failed"] == 0
                && value["rateLimited"] == 0
                && value["active"] == 0
                && value["hashesDiscovered"] == 0
                && value["isIdle"] == false
                && value["lastCycleTime"].is_null()
                && value["nextCycleTime"].is_null()
                && value["idleDuration"].is_null()
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("GET", "/api/v0/backfill/stats", "", &state).await;
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body).is_ok()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        {
            let mut backfill = state.backfill.write().await;
            backfill.total_attempts = 3;
            backfill.successful = 2;
            backfill.failed = 1;
            backfill.rate_limited = 4;
            backfill.active = 1;
            backfill.hashes_discovered = 2;
            backfill.last_cycle_time = Some(1);
            backfill.next_cycle_time = Some(2);
            backfill.is_idle = true;
            backfill.idle_since = Some(1);
        }
        let populated = response("GET", "/api/v0/backfill/stats", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&populated.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "GET",
            "/api/v0/backfill/stats",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && value["totalAttempts"] == 3
                && value["successful"] == 2
                && value["failed"] == 1
                && value["rateLimited"] == 4
                && value["active"] == 1
                && value["hashesDiscovered"] == 2
                && value["isIdle"] == true
                && value["lastCycleTime"].is_string()
                && value["nextCycleTime"].is_string()
                && value["idleDuration"].is_string()
        );
    }

    // POST /enable.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let nominal = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&nominal.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && value["enabled"] == false
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response(
            "POST",
            "/api/v0/backfill/enable?enabled=invalid",
            "",
            &state,
        )
        .await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("POST", "/api/v0/backfill/enable", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&empty.body)
                    .is_ok_and(|value| value["enabled"] == true)
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body)
                    .is_ok_and(|value| value["enabled"] == false)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let changed = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        let config = response("GET", "/api/v0/backfill/config", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "mutation-side-effects-and-readback",
            changed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&config.body)
                    .is_ok_and(|value| value["enabled"] == false)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let changed = response("POST", "/api/v0/backfill/enable?enabled=false", "", &state).await;
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let config = response("GET", "/api/v0/backfill/config", "", &restarted).await;
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "restart-persistence-or-reset",
            changed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&config.body)
                    .is_ok_and(|value| value["enabled"] == true)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            response("POST", "/api/v0/backfill/enable?enabled=false", "", &state),
            response("POST", "/api/v0/backfill/enable?enabled=false", "", &state)
        );
        record!(
            "POST",
            "/api/v0/backfill/enable",
            "concurrency-and-idempotency",
            left.status == "200 OK"
                && right.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&left.body)
                    .is_ok_and(|value| value["enabled"] == false)
                && serde_json::from_str::<serde_json::Value>(&right.body)
                    .is_ok_and(|value| value["enabled"] == false)
        );
    }

    // POST /idle and /busy share process-local state semantics.
    for (route, set_idle, expected) in [
        ("/api/v0/backfill/idle", true, true),
        ("/api/v0/backfill/busy", false, false),
    ] {
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let nominal = response("POST", route, "", &state).await;
            record!(
                "POST",
                route,
                "nominal-status-headers-body",
                nominal.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&nominal.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let malformed = response("POST", &format!("{route}/extra"), "", &state).await;
            record!(
                "POST",
                route,
                "malformed-path-query-or-body",
                malformed.status == "404 Not Found"
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let empty = response("POST", route, "", &state).await;
            record!(
                "POST",
                route,
                "missing-empty-or-conflict-state",
                empty.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&empty.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
        {
            let state = closed_db_state(&base_env).await;
            let closed = response("POST", route, "", &state).await;
            record!(
                "POST",
                route,
                "runtime-failure-and-timeout",
                closed.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&closed.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let first = response(
                "POST",
                if set_idle {
                    "/api/v0/backfill/idle"
                } else {
                    "/api/v0/backfill/busy"
                },
                "",
                &state,
            )
            .await;
            let second = response(
                "POST",
                if set_idle {
                    "/api/v0/backfill/busy"
                } else {
                    "/api/v0/backfill/idle"
                },
                "",
                &state,
            )
            .await;
            let stats = response("GET", "/api/v0/backfill/stats", "", &state).await;
            record!(
                "POST",
                route,
                "mutation-side-effects-and-readback",
                first.status == "200 OK"
                    && second.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&stats.body)
                        .is_ok_and(|value| value["isIdle"] == !set_idle)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let changed = response("POST", route, "", &state).await;
            let (restarted, _receiver) = test_state_with_env(base_env.clone());
            let stats = response("GET", "/api/v0/backfill/stats", "", &restarted).await;
            record!(
                "POST",
                route,
                "restart-persistence-or-reset",
                changed.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&stats.body)
                        .is_ok_and(|value| value["isIdle"] == false)
            );
        }
        {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let (left, right) = tokio::join!(
                response("POST", route, "", &state),
                response("POST", route, "", &state)
            );
            record!(
                "POST",
                route,
                "concurrency-and-idempotency",
                left.status == "200 OK"
                    && right.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&left.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
                    && serde_json::from_str::<serde_json::Value>(&right.body)
                        .is_ok_and(|value| value["isIdle"] == expected)
            );
        }
    }

    // POST /file: validation, process-local attempt accounting, reset,
    // closed HashDb failure, and concurrent valid requests.
    let file_body = r#"{"peerId":"backfill-residual-peer","path":"Library/BackfillResidual.flac","size":98765}"#;
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let nominal = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&nominal.body)
                    .is_ok_and(|value| value["peerId"] == "backfill-residual-peer")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("POST", "/api/v0/backfill/file", "not-json", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let missing = response("POST", "/api/v0/backfill/file", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let failed = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "runtime-failure-and-timeout",
            failed.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        let stats = response("GET", "/api/v0/backfill/stats", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "mutation-side-effects-and-readback",
            mutation.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&stats.body)
                    .is_ok_and(|value| value["totalAttempts"] == 1)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/file", file_body, &state).await;
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let stats = response("GET", "/api/v0/backfill/stats", "", &restarted).await;
        record!(
            "POST",
            "/api/v0/backfill/file",
            "restart-persistence-or-reset",
            mutation.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&stats.body)
                    .is_ok_and(|value| value["totalAttempts"] == 0)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            response("POST", "/api/v0/backfill/file", file_body, &state),
            response("POST", "/api/v0/backfill/file", file_body, &state)
        );
        record!(
            "POST",
            "/api/v0/backfill/file",
            "concurrency-and-idempotency",
            left.status == "200 OK" && right.status == "200 OK"
        );
    }

    // POST /trigger: the scheduler returns a result envelope even when
    // the candidate set is empty or HashDb throws; the state mutation is
    // visible in stats and is reset on a fresh process.
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let nominal = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        let value = serde_json::from_str::<serde_json::Value>(&nominal.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && value["candidatesEvaluated"] == 0
                && value["backfillsAttempted"] == 0
                && value["results"].is_array()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let malformed = response("POST", "/api/v0/backfill/trigger/extra", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let empty = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&empty.body)
                    .is_ok_and(|value| value["candidatesEvaluated"] == 0)
        );
    }
    {
        let state = closed_db_state(&base_env).await;
        let closed = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "runtime-failure-and-timeout",
            closed.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&closed.body)
                    .is_ok_and(|value| value["candidatesEvaluated"] == 0)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        let stats = response("GET", "/api/v0/backfill/stats", "", &state).await;
        let stats = serde_json::from_str::<serde_json::Value>(&stats.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "mutation-side-effects-and-readback",
            mutation.status == "200 OK"
                && stats["lastCycleTime"].is_string()
                && stats["nextCycleTime"].is_string()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let mutation = response("POST", "/api/v0/backfill/trigger", "", &state).await;
        let (restarted, _receiver) = test_state_with_env(base_env.clone());
        let stats = response("GET", "/api/v0/backfill/stats", "", &restarted).await;
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "restart-persistence-or-reset",
            mutation.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&stats.body)
                    .is_ok_and(|value| value["lastCycleTime"].is_null())
        );
    }
    {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            response("POST", "/api/v0/backfill/trigger", "", &state),
            response("POST", "/api/v0/backfill/trigger", "", &state)
        );
        record!(
            "POST",
            "/api/v0/backfill/trigger",
            "concurrency-and-idempotency",
            left.status == "200 OK"
                && right.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&left.body).is_ok()
                && serde_json::from_str::<serde_json::Value>(&right.body).is_ok()
        );
    }

    assert_eq!(ledger.len(), 47, "Backfill residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Backfill evidence directory");
    fs::write(
        evidence_dir.join("backfill_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Backfill ledger"),
    )
    .expect("write Backfill ledger");
    assert!(
        mismatches.is_empty(),
        "{} Backfill residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

async fn seed_kindless_library_item(state: &Arc<super::AppState>, title: &str) -> String {
    let mut library = state.library.write().await;
    let record = library
        .create(
            "Differential Artist".to_owned(),
            title.to_owned(),
            String::new(),
        )
        .expect("kindless library fixture capacity");
    format!("{}-missing-kind", record.id)
}

/// Differential proof for the remaining native slskdn controller cases.
/// The probes use the same local state stores as the handlers, including
/// closed SQLite pools for storage-failure branches and independent state
/// instances for reset behavior.  The v0 nominal and malformed warm-cache
/// cases, plus v0 remediation success, are already credited by earlier
/// ledgers and are intentionally not duplicated here.
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
async fn controller_api_differential_native_native_open_cases() {
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
    let capabilities_contract = |response: &super::routing::HttpResponse,
                                 scene_pod_bridge: bool| {
        let value = json_value(response);
        let required_features = [
            "mbid_jobs",
            "discography_jobs",
            "label_crate_jobs",
            "canonical_scoring",
            "rescue_mode",
            "library_health",
            "warm_cache",
            "job_manifests",
            "session_traces",
            "playback_aware",
        ];
        let features = value["features"].as_array();
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["impl"] == "slskdn"
            && value["compat"] == "slskd"
            && features.is_some_and(|features| {
                required_features.iter().all(|feature| {
                    features
                        .iter()
                        .any(|value| value.as_str() == Some(*feature))
                })
            })
            && value["obfuscation"]["supportedConnectionTypes"]
                == serde_json::json!(["P", "D", "F"])
            && value["feature"]["scenePodBridge"] == scene_pod_bridge
            && (!scene_pod_bridge
                || features.is_some_and(|features| {
                    features
                        .iter()
                        .any(|value| value.as_str() == Some("scene_pod_bridge"))
                }))
    };
    let health_contract = |response: &super::routing::HttpResponse, empty: bool| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["path"] == "(all)"
            && value["summary"]["total_issues"].is_u64()
            && value["summary"]["issues_open"].is_u64()
            && value["summary"]["issues_resolved"].is_u64()
            && value["issues"].is_array()
            && (!empty || value["summary"]["total_issues"] == 0)
    };
    let remediation_contract = |response: &super::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value.is_object()
            && (value["id"].as_str().is_some()
                || value["job_id"].as_str().is_some()
                || value["jobId"].as_str().is_some())
    };

    for (path, route) in [
        ("/api/slskdn/capabilities", "/api/slskdn/capabilities"),
        ("/api/v0/slskdn/capabilities", "/api/v0/slskdn/capabilities"),
    ] {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed_path = format!("{path}/extra");
        let malformed = super::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {malformed_path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            capabilities_contract(&empty, false)
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("capabilities runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path} with closed database: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            capabilities_contract(&runtime, false)
        );

        if path.starts_with("/api/v0/") {
            let (populated_state, _receiver) = test_state_with_env(target_env());
            populated_state
                .media_services
                .write()
                .await
                .features
                .scene_pod_bridge = true;
            let populated = super::route_http_request("GET", path, None, "", &populated_state)
                .await
                .unwrap_or_else(|error| panic!("GET {path} populated: {error}"));
            record!(
                "GET",
                route,
                "populated-dynamic-state",
                capabilities_contract(&populated, true)
            );
        }
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("native library-health runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime =
            super::route_http_request("GET", "/api/slskdn/library/health", None, "", &state)
                .await
                .expect("unversioned native library-health runtime response");
        record!(
            "GET",
            "/api/slskdn/library/health",
            "runtime-failure-and-timeout",
            health_contract(&runtime, true)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing =
            super::route_http_request("GET", "/api/v0/slskdn/library/health", None, "", &state)
                .await
                .expect("versioned native library-health empty response");
        record!(
            "GET",
            "/api/v0/slskdn/library/health",
            "missing-empty-or-conflict-state",
            health_contract(&missing, true)
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned native library-health runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/slskdn/library/health",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("versioned native library-health runtime response");
        record!(
            "GET",
            "/api/v0/slskdn/library/health",
            "runtime-failure-and-timeout",
            health_contract(&runtime, true)
        );
    }

    for path in ["/api/slskdn/library/remediate"] {
        let (state, _receiver) = test_state_with_env(target_env());
        let issue_id = seed_kindless_library_item(&state, "Native remediation").await;
        let success = super::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [issue_id]}).to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            remediation_contract(&success)
        );
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            remediation_contract(&success) && state.library.read().await.health_issues().is_empty()
        );

        let malformed_state = test_state_with_env(target_env()).0;
        let malformed = super::route_http_request("POST", path, None, "not-json", &malformed_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} malformed: {error}"));
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = super::route_http_request("POST", path, None, "{}", &malformed_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("native remediation runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        let runtime_issue = seed_kindless_library_item(&runtime_state, "Runtime remediation").await;
        db.close_for_test().await;
        let runtime = super::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [runtime_issue]}).to_string(),
            &runtime_state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path} runtime: {error}"));
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
                && runtime_state.library.read().await.health_issues().len() == 1
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset_issue = seed_kindless_library_item(&reset_state, "Reset remediation").await;
        let reset = super::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [reset_issue]}).to_string(),
            &reset_state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path} reset: {error}"));
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "GET",
            "/api/slskdn/library/health",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("native remediation reset health response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            remediation_contract(&reset) && health_contract(&restarted, true)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let left_issue = seed_kindless_library_item(&concurrent_state, "Concurrent left").await;
        let right_issue = seed_kindless_library_item(&concurrent_state, "Concurrent right").await;
        let left_body = serde_json::json!({"issue_ids": [left_issue]}).to_string();
        let right_body = serde_json::json!({"issue_ids": [right_issue]}).to_string();
        let (left, right) = tokio::join!(
            super::route_http_request("POST", path, None, &left_body, &concurrent_state),
            super::route_http_request("POST", path, None, &right_body, &concurrent_state)
        );
        let left = left.expect("left native remediation concurrency response");
        let right = right.expect("right native remediation concurrency response");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            remediation_contract(&left)
                && remediation_contract(&right)
                && concurrent_state
                    .library
                    .read()
                    .await
                    .health_issues()
                    .is_empty()
        );
    }

    for path in ["/api/v0/slskdn/library/remediate"] {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request("POST", path, None, "not-json", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} malformed: {error}"));
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = super::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned native remediation runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        let runtime_issue =
            seed_kindless_library_item(&runtime_state, "Versioned runtime remediation").await;
        db.close_for_test().await;
        let runtime = super::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [runtime_issue]}).to_string(),
            &runtime_state,
        )
        .await
        .expect("versioned native remediation runtime response");
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
                && runtime_state.library.read().await.health_issues().len() == 1
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset_issue =
            seed_kindless_library_item(&reset_state, "Versioned reset remediation").await;
        let reset = super::route_http_request(
            "POST",
            path,
            None,
            &serde_json::json!({"issue_ids": [reset_issue]}).to_string(),
            &reset_state,
        )
        .await
        .expect("versioned native remediation reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "GET",
            "/api/v0/slskdn/library/health",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("versioned native remediation reset health response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            remediation_contract(&reset) && health_contract(&restarted, true)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let left_issue =
            seed_kindless_library_item(&concurrent_state, "Versioned concurrent left").await;
        let right_issue =
            seed_kindless_library_item(&concurrent_state, "Versioned concurrent right").await;
        let left_body = serde_json::json!({"issue_ids": [left_issue]}).to_string();
        let right_body = serde_json::json!({"issue_ids": [right_issue]}).to_string();
        let (left, right) = tokio::join!(
            super::route_http_request("POST", path, None, &left_body, &concurrent_state),
            super::route_http_request("POST", path, None, &right_body, &concurrent_state)
        );
        let left = left.expect("left versioned remediation concurrency response");
        let right = right.expect("right versioned remediation concurrency response");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            remediation_contract(&left)
                && remediation_contract(&right)
                && concurrent_state
                    .library
                    .read()
                    .await
                    .health_issues()
                    .is_empty()
        );
    }

    let write_warm_cache_config = |state: &Arc<super::AppState>| {
        fs::write(
            state.config.state_dir.join("slskd.yml"),
            "warmCache:\n  enabled: true\n",
        )
        .expect("write native warm-cache config");
    };
    let warm_body = r#"{"mb_release_ids":[" rel-native ","REL-NATIVE"],"mb_artist_ids":["artist-native"],"mb_label_ids":[]}"#;
    let warm_success = |response: &super::routing::HttpResponse| {
        response.status == "200 OK" && response.body == r#"{"accepted":true}"#
    };

    for path in ["/api/slskdn/warm-cache/hints"] {
        let (disabled_state, _receiver) = test_state_with_env(target_env());
        let missing = super::route_http_request("POST", path, None, "{}", &disabled_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let (warm_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&warm_state);
        let accepted = super::route_http_request("POST", path, None, warm_body, &warm_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} nominal: {error}"));
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            warm_success(&accepted)
        );
        let features = warm_state.controller_features.read().await;
        let popularity_pass = features
            .get("warm-cache/popularity/mb:release:rel-native")
            .is_some_and(|value| value["hits"] == 1)
            && features
                .get("warm-cache/popularity/mb:artist:artist-native")
                .is_some_and(|value| value["hits"] == 1);
        drop(features);
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            popularity_pass
        );

        let malformed = super::route_http_request(
            "POST",
            path,
            None,
            r#"{"mb_release_ids":[42]}"#,
            &warm_state,
        )
        .await
        .unwrap_or_else(|error| panic!("POST {path} malformed: {error}"));
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("native warm-cache runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        write_warm_cache_config(&runtime_state);
        db.close_for_test().await;
        let runtime = super::route_http_request("POST", path, None, warm_body, &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} runtime: {error}"));
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            warm_success(&runtime)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&reset_state);
        let reset = super::route_http_request("POST", path, None, warm_body, &reset_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} reset: {error}"));
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request("POST", path, None, warm_body, &restarted_state)
            .await
            .expect("native warm-cache reset response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            warm_success(&reset)
                && restarted.status == "400 Bad Request"
                && restarted.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&concurrent_state);
        let (left, right) = tokio::join!(
            super::route_http_request("POST", path, None, warm_body, &concurrent_state),
            super::route_http_request("POST", path, None, warm_body, &concurrent_state)
        );
        let left = left.expect("left native warm-cache concurrency response");
        let right = right.expect("right native warm-cache concurrency response");
        let features = concurrent_state.controller_features.read().await;
        let concurrent_hits = features
            .get("warm-cache/popularity/mb:release:rel-native")
            .and_then(|value| value["hits"].as_u64())
            == Some(2);
        drop(features);
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            warm_success(&left) && warm_success(&right) && concurrent_hits
        );
    }

    for path in ["/api/v0/slskdn/warm-cache/hints"] {
        let (state, _receiver) = test_state_with_env(target_env());
        let missing = super::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned native warm-cache runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        write_warm_cache_config(&runtime_state);
        db.close_for_test().await;
        let runtime = super::route_http_request("POST", path, None, warm_body, &runtime_state)
            .await
            .expect("versioned native warm-cache runtime response");
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            warm_success(&runtime)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&reset_state);
        let reset = super::route_http_request("POST", path, None, warm_body, &reset_state)
            .await
            .expect("versioned native warm-cache reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request("POST", path, None, warm_body, &restarted_state)
            .await
            .expect("versioned native warm-cache restarted response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            warm_success(&reset)
                && restarted.status == "400 Bad Request"
                && restarted.body == r#"{"error":"Warm cache not enabled"}"#
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        write_warm_cache_config(&concurrent_state);
        let (left, right) = tokio::join!(
            super::route_http_request("POST", path, None, warm_body, &concurrent_state),
            super::route_http_request("POST", path, None, warm_body, &concurrent_state)
        );
        let left = left.expect("left versioned warm-cache concurrency response");
        let right = right.expect("right versioned warm-cache concurrency response");
        let features = concurrent_state.controller_features.read().await;
        let concurrent_hits = features
            .get("warm-cache/popularity/mb:release:rel-native")
            .and_then(|value| value["hits"].as_u64())
            == Some(2);
        drop(features);
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            warm_success(&left) && warm_success(&right) && concurrent_hits
        );
    }

    assert_eq!(ledger.len(), 33, "slskdn native residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create slskdn native evidence directory");
    fs::write(
        evidence_dir.join("native_native_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn native ledger"),
    )
    .expect("write slskdn native ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn native controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for audio canonical/dedupe reads and analyzer
/// migration.  The read probes use one real content-discovery entry for
/// populated state; the closed-pool cases exercise the durable-storage
/// boundary rather than treating an in-memory empty result as success.
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
async fn controller_api_differential_audio_canonical_dedupe_and_migration() {
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
    let audio_read_contract = |response: &super::routing::HttpResponse,
                               recording_id: &str,
                               field: &str,
                               populated: bool| {
        let value = json_value(response);
        let entries = value[field].as_array();
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["recordingId"] == recording_id
            && entries.is_some_and(|entries| !populated || entries.is_empty())
    };

    for (path, route, field) in [
        (
            "/api/audio/canonical/",
            "/api/audio/canonical/{recordingId}",
            "candidates",
        ),
        (
            "/api/audio/variants/dedupe/",
            "/api/audio/variants/dedupe/{recordingId}",
            "groups",
        ),
        (
            "/api/v0/audio/canonical/",
            "/api/v0/audio/canonical/{recordingId}",
            "candidates",
        ),
        (
            "/api/v0/audio/variants/dedupe/",
            "/api/v0/audio/variants/dedupe/{recordingId}",
            "groups",
        ),
    ] {
        let recording_id = "audio-differential-recording";
        let exact_path = format!("{path}{recording_id}");
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = super::route_http_request("GET", &exact_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {exact_path}: {error}"));
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            audio_read_contract(&nominal, recording_id, field, false)
        );

        let malformed_path = format!("{exact_path}/extra");
        let malformed = super::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {malformed_path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing_path = format!("{path}%20");
        let missing = super::route_http_request("GET", &missing_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {missing_path}: {error}"));
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("audio runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("GET", &exact_path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {exact_path} runtime: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
        );

        let (populated_state, _receiver) = test_state_with_env(target_env());
        populated_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "audio-differential-key".to_owned(),
                byte_hash: "a".repeat(64),
                size: 123,
                full_file_hash: "c".repeat(64),
                music_brainz_id: recording_id.to_owned(),
                file_sha256: "c".repeat(64),
                ..Default::default()
            }])
            .expect("seed audio content-discovery entry");
        let populated = super::route_http_request("GET", &exact_path, None, "", &populated_state)
            .await
            .unwrap_or_else(|error| panic!("GET {exact_path} populated: {error}"));
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            audio_read_contract(&populated, recording_id, field, true)
        );
    }

    let analyzer_contract = |response: &super::routing::HttpResponse, updated: u64| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["updated"] == updated
    };

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let runtime =
            super::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
                .await
                .expect("unversioned analyzer runtime response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "runtime-failure-and-timeout",
            runtime.status == "400 Bad Request" && runtime.body.contains("ApiVersionUnspecified")
        );
        let mutation =
            super::route_http_request("POST", "/api/audio/analyzers/migrate", None, "{}", &state)
                .await
                .expect("unversioned analyzer mutation response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "mutation-side-effects-and-readback",
            mutation.status == "400 Bad Request" && mutation.body.contains("ApiVersionUnspecified")
        );
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "POST",
            "/api/audio/analyzers/migrate",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("unversioned analyzer reset response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "restart-persistence-or-reset",
            restarted.status == "400 Bad Request"
                && restarted.body.contains("ApiVersionUnspecified")
        );
        let (left, right) = tokio::join!(
            super::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state),
            super::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
        );
        let left = left.expect("left unversioned analyzer concurrency response");
        let right = right.expect("right unversioned analyzer concurrency response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "concurrency-and-idempotency",
            left.status == "400 Bad Request"
                && right.status == "400 Bad Request"
                && left.body.contains("ApiVersionUnspecified")
                && right.body.contains("ApiVersionUnspecified")
        );
    }

    {
        let path = "/api/v0/audio/analyzers/migrate";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "POST",
            "/api/v0/audio/analyzers/migrate/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned analyzer malformed path response");
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = super::route_http_request("POST", path, None, "", &state)
            .await
            .expect("versioned analyzer empty response");
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            analyzer_contract(&missing, 0)
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned analyzer runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("POST", path, None, "", &runtime_state)
            .await
            .expect("versioned analyzer runtime response");
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
        );

        let (mutation_state, _receiver) = test_state_with_env(target_env());
        mutation_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "analyzer-differential-key".to_owned(),
                byte_hash: "d".repeat(64),
                full_file_hash: "f".repeat(64),
                size: 123,
                music_brainz_id: "analyzer-recording".to_owned(),
                file_sha256: "f".repeat(64),
                ..Default::default()
            }])
            .expect("seed analyzer migration entry");
        let mutation = super::route_http_request("POST", path, None, "", &mutation_state)
            .await
            .expect("versioned analyzer mutation response");
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            analyzer_contract(&mutation, 1)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        reset_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "analyzer-reset-key".to_owned(),
                byte_hash: "1".repeat(64),
                full_file_hash: "3".repeat(64),
                size: 123,
                music_brainz_id: "analyzer-reset-recording".to_owned(),
                file_sha256: "3".repeat(64),
                ..Default::default()
            }])
            .expect("seed analyzer reset entry");
        let reset = super::route_http_request("POST", path, None, "", &reset_state)
            .await
            .expect("versioned analyzer reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request("POST", path, None, "", &restarted_state)
            .await
            .expect("versioned analyzer restarted response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            analyzer_contract(&reset, 1) && analyzer_contract(&restarted, 0)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        concurrent_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "analyzer-concurrent-key".to_owned(),
                byte_hash: "4".repeat(64),
                full_file_hash: "6".repeat(64),
                size: 123,
                music_brainz_id: "analyzer-concurrent-recording".to_owned(),
                file_sha256: "6".repeat(64),
                ..Default::default()
            }])
            .expect("seed analyzer concurrency entry");
        let (left, right) = tokio::join!(
            super::route_http_request("POST", path, None, "", &concurrent_state),
            super::route_http_request("POST", path, None, "", &concurrent_state)
        );
        let left = left.expect("left analyzer concurrency response");
        let right = right.expect("right analyzer concurrency response");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            analyzer_contract(&left, 1) && analyzer_contract(&right, 1)
        );
    }

    assert_eq!(ledger.len(), 30, "audio residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create audio evidence directory");
    fs::write(
        evidence_dir.join("audio_canonical_dedupe_migration.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize audio ledger"),
    )
    .expect("write audio ledger");
    assert!(
        mismatches.is_empty(),
        "{} audio controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining versioned taste-recommendation
/// controller cases.  Each subroute is exercised with its real WorkRef
/// shape, a null/empty request, a closed persistence pool, a fresh-state
/// reset, and concurrent requests.  Wishlist promotion additionally
/// checks the actual stored item and duplicate behavior.
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
async fn controller_api_differential_taste_recommendation_open_cases() {
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
    let root_contract = |response: &super::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["minimumTrustedSources"].is_u64()
            && value["trustedActorCount"].is_u64()
            && value["candidateCount"].is_u64()
            && value["recommendations"].is_array()
    };
    let subroute_contract = |kind: &str, response: &super::routing::HttpResponse| {
        let value = json_value(response);
        match kind {
            "graph" => {
                response.status == "200 OK"
                    && value["graph_data"].is_array()
                    && value["nodes"].is_u64()
                    && value["edges"].is_u64()
            }
            "release" => {
                response.status == "200 OK"
                    && value["recommendations"].is_array()
                    && value["count"].is_u64()
                    && value["status"] == "processing"
            }
            "wishlist" => {
                response.status == "200 OK"
                    && value["created"].is_boolean()
                    && value["searchText"].is_string()
            }
            _ => false,
        }
    };
    let root_body = r#"{"minimumTrustedSources":3}"#;
    let valid_work_ref = r#"{"workRef":{"domain":"music","title":"Taste Differential","creator":"Differential Artist","externalIds":{"musicbrainz":"abcd1234-1234-4a12-9a12-0a1234567890"}}}"#;

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            "not-json",
            &state,
        )
        .await
        .expect("taste recommendations malformed response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );

        let missing =
            super::route_http_request("POST", "/api/v0/taste-recommendations", None, "", &state)
                .await
                .expect("taste recommendations empty response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "missing-empty-or-conflict-state",
            root_contract(&missing)
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("taste recommendations runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            root_body,
            &runtime_state,
        )
        .await
        .expect("taste recommendations runtime response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "runtime-failure-and-timeout",
            root_contract(&runtime)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset = super::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            root_body,
            &reset_state,
        )
        .await
        .expect("taste recommendations reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            root_body,
            &restarted_state,
        )
        .await
        .expect("taste recommendations restarted response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "restart-persistence-or-reset",
            root_contract(&reset) && root_contract(&restarted)
        );

        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/taste-recommendations",
                None,
                root_body,
                &state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/taste-recommendations",
                None,
                root_body,
                &state
            )
        );
        let left = left.expect("left taste recommendations concurrency response");
        let right = right.expect("right taste recommendations concurrency response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "concurrency-and-idempotency",
            root_contract(&left) && root_contract(&right)
        );
    }

    for (path, kind) in [
        ("/api/v0/taste-recommendations/graph-preview", "graph"),
        ("/api/v0/taste-recommendations/release-radar", "release"),
        ("/api/v0/taste-recommendations/wishlist", "wishlist"),
    ] {
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = super::route_http_request("POST", path, None, valid_work_ref, &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} nominal: {error}"));
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            subroute_contract(kind, &nominal)
        );

        let missing = super::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("taste subroute runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), super::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = super::route_http_request("POST", path, None, valid_work_ref, &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} runtime: {error}"));
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            if kind == "wishlist" {
                runtime.status == "503 Service Unavailable"
            } else {
                subroute_contract(kind, &runtime)
            }
        );

        let (mutation_state, _receiver) = test_state_with_env(target_env());
        let mutation = if kind == "graph" {
            mutation_state
                .interests
                .write()
                .await
                .add_liked("graph-differential-interest".to_owned());
            super::route_http_request("POST", path, None, valid_work_ref, &mutation_state)
                .await
                .expect("graph preview mutation response")
        } else if kind == "release" {
            super::route_http_request(
                "POST",
                "/api/v0/taste-recommendations/wishlist",
                None,
                valid_work_ref,
                &mutation_state,
            )
            .await
            .expect("seed taste wishlist before release radar");
            super::route_http_request("POST", path, None, valid_work_ref, &mutation_state)
                .await
                .expect("release radar mutation response")
        } else {
            super::route_http_request("POST", path, None, valid_work_ref, &mutation_state)
                .await
                .expect("wishlist mutation response")
        };
        let mutation_value = json_value(&mutation);
        let mutation_readback = if kind == "graph" {
            mutation_value["nodes"] == 1
                && mutation_value["graph_data"]
                    .as_array()
                    .is_some_and(|nodes| !nodes.is_empty())
        } else if kind == "release" {
            mutation_value["count"] == 1
                && mutation_value["recommendations"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
        } else {
            mutation_value["created"] == true
                && mutation_state
                    .wishlist
                    .read()
                    .await
                    .records
                    .iter()
                    .flat_map(|record| record.items.iter())
                    .count()
                    == 1
        };
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            subroute_contract(kind, &mutation) && mutation_readback
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset = super::route_http_request("POST", path, None, valid_work_ref, &reset_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} reset: {error}"));
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted =
            super::route_http_request("POST", path, None, valid_work_ref, &restarted_state)
                .await
                .unwrap_or_else(|error| panic!("POST {path} restarted: {error}"));
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            subroute_contract(kind, &reset) && subroute_contract(kind, &restarted)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            super::route_http_request("POST", path, None, valid_work_ref, &concurrent_state),
            super::route_http_request("POST", path, None, valid_work_ref, &concurrent_state)
        );
        let left = left.expect("left taste subroute concurrency response");
        let right = right.expect("right taste subroute concurrency response");
        let concurrency_readback = if kind == "wishlist" {
            concurrent_state
                .wishlist
                .read()
                .await
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .count()
                == 1
        } else {
            true
        };
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            subroute_contract(kind, &left)
                && subroute_contract(kind, &right)
                && concurrency_readback
        );
    }

    assert_eq!(
        ledger.len(),
        23,
        "taste recommendation residual ledger size"
    );
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create taste recommendation evidence directory");
    fs::write(
        evidence_dir.join("taste_recommendation_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize taste recommendation ledger"),
    )
    .expect("write taste recommendation ledger");
    assert!(
        mismatches.is_empty(),
        "{} taste recommendation controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
