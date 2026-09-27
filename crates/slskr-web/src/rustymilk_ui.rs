#[cfg(target_arch = "wasm32")]
fn mount_rustymilk_buttons(
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
fn mount_rustymilk_selects(document: &web_sys::Document) -> Result<(), JsValue> {
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
fn rustymilk_mouse_position(
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
fn mount_rustymilk_mouse_input(
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

#[cfg(target_arch = "wasm32")]
fn rustymilk_preset_count(
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
) -> usize {
    imported_presets.borrow().len()
        + RUSTYMILK_PRESETS.len()
        + usize::from(
            panel
                .get_attribute("data-slskr-rustymilk-custom-source")
                .is_some_and(|value| !value.trim().is_empty()),
        )
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_active_preset_source(
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    index: usize,
) -> String {
    let imports = imported_presets.borrow();
    if let Some(preset) = imports.get(index) {
        return preset.source.clone();
    }
    let index = index.saturating_sub(imports.len());
    drop(imports);
    if let Some(custom_source) = panel
        .get_attribute("data-slskr-rustymilk-custom-source")
        .filter(|value| !value.trim().is_empty())
    {
        if index == 0 {
            return custom_source;
        }
        return RUSTYMILK_PRESETS[(index - 1) % RUSTYMILK_PRESETS.len()].to_string();
    }
    RUSTYMILK_PRESETS[index % RUSTYMILK_PRESETS.len()].to_string()
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_active_preset_name(
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    index: usize,
) -> String {
    let imports = imported_presets.borrow();
    if let Some(preset) = imports.get(index) {
        return preset.title.clone();
    }
    drop(imports);
    rustymilk_preset_name(&rustymilk_active_preset_source(
        panel,
        imported_presets,
        index,
    ))
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_preset_favorite_key(
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    index: usize,
) -> String {
    let source = rustymilk_active_preset_source(panel, imported_presets, index);
    let name = rustymilk_active_preset_name(panel, imported_presets, index);
    let preview = source
        .lines()
        .take(16)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(4096)
        .collect::<String>();
    format!("{name}\n{preview}")
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_favorites_only(panel: &web_sys::Element) -> bool {
    panel
        .get_attribute("data-slskr-rustymilk-favorites-only")
        .as_deref()
        == Some("true")
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_active_playlist(panel: &web_sys::Element) -> String {
    panel
        .get_attribute("data-slskr-rustymilk-playlist")
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_preset_search_query(panel: &web_sys::Element) -> String {
    panel
        .get_attribute("data-slskr-rustymilk-search")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_visible_preset_indices(
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) -> Vec<usize> {
    let count = rustymilk_preset_count(panel, imported_presets);
    let query = rustymilk_preset_search_query(panel);
    let active_playlist = rustymilk_active_playlist(panel);
    let playlist_keys = if active_playlist.is_empty() {
        BTreeSet::new()
    } else {
        load_rustymilk_playlists_from_document(panel)
            .into_iter()
            .find(|playlist| playlist.id == active_playlist)
            .map(|playlist| playlist.preset_keys.into_iter().collect::<BTreeSet<_>>())
            .unwrap_or_default()
    };
    let favorites = favorite_presets.borrow();
    (0..count)
        .filter(|index| {
            let key = rustymilk_preset_favorite_key(panel, imported_presets, *index);
            let favorite_matches = !rustymilk_favorites_only(panel) || favorites.contains(&key);
            let playlist_matches = active_playlist.is_empty() || playlist_keys.contains(&key);
            let search_matches = query.is_empty()
                || rustymilk_active_preset_name(panel, imported_presets, *index)
                    .to_ascii_lowercase()
                    .contains(&query);
            favorite_matches && playlist_matches && search_matches
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_empty_visible_preset_message(panel: &web_sys::Element) -> &'static str {
    if !rustymilk_preset_search_query(panel).is_empty() {
        "No matching RustyMilk presets"
    } else if rustymilk_favorites_only(panel) {
        "No favorite RustyMilk presets"
    } else if !rustymilk_active_playlist(panel).is_empty() {
        "No presets in active RustyMilk playlist"
    } else {
        "No RustyMilk presets"
    }
}

#[cfg(target_arch = "wasm32")]
fn update_rustymilk_library_controls(
    document: &web_sys::Document,
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
    index: usize,
) {
    let count = rustymilk_preset_count(panel, imported_presets).max(1);
    let index = index % count;
    let favorite_key = rustymilk_preset_favorite_key(panel, imported_presets, index);
    let favorite_count = favorite_presets.borrow().len();
    let is_favorite = favorite_presets.borrow().contains(&favorite_key);
    if let Some(button) = document.get_element_by_id("slskr-rustymilk-favorite") {
        button.set_text_content(Some(if is_favorite {
            "Unfavorite"
        } else {
            "Favorite"
        }));
        let _ = button.set_attribute("aria-pressed", if is_favorite { "true" } else { "false" });
    }
    if let Some(button) = document.get_element_by_id("slskr-rustymilk-favorites-only") {
        let only = rustymilk_favorites_only(panel);
        button.set_text_content(Some(if only { "All presets" } else { "Favorites" }));
        let _ = button.set_attribute("aria-pressed", if only { "true" } else { "false" });
    }
    if let Some(status_label) = document.get_element_by_id("slskr-rustymilk-library-status") {
        let imported_count = imported_presets.borrow().len();
        let filter = if rustymilk_favorites_only(panel) {
            " / favorites only"
        } else {
            ""
        };
        let search = rustymilk_preset_search_query(panel);
        let search = if search.is_empty() { "" } else { " / filtered" };
        let playlist = if rustymilk_active_playlist(panel).is_empty() {
            ""
        } else {
            " / playlist"
        };
        status_label.set_text_content(Some(&format!(
            "Preset {} of {} / {} imported / {} favorite{}{}{}{}",
            index + 1,
            count,
            imported_count,
            favorite_count,
            if favorite_count == 1 { "" } else { "s" },
            filter,
            search,
            playlist
        )));
    }
}

#[cfg(target_arch = "wasm32")]
fn set_rustymilk_active_preset(
    document: &web_sys::Document,
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
    index: usize,
    status: &str,
) {
    let count = rustymilk_preset_count(panel, imported_presets).max(1);
    let index = index % count;
    let _ = panel.set_attribute("data-slskr-rustymilk-preset-index", &index.to_string());
    if let Some(label) = document.get_element_by_id("slskr-rustymilk-preset") {
        label.set_text_content(Some(&rustymilk_active_preset_name(
            panel,
            imported_presets,
            index,
        )));
    }
    update_rustymilk_library_controls(document, panel, imported_presets, favorite_presets, index);
    set_player_status(document, status);
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_imported_presets(window: &web_sys::Window) -> Vec<RustyMilkImportedPreset> {
    let Some(storage) = window.local_storage().ok().flatten() else {
        return Vec::new();
    };
    let Some(raw) = storage
        .get_item(RUSTYMILK_IMPORTED_PRESETS_STORAGE_KEY)
        .ok()
        .flatten()
    else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let source = item.get("source")?.as_str()?.to_string();
            if source.trim().is_empty() {
                return None;
            }
            let title = item
                .get("title")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| rustymilk_preset_name(&source));
            Some(RustyMilkImportedPreset { source, title })
        })
        .take(20)
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn persist_rustymilk_imported_presets(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
) {
    let Some(window) = document.default_view() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let items = imported_presets
        .borrow()
        .iter()
        .take(20)
        .map(|preset| {
            serde_json::json!({
                "source": preset.source.clone(),
                "title": preset.title.clone(),
            })
        })
        .collect::<Vec<_>>();
    let _ = storage.set_item(
        RUSTYMILK_IMPORTED_PRESETS_STORAGE_KEY,
        &serde_json::Value::Array(items).to_string(),
    );
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_favorite_presets(window: &web_sys::Window) -> BTreeSet<String> {
    let Some(storage) = window.local_storage().ok().flatten() else {
        return BTreeSet::new();
    };
    let Some(raw) = storage
        .get_item(RUSTYMILK_FAVORITE_PRESETS_STORAGE_KEY)
        .ok()
        .flatten()
    else {
        return BTreeSet::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return BTreeSet::new();
    };
    let Some(items) = value.as_array() else {
        return BTreeSet::new();
    };
    items
        .iter()
        .filter_map(|item| item.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .take(200)
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn persist_rustymilk_favorite_presets(
    document: &web_sys::Document,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(window) = document.default_view() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let items = favorite_presets
        .borrow()
        .iter()
        .take(200)
        .map(|key| serde_json::Value::String(key.clone()))
        .collect::<Vec<_>>();
    let _ = storage.set_item(
        RUSTYMILK_FAVORITE_PRESETS_STORAGE_KEY,
        &serde_json::Value::Array(items).to_string(),
    );
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_preset_search(window: &web_sys::Window) -> String {
    window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| {
            storage
                .get_item(RUSTYMILK_PRESET_SEARCH_STORAGE_KEY)
                .ok()
                .flatten()
        })
        .unwrap_or_default()
        .chars()
        .take(120)
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_simple_setting(
    window: &web_sys::Window,
    storage_key: &str,
    fallback: &str,
) -> String {
    window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(storage_key).ok().flatten())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(target_arch = "wasm32")]
fn persist_rustymilk_simple_setting(document: &web_sys::Document, storage_key: &str, value: &str) {
    let Some(window) = document.default_view() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let _ = storage.set_item(storage_key, value);
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_active_playlist(window: &web_sys::Window) -> String {
    load_rustymilk_simple_setting(window, RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY, "")
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_playlists(window: &web_sys::Window) -> Vec<RustyMilkPlaylist> {
    let Some(storage) = window.local_storage().ok().flatten() else {
        return Vec::new();
    };
    let Some(raw) = storage
        .get_item(RUSTYMILK_PLAYLISTS_STORAGE_KEY)
        .ok()
        .flatten()
    else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let id = item.get("id")?.as_str()?.to_string();
            let name = item.get("name")?.as_str()?.to_string();
            let preset_keys = item
                .get("presetKeys")?
                .as_array()?
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<Vec<_>>();
            if id.is_empty() || name.is_empty() || preset_keys.is_empty() {
                return None;
            }
            Some(RustyMilkPlaylist {
                id,
                name,
                preset_keys,
            })
        })
        .take(12)
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_playlists_from_document(panel: &web_sys::Element) -> Vec<RustyMilkPlaylist> {
    panel
        .owner_document()
        .and_then(|document| document.default_view())
        .map(|window| load_rustymilk_playlists(&window))
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn persist_rustymilk_playlists(document: &web_sys::Document, playlists: &[RustyMilkPlaylist]) {
    let Some(window) = document.default_view() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let items = playlists
        .iter()
        .take(12)
        .map(|playlist| {
            serde_json::json!({
                "id": playlist.id,
                "name": playlist.name,
                "presetKeys": playlist.preset_keys,
            })
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        let _ = storage.remove_item(RUSTYMILK_PLAYLISTS_STORAGE_KEY);
    } else {
        let _ = storage.set_item(
            RUSTYMILK_PLAYLISTS_STORAGE_KEY,
            &serde_json::Value::Array(items).to_string(),
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn persist_rustymilk_preset_search(document: &web_sys::Document, query: &str) {
    let Some(window) = document.default_view() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    if query.trim().is_empty() {
        let _ = storage.remove_item(RUSTYMILK_PRESET_SEARCH_STORAGE_KEY);
    } else {
        let _ = storage.set_item(RUSTYMILK_PRESET_SEARCH_STORAGE_KEY, query.trim());
    }
}

#[cfg(target_arch = "wasm32")]
fn load_rustymilk_texture_assets(window: &web_sys::Window) -> BTreeMap<String, String> {
    let Some(storage) = window.local_storage().ok().flatten() else {
        return BTreeMap::new();
    };
    let Some(raw) = storage
        .get_item(RUSTYMILK_TEXTURE_ASSETS_STORAGE_KEY)
        .ok()
        .flatten()
    else {
        return BTreeMap::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return BTreeMap::new();
    };
    let Some(map) = value.as_object() else {
        return BTreeMap::new();
    };
    map.iter()
        .filter_map(|(name, value)| {
            let data_url = value.as_str()?;
            if name.trim().is_empty() || !data_url.starts_with("data:image/") {
                return None;
            }
            Some((name.clone(), data_url.to_string()))
        })
        .take(100)
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn persist_rustymilk_texture_assets(
    document: &web_sys::Document,
    texture_assets: &Rc<RefCell<BTreeMap<String, String>>>,
) {
    let Some(window) = document.default_view() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let mut map = serde_json::Map::new();
    for (name, data_url) in texture_assets.borrow().iter().take(100) {
        map.insert(name.clone(), serde_json::Value::String(data_url.clone()));
    }
    let _ = storage.set_item(
        RUSTYMILK_TEXTURE_ASSETS_STORAGE_KEY,
        &serde_json::Value::Object(map).to_string(),
    );
}

#[cfg(target_arch = "wasm32")]
fn import_rustymilk_preset(
    window: &web_sys::Window,
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
) {
    let Ok(Some(source)) = window.prompt_with_message("Paste a RustyMilk preset") else {
        return;
    };
    match validate_rustymilk_import(&source) {
        Ok(title) => {
            if let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") {
                imported_presets.borrow_mut().insert(
                    0,
                    RustyMilkImportedPreset {
                        source,
                        title: title.clone(),
                    },
                );
                persist_rustymilk_imported_presets(document, &imported_presets);
                set_rustymilk_active_preset(
                    document,
                    &panel,
                    &imported_presets,
                    &Rc::new(RefCell::new(
                        document
                            .default_view()
                            .map(|window| load_rustymilk_favorite_presets(&window))
                            .unwrap_or_default(),
                    )),
                    0,
                    "RustyMilk preset imported",
                );
            }
        }
        Err(error) => set_player_status(document, &error),
    }
}

#[cfg(target_arch = "wasm32")]
fn reset_rustymilk_import(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    imported_presets.borrow_mut().clear();
    persist_rustymilk_imported_presets(document, &imported_presets);
    let _ = panel.remove_attribute("data-slskr-rustymilk-custom-source");
    set_rustymilk_active_preset(
        document,
        &panel,
        &imported_presets,
        favorite_presets,
        0,
        "RustyMilk import reset",
    );
}

#[cfg(target_arch = "wasm32")]
fn cycle_rustymilk_preset(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    cycle_rustymilk_preset_by(document, imported_presets, favorite_presets, 1);
}

#[cfg(target_arch = "wasm32")]
fn cycle_rustymilk_preset_by(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
    offset: isize,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let current = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let visible = rustymilk_visible_preset_indices(&panel, imported_presets, favorite_presets);
    if visible.is_empty() {
        set_player_status(document, rustymilk_empty_visible_preset_message(&panel));
        return;
    }
    let visible_position = visible
        .iter()
        .position(|index| *index == current)
        .unwrap_or(0) as isize;
    let next_position = (visible_position + offset).rem_euclid(visible.len() as isize) as usize;
    let next = visible[next_position];
    set_rustymilk_active_preset(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        next,
        "RustyMilk preset changed",
    );
}

#[cfg(target_arch = "wasm32")]
fn search_rustymilk_preset(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let query = document
        .get_element_by_id("slskr-rustymilk-search")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value().trim().to_ascii_lowercase())
        .unwrap_or_default();
    if query.is_empty() {
        set_player_status(document, "Enter a preset search");
        return;
    }
    let _ = panel.set_attribute("data-slskr-rustymilk-search", &query);
    persist_rustymilk_preset_search(document, &query);
    for index in rustymilk_visible_preset_indices(&panel, imported_presets, favorite_presets) {
        set_rustymilk_active_preset(
            document,
            &panel,
            imported_presets,
            favorite_presets,
            index,
            "RustyMilk preset filter applied",
        );
        return;
    }
    set_player_status(document, "No matching RustyMilk preset");
}

#[cfg(target_arch = "wasm32")]
fn clear_rustymilk_preset_search(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let _ = panel.set_attribute("data-slskr-rustymilk-search", "");
    if let Some(input) = document
        .get_element_by_id("slskr-rustymilk-search")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        input.set_value("");
    }
    persist_rustymilk_preset_search(document, "");
    let current = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    update_rustymilk_library_controls(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        current,
    );
    set_player_status(document, "RustyMilk preset search cleared");
}

#[cfg(target_arch = "wasm32")]
fn toggle_rustymilk_favorite(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let index = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let key = rustymilk_preset_favorite_key(&panel, imported_presets, index);
    let added = {
        let mut favorites = favorite_presets.borrow_mut();
        if favorites.contains(&key) {
            favorites.remove(&key);
            false
        } else {
            favorites.insert(key);
            true
        }
    };
    persist_rustymilk_favorite_presets(document, &favorite_presets);
    update_rustymilk_library_controls(document, &panel, imported_presets, &favorite_presets, index);
    set_player_status(
        document,
        if added {
            "RustyMilk preset favorited"
        } else {
            "RustyMilk preset unfavorited"
        },
    );
}

#[cfg(target_arch = "wasm32")]
fn toggle_rustymilk_favorites_only(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let next = !rustymilk_favorites_only(&panel);
    let _ = panel.set_attribute(
        "data-slskr-rustymilk-favorites-only",
        if next { "true" } else { "false" },
    );
    let current = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let visible = rustymilk_visible_preset_indices(&panel, imported_presets, favorite_presets);
    if next && visible.is_empty() {
        let _ = panel.set_attribute("data-slskr-rustymilk-favorites-only", "false");
        update_rustymilk_library_controls(
            document,
            &panel,
            imported_presets,
            favorite_presets,
            current,
        );
        set_player_status(document, rustymilk_empty_visible_preset_message(&panel));
        return;
    }
    let selected = if next && !visible.contains(&current) {
        visible[0]
    } else {
        current
    };
    set_rustymilk_active_preset(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        selected,
        if next {
            "Showing favorite RustyMilk presets"
        } else {
            "Showing all RustyMilk presets"
        },
    );
}

#[cfg(target_arch = "wasm32")]
fn random_rustymilk_preset(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let visible = rustymilk_visible_preset_indices(&panel, imported_presets, favorite_presets);
    if visible.is_empty() {
        set_player_status(document, rustymilk_empty_visible_preset_message(&panel));
        return;
    }
    let index = (js_sys::Math::random() * visible.len() as f64).floor() as usize;
    set_rustymilk_active_preset(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        visible[index.min(visible.len() - 1)],
        "Random RustyMilk preset selected",
    );
}

#[cfg(target_arch = "wasm32")]
fn remove_rustymilk_active_preset(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let index = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let imports_len = imported_presets.borrow().len();
    if index < imports_len {
        let key = rustymilk_preset_favorite_key(&panel, &imported_presets, index);
        imported_presets.borrow_mut().remove(index);
        favorite_presets.borrow_mut().remove(&key);
        persist_rustymilk_imported_presets(document, &imported_presets);
        persist_rustymilk_favorite_presets(document, favorite_presets);
        let next = index.min(rustymilk_preset_count(&panel, &imported_presets).saturating_sub(1));
        set_rustymilk_active_preset(
            document,
            &panel,
            &imported_presets,
            favorite_presets,
            next,
            "RustyMilk preset removed",
        );
        return;
    }
    let custom_index = index.saturating_sub(imports_len);
    if custom_index == 0
        && panel
            .get_attribute("data-slskr-rustymilk-custom-source")
            .is_some_and(|value| !value.trim().is_empty())
    {
        let key = rustymilk_preset_favorite_key(&panel, &imported_presets, index);
        let _ = panel.remove_attribute("data-slskr-rustymilk-custom-source");
        favorite_presets.borrow_mut().remove(&key);
        persist_rustymilk_favorite_presets(document, favorite_presets);
        set_rustymilk_active_preset(
            document,
            &panel,
            &imported_presets,
            favorite_presets,
            0,
            "RustyMilk pasted preset removed",
        );
    } else {
        set_player_status(document, "Bundled RustyMilk presets cannot be removed");
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_rustymilk_library(
    window: &web_sys::Window,
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: Rc<RefCell<BTreeSet<String>>>,
) {
    let Ok(confirmed) =
        window.confirm_with_message("Clear imported RustyMilk presets and favorites?")
    else {
        return;
    };
    if !confirmed {
        return;
    }
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    imported_presets.borrow_mut().clear();
    favorite_presets.borrow_mut().clear();
    persist_rustymilk_imported_presets(document, &imported_presets);
    persist_rustymilk_favorite_presets(document, &favorite_presets);
    let _ = panel.remove_attribute("data-slskr-rustymilk-custom-source");
    let _ = panel.set_attribute("data-slskr-rustymilk-favorites-only", "false");
    set_rustymilk_active_preset(
        document,
        &panel,
        &imported_presets,
        &favorite_presets,
        0,
        "RustyMilk library cleared",
    );
}

#[cfg(target_arch = "wasm32")]
fn save_rustymilk_playlist(
    window: &web_sys::Window,
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let keys = rustymilk_visible_preset_indices(&panel, imported_presets, favorite_presets)
        .into_iter()
        .map(|index| rustymilk_preset_favorite_key(&panel, imported_presets, index))
        .collect::<Vec<_>>();
    if keys.is_empty() {
        set_player_status(document, rustymilk_empty_visible_preset_message(&panel));
        return;
    }
    let Ok(Some(name)) = window.prompt_with_message("Name this RustyMilk playlist") else {
        return;
    };
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let id = format!(
        "playlist-{}",
        (js_sys::Date::now() + js_sys::Math::random() * 1000.0).floor() as u64
    );
    let mut playlists = load_rustymilk_playlists(window);
    playlists.insert(
        0,
        RustyMilkPlaylist {
            id: id.clone(),
            name: name.to_string(),
            preset_keys: keys,
        },
    );
    playlists.truncate(12);
    persist_rustymilk_playlists(document, &playlists);
    persist_rustymilk_simple_setting(document, RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY, &id);
    let _ = panel.set_attribute("data-slskr-rustymilk-playlist", &id);
    set_player_status(document, "RustyMilk playlist saved");
}

#[cfg(target_arch = "wasm32")]
fn cycle_rustymilk_playlist(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let Some(window) = document.default_view() else {
        return;
    };
    let playlists = load_rustymilk_playlists(&window);
    if playlists.is_empty() {
        set_player_status(document, "No RustyMilk playlists saved");
        return;
    }
    let current = rustymilk_active_playlist(&panel);
    let next_index = playlists
        .iter()
        .position(|playlist| playlist.id == current)
        .map(|index| (index + 1) % playlists.len())
        .unwrap_or(0);
    let playlist = &playlists[next_index];
    let _ = panel.set_attribute("data-slskr-rustymilk-playlist", &playlist.id);
    persist_rustymilk_simple_setting(
        document,
        RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY,
        &playlist.id,
    );
    let visible = rustymilk_visible_preset_indices(&panel, imported_presets, favorite_presets);
    if let Some(index) = visible.first() {
        set_rustymilk_active_preset(
            document,
            &panel,
            imported_presets,
            favorite_presets,
            *index,
            &format!("RustyMilk playlist: {}", playlist.name),
        );
    } else {
        set_player_status(
            document,
            "Active RustyMilk playlist has no matching presets",
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn rename_rustymilk_playlist(window: &web_sys::Window, document: &web_sys::Document) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let active = rustymilk_active_playlist(&panel);
    if active.is_empty() {
        set_player_status(document, "No active RustyMilk playlist");
        return;
    }
    let Ok(Some(name)) = window.prompt_with_message("Rename active RustyMilk playlist") else {
        return;
    };
    let mut playlists = load_rustymilk_playlists(window);
    if let Some(playlist) = playlists.iter_mut().find(|playlist| playlist.id == active) {
        playlist.name = name.trim().to_string();
        persist_rustymilk_playlists(document, &playlists);
        set_player_status(document, "RustyMilk playlist renamed");
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_rustymilk_playlist_filter(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let _ = panel.set_attribute("data-slskr-rustymilk-playlist", "");
    persist_rustymilk_simple_setting(document, RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY, "");
    update_rustymilk_library_controls(document, &panel, imported_presets, favorite_presets, 0);
    set_player_status(document, "Showing all RustyMilk presets");
}

#[cfg(target_arch = "wasm32")]
fn remove_rustymilk_playlist(
    window: &web_sys::Window,
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let active = rustymilk_active_playlist(&panel);
    if active.is_empty() {
        set_player_status(document, "No active RustyMilk playlist");
        return;
    }
    let Ok(true) = window.confirm_with_message("Remove active RustyMilk playlist?") else {
        return;
    };
    let playlists = load_rustymilk_playlists(window)
        .into_iter()
        .filter(|playlist| playlist.id != active)
        .collect::<Vec<_>>();
    persist_rustymilk_playlists(document, &playlists);
    let _ = panel.set_attribute("data-slskr-rustymilk-playlist", "");
    persist_rustymilk_simple_setting(document, RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY, "");
    update_rustymilk_library_controls(document, &panel, imported_presets, favorite_presets, 0);
    set_player_status(document, "RustyMilk playlist removed");
}

#[cfg(target_arch = "wasm32")]
fn store_rustymilk_edited_source(
    document: &web_sys::Document,
    panel: &web_sys::Element,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
    source: String,
    status: &str,
) {
    let title = rustymilk_preset_name(&source);
    let current = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if current < imported_presets.borrow().len() {
        if let Some(preset) = imported_presets.borrow_mut().get_mut(current) {
            preset.source = source;
            preset.title = title;
        }
        persist_rustymilk_imported_presets(document, &imported_presets);
        set_rustymilk_active_preset(
            document,
            panel,
            &imported_presets,
            favorite_presets,
            current,
            status,
        );
    } else {
        imported_presets.borrow_mut().insert(
            0,
            RustyMilkImportedPreset {
                source,
                title: title.clone(),
            },
        );
        persist_rustymilk_imported_presets(document, &imported_presets);
        let _ = panel.remove_attribute("data-slskr-rustymilk-custom-source");
        set_rustymilk_active_preset(
            document,
            panel,
            &imported_presets,
            favorite_presets,
            0,
            status,
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn active_rustymilk_parsed(
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
) -> RustyMilkPresetSet {
    let index = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let source = rustymilk_active_preset_source(panel, imported_presets, index);
    parse_rustymilk_preset_set(&source, source.contains("[preset01]"))
}

#[cfg(target_arch = "wasm32")]
fn apply_rustymilk_parameter(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let key = document
        .get_element_by_id("slskr-rustymilk-parameter")
        .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|select| select.value())
        .unwrap_or_else(|| "decay".to_string());
    let value = document
        .get_element_by_id("slskr-rustymilk-parameter-value")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_else(|| "0.9".to_string());
    let Some(number) = value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
    else {
        set_player_status(document, "RustyMilk parameter needs a finite number");
        return;
    };
    let mut parsed = active_rustymilk_parsed(&panel, &imported_presets);
    if let Some(preset) = parsed.presets.first_mut() {
        preset
            .base_values
            .insert(key, RustyMilkValue::Number(clamp_range(number, -2.0, 2.0)));
    }
    store_rustymilk_edited_source(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        serialize_rustymilk_preset_set(&parsed),
        "RustyMilk parameter applied",
    );
}

#[cfg(target_arch = "wasm32")]
fn randomize_rustymilk_parameters(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let mut parsed = active_rustymilk_parsed(&panel, &imported_presets);
    if let Some(preset) = parsed.presets.first_mut() {
        for (key, min, max) in [
            ("decay", 0.82, 0.97),
            ("zoom", 0.85, 1.20),
            ("rot", -0.06, 0.06),
            ("wave_r", 0.05, 1.0),
            ("wave_g", 0.05, 1.0),
            ("wave_b", 0.05, 1.0),
            ("wave_a", 0.55, 1.0),
        ] {
            let value = min + js_sys::Math::random() * (max - min);
            preset
                .base_values
                .insert(key.to_string(), RustyMilkValue::Number(value));
        }
    }
    store_rustymilk_edited_source(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        serialize_rustymilk_preset_set(&parsed),
        "RustyMilk parameters randomized",
    );
}

#[cfg(target_arch = "wasm32")]
fn import_rustymilk_fragment(
    window: &web_sys::Window,
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
    fragment_type: &str,
) {
    let Ok(Some(source)) =
        window.prompt_with_message(&format!("Paste a RustyMilk {fragment_type} fragment"))
    else {
        return;
    };
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let fragment = parse_rustymilk_fragment(&source, fragment_type, fragment_type);
    let mut parsed = active_rustymilk_parsed(&panel, &imported_presets);
    if let Some(preset) = parsed.presets.first_mut() {
        if fragment.fragment_type == "wave" {
            preset.waves.extend(fragment.entries);
        } else {
            preset.shapes.extend(fragment.entries);
        }
    }
    store_rustymilk_edited_source(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        serialize_rustymilk_preset_set(&parsed),
        "RustyMilk fragment imported",
    );
}

#[cfg(target_arch = "wasm32")]
fn export_rustymilk_fragment(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    fragment_type: &str,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let parsed = active_rustymilk_parsed(&panel, imported_presets);
    let Some(preset) = parsed.presets.first() else {
        return;
    };
    let entry = if fragment_type == "wave" {
        preset.waves.first()
    } else {
        preset.shapes.first()
    };
    let Some(entry) = entry else {
        set_player_status(
            document,
            &format!("No RustyMilk {fragment_type} fragment available"),
        );
        return;
    };
    let text = serialize_rustymilk_fragment(entry, fragment_type);
    download_rustymilk_text(document, &format!("slskr.{fragment_type}"), &text);
}

#[cfg(target_arch = "wasm32")]
fn remove_rustymilk_fragment(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
    fragment_type: &str,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let mut parsed = active_rustymilk_parsed(&panel, &imported_presets);
    let removed = if let Some(preset) = parsed.presets.first_mut() {
        if fragment_type == "wave" {
            if preset.waves.is_empty() {
                false
            } else {
                preset.waves.remove(0);
                true
            }
        } else if preset.shapes.is_empty() {
            false
        } else {
            preset.shapes.remove(0);
            true
        }
    } else {
        false
    };
    if !removed {
        set_player_status(
            document,
            &format!("No RustyMilk {fragment_type} fragment available"),
        );
        return;
    }
    store_rustymilk_edited_source(
        document,
        &panel,
        imported_presets,
        favorite_presets,
        serialize_rustymilk_preset_set(&parsed),
        "RustyMilk fragment removed",
    );
}

#[cfg(target_arch = "wasm32")]
fn export_rustymilk_preset(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let index = panel
        .get_attribute("data-slskr-rustymilk-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let source = rustymilk_active_preset_source(&panel, imported_presets, index);
    download_rustymilk_text(document, "slskr.milk", &source);
}

#[cfg(target_arch = "wasm32")]
fn download_rustymilk_text(document: &web_sys::Document, file_name: &str, text: &str) {
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(text));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("text/plain");
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &options) else {
        set_player_status(document, "RustyMilk export failed");
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        set_player_status(document, "RustyMilk export failed");
        return;
    };
    if let Some(anchor) = document
        .create_element("a")
        .ok()
        .and_then(|element| element.dyn_into::<web_sys::HtmlAnchorElement>().ok())
    {
        anchor.set_href(&url);
        anchor.set_download(file_name);
        anchor.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
    set_player_status(document, "RustyMilk export downloaded");
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_fps_cap_ms(panel: &web_sys::Element) -> f64 {
    match panel
        .get_attribute("data-slskr-rustymilk-fps")
        .unwrap_or_else(|| "full".to_string())
        .as_str()
    {
        "60" => 1000.0 / 60.0,
        "30" => 1000.0 / 30.0,
        "24" => 1000.0 / 24.0,
        _ => 0.0,
    }
}

#[cfg(target_arch = "wasm32")]
fn cycle_rustymilk_automation(document: &web_sys::Document) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let next = match panel
        .get_attribute("data-slskr-rustymilk-automation")
        .unwrap_or_else(|| "off".to_string())
        .as_str()
    {
        "off" => "beat",
        "beat" => "timed",
        _ => "off",
    };
    let _ = panel.set_attribute("data-slskr-rustymilk-automation", next);
    persist_rustymilk_simple_setting(document, RUSTYMILK_AUTOMATION_STORAGE_KEY, next);
    set_player_status(document, &format!("RustyMilk automation: {next}"));
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_select_number(document: &web_sys::Document, id: &str, fallback: f64) -> f64 {
    document
        .get_element_by_id(id)
        .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .and_then(|select| select.value().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

#[cfg(target_arch = "wasm32")]
fn maybe_advance_rustymilk_automation(
    document: &web_sys::Document,
    panel: &web_sys::Element,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
    state: &mut RustyMilkAutomationState,
    time: f64,
    energy: f64,
) {
    match panel
        .get_attribute("data-slskr-rustymilk-automation")
        .unwrap_or_else(|| "off".to_string())
        .as_str()
    {
        "timed" => {
            let interval =
                rustymilk_select_number(document, "slskr-rustymilk-automation-interval", 30.0);
            if state.last_preset_at == 0.0 {
                state.last_preset_at = time;
            } else if time - state.last_preset_at >= interval {
                cycle_rustymilk_preset(document, imported_presets, favorite_presets);
                state.last_preset_at = time;
            }
        }
        "beat" => {
            state.smoothed_energy = state.smoothed_energy * 0.92 + energy * 0.08;
            if energy > (state.smoothed_energy * 1.25).max(0.35) && time - state.last_beat_at > 0.25
            {
                state.last_beat_at = time;
                state.beat_count += 1;
                let beats =
                    rustymilk_select_number(document, "slskr-rustymilk-automation-beats", 8.0)
                        as usize;
                if state.beat_count >= beats.max(1) {
                    cycle_rustymilk_preset(document, imported_presets, favorite_presets);
                    state.beat_count = 0;
                    state.last_preset_at = time;
                }
            }
        }
        _ => {}
    }
}

#[cfg(target_arch = "wasm32")]
fn toggle_rustymilk_debug(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
) {
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let next = panel.get_attribute("data-slskr-rustymilk-debug").as_deref() != Some("true");
    let _ = panel.set_attribute(
        "data-slskr-rustymilk-debug",
        if next { "true" } else { "false" },
    );
    if let Some(debug) = document.get_element_by_id("slskr-rustymilk-debug") {
        if next {
            let parsed = active_rustymilk_parsed(&panel, imported_presets);
            let preset = parsed.presets.first();
            debug.set_text_content(Some(&format!(
                "format: {}\npresets: {}\nshapes: {}\nwaves: {}\nquality: {}\nfps: {}\nautomation: {}",
                parsed.format,
                parsed.presets.len(),
                preset.map(|preset| preset.shapes.len()).unwrap_or_default(),
                preset.map(|preset| preset.waves.len()).unwrap_or_default(),
                panel.get_attribute("data-slskr-rustymilk-quality").unwrap_or_default(),
                panel.get_attribute("data-slskr-rustymilk-fps").unwrap_or_default(),
                panel.get_attribute("data-slskr-rustymilk-automation").unwrap_or_default(),
            )));
            let _ = debug.remove_attribute("hidden");
        } else {
            let _ = debug.set_attribute("hidden", "");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn import_rustymilk_texture_asset(
    window: &web_sys::Window,
    document: &web_sys::Document,
    texture_assets: Rc<RefCell<BTreeMap<String, String>>>,
) {
    let Ok(Some(value)) =
        window.prompt_with_message("Paste a RustyMilk texture as name=data:image/png;base64,...")
    else {
        return;
    };
    let Some((name, data_url)) = value.split_once('=') else {
        set_player_status(document, "Texture import needs name=data URL");
        return;
    };
    let name = name.trim();
    let data_url = data_url.trim();
    if name.is_empty() || !data_url.starts_with("data:image/") {
        set_player_status(document, "Texture import needs an image data URL");
        return;
    }
    store_rustymilk_texture_asset(document, texture_assets, name, data_url);
}

#[cfg(target_arch = "wasm32")]
fn rustymilk_import_file_name(file: &web_sys::File) -> String {
    js_sys::Reflect::get(file.as_ref(), &JsValue::from_str("webkitRelativePath"))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| file.name())
}

#[cfg(target_arch = "wasm32")]
fn is_rustymilk_preset_file_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".milk") || lower.ends_with(".milk2") || lower.ends_with(".txt")
}

#[cfg(target_arch = "wasm32")]
fn is_rustymilk_texture_file_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".gif"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

#[cfg(target_arch = "wasm32")]
fn mount_rustymilk_preset_input(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
) -> Result<(), JsValue> {
    let Some(input) = document
        .get_element_by_id("slskr-rustymilk-preset-input")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return Ok(());
    };
    if input.has_attribute("data-slskr-mounted") {
        return Ok(());
    }
    input.set_attribute("data-slskr-mounted", "true")?;
    let document_for_change = document.clone();
    let imports_for_change = imported_presets.clone();
    let callback =
        Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
            let Some(input) = event
                .current_target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(files) = input.files() else {
                return;
            };
            if files.length() == 0 {
                return;
            }
            for index in 0..files.length() {
                let Some(file) = files.item(index) else {
                    continue;
                };
                let file_name = rustymilk_import_file_name(&file);
                let Ok(reader) = web_sys::FileReader::new() else {
                    set_player_status(
                        &document_for_change,
                        "Preset import needs FileReader support",
                    );
                    return;
                };
                let document_for_load = document_for_change.clone();
                let imports_for_load = imports_for_change.clone();
                let reader_for_load = reader.clone();
                let onload = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
                    let Some(source) = reader_for_load
                        .result()
                        .ok()
                        .and_then(|value| value.as_string())
                    else {
                        set_player_status(&document_for_load, "Preset file could not be read");
                        return;
                    };
                    store_rustymilk_imported_preset(
                        &document_for_load,
                        imports_for_load.clone(),
                        &file_name,
                        &source,
                    );
                }));
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                let _ = reader.read_as_text(&file);
            }
            set_player_status(
                &document_for_change,
                &format!(
                    "Reading {} RustyMilk preset file{}",
                    files.length(),
                    if files.length() == 1 { "" } else { "s" }
                ),
            );
            input.set_value("");
        }));
    input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn store_rustymilk_imported_preset(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    file_name: &str,
    source: &str,
) {
    let title = match validate_rustymilk_import(source) {
        Ok(title) => {
            if title.trim().is_empty() {
                file_name
                    .trim_end_matches(".milk2")
                    .trim_end_matches(".milk")
                    .trim_end_matches(".txt")
                    .to_string()
            } else {
                title
            }
        }
        Err(error) => {
            set_player_status(document, &format!("{file_name}: {error}"));
            return;
        }
    };
    let Some(panel) = document.get_element_by_id("slskr-rust-rustymilk") else {
        return;
    };
    let index = {
        let mut imports = imported_presets.borrow_mut();
        imports.push(RustyMilkImportedPreset {
            source: source.to_string(),
            title,
        });
        imports.len() - 1
    };
    persist_rustymilk_imported_presets(document, &imported_presets);
    set_rustymilk_active_preset(
        document,
        &panel,
        &imported_presets,
        &Rc::new(RefCell::new(
            document
                .default_view()
                .map(|window| load_rustymilk_favorite_presets(&window))
                .unwrap_or_default(),
        )),
        index,
        "RustyMilk preset imported",
    );
}

#[cfg(target_arch = "wasm32")]
fn read_rustymilk_preset_file(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    file: &web_sys::File,
    file_name: &str,
) {
    let Ok(reader) = web_sys::FileReader::new() else {
        set_player_status(document, "Preset import needs FileReader support");
        return;
    };
    let document_for_load = document.clone();
    let imports_for_load = imported_presets.clone();
    let file_name = file_name.to_string();
    let reader_for_load = reader.clone();
    let onload = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
        let Some(source) = reader_for_load
            .result()
            .ok()
            .and_then(|value| value.as_string())
        else {
            set_player_status(&document_for_load, "Preset file could not be read");
            return;
        };
        store_rustymilk_imported_preset(
            &document_for_load,
            imports_for_load.clone(),
            &file_name,
            &source,
        );
    }));
    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    let _ = reader.read_as_text(file);
}

#[cfg(target_arch = "wasm32")]
fn read_rustymilk_texture_file(
    document: &web_sys::Document,
    texture_assets: Rc<RefCell<BTreeMap<String, String>>>,
    file: &web_sys::File,
    file_name: &str,
) {
    let Ok(reader) = web_sys::FileReader::new() else {
        set_player_status(document, "Texture import needs FileReader support");
        return;
    };
    let document_for_load = document.clone();
    let assets_for_load = texture_assets.clone();
    let file_name = file_name.to_string();
    let reader_for_load = reader.clone();
    let onload = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
        let Some(data_url) = reader_for_load
            .result()
            .ok()
            .and_then(|value| value.as_string())
        else {
            set_player_status(&document_for_load, "Texture file could not be read");
            return;
        };
        store_rustymilk_texture_asset(
            &document_for_load,
            assets_for_load.clone(),
            &file_name,
            &data_url,
        );
    }));
    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    let _ = reader.read_as_data_url(file);
}

#[cfg(target_arch = "wasm32")]
fn mount_rustymilk_pack_input(
    document: &web_sys::Document,
    imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    texture_assets: Rc<RefCell<BTreeMap<String, String>>>,
) -> Result<(), JsValue> {
    let Some(input) = document
        .get_element_by_id("slskr-rustymilk-pack-input")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return Ok(());
    };
    if input.has_attribute("data-slskr-mounted") {
        return Ok(());
    }
    input.set_attribute("data-slskr-mounted", "true")?;
    let document_for_change = document.clone();
    let imports_for_change = imported_presets.clone();
    let assets_for_change = texture_assets.clone();
    let callback =
        Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
            let Some(input) = event
                .current_target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(files) = input.files() else {
                return;
            };
            if files.length() == 0 {
                return;
            }
            let mut readable = 0u32;
            for index in 0..files.length() {
                let Some(file) = files.item(index) else {
                    continue;
                };
                let file_name = rustymilk_import_file_name(&file);
                if is_rustymilk_preset_file_name(&file_name) {
                    readable += 1;
                    read_rustymilk_preset_file(
                        &document_for_change,
                        imports_for_change.clone(),
                        &file,
                        &file_name,
                    );
                } else if is_rustymilk_texture_file_name(&file_name) {
                    readable += 1;
                    read_rustymilk_texture_file(
                        &document_for_change,
                        assets_for_change.clone(),
                        &file,
                        &file_name,
                    );
                }
            }
            set_player_status(
                &document_for_change,
                &format!(
                    "Reading {} RustyMilk pack file{}",
                    readable,
                    if readable == 1 { "" } else { "s" }
                ),
            );
            input.set_value("");
        }));
    input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn mount_rustymilk_texture_input(
    document: &web_sys::Document,
    texture_assets: Rc<RefCell<BTreeMap<String, String>>>,
) -> Result<(), JsValue> {
    let Some(input) = document
        .get_element_by_id("slskr-rustymilk-texture-input")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return Ok(());
    };
    if input.has_attribute("data-slskr-mounted") {
        return Ok(());
    }
    input.set_attribute("data-slskr-mounted", "true")?;
    let document_for_change = document.clone();
    let assets_for_change = texture_assets.clone();
    let callback =
        Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
            let Some(input) = event
                .current_target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(files) = input.files() else {
                return;
            };
            if files.length() == 0 {
                return;
            }
            for index in 0..files.length() {
                let Some(file) = files.item(index) else {
                    continue;
                };
                let file_name = rustymilk_import_file_name(&file);
                let Ok(reader) = web_sys::FileReader::new() else {
                    set_player_status(
                        &document_for_change,
                        "Texture import needs FileReader support",
                    );
                    return;
                };
                let document_for_load = document_for_change.clone();
                let assets_for_load = assets_for_change.clone();
                let reader_for_load = reader.clone();
                let onload = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
                    let Some(data_url) = reader_for_load
                        .result()
                        .ok()
                        .and_then(|value| value.as_string())
                    else {
                        set_player_status(&document_for_load, "Texture file could not be read");
                        return;
                    };
                    store_rustymilk_texture_asset(
                        &document_for_load,
                        assets_for_load.clone(),
                        &file_name,
                        &data_url,
                    );
                }));
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                let _ = reader.read_as_data_url(&file);
            }
            set_player_status(
                &document_for_change,
                &format!(
                    "Reading {} RustyMilk texture file{}",
                    files.length(),
                    if files.length() == 1 { "" } else { "s" }
                ),
            );
            input.set_value("");
        }));
    input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn store_rustymilk_texture_asset(
    document: &web_sys::Document,
    texture_assets: Rc<RefCell<BTreeMap<String, String>>>,
    name: &str,
    data_url: &str,
) {
    let aliases = get_rustymilk_texture_name_aliases(name);
    if aliases.is_empty() || !data_url.starts_with("data:image/") {
        set_player_status(document, "Texture import needs an image file");
        return;
    }
    {
        let mut assets = texture_assets.borrow_mut();
        for alias in &aliases {
            assets.insert(alias.clone(), data_url.to_string());
        }
    }
    persist_rustymilk_texture_assets(document, &texture_assets);
    update_rustymilk_texture_status(document, &texture_assets);
    set_player_status(document, "RustyMilk texture imported");
}

#[cfg(target_arch = "wasm32")]
fn update_rustymilk_texture_status(
    document: &web_sys::Document,
    texture_assets: &Rc<RefCell<BTreeMap<String, String>>>,
) {
    if let Some(label) = document.get_element_by_id("slskr-rustymilk-textures") {
        label.set_text_content(Some(&format!(
            "{} texture assets",
            texture_assets.borrow().len()
        )));
    }
}

#[cfg(target_arch = "wasm32")]
struct RustyMilkAudioAnalyzer {
    _context: web_sys::AudioContext,
    analyser: web_sys::AnalyserNode,
    frequency_bins: RefCell<Vec<u8>>,
    _source: web_sys::MediaElementAudioSourceNode,
    waveform_bins: RefCell<Vec<u8>>,
}

#[cfg(target_arch = "wasm32")]
impl RustyMilkAudioAnalyzer {
    fn new(audio: &web_sys::HtmlAudioElement) -> Result<Self, JsValue> {
        let context = web_sys::AudioContext::new()?;
        let source = context.create_media_element_source(audio)?;
        let analyser = context.create_analyser()?;
        analyser.set_fft_size(1024);
        source.connect_with_audio_node(&analyser)?;
        analyser.connect_with_audio_node(&context.destination())?;
        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
        Ok(Self {
            _context: context,
            analyser,
            frequency_bins,
            _source: source,
            waveform_bins,
        })
    }

    fn snapshot(&self, time: f64) -> RustyMilkAudioSnapshot {
        let mut frequency_bins = self.frequency_bins.borrow_mut();
        self.analyser.get_byte_frequency_data(&mut frequency_bins);
        let length = frequency_bins.len();
        if length == 0 {
            return RustyMilkAudioSnapshot::synthetic(time);
        }
        let band = |start: usize, end: usize| -> f64 {
            let end = end.min(length).max(start + 1);
            let mut total = 0.0;
            let mut count = 0.0;
            for index in start..end {
                total += frequency_bins[index] as f64 / 255.0;
                count += 1.0;
            }
            if count == 0.0 {
                0.0
            } else {
                total / count
            }
        };
        let spectrum = frequency_bins
            .iter()
            .map(|value| *value as f64 / 255.0)
            .collect::<Vec<_>>();
        let bands = RustyMilkAudioBands {
            bass: band(0, length / 8),
            mid: band(length / 8, length / 3),
            treble: band(length / 3, length),
            source: "audio",
        };
        drop(frequency_bins);
        let mut waveform_bins = self.waveform_bins.borrow_mut();
        self.analyser.get_byte_time_domain_data(&mut waveform_bins);
        let waveform = waveform_bins
            .iter()
            .map(|value| (*value as f64 - 128.0) / 128.0)
            .collect::<Vec<_>>();
        RustyMilkAudioSnapshot {
            bands,
            source: "audio",
            spectrum,
            waveform,
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
struct RustyMilkAudioSnapshot {
    bands: RustyMilkAudioBands,
    source: &'static str,
    spectrum: Vec<f64>,
    waveform: Vec<f64>,
}

#[cfg(target_arch = "wasm32")]
impl RustyMilkAudioSnapshot {
    fn synthetic(time: f64) -> Self {
        Self {
            bands: RustyMilkAudioBands::synthetic(time),
            source: "synthetic",
            spectrum: Vec::new(),
            waveform: Vec::new(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq)]
struct RustyMilkAudioBands {
    bass: f64,
    mid: f64,
    treble: f64,
    source: &'static str,
}

#[cfg(target_arch = "wasm32")]
impl RustyMilkAudioBands {
    fn synthetic(time: f64) -> Self {
        Self {
            bass: (time * 1.9).sin() * 0.5 + 0.5,
            mid: (time * 1.17 + 1.3).sin() * 0.5 + 0.5,
            treble: (time * 2.7 + 0.4).sin() * 0.5 + 0.5,
            source: "synthetic",
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn refresh_player_status(window: &web_sys::Window) -> Result<(), JsValue> {
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    set_player_status(&document, "Refreshing player");

    if let Ok(body) = fetch_text(window, &endpoint_url("/nowplaying")).await {
        let (title, detail) = player_now_playing_text(&body);
        let track = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| current_player_track(&value));
        let rating_key = track.as_ref().map(player_rating_key).unwrap_or_default();
        let radio_query = track
            .as_ref()
            .map(|track| build_player_radio_plan(Some(track)).primary_query)
            .unwrap_or_default();
        let direct_stream_url = track.as_ref().map(player_stream_url).unwrap_or_default();
        let stream_url = if let Some(track) = track.as_ref() {
            let content_id = track
                .get("contentId")
                .or_else(|| track.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty());
            if let Some(content_id) = content_id {
                let ticket_path = format!(
                    "/streams/{}/ticket",
                    percent_encode_player_stream_component(&content_id)
                );
                let ticket_body = serde_json::json!({"contentId": content_id}).to_string();
                match fetch_text_with_method(
                    window,
                    &endpoint_url(&ticket_path),
                    "POST",
                    Some(&ticket_body),
                )
                .await
                {
                    Ok(body) => serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|value| {
                            value
                                .get("url")
                                .or_else(|| value.get("streamUrl"))
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned)
                        })
                        .unwrap_or(direct_stream_url.clone()),
                    Err(_) => direct_stream_url.clone(),
                }
            } else {
                direct_stream_url.clone()
            }
        } else {
            String::new()
        };
        if let Some(element) = document.get_element_by_id("slskr-player-now") {
            element.set_text_content(Some(&title));
        }
        if let Some(element) = document.get_element_by_id("slskr-player-now-detail") {
            element.set_text_content(Some(&detail));
        }
        if let Some(audio) = player_audio_element(&document) {
            if audio.get_attribute("src").unwrap_or_default() != stream_url {
                if stream_url.is_empty() {
                    audio.remove_attribute("src")?;
                } else {
                    audio.set_attribute("src", &stream_url)?;
                }
            }
        }
        update_player_rating_controls(window, &document, &rating_key);
        update_player_radio_controls(&document, &radio_query);
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/transfers/speeds")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-transfers") {
            element.set_text_content(Some(&player_transfer_text(&body)));
        }
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/listening-party")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-party") {
            element.set_text_content(Some(&player_party_text(&body)));
        }
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/player/external-visualizer")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-visualizer") {
            element.set_text_content(Some(&player_visualizer_text(&body)));
        }
    }

    set_player_status(&document, "Player status updated");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn live_response_identifier(
    responses: &[EndpointBody],
    endpoint: &str,
    keys: &[&str],
) -> Option<String> {
    endpoint_array(Some(responses), endpoint)
        .into_iter()
        .find_map(|item| value_text(&item, keys).filter(|value| safe_route_segment(value)))
}

#[cfg(target_arch = "wasm32")]
fn live_endpoint_segment(
    route_path: &str,
    endpoint: ApiEndpoint,
    responses: &[EndpointBody],
) -> Option<(String, String)> {
    let route_id = normalize_route_path(route_path)
        .contains(":id")
        .then(|| route_param_value_optional(route_path))
        .flatten();

    if endpoint.path.contains(":id") {
        let id = route_id.or_else(|| {
            if endpoint.path == "/searches/:id/responses" {
                live_response_identifier(
                    responses,
                    "/searches",
                    &["id", "searchId", "search.id", "token"],
                )
                .or_else(|| {
                    live_response_identifier(
                        responses,
                        "/searches/records",
                        &["id", "searchId", "search.id", "token"],
                    )
                })
            } else if endpoint.path.starts_with("/downloads/requests/") {
                live_response_identifier(
                    responses,
                    "/downloads/requests",
                    &["id", "requestId", "request.id", "request.requestId"],
                )
            } else if endpoint.path == "/share-grants/by-collection/:id" {
                live_response_identifier(
                    responses,
                    "/collections",
                    &["id", "collectionId", "collection.id"],
                )
            } else if endpoint.path == "/collections/:id/items" {
                live_response_identifier(
                    responses,
                    "/collections",
                    &["id", "collectionId", "collection.id"],
                )
            } else if endpoint.path.starts_with("/share-grants/") {
                live_response_identifier(
                    responses,
                    "/share-grants",
                    &["id", "grantId", "shareGrantId", "grant.id"],
                )
            } else if endpoint.path.starts_with("/sharegroups/") {
                live_response_identifier(
                    responses,
                    "/sharegroups",
                    &["id", "groupId", "shareGroupId", "group.id"],
                )
            } else if endpoint.path.starts_with("/wishlist/") {
                live_response_identifier(
                    responses,
                    "/wishlist",
                    &["id", "wishlistId", "searchId", "item.id"],
                )
            } else {
                None
            }
        })?;
        return Some((":id".to_string(), id));
    }

    if endpoint.path.contains(":username") {
        let username = if endpoint.path.starts_with("/conversations/") {
            live_response_identifier(
                responses,
                "/conversations",
                &["username", "user", "peer", "name"],
            )
        } else if endpoint.path.starts_with("/users/") {
            live_response_identifier(responses, "/users", &["username", "name", "user"])
        } else {
            None
        }?;
        return Some((":username".to_string(), username));
    }

    if endpoint.path.contains(":roomName") {
        let room =
            live_response_identifier(responses, "/rooms/joined", &["name", "roomName", "room"])?;
        return Some((":roomName".to_string(), room));
    }

    None
}

#[cfg(target_arch = "wasm32")]
fn concrete_live_endpoint_path(
    route_path: &str,
    endpoint: ApiEndpoint,
    responses: &[EndpointBody],
) -> Option<String> {
    if endpoint.path == "/library/health/issues/by-type" {
        let library_path = endpoint_body(responses, "/shares")
            .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
            .and_then(|value| {
                value
                    .get("local")
                    .and_then(serde_json::Value::as_array)
                    .or_else(|| value.as_array())
                    .and_then(|entries| entries.first())
                    .and_then(|entry| {
                        ["localPath", "raw", "path", "directory"]
                            .iter()
                            .find_map(|key| entry.get(*key).map(json_scalar_preview))
                    })
            })
            .filter(|path| !path.trim().is_empty())?;
        return Some(endpoint_url(&format!(
            "/library/health/issues/by-type?libraryPath={}",
            percent_encode_query(&library_path)
        )));
    }
    let path = endpoint.path;
    if !path.contains(':') {
        return Some(endpoint_url(path));
    }

    let (placeholder, value) = live_endpoint_segment(route_path, endpoint, responses)?;
    Some(endpoint_url(&path.replace(&placeholder, &value)))
}

#[cfg(target_arch = "wasm32")]
async fn refresh_route_data(window: &web_sys::Window) -> Result<(), JsValue> {
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let Some(status) = document.get_element_by_id("slskr-route-data") else {
        return Ok(());
    };
    let summary = document.get_element_by_id("slskr-route-summary");
    let page_data = document.get_element_by_id("slskr-page-data");
    let path = window.location().pathname()?;
    let Some(page) = route_page(&path) else {
        return Ok(());
    };
    set_live_status(&document, "Refreshing live data");
    if let Some(page_data) = page_data.as_ref() {
        page_data.set_attribute("data-slskr-live-state", "pending")?;
    }
    if let Some(route_data) = document.get_element_by_id("slskr-route-data") {
        route_data.set_attribute("data-slskr-live-state", "pending")?;
    }

    let mut rendered = String::new();
    let mut responses = Vec::new();
    let mut errors = 0;
    for endpoint in route_endpoints(page.surface)
        .into_iter()
        .filter(|endpoint| endpoint.method == "GET")
    {
        // Parameterized probes are only valid after their collection endpoint
        // has supplied a real identifier.  Skipping an unavailable dependent
        // probe is preferable to sending a fabricated ID and turning an
        // empty/disconnected workspace into a stream of false errors.
        let Some(url) = concrete_live_endpoint_path(&path, endpoint, &responses) else {
            continue;
        };
        let row = match fetch_text(window, &url).await {
            Ok(body) => {
                responses.push(EndpointBody {
                    endpoint,
                    body: body.clone(),
                });
                runtime_probe_result_html(&[(endpoint.method, &url, Ok(body.as_str()))])
            }
            Err(error) => {
                errors += 1;
                let message = error
                    .as_string()
                    .unwrap_or_else(|| "request failed".to_string());
                runtime_probe_result_html(&[(endpoint.method, &url, Err(message.as_str()))])
            }
        };
        rendered.push_str(&row);
        status.set_inner_html(&rendered);
        if let Some(summary) = summary.as_ref() {
            summary.set_inner_html(&route_workflow_stats_html(
                route_kind(&path),
                Some(&responses),
            ));
        }
        if let Some(page_data) = page_data.as_ref() {
            page_data.set_inner_html(&route_workspace_result_html(&path, &responses));
            mount_workspace_tabs(&document)?;
            mount_data_cards(&document)?;
            mount_reference_actions(window, &document)?;
            mount_native_tables(&document)?;
            mount_native_subviews(&document)?;
            mount_native_actions(&document)?;
            mount_transfer_columns(&document)?;
            mount_download_policy_controls(&document)?;
            mount_native_filters(&document)?;
            mount_native_sorters(&document)?;
            mount_browser_local_panels(window, &document)?;
        }
    }
    if let Some(page_data) = page_data.as_ref() {
        page_data.set_inner_html(&route_workspace_result_html(&path, &responses));
        mount_workspace_tabs(&document)?;
        mount_data_cards(&document)?;
        mount_reference_actions(window, &document)?;
        mount_native_tables(&document)?;
        mount_native_subviews(&document)?;
        mount_native_actions(&document)?;
        mount_transfer_columns(&document)?;
        mount_download_policy_controls(&document)?;
        mount_native_filters(&document)?;
        mount_native_sorters(&document)?;
        mount_browser_local_panels(window, &document)?;
        page_data.set_attribute(
            "data-slskr-live-state",
            if errors == 0 { "ready" } else { "error" },
        )?;
    }
    if let Some(route_data) = document.get_element_by_id("slskr-route-data") {
        route_data.set_attribute(
            "data-slskr-live-state",
            if errors == 0 { "ready" } else { "error" },
        )?;
    }
    let message = if errors == 0 {
        format!("Updated {} live probes", responses.len())
    } else {
        format!("Updated {} live probes, {} errors", responses.len(), errors)
    };
    set_live_status(&document, &message);

    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text(window: &web_sys::Window, url: &str) -> Result<String, JsValue> {
    let share_response = url.ends_with("/transfers/speeds");
    let response_value =
        wasm_bindgen_futures::JsFuture::from(fetch_text_request(window, url)).await?;
    let response: web_sys::Response = response_value.dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
    }
    let response = if share_response {
        response.clone()
    } else {
        Ok(response)
    }?;
    let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    Ok(text.as_string().unwrap_or_default())
}

#[cfg(target_arch = "wasm32")]
const TRANSFER_SPEEDS_REQUEST_CACHE_TTL_MS: f64 = 200.0;

#[cfg(target_arch = "wasm32")]
std::thread_local! {
    static TRANSFER_SPEEDS_REQUEST_CACHE: RefCell<Option<(f64, js_sys::Promise)>> = const {
        RefCell::new(None)
    };
}

#[cfg(target_arch = "wasm32")]
fn fetch_text_request(window: &web_sys::Window, url: &str) -> js_sys::Promise {
    if !url.ends_with("/transfers/speeds") {
        return window.fetch_with_str(url);
    }

    let now = js_sys::Date::now();
    TRANSFER_SPEEDS_REQUEST_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((requested_at, request)) = cache.as_ref() {
            if now >= *requested_at && now - *requested_at < TRANSFER_SPEEDS_REQUEST_CACHE_TTL_MS {
                return request.clone();
            }
        }

        let request = window.fetch_with_str(url);
        *cache = Some((now, request.clone()));
        request
    })
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text_with_method(
    window: &web_sys::Window,
    url: &str,
    method: &str,
    body: Option<&str>,
) -> Result<String, JsValue> {
    fetch_text_with_method_and_headers(window, url, method, body, &[]).await
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text_with_method_and_headers(
    window: &web_sys::Window,
    url: &str,
    method: &str,
    body: Option<&str>,
    extra_headers: &[(&str, &str)],
) -> Result<String, JsValue> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    if body.is_some() || !extra_headers.is_empty() {
        let headers = web_sys::Headers::new()?;
        if body.is_some() {
            headers.set("Content-Type", "application/json")?;
        }
        for (name, value) in extra_headers {
            headers.set(name, value)?;
        }
        init.set_headers(&headers);
    }
    if let Some(body) = body {
        init.set_body(&JsValue::from_str(body));
    }
    let response_value =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init(url, &init)).await?;
    let response: web_sys::Response = response_value.dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
    }
    let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    Ok(text.as_string().unwrap_or_default())
}
