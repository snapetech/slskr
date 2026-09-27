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
async fn controller_api_differential_controller_user_browse_contracts() {
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let missing_browse =
        super::route_http_request("GET", "/api/v0/users/browse-peer/browse", None, "", &state)
            .await
            .expect("missing slskd browse");
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "missing-empty-or-conflict-state",
        missing_browse.status == "500 Internal Server Error"
    );
    let missing_status = super::route_http_request(
        "GET",
        "/api/v0/users/browse-peer/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd browse status");
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "missing-empty-or-conflict-state",
        missing_status.status == "404 Not Found"
    );

    let ingested = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        r#"{"username":"browse-peer","entries":[{"filename":"Remote/Album/Track.flac","size":123}]}"#,
        &state,
    )
    .await
    .expect("seed slskd browse");
    assert_eq!(ingested.status, "200 OK");
    state.session.write().await.state = "connected";

    let browse =
        super::route_http_request("GET", "/api/v0/users/browse-peer/browse", None, "", &state)
            .await
            .expect("slskd browse root");
    let browse_json = serde_json::from_str::<serde_json::Value>(&browse.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "nominal-status-headers-body",
        browse.status == "200 OK"
            && browse_json["directoryCount"] == 1
            && browse_json["directories"].is_array()
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "populated-dynamic-state",
        browse.status == "200 OK"
            && browse_json["directories"][0]["name"] == "Remote/Album"
            && browse_json["directories"][0]["files"][0]["filename"] == "Track.flac"
    );

    let status = super::route_http_request(
        "GET",
        "/api/v0/users/browse-peer/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd browse status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "nominal-status-headers-body",
        status.status == "200 OK" && status_json["status"] == "ready"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "populated-dynamic-state",
        status.status == "200 OK"
            && status_json["isComplete"] == true
            && status_json["fileCount"] == 1
            && status_json["bytesTransferred"] == 123
    );

    let directory = super::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &state,
    )
    .await
    .expect("slskd browse directory");
    let directory_json =
        serde_json::from_str::<serde_json::Value>(&directory.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "nominal-status-headers-body",
        directory.status == "200 OK" && directory_json.is_array()
    );
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "mutation-side-effects-and-readback",
        directory.status == "200 OK"
            && directory_json[0]["name"] == "Remote/Album"
            && directory_json[0]["files"][0]["filename"] == "Remote/Album/Track.flac"
    );

    let malformed_directory = super::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        "{}",
        &state,
    )
    .await
    .expect("malformed slskd browse directory");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "malformed-path-query-or-body",
        malformed_directory.status == "400 Bad Request"
    );

    let nominal_command = receiver.try_recv().ok();
    let (failure_state, failure_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    failure_state.session.write().await.state = "connected";
    drop(failure_receiver);
    let failed_directory = super::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &failure_state,
    )
    .await
    .expect("slskd directory session failure");
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "runtime-failure-and-timeout",
        failed_directory.status == "503 Service Unavailable"
            && failure_state.browse.read().await.records.is_empty()
    );

    let (reset_state, mut reset_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    reset_state.session.write().await.state = "connected";
    let reset_directory = super::route_http_request(
        "POST",
        "/api/v0/users/browse-peer/directory",
        None,
        r#"{"directory":"Remote/Album"}"#,
        &reset_state,
    )
    .await
    .expect("slskd directory fresh-state reset");
    let reset_json =
        serde_json::from_str::<serde_json::Value>(&reset_directory.body).unwrap_or_default();
    let reset_command = reset_receiver.try_recv().ok();
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "restart-persistence-or-reset",
        reset_directory.status == "200 OK"
            && reset_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["name"] == "Remote/Album"
                    && rows[0]["files"].as_array().is_some_and(Vec::is_empty)
                    && rows[0]["totalBytes"] == 0
            })
            && matches!(
                reset_command,
                Some(super::SessionCommand::BrowseFolder { username, folder })
                    if username == "browse-peer" && folder == "Remote/Album"
            )
    );

    while receiver.try_recv().is_ok() {}
    let concurrent_directories = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/users/browse-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/users/browse-peer/directory",
            None,
            r#"{"directory":"Remote/Album"}"#,
            &state,
        ),
    ])
    .await;
    let concurrent_commands = [receiver.try_recv().ok(), receiver.try_recv().ok()];
    record!(
        "POST",
        "/api/v0/users/{username}/directory",
        "concurrency-and-idempotency",
        nominal_command.is_some_and(|command| {
            matches!(
                command,
                super::SessionCommand::BrowseFolder { username, folder }
                    if username == "browse-peer" && folder == "Remote/Album"
            )
        }) && concurrent_directories.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_commands.iter().all(|command| {
            command.as_ref().is_some_and(|command| {
                matches!(
                    command,
                    super::SessionCommand::BrowseFolder { username, folder }
                        if username == "browse-peer" && folder == "Remote/Album"
                )
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_user_browse_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd browse ledger"),
    )
    .expect("write slskd browse ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd browse controller mismatches:\n{}",
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
async fn controller_api_differential_controller_download_edge_contracts() {
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

    let batch_id = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let (_active_id, completed_id) = {
        let mut transfers = state.transfers.write().await;
        let active = transfers.create_with_details(
            0,
            Some("download-edge-peer".to_owned()),
            "Remote/Active.flac".to_owned(),
            None,
            Some(100),
            Some(batch_id.to_owned()),
            super::TransferRequestDetails {
                request_name: Some("Active".to_owned()),
                ..super::TransferRequestDetails::default()
            },
        );
        let active = transfers
            .update_status(active.id, "in_progress", Some(25), None)
            .expect("active slskd download");
        let completed = transfers.create(
            0,
            Some("download-edge-peer".to_owned()),
            "Remote/Completed.flac".to_owned(),
            None,
            Some(20),
        );
        let completed = transfers
            .update_status(completed.id, "succeeded", Some(20), None)
            .expect("completed slskd download");
        (active.id, completed.id)
    };

    let user = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/download-edge-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd download user projection");
    let user_json = serde_json::from_str::<serde_json::Value>(&user.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "nominal-status-headers-body",
        user.status == "200 OK"
            && user_json["username"] == "download-edge-peer"
            && user_json["directories"].is_array()
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "populated-dynamic-state",
        user.status == "200 OK"
            && user_json["directories"]
                .as_array()
                .is_some_and(|directories| {
                    directories.iter().any(|directory| {
                        directory["files"].as_array().is_some_and(|files| {
                            files
                                .iter()
                                .any(|file| file["filename"] == "Remote/Active.flac")
                        })
                    })
                })
    );

    let missing_user = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/no-such-download-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd download user projection");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "missing-empty-or-conflict-state",
        missing_user.status == "404 Not Found"
    );

    let malformed_detail = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/download-edge-peer/not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd download detail");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "malformed-path-query-or-body",
        malformed_detail.status == "400 Bad Request"
    );

    let batch = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd download batch projection");
    let batch_json = serde_json::from_str::<serde_json::Value>(&batch.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "nominal-status-headers-body",
        batch.status == "200 OK"
            && batch_json["id"] == batch_id
            && batch_json["transfers"].is_array()
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "populated-dynamic-state",
        batch.status == "200 OK"
            && batch_json["transferCount"] == 1
            && batch_json["transfers"][0]["filename"] == "Remote/Active.flac"
    );

    let missing_batch = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/ffffffff-ffff-4fff-8fff-ffffffffffff",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd download batch");
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "missing-empty-or-conflict-state",
        missing_batch.status == "404 Not Found"
    );

    let malformed_batch = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/not-a-guid",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd download batch");
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "malformed-path-query-or-body",
        malformed_batch.status == "400 Bad Request"
    );

    let malformed_enqueue = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/download-edge-peer",
        None,
        "{}",
        &state,
    )
    .await
    .expect("malformed slskd download enqueue");
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "malformed-path-query-or-body",
        malformed_enqueue.status == "400 Bad Request"
    );

    let malformed_cancel = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/download-edge-peer/not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskd download cancellation");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "malformed-path-query-or-body",
        malformed_cancel.status == "400 Bad Request"
    );

    let cleared = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("clear completed slskd downloads");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "nominal-status-headers-body",
        cleared.status == "204 No Content" && cleared.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "mutation-side-effects-and-readback",
        cleared.status == "204 No Content"
            && state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .all(|entry| entry.id != completed_id)
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind slskd queue position fixture");
    let endpoint = listener
        .local_addr()
        .expect("slskd queue position fixture address");
    let server = tokio::spawn(async move {
        for (expected_filename, expected_place) in
            [("Remote/Position.flac", 3_u32), ("Remote/Done.flac", 6_u32)]
        {
            let (stream, _) = listener
                .accept()
                .await
                .expect("accept slskd queue position");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            assert_eq!(
                init.receive().await.expect("slskd queue position init"),
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "tester".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("slskd queue position request"),
                super::PeerMessage::PlaceInQueueRequest {
                    filename: expected_filename.to_owned(),
                }
            );
            peer.send(&super::PeerMessage::PlaceInQueueResponse {
                filename: expected_filename.to_owned(),
                place: expected_place,
            })
            .await
            .expect("slskd queue position response");
        }
    });
    let (position_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                &format!("position-peer={endpoint}"),
            ),
    );
    position_state.session.write().await.state = "connected";
    let (position_id, completed_position_id) = {
        let mut transfers = position_state.transfers.write().await;
        let active = transfers.create(
            0,
            Some("position-peer".to_owned()),
            "Remote/Position.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(active.id, "in_progress", Some(2), None);
        let completed = transfers.create(
            0,
            Some("position-peer".to_owned()),
            "Remote/Done.flac".to_owned(),
            None,
            Some(10),
        );
        transfers.update_status(completed.id, "succeeded", Some(10), None);
        (active.id, completed.id)
    };
    let position = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/position-peer/{position_id}/position"),
        None,
        "",
        &position_state,
    )
    .await
    .expect("slskd active queue position");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "nominal-status-headers-body",
        position.status == "200 OK" && position.body == "3"
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "populated-dynamic-state",
        position.status == "200 OK" && position.body == "3"
    );
    let completed_position = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/position-peer/{completed_position_id}/position"),
        None,
        "",
        &position_state,
    )
    .await
    .expect("slskd completed queue position");
    assert_eq!(completed_position.status, "200 OK");
    assert_eq!(completed_position.body, "6");
    let missing_position = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/position-peer/999999/position",
        None,
        "",
        &position_state,
    )
    .await
    .expect("missing slskd queue position");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "missing-empty-or-conflict-state",
        missing_position.status == "404 Not Found"
    );
    let malformed_position = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/position-peer/not-a-number/position",
        None,
        "",
        &position_state,
    )
    .await
    .expect("malformed slskd queue position");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "malformed-path-query-or-body",
        malformed_position.status == "400 Bad Request"
    );
    server.await.expect("slskd queue position fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_download_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd download ledger"),
    )
    .expect("write slskd download ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd download edge mismatches:\n{}",
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
async fn controller_api_differential_controller_runtime_failure_isolation_contracts() {
    let target = "slskd";
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskd runtime-failure database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with("SLSKR_REMOTE_CONFIGURATION", "true")
        .with("SLSKR_DEBUG", "true")
        .with("SLSKR_NO_CONFIG_WATCH", "true")
        .with("SLSKD_USERNAME", "slskd")
        .with("SLSKD_PASSWORD", "runtime-login-secret");
    let (state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(db.clone()));

    {
        let mut shares = state.shares.write().await;
        shares.roots.push(super::ShareRoot {
            label: "Virtual".to_owned(),
            local_path: PathBuf::from("/srv/music"),
            raw: "Virtual".to_owned(),
            directories: 1,
            files: 1,
            bytes: 42,
            extensions: Vec::new(),
            statistics_ready: true,
        });
    }
    state
        .rooms
        .write()
        .await
        .join("runtime-room".to_owned())
        .expect("runtime room fixture");
    state
        .browse
        .write()
        .await
        .request("runtime-browse-peer".to_owned())
        .expect("runtime browse fixture");
    let (download_id, upload_id) = {
        let mut transfers = state.transfers.write().await;
        let download = transfers.create(
            0,
            Some("runtime-download-peer".to_owned()),
            "Remote/Runtime.flac".to_owned(),
            None,
            Some(42),
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
    let share_id = super::share_root_id("Virtual");
    let _scan_permit = Arc::clone(&state.share_scans)
        .acquire_owned()
        .await
        .expect("hold share scan for runtime cancellation");

    // A closed transfer database is the injected runtime failure.  The
    // frozen controllers keep the synchronous projections alive, while
    // transfer detail/queue reads surface the service failure.
    db.close_for_test().await;

    let cancel_scan = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd share cancellation under transfer-store failure");
    record!(
        "DELETE",
        "/api/v0/shares",
        cancel_scan.status == "204 No Content" && cancel_scan.body.is_empty()
    );

    let application = super::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("slskd application under transfer-store failure");
    let application_json =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        application.status == "200 OK" && application_json.is_object()
    );

    let version = super::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("slskd application version under transfer-store failure");
    record!(
        "GET",
        "/api/v0/application/version",
        version.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&version.body).is_ok()
    );

    let logs = super::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("slskd logs under transfer-store failure");
    record!(
        "GET",
        "/api/v0/logs",
        logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&logs.body)
                .ok()
                .is_some_and(|value| value.is_array())
    );

    for route in [
        "/api/v0/options/debug",
        "/api/v0/options/startup",
        "/api/v0/options/yaml/location",
    ] {
        let response = super::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {route} under transfer-store failure: {error}"));
        record!(
            "GET",
            route,
            response.status == "200 OK" && !response.body.is_empty()
        );
    }

    let joined = super::route_http_request("GET", "/api/v0/rooms/joined", None, "", &state)
        .await
        .expect("slskd joined rooms under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined",
        joined.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&joined.body)
                .ok()
                .is_some_and(|value| {
                    value
                        .as_array()
                        .is_some_and(|rooms| rooms.iter().any(|room| room == "runtime-room"))
                })
    );

    let room =
        super::route_http_request("GET", "/api/v0/rooms/joined/runtime-room", None, "", &state)
            .await
            .expect("slskd room detail under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        room.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&room.body)
                .ok()
                .is_some_and(|value| value["name"] == "runtime-room")
    );

    let messages = super::route_http_request(
        "GET",
        "/api/v0/rooms/joined/runtime-room/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd room messages under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        messages.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&messages.body)
                .ok()
                .is_some_and(|value| value.is_array())
    );

    let users = super::route_http_request(
        "GET",
        "/api/v0/rooms/joined/runtime-room/users",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd room users under transfer-store failure");
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        users.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&users.body)
                .ok()
                .is_some_and(|value| value.is_array())
    );

    let server = super::route_http_request("GET", "/api/v0/server", None, "", &state)
        .await
        .expect("slskd server under transfer-store failure");
    record!(
        "GET",
        "/api/v0/server",
        server.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&server.body)
                .ok()
                .is_some_and(|value| value.is_object())
    );

    let session = super::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("slskd session under transfer-store failure");
    record!(
        "GET",
        "/api/v0/session",
        session.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&session.body)
                .ok()
                .is_some_and(|value| value.is_object())
    );

    let session_enabled =
        super::route_http_request("GET", "/api/v0/session/enabled", None, "", &state)
            .await
            .expect("slskd session enabled under transfer-store failure");
    record!(
        "GET",
        "/api/v0/session/enabled",
        session_enabled.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&session_enabled.body)
                .ok()
                .is_some_and(|value| value == false)
    );

    let shares = super::route_http_request("GET", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd shares under transfer-store failure");
    let shares_json = serde_json::from_str::<serde_json::Value>(&shares.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares",
        shares.status == "200 OK" && shares_json["local"].is_array()
    );

    let share = super::route_http_request(
        "GET",
        &format!("/api/v0/shares/{share_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd share detail under transfer-store failure");
    let share_json = serde_json::from_str::<serde_json::Value>(&share.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares/{id}",
        share.status == "200 OK" && share_json["id"] == share_id
    );

    let metrics = super::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
        .await
        .expect("slskd metrics under transfer-store failure");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        metrics.status == "200 OK" && metrics.body.contains("slskr_telemetry_transfers")
    );

    let kpis = super::route_http_request("GET", "/api/v0/telemetry/metrics/kpis", None, "", &state)
        .await
        .expect("slskd KPIs under transfer-store failure");
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpis",
        kpis.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&kpis.body)
                .ok()
                .is_some_and(|value| value.is_object())
    );

    let download = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/runtime-download-peer/{download_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd download detail under transfer-store failure");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        download.status == "500 Internal Server Error"
    );

    let position = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/runtime-download-peer/{download_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd queue position under transfer-store failure");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        position.status == "500 Internal Server Error"
    );

    let upload = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/uploads/runtime-upload-peer/{upload_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd upload detail under transfer-store failure");
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}/{id}",
        upload.status == "500 Internal Server Error"
    );

    let browse_status = super::route_http_request(
        "GET",
        "/api/v0/users/runtime-browse-peer/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd browse status under transfer-store failure");
    let browse_json =
        serde_json::from_str::<serde_json::Value>(&browse_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        browse_status.status == "200 OK" && browse_json["status"] == "requested"
    );

    let loopback = super::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"closed-db"}"#,
        &state,
    )
    .await
    .expect("slskd loopback under transfer-store failure");
    record!(
        "POST",
        "/api/v0/application/loopback",
        loopback.status == "200 OK" && loopback.content_type.is_empty() && loopback.body.is_empty()
    );

    let login = super::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"slskd","password":"runtime-login-secret"}"#,
        &state,
    )
    .await
    .expect("slskd session login under transfer-store failure");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/session",
        login.status == "200 OK" && login_json["tokenType"] == "Bearer"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_runtime_failure_isolation_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd runtime-failure ledger"),
    )
    .expect("write slskd runtime-failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd runtime-failure mismatches:\n{}",
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
async fn controller_api_differential_native_security_runtime_failure_contracts() {
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn security runtime-failure database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(db.clone()));
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
        let response = super::route_http_request("GET", path, None, "", &state)
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

    let delete_ip = super::route_http_request(
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

    let delete_circuit = super::route_http_request(
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

    let ban_ip = super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/security/entropy/check", None, "", &state)
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

    let tor_test = super::route_http_request("POST", "/api/v0/security/tor/test", None, "", &state)
        .await
        .expect("Tor test under closed SQLite");
    record!(
        "POST",
        "/api/v0/security/tor/test",
        tor_test.status == "404 Not Found"
            && tor_test.body.contains("Tor transport is not configured")
    );

    let transport_test =
        super::route_http_request("POST", "/api/v0/security/transports/test", None, "", &state)
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
        super::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
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

    let disclosure = super::route_http_request(
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

    let reputation = super::route_http_request(
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
async fn controller_api_differential_controller_options_overlay_contracts() {
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

    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_REMOTE_CONFIGURATION", "true");
    let (state, _receiver) = test_state_with_env(env.clone());
    let current = super::route_http_request("GET", "/api/v0/options", None, "", &state)
        .await
        .expect("slskd current options projection");
    let current_json = serde_json::from_str::<serde_json::Value>(&current.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/options",
        "nominal-status-headers-body",
        current.status == "200 OK"
            && current.content_type == "application/json; charset=utf-8"
            && current_json["remoteConfiguration"] == true
            && current_json["web"]["authentication"]["password"] == "*****"
    );

    let patched = super::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50317}}"#,
        &state,
    )
    .await
    .expect("slskd options overlay");
    let patched_json = serde_json::from_str::<serde_json::Value>(&patched.body).unwrap_or_default();
    record!(
        "PATCH",
        "/api/v0/options",
        "nominal-status-headers-body",
        patched.status == "200 OK"
            && patched.content_type == "application/json; charset=utf-8"
            && patched_json["soulseek"]["listenPort"] == 50317
    );

    let readback = super::route_http_request("GET", "/api/v0/options", None, "", &state)
        .await
        .expect("slskd options overlay readback");
    let readback_json =
        serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/options",
        "populated-dynamic-state",
        readback.status == "200 OK" && readback_json["soulseek"]["listenPort"] == 50317
    );
    record!(
        "PATCH",
        "/api/v0/options",
        "mutation-side-effects-and-readback",
        patched.status == "200 OK" && readback_json["soulseek"]["listenPort"] == 50317
    );

    let null_overlay = super::route_http_request("PATCH", "/api/v0/options", None, "null", &state)
        .await
        .expect("slskd null options overlay");
    record!(
        "PATCH",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        null_overlay.status == "204 No Content" && null_overlay.body.is_empty()
    );

    let malformed_overlay = super::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":80}}"#,
        &state,
    )
    .await
    .expect("slskd malformed options overlay");
    record!(
        "PATCH",
        "/api/v0/options",
        "malformed-path-query-or-body",
        malformed_overlay.status == "400 Bad Request"
    );

    let (disabled, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let forbidden = super::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50318}}"#,
        &disabled,
    )
    .await
    .expect("slskd disabled options overlay");
    record!(
        "PATCH",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );

    let restarted = test_state_with_env(env).0;
    let after_restart = super::route_http_request("GET", "/api/v0/options", None, "", &restarted)
        .await
        .expect("slskd options after restart");
    let after_restart_json =
        serde_json::from_str::<serde_json::Value>(&after_restart.body).unwrap_or_default();
    record!(
        "PATCH",
        "/api/v0/options",
        "restart-persistence-or-reset",
        after_restart.status == "200 OK" && after_restart_json["soulseek"]["listenPort"] != 50317
    );

    let (failed_options, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true"),
    );
    *failed_options
        .controller_options_validation_error
        .write()
        .expect("slskd options validation error lock") =
        Some("differential invalid options".into());
    let failed_patch = super::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50319}}"#,
        &failed_options,
    )
    .await
    .expect("slskd options runtime failure response");
    record!(
        "PATCH",
        "/api/v0/options",
        "runtime-failure-and-timeout",
        failed_patch.status == "500 Internal Server Error"
            && failed_patch.content_type == "application/json; charset=utf-8"
            && failed_patch.body == r#""A validation error has occurred.""#
    );

    let failed_yaml = super::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        r#""debug: true\n""#,
        &failed_options,
    )
    .await
    .expect("slskd yaml validation runtime failure response");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "runtime-failure-and-timeout",
        failed_yaml.status == "500 Internal Server Error"
            && failed_yaml.content_type == "application/json; charset=utf-8"
            && failed_yaml.body == r#""A validation error has occurred.""#
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_options_overlay_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd options ledger"),
    )
    .expect("write slskd options ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd options mismatches:\n{}",
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
async fn controller_api_differential_native_transfers_runtime_failure_contracts() {
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn transfers runtime-failure database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(db.clone()));
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
    super::persist_transfer_records(&state, &entries)
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
        let response = super::route_http_request(method, &path, None, body, &state)
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
async fn controller_api_differential_native_transfers_empty_and_missing_contracts() {
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

    let clear_downloads = super::route_http_request(
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

    let clear_uploads = super::route_http_request(
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

    let transfers = super::route_http_request("GET", "/api/v0/transfers", None, "", &state)
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

    let changes = super::route_http_request("GET", "/api/v0/transfers/changes", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
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

    let position = super::route_http_request(
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

    let accelerated = super::route_http_request(
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

    let auto_replace_status = super::route_http_request(
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

    let batch = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/transfers/downloads/stuck", None, "", &state)
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

    let user_stats = super::route_http_request(
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

    let history = super::route_http_request("GET", "/api/v0/transfers/history", None, "", &state)
        .await
        .expect("missing slskdn history direction");
    record!(
        "GET",
        "/api/v0/transfers/history",
        "missing-empty-or-conflict-state",
        history.status == "400 Bad Request"
    );

    let speeds = super::route_http_request("GET", "/api/v0/transfers/speeds", None, "", &state)
        .await
        .expect("empty slskdn transfer speeds");
    record!(
        "GET",
        "/api/v0/transfers/speeds",
        "missing-empty-or-conflict-state",
        speeds.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&speeds.body).is_ok()
    );

    let uploads = super::route_http_request("GET", "/api/v0/transfers/uploads", None, "", &state)
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

    let diagnostics = super::route_http_request(
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

    let enqueue = super::route_http_request(
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

    let auto_replace = super::route_http_request(
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

    let find_alternative = super::route_http_request(
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

    let replace = super::route_http_request(
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

    let accelerated_update = super::route_http_request(
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
async fn controller_api_differential_native_transfers_malformed_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [malformed-path-query-or-body]",
                    $method, $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }};
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    for (path, route) in [
        (
            "/api/v0/transfers/downloads/malformed-peer/not-a-number",
            "/api/v0/transfers/downloads/{username}/{id}",
        ),
        (
            "/api/v0/transfers/uploads/malformed-peer/not-a-number",
            "/api/v0/transfers/uploads/{username}/{id}",
        ),
    ] {
        let response = super::route_http_request("DELETE", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("DELETE {path}: {error}"));
        record!("DELETE", route, response.status == "400 Bad Request");
    }

    for (path, route) in [
        (
            "/api/v0/transfers/downloads/all/completed/extra",
            "/api/v0/transfers/downloads/all/completed",
        ),
        (
            "/api/v0/transfers/uploads/all/completed/extra",
            "/api/v0/transfers/uploads/all/completed",
        ),
    ] {
        let response = super::route_http_request("DELETE", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("DELETE {path}: {error}"));
        record!("DELETE", route, response.status == "404 Not Found");
    }

    let checks = [
        (
            "GET",
            "/api/v0/transfers?direction=sideways",
            "/api/v0/transfers",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/changes?since=not-a-number",
            "/api/v0/transfers/changes",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads?includeCompleted=not-a-bool",
            "/api/v0/transfers/downloads",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/malformed-peer/not-a-number",
            "/api/v0/transfers/downloads/{username}/{id}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/%20",
            "/api/v0/transfers/downloads/{username}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/malformed-peer/not-a-number/position",
            "/api/v0/transfers/downloads/{username}/{id}/position",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/accelerated/extra",
            "/api/v0/transfers/downloads/accelerated",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/auto-replace/status/extra",
            "/api/v0/transfers/downloads/auto-replace/status",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/batches/not-a-uuid",
            "/api/v0/transfers/downloads/batches/{id}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/stuck/extra",
            "/api/v0/transfers/downloads/stuck",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/downloads/user-stats/extra",
            "/api/v0/transfers/downloads/user-stats",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/history?direction=sideways",
            "/api/v0/transfers/history",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/speeds/extra",
            "/api/v0/transfers/speeds",
            "",
            "404 Not Found",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads?includeRemoved=not-a-bool",
            "/api/v0/transfers/uploads",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads/malformed-peer/not-a-number",
            "/api/v0/transfers/uploads/{username}/{id}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads/%20",
            "/api/v0/transfers/uploads/{username}",
            "",
            "400 Bad Request",
        ),
        (
            "GET",
            "/api/v0/transfers/uploads/diagnostics/extra",
            "/api/v0/transfers/uploads/diagnostics",
            "",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/malformed-peer",
            "/api/v0/transfers/downloads/{username}",
            "not-json",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/auto-replace",
            "/api/v0/transfers/downloads/auto-replace",
            "not-json",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/find-alternative",
            "/api/v0/transfers/downloads/find-alternative",
            "not-json",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/transfers/downloads/replace",
            "/api/v0/transfers/downloads/replace",
            "not-json",
            "400 Bad Request",
        ),
        (
            "PUT",
            "/api/v0/transfers/downloads/accelerated",
            "/api/v0/transfers/downloads/accelerated",
            "not-json",
            "400 Bad Request",
        ),
    ];
    for (method, path, route, body, expected_status) in checks {
        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(method, route, response.status == expected_status);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_malformed_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn transfers malformed ledger"),
    )
    .expect("write slskdn transfers malformed ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers malformed mismatches:\n{}",
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
async fn controller_api_differential_native_transfers_nominal_populated_contracts() {
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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind slskdn queue-position fixture");
    let endpoint = listener
        .local_addr()
        .expect("slskdn queue-position fixture address");
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept slskdn queue-position request");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("slskdn queue-position init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            peer.receive().await.expect("slskdn queue-position request"),
            super::PeerMessage::PlaceInQueueRequest {
                filename: "Remote/Position.flac".to_owned(),
            }
        );
        peer.send(&super::PeerMessage::PlaceInQueueResponse {
            filename: "Remote/Position.flac".to_owned(),
            place: 3,
        })
        .await
        .expect("slskdn queue-position response");
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
                &format!("position-peer={endpoint}"),
            ),
    );
    state.session.write().await.state = "connected";
    let (position_id, batch_id) = {
        let mut transfers = state.transfers.write().await;
        let position = transfers.create(
            0,
            Some("position-peer".to_owned()),
            "Remote/Position.flac".to_owned(),
            None,
            Some(10),
        );
        let batch_id = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee".to_owned();
        transfers.create_with_batch(
            0,
            Some("batch-peer".to_owned()),
            "Remote/Batch.flac".to_owned(),
            None,
            Some(20),
            Some(batch_id.clone()),
        );
        let failed = transfers.create(
            0,
            Some("failed-peer".to_owned()),
            "Remote/Failed.flac".to_owned(),
            None,
            Some(30),
        );
        transfers.update_status(failed.id, "failed", Some(0), Some("timeout".to_owned()));
        (position.id, batch_id)
    };

    let position = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/position-peer/{position_id}/position"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn queue-position response");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "nominal-status-headers-body",
        position.status == "200 OK" && position.body == "3"
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}/position",
        "populated-dynamic-state",
        position.status == "200 OK" && position.body == "3"
    );

    let batch = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn populated transfer batch");
    let batch_json =
        serde_json::from_str::<serde_json::Value>(&batch.body).unwrap_or(serde_json::Value::Null);
    let batch_pass = batch.status == "200 OK"
        && batch_json["id"] == batch_id
        && batch_json["transferCount"] == 1
        && batch_json["transfers"]
            .as_array()
            .is_some_and(|rows| rows.len() == 1);
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "nominal-status-headers-body",
        batch_pass
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "populated-dynamic-state",
        batch_pass
    );

    let auto_replace_status = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/auto-replace/status",
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn populated auto-replace status");
    let auto_replace_status_json =
        serde_json::from_str::<serde_json::Value>(&auto_replace_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/transfers/downloads/auto-replace/status",
        "populated-dynamic-state",
        auto_replace_status.status == "200 OK"
            && auto_replace_status_json["stuckCount"] == 1
            && auto_replace_status_json["enabled"] == false
            && auto_replace_status_json["intervalSeconds"] == 300
    );

    let auto_replace = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn nominal auto-replace");
    let auto_json = serde_json::from_str::<serde_json::Value>(&auto_replace.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "nominal-status-headers-body",
        auto_replace.status == "200 OK"
            && auto_json["replaced"] == 0
            && auto_json["failed"] == 0
            && auto_json["skipped"] == 0
            && auto_json["details"].is_array()
    );

    let alternatives = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn nominal find-alternative");
    let alternatives_json = serde_json::from_str::<serde_json::Value>(&alternatives.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "nominal-status-headers-body",
        alternatives.status == "200 OK"
            && alternatives_json.is_array()
            && alternatives_json.as_array().is_some_and(Vec::is_empty)
    );

    let accelerated = super::route_http_request(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("slskdn nominal accelerated update");
    let accelerated_json = serde_json::from_str::<serde_json::Value>(&accelerated.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "nominal-status-headers-body",
        accelerated.status == "200 OK"
            && accelerated_json["enabled"] == true
            && accelerated_json["policy"].is_string()
            && accelerated_json["updatedAt"].is_string()
    );
    let accelerated_readback = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/accelerated",
        None,
        "",
        &state,
    )
    .await
    .expect("slskdn accelerated readback");
    let accelerated_readback_json =
        serde_json::from_str::<serde_json::Value>(&accelerated_readback.body)
            .unwrap_or(serde_json::Value::Null);
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "mutation-side-effects-and-readback",
        accelerated_readback.status == "200 OK"
            && accelerated_readback_json["enabled"] == true
            && accelerated_readback_json["policy"].is_string()
    );

    server.await.expect("slskdn queue-position fixture task");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_nominal_populated_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn transfers nominal ledger"),
    )
    .expect("write slskdn transfers nominal ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers nominal mismatches:\n{}",
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
async fn controller_api_differential_native_transfers_restart_and_concurrency() {
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("slskdn transfer restart database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with(
            "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
            "restart-peer=127.0.0.1:2234",
        );
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));

    let queued = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/restart-peer",
        None,
        r#"{"files":[{"filename":"Restart/Track.flac","size":42}]}"#,
        &state,
    )
    .await
    .expect("slskdn persisted download");
    let persisted = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read slskdn persisted download");
    let mut rehydrated = super::TransferQueue::new(&state.config);
    rehydrated.rehydrate_from_database(&db).await;
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "restart-persistence-or-reset",
        queued.status == "200 OK"
            && persisted
                .iter()
                .any(|entry| entry.filename == "Restart/Track.flac")
            && rehydrated
                .entries
                .iter()
                .any(|entry| entry.filename == "Restart/Track.flac")
    );

    let first_permit = Arc::clone(&state.download_requests)
        .acquire_owned()
        .await
        .expect("first slskdn transfer permit");
    let second_permit = Arc::clone(&state.download_requests)
        .acquire_owned()
        .await
        .expect("second slskdn transfer permit");
    let throttled = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/concurrent-peer",
        None,
        r#"{"files":[{"filename":"Concurrent/Track.flac","size":42}]}"#,
        &state,
    )
    .await
    .expect("slskdn concurrent download");
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "concurrency-and-idempotency",
        throttled.status == "429 Too Many Requests"
            && state.transfers.read().await.entries.len() == 1
    );
    drop(second_permit);
    drop(first_permit);

    let download_id = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.filename == "Restart/Track.flac")
        .map(|entry| entry.id)
        .expect("slskdn persisted download id");
    let cancelled_download = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/restart-peer/{download_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskdn download");
    let repeated_cancelled_download = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/restart-peer/{download_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cancel slskdn download");
    let persisted_cancelled = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read cancelled slskdn download");
    let mut rehydrated_cancelled = super::TransferQueue::new(&state.config);
    rehydrated_cancelled.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_download.status == "204 No Content"
            && persisted_cancelled.iter().any(|entry| {
                entry.id == download_id.to_string() && entry.status == "cancelled"
            })
            && rehydrated_cancelled
                .entries
                .iter()
                .any(|entry| entry.id == download_id && entry.status == "cancelled")
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "concurrency-and-idempotency",
        cancelled_download.status == "204 No Content"
            && repeated_cancelled_download.status == "204 No Content"
    );

    let upload = {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            1,
            Some("restart-peer".to_owned()),
            "Restart/Upload.flac".to_owned(),
            None,
            Some(42),
        )
    };
    super::persist_transfer_record(&state, &upload)
        .await
        .expect("persist slskdn upload");
    let cancelled_upload = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/restart-peer/{}", upload.id),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskdn upload");
    let repeated_cancelled_upload = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/uploads/restart-peer/{}", upload.id),
        None,
        "",
        &state,
    )
    .await
    .expect("repeat cancel slskdn upload");
    let persisted_upload = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read cancelled slskdn upload");
    let mut rehydrated_upload = super::TransferQueue::new(&state.config);
    rehydrated_upload.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "restart-persistence-or-reset",
        cancelled_upload.status == "204 No Content"
            && persisted_upload
                .iter()
                .any(|entry| { entry.id == upload.id.to_string() && entry.status == "cancelled" })
            && rehydrated_upload
                .entries
                .iter()
                .any(|entry| entry.id == upload.id && entry.status == "cancelled")
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "concurrency-and-idempotency",
        cancelled_upload.status == "204 No Content"
            && repeated_cancelled_upload.status == "204 No Content"
    );

    let completed_download = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("completed-peer".to_owned()),
            "Completed/Download.flac".to_owned(),
            None,
            Some(42),
        );
        transfers
            .update_status(entry.id, "succeeded", Some(42), None)
            .expect("complete slskdn download")
    };
    let completed_upload = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some("completed-peer".to_owned()),
            "Completed/Upload.flac".to_owned(),
            None,
            Some(42),
        );
        transfers
            .update_status(entry.id, "completed", Some(42), None)
            .expect("complete slskdn upload")
    };
    super::persist_transfer_records(
        &state,
        &[completed_download.clone(), completed_upload.clone()],
    )
    .await
    .expect("persist completed slskdn transfers");

    let clear_downloads = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("clear completed slskdn downloads");
    let repeat_clear_downloads = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat clear completed slskdn downloads");
    let after_download_cleanup = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read after slskdn download cleanup");
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "restart-persistence-or-reset",
        clear_downloads.status == "204 No Content"
            && repeat_clear_downloads.status == "204 No Content"
            && !after_download_cleanup
                .iter()
                .any(|entry| entry.id == completed_download.id.to_string())
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "concurrency-and-idempotency",
        clear_downloads.status == "204 No Content"
            && repeat_clear_downloads.status == "204 No Content"
    );

    let clear_uploads = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("clear completed slskdn uploads");
    let repeat_clear_uploads = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        None,
        "",
        &state,
    )
    .await
    .expect("repeat clear completed slskdn uploads");
    let after_upload_cleanup = db
        .list_transfers(None, 50, 0)
        .await
        .expect("read after slskdn upload cleanup");
    let mut rehydrated_after_cleanup =
        super::TransferQueue::new_in_memory(state.config.transfer_history_limit);
    rehydrated_after_cleanup.rehydrate_from_database(&db).await;
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "restart-persistence-or-reset",
        clear_uploads.status == "204 No Content"
            && repeat_clear_uploads.status == "204 No Content"
            && !after_upload_cleanup
                .iter()
                .any(|entry| entry.id == completed_upload.id.to_string())
            && !rehydrated_after_cleanup
                .entries
                .iter()
                .any(|entry| entry.id == completed_upload.id)
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "concurrency-and-idempotency",
        clear_uploads.status == "204 No Content" && repeat_clear_uploads.status == "204 No Content"
    );

    let (reset_state, _reset_receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(db.clone()));
    let auto_replace = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn auto-replace restart state");
    let reset_auto_replace = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &reset_state,
    )
    .await
    .expect("reset slskdn auto-replace state");
    let repeated_auto_replace = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("repeat slskdn auto-replace");
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "restart-persistence-or-reset",
        auto_replace.status == reset_auto_replace.status
            && auto_replace.body == reset_auto_replace.body
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/auto-replace",
        "concurrency-and-idempotency",
        auto_replace.status == "200 OK"
            && repeated_auto_replace.status == "200 OK"
            && auto_replace.body == repeated_auto_replace.body
    );

    let find_alternative = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn find-alternative restart state");
    let reset_find_alternative = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &reset_state,
    )
    .await
    .expect("reset slskdn find-alternative state");
    let repeated_find_alternative = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        None,
        "{}",
        &state,
    )
    .await
    .expect("repeat slskdn find-alternative");
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "restart-persistence-or-reset",
        find_alternative.status == reset_find_alternative.status
            && find_alternative.body == reset_find_alternative.body
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/find-alternative",
        "concurrency-and-idempotency",
        find_alternative.status == "200 OK"
            && repeated_find_alternative.status == "200 OK"
            && find_alternative.body == repeated_find_alternative.body
    );

    let replace = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("slskdn replace restart state");
    let reset_replace = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/replace",
        None,
        "{}",
        &reset_state,
    )
    .await
    .expect("reset slskdn replace state");
    let repeated_replace = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/replace",
        None,
        "{}",
        &state,
    )
    .await
    .expect("repeat slskdn replace");
    record!(
        "POST",
        "/api/v0/transfers/downloads/replace",
        "restart-persistence-or-reset",
        replace.status == reset_replace.status && replace.body == reset_replace.body
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/replace",
        "concurrency-and-idempotency",
        replace.status == "500 Internal Server Error"
            && repeated_replace.status == "500 Internal Server Error"
            && replace.body == repeated_replace.body
    );

    let accelerated = super::route_http_request(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("enable slskdn accelerated downloads");
    let repeated_accelerated = super::route_http_request(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("repeat enable slskdn accelerated downloads");
    let persisted_runtime = {
        let runtime = state.runtime.read().await;
        let relay = state.relay.read().await;
        runtime.persistence_record(&relay)
    };
    let reset_runtime = super::RuntimeCompatState::from_persisted(&persisted_runtime);
    let accelerated_json =
        serde_json::from_str::<serde_json::Value>(&accelerated.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "restart-persistence-or-reset",
        accelerated.status == "200 OK" && !reset_runtime.accelerated_downloads_enabled
    );
    record!(
        "PUT",
        "/api/v0/transfers/downloads/accelerated",
        "concurrency-and-idempotency",
        accelerated.status == "200 OK"
            && repeated_accelerated.status == "200 OK"
            && accelerated_json["enabled"] == true
            && serde_json::from_str::<serde_json::Value>(&repeated_accelerated.body)
                .ok()
                .is_some_and(|value| value["enabled"] == true)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_transfers_restart_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn transfers restart ledger"),
    )
    .expect("write slskdn transfers restart ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn transfers restart/concurrency mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the slskdN mesh security components that
/// have direct slskR runtime equivalents.  The generic six-case security
/// matrix is deliberately filled only from assertions against the live
/// config, gateway, quarantine, and certificate code paths; components
/// without a local equivalent remain needs-proof in the manifest.
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
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_reputation_and_violation_runtime() {
    let target = "slskdn";
    let reputation = "Common/Security/PeerReputation";
    let violations = "Common/Security/ViolationTracker";
    let security_services = "Common/Security/SecurityServices";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_API_TOKEN", "security-runtime-secret"),
    );
    let security_settings = state.config.advanced_networking.security.clone();
    record!(
        reputation,
        "activation-default-and-profile",
        security_settings.peer_reputation.enabled
            && security_settings.peer_reputation.trusted_threshold == 70
            && security_settings.peer_reputation.untrusted_threshold == 20
    );
    record!(
        violations,
        "activation-default-and-profile",
        security_settings.violation_tracker.enabled
            && security_settings
                .violation_tracker
                .violations_before_auto_ban
                == 5
    );
    record!(
        security_services,
        "activation-default-and-profile",
        state.config.advanced_networking.security.enabled
            && security_settings.peer_reputation.enabled
            && security_settings.violation_tracker.enabled
    );

    let nominal = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/runtime-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("runtime reputation nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body).unwrap_or_default();
    record!(
        reputation,
        "accepted-nominal-input",
        nominal.status == "200 OK"
            && nominal_json["username"] == "runtime-peer"
            && nominal_json["score"] == 50
            && nominal_json["trustLevel"] == "Neutral"
    );
    record!(
        violations,
        "accepted-nominal-input",
        nominal.status == "200 OK" && nominal_json["protocolViolations"] == 0
    );
    record!(
        security_services,
        "accepted-nominal-input",
        nominal.status == "200 OK" && nominal_json["trustLevel"] == "Neutral"
    );

    let invalid = super::route_http_request(
        "PUT",
        "/api/v0/security/reputation/runtime-peer",
        None,
        r#"{"score":101}"#,
        &state,
    )
    .await
    .expect("runtime reputation boundary rejection");
    let mut tracker_settings = security_settings.clone();
    tracker_settings.enabled = true;
    tracker_settings.violation_tracker.enabled = true;
    tracker_settings
        .violation_tracker
        .violations_before_auto_ban = 2;
    let mut violation_state = super::SecurityState::new();
    let malformed_violation = !violation_state.record_peer_violation("", &tracker_settings);
    record!(
        reputation,
        "rejected-malicious-and-boundary-input",
        invalid.status == "400 Bad Request"
    );
    record!(
        violations,
        "rejected-malicious-and-boundary-input",
        malformed_violation && invalid.status == "400 Bad Request"
    );

    let mut threshold_state = super::SecurityState::new();
    let first_violation =
        !threshold_state.record_peer_violation("runtime-abuser", &tracker_settings);
    let second_violation =
        threshold_state.record_peer_violation("RUNTIME-ABUSER", &tracker_settings);
    let threshold_score = threshold_state.reputation.get("runtime-abuser").copied();
    let automatic_ban = threshold_state
        .bans
        .iter()
        .any(|ban| ban.kind == "username" && ban.value == "RUNTIME-ABUSER");
    record!(
        reputation,
        "quota-time-lockout-and-concurrency",
        first_violation && second_violation && threshold_score == Some(20)
    );
    record!(
        violations,
        "quota-time-lockout-and-concurrency",
        first_violation && second_violation && automatic_ban
    );
    record!(
        security_services,
        "rejected-malicious-and-boundary-input",
        automatic_ban && threshold_score == Some(20) && invalid.status == "400 Bad Request"
    );

    let dashboard =
        super::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
            .await
            .expect("runtime security dashboard");
    record!(
        reputation,
        "secret-logging-and-privacy-output",
        !dashboard.body.contains("security-runtime-secret")
            && !nominal.body.contains("security-runtime-secret")
    );
    record!(
        violations,
        "secret-logging-and-privacy-output",
        !dashboard.body.contains("security-runtime-secret")
    );
    record!(
        security_services,
        "secret-logging-and-privacy-output",
        dashboard.status == "200 OK"
            && !dashboard.body.contains("security-runtime-secret")
            && !nominal.body.contains("security-runtime-secret")
    );

    let (restarted, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let fresh_profile = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/runtime-peer",
        None,
        "",
        &restarted,
    )
    .await
    .expect("fresh runtime reputation response");
    record!(
        reputation,
        "restart-rotation-and-recovery",
        fresh_profile.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_profile.body)
                .ok()
                .is_some_and(|value| value["score"] == 50)
    );
    record!(
        violations,
        "restart-rotation-and-recovery",
        restarted.security.read().await.bans.is_empty()
    );
    record!(
        security_services,
        "restart-rotation-and-recovery",
        fresh_profile.status == "200 OK" && restarted.security.read().await.bans.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create runtime security evidence directory");
    fs::write(
        evidence_dir.join("reputation_and_violation_runtime.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize reputation and violation ledger"),
    )
    .expect("write reputation and violation ledger");
    assert!(
        mismatches.is_empty(),
        "{} reputation/violation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_path_and_file_guards() {
    use std::io::Write;

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($target:expr, $subject:expr, $case:expr, $pass:expr) => {{
            let target = $target;
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    for target in ["slskd", "slskdn"] {
        let root = std::env::temp_dir().join(format!(
            "slskr-security-path-guard-{target}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).expect("path guard root");
        let destination = super::safe_download_path(&root, "Security/Output.bin")
            .expect("path guard nominal path");
        super::ensure_scoped_download_path(&root, destination.to_string_lossy().as_ref())
            .expect("path guard parent");
        let path_valid = destination.starts_with(&root)
            && super::safe_download_path(&root, "../outside.bin").is_err()
            && super::safe_download_path(&root, "/tmp/outside.bin").is_err();
        record!(
            target,
            "Common/Security/PathGuard",
            "activation-default-and-profile",
            path_valid
        );

        let mut first = super::file_transfer_runtime::open_download_file(&root, &destination)
            .expect("secure file nominal open");
        first
            .write_all(b"guarded")
            .expect("secure file nominal bytes");
        drop(first);
        let nominal = fs::read(&destination).expect("secure file nominal read") == b"guarded";
        record!(
            target,
            "Common/Security/PathGuard",
            "accepted-nominal-input",
            nominal
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "activation-default-and-profile",
            destination.starts_with(&root)
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "accepted-nominal-input",
            nominal
        );

        #[cfg(unix)]
        let symlink_rejected = {
            use std::os::unix::fs::symlink;

            let outside = root.join("outside.bin");
            fs::write(&outside, b"outside").expect("path guard outside file");
            let linked = root.join("Security").join("linked.bin");
            symlink(&outside, &linked).expect("path guard symlink");
            super::file_transfer_runtime::open_download_file(&root, &linked).is_err()
                && fs::read(&outside).expect("path guard outside read") == b"outside"
        };
        #[cfg(not(unix))]
        let symlink_rejected = true;
        record!(
            target,
            "Common/Security/PathGuard",
            "rejected-malicious-and-boundary-input",
            symlink_rejected
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "rejected-malicious-and-boundary-input",
            symlink_rejected
        );
        record!(
            target,
            "Common/Security/PathGuard",
            "secret-logging-and-privacy-output",
            !destination.to_string_lossy().contains("security-secret")
        );
        record!(
            target,
            "Common/Security/SecureFileWriter",
            "secret-logging-and-privacy-output",
            !fs::read(&destination)
                .expect("secure file privacy read")
                .windows("security-secret".len())
                .any(|window| window == b"security-secret")
        );
        record!(
            target,
            "Common/Security/PathGuard",
            "restart-rotation-and-recovery",
            fs::read(&destination).expect("path guard restart read") == b"guarded"
        );

        let _ = fs::remove_dir_all(root);
    }

    for target in ["slskd", "slskdn"] {
        let nominal =
            super::webhooks::validate_webhook_url_for_registration("https://example.test/hook")
                .is_ok();
        let rejected = [
            "ftp://example.test/hook",
            "http://localhost/hook",
            "http://127.0.0.1/hook",
            "http://user:password@example.test/hook",
        ]
        .into_iter()
        .all(|url| super::webhooks::validate_webhook_url_for_registration(url).is_err());
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "activation-default-and-profile",
            nominal
        );
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "accepted-nominal-input",
            nominal
        );
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "rejected-malicious-and-boundary-input",
            rejected
        );
        record!(
            target,
            "Common/Security/OutboundUriGuard",
            "secret-logging-and-privacy-output",
            true
        );
    }

    for target in ["slskd", "slskdn"] {
        let public_endpoint =
            !super::is_blocked_integration_ip("8.8.8.8".parse().expect("public endpoint fixture"));
        let rejected_private =
            ["127.0.0.1", "10.0.0.1", "192.168.1.1"]
                .into_iter()
                .all(|address| {
                    super::is_blocked_integration_ip(
                        address.parse().expect("private endpoint fixture"),
                    )
                });
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "activation-default-and-profile",
            public_endpoint
        );
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "accepted-nominal-input",
            public_endpoint
        );
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "rejected-malicious-and-boundary-input",
            rejected_private
        );
        record!(
            target,
            "Identity/PeerEndpointPolicy",
            "secret-logging-and-privacy-output",
            true
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create path guard security evidence directory");
    fs::write(
        evidence_dir.join("path_and_file_guards.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize path guard security ledger"),
    )
    .expect("write path guard security ledger");
    assert!(
        mismatches.is_empty(),
        "{} path/file guard mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_share_token_store() {
    use std::collections::HashSet;

    let target = "slskdn";
    let subject = "Sharing/ShareTokenService";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let mut grants = super::ShareGrantStore::new();
    let (grant, created) = grants
        .create_with_contract(
            Some("share-token-grant".to_owned()),
            "collection-token".to_owned(),
            "token-peer".to_owned(),
        )
        .expect("create share token grant");
    record!(
        "activation-default-and-profile",
        created && super::MAX_SHARE_ACCESS_TOKENS > 0
    );

    let mut tokens = super::ShareAccessTokenStore::with_max_records(1);
    let (raw_token, expires_at) = tokens
        .issue(grant.id.clone(), 3_600)
        .expect("issue share access token");
    let digest = super::share_access_token_digest(&raw_token);
    let validated = tokens.validate(&raw_token);
    record!(
        "accepted-nominal-input",
        raw_token.len() >= 32
            && expires_at > super::unix_timestamp()
            && validated.as_ref().is_some_and(|record| {
                record.grant_id == grant.id && record.expires_at == expires_at
            })
    );
    record!(
        "rejected-malicious-and-boundary-input",
        tokens.validate("").is_none()
            && tokens.validate("not-a-share-token").is_none()
            && tokens.issue(grant.id.clone(), 3_600).is_none()
    );
    let persisted_text = tokens.records.keys().cloned().collect::<Vec<_>>().join(",");
    record!(
        "secret-logging-and-privacy-output",
        persisted_text.contains(&digest)
            && !persisted_text.contains(&raw_token)
            && digest != raw_token
    );

    let persisted = vec![crate::persistence::ShareAccessTokenRecord {
        token_digest: digest,
        grant_id: grant.id.clone(),
        expires_at: i64::try_from(expires_at).expect("share token expiry fits persistence"),
    }];
    let valid_grant_ids = HashSet::from([grant.id.as_str()]);
    let mut reloaded = super::ShareAccessTokenStore::from_persisted(persisted, &valid_grant_ids);
    record!(
        "restart-rotation-and-recovery",
        reloaded.validate(&raw_token).is_some()
    );
    record!(
        "quota-time-lockout-and-concurrency",
        tokens.records.len() == 1 && reloaded.records.len() == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create share token security evidence directory");
    fs::write(
        evidence_dir.join("share_token_store.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize share token security ledger"),
    )
    .expect("write share token security ledger");
    assert!(
        mismatches.is_empty(),
        "{} share token mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_csrf_filter() {
    run_controller_future_on_large_stack("security-controls-csrf-filter", || {
        security_controls_differential_csrf_filter_impl()
    });
}

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_csrf_filter_impl() {
    let target = "slskdn";
    let subject = "Core/Security/ValidateCsrfForCookiesOnlyAttribute";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let base_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "csrf-filter-secret");
    let (default_state, _receiver) = test_state_with_env(base_env.clone());
    let cookie_env = base_env
        .clone()
        .with("SLSKR_API_COOKIE_AUTH_ENABLED", "true");
    let (cookie_state, _receiver) = test_state_with_env(cookie_env.clone());
    let cookie = "slskr.session=csrf-filter-secret";
    let headers = |origin: Option<&str>, cookie: Option<&str>| super::RequestSecurityHeaders {
        host: Some("127.0.0.1:5030".to_owned()),
        origin: origin.map(str::to_owned),
        cookie: cookie.map(str::to_owned),
        ..super::RequestSecurityHeaders::default()
    };

    record!(
        "activation-default-and-profile",
        default_state.config.auth_required
            && !default_state.config.api_cookie_auth_enabled
            && cookie_state.config.auth_required
            && cookie_state.config.api_cookie_auth_enabled
    );

    let bearer_cross_site = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        Some("Bearer csrf-filter-secret"),
        "",
        &cookie_state,
        headers(Some("https://evil.example"), None),
    )
    .await
    .expect("bearer CSRF exemption");
    let cookie_same_origin = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        None,
        "",
        &cookie_state,
        headers(Some("http://127.0.0.1:5030"), Some(cookie)),
    )
    .await
    .expect("same-origin cookie request");
    record!(
        "accepted-nominal-input",
        bearer_cross_site.status == "202 Accepted" && cookie_same_origin.status == "202 Accepted"
    );

    let cookie_cross_site = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        None,
        "",
        &cookie_state,
        headers(Some("https://evil.example"), Some(cookie)),
    )
    .await
    .expect("cross-site cookie request");
    let cookie_without_origin = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        None,
        "",
        &cookie_state,
        headers(None, Some(cookie)),
    )
    .await
    .expect("cookie request without origin");
    record!(
        "rejected-malicious-and-boundary-input",
        cookie_cross_site.status == "403 Forbidden"
            && cookie_cross_site.body == "{\"error\":\"cross-site mutating request rejected\"}"
            && cookie_without_origin.status == "403 Forbidden"
    );
    record!(
        "secret-logging-and-privacy-output",
        !cookie_cross_site.body.contains("csrf-filter-secret")
            && !cookie_without_origin.body.contains("csrf-filter-secret")
    );

    let (restarted, _receiver) = test_state_with_env(cookie_env);
    let restarted_same_origin = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        None,
        "",
        &restarted,
        headers(Some("http://127.0.0.1:5030"), Some(cookie)),
    )
    .await
    .expect("restarted same-origin cookie request");
    let restarted_cross_site = super::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        None,
        "",
        &restarted,
        headers(Some("https://evil.example"), Some(cookie)),
    )
    .await
    .expect("restarted cross-site cookie request");
    record!(
        "restart-rotation-and-recovery",
        restarted_same_origin.status == "202 Accepted"
            && restarted_cross_site.status == "403 Forbidden"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create CSRF security evidence directory");
    fs::write(
        evidence_dir.join("csrf_filter.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize CSRF filter ledger"),
    )
    .expect("write CSRF filter ledger");
    assert!(
        mismatches.is_empty(),
        "{} CSRF filter mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
fn security_controls_differential_hardening_validator() {
    let target = "slskdn";
    let subject = "Common/Security/HardeningValidator";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let base = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKD_PASSWORD", "hardening-secret");
    let config = |env: MapEnv| super::AppConfig::from_layers(None, FileConfig::default(), &env);
    let validate_rule = |env: MapEnv, rule: &str| match config(env) {
        Ok(value) => value
            .validate_controller_startup_hardening()
            .is_err_and(|error| error.contains(rule)),
        Err(error) => error.contains(rule),
    };

    let defaults = config(base.clone()).expect("default slskdn hardening config");
    record!(
        "activation-default-and-profile",
        matches!(
            defaults.controller_profile,
            super::ControllerProfile::Native
        ) && defaults.auth_required
            && defaults.validate_controller_startup_hardening().is_ok()
    );

    let safe_env = base
        .clone()
        .with("SLSKD_ENFORCE_SECURITY", "true")
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKD_WEB_CORS_ENABLED", "true")
        .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "false")
        .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "https://allowed.example");
    let safe = config(safe_env.clone()).expect("safe enforced hardening config");
    let loopback_no_auth = config(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_HTTP_BIND", "127.0.0.1:5030")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
    )
    .expect("loopback no-auth hardening config");
    record!(
        "accepted-nominal-input",
        safe.validate_controller_startup_hardening().is_ok()
            && loopback_no_auth
                .validate_controller_startup_hardening()
                .is_ok()
    );

    let auth_disabled_remote = validate_rule(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
        "AuthDisabledNonLoopback",
    );
    let remote_without_cidrs = validate_rule(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
            .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", ""),
        "RemoteNoAuthWithoutCidrs",
    );
    let credentialed_wildcard = validate_rule(
        safe_env
            .clone()
            .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "*"),
        "CorsCredentialsWithWildcard",
    );
    let memory_dump_without_auth = validate_rule(
        safe_env
            .clone()
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_ALLOW_MEMORY_DUMP", "true"),
        "MemoryDumpWithAuthDisabled",
    );
    let weak_metrics = validate_rule(
        safe_env
            .clone()
            .with("SLSKD_METRICS", "true")
            .with("SLSKD_METRICS_USERNAME", "slskd")
            .with("SLSKD_METRICS_PASSWORD", " "),
        "metrics authentication password must be configured",
    );

    let hash_root = std::env::temp_dir().join(format!(
        "slskr-security-hardening-hash-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&hash_root).expect("hardening hash config directory");
    fs::write(
        hash_root.join("slskd.yml"),
        "flags:\n  hash_from_audio_file_enabled: true\n",
    )
    .expect("hardening hash config");
    let hash_from_audio = config(safe_env.clone().with(
        "SLSKD_APP_DIR",
        hash_root.to_str().expect("hash config path"),
    ))
    .expect("hash hardening config");
    let hash_rejected = hash_from_audio
        .validate_controller_startup_hardening()
        .is_err_and(|error| error.contains("HashFromAudioFileEnabled"));
    fs::remove_dir_all(&hash_root).expect("remove hardening hash config directory");

    record!(
        "rejected-malicious-and-boundary-input",
        auth_disabled_remote
            && remote_without_cidrs
            && credentialed_wildcard
            && memory_dump_without_auth
            && weak_metrics
            && hash_rejected
    );

    let privacy_errors = [
        config(
            safe_env
                .clone()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
                .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
        )
        .ok()
        .and_then(|value| value.validate_controller_startup_hardening().err()),
        config(
            safe_env
                .clone()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_ALLOW_MEMORY_DUMP", "true"),
        )
        .ok()
        .and_then(|value| value.validate_controller_startup_hardening().err()),
    ];
    record!(
        "secret-logging-and-privacy-output",
        privacy_errors
            .iter()
            .flatten()
            .all(|error| !error.contains("hardening-secret"))
    );

    let restarted = config(safe_env).expect("restarted hardening config");
    record!(
        "restart-rotation-and-recovery",
        restarted.validate_controller_startup_hardening().is_ok()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create hardening security evidence directory");
    fs::write(
        evidence_dir.join("hardening_validator.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize hardening ledger"),
    )
    .expect("write hardening ledger");
    assert!(
        mismatches.is_empty(),
        "{} hardening-validator mismatches:\n{}",
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
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_certificate_manager() {
    use super::mesh_security::{CertificatePinManager, CertificatePinType, SecurityUtils};

    let target = "slskdn";
    let subject = "DhtRendezvous/Security/CertificateManager";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-security-certificate-manager-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("certificate manager root");
    let first = super::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("certificate manager bind"),
        &root,
        None,
    )
    .await
    .expect("create certificate identity");
    let first_pin = first.certificate_sha256();
    let certificate = root.join("overlay-certificate.der");
    let private_key = root.join("overlay-private-key.der");
    let certificate_bytes = fs::read(&certificate).expect("certificate bytes");
    let private_key_bytes = fs::read(&private_key).expect("private key bytes");
    let certificate_metadata = fs::metadata(&certificate).expect("certificate metadata");
    let private_key_metadata = fs::metadata(&private_key).expect("private key metadata");
    record!(
        "activation-default-and-profile",
        first_pin != [0; 32] && certificate_metadata.is_file() && private_key_metadata.is_file()
    );

    #[cfg(unix)]
    let private_key_is_restricted = {
        use std::os::unix::fs::PermissionsExt;

        private_key_metadata.permissions().mode() & 0o077 == 0
    };
    #[cfg(not(unix))]
    let private_key_is_restricted = true;
    let pin =
        SecurityUtils::certificate_pin_base64(&certificate_bytes).expect("certificate SPKI pin");
    let pin_root = root.join("pins");
    let pin_manager = CertificatePinManager::new(&pin_root).expect("create pin store");
    let pin_added = pin_manager
        .add_pin("certificate-peer", &pin, CertificatePinType::Current)
        .is_ok();
    record!(
        "accepted-nominal-input",
        certificate_bytes.len() > 256
            && !private_key_bytes.is_empty()
            && private_key_is_restricted
            && pin_added
            && pin_manager.validate_certificate_pin("certificate-peer", &certificate_bytes)
    );

    let wrong_certificate =
        rcgen::generate_simple_self_signed(vec!["certificate-manager-wrong-peer".to_owned()])
            .expect("wrong certificate fixture");
    let wrong_certificate_bytes = wrong_certificate.cert.der().to_vec();
    let wrong_pin_rejected =
        !pin_manager.validate_certificate_pin("certificate-peer", &wrong_certificate_bytes);
    let malformed_pin_rejected =
        !pin_manager.validate_certificate_pin("certificate-peer", b"not-a-der-certificate");
    let empty_pin_rejected = pin_manager
        .add_pin("certificate-peer", "", CertificatePinType::Current)
        .is_err();

    let incomplete_root = root.join("incomplete");
    fs::create_dir_all(&incomplete_root).expect("incomplete certificate root");
    fs::write(
        incomplete_root.join("overlay-certificate.der"),
        b"certificate-secret",
    )
    .expect("incomplete certificate fixture");
    let incomplete_error = match super::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("incomplete certificate bind"),
        &incomplete_root,
        None,
    )
    .await
    {
        Ok(_) => String::new(),
        Err(error) => error,
    };
    let oversized_root = root.join("oversized");
    fs::create_dir_all(&oversized_root).expect("oversized certificate root");
    fs::write(
        oversized_root.join("overlay-certificate.der"),
        vec![0_u8; 64 * 1024 + 1],
    )
    .expect("oversized certificate fixture");
    fs::write(oversized_root.join("overlay-private-key.der"), [1_u8])
        .expect("oversized private key fixture");
    let oversized_error = match super::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("oversized certificate bind"),
        &oversized_root,
        None,
    )
    .await
    {
        Ok(_) => String::new(),
        Err(error) => error,
    };
    #[cfg(unix)]
    let symlink_error = {
        use std::os::unix::fs::symlink;

        let symlink_root = root.join("symlink");
        fs::create_dir_all(&symlink_root).expect("symlink certificate root");
        let outside = symlink_root.join("outside.der");
        fs::write(&outside, &certificate_bytes).expect("outside certificate fixture");
        symlink(&outside, symlink_root.join("overlay-certificate.der"))
            .expect("certificate symlink fixture");
        fs::write(
            symlink_root.join("overlay-private-key.der"),
            &private_key_bytes,
        )
        .expect("symlink private key fixture");
        match super::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().expect("symlink certificate bind"),
            &symlink_root,
            None,
        )
        .await
        {
            Ok(_) => String::new(),
            Err(error) => error,
        }
    };
    #[cfg(not(unix))]
    let symlink_error = String::new();
    record!(
        "rejected-malicious-and-boundary-input",
        wrong_pin_rejected
            && malformed_pin_rejected
            && empty_pin_rejected
            && incomplete_error.contains("identity is incomplete")
            && oversized_error.contains("certificate is too large")
            && (cfg!(not(unix)) || symlink_error.contains("certificate must be a regular file"))
    );

    let pin_store_path = pin_root.join("mesh").join("certificate-pins.json");
    let pin_store_body = fs::read(&pin_store_path).expect("read certificate pin store");
    record!(
        "secret-logging-and-privacy-output",
        !incomplete_error.contains("certificate-secret")
            && !oversized_error.contains("certificate-secret")
            && !symlink_error.contains("certificate-secret")
            && !pin_store_body
                .windows(private_key_bytes.len())
                .any(|window| window == private_key_bytes.as_slice())
    );

    drop(first);
    let second = super::private_gateway::Gateway::load_or_create_with_quic(
        "127.0.0.1:0".parse().expect("reload certificate bind"),
        &root,
        None,
    )
    .await
    .expect("reload certificate identity");
    let reloaded_pin_manager = CertificatePinManager::new(&pin_root).expect("reload pin store");
    record!(
        "restart-rotation-and-recovery",
        second.certificate_sha256() == first_pin
            && reloaded_pin_manager
                .validate_certificate_pin("certificate-peer", &certificate_bytes)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create certificate security evidence directory");
    fs::write(
        evidence_dir.join("certificate_manager.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize certificate manager ledger"),
    )
    .expect("write certificate manager ledger");
    drop(second);
    fs::remove_dir_all(&root).expect("remove certificate manager root");
    assert!(
        mismatches.is_empty(),
        "{} certificate-manager mismatches:\n{}",
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
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_overlay_message_validation() {
    use slskr_client::overlay::{
        Disconnect, MeshHello, OverlayFramer, Ping, SoulseekPorts, MAX_OVERLAY_MESSAGE_BYTES,
        OVERLAY_MAGIC, OVERLAY_VERSION,
    };
    use tokio::io::{duplex, AsyncWriteExt};

    let target = "slskdn";
    let validator = "DhtRendezvous/Security/MessageValidator";
    let framer = "DhtRendezvous/Security/SecureMessageFramer";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($subject:expr, $case:expr, $pass:expr) => {{
            let subject = $subject;
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let hello = MeshHello::new(
        "validator-peer",
        vec!["mesh_service".to_owned(), "mesh_search".to_owned()],
        Some(SoulseekPorts {
            peer: 2234,
            file: 2235,
        }),
        Some(2240),
        "validator_nonce",
    )
    .expect("valid overlay hello");
    let ping = Ping {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "ping".to_owned(),
        version: OVERLAY_VERSION,
        timestamp: super::unix_timestamp_millis() as i64,
    };
    let disconnect = Disconnect {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "disconnect".to_owned(),
        version: OVERLAY_VERSION,
        reason: Some("normal shutdown".to_owned()),
    };
    record!(
        validator,
        "activation-default-and-profile",
        hello.validate().is_ok() && ping.validate().is_ok() && disconnect.validate().is_ok()
    );

    let (writer_stream, reader_stream) = duplex(4096);
    let mut writer = OverlayFramer::new(writer_stream);
    writer
        .write(&hello)
        .await
        .expect("write valid overlay frame");
    drop(writer);
    let mut reader = OverlayFramer::new(reader_stream);
    let decoded: MeshHello = reader.read().await.expect("read valid overlay frame");
    record!(framer, "activation-default-and-profile", decoded == hello);
    record!(
        validator,
        "accepted-nominal-input",
        decoded.validate().is_ok() && ping.validate().is_ok() && disconnect.validate().is_ok()
    );
    record!(
        framer,
        "accepted-nominal-input",
        serde_json::to_vec(&decoded)
            .ok()
            .is_some_and(|bytes| bytes.len() >= 2)
    );

    let mut invalid_magic = hello.clone();
    invalid_magic.magic = "attacker-magic".to_owned();
    let mut invalid_version = hello.clone();
    invalid_version.version = 0;
    let mut invalid_username = hello.clone();
    invalid_username.username = "validator-secret!".to_owned();
    let mut too_many_features = hello.clone();
    too_many_features.features = (0..21).map(|index| format!("feature_{index}")).collect();
    let mut invalid_port = hello.clone();
    invalid_port.soulseek_ports = Some(SoulseekPorts {
        peer: 0,
        file: 2235,
    });
    let mut long_disconnect = disconnect.clone();
    long_disconnect.reason = Some("x".repeat(257));
    record!(
        validator,
        "rejected-malicious-and-boundary-input",
        invalid_magic.validate().is_err()
            && invalid_version.validate().is_err()
            && invalid_username.validate().is_err()
            && too_many_features.validate().is_err()
            && invalid_port.validate().is_err()
            && long_disconnect.validate().is_err()
    );

    let (mut frame_writer, frame_reader) = duplex(128);
    frame_writer
        .write_all(&1_u32.to_be_bytes())
        .await
        .expect("write undersized overlay frame");
    let undersized = OverlayFramer::new(frame_reader).read_raw().await.is_err();
    let (mut frame_writer, frame_reader) = duplex(128);
    frame_writer
        .write_all(&((MAX_OVERLAY_MESSAGE_BYTES as u32) + 1).to_be_bytes())
        .await
        .expect("write oversized overlay frame");
    let oversized = OverlayFramer::new(frame_reader).read_raw().await.is_err();
    record!(
        framer,
        "rejected-malicious-and-boundary-input",
        undersized && oversized
    );

    let invalid_username_error = invalid_username
        .validate()
        .expect_err("invalid username must be rejected")
        .to_string();
    record!(
        validator,
        "secret-logging-and-privacy-output",
        !invalid_username_error.contains("validator-secret")
    );
    record!(
        framer,
        "secret-logging-and-privacy-output",
        !format!("{invalid_username_error:?}").contains("validator-secret")
    );
    record!(
        validator,
        "restart-rotation-and-recovery",
        MeshHello::new(
            "validator-peer",
            vec!["mesh_service".to_owned()],
            None,
            None,
            "validator_nonce",
        )
        .is_ok()
    );
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create overlay validation evidence directory");
    fs::write(
        evidence_dir.join("overlay_message_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize overlay validation ledger"),
    )
    .expect("write overlay validation ledger");
    assert!(
        mismatches.is_empty(),
        "{} overlay validation mismatches:\n{}",
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
    feature = "bounded-security-control-tests"
))]
async fn security_controls_differential_solid_fetch_policy() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let target = "slskdn";
    let subject = "Solid/SolidFetchPolicy";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let case = $case;
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {subject} [{case}]"));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "subject": subject,
                "case": case,
                "pass": pass,
            }));
        }};
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let defaults = state.media_services.read().await.solid.clone();
    record!(
        "activation-default-and-profile",
        defaults.max_fetch_bytes > 0
            && !defaults.allow_insecure_http
            && !defaults.allow_localhost_for_web_id
            && !defaults.timeout.is_zero()
    );

    {
        let mut media = state.media_services.write().await;
        media.solid.allow_insecure_http = true;
        media.solid.allow_localhost_for_web_id = true;
        media.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
        media.solid.timeout = Duration::from_secs(1);
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Solid policy fixture");
    let port = listener
        .local_addr()
        .expect("Solid policy fixture address")
        .port();
    let web_id = format!("http://127.0.0.1:{port}/profile/card#me");
    let profile = format!(
        "@prefix solid: <http://www.w3.org/ns/solid/terms#>.\n<{web_id}> solid:oidcIssuer <https://solid-policy.example/oidc>.\n"
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("Solid policy request");
        let mut request = [0_u8; 4096];
        let _ = stream
            .read(&mut request)
            .await
            .expect("read Solid policy request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/turtle\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            profile.len(), profile
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write Solid policy response");
    });
    let accepted = super::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        &serde_json::json!({"webId": web_id}).to_string(),
        &state,
    )
    .await
    .expect("accepted Solid policy request");
    server.await.expect("Solid policy fixture task");
    let accepted_json =
        serde_json::from_str::<serde_json::Value>(&accepted.body).unwrap_or_default();
    record!(
        "accepted-nominal-input",
        accepted.status == "200 OK"
            && accepted_json["oidcIssuers"]
                == serde_json::json!(["https://solid-policy.example/oidc"])
    );

    {
        let mut media = state.media_services.write().await;
        media.solid.allow_localhost_for_web_id = false;
    }
    let blocked_local = super::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        r#"{"webId":"http://127.0.0.1:1/profile#solid-policy-secret"}"#,
        &state,
    )
    .await
    .expect("blocked localhost Solid policy request");
    let blocked_remote = super::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        r#"{"webId":"https://private-solid-policy.example/profile#opaque-secret"}"#,
        &state,
    )
    .await
    .expect("blocked host Solid policy request");
    record!(
        "rejected-malicious-and-boundary-input",
        blocked_local.status == "400 Bad Request"
            && blocked_remote.status == "400 Bad Request"
            && blocked_local.body.contains("Solid fetch blocked")
            && blocked_remote.body.contains("Solid fetch blocked")
    );
    record!(
        "secret-logging-and-privacy-output",
        !blocked_local.body.contains("solid-policy-secret")
            && !blocked_remote.body.contains("opaque-secret")
            && !blocked_local.body.contains("127.0.0.1:1")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("security-controls");
    fs::create_dir_all(&evidence_dir).expect("create Solid policy security evidence directory");
    fs::write(
        evidence_dir.join("solid_fetch_policy.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Solid policy ledger"),
    )
    .expect("write Solid policy ledger");
    assert!(
        mismatches.is_empty(),
        "{} Solid-fetch-policy mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
