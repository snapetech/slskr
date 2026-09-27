async fn route_dispatch_group_2_downloads(
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
        ("POST", path) if download_request_path(path).is_some() => {
            let Some((request_id, Some("cancel"))) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let mut transfers = state.transfers.write().await;
            let previous = transfers.mutation_snapshot();
            let ids = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && entry.request_id.as_deref() == Some(request_id)
                        && !is_terminal_transfer_status(&entry.status)
                })
                .map(|entry| entry.id)
                .collect::<Vec<_>>();
            let exists = transfers.entries.iter().any(|entry| {
                entry.direction == 0 && entry.request_id.as_deref() == Some(request_id)
            });
            if !exists {
                return Ok(routing::not_found_response());
            }
            let updated = ids
                .into_iter()
                .filter_map(|id| {
                    transfers.update_status(
                        id,
                        "cancelled",
                        None,
                        Some("cancelled by request".to_owned()),
                    )
                })
                .collect::<Vec<_>>();
            let mutated = transfers.mutation_snapshot();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &updated).await {
                rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::no_content_response())
        }

        ("GET", "/api/transfers/changes") => {
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let include_completed = query_parameter(route.query, "includeCompleted")
                .as_deref()
                .and_then(parse_bool_value)
                .unwrap_or(true);
            let snapshot_at = unix_timestamp_millis();
            let transfers = state.transfers.read().await;
            let rows = transfers
                .entries
                .iter()
                .filter(|entry| entry.updated_at_ms <= snapshot_at)
                .filter(|entry| {
                    since.is_some()
                        || include_completed
                        || !is_successful_transfer_status(&entry.status)
                })
                .filter(|entry| since.is_none_or(|since| entry.updated_at_ms > since))
                .map(TransferEntry::controller_file_json)
                .collect::<Vec<_>>();
            let download = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0)
                .count();
            let upload = transfers.entries.len().saturating_sub(download);
            Ok(routing::ok_response(
                serde_json::json!({
                    "cursor": snapshot_at,
                    "counts": { "download": download, "upload": upload },
                    "transfers": rows,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/transfers/history") => {
            let direction = query_parameter(route.query, "direction").unwrap_or_default();
            let direction = match direction.trim().to_ascii_lowercase().as_str() {
                "download" => 0,
                "upload" => 1,
                _ => {
                    return Ok(routing::bad_request_response(
                        "direction must be 'download' or 'upload'",
                    ))
                }
            };
            let as_of = match query_millis_parameter(route.query, "asOf") {
                Ok(value) => value.unwrap_or_else(unix_timestamp_millis),
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let offset = match query_bounded_usize(route.query, "offset", 0, usize::MAX) {
                Ok(value) => value.unwrap_or(0),
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "offset must be greater than or equal to zero",
                    ))
                }
            };
            let limit = match query_bounded_usize(route.query, "limit", 1, 500) {
                Ok(value) => value.unwrap_or(250),
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 500",
                    ))
                }
            };
            let transfers = state.transfers.read().await;
            let mut rows = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == direction)
                .filter(|entry| is_successful_transfer_status(&entry.status))
                .filter(|entry| entry.updated_at_ms <= as_of)
                .collect::<Vec<_>>();
            rows.sort_by_key(|entry| {
                std::cmp::Reverse((entry.updated_at_ms, entry.requested_at, entry.id))
            });
            let page = rows
                .into_iter()
                .skip(offset)
                .take(limit.saturating_add(1))
                .collect::<Vec<_>>();
            let has_more = page.len() > limit;
            let rows = page
                .into_iter()
                .take(limit)
                .map(TransferEntry::controller_file_json)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({
                    "asOf": as_of,
                    "hasMore": has_more,
                    "nextOffset": offset.saturating_add(rows.len()),
                    "transfers": rows,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/transfers/downloads/batches") => {
            Ok(controller_enqueue_download_batch(body, state).await)
        }

        ("POST", "/api/transfers") => {
            if controller_file_array_exceeds_wire_limits(body) {
                return Ok(routing::bad_request_response(
                    "transfer request exceeds file limits",
                ));
            }
            if let Some((username, mut files)) = controller_enqueue_request(body) {
                let exclusions = effective_download_exclusions(state).await;
                let filenames = files
                    .iter()
                    .filter_map(|file| file.get("filename").and_then(serde_json::Value::as_str))
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
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
                    let details = transfer_request_details_from_json(&file, &filename);
                    let relative = match render_configured_completed_download_path(
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
                    };
                    let local_path =
                        match configured_download_destination_path(state, &relative).await {
                            Ok(path) => Some(path.display().to_string()),
                            Err(error) => return Ok(routing::bad_request_response(&error)),
                        };
                    prepared.push((filename, local_path, size, details));
                }
                let mut transfers = state.transfers.write().await;
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
                drop(transfers);
                if let Err(error) = persist_transfer_records(state, &created_entries).await {
                    remove_transfer_entries_if_unchanged(state, &created_entries).await;
                    return Err(error);
                }
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "queued": count,
                        "transfers": created,
                        "blocked": blocked,
                    })
                    .to_string(),
                ));
            }

            let filename = match extract_json_string_field(body, "filename") {
                Some(f) => f,
                None => return Ok(routing::bad_request_response("filename is required")),
            };

            let direction = extract_json_u32_field(body, "direction").unwrap_or(0);
            if direction == 0 {
                if let Some(response) =
                    download_policy_response(state, std::slice::from_ref(&filename)).await
                {
                    return Ok(response);
                }
            }
            let peer_username = extract_json_string_field(body, "peer_username");
            let supplied_local_path = extract_json_string_field(body, "local_path");
            let batch_id = controller_transfer_batch_id(body);
            let size = extract_json_u64_field(body, "size");
            let payload =
                serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null);
            let details = if direction == 0 {
                transfer_request_details_from_json(&payload, &filename)
            } else {
                TransferRequestDetails::default()
            };
            let local_path = match prepare_transfer_local_path(
                state,
                direction,
                peer_username.as_deref(),
                &filename,
                batch_id.as_deref(),
                &details,
                supplied_local_path,
            )
            .await
            {
                Ok(path) => path,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };

            let mut transfers = state.transfers.write().await;
            let entry = transfers.create_with_details(
                direction,
                peer_username.clone(),
                filename.clone(),
                local_path.clone(),
                size,
                batch_id,
                details,
            );
            drop(transfers);
            if let Err(error) = persist_transfer_record(state, &entry).await {
                remove_transfer_entries_if_unchanged(state, std::slice::from_ref(&entry)).await;
                return Err(error);
            }
            Ok(routing::created_response(entry.json()))
        }

        ("GET", "/api/transfers/downloads") | ("GET", "/api/transfers/downloads/") => {
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            if route.path == "/api/downloads"
                && state.config.controller_profile == ControllerProfile::Native
            {
                let requested_status = query_parameter(route.query, "status")
                    .map(|value| value.to_ascii_lowercase())
                    .filter(|value| {
                        matches!(
                            value.as_str(),
                            "queued" | "running" | "completed" | "cancelled" | "failed"
                        )
                    });
                let downloads = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction == 0)
                    .filter(|entry| {
                        requested_status.as_deref().is_none_or(|requested| {
                            native_download_status(entry.status.as_str()) == requested
                        })
                    })
                    .map(native_compatibility_download_json)
                    .collect::<Vec<_>>();
                drop(transfers);
                return Ok(routing::ok_response(
                    serde_json::json!({"downloads": downloads}).to_string(),
                ));
            }
            let body = transfers.controller_transfers_json(0, None);
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/transfers/uploads") | ("GET", "/api/transfers/uploads/") => {
            if let Some(response) =
                controller_transfer_storage_read_failure_response(state, route.path).await
            {
                return Ok(response);
            }
            let transfers = state.transfers.read().await;
            let body = transfers.controller_transfers_json(1, None);
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/transfers/downloads/accelerated") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                let enabled = state.runtime.read().await.accelerated_downloads_enabled;
                return Ok(routing::ok_response(
                     serde_json::json!({
                         "enabled": enabled,
                         "updatedAt": chrono::Utc::now().to_rfc3339(),
                         "policy": "Normal downloads remain single-source. Underperforming downloads may use verified alternate sources; raw Soulseek peers use sequential failover, while true multipart chunking is reserved for trusted mesh-overlay peers.",
                     })
                     .to_string(),
                 ));
            }
            let transfers = state.transfers.read().await;
            let mut value = serde_json::from_str::<serde_json::Value>(
                &controller_accelerated_downloads_json(route.query, &transfers),
            )
            .unwrap_or_else(|_| serde_json::json!({}));
            value["updatedAt"] = serde_json::json!(unix_timestamp());
            value["policy"] = serde_json::json!({"enabled": false});
            drop(transfers);
            Ok(routing::ok_response(value.to_string()))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
