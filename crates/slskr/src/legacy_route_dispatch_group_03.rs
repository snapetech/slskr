use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_03(
    context: &super::LegacyRouteDispatchContext<'_, '_>,
) -> Result<HttpResponse, String> {
    let super::LegacyRouteDispatchContext {
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
    let _ = (
        authorization,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    );
    match (method, normalized_path.as_str()) {
        ("GET", "/api/transfers/speeds") => {
            let transfers = state.transfers.read().await;
            let json = controller_transfer_speeds_json(&transfers);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        // USER PROFILE ENDPOINTS
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/info") => {
            let Some(username) = user_route_username(path, "/info") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_user_read_failure_response(state, route.path, &username, false).await
            {
                return Ok(response);
            }
            let users = state.users.read().await;
            if let Some(record) = users.records.iter().find(|u| u.username == username) {
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "description": "",
                        "hasFreeUploadSlot": true,
                        "hasPicture": false,
                        "picture": null,
                        "queueLength": 0,
                        "uploadSlots": 0,
                    })
                    .to_string()
                } else {
                    record.controller_info_json().to_string()
                };
                drop(users);
                Ok(routing::ok_response(json))
            } else {
                drop(users);
                if state.config.controller_profile == ControllerProfile::Legacy {
                    return Ok(routing::not_found_response());
                }
                let record = UserRecord {
                    username,
                    watched: false,
                    status: None,
                    privileged: false,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                };
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "description": "",
                        "hasFreeUploadSlot": true,
                        "hasPicture": false,
                        "picture": null,
                        "queueLength": 0,
                        "uploadSlots": 0,
                    })
                    .to_string()
                } else {
                    record.controller_info_json().to_string()
                };
                Ok(routing::ok_response(json))
            }
        }

        ("POST", path) if path.starts_with("/api/users/") && path.ends_with("/directory") => {
            let Some(username) = user_route_username(path, "/directory") else {
                return Ok(routing::not_found_response());
            };
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if route.path.starts_with("/api/v0/") && directory.trim().is_empty() {
                // UsersController validates the bound request before it
                // checks whether the Soulseek connection is ready.
                return Ok(routing::bad_request_response("directory is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let session_command_permit = if route.path.starts_with("/api/v0/") {
                match state.session_commands.reserve().await {
                    Ok(permit) => Some(permit),
                    Err(_) => {
                        return Ok(routing::service_unavailable_response(
                            "session manager is not running",
                        ));
                    }
                }
            } else {
                None
            };
            let browse = state.browse.read().await;
            let entries = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.entries.as_slice())
                .unwrap_or(&[]);
            let json = controller_user_directories_json(&directory, entries, route.query);
            drop(browse);
            if let Some(session_command_permit) = session_command_permit {
                session_command_permit.send(SessionCommand::BrowseFolder {
                    username: username.to_owned(),
                    folder: directory,
                });
            }
            Ok(routing::ok_response(json))
        }

        // USER STATUS ENDPOINTS
        ("GET", path)
            if path.starts_with("/api/users/")
                && path.ends_with("/status")
                && user_route_username(path, "/status").is_some() =>
        {
            let username = user_route_username(path, "/status").expect("guarded user status path");
            if let Some(response) =
                controller_user_read_failure_response(state, route.path, &username, false).await
            {
                return Ok(response);
            }
            let users = state.users.read().await;
            if let Some(record) = users.records.iter().find(|u| u.username == username) {
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    let status = match record.status.as_deref() {
                        Some("online") | Some("Online") => "Online",
                        Some("away") | Some("Away") => "Away",
                        _ => "Offline",
                    };
                    serde_json::json!({
                        "isPrivileged": record.privileged,
                        "presence": status,
                    })
                    .to_string()
                } else {
                    record.controller_status_json().to_string()
                };
                drop(users);
                Ok(routing::ok_response(json))
            } else {
                drop(users);
                if state.config.controller_profile == ControllerProfile::Legacy {
                    return Ok(routing::not_found_response());
                }
                let record = UserRecord {
                    username: username.to_owned(),
                    watched: false,
                    status: None,
                    privileged: false,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                };
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "isPrivileged": false,
                        "presence": "Offline",
                    })
                    .to_string()
                } else {
                    record.controller_status_json().to_string()
                };
                Ok(routing::ok_response(json))
            }
        }

        ("GET", "/api/users/groups") => {
            if state.config.controller_profile != ControllerProfile::Native {
                return Ok(HttpResponse {
                    status: "404 Not Found",
                    content_type: "",
                    body: String::new(),
                });
            }
            let mut usernames = Vec::<String>::new();
            for (key, username) in route.query.map(query_params).unwrap_or_default() {
                let username = username.trim();
                if !key.eq_ignore_ascii_case("usernames") || username.is_empty() {
                    continue;
                }
                if username.len() > MAX_USER_USERNAME_BYTES {
                    return Ok(routing::bad_request_response("username is too long"));
                }
                if usernames
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(username))
                {
                    continue;
                }
                if usernames.len() == MAX_USER_GROUP_BATCH {
                    return Ok(routing::bad_request_response(
                        "a maximum of 100 usernames is allowed",
                    ));
                }
                usernames.push(username.to_owned());
            }

            let mut groups = BTreeMap::new();
            for username in usernames {
                let group = effective_transfer_group(state, &username).await;
                groups.insert(username, group);
            }
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::to_string(&groups).unwrap_or_else(|_| "{}".to_owned()),
            })
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/group") => {
            if state.config.controller_profile != ControllerProfile::Native {
                return Ok(HttpResponse {
                    status: "404 Not Found",
                    content_type: "",
                    body: String::new(),
                });
            }
            let Some(username) = user_route_username(path, "/group") else {
                return Ok(routing::not_found_response());
            };
            let group = effective_transfer_group(state, &username).await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::to_string(&group).unwrap_or_else(|_| "\"default\"".to_owned()),
            })
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/endpoint") => {
            let Some(username) = user_route_username(path, "/endpoint") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_user_read_failure_response(state, route.path, &username, false).await
            {
                return Ok(response);
            }
            if test_user_endpoint_peer_address(state, &username).is_none()
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::not_found_response());
            }
            let address = if let Some(address) = test_user_endpoint_peer_address(state, &username) {
                address
            } else {
                match request_peer_endpoint(state, &username).await {
                    Ok(address) => address,
                    Err(_) => return Ok(routing::not_found_response()),
                }
            };
            let body = if state.config.controller_profile == ControllerProfile::Legacy {
                serde_json::json!({
                    "addressFamily": "IPv4",
                    "address": address.ip.to_string(),
                    "port": address.port,
                })
                .to_string()
            } else {
                serde_json::json!({
                    "username": username,
                    "addressFamily": "IPv4",
                    "address": address.ip.to_string(),
                    "port": address.port,
                })
                .to_string()
            };
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/soulseek/users/similar") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let body = if route.path.starts_with("/api/v0/") {
                mesh.versioned_similar_users_json(&users)
            } else {
                mesh.users_json(&users)
            };
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(body))
        }

        ("GET", path)
            if path.starts_with("/api/soulseek/users/") && path.ends_with("/interests") =>
        {
            let Some(username) = path_segment_between(path, "/api/soulseek/users/", "/interests")
            else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            if username.trim().is_empty() || username.len() > MAX_USER_USERNAME_BYTES {
                return Ok(routing::bad_request_response("username is required"));
            }
            if state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "server session is disconnected",
                ));
            }
            let (sender, receiver) =
                oneshot::channel::<Result<slskr_client::protocol::server::UserInterests, String>>();
            let key = username.to_ascii_lowercase();
            {
                let mut pending = state.pending_user_interests.write().await;
                if pending.values().map(Vec::len).sum::<usize>() >= 128 {
                    return Ok(routing::service_unavailable_response(
                        "user-interest request capacity is full",
                    ));
                }
                pending.entry(key).or_default().push(sender);
            }
            if let Err(error) = send_session_command(
                state,
                SessionCommand::RequestUserInterests(username.clone()),
            )
            .await
            {
                state
                    .pending_user_interests
                    .write()
                    .await
                    .remove(&username.to_ascii_lowercase());
                return Ok(routing::service_unavailable_response(&error));
            }
            let interests = match time::timeout(
                state.config.soulseek_connection.timeout_inactivity,
                receiver,
            )
            .await
            {
                Ok(Ok(Ok(interests))) => interests,
                Ok(Ok(Err(error))) => {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(Err(_)) => {
                    return Ok(routing::service_unavailable_response(
                        "user-interest request was cancelled",
                    ));
                }
                Err(_) => {
                    state
                        .pending_user_interests
                        .write()
                        .await
                        .remove(&username.to_ascii_lowercase());
                    return Ok(routing::service_unavailable_response(
                        "user-interest request timed out",
                    ));
                }
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "username": interests.username,
                    "liked": interests.liked,
                    "hated": interests.hated,
                })
                .to_string(),
            ))
        }

        ("DELETE", "/api/searches") => {
            let _search_persistence = state.search_persistence_lock.lock().await;
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let cleared_count = searches.records.len();
            searches.records.clear();
            let mutated_searches = searches.clone();
            drop(searches);
            if let Err(error) = clear_persisted_searches(state, &_search_persistence).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            let json = if route.path.starts_with("/api/v0/") {
                format!("{{\"deleted\":{cleared_count}}}")
            } else {
                format!("{{\"cleared\":{cleared_count}}}")
            };
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/searches/") && path.ends_with("/responses") => {
            let Some(id) = path
                .strip_prefix("/api/searches/")
                .and_then(|value| value.strip_suffix("/responses"))
                .filter(|value| !value.is_empty() && !value.contains('/'))
            else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_search_responses_read_failure_response(state, route.path, id).await
            {
                return Ok(response);
            }
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(id) {
                let json = record.controller_responses_json_with_query(route.query);
                drop(searches);
                Ok(routing::ok_response(json))
            } else {
                drop(searches);
                Ok(routing::ok_response("[]".to_string()))
            }
        }

        ("GET", path) if path.starts_with("/api/searches/") => {
            let Some(id) = path_segment_after(path, "/api/searches/") else {
                return Ok(routing::not_found_response());
            };
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(id) {
                let json = record.json_with_query(route.query);
                drop(searches);
                Ok(routing::ok_response(json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("DELETE", path)
            if path.starts_with("/api/searches/")
                || route.normalized_path.starts_with("/api/v0/searches/") =>
        {
            let Some(token_str) = path_segment_after(path, "/api/searches/")
                .or_else(|| path_segment_after(route.normalized_path, "/api/v0/searches/"))
            else {
                return Ok(routing::not_found_response());
            };
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let removed = searches.remove_by_identifier(token_str);
            let mutated_searches = searches.clone();
            drop(searches);
            if let Some(record) = removed.as_ref() {
                if let Err(error) = delete_persisted_search(state, record).await {
                    rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                        .await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                publish_search_hub_event(state, "delete", record);
            }
            if route.path.starts_with("/api/v0/")
                && matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                )
                && removed.is_some()
            {
                Ok(routing::no_content_response())
            } else if route.path.starts_with("/api/v0/") && removed.is_none() {
                Ok(routing::not_found_response())
            } else {
                Ok(routing::ok_response("{}".to_string()))
            }
        }

        // MESSAGE ENDPOINTS
        ("POST", "/api/messages") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let message_body = match extract_json_string_field(body, "body") {
                Some(b) => b,
                None => return Ok(routing::bad_request_response("body is required")),
            };

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "outbound", message_body.clone());
            let mutated = messages.clone();
            let message_id = record.id;
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_message_persistence);
            session_command_permit.send(SessionCommand::MessageUser {
                username: username.clone(),
                body: message_body.clone(),
            });
            record_event(
                state,
                "message.sent",
                username.clone(),
                Some(format!("id={message_id}")),
            )
            .await;
            // Dispatch webhook for message.sent event
            let webhook_data = serde_json::json!({
                "message_id": message_id,
                "username": username.clone(),
                "body": message_body.clone(),
                "direction": "outbound",
            });
            let correlation_id = format!("message_{}", message_id);

            dispatch_webhook_event(
                state,
                correlation_id,
                webhooks::WebhookEvent::MessageSent,
                webhook_data,
            )
            .await;

            Ok(routing::created_response(record.json()))
        }

        ("POST", "/api/messages/inbound") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let message_body = match extract_json_string_field(body, "body") {
                Some(b) => b,
                None => return Ok(routing::bad_request_response("body is required")),
            };

            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "inbound", message_body.clone());
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_message_persistence);
            record_event(
                state,
                "message.received",
                "messages",
                Some(format!("id={}", record.id)),
            )
            .await;

            Ok(routing::created_response(record.json()))
        }

        ("POST", _path) if message_ack_path(normalized_path.as_str()).is_some() => {
            let Some(id) = message_ack_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let Ok(protocol_id) = u32::try_from(id) else {
                return Ok(routing::bad_request_response(
                    "message id exceeds u32 range",
                ));
            };
            if !state
                .messages
                .read()
                .await
                .records
                .iter()
                .any(|message| message.id == id)
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();

            if let Some(record) = messages.ack(id) {
                let mutated = messages.clone();
                let username = record.username.clone();
                let json_response = record.json();
                drop(messages);
                if let Err(error) = persist_message_ack_checked(state, id).await {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_message_persistence);
                record_event(state, "message.acked", username, Some(format!("id={id}"))).await;

                session_command_permit.send(SessionCommand::MessageAcked { id: protocol_id });

                Ok(routing::ok_response(json_response))
            } else {
                drop(messages);
                Ok(routing::not_found_response())
            }
        }

        ("PUT", _path) if message_ack_path(normalized_path.as_str()).is_some() => {
            let Some(id) = message_ack_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let Ok(protocol_id) = u32::try_from(id) else {
                return Ok(routing::bad_request_response(
                    "message id exceeds u32 range",
                ));
            };
            if !state
                .messages
                .read()
                .await
                .records
                .iter()
                .any(|message| message.id == id)
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();

            if let Some(record) = messages.ack(id) {
                let mutated = messages.clone();
                let username = record.username.clone();
                let json_response = record.json();
                drop(messages);
                if let Err(error) = persist_message_ack_checked(state, id).await {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_message_persistence);
                record_event(state, "message.acked", username, Some(format!("id={id}"))).await;

                session_command_permit.send(SessionCommand::MessageAcked { id: protocol_id });

                Ok(routing::ok_response(json_response))
            } else {
                drop(messages);
                Ok(routing::not_found_response())
            }
        }

        ("GET", _path) if messages_user_path(normalized_path.as_str()).is_some() => {
            let Some(username) = messages_user_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let messages = state.messages.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: messages.json_for_user(username, route.query),
            })
        }

        // ROOM ENDPOINTS
        ("POST", "/api/rooms/refresh") => {
            if send_session_command(state, SessionCommand::RefreshRooms)
                .await
                .is_err()
            {
                return Ok(routing::service_unavailable_response(
                    "session manager is not running",
                ));
            }
            Ok(routing::accepted_response("{}".to_string()))
        }

        ("POST", _path) if room_join_path(normalized_path.as_str()).is_some() => {
            let Some(room_name) = room_join_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let _room_persistence = state.room_persistence_lock.lock().await;
            let session_connected = state.session.read().await.state == "connected";
            let mut rooms = state.rooms.write().await;
            if session_connected {
                if let Some(existing) = rooms
                    .records
                    .iter()
                    .find(|record| {
                        record.name == bounded_room_name(room_name)
                            && record.joined
                            && record.last_error.is_none()
                    })
                    .cloned()
                {
                    drop(rooms);
                    return Ok(routing::ok_response(
                        existing.controller_room_json().to_string(),
                    ));
                }
            }
            let previous = rooms.clone();
            let Some(record) = rooms.join(room_name.to_string()) else {
                return Ok(routing::service_unavailable_response(
                    "room capacity is full",
                ));
            };
            let response = record.json();
            let should_persist = !route.path.starts_with("/api/v0/")
                || state.config.controller_profile == ControllerProfile::Legacy;
            let mutated = rooms.clone();
            drop(rooms);
            if should_persist {
                if let Err(error) = persist_room_join_checked(state, room_name).await {
                    let mut rooms = state.rooms.write().await;
                    if *rooms == mutated {
                        *rooms = previous;
                    }
                    drop(rooms);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            drop(_room_persistence);
            record_event(state, "room.joined", room_name.to_string(), None).await;

            send_room_join_if_connected(state, room_name.to_string()).await;

            Ok(routing::created_response(response))
        }

        ("DELETE", _path) if room_join_path(normalized_path.as_str()).is_some() => {
            let Some(room_name) = room_join_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let _room_persistence = state.room_persistence_lock.lock().await;
            let mut rooms = state.rooms.write().await;
            let previous = rooms.clone();

            if let Some(record) = rooms.leave(room_name) {
                let json_response = record.json();
                let mutated = rooms.clone();
                drop(rooms);
                if let Err(error) = persist_room_leave_checked(state, room_name).await {
                    let mut rooms = state.rooms.write().await;
                    if *rooms == mutated {
                        *rooms = previous;
                    }
                    drop(rooms);
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_room_persistence);
                record_event(state, "room.left", room_name.to_string(), None).await;

                send_room_leave_if_connected(state, room_name.to_string()).await;

                Ok(routing::ok_response(json_response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("POST", _path) if room_messages_path(normalized_path.as_str()).is_some() => {
            let Some(room_name) = room_messages_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let username = extract_json_string_field(body, "username")
                .unwrap_or_else(|| "unknown".to_string());
            let message_body = extract_json_string_field(body, "body")
                .or_else(|| json_body_string(body))
                .unwrap_or_default();

            if !state
                .rooms
                .read()
                .await
                .records
                .iter()
                .any(|room| room.name == room_name)
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut rooms = state.rooms.write().await;
            if let Some(record) =
                rooms.add_message(room_name, username.clone(), message_body.clone())
            {
                let json_response = record.json();
                drop(rooms);
                session_command_permit.send(SessionCommand::SayRoom {
                    room: room_name.to_string(),
                    body: message_body,
                });
                record_event(
                    state,
                    "room.message",
                    room_name.to_string(),
                    Some(format!("username={username}")),
                )
                .await;

                Ok(routing::ok_response(json_response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("GET", "/api/rooms/available") => {
            if state.config.controller_profile == ControllerProfile::Legacy
                && route.path.starts_with("/api/v0/")
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::internal_server_error_response(
                    "failed to retrieve available rooms",
                ));
            }
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            let rooms = state.rooms.read().await;
            let json = rooms.controller_available_json();
            drop(rooms);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/rooms/activity") => {
            let session = state.session.read().await;
            let local_username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            drop(session);
            let rooms = state.rooms.read().await;
            let json = rooms.activity_json(&local_username);
            drop(rooms);
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/users")
                && joined_room_subresource(path, "/users").is_some() =>
        {
            let room_name =
                joined_room_subresource(path, "/users").expect("guarded joined-room users path");
            let session = state.session.read().await;
            let local_username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            drop(session);
            let rooms = state.rooms.read().await;
            if let Some(room) = rooms.records.iter().find(|r| r.name == room_name) {
                // Matches the oracle's real GetUsersByRoomName: the room's
                // real roster (from the server's JoinedRoom snapshot), not
                // a hardcoded empty list.
                let json = serde_json::Value::Array(
                    room.roster
                        .iter()
                        .map(|user| user.controller_json(&local_username))
                        .collect(),
                )
                .to_string();
                drop(rooms);
                Ok(routing::ok_response(json))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("GET", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/messages")
                && joined_room_subresource(path, "/messages").is_some() =>
        {
            let room_name = joined_room_subresource(path, "/messages")
                .expect("guarded joined-room messages path");
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let rooms = state.rooms.read().await;
            if let Some(room) = rooms.records.iter().find(|r| r.name == room_name) {
                let messages = room
                    .messages
                    .iter()
                    .filter(|message| since.is_none_or(|since| message.created_at_ms > since))
                    .map(|message| message.controller_json(&room.name))
                    .collect::<Vec<_>>();
                let json = serde_json::Value::Array(messages).to_string();
                drop(rooms);
                Ok(routing::ok_response(json))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        // USER ENDPOINTS
        ("POST", "/api/users/watch") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            {
                let users = state.users.read().await;
                let bounded_username = bounded_user_username(&username);
                if users.records.len() >= users.max_records
                    && !users
                        .records
                        .iter()
                        .any(|record| record.username == bounded_username)
                {
                    return Ok(routing::service_unavailable_response(
                        "user watch capacity is full",
                    ));
                }
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _user_persistence = state.user_persistence_lock.lock().await;
            let mut users = state.users.write().await;
            let previous_updated_at = users.updated_at;
            let bounded_username = bounded_user_username(&username);
            let previous_record = users
                .records
                .iter()
                .find(|record| record.username == bounded_username)
                .cloned();
            let Some(record) = users.watch(username.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "user watch capacity is full",
                ));
            };
            drop(users);

            if let Err(error) = persist_user_projection(state, &record).await {
                let mut users = state.users.write().await;
                match previous_record {
                    Some(previous) => {
                        if let Some(current) = users.records.iter_mut().find(|current| {
                            current.username == bounded_username && **current == record
                        }) {
                            *current = previous;
                        }
                    }
                    None => users.records.retain(|current| {
                        current.username != bounded_username || *current != record
                    }),
                }
                users.updated_at = users
                    .records
                    .iter()
                    .map(|current| current.updated_at)
                    .max()
                    .unwrap_or(previous_updated_at);
                drop(users);
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_user_persistence);
            session_command_permit.send(SessionCommand::WatchUser(username));

            Ok(routing::created_response(record.json()))
        }

        ("DELETE", _path) if user_watch_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_watch_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            if !state
                .users
                .read()
                .await
                .records
                .iter()
                .any(|record| record.username == bounded_user_username(username))
            {
                return Ok(routing::not_found_response());
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _user_persistence = state.user_persistence_lock.lock().await;
            let mut users = state.users.write().await;
            let previous_updated_at = users.updated_at;
            let previous_record = users
                .records
                .iter()
                .find(|record| record.username == bounded_user_username(username))
                .cloned();

            if let Some(record) = users.unwatch(username) {
                drop(users);

                if let Err(error) = persist_user_projection(state, &record).await {
                    let mut users = state.users.write().await;
                    if let Some(previous) = previous_record {
                        if let Some(current) = users.records.iter_mut().find(|current| {
                            current.username == previous.username && **current == record
                        }) {
                            *current = previous;
                        }
                    }
                    users.updated_at = users
                        .records
                        .iter()
                        .map(|current| current.updated_at)
                        .max()
                        .unwrap_or(previous_updated_at);
                    drop(users);
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_user_persistence);
                session_command_permit.send(SessionCommand::UnwatchUser(username.to_string()));

                Ok(routing::ok_response(record.json()))
            } else {
                drop(users);
                Ok(routing::not_found_response())
            }
        }

        ("POST", _path) if user_stats_request_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_stats_request_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            if send_session_command(
                state,
                SessionCommand::RequestUserStats(username.to_string()),
            )
            .await
            .is_err()
            {
                return Ok(routing::service_unavailable_response(
                    "session manager is not running",
                ));
            }
            Ok(routing::accepted_response(format!(
                "{{\"username\":\"{}\"}}",
                json_escape(username)
            )))
        }

        ("POST", _path) if user_browse_request_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_browse_request_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };

            let connected = {
                let session = state.session.read().await;
                session.state == "connected"
            };
            if !connected {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            if state.config.controller_profile == ControllerProfile::Native
                && !state.soulseek_safety.try_consume_browse("compatibility")
            {
                return Ok(HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "application/json; charset=utf-8",
                    body: serde_json::json!({
                        "error": "Browse rate limit exceeded. See Soulseek safety configuration."
                    })
                    .to_string(),
                });
            }

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.request(username.to_string()) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);

            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_browse_persistence);
            session_command_permit.send(SessionCommand::BrowseUser(username.to_string()));

            Ok(routing::accepted_response(record.json()))
        }

        ("POST", _path) if user_browse_folder_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_browse_folder_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let folder = extract_json_string_field(body, "folder").unwrap_or_default();

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.request_folder(username.to_string(), folder.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);

            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_browse_persistence);
            session_command_permit.send(SessionCommand::BrowseFolder {
                username: username.to_string(),
                folder,
            });

            Ok(routing::accepted_response(record.json()))
        }

        ("POST", _path) if user_browse_fail_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_browse_fail_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let reason = extract_json_string_field(body, "reason").unwrap_or_default();

            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.fail(username.to_owned(), reason.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);
            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_browse_persistence);

            Ok(routing::ok_response(format!(
                "{{\"username\":\"{}\",\"status\":\"failed\",\"reason\":\"{}\"}}",
                json_escape(username),
                json_escape(&reason)
            )))
        }

        ("POST", _path) if user_browse_cancel_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_browse_cancel_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let reason = extract_json_string_field(body, "reason")
                .unwrap_or_else(|| "cancelled by client".to_owned());

            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.cancel(username, reason) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            let body = record.json();
            drop(browse);
            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_browse_persistence);

            Ok(routing::ok_response(body))
        }

        // BROWSE-RESPONSE ENDPOINT
        ("POST", "/api/browse-responses") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let complete = extract_json_bool_field(body, "complete").unwrap_or(true);

            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload) => payload,
                Err(_) => return Ok(routing::bad_request_response("invalid JSON body")),
            };
            if browse_response_exceeds_wire_limits(&payload) {
                return Ok(routing::bad_request_response(
                    "browse response exceeds wire entry limits",
                ));
            }

            let mut entries = Vec::new();
            if let Some(array) = payload.get("entries").and_then(serde_json::Value::as_array) {
                entries.extend(
                    array
                        .iter()
                        .filter_map(|entry| BrowseEntry::from_json_file(entry, None)),
                );
            }

            if let Some(directories) = payload
                .get("directories")
                .and_then(serde_json::Value::as_array)
            {
                for directory in directories {
                    let directory_name = directory
                        .get("name")
                        .or_else(|| directory.get("directory"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    if let Some(files) =
                        directory.get("files").and_then(serde_json::Value::as_array)
                    {
                        entries.extend(files.iter().filter_map(|file| {
                            BrowseEntry::from_json_file(file, Some(directory_name))
                        }));
                    }
                }
            }

            // Fallback for single entry format (backward compatibility)
            if entries.is_empty() {
                if let Some(entry) = BrowseEntry::from_json_file(&payload, None) {
                    entries.push(entry);
                }
            }

            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let previous = browse.clone();
            let Some(record) = browse.add_entries(username, entries, complete) else {
                return Ok(routing::service_unavailable_response(
                    "browse record capacity is full",
                ));
            };
            let mutated = browse.clone();
            drop(browse);
            if let Err(error) = persist_browse_record_checked(state, &record).await {
                rollback_browse_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_browse_persistence);

            Ok(routing::ok_response(record.json()))
        }
        ("GET", "/api/browse") | ("GET", "/api/v0/browse") => {
            let browse = state.browse.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: browse.json(route.query),
            })
        }
        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
