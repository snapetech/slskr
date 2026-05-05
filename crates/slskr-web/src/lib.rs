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
    ConversationMessage,
    DownloadFiles,
    EnabledFalse,
    EnabledTrue,
    JsonString,
    RoomMessage,
    SearchText,
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
            path: "/contacts",
            surface: "identity",
        },
        ApiEndpoint {
            method: "GET",
            path: "/collections",
            surface: "collections",
        },
        ApiEndpoint {
            method: "GET",
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
            path: "/telemetry/metrics",
            surface: "system",
        },
        ApiEndpoint {
            method: "GET",
            path: "/options",
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
    let search_id = route_param_value(route_path, "1");
    endpoint_url(endpoint.path)
        .replace(":id", &search_id)
        .replace(":username", "peer1")
}

pub fn concrete_action_path(route_path: &str, action: RouteAction) -> String {
    let search_id = route_param_value(route_path, "1");
    endpoint_url(action.path)
        .replace(":id", &search_id)
        .replace(":username", "peer1")
        .replace(":roomName", "contract-room")
}

pub fn action_body_from_value(body: ActionBody, value: &str) -> Option<String> {
    let value = value.trim();
    match body {
        ActionBody::None => None,
        ActionBody::BrowseDirectory => Some(format!(
            r#"{{"directory":"{}"}}"#,
            escape_json_string(value)
        )),
        ActionBody::DownloadFiles => Some(format!(
            r#"[{{"filename":"{}","size":99}}]"#,
            escape_json_string(value)
        )),
        ActionBody::EnabledFalse => Some(r#"{"enabled":false}"#.to_string()),
        ActionBody::EnabledTrue => Some(r#"{"enabled":true}"#.to_string()),
        ActionBody::ConversationMessage | ActionBody::JsonString => {
            Some(format!(r#""{}""#, escape_json_string(value)))
        }
        ActionBody::RoomMessage => Some(format!(r#""{}""#, escape_json_string(value))),
        ActionBody::SearchText => Some(format!(
            r#"{{"searchText":"{}"}}"#,
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
        ActionBody::DownloadFiles => {
            r#"<input class="slskr-action-input" data-slskr-action-input="DownloadFiles" value="Remote/Song.mp3" placeholder="Filename">"#.to_string()
        }
        ActionBody::EnabledFalse | ActionBody::EnabledTrue => String::new(),
        ActionBody::JsonString => {
            r#"<input class="slskr-action-input" data-slskr-action-input="JsonString" value="contract-room" placeholder="Name">"#.to_string()
        }
        ActionBody::RoomMessage => {
            r#"<input class="slskr-action-input" data-slskr-action-input="RoomMessage" value="hello room" placeholder="Message">"#.to_string()
        }
        ActionBody::SearchText => {
            r#"<input class="slskr-action-input" data-slskr-action-input="SearchText" value="public domain jazz" placeholder="Search text">"#.to_string()
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
        r#"<section class="slskr-route-page" data-route="{path}"><header><p class="slskr-kicker">{surface}</p><h2>{title}</h2><p>{description}</p></header><div class="slskr-route-summary"><h3>Summary</h3><ul id="slskr-route-summary">{summary}</ul></div><div class="slskr-route-columns"><div><h3>Route Shape</h3><ul>{routes}</ul></div><div><h3>API Surface</h3><ul>{endpoints}</ul></div></div><div class="slskr-route-actions"><h3>Actions</h3><ul id="slskr-route-actions">{actions}</ul><p id="slskr-action-status" aria-live="polite"></p></div><div class="slskr-route-live"><h3>Live Route Data</h3><ul id="slskr-route-data">{route_data}</ul></div></section>"#,
        path = escape_html(path),
        surface = escape_html(page.surface),
        title = escape_html(page.title),
        description = escape_html(page.description),
        summary = route_summary_pending_html(path),
        routes = route_inventory,
        endpoints = endpoints,
        actions = route_actions_html(path),
        route_data = route_probe_pending_html(path),
    )
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

    let sections = app_sections()
        .iter()
        .map(|section| {
            format!(
                r#"<section class="slskr-panel" id="{id}"><div><h2>{title}</h2><p>{description}</p></div><code>{endpoint}</code></section>"#,
                id = section.title.to_ascii_lowercase(),
                title = section.title,
                description = section.description,
                endpoint = endpoint_url(section.endpoint)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let routes = ui_routes()
        .iter()
        .map(|route| {
            format!(
                r#"<li><code>{path}</code><span>{title}</span></li>"#,
                path = route.path,
                title = route.title
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let endpoints = api_endpoints()
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

    format!(
        r#"<div class="slskr-shell"><nav class="slskr-nav">{nav}</nav><main class="slskr-main"><header class="slskr-hero"><p class="slskr-kicker">Rust web migration target</p><h1>slskr</h1><p>Native Rust/WASM app shell for porting the existing browser UI one route at a time.</p><code>{report}</code></header><section id="slskr-route-view">{route_page}</section><section class="slskr-contract slskr-runtime"><h2>Runtime Status</h2><ul id="slskr-runtime-status">{runtime}</ul></section><div class="slskr-grid">{sections}</div><section class="slskr-contract"><h2>Route Parity</h2><ul>{routes}</ul></section><section class="slskr-contract"><h2>API Contracts</h2><ul>{endpoints}</ul></section></main></div>"#,
        route_page = route_page_html("/searches"),
        runtime = runtime_probe_pending_html(),
        report = compatibility_report()
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
    wasm_bindgen_futures::spawn_local(async {
        let _ = refresh_runtime_status().await;
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
        let method = button
            .get_attribute("data-slskr-action-method")
            .unwrap_or_else(|| "GET".to_string());
        let path = button
            .get_attribute("data-slskr-action-path")
            .unwrap_or_default();
        let body_kind = button
            .get_attribute("data-slskr-action-body")
            .unwrap_or_else(|| "None".to_string());
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
                let body = action_body_from_value(action_body_from_name(&body_kind), &value);
                let window = window.clone();
                let document = document.clone();
                let method = method.clone();
                let path = path.clone();
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
fn action_body_from_name(name: &str) -> ActionBody {
    match name {
        "BrowseDirectory" => ActionBody::BrowseDirectory,
        "ConversationMessage" => ActionBody::ConversationMessage,
        "DownloadFiles" => ActionBody::DownloadFiles,
        "EnabledFalse" => ActionBody::EnabledFalse,
        "EnabledTrue" => ActionBody::EnabledTrue,
        "JsonString" => ActionBody::JsonString,
        "RoomMessage" => ActionBody::RoomMessage,
        "SearchText" => ActionBody::SearchText,
        _ => ActionBody::None,
    }
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

#[cfg(target_arch = "wasm32")]
async fn refresh_route_data(window: &web_sys::Window) -> Result<(), JsValue> {
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let Some(status) = document.get_element_by_id("slskr-route-data") else {
        return Ok(());
    };
    let summary = document.get_element_by_id("slskr-route-summary");
    let path = window.location().pathname()?;
    let Some(page) = route_page(&path) else {
        return Ok(());
    };

    let mut rendered = String::new();
    let mut responses = Vec::new();
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
    }

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
        assert!(html.contains("Rust/WASM"));
        assert!(html.contains("/api/v0/searches"));
        assert!(html.contains("slskr-runtime-status"));
        assert!(html.contains("/api/v0/health"));
        assert!(html.contains("slskr-route-view"));
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
        assert!(html.contains("Live Route Data"));
        assert!(html.contains("slskr-route-actions"));
        assert!(html.contains("slskr-route-summary"));
        assert!(html.contains("Summary"));
        assert!(html.contains("Clear Completed Downloads"));
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
        let html = route_actions_html("/searches/<script>");
        assert!(html.contains("/api/v0/searches/1"));
        assert!(!html.contains("<script>"));
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
            ("POST", "/shares/rescan"),
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
