use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_01(
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
        ("GET", "/api/telemetry") => {
            let session = state.session.read().await;
            let is_connected = session.state == "connected";
            let session_json = session.summary_json();
            drop(session);

            let listeners = state.listeners.read().await;
            let listeners_json = listeners.json();
            drop(listeners);

            let shares = state.shares.read().await;
            let shares_json = shares.summary_json();
            let share_cache_enabled = shares.cache_enabled;
            let share_cache_error = public_share_cache_error(shares.cache_error.as_deref());
            drop(shares);

            let searches = state.searches.read().await;
            let searches_json = searches.summary_json();
            drop(searches);

            let users = state.users.read().await;
            let users_json = users.summary_json();
            drop(users);

            let browse = state.browse.read().await;
            let browse_json = browse.summary_json();
            drop(browse);

            let messages = state.messages.read().await;
            let messages_json = messages.summary_json();
            drop(messages);

            let rooms = state.rooms.read().await;
            let rooms_json = rooms.summary_json();
            drop(rooms);

            let transfers = state.transfers.read().await;
            let transfers_json = transfers.summary_json();
            let transfer_state_healthy = transfers.state_error.is_none();
            let transfer_events_healthy = transfers.events_error.is_none();
            let transfer_state_error =
                public_transfer_state_error(transfers.state_error.as_deref());
            let transfer_events_error =
                public_transfer_events_error(transfers.events_error.as_deref());
            drop(transfers);

            let events = state.events.read().await;
            let event_count = events.records.len();
            let event_next_id = events.next_id;
            let event_history_limit = events.history_limit;
            drop(events);

            let runtime = state.runtime.read().await;
            let runtime_json = runtime.json_value();
            drop(runtime);

            let database = database_stats_value(state).await;
            let projections = database
                .get("projections")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            let value = serde_json::json!({
                "health": {
                    "connected": is_connected,
                    "database": database.get("healthy").and_then(serde_json::Value::as_bool).unwrap_or(false)
                        || !database.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(false),
                    "transferState": transfer_state_healthy,
                    "transferEvents": transfer_events_healthy,
                    "eventsBuffered": event_count,
                },
                "service": {
                    "name": "slskr",
                    "version": APP_VERSION,
                },
                "storage": {
                    "share_cache_file": "share-index.tsv",
                    "share_cache_kind": "compatibility-debug",
                    "share_cache_enabled": share_cache_enabled,
                    "share_cache_error": share_cache_error,
                    "transfer_events_file": "transfer-events.tsv",
                    "transfer_state_error": transfer_state_error,
                    "transfer_events_error": transfer_events_error,
                },
                "database": database,
                "projections": projections,
                "runtime": runtime_json,
                "session": serde_json::from_str::<serde_json::Value>(&session_json).unwrap_or_else(|_| serde_json::json!({})),
                "listeners": serde_json::from_str::<serde_json::Value>(&listeners_json).unwrap_or_else(|_| serde_json::json!({})),
                "shares": serde_json::from_str::<serde_json::Value>(&shares_json).unwrap_or_else(|_| serde_json::json!({})),
                "searches": serde_json::from_str::<serde_json::Value>(&searches_json).unwrap_or_else(|_| serde_json::json!({})),
                "users": serde_json::from_str::<serde_json::Value>(&users_json).unwrap_or_else(|_| serde_json::json!({})),
                "browse": serde_json::from_str::<serde_json::Value>(&browse_json).unwrap_or_else(|_| serde_json::json!({})),
                "messages": serde_json::from_str::<serde_json::Value>(&messages_json).unwrap_or_else(|_| serde_json::json!({})),
                "rooms": serde_json::from_str::<serde_json::Value>(&rooms_json).unwrap_or_else(|_| serde_json::json!({})),
                "transfers": serde_json::from_str::<serde_json::Value>(&transfers_json).unwrap_or_else(|_| serde_json::json!({})),
                "events": {
                    "total": event_count,
                    "next_id": event_next_id,
                    "history_limit": event_history_limit,
                },
            });

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: value.to_string(),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/summary") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = if route.path.starts_with("/api/v0/") {
                controller_versioned_transfer_summary_report(route.query, &transfers)
            } else {
                controller_transfer_summary_report(route.query, &transfers)
            };
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/telemetry/reports/transfers/histogram") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = if route.path.starts_with("/api/v0/") {
                controller_versioned_transfer_histogram_report(route.query, &transfers)
            } else {
                Ok(controller_transfer_histogram_report(route.query, &transfers))
            };
            drop(transfers);
            Ok(match body {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/leaderboard") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let result = controller_transfer_leaderboard_report(route.query, &transfers);
            drop(transfers);
            Ok(match result {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", path) if path.starts_with("/api/telemetry/reports/transfers/users/") => {
            let Some(username) =
                path_segment_after(path, "/api/telemetry/reports/transfers/users/")
            else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = controller_user_transfer_report(&username, &transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/telemetry/reports/transfers/exceptions") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let result = controller_transfer_exceptions_report(route.query, &transfers);
            drop(transfers);
            Ok(match result {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/exceptions/pareto") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let result = controller_transfer_exceptions_pareto_report(route.query, &transfers);
            drop(transfers);
            Ok(match result {
                Ok(body) => routing::ok_response(body),
                Err(error) => routing::bad_request_response(error),
            })
        }
        ("GET", "/api/telemetry/reports/transfers/directories") => {
            if let Some(response) =
                controller_telemetry_report_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = controller_transfer_directories_report(route.query, &transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/metrics") => {
            let (session_connected, share_files, share_bytes) = {
                let session = state.session.read().await;
                let session_connected = if session.state == "connected" { 1 } else { 0 };
                drop(session);
                let _listeners = state.listeners.read().await;
                drop(_listeners);
                let shares = state.shares.read().await;
                let share_files = shares.entries.len();
                let share_bytes: u64 = shares.entries.iter().map(|e| e.size).sum();
                (session_connected, share_files, share_bytes)
            };
            let active_searches = {
                let searches = state.searches.read().await;
                searches
                    .records
                    .iter()
                    .filter(|record| record.status == "active")
                    .count()
            };
            let watched_users = {
                let users = state.users.read().await;
                users.records.iter().filter(|record| record.watched).count()
            };
            let browse_count = {
                let browse = state.browse.read().await;
                browse.records.len()
            };
            let message_count = {
                let messages = state.messages.read().await;
                messages.records.len()
            };
            let joined_rooms = {
                let rooms = state.rooms.read().await;
                rooms.records.iter().filter(|record| record.joined).count()
            };
            let (transfer_count, active_transfers) = {
                let transfers = state.transfers.read().await;
                let active = transfers
                    .entries
                    .iter()
                    .filter(|entry| is_active_transfer_status(&entry.status))
                    .count();
                (transfers.entries.len(), active)
            };
            let event_count = {
                let events = state.events.read().await;
                events.records.len()
            };
            let runtime = state.runtime.read().await;
            let runtime_profile_invites_created = runtime.profile_invites_created;
            let runtime_cache_warm_runs = runtime.cache_warm_runs;
            let runtime_backfill_runs = runtime.backfill_runs;
            let runtime_songid_runs = runtime.songid_runs;
            let runtime_lidarr_sync_runs = runtime.lidarr_sync_runs;
            let runtime_lidarr_manual_imports = runtime.lidarr_manual_imports;
            drop(runtime);
            let (database_stats, database_stats_available) = if let Some(db) = state.db.as_ref() {
                match db.get_stats().await {
                    Ok(stats) => (Some(stats), 1),
                    Err(_) => (None, 0),
                }
            } else {
                (None, 0)
            };
            let database_enabled = if state.db.is_some() { 1 } else { 0 };
            let persisted_searches = database_stats
                .as_ref()
                .map(|stats| stats.search_count)
                .unwrap_or(0);
            let persisted_search_results = database_stats
                .as_ref()
                .map(|stats| stats.search_result_count)
                .unwrap_or(0);
            let persisted_transfers = database_stats
                .as_ref()
                .map(|stats| stats.transfer_count)
                .unwrap_or(0);
            let persisted_transfer_events = database_stats
                .as_ref()
                .map(|stats| stats.transfer_event_count)
                .unwrap_or(0);
            let persisted_shares = database_stats
                .as_ref()
                .map(|stats| stats.share_file_count)
                .unwrap_or(0);
            let persisted_events = database_stats
                .as_ref()
                .map(|stats| stats.event_count)
                .unwrap_or(0);

            let metrics = format!(
                "# HELP slskr_session_connected Session connection status\n\
                 # TYPE slskr_session_connected gauge\n\
                 slskr_session_connected {}\n\
                 # HELP slskr_shares_files Number of shared files\n\
                 # TYPE slskr_shares_files gauge\n\
                 slskr_shares_files {}\n\
                 # HELP slskr_shares_bytes Total bytes shared\n\
                 # TYPE slskr_shares_bytes gauge\n\
                 slskr_shares_bytes {}\n\
                 # HELP slskr_searches_active Active search count\n\
                 # TYPE slskr_searches_active gauge\n\
                 slskr_searches_active {}\n\
                 # HELP slskr_users_watched Watched user count\n\
                 # TYPE slskr_users_watched gauge\n\
                 slskr_users_watched {}\n\
                 # HELP slskr_browse_cache Browse cache size\n\
                 # TYPE slskr_browse_cache gauge\n\
                 slskr_browse_cache {}\n\
                 # HELP slskr_messages_total Message count\n\
                 # TYPE slskr_messages_total counter\n\
                 slskr_messages_total {}\n\
                 # HELP slskr_rooms_joined Joined room count\n\
                 # TYPE slskr_rooms_joined gauge\n\
                 slskr_rooms_joined {}\n\
                 # HELP slskr_transfers Transfer count\n\
                 # TYPE slskr_transfers gauge\n\
                 slskr_transfers{{state=\"total\"}} {}\n\
                 slskr_transfers{{state=\"active\"}} {}\n\
                 # HELP slskr_events_total Recorded event count\n\
                 # TYPE slskr_events_total counter\n\
                 slskr_events_total {}\n\
                 # HELP slskr_runtime_operations_total Runtime compatibility operation counters\n\
                 # TYPE slskr_runtime_operations_total counter\n\
                 slskr_runtime_operations_total{{operation=\"profile_invite\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"cache_warm\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"backfill\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"songid\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"lidarr_sync\"}} {}\n\
                 slskr_runtime_operations_total{{operation=\"lidarr_manual_import\"}} {}\n\
                 # HELP slskr_database_enabled SQLite persistence availability\n\
                 # TYPE slskr_database_enabled gauge\n\
                 slskr_database_enabled {}\n\
                 # HELP slskr_database_stats_available Whether SQLite statistics were collected successfully\n\
                 # TYPE slskr_database_stats_available gauge\n\
                 slskr_database_stats_available {}\n\
                 # HELP slskr_database_rows Persisted SQLite row counts by store\n\
                 # TYPE slskr_database_rows gauge\n\
                 slskr_database_rows{{store=\"searches\"}} {}\n\
                 slskr_database_rows{{store=\"search_results\"}} {}\n\
                 slskr_database_rows{{store=\"transfers\"}} {}\n\
                 slskr_database_rows{{store=\"transfer_events\"}} {}\n\
                 slskr_database_rows{{store=\"shares\"}} {}\n\
                 slskr_database_rows{{store=\"events\"}} {}\n",
                session_connected,
                share_files,
                share_bytes,
                active_searches,
                watched_users,
                browse_count,
                message_count,
                joined_rooms,
                transfer_count,
                active_transfers,
                event_count,
                runtime_profile_invites_created,
                runtime_cache_warm_runs,
                runtime_backfill_runs,
                runtime_songid_runs,
                runtime_lidarr_sync_runs,
                runtime_lidarr_manual_imports,
                database_enabled,
                database_stats_available,
                persisted_searches,
                persisted_search_results,
                persisted_transfers,
                persisted_transfer_events,
                persisted_shares,
                persisted_events
            );

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: metrics,
            })
        }
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
            if let Some(response) =
                controller_events_read_failure_response(state, route.path).await
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
                return Ok(if state.config.controller_profile
                    == ControllerProfile::Native
                {
                    routing::internal_server_error_response("Failed to raise event")
                } else {
                    routing::service_unavailable_response(&error)
                });
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
                Ok(routing::ok_response(serde_json::Value::Array(logs).to_string()))
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
             drop(webhooks);
             Ok(HttpResponse {
                 status: "200 OK",
                 content_type: "application/json",
                 body: serde_json::to_string(&serde_json::json!({"webhooks": webhook_list})).unwrap_or_else(|_| "{}".to_string()),
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

             Ok(routing::created_response(serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())))
         }

         ("DELETE", path)
             if path.starts_with("/api/webhooks/")
                 && webhook_resource_id(path, "/api/webhooks/").is_some() =>
         {
             let webhook_id = webhook_resource_id(path, "/api/webhooks/")
                 .expect("guarded webhook resource path");
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
                 Ok(routing::ok_response(serde_json::json!({"status": "deleted"}).to_string()))
             } else {
                 drop(webhooks);
                 Ok(routing::not_found_response())
             }
         }

         ("PATCH", path)
             if path.starts_with("/api/webhooks/")
                 && webhook_resource_id(path, "/api/webhooks/").is_some() =>
         {
              let webhook_id = webhook_resource_id(path, "/api/webhooks/")
                  .expect("guarded webhook resource path");
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
                  Ok(routing::ok_response(serde_json::to_string(&updated).unwrap_or_else(|_| "{}".to_string())))
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
              let issue_id = library_health_issue_id(path)
                  .expect("guarded library health issue path");
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
                      routing::ok_response(serde_json::json!({
                          "id": issue_id,
                          "updated": false,
                          "status": "not_found",
                          "remaining": remaining,
                      }).to_string())
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
             let webhook_id = webhook_test_id(path, "/api/webhooks/")
                 .expect("guarded webhook test path");
             let webhooks = state.webhooks.read().await;
             if let Some(webhook) = webhooks.get(webhook_id) {
                 let payload = webhooks::WebhookDispatcher::test_payload(
                     webhooks::WebhookEvent::SearchCreated,
                     "test webhook delivery"
                 );
                 let webhook_clone = webhook.clone();
                 drop(webhooks);
                 let Ok(delivery_permit) = Arc::clone(&state.webhook_deliveries).try_acquire_owned() else {
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

                 Ok(routing::ok_response(serde_json::json!({"status": "test_sent"}).to_string()))
             } else {
                 drop(webhooks);
                 Ok(routing::not_found_response())
             }
         }

         ("GET", path) if path.starts_with("/api/webhooks/") && path.ends_with("/logs") => {
             let Some(webhook_id) =
                 path_segment_between(path, "/api/webhooks/", "/logs")
             else {
                 return Ok(routing::not_found_response());
             };
             let limit = if let Some(q) = route.query {
                 query_params(q).iter().find(|(k, _)| k == "limit").map(|(_, v)| parse_list_limit(v) as i32).unwrap_or(50)
             } else {
                 50
             };

             if let Some(db) = &state.db {
                 match db.get_webhook_logs(webhook_id, limit, 0).await {
                     Ok(logs) => {
                         let log_json = logs.iter().map(|l| {
                             serde_json::json!({
                                 "id": l.id,
                                 "event": l.event,
                                 "correlation_id": l.correlation_id,
                                 "status": l.status,
                                 "response_status": l.response_status,
                                 "error_message": l.error_message,
                                 "timestamp": l.timestamp,
                             })
                         }).collect::<Vec<_>>();

                         Ok(HttpResponse {
                             status: "200 OK",
                             content_type: "application/json",
                             body: serde_json::to_string(&serde_json::json!({"logs": log_json})).unwrap_or_else(|_| "{}".to_string()),
                         })
                     }
                     Err(_) => Ok(routing::bad_request_response("database error")),
                 }
             } else {
                 Ok(routing::bad_request_response("database not configured"))
             }
         }

         ("GET", "/api/shares") => {
            let shares = state.shares.read().await;
            let mut roots = shares
                .roots
                .iter()
                .map(controller_share_value)
                .collect::<Vec<_>>();
            if roots.is_empty() && !shares.entries.is_empty() {
                roots.push(serde_json::json!({
                    "localPath": "shares",
                    "id": "shares",
                    "alias": "shares",
                    "raw": "shares",
                    "remotePath": "shares",
                    "directories": 0,
                    "files": shares.entries.len(),
                    "bytes": shares.entries.iter().map(|entry| entry.size).sum::<u64>(),
                    "isExcluded": false,
                }));
            }
            let json = serde_json::json!({ "local": roots }).to_string();
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }
        ("GET", "/api/shares/catalog") => {
            let shares = state.shares.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: shares.catalog_json(route.query),
            })
        }
        ("PUT", "/api/shares") => {
            let rebuilt = match rebuild_share_index(state).await {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(share_rebuild_error_response(&error)),
            };
            let json = rebuilt.json();
            record_event(state, "share.scan.completed", "shares", None).await;
            if route.path.starts_with("/api/v0/") {
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "",
                    body: String::new(),
                });
            }
            Ok(routing::ok_response((!json.is_empty()).to_string()))
        }
        ("GET", "/api/files/downloads/directories")
        | ("GET", "/api/files/incomplete/directories")
        | ("GET", "/api/v0/files/downloads/directories")
        | ("GET", "/api/v0/files/incomplete/directories") => {
            if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            )
                && query_bool_is_invalid(route.query, "recursive")
            {
                return Ok(routing::bad_request_response(
                    "The recursive query value must be a boolean",
                ));
            }
            let root = if normalized_path.contains("/files/downloads/") {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            if state.config.controller_profile == ControllerProfile::Legacy
                && !root.is_dir()
            {
                return Ok(file_storage_error_response(STORAGE_DIRECTORY_NOT_FOUND_ERROR));
            }
            let options = StorageDirectoryListOptions::from_query(route.query);
            match controller_storage_directory_json(&root, None, options) {
                Ok(json) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: target_storage_directory_json(
                        json,
                        state.config.controller_profile,
                    ),
                }),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("GET", path)
            if (path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/incomplete/directories/")
                || path.starts_with("/api/v0/files/downloads/directories/")
                || path.starts_with("/api/v0/files/incomplete/directories/")) =>
        {
            let Some((storage, resource, encoded_name)) = controller_file_storage_resource_path(path)
            else {
                return Ok(routing::not_found_response());
            };
            if resource != "directories" {
                return Ok(routing::not_found_response());
            }
            if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            )
                && query_bool_is_invalid(route.query, "recursive")
            {
                return Ok(routing::bad_request_response(
                    "The recursive query value must be a boolean",
                ));
            }
            let root = if storage == "downloads" {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            let options = StorageDirectoryListOptions::from_query(route.query);
            match controller_storage_directory_json(&root, Some(encoded_name), options) {
                Ok(json) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: target_storage_directory_json(
                        json,
                        state.config.controller_profile,
                    ),
                }),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/downloads/files/")
                || path.starts_with("/api/files/incomplete/directories/")
                || path.starts_with("/api/files/incomplete/files/")
                || path.starts_with("/api/v0/files/downloads/directories/")
                || path.starts_with("/api/v0/files/downloads/files/")
                || path.starts_with("/api/v0/files/incomplete/directories/")
                || path.starts_with("/api/v0/files/incomplete/files/") =>
        {
            if !effective_remote_file_management(state) {
                return Ok(HttpResponse {
                    status: "403 Forbidden",
                    content_type: "",
                    body: String::new(),
                });
            }
            let Some((storage, resource, encoded_name)) = controller_file_storage_resource_path(path)
            else {
                return Ok(routing::not_found_response());
            };
            let root = if storage == "downloads" {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            let delete_result = if resource == "directories" {
                delete_scoped_file_storage_path(&root, encoded_name, true)
            } else {
                delete_scoped_file_storage_path(&root, encoded_name, false)
            };
            match delete_result {
                Ok(true) => Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                }),
                Ok(false)
                    if resource == "files"
                        && matches!(
                            state.config.controller_profile,
                            ControllerProfile::Legacy
                                | ControllerProfile::Native
                        ) => Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                }),
                Ok(false) => Ok(routing::not_found_response()),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("GET", path) if path.starts_with("/api/files/") || path.starts_with("/api/v0/files/") => {
            let root_label = path
                .strip_prefix("/api/v0/files/")
                .or_else(|| path.strip_prefix("/api/files/"))
                .unwrap_or("");

            if root_label.is_empty() {
                return Ok(routing::not_found_response());
            }

            let mut extension_filter: Option<String> = None;
            let mut selected_folder = String::new();
            let mut folder_requested = false;
            let mut recursive = false;
            for (name, value) in query_params(route.query.unwrap_or_default()) {
                match name.as_str() {
                    "extension" => extension_filter = non_empty(value),
                    "folder" | "path" | "prefix" => {
                        folder_requested = true;
                        selected_folder = value.trim_matches('/').to_owned();
                    }
                    "recursive" => recursive = parse_bool_value(&value).unwrap_or(false),
                    _ => {}
                }
            }

            let filter = RecordListFilter::from_query(route.query);
            let shares = state.shares.read().await;

            let Some(root) = shares.roots.iter().find(|r| r.label == root_label) else {
                drop(shares);
                return Ok(routing::not_found_response());
            };

            let base_prefix = if selected_folder.is_empty() {
                root_label.to_owned()
            } else {
                format!("{}/{}", root_label, selected_folder)
            };
            let root_prefix = format!("{root_label}/");
            let base_child_prefix = format!("{base_prefix}/");
            let q = filter.q.as_deref();
            let folder_mode = folder_requested || recursive;

            let root_entries = shares
                .entries
                .iter()
                .filter(|entry| entry.filename.starts_with(&root_prefix))
                .collect::<Vec<_>>();

            let mut directory_summaries = BTreeMap::<String, (usize, u64)>::new();
            for entry in &root_entries {
                let Some(relative_to_base) = entry.filename.strip_prefix(&base_child_prefix) else {
                    continue;
                };
                let Some((child, _)) = relative_to_base.split_once('/') else {
                    continue;
                };
                if child.is_empty() {
                    continue;
                }
                let directory_path = if selected_folder.is_empty() {
                    child.to_owned()
                } else {
                    format!("{selected_folder}/{child}")
                };
                if q.is_some_and(|q| {
                    !directory_path.to_ascii_lowercase().contains(q)
                        && !format!("{root_label}/{directory_path}")
                            .to_ascii_lowercase()
                            .contains(q)
                }) {
                    continue;
                }
                if extension_filter
                    .as_deref()
                    .is_some_and(|ext| entry.extension != ext)
                {
                    continue;
                }
                let summary = directory_summaries.entry(directory_path).or_default();
                summary.0 += 1;
                summary.1 += entry.size;
            }

            let mut entries: Vec<_> = root_entries
                .into_iter()
                .filter(|entry| {
                    if folder_mode {
                        if recursive {
                            entry.filename.starts_with(&base_child_prefix)
                        } else {
                            virtual_folder(&entry.filename) == base_prefix
                        }
                    } else {
                        entry.filename.starts_with(&root_prefix)
                    }
                })
                .filter(|e| {
                    extension_filter.as_deref()
                        .is_none_or(|ext| e.extension == ext)
                })
                .filter(|entry| {
                    q.is_none_or(|q| {
                        entry
                            .filename
                            .strip_prefix(&root_prefix)
                            .unwrap_or(&entry.filename)
                            .to_ascii_lowercase()
                            .contains(q)
                            || entry.filename.to_ascii_lowercase().contains(q)
                    })
                })
                .collect();

            let filtered_count = entries.len();
            let directory_count = directory_summaries.len();
            let total_bytes = entries.iter().map(|entry| entry.size).sum::<u64>();

            entries = entries
                .into_iter()
                .skip(filter.offset)
                .take(filter.limit.unwrap_or(usize::MAX))
                .collect();

            let entries_json = entries
                .iter()
                .map(|entry| {
                    let path = if folder_mode {
                        entry
                            .filename
                            .strip_prefix(&base_child_prefix)
                            .unwrap_or("")
                    } else {
                        entry.filename.strip_prefix(&root_prefix).unwrap_or("")
                    };
                    format!(
                        "{{\"type\":\"file\",\"path\":\"{}\",\"virtual_path\":\"{}\",\"size\":{},\"extension\":\"{}\"}}",
                        json_escape(path),
                        json_escape(&entry.filename),
                        entry.size,
                        json_escape(&entry.extension)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let directories_json = directory_summaries
                .iter()
                .map(|(directory, (file_count, total_bytes))| {
                    let path = if selected_folder.is_empty() {
                        directory.as_str()
                    } else {
                        directory
                            .strip_prefix(&format!("{selected_folder}/"))
                            .unwrap_or(directory)
                    };
                    format!(
                        "{{\"type\":\"directory\",\"name\":\"{}\",\"path\":\"{}\",\"virtual_path\":\"{}/{}\",\"file_count\":{},\"total_bytes\":{}}}",
                        json_escape(path),
                        json_escape(path),
                        json_escape(root_label),
                        json_escape(directory),
                        file_count,
                        total_bytes
                    )
                })
                .collect::<Vec<_>>()
                .join(",");

            let response_body = format!(
                "{{\"label\":\"{}\",\"folder\":{},\"recursive\":{},\"entries\":[{}],\"directories\":[{}],\"count\":{},\"filtered_count\":{},\"directory_count\":{},\"total_bytes\":{},\"offset\":{},\"limit\":{}}}",
                json_escape(&root.label),
                json_option((!selected_folder.is_empty()).then_some(selected_folder.as_str())),
                recursive,
                entries_json,
                directories_json,
                root.files,
                filtered_count,
                directory_count,
                total_bytes,
                filter.offset,
                json_usize_option(filter.limit)
            );

            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: response_body,
            })
        }
        ("POST", "/api/shares/rescan") => {
            let snapshot = match rebuild_share_index(state).await {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(share_rebuild_error_response(&error)),
            };
            record_event(
                state,
                "share.scan.completed",
                "shares",
                Some(format!("{} files", snapshot.entries.len())),
            )
            .await;
            Ok(HttpResponse {
                status: "202 Accepted",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("GET", "/api/session") => {
            let snapshot = state.session.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("POST", "/api/session/connect") => {
            if let Err(error) = send_session_command(state, SessionCommand::Connect).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/ping") => {
            if let Err(error) = send_session_command(state, SessionCommand::Ping).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/disconnect") => {
            if let Err(error) = send_session_command(state, SessionCommand::Disconnect).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/privileges/check") => {
            if let Err(error) = send_session_command(state, SessionCommand::CheckPrivileges).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("GET", "/api/listeners") => {
            let snapshot = state.listeners.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("GET", "/api/users") => {
            let users = state.users.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: users.json(),
            })
        }
        ("GET", "/api/searches/records") => {
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let expired = searches.expire_due();
            let body = searches.json(route.query);
            let mutated_searches = searches.clone();
            drop(searches);
            persist_expired_searches_with_rollback(
                state,
                previous_searches,
                &mutated_searches,
                &expired,
            )
            .await?;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", "/api/searches") => {
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let expired = searches.expire_due();
            let body = searches.controller_list_json(route.query);
            let mutated_searches = searches.clone();
            drop(searches);
            persist_expired_searches_with_rollback(
                state,
                previous_searches,
                &mutated_searches,
                &expired,
            )
            .await?;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", _path) if search_token_path(normalized_path.as_str(), "").is_some() => {
            let Some(token) = search_token_path(normalized_path.as_str(), "") else {
                return Ok(routing::not_found_response());
            };
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let expired = searches.expire_due();
            if let Some(record) = searches.get(token) {
                let body = record.json_with_query(route.query);
                let mutated_searches = searches.clone();
                drop(searches);
                persist_expired_searches_with_rollback(
                    state,
                    previous_searches,
                    &mutated_searches,
                    &expired,
                )
                .await?;
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json",
                    body,
                })
            } else {
                let mutated_searches = searches.clone();
                drop(searches);
                persist_expired_searches_with_rollback(
                    state,
                    previous_searches,
                    &mutated_searches,
                    &expired,
                )
                .await?;
                Ok(routing::not_found_response())
            }
        }
        ("GET", "/api/rooms") => {
            let rooms = state.rooms.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: rooms.json(route.query),
            })
        }
        ("GET", "/api/messages") => {
            let messages = state.messages.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: messages.json(route.query),
            })
        }
        ("GET", "/api/transfers") if route.path.starts_with("/api/v0/") => {
            let transfers = state.transfers.read().await;
            let downloads = serde_json::from_str::<Vec<serde_json::Value>>(
                &transfers.controller_transfers_json(0, None),
            )
            .unwrap_or_default();
            let uploads = serde_json::from_str::<Vec<serde_json::Value>>(
                &transfers.controller_transfers_json(1, None),
            )
            .unwrap_or_default();
            Ok(routing::ok_response(
                serde_json::Value::Array(downloads.into_iter().chain(uploads).collect())
                    .to_string(),
            ))
        }
        ("GET", "/api/transfers") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.json(route.query),
            })
        }
        ("GET", "/api/transfers/stats") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.stats_json(),
            })
        }
        ("POST", "/api/searches") => {
            if route.path.starts_with("/api/v0/")
                && extract_json_string_field(body, "acquisitionProfile").is_some_and(|profile| {
                    !is_known_acquisition_profile(&profile)
                })
            {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!(
                        "The field AcquisitionProfile must be a known acquisition profile"
                    )
                    .to_string(),
                });
            }
            let query = match extract_json_string_field(body, "query")
                .or_else(|| extract_json_string_field(body, "searchText"))
            {
                Some(q) => q,
                None => return Ok(routing::bad_request_response("query/searchText is required")),
            };

            let target_str = extract_json_string_field(body, "target").unwrap_or_else(|| "global".to_string());
            let external_id = extract_json_string_field(body, "id");
            let username_opt = extract_json_string_field(body, "username");
            let room_opt = extract_json_string_field(body, "room");
            let wishlist_item_id = extract_json_string_field(body, "wishlistItemId")
                .or_else(|| extract_json_string_field(body, "wishlist_item_id"));
            let ttl_seconds = match search_ttl_seconds_from_body(body) {
                Ok(ttl_seconds) => ttl_seconds,
                Err(error) => return Ok(routing::bad_request_response(error)),
            };

            if !matches!(target_str.as_str(), "global" | "user" | "room" | "wishlist") {
                return Ok(routing::bad_request_response("invalid search target"));
            }
            if target_str == "user" && username_opt.is_none() {
                return Ok(routing::bad_request_response("username is required for user search"));
            }
            if target_str == "room" && room_opt.is_none() {
                return Ok(routing::bad_request_response("room is required for room target"));
            }

            let shares = state.shares.read().await;
            let matching_results = search_shares(&shares.entries, &query);
            drop(shares);

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

             let mut searches = state.searches.write().await;
             let previous_searches = searches.clone();
             let target_name = if target_str == "user" {
                 username_opt.clone()
             } else if target_str == "room" {
                 room_opt.clone()
             } else if target_str == "wishlist" {
                 wishlist_item_id.clone()
             } else {
                 None
             };
             let result_count = matching_results.len();

              let target = search_target_static(&target_str);
              let outcome = match searches.create(external_id, query.clone(), target, target_name.clone(), matching_results, ttl_seconds) {
                  Ok(outcome) => outcome,
                  Err(error) => return Ok(search_create_error_response(error)),
              };
              let record = outcome.record;
              let evicted = outcome.evicted;
              let expired = outcome.expired;
              let token = record.token;
              let mutated_searches = searches.clone();

             let dispatch_target = match target_str.as_str() {
                 "user" => SearchDispatchTarget::User(username_opt.clone().unwrap_or_default()),
                 "room" => SearchDispatchTarget::Room(room_opt.clone().unwrap_or_default()),
                "wishlist" => SearchDispatchTarget::Wishlist,
                _ => SearchDispatchTarget::Global,
             };
             drop(searches);

             let mut upserts = expired.clone();
             upserts.push(record.clone());
             if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
                 rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                 return Ok(routing::service_unavailable_response(&error));
             }
             for expired_record in &expired {
                 publish_search_hub_event(state, "update", expired_record);
             }
             session_command_permit.send(SessionCommand::Search {
                 token,
                 query: query.clone(),
                 target: dispatch_target,
             });

             record_event(state, "search.started", format!("{}", token), None).await;

             // Dispatch webhook for search.created event
             let webhook_data = serde_json::json!({
                 "token": token,
                 "query": query,
                 "target": target_str,
                 "target_name": target_name,
                 "result_count": result_count,
             });
             let correlation_id = format!("search_{}", token);
              dispatch_webhook_event(
                  state,
                  correlation_id,
                  webhooks::WebhookEvent::SearchCreated,
                  webhook_data,
              ).await;

              // Return simplified response matching C# compatibility contract
              let search_id = record.id.replace("-", "");
              let results_json = record
                  .results
                  .iter()
                  .map(|r| r.json())
                  .collect::<Vec<_>>()
                  .join(",");
              let response_body = format!(
                  r#"{{"searchId":"{}","query":"{}","results":[{}]}}"#,
                  json_escape(&search_id),
                  json_escape(&record.query),
                  results_json
              );
              Ok(routing::ok_response(response_body))
        }

        ("POST", _path) if search_token_path(normalized_path.as_str(), "/complete").is_some() => {
            let Some(token) = search_token_path(normalized_path.as_str(), "/complete") else {
                return Ok(routing::not_found_response());
            };

            // A wishlist search that completes with too few usable results gets
            // one of the current smart-fallback queries immediately.  The
            // session loop also handles this at expiry time, but waiting for the
            // five-second TTL here makes manual/API completion observably slower
            // and loses the current upstream behavior.
            let fallback_query = {
                let record = state.searches.read().await.get(token);
                if record.as_ref().is_some_and(|record| {
                    record.status == "active"
                        && search_fallback::is_enabled_for_source(record.target)
                }) {
                    let record = record.expect("active wishlist search record");
                    let wishlist_policy = if let Some(item_id) = record.wishlist_item_id() {
                        state.wishlist.read().await.result_policy_for(item_id)
                    } else {
                        None
                    };
                    let response_limit = wishlist_policy
                        .as_ref()
                        .map(|policy| policy.max_results)
                        .unwrap_or(MAX_SEARCH_RESULTS_PER_SEARCH);
                    let file_count = record
                        .results
                        .len()
                        .saturating_add(record.hidden_locked_count);
                    (search_fallback::needs_fallback(
                        record.raw_response_count,
                        file_count,
                        response_limit,
                        response_limit,
                    ))
                    .then(|| search_fallback::create_queries(&record.query))
                    .and_then(|queries| queries.get(record.fallback_attempts).cloned())
                } else {
                    None
                }
            };

            if let Some(fallback_query) = fallback_query {
                let (previous_record, fallback_record) = {
                    let mut searches = state.searches.write().await;
                    let previous_record = searches.get(token);
                    let fallback_record = searches.reset_for_fallback(token, fallback_query, 5);
                    (previous_record, fallback_record)
                };
                if let Some(fallback_record) = fallback_record {
                    let body_json = fallback_record.json();
                    if let Err(error) = persist_search_record(state, &fallback_record).await {
                        if let Some(previous_record) = previous_record.as_ref() {
                            rollback_search_record_if_unchanged(
                                state,
                                previous_record,
                                &fallback_record,
                            )
                            .await;
                        }
                        return Err(error);
                    }
                    record_event(
                        state,
                        "wishlist.search.fallback_started",
                        fallback_record.token.to_string(),
                        Some(
                            serde_json::json!({
                                "query": fallback_record.query,
                                "attempt": fallback_record.fallback_attempts,
                                "source": "completion",
                            })
                            .to_string(),
                        ),
                    )
                    .await;
                    if let Err(error) = send_session_command(
                        state,
                        SessionCommand::Search {
                            token: fallback_record.token,
                            query: fallback_record.query,
                            target: SearchDispatchTarget::Wishlist,
                        },
                    )
                    .await
                    {
                        update_session(state, |snapshot| {
                            snapshot.last_error = Some(error.clone());
                        })
                        .await;
                    }
                    return Ok(routing::ok_response(body_json));
                }
            }

            let mut searches = state.searches.write().await;
            if let Some((record, transitioned)) = searches.complete(token) {
                let body_json = record.json();

                // Dispatch webhook for search.completed event
                let result_count = record.results.len();
                let webhook_data = serde_json::json!({
                    "token": token,
                    "query": record.query,
                    "result_count": result_count,
                    "target": record.target,
                });
                let correlation_id = format!("search_{}", token);

                drop(searches);
                if transitioned {
                    persist_search_record(state, &record).await?;
                    publish_search_hub_event(state, "update", &record);
                }
                if transitioned && record.wishlist_item_id().is_some() {
                    {
                        let _wishlist_search_persistence =
                            state.wishlist_search_persistence_lock.lock().await;
                        let mut wishlist = state.wishlist.write().await;
                        let previous = wishlist.clone();
                        if let Some(item) = wishlist.record_completed_search(&record) {
                            let mutated = wishlist.clone();
                            drop(wishlist);
                            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                                return Ok(routing::service_unavailable_response(&error));
                            }
                        }
                    }
                    if let Err(error) = auto_download_completed_wishlist(state, &record).await {
                        update_session(state, |snapshot| {
                            snapshot.last_error = Some(error.clone());
                        })
                        .await;
                        record_daemon_log(
                            state,
                            logging::LogLevel::Warn,
                            "wishlist",
                            error,
                        )
                        .await;
                    }
                }

                if transitioned {
                    dispatch_webhook_event(
                        state,
                        correlation_id,
                        webhooks::WebhookEvent::SearchCompleted,
                        webhook_data,
                    )
                    .await;
                }

                Ok(routing::ok_response(body_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
