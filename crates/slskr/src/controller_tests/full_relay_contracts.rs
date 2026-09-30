//! Controller full relay contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn relay_stream_requests_agent_and_serves_matching_upload() {
    let (state, secret, _now) = configured_relay_test_state().await;
    state
        .media_services
        .write()
        .await
        .features
        .streaming_relay_fallback = true;
    let source = state.config.state_dir.join("agent-source.flac");
    let payload = b"relay-stream-payload";
    fs::write(&source, payload).expect("relay source");
    let remote_filename = "Music/Agent.flac";
    state
        .relay
        .write()
        .await
        .protocol
        .record_share_upload(
            uuid::Uuid::new_v4(),
            "edge-one".to_owned(),
            1,
            vec![crate::relay::RemoteShare {
                filename: remote_filename.to_owned(),
                size: payload.len() as u64,
            }],
            state.config.state_dir.join("remote-shares.db"),
            _now,
        )
        .expect("record relay stream share upload");
    let content_id = crate::stable_content_hash(remote_filename, payload.len() as u64).to_string();

    let (hub_sender, mut hub_receiver) =
        tokio::sync::mpsc::channel(crate::relay::HUB_OUTBOUND_QUEUE_CAPACITY);
    crate::relay::register_hub_connection("connection-1".to_owned(), hub_sender);
    let open_state = Arc::clone(&state);
    let open = tokio::spawn(async move {
        crate::open_relay_controller_stream(&open_state, &content_id, Some("agentName=edge-one"))
            .await
    });
    let info_invocation = hub_receiver
        .recv()
        .await
        .expect("request file info invocation");
    let info_invocation: serde_json::Value =
        serde_json::from_str(&info_invocation).expect("info invocation");
    assert_eq!(info_invocation["target"], "RequestFileInfo");
    let info_token = info_invocation["arguments"][1]
        .as_str()
        .expect("file info token")
        .parse::<uuid::Uuid>()
        .expect("file info UUID");
    assert!(state.relay.write().await.protocol.complete_file_info(
        "connection-1",
        info_token,
        true,
        payload.len() as u64
    ));

    let invocation = hub_receiver
        .recv()
        .await
        .expect("request file upload invocation");
    let invocation: serde_json::Value = serde_json::from_str(&invocation).expect("invocation");
    assert_eq!(invocation["target"], "RequestFileUpload");
    assert_eq!(invocation["arguments"][0], remote_filename);
    let token = invocation["arguments"][2]
        .as_str()
        .expect("file stream token")
        .to_owned();
    let credential = crate::relay::credential_for_test(secret, "edge-one", &token);
    let headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let body = b"--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Music/Agent.flac\"\r\n\r\nrelay-stream-payload\r\n--relay--\r\n";
    let response = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{token}"),
        body,
        &headers,
        &state,
    )
    .await
    .expect("relay upload response");
    assert_eq!(response.status, "200 OK");
    let mut stream = open
        .await
        .expect("relay stream task")
        .expect("relay stream");
    let mut received = Vec::new();
    std::io::Read::read_to_end(&mut stream.file, &mut received).expect("read relay stream");
    assert_eq!(received, payload);
    if let Some(path) = stream.cleanup_path {
        let _ = fs::remove_file(path);
    }
    crate::relay::unregister_hub_connection("connection-1");
    let _ = fs::remove_file(source);
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn relay_signalr_hub_authenticates_agent_and_issues_share_token() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let (state, secret, _) = configured_relay_test_state().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("relay hub listener");
    let address = listener.local_addr().expect("relay hub address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("relay hub connection");
        crate::handle_http_connection(stream, server_state).await
    });

    // The test listener is bound to loopback only; TLS is unavailable on this fixture.
    let (mut socket, _) = connect_async(format!("ws://{address}/hub/relay")) // nosemgrep: javascript.lang.security.detect-insecure-websocket.detect-insecure-websocket
        .await
        .expect("relay SignalR connection");
    socket
        .send(Message::Text("{}\x1e".to_owned().into()))
        .await
        .expect("relay SignalR handshake");
    let handshake = socket
        .next()
        .await
        .expect("handshake response")
        .expect("handshake websocket message");
    assert_eq!(handshake.to_text().expect("handshake text"), "{}\x1e");

    let challenge = socket
        .next()
        .await
        .expect("challenge response")
        .expect("challenge websocket message");
    let challenge = challenge
        .to_text()
        .expect("challenge text")
        .trim_end_matches('\x1e');
    let challenge: serde_json::Value = serde_json::from_str(challenge).expect("challenge JSON");
    assert_eq!(challenge["target"], "Challenge");
    let challenge = challenge["arguments"][0].as_str().expect("challenge token");
    let credential = crate::relay::credential_for_test(secret, "edge-one", challenge);
    let login = serde_json::json!({
        "type": 1,
        "invocationId": "1",
        "target": "Login",
        "arguments": ["edge-one", credential],
    });
    socket
        .send(Message::Text(format!("{}\x1e", login).into()))
        .await
        .expect("relay login");
    let login_result = socket
        .next()
        .await
        .expect("login completion")
        .expect("login websocket message");
    let login_result: serde_json::Value = serde_json::from_str(
        login_result
            .to_text()
            .expect("login completion text")
            .trim_end_matches('\x1e'),
    )
    .expect("login completion JSON");
    assert_eq!(login_result["type"], 3);
    assert!(login_result.get("error").is_none());

    let begin = serde_json::json!({
        "type": 1,
        "invocationId": "2",
        "target": "BeginShareUpload",
        "arguments": [],
    });
    socket
        .send(Message::Text(format!("{}\x1e", begin).into()))
        .await
        .expect("relay share token request");
    let token_result = socket
        .next()
        .await
        .expect("share token completion")
        .expect("share token websocket message");
    let token_result: serde_json::Value = serde_json::from_str(
        token_result
            .to_text()
            .expect("share token completion text")
            .trim_end_matches('\x1e'),
    )
    .expect("share token completion JSON");
    let token = token_result["result"].as_str().expect("share upload token");
    let token_credential = crate::relay::credential_for_test(secret, "edge-one", token);
    let settings = state.advanced_networking.read().await.relay.clone();
    assert!(state
        .relay
        .write()
        .await
        .protocol
        .validate_share_upload(
            &settings,
            crate::relay::credential_scheme(state.config.controller_profile),
            uuid::Uuid::parse_str(token).expect("share token UUID"),
            &token_credential,
            crate::unix_timestamp(),
        )
        .is_some());

    let download_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_download_tokens("Relay/Completed.flac", crate::unix_timestamp())
        .into_iter()
        .next()
        .expect("download notification token")
        .1;
    assert!(crate::relay::send_hub_invocation(
        &state.relay.read().await.protocol,
        "edge-one",
        "NotifyFileDownloadCompleted",
        vec![
            serde_json::Value::String("Relay/Completed.flac".to_owned()),
            serde_json::Value::String(download_token),
        ],
    ));
    let notification = socket
        .next()
        .await
        .expect("download notification")
        .expect("download notification websocket message");
    let notification: serde_json::Value = serde_json::from_str(
        notification
            .to_text()
            .expect("download notification text")
            .trim_end_matches('\x1e'),
    )
    .expect("download notification JSON");
    assert_eq!(notification["target"], "NotifyFileDownloadCompleted");

    socket.close(None).await.expect("relay websocket close");
    server
        .await
        .expect("relay hub server task")
        .expect("relay hub server");
}
