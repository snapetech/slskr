use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_native_filters(document: &web_sys::Document) -> Result<(), JsValue> {
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
pub(super) fn native_state_key(workspace: &web_sys::Element, suffix: &str) -> Option<String> {
    let route = workspace
        .closest("[data-route]")
        .ok()
        .flatten()
        .and_then(|route| route.get_attribute("data-route"))?;
    Some(format!("slskr.native.{route}.{suffix}"))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn session_storage_for_workspace(
    workspace: &web_sys::Element,
) -> Option<web_sys::Storage> {
    workspace
        .owner_document()
        .and_then(|document| document.default_view())
        .and_then(|window| window.session_storage().ok().flatten())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn restore_native_filter(
    document: &web_sys::Document,
    workspace: &web_sys::Element,
) -> String {
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
pub(super) fn persist_native_filter(workspace: &web_sys::Element, term: &str) {
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
pub(super) fn reset_native_table_state(workspace: &web_sys::Element) {
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
pub(super) fn apply_native_filter(workspace: &web_sys::Element, term: &str) {
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
pub(super) fn update_native_filter_count(workspace: &web_sys::Element) {
    let total = workspace
        .query_selector_all("[data-slskr-native-select]")
        .map(|rows| rows.length())
        .unwrap_or_default();
    set_native_filter_count(workspace, total, total);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn set_native_filter_count(workspace: &web_sys::Element, visible: u32, total: u32) {
    if let Ok(Some(count)) = workspace.query_selector("[data-slskr-native-count]") {
        count.set_text_content(Some(&format!("{visible} / {total} rows")));
    }
}
