async fn route_dispatch_group_3_people_browse(
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
        ("POST", _path) if room_messages_path(normalized_path).is_some() => {
            let Some(room_name) = room_messages_path(normalized_path) else {
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

        ("DELETE", _path) if user_watch_path(normalized_path).is_some() => {
            let Some(username) = user_watch_path(normalized_path) else {
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

        ("POST", _path) if user_stats_request_path(normalized_path).is_some() => {
            let Some(username) = user_stats_request_path(normalized_path) else {
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

        ("POST", _path) if user_browse_request_path(normalized_path).is_some() => {
            let Some(username) = user_browse_request_path(normalized_path) else {
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

        ("POST", _path) if user_browse_folder_path(normalized_path).is_some() => {
            let Some(username) = user_browse_folder_path(normalized_path) else {
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

        ("POST", _path) if user_browse_fail_path(normalized_path).is_some() => {
            let Some(username) = user_browse_fail_path(normalized_path) else {
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

        ("POST", _path) if user_browse_cancel_path(normalized_path).is_some() => {
            let Some(username) = user_browse_cancel_path(normalized_path) else {
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
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/browse") => {
            if let Some(username) = user_route_username(path, "/browse") {
                if let Some(response) =
                    controller_user_read_failure_response(state, route.path, &username, true).await
                {
                    return Ok(response);
                }
                let browse = state.browse.read().await;
                let record = browse
                    .records
                    .iter()
                    .find(|record| record.username == username);
                if state.config.controller_profile == ControllerProfile::Native && record.is_none()
                {
                    drop(browse);
                    let session_state = state.session.read().await.state;
                    if session_state != "connected" {
                        return Ok(HttpResponse {
                            status: "503 Service Unavailable",
                            content_type: "application/json",
                            body: serde_json::to_string("Soulseek server connection is not ready")
                                .unwrap_or_else(|_| {
                                    "\"Soulseek server connection is not ready\"".to_owned()
                                }),
                        });
                    }
                    return Ok(routing::not_found_response());
                }
                if state.config.controller_profile == ControllerProfile::Legacy && record.is_none()
                {
                    drop(browse);
                    let session_state = state.session.read().await.state;
                    if session_state != "connected" {
                        let display_state = match session_state {
                            "connecting" => "Connecting",
                            "disconnecting" => "Disconnecting",
                            _ => "Disconnected",
                        };
                        let message = format!(
                            "The server connection must be connected and logged in to browse (currently: {display_state})"
                        );
                        return Ok(HttpResponse {
                            status: "500 Internal Server Error",
                            content_type: "application/json",
                            body: serde_json::to_string(&message)
                                .unwrap_or_else(|_| "\"browse failed\"".to_owned()),
                        });
                    }
                    return Ok(routing::not_found_response());
                }
                let entries = record
                    .map(|record| record.entries.as_slice())
                    .unwrap_or(&[]);
                let body = controller_user_root_json(entries, route.query);
                drop(browse);
                Ok(routing::ok_response(body))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // GET browse requests list
        ("GET", "/api/browse/requests") => {
            let browse = state.browse.read().await;
            let requests = browse
                .records
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "username": r.username,
                        "status": r.status,
                        "requested_at": r.requested_at,
                        "updated_at": r.updated_at,
                    })
                })
                .collect::<Vec<_>>();
            drop(browse);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::to_string(
                    &serde_json::json!({"requests": requests, "count": requests.len()}),
                )
                .unwrap_or_else(|_| "{}".to_string()),
            })
        }
        // WEBHOOK MANAGEMENT ROUTES
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
