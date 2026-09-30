//! Native Web route shell runtime.

use super::*;

pub(super) fn player_footer_html() -> String {
    r#"<footer class="slskr-player" data-slskr-player data-slskr-player-rating-key="" data-slskr-player-radio-query=""><section><strong>Now Playing</strong><span id="slskr-player-now">Queue idle</span><span id="slskr-player-now-detail">No local stream selected</span><audio id="slskr-player-audio" preload="metadata" controls></audio></section><section class="slskr-player-controls" aria-label="Player controls"><button type="button" data-slskr-player-action="play">Play</button><button type="button" data-slskr-player-action="refresh">Refresh</button><button type="button" data-slskr-player-action="clear">Clear</button><button type="button" data-slskr-player-action="visualizer">Visualizer</button><button type="button" data-slskr-player-action="radio">Radio</button></section><section class="slskr-player-rating" aria-label="Player rating"><strong>Rating</strong><div id="slskr-player-rating-controls"><button type="button" data-slskr-player-rating="1">1</button><button type="button" data-slskr-player-rating="2">2</button><button type="button" data-slskr-player-rating="3">3</button><button type="button" data-slskr-player-rating="4">4</button><button type="button" data-slskr-player-rating="5">5</button></div><span id="slskr-player-rating-status">Not rated</span></section><section><strong>Radio</strong><span id="slskr-player-radio">No track selected</span><span id="slskr-player-transfers">0 down / 0 up</span></section><section><strong>Visualizer</strong><span id="slskr-player-visualizer">Checking status</span><span id="slskr-player-status" aria-live="polite">Rust player surface ready</span></section></footer>"#.to_string()
}

pub(super) fn rustymilk_panel_html() -> String {
    r#"<section class="slskr-rustymilk-panel" id="slskr-rust-rustymilk" hidden data-slskr-rustymilk-running="false" data-slskr-rustymilk-favorites-only="false" data-slskr-rustymilk-search="" data-slskr-rustymilk-playlist="" data-slskr-rustymilk-automation="off" data-slskr-rustymilk-fps="full" data-slskr-rustymilk-quality="balanced" data-slskr-rustymilk-debug="false"><header><div><strong>RustyMilk</strong><span id="slskr-rustymilk-preset">slskr native grid smoke</span></div><div class="slskr-rustymilk-actions"><button type="button" data-slskr-rustymilk-action="previous">Previous</button><button type="button" data-slskr-rustymilk-action="preset">Next</button><button type="button" data-slskr-rustymilk-action="random">Random</button><button type="button" data-slskr-rustymilk-action="favorite" id="slskr-rustymilk-favorite">Favorite</button><button type="button" data-slskr-rustymilk-action="favorites" id="slskr-rustymilk-favorites-only">Favorites</button><button type="button" data-slskr-rustymilk-action="remove">Remove</button><button type="button" data-slskr-rustymilk-action="import">Import</button><button type="button" data-slskr-rustymilk-action="texture">Texture</button><button type="button" data-slskr-rustymilk-action="pack">Pack</button><button type="button" data-slskr-rustymilk-action="clear">Clear</button><button type="button" data-slskr-rustymilk-action="reset">Reset</button><button type="button" data-slskr-rustymilk-action="external">External</button><button type="button" data-slskr-rustymilk-action="close">Close</button></div></header><div class="slskr-rustymilk-library"><input id="slskr-rustymilk-search" aria-label="Search RustyMilk presets" placeholder="Search preset library"><button type="button" data-slskr-rustymilk-action="search">Search</button><button type="button" data-slskr-rustymilk-action="clear-search">Clear search</button><button type="button" data-slskr-rustymilk-action="save-playlist">Save playlist</button><button type="button" data-slskr-rustymilk-action="playlist">Playlist</button><button type="button" data-slskr-rustymilk-action="rename-playlist">Rename</button><button type="button" data-slskr-rustymilk-action="clear-playlist">All presets</button><button type="button" data-slskr-rustymilk-action="remove-playlist">Remove playlist</button><input id="slskr-rustymilk-preset-input" type="file" accept=".milk,.milk2,.txt,text/plain" multiple hidden><input id="slskr-rustymilk-texture-input" type="file" accept="image/png,image/jpeg,image/webp,image/gif" multiple hidden><input id="slskr-rustymilk-pack-input" type="file" accept=".milk,.milk2,.txt,text/plain,image/png,image/jpeg,image/webp,image/gif" multiple webkitdirectory hidden><span id="slskr-rustymilk-library-status">3 bundled presets</span><span id="slskr-rustymilk-textures">0 texture assets</span></div><div class="slskr-rustymilk-editor"><select id="slskr-rustymilk-parameter" aria-label="RustyMilk parameter"><option value="decay">Decay</option><option value="zoom">Zoom</option><option value="rot">Rotation</option><option value="wave_r">Wave red</option><option value="wave_g">Wave green</option><option value="wave_b">Wave blue</option><option value="wave_a">Wave alpha</option></select><input id="slskr-rustymilk-parameter-value" aria-label="RustyMilk parameter value" value="0.9"><button type="button" data-slskr-rustymilk-action="apply-parameter">Apply</button><button type="button" data-slskr-rustymilk-action="randomize-parameters">Randomize</button><button type="button" data-slskr-rustymilk-action="import-shape">Import shape</button><button type="button" data-slskr-rustymilk-action="export-shape">Export shape</button><button type="button" data-slskr-rustymilk-action="remove-shape">Remove shape</button><button type="button" data-slskr-rustymilk-action="import-wave">Import wave</button><button type="button" data-slskr-rustymilk-action="export-wave">Export wave</button><button type="button" data-slskr-rustymilk-action="remove-wave">Remove wave</button><button type="button" data-slskr-rustymilk-action="export-preset">Export preset</button><button type="button" data-slskr-rustymilk-action="automation">Automation</button><select id="slskr-rustymilk-automation-beats" aria-label="RustyMilk automation beats"><option value="4">4 beats</option><option value="8" selected>8 beats</option><option value="16">16 beats</option></select><select id="slskr-rustymilk-automation-interval" aria-label="RustyMilk automation interval"><option value="15">15 sec</option><option value="30" selected>30 sec</option><option value="60">60 sec</option></select><select id="slskr-rustymilk-fps" aria-label="RustyMilk FPS cap"><option value="full">Full FPS</option><option value="60">60 FPS</option><option value="30">30 FPS</option><option value="24">24 FPS</option></select><select id="slskr-rustymilk-quality" aria-label="RustyMilk quality"><option value="balanced" selected>Balanced</option><option value="efficient">Efficient</option><option value="full">Full</option><option value="custom">Custom</option></select><button type="button" data-slskr-rustymilk-action="debug">Debug</button></div><canvas id="slskr-rustymilk-canvas" width="960" height="360" aria-label="RustyMilk visualizer"></canvas><footer><span id="slskr-rustymilk-status">Visualizer ready</span><span id="slskr-rustymilk-renderer">Renderer checking</span></footer><pre id="slskr-rustymilk-debug" hidden></pre></section>"#.to_string()
}

pub fn shell_html() -> String {
    let nav = nav_items()
        .iter()
        .map(|item| {
            format!(
                r#"<a class="slskr-nav-item" href="{href}" title="{label}"><span class="slskr-nav-icon">{icon}</span><span>{label}</span></a>"#,
                href = item.href,
                icon = item.icon,
                label = item.label
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="slskr-shell"><nav class="slskr-nav">{nav}</nav><main class="slskr-main"><header class="slskr-appbar"><div><strong>slskr</strong><span>Search, transfers, messages, rooms, browse, sharing, and system control</span><span class="slskr-appbar-support"><b>Keep the node moving</b><a href="https://www.paypal.com/donate/?business=donations%40snape.tech" target="_blank" rel="noopener noreferrer">PayPal</a><a href="https://ko-fi.com/snapetech" target="_blank" rel="noopener noreferrer">Ko-fi</a></span></div><ul id="slskr-runtime-status">{runtime}</ul></header><section id="slskr-route-view">{route_page}</section></main>{rustymilk}{player}</div>"#,
        route_page = route_page_html("/searches"),
        runtime = runtime_probe_pending_html(),
        rustymilk = rustymilk_panel_html(),
        player = player_footer_html(),
    )
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let root = document
        .get_element_by_id("root")
        .ok_or_else(|| JsValue::from_str("#root is missing"))?;
    root.set_inner_html(&shell_html());
    mount_router(&window, &document)?;
    mount_global_shortcuts(&window, &document)?;
    mount_player_controls(&window, &document)?;
    wasm_bindgen_futures::spawn_local(async {
        let _ = refresh_runtime_status().await;
    });
    let window_for_player = window.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = refresh_player_status(&window_for_player).await;
    });
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    Ok(())
}

#[wasm_bindgen(js_name = renderShellHtml)]
pub fn render_shell_html() -> String {
    shell_html()
}

#[wasm_bindgen(js_name = compatibilityReport)]
pub fn wasm_compatibility_report() -> String {
    compatibility_report()
}

#[wasm_bindgen(js_name = renderRuntimeProbePendingHtml)]
pub fn wasm_runtime_probe_pending_html() -> String {
    runtime_probe_pending_html()
}

#[wasm_bindgen(js_name = renderRoutePageHtml)]
pub fn wasm_route_page_html(path: &str) -> String {
    route_page_html(path)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_router(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    render_current_route(window, document)?;

    for item in nav_items() {
        let selector = format!(r#".slskr-nav-item[href="{}"]"#, item.href);
        let Some(element) = document.query_selector(&selector)? else {
            continue;
        };
        let href = item.href.to_owned();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                if let Ok(history) = window.history() {
                    let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&href));
                }
                let _ = render_current_route(&window, &document);
            },
        ));
        element.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let window_for_pop = window.clone();
    let document_for_pop = document.clone();
    let popstate = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
        let _ = render_current_route(&window_for_pop, &document_for_pop);
    }));
    window.add_event_listener_with_callback("popstate", popstate.as_ref().unchecked_ref())?;
    popstate.forget();

    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn render_current_route(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let path = window.location().pathname()?;
    if let Some(view) = document.get_element_by_id("slskr-route-view") {
        view.set_inner_html(&route_page_html(&path));
    }
    mount_route_actions(window, document)?;
    mount_toolbar_actions(window, document)?;
    mount_reference_actions(window, document)?;
    mount_workspace_tabs(document)?;
    mount_data_cards(document)?;
    mount_native_tables(document)?;
    mount_native_subviews(document)?;
    mount_native_actions(document)?;
    mount_transfer_columns(document)?;
    mount_download_policy_controls(document)?;
    mount_native_filters(document)?;
    mount_native_sorters(document)?;
    mount_live_controls(window, document)?;
    mount_browser_local_panels(window, document)?;
    for item in nav_items() {
        let selector = format!(r#".slskr-nav-item[href="{}"]"#, item.href);
        let Some(element) = document.query_selector(&selector)? else {
            continue;
        };
        let active = normalize_route_path(&path) == normalize_route_path(item.href);
        if active {
            element.set_attribute("aria-current", "page")?;
        } else {
            element.remove_attribute("aria-current")?;
        }
    }
    let window_for_data = window.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = refresh_route_data(&window_for_data).await;
    });
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_data_cards(document: &web_sys::Document) -> Result<(), JsValue> {
    let cards = document.query_selector_all("[data-slskr-data-card]")?;
    for card_index in 0..cards.length() {
        let Some(node) = cards.item(card_index) else {
            continue;
        };
        let card: web_sys::Element = node.dyn_into()?;

        if let Some(filter) = card.query_selector(".slskr-card-filter")? {
            let input: web_sys::HtmlInputElement = filter.dyn_into()?;
            let card_for_filter = card.clone();
            let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(
                move |event: web_sys::Event| {
                    let term = event
                        .current_target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value().to_lowercase())
                        .unwrap_or_default();
                    let Ok(rows) = card_for_filter.query_selector_all("[data-slskr-row-text]")
                    else {
                        return;
                    };
                    for row_index in 0..rows.length() {
                        let Some(row) = rows.item(row_index) else {
                            continue;
                        };
                        let Ok(row) = row.dyn_into::<web_sys::Element>() else {
                            continue;
                        };
                        let matches = term.is_empty()
                            || row
                                .get_attribute("data-slskr-row-text")
                                .is_some_and(|value| value.contains(&term));
                        if matches {
                            let _ = row.remove_attribute("hidden");
                        } else {
                            let _ = row.set_attribute("hidden", "");
                        }
                    }
                    update_data_card_count(&card_for_filter);
                },
            ));
            input.add_event_listener_with_callback("input", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        if let Some(clear) = card.query_selector("[data-slskr-card-clear]")? {
            let card_for_clear = card.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    if let Ok(Some(filter)) = card_for_clear.query_selector(".slskr-card-filter") {
                        if let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() {
                            input.set_value("");
                        }
                    }
                    if let Ok(rows) = card_for_clear.query_selector_all("[data-slskr-row-text]") {
                        for row_index in 0..rows.length() {
                            let Some(row) = rows.item(row_index) else {
                                continue;
                            };
                            let Ok(row) = row.dyn_into::<web_sys::Element>() else {
                                continue;
                            };
                            let _ = row.remove_attribute("hidden");
                        }
                    }
                    update_data_card_count(&card_for_clear);
                },
            ));
            clear.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let sort_buttons = card.query_selector_all("[data-slskr-sort-index]")?;
        for button_index in 0..sort_buttons.length() {
            let Some(node) = sort_buttons.item(button_index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let card_for_sort = card.clone();
            let button_for_sort = button.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    sort_data_card_table(&card_for_sort, &button_for_sort);
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let view_buttons = card.query_selector_all("[data-slskr-card-view]")?;
        for button_index in 0..view_buttons.length() {
            let Some(node) = view_buttons.item(button_index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let card_for_view = card.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let Some(target) = event
                        .current_target()
                        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    else {
                        return;
                    };
                    let view = target
                        .get_attribute("data-slskr-card-view")
                        .unwrap_or_else(|| "list".to_string());
                    let _ = card_for_view.set_attribute("data-slskr-view", &view);
                    if let Ok(buttons) = card_for_view.query_selector_all("[data-slskr-card-view]")
                    {
                        for index in 0..buttons.length() {
                            let Some(node) = buttons.item(index) else {
                                continue;
                            };
                            let Ok(button) = node.dyn_into::<web_sys::Element>() else {
                                continue;
                            };
                            let active = button
                                .get_attribute("data-slskr-card-view")
                                .is_some_and(|button_view| button_view == view);
                            let class = if active { "is-active" } else { "" };
                            let _ = button.set_attribute("class", class);
                        }
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let rows = card.query_selector_all("[data-slskr-record-select]")?;
        for row_index in 0..rows.length() {
            let Some(node) = rows.item(row_index) else {
                continue;
            };
            let row: web_sys::Element = node.dyn_into()?;
            let card_for_click = card.clone();
            let row_for_click = row.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |_event: web_sys::MouseEvent| {
                    select_data_card_record(&card_for_click, &row_for_click);
                },
            ));
            row.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();

            let card_for_key = card.clone();
            let row_for_key = row.clone();
            let callback = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
                move |event: web_sys::KeyboardEvent| {
                    let key = event.key();
                    if key == "Enter" || key == " " {
                        event.prevent_default();
                        select_data_card_record(&card_for_key, &row_for_key);
                    }
                },
            ));
            row.add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn mount_native_tables(document: &web_sys::Document) -> Result<(), JsValue> {
    let rows = document.query_selector_all("[data-slskr-native-select]")?;
    for row_index in 0..rows.length() {
        let Some(node) = rows.item(row_index) else {
            continue;
        };
        let row: web_sys::Element = node.dyn_into()?;

        let document_for_click = document.clone();
        let row_for_click = row.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                select_native_row(&document_for_click, &row_for_click);
            },
        ));
        row.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();

        let document_for_key = document.clone();
        let row_for_key = row.clone();
        let callback = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
            move |event: web_sys::KeyboardEvent| {
                let key = event.key();
                match key.as_str() {
                    "Enter" | " " => {
                        event.prevent_default();
                        select_native_row(&document_for_key, &row_for_key);
                    }
                    "ArrowDown" => {
                        event.prevent_default();
                        focus_relative_native_row(&document_for_key, &row_for_key, 1);
                    }
                    "ArrowUp" => {
                        event.prevent_default();
                        focus_relative_native_row(&document_for_key, &row_for_key, -1);
                    }
                    "Home" => {
                        event.prevent_default();
                        focus_edge_native_row(&document_for_key, &row_for_key, true);
                    }
                    "End" => {
                        event.prevent_default();
                        focus_edge_native_row(&document_for_key, &row_for_key, false);
                    }
                    _ => {}
                }
            },
        ));
        row.add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}
