async fn route_dispatch_group_1_telemetry(
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
        ("GET", "/api/config") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: effective_sanitized_config_json(state).await,
        }),
        ("GET", "/api/stats") => {
            let session = state.session.read().await;
            let session_stats = session.summary_json();
            drop(session);

            let _listeners = state.listeners.read().await;
            drop(_listeners);

            let shares = state.shares.read().await;
            let share_stats = shares.summary_json();
            drop(shares);

            let searches = state.searches.read().await;
            let searches_stats = searches.summary_json();
            drop(searches);

            let users = state.users.read().await;
            let users_stats = users.summary_json();
            drop(users);

            let browses = state.browse.read().await;
            let browses_stats = browses.summary_json();
            drop(browses);

            let messages = state.messages.read().await;
            let messages_stats = messages.summary_json();
            drop(messages);

            let rooms = state.rooms.read().await;
            let rooms_stats = rooms.summary_json();
            drop(rooms);

            let transfers = state.transfers.read().await;
            let transfers_stats = transfers.summary_json();
            drop(transfers);

            let database = database_stats_value(state).await;
            let body = serde_json::json!({
                "session": serde_json::from_str::<serde_json::Value>(&session_stats).unwrap_or_else(|_| serde_json::json!({})),
                "listeners": {"count": 1},
                "shares": serde_json::from_str::<serde_json::Value>(&share_stats).unwrap_or_else(|_| serde_json::json!({})),
                "searches": serde_json::from_str::<serde_json::Value>(&searches_stats).unwrap_or_else(|_| serde_json::json!({})),
                "users": serde_json::from_str::<serde_json::Value>(&users_stats).unwrap_or_else(|_| serde_json::json!({})),
                "browse": serde_json::from_str::<serde_json::Value>(&browses_stats).unwrap_or_else(|_| serde_json::json!({})),
                "messages": serde_json::from_str::<serde_json::Value>(&messages_stats).unwrap_or_else(|_| serde_json::json!({})),
                "rooms": serde_json::from_str::<serde_json::Value>(&rooms_stats).unwrap_or_else(|_| serde_json::json!({})),
                "transfers": serde_json::from_str::<serde_json::Value>(&transfers_stats).unwrap_or_else(|_| serde_json::json!({})),
                "database": database,
            })
            .to_string();

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
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
                Ok(controller_transfer_histogram_report(
                    route.query,
                    &transfers,
                ))
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
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
