use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn handle_private_message_auto_response_action(
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
pub(super) fn handle_wishlist_policy_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> bool {
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
pub(super) fn handle_wishlist_ignored_result_action(
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
pub(super) fn native_import_wishlist_list(document: &web_sys::Document) {
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
