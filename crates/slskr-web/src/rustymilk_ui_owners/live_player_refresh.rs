use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) async fn refresh_player_status(window: &web_sys::Window) -> Result<(), JsValue> {
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    set_player_status(&document, "Refreshing player");

    if let Ok(body) = fetch_text(window, &endpoint_url("/nowplaying")).await {
        let (title, detail) = player_now_playing_text(&body);
        let track = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| current_player_track(&value));
        let rating_key = track.as_ref().map(player_rating_key).unwrap_or_default();
        let radio_query = track
            .as_ref()
            .map(|track| build_player_radio_plan(Some(track)).primary_query)
            .unwrap_or_default();
        let direct_stream_url = track.as_ref().map(player_stream_url).unwrap_or_default();
        let stream_url = if let Some(track) = track.as_ref() {
            let content_id = track
                .get("contentId")
                .or_else(|| track.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty());
            if let Some(content_id) = content_id {
                let ticket_path = format!(
                    "/streams/{}/ticket",
                    percent_encode_player_stream_component(&content_id)
                );
                let ticket_body = serde_json::json!({"contentId": content_id}).to_string();
                match fetch_text_with_method(
                    window,
                    &endpoint_url(&ticket_path),
                    "POST",
                    Some(&ticket_body),
                )
                .await
                {
                    Ok(body) => serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|value| {
                            value
                                .get("url")
                                .or_else(|| value.get("streamUrl"))
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned)
                        })
                        .unwrap_or(direct_stream_url.clone()),
                    Err(_) => direct_stream_url.clone(),
                }
            } else {
                direct_stream_url.clone()
            }
        } else {
            String::new()
        };
        if let Some(element) = document.get_element_by_id("slskr-player-now") {
            element.set_text_content(Some(&title));
        }
        if let Some(element) = document.get_element_by_id("slskr-player-now-detail") {
            element.set_text_content(Some(&detail));
        }
        if let Some(audio) = player_audio_element(&document) {
            if audio.get_attribute("src").unwrap_or_default() != stream_url {
                if stream_url.is_empty() {
                    audio.remove_attribute("src")?;
                } else {
                    audio.set_attribute("src", &stream_url)?;
                }
            }
        }
        update_player_rating_controls(window, &document, &rating_key);
        update_player_radio_controls(&document, &radio_query);
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/transfers/speeds")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-transfers") {
            element.set_text_content(Some(&player_transfer_text(&body)));
        }
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/listening-party")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-party") {
            element.set_text_content(Some(&player_party_text(&body)));
        }
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/player/external-visualizer")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-visualizer") {
            element.set_text_content(Some(&player_visualizer_text(&body)));
        }
    }

    set_player_status(&document, "Player status updated");
    Ok(())
}
