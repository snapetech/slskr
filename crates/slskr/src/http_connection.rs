use super::*;

pub(super) async fn handle_http_connection(
    stream: TcpStream,
    state: Arc<AppState>,
) -> Result<(), String> {
    let remote_addr = stream.peer_addr().ok();
    handle_http_stream(stream, remote_addr, false, state).await
}

pub(super) async fn handle_http_stream<S>(
    stream: S,
    remote_addr: Option<SocketAddr>,
    secure: bool,
    state: Arc<AppState>,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (read_half, write_half) = tokio::io::split(stream);
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut writer = tokio::io::BufWriter::new(write_half);

    loop {
        let request_timer = logging::start_timer();

        // Read next request (returns None on clean close, Err on 413/malformed)
        let read_result = http_server::read_http_request_with_body_limit(
            &mut reader,
            state.config.controller_web_max_request_body_size,
        )
        .await;
        let (req, keep_alive) = match read_result {
            Ok(Some(pair)) => pair,
            Ok(None) => break, // client closed connection
            Err(e) => {
                if e.contains("too large")
                    && !state.config.current_upstream_behavior
                    && state.config.controller_profile == ControllerProfile::Native
                {
                    // Frozen native profile#65a14a8 lets Kestrel's body-limit
                    // BadHttpRequestException reach its generic exception
                    // handler, producing a 500 Problem Details response.
                    // Upstream correction: snapetech/slskdN#276.
                    let body = format!(
                        "{{\"title\":\"Internal Server Error\",\"status\":500,\"detail\":\"An unexpected error occurred.\",\"traceId\":\"{}\"}}",
                        uuid::Uuid::new_v4().simple()
                    );
                    let _ = writer
                        .write_all(
                            format!(
                                "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/problem+json\r\ncache-control: no-cache,no-store\r\nexpires: -1\r\npragma: no-cache\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            )
                            .as_bytes(),
                        )
                        .await;
                    let _ = writer.flush().await;
                    break;
                }
                let (status, body) = if e.contains("too large") {
                    (
                        "413 Payload Too Large",
                        r#"{"error":"request body too large"}"#,
                    )
                } else {
                    ("400 Bad Request", r#"{"error":"bad request"}"#)
                };
                let _ = writer
                    .write_all(
                        format!(
                            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                        .as_bytes(),
                    )
                    .await;
                let _ = writer.flush().await;
                break;
            }
        };

        if state.config.controller_web.https.force && !secure {
            let host = req.headers.host.as_deref().unwrap_or("localhost");
            let hostname = host
                .strip_prefix('[')
                .and_then(|value| value.split_once(']').map(|(host, _)| format!("[{host}]")))
                .unwrap_or_else(|| host.split(':').next().unwrap_or("localhost").to_owned());
            let https_port = state
                .config
                .controller_web
                .https
                .binds
                .first()
                .map_or(5031, SocketAddr::port);
            let location = format!("https://{hostname}:{https_port}{}", req.path);
            let response = format!(
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            writer
                .write_all(response.as_bytes())
                .await
                .map_err(|error| error.to_string())?;
            writer.flush().await.map_err(|error| error.to_string())?;
            break;
        }

        let method = req.method.as_str();
        let path_storage;
        let path = if state.config.controller_web.url_base == "/" {
            req.path.as_str()
        } else {
            let (request_path, query) = req
                .path
                .split_once('?')
                .map_or((req.path.as_str(), None), |(path, query)| {
                    (path, Some(query))
                });
            let base = state.config.controller_web.url_base.as_str();
            let stripped = if request_path == base {
                Some("/")
            } else {
                request_path
                    .strip_prefix(base)
                    .filter(|path| path.starts_with('/'))
            };
            let Some(stripped) = stripped else {
                let response = routing::not_found_response();
                http_server::write_http_response(&mut writer, &response, keep_alive, "").await?;
                if !keep_alive {
                    break;
                }
                continue;
            };
            path_storage = query.map_or_else(
                || stripped.to_owned(),
                |query| format!("{stripped}?{query}"),
            );
            path_storage.as_str()
        };
        let body_text = req.body_as_str().ok();
        let websocket_route = routing::parse_route(method, path);
        let websocket_path = if let Some(versioned_path) = websocket_route
            .normalized_path
            .strip_prefix("/api/v0/")
            .or_else(|| websocket_route.normalized_path.strip_prefix("/api/v1/"))
            .or_else(|| websocket_route.normalized_path.strip_prefix("/api/v2/"))
        {
            format!("/api/{}", versioned_path)
        } else {
            websocket_route.normalized_path.to_string()
        };
        let websocket_protocol_authorization = (method == "GET"
            && websocket_path == "/api/events/ws")
            .then(|| {
                websocket_protocol_authorization(req.headers.sec_websocket_protocol.as_deref())
            })
            .flatten();
        if mixed_websocket_auth_credentials(
            &req.headers,
            websocket_protocol_authorization.as_deref(),
        ) {
            let response = routing::bad_request_response("multiple authentication mechanisms");
            let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
            break;
        }
        let api_key_authorization = req
            .headers
            .x_api_key
            .as_ref()
            .map(|api_key| format!("ApiKey {api_key}"));
        let authorization = req
            .headers
            .authorization
            .as_deref()
            .or(api_key_authorization.as_deref())
            .or(websocket_protocol_authorization.as_deref());
        let mut sec_headers = RequestSecurityHeaders::from_http_headers(&req.headers);
        sec_headers.remote_addr = remote_addr;

        // The frozen native profile controller applies distinct fixed-window partitions
        // before authorization. An authenticated principal bypasses only the
        // general anonymous API partition; mesh, inbox, event injection, and
        // warm-cache policies still apply.
        let rate_limit_user = authenticated_rate_limit_user_key(
            &state.config,
            authorization,
            sec_headers.cookie.as_deref(),
            sec_headers.remote_addr,
        );
        let username = rate_limit_user.as_deref();
        let rate_limit_remote_addr =
            rate_limit_remote_addr(&state.config, remote_addr, &req.headers);
        let controller_metrics_request =
            method == "GET" && path == controller_metrics_path(&state.config);
        let controller_rate_policy = controller_rate_limit_policy(
            &state.config,
            method,
            path,
            username,
            rate_limit_remote_addr,
        );
        if controller_rate_policy
            .as_ref()
            .is_some_and(|policy| policy.max_requests == 0)
        {
            // Frozen native profile#65a14a8 accepts non-positive permit counts,
            // then FixedWindowRateLimiter throws when the affected partition
            // is first materialized. Its exception middleware exposes the
            // failure as generic 500 Problem Details.
            let body = format!(
                "{{\"title\":\"Internal Server Error\",\"status\":500,\"detail\":\"An unexpected error occurred.\",\"traceId\":\"{}\"}}",
                uuid::Uuid::new_v4().simple()
            );
            let _ = writer
                .write_all(
                    format!(
                        "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/problem+json\r\ncache-control: no-cache,no-store\r\nexpires: -1\r\npragma: no-cache\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await;
            let _ = writer.flush().await;
            break;
        }
        let allowed = if controller_metrics_request {
            true
        } else if let Some(policy) = controller_rate_policy.as_ref() {
            state
                .rate_limiter
                .check_ip_partition(
                    rate_limit_remote_addr,
                    &policy.partition,
                    policy.max_requests,
                    policy.window_seconds,
                )
                .await
        } else {
            true
        };

        // Relay multipart uploads are the one controller API surface that is
        // intentionally binary.  Keep the normal text route path unchanged,
        // but dispatch an invalid-UTF-8 upload with its original bytes after
        // applying the same rate-limit and authorization gates.
        let raw_relay_upload = body_text.is_none()
            && method == "POST"
            && (path.starts_with("/api/v0/relay/controller/files/")
                || path.starts_with("/api/v0/relay/controller/shares/"));
        if raw_relay_upload {
            let response = if !allowed {
                routing::HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "",
                    body: String::new(),
                }
            } else if request_uses_revoked_jwt(&state, authorization).await {
                routing::unauthorized_response()
            } else if let Err(reason) =
                routing::check_route_auth(&state.config, method, path, authorization, &sec_headers)
            {
                match reason {
                    "unauthorized" => routing::unauthorized_response(),
                    "csrf" => routing::forbidden_response("cross-site mutating request rejected"),
                    _ => routing::forbidden_response("insufficient permissions for this route"),
                }
            } else {
                versioned_relay_request_bytes(method, path, &req.body, &sec_headers, &state)
                    .await
                    .unwrap_or_else(routing::not_found_response)
            };
            http_server::write_http_response(&mut writer, &response, keep_alive, "").await?;
            if !keep_alive {
                break;
            }
            continue;
        }
        let body = body_text.unwrap_or("");

        // SignalR's default bearer-token transport places the relay API key in
        // the `access_token` query parameter during the websocket upgrade.
        // Accept that form in addition to the normal Authorization/X-API-Key
        // headers, but authorize it against the relay controller's API policy
        // before handing the raw socket to the hub protocol.
        if method == "GET" && websocket_path == "/hub/relay" {
            let relay_hub_query_token = req.query.as_deref().and_then(|query| {
                query.split('&').find_map(|part| {
                    let (name, value) = part.split_once('=')?;
                    (name == "access_token").then(|| percent_decode_component(value))
                })
            });
            let relay_hub_query_api_key = relay_hub_query_token
                .as_deref()
                .map(|token| format!("ApiKey {token}"));
            let relay_hub_authorization = authorization.or(relay_hub_query_api_key.as_deref());
            let relay_hub_route =
                "/api/v0/relay/controller/downloads/00000000-0000-0000-0000-000000000000";
            let auth_result = routing::check_route_auth(
                &state.config,
                "GET",
                relay_hub_route,
                relay_hub_authorization,
                &sec_headers,
            );
            if let Err(reason) = auth_result {
                let response = match reason {
                    "unauthorized" => routing::unauthorized_response(),
                    "csrf" => routing::forbidden_response("cross-site mutating request rejected"),
                    _ => routing::forbidden_response("insufficient permissions for this route"),
                };
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }
            let relay_settings = state.advanced_networking.read().await.relay.clone();
            if !relay_settings.enabled
                || !matches!(relay_settings.mode.as_str(), "controller" | "debug")
            {
                let response = routing::forbidden_response("feature is disabled by configuration");
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }
            let websocket_key = req.headers.sec_websocket_key.as_deref();
            let websocket_version = req.headers.sec_websocket_version.as_deref();
            let upgrade = req.headers.upgrade.as_deref();
            let is_upgrade = req.headers.connection_has_token("upgrade")
                && upgrade == Some("websocket")
                && websocket_version == Some("13");
            if let Some(websocket_key) =
                websocket_key.filter(|key| is_upgrade && events_ws::valid_sec_websocket_key(key))
            {
                let Ok(_websocket_permit) =
                    Arc::clone(&state.websocket_connections).try_acquire_owned()
                else {
                    let response = routing::HttpResponse {
                        status: "503 Service Unavailable",
                        content_type: "application/json",
                        body: r#"{"error":"too many websocket connections"}"#.to_owned(),
                    };
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                };
                events_ws::write_upgrade_response(&mut writer, websocket_key, None).await?;
                relay_ws::serve(reader, &mut writer, Arc::clone(&state), remote_addr).await?;
            } else {
                let response = routing::bad_request_response("invalid websocket upgrade");
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
            }
            break;
        }

        // The frozen slskd and native profile web clients use ASP.NET SignalR for
        // application, search, logging, metrics, transfer, song-id, and
        // listening-party updates.  Keep the native event websocket separate,
        // but expose the target hub paths and JSON protocol at their original
        // routes so the frozen clients can connect without a compatibility
        // shim in the browser.
        let hub_query_api_key = req.query.as_deref().and_then(|query| {
            query.split('&').find_map(|part| {
                let (name, value) = part.split_once('=')?;
                (name == "access_token")
                    .then(|| format!("Bearer {}", percent_decode_component(value)))
            })
        });
        let hub_authorization = authorization.or(hub_query_api_key.as_deref());
        if method == "POST" {
            if let Some(hub) = signalr_ws::negotiate_hub_name_for_target(
                &websocket_path,
                state.config.controller_profile,
            ) {
                if !allowed {
                    let response = routing::HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "",
                        body: String::new(),
                    };
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                let fallback_host = state.config.http_bind.to_string();
                if !request_origin_matches_host(&sec_headers, &fallback_host) {
                    let response = routing::forbidden_response("websocket origin rejected");
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                if request_uses_revoked_jwt(&state, hub_authorization).await {
                    let response = routing::unauthorized_response();
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                if let Err(reason) = routing::check_route_auth(
                    &state.config,
                    method,
                    signalr_ws::auth_path(hub),
                    hub_authorization,
                    &sec_headers,
                ) {
                    let response = match reason {
                        "unauthorized" => routing::unauthorized_response(),
                        "csrf" => {
                            routing::forbidden_response("cross-site mutating request rejected")
                        }
                        _ => routing::forbidden_response("insufficient permissions for this route"),
                    };
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                let response = signalr_ws::negotiate_response();
                http_server::write_http_response(&mut writer, &response, keep_alive, "").await?;
                if !keep_alive {
                    break;
                }
                continue;
            }
        }

        if method == "GET" {
            if let Some(hub) =
                signalr_ws::hub_name_for_target(&websocket_path, state.config.controller_profile)
            {
                if !allowed {
                    let response = routing::HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "",
                        body: String::new(),
                    };
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                let fallback_host = state.config.http_bind.to_string();
                if !request_origin_matches_host(&sec_headers, &fallback_host) {
                    let response = routing::forbidden_response("websocket origin rejected");
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                if request_uses_revoked_jwt(&state, hub_authorization).await {
                    let response = routing::unauthorized_response();
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                if let Err(reason) = routing::check_route_auth(
                    &state.config,
                    method,
                    signalr_ws::auth_path(hub),
                    hub_authorization,
                    &sec_headers,
                ) {
                    let response = match reason {
                        "unauthorized" => routing::unauthorized_response(),
                        "csrf" => {
                            routing::forbidden_response("cross-site mutating request rejected")
                        }
                        _ => routing::forbidden_response("insufficient permissions for this route"),
                    };
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
                let websocket_key = req.headers.sec_websocket_key.as_deref();
                let websocket_version = req.headers.sec_websocket_version.as_deref();
                let upgrade = req.headers.upgrade.as_deref();
                let is_upgrade = req.headers.connection_has_token("upgrade")
                    && upgrade == Some("websocket")
                    && websocket_version == Some("13");
                if let Some(websocket_key) = websocket_key
                    .filter(|key| is_upgrade && events_ws::valid_sec_websocket_key(key))
                {
                    let Ok(_websocket_permit) =
                        Arc::clone(&state.websocket_connections).try_acquire_owned()
                    else {
                        let response = routing::HttpResponse {
                            status: "503 Service Unavailable",
                            content_type: "application/json",
                            body: r#"{"error":"too many websocket connections"}"#.to_owned(),
                        };
                        let _ = http_server::write_http_response(&mut writer, &response, false, "")
                            .await;
                        break;
                    };
                    events_ws::write_upgrade_response(&mut writer, websocket_key, None).await?;
                    signalr_ws::serve(reader, &mut writer, Arc::clone(&state), hub).await?;
                } else {
                    let response = routing::bad_request_response("invalid websocket upgrade");
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                }
                break;
            }
        }

        if method == "GET" && websocket_path == "/api/events/ws" {
            if !allowed {
                let response = routing::HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "",
                    body: String::new(),
                };
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }
            let fallback_host = state.config.http_bind.to_string();
            if !request_origin_matches_host(&sec_headers, &fallback_host) {
                let response = routing::forbidden_response("websocket origin rejected");
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }

            if request_uses_revoked_jwt(&state, authorization).await {
                let response = routing::unauthorized_response();
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }

            if let Err(reason) = routing::check_route_auth(
                &state.config,
                method,
                &websocket_path,
                authorization,
                &sec_headers,
            ) {
                let response = match reason {
                    "unauthorized" => routing::unauthorized_response(),
                    "csrf" => routing::forbidden_response("cross-site mutating request rejected"),
                    _ => routing::forbidden_response("insufficient permissions for this route"),
                };
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }

            let websocket_key = req.headers.sec_websocket_key.as_deref();
            let websocket_version = req.headers.sec_websocket_version.as_deref();
            let upgrade = req.headers.upgrade.as_deref();
            let is_upgrade = req.headers.connection_has_token("upgrade")
                && upgrade == Some("websocket")
                && websocket_version == Some("13");

            if let Some(websocket_key) =
                websocket_key.filter(|key| is_upgrade && events_ws::valid_sec_websocket_key(key))
            {
                let Ok(_websocket_permit) =
                    Arc::clone(&state.websocket_connections).try_acquire_owned()
                else {
                    let response = routing::HttpResponse {
                        status: "503 Service Unavailable",
                        content_type: "application/json",
                        body: r#"{"error":"too many websocket connections"}"#.to_owned(),
                    };
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                };
                let accepted_protocol =
                    websocket_auth_protocol(req.headers.sec_websocket_protocol.as_deref());
                events_ws::write_upgrade_response(&mut writer, websocket_key, accepted_protocol)
                    .await?;
                events_ws::stream_events(
                    reader,
                    &mut writer,
                    &state.events,
                    state.event_tx.subscribe(),
                )
                .await?;
            } else {
                let response = routing::bad_request_response("invalid websocket upgrade");
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
            }
            break;
        }

        // Log request
        let req_log = logging::HttpRequestLog {
            method: method.to_string(),
            path: path.to_string(),
            query: req.query.clone(),
            remote_addr: remote_addr.map(|a| a.to_string()),
            timestamp: logging::format_timestamp(),
        };
        let request_id = generate_request_id();

        // Frozen native profile only installs CORS middleware when enabled with a
        // non-empty origin list, and only short-circuits genuine preflights.
        // The slskd profile retains slskR's established OPTIONS contract.
        let native_cors_middleware_active = state.config.controller_profile
            == ControllerProfile::Native
            && state.config.controller_web_cors.enabled
            && !state.config.controller_web_cors.allowed_origins.is_empty();
        let native_cors_preflight = native_cors_middleware_active
            && is_cors_preflight(method)
            && req.headers.origin.is_some()
            && req.headers.access_control_request_method.is_some();
        if (state.config.controller_profile == ControllerProfile::Legacy
            && is_cors_preflight(method))
            || native_cors_preflight
        {
            let fallback_host = state.config.http_bind.to_string();
            let cors_str = controller_cors_headers(
                &state.config,
                &req.headers,
                &fallback_host,
                native_cors_preflight,
            );
            let status = if native_cors_preflight {
                "204 No Content"
            } else {
                "200 OK"
            };
            let _ = writer
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\ncontent-length: 0\r\nconnection: {}\r\n{}\r\n",
                        if keep_alive { "keep-alive" } else { "close" },
                        cors_str
                    )
                    .as_bytes(),
                )
                .await;
            let _ = writer.flush().await;
            if !keep_alive {
                break;
            }
            continue;
        }

        let non_api_controller_route = path == "/.well-known/webfinger"
            || path.starts_with("/actors/")
            || path == "/mesh/http/services"
            || path == "/swagger"
            || path.starts_with("/swagger/")
            || path == controller_metrics_path(&state.config);
        if matches!(method, "GET" | "HEAD")
            && allowed
            && !non_api_controller_route
            && !state.config.controller_headless
        {
            let fallback_host = state.config.http_bind.to_string();
            let cors_str =
                controller_cors_headers(&state.config, &req.headers, &fallback_host, false);
            match write_web_static_response(
                &mut writer,
                path,
                Some(&state.config.controller_web.content_path),
                state.config.controller_profile,
                !state.config.current_upstream_behavior,
                method == "GET",
                keep_alive,
                &cors_str,
            )
            .await
            {
                Ok(Some(content_length)) => {
                    let resp_log = logging::HttpResponseLog {
                        status_code: 200,
                        content_length,
                        duration_ms: logging::elapsed_ms(request_timer),
                        error: None,
                    };
                    let log_config = logging::LogConfig {
                        level: *state.log_level.read().await,
                        log_requests: state.config.controller_web.logging,
                        log_responses: state.config.controller_web.logging,
                        log_errors_only: false,
                        no_color: state.config.logger.no_color,
                    };
                    logging::log_transaction(
                        &log_config,
                        &logging::HttpTransactionLog {
                            request: req_log.clone(),
                            response: resp_log.clone(),
                        },
                    );
                    record_http_log(
                        &state,
                        &request_id,
                        &logging::HttpTransactionLog {
                            request: req_log,
                            response: resp_log,
                        },
                    )
                    .await;
                    if !keep_alive {
                        break;
                    }
                    continue;
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!("web static response failed: {error}");
                    let response = web_static_error_response(&error);
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
            }
        }

        let relay_stream_content_id = relay_versioned_stream_content_id(method, path);
        let request_target = req.query.as_deref().map(|query| format!("{path}?{query}"));
        let routed_path = request_target.as_deref().unwrap_or(path);
        let mut response = if !allowed {
            routing::HttpResponse {
                status: "429 Too Many Requests",
                content_type: "",
                body: String::new(),
            }
        } else {
            match route_http_request_with_state(
                method,
                routed_path,
                authorization,
                body,
                state.clone(),
                &sec_headers,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => routing::HttpResponse {
                    status: "500 Internal Server Error",
                    content_type: "application/json",
                    body: format!("{{\"error\":\"{}\"}}", json_escape(&e)),
                },
            }
        };

        // The route handler owns authorization and feature-gate behavior, but
        // the actual relay stream is a file response and therefore has to be
        // opened by the connection-level streaming pipeline below.
        if relay_stream_content_id.is_some()
            && state
                .media_services
                .read()
                .await
                .features
                .streaming_relay_fallback
            && matches!(response.status, "200 OK" | "404 Not Found")
        {
            response = routing::HttpResponse {
                status: "200 OK",
                content_type: "application/octet-stream",
                body: String::new(),
            };
        }

        // Keep the grant lease in request scope through file writes and disconnects.
        let mut _share_stream_lease = None;
        if method == "GET" && response.status == "200 OK" {
            if let Some(stream_id) = primary_stream_id(path) {
                match crate::share_stream_limits::acquire_ticket_stream(
                    &state,
                    &stream_id,
                    req.query.as_deref(),
                )
                .await
                {
                    Ok(lease) => _share_stream_lease = lease,
                    Err(crate::share_stream_limits::AdmissionError::Unauthorized) => {
                        response = routing::unauthorized_response();
                    }
                    Err(crate::share_stream_limits::AdmissionError::Busy) => {
                        response = routing::HttpResponse {
                            status: "429 Too Many Requests",
                            content_type: "application/json",
                            body: "{\"error\":\"share stream concurrency limit reached\"}"
                                .to_owned(),
                        };
                    }
                }
            }
        }

        let application_dump = application_dump_request(method, path, &state.config);
        let listening_party_path = listening_party_stream_path(path);
        if listening_party_path.is_some()
            && req
                .headers
                .range
                .as_deref()
                .is_some_and(|range| range.contains(','))
            && response.status == "200 OK"
        {
            // Frozen StreamListedParty rejects multipart ranges with 400
            // before the FileStreamResult is created.
            response = routing::bad_request_response("Multiple byte ranges are not supported.");
        }
        let preview_ticket = http_stream_ticket_path(path, req.query.as_deref());
        let relay_download_token = relay_versioned_download_token(method, path);
        let preview_stream = preview_ticket.is_some();
        let mut remote_preview = None;
        let mut remote_preview_head = None;
        let mut preview_permit = None;
        let mut _listening_party_permits: Option<(OwnedSemaphorePermit, OwnedSemaphorePermit)> =
            None;
        if preview_stream
            && listening_party_path.is_none()
            && allowed
            && matches!(method, "GET" | "HEAD")
            && response.status == "200 OK"
        {
            match Arc::clone(&state.preview_streams).try_acquire_owned() {
                Ok(permit) => preview_permit = Some(permit),
                Err(_) => {
                    response = routing::HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "application/json",
                        body: r#"{"error":"preview stream limit reached"}"#.to_owned(),
                    };
                }
            }
        }
        if let Some((party_id, _)) = listening_party_path.as_ref().filter(|_| {
            preview_stream
                && allowed
                && matches!(method, "GET" | "HEAD")
                && response.status == "200 OK"
        }) {
            let remote_ip = remote_addr
                .map(|address| address.ip().to_string())
                .unwrap_or_else(|| "unknown".to_owned());
            match state
                .listening_party_stream_limits
                .write()
                .await
                .try_acquire(party_id, &remote_ip)
            {
                Ok(permits) => _listening_party_permits = Some(permits),
                Err(ListeningPartyStreamLimitRejection::Party) => {
                    response = routing::HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "text/plain; charset=utf-8",
                        body: "Too many concurrent radio streams.".to_owned(),
                    };
                }
                Err(ListeningPartyStreamLimitRejection::Ip) => {
                    response = routing::HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "text/plain; charset=utf-8",
                        body: "Too many concurrent radio streams from this address.".to_owned(),
                    };
                }
                Err(ListeningPartyStreamLimitRejection::Capacity) => {
                    response = routing::service_unavailable_response(
                        "listening-party stream limiter capacity is full",
                    );
                }
            }
        }
        let stream_file = if allowed
            && (application_dump || matches!(method, "GET" | "HEAD"))
            && response.status == "200 OK"
        {
            if application_dump {
                match open_application_dump_file(&state).await {
                    Ok(stream) => Some(stream),
                    Err(error) => {
                        eprintln!("application dump failed: {error}");
                        response = routing::internal_server_error_response(
                            "failed to create application dump",
                        );
                        None
                    }
                }
            } else if let Some(token) = relay_download_token.as_deref() {
                match open_relay_controller_download(&state, token, &sec_headers).await {
                    Ok(stream) => Some(stream),
                    Err(error) => {
                        eprintln!("relay controller download failed: {error}");
                        response = if error == "Invalid filename" {
                            routing::bad_request_response("Invalid filename")
                        } else {
                            routing::internal_server_error_response(
                                "failed to open relay controller download",
                            )
                        };
                        None
                    }
                }
            } else if let Some(content_id) = relay_stream_content_id.as_deref() {
                match open_relay_controller_stream(&state, content_id, req.query.as_deref()).await {
                    Ok(stream) => Some(stream),
                    Err(error) => {
                        eprintln!("relay controller stream failed: {error}");
                        response = if error.contains("agentName query parameter") {
                            routing::bad_request_response(&error)
                        } else if error.contains("timed out") {
                            routing::HttpResponse {
                                status: "504 Gateway Timeout",
                                content_type: "application/json",
                                body: format!("{{\"error\":\"{}\"}}", json_escape(&error)),
                            }
                        } else if error.contains("not found") || error.contains("not registered") {
                            routing::not_found_response()
                        } else {
                            routing::internal_server_error_response("failed to stream content")
                        };
                        None
                    }
                }
            } else if let Some(stream_id) = primary_stream_id(path) {
                match open_primary_stream_file(&state, &stream_id, req.query.as_deref()).await {
                    Ok(Some(stream)) => Some(stream),
                    Ok(None) => {
                        response = routing::not_found_response();
                        None
                    }
                    Err(error) => {
                        eprintln!("local stream open failed: {error}");
                        response = routing::not_found_response();
                        None
                    }
                }
            } else if let Some((family, ticket)) = preview_ticket.as_ref() {
                match open_local_preview_stream_file(&state, family, ticket).await {
                    Ok(Some(stream)) => Some(stream),
                    Ok(None) => {
                        if *family == "listening-party" {
                            response = routing::not_found_response();
                            None
                        } else if method == "HEAD" {
                            let ticket_record = {
                                let mut tickets = state.stream_tickets.write().await;
                                tickets.get(ticket)
                            };
                            match remote_preview_head_ticket(ticket_record, family) {
                                Some(record) => remote_preview_head = Some(record),
                                None => response = routing::not_found_response(),
                            }
                            None
                        } else if *family == "mesh" {
                            match open_remote_mesh_preview_file(&state, family, ticket).await {
                                Ok(stream) => stream,
                                Err(error) => {
                                    eprintln!("remote mesh preview open failed: {error}");
                                    response = routing::not_found_response();
                                    None
                                }
                            }
                        } else {
                            match open_remote_peer_preview_stream(&state, family, ticket).await {
                                Ok(stream) => remote_preview = stream,
                                Err(error) => {
                                    eprintln!("remote preview stream open failed: {error}");
                                    response = routing::not_found_response();
                                }
                            }
                            None
                        }
                    }
                    Err(error) => {
                        eprintln!("local preview stream open failed: {error}");
                        response = routing::not_found_response();
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        // Build extra headers. ASP.NET Core's frozen global limiter does not
        // emit quota headers unless an OnRejected handler adds them; neither
        // frozen controller does so.
        let cache_hdr =
            cache_control_header(method, response.content_type, path).unwrap_or_default();
        let etag_hdr = if method == "GET" && response.content_type.contains("json") {
            format!("ETag: {}\r\n", generate_etag(&response.body))
        } else {
            String::new()
        };
        let fallback_host = state.config.http_bind.to_string();
        let cors_str = controller_cors_headers(&state.config, &req.headers, &fallback_host, false);
        let location_hdr = if method == "GET"
            && path == "/swagger"
            && response.status == "301 Moved Permanently"
        {
            "Location: swagger/index.html\r\n"
        } else {
            ""
        };
        let metrics_authenticate_hdr = if controller_metrics_request
            && response.status == "401 Unauthorized"
            && state.config.controller_profile == ControllerProfile::Native
        {
            "WWW-Authenticate: Basic realm=\"metrics\"\r\n"
        } else {
            ""
        };
        let total_count_hdr = if method == "GET"
            && response.status == "200 OK"
            && matches!(
                path,
                "/api/events" | "/api/events/slskd" | "/api/v0/events" | "/api/v0/events/slskd"
            ) {
            let events = state.events.read().await;
            format!(
                "X-Total-Count: {}\r\n",
                events.controller_total_count(req.query.as_deref())
            )
        } else {
            String::new()
        };
        let extra = format!(
            "{}{}{}{}{}{}X-Request-ID: {}\r\n",
            cache_hdr,
            etag_hdr,
            cors_str,
            location_hdr,
            metrics_authenticate_hdr,
            total_count_hdr,
            request_id
        );

        if let Some(stream) = stream_file {
            let cleanup_path = stream.cleanup_path.clone();
            let disposition = if application_dump {
                "Content-Disposition: attachment; filename=slskd.dmp; filename*=UTF-8''slskd.dmp\r\n"
            } else {
                ""
            };
            let stream_extra = format!(
                "{}{}X-Request-ID: {}\r\n",
                cors_str, disposition, request_id
            );
            let accept_ranges =
                !application_dump && (!preview_stream || listening_party_path.is_some());
            let written = http_server::write_file_response(
                &mut writer,
                stream.file,
                stream.length,
                &stream.content_type,
                req.headers.range.as_deref(),
                accept_ranges,
                application_dump || method == "GET",
                keep_alive,
                &stream_extra,
            )
            .await;
            if let Some(path) = cleanup_path {
                let _ = fs::remove_file(path);
            }
            let written = written?;
            let resp_log = logging::HttpResponseLog {
                status_code: written.status_code,
                content_length: usize::try_from(written.content_length).unwrap_or(usize::MAX),
                duration_ms: logging::elapsed_ms(request_timer),
                error: None,
            };
            let log_config = logging::LogConfig {
                level: *state.log_level.read().await,
                log_requests: state.config.controller_web.logging,
                log_responses: state.config.controller_web.logging,
                log_errors_only: false,
                no_color: state.config.logger.no_color,
            };
            logging::log_transaction(
                &log_config,
                &logging::HttpTransactionLog {
                    request: req_log.clone(),
                    response: resp_log.clone(),
                },
            );
            record_http_log(
                &state,
                &request_id,
                &logging::HttpTransactionLog {
                    request: req_log,
                    response: resp_log,
                },
            )
            .await;
            drop(preview_permit);
            if !keep_alive {
                break;
            }
            continue;
        }

        if let Some(ticket) = remote_preview_head {
            let stream_extra = format!("{}X-Request-ID: {}\r\n", cors_str, request_id);
            let written = write_remote_preview_head_response(
                &mut writer,
                &ticket,
                keep_alive,
                &stream_extra,
                state.config.peer_response_timeout,
            )
            .await?;
            let resp_log = logging::HttpResponseLog {
                status_code: written.status_code,
                content_length: usize::try_from(written.content_length).unwrap_or(usize::MAX),
                duration_ms: logging::elapsed_ms(request_timer),
                error: None,
            };
            let log_config = logging::LogConfig {
                level: *state.log_level.read().await,
                log_requests: state.config.controller_web.logging,
                log_responses: state.config.controller_web.logging,
                log_errors_only: false,
                no_color: state.config.logger.no_color,
            };
            logging::log_transaction(
                &log_config,
                &logging::HttpTransactionLog {
                    request: req_log.clone(),
                    response: resp_log.clone(),
                },
            );
            record_http_log(
                &state,
                &request_id,
                &logging::HttpTransactionLog {
                    request: req_log,
                    response: resp_log,
                },
            )
            .await;
            drop(preview_permit);
            if !keep_alive {
                break;
            }
            continue;
        }

        if let Some(preview) = remote_preview {
            let stream_extra = format!("{}X-Request-ID: {}\r\n", cors_str, request_id);
            let written = write_peer_preview_response(
                &mut writer,
                preview,
                method == "GET",
                keep_alive,
                &stream_extra,
                state.config.peer_response_timeout,
            )
            .await?;
            let resp_log = logging::HttpResponseLog {
                status_code: written.status_code,
                content_length: usize::try_from(written.content_length).unwrap_or(usize::MAX),
                duration_ms: logging::elapsed_ms(request_timer),
                error: None,
            };
            let log_config = logging::LogConfig {
                level: *state.log_level.read().await,
                log_requests: state.config.controller_web.logging,
                log_responses: state.config.controller_web.logging,
                log_errors_only: false,
                no_color: state.config.logger.no_color,
            };
            logging::log_transaction(
                &log_config,
                &logging::HttpTransactionLog {
                    request: req_log.clone(),
                    response: resp_log.clone(),
                },
            );
            record_http_log(
                &state,
                &request_id,
                &logging::HttpTransactionLog {
                    request: req_log,
                    response: resp_log,
                },
            )
            .await;
            drop(preview_permit);
            if !keep_alive {
                break;
            }
            continue;
        }

        // Write response
        http_server::write_http_response_with_body(
            &mut writer,
            &response,
            method != "HEAD",
            keep_alive,
            &extra,
        )
        .await?;

        // Log
        let resp_log = logging::HttpResponseLog {
            status_code: logging::status_code_from_string(response.status),
            content_length: response.body.len(),
            duration_ms: logging::elapsed_ms(request_timer),
            error: None,
        };
        let log_config = logging::LogConfig {
            level: *state.log_level.read().await,
            log_requests: state.config.controller_web.logging,
            log_responses: state.config.controller_web.logging,
            log_errors_only: false,
            no_color: state.config.logger.no_color,
        };
        logging::log_transaction(
            &log_config,
            &logging::HttpTransactionLog {
                request: req_log.clone(),
                response: resp_log.clone(),
            },
        );
        record_http_log(
            &state,
            &request_id,
            &logging::HttpTransactionLog {
                request: req_log,
                response: resp_log,
            },
        )
        .await;

        if !keep_alive {
            break;
        }
    }
    Ok(())
}
