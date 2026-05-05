use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavItem {
    pub href: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiRoute {
    pub nav: bool,
    pub path: &'static str,
    pub title: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppSection {
    pub description: &'static str,
    pub endpoint: &'static str,
    pub title: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiEndpoint {
    pub method: &'static str,
    pub path: &'static str,
    pub surface: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeProbe {
    pub label: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteAction {
    pub body: ActionBody,
    pub label: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub surface: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionBody {
    None,
    BrowseDirectory,
    CollectionItem,
    ConversationMessage,
    DownloadFiles,
    EnabledFalse,
    EnabledTrue,
    FeedPreview,
    JsonString,
    NameDescription,
    Permissions,
    RoomMessage,
    SearchText,
    ShareGrant,
    ShareGroupMember,
    Username,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoutePage {
    pub description: &'static str,
    pub path: &'static str,
    pub surface: &'static str,
    pub title: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointBody {
    pub endpoint: ApiEndpoint,
    pub body: String,
}

pub const fn api_base_path() -> &'static str {
    "/api/v0"
}

pub const fn ui_routes() -> &'static [UiRoute] {
    &[
        UiRoute {
            nav: false,
            path: "/",
            title: "Search",
        },
        UiRoute {
            nav: true,
            path: "/searches",
            title: "Search",
        },
        UiRoute {
            nav: false,
            path: "/searches/:id",
            title: "Search Detail",
        },
        UiRoute {
            nav: true,
            path: "/discovery-graph",
            title: "Discovery Graph",
        },
        UiRoute {
            nav: true,
            path: "/playlist-intake",
            title: "Playlist Intake",
        },
        UiRoute {
            nav: true,
            path: "/wishlist",
            title: "Wishlist",
        },
        UiRoute {
            nav: true,
            path: "/downloads",
            title: "Downloads",
        },
        UiRoute {
            nav: true,
            path: "/uploads",
            title: "Uploads",
        },
        UiRoute {
            nav: true,
            path: "/messages",
            title: "Messages",
        },
        UiRoute {
            nav: true,
            path: "/chat",
            title: "Chat",
        },
        UiRoute {
            nav: true,
            path: "/rooms",
            title: "Rooms",
        },
        UiRoute {
            nav: true,
            path: "/users",
            title: "Users",
        },
        UiRoute {
            nav: true,
            path: "/contacts",
            title: "Contacts",
        },
        UiRoute {
            nav: true,
            path: "/solid",
            title: "Solid",
        },
        UiRoute {
            nav: true,
            path: "/collections",
            title: "Collections",
        },
        UiRoute {
            nav: true,
            path: "/sharegroups",
            title: "Share Groups",
        },
        UiRoute {
            nav: true,
            path: "/shared",
            title: "Shared With Me",
        },
        UiRoute {
            nav: true,
            path: "/browse",
            title: "Browse",
        },
        UiRoute {
            nav: true,
            path: "/system",
            title: "System",
        },
        UiRoute {
            nav: false,
            path: "/system/:tab",
            title: "System Tab",
        },
        UiRoute {
            nav: false,
            path: "/pods",
            title: "Pods",
        },
        UiRoute {
            nav: false,
            path: "/pods/:podId",
            title: "Pod Redirect",
        },
        UiRoute {
            nav: false,
            path: "/pods/:podId/channels/:channelId",
            title: "Pod Channel Redirect",
        },
    ]
}

pub const fn nav_items() -> &'static [NavItem] {
    &[
        NavItem {
            href: "/searches",
            icon: "search",
            label: "Search",
        },
        NavItem {
            href: "/discovery-graph",
            icon: "graph",
            label: "Discovery Graph",
        },
        NavItem {
            href: "/playlist-intake",
            icon: "list",
            label: "Playlist Intake",
        },
        NavItem {
            href: "/wishlist",
            icon: "star",
            label: "Wishlist",
        },
        NavItem {
            href: "/downloads",
            icon: "download",
            label: "Downloads",
        },
        NavItem {
            href: "/uploads",
            icon: "upload",
            label: "Uploads",
        },
        NavItem {
            href: "/messages",
            icon: "message",
            label: "Messages",
        },
        NavItem {
            href: "/users",
            icon: "user",
            label: "Users",
        },
        NavItem {
            href: "/contacts",
            icon: "address",
            label: "Contacts",
        },
        NavItem {
            href: "/solid",
            icon: "key",
            label: "Solid",
        },
        NavItem {
            href: "/collections",
            icon: "collection",
            label: "Collections",
        },
        NavItem {
            href: "/sharegroups",
            icon: "group",
            label: "Share Groups",
        },
        NavItem {
            href: "/shared",
            icon: "share",
            label: "Shared with Me",
        },
        NavItem {
            href: "/browse",
            icon: "folder",
            label: "Browse",
        },
        NavItem {
            href: "/system",
            icon: "settings",
            label: "System",
        },
    ]
}

pub const fn app_sections() -> &'static [AppSection] {
    &[
        AppSection {
            description: "Create searches, review result counts, and open discovery context.",
            endpoint: "/searches",
            title: "Search",
        },
        AppSection {
            description: "Navigate release, track, artist, and query neighborhoods.",
            endpoint: "/discovery-graph",
            title: "Discovery Graph",
        },
        AppSection {
            description: "Import playlist inputs and stage them before search or library actions.",
            endpoint: "/source-feed-imports/preview",
            title: "Playlist Intake",
        },
        AppSection {
            description: "Persist wanted search intents and rerun them from one place.",
            endpoint: "/wishlist",
            title: "Wishlist",
        },
        AppSection {
            description: "Track downloads and uploads with queue, speed, and status state.",
            endpoint: "/transfers",
            title: "Transfers",
        },
        AppSection {
            description: "Read private messages, room activity, and acknowledgement state.",
            endpoint: "/messages",
            title: "Messages",
        },
        AppSection {
            description: "Inspect joined rooms, available rooms, users, and recent messages.",
            endpoint: "/rooms",
            title: "Rooms",
        },
        AppSection {
            description: "Request peer browse data and display cached shared folders.",
            endpoint: "/browse",
            title: "Browse",
        },
        AppSection {
            description: "Manage contacts, notes, groups, shared collections, and peer context.",
            endpoint: "/contacts",
            title: "Identity",
        },
        AppSection {
            description: "Track local collections, share grants, and shared-with-me records.",
            endpoint: "/collections",
            title: "Collections",
        },
        AppSection {
            description:
                "Configure external services, media automation, source feeds, and metadata.",
            endpoint: "/integrations",
            title: "Integrations",
        },
        AppSection {
            description:
                "Show daemon health, version, metrics, telemetry, and configuration state.",
            endpoint: "/telemetry",
            title: "System",
        },
    ]
}

pub const fn api_endpoints() -> &'static [ApiEndpoint] {
    &[
        ApiEndpoint {
            method: "GET",
            path: "/application",
            surface: "application",
        },
        ApiEndpoint {
            method: "GET",
            path: "/application/build",
            surface: "application",
        },
        ApiEndpoint {
            method: "GET",
            path: "/server",
            surface: "session",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/server",
            surface: "session",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/server",
            surface: "session",
        },
        ApiEndpoint {
            method: "GET",
            path: "/searches",
            surface: "search",
        },
        ApiEndpoint {
            method: "GET",
            path: "/searches/records",
            surface: "search",
        },
        ApiEndpoint {
            method: "POST",
            path: "/searches",
            surface: "search",
        },
        ApiEndpoint {
            method: "GET",
            path: "/searches/:id/responses",
            surface: "search",
        },
        ApiEndpoint {
            method: "GET",
            path: "/soulseek/interests",
            surface: "search",
        },
        ApiEndpoint {
            method: "POST",
            path: "/soulseek/interests",
            surface: "search",
        },
        ApiEndpoint {
            method: "GET",
            path: "/soulseek/hated-interests",
            surface: "search",
        },
        ApiEndpoint {
            method: "POST",
            path: "/soulseek/hated-interests",
            surface: "search",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/searches/:id",
            surface: "search",
        },
        ApiEndpoint {
            method: "GET",
            path: "/wishlist",
            surface: "wishlist",
        },
        ApiEndpoint {
            method: "POST",
            path: "/wishlist",
            surface: "wishlist",
        },
        ApiEndpoint {
            method: "POST",
            path: "/wishlist/:id/search",
            surface: "wishlist",
        },
        ApiEndpoint {
            method: "GET",
            path: "/transfers/downloads",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "GET",
            path: "/transfers/uploads",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "GET",
            path: "/transfers/speeds",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "POST",
            path: "/transfers/downloads/:username",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/transfers/downloads/all/completed",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/transfers/uploads/all/completed",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/transfers/downloads/accelerated",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "GET",
            path: "/rooms/available",
            surface: "rooms",
        },
        ApiEndpoint {
            method: "GET",
            path: "/rooms/joined",
            surface: "rooms",
        },
        ApiEndpoint {
            method: "POST",
            path: "/rooms/joined",
            surface: "rooms",
        },
        ApiEndpoint {
            method: "POST",
            path: "/rooms/joined/:roomName/messages",
            surface: "rooms",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/rooms/joined/:roomName",
            surface: "rooms",
        },
        ApiEndpoint {
            method: "GET",
            path: "/conversations",
            surface: "messages",
        },
        ApiEndpoint {
            method: "GET",
            path: "/conversations/:username",
            surface: "messages",
        },
        ApiEndpoint {
            method: "POST",
            path: "/conversations/:username",
            surface: "messages",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/conversations/:username",
            surface: "messages",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/conversations/:username",
            surface: "messages",
        },
        ApiEndpoint {
            method: "GET",
            path: "/pods",
            surface: "messages",
        },
        ApiEndpoint {
            method: "GET",
            path: "/users/:username/browse",
            surface: "browse",
        },
        ApiEndpoint {
            method: "POST",
            path: "/users/:username/directory",
            surface: "browse",
        },
        ApiEndpoint {
            method: "GET",
            path: "/users",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/users/:username/info",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/users/:username/status",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/users/:username/endpoint",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/contacts",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/contacts/nearby",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/users/notes",
            surface: "identity",
        },
        ApiEndpoint {
            method: "POST",
            path: "/users/notes",
            surface: "identity",
        },
        ApiEndpoint {
            method: "POST",
            path: "/users/watch",
            surface: "identity",
        },
        ApiEndpoint {
            method: "POST",
            path: "/contacts/from-discovery",
            surface: "identity",
        },
        ApiEndpoint {
            method: "POST",
            path: "/contacts/from-invite",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/collections",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/collections",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/sharegroups",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/sharegroups",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/shared",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/share-grants",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/share-grants",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/share-grants/by-collection/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/share-grants/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/share-grants/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/share-grants/:id/backfill",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/share-grants/:id/token",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/sharegroups/:id/members",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/shares/catalog",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/shares",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/shares/rescan",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/items",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/items/browser",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/library/items",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/files/downloads/directories",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/files/incomplete/directories",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/source-providers",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "POST",
            path: "/source-feed-imports/preview",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "POST",
            path: "/discovery-graph",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/source-feeds",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "POST",
            path: "/source-feeds",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/musicbrainz/albums/completion",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/musicbrainz/release-radar/subscriptions",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "POST",
            path: "/musicbrainz/release-radar/subscriptions",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "POST",
            path: "/musicbrainz/targets",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/songid/runs",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "POST",
            path: "/songid/runs",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/solid/status",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/pods",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/bridge/status",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/jobs",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "POST",
            path: "/jobs/discography",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/mesh/stats",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/security/dashboard",
            surface: "integrations",
        },
        ApiEndpoint {
            method: "GET",
            path: "/telemetry/metrics",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/telemetry/metrics/kpis",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/telemetry/reports/transfers/summary",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/options",
            surface: "system",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/options",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/events",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/logs",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/shares",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/database/stats",
            surface: "system",
        },
        ApiEndpoint {
            method: "POST",
            path: "/database/cleanup",
            surface: "system",
        },
        ApiEndpoint {
            method: "POST",
            path: "/database/vacuum",
            surface: "system",
        },
    ]
}

pub const fn runtime_probes() -> &'static [RuntimeProbe] {
    &[
        RuntimeProbe {
            label: "Health",
            path: "/health",
        },
        RuntimeProbe {
            label: "Version",
            path: "/version",
        },
        RuntimeProbe {
            label: "Application",
            path: "/application",
        },
        RuntimeProbe {
            label: "Server",
            path: "/server",
        },
    ]
}

pub const fn route_actions() -> &'static [RouteAction] {
    &[
        RouteAction {
            body: ActionBody::SearchText,
            label: "Start Search",
            method: "POST",
            path: "/searches",
            surface: "search",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Stop Search",
            method: "PUT",
            path: "/searches/:id",
            surface: "search",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Remove Search",
            method: "DELETE",
            path: "/searches/:id",
            surface: "search",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Clear Searches",
            method: "DELETE",
            path: "/searches",
            surface: "search",
        },
        RouteAction {
            body: ActionBody::JsonString,
            label: "Add Interest",
            method: "POST",
            path: "/soulseek/interests",
            surface: "search",
        },
        RouteAction {
            body: ActionBody::JsonString,
            label: "Add Hated Interest",
            method: "POST",
            path: "/soulseek/hated-interests",
            surface: "search",
        },
        RouteAction {
            body: ActionBody::SearchText,
            label: "Add Wishlist Item",
            method: "POST",
            path: "/wishlist",
            surface: "wishlist",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Run Wishlist Search",
            method: "POST",
            path: "/wishlist/wish-demo/search",
            surface: "wishlist",
        },
        RouteAction {
            body: ActionBody::DownloadFiles,
            label: "Queue Download",
            method: "POST",
            path: "/transfers/downloads/:username",
            surface: "transfers",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Clear Completed Downloads",
            method: "DELETE",
            path: "/transfers/downloads/all/completed",
            surface: "transfers",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Clear Completed Uploads",
            method: "DELETE",
            path: "/transfers/uploads/all/completed",
            surface: "transfers",
        },
        RouteAction {
            body: ActionBody::EnabledTrue,
            label: "Enable Accelerated Downloads",
            method: "PUT",
            path: "/transfers/downloads/accelerated",
            surface: "transfers",
        },
        RouteAction {
            body: ActionBody::EnabledFalse,
            label: "Disable Accelerated Downloads",
            method: "PUT",
            path: "/transfers/downloads/accelerated",
            surface: "transfers",
        },
        RouteAction {
            body: ActionBody::JsonString,
            label: "Join Room",
            method: "POST",
            path: "/rooms/joined",
            surface: "rooms",
        },
        RouteAction {
            body: ActionBody::RoomMessage,
            label: "Send Room Message",
            method: "POST",
            path: "/rooms/joined/:roomName/messages",
            surface: "rooms",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Leave Room",
            method: "DELETE",
            path: "/rooms/joined/:roomName",
            surface: "rooms",
        },
        RouteAction {
            body: ActionBody::ConversationMessage,
            label: "Send Message",
            method: "POST",
            path: "/conversations/:username",
            surface: "messages",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Acknowledge Conversation",
            method: "PUT",
            path: "/conversations/:username",
            surface: "messages",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Delete Conversation",
            method: "DELETE",
            path: "/conversations/:username",
            surface: "messages",
        },
        RouteAction {
            body: ActionBody::BrowseDirectory,
            label: "Request Directory",
            method: "POST",
            path: "/users/:username/directory",
            surface: "browse",
        },
        RouteAction {
            body: ActionBody::Username,
            label: "Add Contact",
            method: "POST",
            path: "/contacts",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::Username,
            label: "Add Discovery Contact",
            method: "POST",
            path: "/contacts/from-discovery",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::Username,
            label: "Accept Invite Contact",
            method: "POST",
            path: "/contacts/from-invite",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::Username,
            label: "Watch User",
            method: "POST",
            path: "/users/watch",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::Username,
            label: "Add User Note",
            method: "POST",
            path: "/users/notes",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::NameDescription,
            label: "Create Collection",
            method: "POST",
            path: "/collections",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::NameDescription,
            label: "Create Share Group",
            method: "POST",
            path: "/sharegroups",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::ShareGroupMember,
            label: "Add Share Group Member",
            method: "POST",
            path: "/sharegroups/group-demo/members",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::ShareGrant,
            label: "Create Share Grant",
            method: "POST",
            path: "/share-grants",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::Permissions,
            label: "Update Share Grant",
            method: "PUT",
            path: "/share-grants/grant-demo",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Backfill Share Grant",
            method: "POST",
            path: "/share-grants/grant-demo/backfill",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Issue Share Token",
            method: "POST",
            path: "/share-grants/grant-demo/token",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Delete Share Grant",
            method: "DELETE",
            path: "/share-grants/grant-demo",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::CollectionItem,
            label: "Add Library Item",
            method: "POST",
            path: "/library/items",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::FeedPreview,
            label: "Preview Playlist",
            method: "POST",
            path: "/source-feed-imports/preview",
            surface: "integrations",
        },
        RouteAction {
            body: ActionBody::SearchText,
            label: "Build Discovery Graph",
            method: "POST",
            path: "/discovery-graph",
            surface: "integrations",
        },
        RouteAction {
            body: ActionBody::NameDescription,
            label: "Create Source Feed",
            method: "POST",
            path: "/source-feeds",
            surface: "integrations",
        },
        RouteAction {
            body: ActionBody::SearchText,
            label: "Track MusicBrainz Target",
            method: "POST",
            path: "/musicbrainz/targets",
            surface: "integrations",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Subscribe Release Radar",
            method: "POST",
            path: "/musicbrainz/release-radar/subscriptions",
            surface: "integrations",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Create SongID Run",
            method: "POST",
            path: "/songid/runs",
            surface: "integrations",
        },
        RouteAction {
            body: ActionBody::SearchText,
            label: "Queue Discography Job",
            method: "POST",
            path: "/jobs/discography",
            surface: "integrations",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Connect",
            method: "PUT",
            path: "/server",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Disconnect",
            method: "DELETE",
            path: "/server",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Rescan Shares",
            method: "POST",
            path: "/shares/rescan",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Vacuum Database",
            method: "POST",
            path: "/database/vacuum",
            surface: "system",
        },
    ]
}

pub const fn route_pages() -> &'static [RoutePage] {
    &[
        RoutePage {
            description:
                "Create searches, inspect result groups, and open individual search records.",
            path: "/searches",
            surface: "search",
            title: "Search",
        },
        RoutePage {
            description: "Inspect one search, its peer responses, files, and action targets.",
            path: "/searches/:id",
            surface: "search",
            title: "Search Detail",
        },
        RoutePage {
            description:
                "Review release, artist, track, and query neighborhoods from discovery data.",
            path: "/discovery-graph",
            surface: "search",
            title: "Discovery Graph",
        },
        RoutePage {
            description: "Preview playlist imports and stage them for search or library workflows.",
            path: "/playlist-intake",
            surface: "integrations",
            title: "Playlist Intake",
        },
        RoutePage {
            description: "Keep persistent wanted-search intents and rerun them from one view.",
            path: "/wishlist",
            surface: "wishlist",
            title: "Wishlist",
        },
        RoutePage {
            description: "Track download queues, progress, peer grouping, and transfer actions.",
            path: "/downloads",
            surface: "transfers",
            title: "Downloads",
        },
        RoutePage {
            description: "Track upload queues, progress, peer grouping, and transfer actions.",
            path: "/uploads",
            surface: "transfers",
            title: "Uploads",
        },
        RoutePage {
            description: "Read private conversations and room-linked messaging activity.",
            path: "/messages",
            surface: "messages",
            title: "Messages",
        },
        RoutePage {
            description: "Use the legacy chat landing route while message surfaces converge.",
            path: "/chat",
            surface: "messages",
            title: "Chat",
        },
        RoutePage {
            description: "Join rooms, inspect room users, and read recent room messages.",
            path: "/rooms",
            surface: "rooms",
            title: "Rooms",
        },
        RoutePage {
            description: "Watch users, inspect presence, and request peer user context.",
            path: "/users",
            surface: "identity",
            title: "Users",
        },
        RoutePage {
            description: "Manage contacts, notes, groups, and peer relationship metadata.",
            path: "/contacts",
            surface: "identity",
            title: "Contacts",
        },
        RoutePage {
            description: "Manage Solid identity and linked-data integration state.",
            path: "/solid",
            surface: "integrations",
            title: "Solid",
        },
        RoutePage {
            description: "Inspect local collections and the records used for sharing workflows.",
            path: "/collections",
            surface: "collections",
            title: "Collections",
        },
        RoutePage {
            description: "Manage share groups and collection grants.",
            path: "/sharegroups",
            surface: "collections",
            title: "Share Groups",
        },
        RoutePage {
            description: "Inspect records and files shared with this user.",
            path: "/shared",
            surface: "collections",
            title: "Shared with Me",
        },
        RoutePage {
            description: "Request and inspect peer browse trees and cached folders.",
            path: "/browse",
            surface: "browse",
            title: "Browse",
        },
        RoutePage {
            description:
                "Inspect daemon status, telemetry, configuration, network, and integration state.",
            path: "/system",
            surface: "system",
            title: "System",
        },
        RoutePage {
            description: "Inspect a specific system tab while preserving the current route shape.",
            path: "/system/:tab",
            surface: "system",
            title: "System Tab",
        },
        RoutePage {
            description: "Inspect pod-oriented messaging and service-fabric route compatibility.",
            path: "/pods",
            surface: "messages",
            title: "Pods",
        },
        RoutePage {
            description: "Redirect pod detail routes back into the message surface.",
            path: "/pods/:podId",
            surface: "messages",
            title: "Pod Redirect",
        },
        RoutePage {
            description: "Redirect pod channel routes back into the message surface.",
            path: "/pods/:podId/channels/:channelId",
            surface: "messages",
            title: "Pod Channel Redirect",
        },
    ]
}

pub fn endpoint_url(endpoint: &str) -> String {
    format!("{}{}", api_base_path(), endpoint)
}

pub fn compatibility_report() -> String {
    format!(
        "{} UI routes, {} route pages, {} nav items, {} API contracts, {} route actions, {} runtime probes",
        ui_routes().len(),
        route_pages().len(),
        nav_items().len(),
        api_endpoints().len(),
        route_actions().len(),
        runtime_probes().len()
    )
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push(' '),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn compact_preview(value: &str) -> String {
    let trimmed = value.trim();
    let mut preview = String::new();
    for ch in trimmed.chars().take(180) {
        if ch.is_control() {
            preview.push(' ');
        } else {
            preview.push(ch);
        }
    }
    if trimmed.chars().count() > 180 {
        preview.push_str("...");
    }
    preview
}

pub fn runtime_probe_pending_html() -> String {
    runtime_probes()
        .iter()
        .map(|probe| {
            format!(
                r#"<li><strong>{label}</strong><code>{path}</code><span class="slskr-probe-pending">pending</span></li>"#,
                label = probe.label,
                path = endpoint_url(probe.path)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn runtime_probe_result_html(results: &[(&str, &str, Result<&str, &str>)]) -> String {
    results
        .iter()
        .map(|(label, path, result)| match result {
            Ok(body) => {
                let preview = escape_html(&compact_preview(body));
                format!(
                    r#"<li class="slskr-probe-ok"><strong>{label}</strong><code>{path}</code><span>{preview}</span></li>"#,
                    label = escape_html(label),
                    path = escape_html(path),
                )
            }
            Err(error) => {
                let message = escape_html(error);
                format!(
                    r#"<li class="slskr-probe-error"><strong>{label}</strong><code>{path}</code><span>{message}</span></li>"#,
                    label = escape_html(label),
                    path = escape_html(path),
                )
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn normalize_route_path(path: &str) -> &str {
    if path == "/" {
        return "/searches";
    }
    if path.starts_with("/searches/") {
        return "/searches/:id";
    }
    if path.starts_with("/system/") {
        return "/system/:tab";
    }
    if path.starts_with("/pods/") && path.contains("/channels/") {
        return "/pods/:podId/channels/:channelId";
    }
    if path.starts_with("/pods/") {
        return "/pods/:podId";
    }
    path
}

pub fn route_page(path: &str) -> Option<RoutePage> {
    let normalized = normalize_route_path(path);
    route_pages()
        .iter()
        .copied()
        .find(|page| page.path == normalized)
}

pub fn route_endpoints(surface: &str) -> Vec<ApiEndpoint> {
    api_endpoints()
        .iter()
        .copied()
        .filter(|endpoint| endpoint.surface == surface)
        .collect()
}

pub fn surface_actions(surface: &str) -> Vec<RouteAction> {
    route_actions()
        .iter()
        .copied()
        .filter(|action| action.surface == surface)
        .collect()
}

fn route_param_value(path: &str, fallback: &str) -> String {
    let value = path
        .trim_matches('/')
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(fallback);
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        value.to_owned()
    } else {
        fallback.to_owned()
    }
}

pub fn concrete_endpoint_path(route_path: &str, endpoint: ApiEndpoint) -> String {
    let search_id =
        if endpoint.path.contains(":id") && !normalize_route_path(route_path).contains(":id") {
            "1".to_string()
        } else {
            route_param_value(route_path, "1")
        };
    endpoint_url(endpoint.path)
        .replace(":id", &search_id)
        .replace(":username", "peer1")
        .replace(":roomName", "contract-room")
}

pub fn concrete_action_path(route_path: &str, action: RouteAction) -> String {
    let search_id =
        if action.path.contains(":id") && !normalize_route_path(route_path).contains(":id") {
            "1".to_string()
        } else {
            route_param_value(route_path, "1")
        };
    endpoint_url(action.path)
        .replace(":id", &search_id)
        .replace(":username", "peer1")
        .replace(":roomName", "contract-room")
}

pub fn route_action_at(path: &str, index: usize) -> Option<RouteAction> {
    let page = route_page(path)?;
    surface_actions(page.surface).get(index).copied()
}

pub fn action_body_from_value(body: ActionBody, value: &str) -> Option<String> {
    let value = value.trim();
    match body {
        ActionBody::None => None,
        ActionBody::BrowseDirectory => Some(format!(
            r#"{{"directory":"{}"}}"#,
            escape_json_string(value)
        )),
        ActionBody::CollectionItem => Some(format!(
            r#"{{"content_id":"rust-web-demo","artist":"Public Domain","title":"{}","kind":"Audio"}}"#,
            escape_json_string(value)
        )),
        ActionBody::DownloadFiles => Some(format!(
            r#"[{{"filename":"{}","size":99}}]"#,
            escape_json_string(value)
        )),
        ActionBody::EnabledFalse => Some(r#"{"enabled":false}"#.to_string()),
        ActionBody::EnabledTrue => Some(r#"{"enabled":true}"#.to_string()),
        ActionBody::FeedPreview => Some(format!(
            r#"{{"sourceText":"{}","sourceKind":"auto","limit":25,"includeAlbum":true,"fetchProviderUrls":false}}"#,
            escape_json_string(value)
        )),
        ActionBody::ConversationMessage | ActionBody::JsonString => {
            Some(format!(r#""{}""#, escape_json_string(value)))
        }
        ActionBody::NameDescription => Some(format!(
            r#"{{"name":"{}","description":"Created from the Rust web UI"}}"#,
            escape_json_string(value)
        )),
        ActionBody::Permissions => Some(format!(
            r#"{{"permissions":"{}"}}"#,
            escape_json_string(if value.is_empty() { "read" } else { value })
        )),
        ActionBody::RoomMessage => Some(format!(r#""{}""#, escape_json_string(value))),
        ActionBody::SearchText => Some(format!(
            r#"{{"searchText":"{}"}}"#,
            escape_json_string(value)
        )),
        ActionBody::ShareGrant => Some(format!(
            r#"{{"collection_id":"rust-web-demo","username":"{}"}}"#,
            escape_json_string(if value.is_empty() { "peer1" } else { value })
        )),
        ActionBody::ShareGroupMember => Some(format!(
            r#"{{"username":"{}"}}"#,
            escape_json_string(if value.is_empty() { "peer1" } else { value })
        )),
        ActionBody::Username => Some(format!(
            r#"{{"username":"{}","note":"Created from the Rust web UI"}}"#,
            escape_json_string(value)
        )),
    }
}

pub fn action_input_html(action: RouteAction) -> String {
    match action.body {
        ActionBody::None => String::new(),
        ActionBody::BrowseDirectory => {
            r#"<input class="slskr-action-input" data-slskr-action-input="BrowseDirectory" value="" placeholder="Directory">"#.to_string()
        }
        ActionBody::ConversationMessage => {
            r#"<input class="slskr-action-input" data-slskr-action-input="ConversationMessage" value="hello" placeholder="Message">"#.to_string()
        }
        ActionBody::CollectionItem => {
            r#"<input class="slskr-action-input" data-slskr-action-input="CollectionItem" value="Demo Track" placeholder="Title">"#.to_string()
        }
        ActionBody::DownloadFiles => {
            r#"<input class="slskr-action-input" data-slskr-action-input="DownloadFiles" value="Remote/Song.mp3" placeholder="Filename">"#.to_string()
        }
        ActionBody::EnabledFalse | ActionBody::EnabledTrue => String::new(),
        ActionBody::FeedPreview => {
            r#"<input class="slskr-action-input" data-slskr-action-input="FeedPreview" value="Public Domain Jazz - Demo Track" placeholder="Playlist text">"#.to_string()
        }
        ActionBody::JsonString => {
            r#"<input class="slskr-action-input" data-slskr-action-input="JsonString" value="contract-room" placeholder="Name">"#.to_string()
        }
        ActionBody::NameDescription => {
            r#"<input class="slskr-action-input" data-slskr-action-input="NameDescription" value="Rust Web Demo" placeholder="Name">"#.to_string()
        }
        ActionBody::Permissions => {
            r#"<input class="slskr-action-input" data-slskr-action-input="Permissions" value="read" placeholder="Permissions">"#.to_string()
        }
        ActionBody::RoomMessage => {
            r#"<input class="slskr-action-input" data-slskr-action-input="RoomMessage" value="hello room" placeholder="Message">"#.to_string()
        }
        ActionBody::SearchText => {
            r#"<input class="slskr-action-input" data-slskr-action-input="SearchText" value="public domain jazz" placeholder="Search text">"#.to_string()
        }
        ActionBody::ShareGrant | ActionBody::ShareGroupMember => {
            r#"<input class="slskr-action-input" data-slskr-action-input="Username" value="peer1" placeholder="Username">"#.to_string()
        }
        ActionBody::Username => {
            r#"<input class="slskr-action-input" data-slskr-action-input="Username" value="peer1" placeholder="Username">"#.to_string()
        }
    }
}

pub fn route_actions_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return String::new();
    };
    surface_actions(page.surface)
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let url = concrete_action_path(path, *action);
            let input = action_input_html(*action);
            format!(
                r#"<li><div><strong>{method}</strong><code>{path}</code></div>{input}<button type="button" class="slskr-action-button" data-slskr-action-index="{index}" data-slskr-action-method="{method}" data-slskr-action-path="{path}" data-slskr-action-body="{body:?}">{label}</button></li>"#,
                method = escape_html(action.method),
                path = escape_html(&url),
                input = input,
                label = escape_html(action.label),
                body = action.body,
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn stat_card_html(label: &str, value: &str, detail: &str) -> String {
    format!(
        r#"<li><strong>{label}</strong><span>{value}</span><small>{detail}</small></li>"#,
        label = escape_html(label),
        value = escape_html(value),
        detail = escape_html(detail),
    )
}

fn json_array_len(body: &str) -> Option<usize> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.as_array().map(Vec::len))
}

fn json_entries_len(body: &str) -> Option<usize> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("entries").cloned())
        .and_then(|value| value.as_array().map(Vec::len))
}

fn json_field_string(body: &str, field: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get(field).cloned())
        .and_then(|value| match value {
            serde_json::Value::String(text) => Some(text),
            serde_json::Value::Bool(value) => Some(value.to_string()),
            serde_json::Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
}

fn endpoint_body<'a>(responses: &'a [EndpointBody], path: &str) -> Option<&'a str> {
    responses
        .iter()
        .find(|response| response.endpoint.path == path)
        .map(|response| response.body.as_str())
}

pub fn surface_names() -> Vec<&'static str> {
    let mut names = route_pages()
        .iter()
        .map(|page| page.surface)
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    names
}

pub fn surface_route_count(surface: &str) -> usize {
    route_pages()
        .iter()
        .filter(|page| page.surface == surface)
        .count()
}

pub fn surface_matrix_html() -> String {
    surface_names()
        .iter()
        .map(|surface| {
            stat_card_html(
                surface,
                &format!(
                    "{} routes / {} APIs / {} actions",
                    surface_route_count(surface),
                    route_endpoints(surface).len(),
                    surface_actions(surface).len()
                ),
                "bulk Rust coverage",
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn surface_route_catalog_html(surface: &str) -> String {
    route_pages()
        .iter()
        .filter(|page| page.surface == surface)
        .map(|page| {
            format!(
                r#"<li><code>{path}</code><span>{title}</span></li>"#,
                path = escape_html(page.path),
                title = escape_html(page.title)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn surface_endpoint_catalog_html(surface: &str) -> String {
    route_endpoints(surface)
        .iter()
        .map(|endpoint| {
            format!(
                r#"<li><strong>{method}</strong><code>{path}</code></li>"#,
                method = escape_html(endpoint.method),
                path = escape_html(&endpoint_url(endpoint.path)),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn surface_action_catalog_html(surface: &str) -> String {
    surface_actions(surface)
        .iter()
        .map(|action| {
            format!(
                r#"<li><strong>{method}</strong><code>{path}</code><span>{body:?}</span></li>"#,
                method = escape_html(action.method),
                path = escape_html(&endpoint_url(action.path)),
                body = action.body,
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn surface_workbench_html(surface: &str) -> String {
    format!(
        r#"<article class="slskr-workbench-surface" data-slskr-surface="{surface}"><header><h4>{surface}</h4><span>{routes} routes / {apis} APIs / {actions} actions</span></header><div><h5>Routes</h5><ul>{route_catalog}</ul></div><div><h5>Endpoints</h5><ul>{endpoint_catalog}</ul></div><div><h5>Actions</h5><ul>{action_catalog}</ul></div></article>"#,
        surface = escape_html(surface),
        routes = surface_route_count(surface),
        apis = route_endpoints(surface).len(),
        actions = surface_actions(surface).len(),
        route_catalog = surface_route_catalog_html(surface),
        endpoint_catalog = surface_endpoint_catalog_html(surface),
        action_catalog = surface_action_catalog_html(surface),
    )
}

pub fn bulk_workbench_html() -> String {
    surface_names()
        .iter()
        .map(|surface| surface_workbench_html(surface))
        .collect::<Vec<_>>()
        .join("")
}

pub fn route_summary_pending_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return String::new();
    };
    match page.surface {
        "search" => [
            stat_card_html("Searches", "pending", "active records"),
            stat_card_html("Responses", "pending", "selected search"),
            stat_card_html(
                "Actions",
                &surface_actions("search").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "transfers" => [
            stat_card_html("Downloads", "pending", "peer groups"),
            stat_card_html("Uploads", "pending", "peer groups"),
            stat_card_html("Speeds", "pending", "transfer rates"),
        ]
        .join(""),
        "rooms" => [
            stat_card_html("Available", "pending", "rooms"),
            stat_card_html("Joined", "pending", "rooms"),
            stat_card_html(
                "Actions",
                &surface_actions("rooms").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "messages" => [
            stat_card_html("Conversations", "pending", "threads"),
            stat_card_html("Selected", "pending", "peer1"),
            stat_card_html(
                "Actions",
                &surface_actions("messages").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "browse" => [
            stat_card_html("Peer", "peer1", "browse target"),
            stat_card_html("Folders", "pending", "cached entries"),
            stat_card_html(
                "Actions",
                &surface_actions("browse").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "system" => [
            stat_card_html("Metrics", "pending", "runtime"),
            stat_card_html("Options", "pending", "configuration"),
            stat_card_html(
                "Actions",
                &surface_actions("system").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "wishlist" => [
            stat_card_html("Wishlist", "pending", "wanted items"),
            stat_card_html(
                "Actions",
                &surface_actions("wishlist").len().to_string(),
                "Rust owned",
            ),
            stat_card_html("Coverage", "bulk", "route group"),
        ]
        .join(""),
        "identity" => [
            stat_card_html("Users", "pending", "watched peers"),
            stat_card_html("Contacts", "pending", "relationships"),
            stat_card_html(
                "Actions",
                &surface_actions("identity").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "collections" => [
            stat_card_html("Collections", "pending", "records"),
            stat_card_html("Share Groups", "pending", "groups"),
            stat_card_html(
                "Actions",
                &surface_actions("collections").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "integrations" => [
            stat_card_html("Providers", "pending", "sources"),
            stat_card_html("Jobs", "pending", "automation"),
            stat_card_html(
                "Actions",
                &surface_actions("integrations").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        _ => [
            stat_card_html("Surface", page.surface, "route group"),
            stat_card_html(
                "Endpoints",
                &route_endpoints(page.surface).len().to_string(),
                "tracked",
            ),
            stat_card_html(
                "Actions",
                &surface_actions(page.surface).len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
    }
}

pub fn route_summary_result_html(path: &str, responses: &[EndpointBody]) -> String {
    let Some(page) = route_page(path) else {
        return String::new();
    };
    match page.surface {
        "search" => {
            let searches = endpoint_body(responses, "/searches")
                .and_then(json_array_len)
                .or_else(|| {
                    endpoint_body(responses, "/searches/records").and_then(json_entries_len)
                })
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            let responses_count = endpoint_body(responses, "/searches/:id/responses")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            [
                stat_card_html("Searches", &searches, "active records"),
                stat_card_html("Responses", &responses_count, "selected search"),
                stat_card_html(
                    "Actions",
                    &surface_actions("search").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        "transfers" => {
            let downloads = endpoint_body(responses, "/transfers/downloads")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            let uploads = endpoint_body(responses, "/transfers/uploads")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            let speeds = endpoint_body(responses, "/transfers/speeds")
                .map(compact_preview)
                .unwrap_or_else(|| "{}".to_string());
            [
                stat_card_html("Downloads", &downloads, "peer groups"),
                stat_card_html("Uploads", &uploads, "peer groups"),
                stat_card_html("Speeds", &speeds, "transfer rates"),
            ]
            .join("")
        }
        "rooms" => {
            let available = endpoint_body(responses, "/rooms/available")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            let joined = endpoint_body(responses, "/rooms/joined")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            [
                stat_card_html("Available", &available, "rooms"),
                stat_card_html("Joined", &joined, "rooms"),
                stat_card_html(
                    "Actions",
                    &surface_actions("rooms").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        "messages" => {
            let conversations = endpoint_body(responses, "/conversations")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            let selected = endpoint_body(responses, "/conversations/:username")
                .and_then(|body| {
                    json_field_string(body, "username")
                        .or_else(|| json_field_string(body, "message_count"))
                        .or_else(|| json_field_string(body, "messages"))
                })
                .unwrap_or_else(|| "peer1".to_string());
            [
                stat_card_html("Conversations", &conversations, "threads"),
                stat_card_html("Selected", &selected, "peer1"),
                stat_card_html(
                    "Actions",
                    &surface_actions("messages").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        "browse" => {
            let folders = endpoint_body(responses, "/users/:username/browse")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            [
                stat_card_html("Peer", "peer1", "browse target"),
                stat_card_html("Folders", &folders, "cached entries"),
                stat_card_html(
                    "Actions",
                    &surface_actions("browse").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        "wishlist" => {
            let wishlist = endpoint_body(responses, "/wishlist")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            [
                stat_card_html("Wishlist", &wishlist, "wanted items"),
                stat_card_html(
                    "Actions",
                    &surface_actions("wishlist").len().to_string(),
                    "Rust owned",
                ),
                stat_card_html("Coverage", "bulk", "route group"),
            ]
            .join("")
        }
        "identity" => {
            let users = endpoint_body(responses, "/users")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            let contacts = endpoint_body(responses, "/contacts")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            [
                stat_card_html("Users", &users, "watched peers"),
                stat_card_html("Contacts", &contacts, "relationships"),
                stat_card_html(
                    "Actions",
                    &surface_actions("identity").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        "collections" => {
            let collections = endpoint_body(responses, "/collections")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            let sharegroups = endpoint_body(responses, "/sharegroups")
                .and_then(json_array_len)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            [
                stat_card_html("Collections", &collections, "records"),
                stat_card_html("Share Groups", &sharegroups, "groups"),
                stat_card_html(
                    "Actions",
                    &surface_actions("collections").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        "integrations" => {
            let providers = endpoint_body(responses, "/source-providers")
                .and_then(|body| json_field_string(body, "count"))
                .unwrap_or_else(|| "0".to_string());
            let jobs = endpoint_body(responses, "/jobs")
                .and_then(json_array_len)
                .or_else(|| {
                    endpoint_body(responses, "/songid/runs")
                        .and_then(|body| json_field_string(body, "count"))
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string());
            [
                stat_card_html("Providers", &providers, "sources"),
                stat_card_html("Jobs", &jobs, "automation"),
                stat_card_html(
                    "Actions",
                    &surface_actions("integrations").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        "system" => {
            let metrics = endpoint_body(responses, "/telemetry/metrics")
                .map(|body| {
                    if body.contains("slskr_") {
                        "scrapable".to_string()
                    } else {
                        compact_preview(body)
                    }
                })
                .unwrap_or_else(|| "offline".to_string());
            let options = endpoint_body(responses, "/options")
                .and_then(|body| json_field_string(body, "config_file"))
                .unwrap_or_else(|| "runtime".to_string());
            [
                stat_card_html("Metrics", &metrics, "runtime"),
                stat_card_html("Options", &options, "configuration"),
                stat_card_html(
                    "Actions",
                    &surface_actions("system").len().to_string(),
                    "Rust owned",
                ),
            ]
            .join("")
        }
        _ => route_summary_pending_html(path),
    }
}

pub fn route_probe_pending_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return String::new();
    };
    route_endpoints(page.surface)
        .iter()
        .filter(|endpoint| endpoint.method == "GET")
        .map(|endpoint| {
            let path = concrete_endpoint_path(path, *endpoint);
            format!(
                r#"<li><strong>GET</strong><code>{path}</code><span class="slskr-probe-pending">pending</span></li>"#,
                path = escape_html(&path)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn endpoint_title(path: &str) -> String {
    path.trim_start_matches('/')
        .replace(['/', '-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn json_display_array(value: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    if let Some(items) = value.as_array() {
        return Some(items);
    }
    for key in [
        "entries",
        "items",
        "records",
        "results",
        "responses",
        "runs",
        "providers",
        "jobs",
        "events",
        "logs",
        "shares",
        "users",
        "collections",
        "groups",
        "grants",
        "directories",
        "messages",
        "rooms",
        "files",
    ] {
        if let Some(items) = value.get(key).and_then(|entry| entry.as_array()) {
            return Some(items);
        }
    }
    None
}

fn json_scalar_preview(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        _ => compact_preview(&value.to_string()),
    }
}

fn json_object_fields(value: &serde_json::Value) -> Vec<(&str, String)> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| match value {
                    serde_json::Value::Array(_) | serde_json::Value::Object(_) => None,
                    _ => Some((key.as_str(), json_scalar_preview(value))),
                })
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn json_table_columns(items: &[serde_json::Value]) -> Vec<String> {
    let preferred = [
        "name",
        "username",
        "query",
        "title",
        "filename",
        "path",
        "status",
        "state",
        "kind",
        "size",
        "bytes",
        "createdAt",
        "updatedAt",
        "id",
    ];
    let mut columns = Vec::new();
    for key in preferred {
        if items.iter().any(|item| item.get(key).is_some()) {
            columns.push(key.to_string());
        }
    }
    for item in items.iter().take(10) {
        let Some(object) = item.as_object() else {
            continue;
        };
        for (key, value) in object {
            if columns.iter().any(|column| column == key)
                || matches!(
                    value,
                    serde_json::Value::Array(_) | serde_json::Value::Object(_)
                )
            {
                continue;
            }
            columns.push(key.clone());
            if columns.len() >= 6 {
                return columns;
            }
        }
    }
    if columns.is_empty() {
        columns.push("value".to_string());
    }
    columns.truncate(6);
    columns
}

fn json_cell_value(item: &serde_json::Value, column: &str) -> String {
    if column == "value" {
        return compact_preview(&item.to_string());
    }
    item.get(column)
        .map(json_scalar_preview)
        .unwrap_or_default()
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn data_card_table_html(items: &[serde_json::Value]) -> String {
    let columns = json_table_columns(items);
    let header = columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            format!(
                r#"<th><button type="button" data-slskr-sort-index="{index}" aria-label="Sort by {column}">{column}</button></th>"#,
                index = index,
                column = escape_html(column),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let rows = items
        .iter()
        .take(50)
        .map(|item| {
            let label = record_label(item);
            let detail = record_detail(item);
            let raw = record_json(item);
            let search_text = record_search_text(item, &label, &detail);
            let cells = columns
                .iter()
                .map(|column| {
                    format!(
                        r#"<td>{}</td>"#,
                        escape_html(&json_cell_value(item, column))
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            format!(
                r#"<tr tabindex="0" data-slskr-row-text="{search}" data-slskr-record-select data-slskr-record-title="{title}" data-slskr-record-detail="{detail}" data-slskr-record-json="{raw}">{cells}</tr>"#,
                search = escape_html(&search_text),
                title = escape_html(&label),
                detail = escape_html(&detail),
                raw = escape_html(&raw),
                cells = cells,
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="slskr-table-wrap"><table class="slskr-data-table"><thead><tr>{header}</tr></thead><tbody>{rows}</tbody></table></div>"#,
        header = header,
        rows = rows,
    )
}

fn data_card_csv_html(items: &[serde_json::Value]) -> String {
    let columns = json_table_columns(items);
    let mut lines = vec![columns
        .iter()
        .map(|column| csv_escape(column))
        .collect::<Vec<_>>()
        .join(",")];
    lines.extend(items.iter().take(50).map(|item| {
        columns
            .iter()
            .map(|column| csv_escape(&json_cell_value(item, column)))
            .collect::<Vec<_>>()
            .join(",")
    }));
    format!(
        r#"<details class="slskr-card-csv"><summary>CSV</summary><pre>{}</pre></details>"#,
        escape_html(&lines.join("\n"))
    )
}

fn record_label(item: &serde_json::Value) -> String {
    item.get("name")
        .or_else(|| item.get("username"))
        .or_else(|| item.get("query"))
        .or_else(|| item.get("title"))
        .or_else(|| item.get("filename"))
        .or_else(|| item.get("id"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| compact_preview(&item.to_string()))
}

fn record_detail(item: &serde_json::Value) -> String {
    item.get("status")
        .or_else(|| item.get("state"))
        .or_else(|| item.get("kind"))
        .or_else(|| item.get("message"))
        .or_else(|| item.get("path"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| format!("{} fields", json_object_fields(item).len()))
}

fn record_json(item: &serde_json::Value) -> String {
    serde_json::to_string_pretty(item).unwrap_or_else(|_| item.to_string())
}

fn record_search_text(item: &serde_json::Value, label: &str, detail: &str) -> String {
    format!("{label} {detail} {}", compact_preview(&item.to_string())).to_lowercase()
}

fn data_card_inspector_html() -> String {
    r#"<aside class="slskr-card-inspector" aria-live="polite"><h4>Record Inspector</h4><p>Select a list or table row to inspect its details.</p><pre></pre></aside>"#.to_string()
}

fn data_card_pending_html(endpoint: ApiEndpoint) -> String {
    format!(
        r#"<article class="slskr-data-card"><header><h3>{title}</h3><code>{method} {path}</code></header><div class="slskr-empty-state">Loading</div></article>"#,
        title = escape_html(&endpoint_title(endpoint.path)),
        method = escape_html(endpoint.method),
        path = escape_html(&endpoint_url(endpoint.path)),
    )
}

fn data_card_result_html(response: &EndpointBody) -> String {
    let title = endpoint_title(response.endpoint.path);
    let url = endpoint_url(response.endpoint.path);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&response.body) else {
        return format!(
            r#"<article class="slskr-data-card"><header><h3>{title}</h3><code>GET {url}</code></header><pre>{body}</pre></article>"#,
            title = escape_html(&title),
            url = escape_html(&url),
            body = escape_html(&compact_preview(&response.body)),
        );
    };

    if let Some(items) = json_display_array(&value) {
        if items.is_empty() {
            return format!(
                r#"<article class="slskr-data-card" data-slskr-data-card data-slskr-view="list"><header><div><h3>{title}</h3><code>GET {url}</code></div><span>0 records</span></header><div class="slskr-card-tools"><input type="search" class="slskr-card-filter" placeholder="Filter records" aria-label="Filter {title}"><button type="button" class="slskr-card-clear" data-slskr-card-clear>Clear</button><span class="slskr-card-count" data-slskr-card-count>0 / 0</span><div class="slskr-card-view"><button type="button" class="is-active" data-slskr-card-view="list">List</button><button type="button" data-slskr-card-view="table">Table</button></div></div><div class="slskr-empty-state">No records</div>{table}{csv}</article>"#,
                title = escape_html(&title),
                url = escape_html(&url),
                table = data_card_table_html(items),
                csv = data_card_csv_html(items),
            );
        }
        let rows = items
            .iter()
            .take(50)
            .map(|item| {
                let label = record_label(item);
                let detail = record_detail(item);
                let raw = record_json(item);
                let search_text = record_search_text(item, &label, &detail);
                format!(
                    r#"<li tabindex="0" data-slskr-row-text="{search}" data-slskr-record-select data-slskr-record-title="{title}" data-slskr-record-detail="{detail}" data-slskr-record-json="{raw}"><strong>{label}</strong><span>{detail}</span></li>"#,
                    search = escape_html(&search_text),
                    title = escape_html(&label),
                    raw = escape_html(&raw),
                    label = escape_html(&label),
                    detail = escape_html(&detail),
                )
            })
            .collect::<Vec<_>>()
            .join("");
        return format!(
            r#"<article class="slskr-data-card" data-slskr-data-card data-slskr-view="list"><header><div><h3>{title}</h3><code>GET {url}</code></div><span>{count} records</span></header><div class="slskr-card-tools"><input type="search" class="slskr-card-filter" placeholder="Filter records" aria-label="Filter {title}"><button type="button" class="slskr-card-clear" data-slskr-card-clear>Clear</button><span class="slskr-card-count" data-slskr-card-count>{count} / {count}</span><div class="slskr-card-view"><button type="button" class="is-active" data-slskr-card-view="list">List</button><button type="button" data-slskr-card-view="table">Table</button></div></div><ul class="slskr-record-list">{rows}</ul>{table}{inspector}{csv}</article>"#,
            title = escape_html(&title),
            url = escape_html(&url),
            count = items.len(),
            rows = rows,
            table = data_card_table_html(items),
            inspector = data_card_inspector_html(),
            csv = data_card_csv_html(items),
        );
    }

    let fields = json_object_fields(&value)
        .iter()
        .map(|(key, value)| {
            format!(
                r#"<li><strong>{key}</strong><span>{value}</span></li>"#,
                key = escape_html(key),
                value = escape_html(value),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<article class="slskr-data-card"><header><h3>{title}</h3><code>GET {url}</code></header><ul class="slskr-field-list">{fields}</ul></article>"#,
        title = escape_html(&title),
        url = escape_html(&url),
        fields = fields,
    )
}

pub fn route_workspace_pending_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return String::new();
    };
    route_endpoints(page.surface)
        .iter()
        .filter(|endpoint| endpoint.method == "GET")
        .map(|endpoint| data_card_pending_html(*endpoint))
        .collect::<Vec<_>>()
        .join("")
}

pub fn route_workspace_result_html(path: &str, responses: &[EndpointBody]) -> String {
    if responses.is_empty() {
        return route_workspace_pending_html(path);
    }
    let Some(page) = route_page(path) else {
        return String::new();
    };
    match page.surface {
        "search" => search_workspace_html(responses),
        "transfers" => transfers_workspace_html(responses),
        "messages" => messages_workspace_html(responses),
        "rooms" => rooms_workspace_html(responses),
        "browse" => browse_workspace_html(responses),
        "identity" => identity_workspace_html(responses),
        "collections" => collections_workspace_html(responses),
        "integrations" => integrations_workspace_html(responses),
        "system" => system_workspace_html(responses),
        "wishlist" => wishlist_workspace_html(responses),
        _ => data_cards_html(responses),
    }
}

fn data_cards_html(responses: &[EndpointBody]) -> String {
    responses
        .iter()
        .map(data_card_result_html)
        .collect::<Vec<_>>()
        .join("")
}

fn workspace_tabs_html(tabs: &[&str]) -> String {
    let modes = ["all", "primary", "secondary"];
    tabs.iter()
        .enumerate()
        .map(|(index, tab)| {
            format!(
                r#"<button type="button" class="{class}" data-slskr-workspace-mode="{mode}" aria-selected="{selected}">{tab}</button>"#,
                class = if index == 0 {
                    "slskr-workspace-tab is-active"
                } else {
                    "slskr-workspace-tab"
                },
                mode = modes.get(index).copied().unwrap_or("all"),
                selected = if index == 0 { "true" } else { "false" },
                tab = escape_html(tab),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn selected_cards_html(responses: &[EndpointBody], paths: &[&str]) -> String {
    paths
        .iter()
        .filter_map(|path| {
            responses
                .iter()
                .find(|response| response.endpoint.path == *path)
        })
        .map(data_card_result_html)
        .collect::<Vec<_>>()
        .join("")
}

fn workspace_layout_html(tabs: &[&str], primary: String, secondary: String) -> String {
    format!(
        r#"<div class="slskr-workspace-tabs">{tabs}</div><div class="slskr-workspace-grid" data-slskr-workspace-grid><section class="slskr-workspace-primary">{primary}</section><aside class="slskr-workspace-secondary">{secondary}</aside></div>"#,
        tabs = workspace_tabs_html(tabs),
        primary = primary,
        secondary = secondary,
    )
}

fn search_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Searches", "Responses", "Interests"],
        selected_cards_html(responses, &["/searches", "/searches/:id/responses"]),
        selected_cards_html(
            responses,
            &[
                "/searches/records",
                "/soulseek/interests",
                "/soulseek/hated-interests",
            ],
        ),
    )
}

fn transfers_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Downloads", "Uploads", "Speeds"],
        selected_cards_html(responses, &["/transfers/downloads", "/transfers/uploads"]),
        selected_cards_html(responses, &["/transfers/speeds"]),
    )
}

fn messages_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Inbox", "Thread", "Pods"],
        selected_cards_html(responses, &["/conversations", "/conversations/:username"]),
        selected_cards_html(responses, &["/pods"]),
    )
}

fn rooms_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Available", "Joined", "Activity"],
        selected_cards_html(responses, &["/rooms/available", "/rooms/joined"]),
        r#"<article class="slskr-data-card"><header><h3>Room Activity</h3><code>rooms stream</code></header><div class="slskr-empty-state">Join a room to show users and messages.</div></article>"#.to_string(),
    )
}

fn browse_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Folders", "Files", "Peer"],
        selected_cards_html(responses, &["/users/:username/browse"]),
        r#"<article class="slskr-data-card"><header><h3>Peer Browse</h3><code>directory request</code></header><div class="slskr-empty-state">Request a directory to populate the tree.</div></article>"#.to_string(),
    )
}

fn identity_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Users", "Contacts", "Notes"],
        selected_cards_html(responses, &["/users", "/contacts"]),
        selected_cards_html(
            responses,
            &[
                "/users/:username/info",
                "/users/:username/status",
                "/users/:username/endpoint",
                "/contacts/nearby",
                "/users/notes",
            ],
        ),
    )
}

fn collections_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Collections", "Sharing", "Library"],
        selected_cards_html(
            responses,
            &["/collections", "/sharegroups", "/share-grants", "/shared"],
        ),
        selected_cards_html(
            responses,
            &[
                "/shares/catalog",
                "/shares",
                "/library/items",
                "/library/items/browser",
                "/files/downloads/directories",
                "/files/incomplete/directories",
            ],
        ),
    )
}

fn integrations_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Sources", "Metadata", "Automation"],
        selected_cards_html(
            responses,
            &[
                "/source-providers",
                "/source-feeds",
                "/musicbrainz/albums/completion",
                "/musicbrainz/release-radar/subscriptions",
            ],
        ),
        selected_cards_html(
            responses,
            &[
                "/songid/runs",
                "/solid/status",
                "/pods",
                "/bridge/status",
                "/jobs",
                "/mesh/stats",
                "/security/dashboard",
            ],
        ),
    )
}

fn system_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Runtime", "Events", "Storage"],
        selected_cards_html(
            responses,
            &[
                "/telemetry/metrics",
                "/telemetry/metrics/kpis",
                "/telemetry/reports/transfers/summary",
                "/options",
            ],
        ),
        selected_cards_html(
            responses,
            &["/events", "/logs", "/shares", "/database/stats"],
        ),
    )
}

fn wishlist_workspace_html(responses: &[EndpointBody]) -> String {
    workspace_layout_html(
        &["Wishlist", "Search", "Import"],
        selected_cards_html(responses, &["/wishlist"]),
        r#"<article class="slskr-data-card"><header><h3>Wishlist Actions</h3><code>add / run / import</code></header><div class="slskr-empty-state">Add wanted searches, rerun them, or import a CSV.</div></article>"#.to_string(),
    )
}

#[allow(dead_code)]
fn route_toolbar_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return String::new();
    };
    match page.surface {
        "search" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="public domain jazz" aria-label="Search text"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Search</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Stop</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Clear</button></form>"#.to_string(),
        "transfers" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="Remote/Song.mp3" aria-label="Filename"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Queue file</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Clear downloads</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Clear uploads</button></form>"#.to_string(),
        "messages" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="hello" aria-label="Message"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Send</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Acknowledge</button></form>"#.to_string(),
        "rooms" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="contract-room" aria-label="Room"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Join</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Send</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Leave</button></form>"#.to_string(),
        "browse" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="" aria-label="Directory" placeholder="Directory"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Request directory</button></form>"#.to_string(),
        "identity" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="peer1" aria-label="Username"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Watch</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Add contact</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Note</button></form>"#.to_string(),
        "collections" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="Rust Web Demo" aria-label="Name"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Create collection</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Create group</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Share</button></form>"#.to_string(),
        "integrations" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="Public Domain Jazz - Demo Track" aria-label="Playlist text"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Preview playlist</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Discovery graph</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Queue job</button></form>"#.to_string(),
        "system" => r#"<div class="slskr-toolbar"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Connect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Disconnect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Rescan shares</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Vacuum database</button></div>"#.to_string(),
        "wishlist" => r#"<form class="slskr-toolbar"><input class="slskr-toolbar-input" value="public domain jazz" aria-label="Wishlist text"><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="0">Add</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Run search</button></form>"#.to_string(),
        _ => String::new(),
    }
}

pub fn route_page_html(path: &str) -> String {
    let Some(page) = route_page(path) else {
        return route_page_html("/searches");
    };
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
    format!(
        r#"<section class="slskr-route-page" data-route="{path}"><header class="slskr-page-header"><div><p class="slskr-kicker">{surface}</p><h2>{title}</h2><p>{description}</p></div><div class="slskr-page-chip">{surface} workspace</div></header>{toolbar}<div class="slskr-route-summary"><h3>Summary</h3><ul id="slskr-route-summary">{summary}</ul></div><div class="slskr-functional-layout"><aside class="slskr-route-actions"><h3>Commands</h3><ul id="slskr-route-actions">{actions}</ul><p id="slskr-action-status" aria-live="polite"></p></aside><section class="slskr-work-area"><header><div><h3>Live Data</h3><span id="slskr-live-status" aria-live="polite">Updates from daemon APIs</span></div><div class="slskr-live-controls"><button type="button" data-slskr-refresh-route>Refresh</button><button type="button" data-slskr-focus-filter>Filter</button><button type="button" data-slskr-clear-filters>Clear Filters</button></div></header><div id="slskr-page-data" class="slskr-page-data">{page_data}</div></section></div><details class="slskr-diagnostics"><summary>Route diagnostics</summary><div class="slskr-route-columns"><div><h3>Route Shape</h3><ul>{routes}</ul></div><div><h3>API Surface</h3><ul>{endpoints}</ul></div></div><div class="slskr-route-live"><h3>Raw Probe Status</h3><ul id="slskr-route-data">{route_data}</ul></div></details></section>"#,
        path = escape_html(path),
        surface = escape_html(page.surface),
        title = escape_html(page.title),
        description = escape_html(page.description),
        toolbar = route_toolbar_html(path),
        summary = route_summary_pending_html(path),
        routes = route_inventory,
        endpoints = endpoints,
        actions = route_actions_html(path),
        page_data = route_workspace_pending_html(path),
        route_data = route_probe_pending_html(path),
    )
}

fn player_footer_html() -> String {
    r#"<footer class="slskr-player" data-slskr-player data-slskr-player-rating-key="" data-slskr-player-radio-query=""><section><strong>Now Playing</strong><span id="slskr-player-now">Queue idle</span><span id="slskr-player-now-detail">No local stream selected</span></section><section class="slskr-player-controls" aria-label="Player controls"><button type="button" data-slskr-player-action="refresh">Refresh</button><button type="button" data-slskr-player-action="clear">Clear</button><button type="button" data-slskr-player-action="visualizer">Visualizer</button><button type="button" data-slskr-player-action="radio">Radio</button></section><section class="slskr-player-rating" aria-label="Player rating"><strong>Rating</strong><div id="slskr-player-rating-controls"><button type="button" data-slskr-player-rating="1">1</button><button type="button" data-slskr-player-rating="2">2</button><button type="button" data-slskr-player-rating="3">3</button><button type="button" data-slskr-player-rating="4">4</button><button type="button" data-slskr-player-rating="5">5</button></div><span id="slskr-player-rating-status">Not rated</span></section><section><strong>Radio</strong><span id="slskr-player-radio">No track selected</span><span id="slskr-player-transfers">0 down / 0 up</span></section><section><strong>Visualizer</strong><span id="slskr-player-visualizer">Checking status</span><span id="slskr-player-status" aria-live="polite">Rust player surface ready</span></section></footer>"#.to_string()
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
        r#"<div class="slskr-shell"><nav class="slskr-nav">{nav}</nav><main class="slskr-main"><header class="slskr-appbar"><div><strong>slskr</strong><span>Search, transfers, messages, rooms, browse, sharing, and system control</span></div><ul id="slskr-runtime-status">{runtime}</ul></header><section id="slskr-route-view">{route_page}</section></main>{player}</div>"#,
        route_page = route_page_html("/searches"),
        runtime = runtime_probe_pending_html(),
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
fn mount_router(window: &web_sys::Window, document: &web_sys::Document) -> Result<(), JsValue> {
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
fn render_current_route(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let path = window.location().pathname()?;
    if let Some(view) = document.get_element_by_id("slskr-route-view") {
        view.set_inner_html(&route_page_html(&path));
    }
    mount_route_actions(window, document)?;
    mount_toolbar_actions(window, document)?;
    mount_workspace_tabs(document)?;
    mount_data_cards(document)?;
    mount_live_controls(window, document)?;
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
fn mount_data_cards(document: &web_sys::Document) -> Result<(), JsValue> {
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
fn mount_live_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    if let Some(button) = document.query_selector("[data-slskr-refresh-route]")? {
        let window = window.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let window = window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = refresh_route_data(&window).await;
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    if let Some(button) = document.query_selector("[data-slskr-focus-filter]")? {
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                focus_first_card_filter(&document);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    if let Some(button) = document.query_selector("[data-slskr-clear-filters]")? {
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                clear_all_card_filters(&document);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn mount_global_shortcuts(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let window = window.clone();
    let listener_document = document.clone();
    let document = document.clone();
    let callback = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
        move |event: web_sys::KeyboardEvent| {
            if keyboard_event_started_in_text_control(&document) {
                return;
            }
            let key = event.key();
            if key == "/" {
                event.prevent_default();
                focus_first_card_filter(&document);
            } else if key == "Escape" {
                event.prevent_default();
                clear_all_card_filters(&document);
            } else if key.eq_ignore_ascii_case("r") && (event.ctrl_key() || event.meta_key()) {
                event.prevent_default();
                let window = window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = refresh_route_data(&window).await;
                });
            } else if matches!(key.as_str(), "1" | "2" | "3" | "4" | "5") {
                event.prevent_default();
                let rating = key.parse::<u32>().unwrap_or_default();
                let key = document
                    .query_selector("[data-slskr-player]")
                    .ok()
                    .flatten()
                    .and_then(|player| player.get_attribute("data-slskr-player-rating-key"))
                    .unwrap_or_default();
                if key.is_empty() {
                    set_player_status(&document, "No track selected");
                    return;
                }
                let current = read_player_rating(&window, &key);
                let next = if current == rating { 0 } else { rating };
                write_player_rating(&window, &key, next);
                update_player_rating_controls(&window, &document, &key);
                set_player_status(&document, player_rating_summary(next));
            } else if key.eq_ignore_ascii_case("v") {
                event.prevent_default();
                let window = window.clone();
                let document = document.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = fetch_text_with_method(
                        &window,
                        &endpoint_url("/player/external-visualizer/launch"),
                        "POST",
                        None,
                    )
                    .await;
                    match result {
                        Ok(body) => set_player_status(&document, &compact_preview(&body)),
                        Err(error) => {
                            let message = error
                                .as_string()
                                .unwrap_or_else(|| "visualizer request failed".to_string());
                            set_player_status(&document, &message);
                        }
                    }
                    let _ = refresh_player_status(&window).await;
                });
            } else if key.eq_ignore_ascii_case("q") {
                event.prevent_default();
                open_player_radio_search(&window, &document);
            } else if key.eq_ignore_ascii_case("k") || key == " " {
                event.prevent_default();
                let window = window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = refresh_player_status(&window).await;
                });
            }
        },
    ));
    listener_document
        .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn keyboard_event_started_in_text_control(document: &web_sys::Document) -> bool {
    document
        .active_element()
        .map(|element| matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"))
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn focus_first_card_filter(document: &web_sys::Document) {
    if let Ok(Some(filter)) = document.query_selector(".slskr-card-filter") {
        if let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() {
            let _ = input.focus();
            let _ = input.select();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_all_card_filters(document: &web_sys::Document) {
    if let Ok(filters) = document.query_selector_all(".slskr-card-filter") {
        for filter_index in 0..filters.length() {
            let Some(filter) = filters.item(filter_index) else {
                continue;
            };
            let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            input.set_value("");
        }
    }

    if let Ok(rows) = document.query_selector_all("[data-slskr-row-text]") {
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

    if let Ok(cards) = document.query_selector_all("[data-slskr-data-card]") {
        for card_index in 0..cards.length() {
            let Some(card) = cards.item(card_index) else {
                continue;
            };
            let Ok(card) = card.dyn_into::<web_sys::Element>() else {
                continue;
            };
            update_data_card_count(&card);
        }
    }

    set_live_status(document, "Filters cleared");
}

#[cfg(target_arch = "wasm32")]
fn set_live_status(document: &web_sys::Document, message: &str) {
    if let Some(status) = document.get_element_by_id("slskr-live-status") {
        status.set_text_content(Some(message));
    }
}

pub fn player_rating_key(track: &serde_json::Value) -> String {
    if let Some(content_id) = track
        .get("contentId")
        .or_else(|| track.get("content_id"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        return format!("content:{content_id}");
    }
    if let Some(stream_url) = track
        .get("streamUrl")
        .or_else(|| track.get("stream_url"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        return format!("stream:{stream_url}");
    }
    let parts = ["artist", "album", "title", "fileName", "filename"]
        .iter()
        .filter_map(|key| {
            track
                .get(*key)
                .map(json_scalar_preview)
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        String::new()
    } else {
        format!("meta:{}", parts.join("|"))
    }
}

pub fn player_rating_summary(rating: u32) -> &'static str {
    match rating {
        1 | 2 => "Discovery caution",
        3 => "Neutral rating",
        4 | 5 => "Discovery boost",
        _ => "Not rated",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerRadioQuery {
    pub id: String,
    pub query: String,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerRadioPlan {
    pub basis: Vec<String>,
    pub primary_query: String,
    pub queries: Vec<PlayerRadioQuery>,
    pub ready: bool,
    pub seed_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimilarQueueCandidate {
    pub index: usize,
    pub item: serde_json::Value,
    pub score: u32,
}

fn json_track_field(track: &serde_json::Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            track
                .get(*key)
                .map(json_scalar_preview)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn unique_nonempty(values: Vec<String>) -> Vec<String> {
    values.into_iter().filter(|value| !value.is_empty()).fold(
        Vec::<String>::new(),
        |mut unique, value| {
            if !unique.iter().any(|other| other == &value) {
                unique.push(value);
            }
            unique
        },
    )
}

fn unique_nonempty_case_insensitive(values: Vec<String>) -> Vec<String> {
    values.into_iter().filter(|value| !value.is_empty()).fold(
        Vec::<String>::new(),
        |mut unique, value| {
            if !unique
                .iter()
                .any(|other| other.eq_ignore_ascii_case(&value))
            {
                unique.push(value);
            }
            unique
        },
    )
}

fn player_radio_tags(track: &serde_json::Value) -> Vec<String> {
    for key in ["tags", "genres"] {
        let values = track
            .get(key)
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(json_scalar_preview)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !values.is_empty() {
            return values;
        }
    }
    json_track_field(track, &["genre"])
        .split('\n')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn build_player_radio_plan(track: Option<&serde_json::Value>) -> PlayerRadioPlan {
    let Some(track) = track else {
        return PlayerRadioPlan {
            basis: Vec::new(),
            primary_query: String::new(),
            queries: Vec::new(),
            ready: false,
            seed_label: "No track selected".to_string(),
        };
    };

    let artist = json_track_field(track, &["artist"]);
    let title = json_track_field(track, &["title", "fileName", "filename"]);
    let album = json_track_field(track, &["album"]);
    let tags = unique_nonempty(player_radio_tags(track));
    let track_query = unique_nonempty(vec![artist.clone(), title.clone()]).join(" ");
    let album_query = unique_nonempty(vec![artist.clone(), album.clone()]).join(" ");
    let genre_query = unique_nonempty(vec![
        artist.clone(),
        tags.first().cloned().unwrap_or_default(),
    ])
    .join(" ");
    let artist_query = artist.clone();
    let queries = unique_nonempty(vec![
        track_query.clone(),
        album_query.clone(),
        genre_query.clone(),
        artist_query,
    ])
    .into_iter()
    .enumerate()
    .map(|(index, query)| {
        let reason = if query == track_query {
            "Similar track seed"
        } else if query == album_query {
            "Album neighborhood"
        } else if query == genre_query {
            "Artist and genre seed"
        } else {
            "Artist radio seed"
        };
        PlayerRadioQuery {
            id: format!("radio-query-{}", index + 1),
            query,
            reason,
        }
    })
    .collect::<Vec<_>>();
    let seed_label = unique_nonempty(vec![artist.clone(), title.clone()]).join(" - ");
    PlayerRadioPlan {
        basis: vec![
            artist
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Artist: {artist}")),
            title
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Track: {title}")),
            album
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Album: {album}")),
            tags.is_empty().then(String::new).unwrap_or_else(|| {
                format!(
                    "Tags: {}",
                    tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )
            }),
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect(),
        primary_query: queries
            .first()
            .map(|query| query.query.clone())
            .unwrap_or_default(),
        ready: !queries.is_empty(),
        queries,
        seed_label: if !seed_label.is_empty() {
            seed_label
        } else if !title.is_empty() {
            title
        } else if !artist.is_empty() {
            artist
        } else {
            "Untitled seed".to_string()
        },
    }
}

fn percent_encode_query(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (*byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn build_player_radio_search_path(query: &str) -> String {
    let normalized = query.trim();
    if normalized.is_empty() {
        "/searches".to_string()
    } else {
        format!("/searches?q={}", percent_encode_query(normalized))
    }
}

pub fn player_radio_queries(plan: &PlayerRadioPlan, limit: usize) -> Vec<String> {
    unique_nonempty_case_insensitive(
        plan.queries
            .iter()
            .map(|item| item.query.clone())
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .take(limit)
    .collect()
}

fn quote_if_needed(value: &str) -> String {
    let normalized = value.trim();
    if normalized.is_empty() {
        String::new()
    } else if normalized.chars().any(char::is_whitespace) {
        format!("\"{normalized}\"")
    } else {
        normalized.to_string()
    }
}

pub fn player_radio_copy_text(plan: &PlayerRadioPlan) -> String {
    if !plan.ready {
        return String::new();
    }
    let mut lines = vec![format!("Smart radio seed: {}", plan.seed_label)];
    lines.extend(
        plan.queries
            .iter()
            .map(|item| format!("{}: {}", item.reason, quote_if_needed(&item.query))),
    );
    lines.join("\n")
}

fn player_auto_queue_tags(item: &serde_json::Value) -> Vec<String> {
    ["tags", "genres"]
        .iter()
        .flat_map(|key| {
            item.get(*key)
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .map(|value| json_scalar_preview(&value).trim().to_lowercase())
        .chain(std::iter::once(
            json_track_field(item, &["genre"]).to_lowercase(),
        ))
        .filter(|value| !value.is_empty())
        .collect()
}

fn player_title_tokens(item: &serde_json::Value) -> Vec<String> {
    json_track_field(item, &["title", "fileName", "filename"])
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == ' ' {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .map(ToOwned::to_owned)
        .collect()
}

pub fn player_similarity_score(current: &serde_json::Value, candidate: &serde_json::Value) -> u32 {
    let mut score = 0;
    let current_artist = json_track_field(current, &["artist"]).to_lowercase();
    let candidate_artist = json_track_field(candidate, &["artist"]).to_lowercase();
    if !current_artist.is_empty() && current_artist == candidate_artist {
        score += 4;
    }
    let current_album = json_track_field(current, &["album"]).to_lowercase();
    let candidate_album = json_track_field(candidate, &["album"]).to_lowercase();
    if !current_album.is_empty() && current_album == candidate_album {
        score += 3;
    }

    let current_tags = player_auto_queue_tags(current);
    let shared_tags = player_auto_queue_tags(candidate)
        .iter()
        .filter(|tag| current_tags.iter().any(|current_tag| current_tag == *tag))
        .count() as u32;
    score += (shared_tags * 2).min(4);

    let current_tokens = player_title_tokens(current);
    let shared_title_tokens = player_title_tokens(candidate)
        .iter()
        .filter(|token| {
            current_tokens
                .iter()
                .any(|current_token| current_token == *token)
        })
        .count() as u32;
    score += shared_title_tokens.min(2);
    score
}

pub fn build_similar_queue_candidates(
    current: Option<&serde_json::Value>,
    history: &[serde_json::Value],
    queue: &[serde_json::Value],
    limit: usize,
) -> Vec<SimilarQueueCandidate> {
    let Some(current) = current else {
        return Vec::new();
    };
    let mut seen = queue
        .iter()
        .filter_map(|item| {
            item.get("contentId")
                .or_else(|| item.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())
        })
        .collect::<Vec<_>>();
    let mut candidates = history
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let content_id = item
                .get("contentId")
                .or_else(|| item.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())?;
            if seen.iter().any(|seen_id| seen_id == &content_id) {
                return None;
            }
            let score = player_similarity_score(current, item);
            if score == 0 {
                return None;
            }
            seen.push(content_id);
            Some(SimilarQueueCandidate {
                index,
                item: item.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.index.cmp(&right.index))
    });
    candidates.truncate(limit);
    candidates
}

pub fn similar_queue_search_queries(
    candidates: &[SimilarQueueCandidate],
    limit: usize,
) -> Vec<String> {
    unique_nonempty_case_insensitive(
        candidates
            .iter()
            .map(|candidate| {
                unique_nonempty(vec![
                    json_track_field(&candidate.item, &["artist"]),
                    json_track_field(&candidate.item, &["title", "fileName", "filename"]),
                ])
                .join(" ")
            })
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .take(limit)
    .collect()
}

pub fn player_radio_query_from_now_playing_body(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| current_player_track(&value))
        .map(|track| build_player_radio_plan(Some(&track)).primary_query)
        .unwrap_or_default()
}

fn current_player_track(value: &serde_json::Value) -> Option<serde_json::Value> {
    value
        .get("now_playing")
        .and_then(|entry| entry.as_array())
        .and_then(|items| items.first())
        .or_else(|| value.get("current"))
        .or_else(|| value.get("track"))
        .or(Some(value))
        .cloned()
}

#[cfg(target_arch = "wasm32")]
fn player_ratings_storage(window: &web_sys::Window) -> Option<web_sys::Storage> {
    window.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn read_player_rating(window: &web_sys::Window, key: &str) -> u32 {
    if key.is_empty() {
        return 0;
    }
    player_ratings_storage(window)
        .and_then(|storage| storage.get_item("slskr.player.ratings").ok().flatten())
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
        .and_then(|value| value.get(key).and_then(|rating| rating.as_u64()))
        .and_then(|rating| u32::try_from(rating).ok())
        .filter(|rating| (1..=5).contains(rating))
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn write_player_rating(window: &web_sys::Window, key: &str, rating: u32) {
    if key.is_empty() {
        return;
    }
    let Some(storage) = player_ratings_storage(window) else {
        return;
    };
    let mut ratings = storage
        .get_item("slskr.player.ratings")
        .ok()
        .flatten()
        .and_then(|body| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body).ok()
        })
        .unwrap_or_default();
    if (1..=5).contains(&rating) {
        ratings.insert(key.to_string(), serde_json::Value::from(rating));
    } else {
        ratings.remove(key);
    }
    let _ = storage.set_item(
        "slskr.player.ratings",
        &serde_json::Value::Object(ratings).to_string(),
    );
}

#[cfg(target_arch = "wasm32")]
fn update_player_rating_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
    rating_key: &str,
) {
    let rating = read_player_rating(window, rating_key);
    if let Ok(Some(player)) = document.query_selector("[data-slskr-player]") {
        let _ = player.set_attribute("data-slskr-player-rating-key", rating_key);
    }
    if let Ok(buttons) = document.query_selector_all("[data-slskr-player-rating]") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let Ok(button) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let value = button
                .get_attribute("data-slskr-player-rating")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or_default();
            let class = if value <= rating && rating > 0 {
                "is-active"
            } else {
                ""
            };
            let _ = button.set_attribute("class", class);
        }
    }
    if let Some(status) = document.get_element_by_id("slskr-player-rating-status") {
        status.set_text_content(Some(player_rating_summary(rating)));
    }
}

#[cfg(target_arch = "wasm32")]
fn update_player_radio_controls(document: &web_sys::Document, query: &str) {
    if let Ok(Some(player)) = document.query_selector("[data-slskr-player]") {
        let _ = player.set_attribute("data-slskr-player-radio-query", query);
    }
    if let Some(status) = document.get_element_by_id("slskr-player-radio") {
        status.set_text_content(Some(if query.is_empty() {
            "No track selected"
        } else {
            query
        }));
    }
}

#[cfg(target_arch = "wasm32")]
fn open_player_radio_search(window: &web_sys::Window, document: &web_sys::Document) {
    let query = document
        .query_selector("[data-slskr-player]")
        .ok()
        .flatten()
        .and_then(|player| player.get_attribute("data-slskr-player-radio-query"))
        .unwrap_or_default();
    if query.trim().is_empty() {
        set_player_status(document, "No track selected");
        return;
    }
    let path = build_player_radio_search_path(&query);
    if let Ok(history) = window.history() {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&path));
    }
    let _ = render_current_route(window, document);
    if let Ok(Some(input)) = document.query_selector(".slskr-toolbar-input") {
        if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value(&query);
            let _ = input.focus();
            let _ = input.select();
        }
    }
    set_player_status(document, &format!("Smart radio search ready: {query}"));
}

#[cfg(target_arch = "wasm32")]
fn mount_player_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-player-action]")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let action = button
            .get_attribute("data-slskr-player-action")
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let action = action.clone();
                let window = window.clone();
                let document = document.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    set_player_status(&document, "Player action running");
                    if action == "radio" {
                        open_player_radio_search(&window, &document);
                        return;
                    }
                    let result = match action.as_str() {
                        "clear" => {
                            fetch_text_with_method(
                                &window,
                                &endpoint_url("/nowplaying"),
                                "DELETE",
                                None,
                            )
                            .await
                        }
                        "visualizer" => {
                            fetch_text_with_method(
                                &window,
                                &endpoint_url("/player/external-visualizer/launch"),
                                "POST",
                                None,
                            )
                            .await
                        }
                        _ => Ok(String::new()),
                    };
                    match result {
                        Ok(body) if !body.is_empty() => {
                            set_player_status(&document, &compact_preview(&body));
                        }
                        Ok(_) => set_player_status(&document, "Player refreshed"),
                        Err(error) => {
                            let message = error
                                .as_string()
                                .unwrap_or_else(|| "player request failed".to_string());
                            set_player_status(&document, &message);
                        }
                    }
                    let _ = refresh_player_status(&window).await;
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    let rating_buttons = document.query_selector_all("[data-slskr-player-rating]")?;
    for index in 0..rating_buttons.length() {
        let Some(node) = rating_buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let value = button
            .get_attribute("data-slskr-player-rating")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let key = document
                    .query_selector("[data-slskr-player]")
                    .ok()
                    .flatten()
                    .and_then(|player| player.get_attribute("data-slskr-player-rating-key"))
                    .unwrap_or_default();
                if key.is_empty() {
                    set_player_status(&document, "No track selected");
                    return;
                }
                let current = read_player_rating(&window, &key);
                let next = if current == value { 0 } else { value };
                write_player_rating(&window, &key, next);
                update_player_rating_controls(&window, &document, &key);
                set_player_status(&document, player_rating_summary(next));
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn set_player_status(document: &web_sys::Document, message: &str) {
    if let Some(status) = document.get_element_by_id("slskr-player-status") {
        status.set_text_content(Some(message));
    }
}

#[cfg(target_arch = "wasm32")]
fn update_data_card_count(card: &web_sys::Element) {
    let Ok(Some(count)) = card.query_selector("[data-slskr-card-count]") else {
        return;
    };
    let Ok(rows) = card.query_selector_all(".slskr-record-list [data-slskr-row-text]") else {
        return;
    };
    let total = rows.length();
    let mut visible = 0;
    for row_index in 0..rows.length() {
        let Some(row) = rows.item(row_index) else {
            continue;
        };
        let Ok(row) = row.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if !row.has_attribute("hidden") {
            visible += 1;
        }
    }
    count.set_text_content(Some(&format!("{visible} / {total}")));
}

#[cfg(target_arch = "wasm32")]
fn sort_data_card_table(card: &web_sys::Element, button: &web_sys::Element) {
    let column = button
        .get_attribute("data-slskr-sort-index")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_default();
    let next_direction = match button.get_attribute("data-slskr-sort-direction").as_deref() {
        Some("asc") => "desc",
        _ => "asc",
    };
    let Ok(Some(tbody)) = card.query_selector(".slskr-data-table tbody") else {
        return;
    };
    let Ok(rows) = tbody.query_selector_all("tr") else {
        return;
    };
    let mut elements = Vec::new();
    for row_index in 0..rows.length() {
        let Some(row) = rows.item(row_index) else {
            continue;
        };
        let Ok(row) = row.dyn_into::<web_sys::Element>() else {
            continue;
        };
        elements.push(row);
    }
    elements.sort_by(|left, right| {
        let left_value = table_cell_text(left, column);
        let right_value = table_cell_text(right, column);
        if next_direction == "asc" {
            left_value.cmp(&right_value)
        } else {
            right_value.cmp(&left_value)
        }
    });
    if let Ok(buttons) = card.query_selector_all("[data-slskr-sort-index]") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("data-slskr-sort-direction");
            let _ = element.remove_attribute("aria-sort");
        }
    }
    let _ = button.set_attribute("data-slskr-sort-direction", next_direction);
    let _ = button.set_attribute(
        "aria-sort",
        if next_direction == "asc" {
            "ascending"
        } else {
            "descending"
        },
    );
    for row in elements {
        let _ = tbody.append_child(&row);
    }
}

#[cfg(target_arch = "wasm32")]
fn table_cell_text(row: &web_sys::Element, column: u32) -> String {
    let selector = format!("td:nth-child({})", column + 1);
    row.query_selector(&selector)
        .ok()
        .flatten()
        .and_then(|cell| cell.text_content())
        .unwrap_or_default()
        .to_lowercase()
}

#[cfg(target_arch = "wasm32")]
fn select_data_card_record(card: &web_sys::Element, row: &web_sys::Element) {
    if let Ok(rows) = card.query_selector_all("[data-slskr-record-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("aria-selected");
            let _ = element.set_attribute("class", "");
        }
    }
    let _ = row.set_attribute("aria-selected", "true");
    let _ = row.set_attribute("class", "is-selected");

    let title = row
        .get_attribute("data-slskr-record-title")
        .unwrap_or_else(|| "Selected Record".to_string());
    let detail = row
        .get_attribute("data-slskr-record-detail")
        .unwrap_or_default();
    let raw = row
        .get_attribute("data-slskr-record-json")
        .unwrap_or_default();

    if let Ok(Some(header)) = card.query_selector(".slskr-card-inspector h4") {
        header.set_text_content(Some(&title));
    }
    if let Ok(Some(description)) = card.query_selector(".slskr-card-inspector p") {
        description.set_text_content(Some(&detail));
    }
    if let Ok(Some(pre)) = card.query_selector(".slskr-card-inspector pre") {
        pre.set_text_content(Some(&raw));
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_workspace_tabs(document: &web_sys::Document) -> Result<(), JsValue> {
    let tabs = document.query_selector_all(".slskr-workspace-tab")?;
    for index in 0..tabs.length() {
        let Some(node) = tabs.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let Some(target) = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };
                let mode = target
                    .get_attribute("data-slskr-workspace-mode")
                    .unwrap_or_else(|| "all".to_string());

                if let Ok(tabs) = document.query_selector_all(".slskr-workspace-tab") {
                    for index in 0..tabs.length() {
                        let Some(node) = tabs.item(index) else {
                            continue;
                        };
                        let Ok(tab) = node.dyn_into::<web_sys::Element>() else {
                            continue;
                        };
                        let active = tab
                            .get_attribute("data-slskr-workspace-mode")
                            .is_some_and(|tab_mode| tab_mode == mode);
                        let class = if active {
                            "slskr-workspace-tab is-active"
                        } else {
                            "slskr-workspace-tab"
                        };
                        let _ = tab.set_attribute("class", class);
                        let _ = tab
                            .set_attribute("aria-selected", if active { "true" } else { "false" });
                    }
                }

                if let Ok(Some(grid)) = document.query_selector("[data-slskr-workspace-grid]") {
                    let class = match mode.as_str() {
                        "primary" => "slskr-workspace-grid mode-primary",
                        "secondary" => "slskr-workspace-grid mode-secondary",
                        _ => "slskr-workspace-grid",
                    };
                    let _ = grid.set_attribute("class", class);
                }
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn mount_toolbar_actions(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-toolbar-command")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let Some(action_index) = button
            .get_attribute("data-slskr-toolbar-action")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            continue;
        };
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let value = document
                    .query_selector(".slskr-toolbar-input")
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_default();
                let route_path = window.location().pathname().unwrap_or_default();
                let Some(action) = route_action_at(&route_path, action_index) else {
                    return;
                };
                let body = action_body_from_value(action.body, &value);
                let window = window.clone();
                let document = document.clone();
                let method = action.method.to_string();
                let path = concrete_action_path(&route_path, action);
                wasm_bindgen_futures::spawn_local(async move {
                    let result =
                        fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        match result {
                            Ok(response) => status.set_inner_html(&format!(
                                "<strong>{}</strong> {}",
                                escape_html(&method),
                                escape_html(&compact_preview(&response))
                            )),
                            Err(error) => {
                                let message = error
                                    .as_string()
                                    .unwrap_or_else(|| "request failed".to_string());
                                status.set_inner_html(&format!(
                                    "<strong>{}</strong> {}",
                                    escape_html(&method),
                                    escape_html(&message)
                                ));
                            }
                        }
                    }
                    let _ = refresh_route_data(&window).await;
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn mount_route_actions(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-action-button")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let Some(action_index) = button
            .get_attribute("data-slskr-action-index")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            continue;
        };
        let input_selector = format!(
            "#slskr-route-actions li:nth-child({}) .slskr-action-input",
            index + 1
        );
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |_event: web_sys::MouseEvent| {
                let value = document
                    .query_selector(&input_selector)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_default();
                let route_path = window.location().pathname().unwrap_or_default();
                let Some(action) = route_action_at(&route_path, action_index) else {
                    return;
                };
                let body = action_body_from_value(action.body, &value);
                let window = window.clone();
                let document = document.clone();
                let method = action.method.to_string();
                let path = concrete_action_path(&route_path, action);
                wasm_bindgen_futures::spawn_local(async move {
                    let result =
                        fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        match result {
                            Ok(response) => status.set_inner_html(&format!(
                                "<strong>{}</strong> {}",
                                escape_html(&method),
                                escape_html(&compact_preview(&response))
                            )),
                            Err(error) => {
                                let message = error
                                    .as_string()
                                    .unwrap_or_else(|| "request failed".to_string());
                                status.set_inner_html(&format!(
                                    "<strong>{}</strong> {}",
                                    escape_html(&method),
                                    escape_html(&message)
                                ));
                            }
                        }
                    }
                    let _ = refresh_route_data(&window).await;
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn refresh_runtime_status() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let Some(status) = document.get_element_by_id("slskr-runtime-status") else {
        return Ok(());
    };

    let mut rendered = String::new();
    for probe in runtime_probes() {
        let path = endpoint_url(probe.path);
        let result = fetch_text(&window, &path).await;
        let row = match result {
            Ok(body) => runtime_probe_result_html(&[(probe.label, &path, Ok(body.as_str()))]),
            Err(error) => {
                let message = error
                    .as_string()
                    .unwrap_or_else(|| "request failed".to_string());
                runtime_probe_result_html(&[(probe.label, &path, Err(message.as_str()))])
            }
        };
        rendered.push_str(&row);
        status.set_inner_html(&rendered);
    }

    Ok(())
}

pub fn player_now_playing_text(body: &str) -> (String, String) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return (
            "Queue idle".to_string(),
            "No local stream selected".to_string(),
        );
    };
    let current = value
        .get("now_playing")
        .and_then(|entry| entry.as_array())
        .and_then(|items| items.first())
        .or_else(|| value.get("current"))
        .or_else(|| value.get("track"))
        .unwrap_or(&value);
    let title = current
        .get("title")
        .or_else(|| current.get("fileName"))
        .or_else(|| current.get("filename"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Queue idle".to_string());
    let artist = current
        .get("artist")
        .or_else(|| current.get("username"))
        .map(json_scalar_preview)
        .unwrap_or_default();
    let album = current
        .get("album")
        .map(json_scalar_preview)
        .unwrap_or_default();
    let detail = [artist, album]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let detail = if detail.is_empty() {
        "No local stream selected".to_string()
    } else {
        detail
    };
    (title, detail)
}

pub fn player_transfer_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "0 down / 0 up".to_string();
    };
    let downloads = value
        .get("downloads")
        .or_else(|| value.get("downloadSpeed"))
        .or_else(|| value.get("down"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    let uploads = value
        .get("uploads")
        .or_else(|| value.get("uploadSpeed"))
        .or_else(|| value.get("up"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    format!("{downloads} down / {uploads} up")
}

pub fn player_party_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "Listening party idle".to_string();
    };
    let count = value
        .get("count")
        .or_else(|| value.get("active"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    format!("{count} listening parties")
}

pub fn player_visualizer_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "Visualizer status unavailable".to_string();
    };
    value
        .get("status")
        .or_else(|| value.get("next_action"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Visualizer status unavailable".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn refresh_player_status(window: &web_sys::Window) -> Result<(), JsValue> {
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    set_player_status(&document, "Refreshing player");

    if let Ok(body) = fetch_text(window, &endpoint_url("/nowplaying")).await {
        let (title, detail) = player_now_playing_text(&body);
        let track = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| current_player_track(&value));
        let rating_key = track.as_ref().map(player_rating_key).unwrap_or_default();
        let radio_query = track
            .as_ref()
            .map(|track| build_player_radio_plan(Some(track)).primary_query)
            .unwrap_or_default();
        if let Some(element) = document.get_element_by_id("slskr-player-now") {
            element.set_text_content(Some(&title));
        }
        if let Some(element) = document.get_element_by_id("slskr-player-now-detail") {
            element.set_text_content(Some(&detail));
        }
        update_player_rating_controls(window, &document, &rating_key);
        update_player_radio_controls(&document, &radio_query);
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/transfers/speeds")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-transfers") {
            element.set_text_content(Some(&player_transfer_text(&body)));
        }
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/listening-party")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-party") {
            element.set_text_content(Some(&player_party_text(&body)));
        }
    }

    if let Ok(body) = fetch_text(window, &endpoint_url("/player/external-visualizer")).await {
        if let Some(element) = document.get_element_by_id("slskr-player-visualizer") {
            element.set_text_content(Some(&player_visualizer_text(&body)));
        }
    }

    set_player_status(&document, "Player status updated");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn refresh_route_data(window: &web_sys::Window) -> Result<(), JsValue> {
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let Some(status) = document.get_element_by_id("slskr-route-data") else {
        return Ok(());
    };
    let summary = document.get_element_by_id("slskr-route-summary");
    let page_data = document.get_element_by_id("slskr-page-data");
    let path = window.location().pathname()?;
    let Some(page) = route_page(&path) else {
        return Ok(());
    };
    set_live_status(&document, "Refreshing live data");

    let mut rendered = String::new();
    let mut responses = Vec::new();
    let mut errors = 0;
    for endpoint in route_endpoints(page.surface)
        .into_iter()
        .filter(|endpoint| endpoint.method == "GET")
    {
        let url = concrete_endpoint_path(&path, endpoint);
        let row = match fetch_text(window, &url).await {
            Ok(body) => {
                responses.push(EndpointBody {
                    endpoint,
                    body: body.clone(),
                });
                runtime_probe_result_html(&[(endpoint.method, &url, Ok(body.as_str()))])
            }
            Err(error) => {
                errors += 1;
                let message = error
                    .as_string()
                    .unwrap_or_else(|| "request failed".to_string());
                runtime_probe_result_html(&[(endpoint.method, &url, Err(message.as_str()))])
            }
        };
        rendered.push_str(&row);
        status.set_inner_html(&rendered);
        if let Some(summary) = summary.as_ref() {
            summary.set_inner_html(&route_summary_result_html(&path, &responses));
        }
        if let Some(page_data) = page_data.as_ref() {
            page_data.set_inner_html(&route_workspace_result_html(&path, &responses));
            mount_workspace_tabs(&document)?;
            mount_data_cards(&document)?;
        }
    }
    let message = if errors == 0 {
        format!("Updated {} live probes", responses.len())
    } else {
        format!("Updated {} live probes, {} errors", responses.len(), errors)
    };
    set_live_status(&document, &message);

    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text(window: &web_sys::Window, url: &str) -> Result<String, JsValue> {
    let response_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await?;
    let response: web_sys::Response = response_value.dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
    }
    let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    Ok(text.as_string().unwrap_or_default())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text_with_method(
    window: &web_sys::Window,
    url: &str,
    method: &str,
    body: Option<&str>,
) -> Result<String, JsValue> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    if let Some(body) = body {
        let headers = web_sys::Headers::new()?;
        headers.set("Content-Type", "application/json")?;
        init.set_headers(&headers);
        init.set_body(&JsValue::from_str(body));
    }
    let response_value =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init(url, &init)).await?;
    let response: web_sys::Response = response_value.dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
    }
    let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    Ok(text.as_string().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    const REACT_APP: &str = include_str!("../../../web/src/components/App.jsx");
    const STATIC_INDEX: &str = include_str!("../static/index.html");

    #[test]
    fn api_endpoints_are_versioned() {
        for section in app_sections() {
            assert!(endpoint_url(section.endpoint).starts_with("/api/v0/"));
        }
    }

    #[test]
    fn shell_contains_primary_routes() {
        let html = shell_html();
        for item in nav_items() {
            assert!(html.contains(item.label), "missing {}", item.label);
        }
        assert!(html.contains("Search, transfers, messages"));
        assert!(html.contains("slskr-player"));
        assert!(html.contains("data-slskr-player"));
        assert!(html.contains("data-slskr-player-action=\"refresh\""));
        assert!(html.contains("data-slskr-player-action=\"clear\""));
        assert!(html.contains("data-slskr-player-action=\"visualizer\""));
        assert!(html.contains("data-slskr-player-action=\"radio\""));
        assert!(html.contains("data-slskr-player-radio-query"));
        assert!(html.contains("data-slskr-player-rating=\"5\""));
        assert!(html.contains("slskr-player-rating-status"));
        assert!(html.contains("slskr-player-radio"));
        assert!(html.contains("slskr-player-now"));
        assert!(html.contains("slskr-player-transfers"));
        assert!(html.contains("/api/v0/searches"));
        assert!(html.contains("slskr-runtime-status"));
        assert!(html.contains("/api/v0/health"));
        assert!(html.contains("slskr-route-view"));
    }

    #[test]
    fn rust_player_surface_summarizes_live_payloads() {
        let (title, detail) = player_now_playing_text(
            r#"{"now_playing":[{"artist":"Archive Artist","album":"Open Sessions","title":"Public Domain Theme"}]}"#,
        );
        assert_eq!(title, "Public Domain Theme");
        assert_eq!(detail, "Archive Artist / Open Sessions");
        assert_eq!(
            player_transfer_text(r#"{"downloads":2,"uploads":1}"#),
            "2 down / 1 up"
        );
        assert_eq!(
            player_party_text(r#"{"count":3,"active_parties":[]}"#),
            "3 listening parties"
        );
        assert_eq!(
            player_visualizer_text(r#"{"status":"configured","configured":true}"#),
            "configured"
        );
        let track = serde_json::json!({
            "artist": "Archive Artist",
            "album": "Open Sessions",
            "title": "Public Domain Theme"
        });
        assert_eq!(
            player_rating_key(&track),
            "meta:archive artist|open sessions|public domain theme"
        );
        assert_eq!(player_rating_summary(1), "Discovery caution");
        assert_eq!(player_rating_summary(3), "Neutral rating");
        assert_eq!(player_rating_summary(5), "Discovery boost");
        assert_eq!(player_rating_summary(0), "Not rated");
        let radio_plan = build_player_radio_plan(Some(&track));
        assert!(radio_plan.ready);
        assert_eq!(
            radio_plan.seed_label,
            "Archive Artist - Public Domain Theme"
        );
        assert_eq!(
            radio_plan.queries,
            vec![
                PlayerRadioQuery {
                    id: "radio-query-1".to_string(),
                    query: "Archive Artist Public Domain Theme".to_string(),
                    reason: "Similar track seed",
                },
                PlayerRadioQuery {
                    id: "radio-query-2".to_string(),
                    query: "Archive Artist Open Sessions".to_string(),
                    reason: "Album neighborhood",
                },
                PlayerRadioQuery {
                    id: "radio-query-3".to_string(),
                    query: "Archive Artist".to_string(),
                    reason: "Artist and genre seed",
                },
            ]
        );
        assert_eq!(
            build_player_radio_search_path(&radio_plan.primary_query),
            "/searches?q=Archive%20Artist%20Public%20Domain%20Theme"
        );
        assert_eq!(
            player_radio_queries(&radio_plan, 2),
            vec![
                "Archive Artist Public Domain Theme".to_string(),
                "Archive Artist Open Sessions".to_string(),
            ]
        );
        assert!(player_radio_copy_text(&radio_plan)
            .contains("Similar track seed: \"Archive Artist Public Domain Theme\""));
        assert_eq!(
            player_radio_query_from_now_playing_body(
                r#"{"track":{"artist":"Archive Artist","title":"Public Domain Theme"}}"#
            ),
            "Archive Artist Public Domain Theme"
        );
        assert_eq!(
            build_player_radio_plan(None),
            PlayerRadioPlan {
                basis: Vec::new(),
                primary_query: String::new(),
                queries: Vec::new(),
                ready: false,
                seed_label: "No track selected".to_string(),
            }
        );
    }

    #[test]
    fn rust_player_surface_builds_similar_queue_candidates() {
        let current = serde_json::json!({
            "album": "Fixture Album",
            "artist": "Fixture Artist",
            "contentId": "sha256:current",
            "genre": "Fixture Genre",
            "title": "Current Song"
        });
        let history = vec![
            serde_json::json!({
                "album": "Fixture Album",
                "artist": "Fixture Artist",
                "contentId": "sha256:album-match",
                "title": "Album Match"
            }),
            serde_json::json!({
                "artist": "Other Artist",
                "contentId": "sha256:tag-match",
                "tags": ["Fixture Genre"],
                "title": "Tag Match"
            }),
            serde_json::json!({
                "artist": "Other Artist",
                "contentId": "sha256:miss",
                "title": "Miss"
            }),
        ];
        let queue = vec![serde_json::json!({"contentId": "sha256:current"})];
        let candidates = build_similar_queue_candidates(Some(&current), &history, &queue, 5);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.item["contentId"].as_str().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec!["sha256:album-match", "sha256:tag-match"]
        );
        assert_eq!(
            similar_queue_search_queries(&candidates, 3),
            vec![
                "Fixture Artist Album Match".to_string(),
                "Other Artist Tag Match".to_string(),
            ]
        );

        let duplicate_history = vec![
            serde_json::json!({
                "artist": "Fixture Artist",
                "contentId": "sha256:queued",
                "title": "Already Queued"
            }),
            serde_json::json!({
                "artist": "Fixture Artist",
                "contentId": "sha256:new",
                "title": "New Candidate"
            }),
            serde_json::json!({
                "artist": "Fixture Artist",
                "contentId": "sha256:new",
                "title": "Duplicate Candidate"
            }),
        ];
        let duplicate_queue = vec![
            serde_json::json!({"contentId": "sha256:current"}),
            serde_json::json!({"contentId": "sha256:queued"}),
        ];
        let candidates =
            build_similar_queue_candidates(Some(&current), &duplicate_history, &duplicate_queue, 5);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.item["contentId"].as_str().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec!["sha256:new"]
        );
        assert!(build_similar_queue_candidates(None, &history, &queue, 5).is_empty());
    }

    #[test]
    fn static_index_supports_direct_nested_route_loads() {
        assert!(STATIC_INDEX.contains("href=\"/styles.css\""));
        assert!(STATIC_INDEX.contains("import init from '/slskr_web.js'"));
        assert!(!STATIC_INDEX.contains("href=\"./styles.css\""));
        assert!(!STATIC_INDEX.contains("from './slskr_web.js'"));
    }

    #[test]
    fn runtime_probe_html_escapes_api_values() {
        let html = runtime_probe_result_html(&[(
            "Probe",
            "/api/v0/probe",
            Ok(r#"<script>"bad"</script>"#),
        )]);
        assert!(html.contains("&lt;script&gt;&quot;bad&quot;&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
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
    fn route_pages_render_api_surface() {
        let html = route_page_html("/downloads");
        assert!(html.contains("Downloads"));
        assert!(html.contains("/api/v0/transfers/downloads"));
        assert!(html.contains("data-route=\"/downloads\""));
        assert!(html.contains("slskr-route-data"));
        assert!(html.contains("Live Data"));
        assert!(html.contains("slskr-route-actions"));
        assert!(html.contains("slskr-route-summary"));
        assert!(html.contains("Summary"));
        assert!(html.contains("slskr-page-data"));
        assert!(html.contains("Route diagnostics"));
        assert!(html.contains("Clear Completed Downloads"));
        assert!(html.contains("data-slskr-refresh-route"));
        assert!(html.contains("data-slskr-focus-filter"));
        assert!(html.contains("data-slskr-clear-filters"));
        assert!(html.contains("slskr-live-status"));
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
        assert!(collections.contains("Add Library Item"));

        let integrations = route_page_html("/playlist-intake");
        assert!(integrations.contains("Preview Playlist"));
        assert!(integrations.contains("Build Discovery Graph"));
        assert!(integrations.contains("Track MusicBrainz Target"));
        assert!(integrations.contains("Create SongID Run"));
    }

    #[test]
    fn rust_action_bodies_are_json_safe() {
        assert_eq!(
            action_body_from_value(ActionBody::SearchText, "a \"b\"").unwrap(),
            r#"{"searchText":"a \"b\""}"#
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
            r#"{"collection_id":"rust-web-demo","username":"peer1"}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::ShareGroupMember, "peer1").unwrap(),
            r#"{"username":"peer1"}"#
        );
        assert!(
            action_body_from_value(ActionBody::FeedPreview, "artist - song")
                .unwrap()
                .contains("\"sourceText\":\"artist - song\"")
        );
        assert!(action_body_from_value(ActionBody::None, "ignored").is_none());
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

        assert!(route_action_at("/searches/42", usize::MAX).is_none());
        assert!(route_action_at("/not-a-route", 0).is_none());
    }

    #[test]
    fn rust_route_summaries_parse_live_response_shapes() {
        let search = route_summary_result_html(
            "/searches/42",
            &[
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/searches/records",
                        surface: "search",
                    },
                    body: r#"{"entries":[{"id":"1"},{"id":"2"}]}"#.to_string(),
                },
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/searches/:id/responses",
                        surface: "search",
                    },
                    body: r#"[{"username":"peer1"}]"#.to_string(),
                },
            ],
        );
        assert!(search.contains(">2<"));
        assert!(search.contains(">1<"));
        assert!(search.contains("active records"));

        let transfers = route_summary_result_html(
            "/downloads",
            &[
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/transfers/downloads",
                        surface: "transfers",
                    },
                    body: r#"[{"username":"peer1"}]"#.to_string(),
                },
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/transfers/uploads",
                        surface: "transfers",
                    },
                    body: "[]".to_string(),
                },
            ],
        );
        assert!(transfers.contains("Downloads"));
        assert!(transfers.contains(">1<"));
        assert!(transfers.contains("Uploads"));

        let rooms = route_summary_result_html(
            "/rooms",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/rooms/joined",
                    surface: "rooms",
                },
                body: r#"["contract-room"]"#.to_string(),
            }],
        );
        assert!(rooms.contains("Joined"));
        assert!(rooms.contains(">1<"));
    }

    #[test]
    fn rust_route_summaries_escape_live_response_values() {
        let html = route_summary_result_html(
            "/messages",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/conversations/:username",
                    surface: "messages",
                },
                body: r#"{"username":"<script>"}"#.to_string(),
            }],
        );
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
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
                REACT_APP.contains(&format!("path=\"{expected}\""))
                    || REACT_APP.contains(&format!("to=\"{expected}\"")),
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
                REACT_APP.contains(&format!("to=\"{}\"", item.href)),
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
            ("POST", "/wishlist/wish-demo/search"),
            ("POST", "/contacts"),
            ("POST", "/contacts/from-discovery"),
            ("POST", "/contacts/from-invite"),
            ("POST", "/users/watch"),
            ("POST", "/users/notes"),
            ("POST", "/collections"),
            ("POST", "/sharegroups"),
            ("POST", "/sharegroups/group-demo/members"),
            ("POST", "/share-grants"),
            ("PUT", "/share-grants/grant-demo"),
            ("POST", "/share-grants/grant-demo/backfill"),
            ("POST", "/share-grants/grant-demo/token"),
            ("DELETE", "/share-grants/grant-demo"),
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
}
