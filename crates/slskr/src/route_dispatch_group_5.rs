async fn route_dispatch_group_5(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    } = *context;
    match (method, normalized_path) {
        ("GET", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let grants = state.share_grants.read().await;
            let record = grants.get(id);
            drop(grants);
            let Some(record) = record else {
                return Ok(routing::not_found_response());
            };
            if share_grant_collection_forbids(state, &record.collection_id, caller_id.as_deref())
                .await
            {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(record.json()))
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/by-collection/")
                && share_grant_collection_id(path).is_some() =>
        {
            let collection_id =
                share_grant_collection_id(path).expect("guarded share-grant collection path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            // SharesController.GetByCollection first resolves the collection
            // and returns NotFound when the collection no longer exists.
            let collection_exists = state.collections.read().await.get(collection_id).is_some();
            if !collection_exists {
                return Ok(routing::not_found_response());
            }
            // Matches the oracle's real GetByCollection: this is the
            // "outgoing shares" owner-perspective view, so it 404s unless
            // the caller actually owns this collection.
            if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await {
                return Ok(routing::not_found_response());
            }
            let grants = state.share_grants.read().await;
            let records = grants.get_by_collection(collection_id);
            let json = records
                .iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(",");
            let response = format!("[{}]", json);
            drop(grants);
            Ok(routing::ok_response(response))
        }
        ("PUT", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let permissions = extract_json_string_field(body, "permissions")
                .unwrap_or_else(|| "read".to_string());
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut grants = state.share_grants.write().await;
            let owning_collection_id = grants.get(id).map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
                    drop(grants);
                    return Ok(routing::not_found_response());
                }
            }
            let previous = grants.clone();
            if let Some(record) = grants.update(id, permissions) {
                let json = record.json();
                if let Err(error) = persist_share_grant(state, &record).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                Ok(routing::ok_response(json))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let mut grants = state.share_grants.write().await;
            let owning_collection_id = grants.get(id).map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
                    drop(grants);
                    return Ok(routing::not_found_response());
                }
            }
            let previous = grants.clone();
            let deleted = grants.delete(id);
            if deleted {
                if let Err(error) = persist_share_grant_delete_checked(state, id).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                state.share_access_tokens.write().await.revoke_grant(id);
                state
                    .stream_tickets
                    .write()
                    .await
                    .revoke_source(&format!("share:{id}"));
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }

        // LIBRARY ITEMS ENDPOINTS
        ("GET", "/api/library/items") => {
            if state.config.controller_profile == ControllerProfile::Native {
                return Ok(routing::ok_response(
                    native_library_items_search_json(state, route.query).await,
                ));
            }
            let library = state.library.read().await;
            let json = library.json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/items/browser") => {
            let library = state.library.read().await;
            let path = query_parameter(route.query, "path").unwrap_or_default();
            let limit = query_parameter(route.query, "limit")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(100)
                .clamp(1, 1_000);
            let offset = query_parameter(route.query, "offset")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let total_files = library.records.len();
            let files = library
                .records
                .iter()
                .skip(offset)
                .take(limit)
                .map(|record| {
                    serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_default()
                })
                .collect::<Vec<_>>();
            let returned_files = files.len();
            let json = serde_json::json!({
                "path": path,
                "breadcrumbs": [],
                "directories": [],
                "files": files,
                "totalFiles": total_files,
                "totalDirectories": 0,
                "offset": offset,
                "limit": limit,
                "hasMore": offset.saturating_add(returned_files) < total_files,
                "duplicatesRemoved": 0,
            })
            .to_string();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/library/items") => {
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind =
                extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            let mut library = state.library.write().await;
            let previous = library.clone();
            let Some(record) = library.create(artist, title, kind) else {
                return Ok(routing::service_unavailable_response(
                    "library item capacity is full",
                ));
            };
            let mutated = library.clone();
            let json = record.json();
            drop(library);
            if let Err(error) = persist_library_item_checked(state, &record).await {
                rollback_library_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let library = state.library.read().await;
            if let Some(record) = library.get(id) {
                let json = record.json();
                drop(library);
                Ok(routing::ok_response(json))
            } else {
                drop(library);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let mut library = state.library.write().await;
            let previous = library.clone();
            let deleted = library.delete(id);
            let mutated = library.clone();
            drop(library);
            if deleted {
                if let Err(error) = persist_library_item_delete_checked(state, id).await {
                    rollback_library_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // DESTINATIONS ENDPOINTS
        ("GET", "/api/destinations") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_list()
                } else {
                    configured.list()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_list()
            } else {
                destinations.list()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/destinations/default") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_default()
                } else {
                    configured.default()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_default()
            } else {
                destinations.default()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/config/download-filter") => {
            let exclusions = effective_download_exclusions(state).await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "exclude": exclusions,
                    "maxTerms": 100,
                    "maxTermLength": 256,
                })
                .to_string(),
            ))
        }
        ("PUT", "/api/config/download-filter") => Ok(update_download_filter(state, body).await),

        // BROWSE ENDPOINTS
        ("GET", path)
            if path.starts_with("/api/users/")
                && path.ends_with("/browse/status")
                && user_route_username(path, "/browse/status").is_some() =>
        {
            let username = user_route_username(path, "/browse/status")
                .expect("guarded user browse status path");
            if state.runtime.read().await.relay_agent_enabled {
                return Ok(controller_forbidden_response());
            }
            let browse = state.browse.read().await;
            let tracked = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(BrowseRecord::controller_status_json);
            drop(browse);
            match tracked {
                Some(body) => Ok(routing::ok_response(body)),
                None => Ok(routing::not_found_response()),
            }
        }
        // ADDITIONAL MISSING USER ENDPOINTS (Phase 5)
        ("GET", "/api/profile/me") => {
            if route.path.starts_with("/api/v0/") {
                let session = state.session.read().await;
                let display_name = session
                    .username
                    .clone()
                    .or_else(|| state.config.username.clone())
                    .unwrap_or_else(|| "Unknown".to_owned())
                    .trim()
                    .to_owned();
                drop(session);
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                let peer_id = local_profile_peer_id(state);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "peerId": peer_id,
                        "publicKey": STANDARD.encode(descriptor.public_key),
                        "displayName": display_name,
                        "avatar": null,
                        "capabilities": 0,
                        "endpoints": [],
                        "createdAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "expiresAt": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
                        "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                    })
                    .to_string(),
                ));
            }
            let session = state.session.read().await;
            let username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            let privileges_seconds = session.privileges_seconds.unwrap_or(0);
            let connected = session.state == "connected";
            drop(session);
            let users = state.users.read().await;
            let watched = users
                .records
                .iter()
                .any(|user| user.username.eq_ignore_ascii_case(&username) && user.watched);
            drop(users);
            let descriptor = local_capability_descriptor(state).await.ok();
            let json = serde_json::json!({
                "peerId": descriptor.as_ref().map(|value| value.peer_id.as_str()).unwrap_or(""),
                "publicKey": descriptor.as_ref().map(|value| STANDARD.encode(value.public_key)).unwrap_or_default(),
                "displayName": username.clone(),
                "capabilities": descriptor.as_ref().map(|value| value.features.clone()).unwrap_or_default(),
                "endpoints": descriptor.as_ref().map(|value| value.endpoints.clone()).unwrap_or_default(),
                "createdAt": descriptor.as_ref().map(|value| value.issued_at_unix).unwrap_or(0),
                "expiresAt": descriptor.as_ref().map(|value| value.expires_at_unix).unwrap_or(0),
                "signature": descriptor.as_ref().and_then(|value| value.signature).map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                "username": username,
                "description": "",
                "picture": "",
                "user_type": if privileges_seconds > 0 { "privileged" } else { "normal" },
                "privilegesSeconds": privileges_seconds,
                "connected": connected,
                "watched": watched,
            }).to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if route.path.starts_with("/api/v0/") && path.starts_with("/api/profile/") =>
        {
            let Some(raw_peer_id) = path.strip_prefix("/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            if raw_peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            if raw_peer_id.contains('/') {
                return Ok(routing::not_found_response());
            }
            let peer_id = decoded_path_segment(raw_peer_id).trim().to_owned();
            if peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            let local_peer_id = local_profile_peer_id(state);
            if !peer_id.eq_ignore_ascii_case(&local_peer_id) {
                return Ok(routing::not_found_response());
            }
            let session = state.session.read().await;
            let display_name = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "Unknown".to_owned())
                .trim()
                .to_owned();
            drop(session);
            return Ok(routing::ok_response(
                serde_json::json!({
                    "peerId": local_peer_id,
                    "displayName": display_name,
                    "avatar": null,
                    "capabilities": 0,
                    "endpoints": [],
                })
                .to_string(),
            ));
        }

        ("GET", path) if path.starts_with("/api/profile/") => {
            let Some(username) = path_segment_after(path, "/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let users = state.users.read().await;
            let user = users
                .records
                .iter()
                .find(|user| user.username.eq_ignore_ascii_case(&username))
                .cloned();
            drop(users);
            let json = serde_json::json!({
                "username": username,
                "description": "",
                "picture": "",
                "user_type": "normal",
                "watched": user.as_ref().is_some_and(|user| user.watched),
                "status": user.as_ref().and_then(|user| user.status.clone()).unwrap_or_else(|| "Unknown".to_owned()),
                "averageSpeed": user.as_ref().and_then(|user| user.average_speed).unwrap_or(0),
                "uploadCount": user.as_ref().and_then(|user| user.upload_count).unwrap_or(0),
                "fileCount": user.as_ref().and_then(|user| user.file_count).unwrap_or(0),
                "directoryCount": user.as_ref().and_then(|user| user.directory_count).unwrap_or(0),
            }).to_string();
            Ok(routing::ok_response(json))
        }

        // CONVERSATIONS ENDPOINT
        ("GET", "/api/conversations") => {
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.controller_conversations_json(route.query);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/conversations/activity/unacknowledged") => {
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.has_unacknowledged_messages().to_string();
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if conversation_messages_path(path).is_some() => {
            let Some(username) = conversation_messages_path(path) else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let unacknowledged_only = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "unAcknowledgedOnly")
                .map(|(_, value)| value)
            {
                None => false,
                Some(value) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "unAcknowledgedOnly must be a boolean",
                        ))
                    }
                },
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_messages_json(&username, unacknowledged_only);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let include_messages = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includeMessages")
            {
                None => true,
                Some((_, value)) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "includeMessages must be a boolean",
                        ))
                    }
                },
            };
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_conversation_json(&username, include_messages, since);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("POST", "/api/conversations/batch") => {
            let usernames = match extract_json_string_array_field(body, "usernames")
                .or_else(|| extract_json_string_array_field(body, "recipients"))
            {
                Some(usernames) => usernames,
                None => {
                    return Ok(routing::bad_request_response(
                        "usernames/recipients array is required",
                    ))
                }
            };
            let message_body = match extract_json_string_field(body, "body")
                .or_else(|| extract_json_string_field(body, "message"))
            {
                Some(body) => body,
                None => return Ok(routing::bad_request_response("body/message is required")),
            };
            let command = match private_message_users_command(usernames, message_body.clone()) {
                Ok(ServerMessage::MessageUsers { usernames, .. }) => usernames,
                Ok(_) => return Ok(routing::bad_request_response("invalid message command")),
                Err(error) => return Ok(routing::bad_request_response(&error.to_string())),
            };

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let records: Vec<_> = command
                .iter()
                .map(|username| messages.add(username.clone(), "outbound", message_body.clone()))
                .collect();
            let mutated = messages.clone();
            drop(messages);

            if let Err(error) = persist_message_records_checked(state, &records).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::MessageUsers {
                usernames: command.clone(),
                body: message_body,
            });

            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                Ok(HttpResponse {
                    status: "201 Created",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::created_response(
                    serde_json::json!({
                        "conversations": records.iter().map(MessageRecord::json).collect::<Vec<_>>(),
                        "usernames": command,
                        "count": records.len(),
                    })
                    .to_string(),
                ))
            }
        }
        ("POST", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if message_body.trim().is_empty() {
                return Ok(routing::bad_request_response("message is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "outbound", message_body.clone());
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::MessageUser {
                username,
                body: message_body,
            });
            Ok(
                if matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                ) && route.path.starts_with("/api/v0/")
                {
                    HttpResponse {
                        status: "201 Created",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response((record.id > 0).to_string())
                },
            )
        }

        // JOBS ENDPOINT
        ("GET", "/api/jobs") => {
            let searches = state.searches.read().await;
            let transfers = state.transfers.read().await;
            let mut jobs = searches
                .records
                .iter()
                .map(|record| {
                    serde_json::json!({
                        "id": record.id,
                        "kind": "search",
                        "type": "search",
                        "status": record.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": if record.status == "completed" { 1 } else { 0 },
                            "releases_failed": 0,
                        },
                        "progress_percent": if record.status == "completed" { 100 } else { 50 },
                        "query": record.query,
                        "created_at": record.created_at,
                        "updated_at": record.updated_at,
                    })
                })
                .collect::<Vec<_>>();
            jobs.extend(transfers.entries.iter().map(|entry| {
                let size = entry.size.unwrap_or(0);
                let progress = entry
                    .bytes_transferred
                    .saturating_mul(100)
                    .checked_div(size)
                    .unwrap_or(0)
                    .min(100);
                serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "type": "transfer",
                        "status": entry.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": 0,
                            "releases_failed": if is_failed_transfer_status(&entry.status) { 1 } else { 0 },
                        },
                        "progress_percent": progress,
                        "filename": entry.filename,
                        "created_at": entry.requested_at,
                        "updated_at": entry.updated_at,
                })
            }));
            let total = jobs.len();
            drop(transfers);
            drop(searches);
            Ok(routing::ok_response(
                serde_json::json!({
                    "jobs": jobs,
                    "limit": 100,
                    "offset": 0,
                    "total": total,
                    "has_more": total > 100,
                })
                .to_string(),
            ))
        }
        ("GET", path) if path.starts_with("/api/jobs/discography/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/discography/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            if let Some(job) = state
                .controller_features
                .read()
                .await
                .get(&format!("job/discography/{job_id}"))
                .cloned()
            {
                return Ok(routing::ok_response(job.to_string()));
            }
            let searches = state.searches.read().await;
            let Some(record) = searches.get_by_identifier(&job_id) else {
                return Ok(routing::not_found_response());
            };
            let artist = record
                .query
                .strip_suffix(" discography")
                .unwrap_or(&record.query)
                .trim();
            return Ok(routing::ok_response(
                serde_json::json!({
                    "jobId": record.id,
                    "artistId": artist,
                    "artistName": artist,
                    "profile": "CoreDiscography",
                    "targetDirectory": "",
                    "releaseJobIds": [],
                    "releaseIds": [],
                    "totalReleases": 0,
                    "completedReleases": 0,
                    "failedReleases": 0,
                    "status": "Pending",
                    "createdAt": record.created_at.to_string(),
                })
                .to_string(),
            ));
        }
        ("GET", path) if path.starts_with("/api/jobs/label-crate/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/label-crate/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            let features = state.controller_features.read().await;
            let Some(job) = features.get(&format!("job/label-crate/{job_id}")).cloned() else {
                return Ok(routing::not_found_response());
            };
            Ok(routing::ok_response(job.to_string()))
        }
        ("GET", path) if path.starts_with("/api/jobs/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(&job_id) {
                let progress = if record.status == "completed" {
                    100
                } else {
                    50
                };
                let body = serde_json::json!({
                    "id": record.id,
                    "kind": "search",
                    "status": record.status,
                    "progress": progress,
                    "query": record.query,
                    "target": record.target,
                    "result_count": record.results.len(),
                    "created_at": record.created_at,
                    "updated_at": record.updated_at,
                })
                .to_string();
                drop(searches);
                return Ok(routing::ok_response(body));
            }
            drop(searches);

            if let Some(job) = state
                .controller_features
                .read()
                .await
                .entries_with_prefix("job/")
                .into_iter()
                .find_map(|(_, job)| {
                    (job.get("jobId").and_then(serde_json::Value::as_str) == Some(job_id.as_str()))
                        .then_some(job)
                })
            {
                return Ok(routing::ok_response(job.to_string()));
            }

            let transfer_id = job_id.strip_prefix("transfer-").unwrap_or(&job_id);
            let transfers = state.transfers.read().await;
            let body = transfer_id
                .parse::<u64>()
                .ok()
                .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id))
                .map(|entry| {
                    let size = entry.size.unwrap_or(0);
                    let progress = entry
                        .bytes_transferred
                        .saturating_mul(100)
                        .checked_div(size)
                        .unwrap_or(0)
                        .min(100);
                    serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "status": entry.status,
                        "progress": progress,
                        "filename": entry.filename,
                        "bytesTransferred": entry.bytes_transferred,
                        "size": size,
                        "created_at": entry.requested_at,
                        "updated_at": entry.updated_at,
                    })
                })
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "id": job_id,
                        "status": "not_found",
                        "progress": 0,
                    })
                })
                .to_string();
            drop(transfers);
            if let Some(job) = state.library.read().await.remediation_job(&job_id) {
                return Ok(routing::ok_response(job.json().to_string()));
            }
            if state.config.controller_profile == ControllerProfile::Native
                && body.contains("\"status\":\"not_found\"")
            {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(body))
        }

        // LIBRARY HEALTH ENDPOINTS
        ("GET", "/api/library/health/summary") => {
            let library_path = query_parameter(route.query, "libraryPath").unwrap_or_default();
            if library_path.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "libraryPath query parameter is required",
                ));
            }
            let library = state.library.read().await;
            let json = library.health_summary_json(library_path);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/dashboard") => {
            let library_path = query_parameter(route.query, "libraryPath").unwrap_or_default();
            if library_path.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "libraryPath query parameter is required",
                ));
            }
            let artist_limit = match query_bounded_usize(route.query, "artistLimit", 1, 100) {
                Ok(value) => value.unwrap_or(10),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "artistLimit must be between 1 and 100",
                    ));
                }
            };
            let issue_limit = match query_bounded_usize(route.query, "issueLimit", 1, 250) {
                Ok(value) => value.unwrap_or(100),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "issueLimit must be between 1 and 250",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_dashboard_json(library_path, artist_limit, issue_limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues") => {
            let filter = match LibraryHealthIssueQuery::from_query(route.query) {
                Ok(filter) => filter,
                Err(error) => {
                    return Ok(routing::bad_request_response(&error));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_json(&filter);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-artist") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                Ok(value) => value.unwrap_or(20),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 100",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_artist_json(limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-release") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                Ok(value) => value.unwrap_or(20),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 100",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_release_json(limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-codec") => {
            let library = state.library.read().await;
            let json = library.health_issues_by_codec_json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/issues/by-type") => {
            let issue_type = if path == "/api/library/health/issues/by-type" {
                None
            } else if let Some(issue_type) =
                path_segment_after(path, "/api/library/health/issues/by-type/")
            {
                Some(issue_type)
            } else {
                return Ok(routing::not_found_response());
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_type_json(issue_type);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/scans/") => {
            let Some(scan_id) = path_segment_after(path, "/api/library/health/scans/") else {
                return Ok(routing::not_found_response());
            };
            let mut library = state.library.write().await;
            library.refresh_health_scans();
            let scan = library.health_scan(scan_id);
            drop(library);
            Ok(scan
                .map(|record| routing::ok_response(record.json()))
                .unwrap_or_else(routing::not_found_response))
        }
        ("POST", "/api/library/health/scans") => {
            if route.path.starts_with("/api/v0/")
                && (body.trim().is_empty()
                    || !serde_json::from_str::<serde_json::Value>(body)
                        .is_ok_and(|value| value.is_object()))
            {
                return Ok(routing::bad_request_response(
                    "library scan body must be an object",
                ));
            }
            let library_path = extract_json_string_field(body, "libraryPath")
                .or_else(|| extract_json_string_field(body, "path"))
                .unwrap_or_default();
            let mut library = state.library.write().await;
            let scan = match library.start_health_scan(library_path) {
                Ok(scan) => scan,
                Err(active_id) => {
                    return Ok(routing::conflict_response(&format!(
                        "scan already running: {active_id}"
                    )));
                }
            };
            drop(library);
            Ok(routing::ok_response(
                serde_json::json!({
                    "scanId": scan.id,
                    "message": "Scan started successfully",
                })
                .to_string(),
            ))
        }
        ("POST", "/api/library/health/issues/fix") => {
            if route.path.starts_with("/api/v0/")
                && !body.trim().is_empty()
                && !serde_json::from_str::<serde_json::Value>(body)
                    .is_ok_and(|value| value.is_object())
            {
                return Ok(routing::bad_request_response(
                    "library remediation body must be an object",
                ));
            }
            let mut library = state.library.write().await;
            let previous = library.clone();
            let fixable = library
                .health_issues()
                .into_iter()
                .filter(|issue| {
                    issue.get("type").and_then(serde_json::Value::as_str) == Some("missing_kind")
                })
                .count();
            let fixed = library.fix_health_issues();
            let records = library.records.clone();
            let remaining = library.health_issues().len();
            let updated_at = library.updated_at;
            let mutated = library.clone();
            drop(library);
            if !fixed.is_empty() {
                if let Err(error) = persist_library_items_checked(state, &records).await {
                    rollback_library_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "fixed": fixed.len(),
                    "fixable": fixable,
                    "issues": fixed,
                    "remaining": remaining,
                    "persisted": true,
                    "updated_at": updated_at,
                })
                .to_string(),
            ))
        }

        // CONFIGURATION ENDPOINTS
        ("GET", "/api/config/preferences") => {
            let runtime = state.runtime.read().await;
            let body = serde_json::json!({
                "auto_connect": state.config.auto_connect,
                "transfer_allow_outbound": state.config.transfer_allow_outbound,
                "transfer_max_active": state.config.transfer_max_active,
                "autoreplace_enabled": runtime.autoreplace_enabled,
            })
            .to_string();
            drop(runtime);
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/config/preferences") => {
            let requested = extract_json_bool_field(body, "autoreplace_enabled")
                .or_else(|| extract_json_bool_field(body, "autoreplaceEnabled"));
            let response = match mutate_runtime_compat_state(state, |runtime, _| {
                if let Some(enabled) = requested {
                    runtime.set_autoreplace(enabled);
                }
                serde_json::json!({
                    "auto_connect": state.config.auto_connect,
                    "transfer_allow_outbound": state.config.transfer_allow_outbound,
                    "transfer_max_active": state.config.transfer_max_active,
                    "autoreplace_enabled": runtime.autoreplace_enabled,
                    "persisted": true,
                    "updated_at": runtime.updated_at,
                })
                .to_string()
            })
            .await
            {
                Ok(response) => response,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(response))
        }

        // ADDITIONAL MISSING PUT ENDPOINTS (Phase 5)
        ("PUT", "/api/autoreplace/disable") => {
            let versioned = route.path.starts_with("/api/v0/");
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let legacy = runtime.set_autoreplace(false);
                if versioned {
                    serde_json::json!({
                        "enabled": false,
                        "lastRunAt": null,
                        "lastRunProcessedCount": 0,
                        "lastRunReplacedCount": 0,
                        "intervalSeconds": 300,
                    })
                    .to_string()
                } else {
                    legacy.to_string()
                }
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/autoreplace/enable") => {
            let versioned = route.path.starts_with("/api/v0/");
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let legacy = runtime.set_autoreplace(true);
                if versioned {
                    serde_json::json!({
                        "enabled": true,
                        "lastRunAt": null,
                        "lastRunProcessedCount": 0,
                        "lastRunReplacedCount": 0,
                        "intervalSeconds": 300,
                    })
                    .to_string()
                } else {
                    legacy.to_string()
                }
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        // ADDITIONAL MISSING BRIDGE ENDPOINTS (Phase 6)
        ("GET", "/api/bridge/admin/clients") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let status = if runtime.bridge_running {
                "running"
            } else if bridge.enabled {
                "configured"
            } else {
                "disabled"
            };
            // Matches the oracle's real BridgeDashboard client snapshot;
            // only sessions accepted by the embedded Soulfind-protocol
            // listener are included.
            let clients = runtime
                .bridge_active_clients
                .values()
                .cloned()
                .collect::<Vec<_>>();
            if state.config.controller_profile == ControllerProfile::Native {
                drop(runtime);
                return Ok(routing::ok_response(
                    serde_json::json!({"clients": clients}).to_string(),
                ));
            }
            let json = serde_json::json!({
                "clients": clients,
                "count": runtime.bridge_active_clients.len(),
                "status": status,
                "ready": bridge.enabled,
            });
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/admin/config") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            Ok(routing::ok_response(
                serde_json::json!({
                    "enabled": bridge.enabled,
                    "port": bridge.port,
                    "soulfind_path": "soulfind",
                    "max_clients": bridge.max_clients,
                    "require_auth": bridge.require_auth,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/bridge/admin/dashboard") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let active_transfers = transfers
                .entries
                .iter()
                .filter(|entry| is_queued_or_active_transfer_status(&entry.status))
                .count();
            let bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let transfer_count = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let started_at = runtime
                    .bridge_started_at
                    .map(bridge_started_at_string)
                    .unwrap_or_else(|| "0001-01-01T00:00:00+00:00".to_owned());
                let uptime = runtime
                    .bridge_started_at
                    .map(|started| {
                        format_timespan_hms(
                            i64::try_from(unix_timestamp().saturating_sub(started))
                                .unwrap_or(i64::MAX),
                        )
                    })
                    .unwrap_or_else(|| "00:00:00".to_owned());
                let json = serde_json::json!({
                    "health": {
                        "isHealthy": runtime.bridge_running,
                        "version": "1.0.0-proxy",
                        "activeConnections": runtime.bridge_active_clients.len(),
                        "startedAt": started_at,
                    },
                    "connectedClients": [],
                    "stats": {
                        "totalConnections": runtime.bridge_total_connections,
                        "currentConnections": runtime.bridge_active_clients.len(),
                        "totalSearches": runtime.bridge_total_searches,
                        "totalDownloads": runtime.bridge_total_downloads,
                        "totalRoomJoins": runtime.bridge_total_room_joins,
                        "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                        "uptime": uptime,
                    },
                    "meshBenefits": {
                        "bytesViaMesh": 0,
                        "bytesViaSoulseek": 0,
                        "meshPercentage": 0.0,
                        "disasterModeActivations": 0,
                        "timeInDisasterMode": "00:00:00",
                    },
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            // Matches the oracle's real BridgeDashboardData contract:
            // Local HTTP transfer activity remains separate from the
            // protocol bridge counters below.
            let json = serde_json::json!({
                "health": if runtime.bridge_running { "Healthy" } else { "Disabled" },
                "connectedClients": runtime.bridge_active_clients.len(),
                "stats": {
                    "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                    "totalConnections": runtime.bridge_total_connections,
                    "currentConnections": runtime.bridge_active_clients.len(),
                    "totalSearches": runtime.bridge_total_searches,
                    "totalDownloads": runtime.bridge_total_downloads,
                    "totalRoomJoins": runtime.bridge_total_room_joins,
                    "uptime": runtime.bridge_started_at.map(|started| format_timespan_hms(
                        i64::try_from(unix_timestamp().saturating_sub(started)).unwrap_or(i64::MAX)
                    )).unwrap_or_else(|| "00:00:00".to_owned()),
                },
                "meshBenefits": {"enabled": bridge.enabled},
                "active_clients": runtime.bridge_active_clients.len(),
                "transfers": transfer_count,
                "active_transfers": active_transfers,
                "total_bytes": bytes,
                "uptime_seconds": 0,
                "enabled": bridge.enabled,
                "running": runtime.bridge_running,
                "configUpdates": runtime.bridge_config_updates,
                "host": serde_json::Value::Null,
                "port": serde_json::Value::Null,
                "endpoint_configured": bridge.endpoint_configured(),
            });
            drop(runtime);
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/admin/stats") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let total_bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let active_sessions = transfers
                .entries
                .iter()
                .filter(|entry| is_queued_or_active_transfer_status(&entry.status))
                .count();
            let total_requests = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let uptime = runtime
                    .bridge_started_at
                    .map(|started| {
                        format_timespan_hms(
                            i64::try_from(unix_timestamp().saturating_sub(started))
                                .unwrap_or(i64::MAX),
                        )
                    })
                    .unwrap_or_else(|| "00:00:00".to_owned());
                let json = serde_json::json!({
                    "totalConnections": runtime.bridge_total_connections,
                    "currentConnections": runtime.bridge_active_clients.len(),
                    "totalSearches": runtime.bridge_total_searches,
                    "totalDownloads": runtime.bridge_total_downloads,
                    "totalRoomJoins": runtime.bridge_total_room_joins,
                    "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                    "uptime": uptime,
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            // Local HTTP transfer activity remains separate from the
            // protocol bridge counters below.
            let json = serde_json::json!({
                "totalConnections": runtime.bridge_total_connections,
                "currentConnections": runtime.bridge_active_clients.len(),
                "totalSearches": runtime.bridge_total_searches,
                "totalDownloads": runtime.bridge_total_downloads,
                "totalRoomJoins": runtime.bridge_total_room_joins,
                "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                "uptime": runtime.bridge_started_at.map(|started| format_timespan_hms(
                    i64::try_from(unix_timestamp().saturating_sub(started)).unwrap_or(i64::MAX)
                )).unwrap_or_else(|| "00:00:00".to_owned()),
                "total_requests": total_requests,
                "total_bytes": total_bytes,
                "active_sessions": active_sessions,
                "enabled": bridge.enabled,
                "running": runtime.bridge_running,
                "configUpdates": runtime.bridge_config_updates,
            });
            drop(runtime);
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/status") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let transfer_count = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let started_at = runtime
                    .bridge_started_at
                    .map(bridge_started_at_string)
                    .unwrap_or_else(|| "0001-01-01T00:00:00+00:00".to_owned());
                let json = serde_json::json!({
                    "isHealthy": runtime.bridge_running,
                    "version": "1.0.0-proxy",
                    "activeConnections": runtime.bridge_active_clients.len(),
                    "startedAt": started_at,
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            let uptime_seconds = runtime
                .bridge_started_at
                .map(|started| unix_timestamp().saturating_sub(started))
                .unwrap_or(0);
            let json = format!(
                 "{{\"status\":\"{}\",\"version\":\"1.0.0\",\"uptime_seconds\":{},\"enabled\":{},\"configured\":{},\"running\":{},\"configUpdates\":{},\"host\":\"{}\",\"port\":{},\"endpoint_configured\":{},\"transfers\":{},\"next_action\":\"{}\"}}",
                 if runtime.bridge_running { "running" } else if bridge.enabled { "configured" } else { "disabled" },
                 uptime_seconds,
                 bridge.enabled,
                 bridge.enabled && bridge.endpoint_configured(),
                 runtime.bridge_running,
                 runtime.bridge_config_updates,
                 bridge.bind_address,
                 bridge.port,
                 bridge.endpoint_configured(),
                 transfer_count,
                 if runtime.bridge_running {
                     "accept bridge traffic"
                 } else if bridge.enabled {
                     "start bridge service"
                 } else {
                     "enable bridge integration"
                 }
             );
            drop(runtime);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/bridge/transfer/") => {
            Ok(bridge_transfer_progress_response(path, state).await)
        }

        ("POST", "/api/bridge/start") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                return Ok(routing::ok_response(
                    serde_json::json!({"status": "started"}).to_string(),
                ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.set_bridge_running(true, bridge.enabled);
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "next_action".to_owned(),
                        serde_json::json!(if bridge.enabled {
                            "accept bridge traffic"
                        } else {
                            "enable bridge integration"
                        }),
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("POST", "/api/bridge/stop") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                return Ok(routing::ok_response(
                    serde_json::json!({"status": "stopped"}).to_string(),
                ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime
                    .set_bridge_running(false, bridge.enabled)
                    .to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/bridge/admin/config") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                if body.trim().is_empty()
                    || serde_json::from_str::<serde_json::Value>(body).is_err()
                {
                    return Ok(routing::bad_request_response("Request is required"));
                }
                return Ok(routing::ok_response(
                     serde_json::json!({
                         "message": "Configuration updated. Restart bridge service to apply changes.",
                         "restart_required": true,
                     })
                     .to_string(),
                 ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let accepted_keys = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|value| {
                    value
                        .as_object()
                        .map(|object| object.keys().cloned().collect::<Vec<_>>())
                })
                .unwrap_or_default();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.record_bridge_config_update(bridge.enabled, accepted_keys);
                if route.path.starts_with("/api/v0/") {
                    value["message"] = serde_json::json!(
                        "Configuration updated. Restart bridge service to apply changes."
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("POST" | "PUT", path)
            if path.starts_with("/api/collections/") && path.contains("/items/reorder") =>
        {
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
            if route.path.starts_with("/api/v0/")
                && matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                )
            {
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
