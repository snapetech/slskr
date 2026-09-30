use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn store_rustymilk_edited_source(
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
pub(super) fn active_rustymilk_parsed(
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
pub(super) fn apply_rustymilk_parameter(
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
pub(super) fn randomize_rustymilk_parameters(
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
pub(super) fn import_rustymilk_fragment(
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
pub(super) fn export_rustymilk_fragment(
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
pub(super) fn remove_rustymilk_fragment(
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
pub(super) fn export_rustymilk_preset(
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
pub(super) fn download_rustymilk_text(document: &web_sys::Document, file_name: &str, text: &str) {
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
