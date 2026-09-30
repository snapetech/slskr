use wasm_bindgen::prelude::*;

#[cfg(any(test, target_arch = "wasm32"))]
use rustymilk_core::*;

#[cfg(any(test, target_arch = "wasm32"))]
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use std::collections::BTreeSet;

#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_arch = "wasm32")]
pub use rustymilk_wasm::{rustymilk_renderer, WasmRustyMilkEngine};

#[cfg(target_arch = "wasm32")]
fn clamp_range(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

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
    ContactDiscovery,
    ContactInvite,
    DownloadFiles,
    EnabledFalse,
    EnabledTrue,
    FeedPreview,
    InviteRequest,
    JsonString,
    LibraryPath,
    MusicBrainzTarget,
    NameDescription,
    Permissions,
    RoomMessage,
    SearchText,
    ShareGrant,
    ShareGroupMember,
    SongIdSource,
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
            title: "Shared with Me",
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
            path: "/searches/:id",
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
            path: "/downloads/requests",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "GET",
            path: "/destinations",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "GET",
            path: "/destinations/default",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "GET",
            path: "/config/download-filter",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/config/download-filter",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "GET",
            path: "/downloads/requests/:id",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "PATCH",
            path: "/downloads/requests/:id/name",
            surface: "transfers",
        },
        ApiEndpoint {
            method: "POST",
            path: "/downloads/requests/:id/cancel",
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
            path: "/private-message-auto-response",
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
            method: "POST",
            path: "/profile/invite",
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
            path: "/collections/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/collections/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/collections/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
            path: "/collections/:id/items",
            surface: "collections",
        },
        ApiEndpoint {
            method: "POST",
            path: "/collections/:id/items",
            surface: "collections",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/collections/items/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/collections/items/:id",
            surface: "collections",
        },
        ApiEndpoint {
            method: "DELETE",
            path: "/collections/:id/items/:itemId",
            surface: "collections",
        },
        ApiEndpoint {
            method: "PUT",
            path: "/collections/:id/items/:itemId",
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
            method: "GET",
            path: "/share-grants/:id/manifest",
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
            path: "/config",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/options/yaml",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/options/yaml/location",
            surface: "system",
        },
        ApiEndpoint {
            method: "POST",
            path: "/options/yaml/validate",
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
            path: "/source-providers",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/integrations/lidarr/status",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/integrations/lidarr/sync/status",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/integrations/lidarr/manualimport/history",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/musicbrainz/albums/completion",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/musicbrainz/release-radar/subscriptions",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/songid/runs",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/solid/status",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/bridge/status",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/transfers/speeds",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/mesh/stats",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/security/dashboard",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/items",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/health/issues",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/health/issues/by-type",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/health/issues/by-artist",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/health/issues/by-release",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/library/health/issues/by-codec",
            surface: "system",
        },
        ApiEndpoint {
            method: "POST",
            path: "/library/health/scans",
            surface: "system",
        },
        ApiEndpoint {
            method: "POST",
            path: "/library/health/issues/fix",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/quarantine-jury/audit",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/quarantine-jury/requests",
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
            body: ActionBody::InviteRequest,
            label: "Create Invite",
            method: "POST",
            path: "/profile/invite",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Refresh Nearby",
            method: "GET",
            path: "/contacts/nearby",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::ContactDiscovery,
            label: "Add Discovery Contact",
            method: "POST",
            path: "/contacts/from-discovery",
            surface: "identity",
        },
        RouteAction {
            body: ActionBody::ContactInvite,
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
            label: "Update Collection",
            method: "PUT",
            path: "/collections/:id",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Delete Collection",
            method: "DELETE",
            path: "/collections/:id",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Open Collection",
            method: "GET",
            path: "/collections/:id",
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
            label: "Open Shared Manifest",
            method: "GET",
            path: "/share-grants/:id/manifest",
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
            label: "Add Item to Collection",
            method: "POST",
            path: "/collections/:id/items",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Remove Collection Item",
            method: "DELETE",
            path: "/collections/:collectionId/items/:itemId",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::CollectionItem,
            label: "Update Collection Item",
            method: "PUT",
            path: "/collections/:collectionId/items/:itemId",
            surface: "collections",
        },
        RouteAction {
            body: ActionBody::CollectionItem,
            label: "Create Library Item",
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
            body: ActionBody::MusicBrainzTarget,
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
            body: ActionBody::SongIdSource,
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
            label: "Check Lidarr",
            method: "GET",
            path: "/integrations/lidarr/status",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Refresh Lidarr Sync",
            method: "GET",
            path: "/integrations/lidarr/sync/status",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::MusicBrainzTarget,
            label: "Track MusicBrainz Target",
            method: "POST",
            path: "/musicbrainz/targets",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Refresh MusicBrainz",
            method: "GET",
            path: "/musicbrainz/release-radar/subscriptions",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::SongIdSource,
            label: "Start SongID Run",
            method: "POST",
            path: "/songid/runs",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::LibraryPath,
            label: "Start Library Health Scan",
            method: "POST",
            path: "/library/health/scans",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Fix Library Issues",
            method: "POST",
            path: "/library/health/issues/fix",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Refresh SongID",
            method: "GET",
            path: "/songid/runs",
            surface: "system",
        },
        RouteAction {
            body: ActionBody::None,
            label: "Setup Health",
            method: "GET",
            path: "/health",
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
