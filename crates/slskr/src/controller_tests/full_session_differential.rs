//! Controller full session differential ownership.

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
pub(super) async fn controller_api_differential_session_issue_and_revoke() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "secret-token")
            .with("SLSKD_USERNAME", "admin")
            .with("SLSKD_PASSWORD", "secret-token"),
    );
    let login = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"secret-token"}"#,
        &state,
    )
    .await
    .expect("admin login");
    assert_eq!(login.status, "200 OK", "{}", login.body);
    let login = serde_json::from_str::<serde_json::Value>(&login.body).unwrap();
    assert_eq!(login["name"], "admin");
    assert_eq!(login["tokenType"], "Bearer");
    assert!(login["expires"].as_u64().unwrap() > login["issued"].as_u64().unwrap());
    let token = login["token"].as_str().unwrap();
    assert_eq!(token.split('.').count(), 3);
    let authorization = format!("Bearer {token}");

    let authorized = crate::route_http_request(
        "GET",
        "/api/v0/application",
        Some(&authorization),
        "",
        &state,
    )
    .await
    .expect("JWT-authorized request");
    assert_eq!(authorized.status, "200 OK", "{}", authorized.body);

    let mut tampered_token = token.to_owned();
    let replacement = if tampered_token.ends_with('a') {
        'b'
    } else {
        'a'
    };
    tampered_token.pop();
    tampered_token.push(replacement);
    let tampered_authorization = format!("Bearer {tampered_token}");
    let tampered = crate::route_http_request(
        "GET",
        "/api/v0/application",
        Some(&tampered_authorization),
        "",
        &state,
    )
    .await
    .expect("tampered JWT request");
    assert_eq!(tampered.status, "401 Unauthorized", "{}", tampered.body);

    let logout = crate::route_http_request(
        "DELETE",
        "/api/v0/session",
        Some(&authorization),
        "",
        &state,
    )
    .await
    .expect("JWT logout");
    assert_eq!(logout.status, "204 No Content", "{}", logout.body);

    let revoked = crate::route_http_request(
        "GET",
        "/api/v0/application",
        Some(&authorization),
        "",
        &state,
    )
    .await
    .expect("revoked JWT request");
    assert_eq!(revoked.status, "401 Unauthorized", "{}", revoked.body);

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/session",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/session",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/session",
            "case": "mutation-side-effects-and-readback",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("session_issue_and_revoke.json"),
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
pub(super) async fn controller_api_differential_server_session_open_cases() {
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

    let (session_state, _session_receiver) = test_state_with_env(env.clone());
    let session_malformed =
        crate::route_http_request("GET", "/api/v0/session/malformed", None, "", &session_state)
            .await
            .expect("malformed session path");
    record!(
        "GET",
        "/api/v0/session",
        "malformed-path-query-or-body",
        session_malformed.status == "404 Not Found"
    );
    let session_empty =
        crate::route_http_request("GET", "/api/v0/session", None, "", &session_state)
            .await
            .expect("empty session");
    let session_empty_json =
        serde_json::from_str::<serde_json::Value>(&session_empty.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        session_empty.status == "200 OK" && session_empty_json["state"] == "disconnected"
    );
    session_state.session.write().await.state = "connected";
    session_state.session.write().await.username = Some("session-open-user".to_owned());
    let session_populated =
        crate::route_http_request("GET", "/api/v0/session", None, "", &session_state)
            .await
            .expect("populated session");
    let session_populated_json =
        serde_json::from_str::<serde_json::Value>(&session_populated.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "populated-dynamic-state",
        session_populated.status == "200 OK"
            && session_populated_json["state"] == "connected"
            && session_populated_json["username"] == "session-open-user"
    );

    let session_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("session runtime database");
    let (session_runtime_state, _session_runtime_receiver) = test_state_with_env_parts(
        env.clone().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(session_runtime_db.clone()),
    );
    session_runtime_db.close_for_test().await;
    let session_runtime =
        crate::route_http_request("GET", "/api/v0/session", None, "", &session_runtime_state)
            .await
            .expect("runtime session");
    record!(
        "GET",
        "/api/v0/session",
        "runtime-failure-and-timeout",
        session_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&session_runtime.body)
                .is_ok_and(|value| value.is_object())
    );

    let enabled_malformed = crate::route_http_request(
        "GET",
        "/api/v0/session/enabled/extra",
        None,
        "",
        &session_state,
    )
    .await
    .expect("malformed session-enabled path");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "malformed-path-query-or-body",
        enabled_malformed.status == "404 Not Found"
    );
    let enabled_empty =
        crate::route_http_request("GET", "/api/v0/session/enabled", None, "", &session_state)
            .await
            .expect("empty session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "missing-empty-or-conflict-state",
        enabled_empty.status == "200 OK" && enabled_empty.body == "false"
    );
    let enabled_runtime = crate::route_http_request(
        "GET",
        "/api/v0/session/enabled",
        None,
        "",
        &session_runtime_state,
    )
    .await
    .expect("runtime session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "runtime-failure-and-timeout",
        enabled_runtime.status == "200 OK" && enabled_runtime.body == "false"
    );
    let enabled_env = env
        .clone()
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "session-open-api-token");
    let (enabled_state, _enabled_receiver) = test_state_with_env(enabled_env);
    let enabled_populated =
        crate::route_http_request("GET", "/api/v0/session/enabled", None, "", &enabled_state)
            .await
            .expect("populated session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "populated-dynamic-state",
        enabled_populated.status == "200 OK" && enabled_populated.body == "true"
    );

    let login_env = env
        .clone()
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "session-open-api-token")
        .with("SLSKD_USERNAME", "session-open-admin")
        .with("SLSKD_PASSWORD", "session-open-password");
    let (login_state, _login_receiver) = test_state_with_env(login_env.clone());
    let malformed_login_state = test_state_with_env(env.clone()).0;
    let malformed_login = crate::route_http_request(
        "POST",
        "/api/v0/session/extra",
        None,
        r#"{"username":"session-open-admin","password":"session-open-password"}"#,
        &malformed_login_state,
    )
    .await
    .expect("malformed login path");
    record!(
        "POST",
        "/api/v0/session",
        "malformed-path-query-or-body",
        malformed_login.status == "404 Not Found"
    );
    let login_body = r#"{"username":"session-open-admin","password":"session-open-password"}"#;
    let login =
        crate::route_http_request("POST", "/api/v0/session", None, login_body, &login_state)
            .await
            .expect("session login");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    let login_token = login_json["token"].as_str().unwrap_or_default().to_owned();
    let authorization = format!("Bearer {login_token}");
    let authorized_session = crate::route_http_request(
        "GET",
        "/api/v0/session",
        Some(&authorization),
        "",
        &login_state,
    )
    .await
    .expect("authorized session readback");
    record!(
        "POST",
        "/api/v0/session",
        "mutation-side-effects-and-readback",
        login.status == "200 OK"
            && login_json["tokenType"] == "Bearer"
            && login_token.split('.').count() == 3
            && authorized_session.status == "200 OK"
    );
    let restarted_login_state = test_state_with_env(login_env.clone()).0;
    let restarted_login = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        login_body,
        &restarted_login_state,
    )
    .await
    .expect("restarted session login");
    record!(
        "POST",
        "/api/v0/session",
        "restart-persistence-or-reset",
        restarted_login.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&restarted_login.body)
                .is_ok_and(|value| value["tokenType"] == "Bearer")
    );
    let login_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("login runtime database");
    let (login_runtime_state, _login_runtime_receiver) = test_state_with_env_parts(
        login_env.clone().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(login_runtime_db.clone()),
    );
    login_runtime_db.close_for_test().await;
    let runtime_login = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        login_body,
        &login_runtime_state,
    )
    .await
    .expect("runtime session login");
    record!(
        "POST",
        "/api/v0/session",
        "runtime-failure-and-timeout",
        runtime_login.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_login.body)
                .is_ok_and(|value| value["tokenType"] == "Bearer")
    );
    let concurrent_logins = futures_util::future::join_all([
        crate::route_http_request("POST", "/api/v0/session", None, login_body, &login_state),
        crate::route_http_request("POST", "/api/v0/session", None, login_body, &login_state),
    ])
    .await;
    let concurrent_tokens = concurrent_logins
        .iter()
        .filter_map(|response| response.as_ref().ok())
        .filter_map(|response| serde_json::from_str::<serde_json::Value>(&response.body).ok())
        .filter_map(|value| value["token"].as_str().map(str::to_owned))
        .collect::<BTreeSet<_>>();
    record!(
        "POST",
        "/api/v0/session",
        "concurrency-and-idempotency",
        concurrent_logins.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_tokens.len() == 2
    );

    let session_delete_malformed =
        crate::route_http_request("DELETE", "/api/v0/session/extra", None, "", &session_state)
            .await
            .expect("malformed session delete path");
    record!(
        "DELETE",
        "/api/v0/session",
        "malformed-path-query-or-body",
        session_delete_malformed.status == "404 Not Found"
    );
    let session_delete_empty =
        crate::route_http_request("DELETE", "/api/v0/session", None, "", &session_state)
            .await
            .expect("empty session delete");
    record!(
        "DELETE",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        session_delete_empty.status == "204 No Content" && session_delete_empty.body.is_empty()
    );
    let session_delete_runtime = crate::route_http_request(
        "DELETE",
        "/api/v0/session",
        None,
        "",
        &session_runtime_state,
    )
    .await
    .expect("runtime session delete");
    record!(
        "DELETE",
        "/api/v0/session",
        "runtime-failure-and-timeout",
        session_delete_runtime.status == "204 No Content"
    );
    let restarted_session_delete_state = test_state_with_env(env.clone()).0;
    let restarted_session_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/session",
        None,
        "",
        &restarted_session_delete_state,
    )
    .await
    .expect("restarted session delete");
    record!(
        "DELETE",
        "/api/v0/session",
        "restart-persistence-or-reset",
        restarted_session_delete.status == "204 No Content"
    );
    let concurrent_session_deletes = futures_util::future::join_all([
        crate::route_http_request("DELETE", "/api/v0/session", None, "", &session_state),
        crate::route_http_request("DELETE", "/api/v0/session", None, "", &session_state),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/session",
        "concurrency-and-idempotency",
        concurrent_session_deletes.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        })
    );

    let (server_state, mut server_receiver) = test_state_with_env(env.clone());
    let server_malformed =
        crate::route_http_request("GET", "/api/v0/server/extra", None, "", &server_state)
            .await
            .expect("malformed server path");
    record!(
        "GET",
        "/api/v0/server",
        "malformed-path-query-or-body",
        server_malformed.status == "404 Not Found"
    );
    let server_status_malformed =
        crate::route_http_request("GET", "/api/server/status/extra", None, "", &server_state)
            .await
            .expect("malformed server status path");
    record!(
        "GET",
        "/api/server/status",
        "malformed-path-query-or-body",
        server_status_malformed.status == "404 Not Found"
    );
    let server_status =
        crate::route_http_request("GET", "/api/server/status", None, "", &server_state)
            .await
            .expect("empty server status");
    let server_status_json =
        serde_json::from_str::<serde_json::Value>(&server_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/server/status",
        "missing-empty-or-conflict-state",
        server_status.status == "200 OK"
            && server_status_json["connected"] == false
            && server_status_json["state"] == "disconnected"
            && server_status_json["username"] == ""
    );
    let (server_put_missing_state, _server_put_missing_receiver) = test_state_with_env(env.clone());
    let server_put_missing =
        crate::route_http_request("PUT", "/api/v0/server", None, "", &server_put_missing_state)
            .await
            .expect("empty server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "missing-empty-or-conflict-state",
        server_put_missing.status == "200 OK"
            && server_put_missing_state.session.read().await.state == "connecting"
    );
    let server_put_malformed =
        crate::route_http_request("PUT", "/api/v0/server/extra", None, "", &server_state)
            .await
            .expect("malformed server connect path");
    record!(
        "PUT",
        "/api/v0/server",
        "malformed-path-query-or-body",
        server_put_malformed.status == "404 Not Found"
    );
    let server_connect =
        crate::route_http_request("PUT", "/api/v0/server", None, "", &server_state)
            .await
            .expect("server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "mutation-side-effects-and-readback",
        server_connect.status == "200 OK"
            && server_connect.body.is_empty()
            && server_state.session.read().await.state == "connecting"
            && matches!(
                server_receiver.try_recv(),
                Ok(crate::SessionCommand::Connect)
            )
    );
    let (concurrent_server_put_state, _concurrent_server_put_receiver) =
        test_state_with_env(env.clone());
    let first_server_put = crate::route_http_request(
        "PUT",
        "/api/v0/server",
        None,
        "",
        &concurrent_server_put_state,
    )
    .await
    .expect("first concurrent server connect");
    let concurrent_server_puts = futures_util::future::join_all([
        crate::route_http_request(
            "PUT",
            "/api/v0/server",
            None,
            "",
            &concurrent_server_put_state,
        ),
        crate::route_http_request(
            "PUT",
            "/api/v0/server",
            None,
            "",
            &concurrent_server_put_state,
        ),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/server",
        "concurrency-and-idempotency",
        first_server_put.status == "200 OK"
            && concurrent_server_puts.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "205 Reset Content")
            })
    );
    let (restarted_server_state, _restarted_server_receiver) = test_state_with_env(env.clone());
    let restarted_server_put =
        crate::route_http_request("PUT", "/api/v0/server", None, "", &restarted_server_state)
            .await
            .expect("restarted server connect");
    record!(
        "PUT",
        "/api/v0/server",
        "restart-persistence-or-reset",
        restarted_server_put.status == "200 OK"
            && restarted_server_state.session.read().await.state == "connecting"
    );
    let (put_failure_state, put_failure_receiver) = test_state_with_env(env.clone());
    drop(put_failure_receiver);
    let put_failure =
        crate::route_http_request("PUT", "/api/v0/server", None, "", &put_failure_state)
            .await
            .expect("server connect runtime failure");
    record!(
        "PUT",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        put_failure.status == "503 Service Unavailable"
            && put_failure_state.session.read().await.state == "disconnected"
    );

    let (delete_server_state, mut delete_server_receiver) = test_state_with_env(env.clone());
    delete_server_state.session.write().await.state = "connected";
    let delete_server =
        crate::route_http_request("DELETE", "/api/v0/server", None, "", &delete_server_state)
            .await
            .expect("server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "nominal-status-headers-body",
        delete_server.status == "204 No Content" && delete_server.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/server",
        "mutation-side-effects-and-readback",
        delete_server_state.session.read().await.state == "disconnecting"
            && matches!(
                delete_server_receiver.try_recv(),
                Ok(crate::SessionCommand::Disconnect)
            )
    );
    let delete_malformed =
        crate::route_http_request("DELETE", "/api/v0/server", None, "{", &server_state)
            .await
            .expect("malformed server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "malformed-path-query-or-body",
        delete_malformed.status == "400 Bad Request"
    );
    let delete_empty_state = test_state_with_env(env.clone()).0;
    let delete_empty =
        crate::route_http_request("DELETE", "/api/v0/server", None, "", &delete_empty_state)
            .await
            .expect("empty server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "missing-empty-or-conflict-state",
        delete_empty.status == "204 No Content"
    );
    let (delete_failure_state, delete_failure_receiver) = test_state_with_env(env.clone());
    delete_failure_state.session.write().await.state = "connected";
    drop(delete_failure_receiver);
    let delete_failure =
        crate::route_http_request("DELETE", "/api/v0/server", None, "", &delete_failure_state)
            .await
            .expect("server disconnect runtime failure");
    record!(
        "DELETE",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        delete_failure.status == "503 Service Unavailable"
            && delete_failure_state.session.read().await.state == "connected"
    );
    let restarted_server_delete_state = test_state_with_env(env.clone()).0;
    let restarted_server_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/server",
        None,
        "",
        &restarted_server_delete_state,
    )
    .await
    .expect("restarted server disconnect");
    record!(
        "DELETE",
        "/api/v0/server",
        "restart-persistence-or-reset",
        restarted_server_delete.status == "204 No Content"
    );
    let (concurrent_delete_state, _concurrent_delete_receiver) = test_state_with_env(env.clone());
    concurrent_delete_state.session.write().await.state = "connected";
    let concurrent_server_deletes = futures_util::future::join_all([
        crate::route_http_request(
            "DELETE",
            "/api/v0/server",
            None,
            "",
            &concurrent_delete_state,
        ),
        crate::route_http_request(
            "DELETE",
            "/api/v0/server",
            None,
            "",
            &concurrent_delete_state,
        ),
    ])
    .await;
    record!(
        "DELETE",
        "/api/v0/server",
        "concurrency-and-idempotency",
        concurrent_server_deletes.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        })
    );

    let server_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("server runtime database");
    let (server_runtime_state, _server_runtime_receiver) = test_state_with_env_parts(
        env,
        crate::SearchStore::new(),
        Some(server_runtime_db.clone()),
    );
    server_runtime_db.close_for_test().await;
    let server_runtime =
        crate::route_http_request("GET", "/api/v0/server", None, "", &server_runtime_state)
            .await
            .expect("runtime server state");
    let server_runtime_json =
        serde_json::from_str::<serde_json::Value>(&server_runtime.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/server",
        "runtime-failure-and-timeout",
        server_runtime.status == "200 OK"
            && server_runtime_json["address"] == ""
            && server_runtime_json["ipEndPoint"] == "255.255.255.255:0"
    );
    let server_status_runtime =
        crate::route_http_request("GET", "/api/server/status", None, "", &server_runtime_state)
            .await
            .expect("runtime server status");
    let server_status_runtime_json =
        serde_json::from_str::<serde_json::Value>(&server_status_runtime.body).unwrap_or_default();
    record!(
        "GET",
        "/api/server/status",
        "runtime-failure-and-timeout",
        server_status_runtime.status == "200 OK"
            && server_status_runtime_json["connected"] == false
            && server_status_runtime_json["state"] == "disconnected"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create server/session evidence directory");
    fs::write(
        evidence_dir.join("server_session_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize server/session ledger"),
    )
    .expect("write server/session ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api server/session mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
