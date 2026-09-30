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

#[cfg(target_arch = "wasm32")]
fn route_param_value_optional(path: &str) -> Option<String> {
    let value = path
        .trim_matches('/')
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())?;
    safe_route_segment(value).then(|| value.to_owned())
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
    if action.path.contains(":collectionId") || action.path.contains(":itemId") {
        let collection_id = target
            .filter(|value| safe_route_segment(value))
            .unwrap_or("1");
        let item_id = id.filter(|value| safe_route_segment(value)).unwrap_or("1");
        return endpoint_url(action.path)
            .replace(":collectionId", collection_id)
            .replace(":itemId", item_id);
    }
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
        .replace(":id", search_id)
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
        (
            RouteKind::Wishlist,
            "add wanted search" | "add search" | "add your first search" | "import list",
        ) => &["Add Wishlist Item"],
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
        (RouteKind::Messages | RouteKind::Rooms, "open batch private-message dialog") => {
            &["Send Message"]
        }
        (RouteKind::Messages | RouteKind::Rooms, "leave" | "leave room") => &["Leave Room"],
        (RouteKind::Users, "watch") => &["Watch User"],
        (RouteKind::Users, "save note") => &["Add User Note"],
        (RouteKind::Users, "browse") => &["Request Directory"],
        (RouteKind::Users, "message") => &["Send Message"],
        (RouteKind::Contacts, "add contact" | "add friend") => &["Add Contact"],
        (RouteKind::Contacts, "create invite") => &["Create Invite"],
        (RouteKind::Contacts, "refresh nearby") => &["Refresh Nearby"],
        (RouteKind::Contacts, "message") => &["Send Message"],
        (RouteKind::Contacts, "browse") => &["Request Directory"],
        (RouteKind::Contacts, "remove") => &["Remove Contact"],
        (
            RouteKind::Solid,
            "resolve webid" | "connect identity" | "sync storage" | "refresh session",
        ) => &["Resolve WebID"],
        (RouteKind::Collections, "create collection") => &["Create Collection"],
        (RouteKind::Collections, "update collection") => &["Update Collection"],
        (RouteKind::Collections, "delete collection") => &["Delete Collection"],
        (RouteKind::Collections, "open" | "open collection") => &["Open Collection"],
        (RouteKind::Collections, "add item") => &["Add Item to Collection"],
        (RouteKind::Collections, "remove item") => &["Remove Collection Item"],
        (RouteKind::Collections, "update item") => &["Update Collection Item"],
        (RouteKind::Collections, "share") => &["Create Share Grant"],
        (RouteKind::ShareGroups, "create group" | "create your first group") => {
            &["Create Share Group"]
        }
        (RouteKind::ShareGroups, "add member") => &["Add Share Group Member"],
        (RouteKind::ShareGroups, "issue token") => &["Issue Share Token"],
        (RouteKind::ShareGroups, "update share grant") => &["Update Share Grant"],
        (RouteKind::ShareGroups, "create share grant") => &["Create Share Grant"],
        (RouteKind::SharedWithMe, "open" | "open collection") => &["Open Shared Manifest"],
        (RouteKind::SharedWithMe, "backfill") => &["Backfill Share Grant"],
        (RouteKind::Browse, "browse" | "open a new browse tab" | "new tab" | "refresh folder") => {
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
        (RouteKind::System, "check lidarr") => &["Check Lidarr"],
        (RouteKind::System, "refresh lidarr sync") => &["Refresh Lidarr Sync"],
        (RouteKind::System, "refresh musicbrainz") => &["Refresh MusicBrainz"],
        (RouteKind::System, "refresh songid") => &["Refresh SongID"],
        (RouteKind::System, "track musicbrainz target") => &["Track MusicBrainz Target"],
        (RouteKind::System, "start songid run") => &["Start SongID Run"],
        (
            RouteKind::System,
            "start library health scan" | "scan library" | "scan library health",
        ) => &["Start Library Health Scan"],
        (RouteKind::System, "fix library issues" | "fix health issues") => &["Fix Library Issues"],
        (RouteKind::System, "setup health") => &["Setup Health"],
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
            r#"{{"contentId":"{}","artist":"","title":"{}","kind":"Audio"}}"#,
            escape_json_string(value),
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
            Some(format!("[{}]", files.join(",")))
        }
        ActionBody::EnabledFalse => Some(r#"{"enabled":false}"#.to_string()),
        ActionBody::EnabledTrue => Some(r#"{"enabled":true}"#.to_string()),
        ActionBody::FeedPreview => Some(format!(
            r#"{{"sourceText":"{}","sourceKind":"auto","limit":25,"includeAlbum":true,"fetchProviderUrls":false}}"#,
            escape_json_string(value)
        )),
        ActionBody::InviteRequest => Some(r#"{"expiresInHours":24}"#.to_string()),
        ActionBody::LibraryPath => Some(format!(
            r#"{{"libraryPath":"{}"}}"#,
            escape_json_string(value)
        )),
        ActionBody::ConversationMessage | ActionBody::JsonString => {
            Some(format!(r#""{}""#, escape_json_string(value)))
        }
        ActionBody::MusicBrainzTarget => Some(format!(
            r#"{{"releaseId":"{}"}}"#,
            escape_json_string(value)
        )),
        ActionBody::NameDescription => Some(format!(
            r#"{{"title":"{}","description":"Created from the Rust web UI","type":"ShareList"}}"#,
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
            r#"{{"collection_id":"","username":"{}"}}"#,
            escape_json_string(value)
        )),
        ActionBody::ShareGroupMember => {
            Some(format!(r#"{{"userId":"{}"}}"#, escape_json_string(value)))
        }
        ActionBody::SongIdSource => {
            Some(format!(r#"{{"source":"{}"}}"#, escape_json_string(value)))
        }
        ActionBody::Username => Some(format!(
            r#"{{"username":"{}","note":"Created from the Rust web UI"}}"#,
            escape_json_string(value)
        )),
        ActionBody::ContactDiscovery => Some(format!(
            r#"{{"peerId":"{}","nickname":"{}"}}"#,
            escape_json_string(value),
            escape_json_string(value)
        )),
        ActionBody::ContactInvite => Some(format!(
            r#"{{"inviteLink":"slskdn://invite/{}","nickname":"{}"}}"#,
            escape_json_string(value),
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
            r#"<input class="slskr-action-input" data-slskr-action-input="ConversationMessage" value="" placeholder="Message">"#.to_string()
        }
        ActionBody::CollectionItem => {
            r#"<input class="slskr-action-input" data-slskr-action-input="CollectionItem" value="" placeholder="Content ID">"#.to_string()
        }
        ActionBody::DownloadFiles => {
            r#"<input class="slskr-action-input" data-slskr-action-input="DownloadFiles" value="" placeholder="Filename">"#.to_string()
        }
        ActionBody::EnabledFalse | ActionBody::EnabledTrue => String::new(),
        ActionBody::FeedPreview => {
            r#"<input class="slskr-action-input" data-slskr-action-input="FeedPreview" value="" placeholder="Playlist text">"#.to_string()
        }
        ActionBody::InviteRequest => String::new(),
        ActionBody::LibraryPath => {
            r#"<input class="slskr-action-input" data-slskr-action-input="LibraryPath" value="" placeholder="Library path">"#.to_string()
        }
        ActionBody::JsonString => {
            r#"<input class="slskr-action-input" data-slskr-action-input="JsonString" value="" placeholder="Name">"#.to_string()
        }
        ActionBody::MusicBrainzTarget => {
            r#"<input class="slskr-action-input" data-slskr-action-input="MusicBrainzTarget" value="" placeholder="MusicBrainz release ID">"#.to_string()
        }
        ActionBody::NameDescription => {
            r#"<input class="slskr-action-input" data-slskr-action-input="NameDescription" value="" placeholder="Name">"#.to_string()
        }
        ActionBody::Permissions => {
            r#"<input class="slskr-action-input" data-slskr-action-input="Permissions" value="read" placeholder="Permissions">"#.to_string()
        }
        ActionBody::RoomMessage => {
            r#"<input class="slskr-action-input" data-slskr-action-input="RoomMessage" value="" placeholder="Message">"#.to_string()
        }
        ActionBody::SearchText => {
            r#"<input class="slskr-action-input" data-slskr-action-input="SearchText" value="" placeholder="Search text">"#.to_string()
        }
        ActionBody::ShareGrant | ActionBody::ShareGroupMember => {
            r#"<input class="slskr-action-input" data-slskr-action-input="Username" value="" placeholder="Username">"#.to_string()
        }
        ActionBody::SongIdSource => {
            r#"<input class="slskr-action-input" data-slskr-action-input="SongIdSource" value="" placeholder="SongID source">"#.to_string()
        }
        ActionBody::Username => {
            r#"<input class="slskr-action-input" data-slskr-action-input="Username" value="" placeholder="Username">"#.to_string()
        }
        ActionBody::ContactDiscovery | ActionBody::ContactInvite => {
            r#"<input class="slskr-action-input" data-slskr-action-input="Username" value="" placeholder="Username">"#.to_string()
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn native_action_body(
    _document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
    value: &str,
) -> Option<String> {
    if action.body != ActionBody::ShareGrant {
        return action_body_from_value(action.body, value);
    }

    let workspace = button.closest(".slskr-native-workspace").ok().flatten();
    let collection_id = button_native_row_attribute(button, "data-slskr-native-collection-id")
        .or_else(|| {
            workspace.as_ref().and_then(|workspace| {
                selected_native_row_attribute(workspace, "data-slskr-native-collection-id")
            })
        })
        .unwrap_or_default();
    Some(format!(
        r#"{{"collection_id":"{}","username":"{}"}}"#,
        escape_json_string(&collection_id),
        escape_json_string(value.trim()),
    ))
}

#[cfg(target_arch = "wasm32")]
fn native_action_value_is_valid(
    document: &web_sys::Document,
    label: &str,
    action: RouteAction,
    value: &str,
) -> bool {
    let value_required = matches!(
        action.body,
        ActionBody::BrowseDirectory
            | ActionBody::CollectionItem
            | ActionBody::ConversationMessage
            | ActionBody::ContactDiscovery
            | ActionBody::ContactInvite
            | ActionBody::DownloadFiles
            | ActionBody::FeedPreview
            | ActionBody::JsonString
            | ActionBody::LibraryPath
            | ActionBody::MusicBrainzTarget
            | ActionBody::NameDescription
            | ActionBody::RoomMessage
            | ActionBody::SearchText
            | ActionBody::ShareGrant
            | ActionBody::ShareGroupMember
            | ActionBody::SongIdSource
            | ActionBody::Username
    );
    if value_required && value.trim().is_empty() {
        native_set_action_status(
            document,
            label,
            "Enter the required value before running this action.",
        );
        return false;
    }
    true
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
            stat_card_html("Selected", "pending", "conversation"),
            stat_card_html(
                "Actions",
                &surface_actions("messages").len().to_string(),
                "Rust owned",
            ),
        ]
        .join(""),
        "browse" => [
            stat_card_html("Peer", "not selected", "browse target"),
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
                .unwrap_or_else(|| "not selected".to_string());
            [
                stat_card_html("Conversations", &conversations, "threads"),
                stat_card_html("Selected", &selected, "conversation"),
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
                stat_card_html("Peer", "not selected", "browse target"),
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
