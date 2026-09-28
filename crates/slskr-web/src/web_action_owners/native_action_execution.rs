use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn native_set_action_status(document: &web_sys::Document, label: &str, message: &str) {
    set_reference_status(document, label, message);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_fold_duplicate_rows(document: &web_sys::Document, button: &web_sys::Element) {
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
pub(super) fn native_bulk_transfer_action(
    document: &web_sys::Document,
    action: &str,
    selector: &str,
) {
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
pub(super) fn native_open_search(document: &web_sys::Document, button: &web_sys::Element) -> bool {
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
pub(super) fn handle_native_local_action(
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
pub(super) fn run_native_route_action(
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
pub(super) fn native_action_requires_confirmation(action: RouteAction) -> bool {
    action.method == "DELETE"
        || matches!(
            action.label,
            "Cancel Download" | "Deny Upload" | "Shut Down" | "Restart" | "Vacuum Database"
        )
}

#[cfg(target_arch = "wasm32")]
pub(super) fn show_native_confirm_modal(
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
pub(super) fn execute_native_route_action(
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
pub(super) fn download_body_with_selected_destination(
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
pub(super) fn show_toast(document: &web_sys::Document, message: &str) {
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
pub(super) fn mount_live_controls(
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
pub(super) fn set_live_status(document: &web_sys::Document, message: &str) {
    if let Some(status) = document.get_element_by_id("slskr-live-status") {
        status.set_text_content(Some(message));
    }
}
