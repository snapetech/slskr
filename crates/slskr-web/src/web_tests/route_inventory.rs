use super::*;

#[test]
fn api_endpoints_are_versioned() {
    for section in app_sections() {
        assert!(endpoint_url(section.endpoint).starts_with("/api/v0/"));
    }
}

#[test]
fn runtime_probes_cover_public_and_session_status() {
    let paths = runtime_probes()
        .iter()
        .map(|probe| probe.path)
        .collect::<Vec<_>>();
    for expected in ["/health", "/version", "/application", "/server"] {
        assert!(
            paths.contains(&expected),
            "missing runtime probe {expected}"
        );
    }
}

#[test]
fn route_normalization_handles_dynamic_routes() {
    assert_eq!(normalize_route_path("/"), "/searches");
    assert_eq!(normalize_route_path("/searches/42"), "/searches/:id");
    assert_eq!(normalize_route_path("/system/network"), "/system/:tab");
    assert_eq!(normalize_route_path("/pods/abc"), "/pods/:podId");
    assert_eq!(
        normalize_route_path("/pods/abc/channels/general"),
        "/pods/:podId/channels/:channelId"
    );
}

#[test]
fn rust_ui_parity_ledger_tracks_closure_instead_of_stale_gaps() {
    let ledger = include_str!("../../../../docs/rust-ui-parity-ledger.md");
    assert!(ledger.contains("Estimated completion: 95-98%."));
    assert!(ledger.contains("## Route Closure"));
    assert!(ledger.contains("live-backend behavioral validation"));
    assert!(!ledger.contains("Estimated completion: 55-65%."));
    assert!(!ledger.contains("Remaining 1:1 Gaps"));
    assert!(!ledger.contains("| Route | Current Rust Coverage | Remaining 1:1 Gaps |"));
}

#[test]
fn route_probe_urls_use_concrete_paths() {
    let endpoint = ApiEndpoint {
        method: "GET",
        path: "/searches/:id/responses",
        surface: "search",
    };
    assert_eq!(
        concrete_endpoint_path("/searches/42", endpoint),
        "/api/v0/searches/42/responses"
    );
    assert_eq!(
        concrete_endpoint_path("/searches/<script>", endpoint),
        "/api/v0/searches/1/responses"
    );
    let pending = route_probe_pending_html("/messages");
    assert!(pending.contains("/api/v0/conversations"));
    assert!(pending.contains("/api/v0/conversations/peer1"));
}

#[test]
fn bulk_surface_matrix_covers_every_route_group() {
    let matrix = surface_matrix_html();
    for surface in [
        "browse",
        "collections",
        "identity",
        "integrations",
        "messages",
        "rooms",
        "search",
        "system",
        "transfers",
        "wishlist",
    ] {
        assert!(matrix.contains(surface), "missing surface {surface}");
        assert!(surface_route_count(surface) > 0, "no routes for {surface}");
        assert!(
            !route_endpoints(surface).is_empty(),
            "no endpoints for {surface}"
        );
    }
    assert!(surface_actions("collections").len() >= 3);
    assert!(surface_actions("integrations").len() >= 4);
    assert!(surface_actions("identity").len() >= 3);
    assert!(surface_actions("wishlist").len() >= 2);
}

#[test]
fn bulk_workbench_renders_all_surface_catalogs() {
    let html = bulk_workbench_html();
    for surface in surface_names() {
        assert!(
            html.contains(&format!(r#"data-slskr-surface="{surface}""#)),
            "missing workbench surface {surface}"
        );
    }
    for expected in [
        "/api/v0/share-grants",
        "/api/v0/musicbrainz/targets",
        "/api/v0/telemetry/metrics/kpis",
        "/api/v0/source-feeds",
        "/api/v0/soulseek/interests",
        "/api/v0/database/vacuum",
    ] {
        assert!(
            html.contains(expected),
            "missing workbench endpoint {expected}"
        );
    }
    assert!(html.contains("ShareGrant"));
    assert!(html.contains("ShareGroupMember"));
    assert!(html.contains("Permissions"));
}

#[test]
fn every_route_surface_has_bulk_catalog_entries() {
    for surface in surface_names() {
        assert!(
            !surface_route_catalog_html(surface).is_empty(),
            "missing routes for {surface}"
        );
        assert!(
            !surface_endpoint_catalog_html(surface).is_empty(),
            "missing endpoints for {surface}"
        );
    }
}

#[test]
fn rust_route_inventory_matches_current_react_route_surface() {
    let route_paths = ui_routes()
        .iter()
        .map(|route| route.path)
        .collect::<Vec<_>>();
    for expected in [
        "/searches",
        "/searches/:id",
        "/discovery-graph",
        "/playlist-intake",
        "/wishlist",
        "/browse",
        "/users",
        "/contacts",
        "/solid",
        "/collections",
        "/sharegroups",
        "/shared",
        "/chat",
        "/pods",
        "/rooms",
        "/messages",
        "/uploads",
        "/downloads",
        "/system",
        "/system/:tab",
    ] {
        assert!(route_paths.contains(&expected), "missing route {expected}");
        assert!(
            REACT_ROUTES.contains(&format!("path=\"{expected}\""))
                || REACT_ROUTES.contains(&format!("to=\"{expected}\"")),
            "route {expected} is no longer present in the React UI"
        );
    }
}

#[test]
fn rust_nav_inventory_matches_current_react_navigation() {
    let labels = nav_items()
        .iter()
        .map(|item| item.label)
        .collect::<Vec<_>>();
    for expected in [
        "Search",
        "Discovery Graph",
        "Playlist Intake",
        "Wishlist",
        "Downloads",
        "Uploads",
        "Messages",
        "Users",
        "Contacts",
        "Solid",
        "Collections",
        "Share Groups",
        "Shared with Me",
        "Browse",
        "System",
    ] {
        assert!(labels.contains(&expected), "missing nav item {expected}");
    }
    for item in nav_items() {
        assert!(
            REACT_NAV.contains(&format!("to=\"{}\"", item.href))
                || REACT_HEADER.contains(&format!("to=\"{}\"", item.href)),
            "nav item {} does not match a React NavLink",
            item.href
        );
    }
}

#[test]
fn api_contract_inventory_covers_core_old_ui_surfaces() {
    let surfaces = api_endpoints()
        .iter()
        .map(|endpoint| endpoint.surface)
        .collect::<Vec<_>>();
    for expected in [
        "application",
        "session",
        "search",
        "wishlist",
        "transfers",
        "rooms",
        "messages",
        "browse",
        "identity",
        "collections",
        "integrations",
        "system",
    ] {
        assert!(
            surfaces.contains(&expected),
            "missing API surface {expected}"
        );
    }
}
