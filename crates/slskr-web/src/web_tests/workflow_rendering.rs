use super::*;

#[test]
fn rust_route_pages_cover_current_route_inventory() {
    let pages = route_pages()
        .iter()
        .map(|page| page.path)
        .collect::<Vec<_>>();
    for route in ui_routes() {
        if route.path == "/" {
            continue;
        }
        assert!(
            pages.contains(&route.path),
            "missing route page for {}",
            route.path
        );
    }
}

#[test]
fn route_pages_render_api_surface() {
    let html = route_page_html("/downloads");
    assert!(html.contains("Downloads"));
    assert!(html.contains("/api/v0/transfers/downloads"));
    assert!(html.contains("data-route=\"/downloads\""));
    assert!(html.contains("data-slskr-refresh-scope=\"active-route\""));
    assert!(html.contains("data-slskr-lazy-workspace=\"true\""));
    assert!(html.contains("data-slskr-lazy-diagnostics=\"true\""));
    assert!(html.contains("slskr-route-data"));
    assert!(html.contains("Workspace"));
    assert!(html.contains("<h3>Downloads</h3>"));
    assert!(html.contains("slskr-route-actions"));
    assert!(html.contains("slskr-route-summary"));
    assert!(html.contains("Overview"));
    assert!(html.contains("slskr-page-data"));
    assert!(html.contains("Developer"));
    assert!(html.contains("Clear Completed Downloads"));
    assert!(html.contains("data-slskr-refresh-route"));
    assert!(html.contains("data-slskr-focus-filter"));
    assert!(html.contains("data-slskr-clear-filters"));
    assert!(html.contains("slskr-live-status"));
}

#[test]
fn route_pages_render_domain_workflows_before_developer_details() {
    let expectations = [
        ("/searches", "Searches", "Search", "search-native"),
        (
            "/discovery-graph",
            "Discovery Graph Atlas",
            "Build graph",
            "discovery-graph-native",
        ),
        (
            "/playlist-intake",
            "Playlist Intake",
            "Preview playlist",
            "playlist-intake-native",
        ),
        (
            "/wishlist",
            "Wishlist",
            "Add wanted search",
            "wishlist-native",
        ),
        ("/downloads", "Downloads", "Download", "transfers-native"),
        ("/uploads", "Uploads", "Clear completed", "transfers-native"),
        ("/messages", "Conversations", "Reply", "messaging-native"),
        ("/users", "Users", "Watch", "users-native"),
        ("/contacts", "Contacts", "Add contact", "contacts-native"),
        ("/solid", "Solid", "Connect identity", "solid-native"),
        (
            "/collections",
            "Collections",
            "Create Collection",
            "collections-native",
        ),
        (
            "/sharegroups",
            "Share Groups",
            "Issue token",
            "sharegroups-native",
        ),
        ("/shared", "Shared with Me", "Open", "shared-native"),
        ("/browse", "Browse", "Browse", "browse-native"),
        ("/system", "System", "Rescan Shares", "system-native"),
    ];

    for (path, heading, action, native_class) in expectations {
        let html = route_page_html(path);
        let heading_index = html
            .find(heading)
            .unwrap_or_else(|| panic!("missing workflow heading {heading} for route {path}"));
        let developer_index = html
            .find("<summary>Developer</summary>")
            .unwrap_or_else(|| panic!("missing developer drawer for route {path}"));

        assert!(
            heading_index < developer_index,
            "route {path} should show workflow content before developer diagnostics"
        );
        assert!(
            html.contains(action),
            "missing primary action {action} for route {path}"
        );
        assert!(html.contains("slskr-workflow"));
        assert!(html.contains("slskr-native-workspace"));
        assert!(html.contains("slskr-native-subviews"));
        assert!(html.contains("slskr-native-panel-actions"));
        assert!(html.contains("slskr-native-panel-fields"));
        assert!(html.contains("slskr-native-panel-facts"));
        assert!(html.contains("data-slskr-native-tab=\"0\""));
        assert!(html.contains("data-slskr-native-panel=\"0\""));
        assert!(html.contains("data-slskr-native-filter"));
        assert!(html.contains("data-slskr-native-count"));
        assert!(html.contains("data-slskr-native-select-visible"));
        assert!(html.contains("data-slskr-native-clear-selection"));
        assert!(html.contains("data-slskr-native-reset-state"));
        if html.contains("aria-keyshortcuts") {
            assert!(html.contains("aria-keyshortcuts=\"Enter Space ArrowUp ArrowDown Home End\""));
            assert!(html.contains("data-slskr-native-sort=\"0\""));
            assert!(html.contains("data-slskr-native-sort-0="));
            assert!(html.contains("data-slskr-native-index="));
        } else {
            assert!(html.contains("slskr-native-empty"));
        }
        assert!(html.contains("slskr-native-inspector"));
        assert!(html.contains("data-slskr-native-inspector-title"));
        if html.contains("aria-keyshortcuts") {
            assert!(html.contains("data-slskr-native-select"));
        }
        assert!(html.contains("slskr-native-selection-status"));
        assert!(html.contains("slskr-toast-region"));
        assert!(
            html.contains(native_class),
            "route {path} should render native parity class {native_class}"
        );
        assert!(html.contains("data-slskr-parity-reference"));
        assert!(html.contains("data-react-component="));
        assert!(html.contains("slskr-route-summary"));
        assert!(html.contains("data-slskr-refresh-route"));

        let parity_index = html
            .find("data-slskr-parity-reference")
            .unwrap_or_else(|| panic!("missing protocol compatibility panel for route {path}"));
        let native_index = html
            .find("slskr-native-workspace")
            .unwrap_or_else(|| panic!("missing native workspace for route {path}"));
        assert!(
            parity_index < native_index,
            "route {path} should show compatibility content before native page body"
        );
    }
}

#[test]
fn route_workflows_render_populated_api_rows() {
    let cases = [
        (
            "/searches/42",
            ApiEndpoint {
                method: "GET",
                path: "/searches/:id/responses",
                surface: "search",
            },
            r#"[{"username":"peer-live","hasFreeUploadSlot":true,"queueLength":2,"files":[{"filename":"Artist/Album/01 Track.flac"}]}]"#,
            &[
                "Artist/Album/01 Track.flac",
                "peer-live",
                "free slot / queue 2",
            ][..],
        ),
        (
            "/discovery-graph",
            ApiEndpoint {
                method: "GET",
                path: "/searches",
                surface: "search",
            },
            r#"[{"id":42,"searchText":"public domain jazz","state":"Running"}]"#,
            &["public domain jazz", "search 42", "Running"][..],
        ),
        (
            "/playlist-intake",
            ApiEndpoint {
                method: "POST",
                path: "/source-feed-imports/preview",
                surface: "source",
            },
            r#"[{"artist":"Archive Artist","title":"Public Domain Theme","status":"Matched"}]"#,
            &["Public Domain Theme", "Archive Artist", "Matched"][..],
        ),
        (
            "/wishlist",
            ApiEndpoint {
                method: "GET",
                path: "/wishlist",
                surface: "wishlist",
            },
            r#"[{"searchText":"rare live set","filter":"flac","enabled":true,"autoDownload":false}]"#,
            &["rare live set", "flac", "enabled=true / auto=false"][..],
        ),
        (
            "/downloads",
            ApiEndpoint {
                method: "GET",
                path: "/transfers/downloads",
                surface: "transfers",
            },
            r#"[{"username":"peer-down","files":[{"filename":"Remote/Song.mp3","state":"InProgress","progress":0.5,"speed":"1 MB/s"}]}]"#,
            &["Remote/Song.mp3", "peer-down", "InProgress / 50% / 1 MB/s"][..],
        ),
        (
            "/uploads",
            ApiEndpoint {
                method: "GET",
                path: "/transfers/uploads",
                surface: "transfers",
            },
            r#"[{"username":"peer-up","files":[{"filename":"Local/Song.flac","state":"Queued","progress":0.25,"speed":"512 KB/s"}]}]"#,
            &["Local/Song.flac", "peer-up", "Queued / 25% / 512 KB/s"][..],
        ),
        (
            "/messages",
            ApiEndpoint {
                method: "GET",
                path: "/conversations",
                surface: "messages",
            },
            r#"[{"username":"peer-msg","lastMessage":"hello","unreadCount":3}]"#,
            &["peer-msg", "hello", "3 unread"][..],
        ),
        (
            "/users",
            ApiEndpoint {
                method: "GET",
                path: "/users",
                surface: "users",
            },
            r#"[{"username":"peer-user","status":"Online","sharedFileCount":100}]"#,
            &["peer-user", "Online", "100"][..],
        ),
        (
            "/contacts",
            ApiEndpoint {
                method: "GET",
                path: "/contacts",
                surface: "contacts",
            },
            r#"[{"nickname":"Friend","peerId":"peer-contact","verified":true}]"#,
            &["Friend", "peer-contact", "verified=true"][..],
        ),
        (
            "/solid",
            ApiEndpoint {
                method: "GET",
                path: "/solid/status",
                surface: "solid",
            },
            r#"{"webId":"https://example.test/profile#me","storage":"pod-a","status":"connected"}"#,
            &["https://example.test/profile#me", "pod-a", "connected"][..],
        ),
        (
            "/collections",
            ApiEndpoint {
                method: "GET",
                path: "/collections",
                surface: "collections",
            },
            r#"[{"title":"Live Collection","type":"Playlist","itemCount":7}]"#,
            &["Live Collection", "Playlist", "7 items"][..],
        ),
        (
            "/sharegroups",
            ApiEndpoint {
                method: "GET",
                path: "/sharegroups",
                surface: "sharegroups",
            },
            r#"[{"name":"Trusted peers","memberCount":2,"createdAt":"today"}]"#,
            &["Trusted peers", "2 members", "today"][..],
        ),
        (
            "/shared",
            ApiEndpoint {
                method: "GET",
                path: "/share-grants",
                surface: "sharegroups",
            },
            r#"[{"id":"grant-1","title":"Shared Collection","owner":"peer-owner","permissions":"read"}]"#,
            &["Shared Collection", "peer-owner", "read"][..],
        ),
        (
            "/browse",
            ApiEndpoint {
                method: "GET",
                path: "/users/:username/browse",
                surface: "browse",
            },
            r#"{"directories":[{"name":"Music","type":"folder","size":0}],"files":[{"filename":"Music/Track.flac","type":"file","size":12345}]}"#,
            &["Music", "Music/Track.flac", "Download"][..],
        ),
        (
            "/system",
            ApiEndpoint {
                method: "GET",
                path: "/server",
                surface: "system",
            },
            r#"{"state":"Connected","username":"audit-user"}"#,
            &["Connection", "Connected", "audit-user"][..],
        ),
    ];

    for (route, endpoint, body, expected) in cases {
        let html = route_workspace_result_html(
            route,
            &[EndpointBody {
                endpoint,
                body: body.to_string(),
            }],
        );
        for value in expected {
            assert!(
                html.contains(value),
                "route {route} should render live workflow value {value}"
            );
        }
    }
}

#[test]
fn array_data_cards_render_filterable_table_and_csv_views() {
    let response = EndpointBody {
            endpoint: ApiEndpoint {
                method: "GET",
                path: "/searches",
                surface: "search",
            },
            body: r#"[{"id":1,"query":"public domain jazz","status":"Completed","username":"peer1"},{"id":2,"query":"archive live set","status":"Running","username":"peer2"}]"#.to_string(),
        };
    let html = data_card_result_html(&response);
    assert!(html.contains("data-slskr-data-card"));
    assert!(html.contains("data-slskr-view=\"list\""));
    assert!(html.contains("slskr-card-filter"));
    assert!(html.contains("data-slskr-card-clear"));
    assert!(html.contains("data-slskr-card-count"));
    assert!(html.contains("2 / 2"));
    assert!(html.contains("data-slskr-card-view=\"table\""));
    assert!(html.contains("data-slskr-sort-index"));
    assert!(html.contains("slskr-data-table"));
    assert!(html.contains("2 records"));
    assert!(html.contains("public domain jazz"));
    assert!(html.contains("data-slskr-record-select"));
    assert!(html.contains("data-slskr-record-json"));
    assert!(html.contains("slskr-card-inspector"));
    assert!(html.contains("Record Inspector"));
    assert!(html.contains("CSV"));
}

#[test]
fn native_subpanels_cover_deep_route_workflows() {
    let system = route_page_html("/system");
    for value in [
        "MediaCore",
        "Security Policies",
        "Library Health",
        "Quarantine Jury",
        "slskr-system-panel-table",
        "Outbound webhooks",
        "MusicBrainz",
        "Raw metrics",
        "Source providers",
        "Scan Library Health",
        "Vacuum Database",
        "Proxy trust",
    ] {
        assert!(
            system.contains(value),
            "system panel should contain {value}"
        );
    }

    let browse = route_page_html("/browse");
    for value in [
        "Open a New Browse Tab",
        "Breadcrumb",
        "Multi-select",
        "Download Selected",
        "data-slskr-browse-workspace",
        "data-slskr-browse-session",
        "data-slskr-browse-folder",
        "data-slskr-browse-download-manifest",
        "File filter",
        "Refresh Folder",
        "Estimated queue impact appears after selection",
    ] {
        assert!(
            browse.contains(value),
            "browse panel should contain {value}"
        );
    }

    let messages = route_page_html("/messages");
    for value in [
        "Delete Conversation",
        "Unread",
        "Pods",
        "Compose",
        "data-slskr-messages-workspace",
        "data-slskr-message-lifecycle",
        "data-slskr-room-state",
        "data-slskr-pod-state",
        "data-slskr-thread-state",
        "data-slskr-message-transcript",
        "data-slskr-message-actions",
        "data-slskr-compose-history",
        "Search conversations",
        "Clear Search",
        "data-slskr-message-gate-panel",
        "Gate reply",
        "data-slskr-message-gate-save",
        "cooldownMinutes",
    ] {
        assert!(
            messages.contains(value),
            "messages panel should contain {value}"
        );
    }
    let live_messages = route_workspace_result_html(
            "/messages",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/private-message-auto-response",
                    surface: "messages",
                },
                body: r#"{"enabled":true,"message_configured":true,"cooldown_minutes":15,"runtimeMutable":true}"#.to_string(),
            }],
        );
    assert!(live_messages.contains("<mark>armed</mark>"));
    assert!(live_messages.contains(r#"value="15""#));
    assert!(!live_messages.contains("Runtime human check"));

    let sharing = populated_route_html("/sharegroups");
    for value in ["Create Share Grant", "Update Share Grant", "Permissions"] {
        assert!(
            sharing.contains(value),
            "share groups panel should contain {value}"
        );
    }
}

#[test]
fn native_primary_workspaces_include_react_like_structures() {
    for path in [
        "/searches",
        "/discovery-graph",
        "/playlist-intake",
        "/wishlist",
        "/downloads",
        "/uploads",
        "/messages",
        "/users",
        "/contacts",
        "/solid",
        "/collections",
        "/sharegroups",
        "/shared",
        "/browse",
        "/system",
    ] {
        let page = route_page_html(path);
        assert!(
            page.contains("data-slskr-native-preview-title"),
            "{path} should expose a row-driven native preview title"
        );
        assert!(
            page.contains("data-slskr-native-preview-count"),
            "{path} should expose a row-driven native preview count"
        );
        assert!(
            page.contains("data-slskr-native-preview-fields"),
            "{path} should expose row-driven native preview fields"
        );
        assert!(
            page.contains("data-slskr-native-inspector-fields"),
            "{path} should expose row-driven native inspector fields"
        );
        assert!(
            page.contains("data-slskr-native-inspector-actions"),
            "{path} should expose selected-row context actions in the inspector"
        );
    }

    let search = route_page_html("/searches");
    for value in [
        "Search Detail",
        "No search selected",
        "Duplicate folding",
        "data-slskr-search-filter-modal",
        "Fold duplicate results",
        "Search ranking profile",
        "Hide locked files",
        "data-slskr-search-expansion",
    ] {
        assert!(
            search.contains(value),
            "search workspace should contain {value}"
        );
    }

    let discovery = route_page_html("/discovery-graph");
    for value in [
        "Discovery Graph Atlas",
        "No graph node selected",
        "Queue Nearby",
    ] {
        assert!(
            discovery.contains(value),
            "discovery workspace should contain {value}"
        );
    }

    let playlist = route_page_html("/playlist-intake");
    for value in [
        "Import validation",
        "No playlist row selected",
        "Import Playlist",
    ] {
        assert!(
            playlist.contains(value),
            "playlist workspace should contain {value}"
        );
    }

    let wishlist = route_page_html("/wishlist");
    for value in [
        "Request Portal Summary",
        "No wishlist item selected",
        "Run Enabled",
    ] {
        assert!(
            wishlist.contains(value),
            "wishlist workspace should contain {value}"
        );
    }

    let downloads = route_page_html("/downloads");
    for value in ["Transfer Group", "No downloads selected", "Retry All"] {
        assert!(
            downloads.contains(value),
            "downloads workspace should contain {value}"
        );
    }
    assert!(
        downloads.contains("data-slskr-transfer-state-control")
            || downloads.contains("No downloads to display")
    );

    let uploads = route_page_html("/uploads");
    for value in ["Transfer Group", "No uploads selected"] {
        assert!(
            uploads.contains(value),
            "uploads workspace should contain {value}"
        );
    }

    let users = route_page_html("/users");
    for value in ["User Detail", "No user selected", "Save note"] {
        assert!(
            users.contains(value),
            "users workspace should contain {value}"
        );
    }

    let contacts = route_page_html("/contacts");
    for value in ["All Contacts", "No contact selected", "Refresh Nearby"] {
        assert!(
            contacts.contains(value),
            "contacts workspace should contain {value}"
        );
    }

    let solid = route_page_html("/solid");
    for value in [
        "Identity Document",
        "No Solid resource selected",
        "Resolve WebID",
        "Sync Storage",
    ] {
        assert!(
            solid.contains(value),
            "solid workspace should contain {value}"
        );
    }

    let system = route_page_html("/system");
    for value in [
        "Operator Actions",
        "No system item selected",
        "Diagnostic Bundle",
    ] {
        assert!(
            system.contains(value),
            "system workspace should contain {value}"
        );
    }

    let messages = route_page_html("/messages");
    for value in [
        "Thread Workspace",
        "slskr-native-thread-grid",
        "data-slskr-native-preview-title",
        "data-slskr-native-preview-count",
        "No conversation selected",
        "Delete Conversation",
        "No pod channels returned",
    ] {
        assert!(
            messages.contains(value),
            "messages workspace should contain {value}"
        );
    }

    let browse = route_page_html("/browse");
    for value in [
        "Directory Tree",
        "slskr-native-breadcrumb",
        "Download Preview",
        "data-slskr-native-preview-title",
        "Preserve folders",
        "Duplicate warning review",
    ] {
        assert!(
            browse.contains(value),
            "browse workspace should contain {value}"
        );
    }

    let collections = route_page_html("/collections");
    for value in [
        "Item Picker",
        "data-slskr-native-preview-title",
        "Already in collection warning",
        "Audience picker",
        "Stream/download policies",
    ] {
        assert!(
            collections.contains(value),
            "collections workspace should contain {value}"
        );
    }

    let sharegroups = route_page_html("/sharegroups");
    for value in [
        "Grant Matrix",
        "data-slskr-native-preview-title",
        "Token revoke",
        "Grant audit trail",
        "Create Share Grant",
    ] {
        assert!(
            sharegroups.contains(value),
            "share groups workspace should contain {value}"
        );
    }

    let shared = populated_route_html("/shared");
    for value in [
        "Shared Manifest",
        "file-level access preview",
        "data-slskr-native-preview-title",
        "Backfill selected collection",
    ] {
        assert!(
            shared.contains(value),
            "shared workspace should contain {value}"
        );
    }
}

#[test]
fn native_editor_surfaces_cover_modal_workflows() {
    let expectations = [
        (
            "/wishlist",
            &[
                "data-slskr-native-editor",
                "Wishlist Editor",
                "Auto-download",
                "Discovery Inbox bridge",
            ][..],
        ),
        (
            "/users",
            &[
                "data-slskr-native-editor",
                "User Note Editor",
                "Privileges and stats",
                "Save note",
            ],
        ),
        (
            "/contacts",
            &[
                "data-slskr-native-editor",
                "Contact Editor",
                "Create Invite",
                "Groups and notes",
            ],
        ),
        (
            "/collections",
            &[
                "data-slskr-native-editor",
                "Collection Editor",
                "Audience",
                "Remove item",
            ],
        ),
        (
            "/sharegroups",
            &[
                "data-slskr-native-editor",
                "Share Grant Editor",
                "Permissions",
                "Issue Token",
            ],
        ),
        (
            "/shared",
            &[
                "data-slskr-native-editor",
                "Inbound Access Editor",
                "Copy token",
            ],
        ),
        (
            "/system",
            &[
                "data-slskr-native-editor",
                "Settings Editor",
                "Option key",
                "Diagnostic Bundle",
            ],
        ),
    ];

    for (path, labels) in expectations {
        let html = route_page_html(path);
        for label in labels {
            assert!(
                html.contains(label),
                "{path} native editor should contain {label}"
            );
        }
    }

    let search = route_page_html("/searches");
    assert!(
        !search.contains("data-slskr-native-editor"),
        "search should keep its planner in the route workspace instead of the editor modal surface"
    );
}
