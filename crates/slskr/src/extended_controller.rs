use super::*;

#[path = "extended_controller_dynamic_get.rs"]
mod dynamic_get;
#[path = "extended_controller_get.rs"]
mod get;
#[path = "extended_controller_mutations.rs"]
mod mutations;

pub(super) use self::dynamic_get::extended_controller_dynamic_get_response;
pub(super) use self::get::extended_controller_get_response;
pub(super) use self::mutations::extended_controller_mutation_response;

/// Real search-record creation shared by `/api/search` and
/// `/api/bridge/search` -- returns the real `SearchRecord` so each
/// caller can shape its own real response (the oracle's `/api/search`
/// contract vs. the oracle's real `BridgeSearchResult`) without
/// duplicating the real search-creation/dispatch side effects.
pub(super) async fn extended_controller_search_response(
    body: &str,
    state: &AppState,
) -> Result<SearchRecord, HttpResponse> {
    let Some(query) = extract_json_string_field(body, "query")
        .or_else(|| extract_json_string_field(body, "searchText"))
        .map(|query| query.trim().to_owned())
        .filter(|query| !query.trim().is_empty())
    else {
        return Err(routing::bad_request_response(
            "query/searchText is required",
        ));
    };
    let permit = match state.session_commands.reserve().await {
        Ok(permit) => permit,
        Err(_) => {
            return Err(routing::service_unavailable_response(
                "session manager is not running",
            ))
        }
    };
    let shares = state.shares.read().await;
    let matching = search_shares(&shares.entries, &query);
    drop(shares);
    let mut searches = state.searches.write().await;
    let previous_searches = searches.clone();
    let outcome = match searches.create(
        None,
        query.clone(),
        "global",
        None,
        matching,
        DEFAULT_SEARCH_TTL_SECONDS,
    ) {
        Ok(outcome) => outcome,
        Err(error) => return Err(search_create_error_response(error)),
    };
    let record = outcome.record;
    let evicted = outcome.evicted;
    let expired = outcome.expired;
    let mutated_searches = searches.clone();
    drop(searches);
    let mut upserts = expired.clone();
    upserts.push(record.clone());
    if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
        return Err(routing::service_unavailable_response(&error));
    }
    for expired_record in &expired {
        publish_search_hub_event(state, "update", expired_record);
    }
    permit.send(SessionCommand::Search {
        token: record.token,
        query,
        target: SearchDispatchTarget::Global,
    });
    publish_search_hub_event(state, "create", &record);
    Ok(record)
}

/// Real per-item download-queue creation shared by `/api/downloads` and
/// `/api/bridge/download` -- returns the real queued `TransferEntry`
/// list so each caller can shape its own real response (the oracle's
/// batch `/api/downloads` contract vs. the oracle's real single-item
/// `BridgeDownloadRequest`/`{transfer_id}`) without duplicating the
/// real queue-creation/dispatch side effects.
pub(super) fn transfer_batch_with_entries(
    mut batch: serde_json::Value,
    entries: impl IntoIterator<Item = serde_json::Value>,
) -> serde_json::Value {
    batch["transfers"] = serde_json::Value::Array(entries.into_iter().collect());
    batch
}

pub(super) async fn controller_enqueue_download_batch(
    body: &str,
    state: &AppState,
) -> HttpResponse {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(serde_json::Value::Object(payload)) => serde_json::Value::Object(payload),
        Ok(_) => return routing::bad_request_response("The request body must be an object"),
        Err(_) => return routing::bad_request_response("invalid JSON body"),
    };
    if payload
        .get("files")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|files| files.len() > MAX_TRANSFER_REQUEST_FILES)
    {
        return routing::bad_request_response("download batch exceeds file limits");
    }
    let username = payload
        .get("username")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= MAX_TRANSFER_USERNAME_BYTES)
        .map(str::to_owned);
    let Some(username) = username else {
        return routing::bad_request_response("Username is required");
    };
    let Some(files) = payload
        .get("files")
        .and_then(serde_json::Value::as_array)
        .filter(|files| !files.is_empty())
    else {
        return routing::bad_request_response("At least one file is required");
    };
    if files.iter().any(serde_json::Value::is_null) {
        return routing::bad_request_response("One or more files in the request are null");
    }
    let exclusions = effective_download_exclusions(state).await;
    let parse_guid = |name: &str| -> Result<Option<String>, HttpResponse> {
        let Some(value) = payload.get(name) else {
            return Ok(None);
        };
        if value.is_null() {
            return Ok(None);
        }
        let value = value.as_str().map(str::trim).unwrap_or_default();
        if value.is_empty() {
            return Ok(None);
        }
        uuid::Uuid::parse_str(value)
            .map(|value| Some(value.to_string()))
            .map_err(|_| {
                routing::bad_request_response(
                    "One or more provided identifiers is not a valid GUID/UUID",
                )
            })
    };
    let batch_id = match parse_guid("id") {
        Ok(value) => value.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        Err(response) => return response,
    };
    let search_id = match parse_guid("searchId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let _request_permit = match Arc::clone(&state.download_batch_requests).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return HttpResponse {
                status: "429 Too Many Requests",
                content_type: "application/json",
                body: serde_json::json!(
                    "Only one concurrent operation is permitted. Wait until the previous request completes"
                )
                .to_string(),
            }
        }
    };
    let options = payload
        .get("options")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let destination = options
        .get("destination")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let downloads_dir = effective_downloads_dir(state);
    if destination.as_deref().is_some_and(|destination| {
        safe_download_path(
            &downloads_dir,
            &format!("{destination}/slskr-batch-path-validation"),
        )
        .is_err()
    }) {
        return routing::bad_request_response(
            "Destination must be relative and stay within the download root",
        );
    }
    let mut seen_filenames = HashSet::new();
    let mut requested_files = Vec::with_capacity(files.len());
    for file in files {
        let Some(filename) = file
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return routing::bad_request_response("Each file requires a filename");
        };
        if !seen_filenames.insert(filename.to_owned()) {
            return routing::bad_request_response("Two or more files in the request are repeated");
        }
        let Some(size) = file.get("size").and_then(serde_json::Value::as_u64) else {
            return routing::bad_request_response("Each file requires a non-negative size");
        };
        if safe_download_path(&downloads_dir, filename).is_err() {
            return routing::bad_request_response(
                "One or more files in the request contain a dangerous path traversal segment",
            );
        }
        requested_files.push((filename.to_owned(), size));
    }

    let blocked = requested_files
        .iter()
        .filter_map(|(filename, _)| {
            crate::download_filter::matching_exclusion(filename, &exclusions).map(|exclusion| {
                serde_json::json!({
                    "filename": filename,
                    "exclusion": exclusion,
                    "message": "Download blocked by global exclusion",
                })
            })
        })
        .collect::<Vec<_>>();
    if blocked.len() == requested_files.len() && !blocked.is_empty() {
        return HttpResponse {
            status: "403 Forbidden",
            content_type: "application/json",
            body: serde_json::json!({
                "type": "download_blocked",
                "title": "Download blocked",
                "detail": "Every requested file matched a configured global download exclusion.",
                "blocked": blocked,
            })
            .to_string(),
        };
    }
    requested_files
        .retain(|(filename, _)| !crate::download_filter::is_excluded(filename, &exclusions));

    if let Err(error) = request_peer_endpoint(state, &username).await {
        return HttpResponse {
            status: "404 Not Found",
            content_type: "application/json",
            body: serde_json::json!(error).to_string(),
        };
    }

    let existing_record = match controller_read_transfer_batch(state, &batch_id).await {
        Ok(record) => record.is_some(),
        Err(error) => return routing::service_unavailable_response(&error),
    };
    let existing_transfer = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|entry| entry.batch_id.as_deref() == Some(batch_id.as_str()));
    if existing_record || existing_transfer {
        return HttpResponse {
            status: "409 Conflict",
            content_type: "application/json",
            body: serde_json::json!(format!("A batch with ID {batch_id} already exists"))
                .to_string(),
        };
    }

    let mut public_options = serde_json::Map::new();
    if let Some(destination) = destination.as_ref() {
        public_options.insert(
            "destination".to_owned(),
            serde_json::Value::String(destination.clone()),
        );
    }
    // Frozen slskd accepts ExternalId in the request DTO but does not copy it
    // into the persisted BatchOptions record in EnqueueBatchAsync.
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    let mut batch = serde_json::json!({
        "id": batch_id,
        "username": username,
        "direction": "Download",
        "createdAt": created_at,
        "transfers": [],
        "options": serde_json::Value::Object(public_options),
    });
    if let Some(search_id) = search_id.as_ref() {
        batch["searchId"] = serde_json::Value::String(search_id.clone());
    }
    if let Err(error) = controller_create_transfer_batch(state, &batch).await {
        if controller_transfer_batch_insert_is_duplicate(&error) {
            return HttpResponse {
                status: "409 Conflict",
                content_type: "application/json",
                body: serde_json::json!(format!("A batch with ID {batch_id} already exists"))
                    .to_string(),
            };
        }
        return routing::service_unavailable_response(&error);
    }

    let mut failures = blocked;
    let mut file_failures = Vec::new();
    let mut prepared = Vec::new();
    for (index, (filename, size)) in requested_files.into_iter().enumerate() {
        match prepare_download_batch_transfer(
            state,
            index,
            &username,
            &filename,
            size,
            &batch_id,
            destination.as_deref(),
        )
        .await
        {
            Ok(transfer) => prepared.push(transfer),
            Err(BatchTransferPreparationError::Duplicate) => {
                file_failures.push((
                    index,
                    serde_json::json!({
                        "filename": filename,
                        "message": "Transfer is already queued or in progress",
                    }),
                ));
            }
            Err(BatchTransferPreparationError::OutboundDisabled) => {
                file_failures.push((
                    index,
                    serde_json::json!({
                        "filename": filename,
                        "message": "Outbound transfers are disabled",
                    }),
                ));
            }
            Err(BatchTransferPreparationError::DispatchUnavailable) => {
                file_failures.push((
                    index,
                    serde_json::json!({
                        "filename": filename,
                        "message": "Transfer dispatch capacity is unavailable",
                    }),
                ));
            }
            Err(BatchTransferPreparationError::Path(error)) => {
                let duplicate = {
                    let transfers = state.transfers.read().await;
                    has_active_download(&transfers, &username, &filename)
                };
                if duplicate {
                    file_failures.push((
                        index,
                        serde_json::json!({
                            "filename": filename,
                            "message": "Transfer is already queued or in progress",
                        }),
                    ));
                } else {
                    file_failures.push((
                        index,
                        serde_json::json!({"filename": filename, "message": error}),
                    ));
                }
            }
        }
    }

    let (staged, commit_failures) =
        commit_prepared_download_batch_transfers(state, &username, &batch_id, prepared).await;
    file_failures.extend(commit_failures);
    file_failures.sort_by_key(|(index, _)| *index);
    failures.extend(file_failures.into_iter().map(|(_, failure)| failure));

    let staged_entries = staged
        .iter()
        .map(|(entry, _)| entry.clone())
        .collect::<Vec<_>>();
    if let Err(error) = persist_transfer_records(state, &staged_entries).await {
        let removed = remove_transfer_entries_if_unchanged(state, &staged_entries).await;
        if removed.len() == staged_entries.len() {
            if let Err(cleanup_error) = controller_delete_transfer_batch(state, &batch_id).await {
                record_daemon_log(
                    state,
                    logging::LogLevel::Error,
                    "transfers",
                    format!(
                        "failed to remove transfer batch after persistence failure: {cleanup_error}"
                    ),
                )
                .await;
            }
        }
        return routing::service_unavailable_response(&error);
    }
    let public_entries = staged_entries
        .iter()
        .map(TransferEntry::controller_file_json)
        .collect::<Vec<_>>();
    batch = transfer_batch_with_entries(batch, public_entries);
    if state.db.is_none() {
        if let Err(error) = state
            .controller_features
            .upsert(transfer_batch_record_key(&batch_id), batch.clone())
            .await
        {
            let removed = remove_transfer_entries_if_unchanged(state, &staged_entries).await;
            if !removed.is_empty() {
                if let Err(cleanup_error) = delete_persisted_transfers(state, &removed).await {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Error,
                        "transfers",
                        format!(
                            "failed to remove staged transfer records after batch persistence failure: {cleanup_error}"
                        ),
                    )
                    .await;
                }
            }
            if removed.len() == staged_entries.len() {
                if let Err(cleanup_error) = controller_delete_transfer_batch(state, &batch_id).await
                {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Error,
                        "transfers",
                        format!(
                            "failed to remove transfer batch after batch persistence failure: {cleanup_error}"
                        ),
                    )
                    .await;
                }
            }
            return routing::service_unavailable_response(&error);
        }
    }
    for (entry, permit) in staged {
        permit.send(SessionCommand::TransferPeer {
            id: entry.id,
            username: username.clone(),
        });
    }
    let status = if failures.len() == files.len() {
        "200 OK"
    } else if failures.is_empty() {
        "201 Created"
    } else {
        "207 Multi-Status"
    };
    HttpResponse {
        status,
        content_type: "application/json",
        body: serde_json::json!({"batch": batch, "failures": failures}).to_string(),
    }
}

pub(super) async fn extended_controller_download_response(
    body: &str,
    state: &AppState,
) -> Result<Vec<TransferEntry>, HttpResponse> {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload) => payload,
        Err(_) => return Err(routing::bad_request_response("invalid JSON body")),
    };
    if ["items", "Items"].into_iter().any(|field| {
        payload
            .get(field)
            .is_some_and(|items| json_array_exceeds_limit(items, MAX_EXTENDED_DOWNLOAD_ITEMS))
    }) {
        return Err(routing::bad_request_response(
            "items must contain at most 1000 items",
        ));
    }
    let items = payload
        .get("items")
        .or_else(|| payload.get("Items"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .or_else(|| {
            (payload.get("username").is_some() || payload.get("user").is_some())
                .then(|| vec![payload.clone()])
        })
        .unwrap_or_default();
    if items.is_empty() {
        return Err(routing::bad_request_response("items are required"));
    }
    let mut requests = Vec::new();
    for item in items.into_iter().take(1_000) {
        let username = item
            .get("user")
            .or_else(|| item.get("User"))
            .or_else(|| item.get("username"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let filename = item
            .get("remotePath")
            .or_else(|| item.get("RemotePath"))
            .or_else(|| item.get("filename"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if username.is_empty() || filename.is_empty() {
            return Err(routing::bad_request_response(
                "each item requires user and remotePath",
            ));
        }
        let filename = filename.to_owned();
        if let Some(response) =
            crate::route_dispatch::download_policy_response(state, std::slice::from_ref(&filename))
                .await
        {
            return Err(response);
        }
        let permit = match state.session_commands.reserve().await {
            Ok(permit) => permit,
            Err(_) => {
                return Err(routing::service_unavailable_response(
                    "session manager is not running",
                ))
            }
        };
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(0, Some(username.to_owned()), filename.clone(), None, None);
        let entry = transfers
            .update_status(entry.id, "peer_lookup", None, None)
            .unwrap_or(entry);
        drop(transfers);
        if let Err(error) = persist_transfer_record(state, &entry).await {
            remove_transfer_entries_if_unchanged(state, std::slice::from_ref(&entry)).await;
            return Err(routing::service_unavailable_response(&error));
        }
        permit.send(SessionCommand::TransferPeer {
            id: entry.id,
            username: username.to_owned(),
        });
        requests.push(entry);
    }
    Ok(requests)
}

pub(super) fn extended_controller_download_success_response(
    requests: &[TransferEntry],
) -> HttpResponse {
    routing::ok_response(
        serde_json::json!({
            "downloadIds": requests.iter().filter_map(|entry| entry.request_id.clone()).collect::<Vec<_>>(),
            "enqueued": requests.len(),
            "failed": 0,
            "errors": serde_json::Value::Null,
        })
        .to_string(),
    )
}

async fn collection_item_controller_response(
    method: &str,
    path: &str,
    body: &str,
    state: &AppState,
    is_versioned_v0: bool,
) -> HttpResponse {
    let Some(segments) = decoded_segments_after(path, "/api/collections/") else {
        return routing::not_found_response();
    };
    if (method == "PUT" || (method == "POST" && is_versioned_v0))
        && matches!(segments.as_slice(), [_, section, action] if section == "items" && action == "reorder")
    {
        if collection_reorder_exceeds_wire_limits(body) {
            return routing::bad_request_response("collection reorder exceeds item limits");
        }
        let collection_id = &segments[0];
        let compatibility_contract = is_versioned_v0;
        if compatibility_contract {
            let item_ids = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|value| value.get("itemIds").cloned())
                .and_then(|value| value.as_array().cloned())
                .unwrap_or_default();
            if item_ids.is_empty() {
                return routing::bad_request_response("ItemIds is required.");
            }
        }
        let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
        let mut collections = state.collections.write().await;
        let previous = collections.clone();
        let Some(record) = collections.reorder_items(collection_id, body) else {
            return routing::not_found_response();
        };
        let mutated = collections.clone();
        drop(collections);
        if let Err(error) = persist_collection_checked(state, &record).await {
            rollback_collections_if_unchanged(state, previous, &mutated).await;
            return routing::service_unavailable_response(&error);
        }
        return if compatibility_contract {
            routing::no_content_response()
        } else {
            routing::ok_response(record.json())
        };
    }
    let [collection_id, section, item_id] = segments.as_slice() else {
        return routing::not_found_response();
    };
    if section != "items" {
        return routing::not_found_response();
    }
    let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
    let mut collections = state.collections.write().await;
    if collections.get(collection_id).is_none() {
        return routing::not_found_response();
    }
    let compatibility_contract = uuid::Uuid::parse_str(collection_id).is_ok();
    let content_id = extract_json_string_field(body, "contentId");
    if compatibility_contract
        && content_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
    {
        return routing::bad_request_response("ContentId cannot be blank.");
    }
    let previous = collections.clone();
    let response = match method {
        "DELETE" => collections.remove_item(item_id).map(|item| item.json()),
        "PUT" if compatibility_contract => collections
            .update_item_contract(
                item_id,
                content_id,
                extract_json_string_field(body, "artist"),
                extract_json_string_field(body, "title"),
                extract_json_string_field(body, "mediaKind"),
                extract_json_string_field(body, "fileName"),
                extract_json_string_field(body, "album"),
                extract_json_string_field(body, "contentHash")
                    .or_else(|| extract_json_string_field(body, "sha256")),
            )
            .map(|item| item.id),
        "PUT" => collections
            .update_item(
                item_id,
                extract_json_string_field(body, "artist"),
                extract_json_string_field(body, "title"),
                extract_json_string_field(body, "kind")
                    .or_else(|| extract_json_string_field(body, "mediaKind")),
            )
            .map(|item| item.json()),
        _ => None,
    };
    let Some(response) = response else {
        return routing::not_found_response();
    };
    let Some(record) = collections.get(collection_id) else {
        return routing::not_found_response();
    };
    let response = if compatibility_contract && method == "PUT" {
        let Some((ordinal, item)) = record
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| item.id == response)
        else {
            return routing::not_found_response();
        };
        item.native_json(collection_id, ordinal)
    } else {
        response
    };
    let mutated = collections.clone();
    drop(collections);
    if let Err(error) = persist_collection_checked(state, &record).await {
        rollback_collections_if_unchanged(state, previous, &mutated).await;
        return routing::service_unavailable_response(&error);
    }
    if compatibility_contract && method == "DELETE" {
        routing::no_content_response()
    } else {
        routing::ok_response(response)
    }
}
pub(super) const LISTENING_PARTY_ANNOUNCEMENT_TTL_MS: u64 = 900_000;

pub(super) fn listening_party_event_window(
    event: &serde_json::Value,
    now_ms: u64,
) -> Option<(u64, u64, u64)> {
    let started_at = event
        .get("serverTimeUnixMs")
        .and_then(serde_json::Value::as_u64)
        .filter(|timestamp| *timestamp > 0)?;
    let last_seen = event
        .get("lastSeenUnixMs")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(started_at);
    let expires_at = event
        .get("expiresAtUnixMs")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_else(|| last_seen.saturating_add(LISTENING_PARTY_ANNOUNCEMENT_TTL_MS));
    (expires_at > now_ms).then_some((started_at, last_seen, expires_at))
}

/// Matches the oracle's real `PlaybackPriorityService.GetPriority`: High
/// when the buffer is low/empty (< 5s), Low when comfortably ahead
/// (>= 30s), Mid otherwise -- including when no feedback has ever been
/// recorded for this job at all.
pub(super) fn playback_priority_for_latest_feedback(
    latest_feedback: Option<&serde_json::Value>,
) -> &'static str {
    let Some(latest_feedback) = latest_feedback else {
        return "Mid";
    };
    let buffer_ahead_ms = latest_feedback
        .get("bufferAheadMs")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    if buffer_ahead_ms < 5_000 {
        "High"
    } else if buffer_ahead_ms >= 30_000 {
        "Low"
    } else {
        "Mid"
    }
}

/// Finds the most recently received playback-feedback entry for a job
/// among its stored history, matching the oracle's
/// `ConcurrentDictionary<string, PlaybackFeedback>` "latest wins"
/// semantics.
fn latest_playback_feedback(entries: &[serde_json::Value]) -> Option<&serde_json::Value> {
    entries.iter().max_by_key(|entry| {
        entry
            .get("receivedAt")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    })
}

async fn bridge_search_response(body: &str, state: &AppState) -> HttpResponse {
    let native_profile = state.config.controller_profile == ControllerProfile::Native;
    let native_query = if native_profile {
        let query = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| value.get("query").cloned())
            .and_then(|value| value.as_str().map(str::trim).map(ToOwned::to_owned))
            .filter(|query| !query.is_empty());
        let Some(query) = query else {
            return routing::bad_request_response("Query is required");
        };
        Some(query)
    } else {
        None
    };
    let record = match extended_controller_search_response(body, state).await {
        Ok(record) => record,
        Err(response) if native_profile && response.status == "503 Service Unavailable" => {
            return routing::ok_response(
                serde_json::json!({
                    "query": native_query.unwrap_or_default(),
                    "users": [],
                })
                .to_string(),
            );
        }
        Err(response) => return response,
    };
    let local_peer_id = pod_request_peer_id(state)
        .await
        .unwrap_or_else(|| "local".to_owned());
    let files = record
        .results
        .iter()
        .map(|entry| {
            serde_json::json!({
                "path": entry.filename,
                "sizeBytes": entry.size,
                "mbRecordingId": serde_json::Value::Null,
                "bitrateKbps": serde_json::Value::Null,
                "codec": if entry.extension.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(entry.extension)
                },
                "isCanonical": false,
            })
        })
        .collect::<Vec<_>>();
    let users = if files.is_empty() {
        Vec::new()
    } else {
        vec![serde_json::json!({
            "peerId": local_peer_id,
            "username": local_peer_id,
            "files": files,
        })]
    };
    let body = serde_json::json!({"query": record.query, "users": users}).to_string();
    if native_profile {
        routing::ok_response(body)
    } else {
        routing::created_response(body)
    }
}

/// Matches the oracle's real single-item `BridgeDownloadRequest` /
/// `{transfer_id}` contract, built from the same real download-queue
/// creation `/api/downloads` uses. The oracle's request shape is
/// already a single object (`{username, filename, targetPath}`), which
/// the shared handler already accepts directly (its "items" array is
/// optional, falling back to a single-item list) -- only the response
/// shape needed to change.
async fn bridge_download_response(body: &str, state: &AppState) -> HttpResponse {
    let native_profile = state.config.controller_profile == ControllerProfile::Native;
    if native_profile {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(payload)) => payload,
            _ => return routing::bad_request_response("Request is required"),
        };
        let required = ["username", "filename", "targetPath"];
        if required.iter().any(|field| {
            payload
                .get(*field)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .is_none_or(str::is_empty)
        }) {
            return routing::bad_request_response(
                "Username, filename, and targetPath are required",
            );
        }
    }
    match extended_controller_download_response(body, state).await {
        Ok(requests) => {
            let transfer_id = requests.first().map(|entry| {
                entry
                    .request_id
                    .clone()
                    .unwrap_or_else(|| entry.id.to_string())
            });
            routing::ok_response(serde_json::json!({"transfer_id": transfer_id}).to_string())
        }
        Err(response) if native_profile && response.status == "503 Service Unavailable" => {
            routing::internal_server_error_response("Bridge download failed")
        }
        Err(response) => response,
    }
}

fn bridge_legacy_transfer_state(status: &str) -> &'static str {
    match status {
        "queued" => "Queued",
        "peer_lookup" | "peer_negotiating" | "indirect_pending" => "Connecting",
        "accepted" | "in_progress" => "Downloading",
        "succeeded" | "completed" => "Complete",
        "failed" | "rejected" | "errored" => "Errored",
        "cancelled" => "Cancelled",
        _ => "Unknown",
    }
}

pub(super) async fn bridge_transfer_progress_response(
    path: &str,
    state: &AppState,
) -> HttpResponse {
    let Some(raw_transfer_id) = path_segment_between(path, "/api/bridge/transfer/", "/progress")
    else {
        return routing::not_found_response();
    };
    let transfer_id = decoded_path_segment(raw_transfer_id).trim().to_owned();
    if state.config.controller_profile == ControllerProfile::Native {
        if transfer_id.is_empty() {
            return routing::bad_request_response("TransferId is required");
        }
        let transfers = state.transfers.read().await;
        let transfer = transfers.entries.iter().find(|entry| {
            entry.id.to_string() == transfer_id
                || entry
                    .request_id
                    .as_deref()
                    .is_some_and(|request_id| request_id.eq_ignore_ascii_case(&transfer_id))
        });
        let Some(transfer) = transfer else {
            return HttpResponse {
                status: "404 Not Found",
                content_type: "application/json",
                body: serde_json::json!({"error": "Transfer not found"}).to_string(),
            };
        };
        let size = transfer.size.unwrap_or(0);
        let percent_complete = if size == 0 {
            0
        } else {
            transfer
                .bytes_transferred
                .saturating_mul(100)
                .checked_div(size)
                .unwrap_or(0)
                .min(100)
        };
        return routing::ok_response(
            serde_json::json!({
                "proxyId": transfer_id,
                "username": transfer.peer_username.as_deref().unwrap_or_default(),
                "filename": transfer.filename,
                "bytesTransferred": transfer.bytes_transferred,
                "fileSize": size,
                "percentComplete": percent_complete,
                "averageSpeed": transfer.average_speed_at(unix_timestamp()),
                "state": bridge_legacy_transfer_state(&transfer.status),
                "queuePosition": 0,
            })
            .to_string(),
        );
    }

    let id = transfer_id.parse::<u64>().ok();
    let transfers = state.transfers.read().await;
    let json = id
        .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id))
        .map(|entry| {
            let size = entry.size.unwrap_or(0);
            let progress = if size == 0 {
                0.0
            } else {
                (entry.bytes_transferred as f64 / size as f64) * 100.0
            };
            serde_json::json!({
                "transfer_id": transfer_id,
                "id": entry.id,
                "progress": progress,
                "status": entry.status,
                "state": controller_transfer_state(&entry.status),
                "filename": entry.filename,
                "username": entry.peer_username,
                "bytesTransferred": entry.bytes_transferred,
                "size": size,
                "updated_at": entry.updated_at,
            })
            .to_string()
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "transfer_id": transfer_id,
                "progress": 0.0,
                "status": "pending",
                "state": "None",
                "bytesTransferred": 0,
                "size": 0,
            })
            .to_string()
        });
    drop(transfers);
    routing::ok_response(json)
}

async fn realm_subject_dynamic_get_response(path: &str, state: &AppState) -> HttpResponse {
    let Some(segments) = decoded_segments_after(path, "/api/realm-subject-indexes/") else {
        return routing::not_found_response();
    };
    let discovery = state.content_discovery.read().await;
    match segments.as_slice() {
        [realm_id] => routing::ok_response(
            serde_json::Value::Array(
                state
                    .realm_subject_indexes
                    .read()
                    .await
                    .indexes_for_realm(realm_id),
            )
            .to_string(),
        ),
        [realm_id, tail] if tail == "authority-decisions" => {
            let decisions = state
                .realm_subject_indexes
                .read()
                .await
                .authority_decisions_for_realm(realm_id);
            routing::ok_response(serde_json::Value::Array(decisions).to_string())
        }
        [recordings, recording_id, resolutions]
            if recordings == "recordings" && resolutions == "resolutions" =>
        {
            let indexed_resolutions = state
                .realm_subject_indexes
                .read()
                .await
                .resolve_recording(recording_id);
            if !indexed_resolutions.is_empty() {
                return routing::ok_response(
                    serde_json::Value::Array(indexed_resolutions).to_string(),
                );
            }
            let peers = discovery.peer_ids_for_recordings(std::slice::from_ref(recording_id));
            routing::ok_response(serde_json::Value::Array(
                peers.into_iter().map(|peer_id| serde_json::json!({"recordingId": recording_id, "peerId": peer_id})).collect(),
            ).to_string())
        }
        _ => routing::not_found_response(),
    }
}

/// Matches the oracle's `PrometheusMetric` JSON shape (a single-sample
/// gauge, the only kind slskr currently emits).
pub(super) fn prometheus_metric_json(
    name: &str,
    metric_type: &str,
    value: f64,
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "help": "",
        "type": metric_type,
        "sum": null,
        "count": null,
        "samples": [{"value": value, "labels": {}}],
        "buckets": {},
        "quantiles": {},
    })
}

pub(super) fn virtual_soulfind_disaster_mode_level() -> u8 {
    // The frozen coordinator starts at Normal and does not consume the
    // configuration's test-only Force flag when reporting status. Runtime
    // health transitions can replace this with a coordinator-backed level.
    0
}
