use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn reference_action_value(document: &web_sys::Document) -> String {
    let Ok(fields) = document.query_selector_all("[data-slskr-parity-reference] input, [data-slskr-parity-reference] textarea, [data-slskr-parity-reference] select") else {
        return String::new();
    };
    let mut values = Vec::new();
    for index in 0..fields.length() {
        let Some(node) = fields.item(index) else {
            continue;
        };
        if let Ok(input) = node.clone().dyn_into::<web_sys::HtmlInputElement>() {
            let value = input.value();
            if !value.trim().is_empty() {
                values.push(value);
            }
            continue;
        }
        if let Ok(textarea) = node.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
            let value = textarea.value();
            if !value.trim().is_empty() {
                values.push(value);
            }
            continue;
        }
        if let Ok(select) = node.dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            if !value.trim().is_empty() {
                values.push(value);
            }
        }
    }
    values.join("\n")
}

#[cfg(target_arch = "wasm32")]
pub(super) fn set_reference_status(document: &web_sys::Document, label: &str, message: &str) {
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>{}</strong> {}",
            escape_html(label),
            escape_html(message),
        ));
    }
    show_toast(document, &format!("{label}: {message}"));
}

#[cfg(target_arch = "wasm32")]
pub(super) fn copy_reference_text(
    window: &web_sys::Window,
    document: &web_sys::Document,
    text: String,
) {
    let clipboard = window.navigator().clipboard();
    let promise = clipboard.write_text(&text);
    let document = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        if wasm_bindgen_futures::JsFuture::from(promise).await.is_ok() {
            set_reference_status(&document, "Copy", "Copied to the clipboard");
        } else {
            set_reference_status(&document, "Copy", "Clipboard write was rejected");
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_reference_actions(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-reference-action]")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        if button.has_attribute("data-slskr-mounted") {
            continue;
        }
        button.set_attribute("data-slskr-mounted", "true")?;
        let window = window.clone();
        let document = document.clone();
        let button_for_callback = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let label = button_for_callback
                    .get_attribute("data-slskr-reference-action")
                    .unwrap_or_else(|| "Reference action".to_string());
                let route_path = window.location().pathname().unwrap_or_default();
                let value = reference_action_value(&document);
                let normalized = label.to_ascii_lowercase();
                if route_kind(&route_path) == RouteKind::SharedWithMe {
                    if matches!(
                        normalized.as_str(),
                        "open" | "open collection" | "backfill" | "leave share"
                    ) && !native_shared_grant_is_selected(
                        &document,
                        &button_for_callback,
                        &label,
                    ) {
                        return;
                    }
                    if normalized == "copy token" {
                        native_issue_share_token(&window, &document, &button_for_callback);
                        return;
                    }
                    if normalized == "copy manifest" {
                        native_copy_shared_manifest(&window, &document, &button_for_callback);
                        return;
                    }
                    if normalized == "stream" || normalized == "stream item" {
                        native_stream_shared_manifest(&window, &document, &button_for_callback);
                        return;
                    }
                }
                if matches!(
                    normalized.as_str(),
                    "copy review" | "copy manifest" | "copy token"
                ) {
                    let facts = document
                        .query_selector("[data-slskr-parity-reference] .slskr-reference-facts")
                        .ok()
                        .flatten()
                        .map(|element| element.text_content().unwrap_or_default())
                        .unwrap_or_default();
                    let text = [route_path.clone(), value.clone(), facts]
                        .into_iter()
                        .filter(|part| !part.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    copy_reference_text(&window, &document, text);
                    return;
                }
                if normalized == "clear selected user" {
                    set_reference_status(&document, &label, "Selection cleared");
                    return;
                }
                if normalized == "collapse all message panels" {
                    if let Ok(panels) = document.query_selector_all("[data-slskr-native-panel]") {
                        for panel_index in 0..panels.length() {
                            if let Some(panel) = panels.item(panel_index) {
                                if let Ok(panel) = panel.dyn_into::<web_sys::Element>() {
                                    let _ = panel.set_attribute("hidden", "");
                                }
                            }
                        }
                    }
                    set_reference_status(&document, &label, "Message panels collapsed");
                    return;
                }
                if normalized == "refresh nearby" {
                    let window_for_request = window.clone();
                    let document_for_request = document.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let result = fetch_text_with_method(
                            &window_for_request,
                            &endpoint_url("/contacts/nearby"),
                            "GET",
                            None,
                        )
                        .await;
                        match result {
                            Ok(response) => set_reference_status(
                                &document_for_request,
                                "Refresh Nearby",
                                &format!("{}", compact_preview(&response)),
                            ),
                            Err(error) => set_reference_status(
                                &document_for_request,
                                "Refresh Nearby",
                                &error
                                    .as_string()
                                    .unwrap_or_else(|| "request failed".to_string()),
                            ),
                        }
                    });
                    return;
                }
                let Some(action) = route_action_for_native_label(&route_path, &label) else {
                    set_reference_status(
                        &document,
                        &label,
                        "This control has no executable route contract",
                    );
                    return;
                };
                if !native_route_action_is_ready(
                    &document,
                    &button_for_callback,
                    &route_path,
                    action,
                    "",
                    &value,
                ) {
                    return;
                }
                let body = native_action_body(&document, &button_for_callback, action, &value);
                let method = action.method.to_string();
                let target =
                    native_action_target_for_ui(&document, &button_for_callback, action, "");
                let id = native_action_id(&document, &button_for_callback, action);
                let path = concrete_action_path_with_target_and_id(
                    &route_path,
                    action,
                    target.as_deref(),
                    id.as_deref(),
                );
                let window_for_request = window.clone();
                let document_for_request = document.clone();
                let label_for_request = label.clone();
                set_reference_status(&document, &label, "sending");
                wasm_bindgen_futures::spawn_local(async move {
                    let result = fetch_text_with_method(
                        &window_for_request,
                        &path,
                        &method,
                        body.as_deref(),
                    )
                    .await;
                    match result {
                        Ok(response) => set_reference_status(
                            &document_for_request,
                            &label_for_request,
                            &format!("{} {}", method, compact_preview(&response)),
                        ),
                        Err(error) => set_reference_status(
                            &document_for_request,
                            &label_for_request,
                            &error
                                .as_string()
                                .unwrap_or_else(|| "request failed".to_string()),
                        ),
                    }
                    let _ = refresh_route_data(&window_for_request).await;
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_global_shortcuts(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let window = window.clone();
    let listener_document = document.clone();
    let document = document.clone();
    let callback = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
        move |event: web_sys::KeyboardEvent| {
            if keyboard_event_started_in_text_control(&document) {
                return;
            }
            let key = event.key();
            if key == "/" {
                event.prevent_default();
                focus_first_card_filter(&document);
            } else if key == "Escape" {
                event.prevent_default();
                clear_all_card_filters(&document);
            } else if key.eq_ignore_ascii_case("r") && (event.ctrl_key() || event.meta_key()) {
                event.prevent_default();
                let window = window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = refresh_route_data(&window).await;
                });
            } else if matches!(key.as_str(), "1" | "2" | "3" | "4" | "5") {
                event.prevent_default();
                let rating = key.parse::<u32>().unwrap_or_default();
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
                let next = if current == rating { 0 } else { rating };
                write_player_rating(&window, &key, next);
                update_player_rating_controls(&window, &document, &key);
                set_player_status(&document, player_rating_summary(next));
            } else if key.eq_ignore_ascii_case("v") {
                event.prevent_default();
                let window = window.clone();
                let document = document.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = fetch_text_with_method(
                        &window,
                        &endpoint_url("/player/external-visualizer/launch"),
                        "POST",
                        None,
                    )
                    .await;
                    match result {
                        Ok(body) => set_player_status(&document, &compact_preview(&body)),
                        Err(error) => {
                            let message = error
                                .as_string()
                                .unwrap_or_else(|| "visualizer request failed".to_string());
                            set_player_status(&document, &message);
                        }
                    }
                    let _ = refresh_player_status(&window).await;
                });
            } else if key.eq_ignore_ascii_case("q") {
                event.prevent_default();
                open_player_radio_search(&window, &document);
            } else if key.eq_ignore_ascii_case("k") || key == " " {
                event.prevent_default();
                let window = window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = refresh_player_status(&window).await;
                });
            }
        },
    ));
    listener_document
        .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn keyboard_event_started_in_text_control(document: &web_sys::Document) -> bool {
    document
        .active_element()
        .map(|element| matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"))
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn focus_first_card_filter(document: &web_sys::Document) {
    if let Ok(Some(filter)) = document.query_selector(
        ".slskr-card-filter, [data-slskr-native-filter], [data-slskr-search-setting=\"query\"]",
    ) {
        if let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() {
            let _ = input.focus();
            let _ = input.select();
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn clear_all_card_filters(document: &web_sys::Document) {
    if let Ok(filters) = document.query_selector_all(
        ".slskr-card-filter, [data-slskr-native-filter], [data-slskr-search-setting=\"query\"]",
    ) {
        for filter_index in 0..filters.length() {
            let Some(filter) = filters.item(filter_index) else {
                continue;
            };
            let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            input.set_value("");
        }
    }

    if let Ok(rows) = document.query_selector_all("[data-slskr-row-text]") {
        for row_index in 0..rows.length() {
            let Some(row) = rows.item(row_index) else {
                continue;
            };
            let Ok(row) = row.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = row.remove_attribute("hidden");
        }
    }

    if let Ok(cards) = document.query_selector_all("[data-slskr-data-card]") {
        for card_index in 0..cards.length() {
            let Some(card) = cards.item(card_index) else {
                continue;
            };
            let Ok(card) = card.dyn_into::<web_sys::Element>() else {
                continue;
            };
            update_data_card_count(&card);
        }
    }

    if let Ok(workspaces) = document.query_selector_all(".slskr-native-workspace") {
        for workspace_index in 0..workspaces.length() {
            let Some(node) = workspaces.item(workspace_index) else {
                continue;
            };
            let Ok(workspace) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            apply_native_filter(&workspace, "");
            persist_native_filter(&workspace, "");
        }
    }

    set_live_status(document, "Filters cleared");
}
