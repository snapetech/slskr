async fn route_dispatch_group_6_admin_routes(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let method = context.method;
    let normalized_path = context.normalized_path;
    let authorization = context.authorization;
    let body = context.body;
    let state = context.state;
    let route = context.route;
    let headers = context.headers;
    let extended_mutation = context.extended_mutation;
    let request_is_versioned_v0 = context.request_is_versioned_v0;
    match (method, normalized_path) {
        ("PUT", path) if path.ends_with("/adversarial") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.contains("/disclosure/") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.ends_with("/reputation") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("GET", "/api/config/shares") => {
            let shares = state.shares.read().await;
            let share_roots: Vec<String> = shares
                .roots
                .iter()
                .map(|root| {
                    format!(
                        "{{\"label\":\"{}\",\"files\":{},\"bytes\":{}}}",
                        json_escape(&root.label),
                        root.files,
                        root.bytes
                    )
                })
                .collect();
            let json = format!(
                "{{\"roots\":[{}],\"count\":{}}}",
                share_roots.join(","),
                shares.roots.len()
            );
            drop(shares);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/shares") => {
            let path = extract_json_string_field(body, "path")
                .or_else(|| extract_json_string_field(body, "localPath"))
                .unwrap_or_default()
                .trim()
                .to_owned();
            if path.is_empty() {
                return Ok(routing::bad_request_response("path is required"));
            }
            let alias = extract_json_string_field(body, "alias")
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|alias| alias.trim().to_owned())
                .filter(|alias| !alias.is_empty());
            let (directory, snapshot) =
                match add_runtime_share(state, &path, alias.as_deref()).await {
                    Ok(result) => result,
                    Err(error)
                        if error == "share path is already configured"
                            || error.starts_with("share alias '") =>
                    {
                        return Ok(HttpResponse {
                            status: "409 Conflict",
                            content_type: "application/json",
                            body: serde_json::json!({"error": error}).to_string(),
                        });
                    }
                    Err(error)
                        if error.starts_with("Share ")
                            || matches!(
                                error.as_str(),
                                "share path contains an invalid NUL character"
                                    | "share alias contains an invalid NUL character"
                            ) =>
                    {
                        return Ok(routing::bad_request_response(&error));
                    }
                    Err(error) => return Ok(share_rebuild_error_response(&error)),
                };
            record_event(
                state,
                "share.configuration.updated",
                "shares",
                Some(format!("added={}", directory.alias)),
            )
            .await;
            Ok(routing::created_response(
                serde_json::json!({
                    "path": path,
                    "alias": directory.alias,
                    "added": true,
                    "files": snapshot.entries.len(),
                    "bytes": snapshot.entries.iter().map(|entry| entry.size).sum::<u64>(),
                    "scan_errors": snapshot.scan_errors,
                    "scanned": true,
                    "indexPersisted": state.db.is_some(),
                    "configurationPersisted": false,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/config/plugins") => {
            let integrations = state.integration_settings.read().await;
            let plugins = serde_json::json!([
                {
                    "id": "spotify",
                    "name": "Spotify",
                    "enabled": integrations.spotify.enabled,
                    "configured": integrations.spotify.configured(),
                },
                {
                    "id": "lidarr",
                    "name": "Lidarr",
                    "enabled": integrations.lidarr.enabled,
                    "configured": integrations.lidarr.configured(),
                },
                {
                    "id": "external-visualizer",
                    "name": "External Visualizer",
                    "enabled": state.config.integrations.external_visualizer.launch_enabled,
                    "configured": state.config.integrations.external_visualizer.configured(),
                },
                {
                    "id": "bridge",
                    "name": "Bridge",
                    "enabled": state.config.integrations.bridge.enabled,
                    "configured": state.config.integrations.bridge.enabled,
                }
            ]);
            let json = serde_json::json!({
                "plugins": plugins,
                "count": plugins.as_array().map_or(0, Vec::len),
            })
            .to_string();
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/filters") => {
            let filter_type = extract_json_string_field(body, "type").unwrap_or_default();
            let pattern = extract_json_string_field(body, "pattern").unwrap_or_default();
            let json = format!(
                "{{\"type\":\"{}\",\"pattern\":\"{}\",\"created_at\":{}}}",
                json_escape(&filter_type),
                json_escape(&pattern),
                unix_timestamp()
            );
            Ok(routing::created_response(json))
        }

        // ADMIN/SYSTEM ENDPOINTS
        ("GET", "/api/admin/stats") => {
            let transfers = state.transfers.read().await;
            let total_bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let active_transfers = transfers
                .entries
                .iter()
                .filter(|entry| is_queued_or_active_transfer_status(&entry.status))
                .count();
            let searches = state.searches.read().await;
            let users = state.users.read().await;
            let rooms = state.rooms.read().await;
            let shares = state.shares.read().await;
            let json = serde_json::json!({
                "total_transfers": transfers.entries.len(),
                "active_transfers": active_transfers,
                "total_bytes": total_bytes,
                "searches": searches.records.len(),
                "users": users.records.len(),
                "rooms": rooms.records.len(),
                "shared_files": shares.entries.len(),
            })
            .to_string();
            drop(shares);
            drop(rooms);
            drop(users);
            drop(searches);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        // RECOMMENDATIONS & ANALYTICS ENDPOINTS
        ("GET", "/api/soulseek/recommendations") => {
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_recommendations_json()
            } else {
                interests.recommendations_json("recommendations")
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/soulseek/recommendations/global") => {
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_recommendations_json()
            } else {
                interests.recommendations_json("global_recommendations")
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if path.starts_with("/api/soulseek/items/") && path.ends_with("/recommendations") =>
        {
            let Some(item_id) =
                path_segment_between(path, "/api/soulseek/items/", "/recommendations")
            else {
                return Ok(routing::not_found_response());
            };
            let item_id = decoded_path_segment(item_id).trim().to_owned();
            if item_id.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_item_recommendations_json(&item_id)
            } else {
                interests.item_recommendations_json(&item_id)
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if path.starts_with("/api/soulseek/items/") && path.ends_with("/similar-users") =>
        {
            let Some(item_id) =
                path_segment_between(path, "/api/soulseek/items/", "/similar-users")
            else {
                return Ok(routing::not_found_response());
            };
            let item_id = decoded_path_segment(item_id).trim().to_owned();
            if item_id.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            let users = state.users.read().await;
            if route.path.starts_with("/api/v0/") {
                let usernames = users
                    .records
                    .iter()
                    .filter(|user| user.watched || user.status.as_deref() == Some("online"))
                    .map(|user| user.username.clone())
                    .collect::<Vec<_>>();
                let json = serde_json::json!({
                    "item": item_id,
                    "usernames": usernames,
                })
                .to_string();
                drop(users);
                return Ok(routing::ok_response(json));
            }
            let similar_users = users
                .records
                .iter()
                .filter(|user| user.watched || user.status.as_deref() == Some("online"))
                .map(|user| {
                    serde_json::json!({
                        "username": user.username,
                        "status": user.status,
                        "watched": user.watched,
                        "score": 1.0,
                    })
                })
                .collect::<Vec<_>>();
            let count = similar_users.len();
            drop(users);
            let json = serde_json::json!({
                "item_id": item_id,
                "similar_users": similar_users,
                "count": count,
            })
            .to_string();
            Ok(routing::ok_response(json))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
