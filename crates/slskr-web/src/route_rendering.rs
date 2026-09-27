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

fn response_numeric_sum(
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

fn response_bool_count(
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

fn response_online_count(responses: Option<&[EndpointBody]>, endpoint: &str) -> String {
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

fn browse_response_entries(responses: Option<&[EndpointBody]>) -> Vec<serde_json::Value> {
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
        .map(|label| {
            format!(
                r#"<button type="button" data-slskr-reference-action="{label}">{label}</button>"#,
                label = escape_html(label),
            )
        })
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
            "Lidarr, MusicBrainz, SongID, and source-provider integrations."
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

fn native_tabs_html(kind: RouteKind, responses: Option<&[EndpointBody]>) -> String {
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
                r#"<section class="slskr-native-subpanel" data-slskr-native-panel="{index}" data-slskr-native-panel-label="{}"{hidden}>{}</section>"#,
                escape_html(label),
                native_tab_panel_html(kind, label, responses),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="slskr-native-subviews"><div class="slskr-native-subnav" role="tablist">{buttons}</div><div class="slskr-native-subpanels">{panels}</div></div>"#
    )
}

fn native_tab_panel_html(
    kind: RouteKind,
    label: &str,
    responses: Option<&[EndpointBody]>,
) -> String {
    if kind == RouteKind::System {
        return native_system_tab_panel_html(label, responses);
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

fn native_system_tab_panel_html(label: &str, responses: Option<&[EndpointBody]>) -> String {
    let detail = native_tab_detail(RouteKind::System, label);
    let controls = if label == "Experience" {
        String::new()
    } else {
        native_tab_controls(RouteKind::System, label)
            .iter()
            .map(|control| format!(r#"<button type="button">{}</button>"#, escape_html(control)))
            .collect::<Vec<_>>()
            .join("")
    };
    let fields = if label == "Experience" {
        String::new()
    } else {
        native_tab_fields(RouteKind::System, label)
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
            .join("")
    };
    let rows = native_system_live_rows(label, responses);
    let facts = native_tab_facts(RouteKind::System, label)
        .iter()
        .map(|fact| format!(r#"<span>{}</span>"#, escape_html(fact)))
        .collect::<Vec<_>>()
        .join("");
    let local_panel = match label {
        "Experience" => experience_settings_panel_html(),
        "Automations" => automation_center_panel_html(),
        _ => String::new(),
    };
    format!(
        r#"<header><div><h4>{label}</h4><p>{detail}</p></div><div class="slskr-native-panel-actions">{controls}</div></header><div class="slskr-native-panel-fields">{fields}</div><div class="slskr-native-panel-facts">{facts}</div><div class="slskr-native-table-wrap"><table class="slskr-native-table slskr-system-panel-table"><thead><tr><th>Area</th><th>State</th><th>Detail</th><th>Action</th></tr></thead><tbody>{rows}</tbody></table></div>{local_panel}"#,
        label = escape_html(label),
        detail = escape_html(detail),
        controls = controls,
        fields = fields,
        facts = facts,
        rows = rows,
        local_panel = local_panel,
    )
}

fn native_system_live_rows(label: &str, responses: Option<&[EndpointBody]>) -> String {
    let endpoints: &[&str] = match label {
        "Info" => &["/server", "/database/stats"],
        "Network" => &["/server", "/mesh/stats", "/bridge/status"],
        "Mesh" => &["/mesh/stats", "/security/dashboard"],
        "Bridge" => &["/bridge/status"],
        "MediaCore" => &["/songid/runs", "/transfers/speeds"],
        "Security Policies" => &["/security/dashboard"],
        "Experience" => &[],
        "Integrations" => &[
            "/source-providers",
            "/integrations/lidarr/status",
            "/integrations/lidarr/sync/status",
            "/musicbrainz/albums/completion",
            "/musicbrainz/release-radar/subscriptions",
            "/songid/runs",
        ],
        "Options" => &["/options", "/config"],
        "Shares" => &["/shares"],
        "Jobs" | "Automations" => &["/jobs"],
        "Source Providers" => &["/source-providers"],
        "Swarm Analytics" => &["/mesh/stats", "/telemetry/metrics"],
        "Library Health" => &[
            "/library/health/issues",
            "/library/health/issues/by-type",
            "/library/health/issues/by-artist",
            "/library/health/issues/by-release",
            "/library/health/issues/by-codec",
        ],
        "Files" => &["/library/items", "/songid/runs"],
        "Quarantine Jury" => &["/quarantine-jury/audit", "/quarantine-jury/requests"],
        "Data" => &["/database/stats"],
        "Events" => &["/events"],
        "Logs" => &["/logs"],
        "Metrics" => &[
            "/telemetry/metrics",
            "/telemetry/metrics/kpis",
            "/telemetry/reports/transfers/summary",
        ],
        _ => &[],
    };
    let mut rows = endpoints
        .iter()
        .filter_map(|endpoint| {
            let value = json_endpoint_value(responses, endpoint)?;
            let title = endpoint.trim_start_matches('/').replace('/', " / ");
            let detail = compact_preview(&value.to_string());
            Some(format!(
                r#"<tr tabindex="0" data-slskr-native-select data-slskr-native-title="{title}" data-slskr-native-detail="{endpoint}" data-slskr-native-meta="returned"><td><strong>{title}</strong></td><td>returned</td><td><code>{detail}</code></td><td><button type="button">Review Selection</button></td></tr>"#,
                title = escape_html(&title),
                endpoint = escape_html(endpoint),
                detail = escape_html(&detail),
            ))
        })
        .collect::<Vec<_>>();
    if label == "Experience" {
        rows.push(
            r#"<tr tabindex="0" data-slskr-native-select data-slskr-native-title="Browser preferences" data-slskr-native-detail="local storage" data-slskr-native-meta="local"><td><strong>Browser preferences</strong></td><td>local</td><td><code>Stored in this browser</code></td><td><button type="button">Review Selection</button></td></tr>"#.to_string(),
        );
    }
    if rows.is_empty() {
        r#"<tr><td colspan="4"><div class="slskr-native-empty"><strong>No live data returned for this tab</strong><span>Use the route action or refresh after enabling the corresponding service.</span></div></td></tr>"#.to_string()
    } else {
        rows.join("")
    }
}

#[allow(dead_code)]
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
        (RouteKind::Collections, "Collections") => &[
            "Create Collection",
            "Update Collection",
            "Delete Collection",
        ],
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
        (RouteKind::System, "Integrations") => &[
            "Check Lidarr",
            "Refresh Lidarr Sync",
            "Track MusicBrainz Target",
            "Refresh MusicBrainz",
            "Start SongID Run",
            "Refresh SongID",
        ],
        (RouteKind::System, "Logs" | "Events") => &["Refresh", "Clear Filter"],
        (RouteKind::System, "Options" | "Experience") => &[],
        (RouteKind::System, "Jobs" | "Automations") => &["Refresh"],
        (RouteKind::System, "Library Health") => &[
            "Scan Library Health",
            "Fix Library Issues",
            "Copy Action Plan",
        ],
        (RouteKind::System, "Quarantine Jury") => &["Refresh", "Copy Packet"],
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
            &[("Chat username", "Username"), ("Message", "Message")]
        }
        (RouteKind::Messages | RouteKind::Rooms, "Rooms") => &[("Search rooms", "Room name")],
        (RouteKind::Browse, "Tabs") => &[("Username", "Username")],
        (RouteKind::Browse, "Tree" | "Files") => &[("Username", "Username"), ("Folder", "/")],
        (RouteKind::Collections, "Collections") => &[
            ("Title", "Collection title"),
            ("Description", "Optional description"),
        ],
        (RouteKind::Collections, "Picker" | "Items") => &[("Search for item", "Filename or title")],
        (RouteKind::ShareGroups, "Groups") => &[("Group Name", "Trusted peers")],
        (RouteKind::ShareGroups, "Members") => &[("Soulseek Username", "Username")],
        (RouteKind::ShareGroups, "Permissions") => &[("Permissions", "read,download,stream")],
        (RouteKind::Solid, "Identity") => &[("WebID", "https://example.com/profile/card#me")],
        (RouteKind::Search, "Searches" | "Results") => &[("Search text", "public domain jazz")],
        (RouteKind::DiscoveryGraph, "Graph") => &[("Artist Name", "Artist or query")],
        (RouteKind::PlaylistIntake, "Parser") => &[("Playlist rows", "One track per line")],
        (RouteKind::Wishlist, "Wanted") => &[("Search Text", "Search text")],
        (RouteKind::Users, "Directory" | "Detail") => &[("Username", "Username")],
        (RouteKind::Contacts, "Contacts" | "Invites") => &[("Nickname", "Nickname")],
        (RouteKind::System, "Logs" | "Events") => &[("Filter", "Filter text")],
        (RouteKind::System, "Library Health") => &[("Library Path", "Optional library path")],
        (RouteKind::System, "Options") => &[("Option key", "Configuration key")],
        (RouteKind::System, "Integrations") => &[
            ("Release ID", "MusicBrainz release identifier"),
            ("SongID source", "local path, Spotify URL, or query"),
        ],
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
        (RouteKind::System, "Integrations") => {
            &["Lidarr", "MusicBrainz", "SongID", "Source providers"]
        }
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
                native_editor_text_field_html("Search Text", "Search text"),
                native_editor_text_field_html("Filter", "Optional filter"),
                native_editor_text_field_html("Max Results", "Maximum results"),
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
                native_editor_text_field_html("Username", "Username"),
                native_editor_text_field_html("Display note", "Note"),
                native_editor_text_field_html("Privilege note", "Privilege note"),
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
                native_editor_text_field_html("Invite", "Invite link"),
                native_editor_text_field_html("Nickname", "Nickname"),
                native_editor_text_field_html("Group", "Group"),
                native_editor_text_field_html("Note", "Note"),
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
                native_editor_text_field_html("Title", "Collection title"),
                native_editor_text_field_html("Description", "Description"),
                native_editor_text_field_html("Search for item", "Filename, artist, or title"),
                native_editor_text_field_html("Audience", "Share group or token"),
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
                native_editor_text_field_html("Group Name", "Group name"),
                native_editor_text_field_html("Soulseek Username", "Username"),
                native_editor_text_field_html("Permissions", "read, stream, download"),
                native_editor_text_field_html("Expires", "Expiration"),
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
                native_editor_text_field_html("Token", "Share token"),
                native_editor_text_field_html("Owner", "Owner"),
                native_editor_text_field_html("Collection", "Collection"),
                native_editor_text_field_html("Access", "stream/download"),
            ]
            .join(""),
            native_editor_action_buttons(&["Open", "Stream", "Backfill", "Copy token"]),
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
                native_editor_text_field_html("Option key", "Configuration key"),
                native_editor_text_field_html("Value", "Value"),
                native_editor_text_field_html("Filter", "Filter text"),
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

    let route = format!("{kind:?}");
    format!(
        r#"<aside class="slskr-native-editor" data-slskr-native-editor data-slskr-native-editor-route="{route}" aria-label="{title}"><header><div><h3>{title}</h3><p>{summary}</p></div><span>Draft</span></header><div class="slskr-native-editor-grid">{fields}</div><div class="slskr-native-editor-actions">{actions}</div><div class="slskr-native-editor-state">{state}</div></aside>"#,
        route = route,
        title = escape_html(title),
        summary = escape_html(summary),
        fields = fields,
        actions = actions,
        state = state,
    )
}

fn native_browse_workspace_html(route_table: &str, responses: Option<&[EndpointBody]>) -> String {
    let entries = browse_response_entries(responses);
    let folders = entries
        .iter()
        .filter_map(|entry| {
            let is_folder = value_bool(entry, &["isDirectory", "directory"]).unwrap_or(false);
            is_folder
                .then(|| value_text(entry, &["name", "path", "directory", "folder"]))
                .flatten()
        })
        .take(50)
        .collect::<Vec<_>>();
    let folder_controls = if folders.is_empty() {
        "<span>No directories returned by the browse request.</span>".to_string()
    } else {
        folders
            .iter()
            .map(|folder| {
                format!(
                    r#"<button type="button" data-slskr-browse-folder="{}" aria-expanded="false">{}</button>"#,
                    escape_html(folder),
                    escape_html(folder),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let current_folder = folders.first().cloned().unwrap_or_else(|| "/".to_string());
    let session_summary = if entries.is_empty() {
        "No browse result loaded".to_string()
    } else {
        format!("{} entries returned", entries.len())
    };
    let manifest = if entries.is_empty() {
        "No browse entries returned. Select a peer and request a directory to populate this preview."
            .to_string()
    } else {
        format!(
            "{} live browse entries; select files to calculate queue impact.",
            entries.len()
        )
    };
    let preview = native_selection_preview_html(
        "No files selected",
        "Select browsed files to preview batch download impact.",
        "Waiting",
        "Download Selected",
    );
    format!(
        r#"<div class="slskr-native-grid browse-native" data-slskr-browse-workspace><aside class="slskr-native-side slskr-native-sidebar"><h3>Browse</h3><div class="slskr-native-command-row"><input aria-label="Username" placeholder="Username"><button type="button">Open a New Browse Tab</button></div><div class="slskr-native-browse-tabs" role="tablist" aria-label="Browse sessions"><span data-slskr-browse-session="current">{session_summary}</span><button type="button">New Tab</button></div><div class="slskr-native-tree" data-slskr-browse-tree data-slskr-browse-folder-list><strong>Directory Tree</strong>{folder_controls}</div><div class="slskr-native-mini-list"><span data-slskr-browse-entry-count>{entry_count} entries in current response</span><span>Folders expand from live browse data</span><span>Peer browse history is empty until a request succeeds</span></div></aside><section class="slskr-native-main"><h3>Files</h3><div class="slskr-native-breadcrumb" data-slskr-browse-breadcrumb><span>{current_folder}</span></div><div class="slskr-native-command-row"><input aria-label="Folder" placeholder="/" value="{current_folder}"><input aria-label="File filter" placeholder="Filter files"><button type="button">Refresh Folder</button><button type="button">Download Selected</button></div><div class="slskr-native-split-detail"><div>{route_table}</div><aside><h4>Download Preview</h4><p>Selected files, destination, peer, queue impact, and duplicate warnings.</p>{preview}<div class="slskr-native-mini-list" data-slskr-browse-download-manifest><span>{manifest}</span><span>Preserve folders when the destination policy allows it.</span><span>Duplicate warning review runs before queueing.</span><span>Estimated queue impact appears after selection.</span></div></aside></div></section></div>"#,
        session_summary = escape_html(&session_summary),
        folder_controls = folder_controls,
        entry_count = entries.len(),
        current_folder = escape_html(&current_folder),
        route_table = route_table,
        preview = preview,
        manifest = escape_html(&manifest),
    )
}

fn native_collection_items_html(responses: Option<&[EndpointBody]>) -> String {
    let items = endpoint_array(responses, "/collections/:id/items");
    let fallback_collection_id = endpoint_array(responses, "/collections")
        .first()
        .and_then(|collection| value_text(collection, &["id", "collectionId"]))
        .unwrap_or_default();
    if items.is_empty() {
        return r#"<div class="slskr-native-empty"><strong>No items in the selected collection</strong><span>Add a content ID after selecting a collection.</span></div>"#.to_string();
    }
    let rows = items
        .iter()
        .take(100)
        .filter_map(|item| {
            let id = value_text(item, &["id", "itemId"])?;
            let content_id = value_text(item, &["contentId", "content_id"])
                .unwrap_or_else(|| id.clone());
            let title = value_text(item, &["title", "name", "fileName"])
                .unwrap_or_else(|| content_id.clone());
            let artist = value_text(item, &["artist", "albumArtist"]).unwrap_or_default();
            let kind = value_text(item, &["kind", "mediaKind"]).unwrap_or_else(|| "Audio".to_string());
            let collection_id = value_text(item, &["collectionId", "collection_id"])
                .unwrap_or_else(|| fallback_collection_id.clone());
            let collection_attr = if collection_id.is_empty() {
                String::new()
            } else {
                format!(
                    r#" data-slskr-native-collection-id="{}""#,
                    escape_html(&collection_id)
                )
            };
            Some(format!(
                r#"<tr tabindex="0" data-slskr-native-select data-slskr-native-title="{title}" data-slskr-native-detail="{content_id}" data-slskr-native-item-id="{id}"{collection_attr} data-slskr-native-meta="{kind}"><td><strong>{title}</strong><small>{artist}</small></td><td>{content_id}</td><td>{kind}</td><td><button type="button">Remove Collection Item</button></td></tr>"#,
                title = escape_html(&title),
                artist = escape_html(&artist),
                content_id = escape_html(&content_id),
                kind = escape_html(&kind),
                id = escape_html(&id),
                collection_attr = collection_attr,
            ))
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        r#"<div class="slskr-native-empty"><strong>No readable collection items</strong><span>The daemon returned an item payload without an identifier.</span></div>"#.to_string()
    } else {
        format!(
            r#"<div class="slskr-native-table-wrap"><table class="slskr-native-table"><thead><tr><th>Item</th><th>Content ID</th><th>Kind</th><th>Action</th></tr></thead><tbody>{}</tbody></table></div>"#,
            rows.join("")
        )
    }
}

fn private_message_auto_response_panel_html(responses: Option<&[EndpointBody]>) -> String {
    let settings = responses
        .and_then(|responses| endpoint_body(responses, "/private-message-auto-response"))
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let enabled = value_bool(&settings, &["enabled"]).unwrap_or(false);
    let cooldown = settings
        .get("cooldown_minutes")
        .or_else(|| settings.get("cooldownMinutes"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(360);
    format!(
        r#"<section class="slskr-message-gate-panel" data-slskr-message-gate-panel><header><div><span>Human check</span><h4>Gate reply</h4></div><mark>{state}</mark></header><p>Reply once when a new private message asks for human or share verification. The reply text stays redacted from diagnostics.</p><div class="slskr-message-gate-controls"><label><input type="checkbox" data-slskr-message-gate-field="enabled" {enabled}> Enabled</label><label><span>Cooldown minutes</span><input inputmode="numeric" data-slskr-message-gate-field="cooldownMinutes" value="{cooldown}"></label><label><span>Reply text</span><textarea data-slskr-message-gate-field="message" placeholder="Leave blank to keep the configured reply"></textarea></label><button type="button" data-slskr-message-gate-save>Save gate reply</button></div><small>Runtime setting · startup default returns after restart</small></section>"#,
        state = if enabled { "armed" } else { "off" },
        enabled = if enabled { "checked" } else { "" },
        cooldown = cooldown,
    )
}

fn native_message_room_rows_html(responses: Option<&[EndpointBody]>) -> String {
    let mut rows = Vec::new();
    for (endpoint, action, state) in [
        ("/rooms/joined", "Leave Room", "joined"),
        ("/rooms/available", "Join Room", "available"),
    ] {
        for room in endpoint_array(responses, endpoint).iter().take(50) {
            let Some(name) = value_text(room, &["name", "roomName", "room"]) else {
                continue;
            };
            let users = value_count(room, &["userCount", "users", "members"])
                .map(|count| format!("{count} users"))
                .unwrap_or_else(|| "user count unavailable".to_string());
            rows.push(format!(
                r#"<tr tabindex="0" data-slskr-native-select data-slskr-native-title="{name}" data-slskr-native-detail="{name}" data-slskr-native-room-name="{name}" data-slskr-native-meta="{state}"><td>{name}</td><td>{state} / {users}</td><td><button type="button">{action}</button></td></tr>"#,
                name = escape_html(&name),
                state = escape_html(state),
                users = escape_html(&users),
                action = action,
            ));
        }
    }
    if rows.is_empty() {
        r#"<tr><td colspan="3"><div class="slskr-native-empty"><strong>No room records returned</strong><span>Join a room or refresh while the session is connected.</span></div></td></tr>"#.to_string()
    } else {
        rows.join("")
    }
}

fn native_message_thread_html(responses: Option<&[EndpointBody]>) -> String {
    let conversations = endpoint_array(responses, "/conversations");
    if conversations.is_empty() {
        return r#"<div class="slskr-native-empty"><strong>No conversation selected</strong><span>Start a direct message or select a live conversation row.</span></div>"#.to_string();
    }
    conversations
        .iter()
        .take(20)
        .filter_map(|conversation| {
            let username = value_text(conversation, &["username", "user", "name"])?;
            let last = value_text(conversation, &["lastMessage", "message", "latestMessage"])
                .unwrap_or_else(|| "No message text returned".to_string());
            let unread = value_text(
                conversation,
                &["unreadCount", "unacknowledgedCount", "messageCount"],
            )
            .unwrap_or_else(|| "0".to_string());
            Some(format!(
                r#"<article tabindex="0" data-slskr-native-select data-slskr-native-title="{username}" data-slskr-native-detail="{username}" data-slskr-native-peer="{username}" data-slskr-native-meta="{unread} unread"><strong>{username}</strong><span class="slskr-native-badge">{unread} unread</span><p>{last}</p><div class="slskr-native-command-row"><button type="button">Reply</button><button type="button">Acknowledge</button><button type="button">Delete Conversation</button></div></article>"#,
                username = escape_html(&username),
                unread = escape_html(&unread),
                last = escape_html(&last),
            ))
        })
        .collect::<Vec<_>>()
        .join("")
}

fn native_message_transcript_html(responses: Option<&[EndpointBody]>) -> String {
    let conversations = endpoint_array(responses, "/conversations");
    let rows = conversations
        .iter()
        .take(20)
        .filter_map(|conversation| {
            let username = value_text(conversation, &["username", "user", "name"])?;
            let last = value_text(conversation, &["lastMessage", "message", "latestMessage"])
                .unwrap_or_else(|| "No message text returned".to_string());
            Some(format!(
                r#"<article><strong>{username}</strong><p>{last}</p><time>live conversation data</time></article>"#,
                username = escape_html(&username),
                last = escape_html(&last),
            ))
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        "<p>No conversation transcript returned.</p>".to_string()
    } else {
        rows.join("")
    }
}

fn native_message_pods_html(responses: Option<&[EndpointBody]>) -> String {
    let pods = endpoint_array(responses, "/pods");
    if pods.is_empty() {
        return "<span>No pod channels returned.</span>".to_string();
    }
    pods.iter()
        .take(20)
        .filter_map(|pod| {
            let name = value_text(pod, &["name", "id", "podId"])?;
            let state =
                value_text(pod, &["state", "status"]).unwrap_or_else(|| "available".to_string());
            Some(format!(
                "<span>{} / {}</span>",
                escape_html(&name),
                escape_html(&state),
            ))
        })
        .collect::<Vec<_>>()
        .join("")
}

fn native_messaging_workspace_html(
    route_table: &str,
    responses: Option<&[EndpointBody]>,
    show_gate: bool,
) -> String {
    let preview = native_selection_preview_html(
        "Select a conversation",
        "Pick a thread, room, or pod row to load its reply and acknowledgement actions.",
        "Waiting",
        "Reply",
    );
    let gate = if show_gate {
        private_message_auto_response_panel_html(responses)
    } else {
        String::new()
    };
    let room_rows = native_message_room_rows_html(responses);
    let thread = native_message_thread_html(responses);
    let transcript = native_message_transcript_html(responses);
    let pods = native_message_pods_html(responses);
    format!(
        r#"<div class="slskr-native-grid messaging-native" data-slskr-messages-workspace><aside class="slskr-native-side slskr-native-sidebar"><h3>Messages</h3><div class="slskr-native-command-row"><input aria-label="Chat username" placeholder="username"><button type="button">Direct Message</button></div>{gate}<div class="slskr-native-message-search"><input aria-label="Search conversations" placeholder="Search conversations, rooms, pods"><button type="button">Clear Search</button></div><div class="slskr-native-list-stack"><h4>Conversations</h4>{route_table}<div class="slskr-native-mini-list" data-slskr-message-lifecycle><span>Unread badges reflect live conversation data</span><span>Acknowledge and delete use the selected conversation</span><span>Compose history is empty until a draft is entered</span></div><h4>Join Room</h4><div class="slskr-native-command-row"><input aria-label="Search rooms" placeholder="Search rooms"><button type="button">Join Room</button></div><div class="slskr-native-table-wrap"><table class="slskr-native-table" data-slskr-room-state><thead><tr><th>Room</th><th>State</th><th>Action</th></tr></thead><tbody>{room_rows}</tbody></table></div><h4>Pods</h4><div class="slskr-native-mini-list" data-slskr-pod-state>{pods}</div></div></aside><section class="slskr-native-main"><h3>Thread Workspace</h3>{preview}<div class="slskr-native-thread-grid" data-slskr-thread-state>{thread}</div><div class="slskr-native-thread-transcript" data-slskr-message-transcript>{transcript}</div><textarea aria-label="Message" placeholder="Message"></textarea><div class="slskr-native-command-row" data-slskr-message-actions><button type="button">Reply</button><button type="button">Acknowledge</button><button type="button">Delete Conversation</button><button type="button">Collapse All Message Panels</button></div><div class="slskr-native-mini-list" data-slskr-compose-history><span>Draft history is empty until a draft is composed</span><span>Enter sends when enabled</span></div></section></div>"#,
        gate = gate,
        route_table = route_table,
        preview = preview,
        room_rows = room_rows,
        thread = thread,
        transcript = transcript,
        pods = pods,
    )
}

fn native_search_filter_panel_html() -> String {
    r#"<section class="slskr-native-filter-modal" data-slskr-search-filter-modal><header><div><h4>Search Filters</h4><p>Format, bitrate, size, duration, queue, and duplicate controls stay visible beside results.</p></div><button type="button">Apply Filters</button></header><div class="slskr-native-filter-grid"><label><span>Include words</span><input aria-label="Include words" placeholder="remix instrumental"></label><label><span>Exclude words</span><input aria-label="Exclude words" placeholder="live demo"></label><label><span>Min bitrate</span><input aria-label="Min bitrate" placeholder="320"></label><label><span>Format</span><input aria-label="Format" placeholder="flac mp3 wav"></label><label><span>Min size</span><input aria-label="Min size" placeholder="1 MB"></label><label><span>Max size</span><input aria-label="Max size" placeholder="100 MB"></label><label><span>Min duration</span><input aria-label="Min duration" placeholder="3 min"></label><label><span>Max queue</span><input aria-label="Max queue" placeholder="8"></label></div><div class="slskr-native-filter-toggles"><label><input type="checkbox" aria-label="Fold duplicate results" checked> Fold duplicate results</label><label><input type="checkbox" aria-label="Prefer free upload slots" checked> Prefer free slots</label><label><input type="checkbox" aria-label="Hide locked files"> Hide locked files</label><select aria-label="Search ranking profile"><option>Smart ranking</option><option>Exact match first</option><option>Fastest peer first</option><option>Lossless first</option></select></div></section>"#.to_string()
}

fn native_download_policy_panel_html(responses: Option<&[EndpointBody]>) -> String {
    let terms = json_endpoint_value(responses, "/config/download-filter")
        .and_then(|value| value.get("exclude").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let options = endpoint_array(responses, "/destinations")
        .iter()
        .filter_map(|destination| {
            let path = value_text(destination, &["path", "directory"])?;
            let name = value_text(destination, &["name", "label"]).unwrap_or_else(|| path.clone());
            let default = value_bool(destination, &["isDefault", "default"]).unwrap_or(false);
            Some(format!(
                r#"<option value="{}"{}>{}{}</option>"#,
                escape_html(&path),
                if default {
                    " data-default=\"true\""
                } else {
                    ""
                },
                escape_html(&name),
                if default { " (default)" } else { "" },
            ))
        })
        .collect::<Vec<_>>()
        .join("");
    let options = if options.is_empty() {
        r#"<option value="">Configured default (loading)</option>"#.to_owned()
    } else {
        options
    };
    format!(
        r#"<section class="slskr-download-policy" data-slskr-download-policy><header><div><span class="slskr-wishlist-ignore-kicker">Download policy</span><h3>Destinations and exclusions</h3><p>Choose the destination for new browser-queued files and block literal terms before a peer is contacted.</p></div><span data-slskr-download-policy-status aria-live="polite">Live policy</span></header><div class="slskr-download-policy-grid"><label><span>Download destination</span><select aria-label="Download destination" data-slskr-download-destination>{options}</select></label><label><span>Blocked filename/path terms</span><textarea aria-label="Global download exclusions" data-slskr-download-filter>{terms}</textarea></label></div><p class="slskr-download-policy-help">One literal term per line; matching is case-insensitive and checks filenames and folder paths. Up to 100 terms, 256 characters each.</p><div class="slskr-native-command-row"><button type="button" data-slskr-download-filter-save>Save exclusions</button><button type="button" data-slskr-download-filter-reset>Reload policy</button></div></section>"#,
        options = options,
        terms = escape_html(&terms),
    )
}

fn native_system_configuration_panel_html(responses: Option<&[EndpointBody]>) -> String {
    let yaml = responses
        .and_then(|responses| endpoint_body(responses, "/options/yaml"))
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default();
    let location = responses
        .and_then(|responses| endpoint_body(responses, "/options/yaml/location"))
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "configuration path unavailable".to_owned());
    format!(
        r#"<section class="slskr-system-configuration" data-slskr-system-configuration><header><div><span class="slskr-wishlist-ignore-kicker">Configuration</span><h3>Live YAML settings</h3><p>Edit the active compatibility configuration, validate it, and apply it without leaving the System page.</p></div><code>{location}</code></header><textarea aria-label="Configuration YAML" data-slskr-options-yaml spellcheck="false" placeholder="Configuration YAML loads from the daemon">{yaml}</textarea><div class="slskr-native-command-row"><button type="button" data-slskr-options-yaml-validate>Validate YAML</button><button type="button" data-slskr-options-yaml-save>Save YAML</button></div><p data-slskr-options-yaml-status aria-live="polite">{state}</p></section>"#,
        location = escape_html(&location),
        yaml = escape_html(&yaml),
        state = if yaml.is_empty() {
            "Configuration YAML is unavailable until the daemon returns it."
        } else {
            "Loaded from the daemon."
        },
    )
}

#[allow(dead_code)]
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
            &["Open Window", "Join Room", "Restore Draft"],
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
            &["Open Item", "Stream Item", "Copy Exact Token"],
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

fn wishlist_ignored_results_panel_html(responses: Option<&[EndpointBody]>) -> String {
    let items = endpoint_array(responses, "/wishlist");
    let groups = items
        .iter()
        .filter_map(|item| {
            let item_id = value_text(item, &["id", "wishlistId", "searchId"])?;
            let label = value_text(item, &["searchText", "query", "text"])
                .unwrap_or_else(|| "Wanted search".to_string());
            let rules = item
                .get("ignoredResults")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let rule_rows = rules
                .iter()
                .filter_map(|rule| {
                    let rule_id = value_text(rule, &["id"])?;
                    let username = value_text(rule, &["username", "peer"])
                        .unwrap_or_else(|| "unknown peer".to_string());
                    let directory = value_text(rule, &["directory", "folder", "path"])
                        .unwrap_or_else(|| "unknown folder".to_string());
                    Some(format!(
                        r#"<li class="slskr-wishlist-ignore-rule"><span><strong>{username}</strong><code>{directory}</code></span><button type="button" data-slskr-wishlist-ignore-restore data-slskr-native-wishlist-id="{item_id}" data-slskr-wishlist-ignore-rule-id="{rule_id}">Restore</button></li>"#,
                        username = escape_html(&username),
                        directory = escape_html(&directory),
                        item_id = escape_html(&item_id),
                        rule_id = escape_html(&rule_id),
                    ))
                })
                .collect::<Vec<_>>();
            let rule_count = rule_rows.len();
            let rules_html = if rule_rows.is_empty() {
                r#"<li class="slskr-wishlist-ignore-empty">No ignored folders for this search.</li>"#
                    .to_string()
            } else {
                rule_rows.join("")
            };
            Some(format!(
                r#"<details class="slskr-wishlist-ignore-group" {open}><summary><span>{label}</span><small>{rule_count} blocked</small></summary><div class="slskr-wishlist-ignore-add" data-slskr-wishlist-ignore-panel data-slskr-native-wishlist-id="{item_id}"><label><span>Peer</span><input aria-label="Peer to ignore for {label}" placeholder="username"></label><label><span>Folder</span><input aria-label="Folder to ignore for {label}" placeholder="Music/Release"></label><button type="button" data-slskr-wishlist-ignore-submit>Ignore folder</button></div><ul>{rules_html}</ul></details>"#,
                open = if rule_count > 0 { "open" } else { "" },
                label = escape_html(&label),
                rule_count = rule_count,
                item_id = escape_html(&item_id),
                rules_html = rules_html,
            ))
        })
        .collect::<Vec<_>>();
    let body = if groups.is_empty() {
        r#"<p class="slskr-wishlist-ignore-panel-empty">Add a wanted search before creating folder rules.</p>"#
            .to_string()
    } else {
        groups.join("")
    };
    format!(
        r#"<section class="slskr-wishlist-ignore-panel" data-slskr-wishlist-ignore-manager><header><div><span class="slskr-wishlist-ignore-kicker">Noise gate</span><h3>Ignored result folders</h3></div><p>Block a peer and folder pair for one wanted search. Restoring it affects future matches; previously removed results stay removed.</p></header><div class="slskr-wishlist-ignore-groups">{body}</div></section>"#,
        body = body,
    )
}

fn wishlist_policy_history_panel_html(responses: Option<&[EndpointBody]>) -> String {
    let items = endpoint_array(responses, "/wishlist");
    let rows = items
        .iter()
        .filter_map(|item| {
            let item_id = value_text(item, &["id", "wishlistId", "searchId"])?;
            let label = value_text(item, &["searchText", "query", "text"])
                .unwrap_or_else(|| "Wanted search".to_string());
            let filter = value_text(item, &["filter", "searchFilter"]).unwrap_or_default();
            let enabled = value_bool(item, &["enabled"]).unwrap_or(true);
            let automatic =
                value_bool(item, &["autoDownload", "autoDownloadEnabled"]).unwrap_or(false);
            let number = |key: &str| {
                item.get(key)
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            };
            let max_results = number("maxResults").max(1);
            let max_downloads = item
                .get("maxDownloads")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let last_searched = item
                .get("lastSearchedAt")
                .and_then(serde_json::Value::as_u64);
            let last_viewed = item
                .get("lastViewedAt")
                .and_then(serde_json::Value::as_u64);
            let unseen = last_searched.is_some_and(|searched| {
                last_viewed.is_none_or(|viewed| searched > viewed)
            });
            Some(format!(
                r#"<details class="slskr-wishlist-policy-row" {open} data-slskr-wishlist-policy-panel data-slskr-native-wishlist-id="{item_id}"><summary><span><strong>{label}</strong><small>{filter_summary}</small></span><mark class="{status_class}">{status}</mark></summary><div class="slskr-wishlist-run-ledger"><span><strong>{visible}</strong> visible</span><span><strong>{locked}</strong> locked</span><span><strong>{filtered}</strong> filtered</span><span><strong>{ignored}</strong> ignored</span><span><strong>{responses}</strong> responses</span><span><strong>{searches}</strong> runs</span><span><strong>{downloads}</strong> downloads</span></div><div class="slskr-wishlist-policy-grid"><label><span>Result filter</span><input data-slskr-wishlist-policy-field="filter" aria-label="Result filter for {label}" value="{filter}"></label><label><span>Maximum results</span><input data-slskr-wishlist-policy-field="maxResults" aria-label="Maximum results for {label}" inputmode="numeric" value="{max_results}"></label><label><span>Maximum downloads</span><input data-slskr-wishlist-policy-field="maxDownloads" aria-label="Maximum downloads for {label}" inputmode="numeric" placeholder="One-shot" value="{max_downloads}"></label><label class="slskr-wishlist-policy-check"><input type="checkbox" data-slskr-wishlist-policy-field="enabled" {enabled}> Enabled</label><label class="slskr-wishlist-policy-check"><input type="checkbox" data-slskr-wishlist-policy-field="autoDownload" {automatic}> Auto-download</label></div><div class="slskr-wishlist-policy-actions"><button type="button" data-slskr-wishlist-policy-save>Save policy</button><button type="button" data-slskr-wishlist-mark-viewed>Mark viewed</button><button type="button" data-slskr-wishlist-load-history>Load run ledger</button></div><div class="slskr-wishlist-history-results" data-slskr-wishlist-history-results><p>Load the run ledger to inspect recent searches.</p></div></details>"#,
                open = if unseen { "open" } else { "" },
                item_id = escape_html(&item_id),
                label = escape_html(&label),
                filter_summary = escape_html(if filter.is_empty() { "all formats" } else { &filter }),
                status_class = if unseen { "is-unseen" } else { "" },
                status = if unseen { "new results" } else { "reviewed" },
                visible = number("lastVisibleHitCount"),
                locked = number("lastHiddenLockedHitCount"),
                filtered = number("lastFilteredOutHitCount"),
                ignored = number("lastIgnoredResultHitCount"),
                responses = number("lastResponseCount"),
                searches = number("totalSearchCount"),
                downloads = number("totalDownloadCount"),
                filter = escape_html(&filter),
                max_results = max_results,
                max_downloads = escape_html(&max_downloads),
                enabled = if enabled { "checked" } else { "" },
                automatic = if automatic { "checked" } else { "" },
            ))
        })
        .collect::<Vec<_>>();
    let body = if rows.is_empty() {
        r#"<p class="slskr-wishlist-policy-empty">Add a wanted search to configure its policy and inspect runs.</p>"#.to_string()
    } else {
        rows.join("")
    };
    format!(
        r#"<section class="slskr-wishlist-policy-panel" data-slskr-wishlist-policy-manager><header><div><span class="slskr-wishlist-ignore-kicker">Run ledger</span><h3>Search policy and history</h3></div><p>Control result limits and automation, then inspect what each run kept or rejected.</p></header><div class="slskr-wishlist-policy-rows">{body}</div></section>"#,
        body = body,
    )
}

fn download_request_workspace_html(responses: Option<&[EndpointBody]>) -> String {
    const COLUMNS: &[(&str, &str, bool)] = &[
        ("name", "Name", true),
        ("peer", "Peer", true),
        ("type", "Type", false),
        ("size", "Size", true),
        ("progress", "Progress", true),
        ("bitrate", "Bitrate", true),
        ("samplerate", "Sample rate", false),
        ("bitdepth", "Bit depth", false),
        ("length", "Length", true),
        ("state", "State", true),
        ("folder", "Folder", false),
        ("added", "Added", false),
        ("actions", "Actions", true),
    ];
    let chooser = COLUMNS
        .iter()
        .map(|(key, label, visible)| {
            format!(
                r#"<label><input type="checkbox" data-slskr-transfer-column-toggle="{key}" {checked}> {label}</label>"#,
                key = key,
                label = escape_html(label),
                checked = if *visible { "checked" } else { "" },
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let headers = COLUMNS
        .iter()
        .map(|(key, label, visible)| {
            format!(
                r#"<th data-slskr-transfer-column="{key}" {hidden}><span>{label}</span><button type="button" class="slskr-transfer-column-resize" data-slskr-transfer-column-resize="{key}" aria-label="Resize {label}">↔</button></th>"#,
                key = key,
                label = escape_html(label),
                hidden = if *visible { "" } else { "hidden" },
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let rows = endpoint_array(responses, "/downloads/requests")
        .iter()
        .take(100)
        .filter_map(|item| {
            let request_id = value_text(item, &["request.id"])?;
            let name = value_text(item, &["request.name"])
                .unwrap_or_else(|| "Download request".to_string());
            let filename = value_text(item, &["request.originalFilename"]).unwrap_or_default();
            let peer = value_text(item, &["current.peer_username", "current.username"])
                .unwrap_or_else(|| "waiting for source".to_string());
            let extension = filename
                .rsplit_once('.')
                .map(|(_, extension)| extension.to_ascii_uppercase())
                .unwrap_or_else(|| "FILE".to_string());
            let size = value_text(item, &["request.size"]).unwrap_or_else(|| "—".to_string());
            let transferred = value_text(item, &["current.bytes_transferred"])
                .unwrap_or_else(|| "0".to_string());
            let bit_rate = value_text(item, &["request.bitRate"])
                .map(|value| format!("{value} kbps"))
                .unwrap_or_else(|| "—".to_string());
            let sample_rate = value_text(item, &["request.sampleRate"])
                .map(|value| format!("{value} Hz"))
                .unwrap_or_else(|| "—".to_string());
            let bit_depth = value_text(item, &["request.bitDepth"])
                .map(|value| format!("{value}-bit"))
                .unwrap_or_else(|| "—".to_string());
            let length = value_text(item, &["request.length"])
                .map(|value| format!("{value}s"))
                .unwrap_or_else(|| "—".to_string());
            let state = value_text(item, &["request.state"])
                .unwrap_or_else(|| "Active".to_string());
            let folder = value_text(item, &["request.destinationDirectory"])
                .unwrap_or_else(|| "path template".to_string());
            let added = value_text(item, &["request.createdAt"])
                .unwrap_or_else(|| "—".to_string());
            let attempts = value_text(item, &["attemptCount"]).unwrap_or_else(|| "1".to_string());
            let artist = value_text(item, &["request.artist"]).unwrap_or_default();
            let title = value_text(item, &["request.title"]).unwrap_or_default();
            let secondary = match (artist.is_empty(), title.is_empty()) {
                (false, false) => format!("{artist} — {title}"),
                _ => filename.clone(),
            };
            let current_id = value_text(item, &["current.id"]).unwrap_or_default();
            let recovery_action = value_text(item, &["current.recovery_action"]);
            let recovery_label = value_text(item, &["current.recovery_label"])
                .unwrap_or_else(|| "Retry".to_string());
            let recovery = if recovery_action.as_deref() == Some("wait") {
                format!(r#"<button type="button" disabled title="The peer must reconnect before this request can continue">{}</button>"#, escape_html(&recovery_label))
            } else if recovery_action.as_deref() == Some("retry") && !current_id.is_empty() {
                format!(r#"<button type="button" data-slskr-transfer-retry="{}">{}</button>"#, escape_html(&current_id), escape_html(&recovery_label))
            } else {
                String::new()
            };
            Some(format!(
                r#"<tr data-slskr-download-request-row data-slskr-download-request-id="{request_id}"><td data-slskr-transfer-column="name"><span class="slskr-transfer-request-name"><strong>{name}</strong><small>{secondary}</small></span></td><td data-slskr-transfer-column="peer"><span class="slskr-transfer-attempt-spine"><i></i><span>{peer}</span><small>{attempts} attempt(s)</small></span></td><td data-slskr-transfer-column="type" hidden>{extension}</td><td data-slskr-transfer-column="size">{size}</td><td data-slskr-transfer-column="progress"><span>{transferred} / {size}</span></td><td data-slskr-transfer-column="bitrate">{bit_rate}</td><td data-slskr-transfer-column="samplerate" hidden>{sample_rate}</td><td data-slskr-transfer-column="bitdepth" hidden>{bit_depth}</td><td data-slskr-transfer-column="length">{length}</td><td data-slskr-transfer-column="state"><mark>{state}</mark></td><td data-slskr-transfer-column="folder" hidden><code>{folder}</code></td><td data-slskr-transfer-column="added" hidden>{added}</td><td data-slskr-transfer-column="actions"><div class="slskr-transfer-request-actions">{recovery}<button type="button" data-slskr-transfer-attempts>Attempts</button><button type="button" data-slskr-transfer-rename>Rename</button><button type="button" data-slskr-transfer-request-cancel>Cancel</button></div><div class="slskr-transfer-request-inline" data-slskr-transfer-request-inline><input aria-label="Rename {name}" value="{name}"><button type="button" data-slskr-transfer-rename-save>Save name</button></div><div class="slskr-transfer-attempt-results" data-slskr-transfer-attempt-results></div></td></tr>"#,
                request_id = escape_html(&request_id),
                name = escape_html(&name),
                secondary = escape_html(&secondary),
                peer = escape_html(&peer),
                attempts = escape_html(&attempts),
                extension = escape_html(&extension),
                size = escape_html(&size),
                transferred = escape_html(&transferred),
                bit_rate = escape_html(&bit_rate),
                sample_rate = escape_html(&sample_rate),
                bit_depth = escape_html(&bit_depth),
                length = escape_html(&length),
                state = escape_html(&state),
                folder = escape_html(&folder),
                added = escape_html(&added),
                recovery = recovery,
            ))
        })
        .collect::<Vec<_>>();
    let body = if rows.is_empty() {
        format!(
            r#"<tr><td colspan="{}"><div class="slskr-native-empty"><strong>No download requests</strong><span>Queue a file from search or browse to create the first request.</span></div></td></tr>"#,
            COLUMNS.len()
        )
    } else {
        rows.join("")
    };
    format!(
        r#"<section class="slskr-transfer-request-workspace" data-slskr-transfer-request-workspace><header><div><span class="slskr-wishlist-ignore-kicker">Request desk</span><h3>Downloads and attempts</h3><p>Each row is the file you asked for. Source changes stay in its attempt trail.</p></div><details class="slskr-transfer-column-chooser"><summary>Columns</summary><div>{chooser}</div><button type="button" data-slskr-transfer-columns-reset>Reset columns</button></details></header><div class="slskr-native-table-wrap"><table class="slskr-native-table slskr-transfer-request-table"><thead><tr>{headers}</tr></thead><tbody>{body}</tbody></table></div></section>"#,
        chooser = chooser,
        headers = headers,
        body = body,
    )
}

#[cfg(any(target_arch = "wasm32", test))]
#[cfg(any(target_arch = "wasm32", test))]
fn wishlist_history_response_html(response: &str) -> String {
    let Ok(serde_json::Value::Array(records)) = serde_json::from_str(response) else {
        return "<p>Run history could not be read.</p>".to_string();
    };
    if records.is_empty() {
        return "<p>No completed or active runs are retained for this search.</p>".to_string();
    }
    let rows = records
        .iter()
        .take(20)
        .map(|record| {
            let query = value_text(record, &["searchText", "query"])
                .unwrap_or_else(|| "search".to_string());
            let status = value_text(record, &["status", "state"])
                .unwrap_or_else(|| "unknown".to_string());
            let started = value_text(record, &["startedAt", "created_at"])
                .unwrap_or_else(|| "unknown time".to_string());
            let results = record
                .get("result_count")
                .or_else(|| record.get("fileCount"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            format!(
                "<li><time>{}</time><span><strong>{}</strong><small>{} · {} results</small></span></li>",
                escape_html(&started),
                escape_html(&query),
                escape_html(&status),
                results,
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(r#"<ol class="slskr-wishlist-history-list">{rows}</ol>"#)
}

fn route_native_workspace_html(
    kind: RouteKind,
    rows: &[(String, String, String, String)],
    responses: Option<&[EndpointBody]>,
) -> String {
    let route_table = native_route_table_html(kind, rows);
    let html = match kind {
        RouteKind::Search => format!(
            r#"<div class="slskr-native-grid search-native"><section class="slskr-native-main"><h3>Searches</h3><div class="slskr-native-command-row"><input aria-label="Search text" placeholder="Search"><select aria-label="Acquisition profile"><option>Balanced</option><option>Lossless exact</option><option>Fast good enough</option></select><button type="button">Search</button><button type="button">Stop</button><button type="button">Clear</button><button type="button" data-slskr-search-bulk="research">Search selected again</button><button type="button" data-slskr-search-bulk="stop">Stop selected</button><button type="button" data-slskr-search-bulk="remove">Delete selected</button></div>{filter_panel}{route_table}</section><aside class="slskr-native-side"><h3>Search Detail</h3><p>Select searches to inspect files, peers, queue, warnings, duplicate groups, and download preview.</p>{preview}{stats}<div class="slskr-native-mini-list" data-slskr-search-expansion><span>Expanded directories</span><span>Locked file warnings</span><span>Folded duplicate sources</span><span>Ranking reasons</span></div></aside></div>"#,
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
            r#"<div class="slskr-native-grid wishlist-native"><section class="slskr-native-main"><h3>Wishlist</h3><div class="slskr-native-command-row"><textarea aria-label="Search Text" placeholder="Enter search terms, one per line..." rows="3"></textarea><input aria-label="Filter optional" placeholder="e.g., flac OR mp3"><input aria-label="Max Results" value="25"><button type="button">Add Search</button><button type="button">Import List</button><button type="button">Run Enabled</button></div>{route_table}{policy}{ignored}</section><aside class="slskr-native-side"><h3>Request Portal Summary</h3>{preview}{stats}<button type="button">Copy Review</button></aside></div>"#,
            route_table = route_table,
            policy = wishlist_policy_history_panel_html(responses),
            ignored = wishlist_ignored_results_panel_html(responses),
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
            let request_workspace = if kind == RouteKind::Downloads {
                download_request_workspace_html(responses)
            } else {
                String::new()
            };
            let policy = if kind == RouteKind::Downloads {
                native_download_policy_panel_html(responses)
            } else {
                String::new()
            };
            format!(
                r#"<div class="slskr-native-grid transfers-native"><section class="slskr-native-main"><h3>{title}</h3><div class="slskr-native-command-row"><button type="button">{primary}</button><button type="button">{secondary}</button><button type="button">Clear Completed</button><label><input type="checkbox"> Accelerated</label><label><input type="checkbox"> Auto Replace</label></div>{policy}{request_workspace}{table}</section><aside class="slskr-native-side"><h3>Transfer Group</h3>{preview}{stats}</aside></div>"#,
                title = title,
                primary = primary,
                secondary = secondary,
                policy = policy,
                request_workspace = request_workspace,
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
        RouteKind::Messages | RouteKind::Rooms => {
            native_messaging_workspace_html(&route_table, responses, kind == RouteKind::Messages)
        }
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
        RouteKind::Solid => {
            let solid = json_endpoint_value(responses, "/solid/status");
            let enabled = solid
                .as_ref()
                .and_then(|value| value_bool(value, &["enabled"]))
                .unwrap_or(false);
            let state = solid
                .as_ref()
                .and_then(|value| value_text(value, &["status", "state"]))
                .unwrap_or_else(|| {
                    if enabled {
                        "enabled".to_string()
                    } else if solid.is_some() {
                        "disabled".to_string()
                    } else {
                        "not queried".to_string()
                    }
                });
            let description = if enabled {
                format!("Solid integration is enabled ({state}).")
            } else if solid.is_some() {
                format!("Solid integration is disabled ({state}).")
            } else {
                "Solid status was not returned by the daemon.".to_string()
            };
            let document = solid
                .as_ref()
                .map(|value| compact_preview(&value.to_string()))
                .unwrap_or_else(|| "No Solid status document returned.".to_string());
            format!(
                r#"<div class="slskr-native-grid solid-native"><section class="slskr-native-main" data-testid="solid-root"><h3>Solid</h3><p>{description}</p><div class="slskr-native-command-row"><input data-testid="solid-webid-input" aria-label="WebID" placeholder="https://example.com/profile/card#me"><button data-testid="solid-resolve-webid" type="button">Resolve WebID</button></div>{route_table}</section><aside class="slskr-native-side"><h3>Identity Document</h3>{preview}<pre>{document}</pre></aside></div>"#,
                description = escape_html(&description),
                document = escape_html(&document),
                route_table = route_table,
                preview = native_selection_preview_html(
                    "No Solid resource selected",
                    "Select an identity, storage, session, or sync row to inspect setup status.",
                    &state,
                    "Resolve WebID",
                ),
            )
        }
        RouteKind::Collections => format!(
            r#"<div class="slskr-native-grid collections-native"><section class="slskr-native-main"><h3>Collections</h3><div class="slskr-native-command-row"><input aria-label="Title" placeholder="Enter collection title"><input aria-label="Description" placeholder="Optional description"><button type="button">Create Collection</button></div><div class="slskr-native-command-row"><input aria-label="Content ID" placeholder="Library content ID"><input aria-label="Soulseek Username" placeholder="Username for share grant"><button type="button">Add Item</button><button type="button">Share</button></div><div class="slskr-native-split-detail"><div>{route_table}<h4>Items in selected collection</h4>{items}</div><aside><h4>Item Picker</h4><p>Select a collection above, enter a library content ID, and add the item through the collection API.</p>{preview}<div class="slskr-native-mini-list"><span>Library records are loaded from the daemon</span><span>Collection membership is persisted after add</span><span>Already-in-collection errors are returned by the API</span></div></aside></div></section><aside class="slskr-native-side"><h3>Collection Detail</h3>{stats}<div class="slskr-native-mini-list"><span>Items table loads from the selected collection</span><span>Remove item uses the collection item API</span><span>Audience picker uses share grants</span><span>Stream/download policies are returned by the grant</span></div></aside></div>"#,
            route_table = route_table,
            items = native_collection_items_html(responses),
            preview = native_selection_preview_html(
                "No collection selected",
                "Choose a collection or library candidate before adding, removing, or sharing items.",
                "Waiting",
                "Review",
            ),
            stats = [
                native_stat_html(
                    "Title",
                    &endpoint_array(responses, "/collections")
                        .first()
                        .and_then(|item| value_text(item, &["title", "name"]))
                        .unwrap_or_else(|| "not selected".to_string()),
                ),
                native_stat_html(
                    "Type",
                    &endpoint_array(responses, "/collections")
                        .first()
                        .and_then(|item| value_text(item, &["type", "kind"]))
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                native_stat_html(
                    "Items",
                    &endpoint_array(responses, "/collections")
                        .first()
                        .and_then(|item| value_count(item, &["itemCount", "itemsCount", "items"]))
                        .unwrap_or_else(|| "0".to_string()),
                ),
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
            r#"<div class="slskr-native-grid shared-native"><section class="slskr-native-main"><h3>Shared with Me</h3><div class="slskr-native-split-detail"><div>{route_table}</div><aside><h4>Shared Manifest</h4><p>Owner, expiration, permissions, and file-level access preview.</p>{preview}<div class="slskr-native-mini-list"><span>Manifest item rows</span><span>Stream available files</span><span>Backfill selected collection</span></div></aside></div></section><aside class="slskr-native-side"><h3>Access</h3><button type="button">Open</button><button type="button">Stream</button><button type="button">Backfill</button><button type="button">Copy token</button></aside></div>"#,
            route_table = route_table,
            preview = native_selection_preview_html(
                "No shared item selected",
                "Choose an inbound grant or manifest row before opening or backfilling.",
                "Waiting",
                "Open",
            ),
        ),
        RouteKind::Browse => native_browse_workspace_html(&route_table, responses),
        RouteKind::System => format!(
            r#"<div class="slskr-native-grid system-native"><section class="slskr-native-main"><h3>System</h3><div class="slskr-native-operator-bands"><span>Connection</span><span>Shares</span><span>Database</span><span>Events</span><span>Preferences</span><span>Automation</span></div>{config}{route_table}</section><aside class="slskr-native-side"><h3>Operator Actions</h3>{preview}<button type="button">Check for Updates</button><button type="button">Get Privileges</button><button type="button">Diagnostic Bundle</button><button type="button">Setup Health</button></aside></div>"#,
            config = native_system_configuration_panel_html(responses),
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
        tabs = native_tabs_html(kind, responses),
        filter = native_filter_html(),
        // The old parity panel was a collection of unconnected buttons that
        // claimed to cover features without issuing a route action.  The
        // live tables, reference actions, and route editors above are the
        // executable surfaces; do not render a decorative control inventory.
        final_parity = "",
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

fn route_workflow_toolbar_html(kind: RouteKind) -> String {
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

fn route_workflow_html(path: &str, responses: Option<&[EndpointBody]>) -> String {
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

fn player_footer_html() -> String {
    r#"<footer class="slskr-player" data-slskr-player data-slskr-player-rating-key="" data-slskr-player-radio-query=""><section><strong>Now Playing</strong><span id="slskr-player-now">Queue idle</span><span id="slskr-player-now-detail">No local stream selected</span><audio id="slskr-player-audio" preload="metadata" controls></audio></section><section class="slskr-player-controls" aria-label="Player controls"><button type="button" data-slskr-player-action="play">Play</button><button type="button" data-slskr-player-action="refresh">Refresh</button><button type="button" data-slskr-player-action="clear">Clear</button><button type="button" data-slskr-player-action="visualizer">Visualizer</button><button type="button" data-slskr-player-action="radio">Radio</button></section><section class="slskr-player-rating" aria-label="Player rating"><strong>Rating</strong><div id="slskr-player-rating-controls"><button type="button" data-slskr-player-rating="1">1</button><button type="button" data-slskr-player-rating="2">2</button><button type="button" data-slskr-player-rating="3">3</button><button type="button" data-slskr-player-rating="4">4</button><button type="button" data-slskr-player-rating="5">5</button></div><span id="slskr-player-rating-status">Not rated</span></section><section><strong>Radio</strong><span id="slskr-player-radio">No track selected</span><span id="slskr-player-transfers">0 down / 0 up</span></section><section><strong>Visualizer</strong><span id="slskr-player-visualizer">Checking status</span><span id="slskr-player-status" aria-live="polite">Rust player surface ready</span></section></footer>"#.to_string()
}

fn rustymilk_panel_html() -> String {
    r#"<section class="slskr-rustymilk-panel" id="slskr-rust-rustymilk" hidden data-slskr-rustymilk-running="false" data-slskr-rustymilk-favorites-only="false" data-slskr-rustymilk-search="" data-slskr-rustymilk-playlist="" data-slskr-rustymilk-automation="off" data-slskr-rustymilk-fps="full" data-slskr-rustymilk-quality="balanced" data-slskr-rustymilk-debug="false"><header><div><strong>RustyMilk</strong><span id="slskr-rustymilk-preset">slskr native grid smoke</span></div><div class="slskr-rustymilk-actions"><button type="button" data-slskr-rustymilk-action="previous">Previous</button><button type="button" data-slskr-rustymilk-action="preset">Next</button><button type="button" data-slskr-rustymilk-action="random">Random</button><button type="button" data-slskr-rustymilk-action="favorite" id="slskr-rustymilk-favorite">Favorite</button><button type="button" data-slskr-rustymilk-action="favorites" id="slskr-rustymilk-favorites-only">Favorites</button><button type="button" data-slskr-rustymilk-action="remove">Remove</button><button type="button" data-slskr-rustymilk-action="import">Import</button><button type="button" data-slskr-rustymilk-action="texture">Texture</button><button type="button" data-slskr-rustymilk-action="pack">Pack</button><button type="button" data-slskr-rustymilk-action="clear">Clear</button><button type="button" data-slskr-rustymilk-action="reset">Reset</button><button type="button" data-slskr-rustymilk-action="external">External</button><button type="button" data-slskr-rustymilk-action="close">Close</button></div></header><div class="slskr-rustymilk-library"><input id="slskr-rustymilk-search" aria-label="Search RustyMilk presets" placeholder="Search preset library"><button type="button" data-slskr-rustymilk-action="search">Search</button><button type="button" data-slskr-rustymilk-action="clear-search">Clear search</button><button type="button" data-slskr-rustymilk-action="save-playlist">Save playlist</button><button type="button" data-slskr-rustymilk-action="playlist">Playlist</button><button type="button" data-slskr-rustymilk-action="rename-playlist">Rename</button><button type="button" data-slskr-rustymilk-action="clear-playlist">All presets</button><button type="button" data-slskr-rustymilk-action="remove-playlist">Remove playlist</button><input id="slskr-rustymilk-preset-input" type="file" accept=".milk,.milk2,.txt,text/plain" multiple hidden><input id="slskr-rustymilk-texture-input" type="file" accept="image/png,image/jpeg,image/webp,image/gif" multiple hidden><input id="slskr-rustymilk-pack-input" type="file" accept=".milk,.milk2,.txt,text/plain,image/png,image/jpeg,image/webp,image/gif" multiple webkitdirectory hidden><span id="slskr-rustymilk-library-status">3 bundled presets</span><span id="slskr-rustymilk-textures">0 texture assets</span></div><div class="slskr-rustymilk-editor"><select id="slskr-rustymilk-parameter" aria-label="RustyMilk parameter"><option value="decay">Decay</option><option value="zoom">Zoom</option><option value="rot">Rotation</option><option value="wave_r">Wave red</option><option value="wave_g">Wave green</option><option value="wave_b">Wave blue</option><option value="wave_a">Wave alpha</option></select><input id="slskr-rustymilk-parameter-value" aria-label="RustyMilk parameter value" value="0.9"><button type="button" data-slskr-rustymilk-action="apply-parameter">Apply</button><button type="button" data-slskr-rustymilk-action="randomize-parameters">Randomize</button><button type="button" data-slskr-rustymilk-action="import-shape">Import shape</button><button type="button" data-slskr-rustymilk-action="export-shape">Export shape</button><button type="button" data-slskr-rustymilk-action="remove-shape">Remove shape</button><button type="button" data-slskr-rustymilk-action="import-wave">Import wave</button><button type="button" data-slskr-rustymilk-action="export-wave">Export wave</button><button type="button" data-slskr-rustymilk-action="remove-wave">Remove wave</button><button type="button" data-slskr-rustymilk-action="export-preset">Export preset</button><button type="button" data-slskr-rustymilk-action="automation">Automation</button><select id="slskr-rustymilk-automation-beats" aria-label="RustyMilk automation beats"><option value="4">4 beats</option><option value="8" selected>8 beats</option><option value="16">16 beats</option></select><select id="slskr-rustymilk-automation-interval" aria-label="RustyMilk automation interval"><option value="15">15 sec</option><option value="30" selected>30 sec</option><option value="60">60 sec</option></select><select id="slskr-rustymilk-fps" aria-label="RustyMilk FPS cap"><option value="full">Full FPS</option><option value="60">60 FPS</option><option value="30">30 FPS</option><option value="24">24 FPS</option></select><select id="slskr-rustymilk-quality" aria-label="RustyMilk quality"><option value="balanced" selected>Balanced</option><option value="efficient">Efficient</option><option value="full">Full</option><option value="custom">Custom</option></select><button type="button" data-slskr-rustymilk-action="debug">Debug</button></div><canvas id="slskr-rustymilk-canvas" width="960" height="360" aria-label="RustyMilk visualizer"></canvas><footer><span id="slskr-rustymilk-status">Visualizer ready</span><span id="slskr-rustymilk-renderer">Renderer checking</span></footer><pre id="slskr-rustymilk-debug" hidden></pre></section>"#.to_string()
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
        r#"<div class="slskr-shell"><nav class="slskr-nav">{nav}</nav><main class="slskr-main"><header class="slskr-appbar"><div><strong>slskr</strong><span>Search, transfers, messages, rooms, browse, sharing, and system control</span><span class="slskr-appbar-support"><b>Keep the node moving</b><a href="https://www.paypal.com/donate/?business=donations%40snape.tech" target="_blank" rel="noopener noreferrer">PayPal</a><a href="https://ko-fi.com/snapetech" target="_blank" rel="noopener noreferrer">Ko-fi</a></span></div><ul id="slskr-runtime-status">{runtime}</ul></header><section id="slskr-route-view">{route_page}</section></main>{rustymilk}{player}</div>"#,
        route_page = route_page_html("/searches"),
        runtime = runtime_probe_pending_html(),
        rustymilk = rustymilk_panel_html(),
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
    mount_reference_actions(window, document)?;
    mount_workspace_tabs(document)?;
    mount_data_cards(document)?;
    mount_native_tables(document)?;
    mount_native_subviews(document)?;
    mount_native_actions(document)?;
    mount_transfer_columns(document)?;
    mount_download_policy_controls(document)?;
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
    let buttons = document.query_selector_all(".slskr-native-workspace button:not([data-slskr-native-tab]):not([data-slskr-native-filter-clear]):not([data-slskr-native-select-visible]):not([data-slskr-native-clear-selection]):not([data-slskr-native-reset-state]):not([data-slskr-transfer-column-resize]):not([data-slskr-pref-action]):not([data-slskr-automation-action]):not([data-slskr-recipe-dry-run]):not([data-slskr-recipe-copy])")?;
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
    if handle_search_bulk_action(document, button) {
        return;
    }
    if handle_download_policy_action(document, button) {
        return;
    }
    if handle_options_yaml_action(document, button) {
        return;
    }
    if handle_transfer_request_action(document, button) {
        return;
    }
    if handle_private_message_auto_response_action(document, button) {
        return;
    }
    if handle_wishlist_policy_action(document, button) {
        return;
    }
    if handle_wishlist_ignored_result_action(document, button) {
        return;
    }
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
    if route_kind(&route_path) == RouteKind::Browse
        && button.get_attribute("data-slskr-browse-folder").is_some()
    {
        if let Some(folder) = button.get_attribute("data-slskr-browse-folder") {
            if let Ok(Some(input)) = document.query_selector(r#"input[aria-label="Folder"]"#) {
                if let Some(input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                    input.set_value(&folder);
                }
            }
            if let Some(action) = route_action_for_native_label(&route_path, "Browse") {
                run_native_route_action(document, button, action);
            }
        }
        return;
    }
    if handle_native_local_action(document, button, &route_path, &action) {
        return;
    }
    if let Some(route_action) = route_action_for_native_label(&route_path, &action) {
        run_native_route_action(document, button, route_action);
        return;
    }
    set_reference_status(
        document,
        &action,
        &format!(
            "No executable UI contract is registered for the {} route.",
            route_label
        ),
    );
}

#[cfg(target_arch = "wasm32")]
fn handle_search_bulk_action(document: &web_sys::Document, button: &web_sys::Element) -> bool {
    let Some(mode) = button.get_attribute("data-slskr-search-bulk") else {
        return false;
    };
    let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() else {
        show_toast(document, "Search selection is unavailable");
        return true;
    };
    let Ok(rows) = workspace.query_selector_all(
        "[data-slskr-native-select][aria-selected=\"true\"][data-slskr-native-search-id]",
    ) else {
        return true;
    };
    let mut selected = Vec::new();
    for index in 0..rows.length() {
        let Some(node) = rows.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(id) = row
            .get_attribute("data-slskr-native-search-id")
            .filter(|value| safe_route_segment(value))
        else {
            continue;
        };
        let query = row
            .get_attribute("data-slskr-native-search-text")
            .unwrap_or_default()
            .trim()
            .to_owned();
        selected.push((id, query));
    }
    if selected.is_empty() {
        show_toast(document, "Select one or more saved searches first");
        return true;
    }
    if mode == "research" && selected.iter().all(|(_, query)| query.is_empty()) {
        show_toast(document, "Selected searches contain no searchable text");
        return true;
    }
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    let label = match mode.as_str() {
        "research" => "Search selected again",
        "stop" => "Stop selected",
        "remove" => "Delete selected",
        _ => "Search action",
    };
    show_toast(&document, &format!("{label} sending"));
    let total = selected.len();
    wasm_bindgen_futures::spawn_local(async move {
        let mut completed = 0usize;
        for (id, query) in selected {
            let (method, path, body) = match mode.as_str() {
                "research" if !query.is_empty() => (
                    "POST",
                    endpoint_url("/searches"),
                    Some(format!(
                        r#"{{"searchText":"{}"}}"#,
                        escape_json_string(&query)
                    )),
                ),
                "stop" => ("PUT", endpoint_url(&format!("/searches/{id}")), None),
                "remove" => ("DELETE", endpoint_url(&format!("/searches/{id}")), None),
                _ => continue,
            };
            if fetch_text_with_method(&window, &path, method, body.as_deref())
                .await
                .is_ok()
            {
                completed += 1;
            }
        }
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html(&format!(
                "<strong>{}</strong> completed {} of {}",
                escape_html(label),
                completed,
                total,
            ));
        }
        let _ = refresh_route_data(&window).await;
    });
    true
}

#[cfg(target_arch = "wasm32")]
fn mount_download_policy_controls(document: &web_sys::Document) -> Result<(), JsValue> {
    let selects = document.query_selector_all("[data-slskr-download-destination]")?;
    for index in 0..selects.length() {
        let Some(node) = selects.item(index) else {
            continue;
        };
        let select: web_sys::HtmlSelectElement = node.dyn_into()?;
        if select.has_attribute("data-slskr-mounted") {
            continue;
        }
        select.set_attribute("data-slskr-mounted", "true")?;
        if let Some(saved) = document
            .default_view()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| {
                storage
                    .get_item("slskr-download-destination")
                    .ok()
                    .flatten()
            })
        {
            if (0..select.length()).any(|option_index| {
                select
                    .item(option_index)
                    .is_some_and(|option| option.get_attribute("value").as_deref() == Some(&saved))
            }) {
                select.set_value(&saved);
            }
        }
        let document_for_change = document.clone();
        let select_for_change = select.clone();
        let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
            let value = select_for_change.value();
            if let Some(storage) = document_for_change
                .default_view()
                .and_then(|window| window.local_storage().ok().flatten())
            {
                let _ = storage.set_item("slskr-download-destination", &value);
            }
            if let Some(status) = document_for_change
                .query_selector("[data-slskr-download-policy-status]")
                .ok()
                .flatten()
            {
                status.set_text_content(Some(if value.trim().is_empty() {
                    "Using configured default"
                } else {
                    "Destination saved for this browser"
                }));
            }
        }));
        select.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn handle_download_policy_action(document: &web_sys::Document, button: &web_sys::Element) -> bool {
    let save = button.has_attribute("data-slskr-download-filter-save");
    let reset = button.has_attribute("data-slskr-download-filter-reset");
    if !save && !reset {
        return false;
    }
    let Some(panel) = button
        .closest("[data-slskr-download-policy]")
        .ok()
        .flatten()
    else {
        show_toast(document, "Download policy editor is unavailable");
        return true;
    };
    if reset {
        let Some(window) = document.default_view() else {
            return true;
        };
        show_toast(document, "Reloading download policy");
        let document = document.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = fetch_text(&window, &endpoint_url("/config/download-filter")).await;
            if let Some(status) = document
                .query_selector("[data-slskr-download-policy-status]")
                .ok()
                .flatten()
            {
                status.set_text_content(Some(match result {
                    Ok(_) => "Policy reloaded; refresh the page data to apply it",
                    Err(_) => "Policy reload failed",
                }));
            }
            let _ = refresh_route_data(&window).await;
        });
        return true;
    }
    let term_text = panel
        .query_selector("[data-slskr-download-filter]")
        .ok()
        .flatten()
        .and_then(|element| form_control_value(&element))
        .unwrap_or_default();
    let terms = term_text
        .lines()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    if terms.len() > 100 || terms.iter().any(|term| term.chars().count() > 256) {
        show_toast(document, "Use at most 100 terms of 256 characters each");
        return true;
    }
    let body = format!(
        r#"{{"exclude":[{}]}}"#,
        terms
            .iter()
            .map(|term| format!(r#""{}""#, escape_json_string(term)))
            .collect::<Vec<_>>()
            .join(",")
    );
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    show_toast(&document, "Saving download exclusions");
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(
            &window,
            &endpoint_url("/config/download-filter"),
            "PUT",
            Some(&body),
        )
        .await;
        if let Some(status) = document
            .query_selector("[data-slskr-download-policy-status]")
            .ok()
            .flatten()
        {
            status.set_text_content(Some(match &result {
                Ok(_) => "Download exclusions saved",
                Err(_) => "Download exclusions could not be saved",
            }));
        }
        if result.is_ok() {
            let _ = refresh_route_data(&window).await;
        }
    });
    true
}

#[cfg(target_arch = "wasm32")]
fn handle_options_yaml_action(document: &web_sys::Document, button: &web_sys::Element) -> bool {
    let validate = button.has_attribute("data-slskr-options-yaml-validate");
    let save = button.has_attribute("data-slskr-options-yaml-save");
    if !validate && !save {
        return false;
    }
    let Some(panel) = button
        .closest("[data-slskr-system-configuration]")
        .ok()
        .flatten()
    else {
        show_toast(document, "Configuration editor is unavailable");
        return true;
    };
    let yaml = panel
        .query_selector("[data-slskr-options-yaml]")
        .ok()
        .flatten()
        .and_then(|element| form_control_value(&element))
        .unwrap_or_default();
    if yaml.trim().is_empty() {
        show_toast(document, "Configuration YAML is empty");
        return true;
    }
    let body = serde_json::Value::String(yaml).to_string();
    let method = if validate { "POST" } else { "PUT" };
    let endpoint = if validate {
        "/options/yaml/validate"
    } else {
        "/options/yaml"
    };
    let label = if validate {
        "Validating YAML"
    } else {
        "Saving YAML"
    };
    if let Some(status) = panel
        .query_selector("[data-slskr-options-yaml-status]")
        .ok()
        .flatten()
    {
        status.set_text_content(Some(label));
    }
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result =
            fetch_text_with_method(&window, &endpoint_url(endpoint), method, Some(&body)).await;
        if let Some(status) = document
            .query_selector("[data-slskr-options-yaml-status]")
            .ok()
            .flatten()
        {
            status.set_text_content(Some(match &result {
                Ok(response) if response.trim().is_empty() => {
                    if validate {
                        "YAML is valid"
                    } else {
                        "YAML saved"
                    }
                }
                Ok(response) => response,
                Err(_) => "YAML operation failed",
            }));
        }
        if save && result.is_ok() {
            let _ = refresh_route_data(&window).await;
        }
    });
    true
}

#[cfg(target_arch = "wasm32")]
fn mount_transfer_columns(document: &web_sys::Document) -> Result<(), JsValue> {
    const DEFAULTS: &[&str] = &[
        "name", "peer", "size", "progress", "bitrate", "length", "state", "actions",
    ];
    let Some(window) = document.default_view() else {
        return Ok(());
    };
    let storage = window.local_storage().ok().flatten();
    let saved = storage
        .as_ref()
        .and_then(|storage| {
            storage
                .get_item("slskr-transfer-columns-downloads")
                .ok()
                .flatten()
        })
        .map(|value| value.split(',').map(str::to_owned).collect::<BTreeSet<_>>());
    restore_transfer_column_widths(document);
    let toggles = document.query_selector_all("[data-slskr-transfer-column-toggle]")?;
    for index in 0..toggles.length() {
        let Some(node) = toggles.item(index) else {
            continue;
        };
        let input: web_sys::HtmlInputElement = node.dyn_into()?;
        let Some(key) = input.get_attribute("data-slskr-transfer-column-toggle") else {
            continue;
        };
        let visible = saved.as_ref().map_or_else(
            || DEFAULTS.contains(&key.as_str()),
            |saved| saved.contains(&key),
        );
        input.set_checked(visible);
        set_transfer_column_visibility(document, &key, visible);
        let document_for_click = document.clone();
        let input_for_click = input.clone();
        let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
            let Some(key) = input_for_click.get_attribute("data-slskr-transfer-column-toggle")
            else {
                return;
            };
            set_transfer_column_visibility(&document_for_click, &key, input_for_click.checked());
            persist_transfer_columns(&document_for_click);
        }));
        input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    let resize_buttons = document.query_selector_all("[data-slskr-transfer-column-resize]")?;
    for index in 0..resize_buttons.length() {
        let Some(node) = resize_buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        if button.has_attribute("data-slskr-mounted") {
            continue;
        }
        button.set_attribute("data-slskr-mounted", "true")?;
        let document_for_drag = document.clone();
        let button_for_drag = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let Some(key) = button_for_drag.get_attribute("data-slskr-transfer-column-resize")
                else {
                    return;
                };
                let Some(header) = button_for_drag.closest("th").ok().flatten() else {
                    return;
                };
                let Some(window) = document_for_drag.default_view() else {
                    return;
                };
                let start_width = header
                    .dyn_ref::<web_sys::HtmlElement>()
                    .map(|element| element.offset_width().max(80) as i32)
                    .unwrap_or(160);
                let start_x = event.client_x();
                let active = Rc::new(std::cell::Cell::new(true));
                let active_for_move = active.clone();
                let document_for_move = document_for_drag.clone();
                let key_for_move = key.clone();
                let move_callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                    move |move_event: web_sys::MouseEvent| {
                        if !active_for_move.get() {
                            return;
                        }
                        let width = (start_width + move_event.client_x() - start_x).clamp(80, 720);
                        set_transfer_column_width(&document_for_move, &key_for_move, width);
                    },
                ));
                let active_for_up = active.clone();
                let document_for_up = document_for_drag.clone();
                let up_callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                    move |_up_event: web_sys::MouseEvent| {
                        if !active_for_up.replace(false) {
                            return;
                        }
                        persist_transfer_column_widths(&document_for_up);
                    },
                ));
                let _ = window.add_event_listener_with_callback(
                    "mousemove",
                    move_callback.as_ref().unchecked_ref(),
                );
                let _ = window.add_event_listener_with_callback(
                    "mouseup",
                    up_callback.as_ref().unchecked_ref(),
                );
                move_callback.forget();
                up_callback.forget();
            },
        ));
        button.add_event_listener_with_callback("mousedown", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn set_transfer_column_width(document: &web_sys::Document, key: &str, width: i32) {
    let Ok(cells) =
        document.query_selector_all(&format!(r#"[data-slskr-transfer-column="{}"]"#, key))
    else {
        return;
    };
    for index in 0..cells.length() {
        let Some(node) = cells.item(index) else {
            continue;
        };
        if let Ok(element) = node.dyn_into::<web_sys::HtmlElement>() {
            let _ = element.style().set_property("width", &format!("{width}px"));
            let _ = element
                .style()
                .set_property("min-width", &format!("{width}px"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn restore_transfer_column_widths(document: &web_sys::Document) {
    let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    let Some(value) = storage
        .get_item("slskr-transfer-column-widths-downloads")
        .ok()
        .flatten()
    else {
        return;
    };
    for entry in value.split(',') {
        let Some((key, width)) = entry.split_once(':') else {
            continue;
        };
        if let Ok(width) = width.parse::<i32>() {
            set_transfer_column_width(document, key, width.clamp(80, 720));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn persist_transfer_column_widths(document: &web_sys::Document) {
    let Ok(headers) = document.query_selector_all("[data-slskr-transfer-column-resize]") else {
        return;
    };
    let widths = (0..headers.length())
        .filter_map(|index| {
            let node = headers.item(index)?;
            let button = node.dyn_into::<web_sys::Element>().ok()?;
            let key = button.get_attribute("data-slskr-transfer-column-resize")?;
            let header = button.closest("th").ok().flatten()?;
            let width = header
                .dyn_ref::<web_sys::HtmlElement>()?
                .offset_width()
                .max(80);
            Some(format!("{key}:{width}"))
        })
        .collect::<Vec<_>>()
        .join(",");
    if let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item("slskr-transfer-column-widths-downloads", &widths);
    }
}

#[cfg(target_arch = "wasm32")]
fn reset_transfer_column_widths(document: &web_sys::Document) {
    if let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.remove_item("slskr-transfer-column-widths-downloads");
    }
    let Ok(cells) = document.query_selector_all("[data-slskr-transfer-column]") else {
        return;
    };
    for index in 0..cells.length() {
        let Some(node) = cells.item(index) else {
            continue;
        };
        if let Ok(element) = node.dyn_into::<web_sys::HtmlElement>() {
            let _ = element.style().remove_property("width");
            let _ = element.style().remove_property("min-width");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn set_transfer_column_visibility(document: &web_sys::Document, key: &str, visible: bool) {
    if !matches!(
        key,
        "name"
            | "peer"
            | "type"
            | "size"
            | "progress"
            | "bitrate"
            | "samplerate"
            | "bitdepth"
            | "length"
            | "state"
            | "folder"
            | "added"
            | "actions"
    ) {
        return;
    }
    let Ok(cells) =
        document.query_selector_all(&format!(r#"[data-slskr-transfer-column="{}"]"#, key))
    else {
        return;
    };
    for index in 0..cells.length() {
        let Some(node) = cells.item(index) else {
            continue;
        };
        let Ok(cell) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if visible {
            let _ = cell.remove_attribute("hidden");
        } else {
            let _ = cell.set_attribute("hidden", "");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn persist_transfer_columns(document: &web_sys::Document) {
    let Ok(toggles) = document.query_selector_all("[data-slskr-transfer-column-toggle]") else {
        return;
    };
    let mut visible = Vec::new();
    for index in 0..toggles.length() {
        let Some(node) = toggles.item(index) else {
            continue;
        };
        let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
            continue;
        };
        if input.checked() {
            if let Some(key) = input.get_attribute("data-slskr-transfer-column-toggle") {
                visible.push(key);
            }
        }
    }
    if let Some(storage) = document
        .default_view()
        .and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item("slskr-transfer-columns-downloads", &visible.join(","));
    }
}

#[cfg(target_arch = "wasm32")]
fn handle_transfer_request_action(document: &web_sys::Document, button: &web_sys::Element) -> bool {
    if button.has_attribute("data-slskr-transfer-columns-reset") {
        if let Some(storage) = document
            .default_view()
            .and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.remove_item("slskr-transfer-columns-downloads");
        }
        reset_transfer_column_widths(document);
        show_toast(document, "Transfer column widths and visibility reset");
        return true;
    }
    let retry_id = button.get_attribute("data-slskr-transfer-retry");
    let rename = button.has_attribute("data-slskr-transfer-rename");
    let rename_save = button.has_attribute("data-slskr-transfer-rename-save");
    let attempts = button.has_attribute("data-slskr-transfer-attempts");
    let cancel = button.has_attribute("data-slskr-transfer-request-cancel");
    if retry_id.is_none() && !rename && !rename_save && !attempts && !cancel {
        return false;
    }
    let row = button
        .closest("[data-slskr-download-request-row]")
        .ok()
        .flatten();
    if rename {
        if let Some(editor) = row.as_ref().and_then(|row| {
            row.query_selector("[data-slskr-transfer-request-inline]")
                .ok()
                .flatten()
        }) {
            editor.set_class_name("slskr-transfer-request-inline is-open");
        }
        return true;
    }
    let request_id = row
        .as_ref()
        .and_then(|row| row.get_attribute("data-slskr-download-request-id"));
    let (method, path, body, label, load_attempts) = if let Some(id) = retry_id {
        if !safe_route_segment(&id) {
            show_toast(document, "Transfer attempt is invalid");
            return true;
        }
        (
            "POST",
            endpoint_url(&format!("/transfers/{id}/retry")),
            None,
            "Retry transfer",
            false,
        )
    } else {
        let Some(request_id) = request_id.filter(|id| safe_route_segment(id)) else {
            show_toast(document, "Download request is invalid");
            return true;
        };
        if rename_save {
            let name = row
                .as_ref()
                .and_then(|row| {
                    row.query_selector("[data-slskr-transfer-request-inline] input")
                        .ok()
                        .flatten()
                })
                .as_ref()
                .and_then(form_control_value)
                .unwrap_or_default();
            if name.trim().is_empty() || name.len() > 512 {
                show_toast(document, "Request name must be 1 to 512 bytes");
                return true;
            }
            (
                "PATCH",
                endpoint_url(&format!("/downloads/requests/{request_id}/name")),
                Some(format!(
                    r#"{{"name":"{}"}}"#,
                    escape_json_string(name.trim())
                )),
                "Save request name",
                false,
            )
        } else if attempts {
            (
                "GET",
                endpoint_url(&format!("/downloads/requests/{request_id}")),
                None,
                "Load attempts",
                true,
            )
        } else {
            (
                "POST",
                endpoint_url(&format!("/downloads/requests/{request_id}/cancel")),
                None,
                "Cancel request",
                false,
            )
        }
    };
    show_toast(document, &format!("{label} sending"));
    let Some(window) = document.default_view() else {
        return true;
    };
    let document = document.clone();
    let row = row.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = fetch_text_with_method(&window, &path, method, body.as_deref()).await;
        if load_attempts {
            if let Some(target) = row.as_ref().and_then(|row| {
                row.query_selector("[data-slskr-transfer-attempt-results]")
                    .ok()
                    .flatten()
            }) {
                target.set_inner_html(&match &result {
                    Ok(response) => transfer_attempts_response_html(response),
                    Err(error) => format!(
                        "<p>{}</p>",
                        escape_html(
                            &error
                                .as_string()
                                .unwrap_or_else(|| "attempt history failed".to_string())
                        )
                    ),
                });
            }
        }
        if let Some(status) = document.get_element_by_id("slskr-action-status") {
            status.set_inner_html(&match result {
                Ok(response) => format!(
                    "<strong>{}</strong> {}",
                    escape_html(label),
                    escape_html(&compact_preview(&response))
                ),
                Err(error) => format!(
                    "<strong>{}</strong> {}",
                    escape_html(label),
                    escape_html(
                        &error
                            .as_string()
                            .unwrap_or_else(|| "transfer request failed".to_string())
                    )
                ),
            });
        }
    });
    true
}
