//! Native Web route reference panels.

use super::*;

pub(super) fn reference_field_html(label: &str, placeholder: &str) -> String {
    format!(
        r#"<label><span>{label}</span><input type="text" placeholder="{placeholder}" aria-label="{label}"></label>"#,
        label = escape_html(label),
        placeholder = escape_html(placeholder),
    )
}

pub(super) fn reference_buttons_html(labels: &[&str]) -> String {
    labels
        .iter()
        .map(|label| {
            format!(
                r#"<button type="button" data-slskr-reference-action="{label}">{label}</button>"#,
                label = escape_html(label),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn route_component_parity_attrs(kind: RouteKind) -> &'static str {
    match kind {
        RouteKind::Search => r#" data-react-component="Searches""#,
        RouteKind::DiscoveryGraph => {
            r#" data-react-component="DiscoveryGraphAtlasPage" data-testid="discovery-graph-atlas""#
        }
        RouteKind::PlaylistIntake => r#" data-react-component="PlaylistIntake""#,
        RouteKind::Wishlist => r#" data-react-component="Wishlist""#,
        RouteKind::Downloads => r#" data-react-component="Transfers" data-testid="downloads""#,
        RouteKind::Uploads => r#" data-react-component="Transfers" data-testid="uploads""#,
        RouteKind::Messages | RouteKind::Rooms => r#" data-react-component="Messaging""#,
        RouteKind::Users => r#" data-react-component="Users""#,
        RouteKind::Contacts => r#" data-react-component="Contacts""#,
        RouteKind::Solid => r#" data-react-component="SolidSettings" data-testid="solid-root""#,
        RouteKind::Collections => r#" data-react-component="Collections""#,
        RouteKind::ShareGroups => r#" data-react-component="ShareGroups""#,
        RouteKind::SharedWithMe => r#" data-react-component="SharedWithMe""#,
        RouteKind::Browse => r#" data-react-component="Browse""#,
        RouteKind::System => r#" data-react-component="System""#,
    }
}

pub(super) fn route_component_parity_class(kind: RouteKind) -> &'static str {
    match kind {
        RouteKind::Search => "searches view",
        RouteKind::DiscoveryGraph => "view discovery-graph-atlas-page",
        RouteKind::PlaylistIntake => "playlist-intake",
        RouteKind::Wishlist => "wishlist",
        RouteKind::Downloads => "transfers transfers-downloads",
        RouteKind::Uploads => "transfers transfers-uploads",
        RouteKind::Messages | RouteKind::Rooms => "messaging-workspace",
        RouteKind::Users => "users",
        RouteKind::Contacts => "contacts",
        RouteKind::Solid => "solid-settings",
        RouteKind::Collections => "collections",
        RouteKind::ShareGroups => "sharegroups",
        RouteKind::SharedWithMe => "shared-with-me",
        RouteKind::Browse => "browse",
        RouteKind::System => "system",
    }
}

pub(super) type RouteReferenceSpec<'a> = (
    &'a str,
    &'a str,
    Vec<(&'a str, &'a str)>,
    Vec<&'a str>,
    Vec<&'a str>,
);

pub(super) fn route_reference_panel_html(kind: RouteKind) -> String {
    let (title, detail, fields, buttons, facts): RouteReferenceSpec<'_> = match kind {
            RouteKind::Search => (
                "Search",
                "Search phrase, acquisition profile, queue search, and open results.",
                vec![
                    ("Search phrase", "Search phrase"),
                    ("Acquisition profile", "Balanced"),
                ],
                vec!["Queue Search", "Search and Open Results"],
                vec!["Result review", "Duplicate folding", "Download preview"],
            ),
            RouteKind::DiscoveryGraph => (
                "Discovery Graph Atlas",
                "Persistent graph surface for wandering the neighborhood around a seed without opening a modal.",
                vec![
                    ("Seed Scope", "Song / Unknown Seed"),
                    ("Artist Name", "Artist Name"),
                    ("Album Title", "Album Title"),
                    ("Track Title or Seed Label", "Track Title or Seed Label"),
                    ("Optional Artist ID", "Optional Artist ID"),
                    ("Optional Release ID", "Optional Release ID"),
                    ("Optional Recording ID", "Optional Recording ID"),
                ],
                vec!["Build Atlas", "Queue Nearby"],
                vec!["Artist Name", "Depth 2", "Weight 20", "Saved branches"],
            ),
            RouteKind::PlaylistIntake => (
                "Playlist Intake Import playlist text for review before any provider or network activity.",
                "Import playlist text for review before any provider or network activity.",
                vec![
                    ("Name", "Road trip, label sampler, friend recs"),
                    ("Source", "Local file name or provider URL"),
                    (
                        "Playlist rows",
                        "Artist - Title, one row per track, or simple CSV artist,title",
                    ),
                ],
                vec!["Import Playlist"],
                vec![
                    "Playlist Intake Import playlist text for review before any provider or network activity.",
                    "Playlists 0",
                    "Tracks 0",
                    "Unmatched 0",
                ],
            ),
            RouteKind::Wishlist => (
                "Wishlist Saved searches that run automatically",
                "Saved searches that run automatically.",
                vec![
                    ("Search Text", "Enter search terms..."),
                    ("Filter (optional)", "e.g., flac OR mp3"),
                    ("Max Results", "25"),
                ],
                vec!["Add Search", "Import List", "Copy Review", "Run Enabled", "Add Your First Search"],
                vec![
                    "Wishlist Saved searches that run automatically",
                    "Request Portal Summary Operator view of wanted music before acquisition jobs are scheduled.",
                    "Requests 0",
                    "Enabled 0",
                    "Automatic 0",
                    "Needs Review 0",
                    "Within quota 25 left",
                ],
            ),
            RouteKind::Downloads => (
                "Downloads",
                "Transfer queue for incoming files.",
                Vec::new(),
                vec!["Retry", "Cancel", "Remove", "Clear Completed"],
                vec!["No downloads to display"],
            ),
            RouteKind::Uploads => (
                "Uploads",
                "Transfer queue for files requested by peers.",
                Vec::new(),
                vec!["Allow", "Deny", "Clear Completed"],
                vec!["No uploads to display"],
            ),
            RouteKind::Messages => (
                "Messages",
                "Unified direct messages, saved chats, joined rooms, and pod channels.",
                vec![
                    ("Chat username", "username"),
                    ("Search rooms", "Search rooms"),
                    ("Message", "Message"),
                ],
                vec!["Direct Message", "Join Room", "Open Batch Private-Message Dialog", "Collapse All Message Panels"],
                vec!["Saved Chats 0", "Joined Rooms 0", "Pod Channels 0", "Workspace 0 open"],
            ),
            RouteKind::Rooms => (
                "Messages",
                "Room-focused message workspace.",
                vec![("Search rooms", "Search rooms")],
                vec!["Join Room", "Leave Room"],
                vec!["Joined Rooms 0", "Workspace 0 open"],
            ),
            RouteKind::Users => (
                "Users",
                "Peer user lookup and detail.",
                vec![("Username", "Username")],
                vec!["Search for User", "Clear Selected User", "Browse", "Message"],
                vec!["No user info to display"],
            ),
            RouteKind::Contacts => (
                "Contacts Manage your peer contacts",
                "Manage your peer contacts.",
                vec![
                    ("Invite", "slskr://invite/..."),
                    ("Nickname", "Friend's name"),
                ],
                vec!["Create Invite", "Add Friend", "Refresh Nearby", "Message", "Browse", "Remove"],
                vec!["Contacts Manage your peer contacts", "All Contacts", "Nearby"],
            ),
            RouteKind::Solid => (
                "Solid",
                "Solid integration status, identity, storage, and WebID resolution.",
                vec![("WebID", "https://example.com/profile/card#me")],
                vec!["Resolve WebID", "Connect Identity", "Sync Storage"],
                vec!["Live Solid status is shown in the workspace below."],
            ),
            RouteKind::Collections => (
                "Collections Manage your playlists and share lists",
                "Manage your playlists and share lists.",
                vec![
                    ("Title", "Enter collection title"),
                    ("Description", "Optional description"),
                    ("Search for item", "Search by filename, artist, or title"),
                ],
                vec!["Create Collection", "Add Item", "Share", "Create Collection"],
                vec![
                    "Collections Manage your playlists and share lists",
                    "No collections yet",
                    "Title",
                    "Type",
                    "Items",
                    "Actions",
                ],
            ),
            RouteKind::ShareGroups => (
                "Share Groups Manage groups for sharing collections",
                "Manage groups for sharing collections.",
                vec![
                    ("Group Name", "Enter group name"),
                    ("Soulseek Username", "Enter username"),
                ],
                vec!["Create Group", "Create Your First Group", "Add Member", "Issue Token"],
                vec![
                    "Share Groups Manage groups for sharing collections",
                    "No share groups yet",
                    "Name",
                    "Members",
                    "Created",
                    "Actions",
                ],
            ),
            RouteKind::SharedWithMe => (
                "Shared with Me Collections shared with you",
                "Collections shared with you.",
                Vec::new(),
                vec!["Open", "Stream", "Backfill"],
                vec![
                    "Shared with Me Collections shared with you",
                    "No shares yet",
                    "Collection",
                    "Shared By",
                    "Type",
                    "Permissions",
                    "Actions",
                ],
            ),
            RouteKind::Browse => (
                "Browse",
                "Tabbed peer browse sessions.",
                vec![("Username", "Username")],
                vec!["Open a New Browse Tab", "Download Selected"],
                vec!["New Tab"],
            ),
            RouteKind::System => (
                "System",
                "Operator status, network, shares, jobs, automation, files, data, events, logs, and metrics.",
                Vec::new(),
                vec!["Check for Updates", "Get Privileges", "Diagnostic Bundle", "Setup Health"],
                vec![
                    "Info", "Network", "Mesh", "Bridge", "MediaCore", "Security Policies",
                    "Experience", "Integrations", "Options", "Shares", "Jobs", "Automations",
                    "Source Providers", "Swarm Analytics", "Library Health", "Quarantine Jury",
                    "Files", "Data", "Events", "Logs", "Metrics",
                ],
            ),
        };

    let field_html = fields
        .iter()
        .map(|(label, placeholder)| reference_field_html(label, placeholder))
        .collect::<Vec<_>>()
        .join("");
    let facts_html = facts
        .iter()
        .map(|fact| format!(r#"<span>{}</span>"#, escape_html(fact)))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<section class="slskr-reference-panel {component_class}" data-slskr-parity-reference{attrs}><header><div><p class="slskr-kicker">protocol compatibility</p><h2>{title}</h2><p>{detail}</p></div><div class="slskr-reference-actions">{buttons}</div></header><form class="slskr-reference-form">{fields}</form><div class="slskr-reference-facts">{facts}</div></section>"#,
        component_class = escape_html(route_component_parity_class(kind)),
        attrs = route_component_parity_attrs(kind),
        title = escape_html(title),
        detail = escape_html(detail),
        buttons = reference_buttons_html(&buttons),
        fields = field_html,
        facts = facts_html,
    )
}
