//! Controller full relay differential fixtures ownership.

use super::*;

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_relay_open_cases_impl() {
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

    async fn configured_controller() -> (Arc<crate::AppState>, String, u64) {
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
                crate::config::RelayAgentSettings {
                    instance_name: "edge-one".to_owned(),
                    secret: secret.clone(),
                    cidr: "127.0.0.1/32".to_owned(),
                },
            );
        }
        let now = crate::unix_timestamp();
        let challenge = state
            .relay
            .write()
            .await
            .protocol
            .issue_challenge("slskdn-relay-connection", now);
        let credential = crate::relay::credential_for_target(
            crate::ControllerProfile::Native,
            &secret,
            "edge-one",
            &challenge,
        );
        let settings = state.advanced_networking.read().await.relay.clone();
        assert!(state.relay.write().await.protocol.authenticate_agent(
            &settings,
            crate::relay::credential_scheme(crate::ControllerProfile::Native,),
            "slskdn-relay-connection",
            "edge-one",
            &credential,
            "127.0.0.1".parse().expect("relay test address"),
            now,
        ));
        (state, secret, now)
    }

    async fn live_get(state: Arc<crate::AppState>, path: &str) -> Vec<u8> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(crate::handle_http_stream(
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
        crate::route_http_request("PUT", "/api/v0/relay/agent/extra", None, "", &agent_state)
            .await
            .expect("malformed relay PUT");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "malformed-path-query-or-body",
        malformed_put.status == "404 Not Found"
    );
    let started = crate::route_http_request("PUT", "/api/v0/relay/agent", None, "", &agent_state)
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
    let restarted = crate::route_http_request(
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
        crate::route_http_request("PUT", "/api/v0/relay/agent", None, "", &agent_state),
        crate::route_http_request("PUT", "/api/v0/relay/agent", None, "", &agent_state),
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

    let malformed_delete = crate::route_http_request(
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
        crate::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &agent_state)
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
        crate::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &agent_state),
        crate::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &agent_state),
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
    let restarted_delete = crate::route_http_request(
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
    let download_credential = crate::relay::credential_for_target(
        crate::ControllerProfile::Native,
        &secret,
        "edge-one",
        &download_token,
    );
    let download_headers = crate::RequestSecurityHeaders {
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(download_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..crate::RequestSecurityHeaders::default()
    };
    let downloads_root = crate::effective_downloads_dir(&controller_state);
    fs::create_dir_all(downloads_root.join("Relay")).expect("relay download directory");
    fs::write(downloads_root.join("Relay/Open.txt"), b"relay-open-payload")
        .expect("relay download fixture");
    let download = Box::pin(crate::route_http_request_with_headers(
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
    let mut download_stream = crate::open_relay_controller_download(
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
    let malformed_download = Box::pin(crate::route_http_request_with_headers(
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
    let runtime_download = Box::pin(crate::route_http_request_with_headers(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &controller_state,
        crate::RequestSecurityHeaders {
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some("invalid-relay-credential".to_owned()),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..crate::RequestSecurityHeaders::default()
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
    let upload_credential = crate::relay::credential_for_target(
        crate::ControllerProfile::Native,
        &secret,
        "edge-one",
        &upload_token.to_string(),
    );
    let upload_body = "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Upload.flac\"\r\n\r\nrelay-upload-payload\r\n--relay--\r\n";
    let upload_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(upload_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..crate::RequestSecurityHeaders::default()
    };
    let upload = Box::pin(crate::route_http_request_with_headers(
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
    let malformed_upload = Box::pin(crate::route_http_request_with_headers(
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
    let runtime_upload = crate::versioned_relay_request(
        "POST",
        &format!("/api/v0/relay/controller/files/{runtime_upload_token}"),
        upload_body,
        &crate::RequestSecurityHeaders {
            content_type: Some("multipart/form-data; boundary=relay".to_owned()),
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some("invalid-relay-credential".to_owned()),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..crate::RequestSecurityHeaders::default()
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
    let restarted_upload = crate::versioned_relay_request(
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
    let concurrent_upload_credential = crate::relay::credential_for_target(
        crate::ControllerProfile::Native,
        &secret,
        "edge-one",
        &concurrent_upload_token.to_string(),
    );
    let concurrent_upload_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(concurrent_upload_credential),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..crate::RequestSecurityHeaders::default()
    };
    let concurrent_upload_body = "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Concurrent.flac\"\r\n\r\nrelay-upload-payload\r\n--relay--\r\n";
    let concurrent_uploads = futures_util::future::join_all([
        crate::versioned_relay_request(
            "POST",
            &format!("/api/v0/relay/controller/files/{concurrent_upload_token}"),
            concurrent_upload_body,
            &concurrent_upload_headers,
            &controller_state,
        ),
        crate::versioned_relay_request(
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
    let share_credential = crate::relay::credential_for_target(
        crate::ControllerProfile::Native,
        &secret,
        "edge-one",
        &share_token,
    );
    let share_source = controller_state
        .config
        .state_dir
        .join("relay-share-source.db");
    crate::relay::write_share_database(
        &share_source,
        crate::ControllerProfile::Native,
        &[crate::relay::RemoteShare {
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
    let share_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(share_credential.clone()),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..crate::RequestSecurityHeaders::default()
    };
    let share_upload = crate::versioned_relay_request_bytes(
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
    let malformed_share = crate::route_http_request_with_headers(
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
    let runtime_share = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{runtime_share_token}"),
        &share_body,
        &crate::RequestSecurityHeaders {
            content_type: Some("multipart/form-data; boundary=relay".to_owned()),
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some("invalid-relay-credential".to_owned()),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..crate::RequestSecurityHeaders::default()
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
    let restarted_share = crate::versioned_relay_request_bytes(
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
        .issue_share_upload_token("edge-one", crate::unix_timestamp())
        .expect("concurrent relay share token");
    let concurrent_share_credential = crate::relay::credential_for_target(
        crate::ControllerProfile::Native,
        &secret,
        "edge-one",
        &concurrent_share_token,
    );
    let concurrent_share_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(concurrent_share_credential),
        remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
        ..crate::RequestSecurityHeaders::default()
    };
    let concurrent_shares = futures_util::future::join_all([
        crate::versioned_relay_request_bytes(
            "POST",
            &format!("/api/v0/relay/controller/shares/{concurrent_share_token}"),
            &share_body,
            &concurrent_share_headers,
            &restarted_share_state,
        ),
        crate::versioned_relay_request_bytes(
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
        crate::route_http_request("GET", stream_route, None, "", &controller_state)
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
    let content_id = crate::stable_content_hash("Remote/Agent.flac", 6).to_string();
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
        tokio::sync::mpsc::channel(crate::relay::HUB_OUTBOUND_QUEUE_CAPACITY);
    crate::relay::register_hub_connection("slskdn-relay-connection".to_owned(), hub_sender);
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
    let stream_credential = crate::relay::credential_for_target(
        crate::ControllerProfile::Native,
        &secret,
        "edge-one",
        &stream_token,
    );
    let stream_upload = crate::versioned_relay_request(
        "POST",
        &format!("/api/v0/relay/controller/files/{stream_token}"),
        "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Remote/Agent.flac\"\r\n\r\nstream\r\n--relay--\r\n",
        &crate::RequestSecurityHeaders {
            content_type: Some("multipart/form-data; boundary=relay".to_owned()),
            x_relay_agent: Some("edge-one".to_owned()),
            x_relay_credential: Some(stream_credential),
            remote_addr: Some("127.0.0.1:1".parse().expect("relay remote address")),
            ..crate::RequestSecurityHeaders::default()
        },
        &controller_state,
    )
    .await
    .expect("relay stream upload");
    let stream_response = stream_task.await.expect("relay stream task");
    crate::relay::unregister_hub_connection("slskdn-relay-connection");
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

    let _ = fs::remove_dir_all(crate::effective_downloads_dir(&controller_state));
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
