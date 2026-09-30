use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn save_rustymilk_playlist(
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
pub(super) fn cycle_rustymilk_playlist(
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
pub(super) fn rename_rustymilk_playlist(window: &web_sys::Window, document: &web_sys::Document) {
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
pub(super) fn clear_rustymilk_playlist_filter(
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
pub(super) fn remove_rustymilk_playlist(
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
