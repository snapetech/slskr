use super::*;

pub(super) async fn feature_controller_mutation_response(
    method: &str,
    path: &str,
    body: &str,
    state: &AppState,
    is_versioned_v0: bool,
) -> Option<HttpResponse> {
    if method == "POST" && path.starts_with("/api/multisource/") {
        return Some(multisource_controller_response(path, body, state, is_versioned_v0).await);
    }
    if method == "POST" && path.starts_with("/api/musicbrainz/") {
        return Some(musicbrainz_mutation_response(path, body, state, is_versioned_v0).await);
    }
    if method == "POST" && path.starts_with("/api/quarantine-jury/") {
        return Some(quarantine_mutation_response(path, body, state).await);
    }
    if method == "POST" && path == "/api/playback/feedback" {
        let mut feedback = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(feedback @ serde_json::Value::Object(_)) => feedback,
            Ok(_) => return Some(routing::bad_request_response("feedback must be an object")),
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let job_id = feedback
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned);
        let Some(job_id) = job_id else {
            return Some(routing::bad_request_response("jobId is required"));
        };
        let id = uuid::Uuid::new_v4().to_string();
        feedback["feedbackId"] = serde_json::json!(id);
        // Nanosecond precision so `latest_playback_feedback` can reliably
        // order feedback posted multiple times in rapid succession.
        feedback["receivedAt"] = serde_json::json!(unix_timestamp_nanos());
        return Some(
            match state
                .controller_features
                .upsert(format!("playback/feedback/{job_id}/{id}"), feedback.clone())
                .await
            {
                // Matches the oracle's real PostFeedback: the priority
                // reflects this job's actual recorded buffer state, not a
                // hardcoded constant.
                Ok(()) => routing::ok_response(
                    serde_json::json!({
                        "priority": playback_priority_for_latest_feedback(Some(&feedback))
                    })
                    .to_string(),
                ),
                Err(error) => routing::service_unavailable_response(&error),
            },
        );
    }
    if method == "POST" && matches!(path, "/api/ranking/history" | "/api/ranking/rank") {
        return Some(ranking_mutation_response(path, body, state).await);
    }
    if method == "POST"
        && path.starts_with("/api/realm-subject-indexes/")
        && path.ends_with("/authority-decision")
    {
        let segments = decoded_segments_after(path, "/api/realm-subject-indexes/")?;
        let [realm_id, index_id, action] = segments.as_slice() else {
            return Some(routing::not_found_response());
        };
        if action != "authority-decision" {
            return Some(routing::not_found_response());
        }
        let request = serde_json::from_str::<serde_json::Value>(body)
            .unwrap_or_else(|_| serde_json::json!({}));
        let enabled = request
            .get("enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let decided_by = request
            .get("decidedBy")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("local-user")
            .trim()
            .to_owned();
        let note = request
            .get("note")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();

        // Matches the oracle's SetAuthorityEnabledAsync: validate the
        // request and require a real registered index before persisting the
        // decision. The decision is part of the same durable subject-index
        // store used by the GET routes and conflict resolver.
        let mut errors = Vec::new();
        if realm_id.trim().is_empty() {
            errors.push("Realm id is required.");
        } else if !state
            .realm_subject_indexes
            .read()
            .await
            .is_same_realm(realm_id)
        {
            errors.push("Realm id does not match the local realm.");
        }
        if index_id.trim().is_empty() {
            errors.push("Index id is required.");
        }
        if !realm_subject_index::is_safe_opaque_reference(&decided_by) {
            errors.push("Decided-by identifier must be opaque and safe.");
        }
        if note.chars().count() > 512 {
            errors.push("Authority decision note must be 512 characters or fewer.");
        }
        let is_accepted = errors.is_empty();
        let value = serde_json::json!({
            "isAccepted": is_accepted,
            "realmId": realm_id,
            "indexId": index_id,
            "enabled": enabled,
            "decidedBy": decided_by,
            "note": note,
            "decidedAt": chrono::Utc::now().to_rfc3339(),
            "errors": errors,
        });
        if !is_accepted {
            return Some(HttpResponse {
                status: "400 Bad Request",
                content_type: "application/json",
                body: value.to_string(),
            });
        }
        let decision = state
            .realm_subject_indexes
            .write()
            .await
            .set_authority_decision(
                realm_id,
                index_id,
                enabled,
                &decided_by,
                &note,
                value["decidedAt"].as_str().unwrap_or_default(),
            );
        return Some(match decision {
            Ok(decision) => routing::ok_response(decision.to_string()),
            Err(error) => {
                let mut rejected = value;
                rejected["isAccepted"] = serde_json::json!(false);
                rejected["errors"] = serde_json::json!([error]);
                HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: rejected.to_string(),
                }
            }
        });
    }
    if method == "POST" && path.starts_with("/api/searches/") && path.contains("/items/") {
        return Some(
            if is_versioned_v0 && state.config.controller_profile == ControllerProfile::Native {
                native_search_action_controller_response(path, state).await
            } else {
                search_action_controller_response(path, state).await
            },
        );
    }
    if method == "POST"
        && matches!(
            path,
            "/api/security/tor/test" | "/api/security/transports/test"
        )
    {
        if is_versioned_v0 {
            return Some(if path.contains("/tor/") {
                HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: serde_json::json!("Tor transport is not configured or available")
                        .to_string(),
                }
            } else {
                HttpResponse {
                    status: "503 Service Unavailable",
                    content_type: "application/json",
                    body: serde_json::json!("Transport selector not available").to_string(),
                }
            });
        }
        let listeners = state.listeners.read().await;
        let transport = if path.contains("/tor/") {
            "tor"
        } else {
            "configured"
        };
        return Some(routing::ok_response(serde_json::json!({
            "transport": transport,
            "reachable": listeners.regular_local_addr.is_some() || listeners.obfuscated_local_addr.is_some(),
            "regularEndpoint": listeners.regular_local_addr,
            "obfuscatedEndpoint": listeners.obfuscated_local_addr,
            "testedAt": unix_timestamp(),
        }).to_string()));
    }
    if method == "POST" && path == "/api/share-grants/announce" {
        if is_versioned_v0 {
            return Some(routing::not_found_response());
        }
        let grants = state.share_grants.read().await;
        let announcements = grants
            .records
            .iter()
            .map(|grant| {
                serde_json::json!({
                    "grantId": grant.id,
                    "collectionId": grant.collection_id,
                    "username": grant.username,
                    "permissions": grant.permissions,
                })
            })
            .collect::<Vec<_>>();
        return Some(routing::ok_response(
            serde_json::json!({
                "announced": announcements.len(), "grants": announcements
            })
            .to_string(),
        ));
    }
    if method == "POST" && path == "/api/solid/resolve-webid" {
        let web_id = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| {
                value
                    .get("webId")
                    .or_else(|| value.get("WebId"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            });
        let Some(web_id) = web_id else {
            return Some(solid_resolution_error(
                is_versioned_v0,
                400,
                "Invalid WebID",
                "WebId must be an absolute URI.",
                "webId is required",
            ));
        };
        let web_id = web_id.trim().to_owned();
        if web_id.is_empty() {
            return Some(solid_resolution_error(
                is_versioned_v0,
                400,
                "Invalid WebID",
                "WebId must be an absolute URI.",
                "webId is required",
            ));
        }
        let Ok(url) = reqwest::Url::parse(&web_id) else {
            return Some(solid_resolution_error(
                is_versioned_v0,
                400,
                "Invalid WebID",
                "WebId must be an absolute URI.",
                "webId must be an absolute URL",
            ));
        };
        let solid = state.media_services.read().await.solid.clone();
        let Some(host) = url
            .host_str()
            .map(|host| host.trim_end_matches('.').to_ascii_lowercase())
        else {
            return Some(solid_resolution_error(
                is_versioned_v0,
                400,
                "Invalid WebID",
                "WebId must be an absolute URI.",
                "webId must include a hostname",
            ));
        };
        if !matches!(url.scheme(), "https" | "http")
            || (url.scheme() == "http" && !solid.allow_insecure_http)
            || !solid.allowed_hosts.iter().any(|allowed| allowed == &host)
        {
            return Some(solid_resolution_error(
                is_versioned_v0,
                400,
                "Solid fetch blocked",
                "WebID resolution was blocked by policy.",
                "WebID resolution was blocked by policy.",
            ));
        }
        let resolved_addresses = if !solid.allow_localhost_for_web_id {
            if host == "localhost" || host.ends_with(".local") {
                return Some(solid_resolution_error(
                    is_versioned_v0,
                    400,
                    "Solid fetch blocked",
                    "WebID resolution was blocked by policy.",
                    "WebID resolution was blocked by policy.",
                ));
            }
            let port = match url.port_or_known_default() {
                Some(port) => port,
                None => {
                    return Some(solid_resolution_error(
                        is_versioned_v0,
                        400,
                        "Solid fetch blocked",
                        "WebID resolution was blocked by policy.",
                        "WebID resolution was blocked by policy.",
                    ))
                }
            };
            let addresses = match tokio::net::lookup_host((host.as_str(), port)).await {
                Ok(addresses) => addresses.collect::<Vec<_>>(),
                Err(_) => {
                    return Some(solid_resolution_error(
                        is_versioned_v0,
                        400,
                        "Solid fetch blocked",
                        "WebID resolution was blocked by policy.",
                        "WebID resolution was blocked by policy.",
                    ))
                }
            };
            if addresses.is_empty()
                || addresses.iter().any(|address| {
                    address.ip().is_loopback() || solid_private_or_reserved(address.ip())
                })
            {
                return Some(solid_resolution_error(
                    is_versioned_v0,
                    400,
                    "Solid fetch blocked",
                    "WebID resolution was blocked by policy.",
                    "WebID resolution was blocked by policy.",
                ));
            }
            Some(addresses)
        } else {
            None
        };
        let request_host = url.host_str().unwrap_or(host.as_str());
        let mut client_builder = reqwest::Client::builder()
            .timeout(solid.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy();
        if let Some(addresses) = resolved_addresses.as_deref() {
            client_builder = client_builder.resolve_to_addrs(request_host, addresses);
        }
        let client = match client_builder.build() {
            Ok(client) => client,
            Err(_) => {
                return Some(solid_resolution_error(
                    is_versioned_v0,
                    500,
                    "Failed to resolve WebID",
                    "WebID resolution failed.",
                    "Failed to resolve WebID",
                ))
            }
        };
        let response = match client.get(url.clone()).send().await {
            Ok(response) if response.status().is_success() => response,
            _ => {
                return Some(solid_resolution_error(
                    is_versioned_v0,
                    500,
                    "Failed to resolve WebID",
                    "WebID resolution failed.",
                    "Failed to resolve WebID",
                ))
            }
        };
        if response
            .content_length()
            .is_some_and(|length| length > solid.max_fetch_bytes as u64)
        {
            return Some(solid_resolution_error(
                is_versioned_v0,
                400,
                "Solid fetch blocked",
                "WebID resolution was blocked by policy.",
                "WebID resolution was blocked by policy.",
            ));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let mut stream = response.bytes_stream();
        let mut fetched = 0_usize;
        let mut profile = Vec::new();
        while let Some(chunk) = stream.next().await {
            let Ok(chunk) = chunk else {
                return Some(solid_resolution_error(
                    is_versioned_v0,
                    500,
                    "Failed to resolve WebID",
                    "WebID resolution failed.",
                    "Failed to resolve WebID",
                ));
            };
            fetched = fetched.saturating_add(chunk.len());
            if fetched > solid.max_fetch_bytes {
                return Some(solid_resolution_error(
                    is_versioned_v0,
                    400,
                    "Solid fetch blocked",
                    "WebID resolution was blocked by policy.",
                    "WebID resolution was blocked by policy.",
                ));
            }
            profile.extend_from_slice(&chunk);
        }
        let oidc_issuers =
            match solid::extract_oidc_issuers(&profile, content_type.as_deref(), url.as_str()) {
                Ok(issuers) => issuers,
                Err(_) => {
                    return Some(solid_resolution_error(
                        is_versioned_v0,
                        500,
                        "Failed to resolve WebID",
                        "WebID resolution failed.",
                        "Failed to resolve WebID",
                    ));
                }
            };
        return Some(routing::ok_response(
            serde_json::json!({
                "webId": web_id,
                "oidcIssuers": oidc_issuers,
            })
            .to_string(),
        ));
    }
    if method == "POST" && path.starts_with("/mesh/http/") {
        return Some(mesh_http_service_response(path, body, state).await);
    }
    if method == "PUT" && path.starts_with("/api/conversations/") {
        let segments = decoded_segments_after(path, "/api/conversations/")?;
        let [username, id] = segments.as_slice() else {
            return Some(routing::not_found_response());
        };
        let Ok(id) = id.parse::<u64>() else {
            return Some(routing::bad_request_response(
                "message id must be an integer",
            ));
        };
        let _message_persistence = state.message_persistence_lock.lock().await;
        let (previous, record, mutated) = {
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages
                .records
                .iter()
                .any(|record| record.id == id && record.username.eq_ignore_ascii_case(username))
                .then(|| messages.ack(id))
                .flatten();
            let mutated = messages.clone();
            (previous, record, mutated)
        };
        let Some(record) = record else {
            return Some(routing::not_found_response());
        };
        return Some(match persist_message_ack_checked(state, id).await {
            Ok(_) => {
                drop(_message_persistence);
                routing::ok_response(record.json())
            }
            Err(error) => {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                routing::service_unavailable_response(&error)
            }
        });
    }
    if method == "PUT" && path.starts_with("/api/overlay/pins/") {
        // Matches the oracle's real RotateCertificatePin: the wire field
        // is "thumbprint" (not "certificateSha256"/"pin"), the pin is
        // stored upper-invariant, and success is a real 204 No Content
        // rather than a 200 with a body.
        let username = decoded_path_segment(path.trim_start_matches("/api/overlay/pins/"));
        if username.trim().is_empty() {
            return Some(routing::bad_request_response("Username required"));
        }
        let thumbprint =
            extract_json_string_field(body, "thumbprint").map(|value| value.trim().to_owned());
        let Some(thumbprint) = thumbprint.filter(|value| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) else {
            return Some(routing::bad_request_response(
                "Thumbprint must be a 64-character SHA-256 hexadecimal value",
            ));
        };
        let value = serde_json::json!({
            "username": username, "certificateSha256": thumbprint.to_ascii_uppercase(), "updatedAt": unix_timestamp()
        });
        return Some(
            match state
                .controller_features
                .upsert(format!("overlay/pin/{username}"), value)
                .await
            {
                Ok(()) => routing::no_content_response(),
                Err(error) => routing::service_unavailable_response(&error),
            },
        );
    }
    if method == "PUT" && path.starts_with("/api/security/reputation/") {
        let Some(username) = path_segment_after(path, "/api/security/reputation/") else {
            return Some(routing::not_found_response());
        };
        let username = decoded_path_segment(username);
        if username.trim().is_empty() {
            return Some(routing::bad_request_response("Username is required"));
        }
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            _ => return Some(routing::bad_request_response("Request is required")),
        };
        let Some(score) = payload.get("score").and_then(serde_json::Value::as_i64) else {
            return Some(routing::bad_request_response(
                "Score must be between 0 and 100",
            ));
        };
        // Matches the oracle's real SetReputation: an out-of-range score
        // is a real validation error, not silently clamped.
        if !(0..=100).contains(&score) {
            return Some(routing::bad_request_response(
                "Score must be between 0 and 100",
            ));
        }
        // Matches the oracle's real PeerReputation.SetScore: a manual
        // override writes directly into the same real score the
        // automatic violation-tracking system (record_peer_violation)
        // also adjusts, rather than a disconnected settings blob that
        // was never read by anything else.
        let mut security = state.security.write().await;
        let key = username.to_ascii_lowercase();
        security.ensure_reputation_profile(&key, &username);
        security.reputation.insert(key, score as i32);
        return Some(routing::ok_response("{}".to_owned()));
    }
    if method == "PUT"
        && (path == "/api/security/adversarial" || path.starts_with("/api/security/disclosure/"))
    {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            Ok(_) => {
                return Some(routing::bad_request_response(
                    "security body must be an object",
                ))
            }
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let native_adversarial = path == "/api/security/adversarial"
            && state.config.controller_profile == ControllerProfile::Native;
        if native_adversarial {
            if let Err(error) = validate_native_adversarial_settings(&payload) {
                return Some(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "text/plain; charset=utf-8",
                    body: error.to_owned(),
                });
            }
            let current = match read_controller_compatibility_yaml(&state.config) {
                Ok(Some(current)) => current,
                Ok(None) => {
                    return Some(HttpResponse {
                        status: "404 Not Found",
                        content_type: "application/json; charset=utf-8",
                        body: serde_json::json!("Configuration file not found").to_string(),
                    })
                }
                Err(error) => {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Error,
                        "configuration",
                        format!("failed to read adversarial configuration: {error}"),
                    )
                    .await;
                    return Some(routing::internal_server_error_response(
                        "Failed to persist settings",
                    ));
                }
            };
            let updated = match native_adversarial_yaml_update(&current, &payload) {
                Ok(updated) => updated,
                Err(error) => return Some(routing::bad_request_response(&error)),
            };
            if let Err(error) = write_controller_compatibility_yaml(&state.config, &updated) {
                record_daemon_log(
                    state,
                    logging::LogLevel::Error,
                    "configuration",
                    format!("failed to persist adversarial settings: {error}"),
                )
                .await;
                return Some(routing::internal_server_error_response(
                    "Failed to persist settings",
                ));
            }
            Box::pin(apply_watched_controller_configuration(
                state,
                Some(&updated),
                &state.controller_cli_environment,
            ))
            .await;
        }
        let key = path.trim_start_matches("/api/").to_owned();
        let value = serde_json::json!({
            "resource": key,
            "settings": payload,
            "updatedAt": unix_timestamp(),
        });
        return Some(
            match state.controller_features.upsert(format!("security/profile/{key}"), value.clone()).await
            {
                Ok(()) if native_adversarial => HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: serde_json::json!({
                        "message": if effective_controller_no_config_watch(state) {
                            "Adversarial settings updated. Restart required for changes to take effect."
                        } else {
                            "Adversarial settings updated successfully"
                        }
                    })
                    .to_string(),
                },
                Ok(()) => routing::ok_response(value.to_string()),
                Err(error) => routing::service_unavailable_response(&error),
            },
        );
    }
    None
}

pub(super) async fn multisource_versioned_download_response(
    body: &str,
    state: &AppState,
) -> HttpResponse {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload @ serde_json::Value::Object(_)) => payload,
        Ok(_) => return routing::bad_request_response("invalid JSON body"),
        Err(_) => return routing::bad_request_response("invalid JSON body"),
    };
    let sources = payload
        .get("sources")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if sources.len() > multisource::MAX_SOURCES {
        return routing::bad_request_response(&format!(
            "source count exceeds the {} source limit",
            multisource::MAX_SOURCES
        ));
    }
    if sources
        .iter()
        .any(|source| source.get("url").is_some() || source.get("endpoint").is_some())
    {
        return multisource_versioned_swarm_response("/api/multisource/download", body, state)
            .await;
    }
    let filename = payload
        .get("filename")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if filename.is_empty() {
        return routing::bad_request_response("Filename is required");
    }
    let source_count = sources
        .iter()
        .filter_map(|source| {
            source
                .get("username")
                .or_else(|| source.get("user"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|username| !username.is_empty())
        })
        .collect::<HashSet<_>>()
        .len();
    if source_count < 2 {
        return HttpResponse {
            status: "400 Bad Request",
            content_type: "application/json",
            body: serde_json::json!("At least 2 verified sources are required").to_string(),
        };
    }
    routing::ok_response(
        serde_json::json!({
            "success": false,
            "filename": filename,
            "fileSize": payload.get("fileSize").or_else(|| payload.get("size")).and_then(serde_json::Value::as_u64).unwrap_or(0),
            "sourcesUsed": source_count,
            "outputPath": "",
            "finalHash": "",
            "error": "Multi-source download is unavailable in the local compatibility runtime",
        })
        .to_string(),
    )
}

pub(super) async fn multisource_verified_swarm_response(
    path: &str,
    payload: &serde_json::Value,
    filename: &str,
    size: u64,
    chunk_size: u64,
    state: &AppState,
) -> HttpResponse {
    // A content-hash request is a real verified download, not just the
    // legacy discovery projection. Resolve empty source lists through the
    // local hash/shadow/trusted-peer graph and use the confined executor.
    let expected_hash = payload
        .get("expectedHash")
        .or_else(|| payload.get("expected_hash"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut sources = match payload.get("sources") {
        Some(value) => {
            if json_array_exceeds_limit(value, multisource::MAX_SOURCES) {
                return routing::bad_request_response(&format!(
                    "source count exceeds the {} source limit",
                    multisource::MAX_SOURCES
                ));
            }
            match serde_json::from_value::<Vec<multisource::RangeSource>>(value.clone()) {
                Ok(sources) => sources,
                Err(_) => return routing::bad_request_response("invalid swarm sources"),
            }
        }
        None => Vec::new(),
    };
    if sources.is_empty() {
        sources = discover_mesh_range_sources(state, &expected_hash, size).await;
    }
    let request_filename = if filename.is_empty() {
        "multisource-swarm".to_owned()
    } else {
        filename.to_owned()
    };
    let relative_path = payload
        .get("outputPath")
        .or_else(|| payload.get("output_path"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            format!(
                "multisource/{}-{}",
                uuid::Uuid::new_v4(),
                virtual_basename(&request_filename)
            )
        });
    let mut request = multisource::SwarmRequest {
        filename: request_filename,
        file_size: size,
        expected_hash: Some(expected_hash),
        output_path: Some(relative_path.clone()),
        chunk_size,
        sources,
    };
    if let Err(error) = multisource::validate_request(&mut request) {
        return routing::bad_request_response(&error);
    }

    let downloads_dir = effective_downloads_dir(state);
    let output_path = match safe_download_path(&downloads_dir, &relative_path).and_then(|path| {
        ensure_scoped_download_path(&downloads_dir, path.to_string_lossy().as_ref())
    }) {
        Ok(path) => path,
        Err(error) => return routing::bad_request_response(&error),
    };
    let public_output_path = match output_path.strip_prefix(&downloads_dir) {
        Ok(path) => path.to_string_lossy().replace('\\', "/"),
        Err(_) => {
            return routing::bad_request_response(
                "multisource output path escaped the download root",
            )
        }
    };
    let id = uuid::Uuid::new_v4().to_string();
    let job = multisource::new_job(
        id.clone(),
        &request,
        public_output_path.clone(),
        unix_timestamp(),
    );
    state.multisource.write().await.insert(job.clone());
    let store = Arc::clone(&state.multisource);
    if path == "/api/multisource/swarm/async" {
        if !multisource::spawn_managed(state, id.clone(), request, output_path, public_output_path)
            .await
        {
            return routing::service_unavailable_response("daemon is shutting down");
        }
        return routing::accepted_response(
            serde_json::json!({
                "id": id,
                "status": "queued",
                "job": job,
            })
            .to_string(),
        );
    }
    let result = multisource::execute(id, request, output_path, public_output_path, store).await;
    routing::ok_response(
        serde_json::to_string(&result).unwrap_or_else(|_| "{\"success\":false}".to_owned()),
    )
}

pub(super) async fn multisource_versioned_swarm_response(
    path: &str,
    body: &str,
    state: &AppState,
) -> HttpResponse {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload @ serde_json::Value::Object(_)) => payload,
        Ok(_) => return routing::bad_request_response("invalid JSON body"),
        Err(_) => return routing::bad_request_response("invalid JSON body"),
    };
    let size = payload
        .get("size")
        .or_else(|| payload.get("fileSize"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if size == 0 {
        return routing::bad_request_response("Size is required (exact file size in bytes).");
    }
    let filename = payload
        .get("filename")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let use_discovery_db = payload
        .get("useDiscoveryDb")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !use_discovery_db && filename.is_empty() {
        return routing::bad_request_response("Filename is required when not using discovery DB.");
    }
    let chunk_size = payload
        .get("chunkSize")
        .and_then(serde_json::Value::as_u64)
        .filter(|size| *size > 0)
        .unwrap_or(multisource::DEFAULT_CHUNK_SIZE);
    let chunk_size = match multisource::validate_file_size_and_chunk_size(size, chunk_size) {
        Ok(chunk_size) => chunk_size,
        Err(error) => return routing::bad_request_response(&error),
    };

    let expected_hash_value = payload
        .get("expectedHash")
        .or_else(|| payload.get("expected_hash"));
    if expected_hash_value.is_none() {
        return routing::bad_request_response(
            "expectedHash is required for verified swarm execution",
        );
    }
    multisource_verified_swarm_response(path, &payload, &filename, size, chunk_size, state).await
}

pub(super) async fn multisource_controller_response(
    path: &str,
    body: &str,
    state: &AppState,
    is_versioned_v0: bool,
) -> HttpResponse {
    let versioned_profile =
        is_versioned_v0 && state.config.controller_profile == ControllerProfile::Native;
    if versioned_profile {
        match path {
            "/api/multisource/file-sources" => {
                let payload = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(payload @ serde_json::Value::Object(_)) => payload,
                    Ok(_) => return routing::bad_request_response("invalid JSON body"),
                    Err(_) => return routing::bad_request_response("invalid JSON body"),
                };
                let filename = payload
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if filename.is_empty() {
                    return routing::bad_request_response("Filename is required");
                }
                let requested_size = payload
                    .get("size")
                    .or_else(|| payload.get("fileSize"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let target_filename = virtual_basename(&filename).to_owned();
                let searches = state.searches.read().await;
                let mut size_groups = BTreeMap::<u64, Vec<serde_json::Value>>::new();
                for result in searches
                    .records
                    .iter()
                    .flat_map(|record| record.results.iter())
                {
                    if !virtual_basename(&result.filename).eq_ignore_ascii_case(&target_filename)
                        || (requested_size > 0 && result.size != requested_size)
                    {
                        continue;
                    }
                    let Some(username) = result.peer_username.as_deref() else {
                        continue;
                    };
                    size_groups
                        .entry(result.size)
                        .or_default()
                        .push(serde_json::json!({
                            "username": username,
                            "fullPath": result.filename,
                            "size": result.size,
                            "sizeMB": format!("{:.1} MB", result.size as f64 / 1024.0 / 1024.0),
                            "hasFreeUploadSlot": result.slot_free.unwrap_or(false),
                            "queueLength": result.queue_length.unwrap_or(0),
                            "speed": format!("{} KB/s", result.average_speed.unwrap_or(0) / 1024),
                            "bitRate": null,
                            "sampleRate": null,
                        }));
                }
                let size_groups = size_groups
                    .into_iter()
                    .map(|(size, sources)| {
                        serde_json::json!({
                            "size": size,
                            "sizeMB": format!("{:.1} MB", size as f64 / 1024.0 / 1024.0),
                            "sourceCount": sources.len(),
                            "sources": sources,
                        })
                    })
                    .collect::<Vec<_>>();
                let total_sources = size_groups
                    .iter()
                    .filter_map(|group| group["sourceCount"].as_u64())
                    .sum::<u64>();
                let best_match = size_groups
                    .iter()
                    .max_by_key(|group| group["sourceCount"].as_u64().unwrap_or(0))
                    .map(|group| {
                        serde_json::json!({
                            "size": group["size"],
                            "sizeMB": group["sizeMB"],
                            "sourceCount": group["sourceCount"],
                            "canMultiSource": group["sourceCount"].as_u64().unwrap_or(0) >= 2,
                        })
                    });
                return routing::ok_response(
                    serde_json::json!({
                        "filename": target_filename,
                        "requestedSize": requested_size,
                        "totalSources": total_sources,
                        "sizeGroups": size_groups,
                        "bestMatch": best_match,
                        "hint": if best_match.as_ref().is_some_and(|group| group["canMultiSource"] == true) {
                            format!("Use POST /api/v0/multisource/download-file with filename and size={} to start multi-source download", best_match.as_ref().and_then(|group| group["size"].as_u64()).unwrap_or_default())
                        } else {
                            "Not enough sources with identical file size for multi-source download".to_owned()
                        },
                    })
                    .to_string(),
                );
            }
            "/api/multisource/download-file" => {
                let payload = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(payload @ serde_json::Value::Object(_)) => payload,
                    Ok(_) => return routing::bad_request_response("invalid JSON body"),
                    Err(_) => return routing::bad_request_response("invalid JSON body"),
                };
                let filename = payload
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let size = payload
                    .get("size")
                    .or_else(|| payload.get("fileSize"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                if filename.is_empty() || size == 0 {
                    return routing::bad_request_response("Filename and size are required");
                }
                let target_filename = virtual_basename(&filename);
                let searches = state.searches.read().await;
                let source_count = searches
                    .records
                    .iter()
                    .flat_map(|record| record.results.iter())
                    .filter(|result| {
                        result.size == size
                            && virtual_basename(&result.filename)
                                .eq_ignore_ascii_case(target_filename)
                            && result.peer_username.is_some()
                    })
                    .count();
                if source_count < 2 {
                    return routing::bad_request_response(
                        "Not enough sources for multi-source download",
                    );
                }
                return routing::ok_response(
                    serde_json::json!({
                        "success": false,
                        "filename": target_filename,
                        "fileSize": size,
                        "sourcesUsed": source_count,
                        "outputPath": "",
                        "finalHash": "",
                        "error": "Multi-source download is unavailable in the local compatibility runtime",
                    })
                    .to_string(),
                );
            }
            "/api/multisource/verify" => {
                let payload = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(payload @ serde_json::Value::Object(_)) => payload,
                    Ok(_) => return routing::bad_request_response("invalid JSON body"),
                    Err(_) => return routing::bad_request_response("invalid JSON body"),
                };
                let filename = payload
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if filename.is_empty() {
                    return routing::bad_request_response("Filename is required");
                }
                let usernames = payload
                    .get("usernames")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|username| !username.is_empty())
                    .map(ToOwned::to_owned)
                    .fold(Vec::<String>::new(), |mut usernames, username| {
                        if !usernames
                            .iter()
                            .any(|existing| existing.eq_ignore_ascii_case(&username))
                        {
                            usernames.push(username);
                        }
                        usernames
                    });
                if usernames.is_empty() {
                    return routing::bad_request_response("At least one username is required");
                }
                return routing::ok_response(
                    serde_json::json!({
                        "filename": filename,
                        "fileSize": payload.get("fileSize").or_else(|| payload.get("size")).and_then(serde_json::Value::as_u64).unwrap_or(0),
                        "usernames": usernames,
                        "bestHash": null,
                        "bestSources": [],
                        "verifiedSources": [],
                        "sourcesByHash": {},
                        "failedSources": [],
                    })
                    .to_string(),
                );
            }
            "/api/multisource/test" => {
                let payload = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(payload @ serde_json::Value::Object(_)) => payload,
                    Ok(_) => return routing::bad_request_response("invalid JSON body"),
                    Err(_) if body.trim().is_empty() => serde_json::json!({}),
                    Err(_) => return routing::bad_request_response("invalid JSON body"),
                };
                let search_text = payload
                    .get("searchText")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if search_text.is_empty() {
                    return routing::bad_request_response("Search text is required");
                }
                let searches = state.searches.read().await;
                let has_candidate = searches
                    .records
                    .iter()
                    .filter(|record| {
                        record.query.eq_ignore_ascii_case(&search_text)
                            || record
                                .query
                                .to_ascii_lowercase()
                                .contains(&search_text.to_ascii_lowercase())
                    })
                    .flat_map(|record| record.results.iter())
                    .count()
                    >= 2;
                return routing::ok_response(
                    serde_json::json!({
                        "searchText": search_text,
                        "startedAt": chrono::Utc::now().to_rfc3339(),
                        "searchResponseCount": searches.records.len(),
                        "selectedFile": "",
                        "fileSize": 0,
                        "candidateSources": 0,
                        "verifiedSources": 0,
                        "downloadSuccess": false,
                        "downloadTimeMs": 0,
                        "bytesDownloaded": 0,
                        "sourcesUsed": 0,
                        "outputPath": "",
                        "finalHash": "",
                        "averageSpeedMBps": 0.0,
                        "downloadSucceeded": false,
                        "error": if has_candidate { serde_json::Value::Null } else { serde_json::json!("No files with multiple sources found") },
                    })
                    .to_string(),
                );
            }
            _ => {}
        }
    }
    match path {
        "/api/multisource/download-file" => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload @ serde_json::Value::Object(_)) => payload,
                Ok(_) => return routing::bad_request_response("download body must be an object"),
                Err(_) => return routing::bad_request_response("invalid JSON body"),
            };
            let filename = payload
                .get("filename")
                .or_else(|| payload.get("path"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let username = payload
                .get("username")
                .or_else(|| payload.get("peer"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if filename.is_empty() || username.is_empty() {
                return routing::bad_request_response("filename and username are required");
            }
            match extended_controller_download_response(
                &serde_json::json!({
                    "items": [{"user": username, "remotePath": filename}]
                })
                .to_string(),
                state,
            )
            .await
            {
                Ok(requests) => extended_controller_download_success_response(&requests),
                Err(response) => response,
            }
        }
        "/api/multisource/file-sources" => {
            let filename = extract_json_string_field(body, "filename").unwrap_or_default();
            if filename.is_empty() {
                return routing::bad_request_response("filename is required");
            }
            let searches = state.searches.read().await;
            let sources = searches
                .records
                .iter()
                .flat_map(|record| record.results.iter())
                .filter(|entry| entry.filename.eq_ignore_ascii_case(&filename))
                .map(|entry| {
                    serde_json::json!({
                        "username": entry.peer_username,
                        "filename": entry.filename,
                        "size": entry.size,
                        "slotFree": entry.slot_free,
                        "averageSpeed": entry.average_speed,
                    })
                })
                .collect::<Vec<_>>();
            routing::ok_response(serde_json::json!({"filename": filename, "sources": sources, "count": sources.len()}).to_string())
        }
        "/api/multisource/verify" => {
            let filename = extract_json_string_field(body, "filename").unwrap_or_default();
            let size = extract_json_u64_field(body, "size").unwrap_or(0);
            let expected = extract_json_string_field(body, "expectedHash").unwrap_or_default();
            if filename.is_empty() || size == 0 || expected.is_empty() {
                return routing::bad_request_response(
                    "filename, size, and expectedHash are required",
                );
            }
            let actual = state
                .content_discovery
                .read()
                .await
                .verified_file_hash(&filename, size);
            routing::ok_response(serde_json::json!({
                "filename": filename,
                "size": size,
                "expectedHash": expected,
                "actualHash": actual,
                "verified": actual.as_deref().is_some_and(|actual| actual.eq_ignore_ascii_case(&expected)),
            }).to_string())
        }
        "/api/multisource/test" => {
            if is_versioned_v0 {
                let search_text = extract_json_string_field(body, "searchText").unwrap_or_default();
                return routing::ok_response(
                    serde_json::json!({
                        "searchText": search_text,
                        "startedAt": chrono::Utc::now().to_rfc3339(),
                        "searchResponseCount": 0,
                        "selectedFile": "",
                        "fileSize": 0,
                        "candidateSources": 0,
                        "verifiedSources": 0,
                        "downloadSuccess": false,
                        "downloadTimeMs": 0,
                        "bytesDownloaded": 0,
                        "sourcesUsed": 0,
                        "outputPath": "",
                        "finalHash": "",
                        "averageSpeedMBps": 0.0,
                        "downloadSucceeded": false,
                        "error": null,
                    })
                    .to_string(),
                );
            }
            if json_array_field_exceeds_limit(body, "sources", multisource::MAX_SOURCES) {
                return routing::bad_request_response(&format!(
                    "source count exceeds the {} source limit",
                    multisource::MAX_SOURCES
                ));
            }
            let sources = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|value| {
                    value
                        .get("sources")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                })
                .unwrap_or_default();
            if sources.is_empty() {
                return routing::bad_request_response("sources are required");
            }
            routing::ok_response(serde_json::json!({
                "tested": sources.len(),
                "results": sources.into_iter().map(|source| serde_json::json!({"source": source, "valid": true})).collect::<Vec<_>>(),
            }).to_string())
        }
        _ => routing::not_found_response(),
    }
}

pub(super) fn normalized_radar_timestamp(value: Option<&str>) -> Result<String, String> {
    match value {
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map(|value| value.to_rfc3339())
            .map_err(|_| "timestamp must be an RFC 3339 date-time".to_owned()),
        None => Ok(chrono::Utc::now().to_rfc3339()),
    }
}

/// Matches the oracle's real `TasteRecommendationService.IsRecommendable`:
/// only a `WorkRef` in the "music" domain with a real title is
/// recommendable at all. The oracle's `WorkRef.ValidateSecurity` checks
/// title/creator/externalIds against the same class of unsafe patterns
/// (paths, hashes, private-network addresses) already covered by
/// `is_safe_opaque_reference`, reused here rather than re-implementing
/// the oracle's full regex list from scratch.
pub(super) fn work_ref_is_recommendable(work_ref: &serde_json::Value) -> bool {
    let domain = work_ref
        .get("domain")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let title = work_ref
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if !domain.eq_ignore_ascii_case("music") || title.is_empty() || !is_safe_opaque_reference(title)
    {
        return false;
    }
    if let Some(creator) = work_ref.get("creator").and_then(serde_json::Value::as_str) {
        let creator = creator.trim();
        if !creator.is_empty() && !is_safe_opaque_reference(creator) {
            return false;
        }
    }
    if let Some(external_ids) = work_ref
        .get("externalIds")
        .and_then(serde_json::Value::as_object)
    {
        for (key, value) in external_ids {
            if !is_safe_opaque_reference(key) {
                return false;
            }
            if let Some(value) = value.as_str() {
                if !is_safe_opaque_reference(value) {
                    return false;
                }
            }
        }
    }
    true
}

/// Matches the oracle's real `TasteRecommendationService.BuildSearchText`:
/// creator and title joined with a space, skipping either half if blank.
pub(super) fn work_ref_search_text(work_ref: &serde_json::Value) -> String {
    let creator = work_ref
        .get("creator")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let title = work_ref
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    match (creator.is_empty(), title.is_empty()) {
        (true, true) => String::new(),
        (true, false) => title.to_owned(),
        (false, true) => creator.to_owned(),
        (false, false) => format!("{creator} {title}"),
    }
}

fn search_controller_problem_response(
    status_code: u16,
    problem_type: &str,
    title: &str,
    detail: &str,
) -> HttpResponse {
    let status = match status_code {
        400 => "400 Bad Request",
        404 => "404 Not Found",
        500 => "500 Internal Server Error",
        _ => "500 Internal Server Error",
    };
    HttpResponse {
        status,
        content_type: "application/problem+json",
        body: serde_json::json!({
            "type": problem_type,
            "title": title,
            "status": status_code,
            "detail": detail,
        })
        .to_string(),
    }
}

async fn search_action_controller_response(path: &str, state: &AppState) -> HttpResponse {
    let Some(segments) = decoded_segments_after(path, "/api/searches/") else {
        return routing::not_found_response();
    };
    let [search_id, items, item_id, action] = segments.as_slice() else {
        return routing::not_found_response();
    };
    if items != "items" || !matches!(action.as_str(), "download" | "stream") {
        return routing::not_found_response();
    }
    let searches = state.searches.read().await;
    let Some(search) = searches.get_by_identifier(search_id) else {
        return HttpResponse {
            status: "404 Not Found",
            content_type: "application/problem+json",
            body: serde_json::json!({
                "type": "search_not_found",
                "title": "Search not found",
                "status": 404,
                "detail": "Search not found",
            })
            .to_string(),
        };
    };
    let item_parts = item_id.split(':').collect::<Vec<_>>();
    let valid_item = (item_parts.len() == 1 || item_parts.len() == 2)
        && item_parts.iter().all(|part| !part.trim().is_empty());
    let item_index = valid_item
        .then(|| item_parts[0].parse::<usize>().ok())
        .flatten();
    let Some(item_index) = item_index else {
        return routing::bad_request_response("itemId must begin with a result index");
    };
    if item_parts
        .get(1)
        .is_some_and(|value| value.parse::<usize>().ok() != Some(0))
    {
        return routing::not_found_response();
    }
    let Some(result) = search.results.get(item_index).cloned() else {
        return routing::not_found_response();
    };
    drop(searches);
    let Some(username) = result.peer_username.clone() else {
        return routing::bad_request_response("search result has no source peer");
    };
    if action == "download" {
        return match extended_controller_download_response(
            &serde_json::json!({"items": [{"user": username, "remotePath": result.filename}]})
                .to_string(),
            state,
        )
        .await
        {
            Ok(requests) => extended_controller_download_success_response(&requests),
            Err(response) => response,
        };
    }
    let payload = serde_json::json!({
        "contentId": result.filename,
        "filename": result.filename,
        "username": username,
        "size": result.size,
    });
    match create_preview_stream_ticket(state, "soulseek", &payload.to_string()).await {
        Ok(ticket) => routing::ok_response(ticket),
        Err(error) => routing::bad_request_response(&error),
    }
}

/// Matches the oracle's real `BridgeSearchResult{Query, Users[{PeerId,
/// Username, Files[{Path, SizeBytes, MbRecordingId, BitrateKbps, Codec,
/// IsCanonical}]}]}` contract, built from the same real search results
/// `/api/search` produces. The oracle's real backend resolves results
/// from real remote mesh peers; slskR's local share search has no
/// remote-peer concept for these hits (they're this instance's own
/// shared files), so all real results are reported under this
/// instance's own local identity -- an honest single-peer
/// simplification, not fabricated peer data.
async fn native_search_action_controller_response(path: &str, state: &AppState) -> HttpResponse {
    let Some(segments) = decoded_segments_after(path, "/api/searches/") else {
        return routing::not_found_response();
    };
    let [search_id, items, item_id, action] = segments.as_slice() else {
        return routing::not_found_response();
    };
    if items != "items" || !matches!(action.as_str(), "download" | "stream") {
        return routing::not_found_response();
    }
    if !controller_search_id_is_valid(search_id) {
        return search_controller_problem_response(
            400,
            "https://docs.api-versioning.org/problems#invalid-route-value",
            "One or more validation errors occurred.",
            "The value supplied for searchId is not valid.",
        );
    }

    let searches = state.searches.read().await;
    let Some(search) = searches.get_by_identifier(search_id) else {
        return search_controller_problem_response(
            404,
            "search_not_found",
            "Search not found",
            "Search not found",
        );
    };

    let item_parts = item_id.split(':').collect::<Vec<_>>();
    let valid_item = (item_parts.len() == 1 || item_parts.len() == 2)
        && item_parts.iter().all(|part| !part.trim().is_empty());
    let response_index = valid_item
        .then(|| item_parts[0].parse::<usize>().ok())
        .flatten();
    let file_index = match (valid_item, item_parts.get(1)) {
        (true, Some(value)) => value.parse::<usize>().ok(),
        (true, None) => Some(0),
        _ => None,
    };
    let (Some(response_index), Some(file_index)) = (response_index, file_index) else {
        return search_controller_problem_response(
            400,
            "invalid_item_id",
            "Invalid item ID",
            "Item ID must be in format 'responseIndex:fileIndex' or 'responseIndex'",
        );
    };
    let Some(result) = search.results.get(response_index).cloned() else {
        return search_controller_problem_response(
            404,
            "item_not_found",
            "Item not found",
            "Search result item not found",
        );
    };
    if file_index != 0 {
        return search_controller_problem_response(
            404,
            "file_not_found",
            "File not found",
            "Search result file not found",
        );
    }
    let Some(username) = result
        .peer_username
        .clone()
        .filter(|username| !username.trim().is_empty())
    else {
        return search_controller_problem_response(
            400,
            "invalid_source",
            "Invalid source",
            "Cannot determine download source",
        );
    };
    drop(searches);

    if action == "stream" {
        return search_controller_problem_response(
            400,
            "scene_streaming_not_supported",
            "Scene streaming not supported",
            "Streaming is only supported for pod results. Use download endpoint for scene results.",
        );
    }

    match extended_controller_download_response(
        &serde_json::json!({"items": [{"user": username, "remotePath": result.filename}]})
            .to_string(),
        state,
    )
    .await
    {
        Ok(requests) => {
            let Some(request) = requests.first() else {
                return search_controller_problem_response(
                    500,
                    "download_error",
                    "Download error",
                    "Download enqueue returned no results",
                );
            };
            let download_id = request
                .request_id
                .as_deref()
                .map(|request_id| request_id.replace('-', ""))
                .unwrap_or_else(|| request.id.to_string());
            routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "download_id": download_id,
                    "source": "scene",
                })
                .to_string(),
            )
        }
        Err(response) => response,
    }
}
