use super::*;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RustyMilkImportedPreset {
    pub(super) source: String,
    pub(super) title: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RustyMilkPlaylist {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) preset_keys: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
pub(super) struct RustyMilkAutomationState {
    pub(super) beat_count: usize,
    pub(super) last_beat_at: f64,
    pub(super) last_preset_at: f64,
    pub(super) smoothed_energy: f64,
}

#[cfg(target_arch = "wasm32")]
impl Default for RustyMilkAutomationState {
    fn default() -> Self {
        Self {
            beat_count: 0,
            last_beat_at: 0.0,
            last_preset_at: 0.0,
            smoothed_energy: 0.0,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_IMPORTED_PRESETS_STORAGE_KEY: &str = "slskr.rustyMilkImportedPresets";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_FAVORITE_PRESETS_STORAGE_KEY: &str = "slskr.rustyMilkFavoritePresets";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_PRESET_SEARCH_STORAGE_KEY: &str = "slskr.rustyMilkPresetSearch";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_PLAYLISTS_STORAGE_KEY: &str = "slskr.rustyMilkPresetPlaylists";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY: &str =
    "slskr.rustyMilkActivePresetPlaylist";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_AUTOMATION_STORAGE_KEY: &str = "slskr.rustyMilkPresetAutomation";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_FPS_STORAGE_KEY: &str = "slskr.rustyMilkFpsCap";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_QUALITY_STORAGE_KEY: &str = "slskr.rustyMilkQuality";

#[cfg(target_arch = "wasm32")]
pub(super) const RUSTYMILK_TEXTURE_ASSETS_STORAGE_KEY: &str = "slskr.rustyMilkTextureAssets";

#[cfg(target_arch = "wasm32")]
pub(super) fn toggle_rustymilk_visualizer(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let panel = document
        .get_element_by_id("slskr-rust-rustymilk")
        .ok_or_else(|| JsValue::from_str("RustyMilk panel is missing"))?;
    if panel.has_attribute("hidden") {
        panel.remove_attribute("hidden")?;
        start_rustymilk_visualizer(window, document)?;
    } else {
        panel.set_attribute("hidden", "")?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn start_rustymilk_visualizer(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let panel = document
        .get_element_by_id("slskr-rust-rustymilk")
        .ok_or_else(|| JsValue::from_str("RustyMilk panel is missing"))?;
    if panel
        .get_attribute("data-slskr-rustymilk-running")
        .as_deref()
        == Some("true")
    {
        return Ok(());
    }
    panel.set_attribute("data-slskr-rustymilk-running", "true")?;
    let stored_search = load_rustymilk_preset_search(window);
    panel.set_attribute("data-slskr-rustymilk-search", &stored_search)?;
    panel.set_attribute(
        "data-slskr-rustymilk-playlist",
        &load_rustymilk_active_playlist(window),
    )?;
    panel.set_attribute(
        "data-slskr-rustymilk-automation",
        &load_rustymilk_simple_setting(window, RUSTYMILK_AUTOMATION_STORAGE_KEY, "off"),
    )?;
    panel.set_attribute(
        "data-slskr-rustymilk-fps",
        &load_rustymilk_simple_setting(window, RUSTYMILK_FPS_STORAGE_KEY, "full"),
    )?;
    panel.set_attribute(
        "data-slskr-rustymilk-quality",
        &load_rustymilk_simple_setting(window, RUSTYMILK_QUALITY_STORAGE_KEY, "balanced"),
    )?;
    if let Some(search_input) = document
        .get_element_by_id("slskr-rustymilk-search")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        search_input.set_value(&stored_search);
    }

    let texture_assets: Rc<RefCell<BTreeMap<String, String>>> =
        Rc::new(RefCell::new(load_rustymilk_texture_assets(window)));
    let imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>> =
        Rc::new(RefCell::new(load_rustymilk_imported_presets(window)));
    let favorite_presets: Rc<RefCell<BTreeSet<String>>> =
        Rc::new(RefCell::new(load_rustymilk_favorite_presets(window)));
    mount_rustymilk_preset_input(document, imported_presets.clone())?;
    mount_rustymilk_texture_input(document, texture_assets.clone())?;
    mount_rustymilk_pack_input(document, imported_presets.clone(), texture_assets.clone())?;
    mount_rustymilk_selects(document)?;
    mount_rustymilk_buttons(
        window,
        document,
        imported_presets.clone(),
        favorite_presets.clone(),
        texture_assets.clone(),
    )?;

    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("slskr-rustymilk-canvas")
        .ok_or_else(|| JsValue::from_str("RustyMilk canvas is missing"))?
        .dyn_into()?;
    let renderer = Rc::new(rustymilk_renderer(&canvas, texture_assets.clone())?);
    let analyzer = Rc::new(RefCell::new(
        player_audio_element(document).and_then(|audio| RustyMilkAudioAnalyzer::new(&audio).ok()),
    ));
    let input_state = Rc::new(RefCell::new(RustyMilkInputState::default()));
    mount_rustymilk_mouse_input(&canvas, input_state.clone())?;
    let runtime = Rc::new(RefCell::new(RustyMilkFrameSetRuntime::default()));
    let automation_state = Rc::new(RefCell::new(RustyMilkAutomationState::default()));
    let last_render_ms = Rc::new(RefCell::new(0.0));
    if let Some(label) = document.get_element_by_id("slskr-rustymilk-renderer") {
        label.set_text_content(Some(renderer.label()));
    }
    update_rustymilk_texture_status(document, &texture_assets);
    set_rustymilk_active_preset(
        document,
        &panel,
        &imported_presets,
        &favorite_presets,
        panel
            .get_attribute("data-slskr-rustymilk-preset-index")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0),
        "RustyMilk visualizer ready",
    );
    let animation_handle: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> =
        Rc::new(RefCell::new(None));
    let animation_handle_for_frame = animation_handle.clone();
    let window_for_frame = window.clone();
    let document_for_frame = document.clone();
    let analyzer_for_frame = analyzer.clone();
    let imports_for_frame = imported_presets.clone();
    let favorites_for_frame = favorite_presets.clone();
    let input_for_frame = input_state.clone();
    let runtime_for_frame = runtime.clone();
    let automation_for_frame = automation_state.clone();
    let last_render_for_frame = last_render_ms.clone();

    *animation_handle_for_frame.borrow_mut() = Some(Closure::wrap(Box::new(move |time_ms: f64| {
        let Some(panel) = document_for_frame.get_element_by_id("slskr-rust-rustymilk") else {
            return;
        };
        if panel.has_attribute("hidden") {
            return;
        }
        let preset_index = panel
            .get_attribute("data-slskr-rustymilk-preset-index")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let preset_source =
            rustymilk_active_preset_source(&panel, &imports_for_frame, preset_index);
        let time = time_ms / 1000.0;
        let fps_cap = rustymilk_fps_cap_ms(&panel);
        if fps_cap > 0.0 && time_ms - *last_render_for_frame.borrow() < fps_cap {
            if let Some(callback) = animation_handle.borrow().as_ref() {
                let _ = window_for_frame.request_animation_frame(callback.as_ref().unchecked_ref());
            }
            return;
        }
        *last_render_for_frame.borrow_mut() = time_ms;
        if analyzer_for_frame.borrow().is_none() {
            if let Some(audio) = player_audio_element(&document_for_frame) {
                if let Ok(next_analyzer) = RustyMilkAudioAnalyzer::new(&audio) {
                    *analyzer_for_frame.borrow_mut() = Some(next_analyzer);
                }
            }
        }
        let audio = analyzer_for_frame
            .borrow()
            .as_ref()
            .map(|analyzer| analyzer.snapshot(time))
            .unwrap_or_else(|| RustyMilkAudioSnapshot::synthetic(time));
        maybe_advance_rustymilk_automation(
            &document_for_frame,
            &panel,
            &imports_for_frame,
            &favorites_for_frame,
            &mut automation_for_frame.borrow_mut(),
            time,
            audio.bands.bass + audio.bands.mid + audio.bands.treble,
        );
        let frame_set = runtime_for_frame
            .borrow_mut()
            .render_source_with_audio_and_input(
                &preset_source,
                time,
                audio.bands.bass,
                audio.bands.mid,
                audio.bands.treble,
                &audio.waveform,
                &audio.spectrum,
                *input_for_frame.borrow(),
            );
        renderer.render_frame_set(&frame_set, time);
        if let Some(status) = document_for_frame.get_element_by_id("slskr-rustymilk-status") {
            let shape_count = frame_set
                .entries
                .iter()
                .map(|entry| entry.frame.shape_count)
                .sum::<usize>();
            let waveform_count = frame_set
                .entries
                .iter()
                .map(|entry| entry.frame.waveform_count)
                .sum::<usize>();
            status.set_text_content(Some(&format!(
                "RustyMilk running: {} bass {:.0}% mid {:.0}% treble {:.0}% / {} preset{} / {} shapes / {} waves",
                audio.source,
                audio.bands.bass * 100.0,
                audio.bands.mid * 100.0,
                audio.bands.treble * 100.0,
                frame_set.preset_count,
                if frame_set.preset_count == 1 { "" } else { "s" },
                shape_count,
                waveform_count
            )));
        }
        update_rustymilk_library_controls(
            &document_for_frame,
            &panel,
            &imports_for_frame,
            &favorites_for_frame,
            preset_index,
        );
        if let Some(callback) = animation_handle.borrow().as_ref() {
            let _ = window_for_frame.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut(f64)>));

    if let Some(callback) = animation_handle_for_frame.borrow().as_ref() {
        window.request_animation_frame(callback.as_ref().unchecked_ref())?;
    }
    Ok(())
}
