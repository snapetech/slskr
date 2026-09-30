//! Native Web route workflow rendering.

use super::*;

pub(super) fn route_workflow_stats_html(
    kind: RouteKind,
    responses: Option<&[EndpointBody]>,
) -> String {
    let stats = match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph => vec![
            ("Searches", response_count(responses, "/searches"), "active"),
            (
                "Responses",
                response_count(responses, "/searches/:id/responses"),
                "selected",
            ),
            ("Profile", "balanced".to_string(), "ranking"),
        ],
        RouteKind::PlaylistIntake | RouteKind::Solid => vec![
            (
                "Providers",
                response_count(responses, "/source-providers"),
                "sources",
            ),
            ("Jobs", response_count(responses, "/jobs"), "automation"),
            (
                "Review",
                if responses.is_some() {
                    "available"
                } else {
                    "loading"
                }
                .to_string(),
                "queue",
            ),
        ],
        RouteKind::Wishlist => vec![
            ("Wanted", response_count(responses, "/wishlist"), "searches"),
            (
                "Enabled",
                response_bool_count(responses, "/wishlist", &["enabled"]),
                "state",
            ),
            ("Inbox", response_count(responses, "/wishlist"), "pending"),
        ],
        RouteKind::Downloads => vec![
            (
                "Active",
                response_count(responses, "/transfers/downloads"),
                "downloads",
            ),
            (
                "Speed",
                response_value(responses, "/transfers/speeds", "download"),
                "down",
            ),
            (
                "Slots",
                response_value(responses, "/config", "downloadSlots"),
                "limit",
            ),
        ],
        RouteKind::Uploads => vec![
            (
                "Active",
                response_count(responses, "/transfers/uploads"),
                "uploads",
            ),
            (
                "Speed",
                response_value(responses, "/transfers/speeds", "upload"),
                "up",
            ),
            (
                "Policy",
                response_value(responses, "/config", "uploadPolicy"),
                "mode",
            ),
        ],
        RouteKind::Messages => vec![
            (
                "Threads",
                response_count(responses, "/conversations"),
                "inbox",
            ),
            (
                "Unread",
                response_numeric_sum(
                    responses,
                    "/conversations",
                    &["unreadCount", "unacknowledgedCount"],
                ),
                "messages",
            ),
            ("Pods", response_count(responses, "/pods"), "secondary"),
        ],
        RouteKind::Rooms => vec![
            (
                "Available",
                response_count(responses, "/rooms/available"),
                "rooms",
            ),
            (
                "Joined",
                response_count(responses, "/rooms/joined"),
                "rooms",
            ),
            (
                "Activity",
                response_count(responses, "/rooms/joined"),
                "joined rooms",
            ),
        ],
        RouteKind::Users => vec![
            ("Watched", response_count(responses, "/users"), "users"),
            (
                "Online",
                response_online_count(responses, "/users"),
                "presence",
            ),
            ("Notes", response_count(responses, "/users/notes"), "saved"),
        ],
        RouteKind::Contacts => vec![
            ("Contacts", response_count(responses, "/contacts"), "people"),
            (
                "Nearby",
                response_count(responses, "/contacts/nearby"),
                "peers",
            ),
            ("Invites", "not loaded".to_string(), "open"),
        ],
        RouteKind::Collections => vec![
            (
                "Collections",
                response_count(responses, "/collections"),
                "sets",
            ),
            (
                "Items",
                response_count(responses, "/library/items"),
                "library",
            ),
            ("Shared", response_count(responses, "/shared"), "inbound"),
        ],
        RouteKind::ShareGroups => vec![
            ("Groups", response_count(responses, "/sharegroups"), "sets"),
            (
                "Grants",
                response_count(responses, "/share-grants"),
                "active",
            ),
            (
                "Tokens",
                response_numeric_sum(responses, "/share-grants", &["tokenCount", "tokens"]),
                "issued",
            ),
        ],
        RouteKind::SharedWithMe => vec![
            (
                "Shared",
                response_count(responses, "/share-grants"),
                "inbound",
            ),
            (
                "Grants",
                response_count(responses, "/share-grants"),
                "access",
            ),
            ("Expiring", "not calculated".to_string(), "soon"),
        ],
        RouteKind::Browse => vec![
            ("Peer", "not selected".to_string(), "target"),
            (
                "Folders",
                browse_response_entries(responses).len().to_string(),
                "returned",
            ),
            ("Selected", "0".to_string(), "files"),
        ],
        RouteKind::System => vec![
            (
                "Server",
                response_value(responses, "/server", "state"),
                "connection",
            ),
            ("Shares", response_count(responses, "/shares"), "roots"),
            (
                "Database",
                response_value(responses, "/database/stats", "status"),
                "storage",
            ),
        ],
    };
    stats
        .iter()
        .map(|(label, value, detail)| stat_card_html(label, value, detail))
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn route_workflow_toolbar_html(kind: RouteKind) -> String {
    match kind {
        RouteKind::Search => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Search text" aria-label="Search text"><select aria-label="Acquisition profile"><option>Balanced</option><option>Lossless exact</option><option>Fast good enough</option></select><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Search</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Clear</button></form>"#.to_string(),
        RouteKind::DiscoveryGraph => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Artist or query" aria-label="Seed artist or query"><select aria-label="Source"><option>Search history</option><option>Playlist</option><option>MusicBrainz</option></select><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Build graph</button></form>"#.to_string(),
        RouteKind::PlaylistIntake => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Artist - Title" aria-label="Playlist text"><select aria-label="Acquisition profile"><option>Balanced</option><option>Lossless exact</option></select><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Preview playlist</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Queue plans</button></form>"#.to_string(),
        RouteKind::Wishlist => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Wanted search" aria-label="Wanted search"><label><input type="checkbox" checked> Enabled</label><label><input type="checkbox"> Auto-download</label><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Add wanted search</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Run selected</button></form>"#.to_string(),
        RouteKind::Downloads => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Download</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Clear completed</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Enable acceleration</button></div>"#.to_string(),
        RouteKind::Uploads => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="2">Clear completed</button><button type="button" class="slskr-toolbar-command">Allow selected</button><button type="button" class="slskr-toolbar-command">Deny selected</button></div>"#.to_string(),
        RouteKind::Messages => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Username" aria-label="Username"><input class="slskr-toolbar-input" value="" placeholder="Message" aria-label="Message"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Reply</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Acknowledge</button></form>"#.to_string(),
        RouteKind::Rooms => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Room" aria-label="Room"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Join</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Leave</button></form>"#.to_string(),
        RouteKind::Users => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Username" aria-label="Username"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="1">Watch</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Save note</button><button type="button" class="slskr-toolbar-command">Browse</button></form>"#.to_string(),
        RouteKind::Contacts => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Contact username" aria-label="Contact username"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Add contact</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Edit note</button></form>"#.to_string(),
        RouteKind::Solid => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary">Connect identity</button><button type="button" class="slskr-toolbar-command">Sync storage</button><button type="button" class="slskr-toolbar-command">Refresh session</button></div>"#.to_string(),
        RouteKind::Collections => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Collection name" aria-label="Collection name"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Create collection</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="4">Add item</button></form>"#.to_string(),
        RouteKind::ShareGroups => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Group name" aria-label="Group name"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="1">Create group</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Add member</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Issue token</button></form>"#.to_string(),
        RouteKind::SharedWithMe => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary">Open collection</button><button type="button" class="slskr-toolbar-command">Copy token</button></div>"#.to_string(),
        RouteKind::Browse => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Username" aria-label="Username"><input class="slskr-toolbar-input" value="/" aria-label="Folder"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Browse</button><button type="button" class="slskr-toolbar-command">Download selected</button></form>"#.to_string(),
        RouteKind::System => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Connect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Disconnect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Rescan shares</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Vacuum database</button></div>"#.to_string(),
    }
}

pub(super) fn route_workflow_html(path: &str, responses: Option<&[EndpointBody]>) -> String {
    let kind = route_kind(path);
    let _tabs = match kind {
        RouteKind::Search => vec!["Results", "Searches", "Planner"],
        RouteKind::DiscoveryGraph => vec!["Graph", "Recommendations", "Review"],
        RouteKind::PlaylistIntake => vec!["Parser", "Rows", "Plans"],
        RouteKind::Wishlist => vec!["Wanted", "Review", "History"],
        RouteKind::Downloads => vec!["Active", "Queued", "Completed", "Failed"],
        RouteKind::Uploads => vec!["Active", "Queued", "Completed", "Policy"],
        RouteKind::Messages => vec!["Conversations", "Thread", "Pods"],
        RouteKind::Rooms => vec!["Joined", "Available", "Activity"],
        RouteKind::Users => vec!["Directory", "Detail", "Notes"],
        RouteKind::Contacts => vec!["Contacts", "Groups", "Invites"],
        RouteKind::Solid => vec!["Identity", "Storage", "Sync"],
        RouteKind::Collections => vec!["Collections", "Items", "Sharing"],
        RouteKind::ShareGroups => vec!["Groups", "Members", "Tokens"],
        RouteKind::SharedWithMe => vec!["Inbound", "Tokens", "Owners"],
        RouteKind::Browse => vec!["Tree", "Files", "Queue"],
        RouteKind::System => vec!["Connection", "Shares", "Storage", "Logs"],
    };
    let (
        _primary_title,
        _primary_detail,
        _table_headers,
        _legacy_rows,
        _side_title,
        _side_body,
    ) =
        match kind {
        RouteKind::Search => (
            "Grouped results",
            "Ranked peers with duplicate folding, warnings, and download review.",
            vec!["File", "Peer and score", "Action"],
            vec![
                ("01 Public Domain Theme.flac", "Archive Artist / Open Sessions", "peer1 / 94 / free slot", "Download"),
                ("02 Live Room Take.mp3", "Archive Artist / Broadcast", "peer2 / 71 / queue 2", "Preview"),
            ],
            "Search planner",
            "Select a result to review score reasons, duplicate groups, locked files, and the exact download action before queueing.",
        ),
        RouteKind::DiscoveryGraph => (
            "Discovery graph",
            "Seed an artist, album, track, or query and expand nearby searches.",
            vec!["Node", "Relationship", "Action"],
            vec![
                ("Archive Artist", "artist seed", "12 neighbors", "Expand"),
                ("Open Sessions", "album candidate", "lossless profile", "Search"),
            ],
            "Review queue",
            "Recommended next searches are staged here with acquisition profile and source-provider context.",
        ),
        RouteKind::PlaylistIntake => (
            "Playlist parser",
            "Paste or upload playlist text, validate rows, and queue searches.",
            vec!["Parsed row", "Classification", "Action"],
            vec![
                ("Archive Artist - Public Domain Theme", "track / valid", "balanced", "Queue search"),
                ("Unknown entry", "needs review", "missing artist", "Fix row"),
            ],
            "Import validation",
            "Row-level errors stay visible until every item has a title, artist or query, and acquisition profile.",
        ),
        RouteKind::Wishlist => (
            "Wanted searches",
            "Persistent searches with review state and optional automatic downloads.",
            vec!["Search", "State", "Action"],
            vec![
                ("public domain jazz", "enabled / manual review", "last run pending", "Run"),
                ("archive live set flac", "enabled / auto-download off", "0 results", "Review"),
            ],
            "Discovery inbox",
            "Send selected wanted searches to acquisition review, inspect quota, and approve reruns.",
        ),
        RouteKind::Downloads => (
            "Download queue",
            "Active, queued, completed, and failed downloads with progress controls.",
            vec!["File", "Progress", "Action"],
            vec![
                ("Open Sessions/01 Theme.flac", "peer1 / 42% / 1.2 MB/s", "ETA 03:10", "Cancel"),
                ("Broadcast/02 Take.mp3", "peer2 / queued", "slot pending", "Retry"),
            ],
            "Transfer controls",
            "Aggregate speed, active-slot limits, retry, cancel, and remove actions live here. Uploads are kept on the Uploads page.",
        ),
        RouteKind::Uploads => (
            "Upload queue",
            "Peer requests, progress, speed, and allow/deny state.",
            vec!["Request", "Progress", "Action"],
            vec![
                ("peer3 wants Theme.flac", "18% / 420 KB/s", "allow list", "Deny"),
                ("peer4 wants Notes.txt", "queued", "waiting", "Allow"),
            ],
            "Upload policy",
            "Review sharing policy, active upload slots, and clear completed uploads without download queue noise.",
        ),
        RouteKind::Messages => (
            "Conversations",
            "Two-pane private messenger with unread state and compose actions.",
            vec!["Thread", "Last message", "Action"],
            vec![
                ("peer1", "unread / today", "Can you browse my folder?", "Reply"),
                ("peer2", "read / yesterday", "Thanks", "Open"),
            ],
            "Selected thread",
            "Select a conversation or start one by username, then reply, acknowledge, search, or delete.",
        ),
        RouteKind::Rooms => (
            "Room activity",
            "Joined rooms, available rooms, users, and recent messages.",
            vec!["Room", "Activity", "Action"],
            vec![
                ("public-domain", "joined / 18 users", "2 new messages", "Open"),
                ("ambient", "available", "54 users", "Join"),
            ],
            "Compose",
            "Send room messages from the selected joined room and keep available-room browsing secondary.",
        ),
        RouteKind::Users => (
            "User directory",
            "Watched users with status, stats, notes, browse and message actions.",
            vec!["User", "Status", "Action"],
            vec![
                ("peer1", "online / privileged", "note saved", "Browse"),
                ("peer2", "away", "shared 1,240 files", "Message"),
            ],
            "User detail",
            "Readable info, presence, privileges, and endpoint data appear here after selecting a user.",
        ),
        RouteKind::Contacts => (
            "Contact manager",
            "Contacts, groups, nearby peers, invites, and notes.",
            vec!["Contact", "Group", "Action"],
            vec![
                ("peer1", "trusted / online", "note saved", "Message"),
                ("peer5", "nearby", "invite pending", "Accept"),
            ],
            "Contact detail",
            "Edit notes, browse, watch, remove, or invite from the selected contact context.",
        ),
        RouteKind::Solid => (
            "Solid status",
            "Identity, storage, session, linked-data sync, and setup controls.",
            vec!["Area", "State", "Action"],
            vec![
                ("Identity", "not connected", "WebID required", "Connect"),
                ("Storage", "pending", "no pod selected", "Configure"),
            ],
            "Related integrations",
            "Bridge, pods, source providers, and automation state stay secondary to Solid setup.",
        ),
        RouteKind::Collections => (
            "Collection library",
            "Create collections, inspect items, add or remove files, and share.",
            vec!["Collection", "Items", "Action"],
            vec![
                ("Open Sessions", "12 items", "private", "Open"),
                ("Radio Finds", "4 items", "shared", "Share"),
            ],
            "Item picker",
            "Browse library items here, then add selected files to the active collection.",
        ),
        RouteKind::ShareGroups => (
            "Share groups",
            "Groups, members, grants, tokens, and permissions.",
            vec!["Group", "Grant", "Action"],
            vec![
                ("Trusted peers", "read collections", "3 members", "Issue token"),
                ("Reviewers", "expires soon", "1 member", "Update"),
            ],
            "Permissions",
            "Add members, issue tokens, revoke grants, and adjust selected group access.",
        ),
        RouteKind::SharedWithMe => (
            "Inbound shares",
            "Collections, files, grants, tokens, owners, expiration, and access status.",
            vec!["Shared item", "Owner and access", "Action"],
            vec![
                ("Open Sessions", "peer1 / valid", "expires never", "Open"),
                ("Live Notes", "peer2 / token", "expires soon", "Copy token"),
            ],
            "Access detail",
            "Inspect owner, token, expiration, and leave or revoke where allowed.",
        ),
        RouteKind::Browse => (
            "Peer browser",
            "Enter a username, expand folders, filter files, and queue selected downloads.",
            vec!["Path", "Contents", "Action"],
            vec![
                ("/Music/Open Sessions", "12 files / 2 folders", "cached", "Open"),
                ("/Music/Open Sessions/Theme.flac", "24 MB", "selected", "Download"),
            ],
            "Download preview",
            "Selected files appear here before queueing so peers, paths, and sizes can be checked.",
        ),
        RouteKind::System => (
            "Operator dashboard",
            "Connection, shares, database, logs, preferences, and automation.",
            vec!["Area", "State", "Action"],
            vec![
                ("Connection", "server pending", "session unknown", "Connect"),
                ("Shares", "scan idle", "0 roots", "Rescan"),
                ("Database", "stats pending", "maintenance ready", "Vacuum"),
            ],
            "Logs and preferences",
            "Filter events, update preferences, and review automation from tabs without exposing raw metrics by default.",
        ),
    };
    // A pending or failed live request must never be presented as daemon
    // data.  The old fallback rendered bundled demo rows whenever a probe
    // failed, which made an empty/disconnected instance look populated and
    // caused row actions to target placeholder IDs.  Keep the legacy sample
    // tuple above as documentation for the route shape, but only render rows
    // decoded from actual responses.
    let table_rows = route_dynamic_rows(kind, responses).unwrap_or_default();
    format!(
        r#"<div class="slskr-workflow" data-slskr-route-kind="{kind:?}">{reference}{native}</div>"#,
        kind = kind,
        reference = route_reference_panel_html(kind),
        native = route_native_workspace_html(kind, &table_rows, responses),
    )
}

pub fn route_workspace_pending_html(path: &str) -> String {
    route_workflow_html(path, None)
}

pub fn route_workspace_result_html(path: &str, responses: &[EndpointBody]) -> String {
    route_workflow_html(path, Some(responses))
}

pub(super) fn experience_settings_panel_html() -> String {
    let groups = ["Search", "Discovery", "Player", "Messages"];
    let sections = groups
        .iter()
        .map(|group| {
            let fields = experience_preferences()
                .iter()
                .filter(|preference| preference.group == *group)
                .map(|preference| {
                    if preference.input == "checkbox" {
                        format!(
                            r#"<label class="slskr-local-check"><input type="checkbox" data-slskr-pref="{id}" data-slskr-pref-default="{default}" {checked}>{label}</label>"#,
                            id = escape_html(preference.id),
                            default = escape_html(preference.default_value),
                            checked = if preference.default_value == "true" { "checked" } else { "" },
                            label = escape_html(preference.label),
                        )
                    } else {
                        format!(
                            r#"<label><span>{label}</span><input type="text" data-slskr-pref="{id}" data-slskr-pref-default="{default}" value="{default}"></label>"#,
                            id = escape_html(preference.id),
                            default = escape_html(preference.default_value),
                            label = escape_html(preference.label),
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            format!(
                r#"<fieldset><legend>{group}</legend>{fields}</fieldset>"#,
                group = escape_html(group),
                fields = fields
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<article class="slskr-data-card slskr-local-panel" data-slskr-experience-panel><header><div><h3>Experience Preferences</h3><code>browser local</code></div><span id="slskr-experience-summary">Rust owned</span></header><form class="slskr-local-form">{sections}</form><div class="slskr-local-actions"><button type="button" data-slskr-pref-action="save">Save</button><button type="button" data-slskr-pref-action="reset">Reset</button><button type="button" data-slskr-pref-action="copy">Copy Report</button></div><pre id="slskr-experience-report"></pre><p id="slskr-experience-status" aria-live="polite"></p></article>"#,
        sections = sections
    )
}

pub(super) fn automation_center_panel_html() -> String {
    let recipes = automation_recipes()
        .iter()
        .map(|recipe| {
            format!(
                r#"<li data-slskr-recipe="{id}"><div><strong>{title}</strong><span>{description}</span></div><label class="slskr-local-check"><input type="checkbox" data-slskr-recipe-enabled="{id}" {checked}>Enabled</label><dl><dt>Cadence</dt><dd>{cadence}</dd><dt>Cooldown</dt><dd>{cooldown}</dd><dt>Network</dt><dd>{network}</dd><dt>Files</dt><dd>{files}</dd><dt>Approval</dt><dd>{approval}</dd></dl><div class="slskr-local-actions"><button type="button" data-slskr-recipe-dry-run="{id}">Dry Run</button><button type="button" data-slskr-recipe-copy="{id}">Copy Plan</button></div></li>"#,
                id = escape_html(recipe.id),
                title = escape_html(recipe.title),
                description = escape_html(recipe.description),
                checked = if recipe.enabled_by_default { "checked" } else { "" },
                cadence = escape_html(recipe.cadence),
                cooldown = escape_html(recipe.cooldown),
                network = escape_html(recipe.network_impact),
                files = escape_html(recipe.file_impact),
                approval = escape_html(recipe.approval_gate),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<article class="slskr-data-card slskr-local-panel" data-slskr-automation-panel><header><div><h3>Automation Center</h3><code>browser local</code></div><span id="slskr-automation-summary">7 recipes</span></header><div class="slskr-local-actions"><button type="button" data-slskr-automation-action="copy-history">Copy History</button><button type="button" data-slskr-automation-action="reset">Reset</button></div><ul class="slskr-recipe-list">{recipes}</ul><pre id="slskr-automation-report"></pre><p id="slskr-automation-status" aria-live="polite"></p></article>"#,
        recipes = recipes
    )
}

#[allow(dead_code)]
pub(super) fn route_toolbar_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return String::new();
    };
    match page.surface {
        "search" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Search text" aria-label="Search text"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Search</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Stop</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Clear</button></form>"#.to_string(),
        "transfers" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Filename" aria-label="Filename"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Queue file</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Clear downloads</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Clear uploads</button></form>"#.to_string(),
        "messages" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Message" aria-label="Message"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Send</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Acknowledge</button></form>"#.to_string(),
        "rooms" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Room" aria-label="Room"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Join</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Send</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Leave</button></form>"#.to_string(),
        "browse" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" aria-label="Directory" placeholder="Directory"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Request directory</button></form>"#.to_string(),
        "identity" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Username" aria-label="Username"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Watch</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Add contact</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Note</button></form>"#.to_string(),
        "collections" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Collection name" aria-label="Name"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Create collection</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Create group</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Share</button></form>"#.to_string(),
        "integrations" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" placeholder="Playlist text" aria-label="Playlist text"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Preview playlist</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Discovery graph</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Queue job</button></form>"#.to_string(),
        "system" => r#"<div class="slskr-toolbar"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Connect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Disconnect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Rescan shares</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Vacuum database</button></div>"#.to_string(),
        "wishlist" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="public domain jazz" aria-label="Wishlist text"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Add</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Run search</button></form>"#.to_string(),
        _ => String::new(),
    }
}

pub fn route_page_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return route_page_html("/searches");
    };
    let kind = route_kind(path);
    let endpoints = route_endpoints(page.surface)
        .iter()
        .map(|endpoint| {
            format!(
                r#"<li><strong>{method}</strong><code>{path}</code><span>{surface}</span></li>"#,
                method = endpoint.method,
                path = endpoint_url(endpoint.path),
                surface = endpoint.surface
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let route_inventory = ui_routes()
        .iter()
        .filter(|route| route.title == page.title || route.path == page.path)
        .map(|route| {
            format!(
                r#"<li><strong>{nav}</strong><code>{path}</code><span>{title}</span></li>"#,
                nav = if route.nav { "nav" } else { "route" },
                path = route.path,
                title = route.title
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let chips = match kind {
        RouteKind::Search => [
            status_chip_html("Network", "ready"),
            status_chip_html("Ranking", "balanced"),
        ]
        .join(""),
        RouteKind::Downloads => [
            status_chip_html("Queue", "downloads"),
            status_chip_html("Slots", "auto"),
        ]
        .join(""),
        RouteKind::Uploads => [
            status_chip_html("Queue", "uploads"),
            status_chip_html("Policy", "active"),
        ]
        .join(""),
        RouteKind::System => [
            status_chip_html("Daemon", "checking"),
            status_chip_html("Events", "live"),
        ]
        .join(""),
        _ => [
            status_chip_html("Workspace", "ready"),
            status_chip_html("Review", "manual"),
        ]
        .join(""),
    };
    format!(
        r#"<section class="slskr-route-page" data-route="{path}" data-slskr-refresh-scope="active-route"><header class="slskr-page-header"><div><p class="slskr-kicker">{surface}</p><h2>{title}</h2><p>{description}</p></div><div class="slskr-page-status">{chips}</div></header>{toolbar}<div class="slskr-route-summary"><h3>Overview</h3><ul id="slskr-route-summary">{summary}</ul></div><section class="slskr-work-area" data-slskr-lazy-workspace="true"><header><div><h3>Workspace</h3><span id="slskr-live-status" aria-live="polite">Workflow data refreshes from the daemon</span></div><div class="slskr-live-controls"><button type="button" data-slskr-refresh-route>Refresh</button><button type="button" data-slskr-focus-filter>Filter</button><button type="button" data-slskr-clear-filters>Clear filters</button></div></header><div id="slskr-page-data" class="slskr-page-data" data-slskr-live-state="pending">{page_data}</div><p id="slskr-action-status" aria-live="polite"></p><div id="slskr-toast-region" class="slskr-toast-region" aria-live="polite"></div></section><details class="slskr-diagnostics" data-slskr-lazy-diagnostics="true"><summary>Developer</summary><div class="slskr-route-actions"><h3>Action wiring</h3><ul id="slskr-route-actions">{actions}</ul></div><div class="slskr-route-columns"><div><h3>Route Shape</h3><ul>{routes}</ul></div><div><h3>API Surface</h3><ul>{endpoints}</ul></div></div><div class="slskr-route-live"><h3>Raw Probe Status</h3><ul id="slskr-route-data" data-slskr-live-state="pending">{route_data}</ul></div></details></section>"#,
        path = escape_html(path),
        surface = escape_html(page.surface),
        title = escape_html(page.title),
        description = escape_html(page.description),
        chips = chips,
        toolbar = route_workflow_toolbar_html(kind),
        summary = route_workflow_stats_html(kind, None),
        routes = route_inventory,
        endpoints = endpoints,
        actions = route_actions_html(path),
        page_data = route_workspace_pending_html(path),
        route_data = route_probe_pending_html(path),
    )
}
