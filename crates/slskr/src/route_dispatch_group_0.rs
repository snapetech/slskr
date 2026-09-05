async fn route_dispatch_group_0(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
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
            let session = state.session.read().await;
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            drop(session);
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
                    let runtime_credentials_configured =
                        state.runtime_credentials.read().await.is_some();
                    let connected_endpoint = connected_server_address(state);
                    let body = controller_server_state_json(
                        &session,
                        &state.config,
                        runtime_credentials_configured,
                        connected_endpoint.as_deref(),
                    )
                    .to_string();
                    drop(session);
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
            let session = state.session.read().await;
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            drop(session);
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
                    let runtime_credentials_configured =
                        state.runtime_credentials.read().await.is_some();
                    let connected_endpoint = connected_server_address(state);
                    let body = controller_server_state_json(
                        &session,
                        &state.config,
                        runtime_credentials_configured,
                        connected_endpoint.as_deref(),
                    )
                    .to_string();
                    drop(session);
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
            let session = state.session.read().await;
            let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
            let connected_endpoint = connected_server_address(state);
            let body = controller_server_state_json(
                &session,
                &state.config,
                runtime_credentials_configured,
                connected_endpoint.as_deref(),
            )
            .to_string();
            drop(session);
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
        ("GET", "/api/capabilities/peers") => {
            // Matches the oracle's CapabilitiesController contract: known
            // Native capability peers, not the generic connected-user list.
            let mesh = state.mesh.read().await;
            let peers = mesh.capability_service_peers_json();
            let count = peers.len();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "peers": peers,
                    "count": count,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/capabilities/mesh-peers") => {
            let mesh = state.mesh.read().await;
            let peers = mesh.capability_service_mesh_peers_json();
            let count = peers.len();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "peers": peers,
                    "count": count,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/network/stats") => {
            let include_peers = query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includePeers")
                .and_then(|(_, value)| parse_bool_value(&value))
                .unwrap_or(false);
            Ok(routing::ok_response(
                network_stats_value(state, include_peers).await.to_string(),
            ))
        }
        ("GET", "/api/hashdb/stats") => {
            let discovery = state.content_discovery.read().await;
            let persisted_entries = discovery.hash_entries().len();
            let latest_seq = discovery.latest_seq();
            let database_size_bytes = discovery.database_size_bytes();
            let mut peer_ids = discovery
                .shadow_records()
                .iter()
                .flat_map(|record| record.peer_ids.iter().cloned())
                .collect::<HashSet<_>>();
            drop(discovery);
            let features = state.controller_features.read().await;
            peer_ids.extend(hashdb_inventory_peer_ids(&features));
            let inventory = hashdb_inventory_records(&features, usize::MAX, false);
            drop(features);
            let total_flac_entries = inventory.len();
            let hashed_flac_entries = inventory
                .iter()
                .filter(|entry| {
                    entry
                        .get("hashStatusStr")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|status| status.eq_ignore_ascii_case("known"))
                })
                .count();
            let native_peers = state.mesh.read().await.capability_records.len();
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalPeers": peer_ids.len(),
                    "capabilityPeers": native_peers,
                    "totalFlacEntries": total_flac_entries,
                    "hashedFlacEntries": hashed_flac_entries,
                    "totalHashEntries": persisted_entries,
                    "currentSeqId": latest_seq,
                    "databaseSizeBytes": database_size_bytes,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/hashdb/entries") => {
            if route.path.starts_with("/api/v0/") {
                // Frozen HashDbController.GetEntries pages by sequence ID,
                // not by the local vector offset, and returns only HashDb
                // rows. Share-index projections belong to the unversioned
                // slskR compatibility helper below, not this controller.
                let limit = match query_parameter(route.query, "limit") {
                    Some(raw) => match raw.parse::<i64>() {
                        Ok(value) => usize::try_from(value.clamp(1, 1_000)).unwrap_or(1),
                        Err(_) => return Ok(routing::bad_request_response("limit is invalid")),
                    },
                    None => 100,
                };
                let offset = match query_parameter(route.query, "offset") {
                    Some(raw) => match raw.parse::<i64>() {
                        Ok(value) => u64::try_from(value.max(0)).unwrap_or(0),
                        Err(_) => return Ok(routing::bad_request_response("offset is invalid")),
                    },
                    None => 0,
                };
                let discovery = state.content_discovery.read().await;
                let (entries, _) = discovery.hash_entries_since_seq(offset, limit);
                let latest_seq = discovery.latest_seq();
                let count = entries.len();
                Ok(routing::ok_response(
                    serde_json::json!({
                        "latestSeq": latest_seq,
                        "entries": entries,
                        "count": count,
                    })
                    .to_string(),
                ))
            } else {
                let params = route.query.map(query_params).unwrap_or_default();
                let limit = params
                    .iter()
                    .find(|(key, _)| key == "limit")
                    .and_then(|(_, value)| value.parse::<usize>().ok())
                    .unwrap_or(100)
                    .clamp(1, 1_000);
                let offset = params
                    .iter()
                    .find(|(key, _)| key == "offset")
                    .and_then(|(_, value)| value.parse::<usize>().ok())
                    .unwrap_or(0);
                let discovery = state.content_discovery.read().await;
                let mut entries = discovery
                    .hash_entries()
                    .iter()
                    .map(|entry| {
                        serde_json::to_value(entry).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect::<Vec<_>>();
                let latest_seq = discovery.latest_seq();
                drop(discovery);
                let shares = state.shares.read().await;
                entries.extend(shares
                    .entries
                    .iter()
                    .filter(|entry| is_auto_retry_audio_file(&entry.filename))
                    .map(|entry| {
                        serde_json::json!({
                            "id": format!("share-{}-{}", entry.size, entry.filename),
                            "path": entry.filename,
                            "filename": entry.filename,
                            "extension": entry.extension,
                            "size": entry.size,
                            "hash": format!("{:016x}", stable_content_hash(&entry.filename, entry.size)),
                            "source": "share-index",
                        })
                    })
                    .collect::<Vec<_>>());
                let count = entries.len();
                let entries = entries
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .collect::<Vec<_>>();
                drop(shares);
                Ok(routing::ok_response(
                    serde_json::json!({
                        "latestSeq": latest_seq,
                        "entries": entries,
                        "count": count,
                        "offset": offset,
                        "limit": limit,
                    })
                    .to_string(),
                ))
            }
        }
        ("GET", path) if path.starts_with("/api/hashdb/hash/by-size/") => {
            let Some(raw_size) = path_segment_after(path, "/api/hashdb/hash/by-size/") else {
                return Ok(routing::not_found_response());
            };
            let Ok(size) = raw_size.parse::<u64>() else {
                return Ok(routing::bad_request_response(
                    "size must be a positive integer",
                ));
            };
            if size == 0 && !route.path.starts_with("/api/v0/") {
                return Ok(routing::bad_request_response(
                    "size must be a positive integer",
                ));
            }
            let discovery = state.content_discovery.read().await;
            let entries = discovery.hashes_by_size(size);
            Ok(routing::ok_response(
                serde_json::json!({
                    "count": entries.len(),
                    "entries": entries,
                })
                .to_string(),
            ))
        }
        ("GET", path) if path.starts_with("/api/hashdb/hash/") => {
            let Some(raw_key) = path_segment_after(path, "/api/hashdb/hash/") else {
                return Ok(routing::not_found_response());
            };
            let key = decoded_path_segment(raw_key).trim().to_owned();
            if route.path.starts_with("/api/v0/") && key.is_empty() {
                return Ok(routing::bad_request_response("flacKey is required"));
            }
            let discovery = state.content_discovery.read().await;
            Ok(discovery.lookup_hash(&key).map_or_else(
                || {
                    if route.path.starts_with("/api/v0/") {
                        HttpResponse {
                            status: "404 Not Found",
                            content_type: "application/json; charset=utf-8",
                            body: serde_json::json!({"error": "No hash found for key"}).to_string(),
                        }
                    } else {
                        routing::not_found_response()
                    }
                },
                |entry| {
                    routing::ok_response(
                        serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_owned()),
                    )
                },
            ))
        }
        ("POST", "/api/hashdb/hash") => {
            let entry = if route.path.starts_with("/api/v0/") {
                match hashdb_verification_entry_from_body(body) {
                    Ok(entry) => entry,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                }
            } else {
                match serde_json::from_str::<content_discovery::HashDbEntry>(body) {
                    Ok(entry) => entry,
                    Err(_) => return Ok(routing::bad_request_response("invalid hash entry")),
                }
            };
            let (
                result,
                previous_entries,
                previous_latest_seq,
                mutated_entries,
                mutated_latest_seq,
            ) = {
                let mut discovery = state.content_discovery.write().await;
                let previous_entries = discovery.hash_entries().to_vec();
                let previous_latest_seq = discovery.latest_seq();
                let result = discovery
                    .merge_hash_entries(vec![entry])
                    .map(|_| (discovery.latest_seq(), discovery.hash_entries().to_vec()));
                let mutated_entries = discovery.hash_entries().to_vec();
                let mutated_latest_seq = discovery.latest_seq();
                (
                    result,
                    previous_entries,
                    previous_latest_seq,
                    mutated_entries,
                    mutated_latest_seq,
                )
            };
            match result {
                Ok((latest_seq, entries)) => {
                    if let Err(error) = persist_hash_db_snapshot(state, &entries, latest_seq).await
                    {
                        rollback_hash_db_entries_if_unchanged(
                            state,
                            previous_entries,
                            previous_latest_seq,
                            &mutated_entries,
                            mutated_latest_seq,
                        )
                        .await;
                        return Ok(routing::internal_server_error_response(&error));
                    }
                    if route.path.starts_with("/api/v0/") {
                        Ok(routing::ok_response(
                            serde_json::json!({"stored": true}).to_string(),
                        ))
                    } else {
                        Ok(routing::ok_response(
                            serde_json::json!({
                                "stored": true,
                                "latestSeq": latest_seq,
                            })
                            .to_string(),
                        ))
                    }
                }
                Err(error) => Ok(content_discovery_error_response(state, error).await),
            }
        }
        ("POST", "/api/hashdb/sync/merge") => {
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(_) => return Ok(routing::bad_request_response("invalid hash merge request")),
            };
            let from_user = value
                .get("fromUser")
                .or_else(|| value.get("username"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("http-sync")
                .to_owned();
            let sync_settings = state
                .advanced_networking
                .read()
                .await
                .mesh_sync_security
                .clone();
            if state
                .mesh
                .write()
                .await
                .sync_is_quarantined(&from_user, unix_timestamp())
            {
                return Ok(HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "application/json; charset=utf-8",
                    body: serde_json::json!({"error":"mesh peer is quarantined"}).to_string(),
                });
            }
            let entries_value = match value.get("entries") {
                Some(entries) => entries,
                None => return Ok(routing::bad_request_response("entries are required")),
            };
            let max_entries = if route.path.starts_with("/api/v0/") {
                content_discovery::MAX_MESH_MERGE_ENTRIES
            } else {
                content_discovery::MAX_HASH_MERGE_ENTRIES
            };
            if json_array_exceeds_limit(entries_value, max_entries) {
                return Ok(routing::bad_request_response(&format!(
                    "entries must contain at most {max_entries} entries"
                )));
            }
            let entries = match serde_json::from_value::<Vec<content_discovery::HashDbEntry>>(
                entries_value.clone(),
            ) {
                Ok(entries) => entries,
                Err(_) => return Ok(routing::bad_request_response("entries are required")),
            };
            let received = entries.len();
            let (
                result,
                previous_entries,
                previous_latest_seq,
                mutated_entries,
                mutated_latest_seq,
            ) = {
                let mut discovery = state.content_discovery.write().await;
                let previous_entries = discovery.hash_entries().to_vec();
                let previous_latest_seq = discovery.latest_seq();
                let result = if route.path.starts_with("/api/v0/") {
                    discovery
                        .merge_hash_entries_from_mesh(entries)
                        .map(|(merged, _skipped)| {
                            (
                                merged,
                                discovery.latest_seq(),
                                discovery.hash_entries().to_vec(),
                            )
                        })
                } else {
                    discovery.merge_hash_entries(entries).map(|merged| {
                        (
                            merged,
                            discovery.latest_seq(),
                            discovery.hash_entries().to_vec(),
                        )
                    })
                };
                let mutated_entries = discovery.hash_entries().to_vec();
                let mutated_latest_seq = discovery.latest_seq();
                (
                    result,
                    previous_entries,
                    previous_latest_seq,
                    mutated_entries,
                    mutated_latest_seq,
                )
            };
            match result {
                Ok((merged, latest_seq, entries)) => {
                    if let Err(error) = persist_hash_db_snapshot(state, &entries, latest_seq).await
                    {
                        rollback_hash_db_entries_if_unchanged(
                            state,
                            previous_entries,
                            previous_latest_seq,
                            &mutated_entries,
                            mutated_latest_seq,
                        )
                        .await;
                        return Ok(routing::internal_server_error_response(&error));
                    }
                    Ok(routing::ok_response(
                        serde_json::json!({
                            "received": received,
                            "merged": merged,
                            "latestSeq": latest_seq,
                        })
                        .to_string(),
                    ))
                }
                Err(error) => {
                    let invalid = u32::try_from(received).unwrap_or(u32::MAX);
                    let rate_limited = state.mesh.write().await.record_invalid_sync_entries(
                        &from_user,
                        invalid,
                        &sync_settings,
                        unix_timestamp(),
                    );
                    if rate_limited {
                        record_peer_security_violation(state, &from_user).await;
                        Ok(HttpResponse {
                            status: "429 Too Many Requests",
                            content_type: "application/json; charset=utf-8",
                            body: serde_json::json!({"error":"mesh invalid-entry rate limit exceeded"}).to_string(),
                        })
                    } else {
                        Ok(content_discovery_error_response(state, error).await)
                    }
                }
            }
        }
        ("POST", "/api/virtualsoulfind/shadow-index/sync/merge") => {
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "invalid shadow-index merge request",
                    ))
                }
            };
            let records_value = match value.get("records").or_else(|| value.get("entries")) {
                Some(records) => records,
                None => return Ok(routing::bad_request_response("records are required")),
            };
            if json_array_exceeds_limit(records_value, content_discovery::MAX_SHADOW_MERGE_RECORDS)
            {
                return Ok(routing::bad_request_response(&format!(
                    "records must contain at most {} records",
                    content_discovery::MAX_SHADOW_MERGE_RECORDS
                )));
            }
            let records = match serde_json::from_value::<Vec<content_discovery::ShadowIndexRecord>>(
                records_value.clone(),
            ) {
                Ok(records) => records,
                Err(_) => return Ok(routing::bad_request_response("records are required")),
            };
            let realm_indexes = match value
                .get("realmIndexes")
                .or_else(|| value.get("realm_indexes"))
            {
                Some(indexes) => {
                    if json_array_exceeds_limit(indexes, realm_subject_index::MAX_INDEXES) {
                        return Ok(routing::bad_request_response(&format!(
                            "realmIndexes must contain at most {} indexes",
                            realm_subject_index::MAX_INDEXES
                        )));
                    }
                    match serde_json::from_value::<Vec<serde_json::Value>>(indexes.clone()) {
                        Ok(indexes) => indexes,
                        Err(_) => {
                            return Ok(routing::bad_request_response(
                                "realmIndexes must be an array of objects",
                            ))
                        }
                    }
                }
                None => Vec::new(),
            };
            let received = records.len();
            let mut discovery = state.content_discovery.write().await;
            let mut realm_store = state.realm_subject_indexes.write().await;
            if !realm_indexes.is_empty() {
                if let Err(error) = realm_store.validate_indexes(&realm_indexes) {
                    return Ok(routing::bad_request_response(&error));
                }
            }
            match discovery.merge_shadow_records(records) {
                Ok(merged) => {
                    drop(discovery);
                    let indexes_merged = if realm_indexes.is_empty() {
                        0
                    } else {
                        match realm_store.merge_indexes(realm_indexes) {
                            Ok(merged) => merged,
                            Err(error) => return Ok(routing::bad_request_response(&error)),
                        }
                    };
                    Ok(routing::ok_response(
                        serde_json::json!({
                            "received": received,
                            "merged": merged,
                            "realmIndexesMerged": indexes_merged,
                        })
                        .to_string(),
                    ))
                }
                Err(error) => Ok(content_discovery_error_response(state, error).await),
            }
        }
        (method, path) if path.starts_with("/api/virtualsoulfind/v2") => {
            Ok(route_virtual_soulfind_v2(method, path, route.query, body, state).await)
        }
        ("GET", path) if path.starts_with("/api/virtualsoulfind/shadow-index/") => {
            let Some(raw_recording_id) =
                path_segment_after(path, "/api/virtualsoulfind/shadow-index/")
            else {
                return Ok(routing::not_found_response());
            };
            let recording_id = decoded_path_segment(raw_recording_id);
            let discovery = state.content_discovery.read().await;
            if state.config.controller_profile == ControllerProfile::Native {
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "variants": virtual_soulfind_legacy_variants(&discovery, &recording_id),
                    })
                    .to_string(),
                ));
            }
            let record = discovery
                .shadow_records()
                .iter()
                .find(|record| record.recording_id.eq_ignore_ascii_case(&recording_id));
            Ok(routing::ok_response(
                record
                    .map_or_else(
                        || {
                            serde_json::json!({
                                "recordingId": recording_id,
                                "peerIds": [],
                                "totalPeerCount": 0,
                                "variants": [],
                            })
                        },
                        |record| {
                            serde_json::json!({
                                "recordingId": record.recording_id,
                                "peerIds": record.peer_ids,
                                "totalPeerCount": record.peer_ids.len(),
                                "variants": [],
                                "updatedAt": record.updated_at,
                            })
                        },
                    )
                    .to_string(),
            ))
        }
        ("POST", "/api/hashdb/backfill/from-history") => {
            Ok(hashdb_backfill_from_history_response(route.query, state).await)
        }
        ("POST", path) if path.starts_with("/api/hashdb/backfill/from-history") => {
            Ok(routing::not_found_response())
        }
        ("GET", "/api/mesh/stats") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let known_mesh_peers = mesh.candidate_usernames(&users).len();
            let rejected_messages = mesh.sync_rejected_messages;
            let quarantine_events = mesh.sync_quarantine_events;
            let quarantined_peers = mesh.sync_quarantined_until.len();
            let total_syncs = mesh.sync_merge_total;
            let successful_syncs = mesh.sync_merge_successful;
            let failed_syncs = mesh.sync_merge_failed;
            let total_entries_received = mesh.sync_entries_received;
            let total_entries_sent = mesh.sync_entries_sent;
            let skipped_entries = mesh.sync_skipped_entries;
            let rate_limit_violations = mesh.sync_rate_limit_violations;
            let total_entries_merged = mesh.sync_entries_merged;
            drop(mesh);
            drop(users);
            let current_seq_id = state.content_discovery.read().await.latest_seq();
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalSyncs": total_syncs,
                    "successfulSyncs": successful_syncs,
                    "failedSyncs": failed_syncs,
                    "totalEntriesReceived": total_entries_received,
                    "totalEntriesSent": total_entries_sent,
                    "totalEntriesMerged": total_entries_merged,
                    "rejectedMessages": rejected_messages,
                    "skippedEntries": skipped_entries,
                    "signatureVerificationFailures": 0,
                    "reputationBasedRejections": 0,
                    "rateLimitViolations": rate_limit_violations,
                    "quarantinedPeers": quarantined_peers,
                    "quarantineEvents": quarantine_events,
                    "proofOfPossessionFailures": 0,
                    "currentSeqId": current_seq_id,
                    "knownMeshPeers": known_mesh_peers,
                    "warnings": [],
                })
                .to_string(),
            ))
        }
        ("GET", "/api/mesh/peers") if route.path.starts_with("/api/v0/") => {
            let mesh = state.mesh.read().await;
            let peers = mesh
                .capability_records_json()
                .into_iter()
                .filter(|record| record["meshCapable"] == true)
                .collect::<Vec<_>>();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "count": peers.len(),
                    "peers": peers,
                    "overlay": [],
                })
                .to_string(),
            ))
        }
        ("GET", "/api/mesh/peers") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let mut value = serde_json::from_str::<serde_json::Value>(&mesh.users_json(&users))
                .unwrap_or_else(|_| serde_json::json!({}));
            value["peers"] = value["users"].clone();
            let active_connections = match state.private_gateway.as_ref() {
                Some(gateway) => gateway.active_connection_count().await,
                None => 0,
            };
            value["overlay"] = serde_json::json!({
                "enabled": state.private_gateway.is_some(),
                "activeConnections": active_connections,
            });
            let body = value.to_string();
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(body))
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
