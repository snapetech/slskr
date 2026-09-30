use super::*;

pub(super) fn overlay_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub(super) fn mesh_search_file_dto(entry: &crate::FileEntry) -> Option<MeshSearchFileDto> {
    let size = i64::try_from(entry.size).ok()?;
    let extension = (!entry.extension.is_empty()).then(|| entry.extension.clone());
    let bitrate = entry
        .attributes
        .iter()
        .find(|attribute| attribute.code == 0)
        .and_then(|attribute| i32::try_from(attribute.value).ok());
    let duration = entry
        .attributes
        .iter()
        .find(|attribute| attribute.code == 1)
        .and_then(|attribute| i32::try_from(attribute.value).ok());

    Some(MeshSearchFileDto {
        filename: entry.filename.clone(),
        size,
        extension,
        bitrate,
        duration,
        codec: mesh_search_codec(entry.extension.as_str()),
        media_kinds: mesh_search_media_kinds(entry.extension.as_str()),
        content_id: None,
        hash: None,
    })
}

pub(super) fn mesh_search_error_response(
    request_id: String,
    error: &str,
) -> MeshSearchResponseMessage {
    MeshSearchResponseMessage::new(request_id, Vec::new(), false, Some(error.to_owned()))
        .expect("validated mesh search request id must produce a valid error response")
}

pub(super) fn mesh_search_codec(extension: &str) -> Option<String> {
    match extension
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "flac" => Some("FLAC".to_owned()),
        "mp3" => Some("MP3".to_owned()),
        "m4a" | "aac" => Some("AAC".to_owned()),
        "opus" => Some("Opus".to_owned()),
        "ogg" => Some("Vorbis".to_owned()),
        "wav" => Some("WAV".to_owned()),
        _ => None,
    }
}

pub(super) fn mesh_search_media_kinds(extension: &str) -> Option<Vec<String>> {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    let mut kinds = Vec::new();
    if matches!(
        extension.as_str(),
        "mp3" | "flac" | "m4a" | "aac" | "opus" | "ogg" | "wav" | "wma" | "ape" | "mka"
    ) {
        kinds.push("Music".to_owned());
    }
    if matches!(
        extension.as_str(),
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
    ) {
        kinds.push("Video".to_owned());
    }
    if matches!(
        extension.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico"
    ) {
        kinds.push("Image".to_owned());
    }
    (!kinds.is_empty()).then_some(kinds)
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct ShadowQueryRequest {
    #[serde(alias = "MBID", alias = "mbid")]
    pub(super) mbid: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct ShadowBatchRequest {
    #[serde(alias = "MBIDs", alias = "mbids")]
    pub(super) mbids: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct MeshContentRequest {
    #[serde(alias = "ContentId", alias = "contentId")]
    pub(super) content_id: String,
    #[serde(default, alias = "Range", alias = "range")]
    pub(super) range: Option<MeshContentRange>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct MeshContentRange {
    #[serde(alias = "Offset", alias = "offset")]
    pub(super) offset: i64,
    #[serde(alias = "Length", alias = "length")]
    pub(super) length: i64,
}

pub(super) fn valid_shadow_mbid(value: &str) -> Result<&str, (i32, String)> {
    let value = value.trim();
    if !(8..=MAX_SHADOW_MBID_BYTES).contains(&value.len())
        || value.contains("..")
        || value.contains(['/', '\\'])
        || value.chars().any(char::is_control)
    {
        return Err((4, "Invalid MBID".to_owned()));
    }
    Ok(value)
}

pub(super) async fn shadow_index_result(
    state: &crate::AppState,
    mbid: &str,
) -> Option<serde_json::Value> {
    let discovery = state.content_discovery.read().await;
    let shadow = discovery
        .shadow_records()
        .iter()
        .find(|record| record.recording_id.eq_ignore_ascii_case(mbid))?;
    let canonical_variants = discovery
        .hash_entries()
        .iter()
        .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(mbid))
        .take(10)
        .filter_map(|entry| {
            let hash = [&entry.file_sha256, &entry.full_file_hash, &entry.byte_hash]
                .into_iter()
                .find(|hash| !hash.is_empty())?;
            let hash = hex::decode(hash).ok()?;
            Some(serde_json::json!({
                "Codec": "FLAC",
                "BitrateKbps": 0,
                "SizeBytes": entry.size,
                "HashPrefix": BASE64.encode(&hash[..hash.len().min(16)]),
                "QualityScore": 1.0,
            }))
        })
        .collect::<Vec<_>>();
    let last_updated = chrono::DateTime::from_timestamp(shadow.updated_at as i64, 0)
        .map(|timestamp| timestamp.to_rfc3339());
    Some(serde_json::json!({
        "MBID": shadow.recording_id,
        "PeerCount": shadow.peer_ids.len(),
        "CanonicalVariants": canonical_variants,
        "LastUpdated": last_updated,
    }))
}

pub(super) fn mesh_content_range(
    requested: Option<&MeshContentRange>,
    size: u64,
) -> Result<(u64, usize), (i32, String)> {
    let (offset, requested_length) = match requested {
        Some(range) if range.offset >= 0 && range.length >= 0 => {
            (range.offset as u64, range.length as u64)
        }
        Some(_) => return Err((4, "Invalid range request".to_owned())),
        None => (0, size),
    };
    if offset >= size {
        return Err((4, "Invalid range request".to_owned()));
    }
    let remaining = size - offset;
    let length = if requested_length == 0 {
        remaining
    } else {
        requested_length.min(remaining)
    };
    if length == 0 {
        return Err((4, "Invalid range request".to_owned()));
    }
    if length > MAX_MESH_CONTENT_BYTES as u64 {
        return Err((9, "Range too large; request a smaller range".to_owned()));
    }
    Ok((offset, length as usize))
}
