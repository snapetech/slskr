async fn route_dispatch_group_2_transfer_status(
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
        ("GET", "/api/transfers/downloads/auto-replace/status")
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native =>
        {
            let stuck_count = state
                .transfers
                .read()
                .await
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && matches!(
                            entry.status.as_str(),
                            "failed" | "rejected" | "errored" | "cancelled"
                        )
                })
                .count();
            let enabled = state.runtime.read().await.autoreplace_enabled;
            Ok(routing::ok_response(
                serde_json::json!({
                    "stuckCount": stuck_count,
                    "enabled": enabled,
                    "intervalSeconds": 300,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/transfers/downloads/stuck") => {
            let transfers = state.transfers.read().await;
            let value = serde_json::from_str::<serde_json::Value>(
                &controller_stuck_downloads_json(route.query, &transfers),
            )
            .unwrap_or_else(|_| serde_json::json!({"stuck": []}));
            drop(transfers);
            Ok(routing::ok_response(value["stuck"].to_string()))
        }

        ("GET", "/api/transfers/uploads/diagnostics") => {
            let transfers = state.transfers.read().await;
            let uploads = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction != 0)
                .collect::<Vec<_>>();
            let listeners = state.listeners.read().await;
            let shares = state.shares.read().await;
            Ok(routing::ok_response(serde_json::json!({
                 "activeUploads": uploads.iter().filter(|entry| is_active_transfer_status(&entry.status)).count(),
                 "failedUploads": uploads.iter().filter(|entry| is_failed_transfer_status(&entry.status)).count(),
                 "succeededUploads": uploads.iter().filter(|entry| is_successful_transfer_status(&entry.status)).count(),
                 "totalUploadRecords": uploads.len(),
                 "generatedAt": unix_timestamp(),
                 "isConnected": state.session.read().await.state == "connected",
                 "isLoggedIn": state.session.read().await.state == "connected",
                 "listenIpAddress": listeners.regular_bind.as_deref().and_then(|address| address.parse::<SocketAddr>().ok()).map(|address| address.ip()),
                 "listenPort": listeners.regular_bind.as_deref().and_then(|address| address.parse::<SocketAddr>().ok()).map(|address| address.port()).unwrap_or(0),
                 "localListenProbe": serde_json::Value::Null,
                 "recentUploads": uploads.iter().take(20).map(|entry| serde_json::from_str::<serde_json::Value>(&entry.json()).unwrap_or_default()).collect::<Vec<_>>(),
                 "shareDirectories": shares.roots.len(),
                 "shareFiles": shares.entries.len(),
                 "shareScanPending": false,
                 "shareScanning": false,
                 "soulseekState": state.session.read().await.state,
                 "uploadSlots": state.config.transfer_max_active,
                 "uploadSpeedLimit": 0,
                 "warnings": [],
             }).to_string()))
        }

        ("GET", "/api/transfers/downloads/user-stats") => {
            let transfers = state.transfers.read().await;
            let body = controller_download_user_stats_json(route.query, &transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/transfers/downloads/stats") => {
            let transfers = state.transfers.read().await;
            let json = controller_download_stats_json(&transfers);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/transfers/downloads/batches/") => {
            let batch_id =
                decoded_path_segment(path.trim_start_matches("/api/transfers/downloads/batches/"));
            if uuid::Uuid::parse_str(&batch_id).is_err() {
                return Ok(routing::bad_request_response("invalid batch id"));
            }
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let batch_record = match controller_read_transfer_batch(state, &batch_id).await {
                Ok(record) => record,
                Err(error) => return Ok(routing::internal_server_error_response(&error)),
            };
            let transfers = state.transfers.read().await;
            let downloads = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0 && entry.batch_id.as_deref() == Some(batch_id.as_str())
                })
                .map(TransferEntry::controller_file_json)
                .collect::<Vec<_>>();
            if downloads.is_empty() && batch_record.is_none() {
                drop(transfers);
                return Ok(routing::not_found_response());
            }
            let completed_count = downloads
                .iter()
                .filter(|entry| {
                    entry
                        .get("state")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|state| state.eq_ignore_ascii_case("Completed"))
                })
                .count();
            let failed_count = downloads
                .iter()
                .filter(|entry| {
                    entry
                        .get("state")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|state| matches!(state, "Failed" | "Errored"))
                })
                .count();
            let transfer_count = downloads.len();
            drop(transfers);
            if let Some(batch) = batch_record {
                return Ok(routing::ok_response(
                    transfer_batch_with_entries(batch, downloads).to_string(),
                ));
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "id": batch_id,
                    "transfers": downloads,
                    "transferCount": transfer_count,
                    "completedCount": completed_count,
                    "failedCount": failed_count,
                })
                .to_string(),
            ))
        }

        ("GET", path) if controller_transfer_user_path(path, "downloads").is_some() => {
            let Some(username) = controller_transfer_user_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = transfers.controller_transfer_user_json(0, username.trim());
            drop(transfers);
            Ok(body
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response))
        }

        ("GET", path) if controller_transfer_user_path(path, "uploads").is_some() => {
            let Some(username) = controller_transfer_user_path(path, "uploads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if username.trim().is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = transfers.controller_transfer_user_json(1, username.trim());
            drop(transfers);
            Ok(body
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response))
        }

        ("GET", path)
            if controller_transfer_file_path(path, "downloads").is_some()
                && !path.ends_with("/position") =>
        {
            let Some((username, id)) = controller_transfer_file_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let username = decoded_path_segment(username);
            let transfers = state.transfers.read().await;
            let response = transfers
                .controller_transfer_json(0, &username, id)
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response);
            drop(transfers);
            Ok(response)
        }

        ("GET", path)
            if controller_transfer_file_path(path, "uploads").is_some()
                && !path.ends_with("/position") =>
        {
            let Some((username, id)) = controller_transfer_file_path(path, "uploads") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let username = decoded_path_segment(username);
            let transfers = state.transfers.read().await;
            let response = transfers
                .controller_transfer_json(1, &username, id)
                .map(routing::ok_response)
                .unwrap_or_else(routing::not_found_response);
            drop(transfers);
            Ok(response)
        }

        ("GET", path) if controller_transfer_position_path(path).is_some() => {
            let Some((username, id)) = controller_transfer_position_path(path) else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let username = decoded_path_segment(username);
            let transfers = state.transfers.read().await;
            let filename = transfers.entries.iter().find_map(|entry| {
                (entry.direction == 0
                    && entry.id == id
                    && entry.peer_username.as_deref() == Some(username.as_str()))
                .then(|| entry.filename.clone())
            });
            drop(transfers);
            let Some(filename) = filename else {
                return Ok(routing::not_found_response());
            };
            if state.session.read().await.state != "connected" {
                return Ok(routing::no_content_response());
            }
            let address = if let Some(address) = cached_peer_endpoint(state, &username).await {
                address
            } else if state.regular_listener_commands.is_none()
                && state.obfuscated_listener_commands.is_none()
            {
                // A test/in-process state without listener workers cannot
                // service the asynchronous session endpoint lookup. Match
                // The legacy empty queue-position contract immediately rather
                // than waiting for the network timeout; a live daemon still
                // takes the discovery path below.
                return Ok(routing::no_content_response());
            } else {
                match request_peer_endpoint(state, &username).await {
                    Ok(address) => address,
                    Err(_) => return Ok(routing::no_content_response()),
                }
            };
            let response = match send_peer_message_request(
                state,
                &address,
                PeerMessage::PlaceInQueueRequest {
                    filename: filename.clone(),
                },
            )
            .await
            {
                Ok(response) => response,
                Err(_) => return Ok(routing::no_content_response()),
            };
            match response {
                PeerMessage::PlaceInQueueResponse {
                    filename: response_filename,
                    place,
                } if response_filename == filename => Ok(routing::ok_response(place.to_string())),
                _ => Ok(routing::no_content_response()),
            }
        }

        ("POST", "/api/transfers/downloads/find-alternative") => {
            if route.path.starts_with("/api/v0/")
                && extract_json_u64_field(body, "transfer_id").is_none()
            {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            let transfer_id = extract_json_u64_field(body, "transfer_id").unwrap_or(0);
            if transfer_id == 0 {
                return Ok(routing::bad_request_response("transfer_id is required"));
            }
            let transfers = state.transfers.read().await;
            let Some(transfer) = transfers
                .entries
                .iter()
                .find(|entry| entry.id == transfer_id && entry.direction == 0)
                .cloned()
            else {
                drop(transfers);
                return Ok(routing::not_found_response());
            };
            drop(transfers);
            let searches = state.searches.read().await;
            let json = searches.transfer_alternatives_json(&transfer);
            drop(searches);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/transfers/downloads/replace") => {
            if route.path.starts_with("/api/v0/") {
                return Ok(HttpResponse {
                    status: "500 Internal Server Error",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "success": false,
                        "error": "Failed to replace download",
                    })
                    .to_string(),
                });
            }
            let transfer_id = extract_json_u64_field(body, "transfer_id").unwrap_or(0);
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if transfer_id == 0 || username.is_empty() {
                return Ok(routing::bad_request_response(
                    "transfer_id and username are required",
                ));
            }
            let requested_filename = extract_json_string_field(body, "filename");
            let transfers = state.transfers.read().await;
            let Some(original) = transfers
                .entries
                .iter()
                .find(|entry| entry.id == transfer_id && entry.direction == 0)
                .cloned()
            else {
                drop(transfers);
                return Ok(routing::not_found_response());
            };
            drop(transfers);
            let searches = state.searches.read().await;
            let Some(alternative) = searches.find_transfer_alternative(
                &original,
                &username,
                requested_filename.as_deref(),
            ) else {
                drop(searches);
                return Ok(routing::conflict_response("no matching alternative found"));
            };
            drop(searches);
            let Some(replacement_username) = alternative.peer_username.clone() else {
                return Ok(routing::conflict_response(
                    "matching alternative has no peer",
                ));
            };
            if let Some(exclusion) = crate::download_filter::matching_exclusion(
                &alternative.filename,
                &effective_download_exclusions(state).await,
            ) {
                return Ok(routing::conflict_response(&format!(
                    "replacement blocked by download exclusion: {exclusion}"
                )));
            }
            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };
            let mut transfers = state.transfers.write().await;
            let previous = transfers.mutation_snapshot();
            let updated_original = transfers.update_status(
                original.id,
                "cancelled",
                Some(original.bytes_transferred),
                Some("replaced by alternative source".to_owned()),
            );
            let replacement = transfers.create_with_details(
                0,
                alternative.peer_username.clone(),
                alternative.filename.clone(),
                original.local_path.clone(),
                Some(alternative.size),
                original.batch_id.clone(),
                TransferRequestDetails {
                    request_id: original.request_id.clone(),
                    wishlist_item_id: original.wishlist_item_id.clone(),
                    request_name: original.request_name.clone(),
                    destination_directory: original.destination_directory.clone(),
                    bit_rate: original.bit_rate,
                    sample_rate: original.sample_rate,
                    bit_depth: original.bit_depth,
                    length_seconds: original.length_seconds,
                    artist: original.artist.clone(),
                    album: original.album.clone(),
                    title: original.title.clone(),
                    track_number: original.track_number,
                    year: original.year,
                    attempts: 1,
                    auto_replace_attempts: original.auto_replace_attempts.saturating_add(1),
                    next_attempt_at: None,
                },
            );
            let replacement = transfers
                .update_status(replacement.id, "peer_lookup", None, None)
                .unwrap_or(replacement);
            let replacement_json = replacement.json();
            let mutated = transfers.mutation_snapshot();
            drop(transfers);
            let mut persisted = updated_original.into_iter().collect::<Vec<_>>();
            persisted.push(replacement.clone());
            if let Err(error) = persist_transfer_records(state, &persisted).await {
                rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            session_command_permit.send(SessionCommand::TransferPeer {
                id: replacement.id,
                username: replacement_username,
            });
            Ok(routing::accepted_response(
                serde_json::json!({
                    "transfer_id": transfer_id,
                    "replacement_queued": true,
                    "replacement": serde_json::from_str::<serde_json::Value>(&replacement_json)
                        .map_err(|error| format!("replacement json failed: {error}"))?,
                    "status": "queued",
                })
                .to_string(),
            ))
        }

        ("POST", "/api/transfers/downloads/auto-replace") => {
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(
                    r#"{"replaced":0,"failed":0,"skipped":0,"details":[]}"#.to_owned(),
                ));
            }
            let requested_transfer_id = extract_json_u64_field(body, "transfer_id");
            let transfers = state.transfers.read().await;
            let candidates = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && is_failed_transfer_status(&entry.status)
                        && requested_transfer_id.is_none_or(|id| entry.id == id)
                })
                .cloned()
                .collect::<Vec<_>>();
            drop(transfers);

            let exclusions = effective_download_exclusions(state).await;
            let searches = state.searches.read().await;
            let replacements = candidates
                .iter()
                .filter_map(|transfer| {
                    searches
                        .first_transfer_alternative(transfer)
                        .filter(|alternative| {
                            !crate::download_filter::is_excluded(&alternative.filename, &exclusions)
                        })
                        .map(|alternative| (transfer.clone(), alternative))
                })
                .collect::<Vec<_>>();
            drop(searches);

            if replacements.is_empty() {
                return Ok(routing::accepted_response(
                    serde_json::json!({
                        "replacement_queued": false,
                        "alternatives": [],
                        "replacements": [],
                        "status": "idle",
                    })
                    .to_string(),
                ));
            }

            let mut session_command_permits = Vec::with_capacity(replacements.len());
            for _ in 0..replacements.len() {
                match state.session_commands.reserve().await {
                    Ok(permit) => session_command_permits.push(permit),
                    Err(_) => {
                        return Ok(routing::service_unavailable_response(
                            "session manager is not running",
                        ));
                    }
                }
            }

            let mut queued = Vec::new();
            let mut commands = Vec::new();
            let mut persisted = Vec::new();
            let mut transfers = state.transfers.write().await;
            let previous = transfers.mutation_snapshot();
            for (original, alternative) in replacements {
                if let Some(entry) = transfers.update_status(
                    original.id,
                    "cancelled",
                    Some(original.bytes_transferred),
                    Some("auto-replaced by alternative source".to_owned()),
                ) {
                    persisted.push(entry);
                }
                let replacement = transfers.create_with_details(
                    0,
                    alternative.peer_username.clone(),
                    alternative.filename.clone(),
                    original.local_path.clone(),
                    Some(alternative.size),
                    original.batch_id.clone(),
                    TransferRequestDetails {
                        request_id: original.request_id.clone(),
                        wishlist_item_id: original.wishlist_item_id.clone(),
                        request_name: original.request_name.clone(),
                        destination_directory: original.destination_directory.clone(),
                        bit_rate: original.bit_rate,
                        sample_rate: original.sample_rate,
                        bit_depth: original.bit_depth,
                        length_seconds: original.length_seconds,
                        artist: original.artist.clone(),
                        album: original.album.clone(),
                        title: original.title.clone(),
                        track_number: original.track_number,
                        year: original.year,
                        attempts: 1,
                        auto_replace_attempts: original.auto_replace_attempts.saturating_add(1),
                        next_attempt_at: None,
                    },
                );
                let replacement = transfers
                    .update_status(replacement.id, "peer_lookup", None, None)
                    .unwrap_or(replacement);
                persisted.push(replacement.clone());
                if let Some(username) = replacement.peer_username.clone() {
                    commands.push(SessionCommand::TransferPeer {
                        id: replacement.id,
                        username,
                    });
                }
                queued.push(serde_json::json!({
                    "transfer_id": original.id,
                    "replacement": serde_json::from_str::<serde_json::Value>(&replacement.json())
                        .map_err(|error| format!("replacement json failed: {error}"))?,
                    "alternative": {
                        "username": alternative.peer_username.as_deref().unwrap_or_default(),
                        "filename": alternative.filename,
                        "size": alternative.size,
                    },
                }));
            }
            let mutated = transfers.mutation_snapshot();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &persisted).await {
                rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }

            for (permit, command) in session_command_permits.into_iter().zip(commands) {
                permit.send(command);
            }

            Ok(routing::accepted_response(
                serde_json::json!({
                    "replacement_queued": true,
                    "alternatives": queued.iter().map(|entry| entry["alternative"].clone()).collect::<Vec<_>>(),
                    "replacements": queued,
                    "status": "queued",
                })
                .to_string(),
            ))
        }

        ("POST", path) if controller_transfer_user_path(path, "downloads").is_some() => {
            let Some(username) = controller_transfer_user_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            if controller_file_array_exceeds_wire_limits(body) {
                return Ok(routing::bad_request_response(
                    "download request exceeds file limits",
                ));
            }
            let mut files = controller_files_from_body(body);
            if route.path.starts_with("/api/v0/") && files.is_empty() {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!("At least one file is required").to_string(),
                });
            }
            let filenames = files
                .iter()
                .filter_map(|file| file.get("filename").and_then(serde_json::Value::as_str))
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let exclusions = effective_download_exclusions(state).await;
            let blocked = filenames
                .iter()
                .filter_map(|filename| {
                    crate::download_filter::matching_exclusion(filename, &exclusions).map(
                        |exclusion| {
                            serde_json::json!({
                                "filename": filename,
                                "exclusion": exclusion,
                            })
                        },
                    )
                })
                .collect::<Vec<_>>();
            if blocked.len() == filenames.len() && !blocked.is_empty() {
                return Ok(download_policy_response(state, &filenames)
                    .await
                    .expect("non-empty blocked list must produce a policy response"));
            }
            files.retain(|file| {
                file.get("filename")
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|filename| {
                        !crate::download_filter::is_excluded(filename, &exclusions)
                    })
            });
            let _request_permit = if route.path.starts_with("/api/v0/") {
                match Arc::clone(&state.download_requests).try_acquire_owned() {
                    Ok(permit) => Some(permit),
                    Err(_) => {
                        return Ok(HttpResponse {
                             status: "429 Too Many Requests",
                             content_type: "application/json",
                             body: serde_json::json!(
                                 "Only one concurrent operation is permitted. Wait until the previous request completes"
                             )
                             .to_string(),
                         });
                    }
                }
            } else {
                None
            };
            let batch_id = controller_transfer_batch_id(body)
                .or_else(|| (files.len() > 1).then(|| uuid::Uuid::new_v4().to_string()));
            let mut prepared = Vec::with_capacity(files.len());
            for file in files {
                let filename = file
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                if filename.is_empty() {
                    continue;
                }
                let size = file.get("size").and_then(serde_json::Value::as_u64);
                let mut details = transfer_request_details_from_json(&file, &filename);
                if details.destination_directory.is_none() {
                    details.destination_directory = query_parameter(route.query, "destination")
                        .map(|value| truncate_utf8_bytes(value, MAX_TRANSFER_LOCAL_PATH_BYTES));
                }
                let relative = if let Some(destination) = details
                    .destination_directory
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    format!(
                        "{}/{}",
                        destination.trim_matches(['/', '\\']),
                        virtual_basename(&filename)
                    )
                } else {
                    match render_configured_completed_download_path(
                        state,
                        &username,
                        &filename,
                        batch_id.as_deref(),
                        details.request_name.as_deref(),
                        unix_timestamp(),
                    )
                    .await
                    {
                        Ok(relative) => relative,
                        Err(error) => return Ok(routing::bad_request_response(&error)),
                    }
                };
                let local_path = match configured_download_destination_path(state, &relative).await
                {
                    Ok(path) => Some(path.display().to_string()),
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                prepared.push((filename, local_path, size, details));
            }
            let mut transfers = state.transfers.write().await;
            let previous = transfers.mutation_snapshot();
            let mut created = Vec::new();
            let mut created_entries = Vec::new();
            for (filename, local_path, size, details) in prepared {
                let entry = transfers.create_with_details(
                    0,
                    Some(username.clone()),
                    filename,
                    local_path,
                    size,
                    batch_id.clone(),
                    details,
                );
                created.push(entry.controller_file_json());
                created_entries.push(entry);
            }
            let count = created.len();
            let mutated = transfers.mutation_snapshot();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &created_entries).await {
                rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "queued": count,
                    "transfers": created,
                    "blocked": blocked,
                })
                .to_string(),
            ))
        }

        ("DELETE", "/api/transfers/downloads/all/completed")
        | ("DELETE", "/api/transfers/uploads/all/completed") => {
            let direction = if normalized_path.contains("/downloads/") {
                0
            } else {
                1
            };
            let mut transfers = state.transfers.write().await;
            let previous = transfers.mutation_snapshot();
            let before = transfers.entries.len();
            let mut removed_entries = Vec::new();
            transfers.entries.retain(|entry| {
                let remove = entry.direction == direction
                    && matches!(
                        entry.status.as_str(),
                        "succeeded" | "completed" | "cancelled" | "failed" | "rejected" | "errored"
                    );
                if remove {
                    removed_entries.push(entry.clone());
                }
                !remove
            });
            let removed = before.saturating_sub(transfers.entries.len());
            for entry in &removed_entries {
                transfers.progress_persisted_at.remove(&entry.id);
            }
            transfers.persist_state();
            let mutated = transfers.mutation_snapshot();
            drop(transfers);
            if let Err(error) = delete_persisted_transfers(state, &removed_entries).await {
                rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            if route.path.starts_with("/api/v0/") {
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response((removed > 0).to_string()))
            }
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
