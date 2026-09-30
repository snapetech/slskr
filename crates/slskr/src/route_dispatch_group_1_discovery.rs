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

async fn route_dispatch_group_1_discovery(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let method = context.method;
    let normalized_path = context.normalized_path;
    let authorization = context.authorization;
    let body = context.body;
    let state = context.state;
    let route = context.route;
    let headers = context.headers;
    let state_arc = context.state_arc.clone();
    let extended_mutation = context.extended_mutation;
    let request_is_versioned_v0 = context.request_is_versioned_v0;
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
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
