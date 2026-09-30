use super::*;

pub(super) fn collection_items_id(path: &str) -> Option<&str> {
    let path = path.strip_prefix("/api/collections/")?;
    let id = path.strip_suffix("/items")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

/// Accept both slskR's historical flat item action path and the current
/// upstream contract's collection-scoped item action path. The optional
/// collection id is checked by the dispatcher after resolving the item so a
/// valid item cannot be mutated through the wrong collection URL.
pub(super) fn collection_item_action_ids(path: &str) -> Option<(&str, Option<&str>)> {
    let path = path.strip_prefix("/api/collections/")?;
    if let Some(item_id) = path.strip_prefix("items/") {
        return (!item_id.is_empty() && !item_id.contains('/')).then_some((item_id, None));
    }
    let (collection_id, item_id) = path.split_once("/items/")?;
    if collection_id.is_empty()
        || item_id.is_empty()
        || collection_id.contains('/')
        || item_id.contains('/')
    {
        return None;
    }
    Some((item_id, Some(collection_id)))
}

pub(super) fn wishlist_search_item_id(path: &str) -> Option<&str> {
    let path = path.strip_prefix("/api/wishlist/")?;
    let id = path.strip_suffix("/search")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

pub(super) fn wishlist_item_action_id<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    let path = path.strip_prefix("/api/wishlist/")?;
    let id = path.strip_suffix(suffix)?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

pub(super) fn wishlist_ignored_results_item_id(path: &str) -> Option<&str> {
    let path = path.strip_prefix("/api/wishlist/")?;
    let id = path.strip_suffix("/ignored-results")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

pub(super) fn wishlist_ignored_result_ids(path: &str) -> Option<(&str, &str)> {
    let path = path.strip_prefix("/api/wishlist/")?;
    let (item_id, rule_id) = path.split_once("/ignored-results/")?;
    (!item_id.is_empty() && !rule_id.is_empty() && !item_id.contains('/') && !rule_id.contains('/'))
        .then_some((item_id, rule_id))
}

/// The frozen native profile WishlistController binds both route identifiers as
/// `Guid`.  Keep malformed v0 identifiers in the controller's 400 contract
/// instead of allowing the compatibility route helpers to fall through to a
/// generic 404.  `wish-N` is the stable legacy slskR storage identifier; it is
/// accepted here so native profile requests can reverse-resolve old databases
/// before the route handler projects the identifier back to a GUID.
pub(super) fn is_legacy_wishlist_item_id(id: &str) -> bool {
    id.strip_prefix("wish-").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

pub(super) fn versioned_wishlist_invalid_id_response(
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    let _ = method;
    let rest = path.strip_prefix("/api/v0/wishlist/")?;
    let segments = rest
        .split('/')
        .map(decoded_path_segment)
        .map(|segment| segment.trim().to_owned())
        .collect::<Vec<_>>();

    let dynamic_ids = match segments.as_slice() {
        [id] if !matches!(id.as_str(), "bulk-filter" | "mark-all-viewed" | "import") => {
            vec![id]
        }
        [id, action]
            if matches!(
                action.as_str(),
                "search" | "searches" | "mark-viewed" | "ignored-results"
            ) =>
        {
            vec![id]
        }
        [id, action, ignored_id] if action == "ignored-results" => vec![id, ignored_id],
        _ => return None,
    };

    dynamic_ids
        .iter()
        .find(|id| uuid::Uuid::parse_str(id).is_err() && !is_legacy_wishlist_item_id(id))
        .map(|_| routing::bad_request_response("The request is invalid"))
}

pub(super) async fn controller_native_wishlist_read_failure_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if method != "GET" || !path.starts_with("/api/v0/wishlist") {
        return None;
    }
    let db = state.db.as_ref()?;
    let failed = if path == "/api/v0/wishlist" {
        db.list_wishlist_items(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .is_err()
    } else if path.ends_with("/ignored-results") {
        db.list_wishlist_ignored_results("wishlist-read-probe")
            .await
            .is_err()
    } else if path.ends_with("/searches") || path.starts_with("/api/v0/wishlist/") {
        db.list_wishlist_items(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .is_err()
    } else {
        false
    };
    failed.then(|| routing::internal_server_error_response("Failed to read wishlist"))
}

pub(super) fn virtual_soulfind_legacy_dynamic_id(path: &str) -> Option<&'static str> {
    for (prefix, kind) in [
        ("/api/virtualsoulfind/canonical/", "canonical"),
        ("/api/virtualsoulfind/shadow-index/", "shadow-index"),
    ] {
        let Some(id) = path.strip_prefix(prefix) else {
            continue;
        };
        if !id.is_empty() && !id.contains('/') {
            return Some(kind);
        }
    }
    None
}

/// ASP.NET binds an encoded blank route value and lets the VirtualSoulfind
/// controllers return their explicit MBID validation response.  The generic
/// route helpers otherwise treat the same value as an unmatched route.
pub(super) fn virtual_soulfind_legacy_blank_id_response(path: &str) -> Option<HttpResponse> {
    for prefix in [
        "/api/virtualsoulfind/canonical/",
        "/api/virtualsoulfind/shadow-index/",
    ] {
        let Some(raw_id) = path.strip_prefix(prefix) else {
            continue;
        };
        if !raw_id.is_empty()
            && !raw_id.contains('/')
            && decoded_path_segment(raw_id).trim().is_empty()
        {
            return Some(routing::bad_request_response("MBID is required"));
        }
    }
    None
}

/// The local shadow-index projection stores peer membership separately from
/// HashDb metadata.  Reconstruct the same compact FLAC variant hints used by
/// the mesh shadow-query service for the controller DTOs.
pub(super) fn virtual_soulfind_legacy_variants(
    discovery: &content_discovery::ContentDiscoveryStore,
    mbid: &str,
) -> Vec<serde_json::Value> {
    if !discovery
        .shadow_records()
        .iter()
        .any(|record| record.recording_id.eq_ignore_ascii_case(mbid))
    {
        return Vec::new();
    }

    discovery
        .hash_entries()
        .iter()
        .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(mbid))
        .take(10)
        .filter_map(|entry| {
            let hash = [&entry.file_sha256, &entry.full_file_hash, &entry.byte_hash]
                .into_iter()
                .find(|hash| !hash.is_empty())?;
            hex::decode(hash).ok()?;
            Some(serde_json::json!({
                "codec": "FLAC",
                "bitrate": 0,
                "fileSize": entry.size,
                "qualityScore": 1.0,
            }))
        })
        .collect()
}

pub(super) async fn controller_native_virtual_soulfind_read_failure_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native || method != "GET" {
        return None;
    }
    let kind = virtual_soulfind_legacy_dynamic_id(path)?;
    let db = state.db.as_ref()?;
    if db.list_hash_db_entries().await.is_ok() {
        return None;
    }
    Some(match kind {
        "canonical" => {
            routing::internal_server_error_response("Failed to select canonical variant")
        }
        _ => routing::internal_server_error_response("Failed to query shadow index"),
    })
}

pub(super) fn share_contents_id(path: &str) -> Option<String> {
    let path = path.strip_prefix("/api/shares/")?;
    let id = path.strip_suffix("/contents")?;
    if id.is_empty() || id.contains('/') {
        return None;
    }
    Some(decoded_path_segment(id))
}

pub(super) fn share_resource_id(path: &str) -> Option<String> {
    let id = path_segment_after(path, "/api/shares/")?;
    Some(decoded_path_segment(id))
}

pub(super) fn user_route_username(path: &str, suffix: &str) -> Option<String> {
    let path = path.strip_prefix("/api/users/")?;
    let username = path.strip_suffix(suffix)?;
    if username.is_empty() || username.contains('/') {
        return None;
    }
    Some(decoded_path_segment(username))
}

pub(super) async fn controller_user_read_failure_response(
    state: &AppState,
    route_path: &str,
    username: &str,
    browse: bool,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Legacy
        || !route_path.starts_with("/api/v0/")
        || state.session.read().await.state == "connected"
    {
        return None;
    }

    let tracked = if browse {
        state
            .browse
            .read()
            .await
            .records
            .iter()
            .any(|record| record.username == username)
    } else {
        state
            .users
            .read()
            .await
            .records
            .iter()
            .any(|record| record.username == username)
    };
    // The legacy browse controller asks the session-backed user service for
    // every browse request.  When that service is unavailable, even an
    // untracked user reaches the controller's 500 path; the user/status
    // endpoints retain their 404 behavior for users that were never tracked.
    if browse || tracked {
        Some(routing::internal_server_error_response(
            "user service unavailable",
        ))
    } else {
        None
    }
}

pub(super) fn joined_room_subresource(path: &str, suffix: &str) -> Option<String> {
    let path = path.strip_prefix("/api/rooms/joined/")?;
    let room = path.strip_suffix(suffix)?;
    if room.is_empty() || room.contains('/') {
        return None;
    }
    let room = decoded_path_segment(room).trim().to_owned();
    (!room.is_empty()).then_some(room)
}

pub(super) fn rooms_controller_value_bad_request_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "400 Bad Request",
        content_type: "application/json",
        body: serde_json::to_string(message).unwrap_or_else(|_| "\"Bad Request\"".to_owned()),
    }
}

/// The frozen versioned RoomsController binds `roomName` before entering its
/// action and returns BadRequest(string) for an encoded blank value.  The
/// compatibility dispatcher otherwise treats the blank segment as an
/// unmatched route and returns 404.
pub(super) fn versioned_rooms_blank_segment_response(
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if !matches!(method, "GET" | "POST" | "DELETE") {
        return None;
    }
    let rest = path.strip_prefix("/api/rooms/joined/")?;
    let (raw_room, expected_suffix) = if let Some(room) = rest.strip_suffix("/messages") {
        (room, "/messages")
    } else if let Some(room) = rest.strip_suffix("/ticker") {
        (room, "/ticker")
    } else if let Some(room) = rest.strip_suffix("/members") {
        (room, "/members")
    } else if let Some(room) = rest.strip_suffix("/users") {
        (room, "/users")
    } else {
        (rest, "")
    };
    if raw_room.is_empty() || raw_room.contains('/') {
        return None;
    }
    let expected_method = match expected_suffix {
        "" => matches!(method, "GET" | "DELETE"),
        "/messages" => matches!(method, "GET" | "POST"),
        "/ticker" | "/members" => method == "POST",
        "/users" => method == "GET",
        _ => false,
    };
    (expected_method && decoded_path_segment(raw_room).trim().is_empty())
        .then(|| rooms_controller_value_bad_request_response("roomName is required"))
}

pub(super) fn unversioned_rooms_compatibility_blank_id_response(
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if method != "DELETE" {
        return None;
    }
    let room = path.strip_prefix("/api/rooms/")?;
    if room.is_empty() || room.contains('/') {
        return None;
    }
    decoded_path_segment(room)
        .trim()
        .is_empty()
        .then(|| routing::bad_request_response("Room is required"))
}

/// The frozen BridgeController binds an encoded blank transfer id and returns
/// its explicit validation object.  Generic dynamic-route matching rejects
/// the decoded blank segment before the controller projection can run.
pub(super) fn bridge_transfer_blank_segment_response(
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if method != "GET" {
        return None;
    }
    for prefix in ["/api/bridge/transfer/", "/api/v0/bridge/transfer/"] {
        let Some(rest) = path.strip_prefix(prefix) else {
            continue;
        };
        let Some(raw_id) = rest.strip_suffix("/progress") else {
            continue;
        };
        if raw_id.is_empty() || raw_id.contains('/') {
            continue;
        }
        if decoded_path_segment(raw_id).trim().is_empty() {
            return Some(routing::bad_request_response("TransferId is required"));
        }
    }
    None
}

pub(super) fn library_health_issue_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/api/library/health/issues/")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

// Wishlist models are defined in wishlist_store.

// Contact Models

// ShareGroup Models

pub(super) fn share_group_resource_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/api/sharegroups/")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

pub(super) fn share_group_members_id(path: &str) -> Option<&str> {
    let path = path.strip_prefix("/api/sharegroups/")?;
    let id = path.strip_suffix("/members")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

pub(super) fn share_group_member_path(path: &str) -> Option<(&str, String)> {
    let path = path.strip_prefix("/api/sharegroups/")?;
    let (id, username) = path.split_once("/members/")?;
    if id.is_empty() || id.contains('/') || username.is_empty() || username.contains('/') {
        return None;
    }
    Some((id, decoded_path_segment(username)))
}

// User Notes Models

// Interest Models
