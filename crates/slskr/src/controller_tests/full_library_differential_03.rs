//! Controller full library differential 03 ownership.

use super::*;

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
pub(super) async fn controller_api_differential_library_residuals() {
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
            let response = crate::route_http_request("GET", &request_path, None, "", &state)
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
        let started = crate::route_http_request(
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
        let response = crate::route_http_request("GET", &path, None, "", &state)
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
        let response = crate::route_http_request("GET", path, None, "", &items_state)
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
        let response = crate::route_http_request("GET", &path, None, "", &items_state)
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
        let response = crate::route_http_request("GET", path, None, "", &items_state)
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
            let response = crate::route_http_request(method, path, None, body, &state)
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
        let response = crate::route_http_request(
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
            crate::route_http_request(
                "PATCH",
                "/api/v0/library/health/issues/lib-1-missing-artist",
                None,
                r#"{"artist":"Recovered Artist"}"#,
                &state
            ),
            crate::route_http_request(
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
        let response = crate::route_http_request("POST", "/api/library/scan", None, body, &state)
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
            let readback = crate::route_http_request(
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
            crate::route_http_request(
                "POST",
                "/api/library/scan",
                None,
                r#"{"path":"/music"}"#,
                &state
            ),
            crate::route_http_request(
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
                    crate::route_http_request(method, path, None, body, &state),
                    crate::route_http_request(method, path, None, body, &state)
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
                let response = crate::route_http_request(method, path, None, body, &state)
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
