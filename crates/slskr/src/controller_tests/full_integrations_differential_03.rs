//! Controller full integrations differential 03 ownership.

use super::*;

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
pub(super) async fn controller_api_differential_integrations_residuals() {
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
        let response = crate::route_http_request(
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
            crate::route_http_request("DELETE", "/api/integrations/spotify", None, "", &state)
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
        let malformed = crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("Spotify status runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response =
            crate::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
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
        let response = crate::route_http_request(
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
        let malformed = crate::route_http_request(
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned Spotify status runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = crate::route_http_request(
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
        let response = crate::route_http_request("GET", path, None, "", &state)
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("unversioned Spotify callback runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), crate::SearchStore::new(), Some(db.clone()));
        let authorize = crate::route_http_request(
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
        let response = crate::route_http_request("GET", &path, None, "", &state)
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
        let response = crate::route_http_request("GET", path, None, "", &state)
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned Spotify callback runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(spotify_env(), crate::SearchStore::new(), Some(db.clone()));
        crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let malformed = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
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
            crate::route_http_request(
                "POST",
                "/api/v0/integrations/spotify/authorize",
                None,
                "",
                &state
            ),
            crate::route_http_request(
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
        let malformed = crate::route_http_request(
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
            crate::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
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
        let connection_path = crate::spotify_connection_path(&state.config.state_dir);
        fs::create_dir_all(&connection_path).expect("create Spotify connection conflict");
        let response =
            crate::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
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
            crate::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
                .await
                .expect("seed Spotify disconnect restart state");
        let restarted = test_state_with_env(spotify_env()).0;
        let readback = crate::route_http_request(
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
            crate::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state),
            crate::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
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
        let malformed = crate::route_http_request(
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
        let missing = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request("GET", path, None, "", &state)
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
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("Lidarr sync runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            base_env.clone(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let malformed = crate::route_http_request(
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
        let missing = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let malformed = crate::route_http_request(
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
        let missing = crate::route_http_request(
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
        let response = crate::route_http_request(
            "POST",
            "/api/v0/integrations/lidarr/manualimport",
            None,
            r#"{"directory":"/downloads/residual"}"#,
            &state,
        )
        .await
        .expect("seed Lidarr manual import restart state");
        let restarted = test_state_with_env(base_env.clone()).0;
        let readback = crate::route_http_request(
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
        let response = crate::route_http_request("POST", path, None, "", &state)
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
            crate::route_http_request(
                "POST",
                "/api/v0/integrations/lidarr/wanted/sync",
                None,
                "",
                &state
            ),
            crate::route_http_request(
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
