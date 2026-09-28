//! Controller full native api differential ownership.

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
pub(super) async fn controller_api_differential_native_library_fallback_uses_current_case_mode_for_share_filters(
) {
    let root = std::env::temp_dir().join(format!(
        "slskr-library-filter-case-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("SECRET.flac"), b"library").unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_SHARED_DIR", &root.display().to_string())
            .with("SLSKD_SHARE_FILTER", r"(?<=/)secret(?=\.flac$)")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    );
    {
        let mut shares = state.shares.write().await;
        shares.entries.clear();
        shares.local_paths.clear();
    }

    let visible_response = crate::route_http_request(
        "GET",
        "/api/v0/library/items?query=SECRET",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(visible_response.status, "200 OK");
    assert!(visible_response.content_type.contains("application/json"));
    let visible = serde_json::from_str::<serde_json::Value>(&visible_response.body).unwrap();
    assert_eq!(visible["items"].as_array().unwrap().len(), 1);
    assert_eq!(visible["items"][0]["fileName"], "SECRET.flac");
    assert!(visible["items"][0]["contentId"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));

    *state
        .controller_case_sensitive_regex
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = false;
    let filtered_response = crate::route_http_request(
        "GET",
        "/api/v0/library/items?query=SECRET",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(filtered_response.status, "200 OK");
    assert!(filtered_response.content_type.contains("application/json"));
    let filtered = serde_json::from_str::<serde_json::Value>(&filtered_response.body).unwrap();
    assert!(filtered["items"].as_array().unwrap().is_empty());

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/library/items",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/library/items",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("library_items_case_filter_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    fs::remove_dir_all(root).unwrap();
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
pub(super) async fn controller_api_differential_native_network_stats_edge_contracts() {
    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native");

    let (malformed_state, _receiver) = test_state_with_env(target_env());
    let malformed = crate::route_http_request(
        "GET",
        "/api/v0/network/stats?includePeers=not-a-boolean",
        None,
        "",
        &malformed_state,
    )
    .await
    .expect("network stats malformed query");
    assert_eq!(malformed.status, "400 Bad Request");
    assert!(malformed.body.contains("boolean"), "{}", malformed.body);

    let (empty_state, _receiver) = test_state_with_env(target_env());
    let empty = crate::route_http_request("GET", "/api/v0/network/stats", None, "", &empty_state)
        .await
        .expect("network stats empty state");
    assert_eq!(empty.status, "200 OK");
    let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap();
    for key in [
        "backfill",
        "capabilitiesJson",
        "capabilitiesVersion",
        "dht",
        "discoveredPeers",
        "hashDb",
        "mesh",
        "meshPeers",
        "swarmJobs",
        "transport",
    ] {
        assert!(empty_json.get(key).is_some(), "missing {key}");
    }
    assert_eq!(empty_json["discoveredPeers"], serde_json::json!([]));
    assert_eq!(empty_json["meshPeers"], serde_json::json!([]));
    assert_eq!(empty_json["swarmJobs"], serde_json::json!([]));

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("network stats runtime db");
    let (runtime_state, _receiver) =
        test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
    db.close_for_test().await;
    let runtime =
        crate::route_http_request("GET", "/api/v0/network/stats", None, "", &runtime_state)
            .await
            .expect("network stats closed-database state");
    assert_eq!(runtime.status, "200 OK");
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap();
    assert!(runtime_json.get("dht").is_some());
    assert!(runtime_json.get("transport").is_some());

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "malformed-path-query-or-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "missing-empty-or-conflict-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "runtime-failure-and-timeout",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("network_stats_edge_contracts.json"),
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
pub(super) async fn controller_api_differential_native_read_projection_runtime_contracts() {
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

    let federation_env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "runtime.example")
            .with("FEDERATION_BASE_URL", "https://runtime.example/")
    };

    let (logs_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let malformed_logs =
        crate::route_http_request("GET", "/api/v0/logs/malformed", None, "", &logs_state)
            .await
            .expect("malformed logs path");
    record!(
        "/api/v0/logs",
        "malformed-path-query-or-body",
        malformed_logs.status == "404 Not Found"
    );
    let empty_logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &logs_state)
        .await
        .expect("empty logs projection");
    let empty_logs_json =
        serde_json::from_str::<serde_json::Value>(&empty_logs.body).unwrap_or_default();
    record!(
        "/api/v0/logs",
        "missing-empty-or-conflict-state",
        empty_logs.status == "200 OK" && empty_logs_json.as_array().is_some()
    );
    let logs_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("logs runtime database");
    let (logs_runtime_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(logs_db.clone()),
    );
    logs_db.close_for_test().await;
    let runtime_logs =
        crate::route_http_request("GET", "/api/v0/logs", None, "", &logs_runtime_state)
            .await
            .expect("logs closed-database projection");
    record!(
        "/api/v0/logs",
        "runtime-failure-and-timeout",
        runtime_logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_logs.body)
                .is_ok_and(|value| value.is_array())
    );

    let (federation_state, _receiver) = test_state_with_env(federation_env());
    let malformed_federation = crate::route_http_request(
        "GET",
        "/api/v0/federation/diagnostics/malformed",
        None,
        "",
        &federation_state,
    )
    .await
    .expect("malformed federation diagnostics path");
    record!(
        "/api/v0/federation/diagnostics",
        "malformed-path-query-or-body",
        malformed_federation.status == "404 Not Found"
    );
    let empty_federation = crate::route_http_request(
        "GET",
        "/api/v0/federation/diagnostics",
        None,
        "",
        &federation_state,
    )
    .await
    .expect("empty federation diagnostics projection");
    let empty_federation_json =
        serde_json::from_str::<serde_json::Value>(&empty_federation.body).unwrap_or_default();
    record!(
        "/api/v0/federation/diagnostics",
        "missing-empty-or-conflict-state",
        empty_federation.status == "200 OK"
            && empty_federation_json["federation"].is_object()
            && empty_federation_json["warnings"].is_array()
    );
    let federation_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("federation runtime database");
    let (federation_runtime_state, _receiver) = test_state_with_env_parts(
        federation_env(),
        crate::SearchStore::new(),
        Some(federation_db.clone()),
    );
    federation_db.close_for_test().await;
    let runtime_federation = crate::route_http_request(
        "GET",
        "/api/v0/federation/diagnostics",
        None,
        "",
        &federation_runtime_state,
    )
    .await
    .expect("federation closed-database projection");
    let runtime_federation_json =
        serde_json::from_str::<serde_json::Value>(&runtime_federation.body).unwrap_or_default();
    record!(
        "/api/v0/federation/diagnostics",
        "runtime-failure-and-timeout",
        runtime_federation.status == "200 OK" && runtime_federation_json["federation"].is_object()
    );

    let webfinger_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("WebFinger runtime database");
    let (webfinger_runtime_state, _receiver) = test_state_with_env_parts(
        federation_env(),
        crate::SearchStore::new(),
        Some(webfinger_db.clone()),
    );
    webfinger_db.close_for_test().await;
    let runtime_webfinger = crate::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40runtime.example",
        None,
        "",
        &webfinger_runtime_state,
    )
    .await
    .expect("WebFinger closed-database projection");
    record!(
        "/.well-known/webfinger",
        "runtime-failure-and-timeout",
        runtime_webfinger.status == "200 OK"
            && runtime_webfinger.content_type == "application/jrd+json"
    );

    let collection_id = "00000000-0000-0000-0000-000000000001";
    let collections_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("collections runtime database");
    let (collections_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(collections_db.clone()),
    );
    collections_db.close_for_test().await;
    let runtime_collections =
        crate::route_http_request("GET", "/api/v0/collections", None, "", &collections_state)
            .await
            .expect("collections closed-database list");
    record!(
        "/api/v0/collections",
        "runtime-failure-and-timeout",
        runtime_collections.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_collections.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_collection = crate::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}"),
        None,
        "",
        &collections_state,
    )
    .await
    .expect("collection closed-database detail");
    record!(
        "/api/v0/collections/{id}",
        "runtime-failure-and-timeout",
        runtime_collection.status == "404 Not Found"
    );
    let runtime_collection_items = crate::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}/items"),
        None,
        "",
        &collections_state,
    )
    .await
    .expect("collection closed-database items");
    record!(
        "/api/v0/collections/{id}/items",
        "runtime-failure-and-timeout",
        runtime_collection_items.status == "404 Not Found"
    );

    let sharegroup_id = "00000000-0000-0000-0000-000000000002";
    let sharegroups_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("sharegroups runtime database");
    let (sharegroups_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(sharegroups_db.clone()),
    );
    sharegroups_db.close_for_test().await;
    let runtime_sharegroups =
        crate::route_http_request("GET", "/api/v0/sharegroups", None, "", &sharegroups_state)
            .await
            .expect("sharegroups closed-database list");
    record!(
        "/api/v0/sharegroups",
        "runtime-failure-and-timeout",
        runtime_sharegroups.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_sharegroups.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_sharegroup = crate::route_http_request(
        "GET",
        &format!("/api/v0/sharegroups/{sharegroup_id}"),
        None,
        "",
        &sharegroups_state,
    )
    .await
    .expect("sharegroup closed-database detail");
    record!(
        "/api/v0/sharegroups/{id}",
        "runtime-failure-and-timeout",
        runtime_sharegroup.status == "404 Not Found"
    );
    let runtime_sharegroup_members = crate::route_http_request(
        "GET",
        &format!("/api/v0/sharegroups/{sharegroup_id}/members"),
        None,
        "",
        &sharegroups_state,
    )
    .await
    .expect("sharegroup closed-database members");
    record!(
        "/api/v0/sharegroups/{id}/members",
        "runtime-failure-and-timeout",
        runtime_sharegroup_members.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("native_read_projection_runtime_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api read projection mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_native_solid_status_edge_contracts() {
    let target = "slskdn";
    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());

    let malformed =
        crate::route_http_request("GET", "/api/v0/solid/status/extra", None, "", &state)
            .await
            .expect("malformed Solid status path");
    assert_eq!(malformed.status, "404 Not Found");

    let empty = crate::route_http_request("GET", "/api/v0/solid/status", None, "", &state)
        .await
        .expect("empty Solid status");
    let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap();
    assert_eq!(empty.status, "200 OK");
    assert_eq!(empty_json["enabled"], true);
    assert!(empty_json["clientId"].is_string());
    assert!(empty_json["redirectPath"].is_string());

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("Solid status runtime database");
    let (runtime_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(db.clone()));
    db.close_for_test().await;
    let runtime =
        crate::route_http_request("GET", "/api/v0/solid/status", None, "", &runtime_state)
            .await
            .expect("Solid status closed-database response");
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap();
    assert_eq!(runtime.status, "200 OK");
    assert_eq!(runtime_json["enabled"], true);
    assert!(runtime_json["clientId"].is_string());

    let ledger = vec![
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/solid/status",
            "case": "malformed-path-query-or-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/solid/status",
            "case": "missing-empty-or-conflict-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/solid/status",
            "case": "runtime-failure-and-timeout",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("solid_status_edge_contracts.json"),
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
pub(super) async fn controller_api_differential_native_solid_resolution_runtime_contracts() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let target = "slskdn";
    async fn configure_failure(state: &Arc<crate::AppState>) {
        let mut media = state.media_services.write().await;
        media.solid.allow_insecure_http = true;
        media.solid.allow_localhost_for_web_id = true;
        media.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
        media.solid.timeout = Duration::from_millis(50);
    }
    let body = r#"{"webId":"http://127.0.0.1:9/profile#me"}"#;

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("Solid resolution runtime database");
    let (runtime_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    configure_failure(&runtime_state).await;
    db.close_for_test().await;
    let runtime = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        body,
        &runtime_state,
    )
    .await
    .expect("Solid resolution runtime failure");
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap();
    assert_eq!(runtime.status, "500 Internal Server Error");
    assert_eq!(runtime_json["title"], "Failed to resolve WebID");

    let (restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    configure_failure(&restart_state).await;
    let restarted = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        body,
        &restart_state,
    )
    .await
    .expect("Solid resolution restart failure");
    assert_eq!(restarted.status, "500 Internal Server Error");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&restarted.body).unwrap()["title"],
        "Failed to resolve WebID"
    );

    let (concurrent_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    configure_failure(&concurrent_state).await;
    let (first, second) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/solid/resolve-webid",
            None,
            body,
            &concurrent_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/solid/resolve-webid",
            None,
            body,
            &concurrent_state,
        )
    );
    assert_eq!(
        first.expect("first concurrent Solid failure").status,
        "500 Internal Server Error"
    );
    assert_eq!(
        second.expect("second concurrent Solid failure").status,
        "500 Internal Server Error"
    );

    let (mutation_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    configure_failure(&mutation_state).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Solid profile fixture");
    let port = listener.local_addr().expect("Solid fixture address").port();
    let web_id = format!("http://127.0.0.1:{port}/profile/card#me");
    let profile = format!(
        "@prefix solid: <http://www.w3.org/ns/solid/terms#>.\n<{web_id}> solid:oidcIssuer <https://runtime-issuer.example/oidc>.\n"
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("Solid profile request");
        let mut request = [0_u8; 4096];
        let _ = stream
            .read(&mut request)
            .await
            .expect("read Solid profile request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/turtle\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            profile.len(),
            profile
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write Solid profile response");
    });
    let resolved = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        &serde_json::json!({"webId": web_id}).to_string(),
        &mutation_state,
    )
    .await
    .expect("Solid resolution readback");
    server.await.expect("Solid profile fixture task");
    let resolved_json = serde_json::from_str::<serde_json::Value>(&resolved.body).unwrap();
    let mutation_pass = resolved.status == "200 OK"
        && resolved_json["webId"] == web_id
        && resolved_json["oidcIssuers"]
            == serde_json::json!(["https://runtime-issuer.example/oidc"]);
    assert!(mutation_pass, "{}", resolved.body);

    let ledger = vec![
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "runtime-failure-and-timeout",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "restart-persistence-or-reset",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "concurrency-and-idempotency",
            "pass": true,
        }),
        serde_json::json!({
            "target": target,
            "method": "POST",
            "route": "/api/v0/solid/resolve-webid",
            "case": "mutation-side-effects-and-readback",
            "pass": mutation_pass,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("solid_resolution_runtime_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) fn controller_api_differential_native_application_dump_gates() {
    run_controller_future_on_large_stack("native-application-dump-gates", || {
        controller_api_differential_native_application_dump_gates_impl()
    });
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
pub(super) async fn controller_api_differential_native_application_open_cases() {
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

    let (application_state, _receiver) = test_state_with_env(env.clone());
    let application_malformed = crate::route_http_request(
        "GET",
        "/api/v0/application/extra",
        None,
        "",
        &application_state,
    )
    .await
    .expect("malformed application state path");
    record!(
        "GET",
        "/api/v0/application",
        "malformed-path-query-or-body",
        application_malformed.status == "404 Not Found"
    );

    let application_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("application runtime database");
    let (application_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(application_db.clone()),
    );
    application_db.close_for_test().await;
    let loopback_failure = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application-runtime-failure"}"#,
        &application_failure_state,
    )
    .await
    .expect("application runtime fixture");
    let application_runtime = crate::route_http_request(
        "GET",
        "/api/v0/application",
        None,
        "",
        &application_failure_state,
    )
    .await
    .expect("application state after runtime failure");
    record!(
        "GET",
        "/api/v0/application",
        "runtime-failure-and-timeout",
        loopback_failure.status == "200 OK"
            && application_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&application_runtime.body)
                .is_ok_and(|value| value.is_object())
            && application_failure_state
                .session
                .read()
                .await
                .last_error
                .is_some()
    );

    let (version_state, _receiver) = test_state_with_env(env.clone());
    let build_malformed = crate::route_http_request(
        "GET",
        "/api/v0/application/build?checkForUpdates=not-a-bool",
        None,
        "",
        &version_state,
    )
    .await
    .expect("malformed application build query");
    record!(
        "GET",
        "/api/v0/application/build",
        "malformed-path-query-or-body",
        build_malformed.status == "400 Bad Request"
    );
    let build_missing =
        crate::route_http_request("GET", "/api/v0/application/build", None, "", &version_state)
            .await
            .expect("empty application build state");
    record!(
        "GET",
        "/api/v0/application/build",
        "missing-empty-or-conflict-state",
        build_missing.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&build_missing.body)
                .is_ok_and(|value| value.is_object())
    );

    let version_lookup_failure =
        crate::refresh_controller_version_check(&version_state, "http://127.0.0.1:9").await;
    assert!(version_lookup_failure.is_err());
    let build_runtime =
        crate::route_http_request("GET", "/api/v0/application/build", None, "", &version_state)
            .await
            .expect("application build after version lookup failure");
    record!(
        "GET",
        "/api/v0/application/build",
        "runtime-failure-and-timeout",
        build_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&build_runtime.body)
                .is_ok_and(|value| value.is_object())
            && version_state
                .controller_version
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .checked_at
                .is_some()
    );

    let version_malformed = crate::route_http_request(
        "GET",
        "/api/v0/application/version/extra",
        None,
        "",
        &version_state,
    )
    .await
    .expect("malformed application version path");
    record!(
        "GET",
        "/api/v0/application/version",
        "malformed-path-query-or-body",
        version_malformed.status == "404 Not Found"
    );
    let version_missing = crate::route_http_request(
        "GET",
        "/api/v0/application/version",
        None,
        "",
        &version_state,
    )
    .await
    .expect("empty application version state");
    record!(
        "GET",
        "/api/v0/application/version",
        "missing-empty-or-conflict-state",
        version_missing.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&version_missing.body)
                .is_ok_and(|value| value == env!("CARGO_PKG_VERSION"))
    );
    let version_runtime = crate::route_http_request(
        "GET",
        "/api/v0/application/version",
        None,
        "",
        &version_state,
    )
    .await
    .expect("application version after lookup failure");
    record!(
        "GET",
        "/api/v0/application/version",
        "runtime-failure-and-timeout",
        version_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&version_runtime.body)
                .is_ok_and(|value| value == env!("CARGO_PKG_VERSION"))
    );
    let version_mutation =
        crate::route_http_request("PUT", "/api/v0/application", None, "{}", &version_state)
            .await
            .expect("populate application runtime state");
    let populated_version = crate::route_http_request(
        "GET",
        "/api/v0/application/version",
        None,
        "",
        &version_state,
    )
    .await
    .expect("populated application version");
    record!(
        "GET",
        "/api/v0/application/version",
        "populated-dynamic-state",
        version_mutation.status == "204 No Content"
            && populated_version.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&populated_version.body)
                .is_ok_and(|value| value == env!("CARGO_PKG_VERSION"))
    );

    let latest_malformed = crate::route_http_request(
        "GET",
        "/api/v0/application/version/latest?forceCheck=not-a-bool",
        None,
        "",
        &version_state,
    )
    .await
    .expect("malformed latest version query");
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "malformed-path-query-or-body",
        latest_malformed.status == "400 Bad Request"
    );
    let latest_missing = crate::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &version_state,
    )
    .await
    .expect("empty latest version state");
    let latest_missing_json =
        serde_json::from_str::<serde_json::Value>(&latest_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "missing-empty-or-conflict-state",
        latest_missing.status == "200 OK"
            && latest_missing_json.is_object()
            && latest_missing_json["current"] == env!("CARGO_PKG_VERSION")
    );
    let latest_runtime = crate::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &version_state,
    )
    .await
    .expect("latest version after lookup failure");
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "runtime-failure-and-timeout",
        latest_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&latest_runtime.body)
                .is_ok_and(|value| value.is_object())
            && version_state
                .controller_version
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .checked_at
                .is_some()
    );

    let (put_state, _receiver) = test_state_with_env(env.clone());
    let put_nominal =
        crate::route_http_request("PUT", "/api/v0/application", None, "{}", &put_state)
            .await
            .expect("application restart");
    record!(
        "PUT",
        "/api/v0/application",
        "nominal-status-headers-body",
        put_nominal.status == "204 No Content" && put_nominal.body.is_empty()
    );
    let put_malformed =
        crate::route_http_request("PUT", "/api/v0/application/extra", None, "{}", &put_state)
            .await
            .expect("malformed application restart path");
    record!(
        "PUT",
        "/api/v0/application",
        "malformed-path-query-or-body",
        put_malformed.status == "404 Not Found"
    );
    let put_missing = crate::route_http_request("PUT", "/api/v0/application", None, "", &put_state)
        .await
        .expect("empty application restart body");
    record!(
        "PUT",
        "/api/v0/application",
        "missing-empty-or-conflict-state",
        put_missing.status == "204 No Content" && put_missing.body.is_empty()
    );
    let put_readback =
        crate::route_http_request("GET", "/api/v0/application", None, "", &put_state)
            .await
            .expect("application restart readback");
    record!(
        "PUT",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        put_readback.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&put_readback.body)
                .is_ok_and(|value| value["pendingRestart"] == true)
    );
    let (fresh_put_state, _receiver) = test_state_with_env(env.clone());
    let fresh_put_readback =
        crate::route_http_request("GET", "/api/v0/application", None, "", &fresh_put_state)
            .await
            .expect("fresh application restart readback");
    record!(
        "PUT",
        "/api/v0/application",
        "restart-persistence-or-reset",
        fresh_put_readback.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_put_readback.body)
                .is_ok_and(|value| value["pendingRestart"] == false)
    );
    let (concurrent_put_state, _receiver) = test_state_with_env(env.clone());
    let concurrent_puts = futures_util::future::join_all([
        crate::route_http_request(
            "PUT",
            "/api/v0/application",
            None,
            "{}",
            &concurrent_put_state,
        ),
        crate::route_http_request(
            "PUT",
            "/api/v0/application",
            None,
            "{}",
            &concurrent_put_state,
        ),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/application",
        "concurrency-and-idempotency",
        concurrent_puts.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content"))
            && concurrent_put_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    let (delete_state, _receiver) = test_state_with_env(env.clone());
    delete_state
        .runtime
        .write()
        .await
        .set_restart_requested(true);
    let delete_nominal =
        crate::route_http_request("DELETE", "/api/v0/application", None, "", &delete_state)
            .await
            .expect("application shutdown");
    record!(
        "DELETE",
        "/api/v0/application",
        "nominal-status-headers-body",
        delete_nominal.status == "204 No Content" && delete_nominal.body.is_empty()
    );
    let delete_malformed = crate::route_http_request(
        "DELETE",
        "/api/v0/application/extra",
        None,
        "",
        &delete_state,
    )
    .await
    .expect("malformed application shutdown path");
    record!(
        "DELETE",
        "/api/v0/application",
        "malformed-path-query-or-body",
        delete_malformed.status == "404 Not Found"
    );
    let delete_missing =
        crate::route_http_request("DELETE", "/api/v0/application", None, "", &delete_state)
            .await
            .expect("empty application shutdown body");
    record!(
        "DELETE",
        "/api/v0/application",
        "missing-empty-or-conflict-state",
        delete_missing.status == "204 No Content" && delete_missing.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        !delete_state
            .runtime
            .read()
            .await
            .application_restart_requested
    );
    let (fresh_delete_state, _receiver) = test_state_with_env(env.clone());
    let fresh_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/application",
        None,
        "",
        &fresh_delete_state,
    )
    .await
    .expect("fresh application shutdown");
    record!(
        "DELETE",
        "/api/v0/application",
        "restart-persistence-or-reset",
        fresh_delete.status == "204 No Content"
            && !fresh_delete_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );
    let (concurrent_delete_state, _receiver) = test_state_with_env(env.clone());
    concurrent_delete_state
        .runtime
        .write()
        .await
        .set_restart_requested(true);
    let concurrent_deletes = futures_util::future::join_all([
        crate::route_http_request(
            "DELETE",
            "/api/v0/application",
            None,
            "",
            &concurrent_delete_state,
        ),
        crate::route_http_request(
            "DELETE",
            "/api/v0/application",
            None,
            "",
            &concurrent_delete_state,
        ),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/application",
        "concurrency-and-idempotency",
        concurrent_deletes.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content"))
            && !concurrent_delete_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    let (gc_state, _receiver) = test_state_with_env(env.clone());
    let gc_nominal =
        crate::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
            .await
            .expect("application garbage collection");
    record!(
        "POST",
        "/api/v0/application/gc",
        "nominal-status-headers-body",
        gc_nominal.status == "200 OK" && gc_nominal.body.is_empty()
    );
    let gc_malformed =
        crate::route_http_request("POST", "/api/v0/application/gc/extra", None, "", &gc_state)
            .await
            .expect("malformed application gc path");
    record!(
        "POST",
        "/api/v0/application/gc",
        "malformed-path-query-or-body",
        gc_malformed.status == "404 Not Found"
    );
    let gc_missing =
        crate::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
            .await
            .expect("empty application gc body");
    record!(
        "POST",
        "/api/v0/application/gc",
        "missing-empty-or-conflict-state",
        gc_missing.status == "200 OK" && gc_missing.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/application/gc",
        "mutation-side-effects-and-readback",
        gc_state.runtime.read().await.gc_runs == 2
    );
    let (fresh_gc_state, _receiver) = test_state_with_env(env.clone());
    record!(
        "POST",
        "/api/v0/application/gc",
        "restart-persistence-or-reset",
        fresh_gc_state.runtime.read().await.gc_runs == 0
    );
    let (concurrent_gc_state, _receiver) = test_state_with_env(env.clone());
    let concurrent_gcs = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/application/gc",
            None,
            "",
            &concurrent_gc_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/application/gc",
            None,
            "",
            &concurrent_gc_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/application/gc",
        "concurrency-and-idempotency",
        concurrent_gcs.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && concurrent_gc_state.runtime.read().await.gc_runs == 2
    );

    let (loopback_state, _receiver) = test_state_with_env(env.clone());
    let loopback_nominal = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application"}"#,
        &loopback_state,
    )
    .await
    .expect("application loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "nominal-status-headers-body",
        loopback_nominal.status == "200 OK"
            && loopback_nominal.content_type.is_empty()
            && loopback_nominal.body.is_empty()
    );
    let loopback_malformed = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        "{",
        &loopback_state,
    )
    .await
    .expect("malformed application loopback body");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "malformed-path-query-or-body",
        loopback_malformed.status == "400 Bad Request"
    );
    let loopback_missing = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        "",
        &loopback_state,
    )
    .await
    .expect("empty application loopback body");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "missing-empty-or-conflict-state",
        loopback_missing.status == "400 Bad Request"
    );
    let loopback_logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &loopback_state)
        .await
        .expect("application loopback logs");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "mutation-side-effects-and-readback",
        loopback_logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&loopback_logs.body).is_ok_and(|value| {
                value.as_array().is_some_and(|logs| {
                    logs.iter().any(|log| {
                        log["category"] == "application"
                            && log["message"] == "Loopback POST: {\"probe\":\"application\"}"
                    })
                })
            },)
    );
    let loopback_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("loopback runtime database");
    let (loopback_failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(loopback_db.clone()),
    );
    loopback_db.close_for_test().await;
    let loopback_runtime = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application-runtime"}"#,
        &loopback_failure_state,
    )
    .await
    .expect("application loopback runtime failure");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "runtime-failure-and-timeout",
        loopback_runtime.status == "200 OK"
            && loopback_failure_state
                .session
                .read()
                .await
                .last_error
                .is_some()
    );
    let (fresh_loopback_state, _receiver) = test_state_with_env(env.clone());
    let fresh_logs =
        crate::route_http_request("GET", "/api/v0/logs", None, "", &fresh_loopback_state)
            .await
            .expect("fresh application loopback logs");
    let fresh_loopback = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"application-fresh"}"#,
        &fresh_loopback_state,
    )
    .await
    .expect("fresh application loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "restart-persistence-or-reset",
        fresh_loopback.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_logs.body)
                .is_ok_and(|value| value.as_array().is_some_and(Vec::is_empty))
    );
    let (concurrent_loopback_state, _receiver) = test_state_with_env(env);
    let concurrent_loopbacks = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"application-concurrent-a"}"#,
            &concurrent_loopback_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"application-concurrent-b"}"#,
            &concurrent_loopback_state,
        ),
    ])
    .await;
    let concurrent_loopback_logs =
        crate::route_http_request("GET", "/api/v0/logs", None, "", &concurrent_loopback_state)
            .await
            .expect("concurrent application loopback logs");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "concurrency-and-idempotency",
        concurrent_loopbacks.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && serde_json::from_str::<serde_json::Value>(&concurrent_loopback_logs.body).is_ok_and(
                |value| {
                    value.as_array().is_some_and(|logs| {
                        logs.iter()
                            .filter(|log| log["category"] == "application")
                            .count()
                            >= 2
                    })
                }
            )
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create application evidence directory");
    fs::write(
        evidence_dir.join("application_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize application evidence"),
    )
    .expect("write application evidence");
    assert!(
        mismatches.is_empty(),
        "{} slskdN application mismatches:\n{}",
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
pub(super) async fn controller_api_differential_native_ranking_contracts() {
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
        .expect("slskdn ranking database");
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));

    let empty_history =
        crate::route_http_request("GET", "/api/v0/ranking/history/rank-peer", None, "", &state)
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
        crate::route_http_request("GET", "/api/v0/ranking/history/%20", None, "", &state)
            .await
            .expect("malformed ranking history path");
    record!(
        "GET",
        "/api/v0/ranking/history/{username}",
        "malformed-path-query-or-body",
        malformed_history.status == "400 Bad Request"
            && malformed_history.body == "Username is required"
    );

    let missing_history = crate::route_http_request(
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
    crate::persist_transfer_records(&state, &seeded_entries)
        .await
        .expect("persist ranking transfer history");
    let _ = success_id;

    let populated_history =
        crate::route_http_request("GET", "/api/v0/ranking/history/rank-peer", None, "", &state)
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
    let histories = crate::route_http_request(
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
        crate::route_http_request("POST", "/api/v0/ranking/history", None, "not-json", &state)
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
        crate::route_http_request("POST", "/api/v0/ranking/history", None, "[]", &state)
            .await
            .expect("empty ranking histories");
    let blank_histories = crate::route_http_request(
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
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone())).0;
    restart_state
        .transfers
        .write()
        .await
        .rehydrate_from_database(&db)
        .await;
    let restarted_histories = crate::route_http_request(
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
        crate::route_http_request(
            "POST",
            "/api/v0/ranking/history",
            None,
            r#"["rank-peer","other-peer"]"#,
            &state,
        ),
        crate::route_http_request(
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
    let ranked = crate::route_http_request("POST", "/api/v0/ranking/rank", None, rank_body, &state)
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
        crate::route_http_request("POST", "/api/v0/ranking/rank", None, "{}", &state)
            .await
            .expect("malformed rank sources");
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "malformed-path-query-or-body",
        malformed_rank.status == "400 Bad Request"
            && malformed_rank.body == "At least one source candidate is required"
    );

    let empty_rank = crate::route_http_request("POST", "/api/v0/ranking/rank", None, "[]", &state)
        .await
        .expect("empty rank sources");
    let invalid_rank = crate::route_http_request(
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

    let restarted_rank = crate::route_http_request(
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

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("ranking failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_rank = crate::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":"rank-peer","filename":"x.flac"}]"#,
        &failure_state,
    )
    .await
    .expect("ranking failure");
    let failed_history = crate::route_http_request(
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
        crate::route_http_request(
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
        crate::route_http_request("POST", "/api/v0/ranking/rank", None, rank_body, &state),
        crate::route_http_request("POST", "/api/v0/ranking/rank", None, rank_body, &state)
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

/// Differential evidence for the slskdN NowPlayingController webhook and
/// its remaining malformed/runtime cases.  The frozen webhook accepts
/// generic, Plex, and Jellyfin JSON, clears on stop/pause notifications,
/// rejects empty or invalid payloads, and remains process-local rather
/// than depending on SQLite.
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_native_nowplaying_webhook_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method,
                    $route,
                    $case
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

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());

    let malformed_get =
        crate::route_http_request("GET", "/api/v0/nowplaying/extra", None, "", &state)
            .await
            .expect("malformed now-playing GET");
    record!(
        "GET",
        "/api/v0/nowplaying",
        "malformed-path-query-or-body",
        malformed_get.status == "404 Not Found"
    );

    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("now-playing runtime database");
    let (runtime_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(runtime_db.clone()));
    runtime_db.close_for_test().await;
    let runtime_get =
        crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &runtime_state)
            .await
            .expect("now-playing GET after database close");
    record!(
        "GET",
        "/api/v0/nowplaying",
        "runtime-failure-and-timeout",
        runtime_get.status == "204 No Content"
    );

    let malformed_delete =
        crate::route_http_request("DELETE", "/api/v0/nowplaying/extra", None, "", &state)
            .await
            .expect("malformed now-playing DELETE");
    record!(
        "DELETE",
        "/api/v0/nowplaying",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );

    let malformed_put = crate::route_http_request("PUT", "/api/v0/nowplaying", None, "{}", &state)
        .await
        .expect("malformed now-playing PUT");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "malformed-path-query-or-body",
        malformed_put.status == "400 Bad Request"
            && malformed_put.body.contains("Artist and title are required")
    );

    let missing_put = crate::route_http_request("PUT", "/api/v0/nowplaying", None, "", &state)
        .await
        .expect("missing now-playing PUT");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "missing-empty-or-conflict-state",
        missing_put.status == "400 Bad Request"
            && missing_put.body.contains("Track data is required")
    );

    let plex = crate::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"event":"media.play","Metadata":{"grandparentTitle":"Plex Artist","title":"Plex Track","parentTitle":"Plex Album"}}"#,
        &state,
    )
    .await
    .expect("nominal Plex now-playing webhook");
    let plex_current = crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
        .await
        .expect("Plex now-playing readback");
    let plex_json =
        serde_json::from_str::<serde_json::Value>(&plex_current.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "nominal-status-headers-body",
        plex.status == "200 OK"
            && plex.body.is_empty()
            && plex.content_type.starts_with("application/json")
            && plex_json["artist"] == "Plex Artist"
            && plex_json["title"] == "Plex Track"
            && plex_json["album"].is_null()
    );

    let generic = crate::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"artist":"Generic Artist","title":"Generic Track","event":"play"}"#,
        &state,
    )
    .await
    .expect("generic now-playing webhook");
    let generic_current = crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
        .await
        .expect("generic now-playing readback");
    let generic_json =
        serde_json::from_str::<serde_json::Value>(&generic_current.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "mutation-side-effects-and-readback",
        generic.status == "200 OK"
            && generic_current.status == "200 OK"
            && generic_json["artist"] == "Generic Artist"
            && generic_json["title"] == "Generic Track"
    );

    let jellyfin = crate::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"NotificationType":"PlaybackStart","Artist":"Jelly Artist","Name":"Jelly Track","Album":"Jelly Album"}"#,
        &state,
    )
    .await
    .expect("Jellyfin now-playing webhook");
    let jellyfin_stop = crate::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"NotificationType":"PlaybackStop"}"#,
        &state,
    )
    .await
    .expect("Jellyfin stop webhook");
    assert_eq!(jellyfin.status, "200 OK");
    assert_eq!(jellyfin_stop.status, "200 OK");
    assert!(state.now_playing.read().await.records.is_empty());

    let malformed_webhook = crate::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("malformed now-playing webhook");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "malformed-path-query-or-body",
        malformed_webhook.status == "400 Bad Request"
            && malformed_webhook.body.contains("Invalid JSON payload")
    );

    let missing_webhook =
        crate::route_http_request("POST", "/api/v0/nowplaying/webhook", None, "", &state)
            .await
            .expect("missing now-playing webhook");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "missing-empty-or-conflict-state",
        missing_webhook.status == "400 Bad Request"
            && missing_webhook.body.contains("Empty payload")
    );

    let seeded = crate::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"artist":"Transient Artist","title":"Transient Track"}"#,
        &state,
    )
    .await
    .expect("seed transient webhook track");
    let (restarted_state, _receiver) = test_state_with_env(env());
    let restarted =
        crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &restarted_state)
            .await
            .expect("now-playing webhook after restart");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "restart-persistence-or-reset",
        seeded.status == "200 OK" && restarted.status == "204 No Content"
    );

    let concurrent_responses = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/nowplaying/webhook",
            None,
            r#"{"artist":"Concurrent A","title":"Track A"}"#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/nowplaying/webhook",
            None,
            r#"{"artist":"Concurrent B","title":"Track B"}"#,
            &state,
        ),
    ])
    .await;
    let concurrent_current =
        crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("concurrent now-playing webhook readback");
    let concurrent_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_current.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_current.status == "200 OK"
            && ["Concurrent A", "Concurrent B"]
                .contains(&concurrent_json["artist"].as_str().unwrap_or_default())
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("now-playing webhook failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    let failure_webhook = crate::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"artist":"Failure Artist","title":"Failure Track"}"#,
        &failure_state,
    )
    .await
    .expect("now-playing webhook after database close");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "runtime-failure-and-timeout",
        failure_webhook.status == "200 OK"
            && failure_state.now_playing.read().await.records.len() == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_nowplaying_webhook_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn now-playing webhook ledger"),
    )
    .expect("write slskdn now-playing webhook ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn now-playing webhook mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
