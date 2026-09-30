use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn import_rustymilk_preset(
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
pub(super) fn reset_rustymilk_import(
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
pub(super) fn cycle_rustymilk_preset(
    document: &web_sys::Document,
    imported_presets: &Rc<RefCell<Vec<RustyMilkImportedPreset>>>,
    favorite_presets: &Rc<RefCell<BTreeSet<String>>>,
) {
    cycle_rustymilk_preset_by(document, imported_presets, favorite_presets, 1);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn cycle_rustymilk_preset_by(
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
pub(super) fn search_rustymilk_preset(
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
pub(super) fn clear_rustymilk_preset_search(
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
pub(super) fn toggle_rustymilk_favorite(
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
pub(super) fn toggle_rustymilk_favorites_only(
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
pub(super) fn random_rustymilk_preset(
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
pub(super) fn remove_rustymilk_active_preset(
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
pub(super) fn clear_rustymilk_library(
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
