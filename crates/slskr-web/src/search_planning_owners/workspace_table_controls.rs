use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn update_data_card_count(card: &web_sys::Element) {
    let Ok(Some(count)) = card.query_selector("[data-slskr-card-count]") else {
        return;
    };
    let Ok(rows) = card.query_selector_all(".slskr-record-list [data-slskr-row-text]") else {
        return;
    };
    let total = rows.length();
    let mut visible = 0;
    for row_index in 0..rows.length() {
        let Some(row) = rows.item(row_index) else {
            continue;
        };
        let Ok(row) = row.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if !row.has_attribute("hidden") {
            visible += 1;
        }
    }
    count.set_text_content(Some(&format!("{visible} / {total}")));
}

#[cfg(target_arch = "wasm32")]
pub(super) fn sort_data_card_table(card: &web_sys::Element, button: &web_sys::Element) {
    let column = button
        .get_attribute("data-slskr-sort-index")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_default();
    let next_direction = match button.get_attribute("data-slskr-sort-direction").as_deref() {
        Some("asc") => "desc",
        _ => "asc",
    };
    let Ok(Some(tbody)) = card.query_selector(".slskr-data-table tbody") else {
        return;
    };
    let Ok(rows) = tbody.query_selector_all("tr") else {
        return;
    };
    let mut elements = Vec::new();
    for row_index in 0..rows.length() {
        let Some(row) = rows.item(row_index) else {
            continue;
        };
        let Ok(row) = row.dyn_into::<web_sys::Element>() else {
            continue;
        };
        elements.push(row);
    }
    elements.sort_by(|left, right| {
        let left_value = table_cell_text(left, column);
        let right_value = table_cell_text(right, column);
        if next_direction == "asc" {
            left_value.cmp(&right_value)
        } else {
            right_value.cmp(&left_value)
        }
    });
    if let Ok(buttons) = card.query_selector_all("[data-slskr-sort-index]") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("data-slskr-sort-direction");
            let _ = element.remove_attribute("aria-sort");
        }
    }
    let _ = button.set_attribute("data-slskr-sort-direction", next_direction);
    let _ = button.set_attribute(
        "aria-sort",
        if next_direction == "asc" {
            "ascending"
        } else {
            "descending"
        },
    );
    for row in elements {
        let _ = tbody.append_child(&row);
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn table_cell_text(row: &web_sys::Element, column: u32) -> String {
    let selector = format!("td:nth-child({})", column + 1);
    row.query_selector(&selector)
        .ok()
        .flatten()
        .and_then(|cell| cell.text_content())
        .unwrap_or_default()
        .to_lowercase()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn select_data_card_record(card: &web_sys::Element, row: &web_sys::Element) {
    if let Ok(rows) = card.query_selector_all("[data-slskr-record-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("aria-selected");
            let _ = element.set_attribute("class", "");
        }
    }
    let _ = row.set_attribute("aria-selected", "true");
    let _ = row.set_attribute("class", "is-selected");

    let title = row
        .get_attribute("data-slskr-record-title")
        .unwrap_or_else(|| "Selected Record".to_string());
    let detail = row
        .get_attribute("data-slskr-record-detail")
        .unwrap_or_default();
    let raw = row
        .get_attribute("data-slskr-record-json")
        .unwrap_or_default();

    if let Ok(Some(header)) = card.query_selector(".slskr-card-inspector h4") {
        header.set_text_content(Some(&title));
    }
    if let Ok(Some(description)) = card.query_selector(".slskr-card-inspector p") {
        description.set_text_content(Some(&detail));
    }
    if let Ok(Some(pre)) = card.query_selector(".slskr-card-inspector pre") {
        pre.set_text_content(Some(&raw));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_workspace_tabs(document: &web_sys::Document) -> Result<(), JsValue> {
    let tabs = document.query_selector_all(".slskr-workspace-tab")?;
    for index in 0..tabs.length() {
        let Some(node) = tabs.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let Some(target) = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };
                let mode = target
                    .get_attribute("data-slskr-workspace-mode")
                    .unwrap_or_else(|| "all".to_string());

                if let Ok(tabs) = document.query_selector_all(".slskr-workspace-tab") {
                    for index in 0..tabs.length() {
                        let Some(node) = tabs.item(index) else {
                            continue;
                        };
                        let Ok(tab) = node.dyn_into::<web_sys::Element>() else {
                            continue;
                        };
                        let active = tab
                            .get_attribute("data-slskr-workspace-mode")
                            .is_some_and(|tab_mode| tab_mode == mode);
                        let class = if active {
                            "slskr-workspace-tab is-active"
                        } else {
                            "slskr-workspace-tab"
                        };
                        let _ = tab.set_attribute("class", class);
                        let _ = tab
                            .set_attribute("aria-selected", if active { "true" } else { "false" });
                    }
                }

                if let Ok(Some(grid)) = document.query_selector("[data-slskr-workspace-grid]") {
                    let class = match mode.as_str() {
                        "primary" => "slskr-workspace-grid mode-primary",
                        "secondary" => "slskr-workspace-grid mode-secondary",
                        _ => "slskr-workspace-grid",
                    };
                    let _ = grid.set_attribute("class", class);
                }
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}
