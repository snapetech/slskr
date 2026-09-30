//! Native Web native row rendering.

use super::*;

pub(super) fn native_row_cards_html(
    rows: &[(String, String, String, String)],
    empty: &str,
) -> String {
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

pub(super) fn native_table_html(
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

pub(super) fn native_meta_cell_html(
    kind: RouteKind,
    meta: &str,
    primary: &str,
    secondary: &str,
) -> String {
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
            r#"<div class="slskr-native-state-stack" data-slskr-inbound-permission-controls><span>{meta}</span><div class="slskr-native-permission-grid"><span>{permissions}</span><span>{owner}</span><button type="button">Copy token</button></div></div>"#,
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

pub(super) fn native_progress_percent(meta: &str) -> u32 {
    meta.split('/')
        .map(str::trim)
        .find_map(|part| {
            part.strip_suffix('%')
                .and_then(|value| value.trim().parse::<u32>().ok())
        })
        .map(|value| value.min(100))
        .unwrap_or(0)
}

pub(super) fn native_row_detail_list(
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

pub(super) fn native_row_action_summary(
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

pub(super) fn native_row_data_attrs(
    kind: RouteKind,
    primary: &str,
    secondary: &str,
    meta: &str,
) -> String {
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

pub(super) fn native_row_embedded_id(meta: &str) -> Option<&str> {
    meta.split("id=")
        .nth(1)
        .and_then(|tail| tail.split_whitespace().next())
        .map(|value| value.trim_matches(|ch: char| matches!(ch, ',' | ';' | ')' | ']')))
        .filter(|value| safe_route_segment(value))
}

pub(super) fn native_row_resource_id(
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

pub(super) fn native_resource_candidate(value: &str) -> Option<String> {
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

pub(super) fn native_row_action_labels(kind: RouteKind, primary_action: &str) -> Vec<&str> {
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
        RouteKind::Collections => vec![primary_action, "Add Item", "Share"],
        RouteKind::ShareGroups => vec![
            "Add Member",
            "Issue Token",
            "Create Share Grant",
            "Update Share Grant",
            primary_action,
        ],
        RouteKind::SharedWithMe => vec![primary_action, "Stream", "Backfill", "Copy token"],
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

pub(super) fn native_row_action_buttons_html(kind: RouteKind, primary_action: &str) -> String {
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

pub(super) fn native_route_table_html(
    kind: RouteKind,
    rows: &[(String, String, String, String)],
) -> String {
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
