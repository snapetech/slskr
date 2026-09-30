use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn rustymilk_preset_count(
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
pub(super) fn rustymilk_active_preset_source(
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
pub(super) fn rustymilk_active_preset_name(
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
pub(super) fn rustymilk_preset_favorite_key(
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
pub(super) fn rustymilk_favorites_only(panel: &web_sys::Element) -> bool {
    panel
        .get_attribute("data-slskr-rustymilk-favorites-only")
        .as_deref()
        == Some("true")
}

#[cfg(target_arch = "wasm32")]
pub(super) fn rustymilk_active_playlist(panel: &web_sys::Element) -> String {
    panel
        .get_attribute("data-slskr-rustymilk-playlist")
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn rustymilk_preset_search_query(panel: &web_sys::Element) -> String {
    panel
        .get_attribute("data-slskr-rustymilk-search")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn rustymilk_visible_preset_indices(
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
pub(super) fn rustymilk_empty_visible_preset_message(panel: &web_sys::Element) -> &'static str {
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
pub(super) fn update_rustymilk_library_controls(
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
pub(super) fn set_rustymilk_active_preset(
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
