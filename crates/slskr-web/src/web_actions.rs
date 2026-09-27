#[cfg(any(target_arch = "wasm32", test))]
fn transfer_attempts_response_html(response: &str) -> String {
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

#[cfg(target_arch = "wasm32")]
fn handle_private_message_auto_response_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> bool {
    if !button.has_attribute("data-slskr-message-gate-save") {
        return false;
    }
    let Some(panel) = button
        .closest("[data-slskr-message-gate-panel]")
        .ok()
        .flatten()
    else {
        show_toast(document, "Gate reply controls are unavailable");
        return true;
    };
    let field = |name: &str| {
        panel
            .query_selector(&format!(r#"[data-slskr-message-gate-field="{name}"]"#))
            .ok()
            .flatten()
    };
    let enabled = field("enabled")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .is_some_and(|input| input.checked());
    let cooldown = field("cooldownMinutes")
        .as_ref()
        .and_then(form_control_value)
        .unwrap_or_default();
    let cooldown = match cooldown.trim().parse::<u64>() {
        Ok(value) if (1..=1_440).contains(&value) => value,
        _ => {
            show_toast(document, "Cooldown must be between 1 and 1440 minutes");
            return true;
        }
    };
    let message = field("message")
        .as_ref()
        .and_then(form_control_value)
        .unwrap_or_default();
    let message_json = if message.trim().is_empty() {
        String::new()
    } else {
        format!(r#", "message":"{}""#, escape_json_string(message.trim()))
    };
    let body = format!(r#"{{"enabled":{enabled},"cooldownMinutes":{cooldown}{message_json}}}"#);
    show_toast(document, "Save gate reply sending");
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    let path = endpoint_url("/private-message-auto-response");
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(&window, &path, "PUT", Some(&body)).await;
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            match result {
                Ok(response) => status.set_inner_html(&format!(
                    "<strong>Save gate reply</strong> {}",
                    escape_html(&compact_preview(&response))
                )),
                Err(error) => {
                    let error = error
                        .as_string()
                        .unwrap_or_else(|| "gate reply request failed".to_string());
                    status.set_inner_html(&format!(
                        "<strong>Save gate reply</strong> {}",
                        escape_html(&error)
                    ));
                }
            }
        }
    });
    true
}

#[cfg(target_arch = "wasm32")]
fn handle_wishlist_policy_action(document: &web_sys::Document, button: &web_sys::Element) -> bool {
    let save = button.has_attribute("data-slskr-wishlist-policy-save");
    let mark_viewed = button.has_attribute("data-slskr-wishlist-mark-viewed");
    let load_history = button.has_attribute("data-slskr-wishlist-load-history");
    if !save && !mark_viewed && !load_history {
        return false;
    }
    let panel = button
        .closest("[data-slskr-wishlist-policy-panel]")
        .ok()
        .flatten();
    let Some(item_id) = panel
        .as_ref()
        .and_then(|panel| panel.get_attribute("data-slskr-native-wishlist-id"))
        .filter(|value| safe_route_segment(value))
    else {
        show_toast(document, "Choose a wanted search first");
        return true;
    };

    let field = |name: &str| {
        panel.as_ref().and_then(|panel| {
            panel
                .query_selector(&format!(r#"[data-slskr-wishlist-policy-field="{name}"]"#))
                .ok()
                .flatten()
        })
    };
    let (method, path, body, label) = if save {
        let filter = field("filter")
            .as_ref()
            .and_then(form_control_value)
            .unwrap_or_default();
        let max_results = field("maxResults")
            .as_ref()
            .and_then(form_control_value)
            .unwrap_or_default();
        let max_downloads = field("maxDownloads")
            .as_ref()
            .and_then(form_control_value)
            .unwrap_or_default();
        let checked = |name: &str| {
            field(name)
                .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                .is_some_and(|input| input.checked())
        };
        let max_results = match max_results.trim().parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => {
                show_toast(document, "Maximum results must be a positive number");
                return true;
            }
        };
        let max_downloads_json = if max_downloads.trim().is_empty() {
            "null".to_string()
        } else {
            match max_downloads.trim().parse::<u64>() {
                Ok(value) if value > 0 => value.to_string(),
                _ => {
                    show_toast(document, "Maximum downloads must be blank or positive");
                    return true;
                }
            }
        };
        (
            "PUT",
            endpoint_url(&format!("/wishlist/{item_id}")),
            Some(format!(
                r#"{{"filter":"{}","enabled":{},"autoDownload":{},"maxResults":{},"maxDownloads":{}}}"#,
                escape_json_string(filter.trim()),
                checked("enabled"),
                checked("autoDownload"),
                max_results,
                max_downloads_json,
            )),
            "Save policy",
        )
    } else if mark_viewed {
        (
            "POST",
            endpoint_url(&format!("/wishlist/{item_id}/mark-viewed")),
            None,
            "Mark viewed",
        )
    } else {
        (
            "GET",
            endpoint_url(&format!("/wishlist/{item_id}/searches?limit=20")),
            None,
            "Load run ledger",
        )
    };

    show_toast(document, &format!("{label} sending"));
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    let panel = panel.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(&window, &path, method, body.as_deref()).await;
        if load_history {
            if let Some(target) = panel.as_ref().and_then(|panel| {
                panel
                    .query_selector("[data-slskr-wishlist-history-results]")
                    .ok()
                    .flatten()
            }) {
                target.set_inner_html(&match &result {
                    Ok(response) => wishlist_history_response_html(response),
                    Err(error) => format!(
                        "<p>{}</p>",
                        escape_html(
                            &error
                                .as_string()
                                .unwrap_or_else(|| "run history request failed".to_string()),
                        )
                    ),
                });
            }
        }
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            match result {
                Ok(response) => status.set_inner_html(&format!(
                    "<strong>{}</strong> {}",
                    escape_html(label),
                    escape_html(&compact_preview(&response))
                )),
                Err(error) => {
                    let error = error
                        .as_string()
                        .unwrap_or_else(|| "wishlist request failed".to_string());
                    status.set_inner_html(&format!(
                        "<strong>{}</strong> {}",
                        escape_html(label),
                        escape_html(&error)
                    ));
                }
            }
        }
    });
    true
}

#[cfg(target_arch = "wasm32")]
fn handle_wishlist_ignored_result_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> bool {
    let restore = button.has_attribute("data-slskr-wishlist-ignore-restore");
    let create = button.has_attribute("data-slskr-wishlist-ignore-submit");
    if !restore && !create {
        return false;
    }
    let panel = button
        .closest("[data-slskr-wishlist-ignore-panel]")
        .ok()
        .flatten();
    let item_id = button
        .get_attribute("data-slskr-native-wishlist-id")
        .or_else(|| {
            panel
                .as_ref()
                .and_then(|panel| panel.get_attribute("data-slskr-native-wishlist-id"))
        });
    let Some(item_id) = item_id.filter(|value| safe_route_segment(value)) else {
        show_toast(document, "Choose a wanted search first");
        return true;
    };
    let (method, path, body, label) = if restore {
        let Some(rule_id) = button
            .get_attribute("data-slskr-wishlist-ignore-rule-id")
            .filter(|value| safe_route_segment(value))
        else {
            show_toast(document, "Ignored folder rule is unavailable");
            return true;
        };
        (
            "DELETE",
            endpoint_url(&format!("/wishlist/{item_id}/ignored-results/{rule_id}")),
            None,
            "Restore ignored folder",
        )
    } else {
        let username = panel
            .as_ref()
            .and_then(|panel| {
                panel
                    .query_selector(r#"input[aria-label^="Peer to ignore"]"#)
                    .ok()
                    .flatten()
            })
            .and_then(|input| form_control_value(&input))
            .unwrap_or_default();
        let directory = panel
            .as_ref()
            .and_then(|panel| {
                panel
                    .query_selector(r#"input[aria-label^="Folder to ignore"]"#)
                    .ok()
                    .flatten()
            })
            .and_then(|input| form_control_value(&input))
            .unwrap_or_default();
        if username.trim().is_empty() || directory.trim().is_empty() {
            show_toast(document, "Enter both a peer and folder");
            return true;
        }
        (
            "POST",
            endpoint_url(&format!("/wishlist/{item_id}/ignored-results")),
            Some(format!(
                r#"{{"username":"{}","directory":"{}"}}"#,
                escape_json_string(username.trim()),
                escape_json_string(directory.trim())
            )),
            "Ignore result folder",
        )
    };
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>{}</strong> sending {}",
            escape_html(label),
            method
        ));
    }
    show_toast(document, &format!("{label} sending"));
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(&window, &path, method, body.as_deref()).await;
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            match result {
                Ok(response) => status.set_inner_html(&format!(
                    "<strong>{}</strong> {}",
                    escape_html(label),
                    escape_html(&compact_preview(&response))
                )),
                Err(error) => status.set_inner_html(&format!(
                    "<strong>{}</strong> {}",
                    escape_html(label),
                    escape_html(
                        &error
                            .as_string()
                            .unwrap_or_else(|| "request failed".to_string())
                    )
                )),
            }
        }
        let _ = refresh_route_data(&window).await;
    });
    true
}

#[cfg(target_arch = "wasm32")]
fn native_action_row(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> Option<web_sys::Element> {
    let row = button
        .closest("[data-slskr-native-select]")
        .ok()
        .flatten()
        .or_else(|| {
            document
                .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
                .ok()
                .flatten()
        });
    if let Some(row) = row.as_ref() {
        select_native_row(document, row);
    }
    row
}

#[cfg(target_arch = "wasm32")]
fn native_selected_row_attribute(
    document: &web_sys::Document,
    button: &web_sys::Element,
    attribute: &str,
) -> Option<String> {
    button_native_row_attribute(button, attribute).or_else(|| {
        document
            .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
            .ok()
            .flatten()
            .and_then(|row| row.get_attribute(attribute))
            .filter(|value| !value.trim().is_empty())
    })
}

#[cfg(target_arch = "wasm32")]
fn native_selected_context(
    document: &web_sys::Document,
    button: &web_sys::Element,
    route_path: &str,
) -> String {
    let row = native_action_row(document, button);
    let title = row
        .as_ref()
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .unwrap_or_else(|| "No selected row".to_string());
    let details = row
        .as_ref()
        .and_then(|row| row.get_attribute("data-slskr-native-detail-list"))
        .or_else(|| {
            row.as_ref()
                .and_then(|row| row.get_attribute("data-slskr-native-detail"))
        })
        .unwrap_or_default();
    let meta = row
        .as_ref()
        .and_then(|row| row.get_attribute("data-slskr-native-meta"))
        .unwrap_or_default();
    [route_path.to_string(), title, details, meta]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(target_arch = "wasm32")]
fn native_json_string_by_key(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => {
            for key in keys {
                if let Some(value) = object.get(*key).and_then(serde_json::Value::as_str) {
                    if !value.trim().is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
            object
                .values()
                .find_map(|value| native_json_string_by_key(value, keys))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| native_json_string_by_key(value, keys)),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
fn native_set_action_status(document: &web_sys::Document, label: &str, message: &str) {
    set_reference_status(document, label, message);
}

#[cfg(target_arch = "wasm32")]
fn native_copy_context(
    window: &web_sys::Window,
    document: &web_sys::Document,
    button: &web_sys::Element,
    route_path: &str,
    label: &str,
) {
    let text = native_selected_context(document, button, route_path);
    if text == format!("{route_path}\nNo selected row") {
        native_set_action_status(
            document,
            label,
            "Select a live row before copying its report.",
        );
        return;
    }
    copy_reference_text(window, document, text);
}

#[cfg(target_arch = "wasm32")]
fn native_shared_grant_id(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> Option<String> {
    native_selected_row_attribute(document, button, "data-slskr-native-grant-id")
        .filter(|value| safe_route_segment(value))
}

#[cfg(target_arch = "wasm32")]
fn native_shared_grant_is_selected(
    document: &web_sys::Document,
    button: &web_sys::Element,
    label: &str,
) -> bool {
    if native_shared_grant_id(document, button).is_some() {
        return true;
    }
    native_set_action_status(
        document,
        label,
        "Select a live inbound share before running this action.",
    );
    false
}

#[cfg(target_arch = "wasm32")]
fn native_shared_token_storage_key(grant_id: &str) -> String {
    format!("slskr.shared-grant-token.{grant_id}")
}

#[cfg(target_arch = "wasm32")]
fn native_shared_token(window: &web_sys::Window, grant_id: &str) -> Option<String> {
    window
        .session_storage()
        .ok()
        .flatten()
        .and_then(|storage| {
            storage
                .get_item(&native_shared_token_storage_key(grant_id))
                .ok()
                .flatten()
        })
        .filter(|token| !token.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn native_store_shared_token(window: &web_sys::Window, grant_id: &str, token: &str) {
    if let Some(storage) = window.session_storage().ok().flatten() {
        let _ = storage.set_item(&native_shared_token_storage_key(grant_id), token);
    }
}

#[cfg(target_arch = "wasm32")]
async fn native_issue_shared_access_token(
    window: &web_sys::Window,
    grant_id: &str,
) -> Result<String, JsValue> {
    let path = endpoint_url(&format!("/share-grants/{grant_id}/token"));
    let response =
        fetch_text_with_method(window, &path, "POST", Some(r#"{"expiresInSeconds":600}"#)).await?;
    serde_json::from_str::<serde_json::Value>(&response)
        .ok()
        .and_then(|value| {
            native_json_string_by_key(&value, &["token", "shareToken", "share_token"])
        })
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| JsValue::from_str("the token endpoint returned no token"))
}

#[cfg(target_arch = "wasm32")]
fn native_issue_share_token(
    window: &web_sys::Window,
    document: &web_sys::Document,
    button: &web_sys::Element,
) {
    let Some(grant_id) = native_shared_grant_id(document, button) else {
        native_set_action_status(document, "Copy token", "Select a live shared grant first.");
        return;
    };
    native_set_action_status(document, "Copy token", "Issuing a short-lived share token.");
    let window_for_request = window.clone();
    let document_for_request = document.clone();
    let grant_id_for_request = grant_id.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match native_issue_shared_access_token(&window_for_request, &grant_id_for_request).await {
            Ok(token) => {
                native_store_shared_token(&window_for_request, &grant_id_for_request, &token);
                copy_reference_text(&window_for_request, &document_for_request, token);
            }
            Err(error) => native_set_action_status(
                &document_for_request,
                "Copy token",
                &error
                    .as_string()
                    .unwrap_or_else(|| "share token request failed".to_string()),
            ),
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn native_copy_shared_manifest(
    window: &web_sys::Window,
    document: &web_sys::Document,
    button: &web_sys::Element,
) {
    let Some(grant_id) = native_shared_grant_id(document, button) else {
        native_set_action_status(
            document,
            "Copy manifest",
            "Select a live shared grant first.",
        );
        return;
    };
    let window_for_request = window.clone();
    let document_for_request = document.clone();
    let stored_token = native_shared_token(window, &grant_id);
    native_set_action_status(document, "Copy manifest", "Loading the live manifest.");
    wasm_bindgen_futures::spawn_local(async move {
        let path = endpoint_url(&format!("/share-grants/{grant_id}/manifest"));
        let headers = stored_token
            .as_deref()
            .map(|token| [("X-Share-Token", token)])
            .unwrap_or_default();
        match fetch_text_with_method_and_headers(&window_for_request, &path, "GET", None, &headers)
            .await
        {
            Ok(response) => {
                copy_reference_text(&window_for_request, &document_for_request, response)
            }
            Err(error) => native_set_action_status(
                &document_for_request,
                "Copy manifest",
                &error
                    .as_string()
                    .unwrap_or_else(|| "manifest request failed".to_string()),
            ),
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn native_stream_shared_manifest(
    window: &web_sys::Window,
    document: &web_sys::Document,
    button: &web_sys::Element,
) {
    let Some(grant_id) = native_shared_grant_id(document, button) else {
        native_set_action_status(document, "Stream", "Select a live shared grant first.");
        return;
    };
    let window_for_request = window.clone();
    let document_for_request = document.clone();
    native_set_action_status(document, "Stream", "Loading the live shared manifest.");
    wasm_bindgen_futures::spawn_local(async move {
        let path = endpoint_url(&format!("/share-grants/{grant_id}/manifest"));
        let stored_token = native_shared_token(&window_for_request, &grant_id);
        let manifest_headers = stored_token
            .as_deref()
            .map(|token| [("X-Share-Token", token)])
            .unwrap_or_default();
        match fetch_text_with_method_and_headers(
            &window_for_request,
            &path,
            "GET",
            None,
            &manifest_headers,
        )
        .await
        {
            Ok(response) => {
                let value = serde_json::from_str::<serde_json::Value>(&response).ok();
                let Some(value) = value else {
                    native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "The shared manifest was not valid JSON.",
                    );
                    return;
                };
                let content_id = value
                    .get("items")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|items| items.first())
                    .and_then(|item| {
                        ["contentId", "content_id", "id"]
                            .iter()
                            .find_map(|key| item.get(*key).map(json_scalar_preview))
                    })
                    .filter(|content_id| !content_id.trim().is_empty());
                let Some(content_id) = content_id else {
                    native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "The shared manifest contains no streamable items.",
                    );
                    return;
                };
                let token = stored_token
                    .or_else(|| {
                        value.as_object().and_then(|object| {
                            ["shareToken", "share_token", "token"]
                                .iter()
                                .find_map(|key| object.get(*key).map(json_scalar_preview))
                        })
                    })
                    .filter(|token| !token.trim().is_empty());
                let token = match token {
                    Some(token) => token,
                    None => match native_issue_shared_access_token(&window_for_request, &grant_id)
                        .await
                    {
                        Ok(token) => {
                            native_store_shared_token(&window_for_request, &grant_id, &token);
                            token
                        }
                        Err(error) => {
                            native_set_action_status(
                                &document_for_request,
                                "Stream",
                                &error
                                    .as_string()
                                    .unwrap_or_else(|| "share token request failed".to_string()),
                            );
                            return;
                        }
                    },
                };
                let encoded_content_id = percent_encode_player_stream_component(&content_id);
                let ticket_path =
                    endpoint_url(&format!("/streams/{encoded_content_id}/share-ticket"));
                let ticket_headers = [("X-Share-Token", token.as_str())];
                let ticket = match fetch_text_with_method_and_headers(
                    &window_for_request,
                    &ticket_path,
                    "POST",
                    None,
                    &ticket_headers,
                )
                .await
                {
                    Ok(response) => serde_json::from_str::<serde_json::Value>(&response)
                        .ok()
                        .and_then(|value| {
                            ["ticket", "streamTicket"]
                                .iter()
                                .find_map(|key| value.get(*key).map(json_scalar_preview))
                        })
                        .filter(|ticket| !ticket.trim().is_empty()),
                    Err(error) => {
                        native_set_action_status(
                            &document_for_request,
                            "Stream",
                            &error.as_string().unwrap_or_else(|| {
                                "share stream ticket request failed".to_string()
                            }),
                        );
                        None
                    }
                };
                let Some(ticket) = ticket else {
                    if document_for_request
                        .get_element_by_id("slskr-action-status")
                        .and_then(|element| element.text_content())
                        .unwrap_or_default()
                        .is_empty()
                    {
                        native_set_action_status(
                            &document_for_request,
                            "Stream",
                            "The stream ticket endpoint returned no ticket.",
                        );
                    }
                    return;
                };
                let stream_url = endpoint_url(&format!(
                    "/streams/{encoded_content_id}?ticket={}",
                    percent_encode_player_stream_component(&ticket)
                ));
                let url = if stream_url.starts_with("http://") || stream_url.starts_with("https://")
                {
                    stream_url
                } else {
                    format!(
                        "{}{}",
                        window_for_request.location().origin().unwrap_or_default(),
                        stream_url
                    )
                };
                match window_for_request.open_with_url_and_target_and_features(
                    &url,
                    "_blank",
                    "noopener,noreferrer",
                ) {
                    Ok(Some(_)) => native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "Opened the shared stream with a short-lived content ticket.",
                    ),
                    Ok(None) => native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "The browser blocked the shared stream window; allow pop-ups and try again.",
                    ),
                    Err(error) => native_set_action_status(
                        &document_for_request,
                        "Stream",
                        &error
                            .as_string()
                            .unwrap_or_else(|| "unable to open the shared stream window".to_string()),
                    ),
                }
            }
            Err(error) => native_set_action_status(
                &document_for_request,
                "Stream",
                &error
                    .as_string()
                    .unwrap_or_else(|| "shared stream request failed".to_string()),
            ),
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn native_fold_duplicate_rows(document: &web_sys::Document, button: &web_sys::Element) {
    let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() else {
        native_set_action_status(
            document,
            "Fold Duplicates",
            "No result workspace is mounted.",
        );
        return;
    };
    let folded = workspace.has_attribute("data-slskr-native-duplicates-folded");
    let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") else {
        return;
    };
    if folded {
        for index in 0..rows.length() {
            if let Some(node) = rows.item(index) {
                if let Ok(row) = node.dyn_into::<web_sys::Element>() {
                    let _ = row.remove_attribute("hidden");
                }
            }
        }
        let _ = workspace.remove_attribute("data-slskr-native-duplicates-folded");
        native_set_action_status(document, "Fold Duplicates", "Duplicate rows restored.");
        return;
    }
    let mut seen = BTreeSet::new();
    let mut hidden = 0;
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let key = row
            .get_attribute("data-slskr-native-title")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !key.is_empty() && !seen.insert(key) {
            let _ = row.set_attribute("hidden", "");
            hidden += 1;
        }
    }
    let _ = workspace.set_attribute("data-slskr-native-duplicates-folded", "true");
    native_set_action_status(
        document,
        "Fold Duplicates",
        &format!("Folded {hidden} duplicate result rows."),
    );
}

#[cfg(target_arch = "wasm32")]
fn native_import_wishlist_list(document: &web_sys::Document) {
    let value = document
        .query_selector(r#".wishlist-native [aria-label="Search Text"]"#)
        .ok()
        .flatten()
        .or_else(|| {
            document
                .query_selector(r#"input[aria-label="Search Text"]"#)
                .ok()
                .flatten()
        })
        .and_then(|element| form_control_value(&element))
        .unwrap_or_default();
    let entries = value
        .lines()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        native_set_action_status(
            document,
            "Import List",
            "Enter one wanted search per line in Search Text.",
        );
        return;
    }
    let Some(window) = document.default_view() else {
        return;
    };
    native_set_action_status(
        document,
        "Import List",
        &format!("Importing {} wanted searches.", entries.len()),
    );
    let document_for_request = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let mut created = 0;
        let mut failed = 0;
        for entry in entries {
            let body = action_body_from_value(ActionBody::SearchText, &entry);
            match fetch_text_with_method(
                &window,
                &endpoint_url("/wishlist"),
                "POST",
                body.as_deref(),
            )
            .await
            {
                Ok(_) => created += 1,
                Err(_) => failed += 1,
            }
        }
        native_set_action_status(
            &document_for_request,
            "Import List",
            &format!("Imported {created} wanted searches; {failed} failed."),
        );
        let _ = refresh_route_data(&window).await;
    });
}

#[cfg(target_arch = "wasm32")]
fn native_bulk_transfer_action(document: &web_sys::Document, action: &str, selector: &str) {
    let Ok(buttons) = document.query_selector_all(selector) else {
        native_set_action_status(document, action, "Transfer table is not mounted.");
        return;
    };
    let mut dispatched = 0;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let Ok(button) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if handle_transfer_request_action(document, &button) {
            dispatched += 1;
        }
    }
    if dispatched == 0 {
        native_set_action_status(
            document,
            action,
            "No live transfer requests match this action.",
        );
    } else {
        native_set_action_status(
            document,
            action,
            &format!("Dispatched {dispatched} live transfer request actions."),
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn native_open_search(document: &web_sys::Document, button: &web_sys::Element) -> bool {
    let Some(id) = native_selected_row_attribute(document, button, "data-slskr-native-search-id")
        .filter(|value| safe_route_segment(value))
    else {
        native_set_action_status(
            document,
            "Open",
            "Select a saved search with a real ID first.",
        );
        return true;
    };
    let Some(window) = document.default_view() else {
        return true;
    };
    let _ = window.location().set_href(&format!("/searches/{id}"));
    true
}

#[cfg(target_arch = "wasm32")]
fn handle_native_local_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
    route_path: &str,
    action: &str,
) -> bool {
    let normalized = action.trim().to_ascii_lowercase();
    let kind = route_kind(route_path);
    if normalized == "review selection" || normalized == "expand result" {
        if native_action_row(document, button).is_some() {
            native_set_action_status(document, action, "Live row selected in the inspector.");
        } else {
            native_set_action_status(document, action, "Select a live row first.");
        }
        return true;
    }
    if normalized == "fold duplicates" {
        native_fold_duplicate_rows(document, button);
        return true;
    }
    if normalized == "clear filter" {
        clear_all_card_filters(document);
        native_set_action_status(document, action, "Visible filters cleared.");
        return true;
    }
    if normalized == "clear search" {
        if let Ok(Some(input)) =
            document.query_selector(r#"input[aria-label="Search conversations"]"#)
        {
            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                input.set_value("");
            }
        }
        native_set_action_status(document, action, "Conversation search cleared.");
        return true;
    }
    if normalized == "collapse all message panels" {
        if let Ok(panels) = document.query_selector_all("[data-slskr-native-panel]") {
            for index in 0..panels.length() {
                if let Some(node) = panels.item(index) {
                    if let Ok(panel) = node.dyn_into::<web_sys::Element>() {
                        let _ = panel.set_attribute("hidden", "");
                    }
                }
            }
        }
        native_set_action_status(document, action, "Message panels collapsed.");
        return true;
    }
    if kind == RouteKind::Search && normalized == "preview" {
        let _ = native_action_row(document, button);
        if let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() {
            if let Ok(Some(panel)) =
                workspace.query_selector(r#"[data-slskr-native-panel-label="Download Preview"]"#)
            {
                if let Some(index) = panel.get_attribute("data-slskr-native-panel") {
                    if let Ok(Some(tab)) = workspace.query_selector(&format!(
                        r#"[data-slskr-native-tab="{}"]"#,
                        escape_json_string(&index)
                    )) {
                        select_native_subview(document, &tab);
                        native_set_action_status(
                            document,
                            "Preview",
                            "Download Preview opened for the selected result.",
                        );
                        return true;
                    }
                }
            }
        }
        native_set_action_status(document, "Preview", "Download Preview is not mounted.");
        return true;
    }
    if kind == RouteKind::Search && normalized == "open" {
        return native_open_search(document, button);
    }
    if kind == RouteKind::Wishlist && normalized == "import list" {
        native_import_wishlist_list(document);
        return true;
    }
    if kind == RouteKind::Downloads && normalized == "retry all" {
        native_bulk_transfer_action(document, "Retry All", "[data-slskr-transfer-retry]");
        return true;
    }
    if kind == RouteKind::Downloads && normalized == "cancel all" {
        native_bulk_transfer_action(
            document,
            "Cancel All",
            "[data-slskr-transfer-request-cancel]",
        );
        return true;
    }
    if kind == RouteKind::SharedWithMe
        && matches!(
            normalized.as_str(),
            "open" | "open collection" | "backfill" | "leave share"
        )
        && !native_shared_grant_is_selected(document, button, action)
    {
        return true;
    }
    if kind == RouteKind::SharedWithMe && normalized == "copy token" {
        if let Some(window) = document.default_view() {
            native_issue_share_token(&window, document, button);
        }
        return true;
    }
    if kind == RouteKind::SharedWithMe && normalized == "copy manifest" {
        if let Some(window) = document.default_view() {
            native_copy_shared_manifest(&window, document, button);
        }
        return true;
    }
    if kind == RouteKind::SharedWithMe && (normalized == "stream" || normalized == "stream item") {
        if let Some(window) = document.default_view() {
            native_stream_shared_manifest(&window, document, button);
        }
        return true;
    }
    if matches!(
        normalized.as_str(),
        "copy review" | "copy action plan" | "copy packet"
    ) {
        if let Some(window) = document.default_view() {
            native_copy_context(&window, document, button, route_path, action);
        }
        return true;
    }
    if normalized == "refresh" {
        let Some(window) = document.default_view() else {
            return true;
        };
        native_set_action_status(document, action, "Refreshing live route data.");
        let document_for_refresh = document.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match refresh_route_data(&window).await {
                Ok(()) => native_set_action_status(
                    &document_for_refresh,
                    "Refresh",
                    "Live route data refreshed.",
                ),
                Err(error) => native_set_action_status(
                    &document_for_refresh,
                    "Refresh",
                    &error
                        .as_string()
                        .unwrap_or_else(|| "route refresh failed".to_string()),
                ),
            }
        });
        return true;
    }
    false
}

#[cfg(target_arch = "wasm32")]
fn run_native_route_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) {
    let route_path = document
        .default_view()
        .and_then(|window| window.location().pathname().ok())
        .unwrap_or_else(|| "/searches".to_string());
    let value = native_action_value(document, button, action.body);
    if !native_route_action_is_ready(document, button, &route_path, action, "", &value) {
        return;
    }
    if native_action_requires_confirmation(action) {
        show_native_confirm_modal(document, button, action);
        return;
    }
    execute_native_route_action(document, button, action);
}

#[cfg(target_arch = "wasm32")]
fn native_action_requires_confirmation(action: RouteAction) -> bool {
    action.method == "DELETE"
        || matches!(
            action.label,
            "Cancel Download" | "Deny Upload" | "Shut Down" | "Restart" | "Vacuum Database"
        )
}

#[cfg(target_arch = "wasm32")]
fn show_native_confirm_modal(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) {
    if let Some(existing) = document.get_element_by_id("slskr-native-confirm-modal") {
        existing.remove();
    }
    let Ok(backdrop) = document.create_element("div") else {
        execute_native_route_action(document, button, action);
        return;
    };
    backdrop.set_id("slskr-native-confirm-modal");
    backdrop.set_class_name("slskr-modal-backdrop");
    let _ = backdrop.set_attribute("role", "presentation");

    let title = format!("Confirm {}", action.label);
    let target = document_selected_native_row_title(document)
        .unwrap_or_else(|| "the selected workflow item".to_string());
    backdrop.set_inner_html(&format!(
        r#"<section class="slskr-modal" role="dialog" aria-modal="true" aria-labelledby="slskr-confirm-title"><header><h3 id="slskr-confirm-title">{title}</h3></header><p>{message}</p><div class="slskr-modal-actions"><button type="button" data-slskr-confirm-cancel>Cancel</button><button type="button" data-slskr-confirm-run>Confirm</button></div></section>"#,
        title = escape_html(&title),
        message = escape_html(&format!(
            "{} will run against {}. Review the selected row before continuing.",
            action.label, target
        )),
    ));
    let Some(body) = document.body() else {
        execute_native_route_action(document, button, action);
        return;
    };
    let _ = body.append_child(&backdrop);

    if let Ok(Some(cancel)) = backdrop.query_selector("[data-slskr-confirm-cancel]") {
        let document_for_cancel = document.clone();
        let backdrop_for_cancel = backdrop.clone();
        let label = action.label.to_string();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                backdrop_for_cancel.remove();
                if let Some(status) = document_for_cancel.get_element_by_id("slskr-action-status") {
                    status.set_inner_html(&format!(
                        "<strong>{}</strong> cancelled",
                        escape_html(&label)
                    ));
                }
                show_toast(&document_for_cancel, &format!("{label} cancelled"));
            },
        ));
        let _ = cancel.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref());
        callback.forget();
    }

    if let Ok(Some(confirm)) = backdrop.query_selector("[data-slskr-confirm-run]") {
        let document_for_confirm = document.clone();
        let button_for_confirm = button.clone();
        let backdrop_for_confirm = backdrop.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                backdrop_for_confirm.remove();
                execute_native_route_action(&document_for_confirm, &button_for_confirm, action);
            },
        ));
        let _ =
            confirm.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref());
        callback.forget();
    }

    if let Ok(Some(confirm)) = backdrop.query_selector("[data-slskr-confirm-run]") {
        if let Some(element) = confirm.dyn_ref::<web_sys::HtmlElement>() {
            let _ = element.focus();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn execute_native_route_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) {
    let Some(window) = document.default_view() else {
        return;
    };
    let route_path = window
        .location()
        .pathname()
        .unwrap_or_else(|_| "/searches".to_string());
    let value = native_action_value(document, button, action.body);
    if !native_route_action_is_ready(document, button, &route_path, action, "", &value) {
        return;
    }
    let target = native_action_target(document, button, action);
    let id = native_action_id(document, button, action);
    let body = native_action_body(document, button, action, &value).map(|body| {
        if action.body == ActionBody::DownloadFiles {
            download_body_with_selected_destination(document, &body).unwrap_or(body)
        } else {
            body
        }
    });
    let method = action.method.to_string();
    let label = action.label.to_string();
    let path = concrete_action_path_with_target_and_id(
        &route_path,
        action,
        target.as_deref(),
        id.as_deref(),
    );
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>{}</strong> sending {}",
            escape_html(&label),
            escape_html(&method)
        ));
    }
    show_toast(document, &format!("{} sending", label));
    let document = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            match result {
                Ok(response) => status.set_inner_html(&format!(
                    "<strong>{}</strong> {}",
                    escape_html(&label),
                    escape_html(&compact_preview(&response))
                )),
                Err(error) => {
                    let message = error
                        .as_string()
                        .unwrap_or_else(|| "request failed".to_string());
                    status.set_inner_html(&format!(
                        "<strong>{}</strong> {}",
                        escape_html(&label),
                        escape_html(&message)
                    ));
                }
            }
        }
        let _ = refresh_route_data(&window).await;
    });
}

#[cfg(target_arch = "wasm32")]
fn download_body_with_selected_destination(
    document: &web_sys::Document,
    body: &str,
) -> Option<String> {
    let destination = document
        .query_selector("[data-slskr-download-destination]")
        .ok()
        .flatten()
        .and_then(|element| form_control_value(&element))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())?;
    let mut payload = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let files = payload.as_array_mut()?;
    for file in files {
        if let Some(object) = file.as_object_mut() {
            object
                .entry("destinationDirectory".to_owned())
                .or_insert_with(|| serde_json::Value::String(destination.clone()));
        }
    }
    Some(payload.to_string())
}

#[cfg(target_arch = "wasm32")]
fn native_action_value(
    document: &web_sys::Document,
    button: &web_sys::Element,
    body: ActionBody,
) -> String {
    if let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() {
        if matches!(body, ActionBody::DownloadFiles) {
            if let Some(value) =
                selected_native_row_attribute_values(&workspace, "data-slskr-native-filename")
            {
                return value;
            }
            if let Some(value) = button_native_row_attribute(button, "data-slskr-native-filename") {
                return value;
            }
            if let Some(value) = selected_native_row_titles(&workspace) {
                return value;
            }
        }
        if matches!(body, ActionBody::BrowseDirectory) {
            if let Some(value) = button_native_row_attribute(button, "data-slskr-native-path")
                .filter(|_| {
                    button_native_row_attribute(button, "data-slskr-native-entry-kind").as_deref()
                        == Some("folder")
                })
            {
                return value;
            }
        }
        if matches!(body, ActionBody::Username) {
            for attr in [
                "data-slskr-native-username",
                "data-slskr-native-peer",
                "data-slskr-native-owner",
                "data-slskr-native-contact",
            ] {
                if let Some(value) = button_native_row_attribute(button, attr) {
                    return value;
                }
                if let Some(value) = selected_native_row_attribute(&workspace, attr) {
                    return value;
                }
            }
        }
        if matches!(body, ActionBody::SearchText) {
            if let Some(value) = button_native_row_target(button) {
                return value;
            }
            if let Some(value) = selected_native_row_target(&workspace) {
                return value;
            }
        }
        if matches!(body, ActionBody::DownloadFiles) {
            return String::new();
        }
        for selector in native_action_value_selectors(body) {
            if let Some(value) = first_workspace_value(&workspace, selector) {
                return value;
            }
        }
        if matches!(body, ActionBody::CollectionItem) {
            return String::new();
        }
        if matches!(body, ActionBody::ShareGrant | ActionBody::ShareGroupMember) {
            for attr in [
                "data-slskr-native-username",
                "data-slskr-native-peer",
                "data-slskr-native-owner",
            ] {
                if let Some(value) = button_native_row_attribute(button, attr) {
                    return value;
                }
                if let Some(value) = selected_native_row_attribute(&workspace, attr) {
                    return value;
                }
            }
        }
        if matches!(body, ActionBody::Username) {
            if let Some(value) = selected_native_row_title(&workspace) {
                return value;
            }
        }
        for selector in native_generic_value_selectors() {
            if let Some(value) = first_workspace_value(&workspace, selector) {
                return value;
            }
        }
    }
    if matches!(
        body,
        ActionBody::ConversationMessage
            | ActionBody::RoomMessage
            | ActionBody::FeedPreview
            | ActionBody::JsonString
            | ActionBody::MusicBrainzTarget
            | ActionBody::SongIdSource
            | ActionBody::ShareGrant
            | ActionBody::ShareGroupMember
            | ActionBody::ContactDiscovery
            | ActionBody::ContactInvite
    ) {
        return native_action_fallback(body);
    }
    document_selected_native_row_title(document).unwrap_or_else(|| native_action_fallback(body))
}

#[cfg(target_arch = "wasm32")]
fn native_action_value_selectors(body: ActionBody) -> &'static [&'static str] {
    match body {
        ActionBody::BrowseDirectory => &[r#"input[aria-label="Folder"]"#],
        ActionBody::CollectionItem => &[
            r#"input[aria-label="Content ID"]"#,
            r#"input[aria-label="Search for item"]"#,
            r#"input[aria-label="Title"]"#,
        ],
        ActionBody::ConversationMessage | ActionBody::RoomMessage => &[
            r#"textarea[aria-label="Message"]"#,
            r#"input[aria-label="Message"]"#,
        ],
        ActionBody::DownloadFiles => &[
            r#"input[aria-label="Folder"]"#,
            r#"input[aria-label="Search for item"]"#,
        ],
        ActionBody::FeedPreview => &[
            r#"textarea[aria-label="Playlist rows"]"#,
            r#"input[aria-label="Playlist source"]"#,
            r#"input[aria-label="Playlist name"]"#,
            r#"input[aria-label="Playlist text"]"#,
        ],
        ActionBody::JsonString => &[
            r#"input[aria-label="Search rooms"]"#,
            r#"input[aria-label="Room"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Artist Name"]"#,
        ],
        ActionBody::MusicBrainzTarget => &[r#"input[aria-label="Release ID"]"#],
        ActionBody::LibraryPath => &[r#"input[aria-label="Library Path"]"#],
        ActionBody::NameDescription => &[
            r#"input[aria-label="Title"]"#,
            r#"input[aria-label="Group Name"]"#,
            r#"input[aria-label="Collection name"]"#,
            r#"input[aria-label="Description"]"#,
        ],
        ActionBody::Permissions => &[r#"select[aria-label="Permissions"]"#],
        ActionBody::SearchText => &[
            r#"input[aria-label="Search text"]"#,
            r#"input[aria-label="Search Text"]"#,
            r#"input[aria-label="Wanted search"]"#,
            r#"input[aria-label="Artist Name"]"#,
            r#"input[aria-label="Seed artist or query"]"#,
            r#"textarea[aria-label="Playlist rows"]"#,
        ],
        ActionBody::ContactDiscovery
        | ActionBody::ContactInvite
        | ActionBody::ShareGrant
        | ActionBody::ShareGroupMember
        | ActionBody::Username => &[
            r#"input[aria-label="Username"]"#,
            r#"input[aria-label="Soulseek Username"]"#,
            r#"input[aria-label="Contact username"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Nickname"]"#,
        ],
        ActionBody::SongIdSource => &[r#"input[aria-label="SongID source"]"#],
        ActionBody::EnabledFalse
        | ActionBody::EnabledTrue
        | ActionBody::InviteRequest
        | ActionBody::None => &[],
    }
}

#[cfg(target_arch = "wasm32")]
fn native_generic_value_selectors() -> &'static [&'static str] {
    &[
        "input:not([type=checkbox]):not([type=radio])",
        "textarea",
        "select",
    ]
}

#[cfg(target_arch = "wasm32")]
fn native_action_target(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) -> Option<String> {
    if action.path.contains(":collectionId") {
        let workspace = button.closest(".slskr-native-workspace").ok().flatten()?;
        let collection_id = button_native_row_attribute(button, "data-slskr-native-collection-id")
            .or_else(|| {
                selected_native_row_attribute(&workspace, "data-slskr-native-collection-id")
            })?;
        return route_target_segment(&collection_id);
    }
    if action.path.contains(":id")
        && !action.path.contains(":username")
        && !action.path.contains(":roomName")
    {
        return None;
    }
    if !action.path.contains(":username") && !action.path.contains(":roomName") {
        return None;
    }
    let workspace = button.closest(".slskr-native-workspace").ok().flatten()?;
    if action.path.contains(":roomName") {
        let room = button_native_row_attribute(button, "data-slskr-native-room-name")
            .or_else(|| selected_native_row_attribute(&workspace, "data-slskr-native-room-name"))
            .or_else(|| first_workspace_value(&workspace, r#"input[aria-label="Search rooms"]"#))
            .or_else(|| button_native_row_target(button))
            .or_else(|| selected_native_row_target(&workspace))?;
        return route_target_segment(&room);
    }
    if matches!(
        action.body,
        ActionBody::BrowseDirectory | ActionBody::DownloadFiles
    ) {
        for selector in [
            r#"input[aria-label="Username"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Soulseek Username"]"#,
        ] {
            if let Some(value) = first_workspace_value(&workspace, selector)
                .filter(|value| safe_route_segment(value))
            {
                return Some(value);
            }
        }
    }
    if let Some(value) = button_native_row_target(button).filter(|value| safe_route_segment(value))
    {
        return Some(value);
    }
    if let Some(value) =
        selected_native_row_target(&workspace).filter(|value| safe_route_segment(value))
    {
        return Some(value);
    }
    let selectors: &[&str] = if action.path.contains(":roomName") {
        &[
            r#"input[aria-label="Search rooms"]"#,
            r#"input[aria-label="Room"]"#,
        ]
    } else {
        &[
            r#"input[aria-label="Username"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Contact username"]"#,
            r#"input[aria-label="Soulseek Username"]"#,
        ]
    };
    for selector in selectors {
        if let Some(value) = first_workspace_value(&workspace, selector).filter(|value| {
            value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        }) {
            return Some(value);
        }
    }
    document_selected_native_row_title(document).filter(|value| safe_route_segment(value))
}

#[cfg(target_arch = "wasm32")]
fn native_action_target_for_ui(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
    value: &str,
) -> Option<String> {
    if let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() {
        if let Some(target) = native_action_target(document, button, action) {
            return Some(target);
        }
        if let Some(target) = selected_native_row_target(&workspace) {
            return route_target_segment(&target);
        }
    }
    if action.path.contains(":collectionId") {
        return document
            .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
            .ok()
            .flatten()
            .and_then(|row| row.get_attribute("data-slskr-native-collection-id"))
            .filter(|target| safe_route_segment(target))
            .and_then(|target| route_target_segment(&target));
    }
    if action.path.contains(":username") || action.path.contains(":roomName") {
        let selectors: &[&str] = if action.path.contains(":roomName") {
            &[
                r#"input[aria-label="Search rooms"]"#,
                r#"input[aria-label="Room"]"#,
            ]
        } else {
            &[
                r#"input[aria-label="Username"]"#,
                r#"input[aria-label="Chat username"]"#,
                r#"input[aria-label="Contact username"]"#,
                r#"input[aria-label="Soulseek Username"]"#,
            ]
        };
        for selector in selectors {
            if let Ok(Some(element)) = document.query_selector(selector) {
                if let Some(target) = form_control_value(&element)
                    .map(|target| target.trim().to_owned())
                    .filter(|target| safe_route_segment(target))
                    .and_then(|target| route_target_segment(&target))
                {
                    return Some(target);
                }
            }
        }
        if let Some(target) = route_target_segment(value) {
            return Some(target);
        }
        return document
            .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
            .ok()
            .flatten()
            .and_then(|row| {
                [
                    "data-slskr-native-username",
                    "data-slskr-native-peer",
                    "data-slskr-native-contact",
                    "data-slskr-native-detail",
                    "data-slskr-native-title",
                ]
                .iter()
                .find_map(|attribute| row.get_attribute(attribute))
            })
            .filter(|target| safe_route_segment(target))
            .and_then(|target| route_target_segment(&target));
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn native_route_action_is_ready(
    document: &web_sys::Document,
    button: &web_sys::Element,
    route_path: &str,
    action: RouteAction,
    target_value: &str,
    body_value: &str,
) -> bool {
    if !native_action_value_is_valid(document, action.label, action, body_value) {
        return false;
    }
    if action.body == ActionBody::ShareGrant {
        let collection_id = button_native_row_attribute(button, "data-slskr-native-collection-id")
            .or_else(|| {
                button
                    .closest(".slskr-native-workspace")
                    .ok()
                    .flatten()
                    .and_then(|workspace| {
                        selected_native_row_attribute(
                            &workspace,
                            "data-slskr-native-collection-id",
                        )
                    })
            })
            .or_else(|| {
                document
                    .query_selector(
                        "[data-slskr-native-select][aria-selected=\"true\"][data-slskr-native-collection-id]",
                    )
                    .ok()
                    .flatten()
                    .and_then(|row| row.get_attribute("data-slskr-native-collection-id"))
            });
        if collection_id
            .as_deref()
            .filter(|value| safe_route_segment(value))
            .is_none()
        {
            native_set_action_status(
                document,
                action.label,
                "Select a live collection before creating a share grant.",
            );
            return false;
        }
    }
    if action.path.contains(":id") || action.path.contains(":itemId") {
        let route_has_id = route_path
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            > 1;
        if !route_has_id && native_action_id(document, button, action).is_none() {
            native_set_action_status(
                document,
                action.label,
                "Select a live row before running this action.",
            );
            return false;
        }
    }
    if action.path.contains(":username")
        || action.path.contains(":roomName")
        || action.path.contains(":collectionId")
    {
        if native_action_target_for_ui(document, button, action, target_value).is_none() {
            native_set_action_status(
                document,
                action.label,
                "Enter or select a live target before running this action.",
            );
            return false;
        }
    }
    true
}

#[cfg(target_arch = "wasm32")]
fn route_target_segment(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '?' | '#'))
    {
        return None;
    }
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    Some(encoded)
}

#[cfg(target_arch = "wasm32")]
fn native_action_id(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) -> Option<String> {
    if !action.path.contains(":id") && !action.path.contains(":itemId") {
        return None;
    }
    for attr in [
        "data-slskr-native-item-id",
        "data-slskr-native-transfer-id",
        "data-slskr-native-wishlist-id",
        "data-slskr-native-grant-id",
        "data-slskr-native-share-group-id",
        "data-slskr-native-collection-id",
        "data-slskr-native-search-id",
    ] {
        if let Some(value) =
            button_native_row_attribute(button, attr).filter(|value| safe_route_segment(value))
        {
            return Some(value);
        }
        if let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() {
            if let Some(value) = selected_native_row_attribute(&workspace, attr)
                .filter(|value| safe_route_segment(value))
            {
                return Some(value);
            }
        }
        if let Some(value) = document
            .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
            .ok()
            .flatten()
            .and_then(|row| row.get_attribute(attr))
            .filter(|value| safe_route_segment(value))
        {
            return Some(value);
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn first_workspace_value(workspace: &web_sys::Element, selector: &str) -> Option<String> {
    let nodes = workspace.query_selector_all(selector).ok()?;
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(value) = form_control_value(&element)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_detail(workspace: &web_sys::Element) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-detail"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_title(workspace: &web_sys::Element) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_titles(workspace: &web_sys::Element) -> Option<String> {
    let rows = workspace
        .query_selector_all("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()?;
    let mut values = Vec::new();
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(value) = row
            .get_attribute("data-slskr-native-title")
            .filter(|value| !value.trim().is_empty())
        {
            values.push(value);
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values.join("\n"))
    }
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_attribute(workspace: &web_sys::Element, attribute: &str) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute(attribute))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_attribute_values(
    workspace: &web_sys::Element,
    attribute: &str,
) -> Option<String> {
    let rows = workspace
        .query_selector_all("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()?;
    let mut values = Vec::new();
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(value) = row
            .get_attribute(attribute)
            .filter(|value| !value.trim().is_empty())
        {
            values.push(value);
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values.join("\n"))
    }
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_target(workspace: &web_sys::Element) -> Option<String> {
    selected_native_row_detail(workspace)
        .filter(|value| safe_route_segment(value))
        .or_else(|| selected_native_row_title(workspace).filter(|value| safe_route_segment(value)))
}

#[cfg(target_arch = "wasm32")]
fn button_native_row_target(button: &web_sys::Element) -> Option<String> {
    button
        .closest("[data-slskr-native-select]")
        .ok()
        .flatten()
        .and_then(|row| {
            row.get_attribute("data-slskr-native-detail")
                .filter(|value| safe_route_segment(value))
                .or_else(|| {
                    row.get_attribute("data-slskr-native-title")
                        .filter(|value| safe_route_segment(value))
                })
        })
}

#[cfg(target_arch = "wasm32")]
fn button_native_row_attribute(button: &web_sys::Element, attribute: &str) -> Option<String> {
    button
        .closest("[data-slskr-native-select]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute(attribute))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn document_selected_native_row_title(document: &web_sys::Document) -> Option<String> {
    document
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(any(target_arch = "wasm32", test))]
fn native_action_fallback(body: ActionBody) -> String {
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

#[cfg(target_arch = "wasm32")]
fn form_control_value(element: &web_sys::Element) -> Option<String> {
    if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        return Some(input.value());
    }
    if let Some(textarea) = element.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        return Some(textarea.value());
    }
    if let Some(select) = element.dyn_ref::<web_sys::HtmlSelectElement>() {
        return Some(select.value());
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn mount_native_filters(document: &web_sys::Document) -> Result<(), JsValue> {
    let inputs = document.query_selector_all("[data-slskr-native-filter]")?;
    for input_index in 0..inputs.length() {
        let Some(node) = inputs.item(input_index) else {
            continue;
        };
        let input: web_sys::HtmlInputElement = node.dyn_into()?;
        let workspace = input
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native filter is outside workspace"))?;
        let restored = restore_native_filter(document, &workspace);
        if restored.is_empty() {
            update_native_filter_count(&workspace);
        } else {
            input.set_value(&restored);
            apply_native_filter(&workspace, &restored);
        }

        let workspace_for_input = workspace.clone();
        let callback =
            Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
                let term = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value().to_lowercase())
                    .unwrap_or_default();
                apply_native_filter(&workspace_for_input, &term);
                persist_native_filter(&workspace_for_input, &term);
            }));
        input.add_event_listener_with_callback("input", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let clear_buttons = document.query_selector_all("[data-slskr-native-filter-clear]")?;
    for button_index in 0..clear_buttons.length() {
        let Some(node) = clear_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native filter clear is outside workspace"))?;
        let workspace_for_clear = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                if let Ok(Some(filter)) =
                    workspace_for_clear.query_selector("[data-slskr-native-filter]")
                {
                    if let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() {
                        input.set_value("");
                    }
                }
                apply_native_filter(&workspace_for_clear, "");
                persist_native_filter(&workspace_for_clear, "");
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let select_buttons = document.query_selector_all("[data-slskr-native-select-visible]")?;
    for button_index in 0..select_buttons.length() {
        let Some(node) = select_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native select visible is outside workspace"))?;
        let workspace_for_select = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                select_visible_native_rows(&workspace_for_select);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let selection_clear_buttons =
        document.query_selector_all("[data-slskr-native-clear-selection]")?;
    for button_index in 0..selection_clear_buttons.length() {
        let Some(node) = selection_clear_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native clear selection is outside workspace"))?;
        let workspace_for_clear = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                clear_native_selection(&workspace_for_clear);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let reset_buttons = document.query_selector_all("[data-slskr-native-reset-state]")?;
    for button_index in 0..reset_buttons.length() {
        let Some(node) = reset_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native reset table is outside workspace"))?;
        let workspace_for_reset = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                reset_native_table_state(&workspace_for_reset);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn native_state_key(workspace: &web_sys::Element, suffix: &str) -> Option<String> {
    let route = workspace
        .closest("[data-route]")
        .ok()
        .flatten()
        .and_then(|route| route.get_attribute("data-route"))?;
    Some(format!("slskr.native.{route}.{suffix}"))
}

#[cfg(target_arch = "wasm32")]
fn session_storage_for_workspace(workspace: &web_sys::Element) -> Option<web_sys::Storage> {
    workspace
        .owner_document()
        .and_then(|document| document.default_view())
        .and_then(|window| window.session_storage().ok().flatten())
}

#[cfg(target_arch = "wasm32")]
fn restore_native_filter(document: &web_sys::Document, workspace: &web_sys::Element) -> String {
    let Some(key) = native_state_key(workspace, "filter") else {
        return String::new();
    };
    let Some(storage) = session_storage_for_workspace(workspace) else {
        return String::new();
    };
    let value = storage.get_item(&key).ok().flatten().unwrap_or_default();
    if !value.is_empty() {
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html(&format!(
                "<strong>Restored</strong> filter {}",
                escape_html(&value)
            ));
        }
    }
    value
}

#[cfg(target_arch = "wasm32")]
fn persist_native_filter(workspace: &web_sys::Element, term: &str) {
    let Some(key) = native_state_key(workspace, "filter") else {
        return;
    };
    let Some(storage) = session_storage_for_workspace(workspace) else {
        return;
    };
    if term.is_empty() {
        let _ = storage.remove_item(&key);
    } else {
        let _ = storage.set_item(&key, term);
    }
}

#[cfg(target_arch = "wasm32")]
fn reset_native_table_state(workspace: &web_sys::Element) {
    if let Ok(Some(filter)) = workspace.query_selector("[data-slskr-native-filter]") {
        if let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value("");
        }
    }
    apply_native_filter(workspace, "");
    persist_native_filter(workspace, "");
    clear_native_selection(workspace);
    reset_native_sort(workspace);

    if let Some(document) = workspace.owner_document() {
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html("<strong>Reset</strong> table controls cleared");
        }
        show_toast(&document, "Table controls reset");
    }
}

#[cfg(target_arch = "wasm32")]
fn apply_native_filter(workspace: &web_sys::Element, term: &str) {
    let mut visible = 0;
    let mut total = 0;
    if let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            total += 1;
            let haystack = [
                row.get_attribute("data-slskr-native-title"),
                row.get_attribute("data-slskr-native-detail"),
                row.get_attribute("data-slskr-native-meta"),
                row.get_attribute("data-slskr-native-action"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
            let matches = term.is_empty() || haystack.contains(term);
            if matches {
                visible += 1;
                let _ = row.remove_attribute("hidden");
            } else {
                let _ = row.set_attribute("hidden", "");
            }
        }
    }
    set_native_filter_count(workspace, visible, total);
}

#[cfg(target_arch = "wasm32")]
fn update_native_filter_count(workspace: &web_sys::Element) {
    let total = workspace
        .query_selector_all("[data-slskr-native-select]")
        .map(|rows| rows.length())
        .unwrap_or_default();
    set_native_filter_count(workspace, total, total);
}

#[cfg(target_arch = "wasm32")]
fn set_native_filter_count(workspace: &web_sys::Element, visible: u32, total: u32) {
    if let Ok(Some(count)) = workspace.query_selector("[data-slskr-native-count]") {
        count.set_text_content(Some(&format!("{visible} / {total} rows")));
    }
}

#[cfg(target_arch = "wasm32")]
fn select_visible_native_rows(workspace: &web_sys::Element) {
    let mut selected = 0;
    let mut first_selected: Option<web_sys::Element> = None;
    if let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            if row.has_attribute("hidden") {
                continue;
            }
            selected += 1;
            if first_selected.is_none() {
                first_selected = Some(row.clone());
            }
            let _ = row.set_attribute("aria-selected", "true");
            if let Ok(Some(input)) = row.query_selector("input[type=checkbox]") {
                if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                    input.set_checked(true);
                }
            }
        }
    }
    update_native_selection_summary(workspace, selected, "selected", first_selected.as_ref());
    if let Some(row) = first_selected.as_ref() {
        update_native_inspector(workspace, selected, Some(row));
        update_native_selection_preview(workspace, selected, Some(row));
    } else {
        update_native_inspector(workspace, selected, None);
        update_native_selection_preview(workspace, selected, None);
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_native_selection(workspace: &web_sys::Element) {
    if let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = row.remove_attribute("aria-selected");
            if let Ok(Some(input)) = row.query_selector("input[type=checkbox]") {
                if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                    input.set_checked(false);
                }
            }
        }
    }
    update_native_selection_summary(workspace, 0, "selected", None);
    update_native_inspector(workspace, 0, None);
    update_native_selection_preview(workspace, 0, None);
}

#[cfg(target_arch = "wasm32")]
fn update_native_selection_summary(
    workspace: &web_sys::Element,
    selected: u32,
    label: &str,
    row: Option<&web_sys::Element>,
) {
    let Some(document) = workspace.owner_document() else {
        return;
    };
    let message = if selected == 0 {
        "No rows selected".to_string()
    } else if selected == 1 {
        row.and_then(|row| row.get_attribute("data-slskr-native-action-summary"))
            .or_else(|| row.and_then(|row| row.get_attribute("data-slskr-native-title")))
            .unwrap_or_else(|| "1 row selected".to_string())
    } else {
        format!("{selected} visible rows {label}")
    };
    if let Some(status) = document.get_element_by_id("slskr-native-selection-status") {
        status.set_inner_html(&format!(
            "<strong>Selection</strong><span>{}</span>",
            escape_html(&message)
        ));
    }
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>Selection</strong> {}",
            escape_html(&message)
        ));
    }
}

#[cfg(target_arch = "wasm32")]
fn update_native_inspector(
    workspace: &web_sys::Element,
    selected: u32,
    row: Option<&web_sys::Element>,
) {
    let Ok(Some(inspector)) = workspace.query_selector("#slskr-native-inspector") else {
        return;
    };
    let count = if selected == 1 {
        "1 selected".to_string()
    } else {
        format!("{selected} selected")
    };
    if let Ok(Some(element)) = inspector.query_selector("[data-slskr-native-inspector-count]") {
        element.set_text_content(Some(&count));
    }
    let title = row
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Nothing selected".to_string()
            } else {
                "Multiple rows selected".to_string()
            }
        });
    let detail = row
        .and_then(|row| row.get_attribute("data-slskr-native-action-summary"))
        .or_else(|| row.and_then(|row| row.get_attribute("data-slskr-native-detail")))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Use the table to choose an item.".to_string()
            } else {
                "Bulk actions will apply to all selected visible rows.".to_string()
            }
        });
    let meta = row
        .and_then(|row| row.get_attribute("data-slskr-native-meta"))
        .unwrap_or_else(|| count.clone());
    let action = row
        .and_then(|row| row.get_attribute("data-slskr-native-action"))
        .unwrap_or_else(|| "Review".to_string());
    let fields = row
        .and_then(|row| row.get_attribute("data-slskr-native-detail-list"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Selection fields will appear here.".to_string()
            } else {
                "Bulk selection across visible rows.".to_string()
            }
        });
    for (selector, value) in [
        ("[data-slskr-native-inspector-title]", title),
        ("[data-slskr-native-inspector-detail]", detail),
        ("[data-slskr-native-inspector-meta]", meta),
        ("[data-slskr-native-inspector-action]", action),
        ("[data-slskr-native-inspector-fields]", fields),
    ] {
        if let Ok(Some(element)) = inspector.query_selector(selector) {
            element.set_text_content(Some(&value));
        }
    }
    if let Ok(Some(actions)) = inspector.query_selector("[data-slskr-native-inspector-actions]") {
        let menu = row
            .and_then(|row| row.get_attribute("data-slskr-native-action-menu"))
            .unwrap_or_else(|| {
                if selected == 0 {
                    "Review Selection | Queue Selected".to_string()
                } else {
                    "Review Selection | Run Selected".to_string()
                }
            });
        let html = menu
            .split('|')
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(|label| {
                format!(
                    r#"<button type="button" data-slskr-native-row-action="{}">{}</button>"#,
                    escape_html(label),
                    escape_html(label)
                )
            })
            .collect::<Vec<_>>()
            .join("");
        actions.set_inner_html(&html);
    }
}

#[cfg(target_arch = "wasm32")]
fn update_native_selection_preview(
    workspace: &web_sys::Element,
    selected: u32,
    row: Option<&web_sys::Element>,
) {
    let count = if selected == 1 {
        "1 selected".to_string()
    } else {
        format!("{selected} selected")
    };
    let title = row
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Nothing selected".to_string()
            } else {
                "Multiple rows selected".to_string()
            }
        });
    let detail = row
        .and_then(|row| row.get_attribute("data-slskr-native-action-summary"))
        .or_else(|| row.and_then(|row| row.get_attribute("data-slskr-native-detail")))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Choose a row to review actions.".to_string()
            } else {
                "Bulk actions will apply to the selected visible rows.".to_string()
            }
        });
    let meta = row
        .and_then(|row| row.get_attribute("data-slskr-native-meta"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Waiting".to_string()
            } else {
                count.clone()
            }
        });
    let action = row
        .and_then(|row| row.get_attribute("data-slskr-native-action"))
        .unwrap_or_else(|| "Review".to_string());
    let fields = row
        .and_then(|row| row.get_attribute("data-slskr-native-detail-list"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Selection fields will appear here.".to_string()
            } else {
                "Bulk selection across visible rows.".to_string()
            }
        });
    let title = if selected > 1 {
        format!("{selected} rows selected")
    } else {
        title
    };
    for (selector, value) in [
        ("[data-slskr-native-preview-count]", count),
        ("[data-slskr-native-preview-title]", title),
        ("[data-slskr-native-preview-detail]", detail),
        ("[data-slskr-native-preview-fields]", fields),
        ("[data-slskr-native-preview-meta]", meta),
        ("[data-slskr-native-preview-action]", action),
    ] {
        if let Ok(elements) = workspace.query_selector_all(selector) {
            for index in 0..elements.length() {
                let Some(node) = elements.item(index) else {
                    continue;
                };
                if let Ok(element) = node.dyn_into::<web_sys::Element>() {
                    element.set_text_content(Some(&value));
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_native_sorters(document: &web_sys::Document) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-native-sort]")?;
    for button_index in 0..buttons.length() {
        let Some(node) = buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let button_for_click = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                sort_native_table(&button_for_click);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    restore_native_sort(document);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn sort_native_table(button: &web_sys::Element) {
    let index = button
        .get_attribute("data-slskr-native-sort")
        .unwrap_or_else(|| "0".to_string());
    let next_direction = if button
        .get_attribute("aria-sort")
        .is_some_and(|direction| direction == "ascending")
    {
        "descending"
    } else {
        "ascending"
    };
    apply_native_sort(button, &index, next_direction, true);
}

#[cfg(target_arch = "wasm32")]
fn apply_native_sort(button: &web_sys::Element, index: &str, direction: &str, persist: bool) {
    let Some(table) = button.closest("table").ok().flatten() else {
        return;
    };
    if let Ok(sort_buttons) = table.query_selector_all("[data-slskr-native-sort]") {
        for button_index in 0..sort_buttons.length() {
            let Some(node) = sort_buttons.item(button_index) else {
                continue;
            };
            let Ok(sort_button) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let active = sort_button
                .get_attribute("data-slskr-native-sort")
                .is_some_and(|value| value == index);
            let _ = sort_button.set_attribute("aria-sort", if active { direction } else { "none" });
        }
    }

    let Some(tbody) = table.query_selector("tbody").ok().flatten() else {
        return;
    };
    let Ok(row_nodes) = tbody.query_selector_all("[data-slskr-native-select]") else {
        return;
    };
    let attr = format!("data-slskr-native-sort-{index}");
    let mut rows = Vec::new();
    for row_index in 0..row_nodes.length() {
        let Some(node) = row_nodes.item(row_index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        rows.push(row);
    }
    rows.sort_by(|left, right| {
        let left_value = left.get_attribute(&attr).unwrap_or_default().to_lowercase();
        let right_value = right
            .get_attribute(&attr)
            .unwrap_or_default()
            .to_lowercase();
        if direction == "descending" {
            right_value.cmp(&left_value)
        } else {
            left_value.cmp(&right_value)
        }
    });
    for row in rows {
        let _ = tbody.append_child(&row);
    }
    if persist {
        if let Some(workspace) = table.closest(".slskr-native-workspace").ok().flatten() {
            persist_native_sort(&workspace, index, direction);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn restore_native_sort(document: &web_sys::Document) {
    let Ok(workspaces) = document.query_selector_all(".slskr-native-workspace") else {
        return;
    };
    for workspace_index in 0..workspaces.length() {
        let Some(node) = workspaces.item(workspace_index) else {
            continue;
        };
        let Ok(workspace) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(key) = native_state_key(&workspace, "sort") else {
            continue;
        };
        let Some(storage) = session_storage_for_workspace(&workspace) else {
            continue;
        };
        let Some(value) = storage.get_item(&key).ok().flatten() else {
            continue;
        };
        let Some((index, direction)) = value.split_once(':') else {
            continue;
        };
        if !matches!(direction, "ascending" | "descending") {
            continue;
        }
        let selector = format!(r#"[data-slskr-native-sort="{index}"]"#);
        let Ok(Some(button)) = workspace.query_selector(&selector) else {
            continue;
        };
        apply_native_sort(&button, index, direction, false);
    }
}

#[cfg(target_arch = "wasm32")]
fn persist_native_sort(workspace: &web_sys::Element, index: &str, direction: &str) {
    let Some(key) = native_state_key(workspace, "sort") else {
        return;
    };
    let Some(storage) = session_storage_for_workspace(workspace) else {
        return;
    };
    let _ = storage.set_item(&key, &format!("{index}:{direction}"));
}

#[cfg(target_arch = "wasm32")]
fn reset_native_sort(workspace: &web_sys::Element) {
    if let Some(key) = native_state_key(workspace, "sort") {
        if let Some(storage) = session_storage_for_workspace(workspace) {
            let _ = storage.remove_item(&key);
        }
    }
    if let Ok(sort_buttons) = workspace.query_selector_all("[data-slskr-native-sort]") {
        for button_index in 0..sort_buttons.length() {
            let Some(node) = sort_buttons.item(button_index) else {
                continue;
            };
            let Ok(sort_button) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = sort_button.set_attribute("aria-sort", "none");
        }
    }
    let Ok(tables) = workspace.query_selector_all("table") else {
        return;
    };
    for table_index in 0..tables.length() {
        let Some(table_node) = tables.item(table_index) else {
            continue;
        };
        let Ok(table) = table_node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(tbody) = table.query_selector("tbody").ok().flatten() else {
            continue;
        };
        let Ok(row_nodes) = tbody.query_selector_all("[data-slskr-native-select]") else {
            continue;
        };
        let mut rows = Vec::new();
        for row_index in 0..row_nodes.length() {
            let Some(node) = row_nodes.item(row_index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            rows.push(row);
        }
        rows.sort_by_key(|row| {
            row.get_attribute("data-slskr-native-index")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(usize::MAX)
        });
        for row in rows {
            let _ = tbody.append_child(&row);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn show_toast(document: &web_sys::Document, message: &str) {
    let region = document
        .get_element_by_id("slskr-toast-region")
        .or_else(|| {
            let element = document.create_element("div").ok()?;
            element.set_id("slskr-toast-region");
            element.set_class_name("slskr-toast-region");
            let _ = element.set_attribute("aria-live", "polite");
            let body = document.body()?;
            let _ = body.append_child(&element);
            Some(element)
        });
    let Some(region) = region else {
        return;
    };
    region.set_inner_html("");
    let Ok(toast) = document.create_element("div") else {
        return;
    };
    toast.set_class_name("slskr-toast");
    toast.set_text_content(Some(message));
    let _ = region.append_child(&toast);
}

#[cfg(target_arch = "wasm32")]
fn select_native_row(document: &web_sys::Document, row: &web_sys::Element) {
    if let Ok(rows) = document.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("aria-selected");
        }
    }
    let _ = row.set_attribute("aria-selected", "true");
    if let Ok(Some(input)) = row.query_selector("input[type=checkbox]") {
        if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_checked(true);
        }
    }

    let title = row
        .get_attribute("data-slskr-native-title")
        .unwrap_or_else(|| "Selected row".to_string());
    let detail = row
        .get_attribute("data-slskr-native-action-summary")
        .or_else(|| row.get_attribute("data-slskr-native-detail"))
        .unwrap_or_default();
    let meta = row
        .get_attribute("data-slskr-native-meta")
        .unwrap_or_default();
    let action = row
        .get_attribute("data-slskr-native-action")
        .unwrap_or_else(|| "Review".to_string());
    let message = format!(
        "<strong>{}</strong><span>{}</span><span>{}</span><button type=\"button\">{}</button>",
        escape_html(&title),
        escape_html(&detail),
        escape_html(&meta),
        escape_html(&action)
    );
    if let Some(status) = document.get_element_by_id("slskr-native-selection-status") {
        status.set_inner_html(&message);
    }
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>Selected</strong> {}",
            escape_html(&title)
        ));
    }
    if let Some(workspace) = row.closest(".slskr-native-workspace").ok().flatten() {
        update_native_inspector(&workspace, 1, Some(row));
        update_native_selection_preview(&workspace, 1, Some(row));
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_live_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    if let Some(button) = document.query_selector("[data-slskr-refresh-route]")? {
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let window = window.clone();
                let document = document.clone();
                native_set_action_status(&document, "Refresh", "Refreshing live route data.");
                wasm_bindgen_futures::spawn_local(async move {
                    match refresh_route_data(&window).await {
                        Ok(()) => native_set_action_status(
                            &document,
                            "Refresh",
                            "Live route data refreshed.",
                        ),
                        Err(error) => native_set_action_status(
                            &document,
                            "Refresh",
                            &error
                                .as_string()
                                .unwrap_or_else(|| "route refresh failed".to_string()),
                        ),
                    }
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    if let Some(button) = document.query_selector("[data-slskr-focus-filter]")? {
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                focus_first_card_filter(&document);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    if let Some(button) = document.query_selector("[data-slskr-clear-filters]")? {
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                clear_all_card_filters(&document);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn reference_action_value(document: &web_sys::Document) -> String {
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
fn set_reference_status(document: &web_sys::Document, label: &str, message: &str) {
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
fn copy_reference_text(window: &web_sys::Window, document: &web_sys::Document, text: String) {
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
fn mount_reference_actions(
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
fn mount_global_shortcuts(
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
fn keyboard_event_started_in_text_control(document: &web_sys::Document) -> bool {
    document
        .active_element()
        .map(|element| matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"))
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn focus_first_card_filter(document: &web_sys::Document) {
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
fn clear_all_card_filters(document: &web_sys::Document) {
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

#[cfg(target_arch = "wasm32")]
fn set_live_status(document: &web_sys::Document, message: &str) {
    if let Some(status) = document.get_element_by_id("slskr-live-status") {
        status.set_text_content(Some(message));
    }
}

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

fn percent_encode_player_stream_component(value: &str) -> String {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchCandidateRank {
    pub reasons: Vec<String>,
    pub score: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDuplicateGroup {
    pub candidate_count: usize,
    pub folded_count: usize,
    pub key: String,
    pub providers: Vec<String>,
    pub usernames: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchActionPreview {
    pub candidate_score: Option<u32>,
    pub file_count: usize,
    pub filenames: Vec<String>,
    pub locked_count: usize,
    pub provider_labels: Vec<String>,
    pub route: String,
    pub total_size_bytes: u64,
    pub username: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExperiencePreference {
    pub default_value: &'static str,
    pub group: &'static str,
    pub id: &'static str,
    pub input: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutomationRecipe {
    pub approval_gate: &'static str,
    pub cadence: &'static str,
    pub cooldown: &'static str,
    pub description: &'static str,
    pub enabled_by_default: bool,
    pub file_impact: &'static str,
    pub id: &'static str,
    pub max_run_time: &'static str,
    pub network_impact: &'static str,
    pub title: &'static str,
}

pub const fn experience_preferences() -> &'static [ExperiencePreference] {
    &[
        ExperiencePreference {
            default_value: "balanced",
            group: "Search",
            id: "searchRankingProfile",
            input: "text",
            label: "Ranking Profile",
        },
        ExperiencePreference {
            default_value: "lossless",
            group: "Search",
            id: "searchPreferredCondition",
            input: "text",
            label: "Preferred Condition",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Search",
            id: "searchDuplicateFolding",
            input: "checkbox",
            label: "Fold duplicate results",
        },
        ExperiencePreference {
            default_value: "detailed",
            group: "Search",
            id: "searchActionPreviewDensity",
            input: "text",
            label: "Action Preview Density",
        },
        ExperiencePreference {
            default_value: "current",
            group: "Player",
            id: "playerRadioSeedMode",
            input: "text",
            label: "Radio Seed",
        },
        ExperiencePreference {
            default_value: "manual",
            group: "Player",
            id: "playerScrobbleMode",
            input: "text",
            label: "Scrobble Mode",
        },
        ExperiencePreference {
            default_value: "last",
            group: "Player",
            id: "playerDefaultVisualizer",
            input: "text",
            label: "Default Visualizer",
        },
        ExperiencePreference {
            default_value: "false",
            group: "Player",
            id: "playerQueueAutoFill",
            input: "checkbox",
            label: "Enable queue auto-fill",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Player",
            id: "playerShowRatings",
            input: "checkbox",
            label: "Show ratings",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Player",
            id: "playerCaptureHistory",
            input: "checkbox",
            label: "Capture history",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Player",
            id: "playerKeyboardShortcuts",
            input: "checkbox",
            label: "Keyboard shortcuts",
        },
        ExperiencePreference {
            default_value: "all",
            group: "Discovery",
            id: "discoveryApprovalFilter",
            input: "text",
            label: "Approval Filter",
        },
        ExperiencePreference {
            default_value: "0.70",
            group: "Discovery",
            id: "discoveryConfidenceFloor",
            input: "text",
            label: "Confidence Floor",
        },
        ExperiencePreference {
            default_value: "14",
            group: "Discovery",
            id: "discoveryStaleDays",
            input: "text",
            label: "Stale Days",
        },
        ExperiencePreference {
            default_value: "false",
            group: "Messages",
            id: "messagesDenseMode",
            input: "checkbox",
            label: "Dense mode",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Messages",
            id: "messagesPinnedRestore",
            input: "checkbox",
            label: "Restore pinned conversations",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Messages",
            id: "messagesUnreadBadges",
            input: "checkbox",
            label: "Unread badges",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Messages",
            id: "messagesSearchEnabled",
            input: "checkbox",
            label: "Message search",
        },
    ]
}

pub const fn automation_recipes() -> &'static [AutomationRecipe] {
    &[
        AutomationRecipe {
            approval_gate: "None required",
            cadence: "Continuous",
            cooldown: "5 minutes",
            description: "Checks connection, shares, paths, and credentials for setup drift.",
            enabled_by_default: true,
            file_impact: "Read only",
            id: "local-diagnostics",
            max_run_time: "30 seconds",
            network_impact: "Local",
            title: "Local Diagnostics",
        },
        AutomationRecipe {
            approval_gate: "None required",
            cadence: "Daily",
            cooldown: "24 hours",
            description:
                "Surfaces stale share-cache and library-scan reminders before users hit missing results.",
            enabled_by_default: true,
            file_impact: "Read only",
            id: "stale-cache-reminders",
            max_run_time: "1 minute",
            network_impact: "Local",
            title: "Share and Library Reminders",
        },
        AutomationRecipe {
            approval_gate: "None required",
            cadence: "Every 15 minutes",
            cooldown: "15 minutes",
            description: "Keeps local dashboard summaries fresh without contacting public peers.",
            enabled_by_default: true,
            file_impact: "Read only",
            id: "dashboard-refresh",
            max_run_time: "20 seconds",
            network_impact: "Local",
            title: "Dashboard Refresh",
        },
        AutomationRecipe {
            approval_gate: "Download approval",
            cadence: "Manual or scheduled",
            cooldown: "2 hours",
            description: "Retries failed Wishlist items using the selected acquisition profile.",
            enabled_by_default: false,
            file_impact: "Downloads after approval",
            id: "wishlist-retry",
            max_run_time: "20 minutes",
            network_impact: "Public peers possible",
            title: "Wishlist Retry",
        },
        AutomationRecipe {
            approval_gate: "Fix confirmation",
            cadence: "Manual or scheduled",
            cooldown: "24 hours",
            description: "Finds duplicates, dead files, metadata gaps, fake lossless files, and missing art.",
            enabled_by_default: false,
            file_impact: "Read only until fixed",
            id: "library-health-scan",
            max_run_time: "30 minutes",
            network_impact: "Local",
            title: "Library Health Scan",
        },
        AutomationRecipe {
            approval_gate: "Configured import success",
            cadence: "After import",
            cooldown: "10 minutes",
            description: "Asks configured media servers to rescan after successful library imports.",
            enabled_by_default: false,
            file_impact: "Media-server scan",
            id: "media-server-rescan",
            max_run_time: "2 minutes",
            network_impact: "Local network",
            title: "Media Server Rescan",
        },
        AutomationRecipe {
            approval_gate: "Explicit evidence publication opt-in",
            cadence: "Manual or scheduled",
            cooldown: "12 hours",
            description:
                "Publishes explicit opt-in signed quality and verification evidence to trusted mesh peers.",
            enabled_by_default: false,
            file_impact: "No file writes",
            id: "mesh-evidence-publish",
            max_run_time: "10 minutes",
            network_impact: "Trusted mesh",
            title: "Mesh Evidence Publish",
        },
    ]
}
