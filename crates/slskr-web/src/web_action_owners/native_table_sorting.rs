use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_native_sorters(document: &web_sys::Document) -> Result<(), JsValue> {
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
pub(super) fn sort_native_table(button: &web_sys::Element) {
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
pub(super) fn apply_native_sort(
    button: &web_sys::Element,
    index: &str,
    direction: &str,
    persist: bool,
) {
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
pub(super) fn restore_native_sort(document: &web_sys::Document) {
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
pub(super) fn persist_native_sort(workspace: &web_sys::Element, index: &str, direction: &str) {
    let Some(key) = native_state_key(workspace, "sort") else {
        return;
    };
    let Some(storage) = session_storage_for_workspace(workspace) else {
        return;
    };
    let _ = storage.set_item(&key, &format!("{index}:{direction}"));
}

#[cfg(target_arch = "wasm32")]
pub(super) fn reset_native_sort(workspace: &web_sys::Element) {
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
