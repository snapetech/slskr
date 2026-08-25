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

async fn update_download_filter(state: &AppState, body: &str) -> HttpResponse {
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
    route_http_request_inner(method, path, authorization, body, state, headers, None).await
}

pub(super) async fn route_http_request_with_state(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: Arc<AppState>,
    headers: &RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    let state_arc = state.clone();
    route_http_request_inner(
        method,
        path,
        authorization,
        body,
        &state,
        headers,
        Some(state_arc),
    )
    .await
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
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: &RequestSecurityHeaders,
    state_arc: Option<Arc<AppState>>,
) -> Result<HttpResponse, String> {
    // Start request tracing
    let span = tracing::RequestSpan::new(
        method.to_string(),
        path.to_string(),
        None, // user_agent - would need to pass from connection
        None, // client_ip can be added from connection info
    );
    let _correlation_id = span.correlation_id.clone();
    tracing::set_request_span(span);

    let route = routing::parse_route(method, path);
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

    let extended_mutation = extended_controller_mutation_route(method, &normalized_path);
    let mut response = route_dispatch_group_0(
        method,
        &normalized_path,
        authorization,
        body,
        state,
        &route,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    )
    .await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_1(
            method,
            &normalized_path,
            authorization,
            body,
            state,
            &route,
            headers,
            extended_mutation,
            request_is_versioned_v0,
        )
        .await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_2(
            method,
            &normalized_path,
            authorization,
            body,
            state,
            &route,
            headers,
            extended_mutation,
            request_is_versioned_v0,
        )
        .await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_3(
            method,
            &normalized_path,
            authorization,
            body,
            state,
            &route,
            headers,
            extended_mutation,
            request_is_versioned_v0,
        )
        .await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_4(
            method,
            &normalized_path,
            authorization,
            body,
            state,
            &route,
            headers,
            extended_mutation,
            request_is_versioned_v0,
        )
        .await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_5(
            method,
            &normalized_path,
            authorization,
            body,
            state,
            &route,
            headers,
            extended_mutation,
            request_is_versioned_v0,
        )
        .await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6(
            method,
            &normalized_path,
            authorization,
            body,
            state,
            &route,
            headers,
            extended_mutation,
            request_is_versioned_v0,
        )
        .await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_7(
            method,
            &normalized_path,
            authorization,
            body,
            state,
            &route,
            headers,
            extended_mutation,
            request_is_versioned_v0,
        )
        .await;
    }
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

async fn route_dispatch_group_0(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("GET", "/") => Ok(index_html_response()),
        ("HEAD", "/") => Ok(head_response(index_html_response())),
        ("GET", "/dashboard") => Ok(fallback_dashboard_response()),
        ("HEAD", "/dashboard") => Ok(head_response(fallback_dashboard_response())),
        ("GET", "/api/health") => Ok(health_response(&state.config)),
        ("GET", "/health") => Ok(health_response(&state.config)),
        ("HEAD", "/health") => Ok(head_response(health_response(&state.config))),
        ("GET", "/health/mesh") => Ok(mesh_health_response(&state.config)),
        ("HEAD", "/health/mesh") => Ok(head_response(mesh_health_response(&state.config))),
        ("GET", "/api/version") => Ok(version_response()),
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && matches!(
                    route.path,
                    "/api/slskdn/capabilities" | "/api/v0/slskdn/capabilities"
                ) =>
        {
            Ok(native_capabilities_response(state).await)
        }
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/capabilities" =>
        {
            Ok(native_capability_controller_response(state).await)
        }
        ("GET", "/api/capabilities") => Ok(capabilities_response()),
        ("GET", "/.well-known/webfinger") => {
            Ok(activitypub_webfinger_response(route.query, state).await)
        }
        ("GET", path) if path.starts_with("/actors/") => {
            Ok(activitypub_get_response(path, route.query, state).await)
        }
        ("GET", "/mesh/http/services") => Ok(mesh_http_services_response(state).await),
        ("GET", "/api/security/bans") if route.path.starts_with("/api/v0/") => {
            let security = state.security.read().await;
            Ok(routing::ok_response(security.native_bans_json()))
        }
        ("GET", "/api/security/transports/status")
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native =>
        {
            // Matches the native profile's versioned TransportSelectorStatus contract.
            // The native slskR endpoint below intentionally retains its
            // historical selectedTransport/healthy shape.
            let configured = state
                .controller_features
                .read()
                .await
                .get("security/profile/security/transports")
                .and_then(|value| value.get("status"))
                .cloned();
            Ok(routing::ok_response(
                configured
                    .unwrap_or_else(|| {
                        serde_json::json!({
                            "selectedMode": "Direct",
                            "totalTransports": 1,
                            "availableTransports": 0,
                            "availableTransportTypes": [],
                            "lastConnectivityTest": chrono::Utc::now().to_rfc3339(),
                            "primaryTransportAvailable": false,
                            "fallbackAvailable": false,
                        })
                    })
                    .to_string(),
            ))
        }
        // native profile keeps the legacy /api/info compatibility controller as a
        // deliberately small projection.  Do not route it through slskR's
        // richer /api/application lifecycle DTO: clients use these fields to
        // identify the compatibility implementation and Soulseek state.
        ("GET", "/api/application")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/info" =>
        {
            let session = state.session.read().await;
            let connected = matches!(session.state, "connected" | "logged_in");
            let user = if connected {
                session
                    .username
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            } else {
                state
                    .config
                    .username
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "impl": "slskr",
                    "compat": "legacy",
                    "version": APP_VERSION,
                    "soulseek": {
                        "connected": connected,
                        "user": user,
                    },
                })
                .to_string(),
            ))
        }
        ("GET", "/api/application") => {
            let session = state.session.read().await;
            let share_lifecycle = state.share_lifecycle.read().await;
            let rooms = state.rooms.read().await;
            let users = state.users.read().await;
            let relay = state.relay.read().await;
            let runtime = state.runtime.read().await;
            let distributed_network = state.distributed_network.read().await;
            let distributed_settings = *state.soulseek_distributed_settings.read().await;
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = application_state_json(
                &session,
                &share_lifecycle,
                &rooms,
                &users,
                &relay,
                &runtime,
                &distributed_network,
                distributed_settings,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
                controller_version_json(state),
            );
            drop(runtime);
            drop(distributed_network);
            drop(relay);
            drop(users);
            drop(rooms);
            drop(share_lifecycle);
            drop(session);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/application/version/latest") => Ok(controller_version_latest_response(
            state,
            query_parameter(route.query, "forceCheck").as_deref() == Some("true"),
            controller_releases_url(state.config.controller_profile),
        )
        .await),
        ("GET", "/api/application/dump") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/octet-stream",
            body: String::new(),
        }),
        ("GET", "/api/application/version") => Ok(routing::ok_response(
            serde_json::json!(APP_VERSION).to_string(),
        )),
        ("PUT", "/api/application") => {
            if let Err(error) = mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_restart_requested(true).to_string()
            })
            .await
            {
                return Ok(routing::service_unavailable_response(&error));
            }
            schedule_lifecycle_command(state, LifecycleCommand::Restart);
            Ok(HttpResponse {
                status: "204 No Content",
                content_type: "",
                body: String::new(),
            })
        }
        ("DELETE", "/api/application") => {
            if let Err(error) = mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_restart_requested(false).to_string()
            })
            .await
            {
                return Ok(routing::service_unavailable_response(&error));
            }
            initiate_graceful_shutdown(state).await;
            Ok(routing::no_content_response())
        }
        ("POST", "/api/application/gc") => {
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_gc().to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            let _ = body;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "",
                body: String::new(),
            })
        }
        ("GET", "/api/server") => {
            let session = state.session.read().await;
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            drop(session);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/server/status") => {
            let session = state.session.read().await;
            let connected = session.state == "connected";
            Ok(routing::ok_response(
                serde_json::json!({
                    "connected": connected,
                    "state": if connected { "logged_in" } else { "disconnected" },
                    "username": if connected {
                        session.username.clone().unwrap_or_default()
                    } else {
                        String::new()
                    },
                })
                .to_string(),
            ))
        }
        ("PUT", "/api/server") | ("POST", "/api/server") => {
            let username = extract_json_string_field(body, "username")
                .or_else(|| extract_json_string_field(body, "Username"));
            let password = extract_json_string_field(body, "password")
                .or_else(|| extract_json_string_field(body, "Password"));
            let credential_store_mode = extract_json_string_field(body, "credentialStore")
                .or_else(|| extract_json_string_field(body, "credential_store"))
                .or_else(|| extract_json_string_field(body, "CredentialStore"))
                .map(|value| config::CredentialStoreMode::parse(&value))
                .transpose();
            let credential_store_mode = match credential_store_mode {
                Ok(mode) => mode,
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 500",
                    ))
                }
            };
            match (username, password) {
                (Some(username), Some(password)) => {
                    if username.trim().is_empty() || password.is_empty() {
                        return Ok(routing::bad_request_response(
                            "username and password are required",
                        ));
                    }
                    let credentials =
                        LoginCredentials::default_client(username.trim().to_owned(), password);
                    let credential_source = credential_store::store(
                        &state.config,
                        credential_store_mode
                            .as_ref()
                            .unwrap_or(&config::CredentialStoreMode::Memory),
                        &credentials,
                    );
                    let credential_source = match credential_source {
                        Ok(source) => source,
                        Err(error) => return Ok(routing::bad_request_response(&error)),
                    };
                    {
                        let mut runtime_credentials = state.runtime_credentials.write().await;
                        *runtime_credentials = Some(credentials);
                    }
                    record_daemon_log(
                        state,
                        logging::LogLevel::Info,
                        "session",
                        format!(
                            "received Soulseek credentials for {} using {} credential store",
                            redact_username(username.trim()),
                            credential_source
                        ),
                    )
                    .await;
                }
                (Some(_), None) | (None, Some(_)) => {
                    return Ok(routing::bad_request_response(
                        "username and password must be supplied together",
                    ));
                }
                (None, None) => {}
            }
            let previous_session;
            {
                let mut session = state.session.write().await;
                let already_connecting = matches!(session.state, "connecting" | "connected");
                if already_connecting {
                    let runtime_credentials_configured =
                        state.runtime_credentials.read().await.is_some();
                    let connected_endpoint = connected_server_address(state);
                    let body = controller_server_state_json(
                        &session,
                        &state.config,
                        runtime_credentials_configured,
                        connected_endpoint.as_deref(),
                    )
                    .to_string();
                    drop(session);
                    return Ok(if method == "PUT" && route.path.starts_with("/api/v0/") {
                        HttpResponse {
                            status: "205 Reset Content",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::accepted_response(body)
                    });
                }
                previous_session = session.clone();
                session.state = "connecting";
                session.updated_at = unix_timestamp();
            }
            if let Err(error) = send_session_command(state, SessionCommand::Connect).await {
                let mut session = state.session.write().await;
                if session.state == "connecting" {
                    *session = previous_session;
                }
                return Ok(routing::service_unavailable_response(&error));
            }
            record_daemon_log(
                state,
                logging::LogLevel::Info,
                "session",
                "connect requested from API",
            )
            .await;
            let session = state.session.read().await;
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            drop(session);
            if method == "PUT" && route.path.starts_with("/api/v0/") {
                Ok(routing::ok_response(String::new()))
            } else {
                Ok(routing::accepted_response(body))
            }
        }
        ("DELETE", "/api/server") => {
            if route.path.starts_with("/api/v0/") && !body.trim().is_empty() {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Null | serde_json::Value::String(_)) => {}
                    Ok(_) | Err(_) => {
                        return Ok(routing::bad_request_response(
                            "The disconnect message must be a JSON string",
                        ));
                    }
                }
            }
            let previous_session;
            {
                let mut session = state.session.write().await;
                if matches!(session.state, "disconnecting" | "disconnected") {
                    let runtime_credentials_configured =
                        state.runtime_credentials.read().await.is_some();
                    let connected_endpoint = connected_server_address(state);
                    let body = controller_server_state_json(
                        &session,
                        &state.config,
                        runtime_credentials_configured,
                        connected_endpoint.as_deref(),
                    )
                    .to_string();
                    drop(session);
                    return Ok(if route.path.starts_with("/api/v0/") {
                        routing::no_content_response()
                    } else {
                        routing::accepted_response(body)
                    });
                }
                previous_session = session.clone();
                session.state = "disconnecting";
                session.updated_at = unix_timestamp();
            }
            if let Err(error) = send_session_command(state, SessionCommand::Disconnect).await {
                let mut session = state.session.write().await;
                if session.state == "disconnecting" {
                    *session = previous_session;
                }
                return Ok(routing::service_unavailable_response(&error));
            }
            record_daemon_log(
                state,
                logging::LogLevel::Info,
                "session",
                "disconnect requested from API",
            )
            .await;
            let session = state.session.read().await;
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            drop(session);
            Ok(if route.path.starts_with("/api/v0/") {
                routing::no_content_response()
            } else {
                routing::accepted_response(body)
            })
        }
        ("GET", "/api/session/enabled") => {
            Ok(routing::ok_response(state.config.auth_required.to_string()))
        }
        ("POST", "/api/session") => {
            if route.path.starts_with("/api/v0/") {
                if state.config.controller_headless {
                    return Ok(HttpResponse {
                        status: "403 Forbidden",
                        content_type: "",
                        body: String::new(),
                    });
                }
                let username = extract_json_string_field(body, "username").unwrap_or_default();
                let password = extract_json_string_field(body, "password").unwrap_or_default();
                // The bundled Web UI authenticates with the configured static
                // API token in the Bearer header and intentionally does not
                // echo that token into the request body. Treat that already
                // authenticated path as a valid session bootstrap while
                // retaining the legacy username/password controller login.
                let static_api_token_session = authorization
                    .and_then(|value| value.strip_prefix("Bearer "))
                    .is_some_and(|token| state.config.api_token.as_deref() == Some(token));
                if username.trim().is_empty()
                    || (password.trim().is_empty() && !static_api_token_session)
                {
                    return Ok(routing::bad_request_response(
                        "Username and/or Password missing or invalid",
                    ));
                }
                let source = headers
                    .remote_addr
                    .map(|address| address.ip().to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                let attempt_keys = [
                    format!("source:{source}"),
                    format!("credential:{}\n{source}", username.to_ascii_lowercase()),
                ];
                let issued = unix_timestamp();
                if state
                    .login_attempts
                    .write()
                    .await
                    .is_locked(&attempt_keys, issued)
                {
                    return Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "application/json",
                        body: serde_json::json!("Too many failed login attempts. Try again later.")
                            .to_string(),
                    });
                }
                let configured_username = state
                    .controller_web_auth_username
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                let configured_password = state
                    .controller_web_auth_password
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                if !static_api_token_session
                    && (username != configured_username
                        || password.as_bytes() != configured_password.as_bytes())
                {
                    state
                        .login_attempts
                        .write()
                        .await
                        .record_failure(&attempt_keys, issued);
                    return Ok(routing::unauthorized_response());
                }
                state.login_attempts.write().await.clear(&attempt_keys);
                if static_api_token_session {
                    return Ok(routing::ok_response(
                        serde_json::json!({
                            "name": username,
                            "tokenType": "Bearer",
                            "token": "",
                            "issued": issued,
                            "notBefore": issued,
                            "expires": 0,
                        })
                        .to_string(),
                    ));
                }
                let Some((token, claims)) =
                    utils::issue_admin_jwt(&state.config, &username, issued)
                else {
                    return Ok(routing::service_unavailable_response(
                        "JWT signing credential is unavailable",
                    ));
                };
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "name": claims.name,
                        "tokenType": "Bearer",
                        "token": token,
                        "issued": claims.iat,
                        "notBefore": claims.nbf,
                        "expires": claims.exp,
                    })
                    .to_string(),
                ));
            }
            let issued = unix_timestamp();
            Ok(routing::ok_response(
                serde_json::json!({
                    "name": "slskr",
                    "tokenType": "ApiKey",
                    "token": "",
                    "tokenConfigured": state.config.api_token.is_some(),
                    "issued": issued,
                    "notBefore": issued,
                    "expires": 0,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/capabilities/peers") => {
            // Matches the oracle's CapabilitiesController contract: known
            // Native capability peers, not the generic connected-user list.
            let mesh = state.mesh.read().await;
            let peers = mesh.capability_service_peers_json();
            let count = peers.len();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "peers": peers,
                    "count": count,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/capabilities/mesh-peers") => {
            let mesh = state.mesh.read().await;
            let peers = mesh.capability_service_mesh_peers_json();
            let count = peers.len();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "peers": peers,
                    "count": count,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/network/stats") => {
            let include_peers = query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includePeers")
                .and_then(|(_, value)| parse_bool_value(&value))
                .unwrap_or(false);
            Ok(routing::ok_response(
                network_stats_value(state, include_peers).await.to_string(),
            ))
        }
        ("GET", "/api/hashdb/stats") => {
            let discovery = state.content_discovery.read().await;
            let persisted_entries = discovery.hash_entries().len();
            let latest_seq = discovery.latest_seq();
            let database_size_bytes = discovery.database_size_bytes();
            let mut peer_ids = discovery
                .shadow_records()
                .iter()
                .flat_map(|record| record.peer_ids.iter().cloned())
                .collect::<HashSet<_>>();
            drop(discovery);
            let features = state.controller_features.read().await;
            peer_ids.extend(hashdb_inventory_peer_ids(&features));
            let inventory = hashdb_inventory_records(&features, usize::MAX, false);
            drop(features);
            let total_flac_entries = inventory.len();
            let hashed_flac_entries = inventory
                .iter()
                .filter(|entry| {
                    entry
                        .get("hashStatusStr")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|status| status.eq_ignore_ascii_case("known"))
                })
                .count();
            let native_peers = state.mesh.read().await.capability_records.len();
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalPeers": peer_ids.len(),
                    "capabilityPeers": native_peers,
                    "totalFlacEntries": total_flac_entries,
                    "hashedFlacEntries": hashed_flac_entries,
                    "totalHashEntries": persisted_entries,
                    "currentSeqId": latest_seq,
                    "databaseSizeBytes": database_size_bytes,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/hashdb/entries") => {
            if route.path.starts_with("/api/v0/") {
                // Frozen HashDbController.GetEntries pages by sequence ID,
                // not by the local vector offset, and returns only HashDb
                // rows. Share-index projections belong to the unversioned
                // slskR compatibility helper below, not this controller.
                let limit = match query_parameter(route.query, "limit") {
                    Some(raw) => match raw.parse::<i64>() {
                        Ok(value) => usize::try_from(value.clamp(1, 1_000)).unwrap_or(1),
                        Err(_) => return Ok(routing::bad_request_response("limit is invalid")),
                    },
                    None => 100,
                };
                let offset = match query_parameter(route.query, "offset") {
                    Some(raw) => match raw.parse::<i64>() {
                        Ok(value) => u64::try_from(value.max(0)).unwrap_or(0),
                        Err(_) => return Ok(routing::bad_request_response("offset is invalid")),
                    },
                    None => 0,
                };
                let discovery = state.content_discovery.read().await;
                let (entries, _) = discovery.hash_entries_since_seq(offset, limit);
                let latest_seq = discovery.latest_seq();
                let count = entries.len();
                Ok(routing::ok_response(
                    serde_json::json!({
                        "latestSeq": latest_seq,
                        "entries": entries,
                        "count": count,
                    })
                    .to_string(),
                ))
            } else {
                let params = route.query.map(query_params).unwrap_or_default();
                let limit = params
                    .iter()
                    .find(|(key, _)| key == "limit")
                    .and_then(|(_, value)| value.parse::<usize>().ok())
                    .unwrap_or(100)
                    .clamp(1, 1_000);
                let offset = params
                    .iter()
                    .find(|(key, _)| key == "offset")
                    .and_then(|(_, value)| value.parse::<usize>().ok())
                    .unwrap_or(0);
                let discovery = state.content_discovery.read().await;
                let mut entries = discovery
                    .hash_entries()
                    .iter()
                    .map(|entry| {
                        serde_json::to_value(entry).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect::<Vec<_>>();
                let latest_seq = discovery.latest_seq();
                drop(discovery);
                let shares = state.shares.read().await;
                entries.extend(shares
                    .entries
                    .iter()
                    .filter(|entry| is_auto_retry_audio_file(&entry.filename))
                    .map(|entry| {
                        serde_json::json!({
                            "id": format!("share-{}-{}", entry.size, entry.filename),
                            "path": entry.filename,
                            "filename": entry.filename,
                            "extension": entry.extension,
                            "size": entry.size,
                            "hash": format!("{:016x}", stable_content_hash(&entry.filename, entry.size)),
                            "source": "share-index",
                        })
                    })
                    .collect::<Vec<_>>());
                let count = entries.len();
                let entries = entries
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .collect::<Vec<_>>();
                drop(shares);
                Ok(routing::ok_response(
                    serde_json::json!({
                        "latestSeq": latest_seq,
                        "entries": entries,
                        "count": count,
                        "offset": offset,
                        "limit": limit,
                    })
                    .to_string(),
                ))
            }
        }
        ("GET", path) if path.starts_with("/api/hashdb/hash/by-size/") => {
            let Some(raw_size) = path_segment_after(path, "/api/hashdb/hash/by-size/") else {
                return Ok(routing::not_found_response());
            };
            let Ok(size) = raw_size.parse::<u64>() else {
                return Ok(routing::bad_request_response(
                    "size must be a positive integer",
                ));
            };
            if size == 0 && !route.path.starts_with("/api/v0/") {
                return Ok(routing::bad_request_response(
                    "size must be a positive integer",
                ));
            }
            let discovery = state.content_discovery.read().await;
            let entries = discovery.hashes_by_size(size);
            Ok(routing::ok_response(
                serde_json::json!({
                    "count": entries.len(),
                    "entries": entries,
                })
                .to_string(),
            ))
        }
        ("GET", path) if path.starts_with("/api/hashdb/hash/") => {
            let Some(raw_key) = path_segment_after(path, "/api/hashdb/hash/") else {
                return Ok(routing::not_found_response());
            };
            let key = decoded_path_segment(raw_key).trim().to_owned();
            if route.path.starts_with("/api/v0/") && key.is_empty() {
                return Ok(routing::bad_request_response("flacKey is required"));
            }
            let discovery = state.content_discovery.read().await;
            Ok(discovery.lookup_hash(&key).map_or_else(
                || {
                    if route.path.starts_with("/api/v0/") {
                        HttpResponse {
                            status: "404 Not Found",
                            content_type: "application/json; charset=utf-8",
                            body: serde_json::json!({"error": "No hash found for key"}).to_string(),
                        }
                    } else {
                        routing::not_found_response()
                    }
                },
                |entry| {
                    routing::ok_response(
                        serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_owned()),
                    )
                },
            ))
        }
        ("POST", "/api/hashdb/hash") => {
            let entry = if route.path.starts_with("/api/v0/") {
                match hashdb_verification_entry_from_body(body) {
                    Ok(entry) => entry,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                }
            } else {
                match serde_json::from_str::<content_discovery::HashDbEntry>(body) {
                    Ok(entry) => entry,
                    Err(_) => return Ok(routing::bad_request_response("invalid hash entry")),
                }
            };
            let mut discovery = state.content_discovery.write().await;
            let previous_entries = discovery.hash_entries().to_vec();
            let previous_latest_seq = discovery.latest_seq();
            let result = discovery
                .merge_hash_entries(vec![entry])
                .map(|_| (discovery.latest_seq(), discovery.hash_entries().to_vec()));
            match result {
                Ok((latest_seq, entries)) => {
                    if let Err(error) = persist_hash_db_snapshot(state, &entries, latest_seq).await
                    {
                        let _ =
                            discovery.restore_hash_entries(previous_entries, previous_latest_seq);
                        return Ok(routing::internal_server_error_response(&error));
                    }
                    if route.path.starts_with("/api/v0/") {
                        Ok(routing::ok_response(
                            serde_json::json!({"stored": true}).to_string(),
                        ))
                    } else {
                        Ok(routing::ok_response(
                            serde_json::json!({
                                "stored": true,
                                "latestSeq": latest_seq,
                            })
                            .to_string(),
                        ))
                    }
                }
                Err(error) => Ok(content_discovery_error_response(state, error).await),
            }
        }
        ("POST", "/api/hashdb/sync/merge") => {
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(_) => return Ok(routing::bad_request_response("invalid hash merge request")),
            };
            let from_user = value
                .get("fromUser")
                .or_else(|| value.get("username"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("http-sync")
                .to_owned();
            let sync_settings = state
                .advanced_networking
                .read()
                .await
                .mesh_sync_security
                .clone();
            if state
                .mesh
                .write()
                .await
                .sync_is_quarantined(&from_user, unix_timestamp())
            {
                return Ok(HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "application/json; charset=utf-8",
                    body: serde_json::json!({"error":"mesh peer is quarantined"}).to_string(),
                });
            }
            let entries = match value.get("entries").cloned().and_then(|entries| {
                serde_json::from_value::<Vec<content_discovery::HashDbEntry>>(entries).ok()
            }) {
                Some(entries) => entries,
                None => return Ok(routing::bad_request_response("entries are required")),
            };
            let received = entries.len();
            let mut discovery = state.content_discovery.write().await;
            let previous_entries = discovery.hash_entries().to_vec();
            let previous_latest_seq = discovery.latest_seq();
            let result = if route.path.starts_with("/api/v0/") {
                discovery
                    .merge_hash_entries_from_mesh(entries)
                    .map(|(merged, _skipped)| {
                        (
                            merged,
                            discovery.latest_seq(),
                            discovery.hash_entries().to_vec(),
                        )
                    })
            } else {
                discovery.merge_hash_entries(entries).map(|merged| {
                    (
                        merged,
                        discovery.latest_seq(),
                        discovery.hash_entries().to_vec(),
                    )
                })
            };
            match result {
                Ok((merged, latest_seq, entries)) => {
                    if let Err(error) = persist_hash_db_snapshot(state, &entries, latest_seq).await
                    {
                        let _ =
                            discovery.restore_hash_entries(previous_entries, previous_latest_seq);
                        return Ok(routing::internal_server_error_response(&error));
                    }
                    Ok(routing::ok_response(
                        serde_json::json!({
                            "received": received,
                            "merged": merged,
                            "latestSeq": latest_seq,
                        })
                        .to_string(),
                    ))
                }
                Err(error) => {
                    let invalid = u32::try_from(received).unwrap_or(u32::MAX);
                    let rate_limited = state.mesh.write().await.record_invalid_sync_entries(
                        &from_user,
                        invalid,
                        &sync_settings,
                        unix_timestamp(),
                    );
                    if rate_limited {
                        record_peer_security_violation(state, &from_user).await;
                        Ok(HttpResponse {
                            status: "429 Too Many Requests",
                            content_type: "application/json; charset=utf-8",
                            body: serde_json::json!({"error":"mesh invalid-entry rate limit exceeded"}).to_string(),
                        })
                    } else {
                        Ok(content_discovery_error_response(state, error).await)
                    }
                }
            }
        }
        ("POST", "/api/virtualsoulfind/shadow-index/sync/merge") => {
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "invalid shadow-index merge request",
                    ))
                }
            };
            let records = match value
                .get("records")
                .or_else(|| value.get("entries"))
                .cloned()
                .and_then(|records| {
                    serde_json::from_value::<Vec<content_discovery::ShadowIndexRecord>>(records)
                        .ok()
                }) {
                Some(records) => records,
                None => return Ok(routing::bad_request_response("records are required")),
            };
            let realm_indexes = match value
                .get("realmIndexes")
                .or_else(|| value.get("realm_indexes"))
                .cloned()
            {
                Some(indexes) => match serde_json::from_value::<Vec<serde_json::Value>>(indexes) {
                    Ok(indexes) => indexes,
                    Err(_) => {
                        return Ok(routing::bad_request_response(
                            "realmIndexes must be an array of objects",
                        ))
                    }
                },
                None => Vec::new(),
            };
            let received = records.len();
            let mut discovery = state.content_discovery.write().await;
            match discovery.merge_shadow_records(records) {
                Ok(merged) => {
                    drop(discovery);
                    let indexes_merged = if realm_indexes.is_empty() {
                        0
                    } else {
                        match state
                            .realm_subject_indexes
                            .write()
                            .await
                            .merge_indexes(realm_indexes)
                        {
                            Ok(merged) => merged,
                            Err(error) => return Ok(routing::bad_request_response(&error)),
                        }
                    };
                    Ok(routing::ok_response(
                        serde_json::json!({
                            "received": received,
                            "merged": merged,
                            "realmIndexesMerged": indexes_merged,
                        })
                        .to_string(),
                    ))
                }
                Err(error) => Ok(content_discovery_error_response(state, error).await),
            }
        }
        (method, path) if path.starts_with("/api/virtualsoulfind/v2") => {
            Ok(route_virtual_soulfind_v2(method, path, route.query, body, state).await)
        }
        ("GET", path) if path.starts_with("/api/virtualsoulfind/shadow-index/") => {
            let Some(raw_recording_id) =
                path_segment_after(path, "/api/virtualsoulfind/shadow-index/")
            else {
                return Ok(routing::not_found_response());
            };
            let recording_id = decoded_path_segment(raw_recording_id);
            let discovery = state.content_discovery.read().await;
            if state.config.controller_profile == ControllerProfile::Native {
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "variants": virtual_soulfind_legacy_variants(&discovery, &recording_id),
                    })
                    .to_string(),
                ));
            }
            let record = discovery
                .shadow_records()
                .iter()
                .find(|record| record.recording_id.eq_ignore_ascii_case(&recording_id));
            Ok(routing::ok_response(
                record
                    .map_or_else(
                        || {
                            serde_json::json!({
                                "recordingId": recording_id,
                                "peerIds": [],
                                "totalPeerCount": 0,
                                "variants": [],
                            })
                        },
                        |record| {
                            serde_json::json!({
                                "recordingId": record.recording_id,
                                "peerIds": record.peer_ids,
                                "totalPeerCount": record.peer_ids.len(),
                                "variants": [],
                                "updatedAt": record.updated_at,
                            })
                        },
                    )
                    .to_string(),
            ))
        }
        ("POST", "/api/hashdb/backfill/from-history") => {
            Ok(hashdb_backfill_from_history_response(route.query, state).await)
        }
        ("POST", path) if path.starts_with("/api/hashdb/backfill/from-history") => {
            Ok(routing::not_found_response())
        }
        ("GET", "/api/mesh/stats") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let known_mesh_peers = mesh.candidate_usernames(&users).len();
            let rejected_messages = mesh.sync_rejected_messages;
            let quarantine_events = mesh.sync_quarantine_events;
            let quarantined_peers = mesh.sync_quarantined_until.len();
            let total_syncs = mesh.sync_merge_total;
            let successful_syncs = mesh.sync_merge_successful;
            let failed_syncs = mesh.sync_merge_failed;
            let total_entries_received = mesh.sync_entries_received;
            let total_entries_sent = mesh.sync_entries_sent;
            let skipped_entries = mesh.sync_skipped_entries;
            let rate_limit_violations = mesh.sync_rate_limit_violations;
            let total_entries_merged = mesh.sync_entries_merged;
            drop(mesh);
            drop(users);
            let current_seq_id = state.content_discovery.read().await.latest_seq();
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalSyncs": total_syncs,
                    "successfulSyncs": successful_syncs,
                    "failedSyncs": failed_syncs,
                    "totalEntriesReceived": total_entries_received,
                    "totalEntriesSent": total_entries_sent,
                    "totalEntriesMerged": total_entries_merged,
                    "rejectedMessages": rejected_messages,
                    "skippedEntries": skipped_entries,
                    "signatureVerificationFailures": 0,
                    "reputationBasedRejections": 0,
                    "rateLimitViolations": rate_limit_violations,
                    "quarantinedPeers": quarantined_peers,
                    "quarantineEvents": quarantine_events,
                    "proofOfPossessionFailures": 0,
                    "currentSeqId": current_seq_id,
                    "knownMeshPeers": known_mesh_peers,
                    "warnings": [],
                })
                .to_string(),
            ))
        }
        ("GET", "/api/mesh/peers") if route.path.starts_with("/api/v0/") => {
            let mesh = state.mesh.read().await;
            let peers = mesh
                .capability_records_json()
                .into_iter()
                .filter(|record| record["meshCapable"] == true)
                .collect::<Vec<_>>();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "count": peers.len(),
                    "peers": peers,
                    "overlay": [],
                })
                .to_string(),
            ))
        }
        ("GET", "/api/mesh/peers") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let mut value = serde_json::from_str::<serde_json::Value>(&mesh.users_json(&users))
                .unwrap_or_else(|_| serde_json::json!({}));
            value["peers"] = value["users"].clone();
            let active_connections = match state.private_gateway.as_ref() {
                Some(gateway) => gateway.active_connection_count().await,
                None => 0,
            };
            value["overlay"] = serde_json::json!({
                "enabled": state.private_gateway.is_some(),
                "activeConnections": active_connections,
            });
            let body = value.to_string();
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(body))
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn route_dispatch_group_1(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("GET", "/api/dht/peers") if route.path.starts_with("/api/v0/") => {
            let peers = match state.dht.as_ref() {
                Some(dht) => dht
                    .peers()
                    .await
                    .into_iter()
                    .map(|endpoint| {
                        serde_json::json!({
                            "address": endpoint.ip().to_string(),
                            "port": endpoint.port(),
                        })
                    })
                    .collect::<Vec<_>>(),
                None => Vec::new(),
            };
            Ok(routing::ok_response(
                serde_json::Value::Array(peers).to_string(),
            ))
        }
        ("GET", "/api/dht/peers") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let body = serde_json::Value::Array(
                mesh.capability_records_json()
                    .into_iter()
                    .chain(mesh.candidate_usernames(&users).into_iter().map(
                        |username| serde_json::json!({"username": username, "source": "soulseek"}),
                    ))
                    .collect(),
            )
            .to_string();
            Ok(routing::ok_response(body))
        }
        ("POST", path) if path.starts_with("/api/mesh/sync/") => {
            let Some(username) = path_segment_after(path, "/api/mesh/sync/") else {
                return Ok(routing::not_found_response());
            };
            if route.path.starts_with("/api/v0/") {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({"error": "Failed to sync with peer"}).to_string(),
                });
            }
            let username = decoded_path_segment(username);
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let candidate = mesh
                .candidate_usernames(&users)
                .into_iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&username));
            let watched = users.records.iter().any(|user| {
                user.username.eq_ignore_ascii_case(&username)
                    && (user.watched || user.status.as_deref() == Some("online"))
            });
            let capability = mesh
                .capability_records
                .iter()
                .any(|record| record.username.eq_ignore_ascii_case(&username));
            drop(mesh);
            drop(users);
            if let Err(error) =
                send_session_command(state, SessionCommand::ProbePeerCapability(username.clone()))
                    .await
            {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response(serde_json::json!({
                "success": true,
                "username": username,
                "queued": true,
                "probeQueued": true,
                "watched": watched,
                "capabilityRecord": capability,
                "status": if capability { "capable" } else if watched { "watched" } else if candidate { "candidate" } else { "probing" },
            }).to_string()))
        }
        ("GET", "/api/backfill/stats") => {
            let backfill = state.backfill.read().await;
            Ok(routing::ok_response(
                backfill.stats_json(unix_timestamp()).to_string(),
            ))
        }
        ("GET", "/api/backfill/config") => {
            let backfill = state.backfill.read().await;
            Ok(routing::ok_response(backfill.config_json().to_string()))
        }
        ("POST", "/api/backfill/enable") => {
            let enabled = match query_parameter(route.query, "enabled") {
                None => true,
                Some(value) => match value.parse::<bool>() {
                    Ok(enabled) => enabled,
                    Err(_) if route.path.starts_with("/api/v0/") => {
                        return Ok(routing::bad_request_response(
                            "The value 'enabled' is not valid.",
                        ));
                    }
                    Err(_) => true,
                },
            };
            state.backfill.write().await.enabled = enabled;
            Ok(routing::ok_response(
                serde_json::json!({ "enabled": enabled }).to_string(),
            ))
        }
        ("POST", "/api/backfill/idle") => {
            let mut backfill = state.backfill.write().await;
            backfill.is_idle = true;
            backfill.idle_since.get_or_insert_with(unix_timestamp);
            Ok(routing::ok_response(
                serde_json::json!({ "isIdle": true }).to_string(),
            ))
        }
        ("POST", "/api/backfill/busy") => {
            let mut backfill = state.backfill.write().await;
            backfill.is_idle = false;
            backfill.idle_since = None;
            Ok(routing::ok_response(
                serde_json::json!({ "isIdle": false }).to_string(),
            ))
        }
        ("POST", "/api/backfill/trigger") => Ok(routing::ok_response(
            run_backfill_cycle(state).await.to_string(),
        )),
        ("POST", "/api/backfill/file") => {
            let value =
                serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null);
            let peer_id = value
                .get("peerId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let path = value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let size = value
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if peer_id.is_empty() || path.is_empty() || size == 0 {
                return Ok(routing::bad_request_response(
                    "peerId, path, and size are required",
                ));
            }
            if let Some(response) =
                controller_native_backfill_file_write_failure_response(state, method, route.path)
                    .await
            {
                return Ok(response);
            }
            Ok(routing::ok_response(
                backfill_file(state, peer_id, path, size).await.to_string(),
            ))
        }
        ("GET", "/api/backfill/candidates") => {
            let limit = match query_parameter(route.query, "limit") {
                None => BACKFILL_DEFAULT_CANDIDATES,
                Some(value) => match value.parse::<usize>() {
                    Ok(value) if value > 0 => value,
                    Ok(_) => BACKFILL_DEFAULT_CANDIDATES,
                    Err(_) if route.path.starts_with("/api/v0/") => {
                        return Ok(routing::bad_request_response(
                            "The value 'limit' is not valid.",
                        ));
                    }
                    Err(_) => BACKFILL_DEFAULT_CANDIDATES,
                },
            };
            let candidates = backfill_candidates(state, limit)
                .await
                .iter()
                .map(BackfillCandidate::json)
                .collect::<Vec<_>>();
            let count = candidates.len();
            let json = if route.path.starts_with("/api/v0/") {
                serde_json::json!({
                    "count": count,
                    "candidates": candidates,
                })
            } else {
                serde_json::json!({
                    "candidates": candidates.clone(),
                    "entries": candidates,
                    "count": count,
                })
            };
            Ok(routing::ok_response(json.to_string()))
        }
        ("GET", "/api/dht/status") => {
            let mut value = if let Some(dht) = state.dht.as_ref() {
                serde_json::from_str::<serde_json::Value>(&dht.status_json().await)
                    .unwrap_or_else(|_| serde_json::json!({}))
            } else {
                serde_json::json!({
                    "dhtNodeCount": 0,
                    "isLanOnly": false,
                    "lanOnly": false,
                    "isBeaconCapable": false,
                    "isDhtRunning": false,
                    "verifiedBeaconCount": 0,
                })
            };
            let defaults = serde_json::json!({
                "isEnabled": state.dht.is_some(),
                "discoveredPeerCount": 0,
                "activeMeshConnections": 0,
                "totalPeersDiscovered": 0,
                "totalCandidateEndpointsSeen": 0,
                "totalCandidatesAccepted": 0,
                "totalCandidatesSkippedDhtPort": 0,
                "totalCandidatesSkippedDiscoveredCapacity": 0,
                "totalCandidatesDeferredConnectorCapacity": 0,
                "totalCandidatesSkippedReconnectBackoff": 0,
                "totalConnectionsAttempted": 0,
                "totalConnectionsSucceeded": 0,
                "lastAnnounceTime": serde_json::Value::Null,
                "lastDiscoveryTime": serde_json::Value::Null,
                "startedAt": serde_json::Value::Null,
                "uptimeSeconds": 0,
                "rendezvousInfohashes": [],
            });
            if let (Some(object), Some(defaults)) = (value.as_object_mut(), defaults.as_object()) {
                for (key, default) in defaults {
                    object.entry(key.clone()).or_insert_with(|| default.clone());
                }
            }
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: value.to_string(),
            })
        }
        ("POST", "/api/capabilities/negotiate") => Ok(capabilities_negotiate_response(body)),
        ("GET", "/swagger") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "301 Moved Permanently",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/index.html") => Ok(controller_swagger_index_response(&state.config)),
        ("GET", "/swagger/v0/swagger.json") => {
            if state.config.controller_swagger {
                let mut spec =
                    serde_json::from_str::<serde_json::Value>(&openapi::generate_openapi_json())
                        .unwrap_or_else(|_| serde_json::json!({}));
                spec["openapi"] = serde_json::json!("3.0.4");
                spec["info"]["title"] = if state.config.current_upstream_behavior {
                    serde_json::json!("slskR API")
                } else {
                    serde_json::json!(match state.config.controller_profile {
                        ControllerProfile::Legacy => "slskd",
                        ControllerProfile::Native => "slskr API",
                    })
                };
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json;charset=utf-8",
                    body: serde_json::to_string_pretty(&spec).unwrap_or_else(|_| "{}".to_owned()),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/index.js") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/javascript;charset=utf-8",
                    body: openapi::frozen_swagger_index_js("/swagger/v0/swagger.json"),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/swagger-ui.css") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/css;charset=utf-8",
                    body: "@import url('https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css');"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/index.css") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/css;charset=utf-8",
                    body: "html{box-sizing:border-box;overflow-y:scroll}body{margin:0;background:#fafafa}"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/swagger-ui-bundle.js") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/javascript;charset=utf-8",
                    body: "document.write('<script src=\"https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js\"><\\/script>');"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/swagger-ui-standalone-preset.js") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/javascript;charset=utf-8",
                    body: "document.write('<script src=\"https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-standalone-preset.js\"><\\/script>');"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        // Documentation endpoints
        ("GET", "/api/docs") | ("GET", "/api/v1/docs") | ("GET", "/api/v2/docs") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/html",
                body: openapi::swagger_ui_html("/api/openapi.json"),
            })
        }
        ("GET", "/api/openapi.json")
        | ("GET", "/api/v1/openapi.json")
        | ("GET", "/api/v2/openapi.json") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: openapi::generate_openapi_json(),
        }),
        ("GET", "/api/docs/index") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: serde_json::json!({
                "title": "slskr API Documentation",
                "version": "1.0.1",
                "docs": {
                    "swagger_ui": "/api/docs",
                    "openapi_spec": "/api/openapi.json",
                    "guides": {
                        "rate_limiting": "/docs/RATE_LIMITING.md",
                        "api_versioning": "/docs/API_VERSIONING.md",
                        "webhooks": "/docs/WEBHOOK_API.md"
                    }
                },
                "endpoints": {
                    "total": 202,
                    "by_method": {
                        "GET": 81,
                        "POST": 67,
                        "PUT": 6,
                        "DELETE": 15,
                        "PATCH": 1,
                        "OPTIONS": 32
                    }
                }
            })
            .to_string(),
        }),
        ("GET", "/api/docs/stats") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: serde_json::json!({
                "total_endpoints": 202,
                "api_versions": ["v0", "v1", "v2"],
                "categories": {
                    "health": 7,
                    "session": 5,
                    "search": 15,
                    "transfers": 18,
                    "users": 12,
                    "messages": 8,
                    "rooms": 15,
                    "shares": 8,
                    "webhooks": 6,
                    "collections": 22,
                    "wishlist": 18,
                    "contacts": 20,
                    "share_groups": 15,
                    "user_notes": 12,
                    "interests": 12
                },
                "features": {
                    "rate_limiting": {
                        "anonymous": "1000 req/min",
                        "authenticated": "5000 req/min"
                    },
                    "caching": "Cache-Control + ETag",
                    "compression": "gzip",
                    "cors": "Configurable",
                    "webhooks": "HMAC-SHA256"
                }
            })
            .to_string(),
        }),
        ("POST", "/api/batch") | ("POST", "/api/v1/batch") | ("POST", "/api/v2/batch") => {
            let (operations, config) = match batch::parse_batch_request(body) {
                Ok(parsed) => parsed,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            if let Err(error) = batch::validate_batch_operations(&operations) {
                return Ok(routing::bad_request_response(&error));
            }
            let mut results = Vec::new();
            for operation in operations {
                let result = match (operation.method.as_str(), operation.path.as_str()) {
                    ("GET", "/api/health") | ("GET", "/api/v0/health") => {
                        batch::create_success_result(
                            operation.id,
                            200,
                            health_response(&state.config).body,
                        )
                    }
                    ("GET", "/api/config") | ("GET", "/api/v0/config") => {
                        batch::create_success_result(
                            operation.id,
                            200,
                            state.config.sanitized_json(),
                        )
                    }
                    ("GET", "/api/capabilities") => batch::create_success_result(
                        operation.id,
                        200,
                        capabilities_response().body,
                    ),
                    ("GET", "/api/v0/capabilities")
                        if state.config.controller_profile == ControllerProfile::Native =>
                    {
                        batch::create_success_result(
                            operation.id,
                            200,
                            native_capability_controller_response(state).await.body,
                        )
                    }
                    ("GET", "/api/v0/capabilities") => batch::create_success_result(
                        operation.id,
                        200,
                        capabilities_response().body,
                    ),
                    ("GET", "/api/stats") | ("GET", "/api/v0/stats") => {
                        let session = state.session.read().await;
                        let shares = state.shares.read().await;
                        let searches = state.searches.read().await;
                        let users = state.users.read().await;
                        let browse = state.browse.read().await;
                        let messages = state.messages.read().await;
                        let rooms = state.rooms.read().await;
                        let transfers = state.transfers.read().await;
                        let body = format!(
                            "{{\"session\":{},\"listeners\":{{\"count\":1}},\"shares\":{},\"searches\":{},\"users\":{},\"browse\":{},\"messages\":{},\"rooms\":{},\"transfers\":{}}}",
                            session.summary_json(),
                            shares.summary_json(),
                            searches.summary_json(),
                            users.summary_json(),
                            browse.summary_json(),
                            messages.summary_json(),
                            rooms.summary_json(),
                            transfers.summary_json()
                        );
                        drop(transfers);
                        drop(rooms);
                        drop(messages);
                        drop(browse);
                        drop(users);
                        drop(searches);
                        drop(shares);
                        drop(session);
                        batch::create_success_result(operation.id, 200, body)
                    }
                    _ => batch::create_error_result(
                        operation.id,
                        format!(
                            "batch operation {} {} is not supported by the local executor",
                            operation.method, operation.path
                        ),
                    ),
                };
                let is_error = result.error.is_some();
                results.push(result);
                if is_error && !config.continue_on_error {
                    break;
                }
            }
            let executed = results
                .iter()
                .filter(|result| result.error.is_none())
                .count();
            let failed = results.len().saturating_sub(executed);
            let mut value =
                serde_json::from_str::<serde_json::Value>(&batch::format_batch_response(results))
                    .unwrap_or_else(|_| serde_json::json!({ "results": [] }));
            if let Some(object) = value.as_object_mut() {
                object.insert("accepted".to_owned(), serde_json::json!(true));
                object.insert("executed".to_owned(), serde_json::json!(executed));
                object.insert("failed".to_owned(), serde_json::json!(failed));
                object.insert("atomic".to_owned(), serde_json::json!(config.atomic));
                object.insert("timeoutMs".to_owned(), serde_json::json!(config.timeout_ms));
            }
            Ok(routing::accepted_response(value.to_string()))
        }
        ("GET", "/api/config") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: effective_sanitized_config_json(state).await,
        }),
        ("GET", "/api/stats") => {
            let session = state.session.read().await;
            let session_stats = session.summary_json();
            drop(session);

            let _listeners = state.listeners.read().await;
            drop(_listeners);

            let shares = state.shares.read().await;
            let share_stats = shares.summary_json();
            drop(shares);

            let searches = state.searches.read().await;
            let searches_stats = searches.summary_json();
            drop(searches);

            let users = state.users.read().await;
            let users_stats = users.summary_json();
            drop(users);

            let browses = state.browse.read().await;
            let browses_stats = browses.summary_json();
            drop(browses);

            let messages = state.messages.read().await;
            let messages_stats = messages.summary_json();
            drop(messages);

            let rooms = state.rooms.read().await;
            let rooms_stats = rooms.summary_json();
            drop(rooms);

            let transfers = state.transfers.read().await;
            let transfers_stats = transfers.summary_json();
            drop(transfers);

            let database = database_stats_value(state).await;
            let body = serde_json::json!({
                "session": serde_json::from_str::<serde_json::Value>(&session_stats).unwrap_or_else(|_| serde_json::json!({})),
                "listeners": {"count": 1},
                "shares": serde_json::from_str::<serde_json::Value>(&share_stats).unwrap_or_else(|_| serde_json::json!({})),
                "searches": serde_json::from_str::<serde_json::Value>(&searches_stats).unwrap_or_else(|_| serde_json::json!({})),
                "users": serde_json::from_str::<serde_json::Value>(&users_stats).unwrap_or_else(|_| serde_json::json!({})),
                "browse": serde_json::from_str::<serde_json::Value>(&browses_stats).unwrap_or_else(|_| serde_json::json!({})),
                "messages": serde_json::from_str::<serde_json::Value>(&messages_stats).unwrap_or_else(|_| serde_json::json!({})),
                "rooms": serde_json::from_str::<serde_json::Value>(&rooms_stats).unwrap_or_else(|_| serde_json::json!({})),
                "transfers": serde_json::from_str::<serde_json::Value>(&transfers_stats).unwrap_or_else(|_| serde_json::json!({})),
                "database": database,
            })
            .to_string();

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", "/api/telemetry") => {
            let session = state.session.read().await;
            let is_connected = session.state == "connected";
            let session_json = session.summary_json();
            drop(session);

            let listeners = state.listeners.read().await;
            let listeners_json = listeners.json();
            drop(listeners);

            let shares = state.shares.read().await;
            let shares_json = shares.summary_json();
            let share_cache_enabled = shares.cache_enabled;
            let share_cache_error = public_share_cache_error(shares.cache_error.as_deref());
            drop(shares);

            let searches = state.searches.read().await;
            let searches_json = searches.summary_json();
            drop(searches);

            let users = state.users.read().await;
            let users_json = users.summary_json();
            drop(users);

            let browse = state.browse.read().await;
            let browse_json = browse.summary_json();
            drop(browse);

            let messages = state.messages.read().await;
            let messages_json = messages.summary_json();
            drop(messages);

            let rooms = state.rooms.read().await;
            let rooms_json = rooms.summary_json();
            drop(rooms);

            let transfers = state.transfers.read().await;
            let transfers_json = transfers.summary_json();
            let transfer_state_healthy = transfers.state_error.is_none();
            let transfer_events_healthy = transfers.events_error.is_none();
            let transfer_state_error =
                public_transfer_state_error(transfers.state_error.as_deref());
            let transfer_events_error =
                public_transfer_events_error(transfers.events_error.as_deref());
            drop(transfers);

            let events = state.events.read().await;
            let event_count = events.records.len();
            let event_next_id = events.next_id;
            let event_history_limit = events.history_limit;
            drop(events);

            let runtime = state.runtime.read().await;
            let runtime_json = runtime.json_value();
            drop(runtime);

            let database = database_stats_value(state).await;
            let projections = database
                .get("projections")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let value = serde_json::json!({
                "health": {
                    "connected": is_connected,
                    "database": database.get("healthy").and_then(serde_json::Value::as_bool).unwrap_or(false)
                        || !database.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(false),
                    "transferState": transfer_state_healthy,
                    "transferEvents": transfer_events_healthy,
                    "eventsBuffered": event_count,
                },
                "service": {
                    "name": "slskr",
                    "version": APP_VERSION,
                },
                "storage": {
                    "share_cache_file": "share-index.tsv",
                    "share_cache_kind": "compatibility-debug",
                    "share_cache_enabled": share_cache_enabled,
                    "share_cache_error": share_cache_error,
                    "transfer_events_file": "transfer-events.tsv",
                    "transfer_state_error": transfer_state_error,
                    "transfer_events_error": transfer_events_error,
                },
                "database": database,
                "projections": projections,
                "runtime": runtime_json,
                "session": serde_json::from_str::<serde_json::Value>(&session_json).unwrap_or_else(|_| serde_json::json!({})),
                "listeners": serde_json::from_str::<serde_json::Value>(&listeners_json).unwrap_or_else(|_| serde_json::json!({})),
                "shares": serde_json::from_str::<serde_json::Value>(&shares_json).unwrap_or_else(|_| serde_json::json!({})),
                "searches": serde_json::from_str::<serde_json::Value>(&searches_json).unwrap_or_else(|_| serde_json::json!({})),
                "users": serde_json::from_str::<serde_json::Value>(&users_json).unwrap_or_else(|_| serde_json::json!({})),
                "browse": serde_json::from_str::<serde_json::Value>(&browse_json).unwrap_or_else(|_| serde_json::json!({})),
                "messages": serde_json::from_str::<serde_json::Value>(&messages_json).unwrap_or_else(|_| serde_json::json!({})),
                "rooms": serde_json::from_str::<serde_json::Value>(&rooms_json).unwrap_or_else(|_| serde_json::json!({})),
                "transfers": serde_json::from_str::<serde_json::Value>(&transfers_json).unwrap_or_else(|_| serde_json::json!({})),
                "events": {
                    "total": event_count,
                    "next_id": event_next_id,
                    "history_limit": event_history_limit,
                },
            });

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: value.to_string(),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/summary") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = if route.path.starts_with("/api/v0/") {
                controller_versioned_transfer_summary_report(route.query, &transfers)
            } else {
                controller_transfer_summary_report(route.query, &transfers)
            };
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/telemetry/reports/transfers/histogram") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = if route.path.starts_with("/api/v0/") {
                controller_versioned_transfer_histogram_report(route.query, &transfers)
            } else {
                Ok(controller_transfer_histogram_report(
                    route.query,
                    &transfers,
                ))
            };
            drop(transfers);
            Ok(match body {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/leaderboard") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let result = controller_transfer_leaderboard_report(route.query, &transfers);
            drop(transfers);
            Ok(match result {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", path) if path.starts_with("/api/telemetry/reports/transfers/users/") => {
            let Some(username) =
                path_segment_after(path, "/api/telemetry/reports/transfers/users/")
            else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = controller_user_transfer_report(&username, &transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/telemetry/reports/transfers/exceptions") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let result = controller_transfer_exceptions_report(route.query, &transfers);
            drop(transfers);
            Ok(match result {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/exceptions/pareto") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let result = controller_transfer_exceptions_pareto_report(route.query, &transfers);
            drop(transfers);
            Ok(match result {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/directories") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = controller_transfer_directories_report(route.query, &transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/metrics") => {
            let (session_connected, share_files, share_bytes) = {
                let session = state.session.read().await;
                let session_connected = if session.state == "connected" { 1 } else { 0 };
                drop(session);
                let _listeners = state.listeners.read().await;
                drop(_listeners);
                let shares = state.shares.read().await;
                let share_files = shares.entries.len();
                let share_bytes: u64 = shares.entries.iter().map(|e| e.size).sum();
                (session_connected, share_files, share_bytes)
            };
            let active_searches = {
                let searches = state.searches.read().await;
                searches
                    .records
                    .iter()
                    .filter(|record| record.status == "active")
                    .count()
            };
            let watched_users = {
                let users = state.users.read().await;
                users.records.iter().filter(|record| record.watched).count()
            };
            let browse_count = {
                let browse = state.browse.read().await;
                browse.records.len()
            };
            let message_count = {
                let messages = state.messages.read().await;
                messages.records.len()
            };
            let joined_rooms = {
                let rooms = state.rooms.read().await;
                rooms.records.iter().filter(|record| record.joined).count()
            };
            let (transfer_count, active_transfers) = {
                let transfers = state.transfers.read().await;
                let active = transfers
                    .entries
                    .iter()
                    .filter(|entry| is_active_transfer_status(&entry.status))
                    .count();
                (transfers.entries.len(), active)
            };
            let event_count = {
                let events = state.events.read().await;
                events.records.len()
            };
            let runtime = state.runtime.read().await;
            let runtime_profile_invites_created = runtime.profile_invites_created;
            let runtime_cache_warm_runs = runtime.cache_warm_runs;
            let runtime_backfill_runs = runtime.backfill_runs;
            let runtime_songid_runs = runtime.songid_runs;
            let runtime_lidarr_sync_runs = runtime.lidarr_sync_runs;
            let runtime_lidarr_manual_imports = runtime.lidarr_manual_imports;
            drop(runtime);
            let (database_stats, database_stats_available) = if let Some(db) = state.db.as_ref() {
                match db.get_stats().await {
                    Ok(stats) => (Some(stats), 1),
                    Err(_) => (None, 0),
                }
            } else {
                (None, 0)
            };
            let database_enabled = if state.db.is_some() { 1 } else { 0 };
            let persisted_searches = database_stats
                .as_ref()
                .map(|stats| stats.search_count)
                .unwrap_or(0);
            let persisted_search_results = database_stats
                .as_ref()
                .map(|stats| stats.search_result_count)
                .unwrap_or(0);
            let persisted_transfers = database_stats
                .as_ref()
                .map(|stats| stats.transfer_count)
                .unwrap_or(0);
            let persisted_transfer_events = database_stats
                .as_ref()
                .map(|stats| stats.transfer_event_count)
                .unwrap_or(0);
            let persisted_shares = database_stats
                .as_ref()
                .map(|stats| stats.share_file_count)
                .unwrap_or(0);
            let persisted_events = database_stats
                .as_ref()
                .map(|stats| stats.event_count)
                .unwrap_or(0);

            let metrics = format!(
                "# HELP slskr_session_connected Session connection status\n\
                 # TYPE slskr_session_connected gauge\n\
                 slskr_session_connected {}\n\
                 # HELP slskr_shares_files Number of shared files\n\
                 # TYPE slskr_shares_files gauge\n\
                 slskr_shares_files {}\n\
                 # HELP slskr_shares_bytes Total bytes shared\n\
                 # TYPE slskr_shares_bytes gauge\n\
                 slskr_shares_bytes {}\n\
                 # HELP slskr_searches_active Active search count\n\
                 # TYPE slskr_searches_active gauge\n\
                 slskr_searches_active {}\n\
                 # HELP slskr_users_watched Watched user count\n\
                 # TYPE slskr_users_watched gauge\n\
                 slskr_users_watched {}\n\
                 # HELP slskr_browse_cache Browse cache size\n\
                 # TYPE slskr_browse_cache gauge\n\
                 slskr_browse_cache {}\n\
                 # HELP slskr_messages_total Message count\n\
                 # TYPE slskr_messages_total counter\n\
                 slskr_messages_total {}\n\
                 # HELP slskr_rooms_joined Joined room count\n\
                 # TYPE slskr_rooms_joined gauge\n\
                 slskr_rooms_joined {}\n\
                 # HELP slskr_transfers Transfer count\n\
                 # TYPE slskr_transfers gauge\n\
                 slskr_transfers{{state=\"total\"}} {}\n\
                 slskr_transfers{{state=\"active\"}} {}\n\
                 # HELP slskr_events_total Recorded event count\n\
                 # TYPE slskr_events_total counter\n\
                 slskr_events_total {}\n\
                 # HELP slskr_runtime_operations_total Runtime compatibility operation counters\n\
                 # TYPE slskr_runtime_operations_total counter\n\
                 slskr_runtime_operations_total{{operation=\"profile_invite\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"cache_warm\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"backfill\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"songid\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"lidarr_sync\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"lidarr_manual_import\"}} {}\n\
                 # HELP slskr_database_enabled SQLite persistence availability\n\
                 # TYPE slskr_database_enabled gauge\n\
                 slskr_database_enabled {}\n\
                 # HELP slskr_database_stats_available Whether SQLite statistics were collected successfully\n\
                 # TYPE slskr_database_stats_available gauge\n\
                 slskr_database_stats_available {}\n\
                 # HELP slskr_database_rows Persisted SQLite row counts by store\n\
                 # TYPE slskr_database_rows gauge\n\
                 slskr_database_rows{{store=\"searches\"}} {}\n\
                 slskr_database_rows{{store=\"search_results\"}} {}\n\
                 slskr_database_rows{{store=\"transfers\"}} {}\n\
                 slskr_database_rows{{store=\"transfer_events\"}} {}\n\
                 slskr_database_rows{{store=\"shares\"}} {}\n\
                 slskr_database_rows{{store=\"events\"}} {}\n",
                session_connected,
                share_files,
                share_bytes,
                active_searches,
                watched_users,
                browse_count,
                message_count,
                joined_rooms,
                transfer_count,
                active_transfers,
                event_count,
                runtime_profile_invites_created,
                runtime_cache_warm_runs,
                runtime_backfill_runs,
                runtime_songid_runs,
                runtime_lidarr_sync_runs,
                runtime_lidarr_manual_imports,
                database_enabled,
                database_stats_available,
                persisted_searches,
                persisted_search_results,
                persisted_transfers,
                persisted_transfer_events,
                persisted_shares,
                persisted_events
            );

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: metrics,
            })
        }
        ("GET", "/api/events/records") => {
            let events = state.events.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: events.json(route.query),
            })
        }
        ("GET", "/api/events") | ("GET", "/api/events/slskd") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/events"
            {
                if let Some(raw) = query_parameter(route.query, "offset") {
                    if raw.parse::<i64>().is_err() {
                        return Ok(routing::bad_request_response(
                            "Offset must be greater than or equal to zero",
                        ));
                    }
                    if raw.parse::<i64>().is_ok_and(|value| value < 0) {
                        return Ok(routing::bad_request_response(
                            "Offset must be greater than or equal to zero",
                        ));
                    }
                }
                if let Some(raw) = query_parameter(route.query, "limit") {
                    if raw.parse::<i64>().is_err() {
                        return Ok(routing::bad_request_response(
                            "Limit must be greater than zero",
                        ));
                    }
                    if raw.parse::<i64>().is_ok_and(|value| value <= 0) {
                        return Ok(routing::bad_request_response(
                            "Limit must be greater than zero",
                        ));
                    }
                }
            }
            if let Some(response) = controller_events_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            if let Some(response) =
                controller_native_events_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let events = state.events.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: events.controller_json(route.query),
            })
        }
        ("POST", path) if path.starts_with("/api/events/") => {
            let Some(kind) = path_segment_after(path, "/api/events/") else {
                return Ok(routing::not_found_response());
            };
            if route.path.starts_with("/api/v0/") {
                let known = [
                    "DownloadFileComplete",
                    "DownloadDirectoryComplete",
                    "UploadFileComplete",
                    "PrivateMessageReceived",
                    "RoomMessageReceived",
                    "Noop",
                ];
                if !known.iter().any(|value| value.eq_ignore_ascii_case(kind)) {
                    return Ok(HttpResponse {
                        status: "400 Bad Request",
                        content_type: "application/json",
                        body: serde_json::json!("Unknown event type").to_string(),
                    });
                }
                let disambiguator = match serde_json::from_str::<String>(body) {
                    Ok(value) => value.trim().to_owned(),
                    Err(_) => {
                        return Ok(routing::bad_request_response(
                            "event disambiguator must be a JSON string",
                        ))
                    }
                };
                if disambiguator.len() > 128 {
                    return Ok(HttpResponse {
                        status: "400 Bad Request",
                        content_type: "application/json",
                        body: serde_json::json!("Disambiguator cannot exceed 128 characters")
                            .to_string(),
                    });
                }
            }
            let mut events = state.events.write().await;
            let previous = events.clone();
            let record_kind = if route.path.starts_with("/api/v0/") {
                kind
            } else {
                "compat.event"
            };
            let record = events.record(
                record_kind,
                kind,
                json_body_string(body).or_else(|| Some(body.to_owned())),
            );
            let count = events.records.len();
            if let Err(error) = persist_event_record_checked(state, &record).await {
                *events = previous;
                return Ok(
                    if state.config.controller_profile == ControllerProfile::Native {
                        routing::internal_server_error_response("Failed to raise event")
                    } else {
                        routing::service_unavailable_response(&error)
                    },
                );
            }
            drop(events);
            scripts::dispatch(
                state.integration_settings.read().await.scripts.clone(),
                state.config.state_dir.join("scripts"),
                state.config.controller_profile,
                kind,
                &serde_json::json!({}),
            );
            let response_body = serde_json::json!({
                "recorded": true,
                "event": record.controller_json(),
                "count": count,
            })
            .to_string();
            Ok(if route.path.starts_with("/api/v0/") {
                routing::created_response(response_body)
            } else {
                routing::ok_response(response_body)
            })
        }
        ("GET", "/api/logs") => {
            let events = state.events.read().await;
            let logs = events
                .records
                .iter()
                .rev()
                .filter(|event| event.kind == "log.created")
                .map(EventRecord::data_json)
                .collect::<Vec<_>>();
            drop(events);
            if route.path == "/api/v0/logs" {
                Ok(routing::ok_response(
                    serde_json::Value::Array(logs).to_string(),
                ))
            } else {
                Ok(routing::ok_response(
                    serde_json::json!({
                        "entries": logs,
                        "level": logging::LogConfig::level_name(*state.log_level.read().await),
                        "levels": ["Trace", "Debug", "Information", "Warning", "Error"],
                        "limit": EVENT_HISTORY_LIMIT,
                    })
                    .to_string(),
                ))
            }
        }
        ("GET", "/api/logs/level") => Ok(routing::ok_response(
            serde_json::json!({
                "level": logging::LogConfig::level_name(*state.log_level.read().await),
                "levels": ["Trace", "Debug", "Information", "Warning", "Error"],
                "source": "runtime",
            })
            .to_string(),
        )),
        ("PUT", "/api/logs/level") => {
            let requested = extract_json_string_field(body, "level")
                .or_else(|| json_body_string(body))
                .unwrap_or_default();
            let Some(level) = logging::LogConfig::parse_level(&requested) else {
                return Ok(routing::bad_request_response("invalid log level"));
            };
            {
                let mut current = state.log_level.write().await;
                *current = level;
            }
            record_daemon_log(
                state,
                logging::LogLevel::Info,
                "logging",
                format!(
                    "runtime log level changed to {}",
                    logging::LogConfig::level_name(level)
                ),
            )
            .await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "level": logging::LogConfig::level_name(level),
                    "updated": true,
                })
                .to_string(),
            ))
        }
        // WEBHOOK ENDPOINTS
        ("GET", "/api/webhooks") => {
            let webhooks = state.webhooks.read().await;
            let webhook_list: Vec<serde_json::Value> = webhooks
                .get_all()
                .iter()
                .map(|w| {
                    serde_json::json!({
                        "id": w.id,
                        "url": w.url,
                        "events": w.events.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                        "active": w.active,
                        "created_at": w.created_at,
                        "last_triggered": w.last_triggered,
                        "retry_count": w.retry_count,
                        "max_retries": w.max_retries,
                        "timeout_seconds": w.timeout_seconds,
                    })
                })
                .collect();
            drop(webhooks);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::to_string(&serde_json::json!({"webhooks": webhook_list}))
                    .unwrap_or_else(|_| "{}".to_string()),
            })
        }

        ("POST", "/api/webhooks") => {
            let url = match extract_json_string_field(body, "url") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("url is required")),
            };
            if url.len() > 2048 {
                return Ok(routing::bad_request_response("url is too long"));
            }
            if let Err(error) = webhooks::validate_webhook_url_for_registration(&url) {
                return Ok(routing::bad_request_response(&error.to_string()));
            }

            let events_str = extract_json_string_field(body, "events");
            let events = if let Some(ref e) = events_str {
                let mut events = Vec::new();
                for ev in e.split(',') {
                    let event = webhooks::WebhookEvent::from_wire(ev);
                    let Some(event) = event else {
                        return Ok(routing::bad_request_response("invalid webhook event"));
                    };
                    if !events.contains(&event) {
                        events.push(event);
                    }
                }
                events
            } else {
                vec![webhooks::WebhookEvent::SearchCreated]
            };

            if events.is_empty() {
                return Ok(routing::bad_request_response("no valid events specified"));
            }

            let secret = match extract_json_string_field(body, "secret") {
                Some(secret) => {
                    if let Err(error) = webhooks::validate_webhook_secret(&secret) {
                        return Ok(routing::bad_request_response(error));
                    }
                    secret
                }
                None => {
                    let Some(secret) = webhooks::Webhook::generate_secret() else {
                        return Ok(routing::service_unavailable_response(
                            "webhook secret generation unavailable",
                        ));
                    };
                    secret
                }
            };
            let webhook = webhooks::Webhook::new(url, events, secret.clone());

            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            let webhook_id = match webhooks.register(webhook.clone()) {
                Ok(id) => id,
                Err(_) => {
                    drop(webhooks);
                    return Ok(routing::bad_request_response("webhook limit reached"));
                }
            };
            let mutated = webhooks.clone();
            drop(webhooks);
            if let Err(error) = persist_webhook_checked(state, &webhook).await {
                rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }

            let response = serde_json::json!({
                "id": webhook_id,
                "secret": secret,
                "secretReturnedOnce": true,
                "status": "created"
            });

            Ok(routing::created_response(
                serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()),
            ))
        }

        ("DELETE", path)
            if path.starts_with("/api/webhooks/")
                && webhook_resource_id(path, "/api/webhooks/").is_some() =>
        {
            let webhook_id =
                webhook_resource_id(path, "/api/webhooks/").expect("guarded webhook resource path");
            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            if webhooks.unregister(webhook_id).is_some() {
                let mutated = webhooks.clone();
                drop(webhooks);
                if let Err(error) = persist_webhook_delete_checked(state, webhook_id).await {
                    rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(
                    serde_json::json!({"status": "deleted"}).to_string(),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }

        ("PATCH", path)
            if path.starts_with("/api/webhooks/")
                && webhook_resource_id(path, "/api/webhooks/").is_some() =>
        {
            let webhook_id =
                webhook_resource_id(path, "/api/webhooks/").expect("guarded webhook resource path");
            let Some(active) = extract_json_bool_field(body, "active") else {
                return Ok(routing::bad_request_response("active boolean is required"));
            };

            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            if let Some(webhook) = webhooks.get_mut(webhook_id) {
                webhook.active = active;
                let webhook = webhook.clone();
                let mutated = webhooks.clone();
                let updated = serde_json::json!({
                    "id": webhook.id,
                    "active": webhook.active,
                });
                drop(webhooks);
                if let Err(error) = persist_webhook_checked(state, &webhook).await {
                    rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(
                    serde_json::to_string(&updated).unwrap_or_else(|_| "{}".to_string()),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }

        // ADDITIONAL MISSING PATCH ENDPOINTS (Phase 5)
        ("PATCH", "/api/options") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            let model = serde_json::from_str::<serde_json::Value>(body);
            if model.as_ref().is_ok_and(|value| !value.is_object()) {
                return Ok(match state.config.controller_profile {
                    ControllerProfile::Legacy => HttpResponse {
                        status: "204 No Content",
                        content_type: "",
                        body: String::new(),
                    },
                    ControllerProfile::Native => options_model_binding_problem_response(),
                });
            }
            if model.is_err() {
                return Ok(options_model_binding_problem_response());
            }
            Ok(apply_controller_options_overlay(body, state).await)
        }

        ("PATCH", path)
            if path.starts_with("/api/library/health/issues/")
                && library_health_issue_id(path).is_some() =>
        {
            let versioned_contract = route.path.starts_with("/api/v0/");
            if versioned_contract
                && (body.trim().is_empty()
                    || !serde_json::from_str::<serde_json::Value>(body)
                        .is_ok_and(|value| value.is_object()))
            {
                return Ok(routing::bad_request_response(
                    "library issue update body must be an object",
                ));
            }
            let issue_id =
                library_health_issue_id(path).expect("guarded library health issue path");
            let artist = extract_json_string_field(body, "artist");
            let title = extract_json_string_field(body, "title");
            let kind = extract_json_string_field(body, "kind")
                .or_else(|| extract_json_string_field(body, "mediaKind"));
            let mut library = state.library.write().await;
            let previous = library.clone();
            let patched = library.patch_health_issue(issue_id, artist, title, kind);
            let patched_item_id = patched
                .as_ref()
                .and_then(|value| value.get("item_id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let remaining = library.health_issues().len();
            let response = patched
                .map(|mut value| {
                    value["remaining"] = serde_json::json!(remaining);
                    routing::ok_response(value.to_string())
                })
                .unwrap_or_else(|| {
                    routing::ok_response(
                        serde_json::json!({
                            "id": issue_id,
                            "updated": false,
                            "status": "not_found",
                            "remaining": remaining,
                        })
                        .to_string(),
                    )
                });
            let mutated = library.clone();
            drop(library);
            if let Some(item_id) = patched_item_id {
                let library = state.library.read().await;
                let item = library.get(&item_id);
                drop(library);
                if let Some(item) = item {
                    if let Err(error) = persist_library_item_checked(state, &item).await {
                        rollback_library_if_unchanged(state, previous, &mutated).await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            }
            Ok(if versioned_contract {
                routing::no_content_response()
            } else {
                response
            })
        }

        ("POST", path)
            if path.starts_with("/api/webhooks/")
                && path.ends_with("/test")
                && webhook_test_id(path, "/api/webhooks/").is_some() =>
        {
            let webhook_id =
                webhook_test_id(path, "/api/webhooks/").expect("guarded webhook test path");
            let webhooks = state.webhooks.read().await;
            if let Some(webhook) = webhooks.get(webhook_id) {
                let payload = webhooks::WebhookDispatcher::test_payload(
                    webhooks::WebhookEvent::SearchCreated,
                    "test webhook delivery",
                );
                let webhook_clone = webhook.clone();
                drop(webhooks);
                let Ok(delivery_permit) = Arc::clone(&state.webhook_deliveries).try_acquire_owned()
                else {
                    return Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "application/json",
                        body: "{\"error\":\"too many webhook deliveries in progress\"}".to_owned(),
                    });
                };

                tokio::spawn(async move {
                    let _delivery_permit = delivery_permit;
                    let _ = webhooks::WebhookDispatcher::send_webhook(
                        &webhook_clone.url,
                        &webhook_clone.secret,
                        &payload.to_string(),
                        webhook_clone.timeout_seconds,
                    )
                    .await;
                });

                Ok(routing::ok_response(
                    serde_json::json!({"status": "test_sent"}).to_string(),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }

        ("GET", path) if path.starts_with("/api/webhooks/") && path.ends_with("/logs") => {
            let Some(webhook_id) = path_segment_between(path, "/api/webhooks/", "/logs") else {
                return Ok(routing::not_found_response());
            };
            let limit = if let Some(q) = route.query {
                query_params(q)
                    .iter()
                    .find(|(k, _)| k == "limit")
                    .map(|(_, v)| parse_list_limit(v) as i32)
                    .unwrap_or(50)
            } else {
                50
            };

            if let Some(db) = &state.db {
                match db.get_webhook_logs(webhook_id, limit, 0).await {
                    Ok(logs) => {
                        let log_json = logs
                            .iter()
                            .map(|l| {
                                serde_json::json!({
                                    "id": l.id,
                                    "event": l.event,
                                    "correlation_id": l.correlation_id,
                                    "status": l.status,
                                    "response_status": l.response_status,
                                    "error_message": l.error_message,
                                    "timestamp": l.timestamp,
                                })
                            })
                            .collect::<Vec<_>>();

                        Ok(HttpResponse {
                            status: "200 OK",
                            content_type: "application/json",
                            body: serde_json::to_string(&serde_json::json!({"logs": log_json}))
                                .unwrap_or_else(|_| "{}".to_string()),
                        })
                    }
                    Err(_) => Ok(routing::bad_request_response("database error")),
                }
            } else {
                Ok(routing::bad_request_response("database not configured"))
            }
        }

        ("GET", "/api/shares") => {
            let shares = state.shares.read().await;
            let mut roots = shares
                .roots
                .iter()
                .map(controller_share_value)
                .collect::<Vec<_>>();
            if roots.is_empty() && !shares.entries.is_empty() {
                roots.push(serde_json::json!({
                    "localPath": "shares",
                    "id": "shares",
                    "alias": "shares",
                    "raw": "shares",
                    "remotePath": "shares",
                    "directories": 0,
                    "files": shares.entries.len(),
                    "bytes": shares.entries.iter().map(|entry| entry.size).sum::<u64>(),
                    "isExcluded": false,
                }));
            }
            let json = serde_json::json!({ "local": roots }).to_string();
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }
        ("GET", "/api/shares/catalog") => {
            let shares = state.shares.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: shares.catalog_json(route.query),
            })
        }
        ("PUT", "/api/shares") => {
            let rebuilt = match rebuild_share_index(state).await {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(share_rebuild_error_response(&error)),
            };
            let json = rebuilt.json();
            record_event(state, "share.scan.completed", "shares", None).await;
            if route.path.starts_with("/api/v0/") {
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "",
                    body: String::new(),
                });
            }
            Ok(routing::ok_response((!json.is_empty()).to_string()))
        }
        ("GET", "/api/files/downloads/directories")
        | ("GET", "/api/files/incomplete/directories")
        | ("GET", "/api/v0/files/downloads/directories")
        | ("GET", "/api/v0/files/incomplete/directories") => {
            if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            ) && query_bool_is_invalid(route.query, "recursive")
            {
                return Ok(routing::bad_request_response(
                    "The recursive query value must be a boolean",
                ));
            }
            let root = if normalized_path.contains("/files/downloads/") {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            if state.config.controller_profile == ControllerProfile::Legacy && !root.is_dir() {
                return Ok(file_storage_error_response(
                    STORAGE_DIRECTORY_NOT_FOUND_ERROR,
                ));
            }
            let options = StorageDirectoryListOptions::from_query(route.query);
            match controller_storage_directory_json(&root, None, options) {
                Ok(json) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: target_storage_directory_json(json, state.config.controller_profile),
                }),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("GET", path)
            if (path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/incomplete/directories/")
                || path.starts_with("/api/v0/files/downloads/directories/")
                || path.starts_with("/api/v0/files/incomplete/directories/")) =>
        {
            let Some((storage, resource, encoded_name)) =
                controller_file_storage_resource_path(path)
            else {
                return Ok(routing::not_found_response());
            };
            if resource != "directories" {
                return Ok(routing::not_found_response());
            }
            if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            ) && query_bool_is_invalid(route.query, "recursive")
            {
                return Ok(routing::bad_request_response(
                    "The recursive query value must be a boolean",
                ));
            }
            let root = if storage == "downloads" {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            let options = StorageDirectoryListOptions::from_query(route.query);
            match controller_storage_directory_json(&root, Some(encoded_name), options) {
                Ok(json) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: target_storage_directory_json(json, state.config.controller_profile),
                }),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/downloads/files/")
                || path.starts_with("/api/files/incomplete/directories/")
                || path.starts_with("/api/files/incomplete/files/")
                || path.starts_with("/api/v0/files/downloads/directories/")
                || path.starts_with("/api/v0/files/downloads/files/")
                || path.starts_with("/api/v0/files/incomplete/directories/")
                || path.starts_with("/api/v0/files/incomplete/files/") =>
        {
            if !effective_remote_file_management(state) {
                return Ok(HttpResponse {
                    status: "403 Forbidden",
                    content_type: "",
                    body: String::new(),
                });
            }
            let Some((storage, resource, encoded_name)) =
                controller_file_storage_resource_path(path)
            else {
                return Ok(routing::not_found_response());
            };
            let root = if storage == "downloads" {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            let delete_result = if resource == "directories" {
                delete_scoped_file_storage_path(&root, encoded_name, true)
            } else {
                delete_scoped_file_storage_path(&root, encoded_name, false)
            };
            match delete_result {
                Ok(true) => Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                }),
                Ok(false)
                    if resource == "files"
                        && matches!(
                            state.config.controller_profile,
                            ControllerProfile::Legacy | ControllerProfile::Native
                        ) =>
                {
                    Ok(HttpResponse {
                        status: "204 No Content",
                        content_type: "",
                        body: String::new(),
                    })
                }
                Ok(false) => Ok(routing::not_found_response()),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("GET", path) if path.starts_with("/api/files/") || path.starts_with("/api/v0/files/") => {
            let root_label = path
                .strip_prefix("/api/v0/files/")
                .or_else(|| path.strip_prefix("/api/files/"))
                .unwrap_or("");

            if root_label.is_empty() {
                return Ok(routing::not_found_response());
            }

            let mut extension_filter: Option<String> = None;
            let mut selected_folder = String::new();
            let mut folder_requested = false;
            let mut recursive = false;
            for (name, value) in query_params(route.query.unwrap_or_default()) {
                match name.as_str() {
                    "extension" => extension_filter = non_empty(value),
                    "folder" | "path" | "prefix" => {
                        folder_requested = true;
                        selected_folder = value.trim_matches('/').to_owned();
                    }
                    "recursive" => recursive = parse_bool_value(&value).unwrap_or(false),
                    _ => {}
                }
            }

            let filter = RecordListFilter::from_query(route.query);
            let shares = state.shares.read().await;

            let Some(root) = shares.roots.iter().find(|r| r.label == root_label) else {
                drop(shares);
                return Ok(routing::not_found_response());
            };

            let base_prefix = if selected_folder.is_empty() {
                root_label.to_owned()
            } else {
                format!("{}/{}", root_label, selected_folder)
            };
            let root_prefix = format!("{root_label}/");
            let base_child_prefix = format!("{base_prefix}/");
            let q = filter.q.as_deref();
            let folder_mode = folder_requested || recursive;

            let root_entries = shares
                .entries
                .iter()
                .filter(|entry| entry.filename.starts_with(&root_prefix))
                .collect::<Vec<_>>();

            let mut directory_summaries = BTreeMap::<String, (usize, u64)>::new();
            for entry in &root_entries {
                let Some(relative_to_base) = entry.filename.strip_prefix(&base_child_prefix) else {
                    continue;
                };
                let Some((child, _)) = relative_to_base.split_once('/') else {
                    continue;
                };
                if child.is_empty() {
                    continue;
                }
                let directory_path = if selected_folder.is_empty() {
                    child.to_owned()
                } else {
                    format!("{selected_folder}/{child}")
                };
                if q.is_some_and(|q| {
                    !directory_path.to_ascii_lowercase().contains(q)
                        && !format!("{root_label}/{directory_path}")
                            .to_ascii_lowercase()
                            .contains(q)
                }) {
                    continue;
                }
                if extension_filter
                    .as_deref()
                    .is_some_and(|ext| entry.extension != ext)
                {
                    continue;
                }
                let summary = directory_summaries.entry(directory_path).or_default();
                summary.0 += 1;
                summary.1 += entry.size;
            }

            let mut entries: Vec<_> = root_entries
                .into_iter()
                .filter(|entry| {
                    if folder_mode {
                        if recursive {
                            entry.filename.starts_with(&base_child_prefix)
                        } else {
                            virtual_folder(&entry.filename) == base_prefix
                        }
                    } else {
                        entry.filename.starts_with(&root_prefix)
                    }
                })
                .filter(|e| {
                    extension_filter
                        .as_deref()
                        .is_none_or(|ext| e.extension == ext)
                })
                .filter(|entry| {
                    q.is_none_or(|q| {
                        entry
                            .filename
                            .strip_prefix(&root_prefix)
                            .unwrap_or(&entry.filename)
                            .to_ascii_lowercase()
                            .contains(q)
                            || entry.filename.to_ascii_lowercase().contains(q)
                    })
                })
                .collect();

            let filtered_count = entries.len();
            let directory_count = directory_summaries.len();
            let total_bytes = entries.iter().map(|entry| entry.size).sum::<u64>();

            entries = entries
                .into_iter()
                .skip(filter.offset)
                .take(filter.limit.unwrap_or(usize::MAX))
                .collect();

            let entries_json = entries
                .iter()
                .map(|entry| {
                    let path = if folder_mode {
                        entry
                            .filename
                            .strip_prefix(&base_child_prefix)
                            .unwrap_or("")
                    } else {
                        entry.filename.strip_prefix(&root_prefix).unwrap_or("")
                    };
                    format!(
                        "{{\"type\":\"file\",\"path\":\"{}\",\"virtual_path\":\"{}\",\"size\":{},\"extension\":\"{}\"}}",
                        json_escape(path),
                        json_escape(&entry.filename),
                        entry.size,
                        json_escape(&entry.extension)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let directories_json = directory_summaries
                .iter()
                .map(|(directory, (file_count, total_bytes))| {
                    let path = if selected_folder.is_empty() {
                        directory.as_str()
                    } else {
                        directory
                            .strip_prefix(&format!("{selected_folder}/"))
                            .unwrap_or(directory)
                    };
                    format!(
                        "{{\"type\":\"directory\",\"name\":\"{}\",\"path\":\"{}\",\"virtual_path\":\"{}/{}\",\"file_count\":{},\"total_bytes\":{}}}",
                        json_escape(path),
                        json_escape(path),
                        json_escape(root_label),
                        json_escape(directory),
                        file_count,
                        total_bytes
                    )
                })
                .collect::<Vec<_>>()
                .join(",");

            let response_body = format!(
                "{{\"label\":\"{}\",\"folder\":{},\"recursive\":{},\"entries\":[{}],\"directories\":[{}],\"count\":{},\"filtered_count\":{},\"directory_count\":{},\"total_bytes\":{},\"offset\":{},\"limit\":{}}}",
                json_escape(&root.label),
                json_option((!selected_folder.is_empty()).then_some(selected_folder.as_str())),
                recursive,
                entries_json,
                directories_json,
                root.files,
                filtered_count,
                directory_count,
                total_bytes,
                filter.offset,
                json_usize_option(filter.limit)
            );

            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: response_body,
            })
        }
        ("POST", "/api/shares/rescan") => {
            let snapshot = match rebuild_share_index(state).await {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(share_rebuild_error_response(&error)),
            };
            record_event(
                state,
                "share.scan.completed",
                "shares",
                Some(format!("{} files", snapshot.entries.len())),
            )
            .await;
            Ok(HttpResponse {
                status: "202 Accepted",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn route_dispatch_group_2(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("GET", "/api/session") => {
            let snapshot = state.session.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("POST", "/api/session/connect") => {
            if let Err(error) = send_session_command(state, SessionCommand::Connect).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/ping") => {
            if let Err(error) = send_session_command(state, SessionCommand::Ping).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/disconnect") => {
            if let Err(error) = send_session_command(state, SessionCommand::Disconnect).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/privileges/check") => {
            if let Err(error) = send_session_command(state, SessionCommand::CheckPrivileges).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("GET", "/api/listeners") => {
            let snapshot = state.listeners.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("GET", "/api/users") => {
            let users = state.users.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: users.json(),
            })
        }
        ("GET", "/api/searches/records") => {
            let mut searches = state.searches.write().await;
            let expired = searches.expire_due();
            let body = searches.json(route.query);
            drop(searches);
            persist_search_records(state, &expired).await?;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", "/api/searches") => {
            let mut searches = state.searches.write().await;
            let expired = searches.expire_due();
            let body = searches.controller_list_json(route.query);
            drop(searches);
            persist_search_records(state, &expired).await?;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", _path) if search_token_path(normalized_path, "").is_some() => {
            let Some(token) = search_token_path(normalized_path, "") else {
                return Ok(routing::not_found_response());
            };
            let mut searches = state.searches.write().await;
            let expired = searches.expire_due();
            if let Some(record) = searches.get(token) {
                let body = record.json_with_query(route.query);
                drop(searches);
                persist_search_records(state, &expired).await?;
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json",
                    body,
                })
            } else {
                drop(searches);
                persist_search_records(state, &expired).await?;
                Ok(routing::not_found_response())
            }
        }
        ("GET", "/api/rooms") => {
            let rooms = state.rooms.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: rooms.json(route.query),
            })
        }
        ("GET", "/api/messages") => {
            let messages = state.messages.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: messages.json(route.query),
            })
        }
        ("GET", "/api/transfers") if route.path.starts_with("/api/v0/") => {
            let transfers = state.transfers.read().await;
            let downloads = serde_json::from_str::<Vec<serde_json::Value>>(
                &transfers.controller_transfers_json(0, None),
            )
            .unwrap_or_default();
            let uploads = serde_json::from_str::<Vec<serde_json::Value>>(
                &transfers.controller_transfers_json(1, None),
            )
            .unwrap_or_default();
            Ok(routing::ok_response(
                serde_json::Value::Array(downloads.into_iter().chain(uploads).collect())
                    .to_string(),
            ))
        }
        ("GET", "/api/transfers") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.json(route.query),
            })
        }
        ("GET", "/api/transfers/stats") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.stats_json(),
            })
        }
        ("POST", "/api/searches") => {
            if route.path.starts_with("/api/v0/")
                && extract_json_string_field(body, "acquisitionProfile")
                    .is_some_and(|profile| !is_known_acquisition_profile(&profile))
            {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!(
                        "The field AcquisitionProfile must be a known acquisition profile"
                    )
                    .to_string(),
                });
            }
            let query = match extract_json_string_field(body, "query")
                .or_else(|| extract_json_string_field(body, "searchText"))
            {
                Some(q) => q,
                None => {
                    return Ok(routing::bad_request_response(
                        "query/searchText is required",
                    ))
                }
            };

            let target_str =
                extract_json_string_field(body, "target").unwrap_or_else(|| "global".to_string());
            let external_id = extract_json_string_field(body, "id");
            let username_opt = extract_json_string_field(body, "username");
            let room_opt = extract_json_string_field(body, "room");
            let wishlist_item_id = extract_json_string_field(body, "wishlistItemId")
                .or_else(|| extract_json_string_field(body, "wishlist_item_id"));
            let ttl_seconds = match search_ttl_seconds_from_body(body) {
                Ok(ttl_seconds) => ttl_seconds,
                Err(error) => return Ok(routing::bad_request_response(error)),
            };

            if !matches!(target_str.as_str(), "global" | "user" | "room" | "wishlist") {
                return Ok(routing::bad_request_response("invalid search target"));
            }
            if target_str == "user" && username_opt.is_none() {
                return Ok(routing::bad_request_response(
                    "username is required for user search",
                ));
            }
            if target_str == "room" && room_opt.is_none() {
                return Ok(routing::bad_request_response(
                    "room is required for room target",
                ));
            }

            if route.path.starts_with("/api/v0/") {
                let session_state = state.session.read().await.state;
                if session_state != "connected" {
                    let display_state = match session_state {
                        "connecting" => "Connecting",
                        "disconnecting" => "Disconnecting",
                        _ => "Disconnected",
                    };
                    let failed_target = search_target_static(&target_str);
                    let failed_target_name = if target_str == "user" {
                        username_opt.clone()
                    } else if target_str == "room" {
                        room_opt.clone()
                    } else if target_str == "wishlist" {
                        wishlist_item_id.clone()
                    } else {
                        None
                    };
                    let mut searches = state.searches.write().await;
                    let previous_searches = searches.clone();
                    let outcome = searches.create(
                        external_id,
                        query.clone(),
                        failed_target,
                        failed_target_name,
                        Vec::new(),
                        ttl_seconds,
                    );
                    let Ok(outcome) = outcome else {
                        return Ok(disconnected_search_conflict_response(state, display_state));
                    };
                    let token = outcome.record.token;
                    let evicted = outcome.evicted;
                    let mutated_searches = searches.clone();
                    let Some((failed_record, _)) = searches.set_status_by_token(token, "failed")
                    else {
                        return Ok(disconnected_search_conflict_response(state, display_state));
                    };
                    drop(searches);
                    if let Err(error) = delete_persisted_searches(state, &evicted).await {
                        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                            .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    if let Err(error) = persist_search_record(state, &failed_record).await {
                        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                            .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    return Ok(disconnected_search_conflict_response(state, display_state));
                }
            }

            let shares = state.shares.read().await;
            let matching_results = search_shares(&shares.entries, &query);
            drop(shares);

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let target_name = if target_str == "user" {
                username_opt.clone()
            } else if target_str == "room" {
                room_opt.clone()
            } else if target_str == "wishlist" {
                wishlist_item_id.clone()
            } else {
                None
            };
            let result_count = matching_results.len();

            let target = search_target_static(&target_str);
            let outcome = match searches.create(
                external_id,
                query.clone(),
                target,
                target_name.clone(),
                matching_results,
                ttl_seconds,
            ) {
                Ok(outcome) => outcome,
                Err(error) => return Ok(search_create_error_response(error)),
            };
            let record = outcome.record;
            let evicted = outcome.evicted;
            let token = record.token;
            let mutated_searches = searches.clone();

            let dispatch_target = match target_str.as_str() {
                "user" => SearchDispatchTarget::User(username_opt.clone().unwrap_or_default()),
                "room" => SearchDispatchTarget::Room(room_opt.clone().unwrap_or_default()),
                "wishlist" => SearchDispatchTarget::Wishlist,
                _ => SearchDispatchTarget::Global,
            };
            drop(searches);

            if let Err(error) = delete_persisted_searches(state, &evicted).await {
                rollback_searches_if_unchanged(state, previous_searches.clone(), &mutated_searches)
                    .await;
                return Ok(routing::service_unavailable_response(&error));
            }
            if let Err(error) = persist_search_record(state, &record).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::Search {
                token,
                query: query.clone(),
                target: dispatch_target,
            });

            record_event(state, "search.started", format!("{}", token), None).await;

            // Dispatch webhook for search.created event
            let webhook_data = serde_json::json!({
                "token": token,
                "query": query,
                "target": target_str,
                "target_name": target_name,
                "result_count": result_count,
            });
            let correlation_id = format!("search_{}", token);
            dispatch_webhook_event(
                state,
                correlation_id,
                webhooks::WebhookEvent::SearchCreated,
                webhook_data,
            )
            .await;

            // Return simplified response matching C# compatibility contract
            let search_id = record.id.replace("-", "");
            let results_json = record
                .results
                .iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(",");
            let response_body = format!(
                r#"{{"searchId":"{}","query":"{}","results":[{}]}}"#,
                json_escape(&search_id),
                json_escape(&record.query),
                results_json
            );
            Ok(routing::ok_response(response_body))
        }

        ("POST", _path) if search_token_path(normalized_path, "/complete").is_some() => {
            let Some(token) = search_token_path(normalized_path, "/complete") else {
                return Ok(routing::not_found_response());
            };

            // A wishlist search that completes with too few usable results gets
            // one of the current smart-fallback queries immediately.  The
            // session loop also handles this at expiry time, but waiting for the
            // five-second TTL here makes manual/API completion observably slower
            // and loses the current upstream behavior.
            let fallback_query = {
                let record = state.searches.read().await.get(token);
                if record.as_ref().is_some_and(|record| {
                    record.status == "active"
                        && search_fallback::is_enabled_for_source(record.target)
                }) {
                    let record = record.expect("active wishlist search record");
                    let wishlist_policy = if let Some(item_id) = record.wishlist_item_id() {
                        state.wishlist.read().await.result_policy_for(item_id)
                    } else {
                        None
                    };
                    let response_limit = wishlist_policy
                        .as_ref()
                        .map(|policy| policy.max_results)
                        .unwrap_or(MAX_SEARCH_RESULTS_PER_SEARCH);
                    let file_count = record
                        .results
                        .len()
                        .saturating_add(record.hidden_locked_count);
                    (search_fallback::needs_fallback(
                        record.raw_response_count,
                        file_count,
                        response_limit,
                        response_limit,
                    ))
                    .then(|| search_fallback::create_queries(&record.query))
                    .and_then(|queries| queries.get(record.fallback_attempts).cloned())
                } else {
                    None
                }
            };

            if let Some(fallback_query) = fallback_query {
                let fallback_record = {
                    let mut searches = state.searches.write().await;
                    searches.reset_for_fallback(token, fallback_query, 5)
                };
                if let Some(fallback_record) = fallback_record {
                    let body_json = fallback_record.json();
                    persist_search_record(state, &fallback_record).await?;
                    record_event(
                        state,
                        "wishlist.search.fallback_started",
                        fallback_record.token.to_string(),
                        Some(
                            serde_json::json!({
                                "query": fallback_record.query,
                                "attempt": fallback_record.fallback_attempts,
                                "source": "completion",
                            })
                            .to_string(),
                        ),
                    )
                    .await;
                    if let Err(error) = send_session_command(
                        state,
                        SessionCommand::Search {
                            token: fallback_record.token,
                            query: fallback_record.query,
                            target: SearchDispatchTarget::Wishlist,
                        },
                    )
                    .await
                    {
                        update_session(state, |snapshot| {
                            snapshot.last_error = Some(error.clone());
                        })
                        .await;
                    }
                    return Ok(routing::ok_response(body_json));
                }
            }

            let mut searches = state.searches.write().await;
            if let Some((record, transitioned)) = searches.complete(token) {
                let body_json = record.json();

                // Dispatch webhook for search.completed event
                let result_count = record.results.len();
                let webhook_data = serde_json::json!({
                    "token": token,
                    "query": record.query,
                    "result_count": result_count,
                    "target": record.target,
                });
                let correlation_id = format!("search_{}", token);

                drop(searches);
                if transitioned {
                    persist_search_record(state, &record).await?;
                }
                if transitioned && record.wishlist_item_id().is_some() {
                    let mut wishlist = state.wishlist.write().await;
                    let previous = wishlist.clone();
                    if let Some(item) = wishlist.record_completed_search(&record) {
                        let mutated = wishlist.clone();
                        drop(wishlist);
                        if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                            rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                    if let Err(error) = auto_download_completed_wishlist(state, &record).await {
                        update_session(state, |snapshot| {
                            snapshot.last_error = Some(error.clone());
                        })
                        .await;
                        record_daemon_log(state, logging::LogLevel::Warn, "wishlist", error).await;
                    }
                }

                if transitioned {
                    dispatch_webhook_event(
                        state,
                        correlation_id,
                        webhooks::WebhookEvent::SearchCompleted,
                        webhook_data,
                    )
                    .await;
                }

                Ok(routing::ok_response(body_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("POST", _path)
            if search_token_path(normalized_path, "/cancel").is_some()
                || search_token_path(normalized_path, "/fail").is_some()
                || search_token_path(normalized_path, "/expire").is_some() =>
        {
            let (token, status, event_kind) =
                if let Some(token) = search_token_path(normalized_path, "/cancel") {
                    (token, "cancelled", "search.cancelled")
                } else if let Some(token) = search_token_path(normalized_path, "/fail") {
                    (token, "failed", "search.failed")
                } else if let Some(token) = search_token_path(normalized_path, "/expire") {
                    (token, "expired", "search.expired")
                } else {
                    return Ok(routing::not_found_response());
                };
            let mut searches = state.searches.write().await;
            if let Some((record, transitioned)) = searches.set_status_by_token(token, status) {
                let body_json = record.json();
                drop(searches);
                if transitioned {
                    persist_search_record(state, &record).await?;
                    record_event(state, event_kind, token.to_string(), None).await;
                }
                Ok(routing::ok_response(body_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("POST", "/api/searches/prune") => {
            let mut searches = state.searches.write().await;
            let pruned_records = searches.prune_expired();
            let pruned = pruned_records.len();
            let remaining = searches.records.len();
            drop(searches);
            for record in &pruned_records {
                delete_persisted_search(state, record).await?;
            }
            Ok(routing::ok_response(format!(
                "{{\"pruned\":{},\"remaining\":{}}}",
                pruned, remaining
            )))
        }

        ("POST", "/api/search-responses") => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload) => payload,
                Err(_) => return Ok(routing::bad_request_response("invalid JSON body")),
            };

            let token = match payload.get("token").and_then(serde_json::Value::as_u64) {
                Some(t) => match u32::try_from(t) {
                    Ok(token) => token,
                    Err(_) => {
                        return Ok(routing::bad_request_response("token exceeds u32 range"));
                    }
                },
                None => return Ok(routing::bad_request_response("token is required")),
            };

            let peer_username = payload
                .get("peer_username")
                .or_else(|| payload.get("username"))
                .and_then(serde_json::Value::as_str);
            let slot_free = payload
                .get("slot_free")
                .or_else(|| payload.get("hasFreeUploadSlot"))
                .and_then(serde_json::Value::as_bool);
            let average_speed = payload
                .get("average_speed")
                .or_else(|| payload.get("uploadSpeed"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let queue_length = payload
                .get("queue_length")
                .or_else(|| payload.get("queueLength"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let locked = payload
                .get("locked")
                .or_else(|| payload.get("isLocked"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let mut entries = Vec::new();
            if let Some(files) = payload.get("files").and_then(serde_json::Value::as_array) {
                entries.extend(files.iter().filter_map(|file| {
                    SearchResultEntry::from_json_file(
                        file,
                        peer_username,
                        false,
                        slot_free,
                        average_speed,
                        queue_length,
                    )
                }));
            }
            if let Some(files) = payload
                .get("lockedFiles")
                .and_then(serde_json::Value::as_array)
            {
                entries.extend(files.iter().filter_map(|file| {
                    SearchResultEntry::from_json_file(
                        file,
                        peer_username,
                        true,
                        slot_free,
                        average_speed,
                        queue_length,
                    )
                }));
            }

            if entries.is_empty() {
                let entry = SearchResultEntry::from_json_file(
                    &payload,
                    peer_username,
                    locked,
                    slot_free,
                    average_speed,
                    queue_length,
                )
                .unwrap_or_else(|| {
                    bounded_search_result_entry(SearchResultEntry {
                        peer_username: peer_username.map(str::to_owned),
                        filename: String::new(),
                        size: payload
                            .get("size")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                        bit_rate: payload
                            .get("bitRate")
                            .or_else(|| payload.get("bit_rate"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        sample_rate: payload
                            .get("sampleRate")
                            .or_else(|| payload.get("sample_rate"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        bit_depth: payload
                            .get("bitDepth")
                            .or_else(|| payload.get("bit_depth"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        length_seconds: payload
                            .get("length")
                            .or_else(|| payload.get("lengthSeconds"))
                            .or_else(|| payload.get("length_seconds"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        locked,
                        slot_free,
                        average_speed,
                        queue_length,
                        extension: String::new(),
                    })
                });
                entries.push(entry);
            }

            let wishlist_item_id = {
                let searches = state.searches.read().await;
                searches
                    .get(token)
                    .and_then(|record| record.wishlist_item_id().map(str::to_owned))
            };
            let (wishlist_policy, wishlist_counts) =
                if let Some(item_id) = wishlist_item_id.as_deref() {
                    let wishlist = state.wishlist.read().await;
                    let ignored_results = wishlist.ignored_results_for(item_id);
                    let policy = wishlist.result_policy_for(item_id);
                    let mut filtered_out = 0_usize;
                    let mut ignored = 0_usize;
                    let mut hidden_locked = 0_usize;
                    for entry in &entries {
                        if policy
                            .as_ref()
                            .is_some_and(|policy| !policy.filter.matches_entry(entry))
                        {
                            filtered_out = filtered_out.saturating_add(1);
                        } else if entry.peer_username.as_deref().is_some_and(|username| {
                            ignored_results
                                .iter()
                                .any(|rule| rule.matches(username, &entry.filename))
                        }) {
                            ignored = ignored.saturating_add(1);
                        } else if entry.locked {
                            hidden_locked = hidden_locked.saturating_add(1);
                        }
                    }
                    entries.retain(|entry| {
                        if entry.locked
                            || policy
                                .as_ref()
                                .is_some_and(|policy| !policy.filter.matches_entry(entry))
                        {
                            return false;
                        }
                        let Some(username) = entry.peer_username.as_deref() else {
                            return true;
                        };
                        !ignored_results
                            .iter()
                            .any(|rule| rule.matches(username, &entry.filename))
                    });
                    (policy, Some((filtered_out, ignored, hidden_locked)))
                } else {
                    (None, None)
                };

            let mut searches = state.searches.write().await;
            let aggregate_remaining =
                MAX_TOTAL_SEARCH_RESULTS.saturating_sub(searches.total_results());
            if let Some(record) = searches.records.iter_mut().find(|r| r.token == token) {
                if let Some((filtered_out, ignored, hidden_locked)) = wishlist_counts {
                    record.raw_response_count = record.raw_response_count.saturating_add(1);
                    record.filtered_out_count =
                        record.filtered_out_count.saturating_add(filtered_out);
                    record.ignored_result_count =
                        record.ignored_result_count.saturating_add(ignored);
                    record.hidden_locked_count =
                        record.hidden_locked_count.saturating_add(hidden_locked);
                }
                record.extend_results_with_limit_bounded(
                    entries,
                    aggregate_remaining,
                    wishlist_policy
                        .as_ref()
                        .map(|policy| policy.max_results)
                        .unwrap_or(MAX_SEARCH_RESULTS_PER_SEARCH),
                );
                record.updated_at = unix_timestamp();
                let response_json = record.json();
                let record = record.clone();
                drop(searches);
                persist_search_record(state, &record).await?;
                Ok(routing::ok_response(response_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        // TRANSFER ENDPOINTS
        ("GET", "/api/downloads") => {
            let requested_state = query_parameter(route.query, "status")
                .or_else(|| query_parameter(route.query, "state"))
                .map(|value| value.to_ascii_lowercase())
                .filter(|value| {
                    matches!(
                        value.as_str(),
                        "active" | "completed" | "failed" | "cancelled"
                    )
                });
            let transfers = state.transfers.read().await;
            let downloads = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0)
                .filter(|entry| {
                    requested_state.as_deref().is_none_or(|requested| {
                        native_download_status(entry.status.as_str()) == requested
                    })
                })
                .map(native_compatibility_download_json)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({"downloads": downloads}).to_string(),
            ))
        }
        ("GET", "/api/v0/downloads/requests") | ("GET", "/api/downloads/requests") => {
            let requested_state = query_parameter(route.query, "state")
                .map(|value| value.to_ascii_lowercase())
                .filter(|value| {
                    matches!(
                        value.as_str(),
                        "active" | "completed" | "failed" | "cancelled"
                    )
                });
            let transfers = state.transfers.read().await;
            let mut grouped = std::collections::BTreeMap::<&str, Vec<&TransferEntry>>::new();
            for entry in transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0)
            {
                if let Some(request_id) = entry.request_id.as_deref() {
                    grouped.entry(request_id).or_default().push(entry);
                }
            }
            let mut requests = grouped
                .into_values()
                .filter(|entries| {
                    requested_state.as_deref().is_none_or(|requested| {
                        download_request_state(entries).eq_ignore_ascii_case(requested)
                    })
                })
                .map(|entries| download_request_projection(&entries, false))
                .collect::<Vec<_>>();
            requests.sort_by_key(|entry| {
                std::cmp::Reverse(entry["request"]["createdAt"].as_u64().unwrap_or(0))
            });
            Ok(routing::ok_response(
                serde_json::Value::Array(requests).to_string(),
            ))
        }

        ("GET", path) if download_request_path(path).is_some() => {
            let Some((request_id, None)) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let transfers = state.transfers.read().await;
            let attempts = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0 && entry.request_id.as_deref() == Some(request_id)
                })
                .collect::<Vec<_>>();
            if attempts.is_empty() {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(
                download_request_projection(&attempts, true).to_string(),
            ))
        }

        ("PATCH", path) if download_request_path(path).is_some() => {
            let Some((request_id, Some("name"))) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let Some(name) = extract_json_string_field(body, "name")
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
            else {
                return Ok(routing::bad_request_response("name is required"));
            };
            if name.len() > MAX_TRANSFER_REQUEST_NAME_BYTES {
                return Ok(routing::bad_request_response(
                    "name must be 512 bytes or fewer",
                ));
            }
            let mut transfers = state.transfers.write().await;
            let previous = transfers.entries.clone();
            let mut updated = Vec::new();
            for entry in transfers.entries.iter_mut().filter(|entry| {
                entry.direction == 0 && entry.request_id.as_deref() == Some(request_id)
            }) {
                entry.request_name = Some(name.clone());
                entry.updated_at = unix_timestamp();
                entry.updated_at_ms = unix_timestamp_millis();
                updated.push(entry.clone());
            }
            if updated.is_empty() {
                return Ok(routing::not_found_response());
            }
            transfers.persist_state();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &updated).await {
                let mut transfers = state.transfers.write().await;
                transfers.entries = previous.clone();
                transfers.persist_state();
                drop(transfers);
                let _ = persist_transfer_records(state, &previous).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            let attempts = updated.iter().collect::<Vec<_>>();
            Ok(routing::ok_response(
                download_request_projection(&attempts, true).to_string(),
            ))
        }

        ("POST", path) if download_request_path(path).is_some() => {
            let Some((request_id, Some("cancel"))) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let mut transfers = state.transfers.write().await;
            let previous = transfers.entries.clone();
            let ids = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && entry.request_id.as_deref() == Some(request_id)
                        && !matches!(
                            entry.status.as_str(),
                            "succeeded" | "completed" | "cancelled"
                        )
                })
                .map(|entry| entry.id)
                .collect::<Vec<_>>();
            let exists = transfers.entries.iter().any(|entry| {
                entry.direction == 0 && entry.request_id.as_deref() == Some(request_id)
            });
            if !exists {
                return Ok(routing::not_found_response());
            }
            let updated = ids
                .into_iter()
                .filter_map(|id| {
                    transfers.update_status(
                        id,
                        "cancelled",
                        None,
                        Some("cancelled by request".to_owned()),
                    )
                })
                .collect::<Vec<_>>();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &updated).await {
                let mut transfers = state.transfers.write().await;
                transfers.entries = previous.clone();
                transfers.persist_state();
                drop(transfers);
                let _ = persist_transfer_records(state, &previous).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::no_content_response())
        }

        ("GET", "/api/transfers/changes") => {
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let include_completed = query_parameter(route.query, "includeCompleted")
                .as_deref()
                .and_then(parse_bool_value)
                .unwrap_or(true);
            let snapshot_at = unix_timestamp_millis();
            let transfers = state.transfers.read().await;
            let rows = transfers
                .entries
                .iter()
                .filter(|entry| entry.updated_at_ms <= snapshot_at)
                .filter(|entry| {
                    since.is_some()
                        || include_completed
                        || !matches!(entry.status.as_str(), "succeeded" | "completed")
                })
                .filter(|entry| since.is_none_or(|since| entry.updated_at_ms > since))
                .map(TransferEntry::controller_file_json)
                .collect::<Vec<_>>();
            let download = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0)
                .count();
            let upload = transfers.entries.len().saturating_sub(download);
            Ok(routing::ok_response(
                serde_json::json!({
                    "cursor": snapshot_at,
                    "counts": { "download": download, "upload": upload },
                    "transfers": rows,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/transfers/history") => {
            let direction = query_parameter(route.query, "direction").unwrap_or_default();
            let direction = match direction.trim().to_ascii_lowercase().as_str() {
                "download" => 0,
                "upload" => 1,
                _ => {
                    return Ok(routing::bad_request_response(
                        "direction must be 'download' or 'upload'",
                    ))
                }
            };
            let as_of = match query_millis_parameter(route.query, "asOf") {
                Ok(value) => value.unwrap_or_else(unix_timestamp_millis),
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let offset = match query_bounded_usize(route.query, "offset", 0, usize::MAX) {
                Ok(value) => value.unwrap_or(0),
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "offset must be greater than or equal to zero",
                    ))
                }
            };
            let limit = match query_bounded_usize(route.query, "limit", 1, 500) {
                Ok(value) => value.unwrap_or(250),
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 500",
                    ))
                }
            };
            let transfers = state.transfers.read().await;
            let mut rows = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == direction)
                .filter(|entry| matches!(entry.status.as_str(), "succeeded" | "completed"))
                .filter(|entry| entry.updated_at_ms <= as_of)
                .collect::<Vec<_>>();
            rows.sort_by_key(|entry| {
                std::cmp::Reverse((entry.updated_at_ms, entry.requested_at, entry.id))
            });
            let page = rows
                .into_iter()
                .skip(offset)
                .take(limit.saturating_add(1))
                .collect::<Vec<_>>();
            let has_more = page.len() > limit;
            let rows = page
                .into_iter()
                .take(limit)
                .map(TransferEntry::controller_file_json)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({
                    "asOf": as_of,
                    "hasMore": has_more,
                    "nextOffset": offset.saturating_add(rows.len()),
                    "transfers": rows,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/transfers/downloads/batches") => {
            Ok(controller_enqueue_download_batch(body, state).await)
        }

        ("POST", "/api/transfers") => {
            if let Some((username, mut files)) = controller_enqueue_request(body) {
                let exclusions = effective_download_exclusions(state).await;
                let filenames = files
                    .iter()
                    .filter_map(|file| file.get("filename").and_then(serde_json::Value::as_str))
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                let blocked = filenames
                    .iter()
                    .filter_map(|filename| {
                        crate::download_filter::matching_exclusion(filename, &exclusions).map(
                            |exclusion| {
                                serde_json::json!({
                                    "filename": filename,
                                    "exclusion": exclusion,
                                })
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                if blocked.len() == filenames.len() && !blocked.is_empty() {
                    return Ok(download_policy_response(state, &filenames)
                        .await
                        .expect("non-empty blocked list must produce a policy response"));
                }
                files.retain(|file| {
                    file.get("filename")
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(|filename| {
                            !crate::download_filter::is_excluded(filename, &exclusions)
                        })
                });
                let batch_id = controller_transfer_batch_id(body)
                    .or_else(|| (files.len() > 1).then(|| uuid::Uuid::new_v4().to_string()));
                let mut transfers = state.transfers.write().await;
                let mut created = Vec::new();
                let mut created_entries = Vec::new();
                for file in files {
                    let filename = file
                        .get("filename")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    if filename.is_empty() {
                        continue;
                    }
                    let size = file.get("size").and_then(serde_json::Value::as_u64);
                    let details = transfer_request_details_from_json(&file, &filename);
                    let relative = match render_configured_completed_download_path(
                        state,
                        &username,
                        &filename,
                        batch_id.as_deref(),
                        details.request_name.as_deref(),
                        unix_timestamp(),
                    )
                    .await
                    {
                        Ok(relative) => relative,
                        Err(error) => return Ok(routing::bad_request_response(&error)),
                    };
                    let local_path =
                        match configured_download_destination_path(state, &relative).await {
                            Ok(path) => Some(path.display().to_string()),
                            Err(error) => return Ok(routing::bad_request_response(&error)),
                        };
                    let entry = transfers.create_with_details(
                        0,
                        Some(username.clone()),
                        filename,
                        local_path,
                        size,
                        batch_id.clone(),
                        details,
                    );
                    created.push(entry.controller_file_json());
                    created_entries.push(entry);
                }
                let count = created.len();
                drop(transfers);
                persist_transfer_records(state, &created_entries).await?;
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "queued": count,
                        "transfers": created,
                        "blocked": blocked,
                    })
                    .to_string(),
                ));
            }

            let filename = match extract_json_string_field(body, "filename") {
                Some(f) => f,
                None => return Ok(routing::bad_request_response("filename is required")),
            };

            let direction = extract_json_u32_field(body, "direction").unwrap_or(0);
            if direction == 0 {
                if let Some(response) =
                    download_policy_response(state, std::slice::from_ref(&filename)).await
                {
                    return Ok(response);
                }
            }
            let peer_username = extract_json_string_field(body, "peer_username");
            let supplied_local_path = extract_json_string_field(body, "local_path");
            let batch_id = controller_transfer_batch_id(body);
            let size = extract_json_u64_field(body, "size");
            let payload =
                serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null);
            let details = if direction == 0 {
                transfer_request_details_from_json(&payload, &filename)
            } else {
                TransferRequestDetails::default()
            };
            let local_path = match prepare_transfer_local_path(
                state,
                direction,
                peer_username.as_deref(),
                &filename,
                batch_id.as_deref(),
                &details,
                supplied_local_path,
            )
            .await
            {
                Ok(path) => path,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };

            let mut transfers = state.transfers.write().await;
            let entry = transfers.create_with_details(
                direction,
                peer_username.clone(),
                filename.clone(),
                local_path.clone(),
                size,
                batch_id,
                details,
            );
            drop(transfers);
            persist_transfer_record(state, &entry).await?;
            Ok(routing::created_response(entry.json()))
        }

        ("GET", "/api/transfers/downloads") | ("GET", "/api/transfers/downloads/") => {
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            if route.path == "/api/downloads"
                && state.config.controller_profile == ControllerProfile::Native
            {
                let requested_status = query_parameter(route.query, "status")
                    .map(|value| value.to_ascii_lowercase())
                    .filter(|value| {
                        matches!(
                            value.as_str(),
                            "queued" | "running" | "completed" | "cancelled" | "failed"
                        )
                    });
                let downloads = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction == 0)
                    .filter(|entry| {
                        requested_status.as_deref().is_none_or(|requested| {
                            native_download_status(entry.status.as_str()) == requested
                        })
                    })
                    .map(native_compatibility_download_json)
                    .collect::<Vec<_>>();
                drop(transfers);
                return Ok(routing::ok_response(
                    serde_json::json!({"downloads": downloads}).to_string(),
                ));
            }
            let body = transfers.controller_transfers_json(0, None);
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/transfers/uploads") | ("GET", "/api/transfers/uploads/") => {
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = transfers.controller_transfers_json(1, None);
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/transfers/downloads/accelerated") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                let enabled = state.runtime.read().await.accelerated_downloads_enabled;
                return Ok(routing::ok_response(
                     serde_json::json!({
                         "enabled": enabled,
                         "updatedAt": chrono::Utc::now().to_rfc3339(),
                         "policy": "Normal downloads remain single-source. Underperforming downloads may use verified alternate sources; raw Soulseek peers use sequential failover, while true multipart chunking is reserved for trusted mesh-overlay peers.",
                     })
                     .to_string(),
                 ));
            }
            let transfers = state.transfers.read().await;
            let mut value = serde_json::from_str::<serde_json::Value>(
                &controller_accelerated_downloads_json(route.query, &transfers),
            )
            .unwrap_or_else(|_| serde_json::json!({}));
            value["updatedAt"] = serde_json::json!(unix_timestamp());
            value["policy"] = serde_json::json!({"enabled": false});
            drop(transfers);
            Ok(routing::ok_response(value.to_string()))
        }

        ("GET", "/api/transfers/downloads/auto-replace/status")
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native =>
        {
            let stuck_count = state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && matches!(entry.status.as_str(), "failed" | "rejected" | "cancelled")
                })
                .count();
            let enabled = state.runtime.read().await.autoreplace_enabled;
            Ok(routing::ok_response(
                serde_json::json!({
                    "stuckCount": stuck_count,
                    "enabled": enabled,
                    "intervalSeconds": 300,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/transfers/downloads/stuck") => {
            let transfers = state.transfers.read().await;
            let value = serde_json::from_str::<serde_json::Value>(
                &controller_stuck_downloads_json(route.query, &transfers),
            )
            .unwrap_or_else(|_| serde_json::json!({"stuck": []}));
            drop(transfers);
            Ok(routing::ok_response(value["stuck"].to_string()))
        }

        ("GET", "/api/transfers/uploads/diagnostics") => {
            let transfers = state.transfers.read().await;
            let uploads = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction != 0)
                .collect::<Vec<_>>();
            let listeners = state.listeners.read().await;
            let shares = state.shares.read().await;
            Ok(routing::ok_response(serde_json::json!({
                 "activeUploads": uploads.iter().filter(|entry| is_active_transfer_status(&entry.status)).count(),
                 "failedUploads": uploads.iter().filter(|entry| entry.status == "failed").count(),
                 "succeededUploads": uploads.iter().filter(|entry| entry.status == "succeeded").count(),
                 "totalUploadRecords": uploads.len(),
                 "generatedAt": unix_timestamp(),
                 "isConnected": state.session.read().await.state == "connected",
                 "isLoggedIn": state.session.read().await.state == "connected",
                 "listenIpAddress": listeners.regular_bind.as_deref().and_then(|address| address.parse::<SocketAddr>().ok()).map(|address| address.ip()),
                 "listenPort": listeners.regular_bind.as_deref().and_then(|address| address.parse::<SocketAddr>().ok()).map(|address| address.port()).unwrap_or(0),
                 "localListenProbe": serde_json::Value::Null,
                 "recentUploads": uploads.iter().take(20).map(|entry| serde_json::from_str::<serde_json::Value>(&entry.json()).unwrap_or_default()).collect::<Vec<_>>(),
                 "shareDirectories": shares.roots.len(),
                 "shareFiles": shares.entries.len(),
                 "shareScanPending": false,
                 "shareScanning": false,
                 "soulseekState": state.session.read().await.state,
                 "uploadSlots": state.config.transfer_max_active,
                 "uploadSpeedLimit": 0,
                 "warnings": [],
             }).to_string()))
        }

        ("GET", "/api/transfers/downloads/user-stats") => {
            let transfers = state.transfers.read().await;
            let body = controller_download_user_stats_json(route.query, &transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/transfers/downloads/stats") => {
            let transfers = state.transfers.read().await;
            let json = controller_download_stats_json(&transfers);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/transfers/downloads/batches/") => {
            let batch_id =
                decoded_path_segment(path.trim_start_matches("/api/transfers/downloads/batches/"));
            if uuid::Uuid::parse_str(&batch_id).is_err() {
                return Ok(routing::bad_request_response("invalid batch id"));
            }
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let batch_record = match controller_read_transfer_batch(state, &batch_id).await {
                Ok(record) => record,
                Err(error) => return Ok(routing::internal_server_error_response(&error)),
            };
            let transfers = state.transfers.read().await;
            let downloads = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0 && entry.batch_id.as_deref() == Some(batch_id.as_str())
                })
                .map(TransferEntry::controller_file_json)
                .collect::<Vec<_>>();
            if downloads.is_empty() && batch_record.is_none() {
                drop(transfers);
                return Ok(routing::not_found_response());
            }
            let completed_count = downloads
                .iter()
                .filter(|entry| {
                    entry
                        .get("state")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|state| state.eq_ignore_ascii_case("Completed"))
                })
                .count();
            let failed_count = downloads
                .iter()
                .filter(|entry| {
                    entry
                        .get("state")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|state| matches!(state, "Failed" | "Errored"))
                })
                .count();
            let transfer_count = downloads.len();
            drop(transfers);
            if let Some(batch) = batch_record {
                return Ok(routing::ok_response(
                    transfer_batch_with_entries(batch, downloads).to_string(),
                ));
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "id": batch_id,
                    "transfers": downloads,
                    "transferCount": transfer_count,
                    "completedCount": completed_count,
                    "failedCount": failed_count,
                })
                .to_string(),
            ))
        }

        ("GET", path) if controller_transfer_user_path(path, "downloads").is_some() => {
            let Some(username) = controller_transfer_user_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = transfers.controller_transfer_user_json(0, username.trim());
            drop(transfers);
            Ok(body
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response))
        }

        ("GET", path) if controller_transfer_user_path(path, "uploads").is_some() => {
            let Some(username) = controller_transfer_user_path(path, "uploads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = transfers.controller_transfer_user_json(1, username.trim());
            drop(transfers);
            Ok(body
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response))
        }

        ("GET", path)
            if controller_transfer_file_path(path, "downloads").is_some()
                && !path.ends_with("/position") =>
        {
            let Some((username, id)) = controller_transfer_file_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let username = decoded_path_segment(username);
            let transfers = state.transfers.read().await;
            let response = transfers
                .controller_transfer_json(0, &username, id)
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response);
            drop(transfers);
            Ok(response)
        }

        ("GET", path)
            if controller_transfer_file_path(path, "uploads").is_some()
                && !path.ends_with("/position") =>
        {
            let Some((username, id)) = controller_transfer_file_path(path, "uploads") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let username = decoded_path_segment(username);
            let transfers = state.transfers.read().await;
            let response = transfers
                .controller_transfer_json(1, &username, id)
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response);
            drop(transfers);
            Ok(response)
        }

        ("GET", path) if controller_transfer_position_path(path).is_some() => {
            let Some((username, id)) = controller_transfer_position_path(path) else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let username = decoded_path_segment(username);
            let transfers = state.transfers.read().await;
            let filename = transfers.entries.iter().find_map(|entry| {
                (entry.direction == 0
                    && entry.id == id
                    && entry.peer_username.as_deref() == Some(username.as_str()))
                .then(|| entry.filename.clone())
            });
            drop(transfers);
            let Some(filename) = filename else {
                return Ok(routing::not_found_response());
            };
            if state.session.read().await.state != "connected" {
                return Ok(routing::no_content_response());
            }
            let address = if let Some(address) = cached_peer_endpoint(state, &username).await {
                address
            } else if state.regular_listener_commands.is_none()
                && state.obfuscated_listener_commands.is_none()
            {
                // A test/in-process state without listener workers cannot
                // service the asynchronous session endpoint lookup. Match
                // The legacy empty queue-position contract immediately rather
                // than waiting for the network timeout; a live daemon still
                // takes the discovery path below.
                return Ok(routing::no_content_response());
            } else {
                match request_peer_endpoint(state, &username).await {
                    Ok(address) => address,
                    Err(_) => return Ok(routing::no_content_response()),
                }
            };
            let response = match send_peer_message_request(
                state,
                &address,
                PeerMessage::PlaceInQueueRequest {
                    filename: filename.clone(),
                },
            )
            .await
            {
                Ok(response) => response,
                Err(_) => return Ok(routing::no_content_response()),
            };
            match response {
                PeerMessage::PlaceInQueueResponse {
                    filename: response_filename,
                    place,
                } if response_filename == filename => Ok(routing::ok_response(place.to_string())),
                _ => Ok(routing::no_content_response()),
            }
        }

        ("POST", "/api/transfers/downloads/find-alternative") => {
            if route.path.starts_with("/api/v0/")
                && extract_json_u64_field(body, "transfer_id").is_none()
            {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            let transfer_id = extract_json_u64_field(body, "transfer_id").unwrap_or(0);
            if transfer_id == 0 {
                return Ok(routing::bad_request_response("transfer_id is required"));
            }
            let transfers = state.transfers.read().await;
            let Some(transfer) = transfers
                .entries
                .iter()
                .find(|entry| entry.id == transfer_id && entry.direction == 0)
                .cloned()
            else {
                drop(transfers);
                return Ok(routing::not_found_response());
            };
            drop(transfers);
            let searches = state.searches.read().await;
            let json = searches.transfer_alternatives_json(&transfer);
            drop(searches);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/transfers/downloads/replace") => {
            if route.path.starts_with("/api/v0/") {
                return Ok(HttpResponse {
                    status: "500 Internal Server Error",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "success": false,
                        "error": "Failed to replace download",
                    })
                    .to_string(),
                });
            }
            let transfer_id = extract_json_u64_field(body, "transfer_id").unwrap_or(0);
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if transfer_id == 0 || username.is_empty() {
                return Ok(routing::bad_request_response(
                    "transfer_id and username are required",
                ));
            }
            let requested_filename = extract_json_string_field(body, "filename");
            let transfers = state.transfers.read().await;
            let Some(original) = transfers
                .entries
                .iter()
                .find(|entry| entry.id == transfer_id && entry.direction == 0)
                .cloned()
            else {
                drop(transfers);
                return Ok(routing::not_found_response());
            };
            drop(transfers);
            let searches = state.searches.read().await;
            let Some(alternative) = searches.find_transfer_alternative(
                &original,
                &username,
                requested_filename.as_deref(),
            ) else {
                drop(searches);
                return Ok(routing::conflict_response("no matching alternative found"));
            };
            drop(searches);
            let Some(replacement_username) = alternative.peer_username.clone() else {
                return Ok(routing::conflict_response(
                    "matching alternative has no peer",
                ));
            };
            if let Some(exclusion) = crate::download_filter::matching_exclusion(
                &alternative.filename,
                &effective_download_exclusions(state).await,
            ) {
                return Ok(routing::conflict_response(&format!(
                    "replacement blocked by download exclusion: {exclusion}"
                )));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut transfers = state.transfers.write().await;
            let previous_entries = transfers.entries.clone();
            let previous_next_id = transfers.next_id;
            let previous_next_token = transfers.next_token;
            let updated_original = transfers.update_status(
                original.id,
                "cancelled",
                Some(original.bytes_transferred),
                Some("replaced by alternative source".to_owned()),
            );
            let replacement = transfers.create_with_details(
                0,
                alternative.peer_username.clone(),
                alternative.filename.clone(),
                original.local_path.clone(),
                Some(alternative.size),
                original.batch_id.clone(),
                TransferRequestDetails {
                    request_id: original.request_id.clone(),
                    wishlist_item_id: original.wishlist_item_id.clone(),
                    request_name: original.request_name.clone(),
                    destination_directory: original.destination_directory.clone(),
                    bit_rate: original.bit_rate,
                    sample_rate: original.sample_rate,
                    bit_depth: original.bit_depth,
                    length_seconds: original.length_seconds,
                    artist: original.artist.clone(),
                    album: original.album.clone(),
                    title: original.title.clone(),
                    track_number: original.track_number,
                    year: original.year,
                    attempts: 1,
                    auto_replace_attempts: original.auto_replace_attempts.saturating_add(1),
                    next_attempt_at: None,
                },
            );
            let replacement = transfers
                .update_status(replacement.id, "peer_lookup", None, None)
                .unwrap_or(replacement);
            let replacement_json = replacement.json();
            drop(transfers);
            let mut persisted = updated_original.into_iter().collect::<Vec<_>>();
            persisted.push(replacement.clone());
            if let Err(error) = persist_transfer_records(state, &persisted).await {
                let mut transfers = state.transfers.write().await;
                transfers.entries = previous_entries;
                transfers.next_id = previous_next_id;
                transfers.next_token = previous_next_token;
                transfers.persist_state();
                drop(transfers);
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::TransferPeer {
                id: replacement.id,
                username: replacement_username,
            });
            Ok(routing::accepted_response(
                serde_json::json!({
                    "transfer_id": transfer_id,
                    "replacement_queued": true,
                    "replacement": serde_json::from_str::<serde_json::Value>(&replacement_json)
                        .map_err(|error| format!("replacement json failed: {error}"))?,
                    "status": "queued",
                })
                .to_string(),
            ))
        }

        ("POST", "/api/transfers/downloads/auto-replace") => {
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(
                    r#"{"replaced":0,"failed":0,"skipped":0,"details":[]}"#.to_owned(),
                ));
            }
            let requested_transfer_id = extract_json_u64_field(body, "transfer_id");
            let transfers = state.transfers.read().await;
            let candidates = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && matches!(entry.status.as_str(), "failed" | "rejected")
                        && requested_transfer_id.is_none_or(|id| entry.id == id)
                })
                .cloned()
                .collect::<Vec<_>>();
            drop(transfers);

            let exclusions = effective_download_exclusions(state).await;
            let searches = state.searches.read().await;
            let replacements = candidates
                .iter()
                .filter_map(|transfer| {
                    searches
                        .first_transfer_alternative(transfer)
                        .filter(|alternative| {
                            !crate::download_filter::is_excluded(&alternative.filename, &exclusions)
                        })
                        .map(|alternative| (transfer.clone(), alternative))
                })
                .collect::<Vec<_>>();
            drop(searches);

            if replacements.is_empty() {
                return Ok(routing::accepted_response(
                    serde_json::json!({
                        "replacement_queued": false,
                        "alternatives": [],
                        "replacements": [],
                        "status": "idle",
                    })
                    .to_string(),
                ));
            }

            let mut session_command_permits = Vec::with_capacity(replacements.len());
            for _ in 0..replacements.len() {
                match state.session_commands.reserve().await {
                    Ok(permit) => session_command_permits.push(permit),
                    Err(_) => {
                        return Ok(routing::service_unavailable_response(
                            "session manager is not running",
                        ));
                    }
                }
            }

            let mut queued = Vec::new();
            let mut commands = Vec::new();
            let mut persisted = Vec::new();
            let mut transfers = state.transfers.write().await;
            let previous_entries = transfers.entries.clone();
            let previous_next_id = transfers.next_id;
            let previous_next_token = transfers.next_token;
            for (original, alternative) in replacements {
                if let Some(entry) = transfers.update_status(
                    original.id,
                    "cancelled",
                    Some(original.bytes_transferred),
                    Some("auto-replaced by alternative source".to_owned()),
                ) {
                    persisted.push(entry);
                }
                let replacement = transfers.create_with_details(
                    0,
                    alternative.peer_username.clone(),
                    alternative.filename.clone(),
                    original.local_path.clone(),
                    Some(alternative.size),
                    original.batch_id.clone(),
                    TransferRequestDetails {
                        request_id: original.request_id.clone(),
                        wishlist_item_id: original.wishlist_item_id.clone(),
                        request_name: original.request_name.clone(),
                        destination_directory: original.destination_directory.clone(),
                        bit_rate: original.bit_rate,
                        sample_rate: original.sample_rate,
                        bit_depth: original.bit_depth,
                        length_seconds: original.length_seconds,
                        artist: original.artist.clone(),
                        album: original.album.clone(),
                        title: original.title.clone(),
                        track_number: original.track_number,
                        year: original.year,
                        attempts: 1,
                        auto_replace_attempts: original.auto_replace_attempts.saturating_add(1),
                        next_attempt_at: None,
                    },
                );
                let replacement = transfers
                    .update_status(replacement.id, "peer_lookup", None, None)
                    .unwrap_or(replacement);
                persisted.push(replacement.clone());
                if let Some(username) = replacement.peer_username.clone() {
                    commands.push(SessionCommand::TransferPeer {
                        id: replacement.id,
                        username,
                    });
                }
                queued.push(serde_json::json!({
                    "transfer_id": original.id,
                    "replacement": serde_json::from_str::<serde_json::Value>(&replacement.json())
                        .map_err(|error| format!("replacement json failed: {error}"))?,
                    "alternative": {
                        "username": alternative.peer_username.as_deref().unwrap_or_default(),
                        "filename": alternative.filename,
                        "size": alternative.size,
                    },
                }));
            }
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &persisted).await {
                let mut transfers = state.transfers.write().await;
                transfers.entries = previous_entries;
                transfers.next_id = previous_next_id;
                transfers.next_token = previous_next_token;
                transfers.persist_state();
                drop(transfers);
                return Ok(routing::service_unavailable_response(&error));
            }

            for (permit, command) in session_command_permits.into_iter().zip(commands) {
                permit.send(command);
            }

            Ok(routing::accepted_response(
                serde_json::json!({
                    "replacement_queued": true,
                    "alternatives": queued.iter().map(|entry| entry["alternative"].clone()).collect::<Vec<_>>(),
                    "replacements": queued,
                    "status": "queued",
                })
                .to_string(),
            ))
        }

        ("POST", path) if controller_transfer_user_path(path, "downloads").is_some() => {
            let Some(username) = controller_transfer_user_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let mut files = controller_files_from_body(body);
            if route.path.starts_with("/api/v0/") && files.is_empty() {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!("At least one file is required").to_string(),
                });
            }
            let filenames = files
                .iter()
                .filter_map(|file| file.get("filename").and_then(serde_json::Value::as_str))
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let exclusions = effective_download_exclusions(state).await;
            let blocked = filenames
                .iter()
                .filter_map(|filename| {
                    crate::download_filter::matching_exclusion(filename, &exclusions).map(
                        |exclusion| {
                            serde_json::json!({
                                "filename": filename,
                                "exclusion": exclusion,
                            })
                        },
                    )
                })
                .collect::<Vec<_>>();
            if blocked.len() == filenames.len() && !blocked.is_empty() {
                return Ok(download_policy_response(state, &filenames)
                    .await
                    .expect("non-empty blocked list must produce a policy response"));
            }
            files.retain(|file| {
                file.get("filename")
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|filename| {
                        !crate::download_filter::is_excluded(filename, &exclusions)
                    })
            });
            let _request_permit = if route.path.starts_with("/api/v0/") {
                match Arc::clone(&state.download_requests).try_acquire_owned() {
                    Ok(permit) => Some(permit),
                    Err(_) => {
                        return Ok(HttpResponse {
                             status: "429 Too Many Requests",
                             content_type: "application/json",
                             body: serde_json::json!(
                                 "Only one concurrent operation is permitted. Wait until the previous request completes"
                             )
                             .to_string(),
                         });
                    }
                }
            } else {
                None
            };
            let batch_id = controller_transfer_batch_id(body)
                .or_else(|| (files.len() > 1).then(|| uuid::Uuid::new_v4().to_string()));
            let mut transfers = state.transfers.write().await;
            let previous_entries = transfers.entries.clone();
            let previous_next_id = transfers.next_id;
            let previous_next_token = transfers.next_token;
            let mut created = Vec::new();
            let mut created_entries = Vec::new();
            for file in files {
                let filename = file
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                if filename.is_empty() {
                    continue;
                }
                let size = file.get("size").and_then(serde_json::Value::as_u64);
                let mut details = transfer_request_details_from_json(&file, &filename);
                if details.destination_directory.is_none() {
                    details.destination_directory = query_parameter(route.query, "destination")
                        .map(|value| truncate_utf8_bytes(value, MAX_TRANSFER_LOCAL_PATH_BYTES));
                }
                let relative = if let Some(destination) = details
                    .destination_directory
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    format!(
                        "{}/{}",
                        destination.trim_matches(['/', '\\']),
                        virtual_basename(&filename)
                    )
                } else {
                    match render_configured_completed_download_path(
                        state,
                        &username,
                        &filename,
                        batch_id.as_deref(),
                        details.request_name.as_deref(),
                        unix_timestamp(),
                    )
                    .await
                    {
                        Ok(relative) => relative,
                        Err(error) => return Ok(routing::bad_request_response(&error)),
                    }
                };
                let local_path = match configured_download_destination_path(state, &relative).await
                {
                    Ok(path) => Some(path.display().to_string()),
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                let entry = transfers.create_with_details(
                    0,
                    Some(username.clone()),
                    filename,
                    local_path,
                    size,
                    batch_id.clone(),
                    details,
                );
                created.push(entry.controller_file_json());
                created_entries.push(entry);
            }
            let count = created.len();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &created_entries).await {
                let mut transfers = state.transfers.write().await;
                transfers.entries = previous_entries;
                transfers.next_id = previous_next_id;
                transfers.next_token = previous_next_token;
                transfers.persist_state();
                drop(transfers);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "queued": count,
                    "transfers": created,
                    "blocked": blocked,
                })
                .to_string(),
            ))
        }

        ("DELETE", "/api/transfers/downloads/all/completed")
        | ("DELETE", "/api/transfers/uploads/all/completed") => {
            let direction = if normalized_path.contains("/downloads/") {
                0
            } else {
                1
            };
            let mut transfers = state.transfers.write().await;
            let previous_entries = transfers.entries.clone();
            let before = transfers.entries.len();
            let mut removed_entries = Vec::new();
            transfers.entries.retain(|entry| {
                let remove = entry.direction == direction
                    && matches!(
                        entry.status.as_str(),
                        "succeeded" | "completed" | "cancelled" | "failed" | "rejected"
                    );
                if remove {
                    removed_entries.push(entry.clone());
                }
                !remove
            });
            let removed = before.saturating_sub(transfers.entries.len());
            let mutated_entries = transfers.entries.clone();
            drop(transfers);
            if let Err(error) = delete_persisted_transfers(state, &removed_entries).await {
                let mut transfers = state.transfers.write().await;
                if transfers.entries == mutated_entries {
                    transfers.entries = previous_entries;
                }
                drop(transfers);
                return Ok(routing::service_unavailable_response(&error));
            }
            if route.path.starts_with("/api/v0/") {
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response((removed > 0).to_string()))
            }
        }

        ("DELETE", path) if controller_transfer_file_path(path, "downloads").is_some() => {
            let Some((username, id)) = controller_transfer_file_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let remove_file = query_parameter(route.query, "remove")
                .is_some_and(|value| value == "true")
                || query_parameter(route.query, "deleteFile").is_some_and(|value| value == "true");
            let mut transfers = state.transfers.write().await;
            let target = transfers
                .entries
                .iter()
                .find(|entry| {
                    entry.id == id
                        && entry.direction == 0
                        && entry.peer_username.as_deref() == Some(username.as_str())
                })
                .cloned();
            let Some(target) = target else {
                return Ok(routing::not_found_response());
            };
            let updated = transfers.update_status(id, "cancelled", None, None);
            drop(transfers);
            if let Some(entry) = updated.as_ref() {
                if let Err(error) = persist_transfer_record(state, entry).await {
                    let mut transfers = state.transfers.write().await;
                    if let Some(current) = transfers
                        .entries
                        .iter_mut()
                        .find(|current| current.id == id)
                    {
                        if *current == *entry {
                            *current = target.clone();
                        }
                    }
                    transfers.persist_state();
                    drop(transfers);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            if remove_file {
                if let Some(path) = target.local_path.as_deref() {
                    let _ = fs::remove_file(path);
                }
            }
            Ok(routing::no_content_response())
        }

        ("DELETE", path) if controller_transfer_file_path(path, "uploads").is_some() => {
            let Some((username, id)) = controller_transfer_file_path(path, "uploads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let remove_file = query_parameter(route.query, "remove")
                .is_some_and(|value| value == "true")
                || query_parameter(route.query, "deleteFile").is_some_and(|value| value == "true");
            let mut transfers = state.transfers.write().await;
            let target = transfers
                .entries
                .iter()
                .find(|entry| {
                    entry.id == id
                        && entry.direction == 1
                        && entry.peer_username.as_deref() == Some(username.as_str())
                })
                .cloned();
            let Some(target) = target else {
                return Ok(routing::not_found_response());
            };
            let updated = transfers.update_status(id, "cancelled", None, None);
            drop(transfers);
            if let Some(entry) = updated.as_ref() {
                if let Err(error) = persist_transfer_record(state, entry).await {
                    let mut transfers = state.transfers.write().await;
                    if let Some(current) = transfers
                        .entries
                        .iter_mut()
                        .find(|current| current.id == id)
                    {
                        if *current == *entry {
                            *current = target.clone();
                        }
                    }
                    transfers.persist_state();
                    drop(transfers);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            if remove_file {
                if let Some(path) = target.local_path.as_deref() {
                    let _ = fs::remove_file(path);
                }
            }
            Ok(routing::no_content_response())
        }

        // GET individual transfer
        ("GET", path)
            if (path.starts_with("/api/transfers/") || path.starts_with("/api/v0/transfers/"))
                && !path.ends_with("/start")
                && !path.ends_with("/progress")
                && !path.ends_with("/complete")
                && !path.ends_with("/speeds")
                && !path.ends_with("/stats") =>
        {
            let Some(id_str) = transfer_resource_segment(path) else {
                return Ok(routing::not_found_response());
            };
            if let Ok(id) = id_str.parse::<u64>() {
                let transfers = state.transfers.read().await;
                if let Some(entry) = transfers.entries.iter().find(|t| t.id == id) {
                    let json_response = entry.json();
                    drop(transfers);
                    Ok(routing::ok_response(json_response))
                } else {
                    drop(transfers);
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::bad_request_response("invalid transfer id"))
            }
        }

        // DELETE individual transfer (cancel)
        ("DELETE", path)
            if (path.starts_with("/api/transfers/") || path.starts_with("/api/v0/transfers/"))
                && transfer_action_path(normalized_path).is_none() =>
        {
            let Some(id_str) = transfer_resource_segment(path) else {
                return Ok(routing::not_found_response());
            };
            if let Ok(id) = id_str.parse::<u64>() {
                let mut transfers = state.transfers.write().await;
                let previous = transfers
                    .entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .cloned();
                if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                    entry.status = "cancelled".to_owned();
                    entry.updated_at = unix_timestamp();
                    entry.updated_at_ms = unix_timestamp_millis();
                    let json_response = entry.json();
                    let mutated = entry.clone();
                    drop(transfers);
                    if let Err(error) = persist_transfer_record(state, &mutated).await {
                        let mut transfers = state.transfers.write().await;
                        if let Some(current) =
                            transfers.entries.iter_mut().find(|entry| entry.id == id)
                        {
                            if *current == mutated {
                                if let Some(previous) = previous {
                                    *current = previous;
                                }
                            }
                        }
                        drop(transfers);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::ok_response(json_response))
                } else {
                    drop(transfers);
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::bad_request_response("invalid transfer id"))
            }
        }

        ("POST", _path) if transfer_action_path(normalized_path).is_some() => {
            if let Some((id, action)) = transfer_action_path(normalized_path) {
                let session_command_permit = if action == "start" || action == "retry" {
                    let transfers = state.transfers.read().await;
                    let active_count = transfers
                        .entries
                        .iter()
                        .filter(|transfer| {
                            transfer.status == "in_progress" || transfer.status == "peer_lookup"
                        })
                        .count();
                    if active_count >= state.config.transfer_max_active {
                        return Ok(routing::conflict_response("transfer limit reached"));
                    }
                    let Some(entry) = transfers.entries.iter().find(|entry| entry.id == id) else {
                        return Ok(routing::not_found_response());
                    };
                    if action == "retry"
                        && (entry.direction != 0
                            || !matches!(
                                entry.status.as_str(),
                                "failed" | "rejected" | "cancelled"
                            ))
                    {
                        return Ok(routing::conflict_response("transfer is not retryable"));
                    }
                    let peer_username = entry.peer_username.clone();
                    drop(transfers);

                    if let Some(username) = peer_username {
                        if !state.config.transfer_allow_outbound {
                            return Ok(routing::conflict_response(
                                "outbound transfers are disabled",
                            ));
                        }
                        if test_user_endpoint_peer_address(state, &username).is_none() {
                            match state.session_commands.reserve().await {
                                Ok(permit) => Some(permit),
                                Err(_) => {
                                    return Ok(routing::service_unavailable_response(
                                        "session manager is not running",
                                    ));
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                let mut transfers = state.transfers.write().await;

                if action == "start" || action == "retry" {
                    // Check max active transfer limit
                    let max_active = state.config.transfer_max_active;
                    let active_count = transfers
                        .entries
                        .iter()
                        .filter(|t| t.status == "in_progress" || t.status == "peer_lookup")
                        .count();

                    if active_count >= max_active {
                        drop(transfers);
                        return Ok(routing::conflict_response("transfer limit reached"));
                    }

                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        if action == "retry"
                            && (entry.direction != 0
                                || !matches!(
                                    entry.status.as_str(),
                                    "failed" | "rejected" | "cancelled"
                                ))
                        {
                            drop(transfers);
                            return Ok(routing::conflict_response("transfer is not retryable"));
                        }
                        // Check outbound transfer policy
                        if let Some(ref username) = entry.peer_username {
                            if !state.config.transfer_allow_outbound {
                                drop(transfers);
                                return Ok(routing::conflict_response(
                                    "outbound transfers are disabled",
                                ));
                            }

                            entry.status = "peer_lookup".to_owned();
                            entry.reason = None;
                            entry.updated_at = unix_timestamp();
                            entry.updated_at_ms = unix_timestamp_millis();
                            let json_response = entry.json();
                            let username_clone = username.clone();
                            let entry = entry.clone();
                            drop(transfers);
                            persist_transfer_record(state, &entry).await?;

                            if let Some(address) =
                                test_user_endpoint_peer_address(state, &username_clone)
                            {
                                project_peer_transfer_response(state, &address).await;
                            } else {
                                let Some(session_command_permit) = session_command_permit else {
                                    return Err("missing reserved transfer peer dispatch capacity"
                                        .to_owned());
                                };
                                session_command_permit.send(SessionCommand::TransferPeer {
                                    id,
                                    username: username_clone,
                                });
                            }

                            Ok(routing::ok_response(json_response))
                        } else {
                            entry.status = "in_progress".to_owned();
                            entry.reason = None;

                            entry.updated_at = unix_timestamp();
                            entry.updated_at_ms = unix_timestamp_millis();
                            let json_response = entry.json();
                            let entry = entry.clone();
                            drop(transfers);
                            persist_transfer_record(state, &entry).await?;
                            Ok(routing::ok_response(json_response))
                        }
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                } else if action == "progress" {
                    let bytes_transferred =
                        extract_json_u64_field(body, "bytes_transferred").unwrap_or(0);
                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        entry.status = "in_progress".to_owned();
                        entry.bytes_transferred = bytes_transferred;
                        entry.updated_at = unix_timestamp();
                        entry.updated_at_ms = unix_timestamp_millis();
                        let json_response = entry.json();
                        let entry = entry.clone();
                        drop(transfers);
                        persist_transfer_progress_record(state, &entry).await?;
                        Ok(routing::ok_response(json_response))
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                } else if action == "complete" {
                    let bytes_transferred =
                        extract_json_u64_field(body, "bytes_transferred").unwrap_or(0);
                    let status_str = extract_json_string_field(body, "status")
                        .unwrap_or_else(|| "succeeded".to_string());
                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        entry.bytes_transferred = bytes_transferred;
                        entry.status = status_str.clone();
                        entry.updated_at = unix_timestamp();
                        entry.updated_at_ms = unix_timestamp_millis();
                        let json_response = entry.json();
                        let entry_for_persistence = entry.clone();

                        // Prepare webhook dispatch
                        let webhook_event = if status_str == "succeeded" {
                            webhooks::WebhookEvent::TransferCompleted
                        } else if status_str == "failed" {
                            webhooks::WebhookEvent::TransferFailed
                        } else {
                            webhooks::WebhookEvent::TransferCompleted
                        };

                        let webhook_data = serde_json::json!({
                            "transfer_id": id,
                            "filename": entry.filename.clone(),
                            "peer_username": entry.peer_username.clone().unwrap_or_else(|| "unknown".to_string()),
                            "direction": if entry.direction == 0 { "download" } else { "upload" },
                            "size": entry.size.unwrap_or(0),
                            "bytes_transferred": bytes_transferred,
                            "status": status_str.clone(),
                        });
                        let correlation_id = format!("transfer_{}", id);

                        drop(transfers);
                        persist_transfer_record(state, &entry_for_persistence).await?;
                        maybe_import_lidarr_completed_download(state, &entry_for_persistence).await;
                        maybe_upload_ftp_completed_download(state, &entry_for_persistence).await;

                        // Dispatch webhook
                        dispatch_webhook_event(state, correlation_id, webhook_event, webhook_data)
                            .await;

                        Ok(routing::ok_response(json_response))
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                } else {
                    drop(transfers);
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::not_found_response())
            }
        }

        // TRANSFER STATISTICS ENDPOINTS
        ("GET", "/api/transfers/speeds") => {
            let transfers = state.transfers.read().await;
            let json = controller_transfer_speeds_json(&transfers);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        // USER PROFILE ENDPOINTS
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/info") => {
            let Some(username) = user_route_username(path, "/info") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_user_read_failure_response(state, route.path, &username, false).await
            {
                return Ok(response);
            }
            let users = state.users.read().await;
            if let Some(record) = users.records.iter().find(|u| u.username == username) {
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "description": "",
                        "hasFreeUploadSlot": true,
                        "hasPicture": false,
                        "picture": null,
                        "queueLength": 0,
                        "uploadSlots": 0,
                    })
                    .to_string()
                } else {
                    record.controller_info_json().to_string()
                };
                drop(users);
                Ok(routing::ok_response(json))
            } else {
                drop(users);
                if state.config.controller_profile == ControllerProfile::Legacy {
                    return Ok(routing::not_found_response());
                }
                let record = UserRecord {
                    username,
                    watched: false,
                    status: None,
                    privileged: false,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                };
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "description": "",
                        "hasFreeUploadSlot": true,
                        "hasPicture": false,
                        "picture": null,
                        "queueLength": 0,
                        "uploadSlots": 0,
                    })
                    .to_string()
                } else {
                    record.controller_info_json().to_string()
                };
                Ok(routing::ok_response(json))
            }
        }

        ("POST", path) if path.starts_with("/api/users/") && path.ends_with("/directory") => {
            let Some(username) = user_route_username(path, "/directory") else {
                return Ok(routing::not_found_response());
            };
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if route.path.starts_with("/api/v0/") && directory.trim().is_empty() {
                // UsersController validates the bound request before it
                // checks whether the Soulseek connection is ready.
                return Ok(routing::bad_request_response("directory is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let session_command_permit = if route.path.starts_with("/api/v0/") {
                match state.session_commands.reserve().await {
                    Ok(permit) => Some(permit),
                    Err(_) => {
                        return Ok(routing::service_unavailable_response(
                            "session manager is not running",
                        ));
                    }
                }
            } else {
                None
            };
            let browse = state.browse.read().await;
            let entries = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.entries.as_slice())
                .unwrap_or(&[]);
            let json = controller_user_directories_json(&directory, entries, route.query);
            drop(browse);
            if let Some(session_command_permit) = session_command_permit {
                session_command_permit.send(SessionCommand::BrowseFolder {
                    username: username.to_owned(),
                    folder: directory,
                });
            }
            Ok(routing::ok_response(json))
        }

        // USER STATUS ENDPOINTS
        ("GET", path)
            if path.starts_with("/api/users/")
                && path.ends_with("/status")
                && user_route_username(path, "/status").is_some() =>
        {
            let username = user_route_username(path, "/status").expect("guarded user status path");
            if let Some(response) =
                controller_user_read_failure_response(state, route.path, &username, false).await
            {
                return Ok(response);
            }
            let users = state.users.read().await;
            if let Some(record) = users.records.iter().find(|u| u.username == username) {
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    let status = match record.status.as_deref() {
                        Some("online") | Some("Online") => "Online",
                        Some("away") | Some("Away") => "Away",
                        _ => "Offline",
                    };
                    serde_json::json!({
                        "isPrivileged": record.privileged,
                        "presence": status,
                    })
                    .to_string()
                } else {
                    record.controller_status_json().to_string()
                };
                drop(users);
                Ok(routing::ok_response(json))
            } else {
                drop(users);
                if state.config.controller_profile == ControllerProfile::Legacy {
                    return Ok(routing::not_found_response());
                }
                let record = UserRecord {
                    username: username.to_owned(),
                    watched: false,
                    status: None,
                    privileged: false,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                };
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "isPrivileged": false,
                        "presence": "Offline",
                    })
                    .to_string()
                } else {
                    record.controller_status_json().to_string()
                };
                Ok(routing::ok_response(json))
            }
        }

        ("GET", "/api/users/groups") => {
            if state.config.controller_profile != ControllerProfile::Native {
                return Ok(HttpResponse {
                    status: "404 Not Found",
                    content_type: "",
                    body: String::new(),
                });
            }
            let mut usernames = Vec::<String>::new();
            for (key, username) in route.query.map(query_params).unwrap_or_default() {
                let username = username.trim();
                if !key.eq_ignore_ascii_case("usernames") || username.is_empty() {
                    continue;
                }
                if username.len() > MAX_USER_USERNAME_BYTES {
                    return Ok(routing::bad_request_response("username is too long"));
                }
                if usernames
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(username))
                {
                    continue;
                }
                if usernames.len() == MAX_USER_GROUP_BATCH {
                    return Ok(routing::bad_request_response(
                        "a maximum of 100 usernames is allowed",
                    ));
                }
                usernames.push(username.to_owned());
            }

            let mut groups = BTreeMap::new();
            for username in usernames {
                let group = effective_transfer_group(state, &username).await;
                groups.insert(username, group);
            }
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::to_string(&groups).unwrap_or_else(|_| "{}".to_owned()),
            })
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/group") => {
            if state.config.controller_profile != ControllerProfile::Native {
                return Ok(HttpResponse {
                    status: "404 Not Found",
                    content_type: "",
                    body: String::new(),
                });
            }
            let Some(username) = user_route_username(path, "/group") else {
                return Ok(routing::not_found_response());
            };
            let group = effective_transfer_group(state, &username).await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::to_string(&group).unwrap_or_else(|_| "\"default\"".to_owned()),
            })
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/endpoint") => {
            let Some(username) = user_route_username(path, "/endpoint") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_user_read_failure_response(state, route.path, &username, false).await
            {
                return Ok(response);
            }
            if test_user_endpoint_peer_address(state, &username).is_none()
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::not_found_response());
            }
            let address = if let Some(address) = test_user_endpoint_peer_address(state, &username) {
                address
            } else {
                match request_peer_endpoint(state, &username).await {
                    Ok(address) => address,
                    Err(_) => return Ok(routing::not_found_response()),
                }
            };
            let body = if state.config.controller_profile == ControllerProfile::Legacy {
                serde_json::json!({
                    "addressFamily": "IPv4",
                    "address": address.ip.to_string(),
                    "port": address.port,
                })
                .to_string()
            } else {
                serde_json::json!({
                    "username": username,
                    "addressFamily": "IPv4",
                    "address": address.ip.to_string(),
                    "port": address.port,
                })
                .to_string()
            };
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/soulseek/users/similar") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let body = if route.path.starts_with("/api/v0/") {
                mesh.versioned_similar_users_json(&users)
            } else {
                mesh.users_json(&users)
            };
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(body))
        }

        ("GET", path)
            if path.starts_with("/api/soulseek/users/") && path.ends_with("/interests") =>
        {
            let Some(username) = path_segment_between(path, "/api/soulseek/users/", "/interests")
            else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            if username.trim().is_empty() || username.len() > MAX_USER_USERNAME_BYTES {
                return Ok(routing::bad_request_response("username is required"));
            }
            if state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "server session is disconnected",
                ));
            }
            let (sender, receiver) =
                oneshot::channel::<Result<slskr_client::protocol::server::UserInterests, String>>();
            let key = username.to_ascii_lowercase();
            {
                let mut pending = state.pending_user_interests.write().await;
                if pending.values().map(Vec::len).sum::<usize>() >= 128 {
                    return Ok(routing::service_unavailable_response(
                        "user-interest request capacity is full",
                    ));
                }
                pending.entry(key).or_default().push(sender);
            }
            if let Err(error) = send_session_command(
                state,
                SessionCommand::RequestUserInterests(username.clone()),
            )
            .await
            {
                state
                    .pending_user_interests
                    .write()
                    .await
                    .remove(&username.to_ascii_lowercase());
                return Ok(routing::service_unavailable_response(&error));
            }
            let interests = match time::timeout(
                state.config.soulseek_connection.timeout_inactivity,
                receiver,
            )
            .await
            {
                Ok(Ok(Ok(interests))) => interests,
                Ok(Ok(Err(error))) => {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(Err(_)) => {
                    return Ok(routing::service_unavailable_response(
                        "user-interest request was cancelled",
                    ));
                }
                Err(_) => {
                    state
                        .pending_user_interests
                        .write()
                        .await
                        .remove(&username.to_ascii_lowercase());
                    return Ok(routing::service_unavailable_response(
                        "user-interest request timed out",
                    ));
                }
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "username": interests.username,
                    "liked": interests.liked,
                    "hated": interests.hated,
                })
                .to_string(),
            ))
        }

        ("DELETE", "/api/searches") => {
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let cleared_count = searches.records.len();
            searches.records.clear();
            let mutated_searches = searches.clone();
            drop(searches);
            if let Err(error) = clear_persisted_searches(state).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            let json = if route.path.starts_with("/api/v0/") {
                format!("{{\"deleted\":{cleared_count}}}")
            } else {
                format!("{{\"cleared\":{cleared_count}}}")
            };
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/searches/") && path.ends_with("/responses") => {
            let Some(id) = path
                .strip_prefix("/api/searches/")
                .and_then(|value| value.strip_suffix("/responses"))
                .filter(|value| !value.is_empty() && !value.contains('/'))
            else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_search_responses_read_failure_response(state, route.path, id).await
            {
                return Ok(response);
            }
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(id) {
                let json = record.controller_responses_json_with_query(route.query);
                drop(searches);
                Ok(routing::ok_response(json))
            } else {
                drop(searches);
                Ok(routing::ok_response("[]".to_string()))
            }
        }

        ("GET", path) if path.starts_with("/api/searches/") => {
            let Some(id) = path_segment_after(path, "/api/searches/") else {
                return Ok(routing::not_found_response());
            };
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(id) {
                let json = record.json_with_query(route.query);
                drop(searches);
                Ok(routing::ok_response(json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("DELETE", path)
            if path.starts_with("/api/searches/")
                || route.normalized_path.starts_with("/api/v0/searches/") =>
        {
            let Some(token_str) = path_segment_after(path, "/api/searches/")
                .or_else(|| path_segment_after(route.normalized_path, "/api/v0/searches/"))
            else {
                return Ok(routing::not_found_response());
            };
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let removed = searches.remove_by_identifier(token_str);
            let mutated_searches = searches.clone();
            drop(searches);
            if let Some(record) = removed.as_ref() {
                if let Err(error) = delete_persisted_search(state, record).await {
                    rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                        .await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            if route.path.starts_with("/api/v0/")
                && matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                )
                && removed.is_some()
            {
                Ok(routing::no_content_response())
            } else if route.path.starts_with("/api/v0/") && removed.is_none() {
                Ok(routing::not_found_response())
            } else {
                Ok(routing::ok_response("{}".to_string()))
            }
        }

        // MESSAGE ENDPOINTS
        ("POST", "/api/messages") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let message_body = match extract_json_string_field(body, "body") {
                Some(b) => b,
                None => return Ok(routing::bad_request_response("body is required")),
            };

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "outbound", message_body.clone());
            let mutated = messages.clone();
            let message_id = record.id;
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::MessageUser {
                username: username.clone(),
                body: message_body.clone(),
            });
            record_event(
                state,
                "message.sent",
                username.clone(),
                Some(format!("id={message_id}")),
            )
            .await;
            // Dispatch webhook for message.sent event
            let webhook_data = serde_json::json!({
                "message_id": message_id,
                "username": username.clone(),
                "body": message_body.clone(),
                "direction": "outbound",
            });
            let correlation_id = format!("message_{}", message_id);

            dispatch_webhook_event(
                state,
                correlation_id,
                webhooks::WebhookEvent::MessageSent,
                webhook_data,
            )
            .await;

            Ok(routing::created_response(record.json()))
        }

        ("POST", "/api/messages/inbound") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let message_body = match extract_json_string_field(body, "body") {
                Some(b) => b,
                None => return Ok(routing::bad_request_response("body is required")),
            };

            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "inbound", message_body.clone());
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            record_event(
                state,
                "message.received",
                "messages",
                Some(format!("id={}", record.id)),
            )
            .await;

            Ok(routing::created_response(record.json()))
        }

        ("POST", _path) if message_ack_path(normalized_path).is_some() => {
            let Some(id) = message_ack_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let Ok(protocol_id) = u32::try_from(id) else {
                return Ok(routing::bad_request_response(
                    "message id exceeds u32 range",
                ));
            };
            if !state
                .messages
                .read()
                .await
                .records
                .iter()
                .any(|message| message.id == id)
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut messages = state.messages.write().await;
            let previous = messages.clone();

            if let Some(record) = messages.ack(id) {
                let mutated = messages.clone();
                let username = record.username.clone();
                let json_response = record.json();
                drop(messages);
                if let Err(error) = persist_message_ack_checked(state, id).await {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                record_event(state, "message.acked", username, Some(format!("id={id}"))).await;

                session_command_permit.send(SessionCommand::MessageAcked { id: protocol_id });

                Ok(routing::ok_response(json_response))
            } else {
                drop(messages);
                Ok(routing::not_found_response())
            }
        }

        ("PUT", _path) if message_ack_path(normalized_path).is_some() => {
            let Some(id) = message_ack_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let Ok(protocol_id) = u32::try_from(id) else {
                return Ok(routing::bad_request_response(
                    "message id exceeds u32 range",
                ));
            };
            if !state
                .messages
                .read()
                .await
                .records
                .iter()
                .any(|message| message.id == id)
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut messages = state.messages.write().await;
            let previous = messages.clone();

            if let Some(record) = messages.ack(id) {
                let mutated = messages.clone();
                let username = record.username.clone();
                let json_response = record.json();
                drop(messages);
                if let Err(error) = persist_message_ack_checked(state, id).await {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                record_event(state, "message.acked", username, Some(format!("id={id}"))).await;

                session_command_permit.send(SessionCommand::MessageAcked { id: protocol_id });

                Ok(routing::ok_response(json_response))
            } else {
                drop(messages);
                Ok(routing::not_found_response())
            }
        }

        ("GET", _path) if messages_user_path(normalized_path).is_some() => {
            let Some(username) = messages_user_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let messages = state.messages.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: messages.json_for_user(username, route.query),
            })
        }

        // ROOM ENDPOINTS
        ("POST", "/api/rooms/refresh") => {
            if send_session_command(state, SessionCommand::RefreshRooms)
                .await
                .is_err()
            {
                return Ok(routing::service_unavailable_response(
                    "session manager is not running",
                ));
            }
            Ok(routing::accepted_response("{}".to_string()))
        }

        ("POST", _path) if room_join_path(normalized_path).is_some() => {
            let Some(room_name) = room_join_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let mut rooms = state.rooms.write().await;
            if state.session.read().await.state == "connected"
                && rooms.records.iter().any(|record| {
                    record.name == bounded_room_name(room_name)
                        && record.joined
                        && record.last_error.is_none()
                })
            {
                let existing = rooms
                    .records
                    .iter()
                    .find(|record| record.name == bounded_room_name(room_name))
                    .cloned()
                    .expect("joined room exists");
                drop(rooms);
                return Ok(routing::ok_response(
                    existing.controller_room_json().to_string(),
                ));
            }
            let previous = rooms.clone();
            let Some(record) = rooms.join(room_name.to_string()) else {
                return Ok(routing::service_unavailable_response(
                    "room capacity is full",
                ));
            };
            if let Err(error) = persist_room_join_checked(state, room_name).await {
                *rooms = previous;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(rooms);
            record_event(state, "room.joined", room_name.to_string(), None).await;

            send_room_join_if_connected(state, room_name.to_string()).await;

            Ok(routing::created_response(record.json()))
        }

        ("DELETE", _path) if room_join_path(normalized_path).is_some() => {
            let Some(room_name) = room_join_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let mut rooms = state.rooms.write().await;
            let previous = rooms.clone();

            if let Some(record) = rooms.leave(room_name) {
                let json_response = record.json();
                if let Err(error) = persist_room_leave_checked(state, room_name).await {
                    *rooms = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(rooms);
                record_event(state, "room.left", room_name.to_string(), None).await;

                send_room_leave_if_connected(state, room_name.to_string()).await;

                Ok(routing::ok_response(json_response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn route_dispatch_group_3(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("POST", _path) if room_messages_path(normalized_path).is_some() => {
            let Some(room_name) = room_messages_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let username = extract_json_string_field(body, "username")
                .unwrap_or_else(|| "unknown".to_string());
            let message_body = extract_json_string_field(body, "body")
                .or_else(|| json_body_string(body))
                .unwrap_or_default();

            if !state
                .rooms
                .read()
                .await
                .records
                .iter()
                .any(|room| room.name == room_name)
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut rooms = state.rooms.write().await;
            if let Some(record) =
                rooms.add_message(room_name, username.clone(), message_body.clone())
            {
                let json_response = record.json();
                drop(rooms);
                session_command_permit.send(SessionCommand::SayRoom {
                    room: room_name.to_string(),
                    body: message_body,
                });
                record_event(
                    state,
                    "room.message",
                    room_name.to_string(),
                    Some(format!("username={username}")),
                )
                .await;

                Ok(routing::ok_response(json_response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("GET", "/api/rooms/available") => {
            if state.config.controller_profile == ControllerProfile::Legacy
                && route.path.starts_with("/api/v0/")
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::internal_server_error_response(
                    "failed to retrieve available rooms",
                ));
            }
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            let rooms = state.rooms.read().await;
            let json = rooms.controller_available_json();
            drop(rooms);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/rooms/activity") => {
            let session = state.session.read().await;
            let local_username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            drop(session);
            let rooms = state.rooms.read().await;
            let json = rooms.activity_json(&local_username);
            drop(rooms);
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/users")
                && joined_room_subresource(path, "/users").is_some() =>
        {
            let room_name =
                joined_room_subresource(path, "/users").expect("guarded joined-room users path");
            let session = state.session.read().await;
            let local_username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            drop(session);
            let rooms = state.rooms.read().await;
            if let Some(room) = rooms.records.iter().find(|r| r.name == room_name) {
                // Matches the oracle's real GetUsersByRoomName: the room's
                // real roster (from the server's JoinedRoom snapshot), not
                // a hardcoded empty list.
                let json = serde_json::Value::Array(
                    room.roster
                        .iter()
                        .map(|user| user.controller_json(&local_username))
                        .collect(),
                )
                .to_string();
                drop(rooms);
                Ok(routing::ok_response(json))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("GET", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/messages")
                && joined_room_subresource(path, "/messages").is_some() =>
        {
            let room_name = joined_room_subresource(path, "/messages")
                .expect("guarded joined-room messages path");
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let rooms = state.rooms.read().await;
            if let Some(room) = rooms.records.iter().find(|r| r.name == room_name) {
                let messages = room
                    .messages
                    .iter()
                    .filter(|message| since.is_none_or(|since| message.created_at_ms > since))
                    .map(|message| message.controller_json(&room.name))
                    .collect::<Vec<_>>();
                let json = serde_json::Value::Array(messages).to_string();
                drop(rooms);
                Ok(routing::ok_response(json))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        // USER ENDPOINTS
        ("POST", "/api/users/watch") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            {
                let users = state.users.read().await;
                let bounded_username = bounded_user_username(&username);
                if users.records.len() >= users.max_records
                    && !users
                        .records
                        .iter()
                        .any(|record| record.username == bounded_username)
                {
                    return Ok(routing::service_unavailable_response(
                        "user watch capacity is full",
                    ));
                }
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut users = state.users.write().await;
            let previous_updated_at = users.updated_at;
            let bounded_username = bounded_user_username(&username);
            let previous_record = users
                .records
                .iter()
                .find(|record| record.username == bounded_username)
                .cloned();
            let Some(record) = users.watch(username.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "user watch capacity is full",
                ));
            };
            drop(users);

            if let Err(error) = persist_user_projection(state, &record).await {
                let mut users = state.users.write().await;
                match previous_record {
                    Some(previous) => {
                        if let Some(current) = users.records.iter_mut().find(|current| {
                            current.username == bounded_username && **current == record
                        }) {
                            *current = previous;
                        }
                    }
                    None => users.records.retain(|current| {
                        current.username != bounded_username || *current != record
                    }),
                }
                users.updated_at = users
                    .records
                    .iter()
                    .map(|current| current.updated_at)
                    .max()
                    .unwrap_or(previous_updated_at);
                drop(users);
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::WatchUser(username));

            Ok(routing::created_response(record.json()))
        }

        ("DELETE", _path) if user_watch_path(normalized_path).is_some() => {
            let Some(username) = user_watch_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            if !state
                .users
                .read()
                .await
                .records
                .iter()
                .any(|record| record.username == bounded_user_username(username))
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut users = state.users.write().await;
            let previous_updated_at = users.updated_at;
            let previous_record = users
                .records
                .iter()
                .find(|record| record.username == bounded_user_username(username))
                .cloned();

            if let Some(record) = users.unwatch(username) {
                drop(users);

                if let Err(error) = persist_user_projection(state, &record).await {
                    let mut users = state.users.write().await;
                    if let Some(previous) = previous_record {
                        if let Some(current) = users.records.iter_mut().find(|current| {
                            current.username == previous.username && **current == record
                        }) {
                            *current = previous;
                        }
                    }
                    users.updated_at = users
                        .records
                        .iter()
                        .map(|current| current.updated_at)
                        .max()
                        .unwrap_or(previous_updated_at);
                    drop(users);
                    return Ok(routing::service_unavailable_response(&error));
                }
                session_command_permit.send(SessionCommand::UnwatchUser(username.to_string()));

                Ok(routing::ok_response(record.json()))
            } else {
                drop(users);
                Ok(routing::not_found_response())
            }
        }

        ("POST", _path) if user_stats_request_path(normalized_path).is_some() => {
            let Some(username) = user_stats_request_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            if send_session_command(
                state,
                SessionCommand::RequestUserStats(username.to_string()),
            )
            .await
            .is_err()
            {
                return Ok(routing::service_unavailable_response(
                    "session manager is not running",
                ));
            }
            Ok(routing::accepted_response(format!(
                "{{\"username\":\"{}\"}}",
                json_escape(username)
            )))
        }

        ("POST", _path) if user_browse_request_path(normalized_path).is_some() => {
            let Some(username) = user_browse_request_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };

            let connected = {
                let session = state.session.read().await;
                session.state == "connected"
            };
            if !connected {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            if state.config.controller_profile == ControllerProfile::Native
                && !state.soulseek_safety.try_consume_browse("compatibility")
            {
                return Ok(HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "application/json; charset=utf-8",
                    body: serde_json::json!({
                        "error": "Browse rate limit exceeded. See Soulseek safety configuration."
                    })
                    .to_string(),
                });
            }

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.request(username.to_string()) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);

            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::BrowseUser(username.to_string()));

            Ok(routing::accepted_response(record.json()))
        }

        ("POST", _path) if user_browse_folder_path(normalized_path).is_some() => {
            let Some(username) = user_browse_folder_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let folder = extract_json_string_field(body, "folder").unwrap_or_default();

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.request_folder(username.to_string(), folder.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);

            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::BrowseFolder {
                username: username.to_string(),
                folder,
            });

            Ok(routing::accepted_response(record.json()))
        }

        ("POST", _path) if user_browse_fail_path(normalized_path).is_some() => {
            let Some(username) = user_browse_fail_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let reason = extract_json_string_field(body, "reason").unwrap_or_default();

            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.fail(username.to_owned(), reason.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);
            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }

            Ok(routing::ok_response(format!(
                "{{\"username\":\"{}\",\"status\":\"failed\",\"reason\":\"{}\"}}",
                json_escape(username),
                json_escape(&reason)
            )))
        }

        ("POST", _path) if user_browse_cancel_path(normalized_path).is_some() => {
            let Some(username) = user_browse_cancel_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let reason = extract_json_string_field(body, "reason")
                .unwrap_or_else(|| "cancelled by client".to_owned());

            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.cancel(username, reason) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            let body = record.json();
            drop(browse);
            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }

            Ok(routing::ok_response(body))
        }

        // BROWSE-RESPONSE ENDPOINT
        ("POST", "/api/browse-responses") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let complete = extract_json_bool_field(body, "complete").unwrap_or(true);

            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload) => payload,
                Err(_) => return Ok(routing::bad_request_response("invalid JSON body")),
            };

            let mut entries = Vec::new();
            if let Some(array) = payload.get("entries").and_then(serde_json::Value::as_array) {
                entries.extend(
                    array
                        .iter()
                        .filter_map(|entry| BrowseEntry::from_json_file(entry, None)),
                );
            }

            if let Some(directories) = payload
                .get("directories")
                .and_then(serde_json::Value::as_array)
            {
                for directory in directories {
                    let directory_name = directory
                        .get("name")
                        .or_else(|| directory.get("directory"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    if let Some(files) =
                        directory.get("files").and_then(serde_json::Value::as_array)
                    {
                        entries.extend(files.iter().filter_map(|file| {
                            BrowseEntry::from_json_file(file, Some(directory_name))
                        }));
                    }
                }
            }

            // Fallback for single entry format (backward compatibility)
            if entries.is_empty() {
                if let Some(entry) = BrowseEntry::from_json_file(&payload, None) {
                    entries.push(entry);
                }
            }

            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.add_entries(username, entries, complete) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);
            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }

            Ok(routing::ok_response(record.json()))
        }
        ("GET", "/api/browse") | ("GET", "/api/v0/browse") => {
            let browse = state.browse.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: browse.json(route.query),
            })
        }
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/browse") => {
            if let Some(username) = user_route_username(path, "/browse") {
                if let Some(response) =
                    controller_user_read_failure_response(state, route.path, &username, true).await
                {
                    return Ok(response);
                }
                let browse = state.browse.read().await;
                let record = browse
                    .records
                    .iter()
                    .find(|record| record.username == username);
                if state.config.controller_profile == ControllerProfile::Native && record.is_none()
                {
                    drop(browse);
                    let session_state = state.session.read().await.state;
                    if session_state != "connected" {
                        return Ok(HttpResponse {
                            status: "503 Service Unavailable",
                            content_type: "application/json",
                            body: serde_json::to_string("Soulseek server connection is not ready")
                                .unwrap_or_else(|_| {
                                    "\"Soulseek server connection is not ready\"".to_owned()
                                }),
                        });
                    }
                    return Ok(routing::not_found_response());
                }
                if state.config.controller_profile == ControllerProfile::Legacy && record.is_none()
                {
                    drop(browse);
                    let session_state = state.session.read().await.state;
                    if session_state != "connected" {
                        let display_state = match session_state {
                            "connecting" => "Connecting",
                            "disconnecting" => "Disconnecting",
                            _ => "Disconnected",
                        };
                        let message = format!(
                            "The server connection must be connected and logged in to browse (currently: {display_state})"
                        );
                        return Ok(HttpResponse {
                            status: "500 Internal Server Error",
                            content_type: "application/json",
                            body: serde_json::to_string(&message)
                                .unwrap_or_else(|_| "\"browse failed\"".to_owned()),
                        });
                    }
                    return Ok(routing::not_found_response());
                }
                let entries = record
                    .map(|record| record.entries.as_slice())
                    .unwrap_or(&[]);
                let body = controller_user_root_json(entries, route.query);
                drop(browse);
                Ok(routing::ok_response(body))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // GET browse requests list
        ("GET", "/api/browse/requests") => {
            let browse = state.browse.read().await;
            let requests = browse
                .records
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "username": r.username,
                        "status": r.status,
                        "requested_at": r.requested_at,
                        "updated_at": r.updated_at,
                    })
                })
                .collect::<Vec<_>>();
            drop(browse);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::to_string(
                    &serde_json::json!({"requests": requests, "count": requests.len()}),
                )
                .unwrap_or_else(|_| "{}".to_string()),
            })
        }
        // WEBHOOK MANAGEMENT ROUTES
        ("POST", "/api/admin/webhooks") => {
            let url = match extract_json_string_field(body, "url") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("url is required")),
            };
            if url.len() > 2048 {
                return Ok(routing::bad_request_response("url is too long"));
            }
            if let Err(error) = webhooks::validate_webhook_url_for_registration(&url) {
                return Ok(routing::bad_request_response(&error.to_string()));
            }
            let secret = match extract_json_string_field(body, "secret") {
                Some(secret) => {
                    if let Err(error) = webhooks::validate_webhook_secret(&secret) {
                        return Ok(routing::bad_request_response(error));
                    }
                    secret
                }
                None => {
                    let Some(secret) = webhooks::Webhook::generate_secret() else {
                        return Ok(routing::service_unavailable_response(
                            "webhook secret generation unavailable",
                        ));
                    };
                    secret
                }
            };
            let webhook = webhooks::Webhook::new(
                url,
                vec![webhooks::WebhookEvent::SearchCreated],
                secret.clone(),
            );
            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            let webhook_id = match webhooks.register(webhook.clone()) {
                Ok(id) => id,
                Err(_) => {
                    drop(webhooks);
                    return Ok(routing::bad_request_response("webhook limit reached"));
                }
            };
            let mutated = webhooks.clone();
            drop(webhooks);
            if let Err(error) = persist_webhook_checked(state, &webhook).await {
                rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(
                serde_json::json!({
                    "id": webhook_id,
                    "secret": secret,
                    "secretReturnedOnce": true,
                    "status": "created"
                })
                .to_string(),
            ))
        }
        ("GET", "/api/admin/webhooks") => {
            let webhooks = state.webhooks.read().await;
            let webhook_list: Vec<serde_json::Value> = webhooks
                .get_all()
                .iter()
                .map(|w| {
                    serde_json::json!({
                        "id": w.id,
                        "url": w.url,
                        "events": w.events.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                        "active": w.active,
                        "created_at": w.created_at,
                        "last_triggered": w.last_triggered,
                    })
                })
                .collect();
            let total = webhook_list.len();
            drop(webhooks);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::json!({"webhooks": webhook_list, "total": total}).to_string(),
            })
        }
        ("DELETE", path)
            if path.starts_with("/api/admin/webhooks/")
                && webhook_resource_id(path, "/api/admin/webhooks/").is_some() =>
        {
            let webhook_id = webhook_resource_id(path, "/api/admin/webhooks/")
                .expect("guarded admin webhook resource path");
            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            if webhooks.unregister(webhook_id).is_some() {
                let mutated = webhooks.clone();
                drop(webhooks);
                if let Err(error) = persist_webhook_delete_checked(state, webhook_id).await {
                    rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{\"status\":\"deleted\"}".to_owned()))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/admin/webhooks/")
                && path.ends_with("/test")
                && webhook_test_id(path, "/api/admin/webhooks/").is_some() =>
        {
            let webhook_id = webhook_test_id(path, "/api/admin/webhooks/")
                .expect("guarded admin webhook test path");
            let webhooks = state.webhooks.read().await;
            if let Some(webhook) = webhooks.get(webhook_id) {
                let payload = webhooks::WebhookDispatcher::test_payload(
                    webhooks::WebhookEvent::SearchCreated,
                    "test webhook delivery",
                );
                let webhook_clone = webhook.clone();
                drop(webhooks);
                let Ok(delivery_permit) = Arc::clone(&state.webhook_deliveries).try_acquire_owned()
                else {
                    return Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "application/json",
                        body: "{\"error\":\"too many webhook deliveries in progress\"}".to_owned(),
                    });
                };

                tokio::spawn(async move {
                    let _delivery_permit = delivery_permit;
                    let _ = webhooks::WebhookDispatcher::send_webhook(
                        &webhook_clone.url,
                        &webhook_clone.secret,
                        &payload.to_string(),
                        webhook_clone.timeout_seconds,
                    )
                    .await;
                });

                Ok(routing::ok_response(
                    "{\"status\":\"test_sent\"}".to_owned(),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }
        // DATABASE MANAGEMENT ROUTES
        ("GET", "/api/admin/database/stats") => Ok(routing::ok_response(
            database_stats_value(state).await.to_string(),
        )),
        ("POST", "/api/admin/database/cleanup") => Ok(routing::ok_response(
            database_cleanup_value(state, body).await.to_string(),
        )),
        ("POST", "/api/admin/database/vacuum") => Ok(routing::ok_response(
            database_vacuum_value(state).await.to_string(),
        )),
        // API KEYS MANAGEMENT ROUTES
        ("POST", "/api/admin/keys") => Ok(routing::created_response(
            "{\"id\":null,\"created\":false,\"reason\":\"static SLSKR_API_TOKEN auth is active\"}"
                .to_owned(),
        )),
        ("GET", "/api/admin/keys") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: r#"{"keys":[],"total":0}"#.to_owned(),
        }),
        ("DELETE", path) if path.starts_with("/api/admin/keys/") => {
            let key_id = path.rsplit('/').next().unwrap_or("");
            Ok(routing::ok_response(format!(
                "{{\"id\":\"{}\",\"revoked\":false,\"reason\":\"static token auth\"}}",
                json_escape(key_id)
            )))
        }
        ("GET", "/api/admin/keys/validate") => {
            Ok(routing::ok_response("{\"valid\":true}".to_owned()))
        }
        // MONITORING & TELEMETRY ROUTES (already exist but adding for completeness)
        ("GET", "/api/admin/monitoring") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: r#"{"cpu_percent":5.2,"memory_mb":128,"uptime_seconds":3600}"#.to_owned(),
        }),
        // WEBUI PARITY: Room routes with /joined prefix
        ("GET", "/api/rooms/joined") => {
            let rooms = state.rooms.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: rooms.joined_names_json(),
            })
        }
        ("POST", "/api/rooms/joined") => {
            let Some(room_name) = extract_json_string_field(body, "room")
                .or_else(|| extract_json_string_field(body, "name"))
                .or_else(|| json_body_string(body))
                .filter(|room| !room.trim().is_empty())
            else {
                return Ok(if route.path.starts_with("/api/v0/") {
                    rooms_controller_value_bad_request_response("roomName is required")
                } else {
                    routing::bad_request_response("room is required")
                });
            };
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek is reconnecting; try again shortly.",
                ));
            }
            let mut rooms = state.rooms.write().await;
            if route.path.starts_with("/api/v0/")
                && rooms.records.iter().any(|record| {
                    record.name == bounded_room_name(&room_name)
                        && record.joined
                        && record.last_error.is_none()
                })
            {
                let existing = rooms
                    .records
                    .iter()
                    .find(|record| record.name == bounded_room_name(&room_name))
                    .cloned()
                    .expect("joined room exists");
                drop(rooms);
                return Ok(if route.path.starts_with("/api/v0/") {
                    HttpResponse {
                        status: "200 OK",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response(existing.controller_room_json().to_string())
                });
            }
            let previous = rooms.clone();
            let Some(record) = rooms.join(room_name.to_string()) else {
                return Ok(routing::service_unavailable_response(
                    "room capacity is full",
                ));
            };
            let body = record.controller_room_json().to_string();
            // The legacy compatibility contract persists this controller's
            // subscription, while the native profile fork keeps the tracker transient.
            if state.config.controller_profile == ControllerProfile::Legacy {
                if let Err(error) = persist_room_join_checked(state, &room_name).await {
                    *rooms = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            drop(rooms);
            record_event(state, "room.joined", room_name.clone(), None).await;

            send_room_join_if_connected(state, room_name).await;

            Ok(routing::created_response(body))
        }
        ("GET", path)
            if path.starts_with("/api/rooms/joined/") && path.matches('/').count() == 4 =>
        {
            let room_name = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let rooms = state.rooms.read().await;
            let response = rooms
                .records
                .iter()
                .find(|r| {
                    r.name == room_name
                        && (state.config.controller_profile != ControllerProfile::Legacy
                            || r.joined)
                })
                .map(|room| routing::ok_response(room.controller_room_json().to_string()))
                .unwrap_or_else(routing::not_found_response);
            drop(rooms);
            Ok(response)
        }
        ("POST", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/messages")
                && joined_room_subresource(path, "/messages").is_some() =>
        {
            let room_name = joined_room_subresource(path, "/messages")
                .expect("guarded joined-room messages path");
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            if message_body.trim().is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "message is required",
                ));
            }
            if !state
                .rooms
                .read()
                .await
                .records
                .iter()
                .any(|room| room.name == room_name)
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut rooms = state.rooms.write().await;
            if rooms
                .add_message(&room_name, "local".to_owned(), message_body.clone())
                .is_some()
            {
                drop(rooms);
                session_command_permit.send(SessionCommand::SayRoom {
                    room: room_name.to_owned(),
                    body: message_body,
                });
                record_event(
                    state,
                    "room.message",
                    room_name.to_owned(),
                    Some("username=local".to_owned()),
                )
                .await;
                Ok(
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response("true".to_owned())
                    },
                )
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/ticker")
                && joined_room_subresource(path, "/ticker").is_some() =>
        {
            let room_name =
                joined_room_subresource(path, "/ticker").expect("guarded joined-room ticker path");
            let ticker = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "ticker"))
                .or_else(|| extract_json_string_field(body, "message"))
                .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
            if ticker.trim().is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "message is required",
                ));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut rooms = state.rooms.write().await;
            let response = rooms
                .set_ticker(&room_name, ticker.clone())
                .map(|room| {
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response(
                            serde_json::json!({
                                "updated": true,
                                "room": room.controller_room_json(),
                            })
                            .to_string(),
                        )
                    }
                })
                .unwrap_or_else(routing::not_found_response);
            drop(rooms);
            if response.status != "404 Not Found" {
                session_command_permit.send(SessionCommand::SetRoomTicker {
                    room: room_name.to_owned(),
                    ticker,
                });
            }
            Ok(response)
        }
        ("POST", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/members")
                && joined_room_subresource(path, "/members").is_some() =>
        {
            let room_name = joined_room_subresource(path, "/members")
                .expect("guarded joined-room members path");
            let username = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "username"))
                .or_else(|| extract_json_string_field(body, "name"))
                .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
            if username.trim().is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "username is required",
                ));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut rooms = state.rooms.write().await;
            let response = match rooms.add_member(&room_name, username.clone()) {
                Ok(Some(room)) => {
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response(
                            serde_json::json!({
                                "updated": true,
                                "room": room.controller_room_json(),
                                "userCount": room.user_count.unwrap_or(0),
                            })
                            .to_string(),
                        )
                    }
                }
                Ok(None) => routing::not_found_response(),
                Err(()) => routing::service_unavailable_response("room member capacity is full"),
            };
            drop(rooms);
            if response.status != "404 Not Found" && response.status != "503 Service Unavailable" {
                session_command_permit.send(SessionCommand::AddRoomMember {
                    room: room_name.to_owned(),
                    username,
                });
            }
            if response.status == "200 OK" {
                record_event(state, "room.users.updated", room_name.to_string(), None).await;
            }
            Ok(response)
        }
        ("DELETE", path)
            if path.starts_with("/api/rooms/joined/") && path.matches('/').count() == 4 =>
        {
            let room_name = path.strip_prefix("/api/rooms/joined/").unwrap_or("");
            let room_name = decoded_path_segment(room_name).trim().to_owned();
            if room_name.is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "roomName is required",
                ));
            }
            let mut rooms = state.rooms.write().await;
            let previous = rooms.clone();

            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
                && !rooms
                    .records
                    .iter()
                    .any(|record| record.name == room_name && record.joined)
            {
                drop(rooms);
                return Ok(routing::not_found_response());
            }

            if let Some(record) = rooms.leave(&room_name) {
                let json_response = record.json();
                if let Err(error) = persist_room_leave_checked(state, &room_name).await {
                    *rooms = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(rooms);
                record_event(state, "room.left", room_name.to_string(), None).await;

                send_room_leave_if_connected(state, room_name.to_string()).await;

                Ok(
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        routing::no_content_response()
                    } else {
                        routing::ok_response(json_response)
                    },
                )
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        // GET room detail by name
        ("GET", path)
            if path.starts_with("/api/rooms/")
                && !path.ends_with("/messages")
                && !path.ends_with("/users")
                && path.matches('/').count() == 3 =>
        {
            let room_name = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let rooms = state.rooms.read().await;
            if let Some(record) = rooms.records.iter().find(|r| r.name == room_name) {
                Ok(routing::ok_response(record.json()))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        // WEBUI PARITY: Application/Server/Session status endpoints
        ("GET", "/api/application/build") => {
            let mut value = controller_version_json(state);
            value["protocol"] = serde_json::json!({
                "clientName": CLIENT_NAME,
                "major": CLIENT_MAJOR_VERSION,
                "minor": CLIENT_MINOR_VERSION,
            });
            Ok(routing::ok_response(value.to_string()))
        }
        // WEBUI PARITY: Options/Config read-write endpoints
        ("GET", "/api/private-message-auto-response") => {
            let settings = state.private_message_auto_response_settings.read().await;
            let mut value =
                serde_json::from_str::<serde_json::Value>(&settings.sanitized_json())
                    .map_err(|error| format!("auto-response settings json failed: {error}"))?;
            value["runtimeMutable"] = serde_json::Value::Bool(true);
            value["persisted"] = serde_json::Value::Bool(false);
            Ok(routing::ok_response(value.to_string()))
        }
        ("PUT", "/api/private-message-auto-response") => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(payload)) => payload,
                _ => {
                    return Ok(routing::bad_request_response(
                        "JSON object body is required",
                    ))
                }
            };
            let enabled = payload.get("enabled").and_then(serde_json::Value::as_bool);
            let message = payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(str::to_owned);
            if message.as_deref().is_some_and(|message| message.is_empty()) {
                return Ok(routing::bad_request_response(
                    "auto-response message must not be blank",
                ));
            }
            if message
                .as_ref()
                .is_some_and(|message| message.len() > 4_096)
            {
                return Ok(routing::bad_request_response(
                    "auto-response message exceeds 4096 bytes",
                ));
            }
            let cooldown = payload
                .get("cooldownMinutes")
                .or_else(|| payload.get("cooldown_minutes"))
                .and_then(serde_json::Value::as_u64);
            if cooldown.is_some_and(|cooldown| !(1..=1_440).contains(&cooldown)) {
                return Ok(routing::bad_request_response(
                    "cooldownMinutes must be between 1 and 1440",
                ));
            }
            if enabled.is_none() && message.is_none() && cooldown.is_none() {
                return Ok(routing::bad_request_response(
                    "enabled, message, or cooldownMinutes is required",
                ));
            }
            let mut settings = state.private_message_auto_response_settings.write().await;
            if let Some(enabled) = enabled {
                settings.enabled = enabled;
            }
            if let Some(message) = message {
                settings.message = message;
            }
            if let Some(cooldown) = cooldown {
                settings.cooldown_minutes = cooldown;
            }
            let disabled = !settings.enabled;
            let mut value =
                serde_json::from_str::<serde_json::Value>(&settings.sanitized_json())
                    .map_err(|error| format!("auto-response settings json failed: {error}"))?;
            drop(settings);
            if disabled {
                *state.private_message_auto_responses.write().await =
                    PrivateMessageAutoResponseTracker::default();
            }
            value["runtimeMutable"] = serde_json::Value::Bool(true);
            value["persisted"] = serde_json::Value::Bool(false);
            record_event(
                state,
                "message.auto_response_settings_updated",
                "private-message-auto-response",
                Some(format!("enabled={}", value["enabled"])),
            )
            .await;
            Ok(routing::ok_response(value.to_string()))
        }
        ("GET", "/api/options") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            let overlay = state.options_overlay.read().await;
            let body = controller_options_json(&state.config, &overlay, true);
            drop(overlay);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/options/startup") => {
            let overlay = state.options_overlay.read().await;
            let body = controller_options_json(&state.config, &overlay, false);
            drop(overlay);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/options/yaml") => {
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(controller_options_config_text_response(&state.config))
        }
        ("GET", "/api/options/debug") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) || !state.config.controller_debug {
                return Ok(controller_forbidden_response());
            }
            let overlay = state.options_overlay.read().await;
            let debug_view = controller_options_debug_view(state, &overlay);
            drop(overlay);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::Value::String(debug_view).to_string(),
            })
        }
        ("GET", "/api/options/yaml/location") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: controller_options_config_location_json(&state.config),
            })
        }
        ("GET", "/api/autoreplace") => {
            let json = if route.path.starts_with("/api/v0/") {
                let enabled = state.runtime.read().await.autoreplace_enabled;
                serde_json::json!({
                    "enabled": enabled,
                    "lastRunAt": null,
                    "lastRunProcessedCount": 0,
                    "lastRunReplacedCount": 0,
                    "intervalSeconds": 300,
                })
                .to_string()
            } else {
                "{\"enabled\":false,\"intervalSeconds\":60,\"stuckCount\":0,\"lastRunProcessedCount\":0,\"lastRunReplacedCount\":0,\"rules\":[],\"count\":0}".to_string()
            };
            Ok(routing::ok_response(json))
        }
        ("PUT", "/api/options") => {
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(apply_controller_options_overlay(body, state).await)
        }
        // HEALTH & DIAGNOSTICS ENDPOINTS
        ("GET", "/api/health/detailed") => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;
            let messages = state.messages.read().await;
            let users = state.users.read().await;

            let diagnostics = serde_json::json!({
                "status": "operational",
                "transfers": {
                    "active": transfers.entries.iter().filter(|t| t.status == "in_progress").count(),
                    "total": transfers.entries.len(),
                    "succeeded": transfers.entries.iter().filter(|t| t.status == "succeeded").count(),
                    "failed": transfers.entries.iter().filter(|t| t.status == "failed").count(),
                },
                "searches": {
                    "total": searches.records.len(),
                },
                "messages": {
                    "total": messages.records.len(),
                    "unread": messages.records.iter().filter(|m| !m.acknowledged).count(),
                },
                "users": {
                    "total": users.records.len(),
                },
            }).to_string();

            drop(transfers);
            drop(searches);
            drop(messages);
            drop(users);

            Ok(routing::ok_response(diagnostics))
        }

        ("GET", "/api/diagnostics") => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;

            let diag = serde_json::json!({
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "transfers": {
                    "queue_size": transfers.entries.len(),
                    "active_downloads": transfers.entries.iter().filter(|t| t.status == "in_progress" && t.direction == 0).count(),
                    "active_uploads": transfers.entries.iter().filter(|t| t.status == "in_progress" && t.direction != 0).count(),
                },
                "searches": {
                    "total": searches.records.len(),
                },
            }).to_string();

            drop(transfers);
            drop(searches);

            Ok(routing::ok_response(diag))
        }

        // DATABASE MAINTENANCE ENDPOINTS
        ("GET", "/api/v0/database/stats") => Ok(routing::ok_response(
            database_stats_value(state).await.to_string(),
        )),
        ("POST", "/api/v0/database/cleanup") => Ok(routing::ok_response(
            database_cleanup_value(state, body).await.to_string(),
        )),
        ("POST", "/api/v0/database/vacuum") => Ok(routing::ok_response(
            database_vacuum_value(state).await.to_string(),
        )),
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn route_dispatch_group_4(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("GET", "/api/database/stats") => Ok(routing::ok_response(
            database_stats_value(state).await.to_string(),
        )),
        ("POST", "/api/database/cleanup") => Ok(routing::ok_response(
            database_cleanup_value(state, body).await.to_string(),
        )),
        ("POST", "/api/database/vacuum") => Ok(routing::ok_response(
            database_vacuum_value(state).await.to_string(),
        )),

        // COLLECTIONS ENDPOINTS
        ("GET", "/api/collections") => {
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                format!(
                    "[{}]",
                    collections
                        .records
                        .iter()
                        .filter(|record| {
                            !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
                        })
                        .map(CollectionRecord::native_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                collections.json_array(route.query, caller_id.as_deref())
            };
            drop(collections);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/shared") => {
            let shares = state.shares.read().await;
            let entries = shares
                .roots
                .iter()
                .map(|root| {
                    let mut value = controller_share_value(root);
                    value["name"] = serde_json::json!(root.label);
                    value
                })
                .collect::<Vec<_>>();
            drop(shares);
            Ok(routing::ok_response(
                serde_json::Value::Array(entries).to_string(),
            ))
        }
        ("POST", "/api/collections") => {
            let Some(name) = extract_json_string_field(body, "title")
                .or_else(|| extract_json_string_field(body, "name"))
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(routing::bad_request_response("title is required"));
            };
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            // Matches the oracle's real AuthenticatedWebUserId.Resolve:
            // the collection's real owner is the caller's own resolved
            // identity, never a hardcoded placeholder -- empty when no
            // per-caller identity is resolvable (single-operator mode).
            let owner_user_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            )
            .unwrap_or_default();
            let mut collections = state.collections.write().await;
            let previous = collections.clone();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let record = if compatibility_contract {
                let collection_type = extract_json_string_field(body, "type")
                    .filter(|value| value.trim() == "Playlist")
                    .map(|_| "Playlist".to_owned())
                    .unwrap_or_else(|| "ShareList".to_owned());
                collections.create_with_contract(
                    uuid::Uuid::new_v4().to_string(),
                    owner_user_id,
                    name,
                    description,
                    collection_type,
                )
            } else {
                collections.create(owner_user_id, name, description)
            };
            let Some(record) = record else {
                return Ok(routing::service_unavailable_response(
                    "collection capacity is full",
                ));
            };
            let mutated = collections.clone();
            let json = if compatibility_contract {
                record.native_json()
            } else {
                record.json()
            };
            drop(collections);
            if let Err(error) = persist_collection_checked(state, &record).await {
                rollback_collections_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/collections/")
                && !path.ends_with("/items")
                && path.matches('/').count() == 3 =>
        {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            if let Some(record) = collections.get(id).filter(|record| {
                !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                let json = if route.path.starts_with("/api/v0/") {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path)
            if path.starts_with("/api/collections/")
                && !path.contains("/items")
                && path.matches('/').count() == 3 =>
        {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let (name, description, collection_type) = if compatibility_contract {
                let request = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(request @ serde_json::Value::Object(_)) => request,
                    _ => return Ok(routing::bad_request_response("Request is required.")),
                };
                let name = request
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .map(str::to_owned);
                if name.as_ref().is_some_and(|name| name.is_empty()) {
                    return Ok(routing::bad_request_response("Title cannot be blank."));
                }
                let description = request
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .map(str::to_owned);
                let collection_type =
                    request
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .map(|value| {
                            if value.trim() == "Playlist" {
                                "Playlist".to_owned()
                            } else {
                                "ShareList".to_owned()
                            }
                        });
                (name, description, collection_type)
            } else {
                let Some(name) = extract_json_string_field(body, "title")
                    .or_else(|| extract_json_string_field(body, "name"))
                    .filter(|value| !value.trim().is_empty())
                else {
                    return Ok(routing::bad_request_response("title is required"));
                };
                (
                    Some(name),
                    Some(extract_json_string_field(body, "description").unwrap_or_default()),
                    None,
                )
            };
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut collections = state.collections.write().await;
            if collections.get(id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            let updated = if compatibility_contract {
                collections.update_contract(id, name, description, collection_type)
            } else {
                collections.update(
                    id,
                    name.unwrap_or_default(),
                    description.unwrap_or_default(),
                )
            };
            if let Some(record) = updated {
                let mutated = collections.clone();
                let json = if compatibility_contract {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/collections/")
                && !path.contains("/items")
                && path.matches('/').count() == 3 =>
        {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut grants = state.share_grants.write().await;
            let mut collections = state.collections.write().await;
            if collections.get(id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                drop(grants);
                return Ok(routing::not_found_response());
            }
            let previous_collections = collections.clone();
            let previous_grants = grants.clone();
            let deleted = collections.delete(id);
            if deleted {
                let revoked_grants = grants.delete_by_collection(id);
                if let Err(error) = persist_collection_delete(state, id).await {
                    *collections = previous_collections;
                    *grants = previous_grants;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(collections);
                drop(grants);
                let mut tokens = state.share_access_tokens.write().await;
                for grant in &revoked_grants {
                    tokens.revoke_grant(&grant.id);
                }
                drop(tokens);
                let mut tickets = state.stream_tickets.write().await;
                for grant in revoked_grants {
                    tickets.revoke_source(&format!("share:{}", grant.id));
                }
                drop(tickets);
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::ok_response("{}".to_string())
                })
            } else {
                drop(collections);
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("GET", path)
            if path.starts_with("/api/collections/")
                && path.ends_with("/items")
                && collection_items_id(path).is_some() =>
        {
            let id = collection_items_id(path).expect("guarded collection items path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            if let Some(record) = collections.get(id).filter(|record| {
                !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                let compatibility_contract = route.path.starts_with("/api/v0/");
                let items = record
                    .items
                    .iter()
                    .enumerate()
                    .map(|(ordinal, item)| {
                        if compatibility_contract {
                            item.native_json(&record.id, ordinal)
                        } else {
                            item.json()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", items);
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/collections/")
                && path.ends_with("/items")
                && collection_items_id(path).is_some() =>
        {
            let id = collection_items_id(path).expect("guarded collection items path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let content_id = extract_json_string_field(body, "contentId")
                .or_else(|| extract_json_string_field(body, "content_id"))
                .unwrap_or_default();
            if compatibility_contract && content_id.trim().is_empty() {
                return Ok(routing::bad_request_response("ContentId is required."));
            }
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "mediaKind")
                .or_else(|| extract_json_string_field(body, "kind"))
                .unwrap_or_else(|| "Audio".to_string());
            let file_name = extract_json_string_field(body, "fileName").unwrap_or_default();
            let album = extract_json_string_field(body, "album").unwrap_or_default();
            let content_hash = extract_json_string_field(body, "contentHash")
                .or_else(|| extract_json_string_field(body, "sha256"))
                .unwrap_or_default();

            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut collections = state.collections.write().await;
            if collections.get(id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            match collections.add_item_with_contract(
                id,
                compatibility_contract.then(|| uuid::Uuid::new_v4().to_string()),
                content_id,
                artist,
                title,
                kind,
                file_name,
                album,
                content_hash,
            ) {
                Ok(Some(item)) => {
                    let record = collections
                        .get(id)
                        .expect("item was added to an existing collection");
                    let mutated = collections.clone();
                    let json = if compatibility_contract {
                        item.native_json(id, record.items.len().saturating_sub(1))
                    } else {
                        item.json()
                    };
                    drop(collections);
                    if let Err(error) = persist_collection_checked(state, &record).await {
                        rollback_collections_if_unchanged(state, previous, &mutated).await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::created_response(json))
                }
                Ok(None) => {
                    drop(collections);
                    Ok(routing::not_found_response())
                }
                Err(()) => {
                    drop(collections);
                    Ok(routing::service_unavailable_response(
                        "collection item capacity is full",
                    ))
                }
            }
        }
        ("DELETE", path) if collection_item_action_ids(path).is_some() => {
            let (item_id, requested_collection_id) =
                collection_item_action_ids(path).expect("guarded collection item path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut collections = state.collections.write().await;
            let collection_id = collections.collection_id_for_item(item_id);
            if requested_collection_id
                .is_some_and(|expected| collection_id.as_deref() != Some(expected))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            if collection_id
                .as_deref()
                .and_then(|id| collections.get(id))
                .is_some_and(|record| {
                    collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
                })
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            if let Some(item) = collections.remove_item(item_id) {
                let record = collection_id
                    .as_deref()
                    .and_then(|id| collections.get(id))
                    .expect("removed item belonged to an existing collection");
                let mutated = collections.clone();
                let json = serde_json::json!({
                    "deleted": true,
                    "item": serde_json::from_str::<serde_json::Value>(&item.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": item_id })),
                })
                .to_string();
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if collection_item_action_ids(path).is_some() => {
            let (item_id, requested_collection_id) =
                collection_item_action_ids(path).expect("guarded collection item path");
            let artist = extract_json_string_field(body, "artist");
            let title = extract_json_string_field(body, "title");
            let kind = extract_json_string_field(body, "kind")
                .or_else(|| extract_json_string_field(body, "mediaKind"));

            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut collections = state.collections.write().await;
            let collection_id = collections.collection_id_for_item(item_id);
            if requested_collection_id
                .is_some_and(|expected| collection_id.as_deref() != Some(expected))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            if collection_id
                .as_deref()
                .and_then(|id| collections.get(id))
                .is_some_and(|record| {
                    collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
                })
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            if let Some(item) = collections.update_item(item_id, artist, title, kind) {
                let record = collection_id
                    .as_deref()
                    .and_then(|id| collections.get(id))
                    .expect("updated item belonged to an existing collection");
                let mutated = collections.clone();
                let json = item.json();
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }

        // WISHLIST ENDPOINTS
        ("GET", "/api/wishlist") => {
            let mut wishlist = state.wishlist.write().await;
            let json = if route.path.starts_with("/api/v0/") {
                wishlist.get_or_create();
                format!(
                    "[{}]",
                    wishlist
                        .records
                        .iter()
                        .flat_map(|record| record.items.iter())
                        .map(WishlistItem::native_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                wishlist.json_array()
            };
            drop(wishlist);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/wishlist") => {
            let search_text = extract_json_string_field(body, "searchText").unwrap_or_default();
            let artist =
                extract_json_string_field(body, "artist").unwrap_or_else(|| search_text.clone());
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind =
                extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            if artist.trim().is_empty() && title.trim().is_empty() {
                return Ok(routing::bad_request_response("SearchText is required"));
            }
            let filter = extract_json_string_field(body, "filter").unwrap_or_default();
            let enabled = extract_json_bool_field(body, "enabled").unwrap_or(true);
            let auto_download = extract_json_bool_field(body, "autoDownload").unwrap_or(false);
            let max_results = extract_json_u64_field(body, "maxResults").unwrap_or(100);
            if max_results == 0 || max_results > MAX_WISHLIST_RESULTS as u64 {
                return Ok(routing::bad_request_response(
                    "MaxResults must be between 1 and 10000",
                ));
            }
            let max_downloads = extract_json_optional_u64_field(body, "maxDownloads");
            if max_downloads
                .flatten()
                .is_some_and(|value| value == 0 || value > MAX_WISHLIST_DOWNLOADS)
            {
                return Ok(routing::bad_request_response(
                    "MaxDownloads must be null or between 1 and 1000000",
                ));
            }

            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let id = compatibility_contract.then(|| uuid::Uuid::new_v4().to_string());
            match wishlist.add_item_with_contract(
                id,
                artist,
                title,
                kind,
                filter,
                enabled,
                auto_download,
                usize::try_from(max_results).unwrap_or(MAX_WISHLIST_RESULTS),
                max_downloads.flatten(),
            ) {
                Ok(item) => {
                    let mutated = wishlist.clone();
                    let json = if compatibility_contract {
                        item.native_json()
                    } else {
                        item.json()
                    };
                    drop(wishlist);
                    if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                        rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                        return Ok(wishlist_storage_error_response(
                            route.path.starts_with("/api/v0/"),
                            &error,
                        ));
                    }
                    Ok(routing::created_response(json))
                }
                Err(()) => {
                    drop(wishlist);
                    Ok(routing::service_unavailable_response(
                        "wishlist item capacity is full",
                    ))
                }
            }
        }
        ("GET", path) if wishlist_item_action_id(path, "/searches").is_some() => {
            let item_id =
                wishlist_item_action_id(path, "/searches").expect("guarded wishlist history path");
            if state.wishlist.read().await.get_item(item_id).is_none() {
                return Ok(routing::not_found_response());
            }
            let searches = state.searches.read().await;
            let json = searches.wishlist_history_json(item_id, route.query);
            drop(searches);
            Ok(routing::ok_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/wishlist/") && !path.contains("/ignored-results") =>
        {
            let Some(item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let wishlist = state.wishlist.read().await;
            let Some(item) = wishlist.get_item(item_id) else {
                return Ok(routing::not_found_response());
            };
            Ok(routing::ok_response(
                if route.path.starts_with("/api/v0/") {
                    item.native_json()
                } else {
                    item.json()
                },
            ))
        }
        ("POST", "/api/wishlist/mark-all-viewed") => {
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let items = wishlist.mark_all_viewed();
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_items_checked(state, &items).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("POST", path) if wishlist_item_action_id(path, "/mark-viewed").is_some() => {
            let item_id = wishlist_item_action_id(path, "/mark-viewed")
                .expect("guarded wishlist viewed path");
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let Some(item) = wishlist.mark_viewed(item_id) else {
                return Ok(routing::not_found_response());
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("GET", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let item_id = wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let wishlist = state.wishlist.read().await;
            let Some(rules) = wishlist.list_ignored_results(item_id) else {
                return Ok(routing::not_found_response());
            };
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let json = serde_json::Value::Array(
                rules
                    .iter()
                    .map(|rule| {
                        if compatibility_contract {
                            rule.native_json()
                        } else {
                            rule.json()
                        }
                    })
                    .collect(),
            )
            .to_string();
            Ok(routing::ok_response(json))
        }
        ("POST", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let item_id = wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if username.trim().is_empty() || normalize_wishlist_directory(&directory).is_empty() {
                return Ok(routing::bad_request_response(
                    if route.path.starts_with("/api/v0/") {
                        "Username and Directory are required"
                    } else {
                        "username and directory are required"
                    },
                ));
            }

            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let (rule, created) = match wishlist.ignore_result(item_id, &username, &directory) {
                Ok(result) => result,
                Err("not_found") => return Ok(routing::not_found_response()),
                Err("capacity") => {
                    return Ok(routing::service_unavailable_response(
                        "wishlist ignored-result capacity is full",
                    ));
                }
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        if route.path.starts_with("/api/v0/") {
                            "Username and Directory are required"
                        } else {
                            "username and directory are required"
                        },
                    ));
                }
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if created {
                let (previous_searches, mutated_searches, changed_searches) = {
                    let mut searches = state.searches.write().await;
                    let previous = searches.clone();
                    let changed = searches.suppress_ignored_result(&rule);
                    let mutated = searches.clone();
                    (previous, mutated, changed)
                };
                if let Err(error) = persist_wishlist_ignored_result_and_searches_checked(
                    state,
                    &rule,
                    &changed_searches,
                )
                .await
                {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    let mut searches = state.searches.write().await;
                    if *searches == mutated_searches {
                        *searches = previous_searches;
                    }
                    drop(searches);
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let json = if compatibility_contract {
                rule.native_json()
            } else {
                rule.json()
            }
            .to_string();
            if created || compatibility_contract {
                Ok(routing::created_response(json))
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if wishlist_ignored_result_ids(path).is_some() => {
            let (item_id, rule_id) =
                wishlist_ignored_result_ids(path).expect("guarded ignored rule path");
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            if !wishlist.delete_ignored_result(item_id, rule_id) {
                return Ok(routing::not_found_response());
            }
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) =
                persist_wishlist_ignored_result_delete_checked(state, item_id, rule_id).await
            {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("DELETE", path) if path.starts_with("/api/wishlist/") => {
            let Some(item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            if let Some(record) = wishlist.remove_item(item_id) {
                let mutated = wishlist.clone();
                let json = serde_json::json!({
                    "deleted": true,
                    "item_id": item_id,
                    "remaining": record.items.len(),
                    "updated_at": record.updated_at,
                })
                .to_string();
                drop(wishlist);
                if let Err(error) = persist_wishlist_item_delete_checked(state, item_id).await {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::ok_response(json)
                })
            } else {
                drop(wishlist);
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                })
            }
        }

        // CONTACTS ENDPOINTS
        ("GET", "/api/contacts/nearby") => {
            let contacts = state.contacts.read().await;
            let json = contacts.nearby_json(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/contacts") => {
            let contacts = state.contacts.read().await;
            let json = contacts.json_array(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/contacts") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, created) = match contacts.create(username) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            let json = record.json();
            drop(contacts);
            if created {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::created_response(json))
            } else {
                Ok(routing::conflict_response("contact already exists"))
            }
        }
        ("POST", "/api/contacts/from-discovery") => {
            if route.path.starts_with("/api/v0/") {
                let peer_id = extract_json_string_field(body, "peerId")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let nickname = extract_json_string_field(body, "nickname")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if peer_id.trim().is_empty() {
                    return Ok(routing::bad_request_response("PeerId is required."));
                }
                if nickname.trim().is_empty() {
                    return Ok(routing::bad_request_response("Nickname is required."));
                }
                // The profile service always has the local signed profile
                // available.  Unknown peers still require a cached/fetched
                // profile, which this in-process compatibility layer does
                // not synthesize from a username alone.
                if !peer_id.eq_ignore_ascii_case(&local_profile_peer_id(state)) {
                    return Ok(HttpResponse {
                        status: "404 Not Found",
                        content_type: "application/json",
                        body: serde_json::json!("Profile not found.").to_string(),
                    });
                }
                let mut contacts = state.contacts.write().await;
                let previous = contacts.clone();
                let (record, added) = match contacts
                    .create_with_contract(Some(uuid::Uuid::new_v4().to_string()), nickname)
                {
                    Ok(result) => result,
                    Err(()) => {
                        return Ok(routing::service_unavailable_response(
                            "contact capacity is full",
                        ));
                    }
                };
                let mutated = contacts.clone();
                let json = record.native_json(&peer_id);
                drop(contacts);
                if added {
                    if let Err(error) = persist_contact_checked(state, &record).await {
                        let mut contacts = state.contacts.write().await;
                        if *contacts == mutated {
                            *contacts = previous;
                        }
                        drop(contacts);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
                return Ok(if added {
                    routing::created_response(json)
                } else {
                    routing::ok_response(json)
                });
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, added) = match contacts.create(username.clone()) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            drop(contacts);
            if added {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let json = format!(
                "{{\"username\":\"{}\",\"discovered\":true,\"added\":{added}}}",
                json_escape(&username),
            );
            Ok(if added {
                routing::created_response(json)
            } else {
                routing::ok_response(json)
            })
        }
        ("POST", "/api/contacts/from-invite") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let id = route
                .path
                .starts_with("/api/v0/")
                .then(|| uuid::Uuid::new_v4().to_string());
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, added) = match contacts.create_with_contract(id, username.clone()) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            drop(contacts);
            if added {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let json = format!(
                "{{\"username\":\"{}\",\"invited\":true,\"accepted\":true,\"added\":{added}}}",
                json_escape(&username),
            );
            Ok(if added {
                routing::created_response(json)
            } else {
                routing::ok_response(json)
            })
        }
        ("GET", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let contacts = state.contacts.read().await;
            if let Some(record) = contacts.get(id) {
                let json = record.json();
                drop(contacts);
                Ok(routing::ok_response(json))
            } else {
                drop(contacts);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let username = extract_json_string_field(body, "username");
            let online = extract_json_bool_field(body, "online");
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            match contacts.update(id, username, online) {
                Ok(record) => {
                    let mutated = contacts.clone();
                    let json = record.json();
                    drop(contacts);
                    if let Err(error) = persist_contact_checked(state, &record).await {
                        let mut contacts = state.contacts.write().await;
                        if *contacts == mutated {
                            *contacts = previous;
                        }
                        drop(contacts);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::ok_response(json))
                }
                Err(ContactUpdateError::DuplicateUsername) => {
                    drop(contacts);
                    Ok(routing::conflict_response(
                        "contact username already exists",
                    ))
                }
                Err(ContactUpdateError::NotFound) => {
                    drop(contacts);
                    Ok(routing::not_found_response())
                }
            }
        }
        ("DELETE", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let deleted = contacts.delete(id);
            let mutated = contacts.clone();
            drop(contacts);
            if deleted {
                if let Err(error) = persist_contact_delete_checked(state, id).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHAREGROUPS ENDPOINTS
        ("GET", "/api/sharegroups") => {
            let sharegroups = state.sharegroups.read().await;
            let json = sharegroups.json_array(route.query);
            drop(sharegroups);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/sharegroups") => {
            let requested_name = extract_json_string_field(body, "name");
            let name = if route.path.starts_with("/api/v0/") {
                let Some(name) = requested_name else {
                    return Ok(routing::bad_request_response("Name is required."));
                };
                if name.trim().is_empty() {
                    return Ok(routing::bad_request_response("Name is required."));
                }
                name
            } else {
                requested_name.unwrap_or_else(|| "Untitled".to_string())
            };
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            let Some(mut record) = sharegroups.create(name, description) else {
                return Ok(routing::service_unavailable_response(
                    "share group capacity is full",
                ));
            };
            let json = if route.path.starts_with("/api/v0/") {
                let old_id = record.id.clone();
                record.id = uuid::Uuid::new_v4().to_string();
                if let Some(stored) = sharegroups
                    .records
                    .iter_mut()
                    .find(|group| group.id == old_id)
                {
                    stored.id.clone_from(&record.id);
                }
                serde_json::json!({
                    "id": record.id,
                    "name": record.name,
                    "ownerUserId": "Anonymous",
                    "createdAt": unix_seconds_rfc3339(record.created_at),
                    "updatedAt": unix_seconds_rfc3339(record.updated_at),
                })
                .to_string()
            } else {
                record.json()
            };
            if let Err(error) = persist_share_group(state, &record).await {
                *sharegroups = previous;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(sharegroups);
            Ok(routing::created_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let json = record.json();
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let name =
                extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            if let Some(record) = sharegroups.update(id, name, description) {
                let json = record.json();
                if let Err(error) = persist_share_group(state, &record).await {
                    *sharegroups = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            let deleted = sharegroups.delete(id);
            if deleted {
                if let Err(error) = persist_share_group_delete(state, id).await {
                    *sharegroups = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(sharegroups);
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("GET", path)
            if path.starts_with("/api/sharegroups/")
                && path.ends_with("/members")
                && share_group_members_id(path).is_some() =>
        {
            let id = share_group_members_id(path).expect("guarded share-group members path");
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let members = record
                    .members
                    .iter()
                    .map(|m| m.json())
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", members);
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/sharegroups/")
                && path.ends_with("/members")
                && share_group_members_id(path).is_some() =>
        {
            let id = share_group_members_id(path).expect("guarded share-group members path");
            if route.path.starts_with("/api/v0/")
                && state.sharegroups.read().await.get(id).is_none()
            {
                return Ok(routing::not_found_response());
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            match sharegroups.add_member(id, username.clone()) {
                Ok(Some((record, added))) => {
                    let member = record
                        .members
                        .iter()
                        .find(|member| member.username.eq_ignore_ascii_case(&username))
                        .cloned();
                    let json = member
                        .as_ref()
                        .map(ShareGroupMember::json)
                        .unwrap_or_else(|| {
                            format!(
                                "{{\"username\":\"{}\",\"added_at\":{}}}",
                                json_escape(&username),
                                unix_timestamp()
                            )
                        });
                    if added {
                        if let Err(error) = persist_share_group(state, &record).await {
                            *sharegroups = previous;
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                    drop(sharegroups);
                    Ok(if added {
                        routing::created_response(json)
                    } else {
                        routing::ok_response(json)
                    })
                }
                Ok(None) => {
                    drop(sharegroups);
                    Ok(routing::not_found_response())
                }
                Err(()) => {
                    drop(sharegroups);
                    Ok(routing::service_unavailable_response(
                        "share group member capacity is full",
                    ))
                }
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/sharegroups/")
                && path.contains("/members/")
                && share_group_member_path(path).is_some() =>
        {
            let (id, username) =
                share_group_member_path(path).expect("guarded share-group member path");
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            if let Some(record) = sharegroups.remove_member(id, &username) {
                if let Err(error) = persist_share_group(state, &record).await {
                    *sharegroups = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(sharegroups);
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }

        // USER NOTES ENDPOINTS
        ("GET", "/api/users/notes")
            if route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/") =>
        {
            let notes = state.user_notes.read().await;
            let value = notes
                .records
                .iter()
                .filter_map(|record| {
                    serde_json::from_str::<serde_json::Value>(&record.native_json()).ok()
                })
                .collect::<Vec<_>>();
            Ok(routing::ok_response(serde_json::json!(value).to_string()))
        }
        ("GET", "/api/users/notes") => {
            let notes = state.user_notes.read().await;
            let json = notes.json(None);
            drop(notes);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/users/notes") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("Username is required."));
            }
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            let versioned =
                route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let record = if versioned {
                notes.set_versioned(
                    username,
                    note,
                    extract_json_string_field(body, "color").unwrap_or_default(),
                    extract_json_string_field(body, "icon").unwrap_or_default(),
                    extract_json_bool_field(body, "isHighPriority").unwrap_or(false),
                )
            } else {
                notes.create(username, note)
            };
            let Some(record) = record else {
                return Ok(routing::service_unavailable_response(
                    "user note capacity is full",
                ));
            };
            let mutated = notes.clone();
            let json = if versioned {
                record.native_json()
            } else {
                record.json()
            };
            drop(notes);
            if let Err(error) = persist_user_note_checked(state, &record).await {
                let mut notes = state.user_notes.write().await;
                if *notes == mutated {
                    *notes = previous;
                }
                drop(notes);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(if versioned {
                routing::ok_response(json)
            } else {
                routing::created_response(json)
            })
        }
        ("GET", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let notes = state.user_notes.read().await;
            let versioned =
                route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let record = if versioned {
                notes.get_by_username(id)
            } else {
                notes.get(id)
            };
            if let Some(record) = record {
                let json = if versioned {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(notes);
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            if let Some(record) = notes.update(id, note) {
                let mutated = notes.clone();
                let json = record.json();
                drop(notes);
                if let Err(error) = persist_user_note_checked(state, &record).await {
                    let mut notes = state.user_notes.write().await;
                    if *notes == mutated {
                        *notes = previous;
                    }
                    drop(notes);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            let versioned =
                route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let removed_id = if versioned {
                notes.delete_by_username(id).map(|record| record.id)
            } else {
                notes.delete(id).then(|| id.to_owned())
            };
            let mutated = notes.clone();
            drop(notes);
            if let Some(removed_id) = removed_id {
                if let Err(error) = persist_user_note_delete_checked(state, &removed_id).await {
                    let mut notes = state.user_notes.write().await;
                    if *notes == mutated {
                        *notes = previous;
                    }
                    drop(notes);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(if versioned {
                    routing::no_content_response()
                } else {
                    routing::ok_response("{}".to_string())
                })
            } else if versioned {
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Liked)
        ("GET", "/api/soulseek/interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_liked();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/interests") => {
            let versioned = route.path.starts_with("/api/v0/");
            let name = extract_json_string_field(body, "item")
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|name| name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let Some((record, created)) = interests.add_liked(name) else {
                return Ok(routing::service_unavailable_response(
                    "liked interest capacity is full",
                ));
            };
            let mutated = interests.clone();
            let json = record.json();
            drop(interests);
            if created {
                if let Err(error) = persist_interest_checked(state, &record).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::AddThingILike {
                            item: record.name.clone(),
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::created_response(json))
                }
            } else if versioned {
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::AddThingILike {
                        item: record.name.clone(),
                    },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/interests/") => {
            let versioned = route.path.starts_with("/api/v0/");
            let Some(raw_item) = path_segment_after(path, "/api/soulseek/interests/") else {
                return Ok(routing::not_found_response());
            };
            let item = decoded_path_segment(raw_item).trim().to_owned();
            if item.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let id = interests
                .liked
                .iter()
                .find(|record| {
                    record.id.eq_ignore_ascii_case(&item) || record.name.eq_ignore_ascii_case(&item)
                })
                .map(|record| record.id.clone());
            let deleted = id.as_deref().is_some_and(|id| interests.remove_liked(id));
            let mutated = interests.clone();
            drop(interests);
            if deleted {
                let existing_id = id.as_deref().expect("deleted liked interest id");
                let published_item = id
                    .as_deref()
                    .and_then(|id| {
                        previous
                            .liked
                            .iter()
                            .find(|record| record.id == id)
                            .map(|record| record.name.clone())
                    })
                    .unwrap_or_else(|| item.clone());
                if let Err(error) = persist_interest_delete_checked(state, existing_id).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::RemoveThingILike {
                            item: published_item,
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::ok_response("{}".to_string()))
                }
            } else if versioned {
                if let Err(error) =
                    send_active_interest_command(state, ServerMessage::RemoveThingILike { item })
                        .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Hated)
        ("GET", "/api/soulseek/hated-interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_hated();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/hated-interests") => {
            let versioned = route.path.starts_with("/api/v0/");
            let name = extract_json_string_field(body, "item")
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|name| name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let Some((record, created)) = interests.add_hated(name) else {
                return Ok(routing::service_unavailable_response(
                    "hated interest capacity is full",
                ));
            };
            let mutated = interests.clone();
            let json = record.json();
            drop(interests);
            if created {
                if let Err(error) = persist_interest_checked(state, &record).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::AddThingIHate {
                            item: record.name.clone(),
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::created_response(json))
                }
            } else if versioned {
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::AddThingIHate {
                        item: record.name.clone(),
                    },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/hated-interests/") => {
            let versioned = route.path.starts_with("/api/v0/");
            let Some(raw_item) = path_segment_after(path, "/api/soulseek/hated-interests/") else {
                return Ok(routing::not_found_response());
            };
            let item = decoded_path_segment(raw_item).trim().to_owned();
            if item.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let id = interests
                .hated
                .iter()
                .find(|record| {
                    record.id.eq_ignore_ascii_case(&item) || record.name.eq_ignore_ascii_case(&item)
                })
                .map(|record| record.id.clone());
            let deleted = id.as_deref().is_some_and(|id| interests.remove_hated(id));
            let mutated = interests.clone();
            drop(interests);
            if deleted {
                let existing_id = id.as_deref().expect("deleted hated interest id");
                let published_item = id
                    .as_deref()
                    .and_then(|id| {
                        previous
                            .hated
                            .iter()
                            .find(|record| record.id == id)
                            .map(|record| record.name.clone())
                    })
                    .unwrap_or_else(|| item.clone());
                if let Err(error) = persist_interest_delete_checked(state, existing_id).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::RemoveThingIHate {
                            item: published_item,
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::ok_response("{}".to_string()))
                }
            } else if versioned {
                if let Err(error) =
                    send_active_interest_command(state, ServerMessage::RemoveThingIHate { item })
                        .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHARE GRANTS ENDPOINTS
        ("GET", "/api/share-grants") => {
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let grants = state.share_grants.read().await;
            let records = grants.records.clone();
            drop(grants);
            let mut visible = Vec::with_capacity(records.len());
            for record in &records {
                if !share_grant_collection_forbids(
                    state,
                    &record.collection_id,
                    caller_id.as_deref(),
                )
                .await
                {
                    visible.push(record.json());
                }
            }
            Ok(routing::ok_response(format!("[{}]", visible.join(","))))
        }
        ("POST", "/api/share-grants") => {
            let collection_id = extract_json_string_field(body, "collection_id")
                .or_else(|| extract_json_string_field(body, "collectionId"))
                .unwrap_or_default();
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            if !collection_id.is_empty() {
                let collections = state.collections.read().await;
                let collection_exists = collections.get(&collection_id).is_some();
                drop(collections);
                if !collection_exists {
                    return Ok(routing::not_found_response());
                }
                // Matches the oracle's real Create: a grant may only be
                // created against a collection the caller actually owns.
                if share_grant_collection_forbids(state, &collection_id, caller_id.as_deref()).await
                {
                    return Ok(routing::not_found_response());
                }
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let Some(username) = normalize_share_grant_username(&username) else {
                return Ok(routing::conflict_response(
                    "collection_id and username are required",
                ));
            };
            if collection_id.is_empty() {
                return Ok(routing::conflict_response(
                    "collection_id and username are required",
                ));
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let id = compatibility_contract.then(|| uuid::Uuid::new_v4().to_string());
            let mut grants = state.share_grants.write().await;
            let previous = grants.clone();
            let Some((record, created)) = grants.create_with_contract(id, collection_id, username)
            else {
                return Ok(routing::service_unavailable_response(
                    "share grant capacity is full",
                ));
            };
            let json = record.json();
            if created {
                if let Err(error) = persist_share_grant(state, &record).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                Ok(routing::created_response(json))
            } else {
                drop(grants);
                Ok(routing::ok_response(json))
            }
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/")
                && path.ends_with("/manifest")
                && share_grant_manifest_id(path).is_some() =>
        {
            let grant_id =
                share_grant_manifest_id(path).expect("guarded share-grant manifest path");
            if query_parameter(route.query, "token").is_some() {
                return Ok(routing::bad_request_response(
                    "share tokens must be sent in X-Share-Token",
                ));
            }
            let api_authorized = !state.config.auth_required
                || is_authorized(&state.config, authorization, headers.cookie.as_deref());
            let share_token = request_share_token(authorization, &headers);
            if let Some(token) = share_token {
                let mut tokens = state.share_access_tokens.write().await;
                let token_record = tokens.validate(&token);
                drop(tokens);
                if token_record.as_ref().map(|record| record.grant_id.as_str()) != Some(grant_id) {
                    return Ok(routing::unauthorized_response());
                }
            } else if !api_authorized {
                return Ok(routing::unauthorized_response());
            }

            let grants = state.share_grants.read().await;
            let Some(grant) = grants.get(grant_id) else {
                drop(grants);
                return Ok(routing::not_found_response());
            };
            drop(grants);
            let collections = state.collections.read().await;
            let Some(collection) = collections.get(&grant.collection_id) else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            let items = collection
                .items
                .iter()
                .map(|item| {
                    serde_json::from_str::<serde_json::Value>(&item.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": item.id }))
                })
                .collect::<Vec<_>>();
            let item_count = items.len();
            let collection_value = serde_json::from_str::<serde_json::Value>(&collection.json())
                .unwrap_or_else(|_| serde_json::json!({ "id": collection.id }));
            drop(collections);
            Ok(routing::ok_response(
                serde_json::json!({
                    "share": serde_json::from_str::<serde_json::Value>(&grant.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": grant.id })),
                    "collection": collection_value,
                    "items": items,
                    "itemCount": item_count,
                    "permissions": grant.permissions,
                })
                .to_string(),
            ))
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn route_dispatch_group_5(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("GET", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let grants = state.share_grants.read().await;
            let record = grants.get(id);
            drop(grants);
            let Some(record) = record else {
                return Ok(routing::not_found_response());
            };
            if share_grant_collection_forbids(state, &record.collection_id, caller_id.as_deref())
                .await
            {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(record.json()))
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/by-collection/")
                && share_grant_collection_id(path).is_some() =>
        {
            let collection_id =
                share_grant_collection_id(path).expect("guarded share-grant collection path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            // SharesController.GetByCollection first resolves the collection
            // and returns NotFound when the collection no longer exists.
            let collection_exists = state.collections.read().await.get(collection_id).is_some();
            if !collection_exists {
                return Ok(routing::not_found_response());
            }
            // Matches the oracle's real GetByCollection: this is the
            // "outgoing shares" owner-perspective view, so it 404s unless
            // the caller actually owns this collection.
            if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await {
                return Ok(routing::not_found_response());
            }
            let grants = state.share_grants.read().await;
            let records = grants.get_by_collection(collection_id);
            let json = records
                .iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(",");
            let response = format!("[{}]", json);
            drop(grants);
            Ok(routing::ok_response(response))
        }
        ("PUT", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let permissions = extract_json_string_field(body, "permissions")
                .unwrap_or_else(|| "read".to_string());
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut grants = state.share_grants.write().await;
            let owning_collection_id = grants.get(id).map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
                    drop(grants);
                    return Ok(routing::not_found_response());
                }
            }
            let previous = grants.clone();
            if let Some(record) = grants.update(id, permissions) {
                let json = record.json();
                if let Err(error) = persist_share_grant(state, &record).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                Ok(routing::ok_response(json))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut grants = state.share_grants.write().await;
            let owning_collection_id = grants.get(id).map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
                    drop(grants);
                    return Ok(routing::not_found_response());
                }
            }
            let previous = grants.clone();
            let deleted = grants.delete(id);
            if deleted {
                if let Err(error) = persist_share_grant_delete_checked(state, id).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                state.share_access_tokens.write().await.revoke_grant(id);
                state
                    .stream_tickets
                    .write()
                    .await
                    .revoke_source(&format!("share:{id}"));
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }

        // LIBRARY ITEMS ENDPOINTS
        ("GET", "/api/library/items") => {
            if state.config.controller_profile == ControllerProfile::Native {
                return Ok(routing::ok_response(
                    native_library_items_search_json(state, route.query).await,
                ));
            }
            let library = state.library.read().await;
            let json = library.json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/items/browser") => {
            let library = state.library.read().await;
            let path = query_parameter(route.query, "path").unwrap_or_default();
            let limit = query_parameter(route.query, "limit")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(100)
                .clamp(1, 1_000);
            let offset = query_parameter(route.query, "offset")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let total_files = library.records.len();
            let files = library
                .records
                .iter()
                .skip(offset)
                .take(limit)
                .map(|record| {
                    serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_default()
                })
                .collect::<Vec<_>>();
            let returned_files = files.len();
            let json = serde_json::json!({
                "path": path,
                "breadcrumbs": [],
                "directories": [],
                "files": files,
                "totalFiles": total_files,
                "totalDirectories": 0,
                "offset": offset,
                "limit": limit,
                "hasMore": offset.saturating_add(returned_files) < total_files,
                "duplicatesRemoved": 0,
            })
            .to_string();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/library/items") => {
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind =
                extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            let mut library = state.library.write().await;
            let previous = library.clone();
            let Some(record) = library.create(artist, title, kind) else {
                return Ok(routing::service_unavailable_response(
                    "library item capacity is full",
                ));
            };
            let mutated = library.clone();
            let json = record.json();
            drop(library);
            if let Err(error) = persist_library_item_checked(state, &record).await {
                rollback_library_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let library = state.library.read().await;
            if let Some(record) = library.get(id) {
                let json = record.json();
                drop(library);
                Ok(routing::ok_response(json))
            } else {
                drop(library);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let mut library = state.library.write().await;
            let previous = library.clone();
            let deleted = library.delete(id);
            let mutated = library.clone();
            drop(library);
            if deleted {
                if let Err(error) = persist_library_item_delete_checked(state, id).await {
                    rollback_library_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // DESTINATIONS ENDPOINTS
        ("GET", "/api/destinations") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_list()
                } else {
                    configured.list()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_list()
            } else {
                destinations.list()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/destinations/default") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_default()
                } else {
                    configured.default()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_default()
            } else {
                destinations.default()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/config/download-filter") => {
            let exclusions = effective_download_exclusions(state).await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "exclude": exclusions,
                    "maxTerms": 100,
                    "maxTermLength": 256,
                })
                .to_string(),
            ))
        }
        ("PUT", "/api/config/download-filter") => Ok(update_download_filter(state, body).await),

        // BROWSE ENDPOINTS
        ("GET", path)
            if path.starts_with("/api/users/")
                && path.ends_with("/browse/status")
                && user_route_username(path, "/browse/status").is_some() =>
        {
            let username = user_route_username(path, "/browse/status")
                .expect("guarded user browse status path");
            if state.runtime.read().await.relay_agent_enabled {
                return Ok(controller_forbidden_response());
            }
            let browse = state.browse.read().await;
            let tracked = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(BrowseRecord::controller_status_json);
            drop(browse);
            match tracked {
                Some(body) => Ok(routing::ok_response(body)),
                None => Ok(routing::not_found_response()),
            }
        }
        // ADDITIONAL MISSING USER ENDPOINTS (Phase 5)
        ("GET", "/api/profile/me") => {
            if route.path.starts_with("/api/v0/") {
                let session = state.session.read().await;
                let display_name = session
                    .username
                    .clone()
                    .or_else(|| state.config.username.clone())
                    .unwrap_or_else(|| "Unknown".to_owned())
                    .trim()
                    .to_owned();
                drop(session);
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                let peer_id = local_profile_peer_id(state);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "peerId": peer_id,
                        "publicKey": STANDARD.encode(descriptor.public_key),
                        "displayName": display_name,
                        "avatar": null,
                        "capabilities": 0,
                        "endpoints": [],
                        "createdAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "expiresAt": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
                        "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                    })
                    .to_string(),
                ));
            }
            let session = state.session.read().await;
            let username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            let privileges_seconds = session.privileges_seconds.unwrap_or(0);
            let connected = session.state == "connected";
            drop(session);
            let users = state.users.read().await;
            let watched = users
                .records
                .iter()
                .any(|user| user.username.eq_ignore_ascii_case(&username) && user.watched);
            drop(users);
            let descriptor = local_capability_descriptor(state).await.ok();
            let json = serde_json::json!({
                "peerId": descriptor.as_ref().map(|value| value.peer_id.as_str()).unwrap_or(""),
                "publicKey": descriptor.as_ref().map(|value| STANDARD.encode(value.public_key)).unwrap_or_default(),
                "displayName": username.clone(),
                "capabilities": descriptor.as_ref().map(|value| value.features.clone()).unwrap_or_default(),
                "endpoints": descriptor.as_ref().map(|value| value.endpoints.clone()).unwrap_or_default(),
                "createdAt": descriptor.as_ref().map(|value| value.issued_at_unix).unwrap_or(0),
                "expiresAt": descriptor.as_ref().map(|value| value.expires_at_unix).unwrap_or(0),
                "signature": descriptor.as_ref().and_then(|value| value.signature).map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                "username": username,
                "description": "",
                "picture": "",
                "user_type": if privileges_seconds > 0 { "privileged" } else { "normal" },
                "privilegesSeconds": privileges_seconds,
                "connected": connected,
                "watched": watched,
            }).to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if route.path.starts_with("/api/v0/") && path.starts_with("/api/profile/") =>
        {
            let Some(raw_peer_id) = path.strip_prefix("/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            if raw_peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            if raw_peer_id.contains('/') {
                return Ok(routing::not_found_response());
            }
            let peer_id = decoded_path_segment(raw_peer_id).trim().to_owned();
            if peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            let local_peer_id = local_profile_peer_id(state);
            if !peer_id.eq_ignore_ascii_case(&local_peer_id) {
                return Ok(routing::not_found_response());
            }
            let session = state.session.read().await;
            let display_name = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "Unknown".to_owned())
                .trim()
                .to_owned();
            drop(session);
            return Ok(routing::ok_response(
                serde_json::json!({
                    "peerId": local_peer_id,
                    "displayName": display_name,
                    "avatar": null,
                    "capabilities": 0,
                    "endpoints": [],
                })
                .to_string(),
            ));
        }

        ("GET", path) if path.starts_with("/api/profile/") => {
            let Some(username) = path_segment_after(path, "/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let users = state.users.read().await;
            let user = users
                .records
                .iter()
                .find(|user| user.username.eq_ignore_ascii_case(&username))
                .cloned();
            drop(users);
            let json = serde_json::json!({
                "username": username,
                "description": "",
                "picture": "",
                "user_type": "normal",
                "watched": user.as_ref().is_some_and(|user| user.watched),
                "status": user.as_ref().and_then(|user| user.status.clone()).unwrap_or_else(|| "Unknown".to_owned()),
                "averageSpeed": user.as_ref().and_then(|user| user.average_speed).unwrap_or(0),
                "uploadCount": user.as_ref().and_then(|user| user.upload_count).unwrap_or(0),
                "fileCount": user.as_ref().and_then(|user| user.file_count).unwrap_or(0),
                "directoryCount": user.as_ref().and_then(|user| user.directory_count).unwrap_or(0),
            }).to_string();
            Ok(routing::ok_response(json))
        }

        // CONVERSATIONS ENDPOINT
        ("GET", "/api/conversations") => {
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.controller_conversations_json(route.query);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/conversations/activity/unacknowledged") => {
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.has_unacknowledged_messages().to_string();
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if conversation_messages_path(path).is_some() => {
            let Some(username) = conversation_messages_path(path) else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let unacknowledged_only = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "unAcknowledgedOnly")
                .map(|(_, value)| value)
            {
                None => false,
                Some(value) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "unAcknowledgedOnly must be a boolean",
                        ))
                    }
                },
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_messages_json(&username, unacknowledged_only);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let include_messages = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includeMessages")
            {
                None => true,
                Some((_, value)) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "includeMessages must be a boolean",
                        ))
                    }
                },
            };
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_conversation_json(&username, include_messages, since);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("POST", "/api/conversations/batch") => {
            let usernames = match extract_json_string_array_field(body, "usernames")
                .or_else(|| extract_json_string_array_field(body, "recipients"))
            {
                Some(usernames) => usernames,
                None => {
                    return Ok(routing::bad_request_response(
                        "usernames/recipients array is required",
                    ))
                }
            };
            let message_body = match extract_json_string_field(body, "body")
                .or_else(|| extract_json_string_field(body, "message"))
            {
                Some(body) => body,
                None => return Ok(routing::bad_request_response("body/message is required")),
            };
            let command = match private_message_users_command(usernames, message_body.clone()) {
                Ok(ServerMessage::MessageUsers { usernames, .. }) => usernames,
                Ok(_) => return Ok(routing::bad_request_response("invalid message command")),
                Err(error) => return Ok(routing::bad_request_response(&error.to_string())),
            };

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let records: Vec<_> = command
                .iter()
                .map(|username| messages.add(username.clone(), "outbound", message_body.clone()))
                .collect();
            let mutated = messages.clone();
            drop(messages);

            if let Err(error) = persist_message_records_checked(state, &records).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::MessageUsers {
                usernames: command.clone(),
                body: message_body,
            });

            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                Ok(HttpResponse {
                    status: "201 Created",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::created_response(
                    serde_json::json!({
                        "conversations": records.iter().map(MessageRecord::json).collect::<Vec<_>>(),
                        "usernames": command,
                        "count": records.len(),
                    })
                    .to_string(),
                ))
            }
        }
        ("POST", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if message_body.trim().is_empty() {
                return Ok(routing::bad_request_response("message is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "outbound", message_body.clone());
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::MessageUser {
                username,
                body: message_body,
            });
            Ok(
                if matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                ) && route.path.starts_with("/api/v0/")
                {
                    HttpResponse {
                        status: "201 Created",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response((record.id > 0).to_string())
                },
            )
        }

        // JOBS ENDPOINT
        ("GET", "/api/jobs") => {
            let searches = state.searches.read().await;
            let transfers = state.transfers.read().await;
            let mut jobs = searches
                .records
                .iter()
                .map(|record| {
                    serde_json::json!({
                        "id": record.id,
                        "kind": "search",
                        "type": "search",
                        "status": record.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": if record.status == "completed" { 1 } else { 0 },
                            "releases_failed": 0,
                        },
                        "progress_percent": if record.status == "completed" { 100 } else { 50 },
                        "query": record.query,
                        "created_at": record.created_at,
                        "updated_at": record.updated_at,
                    })
                })
                .collect::<Vec<_>>();
            jobs.extend(transfers.entries.iter().map(|entry| {
                let size = entry.size.unwrap_or(0);
                let progress = entry
                    .bytes_transferred
                    .saturating_mul(100)
                    .checked_div(size)
                    .unwrap_or(0)
                    .min(100);
                serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "type": "transfer",
                        "status": entry.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": 0,
                            "releases_failed": if entry.status == "failed" { 1 } else { 0 },
                        },
                        "progress_percent": progress,
                        "filename": entry.filename,
                        "created_at": entry.requested_at,
                        "updated_at": entry.updated_at,
                })
            }));
            let total = jobs.len();
            drop(transfers);
            drop(searches);
            Ok(routing::ok_response(
                serde_json::json!({
                    "jobs": jobs,
                    "limit": 100,
                    "offset": 0,
                    "total": total,
                    "has_more": total > 100,
                })
                .to_string(),
            ))
        }
        ("GET", path) if path.starts_with("/api/jobs/discography/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/discography/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            if let Some(job) = state
                .controller_features
                .read()
                .await
                .get(&format!("job/discography/{job_id}"))
                .cloned()
            {
                return Ok(routing::ok_response(job.to_string()));
            }
            let searches = state.searches.read().await;
            let Some(record) = searches.get_by_identifier(&job_id) else {
                return Ok(routing::not_found_response());
            };
            let artist = record
                .query
                .strip_suffix(" discography")
                .unwrap_or(&record.query)
                .trim();
            return Ok(routing::ok_response(
                serde_json::json!({
                    "jobId": record.id,
                    "artistId": artist,
                    "artistName": artist,
                    "profile": "CoreDiscography",
                    "targetDirectory": "",
                    "releaseJobIds": [],
                    "releaseIds": [],
                    "totalReleases": 0,
                    "completedReleases": 0,
                    "failedReleases": 0,
                    "status": "Pending",
                    "createdAt": record.created_at.to_string(),
                })
                .to_string(),
            ));
        }
        ("GET", path) if path.starts_with("/api/jobs/label-crate/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/label-crate/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            let features = state.controller_features.read().await;
            let Some(job) = features.get(&format!("job/label-crate/{job_id}")).cloned() else {
                return Ok(routing::not_found_response());
            };
            Ok(routing::ok_response(job.to_string()))
        }
        ("GET", path) if path.starts_with("/api/jobs/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(&job_id) {
                let progress = if record.status == "completed" {
                    100
                } else {
                    50
                };
                let body = serde_json::json!({
                    "id": record.id,
                    "kind": "search",
                    "status": record.status,
                    "progress": progress,
                    "query": record.query,
                    "target": record.target,
                    "result_count": record.results.len(),
                    "created_at": record.created_at,
                    "updated_at": record.updated_at,
                })
                .to_string();
                drop(searches);
                return Ok(routing::ok_response(body));
            }
            drop(searches);

            if let Some(job) = state
                .controller_features
                .read()
                .await
                .entries_with_prefix("job/")
                .into_iter()
                .find_map(|(_, job)| {
                    (job.get("jobId").and_then(serde_json::Value::as_str) == Some(job_id.as_str()))
                        .then_some(job)
                })
            {
                return Ok(routing::ok_response(job.to_string()));
            }

            let transfer_id = job_id.strip_prefix("transfer-").unwrap_or(&job_id);
            let transfers = state.transfers.read().await;
            let body = transfer_id
                .parse::<u64>()
                .ok()
                .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id))
                .map(|entry| {
                    let size = entry.size.unwrap_or(0);
                    let progress = entry
                        .bytes_transferred
                        .saturating_mul(100)
                        .checked_div(size)
                        .unwrap_or(0)
                        .min(100);
                    serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "status": entry.status,
                        "progress": progress,
                        "filename": entry.filename,
                        "bytesTransferred": entry.bytes_transferred,
                        "size": size,
                        "created_at": entry.requested_at,
                        "updated_at": entry.updated_at,
                    })
                })
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "id": job_id,
                        "status": "not_found",
                        "progress": 0,
                    })
                })
                .to_string();
            drop(transfers);
            if let Some(job) = state.library.read().await.remediation_job(&job_id) {
                return Ok(routing::ok_response(job.json().to_string()));
            }
            if state.config.controller_profile == ControllerProfile::Native
                && body.contains("\"status\":\"not_found\"")
            {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(body))
        }

        // LIBRARY HEALTH ENDPOINTS
        ("GET", "/api/library/health/summary") => {
            let library_path = query_parameter(route.query, "libraryPath").unwrap_or_default();
            if library_path.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "libraryPath query parameter is required",
                ));
            }
            let library = state.library.read().await;
            let json = library.health_summary_json(library_path);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/dashboard") => {
            let library_path = query_parameter(route.query, "libraryPath").unwrap_or_default();
            if library_path.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "libraryPath query parameter is required",
                ));
            }
            let artist_limit = match query_bounded_usize(route.query, "artistLimit", 1, 100) {
                Ok(value) => value.unwrap_or(10),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "artistLimit must be between 1 and 100",
                    ));
                }
            };
            let issue_limit = match query_bounded_usize(route.query, "issueLimit", 1, 250) {
                Ok(value) => value.unwrap_or(100),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "issueLimit must be between 1 and 250",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_dashboard_json(library_path, artist_limit, issue_limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues") => {
            let filter = match LibraryHealthIssueQuery::from_query(route.query) {
                Ok(filter) => filter,
                Err(error) => {
                    return Ok(routing::bad_request_response(&error));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_json(&filter);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-artist") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                Ok(value) => value.unwrap_or(20),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 100",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_artist_json(limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-release") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                Ok(value) => value.unwrap_or(20),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 100",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_release_json(limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-codec") => {
            let library = state.library.read().await;
            let json = library.health_issues_by_codec_json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/issues/by-type") => {
            let issue_type = if path == "/api/library/health/issues/by-type" {
                None
            } else if let Some(issue_type) =
                path_segment_after(path, "/api/library/health/issues/by-type/")
            {
                Some(issue_type)
            } else {
                return Ok(routing::not_found_response());
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_type_json(issue_type);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/scans/") => {
            let Some(scan_id) = path_segment_after(path, "/api/library/health/scans/") else {
                return Ok(routing::not_found_response());
            };
            let mut library = state.library.write().await;
            library.refresh_health_scans();
            let scan = library.health_scan(scan_id);
            drop(library);
            Ok(scan
                .map(|record| routing::ok_response(record.json()))
                .unwrap_or_else(routing::not_found_response))
        }
        ("POST", "/api/library/health/scans") => {
            if route.path.starts_with("/api/v0/")
                && (body.trim().is_empty()
                    || !serde_json::from_str::<serde_json::Value>(body)
                        .is_ok_and(|value| value.is_object()))
            {
                return Ok(routing::bad_request_response(
                    "library scan body must be an object",
                ));
            }
            let library_path = extract_json_string_field(body, "libraryPath")
                .or_else(|| extract_json_string_field(body, "path"))
                .unwrap_or_default();
            let mut library = state.library.write().await;
            let scan = match library.start_health_scan(library_path) {
                Ok(scan) => scan,
                Err(active_id) => {
                    return Ok(routing::conflict_response(&format!(
                        "scan already running: {active_id}"
                    )));
                }
            };
            drop(library);
            Ok(routing::ok_response(
                serde_json::json!({
                    "scanId": scan.id,
                    "message": "Scan started successfully",
                })
                .to_string(),
            ))
        }
        ("POST", "/api/library/health/issues/fix") => {
            if route.path.starts_with("/api/v0/")
                && !body.trim().is_empty()
                && !serde_json::from_str::<serde_json::Value>(body)
                    .is_ok_and(|value| value.is_object())
            {
                return Ok(routing::bad_request_response(
                    "library remediation body must be an object",
                ));
            }
            let mut library = state.library.write().await;
            let previous = library.clone();
            let fixable = library
                .health_issues()
                .into_iter()
                .filter(|issue| {
                    issue.get("type").and_then(serde_json::Value::as_str) == Some("missing_kind")
                })
                .count();
            let fixed = library.fix_health_issues();
            let records = library.records.clone();
            let remaining = library.health_issues().len();
            let updated_at = library.updated_at;
            let mutated = library.clone();
            drop(library);
            if !fixed.is_empty() {
                if let Err(error) = persist_library_items_checked(state, &records).await {
                    rollback_library_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "fixed": fixed.len(),
                    "fixable": fixable,
                    "issues": fixed,
                    "remaining": remaining,
                    "persisted": true,
                    "updated_at": updated_at,
                })
                .to_string(),
            ))
        }

        // CONFIGURATION ENDPOINTS
        ("GET", "/api/config/preferences") => {
            let runtime = state.runtime.read().await;
            let body = serde_json::json!({
                "auto_connect": state.config.auto_connect,
                "transfer_allow_outbound": state.config.transfer_allow_outbound,
                "transfer_max_active": state.config.transfer_max_active,
                "autoreplace_enabled": runtime.autoreplace_enabled,
            })
            .to_string();
            drop(runtime);
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/config/preferences") => {
            let requested = extract_json_bool_field(body, "autoreplace_enabled")
                .or_else(|| extract_json_bool_field(body, "autoreplaceEnabled"));
            let response = match mutate_runtime_compat_state(state, |runtime, _| {
                if let Some(enabled) = requested {
                    runtime.set_autoreplace(enabled);
                }
                serde_json::json!({
                    "auto_connect": state.config.auto_connect,
                    "transfer_allow_outbound": state.config.transfer_allow_outbound,
                    "transfer_max_active": state.config.transfer_max_active,
                    "autoreplace_enabled": runtime.autoreplace_enabled,
                    "persisted": true,
                    "updated_at": runtime.updated_at,
                })
                .to_string()
            })
            .await
            {
                Ok(response) => response,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(response))
        }

        // ADDITIONAL MISSING PUT ENDPOINTS (Phase 5)
        ("PUT", "/api/autoreplace/disable") => {
            let versioned = route.path.starts_with("/api/v0/");
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let legacy = runtime.set_autoreplace(false);
                if versioned {
                    serde_json::json!({
                        "enabled": false,
                        "lastRunAt": null,
                        "lastRunProcessedCount": 0,
                        "lastRunReplacedCount": 0,
                        "intervalSeconds": 300,
                    })
                    .to_string()
                } else {
                    legacy.to_string()
                }
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/autoreplace/enable") => {
            let versioned = route.path.starts_with("/api/v0/");
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let legacy = runtime.set_autoreplace(true);
                if versioned {
                    serde_json::json!({
                        "enabled": true,
                        "lastRunAt": null,
                        "lastRunProcessedCount": 0,
                        "lastRunReplacedCount": 0,
                        "intervalSeconds": 300,
                    })
                    .to_string()
                } else {
                    legacy.to_string()
                }
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        // ADDITIONAL MISSING BRIDGE ENDPOINTS (Phase 6)
        ("GET", "/api/bridge/admin/clients") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let status = if runtime.bridge_running {
                "running"
            } else if bridge.enabled {
                "configured"
            } else {
                "disabled"
            };
            // Matches the oracle's real BridgeDashboard client snapshot;
            // only sessions accepted by the embedded Soulfind-protocol
            // listener are included.
            let clients = runtime
                .bridge_active_clients
                .values()
                .cloned()
                .collect::<Vec<_>>();
            if state.config.controller_profile == ControllerProfile::Native {
                drop(runtime);
                return Ok(routing::ok_response(
                    serde_json::json!({"clients": clients}).to_string(),
                ));
            }
            let json = serde_json::json!({
                "clients": clients,
                "count": runtime.bridge_active_clients.len(),
                "status": status,
                "ready": bridge.enabled,
            });
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/admin/config") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            Ok(routing::ok_response(
                serde_json::json!({
                    "enabled": bridge.enabled,
                    "port": bridge.port,
                    "soulfind_path": "soulfind",
                    "max_clients": bridge.max_clients,
                    "require_auth": bridge.require_auth,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/bridge/admin/dashboard") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let active_transfers = transfers
                .entries
                .iter()
                .filter(|entry| {
                    matches!(
                        entry.status.as_str(),
                        "queued" | "in_progress" | "requested"
                    )
                })
                .count();
            let bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let transfer_count = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let started_at = runtime
                    .bridge_started_at
                    .map(bridge_started_at_string)
                    .unwrap_or_else(|| "0001-01-01T00:00:00+00:00".to_owned());
                let uptime = runtime
                    .bridge_started_at
                    .map(|started| {
                        format_timespan_hms(
                            i64::try_from(unix_timestamp().saturating_sub(started))
                                .unwrap_or(i64::MAX),
                        )
                    })
                    .unwrap_or_else(|| "00:00:00".to_owned());
                let json = serde_json::json!({
                    "health": {
                        "isHealthy": runtime.bridge_running,
                        "version": "1.0.0-proxy",
                        "activeConnections": runtime.bridge_active_clients.len(),
                        "startedAt": started_at,
                    },
                    "connectedClients": [],
                    "stats": {
                        "totalConnections": runtime.bridge_total_connections,
                        "currentConnections": runtime.bridge_active_clients.len(),
                        "totalSearches": runtime.bridge_total_searches,
                        "totalDownloads": runtime.bridge_total_downloads,
                        "totalRoomJoins": runtime.bridge_total_room_joins,
                        "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                        "uptime": uptime,
                    },
                    "meshBenefits": {
                        "bytesViaMesh": 0,
                        "bytesViaSoulseek": 0,
                        "meshPercentage": 0.0,
                        "disasterModeActivations": 0,
                        "timeInDisasterMode": "00:00:00",
                    },
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            // Matches the oracle's real BridgeDashboardData contract:
            // Local HTTP transfer activity remains separate from the
            // protocol bridge counters below.
            let json = serde_json::json!({
                "health": if runtime.bridge_running { "Healthy" } else { "Disabled" },
                "connectedClients": runtime.bridge_active_clients.len(),
                "stats": {
                    "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                    "totalConnections": runtime.bridge_total_connections,
                    "currentConnections": runtime.bridge_active_clients.len(),
                    "totalSearches": runtime.bridge_total_searches,
                    "totalDownloads": runtime.bridge_total_downloads,
                    "totalRoomJoins": runtime.bridge_total_room_joins,
                    "uptime": runtime.bridge_started_at.map(|started| format_timespan_hms(
                        i64::try_from(unix_timestamp().saturating_sub(started)).unwrap_or(i64::MAX)
                    )).unwrap_or_else(|| "00:00:00".to_owned()),
                },
                "meshBenefits": {"enabled": bridge.enabled},
                "active_clients": runtime.bridge_active_clients.len(),
                "transfers": transfer_count,
                "active_transfers": active_transfers,
                "total_bytes": bytes,
                "uptime_seconds": 0,
                "enabled": bridge.enabled,
                "running": runtime.bridge_running,
                "configUpdates": runtime.bridge_config_updates,
                "host": serde_json::Value::Null,
                "port": serde_json::Value::Null,
                "endpoint_configured": bridge.endpoint_configured(),
            });
            drop(runtime);
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/admin/stats") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let total_bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let active_sessions = transfers
                .entries
                .iter()
                .filter(|entry| {
                    matches!(
                        entry.status.as_str(),
                        "queued" | "in_progress" | "requested"
                    )
                })
                .count();
            let total_requests = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let uptime = runtime
                    .bridge_started_at
                    .map(|started| {
                        format_timespan_hms(
                            i64::try_from(unix_timestamp().saturating_sub(started))
                                .unwrap_or(i64::MAX),
                        )
                    })
                    .unwrap_or_else(|| "00:00:00".to_owned());
                let json = serde_json::json!({
                    "totalConnections": runtime.bridge_total_connections,
                    "currentConnections": runtime.bridge_active_clients.len(),
                    "totalSearches": runtime.bridge_total_searches,
                    "totalDownloads": runtime.bridge_total_downloads,
                    "totalRoomJoins": runtime.bridge_total_room_joins,
                    "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                    "uptime": uptime,
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            // Local HTTP transfer activity remains separate from the
            // protocol bridge counters below.
            let json = serde_json::json!({
                "totalConnections": runtime.bridge_total_connections,
                "currentConnections": runtime.bridge_active_clients.len(),
                "totalSearches": runtime.bridge_total_searches,
                "totalDownloads": runtime.bridge_total_downloads,
                "totalRoomJoins": runtime.bridge_total_room_joins,
                "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                "uptime": runtime.bridge_started_at.map(|started| format_timespan_hms(
                    i64::try_from(unix_timestamp().saturating_sub(started)).unwrap_or(i64::MAX)
                )).unwrap_or_else(|| "00:00:00".to_owned()),
                "total_requests": total_requests,
                "total_bytes": total_bytes,
                "active_sessions": active_sessions,
                "enabled": bridge.enabled,
                "running": runtime.bridge_running,
                "configUpdates": runtime.bridge_config_updates,
            });
            drop(runtime);
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/status") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let transfer_count = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let started_at = runtime
                    .bridge_started_at
                    .map(bridge_started_at_string)
                    .unwrap_or_else(|| "0001-01-01T00:00:00+00:00".to_owned());
                let json = serde_json::json!({
                    "isHealthy": runtime.bridge_running,
                    "version": "1.0.0-proxy",
                    "activeConnections": runtime.bridge_active_clients.len(),
                    "startedAt": started_at,
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            let uptime_seconds = runtime
                .bridge_started_at
                .map(|started| unix_timestamp().saturating_sub(started))
                .unwrap_or(0);
            let json = format!(
                 "{{\"status\":\"{}\",\"version\":\"1.0.0\",\"uptime_seconds\":{},\"enabled\":{},\"configured\":{},\"running\":{},\"configUpdates\":{},\"host\":\"{}\",\"port\":{},\"endpoint_configured\":{},\"transfers\":{},\"next_action\":\"{}\"}}",
                 if runtime.bridge_running { "running" } else if bridge.enabled { "configured" } else { "disabled" },
                 uptime_seconds,
                 bridge.enabled,
                 bridge.enabled && bridge.endpoint_configured(),
                 runtime.bridge_running,
                 runtime.bridge_config_updates,
                 bridge.bind_address,
                 bridge.port,
                 bridge.endpoint_configured(),
                 transfer_count,
                 if runtime.bridge_running {
                     "accept bridge traffic"
                 } else if bridge.enabled {
                     "start bridge service"
                 } else {
                     "enable bridge integration"
                 }
             );
            drop(runtime);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/bridge/transfer/") => {
            Ok(bridge_transfer_progress_response(path, state).await)
        }

        ("POST", "/api/bridge/start") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                return Ok(routing::ok_response(
                    serde_json::json!({"status": "started"}).to_string(),
                ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.set_bridge_running(true, bridge.enabled);
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "next_action".to_owned(),
                        serde_json::json!(if bridge.enabled {
                            "accept bridge traffic"
                        } else {
                            "enable bridge integration"
                        }),
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("POST", "/api/bridge/stop") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                return Ok(routing::ok_response(
                    serde_json::json!({"status": "stopped"}).to_string(),
                ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime
                    .set_bridge_running(false, bridge.enabled)
                    .to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/bridge/admin/config") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                if body.trim().is_empty()
                    || serde_json::from_str::<serde_json::Value>(body).is_err()
                {
                    return Ok(routing::bad_request_response("Request is required"));
                }
                return Ok(routing::ok_response(
                     serde_json::json!({
                         "message": "Configuration updated. Restart bridge service to apply changes.",
                         "restart_required": true,
                     })
                     .to_string(),
                 ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let accepted_keys = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|value| {
                    value
                        .as_object()
                        .map(|object| object.keys().cloned().collect::<Vec<_>>())
                })
                .unwrap_or_default();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.record_bridge_config_update(bridge.enabled, accepted_keys);
                if route.path.starts_with("/api/v0/") {
                    value["message"] = serde_json::json!(
                        "Configuration updated. Restart bridge service to apply changes."
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("PUT", path)
            if path.starts_with("/api/collections/") && path.contains("/items/reorder") =>
        {
            let collection_id = path
                .strip_prefix("/api/collections/")
                .and_then(|rest| rest.strip_suffix("/items/reorder"))
                .unwrap_or_default();
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut collections = state.collections.write().await;
            if collections.get(collection_id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            if let Some(record) = collections.reorder_items(collection_id, body) {
                let mutated = collections.clone();
                let items = record
                    .items
                    .iter()
                    .map(|item| {
                        serde_json::from_str::<serde_json::Value>(&item.json())
                            .unwrap_or_else(|_| serde_json::json!({ "id": item.id }))
                    })
                    .collect::<Vec<_>>();
                let item_count = items.len();
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(
                    serde_json::json!({
                        "reordered": true,
                        "collection_id": collection_id,
                        "items": items,
                        "itemCount": item_count,
                    })
                    .to_string(),
                ))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }

        ("PUT", path) if conversation_message_path(path).is_some() => {
            let Some((username, id)) = conversation_message_path(path) else {
                return Ok(routing::not_found_response());
            };
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let username = decoded_path_segment(username);
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let updated = messages
                .records
                .iter()
                .any(|record| record.username == username && record.id == id);
            let response = if updated {
                messages.ack(id);
                let mutated = messages.clone();
                drop(messages);
                if let Err(error) = persist_message_ack_checked(state, id).await {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                if matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                ) && route.path.starts_with("/api/v0/")
                {
                    HttpResponse {
                        status: "200 OK",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response("true".to_owned())
                }
            } else {
                drop(messages);
                routing::not_found_response()
            };
            Ok(response)
        }
        ("PUT", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            if route.path.starts_with("/api/v0/")
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let ids = messages
                .records
                .iter()
                .filter(|record| record.username == username && !record.acknowledged)
                .map(|record| record.id)
                .collect::<Vec<_>>();
            messages.ack_all_for_user(&username);
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_acks_checked(state, &ids).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(
                if matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                ) && route.path.starts_with("/api/v0/")
                {
                    HttpResponse {
                        status: "200 OK",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response("true".to_owned())
                },
            )
        }

        ("PUT", "/api/nowplaying") => {
            let versioned = route.path.starts_with("/api/v0/");
            if versioned && body.trim().is_empty() {
                return Ok(routing::bad_request_response("Track data is required"));
            }
            let payload = if versioned {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(payload) if !payload.is_null() => Some(payload),
                    Ok(_) if body.trim() == "null" => {
                        return Ok(routing::bad_request_response("Track data is required"));
                    }
                    _ => None,
                }
            } else {
                None
            };
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let artist = payload
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|object| json_object_field_ci(object, "artist"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    (!versioned)
                        .then(|| extract_json_string_field(body, "artist"))
                        .flatten()
                })
                .unwrap_or_default();
            let title = payload
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|object| json_object_field_ci(object, "title"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    (!versioned)
                        .then(|| extract_json_string_field(body, "title"))
                        .flatten()
                })
                .unwrap_or_default();
            if artist.trim().is_empty() || title.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "Artist and title are required",
                ));
            }
            let mut now_playing = state.now_playing.write().await;
            let previous = now_playing.clone();
            let record = now_playing.upsert(username, artist, title);
            let mutated = now_playing.clone();
            drop(now_playing);
            if let Err(error) = persist_now_playing_checked(state, &record).await {
                let mut now_playing = state.now_playing.write().await;
                if *now_playing == mutated {
                    *now_playing = previous;
                }
                drop(now_playing);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(HttpResponse {
                status: "204 No Content",
                content_type: "",
                body: String::new(),
            })
        }

        ("PUT", "/api/options/yaml") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(apply_controller_yaml_upload(body, state).await)
        }

        ("PUT", "/api/profile/me") => {
            if route.path.starts_with("/api/v0/") {
                let payload = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Object(payload)) => payload,
                    Ok(serde_json::Value::Null) => {
                        return Ok(routing::bad_request_response("Request is required."));
                    }
                    Ok(_) | Err(_) => {
                        return Ok(routing::bad_request_response("Request is required."));
                    }
                };
                let display_name = extract_json_string_field(body, "displayName")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if display_name.is_empty() {
                    return Ok(routing::bad_request_response("DisplayName is required."));
                }
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                let avatar = extract_json_string_field(body, "avatar")
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.trim().to_owned());
                let capabilities = extract_json_i32_field(body, "capabilities").unwrap_or(0);
                let endpoints = payload
                    .get("endpoints")
                    .cloned()
                    .filter(serde_json::Value::is_array)
                    .unwrap_or_else(|| serde_json::json!([]));
                state.session.write().await.username = Some(display_name.clone());
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "peerId": descriptor.peer_id,
                        "publicKey": STANDARD.encode(descriptor.public_key),
                        "displayName": display_name,
                        "avatar": avatar,
                        "capabilities": capabilities,
                        "endpoints": endpoints,
                        "createdAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "expiresAt": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
                        "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                    })
                    .to_string(),
                ));
            }
            let username = extract_json_string_field(body, "username")
                .or_else(|| extract_json_string_field(body, "name"));
            let privileges_seconds = extract_json_u32_field(body, "privilegesSeconds")
                .or_else(|| extract_json_u32_field(body, "privileges_seconds"));
            let connected = extract_json_bool_field(body, "connected");
            let supporter = extract_json_bool_field(body, "supporter");
            let mut session = state.session.write().await;
            if let Some(username) = username.filter(|value| !value.trim().is_empty()) {
                session.username = Some(username);
            }
            if let Some(privileges_seconds) = privileges_seconds {
                session.privileges_seconds = Some(privileges_seconds);
            }
            if let Some(supporter) = supporter {
                session.supporter = Some(supporter);
            }
            if let Some(connected) = connected {
                session.state = if connected {
                    "connected"
                } else {
                    "disconnected"
                };
                session.connected_at = if connected {
                    session.connected_at.or_else(|| Some(unix_timestamp()))
                } else {
                    None
                };
            }
            session.updated_at = unix_timestamp();
            let username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            let privileges_seconds = session.privileges_seconds.unwrap_or(0);
            let connected = session.state == "connected";
            let updated_at = session.updated_at;
            drop(session);
            let users = state.users.read().await;
            let watched = users
                .records
                .iter()
                .any(|user| user.username.eq_ignore_ascii_case(&username) && user.watched);
            drop(users);
            Ok(routing::ok_response(
                serde_json::json!({
                    "updated": true,
                    "persisted": true,
                    "profile": {
                        "username": username,
                        "description": "",
                        "picture": "",
                        "user_type": if privileges_seconds > 0 { "privileged" } else { "normal" },
                        "privilegesSeconds": privileges_seconds,
                        "connected": connected,
                        "watched": watched,
                        "updated_at": updated_at,
                    }
                })
                .to_string(),
            ))
        }

        ("PUT", "/api/relay") => {
            let relay_enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
            let json = match mutate_runtime_compat_state(state, |_, relay| {
                relay.set_enabled(relay_enabled).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        ("PUT", "/api/relay/agent") => {
            let enabled = extract_json_bool_field(body, "enabled").unwrap_or(true);
            let json = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_relay_agent(enabled).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        ("PUT", path) if path.starts_with("/api/searches/") => {
            let Some(id) = path_segment_after(path, "/api/searches/") else {
                return Ok(routing::not_found_response());
            };
            if route.path.starts_with("/api/v0/")
                && matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                )
            {
                let mut searches = state.searches.write().await;
                let previous_searches = searches.clone();
                let Some(existing) = searches.get_by_identifier(id) else {
                    drop(searches);
                    return Ok(routing::not_found_response());
                };
                let Some((record, transitioned)) =
                    searches.set_status_by_token(existing.token, "cancelled")
                else {
                    drop(searches);
                    return Ok(routing::not_found_response());
                };
                let mutated_searches = searches.clone();
                drop(searches);
                if transitioned {
                    if let Err(error) = persist_search_record(state, &record).await {
                        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                            .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
                return Ok(routing::ok_response(String::new()));
            }
            let query = extract_json_string_field(body, "query")
                .or_else(|| extract_json_string_field(body, "searchText"));
            let status = extract_json_string_field(body, "status")
                .or_else(|| extract_json_string_field(body, "state"))
                .or_else(|| {
                    extract_json_bool_field(body, "isComplete").map(|is_complete| {
                        if is_complete {
                            "completed".to_owned()
                        } else {
                            "active".to_owned()
                        }
                    })
                });
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            match searches.update_by_identifier(id, query, status.as_deref()) {
                Some((record, updated)) => {
                    let mut value = serde_json::from_str::<serde_json::Value>(&record.json())
                        .map_err(|error| format!("search json build failed: {error}"))?;
                    if let Some(object) = value.as_object_mut() {
                        object.insert("updated".to_owned(), serde_json::Value::Bool(updated));
                    }
                    let mutated_searches = searches.clone();
                    drop(searches);
                    if let Err(error) = persist_search_record(state, &record).await {
                        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                            .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::ok_response(value.to_string()))
                }
                None => {
                    drop(searches);
                    Ok(routing::not_found_response())
                }
            }
        }

        ("PUT", "/api/transfers/downloads/accelerated") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                let enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
                {
                    let mut runtime = state.runtime.write().await;
                    runtime.accelerated_downloads_enabled = enabled;
                    runtime.updated_at = unix_timestamp();
                }
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "enabled": enabled,
                        "updatedAt": chrono::Utc::now().to_rfc3339(),
                        "policy": "Normal downloads remain single-source. Underperforming downloads may use verified alternate sources; raw Soulseek peers use sequential failover, while true multipart chunking is reserved for trusted mesh-overlay peers.",
                    })
                    .to_string(),
                ));
            }
            let transfers = state.transfers.read().await;
            let mut payload = serde_json::from_str::<serde_json::Value>(
                &controller_accelerated_downloads_json(route.query, &transfers),
            )
            .map_err(|error| format!("accelerated json failed: {error}"))?;
            drop(transfers);
            payload["persisted"] = serde_json::Value::Bool(false);
            Ok(routing::ok_response(payload.to_string()))
        }

        ("PUT", "/api/wishlist/bulk-filter") => {
            let ids = extract_json_string_array_field(body, "ids")
                .or_else(|| extract_json_string_array_field(body, "itemIds"))
                .unwrap_or_default();
            if ids.is_empty() {
                return Ok(routing::bad_request_response(
                    "At least one wishlist item ID is required",
                ));
            }
            let filter = extract_json_string_field(body, "filter").unwrap_or_default();
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let updated = match wishlist.update_filters(&ids, filter) {
                Ok(items) => items,
                Err("wishlist item not found") => {
                    drop(wishlist);
                    return Ok(routing::not_found_response());
                }
                Err(message) => {
                    drop(wishlist);
                    return Ok(routing::bad_request_response(message));
                }
            };
            let mutated = wishlist.clone();
            let updated_count = updated.len();
            drop(wishlist);
            if let Err(error) = persist_wishlist_items_checked(state, &updated).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::ok_response(
                serde_json::json!({ "updatedCount": updated_count }).to_string(),
            ))
        }

        ("PUT", path) if path.starts_with("/api/wishlist/") => {
            let Some(item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let search_text = extract_json_string_field(body, "searchText");
            if compatibility_contract
                && search_text
                    .as_ref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Ok(routing::bad_request_response("SearchText is required"));
            }
            let artist = if compatibility_contract {
                search_text.clone()
            } else {
                extract_json_string_field(body, "artist")
            };
            let title = if compatibility_contract {
                Some(String::new())
            } else {
                extract_json_string_field(body, "title").or(search_text)
            };
            let kind = extract_json_string_field(body, "kind");
            let filter = extract_json_string_field(body, "filter");
            let enabled =
                extract_json_bool_field(body, "enabled").or(compatibility_contract.then_some(true));
            let auto_download = extract_json_bool_field(body, "autoDownload")
                .or(compatibility_contract.then_some(false));
            let max_results = extract_json_u64_field(body, "maxResults")
                .or(compatibility_contract.then_some(100));
            if max_results.is_some_and(|value| value == 0 || value > MAX_WISHLIST_RESULTS as u64) {
                return Ok(routing::bad_request_response(
                    "MaxResults must be between 1 and 10000",
                ));
            }
            let max_downloads = extract_json_optional_u64_field(body, "maxDownloads")
                .or(compatibility_contract.then_some(None));
            if max_downloads
                .flatten()
                .is_some_and(|value| value == 0 || value > MAX_WISHLIST_DOWNLOADS)
            {
                return Ok(routing::bad_request_response(
                    "MaxDownloads must be null or between 1 and 1000000",
                ));
            }
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            if let Some(item) = wishlist.update_item(
                item_id,
                artist,
                title,
                kind,
                filter,
                enabled,
                auto_download,
                max_results.and_then(|value| usize::try_from(value).ok()),
                max_downloads,
            ) {
                let mutated = wishlist.clone();
                let json = if compatibility_contract {
                    item.native_json()
                } else {
                    item.json()
                };
                drop(wishlist);
                if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    return Ok(wishlist_storage_error_response(
                        compatibility_contract,
                        &error,
                    ));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(wishlist);
                Ok(routing::not_found_response())
            }
        }

        // Generic :var pattern PUT endpoints (Phase 5)
        ("PUT", path)
            if path.contains("/channels/")
                && path.matches('/').count() == 4
                && !path.contains("/api/") =>
        {
            Ok(routing::not_found_response())
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn route_dispatch_group_6(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("PUT", path) if path.ends_with("/adversarial") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.contains("/disclosure/") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.ends_with("/reputation") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("GET", "/api/config/shares") => {
            let shares = state.shares.read().await;
            let share_roots: Vec<String> = shares.roots
                .iter()
                .map(|root| format!(
                    "{{\"label\":\"{}\",\"files\":{},\"bytes\":{}}}",
                    json_escape(&root.label),
                    root.files,
                    root.bytes
                ))
                .collect();
            let json = format!(
                "{{\"roots\":[{}],\"count\":{}}}",
                share_roots.join(","),
                shares.roots.len()
            );
            drop(shares);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/shares") => {
            let path = extract_json_string_field(body, "path").unwrap_or_default();
            if path.is_empty() {
                return Ok(routing::bad_request_response("path is required"));
            }
            let json = format!(
                "{{\"path\":\"{}\",\"added\":true,\"files\":0,\"bytes\":0}}",
                json_escape(&path)
            );
            Ok(routing::created_response(json))
        }

        ("GET", "/api/config/plugins") => {
            let integrations = state.integration_settings.read().await;
            let plugins = serde_json::json!([
                {
                    "id": "spotify",
                    "name": "Spotify",
                    "enabled": integrations.spotify.enabled,
                    "configured": integrations.spotify.configured(),
                },
                {
                    "id": "lidarr",
                    "name": "Lidarr",
                    "enabled": integrations.lidarr.enabled,
                    "configured": integrations.lidarr.configured(),
                },
                {
                    "id": "external-visualizer",
                    "name": "External Visualizer",
                    "enabled": state.config.integrations.external_visualizer.launch_enabled,
                    "configured": state.config.integrations.external_visualizer.configured(),
                },
                {
                    "id": "bridge",
                    "name": "Bridge",
                    "enabled": state.config.integrations.bridge.enabled,
                    "configured": state.config.integrations.bridge.enabled,
                }
            ]);
            let json = serde_json::json!({
                "plugins": plugins,
                "count": plugins.as_array().map_or(0, Vec::len),
            }).to_string();
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/filters") => {
            let filter_type = extract_json_string_field(body, "type").unwrap_or_default();
            let pattern = extract_json_string_field(body, "pattern").unwrap_or_default();
            let json = format!(
                "{{\"type\":\"{}\",\"pattern\":\"{}\",\"created_at\":{}}}",
                json_escape(&filter_type),
                json_escape(&pattern),
                unix_timestamp()
            );
            Ok(routing::created_response(json))
        }

        // ADMIN/SYSTEM ENDPOINTS
        ("GET", "/api/admin/stats") => {
            let transfers = state.transfers.read().await;
            let total_bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let active_transfers = transfers
                .entries
                .iter()
                .filter(|entry| matches!(entry.status.as_str(), "queued" | "requested" | "in_progress"))
                .count();
            let searches = state.searches.read().await;
            let users = state.users.read().await;
            let rooms = state.rooms.read().await;
            let shares = state.shares.read().await;
            let json = serde_json::json!({
                "total_transfers": transfers.entries.len(),
                "active_transfers": active_transfers,
                "total_bytes": total_bytes,
                "searches": searches.records.len(),
                "users": users.records.len(),
                "rooms": rooms.records.len(),
                "shared_files": shares.entries.len(),
            }).to_string();
            drop(shares);
            drop(rooms);
            drop(users);
            drop(searches);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        // RECOMMENDATIONS & ANALYTICS ENDPOINTS
        ("GET", "/api/soulseek/recommendations") => {
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_recommendations_json()
            } else {
                interests.recommendations_json("recommendations")
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/soulseek/recommendations/global") => {
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_recommendations_json()
            } else {
                interests.recommendations_json("global_recommendations")
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/soulseek/items/") && path.ends_with("/recommendations") => {
            let Some(item_id) =
                path_segment_between(path, "/api/soulseek/items/", "/recommendations")
            else {
                return Ok(routing::not_found_response());
            };
            let item_id = decoded_path_segment(item_id).trim().to_owned();
            if item_id.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_item_recommendations_json(&item_id)
            } else {
                interests.item_recommendations_json(&item_id)
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/soulseek/items/") && path.ends_with("/similar-users") => {
            let Some(item_id) =
                path_segment_between(path, "/api/soulseek/items/", "/similar-users")
            else {
                return Ok(routing::not_found_response());
            };
            let item_id = decoded_path_segment(item_id).trim().to_owned();
            if item_id.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            let users = state.users.read().await;
            if route.path.starts_with("/api/v0/") {
                let usernames = users
                    .records
                    .iter()
                    .filter(|user| user.watched || user.status.as_deref() == Some("online"))
                    .map(|user| user.username.clone())
                    .collect::<Vec<_>>();
                let json = serde_json::json!({
                    "item": item_id,
                    "usernames": usernames,
                })
                .to_string();
                drop(users);
                return Ok(routing::ok_response(json));
            }
            let similar_users = users
                .records
                .iter()
                .filter(|user| user.watched || user.status.as_deref() == Some("online"))
                .map(|user| {
                    serde_json::json!({
                        "username": user.username,
                        "status": user.status,
                        "watched": user.watched,
                        "score": 1.0,
                    })
                })
                .collect::<Vec<_>>();
            let count = similar_users.len();
            drop(users);
            let json = serde_json::json!({
                "item_id": item_id,
                "similar_users": similar_users,
                "count": count,
            })
            .to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/relay/controller/downloads/") => {
            let Some(_token) = path_segment_after(path, "/api/relay/controller/downloads/") else {
                return Ok(routing::not_found_response());
            };
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/octet-stream",
                body: String::new(),
            })
        }

        ("POST", path) if path.starts_with("/api/relay/controller/files/") => {
            let Some(token) = path_segment_after(path, "/api/relay/controller/files/") else {
                return Ok(routing::not_found_response());
            };
            let token = decoded_path_segment(token);
            let relay = state.relay.read().await;
            let runtime = state.runtime.read().await;
            let body = serde_json::json!({
                "accepted": true,
                "token": token,
                "relay_enabled": relay.enabled,
                "relayAgentEnabled": runtime.relay_agent_enabled,
                "kind": "files",
                "updated_at": runtime.updated_at.max(relay.updated_at),
            }).to_string();
            drop(runtime);
            drop(relay);
            Ok(routing::ok_response(body))
        }

        ("POST", path) if path.starts_with("/api/relay/controller/shares/") => {
            let Some(token) = path_segment_after(path, "/api/relay/controller/shares/") else {
                return Ok(routing::not_found_response());
            };
            let token = decoded_path_segment(token);
            let shares = state.shares.read().await;
            let relay = state.relay.read().await;
            let runtime = state.runtime.read().await;
            let body = serde_json::json!({
                "accepted": true,
                "token": token,
                "relay_enabled": relay.enabled,
                "relayAgentEnabled": runtime.relay_agent_enabled,
                "kind": "shares",
                "shareCount": shares.entries.len(),
                "rootCount": shares.roots.len(),
                "updated_at": runtime.updated_at.max(relay.updated_at),
            }).to_string();
            drop(runtime);
            drop(relay);
            drop(shares);
            Ok(routing::ok_response(body))
        }

         // ADDITIONAL MISSING GET ENDPOINTS (Phase 5)
         ("GET", "/api/source-providers") => Ok(routing::ok_response(
             source_provider_catalog_json(state.config.virtual_soulfind_v2_enabled),
         )),

         ("GET", "/api/discovery") => {
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches);
             let total_users = sources
                 .iter()
                 .filter_map(|source| source.get("username").and_then(serde_json::Value::as_str))
                 .map(str::to_ascii_lowercase)
                 .collect::<HashSet<_>>()
                 .len();
             Ok(routing::ok_response(serde_json::json!({
                 "isRunning": discovery.running,
                 "currentSearchTerm": discovery.search_term,
                 "stats": {
                     "totalFiles": sources.len(),
                     "totalUsers": total_users,
                     "searchCycles": discovery.search_cycles,
                     "lastCycleNewFiles": discovery.last_cycle_new_files,
                     "hashVerificationEnabled": discovery.hash_verification_enabled,
                     "filesWithHash": sources.iter().filter(|source| !source["hash"].is_null()).count(),
                 }
             }).to_string()))
         }
         ("POST", "/api/discovery/start") => {
             let search_term = extract_json_string_field(body, "searchTerm")
                 .unwrap_or_default()
                 .trim()
                 .to_owned();
             if search_term.is_empty() {
                 return Ok(routing::bad_request_response("SearchTerm is required"));
             }
            if state.source_discovery.read().await.running {
                let current = state.source_discovery.read().await.search_term.clone();
                return Ok(routing::HttpResponse {
                    status: "409 Conflict",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "error": "Discovery already running",
                        "currentSearchTerm": current,
                        "hint": "Call /api/v0/discovery/stop first",
                    })
                    .to_string(),
                });
            }
            let hash_verification_enabled =
                extract_json_bool_field(body, "enableHashVerification").unwrap_or(true);
            let search_term = truncate_utf8_bytes(search_term, MAX_SEARCH_QUERY_BYTES);
            let record = match dispatch_source_discovery_search(state, search_term.clone()).await {
                Ok(record) => record,
                Err(error) if error == "session manager is not running" => {
                    match create_rescue_search(state, search_term.clone()).await {
                        Ok(record) => record,
                        Err(error) => return Ok(routing::service_unavailable_response(&error)),
                    }
                }
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
             state.source_discovery.write().await.start(
                 search_term.clone(),
                 hash_verification_enabled,
                 record.token,
             );
             Ok(routing::ok_response(serde_json::json!({
                 "message": "Discovery started",
                 "searchTerm": search_term,
                 "hashVerificationEnabled": hash_verification_enabled,
             }).to_string()))
        }
        ("POST", "/api/discovery/stop") => {
            if !state.source_discovery.read().await.running {
                return Ok(routing::ok_response(
                    serde_json::json!({"message": "Discovery not running"}).to_string(),
                ));
            }
            let mut discovery = state.source_discovery.write().await;
            discovery.running = false;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches);
             discovery.last_cycle_new_files = sources.len();
             let total_users = sources
                 .iter()
                 .filter_map(|source| source.get("username").and_then(serde_json::Value::as_str))
                 .map(str::to_ascii_lowercase)
                 .collect::<HashSet<_>>()
                 .len();
             Ok(routing::ok_response(serde_json::json!({
                 "message": "Discovery stopped",
                 "stats": {
                     "totalFiles": sources.len(),
                     "totalUsers": total_users,
                     "searchCycles": discovery.search_cycles,
                     "lastCycleNewFiles": discovery.last_cycle_new_files,
                     "hashVerificationEnabled": discovery.hash_verification_enabled,
                     "filesWithHash": 0,
                 }
             }).to_string()))
         }
         ("GET", path) if path.starts_with("/api/discovery/sources/by-size/") => {
             let Some(size) = path_segment_after(path, "/api/discovery/sources/by-size/")
                 .and_then(|size| size.parse::<u64>().ok())
                 .filter(|size| *size > 0)
             else {
                 return Ok(routing::bad_request_response("size must be greater than zero"));
             };
             let limit = match query_bounded_usize(route.query, "limit", 1, 1_000) {
                 Ok(limit) => limit.unwrap_or(100),
                 Err(()) => return Ok(routing::bad_request_response("limit must be greater than zero")),
             };
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches)
                 .into_iter()
                 .filter(|source| source["size"].as_u64() == Some(size))
                 .take(limit)
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "size": size,
                 "sourceCount": sources.len(),
                 "sources": sources,
             }).to_string()))
         }
         ("GET", "/api/discovery/sources/by-filename") => {
             let pattern = query_parameter(route.query, "pattern")
                 .unwrap_or_default()
                 .trim()
                 .to_owned();
             if pattern.is_empty() {
                 return Ok(routing::bad_request_response("pattern query parameter is required"));
             }
             let limit = match query_bounded_usize(route.query, "limit", 1, 1_000) {
                 Ok(limit) => limit.unwrap_or(100),
                 Err(()) => return Ok(routing::bad_request_response("limit must be greater than zero")),
             };
             let needle = pattern.to_ascii_lowercase();
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches)
                 .into_iter()
                 .filter(|source| source["filename"].as_str().is_some_and(|filename| filename.to_ascii_lowercase().contains(&needle)))
                 .take(limit)
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "pattern": pattern,
                 "sourceCount": sources.len(),
                 "sources": sources,
             }).to_string()))
         }
         ("GET", "/api/discovery/summaries") => {
             let min_sources = match query_bounded_usize(route.query, "minSources", 1, 1_000) {
                 Ok(minimum) => minimum.unwrap_or(2),
                 Err(()) => return Ok(routing::bad_request_response("minSources must be greater than zero")),
             };
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let mut grouped = BTreeMap::<u64, (HashSet<String>, String)>::new();
             for source in source_discovery_sources(&discovery, &searches) {
                 let size = source["size"].as_u64().unwrap_or(0);
                 let username = source["username"].as_str().unwrap_or_default().to_ascii_lowercase();
                 let filename = source["filename"].as_str().unwrap_or_default().to_owned();
                 let entry = grouped.entry(size).or_insert_with(|| (HashSet::new(), filename));
                 entry.0.insert(username);
             }
             let summaries = grouped
                 .into_iter()
                 .filter(|(_, (users, _))| users.len() >= min_sources)
                 .map(|(size, (users, filename))| serde_json::json!({
                     "size": size,
                     "sourceCount": users.len(),
                     "sampleFilename": filename,
                 }))
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "minSources": min_sources,
                 "count": summaries.len(),
                 "summaries": summaries,
             }).to_string()))
         }
         ("GET", "/api/discovery/no-partial-count") => Ok(routing::ok_response(
             serde_json::json!({
                 "usersWithoutPartialSupport": 0,
                 "message": "0 users are flagged as not supporting partial/chunked downloads",
             }).to_string(),
         )),
         ("POST", "/api/discovery/reset-partial-flags") => Ok(routing::ok_response(
             serde_json::json!({
                 "message": "Reset partial support flags for 0 users. They will be tried again on next swarm.",
             }).to_string(),
         )),

         ("GET", "/api/source-feeds") => {
             let wishlist = state.wishlist.read().await;
             let items = wishlist
                 .records
                 .iter()
                 .flat_map(|record| record.items.iter())
                 .map(|item| {
                     let item_json = serde_json::from_str::<serde_json::Value>(&item.json())
                         .unwrap_or_else(|_| serde_json::json!({ "id": item.id }));
                     serde_json::json!({
                         "id": format!("wishlist-{}", item.id),
                         "name": item.search_text(),
                         "provider": "wishlist",
                         "enabled": true,
                         "items": [item_json],
                     })
                 })
                 .collect::<Vec<_>>();
             let count = items.len();
             drop(wishlist);
             Ok(routing::ok_response(serde_json::json!({
                 "feeds": items,
                 "count": count,
             }).to_string()))
         }

         ("POST", "/api/source-feeds") => {
             let name = extract_json_string_field(body, "name")
                 .or_else(|| extract_json_string_field(body, "title"))
                 .unwrap_or_else(|| "source feed".to_owned());
             let raw = extract_json_string_field(body, "text")
                 .or_else(|| extract_json_string_field(body, "content"))
                 .or_else(|| extract_json_string_field(body, "playlist"))
                 .unwrap_or_default();
             let parsed_items = raw
                 .lines()
                 .map(str::trim)
                 .filter(|line| !line.is_empty())
                 .map(|line| {
                     let (artist, title) = line
                         .split_once(" - ")
                         .map(|(artist, title)| (artist.trim().to_owned(), title.trim().to_owned()))
                         .unwrap_or_else(|| (String::new(), line.to_owned()));
                     (artist, title, "SourceFeed".to_owned())
             })
             .collect::<Vec<_>>();
             let mut wishlist = state.wishlist.write().await;
             let previous = wishlist.clone();
             if !wishlist.can_add_items(parsed_items.len()) {
                 return Ok(routing::service_unavailable_response("wishlist item capacity is full"));
             }
             let mut items = Vec::new();
             let mut persisted_items = Vec::new();
             for (artist, title, kind) in parsed_items {
                 let item = wishlist
                     .add_item(artist, title, kind)
                     .map_err(|_| "wishlist capacity changed unexpectedly".to_owned())?;
                 let value = serde_json::from_str::<serde_json::Value>(&item.json())
                     .unwrap_or_else(|_| serde_json::json!({ "id": item.id }));
                 persisted_items.push(item);
                 items.push(value);
             }
             let count = items.len();
             let mutated = wishlist.clone();
             drop(wishlist);
             if let Err(error) = persist_wishlist_items_checked(state, &persisted_items).await {
                 rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                 return Ok(routing::service_unavailable_response(&error));
             }
             Ok(routing::created_response(serde_json::json!({
                 "id": format!("source-feed-{}", unix_timestamp()),
                 "name": name,
                 "enabled": true,
                 "items": items,
                 "count": count,
                 "provider": "manual",
                 "persisted": true,
             }).to_string()))
         }

         ("GET", "/api/songid/runs") => {
             // Matches the oracle's real ListRuns(limit=10): newest-first,
             // bounded by the real `limit` query param -- not the full,
             // unbounded, oldest-first storage order.
             let limit = route
                 .query
                 .map(query_params)
                 .unwrap_or_default()
                 .into_iter()
                 .find(|(key, _)| key == "limit")
                 .and_then(|(_, value)| value.parse::<i64>().ok())
                 .filter(|limit| *limit > 0)
                 .unwrap_or(10) as usize;
             let runtime = state.runtime.read().await;
             let mut runs = runtime.songid_run_records.clone();
             drop(runtime);
             runs.reverse();
             runs.truncate(limit);
             Ok(routing::ok_response(serde_json::Value::Array(runs).to_string()))
         }

         ("GET", "/api/songid/runs/queue") => {
             // Matches the oracle's real GetQueueSummary(activeLimit):
             // counts are windowed over the most recent
             // max(activeLimit, 100) runs (newest-first), and activeRuns
             // is bounded by the real activeLimit query param -- not the
             // entire unbounded, unordered history.
             let active_limit = route
                 .query
                 .map(query_params)
                 .unwrap_or_default()
                 .into_iter()
                 .find(|(key, _)| key == "activeLimit")
                 .and_then(|(_, value)| value.parse::<i64>().ok())
                 .filter(|limit| *limit > 0)
                 .unwrap_or(25) as usize;
             let max_concurrent_runs = state
                 .media_services
                 .read()
                 .await
                 .song_id_max_concurrent_runs;
             let runtime = state.runtime.read().await;
             let mut recent_runs = runtime.songid_run_records.clone();
             drop(runtime);
             recent_runs.reverse();
             recent_runs.truncate(active_limit.max(100));
             let queued = recent_runs.iter().filter(|record| record["status"] == "queued").count();
             let running = recent_runs.iter().filter(|record| record["status"] == "running").count();
             let completed = recent_runs.iter().filter(|record| record["status"] == "completed").count();
             let failed = recent_runs.iter().filter(|record| record["status"] == "failed").count();
             let active_runs = recent_runs
                 .iter()
                 .filter(|record| matches!(record["status"].as_str(), Some("queued" | "running")))
                 .take(active_limit)
                 .cloned()
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "queuedCount": queued,
                 "runningCount": running,
                 "completedCount": completed,
                 "failedCount": failed,
                 "maxConcurrentRuns": max_concurrent_runs,
                 "activeRuns": active_runs,
             }).to_string()))
         }

         ("POST", "/api/songid/runs") => {
             // Matches the oracle's SongIdController.CreateRun: a request
             // with no real source is rejected before ever consuming a
             // concurrency slot, not silently accepted as an empty-source
             // run.
             let source = extract_json_string_field(body, "source")
                 .unwrap_or_default()
                 .trim()
                 .to_owned();
             if source.is_empty() {
                 return Ok(routing::bad_request_response("SongID source is required."));
             }
             let source_type = songid_source_type(&source);
             if source_type == "local_file"
                 && !songid_local_file_is_allowed(&state.config, &source)
             {
                 return Ok(routing::bad_request_response(
                     "SongID analysis could not be queued.",
                 ));
             }
             let Ok(_songid_permit) = Arc::clone(&state.songid_run_slots).try_acquire_owned()
             else {
                 return Ok(routing::service_unavailable_response(
                     "SongID run concurrency limit reached",
                 ));
             };
             let integrations = state.integration_settings.read().await.clone();
             let (fallback_query, metadata, evidence, full_source_fingerprint, acoustid_finding) =
                 songid_source_analysis(&source, source_type, &integrations).await;
             let query = extract_json_string_field(body, "query")
                 .filter(|query| !query.trim().is_empty())
                 .unwrap_or(fallback_query);
             let spotify_metadata_loaded = metadata
                 .pointer("/extra/analysisAudioSource")
                 .and_then(serde_json::Value::as_str)
                 == Some("spotify_page");
             let library = state.library.read().await;
             let shares = state.shares.read().await;
             let runs = songid_runs_value(&library, &shares, None);
             let matches = runs
                 .iter()
                 .flat_map(|run| run.get("matches").and_then(serde_json::Value::as_array).cloned().unwrap_or_default())
                 .collect::<Vec<_>>();
             let library_items = library.records.len();
             let shared_files = shares.entries.len();
             drop(shares);
             drop(library);
             // Unlike the oracle's real async queue+worker pipeline
             // (SongIdService.QueueAnalyzeAsync enqueues a "queued" run
             // that a background worker progresses through "running" to
             // "completed" over real time), slskR analyzes synchronously
             // right here -- record_songid_run has already computed the
             // real, final status by the time this response is built. It
             // must not be overwritten with a fake "queued" placeholder
             // that nothing would ever advance past, permanently hiding
             // the real (already-available) result from every later poll.
             let run = match mutate_runtime_compat_state(state, |runtime, _| {
                 let mut run = runtime.record_songid_run(matches, library_items, shared_files)?;
                 run["source"] = serde_json::json!(source);
                 run["sourceType"] = serde_json::json!(source_type);
                 run["query"] = serde_json::json!(query);
                 run["summary"] = serde_json::json!(match source_type {
                     "local_file" if full_source_fingerprint.is_some() && acoustid_finding.is_some() => {
                         "Analyzed local file with Chromaprint and AcoustID metadata."
                     }
                     "local_file" if full_source_fingerprint.is_some() => {
                         "Analyzed local file with Chromaprint and filename metadata."
                     }
                     "local_file" => "Analyzed local file with filename fallback metadata.",
                     "youtube_url" => {
                         "Classified YouTube URL; optional metadata tools may enrich the run."
                     }
                     "spotify_url" if spotify_metadata_loaded => {
                         "Analyzed Spotify page metadata for SongID query generation."
                     }
                     "spotify_url" => "Spotify metadata fetch failed; using source query fallback.",
                     "url" => "Classified URL; optional source metadata may enrich the run.",
                     _ => "Using free-text SongID query.",
                 });
                 run["evidence"] = serde_json::Value::Array(
                     evidence.iter().cloned().map(serde_json::Value::String).collect(),
                 );
                 run["metadata"] = metadata.clone();
                 if let Some(fingerprint) = full_source_fingerprint.clone() {
                     run["fullSourceFingerprint"] = fingerprint;
                 }
                 if let Some(finding) = acoustid_finding.clone() {
                     run["clips"] = serde_json::json!([{
                         "clipId": "full-source",
                         "acoustId": finding,
                     }]);
                     run["scorecard"] = serde_json::json!({
                         "acoustIdHitCount": 1,
                         "rawAcoustIdHitCount": 1,
                     });
                 }
                 if let Some(stored) = runtime.songid_run_records.last_mut() {
                     *stored = run.clone();
                 }
                 Some(run)
             }).await {
                 Ok(Some(run)) => run,
                 Ok(None) => {
                     return Ok(routing::service_unavailable_response("song id run space exhausted"));
                 }
                 Err(error) => return Ok(routing::service_unavailable_response(&error)),
             };
             Ok(routing::accepted_response(run.to_string()))
         }

         ("GET", path) if path.starts_with("/api/songid/runs/") && path.contains("/evidence-package") => {
             let Some(run_id) =
                 path_segment_between(path, "/api/songid/runs/", "/evidence-package")
             else {
                 return Ok(routing::not_found_response());
             };
             let runtime = state.runtime.read().await;
             let run = runtime.songid_run(run_id);
             drop(runtime);
             Ok(run
                 .map(|run| routing::ok_response(songid_evidence_package_json(&run).to_string()))
                 .unwrap_or_else(routing::not_found_response))
         }

         ("GET", path)
             if path.starts_with("/api/songid/runs/")
                 && !path.contains("/forensic-matrix")
                 && !path.contains("/evidence-package") =>
         {
             let Some(run_id) = path_segment_after(path, "/api/songid/runs/") else {
                 return Ok(routing::not_found_response());
             };
             let runtime = state.runtime.read().await;
             let run = runtime.songid_run(run_id);
             drop(runtime);
             Ok(run
                 .map(|run| routing::ok_response(run.to_string()))
                 .unwrap_or_else(routing::not_found_response))
         }

         ("GET", path) if path.starts_with("/api/songid/runs/") && path.contains("/forensic-matrix") => {
             let Some(run_id) =
                 path_segment_between(path, "/api/songid/runs/", "/forensic-matrix")
             else {
                 return Ok(routing::not_found_response());
             };
             let runtime = state.runtime.read().await;
            let Some(run) = runtime.songid_run(run_id) else {
                return Ok(routing::not_found_response());
            };
            drop(runtime);
            let matrix = run
                .get("forensicMatrix")
                .or_else(|| run.get("matches"))
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|match_value| {
                    serde_json::json!({
                        "libraryItemId": match_value.get("libraryItemId").cloned().unwrap_or(serde_json::Value::Null),
                        "filename": match_value.get("filename").cloned().unwrap_or(serde_json::Value::Null),
                        "score": match_value.get("score").or_else(|| match_value.get("identityScore")).cloned().unwrap_or_else(|| serde_json::json!(0.0)),
                        "signals": match_value.get("signals").cloned().unwrap_or_else(|| serde_json::json!([])),
                    })
                })
                 .collect::<Vec<_>>();
             let count = matrix.len();
             Ok(routing::ok_response(serde_json::json!({
                 "run_id": run_id,
                 "matrix": matrix,
                 "count": count,
             }).to_string()))
         }

         ("GET", path) if path.starts_with("/api/soulseek/users/") && path.contains("/interests") && path.len() > 20 => {
             let username = path.split('/').nth(4).unwrap_or("unknown");
             let json = format!(
                 "{{\"username\":\"{}\",\"interests\":[],\"count\":0}}",
                 json_escape(username)
             );
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/swarm/analytics/dashboard") => {
             let time_window_hours = match query_bounded_usize(
                 route.query,
                 "timeWindowHours",
                 1,
                 168,
             ) {
                 Ok(value) => value.unwrap_or(24) as u64,
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Time window must be between 1 and 168 hours (7 days)",
                     ));
                 }
             };
             let ranking_limit = match query_bounded_usize(route.query, "rankingLimit", 1, 100) {
                 Ok(value) => value.unwrap_or(20),
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Ranking limit must be between 1 and 100",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, time_window_hours, ranking_limit);
             drop(swarms);
             Ok(routing::ok_response(dashboard.to_string()))
         }

         ("GET", "/api/swarm/analytics/performance") => {
             let time_window_hours = match query_bounded_usize(
                 route.query,
                 "timeWindowHours",
                 1,
                 168,
             ) {
                 Ok(value) => value.unwrap_or(24) as u64,
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Time window must be between 1 and 168 hours (7 days)",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, time_window_hours, 20);
             drop(swarms);
             Ok(routing::ok_response(
                 dashboard["performanceMetrics"].to_string(),
             ))
         }

         ("GET", "/api/swarm/analytics/peers/rankings") => {
             let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                 Ok(value) => value.unwrap_or(20),
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Limit must be between 1 and 100",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, 24, limit);
             drop(swarms);
             Ok(routing::ok_response(dashboard["peerRankings"].to_string()))
         }

         ("GET", "/api/swarm/analytics/efficiency") => {
             let time_window_hours = match query_bounded_usize(
                 route.query,
                 "timeWindowHours",
                 1,
                 168,
             ) {
                 Ok(value) => value.unwrap_or(24) as u64,
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Time window must be between 1 and 168 hours (7 days)",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, time_window_hours, 100);
             drop(swarms);
             Ok(routing::ok_response(
                 dashboard["efficiencyMetrics"].to_string(),
             ))
         }

         ("GET", "/api/swarm/analytics/trends") => {
             if query_bounded_usize(route.query, "timeWindowHours", 1, 168).is_err() {
                 return Ok(routing::bad_request_response(
                     "Time window must be between 1 and 168 hours (7 days)",
                 ));
             }
             if query_bounded_usize(route.query, "dataPoints", 2, 168).is_err() {
                 return Ok(routing::bad_request_response(
                     "Data points must be between 2 and 168",
                 ));
             }
             Ok(routing::ok_response(
                 serde_json::json!({
                     "timePoints": [],
                     "successRates": [],
                     "averageSpeeds": [],
                     "averageDurations": [],
                     "averageSourcesUsed": [],
                     "downloadCounts": [],
                 })
                 .to_string(),
             ))
         }

         ("GET", "/api/swarm/analytics/recommendations") => {
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, 24, 10);
             drop(swarms);
             Ok(routing::ok_response(
                 dashboard["recommendations"].to_string(),
             ))
         }

        ("GET", "/api/telemetry/metrics") => {
            let transfers = state.transfers.read().await;
            let transfer_count = transfers.entries.len();
            drop(transfers);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: format!(
                    "# HELP slskr_telemetry_transfers Transfer count\n\
                     # TYPE slskr_telemetry_transfers gauge\n\
                     slskr_telemetry_transfers {}\n",
                    transfer_count
                ),
            })
        }

          // Matches the oracle exactly: TelemetryController.GetKpis and
          // MetricsController.GetKpis are different controllers mounted at
          // different routes, but both call the same
          // Telemetry.Prometheus.GetMetricsAsObject(include: KpiRegexes)
          // with an identical regex list, so they return identical
          // content. slskR's /api/telemetry/prometheus/kpis was already
          // fixed to the real dictionary-of-PrometheusMetric shape; this
          // sibling route reuses the exact same real data instead of its
          // own invented {kpis:[...], count} array.
          ("GET", "/api/telemetry/metrics/kpi") | ("GET", "/api/telemetry/metrics/kpis") => {
              let transfers = state.transfers.read().await;
              let searches = state.searches.read().await;
              let metrics = serde_json::json!({
                  "slskr_transfers": prometheus_metric_json("slskr_transfers", "gauge", transfers.entries.len() as f64),
                  "slskr_searches": prometheus_metric_json("slskr_searches", "gauge", searches.records.len() as f64),
              });
              drop(transfers);
              drop(searches);
              Ok(routing::ok_response(metrics.to_string()))
          }

          // ADDITIONAL MISSING GET ENDPOINTS (Phase 6)
          ("GET", "/api/multisource/jobs") => {
            let versioned_profile = route.path.starts_with("/api/v0/")
                && state.config.controller_profile
                    == ControllerProfile::Native;
            let swarm = state.multisource.read().await;
            let mut jobs = if versioned_profile {
                  swarm
                      .list()
                      .into_iter()
                      .map(|job| {
                          serde_json::json!({
                              "jobId": job.id,
                              "state": job.status,
                              "totalChunks": job.total_chunks,
                              "completedChunks": job.completed_chunks,
                              "percentComplete": if job.total_chunks > 0 {
                                  job.completed_chunks as f64 * 100.0 / job.total_chunks as f64
                              } else {
                                  0.0
                              },
                              "activeWorkers": 0,
                              "chunksPerSecond": 0.0,
                          })
                      })
                      .collect::<Vec<_>>()
              } else {
                  swarm
                      .list()
                      .into_iter()
                      .filter_map(|job| serde_json::to_value(job).ok())
                      .collect::<Vec<_>>()
              };
              drop(swarm);
            if !versioned_profile {
                  let transfers = state.transfers.read().await;
                  jobs.extend(transfers
                      .entries
                      .iter()
                      .filter(|entry| entry.direction == 0)
                      .map(|entry| {
                          let size = entry.size.unwrap_or(0);
                          let progress = if size == 0 {
                              0.0
                          } else {
                              (entry.bytes_transferred as f64 / size as f64) * 100.0
                          };
                          serde_json::json!({
                              "id": format!("transfer-{}", entry.id),
                              "status": entry.status,
                              "filename": entry.filename,
                              "sources": entry.peer_username.as_deref().map(|peer| vec![peer]).unwrap_or_default(),
                              "progress": progress,
                              "bytesTransferred": entry.bytes_transferred,
                              "size": size,
                              "updated_at": entry.updated_at,
                          })
                      }));
                  drop(transfers);
              }
              let count = jobs.len();
              let json = serde_json::json!({
                  "jobs": jobs,
                  "count": count,
              }).to_string();
              Ok(routing::ok_response(json))
          }

          ("GET", path) if pod_channel_messages_path(path).is_some() => {
              let (pod_id, channel_id) = pod_channel_messages_path(path).unwrap_or_default();
              if pods::is_gold_star_club(&pod_id) && !gold_star_club_available(state) {
                  return Ok(routing::not_found_response());
              }
              let since = match query_millis_parameter(route.query, "since") {
                  Ok(value) => value,
                  Err(error) => return Ok(routing::bad_request_response(&error)),
              };
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if pods.get(&pod_id).is_none() || !pods.channel_exists(&pod_id, &channel_id) {
                  return Ok(routing::ok_response("[]".to_owned()));
              }
              if peer_id
                  .as_deref()
                  .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              if let Err(error) = state.pod_channels.read().await.validate_storage() {
                  eprintln!("pod channel message storage failed: {error}");
                  return Ok(routing::internal_server_error_response(
                      "Failed to get messages",
                  ));
              }
              let binding = pods.soulseek_binding(&pod_id, &channel_id);
              drop(pods);
              if let Some(binding) = binding.filter(|binding| binding.kind == "dm") {
                  let local_peer_id = peer_id.unwrap_or_default();
                  let messages = state.messages.read().await;
                  let projected = messages
                      .records
                      .iter()
                      .filter(|message| {
                          message.username.eq_ignore_ascii_case(&binding.identifier)
                              && since.is_none_or(|since| message.created_at_ms > since)
                      })
                      .map(|message| pod_channels::PodChannelMessage {
                          message_id: message.id.to_string(),
                          pod_id: pod_id.clone(),
                          channel_id: channel_id.clone(),
                          sender_peer_id: if message.direction == "inbound" {
                              format!("bridge:{}", message.username)
                          } else {
                              local_peer_id.clone()
                          },
                          body: message.body.clone(),
                          timestamp_unix_ms: message.created_at_ms,
                          signature: String::new(),
                          sig_version: 1,
                      })
                      .collect::<Vec<_>>();
                  return Ok(routing::ok_response(
                      serde_json::to_string(&projected)
                          .map_err(|error| format!("pod message serialization failed: {error}"))?,
                  ));
              }
              let channels = state.pod_channels.read().await;
              let messages = channels.list(&pod_id, &channel_id, since);
              drop(channels);
              Ok(routing::ok_response(
                  serde_json::to_string(&messages)
                      .map_err(|error| format!("pod message serialization failed: {error}"))?,
              ))
          }

          ("POST", path) if pod_channel_messages_path(path).is_some() => {
              let (pod_id, channel_id) = pod_channel_messages_path(path).unwrap_or_default();
              let body_text = extract_json_string_field(body, "body")
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let sender_peer_id = extract_json_string_field(body, "senderPeerId")
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              if body_text.is_empty() {
                  return Ok(routing::bad_request_response("Message body is required"));
              }
              if sender_peer_id.is_empty() {
                  return Ok(routing::bad_request_response("SenderPeerId is required"));
              }
              let authenticated_peer_id = pod_request_peer_id(state).await;
              let Some(authenticated_peer_id) = authenticated_peer_id else {
                  return Ok(routing::forbidden_response(
                      "Authenticated peer identity is required",
                  ));
              };
              if sender_peer_id != authenticated_peer_id {
                  return Ok(routing::forbidden_response(
                      "SenderPeerId must match the authenticated peer identity",
                  ));
              }
              let signature = extract_json_string_field(body, "signature")
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let signature_mode = state
                  .advanced_networking
                  .read()
                  .await
                  .pod_security_signature_mode;
              if signature_mode == PodSignatureMode::Enforce && signature.is_empty() {
                  return Ok(routing::bad_request_response(
                      "Message signature is required when PodCore.Security.SignatureMode is Enforce",
                  ));
              }
              if signature_mode == PodSignatureMode::Warn && signature.is_empty() {
                  record_daemon_log(
                      state,
                      logging::LogLevel::Warn,
                      "podcore",
                      "accepted unsigned pod message in warn mode".to_owned(),
                  )
                  .await;
              }
              let pods = state.pods.read().await;
              if pods.get(&pod_id).is_none() || !pods.channel_exists(&pod_id, &channel_id) {
                  return Ok(routing::not_found_response());
              }
              if !pods.is_member(&pod_id, &authenticated_peer_id) {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              let binding = pods.soulseek_binding(&pod_id, &channel_id);
              drop(pods);
              if let Some(binding) = binding.as_ref().filter(|binding| binding.kind == "dm") {
                  let session_command_permit = match state.session_commands.reserve().await {
                      Ok(permit) => permit,
                      Err(_) => {
                          return Ok(routing::service_unavailable_response(
                              "session manager is not running",
                          ));
                      }
                  };
                  let mut messages = state.messages.write().await;
                  let previous = messages.clone();
                  let record = messages.add(
                      binding.identifier.clone(),
                      "outbound",
                      body_text.clone(),
                  );
                  let mutated = messages.clone();
                  drop(messages);
                  if let Err(error) = persist_message_record_checked(state, &record).await {
                      rollback_messages_if_unchanged(state, previous, &mutated).await;
                      return Ok(routing::service_unavailable_response(&error));
                  }
                  session_command_permit.send(SessionCommand::MessageUser {
                      username: binding.identifier.clone(),
                      body: body_text,
                  });
                  return Ok(routing::ok_response(
                      serde_json::json!({
                          "messageId": record.id.to_string(),
                          "sent": true,
                      })
                      .to_string(),
                  ));
              }
              let result = state.pod_channels.write().await.append(
                  pod_id,
                  channel_id,
                  authenticated_peer_id,
                  body_text,
                  signature,
                  unix_timestamp_millis(),
              );
              match result {
                  Ok(message) => {
                      if let Some(binding) = binding.filter(|binding| {
                          binding.kind == "room" && binding.mode == "mirror"
                      }) {
                          let _ = try_send_session_command(
                              state,
                              SessionCommand::SayRoom {
                                  room: binding.identifier,
                                  body: format!(
                                      "[Pod:{}] {}",
                                      message.sender_peer_id, message.body
                                  ),
                              },
                          );
                      }
                      Ok(routing::ok_response(
                          serde_json::json!({
                              "messageId": message.message_id,
                              "sent": true,
                          })
                          .to_string(),
                      ))
                  }
                  Err(error)
                      if error.contains("required") || error.contains("must be at most") =>
                  {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) => {
                      eprintln!("pod channel message persistence failed: {error}");
                      Ok(routing::internal_server_error_response("Failed to send message"))
                  }
              }
          }

          ("POST", "/api/pods") => {
              let value = match serde_json::from_str::<serde_json::Value>(body) {
                  Ok(value) => value,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              let Some(pod_value) = value
                  .get("pod")
                  .filter(|pod| !pod.is_null())
                  .cloned()
              else {
                  return Ok(routing::bad_request_response("Pod data is required"));
              };
              let pod = match serde_json::from_value::<pods::PodRecord>(pod_value) {
                  Ok(pod) => pod,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              let Some(creator) = pod_request_peer_id(state).await else {
                  return Ok(routing::forbidden_response("Authenticated peer identity is required"));
              };
              match state.pods.write().await.create(pod, creator) {
                  Ok(pod) => Ok(routing::created_response(
                      serde_json::to_string(&pod)
                          .map_err(|error| format!("pod serialization failed: {error}"))?,
                  )),
                  Err(error) if error == "Pod already exists" => {
                      Ok(routing::conflict_response(&error))
                  }
                  Err(error) if error.contains("capacity is full") => {
                      Ok(routing::conflict_response(&error))
                  }
                  Err(error) if error.starts_with("pod state write failed") => {
                      eprintln!("pod persistence failed: {error}");
                      Ok(routing::internal_server_error_response("Failed to create pod"))
                  }
                  Err(error) => Ok(routing::bad_request_response(&error)),
              }
          }

          ("GET", path)
              if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response("Failed to get pod"));
              }
              if pods.get(&pod_id).is_some()
                  && !pods.is_public(&pod_id)
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              Ok(pods
                  .get(&pod_id)
                  .map(|pod| {
                      routing::ok_response(
                          serde_json::to_string(&pod).unwrap_or_else(|_| "{}".to_owned()),
                      )
                  })
                  .unwrap_or_else(routing::not_found_response))
          }

          ("PUT", path)
              if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let value = match serde_json::from_str::<serde_json::Value>(body) {
                  Ok(value) => value,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              let Some(pod_value) = value
                  .get("pod")
                  .filter(|pod| !pod.is_null())
                  .cloned()
              else {
                  return Ok(routing::bad_request_response("Pod data is required"));
              };
              let pod = match serde_json::from_value::<pods::PodRecord>(pod_value) {
                  Ok(pod) => pod,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              if pod.pod_id != pod_id {
                  return Ok(routing::bad_request_response(
                      "PodId in URL must match PodId in body",
                  ));
              }
              let peer_id = pod_request_peer_id(state).await;
              let mut pods = state.pods.write().await;
              if pods.get(&pod_id).is_some()
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              if let Some(gateway_peer_id) = pods.gateway_peer_for_update(&pod_id, &pod) {
                  if peer_id.as_deref() != Some(gateway_peer_id.as_str()) {
                      return Ok(routing::forbidden_response(
                          "Only the designated gateway peer can modify private service policy",
                      ));
                  }
              }
              let proposed_channel_ids = pod
                  .channels
                  .iter()
                  .map(|channel| channel.channel_id.trim().to_owned())
                  .collect::<HashSet<_>>();
              let removed_channel_ids = pods
                  .get(&pod_id)
                  .map(|existing| {
                      existing
                          .channels
                          .into_iter()
                          .map(|channel| channel.channel_id)
                          .filter(|channel_id| !proposed_channel_ids.contains(channel_id))
                          .collect::<HashSet<_>>()
                  })
                  .unwrap_or_default();
              let mut channels = state.pod_channels.write().await;
              let removed_messages = match channels.delete_channels(&pod_id, &removed_channel_ids) {
                  Ok(messages) => messages,
                  Err(error) => {
                      eprintln!("pod channel cleanup failed: {error}");
                      return Ok(routing::internal_server_error_response(
                          "Failed to update pod",
                      ));
                  }
              };
              let update_result = pods.update(&pod_id, pod);
              match update_result {
                  Ok(Some(pod)) => Ok(routing::ok_response(
                      serde_json::to_string(&pod)
                          .map_err(|error| format!("pod serialization failed: {error}"))?,
                  )),
                  Ok(None) => {
                      if let Err(error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {error}");
                          return Ok(routing::internal_server_error_response(
                              "Failed to update pod",
                          ));
                      }
                      Ok(routing::not_found_response())
                  }
                  Err(error) => {
                      if let Err(rollback_error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {rollback_error}");
                          return Ok(routing::internal_server_error_response(
                              "Failed to update pod",
                          ));
                      }
                      if error.starts_with("pod state write failed") {
                          eprintln!("pod persistence failed: {error}");
                          Ok(routing::internal_server_error_response("Failed to update pod"))
                      } else {
                          Ok(routing::bad_request_response(&error))
                    }
                }
            }
        }

          ("DELETE", path)
              if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response("Failed to delete pod"));
              }
              if pods.get(&pod_id).is_some()
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              drop(pods);
              let mut pods = state.pods.write().await;
              if pods.get(&pod_id).is_none() {
                  return Ok(routing::not_found_response());
              }
              let mut channels = state.pod_channels.write().await;
              let removed_messages = match channels.delete_pod(&pod_id) {
                  Ok(messages) => messages,
                  Err(error) => {
                      eprintln!("pod channel cleanup failed: {error}");
                      return Ok(routing::internal_server_error_response(
                          "Failed to delete pod",
                      ));
                  }
              };
              match pods.delete(&pod_id) {
                  Ok(true) => Ok(routing::no_content_response()),
                  Ok(false) => {
                      if let Err(rollback_error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {rollback_error}");
                      }
                      Ok(routing::not_found_response())
                  }
                  Err(error) => {
                      if let Err(rollback_error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {rollback_error}");
                      }
                      eprintln!("pod persistence failed: {error}");
                      Ok(routing::internal_server_error_response("Failed to delete pod"))
                  }
              }
          }

          ("GET", path)
              if pod_resource_segments(path)
                  .is_some_and(|segments| segments.len() == 2 && segments[1] == "members") =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response(
                      "Failed to get pod members",
                  ));
              }
              if pods.get(&pod_id).is_some()
                  && !pods.is_public(&pod_id)
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              Ok(pods
                  .members(&pod_id)
                  .map(|members| {
                      routing::ok_response(
                          serde_json::to_string(&members).unwrap_or_else(|_| "[]".to_owned()),
                      )
                  })
                  .unwrap_or_else(routing::not_found_response))
          }

          ("POST", path)
              if pod_resource_segments(path).is_some_and(|segments| {
                  segments.len() == 2 && matches!(segments[1].as_str(), "join" | "leave" | "ban")
              }) =>
          {
              let segments = pod_resource_segments(path).unwrap_or_default();
              let pod_id = &segments[0];
              let action = &segments[1];
              if pods::is_gold_star_club(pod_id) && !gold_star_club_available(state) {
                  return Ok(routing::not_found_response());
              }
              let peer_id = if action == "ban" {
                  extract_json_string_field(body, "peerId")
                      .unwrap_or_default()
                      .trim()
                      .to_owned()
              } else {
                  pod_request_peer_id(state).await.unwrap_or_default()
              };
              if peer_id.is_empty() {
                  return Ok(routing::bad_request_response("PeerId is required"));
              }
              let moderator = if action == "ban" {
                  pod_request_peer_id(state).await
              } else {
                  None
              };
              let mut pods = state.pods.write().await;
              if action == "ban"
                  && moderator
                      .as_deref()
                      .is_none_or(|moderator| !pods.can_moderate(pod_id, moderator))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              let result = match action.as_str() {
                  "join" => pods.join(pod_id, peer_id.clone()),
                  "leave" => pods.leave(pod_id, &peer_id),
                  _ => pods.ban(pod_id, &peer_id),
              };
              match result {
                  Ok(Some(true)) => {
                      if action == "leave" && pods::is_gold_star_club(pod_id) {
                          if let Err(error) = pods::record_gold_star_club_revocation(
                              &state.config.state_dir,
                              &peer_id,
                          ) {
                              eprintln!("Gold Star Club revocation persistence failed: {error}");
                              return Ok(routing::service_unavailable_response(
                                  "pod revocation storage is unavailable",
                              ));
                          }
                      }
                      let response_key = match action.as_str() {
                          "join" => "joined",
                          "leave" => "left",
                          _ => "banned",
                      };
                      Ok(routing::ok_response(
                          serde_json::json!({ (response_key): true }).to_string(),
                      ))
                  }
                  Ok(Some(false)) if action == "join" => Ok(routing::bad_request_response(
                      "Failed to join pod (may already be a member)",
                  )),
                  Ok(Some(false)) => Ok(routing::not_found_response()),
                  Ok(None) => Ok(routing::not_found_response()),
                  Err(error)
                      if error.contains("capacity")
                          || error.contains("banned")
                          || error.contains("approval")
                          || error.contains("last Pod moderator") =>
                  {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) if error.contains("required") || error.contains("at most") => {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) => {
                      eprintln!("pod persistence failed: {error}");
                      let message = match action.as_str() {
                          "join" => "Failed to join pod",
                          "leave" => "Failed to leave pod",
                          _ => "Failed to ban member",
                      };
                      Ok(routing::internal_server_error_response(message))
                  }
              }
          }

          ("POST", path)
              if pod_resource_segments(path).is_some_and(|segments| {
                  segments.len() == 4
                      && segments[1] == "channels"
                      && matches!(segments[3].as_str(), "bind" | "unbind")
              }) =>
          {
              let segments = pod_resource_segments(path).unwrap_or_default();
              let pod_id = &segments[0];
              let channel_id = &segments[2];
              let action = &segments[3];
              if action == "bind" {
                  let mode = extract_json_string_field(body, "mode")
                      .unwrap_or_else(|| "readonly".to_owned())
                      .trim()
                      .to_ascii_lowercase();
                  if !matches!(mode.as_str(), "readonly" | "mirror") {
                      return Ok(routing::bad_request_response(
                          "Mode must be 'readonly' or 'mirror'",
                      ));
                  }
              } else if state
                  .pods
                  .read()
                  .await
                  .soulseek_binding(pod_id, channel_id)
                  .is_none()
              {
                  return Ok(routing::not_found_response());
              }
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              let can_moderate = peer_id
                  .as_deref()
                  .is_some_and(|peer_id| pods.can_moderate(pod_id, peer_id));
              drop(pods);
              if !can_moderate {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              let result = if action == "bind" {
                  let room_name = extract_json_string_field(body, "roomName")
                      .unwrap_or_default()
                      .trim()
                      .to_owned();
                  let mode = extract_json_string_field(body, "mode")
                      .unwrap_or_else(|| "readonly".to_owned())
                      .trim()
                      .to_ascii_lowercase();
                  state
                      .pods
                      .write()
                      .await
                      .bind_room(pod_id, channel_id, room_name, mode)
              } else {
                  state.pods.write().await.unbind_room(pod_id, channel_id)
              };
              match result {
                  Ok(Some(true)) => {
                      if action == "bind" {
                          if let Some(binding) = state
                              .pods
                              .read()
                              .await
                              .soulseek_binding(pod_id, channel_id)
                          {
                              let _ = try_send_session_command(
                                  state,
                                  SessionCommand::JoinRoom(binding.identifier),
                              );
                          }
                      }
                      let response_key = if action == "bind" { "bound" } else { "unbound" };
                      Ok(routing::ok_response(
                          serde_json::json!({ (response_key): true }).to_string(),
                      ))
                  }
                  Ok(Some(false)) | Ok(None) => Ok(routing::not_found_response()),
                  Err(error) if error.contains("required") || error.starts_with("Mode must") => {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) => {
                      eprintln!("pod binding persistence failed: {error}");
                      Ok(routing::internal_server_error_response(if action == "bind" {
                          "Failed to bind room"
                      } else {
                          "Failed to unbind room"
                      }))
                  }
              }
          }

          ("GET", "/api/pods") => {
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response("Failed to list pods"));
              }
              Ok(routing::ok_response(
                  serde_json::to_string(&pods.list_visible(peer_id.as_deref()))
                      .map_err(|error| format!("pod serialization failed: {error}"))?,
              ))
          }

          ("GET", "/api/solid/status") => {
              let media = state.media_services.read().await;
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": true,
                  "clientId": media
                      .solid
                      .client_id_url
                      .clone()
                      .unwrap_or_else(|| "/solid/clientid.jsonld".to_owned()),
                  "redirectPath": media.solid.redirect_path,
              }).to_string()))
          }

          ("GET", "/api/federation/diagnostics")
              if route.path == "/api/v0/federation/diagnostics" =>
          {
              Ok(federation_diagnostics_response(&state.config))
          }

          ("GET", "/api/federation/diagnostics") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let user_count = users.records.len();
              let watched_users = users.records.iter().filter(|user| user.watched).count();
              let mesh_capabilities = mesh.capability_records.len();
              let items = mesh
                  .capability_records
                  .iter()
                  .map(|record| {
                      serde_json::json!({
                          "username": record.username,
                          "issuedAt": record.issued_at_unix,
                          "expiresAt": record.expires_at_unix,
                          "features": record.features.clone(),
                          "endpoints": record.endpoints.clone(),
                          "source": "peer-capability",
                      })
                  })
                  .chain(users.records.iter().filter(|user| user.watched).map(|user| {
                      serde_json::json!({
                          "username": user.username,
                          "status": user.status,
                          "source": "watched-user",
                      })
                  }))
                  .collect::<Vec<_>>();
              let item_count = items.len();
              let checks = vec![
                  serde_json::json!({
                      "id": "watched-users",
                      "status": if users.records.is_empty() { "empty" } else { "ready" },
                      "count": users.records.len(),
                  }),
                  serde_json::json!({
                      "id": "mesh-capabilities",
                      "status": if mesh.capability_records.is_empty() { "empty" } else { "ready" },
                      "count": mesh.capability_records.len(),
                  }),
              ];
              let ready = checks
                  .iter()
                  .any(|check| check.get("status").and_then(serde_json::Value::as_str) == Some("ready"));
              drop(mesh);
              drop(users);
              Ok(routing::ok_response(serde_json::json!({
                  "federation": {"enabled": ready, "watchedUsers": watched_users},
                  "publishing": {"enabled": false},
                  "pods": {"enabled": true},
                  "mesh": {"capabilityRecords": mesh_capabilities},
                  "status": if ready { "ready" } else { "empty" },
                  "checks": checks,
                  "items": items,
                  "itemCount": item_count,
                  "counts": {
                      "users": user_count,
                      "watchedUsers": watched_users,
                      "meshCapabilities": mesh_capabilities,
                  },
                  "warnings": [],
                  "errors": [],
              }).to_string()))
          }

          ("GET", "/api/security/dashboard") => {
              let users = state.users.read().await;
              let webhooks = state.webhooks.read().await;
              let events = state.events.read().await;
              let security = state.security.read().await;
              let watched = users.records.iter().filter(|user| user.watched).count();
              let webhook_count = webhooks.get_all().len();
              let event_count = events.records.len();
              let ban_count = security.active_bans();
              let bans = security
                  .json_value()
                  .get("bans")
                  .cloned()
                  .unwrap_or_else(|| serde_json::json!([]));
              // Matches the oracle's real PeerReputation.GetStats(): real
              // per-peer scores and violation counts, not watch/online
              // status (which has nothing to do with reputation).
              let mut peer_keys = security
                  .reputation
                  .keys()
                  .chain(security.reputation_profiles.keys())
                  .cloned()
                  .collect::<Vec<_>>();
              peer_keys.sort_unstable();
              peer_keys.dedup();
              let total_peers = peer_keys.len();
              let mut total_score = 0_i64;
              let mut trusted_peers = 0;
              let mut untrusted_peers = 0;
              let mut total_successful_transfers = 0_u64;
              let mut total_failed_transfers = 0_u64;
              let mut total_protocol_violations = 0_u64;
              for username in &peer_keys {
                  let score = security
                      .reputation
                      .get(username)
                      .copied()
                      .unwrap_or(SECURITY_REPUTATION_DEFAULT_SCORE);
                  total_score += i64::from(score);
                  if score >= SECURITY_REPUTATION_TRUSTED_THRESHOLD {
                      trusted_peers += 1;
                  }
                  if score <= SECURITY_REPUTATION_UNTRUSTED_THRESHOLD {
                      untrusted_peers += 1;
                  }
                  if let Some(profile) = security.reputation_profiles.get(username) {
                      total_successful_transfers = total_successful_transfers
                          .saturating_add(profile.successful_transfers);
                      total_failed_transfers = total_failed_transfers
                          .saturating_add(profile.failed_transfers);
                      total_protocol_violations = total_protocol_violations
                          .saturating_add(profile.protocol_violations);
                  } else {
                      total_protocol_violations = total_protocol_violations.saturating_add(
                          u64::from(security.violations.get(username).copied().unwrap_or(0)),
                      );
                  }
              }
              let average_score = if total_peers > 0 {
                  total_score as f64 / total_peers as f64
              } else {
                  f64::from(SECURITY_REPUTATION_DEFAULT_SCORE)
              };
              let reputation_stats = serde_json::json!({
                  "totalPeers": total_peers,
                  "trustedPeers": trusted_peers,
                  "untrustedPeers": untrusted_peers,
                  "averageScore": average_score,
                  "totalSuccessfulTransfers": total_successful_transfers,
                  "totalFailedTransfers": total_failed_transfers,
                  "totalProtocolViolations": total_protocol_violations,
              });
              drop(security);
              drop(events);
              drop(webhooks);
              drop(users);
              Ok(routing::ok_response(serde_json::json!({
                  "eventStats": {"totalEvents": event_count},
                  "networkGuardStats": {"globalConnections": watched},
                  "violationStats": {"activeBans": ban_count},
                  "reputationStats": reputation_stats.clone(),
                  "paranoidStats": {"enabled": false},
                  "fingerprintStats": {"knownFingerprints": 0},
                  "entropyStats": {"checks": 0},
                  "consensusStats": {"decisions": 0},
                  "verificationStats": {"verified": 0},
                  "disclosureStats": {"disclosures": 0},
                  "temporalStats": {"events": event_count},
                  "enabled": true,
                  "status": "local",
                  "stats": {
                      "networkGuardStats": { "globalConnections": watched },
                      "reputationStats": reputation_stats,
                      "threatStats": { "activeThreats": untrusted_peers },
                      "banStats": { "activeBans": ban_count }
                  },
                  "events": event_count,
                  "webhooks": webhook_count,
                  "bans": bans
              }).to_string()))
          }

          ("GET", "/api/security/status") => {
              let users = state.users.read().await;
              let events = state.events.read().await;
              let security = state.security.read().await;
              let watched = users.records.iter().filter(|user| user.watched).count();
              let offline_watched = users
                  .records
                  .iter()
                  .filter(|user| user.watched && user.status.as_deref() == Some("offline"))
                  .count();
              let event_count = events.records.len();
              let ban_count = security.active_bans();
              drop(security);
              drop(events);
              drop(users);
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": true,
                  "status": "local",
                  "watchedPeers": watched,
                  "suspiciousPeers": offline_watched,
                  "activeBans": ban_count,
                  "events": event_count,
              }).to_string()))
          }

          ("GET", path) if path.starts_with("/api/security/") => {
              Ok(security_extended_response(path, route.query, state).await)
          }

          ("GET", "/api/soulseek/mesh-rendezvous/status") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let body = mesh.status_json(&users);
              drop(mesh);
              drop(users);
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/soulseek/mesh-rendezvous/users") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let body = mesh.users_json(&users);
              drop(mesh);
              drop(users);
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/soulseek/mesh-rendezvous/discover") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let candidates = if mesh.rendezvous.active_probe_enabled() {
                  mesh.candidate_usernames(&users)
                      .into_iter()
                      .take(MAX_ACTIVE_MESH_DISCOVERY_PROBES)
                      .collect::<Vec<_>>()
              } else {
                  Vec::new()
              };
              let body = mesh.discover_json(&users);
              drop(mesh);
              drop(users);
              for username in candidates {
                  if let Err(error) = send_session_command(
                      state,
                      SessionCommand::ProbePeerCapability(username),
                  )
                  .await
                  {
                      return Ok(routing::service_unavailable_response(&error));
                  }
              }
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/soulseek/peer-capabilities") => {
              let mesh = state.mesh.read().await;
              let body = serde_json::Value::Array(mesh.capability_records_json()).to_string();
              drop(mesh);
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/mesh/transport") if route.path.starts_with("/api/v0/") => {
              let dht_sessions = match state.dht.as_ref() {
                  Some(dht) => dht.peers().await.len(),
                  None => 0,
              };
              let overlay_sessions = match state.private_gateway.as_ref() {
                  Some(gateway) => gateway.active_connection_count().await,
                  None => 0,
              };
              Ok(routing::ok_response(serde_json::json!({
                  "dht": dht_sessions,
                  "overlay": overlay_sessions,
                  "natType": "Unknown",
              }).to_string()))
          }
          ("GET", "/api/mesh/transport") => {
              let gateway = state.private_gateway.as_ref();
              let enabled = gateway.is_some();
              let connected_peers = match gateway {
                  Some(gateway) => gateway.active_connection_count().await,
                  None => 0,
              };
              Ok(routing::ok_response(serde_json::json!({
                  "dht": {
                      "enabled": state.dht.is_some(),
                      "running": state.dht.is_some(),
                  },
                  "overlay": {
                      "enabled": enabled,
                      "endpoint": gateway.map(|gateway| gateway.bind().to_string()),
                  },
                  "status": if enabled { "Healthy" } else { "Disabled" },
                  "health": if enabled { "Healthy" } else { "Disabled" },
                  "description": if enabled {
                      "TLS mesh service transport is listening"
                  } else {
                      "Mesh service transport is disabled"
                  },
                  "transportPreference": "Auto",
                  "overlayBind": gateway.map(|gateway| gateway.bind()),
                  "overlayPort": gateway.map(|gateway| gateway.bind().port()),
                  "certificateSha256": gateway.map(|gateway| hex::encode(gateway.certificate_sha256())),
                  "dhtEnabled": state.dht.is_some(),
                  "connectedPeers": connected_peers,
                  "totalPeers": connected_peers,
                  "activeCircuits": 0,
                  "activeStreams": 0,
                  "bootstrapPeers": [],
                  "isolatedPeers": 0,
                  "quorumPeers": 0,
                  "relayedPeers": 0,
                  "natType": "Unknown",
                  "publicEndpoint": null,
                  "lastDhtError": null,
                  "lastDhtPublishUtc": null
              }).to_string()))
          }

          ("GET", path) if path.starts_with("/api/multisource/jobs/") => {
              let Some(job_id) = path_segment_after(path, "/api/multisource/jobs/") else {
                  return Ok(routing::not_found_response());
              };
              let job_id = decoded_path_segment(job_id);
              let versioned_profile = route.path.starts_with("/api/v0/")
                  && state.config.controller_profile
                      == ControllerProfile::Native;
              let swarm = state.multisource.read().await;
              if let Some(job) = swarm.get(&job_id) {
                  let body = if versioned_profile {
                      serde_json::json!({
                          "jobId": job.id,
                          "state": job.status,
                          "totalChunks": job.total_chunks,
                          "completedChunks": job.completed_chunks,
                          "percentComplete": if job.total_chunks > 0 {
                              job.completed_chunks as f64 * 100.0 / job.total_chunks as f64
                          } else {
                              0.0
                          },
                          "activeWorkers": 0,
                          "chunksPerSecond": 0.0,
                          "estimatedSecondsRemaining": 0.0,
                          "bytesDownloaded": job.bytes_downloaded,
                          "bytesDownloadedMB": job.bytes_downloaded as f64 / 1024.0 / 1024.0,
                      })
                      .to_string()
                  } else {
                      serde_json::to_string(job)
                          .map_err(|error| format!("multisource job serialization failed: {error}"))?
                  };
                  drop(swarm);
                  return Ok(routing::ok_response(body));
              }
              drop(swarm);
              let transfer_id = job_id.strip_prefix("transfer-").unwrap_or(&job_id);
              let transfers = state.transfers.read().await;
              let body = transfer_id
                  .parse::<u64>()
                  .ok()
                  .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id))
                  .map(|entry| {
                      let size = entry.size.unwrap_or(0);
                      let progress = if size == 0 {
                          0.0
                      } else {
                          (entry.bytes_transferred as f64 / size as f64) * 100.0
                      };
                      serde_json::json!({
                          "id": job_id,
                          "status": entry.status,
                          "filename": entry.filename,
                          "sources": entry.peer_username.as_deref().map(|peer| vec![peer]).unwrap_or_default(),
                          "progress": progress,
                          "bytesTransferred": entry.bytes_transferred,
                          "size": size,
                          "updated_at": entry.updated_at,
                      })
                  });
              drop(transfers);
              // Matches the oracle's real GetJobStatus: an unknown job id
              // is a real 404, not a fabricated 200 with an invented
              // "not_found" status string.
              match body {
                  Some(body) => Ok(routing::ok_response(body.to_string())),
                  None => Ok(HttpResponse {
                      status: "404 Not Found",
                      content_type: "application/json",
                      body: r#"{"error":"Job not found. It may have completed or been cancelled."}"#.to_owned(),
                  }),
              }
          }

          ("GET", "/api/player/external-visualizer") => {
              let visualizer = state
                  .media_services
                  .read()
                  .await
                  .external_visualizer
                  .clone();
              let resolved = resolve_external_visualizer_path(visualizer.command.as_deref());
              let working_directory = resolve_external_visualizer_working_directory(
                  visualizer.working_directory.as_deref(),
                  resolved.as_deref(),
              );
              let name = if visualizer.name.trim().is_empty() {
                  "External visualizer"
              } else {
                  visualizer.name.trim()
              };
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": visualizer.launch_enabled,
                  "configured": visualizer.configured(),
                  "available": resolved.is_some(),
                  "name": name,
                  "path": visualizer.command.as_deref().unwrap_or_default().trim(),
                  "resolvedPath": resolved,
                  "workingDirectory": working_directory,
                  "arguments": visualizer.arguments,
              }).to_string()))
          }

          ("GET", path) if path.starts_with("/api/realm-subject-indexes/") && path.ends_with("/conflicts") => {
              let Some(realm) = path_segment_between(
                  path,
                  "/api/realm-subject-indexes/",
                  "/conflicts",
              ) else {
                  return Ok(routing::not_found_response());
              };
              let realm = decoded_path_segment(realm);
              let report = state.realm_subject_indexes.read().await.conflict_report(&realm);
              Ok(routing::ok_response(report.to_string()))
          }

          ("POST", "/api/discovery-graph") => {
              if route.path.starts_with("/api/v0/") {
                  return Ok(discovery_graph::build_response(body, state).await);
              }
              let interests = state.interests.read().await;
              let wishlist = state.wishlist.read().await;
              let mut nodes = interests
                  .liked
                  .iter()
                  .map(|interest| serde_json::json!({
                      "id": interest.id,
                      "label": interest.name,
                      "kind": "interest",
                  }))
                  .collect::<Vec<_>>();
              nodes.extend(wishlist.records.iter().flat_map(|record| {
                  record.items.iter().map(|item| serde_json::json!({
                      "id": item.id,
                      "label": item.search_text(),
                      "kind": "wishlist",
                  }))
              }));
              let count = nodes.len();
              drop(wishlist);
              drop(interests);
              Ok(routing::accepted_response(serde_json::json!({
                  "nodes": nodes,
                  "edges": [],
                  "count": count,
                  "status": if count == 0 { "empty" } else { "ready" },
              }).to_string()))
          }

        ("POST", "/api/jobs/discography") => {
              if route.path.starts_with("/api/v0/") {
                  let payload = match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(payload @ serde_json::Value::Object(_)) => payload,
                      _ => {
                          return Ok(routing::bad_request_response(
                              "discography job body must be an object",
                          ));
                      }
                  };
                  let has_artist = ["artist", "artist_id", "query"].iter().any(|field| {
                      payload
                          .get(*field)
                          .and_then(serde_json::Value::as_str)
                          .is_some_and(|value| !value.trim().is_empty())
                  });
                  if !has_artist {
                      return Ok(routing::bad_request_response("artist_id is required"));
                  }
              }
              let artist = extract_json_string_field(body, "artist")
                  .or_else(|| extract_json_string_field(body, "artist_id"))
                  .or_else(|| extract_json_string_field(body, "query"))
                  .unwrap_or_else(|| "discography".to_owned());
              let query = format!("{} discography", artist.trim()).trim().to_owned();
              let mut searches = state.searches.write().await;
              let outcome = match searches.create(None, query, "global", None, Vec::new(), DEFAULT_SEARCH_TTL_SECONDS) {
                  Ok(outcome) => outcome,
                  Err(error) => return Ok(search_create_error_response(error)),
              };
              let record = outcome.record;
              let evicted = outcome.evicted;
              let job_projection = serde_json::json!({
                  "jobId": record.id,
                  "type": "discography",
                  "artistId": artist,
                  "artistName": artist,
                  "profile": "CoreDiscography",
                  "targetDirectory": "",
                  "releaseJobIds": [],
                  "releaseIds": [],
                  "totalReleases": 0,
                  "completedReleases": 0,
                  "failedReleases": 0,
                  "status": "Pending",
                  "createdAt": record.created_at.to_string(),
              });
              let job_store_result = state
                  .controller_features
                  .write()
                  .await
                  .upsert(format!("job/discography/{}", record.id), job_projection);
              if let Err(error) = job_store_result {
                  return Ok(routing::service_unavailable_response(&error));
              }
              let response = serde_json::json!({
                  "id": record.id,
                  "search_id": record.id,
                  "token": record.token,
                  "status": "queued",
                  "kind": "discography",
                  "artist": artist,
                  "query": record.query,
                  "results": [],
              }).to_string();
              drop(searches);
              delete_persisted_searches(state, &evicted).await?;
              Ok(routing::accepted_response(response))
          }

          ("POST", "/api/jobs/mb-release") => {
              if route.path.starts_with("/api/v0/") {
                  let payload = match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(payload @ serde_json::Value::Object(_)) => payload,
                      _ => {
                          return Ok(routing::bad_request_response(
                              "MusicBrainz release job body must be an object",
                          ));
                      }
                  };
                  let has_release_or_search = [
                      "mb_release_id",
                      "mbReleaseId",
                      "artist",
                      "title",
                      "release",
                      "query",
                  ]
                  .iter()
                  .any(|field| {
                      payload
                          .get(*field)
                          .and_then(serde_json::Value::as_str)
                          .is_some_and(|value| !value.trim().is_empty())
                  });
                  if !has_release_or_search {
                      return Ok(routing::bad_request_response("mb_release_id is required"));
                  }
              }
              let release_id = extract_json_string_field(body, "mb_release_id")
                  .or_else(|| extract_json_string_field(body, "mbReleaseId"))
                  .filter(|id| !id.trim().is_empty());
              let (artist, title) = if let Some(release_id) =
                  release_id.filter(|_| route.path.starts_with("/api/v0/"))
              {
                  let musicbrainz = state.integration_settings.read().await.musicbrainz.clone();
                  match musicbrainz_release_target_with_settings(&musicbrainz, &release_id).await
                  {
                      Ok(Some(target)) => target,
                      Ok(None) => {
                          return Ok(HttpResponse {
                              status: "404 Not Found",
                              content_type: "application/json",
                              body: serde_json::json!(
                                  "Unable to resolve release into a SongID-ready MusicBrainz target."
                              )
                              .to_string(),
                          });
                      }
                      Err(error) => {
                          return Ok(routing::service_unavailable_response(&format!(
                              "MusicBrainz lookup failed: {error}"
                          )));
                      }
                  }
              } else {
                  let artist = extract_json_string_field(body, "artist").unwrap_or_default();
                  let title = extract_json_string_field(body, "title")
                      .or_else(|| extract_json_string_field(body, "release"))
                      .or_else(|| extract_json_string_field(body, "query"))
                      .unwrap_or_else(|| "release".to_owned());
                  (artist, title)
              };
              let query = [artist.as_str(), title.as_str()]
                  .into_iter()
                  .filter(|value| !value.trim().is_empty())
                  .collect::<Vec<_>>()
                  .join(" ");
              let mut searches = state.searches.write().await;
              let outcome = match searches.create(None, query, "global", None, Vec::new(), DEFAULT_SEARCH_TTL_SECONDS) {
                  Ok(outcome) => outcome,
                  Err(error) => return Ok(search_create_error_response(error)),
              };
              let record = outcome.record;
              let evicted = outcome.evicted;
              let job_projection = serde_json::json!({
                  "jobId": record.id,
                  "type": "mb-release",
                  "artistId": artist,
                  "artistName": artist,
                  "profile": "AllReleases",
                  "targetDirectory": "",
                  "releaseJobIds": [],
                  "releaseIds": [],
                  "totalReleases": 0,
                  "completedReleases": 0,
                  "failedReleases": 0,
                  "status": "Pending",
                  "createdAt": record.created_at.to_string(),
              });
              let job_store_result = state
                  .controller_features
                  .write()
                  .await
                  .upsert(format!("job/mb-release/{}", record.id), job_projection);
              if let Err(error) = job_store_result {
                  return Ok(routing::service_unavailable_response(&error));
              }
              let response = serde_json::json!({
                  "id": record.id,
                  "search_id": record.id,
                  "token": record.token,
                  "status": "queued",
                  "kind": "mb-release",
                  "artist": artist,
                  "title": title,
                  "query": record.query,
                  "results": [],
              }).to_string();
              drop(searches);
              delete_persisted_searches(state, &evicted).await?;
              Ok(routing::accepted_response(response))
          }

          ("POST", "/api/options/yaml/validate") => {
              if let Some(response) = controller_options_validation_failure_response(state) {
                  return Ok(response);
              }
              if !effective_remote_configuration(state) {
                  return Ok(controller_forbidden_response());
              }
              match controller_options_config_validate_response(
                  body,
                  state.config.controller_profile,
              ) {
                  Ok(response) => Ok(response),
                  Err(error) => Ok(routing::bad_request_response(&error)),
              }
          }

          ("POST", "/api/source-feed-imports/preview") => {
              if route.path.starts_with("/api/v0/") {
                  let request = serde_json::from_str::<serde_json::Value>(body)
                      .unwrap_or(serde_json::Value::Null);
                  let raw = extract_json_string_field(body, "sourceText")
                      .or_else(|| extract_json_string_field(body, "text"))
                      .unwrap_or_default();
                  if raw.trim().is_empty() {
                      return Ok(routing::bad_request_response("SourceText is required"));
                  }
                  let requested_limit = extract_json_u64_field(body, "limit").unwrap_or(500);
                  if requested_limit == 0 {
                      return Ok(routing::bad_request_response("Limit must be greater than 0"));
                  }
                  let maximum = state
                      .integration_settings
                      .read()
                      .await
                      .spotify
                      .max_items_per_import;
                  let safe_limit = requested_limit.min(maximum).max(1);
                  let requested_kind = extract_json_string_field(body, "sourceKind")
                      .unwrap_or_else(|| "auto".to_owned())
                      .trim()
                      .to_ascii_lowercase();
                  let fetch_provider_urls = request
                      .get("fetchProviderUrls")
                      .and_then(serde_json::Value::as_bool)
                      .unwrap_or(true);
                  if fetch_provider_urls && looks_like_spotify_source(&raw, &requested_kind) {
                      let provider_access_token = extract_json_string_field(
                          body,
                          "providerAccessToken",
                      )
                      .unwrap_or_default();
                      let result = preview_spotify_source_feed(
                          state,
                          &raw,
                          &provider_access_token,
                          usize::try_from(safe_limit).unwrap_or(usize::MAX),
                          "https://accounts.spotify.com/api/token",
                          "https://api.spotify.com/v1",
                      )
                      .await?;
                      let mut history = state.source_feed_import_history.write().await;
                      let previous = history.clone();
                      history.record(&request, &raw, &result);
                      if let Err(error) = history.persist(&state.config.state_dir) {
                          *history = previous;
                          return Ok(routing::service_unavailable_response(&error));
                      }
                      drop(history);
                      return Ok(HttpResponse {
                          status: "200 OK",
                          content_type: "application/json; charset=utf-8",
                          body: result.to_string(),
                      });
                  }
                  if fetch_provider_urls {
                      if let Some(result) = preview_configured_provider_source_feed(
                          state,
                          &raw,
                          &requested_kind,
                          usize::try_from(safe_limit).unwrap_or(usize::MAX),
                          "https://www.googleapis.com/youtube/v3/playlistItems",
                          "https://ws.audioscrobbler.com/2.0/",
                          "https://itunes.apple.com/lookup",
                          "https://api.listenbrainz.org",
                      )
                      .await?
                      {
                          let mut history = state.source_feed_import_history.write().await;
                          let previous = history.clone();
                          history.record(&request, &raw, &result);
                          if let Err(error) = history.persist(&state.config.state_dir) {
                              *history = previous;
                              return Ok(routing::service_unavailable_response(&error));
                          }
                          drop(history);
                          return Ok(HttpResponse {
                              status: "200 OK",
                              content_type: "application/json; charset=utf-8",
                              body: result.to_string(),
                          });
                      }
                  }
                  let include_album = request
                      .get("includeAlbum")
                      .and_then(serde_json::Value::as_bool)
                      .unwrap_or(false);
                  let result = preview_local_source_feed(
                      &raw,
                      &requested_kind,
                      include_album,
                      usize::try_from(safe_limit).unwrap_or(usize::MAX),
                  );
                  let mut history = state.source_feed_import_history.write().await;
                  let previous = history.clone();
                  history.record(&request, &raw, &result);
                  if let Err(error) = history.persist(&state.config.state_dir) {
                      *history = previous;
                      return Ok(routing::service_unavailable_response(&error));
                  }
                  drop(history);
                  return Ok(HttpResponse {
                      status: "200 OK",
                      content_type: "application/json; charset=utf-8",
                      body: result.to_string(),
                  });
              }
              let raw = extract_json_string_field(body, "sourceText")
                  .or_else(|| extract_json_string_field(body, "text"))
                  .or_else(|| extract_json_string_field(body, "content"))
                  .or_else(|| extract_json_string_field(body, "playlist"))
                  .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
              let items = raw
                  .lines()
                  .map(str::trim)
                  .filter(|line| !line.is_empty())
                  .enumerate()
                  .map(|(index, line)| {
                      let (artist, title) = line
                          .split_once(" - ")
                          .map(|(artist, title)| (artist.trim(), title.trim()))
                          .unwrap_or(("", line));
                      serde_json::json!({
                          "id": format!("preview-{}", index + 1),
                          "artist": artist,
                          "title": title,
                          "searchText": line,
                          "valid": !line.is_empty(),
                      })
                  })
                  .collect::<Vec<_>>();
              let count = items.len();
              Ok(routing::ok_response(serde_json::json!({
                  "items": items,
                  "count": count,
                  "valid": count > 0,
              }).to_string()))
          }

          // TASTE RECOMMENDATIONS POST ENDPOINTS (Phase 6)
          ("POST", "/api/taste-recommendations") => {
              if route.path.starts_with("/api/v0/") && !body.trim().is_empty() {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Null | serde_json::Value::Object(_)) => {}
                      Ok(_) | Err(_) => {
                          return Ok(routing::bad_request_response("The request body is invalid"));
                      }
                  }
              }
              let interests = state.interests.read().await;
              let mut value = serde_json::from_str::<serde_json::Value>(
                  &interests.recommendations_json("recommendations"),
              )
              .unwrap_or_else(|_| serde_json::json!({ "recommendations": [], "count": 0 }));
              drop(interests);
              if route.path.starts_with("/api/v0/") {
                  let recommendations = value
                      .get("recommendations")
                      .and_then(serde_json::Value::as_array)
                      .cloned()
                      .unwrap_or_default();
                  let minimum_trusted_sources = extract_json_u64_field(body, "minimumTrustedSources")
                      .unwrap_or(2)
                      .max(2);
                  return Ok(routing::ok_response(serde_json::json!({
                      "minimumTrustedSources": minimum_trusted_sources,
                      "trustedActorCount": 0,
                      "candidateCount": recommendations.len(),
                      "recommendations": recommendations,
                  }).to_string()));
              }
              value["status"] = serde_json::Value::String("analyzing".to_owned());
              let json = value.to_string();
              Ok(routing::accepted_response(json))
          }

          ("POST", "/api/taste-recommendations/graph-preview") => {
              if route.path.starts_with("/api/v0/") {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Object(_)) => {}
                      Ok(serde_json::Value::Null) | Err(_) => {
                          return Ok(routing::bad_request_response(
                              "graph preview request is required",
                          ));
                      }
                      Ok(_) => return Ok(routing::bad_request_response("The request body is invalid")),
                  }
              }
              if route.path.starts_with("/api/v0/")
                  && serde_json::from_str::<serde_json::Value>(body)
                      .ok()
                      .and_then(|value| value.get("workRef").cloned())
                      .and_then(|value| value.get("@context").cloned())
                      .is_some_and(|value| value.is_null())
              {
                  return Ok(native_model_validation_response());
              }
              let interests = state.interests.read().await;
              let graph_data = interests
                  .liked
                  .iter()
                  .map(|interest| {
                      serde_json::json!({
                          "id": interest.id,
                          "label": interest.name,
                          "kind": "interest",
                      })
                  })
                  .collect::<Vec<_>>();
              let nodes = graph_data.len();
              drop(interests);
              let json = serde_json::json!({
                  "graph_data": graph_data,
                  "nodes": nodes,
                  "edges": 0,
              }).to_string();
              Ok(routing::ok_response(json))
          }

          ("POST", "/api/taste-recommendations/release-radar") => {
              if route.path.starts_with("/api/v0/") {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Object(_)) => {}
                      Ok(serde_json::Value::Null) | Err(_) => {
                          return Ok(routing::bad_request_response(
                              "radar subscription request is required",
                          ));
                      }
                      Ok(_) => return Ok(routing::bad_request_response("The request body is invalid")),
                  }
              }
              if route.path.starts_with("/api/v0/")
                  && serde_json::from_str::<serde_json::Value>(body)
                      .ok()
                      .and_then(|value| value.get("workRef").cloned())
                      .and_then(|value| value.get("@context").cloned())
                      .is_some_and(|value| value.is_null())
              {
                  return Ok(native_model_validation_response());
              }
              let wishlist = state.wishlist.read().await;
              let recommendations = wishlist
                  .records
                  .iter()
                  .flat_map(|record| record.items.iter())
                  .map(|item| {
                      serde_json::json!({
                          "id": item.id,
                          "artist": item.artist,
                          "title": item.title,
                          "searchText": item.search_text(),
                          "source": "wishlist",
                      })
                  })
                  .collect::<Vec<_>>();
              let count = recommendations.len();
              drop(wishlist);
              let json = serde_json::json!({
                  "recommendations": recommendations,
                  "count": count,
                  "status": "processing",
              }).to_string();
              Ok(if route.path.starts_with("/api/v0/") {
                  routing::ok_response(json)
              } else {
                  routing::accepted_response(json)
              })
          }

          ("POST", "/api/taste-recommendations/wishlist") => {
              if route.path.starts_with("/api/v0/") {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Object(_)) => {}
                      Ok(serde_json::Value::Null) | Err(_) => {
                          return Ok(routing::bad_request_response(
                              "promotion request is required",
                          ));
                      }
                      Ok(_) => return Ok(routing::bad_request_response("The request body is invalid")),
                  }
              }
              let request = serde_json::from_str::<serde_json::Value>(body)
                  .unwrap_or_else(|_| serde_json::json!({}));
              let work_ref = request.get("workRef").cloned().unwrap_or_default();
              if route.path.starts_with("/api/v0/")
                  && work_ref.get("@context").is_some_and(serde_json::Value::is_null)
              {
                  return Ok(native_model_validation_response());
              }
              // Matches the oracle's real PromoteToWishlistAsync: a
              // recommendable WorkRef either promotes to a real, honest
              // review-only (disabled, no auto-download) Wishlist seed,
              // or -- if a wishlist item with the same search text
              // already exists -- reports the existing one, rather than
              // just echoing the caller's own current wishlist back as
              // if a promotion had happened.
              if !work_ref_is_recommendable(&work_ref) {
                  return Ok(HttpResponse {
                      status: "400 Bad Request",
                      content_type: "application/json",
                      body: serde_json::json!({
                          "created": false,
                          "message": "WorkRef is not a safe music recommendation.",
                      })
                      .to_string(),
                  });
              }
              let search_text = work_ref_search_text(&work_ref);
              let mut wishlist = state.wishlist.write().await;
              if let Some(existing_id) = wishlist.item_id_for_search_text(&search_text) {
                  drop(wishlist);
                  return Ok(routing::ok_response(
                      serde_json::json!({
                          "created": false,
                          "wishlistItemId": existing_id,
                          "searchText": search_text,
                          "message": "Wishlist already has this recommendation seed.",
                      })
                      .to_string(),
                  ));
              }
              let previous = wishlist.clone();
              let creator = work_ref
                  .get("creator")
                  .and_then(serde_json::Value::as_str)
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let title = work_ref
                  .get("title")
                  .and_then(serde_json::Value::as_str)
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let note = request.get("note").and_then(serde_json::Value::as_str);
              let filter = work_ref_wishlist_filter(&work_ref, note);
              let Ok(item) = wishlist.add_item_with_settings(
                  creator,
                  title,
                  "TasteRecommendation".to_owned(),
                  filter,
                  false,
                  false,
                  25,
                  None,
              ) else {
                  drop(wishlist);
                  return Ok(routing::service_unavailable_response(
                      "wishlist item capacity is full",
                  ));
              };
              let mutated = wishlist.clone();
              drop(wishlist);
              if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                  rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                  return Ok(routing::service_unavailable_response(&error));
              }
              Ok(routing::ok_response(
                  serde_json::json!({
                      "created": true,
                      "wishlistItemId": item.id,
                      "searchText": search_text,
                      "message": "Created review-only Wishlist seed.",
                  })
                  .to_string(),
              ))
          }

          // PLAYER LAUNCH ENDPOINT (Phase 6)
        ("POST", "/api/player/external-visualizer/launch") => {
            let visualizer = state
                .media_services
                .read()
                .await
                .external_visualizer
                .clone();
            if !visualizer.launch_enabled {
                record_event(
                    state,
                    "external_visualizer.launch.blocked",
                    "external_visualizer",
                    Some("launch is disabled by configuration".to_owned()),
                )
                .await;
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "started": false,
                        "name": serde_json::Value::Null,
                        "processId": serde_json::Value::Null,
                        "error": "External visualizer launching is disabled in configuration.",
                    })
                    .to_string(),
                });
            }
            if let Some(command) = resolve_external_visualizer_path(visualizer.command.as_deref()) {
                let Ok(process_permit) = Arc::clone(&state.external_visualizer_processes)
                    .try_acquire_owned()
                else {
                    return Ok(routing::HttpResponse {
                        status: "503 Service Unavailable",
                        content_type: "application/json",
                        body: "{\"error\":\"external visualizer process limit reached\"}".to_owned(),
                    });
                };
                let mut process = std::process::Command::new(&command);
                process.args(
                    visualizer
                        .arguments
                        .iter()
                        .filter(|argument| !argument.trim().is_empty()),
                );
                if let Some(directory) = resolve_external_visualizer_working_directory(
                    visualizer.working_directory.as_deref(),
                    Some(&command),
                ) {
                    process.current_dir(directory);
                }
                match process.spawn() {
                    Ok(mut child) => {
                        let process_id = child.id();
                        let name = if visualizer.name.trim().is_empty() {
                            "External visualizer".to_owned()
                        } else {
                            visualizer.name.trim().to_owned()
                        };
                        tokio::task::spawn_blocking(move || {
                            let _process_permit = process_permit;
                            let _ = child.wait();
                        });
                        record_event(
                            state,
                            "external_visualizer.launch",
                            "external_visualizer".to_owned(),
                            Some("launch requested".to_owned()),
                        )
                        .await;
                        Ok(routing::ok_response(serde_json::json!({
                            "started": true,
                            "name": name,
                            "processId": process_id,
                            "error": serde_json::Value::Null,
                        }).to_string()))
                    }
                    Err(error) => {
                        drop(process_permit);
                        record_event(
                            state,
                            "external_visualizer.launch.failed",
                            "external_visualizer".to_owned(),
                            Some("launch failed".to_owned()),
                        )
                        .await;
                        Ok(HttpResponse {
                            status: "400 Bad Request",
                            content_type: "application/json",
                            body: serde_json::json!({
                                "started": false,
                                "name": serde_json::Value::Null,
                                "processId": serde_json::Value::Null,
                                "error": error.to_string(),
                            })
                            .to_string(),
                        })
                    }
                }
            } else {
                record_event(
                    state,
                    "external_visualizer.launch.failed",
                    "external_visualizer".to_owned(),
                    Some("launch failed".to_owned()),
                )
                .await;
                Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "started": false,
                        "name": serde_json::Value::Null,
                        "processId": serde_json::Value::Null,
                        "error": "External visualizer path is not configured or does not exist.",
                    })
                    .to_string(),
                })
            }
        }

          // BANS & BLOCKING ENDPOINTS
        ("GET", path) if security_ban_route_tail(path).is_some_and(|tail| tail.is_empty()) => {
            let security = state.security.read().await;
            let json = security.json_value().to_string();
            drop(security);
            Ok(routing::ok_response(json))
        }

        ("POST", path)
            if security_ban_route_tail(path).as_deref() == Some(["username"].as_slice()) =>
        {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let Some(username) = normalize_security_ban_value("username", &username) else {
                return Ok(routing::bad_request_response("username is required"));
            };
            let (reason, duration_seconds, is_permanent) = security_ban_options(body);
            let mut security = state.security.write().await;
            let previous = security.clone();
            let Some(record) = security.ban_with_options(
                "username",
                username.clone(),
                reason,
                duration_seconds,
                is_permanent,
            ) else {
                return Ok(routing::service_unavailable_response("security ban capacity is full"));
            };
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    *security = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(String::new()));
            }
            Ok(routing::ok_response(serde_json::json!({
                "username": username,
                "banned": true,
                "persisted": persisted,
                "kind": record.kind,
                "created_at": record.created_at,
                "activeBans": active_bans,
            }).to_string()))
        }

        ("DELETE", path)
            if security_ban_route_tail(path)
                .is_some_and(|tail| tail.len() == 2 && tail[0] == "username") =>
        {
            let username = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let Some(username) = normalize_security_ban_value("username", &username) else {
                return Ok(routing::bad_request_response("username is required"));
            };
            let mut security = state.security.write().await;
            let previous = security.clone();
            let removed = security.unban("username", &username);
            let persisted = if removed {
                match persist_security_unban(state, "username", &username).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        *security = previous;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(if removed {
                    routing::ok_response(String::new())
                } else {
                    routing::not_found_response()
                });
            }
            Ok(routing::ok_response(serde_json::json!({
                "username": username,
                "banned": false,
                "removed": removed,
                "persisted": persisted,
                "activeBans": active_bans,
            }).to_string()))
        }

        ("POST", path)
            if security_ban_route_tail(path).as_deref() == Some(["ip"].as_slice()) =>
        {
            let ip = extract_json_string_field(body, "ipAddress")
                .or_else(|| extract_json_string_field(body, "ip"))
                .unwrap_or_default();
            let Some(ip) = normalize_security_ban_value("ip", &ip) else {
                return Ok(routing::bad_request_response("valid ip is required"));
            };
            let (reason, duration_seconds, is_permanent) = security_ban_options(body);
            let mut security = state.security.write().await;
            let previous = security.clone();
            let Some(record) = security.ban_with_options(
                "ip",
                ip.clone(),
                reason,
                duration_seconds,
                is_permanent,
            ) else {
                return Ok(routing::service_unavailable_response("security ban capacity is full"));
            };
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    *security = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(String::new()));
            }
            Ok(routing::ok_response(serde_json::json!({
                "ip": ip,
                "banned": true,
                "persisted": persisted,
                "kind": record.kind,
                "created_at": record.created_at,
                "activeBans": active_bans,
            }).to_string()))
        }

        ("DELETE", path)
            if security_ban_route_tail(path)
                .is_some_and(|tail| tail.len() == 2 && tail[0] == "ip") =>
        {
            let ip = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let Some(ip) = normalize_security_ban_value("ip", &ip) else {
                return Ok(routing::bad_request_response("valid ip is required"));
            };
            let mut security = state.security.write().await;
            let previous = security.clone();
            let removed = security.unban("ip", &ip);
            let persisted = if removed {
                match persist_security_unban(state, "ip", &ip).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        *security = previous;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(if removed {
                    routing::ok_response(String::new())
                } else {
                    routing::not_found_response()
                });
            }
            Ok(routing::ok_response(serde_json::json!({
                "ip": ip,
                "banned": false,
                "removed": removed,
                "persisted": persisted,
                "activeBans": active_bans,
            }).to_string()))
        }

        // ADDITIONAL MISSING DELETE ENDPOINTS (Phase 5)
        ("DELETE", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let before = messages.records.len();
            messages.records.retain(|record| record.username != username);
            let removed = before.saturating_sub(messages.records.len());
            let mutated = messages.clone();
            drop(messages);
            let persisted_removed = match persist_conversation_delete_checked(state, &username).await {
                Ok(removed) => removed.unwrap_or(0),
                Err(error) => {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            if route.path.starts_with("/api/v0/") {
                Ok(if removed > 0 || persisted_removed > 0 {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                })
            } else {
                Ok(routing::ok_response(
                    (removed > 0 || persisted_removed > 0).to_string(),
                ))
            }
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/directories/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/files/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", "/api/integrations/spotify") => {
            if let Err(error) = delete_spotify_connection_store(state) {
                return Ok(routing::service_unavailable_response(&error));
            }
            *state.spotify_connection.write().await = SpotifyConnectionStore::default();
            if route.path.starts_with("/api/v0/") {
                Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::ok_response("{\"connected\":false,\"removed\":true}".to_owned()))
            }
        }

        ("DELETE", "/api/nowplaying") => {
            let mut now_playing = state.now_playing.write().await;
            let previous = now_playing.clone();
            let cleared = now_playing.clear();
            let mutated = now_playing.clone();
            drop(now_playing);
            if let Err(error) = persist_now_playing_clear_checked(state).await {
                let mut now_playing = state.now_playing.write().await;
                if *now_playing == mutated {
                    *now_playing = previous;
                }
                drop(now_playing);
                return Ok(routing::service_unavailable_response(&error));
            }
            if route.path.starts_with("/api/v0/") {
                Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::ok_response(format!(
                    "{{\"now_playing\":[],\"count\":0,\"cleared\":true,\"cleared_count\":{}}}",
                    cleared
                )))
            }
        }

        ("DELETE", "/api/relay") => {
            let json = match mutate_runtime_compat_state(state, |_, relay| relay.set_enabled(false))
                .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json.to_string()))
        }

        ("DELETE", "/api/relay/agent") => {
            let json = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_relay_agent(false).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        ("DELETE", "/api/shares") => {
            if route.path.starts_with("/api/v0/") {
                if state.share_scans.available_permits() == 0 {
                    state.share_scans.add_permits(1);
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::ok_response("true".to_owned()))
            }
        }

        ("GET", "/api/shares/contents") => {
            let shares = state.shares.read().await;
            if !shares.scan_errors.is_empty() {
                drop(shares);
                return Ok(routing::internal_server_error_response(
                    "share browse unavailable",
                ));
            }
            let json = controller_share_directories_json(&shares.entries, None);
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("GET", path) if path.starts_with("/api/shares/") && path.ends_with("/contents") => {
            let Some(share_id) = share_contents_id(path) else {
                return Ok(routing::not_found_response());
            };
            let shares = state.shares.read().await;
            let Some(root) = shares
                .roots
                .iter()
                .find(|root| share_root_id(&root.label) == share_id)
            else {
                return Ok(routing::not_found_response());
            };
            if !shares.scan_errors.is_empty() {
                drop(shares);
                return Ok(routing::internal_server_error_response(
                    "share browse unavailable",
                ));
            }
            let json = controller_share_directories_json(&shares.entries, Some(&root.label));
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("GET", path) if path.starts_with("/api/shares/") => {
            let Some(share_id) = share_resource_id(path) else {
                return Ok(routing::not_found_response());
            };
            let shares = state.shares.read().await;
            let Some(root) = shares
                .roots
                .iter()
                .find(|root| share_root_id(&root.label) == share_id)
            else {
                return Ok(routing::not_found_response());
            };
            let json = controller_share_value(root).to_string();
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("DELETE", path) if path.starts_with("/api/transfers/") && path.ends_with("/all/completed") => {
            let mut transfers = state.transfers.write().await;
            let before = transfers.entries.len();
            transfers.entries.retain(|entry| {
                !matches!(
                    entry.status.as_str(),
                    "succeeded" | "completed" | "cancelled" | "failed" | "rejected"
                )
            });
            let pruned = before.saturating_sub(transfers.entries.len());
            transfers.persist_state();
            drop(transfers);
            Ok(routing::ok_response(format!("{{\"pruned\":{}}}", pruned)))
        }

        // Generic :var pattern endpoints for mesh/network cleanup & channels (Phase 5)
        ("DELETE", path) if path.contains("/cleanup") && path.matches('/').count() == 3 && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("DELETE", path) if path.contains("/unpublish") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("DELETE", path) if path.contains("/channels/") && path.matches('/').count() == 4 && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        // ADDITIONAL MISSING INTEGRATION & PLATFORM ENDPOINTS (Phase 5)
        ("GET", "/api/integrations/spotify/status") => {
            let spotify = state.integration_settings.read().await.spotify.clone();
            let connection = state.spotify_connection.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: connection.status_json(spotify.configured()).to_string(),
            })
        }

        ("POST", "/api/integrations/spotify/authorize") => {
            let spotify = state.integration_settings.read().await.spotify.clone();
            if !spotify.configured() {
                return Ok(routing::bad_request_response(
                    "Spotify authorization is not configured.",
                ));
            }
            let client_id = spotify.client_id.as_deref().unwrap_or_default();
            let redirect_uri = spotify_redirect_uri(state, &spotify);
            let (state_token, oauth_record, previous, mutated) = {
                let mut oauth_states = state.oauth_states.write().await;
                let previous = oauth_states.clone();
                let Some(state_token) = oauth_states.issue("spotify", &redirect_uri, 600) else {
                    return Ok(routing::service_unavailable_response(
                        "OAuth state capacity is full",
                    ));
                };
                let oauth_record = oauth_states.records.get(&state_token).cloned();
                let mutated = oauth_states.clone();
                (state_token, oauth_record, previous, mutated)
            };
            if let Some(oauth_record) = oauth_record.as_ref() {
                if let Err(error) =
                    persist_oauth_state_checked(state, &state_token, oauth_record).await
                {
                    rollback_oauth_states_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let code_verifier = oauth_record
                .as_ref()
                .and_then(|record| record.code_verifier.as_deref())
                .ok_or_else(|| "Spotify PKCE verifier generation failed".to_owned())?;
            let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
            let authorization_url = format!(
                "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}&code_challenge_method=S256&code_challenge={}",
                url_encode(client_id),
                url_encode(&spotify.scopes),
                url_encode(&redirect_uri),
                url_encode(&state_token),
                url_encode(&code_challenge),
            );
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::json!({
                    "authorizationUrl": authorization_url,
                    "redirectUri": redirect_uri,
                    "scope": spotify.scopes,
                })
                .to_string(),
            })
        }

        ("GET", "/api/integrations/spotify/callback") => {
            let params = route.query.map(query_params).unwrap_or_default();
            let code = params
                .iter()
                .find(|(key, _)| key == "code")
                .map(|(_, value)| value.as_str());
            let error = params
                .iter()
                .find(|(key, _)| key == "error")
                .map(|(_, value)| value.as_str());
            let state_value = params
                .iter()
                .find(|(key, _)| key == "state")
                .map(|(_, value)| value.as_str())
                .unwrap_or("");

            if error.is_some_and(|value| !value.trim().is_empty()) {
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/html; charset=utf-8",
                    body: spotify_callback_html("Spotify authorization failed."),
                });
            }
            let Some(code) = code.filter(|value| !value.trim().is_empty()) else {
                return Ok(routing::bad_request_response(
                    "Missing Spotify authorization code or state.",
                ));
            };
            if state_value.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "Missing Spotify authorization code or state.",
                ));
            }
            let pending = match consume_oauth_state(state, "spotify", state_value).await {
                Ok(Some(record)) if record.code_verifier.is_some() => record,
                Ok(_) => {
                    return Ok(routing::bad_request_response(
                        "Spotify authorization could not be completed.",
                    ))
                }
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            let spotify = state.integration_settings.read().await.spotify.clone();
            match complete_spotify_authorization(
                state,
                &spotify,
                &pending,
                code,
                "https://accounts.spotify.com/api/token",
                "https://api.spotify.com/v1/me",
            )
            .await
            {
                Ok(_) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/html; charset=utf-8",
                    body: spotify_callback_html(
                        "Spotify account connected. You can close this window.",
                    ),
                }),
                Err(error) => {
                    record_daemon_log(state, logging::LogLevel::Warn, "spotify", error).await;
                    Ok(routing::internal_server_error_response(
                        "Spotify authorization could not be completed.",
                    ))
                }
            }
        }

        ("GET", "/api/integrations/lidarr/sync/status") => {
            let sync = state.lidarr_sync_state.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: sync.json().to_string(),
            })
        }

        ("GET", "/api/integrations/lidarr/status") => {
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            match fetch_lidarr_system_status(&lidarr).await {
                Ok(value) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: value.to_string(),
                }),
                Err(error) => Ok(routing::service_unavailable_response(&error)),
            }
        }

        ("GET", "/api/integrations/lidarr/wanted/missing") => {
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            if !lidarr.configured() {
                let library = state.library.read().await;
                let missing_albums = lidarr_missing_albums_value(&library);
                let count = missing_albums.len();
                let updated_at = library.updated_at;
                drop(library);
                return Ok(routing::ok_response(serde_json::json!({
                    "missing_albums": missing_albums,
                    "count": count,
                    "status": if count == 0 { "local_clean" } else { "local" },
                    "source": "library-health",
                    "configured": false,
                    "next_action": if count == 0 { "library metadata is complete" } else { "fix library health issues or configure Lidarr URL and API key" },
                    "updated_at": updated_at,
                }).to_string()));
            }
            let page = query_parameter(route.query, "page")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1)
                .max(1);
            let page_size = query_parameter(route.query, "pageSize")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(50)
                .clamp(1, 250);
            match fetch_lidarr_wanted_missing(&lidarr, page, page_size).await {
                Ok(value) if route.path.starts_with("/api/v0/") => {
                    let mut records = value
                        .get("records")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    for record in &mut records {
                        let artist = record
                            .pointer("/artist/artistName")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let title = record
                            .get("title")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        record["searchText"] = serde_json::Value::String(
                            [artist, title]
                                .into_iter()
                                .filter(|value| !value.trim().is_empty())
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                    }
                    let total = value
                        .get("totalRecords")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(0));
                    Ok(HttpResponse {
                        status: "200 OK",
                        content_type: "application/json; charset=utf-8",
                        body: serde_json::json!({
                            "records": records,
                            "totalRecords": total,
                            "page": page,
                            "pageSize": page_size,
                        })
                        .to_string(),
                    })
                }
                Ok(value) => Ok(routing::ok_response(value.to_string())),
                Err(_) => Ok(routing::ok_response(
                    "{\"missing_albums\":[],\"count\":0,\"status\":\"connection_failed\",\"error\":\"Lidarr connection failed\"}".to_owned(),
                )),
            }
        }

        ("POST", "/api/integrations/lidarr/wanted/sync") => {
            if route.path.starts_with("/api/v0/") {
                let lidarr = state.integration_settings.read().await.lidarr.clone();
                let result = match sync_lidarr_wanted_to_wishlist(state, &lidarr).await {
                    Ok(result) => result,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: result.to_string(),
                });
            }
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            if !lidarr.configured() {
                let library = state.library.read().await;
                let missing_albums = lidarr_missing_albums_value(&library);
                let missing_count = missing_albums.len();
                drop(library);
                let body = match mutate_runtime_compat_state(state, |runtime, _| {
                    let mut value = runtime.record_lidarr_sync(missing_count, false);
                    if let Some(object) = value.as_object_mut() {
                        object.insert("missing_albums".to_owned(), serde_json::json!(missing_albums));
                        object.insert("source".to_owned(), serde_json::json!("library-health"));
                        object.insert(
                            "next_action".to_owned(),
                            serde_json::json!(if missing_count == 0 {
                                "library metadata is complete"
                            } else {
                                "fix library health issues or configure Lidarr URL and API key"
                            }),
                        );
                    }
                    value.to_string()
                }).await {
                    Ok(body) => body,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                return Ok(routing::accepted_response(body));
            }
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.record_lidarr_sync(0, true);
                if let Some(object) = value.as_object_mut() {
                    object.insert("missing_albums".to_owned(), serde_json::json!([]));
                    object.insert(
                        "next_action".to_owned(),
                        serde_json::json!("poll wanted/missing"),
                    );
                }
                value.to_string()
            }).await {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("GET", "/api/integrations/lidarr/manualimport/history")
        | ("GET", "/api/v0/integrations/lidarr/manualimport/history") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 500) {
                Ok(value) => value.unwrap_or(50),
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 500",
                    ))
                }
            };
            Ok(routing::ok_response(
                serde_json::Value::Array(list_lidarr_import_history(state, limit).await)
                    .to_string(),
            ))
        }

        ("POST", path)
            if (path.starts_with("/api/integrations/lidarr/manualimport/history/")
                || path.starts_with("/api/v0/integrations/lidarr/manualimport/history/"))
                && path.ends_with("/retry") =>
        {
            let prefix = if path.starts_with("/api/v0/") {
                "/api/v0/integrations/lidarr/manualimport/history/"
            } else {
                "/api/integrations/lidarr/manualimport/history/"
            };
            let Some(history_id) = path
                .strip_prefix(prefix)
                .and_then(|value| value.strip_suffix("/retry"))
                .filter(|value| uuid::Uuid::parse_str(value).is_ok())
            else {
                return Ok(routing::bad_request_response("HistoryId is required"));
            };
            let Some(history) = state
                .controller_features
                .read()
                .await
                .get(&lidarr_import_history_key(history_id))
                .cloned()
            else {
                return Ok(routing::not_found_response());
            };
            let Some(directory) = history
                .get("sourceDirectory")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(routing::bad_request_response(
                    "Import history record has no source directory",
                ));
            };
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            let result = match run_lidarr_import_with_history(
                state,
                &lidarr,
                directory,
                Some(history_id),
            )
            .await
            {
                Ok(result) => result,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(result.to_string()))
        }

        ("POST", "/api/integrations/lidarr/manualimport") => {
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if route.path.starts_with("/api/v0/") {
                if directory.trim().is_empty() {
                    return Ok(routing::bad_request_response("Directory is required"));
                }
                let lidarr = state.integration_settings.read().await.lidarr.clone();
                let mut result = match run_lidarr_import_with_history(state, &lidarr, &directory, None).await
                {
                    Ok(result) => result,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                // Keep rejected filenames available to the internal completed
                // download policy, but match the frozen controller's public
                // manual-import response shape.
                if let Some(object) = result.as_object_mut() {
                    object.remove("rejectedFilenames");
                }
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: result.to_string(),
                });
            }
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            if !lidarr.configured() {
                let artist = extract_json_string_field(body, "artist")
                    .or_else(|| extract_json_string_field(body, "albumArtist"))
                    .unwrap_or_default();
                let title = extract_json_string_field(body, "title")
                    .or_else(|| extract_json_string_field(body, "album"))
                    .or_else(|| {
                        (!directory.trim().is_empty())
                            .then(|| directory.rsplit('/').next().unwrap_or(&directory).to_owned())
                    })
                    .unwrap_or_else(|| "Manual Import".to_owned());
                let kind =
                    extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_owned());
                let mut library = state.library.write().await;
                let mut runtime = state.runtime.write().await;
                let relay = state.relay.read().await;
                let previous_library = library.clone();
                let previous_runtime = runtime.clone();
                let Some(record) = library.create(artist, title, kind) else {
                    return Ok(routing::service_unavailable_response("library item capacity is full"));
                };
                let item = serde_json::from_str::<serde_json::Value>(&record.json())
                    .unwrap_or_else(|_| serde_json::json!({ "id": record.id }));
                let body = runtime
                    .record_lidarr_manual_import(1, false, directory, vec![item])
                    .to_string();
                if let Some(db) = state.db.as_ref() {
                    let runtime_record = runtime.persistence_record(&relay);
                    if let Err(error) = db
                        .upsert_library_item_and_runtime_compat_state(
                            &persisted_library_item(&record),
                            &runtime_record,
                        )
                        .await
                    {
                        *library = previous_library;
                        *runtime = previous_runtime;
                        return Ok(routing::service_unavailable_response(&format!(
                            "library persistence failed: Lidarr manual import transaction failed: {error}"
                        )));
                    }
                }
                drop(relay);
                drop(runtime);
                drop(library);
                return Ok(routing::accepted_response(body));
            }
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value =
                    runtime.record_lidarr_manual_import(0, true, directory, Vec::new());
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "next_action".to_owned(),
                        serde_json::json!("trigger Lidarr manual import from configured UI"),
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("GET", "/api/musicbrainz/albums/completion") => {
            if route.path.starts_with("/api/v0/") {
                let targets = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/album-target/");
                if !targets.is_empty() {
                    let discovery = state.content_discovery.read().await;
                    let albums = targets
                        .iter()
                        .map(|target| musicbrainz_target_completion_value(target, &discovery))
                        .collect::<Vec<_>>();
                    drop(discovery);
                    return Ok(routing::ok_response(
                        serde_json::json!({"albums": albums}).to_string(),
                    ));
                }
            }
            let library = state.library.read().await;
            let mut value = serde_json::from_str::<serde_json::Value>(
                &library.musicbrainz_completion_json(),
            )
            .unwrap_or_else(|_| serde_json::json!({}));
            let albums = value["completion_status"].clone();
            value["albums"] = albums;
            drop(library);
            Ok(routing::ok_response(value.to_string()))
        }

        ("GET", path)
            if path.starts_with("/api/musicbrainz/artist/")
                && path.ends_with("/discography-coverage") =>
        {
            let Some(artist) = path_segment_between(
                path,
                "/api/musicbrainz/artist/",
                "/discography-coverage",
            ) else {
                return Ok(routing::not_found_response());
            };
            let artist = decoded_path_segment(artist);
            if uuid::Uuid::parse_str(&artist).is_ok() {
                let profile = query_parameter(route.query, "profile")
                    .unwrap_or_else(|| "CoreDiscography".to_owned());
                let force_refresh = query_parameter(route.query, "forceRefresh")
                    .and_then(|value| parse_bool_value(&value))
                    .unwrap_or(false);
                let settings = state.integration_settings.read().await.musicbrainz.clone();
                match musicbrainz_discography_coverage_with_settings(
                    state,
                    &settings,
                    &artist,
                    &profile,
                    force_refresh,
                )
                .await
                {
                    Ok(Some(value)) => return Ok(routing::ok_response(value.to_string())),
                    Ok(None) => return Ok(routing::not_found_response()),
                    Err(error) => {
                        return Ok(routing::service_unavailable_response(&format!(
                            "MusicBrainz coverage lookup failed: {error}"
                        )))
                    }
                }
            }
            let library = state.library.read().await;
            let json = library.discography_coverage_json(&artist);
            drop(library);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/musicbrainz/release-radar/notifications") => {
            if route.path.starts_with("/api/v0/") {
                let unread_only = query_parameter(route.query, "unreadOnly")
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"));
                let mut notifications = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/radar/notification/");
                notifications.retain(|notification| {
                    !unread_only
                        || !notification
                            .get("read")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                });
                notifications.sort_by(|left, right| {
                    right["firstSeenAt"]
                        .as_str()
                        .cmp(&left["firstSeenAt"].as_str())
                        .then_with(|| left["artistId"].as_str().cmp(&right["artistId"].as_str()))
                });
                return Ok(routing::ok_response(
                    serde_json::Value::Array(notifications).to_string(),
                ));
            }
            let wishlist = state.wishlist.read().await;
            let notifications = wishlist
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .map(|item| {
                    serde_json::json!({
                        "id": format!("release-radar-{}", item.id),
                        "artist": item.artist,
                        "title": item.title,
                        "searchText": item.search_text(),
                        "source": "wishlist",
                    })
                })
                .collect::<Vec<_>>();
            drop(wishlist);
            Ok(routing::ok_response(
                serde_json::Value::Array(notifications).to_string(),
            ))
        },
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn route_dispatch_group_7(
    method: &str,
    normalized_path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    route: &routing::ParsedRoute<'_>,
    headers: &RequestSecurityHeaders,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
) -> RouteDispatchResult {
    match (method, normalized_path) {
        ("GET", "/api/musicbrainz/release-radar/subscriptions") => {
            if route.path.starts_with("/api/v0/") {
                let mut subscriptions = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/radar/subscription/");
                subscriptions.sort_by(|left, right| {
                    left["artistName"]
                        .as_str()
                        .map(str::to_ascii_lowercase)
                        .cmp(&right["artistName"].as_str().map(str::to_ascii_lowercase))
                        .then_with(|| {
                            left["artistId"]
                                .as_str()
                                .map(str::to_ascii_lowercase)
                                .cmp(&right["artistId"].as_str().map(str::to_ascii_lowercase))
                        })
                });
                return Ok(routing::ok_response(
                    serde_json::Value::Array(subscriptions).to_string(),
                ));
            }
            let wishlist = state.wishlist.read().await;
            let subscriptions = wishlist
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .map(|item| {
                    serde_json::json!({
                        "id": format!("wishlist-{}", item.id),
                        "artist": item.artist,
                        "title": item.title,
                        "source": "wishlist",
                    })
                })
                .collect::<Vec<_>>();
            drop(wishlist);
            Ok(routing::ok_response(
                serde_json::Value::Array(subscriptions).to_string(),
            ))
        }

        ("GET", "/api/listening-party") => {
            // Matches the oracle's real ListeningPartyController.List
            // response shape (ListeningPartyAnnouncement[]), but built
            // from slskR's own locally-stored listening-party events
            // rather than a real DHT-backed cross-peer directory --
            // slskR is single-peer-per-instance and has no such mesh
            // discovery wired in for this feature yet. This is an
            // honest, local-only simplification (every entry reflects a
            // real, currently-active local listen-along event), not the
            // previous behavior of listing unrelated joined chat rooms.
            const ANNOUNCEMENT_TTL_MS: u64 = 900_000;
            let now_ms = unix_timestamp_millis();
            let events = state
                .controller_features
                .read()
                .await
                .values_with_prefix("listening-party/");
            let mut announcements = Vec::new();
            for event in events {
                if !event
                    .get("listed")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    continue;
                }
                let started_at = event
                    .get("serverTimeUnixMs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let last_seen = now_ms;
                let expires_at = last_seen.saturating_add(ANNOUNCEMENT_TTL_MS);
                if expires_at <= now_ms {
                    continue;
                }
                let party_id = event
                    .get("partyId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let content_id = event
                    .get("contentId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let allow_mesh_streaming = event
                    .get("allowMeshStreaming")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let stream_path = if allow_mesh_streaming {
                    issue_listening_party_stream_ticket(state, party_id, content_id)
                        .await
                        .map(|ticket| {
                            format!(
                                "/api/v0/listening-party/radio/{}/{}?ticket={}",
                                url_encode(party_id),
                                url_encode(content_id),
                                url_encode(&ticket)
                            )
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                announcements.push(serde_json::json!({
                    "kind": "slskdn.listeningParty.announce.v1",
                    "partyId": event.get("partyId").cloned().unwrap_or_default(),
                    "podId": event.get("podId").cloned().unwrap_or_default(),
                    "channelId": event.get("channelId").cloned().unwrap_or_default(),
                    "hostPeerId": event.get("hostPeerId").cloned().unwrap_or_default(),
                    "title": event.get("title").cloned().unwrap_or_default(),
                    "artist": event.get("artist").cloned().unwrap_or_default(),
                    "album": event.get("album").cloned().unwrap_or(serde_json::Value::Null),
                    "contentId": event.get("contentId").cloned().unwrap_or_default(),
                    "description": event.get("description").cloned().unwrap_or_default(),
                    "tags": event.get("tags").cloned().unwrap_or_else(|| serde_json::json!([])),
                    "allowMeshStreaming": allow_mesh_streaming,
                    "streamPath": stream_path,
                    "startedAtUnixMs": started_at,
                    "expiresAtUnixMs": expires_at,
                    "lastSeenUnixMs": last_seen,
                }));
            }
            announcements.sort_by(|left, right| {
                right["lastSeenUnixMs"]
                    .as_u64()
                    .cmp(&left["lastSeenUnixMs"].as_u64())
            });
            Ok(routing::ok_response(
                serde_json::Value::Array(announcements).to_string(),
            ))
        }

        ("POST", "/api/nowplaying") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let mut now_playing = state.now_playing.write().await;
            let previous = now_playing.clone();
            let record = now_playing.upsert(username, artist, title);
            let mutated = now_playing.clone();
            let json = record.json();
            drop(now_playing);
            if let Err(error) = persist_now_playing_checked(state, &record).await {
                let mut now_playing = state.now_playing.write().await;
                if *now_playing == mutated {
                    *now_playing = previous;
                }
                drop(now_playing);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/nowplaying") if route.path.starts_with("/api/v0/") => {
            let now_playing = state.now_playing.read().await;
            let Some(record) = now_playing
                .records
                .iter()
                .max_by_key(|record| record.updated_at)
            else {
                return Ok(routing::no_content_response());
            };
            let started_at = chrono::DateTime::<chrono::Utc>::from_timestamp(
                i64::try_from(record.updated_at).unwrap_or(i64::MAX),
                0,
            )
            .map(|timestamp| timestamp.to_rfc3339());
            Ok(routing::ok_response(
                serde_json::json!({
                    "artist": record.artist,
                    "title": record.title,
                    "album": null,
                    "startedAt": started_at,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/nowplaying") => {
            let now_playing = state.now_playing.read().await;
            let json = now_playing.json();
            drop(now_playing);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/relay") => {
            let relay_enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
            let json = match mutate_runtime_compat_state(state, |_, relay| {
                relay.set_enabled(relay_enabled).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        // ADDITIONAL MISSING POST ENDPOINTS (Phase 6)
        ("POST", "/api/destinations/validate") => {
            let destination = if route.path.starts_with("/api/v0/") {
                let path = serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|value| {
                        value
                            .as_object()
                            .and_then(|object| json_object_field_ci(object, "path"))
                            .and_then(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|path| !path.is_empty())
                            .map(ToOwned::to_owned)
                    });
                let Some(path) = path else {
                    return Ok(routing::bad_request_response("Path is required"));
                };
                path
            } else {
                extract_json_string_field(body, "destination")
                    .or_else(|| extract_json_string_field(body, "path"))
                    .or_else(|| extract_json_string_field(body, "url"))
                    .unwrap_or_default()
            };
            let destinations = state.destinations.read().await;
            let normalized_path = if destinations.records.is_empty() {
                DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                )
                .normalize_explicit_path(&destination)
            } else {
                destinations.normalize_explicit_path(&destination)
            };
            let matched = destinations
                .records
                .iter()
                .find(|record| {
                    record.path == destination
                        || record.name.eq_ignore_ascii_case(destination.trim())
                        || record.id.eq_ignore_ascii_case(destination.trim())
                })
                .cloned();
            let default = destinations
                .records
                .iter()
                .find(|record| record.is_default)
                .cloned();
            let known_count = destinations.records.len();
            drop(destinations);
            if route.path.starts_with("/api/v0/") {
                let path = normalized_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| destination.clone());
                let exists = normalized_path.as_deref().is_some_and(Path::is_dir);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "path": path,
                        "exists": exists,
                        "writable": exists
                            && normalized_path
                                .as_deref()
                                .is_some_and(directory_is_writable),
                    })
                    .to_string(),
                ));
            }
            Ok(routing::ok_response(serde_json::json!({
                 "destination": destination,
                 "valid": !destination.trim().is_empty(),
                 "known": matched.is_some(),
                 "knownCount": known_count,
                 "matched": matched.map(|record| serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({ "id": record.id }))),
                 "default": default.map(|record| serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({ "id": record.id }))),
             }).to_string()))
        }

        ("POST", "/api/profile/invite") => {
            if route.path.starts_with("/api/v0/") {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Object(_)) => {}
                    Ok(serde_json::Value::Null) | Err(_) => {
                        return Ok(routing::bad_request_response("Request is required."));
                    }
                    Ok(_) => return Ok(routing::bad_request_response("Request is required.")),
                }
                let expires_in_hours = extract_json_i32_field(body, "expiresInHours")
                    .filter(|value| *value > 0)
                    .unwrap_or(24);
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                let session = state.session.read().await;
                let display_name = session
                    .username
                    .clone()
                    .or_else(|| state.config.username.clone())
                    .unwrap_or_else(|| "local".to_owned());
                drop(session);
                let expires_at =
                    chrono::Utc::now() + chrono::Duration::hours(i64::from(expires_in_hours));
                let profile_peer_id = local_profile_peer_id(state);
                let invite = serde_json::json!({
                    "InviteVersion": 1,
                    "Profile": {
                        "PeerId": profile_peer_id.clone(),
                        "PublicKey": STANDARD.encode(descriptor.public_key),
                        "DisplayName": display_name,
                        "Avatar": null,
                        "Capabilities": 0,
                        "Endpoints": descriptor.endpoints,
                        "CreatedAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "ExpiresAt": unix_seconds_rfc3339(descriptor.expires_at_unix),
                        "Signature": descriptor.signature.map(|signature| STANDARD.encode(signature)),
                    },
                    "Nonce": uuid::Uuid::new_v4().simple().to_string(),
                    "ExpiresAt": expires_at.to_rfc3339(),
                    "InviteSignature": null,
                });
                let encoded = STANDARD_NO_PAD
                    .encode(invite.to_string())
                    .replace('+', "-")
                    .replace('/', "_");
                let friend_code = profile_friend_code(&profile_peer_id);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "inviteLink": format!("slskdn://invite/{encoded}"),
                        "friendCode": friend_code,
                    })
                    .to_string(),
                ));
            }
            let invite_state = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_profile_invite()
            })
            .await
            {
                Ok(invite_state) => invite_state,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            let count = invite_state
                .get("count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let updated_at = invite_state
                .get("updated_at")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_else(unix_timestamp);
            Ok(routing::created_response(
                serde_json::json!({
                    "invite": format!("local-{count}"),
                    "created_at": updated_at,
                    "count": count,
                    "persisted": true,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/musicbrainz/release-radar/subscriptions") => {
            if route.path.starts_with("/api/v0/") {
                let subscription = match radar_subscription_from_body(body) {
                    Ok(subscription) => subscription,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                let id = subscription["id"].as_str().unwrap_or_default().to_owned();
                return Ok(
                    match state.controller_features.write().await.upsert(
                        format!("musicbrainz/radar/subscription/{id}"),
                        subscription.clone(),
                    ) {
                        Ok(()) => routing::ok_response(subscription.to_string()),
                        Err(error) => routing::service_unavailable_response(&error),
                    },
                );
            }
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let item = match wishlist.add_item(artist, title, "MusicBrainzReleaseRadar".to_owned())
            {
                Ok(item) => item,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "wishlist item capacity is full",
                    ));
                }
            };
            let json = item.json();
            let count = wishlist
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .count();
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(serde_json::json!({
                 "subscriptions": [serde_json::from_str::<serde_json::Value>(&json).unwrap_or_else(|_| serde_json::json!({}))],
                 "created": true,
                 "persisted": true,
                 "status": "local",
                 "count": count,
             }).to_string()))
        }

        ("POST", "/api/musicbrainz/targets") => {
            if route.path.starts_with("/api/v0/")
                && extract_json_string_field(body, "target").is_none()
                && extract_json_string_field(body, "mbid").is_none()
                && extract_json_string_field(body, "artist").is_none()
                && extract_json_string_field(body, "title").is_none()
                && extract_json_string_field(body, "release").is_none()
                && extract_json_string_field(body, "releaseId").is_none()
                && extract_json_string_field(body, "recordingId").is_none()
                && extract_json_string_field(body, "discogsReleaseId").is_none()
            {
                return Ok(routing::not_found_response());
            }

            if route.path.starts_with("/api/v0/") {
                let request = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Object(request)) => request,
                    _ => return Ok(routing::bad_request_response("request body is required")),
                };
                let release_id = request
                    .get("releaseId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                let recording_id = request
                    .get("recordingId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                let discogs_release_id = request
                    .get("discogsReleaseId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                let settings = state.integration_settings.read().await.musicbrainz.clone();

                let mut album = None;
                if let Some(release_id) = release_id.as_deref() {
                    let cached_album = state
                        .controller_features
                        .read()
                        .await
                        .get(&format!("musicbrainz/album-target/{release_id}"))
                        .cloned();
                    album = match musicbrainz_album_target_with_settings(&settings, release_id)
                        .await
                    {
                        Ok(album) => album,
                        Err(error) => {
                            if cached_album.is_some() {
                                ::tracing::warn!(
                                    release_id,
                                    error = %error,
                                    "MusicBrainz release lookup failed; serving cached album target"
                                );
                                cached_album.clone()
                            } else {
                                return Ok(routing::service_unavailable_response(&format!(
                                    "MusicBrainz lookup failed: {error}"
                                )));
                            }
                        }
                    };
                    if album.is_none() {
                        album = cached_album;
                    }
                } else if let Some(discogs_release_id) = discogs_release_id.as_deref() {
                    album = match musicbrainz_discogs_album_target_with_settings(
                        &settings,
                        discogs_release_id,
                    )
                    .await
                    {
                        Ok(album) => album,
                        Err(error) => {
                            return Ok(routing::service_unavailable_response(&format!(
                                "MusicBrainz lookup failed: {error}"
                            )))
                        }
                    };
                }

                let track = if album.is_none() {
                    if let Some(recording_id) = recording_id.as_deref() {
                        let cached_track = state
                            .controller_features
                            .read()
                            .await
                            .get(&format!("musicbrainz/recording-target/{recording_id}"))
                            .cloned();
                        match musicbrainz_recording_target_with_settings(&settings, recording_id).await {
                            Ok(track) => track,
                            Err(error) => {
                                if cached_track.is_some() {
                                    ::tracing::warn!(
                                        recording_id,
                                        error = %error,
                                        "MusicBrainz recording lookup failed; serving cached track target"
                                    );
                                    cached_track.clone()
                                } else {
                                    return Ok(routing::service_unavailable_response(&format!(
                                        "MusicBrainz lookup failed: {error}"
                                    )));
                                }
                            }
                        }
                        .or(cached_track)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if album.is_none() && track.is_none() {
                    return Ok(routing::not_found_response());
                }

                if let Some(album) = album.as_ref() {
                    let release_id = album
                        .get("musicBrainzReleaseId")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    if !release_id.is_empty() {
                        if let Err(error) = state.controller_features.write().await.upsert(
                            format!("musicbrainz/album-target/{release_id}"),
                            album.clone(),
                        ) {
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                }
                if let (Some(recording_id), Some(track)) = (recording_id.as_deref(), track.as_ref())
                {
                    if let Err(error) = state.controller_features.write().await.upsert(
                        format!("musicbrainz/recording-target/{recording_id}"),
                        track.clone(),
                    ) {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }

                return Ok(routing::ok_response(
                    serde_json::json!({"album": album, "track": track}).to_string(),
                ));
            }
            let target = extract_json_string_field(body, "target")
                .or_else(|| extract_json_string_field(body, "mbid"))
                .or_else(|| extract_json_string_field(body, "artist"))
                .unwrap_or_default();
            let title = extract_json_string_field(body, "title")
                .or_else(|| extract_json_string_field(body, "release"))
                .unwrap_or_default();
            if target.trim().is_empty() && title.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "target/artist or title is required",
                ));
            }
            let mut library = state.library.write().await;
            let previous = library.clone();
            let Some(record) =
                library.create(target.clone(), title, "MusicBrainzTarget".to_owned())
            else {
                return Ok(routing::service_unavailable_response(
                    "library item capacity is full",
                ));
            };
            let target_projection = library.target_json(&target);
            let mutated = library.clone();
            drop(library);
            if let Err(error) = persist_library_item_checked(state, &record).await {
                rollback_library_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(serde_json::json!({
                 "target": target,
                 "created": true,
                 "item": serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({})),
                 "projection": serde_json::from_str::<serde_json::Value>(&target_projection).unwrap_or_else(|_| serde_json::json!({})),
             }).to_string()))
        }

        ("POST", path)
            if path.starts_with("/api/wishlist/")
                && path.ends_with("/search")
                && wishlist_search_item_id(path).is_some() =>
        {
            let item_id = wishlist_search_item_id(path).expect("guarded wishlist search path");
            let wishlist = state.wishlist.read().await;
            let Some(item) = wishlist.get_item(item_id) else {
                drop(wishlist);
                return Ok(routing::not_found_response());
            };
            let query = item.search_text();
            drop(wishlist);
            if query.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "wishlist item has no search text",
                ));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let outcome = match searches.create_scheduled_wishlist_for_item(
                query,
                Some(item_id.to_owned()),
                DEFAULT_SEARCH_TTL_SECONDS,
            ) {
                Ok(outcome) => outcome,
                Err(error) => return Ok(search_create_error_response(error)),
            };
            let record = outcome.record;
            let evicted = outcome.evicted;
            let response = serde_json::json!({
                "item_id": item_id,
                "search_started": true,
                "status": "searching",
                "search_id": record.id,
                "token": record.token,
                "query": record.query,
                "target": record.target,
            })
            .to_string();
            let mutated_searches = searches.clone();
            drop(searches);
            if let Err(error) = delete_persisted_searches(state, &evicted).await {
                rollback_searches_if_unchanged(state, previous_searches.clone(), &mutated_searches)
                    .await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            if let Err(error) = persist_search_record(state, &record).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            session_command_permit.send(SessionCommand::Search {
                token: record.token,
                query: record.query.clone(),
                target: SearchDispatchTarget::Wishlist,
            });
            record_event(state, "search.started", record.token.to_string(), None).await;
            Ok(if route.path.starts_with("/api/v0/") {
                routing::ok_response(response)
            } else {
                routing::accepted_response(response)
            })
        }

        ("POST", "/api/wishlist/import/csv") => {
            if route.path.starts_with("/api/v0/") {
                return Ok(versioned_wishlist_csv_import_response(body, state).await);
            }
            let raw = extract_json_string_field(body, "csv")
                .or_else(|| extract_json_string_field(body, "text"))
                .or_else(|| extract_json_string_field(body, "content"))
                .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
            let parsed_items = raw
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .enumerate()
                .filter_map(|(index, line)| {
                    let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
                    if index == 0
                        && parts
                            .first()
                            .is_some_and(|value| value.eq_ignore_ascii_case("artist"))
                    {
                        return None;
                    }
                    let (artist, title) = if parts.len() >= 2 {
                        (parts[0].to_owned(), parts[1].to_owned())
                    } else if let Some((artist, title)) = line.split_once(" - ") {
                        (artist.trim().to_owned(), title.trim().to_owned())
                    } else {
                        (String::new(), line.to_owned())
                    };
                    Some((artist, title, "Audio".to_owned()))
                })
                .collect::<Vec<_>>();
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            if !wishlist.can_add_items(parsed_items.len()) {
                return Ok(routing::service_unavailable_response(
                    "wishlist item capacity is full",
                ));
            }
            let mut imported = Vec::new();
            let mut persisted_items = Vec::new();
            for (artist, title, kind) in parsed_items {
                let item = wishlist
                    .add_item(artist, title, kind)
                    .map_err(|_| "wishlist capacity changed unexpectedly".to_owned())?;
                let value = serde_json::from_str::<serde_json::Value>(&item.json())
                    .unwrap_or_else(|_| serde_json::json!({ "id": item.id }));
                persisted_items.push(item);
                imported.push(value);
            }
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_items_checked(state, &persisted_items).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(
                serde_json::json!({
                    "imported": imported.len(),
                    "items": imported,
                })
                .to_string(),
            ))
        }

        ("POST", path)
            if path.starts_with("/api/share-grants/")
                && path.ends_with("/backfill")
                && share_grant_helper_id(path, "backfill").is_some() =>
        {
            let grant_id =
                share_grant_helper_id(path, "backfill").expect("guarded share-grant backfill path");
            let versioned = route.path.starts_with("/api/v0/");
            if versioned && state.share_grants.read().await.get(grant_id).is_none() {
                return Ok(routing::not_found_response());
            }
            let share_grants = state.share_grants.read().await;
            let collections = state.collections.read().await;
            let grant = share_grants.get(grant_id);
            if versioned {
                let Some(grant_record) = grant.as_ref() else {
                    drop(collections);
                    drop(share_grants);
                    return Ok(routing::not_found_response());
                };
                if !share_grant_allows_download(&grant_record.permissions) {
                    drop(collections);
                    drop(share_grants);
                    return Ok(routing::forbidden_response(
                        "Download not allowed for this share",
                    ));
                }
                let Some(collection) = collections.get(&grant_record.collection_id) else {
                    drop(collections);
                    drop(share_grants);
                    return Ok(routing::not_found_response());
                };
                if collection.items.is_empty() {
                    drop(collections);
                    drop(share_grants);
                    return Ok(routing::ok_response(
                        serde_json::json!({
                            "enqueued": 0,
                            "failed": 0,
                            "total": 0,
                            "message": "No items to backfill",
                        })
                        .to_string(),
                    ));
                }
            }
            let backfilled = grant
                .as_ref()
                .and_then(|grant| collections.get(&grant.collection_id))
                .map(|collection| collection.items.len())
                .unwrap_or(0);
            let persisted = grant.is_some();
            drop(collections);
            drop(share_grants);
            Ok(routing::accepted_response(
                serde_json::json!({
                    "grant_id": grant_id,
                    "backfilled": backfilled,
                    "persisted": persisted,
                    "status": if persisted { "local" } else { "compatibility_acknowledgement" },
                })
                .to_string(),
            ))
        }

        ("POST", path)
            if path.starts_with("/api/share-grants/")
                && path.ends_with("/token")
                && share_grant_helper_id(path, "token").is_some() =>
        {
            let grant_id =
                share_grant_helper_id(path, "token").expect("guarded share-grant token path");
            if route.path.starts_with("/api/v0/")
                && state.share_grants.read().await.get(grant_id).is_none()
            {
                return Ok(routing::not_found_response());
            }
            if route.path.starts_with("/api/v0/") && !body.trim().is_empty() {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(value) if value.is_object() || value.is_null() => {}
                    _ => {
                        return Ok(routing::bad_request_response("The request body is invalid"));
                    }
                }
            }
            let share_grants = state.share_grants.read().await;
            let grant = share_grants.get(grant_id);
            drop(share_grants);
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            // Matches the oracle's real CreateToken: "Caller must own
            // the collection."
            if let Some(collection_id) = grant.as_ref().map(|grant| grant.collection_id.as_str()) {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
                    return Ok(routing::not_found_response());
                }
            }
            let ttl_seconds = extract_json_u64_field(body, "expiresInSeconds")
                .or_else(|| extract_json_u64_field(body, "expires_in_seconds"))
                .unwrap_or(DEFAULT_SHARE_ACCESS_TOKEN_TTL_SECONDS)
                .clamp(1, MAX_SHARE_ACCESS_TOKEN_TTL_SECONDS);
            let issued = if grant.is_some() {
                let mut tokens = state.share_access_tokens.write().await;
                let issued = tokens.issue(grant_id.to_owned(), ttl_seconds);
                drop(tokens);
                issued
            } else {
                None
            };
            if grant.is_some() && issued.is_none() {
                return Ok(routing::service_unavailable_response(
                    "share access token capacity is full or secure token generation failed",
                ));
            }
            let created = issued.is_some();
            let mut persisted = false;
            if let Some((token, expires_at)) = issued.as_ref() {
                let digest = share_access_token_digest(token);
                let record = ShareAccessTokenRecord {
                    grant_id: grant_id.to_owned(),
                    expires_at: *expires_at,
                };
                match persist_share_access_token(state, &digest, &record).await {
                    Ok(was_persisted) => persisted = was_persisted,
                    Err(error) => {
                        state
                            .share_access_tokens
                            .write()
                            .await
                            .remove_if_unchanged(&digest, &record);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            }
            let (token, expires_at) = issued
                .map(|(token, expires_at)| (Some(token), Some(expires_at)))
                .unwrap_or((None, None));
            Ok(routing::created_response(serde_json::json!({
                 "grant_id": grant_id,
                 "token": token,
                 "expiresAt": expires_at,
                 "expiresInSeconds": if created { Some(ttl_seconds) } else { None },
                 "created": created,
                 "persisted": persisted,
                 "status": if persisted { "persistent_token" } else if created { "ephemeral_compatibility_token" } else { "compatibility_acknowledgement" },
             }).to_string()))
        }

        ("GET", "/solid/clientid.jsonld") => Ok(solid_client_id_document_response(state).await),

        ("GET", "/api/slskdn") => {
            let session = state.session.read().await;
            let shares = state.shares.read().await;
            let searches = state.searches.read().await;
            let transfers = state.transfers.read().await;
            let users = state.users.read().await;
            let rooms = state.rooms.read().await;
            let library = state.library.read().await;
            let body = serde_json::json!({
                "status": "local",
                "enabled": true,
                "connected": session.state == "connected",
                "shares": shares.entries.len(),
                "searches": searches.records.len(),
                "transfers": transfers.entries.len(),
                "users": users.records.len(),
                "rooms": rooms.records.len(),
                "libraryItems": library.records.len(),
            })
            .to_string();
            drop(library);
            drop(rooms);
            drop(users);
            drop(transfers);
            drop(searches);
            drop(shares);
            drop(session);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/slskdn/library/health") => {
            let limit = match query_parameter(route.query, "limit") {
                None => 100,
                Some(value) => match value.parse::<i64>() {
                    Ok(value) if (1..=250).contains(&value) => value as usize,
                    _ => {
                        return Ok(routing::bad_request_response(
                            "limit must be between 1 and 250",
                        ))
                    }
                },
            };
            let path_filter = query_parameter(route.query, "path")
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.trim().to_owned());
            let library = state.library.read().await;
            let all_issues = library.health_issues();
            let total_issues = all_issues.len();
            let issues = all_issues
                .into_iter()
                .take(limit)
                .map(|issue| {
                    serde_json::json!({
                        "type": "MissingMetadata",
                        "file": "",
                        "mb_recording_id": "",
                        "reason": issue
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Library metadata is incomplete"),
                        "severity": "Medium",
                    })
                })
                .collect::<Vec<_>>();
            let body = serde_json::json!({
                "path": path_filter.unwrap_or_else(|| "(all)".to_owned()),
                "summary": {
                    "total_issues": total_issues,
                    "issues_open": total_issues,
                    "issues_resolved": 0,
                },
                "issues": issues,
            })
            .to_string();
            drop(library);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/slskdn/warm-cache") => {
            let shares = state.shares.read().await;
            let searches = state.searches.read().await;
            let library = state.library.read().await;
            let warmed = shares.entries.len() + searches.records.len() + library.records.len();
            drop(library);
            drop(searches);
            drop(shares);
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_cache_warm(warmed).to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("POST", path)
            if path.starts_with("/api/streams/")
                && path.ends_with("/share-ticket")
                && share_stream_content_id(path).is_some() =>
        {
            let content_id =
                share_stream_content_id(path).expect("guarded share stream ticket path");
            let Some(token) = request_share_token(authorization, &headers) else {
                return Ok(routing::unauthorized_response());
            };
            let mut tokens = state.share_access_tokens.write().await;
            let token_record = tokens.validate(&token);
            drop(tokens);
            let Some(token_record) = token_record else {
                return Ok(routing::unauthorized_response());
            };
            let grants = state.share_grants.read().await;
            let Some(grant) = grants.get(&token_record.grant_id) else {
                drop(grants);
                return Ok(routing::unauthorized_response());
            };
            drop(grants);
            let collections = state.collections.read().await;
            let Some(collection) = collections.get(&grant.collection_id) else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            let Some(item) = collection
                .items
                .iter()
                .find(|item| item.content_id == content_id)
                .cloned()
            else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            drop(collections);
            let filename = if item.title.trim().is_empty() {
                item.content_id.clone()
            } else {
                item.title.clone()
            };
            let mut tickets = state.stream_tickets.write().await;
            let Some((ticket, _)) = tickets.issue(
                "share",
                &format!("share:{}", grant.id),
                item.content_id,
                filename.clone(),
                Some(grant.username),
                0,
                preview_stream_content_type(&filename).to_owned(),
                SHARE_STREAM_TICKET_TTL_SECONDS,
            ) else {
                return Ok(routing::service_unavailable_response(
                    "share stream ticket capacity is full",
                ));
            };
            drop(tickets);
            Ok(routing::ok_response(
                serde_json::json!({
                    "ticket": ticket,
                    "expiresInSeconds": SHARE_STREAM_TICKET_TTL_SECONDS,
                })
                .to_string(),
            ))
        }

        ("GET" | "HEAD", path) if path.starts_with("/api/streams/") && path.len() > 13 => {
            let stream_id = decoded_path_segment(&path[13..]);
            if query_parameter(route.query, "token").is_some() {
                return Ok(routing::bad_request_response(
                    "share tokens must be exchanged for stream tickets",
                ));
            }
            let api_authorized = !state.config.auth_required
                || is_authorized(&state.config, authorization, headers.cookie.as_deref());
            let ticket = query_parameter(route.query, "ticket");
            let ticket_record = if let Some(ticket) = ticket.as_deref() {
                let mut tickets = state.stream_tickets.write().await;
                let record = tickets.get(ticket);
                drop(tickets);
                record.filter(|record| record.family == "share" && record.content_id == stream_id)
            } else {
                None
            };
            if ticket.is_some() && ticket_record.is_none() {
                return Ok(routing::unauthorized_response());
            }
            let transfers = state.transfers.read().await;
            let shares = state.shares.read().await;
            let transfer = stream_id
                .strip_prefix("transfer-")
                .and_then(|id| id.parse::<u64>().ok())
                .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id));
            let share = shares.entries.iter().find(|entry| {
                entry.filename == stream_id
                    || stable_content_hash(&entry.filename, entry.size).to_string() == stream_id
            });
            if !api_authorized && ticket_record.is_none() {
                drop(shares);
                drop(transfers);
                return Ok(routing::unauthorized_response());
            }
            let body = serde_json::json!({
                "id": stream_id,
                "status": if transfer.is_some() || share.is_some() || ticket_record.is_some() { "available" } else { "not_found" },
                "ticket": ticket.as_ref().map(|_| "accepted"),
                "transfer": transfer.map(|entry| serde_json::json!({
                    "id": entry.id,
                    "filename": entry.filename,
                    "bytesTransferred": entry.bytes_transferred,
                    "size": entry.size,
                    "state": entry.status,
                })),
                "share": share.map(|entry| serde_json::json!({
                    "filename": entry.filename,
                    "size": entry.size,
                    "extension": entry.extension,
                })),
            }).to_string();
            drop(shares);
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/peer-streams/tickets") | ("POST", "/api/mesh-streams/tickets") => {
            let family = if normalized_path.starts_with("/api/mesh-streams") {
                "mesh"
            } else {
                "peer"
            };
            match create_preview_stream_ticket(state, family, body).await {
                Ok(ticket) => Ok(routing::ok_response(ticket)),
                Err(error) if error == "preview stream ticket capacity is full" => {
                    Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "text/plain; charset=utf-8",
                        body: if family == "mesh" {
                            "Mesh stream limit reached.".to_owned()
                        } else {
                            "Peer stream limit reached.".to_owned()
                        },
                    })
                }
                Err(error) => Ok(routing::bad_request_response(&error)),
            }
        }

        ("GET", path)
            if path.starts_with("/api/peer-streams/") || path.starts_with("/api/mesh-streams/") =>
        {
            let (family, raw_ticket) = if let Some(ticket) = path.strip_prefix("/api/mesh-streams/")
            {
                ("mesh", ticket)
            } else if let Some(ticket) = path.strip_prefix("/api/peer-streams/") {
                ("peer", ticket)
            } else {
                unreachable!()
            };
            let ticket = decoded_path_segment(raw_ticket);
            match open_preview_stream_ticket(state, family, &ticket).await {
                Some(body) => Ok(routing::ok_response(body)),
                None => Ok(routing::not_found_response()),
            }
        }

        ("POST", "/api/listening-party/radio/party/content") => {
            let room =
                extract_json_string_field(body, "room").unwrap_or_else(|| "radio".to_owned());
            let title = extract_json_string_field(body, "title")
                .unwrap_or_else(|| "party content".to_owned());
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let mut rooms = state.rooms.write().await;
            let Some(room_record) = rooms.join(room.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "room capacity is full",
                ));
            };
            let room_record = rooms
                .add_message(
                    &room,
                    "local".to_owned(),
                    if artist.trim().is_empty() {
                        title.clone()
                    } else {
                        format!("{artist} - {title}")
                    },
                )
                .unwrap_or(room_record);
            let active_count = rooms.records.iter().filter(|room| room.joined).count();
            drop(rooms);
            let mut now_playing = state.now_playing.write().await;
            let playing = now_playing.upsert(room.clone(), artist, title);
            drop(now_playing);
            if let Err(error) = persist_now_playing_checked(state, &playing).await {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
            }
            Ok(routing::accepted_response(serde_json::json!({
                "status": "queued",
                "room": room,
                "activePartyCount": active_count,
                "party": serde_json::from_str::<serde_json::Value>(&room_record.json()).unwrap_or_else(|_| serde_json::json!({})),
                "nowPlaying": serde_json::from_str::<serde_json::Value>(&playing.json()).unwrap_or_else(|_| serde_json::json!({})),
            }).to_string()))
        }

        ("GET", "/api/mesh/health")
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native =>
        {
            let routing_nodes = if let Some(dht) = state.dht.as_ref() {
                serde_json::from_str::<serde_json::Value>(&dht.status_json().await)
                    .ok()
                    .and_then(|value| value["dhtNodeCount"].as_u64())
                    .unwrap_or(0)
            } else {
                0
            };
            let discovery = state.content_discovery.read().await;
            let stored_keys = discovery.hash_entries().len();
            let content_peer_hints = discovery
                .shadow_records()
                .iter()
                .map(|record| record.peer_ids.len())
                .sum::<usize>();
            drop(discovery);
            Ok(routing::ok_response(
                serde_json::json!({
                    "routingNodes": routing_nodes,
                    "storedKeys": stored_keys,
                    "contentPeerHints": content_peer_hints,
                    "generatedAt": chrono::Utc::now().to_rfc3339(),
                })
                .to_string(),
            ))
        }

        ("GET", "/api/mesh/health") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let candidate_count = mesh.candidate_usernames(&users).len();
            let capability_count = mesh.capability_records.len();
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(serde_json::json!({
                "status": if candidate_count > 0 || capability_count > 0 { "ready" } else { "empty" },
                "healthy": true,
                "candidates": candidate_count,
                "capabilities": capability_count,
                "interestTag": MESH_RENDEZVOUS_INTEREST_TAG,
            }).to_string()))
        }

        ("POST", "/api/multisource/swarm")
        | ("POST", "/api/multisource/swarm/async")
        | ("POST", "/api/multisource/download")
            if normalized_path != "/api/multisource/download"
                || serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("sources")
                            .and_then(serde_json::Value::as_array)
                            .cloned()
                    })
                    .is_some_and(|sources| !sources.is_empty()) =>
        {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                if normalized_path == "/api/multisource/download" {
                    return Ok(multisource_versioned_download_response(body));
                }
                return Ok(
                    multisource_versioned_swarm_response(normalized_path, body, state).await,
                );
            }
            let mut request = match serde_json::from_str::<multisource::SwarmRequest>(body) {
                Ok(request) => request,
                Err(_) => return Ok(routing::bad_request_response("invalid swarm request")),
            };
            if request.sources.is_empty() {
                let expected_hash = request.expected_hash.clone().unwrap_or_default();
                request.sources =
                    discover_mesh_range_sources(state, &expected_hash, request.file_size).await;
            }
            if let Err(error) = multisource::validate_request(&mut request) {
                return Ok(routing::bad_request_response(&error));
            }
            let id = uuid::Uuid::new_v4().to_string();
            let relative_path = request.output_path.clone().unwrap_or_else(|| {
                format!("multisource/{id}-{}", virtual_basename(&request.filename))
            });
            let downloads_dir = effective_downloads_dir(state);
            let output_path =
                match safe_download_path(&downloads_dir, &relative_path).and_then(|path| {
                    ensure_scoped_download_path(&downloads_dir, path.to_string_lossy().as_ref())
                }) {
                    Ok(path) => path,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
            let public_output_path = output_path
                .strip_prefix(&downloads_dir)
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .map_err(|_| "multisource output path escaped the download root".to_owned())?;
            let job = multisource::new_job(
                id.clone(),
                &request,
                public_output_path.clone(),
                unix_timestamp(),
            );
            state.multisource.write().await.insert(job.clone());
            let store = Arc::clone(&state.multisource);
            if normalized_path == "/api/multisource/swarm/async" {
                tokio::spawn(multisource::execute(
                    id.clone(),
                    request,
                    output_path,
                    public_output_path,
                    store,
                ));
                return Ok(routing::accepted_response(
                    serde_json::json!({
                        "id": id,
                        "status": "queued",
                        "job": job,
                    })
                    .to_string(),
                ));
            }
            let result =
                multisource::execute(id, request, output_path, public_output_path, store).await;
            Ok(routing::ok_response(
                serde_json::to_string(&result)
                    .map_err(|error| format!("multisource result serialization failed: {error}"))?,
            ))
        }

        ("POST", "/api/multisource/download") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                return Ok(multisource_versioned_download_response(body));
            }
            if route.path.starts_with("/api/v0/")
                && serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("sources")
                            .and_then(serde_json::Value::as_array)
                            .map(Vec::len)
                    })
                    .is_some_and(|count| count < 2)
            {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!("At least 2 verified sources are required").to_string(),
                });
            }
            let filename = extract_json_string_field(body, "filename")
                .or_else(|| extract_json_string_field(body, "path"))
                .unwrap_or_else(|| "multisource-download".to_owned());
            let size = extract_json_u64_field(body, "size");
            let peer = extract_json_string_field(body, "username")
                .or_else(|| extract_json_string_field(body, "peer"));
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(0, peer, filename, None, size);
            let body = serde_json::json!({
                "id": format!("transfer-{}", entry.id),
                "transfer_id": entry.id,
                "status": "queued",
                "job": serde_json::from_str::<serde_json::Value>(&entry.json()).unwrap_or_else(|_| serde_json::json!({})),
            }).to_string();
            drop(transfers);
            Ok(routing::accepted_response(body))
        }

        ("GET", "/api/podcore/content/search") => {
            let params = route.query.map(query_params).unwrap_or_default();
            let query = params
                .iter()
                .find(|(key, _)| key == "query" || key == "q")
                .map(|(_, value)| value.trim().to_owned())
                .unwrap_or_default();
            if query.is_empty() {
                return Ok(routing::bad_request_response("Search query is required"));
            }
            // The oracle's real backend is a live MusicBrainz recording
            // search. Keep the result as the flat `ContentSearchResult[]`
            // contract rather than the old {query, results, count} wrapper.
            let domain = params
                .iter()
                .find(|(key, _)| key == "domain")
                .map(|(_, value)| value.trim().to_owned())
                .filter(|value| !value.is_empty());
            if domain.is_some_and(|domain| !domain.eq_ignore_ascii_case("audio")) {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            let limit = params
                .iter()
                .find(|(key, _)| key == "limit")
                .and_then(|(_, value)| value.parse::<i64>().ok())
                .unwrap_or(20)
                .clamp(1, 100) as usize;
            let settings = state.integration_settings.read().await.musicbrainz.clone();
            let mut hits = musicbrainz_search_recordings(&settings, &query, limit)
                .await
                .unwrap_or_default();
            if hits.is_empty() {
                // A disconnected or empty MusicBrainz backend must not erase
                // local content search results. The compatibility controller
                // has a real local library, so use it as the bounded fallback
                // while preserving the MusicBrainz-backed result shape.
                let query = query.to_ascii_lowercase();
                let library = state.library.read().await;
                hits = library
                    .records
                    .iter()
                    .filter(|record| {
                        record.kind.eq_ignore_ascii_case("audio")
                            && (record.title.to_ascii_lowercase().contains(&query)
                                || record.artist.to_ascii_lowercase().contains(&query))
                    })
                    .take(limit)
                    .map(|record| MusicBrainzRecordingHit {
                        recording_id: record.id.clone(),
                        title: record.title.clone(),
                        artist: record.artist.clone(),
                        artist_id: None,
                    })
                    .collect();
            }
            let results = hits
                .into_iter()
                .filter(|hit| !hit.recording_id.is_empty())
                .map(|hit| {
                    serde_json::json!({
                        "contentId": format!("content:audio:track:{}", hit.recording_id),
                        "title": hit.title,
                        "subtitle": hit.artist,
                        "type": "track",
                        "domain": "audio",
                        "metadata": {
                            "musicbrainz_recording_id": hit.recording_id,
                            "artist": hit.artist,
                            "title": hit.title,
                            "musicbrainz_artist_id": hit.artist_id.unwrap_or_default(),
                        },
                    })
                })
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::Value::Array(results).to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/join") => {
            let input = match PodJoinSignatureInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let now = unix_timestamp();
            let verified = match verify_pod_join_signature(mode, &input, now.saturating_mul(1_000))
            {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if mode == PodSignatureMode::Warn && !verified {
                record_daemon_log(
                    state,
                    logging::LogLevel::Warn,
                    "podcore",
                    "accepted unsigned or legacy pod join in warn mode".to_owned(),
                )
                .await;
            }
            if mode == PodSignatureMode::Enforce {
                if let Err(error) = reserve_pod_join_replay_key(state, &input, now).await {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        &error
                    }));
                }
            }
            let pod_exists_and_is_not_member = {
                let pods = state.pods.read().await;
                let pod_state = pods.get(&input.pod_id).map(|_| {
                    pods.members(&input.pod_id).is_none_or(|members| {
                        !members
                            .iter()
                            .any(|member| member.peer_id.eq_ignore_ascii_case(&input.peer_id))
                    })
                });
                drop(pods);
                if pod_state.is_some() {
                    pod_state
                } else {
                    let rooms = state.rooms.read().await;
                    rooms
                        .records
                        .iter()
                        .find(|room| room.name == input.pod_id)
                        .map(|room| {
                            !room
                                .members
                                .iter()
                                .any(|member| member.eq_ignore_ascii_case(&input.peer_id))
                        })
                }
            };
            match pod_exists_and_is_not_member {
                None => {
                    if mode == PodSignatureMode::Enforce {
                        release_pod_join_replay_key(state, &input).await;
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Pod not found"
                    }));
                }
                Some(false) => {
                    if mode == PodSignatureMode::Enforce {
                        release_pod_join_replay_key(state, &input).await;
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Already a member of this pod"
                    }));
                }
                Some(true) => {}
            }
            let mut workflow = state.pod_membership_workflow.write().await;
            if let Err(error) = workflow.add_join(input.clone()) {
                drop(workflow);
                if mode == PodSignatureMode::Enforce {
                    release_pod_join_replay_key(state, &input).await;
                }
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join request could not be processed"
                } else {
                    error
                }));
            }
            drop(workflow);
            let body = serde_json::json!({
                "success": true,
                "podId": input.pod_id,
                "peerId": input.peer_id,
                "joinRequest": input,
                "signatureMode": mode.as_str(),
                "signatureVerified": verified,
            })
            .to_string();
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/podcore/membership/join/accept") => {
            let input = match PodJoinAcceptanceInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.acceptor_public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod join acceptance",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join acceptance could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if !pod_acceptor_has_permission(state, &input.pod_id, &input.acceptor_peer_id).await {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept join requests"
                }));
            }
            let request = {
                state
                    .pod_membership_workflow
                    .write()
                    .await
                    .remove_join(&input.pod_id, &input.peer_id)
            };
            let Some(request) = request else {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join acceptance could not be processed"
                } else {
                    "No pending join request found"
                }));
            };
            let added = state
                .rooms
                .write()
                .await
                .add_member(&input.pod_id, input.peer_id.clone());
            match added {
                Ok(Some(_)) => {}
                Ok(None) => {
                    let _ = state
                        .pod_membership_workflow
                        .write()
                        .await
                        .add_join(request);
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join acceptance could not be processed"
                    } else {
                        "Pod not found"
                    }));
                }
                Err(()) => {
                    let _ = state
                        .pod_membership_workflow
                        .write()
                        .await
                        .add_join(request);
                    return Ok(if request_is_versioned_v0 {
                        routing::bad_request_response("Join acceptance could not be processed")
                    } else {
                        routing::service_unavailable_response("pod member capacity is full")
                    });
                }
            }
            state.pod_membership_workflow.write().await.set_role(
                &input.pod_id,
                &input.peer_id,
                input.accepted_role.clone(),
            );
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "operation": "join_acceptance",
                    "request": request,
                    "response": input,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/leave") => {
            let input = match PodLeaveRequestInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod leave",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave request could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            let is_member = state
                .rooms
                .read()
                .await
                .records
                .iter()
                .find(|room| room.name == input.pod_id)
                .is_some_and(|room| {
                    room.members
                        .iter()
                        .any(|member| member.eq_ignore_ascii_case(&input.peer_id))
                });
            if !is_member {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave request could not be processed"
                } else {
                    "Not a member of this pod"
                }));
            }
            let privileged = matches!(
                state
                    .pod_membership_workflow
                    .read()
                    .await
                    .role(&input.pod_id, &input.peer_id),
                "owner" | "moderator"
            );
            if privileged {
                if let Err(error) = state
                    .pod_membership_workflow
                    .write()
                    .await
                    .add_leave(input.clone())
                {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave request could not be processed"
                    } else {
                        error
                    }));
                }
            } else {
                state
                    .rooms
                    .write()
                    .await
                    .remove_member(&input.pod_id, &input.peer_id);
                state
                    .pod_membership_workflow
                    .write()
                    .await
                    .remove_role(&input.pod_id, &input.peer_id);
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "leaveRequest": input,
                    "pending": privileged,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/leave/accept") => {
            let input = match PodLeaveAcceptanceInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.acceptor_public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod leave acceptance",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave acceptance could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if !pod_acceptor_has_permission(state, &input.pod_id, &input.acceptor_peer_id).await {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept leave requests"
                }));
            }
            let request = state
                .pod_membership_workflow
                .write()
                .await
                .remove_leave(&input.pod_id, &input.peer_id);
            let Some(request) = request else {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "No pending leave request found"
                }));
            };
            if state
                .rooms
                .write()
                .await
                .remove_member(&input.pod_id, &input.peer_id)
                .is_none()
            {
                let _ = state
                    .pod_membership_workflow
                    .write()
                    .await
                    .add_leave(request);
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "Not a member of this pod"
                }));
            }
            state
                .pod_membership_workflow
                .write()
                .await
                .remove_role(&input.pod_id, &input.peer_id);
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "operation": "leave_acceptance",
                    "request": request,
                    "response": input,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("GET", path) if pod_pending_request_has_blank_id(path, "join") => {
            Ok(routing::bad_request_response("PodId is required"))
        }

        ("GET", path) if pod_pending_request_has_blank_id(path, "leave") => {
            Ok(routing::bad_request_response("PodId is required"))
        }

        ("GET", path) if pod_pending_request_path(path, "join").is_some() => {
            let pod_id = pod_pending_request_path(path, "join").unwrap_or_default();
            let workflow = state.pod_membership_workflow.read().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "pendingJoinRequests": workflow.pending_joins(&pod_id),
                })
                .to_string(),
            ))
        }

        ("GET", path) if pod_pending_request_path(path, "leave").is_some() => {
            let pod_id = pod_pending_request_path(path, "leave").unwrap_or_default();
            let workflow = state.pod_membership_workflow.read().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "pendingLeaveRequests": workflow.pending_leaves(&pod_id),
                })
                .to_string(),
            ))
        }

        ("DELETE", path) if pod_cancel_request_path(path, "join").is_some() => {
            let (pod_id, peer_id) = pod_cancel_request_path(path, "join").unwrap_or_default();
            if state
                .pod_membership_workflow
                .write()
                .await
                .remove_join(&pod_id, &peer_id)
                .is_some()
            {
                Ok(routing::ok_response(r#"{"cancelled":true}"#.to_owned()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        ("DELETE", path) if pod_cancel_request_path(path, "leave").is_some() => {
            let (pod_id, peer_id) = pod_cancel_request_path(path, "leave").unwrap_or_default();
            if state
                .pod_membership_workflow
                .write()
                .await
                .remove_leave(&pod_id, &peer_id)
                .is_some()
            {
                Ok(routing::ok_response(r#"{"cancelled":true}"#.to_owned()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        ("GET", "/api/playback/status") => {
            let now_playing = state.now_playing.read().await;
            let body = serde_json::json!({
                "status": if now_playing.records.is_empty() { "stopped" } else { "playing" },
                "nowPlaying": now_playing.records.iter().map(|record| {
                    serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({}))
                }).collect::<Vec<_>>(),
                "count": now_playing.records.len(),
                "updated_at": now_playing.updated_at,
            }).to_string();
            drop(now_playing);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/traces") => {
            let events = state.events.read().await;
            let traces = events
                .records
                .iter()
                .rev()
                .take(100)
                .map(|event| {
                    serde_json::json!({
                        "id": event.id,
                        "kind": event.kind,
                        "resource": event.resource,
                        "detail": event.detail,
                        "created_at": event.created_at,
                    })
                })
                .collect::<Vec<_>>();
            let count = traces.len();
            drop(events);
            Ok(routing::ok_response(
                serde_json::json!({
                    "traces": traces,
                    "count": count,
                })
                .to_string(),
            ))
        }

        // FairnessController.GetSummary is a versioned native profile DTO, while the
        // unversioned /api/fairness route remains slskR's legacy ranking
        // projection.  With no recorded traffic, the frozen guard returns a
        // neutral upload/download ratio, a zero overlay/Soulseek ratio, and
        // an explicit within-constraints reason.
        ("GET", "/api/fairness")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/fairness/summary" =>
        {
            let totals = if let Some(db) = state.db.as_ref() {
                match db.get_traffic_totals().await {
                    Ok(totals) => totals,
                    Err(error) => {
                        return Ok(routing::internal_server_error_response(&format!(
                            "fairness storage unavailable: {error}"
                        )))
                    }
                }
            } else {
                persistence::TrafficTotalsRecord::default()
            };
            let overlay_upload = totals.overlay_upload_bytes.max(0) as f64;
            let overlay_download = totals.overlay_download_bytes.max(0) as f64;
            let soulseek_upload = totals.soulseek_upload_bytes.max(0) as f64;
            let upload_download_ratio = if overlay_download > 0.0 {
                overlay_upload / overlay_download
            } else {
                1.0
            };
            let overlay_to_soulseek_ratio = if soulseek_upload > 0.0 {
                overlay_upload / soulseek_upload
            } else if overlay_upload > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };
            let mut throttle = upload_download_ratio < 0.5;
            let mut reasons = Vec::new();
            if throttle {
                reasons.push(format!(
                    "overlay upload/download ratio {upload_download_ratio:.2} below minimum 0.50"
                ));
            }
            if overlay_to_soulseek_ratio > 3.0 {
                throttle = true;
                reasons.push(format!(
                    "overlay/Soulseek upload ratio {overlay_to_soulseek_ratio:.2} above maximum 3.00"
                ));
            }
            let reason = if reasons.is_empty() {
                "within fairness constraints".to_owned()
            } else {
                reasons.join("; ")
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "throttleOverlayDownloads": throttle,
                    "reason": reason,
                    "overlayUploadDownloadRatio": upload_download_ratio,
                    "overlayToSoulseekUploadRatio": if overlay_to_soulseek_ratio.is_finite() {
                        serde_json::json!(overlay_to_soulseek_ratio)
                    } else {
                        serde_json::json!("Infinity")
                    },
                    "totals": {
                        "overlayUploadBytes": totals.overlay_upload_bytes.max(0),
                        "overlayDownloadBytes": totals.overlay_download_bytes.max(0),
                        "soulseekUploadBytes": totals.soulseek_upload_bytes.max(0),
                        "soulseekDownloadBytes": totals.soulseek_download_bytes.max(0),
                    },
                })
                .to_string(),
            ))
        }
        ("GET", "/api/fairness") | ("GET", "/api/ranking") => {
            let mut response = native_compat_response(method, normalized_path, state).await;
            if normalized_path == "/api/fairness" {
                let transfers = state.transfers.read().await;
                let downloaded = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction == 0)
                    .map(|entry| entry.bytes_transferred)
                    .sum::<u64>();
                let uploaded = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction != 0)
                    .map(|entry| entry.bytes_transferred)
                    .sum::<u64>();
                let mut value = serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_else(|_| serde_json::json!({}));
                value["throttleOverlayDownloads"] = serde_json::json!(false);
                value["reason"] = serde_json::Value::Null;
                value["overlayUploadDownloadRatio"] = serde_json::json!(0.0);
                value["overlayToSoulseekUploadRatio"] = serde_json::json!(0.0);
                value["totals"] = serde_json::json!({
                    "downloadedBytes": downloaded,
                    "uploadedBytes": uploaded,
                });
                response.body = value.to_string();
            }
            Ok(response)
        }

        ("GET", "/api/port-forwarding/status") => Ok(routing::ok_response(
            serde_json::to_string(&state.port_forwarding.statuses().await)
                .unwrap_or_else(|_| "[]".to_owned()),
        )),

        ("GET", path) if path.starts_with("/api/port-forwarding/status/") => {
            let Some(local_port) = path_segment_after(path, "/api/port-forwarding/status/") else {
                return Ok(routing::not_found_response());
            };
            if local_port.parse::<u16>().is_err() || local_port == "0" {
                return Ok(routing::not_found_response());
            }
            let local_port = local_port.parse::<u16>().unwrap_or_default();
            match state.port_forwarding.status(local_port).await {
                Some(status) => Ok(routing::ok_response(
                    serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_owned()),
                )),
                None => Ok(routing::not_found_response()),
            }
        }

        ("GET", "/api/port-forwarding/available-ports") => {
            let start_port = match query_bounded_usize(route.query, "startPort", 1, 65_535) {
                Ok(value) => value.unwrap_or(1_024),
                Err(()) => {
                    return Ok(routing::bad_request_response("Invalid port range"));
                }
            };
            let end_port = match query_bounded_usize(route.query, "endPort", 1, 65_535) {
                Ok(value) => value.unwrap_or(65_535),
                Err(()) => {
                    return Ok(routing::bad_request_response("Invalid port range"));
                }
            };
            if start_port > end_port {
                return Ok(routing::bad_request_response("Invalid port range"));
            }
            let limit = match query_bounded_usize(route.query, "limit", 1, 1_000) {
                Ok(value) => value,
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "Limit must be between 1 and 1000",
                    ));
                }
            };
            let used_ports = state
                .port_forwarding
                .used_ports()
                .await
                .into_iter()
                .map(usize::from)
                .filter(|port| (start_port..=end_port).contains(port))
                .collect::<HashSet<_>>();
            let available_port_count = end_port - start_port + 1 - used_ports.len();
            let returned_port_count = limit.unwrap_or(1_000).min(available_port_count);
            let available_ports = (start_port..=end_port)
                .filter(|port| !used_ports.contains(port))
                .take(returned_port_count)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({
                    "availablePortCount": available_port_count,
                    "availablePorts": available_ports,
                    "usedPortCount": used_ports.len(),
                })
                .to_string(),
            ))
        }

        ("GET", "/api/port-forwarding/stream-stats") => {
            let rules = state.port_forwarding.statuses().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalForwardingRules": rules.len(),
                    "activeRules": rules.iter().filter(|rule| rule.is_active).count(),
                    "totalConnections": rules.iter().map(|rule| rule.active_connections).sum::<usize>(),
                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
                    "rules": rules,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/port-forwarding/start") => {
            let request = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) if value.is_object() => value,
                _ => return Ok(routing::bad_request_response("Request is required")),
            };
            let local_port = request.get("localPort").and_then(serde_json::Value::as_u64);
            let destination_port = request
                .get("destinationPort")
                .and_then(serde_json::Value::as_u64);
            let pod_id = request
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let destination_host = request
                .get("destinationHost")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let service_name = request
                .get("serviceName")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            if !matches!(local_port, Some(1_024..=65_535)) {
                return Ok(routing::bad_request_response(
                    "Local port must be between 1024 and 65535",
                ));
            }
            if pod_id.is_empty() || pod_id.len() > 100 {
                return Ok(routing::bad_request_response(
                    "PodId must be between 1 and 100 characters",
                ));
            }
            if destination_host.is_empty() || destination_host.len() > 253 {
                return Ok(routing::bad_request_response(
                    "Destination host must be between 1 and 253 characters",
                ));
            }
            if !matches!(destination_port, Some(1..=65_535)) {
                return Ok(routing::bad_request_response(
                    "Destination port must be between 1 and 65535",
                ));
            }
            if service_name.is_some_and(|value| value.len() > 100) {
                return Ok(routing::bad_request_response(
                    "Service name must be at most 100 characters",
                ));
            }
            let local_port = local_port.unwrap_or_default() as u16;
            let destination_port = destination_port.unwrap_or_default() as u16;
            let Some(local_username) = pod_request_peer_id(state).await else {
                return Ok(routing::forbidden_response(
                    "Authenticated peer identity is required",
                ));
            };
            let (pod, pod_gateway_certificate_sha256) = {
                let pods = state.pods.read().await;
                if !pods.is_member(pod_id, &local_username) {
                    return Ok(routing::forbidden_response(
                        "Only pod members can start port forwarding",
                    ));
                }
                if !pods.destination_allowed(pod_id, destination_host, destination_port) {
                    return Ok(routing::forbidden_response(
                        "Destination is not allowed by the Pod private-gateway policy",
                    ));
                }
                (pods.get(pod_id), pods.gateway_certificate_sha256(pod_id))
            };
            let Some(pod) = pod else {
                return Ok(routing::not_found_response());
            };
            let gateway_peer_id = pod
                .private_service_policy
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|policy| policy.get("gatewayPeerId"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let Some(gateway_peer_id) = gateway_peer_id else {
                return Ok(routing::service_unavailable_response(
                    "Pod private-gateway policy has no designated gateway",
                ));
            };
            let trusted_gateway = state
                .config
                .trusted_mesh_peers
                .iter()
                .find(|peer| peer.matches(gateway_peer_id));
            if let (Some(pod_pin), Some(trusted)) =
                (pod_gateway_certificate_sha256, trusted_gateway)
            {
                if pod_pin != trusted.certificate_sha256 {
                    return Ok(routing::service_unavailable_response(
                        "Pod and operator gateway certificate pins conflict",
                    ));
                }
            }
            let gateway_certificate_sha256 = pod_gateway_certificate_sha256
                .or_else(|| trusted_gateway.map(|peer| peer.certificate_sha256));
            let Some(gateway_certificate_sha256) = gateway_certificate_sha256 else {
                return Ok(routing::service_unavailable_response(
                    "Gateway has no authenticated TLS certificate pin",
                ));
            };
            let descriptor = state
                .mesh
                .read()
                .await
                .capability_records
                .iter()
                .find(|descriptor| {
                    descriptor.peer_id.eq_ignore_ascii_case(gateway_peer_id)
                        || descriptor.username.eq_ignore_ascii_case(gateway_peer_id)
                })
                .cloned();
            if descriptor.is_none() && trusted_gateway.is_none() {
                return Ok(routing::service_unavailable_response(
                    "Gateway peer capability record and operator trust entry are unavailable",
                ));
            }
            let gateway_username = descriptor
                .as_ref()
                .map(|descriptor| descriptor.username.clone())
                .or_else(|| trusted_gateway.map(|peer| peer.username.clone()))
                .expect("a descriptor or trusted gateway exists");
            let mut gateway_endpoints = Vec::new();
            if let Some(trusted) = trusted_gateway {
                gateway_endpoints.push(trusted.overlay_endpoint);
            }
            if let Some(descriptor) = descriptor.as_ref() {
                if let Some(overlay_port) = descriptor.overlay_port {
                    if let Some(peer_address) =
                        cached_peer_endpoint(state, &descriptor.username).await
                    {
                        let endpoint = SocketAddr::V4(SocketAddrV4::new(
                            peer_connect_ip(state, &peer_address),
                            overlay_port,
                        ));
                        if !gateway_endpoints.contains(&endpoint) {
                            gateway_endpoints.push(endpoint);
                        }
                    }
                }
            }
            if let Some(dht) = state.dht.as_ref() {
                for endpoint in dht.peers().await {
                    if gateway_endpoints.len() >= port_forwarding::MAX_GATEWAY_ENDPOINTS {
                        break;
                    }
                    if !gateway_endpoints.contains(&endpoint) {
                        gateway_endpoints.push(endpoint);
                    }
                }
            }
            if gateway_endpoints.is_empty() {
                if let Some(descriptor) = descriptor.as_ref() {
                    if let Some(overlay_port) = descriptor.overlay_port {
                        let peer_address =
                            match request_peer_endpoint(state, &descriptor.username).await {
                                Ok(address) => address,
                                Err(error) => {
                                    return Ok(routing::service_unavailable_response(&error));
                                }
                            };
                        gateway_endpoints.push(SocketAddr::V4(SocketAddrV4::new(
                            peer_connect_ip(state, &peer_address),
                            overlay_port,
                        )));
                    }
                }
            }
            if gateway_endpoints.is_empty() {
                return Ok(routing::service_unavailable_response(
                    "Gateway has no reachable overlay endpoint",
                ));
            }
            match state
                .port_forwarding
                .start(port_forwarding::StartRequest {
                    local_port,
                    pod_id: pod_id.to_owned(),
                    destination_host: destination_host.to_owned(),
                    destination_port,
                    service_name: service_name.map(str::to_owned),
                    gateway_username,
                    gateway_endpoints,
                    gateway_certificate_sha256,
                    local_username,
                    authentication_key: Arc::new(state.capability_signing_key.clone()),
                })
                .await
            {
                Ok(_) => Ok(routing::ok_response(
                    r#"{"message":"Port forwarding started"}"#.to_owned(),
                )),
                Err(error) if error.contains("already being forwarded") => {
                    Ok(routing::conflict_response(&error))
                }
                Err(error) => {
                    eprintln!("port forwarding start failed: {error}");
                    Ok(
                        if state.config.controller_profile == ControllerProfile::Native
                            && route.path.starts_with("/api/v0/")
                        {
                            routing::internal_server_error_response(
                                "Failed to start port forwarding",
                            )
                        } else {
                            routing::service_unavailable_response("port forwarding is unavailable")
                        },
                    )
                }
            }
        }

        ("POST", path) if path.starts_with("/api/port-forwarding/stop/") => {
            let Some(local_port) = path_segment_after(path, "/api/port-forwarding/stop/") else {
                return Ok(routing::not_found_response());
            };
            if local_port.parse::<u16>().is_err() || local_port == "0" {
                return Ok(routing::not_found_response());
            }
            state
                .port_forwarding
                .stop(local_port.parse().unwrap_or_default())
                .await;
            Ok(routing::ok_response(
                r#"{"message":"Port forwarding stopped"}"#.to_owned(),
            ))
        }

        ("GET", "/api/portforwarding/status") => Ok(routing::ok_response(
            serde_json::to_string(&state.port_forwarding.statuses().await)
                .unwrap_or_else(|_| "[]".to_owned()),
        )),

        ("GET", "/api/signals") => {
            let session = state.session.read().await;
            let events = state.events.read().await;
            let body = serde_json::json!({
                "signals": events.records.iter().rev().take(25).map(|event| {
                    serde_json::json!({
                        "id": event.id,
                        "kind": event.kind,
                        "resource": event.resource,
                        "created_at": event.created_at,
                    })
                }).collect::<Vec<_>>(),
                "connected": session.state == "connected",
                "count": events.records.len().min(25),
            })
            .to_string();
            drop(events);
            drop(session);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/backfill") => {
            let searches = state.searches.read().await;
            let shares = state.shares.read().await;
            let queued = searches.records.len() + shares.entries.len();
            drop(shares);
            drop(searches);
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_backfill(queued).to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }
        ("GET", path) if path.starts_with("/api/mediacore/") => {
            Ok(mediacore_extended_response(path, route.query, state).await)
        }
        ("GET", path) if extended_controller_get_route(path) => {
            Ok(extended_controller_get_response(
                path,
                route.query,
                state,
                route.path.starts_with("/api/v0/"),
            )
            .await)
        }
        ("GET", path) if extended_controller_dynamic_get_route(path) => {
            Ok(extended_controller_dynamic_get_response(
                path,
                route.query,
                state,
                route.path.starts_with("/api/v0/"),
            )
            .await)
        }
        (method, path) if extended_mutation => Ok(extended_controller_mutation_response(
            method,
            path,
            route.query,
            body,
            state,
            route.path.starts_with("/api/v0/"),
            &headers,
        )
        .await),
        (method, path) if native_compat_route(method, path) => {
            Ok(native_compat_response(method, path, state).await)
        }
        ("GET", path) if is_spa_navigation_path(path) => Ok(index_html_response()),
        ("HEAD", path) if is_spa_navigation_path(path) => Ok(head_response(index_html_response())),
        _ => {
            tracing::complete_request_span(404);
            Ok(routing::unmatched_route_response())
        }
    }
}
