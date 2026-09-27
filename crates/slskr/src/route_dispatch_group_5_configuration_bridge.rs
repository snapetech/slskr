async fn route_dispatch_group_5_configuration_bridge(
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
        ("GET", "/api/config/preferences") => {
            let runtime = state.runtime.read().await;
            let body = serde_json::json!({
                "auto_connect": state.config.auto_connect,
                "transfer_allow_outbound": state.config.transfer_allow_outbound,
                "transfer_max_active": state.config.transfer_max_active,
                "autoreplace_enabled": runtime.autoreplace_enabled,
            })
            .to_string();
            drop(runtime);
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/config/preferences") => {
            let requested = extract_json_bool_field(body, "autoreplace_enabled")
                .or_else(|| extract_json_bool_field(body, "autoreplaceEnabled"));
            let response = match mutate_runtime_compat_state(state, |runtime, _| {
                if let Some(enabled) = requested {
                    runtime.set_autoreplace(enabled);
                }
                serde_json::json!({
                    "auto_connect": state.config.auto_connect,
                    "transfer_allow_outbound": state.config.transfer_allow_outbound,
                    "transfer_max_active": state.config.transfer_max_active,
                    "autoreplace_enabled": runtime.autoreplace_enabled,
                    "persisted": true,
                    "updated_at": runtime.updated_at,
                })
                .to_string()
            })
            .await
            {
                Ok(response) => response,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(response))
        }

        // ADDITIONAL MISSING PUT ENDPOINTS (Phase 5)
        ("PUT", "/api/autoreplace/disable") => {
            let versioned = route.path.starts_with("/api/v0/");
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let legacy = runtime.set_autoreplace(false);
                if versioned {
                    serde_json::json!({
                        "enabled": false,
                        "lastRunAt": null,
                        "lastRunProcessedCount": 0,
                        "lastRunReplacedCount": 0,
                        "intervalSeconds": 300,
                    })
                    .to_string()
                } else {
                    legacy.to_string()
                }
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/autoreplace/enable") => {
            let versioned = route.path.starts_with("/api/v0/");
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let legacy = runtime.set_autoreplace(true);
                if versioned {
                    serde_json::json!({
                        "enabled": true,
                        "lastRunAt": null,
                        "lastRunProcessedCount": 0,
                        "lastRunReplacedCount": 0,
                        "intervalSeconds": 300,
                    })
                    .to_string()
                } else {
                    legacy.to_string()
                }
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        // ADDITIONAL MISSING BRIDGE ENDPOINTS (Phase 6)
        ("GET", "/api/bridge/admin/clients") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let status = if runtime.bridge_running {
                "running"
            } else if bridge.enabled {
                "configured"
            } else {
                "disabled"
            };
            // Matches the oracle's real BridgeDashboard client snapshot;
            // only sessions accepted by the embedded Soulfind-protocol
            // listener are included.
            let clients = runtime
                .bridge_active_clients
                .values()
                .cloned()
                .collect::<Vec<_>>();
            if state.config.controller_profile == ControllerProfile::Native {
                drop(runtime);
                return Ok(routing::ok_response(
                    serde_json::json!({"clients": clients}).to_string(),
                ));
            }
            let json = serde_json::json!({
                "clients": clients,
                "count": runtime.bridge_active_clients.len(),
                "status": status,
                "ready": bridge.enabled,
            });
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/admin/config") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            Ok(routing::ok_response(
                serde_json::json!({
                    "enabled": bridge.enabled,
                    "port": bridge.port,
                    "soulfind_path": "soulfind",
                    "max_clients": bridge.max_clients,
                    "require_auth": bridge.require_auth,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/bridge/admin/dashboard") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let active_transfers = transfers
                .entries
                .iter()
                .filter(|entry| is_queued_or_active_transfer_status(&entry.status))
                .count();
            let bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let transfer_count = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let started_at = runtime
                    .bridge_started_at
                    .map(bridge_started_at_string)
                    .unwrap_or_else(|| "0001-01-01T00:00:00+00:00".to_owned());
                let uptime = runtime
                    .bridge_started_at
                    .map(|started| {
                        format_timespan_hms(
                            i64::try_from(unix_timestamp().saturating_sub(started))
                                .unwrap_or(i64::MAX),
                        )
                    })
                    .unwrap_or_else(|| "00:00:00".to_owned());
                let json = serde_json::json!({
                    "health": {
                        "isHealthy": runtime.bridge_running,
                        "version": "1.0.0-proxy",
                        "activeConnections": runtime.bridge_active_clients.len(),
                        "startedAt": started_at,
                    },
                    "connectedClients": [],
                    "stats": {
                        "totalConnections": runtime.bridge_total_connections,
                        "currentConnections": runtime.bridge_active_clients.len(),
                        "totalSearches": runtime.bridge_total_searches,
                        "totalDownloads": runtime.bridge_total_downloads,
                        "totalRoomJoins": runtime.bridge_total_room_joins,
                        "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                        "uptime": uptime,
                    },
                    "meshBenefits": {
                        "bytesViaMesh": 0,
                        "bytesViaSoulseek": 0,
                        "meshPercentage": 0.0,
                        "disasterModeActivations": 0,
                        "timeInDisasterMode": "00:00:00",
                    },
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            // Matches the oracle's real BridgeDashboardData contract:
            // Local HTTP transfer activity remains separate from the
            // protocol bridge counters below.
            let json = serde_json::json!({
                "health": if runtime.bridge_running { "Healthy" } else { "Disabled" },
                "connectedClients": runtime.bridge_active_clients.len(),
                "stats": {
                    "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                    "totalConnections": runtime.bridge_total_connections,
                    "currentConnections": runtime.bridge_active_clients.len(),
                    "totalSearches": runtime.bridge_total_searches,
                    "totalDownloads": runtime.bridge_total_downloads,
                    "totalRoomJoins": runtime.bridge_total_room_joins,
                    "uptime": runtime.bridge_started_at.map(|started| format_timespan_hms(
                        i64::try_from(unix_timestamp().saturating_sub(started)).unwrap_or(i64::MAX)
                    )).unwrap_or_else(|| "00:00:00".to_owned()),
                },
                "meshBenefits": {"enabled": bridge.enabled},
                "active_clients": runtime.bridge_active_clients.len(),
                "transfers": transfer_count,
                "active_transfers": active_transfers,
                "total_bytes": bytes,
                "uptime_seconds": 0,
                "enabled": bridge.enabled,
                "running": runtime.bridge_running,
                "configUpdates": runtime.bridge_config_updates,
                "host": serde_json::Value::Null,
                "port": serde_json::Value::Null,
                "endpoint_configured": bridge.endpoint_configured(),
            });
            drop(runtime);
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/admin/stats") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let total_bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let active_sessions = transfers
                .entries
                .iter()
                .filter(|entry| is_queued_or_active_transfer_status(&entry.status))
                .count();
            let total_requests = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let uptime = runtime
                    .bridge_started_at
                    .map(|started| {
                        format_timespan_hms(
                            i64::try_from(unix_timestamp().saturating_sub(started))
                                .unwrap_or(i64::MAX),
                        )
                    })
                    .unwrap_or_else(|| "00:00:00".to_owned());
                let json = serde_json::json!({
                    "totalConnections": runtime.bridge_total_connections,
                    "currentConnections": runtime.bridge_active_clients.len(),
                    "totalSearches": runtime.bridge_total_searches,
                    "totalDownloads": runtime.bridge_total_downloads,
                    "totalRoomJoins": runtime.bridge_total_room_joins,
                    "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                    "uptime": uptime,
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            // Local HTTP transfer activity remains separate from the
            // protocol bridge counters below.
            let json = serde_json::json!({
                "totalConnections": runtime.bridge_total_connections,
                "currentConnections": runtime.bridge_active_clients.len(),
                "totalSearches": runtime.bridge_total_searches,
                "totalDownloads": runtime.bridge_total_downloads,
                "totalRoomJoins": runtime.bridge_total_room_joins,
                "totalBytesProxied": runtime.bridge_total_bytes_proxied,
                "uptime": runtime.bridge_started_at.map(|started| format_timespan_hms(
                    i64::try_from(unix_timestamp().saturating_sub(started)).unwrap_or(i64::MAX)
                )).unwrap_or_else(|| "00:00:00".to_owned()),
                "total_requests": total_requests,
                "total_bytes": total_bytes,
                "active_sessions": active_sessions,
                "enabled": bridge.enabled,
                "running": runtime.bridge_running,
                "configUpdates": runtime.bridge_config_updates,
            });
            drop(runtime);
            Ok(routing::ok_response(json.to_string()))
        }

        ("GET", "/api/bridge/status") => {
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let runtime = state.runtime.read().await;
            let transfers = state.transfers.read().await;
            let transfer_count = transfers.entries.len();
            drop(transfers);
            if state.config.controller_profile == ControllerProfile::Native {
                let started_at = runtime
                    .bridge_started_at
                    .map(bridge_started_at_string)
                    .unwrap_or_else(|| "0001-01-01T00:00:00+00:00".to_owned());
                let json = serde_json::json!({
                    "isHealthy": runtime.bridge_running,
                    "version": "1.0.0-proxy",
                    "activeConnections": runtime.bridge_active_clients.len(),
                    "startedAt": started_at,
                });
                drop(runtime);
                return Ok(routing::ok_response(json.to_string()));
            }
            let uptime_seconds = runtime
                .bridge_started_at
                .map(|started| unix_timestamp().saturating_sub(started))
                .unwrap_or(0);
            let json = format!(
                 "{{\"status\":\"{}\",\"version\":\"1.0.0\",\"uptime_seconds\":{},\"enabled\":{},\"configured\":{},\"running\":{},\"configUpdates\":{},\"host\":\"{}\",\"port\":{},\"endpoint_configured\":{},\"transfers\":{},\"next_action\":\"{}\"}}",
                 if runtime.bridge_running { "running" } else if bridge.enabled { "configured" } else { "disabled" },
                 uptime_seconds,
                 bridge.enabled,
                 bridge.enabled && bridge.endpoint_configured(),
                 runtime.bridge_running,
                 runtime.bridge_config_updates,
                 bridge.bind_address,
                 bridge.port,
                 bridge.endpoint_configured(),
                 transfer_count,
                 if runtime.bridge_running {
                     "accept bridge traffic"
                 } else if bridge.enabled {
                     "start bridge service"
                 } else {
                     "enable bridge integration"
                 }
             );
            drop(runtime);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/bridge/transfer/") => {
            Ok(bridge_transfer_progress_response(path, state).await)
        }

        ("POST", "/api/bridge/start") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                return Ok(routing::ok_response(
                    serde_json::json!({"status": "started"}).to_string(),
                ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.set_bridge_running(true, bridge.enabled);
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "next_action".to_owned(),
                        serde_json::json!(if bridge.enabled {
                            "accept bridge traffic"
                        } else {
                            "enable bridge integration"
                        }),
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("POST", "/api/bridge/stop") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                return Ok(routing::ok_response(
                    serde_json::json!({"status": "stopped"}).to_string(),
                ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime
                    .set_bridge_running(false, bridge.enabled)
                    .to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        ("PUT", "/api/bridge/admin/config") => {
            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                if body.trim().is_empty()
                    || serde_json::from_str::<serde_json::Value>(body).is_err()
                {
                    return Ok(routing::bad_request_response("Request is required"));
                }
                return Ok(routing::ok_response(
                     serde_json::json!({
                         "message": "Configuration updated. Restart bridge service to apply changes.",
                         "restart_required": true,
                     })
                     .to_string(),
                 ));
            }
            let bridge = state
                .media_services
                .read()
                .await
                .virtual_soulfind
                .bridge
                .clone();
            let accepted_keys = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|value| {
                    value
                        .as_object()
                        .map(|object| object.keys().cloned().collect::<Vec<_>>())
                })
                .unwrap_or_default();
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.record_bridge_config_update(bridge.enabled, accepted_keys);
                if route.path.starts_with("/api/v0/") {
                    value["message"] = serde_json::json!(
                        "Configuration updated. Restart bridge service to apply changes."
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(body))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
