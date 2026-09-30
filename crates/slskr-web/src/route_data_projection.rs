//! Native Web route data projection.

use super::*;
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

pub(super) fn response_count(responses: Option<&[EndpointBody]>, endpoint: &str) -> String {
    responses
        .and_then(|items| endpoint_body(items, endpoint))
        .and_then(json_array_len)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "0".to_string())
}

pub(super) fn response_numeric_sum(
    responses: Option<&[EndpointBody]>,
    endpoint: &str,
    keys: &[&str],
) -> String {
    let total = endpoint_array(responses, endpoint)
        .iter()
        .filter_map(|item| value_text(item, keys))
        .filter_map(|value| value.parse::<u64>().ok())
        .sum::<u64>();
    total.to_string()
}

pub(super) fn response_bool_count(
    responses: Option<&[EndpointBody]>,
    endpoint: &str,
    keys: &[&str],
) -> String {
    endpoint_array(responses, endpoint)
        .iter()
        .filter(|item| value_bool(item, keys).unwrap_or(false))
        .count()
        .to_string()
}

pub(super) fn response_online_count(responses: Option<&[EndpointBody]>, endpoint: &str) -> String {
    endpoint_array(responses, endpoint)
        .iter()
        .filter(|item| {
            value_bool(item, &["online", "connected", "isOnline"]).unwrap_or_else(|| {
                value_text(item, &["status", "state"]).is_some_and(|state| {
                    matches!(state.to_ascii_lowercase().as_str(), "online" | "connected")
                })
            })
        })
        .count()
        .to_string()
}

pub(super) fn response_value(
    responses: Option<&[EndpointBody]>,
    endpoint: &str,
    field: &str,
) -> String {
    responses
        .and_then(|items| endpoint_body(items, endpoint))
        .and_then(|body| json_field_string(body, field))
        .unwrap_or_else(|| "pending".to_string())
}

pub(super) fn status_chip_html(label: &str, value: &str) -> String {
    format!(
        r#"<span class="slskr-status-chip"><strong>{}</strong>{}</span>"#,
        escape_html(label),
        escape_html(value)
    )
}

pub(super) fn json_endpoint_value(
    responses: Option<&[EndpointBody]>,
    endpoint: &str,
) -> Option<serde_json::Value> {
    responses
        .and_then(|items| endpoint_body(items, endpoint))
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
}

pub(super) fn value_array(value: &serde_json::Value) -> Vec<serde_json::Value> {
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

pub(super) fn endpoint_array(
    responses: Option<&[EndpointBody]>,
    endpoint: &str,
) -> Vec<serde_json::Value> {
    json_endpoint_value(responses, endpoint)
        .map(|value| value_array(&value))
        .unwrap_or_default()
}

pub(super) fn value_at_path<'a>(
    value: &'a serde_json::Value,
    key: &str,
) -> Option<&'a serde_json::Value> {
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

pub(super) fn value_text(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
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

pub(super) fn value_number(value: &serde_json::Value, keys: &[&str]) -> Option<f64> {
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

pub(super) fn value_bool(value: &serde_json::Value, keys: &[&str]) -> Option<bool> {
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

pub(super) fn nested_items(value: &serde_json::Value, keys: &[&str]) -> Vec<serde_json::Value> {
    for key in keys {
        if let Some(items) = value_at_path(value, key).and_then(|entry| entry.as_array()) {
            return items.clone();
        }
    }
    Vec::new()
}

pub(super) fn value_count(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
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

pub(super) fn value_text_with_parent(
    value: &serde_json::Value,
    parent: Option<&serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    value_text(value, keys).or_else(|| parent.and_then(|parent| value_text(parent, keys)))
}

pub(super) fn value_number_with_parent(
    value: &serde_json::Value,
    parent: Option<&serde_json::Value>,
    keys: &[&str],
) -> Option<f64> {
    value_number(value, keys).or_else(|| parent.and_then(|parent| value_number(parent, keys)))
}

pub(super) fn format_speed(
    value: &serde_json::Value,
    parent: Option<&serde_json::Value>,
) -> String {
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

pub(super) fn first_nested_text(
    value: &serde_json::Value,
    array_keys: &[&str],
    field_keys: &[&str],
) -> Option<String> {
    nested_items(value, array_keys)
        .first()
        .and_then(|item| value_text(item, field_keys))
}

pub(super) fn format_transfer_progress(
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

pub(super) fn row_meta_with_id(meta: String, id: Option<&str>) -> String {
    id.filter(|value| safe_route_segment(value.trim()))
        .map(|id| format!("{meta} / id={}", id.trim()))
        .unwrap_or(meta)
}

pub(super) fn browse_response_entries(
    responses: Option<&[EndpointBody]>,
) -> Vec<serde_json::Value> {
    let Some(root) = json_endpoint_value(responses, "/users/:username/browse") else {
        return Vec::new();
    };
    let mut entries = value_array(&root);
    entries.extend(nested_items(&root, &["directories", "root.directories"]));
    for directory in nested_items(&root, &["directories", "root.directories"]) {
        entries.extend(nested_items(&directory, &["files", "children", "items"]));
    }
    entries.extend(nested_items(
        &root,
        &["files", "root.files", "children", "items"],
    ));
    entries
}

pub(super) fn route_dynamic_rows(
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
        RouteKind::SharedWithMe => {
            // `/shared` is the local share-directory compatibility endpoint,
            // not the inbound ShareGrant collection used by slskdn's
            // SharedWithMe page.  Treating its directory identifiers as grant
            // IDs made an empty inbound page render executable-looking rows
            // that could only produce 400/404 responses.
            endpoint_array(responses, "/share-grants")
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
                .collect::<Vec<_>>()
        }
        RouteKind::Browse => browse_response_entries(responses)
            .iter()
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
            .collect::<Vec<_>>(),
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
