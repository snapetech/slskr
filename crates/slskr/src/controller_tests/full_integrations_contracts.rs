//! Controller full integrations contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn integration_json_reader_rejects_declared_oversized_response() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture server");
    let address = listener.local_addr().expect("fixture address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept fixture request");
        let mut request = [0_u8; 1024];
        let request_bytes = stream
            .read(&mut request)
            .await
            .expect("read fixture request");
        assert!(request_bytes > 0, "fixture request was empty");
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    crate::MAX_INTEGRATION_RESPONSE_BYTES + 1
                )
                .as_bytes(),
            )
            .await
            .expect("write fixture response");
    });

    let response = reqwest::get(format!("http://{address}/oversized"))
        .await
        .expect("receive fixture headers");
    let error = crate::read_bounded_integration_json(response, "fixture")
        .await
        .expect_err("oversized response must be rejected");
    assert!(error.contains("response exceeds"), "{error}");
    server.await.expect("fixture server task");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn integration_json_reader_rejects_chunked_oversized_response() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture server");
    let address = listener.local_addr().expect("fixture address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept fixture request");
        let mut request = [0_u8; 1024];
        let request_bytes = stream
            .read(&mut request)
            .await
            .expect("read fixture request");
        assert!(request_bytes > 0, "fixture request was empty");
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write fixture headers");
        let chunk = vec![b' '; 64 * 1024];
        for _ in 0..=(crate::MAX_INTEGRATION_RESPONSE_BYTES / chunk.len()) {
            stream
                .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await
                .expect("write chunk length");
            stream
                .write_all(&chunk)
                .await
                .expect("write oversized chunk");
            stream.write_all(b"\r\n").await.expect("finish chunk");
        }
        stream
            .write_all(b"0\r\n\r\n")
            .await
            .expect("finish response");
    });

    let response = reqwest::get(format!("http://{address}/oversized"))
        .await
        .expect("receive fixture headers");
    let error = crate::read_bounded_integration_json(response, "fixture")
        .await
        .expect_err("chunked oversized response must be rejected");
    assert!(error.contains("response exceeds"), "{error}");
    server.await.expect("fixture server task");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn source_provider_reader_rejects_declared_and_chunked_oversized_responses() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    for chunked in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind source provider fixture");
        let address = listener
            .local_addr()
            .expect("source provider fixture address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept fixture request");
            let mut request = [0_u8; 1024];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            if chunked {
                stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
                let chunk = vec![b'x'; 64 * 1024];
                for _ in 0..=(crate::MAX_SOURCE_PROVIDER_RESPONSE_BYTES / chunk.len()) {
                    stream
                        .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                        .await
                        .unwrap();
                    stream.write_all(&chunk).await.unwrap();
                    stream.write_all(b"\r\n").await.unwrap();
                }
                let _ = stream.write_all(b"0\r\n\r\n").await;
            } else {
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            crate::MAX_SOURCE_PROVIDER_RESPONSE_BYTES + 1
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        });
        let response = reqwest::get(format!("http://{address}/oversized"))
            .await
            .expect("receive source provider fixture headers");
        let error = crate::read_bounded_source_provider_bytes(response, "provider")
            .await
            .expect_err("oversized source provider response must fail");
        assert!(error.contains("response exceeds"), "{error}");
        server.await.expect("source provider fixture task");
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn integration_ssrf_filter_blocks_special_use_ip_ranges() {
    let error = crate::validate_integration_base_url("https://operator:secret@example.com/lidarr")
        .err()
        .expect("credential-bearing integration URL should be rejected");
    assert_eq!(
        error,
        "integration URL must not contain embedded credentials"
    );
    assert!(!error.contains("operator"));
    assert!(!error.contains("secret"));

    for url in [
        "https://example.com/lidarr?api_key=secret",
        "https://example.com/lidarr#ignored-api-path",
    ] {
        assert_eq!(
            crate::validate_integration_base_url(url)
                .err()
                .expect("non-base integration URL should be rejected"),
            "integration URL must not contain a query or fragment"
        );
    }

    for address in [
        "100.64.0.1",
        "192.0.0.8",
        "192.31.196.1",
        "192.52.193.1",
        "192.88.99.1",
        "198.18.0.1",
    ] {
        let ip = address.parse::<std::net::IpAddr>().expect("fixture IP");
        assert!(crate::is_blocked_integration_ip(ip), "accepted {address}");
    }
    for address in [
        "ff02::1",
        "2001:db8::1",
        "2002:c0a8:101::1",
        "2001:0000:4136:e378::1",
        "fec0::1",
        "64:ff9b::7f00:1",
        "64:ff9b:1::1",
        "100::1",
        "2001:2::1",
        "2001:10::1",
        "2001:20::1",
        "3fff::1",
    ] {
        let ip = address.parse::<std::net::IpAddr>().expect("fixture IP");
        assert!(crate::is_blocked_integration_ip(ip), "accepted {address}");
    }
    assert!(!crate::is_blocked_integration_ip(
        "2606:4700:4700::1111"
            .parse::<std::net::IpAddr>()
            .expect("global fixture IP")
    ));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_oauth_callback_requires_server_issued_state() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_HTTP_BIND", "127.0.0.1:7788"),
    );

    let authorize = crate::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    assert_eq!(authorize.status, "200 OK");
    assert!(authorize
        .body
        .contains("127.0.0.1:7788/api/v0/integrations/spotify/callback"));

    let authorization: serde_json::Value = serde_json::from_str(&authorize.body).unwrap();
    let authorization_url = authorization["authorizationUrl"].as_str().unwrap();
    assert!(authorization_url.contains("code_challenge_method=S256"));
    assert!(authorization_url.contains(
        "scope=user-library-read%20user-follow-read%20playlist-read-private%20playlist-read-collaborative"
    ));

    let (issued_state, verifier) = {
        let states = state.oauth_states.read().await;
        let (issued_state, record) = states.records.iter().next().expect("issued state");
        (
            issued_state.clone(),
            record.code_verifier.clone().expect("PKCE verifier"),
        )
    };
    use base64::Engine as _;
    use sha2::Digest as _;
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(crate::Sha256::digest(verifier.as_bytes()));
    assert!(authorization_url.contains(&format!("code_challenge={challenge}")));
    assert!(!authorize.body.contains(&verifier));

    let invalid = crate::route_http_request(
        "GET",
        "/api/integrations/spotify/callback?code=abc&state=bogus",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid callback response");
    assert_eq!(invalid.status, "400 Bad Request");
    assert!(state
        .oauth_states
        .read()
        .await
        .records
        .contains_key(&issued_state));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_oauth_error_callback_returns_html_without_consuming_state() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
    );
    crate::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    let issued_state = state
        .oauth_states
        .read()
        .await
        .records
        .keys()
        .next()
        .cloned()
        .expect("issued state");

    let missing = crate::route_http_request(
        "GET",
        "/api/integrations/spotify/callback?error=access_denied",
        None,
        "",
        &state,
    )
    .await
    .expect("missing state response");
    assert_eq!(missing.status, "200 OK");
    assert_eq!(missing.content_type, "text/html; charset=utf-8");
    assert!(missing.body.contains("Spotify authorization failed."));

    let invalid = crate::route_http_request(
        "GET",
        "/api/integrations/spotify/callback?error=access_denied&state=bogus",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid state response");
    assert_eq!(invalid.status, "200 OK");

    let error_path =
        format!("/api/integrations/spotify/callback?error=access_denied&state={issued_state}");
    let denied = crate::route_http_request("GET", &error_path, None, "", &state)
        .await
        .expect("valid error callback");
    assert_eq!(denied.status, "200 OK");
    assert!(denied.body.contains("Spotify authorization failed."));

    let replay = crate::route_http_request("GET", &error_path, None, "", &state)
        .await
        .expect("replayed error callback");
    assert_eq!(replay.status, "200 OK");
    assert!(state
        .oauth_states
        .read()
        .await
        .records
        .contains_key(&issued_state));
}
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_authorization_exchanges_profiles_persists_and_disconnects() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for (expected_path, response) in [
            (
                "/token",
                serde_json::json!({
                    "access_token": "access-secret",
                    "refresh_token": "refresh-secret",
                    "expires_in": 3600,
                    "scope": "user-library-read playlist-read-private"
                })
                .to_string(),
            ),
            (
                "/me",
                serde_json::json!({"id": "spotify-user", "display_name": "Fixture User"})
                    .to_string(),
            ),
        ] {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            let header_end = loop {
                let count = stream.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(position) = request.windows(4).position(|row| row == b"\r\n\r\n") {
                    break position + 4;
                }
            };
            let headers = String::from_utf8_lossy(&request[..header_end]).to_string();
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.strip_prefix("content-length: ")
                        .or_else(|| line.strip_prefix("Content-Length: "))
                })
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or_default();
            while request.len() < header_end + content_length {
                let count = stream.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
            }
            let request = String::from_utf8_lossy(&request).to_string();
            assert!(request.starts_with(&format!(
                "{} {expected_path} HTTP/1.1",
                if expected_path == "/token" {
                    "POST"
                } else {
                    "GET"
                }
            )));
            if expected_path == "/token" {
                assert!(request.contains("client_id=client-id"));
                assert!(request.contains("grant_type=authorization_code"));
                assert!(request.contains("code=authorization-code"));
                assert!(request.contains("code_verifier=pkce-verifier"));
            } else {
                assert!(request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer access-secret"));
            }
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.len(),
                response
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
        }
    });

    let root = std::env::temp_dir().join(format!(
        "slskr-spotify-connection-test-{}-{}",
        std::process::id(),
        crate::unix_timestamp_millis()
    ));
    crate::ensure_private_state_dir(&root).unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    let spotify = state.integration_settings.read().await.spotify.clone();
    let pending = crate::OAuthStateRecord {
        provider: "spotify".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        code_verifier: Some("pkce-verifier".to_owned()),
        created_at: crate::unix_timestamp(),
        expires_at: crate::unix_timestamp().saturating_add(600),
    };
    let status = crate::complete_spotify_authorization(
        &state,
        &spotify,
        &pending,
        "authorization-code",
        &format!("http://{address}/token"),
        &format!("http://{address}/me"),
    )
    .await
    .unwrap();
    server.await.unwrap();

    assert_eq!(status["configured"], true);
    assert_eq!(status["connected"], true);
    assert_eq!(status["displayName"], "Fixture User");
    assert_eq!(status["spotifyUserId"], "spotify-user");
    let path = crate::spotify_connection_path(&root);
    let encrypted = fs::read_to_string(&path).unwrap();
    assert!(!encrypted.contains("access-secret"));
    assert!(!encrypted.contains("refresh-secret"));
    let loaded = crate::load_spotify_connection_store(&root, &state.capability_signing_key);
    assert_eq!(loaded, *state.spotify_connection.read().await);

    let response = crate::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/status",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let response: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(response["connected"], true);
    assert!(response["expiresAt"].is_string());

    let disconnect =
        crate::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
            .await
            .unwrap();
    assert_eq!(disconnect.status, "204 No Content");
    assert!(!path.exists());
    assert_eq!(
        state.spotify_connection.read().await.status_json(true)["connected"],
        false
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn spotify_source_targets_match_frozen_native_forms() {
    for (input, kind, id, user_token, scope) in [
        (
            "spotify:playlist:playlist-1",
            "playlist",
            "playlist-1",
            false,
            "",
        ),
        (
            "https://open.spotify.com/album/album-1?si=ignored",
            "album",
            "album-1",
            false,
            "",
        ),
        (
            "https://open.spotify.com/collection/tracks",
            "saved-tracks",
            "",
            true,
            "user-library-read",
        ),
        (
            "spotify:followed-artists",
            "followed-artists",
            "",
            true,
            "user-follow-read",
        ),
        (
            "playlists",
            "playlists",
            "",
            true,
            "playlist-read-private playlist-read-collaborative",
        ),
    ] {
        let target = crate::parse_spotify_source_target(input);
        assert_eq!(target.kind, kind, "{input}");
        assert_eq!(target.id, id, "{input}");
        assert_eq!(target.requires_user_token, user_token, "{input}");
        assert_eq!(target.scope_hint, scope, "{input}");
    }
    assert!(crate::looks_like_spotify_source(
        "https://open.spotify.com/track/track-1",
        "auto"
    ));
    assert!(!crate::looks_like_spotify_source(
        "https://open.spotify.com.evil.example/track/track-1",
        "auto"
    ));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_source_preview_refreshes_and_pages_saved_tracks() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Spotify fixture");
    let address = listener.local_addr().expect("Spotify fixture address");
    let server = tokio::spawn(async move {
        let refresh = serve_json_fixture(
            &listener,
            serde_json::json!({
                "access_token": "refreshed-access",
                "expires_in": 3600,
                "scope": "user-library-read"
            }),
        )
        .await;
        assert!(refresh.starts_with("POST /token HTTP/1.1"));
        assert!(refresh.contains("client_id=client-id"));
        assert!(refresh.contains("grant_type=refresh_token"));
        assert!(refresh.contains("refresh_token=refresh-secret"));

        let first = serve_json_fixture(
            &listener,
            serde_json::json!({
                "total": 2,
                "next": "next-page",
                "items": [{"track": {
                    "id": "track-1",
                    "name": "First Song",
                    "artists": [{"name": "First Artist"}],
                    "album": {"name": "First Album"},
                    "external_urls": {"spotify": "https://open.spotify.com/track/track-1"}
                }}]
            }),
        )
        .await;
        assert!(first.starts_with("GET /v1/me/tracks?limit=2&offset=0 HTTP/1.1"));
        assert!(first
            .to_ascii_lowercase()
            .contains("authorization: bearer refreshed-access"));

        let second = serve_json_fixture(
            &listener,
            serde_json::json!({
                "total": 2,
                "next": null,
                "items": [{"track": {
                    "id": "track-2",
                    "name": "Second Song",
                    "artists": [{"name": "Second Artist"}],
                    "album": {"name": "Second Album"},
                    "external_urls": {"spotify": "https://open.spotify.com/track/track-2"}
                }}]
            }),
        )
        .await;
        assert!(second.starts_with("GET /v1/me/tracks?limit=1&offset=1 HTTP/1.1"));
    });

    let root = std::env::temp_dir().join(format!(
        "slskr-spotify-source-refresh-test-{}-{}",
        std::process::id(),
        crate::unix_timestamp_millis()
    ));
    crate::ensure_private_state_dir(&root).unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    *state.spotify_connection.write().await = crate::SpotifyConnectionStore {
        refresh_token: "refresh-secret".to_owned(),
        expires_at: 0,
        display_name: "Fixture User".to_owned(),
        spotify_user_id: "fixture-user".to_owned(),
        ..Default::default()
    };
    let result = crate::preview_spotify_source_feed(
        &state,
        "spotify:saved-tracks",
        "",
        2,
        &format!("http://{address}/token"),
        &format!("http://{address}/v1"),
    )
    .await
    .expect("Spotify preview");
    server.await.expect("Spotify fixture task");

    assert_eq!(result["provider"], "spotify");
    assert_eq!(result["sourceKind"], "saved-tracks");
    assert_eq!(result["suggestionCount"], 2);
    assert_eq!(result["networkRequestCount"], 2);
    assert_eq!(result["requiresAccessToken"], false);
    assert_eq!(
        result["suggestions"][0]["searchText"],
        "First Artist First Song First Album"
    );
    assert_eq!(result["suggestions"][0]["source"], "spotify:liked");
    assert_eq!(result["suggestions"][0]["sourceId"], "track-1");
    assert_eq!(
        state.spotify_connection.read().await.access_token,
        "refreshed-access"
    );
    let protected = fs::read_to_string(crate::spotify_connection_path(&root)).unwrap();
    assert!(!protected.contains("refreshed-access"));
    assert!(!protected.contains("refresh-secret"));
    let _ = fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_source_preview_uses_app_token_and_market() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Spotify fixture");
    let address = listener.local_addr().expect("Spotify fixture address");
    let server = tokio::spawn(async move {
        let token =
            serve_json_fixture(&listener, serde_json::json!({"access_token": "app-access"})).await;
        assert!(token.starts_with("POST /token HTTP/1.1"));
        assert!(token.contains("grant_type=client_credentials"));
        let expected = base64::engine::general_purpose::STANDARD.encode("client-id:client-secret");
        assert!(token.to_ascii_lowercase().contains(&format!(
            "authorization: basic {}",
            expected.to_ascii_lowercase()
        )));

        let top = serve_json_fixture(
            &listener,
            serde_json::json!({"tracks": [{
                "id": "track-1",
                "name": "Market Song",
                "artists": [{"name": "Market Artist"}],
                "album": {"name": "Market Album"},
                "external_urls": {"spotify": "https://open.spotify.com/track/track-1"}
            }]}),
        )
        .await;
        assert!(top.starts_with("GET /v1/artists/artist-1/top-tracks?market=CA HTTP/1.1"));
        assert!(top
            .to_ascii_lowercase()
            .contains("authorization: bearer app-access"));
    });

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_SPOTIFY_CLIENT_SECRET", "client-secret")
            .with("SLSKR_SPOTIFY_MARKET", "CA")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    let result = crate::preview_spotify_source_feed(
        &state,
        "spotify:artist:artist-1",
        "",
        10,
        &format!("http://{address}/token"),
        &format!("http://{address}/v1"),
    )
    .await
    .expect("Spotify preview");
    server.await.expect("Spotify fixture task");
    assert_eq!(result["suggestionCount"], 1);
    assert_eq!(result["networkRequestCount"], 1);
    assert_eq!(result["suggestions"][0]["sourceId"], "artist-1");
    assert_eq!(
        result["suggestions"][0]["evidenceKey"],
        "spotify:spotify:artist-top-tracks:market artist market song market album"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_source_route_reports_required_user_scope_without_connection() {
    let root = std::env::temp_dir().join(format!(
        "slskr-spotify-source-route-test-{}-{}",
        std::process::id(),
        crate::unix_timestamp_millis()
    ));
    crate::ensure_private_state_dir(&root).unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
    );
    let response = crate::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        r#"{"sourceText":"spotify:saved-tracks","sourceKind":"spotify","fetchProviderUrls":true,"limit":10}"#,
        &state,
    )
    .await
    .expect("Spotify source route response");
    assert_eq!(response.status, "200 OK");
    let result: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(result["provider"], "spotify");
    assert_eq!(result["sourceKind"], "saved-tracks");
    assert_eq!(result["requiresAccessToken"], true);
    assert_eq!(result["requiredScopeHint"], "user-library-read");
    assert_eq!(result["networkRequestCount"], 0);
    assert_eq!(
        state.source_feed_import_history.read().await.history.len(),
        1
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_source_requests_enforce_configured_timeout() {
    use tokio::io::AsyncReadExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Spotify timeout fixture");
    let address = listener.local_addr().expect("Spotify fixture address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept Spotify request");
        let mut request = [0_u8; 1024];
        assert!(stream.read(&mut request).await.unwrap() > 0);
        tokio::time::sleep(Duration::from_secs(2)).await;
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SPOTIFY_TIMEOUT", "1")
            .with("SLSKR_SPOTIFY_MAX_ITEMS_PER_IMPORT", "10"),
    );
    let error = crate::preview_spotify_source_feed(
        &state,
        "spotify:track:track-1",
        "provided-access",
        10,
        &format!("http://{address}/unused"),
        &format!("http://{address}/v1"),
    )
    .await
    .expect_err("Spotify timeout must fail");
    assert!(error.contains("Spotify API request failed"), "{error}");
    server.abort();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn musicbrainz_release_lookup_resolves_a_release_with_an_artist_credit() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind MusicBrainz fixture");
    let address = listener.local_addr().expect("MusicBrainz fixture address");
    let server = tokio::spawn(async move {
        serve_json_fixture(
            &listener,
            serde_json::json!({
                "id": "release-1",
                "title": "Route Audit Release",
                "artist-credit": [
                    {"name": "Route Audit Artist", "artist": {"id": "artist-1"}},
                ],
            }),
        )
        .await
    });
    let target = crate::musicbrainz_release_target(&format!("http://{address}"), "release-1")
        .await
        .expect("MusicBrainz lookup");
    server.await.expect("MusicBrainz fixture task");
    assert_eq!(
        target,
        Some((
            "Route Audit Artist".to_owned(),
            "Route Audit Release".to_owned()
        ))
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn musicbrainz_release_lookup_returns_none_when_release_is_not_found() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind MusicBrainz fixture");
    let address = listener.local_addr().expect("MusicBrainz fixture address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept MusicBrainz request");
        let mut buffer = [0_u8; 1024];
        assert!(stream.read(&mut buffer).await.unwrap() > 0);
        let reply = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream
            .write_all(reply.as_bytes())
            .await
            .expect("write MusicBrainz 404");
    });
    let target = crate::musicbrainz_release_target(&format!("http://{address}"), "missing-release")
        .await
        .expect("MusicBrainz lookup");
    server.await.expect("MusicBrainz fixture task");
    assert_eq!(target, None);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn youtube_and_lastfm_source_providers_fetch_real_rows() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind source provider fixture");
    let address = listener.local_addr().expect("provider fixture address");
    let server = tokio::spawn(async move {
        let first = serve_json_fixture(
            &listener,
            serde_json::json!({
                "nextPageToken": "page-2",
                "items": [{"snippet": {
                    "title": "Artist One - Song One",
                    "resourceId": {"videoId": "video-1"}
                }}]
            }),
        )
        .await;
        assert!(first.starts_with(
            "GET /youtube?part=snippet&maxResults=2&playlistId=playlist-1&key=youtube-key HTTP/1.1"
        ));

        let second = serve_json_fixture(
            &listener,
            serde_json::json!({
                "items": [{"snippet": {
                    "title": "Artist Two - Song Two",
                    "resourceId": {"videoId": "video-2"}
                }}]
            }),
        )
        .await;
        assert!(second.starts_with("GET /youtube?part=snippet&maxResults=1&playlistId=playlist-1&key=youtube-key&pageToken=page-2 HTTP/1.1"));

        let lastfm = serve_json_fixture(
            &listener,
            serde_json::json!({"lovedtracks": {"track": [{
                "name": "Loved Song",
                "artist": {"name": "Loved Artist"},
                "album": {"#text": "Loved Album"},
                "mbid": "recording-1",
                "url": "https://last.fm/music/loved"
            }]}}),
        )
        .await;
        assert!(lastfm.starts_with("GET /lastfm?method=user.getlovedtracks&user=fixture-user&api_key=lastfm-key&format=json&limit=10 HTTP/1.1"));
    });

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_YOUTUBE", "true")
            .with("SLSKD_YOUTUBE_API_KEY", "youtube-key")
            .with("SLSKD_LASTFM", "true")
            .with("SLSKD_LASTFM_API_KEY", "lastfm-key")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    let youtube = crate::preview_configured_provider_source_feed(
        &state,
        "https://www.youtube.com/playlist?list=playlist-1",
        "auto",
        2,
        &format!("http://{address}/youtube"),
        &format!("http://{address}/lastfm"),
        &format!("http://{address}/apple"),
        &format!("http://{address}/listenbrainz"),
    )
    .await
    .unwrap()
    .expect("YouTube provider result");
    assert_eq!(youtube["provider"], "youtube");
    assert_eq!(youtube["suggestionCount"], 2);
    assert_eq!(youtube["networkRequestCount"], 2);
    assert_eq!(youtube["suggestions"][0]["artist"], "Artist One");
    assert_eq!(youtube["suggestions"][0]["title"], "Song One");
    assert_eq!(youtube["suggestions"][0]["sourceId"], "video-1");
    assert_eq!(
        youtube["suggestions"][0]["providerUrl"],
        "https://www.youtube.com/watch?v=video-1"
    );

    let lastfm = crate::preview_configured_provider_source_feed(
        &state,
        "https://www.last.fm/user/fixture-user/loved",
        "auto",
        10,
        &format!("http://{address}/youtube"),
        &format!("http://{address}/lastfm"),
        &format!("http://{address}/apple"),
        &format!("http://{address}/listenbrainz"),
    )
    .await
    .unwrap()
    .expect("Last.fm provider result");
    server.await.expect("provider fixture task");
    assert_eq!(lastfm["provider"], "lastfm");
    assert_eq!(lastfm["suggestionCount"], 1);
    assert_eq!(lastfm["networkRequestCount"], 1);
    assert_eq!(lastfm["suggestions"][0]["source"], "lastfm:loved");
    assert_eq!(lastfm["suggestions"][0]["sourceId"], "recording-1");
    assert_eq!(
        lastfm["suggestions"][0]["searchText"],
        "Loved Artist Loved Song Loved Album"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn apple_and_listenbrainz_source_providers_fetch_real_rows() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let apple = serve_json_fixture(
            &listener,
            serde_json::json!({"results": [{
                "trackId": 123,
                "trackName": "Apple Song",
                "artistName": "Apple Artist",
                "collectionName": "Apple Album",
                "trackViewUrl": "https://music.apple.com/track/123"
            }]}),
        )
        .await;
        assert!(apple.starts_with("GET /apple?id=123&entity=song&limit=10 HTTP/1.1"));

        let listenbrainz = serve_json_fixture(
            &listener,
            serde_json::json!({"payload": {"listens": [{"track_metadata": {
                "track_name": "Brainz Song",
                "artist_name": "Brainz Artist",
                "release_name": "Brainz Album",
                "additional_info": {"recording_msid": "recording-msid"}
            }}]}}),
        )
        .await;
        assert!(listenbrainz
            .starts_with("GET /listenbrainz/1/user/fixture-user/listens?count=10 HTTP/1.1"));
    });
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let apple = crate::preview_configured_provider_source_feed(
        &state,
        "https://music.apple.com/ca/album/example/1?i=123",
        "auto",
        10,
        &format!("http://{address}/youtube"),
        &format!("http://{address}/lastfm"),
        &format!("http://{address}/apple"),
        &format!("http://{address}/listenbrainz"),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(apple["provider"], "apple");
    assert_eq!(apple["suggestions"][0]["sourceId"], "123");
    assert_eq!(
        apple["suggestions"][0]["searchText"],
        "Apple Artist Apple Song Apple Album"
    );

    let listenbrainz = crate::preview_configured_provider_source_feed(
        &state,
        "https://listenbrainz.org/user/fixture-user/",
        "auto",
        10,
        &format!("http://{address}/youtube"),
        &format!("http://{address}/lastfm"),
        &format!("http://{address}/apple"),
        &format!("http://{address}/listenbrainz"),
    )
    .await
    .unwrap()
    .unwrap();
    server.await.unwrap();
    assert_eq!(listenbrainz["provider"], "listenbrainz");
    assert_eq!(listenbrainz["suggestions"][0]["sourceId"], "recording-msid");
    assert_eq!(
        listenbrainz["suggestions"][0]["source"],
        "listenbrainz:listens"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn youtube_and_lastfm_settings_require_keys_only_when_enabled() {
    for (enabled, key, expected_error) in [
        (
            "SLSKD_YOUTUBE",
            "SLSKD_YOUTUBE_API_KEY",
            "YouTube source feed imports are enabled",
        ),
        (
            "SLSKD_LASTFM",
            "SLSKD_LASTFM_API_KEY",
            "Last.fm source feed imports are enabled",
        ),
    ] {
        let error = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default().with(enabled, "true"),
        )
        .expect_err("enabled provider without API key must fail");
        assert!(error.contains(expected_error), "{error}");
        let configured = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with(enabled, "true")
                .with(key, "fixture-key"),
        )
        .expect("configured provider");
        let settings = if enabled.contains("YOUTUBE") {
            &configured.integrations.youtube
        } else {
            &configured.integrations.lastfm
        };
        assert!(settings.configured());
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn source_feed_local_formats_match_frozen_parsing_and_deduplication() {
    let csv = crate::preview_local_source_feed(
        "Track Name,Artist Name,Album Name,URL\n\"Song, One\",Artist One,Album One,https://example/one\nSong Two,Artist Two,Album Two,https://example/two",
        "auto",
        true,
        10,
    )
    .expect("CSV source preview");
    assert_eq!(csv["sourceKind"], "csv");
    assert_eq!(csv["suggestionCount"], 2);
    assert_eq!(csv["suggestions"][0]["title"], "Song, One");
    assert_eq!(csv["suggestions"][0]["artist"], "Artist One");
    assert_eq!(csv["suggestions"][0]["album"], "Album One");
    assert_eq!(csv["suggestions"][0]["sourceId"], "2");
    assert_eq!(csv["suggestions"][0]["providerUrl"], "https://example/one");

    let m3u = crate::preview_local_source_feed(
        "#EXTM3U\n#EXTINF:123,Artist One - Song One\nfile-one.flac\n#EXTINF:456,Artist Two - Song Two\nfile-two.flac",
        "auto",
        false,
        10,
    )
    .expect("M3U source preview");
    assert_eq!(m3u["sourceKind"], "m3u");
    assert_eq!(m3u["suggestionCount"], 2);
    assert_eq!(m3u["suggestions"][0]["searchText"], "Artist One Song One");
    assert_eq!(m3u["suggestions"][0]["sourceId"], "2");

    let rss = crate::preview_local_source_feed(
        "<rss><channel><item><title>Artist One - Song One</title></item><item><title>Artist One - Song One</title></item></channel></rss>",
        "auto",
        false,
        10,
    )
    .expect("RSS source preview");
    assert_eq!(rss["sourceKind"], "rss");
    assert_eq!(rss["totalRows"], 2);
    assert_eq!(rss["suggestionCount"], 1);
    assert_eq!(rss["duplicateCount"], 1);
    assert_eq!(
        rss["suggestions"][0]["evidenceKey"],
        "local:rss:artist one song one"
    );

    let opml = crate::preview_local_source_feed(
        "<opml><body><outline text=\"Artist One - Song One\"/><outline title='Artist Two - Song Two'/><outline text=\"\" title=\"ignored\"/></body></opml>",
        "opml",
        false,
        10,
    )
    .expect("OPML source preview");
    assert_eq!(opml["sourceKind"], "opml");
    assert_eq!(opml["totalRows"], 3);
    assert_eq!(opml["suggestionCount"], 2);
    assert_eq!(opml["skippedCount"], 1);
    assert_eq!(opml["suggestions"][0]["source"], "opml");
    assert_eq!(opml["suggestions"][0]["sourceId"], "1");
    assert_eq!(opml["suggestions"][1]["sourceId"], "2");
    assert_eq!(opml["suggestions"][1]["artist"], "Artist Two");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn source_provider_metadata_fallback_matches_frozen_priority_and_cleanup() {
    let html = r#"<html><head>
        <meta content="Lower Priority" property="twitter:title">
        <meta property="og:title" content="Quoted &amp; Song - YouTube">
        <meta name="byl" content="Fixture &amp; Artist">
        <title>Ignored Title - YouTube</title>
    </head></html>"#;
    let row =
        crate::provider_metadata_row("youtube", "https://www.youtube.com/watch?v=fixture", html)
            .expect("metadata row");
    assert_eq!(row.title, "Quoted & Song");
    assert_eq!(row.artist, "Fixture & Artist");
    assert_eq!(row.source, "youtube");
    assert_eq!(row.source_id, "1");
    assert_eq!(row.provider_url, "https://www.youtube.com/watch?v=fixture");

    let music_song = crate::provider_metadata_row(
        "bandcamp",
        "https://fixture.bandcamp.com/track/song",
        r#"<meta property='music:song' content='Artist - Preferred | Bandcamp'><meta property='og:title' content='Ignored'>"#,
    )
    .expect("music:song metadata row");
    assert_eq!(music_song.artist, "Artist");
    assert_eq!(music_song.title, "Preferred");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn source_provider_host_matching_rejects_suffix_spoofing() {
    for url in [
        "https://bandcamp.com.evil.example/track",
        "https://youtube.com.evil.example/watch?v=x",
        "https://last.fm.evil.example/user/x",
        "https://listenbrainz.org.evil.example/user/x",
        "https://music.apple.com.evil.example/album/x/1",
    ] {
        assert_eq!(crate::source_provider(url, "auto"), None, "accepted {url}");
    }
    assert_eq!(
        crate::source_provider("https://artist.bandcamp.com/track/song", "auto"),
        Some("bandcamp")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn notification_integrations_match_frozen_defaults_layers_and_validation() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_NTFY", "true")
            .with("SLSKD_NTFY_URL", "https://ntfy.sh/fixture")
            .with("SLSKD_NTFY_TOKEN", "ntfy-secret")
            .with("SLSKD_NTFY_NOTIFICATION_PREFIX", "Ntfy Prefix")
            .with("SLSKD_PUSHOVER", "true")
            .with("SLSKD_PUSHOVER_USER_KEY", "user-secret")
            .with("SLSKD_PUSHOVER_TOKEN", "app-secret")
            .with("SLSKD_PUSHOVER_NOTIFICATION_PREFIX", "Push Prefix")
            .with("SLSKD_PUSHBULLET", "true")
            .with("SLSKD_PUSHBULLET_ACCESS_TOKEN", "bullet-secret")
            .with("SLSKD_PUSHBULLET_NOTIFICATION_PREFIX", "Bullet Prefix")
            .with("SLSKD_PUSHBULLET_RETRY_ATTEMPTS", "5")
            .with("SLSKD_PUSHBULLET_COOLDOWN_TIME", "1234"),
    )
    .expect("configured notifications");
    assert!(config.integrations.ntfy.notify_on_private_message);
    assert!(config.integrations.ntfy.notify_on_room_mention);
    assert_eq!(config.integrations.ntfy.notification_prefix, "Ntfy Prefix");
    assert!(config.integrations.pushover.notify_on_private_message);
    assert!(config.integrations.pushover.notify_on_room_mention);
    assert_eq!(
        config.integrations.pushover.notification_prefix,
        "Push Prefix"
    );
    assert_eq!(
        config.integrations.pushbullet.notification_prefix,
        "Bullet Prefix"
    );
    assert_eq!(config.integrations.pushbullet.retry_attempts, 5);
    assert_eq!(config.integrations.pushbullet.cooldown_time, 1234);
    let sanitized = config.sanitized_json();
    for secret in ["ntfy-secret", "user-secret", "app-secret", "bullet-secret"] {
        assert!(!sanitized.contains(secret));
    }

    let ntfy_error = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_NTFY", "true"),
    )
    .expect_err("enabled Ntfy without URL must fail");
    assert_eq!(
        ntfy_error,
        "The Enabled field is true, but no Url has been specified for Ntfy."
    );
    let pushover_error = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_PUSHOVER", "true"),
    )
    .expect_err("enabled Pushover without keys must fail");
    assert_eq!(
        pushover_error,
        "The Enabled field is true, but no UserKey has been specified for Pushover."
    );
    let pushbullet_error = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_PUSHBULLET", "true"),
    )
    .expect_err("enabled Pushbullet without token must fail");
    assert_eq!(
        pushbullet_error,
        "The Enabled field is true, but no AccessToken has been specified."
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn notification_integrations_emit_frozen_wire_requests() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let ntfy = serve_json_fixture(&listener, serde_json::json!({"ok": true})).await;
        assert!(ntfy.starts_with("POST /ntfy HTTP/1.1"), "{ntfy}");
        assert!(
            ntfy.to_ascii_lowercase()
                .contains("authorization: bearer ntfy-secret"),
            "{ntfy}"
        );
        assert!(
            ntfy.contains("title: slskdN: Private Message from friend"),
            "{ntfy}"
        );
        assert!(ntfy.ends_with("hello from friend"), "{ntfy}");

        let pushover = serve_json_fixture(&listener, serde_json::json!({"status": 1})).await;
        assert!(
            pushover.starts_with("POST /pushover HTTP/1.1"),
            "{pushover}"
        );
        assert!(pushover.contains("token=app-secret"), "{pushover}");
        assert!(pushover.contains("user=user-secret"), "{pushover}");
        assert!(
            pushover.contains("title=slskdN%3A+Room+Mention+by+friend+in+room"),
            "{pushover}"
        );
        assert!(pushover.contains("message=hello+room"), "{pushover}");

        let pushbullet = serve_json_fixture(&listener, serde_json::json!({"iden": "push"})).await;
        assert!(
            pushbullet.starts_with("POST /pushbullet HTTP/1.1"),
            "{pushbullet}"
        );
        assert!(
            pushbullet
                .to_ascii_lowercase()
                .contains("access-token: bullet-secret"),
            "{pushbullet}"
        );
        let body = pushbullet.split("\r\n\r\n").nth(1).unwrap();
        let body = serde_json::from_str::<serde_json::Value>(body).unwrap();
        assert_eq!(body["type"], "note");
        assert_eq!(body["title"], "From slskdN: Private Message from friend");
        assert_eq!(body["body"], "hello from friend");
    });
    let ntfy = crate::config::NtfyIntegrationSettings {
        enabled: true,
        url: format!("http://{address}/ntfy"),
        access_token: "ntfy-secret".to_owned(),
        notification_prefix: "slskdN".to_owned(),
        notify_on_private_message: true,
        notify_on_room_mention: true,
    };
    let resolved = crate::ResolvedIntegrationTarget {
        host: "127.0.0.1".to_owned(),
        addrs: vec![address],
    };
    crate::send_ntfy_notification_to(
        &ntfy,
        "Private Message from friend",
        "hello from friend",
        &ntfy.url,
        &resolved,
    )
    .await
    .unwrap();
    let pushover = crate::config::PushoverIntegrationSettings {
        enabled: true,
        user_key: "user-secret".to_owned(),
        token: "app-secret".to_owned(),
        notification_prefix: "slskdN".to_owned(),
        notify_on_private_message: true,
        notify_on_room_mention: true,
    };
    crate::send_pushover_notification(
        &pushover,
        "Room Mention by friend in room",
        "hello room",
        &format!("http://{address}/pushover"),
    )
    .await
    .unwrap();
    let pushbullet = crate::config::PushbulletIntegrationSettings {
        enabled: true,
        access_token: "bullet-secret".to_owned(),
        notification_prefix: "From slskdN:".to_owned(),
        notify_on_private_message: true,
        notify_on_room_mention: true,
        retry_attempts: 1,
        cooldown_time: 900_000,
    };
    crate::send_pushbullet_notification(
        &pushbullet,
        "Private Message from friend",
        "hello from friend",
        &format!("http://{address}/pushbullet"),
    )
    .await
    .unwrap();
    server.await.unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn ftp_configuration_matches_frozen_layers_validation_and_secret_projection() {
    let state_dir = std::env::temp_dir().join(format!("slskr-ftp-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "integrations:\n  ftp:\n    enabled: true\n    address: ftp.example\n    port: 2121\n    encryption_mode: explicit\n    ignore_certificate_errors: true\n    username: fixture-user\n    password: fixture-password\n    remote_path: /incoming\n    overwrite_existing: false\n    connection_timeout: 4321\n    retry_attempts: 5\n",
    )
    .unwrap();
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", state_dir.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_FTP_PORT", "2222"),
    )
    .unwrap();
    let ftp = &config.integrations.ftp;
    assert!(ftp.enabled);
    assert_eq!(ftp.address, "ftp.example");
    assert_eq!(ftp.port, 2121);
    assert_eq!(ftp.encryption_mode, "explicit");
    assert!(ftp.ignore_certificate_errors);
    assert_eq!(ftp.username, "fixture-user");
    assert_eq!(ftp.password, "fixture-password");
    assert_eq!(ftp.remote_path, "/incoming");
    assert!(!ftp.overwrite_existing);
    assert_eq!(ftp.connection_timeout, 4321);
    assert_eq!(ftp.retry_attempts, 5);
    assert!(!config.sanitized_json().contains("fixture-password"));

    let env_config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_FTP", "true")
            .with("SLSKD_FTP_ADDRESS", "env.example")
            .with("SLSKD_FTP_PORT", "2021")
            .with("SLSKD_FTP_ENCRYPTION_MODE", "Implicit")
            .with("SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS", "true")
            .with("SLSKD_FTP_USERNAME", "env-user")
            .with("SLSKD_FTP_PASSWORD", "env-password")
            .with("SLSKD_FTP_REMOTE_PATH", "/env")
            .with("SLSKD_FTP_OVERWRITE_EXISTING", "false")
            .with("SLSKD_FTP_CONNECTION_TIMEOUT", "3210")
            .with("SLSKD_FTP_RETRY_ATTEMPTS", "4"),
    )
    .unwrap();
    assert_eq!(env_config.integrations.ftp.address, "env.example");
    assert_eq!(env_config.integrations.ftp.port, 2021);
    assert_eq!(env_config.integrations.ftp.encryption_mode, "Implicit");
    assert_eq!(env_config.integrations.ftp.username, "env-user");
    assert_eq!(env_config.integrations.ftp.password, "env-password");
    assert_eq!(env_config.integrations.ftp.remote_path, "/env");
    assert!(!env_config.integrations.ftp.overwrite_existing);
    assert_eq!(env_config.integrations.ftp.connection_timeout, 3210);
    assert_eq!(env_config.integrations.ftp.retry_attempts, 4);

    let overlay = crate::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["integration"]["ftp"],
        serde_json::json!({
            "enabled": true,
            "address": "ftp.example",
            "port": 2121,
            "encryptionMode": "explicit",
            "ignoreCertificateErrors": true,
            "username": "fixture-user",
            "password": "*****",
            "remotePath": "/incoming",
            "overwriteExisting": false,
            "connectionTimeout": 4321,
            "retryAttempts": 5,
        })
    );

    for (name, value, expected) in [
        ("SLSKD_FTP", "true", "no Address"),
        ("SLSKD_FTP_PORT", "0", "Port"),
        ("SLSKD_FTP_ENCRYPTION_MODE", "bogus", "EncryptionMode"),
        ("SLSKD_FTP_RETRY_ATTEMPTS", "6", "RetryAttempts"),
    ] {
        let mut env = MapEnv::default().with(name, value);
        if name != "SLSKD_FTP" {
            env = env.with("SLSKD_FTP_ADDRESS", "ftp.example");
        }
        let error = crate::AppConfig::from_layers(None, FileConfig::default(), &env)
            .expect_err("invalid FTP configuration must fail");
        assert!(error.contains(expected), "{error}");
    }
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn vpn_configuration_matches_frozen_layers_validation_and_secret_projection() {
    let state_dir = std::env::temp_dir().join(format!("slskr-vpn-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "integrations:\n  vpn:\n    enabled: true\n    port_forwarding: true\n    polling_interval: 3456\n    gluetun:\n      url: http://127.0.0.1:8000\n      timeout: 2345\n      auth: ignored-documentation-leaf\n      username: fixture-user\n      password: fixture-password\n      api_key: fixture-api-key\n",
    )
    .unwrap();
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", state_dir.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    let vpn = &config.integrations.vpn;
    assert!(vpn.enabled);
    assert!(vpn.port_forwarding);
    assert!(!vpn.self_hosted_relay);
    assert_eq!(vpn.polling_interval, 3456);
    assert_eq!(vpn.gluetun.url, "http://127.0.0.1:8000");
    assert_eq!(vpn.gluetun.timeout, 2345);
    assert_eq!(vpn.gluetun.auth, "");
    assert_eq!(vpn.gluetun.username, "fixture-user");
    assert_eq!(vpn.gluetun.password, "fixture-password");
    assert_eq!(vpn.gluetun.api_key, "fixture-api-key");
    assert!(!config.sanitized_json().contains("fixture-password"));
    assert!(!config.sanitized_json().contains("fixture-api-key"));

    let overlay = crate::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&crate::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["integration"]["vpn"],
        serde_json::json!({
            "enabled": true,
            "portForwarding": true,
            "pollingInterval": 3456,
            "gluetun": {
                "version": 1,
                "url": "http://127.0.0.1:8000",
                "timeout": 2345,
                "username": "fixture-user",
                "password": "*****",
                "apiKey": "*****",
            },
        })
    );
    assert!(!options.to_string().contains("ignored-documentation-leaf"));
    assert!(!options.to_string().contains("fixture-password"));
    assert!(!options.to_string().contains("fixture-api-key"));

    let env_config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_VPN", "true")
            .with("SLSKD_VPN_PORT_FORWARDING", "true")
            .with("SLSKD_VPN_POLLING_INTERVAL", "4000")
            .with("SLSKD_VPN_GLUETUN_URL", "http://127.0.0.1:9000")
            .with("SLSKD_VPN_GLUETUN_TIMEOUT", "3000")
            .with("SLSKD_VPN_GLUETUN_USERNAME", "env-user")
            .with("SLSKD_VPN_GLUETUN_PASSWORD", "env-password")
            .with("SLSKD_VPN_GLUETUN_API_KEY", "env-api-key"),
    )
    .unwrap();
    assert!(env_config.integrations.vpn.enabled);
    assert!(env_config.integrations.vpn.port_forwarding);
    assert_eq!(env_config.integrations.vpn.polling_interval, 4000);
    assert_eq!(env_config.integrations.vpn.gluetun.timeout, 3000);
    assert_eq!(env_config.integrations.vpn.gluetun.username, "env-user");

    for (name, value, expected) in [
        ("SLSKD_VPN", "true", "no client"),
        ("SLSKD_VPN_POLLING_INTERVAL", "499", "PollingInterval"),
        ("SLSKD_VPN_GLUETUN_TIMEOUT", "499", "Timeout"),
        ("SLSKD_VPN_GLUETUN_TIMEOUT", "10001", "Timeout"),
    ] {
        let error = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default().with(name, value),
        )
        .expect_err("invalid VPN configuration must fail");
        assert!(error.contains(expected), "{error}");
    }
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn vpn_state_projects_and_blocks_soulseek_until_ready() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_VPN", "true")
            .with("SLSKD_VPN_GLUETUN_URL", "http://127.0.0.1:8000"),
    );
    let mut session = None;
    let mut next_ping = tokio::time::Instant::now();
    assert!(!crate::connect_session(&state, &mut session, &mut next_ping).await);
    assert!(session.is_none());
    assert_eq!(
        state.session.read().await.last_error.as_deref(),
        Some("Waiting for VPN client")
    );
    assert!(state.runtime.read().await.vpn_reconnect_requested);

    state.runtime.write().await.vpn = crate::vpn::Status {
        is_ready: true,
        is_connected: true,
        public_ip_address: Some("203.0.113.5".parse().unwrap()),
        location: "Regina, Canada".to_owned(),
        forwarded_port: Some(44_444),
        port_forwards: vec![crate::vpn::PortForward {
            slot: 0,
            local_port: 50_300,
            target_port: 50_300,
            proto: "tcp".to_owned(),
            public_port: 44_444,
            public_ip_address: Some("203.0.113.5".parse().unwrap()),
            namespace: "slskdn".to_owned(),
        }],
        relay: None,
    };
    let response = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .unwrap();
    let application = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(
        application["vpn"],
        serde_json::json!({
            "isReady": true,
            "isConnected": true,
            "publicIPAddress": "203.0.113.5",
            "location": "Regina, Canada",
            "forwardedPort": 44444,
            "portForwards": [{
                "slot": 0,
                "localPort": 50300,
                "targetPort": 50300,
                "proto": "tcp",
                "publicPort": 44444,
                "publicIPAddress": "203.0.113.5",
                "namespace": "slskdn",
            }],
        })
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn oauth_state_loader_reports_expired_state_cleanup_failure() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    db.upsert_oauth_state(&crate::persistence::OAuthStateRecord {
        state: "expired-state".to_owned(),
        provider: "spotify".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        created_at: 0,
        expires_at: 1,
    })
    .await
    .expect("persist expired OAuth state");
    db.fail_oauth_delete_for_test()
        .await
        .expect("install delete failure trigger");

    let error = crate::load_oauth_state_store(Some(&db))
        .await
        .expect_err("cleanup failure must fail initialization");
    assert!(error.contains("failed to delete expired persisted OAuth states"));
    assert!(error.contains("forced OAuth delete failure"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_oauth_state_persists_rehydrates_and_consumes() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_HTTP_BIND", "127.0.0.1:7788"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let authorize = crate::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    assert_eq!(authorize.status, "200 OK");

    let now = i64::try_from(crate::unix_timestamp()).unwrap_or(i64::MAX);
    let persisted = db
        .list_oauth_states(now, 10, 0)
        .await
        .expect("list persisted oauth states");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].provider, "spotify");

    let rehydrated = crate::OAuthStateStore::from_persisted(persisted.clone());
    assert!(rehydrated.records.contains_key(&persisted[0].state));
    assert!(rehydrated.records[&persisted[0].state]
        .code_verifier
        .is_none());
    *state.oauth_states.write().await = rehydrated;

    let callback_path = format!(
        "/api/integrations/spotify/callback?code=abc123&state={}",
        persisted[0].state
    );
    let callback = crate::route_http_request("GET", &callback_path, None, "", &state)
        .await
        .expect("callback response");
    assert_eq!(callback.status, "400 Bad Request");
    assert!(callback
        .body
        .contains("Spotify authorization could not be completed."));
    assert!(db
        .list_oauth_states(now, 10, 0)
        .await
        .expect("list after consume")
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_oauth_state_is_not_consumed_when_persistence_delete_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    crate::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    let issued_state = state
        .oauth_states
        .read()
        .await
        .records
        .keys()
        .next()
        .cloned()
        .expect("issued state");
    db.close_for_test().await;

    let callback_path =
        format!("/api/integrations/spotify/callback?code=abc123&state={issued_state}");
    let callback = crate::route_http_request("GET", &callback_path, None, "", &state)
        .await
        .expect("callback response");
    assert_eq!(callback.status, "503 Service Unavailable");
    assert!(state
        .oauth_states
        .read()
        .await
        .records
        .contains_key(&issued_state));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_oauth_state_is_not_issued_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.oauth_states.write().await.records.insert(
        "expired".to_owned(),
        crate::OAuthStateRecord {
            provider: "spotify".to_owned(),
            redirect_uri: "http://localhost/callback".to_owned(),
            code_verifier: None,
            created_at: 0,
            expires_at: 0,
        },
    );
    let previous = state.oauth_states.read().await.clone();
    db.close_for_test().await;

    let response = crate::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("failed OAuth state persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("OAuth state persistence failed"));
    assert_eq!(*state.oauth_states.read().await, previous);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn lidarr_manual_import_rolls_back_both_stores_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let previous_library = state.library.read().await.clone();
    let previous_runtime = state.runtime.read().await.clone();
    db.close_for_test().await;

    let response = crate::route_http_request(
        "POST",
        "/api/integrations/lidarr/manualimport",
        None,
        r#"{"artist":"Artist","title":"Album","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("failed Lidarr manual import persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("library persistence failed: Lidarr manual import transaction failed"));
    assert_eq!(*state.library.read().await, previous_library);
    assert_eq!(*state.runtime.read().await, previous_runtime);
}

#[cfg_attr(test, tokio::test)]
#[cfg(all(
    feature = "full-controller-tests",
    not(feature = "legacy-route-dispatch")
))]
pub(super) async fn lidarr_manual_import_releases_store_guards_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-legacy-lidarr-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = crate::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create manual import database");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let blocker_pool = sqlx_sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx_sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await
        .expect("open SQLite lock connection");
    let mut blocker = blocker_pool
        .acquire()
        .await
        .expect("acquire SQLite lock connection");
    sqlx_core::query::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold SQLite write lock");

    let task_state = Arc::clone(&state);
    let mutation = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/integrations/lidarr/manualimport",
            None,
            r#"{"artist":"Artist","title":"Album","kind":"Audio"}"#,
            &task_state,
        )
        .await
    });
    let stores_visible_during_sqlite_wait = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let library_visible = state.library.try_read().is_ok_and(|library| {
                library
                    .records
                    .iter()
                    .any(|record| record.artist == "Artist" && record.title == "Album")
            });
            let runtime_visible = state
                .runtime
                .try_read()
                .is_ok_and(|runtime| runtime.lidarr_manual_imports == 1);
            if library_visible && runtime_visible {
                break true;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let request_waited_for_sqlite = !mutation.is_finished();
    let library_reader_responsive = state.library.try_read().is_ok();
    let runtime_reader_responsive = state.runtime.try_read().is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let response = tokio::time::timeout(Duration::from_secs(3), mutation)
        .await
        .expect("manual import should finish after releasing SQLite")
        .expect("manual import route task should join")
        .expect("manual import response");

    assert!(
        stores_visible_during_sqlite_wait,
        "both in-memory mutations should be readable while SQLite is blocked"
    );
    assert!(
        request_waited_for_sqlite,
        "the request should wait for SQLite"
    );
    assert!(
        library_reader_responsive,
        "library readers should not wait for SQLite"
    );
    assert!(
        runtime_reader_responsive,
        "runtime readers should not wait for SQLite"
    );
    assert_eq!(response.status, "202 Accepted");
    let persisted_library = db
        .list_library_items(10, 0)
        .await
        .expect("list persisted library items");
    assert!(persisted_library
        .iter()
        .any(|record| record.artist == "Artist" && record.title == "Album"));
    assert_eq!(
        db.get_runtime_compat_state()
            .await
            .expect("load runtime compatibility state")
            .expect("persisted runtime state")
            .lidarr_manual_imports,
        1
    );

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_registration_rejects_invalid_events_and_caps_count() {
    let (state, _receiver) = test_state();

    let (array_state, _array_receiver) = test_state();
    let array_created = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/array-hook","events":["message.sent","search.created","message.sent"]}"#,
        &array_state,
    )
    .await
    .expect("array webhook");
    assert_eq!(array_created.status, "201 Created");
    let array_events = array_state
        .webhooks
        .read()
        .await
        .get_all()
        .first()
        .map(|webhook| webhook.events.clone())
        .expect("array webhook record");
    assert_eq!(
        array_events,
        vec![
            crate::webhooks::WebhookEvent::MessageSent,
            crate::webhooks::WebhookEvent::SearchCreated,
        ]
    );

    let admin_array_created = crate::route_http_request(
        "POST",
        "/api/admin/webhooks",
        None,
        r#"{"url":"https://example.test/admin-array-hook","events":["transfer.failed"]}"#,
        &array_state,
    )
    .await
    .expect("admin array webhook");
    assert_eq!(admin_array_created.status, "201 Created");
    assert!(array_state
        .webhooks
        .read()
        .await
        .get_all()
        .iter()
        .any(|webhook| webhook.events == vec![crate::webhooks::WebhookEvent::TransferFailed]));

    let invalid = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created,bogus"}"#,
        &state,
    )
    .await
    .expect("invalid webhook");
    assert_eq!(invalid.status, "400 Bad Request");
    assert_eq!(invalid.body, "{\"error\":\"invalid webhook event\"}");

    let weak_secret = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created","secret":"short"}"#,
        &state,
    )
    .await
    .expect("weak webhook secret");
    assert_eq!(weak_secret.status, "400 Bad Request");
    assert!(weak_secret.body.contains("webhook secret"));

    let weak_admin_secret = crate::route_http_request(
        "POST",
        "/api/admin/webhooks",
        None,
        r#"{"url":"https://example.test/admin-hook","secret":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        &state,
    )
    .await
    .expect("weak admin webhook secret");
    assert_eq!(weak_admin_secret.status, "400 Bad Request");
    assert!(weak_admin_secret.body.contains("webhook secret"));

    for index in 0..crate::webhooks::MAX_WEBHOOKS {
        let body =
            format!(r#"{{"url":"https://example.test/hook/{index}","events":"search.created"}}"#);
        let created = crate::route_http_request("POST", "/api/webhooks", None, &body, &state)
            .await
            .expect("create webhook");
        assert_eq!(created.status, "201 Created");
        if index == 0 {
            let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
            assert_eq!(created_json["secretReturnedOnce"], true);
            assert!(created_json["secret"]
                .as_str()
                .is_some_and(|value| !value.is_empty()));
        }
    }

    let listed = crate::route_http_request("GET", "/api/webhooks", None, "", &state)
        .await
        .expect("list webhooks");
    assert_eq!(listed.status, "200 OK");
    assert!(!listed.body.contains("\"secret\""));

    let capped = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook/overflow","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("capped webhook");
    assert_eq!(capped.status, "400 Bad Request");
    assert_eq!(capped.body, "{\"error\":\"webhook limit reached\"}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_config_persists_rehydrates_and_records_dispatch_logs() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.session.write().await.state = "connected";
    let secret = crate::webhooks::Webhook::generate_secret().expect("test randomness");
    let created = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        &format!(
            "{{\"url\":\"https://example.com/hook\",\"events\":\"search.created,message.sent\",\"secret\":\"{}\"}}",
            crate::json_escape(&secret)
        ),
        &state,
    )
    .await
    .expect("create webhook");
    assert_eq!(created.status, "201 Created");
    let created_json: serde_json::Value = serde_json::from_str(&created.body).unwrap();
    let webhook_id = created_json["id"].as_str().expect("webhook id");

    let persisted = db.list_webhooks().await.expect("list webhooks");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].id, webhook_id);
    assert_eq!(persisted[0].events, "search.created,message.sent");

    let rehydrated = crate::webhooks::WebhookManager::from_webhooks(
        persisted
            .clone()
            .into_iter()
            .filter_map(crate::webhooks::webhook_from_persisted)
            .collect(),
    );
    assert_eq!(rehydrated.get_all().len(), 1);
    assert!(rehydrated
        .get_for_event(crate::webhooks::WebhookEvent::MessageSent)
        .iter()
        .any(|webhook| webhook.id == webhook_id));

    let admin_list = crate::route_http_request("GET", "/api/admin/webhooks", None, "", &state)
        .await
        .expect("list admin webhooks");
    assert_eq!(admin_list.status, "200 OK");
    assert!(admin_list.body.contains("\"retry_count\":0"));
    assert!(admin_list.body.contains("\"max_retries\":3"));
    assert!(admin_list.body.contains("\"timeout_seconds\":30"));

    let patched = crate::route_http_request(
        "PATCH",
        &format!("/api/webhooks/{webhook_id}"),
        None,
        "{\"active\":false}",
        &state,
    )
    .await
    .expect("patch webhook");
    assert_eq!(patched.status, "200 OK");
    let persisted_patch = db.get_webhook(webhook_id).await.unwrap().unwrap();
    assert!(!persisted_patch.active);

    let _ = crate::route_http_request(
        "PATCH",
        &format!("/api/webhooks/{webhook_id}"),
        None,
        "{\"active\":true}",
        &state,
    )
    .await
    .expect("reactivate webhook");
    let created_search = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"webhook log\"}",
        &state,
    )
    .await
    .expect("create search");
    assert_eq!(created_search.status, "200 OK");
    let logs = db
        .get_webhook_logs(webhook_id, 10, 0)
        .await
        .expect("webhook logs");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].event, "search.created");
    assert!(
        matches!(logs[0].status.as_str(), "queued" | "success" | "failed"),
        "unexpected webhook delivery status: {}",
        logs[0].status
    );

    let routed_logs = crate::route_http_request(
        "GET",
        &format!("/api/webhooks/{webhook_id}/logs"),
        None,
        "",
        &state,
    )
    .await
    .expect("route webhook logs");
    assert_eq!(routed_logs.status, "200 OK");
    assert!(routed_logs.body.contains("search.created"));

    let aliased_logs = crate::route_http_request(
        "GET",
        &format!("/api/webhooks/unrelated/{webhook_id}/logs"),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased webhook logs");
    assert_eq!(aliased_logs.status, "404 Not Found");

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/webhooks/{webhook_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete webhook");
    assert_eq!(deleted.status, "200 OK");
    assert!(db.list_webhooks().await.expect("list deleted").is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_audit_persistence_failures_surface_in_session_health() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let mut manager = crate::webhooks::WebhookManager::new();
    manager
        .register(crate::webhooks::Webhook::new(
            "https://example.com/hook".to_owned(),
            vec![crate::webhooks::WebhookEvent::SearchCreated],
            "secret_0123456789abcdef0123456789abcdef".to_owned(),
        ))
        .expect("register webhook");
    db.close_for_test().await;

    let persistence_turn = state.webhook_persistence_lock.lock().await;
    let error = crate::webhooks::persist_webhook_dispatch_logs(
        &state,
        &manager,
        crate::webhooks::WebhookEvent::SearchCreated,
        "correlation-test",
        r#"{"query":"test"}"#,
        &persistence_turn,
    )
    .await;
    drop(persistence_turn);
    if let Some(error) = error {
        crate::session_runtime::update_session(&state, |session| {
            session.last_error = Some(error);
        })
        .await;
    }

    let session = state.session.read().await;
    let error = session.last_error.as_deref().unwrap_or_default();
    assert!(error.contains("webhook audit persistence failed for 1 delivery record"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_routes_roll_back_when_persistence_fails() {
    for path in ["/api/webhooks", "/api/admin/webhooks"] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.webhooks.read().await.clone();
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            path,
            None,
            r#"{"url":"https://example.test/hook","events":"search.created"}"#,
            &state,
        )
        .await
        .expect("failed webhook create response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("webhook persistence failed"),
            "{path}"
        );
        assert_eq!(*state.webhooks.read().await, previous, "{path}");
    }

    for (method, prefix, body, expected_error) in [
        (
            "PATCH",
            "/api/webhooks/",
            r#"{"active":false}"#,
            "webhook persistence failed",
        ),
        (
            "DELETE",
            "/api/webhooks/",
            "",
            "webhook deletion persistence failed",
        ),
        (
            "DELETE",
            "/api/admin/webhooks/",
            "",
            "webhook deletion persistence failed",
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let webhook = crate::webhooks::Webhook::new(
            "https://example.test/hook".to_owned(),
            vec![crate::webhooks::WebhookEvent::SearchCreated],
            "0123456789abcdef0123456789abcdef".to_owned(),
        );
        let webhook_id = webhook.id.clone();
        state.webhooks.write().await.register(webhook).unwrap();
        let previous = state.webhooks.read().await.clone();
        db.close_for_test().await;

        let response =
            crate::route_http_request(method, &format!("{prefix}{webhook_id}"), None, body, &state)
                .await
                .expect("failed webhook mutation response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {prefix}"
        );
        assert!(response.body.contains(expected_error), "{method} {prefix}");
        assert_eq!(*state.webhooks.read().await, previous, "{method} {prefix}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_test_send_rejects_when_delivery_pool_is_full() {
    let (state, _receiver) = test_state();

    let created = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("create webhook");
    assert_eq!(created.status, "201 Created");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let webhook_id = created_json["id"].as_str().unwrap();
    let _all_delivery_permits = Arc::clone(&state.webhook_deliveries)
        .acquire_many_owned(crate::MAX_WEBHOOK_DELIVERY_TASKS as u32)
        .await
        .expect("acquire webhook delivery permits");

    let response = crate::route_http_request(
        "POST",
        &format!("/api/webhooks/{webhook_id}/test"),
        None,
        "",
        &state,
    )
    .await
    .expect("test webhook");

    assert_eq!(response.status, "429 Too Many Requests");
    assert!(response.body.contains("webhook deliveries"));

    let admin_response = crate::route_http_request(
        "POST",
        &format!("/api/admin/webhooks/{webhook_id}/test"),
        None,
        "",
        &state,
    )
    .await
    .expect("admin test webhook");

    assert_eq!(admin_response.status, "429 Too Many Requests");
    assert!(admin_response.body.contains("webhook deliveries"));
}
