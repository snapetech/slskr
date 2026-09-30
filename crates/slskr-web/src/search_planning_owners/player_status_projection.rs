use super::*;

pub fn player_now_playing_text(body: &str) -> (String, String) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return (
            "Queue idle".to_string(),
            "No local stream selected".to_string(),
        );
    };
    let current = value
        .get("now_playing")
        .and_then(|entry| entry.as_array())
        .and_then(|items| items.first())
        .or_else(|| value.get("current"))
        .or_else(|| value.get("track"))
        .unwrap_or(&value);
    let title = current
        .get("title")
        .or_else(|| current.get("fileName"))
        .or_else(|| current.get("filename"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Queue idle".to_string());
    let artist = current
        .get("artist")
        .or_else(|| current.get("username"))
        .map(json_scalar_preview)
        .unwrap_or_default();
    let album = current
        .get("album")
        .map(json_scalar_preview)
        .unwrap_or_default();
    let detail = [artist, album]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let detail = if detail.is_empty() {
        "No local stream selected".to_string()
    } else {
        detail
    };
    (title, detail)
}

pub fn player_transfer_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "0 down / 0 up".to_string();
    };
    let downloads = value
        .get("downloads")
        .or_else(|| value.get("downloadSpeed"))
        .or_else(|| value.get("down"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    let uploads = value
        .get("uploads")
        .or_else(|| value.get("uploadSpeed"))
        .or_else(|| value.get("up"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    format!("{downloads} down / {uploads} up")
}

pub fn player_party_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "Listening party idle".to_string();
    };
    let count = value
        .get("count")
        .or_else(|| value.get("active"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    format!("{count} listening parties")
}

pub fn player_visualizer_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "Visualizer status unavailable".to_string();
    };
    value
        .get("status")
        .or_else(|| value.get("next_action"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Visualizer status unavailable".to_string())
}
