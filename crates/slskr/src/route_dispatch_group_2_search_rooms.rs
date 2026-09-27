async fn route_dispatch_group_2_search_rooms(
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
            let cleared_records = searches.records.clone();
            let cleared_count = searches.records.len();
            searches.records.clear();
            let mutated_searches = searches.clone();
            drop(searches);
            if let Err(error) = clear_persisted_searches(state, &_search_persistence).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            for record in &cleared_records {
                publish_search_hub_event(state, "delete", record);
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

        ("POST", _path) if message_ack_path(normalized_path).is_some() => {
            let Some(id) = message_ack_path(normalized_path) else {
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

        ("PUT", _path) if message_ack_path(normalized_path).is_some() => {
            let Some(id) = message_ack_path(normalized_path) else {
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

        ("GET", _path) if messages_user_path(normalized_path).is_some() => {
            let Some(username) = messages_user_path(normalized_path) else {
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

        ("POST", _path) if room_join_path(normalized_path).is_some() => {
            let Some(room_name) = room_join_path(normalized_path) else {
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
            let mutated = rooms.clone();
            drop(rooms);
            if let Err(error) = persist_room_join_checked(state, room_name).await {
                let mut rooms = state.rooms.write().await;
                if *rooms == mutated {
                    *rooms = previous;
                }
                drop(rooms);
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_room_persistence);
            record_event(state, "room.joined", room_name.to_string(), None).await;

            send_room_join_if_connected(state, room_name.to_string()).await;

            Ok(routing::created_response(response))
        }

        ("DELETE", _path) if room_join_path(normalized_path).is_some() => {
            let Some(room_name) = room_join_path(normalized_path) else {
                return Ok(routing::not_found_response());
            };
            let _room_persistence = state.room_persistence_lock.lock().await;
            let mut rooms = state.rooms.write().await;
            let previous = rooms.clone();

            if let Some(record) = rooms.leave(room_name) {
                let response = record.json();
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

                Ok(routing::ok_response(response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
