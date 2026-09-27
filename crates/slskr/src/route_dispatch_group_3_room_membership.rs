async fn route_dispatch_group_3_room_membership(
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
        ("GET", "/api/rooms/joined") => {
            let rooms = state.rooms.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: rooms.joined_names_json(),
            })
        }
        ("POST", "/api/rooms/joined") => {
            let Some(room_name) = extract_json_string_field(body, "room")
                .or_else(|| extract_json_string_field(body, "name"))
                .or_else(|| json_body_string(body))
                .filter(|room| !room.trim().is_empty())
            else {
                return Ok(if route.path.starts_with("/api/v0/") {
                    rooms_controller_value_bad_request_response("roomName is required")
                } else {
                    routing::bad_request_response("room is required")
                });
            };
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek is reconnecting; try again shortly.",
                ));
            }
            let _room_persistence = state.room_persistence_lock.lock().await;
            let mut rooms = state.rooms.write().await;
            if route.path.starts_with("/api/v0/")
                && rooms.records.iter().any(|record| {
                    record.name == bounded_room_name(&room_name)
                        && record.joined
                        && record.last_error.is_none()
                })
            {
                drop(rooms);
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "",
                    body: String::new(),
                });
            }
            let previous = rooms.clone();
            let Some(record) = rooms.join(room_name.to_string()) else {
                return Ok(routing::service_unavailable_response(
                    "room capacity is full",
                ));
            };
            let body = record.controller_room_json().to_string();
            let should_persist = !route.path.starts_with("/api/v0/")
                || state.config.controller_profile == ControllerProfile::Legacy;
            let mutated = rooms.clone();
            drop(rooms);
            if should_persist {
                if let Err(error) = persist_room_join_checked(state, &room_name).await {
                    let mut rooms = state.rooms.write().await;
                    if *rooms == mutated {
                        *rooms = previous;
                    }
                    drop(rooms);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            drop(_room_persistence);
            record_event(state, "room.joined", room_name.clone(), None).await;

            send_room_join_if_connected(state, room_name).await;

            Ok(routing::created_response(body))
        }
        ("GET", path)
            if path.starts_with("/api/rooms/joined/") && path.matches('/').count() == 4 =>
        {
            let room_name = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let rooms = state.rooms.read().await;
            let response = rooms
                .records
                .iter()
                .find(|r| {
                    r.name == room_name
                        && (state.config.controller_profile != ControllerProfile::Legacy
                            || r.joined)
                })
                .map(|room| routing::ok_response(room.controller_room_json().to_string()))
                .unwrap_or_else(routing::not_found_response);
            drop(rooms);
            Ok(response)
        }
        ("POST", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/messages")
                && joined_room_subresource(path, "/messages").is_some() =>
        {
            let room_name = joined_room_subresource(path, "/messages")
                .expect("guarded joined-room messages path");
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            if message_body.trim().is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "message is required",
                ));
            }
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
            if rooms
                .add_message(&room_name, "local".to_owned(), message_body.clone())
                .is_some()
            {
                drop(rooms);
                session_command_permit.send(SessionCommand::SayRoom {
                    room: room_name.to_owned(),
                    body: message_body,
                });
                record_event(
                    state,
                    "room.message",
                    room_name.to_owned(),
                    Some("username=local".to_owned()),
                )
                .await;
                Ok(
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response("true".to_owned())
                    },
                )
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/ticker")
                && joined_room_subresource(path, "/ticker").is_some() =>
        {
            let room_name =
                joined_room_subresource(path, "/ticker").expect("guarded joined-room ticker path");
            let ticker = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "ticker"))
                .or_else(|| extract_json_string_field(body, "message"))
                .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
            if ticker.trim().is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "message is required",
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
            let mut rooms = state.rooms.write().await;
            let response = rooms
                .set_ticker(&room_name, ticker.clone())
                .map(|room| {
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response(
                            serde_json::json!({
                                "updated": true,
                                "room": room.controller_room_json(),
                            })
                            .to_string(),
                        )
                    }
                })
                .unwrap_or_else(routing::not_found_response);
            drop(rooms);
            if response.status != "404 Not Found" {
                session_command_permit.send(SessionCommand::SetRoomTicker {
                    room: room_name.to_owned(),
                    ticker,
                });
            }
            Ok(response)
        }
        ("POST", path)
            if path.starts_with("/api/rooms/joined/")
                && path.ends_with("/members")
                && joined_room_subresource(path, "/members").is_some() =>
        {
            let room_name = joined_room_subresource(path, "/members")
                .expect("guarded joined-room members path");
            let username = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "username"))
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|username| {
                    truncate_utf8_bytes(username.trim().to_owned(), MAX_ROOM_USERNAME_BYTES)
                })
                .unwrap_or_else(|| {
                    truncate_utf8_bytes(
                        body.trim().trim_matches('"').to_owned(),
                        MAX_ROOM_USERNAME_BYTES,
                    )
                });
            if username.trim().is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "username is required",
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
            let mut rooms = state.rooms.write().await;
            let response = match rooms.add_member(&room_name, username.clone()) {
                Ok(Some(room)) => {
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response(
                            serde_json::json!({
                                "updated": true,
                                "room": room.controller_room_json(),
                                "userCount": room.user_count.unwrap_or(0),
                            })
                            .to_string(),
                        )
                    }
                }
                Ok(None) => routing::not_found_response(),
                Err(()) => routing::service_unavailable_response("room member capacity is full"),
            };
            drop(rooms);
            if response.status != "404 Not Found" && response.status != "503 Service Unavailable" {
                session_command_permit.send(SessionCommand::AddRoomMember {
                    room: room_name.to_owned(),
                    username,
                });
            }
            if response.status == "200 OK" {
                record_event(state, "room.users.updated", room_name.to_string(), None).await;
            }
            Ok(response)
        }
        ("DELETE", path)
            if path.starts_with("/api/rooms/joined/") && path.matches('/').count() == 4 =>
        {
            let room_name = path.strip_prefix("/api/rooms/joined/").unwrap_or("");
            let room_name = decoded_path_segment(room_name).trim().to_owned();
            if room_name.is_empty() {
                return Ok(rooms_controller_value_bad_request_response(
                    "roomName is required",
                ));
            }
            let _room_persistence = state.room_persistence_lock.lock().await;
            let mut rooms = state.rooms.write().await;
            let previous = rooms.clone();

            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
                && !rooms
                    .records
                    .iter()
                    .any(|record| record.name == room_name && record.joined)
            {
                drop(rooms);
                return Ok(routing::not_found_response());
            }

            if let Some(record) = rooms.leave(&room_name) {
                let json_response = record.json();
                let mutated = rooms.clone();
                drop(rooms);
                if let Err(error) = persist_room_leave_checked(state, &room_name).await {
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

                Ok(
                    if route.path.starts_with("/api/v0/")
                        || state.config.controller_profile == ControllerProfile::Legacy
                    {
                        routing::no_content_response()
                    } else {
                        routing::ok_response(json_response)
                    },
                )
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        // GET room detail by name
        ("GET", path)
            if path.starts_with("/api/rooms/")
                && !path.ends_with("/messages")
                && !path.ends_with("/users")
                && path.matches('/').count() == 3 =>
        {
            let room_name = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let rooms = state.rooms.read().await;
            if let Some(record) = rooms.records.iter().find(|r| r.name == room_name) {
                Ok(routing::ok_response(record.json()))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        // WEBUI PARITY: Application/Server/Session status endpoints
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
