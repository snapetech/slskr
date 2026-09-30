use super::*;

pub(super) async fn preview_spotify_source_feed(
    state: &AppState,
    source_text: &str,
    provider_access_token: &str,
    limit: usize,
    token_url: &str,
    api_base: &str,
) -> Result<serde_json::Value, String> {
    let target = parse_spotify_source_target(source_text);
    let spotify = state.integration_settings.read().await.spotify.clone();
    let token =
        spotify_source_access_token(state, &spotify, &target, provider_access_token, token_url)
            .await?;
    if token.trim().is_empty() {
        let scope = if target.requires_user_token {
            target.scope_hint.as_str()
        } else {
            "Configure integrations.spotify.client_id/client_secret, connect a Spotify account, or provide a Spotify bearer token."
        };
        return Ok(serde_json::json!({
            "provider": "spotify",
            "sourceKind": target.kind,
            "sourceId": target.id,
            "totalRows": 0,
            "suggestionCount": 0,
            "duplicateCount": 0,
            "skippedCount": 0,
            "networkRequestCount": 0,
            "requiresAccessToken": true,
            "requiredScopeHint": scope,
            "suggestions": [],
            "skippedRows": [],
        }));
    }

    let base = api_base.trim_end_matches('/');
    let mut rows = Vec::new();
    let mut requests = 0_u64;
    match target.kind.as_str() {
        "playlist" => {
            requests +=
                fetch_spotify_playlist_tracks(&spotify, base, &token, &target.id, &mut rows, limit)
                    .await?;
        }
        "album" => {
            let album = spotify_get_json(
                &spotify,
                format!("{base}/albums/{}", url_encode(&target.id)),
                &token,
            )
            .await?;
            requests += 1;
            let album_name = album
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            for track in album
                .pointer("/tracks/items")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(row) =
                    spotify_track_row(track, "spotify:album", &target.id, Some(album_name))
                {
                    rows.push(row);
                }
            }
        }
        "track" => {
            let track = spotify_get_json(
                &spotify,
                format!("{base}/tracks/{}", url_encode(&target.id)),
                &token,
            )
            .await?;
            requests += 1;
            if let Some(row) = spotify_track_row(&track, "spotify:track", &target.id, None) {
                rows.push(row);
            }
        }
        "artist" => {
            let top = spotify_get_json(
                &spotify,
                format!(
                    "{base}/artists/{}/top-tracks?market={}",
                    url_encode(&target.id),
                    url_encode(&spotify.market)
                ),
                &token,
            )
            .await?;
            requests += 1;
            for track in top
                .get("tracks")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .take(limit)
            {
                if let Some(row) =
                    spotify_track_row(track, "spotify:artist-top-tracks", &target.id, None)
                {
                    rows.push(row);
                }
            }
        }
        "saved-tracks" => {
            requests += fetch_spotify_track_pages(
                &spotify,
                base,
                &token,
                "/me/tracks",
                "spotify:liked",
                &mut rows,
                limit,
            )
            .await?;
        }
        "saved-albums" => {
            let mut offset = 0_usize;
            while rows.len() < limit {
                let page_limit = 20_usize.min(limit - rows.len());
                let page = spotify_get_json(
                    &spotify,
                    format!("{base}/me/albums?limit={page_limit}&offset={offset}"),
                    &token,
                )
                .await?;
                requests += 1;
                let items = page
                    .get("items")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for item in &items {
                    let album = item.get("album").unwrap_or(&serde_json::Value::Null);
                    let name = album
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let artist = album
                        .get("artists")
                        .and_then(serde_json::Value::as_array)
                        .map(|artists| {
                            artists
                                .iter()
                                .filter_map(|artist| {
                                    artist.get("name").and_then(serde_json::Value::as_str)
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default();
                    rows.push(SpotifySourceRow {
                        title: name.clone(),
                        artist,
                        album: name.clone(),
                        source: "spotify:saved-albums".to_owned(),
                        source_id: album
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        provider_url: album
                            .pointer("/external_urls/spotify")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        raw_text: name,
                    });
                }
                if spotify_page_done(&page, items.len(), offset) {
                    break;
                }
                offset = offset.saturating_add(items.len());
            }
        }
        "followed-artists" => {
            let mut after = String::new();
            while rows.len() < limit {
                let page_limit = 50_usize.min(limit - rows.len());
                let after_query = if after.is_empty() {
                    String::new()
                } else {
                    format!("&after={}", url_encode(&after))
                };
                let page = spotify_get_json(
                    &spotify,
                    format!("{base}/me/following?type=artist&limit={page_limit}{after_query}"),
                    &token,
                )
                .await?;
                requests += 1;
                let artists = page
                    .pointer("/artists/items")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for artist in &artists {
                    let name = artist
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    rows.push(SpotifySourceRow {
                        title: name.clone(),
                        artist: name.clone(),
                        album: String::new(),
                        source: "spotify:followed-artists".to_owned(),
                        source_id: artist
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        provider_url: artist
                            .pointer("/external_urls/spotify")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        raw_text: name,
                    });
                }
                after = page
                    .pointer("/artists/cursors/after")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                if artists.is_empty() || after.is_empty() {
                    break;
                }
            }
        }
        "playlists" => {
            requests += fetch_spotify_playlist_collection(
                &spotify,
                base,
                &token,
                "/me/playlists",
                &mut rows,
                limit,
            )
            .await?;
        }
        "user" => {
            requests += fetch_spotify_playlist_collection(
                &spotify,
                base,
                &token,
                &format!("/users/{}/playlists", url_encode(&target.id)),
                &mut rows,
                limit,
            )
            .await?;
        }
        _ => {}
    }
    Ok(build_spotify_source_result(&target, rows, requests))
}

fn youtube_playlist_id(source_text: &str) -> String {
    reqwest::Url::parse(source_text.trim())
        .ok()
        .and_then(|url| {
            url.query_pairs()
                .find(|(key, _)| key == "list")
                .map(|(_, value)| value.into_owned())
        })
        .unwrap_or_default()
}

fn lastfm_target(source_text: &str) -> (String, &'static str, &'static str) {
    let Ok(url) = reqwest::Url::parse(source_text.trim()) else {
        return (String::new(), "", "");
    };
    let parts = url
        .path_segments()
        .map(|parts| parts.filter(|part| !part.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    let Some(user_index) = parts
        .iter()
        .position(|part| part.eq_ignore_ascii_case("user"))
    else {
        return (String::new(), "", "");
    };
    let user = parts.get(user_index + 1).copied().unwrap_or_default();
    let suffix = parts
        .iter()
        .skip(user_index + 2)
        .copied()
        .collect::<Vec<_>>()
        .join("/")
        .to_ascii_lowercase();
    if suffix.contains("loved") {
        (user.to_owned(), "user.getlovedtracks", "lastfm:loved")
    } else if suffix.contains("toptracks") {
        (user.to_owned(), "user.gettoptracks", "lastfm:top-tracks")
    } else {
        (
            user.to_owned(),
            "user.getrecenttracks",
            "lastfm:recent-tracks",
        )
    }
}

fn apple_source_ids(source_text: &str) -> Vec<String> {
    let Ok(url) = reqwest::Url::parse(source_text.trim()) else {
        return Vec::new();
    };
    if let Some(id) = url
        .query_pairs()
        .find(|(key, _)| key == "i")
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.trim().is_empty())
    {
        return vec![id];
    }
    url.path_segments()
        .into_iter()
        .flatten()
        .rev()
        .find(|segment| !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit()))
        .map(|segment| vec![segment.to_owned()])
        .unwrap_or_default()
}

fn listenbrainz_user(source_text: &str) -> String {
    let Ok(url) = reqwest::Url::parse(source_text.trim()) else {
        return String::new();
    };
    let parts = url
        .path_segments()
        .map(|parts| parts.filter(|part| !part.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    parts
        .iter()
        .position(|part| part.eq_ignore_ascii_case("user"))
        .and_then(|index| parts.get(index + 1))
        .copied()
        .unwrap_or_default()
        .to_owned()
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn preview_configured_provider_source_feed(
    state: &AppState,
    source_text: &str,
    source_kind: &str,
    limit: usize,
    youtube_api_base: &str,
    lastfm_api_base: &str,
    apple_api_base: &str,
    listenbrainz_api_base: &str,
) -> Result<Option<serde_json::Value>, String> {
    let Some(provider) = source_provider(source_text, source_kind) else {
        return Ok(None);
    };
    let integrations = state.integration_settings.read().await.clone();
    let timeout = integrations.spotify.timeout_seconds;
    let mut rows = Vec::new();
    let mut requests = 0_u64;
    match provider {
        "youtube" => {
            let playlist_id = youtube_playlist_id(source_text);
            if integrations.youtube.configured() && !playlist_id.is_empty() {
                let api_key = integrations.youtube.api_key.as_deref().unwrap_or_default();
                let mut page_token = String::new();
                while rows.len() < limit {
                    let page_limit = 50_usize.min(limit - rows.len());
                    let page_query = if page_token.is_empty() {
                        String::new()
                    } else {
                        format!("&pageToken={}", url_encode(&page_token))
                    };
                    let page = provider_get_json(
                        timeout,
                        format!(
                            "{}?part=snippet&maxResults={page_limit}&playlistId={}&key={}{page_query}",
                            youtube_api_base.trim_end_matches('/'),
                            url_encode(&playlist_id),
                            url_encode(api_key),
                        ),
                        "YouTube",
                    )
                    .await?;
                    requests += 1;
                    let items = page
                        .get("items")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    let starting_row = rows.len();
                    for (index, item) in items.iter().enumerate() {
                        let title = item
                            .pointer("/snippet/title")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let video_id = item
                            .pointer("/snippet/resourceId/videoId")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let mut row =
                            loose_source_row(title, "youtube:playlist", starting_row + index + 1);
                        if !video_id.is_empty() {
                            row.source_id = video_id.to_owned();
                            row.provider_url =
                                format!("https://www.youtube.com/watch?v={}", url_encode(video_id));
                        } else {
                            row.provider_url = source_text.to_owned();
                        }
                        rows.push(row);
                    }
                    page_token = page
                        .get("nextPageToken")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    if items.is_empty() || page_token.is_empty() {
                        break;
                    }
                }
            }
        }
        "lastfm" => {
            let (user, method, source) = lastfm_target(source_text);
            if integrations.lastfm.configured() && !user.is_empty() {
                let api_key = integrations.lastfm.api_key.as_deref().unwrap_or_default();
                let page = provider_get_json(
                    timeout,
                    format!(
                        "{}?method={method}&user={}&api_key={}&format=json&limit={}",
                        lastfm_api_base.trim_end_matches('/'),
                        url_encode(&user),
                        url_encode(api_key),
                        200_usize.min(limit),
                    ),
                    "Last.fm",
                )
                .await?;
                requests = 1;
                let tracks = page
                    .pointer("/recenttracks/track")
                    .or_else(|| page.pointer("/lovedtracks/track"))
                    .or_else(|| page.pointer("/toptracks/track"))
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for (index, track) in tracks.iter().take(limit).enumerate() {
                    let title = track
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    rows.push(SpotifySourceRow {
                        title: title.clone(),
                        artist: track
                            .pointer("/artist/name")
                            .or_else(|| track.pointer("/artist/#text"))
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        album: track
                            .pointer("/album/#text")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        source: source.to_owned(),
                        source_id: track
                            .get("mbid")
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| !value.trim().is_empty())
                            .map(str::to_owned)
                            .unwrap_or_else(|| (index + 1).to_string()),
                        provider_url: track
                            .get("url")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        raw_text: title,
                    });
                }
            }
        }
        "apple" => {
            for id in apple_source_ids(source_text).into_iter().take(2) {
                let page = provider_get_json(
                    timeout,
                    format!(
                        "{}?id={}&entity=song&limit={}",
                        apple_api_base.trim_end_matches('/'),
                        url_encode(&id),
                        200_usize.min(limit),
                    ),
                    "Apple Music",
                )
                .await?;
                requests += 1;
                for item in page
                    .get("results")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|item| {
                        item.get("trackName")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|value| !value.trim().is_empty())
                    })
                {
                    let title = item
                        .get("trackName")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    rows.push(SpotifySourceRow {
                        title: title.clone(),
                        artist: item
                            .get("artistName")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        album: item
                            .get("collectionName")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        source: "apple".to_owned(),
                        source_id: item
                            .get("trackId")
                            .and_then(serde_json::Value::as_i64)
                            .map(|value| value.to_string())
                            .unwrap_or_default(),
                        provider_url: item
                            .get("trackViewUrl")
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| !value.trim().is_empty())
                            .unwrap_or(source_text)
                            .to_owned(),
                        raw_text: title,
                    });
                }
                if rows.len() >= limit {
                    break;
                }
            }
        }
        "listenbrainz" => {
            let user = listenbrainz_user(source_text);
            if !user.is_empty() {
                let page = provider_get_json(
                    timeout,
                    format!(
                        "{}/1/user/{}/listens?count={}",
                        listenbrainz_api_base.trim_end_matches('/'),
                        url_encode(&user),
                        100_usize.min(limit),
                    ),
                    "ListenBrainz",
                )
                .await?;
                requests = 1;
                for (index, listen) in page
                    .pointer("/payload/listens")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .take(limit)
                    .enumerate()
                {
                    let metadata = listen
                        .get("track_metadata")
                        .unwrap_or(&serde_json::Value::Null);
                    let title = metadata
                        .get("track_name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    rows.push(SpotifySourceRow {
                        title: title.clone(),
                        artist: metadata
                            .get("artist_name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        album: metadata
                            .get("release_name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        source: "listenbrainz:listens".to_owned(),
                        source_id: metadata
                            .pointer("/additional_info/recording_msid")
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| !value.trim().is_empty())
                            .map(str::to_owned)
                            .unwrap_or_else(|| (index + 1).to_string()),
                        provider_url: source_text.to_owned(),
                        raw_text: title,
                    });
                }
            }
        }
        "bandcamp" => {}
        _ => unreachable!(),
    }
    if rows.is_empty() {
        if let Some(row) = fetch_provider_metadata_page(provider, source_text, timeout).await? {
            rows.push(row);
        }
        requests += 1;
    }
    Ok(Some(build_provider_source_result(
        provider,
        source_text,
        rows.into_iter().take(limit).collect(),
        requests,
    )))
}
