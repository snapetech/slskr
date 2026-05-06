use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteKind {
    Search,
    DiscoveryGraph,
    PlaylistIntake,
    Wishlist,
    Downloads,
    Uploads,
    Messages,
    Rooms,
    Users,
    Contacts,
    Solid,
    Collections,
    ShareGroups,
    SharedWithMe,
    Browse,
    System,
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
            path: "/transfers/downloads/:username/:id",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/transfers/uploads/all/completed",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/transfers/uploads/:username/:id",
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
            method: "DELETE",
            path: "/contacts/:id",
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
        ApiEndpoint {
            method: "POST",
            path: "/session/privileges/check",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/diagnostics",
            surface: "system",
        },
        ApiEndpoint {
            method: "POST",
            path: "/admin/shutdown",
            surface: "system",
        },
        ApiEndpoint {
            method: "POST",
            path: "/admin/restart",
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
            path: "/wishlist/:id/search",
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
            label: "Cancel Download",
            method: "DELETE",
            path: "/transfers/downloads/:username/:id",
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
            body: ActionBody::None,
            label: "Deny Upload",
            method: "DELETE",
            path: "/transfers/uploads/:username/:id",
            surface: "transfers",
        },
        RouteAction {
            body: ActionBody::EnabledTrue,
            label: "Allow Upload",
            method: "PUT",
            path: "/transfers/uploads/:username/:id",
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
            body: ActionBody::None,
            label: "Remove Contact",
            method: "DELETE",
            path: "/contacts/:id",
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
            path: "/sharegroups/:id/members",
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
            path: "/share-grants/:id",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Backfill Share Grant",
            method: "POST",
            path: "/share-grants/:id/backfill",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Issue Share Token",
            method: "POST",
            path: "/share-grants/:id/token",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Delete Share Grant",
            method: "DELETE",
            path: "/share-grants/:id",
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
            body: ActionBody::None,
            label: "Resolve WebID",
            method: "GET",
            path: "/solid/status",
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
        RouteAction {
            body: ActionBody::None,
            label: "Check for Updates",
            method: "GET",
            path: "/version",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Get Privileges",
            method: "POST",
            path: "/session/privileges/check",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Diagnostic Bundle",
            method: "GET",
            path: "/diagnostics",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Setup Health",
            method: "GET",
            path: "/health",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Shut Down",
            method: "POST",
            path: "/admin/shutdown",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Restart",
            method: "POST",
            path: "/admin/restart",
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
    concrete_action_path_with_target(route_path, action, None)
}

pub fn concrete_action_path_with_target(
    route_path: &str,
    action: RouteAction,
    target: Option<&str>,
) -> String {
    concrete_action_path_with_target_and_id(route_path, action, target, None)
}

pub fn concrete_action_path_with_target_and_id(
    route_path: &str,
    action: RouteAction,
    target: Option<&str>,
    id: Option<&str>,
) -> String {
    let selected_id = id
        .filter(|value| safe_route_segment(value))
        .or_else(|| target.filter(|value| safe_route_segment(value)));
    let search_id = selected_id.unwrap_or_else(|| {
        if action.path.contains(":id") && !normalize_route_path(route_path).contains(":id") {
            "1"
        } else {
            // Keep the owned route parameter alive below by falling back after this branch.
            ""
        }
    });
    let route_search_id;
    let search_id = if search_id.is_empty() {
        route_search_id = route_param_value(route_path, "1");
        route_search_id.as_str()
    } else {
        search_id
    };
    let target = target.filter(|value| safe_route_segment(value)).unwrap_or(
        if action.path.contains(":roomName") {
            "contract-room"
        } else {
            "peer1"
        },
    );
    endpoint_url(action.path)
        .replace(":id", &search_id)
        .replace(":username", target)
        .replace(":roomName", target)
}

fn safe_route_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

pub fn route_action_at(path: &str, index: usize) -> Option<RouteAction> {
    let page = route_page(path)?;
    surface_actions(page.surface).get(index).copied()
}

pub fn route_action_for_native_label(path: &str, label: &str) -> Option<RouteAction> {
    let label = label.trim();
    if label.is_empty() {
        return None;
    }
    let normalized = label.to_ascii_lowercase();
    let aliases: &[&str] = match (route_kind(path), normalized.as_str()) {
        (RouteKind::Search, "search" | "queue search" | "search and open results") => {
            &["Start Search"]
        }
        (RouteKind::Search, "stop") => &["Stop Search"],
        (RouteKind::Search, "clear") => &["Clear Searches"],
        (RouteKind::Search, "download" | "queue selected") => &["Queue Download"],
        (RouteKind::DiscoveryGraph, "build graph" | "build atlas" | "queue nearby") => {
            &["Build Discovery Graph"]
        }
        (RouteKind::PlaylistIntake, "preview playlist" | "import playlist") => {
            &["Preview Playlist"]
        }
        (RouteKind::PlaylistIntake, "queue plans") => &["Queue Discography Job"],
        (RouteKind::Wishlist, "add wanted search" | "add search" | "add your first search") => {
            &["Add Wishlist Item"]
        }
        (RouteKind::Wishlist, "run selected" | "run enabled" | "run") => &["Run Wishlist Search"],
        (RouteKind::Downloads, "download" | "queue download" | "retry" | "retry all") => {
            &["Queue Download"]
        }
        (RouteKind::Downloads, "cancel" | "cancel all" | "remove") => &["Cancel Download"],
        (RouteKind::Downloads, "clear completed") => &["Clear Completed Downloads"],
        (RouteKind::Downloads, "enable acceleration") => &["Enable Accelerated Downloads"],
        (RouteKind::Uploads, "clear completed") => &["Clear Completed Uploads"],
        (RouteKind::Uploads, "allow selected" | "allow") => &["Allow Upload"],
        (RouteKind::Uploads, "deny selected" | "deny") => &["Deny Upload"],
        (RouteKind::Messages | RouteKind::Rooms, "reply" | "direct message" | "send message") => {
            &["Send Message"]
        }
        (RouteKind::Messages | RouteKind::Rooms, "acknowledge") => &["Acknowledge Conversation"],
        (RouteKind::Messages | RouteKind::Rooms, "join" | "join room") => &["Join Room"],
        (RouteKind::Messages | RouteKind::Rooms, "leave" | "leave room") => &["Leave Room"],
        (RouteKind::Users, "watch") => &["Watch User"],
        (RouteKind::Users, "save note") => &["Add User Note"],
        (RouteKind::Users, "browse") => &["Request Directory"],
        (RouteKind::Users, "message") => &["Send Message"],
        (RouteKind::Contacts, "add contact" | "add friend") => &["Add Contact"],
        (RouteKind::Contacts, "message") => &["Send Message"],
        (RouteKind::Contacts, "browse") => &["Request Directory"],
        (RouteKind::Contacts, "remove") => &["Remove Contact"],
        (RouteKind::Solid, "resolve webid" | "connect identity" | "sync storage") => {
            &["Resolve WebID"]
        }
        (RouteKind::Collections, "create collection") => &["Create Collection"],
        (RouteKind::Collections, "add item") => &["Add Library Item"],
        (RouteKind::Collections, "open" | "remove item") => &["Add Library Item"],
        (RouteKind::Collections, "share") => &["Create Share Grant"],
        (RouteKind::ShareGroups, "create group" | "create your first group") => {
            &["Create Share Group"]
        }
        (RouteKind::ShareGroups, "add member") => &["Add Share Group Member"],
        (RouteKind::ShareGroups, "issue token") => &["Issue Share Token"],
        (RouteKind::ShareGroups, "update share grant") => &["Update Share Grant"],
        (RouteKind::ShareGroups, "create share grant") => &["Create Share Grant"],
        (RouteKind::SharedWithMe, "open" | "open collection" | "stream") => {
            &["Backfill Share Grant"]
        }
        (RouteKind::SharedWithMe, "backfill") => &["Backfill Share Grant"],
        (RouteKind::SharedWithMe, "copy token") => &["Issue Share Token"],
        (RouteKind::SharedWithMe, "leave share") => &["Delete Share Grant"],
        (RouteKind::Browse, "browse" | "open a new browse tab" | "new tab") => {
            &["Request Directory"]
        }
        (RouteKind::Browse, "open") => &["Request Directory"],
        (RouteKind::Browse, "download selected" | "download") => &["Queue Download"],
        (RouteKind::System, "connect") => &["Connect"],
        (RouteKind::System, "disconnect") => &["Disconnect"],
        (RouteKind::System, "rescan" | "rescan shares") => &["Rescan Shares"],
        (RouteKind::System, "vacuum" | "vacuum database") => &["Vacuum Database"],
        (RouteKind::System, "check for updates") => &["Check for Updates"],
        (RouteKind::System, "get privileges") => &["Get Privileges"],
        (RouteKind::System, "diagnostic bundle") => &["Diagnostic Bundle"],
        (RouteKind::System, "setup health") => &["Setup Health"],
        (RouteKind::System, "shut down") => &["Shut Down"],
        (RouteKind::System, "restart") => &["Restart"],
        _ => &[],
    };
    aliases
        .iter()
        .chain(std::iter::once(&label))
        .find_map(|candidate| {
            route_actions()
                .iter()
                .copied()
                .find(|action| action.label.eq_ignore_ascii_case(candidate))
        })
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
        ActionBody::DownloadFiles => {
            let files = value
                .lines()
                .flat_map(|line| line.split('|'))
                .map(str::trim)
                .filter(|filename| !filename.is_empty())
                .map(|filename| {
                    format!(
                        r#"{{"filename":"{}","size":99}}"#,
                        escape_json_string(filename)
                    )
                })
                .collect::<Vec<_>>();
            let files = if files.is_empty() {
                vec![r#"{"filename":"Remote/Song.mp3","size":99}"#.to_string()]
            } else {
                files
            };
            Some(format!("[{}]", files.join(",")))
        }
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn json_cell_value(item: &serde_json::Value, column: &str) -> String {
    if column == "value" {
        return compact_preview(&item.to_string());
    }
    item.get(column)
        .map(json_scalar_preview)
        .unwrap_or_default()
}

#[cfg(test)]
fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn record_detail(item: &serde_json::Value) -> String {
    item.get("status")
        .or_else(|| item.get("state"))
        .or_else(|| item.get("kind"))
        .or_else(|| item.get("message"))
        .or_else(|| item.get("path"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| format!("{} fields", json_object_fields(item).len()))
}

#[cfg(test)]
fn record_json(item: &serde_json::Value) -> String {
    serde_json::to_string_pretty(item).unwrap_or_else(|_| item.to_string())
}

#[cfg(test)]
fn record_search_text(item: &serde_json::Value, label: &str, detail: &str) -> String {
    format!("{label} {detail} {}", compact_preview(&item.to_string())).to_lowercase()
}

#[cfg(test)]
fn data_card_inspector_html() -> String {
    r#"<aside class="slskr-card-inspector" aria-live="polite"><h4>Record Inspector</h4><p>Select a list or table row to inspect its details.</p><pre></pre></aside>"#.to_string()
}

#[cfg(test)]
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

pub fn route_kind(path: &str) -> RouteKind {
    match normalize_route_path(path) {
        "/discovery-graph" => RouteKind::DiscoveryGraph,
        "/playlist-intake" => RouteKind::PlaylistIntake,
        "/wishlist" => RouteKind::Wishlist,
        "/downloads" => RouteKind::Downloads,
        "/uploads" => RouteKind::Uploads,
        "/messages" | "/chat" | "/pods" | "/pods/:podId" | "/pods/:podId/channels/:channelId" => {
            RouteKind::Messages
        }
        "/rooms" => RouteKind::Rooms,
        "/users" => RouteKind::Users,
        "/contacts" => RouteKind::Contacts,
        "/solid" => RouteKind::Solid,
        "/collections" => RouteKind::Collections,
        "/sharegroups" => RouteKind::ShareGroups,
        "/shared" => RouteKind::SharedWithMe,
        "/browse" => RouteKind::Browse,
        "/system" | "/system/:tab" => RouteKind::System,
        _ => RouteKind::Search,
    }
}

fn response_count(responses: Option<&[EndpointBody]>, endpoint: &str) -> String {
    responses
        .and_then(|items| endpoint_body(items, endpoint))
        .and_then(json_array_len)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "0".to_string())
}

fn response_value(responses: Option<&[EndpointBody]>, endpoint: &str, field: &str) -> String {
    responses
        .and_then(|items| endpoint_body(items, endpoint))
        .and_then(|body| json_field_string(body, field))
        .unwrap_or_else(|| "pending".to_string())
}

fn status_chip_html(label: &str, value: &str) -> String {
    format!(
        r#"<span class="slskr-status-chip"><strong>{}</strong>{}</span>"#,
        escape_html(label),
        escape_html(value)
    )
}

fn workflow_tabs_html(tabs: &[&str]) -> String {
    tabs.iter()
        .enumerate()
        .map(|(index, tab)| {
            format!(
                r#"<button type="button" class="{class}" aria-selected="{selected}">{tab}</button>"#,
                class = if index == 0 {
                    "slskr-tab is-active"
                } else {
                    "slskr-tab"
                },
                selected = if index == 0 { "true" } else { "false" },
                tab = escape_html(tab),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn empty_state_html(title: &str, detail: &str, action: &str) -> String {
    format!(
        r#"<div class="slskr-empty-workflow"><strong>{title}</strong><span>{detail}</span><button type="button">{action}</button></div>"#,
        title = escape_html(title),
        detail = escape_html(detail),
        action = escape_html(action),
    )
}

fn workflow_table_owned_html(
    headers: &[&str],
    rows: &[(String, String, String, String)],
) -> String {
    let header = headers
        .iter()
        .map(|header| format!(r#"<th>{}</th>"#, escape_html(header)))
        .collect::<Vec<_>>()
        .join("");
    let rows = rows
        .iter()
        .map(|(primary, secondary, meta, action)| {
            format!(
                r#"<tr><td><strong>{primary}</strong><span>{secondary}</span></td><td>{meta}</td><td><button type="button">{action}</button></td></tr>"#,
                primary = escape_html(primary),
                secondary = escape_html(secondary),
                meta = escape_html(meta),
                action = escape_html(action),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="slskr-table-wrap slskr-domain-table"><table><thead><tr>{header}</tr></thead><tbody>{rows}</tbody></table></div>"#,
        header = header,
        rows = rows,
    )
}

fn json_endpoint_value(
    responses: Option<&[EndpointBody]>,
    endpoint: &str,
) -> Option<serde_json::Value> {
    responses
        .and_then(|items| endpoint_body(items, endpoint))
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
}

fn value_array(value: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(items) = value.as_array() {
        return items.clone();
    }
    for key in [
        "entries",
        "items",
        "records",
        "results",
        "responses",
        "messages",
        "conversations",
        "rooms",
        "users",
        "contacts",
        "collections",
        "groups",
        "grants",
        "directories",
        "files",
        "shares",
        "providers",
        "jobs",
    ] {
        if let Some(items) = value.get(key).and_then(|entry| entry.as_array()) {
            return items.clone();
        }
    }
    Vec::new()
}

fn endpoint_array(responses: Option<&[EndpointBody]>, endpoint: &str) -> Vec<serde_json::Value> {
    json_endpoint_value(responses, endpoint)
        .map(|value| value_array(&value))
        .unwrap_or_default()
}

fn value_at_path<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in key.split('.') {
        if let Ok(index) = part.parse::<usize>() {
            current = current.as_array()?.get(index)?;
        } else {
            current = current.get(part)?;
        }
    }
    Some(current)
}

fn value_text(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(current) = value_at_path(value, key) else {
            continue;
        };
        match current {
            serde_json::Value::String(text) if !text.is_empty() => return Some(text.clone()),
            serde_json::Value::Bool(value) => return Some(value.to_string()),
            serde_json::Value::Number(value) => return Some(value.to_string()),
            serde_json::Value::Array(items) => return Some(items.len().to_string()),
            _ => {}
        }
    }
    None
}

fn value_number(value: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        let Some(current) = value_at_path(value, key) else {
            continue;
        };
        if let Some(number) = current.as_f64() {
            return Some(number);
        }
        if let Some(text) = current.as_str().and_then(|text| text.parse::<f64>().ok()) {
            return Some(text);
        }
    }
    None
}

fn value_bool(value: &serde_json::Value, keys: &[&str]) -> Option<bool> {
    for key in keys {
        let Some(current) = value_at_path(value, key) else {
            continue;
        };
        if let Some(value) = current.as_bool() {
            return Some(value);
        }
        if let Some(text) = current.as_str() {
            match text.to_ascii_lowercase().as_str() {
                "true" | "yes" | "online" | "enabled" => return Some(true),
                "false" | "no" | "offline" | "disabled" => return Some(false),
                _ => {}
            }
        }
    }
    None
}

fn nested_items(value: &serde_json::Value, keys: &[&str]) -> Vec<serde_json::Value> {
    for key in keys {
        if let Some(items) = value_at_path(value, key).and_then(|entry| entry.as_array()) {
            return items.clone();
        }
    }
    Vec::new()
}

fn value_count(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(current) = value_at_path(value, key) else {
            continue;
        };
        match current {
            serde_json::Value::Array(items) => return Some(items.len().to_string()),
            serde_json::Value::Number(number) => return Some(number.to_string()),
            serde_json::Value::String(text) if !text.is_empty() => return Some(text.clone()),
            _ => {}
        }
    }
    None
}

fn value_text_with_parent(
    value: &serde_json::Value,
    parent: Option<&serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    value_text(value, keys).or_else(|| parent.and_then(|parent| value_text(parent, keys)))
}

fn value_number_with_parent(
    value: &serde_json::Value,
    parent: Option<&serde_json::Value>,
    keys: &[&str],
) -> Option<f64> {
    value_number(value, keys).or_else(|| parent.and_then(|parent| value_number(parent, keys)))
}

fn format_speed(value: &serde_json::Value, parent: Option<&serde_json::Value>) -> String {
    if let Some(speed) = value_text_with_parent(value, parent, &["speed", "averageSpeed"]) {
        return speed;
    }
    value_number_with_parent(
        value,
        parent,
        &["bytesPerSecond", "transferSpeed", "speedBytesPerSecond"],
    )
    .map(|bytes| format!("{bytes:.0} B/s"))
    .unwrap_or_else(|| "0 B/s".to_string())
}

fn first_nested_text(
    value: &serde_json::Value,
    array_keys: &[&str],
    field_keys: &[&str],
) -> Option<String> {
    nested_items(value, array_keys)
        .first()
        .and_then(|item| value_text(item, field_keys))
}

fn format_transfer_progress(
    value: &serde_json::Value,
    parent: Option<&serde_json::Value>,
) -> String {
    let state = value_text_with_parent(value, parent, &["state", "status"])
        .unwrap_or_else(|| "pending".to_string());
    let progress = value_number_with_parent(
        value,
        parent,
        &[
            "percentComplete",
            "percentage",
            "progress",
            "progressPercent",
        ],
    )
    .map(|value| {
        if value <= 1.0 {
            format!("{:.0}%", value * 100.0)
        } else {
            format!("{value:.0}%")
        }
    })
    .unwrap_or_else(|| "0%".to_string());
    let speed = format_speed(value, parent);
    let eta = value_text_with_parent(
        value,
        parent,
        &["eta", "remaining", "remainingTime", "timeRemaining"],
    )
    .map(|value| format!(" / ETA {value}"))
    .unwrap_or_default();
    format!("{state} / {progress} / {speed}{eta}")
}

fn row_meta_with_id(meta: String, id: Option<&str>) -> String {
    id.filter(|value| safe_route_segment(value.trim()))
        .map(|id| format!("{meta} / id={}", id.trim()))
        .unwrap_or(meta)
}

fn route_dynamic_rows(
    kind: RouteKind,
    responses: Option<&[EndpointBody]>,
) -> Option<Vec<(String, String, String, String)>> {
    let rows = match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph => {
            let responses_rows = endpoint_array(responses, "/searches/:id/responses");
            if !responses_rows.is_empty() {
                responses_rows
                    .iter()
                    .take(50)
                    .map(|item| {
                        let filename =
                            first_nested_text(item, &["files"], &["filename", "path", "name"])
                                .unwrap_or_else(|| "Result group".to_string());
                        let username = value_text(item, &["username", "user", "peer"])
                            .unwrap_or_else(|| "unknown peer".to_string());
                        let queue = value_text(item, &["queueLength", "queue", "placeInQueue"])
                            .unwrap_or_else(|| "0".to_string());
                        let slot = if value_bool(item, &["hasFreeUploadSlot", "freeUploadSlot"])
                            .unwrap_or(false)
                        {
                            "free slot"
                        } else {
                            "queue"
                        };
                        (
                            filename,
                            username,
                            format!("{slot} / queue {queue}"),
                            "Download".to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                endpoint_array(responses, "/searches")
                    .iter()
                    .take(50)
                    .map(|item| {
                        let query = value_text(item, &["searchText", "query", "text"])
                            .unwrap_or_else(|| "Saved search".to_string());
                        let id = value_text(item, &["id"]).unwrap_or_else(|| "pending".to_string());
                        let state = value_text(item, &["state", "status"])
                            .unwrap_or_else(|| "created".to_string());
                        (query, format!("search {id}"), state, "Open".to_string())
                    })
                    .collect::<Vec<_>>()
            }
        }
        RouteKind::PlaylistIntake => endpoint_array(responses, "/source-feed-imports/preview")
            .iter()
            .take(50)
            .map(|item| {
                let title = value_text(item, &["title", "track", "name"])
                    .unwrap_or_else(|| "Playlist row".to_string());
                let artist = value_text(item, &["artist", "albumArtist"])
                    .unwrap_or_else(|| "unknown artist".to_string());
                let status = value_text(item, &["status", "classification"])
                    .unwrap_or_else(|| "review".to_string());
                (title, artist, status, "Import".to_string())
            })
            .collect::<Vec<_>>(),
        RouteKind::Wishlist => endpoint_array(responses, "/wishlist")
            .iter()
            .take(50)
            .map(|item| {
                let text = value_text(item, &["searchText", "query", "text"])
                    .unwrap_or_else(|| "Wanted search".to_string());
                let filter = value_text(item, &["filter", "searchFilter"])
                    .unwrap_or_else(|| "no filter".to_string());
                let enabled = value_bool(item, &["enabled"]).unwrap_or(false);
                let auto =
                    value_bool(item, &["autoDownload", "autoDownloadEnabled"]).unwrap_or(false);
                let id = value_text(item, &["id", "wishlistId", "searchId"]);
                (
                    text,
                    filter,
                    row_meta_with_id(format!("enabled={enabled} / auto={auto}"), id.as_deref()),
                    "Run".to_string(),
                )
            })
            .collect::<Vec<_>>(),
        RouteKind::Downloads | RouteKind::Uploads => {
            let endpoint = if kind == RouteKind::Downloads {
                "/transfers/downloads"
            } else {
                "/transfers/uploads"
            };
            endpoint_array(responses, endpoint)
                .iter()
                .take(50)
                .flat_map(|item| {
                    let username = value_text(item, &["username", "user", "peer"])
                        .unwrap_or_else(|| "unknown peer".to_string());
                    let files = nested_items(
                        item,
                        &[
                            "files",
                            "directories",
                            "items",
                            "transfer.files",
                            "transfer.directories",
                        ],
                    );
                    if files.is_empty() {
                        let id = value_text(item, &["id", "token", "transferId", "fileId"]);
                        vec![(
                            value_text(
                                item,
                                &["filename", "path", "name", "remotePath", "localPath"],
                            )
                            .unwrap_or_else(|| "Transfer".to_string()),
                            username,
                            row_meta_with_id(format_transfer_progress(item, None), id.as_deref()),
                            if kind == RouteKind::Downloads {
                                "Cancel"
                            } else {
                                "Deny"
                            }
                            .to_string(),
                        )]
                    } else {
                        files
                            .iter()
                            .map(|file| {
                                let id = value_text(file, &["id", "token", "transferId", "fileId"])
                                    .or_else(|| {
                                        value_text(item, &["id", "token", "transferId", "fileId"])
                                    });
                                (
                                    value_text(
                                        file,
                                        &["filename", "path", "name", "remotePath", "localPath"],
                                    )
                                    .unwrap_or_else(|| "Transfer".to_string()),
                                    value_text(file, &["username", "user", "peer"])
                                        .unwrap_or_else(|| username.clone()),
                                    row_meta_with_id(
                                        format_transfer_progress(file, Some(item)),
                                        id.as_deref(),
                                    ),
                                    if kind == RouteKind::Downloads {
                                        "Cancel"
                                    } else {
                                        "Deny"
                                    }
                                    .to_string(),
                                )
                            })
                            .collect::<Vec<_>>()
                    }
                })
                .collect::<Vec<_>>()
        }
        RouteKind::Messages | RouteKind::Rooms => endpoint_array(responses, "/conversations")
            .iter()
            .take(50)
            .map(|item| {
                let username = value_text(item, &["username", "user", "roomName", "name"])
                    .unwrap_or_else(|| "conversation".to_string());
                let last = value_text(item, &["lastMessage", "message", "latestMessage"])
                    .unwrap_or_else(|| "No messages".to_string());
                let unread = value_text(
                    item,
                    &["unreadCount", "unacknowledgedCount", "messageCount"],
                )
                .unwrap_or_else(|| "0".to_string());
                (
                    username,
                    last,
                    format!("{unread} unread"),
                    "Reply".to_string(),
                )
            })
            .collect::<Vec<_>>(),
        RouteKind::Users => endpoint_array(responses, "/users")
            .iter()
            .take(50)
            .map(|item| {
                let username =
                    value_text(item, &["username", "name"]).unwrap_or_else(|| "peer".to_string());
                let status =
                    value_text(item, &["status", "state"]).unwrap_or_else(|| "unknown".to_string());
                let file_count = value_count(item, &["sharedFileCount", "files", "stats.files"])
                    .unwrap_or_else(|| "files pending".to_string());
                let speed = value_text(item, &["uploadSpeed", "stats.uploadSpeed", "speed"])
                    .unwrap_or_else(|| "speed pending".to_string());
                let privileges =
                    value_text(item, &["privileges", "privileged", "stats.privileges"])
                        .unwrap_or_else(|| "standard".to_string());
                let stats = format!("{file_count} files / {speed} / {privileges}");
                (username, status, stats, "Browse".to_string())
            })
            .collect::<Vec<_>>(),
        RouteKind::Contacts => endpoint_array(responses, "/contacts")
            .iter()
            .take(50)
            .map(|item| {
                let name = value_text(item, &["nickname", "username", "name", "peerId"])
                    .unwrap_or_else(|| "contact".to_string());
                let peer = value_text(item, &["peerId", "username"])
                    .unwrap_or_else(|| "unknown peer".to_string());
                let verified = value_bool(item, &["verified"])
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "pending".to_string());
                let group =
                    value_text(item, &["group", "groupName", "tags.0"]).unwrap_or_else(|| {
                        value_text(item, &["status", "state"])
                            .unwrap_or_else(|| "ungrouped".to_string())
                    });
                (
                    name,
                    peer,
                    format!("{group} / verified={verified}"),
                    "Message".to_string(),
                )
            })
            .collect::<Vec<_>>(),
        RouteKind::Solid => json_endpoint_value(responses, "/solid/status")
            .map(|value| {
                vec![(
                    value_text(&value, &["webId", "identity"])
                        .unwrap_or_else(|| "Identity".to_string()),
                    value_text(&value, &["storage", "pod", "storageRoot"])
                        .unwrap_or_else(|| "No storage".to_string()),
                    value_text(&value, &["status", "state"])
                        .unwrap_or_else(|| "not connected".to_string()),
                    "Resolve WebID".to_string(),
                )]
            })
            .unwrap_or_default(),
        RouteKind::Collections => endpoint_array(responses, "/collections")
            .iter()
            .take(50)
            .map(|item| {
                let title = value_text(item, &["title", "name"])
                    .unwrap_or_else(|| "Collection".to_string());
                let kind =
                    value_text(item, &["type", "kind"]).unwrap_or_else(|| "Playlist".to_string());
                let count = value_count(item, &["itemCount", "itemsCount", "items", "files"])
                    .unwrap_or_else(|| "0".to_string());
                let id = value_text(item, &["id", "collectionId", "contentId"]);
                (
                    title,
                    kind,
                    row_meta_with_id(format!("{count} items"), id.as_deref()),
                    "Open".to_string(),
                )
            })
            .collect::<Vec<_>>(),
        RouteKind::ShareGroups => endpoint_array(responses, "/sharegroups")
            .iter()
            .take(50)
            .map(|item| {
                let name = value_text(item, &["name", "title"])
                    .unwrap_or_else(|| "Share group".to_string());
                let members = value_count(item, &["memberCount", "members", "grants", "users"])
                    .unwrap_or_else(|| "0".to_string());
                let created = value_text(item, &["createdAt", "created"])
                    .unwrap_or_else(|| "created pending".to_string());
                let id = value_text(item, &["id", "groupId", "shareGroupId"]);
                (
                    name,
                    format!("{members} members"),
                    row_meta_with_id(created, id.as_deref()),
                    "Add Member".to_string(),
                )
            })
            .collect::<Vec<_>>(),
        RouteKind::SharedWithMe => endpoint_array(responses, "/shared")
            .iter()
            .take(50)
            .map(|item| {
                let title = value_text(item, &["collection.title", "title", "name"])
                    .unwrap_or_else(|| "Shared collection".to_string());
                let owner = value_text(
                    item,
                    &[
                        "owner.username",
                        "owner",
                        "sharedBy.username",
                        "sharedBy",
                        "grant.owner",
                        "username",
                    ],
                )
                .unwrap_or_else(|| "unknown owner".to_string());
                let permissions = value_text(
                    item,
                    &["permissions", "access", "grant.permissions", "grant.access"],
                )
                .unwrap_or_else(|| "read".to_string());
                let id = value_text(item, &["id", "grant.id", "grantId", "shareGrantId"]);
                (
                    title,
                    owner,
                    row_meta_with_id(permissions, id.as_deref()),
                    "Open".to_string(),
                )
            })
            .collect::<Vec<_>>(),
        RouteKind::Browse => {
            let root = json_endpoint_value(responses, "/users/:username/browse");
            let mut rows = root.as_ref().map(value_array).unwrap_or_default();
            if let Some(value) = root.as_ref() {
                rows.extend(nested_items(value, &["directories", "root.directories"]));
                for directory in nested_items(value, &["directories", "root.directories"]) {
                    rows.extend(nested_items(&directory, &["files", "children", "items"]));
                }
                rows.extend(nested_items(
                    value,
                    &["files", "root.files", "children", "items"],
                ));
            }
            rows.iter()
                .take(50)
                .map(|item| {
                    let name = value_text(
                        item,
                        &[
                            "name",
                            "filename",
                            "path",
                            "directory",
                            "folder",
                            "remotePath",
                        ],
                    )
                    .unwrap_or_else(|| "Browse entry".to_string());
                    let kind = if value_bool(item, &["isDirectory", "directory"]).unwrap_or(false) {
                        "folder".to_string()
                    } else {
                        value_text(item, &["type", "kind"]).unwrap_or_else(|| "file".to_string())
                    };
                    let size = value_text(item, &["size", "bytes", "fileSize"])
                        .unwrap_or_else(|| "0".to_string());
                    (name, kind, size, "Download".to_string())
                })
                .collect::<Vec<_>>()
        }
        RouteKind::System => {
            let mut rows = Vec::new();
            if let Some(server) = json_endpoint_value(responses, "/server") {
                rows.push((
                    "Connection".to_string(),
                    value_text(&server, &["state", "status"])
                        .unwrap_or_else(|| "pending".to_string()),
                    value_text(&server, &["username", "server", "session.username"])
                        .unwrap_or_else(|| "session".to_string()),
                    "Connect".to_string(),
                ));
            }
            if let Some(shares) = json_endpoint_value(responses, "/shares") {
                rows.push((
                    "Shares".to_string(),
                    value_text(&shares, &["status", "state", "scanStatus"])
                        .unwrap_or_else(|| "ready".to_string()),
                    value_count(&shares, &["roots", "directories", "shares", "count"])
                        .map(|count| format!("{count} roots"))
                        .unwrap_or_else(|| "roots pending".to_string()),
                    "Rescan".to_string(),
                ));
            }
            if let Some(database) = json_endpoint_value(responses, "/database/stats") {
                rows.push((
                    "Database".to_string(),
                    value_text(&database, &["status", "state"])
                        .unwrap_or_else(|| "ready".to_string()),
                    value_text(&database, &["size", "path", "databaseSize"])
                        .unwrap_or_else(|| "stats".to_string()),
                    "Vacuum".to_string(),
                ));
            }
            if let Some(logs) = json_endpoint_value(responses, "/logs") {
                rows.push((
                    "Events".to_string(),
                    value_text(&logs, &["level", "status"]).unwrap_or_else(|| "live".to_string()),
                    value_count(&logs, &["events", "entries", "items"])
                        .map(|count| format!("{count} entries"))
                        .unwrap_or_else(|| "stream pending".to_string()),
                    "Filter".to_string(),
                ));
            }
            rows
        }
    };

    if rows.is_empty() {
        None
    } else {
        Some(rows)
    }
}

fn reference_field_html(label: &str, placeholder: &str) -> String {
    format!(
        r#"<label><span>{label}</span><input type="text" placeholder="{placeholder}" aria-label="{label}"></label>"#,
        label = escape_html(label),
        placeholder = escape_html(placeholder),
    )
}

fn reference_buttons_html(labels: &[&str]) -> String {
    labels
        .iter()
        .map(|label| format!(r#"<button type="button">{}</button>"#, escape_html(label)))
        .collect::<Vec<_>>()
        .join("")
}

fn route_component_parity_attrs(kind: RouteKind) -> &'static str {
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

fn route_component_parity_class(kind: RouteKind) -> &'static str {
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

type RouteReferenceSpec<'a> = (
    &'a str,
    &'a str,
    Vec<(&'a str, &'a str)>,
    Vec<&'a str>,
    Vec<&'a str>,
);

fn route_reference_panel_html(kind: RouteKind) -> String {
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
                    "Request Portal Summary Operator view of wanted music before acquisition jobs are wired.",
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
                vec!["Direct Message", "Join Room", "Create Room", "Open Batch Private-Message Dialog", "Collapse All Message Panels"],
                vec!["Saved Chats 0", "Joined Rooms 0", "Pod Channels 0", "Workspace 0 open"],
            ),
            RouteKind::Rooms => (
                "Messages",
                "Room-focused message workspace.",
                vec![("Search rooms", "Search rooms")],
                vec!["Join Room", "Create Room", "Leave Room"],
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
                vec!["Solid integration is disabled (Feature.Solid=false)."],
            ),
            RouteKind::Collections => (
                "Collections Manage your playlists and share lists",
                "Manage your playlists and share lists.",
                vec![
                    ("Title", "Enter collection title"),
                    ("Description", "Optional description"),
                    ("Search for item", "Search by filename (e.g., sintel, aria, treasure)..."),
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
                vec!["Check for Updates", "Get Privileges", "Diagnostic Bundle", "Setup Health", "Shut Down", "Restart"],
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
        r#"<section class="slskr-reference-panel {component_class}" data-slskr-parity-reference{attrs}><header><div><p class="slskr-kicker">slskd compatibility</p><h2>{title}</h2><p>{detail}</p></div><div class="slskr-reference-actions">{buttons}</div></header><form class="slskr-reference-form">{fields}</form><div class="slskr-reference-facts">{facts}</div></section>"#,
        component_class = escape_html(route_component_parity_class(kind)),
        attrs = route_component_parity_attrs(kind),
        title = escape_html(title),
        detail = escape_html(detail),
        buttons = reference_buttons_html(&buttons),
        fields = field_html,
        facts = facts_html,
    )
}

fn native_row_cards_html(rows: &[(String, String, String, String)], empty: &str) -> String {
    if rows.is_empty() {
        return format!(
            r#"<div class="slskr-native-empty"><strong>{}</strong><span>Use the controls above to load this workspace.</span></div>"#,
            escape_html(empty)
        );
    }
    rows.iter()
        .take(12)
        .map(|(primary, secondary, meta, action)| {
            format!(
                r#"<article class="slskr-native-row"><div><strong>{primary}</strong><span>{secondary}</span></div><span>{meta}</span><button type="button">{action}</button></article>"#,
                primary = escape_html(primary),
                secondary = escape_html(secondary),
                meta = escape_html(meta),
                action = escape_html(action),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn native_table_html(
    kind: RouteKind,
    headers: &[&str],
    rows: &[(String, String, String, String)],
    empty: &str,
) -> String {
    if rows.is_empty() {
        return native_row_cards_html(rows, empty);
    }
    let headers = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            format!(
                r#"<th><button type="button" data-slskr-native-sort="{index}" aria-sort="none">{}</button></th>"#,
                escape_html(header)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let rows = rows
        .iter()
        .take(50)
        .enumerate()
        .map(|(index, (primary, secondary, meta, action))| {
            let actions = native_row_action_buttons_html(kind, action);
            let resource_id = native_row_resource_id(kind, primary, secondary, meta, index);
            let row_attrs = native_row_data_attrs(kind, primary, secondary, meta);
            let action_summary =
                native_row_action_summary(kind, primary, secondary, meta, action, &resource_id);
            let detail_list = native_row_detail_list(kind, primary, secondary, meta, action);
            let action_menu = native_row_action_labels(kind, action).join(" | ");
            let meta_cell = native_meta_cell_html(kind, meta, primary, secondary);
            format!(
                r#"<tr tabindex="0" aria-keyshortcuts="Enter Space ArrowUp ArrowDown Home End" data-slskr-native-select data-slskr-native-index="{index}" data-slskr-native-resource-id="{resource_id}"{row_attrs} data-slskr-native-action-summary="{action_summary}" data-slskr-native-detail-list="{detail_list}" data-slskr-native-action-menu="{action_menu}" data-slskr-native-sort-0="{primary}" data-slskr-native-sort-1="{secondary}" data-slskr-native-sort-2="{meta}" data-slskr-native-sort-3="{action}" data-slskr-native-title="{primary}" data-slskr-native-detail="{secondary}" data-slskr-native-meta="{meta}" data-slskr-native-action="{action}"><td><label><input type="checkbox" aria-label="Select {primary}"><strong>{primary}</strong></label></td><td>{secondary}</td><td>{meta_cell}</td><td><div class="slskr-native-row-actions">{actions}</div></td></tr>"#,
                primary = escape_html(primary),
                secondary = escape_html(secondary),
                meta = escape_html(meta),
                meta_cell = meta_cell,
                action = escape_html(action),
                actions = actions,
                resource_id = escape_html(&resource_id),
                row_attrs = row_attrs,
                action_summary = escape_html(&action_summary),
                detail_list = escape_html(&detail_list),
                action_menu = escape_html(&action_menu),
                index = index,
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="slskr-native-table-wrap"><table class="slskr-native-table"><thead><tr>{headers}</tr></thead><tbody>{rows}</tbody></table></div>"#,
        headers = headers,
        rows = rows,
    )
}

fn native_meta_cell_html(kind: RouteKind, meta: &str, primary: &str, secondary: &str) -> String {
    match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph => {
            let free_slot = meta.contains("free slot");
            let queue = meta
                .split("queue")
                .nth(1)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("pending");
            format!(
                r#"<div class="slskr-native-state-stack" data-slskr-search-result-controls><span>{meta}</span><div class="slskr-native-state-controls"><span>{slot}</span><span>Queue {queue}</span><button type="button">Expand Result</button><button type="button">Fold Duplicates</button></div><div class="slskr-native-ranking-chips"><span>Smart rank</span><span>Exact match</span><span>Duplicate review</span></div></div>"#,
                meta = escape_html(meta),
                slot = if free_slot { "Free slot" } else { "Queued" },
                queue = escape_html(queue),
            )
        }
        RouteKind::Downloads | RouteKind::Uploads => {
            let progress = native_progress_percent(meta);
            let state = meta.split('/').next().unwrap_or(meta).trim();
            let eta = meta
                .split("ETA")
                .nth(1)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("pending");
            format!(
                r#"<div class="slskr-native-state-stack" data-slskr-transfer-state-control><span>{meta}</span><meter min="0" max="100" value="{progress}" aria-label="Transfer progress for {primary}">{progress}%</meter><div class="slskr-native-state-controls"><button type="button">Retry</button><button type="button">Cancel</button><span>{state}</span><span>ETA {eta}</span></div></div>"#,
                meta = escape_html(meta),
                progress = progress,
                primary = escape_html(primary),
                state = escape_html(state),
                eta = escape_html(eta),
            )
        }
        RouteKind::Wishlist => {
            let enabled = meta.contains("enabled=true");
            let auto = meta.contains("auto=true");
            format!(
                r#"<div class="slskr-native-state-stack" data-slskr-wishlist-row-controls><span>{meta}</span><label><input type="checkbox" aria-label="Enabled {primary}" {enabled}> Enabled</label><label><input type="checkbox" aria-label="Auto-download {primary}" {auto}> Auto-download</label></div>"#,
                meta = escape_html(meta),
                primary = escape_html(primary),
                enabled = if enabled { "checked" } else { "" },
                auto = if auto { "checked" } else { "" },
            )
        }
        RouteKind::SharedWithMe => format!(
            r#"<div class="slskr-native-state-stack" data-slskr-inbound-permission-controls><span>{meta}</span><div class="slskr-native-permission-grid"><span>{permissions}</span><span>{owner}</span><button type="button">Copy token</button><button type="button">Leave share</button></div></div>"#,
            meta = escape_html(meta),
            permissions = escape_html(meta),
            owner = escape_html(secondary),
        ),
        RouteKind::ShareGroups => format!(
            r#"<div class="slskr-native-state-stack" data-slskr-share-group-row-controls><span>{meta}</span><div class="slskr-native-permission-grid"><span>Members</span><span>Grants</span><button type="button">Issue Token</button><button type="button">Update Share Grant</button></div></div>"#,
            meta = escape_html(meta),
        ),
        RouteKind::Browse => {
            let action = if secondary == "folder" {
                "Open folder"
            } else {
                "Queue file"
            };
            format!(
                r#"<div class="slskr-native-state-stack" data-slskr-browse-entry-controls><span>{meta}</span><div class="slskr-native-state-controls"><span>{kind}</span><button type="button">{action}</button></div></div>"#,
                meta = escape_html(meta),
                kind = escape_html(secondary),
                action = escape_html(action),
            )
        }
        _ => escape_html(meta),
    }
}

fn native_progress_percent(meta: &str) -> u32 {
    meta.split('/')
        .map(str::trim)
        .find_map(|part| {
            part.strip_suffix('%')
                .and_then(|value| value.trim().parse::<u32>().ok())
        })
        .map(|value| value.min(100))
        .unwrap_or(0)
}

fn native_row_detail_list(
    kind: RouteKind,
    primary: &str,
    secondary: &str,
    meta: &str,
    action: &str,
) -> String {
    let fields: [(&str, &str); 4] = match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph if secondary.starts_with("search ") => [
            ("Search", primary),
            ("Identifier", secondary),
            ("State", meta),
            ("Next action", action),
        ],
        RouteKind::Search | RouteKind::DiscoveryGraph => [
            ("File", primary),
            ("Peer", secondary),
            ("Queue", meta),
            ("Next action", action),
        ],
        RouteKind::PlaylistIntake => [
            ("Track", primary),
            ("Artist", secondary),
            ("Validation", meta),
            ("Next action", action),
        ],
        RouteKind::Wishlist => [
            ("Wanted search", primary),
            ("Filter", secondary),
            ("Automation", meta),
            ("Next action", action),
        ],
        RouteKind::Downloads => [
            ("File", primary),
            ("Peer", secondary),
            ("Download state", meta),
            ("Next action", action),
        ],
        RouteKind::Uploads => [
            ("File", primary),
            ("Peer", secondary),
            ("Upload state", meta),
            ("Next action", action),
        ],
        RouteKind::Messages | RouteKind::Rooms => [
            ("Conversation", primary),
            ("Last message", secondary),
            ("Unread", meta),
            ("Next action", action),
        ],
        RouteKind::Users => [
            ("Username", primary),
            ("Status", secondary),
            ("Stats", meta),
            ("Next action", action),
        ],
        RouteKind::Contacts => [
            ("Contact", primary),
            ("Peer", secondary),
            ("Group", meta),
            ("Next action", action),
        ],
        RouteKind::Solid => [
            ("Identity", primary),
            ("Storage", secondary),
            ("Session", meta),
            ("Next action", action),
        ],
        RouteKind::Collections => [
            ("Collection", primary),
            ("Type", secondary),
            ("Items", meta),
            ("Next action", action),
        ],
        RouteKind::ShareGroups => [
            ("Group", primary),
            ("Members", secondary),
            ("Created", meta),
            ("Next action", action),
        ],
        RouteKind::SharedWithMe => [
            ("Shared item", primary),
            ("Owner", secondary),
            ("Permissions", meta),
            ("Next action", action),
        ],
        RouteKind::Browse => [
            ("Path", primary),
            ("Entry type", secondary),
            ("Size", meta),
            ("Next action", action),
        ],
        RouteKind::System => [
            ("Area", primary),
            ("State", secondary),
            ("Detail", meta),
            ("Next action", action),
        ],
    };
    fields
        .iter()
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(label, value)| format!("{label}: {}", value.trim()))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn native_row_action_summary(
    kind: RouteKind,
    primary: &str,
    secondary: &str,
    meta: &str,
    action: &str,
    resource_id: &str,
) -> String {
    let id = native_row_embedded_id(meta).unwrap_or(resource_id);
    match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph if secondary.starts_with("search ") => {
            format!("Open search {id} for {primary}")
        }
        RouteKind::Search | RouteKind::DiscoveryGraph => {
            format!("Queue download from {secondary}: {primary}")
        }
        RouteKind::PlaylistIntake => format!("Review parsed row {primary} by {secondary}"),
        RouteKind::Wishlist => format!("Run wishlist {id}: {primary}"),
        RouteKind::Downloads => format!("Cancel download {id} from {secondary}: {primary}"),
        RouteKind::Uploads => format!("Review upload {id} for {secondary}: {primary}"),
        RouteKind::Messages | RouteKind::Rooms => format!("Reply to {primary}: {secondary}"),
        RouteKind::Users => format!("Open user {primary} for browse, message, watch, or note"),
        RouteKind::Contacts => format!("Manage contact {primary} mapped to {secondary}"),
        RouteKind::Solid => format!("Inspect Solid identity {primary} with storage {secondary}"),
        RouteKind::Collections => format!("Open collection {id}: {primary}"),
        RouteKind::ShareGroups => format!("Manage share group {id}: {primary}"),
        RouteKind::SharedWithMe => format!("Open inbound grant {id} from {secondary}: {primary}"),
        RouteKind::Browse => {
            if secondary == "folder" {
                format!("Open folder {primary}")
            } else {
                format!("Queue browsed file {primary}")
            }
        }
        RouteKind::System => format!("Run {action} for system area {primary}"),
    }
}

fn native_row_data_attrs(kind: RouteKind, primary: &str, secondary: &str, meta: &str) -> String {
    let mut attrs: Vec<(&str, &str)> = Vec::new();
    if let Some(id) = native_row_embedded_id(meta) {
        attrs.push(("data-slskr-native-item-id", id));
    }
    match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph => {
            if secondary.starts_with("search ") {
                attrs.push((
                    "data-slskr-native-search-id",
                    secondary.trim_start_matches("search "),
                ));
                attrs.push(("data-slskr-native-search-text", primary));
            } else {
                attrs.push(("data-slskr-native-filename", primary));
                attrs.push(("data-slskr-native-peer", secondary));
                attrs.push(("data-slskr-native-queue-state", meta));
            }
        }
        RouteKind::PlaylistIntake => {
            attrs.push(("data-slskr-native-track", primary));
            attrs.push(("data-slskr-native-artist", secondary));
            attrs.push(("data-slskr-native-row-state", meta));
        }
        RouteKind::Wishlist => {
            attrs.push(("data-slskr-native-search-text", primary));
            attrs.push(("data-slskr-native-search-filter", secondary));
            attrs.push(("data-slskr-native-row-state", meta));
            if let Some(id) = native_row_embedded_id(meta) {
                attrs.push(("data-slskr-native-wishlist-id", id));
            }
        }
        RouteKind::Downloads | RouteKind::Uploads => {
            attrs.push(("data-slskr-native-filename", primary));
            attrs.push(("data-slskr-native-peer", secondary));
            attrs.push(("data-slskr-native-transfer-state", meta));
            if let Some(id) = native_row_embedded_id(meta) {
                attrs.push(("data-slskr-native-transfer-id", id));
            }
        }
        RouteKind::Messages | RouteKind::Rooms => {
            attrs.push(("data-slskr-native-conversation", primary));
            attrs.push(("data-slskr-native-peer", primary));
            attrs.push(("data-slskr-native-last-message", secondary));
            attrs.push(("data-slskr-native-row-state", meta));
        }
        RouteKind::Users => {
            attrs.push(("data-slskr-native-username", primary));
            attrs.push(("data-slskr-native-row-state", secondary));
            attrs.push(("data-slskr-native-user-stats", meta));
        }
        RouteKind::Contacts => {
            attrs.push(("data-slskr-native-contact", primary));
            attrs.push(("data-slskr-native-username", secondary));
            attrs.push(("data-slskr-native-row-state", meta));
        }
        RouteKind::Solid => {
            attrs.push(("data-slskr-native-solid-identity", primary));
            attrs.push(("data-slskr-native-solid-storage", secondary));
            attrs.push(("data-slskr-native-row-state", meta));
        }
        RouteKind::Collections => {
            attrs.push(("data-slskr-native-collection", primary));
            attrs.push(("data-slskr-native-collection-kind", secondary));
            attrs.push(("data-slskr-native-row-state", meta));
            if let Some(id) = native_row_embedded_id(meta) {
                attrs.push(("data-slskr-native-collection-id", id));
            }
        }
        RouteKind::ShareGroups => {
            attrs.push(("data-slskr-native-share-group", primary));
            attrs.push(("data-slskr-native-member-count", secondary));
            attrs.push(("data-slskr-native-row-state", meta));
            if let Some(id) = native_row_embedded_id(meta) {
                attrs.push(("data-slskr-native-share-group-id", id));
            }
        }
        RouteKind::SharedWithMe => {
            attrs.push(("data-slskr-native-collection", primary));
            attrs.push(("data-slskr-native-owner", secondary));
            attrs.push(("data-slskr-native-permissions", meta));
            if let Some(id) = native_row_embedded_id(meta) {
                attrs.push(("data-slskr-native-grant-id", id));
            }
        }
        RouteKind::Browse => {
            attrs.push(("data-slskr-native-path", primary));
            attrs.push(("data-slskr-native-entry-kind", secondary));
            attrs.push(("data-slskr-native-size", meta));
            if secondary != "folder" {
                attrs.push(("data-slskr-native-filename", primary));
            }
        }
        RouteKind::System => {
            attrs.push(("data-slskr-native-system-area", primary));
            attrs.push(("data-slskr-native-row-state", secondary));
        }
    }
    attrs
        .into_iter()
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(name, value)| format!(r#" {name}="{}""#, escape_html(value.trim())))
        .collect::<Vec<_>>()
        .join("")
}

fn native_row_embedded_id(meta: &str) -> Option<&str> {
    meta.split("id=")
        .nth(1)
        .and_then(|tail| tail.split_whitespace().next())
        .map(|value| value.trim_matches(|ch: char| matches!(ch, ',' | ';' | ')' | ']')))
        .filter(|value| safe_route_segment(value))
}

fn native_row_resource_id(
    kind: RouteKind,
    primary: &str,
    secondary: &str,
    meta: &str,
    index: usize,
) -> String {
    let candidates: &[&str] = match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph if secondary.starts_with("search ") => {
            &[secondary]
        }
        RouteKind::Downloads | RouteKind::Uploads | RouteKind::Messages | RouteKind::Rooms => {
            &[secondary, primary]
        }
        RouteKind::Users => &[primary],
        RouteKind::Contacts | RouteKind::SharedWithMe => &[secondary, primary],
        RouteKind::Collections | RouteKind::ShareGroups | RouteKind::Wishlist => &[primary],
        RouteKind::Browse => &[primary],
        RouteKind::System => &[primary],
        _ => &[primary, secondary, meta],
    };
    candidates
        .iter()
        .find_map(|candidate| native_resource_candidate(candidate))
        .unwrap_or_else(|| format!("row-{}", index + 1))
}

fn native_resource_candidate(value: &str) -> Option<String> {
    let value = value
        .trim()
        .strip_prefix("search ")
        .unwrap_or_else(|| value.trim())
        .split('/')
        .next()
        .unwrap_or(value)
        .trim();
    if safe_route_segment(value) {
        Some(value.to_string())
    } else {
        None
    }
}

fn native_row_action_labels(kind: RouteKind, primary_action: &str) -> Vec<&str> {
    let labels: Vec<&str> = match kind {
        RouteKind::Search => vec![primary_action, "Preview", "Download"],
        RouteKind::DiscoveryGraph => vec![primary_action, "Queue Nearby", "Build Atlas"],
        RouteKind::PlaylistIntake => vec![primary_action, "Import Playlist", "Queue Plans"],
        RouteKind::Wishlist => vec![primary_action, "Run Enabled", "Copy Review"],
        RouteKind::Downloads => vec![primary_action, "Retry", "Cancel", "Remove"],
        RouteKind::Uploads => vec![primary_action, "Allow selected", "Deny selected"],
        RouteKind::Messages | RouteKind::Rooms => {
            vec![
                primary_action,
                "Reply",
                "Acknowledge",
                "Delete Conversation",
            ]
        }
        RouteKind::Users => vec![primary_action, "Message", "Watch", "Save note"],
        RouteKind::Contacts => vec![primary_action, "Message", "Browse", "Remove"],
        RouteKind::Solid => vec![
            primary_action,
            "Resolve WebID",
            "Connect Identity",
            "Sync Storage",
        ],
        RouteKind::Collections => vec![primary_action, "Add Item", "Share", "Remove item"],
        RouteKind::ShareGroups => vec![
            "Add Member",
            "Issue Token",
            "Create Share Grant",
            "Update Share Grant",
            primary_action,
        ],
        RouteKind::SharedWithMe => vec![
            primary_action,
            "Stream",
            "Backfill",
            "Copy token",
            "Leave share",
        ],
        RouteKind::Browse => vec![primary_action, "Download Selected", "Open a New Browse Tab"],
        RouteKind::System => vec![
            primary_action,
            "Rescan shares",
            "Vacuum database",
            "Diagnostic Bundle",
        ],
    };
    let mut seen = Vec::new();
    labels
        .into_iter()
        .filter(|label| !label.trim().is_empty())
        .filter(|label| {
            let normalized = label.trim().to_ascii_lowercase();
            if seen.contains(&normalized) {
                false
            } else {
                seen.push(normalized);
                true
            }
        })
        .collect()
}

fn native_row_action_buttons_html(kind: RouteKind, primary_action: &str) -> String {
    native_row_action_labels(kind, primary_action)
        .into_iter()
        .map(|label| {
            format!(
                r#"<button type="button" data-slskr-native-row-action="{}">{}</button>"#,
                escape_html(label),
                escape_html(label)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn native_route_table_html(kind: RouteKind, rows: &[(String, String, String, String)]) -> String {
    match kind {
        RouteKind::Search | RouteKind::DiscoveryGraph => native_table_html(
            kind,
            &["File or query", "Peer or id", "Queue / score", "Action"],
            rows,
            "No search results to display",
        ),
        RouteKind::PlaylistIntake => native_table_html(
            kind,
            &["Parsed row", "Artist", "State", "Action"],
            rows,
            "No playlist rows to review",
        ),
        RouteKind::Wishlist => native_table_html(
            kind,
            &["Search Text", "Filter", "State", "Action"],
            rows,
            "No wishlist searches yet",
        ),
        RouteKind::Downloads | RouteKind::Uploads => native_table_html(
            kind,
            &["Filename", "Peer", "Progress", "Action"],
            rows,
            "No transfers to display",
        ),
        RouteKind::Messages | RouteKind::Rooms => native_table_html(
            kind,
            &["Thread", "Last message", "Unread", "Action"],
            rows,
            "No conversations to display",
        ),
        RouteKind::Users => native_table_html(
            kind,
            &["Username", "Status", "Stats", "Action"],
            rows,
            "No users to display",
        ),
        RouteKind::Contacts => native_table_html(
            kind,
            &["Contact", "Peer", "Verification", "Action"],
            rows,
            "No contacts to display",
        ),
        RouteKind::Solid => native_table_html(
            kind,
            &["Identity", "Storage", "Status", "Action"],
            rows,
            "No Solid status to display",
        ),
        RouteKind::Collections => native_table_html(
            kind,
            &["Title", "Type", "Items", "Action"],
            rows,
            "No collections yet",
        ),
        RouteKind::ShareGroups => native_table_html(
            kind,
            &["Name", "Members", "Created", "Action"],
            rows,
            "No share groups yet",
        ),
        RouteKind::SharedWithMe => native_table_html(
            kind,
            &["Collection", "Shared By", "Permissions", "Action"],
            rows,
            "No shares yet",
        ),
        RouteKind::Browse => native_table_html(
            kind,
            &["Path", "Type", "Size", "Action"],
            rows,
            "No browse entries to display",
        ),
        RouteKind::System => native_table_html(
            kind,
            &["Area", "State", "Detail", "Action"],
            rows,
            "No system status to display",
        ),
    }
}

fn native_stat_html(label: &str, value: &str) -> String {
    format!(
        r#"<span class="slskr-native-stat"><strong>{}</strong><em>{}</em></span>"#,
        escape_html(value),
        escape_html(label)
    )
}

fn native_tab_labels(kind: RouteKind) -> &'static [&'static str] {
    match kind {
        RouteKind::Search => &[
            "Results",
            "Searches",
            "Planner",
            "Filters",
            "Download Preview",
        ],
        RouteKind::DiscoveryGraph => &["Graph", "Recommendations", "Review Queue", "Profiles"],
        RouteKind::PlaylistIntake => &["Parser", "Rows", "Classification", "Plans"],
        RouteKind::Wishlist => &["Wanted", "Review", "History", "Discovery Inbox"],
        RouteKind::Downloads => &["Active", "Queued", "Completed", "Failed"],
        RouteKind::Uploads => &["Active", "Queued", "Completed", "Policy"],
        RouteKind::Messages | RouteKind::Rooms => {
            &["Conversations", "Thread", "Rooms", "Pods", "Search"]
        }
        RouteKind::Users => &["Directory", "Detail", "Watched", "Notes"],
        RouteKind::Contacts => &["Contacts", "Groups", "Nearby", "Invites", "Notes"],
        RouteKind::Solid => &["Identity", "Storage", "Session", "Sync", "Related"],
        RouteKind::Collections => &["Collections", "Items", "Picker", "Sharing"],
        RouteKind::ShareGroups => &["Groups", "Members", "Grants", "Tokens", "Permissions"],
        RouteKind::SharedWithMe => &["Inbound", "Collections", "Tokens", "Owners", "Access"],
        RouteKind::Browse => &["Tabs", "Tree", "Files", "Selected", "Queue"],
        RouteKind::System => &[
            "Info",
            "Network",
            "Mesh",
            "Bridge",
            "MediaCore",
            "Security Policies",
            "Experience",
            "Integrations",
            "Options",
            "Shares",
            "Jobs",
            "Automations",
            "Source Providers",
            "Swarm Analytics",
            "Library Health",
            "Quarantine Jury",
            "Files",
            "Data",
            "Events",
            "Logs",
            "Metrics",
        ],
    }
}

fn native_tab_detail(kind: RouteKind, label: &str) -> &'static str {
    match (kind, label) {
        (RouteKind::Search, "Results") => {
            "Grouped file results with peer, queue, score, warning, and download actions."
        }
        (RouteKind::Search, "Searches") => {
            "Active and historical searches with stop, clear, and reopen controls."
        }
        (RouteKind::Search, "Planner") => {
            "Review selected results before acquisition plans or downloads are created."
        }
        (RouteKind::Search, "Filters") => "Format, bitrate, size, queue, and duplicate filters.",
        (RouteKind::Search, "Download Preview") => {
            "Selected files, peers, destination, and queued download summary."
        }
        (RouteKind::DiscoveryGraph, "Graph") => {
            "Artist, album, track, query, and provider nodes with weighted links."
        }
        (RouteKind::DiscoveryGraph, "Recommendations") => {
            "Next searches suggested from the selected graph neighborhood."
        }
        (RouteKind::DiscoveryGraph, "Review Queue") => {
            "Candidate searches staged for acquisition review."
        }
        (RouteKind::DiscoveryGraph, "Profiles") => {
            "Acquisition profile selector for graph-generated searches."
        }
        (RouteKind::PlaylistIntake, "Parser") => {
            "Paste or upload playlist text before provider or network work starts."
        }
        (RouteKind::PlaylistIntake, "Rows") => {
            "Parsed rows with artist, title, source, and row-level validation."
        }
        (RouteKind::PlaylistIntake, "Classification") => {
            "Track, album, ambiguous, and error buckets for review."
        }
        (RouteKind::PlaylistIntake, "Plans") => {
            "Queue searches or acquisition plans after validation."
        }
        (RouteKind::Wishlist, "Wanted") => {
            "Saved wanted searches with enabled state, filters, and result limits."
        }
        (RouteKind::Wishlist, "Review") => {
            "Result review state before automatic or manual download decisions."
        }
        (RouteKind::Wishlist, "History") => "Last run, result counts, failures, and audit trail.",
        (RouteKind::Wishlist, "Discovery Inbox") => {
            "Bridge selected wanted searches into acquisition request review."
        }
        (RouteKind::Downloads, "Active") => "Running downloads with progress, speed, and ETA.",
        (RouteKind::Downloads, "Queued") => "Pending downloads ordered by peer and slot state.",
        (RouteKind::Downloads, "Completed") => "Finished downloads ready to clear or inspect.",
        (RouteKind::Downloads, "Failed") => "Failed downloads with retry and remove actions.",
        (RouteKind::Uploads, "Active") => "Running uploads with requester, speed, and progress.",
        (RouteKind::Uploads, "Queued") => "Peer requests waiting for an upload slot.",
        (RouteKind::Uploads, "Completed") => "Finished uploads and clear-completed controls.",
        (RouteKind::Uploads, "Policy") => "Allow, deny, queue, and sharing policy controls.",
        (RouteKind::Messages | RouteKind::Rooms, "Conversations") => {
            "Direct message list with unread and acknowledge state."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Thread") => {
            "Selected direct message, room, or pod channel conversation."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Rooms") => {
            "Joined and available rooms with join, leave, and compose actions."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Pods") => {
            "Pod channels stay secondary inside Messages."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Search") => {
            "Search conversations and room names without leaving the messenger."
        }
        (RouteKind::Users, "Directory") => "Watched and searched users with online state.",
        (RouteKind::Users, "Detail") => {
            "Readable user status, privileges, stats, and endpoint info."
        }
        (RouteKind::Users, "Watched") => "Watch list controls for peers you monitor.",
        (RouteKind::Users, "Notes") => "Private notes tied to selected users.",
        (RouteKind::Contacts, "Contacts") => {
            "Saved contacts with message, browse, and remove actions."
        }
        (RouteKind::Contacts, "Groups") => "Contact grouping for trusted or nearby peers.",
        (RouteKind::Contacts, "Nearby") => "Nearby contacts and invite candidates.",
        (RouteKind::Contacts, "Invites") => "Invite, accept, and link handling.",
        (RouteKind::Contacts, "Notes") => "Contact notes and verification context.",
        (RouteKind::Solid, "Identity") => "WebID identity resolution and connection state.",
        (RouteKind::Solid, "Storage") => "Solid storage root and linked-data persistence.",
        (RouteKind::Solid, "Session") => "Authentication and session state.",
        (RouteKind::Solid, "Sync") => "Linked-data sync status and retry controls.",
        (RouteKind::Solid, "Related") => "Bridge, pod, and source-provider context.",
        (RouteKind::Collections, "Collections") => {
            "Collection list with create and select actions."
        }
        (RouteKind::Collections, "Items") => "Selected collection item table with remove controls.",
        (RouteKind::Collections, "Picker") => "Library item browser used as an add-item picker.",
        (RouteKind::Collections, "Sharing") => "Collection share controls and current grants.",
        (RouteKind::ShareGroups, "Groups") => "Share group list with selected group detail.",
        (RouteKind::ShareGroups, "Members") => "Add, remove, and inspect group members.",
        (RouteKind::ShareGroups, "Grants") => "Collection grants issued to the selected group.",
        (RouteKind::ShareGroups, "Tokens") => "Issue, copy, and revoke access tokens.",
        (RouteKind::ShareGroups, "Permissions") => {
            "Read, download, stream, and expiration settings."
        }
        (RouteKind::SharedWithMe, "Inbound") => "Inbound grants and tokens shared by other users.",
        (RouteKind::SharedWithMe, "Collections") => {
            "Shared collections and files available to open."
        }
        (RouteKind::SharedWithMe, "Tokens") => "Token status, copy actions, and expiration.",
        (RouteKind::SharedWithMe, "Owners") => "Owner identity, trust, and contact actions.",
        (RouteKind::SharedWithMe, "Access") => "Open, leave, revoke, or backfill where allowed.",
        (RouteKind::Browse, "Tabs") => {
            "Multiple peer browse sessions, matching the old tabbed browser."
        }
        (RouteKind::Browse, "Tree") => "Directory tree with breadcrumbs and folder expansion.",
        (RouteKind::Browse, "Files") => "File list with size, type, filter, and selection state.",
        (RouteKind::Browse, "Selected") => "Multi-select download preview before queueing.",
        (RouteKind::Browse, "Queue") => "Download queue action for selected browse files.",
        (RouteKind::System, "Info") => "Server, version, session, and operator overview.",
        (RouteKind::System, "Network") => "Connection, ports, privileges, and server state.",
        (RouteKind::System, "Mesh") => "Mesh and federation diagnostics.",
        (RouteKind::System, "Bridge") => "External bridge and integration status.",
        (RouteKind::System, "MediaCore") => {
            "MediaCore routing, validation, storage, and content tools."
        }
        (RouteKind::System, "Security Policies") => "Security policy status and decisions.",
        (RouteKind::System, "Experience") => "User experience preferences.",
        (RouteKind::System, "Integrations") => {
            "Lidarr, FTP, media server, and provider integrations."
        }
        (RouteKind::System, "Options") => "Daemon options and preferences.",
        (RouteKind::System, "Shares") => "Share roots, scan status, and rescan controls.",
        (RouteKind::System, "Jobs") => "Jobs, queues, and execution history.",
        (RouteKind::System, "Automations") => "Automation recipes and bounded execution.",
        (RouteKind::System, "Source Providers") => {
            "Search, metadata, and verification source providers."
        }
        (RouteKind::System, "Swarm Analytics") => "Swarm and peer analytics.",
        (RouteKind::System, "Library Health") => "Library health issues and replacement searches.",
        (RouteKind::System, "Quarantine Jury") => "Quarantine review and decision workflow.",
        (RouteKind::System, "Files") => "File index, fingerprints, and library records.",
        (RouteKind::System, "Data") => "Database and storage maintenance.",
        (RouteKind::System, "Events") => "Filterable event stream.",
        (RouteKind::System, "Logs") => "Operator logs with filters.",
        (RouteKind::System, "Metrics") => "Raw metrics summarized for operators.",
        _ => "Route-specific workflow section.",
    }
}

fn native_tabs_html(kind: RouteKind) -> String {
    let labels = native_tab_labels(kind);
    let buttons = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let selected = if index == 0 { "true" } else { "false" };
            let class = if index == 0 { " is-active" } else { "" };
            format!(
                r#"<button type="button" role="tab" class="slskr-native-tab{class}" aria-selected="{selected}" data-slskr-native-tab="{index}">{}</button>"#,
                escape_html(label)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let panels = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let hidden = if index == 0 { "" } else { " hidden" };
            format!(
                r#"<section class="slskr-native-subpanel" data-slskr-native-panel="{index}"{hidden}>{}</section>"#,
                native_tab_panel_html(kind, label),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="slskr-native-subviews"><div class="slskr-native-subnav" role="tablist">{buttons}</div><div class="slskr-native-subpanels">{panels}</div></div>"#
    )
}

fn native_tab_panel_html(kind: RouteKind, label: &str) -> String {
    if kind == RouteKind::System {
        return native_system_tab_panel_html(label);
    }
    let detail = native_tab_detail(kind, label);
    let controls = native_tab_controls(kind, label)
        .iter()
        .map(|control| format!(r#"<button type="button">{}</button>"#, escape_html(control)))
        .collect::<Vec<_>>()
        .join("");
    let fields = native_tab_fields(kind, label)
        .iter()
        .map(|(field, placeholder)| {
            format!(
                r#"<label><span>{}</span><input type="text" aria-label="{}" placeholder="{}"></label>"#,
                escape_html(field),
                escape_html(field),
                escape_html(placeholder)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let facts = native_tab_facts(kind, label)
        .iter()
        .map(|fact| format!(r#"<span>{}</span>"#, escape_html(fact)))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<header><div><h4>{}</h4><p>{}</p></div><div class="slskr-native-panel-actions">{}</div></header><div class="slskr-native-panel-fields">{}</div><div class="slskr-native-panel-facts">{}</div>"#,
        escape_html(label),
        escape_html(detail),
        controls,
        fields,
        facts,
    )
}

fn native_system_tab_panel_html(label: &str) -> String {
    let detail = native_tab_detail(RouteKind::System, label);
    let controls = native_tab_controls(RouteKind::System, label)
        .iter()
        .map(|control| format!(r#"<button type="button">{}</button>"#, escape_html(control)))
        .collect::<Vec<_>>()
        .join("");
    let fields = native_tab_fields(RouteKind::System, label)
        .iter()
        .map(|(field, placeholder)| {
            format!(
                r#"<label><span>{}</span><input type="text" aria-label="{}" placeholder="{}"></label>"#,
                escape_html(field),
                escape_html(field),
                escape_html(placeholder)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let rows = native_system_tab_rows(label)
        .iter()
        .map(|(area, state, detail, action)| {
            format!(
                r#"<tr tabindex="0"><td><strong>{}</strong></td><td>{}</td><td>{}</td><td><button type="button">{}</button></td></tr>"#,
                escape_html(area),
                escape_html(state),
                escape_html(detail),
                escape_html(action)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let facts = native_tab_facts(RouteKind::System, label)
        .iter()
        .map(|fact| format!(r#"<span>{}</span>"#, escape_html(fact)))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<header><div><h4>{label}</h4><p>{detail}</p></div><div class="slskr-native-panel-actions">{controls}</div></header><div class="slskr-native-panel-fields">{fields}</div><div class="slskr-native-panel-facts">{facts}</div><div class="slskr-native-table-wrap"><table class="slskr-native-table slskr-system-panel-table"><thead><tr><th>Area</th><th>State</th><th>Detail</th><th>Action</th></tr></thead><tbody>{rows}</tbody></table></div>"#,
        label = escape_html(label),
        detail = escape_html(detail),
        controls = controls,
        fields = fields,
        facts = facts,
        rows = rows,
    )
}

fn native_system_tab_rows(
    label: &str,
) -> &'static [(&'static str, &'static str, &'static str, &'static str)] {
    match label {
        "Info" => &[
            (
                "Daemon",
                "running",
                "version and build channel",
                "Check for Updates",
            ),
            (
                "Session",
                "pending",
                "account, privileges, and uptime",
                "Get Privileges",
            ),
            (
                "Diagnostics",
                "ready",
                "support bundle and health summary",
                "Diagnostic Bundle",
            ),
        ],
        "Network" => &[
            (
                "Soulseek server",
                "disconnected",
                "connect, disconnect, reconnect",
                "Connect",
            ),
            (
                "Listening port",
                "unknown",
                "port mapping and reachability",
                "Refresh",
            ),
            (
                "Privileges",
                "unknown",
                "privilege expiry and purchase status",
                "Get Privileges",
            ),
        ],
        "Mesh" => &[
            (
                "Federation",
                "observing",
                "mesh peers and evidence policy",
                "Refresh",
            ),
            (
                "Conflict queue",
                "empty",
                "remote claims and merge decisions",
                "Review Selection",
            ),
            (
                "Relay health",
                "pending",
                "bridge latency and retry window",
                "Diagnostic Bundle",
            ),
        ],
        "Bridge" => &[
            ("Gateway", "idle", "external bridge listener", "Refresh"),
            (
                "Relay",
                "not linked",
                "remote relay credentials",
                "Setup Health",
            ),
            (
                "Sync cursor",
                "pending",
                "last successful bridge sync",
                "Run Selected",
            ),
        ],
        "MediaCore" => &[
            (
                "Routing",
                "ready",
                "stream routes and player handoff",
                "Refresh",
            ),
            (
                "Validation",
                "enabled",
                "file metadata and format checks",
                "Run Selected",
            ),
            (
                "Content tools",
                "available",
                "transcode, fingerprint, repair",
                "Diagnostic Bundle",
            ),
        ],
        "Security Policies" => &[
            (
                "Admin policy",
                "enforced",
                "dangerous actions require confirmation",
                "Review Selection",
            ),
            (
                "Quarantine",
                "enabled",
                "approval, rejection, and evidence retention",
                "Approve",
            ),
            (
                "Outbound webhooks",
                "guarded",
                "allowlist and secret handling",
                "Setup Health",
            ),
        ],
        "Experience" => &[
            (
                "Theme",
                "saved locally",
                "density, contrast, and shell preferences",
                "Save",
            ),
            (
                "Player",
                "compact",
                "bottom player reserve and radio seed mode",
                "Reset",
            ),
            (
                "Notifications",
                "quiet",
                "toast and event notification defaults",
                "Save",
            ),
        ],
        "Integrations" => &[
            (
                "Lidarr",
                "not configured",
                "metadata and import automation",
                "Setup Health",
            ),
            (
                "FTP",
                "not configured",
                "remote drop and library import",
                "Setup Health",
            ),
            (
                "ListenBrainz",
                "not linked",
                "scrobble and feedback sync",
                "Setup Health",
            ),
        ],
        "Options" => &[
            ("Config", "loaded", "daemon options and overrides", "Save"),
            (
                "Debug",
                "off",
                "developer diagnostics stay collapsed",
                "Reset",
            ),
            (
                "Automation bounds",
                "default",
                "limits for unattended jobs",
                "Save",
            ),
        ],
        "Shares" => &[
            (
                "Roots",
                "pending",
                "shared folders and exclusions",
                "Rescan Shares",
            ),
            (
                "Scan progress",
                "idle",
                "last scan, changed files, failures",
                "Refresh",
            ),
            (
                "Contents",
                "indexed",
                "share counts and locked files",
                "Review Selection",
            ),
        ],
        "Jobs" => &[
            (
                "Queued",
                "0",
                "scheduled work waiting to run",
                "Run Selected",
            ),
            ("Running", "0", "active backend jobs", "Cancel Selected"),
            (
                "History",
                "pending",
                "recent completions and failures",
                "Refresh",
            ),
        ],
        "Automations" => &[
            (
                "Recipes",
                "available",
                "library health, replacements, cleanup",
                "Run Selected",
            ),
            (
                "Approvals",
                "required",
                "bounded unattended actions",
                "Review Selection",
            ),
            (
                "Dry run",
                "enabled",
                "preview before mutation",
                "Copy Action Plan",
            ),
        ],
        "Source Providers" => &[
            (
                "Search",
                "ready",
                "provider search and fallback order",
                "Refresh",
            ),
            (
                "Metadata",
                "pending",
                "MusicBrainz and provider enrichment",
                "Run Selected",
            ),
            (
                "Verification",
                "manual",
                "confidence and match warnings",
                "Review Selection",
            ),
        ],
        "Swarm Analytics" => &[
            (
                "Peers",
                "sampling",
                "availability and quality signals",
                "Refresh",
            ),
            (
                "Availability",
                "unknown",
                "result recurrence and queue health",
                "Run Selected",
            ),
            (
                "Quality",
                "pending",
                "bitrate, format, and duplicate signals",
                "Copy Action Plan",
            ),
        ],
        "Library Health" => &[
            (
                "Issues",
                "review",
                "missing, corrupt, duplicate, and low-quality files",
                "Run Replacement Searches",
            ),
            (
                "Replacements",
                "staged",
                "candidate searches for bad files",
                "Copy Action Plan",
            ),
            (
                "Reports",
                "ready",
                "operator review packets",
                "Diagnostic Bundle",
            ),
        ],
        "Quarantine Jury" => &[
            ("Pending", "0", "files awaiting decision", "Approve"),
            ("Rejected", "0", "blocked items and reasons", "Reject"),
            (
                "Evidence",
                "retained",
                "packet copied for review",
                "Copy Packet",
            ),
        ],
        "Files" => &[
            ("Index", "ready", "library records and paths", "Refresh"),
            (
                "Fingerprints",
                "pending",
                "hashes and acoustic IDs",
                "Run Selected",
            ),
            (
                "Records",
                "filterable",
                "open file detail and repair context",
                "Review Selection",
            ),
        ],
        "Data" => &[
            (
                "Database",
                "ready",
                "stats, size, and vacuum state",
                "Vacuum Database",
            ),
            (
                "Storage",
                "bounded",
                "cache and archive cleanup",
                "Run Selected",
            ),
            (
                "Backups",
                "manual",
                "operator export and restore checks",
                "Diagnostic Bundle",
            ),
        ],
        "Events" => &[
            ("Stream", "live", "filterable operator events", "Refresh"),
            (
                "Acknowledgements",
                "pending",
                "review and clear handled events",
                "Clear Filter",
            ),
            (
                "Severity",
                "warn+",
                "level and source filters",
                "Review Selection",
            ),
        ],
        "Logs" => &[
            ("Level", "info", "filter by level and source", "Refresh"),
            (
                "Search",
                "ready",
                "text filter across recent logs",
                "Clear Filter",
            ),
            (
                "Export",
                "available",
                "include logs in diagnostic bundle",
                "Diagnostic Bundle",
            ),
        ],
        "Metrics" => &[
            (
                "Transfers",
                "summarized",
                "speed, slots, and failure rate",
                "Refresh",
            ),
            (
                "API",
                "summarized",
                "requests, errors, and latency",
                "Diagnostic Bundle",
            ),
            (
                "Raw metrics",
                "developer-only",
                "hidden outside Developer drawer",
                "Review Selection",
            ),
        ],
        _ => &[("Status", "pending", "operator workflow", "Review Selection")],
    }
}

fn native_tab_controls(kind: RouteKind, label: &str) -> &'static [&'static str] {
    match (kind, label) {
        (RouteKind::Messages | RouteKind::Rooms, "Thread") => {
            &["Reply", "Acknowledge", "Delete Conversation"]
        }
        (RouteKind::Messages | RouteKind::Rooms, "Rooms") => &["Join Room", "Leave Room"],
        (RouteKind::Browse, "Tabs") => &["Open a New Browse Tab", "New Tab"],
        (RouteKind::Browse, "Tree") => &["Browse", "Refresh Folder"],
        (RouteKind::Browse, "Selected" | "Queue") => &["Download Selected"],
        (RouteKind::Collections, "Collections") => &["Create Collection"],
        (RouteKind::Collections, "Items" | "Picker") => &["Add Item"],
        (RouteKind::Collections, "Sharing") => &["Share"],
        (RouteKind::ShareGroups, "Groups") => &["Create Group"],
        (RouteKind::ShareGroups, "Members") => &["Add Member"],
        (RouteKind::ShareGroups, "Tokens") => &["Issue Token"],
        (RouteKind::ShareGroups, "Grants" | "Permissions") => {
            &["Create Share Grant", "Update Share Grant"]
        }
        (RouteKind::SharedWithMe, "Access") => &["Open", "Stream", "Backfill", "Copy token"],
        (RouteKind::System, "Network") => &["Connect", "Disconnect", "Get Privileges"],
        (RouteKind::System, "Shares") => &["Rescan Shares"],
        (RouteKind::System, "Data") => &["Vacuum Database"],
        (RouteKind::System, "Info") => &["Check for Updates", "Diagnostic Bundle"],
        (RouteKind::System, "Logs" | "Events") => &["Refresh", "Clear Filter"],
        (RouteKind::System, "Options" | "Experience") => &["Save", "Reset"],
        (RouteKind::System, "Jobs" | "Automations") => &["Run Selected", "Cancel Selected"],
        (RouteKind::System, "Library Health") => &["Run Replacement Searches", "Copy Action Plan"],
        (RouteKind::System, "Quarantine Jury") => &["Approve", "Reject", "Copy Packet"],
        (RouteKind::Search, "Results" | "Download Preview") => &["Download", "Queue Selected"],
        (RouteKind::Search, "Searches") => &["Search", "Stop", "Clear"],
        (RouteKind::DiscoveryGraph, "Graph" | "Recommendations") => {
            &["Build Atlas", "Queue Nearby"]
        }
        (RouteKind::PlaylistIntake, "Parser" | "Rows") => &["Import Playlist", "Queue Plans"],
        (RouteKind::Wishlist, "Wanted" | "Discovery Inbox") => &["Add Search", "Run Enabled"],
        (RouteKind::Downloads, "Active" | "Failed") => &["Retry All", "Cancel All"],
        (RouteKind::Downloads, "Completed") => &["Clear Completed"],
        (RouteKind::Uploads, "Active" | "Queued" | "Policy") => {
            &["Allow selected", "Deny selected"]
        }
        (RouteKind::Uploads, "Completed") => &["Clear Completed"],
        (RouteKind::Users, "Directory" | "Detail") => &["Watch", "Browse", "Message"],
        (RouteKind::Users, "Notes") => &["Save note"],
        (RouteKind::Contacts, "Contacts" | "Nearby") => &["Add Friend", "Message", "Browse"],
        (RouteKind::Contacts, "Invites") => &["Create Invite", "Add Friend"],
        (RouteKind::Solid, "Identity" | "Session") => &["Resolve WebID", "Connect Identity"],
        (RouteKind::Solid, "Storage" | "Sync") => &["Sync Storage"],
        _ => &["Review Selection"],
    }
}

fn native_tab_fields(kind: RouteKind, label: &str) -> &'static [(&'static str, &'static str)] {
    match (kind, label) {
        (RouteKind::Messages | RouteKind::Rooms, "Thread") => {
            &[("Chat username", "peer1"), ("Message", "Message")]
        }
        (RouteKind::Messages | RouteKind::Rooms, "Rooms") => &[("Search rooms", "public-domain")],
        (RouteKind::Browse, "Tabs") => &[("Username", "peer1")],
        (RouteKind::Browse, "Tree" | "Files") => &[("Username", "peer1"), ("Folder", "/Music")],
        (RouteKind::Collections, "Collections") => &[
            ("Title", "Collection title"),
            ("Description", "Optional description"),
        ],
        (RouteKind::Collections, "Picker" | "Items") => &[("Search for item", "filename")],
        (RouteKind::ShareGroups, "Groups") => &[("Group Name", "Trusted peers")],
        (RouteKind::ShareGroups, "Members") => &[("Soulseek Username", "peer1")],
        (RouteKind::ShareGroups, "Permissions") => &[("Permissions", "read,download,stream")],
        (RouteKind::Solid, "Identity") => &[("WebID", "https://example.com/profile/card#me")],
        (RouteKind::Search, "Searches" | "Results") => &[("Search text", "public domain jazz")],
        (RouteKind::DiscoveryGraph, "Graph") => &[("Artist Name", "Archive Artist")],
        (RouteKind::PlaylistIntake, "Parser") => &[("Playlist rows", "Artist - Title")],
        (RouteKind::Wishlist, "Wanted") => &[("Search Text", "wanted search")],
        (RouteKind::Users, "Directory" | "Detail") => &[("Username", "peer1")],
        (RouteKind::Contacts, "Contacts" | "Invites") => &[("Nickname", "Friend's name")],
        (RouteKind::System, "Logs" | "Events") => &[("Filter", "level:warn")],
        (RouteKind::System, "Options") => &[("Option key", "shares.scanInterval")],
        _ => &[],
    }
}

fn native_tab_facts(kind: RouteKind, label: &str) -> &'static [&'static str] {
    match (kind, label) {
        (RouteKind::System, "Info") => &["Version", "Session", "Privileges", "Uptime"],
        (RouteKind::System, "Network") => &["Server state", "Ports", "Rate limits", "Proxy trust"],
        (RouteKind::System, "Mesh") => &["Federation health", "Evidence policy", "Conflicts"],
        (RouteKind::System, "Bridge") => &["Bridge status", "Gateway", "Relay"],
        (RouteKind::System, "MediaCore") => &["Routing", "Validation", "Storage", "Content tools"],
        (RouteKind::System, "Security Policies") => {
            &["Admin policy", "Quarantine", "Outbound webhooks"]
        }
        (RouteKind::System, "Experience") => &["Theme", "Density", "Player", "Notifications"],
        (RouteKind::System, "Integrations") => &["Lidarr", "FTP", "Media server", "ListenBrainz"],
        (RouteKind::System, "Options") => &["Config", "Debug", "Overrides"],
        (RouteKind::System, "Shares") => &["Roots", "Exclusions", "Scan progress", "Contents"],
        (RouteKind::System, "Jobs") => &["Queued", "Running", "Failed", "History"],
        (RouteKind::System, "Automations") => &["Recipes", "Bounds", "Approvals"],
        (RouteKind::System, "Source Providers") => &["Search", "Metadata", "Verification"],
        (RouteKind::System, "Swarm Analytics") => &["Peers", "Availability", "Quality"],
        (RouteKind::System, "Library Health") => &["Issues", "Replacements", "Reports"],
        (RouteKind::System, "Quarantine Jury") => &["Pending", "Approved", "Rejected"],
        (RouteKind::System, "Files") => &["Index", "Fingerprints", "Records"],
        (RouteKind::System, "Data") => &["Database", "Vacuum", "Cleanup", "Storage"],
        (RouteKind::System, "Events") => &["Filterable stream", "Acknowledgements"],
        (RouteKind::System, "Logs") => &["Level", "Source", "Search"],
        (RouteKind::System, "Metrics") => &["KPIs", "Transfer summary", "Raw metrics"],
        (RouteKind::Browse, _) => &[
            "Cached browse",
            "Breadcrumb",
            "Multi-select",
            "Queue preview",
        ],
        (RouteKind::Messages | RouteKind::Rooms, _) => {
            &["Conversations", "Unread", "Rooms", "Pods", "Compose"]
        }
        (RouteKind::Collections, _) => &["Collection detail", "Items", "Picker", "Sharing"],
        (RouteKind::ShareGroups, _) => &["Members", "Grants", "Tokens", "Permissions"],
        (RouteKind::SharedWithMe, _) => &["Owner", "Expiration", "Access", "Manifest"],
        _ => &["Loading", "Empty", "Error", "Success"],
    }
}

fn native_filter_html() -> String {
    r#"<div class="slskr-native-filterbar"><input type="search" data-slskr-native-filter aria-label="Filter visible rows" placeholder="Filter visible rows"><button type="button" data-slskr-native-filter-clear>Clear Filter</button><button type="button" data-slskr-native-select-visible>Select Visible</button><button type="button" data-slskr-native-clear-selection>Clear Selection</button><button type="button" data-slskr-native-reset-state>Reset Table</button><span data-slskr-native-count>0 rows</span></div>"#.to_string()
}

fn native_inspector_html() -> String {
    r#"<aside class="slskr-native-inspector" id="slskr-native-inspector" aria-live="polite"><header><div><h3>Selection Inspector</h3><p>Choose a row to inspect details and actions.</p></div><span data-slskr-native-inspector-count>0 selected</span></header><dl><dt>Item</dt><dd data-slskr-native-inspector-title>Nothing selected</dd><dt>Detail</dt><dd data-slskr-native-inspector-detail>Use the table to choose an item.</dd><dt>State</dt><dd data-slskr-native-inspector-meta>Waiting</dd><dt>Action</dt><dd data-slskr-native-inspector-action>Review</dd><dt>Fields</dt><dd data-slskr-native-inspector-fields>Selection fields will appear here.</dd></dl><div class="slskr-native-inspector-actions" data-slskr-native-inspector-actions><button type="button">Review Selection</button><button type="button">Queue Selected</button></div></aside>"#.to_string()
}

fn native_selection_preview_html(title: &str, detail: &str, meta: &str, action: &str) -> String {
    format!(
        r#"<div class="slskr-native-preview-card" aria-live="polite"><span data-slskr-native-preview-count>0 selected</span><strong data-slskr-native-preview-title>{}</strong><p data-slskr-native-preview-detail>{}</p><small data-slskr-native-preview-fields>Selection fields will appear here.</small><em data-slskr-native-preview-meta>{}</em><button type="button" data-slskr-native-preview-action>{}</button></div>"#,
        escape_html(title),
        escape_html(detail),
        escape_html(meta),
        escape_html(action)
    )
}

fn native_editor_field_html(label: &str, control: &str) -> String {
    format!(
        r#"<label><span>{}</span>{}</label>"#,
        escape_html(label),
        control
    )
}

fn native_editor_text_field_html(label: &str, placeholder: &str) -> String {
    native_editor_field_html(
        label,
        &format!(
            r#"<input aria-label="{}" placeholder="{}">"#,
            escape_html(label),
            escape_html(placeholder)
        ),
    )
}

fn native_editor_checkbox_html(label: &str) -> String {
    native_editor_field_html(
        label,
        &format!(
            r#"<span class="slskr-native-editor-check"><input type="checkbox" aria-label="{}"> {}</span>"#,
            escape_html(label),
            escape_html(label)
        ),
    )
}

fn native_editor_action_buttons(labels: &[&str]) -> String {
    labels
        .iter()
        .map(|label| format!(r#"<button type="button">{}</button>"#, escape_html(label)))
        .collect::<Vec<_>>()
        .join("")
}

fn native_editor_state_items(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!(r#"<span>{}</span>"#, escape_html(item)))
        .collect::<Vec<_>>()
        .join("")
}

fn native_editor_modal_html(kind: RouteKind) -> String {
    let Some((title, summary, fields, actions, state)) = (match kind {
        RouteKind::Wishlist => Some((
            "Wishlist Editor",
            "Edit the wanted search, toggle automation, run it, or send the selected request into review.",
            [
                native_editor_text_field_html("Search Text", "wanted artist album"),
                native_editor_text_field_html("Filter", "flac OR mp3"),
                native_editor_text_field_html("Max Results", "25"),
                native_editor_checkbox_html("Enabled"),
                native_editor_checkbox_html("Auto-download"),
            ]
            .join(""),
            native_editor_action_buttons(&["Add Search", "Run Enabled", "Copy Review"]),
            native_editor_state_items(&[
                "Discovery Inbox bridge",
                "Quota preview",
                "Review state",
                "Last run and result count",
            ]),
        )),
        RouteKind::Users => Some((
            "User Note Editor",
            "Update watched-user context before browsing, messaging, or saving notes.",
            [
                native_editor_text_field_html("Username", "peer1"),
                native_editor_text_field_html("Display note", "trades live sets"),
                native_editor_text_field_html("Privilege note", "trusted / queued"),
                native_editor_checkbox_html("Watched"),
            ]
            .join(""),
            native_editor_action_buttons(&["Search for User", "Message", "Browse", "Watch", "Save note"]),
            native_editor_state_items(&[
                "Online status",
                "Privileges and stats",
                "Browse/message handoff",
            ]),
        )),
        RouteKind::Contacts => Some((
            "Contact Editor",
            "Create invites, classify nearby contacts, and maintain notes or groups without leaving Contacts.",
            [
                native_editor_text_field_html("Invite", "slskr://invite/..."),
                native_editor_text_field_html("Nickname", "friend name"),
                native_editor_text_field_html("Group", "trusted"),
                native_editor_text_field_html("Note", "met in public-domain room"),
            ]
            .join(""),
            native_editor_action_buttons(&["Create Invite", "Add Friend", "Message", "Browse", "Remove"]),
            native_editor_state_items(&[
                "Invite accept flow",
                "Nearby contacts",
                "Groups and notes",
                "Context actions",
            ]),
        )),
        RouteKind::Collections => Some((
            "Collection Editor",
            "Create a collection, pick library items, and prepare the sharing audience in one modal workflow.",
            [
                native_editor_text_field_html("Title", "Sunday radio archive"),
                native_editor_text_field_html("Description", "public domain recordings"),
                native_editor_text_field_html("Search for item", "filename, artist, title"),
                native_editor_text_field_html("Audience", "share group or token"),
            ]
            .join(""),
            native_editor_action_buttons(&["Create Collection", "Add Item", "Share", "Remove item"]),
            native_editor_state_items(&[
                "Library item picker",
                "Already in collection warning",
                "Audience picker",
                "Stream/download policies",
            ]),
        )),
        RouteKind::ShareGroups => Some((
            "Share Grant Editor",
            "Manage group members, tokens, permissions, and grant expiry from the selected share group.",
            [
                native_editor_text_field_html("Group Name", "crate-diggers"),
                native_editor_text_field_html("Soulseek Username", "peer1"),
                native_editor_text_field_html("Permissions", "read, stream, download"),
                native_editor_text_field_html("Expires", "never"),
            ]
            .join(""),
            native_editor_action_buttons(&[
                "Add Member",
                "Issue Token",
                "Create Share Grant",
                "Update Share Grant",
            ]),
            native_editor_state_items(&[
                "Member picker",
                "Token revoke",
                "Permission matrix",
                "Grant audit trail",
            ]),
        )),
        RouteKind::SharedWithMe => Some((
            "Inbound Access Editor",
            "Inspect inbound tokens, owner context, available files, and leave or backfill shared content.",
            [
                native_editor_text_field_html("Token", "share token"),
                native_editor_text_field_html("Owner", "peer1"),
                native_editor_text_field_html("Collection", "shared collection"),
                native_editor_text_field_html("Access", "stream/download"),
            ]
            .join(""),
            native_editor_action_buttons(&["Open", "Stream", "Backfill", "Copy token", "Leave share"]),
            native_editor_state_items(&[
                "Owner context",
                "Expiration",
                "Access status",
                "Manifest preview",
            ]),
        )),
        RouteKind::System => Some((
            "Settings Editor",
            "Edit operator preferences, confirm maintenance actions, and keep raw metrics in Developer only.",
            [
                native_editor_text_field_html("Option key", "transfers.downloadSlots"),
                native_editor_text_field_html("Value", "4"),
                native_editor_text_field_html("Filter", "shares, logs, database"),
                native_editor_checkbox_html("Dry run"),
            ]
            .join(""),
            native_editor_action_buttons(&[
                "Check for Updates",
                "Rescan shares",
                "Vacuum database",
                "Diagnostic Bundle",
            ]),
            native_editor_state_items(&[
                "Connection",
                "Shares",
                "Database/storage",
                "Logs/events",
                "Preferences",
            ]),
        )),
        _ => None,
    }) else {
        return String::new();
    };

    format!(
        r#"<aside class="slskr-native-editor" data-slskr-native-editor data-slskr-native-editor-route="{route}" aria-label="{title}"><header><div><h3>{title}</h3><p>{summary}</p></div><span>Draft</span></header><div class="slskr-native-editor-grid">{fields}</div><div class="slskr-native-editor-actions">{actions}</div><div class="slskr-native-editor-state">{state}</div></aside>"#,
        route = format!("{kind:?}"),
        title = escape_html(title),
        summary = escape_html(summary),
        fields = fields,
        actions = actions,
        state = state,
    )
}

fn native_browse_workspace_html(route_table: &str) -> String {
    let preview = native_selection_preview_html(
        "No files selected",
        "Select browsed files to preview batch download impact.",
        "Waiting",
        "Download Selected",
    );
    format!(
        r#"<div class="slskr-native-grid browse-native" data-slskr-browse-workspace><aside class="slskr-native-side slskr-native-sidebar"><h3>Browse</h3><div class="slskr-native-command-row"><input aria-label="Username" placeholder="Username"><button type="button">Open a New Browse Tab</button></div><div class="slskr-native-browse-tabs" role="tablist" aria-label="Browse sessions"><button type="button" aria-selected="true" data-slskr-browse-session="peer1:/Music/Open Sessions">peer1 /Music</button><button type="button" data-slskr-browse-session="peer2:/Incoming">peer2 /Incoming</button><button type="button">New Tab</button></div><div class="slskr-native-tree" data-slskr-browse-tree><strong>Directory Tree</strong><button type="button" data-slskr-browse-folder="/" aria-expanded="true">/</button><button type="button" data-slskr-browse-folder="/Music" aria-expanded="true">/Music</button><button type="button" data-slskr-browse-folder="/Music/Open Sessions" aria-current="true">/Music/Open Sessions</button><button type="button" data-slskr-browse-folder="/Incoming">/Incoming</button></div><div class="slskr-native-mini-list"><span>Cached browse result</span><span>Folder expansion state</span><span>Peer browse history</span></div></aside><section class="slskr-native-main"><h3>Files</h3><div class="slskr-native-breadcrumb" data-slskr-browse-breadcrumb><button type="button">peer1</button><span>/</span><button type="button">Music</button><span>/</span><button type="button">Open Sessions</button></div><div class="slskr-native-command-row"><input aria-label="Folder" placeholder="/" value="/Music/Open Sessions"><input aria-label="File filter" placeholder="Filter files"><button type="button">Refresh Folder</button><button type="button">Download Selected</button></div><div class="slskr-native-split-detail"><div>{route_table}</div><aside><h4>Download Preview</h4><p>Selected files, destination, peer, queue impact, and duplicate warnings.</p>{preview}<div class="slskr-native-mini-list" data-slskr-browse-download-manifest><span>Queue as batch</span><span>Preserve folders</span><span>Duplicate warning review</span><span>Destination: downloads/peer1/Music</span><span>Estimated queue impact: 2 files</span></div><div class="slskr-native-table-wrap"><table class="slskr-native-table"><thead><tr><th>Selected file</th><th>Peer</th><th>Action body</th></tr></thead><tbody><tr><td>/Music/Open Sessions/Theme.flac</td><td>peer1</td><td>download batch item</td></tr><tr><td>/Music/Open Sessions/Notes.txt</td><td>peer1</td><td>preserve folder</td></tr></tbody></table></div></aside></div></section></div>"#,
        route_table = route_table,
        preview = preview,
    )
}

fn native_messaging_workspace_html(route_table: &str) -> String {
    let preview = native_selection_preview_html(
        "Select a conversation",
        "Pick a thread, room, or pod row to load its reply and acknowledgement actions.",
        "Waiting",
        "Reply",
    );
    format!(
        r#"<div class="slskr-native-grid messaging-native" data-slskr-messages-workspace><aside class="slskr-native-side slskr-native-sidebar"><h3>Messages</h3><div class="slskr-native-command-row"><input aria-label="Chat username" placeholder="username"><button type="button">Direct Message</button></div><div class="slskr-native-message-search"><input aria-label="Search conversations" placeholder="Search conversations, rooms, pods"><button type="button">Clear Search</button></div><div class="slskr-native-list-stack"><h4>Conversations</h4>{route_table}<div class="slskr-native-mini-list" data-slskr-message-lifecycle><span>Unread badges</span><span>Acknowledge state</span><span>Delete lifecycle</span><span>Compose history</span></div><h4>Join Room</h4><div class="slskr-native-command-row"><input aria-label="Search rooms" placeholder="Search rooms"><button type="button">Join Room</button><button type="button">Create Room</button></div><div class="slskr-native-table-wrap"><table class="slskr-native-table" data-slskr-room-state><thead><tr><th>Room</th><th>State</th><th>Action</th></tr></thead><tbody><tr><td>public-domain</td><td>joined / 18 users / 2 unread</td><td><button type="button">Leave Room</button></td></tr><tr><td>ambient</td><td>available / 54 users</td><td><button type="button">Join Room</button></td></tr></tbody></table></div><h4>Pods</h4><div class="slskr-native-mini-list" data-slskr-pod-state><span>Pod channels stay secondary</span><span>Unread badges and room activity remain visible</span><span>Pod channel: library-review</span></div></div></aside><section class="slskr-native-main"><h3>Thread Workspace</h3>{preview}<div class="slskr-native-thread-grid" data-slskr-thread-state><article data-slskr-thread-kind="direct"><strong>peer1</strong><span class="slskr-native-badge">Unread 0</span><p>Last private message and acknowledgement state.</p><button type="button">Acknowledge</button></article><article data-slskr-thread-kind="room"><strong>public-domain</strong><span class="slskr-native-badge">Room</span><p>Joined room messages and member activity.</p><button type="button">Leave Room</button></article><article data-slskr-thread-kind="pod"><strong>pod channel</strong><span class="slskr-native-badge">Pod</span><p>Pod-linked channel messages stay in the same workspace.</p><button type="button">Delete Conversation</button></article></div><div class="slskr-native-thread-transcript" data-slskr-message-transcript><article><strong>peer1</strong><p>Can you browse my shared folder?</p><time>unread</time></article><article><strong>you</strong><p>I can, sending a browse request now.</p><time>draft history</time></article></div><textarea aria-label="Message" placeholder="Message"></textarea><div class="slskr-native-command-row" data-slskr-message-actions><button type="button">Reply</button><button type="button">Acknowledge</button><button type="button">Delete Conversation</button><button type="button">Collapse All Message Panels</button></div><div class="slskr-native-mini-list" data-slskr-compose-history><span>Last draft restored</span><span>Reply target: peer1</span><span>Enter sends when enabled</span></div></section></div>"#,
        route_table = route_table,
        preview = preview,
    )
}

fn native_search_filter_panel_html() -> String {
    r#"<section class="slskr-native-filter-modal" data-slskr-search-filter-modal><header><div><h4>Search Filters</h4><p>Format, bitrate, size, duration, queue, and duplicate controls stay visible beside results.</p></div><button type="button">Apply Filters</button></header><div class="slskr-native-filter-grid"><label><span>Include words</span><input aria-label="Include words" placeholder="remix instrumental"></label><label><span>Exclude words</span><input aria-label="Exclude words" placeholder="live demo"></label><label><span>Min bitrate</span><input aria-label="Min bitrate" placeholder="320"></label><label><span>Format</span><input aria-label="Format" placeholder="flac mp3 wav"></label><label><span>Min size</span><input aria-label="Min size" placeholder="1 MB"></label><label><span>Max size</span><input aria-label="Max size" placeholder="100 MB"></label><label><span>Min duration</span><input aria-label="Min duration" placeholder="3 min"></label><label><span>Max queue</span><input aria-label="Max queue" placeholder="8"></label></div><div class="slskr-native-filter-toggles"><label><input type="checkbox" aria-label="Fold duplicate results" checked> Fold duplicate results</label><label><input type="checkbox" aria-label="Prefer free upload slots" checked> Prefer free slots</label><label><input type="checkbox" aria-label="Hide locked files"> Hide locked files</label><select aria-label="Search ranking profile"><option>Smart ranking</option><option>Exact match first</option><option>Fastest peer first</option><option>Lossless first</option></select></div></section>"#.to_string()
}

fn native_final_parity_panel_html(kind: RouteKind) -> String {
    let (title, attr, controls, facts): (&str, &str, &[&str], &[&str]) = match kind {
        RouteKind::Search => (
            "Result Expansion",
            "data-slskr-search-final-parity",
            &[
                "Expand Result",
                "Fold Duplicates",
                "Apply Ranking",
                "Queue Exact",
            ],
            &[
                "Directory tree",
                "Locked files",
                "Duplicate providers",
                "Score reasons",
            ],
        ),
        RouteKind::DiscoveryGraph => (
            "Graph Canvas",
            "data-slskr-graph-final-parity",
            &[
                "Save Branch",
                "Weight Edges",
                "Queue Recommendation",
                "Open Node",
            ],
            &[
                "Artist nodes",
                "Album nodes",
                "Track nodes",
                "Provider edges",
            ],
        ),
        RouteKind::PlaylistIntake => (
            "Playlist Correction",
            "data-slskr-playlist-final-parity",
            &[
                "Upload Playlist",
                "Correct Row",
                "Open Provider Tab",
                "Queue Plan",
            ],
            &[
                "MusicBrainz tab",
                "SongID tab",
                "Organization plan",
                "Row errors",
            ],
        ),
        RouteKind::Wishlist => (
            "Discovery Inbox",
            "data-slskr-wishlist-final-parity",
            &[
                "Check Quota",
                "Persist Inbox",
                "Toggle Enabled",
                "Toggle Auto",
            ],
            &[
                "Quota portal",
                "Review requests",
                "Last run",
                "Result count",
            ],
        ),
        RouteKind::Downloads => (
            "Download Groups",
            "data-slskr-downloads-final-parity",
            &["Group by Peer", "Retry Group", "Cancel Group", "Set Slots"],
            &["Progress", "ETA", "Speed", "Queue position"],
        ),
        RouteKind::Uploads => (
            "Upload Policy",
            "data-slskr-uploads-final-parity",
            &["Edit Policy", "Group by Peer", "Allow Peer", "Deny Peer"],
            &["Allow list", "Deny list", "Queue rules", "Peer groups"],
        ),
        RouteKind::Messages | RouteKind::Rooms => (
            "Conversation Windows",
            "data-slskr-messages-final-parity",
            &["Open Window", "Create Room", "Join Room", "Restore Draft"],
            &[
                "Unread lifecycle",
                "Delete state",
                "Room list",
                "Pod channels",
            ],
        ),
        RouteKind::Users => (
            "Selected User Card",
            "data-slskr-users-final-parity",
            &[
                "Open Context Menu",
                "Browse User",
                "Message User",
                "Save Note",
            ],
            &["Privileges", "Stats", "Status", "Browse handoff"],
        ),
        RouteKind::Contacts => (
            "Invite and QR Flow",
            "data-slskr-contacts-final-parity",
            &[
                "Create QR Invite",
                "Scan QR Image",
                "Refresh Nearby",
                "Edit Group",
            ],
            &[
                "Invite link",
                "QR preview",
                "Nearby contacts",
                "Persisted notes",
            ],
        ),
        RouteKind::Solid => (
            "Solid Setup",
            "data-slskr-solid-final-parity",
            &[
                "Resolve WebID",
                "Connect Session",
                "Sync Storage",
                "Open Related",
            ],
            &["Identity", "Storage root", "Auth state", "Linked-data sync"],
        ),
        RouteKind::Collections => (
            "Collection Share Modal",
            "data-slskr-collections-final-parity",
            &[
                "Persist Draft",
                "Search Library",
                "Remove Item",
                "Pick Audience",
            ],
            &[
                "Item picker",
                "Audience picker",
                "Stream grant",
                "Download grant",
            ],
        ),
        RouteKind::ShareGroups => (
            "Share Group Detail",
            "data-slskr-sharegroups-final-parity",
            &[
                "Pick Member",
                "Remove Member",
                "Revoke Token",
                "Update Permissions",
            ],
            &["Members", "Grants", "Tokens", "Permission matrix"],
        ),
        RouteKind::SharedWithMe => (
            "Inbound Manifest",
            "data-slskr-shared-final-parity",
            &[
                "Open Item",
                "Stream Item",
                "Copy Exact Token",
                "Leave Share",
            ],
            &[
                "Manifest rows",
                "Owner contact",
                "Expiration",
                "Access status",
            ],
        ),
        RouteKind::Browse => (
            "Cached Browse Session",
            "data-slskr-browse-final-parity",
            &[
                "Restore Session",
                "Expand Tree",
                "Persist Breadcrumb",
                "Queue Selected",
            ],
            &[
                "Cached tree",
                "Folder expansion",
                "File split",
                "Download preview",
            ],
        ),
        RouteKind::System => (
            "Operator Tab Parity",
            "data-slskr-system-final-parity",
            &["Open Tab", "Run Job", "Copy Manifest", "Save Preference"],
            &["Info", "Network", "Security", "Metrics"],
        ),
    };
    let buttons = controls
        .iter()
        .map(|control| format!(r#"<button type="button">{}</button>"#, escape_html(control)))
        .collect::<Vec<_>>()
        .join("");
    let facts = facts
        .iter()
        .map(|fact| format!(r#"<span>{}</span>"#, escape_html(fact)))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<section class="slskr-native-final-parity" {attr}><header><div><h3>{title}</h3><p>Remaining legacy workflow controls are available from this route without opening Developer diagnostics.</p></div><div class="slskr-native-panel-actions">{buttons}</div></header><div class="slskr-native-panel-facts">{facts}</div></section>"#,
        attr = attr,
        title = escape_html(title),
        buttons = buttons,
        facts = facts,
    )
}

fn route_native_workspace_html(
    kind: RouteKind,
    rows: &[(String, String, String, String)],
) -> String {
    let route_table = native_route_table_html(kind, rows);
    let html = match kind {
        RouteKind::Search => format!(
            r#"<div class="slskr-native-grid search-native"><section class="slskr-native-main"><h3>Searches</h3><div class="slskr-native-command-row"><input aria-label="Search text" placeholder="Search" value="public domain jazz"><select aria-label="Acquisition profile"><option>Balanced</option><option>Lossless exact</option><option>Fast good enough</option></select><button type="button">Search</button><button type="button">Stop</button><button type="button">Clear</button></div>{filter_panel}{route_table}</section><aside class="slskr-native-side"><h3>Search Detail</h3><p>Select a search to inspect files, peers, queue, warnings, duplicate groups, and download preview.</p>{preview}{stats}<div class="slskr-native-mini-list" data-slskr-search-expansion><span>Expanded directories</span><span>Locked file warnings</span><span>Folded duplicate sources</span><span>Ranking reasons</span></div></aside></div>"#,
            filter_panel = native_search_filter_panel_html(),
            route_table = route_table,
            preview = native_selection_preview_html(
                "No search selected",
                "Choose a query or file result to inspect peers, score, queue state, and download actions.",
                "Waiting",
                "Download",
            ),
            stats = [
                native_stat_html("Result review", "ready"),
                native_stat_html("Duplicate folding", "on"),
                native_stat_html("Download preview", "manual"),
            ]
            .join(""),
        ),
        RouteKind::DiscoveryGraph => format!(
            r#"<div class="slskr-native-grid discovery-graph-native"><section class="slskr-native-main"><h3>Discovery Graph Atlas</h3><div class="slskr-native-command-row"><input aria-label="Artist Name" placeholder="Artist Name"><input aria-label="Album Title" placeholder="Album Title"><input aria-label="Track Title or Seed Label" placeholder="Track Title or Seed Label"><button type="button">Build Atlas</button><button type="button">Queue Nearby</button></div><div class="slskr-native-graph"><span>Artist</span><span>Album</span><span>Track</span><span>Query</span></div>{preview}</section><aside class="slskr-native-side"><h3>Recommendations</h3>{route_table}</aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No graph node selected",
                "Select a seed, recommendation, or next-search row to inspect graph context.",
                "Waiting",
                "Queue Nearby",
            ),
        ),
        RouteKind::PlaylistIntake => format!(
            r#"<div class="slskr-native-grid playlist-intake-native"><section class="slskr-native-main"><h3>Playlist Intake</h3><div class="slskr-native-command-row"><input aria-label="Playlist name" placeholder="Road trip, label sampler, friend recs"><input aria-label="Playlist source" placeholder="Local file name or provider URL"><button type="button">Import Playlist</button></div><textarea aria-label="Playlist rows" placeholder="Artist - Title, one row per track, or simple CSV artist,title"></textarea>{route_table}</section><aside class="slskr-native-side"><h3>Import validation</h3>{preview}{stats}</aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No playlist row selected",
                "Select a parsed row to review validation, classification, and acquisition planning.",
                "Waiting",
                "Import Playlist",
            ),
            stats = [
                native_stat_html("Playlists", "0"),
                native_stat_html("Tracks", "0"),
                native_stat_html("Unmatched", "0"),
            ]
            .join(""),
        ),
        RouteKind::Wishlist => format!(
            r#"<div class="slskr-native-grid wishlist-native"><section class="slskr-native-main"><h3>Wishlist</h3><div class="slskr-native-command-row"><input aria-label="Search Text" placeholder="Enter search terms..."><input aria-label="Filter optional" placeholder="e.g., flac OR mp3"><input aria-label="Max Results" value="25"><button type="button">Add Search</button><button type="button">Import List</button><button type="button">Run Enabled</button></div>{route_table}</section><aside class="slskr-native-side"><h3>Request Portal Summary</h3>{preview}{stats}<button type="button">Copy Review</button></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No wishlist item selected",
                "Choose a wanted search to run it, review matches, or send it to the discovery inbox.",
                "Waiting",
                "Run Enabled",
            ),
            stats = [
                native_stat_html("Requests", "0"),
                native_stat_html("Enabled", "0"),
                native_stat_html("Automatic", "0"),
                native_stat_html("Needs Review", "0"),
                native_stat_html("Within quota", "25 left"),
            ]
            .join(""),
        ),
        RouteKind::Downloads | RouteKind::Uploads => {
            let (title, empty, primary, secondary) = if kind == RouteKind::Downloads {
                (
                    "Downloads",
                    "No downloads to display",
                    "Retry All",
                    "Cancel All",
                )
            } else {
                (
                    "Uploads",
                    "No uploads to display",
                    "Allow selected",
                    "Deny selected",
                )
            };
            let table = native_table_html(
                kind,
                &["Filename", "Peer", "Progress", "Action"],
                rows,
                empty,
            );
            format!(
                r#"<div class="slskr-native-grid transfers-native"><section class="slskr-native-main"><h3>{title}</h3><div class="slskr-native-command-row"><button type="button">{primary}</button><button type="button">{secondary}</button><button type="button">Clear Completed</button><label><input type="checkbox"> Accelerated</label><label><input type="checkbox"> Auto Replace</label></div>{table}</section><aside class="slskr-native-side"><h3>Transfer Group</h3>{preview}{stats}</aside></div>"#,
                title = title,
                primary = primary,
                secondary = secondary,
                table = table,
                preview = native_selection_preview_html(
                    &format!("No {} selected", title.to_lowercase()),
                    "Choose a transfer to inspect peer, speed, progress, ETA, and retry or cancel actions.",
                    "Waiting",
                    primary,
                ),
                stats = [
                    native_stat_html("Active", "0"),
                    native_stat_html("Queued", "0"),
                    native_stat_html("Completed", "0"),
                ]
                .join(""),
            )
        }
        RouteKind::Messages | RouteKind::Rooms => native_messaging_workspace_html(&route_table),
        RouteKind::Users => format!(
            r#"<div class="slskr-native-grid users-native"><section class="slskr-native-main"><h3>Users</h3><div class="slskr-native-command-row"><input aria-label="Username" placeholder="Username"><button type="button">Search for User</button><button type="button">Clear Selected User</button><button type="button">Browse</button><button type="button">Message</button></div>{route_table}</section><aside class="slskr-native-side"><h3>User Detail</h3><p>No user info to display</p>{preview}<button type="button">Save note</button><button type="button">Watch</button></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No user selected",
                "Select a user to inspect status, privileges, browse, message, watch, or edit notes.",
                "Waiting",
                "Message",
            ),
        ),
        RouteKind::Contacts => format!(
            r#"<div class="slskr-native-grid contacts-native"><section class="slskr-native-main"><h3>Contacts</h3><div class="slskr-native-command-row"><input aria-label="Invite" placeholder="slskr://invite/..."><input aria-label="Nickname" placeholder="Friend's name"><button type="button">Create Invite</button><button type="button">Add Friend</button><button type="button">Refresh Nearby</button></div>{route_table}</section><aside class="slskr-native-side"><h3>All Contacts</h3>{preview}<button type="button">Message</button><button type="button">Browse</button><button type="button">Remove</button></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No contact selected",
                "Select a contact, nearby peer, or invite to message, browse, watch, or remove.",
                "Waiting",
                "Message",
            ),
        ),
        RouteKind::Solid => format!(
            r#"<div class="slskr-native-grid solid-native"><section class="slskr-native-main" data-testid="solid-root"><h3>Solid</h3><p>Solid integration is disabled (Feature.Solid=false).</p><div class="slskr-native-command-row"><input data-testid="solid-webid-input" aria-label="WebID" placeholder="https://example.com/profile/card#me"><button data-testid="solid-resolve-webid" type="button">Resolve WebID</button></div>{route_table}</section><aside class="slskr-native-side"><h3>Identity Document</h3>{preview}<pre>{{}}</pre></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No Solid resource selected",
                "Select an identity, storage, session, or sync row to inspect setup status.",
                "Waiting",
                "Resolve WebID",
            ),
        ),
        RouteKind::Collections => format!(
            r#"<div class="slskr-native-grid collections-native"><section class="slskr-native-main"><h3>Collections</h3><div class="slskr-native-command-row"><input aria-label="Title" placeholder="Enter collection title"><input aria-label="Description" placeholder="Optional description"><button type="button">Create Collection</button></div><div class="slskr-native-command-row"><input aria-label="Search for item" placeholder="Search by filename (e.g., sintel, aria, treasure)..."><button type="button">Add Item</button><button type="button">Share</button></div><div class="slskr-native-split-detail"><div>{route_table}</div><aside><h4>Item Picker</h4><p>Search library items, preview candidate metadata, and add selected tracks.</p>{preview}<div class="slskr-native-mini-list"><span>Filename match</span><span>Artist/title match</span><span>Already in collection warning</span></div></aside></div></section><aside class="slskr-native-side"><h3>Collection Detail</h3>{stats}<div class="slskr-native-mini-list"><span>Items table</span><span>Remove item</span><span>Audience picker</span><span>Stream/download policies</span></div></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No collection selected",
                "Choose a collection or library candidate before adding, removing, or sharing items.",
                "Waiting",
                "Review",
            ),
            stats = [
                native_stat_html("Title", "ready"),
                native_stat_html("Type", "Playlist"),
                native_stat_html("Items", "0"),
            ]
            .join(""),
        ),
        RouteKind::ShareGroups => format!(
            r#"<div class="slskr-native-grid sharegroups-native"><section class="slskr-native-main"><h3>Share Groups</h3><div class="slskr-native-command-row"><input aria-label="Group Name" placeholder="Enter group name"><button type="button">Create Group</button><button type="button">Create Your First Group</button></div><div class="slskr-native-split-detail"><div>{route_table}</div><aside><h4>Grant Matrix</h4>{preview}<div class="slskr-native-permission-grid"><span>Read</span><span>Stream</span><span>Download</span><span>Expires</span></div><button type="button">Create Share Grant</button><button type="button">Update Share Grant</button></aside></div></section><aside class="slskr-native-side"><h3>Members and Tokens</h3><input aria-label="Soulseek Username" placeholder="Enter username"><button type="button">Add Member</button><button type="button">Issue Token</button><div class="slskr-native-mini-list"><span>Member picker</span><span>Token revoke</span><span>Grant audit trail</span></div></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No group selected",
                "Select a group, member, grant, or token to inspect permissions.",
                "Waiting",
                "Update Grant",
            ),
        ),
        RouteKind::SharedWithMe => format!(
            r#"<div class="slskr-native-grid shared-native"><section class="slskr-native-main"><h3>Shared with Me</h3><div class="slskr-native-split-detail"><div>{route_table}</div><aside><h4>Shared Manifest</h4><p>Owner, expiration, permissions, and file-level access preview.</p>{preview}<div class="slskr-native-mini-list"><span>Manifest item rows</span><span>Stream available files</span><span>Backfill selected collection</span></div></aside></div></section><aside class="slskr-native-side"><h3>Access</h3><button type="button">Open</button><button type="button">Stream</button><button type="button">Backfill</button><button type="button">Copy token</button><button type="button">Leave share</button></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No shared item selected",
                "Choose an inbound grant or manifest row before opening or backfilling.",
                "Waiting",
                "Open",
            ),
        ),
        RouteKind::Browse => native_browse_workspace_html(&route_table),
        RouteKind::System => format!(
            r#"<div class="slskr-native-grid system-native"><section class="slskr-native-main"><h3>System</h3><div class="slskr-native-operator-bands"><span>Connection</span><span>Shares</span><span>Database</span><span>Events</span><span>Preferences</span><span>Automation</span></div>{route_table}</section><aside class="slskr-native-side"><h3>Operator Actions</h3>{preview}<button type="button">Check for Updates</button><button type="button">Get Privileges</button><button type="button">Diagnostic Bundle</button><button type="button">Setup Health</button><button type="button">Shut Down</button><button type="button">Restart</button></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No system item selected",
                "Select a status, share, database, event, or metric row before running operator actions.",
                "Waiting",
                "Diagnostic Bundle",
            ),
        ),
    };
    let editor = native_editor_modal_html(kind);
    format!(
        r#"<section class="slskr-native-workspace">{tabs}{filter}{html}{final_parity}{editor}{inspector}<p class="slskr-native-selection" id="slskr-native-selection-status" aria-live="polite">Select a row to review actions.</p></section>"#,
        tabs = native_tabs_html(kind),
        filter = native_filter_html(),
        final_parity = native_final_parity_panel_html(kind),
        editor = editor,
        inspector = native_inspector_html(),
    )
}

fn route_workflow_stats_html(kind: RouteKind, responses: Option<&[EndpointBody]>) -> String {
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
                response_value(responses, "/source-providers", "count"),
                "sources",
            ),
            ("Jobs", response_count(responses, "/jobs"), "automation"),
            ("Review", "ready".to_string(), "queue"),
        ],
        RouteKind::Wishlist => vec![
            ("Wanted", response_count(responses, "/wishlist"), "searches"),
            ("Enabled", "review".to_string(), "state"),
            ("Inbox", "0".to_string(), "pending"),
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
            ("Slots", "auto".to_string(), "limit"),
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
            ("Policy", "allow list".to_string(), "mode"),
        ],
        RouteKind::Messages => vec![
            (
                "Threads",
                response_count(responses, "/conversations"),
                "inbox",
            ),
            ("Unread", "0".to_string(), "messages"),
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
            ("Activity", "live".to_string(), "stream"),
        ],
        RouteKind::Users => vec![
            ("Watched", response_count(responses, "/users"), "users"),
            ("Online", "pending".to_string(), "presence"),
            ("Notes", response_count(responses, "/users/notes"), "saved"),
        ],
        RouteKind::Contacts => vec![
            ("Contacts", response_count(responses, "/contacts"), "people"),
            (
                "Nearby",
                response_count(responses, "/contacts/nearby"),
                "peers",
            ),
            ("Invites", "0".to_string(), "open"),
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
            ("Tokens", "0".to_string(), "issued"),
        ],
        RouteKind::SharedWithMe => vec![
            ("Shared", response_count(responses, "/shared"), "records"),
            (
                "Grants",
                response_count(responses, "/share-grants"),
                "access",
            ),
            ("Expiring", "0".to_string(), "soon"),
        ],
        RouteKind::Browse => vec![
            ("Peer", "peer1".to_string(), "target"),
            (
                "Folders",
                response_count(responses, "/users/:username/browse"),
                "cached",
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

fn route_workflow_toolbar_html(kind: RouteKind) -> String {
    match kind {
        RouteKind::Search => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="public domain jazz" aria-label="Search text"><select aria-label="Acquisition profile"><option>Balanced</option><option>Lossless exact</option><option>Fast good enough</option></select><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Search</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Clear</button></form>"#.to_string(),
        RouteKind::DiscoveryGraph => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="Archive Artist" aria-label="Seed artist or query"><select aria-label="Source"><option>Search history</option><option>Playlist</option><option>MusicBrainz</option></select><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Build graph</button></form>"#.to_string(),
        RouteKind::PlaylistIntake => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="Artist - Title" aria-label="Playlist text"><select aria-label="Acquisition profile"><option>Balanced</option><option>Lossless exact</option></select><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Preview playlist</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Queue plans</button></form>"#.to_string(),
        RouteKind::Wishlist => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="public domain jazz" aria-label="Wanted search"><label><input type="checkbox" checked> Enabled</label><label><input type="checkbox"> Auto-download</label><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Add wanted search</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Run selected</button></form>"#.to_string(),
        RouteKind::Downloads => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Download</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Clear completed</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Enable acceleration</button></div>"#.to_string(),
        RouteKind::Uploads => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="2">Clear completed</button><button type="button" class="slskr-toolbar-command">Allow selected</button><button type="button" class="slskr-toolbar-command">Deny selected</button></div>"#.to_string(),
        RouteKind::Messages => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="peer1" aria-label="Username"><input class="slskr-toolbar-input" value="hello" aria-label="Message"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Reply</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Acknowledge</button></form>"#.to_string(),
        RouteKind::Rooms => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="public-domain" aria-label="Room"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Join</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Leave</button></form>"#.to_string(),
        RouteKind::Users => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="peer1" aria-label="Username"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="1">Watch</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Save note</button><button type="button" class="slskr-toolbar-command">Browse</button></form>"#.to_string(),
        RouteKind::Contacts => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="peer1" aria-label="Contact username"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Add contact</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Edit note</button></form>"#.to_string(),
        RouteKind::Solid => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary">Connect identity</button><button type="button" class="slskr-toolbar-command">Sync storage</button><button type="button" class="slskr-toolbar-command">Refresh session</button></div>"#.to_string(),
        RouteKind::Collections => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="Open Sessions" aria-label="Collection name"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Create collection</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="4">Add item</button></form>"#.to_string(),
        RouteKind::ShareGroups => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="Trusted peers" aria-label="Group name"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="1">Create group</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Add member</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Issue token</button></form>"#.to_string(),
        RouteKind::SharedWithMe => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary">Open collection</button><button type="button" class="slskr-toolbar-command">Copy token</button><button type="button" class="slskr-toolbar-command">Leave share</button></div>"#.to_string(),
        RouteKind::Browse => r#"<form class="slskr-toolbar slskr-workflow-toolbar"><input class="slskr-toolbar-input" value="peer1" aria-label="Username"><input class="slskr-toolbar-input" value="/" aria-label="Folder"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Browse</button><button type="button" class="slskr-toolbar-command">Download selected</button></form>"#.to_string(),
        RouteKind::System => r#"<div class="slskr-toolbar slskr-workflow-toolbar"><button type="button" class="slskr-toolbar-command primary" data-slskr-toolbar-action="0">Connect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="1">Disconnect</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="2">Rescan shares</button><button type="button" class="slskr-toolbar-command" data-slskr-toolbar-action="3">Vacuum database</button></div>"#.to_string(),
    }
}

fn route_workflow_html(path: &str, responses: Option<&[EndpointBody]>) -> String {
    let kind = route_kind(path);
    let tabs = match kind {
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
    let (primary_title, primary_detail, table_headers, rows, side_title, side_body) = match kind {
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
    let table_rows = route_dynamic_rows(kind, responses).unwrap_or_else(|| {
        rows.iter()
            .map(|(primary, secondary, meta, action)| {
                (
                    (*primary).to_string(),
                    (*secondary).to_string(),
                    (*meta).to_string(),
                    (*action).to_string(),
                )
            })
            .collect()
    });
    format!(
        r#"<div class="slskr-workflow" data-slskr-route-kind="{kind:?}">{reference}{native}<details class="slskr-legacy-workflow"><summary>Additional workflow detail</summary><div class="slskr-workflow-tabs">{tabs}</div><div class="slskr-workflow-grid"><section class="slskr-workflow-primary"><header><div><h3>{primary_title}</h3><p>{primary_detail}</p></div>{fresh}</header>{table}</section><aside class="slskr-workflow-inspector"><h3>{side_title}</h3><p>{side_body}</p>{empty}</aside></div></details></div>"#,
        kind = kind,
        tabs = workflow_tabs_html(&tabs),
        reference = route_reference_panel_html(kind),
        native = route_native_workspace_html(kind, &table_rows),
        primary_title = escape_html(primary_title),
        primary_detail = escape_html(primary_detail),
        fresh = status_chip_html(
            "Data",
            if responses.is_some() {
                "loaded"
            } else {
                "loading"
            }
        ),
        table = workflow_table_owned_html(&table_headers, &table_rows),
        side_title = escape_html(side_title),
        side_body = escape_html(side_body),
        empty = empty_state_html(
            "Nothing selected",
            "Choose a row to inspect details and available actions.",
            "Review"
        ),
    )
}

pub fn route_workspace_pending_html(path: &str) -> String {
    route_workflow_html(path, None)
}

pub fn route_workspace_result_html(path: &str, responses: &[EndpointBody]) -> String {
    route_workflow_html(path, Some(responses))
}

#[cfg(test)]
fn experience_settings_panel_html() -> String {
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

#[cfg(test)]
fn automation_center_panel_html() -> String {
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
        r#"<section class="slskr-route-page" data-route="{path}"><header class="slskr-page-header"><div><p class="slskr-kicker">{surface}</p><h2>{title}</h2><p>{description}</p></div><div class="slskr-page-status">{chips}</div></header>{toolbar}<div class="slskr-route-summary"><h3>Overview</h3><ul id="slskr-route-summary">{summary}</ul></div><section class="slskr-work-area"><header><div><h3>Workspace</h3><span id="slskr-live-status" aria-live="polite">Workflow data refreshes from the daemon</span></div><div class="slskr-live-controls"><button type="button" data-slskr-refresh-route>Refresh</button><button type="button" data-slskr-focus-filter>Filter</button><button type="button" data-slskr-clear-filters>Clear filters</button></div></header><div id="slskr-page-data" class="slskr-page-data">{page_data}</div><p id="slskr-action-status" aria-live="polite"></p><div id="slskr-toast-region" class="slskr-toast-region" aria-live="polite"></div></section><details class="slskr-diagnostics"><summary>Developer</summary><div class="slskr-route-actions"><h3>Action wiring</h3><ul id="slskr-route-actions">{actions}</ul></div><div class="slskr-route-columns"><div><h3>Route Shape</h3><ul>{routes}</ul></div><div><h3>API Surface</h3><ul>{endpoints}</ul></div></div><div class="slskr-route-live"><h3>Raw Probe Status</h3><ul id="slskr-route-data">{route_data}</ul></div></details></section>"#,
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

fn player_footer_html() -> String {
    r#"<footer class="slskr-player" data-slskr-player data-slskr-player-rating-key="" data-slskr-player-radio-query=""><section><strong>Now Playing</strong><span id="slskr-player-now">Queue idle</span><span id="slskr-player-now-detail">No local stream selected</span></section><section class="slskr-player-controls" aria-label="Player controls"><button type="button" data-slskr-player-action="refresh">Refresh</button><button type="button" data-slskr-player-action="clear">Clear</button><button type="button" data-slskr-player-action="visualizer">Visualizer</button><button type="button" data-slskr-player-action="radio">Radio</button></section><section class="slskr-player-rating" aria-label="Player rating"><strong>Rating</strong><div id="slskr-player-rating-controls"><button type="button" data-slskr-player-rating="1">1</button><button type="button" data-slskr-player-rating="2">2</button><button type="button" data-slskr-player-rating="3">3</button><button type="button" data-slskr-player-rating="4">4</button><button type="button" data-slskr-player-rating="5">5</button></div><span id="slskr-player-rating-status">Not rated</span></section><section><strong>Radio</strong><span id="slskr-player-radio">No track selected</span><span id="slskr-player-transfers">0 down / 0 up</span></section><section><strong>Visualizer</strong><span id="slskr-player-visualizer">Checking status</span><span id="slskr-player-status" aria-live="polite">Rust player surface ready</span></section></footer>"#.to_string()
}

fn rust_milkdrop_panel_html() -> String {
    r#"<section class="slskr-milkdrop-panel" id="slskr-rust-milkdrop" hidden data-slskr-milkdrop-running="false"><header><div><strong>MilkDrop</strong><span id="slskr-milkdrop-preset">slskr native warp</span></div><div class="slskr-milkdrop-actions"><button type="button" data-slskr-milkdrop-action="preset">Preset</button><button type="button" data-slskr-milkdrop-action="external">External</button><button type="button" data-slskr-milkdrop-action="close">Close</button></div></header><canvas id="slskr-milkdrop-canvas" width="960" height="360" aria-label="MilkDrop visualizer"></canvas><footer><span id="slskr-milkdrop-status">Visualizer ready</span><span>Preset-reactive player visualization</span></footer></section>"#.to_string()
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
        r#"<div class="slskr-shell"><nav class="slskr-nav">{nav}</nav><main class="slskr-main"><header class="slskr-appbar"><div><strong>slskr</strong><span>Search, transfers, messages, rooms, browse, sharing, and system control</span></div><ul id="slskr-runtime-status">{runtime}</ul></header><section id="slskr-route-view">{route_page}</section></main>{milkdrop}{player}</div>"#,
        route_page = route_page_html("/searches"),
        runtime = runtime_probe_pending_html(),
        milkdrop = rust_milkdrop_panel_html(),
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
    mount_native_tables(document)?;
    mount_native_subviews(document)?;
    mount_native_actions(document)?;
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
fn mount_native_tables(document: &web_sys::Document) -> Result<(), JsValue> {
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

#[cfg(target_arch = "wasm32")]
fn focus_relative_native_row(
    document: &web_sys::Document,
    current: &web_sys::Element,
    offset: isize,
) {
    let Some(workspace) = current.closest(".slskr-native-workspace").ok().flatten() else {
        return;
    };
    let rows = visible_native_rows(&workspace);
    let Some(current_index) = rows.iter().position(|row| row.is_same_node(Some(current))) else {
        return;
    };
    let target_index = (current_index as isize + offset).clamp(0, rows.len() as isize - 1) as usize;
    if let Some(row) = rows.get(target_index) {
        focus_native_row(document, row);
    }
}

#[cfg(target_arch = "wasm32")]
fn focus_edge_native_row(document: &web_sys::Document, current: &web_sys::Element, first: bool) {
    let Some(workspace) = current.closest(".slskr-native-workspace").ok().flatten() else {
        return;
    };
    let rows = visible_native_rows(&workspace);
    let row = if first { rows.first() } else { rows.last() };
    if let Some(row) = row {
        focus_native_row(document, row);
    }
}

#[cfg(target_arch = "wasm32")]
fn visible_native_rows(workspace: &web_sys::Element) -> Vec<web_sys::Element> {
    let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") else {
        return Vec::new();
    };
    let mut visible = Vec::new();
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if row.has_attribute("hidden") {
            continue;
        }
        visible.push(row);
    }
    visible
}

#[cfg(target_arch = "wasm32")]
fn focus_native_row(document: &web_sys::Document, row: &web_sys::Element) {
    if let Some(element) = row.dyn_ref::<web_sys::HtmlElement>() {
        let _ = element.focus();
    }
    select_native_row(document, row);
}

#[cfg(target_arch = "wasm32")]
fn mount_native_subviews(document: &web_sys::Document) -> Result<(), JsValue> {
    let tabs = document.query_selector_all("[data-slskr-native-tab]")?;
    for tab_index in 0..tabs.length() {
        let Some(node) = tabs.item(tab_index) else {
            continue;
        };
        let tab: web_sys::Element = node.dyn_into()?;
        let document_for_click = document.clone();
        let tab_for_click = tab.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                select_native_subview(&document_for_click, &tab_for_click);
            },
        ));
        tab.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn select_native_subview(document: &web_sys::Document, tab: &web_sys::Element) {
    let selected_index = tab
        .get_attribute("data-slskr-native-tab")
        .unwrap_or_else(|| "0".to_string());
    let workspace = tab
        .closest(".slskr-native-workspace")
        .ok()
        .flatten()
        .or_else(|| {
            document
                .query_selector(".slskr-native-workspace")
                .ok()
                .flatten()
        });
    let Some(workspace) = workspace else {
        return;
    };
    if let Ok(tabs) = workspace.query_selector_all("[data-slskr-native-tab]") {
        for index in 0..tabs.length() {
            let Some(node) = tabs.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let active = element
                .get_attribute("data-slskr-native-tab")
                .is_some_and(|value| value == selected_index);
            let _ = element.set_attribute("aria-selected", if active { "true" } else { "false" });
            element.set_class_name(if active {
                "slskr-native-tab is-active"
            } else {
                "slskr-native-tab"
            });
        }
    }
    if let Ok(panels) = workspace.query_selector_all("[data-slskr-native-panel]") {
        for index in 0..panels.length() {
            let Some(node) = panels.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let active = element
                .get_attribute("data-slskr-native-panel")
                .is_some_and(|value| value == selected_index);
            if active {
                let _ = element.remove_attribute("hidden");
            } else {
                let _ = element.set_attribute("hidden", "");
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_native_actions(document: &web_sys::Document) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-native-workspace button:not([data-slskr-native-tab]):not([data-slskr-native-filter-clear]):not([data-slskr-native-select-visible]):not([data-slskr-native-clear-selection]):not([data-slskr-native-reset-state])")?;
    for button_index in 0..buttons.length() {
        let Some(node) = buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let document_for_click = document.clone();
        let button_for_click = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                event.stop_propagation();
                handle_native_action(&document_for_click, &button_for_click);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn handle_native_action(document: &web_sys::Document, button: &web_sys::Element) {
    let action = button
        .text_content()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Run action".to_string());
    let route_label = button
        .closest(".slskr-workflow")
        .ok()
        .flatten()
        .and_then(|workflow| workflow.get_attribute("data-slskr-route-kind"))
        .unwrap_or_else(|| "Workflow".to_string());
    let route_path = document
        .default_view()
        .and_then(|window| window.location().pathname().ok())
        .unwrap_or_else(|| "/searches".to_string());
    if let Some(route_action) = route_action_for_native_label(&route_path, &action) {
        run_native_route_action(document, button, route_action);
        return;
    }
    let selected = document
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-title"));
    let target = selected.unwrap_or_else(|| route_label.clone());
    if let Some(message) = native_local_action_message(&route_path, &action, &target) {
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html(&format!(
                "<strong>{}</strong> {}",
                escape_html(&action),
                escape_html(&message)
            ));
        }
        show_toast(document, &format!("{action}: {message}"));
        return;
    }
    let message = format!("{} queued for {}", action, target);
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>Action</strong> {}",
            escape_html(&message)
        ));
    }
    show_toast(document, &message);
}

#[cfg(target_arch = "wasm32")]
fn native_local_action_message(route_path: &str, action: &str, target: &str) -> Option<String> {
    let action = action.trim().to_ascii_lowercase();
    match (route_kind(route_path), action.as_str()) {
        (RouteKind::Search, "preview") => Some(format!("download preview opened for {target}")),
        (RouteKind::DiscoveryGraph, "expand") => {
            Some(format!("graph inspector expanded for {target}"))
        }
        (RouteKind::Messages | RouteKind::Rooms, "collapse all message panels") => {
            Some("message panels collapsed in this workspace".to_string())
        }
        (RouteKind::Messages | RouteKind::Rooms, "create room") => {
            Some(format!("room draft prepared for {target}"))
        }
        (RouteKind::Wishlist, "copy review") => Some(format!("review summary copied for {target}")),
        (RouteKind::Wishlist, "import list") => {
            Some("wishlist import staged for review".to_string())
        }
        (RouteKind::Contacts, "create invite") => {
            Some("invite draft created for the current contact form".to_string())
        }
        (RouteKind::Contacts, "refresh nearby") => {
            Some("nearby contacts refresh requested".to_string())
        }
        (RouteKind::Collections, "open") => Some(format!("collection opened for {target}")),
        (RouteKind::Collections, "remove item") => {
            Some(format!("remove-item review staged for {target}"))
        }
        (RouteKind::SharedWithMe, "copy token") => Some(format!("token copied for {target}")),
        (RouteKind::ShareGroups, "token revoke") => {
            Some(format!("token revoke staged for {target}"))
        }
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
fn run_native_route_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) {
    if native_action_requires_confirmation(action) {
        show_native_confirm_modal(document, button, action);
        return;
    }
    execute_native_route_action(document, button, action);
}

#[cfg(target_arch = "wasm32")]
fn native_action_requires_confirmation(action: RouteAction) -> bool {
    action.method == "DELETE"
        || matches!(
            action.label,
            "Cancel Download" | "Deny Upload" | "Shut Down" | "Restart" | "Vacuum Database"
        )
}

#[cfg(target_arch = "wasm32")]
fn show_native_confirm_modal(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) {
    if let Some(existing) = document.get_element_by_id("slskr-native-confirm-modal") {
        existing.remove();
    }
    let Ok(backdrop) = document.create_element("div") else {
        execute_native_route_action(document, button, action);
        return;
    };
    backdrop.set_id("slskr-native-confirm-modal");
    backdrop.set_class_name("slskr-modal-backdrop");
    let _ = backdrop.set_attribute("role", "presentation");

    let title = format!("Confirm {}", action.label);
    let target = document_selected_native_row_title(document)
        .unwrap_or_else(|| "the selected workflow item".to_string());
    backdrop.set_inner_html(&format!(
        r#"<section class="slskr-modal" role="dialog" aria-modal="true" aria-labelledby="slskr-confirm-title"><header><h3 id="slskr-confirm-title">{title}</h3></header><p>{message}</p><div class="slskr-modal-actions"><button type="button" data-slskr-confirm-cancel>Cancel</button><button type="button" data-slskr-confirm-run>Confirm</button></div></section>"#,
        title = escape_html(&title),
        message = escape_html(&format!(
            "{} will run against {}. Review the selected row before continuing.",
            action.label, target
        )),
    ));
    let Some(body) = document.body() else {
        execute_native_route_action(document, button, action);
        return;
    };
    let _ = body.append_child(&backdrop);

    if let Ok(Some(cancel)) = backdrop.query_selector("[data-slskr-confirm-cancel]") {
        let document_for_cancel = document.clone();
        let backdrop_for_cancel = backdrop.clone();
        let label = action.label.to_string();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                backdrop_for_cancel.remove();
                if let Some(status) = document_for_cancel.get_element_by_id("slskr-action-status") {
                    status.set_inner_html(&format!(
                        "<strong>{}</strong> cancelled",
                        escape_html(&label)
                    ));
                }
                show_toast(&document_for_cancel, &format!("{label} cancelled"));
            },
        ));
        let _ = cancel.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref());
        callback.forget();
    }

    if let Ok(Some(confirm)) = backdrop.query_selector("[data-slskr-confirm-run]") {
        let document_for_confirm = document.clone();
        let button_for_confirm = button.clone();
        let backdrop_for_confirm = backdrop.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                backdrop_for_confirm.remove();
                execute_native_route_action(&document_for_confirm, &button_for_confirm, action);
            },
        ));
        let _ =
            confirm.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref());
        callback.forget();
    }

    if let Ok(Some(confirm)) = backdrop.query_selector("[data-slskr-confirm-run]") {
        if let Some(element) = confirm.dyn_ref::<web_sys::HtmlElement>() {
            let _ = element.focus();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn execute_native_route_action(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) {
    let Some(window) = document.default_view() else {
        return;
    };
    let route_path = window
        .location()
        .pathname()
        .unwrap_or_else(|_| "/searches".to_string());
    let value = native_action_value(document, button, action.body);
    let target = native_action_target(document, button, action);
    let id = native_action_id(document, button, action);
    let body = action_body_from_value(action.body, &value);
    let method = action.method.to_string();
    let label = action.label.to_string();
    let path = concrete_action_path_with_target_and_id(
        &route_path,
        action,
        target.as_deref(),
        id.as_deref(),
    );
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>{}</strong> sending {}",
            escape_html(&label),
            escape_html(&method)
        ));
    }
    show_toast(document, &format!("{} sending", label));
    let document = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            match result {
                Ok(response) => status.set_inner_html(&format!(
                    "<strong>{}</strong> {}",
                    escape_html(&label),
                    escape_html(&compact_preview(&response))
                )),
                Err(error) => {
                    let message = error
                        .as_string()
                        .unwrap_or_else(|| "request failed".to_string());
                    status.set_inner_html(&format!(
                        "<strong>{}</strong> {}",
                        escape_html(&label),
                        escape_html(&message)
                    ));
                }
            }
        }
        let _ = refresh_route_data(&window).await;
    });
}

#[cfg(target_arch = "wasm32")]
fn native_action_value(
    document: &web_sys::Document,
    button: &web_sys::Element,
    body: ActionBody,
) -> String {
    if let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() {
        if matches!(body, ActionBody::DownloadFiles) {
            if let Some(value) =
                selected_native_row_attribute_values(&workspace, "data-slskr-native-filename")
            {
                return value;
            }
            if let Some(value) = button_native_row_attribute(button, "data-slskr-native-filename") {
                return value;
            }
            if let Some(value) = selected_native_row_titles(&workspace) {
                return value;
            }
        }
        if matches!(body, ActionBody::BrowseDirectory) {
            if let Some(value) = button_native_row_attribute(button, "data-slskr-native-path")
                .filter(|_| {
                    button_native_row_attribute(button, "data-slskr-native-entry-kind").as_deref()
                        == Some("folder")
                })
            {
                return value;
            }
        }
        if matches!(body, ActionBody::Username) {
            for attr in [
                "data-slskr-native-username",
                "data-slskr-native-peer",
                "data-slskr-native-owner",
                "data-slskr-native-contact",
            ] {
                if let Some(value) = button_native_row_attribute(button, attr) {
                    return value;
                }
                if let Some(value) = selected_native_row_attribute(&workspace, attr) {
                    return value;
                }
            }
        }
        if matches!(body, ActionBody::SearchText) {
            if let Some(value) = button_native_row_target(button) {
                return value;
            }
            if let Some(value) = selected_native_row_target(&workspace) {
                return value;
            }
        }
        for selector in native_action_value_selectors(body) {
            if let Some(value) = first_workspace_value(&workspace, selector) {
                return value;
            }
        }
        if matches!(body, ActionBody::CollectionItem) {
            if let Some(value) = button_native_row_attribute(button, "data-slskr-native-collection")
                .or_else(|| {
                    selected_native_row_attribute(&workspace, "data-slskr-native-collection")
                })
            {
                return value;
            }
        }
        if matches!(body, ActionBody::ShareGrant | ActionBody::ShareGroupMember) {
            for attr in [
                "data-slskr-native-username",
                "data-slskr-native-peer",
                "data-slskr-native-owner",
            ] {
                if let Some(value) = button_native_row_attribute(button, attr) {
                    return value;
                }
                if let Some(value) = selected_native_row_attribute(&workspace, attr) {
                    return value;
                }
            }
        }
        if matches!(
            body,
            ActionBody::DownloadFiles
                | ActionBody::ShareGrant
                | ActionBody::ShareGroupMember
                | ActionBody::Username
        ) {
            if let Some(value) = selected_native_row_title(&workspace) {
                return value;
            }
        }
        for selector in native_generic_value_selectors() {
            if let Some(value) = first_workspace_value(&workspace, selector) {
                return value;
            }
        }
    }
    document_selected_native_row_title(document).unwrap_or_else(|| native_action_fallback(body))
}

#[cfg(target_arch = "wasm32")]
fn native_action_value_selectors(body: ActionBody) -> &'static [&'static str] {
    match body {
        ActionBody::BrowseDirectory => &[r#"input[aria-label="Folder"]"#],
        ActionBody::CollectionItem => &[
            r#"input[aria-label="Search for item"]"#,
            r#"input[aria-label="Title"]"#,
        ],
        ActionBody::ConversationMessage | ActionBody::RoomMessage => &[
            r#"textarea[aria-label="Message"]"#,
            r#"input[aria-label="Message"]"#,
            r#"input[aria-label="Chat username"]"#,
        ],
        ActionBody::DownloadFiles => &[
            r#"input[aria-label="Folder"]"#,
            r#"input[aria-label="Search for item"]"#,
        ],
        ActionBody::FeedPreview => &[
            r#"textarea[aria-label="Playlist rows"]"#,
            r#"input[aria-label="Playlist source"]"#,
            r#"input[aria-label="Playlist name"]"#,
            r#"input[aria-label="Playlist text"]"#,
        ],
        ActionBody::JsonString => &[
            r#"input[aria-label="Search rooms"]"#,
            r#"input[aria-label="Room"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Artist Name"]"#,
        ],
        ActionBody::NameDescription => &[
            r#"input[aria-label="Title"]"#,
            r#"input[aria-label="Group Name"]"#,
            r#"input[aria-label="Collection name"]"#,
            r#"input[aria-label="Description"]"#,
        ],
        ActionBody::Permissions => &[r#"select[aria-label="Permissions"]"#],
        ActionBody::SearchText => &[
            r#"input[aria-label="Search text"]"#,
            r#"input[aria-label="Search Text"]"#,
            r#"input[aria-label="Wanted search"]"#,
            r#"input[aria-label="Artist Name"]"#,
            r#"input[aria-label="Seed artist or query"]"#,
            r#"textarea[aria-label="Playlist rows"]"#,
        ],
        ActionBody::ShareGrant | ActionBody::ShareGroupMember | ActionBody::Username => &[
            r#"input[aria-label="Username"]"#,
            r#"input[aria-label="Soulseek Username"]"#,
            r#"input[aria-label="Contact username"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Nickname"]"#,
        ],
        ActionBody::EnabledFalse | ActionBody::EnabledTrue | ActionBody::None => &[],
    }
}

#[cfg(target_arch = "wasm32")]
fn native_generic_value_selectors() -> &'static [&'static str] {
    &[
        "input:not([type=checkbox]):not([type=radio])",
        "textarea",
        "select",
    ]
}

#[cfg(target_arch = "wasm32")]
fn native_action_target(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) -> Option<String> {
    if !action.path.contains(":username")
        && !action.path.contains(":roomName")
        && !action.path.contains(":id")
    {
        return None;
    }
    let workspace = button.closest(".slskr-native-workspace").ok().flatten()?;
    if let Some(value) =
        button_native_row_resource_id(button).filter(|value| safe_route_segment(value))
    {
        return Some(value);
    }
    if let Some(value) =
        selected_native_row_resource_id(&workspace).filter(|value| safe_route_segment(value))
    {
        return Some(value);
    }
    if let Some(value) = button_native_row_target(button).filter(|value| safe_route_segment(value))
    {
        return Some(value);
    }
    if let Some(value) =
        selected_native_row_target(&workspace).filter(|value| safe_route_segment(value))
    {
        return Some(value);
    }
    let selectors: &[&str] = if action.path.contains(":roomName") {
        &[
            r#"input[aria-label="Search rooms"]"#,
            r#"input[aria-label="Room"]"#,
        ]
    } else {
        &[
            r#"input[aria-label="Username"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Contact username"]"#,
            r#"input[aria-label="Soulseek Username"]"#,
        ]
    };
    for selector in selectors {
        if let Some(value) = first_workspace_value(&workspace, selector).filter(|value| {
            value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        }) {
            return Some(value);
        }
    }
    document_selected_native_row_title(document).filter(|value| safe_route_segment(value))
}

#[cfg(target_arch = "wasm32")]
fn native_action_id(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) -> Option<String> {
    if !action.path.contains(":id") {
        return None;
    }
    let workspace = button.closest(".slskr-native-workspace").ok().flatten()?;
    for attr in [
        "data-slskr-native-item-id",
        "data-slskr-native-transfer-id",
        "data-slskr-native-wishlist-id",
        "data-slskr-native-grant-id",
        "data-slskr-native-share-group-id",
        "data-slskr-native-collection-id",
        "data-slskr-native-search-id",
    ] {
        if let Some(value) =
            button_native_row_attribute(button, attr).filter(|value| safe_route_segment(value))
        {
            return Some(value);
        }
        if let Some(value) = selected_native_row_attribute(&workspace, attr)
            .filter(|value| safe_route_segment(value))
        {
            return Some(value);
        }
    }
    if !action.path.contains(":username") && !action.path.contains(":roomName") {
        if let Some(value) =
            button_native_row_resource_id(button).filter(|value| safe_route_segment(value))
        {
            return Some(value);
        }
        if let Some(value) =
            selected_native_row_resource_id(&workspace).filter(|value| safe_route_segment(value))
        {
            return Some(value);
        }
    }
    document_selected_native_row_title(document).filter(|value| safe_route_segment(value))
}

#[cfg(target_arch = "wasm32")]
fn first_workspace_value(workspace: &web_sys::Element, selector: &str) -> Option<String> {
    workspace
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(|element| form_control_value(&element))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_detail(workspace: &web_sys::Element) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-detail"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_title(workspace: &web_sys::Element) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_titles(workspace: &web_sys::Element) -> Option<String> {
    let rows = workspace
        .query_selector_all("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()?;
    let mut values = Vec::new();
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(value) = row
            .get_attribute("data-slskr-native-title")
            .filter(|value| !value.trim().is_empty())
        {
            values.push(value);
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values.join("\n"))
    }
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_attribute(workspace: &web_sys::Element, attribute: &str) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute(attribute))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_attribute_values(
    workspace: &web_sys::Element,
    attribute: &str,
) -> Option<String> {
    let rows = workspace
        .query_selector_all("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()?;
    let mut values = Vec::new();
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(value) = row
            .get_attribute(attribute)
            .filter(|value| !value.trim().is_empty())
        {
            values.push(value);
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values.join("\n"))
    }
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_target(workspace: &web_sys::Element) -> Option<String> {
    selected_native_row_detail(workspace)
        .filter(|value| safe_route_segment(value))
        .or_else(|| selected_native_row_title(workspace).filter(|value| safe_route_segment(value)))
}

#[cfg(target_arch = "wasm32")]
fn selected_native_row_resource_id(workspace: &web_sys::Element) -> Option<String> {
    workspace
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-resource-id"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn button_native_row_target(button: &web_sys::Element) -> Option<String> {
    button
        .closest("[data-slskr-native-select]")
        .ok()
        .flatten()
        .and_then(|row| {
            row.get_attribute("data-slskr-native-detail")
                .filter(|value| safe_route_segment(value))
                .or_else(|| {
                    row.get_attribute("data-slskr-native-title")
                        .filter(|value| safe_route_segment(value))
                })
        })
}

#[cfg(target_arch = "wasm32")]
fn button_native_row_attribute(button: &web_sys::Element, attribute: &str) -> Option<String> {
    button
        .closest("[data-slskr-native-select]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute(attribute))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn button_native_row_resource_id(button: &web_sys::Element) -> Option<String> {
    button
        .closest("[data-slskr-native-select]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-resource-id"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn document_selected_native_row_title(document: &web_sys::Document) -> Option<String> {
    document
        .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
        .ok()
        .flatten()
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .filter(|value| !value.trim().is_empty())
}

#[cfg(any(target_arch = "wasm32", test))]
fn native_action_fallback(body: ActionBody) -> String {
    match body {
        ActionBody::BrowseDirectory => "/".to_string(),
        ActionBody::CollectionItem => "Demo Track".to_string(),
        ActionBody::ConversationMessage => "hello".to_string(),
        ActionBody::DownloadFiles => "Remote/Song.mp3".to_string(),
        ActionBody::FeedPreview => "Public Domain Jazz - Demo Track".to_string(),
        ActionBody::JsonString => "contract-room".to_string(),
        ActionBody::NameDescription => "Rust Web Demo".to_string(),
        ActionBody::Permissions => "read".to_string(),
        ActionBody::RoomMessage => "hello room".to_string(),
        ActionBody::SearchText => "public domain jazz".to_string(),
        ActionBody::ShareGrant | ActionBody::ShareGroupMember | ActionBody::Username => {
            "peer1".to_string()
        }
        ActionBody::EnabledFalse | ActionBody::EnabledTrue | ActionBody::None => String::new(),
    }
}

#[cfg(target_arch = "wasm32")]
fn form_control_value(element: &web_sys::Element) -> Option<String> {
    if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        return Some(input.value());
    }
    if let Some(textarea) = element.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        return Some(textarea.value());
    }
    if let Some(select) = element.dyn_ref::<web_sys::HtmlSelectElement>() {
        return Some(select.value());
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn mount_native_filters(document: &web_sys::Document) -> Result<(), JsValue> {
    let inputs = document.query_selector_all("[data-slskr-native-filter]")?;
    for input_index in 0..inputs.length() {
        let Some(node) = inputs.item(input_index) else {
            continue;
        };
        let input: web_sys::HtmlInputElement = node.dyn_into()?;
        let workspace = input
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native filter is outside workspace"))?;
        let restored = restore_native_filter(document, &workspace);
        if restored.is_empty() {
            update_native_filter_count(&workspace);
        } else {
            input.set_value(&restored);
            apply_native_filter(&workspace, &restored);
        }

        let workspace_for_input = workspace.clone();
        let callback =
            Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
                let term = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value().to_lowercase())
                    .unwrap_or_default();
                apply_native_filter(&workspace_for_input, &term);
                persist_native_filter(&workspace_for_input, &term);
            }));
        input.add_event_listener_with_callback("input", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let clear_buttons = document.query_selector_all("[data-slskr-native-filter-clear]")?;
    for button_index in 0..clear_buttons.length() {
        let Some(node) = clear_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native filter clear is outside workspace"))?;
        let workspace_for_clear = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                if let Ok(Some(filter)) =
                    workspace_for_clear.query_selector("[data-slskr-native-filter]")
                {
                    if let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() {
                        input.set_value("");
                    }
                }
                apply_native_filter(&workspace_for_clear, "");
                persist_native_filter(&workspace_for_clear, "");
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let select_buttons = document.query_selector_all("[data-slskr-native-select-visible]")?;
    for button_index in 0..select_buttons.length() {
        let Some(node) = select_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native select visible is outside workspace"))?;
        let workspace_for_select = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                select_visible_native_rows(&workspace_for_select);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let selection_clear_buttons =
        document.query_selector_all("[data-slskr-native-clear-selection]")?;
    for button_index in 0..selection_clear_buttons.length() {
        let Some(node) = selection_clear_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native clear selection is outside workspace"))?;
        let workspace_for_clear = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                clear_native_selection(&workspace_for_clear);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    let reset_buttons = document.query_selector_all("[data-slskr-native-reset-state]")?;
    for button_index in 0..reset_buttons.length() {
        let Some(node) = reset_buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let workspace = button
            .closest(".slskr-native-workspace")?
            .ok_or_else(|| JsValue::from_str("native reset table is outside workspace"))?;
        let workspace_for_reset = workspace.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                reset_native_table_state(&workspace_for_reset);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn native_state_key(workspace: &web_sys::Element, suffix: &str) -> Option<String> {
    let route = workspace
        .closest("[data-route]")
        .ok()
        .flatten()
        .and_then(|route| route.get_attribute("data-route"))?;
    Some(format!("slskr.native.{route}.{suffix}"))
}

#[cfg(target_arch = "wasm32")]
fn session_storage_for_workspace(workspace: &web_sys::Element) -> Option<web_sys::Storage> {
    workspace
        .owner_document()
        .and_then(|document| document.default_view())
        .and_then(|window| window.session_storage().ok().flatten())
}

#[cfg(target_arch = "wasm32")]
fn restore_native_filter(document: &web_sys::Document, workspace: &web_sys::Element) -> String {
    let Some(key) = native_state_key(workspace, "filter") else {
        return String::new();
    };
    let Some(storage) = session_storage_for_workspace(workspace) else {
        return String::new();
    };
    let value = storage.get_item(&key).ok().flatten().unwrap_or_default();
    if !value.is_empty() {
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html(&format!(
                "<strong>Restored</strong> filter {}",
                escape_html(&value)
            ));
        }
    }
    value
}

#[cfg(target_arch = "wasm32")]
fn persist_native_filter(workspace: &web_sys::Element, term: &str) {
    let Some(key) = native_state_key(workspace, "filter") else {
        return;
    };
    let Some(storage) = session_storage_for_workspace(workspace) else {
        return;
    };
    if term.is_empty() {
        let _ = storage.remove_item(&key);
    } else {
        let _ = storage.set_item(&key, term);
    }
}

#[cfg(target_arch = "wasm32")]
fn reset_native_table_state(workspace: &web_sys::Element) {
    if let Ok(Some(filter)) = workspace.query_selector("[data-slskr-native-filter]") {
        if let Ok(input) = filter.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value("");
        }
    }
    apply_native_filter(workspace, "");
    persist_native_filter(workspace, "");
    clear_native_selection(workspace);
    reset_native_sort(workspace);

    if let Some(document) = workspace.owner_document() {
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html("<strong>Reset</strong> table controls cleared");
        }
        show_toast(&document, "Table controls reset");
    }
}

#[cfg(target_arch = "wasm32")]
fn apply_native_filter(workspace: &web_sys::Element, term: &str) {
    let mut visible = 0;
    let mut total = 0;
    if let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            total += 1;
            let haystack = [
                row.get_attribute("data-slskr-native-title"),
                row.get_attribute("data-slskr-native-detail"),
                row.get_attribute("data-slskr-native-meta"),
                row.get_attribute("data-slskr-native-action"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
            let matches = term.is_empty() || haystack.contains(term);
            if matches {
                visible += 1;
                let _ = row.remove_attribute("hidden");
            } else {
                let _ = row.set_attribute("hidden", "");
            }
        }
    }
    set_native_filter_count(workspace, visible, total);
}

#[cfg(target_arch = "wasm32")]
fn update_native_filter_count(workspace: &web_sys::Element) {
    let total = workspace
        .query_selector_all("[data-slskr-native-select]")
        .map(|rows| rows.length())
        .unwrap_or_default();
    set_native_filter_count(workspace, total, total);
}

#[cfg(target_arch = "wasm32")]
fn set_native_filter_count(workspace: &web_sys::Element, visible: u32, total: u32) {
    if let Ok(Some(count)) = workspace.query_selector("[data-slskr-native-count]") {
        count.set_text_content(Some(&format!("{visible} / {total} rows")));
    }
}

#[cfg(target_arch = "wasm32")]
fn select_visible_native_rows(workspace: &web_sys::Element) {
    let mut selected = 0;
    let mut first_selected: Option<web_sys::Element> = None;
    if let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            if row.has_attribute("hidden") {
                continue;
            }
            selected += 1;
            if first_selected.is_none() {
                first_selected = Some(row.clone());
            }
            let _ = row.set_attribute("aria-selected", "true");
            if let Ok(Some(input)) = row.query_selector("input[type=checkbox]") {
                if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                    input.set_checked(true);
                }
            }
        }
    }
    update_native_selection_summary(workspace, selected, "selected", first_selected.as_ref());
    if let Some(row) = first_selected.as_ref() {
        update_native_inspector(workspace, selected, Some(row));
        update_native_selection_preview(workspace, selected, Some(row));
    } else {
        update_native_inspector(workspace, selected, None);
        update_native_selection_preview(workspace, selected, None);
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_native_selection(workspace: &web_sys::Element) {
    if let Ok(rows) = workspace.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = row.remove_attribute("aria-selected");
            if let Ok(Some(input)) = row.query_selector("input[type=checkbox]") {
                if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                    input.set_checked(false);
                }
            }
        }
    }
    update_native_selection_summary(workspace, 0, "selected", None);
    update_native_inspector(workspace, 0, None);
    update_native_selection_preview(workspace, 0, None);
}

#[cfg(target_arch = "wasm32")]
fn update_native_selection_summary(
    workspace: &web_sys::Element,
    selected: u32,
    label: &str,
    row: Option<&web_sys::Element>,
) {
    let Some(document) = workspace.owner_document() else {
        return;
    };
    let message = if selected == 0 {
        "No rows selected".to_string()
    } else if selected == 1 {
        row.and_then(|row| row.get_attribute("data-slskr-native-action-summary"))
            .or_else(|| row.and_then(|row| row.get_attribute("data-slskr-native-title")))
            .unwrap_or_else(|| "1 row selected".to_string())
    } else {
        format!("{selected} visible rows {label}")
    };
    if let Some(status) = document.get_element_by_id("slskr-native-selection-status") {
        status.set_inner_html(&format!(
            "<strong>Selection</strong><span>{}</span>",
            escape_html(&message)
        ));
    }
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>Selection</strong> {}",
            escape_html(&message)
        ));
    }
}

#[cfg(target_arch = "wasm32")]
fn update_native_inspector(
    workspace: &web_sys::Element,
    selected: u32,
    row: Option<&web_sys::Element>,
) {
    let Ok(Some(inspector)) = workspace.query_selector("#slskr-native-inspector") else {
        return;
    };
    let count = if selected == 1 {
        "1 selected".to_string()
    } else {
        format!("{selected} selected")
    };
    if let Ok(Some(element)) = inspector.query_selector("[data-slskr-native-inspector-count]") {
        element.set_text_content(Some(&count));
    }
    let title = row
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Nothing selected".to_string()
            } else {
                "Multiple rows selected".to_string()
            }
        });
    let detail = row
        .and_then(|row| row.get_attribute("data-slskr-native-action-summary"))
        .or_else(|| row.and_then(|row| row.get_attribute("data-slskr-native-detail")))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Use the table to choose an item.".to_string()
            } else {
                "Bulk actions will apply to all selected visible rows.".to_string()
            }
        });
    let meta = row
        .and_then(|row| row.get_attribute("data-slskr-native-meta"))
        .unwrap_or_else(|| count.clone());
    let action = row
        .and_then(|row| row.get_attribute("data-slskr-native-action"))
        .unwrap_or_else(|| "Review".to_string());
    let fields = row
        .and_then(|row| row.get_attribute("data-slskr-native-detail-list"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Selection fields will appear here.".to_string()
            } else {
                "Bulk selection across visible rows.".to_string()
            }
        });
    for (selector, value) in [
        ("[data-slskr-native-inspector-title]", title),
        ("[data-slskr-native-inspector-detail]", detail),
        ("[data-slskr-native-inspector-meta]", meta),
        ("[data-slskr-native-inspector-action]", action),
        ("[data-slskr-native-inspector-fields]", fields),
    ] {
        if let Ok(Some(element)) = inspector.query_selector(selector) {
            element.set_text_content(Some(&value));
        }
    }
    if let Ok(Some(actions)) = inspector.query_selector("[data-slskr-native-inspector-actions]") {
        let menu = row
            .and_then(|row| row.get_attribute("data-slskr-native-action-menu"))
            .unwrap_or_else(|| {
                if selected == 0 {
                    "Review Selection | Queue Selected".to_string()
                } else {
                    "Review Selection | Run Selected".to_string()
                }
            });
        let html = menu
            .split('|')
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(|label| {
                format!(
                    r#"<button type="button" data-slskr-native-row-action="{}">{}</button>"#,
                    escape_html(label),
                    escape_html(label)
                )
            })
            .collect::<Vec<_>>()
            .join("");
        actions.set_inner_html(&html);
    }
}

#[cfg(target_arch = "wasm32")]
fn update_native_selection_preview(
    workspace: &web_sys::Element,
    selected: u32,
    row: Option<&web_sys::Element>,
) {
    let count = if selected == 1 {
        "1 selected".to_string()
    } else {
        format!("{selected} selected")
    };
    let title = row
        .and_then(|row| row.get_attribute("data-slskr-native-title"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Nothing selected".to_string()
            } else {
                "Multiple rows selected".to_string()
            }
        });
    let detail = row
        .and_then(|row| row.get_attribute("data-slskr-native-action-summary"))
        .or_else(|| row.and_then(|row| row.get_attribute("data-slskr-native-detail")))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Choose a row to review actions.".to_string()
            } else {
                "Bulk actions will apply to the selected visible rows.".to_string()
            }
        });
    let meta = row
        .and_then(|row| row.get_attribute("data-slskr-native-meta"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Waiting".to_string()
            } else {
                count.clone()
            }
        });
    let action = row
        .and_then(|row| row.get_attribute("data-slskr-native-action"))
        .unwrap_or_else(|| "Review".to_string());
    let fields = row
        .and_then(|row| row.get_attribute("data-slskr-native-detail-list"))
        .unwrap_or_else(|| {
            if selected == 0 {
                "Selection fields will appear here.".to_string()
            } else {
                "Bulk selection across visible rows.".to_string()
            }
        });
    let title = if selected > 1 {
        format!("{selected} rows selected")
    } else {
        title
    };
    for (selector, value) in [
        ("[data-slskr-native-preview-count]", count),
        ("[data-slskr-native-preview-title]", title),
        ("[data-slskr-native-preview-detail]", detail),
        ("[data-slskr-native-preview-fields]", fields),
        ("[data-slskr-native-preview-meta]", meta),
        ("[data-slskr-native-preview-action]", action),
    ] {
        if let Ok(elements) = workspace.query_selector_all(selector) {
            for index in 0..elements.length() {
                let Some(node) = elements.item(index) else {
                    continue;
                };
                if let Ok(element) = node.dyn_into::<web_sys::Element>() {
                    element.set_text_content(Some(&value));
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_native_sorters(document: &web_sys::Document) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-native-sort]")?;
    for button_index in 0..buttons.length() {
        let Some(node) = buttons.item(button_index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let button_for_click = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                sort_native_table(&button_for_click);
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    restore_native_sort(document);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn sort_native_table(button: &web_sys::Element) {
    let index = button
        .get_attribute("data-slskr-native-sort")
        .unwrap_or_else(|| "0".to_string());
    let next_direction = if button
        .get_attribute("aria-sort")
        .is_some_and(|direction| direction == "ascending")
    {
        "descending"
    } else {
        "ascending"
    };
    apply_native_sort(button, &index, next_direction, true);
}

#[cfg(target_arch = "wasm32")]
fn apply_native_sort(button: &web_sys::Element, index: &str, direction: &str, persist: bool) {
    let Some(table) = button.closest("table").ok().flatten() else {
        return;
    };
    if let Ok(sort_buttons) = table.query_selector_all("[data-slskr-native-sort]") {
        for button_index in 0..sort_buttons.length() {
            let Some(node) = sort_buttons.item(button_index) else {
                continue;
            };
            let Ok(sort_button) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let active = sort_button
                .get_attribute("data-slskr-native-sort")
                .is_some_and(|value| value == index);
            let _ = sort_button.set_attribute("aria-sort", if active { direction } else { "none" });
        }
    }

    let Some(tbody) = table.query_selector("tbody").ok().flatten() else {
        return;
    };
    let Ok(row_nodes) = tbody.query_selector_all("[data-slskr-native-select]") else {
        return;
    };
    let attr = format!("data-slskr-native-sort-{index}");
    let mut rows = Vec::new();
    for row_index in 0..row_nodes.length() {
        let Some(node) = row_nodes.item(row_index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        rows.push(row);
    }
    rows.sort_by(|left, right| {
        let left_value = left.get_attribute(&attr).unwrap_or_default().to_lowercase();
        let right_value = right
            .get_attribute(&attr)
            .unwrap_or_default()
            .to_lowercase();
        if direction == "descending" {
            right_value.cmp(&left_value)
        } else {
            left_value.cmp(&right_value)
        }
    });
    for row in rows {
        let _ = tbody.append_child(&row);
    }
    if persist {
        if let Some(workspace) = table.closest(".slskr-native-workspace").ok().flatten() {
            persist_native_sort(&workspace, index, direction);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn restore_native_sort(document: &web_sys::Document) {
    let Ok(workspaces) = document.query_selector_all(".slskr-native-workspace") else {
        return;
    };
    for workspace_index in 0..workspaces.length() {
        let Some(node) = workspaces.item(workspace_index) else {
            continue;
        };
        let Ok(workspace) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(key) = native_state_key(&workspace, "sort") else {
            continue;
        };
        let Some(storage) = session_storage_for_workspace(&workspace) else {
            continue;
        };
        let Some(value) = storage.get_item(&key).ok().flatten() else {
            continue;
        };
        let Some((index, direction)) = value.split_once(':') else {
            continue;
        };
        if !matches!(direction, "ascending" | "descending") {
            continue;
        }
        let selector = format!(r#"[data-slskr-native-sort="{index}"]"#);
        let Ok(Some(button)) = workspace.query_selector(&selector) else {
            continue;
        };
        apply_native_sort(&button, index, direction, false);
    }
}

#[cfg(target_arch = "wasm32")]
fn persist_native_sort(workspace: &web_sys::Element, index: &str, direction: &str) {
    let Some(key) = native_state_key(workspace, "sort") else {
        return;
    };
    let Some(storage) = session_storage_for_workspace(workspace) else {
        return;
    };
    let _ = storage.set_item(&key, &format!("{index}:{direction}"));
}

#[cfg(target_arch = "wasm32")]
fn reset_native_sort(workspace: &web_sys::Element) {
    if let Some(key) = native_state_key(workspace, "sort") {
        if let Some(storage) = session_storage_for_workspace(workspace) {
            let _ = storage.remove_item(&key);
        }
    }
    if let Ok(sort_buttons) = workspace.query_selector_all("[data-slskr-native-sort]") {
        for button_index in 0..sort_buttons.length() {
            let Some(node) = sort_buttons.item(button_index) else {
                continue;
            };
            let Ok(sort_button) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = sort_button.set_attribute("aria-sort", "none");
        }
    }
    let Ok(tables) = workspace.query_selector_all("table") else {
        return;
    };
    for table_index in 0..tables.length() {
        let Some(table_node) = tables.item(table_index) else {
            continue;
        };
        let Ok(table) = table_node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(tbody) = table.query_selector("tbody").ok().flatten() else {
            continue;
        };
        let Ok(row_nodes) = tbody.query_selector_all("[data-slskr-native-select]") else {
            continue;
        };
        let mut rows = Vec::new();
        for row_index in 0..row_nodes.length() {
            let Some(node) = row_nodes.item(row_index) else {
                continue;
            };
            let Ok(row) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            rows.push(row);
        }
        rows.sort_by_key(|row| {
            row.get_attribute("data-slskr-native-index")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(usize::MAX)
        });
        for row in rows {
            let _ = tbody.append_child(&row);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn show_toast(document: &web_sys::Document, message: &str) {
    let region = document
        .get_element_by_id("slskr-toast-region")
        .or_else(|| {
            let element = document.create_element("div").ok()?;
            element.set_id("slskr-toast-region");
            element.set_class_name("slskr-toast-region");
            let _ = element.set_attribute("aria-live", "polite");
            let body = document.body()?;
            let _ = body.append_child(&element);
            Some(element)
        });
    let Some(region) = region else {
        return;
    };
    region.set_inner_html("");
    let Ok(toast) = document.create_element("div") else {
        return;
    };
    toast.set_class_name("slskr-toast");
    toast.set_text_content(Some(message));
    let _ = region.append_child(&toast);
}

#[cfg(target_arch = "wasm32")]
fn select_native_row(document: &web_sys::Document, row: &web_sys::Element) {
    if let Ok(rows) = document.query_selector_all("[data-slskr-native-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("aria-selected");
        }
    }
    let _ = row.set_attribute("aria-selected", "true");
    if let Ok(Some(input)) = row.query_selector("input[type=checkbox]") {
        if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_checked(true);
        }
    }

    let title = row
        .get_attribute("data-slskr-native-title")
        .unwrap_or_else(|| "Selected row".to_string());
    let detail = row
        .get_attribute("data-slskr-native-action-summary")
        .or_else(|| row.get_attribute("data-slskr-native-detail"))
        .unwrap_or_default();
    let meta = row
        .get_attribute("data-slskr-native-meta")
        .unwrap_or_default();
    let action = row
        .get_attribute("data-slskr-native-action")
        .unwrap_or_else(|| "Review".to_string());
    let message = format!(
        "<strong>{}</strong><span>{}</span><span>{}</span><button type=\"button\">{}</button>",
        escape_html(&title),
        escape_html(&detail),
        escape_html(&meta),
        escape_html(&action)
    );
    if let Some(status) = document.get_element_by_id("slskr-native-selection-status") {
        status.set_inner_html(&message);
    }
    if let Some(status) = document.get_element_by_id("slskr-action-status") {
        status.set_inner_html(&format!(
            "<strong>Selected</strong> {}",
            escape_html(&title)
        ));
    }
    if let Some(workspace) = row.closest(".slskr-native-workspace").ok().flatten() {
        update_native_inspector(&workspace, 1, Some(row));
        update_native_selection_preview(&workspace, 1, Some(row));
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchCandidateRank {
    pub reasons: Vec<String>,
    pub score: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDuplicateGroup {
    pub candidate_count: usize,
    pub folded_count: usize,
    pub key: String,
    pub providers: Vec<String>,
    pub usernames: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchActionPreview {
    pub candidate_score: Option<u32>,
    pub file_count: usize,
    pub filenames: Vec<String>,
    pub locked_count: usize,
    pub provider_labels: Vec<String>,
    pub route: String,
    pub total_size_bytes: u64,
    pub username: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExperiencePreference {
    pub default_value: &'static str,
    pub group: &'static str,
    pub id: &'static str,
    pub input: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutomationRecipe {
    pub approval_gate: &'static str,
    pub cadence: &'static str,
    pub cooldown: &'static str,
    pub description: &'static str,
    pub enabled_by_default: bool,
    pub file_impact: &'static str,
    pub id: &'static str,
    pub max_run_time: &'static str,
    pub network_impact: &'static str,
    pub title: &'static str,
}

pub const fn experience_preferences() -> &'static [ExperiencePreference] {
    &[
        ExperiencePreference {
            default_value: "balanced",
            group: "Search",
            id: "searchRankingProfile",
            input: "text",
            label: "Ranking Profile",
        },
        ExperiencePreference {
            default_value: "lossless",
            group: "Search",
            id: "searchPreferredCondition",
            input: "text",
            label: "Preferred Condition",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Search",
            id: "searchDuplicateFolding",
            input: "checkbox",
            label: "Fold duplicate results",
        },
        ExperiencePreference {
            default_value: "detailed",
            group: "Search",
            id: "searchActionPreviewDensity",
            input: "text",
            label: "Action Preview Density",
        },
        ExperiencePreference {
            default_value: "current",
            group: "Player",
            id: "playerRadioSeedMode",
            input: "text",
            label: "Radio Seed",
        },
        ExperiencePreference {
            default_value: "manual",
            group: "Player",
            id: "playerScrobbleMode",
            input: "text",
            label: "Scrobble Mode",
        },
        ExperiencePreference {
            default_value: "last",
            group: "Player",
            id: "playerDefaultVisualizer",
            input: "text",
            label: "Default Visualizer",
        },
        ExperiencePreference {
            default_value: "false",
            group: "Player",
            id: "playerQueueAutoFill",
            input: "checkbox",
            label: "Enable queue auto-fill",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Player",
            id: "playerShowRatings",
            input: "checkbox",
            label: "Show ratings",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Player",
            id: "playerCaptureHistory",
            input: "checkbox",
            label: "Capture history",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Player",
            id: "playerKeyboardShortcuts",
            input: "checkbox",
            label: "Keyboard shortcuts",
        },
        ExperiencePreference {
            default_value: "all",
            group: "Discovery",
            id: "discoveryApprovalFilter",
            input: "text",
            label: "Approval Filter",
        },
        ExperiencePreference {
            default_value: "0.70",
            group: "Discovery",
            id: "discoveryConfidenceFloor",
            input: "text",
            label: "Confidence Floor",
        },
        ExperiencePreference {
            default_value: "14",
            group: "Discovery",
            id: "discoveryStaleDays",
            input: "text",
            label: "Stale Days",
        },
        ExperiencePreference {
            default_value: "false",
            group: "Messages",
            id: "messagesDenseMode",
            input: "checkbox",
            label: "Dense mode",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Messages",
            id: "messagesPinnedRestore",
            input: "checkbox",
            label: "Restore pinned conversations",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Messages",
            id: "messagesUnreadBadges",
            input: "checkbox",
            label: "Unread badges",
        },
        ExperiencePreference {
            default_value: "true",
            group: "Messages",
            id: "messagesSearchEnabled",
            input: "checkbox",
            label: "Message search",
        },
    ]
}

pub const fn automation_recipes() -> &'static [AutomationRecipe] {
    &[
        AutomationRecipe {
            approval_gate: "None required",
            cadence: "Continuous",
            cooldown: "5 minutes",
            description: "Checks connection, shares, paths, and credentials for setup drift.",
            enabled_by_default: true,
            file_impact: "Read only",
            id: "local-diagnostics",
            max_run_time: "30 seconds",
            network_impact: "Local",
            title: "Local Diagnostics",
        },
        AutomationRecipe {
            approval_gate: "None required",
            cadence: "Daily",
            cooldown: "24 hours",
            description:
                "Surfaces stale share-cache and library-scan reminders before users hit missing results.",
            enabled_by_default: true,
            file_impact: "Read only",
            id: "stale-cache-reminders",
            max_run_time: "1 minute",
            network_impact: "Local",
            title: "Share and Library Reminders",
        },
        AutomationRecipe {
            approval_gate: "None required",
            cadence: "Every 15 minutes",
            cooldown: "15 minutes",
            description: "Keeps local dashboard summaries fresh without contacting public peers.",
            enabled_by_default: true,
            file_impact: "Read only",
            id: "dashboard-refresh",
            max_run_time: "20 seconds",
            network_impact: "Local",
            title: "Dashboard Refresh",
        },
        AutomationRecipe {
            approval_gate: "Download approval",
            cadence: "Manual or scheduled",
            cooldown: "2 hours",
            description: "Retries failed Wishlist items using the selected acquisition profile.",
            enabled_by_default: false,
            file_impact: "Downloads after approval",
            id: "wishlist-retry",
            max_run_time: "20 minutes",
            network_impact: "Public peers possible",
            title: "Wishlist Retry",
        },
        AutomationRecipe {
            approval_gate: "Fix confirmation",
            cadence: "Manual or scheduled",
            cooldown: "24 hours",
            description: "Finds duplicates, dead files, metadata gaps, fake lossless files, and missing art.",
            enabled_by_default: false,
            file_impact: "Read only until fixed",
            id: "library-health-scan",
            max_run_time: "30 minutes",
            network_impact: "Local",
            title: "Library Health Scan",
        },
        AutomationRecipe {
            approval_gate: "Configured import success",
            cadence: "After import",
            cooldown: "10 minutes",
            description: "Asks configured media servers to rescan after successful library imports.",
            enabled_by_default: false,
            file_impact: "Media-server scan",
            id: "media-server-rescan",
            max_run_time: "2 minutes",
            network_impact: "Local network",
            title: "Media Server Rescan",
        },
        AutomationRecipe {
            approval_gate: "Explicit evidence publication opt-in",
            cadence: "Manual or scheduled",
            cooldown: "12 hours",
            description:
                "Publishes explicit opt-in signed quality and verification evidence to trusted mesh peers.",
            enabled_by_default: false,
            file_impact: "No file writes",
            id: "mesh-evidence-publish",
            max_run_time: "10 minutes",
            network_impact: "Trusted mesh",
            title: "Mesh Evidence Publish",
        },
    ]
}

pub fn default_experience_preferences() -> serde_json::Map<String, serde_json::Value> {
    experience_preferences()
        .iter()
        .map(|preference| {
            let value = if preference.input == "checkbox" {
                serde_json::Value::Bool(preference.default_value == "true")
            } else {
                serde_json::Value::String(preference.default_value.to_string())
            };
            (preference.id.to_string(), value)
        })
        .collect()
}

fn preference_string(
    values: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: &str,
) -> String {
    values
        .get(key)
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn preference_bool(
    values: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: bool,
) -> bool {
    values
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(fallback)
}

pub fn experience_preferences_report(
    values: &serde_json::Map<String, serde_json::Value>,
) -> String {
    [
        "slskr experience preferences".to_string(),
        format!(
            "Search: ranking={}, condition={}, duplicate_folding={}, previews={}",
            preference_string(values, "searchRankingProfile", "balanced"),
            preference_string(values, "searchPreferredCondition", "lossless"),
            preference_bool(values, "searchDuplicateFolding", true),
            preference_string(values, "searchActionPreviewDensity", "detailed")
        ),
        format!(
            "Discovery: approval={}, confidence>={}, stale_days={}",
            preference_string(values, "discoveryApprovalFilter", "all"),
            preference_string(values, "discoveryConfidenceFloor", "0.70"),
            preference_string(values, "discoveryStaleDays", "14")
        ),
        format!(
            "Player: queue_auto_fill={}, radio_seed={}, ratings={}, history={}, scrobble={}, visualizer={}, shortcuts={}",
            preference_bool(values, "playerQueueAutoFill", false),
            preference_string(values, "playerRadioSeedMode", "current"),
            preference_bool(values, "playerShowRatings", true),
            preference_bool(values, "playerCaptureHistory", true),
            preference_string(values, "playerScrobbleMode", "manual"),
            preference_string(values, "playerDefaultVisualizer", "last"),
            preference_bool(values, "playerKeyboardShortcuts", true)
        ),
        format!(
            "Messages: dense={}, pinned_restore={}, unread_badges={}, search={}",
            preference_bool(values, "messagesDenseMode", false),
            preference_bool(values, "messagesPinnedRestore", true),
            preference_bool(values, "messagesUnreadBadges", true),
            preference_bool(values, "messagesSearchEnabled", true)
        ),
    ]
    .join("\n")
}

pub fn automation_summary_from_state(
    state: &serde_json::Map<String, serde_json::Value>,
) -> (usize, usize, usize) {
    let enabled = automation_recipes()
        .iter()
        .filter(|recipe| {
            state
                .get(recipe.id)
                .and_then(|entry| entry.get("enabled"))
                .and_then(|value| value.as_bool())
                .unwrap_or(recipe.enabled_by_default)
        })
        .count();
    let total = automation_recipes().len();
    (total, enabled, total.saturating_sub(enabled))
}

pub fn automation_dry_run_report(recipe: AutomationRecipe, timestamp: &str) -> serde_json::Value {
    serde_json::json!({
        "approvalGate": recipe.approval_gate,
        "cooldown": recipe.cooldown,
        "executed": false,
        "fileImpact": recipe.file_impact,
        "generatedAt": timestamp,
        "maxRunTime": recipe.max_run_time,
        "networkImpact": recipe.network_impact,
        "recipeId": recipe.id,
        "title": recipe.title,
    })
}

pub fn automation_history_report(state: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut entries = Vec::new();
    for recipe in automation_recipes() {
        let stored = state.get(recipe.id);
        let enabled = stored
            .and_then(|entry| entry.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(recipe.enabled_by_default);
        let last_dry_run = stored
            .and_then(|entry| entry.get("lastDryRunAt"))
            .map(json_scalar_preview)
            .filter(|value| !value.is_empty());
        let last_run = stored
            .and_then(|entry| entry.get("lastRunAt"))
            .map(json_scalar_preview)
            .filter(|value| !value.is_empty());
        if enabled || last_dry_run.is_some() || last_run.is_some() {
            entries.push((recipe, enabled, last_dry_run, last_run));
        }
    }

    let mut lines = vec![
        "slskr automation review history".to_string(),
        format!("Entries: {}", entries.len()),
        String::new(),
    ];
    if entries.is_empty() {
        lines.push("No enabled recipes or dry-run checkpoints.".to_string());
        return lines.join("\n");
    }
    for (recipe, enabled, last_dry_run, last_run) in entries {
        lines.push(format!("- {}", recipe.title));
        lines.push(format!("  Enabled: {}", if enabled { "yes" } else { "no" }));
        lines.push(format!(
            "  Last run: {}",
            last_run.unwrap_or_else(|| "not recorded".to_string())
        ));
        lines.push(format!(
            "  Last dry run: {}",
            last_dry_run.unwrap_or_else(|| "not recorded".to_string())
        ));
        lines.push(format!("  Network impact: {}", recipe.network_impact));
        lines.push(format!("  File impact: {}", recipe.file_impact));
    }
    lines.join("\n")
}

fn file_extension(filename: &str) -> String {
    filename
        .rsplit(['/', '\\'])
        .next()
        .and_then(|name| name.rsplit_once('.').map(|(_, extension)| extension))
        .unwrap_or_default()
        .to_lowercase()
}

fn search_response_files(response: &serde_json::Value) -> Vec<serde_json::Value> {
    ["files", "lockedFiles", "locked_files"]
        .iter()
        .flat_map(|key| {
            response
                .get(*key)
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

fn search_file_name(file: &serde_json::Value) -> String {
    json_track_field(file, &["filename", "fileName", "path"])
}

fn search_file_size(file: &serde_json::Value) -> u64 {
    file.get("size")
        .or_else(|| file.get("bytes"))
        .and_then(|value| value.as_u64())
        .unwrap_or_default()
}

fn search_file_number(file: &serde_json::Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|key| file.get(*key).and_then(|value| value.as_u64()))
        .unwrap_or_default()
}

fn search_tokens(value: &str) -> Vec<String> {
    unique_nonempty(
        value
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
            .collect(),
    )
}

fn is_lossless_extension(extension: &str) -> bool {
    matches!(
        extension,
        "aif" | "aiff" | "alac" | "ape" | "flac" | "wav" | "wv"
    )
}

fn is_lossy_extension(extension: &str) -> bool {
    matches!(extension, "aac" | "m4a" | "mp3" | "ogg" | "opus" | "wma")
}

fn is_artwork_extension(extension: &str) -> bool {
    matches!(extension, "gif" | "jpeg" | "jpg" | "png" | "webp")
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

fn push_unique_reason(reasons: &mut Vec<String>, reason: impl Into<String>) {
    let reason = reason.into();
    if !reason.is_empty() && !reasons.iter().any(|item| item == &reason) {
        reasons.push(reason);
    }
}

pub fn rank_search_candidate(
    response: &serde_json::Value,
    search_text: &str,
    acquisition_profile: &str,
    download_stats: Option<&serde_json::Value>,
    community_quality_summary: Option<&serde_json::Value>,
    preferred_conditions: Option<&serde_json::Value>,
) -> SearchCandidateRank {
    let files = search_response_files(response);
    let mut reasons = Vec::new();
    let mut score: i64 = 0;

    let tokens = search_tokens(search_text);
    if !tokens.is_empty() && !files.is_empty() {
        let best = files
            .iter()
            .map(|file| {
                let filename = search_file_name(file).to_lowercase();
                let matched = tokens
                    .iter()
                    .filter(|token| filename.contains(token.as_str()))
                    .count();
                matched as f64 / tokens.len() as f64
            })
            .fold(0.0_f64, f64::max);
        score += (best * 18.0).round() as i64;
        if best >= 0.8 {
            push_unique_reason(&mut reasons, "strong filename match");
        } else if best >= 0.45 {
            push_unique_reason(&mut reasons, "partial filename match");
        } else {
            push_unique_reason(&mut reasons, "weak filename match");
        }
    }

    let media_files = files
        .iter()
        .filter(|file| !is_artwork_extension(&file_extension(&search_file_name(file))))
        .collect::<Vec<_>>();
    if media_files.is_empty() {
        push_unique_reason(&mut reasons, "no media files visible");
    } else {
        let lossless_count = media_files
            .iter()
            .filter(|file| {
                is_lossless_extension(&file_extension(&search_file_name(file)))
                    || (file.get("bitDepth").is_some() && file.get("sampleRate").is_some())
            })
            .count();
        let high_bitrate_lossy_count = media_files
            .iter()
            .filter(|file| {
                is_lossy_extension(&file_extension(&search_file_name(file)))
                    && search_file_number(file, &["bitRate", "bitrate"]) >= 256
            })
            .count();
        let lossless_ratio = lossless_count as f64 / media_files.len() as f64;
        let high_bitrate_ratio = high_bitrate_lossy_count as f64 / media_files.len() as f64;
        match acquisition_profile {
            "fast-good-enough" => {
                score += (lossless_ratio * 12.0 + high_bitrate_ratio * 16.0).round() as i64;
                if lossless_count > 0 {
                    push_unique_reason(&mut reasons, "lossless fast-good-enough candidate");
                } else if high_bitrate_lossy_count > 0 {
                    push_unique_reason(&mut reasons, "high bitrate fast-good-enough candidate");
                } else {
                    push_unique_reason(&mut reasons, "limited fast-good-enough quality evidence");
                }
            }
            "album-complete" => {
                score += (lossless_ratio * 14.0).round() as i64
                    + clamp_i64(media_files.len() as i64, 0, 18);
                push_unique_reason(
                    &mut reasons,
                    if media_files.len() >= 8 {
                        "broad folder candidate"
                    } else {
                        "small folder candidate"
                    },
                );
            }
            _ => {
                score += (lossless_ratio * 28.0 + high_bitrate_ratio * 6.0).round() as i64;
                if lossless_ratio >= 0.8 {
                    push_unique_reason(&mut reasons, "mostly lossless files");
                } else if lossless_ratio > 0.0 {
                    push_unique_reason(&mut reasons, "mixed lossless files");
                } else {
                    push_unique_reason(&mut reasons, "no lossless signal");
                }
            }
        }
    }

    let audio_files = files
        .iter()
        .filter(|file| {
            let extension = file_extension(&search_file_name(file));
            is_lossless_extension(&extension) || is_lossy_extension(&extension)
        })
        .collect::<Vec<_>>();
    if !audio_files.is_empty() {
        let plausible = audio_files
            .iter()
            .filter(|file| {
                let size = search_file_size(file);
                let length = search_file_number(file, &["length", "duration"]);
                let extension = file_extension(&search_file_name(file));
                if is_lossless_extension(&extension) {
                    (8_000_000..=250_000_000).contains(&size)
                } else if length > 0 {
                    size >= (length * 8).min(2_000_000) && size <= 80_000_000
                } else {
                    (1_000_000..=80_000_000).contains(&size)
                }
            })
            .count();
        let ratio = plausible as f64 / audio_files.len() as f64;
        score += (ratio * 9.0).round() as i64;
        push_unique_reason(
            &mut reasons,
            if ratio >= 0.8 {
                "plausible file sizes"
            } else {
                "mixed file size evidence"
            },
        );
    }

    if response
        .get("hasFreeUploadSlot")
        .or_else(|| response.get("has_free_upload_slot"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        score += 12;
        push_unique_reason(&mut reasons, "free upload slot");
    } else {
        push_unique_reason(&mut reasons, "queued upload");
    }
    let queue_length = response
        .get("queueLength")
        .or_else(|| response.get("queue_length"))
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    score += clamp_i64(10 - queue_length * 2, 0, 10);
    if queue_length <= 1 {
        push_unique_reason(&mut reasons, "short queue");
    } else if queue_length >= 5 {
        push_unique_reason(&mut reasons, "long queue");
    }
    let upload_speed = response
        .get("uploadSpeed")
        .or_else(|| response.get("upload_speed"))
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    score += clamp_i64(
        ((upload_speed as f64 / 5_242_880.0) * 10.0).round() as i64,
        0,
        10,
    );
    if upload_speed >= 2_097_152 {
        push_unique_reason(&mut reasons, "fast peer");
    }

    if let Some(stats) = download_stats {
        let successes = stats
            .get("successfulDownloads")
            .and_then(|value| value.as_i64())
            .unwrap_or_default();
        let failures = stats
            .get("failedDownloads")
            .and_then(|value| value.as_i64())
            .unwrap_or_default();
        let history_points = clamp_i64(successes * 2 - failures * 3, -9, 10);
        score += history_points;
        if history_points >= 5 {
            push_unique_reason(&mut reasons, "trusted download history");
        } else if history_points < 0 {
            push_unique_reason(&mut reasons, "poor download history");
        } else {
            push_unique_reason(&mut reasons, "limited download history");
        }
    }

    if response
        .get("sourceProviders")
        .and_then(|value| value.as_array())
        .is_some_and(|providers| providers.iter().any(|value| value == "local"))
    {
        score += 8;
        push_unique_reason(&mut reasons, "local source available");
    } else if response
        .get("sourceProviders")
        .and_then(|value| value.as_array())
        .is_some_and(|providers| {
            providers
                .iter()
                .any(|value| value == "mesh" || value == "pod")
        })
    {
        score += 5;
        push_unique_reason(&mut reasons, "mesh source available");
    }

    if let Some(summary) = community_quality_summary {
        let quality_score = summary
            .get("score")
            .and_then(|value| value.as_i64())
            .unwrap_or_default();
        let override_mode = summary
            .get("override")
            .and_then(|value| value.get("mode"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        match override_mode {
            "ignore" => push_unique_reason(&mut reasons, "local quality signals ignored"),
            "trust" => {
                score += 8;
                push_unique_reason(&mut reasons, "local trust override");
            }
            "caution" => {
                score -= 6;
                push_unique_reason(&mut reasons, "local caution override");
            }
            _ if quality_score >= 8 => {
                score += quality_score.min(10);
                push_unique_reason(&mut reasons, "positive local quality signals");
            }
            _ if quality_score <= -6 => {
                score += quality_score.max(-15);
                push_unique_reason(&mut reasons, "local caution signals");
            }
            _ if quality_score != 0 => {
                score += quality_score;
                push_unique_reason(&mut reasons, "mixed local quality signals");
            }
            _ => {}
        }
    }

    if let Some(preferred) = preferred_conditions {
        if preferred
            .get("preferLossless")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            && !files.is_empty()
        {
            let lossless = files
                .iter()
                .filter(|file| is_lossless_extension(&file_extension(&search_file_name(file))))
                .count();
            if lossless > 0 {
                score += ((lossless as f64 / files.len() as f64) * 12.0)
                    .round()
                    .min(12.0) as i64;
                push_unique_reason(&mut reasons, "preferred lossless match");
            } else {
                score -= 6;
                push_unique_reason(&mut reasons, "missing preferred lossless files");
            }
        }
        if let Some(extensions) = preferred
            .get("preferExtensions")
            .and_then(|value| value.as_array())
        {
            if !extensions.is_empty() && !files.is_empty() {
                let matching = files
                    .iter()
                    .filter(|file| {
                        let extension = file_extension(&search_file_name(file));
                        extensions
                            .iter()
                            .any(|value| value.as_str() == Some(extension.as_str()))
                    })
                    .count();
                if matching > 0 {
                    score += ((matching as f64 / files.len() as f64) * 10.0)
                        .round()
                        .min(10.0) as i64;
                    push_unique_reason(&mut reasons, "preferred extension match");
                } else {
                    score -= 4;
                    push_unique_reason(&mut reasons, "missing preferred extension");
                }
            }
        }
        let min_bitrate = preferred
            .get("preferMinBitRate")
            .and_then(|value| value.as_u64())
            .unwrap_or_default();
        if min_bitrate > 0 && !files.is_empty() {
            let matching = files
                .iter()
                .filter(|file| search_file_number(file, &["bitRate", "bitrate"]) >= min_bitrate)
                .count();
            if matching > 0 {
                score += ((matching as f64 / files.len() as f64) * 8.0)
                    .round()
                    .min(8.0) as i64;
                push_unique_reason(&mut reasons, "preferred bitrate match");
            } else {
                score -= 3;
                push_unique_reason(&mut reasons, "below preferred bitrate");
            }
        }
    }

    reasons.truncate(9);
    SearchCandidateRank {
        reasons,
        score: clamp_i64(score, 0, 100) as u32,
    }
}

fn search_provider_labels(response: &serde_json::Value) -> Vec<String> {
    let mut providers = response
        .get("sourceProviders")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(primary) = response
        .get("primarySource")
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        providers.push(primary);
    }
    if providers.is_empty() {
        providers.push("soulseek".to_string());
    }
    providers.sort();
    providers.dedup();
    providers
}

fn search_response_signature(response: &serde_json::Value) -> String {
    let mut media = search_response_files(response)
        .iter()
        .filter_map(|file| {
            let filename = search_file_name(file);
            let extension = file_extension(&filename);
            if !(is_lossless_extension(&extension) || is_lossy_extension(&extension)) {
                return None;
            }
            let basename = filename
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(filename.as_str())
                .to_lowercase();
            Some((basename, search_file_size(file)))
        })
        .collect::<Vec<_>>();
    if media.is_empty() {
        return String::new();
    }
    media.sort_by(|left, right| left.0.cmp(&right.0));
    let total_size = media.iter().map(|(_, size)| *size).sum::<u64>();
    let mut parts = vec![
        media.len().to_string(),
        ((total_size as f64 / 1_000_000.0).round() as u64).to_string(),
    ];
    parts.extend(
        media
            .iter()
            .take(20)
            .map(|(name, size)| format!("{name}:{}", (size + 5_000) / 10_000)),
    );
    parts.join("|")
}

pub fn deduplicate_search_response_groups(
    responses: &[serde_json::Value],
    enabled: bool,
) -> (usize, Vec<SearchDuplicateGroup>) {
    if !enabled || responses.is_empty() {
        return (0, Vec::new());
    }
    let mut groups = std::collections::BTreeMap::<String, Vec<&serde_json::Value>>::new();
    for response in responses {
        let key = search_response_signature(response);
        if !key.is_empty() {
            groups.entry(key).or_default().push(response);
        }
    }
    let mut folded = 0;
    let duplicate_groups = groups
        .into_iter()
        .filter_map(|(key, group)| {
            if group.len() <= 1 {
                return None;
            }
            folded += group.len() - 1;
            let providers = unique_nonempty_case_insensitive(
                group
                    .iter()
                    .flat_map(|response| search_provider_labels(response))
                    .collect(),
            );
            let mut usernames = unique_nonempty_case_insensitive(
                group
                    .iter()
                    .map(|response| json_track_field(response, &["username"]))
                    .collect(),
            );
            usernames.sort();
            Some(SearchDuplicateGroup {
                candidate_count: group.len(),
                folded_count: group.len() - 1,
                key,
                providers,
                usernames,
            })
        })
        .collect::<Vec<_>>();
    (folded, duplicate_groups)
}

pub fn build_search_action_preview(
    response: &serde_json::Value,
    files: &[serde_json::Value],
    candidate_rank: Option<&SearchCandidateRank>,
    community_quality_summary: Option<&serde_json::Value>,
    route: &str,
) -> SearchActionPreview {
    let locked_count = files
        .iter()
        .filter(|file| {
            file.get("locked")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .count();
    let mut warnings = Vec::new();
    if locked_count > 0 {
        push_unique_reason(
            &mut warnings,
            format!(
                "{locked_count} selected file{} may be locked",
                if locked_count == 1 { "" } else { "s" }
            ),
        );
    }
    if !response
        .get("hasFreeUploadSlot")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        push_unique_reason(&mut warnings, "No free upload slot is currently advertised");
    }
    let queue_length = response
        .get("queueLength")
        .and_then(|value| value.as_u64())
        .unwrap_or_default();
    if queue_length >= 5 {
        push_unique_reason(&mut warnings, format!("Queue depth is {queue_length}"));
    }
    if community_quality_summary
        .and_then(|summary| summary.get("score"))
        .and_then(|value| value.as_i64())
        .unwrap_or_default()
        <= -6
    {
        push_unique_reason(&mut warnings, "Local caution signals exist for this peer");
    }
    if let Some(note) = community_quality_summary
        .and_then(|summary| summary.get("override"))
        .and_then(|value| value.get("note"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        push_unique_reason(&mut warnings, format!("Local quality note: {note}"));
    }
    if community_quality_summary
        .and_then(|summary| summary.get("override"))
        .and_then(|value| value.get("mode"))
        .and_then(|value| value.as_str())
        == Some("ignore")
    {
        push_unique_reason(
            &mut warnings,
            "Local quality signals are ignored by reviewer override",
        );
    }
    if let Some(rank) = candidate_rank {
        if rank.score > 0 && rank.score < 45 {
            push_unique_reason(
                &mut warnings,
                format!("Candidate score is {}/100", rank.score),
            );
        }
    }
    SearchActionPreview {
        candidate_score: candidate_rank.map(|rank| rank.score),
        file_count: files.len(),
        filenames: files.iter().map(search_file_name).collect(),
        locked_count,
        provider_labels: search_provider_labels(response),
        route: route.to_string(),
        total_size_bytes: files.iter().map(search_file_size).sum(),
        username: json_track_field(response, &["username"]),
        warnings,
    }
}

pub fn format_search_action_preview(preview: &SearchActionPreview) -> String {
    let mut lines = vec![
        format!("Action: {}", preview.route),
        format!(
            "Source: {}",
            if preview.username.is_empty() {
                "unknown"
            } else {
                &preview.username
            }
        ),
        format!("Providers: {}", preview.provider_labels.join(", ")),
        format!("Files: {}", preview.file_count),
        format!("Total bytes: {}", preview.total_size_bytes),
    ];
    if let Some(score) = preview.candidate_score {
        lines.push(format!("Candidate score: {score}/100"));
    }
    if !preview.warnings.is_empty() {
        lines.push("Warnings:".to_string());
        lines.extend(
            preview
                .warnings
                .iter()
                .map(|warning| format!("- {warning}")),
        );
    }
    lines.push("Selected files:".to_string());
    lines.extend(
        preview
            .filenames
            .iter()
            .map(|filename| format!("- {filename}")),
    );
    lines.join("\n")
}

pub fn search_planner_report(
    search_text: &str,
    acquisition_profile: &str,
    fold_duplicates: bool,
) -> String {
    let response = serde_json::json!({
        "files": [
            {
                "bitDepth": 16,
                "filename": "Archive Artist/Open Sessions/01 Public Domain Theme.flac",
                "sampleRate": 44100,
                "size": 24000000
            }
        ],
        "hasFreeUploadSlot": true,
        "queueLength": 0,
        "sourceProviders": ["soulseek"],
        "uploadSpeed": 4000000,
        "username": "archive-peer"
    });
    let rank = rank_search_candidate(
        &response,
        search_text,
        acquisition_profile,
        Some(&serde_json::json!({"successfulDownloads": 2, "failedDownloads": 0})),
        None,
        Some(&serde_json::json!({"preferLossless": acquisition_profile == "lossless-exact"})),
    );
    let preview = build_search_action_preview(
        &response,
        &search_response_files(&response),
        Some(&rank),
        None,
        "download",
    );
    let (folded, _) = deduplicate_search_response_groups(&[response], fold_duplicates);
    format!(
        "Search planner\nQuery: {}\nProfile: {}\nDuplicate folding: {}\nFolded duplicates: {}\nScore: {}/100\nReasons: {}\n\n{}",
        search_text,
        acquisition_profile,
        fold_duplicates,
        folded,
        rank.score,
        rank.reasons.join(", "),
        format_search_action_preview(&preview)
    )
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
                    if action == "visualizer" {
                        match toggle_rust_milkdrop_visualizer(&window, &document) {
                            Ok(()) => {
                                set_player_status(&document, "Rust MilkDrop visualizer ready")
                            }
                            Err(error) => set_player_status(
                                &document,
                                &error.as_string().unwrap_or_else(|| {
                                    "Rust MilkDrop visualizer failed".to_string()
                                }),
                            ),
                        }
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
fn storage_json_object(
    window: &web_sys::Window,
    key: &str,
    fallback: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(key).ok().flatten())
        .and_then(|body| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body).ok()
        })
        .map(|stored| {
            let mut merged = fallback.clone();
            for (key, value) in stored {
                merged.insert(key, value);
            }
            merged
        })
        .unwrap_or(fallback)
}

#[cfg(target_arch = "wasm32")]
fn write_storage_json_object(
    window: &web_sys::Window,
    key: &str,
    value: &serde_json::Map<String, serde_json::Value>,
) {
    if let Some(storage) = window.local_storage().ok().flatten() {
        let _ = storage.set_item(key, &serde_json::Value::Object(value.clone()).to_string());
    }
}

#[cfg(target_arch = "wasm32")]
fn remove_storage_item(window: &web_sys::Window, key: &str) {
    if let Some(storage) = window.local_storage().ok().flatten() {
        let _ = storage.remove_item(key);
    }
}

#[cfg(target_arch = "wasm32")]
fn collect_experience_form(
    document: &web_sys::Document,
) -> serde_json::Map<String, serde_json::Value> {
    let mut values = serde_json::Map::new();
    if let Ok(inputs) = document.query_selector_all("[data-slskr-pref]") {
        for index in 0..inputs.length() {
            let Some(node) = inputs.item(index) else {
                continue;
            };
            let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            let Some(key) = input.get_attribute("data-slskr-pref") else {
                continue;
            };
            let value = if input.type_() == "checkbox" {
                serde_json::Value::Bool(input.checked())
            } else {
                serde_json::Value::String(input.value())
            };
            values.insert(key, value);
        }
    }
    values
}

#[cfg(target_arch = "wasm32")]
fn apply_experience_form(
    document: &web_sys::Document,
    values: &serde_json::Map<String, serde_json::Value>,
) {
    if let Ok(inputs) = document.query_selector_all("[data-slskr-pref]") {
        for index in 0..inputs.length() {
            let Some(node) = inputs.item(index) else {
                continue;
            };
            let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            let Some(key) = input.get_attribute("data-slskr-pref") else {
                continue;
            };
            let value = values.get(&key).cloned().unwrap_or_else(|| {
                serde_json::Value::String(
                    input
                        .get_attribute("data-slskr-pref-default")
                        .unwrap_or_default(),
                )
            });
            if input.type_() == "checkbox" {
                input.set_checked(value.as_bool().unwrap_or(false));
            } else {
                input.set_value(&json_scalar_preview(&value));
            }
        }
    }
    let report = experience_preferences_report(values);
    if let Some(output) = document.get_element_by_id("slskr-experience-report") {
        output.set_text_content(Some(&report));
    }
    if let Some(summary) = document.get_element_by_id("slskr-experience-summary") {
        summary.set_text_content(Some("18 preferences"));
    }
}

#[cfg(target_arch = "wasm32")]
fn default_automation_state() -> serde_json::Map<String, serde_json::Value> {
    automation_recipes()
        .iter()
        .map(|recipe| {
            (
                recipe.id.to_string(),
                serde_json::json!({
                    "enabled": recipe.enabled_by_default,
                    "lastDryRunAt": null,
                }),
            )
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn apply_automation_state(
    document: &web_sys::Document,
    state: &serde_json::Map<String, serde_json::Value>,
) {
    for recipe in automation_recipes() {
        let enabled = state
            .get(recipe.id)
            .and_then(|entry| entry.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(recipe.enabled_by_default);
        let selector = format!(r#"[data-slskr-recipe-enabled="{}"]"#, recipe.id);
        if let Ok(Some(input)) = document.query_selector(&selector) {
            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                input.set_checked(enabled);
            }
        }
    }
    let (total, enabled, disabled) = automation_summary_from_state(state);
    if let Some(summary) = document.get_element_by_id("slskr-automation-summary") {
        summary.set_text_content(Some(&format!(
            "{total} recipes / {enabled} enabled / {disabled} disabled"
        )));
    }
    if let Some(report) = document.get_element_by_id("slskr-automation-report") {
        report.set_text_content(Some(&automation_history_report(state)));
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_browser_local_panels(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    if document
        .query_selector("[data-slskr-experience-panel]")?
        .is_some()
    {
        let values = storage_json_object(
            window,
            "slskr:experience-preferences:v1",
            default_experience_preferences(),
        );
        apply_experience_form(document, &values);
        let buttons = document.query_selector_all("[data-slskr-pref-action]")?;
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-pref-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let values = if action == "reset" {
                        remove_storage_item(&window, "slskr:experience-preferences:v1");
                        default_experience_preferences()
                    } else {
                        collect_experience_form(&document)
                    };
                    if action == "save" {
                        write_storage_json_object(
                            &window,
                            "slskr:experience-preferences:v1",
                            &values,
                        );
                    }
                    apply_experience_form(&document, &values);
                    if let Some(status) = document.get_element_by_id("slskr-experience-status") {
                        let message = match action.as_str() {
                            "copy" => "Experience preference report prepared.",
                            "reset" => "Experience preferences reset.",
                            _ => "Experience preferences saved locally.",
                        };
                        status.set_text_content(Some(message));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    if document
        .query_selector("[data-slskr-automation-panel]")?
        .is_some()
    {
        let state = storage_json_object(
            window,
            "slskr.automationRecipeState",
            default_automation_state(),
        );
        apply_automation_state(document, &state);
        let enabled_inputs = document.query_selector_all("[data-slskr-recipe-enabled]")?;
        for index in 0..enabled_inputs.length() {
            let Some(node) = enabled_inputs.item(index) else {
                continue;
            };
            let input: web_sys::HtmlInputElement = node.dyn_into()?;
            let recipe_id = input
                .get_attribute("data-slskr-recipe-enabled")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(
                move |event: web_sys::Event| {
                    let checked = event
                        .current_target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.checked())
                        .unwrap_or(false);
                    let mut state = storage_json_object(
                        &window,
                        "slskr.automationRecipeState",
                        default_automation_state(),
                    );
                    let entry = state
                        .entry(recipe_id.clone())
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(object) = entry.as_object_mut() {
                        object.insert("enabled".to_string(), serde_json::Value::Bool(checked));
                    }
                    write_storage_json_object(&window, "slskr.automationRecipeState", &state);
                    apply_automation_state(&document, &state);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some("Automation recipe state saved."));
                    }
                },
            ));
            input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let dry_run_buttons = document.query_selector_all("[data-slskr-recipe-dry-run]")?;
        for index in 0..dry_run_buttons.length() {
            let Some(node) = dry_run_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let recipe_id = button
                .get_attribute("data-slskr-recipe-dry-run")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let Some(recipe) = automation_recipes()
                        .iter()
                        .find(|recipe| recipe.id == recipe_id)
                        .copied()
                    else {
                        return;
                    };
                    let report = automation_dry_run_report(recipe, "browser-local");
                    let mut state = storage_json_object(
                        &window,
                        "slskr.automationRecipeState",
                        default_automation_state(),
                    );
                    let entry = state
                        .entry(recipe.id.to_string())
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(object) = entry.as_object_mut() {
                        object.insert(
                            "lastDryRunAt".to_string(),
                            serde_json::Value::String("browser-local".to_string()),
                        );
                        object.insert("lastDryRunReport".to_string(), report.clone());
                    }
                    write_storage_json_object(&window, "slskr.automationRecipeState", &state);
                    apply_automation_state(&document, &state);
                    if let Some(output) = document.get_element_by_id("slskr-automation-report") {
                        output.set_text_content(Some(
                            &serde_json::to_string_pretty(&report).unwrap_or_default(),
                        ));
                    }
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status
                            .set_text_content(Some(&format!("{} dry run recorded.", recipe.title)));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let action_buttons = document.query_selector_all("[data-slskr-automation-action]")?;
        for index in 0..action_buttons.length() {
            let Some(node) = action_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-automation-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let state = if action == "reset" {
                        remove_storage_item(&window, "slskr.automationRecipeState");
                        default_automation_state()
                    } else {
                        storage_json_object(
                            &window,
                            "slskr.automationRecipeState",
                            default_automation_state(),
                        )
                    };
                    apply_automation_state(&document, &state);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some(if action == "reset" {
                            "Automation recipe state reset."
                        } else {
                            "Automation history report prepared."
                        }));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    if document
        .query_selector("[data-slskr-search-planner]")?
        .is_some()
    {
        let render_search_plan =
            |document: &web_sys::Document, window: &web_sys::Window, message: &str| {
                let query = document
                    .query_selector(r#"[data-slskr-search-setting="query"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_else(|| "public domain theme".to_string());
                let profile = document
                    .query_selector(r#"[data-slskr-search-setting="profile"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_else(|| "lossless-exact".to_string());
                let fold = document
                    .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.checked())
                    .unwrap_or(true);
                if let Some(output) = document.get_element_by_id("slskr-search-planner-report") {
                    output.set_text_content(Some(&search_planner_report(&query, &profile, fold)));
                }
                if let Some(status) = document.get_element_by_id("slskr-search-planner-status") {
                    status.set_text_content(Some(message));
                }
                let mut stored = serde_json::Map::new();
                stored.insert("query".to_string(), serde_json::Value::String(query));
                stored.insert("profile".to_string(), serde_json::Value::String(profile));
                stored.insert("foldDuplicates".to_string(), serde_json::Value::Bool(fold));
                write_storage_json_object(window, "slskr.search.planner", &stored);
            };

        render_search_plan(document, window, "Search planner ready.");
        let buttons = document.query_selector_all("[data-slskr-search-action]")?;
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-search-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    if action == "reset" {
                        if let Ok(Some(input)) =
                            document.query_selector(r#"[data-slskr-search-setting="query"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_value("public domain theme");
                            }
                        }
                        if let Ok(Some(input)) =
                            document.query_selector(r#"[data-slskr-search-setting="profile"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_value("lossless-exact");
                            }
                        }
                        if let Ok(Some(input)) = document
                            .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_checked(true);
                            }
                        }
                        remove_storage_item(&window, "slskr.search.planner");
                    }
                    let query = document
                        .query_selector(r#"[data-slskr-search-setting="query"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_default();
                    let profile = document
                        .query_selector(r#"[data-slskr-search-setting="profile"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_else(|| "lossless-exact".to_string());
                    let fold = document
                        .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.checked())
                        .unwrap_or(true);
                    if let Some(output) = document.get_element_by_id("slskr-search-planner-report")
                    {
                        output
                            .set_text_content(Some(&search_planner_report(&query, &profile, fold)));
                    }
                    if let Some(status) = document.get_element_by_id("slskr-search-planner-status")
                    {
                        status.set_text_content(Some(if action == "reset" {
                            "Search planner reset."
                        } else {
                            "Search action preview prepared."
                        }));
                    }
                    let mut stored = serde_json::Map::new();
                    stored.insert("query".to_string(), serde_json::Value::String(query));
                    stored.insert("profile".to_string(), serde_json::Value::String(profile));
                    stored.insert("foldDuplicates".to_string(), serde_json::Value::Bool(fold));
                    write_storage_json_object(&window, "slskr.search.planner", &stored);
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
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

#[derive(Clone, Debug, PartialEq)]
pub struct RustMilkdropPreset {
    pub decay: f64,
    pub rot: f64,
    pub wave_a: f64,
    pub wave_b: f64,
    pub wave_g: f64,
    pub wave_r: f64,
    pub wave_scale: f64,
    pub zoom: f64,
}

impl Default for RustMilkdropPreset {
    fn default() -> Self {
        Self {
            decay: 0.89,
            rot: 0.012,
            wave_a: 0.86,
            wave_b: 0.92,
            wave_g: 0.58,
            wave_r: 0.16,
            wave_scale: 1.25,
            zoom: 1.02,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RustMilkdropFrame {
    pub background_alpha: f64,
    pub rotation: f64,
    pub wave_color: (u8, u8, u8),
    pub wave_radius: f64,
    pub zoom: f64,
}

fn clamp_unit(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn clamp_range(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

pub fn parse_rust_milkdrop_preset(source: &str) -> RustMilkdropPreset {
    let mut preset = RustMilkdropPreset::default();
    for line in source.lines() {
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let Ok(value) = raw_value.trim().parse::<f64>() else {
            continue;
        };
        match key.as_str() {
            "decay" => preset.decay = clamp_range(value, 0.5, 0.99),
            "rot" => preset.rot = clamp_range(value, -0.5, 0.5),
            "wave_a" => preset.wave_a = clamp_unit(value),
            "wave_b" => preset.wave_b = clamp_unit(value),
            "wave_g" => preset.wave_g = clamp_unit(value),
            "wave_r" => preset.wave_r = clamp_unit(value),
            "wave_scale" => preset.wave_scale = clamp_range(value, 0.2, 3.0),
            "zoom" => preset.zoom = clamp_range(value, 0.5, 1.8),
            _ => {}
        }
    }
    preset
}

pub fn rust_milkdrop_frame(
    preset: &RustMilkdropPreset,
    time_seconds: f64,
    bass: f64,
    mid: f64,
    treble: f64,
) -> RustMilkdropFrame {
    let bass = clamp_unit(bass);
    let mid = clamp_unit(mid);
    let treble = clamp_unit(treble);
    let pulse = (time_seconds * 1.7).sin() * 0.5 + 0.5;
    RustMilkdropFrame {
        background_alpha: clamp_range(1.0 - preset.decay, 0.01, 0.5),
        rotation: preset.rot + (time_seconds * 0.37).sin() * 0.035 + (treble - 0.5) * 0.05,
        wave_color: (
            ((preset.wave_r + bass * 0.35) * 255.0).min(255.0) as u8,
            ((preset.wave_g + mid * 0.30) * 255.0).min(255.0) as u8,
            ((preset.wave_b + treble * 0.25) * 255.0).min(255.0) as u8,
        ),
        wave_radius: clamp_range(
            0.18 + preset.wave_scale * 0.09 + bass * 0.12 + pulse * 0.04,
            0.12,
            0.68,
        ),
        zoom: clamp_range(preset.zoom + (pulse - 0.5) * 0.035, 0.5, 1.8),
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const RUST_MILKDROP_PRESETS: [&str; 3] = [
    "name=slskr native warp\ndecay=0.89\nwave_r=0.16\nwave_g=0.58\nwave_b=0.92\nwave_a=0.86\nwave_scale=1.25\nzoom=1.02\nrot=0.012",
    "name=slskr amber tunnel\ndecay=0.86\nwave_r=0.92\nwave_g=0.52\nwave_b=0.18\nwave_a=0.82\nwave_scale=1.55\nzoom=1.05\nrot=-0.018",
    "name=slskr green pulse\ndecay=0.91\nwave_r=0.20\nwave_g=0.86\nwave_b=0.44\nwave_a=0.78\nwave_scale=1.1\nzoom=0.98\nrot=0.028",
];

pub fn rust_milkdrop_preset_name(source: &str) -> String {
    source
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            if key.trim().eq_ignore_ascii_case("name") {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| "Rust MilkDrop preset".to_string())
}

#[derive(Clone, Debug, PartialEq)]
pub enum MilkdropValue {
    Number(f64),
    Text(String),
}

impl MilkdropValue {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Text(_) => None,
        }
    }

    pub fn as_text(&self) -> String {
        match self {
            Self::Number(value) => {
                if value.fract().abs() < f64::EPSILON {
                    format!("{}", *value as i64)
                } else {
                    format!("{value}")
                }
            }
            Self::Text(value) => value.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MilkdropEquations {
    pub frame: String,
    pub init: String,
    pub per_frame: String,
    pub per_pixel: String,
    pub point: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MilkdropIndexedEntry {
    pub base_values: BTreeMap<String, MilkdropValue>,
    pub equations: MilkdropEquations,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MilkdropPresetDocument {
    pub base_values: BTreeMap<String, MilkdropValue>,
    pub comp_shader: String,
    pub format: String,
    pub index: usize,
    pub raw_sections: BTreeMap<String, BTreeMap<String, MilkdropValue>>,
    pub shapes: Vec<MilkdropIndexedEntry>,
    pub source: String,
    pub sprites: Vec<MilkdropIndexedEntry>,
    pub title: String,
    pub warp_shader: String,
    pub waves: Vec<MilkdropIndexedEntry>,
    pub equations: MilkdropEquations,
}

impl MilkdropPresetDocument {
    fn new(source: &str, index: usize) -> Self {
        Self {
            base_values: BTreeMap::new(),
            comp_shader: String::new(),
            equations: MilkdropEquations::default(),
            format: "milk".to_string(),
            index,
            raw_sections: BTreeMap::new(),
            shapes: Vec::new(),
            source: source.to_string(),
            sprites: Vec::new(),
            title: String::new(),
            warp_shader: String::new(),
            waves: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MilkdropPresetSet {
    pub format: String,
    pub presets: Vec<MilkdropPresetDocument>,
}

fn is_numeric_milkdrop_value(value: &str) -> bool {
    value.trim().parse::<f64>().is_ok()
}

fn normalize_milkdrop_value(value: &str) -> MilkdropValue {
    let trimmed = value.trim();
    if is_numeric_milkdrop_value(trimmed) {
        MilkdropValue::Number(trimmed.parse::<f64>().unwrap_or(0.0))
    } else {
        MilkdropValue::Text(trimmed.to_string())
    }
}

fn append_milkdrop_statement(target: &mut String, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(value.trim());
}

fn split_milkdrop_preset_pair(source: &str) -> Vec<String> {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut offset = 0usize;
    for line in normalized.split_inclusive('\n') {
        if line.trim().eq_ignore_ascii_case("[preset01]") {
            return vec![
                normalized[..offset].to_string(),
                normalized[offset..].to_string(),
            ];
        }
        offset += line.len();
    }
    vec![normalized]
}

fn parse_indexed_key<'a>(key: &'a str, prefix: &str) -> Option<(usize, &'a str)> {
    let rest = key.strip_prefix(prefix)?;
    let digit_count = rest.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 || !rest[digit_count..].starts_with('_') {
        return None;
    }
    let index = rest[..digit_count].parse::<usize>().ok()?;
    Some((index, &rest[digit_count + 1..]))
}

fn ensure_milkdrop_entry(
    entries: &mut Vec<MilkdropIndexedEntry>,
    index: usize,
) -> &mut MilkdropIndexedEntry {
    while entries.len() <= index {
        entries.push(MilkdropIndexedEntry::default());
    }
    &mut entries[index]
}

fn assign_milkdrop_equation(equations: &mut MilkdropEquations, key: &str, value: &str) -> bool {
    if key.starts_with("per_frame") || key.starts_with("frame") {
        append_milkdrop_statement(&mut equations.per_frame, value);
        return true;
    }
    if key.starts_with("per_pixel") || key.starts_with("per_vertex") {
        append_milkdrop_statement(&mut equations.per_pixel, value);
        return true;
    }
    if key.starts_with("init") {
        append_milkdrop_statement(&mut equations.init, value);
        return true;
    }
    matches!(
        key.split('_').next().unwrap_or_default(),
        "per_frame" | "per_pixel" | "per_vertex" | "init"
    )
}

fn assign_milkdrop_indexed_equation(
    equations: &mut MilkdropEquations,
    key: &str,
    value: &str,
) -> bool {
    if key.starts_with("init") {
        append_milkdrop_statement(&mut equations.init, value);
        return true;
    }
    if key.starts_with("frame") || key.starts_with("per_frame") {
        append_milkdrop_statement(&mut equations.frame, value);
        return true;
    }
    if key.starts_with("point") || key.starts_with("per_point") {
        append_milkdrop_statement(&mut equations.point, value);
        return true;
    }
    false
}

fn parse_milkdrop_preset_text(text: &str, index: usize) -> MilkdropPresetDocument {
    let mut preset = MilkdropPresetDocument::new(text, index);
    let mut section = "preset".to_string();

    for raw_line in text.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
            section = trimmed[1..trimmed.len() - 1].trim().to_ascii_lowercase();
            preset.raw_sections.entry(section.clone()).or_default();
            continue;
        }
        let Some((raw_key, raw_value)) = raw_line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let raw_value = raw_value.trim();
        let value = normalize_milkdrop_value(raw_value);
        preset
            .raw_sections
            .entry(section.clone())
            .or_default()
            .insert(key.clone(), value.clone());

        if key == "name" || key == "preset_name" {
            preset.title = raw_value.to_string();
            continue;
        }
        if let Some((shape_index, shape_key)) = parse_indexed_key(&key, "shape") {
            let entry = ensure_milkdrop_entry(&mut preset.shapes, shape_index);
            if !assign_milkdrop_indexed_equation(&mut entry.equations, shape_key, raw_value) {
                entry.base_values.insert(shape_key.to_string(), value);
            }
            continue;
        }
        if let Some((sprite_index, sprite_key)) = parse_indexed_key(&key, "sprite") {
            let entry = ensure_milkdrop_entry(&mut preset.sprites, sprite_index);
            if !assign_milkdrop_indexed_equation(&mut entry.equations, sprite_key, raw_value) {
                entry.base_values.insert(sprite_key.to_string(), value);
            }
            continue;
        }
        if let Some((wave_index, wave_key)) = parse_indexed_key(&key, "wavecode_") {
            let entry = ensure_milkdrop_entry(&mut preset.waves, wave_index);
            if !assign_milkdrop_indexed_equation(&mut entry.equations, wave_key, raw_value) {
                entry.base_values.insert(wave_key.to_string(), value);
            }
            continue;
        }
        if key.starts_with("warp_shader") {
            append_milkdrop_statement(&mut preset.warp_shader, raw_value);
            continue;
        }
        if key.starts_with("comp_shader") {
            append_milkdrop_statement(&mut preset.comp_shader, raw_value);
            continue;
        }
        if assign_milkdrop_equation(&mut preset.equations, &key, raw_value) {
            continue;
        }
        preset.base_values.insert(key, value);
    }

    preset
}

pub fn parse_milkdrop_preset_set(source: &str, force_milk2: bool) -> MilkdropPresetSet {
    let chunks = split_milkdrop_preset_pair(source);
    let format = if force_milk2 || chunks.len() > 1 {
        "milk2"
    } else {
        "milk"
    }
    .to_string();
    let presets = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let mut preset = parse_milkdrop_preset_text(chunk, index);
            preset.format = format.clone();
            preset
        })
        .collect::<Vec<_>>();
    MilkdropPresetSet { format, presets }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MilkdropFragment {
    pub entries: Vec<MilkdropIndexedEntry>,
    pub fragment_type: String,
    pub source: String,
}

fn milkdrop_fragment_type(file_name: &str, requested_type: &str) -> String {
    if requested_type == "shape" || requested_type == "wave" {
        return requested_type.to_string();
    }
    if file_name.to_ascii_lowercase().ends_with(".wave") {
        "wave".to_string()
    } else {
        "shape".to_string()
    }
}

fn parse_standalone_milkdrop_fragment_entry(source: &str) -> MilkdropIndexedEntry {
    let mut entry = MilkdropIndexedEntry::default();
    entry
        .base_values
        .insert("enabled".to_string(), MilkdropValue::Number(1.0));
    for raw_line in source.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with(';')
            || trimmed.starts_with("//")
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            continue;
        }
        let Some((raw_key, raw_value)) = raw_line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let raw_value = raw_value.trim();
        if !assign_milkdrop_indexed_equation(&mut entry.equations, &key, raw_value) {
            entry
                .base_values
                .insert(key, normalize_milkdrop_value(raw_value));
        }
    }
    entry
}

pub fn parse_milkdrop_fragment(
    source: &str,
    file_name: &str,
    requested_type: &str,
) -> MilkdropFragment {
    let fragment_type = milkdrop_fragment_type(file_name, requested_type);
    let parsed = parse_milkdrop_preset_set(source, false);
    let parsed_entries = if fragment_type == "wave" {
        parsed
            .presets
            .first()
            .map(|preset| preset.waves.clone())
            .unwrap_or_default()
    } else {
        parsed
            .presets
            .first()
            .map(|preset| preset.shapes.clone())
            .unwrap_or_default()
    };
    let has_prefixed_entries = parsed_entries.iter().any(|entry| {
        !entry.base_values.is_empty() || entry.equations != MilkdropEquations::default()
    });
    let entries = if has_prefixed_entries {
        parsed_entries
            .into_iter()
            .filter(|entry| {
                !entry.base_values.is_empty() || entry.equations != MilkdropEquations::default()
            })
            .collect()
    } else {
        vec![parse_standalone_milkdrop_fragment_entry(source)]
    };
    MilkdropFragment {
        entries,
        fragment_type,
        source: source.to_string(),
    }
}

fn append_milkdrop_equation_lines(lines: &mut Vec<String>, key: &str, equation_text: &str) {
    for (index, line) in equation_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
    {
        lines.push(format!("{key}_{}={line}", index + 1));
    }
}

fn append_milkdrop_base_value_lines(
    lines: &mut Vec<String>,
    values: &BTreeMap<String, MilkdropValue>,
    prefix: &str,
) {
    for (key, value) in values {
        lines.push(format!("{prefix}{key}={}", value.as_text()));
    }
}

fn append_milkdrop_indexed_entry_lines(
    lines: &mut Vec<String>,
    prefix: &str,
    entry: &MilkdropIndexedEntry,
) {
    append_milkdrop_base_value_lines(lines, &entry.base_values, prefix);
    append_milkdrop_equation_lines(lines, &format!("{prefix}init"), &entry.equations.init);
    append_milkdrop_equation_lines(lines, &format!("{prefix}per_frame"), &entry.equations.frame);
    append_milkdrop_equation_lines(lines, &format!("{prefix}per_point"), &entry.equations.point);
}

pub fn serialize_milkdrop_fragment(entry: &MilkdropIndexedEntry, requested_type: &str) -> String {
    let fragment_type = milkdrop_fragment_type("", requested_type);
    let mut lines = vec![format!("[{fragment_type}]")];
    append_milkdrop_indexed_entry_lines(&mut lines, "", entry);
    format!("{}\n", lines.join("\n"))
}

pub fn serialize_milkdrop_preset_set(parsed: &MilkdropPresetSet) -> String {
    let include_sections = parsed.format == "milk2" || parsed.presets.len() > 1;
    let mut rendered_presets = Vec::new();
    for (index, preset) in parsed.presets.iter().enumerate() {
        let mut lines = Vec::new();
        if include_sections {
            lines.push(format!("[preset{index:02}]"));
        }
        if !preset.title.is_empty() {
            lines.push(format!("name={}", preset.title));
        }
        append_milkdrop_base_value_lines(&mut lines, &preset.base_values, "");
        append_milkdrop_equation_lines(&mut lines, "init", &preset.equations.init);
        append_milkdrop_equation_lines(&mut lines, "per_frame", &preset.equations.per_frame);
        append_milkdrop_equation_lines(&mut lines, "per_pixel", &preset.equations.per_pixel);
        append_milkdrop_equation_lines(&mut lines, "warp_shader", &preset.warp_shader);
        append_milkdrop_equation_lines(&mut lines, "comp_shader", &preset.comp_shader);
        for (shape_index, shape) in preset.shapes.iter().enumerate() {
            append_milkdrop_indexed_entry_lines(
                &mut lines,
                &format!("shape{shape_index:02}_"),
                shape,
            );
        }
        for (sprite_index, sprite) in preset.sprites.iter().enumerate() {
            append_milkdrop_indexed_entry_lines(
                &mut lines,
                &format!("sprite{sprite_index:02}_"),
                sprite,
            );
        }
        for (wave_index, wave) in preset.waves.iter().enumerate() {
            append_milkdrop_indexed_entry_lines(
                &mut lines,
                &format!("wavecode_{wave_index}_"),
                wave,
            );
        }
        rendered_presets.push(lines.join("\n"));
    }
    format!("{}\n", rendered_presets.join("\n"))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MilkdropShaderProgram {
    pub declarations: Vec<String>,
    pub expression: String,
    pub texture_samplers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MilkdropShaderSupport {
    pub supported: bool,
}

fn strip_milkdrop_shader_comments(source: &str) -> String {
    let mut output = String::new();
    let mut chars = source.chars().peekable();
    let mut in_block = false;
    while let Some(ch) = chars.next() {
        if in_block {
            if ch == '*' && chars.peek() == Some(&'/') {
                let _ = chars.next();
                in_block = false;
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'*') {
            let _ = chars.next();
            in_block = true;
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }
        output.push(ch);
    }
    output.trim().to_string()
}

fn unwrap_milkdrop_shader_body(source: &str) -> String {
    let mut source = strip_milkdrop_shader_comments(source);
    let lower = source.to_ascii_lowercase();
    if let Some(index) = lower.find("shader_body") {
        source.replace_range(index..index + "shader_body".len(), "");
    }
    let trimmed = source.trim();
    let trimmed = trimmed.strip_prefix('{').unwrap_or(trimmed).trim();
    let trimmed = trimmed.strip_suffix('}').unwrap_or(trimmed).trim();
    trimmed.to_string()
}

fn normalize_simple_milkdrop_conditional_return(source: &str) -> String {
    let unwrapped = unwrap_milkdrop_shader_body(source);
    let compact = unwrapped
        .replace('{', " ")
        .replace('}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let lower = compact.to_ascii_lowercase();
    if !lower.starts_with("if") || !lower.contains(" else ") {
        return source.to_string();
    }
    let Some(condition_start) = compact.find('(') else {
        return source.to_string();
    };
    let Some(condition_end) = compact[condition_start + 1..].find(')') else {
        return source.to_string();
    };
    let condition_end = condition_start + 1 + condition_end;
    let condition = compact[condition_start + 1..condition_end].trim();
    let rest = compact[condition_end + 1..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let Some(else_index) = lower_rest.find(" else ") else {
        return source.to_string();
    };
    let true_part = rest[..else_index].trim();
    let false_part = rest[else_index + " else ".len()..].trim();
    let extract_ret = |part: &str| -> Option<String> {
        let part = part.trim();
        let lower = part.to_ascii_lowercase();
        let value = lower.strip_prefix("ret")?;
        let value = value.trim_start();
        let value = value.strip_prefix('=')?.trim();
        Some(value.trim_end_matches(';').trim().to_string())
    };
    let Some(true_ret) = extract_ret(true_part) else {
        return source.to_string();
    };
    let Some(false_ret) = extract_ret(false_part) else {
        return source.to_string();
    };
    format!("ret = ({condition}) ? ({true_ret}) : ({false_ret});")
}

fn is_milkdrop_main_sampler(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "previousframe" | "sampler_main" | "sampler_fc_main" | "sampler_sampler_main"
    )
}

pub fn get_milkdrop_shader_texture_samplers(source: &str) -> Vec<String> {
    let cleaned = strip_milkdrop_shader_comments(source);
    let mut samplers = Vec::new();
    for marker in ["tex2D(", "tex("] {
        let mut rest = cleaned.as_str();
        while let Some(index) = rest.to_ascii_lowercase().find(&marker.to_ascii_lowercase()) {
            let after = &rest[index + marker.len()..];
            let sampler = after
                .trim_start()
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .collect::<String>();
            if !sampler.is_empty()
                && !is_milkdrop_main_sampler(&sampler)
                && !samplers.contains(&sampler)
            {
                samplers.push(sampler);
            }
            rest = &after[after.find(',').unwrap_or(after.len())..];
        }
    }
    samplers.truncate(4);
    samplers
}

fn normalize_milkdrop_shader_expression(expression: &str) -> String {
    expression
        .replace("float4(", "vec4(")
        .replace("float3(", "vec3(")
        .replace("float2(", "vec2(")
        .replace("saturate(", "clamp01(")
        .replace("lerp(", "mix(")
        .replace("frac(", "fract(")
        .replace("fmod(", "mod(")
        .replace("rsqrt(", "inversesqrt(")
        .replace("atan2(", "atan(")
}

fn normalize_milkdrop_shader_type(shader_type: &str) -> String {
    shader_type
        .to_ascii_lowercase()
        .replace("float2", "vec2")
        .replace("float3", "vec3")
        .replace("float4", "vec4")
}

fn normalize_milkdrop_shader_source(source: &str, texture_samplers: &[String]) -> String {
    let mut normalized =
        unwrap_milkdrop_shader_body(&normalize_simple_milkdrop_conditional_return(source));
    for sampler in ["tex2D", "tex"] {
        loop {
            let Some(index) = normalized
                .to_ascii_lowercase()
                .find(&format!("{}(", sampler.to_ascii_lowercase()))
            else {
                break;
            };
            let start = index + sampler.len() + 1;
            let after = &normalized[start..];
            let name = after
                .trim_start()
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .collect::<String>();
            if name.is_empty() {
                break;
            }
            let whitespace = after.len() - after.trim_start().len();
            let name_start = start + whitespace;
            let name_end = name_start + name.len();
            let replacement = if is_milkdrop_main_sampler(&name) {
                "previousFrame".to_string()
            } else if let Some(texture_index) =
                texture_samplers.iter().position(|value| value == &name)
            {
                format!("shaderTexture{texture_index}")
            } else {
                name.clone()
            };
            normalized.replace_range(index..name_end, &format!("texture({replacement}"));
        }
    }
    normalized
}

fn is_safe_milkdrop_shader_expression(expression: &str) -> bool {
    if expression.trim().is_empty() {
        return false;
    }
    if !expression.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '_' | '.'
                    | ','
                    | '+'
                    | '-'
                    | '*'
                    | '/'
                    | '%'
                    | '<'
                    | '>'
                    | '='
                    | '!'
                    | '&'
                    | '|'
                    | '^'
                    | '~'
                    | '?'
                    | ':'
                    | '('
                    | ')'
                    | ' '
            )
    }) {
        return false;
    }
    if expression.contains("texture(")
        && !(expression.contains("previousFrame") || expression.contains("shaderTexture"))
    {
        return false;
    }
    true
}

fn split_milkdrop_shader_declaration(statement: &str) -> Option<(&str, &str, &str)> {
    for shader_type in [
        "float4", "float3", "float2", "float", "vec4", "vec3", "vec2",
    ] {
        let Some(rest) = statement.strip_prefix(shader_type) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some((name, expression)) = rest.split_once('=') else {
            return None;
        };
        let name = name.trim();
        if !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return None;
        }
        return Some((shader_type, name, expression.trim()));
    }
    None
}

fn split_milkdrop_shader_assignment(statement: &str) -> Option<(&str, &str, &str)> {
    for operator in ["+=", "-=", "*=", "/=", "="] {
        let Some((name, expression)) = statement.split_once(operator) else {
            continue;
        };
        let name = name.trim();
        if !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return None;
        }
        return Some((name, operator, expression.trim()));
    }
    None
}

fn parse_milkdrop_shader_program(source: &str) -> Option<MilkdropShaderProgram> {
    let normalized_source = normalize_simple_milkdrop_conditional_return(source);
    let lowered = normalized_source.to_ascii_lowercase();
    if lowered.contains("for (")
        || lowered.contains("while (")
        || lowered.contains("float3x")
        || lowered.contains("float4x")
        || lowered.contains("mul(")
        || lowered.contains("sampler2d ")
    {
        return None;
    }
    if lowered.starts_with("if") {
        return None;
    }
    let texture_samplers = get_milkdrop_shader_texture_samplers(&normalized_source);
    let cleaned = normalize_milkdrop_shader_source(&normalized_source, &texture_samplers);
    let mut declarations = Vec::new();
    let mut mutable_variables = Vec::new();
    let mut expression = String::new();

    for statement in cleaned
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Some(ret_expression) = statement
            .strip_prefix("ret")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
        {
            if !expression.is_empty() {
                return None;
            }
            expression = normalize_milkdrop_shader_expression(ret_expression.trim());
            continue;
        }
        if !expression.is_empty() {
            return None;
        }
        if let Some((shader_type, name, declaration_expression)) =
            split_milkdrop_shader_declaration(statement)
        {
            let declaration_expression =
                normalize_milkdrop_shader_expression(declaration_expression);
            if !is_safe_milkdrop_shader_expression(&declaration_expression) {
                return None;
            }
            mutable_variables.push(name.to_string());
            declarations.push(format!(
                "{} {name} = {declaration_expression};",
                normalize_milkdrop_shader_type(shader_type)
            ));
            continue;
        }
        if let Some((name, operator, assignment_expression)) =
            split_milkdrop_shader_assignment(statement)
        {
            if !mutable_variables.iter().any(|value| value == name) {
                return None;
            }
            let assignment_expression = normalize_milkdrop_shader_expression(assignment_expression);
            if !is_safe_milkdrop_shader_expression(&assignment_expression) {
                return None;
            }
            declarations.push(format!("{name} {operator} {assignment_expression};"));
            continue;
        }
        return None;
    }

    if !is_safe_milkdrop_shader_expression(&expression) {
        return None;
    }
    Some(MilkdropShaderProgram {
        declarations,
        expression,
        texture_samplers,
    })
}

pub fn translate_milkdrop_shader_expression(source: &str) -> String {
    parse_milkdrop_shader_program(source)
        .map(|program| program.expression)
        .unwrap_or_default()
}

pub fn create_translated_milkdrop_fragment_shader(source: &str) -> String {
    let Some(program) = parse_milkdrop_shader_program(source) else {
        return String::new();
    };
    let uniforms = (1..=64)
        .map(|index| format!("uniform float q{index};"))
        .chain(
            ["bass", "bass_att", "mid", "mid_att", "treb", "treb_att"]
                .into_iter()
                .map(|name| format!("uniform float {name};")),
        )
        .collect::<Vec<_>>()
        .join("\n");
    let texture_uniforms = program
        .texture_samplers
        .iter()
        .enumerate()
        .map(|(index, _)| format!("uniform sampler2D shaderTexture{index};"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"#version 300 es
precision highp float;
uniform vec3 color;
uniform sampler2D previousFrame;
{texture_uniforms}
uniform float feedback;
uniform float outputAlpha;
uniform float time;
uniform float sampleRate;
uniform float fftBins[64];
uniform float waveformBins[64];
uniform vec2 resolution;
uniform vec2 pixelSize;
uniform float aspect;
uniform vec4 texsize;
{uniforms}
in vec2 uv;
out vec4 outColor;
float clamp01(float value) {{ return clamp(value, 0.0, 1.0); }}
vec2 clamp01(vec2 value) {{ return clamp(value, vec2(0.0), vec2(1.0)); }}
vec3 clamp01(vec3 value) {{ return clamp(value, vec3(0.0), vec3(1.0)); }}
vec4 clamp01(vec4 value) {{ return clamp(value, vec4(0.0), vec4(1.0)); }}
float get_fft(float position) {{ int index = int(clamp(position, 0.0, 1.0) * 63.0); return fftBins[index]; }}
float get_fft_hz(float hz) {{ float nyquist = max(sampleRate * 0.5, 1.0); return get_fft(hz / nyquist); }}
float get_waveform(float position) {{ int index = int(clamp(position, 0.0, 1.0) * 63.0); return waveformBins[index]; }}
void main() {{
  float x = uv.x;
  float y = uv.y;
  vec2 centeredUv = uv - vec2(0.5);
  float rad = length(centeredUv);
  float ang = atan(centeredUv.y, centeredUv.x);
  {}
  vec3 ret = vec3({});
  vec3 previous = texture(previousFrame, clamp(uv, vec2(0.0), vec2(1.0))).rgb;
  outColor = vec4(mix(ret, previous, feedback), outputAlpha);
}}"#,
        program.declarations.join("\n  "),
        program.expression
    )
}

fn normalize_milkdrop_wgsl_expression(expression: &str) -> String {
    expression
        .replace(
            "texture(previousFrame,",
            "textureSample(previousFrame, previousSampler,",
        )
        .replace(
            "texture(shaderTexture0,",
            "textureSample(shaderTexture0, shaderTextureSampler,",
        )
        .replace(
            "texture(shaderTexture1,",
            "textureSample(shaderTexture1, shaderTextureSampler,",
        )
        .replace(
            "texture(shaderTexture2,",
            "textureSample(shaderTexture2, shaderTextureSampler,",
        )
        .replace(
            "texture(shaderTexture3,",
            "textureSample(shaderTexture3, shaderTextureSampler,",
        )
        .replace("vec2(", "vec2f(")
        .replace("vec3(", "vec3f(")
        .replace("vec4(", "vec4f(")
        .replace("clamp01(vec2f(", "clamp01v2(vec2f(")
        .replace("clamp01(vec3f(", "clamp01v3(vec3f(")
        .replace("clamp01(vec4f(", "clamp01v4(vec4f(")
        .replace("atan(", "atan2(")
}

fn normalize_milkdrop_wgsl_declaration(declaration: &str) -> String {
    let declaration = normalize_milkdrop_wgsl_expression(declaration);
    for prefix in ["vec2 ", "vec3 ", "vec4 ", "float "] {
        if let Some(rest) = declaration.strip_prefix(prefix) {
            return format!("var {rest}");
        }
    }
    declaration
}

pub fn create_translated_milkdrop_wgsl_shader(source: &str) -> String {
    let Some(program) = parse_milkdrop_shader_program(source) else {
        return String::new();
    };
    if std::iter::once(&program.expression)
        .chain(program.declarations.iter())
        .any(|statement| {
            statement.contains('?')
                || statement.contains('&')
                || statement.contains('|')
                || statement.contains('^')
                || statement.contains('~')
        })
    {
        return String::new();
    }
    let q_fields = (1..=64)
        .map(|index| format!("  q{index}: f32,"))
        .collect::<Vec<_>>()
        .join("\n");
    let q_locals = (1..=64)
        .map(|index| format!("  let q{index} = uniforms.q{index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let texture_declarations = program
        .texture_samplers
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "@group(0) @binding({}) var shaderTexture{index}: texture_2d<f32>;",
                index + 3
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let declarations = program
        .declarations
        .iter()
        .map(|declaration| format!("  {}", normalize_milkdrop_wgsl_declaration(declaration)))
        .collect::<Vec<_>>()
        .join("\n");
    let expression = normalize_milkdrop_wgsl_expression(&program.expression);
    format!(
        r#"struct Uniforms {{
  color: vec4f,
  time: f32,
  bass: f32,
  mid: f32,
  treb: f32,
  bass_att: f32,
  mid_att: f32,
  treb_att: f32,
  feedback: f32,
  outputAlpha: f32,
  sampleRate: f32,
{q_fields}
  fft63: f32,
  waveform63: f32,
}};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var previousFrame: texture_2d<f32>;
@group(0) @binding(2) var previousSampler: sampler;
@group(0) @binding(7) var shaderTextureSampler: sampler;
{texture_declarations}
fn get_fft(position: f32) -> f32 {{ return uniforms.fft63; }}
fn get_fft_hz(hz: f32) -> f32 {{ return get_fft(hz / max(uniforms.sampleRate * 0.5, 1.0)); }}
fn get_waveform(position: f32) -> f32 {{ return uniforms.waveform63; }}
@fragment
fn fragmentMain() -> @location(0) vec4f {{
  let uv = vec2f(0.5);
  let color = uniforms.color.rgb;
  let time = uniforms.time;
  let bass = uniforms.bass;
  let bass_att = uniforms.bass_att;
{q_locals}
{declarations}
  let ret = vec3f({expression});
  return vec4f(ret, uniforms.outputAlpha);
}}"#
    )
}

pub fn analyze_milkdrop_shader_support(source: &str) -> MilkdropShaderSupport {
    MilkdropShaderSupport {
        supported: source.trim().is_empty()
            || !create_translated_milkdrop_fragment_shader(source).is_empty(),
    }
}

pub fn analyze_milkdrop_webgpu_shader_support(source: &str) -> MilkdropShaderSupport {
    MilkdropShaderSupport {
        supported: source.trim().is_empty()
            || !create_translated_milkdrop_wgsl_shader(source).is_empty(),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MilkdropToken {
    Ident(String),
    Number(f64),
    Op(String),
}

fn tokenize_milkdrop_expression(expression: &str) -> Result<Vec<MilkdropToken>, String> {
    let chars = expression.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric()
                    || chars[index] == '_'
                    || chars[index] == '.')
            {
                index += 1;
            }
            tokens.push(MilkdropToken::Ident(
                chars[start..index]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase(),
            ));
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            if index < chars.len() && matches!(chars[index], 'e' | 'E') {
                index += 1;
                if index < chars.len() && matches!(chars[index], '+' | '-') {
                    index += 1;
                }
                while index < chars.len() && chars[index].is_ascii_digit() {
                    index += 1;
                }
            }
            let value = chars[start..index]
                .iter()
                .collect::<String>()
                .parse::<f64>()
                .map_err(|_| format!("Unsupported MilkDrop expression syntax: {expression}"))?;
            tokens.push(MilkdropToken::Number(value));
            continue;
        }
        let two = if index + 1 < chars.len() {
            Some([chars[index], chars[index + 1]].iter().collect::<String>())
        } else {
            None
        };
        if let Some(two) = two.as_deref().filter(|value| {
            matches!(
                *value,
                "&&" | "||" | "<<" | ">>" | "==" | "!=" | "<=" | ">="
            )
        }) {
            tokens.push(MilkdropToken::Op(two.to_string()));
            index += 2;
            continue;
        }
        if matches!(
            ch,
            '(' | ')' | '+' | '-' | '*' | '/' | '%' | ',' | '<' | '>' | '&' | '|' | '^' | '!' | '~'
        ) {
            tokens.push(MilkdropToken::Op(ch.to_string()));
            index += 1;
            continue;
        }
        return Err(format!(
            "Unsupported MilkDrop expression syntax: {expression}"
        ));
    }
    Ok(tokens)
}

fn milkdrop_number(scope: &BTreeMap<String, MilkdropValue>, name: &str) -> f64 {
    scope
        .get(name)
        .and_then(MilkdropValue::as_number)
        .unwrap_or(0.0)
}

fn milkdrop_indexed_sample(values: &[f64], position: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let normalized = position.clamp(0.0, 1.0);
    let index = ((normalized * values.len() as f64).floor() as usize).min(values.len() - 1);
    let value = values[index];
    if value > 1.0 {
        value / 255.0
    } else {
        value
    }
}

struct MilkdropExpressionParser<'a> {
    scope: &'a BTreeMap<String, MilkdropValue>,
    tokens: Vec<MilkdropToken>,
    index: usize,
}

impl<'a> MilkdropExpressionParser<'a> {
    fn new(tokens: Vec<MilkdropToken>, scope: &'a BTreeMap<String, MilkdropValue>) -> Self {
        Self {
            scope,
            tokens,
            index: 0,
        }
    }

    fn peek_op(&self) -> Option<&str> {
        match self.tokens.get(self.index) {
            Some(MilkdropToken::Op(value)) => Some(value),
            _ => None,
        }
    }

    fn match_op(&mut self, expected: &str) -> bool {
        if self.peek_op() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn consume(&mut self) -> Option<MilkdropToken> {
        let token = self.tokens.get(self.index).cloned();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    fn parse(&mut self) -> Result<f64, String> {
        let value = self.parse_logical_or()?;
        if self.index < self.tokens.len() {
            return Err("Unexpected trailing MilkDrop token".to_string());
        }
        Ok(value)
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.consume() {
            Some(MilkdropToken::Number(value)) => Ok(value),
            Some(MilkdropToken::Op(op)) if op == "(" => {
                let value = self.parse_logical_or()?;
                if !self.match_op(")") {
                    return Err("Unclosed MilkDrop expression group.".to_string());
                }
                Ok(value)
            }
            Some(MilkdropToken::Ident(name)) => {
                if self.match_op("(") {
                    let mut args = Vec::new();
                    if self.peek_op() != Some(")") {
                        loop {
                            args.push(self.parse_logical_or()?);
                            if !self.match_op(",") {
                                break;
                            }
                        }
                    }
                    if !self.match_op(")") {
                        return Err(format!("Unclosed function call: {name}"));
                    }
                    self.call_function(&name, &args)
                } else {
                    Ok(match name.as_str() {
                        "e" => std::f64::consts::E,
                        "pi" => std::f64::consts::PI,
                        _ => milkdrop_number(self.scope, &name),
                    })
                }
            }
            Some(token) => Err(format!("Unexpected MilkDrop token: {token:?}")),
            None => Err("Unexpected end of MilkDrop expression.".to_string()),
        }
    }

    fn call_function(&self, name: &str, args: &[f64]) -> Result<f64, String> {
        let arg = |index: usize, default: f64| args.get(index).copied().unwrap_or(default);
        let out = match name {
            "abs" => arg(0, 0.0).abs(),
            "above" => (arg(0, 0.0) > arg(1, 0.0)) as i32 as f64,
            "acos" => arg(0, 0.0).clamp(-1.0, 1.0).acos(),
            "asin" => arg(0, 0.0).clamp(-1.0, 1.0).asin(),
            "atan" => arg(0, 0.0).atan(),
            "atan2" => arg(0, 0.0).atan2(arg(1, 0.0)),
            "below" => (arg(0, 0.0) < arg(1, 0.0)) as i32 as f64,
            "band" => ((arg(0, 0.0).trunc() as i64) & (arg(1, 0.0).trunc() as i64)) as f64,
            "bor" => ((arg(0, 0.0).trunc() as i64) | (arg(1, 0.0).trunc() as i64)) as f64,
            "bnot" => (!(arg(0, 0.0).trunc() as i64)) as f64,
            "bxor" => ((arg(0, 0.0).trunc() as i64) ^ (arg(1, 0.0).trunc() as i64)) as f64,
            "ceil" => arg(0, 0.0).ceil(),
            "cos" => arg(0, 0.0).cos(),
            "div" => {
                let right = arg(1, 0.0);
                if right == 0.0 {
                    0.0
                } else {
                    arg(0, 0.0) / right
                }
            }
            "equal" => ((arg(0, 0.0) - arg(1, 0.0)).abs() < 0.00001) as i32 as f64,
            "exp" => arg(0, 0.0).exp(),
            "floor" => arg(0, 0.0).floor(),
            "get_fft" => {
                let values = milkdrop_frequency_data(self.scope);
                milkdrop_indexed_sample(&values, arg(0, 0.0))
            }
            "get_fft_hz" => {
                let sample_rate = milkdrop_number(self.scope, "sample_rate")
                    .max(milkdrop_number(self.scope, "samplerate"))
                    .max(44100.0);
                let nyquist = sample_rate / 2.0;
                let values = milkdrop_frequency_data(self.scope);
                milkdrop_indexed_sample(
                    &values,
                    if nyquist > 0.0 {
                        arg(0, 0.0) / nyquist
                    } else {
                        0.0
                    },
                )
            }
            "if" => {
                if arg(0, 0.0) != 0.0 {
                    arg(1, 0.0)
                } else {
                    arg(2, 0.0)
                }
            }
            "int" => arg(0, 0.0).trunc(),
            "log" => {
                if arg(0, 0.0) <= 0.0 {
                    0.0
                } else {
                    arg(0, 0.0).ln()
                }
            }
            "log10" => {
                if arg(0, 0.0) <= 0.0 {
                    0.0
                } else {
                    arg(0, 0.0).log10()
                }
            }
            "max" => arg(0, 0.0).max(arg(1, 0.0)),
            "min" => arg(0, 0.0).min(arg(1, 0.0)),
            "mod" => {
                let right = arg(1, 0.0);
                if right == 0.0 {
                    0.0
                } else {
                    arg(0, 0.0) % right
                }
            }
            "pow" => arg(0, 0.0).powf(arg(1, 0.0)),
            "rand" => {
                let upper = arg(0, 1.0).trunc().max(0.0);
                if upper <= 0.0 {
                    0.0
                } else {
                    (upper / 2.0).floor()
                }
            }
            "sign" => arg(0, 0.0).signum(),
            "sigmoid" => {
                let constraint = if arg(1, 1.0) == 0.0 { 1.0 } else { arg(1, 1.0) };
                1.0 / (1.0 + (-arg(0, 0.0) * constraint).exp())
            }
            "sin" => arg(0, 0.0).sin(),
            "sqr" => arg(0, 0.0) * arg(0, 0.0),
            "sqrt" => {
                if arg(0, 0.0) < 0.0 {
                    0.0
                } else {
                    arg(0, 0.0).sqrt()
                }
            }
            "tan" => arg(0, 0.0).tan(),
            _ => return Err(format!("Unsupported MilkDrop function: {name}")),
        };
        Ok(if out.is_finite() { out } else { 0.0 })
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        if self.match_op("+") {
            return self.parse_unary();
        }
        if self.match_op("-") {
            return Ok(-self.parse_unary()?);
        }
        if self.match_op("!") {
            return Ok((self.parse_unary()? == 0.0) as i32 as f64);
        }
        if self.match_op("~") {
            return Ok((!(self.parse_unary()?.trunc() as i64)) as f64);
        }
        self.parse_primary()
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        let mut value = self.parse_unary()?;
        while let Some(op) = self
            .peek_op()
            .filter(|op| matches!(*op, "*" | "/" | "%"))
            .map(str::to_string)
        {
            self.index += 1;
            let right = self.parse_unary()?;
            value = match op.as_str() {
                "*" => value * right,
                "/" => {
                    if right == 0.0 {
                        0.0
                    } else {
                        value / right
                    }
                }
                "%" => {
                    if right == 0.0 {
                        0.0
                    } else {
                        value % right
                    }
                }
                _ => value,
            };
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_factor()?;
        while let Some(op) = self
            .peek_op()
            .filter(|op| matches!(*op, "+" | "-"))
            .map(str::to_string)
        {
            self.index += 1;
            let right = self.parse_factor()?;
            value = if op == "+" {
                value + right
            } else {
                value - right
            };
        }
        Ok(value)
    }

    fn parse_shift(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        while let Some(op) = self
            .peek_op()
            .filter(|op| matches!(*op, "<<" | ">>"))
            .map(str::to_string)
        {
            self.index += 1;
            let right = self.parse_term()?;
            value = if op == "<<" {
                ((value.trunc() as i64) << (right.trunc() as u32)) as f64
            } else {
                ((value.trunc() as i64) >> (right.trunc() as u32)) as f64
            };
        }
        Ok(value)
    }

    fn parse_comparison(&mut self) -> Result<f64, String> {
        let mut value = self.parse_shift()?;
        while let Some(op) = self
            .peek_op()
            .filter(|op| matches!(*op, "<" | ">" | "<=" | ">=" | "==" | "!="))
            .map(str::to_string)
        {
            self.index += 1;
            let right = self.parse_shift()?;
            value = match op.as_str() {
                "<" => (value < right) as i32 as f64,
                ">" => (value > right) as i32 as f64,
                "<=" => (value <= right) as i32 as f64,
                ">=" => (value >= right) as i32 as f64,
                "==" => ((value - right).abs() < 0.00001) as i32 as f64,
                "!=" => ((value - right).abs() >= 0.00001) as i32 as f64,
                _ => value,
            };
        }
        Ok(value)
    }

    fn parse_bitwise_and(&mut self) -> Result<f64, String> {
        let mut value = self.parse_comparison()?;
        while self.match_op("&") {
            value = ((value.trunc() as i64) & (self.parse_comparison()?.trunc() as i64)) as f64;
        }
        Ok(value)
    }

    fn parse_bitwise_xor(&mut self) -> Result<f64, String> {
        let mut value = self.parse_bitwise_and()?;
        while self.match_op("^") {
            value = ((value.trunc() as i64) ^ (self.parse_bitwise_and()?.trunc() as i64)) as f64;
        }
        Ok(value)
    }

    fn parse_bitwise_or(&mut self) -> Result<f64, String> {
        let mut value = self.parse_bitwise_xor()?;
        while self.match_op("|") {
            value = ((value.trunc() as i64) | (self.parse_bitwise_xor()?.trunc() as i64)) as f64;
        }
        Ok(value)
    }

    fn parse_logical_and(&mut self) -> Result<f64, String> {
        let mut value = self.parse_bitwise_or()?;
        while self.match_op("&&") {
            value = (value != 0.0 && self.parse_bitwise_or()? != 0.0) as i32 as f64;
        }
        Ok(value)
    }

    fn parse_logical_or(&mut self) -> Result<f64, String> {
        let mut value = self.parse_logical_and()?;
        while self.match_op("||") {
            value = (value != 0.0 || self.parse_logical_and()? != 0.0) as i32 as f64;
        }
        Ok(value)
    }
}

fn milkdrop_frequency_data(scope: &BTreeMap<String, MilkdropValue>) -> Vec<f64> {
    [
        "frequency_data",
        "frequencies",
        "frequency",
        "spectrum",
        "fft",
    ]
    .into_iter()
    .find_map(|name| match scope.get(name) {
        Some(MilkdropValue::Text(value)) => Some(
            value
                .split(',')
                .filter_map(|item| item.trim().parse::<f64>().ok())
                .collect::<Vec<_>>(),
        ),
        Some(MilkdropValue::Number(value)) => Some(vec![*value]),
        None => None,
    })
    .unwrap_or_default()
}

pub fn evaluate_milkdrop_expression(
    expression: &str,
    variables: &BTreeMap<String, MilkdropValue>,
) -> Result<f64, String> {
    let scope = variables
        .iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let tokens = tokenize_milkdrop_expression(expression)?;
    MilkdropExpressionParser::new(tokens, &scope).parse()
}

pub fn evaluate_milkdrop_equations(
    equations: &str,
    variables: &BTreeMap<String, MilkdropValue>,
) -> Result<BTreeMap<String, MilkdropValue>, String> {
    let mut scope = variables
        .iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    for statement in equations
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let Some((name, operator, expression)) = split_milkdrop_assignment(statement) else {
            let _ = evaluate_milkdrop_expression(statement, &scope)?;
            continue;
        };
        let current = milkdrop_number(&scope, &name);
        let next = evaluate_milkdrop_expression(expression, &scope)?;
        let value = match operator {
            "=" => next,
            "+=" => current + next,
            "-=" => current - next,
            "*=" => current * next,
            "/=" => {
                if next == 0.0 {
                    0.0
                } else {
                    current / next
                }
            }
            _ => next,
        };
        scope.insert(name, MilkdropValue::Number(value));
    }
    Ok(scope)
}

fn split_milkdrop_assignment(statement: &str) -> Option<(String, &'static str, &str)> {
    for operator in ["+=", "-=", "*=", "/=", "="] {
        if let Some((raw_name, expression)) = statement.split_once(operator) {
            let name = raw_name.trim();
            if !name.is_empty()
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
                && name
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
            {
                return Some((name.to_ascii_lowercase(), operator, expression.trim()));
            }
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn toggle_rust_milkdrop_visualizer(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let panel = document
        .get_element_by_id("slskr-rust-milkdrop")
        .ok_or_else(|| JsValue::from_str("Rust MilkDrop panel is missing"))?;
    if panel.has_attribute("hidden") {
        panel.remove_attribute("hidden")?;
        start_rust_milkdrop_visualizer(window, document)?;
    } else {
        panel.set_attribute("hidden", "")?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn start_rust_milkdrop_visualizer(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let panel = document
        .get_element_by_id("slskr-rust-milkdrop")
        .ok_or_else(|| JsValue::from_str("Rust MilkDrop panel is missing"))?;
    if panel
        .get_attribute("data-slskr-milkdrop-running")
        .as_deref()
        == Some("true")
    {
        return Ok(());
    }
    panel.set_attribute("data-slskr-milkdrop-running", "true")?;

    mount_rust_milkdrop_buttons(window, document)?;

    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("slskr-milkdrop-canvas")
        .ok_or_else(|| JsValue::from_str("Rust MilkDrop canvas is missing"))?
        .dyn_into()?;
    let context: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("2D canvas is unavailable"))?
        .dyn_into()?;
    let animation_handle: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> =
        Rc::new(RefCell::new(None));
    let animation_handle_for_frame = animation_handle.clone();
    let window_for_frame = window.clone();
    let document_for_frame = document.clone();

    *animation_handle_for_frame.borrow_mut() = Some(Closure::wrap(Box::new(move |time_ms: f64| {
        let Some(panel) = document_for_frame.get_element_by_id("slskr-rust-milkdrop") else {
            return;
        };
        if panel.has_attribute("hidden") {
            return;
        }
        let preset_index = panel
            .get_attribute("data-slskr-milkdrop-preset-index")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let preset_source = RUST_MILKDROP_PRESETS[preset_index % RUST_MILKDROP_PRESETS.len()];
        let preset = parse_rust_milkdrop_preset(preset_source);
        let time = time_ms / 1000.0;
        let bass = (time * 1.9).sin() * 0.5 + 0.5;
        let mid = (time * 1.17 + 1.3).sin() * 0.5 + 0.5;
        let treble = (time * 2.7 + 0.4).sin() * 0.5 + 0.5;
        let frame = rust_milkdrop_frame(&preset, time, bass, mid, treble);
        render_rust_milkdrop_frame(&context, &canvas, &frame, time);
        if let Some(status) = document_for_frame.get_element_by_id("slskr-milkdrop-status") {
            status.set_text_content(Some(&format!(
                "MilkDrop running: bass {:.0}% mid {:.0}% treble {:.0}%",
                bass * 100.0,
                mid * 100.0,
                treble * 100.0
            )));
        }
        if let Some(callback) = animation_handle.borrow().as_ref() {
            let _ = window_for_frame.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut(f64)>));

    if let Some(callback) = animation_handle_for_frame.borrow().as_ref() {
        window.request_animation_frame(callback.as_ref().unchecked_ref())?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn mount_rust_milkdrop_buttons(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-milkdrop-action]")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        if button.has_attribute("data-slskr-mounted") {
            continue;
        }
        button.set_attribute("data-slskr-mounted", "true")?;
        let action = button
            .get_attribute("data-slskr-milkdrop-action")
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                match action.as_str() {
                    "close" => {
                        if let Some(panel) = document.get_element_by_id("slskr-rust-milkdrop") {
                            let _ = panel.set_attribute("hidden", "");
                        }
                        set_player_status(&document, "Rust MilkDrop hidden");
                    }
                    "external" => {
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
                                Err(error) => set_player_status(
                                    &document,
                                    &error.as_string().unwrap_or_else(|| {
                                        "external visualizer request failed".to_string()
                                    }),
                                ),
                            }
                        });
                    }
                    _ => cycle_rust_milkdrop_preset(&document),
                }
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn cycle_rust_milkdrop_preset(document: &web_sys::Document) {
    let Some(panel) = document.get_element_by_id("slskr-rust-milkdrop") else {
        return;
    };
    let current = panel
        .get_attribute("data-slskr-milkdrop-preset-index")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let next = (current + 1) % RUST_MILKDROP_PRESETS.len();
    let _ = panel.set_attribute("data-slskr-milkdrop-preset-index", &next.to_string());
    if let Some(label) = document.get_element_by_id("slskr-milkdrop-preset") {
        label.set_text_content(Some(&rust_milkdrop_preset_name(
            RUST_MILKDROP_PRESETS[next],
        )));
    }
    set_player_status(document, "Rust MilkDrop preset changed");
}

#[cfg(target_arch = "wasm32")]
fn render_rust_milkdrop_frame(
    context: &web_sys::CanvasRenderingContext2d,
    canvas: &web_sys::HtmlCanvasElement,
    frame: &RustMilkdropFrame,
    time: f64,
) {
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    context.set_fill_style_str(&format!("rgba(9, 13, 18, {:.3})", frame.background_alpha));
    context.fill_rect(0.0, 0.0, width, height);

    let (r, g, b) = frame.wave_color;
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let base = width.min(height) * frame.wave_radius * frame.zoom;
    context.save();
    let _ = context.translate(center_x, center_y);
    let _ = context.rotate(frame.rotation + time * 0.08);
    context.set_stroke_style_str(&format!("rgba({r}, {g}, {b}, 0.92)"));
    context.set_line_width(2.0);
    context.begin_path();
    for index in 0..192 {
        let unit = index as f64 / 192.0;
        let angle = unit * std::f64::consts::TAU;
        let radius = base
            * (0.72
                + 0.12 * (time * 1.9 + angle * 3.0).sin()
                + 0.08 * (time * 2.7 + angle * 7.0).cos());
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        if index == 0 {
            context.move_to(x, y);
        } else {
            context.line_to(x, y);
        }
    }
    context.close_path();
    context.stroke();

    context.set_fill_style_str(&format!("rgba({r}, {g}, {b}, 0.14)"));
    context.fill();

    context.set_stroke_style_str("rgba(255, 209, 102, 0.34)");
    context.set_line_width(1.0);
    for ring in 1..=4 {
        context.begin_path();
        let ring_radius = base * ring as f64 * 0.18 * (1.0 + 0.03 * (time + ring as f64).sin());
        let _ = context.arc(0.0, 0.0, ring_radius, 0.0, std::f64::consts::TAU);
        context.stroke();
    }
    context.restore();

    context.set_stroke_style_str("rgba(216, 232, 225, 0.36)");
    context.set_line_width(1.5);
    context.begin_path();
    for index in 0..128 {
        let x = width * index as f64 / 127.0;
        let amp =
            (time * 5.0 + index as f64 * 0.19).sin() * (0.25 + 0.18 * (time * 2.0).sin().abs());
        let y = height * (0.82 + amp * 0.28);
        if index == 0 {
            context.move_to(x, y);
        } else {
            context.line_to(x, y);
        }
    }
    context.stroke();
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
            summary.set_inner_html(&route_workflow_stats_html(
                route_kind(&path),
                Some(&responses),
            ));
        }
        if let Some(page_data) = page_data.as_ref() {
            page_data.set_inner_html(&route_workspace_result_html(&path, &responses));
            mount_workspace_tabs(&document)?;
            mount_data_cards(&document)?;
            mount_native_tables(&document)?;
            mount_native_subviews(&document)?;
            mount_native_actions(&document)?;
            mount_native_filters(&document)?;
            mount_native_sorters(&document)?;
            mount_browser_local_panels(window, &document)?;
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
        assert!(html.contains("Grouped results"));
        assert!(html.contains("Search planner"));
        assert!(html.contains("data-slskr-route-kind=\"Search\""));
        assert!(html.contains("slskr-player-now"));
        assert!(html.contains("slskr-player-transfers"));
        assert!(html.contains("slskr-rust-milkdrop"));
        assert!(html.contains("slskr-milkdrop-canvas"));
        assert!(html.contains("data-slskr-milkdrop-action=\"preset\""));
        assert!(html.contains("/api/v0/searches"));
        assert!(html.contains("slskr-runtime-status"));
        assert!(html.contains("/api/v0/health"));
        assert!(html.contains("slskr-route-view"));
        let system = route_page_html("/system");
        assert!(system.contains("Operator dashboard"));
        assert!(system.contains("Rescan shares"));
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
    fn rust_milkdrop_parser_and_frame_state_are_deterministic() {
        let preset = parse_rust_milkdrop_preset(
            "name=Test\ndecay=0.88\nwave_r=0.2\nwave_g=0.4\nwave_b=0.6\nwave_a=0.9\nwave_scale=1.5\nzoom=1.1\nrot=0.03",
        );

        assert_eq!(preset.decay, 0.88);
        assert_eq!(preset.wave_r, 0.2);
        assert_eq!(preset.wave_g, 0.4);
        assert_eq!(preset.wave_b, 0.6);
        assert_eq!(preset.wave_a, 0.9);
        assert_eq!(preset.wave_scale, 1.5);
        assert_eq!(preset.zoom, 1.1);
        assert_eq!(preset.rot, 0.03);
        assert_eq!(
            rust_milkdrop_preset_name("name=Archive Tunnel\nwave_r=1"),
            "Archive Tunnel"
        );

        let frame = rust_milkdrop_frame(&preset, 2.0, 0.5, 0.25, 0.75);
        assert!(frame.background_alpha > 0.0);
        assert!(frame.rotation.abs() > 0.0);
        assert!(frame.wave_radius > 0.1);
        assert!((0.5..=1.8).contains(&frame.zoom));
        assert!(frame.wave_color.0 >= 95);
        assert!(frame.wave_color.1 >= 120);
        assert!(frame.wave_color.2 >= 200);
    }

    #[test]
    fn rust_milkdrop_preset_parser_matches_js_fixture_shapes() {
        let parsed = parse_milkdrop_preset_set(
            r#"
            // comments are ignored
            [preset00]
            name=slskr smoke preset
            fRating=4.0
            fGammaAdj=1.35
            zoom=1.01
            rot=0
            per_frame_1=q1 = bass_att * 0.2;
            per_frame_2=zoom = zoom + q1;
            per_pixel_1=rot = rot + rad * 0.01;
            warp_shader=shader_body {
            warp_shader_1=  ret = texture(sampler_main, uv).xyz;
            warp_shader_2=}
            comp_shader=shader_body { ret = vec3(q1); }
            shape00_enabled=1
            shape00_sides=5
            shape00_init1=q2=0;
            shape00_per_frame1=q2=q2+0.1;
            sprite00_enabled=1
            sprite00_image=logo.png
            sprite00_init1=q3=0.2;
            sprite00_per_frame1=x=0.5+q3;
            wavecode_0_enabled=1
            wavecode_0_samples=512
            wavecode_0_per_point1=x=sample;
            "#,
            false,
        );
        let preset = &parsed.presets[0];
        assert_eq!(parsed.format, "milk");
        assert_eq!(preset.title, "slskr smoke preset");
        assert_eq!(
            preset.base_values.get("fgammaadj"),
            Some(&MilkdropValue::Number(1.35))
        );
        assert_eq!(
            preset.equations.per_frame,
            "q1 = bass_att * 0.2;\nzoom = zoom + q1;"
        );
        assert_eq!(preset.equations.per_pixel, "rot = rot + rad * 0.01;");
        assert!(preset.warp_shader.contains("texture(sampler_main, uv)"));
        assert_eq!(preset.comp_shader, "shader_body { ret = vec3(q1); }");
        assert_eq!(
            preset.shapes[0].base_values.get("enabled"),
            Some(&MilkdropValue::Number(1.0))
        );
        assert_eq!(
            preset.shapes[0].base_values.get("sides"),
            Some(&MilkdropValue::Number(5.0))
        );
        assert_eq!(preset.shapes[0].equations.init, "q2=0;");
        assert_eq!(preset.shapes[0].equations.frame, "q2=q2+0.1;");
        assert_eq!(
            preset.sprites[0].base_values.get("image"),
            Some(&MilkdropValue::Text("logo.png".to_string()))
        );
        assert_eq!(preset.waves[0].equations.point, "x=sample;");
    }

    #[test]
    fn rust_milkdrop_expression_vm_matches_js_compatibility_cases() {
        let mut vars = BTreeMap::new();
        vars.insert("bass_att".to_string(), MilkdropValue::Number(2.0));
        assert_eq!(
            evaluate_milkdrop_expression("pow(bass_att, 2) + sqr(3)", &vars).unwrap(),
            13.0
        );
        vars.insert("treb".to_string(), MilkdropValue::Number(2.0));
        assert_eq!(
            evaluate_milkdrop_expression("if(above(treb, 1.5), sin(0), 7)", &vars).unwrap(),
            0.0
        );
        assert_eq!(
            evaluate_milkdrop_expression("div(10, 0) + sqrt(-1)", &BTreeMap::new()).unwrap(),
            0.0
        );
        assert_eq!(
            evaluate_milkdrop_expression("sin(pi/2)+log(e)+log10(100)", &BTreeMap::new())
                .unwrap()
                .round(),
            4.0
        );
        assert_eq!(
            evaluate_milkdrop_expression("(7 & 3) + (4 | 1) + (7 ^ 3)", &BTreeMap::new()).unwrap(),
            12.0
        );
        assert_eq!(
            evaluate_milkdrop_expression("(1 << 3) + (8 >> 1)", &BTreeMap::new()).unwrap(),
            12.0
        );
        assert_eq!(
            evaluate_milkdrop_expression("~0 + !0 + !2", &BTreeMap::new()).unwrap(),
            0.0
        );

        let mut equation_vars = BTreeMap::new();
        equation_vars.insert("bass_att".to_string(), MilkdropValue::Number(3.0));
        equation_vars.insert("treb_att".to_string(), MilkdropValue::Number(1.0));
        equation_vars.insert("wave_r".to_string(), MilkdropValue::Number(0.8));
        equation_vars.insert("zoom".to_string(), MilkdropValue::Number(1.0));
        let scope = evaluate_milkdrop_equations(
            "q1 = bass_att * 0.2; zoom += q1; q33 = if(below(treb_att, 2), 7, 9); wave_r *= 0.5;",
            &equation_vars,
        )
        .unwrap();
        assert_eq!(
            scope.get("q1"),
            Some(&MilkdropValue::Number(0.6000000000000001))
        );
        assert_eq!(scope.get("q33"), Some(&MilkdropValue::Number(7.0)));
        assert_eq!(scope.get("wave_r"), Some(&MilkdropValue::Number(0.4)));
        assert!(matches!(
            evaluate_milkdrop_expression("megabuf(1)", &BTreeMap::new()),
            Err(error) if error.contains("Unsupported MilkDrop function")
        ));
    }

    #[test]
    fn rust_milkdrop_expression_vm_samples_fft_data() {
        let mut vars = BTreeMap::new();
        vars.insert(
            "frequency_data".to_string(),
            MilkdropValue::Text("0,128,255,64".to_string()),
        );
        assert_eq!(
            evaluate_milkdrop_expression("get_fft(0.5)", &vars).unwrap(),
            1.0
        );
        vars.insert(
            "frequency_data".to_string(),
            MilkdropValue::Text("0,255,0,0".to_string()),
        );
        vars.insert("sample_rate".to_string(), MilkdropValue::Number(44100.0));
        assert_eq!(
            evaluate_milkdrop_expression("get_fft_hz(5512.5)", &vars).unwrap(),
            1.0
        );
    }

    #[test]
    fn rust_milkdrop_fragment_parser_and_serializer_match_js_shapes() {
        let fragment = parse_milkdrop_fragment(
            r#"
            [shape]
            sides=7
            rad=0.22
            r=1
            per_frame_1=ang=time;
            "#,
            "star.shape",
            "",
        );
        assert_eq!(fragment.fragment_type, "shape");
        assert_eq!(
            fragment.entries[0].base_values.get("enabled"),
            Some(&MilkdropValue::Number(1.0))
        );
        assert_eq!(
            fragment.entries[0].base_values.get("sides"),
            Some(&MilkdropValue::Number(7.0))
        );
        assert_eq!(fragment.entries[0].equations.frame, "ang=time;");
        assert!(serialize_milkdrop_fragment(&fragment.entries[0], "shape")
            .contains("per_frame_1=ang=time;"));

        let wave = parse_milkdrop_fragment(
            "samples=64\nspectrum=1\nper_point_1=x=i;\nper_point_2=y=sample;",
            "spectrum.wave",
            "",
        );
        assert_eq!(wave.fragment_type, "wave");
        assert_eq!(
            wave.entries[0].base_values.get("samples"),
            Some(&MilkdropValue::Number(64.0))
        );
        assert_eq!(wave.entries[0].equations.point, "x=i;\ny=sample;");

        let prefixed = parse_milkdrop_fragment(
            "shape00_enabled=1\nshape00_sides=4\nshape00_per_frame1=rad=0.25+0.05*sin(time);",
            "prefixed.shape",
            "",
        );
        assert_eq!(
            prefixed.entries[0].base_values.get("sides"),
            Some(&MilkdropValue::Number(4.0))
        );
        assert_eq!(
            prefixed.entries[0].equations.frame,
            "rad=0.25+0.05*sin(time);"
        );
    }

    #[test]
    fn rust_milkdrop_preset_serializer_keeps_custom_primitives() {
        let parsed = parse_milkdrop_preset_set(
            "name=Serializable\nwave_r=1\nshape00_enabled=1\nshape00_sides=5\nwavecode_0_enabled=1\nwavecode_0_samples=16\nwavecode_0_per_point1=x=i;",
            false,
        );
        let serialized = serialize_milkdrop_preset_set(&parsed);
        assert!(serialized.contains("name=Serializable"));
        assert!(serialized.contains("shape00_sides=5"));
        assert!(serialized.contains("wavecode_0_samples=16"));
        assert!(serialized.contains("wavecode_0_per_point_1=x=i;"));
    }

    #[test]
    fn rust_milkdrop_shader_translator_handles_glsl_safe_subset() {
        assert_eq!(
            translate_milkdrop_shader_expression(
                "ret = tex2D(sampler_main, uv).rgb * vec3(0.5, 1.0, 0.25);"
            ),
            "texture(previousFrame, uv).rgb * vec3(0.5, 1.0, 0.25)"
        );
        let shader = create_translated_milkdrop_fragment_shader(
            "ret = saturate(vec3(uv.x, uv.y, sin(time)));",
        );
        assert!(shader.contains("uniform sampler2D previousFrame;"));
        assert!(shader.contains("uniform float fftBins[64];"));
        assert!(shader.contains("uniform float waveformBins[64];"));
        assert!(shader.contains("uniform float aspect;"));
        assert!(shader.contains("float rad = length(centeredUv);"));
        assert!(shader.contains("float ang = atan(centeredUv.y, centeredUv.x);"));
        assert!(shader.contains("float get_fft(float position)"));
        assert!(shader.contains("uniform float q64;"));
        assert!(shader.contains("vec3 ret = vec3(clamp01(vec3(uv.x, uv.y, sin(time))));"));
        assert!(analyze_milkdrop_shader_support("ret = vec3(color);").supported);
    }

    #[test]
    fn rust_milkdrop_shader_translator_handles_temps_textures_and_conditionals() {
        let shader = create_translated_milkdrop_fragment_shader(
            r#"
            float2 shifted = uv + float2(frac(time), fmod(time, 1.0));
            float3 tinted = lerp(color, tex2D(sampler_main, shifted).rgb, 0.25);
            float energy = rsqrt(max(get_fft(0.25), 0.001));
            ret = tinted * vec3(energy, atan2(shifted.y, shifted.x), 1.0);
            "#,
        );
        assert!(shader.contains("vec2 shifted = uv + vec2(fract(time), mod(time, 1.0));"));
        assert!(
            shader.contains("vec3 tinted = mix(color, texture(previousFrame, shifted).rgb, 0.25);")
        );
        assert!(shader.contains("float energy = inversesqrt(max(get_fft(0.25), 0.001));"));
        assert_eq!(
            translate_milkdrop_shader_expression("float3 tint = vec3(1.0); ret = tint;"),
            "tint"
        );

        assert_eq!(
            get_milkdrop_shader_texture_samplers(
                "ret = tex2D(sampler_noise, uv).rgb + tex2D(album_art, uv).rgb;"
            ),
            vec!["sampler_noise".to_string(), "album_art".to_string()]
        );
        let textured = create_translated_milkdrop_fragment_shader(
            "float3 noise = tex2D(sampler_noise, uv).rgb; ret = noise;",
        );
        assert!(textured.contains("uniform sampler2D shaderTexture0;"));
        assert!(textured.contains("vec3 noise = texture(shaderTexture0, uv).rgb;"));

        let conditional = create_translated_milkdrop_fragment_shader(
            "if (bass_att > 1.0) { ret = tex2D(sampler_noise, uv).rgb; } else { ret = vec3(x, y, rad); }",
        );
        assert!(conditional.contains("vec3 ret = vec3((bass_att > 1.0)"));
        assert!(conditional.contains("shaderTexture0"));
        assert!(conditional.contains("vec3(x, y, rad)"));
        assert_eq!(
            translate_milkdrop_shader_expression(
                "if (q1 > 0.5) ret = vec3(1.0); else ret = vec3(0.0);"
            ),
            "(q1 > 0.5) ? (vec3(1.0)) : (vec3(0.0))"
        );
    }

    #[test]
    fn rust_milkdrop_shader_translator_rejects_unsafe_subset() {
        assert_eq!(
            translate_milkdrop_shader_expression("for (;;) { ret = vec3(1.0); }"),
            ""
        );
        assert_eq!(
            translate_milkdrop_shader_expression("ret = unknown[index];"),
            ""
        );
        assert_eq!(
            translate_milkdrop_shader_expression("float3 tint; ret = tint;"),
            ""
        );
        assert!(!analyze_milkdrop_shader_support("if (uv.x > 0.5) ret = vec3(1.0);").supported);
        assert_eq!(
            translate_milkdrop_shader_expression(
                "float3 tint = vec3(1.0); ret = tint; tint *= 0.5;"
            ),
            ""
        );
        assert_eq!(
            translate_milkdrop_shader_expression("ret = vec3(1.0); ret = vec3(0.0);"),
            ""
        );
    }

    #[test]
    fn rust_milkdrop_shader_translator_generates_wgsl_subset() {
        let shader = create_translated_milkdrop_wgsl_shader(
            r#"
            float3 tint = saturate(vec3(q1, bass_att, uv.x));
            tint *= tex2D(sampler_main, uv).rgb;
            ret = tint + vec3(time * 0.01, get_fft(0.25), get_waveform(0.5));
            "#,
        );
        assert!(shader.contains("@fragment"));
        assert!(shader.contains("q64: f32"));
        assert!(shader.contains("fft63: f32"));
        assert!(shader.contains("waveform63: f32"));
        assert!(shader.contains("let q1 = uniforms.q1;"));
        assert!(shader.contains("fn get_fft(position: f32) -> f32"));
        assert!(shader.contains("var tint = clamp01v3(vec3f(q1, bass_att, uv.x));"));
        assert!(shader.contains("tint *= textureSample(previousFrame, previousSampler, uv).rgb;"));
        assert!(shader.contains(
            "let ret = vec3f(tint + vec3f(time * 0.01, get_fft(0.25), get_waveform(0.5)));"
        ));

        let textured =
            create_translated_milkdrop_wgsl_shader("ret = tex2D(sampler_noise, uv).rgb;");
        assert!(textured.contains("@group(0) @binding(3) var shaderTexture0: texture_2d<f32>;"));
        assert!(textured.contains("textureSample(shaderTexture0, shaderTextureSampler, uv).rgb"));
        assert!(
            analyze_milkdrop_webgpu_shader_support("ret = tex2D(sampler_noise, uv).rgb;").supported
        );
        assert_eq!(
            create_translated_milkdrop_wgsl_shader("ret = q1 > 0.5 ? vec3(1.0) : vec3(0.0);"),
            ""
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
    fn rust_browser_local_system_panels_match_react_surfaces() {
        assert_eq!(experience_preferences().len(), 18);
        let defaults = default_experience_preferences();
        assert_eq!(
            defaults
                .get("searchRankingProfile")
                .and_then(|value| value.as_str()),
            Some("balanced")
        );
        assert_eq!(
            defaults
                .get("playerKeyboardShortcuts")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        let report = experience_preferences_report(&defaults);
        assert!(report.contains("slskr experience preferences"));
        assert!(report.contains("Player: queue_auto_fill=false"));
        assert!(
            experience_settings_panel_html().contains("data-slskr-pref=\"playerRadioSeedMode\"")
        );

        assert_eq!(automation_recipes().len(), 7);
        let mut state = serde_json::Map::new();
        state.insert(
            "wishlist-retry".to_string(),
            serde_json::json!({
                "enabled": true,
                "lastDryRunAt": "browser-local"
            }),
        );
        let (total, enabled, disabled) = automation_summary_from_state(&state);
        assert_eq!((total, enabled, disabled), (7, 4, 3));
        let dry_run = automation_dry_run_report(automation_recipes()[3], "browser-local");
        assert_eq!(dry_run["recipeId"], "wishlist-retry");
        assert_eq!(dry_run["executed"], false);
        let history = automation_history_report(&state);
        assert!(history.contains("slskr automation review history"));
        assert!(history.contains("Wishlist Retry"));
        assert!(
            automation_center_panel_html().contains("data-slskr-recipe=\"library-health-scan\"")
        );
    }

    #[test]
    fn rust_search_planner_matches_react_search_helpers() {
        let strong = serde_json::json!({
            "files": [{
                "bitDepth": 16,
                "filename": "Boards of Canada/Music Has The Right/01 Wildlife Analysis.flac",
                "sampleRate": 44100,
                "size": 24000000
            }],
            "hasFreeUploadSlot": true,
            "queueLength": 0,
            "uploadSpeed": 4000000,
            "username": "good-peer"
        });
        let rank = rank_search_candidate(
            &strong,
            "boards canada wildlife analysis",
            "lossless-exact",
            Some(&serde_json::json!({"successfulDownloads": 4, "failedDownloads": 0})),
            None,
            None,
        );
        assert!(rank.score >= 80);
        assert!(rank.reasons.contains(&"strong filename match".to_string()));
        assert!(rank.reasons.contains(&"mostly lossless files".to_string()));
        assert!(rank.reasons.contains(&"free upload slot".to_string()));

        let weak = serde_json::json!({
            "files": [{"bitRate": 128, "filename": "misc/upload/track.mp3", "size": 2000000}],
            "hasFreeUploadSlot": false,
            "queueLength": 8,
            "uploadSpeed": 64000,
            "username": "rough-peer"
        });
        let weak_rank = rank_search_candidate(
            &weak,
            "boards canada wildlife analysis",
            "lossless-exact",
            Some(&serde_json::json!({"successfulDownloads": 0, "failedDownloads": 4})),
            None,
            None,
        );
        assert!(weak_rank.score < 35);
        assert!(weak_rank.reasons.contains(&"long queue".to_string()));
        assert!(weak_rank
            .reasons
            .contains(&"poor download history".to_string()));

        let fast = serde_json::json!({
            "files": [{"bitRate": 320, "filename": "Stereolab/Peng!/Super Falling Star.mp3", "size": 8000000}],
            "hasFreeUploadSlot": true,
            "queueLength": 1,
            "uploadSpeed": 1000000
        });
        let fast_rank = rank_search_candidate(
            &fast,
            "stereolab super falling star",
            "fast-good-enough",
            None,
            None,
            None,
        );
        assert!(fast_rank.score >= 60);
        assert!(fast_rank
            .reasons
            .contains(&"high bitrate fast-good-enough candidate".to_string()));

        let responses = vec![
            serde_json::json!({
                "files": [{"filename": "Artist/Album/01 Track.flac", "size": 24000000}],
                "primarySource": "mesh",
                "sourceProviders": ["mesh"],
                "username": "best-peer"
            }),
            serde_json::json!({
                "files": [{"filename": "Different Root/01 Track.flac", "size": 24000000}],
                "primarySource": "soulseek",
                "sourceProviders": ["soulseek"],
                "username": "backup-peer"
            }),
            serde_json::json!({
                "files": [{"filename": "Artist/Album/02 Other.flac", "size": 22000000}],
                "username": "other-peer"
            }),
        ];
        let (folded, groups) = deduplicate_search_response_groups(&responses, true);
        assert_eq!(folded, 1);
        assert_eq!(groups[0].candidate_count, 2);
        assert_eq!(groups[0].folded_count, 1);
        assert_eq!(
            groups[0].usernames,
            vec!["backup-peer".to_string(), "best-peer".to_string()]
        );

        let preview = build_search_action_preview(
            &serde_json::json!({
                "hasFreeUploadSlot": false,
                "queueLength": 7,
                "sourceProviders": ["pod", "scene"],
                "username": "peer"
            }),
            &[
                serde_json::json!({"filename": "Artist/Album/01 Track.flac", "size": 20}),
                serde_json::json!({"filename": "Artist/Album/02 Track.flac", "locked": true, "size": 30}),
            ],
            Some(&SearchCandidateRank {
                reasons: Vec::new(),
                score: 38,
            }),
            Some(&serde_json::json!({
                "override": {"mode": "ignore", "note": "Known private peer."},
                "score": -6
            })),
            "download",
        );
        assert_eq!(preview.file_count, 2);
        assert_eq!(preview.locked_count, 1);
        assert!(preview
            .warnings
            .contains(&"No free upload slot is currently advertised".to_string()));
        let text = format_search_action_preview(&preview);
        assert!(text.contains("Action: download"));
        assert!(text.contains("Candidate score: 38/100"));
        assert!(
            search_planner_report("public domain theme", "lossless-exact", true)
                .contains("Search planner")
        );
    }

    #[test]
    fn static_index_supports_direct_nested_route_loads() {
        assert!(STATIC_INDEX.contains("href=\"/styles.css\""));
        assert!(STATIC_INDEX.contains("src=\"/slskr_web_bootstrap.js\""));
        assert!(!STATIC_INDEX.contains("href=\"./styles.css\""));
        assert!(!STATIC_INDEX.contains("src=\"./slskr_web_bootstrap.js\""));
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
        assert!(html.contains("Workspace"));
        assert!(html.contains("Download queue"));
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
            ("/searches", "Grouped results", "Search", "search-native"),
            (
                "/discovery-graph",
                "Discovery graph",
                "Build graph",
                "discovery-graph-native",
            ),
            (
                "/playlist-intake",
                "Playlist parser",
                "Preview playlist",
                "playlist-intake-native",
            ),
            (
                "/wishlist",
                "Wanted searches",
                "Add wanted search",
                "wishlist-native",
            ),
            (
                "/downloads",
                "Download queue",
                "Download",
                "transfers-native",
            ),
            (
                "/uploads",
                "Upload queue",
                "Clear completed",
                "transfers-native",
            ),
            ("/messages", "Conversations", "Reply", "messaging-native"),
            ("/users", "User directory", "Watch", "users-native"),
            (
                "/contacts",
                "Contact manager",
                "Add contact",
                "contacts-native",
            ),
            ("/solid", "Solid status", "Connect identity", "solid-native"),
            (
                "/collections",
                "Collection library",
                "Create collection",
                "collections-native",
            ),
            (
                "/sharegroups",
                "Share groups",
                "Issue token",
                "sharegroups-native",
            ),
            (
                "/shared",
                "Inbound shares",
                "Open collection",
                "shared-native",
            ),
            ("/browse", "Peer browser", "Browse", "browse-native"),
            (
                "/system",
                "Operator dashboard",
                "Rescan shares",
                "system-native",
            ),
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
            assert!(html.contains("slskr-native-table"));
            assert!(html.contains("aria-keyshortcuts=\"Enter Space ArrowUp ArrowDown Home End\""));
            assert!(html.contains("data-slskr-native-sort=\"0\""));
            assert!(html.contains("data-slskr-native-sort-0="));
            assert!(html.contains("data-slskr-native-index="));
            assert!(html.contains("slskr-native-inspector"));
            assert!(html.contains("data-slskr-native-inspector-title"));
            assert!(html.contains("data-slskr-native-select"));
            assert!(html.contains("slskr-native-selection-status"));
            assert!(html.contains("slskr-toast-region"));
            assert!(html.contains("slskr-legacy-workflow"));
            assert!(html.contains("Additional workflow detail"));
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
                .unwrap_or_else(|| panic!("missing slskd compatibility panel for route {path}"));
            let workflow_tabs_index = html
                .find("slskr-workflow-tabs")
                .unwrap_or_else(|| panic!("missing workflow tabs for route {path}"));
            let native_index = html
                .find("slskr-native-workspace")
                .unwrap_or_else(|| panic!("missing native workspace for route {path}"));
            assert!(
                parity_index < workflow_tabs_index,
                "route {path} should show compatibility content before Rust workflow tabs"
            );
            assert!(
                native_index < workflow_tabs_index,
                "route {path} should show native page body before Rust workflow tabs"
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
                    path: "/shared",
                    surface: "sharegroups",
                },
                r#"[{"title":"Shared Collection","owner":"peer-owner","permissions":"read"}]"#,
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
    fn rust_ui_parity_ledger_tracks_closure_instead_of_stale_gaps() {
        let ledger = include_str!("../../../docs/rust-ui-parity-ledger.md");
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
            ("/solid", "Resolve WebID", "Resolve WebID"),
            ("/collections", "Add Item", "Add Library Item"),
            ("/collections", "Share", "Create Share Grant"),
            ("/sharegroups", "Create Share Grant", "Create Share Grant"),
            ("/sharegroups", "Update Share Grant", "Update Share Grant"),
            ("/sharegroups", "Issue Token", "Issue Share Token"),
            ("/shared", "Open", "Backfill Share Grant"),
            ("/shared", "Stream", "Backfill Share Grant"),
            ("/shared", "Backfill", "Backfill Share Grant"),
            ("/shared", "Copy token", "Issue Share Token"),
            ("/shared", "Leave share", "Delete Share Grant"),
            ("/browse", "Download Selected", "Queue Download"),
            ("/system", "Check for Updates", "Check for Updates"),
            ("/system", "Get Privileges", "Get Privileges"),
            ("/system", "Diagnostic Bundle", "Diagnostic Bundle"),
            ("/system", "Setup Health", "Setup Health"),
            ("/system", "Shut Down", "Shut Down"),
            ("/system", "Restart", "Restart"),
            ("/system", "Vacuum database", "Vacuum Database"),
        ];

        for (path, label, expected_action) in expectations {
            let action = route_action_for_native_label(path, label)
                .unwrap_or_else(|| panic!("{path} {label} should resolve"));
            assert_eq!(action.label, expected_action);
        }
    }

    #[test]
    fn native_action_fallbacks_are_domain_specific() {
        assert_eq!(
            native_action_fallback(ActionBody::SearchText),
            "public domain jazz"
        );
        assert_eq!(
            native_action_fallback(ActionBody::FeedPreview),
            "Public Domain Jazz - Demo Track"
        );
        assert_eq!(
            native_action_fallback(ActionBody::DownloadFiles),
            "Remote/Song.mp3"
        );
        assert_eq!(native_action_fallback(ActionBody::BrowseDirectory), "/");
        assert_eq!(native_action_fallback(ActionBody::Username), "peer1");
        assert!(native_action_fallback(ActionBody::None).is_empty());
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
            "ListenBrainz",
            "Raw metrics",
            "developer-only",
            "Run Replacement Searches",
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
            "Estimated queue impact",
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
        ] {
            assert!(
                messages.contains(value),
                "messages panel should contain {value}"
            );
        }

        let sharing = route_page_html("/sharegroups");
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
            assert!(
                page.contains("slskr-native-final-parity"),
                "{path} should expose final old-WebUI workflow parity controls"
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
            "data-slskr-graph-final-parity",
            "Save Branch",
            "Weight Edges",
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
            "data-slskr-playlist-final-parity",
            "Upload Playlist",
            "Correct Row",
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
            "data-slskr-wishlist-final-parity",
            "Check Quota",
            "Persist Inbox",
        ] {
            assert!(
                wishlist.contains(value),
                "wishlist workspace should contain {value}"
            );
        }

        let downloads = route_page_html("/downloads");
        for value in [
            "Transfer Group",
            "No downloads selected",
            "Retry All",
            "data-slskr-downloads-final-parity",
            "Group by Peer",
        ] {
            assert!(
                downloads.contains(value),
                "downloads workspace should contain {value}"
            );
        }
        assert!(downloads.contains("data-slskr-transfer-state-control"));

        let uploads = route_page_html("/uploads");
        for value in [
            "data-slskr-uploads-final-parity",
            "Edit Policy",
            "Group by Peer",
        ] {
            assert!(
                uploads.contains(value),
                "uploads workspace should contain {value}"
            );
        }

        let users = route_page_html("/users");
        for value in [
            "User Detail",
            "No user selected",
            "Save note",
            "data-slskr-users-final-parity",
            "Open Context Menu",
        ] {
            assert!(
                users.contains(value),
                "users workspace should contain {value}"
            );
        }

        let contacts = route_page_html("/contacts");
        for value in [
            "All Contacts",
            "No contact selected",
            "Refresh Nearby",
            "data-slskr-contacts-final-parity",
            "Create QR Invite",
            "Scan QR Image",
        ] {
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
            "data-slskr-solid-final-parity",
            "Connect Session",
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
            "data-slskr-system-final-parity",
            "Operator Tab Parity",
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
            "Unread 0",
            "Delete Conversation",
            "pod channel",
            "data-slskr-messages-final-parity",
            "Open Window",
            "Restore Draft",
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
            "data-slskr-browse-final-parity",
            "Restore Session",
            "Persist Breadcrumb",
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
            "data-slskr-collections-final-parity",
            "Persist Draft",
            "Pick Audience",
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
            "data-slskr-sharegroups-final-parity",
            "Remove Member",
            "Revoke Token",
        ] {
            assert!(
                sharegroups.contains(value),
                "share groups workspace should contain {value}"
            );
        }

        let shared = route_page_html("/shared");
        for value in [
            "Shared Manifest",
            "file-level access preview",
            "data-slskr-native-preview-title",
            "Backfill selected collection",
            "Leave share",
            "data-slskr-shared-final-parity",
            "Copy Exact Token",
            "Owner contact",
        ] {
            assert!(
                shared.contains(value),
                "shared workspace should contain {value}"
            );
        }
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
            (
                "/shared",
                &["Stream", "Backfill", "Copy token", "Leave share"],
            ),
            ("/browse", &["Download Selected", "Open a New Browse Tab"]),
            (
                "/system",
                &["Rescan shares", "Vacuum database", "Diagnostic Bundle"],
            ),
        ];

        for (path, labels) in expectations {
            let html = route_page_html(path);
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
                    "Leave share",
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

    #[test]
    fn native_shell_contains_in_app_confirmation_modal_styles() {
        let html = route_page_html("/downloads");
        assert!(html.contains("Cancel"));
        assert!(html.contains("Remove"));
        let css = include_str!("../static/styles.css");
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
    fn native_row_actions_are_marked_for_selected_row_execution() {
        let html = route_page_html("/browse");
        assert!(html.contains(r#"data-slskr-native-row-action="Download Selected""#));
        assert!(html.contains(r#"data-slskr-native-row-action="Open a New Browse Tab""#));
        assert!(html.contains(r#"data-slskr-native-title="/Music/Open Sessions""#));
        assert!(html.contains(r#"data-slskr-native-resource-id="row-1""#));
        let users = route_page_html("/users");
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
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/transfers/downloads",
                    surface: "transfers",
                },
                body: r#"[{"id":77,"username":"peer2","filename":"Remote/Song.mp3","state":"Queued","progress":0.5}]"#.to_string(),
            }],
        );
        assert!(downloads.contains(r#"data-slskr-native-filename="Remote/Song.mp3""#));
        assert!(downloads.contains(r#"data-slskr-native-peer="peer2""#));
        assert!(downloads
            .contains(r#"data-slskr-native-transfer-state="Queued / 50% / 0 B/s / id=77""#));
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

        let wishlist = route_page_html("/wishlist");
        assert!(wishlist.contains("data-slskr-native-search-filter"));
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
                body: r#"[{"id":"wish-7","searchText":"rare live set","filter":"flac","enabled":true}]"#
                    .to_string(),
            }],
        );
        assert!(live_wishlist.contains(r#"data-slskr-native-wishlist-id="wish-7""#));
        assert!(live_wishlist
            .contains(r#"data-slskr-native-action-summary="Run wishlist wish-7: rare live set""#));
        assert!(live_wishlist.contains(
            r#"data-slskr-native-detail-list="Wanted search: rare live set | Filter: flac | Automation: enabled=true / auto=false / id=wish-7 | Next action: Run""#
        ));
        assert!(live_wishlist
            .contains(r#"data-slskr-native-action-menu="Run | Run Enabled | Copy Review""#));
        assert!(live_wishlist.contains("data-slskr-wishlist-row-controls"));
        assert!(live_wishlist.contains(r#"aria-label="Enabled rare live set" checked"#));

        let sharing = route_page_html("/sharegroups");
        assert!(sharing.contains("data-slskr-native-share-group"));
        assert!(sharing.contains("data-slskr-native-member-count"));
        assert!(sharing.contains("data-slskr-share-group-row-controls"));

        let shared = route_page_html("/shared");
        assert!(shared.contains("data-slskr-native-owner"));
        assert!(shared.contains("data-slskr-native-permissions"));
        assert!(shared.contains("data-slskr-inbound-permission-controls"));

        let browse = route_page_html("/browse");
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
            concrete_action_path_with_target_and_id(
                "/downloads",
                transfer,
                Some("peer2"),
                Some("77")
            ),
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

        let browse = route_action_for_native_label("/browse", "Open a New Browse Tab")
            .expect("browse action");
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
    fn native_workspaces_parse_nested_live_domain_payloads() {
        let downloads = route_workspace_result_html(
            "/downloads",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/transfers/downloads",
                    surface: "transfers",
                },
                body: r#"[{"username":"parent-peer","state":"InProgress","bytesPerSecond":2048,"eta":"00:42","files":[{"filename":"Album/Track.flac","progress":0.625}]}]"#
                    .to_string(),
            }],
        );
        assert!(downloads.contains("Album/Track.flac"));
        assert!(downloads.contains("parent-peer"));
        assert!(downloads.contains("InProgress / 62% / 2048 B/s / ETA 00:42"));

        let browse = route_workspace_result_html(
            "/browse",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/users/:username/browse",
                    surface: "browse",
                },
                body: r#"{"directories":[{"name":"Music","isDirectory":true,"files":[{"filename":"Music/live.mp3","size":321}]}],"root":{"files":[{"name":"root.flac","fileSize":654}]}}"#
                    .to_string(),
            }],
        );
        assert!(browse.contains("Music"));
        assert!(browse.contains("folder"));
        assert!(browse.contains("Music/live.mp3"));
        assert!(browse.contains("root.flac"));

        let collections = route_workspace_result_html(
            "/collections",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/collections",
                    surface: "collections",
                },
                body:
                    r#"[{"title":"Road Trips","kind":"playlist","items":[{"id":"1"},{"id":"2"}]}]"#
                        .to_string(),
            }],
        );
        assert!(collections.contains("Road Trips"));
        assert!(collections.contains("2 items"));

        let sharegroups = route_workspace_result_html(
            "/sharegroups",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/sharegroups",
                    surface: "collections",
                },
                body: r#"[{"name":"Trusted","members":[{"username":"peer1"},{"username":"peer2"}],"createdAt":"today"}]"#
                    .to_string(),
            }],
        );
        assert!(sharegroups.contains("Trusted"));
        assert!(sharegroups.contains("2 members"));

        let shared = route_workspace_result_html(
            "/shared",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/shared",
                    surface: "collections",
                },
                body: r#"[{"collection":{"title":"Inbox"},"owner":{"username":"sender"},"grant":{"permissions":"read/write"}}]"#
                    .to_string(),
            }],
        );
        assert!(shared.contains("Inbox"));
        assert!(shared.contains("sender"));
        assert!(shared.contains("read/write"));
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
}
