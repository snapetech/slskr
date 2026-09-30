use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn load_rustymilk_imported_presets(
    window: &web_sys::Window,
) -> Vec<RustyMilkImportedPreset> {
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
pub(super) fn persist_rustymilk_imported_presets(
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
pub(super) fn load_rustymilk_favorite_presets(window: &web_sys::Window) -> BTreeSet<String> {
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
pub(super) fn persist_rustymilk_favorite_presets(
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
pub(super) fn load_rustymilk_preset_search(window: &web_sys::Window) -> String {
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
pub(super) fn load_rustymilk_simple_setting(
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
pub(super) fn persist_rustymilk_simple_setting(
    document: &web_sys::Document,
    storage_key: &str,
    value: &str,
) {
    let Some(window) = document.default_view() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let _ = storage.set_item(storage_key, value);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn load_rustymilk_active_playlist(window: &web_sys::Window) -> String {
    load_rustymilk_simple_setting(window, RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY, "")
}

#[cfg(target_arch = "wasm32")]
pub(super) fn load_rustymilk_playlists(window: &web_sys::Window) -> Vec<RustyMilkPlaylist> {
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
pub(super) fn load_rustymilk_playlists_from_document(
    panel: &web_sys::Element,
) -> Vec<RustyMilkPlaylist> {
    panel
        .owner_document()
        .and_then(|document| document.default_view())
        .map(|window| load_rustymilk_playlists(&window))
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn persist_rustymilk_playlists(
    document: &web_sys::Document,
    playlists: &[RustyMilkPlaylist],
) {
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
pub(super) fn persist_rustymilk_preset_search(document: &web_sys::Document, query: &str) {
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
pub(super) fn load_rustymilk_texture_assets(window: &web_sys::Window) -> BTreeMap<String, String> {
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
pub(super) fn persist_rustymilk_texture_assets(
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
