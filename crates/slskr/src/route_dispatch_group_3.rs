async fn route_dispatch_group_3(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
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
        ("POST", "/api/admin/webhooks") => {
            let url = match extract_json_string_field(body, "url") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("url is required")),
            };
            if url.len() > 2048 {
                return Ok(routing::bad_request_response("url is too long"));
            }
            if let Err(error) = webhooks::validate_webhook_url_for_registration(&url) {
                return Ok(routing::bad_request_response(&error.to_string()));
            }
            let events = match extract_webhook_events(body) {
                Ok(events) => events,
                Err(error) => return Ok(routing::bad_request_response(error)),
            };
            let secret = match extract_json_string_field(body, "secret") {
                Some(secret) => {
                    if let Err(error) = webhooks::validate_webhook_secret(&secret) {
                        return Ok(routing::bad_request_response(error));
                    }
                    secret
                }
                None => {
                    let Some(secret) = webhooks::Webhook::generate_secret() else {
                        return Ok(routing::service_unavailable_response(
                            "webhook secret generation unavailable",
                        ));
                    };
                    secret
                }
            };
            let webhook = webhooks::Webhook::new(url, events, secret.clone());
            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            let webhook_id = match webhooks.register(webhook.clone()) {
                Ok(id) => id,
                Err(_) => {
                    drop(webhooks);
                    return Ok(routing::bad_request_response("webhook limit reached"));
                }
            };
            let mutated = webhooks.clone();
            drop(webhooks);
            if let Err(error) = persist_webhook_checked(state, &webhook).await {
                rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(
                serde_json::json!({
                    "id": webhook_id,
                    "secret": secret,
                    "secretReturnedOnce": true,
                    "status": "created"
                })
                .to_string(),
            ))
        }
        ("GET", "/api/admin/webhooks") => {
            let webhooks = state.webhooks.read().await;
            let webhook_list: Vec<serde_json::Value> = webhooks
                .get_all()
                .iter()
                .map(|w| {
                    serde_json::json!({
                        "id": w.id,
                        "url": w.url,
                        "events": w.events.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                        "active": w.active,
                        "created_at": w.created_at,
                        "last_triggered": w.last_triggered,
                    })
                })
                .collect();
            let total = webhook_list.len();
            drop(webhooks);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::json!({"webhooks": webhook_list, "total": total}).to_string(),
            })
        }
        ("DELETE", path)
            if path.starts_with("/api/admin/webhooks/")
                && webhook_resource_id(path, "/api/admin/webhooks/").is_some() =>
        {
            let webhook_id = webhook_resource_id(path, "/api/admin/webhooks/")
                .expect("guarded admin webhook resource path");
            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            if webhooks.unregister(webhook_id).is_some() {
                let mutated = webhooks.clone();
                drop(webhooks);
                if let Err(error) = persist_webhook_delete_checked(state, webhook_id).await {
                    rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{\"status\":\"deleted\"}".to_owned()))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/admin/webhooks/")
                && path.ends_with("/test")
                && webhook_test_id(path, "/api/admin/webhooks/").is_some() =>
        {
            let webhook_id = webhook_test_id(path, "/api/admin/webhooks/")
                .expect("guarded admin webhook test path");
            let webhooks = state.webhooks.read().await;
            if let Some(webhook) = webhooks.get(webhook_id) {
                let payload = webhooks::WebhookDispatcher::test_payload(
                    webhooks::WebhookEvent::SearchCreated,
                    "test webhook delivery",
                );
                let webhook_clone = webhook.clone();
                drop(webhooks);
                let Ok(delivery_permit) = Arc::clone(&state.webhook_deliveries).try_acquire_owned()
                else {
                    return Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "application/json",
                        body: "{\"error\":\"too many webhook deliveries in progress\"}".to_owned(),
                    });
                };

                tokio::spawn(async move {
                    let _delivery_permit = delivery_permit;
                    let webhook_id = webhook_clone.id.clone();
                    if let Err(error) = webhooks::WebhookDispatcher::send_webhook(
                        &webhook_clone.url,
                        &webhook_clone.secret,
                        &payload.to_string(),
                        webhook_clone.timeout_seconds,
                    )
                    .await
                    {
                        ::tracing::warn!(%webhook_id, %error, "webhook test delivery failed");
                    }
                });

                Ok(routing::ok_response(
                    "{\"status\":\"test_sent\"}".to_owned(),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }
        // DATABASE MANAGEMENT ROUTES
        ("GET", "/api/admin/database/stats") => Ok(routing::ok_response(
            database_stats_value(state).await.to_string(),
        )),
        ("POST", "/api/admin/database/cleanup") => Ok(routing::ok_response(
            database_cleanup_value(state, body).await.to_string(),
        )),
        ("POST", "/api/admin/database/vacuum") => Ok(routing::ok_response(
            database_vacuum_value(state).await.to_string(),
        )),
        // API KEYS MANAGEMENT ROUTES
        ("POST", "/api/admin/keys") => Ok(routing::created_response(
            "{\"id\":null,\"created\":false,\"reason\":\"static SLSKR_API_TOKEN auth is active\"}"
                .to_owned(),
        )),
        ("GET", "/api/admin/keys") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: r#"{"keys":[],"total":0,"mode":"static","reason":"static SLSKR_API_TOKEN auth is active"}"#.to_owned(),
        }),
        ("DELETE", path) if path.starts_with("/api/admin/keys/") => {
            let key_id = path.rsplit('/').next().unwrap_or("");
            Ok(routing::ok_response(format!(
                "{{\"id\":\"{}\",\"revoked\":false,\"reason\":\"static token auth\"}}",
                json_escape(key_id)
            )))
        }
        ("GET", "/api/admin/keys/validate") => {
            Ok(routing::ok_response("{\"valid\":true}".to_owned()))
        }
        // MONITORING & TELEMETRY ROUTES (already exist but adding for completeness)
        ("GET", "/api/admin/monitoring") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: r#"{"cpu_percent":5.2,"memory_mb":128,"uptime_seconds":3600}"#.to_owned(),
        }),
        // WEBUI PARITY: Room routes with /joined prefix
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
            if should_persist {
                if let Err(error) = persist_room_join_checked(state, &room_name).await {
                    *rooms = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            drop(rooms);
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
                .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
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
                if let Err(error) = persist_room_leave_checked(state, &room_name).await {
                    *rooms = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(rooms);
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
        ("GET", "/api/application/build") => {
            let mut value = controller_version_json(state);
            value["protocol"] = serde_json::json!({
                "clientName": CLIENT_NAME,
                "major": CLIENT_MAJOR_VERSION,
                "minor": CLIENT_MINOR_VERSION,
            });
            Ok(routing::ok_response(value.to_string()))
        }
        // WEBUI PARITY: Options/Config read-write endpoints
        ("GET", "/api/private-message-auto-response") => {
            let settings = state.private_message_auto_response_settings.read().await;
            let mut value =
                serde_json::from_str::<serde_json::Value>(&settings.sanitized_json())
                    .map_err(|error| format!("auto-response settings json failed: {error}"))?;
            value["runtimeMutable"] = serde_json::Value::Bool(true);
            value["persisted"] = serde_json::Value::Bool(false);
            Ok(routing::ok_response(value.to_string()))
        }
        ("PUT", "/api/private-message-auto-response") => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(payload)) => payload,
                _ => {
                    return Ok(routing::bad_request_response(
                        "JSON object body is required",
                    ))
                }
            };
            let enabled = payload.get("enabled").and_then(serde_json::Value::as_bool);
            let message = payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(str::to_owned);
            if message.as_deref().is_some_and(|message| message.is_empty()) {
                return Ok(routing::bad_request_response(
                    "auto-response message must not be blank",
                ));
            }
            if message
                .as_ref()
                .is_some_and(|message| message.len() > 4_096)
            {
                return Ok(routing::bad_request_response(
                    "auto-response message exceeds 4096 bytes",
                ));
            }
            let cooldown = payload
                .get("cooldownMinutes")
                .or_else(|| payload.get("cooldown_minutes"))
                .and_then(serde_json::Value::as_u64);
            if cooldown.is_some_and(|cooldown| !(1..=1_440).contains(&cooldown)) {
                return Ok(routing::bad_request_response(
                    "cooldownMinutes must be between 1 and 1440",
                ));
            }
            if enabled.is_none() && message.is_none() && cooldown.is_none() {
                return Ok(routing::bad_request_response(
                    "enabled, message, or cooldownMinutes is required",
                ));
            }
            let mut settings = state.private_message_auto_response_settings.write().await;
            if let Some(enabled) = enabled {
                settings.enabled = enabled;
            }
            if let Some(message) = message {
                settings.message = message;
            }
            if let Some(cooldown) = cooldown {
                settings.cooldown_minutes = cooldown;
            }
            let disabled = !settings.enabled;
            let mut value =
                serde_json::from_str::<serde_json::Value>(&settings.sanitized_json())
                    .map_err(|error| format!("auto-response settings json failed: {error}"))?;
            drop(settings);
            if disabled {
                *state.private_message_auto_responses.write().await =
                    PrivateMessageAutoResponseTracker::default();
            }
            value["runtimeMutable"] = serde_json::Value::Bool(true);
            value["persisted"] = serde_json::Value::Bool(false);
            record_event(
                state,
                "message.auto_response_settings_updated",
                "private-message-auto-response",
                Some(format!("enabled={}", value["enabled"])),
            )
            .await;
            Ok(routing::ok_response(value.to_string()))
        }
        ("GET", "/api/options") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            let overlay = state.options_overlay.read().await;
            let body = controller_options_json(&state.config, &overlay, true);
            drop(overlay);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/options/startup") => {
            let overlay = state.options_overlay.read().await;
            let body = controller_options_json(&state.config, &overlay, false);
            drop(overlay);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/options/yaml") => {
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(controller_options_config_text_response(&state.config))
        }
        ("GET", "/api/options/debug") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) || !state.config.controller_debug {
                return Ok(controller_forbidden_response());
            }
            let overlay = state.options_overlay.read().await;
            let debug_view = controller_options_debug_view(state, &overlay);
            drop(overlay);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::Value::String(debug_view).to_string(),
            })
        }
        ("GET", "/api/options/yaml/location") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: controller_options_config_location_json(&state.config),
            })
        }
        ("GET", "/api/autoreplace") => {
            let json = if route.path.starts_with("/api/v0/") {
                let enabled = state.runtime.read().await.autoreplace_enabled;
                serde_json::json!({
                    "enabled": enabled,
                    "lastRunAt": null,
                    "lastRunProcessedCount": 0,
                    "lastRunReplacedCount": 0,
                    "intervalSeconds": 300,
                })
                .to_string()
            } else {
                "{\"enabled\":false,\"intervalSeconds\":60,\"stuckCount\":0,\"lastRunProcessedCount\":0,\"lastRunReplacedCount\":0,\"rules\":[],\"count\":0}".to_string()
            };
            Ok(routing::ok_response(json))
        }
        ("PUT", "/api/options") => {
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            Ok(apply_controller_options_overlay(body, state).await)
        }
        // HEALTH & DIAGNOSTICS ENDPOINTS
        ("GET", "/api/health/detailed") => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;
            let messages = state.messages.read().await;
            let users = state.users.read().await;

            let diagnostics = serde_json::json!({
                "status": "operational",
                "transfers": {
                    "active": transfers.entries.iter().filter(|t| is_active_transfer_status(&t.status)).count(),
                    "total": transfers.entries.len(),
                    "succeeded": transfers.entries.iter().filter(|t| is_successful_transfer_status(&t.status)).count(),
                    "failed": transfers.entries.iter().filter(|t| is_failed_transfer_status(&t.status)).count(),
                },
                "searches": {
                    "total": searches.records.len(),
                },
                "messages": {
                    "total": messages.records.len(),
                    "unread": messages.records.iter().filter(|m| !m.acknowledged).count(),
                },
                "users": {
                    "total": users.records.len(),
                },
            }).to_string();

            drop(transfers);
            drop(searches);
            drop(messages);
            drop(users);

            Ok(routing::ok_response(diagnostics))
        }

        ("GET", "/api/diagnostics") => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;

            let diag = serde_json::json!({
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "transfers": {
                    "queue_size": transfers.entries.len(),
                    "active_downloads": transfers.entries.iter().filter(|t| is_active_transfer_status(&t.status) && t.direction == 0).count(),
                    "active_uploads": transfers.entries.iter().filter(|t| is_active_transfer_status(&t.status) && t.direction != 0).count(),
                },
                "searches": {
                    "total": searches.records.len(),
                },
            }).to_string();

            drop(transfers);
            drop(searches);

            Ok(routing::ok_response(diag))
        }

        // DATABASE MAINTENANCE ENDPOINTS
        ("GET", "/api/v0/database/stats") => Ok(routing::ok_response(
            database_stats_value(state).await.to_string(),
        )),
        ("POST", "/api/v0/database/cleanup") => Ok(routing::ok_response(
            database_cleanup_value(state, body).await.to_string(),
        )),
        ("POST", "/api/v0/database/vacuum") => Ok(routing::ok_response(
            database_vacuum_value(state).await.to_string(),
        )),
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
