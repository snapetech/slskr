fn batch_header_value<'a>(operation: &'a batch::BatchOperation, name: &str) -> Option<&'a str> {
    operation
        .headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn batch_security_headers(
    operation: &batch::BatchOperation,
    parent: &RequestSecurityHeaders,
) -> RequestSecurityHeaders {
    let value = |name: &str, fallback: &Option<String>| {
        batch_header_value(operation, name)
            .map(str::to_owned)
            .or_else(|| fallback.clone())
    };
    RequestSecurityHeaders {
        host: value("host", &parent.host),
        origin: value("origin", &parent.origin),
        referer: value("referer", &parent.referer),
        cookie: value("cookie", &parent.cookie),
        content_type: value("content-type", &parent.content_type),
        x_share_token: value("x-share-token", &parent.x_share_token),
        x_gateway_api_key: value("x-api-key", &parent.x_gateway_api_key)
            .or_else(|| value("x-gateway-api-key", &parent.x_gateway_api_key)),
        x_gateway_csrf: value("x-slskdn-csrf", &parent.x_gateway_csrf),
        x_relay_agent: value("x-relay-agent", &parent.x_relay_agent),
        x_relay_credential: value("x-relay-credential", &parent.x_relay_credential),
        date: value("date", &parent.date),
        digest: value("digest", &parent.digest),
        signature: value("signature", &parent.signature),
        remote_addr: parent.remote_addr,
    }
}

fn batch_result_from_response(id: String, response: HttpResponse) -> batch::BatchOperationResult {
    let status = response
        .status
        .split(' ')
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(500);
    let error =
        (!(200..300).contains(&status)).then(|| format!("nested request returned HTTP {status}"));
    batch::BatchOperationResult {
        id,
        status,
        body: response.body,
        headers: std::collections::HashMap::new(),
        error,
    }
}

async fn route_dispatch_group_1(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
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
        ("GET", "/api/dht/peers") if route.path.starts_with("/api/v0/") => {
            let peers = match state.dht.as_ref() {
                Some(dht) => dht
                    .peers()
                    .await
                    .into_iter()
                    .map(|endpoint| {
                        serde_json::json!({
                            "address": endpoint.ip().to_string(),
                            "port": endpoint.port(),
                        })
                    })
                    .collect::<Vec<_>>(),
                None => Vec::new(),
            };
            Ok(routing::ok_response(
                serde_json::Value::Array(peers).to_string(),
            ))
        }
        ("GET", "/api/dht/peers") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let body = serde_json::Value::Array(
                mesh.capability_records_json()
                    .into_iter()
                    .chain(mesh.candidate_usernames(&users).into_iter().map(
                        |username| serde_json::json!({"username": username, "source": "soulseek"}),
                    ))
                    .collect(),
            )
            .to_string();
            Ok(routing::ok_response(body))
        }
        ("POST", path) if path.starts_with("/api/mesh/sync/") => {
            let Some(username) = path_segment_after(path, "/api/mesh/sync/") else {
                return Ok(routing::not_found_response());
            };
            if route.path.starts_with("/api/v0/") {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({"error": "Failed to sync with peer"}).to_string(),
                });
            }
            let username = decoded_path_segment(username);
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let candidate = mesh
                .candidate_usernames(&users)
                .into_iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&username));
            let watched = users.records.iter().any(|user| {
                user.username.eq_ignore_ascii_case(&username)
                    && (user.watched || user.status.as_deref() == Some("online"))
            });
            let capability = mesh
                .capability_records
                .iter()
                .any(|record| record.username.eq_ignore_ascii_case(&username));
            drop(mesh);
            drop(users);
            if let Err(error) =
                send_session_command(state, SessionCommand::ProbePeerCapability(username.clone()))
                    .await
            {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response(serde_json::json!({
                "success": true,
                "username": username,
                "queued": true,
                "probeQueued": true,
                "watched": watched,
                "capabilityRecord": capability,
                "status": if capability { "capable" } else if watched { "watched" } else if candidate { "candidate" } else { "probing" },
            }).to_string()))
        }
        ("GET", "/api/backfill/stats") => {
            let backfill = state.backfill.read().await;
            Ok(routing::ok_response(
                backfill.stats_json(unix_timestamp()).to_string(),
            ))
        }
        ("GET", "/api/backfill/config") => {
            let backfill = state.backfill.read().await;
            Ok(routing::ok_response(backfill.config_json().to_string()))
        }
        ("POST", "/api/backfill/enable") => {
            let enabled = match query_parameter(route.query, "enabled") {
                None => true,
                Some(value) => match value.parse::<bool>() {
                    Ok(enabled) => enabled,
                    Err(_) if route.path.starts_with("/api/v0/") => {
                        return Ok(routing::bad_request_response(
                            "The value 'enabled' is not valid.",
                        ));
                    }
                    Err(_) => true,
                },
            };
            state.backfill.write().await.enabled = enabled;
            Ok(routing::ok_response(
                serde_json::json!({ "enabled": enabled }).to_string(),
            ))
        }
        ("POST", "/api/backfill/idle") => {
            let mut backfill = state.backfill.write().await;
            backfill.is_idle = true;
            backfill.idle_since.get_or_insert_with(unix_timestamp);
            Ok(routing::ok_response(
                serde_json::json!({ "isIdle": true }).to_string(),
            ))
        }
        ("POST", "/api/backfill/busy") => {
            let mut backfill = state.backfill.write().await;
            backfill.is_idle = false;
            backfill.idle_since = None;
            Ok(routing::ok_response(
                serde_json::json!({ "isIdle": false }).to_string(),
            ))
        }
        ("POST", "/api/backfill/trigger") => Ok(routing::ok_response(
            run_backfill_cycle(state).await.to_string(),
        )),
        ("POST", "/api/backfill/file") => {
            let value =
                serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null);
            let peer_id = value
                .get("peerId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let path = value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let size = value
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if peer_id.is_empty() || path.is_empty() || size == 0 {
                return Ok(routing::bad_request_response(
                    "peerId, path, and size are required",
                ));
            }
            if let Some(response) =
                controller_native_backfill_file_write_failure_response(state, method, route.path)
                    .await
            {
                return Ok(response);
            }
            Ok(routing::ok_response(
                backfill_file(state, peer_id, path, size).await.to_string(),
            ))
        }
        ("GET", "/api/backfill/candidates") => {
            let limit = match query_parameter(route.query, "limit") {
                None => BACKFILL_DEFAULT_CANDIDATES,
                Some(value) => match value.parse::<usize>() {
                    Ok(value) if value > 0 => value,
                    Ok(_) => BACKFILL_DEFAULT_CANDIDATES,
                    Err(_) if route.path.starts_with("/api/v0/") => {
                        return Ok(routing::bad_request_response(
                            "The value 'limit' is not valid.",
                        ));
                    }
                    Err(_) => BACKFILL_DEFAULT_CANDIDATES,
                },
            };
            let candidates = backfill_candidates(state, limit)
                .await
                .iter()
                .map(BackfillCandidate::json)
                .collect::<Vec<_>>();
            let count = candidates.len();
            let json = if route.path.starts_with("/api/v0/") {
                serde_json::json!({
                    "count": count,
                    "candidates": candidates,
                })
            } else {
                serde_json::json!({
                    "candidates": candidates.clone(),
                    "entries": candidates,
                    "count": count,
                })
            };
            Ok(routing::ok_response(json.to_string()))
        }
        ("GET", "/api/dht/status") => {
            let mut value = if let Some(dht) = state.dht.as_ref() {
                serde_json::from_str::<serde_json::Value>(&dht.status_json().await)
                    .unwrap_or_else(|_| serde_json::json!({}))
            } else {
                serde_json::json!({
                    "dhtNodeCount": 0,
                    "isLanOnly": false,
                    "lanOnly": false,
                    "isBeaconCapable": false,
                    "isDhtRunning": false,
                    "verifiedBeaconCount": 0,
                })
            };
            let defaults = serde_json::json!({
                "isEnabled": state.dht.is_some(),
                "discoveredPeerCount": 0,
                "activeMeshConnections": 0,
                "totalPeersDiscovered": 0,
                "totalCandidateEndpointsSeen": 0,
                "totalCandidatesAccepted": 0,
                "totalCandidatesSkippedDhtPort": 0,
                "totalCandidatesSkippedDiscoveredCapacity": 0,
                "totalCandidatesDeferredConnectorCapacity": 0,
                "totalCandidatesSkippedReconnectBackoff": 0,
                "totalConnectionsAttempted": 0,
                "totalConnectionsSucceeded": 0,
                "lastAnnounceTime": serde_json::Value::Null,
                "lastDiscoveryTime": serde_json::Value::Null,
                "startedAt": serde_json::Value::Null,
                "uptimeSeconds": 0,
                "rendezvousInfohashes": [],
            });
            if let (Some(object), Some(defaults)) = (value.as_object_mut(), defaults.as_object()) {
                for (key, default) in defaults {
                    object.entry(key.clone()).or_insert_with(|| default.clone());
                }
            }
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: value.to_string(),
            })
        }
        ("POST", "/api/capabilities/negotiate") => Ok(capabilities_negotiate_response(body)),
        ("GET", "/swagger") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "301 Moved Permanently",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/index.html") => Ok(controller_swagger_index_response(&state.config)),
        ("GET", "/swagger/v0/swagger.json") => {
            if state.config.controller_swagger {
                let mut spec =
                    serde_json::from_str::<serde_json::Value>(&openapi::generate_openapi_json())
                        .unwrap_or_else(|_| serde_json::json!({}));
                spec["openapi"] = serde_json::json!("3.0.4");
                spec["info"]["title"] = if state.config.current_upstream_behavior {
                    serde_json::json!("slskR API")
                } else {
                    serde_json::json!(match state.config.controller_profile {
                        ControllerProfile::Legacy => "slskd",
                        ControllerProfile::Native => "slskr API",
                    })
                };
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json;charset=utf-8",
                    body: serde_json::to_string_pretty(&spec).unwrap_or_else(|_| "{}".to_owned()),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/index.js") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/javascript;charset=utf-8",
                    body: openapi::frozen_swagger_index_js("/swagger/v0/swagger.json"),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/swagger-ui.css") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/css;charset=utf-8",
                    body: "@import url('https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css');"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/index.css") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/css;charset=utf-8",
                    body: "html{box-sizing:border-box;overflow-y:scroll}body{margin:0;background:#fafafa}"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/swagger-ui-bundle.js") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/javascript;charset=utf-8",
                    body: "document.write('<script src=\"https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js\"><\\/script>');"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        ("GET", "/swagger/swagger-ui-standalone-preset.js") => {
            if state.config.controller_swagger {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/javascript;charset=utf-8",
                    body: "document.write('<script src=\"https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-standalone-preset.js\"><\\/script>');"
                        .to_owned(),
                })
            } else {
                Ok(controller_swagger_not_found_response())
            }
        }
        // Documentation endpoints
        ("GET", "/api/docs") | ("GET", "/api/v1/docs") | ("GET", "/api/v2/docs") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/html",
                body: openapi::swagger_ui_html("/api/openapi.json"),
            })
        }
        ("GET", "/api/openapi.json")
        | ("GET", "/api/v1/openapi.json")
        | ("GET", "/api/v2/openapi.json") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: openapi::generate_openapi_json(),
        }),
        ("GET", "/api/docs/index") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: serde_json::json!({
                "title": "slskr API Documentation",
                "version": "1.0.1",
                "docs": {
                    "swagger_ui": "/api/docs",
                    "openapi_spec": "/api/openapi.json",
                    "guides": {
                        "rate_limiting": "/docs/RATE_LIMITING.md",
                        "api_versioning": "/docs/API_VERSIONING.md",
                        "webhooks": "/docs/WEBHOOK_API.md"
                    }
                },
                "endpoints": {
                    "total": 202,
                    "by_method": {
                        "GET": 81,
                        "POST": 67,
                        "PUT": 6,
                        "DELETE": 15,
                        "PATCH": 1,
                        "OPTIONS": 32
                    }
                }
            })
            .to_string(),
        }),
        ("GET", "/api/docs/stats") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: serde_json::json!({
                "total_endpoints": 202,
                "api_versions": ["v0", "v1", "v2"],
                "categories": {
                    "health": 7,
                    "session": 5,
                    "search": 15,
                    "transfers": 18,
                    "users": 12,
                    "messages": 8,
                    "rooms": 15,
                    "shares": 8,
                    "webhooks": 6,
                    "collections": 22,
                    "wishlist": 18,
                    "contacts": 20,
                    "share_groups": 15,
                    "user_notes": 12,
                    "interests": 12
                },
                "features": {
                    "rate_limiting": {
                        "anonymous": "1000 req/min",
                        "authenticated": "5000 req/min"
                    },
                    "caching": "Cache-Control + ETag",
                    "compression": "gzip",
                    "cors": "Configurable",
                    "webhooks": "HMAC-SHA256"
                }
            })
            .to_string(),
        }),
        ("POST", "/api/batch") | ("POST", "/api/v1/batch") | ("POST", "/api/v2/batch") => {
            let (operations, config) = match batch::parse_batch_request(body) {
                Ok(parsed) => parsed,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            if let Err(error) = batch::validate_batch_operations(&operations) {
                return Ok(routing::bad_request_response(&error));
            }
            let started = Instant::now();
            let mut results = Vec::new();
            for operation in operations {
                let operation_id = operation.id.clone();
                let operation_headers = batch_security_headers(&operation, headers);
                let operation_authorization =
                    batch_header_value(&operation, "authorization").or(authorization);
                let operation_body = operation.body.as_deref().unwrap_or_default();
                let request = routing::RouteRequest::new(
                    &operation.method,
                    &operation.path,
                    operation_authorization,
                    operation_body,
                    &operation_headers,
                );
                let response = tokio::time::timeout(
                    Duration::from_millis(config.timeout_ms),
                    Box::pin(route_http_request_inner_with_batch(
                        request,
                        state,
                        state_arc.clone(),
                        false,
                    )),
                )
                .await;
                let result = match response {
                    Ok(Ok(response)) => batch_result_from_response(operation_id, response),
                    Ok(Err(error)) => batch::create_failure_result(operation_id, 500, error),
                    Err(_) => batch::create_failure_result(
                        operation_id,
                        504,
                        format!(
                            "batch operation {} {} timed out after {} ms",
                            operation.method, operation.path, config.timeout_ms
                        ),
                    ),
                };
                let is_error = !(200..300).contains(&result.status);
                results.push(result);
                if is_error && !config.continue_on_error {
                    break;
                }
            }
            let executed = results
                .iter()
                .filter(|result| result.error.is_none())
                .count();
            let failed = results.len().saturating_sub(executed);
            let mut value =
                serde_json::from_str::<serde_json::Value>(&batch::format_batch_response(results))
                    .unwrap_or_else(|_| serde_json::json!({ "results": [] }));
            if let Some(object) = value.as_object_mut() {
                let total_time_ms =
                    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                object.insert("accepted".to_owned(), serde_json::json!(true));
                object.insert("executed".to_owned(), serde_json::json!(executed));
                object.insert("failed".to_owned(), serde_json::json!(failed));
                object.insert("atomic".to_owned(), serde_json::json!(config.atomic));
                object.insert("timeoutMs".to_owned(), serde_json::json!(config.timeout_ms));
                object.insert("total_time_ms".to_owned(), serde_json::json!(total_time_ms));
            }
            Ok(routing::accepted_response(value.to_string()))
        }
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
            if let Err(error) = persist_event_record_checked(state, &record).await {
                *events = previous;
                return Ok(
                    if state.config.controller_profile == ControllerProfile::Native {
                        routing::internal_server_error_response("Failed to raise event")
                    } else {
                        routing::service_unavailable_response(&error)
                    },
                );
            }
            drop(events);
            scripts::dispatch(
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

                tokio::spawn(async move {
                    let _delivery_permit = delivery_permit;
                    let _ = webhooks::WebhookDispatcher::send_webhook(
                        &webhook_clone.url,
                        &webhook_clone.secret,
                        &payload.to_string(),
                        webhook_clone.timeout_seconds,
                    )
                    .await;
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
            ) && query_bool_is_invalid(route.query, "recursive")
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
            if state.config.controller_profile == ControllerProfile::Legacy && !root.is_dir() {
                return Ok(file_storage_error_response(
                    STORAGE_DIRECTORY_NOT_FOUND_ERROR,
                ));
            }
            let options = StorageDirectoryListOptions::from_query(route.query);
            match controller_storage_directory_json(&root, None, options) {
                Ok(json) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: target_storage_directory_json(json, state.config.controller_profile),
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
            let Some((storage, resource, encoded_name)) =
                controller_file_storage_resource_path(path)
            else {
                return Ok(routing::not_found_response());
            };
            if resource != "directories" {
                return Ok(routing::not_found_response());
            }
            if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            ) && query_bool_is_invalid(route.query, "recursive")
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
                    body: target_storage_directory_json(json, state.config.controller_profile),
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
            let Some((storage, resource, encoded_name)) =
                controller_file_storage_resource_path(path)
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
                            ControllerProfile::Legacy | ControllerProfile::Native
                        ) =>
                {
                    Ok(HttpResponse {
                        status: "204 No Content",
                        content_type: "",
                        body: String::new(),
                    })
                }
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
                    extension_filter
                        .as_deref()
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
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
