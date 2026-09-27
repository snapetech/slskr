use super::*;

/// Resolves a MusicBrainz release id into a SongID-ready `(artist, title)`
/// pair, matching the oracle's `MusicBrainzClient.GetReleaseAsync` +
/// `MapToAlbumTarget`: `Ok(None)` when the release doesn't exist or has no
/// resolvable artist credit, `Err` only on a genuine transport/parse failure.
#[cfg(any(test, feature = "bounded-differential"))]
#[allow(dead_code)]
pub(super) async fn musicbrainz_release_target(
    base_url: &str,
    release_id: &str,
) -> Result<Option<(String, String)>, String> {
    let settings = MusicBrainzIntegrationSettings {
        base_url: base_url.trim_end_matches('/').to_owned(),
        user_agent: format!("slskR v{APP_VERSION}"),
        timeout_seconds: 10.0,
        retry_attempts: 1,
    };
    musicbrainz_release_target_with_settings(&settings, release_id).await
}

pub(super) async fn musicbrainz_release_target_with_settings(
    settings: &MusicBrainzIntegrationSettings,
    release_id: &str,
) -> Result<Option<(String, String)>, String> {
    let Some(value) = musicbrainz_json_request(
        settings,
        &format!(
            "/release/{}?fmt=json&inc=recordings+artists+labels+discids+isrcs",
            url_encode(release_id.trim())
        ),
    )
    .await?
    else {
        return Ok(None);
    };
    let artist_id = value
        .pointer("/artist-credit/0/artist/id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.trim().is_empty());
    if artist_id.is_none() {
        return Ok(None);
    }
    let title = value
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let artist_name = value
        .pointer("/artist-credit/0/name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Ok(Some((artist_name, title)))
}

#[derive(Clone, Debug)]
pub(super) struct MusicBrainzRecordingHit {
    pub(super) recording_id: String,
    pub(super) title: String,
    pub(super) artist: String,
    pub(super) artist_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct ParsedPodcoreContentId {
    domain: String,
    content_type: String,
    pub(super) domain_lower: String,
    type_lower: String,
    id: String,
    full_id: String,
}

pub(super) fn parse_podcore_content_id(content_id: &str) -> Option<ParsedPodcoreContentId> {
    let content_id = content_id.trim();
    let (prefix, remainder) = content_id.split_once(':')?;
    if !prefix.eq_ignore_ascii_case("content") {
        return None;
    }
    let (domain, remainder) = remainder.split_once(':')?;
    let (content_type, id) = remainder.split_once(':')?;
    if domain.trim().is_empty() || content_type.trim().is_empty() || id.trim().is_empty() {
        return None;
    }
    let domain = domain.to_owned();
    let content_type = content_type.to_owned();
    let id = id.to_owned();
    Some(ParsedPodcoreContentId {
        domain_lower: domain.to_ascii_lowercase(),
        type_lower: content_type.to_ascii_lowercase(),
        full_id: format!("content:{domain}:{content_type}:{id}"),
        domain,
        content_type,
        id,
    })
}

pub(super) fn musicbrainz_query_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub(super) async fn musicbrainz_json_request(
    settings: &MusicBrainzIntegrationSettings,
    path_and_query: &str,
) -> Result<Option<serde_json::Value>, String> {
    let timeout = Duration::try_from_secs_f64(settings.timeout_seconds)
        .map_err(|error| format!("MusicBrainz timeout is invalid: {error}"))?;
    let base_url = reqwest::Url::parse(&settings.base_url)
        .map_err(|error| format!("MusicBrainz URL is invalid: {error}"))?;
    let resolved = validate_lidarr_base_url(base_url.as_str())
        .map_err(|error| format!("MusicBrainz URL is invalid: {error}"))?;
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve_to_addrs(&resolved.host, &resolved.addrs)
        .build()
        .map_err(|error| format!("failed to build MusicBrainz client: {error}"))?;
    let url = format!(
        "{}{}",
        base_url.as_str().trim_end_matches('/'),
        path_and_query
    );

    for attempt in 0..settings.retry_attempts {
        let response = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, &settings.user_agent)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en")
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(_error) if attempt + 1 < settings.retry_attempts => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            Err(error) => {
                return Err(format!("MusicBrainz API request failed: {error}"));
            }
        };
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(format!("MusicBrainz returned HTTP {}", response.status()));
        }
        return read_bounded_integration_json(response, "MusicBrainz API")
            .await
            .map(Some);
    }

    Err("MusicBrainz request did not run".to_owned())
}

fn musicbrainz_artist_credit(value: &serde_json::Value) -> String {
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

fn musicbrainz_artist_id(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/artist-credit/0/artist/id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

pub(super) async fn musicbrainz_search_recordings(
    settings: &MusicBrainzIntegrationSettings,
    query: &str,
    limit: usize,
) -> Result<Vec<MusicBrainzRecordingHit>, String> {
    let Some(value) = musicbrainz_json_request(
        settings,
        &format!(
            "/recording?query={}&fmt=json&limit={}",
            musicbrainz_query_encode(query.trim()),
            limit.clamp(1, 100)
        ),
    )
    .await?
    else {
        return Ok(Vec::new());
    };
    let Some(recordings) = value
        .get("recordings")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut seen = HashSet::new();
    let mut hits = Vec::new();
    for recording in recordings {
        let recording_id = recording
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let title = recording
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let artist = musicbrainz_artist_credit(recording);
        if recording_id.is_empty() && title.is_empty() && artist.is_empty() {
            continue;
        }
        let key = if recording_id.is_empty() {
            format!(
                "{}\u{1f}{}",
                artist.to_ascii_lowercase(),
                title.to_ascii_lowercase()
            )
        } else {
            recording_id.to_ascii_lowercase()
        };
        if !seen.insert(key) {
            continue;
        }
        hits.push(MusicBrainzRecordingHit {
            recording_id,
            title,
            artist,
            artist_id: musicbrainz_artist_id(recording),
        });
    }
    Ok(hits)
}

fn basic_podcore_metadata(
    parsed: &ParsedPodcoreContentId,
    title_override: Option<&str>,
    artist_override: Option<&str>,
    mut additional_info: BTreeMap<String, String>,
) -> serde_json::Value {
    additional_info.insert("id".to_owned(), parsed.id.clone());
    additional_info.insert("domain".to_owned(), parsed.domain.clone());
    additional_info.insert("type".to_owned(), parsed.content_type.clone());
    let default_title = format!("{}: {}", parsed.content_type, parsed.id);
    serde_json::json!({
        "contentId": parsed.full_id,
        "title": title_override.unwrap_or(&default_title),
        "artist": artist_override.unwrap_or("Unknown"),
        "type": parsed.content_type,
        "domain": parsed.domain,
        "additionalInfo": additional_info,
    })
}

pub(super) fn fallback_podcore_metadata(parsed: &ParsedPodcoreContentId) -> serde_json::Value {
    let title = if parsed.domain_lower == "video" {
        parsed.id.clone()
    } else {
        format!("{}: {}", parsed.content_type, parsed.id)
    };
    basic_podcore_metadata(parsed, Some(&title), None, BTreeMap::new())
}

pub(super) async fn podcore_audio_metadata(
    settings: &MusicBrainzIntegrationSettings,
    parsed: &ParsedPodcoreContentId,
) -> Result<serde_json::Value, String> {
    match parsed.type_lower.as_str() {
        "artist" => {
            let hits = musicbrainz_search_recordings(settings, &parsed.id, 10).await?;
            let matching = hits
                .iter()
                .find(|hit| {
                    hit.artist_id
                        .as_deref()
                        .is_some_and(|id| id.eq_ignore_ascii_case(&parsed.id))
                })
                .or_else(|| {
                    hits.iter()
                        .find(|hit| hit.artist.eq_ignore_ascii_case(parsed.id.trim()))
                });
            let title = matching
                .map(|hit| hit.artist.as_str())
                .unwrap_or(parsed.id.as_str());
            let mut additional_info = BTreeMap::new();
            additional_info.insert("musicbrainz_id".to_owned(), parsed.id.clone());
            additional_info.insert("type".to_owned(), "artist".to_owned());
            additional_info.insert(
                "musicbrainz_artist_id".to_owned(),
                matching
                    .and_then(|hit| hit.artist_id.as_deref())
                    .unwrap_or(parsed.id.as_str())
                    .to_owned(),
            );
            Ok(basic_podcore_metadata(
                parsed,
                Some(title),
                Some(title),
                additional_info,
            ))
        }
        "album" => {
            let Some(value) = musicbrainz_json_request(
                settings,
                &format!(
                    "/release/{}?fmt=json&inc=recordings+artists+labels+discids+isrcs",
                    url_encode(parsed.id.trim())
                ),
            )
            .await?
            else {
                return Ok(fallback_podcore_metadata(parsed));
            };
            let title = value
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let release_date = value
                .get("date")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_musicbrainz_release_date)
                .unwrap_or_else(|| "Unknown".to_owned());
            let track_count = value
                .get("media")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flat_map(|media| media.iter())
                .filter_map(|media| media.get("tracks"))
                .filter_map(serde_json::Value::as_array)
                .flat_map(|tracks| tracks.iter())
                .filter(|track| {
                    track
                        .get("recording")
                        .is_some_and(serde_json::Value::is_object)
                })
                .count();
            let label = value
                .get("label-info")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flat_map(|labels| labels.iter())
                .filter_map(|entry| entry.pointer("/label/name"))
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .find(|name| !name.is_empty())
                .unwrap_or("Unknown");
            let mut additional_info = BTreeMap::new();
            additional_info.insert("musicbrainz_id".to_owned(), parsed.id.clone());
            additional_info.insert("release_date".to_owned(), release_date);
            additional_info.insert("track_count".to_owned(), track_count.to_string());
            additional_info.insert("label".to_owned(), label.to_owned());
            Ok(serde_json::json!({
                "contentId": parsed.full_id,
                "title": title,
                "artist": musicbrainz_artist_credit(&value),
                "type": parsed.content_type,
                "domain": parsed.domain,
                "additionalInfo": additional_info,
            }))
        }
        "track" => {
            let Some(value) = musicbrainz_json_request(
                settings,
                &format!(
                    "/recording/{}?fmt=json&inc=artists+isrcs",
                    url_encode(parsed.id.trim())
                ),
            )
            .await?
            else {
                return Ok(fallback_podcore_metadata(parsed));
            };
            let title = value
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let duration_ms = value
                .get("length")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let mut additional_info = BTreeMap::new();
            additional_info.insert("musicbrainz_id".to_owned(), parsed.id.clone());
            additional_info.insert("duration_ms".to_owned(), duration_ms.to_string());
            additional_info.insert("album".to_owned(), "Unknown".to_owned());
            additional_info.insert("position".to_owned(), "0".to_owned());
            Ok(serde_json::json!({
                "contentId": parsed.full_id,
                "title": title,
                "artist": musicbrainz_artist_credit(&value),
                "type": parsed.content_type,
                "domain": parsed.domain,
                "additionalInfo": additional_info,
            }))
        }
        _ => Ok(fallback_podcore_metadata(parsed)),
    }
}

fn parse_musicbrainz_release_date(value: &str) -> Option<String> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(1);
    let day = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(1);
    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .map(|date| date.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::{fallback_podcore_metadata, parse_podcore_content_id};

    #[test]
    fn content_id_parser_preserves_display_values_and_normalizes_match_keys() {
        let parsed = parse_podcore_content_id(" CONTENT:Audio:Track:recording-1 ")
            .expect("valid PodCore content id");
        assert_eq!(parsed.domain, "Audio");
        assert_eq!(parsed.content_type, "Track");
        assert_eq!(parsed.domain_lower, "audio");
        assert_eq!(parsed.type_lower, "track");
        assert_eq!(parsed.id, "recording-1");
        assert_eq!(parsed.full_id, "content:Audio:Track:recording-1");
    }

    #[test]
    fn content_id_parser_rejects_missing_or_blank_segments() {
        for invalid in [
            "not-content:audio:track:id",
            "content:audio:track",
            "content::track:id",
            "content:audio::id",
            "content:audio:track: ",
        ] {
            assert!(parse_podcore_content_id(invalid).is_none(), "{invalid}");
        }
    }

    #[test]
    fn fallback_metadata_uses_video_id_as_title() {
        let parsed =
            parse_podcore_content_id("content:video:clip:movie-1").expect("valid video content id");
        let metadata = fallback_podcore_metadata(&parsed);

        assert_eq!(metadata["contentId"], "content:video:clip:movie-1");
        assert_eq!(metadata["title"], "movie-1");
        assert_eq!(metadata["artist"], "Unknown");
        assert_eq!(metadata["additionalInfo"]["id"], "movie-1");
    }
}
