//! Controller full controller api differential fixtures ownership.

use super::*;

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_controller_residual_core_contracts_impl() {
    let target = "slskd";
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

    // The bare controller profile reports that authentication is not
    // required.  The same route reports true once the real API-key gate
    // is enabled and the request carries the configured credential.
    let (default_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let default_enabled =
        crate::route_http_request("GET", "/api/v0/session/enabled", None, "", &default_state)
            .await
            .expect("default session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "missing-empty-or-conflict-state",
        default_enabled.status == "200 OK"
            && default_enabled.content_type == "application/json"
            && default_enabled.body == "false"
    );
    let malformed_enabled = crate::route_http_request(
        "GET",
        "/api/v0/session/enabled/extra",
        None,
        "",
        &default_state,
    )
    .await
    .expect("malformed session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "malformed-path-query-or-body",
        malformed_enabled.status == "404 Not Found"
    );

    let enabled_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "residual-api-token");
    let (enabled_state, _receiver) = test_state_with_env(enabled_env);
    let enabled = crate::route_http_request(
        "GET",
        "/api/v0/session/enabled",
        Some("ApiKey residual-api-token"),
        "",
        &enabled_state,
    )
    .await
    .expect("authenticated session-enabled response");
    record!(
        "GET",
        "/api/v0/session/enabled",
        "populated-dynamic-state",
        enabled.status == "200 OK"
            && enabled.content_type == "application/json"
            && enabled.body == "true"
    );

    // The frozen slskd controller propagates a failed forced GitHub
    // version lookup. Use a local connection that closes immediately so
    // this exercises the production response path without depending on
    // the public network.
    let version_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind version failure fixture");
    let version_address = version_listener
        .local_addr()
        .expect("version failure fixture address");
    let version_server = tokio::spawn(async move {
        let _ = version_listener.accept().await;
    });
    let forced_version_failure = crate::controller_version_latest_response(
        &default_state,
        true,
        &format!("http://{version_address}/latest"),
    )
    .await;
    version_server.await.expect("version failure fixture task");
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "runtime-failure-and-timeout",
        forced_version_failure.status == "500 Internal Server Error"
            && !forced_version_failure
                .body
                .contains(&version_address.to_string())
    );

    let (user_failure_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                "user-read-failure-peer=127.0.0.1:2234",
            ),
    );
    user_failure_state
        .users
        .write()
        .await
        .records
        .push(crate::UserRecord {
            username: "user-read-failure-peer".to_owned(),
            watched: true,
            status: Some("Online".to_owned()),
            privileged: false,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: crate::unix_timestamp(),
        });
    let browse_response = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        r#"{"username":"user-read-failure-peer","entries":[{"filename":"Remote/Track.flac","size":1}]}"#,
        &user_failure_state,
    )
    .await
    .expect("seed disconnected user browse failure");
    assert_eq!(browse_response.status, "200 OK");
    for (path, route) in [
        (
            "/api/v0/users/user-read-failure-peer/info",
            "/api/v0/users/{username}/info",
        ),
        (
            "/api/v0/users/user-read-failure-peer/status",
            "/api/v0/users/{username}/status",
        ),
        (
            "/api/v0/users/user-read-failure-peer/endpoint",
            "/api/v0/users/{username}/endpoint",
        ),
        (
            "/api/v0/users/user-read-failure-peer/browse",
            "/api/v0/users/{username}/browse",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &user_failure_state)
            .await
            .expect("disconnected user read failure response");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && !response.body.contains("user-read-failure-peer")
        );
    }

    // A successful login has a real observable side effect: its returned
    // administrator JWT authorizes the next controller request.  Two
    // concurrent valid logins remain independent, and a fresh state
    // instance can issue another token from the same configured profile.
    let login_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "residual-login-token")
        .with("SLSKD_USERNAME", "residual-admin")
        .with("SLSKD_PASSWORD", "residual-password");
    let (login_state, _receiver) = test_state_with_env(login_env.clone());
    let login_body = r#"{"username":"residual-admin","password":"residual-password"}"#;
    let login =
        crate::route_http_request("POST", "/api/v0/session", None, login_body, &login_state)
            .await
            .expect("residual login");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    let login_token = login_json["token"].as_str().unwrap_or_default().to_owned();
    let authorized = crate::route_http_request(
        "GET",
        "/api/v0/session/enabled",
        Some(&format!("Bearer {login_token}")),
        "",
        &login_state,
    )
    .await
    .expect("JWT-authorized residual session-enabled request");
    record!(
        "POST",
        "/api/v0/session",
        "mutation-side-effects-and-readback",
        login.status == "200 OK"
            && login_json["tokenType"] == "Bearer"
            && login_token.split('.').count() == 3
            && authorized.status == "200 OK"
            && authorized.body == "true"
    );

    let reloaded_state = test_state_with_env(login_env.clone()).0;
    let reloaded_login =
        crate::route_http_request("POST", "/api/v0/session", None, login_body, &reloaded_state)
            .await
            .expect("reloaded residual login");
    record!(
        "POST",
        "/api/v0/session",
        "restart-persistence-or-reset",
        reloaded_login.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&reloaded_login.body)
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

    // YAML validation is intentionally non-mutating.  The empty request
    // is a real missing-body rejection; valid concurrent validations must
    // leave the durable file byte-for-byte unchanged.
    let options_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_REMOTE_CONFIGURATION", "true")
        .with("SLSKR_NO_CONFIG_WATCH", "true");
    let (options_state, _receiver) = test_state_with_env(options_env.clone());
    let options_path = options_state.config.state_dir.join("slskd.yml");
    let original_yaml = b"soulseek:\n  description: residual-validation\n";
    fs::write(&options_path, original_yaml).expect("write residual YAML fixture");
    let missing_validation = crate::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        "",
        &options_state,
    )
    .await
    .expect("missing YAML validation body");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "missing-empty-or-conflict-state",
        missing_validation.status == "400 Bad Request"
    );
    let before_validation = fs::read(&options_path).expect("read YAML before validation");
    let valid_yaml = serde_json::to_string("soulseek:\n  description: validated\n").unwrap();
    let validation = crate::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        &valid_yaml,
        &options_state,
    )
    .await
    .expect("valid YAML validation");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "mutation-side-effects-and-readback",
        validation.status == "200 OK"
            && validation.body.is_empty()
            && fs::read(&options_path).ok().as_deref() == Some(before_validation.as_slice())
    );
    let concurrent_validations = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &valid_yaml,
            &options_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &valid_yaml,
            &options_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "concurrency-and-idempotency",
        concurrent_validations.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && fs::read(&options_path).ok().as_deref() == Some(original_yaml)
    );
    let reloaded_options = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with(
            "SLSKR_STATE_DIR",
            options_state
                .config
                .state_dir
                .to_str()
                .expect("options state path"),
        ),
    )
    .expect("reload residual validation state");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "restart-persistence-or-reset",
        reloaded_options.state_dir == options_state.config.state_dir
            && fs::read(&options_path).ok().as_deref() == Some(original_yaml)
    );

    // YAML update rejects both an absent body and malformed JSON/YAML,
    // performs atomic concurrent replacement, and exposes a real 500
    // when its configured state directory is a regular file.
    let malformed_update =
        crate::route_http_request("PUT", "/api/v0/options/yaml", None, "{", &options_state)
            .await
            .expect("malformed YAML update");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "malformed-path-query-or-body",
        malformed_update.status == "400 Bad Request"
    );
    let missing_update =
        crate::route_http_request("PUT", "/api/v0/options/yaml", None, "", &options_state)
            .await
            .expect("missing YAML update body");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "missing-empty-or-conflict-state",
        missing_update.status == "400 Bad Request"
    );
    let update_a = serde_json::to_string("soulseek:\n  description: update-a\n").unwrap();
    let update_b = serde_json::to_string("soulseek:\n  description: update-b\n").unwrap();
    let concurrent_updates = futures_util::future::join_all([
        crate::route_http_request(
            "PUT",
            "/api/v0/options/yaml",
            None,
            &update_a,
            &options_state,
        ),
        crate::route_http_request(
            "PUT",
            "/api/v0/options/yaml",
            None,
            &update_b,
            &options_state,
        ),
    ])
    .await;
    let final_yaml = fs::read_to_string(&options_path).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "concurrency-and-idempotency",
        concurrent_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty()))
            && [
                "soulseek:\n  description: update-a\n",
                "soulseek:\n  description: update-b\n",
            ]
            .contains(&final_yaml.as_str())
    );
    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-residual-options-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file").expect("create state conflict");
    let mut conflict_state = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true"),
    )
    .0;
    Arc::get_mut(&mut conflict_state)
        .expect("exclusive conflict state")
        .config
        .state_dir = conflict_root.clone();
    let failed_update = crate::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        &update_a,
        &conflict_state,
    )
    .await
    .expect("YAML update filesystem failure");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "runtime-failure-and-timeout",
        failed_update.status == "500 Internal Server Error"
            && !failed_update.body.contains("state directory is a file")
    );
    let _ = fs::remove_file(conflict_root);

    // PATCH is volatile but still has a real serialization/idempotency
    // contract.  Both writes must complete and the final projection must
    // be one complete normalized overlay, never a torn JSON value.
    let patch_a = r#"{"soulseek":{"listenPort":50321}}"#;
    let patch_b = r#"{"soulseek":{"listenPort":50322}}"#;
    let concurrent_patches = futures_util::future::join_all([
        crate::route_http_request("PATCH", "/api/v0/options", None, patch_a, &options_state),
        crate::route_http_request("PATCH", "/api/v0/options", None, patch_b, &options_state),
    ])
    .await;
    let options_readback =
        crate::route_http_request("GET", "/api/v0/options", None, "", &options_state)
            .await
            .expect("options patch readback");
    let options_readback_json =
        serde_json::from_str::<serde_json::Value>(&options_readback.body).unwrap_or_default();
    record!(
        "PATCH",
        "/api/v0/options",
        "concurrency-and-idempotency",
        concurrent_patches.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
            && matches!(
                options_readback_json["soulseek"]["listenPort"].as_u64(),
                Some(50321 | 50322)
            )
    );

    // Application restart requests are idempotent even when two real
    // callers race to set the same durable runtime flag.
    let (application_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
    );
    let concurrent_restarts = futures_util::future::join_all([
        crate::route_http_request("PUT", "/api/v0/application", None, "{}", &application_state),
        crate::route_http_request("PUT", "/api/v0/application", None, "{}", &application_state),
    ])
    .await;
    record!(
        "PUT",
        "/api/v0/application",
        "concurrency-and-idempotency",
        concurrent_restarts.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content" && response.body.is_empty()))
            && application_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    // Share rescans expose both the real persistence failure rollback and
    // the semaphore-backed concurrent-scan rejection.
    let share_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("residual share database");
    let (share_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(share_db.clone()),
    );
    let previous_shares = share_failure_state.shares.read().await.clone();
    share_db.close_for_test().await;
    let failed_share_scan =
        crate::route_http_request("PUT", "/api/v0/shares", None, "", &share_failure_state)
            .await
            .expect("share persistence failure response");
    record!(
        "PUT",
        "/api/v0/shares",
        "runtime-failure-and-timeout",
        failed_share_scan.status == "503 Service Unavailable"
            && share_failure_state.shares.read().await.json() == previous_shares.json()
    );
    let (share_busy_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let _scan_permit = Arc::clone(&share_busy_state.share_scans)
        .acquire_owned()
        .await
        .expect("hold share scan permit");
    let busy_share_scan =
        crate::route_http_request("PUT", "/api/v0/shares", None, "", &share_busy_state)
            .await
            .expect("busy share scan response");
    record!(
        "PUT",
        "/api/v0/shares",
        "concurrency-and-idempotency",
        busy_share_scan.status == "503 Service Unavailable"
            && busy_share_scan
                .body
                .contains("share scan already in progress")
    );

    // A failed share scan is a real repository/filesystem fault state.
    // Frozen BrowseAsync enumerates the backing repositories directly, so
    // an equivalent browse after that fault reaches the controller's 500
    // middleware instead of returning a stale directory projection.
    let (browse_failure_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let browse_root_id = {
        let mut shares = browse_failure_state.shares.write().await;
        if shares.roots.is_empty() {
            let files = shares.entries.len();
            let bytes = shares.entries.iter().map(|entry| entry.size).sum();
            shares.roots.push(crate::ShareRoot {
                label: "shares".to_owned(),
                local_path: std::env::temp_dir(),
                raw: "shares".to_owned(),
                directories: 0,
                files,
                bytes,
                extensions: Vec::new(),
                statistics_ready: true,
            });
        }
        let root_id = crate::share_root_id(&shares.roots[0].label);
        shares
            .scan_errors
            .push("repository browse failed".to_owned());
        root_id
    };
    let failed_browse_all = crate::route_http_request(
        "GET",
        "/api/v0/shares/contents",
        None,
        "",
        &browse_failure_state,
    )
    .await
    .expect("share browse-all failure response");
    record!(
        "GET",
        "/api/v0/shares/contents",
        "runtime-failure-and-timeout",
        failed_browse_all.status == "500 Internal Server Error"
            && !failed_browse_all.body.contains("repository browse failed")
    );
    let failed_browse_share = crate::route_http_request(
        "GET",
        &format!("/api/v0/shares/{browse_root_id}/contents"),
        None,
        "",
        &browse_failure_state,
    )
    .await
    .expect("share browse-share failure response");
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "runtime-failure-and-timeout",
        failed_browse_share.status == "500 Internal Server Error"
            && !failed_browse_share
                .body
                .contains("repository browse failed")
    );

    // The frozen RoomsController fetches the room list from the live
    // Soulseek client. A disconnected session therefore reaches the
    // framework's 500 path instead of serving the last cached projection.
    let (available_rooms_failure_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let available_rooms_failure = crate::route_http_request(
        "GET",
        "/api/v0/rooms/available",
        None,
        "",
        &available_rooms_failure_state,
    )
    .await
    .expect("available rooms disconnected failure response");
    record!(
        "GET",
        "/api/v0/rooms/available",
        "runtime-failure-and-timeout",
        available_rooms_failure.status == "500 Internal Server Error"
            && available_rooms_failure
                .body
                .contains("failed to retrieve available rooms")
    );

    // Frozen ConversationsController reads through its EF-backed
    // messaging service for all three GET shapes. Exercise the same
    // failure through the live HTTP stream after closing a real SQLite
    // manager; the memory projection must not hide a failed storage read.
    async fn live_controller_get(state: Arc<crate::AppState>, path: &str) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(crate::handle_http_stream(server, None, false, state));
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        client
            .write_all(request.as_bytes())
            .await
            .expect("write conversation read failure request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read conversation read failure response");
        task.await
            .expect("conversation read failure HTTP task")
            .expect("conversation read failure HTTP response");
        response
    }

    let conversation_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("conversation read failure database");
    let (conversation_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(conversation_db.clone()),
    );
    conversation_failure_state.messages.write().await.add(
        "conversation-read-failure-peer".to_owned(),
        "inbound",
        "persisted conversation read failure".to_owned(),
    );
    conversation_db.close_for_test().await;
    for (path, route) in [
        ("/api/v0/conversations", "/api/v0/conversations"),
        (
            "/api/v0/conversations/conversation-read-failure-peer",
            "/api/v0/conversations/{username}",
        ),
        (
            "/api/v0/conversations/conversation-read-failure-peer/messages",
            "/api/v0/conversations/{username}/messages",
        ),
    ] {
        let response = live_controller_get(Arc::clone(&conversation_failure_state), path).await;
        let response = String::from_utf8_lossy(&response);
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.starts_with("HTTP/1.1 500 Internal Server Error")
        );
    }

    // Frozen slskd transfer reports query SQLite through the telemetry
    // report service. A closed store therefore reaches the controller's
    // generic 500 middleware; serving the in-memory transfer projection
    // would incorrectly hide that repository failure.
    let telemetry_report_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("telemetry report failure database");
    let (telemetry_report_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(telemetry_report_db.clone()),
    );
    telemetry_report_failure_state.session.write().await.state = "connected";
    {
        let mut transfers = telemetry_report_failure_state.transfers.write().await;
        transfers.create(
            0,
            Some("transfer-read-failure-peer".to_owned()),
            "Remote/Failure.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.create(
            1,
            Some("transfer-read-failure-peer".to_owned()),
            "Upload/Failure.flac".to_owned(),
            None,
            Some(1),
        );
    }
    let search_read_failure_id = "33333333-3333-4333-8333-333333333333";
    let search_created = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        &format!(r#"{{"id":"{search_read_failure_id}","searchText":"storage failure"}}"#),
        &telemetry_report_failure_state,
    )
    .await
    .expect("seed search response read failure");
    assert_eq!(search_created.status, "200 OK");
    telemetry_report_db.close_for_test().await;
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
            "/api/v0/telemetry/reports/transfers/users/telemetry-report-failure-peer",
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
        let response = crate::route_http_request(
            "GET",
            path,
            None,
            "",
            &telemetry_report_failure_state,
        )
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

    for (path, route) in [
        ("/api/v0/transfers/downloads", "/api/v0/transfers/downloads"),
        ("/api/v0/transfers/uploads", "/api/v0/transfers/uploads"),
        (
            "/api/v0/transfers/downloads/transfer-read-failure-peer",
            "/api/v0/transfers/downloads/{username}",
        ),
        (
            "/api/v0/transfers/uploads/transfer-read-failure-peer",
            "/api/v0/transfers/uploads/{username}",
        ),
        (
            "/api/v0/transfers/downloads/batches/00000000-0000-4000-8000-000000000000",
            "/api/v0/transfers/downloads/batches/{id}",
        ),
    ] {
        let response =
            crate::route_http_request("GET", path, None, "", &telemetry_report_failure_state)
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

    let events_failure = crate::route_http_request(
        "GET",
        "/api/v0/events?limit=1",
        None,
        "",
        &telemetry_report_failure_state,
    )
    .await
    .expect("events read failure response");
    record!(
        "GET",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        events_failure.status == "500 Internal Server Error"
            && events_failure.body.contains("event storage unavailable")
    );
    let search_responses_failure = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{search_read_failure_id}/responses"),
        None,
        "",
        &telemetry_report_failure_state,
    )
    .await
    .expect("search responses read failure response");
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "runtime-failure-and-timeout",
        search_responses_failure.status == "500 Internal Server Error"
            && search_responses_failure
                .body
                .contains("search storage unavailable")
    );

    // Room subresources are real stateful projections: absent rooms are
    // 404, duplicate member insertion is idempotent, while messages and
    // ticker updates serialize under the room store write lock.
    let (room_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    room_state.session.write().await.state = "connected";
    let joined = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined",
        None,
        r#""residual-room""#,
        &room_state,
    )
    .await
    .expect("join residual room");
    assert_eq!(joined.status, "201 Created");
    for (route, body) in [
        ("/api/v0/rooms/joined/missing-room/members", r#""member""#),
        ("/api/v0/rooms/joined/missing-room/messages", r#""message""#),
        ("/api/v0/rooms/joined/missing-room/ticker", r#""ticker""#),
    ] {
        let response = crate::route_http_request("POST", route, None, body, &room_state)
            .await
            .unwrap_or_else(|error| panic!("POST {route}: {error}"));
        let route_template = if route.ends_with("/members") {
            "/api/v0/rooms/joined/{roomName}/members"
        } else if route.ends_with("/messages") {
            "/api/v0/rooms/joined/{roomName}/messages"
        } else {
            "/api/v0/rooms/joined/{roomName}/ticker"
        };
        record!(
            "POST",
            route_template,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    let member_updates = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/members",
            None,
            r#""same-member""#,
            &room_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/members",
            None,
            r#""same-member""#,
            &room_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/members",
        "concurrency-and-idempotency",
        member_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && room_state.rooms.read().await.records[0]
                .members
                .iter()
                .filter(|member| member == &&"same-member".to_owned())
                .count()
                == 1
    );
    let message_updates = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/messages",
            None,
            r#""message-a""#,
            &room_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/messages",
            None,
            r#""message-b""#,
            &room_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "concurrency-and-idempotency",
        message_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && room_state.rooms.read().await.records[0].messages.len() >= 2
    );
    let ticker_updates = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/ticker",
            None,
            r#""ticker-a""#,
            &room_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/rooms/joined/residual-room/ticker",
            None,
            r#""ticker-b""#,
            &room_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/ticker",
        "concurrency-and-idempotency",
        ticker_updates.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && room_state.rooms.read().await.records[0]
                .ticker
                .as_deref()
                .is_some_and(|ticker| matches!(ticker, "ticker-a" | "ticker-b"))
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_residual_core_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd residual core ledger"),
    )
    .expect("write slskd residual core ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd residual core mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
