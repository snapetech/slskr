use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_02(
    context: &super::LegacyRouteDispatchContext<'_, '_>,
) -> Result<HttpResponse, String> {
    let super::LegacyRouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    } = *context;
    let _ = (
        authorization,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    );
    match (method, normalized_path.as_str()) {
        ("POST", _path)
            if search_token_path(normalized_path.as_str(), "/cancel").is_some()
                || search_token_path(normalized_path.as_str(), "/fail").is_some()
                || search_token_path(normalized_path.as_str(), "/expire").is_some() =>
        {
            let (token, status, event_kind) =
                if let Some(token) = search_token_path(normalized_path.as_str(), "/cancel") {
                    (token, "cancelled", "search.cancelled")
                } else if let Some(token) = search_token_path(normalized_path.as_str(), "/fail") {
                    (token, "failed", "search.failed")
                } else if let Some(token) = search_token_path(normalized_path.as_str(), "/expire") {
                    (token, "expired", "search.expired")
                } else {
                    return Ok(routing::not_found_response());
                };
            let mut searches = state.searches.write().await;
            if let Some((record, transitioned)) = searches.set_status_by_token(token, status) {
                let body_json = record.json();
                drop(searches);
                if transitioned {
                    persist_search_record(state, &record).await?;
                    record_event(state, event_kind, token.to_string(), None).await;
                }
                Ok(routing::ok_response(body_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("POST", "/api/searches/prune") => {
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let pruned_records = searches.prune_expired();
            let pruned = pruned_records.len();
            let remaining = searches.records.len();
            let mutated_searches = searches.clone();
            drop(searches);
            for record in &pruned_records {
                if let Err(error) = delete_persisted_search(state, record).await {
                    rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                        .await;
                    return Err(error);
                }
                publish_search_hub_event(state, "delete", record);
            }
            Ok(routing::ok_response(format!("{{\"pruned\":{},\"remaining\":{}}}", pruned, remaining)))
        }

        ("POST", "/api/search-responses") => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload) => payload,
                Err(_) => return Ok(routing::bad_request_response("invalid JSON body")),
            };
            if search_response_exceeds_wire_limits(&payload) {
                return Ok(routing::bad_request_response(
                    "search response exceeds result limits",
                ));
            }

            let token = match payload.get("token").and_then(serde_json::Value::as_u64) {
                Some(t) => match u32::try_from(t) {
                    Ok(token) => token,
                    Err(_) => {
                        return Ok(routing::bad_request_response("token exceeds u32 range"));
                    }
                },
                None => return Ok(routing::bad_request_response("token is required")),
            };

            let peer_username = payload
                .get("peer_username")
                .or_else(|| payload.get("username"))
                .and_then(serde_json::Value::as_str);
            let slot_free = payload
                .get("slot_free")
                .or_else(|| payload.get("hasFreeUploadSlot"))
                .and_then(serde_json::Value::as_bool);
            let average_speed = payload
                .get("average_speed")
                .or_else(|| payload.get("uploadSpeed"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let queue_length = payload
                .get("queue_length")
                .or_else(|| payload.get("queueLength"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let locked = payload
                .get("locked")
                .or_else(|| payload.get("isLocked"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let mut entries = Vec::new();
            if let Some(files) = payload.get("files").and_then(serde_json::Value::as_array) {
                entries.extend(files.iter().filter_map(|file| {
                    SearchResultEntry::from_json_file(
                        file,
                        peer_username,
                        false,
                        slot_free,
                        average_speed,
                        queue_length,
                    )
                }));
            }
            if let Some(files) = payload
                .get("lockedFiles")
                .and_then(serde_json::Value::as_array)
            {
                entries.extend(files.iter().filter_map(|file| {
                    SearchResultEntry::from_json_file(
                        file,
                        peer_username,
                        true,
                        slot_free,
                        average_speed,
                        queue_length,
                    )
                }));
            }

            if entries.is_empty() {
                let entry = SearchResultEntry::from_json_file(
                    &payload,
                    peer_username,
                    locked,
                    slot_free,
                    average_speed,
                    queue_length,
                )
                .unwrap_or_else(|| bounded_search_result_entry(SearchResultEntry {
                    peer_username: peer_username.map(str::to_owned),
                    filename: String::new(),
                    size: payload
                        .get("size")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked,
                    slot_free,
                    average_speed,
                    queue_length,
                    extension: String::new(),
                }));
                entries.push(entry);
            }

            let wishlist_item_id = {
                let searches = state.searches.read().await;
                searches
                    .get(token)
                    .and_then(|record| record.wishlist_item_id().map(str::to_owned))
            };
            let _wishlist_search_persistence = if wishlist_item_id.is_some() {
                Some(state.wishlist_search_persistence_lock.lock().await)
            } else {
                None
            };
            let (wishlist_policy, wishlist_counts) = if let Some(item_id) = wishlist_item_id.as_deref() {
                let wishlist = state.wishlist.read().await;
                let ignored_results = wishlist.ignored_results_for(item_id);
                let policy = wishlist.result_policy_for(item_id);
                let mut filtered_out = 0_usize;
                let mut ignored = 0_usize;
                let mut hidden_locked = 0_usize;
                for entry in &entries {
                    if policy
                        .as_ref()
                        .is_some_and(|policy| !policy.filter.matches_entry(entry))
                    {
                        filtered_out = filtered_out.saturating_add(1);
                    } else if entry.peer_username.as_deref().is_some_and(|username| {
                        ignored_results
                            .iter()
                            .any(|rule| rule.matches(username, &entry.filename))
                    }) {
                        ignored = ignored.saturating_add(1);
                    } else if entry.locked {
                        hidden_locked = hidden_locked.saturating_add(1);
                    }
                }
                entries.retain(|entry| {
                    if entry.locked
                        || policy
                            .as_ref()
                            .is_some_and(|policy| !policy.filter.matches_entry(entry))
                    {
                        return false;
                    }
                    let Some(username) = entry.peer_username.as_deref() else {
                        return true;
                    };
                    !ignored_results
                        .iter()
                        .any(|rule| rule.matches(username, &entry.filename))
                });
                (policy, Some((filtered_out, ignored, hidden_locked)))
            } else {
                (None, None)
            };

            let mut searches = state.searches.write().await;
            let aggregate_remaining =
                MAX_TOTAL_SEARCH_RESULTS.saturating_sub(searches.total_results());
            if let Some(record) = searches.records.iter_mut().find(|r| r.token == token) {
                if let Some((filtered_out, ignored, hidden_locked)) = wishlist_counts {
                    record.raw_response_count = record.raw_response_count.saturating_add(1);
                    record.filtered_out_count =
                        record.filtered_out_count.saturating_add(filtered_out);
                    record.ignored_result_count =
                        record.ignored_result_count.saturating_add(ignored);
                    record.hidden_locked_count =
                        record.hidden_locked_count.saturating_add(hidden_locked);
                }
                record.extend_results_with_limit_bounded(
                    entries,
                    aggregate_remaining,
                    wishlist_policy
                        .as_ref()
                        .map(|policy| policy.max_results)
                        .unwrap_or(MAX_SEARCH_RESULTS_PER_SEARCH),
                );
                record.updated_at = unix_timestamp();
                let response_json = record.json();
                let record = record.clone();
                drop(searches);
                persist_search_record(state, &record).await?;
                publish_search_hub_event(state, "update", &record);
                Ok(routing::ok_response(response_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        // TRANSFER ENDPOINTS
        ("GET", "/api/downloads") => {
            let requested_state = query_parameter(route.query, "status")
                .or_else(|| query_parameter(route.query, "state"))
                .map(|value| value.to_ascii_lowercase())
                .filter(|value| {
                    matches!(value.as_str(), "active" | "completed" | "failed" | "cancelled")
                });
            let transfers = state.transfers.read().await;
            let downloads = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0)
                .filter(|entry| {
                    requested_state.as_deref().is_none_or(|requested| {
                        native_download_status(entry.status.as_str()) == requested
                    })
                })
                .map(native_compatibility_download_json)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({"downloads": downloads}).to_string(),
            ))
        }
        ("GET", "/api/v0/downloads/requests")
        | ("GET", "/api/downloads/requests") => {
            let requested_state = query_parameter(route.query, "state")
                .map(|value| value.to_ascii_lowercase())
                .filter(|value| {
                    matches!(value.as_str(), "active" | "completed" | "failed" | "cancelled")
                });
            let transfers = state.transfers.read().await;
            let mut grouped = std::collections::BTreeMap::<&str, Vec<&TransferEntry>>::new();
            for entry in transfers.entries.iter().filter(|entry| entry.direction == 0) {
                if let Some(request_id) = entry.request_id.as_deref() {
                    grouped.entry(request_id).or_default().push(entry);
                }
            }
            let mut requests = grouped
                .into_values()
                .filter(|entries| {
                    requested_state.as_deref().is_none_or(|requested| {
                        download_request_state(entries).eq_ignore_ascii_case(requested)
                    })
                })
                .map(|entries| download_request_projection(&entries, false))
                .collect::<Vec<_>>();
            requests.sort_by_key(|entry| {
                std::cmp::Reverse(entry["request"]["createdAt"].as_u64().unwrap_or(0))
            });
            Ok(routing::ok_response(serde_json::Value::Array(requests).to_string()))
        }

        ("GET", path) if download_request_path(path).is_some() => {
            let Some((request_id, None)) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let transfers = state.transfers.read().await;
            let attempts = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0 && entry.request_id.as_deref() == Some(request_id))
                .collect::<Vec<_>>();
            if attempts.is_empty() {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(
                download_request_projection(&attempts, true).to_string(),
            ))
        }

        ("PATCH", path) if download_request_path(path).is_some() => {
            let Some((request_id, Some("name"))) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let Some(name) = extract_json_string_field(body, "name")
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
            else {
                return Ok(routing::bad_request_response("name is required"));
            };
            if name.len() > MAX_TRANSFER_REQUEST_NAME_BYTES {
                return Ok(routing::bad_request_response(
                    "name must be 512 bytes or fewer",
                ));
            }
            let mut transfers = state.transfers.write().await;
            let previous = transfers.mutation_snapshot();
            let mut updated = Vec::new();
            for entry in transfers.entries.iter_mut().filter(|entry| {
                entry.direction == 0 && entry.request_id.as_deref() == Some(request_id)
            }) {
                entry.request_name = Some(name.clone());
                entry.updated_at = unix_timestamp();
                entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
                updated.push(entry.clone());
            }
            if updated.is_empty() {
                return Ok(routing::not_found_response());
            }
            let mutated = transfers.mutation_snapshot();
            transfers.persist_state();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &updated).await {
                rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            let attempts = updated.iter().collect::<Vec<_>>();
            Ok(routing::ok_response(
                download_request_projection(&attempts, true).to_string(),
            ))
        }

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
                    .filter(|filename| {
                        crate::download_filter::is_excluded(
                            filename,
                            &exclusions,
                        )
                    })
                    .count();
                if blocked == filenames.len() && !filenames.is_empty() {
                    return Ok(crate::route_dispatch::download_policy_response(
                        state,
                        &filenames,
                    )
                    .await
                    .expect("non-empty blocked list must produce a policy response"));
                }
                files.retain(|file| {
                    file.get("filename")
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(|filename| {
                            !crate::download_filter::is_excluded(
                                filename,
                                &exclusions,
                            )
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
                    let local_path = match
                        configured_download_destination_path(state, &relative).await
                    {
                        Ok(path) => Some(path.display().to_string()),
                        Err(error) => return Ok(routing::bad_request_response(&error)),
                    };
                    prepared.push((filename, local_path, size, details));
                }
                let (created, created_entries, previous, mutated) = {
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
                    let mutated = transfers.mutation_snapshot();
                    (created, created_entries, previous, mutated)
                };
                let count = created.len();
                if let Err(error) = persist_transfer_records(state, &created_entries).await {
                    rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "queued": count,
                        "transfers": created,
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
                if let Some(response) = crate::route_dispatch::download_policy_response(
                    state,
                    std::slice::from_ref(&filename),
                )
                .await
                {
                    return Ok(response);
                }
            }
            let peer_username = extract_json_string_field(body, "peer_username");
            let supplied_local_path = extract_json_string_field(body, "local_path");
            let batch_id = controller_transfer_batch_id(body);
            let size = extract_json_u64_field(body, "size");
            let payload = serde_json::from_str::<serde_json::Value>(body)
                .unwrap_or(serde_json::Value::Null);
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
            ).await {
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
                 && state.config.controller_profile
                     == ControllerProfile::Native
             {
                 let requested_status = query_parameter(route.query, "status")
                     .map(|value| value.to_ascii_lowercase())
                     .filter(|value| {
                         matches!(value.as_str(), "queued" | "running" | "completed" | "cancelled" | "failed")
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
                 && state.config.controller_profile
                     == ControllerProfile::Native
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

         ("GET", "/api/transfers/downloads/auto-replace/status")
             if route.path.starts_with("/api/v0/")
                 && state.config.controller_profile
                     == ControllerProfile::Native =>
         {
             let stuck_count = state
                 .transfers
                 .read()
                 .await
                 .entries
                 .iter()
                 .filter(|entry| {
                     entry.direction == 0 && is_failed_transfer_status(&entry.status)
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
             let uploads = transfers.entries.iter().filter(|entry| entry.direction != 0).collect::<Vec<_>>();
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
             let batch_id = decoded_path_segment(
                 path.trim_start_matches("/api/transfers/downloads/batches/"),
             );
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
             Ok(routing::ok_response(serde_json::json!({
                 "id": batch_id,
                 "transfers": downloads,
                 "transferCount": transfer_count,
                 "completedCount": completed_count,
                 "failedCount": failed_count,
             }).to_string()))
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

         ("GET", path) if controller_transfer_file_path(path, "downloads").is_some() && !path.ends_with("/position") => {
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

         ("GET", path) if controller_transfer_file_path(path, "uploads").is_some() && !path.ends_with("/position") => {
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
                // slskd's empty queue-position contract immediately rather
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
                 } if response_filename == filename => {
                     Ok(routing::ok_response(place.to_string()))
                 }
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
            let Some(alternative) =
                searches.find_transfer_alternative(&original, &username, requested_filename.as_deref())
            else {
                drop(searches);
                return Ok(routing::conflict_response("no matching alternative found"));
            };
            drop(searches);
            let Some(replacement_username) = alternative.peer_username.clone() else {
                return Ok(routing::conflict_response(
                    "matching alternative has no peer",
                ));
            };
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
                    attempts: original.attempts,
                    auto_replace_attempts: original.auto_replace_attempts,
                    next_attempt_at: original.next_attempt_at,
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

            let searches = state.searches.read().await;
            let replacements = candidates
                .iter()
                .filter_map(|transfer| {
                    searches
                        .first_transfer_alternative(transfer)
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
                        attempts: original.attempts,
                        auto_replace_attempts: original.auto_replace_attempts,
                        next_attempt_at: original.next_attempt_at,
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
             let files = controller_files_from_body(body);
             if route.path.starts_with("/api/v0/") && files.is_empty() {
                 return Ok(HttpResponse {
                     status: "400 Bad Request",
                     content_type: "application/json",
                     body: serde_json::json!("At least one file is required").to_string(),
                 });
             }
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
                     ).await {
                         Ok(relative) => relative,
                         Err(error) => return Ok(routing::bad_request_response(&error)),
                     }
                 };
                 let local_path = match
                     configured_download_destination_path(state, &relative).await
                 {
                     Ok(path) => Some(path.display().to_string()),
                     Err(error) => return Ok(routing::bad_request_response(&error)),
                 };
                 prepared.push((filename, local_path, size, details));
             }
             let (created, created_entries, previous, mutated) = {
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
                 let mutated = transfers.mutation_snapshot();
                 (created, created_entries, previous, mutated)
             };
             let count = created.len();
             if let Err(error) = persist_transfer_records(state, &created_entries).await {
                 rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                 return Ok(routing::service_unavailable_response(&error));
             }
             Ok(routing::ok_response(serde_json::json!({
                 "queued": count,
                 "transfers": created,
             }).to_string()))
         }

         ("DELETE", "/api/transfers/downloads/all/completed")
         | ("DELETE", "/api/transfers/uploads/all/completed") => {
             let direction = if normalized_path.contains("/downloads/") { 0 } else { 1 };
             let mut transfers = state.transfers.write().await;
             let previous = transfers.mutation_snapshot();
             let before = transfers.entries.len();
             let mut removed_entries = Vec::new();
             transfers.entries.retain(|entry| {
                 let remove = entry.direction == direction
                     && matches!(
                        entry.status.as_str(),
                        "succeeded"
                            | "completed"
                            | "cancelled"
                            | "failed"
                            | "rejected"
                            | "errored"
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

         ("DELETE", path) if controller_transfer_file_path(path, "downloads").is_some() => {
             let Some((username, id)) = controller_transfer_file_path(path, "downloads") else {
                 return Ok(routing::not_found_response());
             };
              let username = decoded_path_segment(username);
              let remove_file = query_parameter(route.query, "remove").is_some_and(|value| value == "true")
                  || query_parameter(route.query, "deleteFile").is_some_and(|value| value == "true");
              let mut transfers = state.transfers.write().await;
              let target = transfers.entries.iter().find(|entry| {
                  entry.id == id && entry.direction == 0 && entry.peer_username.as_deref() == Some(username.as_str())
              }).cloned();
              let Some(target) = target else { return Ok(routing::not_found_response()); };
              let previous = transfers.mutation_snapshot();
              let updated = transfers.update_status(id, "cancelled", None, None);
              let mutation = transfers.mutation_snapshot();
             drop(transfers);
             if let Some(entry) = updated.as_ref() {
                 if let Err(error) = persist_transfer_record(state, entry).await {
                     rollback_transfer_mutation_if_unchanged(state, previous, mutation).await;
                      return Ok(routing::service_unavailable_response(&error));
                 }
             }
              if remove_file {
                  if let Some(path) = target.local_path.as_deref() {
                      if let Err(error) = controller_remove_transfer_file_if_present(path).await {
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
              let remove_file = query_parameter(route.query, "remove").is_some_and(|value| value == "true")
                  || query_parameter(route.query, "deleteFile").is_some_and(|value| value == "true");
              let mut transfers = state.transfers.write().await;
              let target = transfers.entries.iter().find(|entry| {
                  entry.id == id && entry.direction == 1 && entry.peer_username.as_deref() == Some(username.as_str())
              }).cloned();
              let Some(target) = target else { return Ok(routing::not_found_response()); };
              let previous = transfers.mutation_snapshot();
              let updated = transfers.update_status(id, "cancelled", None, None);
              let mutation = transfers.mutation_snapshot();
             drop(transfers);
             if let Some(entry) = updated.as_ref() {
                 if let Err(error) = persist_transfer_record(state, entry).await {
                     rollback_transfer_mutation_if_unchanged(state, previous, mutation).await;
                      return Ok(routing::service_unavailable_response(&error));
                 }
             }
              if remove_file {
                  if let Some(path) = target.local_path.as_deref() {
                      if let Err(error) = controller_remove_transfer_file_if_present(path).await {
                          return Ok(routing::service_unavailable_response(&error));
                      }
                  }
              }
              Ok(routing::no_content_response())
         }

         // GET individual transfer
         ("GET", path) if (path.starts_with("/api/transfers/") || path.starts_with("/api/v0/transfers/"))
             && !path.ends_with("/start") && !path.ends_with("/progress") && !path.ends_with("/complete")
             && !path.ends_with("/speeds")
             && !path.ends_with("/stats") => {
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
         ("DELETE", path) if (path.starts_with("/api/transfers/") || path.starts_with("/api/v0/transfers/"))
             && transfer_action_path(normalized_path.as_str()).is_none() => {
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

         ("POST", _path) if transfer_action_path(normalized_path.as_str()).is_some() => {
            if let Some((id, action)) = transfer_action_path(normalized_path.as_str()) {
                let session_command_permit = if action == "start" || action == "retry" {
                    let transfers = state.transfers.read().await;
                    let active_count = transfers
                        .entries
                        .iter()
                        .filter(|transfer| {
                            is_active_transfer_status(&transfer.status)
                        })
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
                    let active_count = transfers.entries.iter()
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
                                return Ok(routing::conflict_response("outbound transfers are disabled"));
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

                            if let Some(address) = test_user_endpoint_peer_address(state, &username_clone) {
                                project_peer_transfer_response(state, &address).await;
                            } else {
                                let Some(session_command_permit) = session_command_permit else {
                                    return Err(
                                        "missing reserved transfer peer dispatch capacity".to_owned(),
                                    );
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
                    let Some(bytes_transferred) = extract_json_u64_field(body, "bytes_transferred") else {
                        drop(transfers);
                        return Ok(routing::bad_request_response("bytes_transferred is required"));
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
                    let Some(bytes_transferred) = extract_json_u64_field(body, "bytes_transferred") else {
                        drop(transfers);
                        return Ok(routing::bad_request_response("bytes_transferred is required"));
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
                         let webhook_event = if is_successful_transfer_status(&status_str) {
                                 Some(webhooks::WebhookEvent::TransferCompleted)
                             } else if is_failed_transfer_status(&status_str) {
                                 Some(webhooks::WebhookEvent::TransferFailed)
                             } else {
                                 None
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
        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
