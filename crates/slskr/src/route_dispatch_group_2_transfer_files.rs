async fn route_dispatch_group_2_transfer_files(
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
        ("DELETE", path) if controller_transfer_file_path(path, "downloads").is_some() => {
            let Some((username, id)) = controller_transfer_file_path(path, "downloads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let remove_file = query_parameter(route.query, "remove")
                .is_some_and(|value| value == "true")
                || query_parameter(route.query, "deleteFile").is_some_and(|value| value == "true");
            let mut transfers = state.transfers.write().await;
            let target = transfers
                .entries
                .iter()
                .find(|entry| {
                    entry.id == id
                        && entry.direction == 0
                        && entry.peer_username.as_deref() == Some(username.as_str())
                })
                .cloned();
            let Some(target) = target else {
                return Ok(routing::not_found_response());
            };
            let previous = transfers.mutation_snapshot();
            let updated = transfers.update_status(id, "cancelled", None, None);
            let mutated = transfers.mutation_snapshot();
            drop(transfers);
            if let Some(entry) = updated.as_ref() {
                if let Err(error) = persist_transfer_record(state, entry).await {
                    rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            if remove_file {
                if let Some(path) = target.local_path.as_deref() {
                    if let Err(error) = remove_transfer_file_if_present(path).await {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            }
            Ok(routing::no_content_response())
        }

        ("DELETE", path) if controller_transfer_file_path(path, "uploads").is_some() => {
            let Some((username, id)) = controller_transfer_file_path(path, "uploads") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let remove_file = query_parameter(route.query, "remove")
                .is_some_and(|value| value == "true")
                || query_parameter(route.query, "deleteFile").is_some_and(|value| value == "true");
            let mut transfers = state.transfers.write().await;
            let target = transfers
                .entries
                .iter()
                .find(|entry| {
                    entry.id == id
                        && entry.direction == 1
                        && entry.peer_username.as_deref() == Some(username.as_str())
                })
                .cloned();
            let Some(target) = target else {
                return Ok(routing::not_found_response());
            };
            let previous = transfers.mutation_snapshot();
            let updated = transfers.update_status(id, "cancelled", None, None);
            let mutated = transfers.mutation_snapshot();
            drop(transfers);
            if let Some(entry) = updated.as_ref() {
                if let Err(error) = persist_transfer_record(state, entry).await {
                    rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            if remove_file {
                if let Some(path) = target.local_path.as_deref() {
                    if let Err(error) = remove_transfer_file_if_present(path).await {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            }
            Ok(routing::no_content_response())
        }

        // GET individual transfer
        ("GET", path)
            if (path.starts_with("/api/transfers/") || path.starts_with("/api/v0/transfers/"))
                && !path.ends_with("/start")
                && !path.ends_with("/progress")
                && !path.ends_with("/complete")
                && !path.ends_with("/speeds")
                && !path.ends_with("/stats") =>
        {
            let Some(id_str) = transfer_resource_segment(path) else {
                return Ok(routing::not_found_response());
            };
            if let Ok(id) = id_str.parse::<u64>() {
                let transfers = state.transfers.read().await;
                if let Some(entry) = transfers.entries.iter().find(|t| t.id == id) {
                    let json_response = entry.json();
                    drop(transfers);
                    Ok(routing::ok_response(json_response))
                } else {
                    drop(transfers);
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::bad_request_response("invalid transfer id"))
            }
        }

        // DELETE individual transfer (cancel)
        ("DELETE", path)
            if (path.starts_with("/api/transfers/") || path.starts_with("/api/v0/transfers/"))
                && transfer_action_path(normalized_path).is_none() =>
        {
            let Some(id_str) = transfer_resource_segment(path) else {
                return Ok(routing::not_found_response());
            };
            if let Ok(id) = id_str.parse::<u64>() {
                let mut transfers = state.transfers.write().await;
                let previous = transfers.mutation_snapshot();
                if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                    entry.previous_status = Some(entry.status.clone());
                    entry.status = "cancelled".to_owned();
                    entry.updated_at = unix_timestamp();
                    entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
                    let json_response = entry.json();
                    let mutated = entry.clone();
                    let mutation = transfers.mutation_snapshot();
                    drop(transfers);
                    if let Err(error) = persist_transfer_record(state, &mutated).await {
                        rollback_transfer_mutation_if_unchanged(state, previous, mutation).await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::ok_response(json_response))
                } else {
                    drop(transfers);
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::bad_request_response("invalid transfer id"))
            }
        }

        ("POST", _path) if transfer_action_path(normalized_path).is_some() => {
            if let Some((id, action)) = transfer_action_path(normalized_path) {
                let session_command_permit = if action == "start" || action == "retry" {
                    let transfers = state.transfers.read().await;
                    let active_count = transfers
                        .entries
                        .iter()
                        .filter(|transfer| is_active_transfer_status(&transfer.status))
                        .count();
                    if active_count >= state.config.transfer_max_active {
                        return Ok(routing::conflict_response("transfer limit reached"));
                    }
                    let Some(entry) = transfers.entries.iter().find(|entry| entry.id == id) else {
                        return Ok(routing::not_found_response());
                    };
                    if action == "retry"
                        && (entry.direction != 0
                            || !matches!(
                                entry.status.as_str(),
                                "failed" | "rejected" | "errored" | "cancelled"
                            ))
                    {
                        return Ok(routing::conflict_response("transfer is not retryable"));
                    }
                    let peer_username = entry.peer_username.clone();
                    drop(transfers);

                    if let Some(username) = peer_username {
                        if !state.config.transfer_allow_outbound {
                            return Ok(routing::conflict_response(
                                "outbound transfers are disabled",
                            ));
                        }
                        if test_user_endpoint_peer_address(state, &username).is_none() {
                            match state.session_commands.reserve().await {
                                Ok(permit) => Some(permit),
                                Err(_) => {
                                    return Ok(routing::service_unavailable_response(
                                        "session manager is not running",
                                    ));
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                let mut transfers = state.transfers.write().await;

                if action == "start" || action == "retry" {
                    // Check max active transfer limit
                    let max_active = state.config.transfer_max_active;
                    let active_count = transfers
                        .entries
                        .iter()
                        .filter(|t| is_active_transfer_status(&t.status))
                        .count();

                    if active_count >= max_active {
                        drop(transfers);
                        return Ok(routing::conflict_response("transfer limit reached"));
                    }

                    let previous = transfers.mutation_snapshot();
                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        if action == "retry"
                            && (entry.direction != 0
                                || !matches!(
                                    entry.status.as_str(),
                                    "failed" | "rejected" | "errored" | "cancelled"
                                ))
                        {
                            drop(transfers);
                            return Ok(routing::conflict_response("transfer is not retryable"));
                        }
                        // Check outbound transfer policy
                        if let Some(ref username) = entry.peer_username {
                            if !state.config.transfer_allow_outbound {
                                drop(transfers);
                                return Ok(routing::conflict_response(
                                    "outbound transfers are disabled",
                                ));
                            }

                            entry.previous_status = Some(entry.status.clone());
                            entry.status = "peer_lookup".to_owned();
                            entry.reason = None;
                            entry.updated_at = unix_timestamp();
                            entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
                            let json_response = entry.json();
                            let username_clone = username.clone();
                            let entry = entry.clone();
                            let mutation = transfers.mutation_snapshot();
                            drop(transfers);
                            if let Err(error) = persist_transfer_record(state, &entry).await {
                                rollback_transfer_mutation_if_unchanged(state, previous, mutation)
                                    .await;
                                return Err(error);
                            }

                            if let Some(address) =
                                test_user_endpoint_peer_address(state, &username_clone)
                            {
                                project_peer_transfer_response(state, &address).await;
                            } else {
                                let Some(session_command_permit) = session_command_permit else {
                                    return Err("missing reserved transfer peer dispatch capacity"
                                        .to_owned());
                                };
                                session_command_permit.send(SessionCommand::TransferPeer {
                                    id,
                                    username: username_clone,
                                });
                            }

                            Ok(routing::ok_response(json_response))
                        } else {
                            entry.previous_status = Some(entry.status.clone());
                            entry.status = "in_progress".to_owned();
                            entry.reason = None;

                            entry.updated_at = unix_timestamp();
                            entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
                            let json_response = entry.json();
                            let entry = entry.clone();
                            let mutation = transfers.mutation_snapshot();
                            drop(transfers);
                            if let Err(error) = persist_transfer_record(state, &entry).await {
                                rollback_transfer_mutation_if_unchanged(state, previous, mutation)
                                    .await;
                                return Err(error);
                            }
                            Ok(routing::ok_response(json_response))
                        }
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                } else if action == "progress" {
                    let Some(bytes_transferred) = extract_json_u64_field(body, "bytes_transferred")
                    else {
                        drop(transfers);
                        return Ok(routing::bad_request_response(
                            "bytes_transferred is required",
                        ));
                    };
                    let previous = transfers.mutation_snapshot();
                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        if is_terminal_transfer_status(&entry.status) {
                            drop(transfers);
                            return Ok(routing::conflict_response(
                                "terminal transfers cannot receive progress updates",
                            ));
                        }
                        if entry.size.is_some_and(|size| bytes_transferred > size) {
                            drop(transfers);
                            return Ok(routing::bad_request_response(
                                "bytes_transferred exceeds transfer size",
                            ));
                        }
                        entry.previous_status = Some(entry.status.clone());
                        entry.status = "in_progress".to_owned();
                        entry.bytes_transferred = bytes_transferred;
                        entry.updated_at = unix_timestamp();
                        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
                        let json_response = entry.json();
                        let entry = entry.clone();
                        let mutation = transfers.mutation_snapshot();
                        drop(transfers);
                        if let Err(error) = persist_transfer_progress_record(state, &entry).await {
                            rollback_transfer_mutation_if_unchanged(state, previous, mutation)
                                .await;
                            return Err(error);
                        }
                        Ok(routing::ok_response(json_response))
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                } else if action == "complete" {
                    let Some(bytes_transferred) = extract_json_u64_field(body, "bytes_transferred")
                    else {
                        drop(transfers);
                        return Ok(routing::bad_request_response(
                            "bytes_transferred is required",
                        ));
                    };
                    let status_str = extract_json_string_field(body, "status")
                        .map(|status| status.to_ascii_lowercase())
                        .unwrap_or_else(|| "succeeded".to_string());
                    if !is_terminal_transfer_status(&status_str) {
                        drop(transfers);
                        return Ok(routing::bad_request_response(
                            "status must be a terminal transfer status",
                        ));
                    }
                    let previous = transfers.mutation_snapshot();
                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        if entry.size.is_some_and(|size| bytes_transferred > size) {
                            drop(transfers);
                            return Ok(routing::bad_request_response(
                                "bytes_transferred exceeds transfer size",
                            ));
                        }
                        if is_successful_transfer_status(&status_str)
                            && entry.size.is_some_and(|size| bytes_transferred != size)
                        {
                            drop(transfers);
                            return Ok(routing::bad_request_response(
                                "successful transfers must report their complete size",
                            ));
                        }
                        entry.previous_status = Some(entry.status.clone());
                        entry.bytes_transferred = bytes_transferred;
                        entry.status = status_str.clone();
                        entry.updated_at = unix_timestamp();
                        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
                        let json_response = entry.json();
                        let entry_for_persistence = entry.clone();

                        // Prepare webhook dispatch
                        let webhook_event = match status_str.as_str() {
                            "succeeded" | "completed" => {
                                Some(webhooks::WebhookEvent::TransferCompleted)
                            }
                            "failed" | "rejected" | "errored" => {
                                Some(webhooks::WebhookEvent::TransferFailed)
                            }
                            "cancelled" => None,
                            _ => unreachable!("validated terminal transfer status"),
                        };

                        let webhook_data = serde_json::json!({
                            "transfer_id": id,
                            "filename": entry.filename.clone(),
                            "peer_username": entry.peer_username.clone().unwrap_or_else(|| "unknown".to_string()),
                            "direction": if entry.direction == 0 { "download" } else { "upload" },
                            "size": entry.size.unwrap_or(0),
                            "bytes_transferred": bytes_transferred,
                            "status": status_str.clone(),
                        });
                        let correlation_id = format!("transfer_{}", id);
                        let mutation = transfers.mutation_snapshot();

                        drop(transfers);
                        if let Err(error) =
                            persist_transfer_record(state, &entry_for_persistence).await
                        {
                            rollback_transfer_mutation_if_unchanged(state, previous, mutation)
                                .await;
                            return Err(error);
                        }
                        maybe_import_lidarr_completed_download(state, &entry_for_persistence).await;
                        maybe_upload_ftp_completed_download(state, &entry_for_persistence).await;

                        // Dispatch webhook
                        if let Some(webhook_event) = webhook_event {
                            dispatch_webhook_event(
                                state,
                                correlation_id,
                                webhook_event,
                                webhook_data,
                            )
                            .await;
                        }

                        Ok(routing::ok_response(json_response))
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                } else {
                    drop(transfers);
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::not_found_response())
            }
        }

        // TRANSFER STATISTICS ENDPOINTS
        ("GET", "/api/transfers/speeds") => {
            let transfers = state.transfers.read().await;
            let json = controller_transfer_speeds_json(&transfers);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        // USER PROFILE ENDPOINTS
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/info") => {
            let Some(username) = user_route_username(path, "/info") else {
                return Ok(routing::not_found_response());
            };
            if let Some(response) =
                controller_user_read_failure_response(state, route.path, &username, false).await
            {
                return Ok(response);
            }
            let users = state.users.read().await;
            if let Some(record) = users.records.iter().find(|u| u.username == username) {
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "description": "",
                        "hasFreeUploadSlot": true,
                        "hasPicture": false,
                        "picture": null,
                        "queueLength": 0,
                        "uploadSlots": 0,
                    })
                    .to_string()
                } else {
                    record.controller_info_json().to_string()
                };
                drop(users);
                Ok(routing::ok_response(json))
            } else {
                drop(users);
                if state.config.controller_profile == ControllerProfile::Legacy {
                    return Ok(routing::not_found_response());
                }
                let record = UserRecord {
                    username,
                    watched: false,
                    status: None,
                    privileged: false,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                };
                let json = if state.config.controller_profile == ControllerProfile::Legacy {
                    serde_json::json!({
                        "description": "",
                        "hasFreeUploadSlot": true,
                        "hasPicture": false,
                        "picture": null,
                        "queueLength": 0,
                        "uploadSlots": 0,
                    })
                    .to_string()
                } else {
                    record.controller_info_json().to_string()
                };
                Ok(routing::ok_response(json))
            }
        }

        ("POST", path) if path.starts_with("/api/users/") && path.ends_with("/directory") => {
            let Some(username) = user_route_username(path, "/directory") else {
                return Ok(routing::not_found_response());
            };
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if route.path.starts_with("/api/v0/") && directory.trim().is_empty() {
                // UsersController validates the bound request before it
                // checks whether the Soulseek connection is ready.
                return Ok(routing::bad_request_response("directory is required"));
            }
            if route.path.starts_with("/api/v0/") && state.session.read().await.state != "connected"
            {
                return Ok(routing::service_unavailable_response(
                    "Soulseek server connection is not ready",
                ));
            }
            let session_command_permit = if route.path.starts_with("/api/v0/") {
                match state.session_commands.reserve().await {
                    Ok(permit) => Some(permit),
                    Err(_) => {
                        return Ok(routing::service_unavailable_response(
                            "session manager is not running",
                        ));
                    }
                }
            } else {
                None
            };
            let browse = state.browse.read().await;
            let entries = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.entries.as_slice())
                .unwrap_or(&[]);
            let json = controller_user_directories_json(&directory, entries, route.query);
            drop(browse);
            if let Some(session_command_permit) = session_command_permit {
                session_command_permit.send(SessionCommand::BrowseFolder {
                    username: username.to_owned(),
                    folder: directory,
                });
            }
            Ok(routing::ok_response(json))
        }

        // USER STATUS ENDPOINTS
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

async fn remove_transfer_file_if_present(path: &str) -> Result<(), String> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("transfer file removal failed: {error}")),
    }
}
