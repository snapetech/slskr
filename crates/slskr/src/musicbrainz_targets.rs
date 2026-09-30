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
        .upsert(cache_key, result.clone())
        .await?;
    Ok(Some(result))
}
