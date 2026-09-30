use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn storage_json_object(
    window: &web_sys::Window,
    key: &str,
    fallback: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(key).ok().flatten())
        .and_then(|body| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body).ok()
        })
        .map(|stored| {
            let mut merged = fallback.clone();
            for (key, value) in stored {
                merged.insert(key, value);
            }
            merged
        })
        .unwrap_or(fallback)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn write_storage_json_object(
    window: &web_sys::Window,
    key: &str,
    value: &serde_json::Map<String, serde_json::Value>,
) {
    if let Some(storage) = window.local_storage().ok().flatten() {
        let _ = storage.set_item(key, &serde_json::Value::Object(value.clone()).to_string());
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn remove_storage_item(window: &web_sys::Window, key: &str) {
    if let Some(storage) = window.local_storage().ok().flatten() {
        let _ = storage.remove_item(key);
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn collect_experience_form(
    document: &web_sys::Document,
) -> serde_json::Map<String, serde_json::Value> {
    let mut values = serde_json::Map::new();
    if let Ok(inputs) = document.query_selector_all("[data-slskr-pref]") {
        for index in 0..inputs.length() {
            let Some(node) = inputs.item(index) else {
                continue;
            };
            let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            let Some(key) = input.get_attribute("data-slskr-pref") else {
                continue;
            };
            let value = if input.type_() == "checkbox" {
                serde_json::Value::Bool(input.checked())
            } else {
                serde_json::Value::String(input.value())
            };
            values.insert(key, value);
        }
    }
    values
}

#[cfg(target_arch = "wasm32")]
pub(super) fn apply_experience_form(
    document: &web_sys::Document,
    values: &serde_json::Map<String, serde_json::Value>,
) {
    if let Ok(inputs) = document.query_selector_all("[data-slskr-pref]") {
        for index in 0..inputs.length() {
            let Some(node) = inputs.item(index) else {
                continue;
            };
            let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            let Some(key) = input.get_attribute("data-slskr-pref") else {
                continue;
            };
            let value = values.get(&key).cloned().unwrap_or_else(|| {
                serde_json::Value::String(
                    input
                        .get_attribute("data-slskr-pref-default")
                        .unwrap_or_default(),
                )
            });
            if input.type_() == "checkbox" {
                input.set_checked(value.as_bool().unwrap_or(false));
            } else {
                input.set_value(&json_scalar_preview(&value));
            }
        }
    }
    let report = experience_preferences_report(values);
    if let Some(output) = document.get_element_by_id("slskr-experience-report") {
        output.set_text_content(Some(&report));
    }
    if let Some(summary) = document.get_element_by_id("slskr-experience-summary") {
        summary.set_text_content(Some("18 preferences"));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn default_automation_state() -> serde_json::Map<String, serde_json::Value> {
    automation_recipes()
        .iter()
        .map(|recipe| {
            (
                recipe.id.to_string(),
                serde_json::json!({
                    "enabled": recipe.enabled_by_default,
                    "lastDryRunAt": null,
                }),
            )
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn apply_automation_state(
    document: &web_sys::Document,
    state: &serde_json::Map<String, serde_json::Value>,
) {
    for recipe in automation_recipes() {
        let enabled = state
            .get(recipe.id)
            .and_then(|entry| entry.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(recipe.enabled_by_default);
        let selector = format!(r#"[data-slskr-recipe-enabled="{}"]"#, recipe.id);
        if let Ok(Some(input)) = document.query_selector(&selector) {
            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                input.set_checked(enabled);
            }
        }
    }
    let (total, enabled, disabled) = automation_summary_from_state(state);
    if let Some(summary) = document.get_element_by_id("slskr-automation-summary") {
        summary.set_text_content(Some(&format!(
            "{total} recipes / {enabled} enabled / {disabled} disabled"
        )));
    }
    if let Some(report) = document.get_element_by_id("slskr-automation-report") {
        report.set_text_content(Some(&automation_history_report(state)));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_browser_local_panels(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    if document
        .query_selector("[data-slskr-experience-panel]")?
        .is_some()
    {
        let values = storage_json_object(
            window,
            "slskr:experience-preferences:v1",
            default_experience_preferences(),
        );
        apply_experience_form(document, &values);
        let buttons = document.query_selector_all("[data-slskr-pref-action]")?;
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-pref-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let values = if action == "reset" {
                        remove_storage_item(&window, "slskr:experience-preferences:v1");
                        default_experience_preferences()
                    } else {
                        collect_experience_form(&document)
                    };
                    if action == "save" {
                        write_storage_json_object(
                            &window,
                            "slskr:experience-preferences:v1",
                            &values,
                        );
                    }
                    apply_experience_form(&document, &values);
                    if action == "copy" {
                        let report = experience_preferences_report(&values);
                        copy_reference_text(&window, &document, report);
                    }
                    if let Some(status) = document.get_element_by_id("slskr-experience-status") {
                        let message = match action.as_str() {
                            "copy" => "Experience preference report copied.",
                            "reset" => "Experience preferences reset.",
                            _ => "Experience preferences saved locally.",
                        };
                        status.set_text_content(Some(message));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    if document
        .query_selector("[data-slskr-automation-panel]")?
        .is_some()
    {
        let state = storage_json_object(
            window,
            "slskr.automationRecipeState",
            default_automation_state(),
        );
        apply_automation_state(document, &state);
        let enabled_inputs = document.query_selector_all("[data-slskr-recipe-enabled]")?;
        for index in 0..enabled_inputs.length() {
            let Some(node) = enabled_inputs.item(index) else {
                continue;
            };
            let input: web_sys::HtmlInputElement = node.dyn_into()?;
            let recipe_id = input
                .get_attribute("data-slskr-recipe-enabled")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(
                move |event: web_sys::Event| {
                    let checked = event
                        .current_target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.checked())
                        .unwrap_or(false);
                    let mut state = storage_json_object(
                        &window,
                        "slskr.automationRecipeState",
                        default_automation_state(),
                    );
                    let entry = state
                        .entry(recipe_id.clone())
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(object) = entry.as_object_mut() {
                        object.insert("enabled".to_string(), serde_json::Value::Bool(checked));
                    }
                    write_storage_json_object(&window, "slskr.automationRecipeState", &state);
                    apply_automation_state(&document, &state);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some("Automation recipe state saved."));
                    }
                },
            ));
            input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let dry_run_buttons = document.query_selector_all("[data-slskr-recipe-dry-run]")?;
        for index in 0..dry_run_buttons.length() {
            let Some(node) = dry_run_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let recipe_id = button
                .get_attribute("data-slskr-recipe-dry-run")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let Some(recipe) = automation_recipes()
                        .iter()
                        .find(|recipe| recipe.id == recipe_id)
                        .copied()
                    else {
                        return;
                    };
                    let report = automation_dry_run_report(recipe, "browser-local");
                    let mut state = storage_json_object(
                        &window,
                        "slskr.automationRecipeState",
                        default_automation_state(),
                    );
                    let entry = state
                        .entry(recipe.id.to_string())
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(object) = entry.as_object_mut() {
                        object.insert(
                            "lastDryRunAt".to_string(),
                            serde_json::Value::String("browser-local".to_string()),
                        );
                        object.insert("lastDryRunReport".to_string(), report.clone());
                    }
                    write_storage_json_object(&window, "slskr.automationRecipeState", &state);
                    apply_automation_state(&document, &state);
                    if let Some(output) = document.get_element_by_id("slskr-automation-report") {
                        output.set_text_content(Some(
                            &serde_json::to_string_pretty(&report).unwrap_or_default(),
                        ));
                    }
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status
                            .set_text_content(Some(&format!("{} dry run recorded.", recipe.title)));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let copy_plan_buttons = document.query_selector_all("[data-slskr-recipe-copy]")?;
        for index in 0..copy_plan_buttons.length() {
            let Some(node) = copy_plan_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let recipe_id = button
                .get_attribute("data-slskr-recipe-copy")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let Some(recipe) = automation_recipes()
                        .iter()
                        .find(|recipe| recipe.id == recipe_id)
                        .copied()
                    else {
                        return;
                    };
                    let report = automation_dry_run_report(recipe, "browser-local");
                    let text = serde_json::to_string_pretty(&report).unwrap_or_default();
                    copy_reference_text(&window, &document, text);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some(&format!("{} plan copied.", recipe.title)));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let action_buttons = document.query_selector_all("[data-slskr-automation-action]")?;
        for index in 0..action_buttons.length() {
            let Some(node) = action_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-automation-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let state = if action == "reset" {
                        remove_storage_item(&window, "slskr.automationRecipeState");
                        default_automation_state()
                    } else {
                        storage_json_object(
                            &window,
                            "slskr.automationRecipeState",
                            default_automation_state(),
                        )
                    };
                    apply_automation_state(&document, &state);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some(if action == "reset" {
                            "Automation recipe state reset."
                        } else {
                            "Automation history report prepared."
                        }));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    if document
        .query_selector("[data-slskr-search-planner]")?
        .is_some()
    {
        let render_search_plan =
            |document: &web_sys::Document, window: &web_sys::Window, message: &str| {
                let query = document
                    .query_selector(r#"[data-slskr-search-setting="query"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_else(|| "public domain theme".to_string());
                let profile = document
                    .query_selector(r#"[data-slskr-search-setting="profile"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_else(|| "lossless-exact".to_string());
                let fold = document
                    .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.checked())
                    .unwrap_or(true);
                if let Some(output) = document.get_element_by_id("slskr-search-planner-report") {
                    output.set_text_content(Some(&search_planner_report(&query, &profile, fold)));
                }
                if let Some(status) = document.get_element_by_id("slskr-search-planner-status") {
                    status.set_text_content(Some(message));
                }
                let mut stored = serde_json::Map::new();
                stored.insert("query".to_string(), serde_json::Value::String(query));
                stored.insert("profile".to_string(), serde_json::Value::String(profile));
                stored.insert("foldDuplicates".to_string(), serde_json::Value::Bool(fold));
                write_storage_json_object(window, "slskr.search.planner", &stored);
            };

        render_search_plan(document, window, "Search planner ready.");
        let buttons = document.query_selector_all("[data-slskr-search-action]")?;
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-search-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    if action == "reset" {
                        if let Ok(Some(input)) =
                            document.query_selector(r#"[data-slskr-search-setting="query"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_value("public domain theme");
                            }
                        }
                        if let Ok(Some(input)) =
                            document.query_selector(r#"[data-slskr-search-setting="profile"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_value("lossless-exact");
                            }
                        }
                        if let Ok(Some(input)) = document
                            .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_checked(true);
                            }
                        }
                        remove_storage_item(&window, "slskr.search.planner");
                    }
                    let query = document
                        .query_selector(r#"[data-slskr-search-setting="query"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_default();
                    let profile = document
                        .query_selector(r#"[data-slskr-search-setting="profile"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_else(|| "lossless-exact".to_string());
                    let fold = document
                        .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.checked())
                        .unwrap_or(true);
                    if let Some(output) = document.get_element_by_id("slskr-search-planner-report")
                    {
                        output
                            .set_text_content(Some(&search_planner_report(&query, &profile, fold)));
                    }
                    if let Some(status) = document.get_element_by_id("slskr-search-planner-status")
                    {
                        status.set_text_content(Some(if action == "reset" {
                            "Search planner reset."
                        } else {
                            "Search action preview prepared."
                        }));
                    }
                    let mut stored = serde_json::Map::new();
                    stored.insert("query".to_string(), serde_json::Value::String(query));
                    stored.insert("profile".to_string(), serde_json::Value::String(profile));
                    stored.insert("foldDuplicates".to_string(), serde_json::Value::Bool(fold));
                    write_storage_json_object(&window, "slskr.search.planner", &stored);
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    Ok(())
}
