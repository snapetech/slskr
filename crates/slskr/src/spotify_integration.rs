use super::*;

pub(super) fn spotify_redirect_uri(
    state: &AppState,
    spotify: &config::SpotifyIntegrationSettings,
) -> String {
    spotify.redirect_uri.clone().unwrap_or_else(|| {
        let host = if state.config.http_bind.ip().is_unspecified() {
            format!("127.0.0.1:{}", state.config.http_bind.port())
        } else {
            state.config.http_bind.to_string()
        };
        format!("http://{host}/api/v0/integrations/spotify/callback")
    })
}

const SPOTIFY_CONNECTION_AAD: &[u8] = b"slskr.source-feeds.spotify-token-store.v1";

pub(super) fn spotify_connection_path(state_dir: &Path) -> PathBuf {
    state_dir
        .join("source-feeds")
        .join("spotify-connection.json")
}

fn spotify_connection_key(signing_key: &SigningKey) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(SPOTIFY_CONNECTION_AAD);
    digest.update(signing_key.to_bytes());
    digest.finalize().into()
}

pub(super) fn load_spotify_connection_store(
    state_dir: &Path,
    signing_key: &SigningKey,
) -> SpotifyConnectionStore {
    use std::io::Read as _;

    let path = spotify_connection_path(state_dir);
    let Ok(metadata) = fs::symlink_metadata(&path) else {
        return SpotifyConnectionStore::default();
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return SpotifyConnectionStore::default();
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let Ok(mut file) = options.open(path) else {
        return SpotifyConnectionStore::default();
    };
    let Ok(opened) = file.metadata() else {
        return SpotifyConnectionStore::default();
    };
    if !opened.is_file() || opened.len() > 1024 * 1024 {
        return SpotifyConnectionStore::default();
    }
    let mut encoded = String::new();
    if std::io::Read::take(&mut file, 1024 * 1024 + 1)
        .read_to_string(&mut encoded)
        .is_err()
        || encoded.len() > 1024 * 1024
    {
        return SpotifyConnectionStore::default();
    }
    let Ok(envelope) = serde_json::from_str::<ProtectedSpotifyConnection>(&encoded) else {
        return SpotifyConnectionStore::default();
    };
    if envelope.version != 1 {
        return SpotifyConnectionStore::default();
    }
    let Ok(nonce) = URL_SAFE_NO_PAD.decode(envelope.nonce) else {
        return SpotifyConnectionStore::default();
    };
    let Ok(nonce): Result<[u8; 12], _> = nonce.try_into() else {
        return SpotifyConnectionStore::default();
    };
    let Ok(mut ciphertext) = URL_SAFE_NO_PAD.decode(envelope.ciphertext) else {
        return SpotifyConnectionStore::default();
    };
    let Ok(unbound) = ring::aead::UnboundKey::new(
        &ring::aead::AES_256_GCM,
        &spotify_connection_key(signing_key),
    ) else {
        return SpotifyConnectionStore::default();
    };
    let key = ring::aead::LessSafeKey::new(unbound);
    let Ok(plaintext) = key.open_in_place(
        ring::aead::Nonce::assume_unique_for_key(nonce),
        ring::aead::Aad::from(SPOTIFY_CONNECTION_AAD),
        &mut ciphertext,
    ) else {
        return SpotifyConnectionStore::default();
    };
    serde_json::from_slice(plaintext).unwrap_or_default()
}

fn persist_spotify_connection_store(
    state: &AppState,
    store: &SpotifyConnectionStore,
) -> Result<(), String> {
    let path = spotify_connection_path(&state.config.state_dir);
    let directory = path.parent().expect("Spotify connection path has a parent");
    ensure_private_storage_dir(directory, "Spotify connection state")?;
    let mut nonce = [0_u8; 12];
    SysRng
        .try_fill_bytes(&mut nonce)
        .map_err(|_| "secure randomness unavailable for Spotify connection state".to_owned())?;
    let unbound = ring::aead::UnboundKey::new(
        &ring::aead::AES_256_GCM,
        &spotify_connection_key(&state.capability_signing_key),
    )
    .map_err(|_| "Spotify connection encryption key is invalid".to_owned())?;
    let key = ring::aead::LessSafeKey::new(unbound);
    let mut ciphertext = serde_json::to_vec(store)
        .map_err(|error| format!("Spotify connection encode failed: {error}"))?;
    key.seal_in_place_append_tag(
        ring::aead::Nonce::assume_unique_for_key(nonce),
        ring::aead::Aad::from(SPOTIFY_CONNECTION_AAD),
        &mut ciphertext,
    )
    .map_err(|_| "Spotify connection encryption failed".to_owned())?;
    let envelope = ProtectedSpotifyConnection {
        version: 1,
        nonce: URL_SAFE_NO_PAD.encode(nonce),
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
    };
    let body = serde_json::to_vec(&envelope)
        .map_err(|error| format!("Spotify connection envelope encode failed: {error}"))?;
    write_file_atomic(&path, body)
        .map_err(|error| format!("Spotify connection persistence failed: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Spotify connection permissions failed: {error}"))?;
    }
    Ok(())
}

fn delete_spotify_connection_store(state: &AppState) -> Result<(), String> {
    let path = spotify_connection_path(&state.config.state_dir);
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Spotify connection delete failed: {error}")),
    }
}

async fn spotify_token_request(
    spotify: &config::SpotifyIntegrationSettings,
    token_url: &str,
    form: &[(&str, String)],
) -> Result<serde_json::Value, String> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(spotify.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build Spotify client: {error}"))?
        .post(token_url)
        .form(form)
        .send()
        .await
        .map_err(|error| format!("Spotify token request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Spotify returned HTTP {}", response.status()));
    }
    read_bounded_integration_json(response, "Spotify token").await
}

async fn spotify_profile_request(
    spotify: &config::SpotifyIntegrationSettings,
    profile_url: &str,
    access_token: &str,
) -> Result<serde_json::Value, String> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(spotify.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build Spotify client: {error}"))?
        .get(profile_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| format!("Spotify profile request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Spotify returned HTTP {}", response.status()));
    }
    read_bounded_integration_json(response, "Spotify profile").await
}

pub(super) async fn complete_spotify_authorization(
    state: &AppState,
    spotify: &config::SpotifyIntegrationSettings,
    pending: &OAuthStateRecord,
    code: &str,
    token_url: &str,
    profile_url: &str,
) -> Result<serde_json::Value, String> {
    let (_, expected_generation) = spotify_connection_snapshot(state).await;
    let client_id = spotify.client_id.as_deref().unwrap_or_default();
    let verifier = pending
        .code_verifier
        .as_deref()
        .ok_or_else(|| "Spotify authorization state is invalid or expired".to_owned())?;
    let token = spotify_token_request(
        spotify,
        token_url,
        &[
            ("client_id", client_id.to_owned()),
            ("grant_type", "authorization_code".to_owned()),
            ("code", code.to_owned()),
            ("redirect_uri", pending.redirect_uri.clone()),
            ("code_verifier", verifier.to_owned()),
        ],
    )
    .await?;
    let access_token = token
        .get("access_token")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let refresh_token = token
        .get("refresh_token")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if access_token.trim().is_empty() || refresh_token.trim().is_empty() {
        return Err("Spotify token response was incomplete".to_owned());
    }
    let profile = spotify_profile_request(spotify, profile_url, access_token).await?;
    let expires_in = token
        .get("expires_in")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let store = SpotifyConnectionStore {
        access_token: access_token.to_owned(),
        refresh_token: refresh_token.to_owned(),
        scope: token
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        expires_at: i64::try_from(unix_timestamp())
            .unwrap_or(i64::MAX)
            .saturating_add(expires_in.saturating_sub(30).max(30)),
        display_name: profile
            .get("display_name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        spotify_user_id: profile
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    };
    if !persist_spotify_connection_if_current(state, expected_generation, &store).await? {
        return Err("Spotify connection changed while authorization was completing".to_owned());
    }
    let status = store.status_json(spotify.configured());
    Ok(status)
}

fn normalize_spotify_source_target(kind: &str, id: &str) -> SpotifySourceTarget {
    let kind = match kind.trim().to_ascii_lowercase().as_str() {
        "collection" | "liked" | "saved-tracks" => "saved-tracks",
        "saved-albums" => "saved-albums",
        "followed-artists" => "followed-artists",
        "playlists" => "playlists",
        other => other,
    }
    .to_owned();
    let (requires_user_token, scope_hint) = match kind.as_str() {
        "saved-tracks" | "saved-albums" => (true, "user-library-read"),
        "followed-artists" => (true, "user-follow-read"),
        "playlists" => (true, "playlist-read-private playlist-read-collaborative"),
        _ => (false, ""),
    };
    SpotifySourceTarget {
        kind,
        id: id.trim().to_owned(),
        requires_user_token,
        scope_hint: scope_hint.to_owned(),
    }
}

pub(super) fn parse_spotify_source_target(source_text: &str) -> SpotifySourceTarget {
    let trimmed = source_text.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("spotify:") {
        let mut parts = trimmed.split(':');
        let _ = parts.next();
        let kind = parts.next().unwrap_or_default();
        let id = parts.next().unwrap_or_default();
        return normalize_spotify_source_target(kind, id);
    }
    if let Ok(url) = reqwest::Url::parse(trimmed) {
        if url
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("open.spotify.com"))
        {
            let parts = url
                .path_segments()
                .map(|segments| segments.filter(|part| !part.is_empty()).collect::<Vec<_>>())
                .unwrap_or_default();
            let kind = parts.first().copied().unwrap_or_default();
            let id = parts.get(1).copied().unwrap_or_default();
            if kind.eq_ignore_ascii_case("collection") {
                let path = url.path().to_ascii_lowercase();
                return if path.contains("albums") {
                    normalize_spotify_source_target("saved-albums", "")
                } else if path.contains("following") || path.contains("artists") {
                    normalize_spotify_source_target("followed-artists", "")
                } else {
                    normalize_spotify_source_target("saved-tracks", "")
                };
            }
            return normalize_spotify_source_target(kind, id);
        }
    }
    normalize_spotify_source_target(&lower, "")
}

pub(super) fn looks_like_spotify_source(source_text: &str, source_kind: &str) -> bool {
    source_kind.eq_ignore_ascii_case("spotify")
        || source_text
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("spotify:")
        || reqwest::Url::parse(source_text.trim()).is_ok_and(|url| {
            url.host_str()
                .is_some_and(|host| host.eq_ignore_ascii_case("open.spotify.com"))
        })
}

async fn refresh_spotify_user_access_token(
    state: &AppState,
    spotify: &config::SpotifyIntegrationSettings,
    token_url: &str,
) -> Result<String, String> {
    let _permit = state
        .spotify_token_gate
        .acquire()
        .await
        .map_err(|_| "Spotify token gate is unavailable".to_owned())?;
    let (current, expected_generation) = spotify_connection_snapshot(state).await;
    let now = i64::try_from(unix_timestamp()).unwrap_or(i64::MAX);
    if !current.access_token.trim().is_empty() && current.expires_at > now.saturating_add(60) {
        return Ok(current.access_token);
    }
    if current.refresh_token.trim().is_empty() {
        return Ok(String::new());
    }
    let token = spotify_token_request(
        spotify,
        token_url,
        &[
            (
                "client_id",
                spotify.client_id.as_deref().unwrap_or_default().to_owned(),
            ),
            ("grant_type", "refresh_token".to_owned()),
            ("refresh_token", current.refresh_token.clone()),
        ],
    )
    .await?;
    let access_token = token
        .get("access_token")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if access_token.is_empty() {
        return Err("Spotify refresh response was incomplete".to_owned());
    }
    let expires_in = token
        .get("expires_in")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let mut refreshed = current;
    refreshed.access_token = access_token.to_owned();
    if let Some(value) = token
        .get("refresh_token")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        refreshed.refresh_token = value.to_owned();
    }
    if let Some(value) = token
        .get("scope")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        refreshed.scope = value.to_owned();
    }
    refreshed.expires_at = now.saturating_add(expires_in.saturating_sub(30).max(30));
    if persist_spotify_connection_if_current(state, expected_generation, &refreshed).await? {
        Ok(access_token.to_owned())
    } else {
        Ok(state.spotify_connection.read().await.access_token.clone())
    }
}

async fn spotify_app_access_token(
    spotify: &config::SpotifyIntegrationSettings,
    token_url: &str,
) -> Result<String, String> {
    let client_id = spotify.client_id.as_deref().unwrap_or_default().trim();
    let client_secret = spotify.client_secret.as_deref().unwrap_or_default().trim();
    if !spotify.enabled || client_id.is_empty() || client_secret.is_empty() {
        return Ok(String::new());
    }
    use base64::Engine as _;
    let credentials =
        base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(spotify.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build Spotify client: {error}"))?
        .post(token_url)
        .header("Authorization", format!("Basic {credentials}"))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .map_err(|error| format!("Spotify token request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Spotify returned HTTP {}", response.status()));
    }
    let token = read_bounded_integration_json(response, "Spotify token").await?;
    Ok(token
        .get("access_token")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned())
}

pub(super) async fn spotify_source_access_token(
    state: &AppState,
    spotify: &config::SpotifyIntegrationSettings,
    target: &SpotifySourceTarget,
    provided: &str,
    token_url: &str,
) -> Result<String, String> {
    if !provided.trim().is_empty() {
        return Ok(provided.trim().to_owned());
    }
    if target.requires_user_token {
        return refresh_spotify_user_access_token(state, spotify, token_url).await;
    }
    let app_token = spotify_app_access_token(spotify, token_url).await?;
    if !app_token.trim().is_empty() {
        return Ok(app_token);
    }
    refresh_spotify_user_access_token(state, spotify, token_url).await
}

pub(super) async fn spotify_get_json(
    spotify: &config::SpotifyIntegrationSettings,
    url: String,
    token: &str,
) -> Result<serde_json::Value, String> {
    let token = token.trim();
    let token = token
        .get(..7)
        .filter(|prefix| prefix.eq_ignore_ascii_case("Bearer "))
        .map(|_| &token[7..])
        .unwrap_or(token)
        .trim();
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(spotify.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build Spotify client: {error}"))?
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| format!("Spotify API request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Spotify returned HTTP {}", response.status()));
    }
    read_bounded_integration_json(response, "Spotify API").await
}

pub(super) fn spotify_track_row(
    track: &serde_json::Value,
    source: &str,
    source_id: &str,
    album_override: Option<&str>,
) -> Option<SpotifySourceRow> {
    if !track.is_object() {
        return None;
    }
    let title = track
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let artist = track
        .get("artists")
        .and_then(serde_json::Value::as_array)
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| artist.get("name").and_then(serde_json::Value::as_str))
                .filter(|name| !name.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let album = album_override
        .map(str::to_owned)
        .or_else(|| {
            track
                .pointer("/album/name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    Some(SpotifySourceRow {
        raw_text: title.clone(),
        title,
        artist,
        album,
        source: source.to_owned(),
        source_id: source_id.to_owned(),
        provider_url: track
            .pointer("/external_urls/spotify")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    })
}

pub(super) fn spotify_page_done(
    page: &serde_json::Value,
    item_count: usize,
    offset: usize,
) -> bool {
    item_count == 0
        || page.get("next").is_none_or(serde_json::Value::is_null)
        || page
            .get("total")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|total| offset.saturating_add(item_count) >= total as usize)
}

pub(super) async fn fetch_spotify_playlist_tracks(
    spotify: &config::SpotifyIntegrationSettings,
    api_base: &str,
    token: &str,
    playlist_id: &str,
    rows: &mut Vec<SpotifySourceRow>,
    limit: usize,
) -> Result<u64, String> {
    let mut requests = 0_u64;
    let mut offset = 0_usize;
    while rows.len() < limit {
        let page_limit = 50_usize.min(limit - rows.len());
        let page = spotify_get_json(
            spotify,
            format!(
                "{}/playlists/{}/items?limit={page_limit}&offset={offset}&additional_types=track",
                api_base.trim_end_matches('/'),
                url_encode(playlist_id)
            ),
            token,
        )
        .await?;
        requests += 1;
        let items = page
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for item in &items {
            if let Some(row) = spotify_track_row(
                item.get("track").unwrap_or(&serde_json::Value::Null),
                "spotify:playlist",
                playlist_id,
                None,
            ) {
                rows.push(row);
            }
        }
        if spotify_page_done(&page, items.len(), offset) {
            break;
        }
        offset = offset.saturating_add(items.len());
    }
    Ok(requests)
}

pub(super) async fn fetch_spotify_track_pages(
    spotify: &config::SpotifyIntegrationSettings,
    api_base: &str,
    token: &str,
    path: &str,
    source: &str,
    rows: &mut Vec<SpotifySourceRow>,
    limit: usize,
) -> Result<u64, String> {
    let mut requests = 0_u64;
    let mut offset = 0_usize;
    while rows.len() < limit {
        let page_limit = 50_usize.min(limit - rows.len());
        let page = spotify_get_json(
            spotify,
            format!(
                "{}{path}?limit={page_limit}&offset={offset}",
                api_base.trim_end_matches('/')
            ),
            token,
        )
        .await?;
        requests += 1;
        let items = page
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for item in &items {
            let track = item.get("track").unwrap_or(&serde_json::Value::Null);
            let source_id = track
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if let Some(row) = spotify_track_row(track, source, source_id, None) {
                rows.push(row);
            }
        }
        if spotify_page_done(&page, items.len(), offset) {
            break;
        }
        offset = offset.saturating_add(items.len());
    }
    Ok(requests)
}

pub(super) async fn fetch_spotify_playlist_collection(
    spotify: &config::SpotifyIntegrationSettings,
    api_base: &str,
    token: &str,
    path: &str,
    rows: &mut Vec<SpotifySourceRow>,
    limit: usize,
) -> Result<u64, String> {
    let mut requests = 0_u64;
    let mut offset = 0_usize;
    while rows.len() < limit {
        let page = spotify_get_json(
            spotify,
            format!(
                "{}{path}?limit=20&offset={offset}",
                api_base.trim_end_matches('/')
            ),
            token,
        )
        .await?;
        requests += 1;
        let items = page
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for playlist in &items {
            let playlist_id = playlist
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            requests +=
                fetch_spotify_playlist_tracks(spotify, api_base, token, playlist_id, rows, limit)
                    .await?;
            if rows.len() >= limit {
                break;
            }
        }
        if spotify_page_done(&page, items.len(), offset) {
            break;
        }
        offset = offset.saturating_add(items.len());
    }
    Ok(requests)
}

pub(super) fn build_spotify_source_result(
    target: &SpotifySourceTarget,
    rows: Vec<SpotifySourceRow>,
    network_request_count: u64,
) -> serde_json::Value {
    let mut seen = HashSet::new();
    let mut suggestions = Vec::new();
    let mut skipped_rows = Vec::new();
    let mut duplicate_count = 0_u64;
    let mut skipped_count = 0_u64;
    for (index, row) in rows.iter().enumerate() {
        let parts = [row.artist.trim(), row.title.trim(), row.album.trim()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let search_text = if parts.len() >= 2 || row.artist.trim().is_empty() {
            parts.join(" ")
        } else {
            String::new()
        };
        if search_text.is_empty() {
            skipped_count += 1;
            skipped_rows.push(serde_json::json!({
                "rowNumber": index + 1,
                "reason": "Missing artist/title metadata",
                "rawText": row.raw_text,
            }));
            continue;
        }
        let normalized = search_text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        let evidence_key = format!("spotify:{}:{normalized}", row.source);
        if !seen.insert(evidence_key.clone()) {
            duplicate_count += 1;
            continue;
        }
        suggestions.push(serde_json::json!({
            "title": if row.title.trim().is_empty() { search_text.as_str() } else { row.title.as_str() },
            "artist": row.artist,
            "album": row.album,
            "searchText": search_text,
            "source": row.source,
            "sourceId": row.source_id,
            "sourceItemId": row.source_id,
            "providerUrl": row.provider_url,
            "evidenceKey": evidence_key,
            "reason": format!("Imported from spotify {}.", target.kind),
        }));
    }
    serde_json::json!({
        "provider": "spotify",
        "sourceKind": target.kind,
        "sourceId": target.id,
        "totalRows": rows.len(),
        "suggestionCount": suggestions.len(),
        "duplicateCount": duplicate_count,
        "skippedCount": skipped_count,
        "networkRequestCount": network_request_count,
        "requiresAccessToken": false,
        "requiredScopeHint": "",
        "suggestions": suggestions,
        "skippedRows": skipped_rows,
    })
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn spotify_callback_html(message: &str) -> String {
    format!(
        "<!doctype html>\n<html>\n  <head><title>Spotify Connection</title></head>\n  <body>\n    <p>{}</p>\n    <script>\n      if (window.opener) {{\n        window.opener.postMessage({{ type: 'slskr:spotify-connected' }}, window.location.origin);\n      }}\n    </script>\n  </body>\n</html>",
        html_escape(message)
    )
}

async fn spotify_connection_snapshot(state: &AppState) -> (SpotifyConnectionStore, u64) {
    let _persistence_turn = state.spotify_connection_persistence_lock.lock().await;
    let connection = state.spotify_connection.read().await.clone();
    let generation = state
        .spotify_connection_generation
        .load(std::sync::atomic::Ordering::Acquire);
    (connection, generation)
}

pub(super) async fn persist_spotify_connection_if_current(
    state: &AppState,
    expected_generation: u64,
    connection: &SpotifyConnectionStore,
) -> Result<bool, String> {
    let _persistence_turn = state.spotify_connection_persistence_lock.lock().await;
    if state
        .spotify_connection_generation
        .load(std::sync::atomic::Ordering::Acquire)
        != expected_generation
    {
        return Ok(false);
    }
    persist_spotify_connection_store(state, connection)?;
    *state.spotify_connection.write().await = connection.clone();
    state
        .spotify_connection_generation
        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    Ok(true)
}

pub(super) async fn disconnect_spotify_connection(state: &AppState) -> Result<(), String> {
    let _persistence_turn = state.spotify_connection_persistence_lock.lock().await;
    delete_spotify_connection_store(state)?;
    *state.spotify_connection.write().await = SpotifyConnectionStore::default();
    state
        .spotify_connection_generation
        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    Ok(())
}
