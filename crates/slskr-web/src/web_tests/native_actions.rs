use super::*;

#[test]
fn rust_actions_render_core_mutations() {
    let html = route_page_html("/searches/42");
    assert!(html.contains("Start Search"));
    assert!(html.contains("Stop Search"));
    assert!(html.contains("Remove Search"));
    assert!(html.contains("Clear Searches"));
    assert!(html.contains("/api/v0/searches/42"));
    assert!(html.contains("data-slskr-action-body=\"SearchText\""));

    let transfers = route_page_html("/downloads");
    assert!(transfers.contains("Queue Download"));
    assert!(transfers.contains("Enable Accelerated Downloads"));
    assert!(transfers.contains("Disable Accelerated Downloads"));
    assert!(transfers.contains("/api/v0/transfers/downloads/peer1"));

    let rooms = route_page_html("/rooms");
    assert!(rooms.contains("Join Room"));
    assert!(rooms.contains("Send Room Message"));
    assert!(rooms.contains("Leave Room"));
    assert!(rooms.contains("/api/v0/rooms/joined/contract-room/messages"));

    let messages = route_page_html("/messages");
    assert!(messages.contains("Send Message"));
    assert!(messages.contains("Acknowledge Conversation"));
    assert!(messages.contains("Delete Conversation"));

    let system = route_page_html("/system/network");
    assert!(system.contains("Connect"));
    assert!(system.contains("Disconnect"));
    assert!(system.contains("Rescan Shares"));
    assert!(system.contains("/api/v0/server"));

    let wishlist = route_page_html("/wishlist");
    assert!(wishlist.contains("Add Wishlist Item"));
    assert!(wishlist.contains("Run Wishlist Search"));

    let contacts = route_page_html("/contacts");
    assert!(contacts.contains("Add Contact"));
    assert!(contacts.contains("Watch User"));
    assert!(contacts.contains("Add User Note"));

    let collections = route_page_html("/collections");
    assert!(collections.contains("Create Collection"));
    assert!(collections.contains("Create Share Group"));
    assert!(collections.contains("Create Share Grant"));
    assert!(collections.contains("Backfill Share Grant"));
    assert!(collections.contains("Add Item to Collection"));

    let integrations = route_page_html("/playlist-intake");
    assert!(integrations.contains("Preview Playlist"));
    assert!(integrations.contains("Build Discovery Graph"));
    assert!(integrations.contains("Track MusicBrainz Target"));
    assert!(integrations.contains("Create SongID Run"));
}

#[test]
fn native_workflow_labels_resolve_to_real_route_actions() {
    let expectations = [
        ("/searches", "Search", "Start Search"),
        ("/discovery-graph", "Build Atlas", "Build Discovery Graph"),
        ("/playlist-intake", "Import Playlist", "Preview Playlist"),
        ("/wishlist", "Run Enabled", "Run Wishlist Search"),
        ("/downloads", "Clear Completed", "Clear Completed Downloads"),
        ("/downloads", "Cancel All", "Cancel Download"),
        ("/uploads", "Clear Completed", "Clear Completed Uploads"),
        ("/uploads", "Allow selected", "Allow Upload"),
        ("/uploads", "Deny selected", "Deny Upload"),
        ("/messages", "Reply", "Send Message"),
        ("/messages", "Delete Conversation", "Delete Conversation"),
        ("/users", "Watch", "Watch User"),
        ("/users", "Message", "Send Message"),
        ("/contacts", "Add Friend", "Add Contact"),
        ("/contacts", "Message", "Send Message"),
        ("/contacts", "Browse", "Request Directory"),
        ("/contacts", "Remove", "Remove Contact"),
        ("/contacts", "Create Invite", "Create Invite"),
        ("/contacts", "Refresh Nearby", "Refresh Nearby"),
        ("/solid", "Resolve WebID", "Resolve WebID"),
        ("/collections", "Open", "Open Collection"),
        ("/collections", "Add Item", "Add Item to Collection"),
        ("/collections", "Share", "Create Share Grant"),
        ("/sharegroups", "Create Share Grant", "Create Share Grant"),
        ("/sharegroups", "Update Share Grant", "Update Share Grant"),
        ("/sharegroups", "Issue Token", "Issue Share Token"),
        ("/shared", "Open", "Open Shared Manifest"),
        ("/shared", "Backfill", "Backfill Share Grant"),
        ("/browse", "Download Selected", "Queue Download"),
        ("/system", "Check for Updates", "Check for Updates"),
        ("/system", "Get Privileges", "Get Privileges"),
        ("/system", "Diagnostic Bundle", "Diagnostic Bundle"),
        ("/system", "Setup Health", "Setup Health"),
        ("/system", "Vacuum database", "Vacuum Database"),
        ("/system", "Check Lidarr", "Check Lidarr"),
        ("/system", "Refresh Lidarr Sync", "Refresh Lidarr Sync"),
        (
            "/system",
            "Track MusicBrainz Target",
            "Track MusicBrainz Target",
        ),
        ("/system", "Refresh MusicBrainz", "Refresh MusicBrainz"),
        ("/system", "Start SongID Run", "Start SongID Run"),
        ("/system", "Refresh SongID", "Refresh SongID"),
        (
            "/system",
            "Scan Library Health",
            "Start Library Health Scan",
        ),
        ("/system", "Fix Library Issues", "Fix Library Issues"),
    ];

    for (path, label, expected_action) in expectations {
        let action = route_action_for_native_label(path, label)
            .unwrap_or_else(|| panic!("{path} {label} should resolve"));
        assert_eq!(action.label, expected_action);
    }
}

#[test]
fn native_action_fallbacks_are_domain_specific() {
    assert!(native_action_fallback(ActionBody::SearchText).is_empty());
    assert!(native_action_fallback(ActionBody::FeedPreview).is_empty());
    assert!(native_action_fallback(ActionBody::DownloadFiles).is_empty());
    assert_eq!(native_action_fallback(ActionBody::BrowseDirectory), "/");
    assert!(native_action_fallback(ActionBody::Username).is_empty());
    assert!(native_action_fallback(ActionBody::None).is_empty());
}

#[test]
fn native_tables_expose_domain_row_action_sets() {
    let expectations = [
        ("/searches", &["Preview", "Download"][..]),
        ("/discovery-graph", &["Queue Nearby", "Build Atlas"]),
        ("/playlist-intake", &["Import Playlist", "Queue Plans"]),
        ("/wishlist", &["Run Enabled", "Copy Review"]),
        ("/downloads", &["Retry", "Cancel", "Remove"]),
        ("/uploads", &["Allow selected", "Deny selected"]),
        (
            "/messages",
            &["Reply", "Acknowledge", "Delete Conversation"],
        ),
        ("/users", &["Message", "Watch", "Save note"]),
        ("/contacts", &["Message", "Browse", "Remove"]),
        (
            "/solid",
            &["Resolve WebID", "Connect Identity", "Sync Storage"],
        ),
        ("/collections", &["Add Item", "Share", "Remove item"]),
        (
            "/sharegroups",
            &[
                "Add Member",
                "Issue Token",
                "Create Share Grant",
                "Update Share Grant",
            ],
        ),
        ("/shared", &["Stream", "Backfill", "Copy token"]),
        ("/browse", &["Download Selected", "Open a New Browse Tab"]),
        (
            "/system",
            &["Rescan shares", "Vacuum database", "Diagnostic Bundle"],
        ),
    ];

    for (path, labels) in expectations {
        let html = populated_route_html(path);
        assert!(
            html.contains("slskr-native-row-actions"),
            "{path} should render row action toolbar"
        );
        assert!(
            html.contains("data-slskr-native-action-menu"),
            "{path} should expose selected-row action menu data"
        );
        for label in labels {
            assert!(
                html.contains(label),
                "{path} row action toolbar should contain {label}"
            );
        }
    }
}

#[test]
fn rust_action_bodies_are_json_safe() {
    assert_eq!(
        action_body_from_value(ActionBody::SearchText, "a \"b\"").unwrap(),
        r#"{"searchText":"a \"b\""}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::MusicBrainzTarget, "release-1").unwrap(),
        r#"{"releaseId":"release-1"}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::SongIdSource, "query").unwrap(),
        r#"{"source":"query"}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::BrowseDirectory, "Music\\Jazz\nLive").unwrap(),
        r#"{"directory":"Music\\Jazz\nLive"}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::JsonString, "room\t<script>").unwrap(),
        "\"room\\t<script>\""
    );
    assert_eq!(
        action_body_from_value(ActionBody::DownloadFiles, "Remote/Track.flac").unwrap(),
        r#"[{"filename":"Remote/Track.flac","size":99}]"#
    );
    assert_eq!(
        action_body_from_value(
            ActionBody::DownloadFiles,
            "Remote/Track.flac\nRemote/Other.mp3"
        )
        .unwrap(),
        r#"[{"filename":"Remote/Track.flac","size":99},{"filename":"Remote/Other.mp3","size":99}]"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::EnabledTrue, "ignored").unwrap(),
        r#"{"enabled":true}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::EnabledFalse, "ignored").unwrap(),
        r#"{"enabled":false}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::Username, "peer1").unwrap(),
        r#"{"username":"peer1","note":"Created from the Rust web UI"}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::Permissions, "").unwrap(),
        r#"{"permissions":"read"}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::ShareGrant, "peer1").unwrap(),
        r#"{"collection_id":"","username":"peer1"}"#
    );
    assert_eq!(
        action_body_from_value(ActionBody::ShareGroupMember, "peer1").unwrap(),
        r#"{"userId":"peer1"}"#
    );
    assert!(
        action_body_from_value(ActionBody::FeedPreview, "artist - song")
            .unwrap()
            .contains("\"sourceText\":\"artist - song\"")
    );
    assert!(action_body_from_value(ActionBody::None, "ignored").is_none());
}

#[test]
fn native_row_actions_are_marked_for_selected_row_execution() {
    let html = populated_route_html("/browse");
    assert!(html.contains(r#"data-slskr-native-row-action="Download Selected""#));
    assert!(html.contains(r#"data-slskr-native-row-action="Open a New Browse Tab""#));
    assert!(html.contains(r#"data-slskr-native-title="/Music/Open Sessions""#));
    assert!(html.contains(r#"data-slskr-native-resource-id="row-1""#));
    let users = populated_route_html("/users");
    assert!(users.contains(r#"data-slskr-native-resource-id="peer1""#));
}

#[test]
fn native_rows_expose_structured_domain_action_values() {
    let search = route_workspace_result_html(
            "/searches",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/searches/:id/responses",
                    surface: "search",
                },
                body: r#"[{"username":"peer1","files":[{"filename":"Archive/Track.flac"}],"queueLength":2,"hasFreeUploadSlot":true}]"#.to_string(),
            }],
        );
    assert!(search.contains(r#"data-slskr-native-filename="Archive/Track.flac""#));
    assert!(search.contains(r#"data-slskr-native-peer="peer1""#));
    assert!(search.contains(r#"data-slskr-native-queue-state="free slot / queue 2""#));
    assert!(search.contains("data-slskr-search-result-controls"));
    assert!(search.contains("Expand Result"));
    assert!(search.contains("Fold Duplicates"));
    assert!(search.contains("Smart rank"));

    let downloads = route_workspace_result_html(
            "/downloads",
            &[
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/transfers/downloads",
                        surface: "transfers",
                    },
                    body: r#"[{"id":77,"username":"peer2","filename":"Remote/Song.mp3","state":"Queued","progress":0.5}]"#.to_string(),
                },
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/downloads/requests",
                        surface: "transfers",
                    },
                    body: r#"[{"request":{"id":"11111111-1111-4111-8111-111111111111","name":"Archive Cut","originalFilename":"Remote/Song.mp3","size":1200,"state":"Failed","bitRate":320,"sampleRate":48000,"bitDepth":24,"length":180,"artist":"Archive Artist","title":"Song"},"attemptCount":2,"current":{"id":77,"peer_username":"peer2","bytes_transferred":600,"recovery_action":"retry","recovery_label":"Find other sources"}}]"#.to_string(),
                },
            ],
        );
    assert!(downloads.contains(r#"data-slskr-native-filename="Remote/Song.mp3""#));
    assert!(downloads.contains(r#"data-slskr-native-peer="peer2""#));
    assert!(
        downloads.contains(r#"data-slskr-native-transfer-state="Queued / 50% / 0 B/s / id=77""#)
    );
    assert!(downloads.contains(r#"data-slskr-native-transfer-id="77""#));
    assert!(downloads.contains(
        r#"data-slskr-native-action-summary="Cancel download 77 from peer2: Remote/Song.mp3""#
    ));
    assert!(downloads.contains(
            r#"data-slskr-native-detail-list="File: Remote/Song.mp3 | Peer: peer2 | Download state: Queued / 50% / 0 B/s / id=77 | Next action: Cancel""#
        ));
    assert!(downloads.contains(r#"data-slskr-native-action-menu="Cancel | Retry | Remove""#));
    assert!(downloads.contains(r#"<meter min="0" max="100" value="50""#));
    assert!(downloads.contains("data-slskr-transfer-state-control"));
    for value in [
        "data-slskr-transfer-request-workspace",
        "Downloads and attempts",
        "Archive Cut",
        "Archive Artist — Song",
        "Find other sources",
        "data-slskr-transfer-column-toggle=\"bitrate\"",
        "data-slskr-transfer-retry=\"77\"",
        "data-slskr-transfer-attempts",
        "data-slskr-transfer-rename-save",
    ] {
        assert!(
            downloads.contains(value),
            "downloads should contain {value}"
        );
    }

    let contacts = route_workspace_result_html(
        "/contacts",
        &[EndpointBody {
            endpoint: ApiEndpoint {
                method: "GET",
                path: "/contacts",
                surface: "identity",
            },
            body: r#"[{"nickname":"Nick","peerId":"peer3","group":"trusted","verified":true}]"#
                .to_string(),
        }],
    );
    assert!(contacts.contains(r#"data-slskr-native-contact="Nick""#));
    assert!(contacts.contains(r#"data-slskr-native-username="peer3""#));

    let wishlist = populated_route_html("/wishlist");
    assert!(wishlist.contains("data-slskr-native-search-filter"));
    assert!(wishlist.contains("data-slskr-wishlist-ignore-manager"));
    assert!(wishlist.contains("Ignored result folders"));
    assert_eq!(
        wishlist.matches("data-slskr-native-filter ").count(),
        1,
        "only the filter input should use data-slskr-native-filter"
    );

    let live_wishlist = route_workspace_result_html(
            "/wishlist",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/wishlist",
                    surface: "wishlist",
                },
                body: r#"[{"id":"wish-7","searchText":"rare live set","filter":"flac","enabled":true,"autoDownload":true,"maxResults":25,"maxDownloads":3,"lastSearchedAt":200,"lastViewedAt":100,"lastVisibleHitCount":8,"lastHiddenLockedHitCount":2,"lastFilteredOutHitCount":4,"lastIgnoredResultHitCount":1,"lastResponseCount":6,"totalSearchCount":9,"totalDownloadCount":2,"ignoredResultCount":1,"ignoredResults":[{"id":"rule-9","username":"noisy-peer","directory":"Bootlegs/Unsorted"}]}]"#
                    .to_string(),
            }],
        );
    assert!(live_wishlist.contains(r#"data-slskr-native-wishlist-id="wish-7""#));
    assert!(live_wishlist
        .contains(r#"data-slskr-native-action-summary="Run wishlist wish-7: rare live set""#));
    assert!(live_wishlist.contains(
            r#"data-slskr-native-detail-list="Wanted search: rare live set | Filter: flac | Automation: enabled=true / auto=true / id=wish-7 | Next action: Run""#
        ));
    assert!(live_wishlist
        .contains(r#"data-slskr-native-action-menu="Run | Run Enabled | Copy Review""#));
    assert!(live_wishlist.contains("data-slskr-wishlist-row-controls"));
    assert!(live_wishlist.contains(r#"aria-label="Enabled rare live set" checked"#));
    assert!(live_wishlist.contains("data-slskr-wishlist-policy-manager"));
    assert!(live_wishlist.contains("Search policy and history"));
    assert!(live_wishlist.contains("new results"));
    assert!(live_wishlist.contains("<strong>8</strong> visible"));
    assert!(live_wishlist.contains(r#"data-slskr-wishlist-policy-save"#));
    assert!(live_wishlist.contains(r#"data-slskr-wishlist-mark-viewed"#));
    assert!(live_wishlist.contains(r#"data-slskr-wishlist-load-history"#));
    assert!(live_wishlist.contains(r#"data-slskr-wishlist-policy-field="maxDownloads""#));
    assert!(live_wishlist.contains("Noise gate"));
    assert!(live_wishlist.contains("noisy-peer"));
    assert!(live_wishlist.contains("Bootlegs/Unsorted"));
    assert!(live_wishlist.contains(r#"data-slskr-wishlist-ignore-rule-id="rule-9""#));
    assert!(live_wishlist.contains(r#"data-slskr-wishlist-ignore-submit"#));
    assert!(live_wishlist.contains(">Restore</button>"));

    let history = wishlist_history_response_html(
        r#"[{"searchText":"rare live set","status":"completed","startedAt":"200","result_count":8}]"#,
    );
    assert!(history.contains("slskr-wishlist-history-list"));
    assert!(history.contains("rare live set"));
    assert!(history.contains("completed · 8 results"));

    let sharing = populated_route_html("/sharegroups");
    assert!(sharing.contains("data-slskr-native-share-group"));
    assert!(sharing.contains("data-slskr-native-member-count"));
    assert!(sharing.contains("data-slskr-share-group-row-controls"));

    let shared = populated_route_html("/shared");
    assert!(shared.contains("data-slskr-native-owner"));
    assert!(shared.contains("data-slskr-native-permissions"));
    assert!(shared.contains("data-slskr-inbound-permission-controls"));

    let browse = populated_route_html("/browse");
    assert!(browse.contains("data-slskr-native-path"));
    assert!(browse.contains("data-slskr-native-entry-kind"));
    assert!(browse.contains("data-slskr-native-filename"));
    assert!(browse.contains("data-slskr-browse-entry-controls"));
}

#[test]
fn rust_action_paths_reject_untrusted_route_params() {
    let endpoint = RouteAction {
        body: ActionBody::None,
        label: "Cancel Search",
        method: "DELETE",
        path: "/searches/:id",
        surface: "search",
    };
    assert_eq!(
        concrete_action_path("/searches/42", endpoint),
        "/api/v0/searches/42"
    );
    assert_eq!(
        concrete_action_path("/searches/<script>", endpoint),
        "/api/v0/searches/1"
    );
    assert_eq!(
        concrete_action_path("/searches", endpoint),
        "/api/v0/searches/1"
    );
    let transfer = RouteAction {
        body: ActionBody::None,
        label: "Cancel Download",
        method: "DELETE",
        path: "/transfers/downloads/:username/:id",
        surface: "transfers",
    };
    assert_eq!(
        concrete_action_path_with_target_and_id("/downloads", transfer, Some("peer2"), Some("77")),
        "/api/v0/transfers/downloads/peer2/77"
    );
    let wishlist = RouteAction {
        body: ActionBody::None,
        label: "Run Wishlist Search",
        method: "POST",
        path: "/wishlist/:id/search",
        surface: "wishlist",
    };
    assert_eq!(
        concrete_action_path_with_target_and_id(
            "/wishlist",
            wishlist,
            Some("ignored-peer"),
            Some("wish-7")
        ),
        "/api/v0/wishlist/wish-7/search"
    );
    let html = route_actions_html("/searches/<script>");
    assert!(html.contains("/api/v0/searches/1"));
    assert!(!html.contains("<script>"));
}

#[test]
fn route_action_lookup_uses_current_route_surface() {
    let search = route_action_at("/searches/42", 0).unwrap();
    assert_eq!(search.label, "Start Search");
    assert_eq!(
        concrete_action_path("/searches/42", search),
        "/api/v0/searches"
    );

    let remove = route_action_at("/searches/42", 2).unwrap();
    assert_eq!(remove.label, "Remove Search");
    assert_eq!(
        concrete_action_path("/searches/42", remove),
        "/api/v0/searches/42"
    );

    let browse =
        route_action_for_native_label("/browse", "Open a New Browse Tab").expect("browse action");
    assert_eq!(
        concrete_action_path_with_target("/browse", browse, Some("browse-peer")),
        "/api/v0/users/browse-peer/directory"
    );
    assert_eq!(
        concrete_action_path_with_target("/browse", browse, Some("../bad")),
        "/api/v0/users/peer1/directory"
    );
    let remove_item = route_action_at("/searches", 2).expect("remove search");
    assert_eq!(
        concrete_action_path_with_target("/searches", remove_item, Some("search-42")),
        "/api/v0/searches/search-42"
    );

    let collection_item = route_action_for_native_label("/collections", "Remove Collection Item")
        .expect("collection item action");
    assert_eq!(
        concrete_action_path_with_target_and_id(
            "/collections",
            collection_item,
            Some("collection-7"),
            Some("item-9"),
        ),
        "/api/v0/collections/collection-7/items/item-9"
    );
    assert_eq!(
        concrete_action_path_with_target_and_id(
            "/collections",
            collection_item,
            Some("../bad"),
            Some("item-9"),
        ),
        "/api/v0/collections/1/items/item-9"
    );

    assert!(route_action_at("/searches/42", usize::MAX).is_none());
    assert!(route_action_at("/not-a-route", 0).is_none());
}

#[test]
fn route_actions_cover_core_old_ui_surfaces() {
    let actions = route_actions();
    let surfaces = actions
        .iter()
        .map(|action| action.surface)
        .collect::<Vec<_>>();
    for expected in [
        "search",
        "transfers",
        "rooms",
        "messages",
        "browse",
        "wishlist",
        "identity",
        "collections",
        "integrations",
        "system",
    ] {
        assert!(surfaces.contains(&expected), "missing action {expected}");
    }
    for expected in [
        ("POST", "/searches"),
        ("PUT", "/searches/:id"),
        ("DELETE", "/searches/:id"),
        ("DELETE", "/searches"),
        ("POST", "/transfers/downloads/:username"),
        ("PUT", "/transfers/downloads/accelerated"),
        ("POST", "/rooms/joined"),
        ("POST", "/rooms/joined/:roomName/messages"),
        ("DELETE", "/rooms/joined/:roomName"),
        ("POST", "/conversations/:username"),
        ("PUT", "/conversations/:username"),
        ("DELETE", "/conversations/:username"),
        ("POST", "/users/:username/directory"),
        ("POST", "/wishlist"),
        ("POST", "/wishlist/:id/search"),
        ("POST", "/contacts"),
        ("POST", "/contacts/from-discovery"),
        ("POST", "/contacts/from-invite"),
        ("POST", "/users/watch"),
        ("POST", "/users/notes"),
        ("POST", "/collections"),
        ("POST", "/sharegroups"),
        ("POST", "/sharegroups/:id/members"),
        ("POST", "/share-grants"),
        ("PUT", "/share-grants/:id"),
        ("POST", "/share-grants/:id/backfill"),
        ("POST", "/share-grants/:id/token"),
        ("DELETE", "/share-grants/:id"),
        ("POST", "/library/items"),
        ("POST", "/source-feed-imports/preview"),
        ("POST", "/discovery-graph"),
        ("POST", "/source-feeds"),
        ("POST", "/musicbrainz/targets"),
        ("POST", "/musicbrainz/release-radar/subscriptions"),
        ("POST", "/songid/runs"),
        ("POST", "/jobs/discography"),
        ("POST", "/shares/rescan"),
        ("POST", "/database/vacuum"),
    ] {
        assert!(
            actions
                .iter()
                .any(|action| action.method == expected.0 && action.path == expected.1),
            "missing action {} {}",
            expected.0,
            expected.1
        );
    }
}
