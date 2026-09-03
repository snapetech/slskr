#![allow(
    unused_variables,
    clippy::needless_borrow,
    clippy::needless_return,
    clippy::too_many_arguments,
    reason = "route groups share one compatibility-dispatch signature across frozen target profiles"
)]

use super::*;

const ROUTE_NOT_HANDLED: &str = "\0slskr-route-not-handled\0";
type RouteDispatchResult = Result<HttpResponse, String>;

fn route_is_unhandled(result: &RouteDispatchResult) -> bool {
    matches!(result, Err(error) if error == ROUTE_NOT_HANDLED)
}

fn complete_route_dispatch(response: RouteDispatchResult) -> RouteDispatchResult {
    response.inspect(|response| {
        let status_code: u16 = response
            .status
            .split(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500);
        tracing::complete_request_span(status_code);
    })
}

/// Keep the most frequent immutable controller reads out of the large route
/// group futures. Constructing a future for the full compatibility match is
/// measurable on sub-millisecond responses, while these routes do not depend
/// on the rest of the dispatch context.
fn route_dispatch_fast_read(
    method: &str,
    normalized_path: &str,
    route: &routing::ParsedRoute<'_>,
    state: &AppState,
) -> Option<RouteDispatchResult> {
    match (method, normalized_path) {
        ("GET", "/") => Some(Ok(index_html_response())),
        ("HEAD", "/") => Some(Ok(head_response(index_html_response()))),
        ("GET", "/dashboard") => Some(Ok(fallback_dashboard_response())),
        ("HEAD", "/dashboard") => Some(Ok(head_response(fallback_dashboard_response()))),
        ("GET", "/api/health") => Some(Ok(health_response(&state.config))),
        ("GET", "/health") => Some(Ok(health_response(&state.config))),
        ("HEAD", "/health") => Some(Ok(head_response(health_response(&state.config)))),
        ("GET", "/health/mesh") => Some(Ok(mesh_health_response(&state.config))),
        ("HEAD", "/health/mesh") => Some(Ok(head_response(mesh_health_response(&state.config)))),
        ("GET", "/api/version") => Some(Ok(version_response())),
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && matches!(
                    route.path,
                    "/api/slskdn/capabilities" | "/api/v0/slskdn/capabilities"
                ) =>
        {
            None
        }
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/capabilities" =>
        {
            None
        }
        ("GET", "/api/capabilities") => Some(Ok(capabilities_response())),
        _ => None,
    }
}

fn parse_download_filter_update(body: &str) -> Result<Vec<String>, String> {
    let payload = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|_| "Download filter payload must be valid JSON".to_owned())?;
    let terms = payload
        .get("exclude")
        .or_else(|| payload.get("terms"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Download filter payload must contain an exclude array".to_owned())?;
    if terms.len() > 100 {
        return Err("Download filter supports at most 100 terms".to_owned());
    }
    let mut normalized = Vec::with_capacity(terms.len());
    for term in terms {
        let term = term
            .as_str()
            .ok_or_else(|| "Download filter terms must be strings".to_owned())?
            .trim();
        if term.is_empty() {
            return Err("Download filter terms cannot be blank".to_owned());
        }
        if term.chars().count() > 256 {
            return Err("Download filter terms must be at most 256 characters".to_owned());
        }
        if !normalized.iter().any(|existing| existing == term) {
            normalized.push(term.to_owned());
        }
    }
    Ok(normalized)
}

fn share_grant_permissions_from_request(body: &str, versioned: bool) -> String {
    let payload = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
    if !versioned {
        return extract_json_string_field(body, "permissions")
            .unwrap_or_else(|| "download,stream".to_owned());
    }

    let bool_field = |camel: &str, snake: &str, default: bool| {
        payload
            .get(camel)
            .or_else(|| payload.get(snake))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default)
    };
    let permissions = [
        bool_field("allowDownload", "allow_download", true).then_some("download"),
        bool_field("allowStream", "allow_stream", true).then_some("stream"),
        bool_field("allowReshare", "allow_reshare", false).then_some("reshare"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if permissions.is_empty() {
        "none".to_owned()
    } else {
        permissions.join(",")
    }
}

pub(super) async fn update_download_filter(state: &AppState, body: &str) -> HttpResponse {
    if !effective_remote_configuration(state) {
        return controller_forbidden_response();
    }
    let terms = match parse_download_filter_update(body) {
        Ok(terms) => terms,
        Err(error) => return routing::bad_request_response(&error),
    };
    let existing = match crate::read_controller_compatibility_yaml(&state.config) {
        Ok(Some(text)) => text,
        Ok(None) => String::new(),
        Err(error) => return routing::service_unavailable_response(&error),
    };
    let mut yaml = if existing.trim().is_empty() {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    } else {
        match serde_yaml::from_str::<serde_yaml::Value>(&existing) {
            Ok(value @ serde_yaml::Value::Mapping(_)) => value,
            Ok(serde_yaml::Value::Null) => serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
            Ok(_) | Err(_) => {
                return routing::bad_request_response(
                    "Existing configuration is not a YAML mapping",
                )
            }
        }
    };
    let serde_yaml::Value::Mapping(root) = &mut yaml else {
        return routing::bad_request_response("Existing configuration is not a YAML mapping");
    };
    let filters_key = serde_yaml::Value::String("filters".to_owned());
    let download_key = serde_yaml::Value::String("download".to_owned());
    let exclude_key = serde_yaml::Value::String("exclude".to_owned());
    let filters = root
        .entry(filters_key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let serde_yaml::Value::Mapping(filters) = filters else {
        return routing::bad_request_response("filters must be a YAML mapping");
    };
    let download = filters
        .entry(download_key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let serde_yaml::Value::Mapping(download) = download else {
        return routing::bad_request_response("filters.download must be a YAML mapping");
    };
    download.insert(
        exclude_key,
        serde_yaml::Value::Sequence(terms.into_iter().map(serde_yaml::Value::String).collect()),
    );
    let yaml_text = match serde_yaml::to_string(&yaml) {
        Ok(text) => text,
        Err(error) => {
            return routing::service_unavailable_response(&format!(
                "failed to serialize configuration: {error}"
            ))
        }
    };
    // The shared controller YAML writer accepts the target's JSON-string
    // envelope (`"<yaml>"`), not an object wrapper.  Keep the download
    // policy endpoint on that same validated persistence path.
    apply_controller_yaml_upload(&serde_json::Value::String(yaml_text).to_string(), state).await
}

pub(super) async fn download_policy_response(
    state: &AppState,
    filenames: &[String],
) -> Option<HttpResponse> {
    let exclusions = effective_download_exclusions(state).await;
    let blocked = filenames
        .iter()
        .filter_map(|filename| {
            crate::download_filter::matching_exclusion(filename, &exclusions).map(|exclusion| {
                serde_json::json!({
                    "filename": filename,
                    "exclusion": exclusion,
                })
            })
        })
        .collect::<Vec<_>>();
    if blocked.is_empty() {
        return None;
    }
    Some(HttpResponse {
        status: "403 Forbidden",
        content_type: "application/json",
        body: serde_json::json!({
            "type": "download_blocked",
            "title": "Download blocked",
            "detail": "Every requested file matched a configured global download exclusion.",
            "blocked": blocked,
        })
        .to_string(),
    })
}

#[allow(
    dead_code,
    reason = "used by focused and bounded in-process route tests"
)]
pub(super) async fn route_http_request_with_headers(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: &RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    let request = routing::RouteRequest::new(method, path, authorization, body, headers);
    route_http_request_inner(request, state, None).await
}

#[allow(
    dead_code,
    reason = "the historical dispatcher owns the legacy server call path"
)]
pub(super) async fn route_http_request_with_state(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: Arc<AppState>,
    headers: &RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    let state_arc = state.clone();
    let request = routing::RouteRequest::new(method, path, authorization, body, headers);
    route_http_request_inner(request, &state, Some(state_arc)).await
}

fn versioned_share_rescan_response(state: &AppState, state_arc: Arc<AppState>) -> HttpResponse {
    let permit = match Arc::clone(&state.share_scans).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            // The legacy and native profile controllers expose PUT /api/v0/shares
            // as an asynchronous, idempotent trigger.  A request that arrives
            // while the scan is already running still returns an empty 200; the
            // separate /shares/rescan route retains the explicit busy error.
            return HttpResponse {
                status: "200 OK",
                content_type: "",
                body: String::new(),
            };
        }
    };
    tokio::spawn(async move {
        if let Ok(snapshot) = rebuild_share_index_with_permit(&state_arc, permit).await {
            record_event(
                &state_arc,
                "share.scan.completed",
                "shares",
                Some(format!("{} files", snapshot.entries.len())),
            )
            .await;
        }
    });
    HttpResponse {
        status: "200 OK",
        content_type: "",
        body: String::new(),
    }
}

fn disconnected_search_conflict_response(state: &AppState, display_state: &str) -> HttpResponse {
    let message = if state.config.controller_profile == ControllerProfile::Native {
        "Search could not be started".to_owned()
    } else {
        format!(
            "The server connection must be connected and logged in to perform a search (currently: {display_state})"
        )
    };
    HttpResponse {
        status: "409 Conflict",
        content_type: "application/json",
        body: serde_json::to_string(&message)
            .unwrap_or_else(|_| "\"Search could not be started\"".to_owned()),
    }
}

fn json_string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn musicbrainz_artist_credit_value(value: &serde_json::Value) -> String {
    value
        .get("artist-credit")
        .and_then(serde_json::Value::as_array)
        .map(|credits| {
            credits
                .iter()
                .map(|credit| {
                    let name = credit
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    let join_phrase = credit
                        .get("joinphrase")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    format!("{name}{join_phrase}")
                })
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .unwrap_or_default()
}

fn musicbrainz_artist_id_value(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/artist-credit/0/artist/id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn musicbrainz_position(value: Option<&serde_json::Value>, fallback: usize) -> usize {
    let Some(value) = value.and_then(serde_json::Value::as_str) else {
        return fallback;
    };
    value
        .split('.')
        .rev()
        .find_map(|part| part.parse::<usize>().ok())
        .unwrap_or(fallback)
}

fn musicbrainz_album_target_value(
    release: &serde_json::Value,
    fallback_release_id: &str,
    discogs_release_id: Option<&str>,
) -> serde_json::Value {
    let release_id = release
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback_release_id)
        .to_owned();
    let artist = musicbrainz_artist_credit_value(release);
    let artist_id = musicbrainz_artist_id_value(release);
    let mut tracks = Vec::new();
    let mut fallback_position = 1_usize;
    for media in release
        .get("media")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        for track in media
            .get("tracks")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(recording) = track.get("recording") else {
                continue;
            };
            let Some(recording_id) = recording
                .get("id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
            else {
                continue;
            };
            let title = json_string_field(track, "title")
                .or_else(|| json_string_field(recording, "title"))
                .unwrap_or_default();
            let duration = recording
                .get("length")
                .and_then(serde_json::Value::as_i64)
                .filter(|value| *value > 0);
            let recording_artist = musicbrainz_artist_credit_value(recording);
            let recording_artist = if recording_artist.is_empty() {
                artist.clone()
            } else {
                recording_artist
            };
            let isrc = recording
                .get("isrcs")
                .and_then(serde_json::Value::as_array)
                .and_then(|isrcs| isrcs.first())
                .and_then(serde_json::Value::as_str);
            let position = musicbrainz_position(track.get("position"), fallback_position);
            fallback_position = fallback_position.max(position.saturating_add(1));
            tracks.push(serde_json::json!({
                "musicBrainzRecordingId": recording_id,
                "position": position,
                "title": title,
                "artist": recording_artist,
                "duration": duration,
                "isrc": isrc,
            }));
        }
    }

    let discogs_release_id = discogs_release_id.map(str::to_owned).or_else(|| {
        release
            .get("relations")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find_map(|relation| {
                let relation_type = relation.get("type").and_then(serde_json::Value::as_str)?;
                if !relation_type.eq_ignore_ascii_case("discogs release")
                    && !relation_type.eq_ignore_ascii_case("discogs master")
                {
                    return None;
                }
                let resource = relation
                    .pointer("/url/resource")
                    .and_then(serde_json::Value::as_str)?;
                resource
                    .split("/release/")
                    .nth(1)
                    .and_then(|value| value.split('/').next())
                    .filter(|value| value.chars().all(|character| character.is_ascii_digit()))
                    .map(str::to_owned)
            })
    });

    serde_json::json!({
        "musicBrainzReleaseId": release_id,
        "discogsReleaseId": discogs_release_id,
        "title": release.get("title").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "artist": artist,
        "musicBrainzArtistId": artist_id,
        "metadata": {
            "releaseDate": release.get("date").and_then(serde_json::Value::as_str),
            "country": release.get("country").and_then(serde_json::Value::as_str),
            "label": release.pointer("/label-info/0/label/name").and_then(serde_json::Value::as_str),
            "status": release.get("status").and_then(serde_json::Value::as_str),
        },
        "tracks": tracks,
    })
}

fn musicbrainz_recording_target_value(
    recording: &serde_json::Value,
    fallback_id: &str,
) -> serde_json::Value {
    let recording_id = recording
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback_id);
    serde_json::json!({
        "musicBrainzRecordingId": recording_id,
        "position": 0,
        "title": recording.get("title").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "artist": musicbrainz_artist_credit_value(recording),
        "duration": recording.get("length").and_then(serde_json::Value::as_i64).filter(|value| *value > 0),
        "isrc": recording.get("isrcs").and_then(serde_json::Value::as_array).and_then(|isrcs| isrcs.first()).and_then(serde_json::Value::as_str),
    })
}

async fn musicbrainz_album_target_with_settings(
    settings: &MusicBrainzIntegrationSettings,
    release_id: &str,
) -> Result<Option<serde_json::Value>, String> {
    let Some(release) = musicbrainz_json_request(
        settings,
        &format!(
            "/release/{}?fmt=json&inc=recordings+artists+labels+discids+isrcs+url-rels",
            url_encode(release_id.trim())
        ),
    )
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(musicbrainz_album_target_value(
        &release, release_id, None,
    )))
}

async fn musicbrainz_discogs_album_target_with_settings(
    settings: &MusicBrainzIntegrationSettings,
    discogs_release_id: &str,
) -> Result<Option<serde_json::Value>, String> {
    let Some(search) = musicbrainz_json_request(
        settings,
        &format!(
            "/release/?query=discogsrelease:{}&fmt=json&limit=1",
            musicbrainz_query_encode(discogs_release_id.trim())
        ),
    )
    .await?
    else {
        return Ok(None);
    };
    let Some(release_id) = search
        .get("releases")
        .and_then(serde_json::Value::as_array)
        .and_then(|releases| releases.first())
        .and_then(|release| release.get("id"))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };
    let Some(release) = musicbrainz_json_request(
        settings,
        &format!(
            "/release/{}?fmt=json&inc=recordings+artists+labels+discids+isrcs+url-rels",
            url_encode(release_id)
        ),
    )
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(musicbrainz_album_target_value(
        &release,
        release_id,
        Some(discogs_release_id.trim()),
    )))
}

async fn musicbrainz_recording_target_with_settings(
    settings: &MusicBrainzIntegrationSettings,
    recording_id: &str,
) -> Result<Option<serde_json::Value>, String> {
    let Some(recording) = musicbrainz_json_request(
        settings,
        &format!(
            "/recording/{}?fmt=json&inc=artists+isrcs",
            url_encode(recording_id.trim())
        ),
    )
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(musicbrainz_recording_target_value(
        &recording,
        recording_id,
    )))
}

fn musicbrainz_hash_matches(
    discovery: &content_discovery::ContentDiscoveryStore,
    recording_id: &str,
) -> Vec<serde_json::Value> {
    discovery
        .hash_entries()
        .iter()
        .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(recording_id))
        .map(|entry| serde_json::to_value(entry).unwrap_or_else(|_| serde_json::json!({})))
        .collect()
}

fn musicbrainz_target_completion_value(
    album: &serde_json::Value,
    discovery: &content_discovery::ContentDiscoveryStore,
) -> serde_json::Value {
    let tracks = album
        .get("tracks")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut completed_tracks = 0_usize;
    let track_summaries = tracks
        .into_iter()
        .map(|track| {
            let recording_id = track
                .get("musicBrainzRecordingId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let matches = musicbrainz_hash_matches(discovery, &recording_id);
            let complete = !matches.is_empty();
            if complete {
                completed_tracks = completed_tracks.saturating_add(1);
            }
            let mut summary = track;
            summary["recordingId"] = serde_json::json!(recording_id);
            summary["durationMs"] = summary
                .get("duration")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            summary["complete"] = serde_json::json!(complete);
            summary["matches"] = serde_json::Value::Array(matches);
            summary
        })
        .collect::<Vec<_>>();
    let total_tracks = track_summaries.len();
    serde_json::json!({
        "releaseId": album.get("musicBrainzReleaseId").cloned().unwrap_or_default(),
        "title": album.get("title").cloned().unwrap_or_default(),
        "artist": album.get("artist").cloned().unwrap_or_default(),
        "releaseDate": album.pointer("/metadata/releaseDate").cloned().unwrap_or(serde_json::Value::Null),
        "discogsReleaseId": album.get("discogsReleaseId").cloned().unwrap_or(serde_json::Value::Null),
        "totalTracks": total_tracks,
        "completedTracks": completed_tracks,
        "tracks": track_summaries,
    })
}

fn normalized_search_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn musicbrainz_profile_allows_group(
    profile: &str,
    primary_type: &str,
    group: &serde_json::Value,
) -> bool {
    let primary_type = primary_type.to_ascii_lowercase();
    let secondary_types = group
        .get("secondary-types")
        .and_then(serde_json::Value::as_array)
        .map(|types| {
            types
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_ascii_lowercase)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    match profile {
        "ExtendedDiscography" => {
            matches!(primary_type.as_str(), "album" | "ep") || secondary_types.contains("live")
        }
        "AllReleases" => true,
        _ => primary_type == "album",
    }
}

pub(crate) async fn musicbrainz_discography_coverage_with_settings(
    state: &AppState,
    settings: &MusicBrainzIntegrationSettings,
    artist_id: &str,
    profile: &str,
    force_refresh: bool,
) -> Result<Option<serde_json::Value>, String> {
    let cache_key = format!("musicbrainz/discography/{artist_id}/{profile}");
    if !force_refresh {
        if let Some(cached) = state
            .controller_features
            .read()
            .await
            .get(&cache_key)
            .cloned()
        {
            return Ok(Some(cached));
        }
    }

    let Some(artist) = musicbrainz_json_request(
        settings,
        &format!("/artist/{}?fmt=json", url_encode(artist_id)),
    )
    .await?
    else {
        return Ok(None);
    };
    let artist_name = artist
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(artist_id)
        .to_owned();
    let Some(group_page) = musicbrainz_json_request(
        settings,
        &format!(
            "/release-group?artist={}&fmt=json&limit=50&offset=0",
            url_encode(artist_id)
        ),
    )
    .await?
    else {
        return Ok(None);
    };

    // A coverage request is interactive. Keep the first twelve selected
    // releases bounded while retaining the same release-group/profile model
    // as the controller. The cached result is used on subsequent reads.
    const MAX_COVERAGE_RELEASES: usize = 12;
    let groups = group_page
        .get("release-groups")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut selected = Vec::new();
    for group in groups {
        let primary_type = group
            .get("primary-type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Other")
            .to_owned();
        if !musicbrainz_profile_allows_group(profile, &primary_type, &group) {
            continue;
        }
        let Some(group_id) = json_string_field(&group, "id") else {
            continue;
        };
        // The release-group browse response does not include recordings. Ask
        // MusicBrainz for one representative release from each group with
        // recordings included, avoiding a second detail request per group.
        let Some(release_page) = (match musicbrainz_json_request(
            settings,
            &format!(
                "/release?release-group={}&fmt=json&limit=1&offset=0&inc=recordings+labels+discids+isrcs",
                url_encode(&group_id)
            ),
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                ::tracing::warn!(artist_id, group_id, %error, "MusicBrainz release-group coverage item skipped");
                continue;
            }
        }) else {
            continue;
        };
        let Some(release) = release_page
            .get("releases")
            .and_then(serde_json::Value::as_array)
            .and_then(|releases| releases.first())
            .cloned()
        else {
            continue;
        };
        let Some(release_id) = json_string_field(&release, "id") else {
            continue;
        };
        let album = musicbrainz_album_target_value(&release, &release_id, None);
        selected.push((group, primary_type, album));
        if selected.len() >= MAX_COVERAGE_RELEASES {
            break;
        }
    }

    let wishlist_texts = {
        let wishlist = state.wishlist.read().await;
        wishlist
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .map(|item| normalized_search_text(&item.search_text()))
            .filter(|value| !value.is_empty())
            .collect::<HashSet<_>>()
    };
    let discovery = state.content_discovery.read().await;
    let mut releases = Vec::new();
    for (group, primary_type, album) in selected {
        let tracks = album
            .get("tracks")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut covered_tracks = 0_usize;
        let track_values = tracks
            .into_iter()
            .map(|track| {
                let recording_id = track
                    .get("musicBrainzRecordingId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let matches = musicbrainz_hash_matches(&discovery, recording_id);
                let artist = track
                    .get("artist")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let title = track
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let search_text = normalized_search_text(&format!("{artist} {title}"));
                let (status, evidence) = if !matches.is_empty() {
                    covered_tracks = covered_tracks.saturating_add(1);
                    (
                        "MeshAvailable",
                        vec!["HashDb has verified content evidence for this recording."],
                    )
                } else if wishlist_texts.contains(&search_text) {
                    (
                        "WishlistSeeded",
                        vec!["Wishlist already has a matching search seed."],
                    )
                } else if recording_id.is_empty() {
                    ("Ambiguous", vec!["MusicBrainz recording id is missing."])
                } else {
                    ("Absent", Vec::new())
                };
                serde_json::json!({
                    "position": track.get("position").cloned().unwrap_or_default(),
                    "title": title,
                    "artist": artist,
                    "recordingId": recording_id,
                    "durationMs": track.get("duration").cloned().unwrap_or(serde_json::Value::Null),
                    "status": status,
                    "evidence": evidence,
                    "matches": matches,
                })
            })
            .collect::<Vec<_>>();
        let total_tracks = track_values.len();
        let release_group_id = json_string_field(&group, "id").unwrap_or_default();
        let release_id = album
            .get("musicBrainzReleaseId")
            .cloned()
            .unwrap_or_default();
        let missing = total_tracks.saturating_sub(covered_tracks);
        let complete = total_tracks > 0 && covered_tracks == total_tracks;
        releases.push(serde_json::json!({
            "releaseGroupId": release_group_id,
            "releaseId": release_id,
            "title": album.get("title").cloned().unwrap_or_default(),
            "releaseDate": album.pointer("/metadata/releaseDate").cloned().unwrap_or(serde_json::Value::Null),
            "type": primary_type,
            "totalTracks": total_tracks,
            "coveredTracks": covered_tracks,
            "complete": complete,
            "priorityScore": if missing == 0 { 0.0 } else { 1.0 },
            "graphDensityScore": 0.0,
            "evidenceScore": if total_tracks == 0 { 0.0 } else { covered_tracks as f64 / total_tracks as f64 },
            "gapScore": if total_tracks == 0 { 0.0 } else { missing as f64 / total_tracks as f64 },
            "priorityReasons": if missing == 0 { Vec::<String>::new() } else { vec![format!("{missing} missing track(s) remain in this release.")] },
            "tracks": track_values,
        }));
    }
    drop(discovery);

    let total_releases = releases.len();
    let complete_releases = releases
        .iter()
        .filter(|release| {
            release.get("complete").and_then(serde_json::Value::as_bool) == Some(true)
        })
        .count();
    let total_tracks = releases
        .iter()
        .filter_map(|release| {
            release
                .get("totalTracks")
                .and_then(serde_json::Value::as_u64)
        })
        .sum::<u64>();
    let covered_tracks = releases
        .iter()
        .filter_map(|release| {
            release
                .get("coveredTracks")
                .and_then(serde_json::Value::as_u64)
        })
        .sum::<u64>();
    let coverage_ratio = if total_tracks == 0 {
        0.0
    } else {
        covered_tracks as f64 / total_tracks as f64
    };
    let result = serde_json::json!({
        "artistId": artist_id,
        "artistName": artist_name,
        "profile": profile,
        "totalReleases": total_releases,
        "completeReleases": complete_releases,
        "totalTracks": total_tracks,
        "coveredTracks": covered_tracks,
        "coverageRatio": coverage_ratio,
        "promotionSuggested": total_tracks > 0 && coverage_ratio >= 0.70 && covered_tracks < total_tracks,
        "graphPriority": {
            "nodeCount": total_releases,
            "edgeCount": 0,
            "neighborhoodDensityScore": 0.0,
            "evidenceScore": coverage_ratio,
            "recommendedReleaseIds": releases.iter().filter(|release| release.get("complete").and_then(serde_json::Value::as_bool) != Some(true)).take(5).filter_map(|release| release.get("releaseId").and_then(serde_json::Value::as_str)).collect::<Vec<_>>(),
            "reasons": ["Priority is derived from local HashDb and Wishlist evidence."],
        },
        "releases": releases,
    });
    state
        .controller_features
        .write()
        .await
        .upsert(cache_key, result.clone())?;
    Ok(Some(result))
}

async fn route_http_request_inner(
    request: routing::RouteRequest<'_>,
    state: &AppState,
    state_arc: Option<Arc<AppState>>,
) -> Result<HttpResponse, String> {
    let routing::RouteRequest {
        method,
        path,
        authorization,
        body,
        headers,
    } = request;
    // Start request tracing
    let span = tracing::RequestSpan::new(
        method.to_string(),
        path.to_string(),
        None, // user_agent - would need to pass from connection
        None, // client_ip can be added from connection info
    );
    let _correlation_id = span.correlation_id.clone();
    tracing::set_request_span(span);

    let route = request.parsed();
    let request_is_versioned_v0 = route.path.starts_with("/api/v0/");

    let (raw_path, _) = crate::utils::split_request_target(path);
    let mut decoded_path = raw_path.to_owned();
    let mut traversal = false;
    for _ in 0..=2 {
        if crate::utils::contains_traversal_component(&decoded_path) {
            traversal = true;
            break;
        }
        let next = crate::utils::percent_decode(&decoded_path);
        if next == decoded_path {
            break;
        }
        decoded_path = next;
    }
    if traversal {
        return Ok(routing::bad_request_response(
            "One or more files in the request contain a dangerous path traversal segment",
        ));
    }

    // Normalize versioned paths before matching so static and dynamic routes
    // share the same dispatch behavior.
    let mut normalized_path = if let Some(versioned_path) = route
        .normalized_path
        .strip_prefix("/api/v0/")
        .or_else(|| route.normalized_path.strip_prefix("/api/v1/"))
        .or_else(|| route.normalized_path.strip_prefix("/api/v2/"))
    {
        format!("/api/{}", versioned_path)
    } else {
        route.normalized_path.to_string()
    };
    if route.path == "/api/server/status" {
        normalized_path = "/api/server/status".to_owned();
    }

    // native profile's mesh-gateway middleware short-circuits every /mesh request
    // while the feature is disabled, before auth or controller fallback can
    // change the wire response.  Keep the same disabled contract here.
    if (normalized_path == "/mesh" || normalized_path.starts_with("/mesh/"))
        && !state.config.mesh_gateway.enabled
    {
        return Ok(mesh_gateway_disabled_response());
    }
    if normalized_path == "/mesh" || normalized_path.starts_with("/mesh/") {
        if let Some(response) = mesh_gateway_auth_failure(state, &headers) {
            return Ok(response);
        }
    }

    if route.path == "/api/v0/share-grants/announce" && method == "POST" {
        if !e2e_share_announce_enabled() {
            return Ok(routing::not_found_response());
        }
        if !is_authorized(&state.config, authorization, headers.cookie.as_deref()) {
            return Ok(routing::unauthorized_response());
        }
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
            return Ok(routing::bad_request_response("invalid JSON body"));
        };
        let string_field = |field: &str| {
            payload
                .get(field)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let share_grant_id = string_field("shareGrantId");
        let collection_id = string_field("collectionId");
        let recipient_user_id = string_field("recipientUserId");
        let owner_endpoint = string_field("ownerEndpoint");
        if share_grant_id.is_empty()
            || collection_id.is_empty()
            || recipient_user_id.is_empty()
            || owner_endpoint.is_empty()
        {
            return Ok(routing::bad_request_response(
                "shareGrantId, collectionId, recipientUserId, and ownerEndpoint are required",
            ));
        }
        let allow_download = payload
            .get("allowDownload")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let allow_stream = payload
            .get("allowStream")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let allow_reshare = payload
            .get("allowReshare")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let permissions = [
            allow_download.then_some("download"),
            allow_stream.then_some("stream"),
            allow_reshare.then_some("reshare"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(",");
        let items = payload
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(MAX_INCOMING_SHARE_ITEMS)
            .collect::<Vec<_>>();
        let record = IncomingShareRecord {
            id: share_grant_id,
            owner_endpoint,
            owner_user_id: string_field("ownerUserId"),
            recipient_user_id,
            collection_id,
            collection_title: string_field("collectionTitle"),
            collection_description: string_field("collectionDescription"),
            collection_type: string_field("collectionType"),
            permissions,
            token: string_field("token"),
            expiry_utc: string_field("expiryUtc"),
            max_bitrate_kbps: payload
                .get("maxBitrateKbps")
                .and_then(serde_json::Value::as_u64),
            max_concurrent_streams: payload
                .get("maxConcurrentStreams")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            items,
            received_at: unix_timestamp(),
        };
        let json = record.json();
        let mut incoming = state.incoming_shares.write().await;
        incoming.upsert(record);
        drop(incoming);
        return Ok(routing::created_response(json.to_string()));
    }
    if route.path == "/api/v0/share-grants/incoming" && method == "GET" {
        if !is_authorized(&state.config, authorization, headers.cookie.as_deref()) {
            return Ok(routing::unauthorized_response());
        }
        let incoming = state.incoming_shares.read().await;
        let records = incoming
            .list()
            .iter()
            .map(IncomingShareRecord::json)
            .collect::<Vec<_>>();
        drop(incoming);
        return Ok(routing::ok_response(
            serde_json::to_string(&records).unwrap_or_else(|_| "[]".to_owned()),
        ));
    }

    let controller_metrics_request =
        method == "GET" && route.path == controller_metrics_path(&state.config);
    if controller_metrics_request {
        if !state.config.controller_metrics_enabled {
            return Ok(controller_swagger_not_found_response());
        }
        if let Some(response) = controller_metrics_auth_failure(&state.config, authorization) {
            return Ok(response);
        }
        normalized_path = "/api/metrics".to_owned();
    } else {
        if request_uses_revoked_jwt(state, authorization).await {
            tracing::complete_request_span(401);
            return Ok(routing::unauthorized_response());
        }

        if let Err(err) =
            routing::check_route_auth(&state.config, method, route.path, authorization, &headers)
        {
            let status = if err == "unauthorized" { 401 } else { 403 };
            tracing::complete_request_span(status);
            return Ok(match err {
                "unauthorized" => routing::unauthorized_response(),
                "csrf" => routing::forbidden_response("cross-site mutating request rejected"),
                _ => routing::forbidden_response("insufficient permissions for this route"),
            });
        }
    }

    if state.config.controller_profile == ControllerProfile::Native {
        let feature = state.media_services.read().await.features.clone();
        let feature_disabled = (!feature.collections_sharing
            && (normalized_path.starts_with("/api/collections")
                || normalized_path.starts_with("/api/share-grants")
                || normalized_path.starts_with("/api/sharegroups")
                || normalized_path.starts_with("/api/sharing")))
            || (!feature.streaming
                && (normalized_path.starts_with("/api/streams")
                    || normalized_path.starts_with("/api/peer-streams")
                    || normalized_path.starts_with("/api/mesh-streams")
                    || normalized_path.starts_with("/api/listening-party/radio")))
            || (!feature.streaming_relay_fallback
                && normalized_path.starts_with("/api/relay/streams"))
            || (!feature.identity_friends
                && (normalized_path.starts_with("/api/profile")
                    || normalized_path.starts_with("/api/contacts")
                    || normalized_path.starts_with("/api/identity")))
            || (!feature.solid && normalized_path.starts_with("/api/solid"))
            || (!feature.song_id && normalized_path.starts_with("/api/songid"))
            || (!feature.mesh && normalized_path.starts_with("/api/mesh"))
            || (!feature.dht
                && (normalized_path.starts_with("/api/dht")
                    || normalized_path.starts_with("/api/overlay")))
            || (!feature.pods
                && (normalized_path.starts_with("/api/pods")
                    || normalized_path.starts_with("/api/podcore")))
            || (!feature.social_federation
                && (normalized_path.starts_with("/api/federation")
                    || normalized_path.starts_with("/api/activitypub")
                    || normalized_path.starts_with("/api/taste-recommendations")
                    || normalized_path.starts_with("/actors/")
                    || normalized_path.starts_with("/.well-known/webfinger")))
            || (!feature.virtual_soulfind
                && (normalized_path.starts_with("/api/virtualsoulfind")
                    || normalized_path.starts_with("/api/bridge")))
            || (!feature.multi_source_downloads
                && (normalized_path.starts_with("/api/multisource")
                    || normalized_path.starts_with("/api/swarm")));
        if feature_disabled {
            return Ok(controller_swagger_not_found_response());
        }
    }

    if route.path == "/api/v0/application/dump"
        && ((state.config.controller_profile == ControllerProfile::Legacy && method == "POST")
            || (state.config.controller_profile == ControllerProfile::Native && method == "GET"))
    {
        return Ok(routing::method_not_allowed_response());
    }

    if (normalized_path.starts_with("/actors/") || normalized_path == "/.well-known/webfinger")
        && !social_federation_is_active(&state.config)
    {
        return Ok(controller_swagger_not_found_response());
    }

    if normalized_path.starts_with("/api/mesh")
        || normalized_path.starts_with("/api/dht")
        || normalized_path.starts_with("/api/overlay")
        || normalized_path.starts_with("/api/pod")
    {
        let advanced = state.advanced_networking.read().await;
        let mesh_nat_detect = normalized_path == "/api/mesh/nat/detect";
        let disabled = (normalized_path.starts_with("/api/mesh")
            && !mesh_nat_detect
            && (!advanced.mesh.enabled || !advanced.mesh.enable_overlay))
            || (mesh_nat_detect && (!advanced.mesh.enabled || !advanced.mesh.enable_stun))
            || (normalized_path.starts_with("/api/dht")
                && normalized_path != "/api/dht/status"
                && (!advanced.dht.enabled || !advanced.mesh.enable_dht))
            || (normalized_path.starts_with("/api/overlay/data")
                && (!advanced.mesh.enable_overlay || !advanced.overlay_data.enable))
            || (normalized_path.starts_with("/api/overlay")
                && (!advanced.mesh.enable_overlay || !advanced.overlay.enable));
        if disabled {
            return Ok(controller_swagger_not_found_response());
        }
        let remote_limit = advanced.mesh.effective_max_remote_payload_size();
        let network_limit = if advanced.security.enabled && advanced.security.network_guard.enabled
        {
            advanced.security.network_guard.max_message_size
        } else {
            usize::MAX
        };
        if body.len() > remote_limit.min(network_limit) {
            return Ok(HttpResponse {
                status: "413 Payload Too Large",
                content_type: "application/json; charset=utf-8",
                body: r#"{"error":"remote payload exceeds configured security limit"}"#.to_owned(),
            });
        }
    }

    if state.config.controller_profile == ControllerProfile::Native
        && method == "POST"
        && normalized_path == "/api/application/dump"
    {
        if !*state.diagnostics_allow_memory_dump.read().await {
            return Ok(HttpResponse {
                status: "404 Not Found",
                content_type: "",
                body: String::new(),
            });
        }
        let is_loopback = headers
            .remote_addr
            .is_some_and(|address| match address.ip() {
                std::net::IpAddr::V4(address) => address.is_loopback(),
                std::net::IpAddr::V6(address) => address
                    .to_ipv4_mapped()
                    .map_or_else(|| address.is_loopback(), |address| address.is_loopback()),
            });
        if !*state.diagnostics_allow_remote_dump.read().await && !is_loopback {
            return Ok(HttpResponse {
                status: "403 Forbidden",
                content_type: "",
                body: String::new(),
            });
        }
        return Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/octet-stream",
            body: String::new(),
        });
    }

    if unversioned_mutation_requires_api_version(method, route.path) {
        return Ok(HttpResponse {
            status: "400 Bad Request",
            content_type: "application/problem+json",
            body: serde_json::json!({
                "type": "https://docs.api-versioning.org/problems#unspecified",
                "title": "Unspecified API version",
                "status": 400,
                "detail": "An API version is required, but was not specified.",
                "code": "ApiVersionUnspecified",
            })
            .to_string(),
        });
    }

    if request_is_versioned_v0 {
        if let Some(response) = versioned_pods_blank_segment_response(method, route.path) {
            return Ok(response);
        }
        if let Some(response) = versioned_wishlist_invalid_id_response(method, route.path) {
            return Ok(response);
        }
    }

    if let Some(response) = virtual_soulfind_legacy_blank_id_response(&normalized_path) {
        return Ok(response);
    }

    if request_is_versioned_v0 {
        if let Some(response) = versioned_rooms_blank_segment_response(method, &normalized_path) {
            return Ok(response);
        }
    } else if let Some(response) =
        unversioned_rooms_compatibility_blank_id_response(method, &normalized_path)
    {
        return Ok(response);
    }

    if state.config.controller_profile == ControllerProfile::Native {
        if let Some(response) = bridge_transfer_blank_segment_response(method, route.path) {
            return Ok(response);
        }
    }

    if let Some(response) =
        controller_native_virtual_soulfind_read_failure_response(state, method, &normalized_path)
            .await
    {
        return Ok(response);
    }

    if let Some(response) =
        controller_native_wishlist_read_failure_response(state, method, route.path).await
    {
        return Ok(response);
    }

    if method == "GET" {
        if let Some(response) =
            controller_native_hashdb_read_failure_response(state, route.path).await
        {
            return Ok(response);
        }
        if let Some(response) =
            controller_native_backfill_candidates_read_failure_response(state, method, route.path)
                .await
        {
            return Ok(response);
        }
    }
    if method == "POST" {
        if let Some(response) =
            controller_native_hashdb_write_failure_response(state, route.path).await
        {
            return Ok(response);
        }
    }

    if let Some(response) =
        controller_native_transfer_storage_failure_response(state, method, route.path).await
    {
        return Ok(response);
    }
    if let Some(response) = controller_native_transfer_input_validation_response(
        state,
        method,
        route.path,
        route.query,
        body,
    ) {
        return Ok(response);
    }
    if let Some(response) =
        controller_native_transfer_auto_replace_status_response(state, method, route.path).await
    {
        return Ok(response);
    }
    if let Some(response) =
        controller_native_autoreplace_mutation_response(state, method, route.path).await
    {
        return Ok(response);
    }

    if method == "DELETE" && route.path == "/api/v0/session" {
        if let Some(token) = authorization.and_then(|value| value.strip_prefix("Bearer ")) {
            let now = unix_timestamp();
            if let Some(claims) = utils::verify_admin_jwt(&state.config, token, now) {
                state
                    .revoked_jwts
                    .write()
                    .await
                    .revoke(claims.jti, claims.exp, now);
            }
        }
        return Ok(routing::no_content_response());
    }

    if method == "GET" {
        if let Some(response) = versioned_get_failure_contract(route.path, route.query, state).await
        {
            return Ok(response);
        }
    }

    if state.config.controller_profile == ControllerProfile::Native {
        if let Some(response) =
            controller_native_search_query_validation(method, route.path, route.query)
        {
            return Ok(response);
        }
        if let Some(response) =
            controller_native_search_storage_failure_response(state, method, route.path).await
        {
            return Ok(response);
        }
    }

    if method == "DELETE" && state.config.controller_profile == ControllerProfile::Legacy {
        for prefix in ["/api/v0/transfers/downloads/", "/api/v0/transfers/uploads/"] {
            if let Some(value) = route.path.strip_prefix(prefix) {
                let segments = value.split('/').collect::<Vec<_>>();
                if segments.len() == 2
                    && segments[0] != "all"
                    && segments[1].parse::<u64>().is_err()
                {
                    return Ok(routing::bad_request_response("The request is invalid"));
                }
            }
        }
    }

    let relay_settings = state.advanced_networking.read().await.relay.clone();
    let relay_route = route.path.starts_with("/api/v0/relay/");
    if relay_route
        && relay_versioned_route_known(method, route.path)
        && !relay_versioned_route_allowed(&relay_settings, method, route.path)
    {
        return Ok(routing::forbidden_response(
            "feature is disabled by configuration",
        ));
    }
    if route.path.starts_with("/api/v0/")
        && matches!(
            (method, route.path),
            ("POST", "/api/v0/soulseek/mesh-rendezvous/interest")
                | ("DELETE", "/api/v0/soulseek/mesh-rendezvous/interest")
        )
    {
        return Ok(routing::forbidden_response(
            "feature is disabled by configuration",
        ));
    }

    if let Some(response) = versioned_relay_request(method, route.path, body, &headers, state).await
    {
        return Ok(response);
    }

    if method == "POST" && route.path == "/api/v0/podcore/signing/sign" {
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|payload| payload.get("message").cloned())
            .unwrap_or(serde_json::Value::Null);
        let sender_peer_id = message
            .get("senderPeerId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if !sender_peer_id.is_empty()
            && pod_request_peer_id(state).await.as_deref() != Some(sender_peer_id)
        {
            return Ok(routing::forbidden_response(
                "authenticated Pod membership is required",
            ));
        }
    }

    if method == "POST"
        && route.path.starts_with("/api/v0/podcore/membership/")
        && route.path.ends_with("/members")
    {
        let member_peer_id = extract_json_string_field(body, "peerId").unwrap_or_default();
        if pod_request_peer_id(state).await.as_deref() != Some(member_peer_id.as_str()) {
            return Ok(routing::forbidden_response(
                "authenticated Pod membership is required",
            ));
        }
    }

    // Keep the descriptor-unpublish path out of the large compatibility
    // mutation future.  The latter owns hundreds of unrelated branches and
    // can exceed the default Tokio worker stack before this small handler is
    // polled on a live HTTP request.
    if method == "DELETE" && normalized_path.starts_with("/api/mediacore/publish/descriptor/") {
        if let Some(response) = Box::pin(mediacore_mutation_response(
            method,
            &normalized_path,
            body,
            state,
        ))
        .await
        {
            return Ok(response);
        }
    }

    if method == "DELETE" && normalized_path.starts_with("/api/podcore/") {
        if let Some(response) = Box::pin(podcore_mutation_response(
            method,
            &normalized_path,
            route.query,
            body,
            state,
            route.path.starts_with("/api/v0/"),
        ))
        .await
        {
            return Ok(response);
        }
    }

    if method == "GET" && route.path == "/swagger/" {
        return Ok(controller_swagger_index_response(&state.config));
    }

    if state.config.controller_headless
        && matches!(method, "GET" | "HEAD")
        && (matches!(route.path, "/" | "/dashboard") || is_spa_navigation_path(route.path))
    {
        return Ok(controller_swagger_not_found_response());
    }

    if method == "GET" && audio_blank_recording_id_path(&normalized_path) {
        return Ok(routing::bad_request_response("RecordingId is required."));
    }

    if let Some(response) = versioned_conversation_mutation_validation_response(method, route.path)
    {
        return Ok(response);
    }

    if let Some(state_arc) = state_arc {
        if method == "PUT" && route.path == "/api/v0/shares" {
            return Ok(versioned_share_rescan_response(state, state_arc));
        }
    }

    if let Some(response) = route_dispatch_fast_read(method, &normalized_path, &route, state) {
        return complete_route_dispatch(response);
    }

    let extended_mutation = extended_controller_mutation_route(method, &normalized_path);
    let context = RouteDispatchContext {
        method,
        normalized_path: &normalized_path,
        authorization,
        body,
        state,
        route: &route,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    };
    let mut response = route_dispatch_group_0(&context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_1(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_2(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_3(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_4(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_5(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_7(&context).await;
    }
    complete_route_dispatch(response)
}

#[derive(Clone, Copy)]
struct RouteDispatchContext<'request, 'state> {
    method: &'request str,
    normalized_path: &'request str,
    authorization: Option<&'request str>,
    body: &'request str,
    state: &'state AppState,
    route: &'state routing::ParsedRoute<'request>,
    headers: &'state RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
}

include!("route_dispatch_group_0.rs");
include!("route_dispatch_group_1.rs");
include!("route_dispatch_group_2.rs");
include!("route_dispatch_group_3.rs");
include!("route_dispatch_group_4.rs");
include!("route_dispatch_group_5.rs");
include!("route_dispatch_group_6.rs");
include!("route_dispatch_group_7.rs");
