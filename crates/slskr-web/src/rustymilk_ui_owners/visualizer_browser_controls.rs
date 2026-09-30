use super::*;
#[cfg(target_arch = "wasm32")]
pub(super) fn mount_rustymilk_buttons(
    window: &web_sys::Window,
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: Rc<RefCell<BTreeSet<String>>>,
    texture_assets: Rc<RefCell<BTreeMap<String, String>>>,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-rustymilk-action]")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        if button.has_attribute("data-slskr-mounted") {
            continue;
        }
        button.set_attribute("data-slskr-mounted", "true")?;
        let action = button
            .get_attribute("data-slskr-rustymilk-action")
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let imported_presets = imported_presets.clone();
        let favorite_presets = favorite_presets.clone();
        let texture_assets = texture_assets.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                match action.as_str() {
                    "close" => {
                        if let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") {
                            let _ = panel.set_attribute("hidden", "");
                        }
                        set_player_status(&document, "RustyMilk hidden");
                    }
                    "external" => {
                        let window = window.clone();
                        let document = document.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let result = fetch_text_with_method(
                                &window,
                                &endpoint_url("/player/external-visualizer/launch"),
                                "POST",
                                None,
                            )
                            .await;
                            match result {
                                Ok(body) => set_player_status(&document, &compact_preview(&body)),
                                Err(error) => set_player_status(
                                    &document,
                                    &error.as_string().unwrap_or_else(|| {
                                        "external visualizer request failed".to_string()
                                    }),
                                ),
                            }
                        });
                    }
                    "import" => {
                        if let Some(input) = document
                            .get_element_by_id("slskr-rustymilk-preset-input")
                            .and_then(|element| {
                                element.dyn_into::<web_sys::HtmlInputElement>().ok()
                            })
                        {
                            input.click();
                        } else {
                            import_rustymilk_preset(&window, &document, imported_presets.clone());
                        }
                    }
                    "clear" => clear_rustymilk_library(
                        &window,
                        &document,
                        imported_presets.clone(),
                        favorite_presets.clone(),
                    ),
                    "clear-search" => clear_rustymilk_preset_search(
                        &document,
                        &imported_presets,
                        &favorite_presets,
                    ),
                    "save-playlist" => save_rustymilk_playlist(
                        &window,
                        &document,
                        &imported_presets,
                        &favorite_presets,
                    ),
                    "playlist" => {
                        cycle_rustymilk_playlist(&document, &imported_presets, &favorite_presets)
                    }
                    "rename-playlist" => rename_rustymilk_playlist(&window, &document),
                    "clear-playlist" => clear_rustymilk_playlist_filter(
                        &document,
                        &imported_presets,
                        &favorite_presets,
                    ),
                    "remove-playlist" => remove_rustymilk_playlist(
                        &window,
                        &document,
                        &imported_presets,
                        &favorite_presets,
                    ),
                    "favorite" => toggle_rustymilk_favorite(
                        &document,
                        &imported_presets,
                        favorite_presets.clone(),
                    ),
                    "favorites" => toggle_rustymilk_favorites_only(
                        &document,
                        &imported_presets,
                        &favorite_presets,
                    ),
                    "previous" => cycle_rustymilk_preset_by(
                        &document,
                        &imported_presets,
                        &favorite_presets,
                        -1,
                    ),
                    "random" => {
                        random_rustymilk_preset(&document, &imported_presets, &favorite_presets)
                    }
                    "remove" => remove_rustymilk_active_preset(
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                    ),
                    "reset" => reset_rustymilk_import(
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                    ),
                    "search" => {
                        search_rustymilk_preset(&document, &imported_presets, &favorite_presets)
                    }
                    "apply-parameter" => apply_rustymilk_parameter(
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                    ),
                    "randomize-parameters" => randomize_rustymilk_parameters(
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                    ),
                    "import-shape" => import_rustymilk_fragment(
                        &window,
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                        "shape",
                    ),
                    "import-wave" => import_rustymilk_fragment(
                        &window,
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                        "wave",
                    ),
                    "export-shape" => {
                        export_rustymilk_fragment(&document, &imported_presets, "shape")
                    }
                    "export-wave" => {
                        export_rustymilk_fragment(&document, &imported_presets, "wave")
                    }
                    "remove-shape" => remove_rustymilk_fragment(
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                        "shape",
                    ),
                    "remove-wave" => remove_rustymilk_fragment(
                        &document,
                        imported_presets.clone(),
                        &favorite_presets,
                        "wave",
                    ),
                    "export-preset" => export_rustymilk_preset(&document, &imported_presets),
                    "automation" => cycle_rustymilk_automation(&document),
                    "debug" => toggle_rustymilk_debug(&document, &imported_presets),
                    "texture" => {
                        if let Some(input) = document
                            .get_element_by_id("slskr-rustymilk-texture-input")
                            .and_then(|element| {
                                element.dyn_into::<web_sys::HtmlInputElement>().ok()
                            })
                        {
                            input.click();
                        } else {
                            import_rustymilk_texture_asset(
                                &window,
                                &document,
                                texture_assets.clone(),
                            );
                        }
                    }
                    "pack" => {
                        if let Some(input) = document
                            .get_element_by_id("slskr-rustymilk-pack-input")
                            .and_then(|element| {
                                element.dyn_into::<web_sys::HtmlInputElement>().ok()
                            })
                        {
                            input.click();
                        }
                    }
                    _ => cycle_rustymilk_preset(&document, &imported_presets, &favorite_presets),
                }
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_rustymilk_selects(document: &web_sys::Document) -> Result<(), JsValue> {
    for (id, attribute, storage_key) in [
        (
            "slskr-rustymilk-fps",
            "data-slskr-rustymilk-fps",
            RUSTYMILK_FPS_STORAGE_KEY,
        ),
        (
            "slskr-rustymilk-quality",
            "data-slskr-rustymilk-quality",
            RUSTYMILK_QUALITY_STORAGE_KEY,
        ),
    ] {
        let Some(select) = document
            .get_element_by_id(id)
            .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
        else {
            continue;
        };
        if select.has_attribute("data-slskr-mounted") {
            continue;
        }
        if let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") {
            if let Some(value) = panel.get_attribute(attribute) {
                select.set_value(&value);
            }
        }
        select.set_attribute("data-slskr-mounted", "true")?;
        let document_for_change = document.clone();
        let attribute = attribute.to_string();
        let storage_key = storage_key.to_string();
        let callback =
            Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
                let Some(select) = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlSelectElement>().ok())
                else {
                    return;
                };
                if let Some(panel) = document_for_change.get_element_by_id("slskr-rust-rustymilk") {
                    let _ = panel.set_attribute(&attribute, &select.value());
                }
                persist_rustymilk_simple_setting(
                    &document_for_change,
                    &storage_key,
                    &select.value(),
                );
                set_player_status(&document_for_change, "RustyMilk render setting updated");
            }));
        select.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn rustymilk_mouse_position(
    canvas: &web_sys::HtmlCanvasElement,
    event: &web_sys::MouseEvent,
) -> (f64, f64) {
    let width = canvas.width().max(1) as f64;
    let height = canvas.height().max(1) as f64;
    (
        (event.offset_x() as f64 / width).clamp(0.0, 1.0),
        (event.offset_y() as f64 / height).clamp(0.0, 1.0),
    )
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_rustymilk_mouse_input(
    canvas: &web_sys::HtmlCanvasElement,
    input_state: Rc<RefCell<RustyMilkInputState>>,
) -> Result<(), JsValue> {
    let canvas_for_move = canvas.clone();
    let input_for_move = input_state.clone();
    let move_callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
        move |event: web_sys::MouseEvent| {
            let (mouse_x, mouse_y) = rustymilk_mouse_position(&canvas_for_move, &event);
            let mut input = input_for_move.borrow_mut();
            input.mouse_dx = mouse_x - input.mouse_x;
            input.mouse_dy = mouse_y - input.mouse_y;
            input.mouse_x = mouse_x;
            input.mouse_y = mouse_y;
        },
    ));
    canvas.add_event_listener_with_callback("mousemove", move_callback.as_ref().unchecked_ref())?;
    move_callback.forget();

    let canvas_for_down = canvas.clone();
    let input_for_down = input_state.clone();
    let down_callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
        move |event: web_sys::MouseEvent| {
            let (mouse_x, mouse_y) = rustymilk_mouse_position(&canvas_for_down, &event);
            let mut input = input_for_down.borrow_mut();
            input.mouse_down = 1.0;
            input.mouse_dx = mouse_x - input.mouse_x;
            input.mouse_dy = mouse_y - input.mouse_y;
            input.mouse_x = mouse_x;
            input.mouse_y = mouse_y;
        },
    ));
    canvas.add_event_listener_with_callback("mousedown", down_callback.as_ref().unchecked_ref())?;
    down_callback.forget();

    let input_for_up = input_state.clone();
    let up_callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
        move |_event: web_sys::MouseEvent| {
            input_for_up.borrow_mut().mouse_down = 0.0;
        },
    ));
    canvas.add_event_listener_with_callback("mouseup", up_callback.as_ref().unchecked_ref())?;
    canvas.add_event_listener_with_callback("mouseleave", up_callback.as_ref().unchecked_ref())?;
    up_callback.forget();
    Ok(())
}
