use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn rustymilk_import_file_name(file: &web_sys::File) -> String {
    js_sys::Reflect::get(file.as_ref(), &JsValue::from_str("webkitRelativePath"))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| file.name())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn is_rustymilk_preset_file_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".milk") || lower.ends_with(".milk2") || lower.ends_with(".txt")
}

#[cfg(target_arch = "wasm32")]
pub(super) fn is_rustymilk_texture_file_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".gif"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_rustymilk_preset_input(
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
pub(super) fn store_rustymilk_imported_preset(
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
pub(super) fn read_rustymilk_preset_file(
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
pub(super) fn read_rustymilk_texture_file(
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
pub(super) fn mount_rustymilk_pack_input(
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
pub(super) fn mount_rustymilk_texture_input(
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
pub(super) fn store_rustymilk_texture_asset(
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
pub(super) fn update_rustymilk_texture_status(
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
