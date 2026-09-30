use super::*;

pub(super) fn json_track_field(track: &serde_json::Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            track
                .get(*key)
                .map(json_scalar_preview)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

pub(super) fn unique_nonempty(values: Vec<String>) -> Vec<String> {
    values.into_iter().filter(|value| !value.is_empty()).fold(
        Vec::<String>::new(),
        |mut unique, value| {
            if !unique.iter().any(|other| other == &value) {
                unique.push(value);
            }
            unique
        },
    )
}

pub(super) fn unique_nonempty_case_insensitive(values: Vec<String>) -> Vec<String> {
    values.into_iter().filter(|value| !value.is_empty()).fold(
        Vec::<String>::new(),
        |mut unique, value| {
            if !unique
                .iter()
                .any(|other| other.eq_ignore_ascii_case(&value))
            {
                unique.push(value);
            }
            unique
        },
    )
}

pub(super) fn player_radio_tags(track: &serde_json::Value) -> Vec<String> {
    for key in ["tags", "genres"] {
        let values = track
            .get(key)
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(json_scalar_preview)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !values.is_empty() {
            return values;
        }
    }
    json_track_field(track, &["genre"])
        .split('\n')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn build_player_radio_plan(track: Option<&serde_json::Value>) -> PlayerRadioPlan {
    let Some(track) = track else {
        return PlayerRadioPlan {
            basis: Vec::new(),
            primary_query: String::new(),
            queries: Vec::new(),
            ready: false,
            seed_label: "No track selected".to_string(),
        };
    };

    let artist = json_track_field(track, &["artist"]);
    let title = json_track_field(track, &["title", "fileName", "filename"]);
    let album = json_track_field(track, &["album"]);
    let tags = unique_nonempty(player_radio_tags(track));
    let track_query = unique_nonempty(vec![artist.clone(), title.clone()]).join(" ");
    let album_query = unique_nonempty(vec![artist.clone(), album.clone()]).join(" ");
    let genre_query = unique_nonempty(vec![
        artist.clone(),
        tags.first().cloned().unwrap_or_default(),
    ])
    .join(" ");
    let artist_query = artist.clone();
    let queries = unique_nonempty(vec![
        track_query.clone(),
        album_query.clone(),
        genre_query.clone(),
        artist_query,
    ])
    .into_iter()
    .enumerate()
    .map(|(index, query)| {
        let reason = if query == track_query {
            "Similar track seed"
        } else if query == album_query {
            "Album neighborhood"
        } else if query == genre_query {
            "Artist and genre seed"
        } else {
            "Artist radio seed"
        };
        PlayerRadioQuery {
            id: format!("radio-query-{}", index + 1),
            query,
            reason,
        }
    })
    .collect::<Vec<_>>();
    let seed_label = unique_nonempty(vec![artist.clone(), title.clone()]).join(" - ");
    PlayerRadioPlan {
        basis: vec![
            artist
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Artist: {artist}")),
            title
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Track: {title}")),
            album
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Album: {album}")),
            tags.is_empty().then(String::new).unwrap_or_else(|| {
                format!(
                    "Tags: {}",
                    tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )
            }),
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect(),
        primary_query: queries
            .first()
            .map(|query| query.query.clone())
            .unwrap_or_default(),
        ready: !queries.is_empty(),
        queries,
        seed_label: if !seed_label.is_empty() {
            seed_label
        } else if !title.is_empty() {
            title
        } else if !artist.is_empty() {
            artist
        } else {
            "Untitled seed".to_string()
        },
    }
}

pub(super) fn percent_encode_query(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (*byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn build_player_radio_search_path(query: &str) -> String {
    let normalized = query.trim();
    if normalized.is_empty() {
        "/searches".to_string()
    } else {
        format!("/searches?q={}", percent_encode_query(normalized))
    }
}

pub fn player_radio_queries(plan: &PlayerRadioPlan, limit: usize) -> Vec<String> {
    unique_nonempty_case_insensitive(
        plan.queries
            .iter()
            .map(|item| item.query.clone())
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .take(limit)
    .collect()
}

pub(super) fn quote_if_needed(value: &str) -> String {
    let normalized = value.trim();
    if normalized.is_empty() {
        String::new()
    } else if normalized.chars().any(char::is_whitespace) {
        format!("\"{normalized}\"")
    } else {
        normalized.to_string()
    }
}

pub fn player_radio_copy_text(plan: &PlayerRadioPlan) -> String {
    if !plan.ready {
        return String::new();
    }
    let mut lines = vec![format!("Smart radio seed: {}", plan.seed_label)];
    lines.extend(
        plan.queries
            .iter()
            .map(|item| format!("{}: {}", item.reason, quote_if_needed(&item.query))),
    );
    lines.join("\n")
}
