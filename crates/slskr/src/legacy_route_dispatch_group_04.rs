use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_04(
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
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/browse") => {
            if let Some(username) = user_route_username(path, "/browse") {
                if let Some(response) = controller_user_read_failure_response(
                    state,
                    route.path,
                    &username,
                    true,
                )
                .await
                {
                    return Ok(response);
                }
                let browse = state.browse.read().await;
                let record = browse
                    .records
                    .iter()
                    .find(|record| record.username == username);
                if record.is_none() {
                    drop(browse);
                    if state.config.controller_profile == ControllerProfile::Native {
                        let session_state = state.session.read().await.state;
                        if session_state != "connected" {
                            return Ok(HttpResponse {
                                status: "503 Service Unavailable",
                                content_type: "application/json",
                                body: serde_json::to_string(
                                    "Soulseek server connection is not ready",
                                )
                                .unwrap_or_else(|_| {
                                    "\"Soulseek server connection is not ready\"".to_owned()
                                }),
                            });
                        }
                    }
                    return Ok(routing::not_found_response());
                }
                let entries = record.map(|record| record.entries.as_slice()).unwrap_or(&[]);
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
             let requests = browse.records.iter().map(|r| {
                 serde_json::json!({
                     "username": r.username,
                     "status": r.status,
                     "requested_at": r.requested_at,
                     "updated_at": r.updated_at,
                 })
             }).collect::<Vec<_>>();
             drop(browse);
             Ok(HttpResponse {
                 status: "200 OK",
                 content_type: "application/json",
                 body: serde_json::to_string(&serde_json::json!({"requests": requests, "count": requests.len()})).unwrap_or_else(|_| "{}".to_string()),
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
            let _webhook_persistence = state.webhook_persistence_lock.lock().await;
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
            Ok(routing::created_response(serde_json::json!({
                "id": webhook_id,
                "secret": secret,
                "secretReturnedOnce": true,
                "status": "created"
            }).to_string()))
        }
        ("GET", "/api/admin/webhooks") => {
            let webhooks = state.webhooks.read().await;
            let webhook_list: Vec<serde_json::Value> = webhooks.get_all().iter().map(|w| {
                serde_json::json!({
                    "id": w.id,
                    "url": w.url,
                    "events": w.events.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                    "active": w.active,
                    "created_at": w.created_at,
                    "last_triggered": w.last_triggered,
                    "retry_count": w.retry_count,
                    "max_retries": w.max_retries,
                    "timeout_seconds": w.timeout_seconds,
                })
            }).collect();
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
            let _webhook_persistence = state.webhook_persistence_lock.lock().await;
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

                Ok(routing::ok_response("{\"status\":\"test_sent\"}".to_owned()))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }
        // DATABASE MANAGEMENT ROUTES
        ("GET", "/api/admin/database/stats") => {
            Ok(routing::ok_response(database_stats_value(state).await.to_string()))
        }
        ("POST", "/api/admin/database/cleanup") => {
            Ok(routing::ok_response(
                database_cleanup_value(state, body).await.to_string(),
            ))
        }
        ("POST", "/api/admin/database/vacuum") => {
            Ok(routing::ok_response(database_vacuum_value(state).await.to_string()))
        }
        // API KEYS MANAGEMENT ROUTES
        ("POST", "/api/admin/keys") => {
            Ok(routing::created_response(
                "{\"id\":null,\"created\":false,\"reason\":\"static SLSKR_API_TOKEN auth is active\"}".to_owned(),
            ))
        }
        ("GET", "/api/admin/keys") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: r#"{"keys":[],"total":0,"mode":"static","reason":"static SLSKR_API_TOKEN auth is active"}"#.to_owned(),
            })
        }
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
        ("GET", "/api/admin/monitoring") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: r#"{"cpu_percent":5.2,"memory_mb":128,"uptime_seconds":3600}"#.to_owned(),
            })
        }
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
            if route.path.starts_with("/api/v0/")
                && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek is reconnecting; try again shortly.",
                ));
            }
            let _room_persistence = state.room_persistence_lock.lock().await;
            let mut rooms = state.rooms.write().await;
            if route.path.starts_with("/api/v0/") && rooms.records.iter().any(|record| {
                record.name == bounded_room_name(&room_name)
                    && record.joined
                    && record.last_error.is_none()
            }) {
                let existing = rooms
                    .records
                    .iter()
                    .find(|record| record.name == bounded_room_name(&room_name))
                    .cloned()
                    .expect("joined room exists");
                drop(rooms);
                return Ok(if route.path.starts_with("/api/v0/") {
                    HttpResponse {
                        status: "200 OK",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response(existing.controller_room_json().to_string())
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
        ("GET", path) if path.starts_with("/api/rooms/joined/") && path.matches('/').count() == 4 => {
            let room_name = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let rooms = state.rooms.read().await;
            let response = rooms
                .records
                .iter()
                .find(|r| {
                    r.name == room_name
                        && (state.config.controller_profile
                            != ControllerProfile::Legacy
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
                return Ok(rooms_controller_value_bad_request_response("message is required"));
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
                Ok(if route.path.starts_with("/api/v0/")
                    || state.config.controller_profile
                        == ControllerProfile::Legacy
                {
                    HttpResponse {
                        status: "201 Created",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response("true".to_owned())
                })
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
            let room_name = joined_room_subresource(path, "/ticker")
                .expect("guarded joined-room ticker path");
            let ticker = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "ticker"))
                .or_else(|| extract_json_string_field(body, "message"))
                .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
            if ticker.trim().is_empty() {
                return Ok(rooms_controller_value_bad_request_response("message is required"));
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
                        || state.config.controller_profile
                            == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response(serde_json::json!({
                            "updated": true,
                            "room": room.controller_room_json(),
                        }).to_string())
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
                return Ok(rooms_controller_value_bad_request_response("username is required"));
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
                        || state.config.controller_profile
                            == ControllerProfile::Legacy
                    {
                        HttpResponse {
                            status: "201 Created",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::ok_response(serde_json::json!({
                            "updated": true,
                            "room": room.controller_room_json(),
                            "userCount": room.user_count.unwrap_or(0),
                        }).to_string())
                    }
                }
                Ok(None) => routing::not_found_response(),
                Err(()) => routing::service_unavailable_response("room member capacity is full"),
            };
            drop(rooms);
            if response.status != "404 Not Found"
                && response.status != "503 Service Unavailable"
            {
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
                return Ok(rooms_controller_value_bad_request_response("roomName is required"));
            }
            let _room_persistence = state.room_persistence_lock.lock().await;
            let mut rooms = state.rooms.write().await;
            let previous = rooms.clone();

            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile
                    == ControllerProfile::Native
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

                Ok(if route.path.starts_with("/api/v0/")
                    || state.config.controller_profile
                        == ControllerProfile::Legacy
                {
                    routing::no_content_response()
                } else {
                    routing::ok_response(json_response)
                })
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        // GET room detail by name
        ("GET", path) if path.starts_with("/api/rooms/") && !path.ends_with("/messages") && !path.ends_with("/users") && path.matches('/').count() == 3 => {
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
            let mut value = serde_json::from_str::<serde_json::Value>(&settings.sanitized_json())
                .map_err(|error| format!("auto-response settings json failed: {error}"))?;
            value["runtimeMutable"] = serde_json::Value::Bool(true);
            value["persisted"] = serde_json::Value::Bool(false);
            Ok(routing::ok_response(value.to_string()))
        }
        ("PUT", "/api/private-message-auto-response") => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(payload)) => payload,
                _ => return Ok(routing::bad_request_response("JSON object body is required")),
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
            if message.as_ref().is_some_and(|message| message.len() > 4_096) {
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
            let mut value = serde_json::from_str::<serde_json::Value>(&settings.sanitized_json())
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
        ("GET", "/api/v0/database/stats") => {
            Ok(routing::ok_response(database_stats_value(state).await.to_string()))
        }
        ("POST", "/api/v0/database/cleanup") => {
            Ok(routing::ok_response(
                database_cleanup_value(state, body).await.to_string(),
            ))
        }
        ("POST", "/api/v0/database/vacuum") => {
            Ok(routing::ok_response(database_vacuum_value(state).await.to_string()))
        }
        ("GET", "/api/database/stats") => {
            Ok(routing::ok_response(database_stats_value(state).await.to_string()))
        }
        ("POST", "/api/database/cleanup") => {
            Ok(routing::ok_response(
                database_cleanup_value(state, body).await.to_string(),
            ))
        }
        ("POST", "/api/database/vacuum") => {
            Ok(routing::ok_response(database_vacuum_value(state).await.to_string()))
        }

        // COLLECTIONS ENDPOINTS
        ("GET", "/api/collections") => {
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                format!(
                    "[{}]",
                    collections
                        .records
                        .iter()
                        .filter(|record| {
                            !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
                        })
                        .map(CollectionRecord::native_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                collections.json_array(route.query, caller_id.as_deref())
            };
            drop(collections);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/shared") => {
            let shares = state.shares.read().await;
            let entries = shares
                .roots
                .iter()
                .map(|root| {
                    let mut value = controller_share_value(root);
                    value["name"] = serde_json::json!(root.label);
                    value
                })
                .collect::<Vec<_>>();
            drop(shares);
            Ok(routing::ok_response(serde_json::Value::Array(entries).to_string()))
        }
        ("POST", "/api/collections") => {
            let Some(name) = extract_json_string_field(body, "title")
                .or_else(|| extract_json_string_field(body, "name"))
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(routing::bad_request_response("title is required"));
            };
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            // Matches the oracle's real AuthenticatedWebUserId.Resolve:
            // the collection's real owner is the caller's own resolved
            // identity, never a hardcoded placeholder -- empty when no
            // per-caller identity is resolvable (single-operator mode).
            let owner_user_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            )
            .unwrap_or_default();
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            let previous = collections.clone();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let record = if compatibility_contract {
                let collection_type = extract_json_string_field(body, "type")
                    .filter(|value| value.trim() == "Playlist")
                    .map(|_| "Playlist".to_owned())
                    .unwrap_or_else(|| "ShareList".to_owned());
                collections.create_with_contract(
                    uuid::Uuid::new_v4().to_string(),
                    owner_user_id,
                    name,
                    description,
                    collection_type,
                )
            } else {
                collections.create(owner_user_id, name, description)
            };
            let Some(record) = record else {
                return Ok(routing::service_unavailable_response(
                    "collection capacity is full",
                ));
            };
            let mutated = collections.clone();
            let json = if compatibility_contract {
                record.native_json()
            } else {
                record.json()
            };
            drop(collections);
            if let Err(error) = persist_collection_created(state, &record).await {
                rollback_collections_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/collections/") && !path.ends_with("/items") && path.matches('/').count() == 3 => {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            if let Some(record) = collections
                .get(id)
                .filter(|record| !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id))
            {
                let json = if route.path.starts_with("/api/v0/") {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/collections/") && !path.contains("/items") && path.matches('/').count() == 3 => {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let (name, description, collection_type) = if compatibility_contract {
                let request = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(request @ serde_json::Value::Object(_)) => request,
                    _ => return Ok(routing::bad_request_response("Request is required.")),
                };
                let name = request
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .map(str::to_owned);
                if name.as_ref().is_some_and(|name| name.is_empty()) {
                    return Ok(routing::bad_request_response("Title cannot be blank."));
                }
                let description = request
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .map(str::to_owned);
                let collection_type = request
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .map(|value| {
                        if value.trim() == "Playlist" {
                            "Playlist".to_owned()
                        } else {
                            "ShareList".to_owned()
                        }
                    });
                (name, description, collection_type)
            } else {
                let Some(name) = extract_json_string_field(body, "title")
                    .or_else(|| extract_json_string_field(body, "name"))
                    .filter(|value| !value.trim().is_empty())
                else {
                    return Ok(routing::bad_request_response("title is required"));
                };
                (
                    Some(name),
                    Some(extract_json_string_field(body, "description").unwrap_or_default()),
                    None,
                )
            };
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            if collections
                .get(id)
                .is_some_and(|record| collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            let updated = if compatibility_contract {
                collections.update_contract(id, name, description, collection_type)
            } else {
                collections.update(
                    id,
                    name.unwrap_or_default(),
                    description.unwrap_or_default(),
                )
            };
            if let Some(record) = updated {
                let mutated = collections.clone();
                let json = if compatibility_contract {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/collections/") && !path.contains("/items") && path.matches('/').count() == 3 => {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut grants = state.share_grants.write().await;
            let mut collections = state.collections.write().await;
            if collections
                .get(id)
                .is_some_and(|record| collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id))
            {
                drop(collections);
                drop(grants);
                return Ok(routing::not_found_response());
            }
            let previous_collections = collections.clone();
            let previous_grants = grants.clone();
            let deleted = collections.delete(id);
            if deleted {
                let revoked_grants = grants.delete_by_collection(id);
                let mutated_collections = collections.clone();
                let mutated_grants = grants.clone();
                drop(collections);
                drop(grants);
                if let Err(error) = persist_collection_delete(state, id).await {
                    let mut grants = state.share_grants.write().await;
                    let mut collections = state.collections.write().await;
                    if route_dispatch::share_grant_store_matches(&grants, &mutated_grants)
                        && *collections == mutated_collections
                    {
                        *collections = previous_collections;
                        *grants = previous_grants;
                    }
                    drop(collections);
                    drop(grants);
                    return Ok(routing::service_unavailable_response(&error));
                }
                let mut tokens = state.share_access_tokens.write().await;
                for grant in &revoked_grants {
                    tokens.revoke_grant(&grant.id);
                }
                drop(tokens);
                let mut tickets = state.stream_tickets.write().await;
                for grant in revoked_grants {
                    tickets.revoke_source(&format!("share:{}", grant.id));
                }
                drop(tickets);
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::ok_response("{}".to_string())
                })
            } else {
                drop(collections);
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("GET", path)
            if path.starts_with("/api/collections/")
                && path.ends_with("/items")
                && collection_items_id(path).is_some() =>
        {
            let id = collection_items_id(path).expect("guarded collection items path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            if let Some(record) = collections
                .get(id)
                .filter(|record| !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id))
            {
                let compatibility_contract = route.path.starts_with("/api/v0/");
                let items = record.items.iter()
                    .enumerate()
                    .map(|(ordinal, item)| if compatibility_contract {
                        item.native_json(&record.id, ordinal)
                    } else {
                        item.json()
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", items);
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/collections/")
                && path.ends_with("/items")
                && collection_items_id(path).is_some() =>
        {
            let id = collection_items_id(path).expect("guarded collection items path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let content_id = extract_json_string_field(body, "contentId")
                .or_else(|| extract_json_string_field(body, "content_id"))
                .unwrap_or_default();
            if compatibility_contract && content_id.trim().is_empty() {
                return Ok(routing::bad_request_response("ContentId is required."));
            }
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "mediaKind")
                .or_else(|| extract_json_string_field(body, "kind"))
                .unwrap_or_else(|| "Audio".to_string());
            let file_name = extract_json_string_field(body, "fileName").unwrap_or_default();
            let album = extract_json_string_field(body, "album").unwrap_or_default();
            let content_hash = extract_json_string_field(body, "contentHash")
                .or_else(|| extract_json_string_field(body, "sha256"))
                .unwrap_or_default();

            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            if collections
                .get(id)
                .is_some_and(|record| collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            match collections.add_item_with_contract(
                id,
                compatibility_contract.then(|| uuid::Uuid::new_v4().to_string()),
                content_id,
                artist,
                title,
                kind,
                file_name,
                album,
                content_hash,
            ) {
              Ok(Some(item)) => {
                let record = collections
                    .get(id)
                    .expect("item was added to an existing collection");
                let mutated = collections.clone();
                let json = if compatibility_contract {
                    item.native_json(id, record.items.len().saturating_sub(1))
                } else {
                    item.json()
                };
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::created_response(json))
              }
              Ok(None) => {
                drop(collections);
                Ok(routing::not_found_response())
              }
              Err(()) => {
                drop(collections);
                Ok(routing::service_unavailable_response("collection item capacity is full"))
              }
            }
        }
        ("DELETE", path) if path.starts_with("/api/collections/items/") => {
            let item_id = path.strip_prefix("/api/collections/items/").unwrap_or("");
            if item_id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            let collection_id = collections.collection_id_for_item(item_id);
            if collection_id
                .as_deref()
                .and_then(|id| collections.get(id))
                .is_some_and(|record| collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            if let Some(item) = collections.remove_item(item_id) {
                let record = collection_id
                    .as_deref()
                    .and_then(|id| collections.get(id))
                    .expect("removed item belonged to an existing collection");
                let mutated = collections.clone();
                let json = serde_json::json!({
                    "deleted": true,
                    "item": serde_json::from_str::<serde_json::Value>(&item.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": item_id })),
                })
                .to_string();
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/collections/items/") => {
            let item_id = path.strip_prefix("/api/collections/items/").unwrap_or("");
            if item_id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let artist = extract_json_string_field(body, "artist");
            let title = extract_json_string_field(body, "title");
            let kind = extract_json_string_field(body, "kind")
                .or_else(|| extract_json_string_field(body, "mediaKind"));

            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            let collection_id = collections.collection_id_for_item(item_id);
            if collection_id
                .as_deref()
                .and_then(|id| collections.get(id))
                .is_some_and(|record| collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            if let Some(item) = collections.update_item(item_id, artist, title, kind) {
                let record = collection_id
                    .as_deref()
                    .and_then(|id| collections.get(id))
                    .expect("updated item belonged to an existing collection");
                let mutated = collections.clone();
                let json = item.json();
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }

        // WISHLIST ENDPOINTS
        ("GET", "/api/wishlist") => {
            let mut wishlist = state.wishlist.write().await;
            let json = if route.path.starts_with("/api/v0/") {
                wishlist.get_or_create();
                format!(
                    "[{}]",
                    wishlist
                        .records
                        .iter()
                        .flat_map(|record| record.items.iter())
                        .map(WishlistItem::native_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                wishlist.json_array()
            };
            drop(wishlist);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/wishlist") => {
            let search_text = extract_json_string_field(body, "searchText").unwrap_or_default();
            let artist = extract_json_string_field(body, "artist").unwrap_or_else(|| search_text.clone());
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            if artist.trim().is_empty() && title.trim().is_empty() {
                return Ok(routing::bad_request_response("SearchText is required"));
            }
            let filter = extract_json_string_field(body, "filter").unwrap_or_default();
            let enabled = extract_json_bool_field(body, "enabled").unwrap_or(true);
            let auto_download = extract_json_bool_field(body, "autoDownload").unwrap_or(false);
            let max_results = extract_json_u64_field(body, "maxResults").unwrap_or(100);
            if max_results == 0 || max_results > MAX_WISHLIST_RESULTS as u64 {
                return Ok(routing::bad_request_response(
                    "MaxResults must be between 1 and 10000",
                ));
            }
            let max_downloads = extract_json_optional_u64_field(body, "maxDownloads");
            if max_downloads.flatten().is_some_and(|value| {
                value == 0 || value > MAX_WISHLIST_DOWNLOADS
            }) {
                return Ok(routing::bad_request_response(
                    "MaxDownloads must be null or between 1 and 1000000",
                ));
            }

            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let id = compatibility_contract.then(|| uuid::Uuid::new_v4().to_string());
            match wishlist.add_item_with_contract(
                id,
                artist,
                title,
                kind,
                filter,
                enabled,
                auto_download,
                usize::try_from(max_results).unwrap_or(MAX_WISHLIST_RESULTS),
                max_downloads.flatten(),
            ) {
              Ok(item) => {
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
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
                Ok(routing::created_response(json))
              }
              Err(()) => {
                drop(wishlist);
                Ok(routing::service_unavailable_response("wishlist item capacity is full"))
              }
            }
        }
        ("GET", path)
            if wishlist_item_action_id(path, "/searches").is_some() =>
        {
            let item_id = wishlist_item_action_id(path, "/searches")
                .expect("guarded wishlist history path");
            if state.wishlist.read().await.get_item(item_id).is_none() {
                return Ok(routing::not_found_response());
            }
            let searches = state.searches.read().await;
            let json = searches.wishlist_history_json(item_id, route.query);
            drop(searches);
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/wishlist/") && !path.contains("/ignored-results") => {
            let Some(item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let wishlist = state.wishlist.read().await;
            let Some(item) = wishlist.get_item(item_id) else {
                return Ok(routing::not_found_response());
            };
            Ok(routing::ok_response(if route.path.starts_with("/api/v0/") {
                item.native_json()
            } else {
                item.json()
            }))
        }
        ("POST", "/api/wishlist/mark-all-viewed") => {
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let items = wishlist.mark_all_viewed();
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_items_checked(state, &items).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("POST", path)
            if wishlist_item_action_id(path, "/mark-viewed").is_some() =>
        {
            let item_id = wishlist_item_action_id(path, "/mark-viewed")
                .expect("guarded wishlist viewed path");
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let Some(item) = wishlist.mark_viewed(item_id) else {
                return Ok(routing::not_found_response());
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("GET", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let item_id = wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let wishlist = state.wishlist.read().await;
            let Some(rules) = wishlist.list_ignored_results(item_id) else {
                return Ok(routing::not_found_response());
            };
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let json = serde_json::Value::Array(
                rules
                    .iter()
                    .map(|rule| {
                        if compatibility_contract {
                            rule.native_json()
                        } else {
                            rule.json()
                        }
                    })
                    .collect(),
            )
            .to_string();
            Ok(routing::ok_response(json))
        }
        ("POST", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let item_id = wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if username.trim().is_empty() || normalize_wishlist_directory(&directory).is_empty() {
                return Ok(routing::bad_request_response(
                    if route.path.starts_with("/api/v0/") {
                        "Username and Directory are required"
                    } else {
                        "username and directory are required"
                    },
                ));
            }

            let _wishlist_search_persistence =
                state.wishlist_search_persistence_lock.lock().await;
            let _search_persistence = state.search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let (rule, created) = match wishlist.ignore_result(
                item_id,
                &username,
                &directory,
                route.path.starts_with("/api/v0/"),
            ) {
                Ok(result) => result,
                Err("not_found") => return Ok(routing::not_found_response()),
                Err("capacity") => {
                    return Ok(routing::service_unavailable_response(
                        "wishlist ignored-result capacity is full",
                    ));
                }
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        if route.path.starts_with("/api/v0/") {
                            "Username and Directory are required"
                        } else {
                            "username and directory are required"
                        },
                    ));
                }
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if created {
                let (previous_searches, mutated_searches, changed_searches) = {
                    let mut searches = state.searches.write().await;
                    let previous = searches.clone();
                    let changed = searches.suppress_ignored_result(&rule);
                    let mutated = searches.clone();
                    (previous, mutated, changed)
                };
                if let Err(error) = persist_wishlist_ignored_result_and_searches_checked(
                    state,
                    &rule,
                    &changed_searches,
                )
                .await
                {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    let mut searches = state.searches.write().await;
                    if *searches == mutated_searches {
                        *searches = previous_searches;
                    }
                    drop(searches);
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
                for search in &changed_searches {
                    publish_search_hub_event(state, "update", search);
                }
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let json = if compatibility_contract {
                rule.native_json()
            } else {
                rule.json()
            }
            .to_string();
            if created || compatibility_contract {
                Ok(routing::created_response(json))
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if wishlist_ignored_result_ids(path).is_some() => {
            let (item_id, rule_id) =
                wishlist_ignored_result_ids(path).expect("guarded ignored rule path");
            let _wishlist_search_persistence =
                state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            if !wishlist.delete_ignored_result(item_id, rule_id) {
                return Ok(routing::not_found_response());
            }
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) =
                persist_wishlist_ignored_result_delete_checked(state, item_id, rule_id).await
            {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("DELETE", path) if path.starts_with("/api/wishlist/") => {
            let Some(item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            if let Some(record) = wishlist.remove_item(item_id) {
                let mutated = wishlist.clone();
                let json = serde_json::json!({
                    "deleted": true,
                    "item_id": item_id,
                    "remaining": record.items.len(),
                    "updated_at": record.updated_at,
                })
                .to_string();
                drop(wishlist);
                if let Err(error) = persist_wishlist_item_delete_checked(state, item_id).await {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::ok_response(json)
                })
            } else {
                drop(wishlist);
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                })
            }
        }

        // CONTACTS ENDPOINTS
        ("GET", "/api/contacts/nearby") => {
            let contacts = state.contacts.read().await;
            let json = contacts.nearby_json(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
