use super::*;
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn transfer_attempts_response_html(response: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(response) else {
        return "<p>Attempt history could not be read.</p>".to_string();
    };
    let attempts = value
        .get("attempts")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    if attempts.is_empty() {
        return "<p>No attempts are retained for this request.</p>".to_string();
    }
    let items = attempts
        .iter()
        .map(|attempt| {
            let peer = value_text(attempt, &["peer_username", "username"])
                .unwrap_or_else(|| "unknown peer".to_string());
            let state =
                value_text(attempt, &["status", "state"]).unwrap_or_else(|| "unknown".to_string());
            let filename = value_text(attempt, &["filename"]).unwrap_or_default();
            format!(
                "<li><i></i><span><strong>{}</strong><small>{} · {}</small></span></li>",
                escape_html(&peer),
                escape_html(&state),
                escape_html(&filename),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!("<ol>{items}</ol>")
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn native_action_fallback(body: ActionBody) -> String {
    match body {
        ActionBody::BrowseDirectory => "/".to_string(),
        ActionBody::CollectionItem
        | ActionBody::ConversationMessage
        | ActionBody::DownloadFiles
        | ActionBody::FeedPreview
        | ActionBody::InviteRequest
        | ActionBody::LibraryPath
        | ActionBody::JsonString
        | ActionBody::MusicBrainzTarget
        | ActionBody::NameDescription
        | ActionBody::RoomMessage
        | ActionBody::SearchText
        | ActionBody::ShareGrant
        | ActionBody::ShareGroupMember
        | ActionBody::Username
        | ActionBody::ContactDiscovery
        | ActionBody::ContactInvite
        | ActionBody::SongIdSource => String::new(),
        ActionBody::Permissions => "read".to_string(),
        ActionBody::EnabledFalse | ActionBody::EnabledTrue | ActionBody::None => String::new(),
    }
}
