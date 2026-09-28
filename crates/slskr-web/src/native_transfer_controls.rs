//! Native Web native transfer controls.

use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_download_policy_controls(document: &web_sys::Document) -> Result<(), JsValue> {
    let selects = document.query_selector_all("[data-slskr-download-destination]")?;
    for index in 0..selects.length() {
        let Some(node) = selects.item(index) else {
            continue;
        };
        let select: web_sys::HtmlSelectElement = node.dyn_into()?;
        if select.has_attribute("data-slskr-mounted") {
            continue;
        }
        select.set_attribute("data-slskr-mounted", "true")?;
        if let Some(saved) = document
            .default_view()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| {
                storage
                    .get_item("slskr-download-destination")
                    .ok()
                    .flatten()
            })
        {
            if (0..select.length()).any(|option_index| {
                select
                    .item(option_index)
                    .is_some_and(|option| option.get_attribute("value").as_deref() == Some(&saved))
            }) {
                select.set_value(&saved);
            }
        }
        let document_for_change = document.clone();
        let select_for_change = select.clone();
        let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
            let value = select_for_change.value();
            if let Some(storage) = document_for_change
                .default_view()
                .and_then(|window| window.local_storage().ok().flatten())
            {
                let _ = storage.set_item("slskr-download-destination", &value);
            }
            if let Some(status) = document_for_change
                .query_selector("[data-slskr-download-policy-status]")
                .ok()
                .flatten()
            {
                status.set_text_content(Some(if value.trim().is_empty() {
                    "Using configured default"
                } else {
                    "Destination saved for this browser"
                }));
            }
        }));
        select.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn handle_download_policy_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> bool {
    let save = button.has_attribute("data-slskr-download-filter-save");
    let reset = button.has_attribute("data-slskr-download-filter-reset");
    if !save && !reset {
        return false;
    }
    let Some(panel) = button
        .closest("[data-slskr-download-policy]")
        .ok()
        .flatten()
    else {
        show_toast(document, "Download policy editor is unavailable");
        return true;
    };
    if reset {
        let Some(window) = document.default_view() else {
            return true;
        };
        show_toast(document, "Reloading download policy");
        let document = document.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = fetch_text(&window, &endpoint_url("/config/download-filter")).await;
            if let Some(status) = document
                .query_selector("[data-slskr-download-policy-status]")
                .ok()
                .flatten()
            {
                status.set_text_content(Some(match result {
                    Ok(_) => "Policy reloaded; refresh the page data to apply it",
                    Err(_) => "Policy reload failed",
                }));
            }
            let _ = refresh_route_data(&window).await;
        });
        return true;
    }
    let term_text = panel
        .query_selector("[data-slskr-download-filter]")
        .ok()
        .flatten()
        .and_then(|element| form_control_value(&element))
        .unwrap_or_default();
    let terms = term_text
        .lines()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    if terms.len() > 100 || terms.iter().any(|term| term.chars().count() > 256) {
        show_toast(document, "Use at most 100 terms of 256 characters each");
        return true;
    }
    let body = format!(
        r#"{{"exclude":[{}]}}"#,
        terms
            .iter()
            .map(|term| format!(r#""{}""#, escape_json_string(term)))
            .collect::<Vec<_>>()
            .join(",")
    );
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    show_toast(&document, "Saving download exclusions");
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(
            &window,
            &endpoint_url("/config/download-filter"),
            "PUT",
            Some(&body),
        )
        .await;
        if let Some(status) = document
            .query_selector("[data-slskr-download-policy-status]")
            .ok()
            .flatten()
        {
            status.set_text_content(Some(match &result {
                Ok(_) => "Download exclusions saved",
                Err(_) => "Download exclusions could not be saved",
            }));
        }
        if result.is_ok() {
            let _ = refresh_route_data(&window).await;
        }
    });
    true
}

#[cfg(target_arch = "wasm32")]
pub(super) fn handle_options_yaml_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> bool {
    let validate = button.has_attribute("data-slskr-options-yaml-validate");
    let save = button.has_attribute("data-slskr-options-yaml-save");
    if !validate && !save {
        return false;
    }
    let Some(panel) = button
        .closest("[data-slskr-system-configuration]")
        .ok()
        .flatten()
    else {
        show_toast(document, "Configuration editor is unavailable");
        return true;
    };
    let yaml = panel
        .query_selector("[data-slskr-options-yaml]")
        .ok()
        .flatten()
        .and_then(|element| form_control_value(&element))
        .unwrap_or_default();
    if yaml.trim().is_empty() {
        show_toast(document, "Configuration YAML is empty");
        return true;
    }
    let body = serde_json::Value::String(yaml).to_string();
    let method = if validate { "POST" } else { "PUT" };
    let endpoint = if validate {
        "/options/yaml/validate"
    } else {
        "/options/yaml"
    };
    let label = if validate {
        "Validating YAML"
    } else {
        "Saving YAML"
    };
    if let Some(status) = panel
        .query_selector("[data-slskr-options-yaml-status]")
        .ok()
        .flatten()
    {
        status.set_text_content(Some(label));
    }
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result =
            fetch_text_with_method(&window, &endpoint_url(endpoint), method, Some(&body)).await;
        if let Some(status) = document
            .query_selector("[data-slskr-options-yaml-status]")
            .ok()
            .flatten()
        {
            status.set_text_content(Some(match &result {
                Ok(response) if response.trim().is_empty() => {
                    if validate {
                        "YAML is valid"
                    } else {
                        "YAML saved"
                    }
                }
                Ok(response) => response,
                Err(_) => "YAML operation failed",
            }));
        }
        if save && result.is_ok() {
            let _ = refresh_route_data(&window).await;
        }
    });
    true
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_transfer_columns(document: &web_sys::Document) -> Result<(), JsValue> {
    const DEFAULTS: &[&str] = &[
        "name", "peer", "size", "progress", "bitrate", "length", "state", "actions",
    ];
    let Some(window) = document.default_view() else {
        return Ok(());
    };
    let storage = window.local_storage().ok().flatten();
    let saved = storage
        .as_ref()
        .and_then(|storage| {
            storage
                .get_item("slskr-transfer-columns-downloads")
                .ok()
                .flatten()
        })
        .map(|value| value.split(',').map(str::to_owned).collect::<BTreeSet<_>>());
    restore_transfer_column_widths(document);
    let toggles = document.query_selector_all("[data-slskr-transfer-column-toggle]")?;
    for index in 0..toggles.length() {
        let Some(node) = toggles.item(index) else {
            continue;
        };
        let input: web_sys::HtmlInputElement = node.dyn_into()?;
        let Some(key) = input.get_attribute("data-slskr-transfer-column-toggle") else {
            continue;
        };
        let visible = saved.as_ref().map_or_else(
            || DEFAULTS.contains(&key.as_str()),
            |saved| saved.contains(&key),
        );
        input.set_checked(visible);
        set_transfer_column_visibility(document, &key, visible);
        let document_for_click = document.clone();
        let input_for_click = input.clone();
        let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
            let Some(key) = input_for_click.get_attribute("data-slskr-transfer-column-toggle")
            else {
                return;
            };
            set_transfer_column_visibility(&document_for_click, &key, input_for_click.checked());
            persist_transfer_columns(&document_for_click);
        }));
        input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    let resize_buttons = document.query_selector_all("[data-slskr-transfer-column-resize]")?;
    for index in 0..resize_buttons.length() {
        let Some(node) = resize_buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        if button.has_attribute("data-slskr-mounted") {
            continue;
        }
        button.set_attribute("data-slskr-mounted", "true")?;
        let document_for_drag = document.clone();
        let button_for_drag = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let Some(key) = button_for_drag.get_attribute("data-slskr-transfer-column-resize")
                else {
                    return;
                };
                let Some(header) = button_for_drag.closest("th").ok().flatten() else {
                    return;
                };
                let Some(window) = document_for_drag.default_view() else {
                    return;
                };
                let start_width = header
                    .dyn_ref::<web_sys::HtmlElement>()
                    .map(|element| element.offset_width().max(80) as i32)
                    .unwrap_or(160);
                let start_x = event.client_x();
                let active = Rc::new(std::cell::Cell::new(true));
                let active_for_move = active.clone();
                let document_for_move = document_for_drag.clone();
                let key_for_move = key.clone();
                let move_callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                    move |move_event: web_sys::MouseEvent| {
                        if !active_for_move.get() {
                            return;
                        }
                        let width = (start_width + move_event.client_x() - start_x).clamp(80, 720);
                        set_transfer_column_width(&document_for_move, &key_for_move, width);
                    },
                ));
                let active_for_up = active.clone();
                let document_for_up = document_for_drag.clone();
                let up_callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                    move |_up_event: web_sys::MouseEvent| {
                        if !active_for_up.replace(false) {
                            return;
                        }
                        persist_transfer_column_widths(&document_for_up);
                    },
                ));
                let _ = window.add_event_listener_with_callback(
                    "mousemove",
                    move_callback.as_ref().unchecked_ref(),
                );
                let _ = window.add_event_listener_with_callback(
                    "mouseup",
                    up_callback.as_ref().unchecked_ref(),
                );
                move_callback.forget();
                up_callback.forget();
            },
        ));
        button.add_event_listener_with_callback("mousedown", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn set_transfer_column_width(document: &web_sys::Document, key: &str, width: i32) {
    let Ok(cells) =
        document.query_selector_all(&format!(r#"[data-slskr-transfer-column="{}"]"#, key))
    else {
        return;
    };
    for index in 0..cells.length() {
        let Some(node) = cells.item(index) else {
            continue;
        };
        if let Ok(element) = node.dyn_into::<web_sys::HtmlElement>() {
            let _ = element.style().set_property("width", &format!("{width}px"));
            let _ = element
                .style()
                .set_property("min-width", &format!("{width}px"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn restore_transfer_column_widths(document: &web_sys::Document) {
    let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    let Some(value) = storage
        .get_item("slskr-transfer-column-widths-downloads")
        .ok()
        .flatten()
    else {
        return;
    };
    for entry in value.split(',') {
        let Some((key, width)) = entry.split_once(':') else {
            continue;
        };
        if let Ok(width) = width.parse::<i32>() {
            set_transfer_column_width(document, key, width.clamp(80, 720));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn persist_transfer_column_widths(document: &web_sys::Document) {
    let Ok(headers) = document.query_selector_all("[data-slskr-transfer-column-resize]") else {
        return;
    };
    let widths = (0..headers.length())
        .filter_map(|index| {
            let node = headers.item(index)?;
            let button = node.dyn_into::<web_sys::Element>().ok()?;
            let key = button.get_attribute("data-slskr-transfer-column-resize")?;
            let header = button.closest("th").ok().flatten()?;
            let width = header
                .dyn_ref::<web_sys::HtmlElement>()?
                .offset_width()
                .max(80);
            Some(format!("{key}:{width}"))
        })
        .collect::<Vec<_>>()
        .join(",");
    if let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item("slskr-transfer-column-widths-downloads", &widths);
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn reset_transfer_column_widths(document: &web_sys::Document) {
    if let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.remove_item("slskr-transfer-column-widths-downloads");
    }
    let Ok(cells) = document.query_selector_all("[data-slskr-transfer-column]") else {
        return;
    };
    for index in 0..cells.length() {
        let Some(node) = cells.item(index) else {
            continue;
        };
        if let Ok(element) = node.dyn_into::<web_sys::HtmlElement>() {
            let _ = element.style().remove_property("width");
            let _ = element.style().remove_property("min-width");
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn set_transfer_column_visibility(
    document: &web_sys::Document,
    key: &str,
    visible: bool,
) {
    if !matches!(
        key,
        "name"
            | "peer"
            | "type"
            | "size"
            | "progress"
            | "bitrate"
            | "samplerate"
            | "bitdepth"
            | "length"
            | "state"
            | "folder"
            | "added"
            | "actions"
    ) {
        return;
    }
    let Ok(cells) =
        document.query_selector_all(&format!(r#"[data-slskr-transfer-column="{}"]"#, key))
    else {
        return;
    };
    for index in 0..cells.length() {
        let Some(node) = cells.item(index) else {
            continue;
        };
        let Ok(cell) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if visible {
            let _ = cell.remove_attribute("hidden");
        } else {
            let _ = cell.set_attribute("hidden", "");
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn persist_transfer_columns(document: &web_sys::Document) {
    let Ok(toggles) = document.query_selector_all("[data-slskr-transfer-column-toggle]") else {
        return;
    };
    let mut visible = Vec::new();
    for index in 0..toggles.length() {
        let Some(node) = toggles.item(index) else {
            continue;
        };
        let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
            continue;
        };
        if input.checked() {
            if let Some(key) = input.get_attribute("data-slskr-transfer-column-toggle") {
                visible.push(key);
            }
        }
    }
    if let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item("slskr-transfer-columns-downloads", &visible.join(","));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn handle_transfer_request_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> bool {
    if button.has_attribute("data-slskr-transfer-columns-reset") {
        if let Some(storage) = document
            .default_view()
            .and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.remove_item("slskr-transfer-columns-downloads");
        }
        reset_transfer_column_widths(document);
        show_toast(document, "Transfer column widths and visibility reset");
        return true;
    }
    let retry_id = button.get_attribute("data-slskr-transfer-retry");
    let rename = button.has_attribute("data-slskr-transfer-rename");
    let rename_save = button.has_attribute("data-slskr-transfer-rename-save");
    let attempts = button.has_attribute("data-slskr-transfer-attempts");
    let cancel = button.has_attribute("data-slskr-transfer-request-cancel");
    if retry_id.is_none() && !rename && !rename_save && !attempts && !cancel {
        return false;
    }
    let row = button
        .closest("[data-slskr-download-request-row]")
        .ok()
        .flatten();
    if rename {
        if let Some(editor) = row.as_ref().and_then(|row| {
            row.query_selector("[data-slskr-transfer-request-inline]")
                .ok()
                .flatten()
        }) {
            editor.set_class_name("slskr-transfer-request-inline is-open");
        }
        return true;
    }
    let request_id = row
        .as_ref()
        .and_then(|row| row.get_attribute("data-slskr-download-request-id"));
    let (method, path, body, label, load_attempts) = if let Some(id) = retry_id {
        if !safe_route_segment(&id) {
            show_toast(document, "Transfer attempt is invalid");
            return true;
        }
        (
            "POST",
            endpoint_url(&format!("/transfers/{id}/retry")),
            None,
            "Retry transfer",
            false,
        )
    } else {
        let Some(request_id) = request_id.filter(|id| safe_route_segment(id)) else {
            show_toast(document, "Download request is invalid");
            return true;
        };
        if rename_save {
            let name = row
                .as_ref()
                .and_then(|row| {
                    row.query_selector("[data-slskr-transfer-request-inline] input")
                        .ok()
                        .flatten()
                })
                .as_ref()
                .and_then(form_control_value)
                .unwrap_or_default();
            if name.trim().is_empty() || name.len() > 512 {
                show_toast(document, "Request name must be 1 to 512 bytes");
                return true;
            }
            (
                "PATCH",
                endpoint_url(&format!("/downloads/requests/{request_id}/name")),
                Some(format!(
                    r#"{{"name":"{}"}}"#,
                    escape_json_string(name.trim())
                )),
                "Save request name",
                false,
            )
        } else if attempts {
            (
                "GET",
                endpoint_url(&format!("/downloads/requests/{request_id}")),
                None,
                "Load attempts",
                true,
            )
        } else {
            (
                "POST",
                endpoint_url(&format!("/downloads/requests/{request_id}/cancel")),
                None,
                "Cancel request",
                false,
            )
        }
    };
    show_toast(document, &format!("{label} sending"));
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    let row = row.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(&window, &path, method, body.as_deref()).await;
        if load_attempts {
            if let Some(target) = row.as_ref().and_then(|row| {
                row.query_selector("[data-slskr-transfer-attempt-results]")
                    .ok()
                    .flatten()
            }) {
                target.set_inner_html(&match &result {
                    Ok(response) => transfer_attempts_response_html(response),
                    Err(error) => format!(
                        "<p>{}</p>",
                        escape_html(
                            &error
                                .as_string()
                                .unwrap_or_else(|| "attempt history failed".to_string())
                        )
                    ),
                });
            }
        }
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html(&match result {
                Ok(response) => format!(
                    "<strong>{}</strong> {}",
                    escape_html(label),
                    escape_html(&compact_preview(&response))
                ),
                Err(error) => format!(
                    "<strong>{}</strong> {}",
                    escape_html(label),
                    escape_html(
                        &error
                            .as_string()
                            .unwrap_or_else(|| "transfer request failed".to_string())
                    )
                ),
            });
        }
    });
    true
}
