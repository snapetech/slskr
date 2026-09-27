async fn route_dispatch_group_6_security_shares(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        state_arc,
        extended_mutation,
        request_is_versioned_v0,
    } = context.clone();
    match (method, normalized_path) {
        ("GET", path) if security_ban_route_tail(path).is_some_and(|tail| tail.is_empty()) => {
            let security = state.security.read().await;
            let json = security.json_value().to_string();
            drop(security);
            Ok(routing::ok_response(json))
        }

        ("POST", path)
            if security_ban_route_tail(path).as_deref() == Some(["username"].as_slice()) =>
        {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let Some(username) = normalize_security_ban_value("username", &username) else {
                return Ok(routing::bad_request_response("username is required"));
            };
            let (reason, duration_seconds, is_permanent) = security_ban_options(body);
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, record, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let Some(record) = security.ban_with_options(
                    "username",
                    username.clone(),
                    reason,
                    duration_seconds,
                    is_permanent,
                ) else {
                    return Ok(routing::service_unavailable_response(
                        "security ban capacity is full",
                    ));
                };
                let mutated_bans = security.bans.clone();
                let mutated_updated_at = security.updated_at;
                (
                    previous_bans,
                    previous_updated_at,
                    record,
                    mutated_bans,
                    mutated_updated_at,
                )
            };
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    rollback_security_ban_if_unchanged(
                        state,
                        previous_bans,
                        previous_updated_at,
                        mutated_bans,
                        mutated_updated_at,
                    )
                    .await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = state.security.read().await.active_bans();
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(String::new()));
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "username": username,
                    "banned": true,
                    "persisted": persisted,
                    "kind": record.kind,
                    "created_at": record.created_at,
                    "activeBans": active_bans,
                })
                .to_string(),
            ))
        }

        ("DELETE", path)
            if security_ban_route_tail(path)
                .is_some_and(|tail| tail.len() == 2 && tail[0] == "username") =>
        {
            let username = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let Some(username) = normalize_security_ban_value("username", &username) else {
                return Ok(routing::bad_request_response("username is required"));
            };
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, removed, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let removed = security.unban("username", &username);
                let mutated_bans = security.bans.clone();
                let mutated_updated_at = security.updated_at;
                (
                    previous_bans,
                    previous_updated_at,
                    removed,
                    mutated_bans,
                    mutated_updated_at,
                )
            };
            let persisted = if removed {
                match persist_security_unban(state, "username", &username).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        rollback_security_ban_if_unchanged(
                            state,
                            previous_bans,
                            previous_updated_at,
                            mutated_bans,
                            mutated_updated_at,
                        )
                        .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = state.security.read().await.active_bans();
            if route.path.starts_with("/api/v0/") {
                return Ok(if removed {
                    routing::ok_response(String::new())
                } else {
                    routing::not_found_response()
                });
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "username": username,
                    "banned": false,
                    "removed": removed,
                    "persisted": persisted,
                    "activeBans": active_bans,
                })
                .to_string(),
            ))
        }

        ("POST", path) if security_ban_route_tail(path).as_deref() == Some(["ip"].as_slice()) => {
            let ip = extract_json_string_field(body, "ipAddress")
                .or_else(|| extract_json_string_field(body, "ip"))
                .unwrap_or_default();
            let Some(ip) = normalize_security_ban_value("ip", &ip) else {
                return Ok(routing::bad_request_response("valid ip is required"));
            };
            let (reason, duration_seconds, is_permanent) = security_ban_options(body);
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, record, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let Some(record) = security.ban_with_options(
                    "ip",
                    ip.clone(),
                    reason,
                    duration_seconds,
                    is_permanent,
                ) else {
                    return Ok(routing::service_unavailable_response(
                        "security ban capacity is full",
                    ));
                };
                let mutated_bans = security.bans.clone();
                let mutated_updated_at = security.updated_at;
                (
                    previous_bans,
                    previous_updated_at,
                    record,
                    mutated_bans,
                    mutated_updated_at,
                )
            };
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    rollback_security_ban_if_unchanged(
                        state,
                        previous_bans,
                        previous_updated_at,
                        mutated_bans,
                        mutated_updated_at,
                    )
                    .await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = state.security.read().await.active_bans();
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(String::new()));
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "ip": ip,
                    "banned": true,
                    "persisted": persisted,
                    "kind": record.kind,
                    "created_at": record.created_at,
                    "activeBans": active_bans,
                })
                .to_string(),
            ))
        }

        ("DELETE", path)
            if security_ban_route_tail(path)
                .is_some_and(|tail| tail.len() == 2 && tail[0] == "ip") =>
        {
            let ip = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let Some(ip) = normalize_security_ban_value("ip", &ip) else {
                return Ok(routing::bad_request_response("valid ip is required"));
            };
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, removed, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let removed = security.unban("ip", &ip);
                let mutated_bans = security.bans.clone();
                let mutated_updated_at = security.updated_at;
                (
                    previous_bans,
                    previous_updated_at,
                    removed,
                    mutated_bans,
                    mutated_updated_at,
                )
            };
            let persisted = if removed {
                match persist_security_unban(state, "ip", &ip).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        rollback_security_ban_if_unchanged(
                            state,
                            previous_bans,
                            previous_updated_at,
                            mutated_bans,
                            mutated_updated_at,
                        )
                        .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = state.security.read().await.active_bans();
            if route.path.starts_with("/api/v0/") {
                return Ok(if removed {
                    routing::ok_response(String::new())
                } else {
                    routing::not_found_response()
                });
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "ip": ip,
                    "banned": false,
                    "removed": removed,
                    "persisted": persisted,
                    "activeBans": active_bans,
                })
                .to_string(),
            ))
        }

        // ADDITIONAL MISSING DELETE ENDPOINTS (Phase 5)
        ("DELETE", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let before = messages.records.len();
            messages
                .records
                .retain(|record| record.username != username);
            let removed = before.saturating_sub(messages.records.len());
            let mutated = messages.clone();
            drop(messages);
            let persisted_removed =
                match persist_conversation_delete_checked(state, &username).await {
                    Ok(removed) => removed.unwrap_or(0),
                    Err(error) => {
                        rollback_messages_if_unchanged(state, previous, &mutated).await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                };
            drop(_message_persistence);
            if route.path.starts_with("/api/v0/") {
                Ok(if removed > 0 || persisted_removed > 0 {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                })
            } else {
                Ok(routing::ok_response(
                    (removed > 0 || persisted_removed > 0).to_string(),
                ))
            }
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/directories/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/files/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", "/api/integrations/spotify") => {
            if let Err(error) = disconnect_spotify_connection(state).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            if route.path.starts_with("/api/v0/") {
                Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::ok_response(
                    "{\"connected\":false,\"removed\":true}".to_owned(),
                ))
            }
        }

        ("DELETE", "/api/nowplaying") => {
            let _now_playing_persistence = state.now_playing_persistence_lock.lock().await;
            let mut now_playing = state.now_playing.write().await;
            let previous = now_playing.clone();
            let cleared = now_playing.clear();
            let mutated = now_playing.clone();
            drop(now_playing);
            if let Err(error) = persist_now_playing_clear_checked(state).await {
                let mut now_playing = state.now_playing.write().await;
                if *now_playing == mutated {
                    *now_playing = previous;
                }
                drop(now_playing);
                return Ok(routing::service_unavailable_response(&error));
            }
            if route.path.starts_with("/api/v0/") {
                Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::ok_response(format!(
                    "{{\"now_playing\":[],\"count\":0,\"cleared\":true,\"cleared_count\":{}}}",
                    cleared
                )))
            }
        }

        ("DELETE", "/api/relay") => {
            let json = match mutate_runtime_compat_state(state, |_, relay| relay.set_enabled(false))
                .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json.to_string()))
        }

        ("DELETE", "/api/relay/agent") => {
            let json = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_relay_agent(false).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        ("DELETE", "/api/shares") => {
            if route.path.starts_with("/api/v0/") {
                if state.share_scans.available_permits() == 0 {
                    state.share_scans.add_permits(1);
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::ok_response("true".to_owned()))
            }
        }

        ("GET", "/api/shares/contents") => {
            let shares = state.shares.read().await;
            if !shares.scan_errors.is_empty() {
                drop(shares);
                return Ok(routing::internal_server_error_response(
                    "share browse unavailable",
                ));
            }
            let json = controller_share_directories_json(&shares.entries, None);
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("GET", path) if path.starts_with("/api/shares/") && path.ends_with("/contents") => {
            let Some(share_id) = share_contents_id(path) else {
                return Ok(routing::not_found_response());
            };
            let shares = state.shares.read().await;
            let Some(root) = shares
                .roots
                .iter()
                .find(|root| share_root_id(&root.label) == share_id)
            else {
                return Ok(routing::not_found_response());
            };
            if !shares.scan_errors.is_empty() {
                drop(shares);
                return Ok(routing::internal_server_error_response(
                    "share browse unavailable",
                ));
            }
            let json = controller_share_directories_json(&shares.entries, Some(&root.label));
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("GET", path) if path.starts_with("/api/shares/") => {
            let Some(share_id) = share_resource_id(path) else {
                return Ok(routing::not_found_response());
            };
            let shares = state.shares.read().await;
            let Some(root) = shares
                .roots
                .iter()
                .find(|root| share_root_id(&root.label) == share_id)
            else {
                return Ok(routing::not_found_response());
            };
            let json = controller_share_value(root).to_string();
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("DELETE", path)
            if path.starts_with("/api/transfers/") && path.ends_with("/all/completed") =>
        {
            let mut transfers = state.transfers.write().await;
            let before = transfers.entries.len();
            transfers
                .entries
                .retain(|entry| !is_terminal_transfer_status(&entry.status));
            let pruned = before.saturating_sub(transfers.entries.len());
            transfers.persist_state();
            drop(transfers);
            Ok(routing::ok_response(format!("{{\"pruned\":{}}}", pruned)))
        }

        // Generic :var pattern endpoints for mesh/network cleanup & channels (Phase 5)
        ("DELETE", path)
            if path.contains("/cleanup")
                && path.matches('/').count() == 3
                && !path.contains("/api/") =>
        {
            Ok(routing::not_found_response())
        }

        ("DELETE", path) if path.contains("/unpublish") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("DELETE", path)
            if path.contains("/channels/")
                && path.matches('/').count() == 4
                && !path.contains("/api/") =>
        {
            Ok(routing::not_found_response())
        }

        // ADDITIONAL MISSING INTEGRATION & PLATFORM ENDPOINTS (Phase 5)
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
