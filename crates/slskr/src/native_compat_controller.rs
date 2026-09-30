use super::*;

pub(super) fn native_compat_route(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        (
            "GET",
            "/api/slskdn"
                | "/api/slskdn/library/health"
                | "/api/hashdb"
                | "/api/hashdb/stats"
                | "/api/mesh"
                | "/api/mesh/health"
                | "/api/mesh/transport"
                | "/api/virtualsoulfind"
                | "/api/virtualsoulfind/canonical/status"
                | "/api/audio"
                | "/api/mediacore"
                | "/api/playback"
                | "/api/security"
                | "/api/security/status"
                | "/api/pods"
                | "/api/solid"
                | "/api/solid/status"
                | "/api/federation"
                | "/api/federation/diagnostics"
        ) | (
            "POST",
            "/api/slskdn/warm-cache" | "/api/audio/variants/dedupe" | "/api/mediacore/retrieve"
        )
    )
}

pub(super) async fn native_compat_response(
    method: &str,
    path: &str,
    state: &AppState,
) -> HttpResponse {
    let session = state.session.read().await;
    let shares = state.shares.read().await;
    let searches = state.searches.read().await;
    let transfers = state.transfers.read().await;
    let listeners = state.listeners.read().await;
    let users = state.users.read().await;
    let rooms = state.rooms.read().await;
    let events = state.events.read().await;
    let library = state.library.read().await;
    let now_playing = state.now_playing.read().await;
    let security = state.security.read().await;
    let mesh = state.mesh.read().await;
    let collections = state.collections.read().await;
    let wishlist = state.wishlist.read().await;

    let family = path
        .trim_start_matches("/api/")
        .split('/')
        .next()
        .unwrap_or("native");
    let status = if method == "POST" || method == "PUT" || method == "DELETE" {
        "accepted"
    } else if family == "security" && security.active_bans() > 0 {
        "guarded"
    } else if family == "playback" && !now_playing.records.is_empty() {
        "playing"
    } else if family == "mesh" && (!users.records.is_empty() || !mesh.capability_records.is_empty())
    {
        "ready"
    } else if family == "solid" && (!collections.records.is_empty()) {
        "local"
    } else if matches!(family, "fairness" | "ranking")
        && (!transfers.entries.is_empty()
            || !searches.records.is_empty()
            || !users.records.is_empty())
    {
        "ready"
    } else if family == "portforwarding"
        && (listeners.regular_bind.is_some() || listeners.obfuscated_bind.is_some())
    {
        "configured"
    } else if family == "portforwarding" {
        "disabled"
    } else if family == "federation"
        && (!mesh.capability_records.is_empty() || !users.records.is_empty())
    {
        "ready"
    } else if session.state == "connected"
        || !shares.entries.is_empty()
        || !searches.records.is_empty()
    {
        "local"
    } else {
        "empty"
    };
    let jobs = searches
        .records
        .iter()
        .map(|record| {
            serde_json::json!({
                "id": record.id,
                "token": record.token,
                "kind": "search",
                "status": record.status,
                "query": record.query,
            })
        })
        .chain(transfers.entries.iter().map(|entry| {
            serde_json::json!({
                "id": format!("transfer-{}", entry.id),
                "kind": "transfer",
                "status": entry.status,
                "filename": entry.filename,
                "username": entry.peer_username,
            })
        }))
        .collect::<Vec<_>>();
    let items = match family {
        "library" => library
            .records
            .iter()
            .map(|record| {
                serde_json::from_str::<serde_json::Value>(&record.json())
                    .unwrap_or_else(|_| serde_json::json!({ "id": record.id }))
            })
            .collect::<Vec<_>>(),
        "playback" => now_playing
            .records
            .iter()
            .map(|record| {
                serde_json::from_str::<serde_json::Value>(&record.json())
                    .unwrap_or_else(|_| serde_json::json!({}))
            })
            .collect::<Vec<_>>(),
        "security" => security
            .json_value()
            .get("bans")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default(),
        "fairness" => users
            .records
            .iter()
            .map(|user| {
                let queued = transfers
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.peer_username.as_deref() == Some(user.username.as_str())
                            && is_queued_or_active_transfer_status(&entry.status)
                    })
                    .count();
                serde_json::json!({
                    "username": user.username,
                    "watched": user.watched,
                    "status": user.status,
                    "averageSpeed": user.average_speed.unwrap_or(0),
                    "queuedTransfers": queued,
                    "score": user.average_speed.unwrap_or(0) as u64 + user.file_count.unwrap_or(0) as u64,
                })
            })
            .collect::<Vec<_>>(),
        "ranking" => {
            let mut rows = transfers
                .entries
                .iter()
                .map(|entry| {
                    serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "score": entry.bytes_transferred,
                        "filename": entry.filename,
                        "username": entry.peer_username,
                        "status": entry.status,
                    })
                })
                .chain(searches.records.iter().map(|record| {
                    serde_json::json!({
                        "id": record.id,
                        "kind": "search",
                        "score": record.results.len(),
                        "query": record.query,
                        "status": record.status,
                    })
                }))
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| {
                b.get("score")
                    .and_then(serde_json::Value::as_u64)
                    .cmp(&a.get("score").and_then(serde_json::Value::as_u64))
            });
            rows
        }
        "portforwarding" => vec![serde_json::json!({
            "regularBind": listeners.regular_bind,
            "regularLocalAddr": listeners.regular_local_addr,
            "obfuscatedBind": listeners.obfuscated_bind,
            "obfuscatedLocalAddr": listeners.obfuscated_local_addr,
            "regularAccepts": listeners.regular_accepts,
            "obfuscatedAccepts": listeners.obfuscated_accepts,
            "errors": listeners.errors,
            "lastError": public_listener_error(listeners.last_error.as_deref()),
            "updated_at": listeners.updated_at,
        })],
        "solid" => collections
            .records
            .iter()
            .map(|collection| {
                serde_json::json!({
                    "id": collection.id,
                    "name": collection.name,
                    "itemCount": collection.items.len(),
                    "updated_at": collection.updated_at,
                    "storage": "local-collection",
                })
            })
            .collect::<Vec<_>>(),
        "federation" => mesh
            .capability_records
            .iter()
            .map(|record| {
                serde_json::json!({
                    "username": record.username,
                    "issuedAt": record.issued_at_unix,
                    "expiresAt": record.expires_at_unix,
                    "features": record.features.clone(),
                    "endpoints": record.endpoints.clone(),
                    "source": "peer-capability",
                })
            })
            .chain(users.records.iter().filter(|user| user.watched).map(|user| {
                serde_json::json!({
                    "username": user.username,
                    "status": user.status,
                    "source": "watched-user",
                })
            }))
            .collect::<Vec<_>>(),
        "pods" | "podcore" | "listening-party" => rooms
            .records
            .iter()
            .map(|room| {
                serde_json::json!({
                    "id": room.name,
                    "name": room.name,
                    "joined": room.joined,
                    "messageCount": room.messages.len(),
                    "userCount": room.user_count,
                })
            })
            .collect::<Vec<_>>(),
        "hashdb" | "streams" | "audio" | "mediacore" => shares
            .entries
            .iter()
            .take(DEFAULT_LIST_LIMIT)
            .map(|entry| {
                serde_json::json!({
                    "filename": entry.filename,
                    "size": entry.size,
                    "extension": entry.extension,
                })
            })
            .collect::<Vec<_>>(),
        "discovery" | "signals" | "traces" => events
            .records
            .iter()
            .rev()
            .take(50)
            .map(|event| {
                serde_json::json!({
                    "id": event.id,
                    "kind": event.kind,
                    "resource": event.resource,
                    "created_at": event.created_at,
                })
            })
            .collect::<Vec<_>>(),
        _ => users
            .records
            .iter()
            .map(|user| {
                serde_json::json!({
                    "username": user.username,
                    "status": user.status,
                    "watched": user.watched,
                })
            })
            .collect::<Vec<_>>(),
    };
    let item_count = items.len();
    let job_count = jobs.len();
    let body = serde_json::json!({
        "path": path,
        "method": method,
        "family": family,
        "status": status,
        "enabled": true,
        "supported": true,
        "connected": session.state == "connected",
        "items": items,
        "itemCount": item_count,
        "jobs": jobs,
        "jobCount": job_count,
        "counts": {
            "connected": session.state == "connected",
            "shares": shares.entries.len(),
            "searches": searches.records.len(),
            "transfers": transfers.entries.len(),
            "activeTransfers": transfers.entries.iter().filter(|entry| is_queued_or_active_transfer_status(&entry.status)).count(),
            "users": users.records.len(),
            "watchedUsers": users.records.iter().filter(|user| user.watched).count(),
            "rooms": rooms.records.len(),
            "joinedRooms": rooms.records.iter().filter(|room| room.joined).count(),
            "events": events.records.len(),
            "libraryItems": library.records.len(),
            "wishlistItems": wishlist.records.iter().map(|record| record.items.len()).sum::<usize>(),
            "securityBans": security.active_bans(),
            "meshCapabilities": mesh.capability_records.len(),
            "collections": collections.records.len(),
            "listenerAccepts": listeners.regular_accepts + listeners.obfuscated_accepts,
            "listenerErrors": listeners.errors,
        },
    })
    .to_string();

    drop(wishlist);
    drop(collections);
    drop(mesh);
    drop(security);
    drop(now_playing);
    drop(library);
    drop(events);
    drop(rooms);
    drop(users);
    drop(listeners);
    drop(transfers);
    drop(searches);
    drop(shares);
    drop(session);

    if method == "POST" {
        routing::accepted_response(body)
    } else {
        routing::ok_response(body)
    }
}

pub(super) fn native_model_validation_response() -> HttpResponse {
    HttpResponse {
        status: "400 Bad Request",
        content_type: "application/problem+json",
        body: serde_json::json!({
            "title": "One or more validation errors occurred.",
            "status": 400,
            "detail": "The request is invalid.",
            "errors": {},
        })
        .to_string(),
    }
}
