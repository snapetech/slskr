async fn route_dispatch_group_3_options_diagnostics(
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
