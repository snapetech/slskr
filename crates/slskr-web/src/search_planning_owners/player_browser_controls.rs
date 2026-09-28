use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn player_ratings_storage(window: &web_sys::Window) -> Option<web_sys::Storage> {
    window.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn read_player_rating(window: &web_sys::Window, key: &str) -> u32 {
    if key.is_empty() {
        return 0;
    }
    player_ratings_storage(window)
        .and_then(|storage| storage.get_item("slskr.player.ratings").ok().flatten())
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
        .and_then(|value| value.get(key).and_then(|rating| rating.as_u64()))
        .and_then(|rating| u32::try_from(rating).ok())
        .filter(|rating| (1..=5).contains(rating))
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn write_player_rating(window: &web_sys::Window, key: &str, rating: u32) {
    if key.is_empty() {
        return;
    }
    let Some(storage) = player_ratings_storage(window) else {
        return;
    };
    let mut ratings = storage
        .get_item("slskr.player.ratings")
        .ok()
        .flatten()
        .and_then(|body| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body).ok()
        })
        .unwrap_or_default();
    if (1..=5).contains(&rating) {
        ratings.insert(key.to_string(), serde_json::Value::from(rating));
    } else {
        ratings.remove(key);
    }
    let _ = storage.set_item(
        "slskr.player.ratings",
        &serde_json::Value::Object(ratings).to_string(),
    );
}

#[cfg(target_arch = "wasm32")]
pub(super) fn update_player_rating_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
    rating_key: &str,
) {
    let rating = read_player_rating(window, rating_key);
    if let Ok(Some(player)) = document.query_selector("[data-slskr-player]") {
        let _ = player.set_attribute("data-slskr-player-rating-key", rating_key);
    }
    if let Ok(buttons) = document.query_selector_all("[data-slskr-player-rating]") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let Ok(button) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let value = button
                .get_attribute("data-slskr-player-rating")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or_default();
            let class = if value <= rating && rating > 0 {
                "is-active"
            } else {
                ""
            };
            let _ = button.set_attribute("class", class);
        }
    }
    if let Some(status) = document.get_element_by_id("slskr-player-rating-status") {
        status.set_text_content(Some(player_rating_summary(rating)));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn update_player_radio_controls(document: &web_sys::Document, query: &str) {
    if let Ok(Some(player)) = document.query_selector("[data-slskr-player]") {
        let _ = player.set_attribute("data-slskr-player-radio-query", query);
    }
    if let Some(status) = document.get_element_by_id("slskr-player-radio") {
        status.set_text_content(Some(if query.is_empty() {
            "No track selected"
        } else {
            query
        }));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn open_player_radio_search(window: &web_sys::Window, document: &web_sys::Document) {
    let query = document
        .query_selector("[data-slskr-player]")
        .ok()
        .flatten()
        .and_then(|player| player.get_attribute("data-slskr-player-radio-query"))
        .unwrap_or_default();
    if query.trim().is_empty() {
        set_player_status(document, "No track selected");
        return;
    }
    let path = build_player_radio_search_path(&query);
    if let Ok(history) = window.history() {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&path));
    }
    let _ = render_current_route(window, document);
    if let Ok(Some(input)) = document.query_selector(".slskr-toolbar-input") {
        if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value(&query);
            let _ = input.focus();
            let _ = input.select();
        }
    }
    set_player_status(document, &format!("Smart radio search ready: {query}"));
}

#[cfg(target_arch = "wasm32")]
pub(super) fn player_audio_element(
    document: &web_sys::Document,
) -> Option<web_sys::HtmlAudioElement> {
    document
        .get_element_by_id("slskr-player-audio")
        .and_then(|element| element.dyn_into::<web_sys::HtmlAudioElement>().ok())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn toggle_player_audio(document: &web_sys::Document) {
    let Some(audio) = player_audio_element(document) else {
        set_player_status(document, "Player audio element unavailable");
        return;
    };
    if audio.get_attribute("src").unwrap_or_default().is_empty() {
        set_player_status(document, "No stream loaded");
        return;
    }
    if audio.paused() {
        let _ = audio.play();
        set_player_status(document, "Playback requested");
    } else {
        let _ = audio.pause();
        set_player_status(document, "Playback paused");
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_player_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-player-action]")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let action = button
            .get_attribute("data-slskr-player-action")
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let action = action.clone();
                let window = window.clone();
                let document = document.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    set_player_status(&document, "Player action running");
                    if action == "play" {
                        toggle_player_audio(&document);
                        return;
                    }
                    if action == "radio" {
                        open_player_radio_search(&window, &document);
                        return;
                    }
                    if action == "visualizer" {
                        match toggle_rustymilk_visualizer(&window, &document) {
                            Ok(()) => set_player_status(&document, "RustyMilk visualizer ready"),
                            Err(error) => set_player_status(
                                &document,
                                &error
                                    .as_string()
                                    .unwrap_or_else(|| "RustyMilk visualizer failed".to_string()),
                            ),
                        }
                        return;
                    }
                    let result = match action.as_str() {
                        "clear" => {
                            fetch_text_with_method(
                                &window,
                                &endpoint_url("/nowplaying"),
                                "DELETE",
                                None,
                            )
                            .await
                        }
                        _ => Ok(String::new()),
                    };
                    match result {
                        Ok(body) if !body.is_empty() => {
                            set_player_status(&document, &compact_preview(&body));
                        }
                        Ok(_) => set_player_status(&document, "Player refreshed"),
                        Err(error) => {
                            let message = error
                                .as_string()
                                .unwrap_or_else(|| "player request failed".to_string());
                            set_player_status(&document, &message);
                        }
                    }
                    let _ = refresh_player_status(&window).await;
                    set_player_status(
                        &document,
                        if action == "clear" {
                            "Player cleared"
                        } else {
                            "Player refreshed"
                        },
                    );
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    let rating_buttons = document.query_selector_all("[data-slskr-player-rating]")?;
    for index in 0..rating_buttons.length() {
        let Some(node) = rating_buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let value = button
            .get_attribute("data-slskr-player-rating")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let key = document
                    .query_selector("[data-slskr-player]")
                    .ok()
                    .flatten()
                    .and_then(|player| player.get_attribute("data-slskr-player-rating-key"))
                    .unwrap_or_default();
                if key.is_empty() {
                    set_player_status(&document, "No track selected");
                    return;
                }
                let current = read_player_rating(&window, &key);
                let next = if current == value { 0 } else { value };
                write_player_rating(&window, &key, next);
                update_player_rating_controls(&window, &document, &key);
                set_player_status(&document, player_rating_summary(next));
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn set_player_status(document: &web_sys::Document, message: &str) {
    if let Some(status) = document.get_element_by_id("slskr-player-status") {
        status.set_text_content(Some(message));
    }
}
