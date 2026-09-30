//! Native Web native workspace panels.

use super::*;

pub(super) fn native_filter_html() -> String {
    r#"<div class="slskr-native-filterbar"><input type="search" data-slskr-native-filter aria-label="Filter visible rows" placeholder="Filter visible rows"><button type="button" data-slskr-native-filter-clear>Clear Filter</button><button type="button" data-slskr-native-select-visible>Select Visible</button><button type="button" data-slskr-native-clear-selection>Clear Selection</button><button type="button" data-slskr-native-reset-state>Reset Table</button><span data-slskr-native-count>0 rows</span></div>"#.to_string()
}

pub(super) fn native_inspector_html() -> String {
    r#"<aside class="slskr-native-inspector" id="slskr-native-inspector" aria-live="polite"><header><div><h3>Selection Inspector</h3><p>Choose a row to inspect details and actions.</p></div><span data-slskr-native-inspector-count>0 selected</span></header><dl><dt>Item</dt><dd data-slskr-native-inspector-title>Nothing selected</dd><dt>Detail</dt><dd data-slskr-native-inspector-detail>Use the table to choose an item.</dd><dt>State</dt><dd data-slskr-native-inspector-meta>Waiting</dd><dt>Action</dt><dd data-slskr-native-inspector-action>Review</dd><dt>Fields</dt><dd data-slskr-native-inspector-fields>Selection fields will appear here.</dd></dl><div class="slskr-native-inspector-actions" data-slskr-native-inspector-actions><button type="button">Review Selection</button><button type="button">Queue Selected</button></div></aside>"#.to_string()
}

pub(super) fn native_selection_preview_html(
    title: &str,
    detail: &str,
    meta: &str,
    action: &str,
) -> String {
    format!(
        r#"<div class="slskr-native-preview-card" aria-live="polite"><span data-slskr-native-preview-count>0 selected</span><strong data-slskr-native-preview-title>{}</strong><p data-slskr-native-preview-detail>{}</p><small data-slskr-native-preview-fields>Selection fields will appear here.</small><em data-slskr-native-preview-meta>{}</em><button type="button" data-slskr-native-preview-action>{}</button></div>"#,
        escape_html(title),
        escape_html(detail),
        escape_html(meta),
        escape_html(action)
    )
}

pub(super) fn native_editor_field_html(label: &str, control: &str) -> String {
    format!(
        r#"<label><span>{}</span>{}</label>"#,
        escape_html(label),
        control
    )
}

pub(super) fn native_editor_text_field_html(label: &str, placeholder: &str) -> String {
    native_editor_field_html(
        label,
        &format!(
            r#"<input aria-label="{}" placeholder="{}">"#,
            escape_html(label),
            escape_html(placeholder)
        ),
    )
}

pub(super) fn native_editor_checkbox_html(label: &str) -> String {
    native_editor_field_html(
        label,
        &format!(
            r#"<span class="slskr-native-editor-check"><input type="checkbox" aria-label="{}"> {}</span>"#,
            escape_html(label),
            escape_html(label)
        ),
    )
}

pub(super) fn native_editor_action_buttons(labels: &[&str]) -> String {
    labels
        .iter()
        .map(|label| format!(r#"<button type="button">{}</button>"#, escape_html(label)))
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn native_editor_state_items(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!(r#"<span>{}</span>"#, escape_html(item)))
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn native_editor_modal_html(kind: RouteKind) -> String {
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

pub(super) fn native_browse_workspace_html(
    route_table: &str,
    responses: Option<&[EndpointBody]>,
) -> String {
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

pub(super) fn native_collection_items_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn private_message_auto_response_panel_html(
    responses: Option<&[EndpointBody]>,
) -> String {
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

pub(super) fn native_message_room_rows_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn native_message_thread_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn native_message_transcript_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn native_message_pods_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn native_messaging_workspace_html(
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

pub(super) fn native_search_filter_panel_html() -> String {
    r#"<section class="slskr-native-filter-modal" data-slskr-search-filter-modal><header><div><h4>Search Filters</h4><p>Format, bitrate, size, duration, queue, and duplicate controls stay visible beside results.</p></div><button type="button">Apply Filters</button></header><div class="slskr-native-filter-grid"><label><span>Include words</span><input aria-label="Include words" placeholder="remix instrumental"></label><label><span>Exclude words</span><input aria-label="Exclude words" placeholder="live demo"></label><label><span>Min bitrate</span><input aria-label="Min bitrate" placeholder="320"></label><label><span>Format</span><input aria-label="Format" placeholder="flac mp3 wav"></label><label><span>Min size</span><input aria-label="Min size" placeholder="1 MB"></label><label><span>Max size</span><input aria-label="Max size" placeholder="100 MB"></label><label><span>Min duration</span><input aria-label="Min duration" placeholder="3 min"></label><label><span>Max queue</span><input aria-label="Max queue" placeholder="8"></label></div><div class="slskr-native-filter-toggles"><label><input type="checkbox" aria-label="Fold duplicate results" checked> Fold duplicate results</label><label><input type="checkbox" aria-label="Prefer free upload slots" checked> Prefer free slots</label><label><input type="checkbox" aria-label="Hide locked files"> Hide locked files</label><select aria-label="Search ranking profile"><option>Smart ranking</option><option>Exact match first</option><option>Fastest peer first</option><option>Lossless first</option></select></div></section>"#.to_string()
}

pub(super) fn native_download_policy_panel_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn native_system_configuration_panel_html(responses: Option<&[EndpointBody]>) -> String {
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
pub(super) fn native_final_parity_panel_html(kind: RouteKind) -> String {
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

pub(super) fn wishlist_ignored_results_panel_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn wishlist_policy_history_panel_html(responses: Option<&[EndpointBody]>) -> String {
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

pub(super) fn download_request_workspace_html(responses: Option<&[EndpointBody]>) -> String {
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
pub(super) fn wishlist_history_response_html(response: &str) -> String {
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

pub(super) fn route_native_workspace_html(
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
