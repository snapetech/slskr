use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_toolbar_actions(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-toolbar-command")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let action_index = button
            .get_attribute("data-slskr-toolbar-action")
            .and_then(|value| value.parse::<usize>().ok());
        let action_label = button.text_content().unwrap_or_default().trim().to_owned();
        if action_index.is_none() && action_label.is_empty() {
            continue;
        }
        let window = window.clone();
        let document = document.clone();
        let button_for_callback = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let toolbar_values = document
                    .query_selector_all(".slskr-toolbar-input")
                    .ok()
                    .map(|inputs| {
                        (0..inputs.length())
                            .filter_map(|index| {
                                inputs
                                    .item(index)
                                    .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
                                    .and_then(|element| form_control_value(&element))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let route_path = window.location().pathname().unwrap_or_default();
                if route_kind(&route_path) == RouteKind::SharedWithMe {
                    let normalized = action_label.to_ascii_lowercase();
                    if matches!(
                        normalized.as_str(),
                        "open" | "open collection" | "backfill" | "leave share"
                    ) && !native_shared_grant_is_selected(
                        &document,
                        &button_for_callback,
                        &action_label,
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
                let Some(action) = route_action_for_native_label(&route_path, &action_label)
                    .or_else(|| action_index.and_then(|index| route_action_at(&route_path, index)))
                else {
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        status.set_inner_html(&format!(
                            "<strong>{}</strong> no executable route contract",
                            escape_html(&action_label)
                        ));
                    }
                    return;
                };
                let target_value =
                    if action.path.contains(":username") || action.path.contains(":roomName") {
                        toolbar_values.first().cloned().unwrap_or_default()
                    } else {
                        String::new()
                    };
                let body_value = match action.body {
                    ActionBody::ConversationMessage | ActionBody::RoomMessage => {
                        toolbar_values.get(1).cloned().unwrap_or_else(|| {
                            native_action_value(&document, &button_for_callback, action.body)
                        })
                    }
                    ActionBody::BrowseDirectory => {
                        toolbar_values.get(1).cloned().unwrap_or_else(|| {
                            native_action_value(&document, &button_for_callback, action.body)
                        })
                    }
                    ActionBody::DownloadFiles => {
                        native_action_value(&document, &button_for_callback, action.body)
                    }
                    _ => toolbar_values.first().cloned().unwrap_or_else(|| {
                        native_action_value(&document, &button_for_callback, action.body)
                    }),
                };
                if !native_route_action_is_ready(
                    &document,
                    &button_for_callback,
                    &route_path,
                    action,
                    &target_value,
                    &body_value,
                ) {
                    return;
                }
                let body = native_action_body(&document, &button_for_callback, action, &body_value);
                let window = window.clone();
                let document = document.clone();
                let method = action.method.to_string();
                let return_to_searches = route_kind(&route_path) == RouteKind::Search
                    && route_path != "/searches"
                    && matches!(action.label, "Clear Searches" | "Remove Search");
                let target = native_action_target_for_ui(
                    &document,
                    &button_for_callback,
                    action,
                    &target_value,
                );
                let id = native_action_id(&document, &button_for_callback, action);
                let path = concrete_action_path_with_target_and_id(
                    &route_path,
                    action,
                    target.as_deref(),
                    id.as_deref(),
                );
                wasm_bindgen_futures::spawn_local(async move {
                    let result =
                        fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
                    let succeeded = result.is_ok();
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        match result {
                            Ok(response) => status.set_inner_html(&format!(
                                "<strong>{}</strong> {}",
                                escape_html(&method),
                                escape_html(&compact_preview(&response))
                            )),
                            Err(error) => {
                                let message = error
                                    .as_string()
                                    .unwrap_or_else(|| "request failed".to_string());
                                status.set_inner_html(&format!(
                                    "<strong>{}</strong> {}",
                                    escape_html(&method),
                                    escape_html(&message)
                                ));
                            }
                        }
                    }
                    if return_to_searches && succeeded {
                        let _ = window.location().set_href("/searches");
                    } else {
                        let _ = refresh_route_data(&window).await;
                    }
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_route_actions(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-action-button")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let Some(action_index) = button
            .get_attribute("data-slskr-action-index")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            continue;
        };
        let input_selector = format!(
            "#slskr-route-actions li:nth-child({}) .slskr-action-input",
            index + 1
        );
        let window = window.clone();
        let document = document.clone();
        let button_for_callback = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |_event: web_sys::MouseEvent| {
                let value = document
                    .query_selector(&input_selector)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_default();
                let route_path = window.location().pathname().unwrap_or_default();
                let Some(action) = route_action_at(&route_path, action_index) else {
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
                let window = window.clone();
                let document = document.clone();
                let method = action.method.to_string();
                let return_to_searches = route_kind(&route_path) == RouteKind::Search
                    && route_path != "/searches"
                    && matches!(action.label, "Clear Searches" | "Remove Search");
                let target =
                    native_action_target_for_ui(&document, &button_for_callback, action, "");
                let id = native_action_id(&document, &button_for_callback, action);
                let path = concrete_action_path_with_target_and_id(
                    &route_path,
                    action,
                    target.as_deref(),
                    id.as_deref(),
                );
                wasm_bindgen_futures::spawn_local(async move {
                    let result =
                        fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
                    let succeeded = result.is_ok();
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        match result {
                            Ok(response) => status.set_inner_html(&format!(
                                "<strong>{}</strong> {}",
                                escape_html(&method),
                                escape_html(&compact_preview(&response))
                            )),
                            Err(error) => {
                                let message = error
                                    .as_string()
                                    .unwrap_or_else(|| "request failed".to_string());
                                status.set_inner_html(&format!(
                                    "<strong>{}</strong> {}",
                                    escape_html(&method),
                                    escape_html(&message)
                                ));
                            }
                        }
                    }
                    if return_to_searches && succeeded {
                        let _ = window.location().set_href("/searches");
                    } else {
                        let _ = refresh_route_data(&window).await;
                    }
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn refresh_runtime_status() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let Some(status) = document.get_element_by_id("slskr-runtime-status") else {
        return Ok(());
    };

    let mut rendered = String::new();
    for probe in runtime_probes() {
        let path = endpoint_url(probe.path);
        let result = fetch_text(&window, &path).await;
        let row = match result {
            Ok(body) => runtime_probe_result_html(&[(probe.label, &path, Ok(body.as_str()))]),
            Err(error) => {
                let message = error
                    .as_string()
                    .unwrap_or_else(|| "request failed".to_string());
                runtime_probe_result_html(&[(probe.label, &path, Err(message.as_str()))])
            }
        };
        rendered.push_str(&row);
        status.set_inner_html(&rendered);
    }

    Ok(())
}
