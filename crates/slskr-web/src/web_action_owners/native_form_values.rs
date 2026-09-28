use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn native_action_value(
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
        if matches!(body, ActionBody::DownloadFiles) {
            return String::new();
        }
        for selector in native_action_value_selectors(body) {
            if let Some(value) = first_workspace_value(&workspace, selector) {
                return value;
            }
        }
        if matches!(body, ActionBody::CollectionItem) {
            return String::new();
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
        if matches!(body, ActionBody::Username) {
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
    if matches!(
        body,
        ActionBody::ConversationMessage
            | ActionBody::RoomMessage
            | ActionBody::FeedPreview
            | ActionBody::JsonString
            | ActionBody::MusicBrainzTarget
            | ActionBody::SongIdSource
            | ActionBody::ShareGrant
            | ActionBody::ShareGroupMember
            | ActionBody::ContactDiscovery
            | ActionBody::ContactInvite
    ) {
        return native_action_fallback(body);
    }
    document_selected_native_row_title(document).unwrap_or_else(|| native_action_fallback(body))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_action_value_selectors(body: ActionBody) -> &'static [&'static str] {
    match body {
        ActionBody::BrowseDirectory => &[r#"input[aria-label="Folder"]"#],
        ActionBody::CollectionItem => &[
            r#"input[aria-label="Content ID"]"#,
            r#"input[aria-label="Search for item"]"#,
            r#"input[aria-label="Title"]"#,
        ],
        ActionBody::ConversationMessage | ActionBody::RoomMessage => &[
            r#"textarea[aria-label="Message"]"#,
            r#"input[aria-label="Message"]"#,
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
        ActionBody::MusicBrainzTarget => &[r#"input[aria-label="Release ID"]"#],
        ActionBody::LibraryPath => &[r#"input[aria-label="Library Path"]"#],
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
        ActionBody::ContactDiscovery
        | ActionBody::ContactInvite
        | ActionBody::ShareGrant
        | ActionBody::ShareGroupMember
        | ActionBody::Username => &[
            r#"input[aria-label="Username"]"#,
            r#"input[aria-label="Soulseek Username"]"#,
            r#"input[aria-label="Contact username"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Nickname"]"#,
        ],
        ActionBody::SongIdSource => &[r#"input[aria-label="SongID source"]"#],
        ActionBody::EnabledFalse
        | ActionBody::EnabledTrue
        | ActionBody::InviteRequest
        | ActionBody::None => &[],
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_generic_value_selectors() -> &'static [&'static str] {
    &[
        "input:not([type=checkbox]):not([type=radio])",
        "textarea",
        "select",
    ]
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_action_target(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) -> Option<String> {
    if action.path.contains(":collectionId") {
        let workspace = button.closest(".slskr-native-workspace").ok().flatten()?;
        let collection_id = button_native_row_attribute(button, "data-slskr-native-collection-id")
            .or_else(|| {
                selected_native_row_attribute(&workspace, "data-slskr-native-collection-id")
            })?;
        return route_target_segment(&collection_id);
    }
    if action.path.contains(":id")
        && !action.path.contains(":username")
        && !action.path.contains(":roomName")
    {
        return None;
    }
    if !action.path.contains(":username") && !action.path.contains(":roomName") {
        return None;
    }
    let workspace = button.closest(".slskr-native-workspace").ok().flatten()?;
    if action.path.contains(":roomName") {
        let room = button_native_row_attribute(button, "data-slskr-native-room-name")
            .or_else(|| selected_native_row_attribute(&workspace, "data-slskr-native-room-name"))
            .or_else(|| first_workspace_value(&workspace, r#"input[aria-label="Search rooms"]"#))
            .or_else(|| button_native_row_target(button))
            .or_else(|| selected_native_row_target(&workspace))?;
        return route_target_segment(&room);
    }
    if matches!(
        action.body,
        ActionBody::BrowseDirectory | ActionBody::DownloadFiles
    ) {
        for selector in [
            r#"input[aria-label="Username"]"#,
            r#"input[aria-label="Chat username"]"#,
            r#"input[aria-label="Soulseek Username"]"#,
        ] {
            if let Some(value) = first_workspace_value(&workspace, selector)
                .filter(|value| safe_route_segment(value))
            {
                return Some(value);
            }
        }
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
pub(super) fn native_action_target_for_ui(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
    value: &str,
) -> Option<String> {
    if let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() {
        if let Some(target) = native_action_target(document, button, action) {
            return Some(target);
        }
        if let Some(target) = selected_native_row_target(&workspace) {
            return route_target_segment(&target);
        }
    }
    if action.path.contains(":collectionId") {
        return document
            .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
            .ok()
            .flatten()
            .and_then(|row| row.get_attribute("data-slskr-native-collection-id"))
            .filter(|target| safe_route_segment(target))
            .and_then(|target| route_target_segment(&target));
    }
    if action.path.contains(":username") || action.path.contains(":roomName") {
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
            if let Ok(Some(element)) = document.query_selector(selector) {
                if let Some(target) = form_control_value(&element)
                    .map(|target| target.trim().to_owned())
                    .filter(|target| safe_route_segment(target))
                    .and_then(|target| route_target_segment(&target))
                {
                    return Some(target);
                }
            }
        }
        if let Some(target) = route_target_segment(value) {
            return Some(target);
        }
        return document
            .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
            .ok()
            .flatten()
            .and_then(|row| {
                [
                    "data-slskr-native-username",
                    "data-slskr-native-peer",
                    "data-slskr-native-contact",
                    "data-slskr-native-detail",
                    "data-slskr-native-title",
                ]
                .iter()
                .find_map(|attribute| row.get_attribute(attribute))
            })
            .filter(|target| safe_route_segment(target))
            .and_then(|target| route_target_segment(&target));
    }
    None
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_route_action_is_ready(
    document: &web_sys::Document,
    button: &web_sys::Element,
    route_path: &str,
    action: RouteAction,
    target_value: &str,
    body_value: &str,
) -> bool {
    if !native_action_value_is_valid(document, action.label, action, body_value) {
        return false;
    }
    if action.body == ActionBody::ShareGrant {
        let collection_id = button_native_row_attribute(button, "data-slskr-native-collection-id")
            .or_else(|| {
                button
                    .closest(".slskr-native-workspace")
                    .ok()
                    .flatten()
                    .and_then(|workspace| {
                        selected_native_row_attribute(
                            &workspace,
                            "data-slskr-native-collection-id",
                        )
                    })
            })
            .or_else(|| {
                document
                    .query_selector(
                        "[data-slskr-native-select][aria-selected=\"true\"][data-slskr-native-collection-id]",
                    )
                    .ok()
                    .flatten()
                    .and_then(|row| row.get_attribute("data-slskr-native-collection-id"))
            });
        if collection_id
            .as_deref()
            .filter(|value| safe_route_segment(value))
            .is_none()
        {
            native_set_action_status(
                document,
                action.label,
                "Select a live collection before creating a share grant.",
            );
            return false;
        }
    }
    if action.path.contains(":id") || action.path.contains(":itemId") {
        let route_has_id = route_path
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            > 1;
        if !route_has_id && native_action_id(document, button, action).is_none() {
            native_set_action_status(
                document,
                action.label,
                "Select a live row before running this action.",
            );
            return false;
        }
    }
    if action.path.contains(":username")
        || action.path.contains(":roomName")
        || action.path.contains(":collectionId")
    {
        if native_action_target_for_ui(document, button, action, target_value).is_none() {
            native_set_action_status(
                document,
                action.label,
                "Enter or select a live target before running this action.",
            );
            return false;
        }
    }
    true
}

#[cfg(target_arch = "wasm32")]
pub(super) fn route_target_segment(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '?' | '#'))
    {
        return None;
    }
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    Some(encoded)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_action_id(
    document: &web_sys::Document,
    button: &web_sys::Element,
    action: RouteAction,
) -> Option<String> {
    if !action.path.contains(":id") && !action.path.contains(":itemId") {
        return None;
    }
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
        if let Some(workspace) = button.closest(".slskr-native-workspace").ok().flatten() {
            if let Some(value) = selected_native_row_attribute(&workspace, attr)
                .filter(|value| safe_route_segment(value))
            {
                return Some(value);
            }
        }
        if let Some(value) = document
            .query_selector("[data-slskr-native-select][aria-selected=\"true\"]")
            .ok()
            .flatten()
            .and_then(|row| row.get_attribute(attr))
            .filter(|value| safe_route_segment(value))
        {
            return Some(value);
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
pub(super) fn first_workspace_value(
    workspace: &web_sys::Element,
    selector: &str,
) -> Option<String> {
    let nodes = workspace.query_selector_all(selector).ok()?;
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(value) = form_control_value(&element)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
pub(super) fn form_control_value(element: &web_sys::Element) -> Option<String> {
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
