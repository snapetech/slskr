use super::*;

pub(super) fn controller_transfer_user_path<'a>(path: &'a str, direction: &str) -> Option<&'a str> {
    let prefix = format!("/api/transfers/{direction}/");
    path.strip_prefix(&prefix)
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub(super) fn controller_transfer_file_path<'a>(
    path: &'a str,
    direction: &str,
) -> Option<(&'a str, u64)> {
    let prefix = format!("/api/transfers/{direction}/");
    let rest = path.strip_prefix(&prefix)?;
    let (username, tail) = rest.split_once('/')?;
    let id = tail.split_once('/').map_or(
        tail,
        |(id, suffix)| {
            if suffix == "position" {
                id
            } else {
                ""
            }
        },
    );
    if username.is_empty() || username.contains('/') || id.is_empty() {
        return None;
    }
    Some((username, id.parse().ok()?))
}

pub(super) fn controller_transfer_position_path(path: &str) -> Option<(&str, u64)> {
    controller_transfer_file_path(path, "downloads").filter(|_| path.ends_with("/position"))
}

pub(super) fn transfer_resource_segment(path: &str) -> Option<&str> {
    path_segment_after(path, "/api/transfers/")
        .or_else(|| path_segment_after(path, "/api/v0/transfers/"))
}

pub(super) fn controller_file_storage_resource_path(path: &str) -> Option<(&str, &str, &str)> {
    let mut segments = path
        .strip_prefix("/api/files/")
        .or_else(|| path.strip_prefix("/api/v0/files/"))?
        .split('/');
    let storage = segments.next()?;
    let resource = segments.next()?;
    let encoded_name = segments.next()?;
    if segments.next().is_some()
        || !matches!(storage, "downloads" | "incomplete")
        || !matches!(resource, "directories" | "files")
        || encoded_name.is_empty()
    {
        return None;
    }
    Some((storage, resource, encoded_name))
}

pub(super) fn file_storage_error_response(error: &str) -> HttpResponse {
    if error == STORAGE_DIRECTORY_NOT_FOUND_ERROR {
        routing::not_found_response()
    } else if matches!(
        error,
        STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR
            | STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR
            | STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR
            | STORAGE_DIRECTORY_DELETE_DEPTH_ERROR
    ) {
        HttpResponse {
            status: "413 Payload Too Large",
            content_type: "application/json",
            body: if error == STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR {
                "{\"error\":\"storage directory is too large to list\"}".to_owned()
            } else if matches!(
                error,
                STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR
                    | STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR
            ) {
                "{\"error\":\"storage directory is too large to delete\"}".to_owned()
            } else {
                "{\"error\":\"storage directory tree is too deep to delete\"}".to_owned()
            },
        }
    } else if error.contains("failed:") {
        eprintln!("file storage operation failed: {error}");
        routing::service_unavailable_response("file storage unavailable")
    } else {
        routing::bad_request_response(error)
    }
}

pub(super) fn controller_file_array_exceeds_wire_limits(body: &str) -> bool {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    payload
        .get("files")
        .and_then(serde_json::Value::as_array)
        .or_else(|| payload.as_array())
        .is_some_and(|files| files.len() > MAX_TRANSFER_REQUEST_FILES)
}

pub(super) fn controller_files_from_body(body: &str) -> Vec<serde_json::Value> {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    if let Some(files) = payload.get("files").and_then(serde_json::Value::as_array) {
        return files.clone();
    }
    if payload.get("filename").is_some() {
        return vec![payload];
    }
    payload.as_array().cloned().unwrap_or_default()
}

pub(super) fn controller_transfer_batch_id(body: &str) -> Option<String> {
    let payload = serde_json::from_str::<serde_json::Value>(body).ok()?;
    ["batchId", "batch_id", "id"]
        .iter()
        .find_map(|field| payload.get(field).and_then(serde_json::Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty() && uuid::Uuid::parse_str(value).is_ok())
        .map(str::to_owned)
}

pub(super) fn transfer_batch_record_key(batch_id: &str) -> String {
    format!("slskd/transfer-batch/{batch_id}")
}

pub(super) fn persisted_transfer_batch_record(
    batch: &serde_json::Value,
) -> Result<persistence::TransferBatchRecord, String> {
    let id = batch
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "transfer batch is missing an id".to_owned())?;
    let username = batch
        .get("username")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_owned();
    let direction = match batch
        .get("direction")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Download")
    {
        "Upload" | "upload" => 1,
        _ => 0,
    };
    let search_id = batch
        .get("searchId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let created_at = batch
        .get("createdAt")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "transfer batch is missing createdAt".to_owned())?;
    let options_json = batch
        .get("options")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !options_json.is_object() {
        return Err("transfer batch options must be an object".to_owned());
    }
    Ok(persistence::TransferBatchRecord {
        id: id.to_owned(),
        search_id,
        username,
        direction,
        created_at,
        options_json: Some(options_json.to_string()),
    })
}

pub(super) fn transfer_batch_json_from_record(
    record: persistence::TransferBatchRecord,
) -> Result<serde_json::Value, String> {
    let options = record
        .options_json
        .as_deref()
        .map(serde_json::from_str::<serde_json::Value>)
        .transpose()
        .map_err(|error| format!("transfer batch options could not be decoded: {error}"))?
        .unwrap_or_else(|| serde_json::json!({}));
    if !options.is_object() {
        return Err("transfer batch options must be an object".to_owned());
    }
    let mut batch = serde_json::json!({
        "id": record.id,
        "username": record.username,
        "direction": if record.direction == 1 { "Upload" } else { "Download" },
        "createdAt": record.created_at,
        "transfers": [],
        "options": options,
    });
    if let Some(search_id) = record.search_id {
        batch["searchId"] = serde_json::Value::String(search_id);
    }
    Ok(batch)
}

pub(super) async fn controller_read_transfer_batch(
    state: &AppState,
    batch_id: &str,
) -> Result<Option<serde_json::Value>, String> {
    if let Some(db) = state.db.as_ref() {
        let record = db
            .get_transfer_batch(batch_id)
            .await
            .map_err(|error| format!("transfer batch storage unavailable: {error}"))?;
        return record.map(transfer_batch_json_from_record).transpose();
    }
    Ok(state
        .controller_features
        .read()
        .await
        .get(&transfer_batch_record_key(batch_id))
        .cloned())
}

pub(super) async fn controller_create_transfer_batch(
    state: &AppState,
    batch: &serde_json::Value,
) -> Result<(), String> {
    if let Some(db) = state.db.as_ref() {
        let record = persisted_transfer_batch_record(batch)?;
        return db
            .insert_transfer_batch(&record)
            .await
            .map_err(|error| format!("transfer batch storage unavailable: {error}"));
    }
    state
        .controller_features
        .upsert(
            transfer_batch_record_key(
                batch
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
            ),
            batch.clone(),
        )
        .await
}

pub(super) async fn controller_delete_transfer_batch(
    state: &AppState,
    batch_id: &str,
) -> Result<(), String> {
    if let Some(db) = state.db.as_ref() {
        db.delete_transfer_batch(batch_id)
            .await
            .map_err(|error| format!("transfer batch storage unavailable: {error}"))?;
        return Ok(());
    }
    state
        .controller_features
        .remove(&transfer_batch_record_key(batch_id))
        .await
        .map(|_| ())
}

pub(super) fn controller_transfer_batch_insert_is_duplicate(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("unique constraint")
        || error.contains("constraint failed")
        || error.contains("primary key")
}

pub(super) fn transfer_request_details_from_json(
    value: &serde_json::Value,
    filename: &str,
) -> TransferRequestDetails {
    let text = |names: &[&str], max_bytes: usize| {
        names
            .iter()
            .find_map(|name| value.get(name).and_then(serde_json::Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_utf8_bytes(value.to_owned(), max_bytes))
    };
    let number = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| value.get(name).and_then(serde_json::Value::as_u64))
            .and_then(|value| u32::try_from(value).ok())
    };
    let request_id = text(&["requestId", "request_id"], MAX_TRANSFER_REQUEST_ID_BYTES)
        .filter(|value| uuid::Uuid::parse_str(value).is_ok())
        .or_else(|| Some(uuid::Uuid::new_v4().to_string()));
    TransferRequestDetails {
        request_id,
        wishlist_item_id: text(
            &["wishlistItemId", "wishlist_item_id"],
            MAX_TRANSFER_REQUEST_ID_BYTES,
        ),
        request_name: text(
            &["requestName", "request_name", "name"],
            MAX_TRANSFER_REQUEST_NAME_BYTES,
        )
        .or_else(|| Some(virtual_basename(filename).to_owned())),
        destination_directory: text(
            &[
                "destinationDirectory",
                "destination_directory",
                "destination",
            ],
            MAX_TRANSFER_LOCAL_PATH_BYTES,
        ),
        bit_rate: number(&["bitRate", "bit_rate", "bitrate"]),
        sample_rate: number(&["sampleRate", "sample_rate", "samplerate"]),
        bit_depth: number(&["bitDepth", "bit_depth", "bitdepth"]),
        length_seconds: number(&["length", "lengthSeconds", "length_seconds"]),
        artist: text(&["artist"], MAX_TRANSFER_METADATA_TEXT_BYTES),
        album: text(&["album"], MAX_TRANSFER_METADATA_TEXT_BYTES),
        title: text(&["title"], MAX_TRANSFER_METADATA_TEXT_BYTES),
        track_number: number(&["trackNumber", "track_number"]),
        year: number(&["year"]),
        attempts: number(&["attempts"]).unwrap_or(0),
        auto_replace_attempts: number(&["autoReplaceAttempts", "auto_replace_attempts"])
            .unwrap_or(0),
        next_attempt_at: value
            .get("nextAttemptAt")
            .or_else(|| value.get("next_attempt_at"))
            .and_then(serde_json::Value::as_u64),
    }
}

pub(super) fn controller_enqueue_request(body: &str) -> Option<(String, Vec<serde_json::Value>)> {
    let payload = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let username = payload.get("username")?.as_str()?.to_owned();
    let files = payload
        .get("files")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    (!files.is_empty()).then_some((username, files))
}

#[derive(Debug)]
pub(super) enum BatchTransferPreparationError {
    Duplicate,
    OutboundDisabled,
    DispatchUnavailable,
    Path(String),
}

pub(super) struct PreparedBatchTransfer {
    pub(super) index: usize,
    pub(super) filename: String,
    pub(super) size: u64,
    pub(super) local_path: Option<String>,
    pub(super) details: TransferRequestDetails,
    pub(super) permit: mpsc::OwnedPermit<SessionCommand>,
}

pub(super) fn has_active_download(
    transfers: &TransferQueue,
    username: &str,
    filename: &str,
) -> bool {
    transfers.entries.iter().any(|entry| {
        entry.direction == 0
            && entry
                .peer_username
                .as_deref()
                .is_some_and(|peer| peer.eq_ignore_ascii_case(username))
            && entry.filename == filename
            && !is_terminal_transfer_status(&entry.status)
    })
}

pub(super) async fn prepare_download_batch_transfer(
    state: &AppState,
    index: usize,
    username: &str,
    filename: &str,
    size: u64,
    batch_id: &str,
    destination: Option<&str>,
) -> Result<PreparedBatchTransfer, BatchTransferPreparationError> {
    let duplicate = {
        let transfers = state.transfers.read().await;
        has_active_download(&transfers, username, filename)
    };
    if duplicate {
        return Err(BatchTransferPreparationError::Duplicate);
    }
    if !state.config.transfer_allow_outbound {
        return Err(BatchTransferPreparationError::OutboundDisabled);
    }
    let permit = state
        .session_commands
        .clone()
        .try_reserve_owned()
        .map_err(|_| BatchTransferPreparationError::DispatchUnavailable)?;
    let details = TransferRequestDetails {
        destination_directory: destination.map(str::to_owned),
        request_name: Some(virtual_basename(filename).to_owned()),
        ..TransferRequestDetails::default()
    };
    let relative = if let Some(destination) = destination {
        format!(
            "{}/{}",
            destination.trim_matches(['/', '\\']),
            virtual_basename(filename)
        )
    } else {
        render_configured_completed_download_path(
            state,
            username,
            filename,
            Some(batch_id),
            details.request_name.as_deref(),
            unix_timestamp(),
        )
        .await
        .map_err(BatchTransferPreparationError::Path)?
    };
    let local_path = configured_download_destination_path(state, &relative)
        .await
        .map(|path| path.display().to_string())
        .map_err(BatchTransferPreparationError::Path)?;
    Ok(PreparedBatchTransfer {
        index,
        filename: filename.to_owned(),
        size,
        local_path: Some(local_path),
        details,
        permit,
    })
}

pub(super) async fn commit_prepared_download_batch_transfers(
    state: &AppState,
    username: &str,
    batch_id: &str,
    prepared: Vec<PreparedBatchTransfer>,
) -> (
    Vec<(TransferEntry, mpsc::OwnedPermit<SessionCommand>)>,
    Vec<(usize, serde_json::Value)>,
) {
    let mut staged = Vec::with_capacity(prepared.len());
    let mut failures = Vec::new();
    {
        let mut transfers = state.transfers.write().await;
        for transfer in prepared {
            if has_active_download(&transfers, username, &transfer.filename) {
                failures.push((
                    transfer.index,
                    serde_json::json!({
                        "filename": transfer.filename,
                        "message": "Transfer is already queued or in progress",
                    }),
                ));
                continue;
            }
            let entry = transfers.create_with_details(
                0,
                Some(username.to_owned()),
                transfer.filename,
                transfer.local_path,
                Some(transfer.size),
                Some(batch_id.to_owned()),
                transfer.details,
            );
            let entry = transfers
                .update_status(entry.id, "peer_lookup", None, None)
                .unwrap_or(entry);
            staged.push((entry, transfer.permit));
        }
    }
    (staged, failures)
}
