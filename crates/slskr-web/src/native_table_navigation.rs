//! Native Web native table navigation.

use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn focus_relative_native_row(
    document: &web_sys::Document,
    current: &web_sys::Element,
    offset: isize,
) {
    let Some(workspace) = current.closest(".slskr-native-workspace").ok().flatten() else {
        return;
    };
    let rows = visible_native_rows(&workspace);
    let Some(current_index) = rows.iter().position(|row| row.is_same_node(Some(current))) else {
        return;
    };
    let target_index = (current_index as isize + offset).clamp(0, rows.len() as isize - 1) as usize;
    if let Some(row) = rows.get(target_index) {
        focus_native_row(document, row);
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn focus_edge_native_row(
    document: &web_sys::Document,
    current: &web_sys::Element,
    first: bool,
) {
    let Some(workspace) = current.closest(".slskr-native-workspace").ok().flatten() else {
        return;
    };
    let rows = visible_native_rows(&workspace);
    let row = if first { rows.first() } else { rows.last() };
    if let Some(row) = row {
        focus_native_row(document, row);
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn visible_native_rows(workspace: &web_sys::Element) -> Vec<web_sys::Element> {
    let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") else {
        return Vec::new();
    };
    let mut visible = Vec::new();
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
        visible.push(row);
    }
    visible
}

#[cfg(target_arch = "wasm32")]
pub(super) fn focus_native_row(document: &web_sys::Document, row: &web_sys::Element) {
    if let Some(element) = row.dyn_ref::<web_sys::HtmlElement>() {
        let _ = element.focus();
    }
    select_native_row(document, row);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_native_subviews(document: &web_sys::Document) -> Result<(), JsValue> {
    let tabs = document.query_selector_all("[data-slskr-native-tab]")?;
    for tab_index in 0..tabs.length() {
        let Some(node) = tabs.item(tab_index) else {
            continue;
        };
        let tab: web_sys::Element = node.dyn_into()?;
        let document_for_click = document.clone();
        let tab_for_click = tab.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                select_native_subview(&document_for_click, &tab_for_click);
            },
        ));
        tab.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn select_native_subview(document: &web_sys::Document, tab: &web_sys::Element) {
    let selected_index = tab
        .get_attribute("data-slskr-native-tab")
        .unwrap_or_else(|| "0".to_string());
    let workspace = tab
        .closest(".slskr-native-workspace")
        .ok()
        .flatten()
        .or_else(|| {
            document
                .query_selector(".slskr-native-workspace")
                .ok()
                .flatten()
        });
    let Some(workspace) = workspace else {
        return;
    };
    if let Ok(tabs) = workspace.query_selector_all("[data-slskr-native-tab]") {
        for index in 0..tabs.length() {
            let Some(node) = tabs.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let active = element
                .get_attribute("data-slskr-native-tab")
                .is_some_and(|value| value == selected_index);
            let _ = element.set_attribute("aria-selected", if active { "true" } else { "false" });
            element.set_class_name(if active {
                "slskr-native-tab is-active"
            } else {
                "slskr-native-tab"
            });
        }
    }
    if let Ok(panels) = workspace.query_selector_all("[data-slskr-native-panel]") {
        for index in 0..panels.length() {
            let Some(node) = panels.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let active = element
                .get_attribute("data-slskr-native-panel")
                .is_some_and(|value| value == selected_index);
            if active {
                let _ = element.remove_attribute("hidden");
            } else {
                let _ = element.set_attribute("hidden", "");
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_native_actions(document: &web_sys::Document) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-native-workspace button:not([data-slskr-native-tab]):not([data-slskr-native-filter-clear]):not([data-slskr-native-select-visible]):not([data-slskr-native-clear-selection]):not([data-slskr-native-reset-state]):not([data-slskr-transfer-column-resize]):not([data-slskr-pref-action]):not([data-slskr-automation-action]):not([data-slskr-recipe-dry-run]):not([data-slskr-recipe-copy])")?;
    for button_index in 0..buttons.length() {
        let Some(node) = buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let document_for_click = document.clone();
        let button_for_click = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                event.stop_propagation();
                handle_native_action(&document_for_click, &button_for_click);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn handle_native_action(document: &web_sys::Document, button: &web_sys::Element) {
    if handle_search_bulk_action(document, button) {
        return;
    }
    if handle_download_policy_action(document, button) {
        return;
    }
    if handle_options_yaml_action(document, button) {
        return;
    }
    if handle_transfer_request_action(document, button) {
        return;
    }
    if handle_private_message_auto_response_action(document, button) {
        return;
    }
    if handle_wishlist_policy_action(document, button) {
        return;
    }
    if handle_wishlist_ignored_result_action(document, button) {
        return;
    }
    let action = button
        .text_content()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Run action".to_string());
    let route_label = button
        .closest(".slskr-workflow")
        .ok()
        .flatten()
        .and_then(|workflow| workflow.get_attribute("data-slskr-route-kind"))
        .unwrap_or_else(|| "Workflow".to_string());
    let route_path = document
        .default_view()
        .and_then(|window| window.location().pathname().ok())
        .unwrap_or_else(|| "/searches".to_string());
    if route_kind(&route_path) == RouteKind::Browse
        && button.get_attribute("data-slskr-browse-folder").is_some()
    {
        if let Some(folder) = button.get_attribute("data-slskr-browse-folder") {
            if let Ok(Some(input)) = document.query_selector(r#"input[aria-label="Folder"]"#) {
                if let Some(input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                    input.set_value(&folder);
                }
            }
            if let Some(action) = route_action_for_native_label(&route_path, "Browse") {
                run_native_route_action(document, button, action);
            }
        }
        return;
    }
    if handle_native_local_action(document, button, &route_path, &action) {
        return;
    }
    if let Some(route_action) = route_action_for_native_label(&route_path, &action) {
        run_native_route_action(document, button, route_action);
        return;
    }
    set_reference_status(
        document,
        &action,
        &format!(
            "No executable UI contract is registered for the {} route.",
            route_label
        ),
    );
}

#[cfg(target_arch = "wasm32")]
pub(super) fn handle_search_bulk_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> bool {
    let Some(mode) = button.get_attribute("data-slskr-search-bulk") else {
        return false;
    };
    let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() else {
        show_toast(document, "Search selection is unavailable");
        return true;
    };
    let Ok(rows) = workspace.query_selector_all(
        "[data-slskr-native-select][aria-selected=\"true\"][data-slskr-native-search-id]",
    ) else {
        return true;
    };
    let mut selected = Vec::new();
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(id) = row
            .get_attribute("data-slskr-native-search-id")
            .filter(|value| safe_route_segment(value))
        else {
            continue;
        };
        let query = row
            .get_attribute("data-slskr-native-search-text")
            .unwrap_or_default()
            .trim()
            .to_owned();
        selected.push((id, query));
    }
    if selected.is_empty() {
        show_toast(document, "Select one or more saved searches first");
        return true;
    }
    if mode == "research" && selected.iter().all(|(_, query)| query.is_empty()) {
        show_toast(document, "Selected searches contain no searchable text");
        return true;
    }
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    let label = match mode.as_str() {
        "research" => "Search selected again",
        "stop" => "Stop selected",
        "remove" => "Delete selected",
        _ => "Search action",
    };
    show_toast(&document, &format!("{label} sending"));
    let total = selected.len();
    wasm_bindgen_futures::spawn_local(async move {
        let mut completed = 0usize;
        for (id, query) in selected {
            let (method, path, body) = match mode.as_str() {
                "research" if !query.is_empty() => (
                    "POST",
                    endpoint_url("/searches"),
                    Some(format!(
                        r#"{{"searchText":"{}"}}"#,
                        escape_json_string(&query)
                    )),
                ),
                "stop" => ("PUT", endpoint_url(&format!("/searches/{id}")), None),
                "remove" => ("DELETE", endpoint_url(&format!("/searches/{id}")), None),
                _ => continue,
            };
            if fetch_text_with_method(&window, &path, method, body.as_deref())
                .await
                .is_ok()
            {
                completed += 1;
            }
        }
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html(&format!(
                "<strong>{}</strong> completed {} of {}",
                escape_html(label),
                completed,
                total,
            ));
        }
        let _ = refresh_route_data(&window).await;
    });
    true
}
