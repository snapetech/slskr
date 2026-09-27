async fn route_dispatch_group_0_control_plane(
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
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && matches!(
                    route.path,
                    "/api/slskdn/capabilities" | "/api/v0/slskdn/capabilities"
                ) =>
        {
            Ok(native_capabilities_response(state).await)
        }
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/capabilities" =>
        {
            Ok(native_capability_controller_response(state).await)
        }
        ("GET", "/api/capabilities") => Ok(capabilities_response()),
        ("GET", "/.well-known/webfinger") => {
            Ok(activitypub_webfinger_response(route.query, state).await)
        }
        ("GET", path) if path.starts_with("/actors/") => {
            Ok(activitypub_get_response(path, route.query, state).await)
        }
        ("GET", "/mesh/http/services") => Ok(mesh_http_services_response(state).await),
        ("GET", "/api/security/bans") if route.path.starts_with("/api/v0/") => {
            let security = state.security.read().await;
            Ok(routing::ok_response(security.native_bans_json()))
        }
        ("GET", "/api/security/transports/status")
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native =>
        {
            // Matches the native profile's versioned TransportSelectorStatus contract.
            // The native slskR endpoint below intentionally retains its
            // historical selectedTransport/healthy shape.
            let configured = state
                .controller_features
                .read()
                .await
                .get("security/profile/security/transports")
                .and_then(|value| value.get("status"))
                .cloned();
            Ok(routing::ok_response(
                configured
                    .unwrap_or_else(|| {
                        serde_json::json!({
                            "selectedMode": "Direct",
                            "totalTransports": 1,
                            "availableTransports": 0,
                            "availableTransportTypes": [],
                            "lastConnectivityTest": chrono::Utc::now().to_rfc3339(),
                            "primaryTransportAvailable": false,
                            "fallbackAvailable": false,
                        })
                    })
                    .to_string(),
            ))
        }
        // native profile keeps the legacy /api/info compatibility controller as a
        // deliberately small projection.  Do not route it through slskR's
        // richer /api/application lifecycle DTO: clients use these fields to
        // identify the compatibility implementation and Soulseek state.
        ("GET", "/api/application")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/info" =>
        {
            let session = state.session.read().await;
            let connected = matches!(session.state, "connected" | "logged_in");
            let user = if connected {
                session
                    .username
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            } else {
                state
                    .config
                    .username
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "impl": "slskdn",
                    "compat": "slskd",
                    "version": APP_VERSION,
                    "soulseek": {
                        "connected": connected,
                        "user": user,
                    },
                })
                .to_string(),
            ))
        }
        ("GET", "/api/application") => {
            let body = application_state_json_for_state(state).await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/application/version/latest") => Ok(controller_version_latest_response(
            state,
            query_parameter(route.query, "forceCheck").as_deref() == Some("true"),
            controller_releases_url(state.config.controller_profile),
        )
        .await),
        ("GET", "/api/application/dump") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/octet-stream",
            body: String::new(),
        }),
        ("GET", "/api/application/version") => Ok(routing::ok_response(
            serde_json::json!(APP_VERSION).to_string(),
        )),
        ("PUT", "/api/application") => {
            if let Err(error) = mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_restart_requested(true).to_string()
            })
            .await
            {
                return Ok(routing::service_unavailable_response(&error));
            }
            schedule_lifecycle_command(state, LifecycleCommand::Restart);
            Ok(HttpResponse {
                status: "204 No Content",
                content_type: "",
                body: String::new(),
            })
        }
        ("DELETE", "/api/application") => {
            if let Err(error) = mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_restart_requested(false).to_string()
            })
            .await
            {
                return Ok(routing::service_unavailable_response(&error));
            }
            initiate_graceful_shutdown(state).await;
            Ok(routing::no_content_response())
        }
        ("POST", "/api/application/gc") => {
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_gc().to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            if route.path.starts_with("/api/v0/") {
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::ok_response(body))
            }
        }
        ("GET", "/api/server") => {
            let session = state.session.read().await.clone();
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            })
        }
        ("GET", "/api/server/status") => {
            let session = state.session.read().await;
            let connected = session.state == "connected";
            Ok(routing::ok_response(
                serde_json::json!({
                    "connected": connected,
                    "state": if connected { "logged_in" } else { "disconnected" },
                    "username": if connected {
                        session.username.clone().unwrap_or_default()
                    } else {
                        String::new()
                    },
                })
                .to_string(),
            ))
        }
        ("PUT", "/api/server") | ("POST", "/api/server") => {
            let username = extract_json_string_field(body, "username")
                .or_else(|| extract_json_string_field(body, "Username"));
            let password = extract_json_string_field(body, "password")
                .or_else(|| extract_json_string_field(body, "Password"));
            let credential_store_mode = extract_json_string_field(body, "credentialStore")
                .or_else(|| extract_json_string_field(body, "credential_store"))
                .or_else(|| extract_json_string_field(body, "CredentialStore"))
                .map(|value| config::CredentialStoreMode::parse(&value))
                .transpose();
            let credential_store_mode = match credential_store_mode {
                Ok(mode) => mode,
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 500",
                    ))
                }
            };
            match (username, password) {
                (Some(username), Some(password)) => {
                    if username.trim().is_empty() || password.is_empty() {
                        return Ok(routing::bad_request_response(
                            "username and password are required",
                        ));
                    }
                    let credentials =
                        LoginCredentials::default_client(username.trim().to_owned(), password);
                    let credential_source = credential_store::store(
                        &state.config,
                        credential_store_mode
                            .as_ref()
                            .unwrap_or(&config::CredentialStoreMode::Memory),
                        &credentials,
                    );
                    let credential_source = match credential_source {
                        Ok(source) => source,
                        Err(error) => return Ok(routing::bad_request_response(&error)),
                    };
                    {
                        let mut runtime_credentials = state.runtime_credentials.write().await;
                        *runtime_credentials = Some(credentials);
                    }
                    record_daemon_log(
                        state,
                        logging::LogLevel::Info,
                        "session",
                        format!(
                            "received Soulseek credentials for {} using {} credential store",
                            redact_username(username.trim()),
                            credential_source
                        ),
                    )
                    .await;
                }
                (Some(_), None) | (None, Some(_)) => {
                    return Ok(routing::bad_request_response(
                        "username and password must be supplied together",
                    ));
                }
                (None, None) => {}
            }
            let previous_session;
            {
                let mut session = state.session.write().await;
                let already_connecting = matches!(session.state, "connecting" | "connected");
                if already_connecting {
                    let session_snapshot = session.clone();
                    drop(session);
                    let runtime_credentials_configured =
                        state.runtime_credentials.read().await.is_some();
                    let connected_endpoint = connected_server_address(state);
                    let body = controller_server_state_json(
                        &session_snapshot,
                        &state.config,
                        runtime_credentials_configured,
                        connected_endpoint.as_deref(),
                    )
                    .to_string();
                    return Ok(if method == "PUT" && route.path.starts_with("/api/v0/") {
                        HttpResponse {
                            status: "205 Reset Content",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        routing::accepted_response(body)
                    });
                }
                previous_session = session.clone();
                session.state = "connecting";
                session.updated_at = unix_timestamp();
            }
            if let Err(error) = send_session_command(state, SessionCommand::Connect).await {
                let mut session = state.session.write().await;
                if session.state == "connecting" {
                    *session = previous_session;
                }
                return Ok(routing::service_unavailable_response(&error));
            }
            record_daemon_log(
                state,
                logging::LogLevel::Info,
                "session",
                "connect requested from API",
            )
            .await;
            let session = state.session.read().await.clone();
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            if method == "PUT" && route.path.starts_with("/api/v0/") {
                Ok(routing::ok_response(String::new()))
            } else {
                Ok(routing::accepted_response(body))
            }
        }
        ("DELETE", "/api/server") => {
            if route.path.starts_with("/api/v0/") && !body.trim().is_empty() {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Null | serde_json::Value::String(_)) => {}
                    Ok(_) | Err(_) => {
                        return Ok(routing::bad_request_response(
                            "The disconnect message must be a JSON string",
                        ));
                    }
                }
            }
            let previous_session;
            {
                let mut session = state.session.write().await;
                if matches!(session.state, "disconnecting" | "disconnected") {
                    let session_snapshot = session.clone();
                    drop(session);
                    let runtime_credentials_configured =
                        state.runtime_credentials.read().await.is_some();
                    let connected_endpoint = connected_server_address(state);
                    let body = controller_server_state_json(
                        &session_snapshot,
                        &state.config,
                        runtime_credentials_configured,
                        connected_endpoint.as_deref(),
                    )
                    .to_string();
                    return Ok(if route.path.starts_with("/api/v0/") {
                        routing::no_content_response()
                    } else {
                        routing::accepted_response(body)
                    });
                }
                previous_session = session.clone();
                session.state = "disconnecting";
                session.updated_at = unix_timestamp();
            }
            if let Err(error) = send_session_command(state, SessionCommand::Disconnect).await {
                let mut session = state.session.write().await;
                if session.state == "disconnecting" {
                    *session = previous_session;
                }
                return Ok(routing::service_unavailable_response(&error));
            }
            record_daemon_log(
                state,
                logging::LogLevel::Info,
                "session",
                "disconnect requested from API",
            )
            .await;
            let session = state.session.read().await.clone();
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            Ok(if route.path.starts_with("/api/v0/") {
                routing::no_content_response()
            } else {
                routing::accepted_response(body)
            })
        }
        ("GET", "/api/session/enabled") => {
            Ok(routing::ok_response(state.config.auth_required.to_string()))
        }
        ("POST", "/api/session") => {
            if route.path.starts_with("/api/v0/") {
                if state.config.controller_headless {
                    return Ok(HttpResponse {
                        status: "403 Forbidden",
                        content_type: "",
                        body: String::new(),
                    });
                }
                let username = extract_json_string_field(body, "username").unwrap_or_default();
                let password = extract_json_string_field(body, "password").unwrap_or_default();
                // The bundled Web UI authenticates with the configured static
                // API token in the Bearer header and intentionally does not
                // echo that token into the request body. Treat that already
                // authenticated path as a valid session bootstrap while
                // retaining the legacy username/password controller login.
                let static_api_token_session = utils::bearer_authorization_token(authorization)
                    .is_some_and(|token| state.config.api_token.as_deref() == Some(token));
                if username.trim().is_empty()
                    || (password.trim().is_empty() && !static_api_token_session)
                {
                    return Ok(routing::bad_request_response(
                        "Username and/or Password missing or invalid",
                    ));
                }
                let source = headers
                    .remote_addr
                    .map(|address| address.ip().to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                let attempt_keys = [
                    format!("source:{source}"),
                    format!("credential:{}\n{source}", username.to_ascii_lowercase()),
                ];
                let issued = unix_timestamp();
                if state
                    .login_attempts
                    .write()
                    .await
                    .is_locked(&attempt_keys, issued)
                {
                    return Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "application/json",
                        body: serde_json::json!("Too many failed login attempts. Try again later.")
                            .to_string(),
                    });
                }
                let configured_username = state
                    .controller_web_auth_username
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                let configured_password = state
                    .controller_web_auth_password
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                if !static_api_token_session
                    && (username != configured_username
                        || password.as_bytes() != configured_password.as_bytes())
                {
                    state
                        .login_attempts
                        .write()
                        .await
                        .record_failure(&attempt_keys, issued);
                    return Ok(routing::unauthorized_response());
                }
                state.login_attempts.write().await.clear(&attempt_keys);
                if static_api_token_session {
                    return Ok(routing::ok_response(
                        serde_json::json!({
                            "name": username,
                            "tokenType": "Bearer",
                            "token": "",
                            "issued": issued,
                            "notBefore": issued,
                            "expires": 0,
                        })
                        .to_string(),
                    ));
                }
                let Some((token, claims)) =
                    utils::issue_admin_jwt(&state.config, &username, issued)
                else {
                    return Ok(routing::service_unavailable_response(
                        "JWT signing credential is unavailable",
                    ));
                };
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "name": claims.name,
                        "tokenType": "Bearer",
                        "token": token,
                        "issued": claims.iat,
                        "notBefore": claims.nbf,
                        "expires": claims.exp,
                    })
                    .to_string(),
                ));
            }
            let issued = unix_timestamp();
            Ok(routing::ok_response(
                serde_json::json!({
                    "name": "slskr",
                    "tokenType": "ApiKey",
                    "token": "",
                    "tokenConfigured": state.config.api_token.is_some(),
                    "issued": issued,
                    "notBefore": issued,
                    "expires": 0,
                })
                .to_string(),
            ))
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
