use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn rustymilk_fps_cap_ms(panel: &web_sys::Element) -> f64 {
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
pub(super) fn cycle_rustymilk_automation(document: &web_sys::Document) {
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
pub(super) fn rustymilk_select_number(
    document: &web_sys::Document,
    id: &str,
    fallback: f64,
) -> f64 {
    document
        .get_element_by_id(id)
        .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .and_then(|select| select.value().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn maybe_advance_rustymilk_automation(
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
pub(super) fn toggle_rustymilk_debug(
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
pub(super) fn import_rustymilk_texture_asset(
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
