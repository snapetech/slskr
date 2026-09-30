async fn route_dispatch_group_1_events(
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
        ("GET", "/api/events/records") => {
            let events = state.events.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: events.json(route.query),
            })
        }
        ("GET", "/api/events") | ("GET", "/api/events/slskd") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/events"
            {
                if let Some(raw) = query_parameter(route.query, "offset") {
                    if raw.parse::<i64>().is_err() {
                        return Ok(routing::bad_request_response(
                            "Offset must be greater than or equal to zero",
                        ));
                    }
                    if raw.parse::<i64>().is_ok_and(|value| value < 0) {
                        return Ok(routing::bad_request_response(
                            "Offset must be greater than or equal to zero",
                        ));
                    }
                }
                if let Some(raw) = query_parameter(route.query, "limit") {
                    if raw.parse::<i64>().is_err() {
                        return Ok(routing::bad_request_response(
                            "Limit must be greater than zero",
                        ));
                    }
                    if raw.parse::<i64>().is_ok_and(|value| value <= 0) {
                        return Ok(routing::bad_request_response(
                            "Limit must be greater than zero",
                        ));
                    }
                }
            }
            if let Some(response) = controller_events_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            if let Some(response) =
                controller_native_events_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let events = state.events.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: events.controller_json(route.query),
            })
        }
        ("POST", path) if path.starts_with("/api/events/") => {
            let Some(kind) = path_segment_after(path, "/api/events/") else {
                return Ok(routing::not_found_response());
            };
            if route.path.starts_with("/api/v0/") {
                let known = [
                    "DownloadFileComplete",
                    "DownloadDirectoryComplete",
                    "UploadFileComplete",
                    "PrivateMessageReceived",
                    "RoomMessageReceived",
                    "Noop",
                ];
                if !known.iter().any(|value| value.eq_ignore_ascii_case(kind)) {
                    return Ok(HttpResponse {
                        status: "400 Bad Request",
                        content_type: "application/json",
                        body: serde_json::json!("Unknown event type").to_string(),
                    });
                }
                let disambiguator = match serde_json::from_str::<String>(body) {
                    Ok(value) => value.trim().to_owned(),
                    Err(_) => {
                        return Ok(routing::bad_request_response(
                            "event disambiguator must be a JSON string",
                        ))
                    }
                };
                if disambiguator.len() > 128 {
                    return Ok(HttpResponse {
                        status: "400 Bad Request",
                        content_type: "application/json",
                        body: serde_json::json!("Disambiguator cannot exceed 128 characters")
                            .to_string(),
                    });
                }
            }
            let _event_persistence = state.event_persistence_lock.lock().await;
            let mut events = state.events.write().await;
            let previous = events.clone();
            let record_kind = if route.path.starts_with("/api/v0/") {
                kind
            } else {
                "compat.event"
            };
            let record = events.record(
                record_kind,
                kind,
                json_body_string(body).or_else(|| Some(body.to_owned())),
            );
            let count = events.records.len();
            let mutated = events.clone();
            drop(events);
            if let Err(error) = persist_event_record_checked(state, &record).await {
                let mut events = state.events.write().await;
                if *events == mutated {
                    *events = previous;
                }
                drop(events);
                return Ok(
                    if state.config.controller_profile == ControllerProfile::Native {
                        routing::internal_server_error_response("Failed to raise event")
                    } else {
                        routing::service_unavailable_response(&error)
                    },
                );
            }
            drop(_event_persistence);
            scripts::dispatch(
                &state.managed_background_tasks,
                state.integration_settings.read().await.scripts.clone(),
                state.config.state_dir.join("scripts"),
                state.config.controller_profile,
                kind,
                &serde_json::json!({}),
            );
            let response_body = serde_json::json!({
                "recorded": true,
                "event": record.controller_json(),
                "count": count,
            })
            .to_string();
            Ok(if route.path.starts_with("/api/v0/") {
                routing::created_response(response_body)
            } else {
                routing::ok_response(response_body)
            })
        }
        ("GET", "/api/logs") => {
            let events = state.events.read().await;
            let logs = events
                .records
                .iter()
                .rev()
                .filter(|event| event.kind == "log.created")
                .map(EventRecord::data_json)
                .collect::<Vec<_>>();
            drop(events);
            if route.path == "/api/v0/logs" {
                Ok(routing::ok_response(
                    serde_json::Value::Array(logs).to_string(),
                ))
            } else {
                Ok(routing::ok_response(
                    serde_json::json!({
                        "entries": logs,
                        "level": logging::LogConfig::level_name(*state.log_level.read().await),
                        "levels": ["Trace", "Debug", "Information", "Warning", "Error"],
                        "limit": EVENT_HISTORY_LIMIT,
                    })
                    .to_string(),
                ))
            }
        }
        ("GET", "/api/logs/level") => Ok(routing::ok_response(
            serde_json::json!({
                "level": logging::LogConfig::level_name(*state.log_level.read().await),
                "levels": ["Trace", "Debug", "Information", "Warning", "Error"],
                "source": "runtime",
            })
            .to_string(),
        )),
        ("PUT", "/api/logs/level") => {
            let requested = extract_json_string_field(body, "level")
                .or_else(|| json_body_string(body))
                .unwrap_or_default();
            let Some(level) = logging::LogConfig::parse_level(&requested) else {
                return Ok(routing::bad_request_response("invalid log level"));
            };
            {
                let mut current = state.log_level.write().await;
                *current = level;
            }
            record_daemon_log(
                state,
                logging::LogLevel::Info,
                "logging",
                format!(
                    "runtime log level changed to {}",
                    logging::LogConfig::level_name(level)
                ),
            )
            .await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "level": logging::LogConfig::level_name(level),
                    "updated": true,
                })
                .to_string(),
            ))
        }
        // WEBHOOK ENDPOINTS
        ("GET", "/api/webhooks") => {
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
                        "retry_count": w.retry_count,
                        "max_retries": w.max_retries,
                        "timeout_seconds": w.timeout_seconds,
                    })
                })
                .collect();
            drop(webhooks);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::to_string(&serde_json::json!({"webhooks": webhook_list}))
                    .unwrap_or_else(|_| "{}".to_string()),
            })
        }

        ("POST", "/api/webhooks") => {
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

            let response = serde_json::json!({
                "id": webhook_id,
                "secret": secret,
                "secretReturnedOnce": true,
                "status": "created"
            });

            Ok(routing::created_response(
                serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()),
            ))
        }

        ("DELETE", path)
            if path.starts_with("/api/webhooks/")
                && webhook_resource_id(path, "/api/webhooks/").is_some() =>
        {
            let webhook_id =
                webhook_resource_id(path, "/api/webhooks/").expect("guarded webhook resource path");
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
                Ok(routing::ok_response(
                    serde_json::json!({"status": "deleted"}).to_string(),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }

        ("PATCH", path)
            if path.starts_with("/api/webhooks/")
                && webhook_resource_id(path, "/api/webhooks/").is_some() =>
        {
            let webhook_id =
                webhook_resource_id(path, "/api/webhooks/").expect("guarded webhook resource path");
            let Some(active) = extract_json_bool_field(body, "active") else {
                return Ok(routing::bad_request_response("active boolean is required"));
            };

            let _webhook_persistence = state.webhook_persistence_lock.lock().await;
            let mut webhooks = state.webhooks.write().await;
            let previous = webhooks.clone();
            if let Some(webhook) = webhooks.get_mut(webhook_id) {
                webhook.active = active;
                let webhook = webhook.clone();
                let mutated = webhooks.clone();
                let updated = serde_json::json!({
                    "id": webhook.id,
                    "active": webhook.active,
                });
                drop(webhooks);
                if let Err(error) = persist_webhook_checked(state, &webhook).await {
                    rollback_webhooks_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(
                    serde_json::to_string(&updated).unwrap_or_else(|_| "{}".to_string()),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }

        // ADDITIONAL MISSING PATCH ENDPOINTS (Phase 5)
        ("PATCH", "/api/options") => {
            if let Some(response) = controller_options_validation_failure_response(state) {
                return Ok(response);
            }
            if !effective_remote_configuration(state) {
                return Ok(controller_forbidden_response());
            }
            let model = serde_json::from_str::<serde_json::Value>(body);
            if model.as_ref().is_ok_and(|value| !value.is_object()) {
                return Ok(match state.config.controller_profile {
                    ControllerProfile::Legacy => HttpResponse {
                        status: "204 No Content",
                        content_type: "",
                        body: String::new(),
                    },
                    ControllerProfile::Native => options_model_binding_problem_response(),
                });
            }
            if model.is_err() {
                return Ok(options_model_binding_problem_response());
            }
            Ok(apply_controller_options_overlay(body, state).await)
        }

        ("PATCH", path)
            if path.starts_with("/api/library/health/issues/")
                && library_health_issue_id(path).is_some() =>
        {
            let versioned_contract = route.path.starts_with("/api/v0/");
            if versioned_contract
                && (body.trim().is_empty()
                    || !serde_json::from_str::<serde_json::Value>(body)
                        .is_ok_and(|value| value.is_object()))
            {
                return Ok(routing::bad_request_response(
                    "library issue update body must be an object",
                ));
            }
            let issue_id =
                library_health_issue_id(path).expect("guarded library health issue path");
            let artist = extract_json_string_field(body, "artist");
            let title = extract_json_string_field(body, "title");
            let kind = extract_json_string_field(body, "kind")
                .or_else(|| extract_json_string_field(body, "mediaKind"));
            let _library_persistence = state.library_persistence_lock.lock().await;
            let mut library = state.library.write().await;
            let previous = library.clone();
            let patched = library.patch_health_issue(issue_id, artist, title, kind);
            let patched_item_id = patched
                .as_ref()
                .and_then(|value| value.get("item_id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let remaining = library.health_issues().len();
            let response = patched
                .map(|mut value| {
                    value["remaining"] = serde_json::json!(remaining);
                    routing::ok_response(value.to_string())
                })
                .unwrap_or_else(|| {
                    routing::ok_response(
                        serde_json::json!({
                            "id": issue_id,
                            "updated": false,
                            "status": "not_found",
                            "remaining": remaining,
                        })
                        .to_string(),
                    )
                });
            let mutated = library.clone();
            drop(library);
            if let Some(item_id) = patched_item_id {
                let library = state.library.read().await;
                let item = library.get(&item_id);
                drop(library);
                if let Some(item) = item {
                    if let Err(error) = persist_library_item_checked(state, &item).await {
                        rollback_library_if_unchanged(state, previous, &mutated).await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            }
            Ok(if versioned_contract {
                routing::no_content_response()
            } else {
                response
            })
        }

        ("POST", path)
            if path.starts_with("/api/webhooks/")
                && path.ends_with("/test")
                && webhook_test_id(path, "/api/webhooks/").is_some() =>
        {
            let webhook_id =
                webhook_test_id(path, "/api/webhooks/").expect("guarded webhook test path");
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

                state.spawn_managed_task(async move {
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
                    serde_json::json!({"status": "test_sent"}).to_string(),
                ))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }

        ("GET", path) if path.starts_with("/api/webhooks/") && path.ends_with("/logs") => {
            let Some(webhook_id) = path_segment_between(path, "/api/webhooks/", "/logs") else {
                return Ok(routing::not_found_response());
            };
            let limit = if let Some(q) = route.query {
                query_params(q)
                    .iter()
                    .find(|(k, _)| k == "limit")
                    .map(|(_, v)| parse_list_limit(v) as i32)
                    .unwrap_or(50)
            } else {
                50
            };

            if let Some(db) = &state.db {
                match db.get_webhook_logs(webhook_id, limit, 0).await {
                    Ok(logs) => {
                        let log_json = logs
                            .iter()
                            .map(|l| {
                                serde_json::json!({
                                    "id": l.id,
                                    "event": l.event,
                                    "correlation_id": l.correlation_id,
                                    "status": l.status,
                                    "response_status": l.response_status,
                                    "error_message": l.error_message,
                                    "timestamp": l.timestamp,
                                })
                            })
                            .collect::<Vec<_>>();

                        Ok(HttpResponse {
                            status: "200 OK",
                            content_type: "application/json",
                            body: serde_json::to_string(&serde_json::json!({"logs": log_json}))
                                .unwrap_or_else(|_| "{}".to_string()),
                        })
                    }
                    Err(_) => Ok(routing::bad_request_response("database error")),
                }
            } else {
                Ok(routing::bad_request_response("database not configured"))
            }
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
