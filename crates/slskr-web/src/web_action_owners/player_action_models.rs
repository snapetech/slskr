use super::*;

pub fn player_rating_key(track: &serde_json::Value) -> String {
    if let Some(content_id) = track
        .get("contentId")
        .or_else(|| track.get("content_id"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        return format!("content:{content_id}");
    }
    if let Some(stream_url) = track
        .get("streamUrl")
        .or_else(|| track.get("stream_url"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        return format!("stream:{stream_url}");
    }
    let parts = ["artist", "album", "title", "fileName", "filename"]
        .iter()
        .filter_map(|key| {
            track
                .get(*key)
                .map(json_scalar_preview)
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        String::new()
    } else {
        format!("meta:{}", parts.join("|"))
    }
}

pub(super) fn percent_encode_player_stream_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

pub fn player_stream_url(track: &serde_json::Value) -> String {
    if let Some(stream_url) = track
        .get("streamUrl")
        .or_else(|| track.get("stream_url"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        return stream_url;
    }
    track
        .get("contentId")
        .or_else(|| track.get("content_id"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .map(|content_id| {
            format!(
                "/api/v0/streams/{}",
                percent_encode_player_stream_component(&content_id)
            )
        })
        .unwrap_or_default()
}

pub fn player_rating_summary(rating: u32) -> &'static str {
    match rating {
        1 | 2 => "Discovery caution",
        3 => "Neutral rating",
        4 | 5 => "Discovery boost",
        _ => "Not rated",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerRadioQuery {
    pub id: String,
    pub query: String,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerRadioPlan {
    pub basis: Vec<String>,
    pub primary_query: String,
    pub queries: Vec<PlayerRadioQuery>,
    pub ready: bool,
    pub seed_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimilarQueueCandidate {
    pub index: usize,
    pub item: serde_json::Value,
    pub score: u32,
}
