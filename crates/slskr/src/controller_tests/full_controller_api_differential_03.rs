//! Controller full controller api differential 03 ownership.

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
pub(super) async fn controller_api_differential_controller_relay_controller_routes() {
    let target = "slskd";
    let (state, secret, now) = configured_relay_test_state().await;
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    async fn live_relay_download(
        state: Arc<crate::AppState>,
        token: &str,
        credential: &str,
    ) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(crate::handle_http_stream(server, None, false, state));
        let request = format!(
            "GET /api/v0/relay/controller/downloads/{token} HTTP/1.1\r\n\
             Host: localhost\r\n\
             X-Relay-Credential: {credential}\r\n\
             Connection: close\r\n\r\n"
        );
        client
            .write_all(request.as_bytes())
            .await
            .expect("write relay download request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read relay download response");
        task.await
            .expect("relay download HTTP task")
            .expect("relay download HTTP response");
        response
    }

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

    let download_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_download_tokens("Relay/Agent.txt", now)
        .into_iter()
        .next()
        .map(|(_, token)| token)
        .expect("relay download token");
    let download_credential =
        crate::relay::credential_for_test(secret, "edge-one", &download_token);
    let downloads_root = crate::effective_downloads_dir(&state);
    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download root");
    fs::write(downloads_root.join("Relay/Agent.txt"), b"relay payload")
        .expect("relay download fixture");
    let download_headers = crate::RequestSecurityHeaders {
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(download_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let download = Box::pin(crate::route_http_request_with_headers(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &state,
        download_headers.clone(),
    ))
    .await
    .expect("relay controller download");
    let mut stream =
        crate::open_relay_controller_download(&state, &download_token, &download_headers)
            .await
            .expect("open relay controller download");
    let mut payload = Vec::new();
    std::io::Read::read_to_end(&mut stream.file, &mut payload).expect("read relay download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "nominal-status-headers-body",
        download.status == "200 OK"
            && download.content_type == "application/octet-stream"
            && download.body.is_empty()
    );
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "populated-dynamic-state",
        payload == b"relay payload"
    );
    fs::remove_file(downloads_root.join("Relay/Agent.txt")).expect("remove relay download fixture");
    let runtime_download = live_relay_download(
        Arc::clone(&state),
        &download_token.to_string(),
        &download_credential,
    )
    .await;
    let runtime_download = String::from_utf8_lossy(&runtime_download);
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "runtime-failure-and-timeout",
        runtime_download.starts_with("HTTP/1.1 500 Internal Server Error")
            && runtime_download.contains("failed to open relay controller download")
            && !runtime_download.contains("Relay/Agent.txt")
    );

    let (upload_token, upload_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Upload.flac", 0, now)
        .expect("relay upload stream");
    let upload_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(crate::relay::credential_for_test(
            secret,
            "edge-one",
            &upload_token.to_string(),
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let upload_body =
        "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Upload.flac\"\r\n\r\npayload\r\n--relay--\r\n";
    let upload = Box::pin(crate::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay controller file upload");
    let uploaded = upload_receiver
        .await
        .expect("relay upload receiver")
        .expect("relay upload result");
    assert_eq!(uploaded.filename, "Upload.flac");
    let stored_upload = state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!("file-{}.part", upload_token.simple()));
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "nominal-status-headers-body",
        upload.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "mutation-side-effects-and-readback",
        fs::read(&stored_upload).ok().as_deref() == Some(b"payload".as_slice())
    );
    let replay = Box::pin(crate::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay controller file replay");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "concurrency-and-idempotency",
        replay.status == "401 Unauthorized"
    );

    let missing_file_token = uuid::Uuid::new_v4();
    let missing_file_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(crate::relay::credential_for_test(
            secret,
            "edge-one",
            &missing_file_token.to_string(),
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let missing_file = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{missing_file_token}"),
        upload_body.as_bytes(),
        &missing_file_headers,
        &state,
    )
    .await
    .expect("missing relay controller file token response");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "missing-empty-or-conflict-state",
        missing_file.status == "401 Unauthorized"
    );

    let (fresh_relay_state, _fresh_secret, _fresh_now) = configured_relay_test_state().await;
    let restart_file = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        upload_body.as_bytes(),
        &upload_headers,
        &fresh_relay_state,
    )
    .await
    .expect("reset relay controller file token response");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "restart-persistence-or-reset",
        restart_file.status == "401 Unauthorized"
    );

    let incoming_directory = state.config.state_dir.join("relay").join("incoming");
    fs::remove_dir_all(&incoming_directory).expect("remove relay upload directory");
    fs::write(&incoming_directory, b"relay upload directory is a file")
        .expect("create relay upload directory conflict");
    let (runtime_file_token, runtime_file_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Upload.flac", 0, now)
        .expect("runtime relay upload stream");
    let runtime_file_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(crate::relay::credential_for_test(
            secret,
            "edge-one",
            &runtime_file_token.to_string(),
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let runtime_file = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{runtime_file_token}"),
        upload_body.as_bytes(),
        &runtime_file_headers,
        &state,
    )
    .await
    .expect("runtime relay controller file response");
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "runtime-failure-and-timeout",
        runtime_file.status == "503 Service Unavailable"
            && !runtime_file
                .body
                .contains("relay upload directory is a file")
    );
    assert!(runtime_file_receiver
        .await
        .expect("runtime relay upload receiver")
        .is_err());
    fs::remove_file(&incoming_directory).expect("remove relay upload conflict");
    fs::create_dir_all(&incoming_directory).expect("restore relay upload directory");

    let share_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("relay share upload token");
    let share_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(crate::relay::credential_for_test(
            secret,
            "edge-one",
            &share_token,
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let database_source = state.config.state_dir.join("relay-differential-source.db");
    crate::relay::write_share_database(
        &database_source,
        crate::ControllerProfile::Legacy,
        &[crate::relay::RemoteShare {
            filename: "Remote/Agent.flac".to_owned(),
            size: 6,
        }],
    )
    .await
    .expect("relay differential share database");
    let database_bytes = fs::read(&database_source).expect("read relay differential database");
    let mut share_body = Vec::new();
    share_body.extend_from_slice(
        b"--relay\r\nContent-Disposition: form-data; name=\"shares\"\r\n\r\n[]\r\n--relay\r\nContent-Disposition: form-data; name=\"database\"; filename=\"shares.db\"\r\n\r\n",
    );
    share_body.extend_from_slice(&database_bytes);
    share_body.extend_from_slice(b"\r\n--relay--\r\n");
    let shares = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &state,
    )
    .await
    .expect("relay controller share upload");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "nominal-status-headers-body",
        shares.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "mutation-side-effects-and-readback",
        state
            .relay
            .read()
            .await
            .protocol
            .remote_file_for_agent("edge-one", "Remote/Agent.flac")
            .is_some()
    );
    let share_replay = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &state,
    )
    .await
    .expect("relay controller share replay");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "concurrency-and-idempotency",
        share_replay.status == "401 Unauthorized"
    );

    let missing_share_token = uuid::Uuid::new_v4().to_string();
    let missing_share_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(crate::relay::credential_for_test(
            secret,
            "edge-one",
            &missing_share_token,
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let missing_share = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{missing_share_token}"),
        &share_body,
        &missing_share_headers,
        &state,
    )
    .await
    .expect("missing relay controller share token response");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "missing-empty-or-conflict-state",
        missing_share.status == "401 Unauthorized"
    );

    let restart_share = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &fresh_relay_state,
    )
    .await
    .expect("reset relay controller share token response");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "restart-persistence-or-reset",
        restart_share.status == "401 Unauthorized"
    );

    fs::remove_dir_all(&incoming_directory).expect("remove relay share upload directory");
    fs::write(
        &incoming_directory,
        b"relay share upload directory is a file",
    )
    .expect("create relay share upload directory conflict");
    let runtime_share_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("runtime relay share upload token");
    let runtime_share_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(crate::relay::credential_for_test(
            secret,
            "edge-one",
            &runtime_share_token,
        )),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let runtime_share = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{runtime_share_token}"),
        &share_body,
        &runtime_share_headers,
        &state,
    )
    .await
    .expect("runtime relay controller share response");
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "runtime-failure-and-timeout",
        runtime_share.status == "503 Service Unavailable"
            && !runtime_share
                .body
                .contains("relay share upload directory is a file")
    );
    fs::remove_file(&incoming_directory).expect("remove relay share upload conflict");
    fs::create_dir_all(&incoming_directory).expect("restore relay share upload directory");
    let _ = fs::remove_file(database_source);
    let _ = fs::remove_dir_all(downloads_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_relay_controller_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd relay controller ledger"),
    )
    .expect("write slskd relay controller ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd relay controller mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_core_application_session_events_and_telemetry(
) {
    let target = "slskd";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
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

    let application = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("slskd application state");
    let application_json =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        "nominal-status-headers-body",
        application.status == "200 OK"
            && application.content_type == "application/json; charset=utf-8"
            && application_json["runtimeProfile"]
                == if target == "slskd" {
                    "legacy"
                } else {
                    "native"
                }
            && application_json["version"]["current"] == env!("CARGO_PKG_VERSION")
            && application_json["version"]["full"].is_string()
            && application_json["pendingReconnect"].is_boolean()
            && application_json["pendingRestart"] == false
            && application_json["server"].is_object()
            && application_json["connectionWatchdog"].is_object()
            && application_json["vpn"].is_object()
            && application_json["shares"].is_object()
            && application_json["rooms"].is_array()
            && application_json["users"].is_array()
            && application_json["vpn"].get("portForwards").is_none()
    );

    let restart = crate::route_http_request("PUT", "/api/v0/application", None, "{}", &state)
        .await
        .expect("slskd application restart");
    record!(
        "PUT",
        "/api/v0/application",
        "nominal-status-headers-body",
        restart.status == "204 No Content" && restart.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        state.runtime.read().await.application_restart_requested
    );
    let populated_application =
        crate::route_http_request("GET", "/api/v0/application", None, "", &state)
            .await
            .expect("populated slskd application state");
    let populated_application_json =
        serde_json::from_str::<serde_json::Value>(&populated_application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        "populated-dynamic-state",
        populated_application.status == "200 OK"
            && populated_application_json["pendingRestart"] == true
    );

    let version = crate::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("slskd application version");
    record!(
        "GET",
        "/api/v0/application/version",
        "nominal-status-headers-body",
        version.status == "200 OK"
            && version.content_type == "application/json"
            && serde_json::from_str::<serde_json::Value>(&version.body).unwrap_or_default()
                == serde_json::json!(env!("CARGO_PKG_VERSION"))
    );

    let latest = crate::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd latest version");
    let latest_json = serde_json::from_str::<serde_json::Value>(&latest.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "nominal-status-headers-body",
        latest.status == "200 OK"
            && latest_json["current"] == env!("CARGO_PKG_VERSION")
            && latest_json["isCanary"].is_boolean()
            && latest_json["isDevelopment"].is_boolean()
            && latest_json.get("latest").is_none()
    );
    {
        let mut version_state = state
            .controller_version
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        version_state.latest = Some("0.0.1".to_owned());
        version_state.latest_tag = Some("v0.0.1".to_owned());
        version_state.latest_url = Some("https://example.test/slskd/0.0.1".to_owned());
        version_state.checked_at = Some("2026-08-08T00:00:00Z".to_owned());
        version_state.is_update_available = Some(true);
    }
    let populated_latest = crate::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("populated slskd latest version");
    let populated_latest_json =
        serde_json::from_str::<serde_json::Value>(&populated_latest.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/version/latest",
        "populated-dynamic-state",
        populated_latest.status == "200 OK"
            && populated_latest_json["latest"] == "0.0.1"
            && populated_latest_json["latestTag"] == "v0.0.1"
            && populated_latest_json["latestUrl"] == "https://example.test/slskd/0.0.1"
            && populated_latest_json["checkedAt"] == "2026-08-08T00:00:00Z"
            && populated_latest_json["isUpdateAvailable"] == true
    );

    let gc_before = state.runtime.read().await.gc_runs;
    let gc = crate::route_http_request("POST", "/api/v0/application/gc", None, "", &state)
        .await
        .expect("slskd garbage collection");
    record!(
        "POST",
        "/api/v0/application/gc",
        "nominal-status-headers-body",
        gc.status == "200 OK" && gc.content_type.is_empty() && gc.body.is_empty()
    );
    record!(
        "POST",
        "/api/v0/application/gc",
        "mutation-side-effects-and-readback",
        state.runtime.read().await.gc_runs == gc_before.saturating_add(1)
    );

    let loopback = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"slskd"}"#,
        &state,
    )
    .await
    .expect("slskd loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "nominal-status-headers-body",
        loopback.status == "200 OK" && loopback.content_type.is_empty() && loopback.body.is_empty()
    );
    let loopback_logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("slskd loopback log readback");
    let loopback_logs_json =
        serde_json::from_str::<serde_json::Value>(&loopback_logs.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/application/loopback",
        "mutation-side-effects-and-readback",
        loopback_logs.status == "200 OK"
            && loopback_logs_json.as_array().is_some_and(|logs| {
                logs.iter().any(|log| {
                    log["category"] == "application"
                        && log["message"] == "Loopback POST: {\"probe\":\"slskd\"}"
                })
            })
    );
    let (fresh_loopback_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let fresh_loopback_logs =
        crate::route_http_request("GET", "/api/v0/logs", None, "", &fresh_loopback_state)
            .await
            .expect("fresh slskd loopback logs");
    let fresh_loopback = crate::route_http_request(
        "POST",
        "/api/v0/application/loopback",
        None,
        r#"{"probe":"slskd-fresh"}"#,
        &fresh_loopback_state,
    )
    .await
    .expect("fresh slskd loopback");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "restart-persistence-or-reset",
        fresh_loopback_logs.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&fresh_loopback_logs.body)
                .is_ok_and(|logs| logs.as_array().is_some_and(Vec::is_empty))
            && fresh_loopback.status == "200 OK"
    );
    let loopback_missing =
        crate::route_http_request("POST", "/api/v0/application/loopback", None, "null", &state)
            .await
            .expect("slskd missing loopback body");
    record!(
        "POST",
        "/api/v0/application/loopback",
        "malformed-path-query-or-body",
        loopback_missing.status == "400 Bad Request"
    );
    let loopback_retries = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"slskd-retry-a"}"#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/application/loopback",
            None,
            r#"{"probe":"slskd-retry-b"}"#,
            &state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/application/loopback",
        "concurrency-and-idempotency",
        loopback_retries.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty())
        }) && crate::route_http_request("GET", "/api/v0/logs", None, "", &state)
            .await
            .ok()
            .and_then(|logs| serde_json::from_str::<serde_json::Value>(&logs.body).ok())
            .is_some_and(|logs| {
                [
                    "Loopback POST: {\"probe\":\"slskd-retry-a\"}",
                    "Loopback POST: {\"probe\":\"slskd-retry-b\"}",
                ]
                .iter()
                .all(|message| {
                    logs.as_array()
                        .is_some_and(|rows| rows.iter().any(|row| row["message"] == *message))
                })
            })
    );
    let gc_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("application GC differential database");
    let (gc_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(gc_db.clone()),
    );
    let gc_first = crate::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
        .await
        .expect("first persisted slskd garbage collection");
    let gc_second =
        crate::route_http_request("POST", "/api/v0/application/gc", None, "", &gc_state)
            .await
            .expect("repeated persisted slskd garbage collection");
    let persisted_gc = gc_db
        .get_runtime_compat_state()
        .await
        .expect("read persisted GC state")
        .expect("persisted GC row");
    let rehydrated_gc = crate::RuntimeCompatState::from_persisted(&persisted_gc);
    record!(
        "POST",
        "/api/v0/application/gc",
        "restart-persistence-or-reset",
        gc_first.status == "200 OK"
            && gc_second.status == "200 OK"
            && persisted_gc.gc_runs >= 2
            && rehydrated_gc.gc_runs >= 2
    );
    record!(
        "POST",
        "/api/v0/application/gc",
        "concurrency-and-idempotency",
        gc_second.status == "200 OK" && persisted_gc.gc_runs == 2
    );

    let session = crate::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("slskd session state");
    let session_json = serde_json::from_str::<serde_json::Value>(&session.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "nominal-status-headers-body",
        session.status == "200 OK"
            && session.content_type == "application/json"
            && session_json["state"].is_string()
    );

    let login_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "slskd-login-secret")
        .with("SLSKD_USERNAME", "admin")
        .with("SLSKD_PASSWORD", "slskd-login-secret");
    let (login_state, _receiver) = test_state_with_env(login_env.clone());
    let login = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"slskd-login-secret"}"#,
        &login_state,
    )
    .await
    .expect("slskd login");
    let login_json = serde_json::from_str::<serde_json::Value>(&login.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/session",
        "nominal-status-headers-body",
        login.status == "200 OK"
            && login_json["name"] == "admin"
            && login_json["tokenType"] == "Bearer"
            && login_json["token"]
                .as_str()
                .is_some_and(|token| token.split('.').count() == 3)
    );
    let (invalid_login_state, _receiver) = test_state_with_env(login_env);
    let invalid_login =
        crate::route_http_request("POST", "/api/v0/session", None, "{}", &invalid_login_state)
            .await
            .expect("slskd malformed login");
    record!(
        "POST",
        "/api/v0/session",
        "malformed-path-query-or-body",
        invalid_login.status == "400 Bad Request"
    );
    let unauthorized_login = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"wrong"}"#,
        &invalid_login_state,
    )
    .await
    .expect("slskd invalid login");
    record!(
        "POST",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        unauthorized_login.status == "401 Unauthorized"
    );

    let events_before = crate::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("slskd events baseline");
    let events_before_json =
        serde_json::from_str::<serde_json::Value>(&events_before.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/events",
        "nominal-status-headers-body",
        events_before.status == "200 OK"
            && events_before_json
                .as_array()
                .is_some_and(|events| { events.iter().all(|event| event["type"] != "Noop") })
    );
    let events_before_count = events_before_json
        .as_array()
        .map_or(0, |events| events.len());
    let event = crate::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""core-slice""#,
        &state,
    )
    .await
    .expect("slskd event injection");
    let event_json = serde_json::from_str::<serde_json::Value>(&event.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/events",
        "nominal-status-headers-body",
        event.status == "201 Created"
            && event_json["recorded"] == true
            && event_json["event"]["type"] == "Noop"
    );
    record!(
        "POST",
        "/api/v0/events",
        "mutation-side-effects-and-readback",
        event_json["count"].as_u64() == Some(events_before_count as u64 + 1)
    );
    let events_after = crate::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("slskd populated events");
    let events_after_json =
        serde_json::from_str::<serde_json::Value>(&events_after.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/events",
        "populated-dynamic-state",
        events_after.status == "200 OK"
            && events_after_json.as_array().is_some_and(|events| {
                events.len() == events_before_count + 1
                    && events.iter().any(|event| event["type"] == "Noop")
            })
    );
    let unknown_event = crate::route_http_request(
        "POST",
        "/api/v0/events/Unknown",
        None,
        r#""core-slice""#,
        &state,
    )
    .await
    .expect("slskd unknown event");
    record!(
        "POST",
        "/api/v0/events",
        "malformed-path-query-or-body",
        unknown_event.status == "400 Bad Request"
    );

    let metrics = crate::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
        .await
        .expect("slskd metrics");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "nominal-status-headers-body",
        metrics.status == "200 OK"
            && metrics
                .content_type
                .starts_with("text/plain; version=0.0.4")
            && metrics.body.contains("# HELP slskr_telemetry_transfers")
            && metrics.body.contains("slskr_telemetry_transfers 0")
    );
    let kpis = crate::route_http_request("GET", "/api/v0/telemetry/metrics/kpis", None, "", &state)
        .await
        .expect("slskd KPI metrics");
    let kpis_json = serde_json::from_str::<serde_json::Value>(&kpis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpis",
        "nominal-status-headers-body",
        kpis.status == "200 OK"
            && kpis.content_type == "application/json"
            && kpis_json["slskr_transfers"]["samples"].is_array()
            && kpis_json["slskr_searches"]["samples"].is_array()
    );

    let empty_summary = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty transfer summary");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        "nominal-status-headers-body",
        empty_summary.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&empty_summary.body).unwrap_or_default()
                == serde_json::json!({"Download": {}, "Upload": {}})
    );
    let empty_histogram = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty transfer histogram");
    let empty_histogram_json =
        serde_json::from_str::<serde_json::Value>(&empty_histogram.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram",
        "nominal-status-headers-body",
        empty_histogram.status == "200 OK"
            && empty_histogram_json["2100-01-01T00:00:00Z"].is_object()
    );

    {
        let mut transfers = state.transfers.write().await;
        let download = transfers.create(
            0,
            Some("telemetry peer".to_owned()),
            "Telemetry/Report.flac".to_owned(),
            None,
            Some(321),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == download.id)
            .expect("populated download transfer");
        entry.status = "succeeded".to_owned();
        entry.requested_at = 3_600;
        entry.started_at = Some(3_610);
        entry.bytes_transferred = 321;
        entry.updated_at = 3_630;
        entry.updated_at_ms = 3_630_000;

        let upload = transfers.create(
            1,
            Some("upload peer".to_owned()),
            "Albums/Release/Track.flac".to_owned(),
            None,
            Some(99),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == upload.id)
            .expect("populated upload transfer");
        entry.status = "succeeded".to_owned();
        entry.requested_at = 3_600;
        entry.started_at = Some(3_605);
        entry.bytes_transferred = 99;
        entry.updated_at = 3_620;
        entry.updated_at_ms = 3_620_000;
    }

    let populated_metrics =
        crate::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
            .await
            .expect("slskd populated metrics");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "populated-dynamic-state",
        populated_metrics.status == "200 OK"
            && populated_metrics
                .body
                .contains("slskr_telemetry_transfers 2")
    );
    let populated_kpis =
        crate::route_http_request("GET", "/api/v0/telemetry/metrics/kpis", None, "", &state)
            .await
            .expect("slskd populated KPI metrics");
    let populated_kpis_json =
        serde_json::from_str::<serde_json::Value>(&populated_kpis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpis",
        "populated-dynamic-state",
        populated_kpis_json["slskr_transfers"]["samples"][0]["value"] == 2.0
    );

    let summary = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary?start=3600&end=7200",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated transfer summary");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        "populated-dynamic-state",
        summary.status == "200 OK"
            && summary_json["Download"]["Succeeded"]["count"] == 1
            && summary_json["Download"]["Succeeded"]["totalBytes"] == 321
            && summary_json["Upload"]["Succeeded"]["count"] == 1
            && summary_json["Upload"]["Succeeded"]["totalBytes"] == 99
    );
    let histogram = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=3600&end=7200&interval=60",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated transfer histogram");
    let histogram_json =
        serde_json::from_str::<serde_json::Value>(&histogram.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram",
        "populated-dynamic-state",
        histogram.status == "200 OK"
            && histogram_json["1970-01-01T01:00:00Z"]["Download"]["Succeeded"]["count"] == 1
            && histogram_json["1970-01-01T01:00:00Z"]["Upload"]["Succeeded"]["count"] == 1
    );

    let leaderboard_missing_direction = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd leaderboard missing direction");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "malformed-path-query-or-body",
        leaderboard_missing_direction.status == "400 Bad Request"
    );
    let leaderboard_empty = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty leaderboard");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "nominal-status-headers-body",
        leaderboard_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&leaderboard_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let leaderboard = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&start=3600&end=7200",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated leaderboard");
    let leaderboard_json =
        serde_json::from_str::<serde_json::Value>(&leaderboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "populated-dynamic-state",
        leaderboard.status == "200 OK"
            && leaderboard_json[0]["username"] == "telemetry peer"
            && leaderboard_json[0]["count"] == 1
            && leaderboard_json[0]["totalBytes"] == 321
    );

    let user_empty = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/unknown",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty user report");
    let user_empty_json =
        serde_json::from_str::<serde_json::Value>(&user_empty.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "nominal-status-headers-body",
        user_empty.status == "200 OK"
            && user_empty_json["username"] == "unknown"
            && user_empty_json["count"] == 0
    );
    let user = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated user report");
    let user_json = serde_json::from_str::<serde_json::Value>(&user.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "populated-dynamic-state",
        user.status == "200 OK"
            && user_json["username"] == "telemetry peer"
            && user_json["count"] == 1
            && user_json["transfers"]
                .as_array()
                .is_some_and(|rows| rows.len() == 1)
    );

    {
        let mut transfers = state.transfers.write().await;
        let failed = transfers.create(
            0,
            Some("telemetry peer".to_owned()),
            "Telemetry/Failed.flac".to_owned(),
            None,
            Some(12),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == failed.id)
            .expect("populated failed transfer");
        entry.status = "failed".to_owned();
        entry.reason = Some("network: timeout".to_owned());
        entry.requested_at = 3_600;
        entry.started_at = Some(3_601);
        entry.updated_at = 3_602;
        entry.updated_at_ms = 3_602_000;
    }
    let exceptions_missing_direction = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd exceptions missing direction");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "malformed-path-query-or-body",
        exceptions_missing_direction.status == "400 Bad Request"
    );
    let exceptions_empty = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions?direction=Upload&username=none",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty exceptions");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "nominal-status-headers-body",
        exceptions_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&exceptions_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let exceptions = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated exceptions");
    let exceptions_json =
        serde_json::from_str::<serde_json::Value>(&exceptions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "populated-dynamic-state",
        exceptions.status == "200 OK"
            && exceptions_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["username"] == "telemetry peer"
                    && rows[0]["direction"] == "Download"
                    && rows[0]["state"] == "Failed"
            })
    );

    let pareto_missing_direction = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd pareto missing direction");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "malformed-path-query-or-body",
        pareto_missing_direction.status == "400 Bad Request"
    );
    let pareto_empty = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Upload&username=none",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty pareto");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "nominal-status-headers-body",
        pareto_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&pareto_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let pareto = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated pareto");
    let pareto_json = serde_json::from_str::<serde_json::Value>(&pareto.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "populated-dynamic-state",
        pareto.status == "200 OK"
            && pareto_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["exception"] == "transfer failed"
                    && rows[0]["count"] == 1
            })
    );

    let directories_empty = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories?username=none",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd empty transfer directories");
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "nominal-status-headers-body",
        directories_empty.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&directories_empty.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );
    let directories = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated transfer directories");
    let directories_json =
        serde_json::from_str::<serde_json::Value>(&directories.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "populated-dynamic-state",
        directories.status == "200 OK"
            && directories_json.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows[0]["path"] == "Albums/Release"
                    && rows[0]["count"] == 1
                    && rows[0]["distinctUsers"] == 1
            })
    );

    let shutdown = crate::route_http_request("DELETE", "/api/v0/application", None, "", &state)
        .await
        .expect("slskd application shutdown");
    record!(
        "DELETE",
        "/api/v0/application",
        "nominal-status-headers-body",
        shutdown.status == "204 No Content" && shutdown.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/application",
        "mutation-side-effects-and-readback",
        !state.runtime.read().await.application_restart_requested
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_core_application_session_events_telemetry.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd core ledger"),
    )
    .expect("write slskd core ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd core controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
/// The nominal core slice above deliberately keeps its fixtures small.
/// This companion closes the remaining deterministic empty-state,
/// malformed-input, restart, idempotency, and persistence-failure cases
/// for the same frozen slskd routes.  Every row is emitted only after a
/// real dispatcher call and, where applicable, a raw SQLite readback or a
/// fresh store rehydration check.
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
pub(super) async fn controller_api_differential_controller_core_failure_restart_and_empty_contracts(
) {
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

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("core differential database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));

    // Disconnected application/session/event state is a real empty
    // projection, not a missing handler.
    let application = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("empty slskd application");
    let application_json =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application",
        "missing-empty-or-conflict-state",
        application.status == "200 OK"
            && application_json["pendingRestart"] == false
            && application_json["server"]["isConnected"] == false
    );

    let session = crate::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("empty slskd session");
    let session_json = serde_json::from_str::<serde_json::Value>(&session.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "missing-empty-or-conflict-state",
        session.status == "200 OK" && session_json["state"] == "disconnected"
    );

    let events = crate::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("empty slskd events");
    record!(
        "GET",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        events.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&events.body)
                .unwrap_or_default()
                .as_array()
                .is_some_and(Vec::is_empty)
    );

    // Invalid JSON is rejected before any event is recorded.
    let malformed_event =
        crate::route_http_request("POST", "/api/v0/events/Noop", None, "{", &state)
            .await
            .expect("malformed slskd event");
    record!(
        "POST",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        malformed_event.status == "400 Bad Request"
    );

    // Restart requests are latches owned by the current process. Durable
    // compatibility rows and boot rehydration must keep that flag cleared.
    let restart = crate::route_http_request("PUT", "/api/v0/application", None, "{}", &state)
        .await
        .expect("persist slskd restart request");
    let persisted_restart = db
        .get_runtime_compat_state()
        .await
        .expect("read persisted restart state")
        .expect("runtime compatibility row");
    let rehydrated_restart = crate::RuntimeCompatState::from_persisted(&persisted_restart);
    record!(
        "PUT",
        "/api/v0/application",
        "restart-persistence-or-reset",
        restart.status == "204 No Content"
            && state.runtime.read().await.application_restart_requested
            && !persisted_restart.application_restart_requested
            && !rehydrated_restart.application_restart_requested
    );

    // DELETE is safe to repeat and persists the reset state rather than
    // leaving a stale restart request behind.
    let shutdown = crate::route_http_request("DELETE", "/api/v0/application", None, "", &state)
        .await
        .expect("persist slskd shutdown reset");
    let shutdown_row = db
        .get_runtime_compat_state()
        .await
        .expect("read persisted shutdown state")
        .expect("runtime compatibility shutdown row");
    record!(
        "DELETE",
        "/api/v0/application",
        "restart-persistence-or-reset",
        shutdown.status == "204 No Content" && !shutdown_row.application_restart_requested
    );
    let repeated_shutdown =
        crate::route_http_request("DELETE", "/api/v0/application", None, "", &state)
            .await
            .expect("repeat slskd shutdown reset");
    record!(
        "DELETE",
        "/api/v0/application",
        "concurrency-and-idempotency",
        repeated_shutdown.status == "204 No Content"
            && db
                .get_runtime_compat_state()
                .await
                .expect("read repeated shutdown state")
                .is_some_and(|row| !row.application_restart_requested)
    );

    // Events use the same durable database row as the boot rehydration
    // path.  The fresh store check avoids crediting a write that only
    // changed the in-memory projection.
    let event = crate::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""durable-event""#,
        &state,
    )
    .await
    .expect("durable slskd event");
    let persisted_events = db.list_events(10, 0).await.expect("list durable events");
    let rehydrated_events =
        crate::EventStore::from_persisted(persisted_events.clone(), crate::EVENT_HISTORY_LIMIT);
    record!(
        "POST",
        "/api/v0/events",
        "restart-persistence-or-reset",
        event.status == "201 Created"
            && persisted_events
                .iter()
                .any(|row| row.detail.as_deref() == Some("durable-event"))
            && rehydrated_events
                .controller_json(None)
                .contains("durable-event")
    );

    // Two distinct event writes are serialized by the real store and
    // both survive the raw durable readback.
    let event_responses = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-a""#,
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-b""#,
            &state,
        ),
    ])
    .await;
    let concurrent_events = db.list_events(10, 0).await.expect("list concurrent events");
    record!(
        "POST",
        "/api/v0/events",
        "concurrency-and-idempotency",
        event_responses.iter().all(|response| response
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created"))
            && concurrent_events
                .iter()
                .filter_map(|row| row.detail.as_deref())
                .collect::<std::collections::BTreeSet<_>>()
                .is_superset(&std::collections::BTreeSet::from([
                    "concurrent-a",
                    "concurrent-b"
                ]))
    );

    // Closing the real database exercises the route's recoverable
    // persistence error rather than a synthetic response.
    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("event failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_event = crate::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""failure""#,
        &failure_state,
    )
    .await
    .expect("slskd event persistence failure response");
    record!(
        "POST",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        failed_event.status == "503 Service Unavailable"
            && failed_event.body.contains("persistence")
    );

    // Telemetry reports have explicit empty projections and query
    // validation.  Keep the time range outside the fixture window so
    // the empty assertions cannot accidentally pass from seeded state.
    let (telemetry_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let telemetry_empty_cases = [
        ("/api/v0/telemetry/metrics", "GET"),
        ("/api/v0/telemetry/metrics/kpis", "GET"),
        ("/api/v0/telemetry/reports/transfers/summary?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z", "GET"),
        ("/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60", "GET"),
        ("/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z", "GET"),
        ("/api/v0/telemetry/reports/transfers/exceptions?direction=Download&username=none", "GET"),
        ("/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download&username=none", "GET"),
        ("/api/v0/telemetry/reports/transfers/directories?username=none", "GET"),
        ("/api/v0/telemetry/reports/transfers/users/none", "GET"),
    ];
    for (path, method) in telemetry_empty_cases {
        let response = crate::route_http_request(method, path, None, "", &telemetry_state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        let body = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let empty_shape = if path.contains("/metrics") {
            response.status == "200 OK" && (body.is_object() || response.body.contains("# HELP"))
        } else if path.contains("/summary") {
            response.status == "200 OK"
                && body["Download"].is_object()
                && body["Upload"].is_object()
        } else if path.contains("/histogram") {
            response.status == "200 OK" && body.is_object()
        } else if path.contains("/users/") {
            response.status == "200 OK" && body["count"] == 0
        } else {
            response.status == "200 OK" && body.as_array().is_some_and(Vec::is_empty)
        };
        let route = path.split('?').next().unwrap_or(path);
        record!("GET", route, "missing-empty-or-conflict-state", empty_shape);
    }

    for (path, route) in [
        (
            "/api/v0/telemetry/reports/transfers/leaderboard?direction=NoSuchDirection",
            "/api/v0/telemetry/reports/transfers/leaderboard",
        ),
        (
            "/api/v0/telemetry/reports/transfers/exceptions?direction=NoSuchDirection",
            "/api/v0/telemetry/reports/transfers/exceptions",
        ),
        (
            "/api/v0/telemetry/reports/transfers/histogram?interval=0",
            "/api/v0/telemetry/reports/transfers/histogram",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &telemetry_state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_core_failure_restart_and_empty_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd core failure ledger"),
    )
    .expect("write slskd core failure ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd core failure/restart mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_users_and_shares() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "peer=127.0.0.1:2234"),
    );
    state.session.write().await.state = "connected";
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

    {
        let mut users = state.users.write().await;
        users.records.push(crate::UserRecord {
            username: "peer".to_owned(),
            watched: true,
            status: Some("Online".to_owned()),
            privileged: true,
            average_speed: Some(1_024),
            upload_count: Some(3),
            file_count: Some(12),
            directory_count: Some(4),
            updated_at: crate::unix_timestamp(),
        });
    }
    let info = crate::route_http_request("GET", "/api/v0/users/peer/info", None, "", &state)
        .await
        .expect("slskd user info");
    let info_json = serde_json::from_str::<serde_json::Value>(&info.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "nominal-status-headers-body",
        info.status == "200 OK"
            && info_json.as_object().is_some_and(|object| {
                object.len() == 6
                    && object["description"] == ""
                    && object["hasFreeUploadSlot"] == true
                    && object["hasPicture"] == false
                    && object["picture"].is_null()
                    && object["queueLength"] == 0
                    && object["uploadSlots"] == 0
            })
    );
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "populated-dynamic-state",
        info.status == "200 OK"
            && info_json["hasFreeUploadSlot"] == true
            && info_json["uploadSlots"] == 0
    );
    let missing_info =
        crate::route_http_request("GET", "/api/v0/users/missing/info", None, "", &state)
            .await
            .expect("slskd missing user info");
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "missing-empty-or-conflict-state",
        missing_info.status == "404 Not Found"
    );

    let status = crate::route_http_request("GET", "/api/v0/users/peer/status", None, "", &state)
        .await
        .expect("slskd user status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "nominal-status-headers-body",
        status.status == "200 OK"
            && status_json.as_object().is_some_and(|object| {
                object.len() == 2
                    && object["isPrivileged"] == true
                    && object["presence"] == "Online"
            })
    );
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "populated-dynamic-state",
        status.status == "200 OK" && status_json["presence"] == "Online"
    );
    let missing_status =
        crate::route_http_request("GET", "/api/v0/users/missing/status", None, "", &state)
            .await
            .expect("slskd missing user status");
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "missing-empty-or-conflict-state",
        missing_status.status == "404 Not Found"
    );

    let endpoint =
        crate::route_http_request("GET", "/api/v0/users/peer/endpoint", None, "", &state)
            .await
            .expect("slskd user endpoint");
    let endpoint_json =
        serde_json::from_str::<serde_json::Value>(&endpoint.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "nominal-status-headers-body",
        endpoint.status == "200 OK"
            && endpoint_json.as_object().is_some_and(|object| {
                object.len() == 3
                    && object["addressFamily"] == "IPv4"
                    && object["address"] == "127.0.0.1"
                    && object["port"] == 2234
            })
    );
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "populated-dynamic-state",
        endpoint.status == "200 OK"
            && endpoint_json["address"] == "127.0.0.1"
            && endpoint_json["port"] == 2234
    );

    let unknown_endpoint =
        crate::route_http_request("GET", "/api/v0/users/unknown/endpoint", None, "", &state)
            .await
            .expect("slskd unknown user endpoint");
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "missing-empty-or-conflict-state",
        unknown_endpoint.status == "404 Not Found"
    );

    {
        let mut shares = state.shares.write().await;
        shares.roots.clear();
        shares.entries.clear();
        shares.local_paths.clear();
        shares.roots.push(crate::ShareRoot {
            label: "Virtual".to_owned(),
            local_path: PathBuf::from("/srv/music"),
            raw: "Virtual".to_owned(),
            directories: 1,
            files: 1,
            bytes: 42,
            extensions: vec![crate::ShareExtensionSummary {
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
    }
    let share_id = crate::share_root_id("Virtual");
    let shares = crate::route_http_request("GET", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd shares");
    let shares_json = serde_json::from_str::<serde_json::Value>(&shares.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares",
        "nominal-status-headers-body",
        shares.status == "200 OK"
            && shares_json["local"]
                .as_array()
                .is_some_and(|roots| roots.len() == 1)
    );
    record!(
        "GET",
        "/api/v0/shares",
        "populated-dynamic-state",
        shares.status == "200 OK"
            && shares_json["local"][0]["id"] == share_id
            && shares_json["local"][0]["files"] == 1
    );

    let share = crate::route_http_request(
        "GET",
        &format!("/api/v0/shares/{share_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd share detail");
    let share_json = serde_json::from_str::<serde_json::Value>(&share.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "nominal-status-headers-body",
        share.status == "200 OK" && share_json["id"] == share_id
    );
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "populated-dynamic-state",
        share.status == "200 OK" && share_json["files"] == 1 && share_json["directories"] == 1
    );

    for (path, route) in [
        ("/api/v0/shares/contents", "/api/v0/shares/contents"),
        (
            &format!("/api/v0/shares/{share_id}/contents"),
            "/api/v0/shares/{id}/contents",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && json.as_array().is_some_and(|rows| !rows.is_empty())
        );
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && json[0]["files"]
                    .as_array()
                    .is_some_and(|files| !files.is_empty())
        );
    }

    for path in ["/api/v0/shares/missing", "/api/v0/shares/missing/contents"] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let route = if path.ends_with("/contents") {
            "/api/v0/shares/{id}/contents"
        } else {
            "/api/v0/shares/{id}"
        };
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    let rescan = crate::route_http_request("PUT", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd share rescan");
    record!(
        "PUT",
        "/api/v0/shares",
        "nominal-status-headers-body",
        rescan.status == "200 OK" && rescan.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/shares",
        "mutation-side-effects-and-readback",
        rescan.status == "200 OK" && state.share_lifecycle.read().await.ready
    );

    let (restarted_share_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let restarted_rescan =
        crate::route_http_request("PUT", "/api/v0/shares", None, "", &restarted_share_state)
            .await
            .expect("restarted slskd share rescan");
    record!(
        "PUT",
        "/api/v0/shares",
        "restart-persistence-or-reset",
        restarted_rescan.status == "200 OK"
            && restarted_rescan.body.is_empty()
            && restarted_share_state.share_lifecycle.read().await.ready
    );

    let cancel = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .expect("slskd share scan cancel");
    record!(
        "DELETE",
        "/api/v0/shares",
        "missing-empty-or-conflict-state",
        cancel.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_users_shares.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd users/shares ledger"),
    )
    .expect("write slskd users/shares ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd users/shares controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
