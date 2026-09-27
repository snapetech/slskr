#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spotify_authorization_exchanges_profiles_persists_and_disconnects() {
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
        super::unix_timestamp_millis()
    ));
    super::ensure_private_state_dir(&root).unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    let spotify = state.integration_settings.read().await.spotify.clone();
    let pending = super::OAuthStateRecord {
        provider: "spotify".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        code_verifier: Some("pkce-verifier".to_owned()),
        created_at: super::unix_timestamp(),
        expires_at: super::unix_timestamp().saturating_add(600),
    };
    let status = super::complete_spotify_authorization(
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
    let path = super::spotify_connection_path(&root);
    let encrypted = fs::read_to_string(&path).unwrap();
    assert!(!encrypted.contains("access-secret"));
    assert!(!encrypted.contains("refresh-secret"));
    let loaded = super::load_spotify_connection_store(&root, &state.capability_signing_key);
    assert_eq!(loaded, *state.spotify_connection.read().await);

    let response = super::route_http_request(
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
        super::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
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
fn spotify_source_targets_match_frozen_native_forms() {
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
        let target = super::parse_spotify_source_target(input);
        assert_eq!(target.kind, kind, "{input}");
        assert_eq!(target.id, id, "{input}");
        assert_eq!(target.requires_user_token, user_token, "{input}");
        assert_eq!(target.scope_hint, scope, "{input}");
    }
    assert!(super::looks_like_spotify_source(
        "https://open.spotify.com/track/track-1",
        "auto"
    ));
    assert!(!super::looks_like_spotify_source(
        "https://open.spotify.com.evil.example/track/track-1",
        "auto"
    ));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spotify_source_preview_refreshes_and_pages_saved_tracks() {
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
        super::unix_timestamp_millis()
    ));
    super::ensure_private_state_dir(&root).unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    *state.spotify_connection.write().await = super::SpotifyConnectionStore {
        refresh_token: "refresh-secret".to_owned(),
        expires_at: 0,
        display_name: "Fixture User".to_owned(),
        spotify_user_id: "fixture-user".to_owned(),
        ..Default::default()
    };
    let result = super::preview_spotify_source_feed(
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
    let protected = fs::read_to_string(super::spotify_connection_path(&root)).unwrap();
    assert!(!protected.contains("refreshed-access"));
    assert!(!protected.contains("refresh-secret"));
    let _ = fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spotify_source_preview_uses_app_token_and_market() {
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
    let result = super::preview_spotify_source_feed(
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
async fn spotify_source_route_reports_required_user_scope_without_connection() {
    let root = std::env::temp_dir().join(format!(
        "slskr-spotify-source-route-test-{}-{}",
        std::process::id(),
        super::unix_timestamp_millis()
    ));
    super::ensure_private_state_dir(&root).unwrap();
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
    );
    let response = super::route_http_request(
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
async fn spotify_source_requests_enforce_configured_timeout() {
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
    let error = super::preview_spotify_source_feed(
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
async fn force_checking_the_latest_version_awaits_a_real_github_lookup() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind version fixture");
    let address = listener.local_addr().expect("version fixture address");
    let server = tokio::spawn(async move {
        serve_json_fixture(
            &listener,
            serde_json::json!({
                "tag_name": "v9.9.9-slskdn.20260101120000",
                "html_url": "https://github.com/snapetech/slskdn/releases/tag/v9.9.9-slskdn.20260101120000",
            }),
        )
        .await
    });
    let (state, _receiver) = test_state();
    super::refresh_controller_version_check(&state, &format!("http://{address}"))
        .await
        .expect("version fixture lookup");
    server.await.expect("version fixture task");

    let version = state
        .controller_version
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        version.latest.as_deref(),
        Some("9.9.9-slskdn.20260101120000")
    );
    assert!(version.latest_tag.is_some());
    assert!(version.checked_at.is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn musicbrainz_release_lookup_resolves_a_release_with_an_artist_credit() {
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
    let target = super::musicbrainz_release_target(&format!("http://{address}"), "release-1")
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
async fn musicbrainz_release_lookup_returns_none_when_release_is_not_found() {
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
    let target = super::musicbrainz_release_target(&format!("http://{address}"), "missing-release")
        .await
        .expect("MusicBrainz lookup");
    server.await.expect("MusicBrainz fixture task");
    assert_eq!(target, None);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn youtube_and_lastfm_source_providers_fetch_real_rows() {
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
    let youtube = super::preview_configured_provider_source_feed(
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

    let lastfm = super::preview_configured_provider_source_feed(
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
async fn apple_and_listenbrainz_source_providers_fetch_real_rows() {
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
    let apple = super::preview_configured_provider_source_feed(
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

    let listenbrainz = super::preview_configured_provider_source_feed(
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
fn youtube_and_lastfm_settings_require_keys_only_when_enabled() {
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
        let error = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default().with(enabled, "true"),
        )
        .expect_err("enabled provider without API key must fail");
        assert!(error.contains(expected_error), "{error}");
        let configured = super::AppConfig::from_layers(
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
fn source_feed_local_formats_match_frozen_parsing_and_deduplication() {
    let csv = super::preview_local_source_feed(
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

    let m3u = super::preview_local_source_feed(
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

    let rss = super::preview_local_source_feed(
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

    let opml = super::preview_local_source_feed(
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
fn source_provider_metadata_fallback_matches_frozen_priority_and_cleanup() {
    let html = r#"<html><head>
        <meta content="Lower Priority" property="twitter:title">
        <meta property="og:title" content="Quoted &amp; Song - YouTube">
        <meta name="byl" content="Fixture &amp; Artist">
        <title>Ignored Title - YouTube</title>
    </head></html>"#;
    let row =
        super::provider_metadata_row("youtube", "https://www.youtube.com/watch?v=fixture", html)
            .expect("metadata row");
    assert_eq!(row.title, "Quoted & Song");
    assert_eq!(row.artist, "Fixture & Artist");
    assert_eq!(row.source, "youtube");
    assert_eq!(row.source_id, "1");
    assert_eq!(row.provider_url, "https://www.youtube.com/watch?v=fixture");

    let music_song = super::provider_metadata_row(
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
fn source_provider_host_matching_rejects_suffix_spoofing() {
    for url in [
        "https://bandcamp.com.evil.example/track",
        "https://youtube.com.evil.example/watch?v=x",
        "https://last.fm.evil.example/user/x",
        "https://listenbrainz.org.evil.example/user/x",
        "https://music.apple.com.evil.example/album/x/1",
    ] {
        assert_eq!(super::source_provider(url, "auto"), None, "accepted {url}");
    }
    assert_eq!(
        super::source_provider("https://artist.bandcamp.com/track/song", "auto"),
        Some("bandcamp")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn notification_integrations_match_frozen_defaults_layers_and_validation() {
    let config = super::AppConfig::from_layers(
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

    let ntfy_error = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_NTFY", "true"),
    )
    .expect_err("enabled Ntfy without URL must fail");
    assert_eq!(
        ntfy_error,
        "The Enabled field is true, but no Url has been specified for Ntfy."
    );
    let pushover_error = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_PUSHOVER", "true"),
    )
    .expect_err("enabled Pushover without keys must fail");
    assert_eq!(
        pushover_error,
        "The Enabled field is true, but no UserKey has been specified for Pushover."
    );
    let pushbullet_error = super::AppConfig::from_layers(
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
async fn notification_integrations_emit_frozen_wire_requests() {
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
    let resolved = super::ResolvedIntegrationTarget {
        host: "127.0.0.1".to_owned(),
        addrs: vec![address],
    };
    super::send_ntfy_notification_to(
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
    super::send_pushover_notification(
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
    super::send_pushbullet_notification(
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
fn frozen_webhook_configuration_projects_dynamic_names_and_redacts_headers() {
    let state_dir =
        std::env::temp_dir().join(format!("slskr-webhook-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(state_dir.join("slskd.yml"), "flags:\n  no_connect: true\nintegrations:\n  webhooks:\n    my_webhook:\n      on: [Any, PrivateMessageReceived]\n      call:\n        url: https://example.com/hook\n        headers:\n          - name: Authorization\n            value: secret-value\n        ignore_certificate_errors: false\n      timeout: 1234\n      retry:\n        attempts: 2\n").unwrap();
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKD_APP_DIR", state_dir.to_str().unwrap()),
    )
    .unwrap();
    let hook = &config.integrations.frozen_webhooks["my_webhook"];
    assert_eq!(hook.on, ["Any", "PrivateMessageReceived"]);
    assert_eq!(hook.call.headers[0].value, "secret-value");
    assert_eq!(hook.timeout, 1234);
    assert_eq!(hook.retry.attempts, 2);
    let overlay = super::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["integration"]["webhooks"]["mywebhook"]["call"]["headers"][0]["value"],
        "*****"
    );
    assert!(!options.to_string().contains("secret-value"));
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn ftp_configuration_matches_frozen_layers_validation_and_secret_projection() {
    let state_dir = std::env::temp_dir().join(format!("slskr-ftp-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "integrations:\n  ftp:\n    enabled: true\n    address: ftp.example\n    port: 2121\n    encryption_mode: explicit\n    ignore_certificate_errors: true\n    username: fixture-user\n    password: fixture-password\n    remote_path: /incoming\n    overwrite_existing: false\n    connection_timeout: 4321\n    retry_attempts: 5\n",
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
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

    let env_config = super::AppConfig::from_layers(
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

    let overlay = super::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
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
        let error = super::AppConfig::from_layers(None, FileConfig::default(), &env)
            .expect_err("invalid FTP configuration must fail");
        assert!(error.contains(expected), "{error}");
    }
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn vpn_configuration_matches_frozen_layers_validation_and_secret_projection() {
    let state_dir = std::env::temp_dir().join(format!("slskr-vpn-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "integrations:\n  vpn:\n    enabled: true\n    port_forwarding: true\n    polling_interval: 3456\n    gluetun:\n      url: http://127.0.0.1:8000\n      timeout: 2345\n      auth: ignored-documentation-leaf\n      username: fixture-user\n      password: fixture-password\n      api_key: fixture-api-key\n",
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
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

    let overlay = super::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
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

    let env_config = super::AppConfig::from_layers(
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
        let error = super::AppConfig::from_layers(
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
async fn vpn_state_projects_and_blocks_soulseek_until_ready() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_VPN", "true")
            .with("SLSKD_VPN_GLUETUN_URL", "http://127.0.0.1:8000"),
    );
    let mut session = None;
    let mut next_ping = tokio::time::Instant::now();
    assert!(!super::connect_session(&state, &mut session, &mut next_ping).await);
    assert!(session.is_none());
    assert_eq!(
        state.session.read().await.last_error.as_deref(),
        Some("Waiting for VPN client")
    );
    assert!(state.runtime.read().await.vpn_reconnect_requested);

    state.runtime.write().await.vpn = super::vpn::Status {
        is_ready: true,
        is_connected: true,
        public_ip_address: Some("203.0.113.5".parse().unwrap()),
        location: "Regina, Canada".to_owned(),
        forwarded_port: Some(44_444),
        port_forwards: vec![super::vpn::PortForward {
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
    let response = super::route_http_request("GET", "/api/v0/application", None, "", &state)
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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn frozen_script_configuration_validates_and_projects_all_execution_modes() {
    let state_dir =
        std::env::temp_dir().join(format!("slskr-script-config-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("slskd.yml"),
        "integrations:\n  scripts:\n    command_mode:\n      on: [DownloadFileComplete]\n      run:\n        command: echo fixture\n    args_mode:\n      on: [Any]\n      run:\n        executable: /bin/sh\n        args: '-c \"echo fixture\"'\n    arglist_mode:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        args_list: [-c, 'echo fixture']\n",
    )
    .unwrap();
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_APP_DIR", state_dir.to_str().unwrap())
            .with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    assert_eq!(config.integrations.scripts.len(), 3);
    assert_eq!(
        config.integrations.scripts["arglist_mode"].run.arglist,
        Some(vec!["-c".to_owned(), "echo fixture".to_owned()])
    );
    let overlay = super::ControllerOptionsOverlayState::load(&config).unwrap();
    let options = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &config, &overlay, true,
    ))
    .unwrap();
    assert_eq!(
        options["integration"]["scripts"]["commandmode"]["run"]["command"],
        "echo fixture"
    );
    assert_eq!(
        options["integration"]["scripts"]["argsmode"]["run"]["executable"],
        "/bin/sh"
    );
    assert_eq!(
        options["integration"]["scripts"]["arglistmode"]["run"]["arglist"],
        serde_json::json!(["-c", "echo fixture"])
    );

    for (yaml, expected) in [
        (
            "integrations:\n  scripts:\n    bad:\n      on: [NotAnEvent]\n      run:\n        command: echo\n",
            "invalid event",
        ),
        (
            "integrations:\n  scripts:\n    bad:\n      on: [Any]\n      run: {}\n",
            "One and only one",
        ),
        (
            "integrations:\n  scripts:\n    bad:\n      on: [Any]\n      run:\n        command: echo\n        executable: /bin/sh\n",
            "One and only one",
        ),
        (
            "integrations:\n  scripts:\n    bad:\n      on: [Any]\n      run:\n        executable: /bin/sh\n        args: -c echo\n        arglist: [-c, echo]\n",
            "Only one",
        ),
    ] {
        fs::write(state_dir.join("slskd.yml"), yaml).unwrap();
        let error = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default().with("SLSKD_APP_DIR", state_dir.to_str().unwrap()),
        )
        .expect_err("invalid script configuration must fail");
        assert!(error.contains(expected), "{error}");
    }
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn oauth_state_loader_reports_expired_state_cleanup_failure() {
    let db = super::persistence::DatabaseManager::in_memory()
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

    let error = super::load_oauth_state_store(Some(&db))
        .await
        .expect_err("cleanup failure must fail initialization");
    assert!(error.contains("failed to delete expired persisted OAuth states"));
    assert!(error.contains("forced OAuth delete failure"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spotify_oauth_state_persists_rehydrates_and_consumes() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
            .with("SLSKR_HTTP_BIND", "127.0.0.1:7788"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let authorize = super::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    assert_eq!(authorize.status, "200 OK");

    let now = i64::try_from(super::unix_timestamp()).unwrap_or(i64::MAX);
    let persisted = db
        .list_oauth_states(now, 10, 0)
        .await
        .expect("list persisted oauth states");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].provider, "spotify");

    let rehydrated = super::OAuthStateStore::from_persisted(persisted.clone());
    assert!(rehydrated.records.contains_key(&persisted[0].state));
    assert!(rehydrated.records[&persisted[0].state]
        .code_verifier
        .is_none());
    *state.oauth_states.write().await = rehydrated;

    let callback_path = format!(
        "/api/integrations/spotify/callback?code=abc123&state={}",
        persisted[0].state
    );
    let callback = super::route_http_request("GET", &callback_path, None, "", &state)
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
async fn spotify_oauth_state_is_not_consumed_when_persistence_delete_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    super::route_http_request(
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
    let callback = super::route_http_request("GET", &callback_path, None, "", &state)
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
async fn spotify_oauth_state_is_not_issued_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.oauth_states.write().await.records.insert(
        "expired".to_owned(),
        super::OAuthStateRecord {
            provider: "spotify".to_owned(),
            redirect_uri: "http://localhost/callback".to_owned(),
            code_verifier: None,
            created_at: 0,
            expires_at: 0,
        },
    );
    let previous = state.oauth_states.read().await.clone();
    db.close_for_test().await;

    let response = super::route_http_request(
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
async fn metrics_api_returns_scrapable_counters() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request("GET", "/api/v0/metrics", None, "", &state)
        .await
        .expect("metrics response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(
        response.content_type,
        "text/plain; version=0.0.4; charset=utf-8"
    );
    assert!(response.body.contains("slskr_session_connected 0"));
    assert!(response.body.contains("slskr_shares_files 1"));
    assert!(response.body.contains("slskr_shares_bytes 42"));
    assert!(response.body.contains("slskr_transfers{state=\"total\"} 0"));
    assert!(response
        .body
        .contains("slskr_transfers{state=\"active\"} 0"));
    assert!(response.body.contains("slskr_events_total 0"));
    assert!(response.body.contains("slskr_database_enabled 0"));
    assert!(response.body.contains("slskr_database_stats_available 0"));
    assert!(response
        .body
        .contains("slskr_database_rows{store=\"searches\"} 0"));
    assert!(response
        .body
        .contains("slskr_runtime_operations_total{operation=\"backfill\"} 0"));
    assert!(!response.body.contains("secret"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn persisted_http_logs_redact_stream_ticket_paths() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKD_HTTP_LOGGING", "true"));
    super::record_http_log(
        &state,
        "request-1",
        &super::logging::HttpTransactionLog {
            request: super::logging::HttpRequestLog {
                method: "GET".to_owned(),
                path: "/api/v0/peer-streams/bearer-ticket-secret".to_owned(),
                query: None,
                remote_addr: None,
                timestamp: "fixture".to_owned(),
            },
            response: super::logging::HttpResponseLog {
                status_code: 200,
                content_length: 0,
                duration_ms: 1,
                error: None,
            },
        },
    )
    .await;

    let events = state.events.read().await;
    let detail = events.records[0].detail.as_deref().expect("log detail");
    assert!(detail.contains("/api/v0/peer-streams/<redacted>"));
    assert!(!detail.contains("bearer-ticket-secret"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn metrics_distinguish_database_stats_failure_from_empty_database() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    let healthy = super::route_http_request("GET", "/api/metrics", None, "", &state)
        .await
        .expect("healthy metrics response");
    assert!(healthy.body.contains("slskr_database_enabled 1"));
    assert!(healthy.body.contains("slskr_database_stats_available 1"));

    db.close_for_test().await;
    let failed = super::route_http_request("GET", "/api/metrics", None, "", &state)
        .await
        .expect("failed database metrics response");
    assert!(failed.body.contains("slskr_database_enabled 1"));
    assert!(failed.body.contains("slskr_database_stats_available 0"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn capabilities_negotiate_returns_intersection() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request(
        "POST",
        "/api/v0/capabilities/negotiate",
        None,
        "{\"capabilities\":[\"shares\",\"telemetry\",\"bogus\"]}",
        &state,
    )
    .await
    .expect("capability negotiation response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "application/json");
    assert!(response
        .body
        .contains("\"accepted\":[\"shares\",\"telemetry\"]"));
    assert!(response.body.contains("\"unsupported\":[\"bogus\"]"));
    assert!(response.body.contains("\"server_capabilities\":["));
    assert!(!response.body.contains("secret"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn capabilities_parse_uses_real_tag_and_version_grammar() {
    let (state, _receiver) = test_state();

    let tagged = super::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"Client slskr_caps:v7;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1;partial=1"}"#,
        &state,
    )
    .await
    .expect("tag capability parse response");
    assert_eq!(tagged.status, "200 OK");
    let tagged_json = serde_json::from_str::<serde_json::Value>(&tagged.body).unwrap();
    assert_eq!(tagged_json["isSlskdn"], true);
    assert_eq!(tagged_json["flags"], "SupportsDHT, SupportsHashExchange, SupportsPartialDownload, SupportsMeshSync, SupportsFlacHashDb, SupportsSwarm");
    assert_eq!(tagged_json["flagsValue"], 63);
    assert_eq!(tagged_json["protocolVersion"], 7);
    assert_eq!(tagged_json["clientVersion"], "");
    assert_eq!(tagged_json["canSwarm"], true);
    assert_eq!(tagged_json["canMeshSync"], true);

    let version = super::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"ordinary client","versionString":"slskdn/2.4.1+dht+mesh+swarm"}"#,
        &state,
    )
    .await
    .expect("version capability parse response");
    let version_json = serde_json::from_str::<serde_json::Value>(&version.body).unwrap();
    assert_eq!(
        version_json["flags"],
        "SupportsDHT, SupportsMeshSync, SupportsSwarm"
    );
    assert_eq!(version_json["flagsValue"], 41);
    assert_eq!(version_json["protocolVersion"], 1);
    assert_eq!(version_json["clientVersion"], "2.4.1");

    let tag_takes_precedence = super::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"slskr_caps:v3;mesh=1","versionString":"slskdn/9.9+swarm"}"#,
        &state,
    )
    .await
    .expect("tag precedence response");
    let precedence_json =
        serde_json::from_str::<serde_json::Value>(&tag_takes_precedence.body).unwrap();
    assert_eq!(precedence_json["flagsValue"], 8);
    assert_eq!(precedence_json["protocolVersion"], 3);
    assert_eq!(precedence_json["clientVersion"], "");

    let false_positive = super::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"ordinary slskdn client"}"#,
        &state,
    )
    .await
    .expect("non-capability response");
    assert_eq!(false_positive.body, r#"{"isSlskdn":false}"#);

    let duplicate_flags = super::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"slskr_caps:v1;dht=1;dht=1"}"#,
        &state,
    )
    .await
    .expect("duplicate capability parse response");
    let duplicate_json = serde_json::from_str::<serde_json::Value>(&duplicate_flags.body).unwrap();
    assert_eq!(duplicate_json["flags"], "SupportsDHT");
    assert_eq!(duplicate_json["flagsValue"], 1);

    let no_flags = super::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"versionString":"slskdn/2.4.1"}"#,
        &state,
    )
    .await
    .expect("empty capability parse response");
    let no_flags_json = serde_json::from_str::<serde_json::Value>(&no_flags.body).unwrap();
    assert_eq!(no_flags_json["flags"], "None");
    assert_eq!(no_flags_json["flagsValue"], 0);
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
async fn controller_api_differential_incremental_transfer_and_message_routes_validate_cursors_and_bound_history(
) {
    let (state, _receiver) = test_state();
    {
        let mut transfers = state.transfers.write().await;
        for (filename, direction, status, updated_at_ms) in [
            ("old.flac", 0, "succeeded", 1_000),
            ("new.flac", 0, "succeeded", 3_000),
            ("active.flac", 0, "in_progress", 4_000),
            ("upload.flac", 1, "failed", 5_000),
        ] {
            let entry = transfers.create(
                direction,
                Some("peer".to_owned()),
                filename.to_owned(),
                None,
                Some(10),
            );
            let entry = transfers
                .entries
                .iter_mut()
                .find(|candidate| candidate.id == entry.id)
                .unwrap();
            entry.status = status.to_owned();
            entry.updated_at = updated_at_ms / 1_000;
            entry.updated_at_ms = updated_at_ms;
        }
    }

    let initial_response = super::route_http_request(
        "GET",
        "/api/v0/transfers/changes?includeCompleted=false",
        None,
        "",
        &state,
    )
    .await
    .expect("initial transfer changes");
    assert_eq!(initial_response.status, "200 OK");
    assert!(initial_response.content_type.contains("application/json"));
    let initial = serde_json::from_str::<serde_json::Value>(&initial_response.body).unwrap();
    assert_eq!(initial["counts"]["download"], 3);
    assert_eq!(initial["counts"]["upload"], 1);
    assert_eq!(initial["transfers"].as_array().unwrap().len(), 2);

    let changes =
        super::route_http_request("GET", "/api/transfers/changes?since=3500", None, "", &state)
            .await
            .expect("incremental transfer changes");
    let changes = serde_json::from_str::<serde_json::Value>(&changes.body).unwrap();
    let changed = changes["transfers"].as_array().unwrap();
    assert_eq!(changed.len(), 2);
    assert!(changed
        .iter()
        .any(|entry| entry["filename"] == "active.flac"));
    assert!(changed
        .iter()
        .any(|entry| entry["filename"] == "upload.flac"));

    let history_response = super::route_http_request(
        "GET",
        "/api/v0/transfers/history?direction=download&asOf=3500&offset=0&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("transfer history page");
    assert_eq!(history_response.status, "200 OK");
    assert!(history_response.content_type.contains("application/json"));
    let history = serde_json::from_str::<serde_json::Value>(&history_response.body).unwrap();
    assert_eq!(history["asOf"], 3_500);
    assert_eq!(history["hasMore"], true);
    assert_eq!(history["nextOffset"], 1);
    assert_eq!(history["transfers"][0]["filename"], "new.flac");
    let invalid_history = super::route_http_request(
        "GET",
        "/api/transfers/history?direction=download&limit=501",
        None,
        "",
        &state,
    )
    .await
    .expect("invalid transfer history");
    assert_eq!(invalid_history.status, "400 Bad Request");

    {
        let mut messages = state.messages.write().await;
        messages.add("friend".to_owned(), "inbound", "old".to_owned());
        messages.add("friend".to_owned(), "inbound", "new".to_owned());
        assert!(messages.records[1].created_at_ms > messages.records[0].created_at_ms);
        messages.records[0].created_at_ms = 1_000;
        messages.records[1].created_at_ms = 2_000;
    }
    let conversation = super::route_http_request(
        "GET",
        "/api/v0/conversations/friend?since=1500",
        None,
        "",
        &state,
    )
    .await
    .expect("incremental conversation");
    let conversation = serde_json::from_str::<serde_json::Value>(&conversation.body).unwrap();
    assert_eq!(conversation["unAcknowledgedMessageCount"], 2);
    assert_eq!(conversation["messages"].as_array().unwrap().len(), 1);
    assert_eq!(conversation["messages"][0]["message"], "new");
    assert_eq!(conversation["messages"][0]["createdAtMs"], 2_000);

    {
        let mut rooms = state.rooms.write().await;
        rooms.join("music".to_owned()).unwrap();
        rooms
            .add_message("music", "friend".to_owned(), "old".to_owned())
            .unwrap();
        rooms
            .add_message("music", "friend".to_owned(), "new".to_owned())
            .unwrap();
        let room = rooms
            .records
            .iter_mut()
            .find(|room| room.name == "music")
            .unwrap();
        assert!(room.messages[1].created_at_ms > room.messages[0].created_at_ms);
        room.messages[0].created_at_ms = 1_000;
        room.messages[1].created_at_ms = 2_000;
    }
    let room = super::route_http_request(
        "GET",
        "/api/v0/rooms/joined/music/messages?since=1500",
        None,
        "",
        &state,
    )
    .await
    .expect("incremental room messages");
    let room = serde_json::from_str::<serde_json::Value>(&room.body).unwrap();
    assert_eq!(room.as_array().unwrap().len(), 1);
    assert_eq!(room[0]["message"], "new");
    assert!(room[0]["id"].as_str().is_some());
    assert_eq!(room[0]["createdAtMs"], 2_000);

    for path in [
        "/api/transfers/changes?since=-1",
        "/api/conversations/friend?since=-1",
        "/api/rooms/joined/music/messages?since=-1",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("negative cursor response");
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/transfers/changes",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/transfers/history",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("incremental_transfer_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn swarm_analytics_routes_share_a_bounded_snapshot() {
    let (state, _receiver) = test_state();
    let now = super::unix_timestamp();
    state
        .multisource
        .write()
        .await
        .insert(super::multisource::SwarmJob {
            id: "swarm-analytics-fixture".to_owned(),
            status: "completed".to_owned(),
            filename: "album.flac".to_owned(),
            output_path: "album.flac".to_owned(),
            file_size: 1_024,
            chunk_size: 512,
            sources: vec!["alice".to_owned(), "bob".to_owned()],
            completed_chunks: 2,
            total_chunks: 2,
            bytes_downloaded: 1_024,
            created_at: now,
            updated_at: now,
            result: Some(super::multisource::SwarmResult {
                id: "swarm-analytics-fixture".to_owned(),
                success: true,
                filename: "album.flac".to_owned(),
                output_path: "album.flac".to_owned(),
                bytes_downloaded: 1_024,
                total_time_ms: 100,
                sources_used: 2,
                final_hash: "00".repeat(32),
                chunks: vec![
                    super::multisource::ChunkResult {
                        index: 0,
                        username: "alice".to_owned(),
                        start_offset: 0,
                        end_offset: 511,
                        bytes_downloaded: 512,
                        time_ms: 40,
                    },
                    super::multisource::ChunkResult {
                        index: 1,
                        username: "bob".to_owned(),
                        start_offset: 512,
                        end_offset: 1_023,
                        bytes_downloaded: 512,
                        time_ms: 60,
                    },
                ],
                error: None,
            }),
        });

    let dashboard = super::route_http_request(
        "GET",
        "/api/v0/swarm/analytics/dashboard?timeWindowHours=24&rankingLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("swarm analytics dashboard");
    assert_eq!(dashboard.status, "200 OK");
    let dashboard = serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap();
    assert_eq!(dashboard["performanceMetrics"]["totalDownloads"], 1);
    assert_eq!(dashboard["performanceMetrics"]["successRate"], 1.0);
    assert_eq!(dashboard["performanceMetrics"]["timeWindow"], "1.00:00:00");
    assert_eq!(dashboard["peerRankings"].as_array().unwrap().len(), 1);
    assert_eq!(dashboard["peerRankings"][0]["rank"], 1);
    assert!(dashboard["efficiencyMetrics"].is_object());
    assert!(dashboard["recommendations"].is_array());

    for (path, expected_kind) in [
        ("/api/swarm/analytics/performance", "object"),
        ("/api/swarm/analytics/peers/rankings?limit=2", "array"),
        ("/api/swarm/analytics/efficiency", "object"),
        ("/api/swarm/analytics/recommendations", "array"),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("swarm analytics projection");
        assert_eq!(response.status, "200 OK", "{path}");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(
            if value.is_array() { "array" } else { "object" },
            expected_kind,
            "{path}"
        );
    }

    let trends = super::route_http_request(
        "GET",
        "/api/swarm/analytics/trends?timeWindowHours=48&dataPoints=12",
        None,
        "",
        &state,
    )
    .await
    .expect("swarm analytics trends");
    let trends = serde_json::from_str::<serde_json::Value>(&trends.body).unwrap();
    assert_eq!(trends["timePoints"], serde_json::json!([]));

    for path in [
        "/api/swarm/analytics/dashboard?timeWindowHours=0",
        "/api/swarm/analytics/dashboard?rankingLimit=101",
        "/api/swarm/analytics/peers/rankings?limit=0",
        "/api/swarm/analytics/trends?dataPoints=1",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("invalid swarm analytics query");
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn port_forwarding_reads_are_bounded_and_start_requires_an_authorized_pinned_gateway() {
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));

    let status =
        super::route_http_request("GET", "/api/v0/port-forwarding/status", None, "", &state)
            .await
            .expect("port forwarding status");
    assert_eq!(status.status, "200 OK");
    assert_eq!(status.body, "[]");

    let available = super::route_http_request(
        "GET",
        "/api/v0/port-forwarding/available-ports?startPort=2000&endPort=2010&limit=3",
        None,
        "",
        &state,
    )
    .await
    .expect("available port page");
    let available = serde_json::from_str::<serde_json::Value>(&available.body).unwrap();
    assert_eq!(available["availablePortCount"], 11);
    assert_eq!(available["usedPortCount"], 0);
    assert_eq!(
        available["availablePorts"],
        serde_json::json!([2000, 2001, 2002])
    );

    let stats =
        super::route_http_request("GET", "/api/port-forwarding/stream-stats", None, "", &state)
            .await
            .expect("port forwarding stream stats");
    let stats = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats["totalForwardingRules"], 0);
    assert_eq!(stats["rules"], serde_json::json!([]));

    let default_page = super::route_http_request(
        "GET",
        "/api/port-forwarding/available-ports",
        None,
        "",
        &state,
    )
    .await
    .expect("default available port page");
    let default_page = serde_json::from_str::<serde_json::Value>(&default_page.body).unwrap();
    assert_eq!(
        default_page["availablePorts"].as_array().unwrap().len(),
        1_000
    );

    for path in [
        "/api/port-forwarding/available-ports?startPort=0",
        "/api/port-forwarding/available-ports?startPort=3000&endPort=2000",
        "/api/port-forwarding/available-ports?limit=0",
        "/api/port-forwarding/available-ports?limit=1001",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("invalid port forwarding query");
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }

    let missing =
        super::route_http_request("GET", "/api/port-forwarding/status/2000", None, "", &state)
            .await
            .expect("missing port forwarding status");
    assert_eq!(missing.status, "404 Not Found");

    let start = super::route_http_request(
        "POST",
        "/api/v0/port-forwarding/start",
        None,
        r#"{"localPort":2000,"podId":"pod-1","destinationHost":"service","destinationPort":80}"#,
        &state,
    )
    .await
    .expect("unsupported port forwarding start");
    assert_eq!(start.status, "403 Forbidden");

    let pin = "07".repeat(32);
    let create = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-forward","name":"Forward","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &state,
    )
    .await
    .expect("create gateway pod");
    assert_eq!(create.status, "201 Created", "{}", create.body);
    let mut gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    gateway.peer_id = "tester".to_owned();
    state.mesh.write().await.capability_records.push(gateway);
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);
    let start = super::route_http_request(
        "POST",
        "/api/v0/port-forwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start port forwarding");
    assert_eq!(start.status, "200 OK", "{}", start.body);
    let status = super::route_http_request(
        "GET",
        &format!("/api/v0/port-forwarding/status/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forwarding status");
    assert_eq!(status.status, "200 OK");
    assert!(status.body.contains("pod-forward"));

    let stop = super::route_http_request(
        "POST",
        &format!("/api/port-forwarding/stop/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("port forwarding stop");
    assert_eq!(stop.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn port_forwarding_uses_operator_pinned_gateway_when_frozen_pod_omits_pin() {
    let trusted_peers = serde_json::json!([{
        "peerId": "tester",
        "username": "gateway",
        "overlayEndpoint": "127.0.0.1:50305",
        "certificateSha256": "07".repeat(32)
    }]);
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &trusted_peers.to_string()),
    );
    let create = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-trusted-forward","name":"Trusted Forward","capabilities":[0],"privateServicePolicy":{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","registeredServices":[],"allowedDestinations":[{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}]}}}"#,
        &state,
    )
    .await
    .expect("create frozen gateway pod without a certificate field");
    assert_eq!(create.status, "201 Created", "{}", create.body);

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);
    let start = super::route_http_request(
        "POST",
        "/api/v0/port-forwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-trusted-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start operator-pinned port forwarding");
    assert_eq!(start.status, "200 OK", "{}", start.body);

    let stop = super::route_http_request(
        "POST",
        &format!("/api/port-forwarding/stop/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("stop operator-pinned port forwarding");
    assert_eq!(stop.status, "200 OK");
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
async fn controller_api_differential_portforwarding_start_readback() {
    // The oracle's real route (ASP.NET's default [controller] token for
    // PortForwardingController has no hyphen) is
    // "api/v0/portforwarding/start". A routing-table typo previously
    // mapped that exact path to an internal literal that only a
    // fake-success stub matched -- any real caller using the real
    // oracle-spelled path always got a canned "Port forwarding
    // started" message with none of the real handler's pod-membership/
    // gateway-pinning validation ever running and no forward ever
    // actually starting. This proves the fixed routing table now
    // reaches the same real handler already exercised (via the
    // internal hyphenated spelling) by the sibling tests above.
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));
    let pin = "07".repeat(32);
    let create = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-oracle-forward","name":"Oracle Forward","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &state,
    )
    .await
    .expect("create gateway pod");
    assert_eq!(create.status, "201 Created", "{}", create.body);
    let mut gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    gateway.peer_id = "tester".to_owned();
    state.mesh.write().await.capability_records.push(gateway);
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);

    let start = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-oracle-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start port forwarding via the oracle-spelled route");
    assert_eq!(start.status, "200 OK", "{}", start.body);
    let status = super::route_http_request(
        "GET",
        &format!("/api/v0/port-forwarding/status/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forwarding status");
    assert_eq!(status.status, "200 OK");
    assert!(status.body.contains("pod-oracle-forward"));

    let stop = super::route_http_request(
        "POST",
        &format!("/api/port-forwarding/stop/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("stop port forwarding");
    assert_eq!(stop.status, "200 OK");

    let ledger = [serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/portforwarding/start",
        "case": "mutation-side-effects-and-readback",
        "pass": true,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("portforwarding_start_readback.json"),
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
async fn controller_api_differential_overlay_gateway_populated_gets() {
    use sha2::Digest as _;
    use slskr_client::overlay::{
        CloseTunnelRequest, GetTunnelDataRequest, MeshHello, MeshServiceCall, OpenTunnelRequest,
        OpenTunnelResponse, TunnelDataRequest, TunnelDataResponse, FEATURE_MESH_SERVICE,
    };

    let root = std::env::temp_dir().join(format!(
        "slskr-private-gateway-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("gateway state directory");
    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": true,
            "enableDht": true,
        },
        "feature": {
            "mesh": true,
            "pods": true,
            "virtualSoulfind": true,
        }
    });
    let (mut state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string())
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "member=127.0.0.1:1"),
    );
    let gateway = Arc::new(
        super::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    Arc::get_mut(&mut state)
        .expect("unshared state")
        .private_gateway = Some(gateway.clone());
    Arc::get_mut(&mut state)
        .expect("unshared state")
        .config
        .trusted_mesh_peers
        .push(super::TrustedMeshPeer {
            peer_id: "tester".to_owned(),
            username: "tester".to_owned(),
            overlay_endpoint: endpoint,
            certificate_sha256: certificate_pin,
            range_endpoint: None,
        });
    let mesh_content = b"frozen mesh content bytes";
    let mesh_content_path = root.join("mesh-content.flac");
    std::fs::write(&mesh_content_path, mesh_content).expect("write mesh content fixture");
    add_test_share(
        &state,
        "Virtual/MeshContent.flac",
        &mesh_content_path,
        mesh_content.len() as u64,
    )
    .await;
    let mesh_content_hash = hex::encode(sha2::Sha256::digest(mesh_content));
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "mesh-content-key".to_owned(),
                file_sha256: mesh_content_hash,
                size: mesh_content.len() as u64,
                music_brainz_id: "recording-1".to_owned(),
                ..Default::default()
            }])
            .expect("merge mesh content hash");
        discovery
            .merge_shadow_records(vec![super::content_discovery::ShadowIndexRecord {
                recording_id: "recording-1".to_owned(),
                peer_ids: vec!["peer-hint".to_owned()],
                updated_at: 0,
            }])
            .expect("merge shadow index fixture");
    }

    let echo_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("echo listener");
    let echo_port = echo_listener.local_addr().unwrap().port();
    let echo = tokio::spawn(async move {
        let (mut stream, _) = echo_listener.accept().await.expect("echo accept");
        let mut request = [0_u8; 4];
        tokio::io::AsyncReadExt::read_exact(&mut stream, &mut request)
            .await
            .expect("echo read");
        assert_eq!(&request, b"ping");
        tokio::io::AsyncWriteExt::write_all(&mut stream, b"pong")
            .await
            .expect("echo write");
        let _ = echo_listener.accept().await.expect("legacy echo accept");
    });

    let create = super::route_http_request(
        "POST",
        "/api/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-gateway","name":"Gateway","isPublic":true,"requireApproval":false,"channels":[{{"channelId":"general","kind":0,"name":"General"}}],"capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"127.0.0.1","port":{},"protocol":"tcp","allowPublic":false}}]}}}}}}"#,
            hex::encode(certificate_pin),
            echo_port
        ),
        &state,
    )
    .await
    .expect("create gateway pod");
    assert_eq!(create.status, "201 Created", "{}", create.body);
    *state.runtime_credentials.write().await =
        Some(super::LoginCredentials::default_client("member", "secret"));
    let join = super::route_http_request("POST", "/api/pods/pod-gateway/join", None, "{}", &state)
        .await
        .expect("join gateway pod");
    assert_eq!(join.status, "200 OK", "{}", join.body);
    *state.runtime_credentials.write().await = None;

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "member",
        vec!["mesh_sync".to_owned()],
        Vec::new(),
        std::time::Duration::from_secs(300),
        &remote_key,
        std::time::SystemTime::now(),
    )
    .map(|descriptor| descriptor.with_overlay_port(Some(endpoint.port())))
    .and_then(|descriptor| descriptor.sign(&remote_key))
    .expect("member capability");
    state
        .mesh
        .write()
        .await
        .update_capability(descriptor)
        .expect("register member capability");
    let local_descriptor = super::local_capability_descriptor(&state)
        .await
        .expect("local capability descriptor");
    state
        .mesh
        .write()
        .await
        .update_capability(local_descriptor)
        .expect("register local capability");
    super::remember_peer_endpoint(
        &state,
        super::PeerAddress {
            username: "tester".to_owned(),
            ip: std::net::Ipv4Addr::LOCALHOST,
            port: 1,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
    let routed_message_id = super::route_pod_message_to_peer(
        &state,
        &serde_json::json!({
            "messageId": "routed-message",
            "podId": "pod-gateway",
            "channelId": "general",
            "body": "routed through trusted mesh",
            "signature": "",
        }),
        "tester",
    )
    .await
    .expect("trusted mesh pod route");
    assert!(!routed_message_id.is_empty());

    let mut hello = MeshHello::new(
        "member",
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        "gateway-test-nonce",
    )
    .expect("hello");
    hello
        .authenticate(&remote_key, &certificate_pin)
        .expect("authenticate hello");
    let mut client = slskr_client::overlay::connect_tls_overlay(endpoint, certificate_pin, hello)
        .await
        .expect("connect gateway");
    assert_eq!(client.remote_username, "tester");

    let pods_reply = client
        .call(&MeshServiceCall::new("pods-list", "pods", "List", Vec::new()).unwrap())
        .await
        .expect("pods list reply");
    assert_eq!(pods_reply.status_code, 0, "{:?}", pods_reply.error_message);
    let pod_list = serde_json::from_slice::<serde_json::Value>(&pods_reply.payload).unwrap();
    assert_eq!(pod_list[0]["podId"], "pod-gateway");

    let post_reply = client
        .call(
            &MeshServiceCall::new(
                "pods-post",
                "pods",
                "PostMessage",
                br#"{"PodId":"pod-gateway","ChannelId":"general","Body":"mesh hello"}"#.to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("pods post reply");
    assert_eq!(post_reply.status_code, 0, "{:?}", post_reply.error_message);
    let messages_reply = client
        .call(
            &MeshServiceCall::new(
                "pods-messages",
                "pods",
                "GetMessages",
                br#"{"PodId":"pod-gateway","ChannelId":"general"}"#.to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("pods messages reply");
    assert_eq!(
        messages_reply.status_code, 0,
        "{:?}",
        messages_reply.error_message
    );
    let messages = serde_json::from_slice::<serde_json::Value>(&messages_reply.payload).unwrap();
    assert!(messages
        .as_array()
        .unwrap()
        .iter()
        .any(|message| message["senderPeerId"] == "member" && message["body"] == "mesh hello"));
    assert!(messages
        .as_array()
        .unwrap()
        .iter()
        .any(|message| message["senderPeerId"] == "tester"
            && message["body"] == "routed through trusted mesh"));

    let shadow_reply = client
        .call(
            &MeshServiceCall::new(
                "shadow-query",
                "shadow-index",
                "QueryByMbid",
                br#"{"MBID":"recording-1"}"#.to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("shadow-index reply");
    assert_eq!(
        shadow_reply.status_code, 0,
        "{:?}",
        shadow_reply.error_message
    );
    let shadow = serde_json::from_slice::<serde_json::Value>(&shadow_reply.payload).unwrap();
    assert_eq!(shadow["MBID"], "recording-1");
    assert_eq!(shadow["PeerCount"], 1);
    assert_eq!(
        shadow["CanonicalVariants"][0]["SizeBytes"],
        mesh_content.len()
    );

    let content_reply = client
        .call(
            &MeshServiceCall::new(
                "content-range",
                "MeshContent",
                "GetByContentId",
                br#"{"contentId":"Virtual/MeshContent.flac","range":{"offset":7,"length":4}}"#
                    .to_vec(),
            )
            .unwrap(),
        )
        .await
        .expect("mesh content reply");
    assert_eq!(
        content_reply.status_code, 0,
        "{:?}",
        content_reply.error_message
    );
    assert_eq!(content_reply.payload, b"mesh");

    let open = OpenTunnelRequest::new(
        "pod-gateway",
        "127.0.0.1",
        echo_port,
        None,
        "open-tunnel-nonce",
    )
    .expect("open request");
    let reply = client
        .call(
            &MeshServiceCall::new(
                "open",
                "private-gateway",
                "OpenTunnel",
                serde_json::to_vec(&open).unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("open reply");
    assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);
    let opened: OpenTunnelResponse = serde_json::from_slice(&reply.payload).unwrap();

    // The frozen MeshController transport projection reads active DHT
    // and overlay session counts from live services.  This open tunnel
    // is the real populated overlay state used by the following checks.
    let mesh_transport =
        super::route_http_request("GET", "/api/v0/mesh/transport", None, "", &state)
            .await
            .expect("mesh transport with an open overlay session");
    let mesh_transport_json =
        serde_json::from_str::<serde_json::Value>(&mesh_transport.body).unwrap();
    let mesh_transport_keys = mesh_transport_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    assert_eq!(mesh_transport.status, "200 OK");
    assert_eq!(mesh_transport.content_type, "application/json");
    assert_eq!(
        mesh_transport_keys,
        BTreeSet::from(["dht".to_owned(), "natType".to_owned(), "overlay".to_owned(),])
    );
    assert_eq!(mesh_transport_json["dht"], 0);
    assert_eq!(mesh_transport_json["overlay"], 1);
    assert_eq!(mesh_transport_json["natType"], "Unknown");

    // Matches the oracle's real ServerStatsResponse.ActiveConnections:
    // must reflect this real, currently-open tunnel, not a hardcoded
    // 0 regardless of live connection state.
    let overlay_stats = super::route_http_request("GET", "/api/v0/overlay/stats", None, "", &state)
        .await
        .expect("overlay stats with an open tunnel");
    let overlay_stats_json =
        serde_json::from_str::<serde_json::Value>(&overlay_stats.body).unwrap();
    assert_eq!(
        overlay_stats_json["activeConnections"], 1,
        "{overlay_stats_json}"
    );
    assert_eq!(
        overlay_stats_json["server"]["activeConnections"], 1,
        "{overlay_stats_json}"
    );

    // Matches the oracle's real overlay/mesh session list -- report the
    // authenticated TLS session, not tunnel ownership state.
    let overlay_connections =
        super::route_http_request("GET", "/api/v0/overlay/connections", None, "", &state)
            .await
            .expect("overlay connections with an open tunnel");
    let overlay_connections_json =
        serde_json::from_str::<serde_json::Value>(&overlay_connections.body).unwrap();
    assert_eq!(overlay_connections_json.as_array().unwrap().len(), 1);
    let connection = &overlay_connections_json[0];
    assert_eq!(connection["username"], "member");
    assert_eq!(connection["address"], "127.0.0.1");
    assert!(connection["port"].as_u64().is_some_and(|port| port > 0));
    assert_eq!(
        connection["features"],
        serde_json::json!([FEATURE_MESH_SERVICE])
    );
    assert!(connection["connectedAt"]
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some());
    assert!(connection["lastActivity"]
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some());
    assert!(connection["certificateThumbprint"].is_null());
    assert_eq!(connection["version"], 1);
    assert_eq!(connection["isOutbound"], false);

    let reply = client
        .call(
            &MeshServiceCall::new(
                "send",
                "private-gateway",
                "TunnelData",
                serde_json::to_vec(&TunnelDataRequest {
                    tunnel_id: opened.tunnel_id.clone(),
                    data: b"ping".to_vec(),
                })
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("send reply");
    assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);

    let received = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let reply = client
                .call(
                    &MeshServiceCall::new(
                        uuid::Uuid::new_v4().to_string(),
                        "private-gateway",
                        "GetTunnelData",
                        serde_json::to_vec(&GetTunnelDataRequest {
                            tunnel_id: opened.tunnel_id.clone(),
                        })
                        .unwrap(),
                    )
                    .unwrap(),
                )
                .await
                .expect("receive reply");
            assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);
            let response: TunnelDataResponse = serde_json::from_slice(&reply.payload).unwrap();
            if !response.data.is_empty() {
                break response.data;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("receive timeout");
    assert_eq!(received, b"pong");

    let reply = client
        .call(
            &MeshServiceCall::new(
                "close",
                "private-gateway",
                "CloseTunnel",
                serde_json::to_vec(&CloseTunnelRequest {
                    tunnel_id: opened.tunnel_id,
                })
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("close reply");
    assert_eq!(reply.status_code, 0, "{:?}", reply.error_message);

    let overlay_stats_after_close =
        super::route_http_request("GET", "/api/v0/overlay/stats", None, "", &state)
            .await
            .expect("overlay stats after closing the tunnel");
    let overlay_stats_after_close_json =
        serde_json::from_str::<serde_json::Value>(&overlay_stats_after_close.body).unwrap();
    assert_eq!(
        overlay_stats_after_close_json["activeConnections"], 0,
        "{overlay_stats_after_close_json}"
    );
    let overlay_connections_after_close =
        super::route_http_request("GET", "/api/v0/overlay/connections", None, "", &state)
            .await
            .expect("overlay connections after closing the tunnel");
    let overlay_connections_after_close_json =
        serde_json::from_str::<serde_json::Value>(&overlay_connections_after_close.body).unwrap();
    assert_eq!(
        overlay_connections_after_close_json[0]["username"], "member",
        "closing a tunnel must not remove its still-open overlay session"
    );

    let mut client_stream = client.into_inner();
    tokio::io::AsyncWriteExt::shutdown(&mut client_stream)
        .await
        .expect("close overlay client connection");
    let overlay_connections_after_client_close =
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let response = super::route_http_request(
                    "GET",
                    "/api/v0/overlay/connections",
                    None,
                    "",
                    &state,
                )
                .await
                .expect("overlay connections after client close");
                let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
                if value.as_array().is_some_and(Vec::is_empty) {
                    break value;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("overlay connection cleanup timeout");
    assert_eq!(
        overlay_connections_after_client_close,
        serde_json::json!([])
    );

    let mut second_hello = MeshHello::new(
        "member",
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        "second-gateway-test-nonce",
    )
    .expect("second hello");
    second_hello
        .authenticate(&remote_key, &certificate_pin)
        .expect("authenticate second hello");
    let mut second_client =
        slskr_client::overlay::connect_tls_overlay(endpoint, certificate_pin, second_hello)
            .await
            .expect("connect second authenticated gateway client");
    let second_reply = second_client
        .call(
            &MeshServiceCall::new(
                "legacy-open",
                "private-gateway",
                "OpenTunnel",
                serde_json::to_vec(
                    &OpenTunnelRequest::new(
                        "pod-gateway",
                        "127.0.0.1",
                        echo_port,
                        None,
                        "legacy-open-nonce",
                    )
                    .unwrap(),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .await
        .expect("second gateway reply");
    assert_eq!(
        second_reply.status_code, 0,
        "{:?}",
        second_reply.error_message
    );
    echo.await.expect("echo task");

    let ledger = [
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/overlay/stats",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/overlay/connections",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/mesh/transport",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("overlay_gateway_populated_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    gateway_server.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_management_routes_persist_crud_members_and_bindings() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod:api","name":"API Pod","isPublic":true,"maxMembers":4,"tags":["music"],"channels":[{"channelId":"general","kind":0,"name":"General"}]},"requestingPeerId":"ignored-by-auth"}"#,
        &state,
    )
    .await
    .expect("create pod");
    assert_eq!(created.status, "201 Created");
    let created = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created["podId"], "pod:api");
    assert_eq!(created["name"], "API Pod");
    assert!(created.get("members").is_none());

    let listed = super::route_http_request("GET", "/api/pods", None, "", &state)
        .await
        .expect("list pods");
    let listed = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["podId"], "pod:api");

    let detail = super::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("pod detail");
    assert_eq!(detail.status, "200 OK");

    let members = super::route_http_request("GET", "/api/pods/pod%3Aapi/members", None, "", &state)
        .await
        .expect("pod members");
    let members = serde_json::from_str::<serde_json::Value>(&members.body).unwrap();
    assert_eq!(members.as_array().unwrap().len(), 1);
    assert_eq!(members[0]["peerId"], "tester");
    assert_eq!(members[0]["role"], "owner");

    *state.runtime_credentials.write().await =
        Some(super::LoginCredentials::default_client("member", "secret"));
    let joined = super::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/join",
        None,
        r#"{"peerId":"member"}"#,
        &state,
    )
    .await
    .expect("join pod");
    assert_eq!(joined.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&joined.body).unwrap()["joined"],
        true
    );
    *state.runtime_credentials.write().await = None;

    let bound = super::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/channels/general/bind",
        None,
        r#"{"roomName":"ambient","mode":"mirror"}"#,
        &state,
    )
    .await
    .expect("bind pod channel");
    assert_eq!(bound.status, "200 OK");
    let detail = super::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("bound pod detail");
    let detail = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap();
    assert_eq!(
        detail["channels"][0]["bindingInfo"],
        "soulseek-room:ambient"
    );

    let banned = super::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/ban",
        None,
        r#"{"peerId":"member"}"#,
        &state,
    )
    .await
    .expect("ban pod member");
    assert_eq!(banned.status, "200 OK");
    let members = super::route_http_request("GET", "/api/pods/pod%3Aapi/members", None, "", &state)
        .await
        .expect("members after ban");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&members.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let updated = super::route_http_request(
        "PUT",
        "/api/pods/pod%3Aapi",
        None,
        r#"{"pod":{"podId":"pod:api","name":"Renamed Pod","isPublic":true,"maxMembers":4,"channels":[{"channelId":"general","kind":0,"name":"General"}]}}"#,
        &state,
    )
    .await
    .expect("update pod");
    assert_eq!(updated.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.body).unwrap()["name"],
        "Renamed Pod"
    );

    let message = super::route_http_request(
        "POST",
        "/api/v0/pods/pod%3Aapi/channels/general/messages",
        None,
        r#"{"body":"delete me","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("pod message before delete");
    assert_eq!(message.status, "200 OK");

    let deleted = super::route_http_request("DELETE", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("delete pod");
    assert_eq!(deleted.status, "204 No Content");
    let missing = super::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("deleted pod");
    assert_eq!(missing.status, "404 Not Found");
    assert!(state
        .pod_channels
        .read()
        .await
        .list("pod:api", "general", None)
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_channel_messages_are_durable_shaped_and_incremental() {
    let (state, _receiver) = test_state();
    let path = "/api/v0/pods/pod-1/channels/general/messages";
    let created = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-1","name":"Pod One","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"}]},"requestingPeerId":"peer-1"}"#,
        &state,
    )
    .await
    .expect("create pod for messages");
    assert_eq!(created.status, "201 Created");

    state
        .pods
        .write()
        .await
        .join("pod-1", "peer-1".to_owned())
        .unwrap();
    let spoofed = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"spoofed","senderPeerId":"peer-1"}"#,
        &state,
    )
    .await
    .expect("spoofed pod message");
    assert_eq!(spoofed.status, "403 Forbidden");

    *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
        "public-intruder",
        "secret",
    ));
    let forbidden = super::route_http_request("GET", path, None, "", &state)
        .await
        .expect("public pod history still requires membership");
    assert_eq!(forbidden.status, "403 Forbidden");
    *state.runtime_credentials.write().await = None;

    let first = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"first","senderPeerId":"tester","signature":"sig"}"#,
        &state,
    )
    .await
    .expect("first pod message");
    assert_eq!(first.status, "200 OK");
    let first = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    assert_eq!(first["sent"], true);
    assert_eq!(first["messageId"].as_str().unwrap().len(), 32);

    let initial = super::route_http_request("GET", path, None, "", &state)
        .await
        .expect("initial pod messages");
    let initial = serde_json::from_str::<serde_json::Value>(&initial.body).unwrap();
    assert_eq!(initial.as_array().unwrap().len(), 1);
    assert_eq!(initial[0]["podId"], "pod-1");
    assert_eq!(initial[0]["channelId"], "general");
    assert_eq!(initial[0]["senderPeerId"], "tester");
    assert_eq!(initial[0]["body"], "first");
    assert_eq!(initial[0]["sigVersion"], 1);
    let cursor = initial[0]["timestampUnixMs"].as_u64().unwrap();

    let second = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"second","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("second pod message");
    assert_eq!(second.status, "200 OK");
    {
        let mut channels = state.pod_channels.write().await;
        let latest = channels.list("pod-1", "general", None).pop().unwrap();
        if latest.timestamp_unix_ms == cursor {
            channels
                .append(
                    "pod-1".to_owned(),
                    "general".to_owned(),
                    "tester".to_owned(),
                    "third".to_owned(),
                    String::new(),
                    cursor + 1,
                )
                .unwrap();
        }
    }
    let incremental =
        super::route_http_request("GET", &format!("{path}?since={cursor}"), None, "", &state)
            .await
            .expect("incremental pod messages");
    let incremental = serde_json::from_str::<serde_json::Value>(&incremental.body).unwrap();
    assert!(!incremental.as_array().unwrap().is_empty());
    assert!(incremental
        .as_array()
        .unwrap()
        .iter()
        .all(|message| message["timestampUnixMs"].as_u64().unwrap() > cursor));

    let invalid = super::route_http_request("GET", &format!("{path}?since=-1"), None, "", &state)
        .await
        .expect("invalid pod message cursor");
    assert_eq!(invalid.status, "400 Bad Request");

    let invalid = super::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("invalid pod message");
    assert_eq!(invalid.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_channel_bindings_bridge_room_and_private_messages() {
    let (state, mut receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/pods",
        None,
        r#"{"pod":{"podId":"pod-bridge","name":"Bridge","isPublic":true,"channels":[{"channelId":"room","kind":0,"name":"Room"},{"channelId":"dm","kind":1,"name":"DM","bindingInfo":"soulseek-dm:bob"}]}}"#,
        &state,
    )
    .await
    .expect("create bridge pod");
    assert_eq!(created.status, "201 Created");

    let bound = super::route_http_request(
        "POST",
        "/api/pods/pod-bridge/channels/room/bind",
        None,
        r#"{"roomName":"ambient","mode":"mirror"}"#,
        &state,
    )
    .await
    .expect("bind bridge room");
    assert_eq!(bound.status, "200 OK");
    assert_eq!(
        receiver.recv().await,
        Some(super::SessionCommand::JoinRoom("ambient".to_owned()))
    );

    let sent = super::route_http_request(
        "POST",
        "/api/pods/pod-bridge/channels/room/messages",
        None,
        r#"{"body":"from pod","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("send mirrored pod message");
    assert_eq!(sent.status, "200 OK");
    assert_eq!(
        receiver.recv().await,
        Some(super::SessionCommand::SayRoom {
            room: "ambient".to_owned(),
            body: "[Pod:tester] from pod".to_owned(),
        })
    );

    super::bridge_soulseek_room_message_to_pods(&state, "AMBIENT", "alice", "from room").await;
    let room_history = super::route_http_request(
        "GET",
        "/api/pods/pod-bridge/channels/room/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("read bridged room history");
    let room_history = serde_json::from_str::<serde_json::Value>(&room_history.body).unwrap();
    assert!(room_history.as_array().unwrap().iter().any(|message| {
        message["senderPeerId"] == "bridge:alice" && message["body"] == "[Soulseek:alice] from room"
    }));

    let inbound = super::route_http_request(
        "POST",
        "/api/messages/inbound",
        None,
        r#"{"username":"bob","body":"from dm"}"#,
        &state,
    )
    .await
    .expect("record inbound private message");
    assert_eq!(inbound.status, "201 Created");
    let dm_history = super::route_http_request(
        "GET",
        "/api/pods/pod-bridge/channels/dm/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("read bridged dm history");
    let dm_history = serde_json::from_str::<serde_json::Value>(&dm_history.body).unwrap();
    assert_eq!(dm_history[0]["senderPeerId"], "bridge:bob");
    assert_eq!(dm_history[0]["body"], "from dm");

    let dm_sent = super::route_http_request(
        "POST",
        "/api/pods/pod-bridge/channels/dm/messages",
        None,
        r#"{"body":"to dm","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("send bridged dm");
    assert_eq!(dm_sent.status, "200 OK");
    assert_eq!(
        receiver.recv().await,
        Some(super::SessionCommand::MessageUser {
            username: "bob".to_owned(),
            body: "to dm".to_owned(),
        })
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn private_pod_message_history_requires_membership() {
    let (state, _receiver) = test_state();
    let path = "/api/v0/pods/private-pod/channels/general/messages";
    let created = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"private-pod","name":"Private","isPublic":false,"channels":[{"channelId":"general","kind":0,"name":"General"}]}}"#,
        &state,
    )
    .await
    .expect("create private pod");
    assert_eq!(created.status, "201 Created");

    *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
        "intruder", "secret",
    ));
    let forbidden = super::route_http_request("GET", path, None, "", &state)
        .await
        .expect("private pod history");
    assert_eq!(forbidden.status, "403 Forbidden");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn inbound_soulseek_pod_messages_are_validated_and_stored() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000001";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Inbound Pod",
                "isPublic": true,
                "channels": [{
                    "channelId": "general",
                    "kind": 0,
                    "name": "General"
                }]
            }))
            .expect("deserialize inbound pod fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create inbound pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "bridge:alice".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add inbound pod member");
    state
        .advanced_networking
        .write()
        .await
        .pod_security_signature_mode = super::PodSignatureMode::Off;

    let timestamp = super::unix_timestamp_millis();
    let payload = serde_json::json!({
        "MessageId": "550e8400-e29b-41d4-a716-446655440000",
        "PodId": pod_id,
        "ChannelId": "general",
        "SenderPeerId": "bridge:alice",
        "Body": "hello from the bridge",
        "TimestampUnixMs": timestamp,
        "SigVersion": 1,
        "Signature": ""
    });
    assert!(
        super::handle_incoming_soulseek_pod_message(&state, "alice", &format!("PODMSG:{payload}"),)
            .await
    );
    assert!(
        super::handle_incoming_soulseek_pod_message(&state, "alice", &format!("PODMSG:{payload}"),)
            .await
    );
    assert!(!super::handle_incoming_soulseek_pod_message(&state, "alice", "ordinary PM").await);

    let messages = state
        .pod_channels
        .read()
        .await
        .list(pod_id, "general", None);
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].message_id,
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(messages[0].sender_peer_id, "bridge:alice");
    assert_eq!(messages[0].body, "hello from the bridge");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn removing_a_pod_channel_permanently_removes_its_message_history() {
    let (state, _receiver) = test_state();
    let created = super::route_http_request(
        "POST",
        "/api/pods",
        None,
        r#"{"pod":{"podId":"pod-cleanup","name":"Cleanup","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"},{"channelId":"private","kind":0,"name":"Private"}]}}"#,
        &state,
    )
    .await
    .expect("create pod");
    assert_eq!(created.status, "201 Created");
    let message_path = "/api/pods/pod-cleanup/channels/private/messages";
    let sent = super::route_http_request(
        "POST",
        message_path,
        None,
        r#"{"body":"must not resurface","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("send channel message");
    assert_eq!(sent.status, "200 OK");

    let removed = super::route_http_request(
        "PUT",
        "/api/pods/pod-cleanup",
        None,
        r#"{"pod":{"podId":"pod-cleanup","name":"Cleanup","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"}]}}"#,
        &state,
    )
    .await
    .expect("remove channel");
    assert_eq!(removed.status, "200 OK");
    assert!(state
        .pod_channels
        .read()
        .await
        .list("pod-cleanup", "private", None)
        .is_empty());

    let recreated = super::route_http_request(
        "PUT",
        "/api/pods/pod-cleanup",
        None,
        r#"{"pod":{"podId":"pod-cleanup","name":"Cleanup","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"},{"channelId":"private","kind":0,"name":"Private"}]}}"#,
        &state,
    )
    .await
    .expect("recreate channel");
    assert_eq!(recreated.status, "200 OK");
    let history = super::route_http_request("GET", message_path, None, "", &state)
        .await
        .expect("read recreated channel history");
    assert_eq!(history.status, "200 OK");
    assert_eq!(history.body, "[]");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_creation_never_trusts_a_caller_supplied_peer_identity() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "")
            .with("SLSK_PASSWORD", "")
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "test-token"),
    );
    let response = super::route_http_request(
        "POST",
        "/api/v0/pods",
        Some("Bearer test-token"),
        r#"{"pod":{"podId":"pod:spoofed","name":"Spoofed"},"requestingPeerId":"caller-controlled"}"#,
        &state,
    )
    .await
    .expect("pod create without local identity");
    assert_eq!(response.status, "403 Forbidden");
}

fn test_capability_descriptor(
    username: &str,
    features: Vec<String>,
) -> slskr_client::capabilities::PeerCapabilityDescriptor {
    slskr_client::capabilities::PeerCapabilityDescriptor {
        peer_id: format!("{username}-peer-id"),
        username: username.to_owned(),
        features,
        endpoints: vec!["tcp:127.0.0.1:2234".to_owned()],
        overlay_port: Some(50_305),
        max_payload_length: 65_536,
        issued_at_unix: 1_700_000_000,
        expires_at_unix: 1_800_000_000,
        public_key: [7; 32],
        signature: Some([9; 64]),
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_rendezvous_api_discovers_users_and_mesh_capabilities() {
    let (state, _receiver) = test_state();
    {
        let mut users = state.users.write().await;
        users.watch("alice".to_owned());
        users.watch("Bob".to_owned());
    }
    {
        let mut mesh = state.mesh.write().await;
        mesh.capability_records.push(test_capability_descriptor(
            "ALICE",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records.push(test_capability_descriptor(
            "carol",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records.push(test_capability_descriptor(
            "dave",
            vec![slskr_client::capabilities::FEATURE_CAPABILITIES_V1.to_owned()],
        ));
    }

    let status = super::route_http_request(
        "GET",
        "/api/soulseek/mesh-rendezvous/status",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh status");
    assert_eq!(status.status, "200 OK");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["enabled"], true);
    assert_eq!(status_json["activeProbe"], true);
    assert_eq!(status_json["interestTag"], "slskdn-mesh-v1");
    assert_eq!(status_json["candidateCount"], 3);

    let discover = super::route_http_request(
        "GET",
        "/api/soulseek/mesh-rendezvous/discover",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh discover");
    assert_eq!(discover.status, "200 OK");
    let discover_json = serde_json::from_str::<serde_json::Value>(&discover.body).unwrap();
    let users = discover_json["users"].as_array().unwrap();
    assert_eq!(users[0]["username"], "alice");
    assert_eq!(users[1]["username"], "Bob");
    assert_eq!(users[2]["username"], "carol");
    assert_eq!(discover_json["capabilityRecordCount"], 3);

    let capabilities =
        super::route_http_request("GET", "/api/soulseek/peer-capabilities", None, "", &state)
            .await
            .expect("peer capabilities");
    assert_eq!(capabilities.status, "200 OK");
    let capabilities_json = serde_json::from_str::<serde_json::Value>(&capabilities.body).unwrap();
    assert_eq!(capabilities_json.as_array().unwrap().len(), 3);
    assert_eq!(capabilities_json[0]["meshCapable"], true);
    assert_eq!(capabilities_json[2]["meshCapable"], false);

    let peers = super::route_http_request("GET", "/api/mesh/peers", None, "", &state)
        .await
        .expect("mesh peers");
    assert_eq!(peers.status, "200 OK");
    assert!(peers.body.contains("\"peers\""));
    assert!(peers.body.contains("\"carol\""));
}

async fn spawn_mesh_range_source(
    content: Arc<Vec<u8>>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh range source");
    let address = listener.local_addr().expect("mesh range source address");
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let content = Arc::clone(&content);
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1_024];
                loop {
                    let count = stream.read(&mut buffer).await.expect("read range request");
                    if count == 0 {
                        return;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).expect("range request UTF-8");
                let range = request
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("range"))
                    .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                    .expect("range header");
                let (start, end) = range.split_once('-').expect("range bounds");
                let start = start.parse::<usize>().expect("range start");
                let end = end.parse::<usize>().expect("range end");
                let body = &content[start..=end];
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                            body.len(),
                            content.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .expect("write range headers");
                stream.write_all(body).await.expect("write range body");
            });
        }
    });
    (address, task)
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn hashdb_shadow_index_uses_operator_trusted_frozen_peer_for_mesh_swarm() {
    use sha2::{Digest, Sha256};

    let content = Arc::new(b"verified mesh source discovery".to_vec());
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));
    let (source_a, task_a) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let (source_b, task_b) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let trusted_peers = serde_json::json!([{
        "peerId": "peer-a",
        "username": "source-a",
        "overlayEndpoint": "127.0.0.1:50305",
        "certificateSha256": "11".repeat(32),
        "rangeEndpoint": format!("http://{source_a}/content")
    }]);
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &trusted_peers.to_string()),
    );

    let hash_merge = super::route_http_request(
        "POST",
        "/api/v0/hashdb/sync/merge",
        None,
        &format!(
            r#"{{"entries":[{{"flacKey":"track-key","size":{},"fileSha256":"{}","musicBrainzId":"recording-1"}}]}}"#,
            content.len(),
            expected_hash.to_ascii_uppercase()
        ),
        &state,
    )
    .await
    .expect("merge hash metadata");
    assert_eq!(hash_merge.status, "200 OK");

    let shadow_merge = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        r#"{"records":[{"recordingId":"recording-1","peerIds":["peer-a","peer-b"]}]}"#,
        &state,
    )
    .await
    .expect("merge shadow index");
    assert_eq!(shadow_merge.status, "200 OK");

    {
        let mut mesh = state.mesh.write().await;
        let mut descriptor = test_capability_descriptor(
            "source-b",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        );
        descriptor.peer_id = "peer-b".to_owned();
        descriptor.endpoints = vec![format!("http://{source_b}/content")];
        mesh.capability_records.push(descriptor);
    }

    let by_size = super::route_http_request(
        "GET",
        &format!("/api/v0/hashdb/hash/by-size/{}", content.len()),
        None,
        "",
        &state,
    )
    .await
    .expect("lookup hashes by size");
    let by_size_json = serde_json::from_str::<serde_json::Value>(&by_size.body).unwrap();
    assert_eq!(by_size_json["count"], 1);
    assert_eq!(by_size_json["entries"][0]["musicBrainzId"], "recording-1");

    let output_path = format!("mesh-discovery/{}.flac", uuid::Uuid::new_v4());
    let swarm = super::route_http_request(
        "POST",
        "/api/v0/multisource/swarm",
        None,
        &format!(
            r#"{{"filename":"Track.flac","fileSize":{},"expectedHash":"{}","outputPath":"{}","sources":[]}}"#,
            content.len(),
            expected_hash.to_ascii_uppercase(),
            output_path
        ),
        &state,
    )
    .await
    .expect("execute discovered mesh swarm");
    let swarm_json = serde_json::from_str::<serde_json::Value>(&swarm.body).unwrap();
    assert_eq!(swarm.status, "200 OK", "{}", swarm.body);
    assert_eq!(swarm_json["success"], true, "{}", swarm.body);
    assert_eq!(
        std::fs::read(state.config.downloads_dir.join(&output_path))
            .expect("read verified mesh output"),
        content.as_slice()
    );

    task_a.abort();
    task_b.abort();
    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
}

fn build_stun_success_response(transaction_id: [u8; 12], mapped: SocketAddr) -> Vec<u8> {
    let SocketAddr::V4(mapped) = mapped else {
        panic!("STUN test fixture requires an IPv4 mapped address");
    };
    let xor_port = mapped.port() ^ ((crate::mesh_dht_runtime::STUN_MAGIC_COOKIE >> 16) as u16);
    let xor_address = u32::from(*mapped.ip()) ^ crate::mesh_dht_runtime::STUN_MAGIC_COOKIE;
    let mut attribute = Vec::with_capacity(8);
    attribute.push(0x00);
    attribute.push(0x01);
    attribute.extend_from_slice(&xor_port.to_be_bytes());
    attribute.extend_from_slice(&xor_address.to_be_bytes());

    let mut response = Vec::with_capacity(32);
    response.extend_from_slice(&0x0101_u16.to_be_bytes());
    response.extend_from_slice(&(attribute.len() as u16 + 4).to_be_bytes());
    response.extend_from_slice(&crate::mesh_dht_runtime::STUN_MAGIC_COOKIE.to_be_bytes());
    response.extend_from_slice(&transaction_id);
    response.extend_from_slice(&0x0020_u16.to_be_bytes());
    response.extend_from_slice(&(attribute.len() as u16).to_be_bytes());
    response.extend_from_slice(&attribute);
    response
}

async fn serve_one_stun_response(socket: &tokio::net::UdpSocket, mapped: SocketAddr) {
    let mut buf = [0_u8; 128];
    let (count, peer) = socket.recv_from(&mut buf).await.expect("recv STUN request");
    assert!(count >= 20, "STUN request too short");
    let mut transaction_id = [0_u8; 12];
    transaction_id.copy_from_slice(&buf[8..20]);
    let response = build_stun_success_response(transaction_id, mapped);
    socket
        .send_to(&response, peer)
        .await
        .expect("send STUN response");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn stun_response_parsing_decodes_the_xor_mapped_address() {
    let mapped = "203.0.113.5:51820".parse::<SocketAddr>().unwrap();
    let response = build_stun_success_response([7_u8; 12], mapped);
    assert_eq!(super::parse_stun_mapped_address(&response), Some(mapped));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn stun_probe_resolves_the_mapped_address_from_a_real_udp_round_trip() {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind STUN fixture");
    let address = socket.local_addr().expect("STUN fixture address");
    let mapped = "198.51.100.9:4000".parse::<SocketAddr>().unwrap();
    let server = tokio::spawn(async move { serve_one_stun_response(&socket, mapped).await });
    let result = super::stun_probe(&address.to_string())
        .await
        .expect("STUN probe");
    server.await.expect("STUN fixture task");
    assert_eq!(result.mapped, mapped);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn detect_nat_type_reports_symmetric_when_the_mapping_changes_between_probes() {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind STUN fixture");
    let address = socket.local_addr().expect("STUN fixture address");
    let server = tokio::spawn(async move {
        serve_one_stun_response(&socket, "198.51.100.9:4000".parse().unwrap()).await;
        serve_one_stun_response(&socket, "198.51.100.9:4001".parse().unwrap()).await;
    });
    let server_addr = address.to_string();
    let (nat_type, detected) = super::detect_nat_type(&[&server_addr]).await;
    server.await.expect("STUN fixture task");
    assert_eq!(nat_type, "symmetric");
    assert!(detected);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn detect_nat_type_reports_restricted_when_the_mapping_is_stable() {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind STUN fixture");
    let address = socket.local_addr().expect("STUN fixture address");
    let mapped: SocketAddr = "198.51.100.9:4000".parse().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..3 {
            serve_one_stun_response(&socket, mapped).await;
        }
    });
    let server_addr = address.to_string();
    let (nat_type, detected) = super::detect_nat_type(&[&server_addr, &server_addr]).await;
    server.await.expect("STUN fixture task");
    assert_eq!(nat_type, "restricted");
    assert!(detected);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn detect_nat_type_reports_unknown_when_no_server_responds() {
    let (nat_type, detected) = super::detect_nat_type(&["127.0.0.1:1"]).await;
    assert_eq!(nat_type, "unknown");
    assert!(!detected);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_sync_and_realm_conflict_routes_require_single_encoded_segments() {
    let (state, _receiver) = test_state();
    {
        let mut users = state.users.write().await;
        users.watch("mesh peer".to_owned());
    }

    let mesh_sync =
        super::route_http_request("POST", "/api/mesh/sync/mesh%20peer", None, "{}", &state)
            .await
            .expect("encoded mesh sync");
    assert_eq!(mesh_sync.status, "202 Accepted");
    let mesh_sync_json = serde_json::from_str::<serde_json::Value>(&mesh_sync.body).unwrap();
    assert_eq!(mesh_sync_json["username"], "mesh peer");
    assert_eq!(mesh_sync_json["queued"], true);

    let aliased_mesh_sync = super::route_http_request(
        "POST",
        "/api/mesh/sync/mesh%20peer/untrusted",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject aliased mesh sync");
    assert_eq!(aliased_mesh_sync.status, "404 Not Found");

    let realm_conflicts = super::route_http_request(
        "GET",
        "/api/realm-subject-indexes/local%20realm/conflicts",
        None,
        "",
        &state,
    )
    .await
    .expect("encoded realm conflicts");
    assert_eq!(realm_conflicts.status, "200 OK");
    assert!(realm_conflicts.body.contains("local realm"));

    let aliased_realm_conflicts = super::route_http_request(
        "GET",
        "/api/realm-subject-indexes/local/extra/conflicts",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased realm conflicts");
    assert_eq!(aliased_realm_conflicts.status, "404 Not Found");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn realm_subject_indexes_persist_authority_and_compute_conflicts() {
    let (state, _receiver) = test_state();
    let index = |id: &str, subject: &str, title: &str, discogs: &str| {
        let mut index = serde_json::json!({
            "id": id,
            "realmId": super::realm_subject_index::DEFAULT_REALM_ID,
            "subjectNamespace": "music",
            "revision": 1,
            "publishedAt": "2026-08-01T00:00:00Z",
            "entries": [{
                "subjectId": subject,
                "workRef": {
                    "domain": "music",
                    "title": title,
                    "creator": "Artist",
                },
                "externalIds": {
                    "musicbrainz:recording": "recording-1",
                    "discogs": discogs,
                },
                "aliases": ["shared-alias"],
            }],
            "signature": {
                "signer": super::realm_subject_index::DEFAULT_GOVERNANCE_ROOT,
                "algorithm": "realm-governance-sha256",
                "payloadHash": "",
                "value": "signature",
            },
        });
        index["signature"]["payloadHash"] =
            serde_json::json!(super::realm_subject_index::compute_payload_hash(&index));
        index
    };
    let merged = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"recording-1","peerIds":["peer-a"],"updatedAt":1}],
            "realmIndexes": [
                index("index-a", "subject-a", "Title A", "discogs-a"),
                index("index-b", "subject-b", "Title B", "discogs-b"),
                index("index-c", "subject-a", "Title C", "discogs-c"),
            ],
        })
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(merged.status, "200 OK", "{}", merged.body);

    let indexes = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&indexes.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let conflicts = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let conflicts = serde_json::from_str::<serde_json::Value>(&conflicts.body).unwrap();
    assert_eq!(conflicts["indexCount"], 3);
    assert_eq!(conflicts["entryCount"], 3);
    assert_eq!(conflicts["hasConflicts"], true);
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "external-id"));
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "recording-subject"));
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "workref-identity"));
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "alias-subject"));

    let resolutions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-1/resolutions",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&resolutions.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let disabled = super::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-b/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"operator","note":"conflicting authority"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(disabled.status, "200 OK", "{}", disabled.body);
    let disabled = super::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-c/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"operator","note":"conflicting authority"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(disabled.status, "200 OK", "{}", disabled.body);
    let decisions = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&decisions.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let after_disable = super::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let after_disable = serde_json::from_str::<serde_json::Value>(&after_disable.body).unwrap();
    assert_eq!(after_disable["disabledAuthorityCount"], 2);
    assert_eq!(after_disable["hasConflicts"], false);

    let missing_decision = super::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/missing/authority-decision",
        None,
        r#"{"enabled":true,"decidedBy":"operator"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_decision.status, "400 Bad Request");
    assert!(missing_decision
        .body
        .contains("Index authority was not found"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn telemetry_api_returns_runtime_health_without_secrets() {
    let (state, _receiver) = test_state();
    state.shares.write().await.cache_error =
        Some("share cache write failed: /private/share-index.tsv denied".to_owned());
    {
        let mut transfers = state.transfers.write().await;
        transfers.state_error =
            Some("transfer state parse failed at /private/transfer-state.json".to_owned());
        transfers.events_error =
            Some("transfer event open failed at /private/transfer-events.tsv".to_owned());
    }

    let response = super::route_http_request("GET", "/api/v0/telemetry", None, "", &state)
        .await
        .expect("telemetry response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "application/json");
    assert!(response.body.contains("\"service\":{\"name\":\"slskr\""));
    assert!(response.body.contains("\"health\":{"));
    assert!(response.body.contains("\"connected\":false"));
    assert!(response
        .body
        .contains("\"share_cache_file\":\"share-index.tsv\""));
    assert!(response
        .body
        .contains("\"share_cache_kind\":\"compatibility-debug\""));
    assert!(response
        .body
        .contains("\"transfer_events_file\":\"transfer-events.tsv\""));
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["shares"]["roots"], 0);
    assert_eq!(json["shares"]["files"], 1);
    assert_eq!(json["storage"]["share_cache_enabled"], true);
    assert_eq!(
        json["storage"]["share_cache_error"],
        "share cache unavailable"
    );
    assert!(!response.body.contains("/private"));
    assert!(!response.body.contains("denied"));
    assert_eq!(
        json["storage"]["transfer_state_error"],
        "transfer state unavailable"
    );
    assert_eq!(
        json["storage"]["transfer_events_error"],
        "transfer events unavailable"
    );
    assert_eq!(json["health"]["transferState"], false);
    assert_eq!(json["health"]["transferEvents"], false);
    assert_eq!(json["shares"]["cache_enabled"], true);
    assert_eq!(json["database"]["enabled"], false);
    assert_eq!(json["health"]["database"], true);
    assert_eq!(json["events"]["total"], 0);
    assert_eq!(json["messages"]["total"], 0);
    assert_eq!(json["rooms"]["joined"], 0);
    assert_eq!(json["transfers"]["total"], 0);
    assert_eq!(json["runtime"]["backfillRuns"], 0);
    assert!(!response.body.contains("secret"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_storage_errors_redact_internal_details() {
    let mut transfers = super::TransferQueue::new_in_memory(8);
    transfers.state_error =
        Some("transfer state parse failed at /private/transfer-state.json".to_owned());
    transfers.events_error =
        Some("transfer event open failed at /private/transfer-events.tsv".to_owned());

    let json = transfers.json(None);
    assert!(json.contains("\"state_error\":\"transfer state unavailable\""));
    assert!(json.contains("\"events_error\":\"transfer events unavailable\""));
    assert!(!json.contains("/private"));
    assert!(!json.contains("parse failed"));
    assert!(!json.contains("open failed"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_json_redacts_local_storage_details() {
    let mut transfers = super::TransferQueue::new_in_memory(8);
    let entry = transfers.create(
        0,
        None,
        "Remote/Track.flac".to_owned(),
        Some("/private/downloads/Track.flac".to_owned()),
        Some(4),
    );

    assert_eq!(
        entry.local_path.as_deref(),
        Some("/private/downloads/Track.flac")
    );
    transfers.update_status(
        entry.id,
        "failed",
        None,
        Some("write failed at /private/downloads/Track.flac: denied".to_owned()),
    );
    let json = transfers.json(None);
    assert!(json.contains("\"local_path\":null"));
    assert!(json.contains("\"reason\":\"transfer failed\""));
    assert!(!json.contains("/private/downloads"));
    assert!(!json.contains("denied"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn listener_errors_redact_internal_details() {
    let (state, _receiver) = test_state();
    state.listeners.write().await.last_error =
        Some("regular listener bind failed at 10.0.0.8:2234: permission denied".to_owned());

    let response = super::route_http_request("GET", "/api/v0/listeners", None, "", &state)
        .await
        .expect("listener response");
    assert_eq!(response.status, "200 OK");
    assert!(response
        .body
        .contains("\"last_error\":\"listener unavailable\""));
    assert!(!response.body.contains("10.0.0.8"));
    assert!(!response.body.contains("permission denied"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn session_errors_redact_internal_details() {
    let (state, _receiver) = test_state();
    state.session.write().await.last_error =
        Some("server receive failed from 10.0.0.9:2242: /private/session.db denied".to_owned());

    for path in ["/api/v0/server"] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("session response");
        assert_eq!(response.status, "200 OK", "{path}");
        assert!(response.body.contains("session operation failed"), "{path}");
        assert!(!response.body.contains("10.0.0.9"), "{path}");
        assert!(!response.body.contains("/private"), "{path}");
        assert!(!response.body.contains("denied"), "{path}");
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn browse_errors_redact_internal_details() {
    let record = super::BrowseRecord {
        username: "friend".to_owned(),
        status: "failed",
        entries: Vec::new(),
        reason: Some(
            "plain browse connect failed at 10.0.0.9:2242: /private/socket denied".to_owned(),
        ),
        folder: None,
        indirect_token: None,
        requested_at: Some(1),
        updated_at: 2,
    };

    for body in [record.json(), record.controller_status_json()] {
        assert!(body.contains("browse failed"));
        assert!(!body.contains("10.0.0.9"));
        assert!(!body.contains("/private"));
        assert!(!body.contains("denied"));
    }
    assert!(record.reason.as_deref().unwrap().contains("10.0.0.9"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn browse_failure_events_redact_internal_details() {
    let (state, _receiver) = test_state();
    super::record_event(
        &state,
        "browse.failed",
        "friend",
        Some("connect to 10.0.0.9:2242 via /private/socket denied".to_owned()),
    )
    .await;

    let events = state.events.read().await;
    let event = events.records.last().expect("browse failure event");
    assert_eq!(event.detail.as_deref(), Some("browse failed"));
    for body in [event.json(), event.controller_json().to_string()] {
        assert!(!body.contains("10.0.0.9"));
        assert!(!body.contains("/private"));
        assert!(!body.contains("denied"));
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn root_serves_webui_or_fallback_dashboard() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request("GET", "/", None, "", &state)
        .await
        .expect("root response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "text/html; charset=utf-8");
    if response.body.contains("id=\"root\"") && response.body.contains("/assets/") {
        assert!(response.body.contains("<script"));
        return;
    }

    assert!(response.body.contains("<h1>slskr</h1>"));
    assert!(response.body.contains("Fallback dashboard"));
    assert!(response.body.contains("href=\"/\""));
    assert!(response.body.contains("SLSKR_WEB_BUILD_DIR"));
    assert!(response.body.contains("id=\"session-actions\""));
    assert!(response
        .body
        .contains("data-session-action=\"privileges/check\""));
    assert!(response.body.contains("id=\"search-form\""));
    assert!(response.body.contains("id=\"watch-form\""));
    assert!(response.body.contains("id=\"unwatch-button\""));
    assert!(response.body.contains("id=\"browse-request-form\""));
    assert!(response.body.contains("id=\"browse-folder\""));
    assert!(response.body.contains("id=\"share-rescan-form\""));
    assert!(response.body.contains("id=\"transfer-form\""));
    assert!(response.body.contains("id=\"transfer-progress\""));
    assert!(response.body.contains("id=\"transfer-local-path\""));
    assert!(response.body.contains("id=\"message-form\""));
    assert!(response.body.contains("id=\"room-join-form\""));
    assert!(response.body.contains("id=\"room-message-form\""));
    assert!(response.body.contains("id=\"room-message-username\""));
    assert!(response.body.contains("id=\"token-form\""));
    assert!(response
        .body
        .contains("id=\"api-token\" name=\"token\" type=\"password\""));
    assert!(response.body.contains("id=\"user-table\""));
    assert!(response.body.contains("id=\"share-table\""));
    assert!(response.body.contains("id=\"message-table\""));
    assert!(response.body.contains("id=\"room-table\""));
    assert!(response.body.contains("id=\"browse-table\""));
    assert!(response.body.contains("id=\"search-filter-q\""));
    assert!(response.body.contains("id=\"transfer-filter-status\""));
    assert!(response.body.contains("id=\"share-filter-extension\""));
    assert!(response.body.contains("id=\"message-filter-direction\""));
    assert!(response.body.contains("id=\"room-filter-joined\""));
    assert!(response.body.contains("id=\"browse-filter-status\""));
    assert!(response.body.contains("/api/v0/stats"));
    assert!(response.body.contains("/api/v0/session/"));
    assert!(response.body.contains("/api/v0/searches"));
    assert!(response.body.contains("data-search-action=\"complete\""));
    assert!(response.body.contains("/api/v0/users/watch"));
    assert!(response.body.contains("data-user-action=\"browse\""));
    assert!(response.body.contains("data-user-action=\"stats\""));
    assert!(response.body.contains("/stats/request"));
    assert!(response.body.contains("/api/v0/shares/catalog"));
    assert!(response.body.contains("/browse/request"));
    assert!(response.body.contains("/api/v0/shares/rescan"));
    assert!(response.body.contains("/api/v0/transfers"));
    assert!(response.body.contains("data-transfer-action=\"complete\""));
    assert!(response.body.contains("/api/v0/messages"));
    assert!(response.body.contains("data-message-action=\"ack\""));
    assert!(response.body.contains("/api/v0/rooms/"));
    assert!(response.body.contains("/api/v0/rooms/refresh"));
    assert!(response.body.contains("data-room-action=\"leave\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn spa_deep_links_serve_html_shell() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request("GET", "/system/network", None, "", &state)
        .await
        .expect("SPA deep-link response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "text/html; charset=utf-8");
    assert!(response.body.contains("id=\"root\"") || response.body.contains("<h1>slskr</h1>"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn head_spa_routes_return_ok_without_body() {
    let (state, _receiver) = test_state();

    for path in ["/", "/system/network"] {
        let response = super::route_http_request("HEAD", path, None, "", &state)
            .await
            .expect("HEAD SPA response");

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(!response.body.is_empty());
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn fallback_dashboard_is_available_on_dashboard_route() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request("GET", "/dashboard", None, "", &state)
        .await
        .expect("dashboard response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "text/html; charset=utf-8");
    assert!(response.body.contains("Fallback dashboard"));
    assert!(response.body.contains("href=\"/\""));
    assert!(response.body.contains("id=\"session-actions\""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn web_static_resolver_stays_under_build_root() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-static-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();
    std::fs::write(root.join("index.html"), "<html></html>").unwrap();
    std::fs::write(root.join("assets").join("app.js"), "console.log('ok')").unwrap();
    std::fs::write(root.join("static").join("app.js"), "console.log('static')").unwrap();
    std::fs::write(root.join("favicon.ico"), "ico").unwrap();

    let (asset, content_type) =
        super::web_static_file_for_request_under_root(&root, "/assets/app.js")
            .expect("asset resolved");
    assert!(asset.ends_with("assets/app.js"));
    assert_eq!(content_type, "text/javascript; charset=utf-8");

    let (nested_asset, nested_content_type) =
        super::web_static_file_for_request_under_root(&root, "/system/mediacore/assets/app.js")
            .expect("nested SPA asset resolved");
    assert!(nested_asset.ends_with("assets/app.js"));
    assert_eq!(nested_content_type, "text/javascript; charset=utf-8");

    let (nested_static_asset, nested_static_content_type) =
        super::web_static_file_for_request_under_root(&root, "/system/static/app.js")
            .expect("nested static asset resolved");
    assert!(nested_static_asset.ends_with("static/app.js"));
    assert_eq!(nested_static_content_type, "text/javascript; charset=utf-8");

    let (nested_icon, nested_icon_type) =
        super::web_static_file_for_request_under_root(&root, "/system/favicon.ico")
            .expect("nested SPA root icon resolved");
    assert!(nested_icon.ends_with("favicon.ico"));
    assert_eq!(nested_icon_type, "image/x-icon");

    let (fallback, _) = super::web_static_file_for_request_under_root(&root, "/missing-route")
        .expect("SPA fallback resolved");
    assert!(fallback.ends_with("index.html"));
    assert!(super::web_static_file_for_request_under_root(&root, "/../Cargo.toml").is_none());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn web_static_resolver_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "slskr-web-static-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-web-static-outside-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("index.html"), "<html></html>").unwrap();
    std::fs::write(&outside, "secret").unwrap();
    symlink(&outside, root.join("leak.txt")).unwrap();

    assert!(super::web_static_file_for_request_under_root(&root, "/leak.txt").is_none());
    assert!(super::read_bounded_web_static_file(&root.join("leak.txt")).is_err());

    let outside_dir = outside.with_extension("dir");
    std::fs::create_dir_all(&outside_dir).unwrap();
    std::fs::write(outside_dir.join("secret.txt"), "secret").unwrap();
    symlink(&outside_dir, root.join("linked")).unwrap();
    assert!(
        super::read_bounded_web_static_file_under_root(&root, &root.join("linked/secret.txt"))
            .is_err()
    );

    let _ = std::fs::remove_file(outside);
    let _ = std::fs::remove_dir_all(outside_dir);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn web_static_reader_rejects_oversized_assets() {
    let path = std::env::temp_dir().join(format!(
        "slskr-web-static-large-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(super::MAX_WEB_STATIC_BYTES + 1).unwrap();

    let error = super::read_bounded_web_static_file(&path)
        .expect_err("oversized static asset should be rejected");
    assert!(error.contains("static asset is too large"));

    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn web_static_oversize_errors_return_413() {
    let response = super::web_static_error_response(
        "static asset is too large: 1 bytes at /srv/private/web/index.html",
    );
    assert_eq!(response.status, "413 Payload Too Large");
    assert!(response.body.contains("static asset is too large"));
    assert!(!response.body.contains("1 bytes"));
    assert!(!response.body.contains("/srv/private"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn web_static_internal_errors_do_not_expose_filesystem_details() {
    let response = super::web_static_error_response(
        "static file confined open failed: permission denied: /srv/private/web/index.html",
    );
    assert_eq!(response.status, "500 Internal Server Error");
    assert_eq!(response.body, "{\"error\":\"static asset is unavailable\"}");
    assert!(!response.body.contains("permission denied"));
    assert!(!response.body.contains("/srv/private"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn web_static_string_reader_rejects_invalid_utf8() {
    let path = std::env::temp_dir().join(format!(
        "slskr-web-static-invalid-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&path, [0xff_u8]).unwrap();

    let error = super::read_bounded_web_static_string(&path)
        .expect_err("invalid UTF-8 static text should be rejected");
    assert!(error.contains("static asset is not UTF-8"));

    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn web_static_csp_rejects_inline_scripts_and_scopes_style_and_wasm_exceptions() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-static-csp-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let react_index = root.join("index.html");
    std::fs::write(&react_index, "<html></html>").unwrap();

    let react_csp = super::web_static_content_security_policy(&react_index);
    assert!(react_csp.contains("script-src 'self'"));
    assert!(!react_csp.contains("script-src 'self' 'unsafe-inline'"));
    assert!(react_csp.contains("style-src-attr 'unsafe-inline'"));
    assert!(react_csp.contains("'sha256-AVTm08UMHPqpttgoudpSsvenKKfidtwuSnUVJLIuqcA='"));
    assert!(!react_csp.contains("wasm-unsafe-eval"));

    std::fs::write(root.join("slskr_web.wasm"), []).unwrap();
    let wasm_csp = super::web_static_content_security_policy(&react_index);
    assert!(wasm_csp.contains("script-src 'self' 'wasm-unsafe-eval'"));
    assert!(!wasm_csp.contains("script-src 'self' 'unsafe-inline'"));
    assert!(wasm_csp.contains("style-src-attr 'unsafe-inline'"));

    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_catalog_supports_filters_and_pagination() {
    let (state, _receiver) = test_state();
    {
        let mut shares = state.shares.write().await;
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Virtual/Other.mp3".to_owned(),
            size: 12,
            extension: "mp3".to_owned(),
            attributes: Vec::new(),
        });
    }

    let response = super::route_http_request(
        "GET",
        "/api/v0/shares/catalog?q=test&extension=flac&limit=1&offset=0",
        None,
        "",
        &state,
    )
    .await
    .expect("catalog response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"count\":2"));
    assert!(response.body.contains("\"filtered_count\":1"));
    assert!(response.body.contains("\"total_bytes\":42"));
    assert!(response.body.contains("\"limit\":1"));
    assert!(response.body.contains("\"path\":\"Virtual/Test.flac\""));
    assert!(!response.body.contains("Other.mp3"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn files_api_lists_one_share_root_without_local_paths() {
    let (state, _receiver) = test_state();
    {
        let mut shares = state.shares.write().await;
        shares.roots.push(super::ShareRoot {
            label: "Music".to_owned(),
            local_path: PathBuf::from("Music"),
            raw: "Music".to_owned(),
            directories: 0,
            files: 2,
            bytes: 142,
            extensions: vec![super::ShareExtensionSummary {
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
            filename: "Music/Other.mp3".to_owned(),
            size: 100,
            extension: "mp3".to_owned(),
            attributes: Vec::new(),
        });
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Music/Test.flac".to_owned(),
            size: 42,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Library/Known/Release.jpg".to_owned(),
            size: 123,
            extension: "jpg".to_owned(),
            attributes: Vec::new(),
        });
        shares.local_paths.insert(
            "Music/Test.flac".to_owned(),
            PathBuf::from("/tmp/private/Test.flac"),
        );
    }

    let response = super::route_http_request(
        "GET",
        "/api/v0/files/Music?extension=flac",
        None,
        "",
        &state,
    )
    .await
    .expect("files response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"label\":\"Music\""));
    assert!(response.body.contains("\"path\":\"Test.flac\""));
    assert!(response
        .body
        .contains("\"virtual_path\":\"Music/Test.flac\""));
    assert!(response.body.contains("\"filtered_count\":1"));
    assert!(!response.body.contains("/tmp/private"));
    assert!(!response.body.contains("Other.mp3"));

    let missing = super::route_http_request("GET", "/api/v0/files/Missing", None, "", &state)
        .await
        .expect("missing files response");
    assert_eq!(missing.status, "404 Not Found");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn files_api_projects_folder_views_and_directory_summaries() {
    let (state, _receiver) = test_state();
    {
        let mut shares = state.shares.write().await;
        shares.roots.push(super::ShareRoot {
            label: "Music".to_owned(),
            local_path: PathBuf::from("Music"),
            raw: "Music".to_owned(),
            directories: 2,
            files: 4,
            bytes: 410,
            extensions: vec![
                super::ShareExtensionSummary {
                    extension: "flac".to_owned(),
                    files: 3,
                    bytes: 310,
                },
                super::ShareExtensionSummary {
                    extension: "mp3".to_owned(),
                    files: 1,
                    bytes: 100,
                },
            ],
            statistics_ready: true,
        });
        for (filename, size, extension) in [
            ("Music/Artist/Album/Track.flac", 100, "flac"),
            ("Music/Artist/Album/Second.flac", 110, "flac"),
            ("Music/Artist/Live/Bootleg.flac", 100, "flac"),
            ("Music/Artist/Loose.mp3", 100, "mp3"),
        ] {
            shares.entries.push(FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: filename.to_owned(),
                size,
                extension: extension.to_owned(),
                attributes: Vec::new(),
            });
        }
    }

    let root = super::route_http_request("GET", "/api/v0/files/Music", None, "", &state)
        .await
        .expect("root files response");
    assert_eq!(root.status, "200 OK");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["count"], 4);
    assert_eq!(root_json["filtered_count"], 4);
    assert_eq!(root_json["directory_count"], 1);
    assert_eq!(root_json["directories"][0]["path"], "Artist");
    assert_eq!(root_json["directories"][0]["file_count"], 4);
    assert_eq!(root_json["entries"].as_array().unwrap().len(), 4);

    let folder = super::route_http_request(
        "GET",
        "/api/v0/files/Music?folder=Artist&extension=mp3",
        None,
        "",
        &state,
    )
    .await
    .expect("folder files response");
    assert_eq!(folder.status, "200 OK");
    let folder_json = serde_json::from_str::<serde_json::Value>(&folder.body).unwrap();
    assert_eq!(folder_json["folder"], "Artist");
    assert_eq!(folder_json["recursive"], false);
    assert_eq!(folder_json["filtered_count"], 1);
    assert_eq!(folder_json["directory_count"], 0);
    assert_eq!(folder_json["entries"][0]["path"], "Loose.mp3");
    assert_eq!(
        folder_json["entries"][0]["virtual_path"],
        "Music/Artist/Loose.mp3"
    );

    let nested = super::route_http_request(
        "GET",
        "/api/v0/files/Music?folder=Artist&recursive=true&extension=flac&q=album",
        None,
        "",
        &state,
    )
    .await
    .expect("recursive folder files response");
    assert_eq!(nested.status, "200 OK");
    let nested_json = serde_json::from_str::<serde_json::Value>(&nested.body).unwrap();
    assert_eq!(nested_json["recursive"], true);
    assert_eq!(nested_json["filtered_count"], 2);
    assert_eq!(nested_json["directory_count"], 1);
    assert_eq!(nested_json["directories"][0]["path"], "Album");
    assert_eq!(nested_json["entries"][0]["path"], "Album/Track.flac");
    assert!(!nested.body.contains("Bootleg.flac"));
    assert!(!nested.body.contains("/tmp/private"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_file_delete_routes_are_forbidden_by_default() {
    let (state, _receiver) = test_state();
    let response = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("default remote file management policy");
    assert_eq!(response.status, "403 Forbidden");
    assert!(response.content_type.is_empty());
    assert!(response.body.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_file_delete_routes_are_scoped_to_storage_roots() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let download_file = state.config.downloads_dir.join("Remote").join("Song.mp3");
    std::fs::create_dir_all(download_file.parent().unwrap()).unwrap();
    std::fs::write(&download_file, b"song").unwrap();

    let aliased_delete = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/unrelated/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased file delete");
    assert_eq!(aliased_delete.status, "404 Not Found");
    assert!(download_file.exists());

    let deleted = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("delete downloaded file");
    assert_eq!(deleted.status, "204 No Content");
    assert!(deleted.body.is_empty());
    assert!(!download_file.exists());

    let missing = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &state,
    )
    .await
    .expect("delete missing downloaded file");
    assert_eq!(missing.status, "204 No Content");

    let (controller_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let controller_missing = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/UmVtb3RlL1NvbmcubXAz",
        None,
        "",
        &controller_state,
    )
    .await
    .expect("slskd repeated file delete");
    assert_eq!(controller_missing.status, "204 No Content");

    let traversal = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/Li4vc2VjcmV0",
        None,
        "",
        &state,
    )
    .await
    .expect("delete traversal path");
    assert_eq!(traversal.status, "400 Bad Request");

    let newline_encoded_missing = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/directories/Wm05dg==%0A",
        None,
        "",
        &state,
    )
    .await
    .expect("delete slskd encoded directory");
    assert_eq!(newline_encoded_missing.status, "404 Not Found");
}
