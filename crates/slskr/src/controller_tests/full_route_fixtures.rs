//! Controller full route fixtures ownership.

use super::*;

pub(super) async fn serve_json_fixture(
    listener: &tokio::net::TcpListener,
    response: serde_json::Value,
) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut stream, _) = listener.accept().await.expect("accept fixture request");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let count = stream
            .read(&mut buffer)
            .await
            .expect("read fixture request");
        assert!(count > 0, "fixture request ended before headers");
        request.extend_from_slice(&buffer[..count]);
        if let Some(position) = request.windows(4).position(|row| row == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let headers = String::from_utf8_lossy(&request[..header_end]).to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or_default();
    while request.len() < header_end.saturating_add(content_length) {
        let count = stream.read(&mut buffer).await.expect("read fixture body");
        assert!(count > 0, "fixture request ended before body");
        request.extend_from_slice(&buffer[..count]);
    }
    let body = response.to_string();
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(reply.as_bytes())
        .await
        .expect("write fixture response");
    String::from_utf8(request).expect("fixture request UTF-8")
}

pub(super) static APPLICATION_DUMP_ENV_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_tokens_use_headers_and_content_bound_stream_tickets_impl() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "route-token"),
    );
    let api_authorization = Some("Bearer route-token");
    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        api_authorization,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let item = crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        api_authorization,
        r#"{"content_id":"content/one","title":"track.flac","kind":"Audio"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(item.status, "201 Created");
    let grant = crate::route_http_request(
        "POST",
        "/api/share-grants",
        api_authorization,
        &format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}"),
        &state,
    )
    .await
    .unwrap();
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let issued = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        api_authorization,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .unwrap();
    let issued_json = serde_json::from_str::<serde_json::Value>(&issued.body).unwrap();
    let token = issued_json["token"].as_str().unwrap().to_owned();
    assert_eq!(issued_json["expiresInSeconds"], 600);
    assert!(!issued.body.contains("?token="));

    let query_token = crate::route_http_request(
        "GET",
        &format!(
            "/api/share-grants/{grant_id}/manifest?token={}",
            crate::url_encode(&token)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(query_token.status, "400 Bad Request");

    let share_headers = crate::RequestSecurityHeaders {
        x_share_token: Some(token.clone()),
        ..Default::default()
    };
    let manifest = crate::route_http_request_with_headers(
        "GET",
        &format!("/api/share-grants/{grant_id}/manifest"),
        None,
        "",
        &state,
        share_headers.clone(),
    )
    .await
    .unwrap();
    assert_eq!(manifest.status, "200 OK");
    let manifest_json = serde_json::from_str::<serde_json::Value>(&manifest.body).unwrap();
    assert_eq!(manifest_json["itemCount"], 1);
    assert_eq!(manifest_json["items"][0]["contentId"], "content/one");
    assert!(!manifest.body.contains(&token));

    let query_stream = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fone?token={}",
            crate::url_encode(&token)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(query_stream.status, "400 Bad Request");

    let ticket_response = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/streams/content%2Fone/share-ticket",
        None,
        "",
        &state,
        share_headers,
    )
    .await
    .unwrap();
    assert_eq!(ticket_response.status, "200 OK");
    let ticket = serde_json::from_str::<serde_json::Value>(&ticket_response.body).unwrap()
        ["ticket"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!ticket_response.body.contains(&token));

    let stream = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fone?ticket={}",
            crate::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(stream.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stream.body).unwrap()["status"],
        "available"
    );

    let wrong_content = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/different?ticket={}",
            crate::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(wrong_content.status, "401 Unauthorized");

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        api_authorization,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state.share_access_tokens.read().await.records.is_empty());
    assert!(state.stream_tickets.read().await.records.is_empty());
    let revoked_stream = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fone?ticket={}",
            crate::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(revoked_stream.status, "401 Unauthorized");
}

#[cfg(feature = "full-controller-tests")]
pub(super) async fn primary_stream_route_serves_authenticated_file_ranges_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-stream-share-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create stream share root");
    std::fs::write(root.join("track.flac"), b"0123456789").expect("write stream fixture");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "stream-token")
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let virtual_filename = {
        let shares = state.shares.read().await;
        shares.entries[0].filename.clone()
    };
    let ticket = state
        .stream_tickets
        .write()
        .await
        .issue(
            "share",
            "share:test",
            virtual_filename.clone(),
            virtual_filename.clone(),
            Some("friend".to_owned()),
            10,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue stream ticket")
        .0;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stream server");
    let address = listener.local_addr().expect("stream server address");
    let server_state = Arc::clone(&state);
    let server = std::thread::Builder::new()
        .name("primary-stream-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create primary stream server runtime")
                .block_on(async move {
                    let (stream, _) = listener.accept().await.expect("accept stream request");
                    crate::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve stream response");
                });
        })
        .expect("spawn primary stream server");
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect stream client");
    client
        .write_all(
            format!(
                "GET /api/v0/streams/{}?ticket={} HTTP/1.1\r\nHost: localhost\r\nRange: bytes=3-6\r\nConnection: close\r\n\r\n",
                crate::url_encode(&virtual_filename),
                crate::url_encode(&ticket)
            )
            .as_bytes(),
        )
        .await
        .expect("write stream request");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read stream response");
    server.join().expect("stream server task");
    std::fs::remove_dir_all(root).expect("remove stream fixture");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("stream response header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("stream response headers");
    assert!(
        headers.starts_with("HTTP/1.1 206 Partial Content\r\n"),
        "unexpected stream response: {headers}"
    );
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Length: 4\r\n"));
    assert!(headers.contains("Content-Range: bytes 3-6/10\r\n"));
    assert!(headers.contains("Accept-Ranges: bytes\r\n"));
    assert_eq!(&raw[split + 4..], b"3456");
}

#[cfg(feature = "full-controller-tests")]
pub(super) async fn preview_ticket_get_is_anonymous_and_streams_local_audio_without_ranges_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let root = std::env::temp_dir().join(format!(
        "slskr-peer-preview-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create preview share root");
    std::fs::write(root.join("preview.flac"), b"preview-bytes").expect("write preview fixture");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "stream-token")
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let virtual_filename = state.shares.read().await.entries[0].filename.clone();
    let ticket = state
        .stream_tickets
        .write()
        .await
        .issue(
            "peer",
            "local-share",
            virtual_filename.clone(),
            virtual_filename,
            Some("peer".to_owned()),
            13,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue peer preview ticket")
        .0;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind preview server");
    let address = listener.local_addr().expect("preview server address");
    let server_state = Arc::clone(&state);
    let server = std::thread::Builder::new()
        .name("local-preview-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create local preview server runtime")
                .block_on(async move {
                    let (stream, _) = listener.accept().await.expect("accept preview request");
                    crate::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve preview response");
                });
        })
        .expect("spawn local preview server");
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect preview client");
    client
        .write_all(
            format!(
                "GET /api/v0/peer-streams/{} HTTP/1.1\r\nHost: localhost\r\nRange: bytes=2-4\r\nConnection: close\r\n\r\n",
                crate::url_encode(&ticket)
            )
            .as_bytes(),
        )
        .await
        .expect("write preview request");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read preview response");
    server.join().expect("preview server task");
    std::fs::remove_dir_all(root).expect("remove preview fixture");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("preview response header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("preview response headers");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Length: 13\r\n"));
    assert!(headers.contains("Accept-Ranges: none\r\n"));
    assert_eq!(&raw[split + 4..], b"preview-bytes");
}

#[cfg(feature = "full-controller-tests")]
pub(super) async fn peer_preview_ticket_streams_remote_soulseek_bytes_without_transfer_record_impl()
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let peer_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind peer preview source");
    let peer_address = peer_listener.local_addr().expect("peer preview address");
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        &format!("friend={peer_address}"),
    ));
    let ticket = state
        .stream_tickets
        .write()
        .await
        .issue(
            "peer",
            "peer-preview",
            "Remote/Song.flac".to_owned(),
            "Remote/Song.flac".to_owned(),
            Some("friend".to_owned()),
            4,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue remote peer preview ticket")
        .0;
    let peer = tokio::spawn(async move {
        let (stream, _) = peer_listener
            .accept()
            .await
            .expect("accept preview negotiation");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("preview negotiation init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut messages = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            messages.receive().await.expect("preview transfer request"),
            crate::PeerMessage::TransferRequest(crate::TransferRequest {
                filename_encoding: Default::default(),
                direction: 0,
                token: 1,
                filename: "Remote/Song.flac".to_owned(),
                size: None,
            })
        );
        messages
            .send(&crate::PeerMessage::TransferResponse(
                crate::TransferResponse::Allowed {
                    token: 1,
                    size: Some(4),
                },
            ))
            .await
            .expect("allow preview transfer");

        let (stream, _) = peer_listener
            .accept()
            .await
            .expect("accept preview file transfer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("preview file init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        file.send_token(1).await.expect("send preview token");
        assert_eq!(file.receive_offset().await.expect("preview offset"), 0);
        file.write_chunk(b"song")
            .await
            .expect("write preview bytes");
    });

    let http_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind peer preview HTTP server");
    let http_address = http_listener.local_addr().expect("preview HTTP address");
    let server_state = Arc::clone(&state);
    let http = std::thread::Builder::new()
        .name("peer-preview-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create peer preview server runtime")
                .block_on(async move {
                    let (stream, _) = http_listener
                        .accept()
                        .await
                        .expect("accept preview HTTP request");
                    crate::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve remote preview response");
                });
        })
        .expect("spawn peer preview server");
    let mut client = tokio::net::TcpStream::connect(http_address)
        .await
        .expect("connect remote preview client");
    client
        .write_all(
            format!(
                "GET /api/v0/peer-streams/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                crate::url_encode(&ticket)
            )
            .as_bytes(),
        )
        .await
        .expect("request remote preview");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read remote preview");
    http.join().expect("preview HTTP task");
    peer.await.expect("preview peer task");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("remote preview header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("remote preview headers");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Length: 4\r\n"));
    assert!(headers.contains("Accept-Ranges: none\r\n"));
    assert_eq!(&raw[split + 4..], b"song");
    assert!(state.transfers.read().await.entries.is_empty());
}

#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_preview_ticket_fetches_verifies_streams_and_removes_staging_file_impl() {
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let content = Arc::new(b"mesh-preview".to_vec());
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));
    let source_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh preview source");
    let source_address = source_listener.local_addr().expect("mesh source address");
    let source_content = Arc::clone(&content);
    let source = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = source_listener
                .accept()
                .await
                .expect("accept mesh range request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let count = stream.read(&mut buffer).await.expect("read mesh request");
                request.extend_from_slice(&buffer[..count]);
                if count == 0 || request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("mesh request UTF-8");
            let range = request
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("range"))
                .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                .expect("mesh range header");
            let (start, end) = range.split_once('-').expect("mesh range bounds");
            let start = start.parse::<usize>().expect("mesh range start");
            let end = end.parse::<usize>().expect("mesh range end");
            let body = &source_content[start..=end];
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                        body.len(),
                        source_content.len()
                    )
                    .as_bytes(),
                )
                .await
                .expect("write mesh range headers");
            stream.write_all(body).await.expect("write mesh range body");
        }
    });

    let (state, _receiver) = test_state();
    let ticket_response = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        &format!(
            r#"{{"contentId":"mesh-content","filename":"Remote/Mesh.flac","peerId":"mesh-peer","size":{},"expectedHash":"{expected_hash}","sourceUrl":"http://{source_address}/content"}}"#,
            content.len()
        ),
        &state,
    )
    .await
    .expect("create executable mesh ticket");
    assert_eq!(ticket_response.status, "200 OK");
    let stream_url = serde_json::from_str::<serde_json::Value>(&ticket_response.body).unwrap()
        ["streamUrl"]
        .as_str()
        .unwrap()
        .to_owned();
    let http_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh preview HTTP server");
    let http_address = http_listener
        .local_addr()
        .expect("mesh preview HTTP address");
    let server_state = Arc::clone(&state);
    let http = std::thread::Builder::new()
        .name("mesh-preview-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create mesh preview server runtime")
                .block_on(async move {
                    let (stream, _) = http_listener
                        .accept()
                        .await
                        .expect("accept mesh preview request");
                    crate::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve mesh preview response");
                });
        })
        .expect("spawn mesh preview server");
    let mut client = tokio::net::TcpStream::connect(http_address)
        .await
        .expect("connect mesh preview client");
    client
        .write_all(
            format!("GET {stream_url} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .expect("request mesh preview");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read mesh preview");
    http.join().expect("mesh preview HTTP task");
    source.await.expect("mesh preview source task");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("mesh preview header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("mesh preview headers");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Accept-Ranges: none\r\n"));
    assert_eq!(&raw[split + 4..], content.as_slice());
    let preview_dir = state.config.downloads_dir.join(".preview");
    assert_eq!(
        std::fs::read_dir(preview_dir)
            .expect("read preview staging directory")
            .count(),
        0
    );
}

#[cfg(feature = "full-controller-tests")]
pub(super) async fn listening_party_directory_ticket_streams_local_audio_ranges_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let root = std::env::temp_dir().join(format!(
        "slskr-listening-party-stream-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("create listening-party share root");
    std::fs::write(root.join("party.flac"), b"party-audio-bytes")
        .expect("write listening-party fixture");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let (content_id, filename) = {
        let shares = state.shares.read().await;
        let entry = shares.entries.first().expect("share fixture entry");
        (
            crate::stable_content_hash(&entry.filename, entry.size).to_string(),
            entry.filename.clone(),
        )
    };
    let party_id = "party:stream-audit";
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            "listening-party/pod:stream-audit/general".to_owned(),
            serde_json::json!({
                "partyId": party_id,
                "podId": "pod:stream-audit",
                "channelId": "general",
                "hostPeerId": "tester",
                "action": "play",
                "contentId": content_id,
                "title": "Party Track",
                "artist": "Party Artist",
                "serverTimeUnixMs": crate::unix_timestamp_millis(),
                "listed": true,
                "allowMeshStreaming": true,
            }),
        )
        .expect("persist listening-party fixture");

    let directory = crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
        .await
        .expect("list listening-party directory");
    assert_eq!(directory.status, "200 OK", "{}", directory.body);
    let stream_path = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap()[0]
        ["streamPath"]
        .as_str()
        .expect("directory stream path")
        .to_owned();
    assert!(stream_path.contains(&crate::url_encode(&content_id)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listening-party stream server");
    let address = listener
        .local_addr()
        .expect("listening-party stream address");
    let server_state = Arc::clone(&state);
    let server = std::thread::Builder::new()
        .name("listening-party-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create listening-party server runtime")
                .block_on(async move {
                    let (stream, _) = listener.accept().await.expect("accept radio request");
                    crate::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve radio response");
                });
        })
        .expect("spawn listening-party server");
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect radio client");
    client
        .write_all(
            format!(
                "GET {stream_path} HTTP/1.1\r\nHost: localhost\r\nRange: bytes=2-6\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .expect("write radio request");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read radio response");
    server.join().expect("radio server task");
    std::fs::remove_dir_all(root).expect("remove listening-party fixture");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("radio response header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("radio response headers");
    assert!(
        headers.starts_with("HTTP/1.1 206 Partial Content\r\n"),
        "{headers}"
    );
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Range: bytes 2-6/17\r\n"));
    assert_eq!(&raw[split + 4..], b"rty-a");
    assert!(!filename.is_empty());
}

#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_gateway_enabled_enforces_target_auth_and_origin_contract_impl() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "route-token")
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKD_MESH_GATEWAY_API_KEY", "gateway-key")
            .with("SLSKD_MESH_GATEWAY_CSRF_TOKEN", "csrf-token"),
    );
    assert_eq!(
        state.config.mesh_gateway.api_key.as_deref(),
        Some("gateway-key")
    );

    let remote = crate::RequestSecurityHeaders {
        remote_addr: Some("192.0.2.30:1234".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let unauthorized = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        None,
        "{}",
        &state,
        remote.clone(),
    ))
    .await
    .expect("remote auth response");
    assert_eq!(unauthorized.status, "401 Unauthorized");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unauthorized.body).unwrap(),
        serde_json::json!({
            "error": "unauthorized",
            "message": "Valid X-Slskdn-ApiKey header is required",
        })
    );

    let mut authorized = remote;
    authorized.x_gateway_api_key = Some("gateway-key".to_owned());
    assert!(crate::mesh_gateway_auth_failure(&state, &authorized).is_none());
    let unavailable = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        Some("Bearer route-token"),
        "{}",
        &state,
        authorized,
    ))
    .await
    .expect("authorized remote response");
    assert_eq!(unavailable.status, "503 Service Unavailable");

    let local = crate::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:1234".parse().unwrap()),
        origin: Some("https://evil.example".to_owned()),
        x_gateway_csrf: Some("csrf-token".to_owned()),
        ..crate::RequestSecurityHeaders::default()
    };
    let origin_denied = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        Some("Bearer route-token"),
        "{}",
        &state,
        local,
    ))
    .await
    .expect("origin response");
    assert_eq!(origin_denied.status, "403 Forbidden");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&origin_denied.body).unwrap(),
        serde_json::json!({
            "error": "origin_not_allowed",
            "message": "Cross-origin requests to localhost are not allowed by default",
        })
    );

    let valid_local = crate::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:1234".parse().unwrap()),
        origin: Some("https://localhost:3000".to_owned()),
        x_gateway_csrf: Some("csrf-token".to_owned()),
        ..crate::RequestSecurityHeaders::default()
    };
    let local_unavailable = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        Some("Bearer route-token"),
        "{}",
        &state,
        valid_local,
    ))
    .await
    .expect("valid local response");
    assert_eq!(local_unavailable.status, "503 Service Unavailable");
}

pub(super) async fn configured_api_token_protects_api_routes_impl() {
    let env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            &std::env::temp_dir().display().to_string(),
        )
        .with("SLSKR_API_TOKEN", "route-token");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("auth config");
    let (sender, _receiver) = mpsc::channel(8);
    let (event_tx, _) = tokio::sync::broadcast::channel(crate::EVENT_HISTORY_LIMIT);
    let rate_limiter = crate::rate_limit::RateLimiter::new(crate::rate_limit::RateLimitConfig {
        max_requests_anonymous: 1000,
        max_requests_authenticated: 5000,
        window_seconds: 60,
        enabled: true,
    });
    let share_index = crate::build_share_index(&config);
    let share_lifecycle = crate::ShareLifecycleState::from_snapshot(&share_index);

    let state = Arc::new(crate::AppState {
        controller_version: std::sync::RwLock::new(crate::ControllerVersionState::initial()),
        controller_cli_environment: BTreeMap::new(),
        log_level: RwLock::new(crate::logging::LogLevel::Info),
        runtime_credentials: RwLock::new(None),
        configured_credentials: RwLock::new(config.credentials()),
        controller_web_auth_username: std::sync::RwLock::new(
            config.controller_web_auth_username.clone(),
        ),
        controller_web_auth_password: std::sync::RwLock::new(
            config.controller_web_auth_password.clone(),
        ),
        controller_web_jwt_key_current: std::sync::RwLock::new(
            config.controller_web_jwt_key.clone(),
        ),
        session: RwLock::new(crate::SessionSnapshot::disconnected(&config)),
        server_address: std::sync::RwLock::new(config.server_address.clone()),
        connected_server_address: std::sync::RwLock::new(None),
        listeners: RwLock::new(crate::ListenerSnapshot::new(&config)),
        distributed_network: RwLock::new(crate::DistributedRuntime::new(
            config.username.as_deref(),
        )),
        distributed_persistence_snapshots: tokio::sync::watch::channel(
            crate::DistributedRuntime::new(config.username.as_deref()).persistence_snapshot(),
        )
        .0,
        distributed_persistence_status: tokio::sync::watch::channel(
            crate::DistributedPersistenceStatus {
                revision: 0,
                result: Ok(()),
            },
        )
        .1,
        soulseek_distributed_settings: RwLock::new(config.soulseek_distributed),
        shares: RwLock::new(share_index),
        share_settings: RwLock::new(config.share_settings.clone()),
        share_index_persistence_lock: tokio::sync::Mutex::new(()),
        share_settings_generation: std::sync::atomic::AtomicU64::new(0),
        core_workflow_settings: RwLock::new(config.core_workflow.clone()),
        advanced_networking: RwLock::new(config.advanced_networking.clone()),
        media_services: RwLock::new(config.media_services.clone()),
        share_lifecycle: RwLock::new(share_lifecycle),
        downloads_dir: std::sync::RwLock::new(config.downloads_dir.clone()),
        incomplete_dir: std::sync::RwLock::new(config.incomplete_dir.clone()),
        download_completed_path_template: std::sync::RwLock::new(
            config.download_completed_path_template.clone(),
        ),
        remote_file_management: std::sync::RwLock::new(config.remote_file_management),
        remote_configuration: std::sync::RwLock::new(config.remote_configuration),
        controller_no_config_watch: std::sync::RwLock::new(config.controller_no_config_watch),
        controller_case_sensitive_regex: std::sync::RwLock::new(
            config.controller_case_sensitive_regex,
        ),
        user_info_description: std::sync::RwLock::new(config.user_info_description.clone()),
        user_info_picture: std::sync::RwLock::new(config.user_info_picture.clone()),
        controller_options_validation_error: std::sync::RwLock::new(None),
        regular_listener_commands: None,
        obfuscated_listener_commands: None,
        advertised_port: std::sync::RwLock::new(config.advertised_port),
        obfuscated_advertised_port: std::sync::RwLock::new(config.obfuscated_advertised_port),
        searches: RwLock::new(crate::SearchStore::new()),
        users: RwLock::new(crate::UserStore::new()),
        user_persistence_lock: tokio::sync::Mutex::new(()),
        event_persistence_lock: tokio::sync::Mutex::new(()),
        mesh: RwLock::new(crate::MeshState::new()),
        capability_signing_key: crate::controller_capabilities::new_capability_signing_key()
            .expect("capability signing key"),
        content_discovery: RwLock::new(crate::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(crate::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(crate::BrowseStore::new()),
        browse_persistence_lock: tokio::sync::Mutex::new(()),
        remote_path_encodings: RwLock::new(crate::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(crate::MessageStore::new()),
        message_persistence_lock: tokio::sync::Mutex::new(()),
        managed_blacklist: RwLock::new(crate::ManagedBlacklistRuntime::new(
            config.managed_blacklist.clone(),
            config.controller_profile,
            config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(
            crate::compile_controller_regexes(
                &config.controller_search_request_filters,
                config.controller_case_sensitive_regex,
                config.controller_profile,
            )
            .unwrap(),
        ),
        integration_settings: RwLock::new(config.integrations.clone()),
        source_feed_import_history: RwLock::new(crate::SourceFeedImportHistoryStore::default()),
        source_feed_import_history_persistence_lock: tokio::sync::Mutex::new(()),
        lidarr_sync_state: RwLock::new(crate::LidarrSyncRuntimeState::new(
            &config.integrations.lidarr,
        )),
        lidarr_recent_imports: RwLock::new(BTreeMap::new()),
        lidarr_import_gate: tokio::sync::Semaphore::new(1),
        private_message_auto_response_settings: RwLock::new(
            config.private_message_auto_response.clone(),
        ),
        transfer_auto_retry_settings: RwLock::new(config.transfer_auto_retry.clone()),
        transfer_upload_settings: RwLock::new(config.transfer_upload.clone()),
        transfer_download_settings: RwLock::new(config.transfer_download.clone()),
        transfer_groups_settings: RwLock::new(config.transfer_groups.clone()),
        failed_upload_peer_cooldowns: RwLock::new(crate::UploadPeerCooldowns::default()),
        private_message_auto_responses: RwLock::new(
            crate::PrivateMessageAutoResponseTracker::default(),
        ),
        rooms: RwLock::new(crate::RoomStore::new()),
        room_persistence_lock: tokio::sync::Mutex::new(()),
        pod_join_replays: RwLock::new(crate::PodJoinReplayStore::default()),
        pod_membership_workflow: RwLock::new(crate::PodMembershipWorkflowStore::default()),
        pod_channels: RwLock::new(crate::pod_channels::PodChannelStore::empty(
            &config.state_dir,
        )),
        pods: RwLock::new(crate::pods::PodStore::empty(&config.state_dir)),
        port_forwarding: crate::port_forwarding::Manager::new(),
        private_gateway: None,
        dht: None,
        transfers: RwLock::new(crate::TransferQueue::new(&config)),
        events: RwLock::new(crate::EventStore::new(crate::EVENT_HISTORY_LIMIT)),
        event_tx: event_tx.clone(),
        webhooks: Arc::new(RwLock::new(crate::webhooks::WebhookManager::new())),
        webhook_deliveries: Arc::new(crate::Semaphore::new(crate::MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(crate::Semaphore::new(crate::MAX_SHARE_SCAN_TASKS)),
        share_scan_cancellation: Arc::new(std::sync::Mutex::new(None)),
        incoming_connections: Arc::new(crate::Semaphore::new(crate::MAX_INCOMING_CONNECTION_TASKS)),
        incoming_connection_ips: std::sync::Mutex::new(BTreeMap::new()),
        incoming_searches: Arc::new(crate::Semaphore::new(
            config.core_workflow.incoming_search.concurrency,
        )),
        incoming_search_queue_depth: crate::AtomicUsize::new(0),
        download_requests: Arc::new(crate::Semaphore::new(2)),
        download_batch_requests: Arc::new(crate::Semaphore::new(1)),
        websocket_connections: Arc::new(crate::Semaphore::new(crate::MAX_WEBSOCKET_CONNECTIONS)),
        ftp_uploads: crate::ftp::FtpUploadQueue::default(),
        relay_cleanup: crate::relay::ConnectionCleanup::default(),
        external_visualizer_processes: Arc::new(crate::Semaphore::new(
            crate::MAX_EXTERNAL_VISUALIZER_PROCESSES,
        )),
        visualizer_children:
            crate::external_visualizer_processes::ExternalVisualizerProcesses::default(),
        songid_run_slots: Arc::new(crate::Semaphore::new(
            config.media_services.song_id_max_concurrent_runs,
        )),
        songid_jobs: None,
        collections: RwLock::new(crate::CollectionStore::new()),
        collection_grant_persistence_lock: tokio::sync::Mutex::new(()),
        share_group_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist_search_persistence_lock: tokio::sync::Mutex::new(()),
        search_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist: RwLock::new(crate::WishlistStore::new()),
        contact_persistence_lock: tokio::sync::Mutex::new(()),
        contacts: RwLock::new(crate::ContactStore::new()),
        sharegroups: RwLock::new(crate::ShareGroupStore::new()),
        user_note_persistence_lock: tokio::sync::Mutex::new(()),
        user_notes: RwLock::new(crate::UserNoteStore::new()),
        interest_persistence_lock: tokio::sync::Mutex::new(()),
        interests: RwLock::new(crate::InterestStore::new()),
        now_playing_persistence_lock: tokio::sync::Mutex::new(()),
        now_playing: RwLock::new(crate::NowPlayingStore::new()),
        webhook_persistence_lock: Arc::new(tokio::sync::Mutex::new(())),
        relay: RwLock::new(crate::RelayState::new()),
        runtime: RwLock::new(crate::RuntimeCompatState::new()),
        runtime_persistence_lock: tokio::sync::Mutex::new(()),
        options_overlay: RwLock::new(crate::ControllerOptionsOverlayState::default()),
        diagnostics_allow_memory_dump: RwLock::new(config.controller_diagnostics_allow_memory_dump),
        diagnostics_allow_remote_dump: RwLock::new(config.controller_diagnostics_allow_remote_dump),
        backfill: RwLock::new(crate::BackfillState::default()),
        backfill_connections: Arc::new(crate::Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security_ban_persistence_lock: tokio::sync::Mutex::new(()),
        security: RwLock::new(crate::SecurityState::new()),
        share_grants: RwLock::new(crate::ShareGrantStore::new()),
        share_access_tokens: RwLock::new(crate::ShareAccessTokenStore::default()),
        incoming_shares: RwLock::new(crate::IncomingShareStore::default()),
        library: RwLock::new(crate::LibraryStore::new()),
        library_persistence_lock: tokio::sync::Mutex::new(()),
        virtual_soulfind_v2: Arc::new(RwLock::new(crate::virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(crate::SourceDiscoveryState::default()),
        destinations: RwLock::new(crate::DestinationStore::new()),
        db: None,
        config,
        session_commands: sender.clone(),
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands: None,
        managed_background_tasks: crate::ManagedTaskRegistry::default(),
        rate_limiter,
        soulseek_safety: crate::rate_limit::SoulseekSafetyLimiter::new(
            crate::rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(crate::OAuthStateStore::default()),
        oauth_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection: RwLock::new(crate::SpotifyConnectionStore::default()),
        spotify_connection_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection_generation: std::sync::atomic::AtomicU64::new(0),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(crate::PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(crate::multisource::SwarmStore::default())),
        controller_features: crate::ControllerFeatureStore::new(
            crate::ControllerFeatureState::in_memory(),
        ),
        peer_endpoints: RwLock::new(BTreeMap::new()),
        preview_streams: Arc::new(tokio::sync::Semaphore::new(crate::MAX_PREVIEW_STREAMS)),
        listening_party_stream_limits: RwLock::new(crate::ListeningPartyStreamLimits::default()),
        revoked_jwts: RwLock::new(crate::RevokedJwtStore::default()),
        login_attempts: RwLock::new(crate::LoginAttemptStore::default()),
        pod_signature_stats: crate::PodSignatureStats::default(),
        pod_verification_stats: crate::PodVerificationStats::default(),
        pod_dht_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_failed_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_publish_time_ms: std::sync::atomic::AtomicU64::new(0),
        podcore_runtime_stats: crate::PodCoreRuntimeStats::default(),
    });
    let missing = crate::route_http_request("GET", "/api/v0/config", None, "", &state)
        .await
        .unwrap();
    assert_eq!(missing.status, "401 Unauthorized");

    let wrong =
        crate::route_http_request("GET", "/api/v0/config", Some("Bearer wrong"), "", &state)
            .await
            .unwrap();
    assert_eq!(wrong.status, "401 Unauthorized");

    let allowed = crate::route_http_request(
        "GET",
        "/api/v0/config",
        Some("Bearer route-token"),
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(allowed.status, "200 OK");

    let x_api_key_allowed = crate::route_http_request(
        "GET",
        "/api/v0/config",
        Some("ApiKey route-token"),
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(x_api_key_allowed.status, "200 OK");

    let cross_site = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        Some("Bearer route-token"),
        "",
        &state,
        crate::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: Some("https://evil.example".to_string()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(cross_site.status, "202 Accepted");

    let same_origin = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        Some("Bearer route-token"),
        "",
        &state,
        crate::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: Some("http://127.0.0.1:5030".to_string()),
            referer: None,
            cookie: None,
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(same_origin.status, "202 Accepted");

    let cookie_rejected_by_default = crate::route_http_request_with_headers(
        "GET",
        "/api/v0/config",
        None,
        "",
        &state,
        crate::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: None,
            referer: None,
            cookie: Some("other=value; slskr.session=route-token".to_string()),
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(cookie_rejected_by_default.status, "401 Unauthorized");

    let cookie_enabled_env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            &std::env::temp_dir().display().to_string(),
        )
        .with("SLSKR_API_TOKEN", "route-token")
        .with("SLSKR_API_COOKIE_AUTH_ENABLED", "true");
    let cookie_enabled_config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &cookie_enabled_env)
            .expect("cookie auth config");
    let cookie_share_index = crate::build_share_index(&cookie_enabled_config);
    let cookie_share_lifecycle = crate::ShareLifecycleState::from_snapshot(&cookie_share_index);
    let cookie_enabled_state = crate::AppState {
        controller_version: std::sync::RwLock::new(crate::ControllerVersionState::initial()),
        controller_cli_environment: BTreeMap::new(),
        log_level: RwLock::new(crate::logging::LogLevel::Info),
        runtime_credentials: RwLock::new(None),
        configured_credentials: RwLock::new(cookie_enabled_config.credentials()),
        controller_web_auth_username: std::sync::RwLock::new(
            cookie_enabled_config.controller_web_auth_username.clone(),
        ),
        controller_web_auth_password: std::sync::RwLock::new(
            cookie_enabled_config.controller_web_auth_password.clone(),
        ),
        controller_web_jwt_key_current: std::sync::RwLock::new(
            cookie_enabled_config.controller_web_jwt_key.clone(),
        ),
        session: RwLock::new(crate::SessionSnapshot::disconnected(&cookie_enabled_config)),
        server_address: std::sync::RwLock::new(cookie_enabled_config.server_address.clone()),
        connected_server_address: std::sync::RwLock::new(None),
        listeners: RwLock::new(crate::ListenerSnapshot::new(&cookie_enabled_config)),
        distributed_network: RwLock::new(crate::DistributedRuntime::new(
            cookie_enabled_config.username.as_deref(),
        )),
        distributed_persistence_snapshots: tokio::sync::watch::channel(
            crate::DistributedRuntime::new(cookie_enabled_config.username.as_deref())
                .persistence_snapshot(),
        )
        .0,
        distributed_persistence_status: tokio::sync::watch::channel(
            crate::DistributedPersistenceStatus {
                revision: 0,
                result: Ok(()),
            },
        )
        .1,
        soulseek_distributed_settings: RwLock::new(cookie_enabled_config.soulseek_distributed),
        shares: RwLock::new(cookie_share_index),
        share_settings: RwLock::new(cookie_enabled_config.share_settings.clone()),
        share_index_persistence_lock: tokio::sync::Mutex::new(()),
        share_settings_generation: std::sync::atomic::AtomicU64::new(0),
        core_workflow_settings: RwLock::new(cookie_enabled_config.core_workflow.clone()),
        advanced_networking: RwLock::new(cookie_enabled_config.advanced_networking.clone()),
        media_services: RwLock::new(cookie_enabled_config.media_services.clone()),
        share_lifecycle: RwLock::new(cookie_share_lifecycle),
        downloads_dir: std::sync::RwLock::new(cookie_enabled_config.downloads_dir.clone()),
        incomplete_dir: std::sync::RwLock::new(cookie_enabled_config.incomplete_dir.clone()),
        download_completed_path_template: std::sync::RwLock::new(
            cookie_enabled_config
                .download_completed_path_template
                .clone(),
        ),
        remote_file_management: std::sync::RwLock::new(
            cookie_enabled_config.remote_file_management,
        ),
        remote_configuration: std::sync::RwLock::new(cookie_enabled_config.remote_configuration),
        controller_no_config_watch: std::sync::RwLock::new(
            cookie_enabled_config.controller_no_config_watch,
        ),
        controller_case_sensitive_regex: std::sync::RwLock::new(
            cookie_enabled_config.controller_case_sensitive_regex,
        ),
        user_info_description: std::sync::RwLock::new(
            cookie_enabled_config.user_info_description.clone(),
        ),
        user_info_picture: std::sync::RwLock::new(cookie_enabled_config.user_info_picture.clone()),
        controller_options_validation_error: std::sync::RwLock::new(None),
        regular_listener_commands: None,
        obfuscated_listener_commands: None,
        advertised_port: std::sync::RwLock::new(cookie_enabled_config.advertised_port),
        obfuscated_advertised_port: std::sync::RwLock::new(
            cookie_enabled_config.obfuscated_advertised_port,
        ),
        searches: RwLock::new(crate::SearchStore::new()),
        users: RwLock::new(crate::UserStore::new()),
        user_persistence_lock: tokio::sync::Mutex::new(()),
        event_persistence_lock: tokio::sync::Mutex::new(()),
        mesh: RwLock::new(crate::MeshState::new()),
        capability_signing_key: crate::controller_capabilities::new_capability_signing_key()
            .expect("capability signing key"),
        content_discovery: RwLock::new(crate::content_discovery::ContentDiscoveryStore::in_memory()),
        realm_subject_indexes: RwLock::new(crate::realm_subject_index::Store::in_memory()),
        browse: RwLock::new(crate::BrowseStore::new()),
        browse_persistence_lock: tokio::sync::Mutex::new(()),
        remote_path_encodings: RwLock::new(crate::RemotePathEncodingRegistry::default()),
        messages: RwLock::new(crate::MessageStore::new()),
        message_persistence_lock: tokio::sync::Mutex::new(()),
        managed_blacklist: RwLock::new(crate::ManagedBlacklistRuntime::new(
            cookie_enabled_config.managed_blacklist.clone(),
            cookie_enabled_config.controller_profile,
            cookie_enabled_config.controller_case_sensitive_regex,
        )),
        search_request_filters: RwLock::new(
            crate::compile_controller_regexes(
                &cookie_enabled_config.controller_search_request_filters,
                cookie_enabled_config.controller_case_sensitive_regex,
                cookie_enabled_config.controller_profile,
            )
            .unwrap(),
        ),
        integration_settings: RwLock::new(cookie_enabled_config.integrations.clone()),
        source_feed_import_history: RwLock::new(crate::SourceFeedImportHistoryStore::default()),
        source_feed_import_history_persistence_lock: tokio::sync::Mutex::new(()),
        lidarr_sync_state: RwLock::new(crate::LidarrSyncRuntimeState::new(
            &cookie_enabled_config.integrations.lidarr,
        )),
        lidarr_recent_imports: RwLock::new(BTreeMap::new()),
        lidarr_import_gate: tokio::sync::Semaphore::new(1),
        private_message_auto_response_settings: RwLock::new(
            cookie_enabled_config.private_message_auto_response.clone(),
        ),
        transfer_auto_retry_settings: RwLock::new(
            cookie_enabled_config.transfer_auto_retry.clone(),
        ),
        transfer_upload_settings: RwLock::new(cookie_enabled_config.transfer_upload.clone()),
        transfer_download_settings: RwLock::new(cookie_enabled_config.transfer_download.clone()),
        transfer_groups_settings: RwLock::new(cookie_enabled_config.transfer_groups.clone()),
        failed_upload_peer_cooldowns: RwLock::new(crate::UploadPeerCooldowns::default()),
        private_message_auto_responses: RwLock::new(
            crate::PrivateMessageAutoResponseTracker::default(),
        ),
        rooms: RwLock::new(crate::RoomStore::new()),
        room_persistence_lock: tokio::sync::Mutex::new(()),
        pod_join_replays: RwLock::new(crate::PodJoinReplayStore::default()),
        pod_membership_workflow: RwLock::new(crate::PodMembershipWorkflowStore::default()),
        pod_channels: RwLock::new(crate::pod_channels::PodChannelStore::empty(
            &cookie_enabled_config.state_dir,
        )),
        pods: RwLock::new(crate::pods::PodStore::empty(
            &cookie_enabled_config.state_dir,
        )),
        port_forwarding: crate::port_forwarding::Manager::new(),
        private_gateway: None,
        dht: None,
        transfers: RwLock::new(crate::TransferQueue::new(&cookie_enabled_config)),
        events: RwLock::new(crate::EventStore::new(crate::EVENT_HISTORY_LIMIT)),
        event_tx: event_tx.clone(),
        webhooks: Arc::new(RwLock::new(crate::webhooks::WebhookManager::new())),
        webhook_deliveries: Arc::new(crate::Semaphore::new(crate::MAX_WEBHOOK_DELIVERY_TASKS)),
        share_scans: Arc::new(crate::Semaphore::new(crate::MAX_SHARE_SCAN_TASKS)),
        share_scan_cancellation: Arc::new(std::sync::Mutex::new(None)),
        incoming_connections: Arc::new(crate::Semaphore::new(crate::MAX_INCOMING_CONNECTION_TASKS)),
        incoming_connection_ips: std::sync::Mutex::new(BTreeMap::new()),
        incoming_searches: Arc::new(crate::Semaphore::new(
            cookie_enabled_config
                .core_workflow
                .incoming_search
                .concurrency,
        )),
        incoming_search_queue_depth: crate::AtomicUsize::new(0),
        download_requests: Arc::new(crate::Semaphore::new(2)),
        download_batch_requests: Arc::new(crate::Semaphore::new(1)),
        websocket_connections: Arc::new(crate::Semaphore::new(crate::MAX_WEBSOCKET_CONNECTIONS)),
        ftp_uploads: crate::ftp::FtpUploadQueue::default(),
        relay_cleanup: crate::relay::ConnectionCleanup::default(),
        external_visualizer_processes: Arc::new(crate::Semaphore::new(
            crate::MAX_EXTERNAL_VISUALIZER_PROCESSES,
        )),
        visualizer_children:
            crate::external_visualizer_processes::ExternalVisualizerProcesses::default(),
        songid_run_slots: Arc::new(crate::Semaphore::new(
            cookie_enabled_config
                .media_services
                .song_id_max_concurrent_runs,
        )),
        songid_jobs: None,
        collections: RwLock::new(crate::CollectionStore::new()),
        collection_grant_persistence_lock: tokio::sync::Mutex::new(()),
        share_group_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist_search_persistence_lock: tokio::sync::Mutex::new(()),
        search_persistence_lock: tokio::sync::Mutex::new(()),
        wishlist: RwLock::new(crate::WishlistStore::new()),
        contact_persistence_lock: tokio::sync::Mutex::new(()),
        contacts: RwLock::new(crate::ContactStore::new()),
        sharegroups: RwLock::new(crate::ShareGroupStore::new()),
        user_note_persistence_lock: tokio::sync::Mutex::new(()),
        user_notes: RwLock::new(crate::UserNoteStore::new()),
        interest_persistence_lock: tokio::sync::Mutex::new(()),
        interests: RwLock::new(crate::InterestStore::new()),
        now_playing_persistence_lock: tokio::sync::Mutex::new(()),
        now_playing: RwLock::new(crate::NowPlayingStore::new()),
        webhook_persistence_lock: Arc::new(tokio::sync::Mutex::new(())),
        relay: RwLock::new(crate::RelayState::new()),
        runtime: RwLock::new(crate::RuntimeCompatState::new()),
        runtime_persistence_lock: tokio::sync::Mutex::new(()),
        options_overlay: RwLock::new(crate::ControllerOptionsOverlayState::default()),
        diagnostics_allow_memory_dump: RwLock::new(
            cookie_enabled_config.controller_diagnostics_allow_memory_dump,
        ),
        diagnostics_allow_remote_dump: RwLock::new(
            cookie_enabled_config.controller_diagnostics_allow_remote_dump,
        ),
        backfill: RwLock::new(crate::BackfillState::default()),
        backfill_connections: Arc::new(crate::Semaphore::new(2)),
        pending_backfill_transfers: RwLock::new(BTreeMap::new()),
        security_ban_persistence_lock: tokio::sync::Mutex::new(()),
        security: RwLock::new(crate::SecurityState::new()),
        share_grants: RwLock::new(crate::ShareGrantStore::new()),
        share_access_tokens: RwLock::new(crate::ShareAccessTokenStore::default()),
        incoming_shares: RwLock::new(crate::IncomingShareStore::default()),
        library: RwLock::new(crate::LibraryStore::new()),
        library_persistence_lock: tokio::sync::Mutex::new(()),
        virtual_soulfind_v2: Arc::new(RwLock::new(crate::virtual_soulfind_v2::State::default())),
        source_discovery: RwLock::new(crate::SourceDiscoveryState::default()),
        destinations: RwLock::new(crate::DestinationStore::new()),
        db: None,
        config: cookie_enabled_config,
        session_commands: sender.clone(),
        pending_user_interests: RwLock::new(BTreeMap::new()),
        lifecycle_commands: None,
        managed_background_tasks: crate::ManagedTaskRegistry::default(),
        rate_limiter: crate::rate_limit::RateLimiter::new(crate::rate_limit::RateLimitConfig {
            max_requests_anonymous: 1000,
            max_requests_authenticated: 5000,
            window_seconds: 60,
            enabled: true,
        }),
        soulseek_safety: crate::rate_limit::SoulseekSafetyLimiter::new(
            crate::rate_limit::SoulseekSafetyConfig::default(),
        ),
        oauth_states: RwLock::new(crate::OAuthStateStore::default()),
        oauth_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection: RwLock::new(crate::SpotifyConnectionStore::default()),
        spotify_connection_persistence_lock: tokio::sync::Mutex::new(()),
        spotify_connection_generation: std::sync::atomic::AtomicU64::new(0),
        spotify_token_gate: tokio::sync::Semaphore::new(1),
        stream_tickets: RwLock::new(crate::PreviewStreamTicketStore::default()),
        multisource: Arc::new(RwLock::new(crate::multisource::SwarmStore::default())),
        controller_features: crate::ControllerFeatureStore::new(
            crate::ControllerFeatureState::in_memory(),
        ),
        peer_endpoints: RwLock::new(BTreeMap::new()),
        preview_streams: Arc::new(tokio::sync::Semaphore::new(crate::MAX_PREVIEW_STREAMS)),
        listening_party_stream_limits: RwLock::new(crate::ListeningPartyStreamLimits::default()),
        revoked_jwts: RwLock::new(crate::RevokedJwtStore::default()),
        login_attempts: RwLock::new(crate::LoginAttemptStore::default()),
        pod_signature_stats: crate::PodSignatureStats::default(),
        pod_verification_stats: crate::PodVerificationStats::default(),
        pod_dht_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_failed_publish_count: std::sync::atomic::AtomicU64::new(0),
        pod_dht_publish_time_ms: std::sync::atomic::AtomicU64::new(0),
        podcore_runtime_stats: crate::PodCoreRuntimeStats::default(),
    };
    let cookie_allowed = crate::route_http_request_with_headers(
        "GET",
        "/api/v0/config",
        None,
        "",
        &cookie_enabled_state,
        crate::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_string()),
            origin: None,
            referer: None,
            cookie: Some("other=value; slskr.session=route-token".to_string()),
            content_type: None,
            x_share_token: None,
            x_gateway_api_key: None,
            x_gateway_csrf: None,
            x_relay_agent: None,
            x_relay_credential: None,
            remote_addr: None,
            date: None,
            digest: None,
            signature: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(cookie_allowed.status, "200 OK");

    let health = crate::route_http_request("GET", "/api/v0/health", None, "", &state)
        .await
        .unwrap();
    assert_eq!(health.status, "200 OK");

    let session_enabled =
        crate::route_http_request("GET", "/api/v0/session/enabled", None, "", &state)
            .await
            .unwrap();
    assert_eq!(session_enabled.status, "200 OK");
    assert_eq!(session_enabled.body, "true");

    let capabilities = crate::route_http_request("GET", "/api/v0/capabilities", None, "", &state)
        .await
        .unwrap();
    assert_eq!(capabilities.status, "401 Unauthorized");
    let public_capabilities =
        crate::route_http_request("GET", "/api/capabilities", None, "", &state)
            .await
            .unwrap();
    assert_eq!(public_capabilities.status, "200 OK");
    assert!(public_capabilities.body.contains("\"room-list-sync\""));
    assert!(public_capabilities
        .body
        .contains("\"browser-session-auth\""));
}

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-security-control-tests"
))]
pub(super) async fn security_controls_differential_csrf_filter_impl() {
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
    let headers = |origin: Option<&str>, cookie: Option<&str>| crate::RequestSecurityHeaders {
        host: Some("127.0.0.1:5030".to_owned()),
        origin: origin.map(str::to_owned),
        cookie: cookie.map(str::to_owned),
        ..crate::RequestSecurityHeaders::default()
    };

    record!(
        "activation-default-and-profile",
        default_state.config.auth_required
            && !default_state.config.api_cookie_auth_enabled
            && cookie_state.config.auth_required
            && cookie_state.config.api_cookie_auth_enabled
    );

    let bearer_cross_site = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        Some("Bearer csrf-filter-secret"),
        "",
        &cookie_state,
        headers(Some("https://evil.example"), None),
    )
    .await
    .expect("bearer CSRF exemption");
    let cookie_same_origin = crate::route_http_request_with_headers(
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

    let cookie_cross_site = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        None,
        "",
        &cookie_state,
        headers(Some("https://evil.example"), Some(cookie)),
    )
    .await
    .expect("cross-site cookie request");
    let cookie_without_origin = crate::route_http_request_with_headers(
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
    let restarted_same_origin = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/session/ping",
        None,
        "",
        &restarted,
        headers(Some("http://127.0.0.1:5030"), Some(cookie)),
    )
    .await
    .expect("restarted same-origin cookie request");
    let restarted_cross_site = crate::route_http_request_with_headers(
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
