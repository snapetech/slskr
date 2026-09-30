use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn native_action_row(
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
pub(super) fn native_selected_row_attribute(
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
pub(super) fn native_selected_context(
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
pub(super) fn native_json_string_by_key(
    value: &serde_json::Value,
    keys: &[&str],
) -> Option<String> {
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
pub(super) fn native_copy_context(
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
pub(super) fn selected_native_row_detail(workspace: &web_sys::Element) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-detail"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn selected_native_row_title(workspace: &web_sys::Element) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn selected_native_row_titles(workspace: &web_sys::Element) -> Option<String> {
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
pub(super) fn selected_native_row_attribute(
    workspace: &web_sys::Element,
    attribute: &str,
) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute(attribute))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn selected_native_row_attribute_values(
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
pub(super) fn selected_native_row_target(workspace: &web_sys::Element) -> Option<String> {
    selected_native_row_detail(workspace)
        .filter(|value| safe_route_segment(value))
        .or_else(|| selected_native_row_title(workspace).filter(|value| safe_route_segment(value)))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn button_native_row_target(button: &web_sys::Element) -> Option<String> {
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
pub(super) fn button_native_row_attribute(
    button: &web_sys::Element,
    attribute: &str,
) -> Option<String> {
    button
        .closest("[data-slskr-native-select]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute(attribute))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn document_selected_native_row_title(document: &web_sys::Document) -> Option<String> {
    document
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .filter(|value| !value.trim().is_empty())
}
