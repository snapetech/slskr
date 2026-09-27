async fn route_dispatch_group_5_conversations_jobs(
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
        ("GET", "/api/conversations") => {
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.controller_conversations_json(route.query);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/conversations/activity/unacknowledged") => {
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let messages = state.messages.read().await;
            let body = messages.has_unacknowledged_messages().to_string();
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if conversation_messages_path(path).is_some() => {
            let Some(username) = conversation_messages_path(path) else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let unacknowledged_only = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "unAcknowledgedOnly")
                .map(|(_, value)| value)
            {
                None => false,
                Some(value) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "unAcknowledgedOnly must be a boolean",
                        ))
                    }
                },
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_messages_json(&username, unacknowledged_only);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_conversation_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let include_messages = match query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includeMessages")
            {
                None => true,
                Some((_, value)) => match parse_bool_value(&value) {
                    Some(value) => value,
                    None => {
                        return Ok(routing::bad_request_response(
                            "includeMessages must be a boolean",
                        ))
                    }
                },
            };
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let messages = state.messages.read().await;
            if state.config.controller_profile == ControllerProfile::Legacy
                && !messages
                    .records
                    .iter()
                    .any(|record| record.username == username)
            {
                drop(messages);
                return Ok(routing::not_found_response());
            }
            let body = messages.controller_conversation_json(&username, include_messages, since);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("POST", "/api/conversations/batch") => {
            if conversation_batch_exceeds_wire_limits(body) {
                return Ok(routing::bad_request_response(
                    "conversation batch exceeds recipient limits",
                ));
            }
            let usernames = match extract_json_string_array_field(body, "usernames")
                .or_else(|| extract_json_string_array_field(body, "recipients"))
            {
                Some(usernames) => usernames,
                None => {
                    return Ok(routing::bad_request_response(
                        "usernames/recipients array is required",
                    ))
                }
            };
            let message_body = match extract_json_string_field(body, "body")
                .or_else(|| extract_json_string_field(body, "message"))
            {
                Some(body) => body,
                None => return Ok(routing::bad_request_response("body/message is required")),
            };
            let command = match private_message_users_command(usernames, message_body.clone()) {
                Ok(ServerMessage::MessageUsers { usernames, .. }) => usernames,
                Ok(_) => return Ok(routing::bad_request_response("invalid message command")),
                Err(error) => return Ok(routing::bad_request_response(&error.to_string())),
            };

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let records: Vec<_> = command
                .iter()
                .map(|username| messages.add(username.clone(), "outbound", message_body.clone()))
                .collect();
            let mutated = messages.clone();
            drop(messages);

            if let Err(error) = persist_message_records_checked(state, &records).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_message_persistence);
            session_command_permit.send(SessionCommand::MessageUsers {
                usernames: command.clone(),
                body: message_body,
            });

            if state.config.controller_profile == ControllerProfile::Native
                && route.path.starts_with("/api/v0/")
            {
                Ok(HttpResponse {
                    status: "201 Created",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::created_response(
                    serde_json::json!({
                        "conversations": records.iter().map(MessageRecord::json).collect::<Vec<_>>(),
                        "usernames": command,
                        "count": records.len(),
                    })
                    .to_string(),
                ))
            }
        }
        ("POST", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if message_body.trim().is_empty() {
                return Ok(routing::bad_request_response("message is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let record = messages.add(username.clone(), "outbound", message_body.clone());
            let mutated = messages.clone();
            drop(messages);
            if let Err(error) = persist_message_record_checked(state, &record).await {
                rollback_messages_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(_message_persistence);
            session_command_permit.send(SessionCommand::MessageUser {
                username,
                body: message_body,
            });
            Ok(
                if matches!(
                    state.config.controller_profile,
                    ControllerProfile::Legacy | ControllerProfile::Native
                ) && route.path.starts_with("/api/v0/")
                {
                    HttpResponse {
                        status: "201 Created",
                        content_type: "",
                        body: String::new(),
                    }
                } else {
                    routing::ok_response((record.id > 0).to_string())
                },
            )
        }

        // JOBS ENDPOINT
        ("GET", "/api/jobs") => {
            let searches = state.searches.read().await;
            let transfers = state.transfers.read().await;
            let mut jobs = searches
                .records
                .iter()
                .map(|record| {
                    serde_json::json!({
                        "id": record.id,
                        "kind": "search",
                        "type": "search",
                        "status": record.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": if record.status == "completed" { 1 } else { 0 },
                            "releases_failed": 0,
                        },
                        "progress_percent": if record.status == "completed" { 100 } else { 50 },
                        "query": record.query,
                        "created_at": record.created_at,
                        "updated_at": record.updated_at,
                    })
                })
                .collect::<Vec<_>>();
            jobs.extend(transfers.entries.iter().map(|entry| {
                let size = entry.size.unwrap_or(0);
                let progress = entry
                    .bytes_transferred
                    .saturating_mul(100)
                    .checked_div(size)
                    .unwrap_or(0)
                    .min(100);
                serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "type": "transfer",
                        "status": entry.status,
                        "progress": {
                            "releases_total": 0,
                            "releases_done": 0,
                            "releases_failed": if is_failed_transfer_status(&entry.status) { 1 } else { 0 },
                        },
                        "progress_percent": progress,
                        "filename": entry.filename,
                        "created_at": entry.requested_at,
                        "updated_at": entry.updated_at,
                })
            }));
            let total = jobs.len();
            drop(transfers);
            drop(searches);
            Ok(routing::ok_response(
                serde_json::json!({
                    "jobs": jobs,
                    "limit": 100,
                    "offset": 0,
                    "total": total,
                    "has_more": total > 100,
                })
                .to_string(),
            ))
        }
        ("GET", path) if path.starts_with("/api/jobs/discography/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/discography/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            if let Some(job) = state
                .controller_features
                .read()
                .await
                .get(&format!("job/discography/{job_id}"))
                .cloned()
            {
                return Ok(routing::ok_response(job.to_string()));
            }
            let searches = state.searches.read().await;
            let Some(record) = searches.get_by_identifier(&job_id) else {
                return Ok(routing::not_found_response());
            };
            let artist = record
                .query
                .strip_suffix(" discography")
                .unwrap_or(&record.query)
                .trim();
            return Ok(routing::ok_response(
                serde_json::json!({
                    "jobId": record.id,
                    "artistId": artist,
                    "artistName": artist,
                    "profile": "CoreDiscography",
                    "targetDirectory": "",
                    "releaseJobIds": [],
                    "releaseIds": [],
                    "totalReleases": 0,
                    "completedReleases": 0,
                    "failedReleases": 0,
                    "status": "Pending",
                    "createdAt": record.created_at.to_string(),
                })
                .to_string(),
            ));
        }
        ("GET", path) if path.starts_with("/api/jobs/label-crate/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/label-crate/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            let features = state.controller_features.read().await;
            let Some(job) = features.get(&format!("job/label-crate/{job_id}")).cloned() else {
                return Ok(routing::not_found_response());
            };
            Ok(routing::ok_response(job.to_string()))
        }
        ("GET", path) if path.starts_with("/api/jobs/") => {
            let Some(job_id) = path_segment_after(path, "/api/jobs/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(&job_id) {
                let progress = if record.status == "completed" {
                    100
                } else {
                    50
                };
                let body = serde_json::json!({
                    "id": record.id,
                    "kind": "search",
                    "status": record.status,
                    "progress": progress,
                    "query": record.query,
                    "target": record.target,
                    "result_count": record.results.len(),
                    "created_at": record.created_at,
                    "updated_at": record.updated_at,
                })
                .to_string();
                drop(searches);
                return Ok(routing::ok_response(body));
            }
            drop(searches);

            if let Some(job) = state
                .controller_features
                .read()
                .await
                .entries_with_prefix("job/")
                .into_iter()
                .find_map(|(_, job)| {
                    (job.get("jobId").and_then(serde_json::Value::as_str) == Some(job_id.as_str()))
                        .then_some(job)
                })
            {
                return Ok(routing::ok_response(job.to_string()));
            }

            let transfer_id = job_id.strip_prefix("transfer-").unwrap_or(&job_id);
            let transfers = state.transfers.read().await;
            let body = transfer_id
                .parse::<u64>()
                .ok()
                .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id))
                .map(|entry| {
                    let size = entry.size.unwrap_or(0);
                    let progress = entry
                        .bytes_transferred
                        .saturating_mul(100)
                        .checked_div(size)
                        .unwrap_or(0)
                        .min(100);
                    serde_json::json!({
                        "id": format!("transfer-{}", entry.id),
                        "kind": "transfer",
                        "status": entry.status,
                        "progress": progress,
                        "filename": entry.filename,
                        "bytesTransferred": entry.bytes_transferred,
                        "size": size,
                        "created_at": entry.requested_at,
                        "updated_at": entry.updated_at,
                    })
                })
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "id": job_id,
                        "status": "not_found",
                        "progress": 0,
                    })
                })
                .to_string();
            drop(transfers);
            if let Some(job) = state.library.read().await.remediation_job(&job_id) {
                return Ok(routing::ok_response(job.json().to_string()));
            }
            if state.config.controller_profile == ControllerProfile::Native
                && body.contains("\"status\":\"not_found\"")
            {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(body))
        }

        // LIBRARY HEALTH ENDPOINTS
        ("GET", "/api/library/health/summary") => {
            let library_path = query_parameter(route.query, "libraryPath").unwrap_or_default();
            if library_path.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "libraryPath query parameter is required",
                ));
            }
            let library = state.library.read().await;
            let json = library.health_summary_json(library_path);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/dashboard") => {
            let library_path = query_parameter(route.query, "libraryPath").unwrap_or_default();
            if library_path.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "libraryPath query parameter is required",
                ));
            }
            let artist_limit = match query_bounded_usize(route.query, "artistLimit", 1, 100) {
                Ok(value) => value.unwrap_or(10),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "artistLimit must be between 1 and 100",
                    ));
                }
            };
            let issue_limit = match query_bounded_usize(route.query, "issueLimit", 1, 250) {
                Ok(value) => value.unwrap_or(100),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "issueLimit must be between 1 and 250",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_dashboard_json(library_path, artist_limit, issue_limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues") => {
            let filter = match LibraryHealthIssueQuery::from_query(route.query) {
                Ok(filter) => filter,
                Err(error) => {
                    return Ok(routing::bad_request_response(&error));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_json(&filter);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-artist") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                Ok(value) => value.unwrap_or(20),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 100",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_artist_json(limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-release") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                Ok(value) => value.unwrap_or(20),
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 100",
                    ));
                }
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_release_json(limit);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-codec") => {
            let library = state.library.read().await;
            let json = library.health_issues_by_codec_json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/issues/by-type") => {
            let issue_type = if path == "/api/library/health/issues/by-type" {
                None
            } else if let Some(issue_type) =
                path_segment_after(path, "/api/library/health/issues/by-type/")
            {
                Some(issue_type)
            } else {
                return Ok(routing::not_found_response());
            };
            let library = state.library.read().await;
            let json = library.health_issues_by_type_json(issue_type);
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/scans/") => {
            let Some(scan_id) = path_segment_after(path, "/api/library/health/scans/") else {
                return Ok(routing::not_found_response());
            };
            let mut library = state.library.write().await;
            library.refresh_health_scans();
            let scan = library.health_scan(scan_id);
            drop(library);
            Ok(scan
                .map(|record| routing::ok_response(record.json()))
                .unwrap_or_else(routing::not_found_response))
        }
        ("POST", "/api/library/health/scans") => {
            if route.path.starts_with("/api/v0/")
                && (body.trim().is_empty()
                    || !serde_json::from_str::<serde_json::Value>(body)
                        .is_ok_and(|value| value.is_object()))
            {
                return Ok(routing::bad_request_response(
                    "library scan body must be an object",
                ));
            }
            let library_path = extract_json_string_field(body, "libraryPath")
                .or_else(|| extract_json_string_field(body, "path"))
                .unwrap_or_default();
            let mut library = state.library.write().await;
            let scan = match library.start_health_scan(library_path) {
                Ok(scan) => scan,
                Err(active_id) => {
                    return Ok(routing::conflict_response(&format!(
                        "scan already running: {active_id}"
                    )));
                }
            };
            drop(library);
            Ok(routing::ok_response(
                serde_json::json!({
                    "scanId": scan.id,
                    "message": "Scan started successfully",
                })
                .to_string(),
            ))
        }
        ("POST", "/api/library/health/issues/fix") => {
            if route.path.starts_with("/api/v0/")
                && !body.trim().is_empty()
                && !serde_json::from_str::<serde_json::Value>(body)
                    .is_ok_and(|value| value.is_object())
            {
                return Ok(routing::bad_request_response(
                    "library remediation body must be an object",
                ));
            }
            let _library_persistence = state.library_persistence_lock.lock().await;
            let mut library = state.library.write().await;
            let previous = library.clone();
            let fixable = library
                .health_issues()
                .into_iter()
                .filter(|issue| {
                    issue.get("type").and_then(serde_json::Value::as_str) == Some("missing_kind")
                })
                .count();
            let fixed = library.fix_health_issues();
            let records = library.records.clone();
            let remaining = library.health_issues().len();
            let updated_at = library.updated_at;
            let mutated = library.clone();
            drop(library);
            if !fixed.is_empty() {
                if let Err(error) = persist_library_items_checked(state, &records).await {
                    rollback_library_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "fixed": fixed.len(),
                    "fixable": fixable,
                    "issues": fixed,
                    "remaining": remaining,
                    "persisted": true,
                    "updated_at": updated_at,
                })
                .to_string(),
            ))
        }

        // CONFIGURATION ENDPOINTS
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
