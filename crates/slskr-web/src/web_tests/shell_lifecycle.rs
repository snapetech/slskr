use super::*;

#[test]
fn shell_contains_primary_routes() {
    let html = shell_html();
    for item in nav_items() {
        assert!(html.contains(item.label), "missing {}", item.label);
    }
    assert!(html.contains("Search, transfers, messages"));
    assert!(html.contains("slskr-player"));
    assert!(html.contains("data-slskr-player"));
    assert!(html.contains("slskr-player-audio"));
    assert!(html.contains("data-slskr-player-action=\"play\""));
    assert!(html.contains("data-slskr-player-action=\"refresh\""));
    assert!(html.contains("data-slskr-player-action=\"clear\""));
    assert!(html.contains("data-slskr-player-action=\"visualizer\""));
    assert!(html.contains("data-slskr-player-action=\"radio\""));
    assert!(html.contains("data-slskr-player-radio-query"));
    assert!(html.contains("data-slskr-player-rating=\"5\""));
    assert!(html.contains("slskr-player-rating-status"));
    assert!(html.contains("slskr-player-radio"));
    assert!(html.contains("Searches"));
    assert!(html.contains("Search Detail"));
    assert!(html.contains("data-slskr-route-kind=\"Search\""));
    assert!(html.contains("slskr-player-now"));
    assert!(html.contains("slskr-player-transfers"));
    assert!(html.contains("slskr-rust-rustymilk"));
    assert!(html.contains("data-slskr-rustymilk-search=\"\""));
    assert!(html.contains("data-slskr-rustymilk-playlist=\"\""));
    assert!(html.contains("data-slskr-rustymilk-automation=\"off\""));
    assert!(html.contains("data-slskr-rustymilk-fps=\"full\""));
    assert!(html.contains("data-slskr-rustymilk-quality=\"balanced\""));
    assert!(html.contains("slskr-rustymilk-canvas"));
    assert!(html.contains("Search RustyMilk presets"));
    assert!(html.contains("data-slskr-rustymilk-action=\"previous\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"preset\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"random\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"favorite\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"favorites\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"remove\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"search\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"clear-search\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"save-playlist\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"playlist\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"rename-playlist\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"clear-playlist\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"remove-playlist\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"import\""));
    assert!(html.contains("slskr-rustymilk-preset-input"));
    assert!(html.contains("accept=\".milk,.milk2,.txt,text/plain\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"pack\""));
    assert!(html.contains("slskr-rustymilk-pack-input"));
    assert!(html.contains("webkitdirectory hidden"));
    assert!(html.contains("data-slskr-rustymilk-action=\"texture\""));
    assert!(html.contains("slskr-rustymilk-texture-input"));
    assert!(html.contains("multiple hidden"));
    assert!(html.contains("slskr-rustymilk-textures"));
    assert!(html.contains("data-slskr-rustymilk-action=\"clear\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"reset\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"apply-parameter\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"randomize-parameters\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"import-shape\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"export-shape\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"remove-shape\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"import-wave\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"export-wave\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"remove-wave\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"export-preset\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"automation\""));
    assert!(html.contains("data-slskr-rustymilk-action=\"debug\""));
    assert!(html.contains("slskr-rustymilk-debug"));
    assert!(html.contains("slskr-rustymilk-renderer"));
    assert!(html.contains("/api/v0/searches"));
    assert!(html.contains("slskr-runtime-status"));
    assert!(html.contains("/api/v0/health"));
    assert!(html.contains("slskr-route-view"));
    let system = route_page_html("/system");
    assert!(system.contains("System"));
    assert!(system.contains("Rescan Shares"));
}

#[test]
fn static_index_supports_direct_nested_route_loads() {
    assert!(STATIC_INDEX.contains("href=\"/styles.css\""));
    assert!(STATIC_INDEX.contains("src=\"/slskr_web_bootstrap.js\""));
    assert!(!STATIC_INDEX.contains("href=\"./styles.css\""));
    assert!(!STATIC_INDEX.contains("src=\"./slskr_web_bootstrap.js\""));
}

#[test]
fn active_route_refresh_contract_keeps_hidden_panes_lazy() {
    let shell = shell_html();
    assert_eq!(
        shell
            .matches("data-slskr-refresh-scope=\"active-route\"")
            .count(),
        1,
        "the shell should render one active route instead of pre-rendering every route pane"
    );
    assert_eq!(
        shell.matches("id=\"slskr-route-data\"").count(),
        1,
        "only the active route should own live probe status"
    );
    assert_eq!(
        shell.matches("id=\"slskr-page-data\"").count(),
        1,
        "only the active route should own live workspace data"
    );
    assert!(shell.contains("data-slskr-lazy-workspace=\"true\""));
    assert!(shell.contains("data-slskr-lazy-diagnostics=\"true\""));
    assert!(shell.contains("id=\"slskr-rust-rustymilk\" hidden"));
    assert!(shell.contains("data-slskr-rustymilk-running=\"false\""));
    assert!(
        !shell.contains("setInterval("),
        "initial shell markup should not register polling loops for hidden panes"
    );

    for path in ["/searches", "/downloads", "/messages", "/browse", "/system"] {
        let page = route_page_html(path);
        assert!(
            page.contains("data-slskr-live-state=\"pending\""),
            "{path} should render pending live data until the active route refresh runs"
        );
    }
}

#[test]
fn shell_prioritizes_functional_webui_over_migration_inventory() {
    let html = shell_html();
    assert!(html.contains("slskr-appbar"));
    assert!(html.contains("Now Playing"));
    assert!(html.contains("Queue idle"));
    assert!(html.contains("slskr-page-data"));
    assert!(!html.contains("Rust web migration target"));
    assert!(!html.contains("Rust/WASM"));
    assert!(!html.contains("Bulk Endpoint Workbench"));
}

#[test]
fn native_shell_contains_in_app_confirmation_modal_styles() {
    let html = route_page_html("/downloads");
    assert!(html.contains("Cancel"));
    assert!(html.contains("Remove"));
    let manifest: Vec<String> =
        serde_json::from_str(include_str!("../../static/styles.manifest.json")).unwrap();
    let static_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
    let css: String = manifest
        .iter()
        .map(|owner| std::fs::read_to_string(static_root.join(owner)).unwrap())
        .collect();
    for value in [
        "slskr-modal-backdrop",
        "slskr-modal",
        "data-slskr-confirm-run",
    ] {
        assert!(
            css.contains(value),
            "confirmation modal CSS should contain {value}"
        );
    }
}
