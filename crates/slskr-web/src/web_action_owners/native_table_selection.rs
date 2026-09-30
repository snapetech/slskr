use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn select_visible_native_rows(workspace: &web_sys::Element) {
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
pub(super) fn clear_native_selection(workspace: &web_sys::Element) {
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
pub(super) fn update_native_selection_summary(
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
pub(super) fn update_native_inspector(
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
pub(super) fn update_native_selection_preview(
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
pub(super) fn select_native_row(document: &web_sys::Document, row: &web_sys::Element) {
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
