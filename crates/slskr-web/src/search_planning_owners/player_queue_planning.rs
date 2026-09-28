use super::*;

pub(super) fn player_auto_queue_tags(item: &serde_json::Value) -> Vec<String> {
    ["tags", "genres"]
        .iter()
        .flat_map(|key| {
            item.get(*key)
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .map(|value| json_scalar_preview(&value).trim().to_lowercase())
        .chain(std::iter::once(
            json_track_field(item, &["genre"]).to_lowercase(),
        ))
        .filter(|value| !value.is_empty())
        .collect()
}

pub(super) fn player_title_tokens(item: &serde_json::Value) -> Vec<String> {
    json_track_field(item, &["title", "fileName", "filename"])
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == ' ' {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .map(ToOwned::to_owned)
        .collect()
}

pub fn player_similarity_score(current: &serde_json::Value, candidate: &serde_json::Value) -> u32 {
    let mut score = 0;
    let current_artist = json_track_field(current, &["artist"]).to_lowercase();
    let candidate_artist = json_track_field(candidate, &["artist"]).to_lowercase();
    if !current_artist.is_empty() && current_artist == candidate_artist {
        score += 4;
    }
    let current_album = json_track_field(current, &["album"]).to_lowercase();
    let candidate_album = json_track_field(candidate, &["album"]).to_lowercase();
    if !current_album.is_empty() && current_album == candidate_album {
        score += 3;
    }

    let current_tags = player_auto_queue_tags(current);
    let shared_tags = player_auto_queue_tags(candidate)
        .iter()
        .filter(|tag| current_tags.iter().any(|current_tag| current_tag == *tag))
        .count() as u32;
    score += (shared_tags * 2).min(4);

    let current_tokens = player_title_tokens(current);
    let shared_title_tokens = player_title_tokens(candidate)
        .iter()
        .filter(|token| {
            current_tokens
                .iter()
                .any(|current_token| current_token == *token)
        })
        .count() as u32;
    score += shared_title_tokens.min(2);
    score
}

pub fn build_similar_queue_candidates(
    current: Option<&serde_json::Value>,
    history: &[serde_json::Value],
    queue: &[serde_json::Value],
    limit: usize,
) -> Vec<SimilarQueueCandidate> {
    let Some(current) = current else {
        return Vec::new();
    };
    let mut seen = queue
        .iter()
        .filter_map(|item| {
            item.get("contentId")
                .or_else(|| item.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())
        })
        .collect::<Vec<_>>();
    let mut candidates = history
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let content_id = item
                .get("contentId")
                .or_else(|| item.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())?;
            if seen.iter().any(|seen_id| seen_id == &content_id) {
                return None;
            }
            let score = player_similarity_score(current, item);
            if score == 0 {
                return None;
            }
            seen.push(content_id);
            Some(SimilarQueueCandidate {
                index,
                item: item.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.index.cmp(&right.index))
    });
    candidates.truncate(limit);
    candidates
}

pub fn similar_queue_search_queries(
    candidates: &[SimilarQueueCandidate],
    limit: usize,
) -> Vec<String> {
    unique_nonempty_case_insensitive(
        candidates
            .iter()
            .map(|candidate| {
                unique_nonempty(vec![
                    json_track_field(&candidate.item, &["artist"]),
                    json_track_field(&candidate.item, &["title", "fileName", "filename"]),
                ])
                .join(" ")
            })
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .take(limit)
    .collect()
}

pub fn player_radio_query_from_now_playing_body(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| current_player_track(&value))
        .map(|track| build_player_radio_plan(Some(&track)).primary_query)
        .unwrap_or_default()
}

pub(super) fn current_player_track(value: &serde_json::Value) -> Option<serde_json::Value> {
    value
        .get("now_playing")
        .and_then(|entry| entry.as_array())
        .and_then(|items| items.first())
        .or_else(|| value.get("current"))
        .or_else(|| value.get("track"))
        .or(Some(value))
        .cloned()
}
