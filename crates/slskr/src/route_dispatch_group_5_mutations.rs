async fn route_dispatch_group_5_mutations(
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
        ("POST" | "PUT", path)
            if path.starts_with("/api/collections/") && path.contains("/items/reorder") =>
        {
            if collection_reorder_exceeds_wire_limits(body) {
                return Ok(routing::bad_request_response(
                    "collection reorder exceeds item limits",
                ));
            }
            let collection_id = path
                .strip_prefix("/api/collections/")
                .and_then(|rest| rest.strip_suffix("/items/reorder"))
                .unwrap_or_default();
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            if collections.get(collection_id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            if let Some(record) = collections.reorder_items(collection_id, body) {
                let mutated = collections.clone();
                let items = record
                    .items
                    .iter()
                    .map(|item| {
                        serde_json::from_str::<serde_json::Value>(&item.json())
                            .unwrap_or_else(|_| serde_json::json!({ "id": item.id }))
                    })
                    .collect::<Vec<_>>();
                let item_count = items.len();
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(if request_is_versioned_v0 {
                    routing::no_content_response()
                } else {
                    routing::ok_response(
                        serde_json::json!({
                            "reordered": true,
                            "collection_id": collection_id,
                            "items": items,
                            "itemCount": item_count,
                        })
                        .to_string(),
                    )
                })
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }

        ("PUT", path) if conversation_message_path(path).is_some() => {
            let Some((username, id)) = conversation_message_path(path) else {
                return Ok(routing::not_found_response());
            };
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let username = decoded_path_segment(username);
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let updated = messages
                .records
                .iter()
                .any(|record| record.username == username && record.id == id);
            let response = if updated {
                messages.ack(id);
                let mutated = messages.clone();
                drop(messages);
                if let Err(error) = persist_message_ack_checked(state, id).await {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_message_persistence);
                if matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                ) && route.path.starts_with("/api/v0/")
                {
                    HttpResponse {
                        status: "200 OK",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response("true".to_owned())
                }
            } else {
                drop(messages);
                routing::not_found_response()
            };
            Ok(response)
        }
        ("PUT", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            if route.path.starts_with("/api/v0/")
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let ids = messages
                .records
                .iter()
                .filter(|record| record.username == username && !record.acknowledged)
                .map(|record| record.id)
                .collect::<Vec<_>>();
            messages.ack_all_for_user(&username);
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_acks_checked(state, &ids).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_message_persistence);
            Ok(
                if matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                ) && route.path.starts_with("/api/v0/")
                {
                    HttpResponse {
                        status: "200 OK",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response("true".to_owned())
                },
            )
        }

        ("PUT", "/api/nowplaying") => {
            let versioned = route.path.starts_with("/api/v0/");
            if versioned && body.trim().is_empty() {
                return Ok(routing::bad_request_response("Track data is required"));
            }
            let payload = if versioned {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(payload) if !payload.is_null() => Some(payload),
                    Ok(_) if body.trim() == "null" => {
                        return Ok(routing::bad_request_response("Track data is required"));
                    }
                    _ => None,
                }
            } else {
                None
            };
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let artist = payload
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|object| json_object_field_ci(object, "artist"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    (!versioned)
                        .then(|| extract_json_string_field(body, "artist"))
                        .flatten()
                })
                .unwrap_or_default();
            let title = payload
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|object| json_object_field_ci(object, "title"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    (!versioned)
                        .then(|| extract_json_string_field(body, "title"))
                        .flatten()
                })
                .unwrap_or_default();
            if artist.trim().is_empty() || title.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "Artist and title are required",
                ));
            }
            let _now_playing_persistence = state.now_playing_persistence_lock.lock().await;
            let mut now_playing = state.now_playing.write().await;
            let previous = now_playing.clone();
            let record = now_playing.upsert(username, artist, title);
            let mutated = now_playing.clone();
            drop(now_playing);
            if let Err(error) = persist_now_playing_checked(state, &record).await {
                let mut now_playing = state.now_playing.write().await;
                if *now_playing == mutated {
                    *now_playing = previous;
                }
                drop(now_playing);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(HttpResponse {
                status: "204 No Content",
                content_type: "",
                body: String::new(),
            })
        }

        ("PUT", "/api/options/yaml") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(apply_controller_yaml_upload(body, state).await)
        }

        ("PUT", "/api/profile/me") => {
            if route.path.starts_with("/api/v0/") {
                let payload = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Object(payload)) => payload,
                    Ok(serde_json::Value::Null) => {
                        return Ok(routing::bad_request_response("Request is required."));
                    }
                    Ok(_) | Err(_) => {
                        return Ok(routing::bad_request_response("Request is required."));
                    }
                };
                let display_name = extract_json_string_field(body, "displayName")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if display_name.is_empty() {
                    return Ok(routing::bad_request_response("DisplayName is required."));
                }
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                let avatar = extract_json_string_field(body, "avatar")
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.trim().to_owned());
                let capabilities = extract_json_i32_field(body, "capabilities").unwrap_or(0);
                let endpoints = payload
                    .get("endpoints")
                    .cloned()
                    .filter(serde_json::Value::is_array)
                    .unwrap_or_else(|| serde_json::json!([]));
                state.session.write().await.username = Some(display_name.clone());
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "peerId": descriptor.peer_id,
                        "publicKey": STANDARD.encode(descriptor.public_key),
                        "displayName": display_name,
                        "avatar": avatar,
                        "capabilities": capabilities,
                        "endpoints": endpoints,
                        "createdAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "expiresAt": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
                        "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                    })
                    .to_string(),
                ));
            }
            let username = extract_json_string_field(body, "username")
                .or_else(|| extract_json_string_field(body, "name"));
            let privileges_seconds = extract_json_u32_field(body, "privilegesSeconds")
                .or_else(|| extract_json_u32_field(body, "privileges_seconds"));
            let connected = extract_json_bool_field(body, "connected");
            let supporter = extract_json_bool_field(body, "supporter");
            let mut session = state.session.write().await;
            if let Some(username) = username.filter(|value| !value.trim().is_empty()) {
                session.username = Some(username);
            }
            if let Some(privileges_seconds) = privileges_seconds {
                session.privileges_seconds = Some(privileges_seconds);
            }
            if let Some(supporter) = supporter {
                session.supporter = Some(supporter);
            }
            if let Some(connected) = connected {
                session.state = if connected {
                    "connected"
                } else {
                    "disconnected"
                };
                session.connected_at = if connected {
                    session.connected_at.or_else(|| Some(unix_timestamp()))
                } else {
                    None
                };
            }
            session.updated_at = unix_timestamp();
            let username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            let privileges_seconds = session.privileges_seconds.unwrap_or(0);
            let connected = session.state == "connected";
            let updated_at = session.updated_at;
            drop(session);
            let users = state.users.read().await;
            let watched = users
                .records
                .iter()
                .any(|user| user.username.eq_ignore_ascii_case(&username) && user.watched);
            drop(users);
            Ok(routing::ok_response(
                serde_json::json!({
                    "updated": true,
                    "persisted": true,
                    "profile": {
                        "username": username,
                        "description": "",
                        "picture": "",
                        "user_type": if privileges_seconds > 0 { "privileged" } else { "normal" },
                        "privilegesSeconds": privileges_seconds,
                        "connected": connected,
                        "watched": watched,
                        "updated_at": updated_at,
                    }
                })
                .to_string(),
            ))
        }

        ("PUT", "/api/relay") => {
            let relay_enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
            let json = match mutate_runtime_compat_state(state, |_, relay| {
                relay.set_enabled(relay_enabled).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        ("PUT", "/api/relay/agent") => {
            let enabled = extract_json_bool_field(body, "enabled").unwrap_or(true);
            let json = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_relay_agent(enabled).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        ("PUT", path) if path.starts_with("/api/searches/") => {
            let Some(id) = path_segment_after(path, "/api/searches/") else {
                return Ok(routing::not_found_response());
            };
            // The WebUI uses the unversioned REST route with an empty body to
            // stop a search.  Treat that shape as the cancellation contract
            // for every profile; falling through to update_by_identifier()
            // leaves an active search running because it has no fields to
            // update.
            if body.trim().is_empty() {
                let mut searches = state.searches.write().await;
                let previous_searches = searches.clone();
                let Some(existing) = searches.get_by_identifier(id) else {
                    drop(searches);
                    return Ok(routing::not_found_response());
                };
                let Some((record, transitioned)) =
                    searches.set_status_by_token(existing.token, "cancelled")
                else {
                    drop(searches);
                    return Ok(routing::not_found_response());
                };
                let mutated_searches = searches.clone();
                drop(searches);
                if transitioned {
                    if let Err(error) = persist_search_record(state, &record).await {
                        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                            .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    publish_search_hub_event(state, "update", &record);
                    record_event(state, "search.cancelled", record.token.to_string(), None).await;
                }
                return Ok(routing::ok_response(String::new()));
            }
            let query = extract_json_string_field(body, "query")
                .or_else(|| extract_json_string_field(body, "searchText"));
            let status = extract_json_string_field(body, "status")
                .or_else(|| extract_json_string_field(body, "state"))
                .or_else(|| {
                    extract_json_bool_field(body, "isComplete").map(|is_complete| {
                        if is_complete {
                            "completed".to_owned()
                        } else {
                            "active".to_owned()
                        }
                    })
                });
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            match searches.update_by_identifier(id, query, status.as_deref()) {
                Some((record, updated)) => {
                    let mut value = serde_json::from_str::<serde_json::Value>(&record.json())
                        .map_err(|error| format!("search json build failed: {error}"))?;
                    if let Some(object) = value.as_object_mut() {
                        object.insert("updated".to_owned(), serde_json::Value::Bool(updated));
                    }
                    let mutated_searches = searches.clone();
                    drop(searches);
                    if let Err(error) = persist_search_record(state, &record).await {
                        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                            .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::ok_response(value.to_string()))
                }
                None => {
                    drop(searches);
                    Ok(routing::not_found_response())
                }
            }
        }

        ("PUT", "/api/transfers/downloads/accelerated") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                let enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
                {
                    let mut runtime = state.runtime.write().await;
                    runtime.accelerated_downloads_enabled = enabled;
                    runtime.updated_at = unix_timestamp();
                }
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "enabled": enabled,
                        "updatedAt": chrono::Utc::now().to_rfc3339(),
                        "policy": "Normal downloads remain single-source. Underperforming downloads may use verified alternate sources; raw Soulseek peers use sequential failover, while true multipart chunking is reserved for trusted mesh-overlay peers.",
                    })
                    .to_string(),
                ));
            }
            let transfers = state.transfers.read().await;
            let mut payload = serde_json::from_str::<serde_json::Value>(
                &controller_accelerated_downloads_json(route.query, &transfers),
            )
            .map_err(|error| format!("accelerated json failed: {error}"))?;
            drop(transfers);
            payload["persisted"] = serde_json::Value::Bool(false);
            Ok(routing::ok_response(payload.to_string()))
        }

        ("PUT", "/api/wishlist/bulk-filter") => {
            if wishlist_bulk_filter_exceeds_wire_limits(body) {
                return Ok(routing::bad_request_response(
                    "wishlist bulk filter exceeds item limits",
                ));
            }
            let requested_ids = extract_json_string_array_field(body, "ids")
                .or_else(|| extract_json_string_array_field(body, "itemIds"))
                .unwrap_or_default();
            if requested_ids.is_empty() {
                return Ok(routing::bad_request_response(
                    "At least one wishlist item ID is required",
                ));
            }
            let filter = extract_json_string_field(body, "filter").unwrap_or_default();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let ids = requested_ids
                .iter()
                .map(|id| wishlist.resolve_item_id(id, compatibility_contract))
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default();
            if ids.is_empty() {
                drop(wishlist);
                return Ok(routing::not_found_response());
            }
            let previous = wishlist.clone();
            let updated = match wishlist.update_filters(&ids, filter) {
                Ok(items) => items,
                Err("wishlist item not found") => {
                    drop(wishlist);
                    return Ok(routing::not_found_response());
                }
                Err(message) => {
                    drop(wishlist);
                    return Ok(routing::bad_request_response(message));
                }
            };
            let mutated = wishlist.clone();
            let updated_count = updated.len();
            drop(wishlist);
            if let Err(error) = persist_wishlist_items_checked(state, &updated).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::ok_response(
                serde_json::json!({ "updatedCount": updated_count }).to_string(),
            ))
        }

        ("PUT", path) if path.starts_with("/api/wishlist/") => {
            let Some(requested_item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let search_text = extract_json_string_field(body, "searchText");
            if compatibility_contract
                && search_text
                    .as_ref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Ok(routing::bad_request_response("SearchText is required"));
            }
            let artist = if compatibility_contract {
                search_text.clone()
            } else {
                extract_json_string_field(body, "artist")
            };
            let title = if compatibility_contract {
                Some(String::new())
            } else {
                extract_json_string_field(body, "title").or(search_text)
            };
            let kind = extract_json_string_field(body, "kind");
            let filter = extract_json_string_field(body, "filter");
            let enabled =
                extract_json_bool_field(body, "enabled").or(compatibility_contract.then_some(true));
            let auto_download = extract_json_bool_field(body, "autoDownload")
                .or(compatibility_contract.then_some(false));
            let max_results = extract_json_u64_field(body, "maxResults")
                .or(compatibility_contract.then_some(100));
            if max_results.is_some_and(|value| value == 0 || value > MAX_WISHLIST_RESULTS as u64) {
                return Ok(routing::bad_request_response(
                    "MaxResults must be between 1 and 10000",
                ));
            }
            let max_downloads = extract_json_optional_u64_field(body, "maxDownloads")
                .or(compatibility_contract.then_some(None));
            if max_downloads
                .flatten()
                .is_some_and(|value| value == 0 || value > MAX_WISHLIST_DOWNLOADS)
            {
                return Ok(routing::bad_request_response(
                    "MaxDownloads must be null or between 1 and 1000000",
                ));
            }
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(routing::not_found_response());
            };
            let previous = wishlist.clone();
            if let Some(item) = wishlist.update_item(
                &item_id,
                artist,
                title,
                kind,
                filter,
                enabled,
                auto_download,
                max_results.and_then(|value| usize::try_from(value).ok()),
                max_downloads,
            ) {
                let mutated = wishlist.clone();
                let json = if compatibility_contract {
                    item.native_json()
                } else {
                    item.json()
                };
                drop(wishlist);
                if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    return Ok(wishlist_storage_error_response(
                        compatibility_contract,
                        &error,
                    ));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(wishlist);
                Ok(routing::not_found_response())
            }
        }

        // Generic :var pattern PUT endpoints (Phase 5)
        ("PUT", path)
            if path.contains("/channels/")
                && path.matches('/').count() == 4
                && !path.contains("/api/") =>
        {
            Ok(routing::not_found_response())
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
