use super::*;

pub(super) async fn controller_telemetry_report_read_failure_response(
    state: &AppState,
    path: &str,
) -> Option<HttpResponse> {
    if !matches!(
        state.config.controller_profile,
        ControllerProfile::Legacy | ControllerProfile::Native
    ) || !path.starts_with("/api/v0/telemetry/reports/transfers/")
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_transfers(None, 1, 0).await.is_err() {
        return Some(routing::internal_server_error_response(
            "transfer storage unavailable",
        ));
    }
    None
}

pub(super) async fn controller_transfer_storage_read_failure_response(
    state: &AppState,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Legacy
        || !path.starts_with("/api/v0/transfers/")
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_transfers(None, 1, 0).await.is_err() {
        return Some(routing::internal_server_error_response(
            "transfer storage unavailable",
        ));
    }
    None
}

pub(super) async fn controller_native_transfer_storage_failure_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    let download_route = path == "/api/v0/downloads"
        || path.starts_with("/api/v0/downloads/")
        || path == "/api/downloads"
        || path.starts_with("/api/downloads/");
    if state.config.controller_profile != ControllerProfile::Native
        || !(path == "/api/v0/transfers"
            || path.starts_with("/api/v0/transfers/")
            || download_route)
        // Accelerated download mode is process-local in native profile and does not
        // use the transfer database.  Keep its GET/PUT contract available
        // while the persistence store is unavailable.
        || matches!(
            (method, path),
            ("GET", "/api/v0/transfers/downloads/accelerated")
                | ("PUT", "/api/v0/transfers/downloads/accelerated")
        )
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_transfers(None, 1, 0).await.is_err() {
        return Some(routing::internal_server_error_response(
            "transfer storage unavailable",
        ));
    }
    None
}

pub(super) fn controller_native_transfer_input_validation_response(
    state: &AppState,
    method: &str,
    path: &str,
    query: Option<&str>,
    body: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native
        || !(path == "/api/v0/transfers" || path.starts_with("/api/v0/transfers/"))
    {
        return None;
    }

    if method == "DELETE" {
        for prefix in ["/api/v0/transfers/downloads/", "/api/v0/transfers/uploads/"] {
            if let Some(value) = path.strip_prefix(prefix) {
                let segments = value.split('/').collect::<Vec<_>>();
                if segments.len() == 2
                    && segments[0] != "all"
                    && segments[1].parse::<u64>().is_err()
                {
                    return Some(routing::bad_request_response("The request is invalid"));
                }
            }
        }
    }

    if method == "GET" {
        if path == "/api/v0/transfers" {
            if let Some(direction) = query_parameter(query, "direction") {
                if !direction.trim().is_empty()
                    && !matches!(
                        direction.trim().to_ascii_lowercase().as_str(),
                        "download" | "upload"
                    )
                {
                    return Some(routing::bad_request_response(
                        "direction must be 'download' or 'upload' when provided",
                    ));
                }
            }
        }
        if matches!(
            path,
            "/api/v0/transfers/downloads" | "/api/v0/transfers/uploads"
        ) {
            for parameter in ["includeCompleted", "includeRemoved"] {
                if let Some(value) = query_parameter(query, parameter) {
                    if parse_bool_value(&value).is_none() {
                        return Some(routing::bad_request_response(&format!(
                            "{parameter} must be a boolean"
                        )));
                    }
                }
            }
        }
    }

    if method == "POST"
        && matches!(
            path,
            "/api/v0/transfers/downloads/auto-replace"
                | "/api/v0/transfers/downloads/find-alternative"
                | "/api/v0/transfers/downloads/replace"
        )
    {
        let valid_object = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .is_some_and(|value| value.is_object());
        if !valid_object {
            return Some(routing::bad_request_response("The request is invalid"));
        }
    }

    if method == "PUT" && path == "/api/v0/transfers/downloads/accelerated" {
        let valid_payload = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .is_some_and(|value| {
                value.is_object()
                    && value
                        .get("enabled")
                        .is_none_or(serde_json::Value::is_boolean)
            });
        if !valid_payload {
            return Some(routing::bad_request_response("The request is invalid"));
        }
    }

    None
}

pub(super) async fn controller_native_transfer_auto_replace_status_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native
        || method != "GET"
        || path != "/api/v0/transfers/downloads/auto-replace/status"
    {
        return None;
    }

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
    Some(routing::ok_response(
        serde_json::json!({
            "stuckCount": stuck_count,
            "enabled": enabled,
            "intervalSeconds": 300,
        })
        .to_string(),
    ))
}

pub(super) async fn controller_native_autoreplace_mutation_response(
    state: &AppState,
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    if state.config.controller_profile != ControllerProfile::Native || method != "PUT" {
        return None;
    }
    let enabled = match path {
        "/api/v0/autoreplace/enable" => true,
        "/api/v0/autoreplace/disable" => false,
        _ => return None,
    };

    let _runtime_persistence = state.runtime_persistence_lock.lock().await;
    let (body, previous_runtime, mutated_runtime, persisted) = {
        let mut runtime = state.runtime.write().await;
        let previous_runtime = runtime.clone();
        runtime.set_autoreplace(enabled);
        let mutated_runtime = runtime.clone();
        let relay = state.relay.read().await;
        (
            serde_json::json!({
                "enabled": enabled,
                "lastRunAt": null,
                "lastRunProcessedCount": 0,
                "lastRunReplacedCount": 0,
                "intervalSeconds": 300,
            })
            .to_string(),
            previous_runtime,
            mutated_runtime,
            runtime.persistence_record(&relay),
        )
    };

    // Frozen native profile saves this toggle in a local state file and deliberately
    // does not make controller availability depend on the transfer database.
    // Keep the same response when the optional SQLite mirror is unavailable.
    let mut database_persisted = false;
    if let Some(db) = state.db.as_ref() {
        let database_result = db
            .upsert_runtime_compat_state(&persisted)
            .await
            .map_err(|error| error.to_string());
        match database_result {
            Ok(()) => database_persisted = true,
            Err(error) => {
                record_daemon_log(
                    state,
                    logging::LogLevel::Warn,
                    "autoreplace",
                    format!("runtime compatibility mirror unavailable: {error}"),
                )
                .await;
            }
        }
    }
    let state_file = state.config.state_dir.join("auto-replace-state.json");
    let state_body = serde_json::json!({
        "Enabled": enabled,
        "UserConfigured": true,
    })
    .to_string();
    if let Err(error) = write_file_atomic(&state_file, state_body.as_bytes()) {
        let rolled_back = {
            let mut runtime = state.runtime.write().await;
            if *runtime == mutated_runtime {
                *runtime = previous_runtime.clone();
                true
            } else {
                false
            }
        };
        if database_persisted && rolled_back {
            let relay = state.relay.read().await;
            let previous_persisted = previous_runtime.persistence_record(&relay);
            if let Some(db) = state.db.as_ref() {
                let rollback_result = db
                    .upsert_runtime_compat_state(&previous_persisted)
                    .await
                    .map_err(|error| error.to_string());
                if let Err(rollback_error) = rollback_result {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Error,
                        "autoreplace",
                        format!(
                            "failed to roll back runtime compatibility mirror after state-file failure: {rollback_error}"
                        ),
                    )
                    .await;
                }
            }
        }
        record_daemon_log(
            state,
            logging::LogLevel::Error,
            "autoreplace",
            format!(
                "auto-replace state persistence failed: {error}; runtime rolled back={rolled_back}"
            ),
        )
        .await;
        return Some(routing::service_unavailable_response(
            "auto-replace state persistence failed",
        ));
    }

    Some(routing::ok_response(body))
}

pub(super) fn controller_transfer_summary_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> String {
    let direction = controller_transfer_query_direction(query);
    let username = controller_transfer_query_username(query);
    let entries = transfers
        .entries
        .iter()
        .filter(|entry| controller_transfer_matches_query(entry, direction, username.as_deref()))
        .collect::<Vec<_>>();
    let downloads = entries.iter().filter(|entry| entry.direction == 0).count();
    let uploads = entries.iter().filter(|entry| entry.direction == 1).count();
    let total_bytes = entries
        .iter()
        .map(|entry| entry.bytes_transferred)
        .sum::<u64>();
    let average_speed = average_transfer_speed_at(&entries, unix_timestamp());
    let mut by_state = BTreeMap::new();
    for entry in &entries {
        let state = controller_transfer_state(&entry.status).to_owned();
        let count = by_state.entry(state).or_insert(0usize);
        *count += 1;
    }
    let by_state = by_state
        .into_iter()
        .map(|(state, count)| (state, serde_json::json!({ "count": count })))
        .collect::<BTreeMap<_, _>>();
    serde_json::json!({
        "Download": {},
        "Upload": {},
        "count": entries.len(),
        "downloads": downloads,
        "uploads": uploads,
        "totalBytes": total_bytes,
        "averageSpeed": average_speed,
        "byDirection": {
            "Download": { "count": downloads },
            "Upload": { "count": uploads },
        },
        "byState": by_state,
    })
    .to_string()
}

pub(super) fn controller_versioned_transfer_summary_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> String {
    let direction = controller_transfer_query_direction(query);
    let username = controller_transfer_query_username(query);
    let start = controller_transfer_query_unix_timestamp(query, "start")
        .or_else(|| controller_transfer_query_unix_timestamp(query, "startDate"))
        .unwrap_or(0);
    let end = controller_transfer_query_unix_timestamp(query, "end")
        .or_else(|| controller_transfer_query_unix_timestamp(query, "endDate"))
        .unwrap_or(u64::MAX);

    let mut grouped = BTreeMap::<(u32, String), Vec<&TransferEntry>>::new();
    for entry in transfers.entries.iter().filter(|entry| {
        is_terminal_transfer_status(&entry.status)
            && entry.updated_at >= start
            && entry.updated_at <= end
            && controller_transfer_matches_query(entry, direction, username.as_deref())
    }) {
        grouped
            .entry((
                entry.direction,
                controller_transfer_summary_state(&entry.status).to_owned(),
            ))
            .or_default()
            .push(entry);
    }

    let mut response = serde_json::json!({
        "Download": {},
        "Upload": {},
    });
    for ((direction, state), entries) in grouped {
        let summary = controller_versioned_transfer_summary_value(&entries);
        let direction_name = if direction == 0 { "Download" } else { "Upload" };
        response[direction_name][state] = summary;
    }
    response.to_string()
}

fn controller_versioned_transfer_summary_value(entries: &[&TransferEntry]) -> serde_json::Value {
    let usernames = entries
        .iter()
        .map(|entry| entry.peer_username.as_deref().unwrap_or_default())
        .collect::<HashSet<_>>();
    let count = entries.len() as u64;
    let average_speed = entries
        .iter()
        .map(|entry| entry.average_speed_at(entry.updated_at))
        .sum::<f64>()
        / count.max(1) as f64;
    let average_wait = entries
        .iter()
        .filter_map(|entry| {
            entry
                .started_at
                .map(|started_at| started_at.saturating_sub(entry.requested_at) as f64)
        })
        .sum::<f64>()
        / count.max(1) as f64;
    let average_duration = entries
        .iter()
        .filter_map(|entry| {
            entry
                .started_at
                .map(|started_at| entry.updated_at.saturating_sub(started_at) as f64)
        })
        .sum::<f64>()
        / count.max(1) as f64;
    serde_json::json!({
        "username": "",
        "totalBytes": entries
            .iter()
            .map(|entry| entry.size.unwrap_or(0))
            .sum::<u64>(),
        "count": count,
        "distinctUsers": usernames.len() as u64,
        "averageSpeed": average_speed,
        "averageWait": average_wait,
        "averageDuration": average_duration,
    })
}

fn controller_transfer_summary_state(status: &str) -> &'static str {
    match status {
        "succeeded" | "completed" => "Succeeded",
        "cancelled" => "Cancelled",
        "rejected" => "Rejected",
        "timed_out" | "timeout" => "TimedOut",
        "aborted" => "Aborted",
        "failed" | "errored" => "Errored",
        _ => "Errored",
    }
}

fn controller_transfer_query_unix_timestamp(query: Option<&str>, key: &str) -> Option<u64> {
    let value = controller_transfer_query_value(query, key)?;
    value.parse::<u64>().ok().or_else(|| {
        chrono::DateTime::parse_from_rfc3339(&value)
            .ok()
            .and_then(|timestamp| u64::try_from(timestamp.timestamp()).ok())
    })
}

fn controller_transfer_query_direction(query: Option<&str>) -> Option<u32> {
    query_params(query.unwrap_or_default())
        .into_iter()
        .find_map(|(name, value)| {
            (name == "direction").then(|| match value.to_ascii_lowercase().as_str() {
                "download" | "0" => Some(0),
                "upload" | "1" => Some(1),
                _ => None,
            })?
        })
}

fn controller_transfer_query_username(query: Option<&str>) -> Option<String> {
    query_params(query.unwrap_or_default())
        .into_iter()
        .find_map(|(name, value)| (name == "username" && !value.is_empty()).then_some(value))
}

fn controller_transfer_query_limit_offset(query: Option<&str>) -> (usize, usize) {
    let mut limit = DEFAULT_LIST_LIMIT;
    let mut offset = 0usize;
    for (name, value) in query_params(query.unwrap_or_default()) {
        match name.as_str() {
            "limit" => limit = parse_list_limit(&value),
            "offset" => offset = value.parse::<usize>().unwrap_or(0),
            _ => {}
        }
    }
    (limit, offset)
}

fn controller_transfer_query_value(query: Option<&str>, key: &str) -> Option<String> {
    query_params(query.unwrap_or_default())
        .into_iter()
        .find_map(|(name, value)| {
            (name.eq_ignore_ascii_case(key) && !value.is_empty()).then_some(value)
        })
}

/// Matches the oracle's real ReportsController contract for
/// leaderboard/exceptions/exceptions-pareto: `direction` is required,
/// not optional -- a missing direction previously meant "no filter",
/// silently mixing Upload and Download rows into a single report
/// instead of rejecting the request the way the oracle does.
fn controller_transfer_query_required_direction(query: Option<&str>) -> Result<u32, &'static str> {
    match controller_transfer_query_value(query, "direction") {
        None => Err("Direction is required"),
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "download" | "0" => Ok(0),
            "upload" | "1" => Ok(1),
            _ => Err("Invalid direction"),
        },
    }
}

fn controller_transfer_matches_query(
    entry: &TransferEntry,
    direction: Option<u32>,
    username: Option<&str>,
) -> bool {
    direction.is_none_or(|direction| entry.direction == direction)
        && username.is_none_or(|username| entry.peer_username.as_deref() == Some(username))
}

pub(super) fn controller_transfer_speeds_json(transfers: &TransferQueue) -> String {
    let now = unix_timestamp();
    let active_downloads = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0 && is_active_transfer_status(&entry.status))
        .count();
    let active_uploads = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .count();
    let download_bytes = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0)
        .map(|entry| entry.bytes_transferred)
        .sum::<u64>();
    let upload_bytes = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1)
        .map(|entry| entry.bytes_transferred)
        .sum::<u64>();
    let download_speed = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0 && is_active_transfer_status(&entry.status))
        .map(|entry| entry.average_speed_at(now))
        .sum::<f64>();
    let upload_speed = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .map(|entry| entry.average_speed_at(now))
        .sum::<f64>();
    let total_speed = download_speed + upload_speed;
    let active_transfers = active_downloads + active_uploads;
    let average_speed = if active_transfers == 0 {
        0.0
    } else {
        total_speed / active_transfers as f64
    };
    serde_json::json!({
        "active_transfers": active_transfers,
        "activeDownloads": active_downloads,
        "activeUploads": active_uploads,
        "total_bytes_transferred": download_bytes.saturating_add(upload_bytes),
        "downloadBytesTransferred": download_bytes,
        "uploadBytesTransferred": upload_bytes,
        "downloadSpeed": download_speed,
        "uploadSpeed": upload_speed,
        "average_speed": average_speed,
        "total": total_speed,
        "soulseek": total_speed,
        "mesh": 0.0,
        "download": download_speed,
        "upload": upload_speed,
        "sessionBytesDownloaded": download_bytes,
        "sessionBytesUploaded": upload_bytes,
        "sessionBytesTotal": download_bytes.saturating_add(upload_bytes),
    })
    .to_string()
}

fn average_transfer_speed_at(entries: &[&TransferEntry], now: u64) -> f64 {
    if entries.is_empty() {
        return 0.0;
    }
    entries
        .iter()
        .map(|entry| entry.average_speed_at(now))
        .sum::<f64>()
        / entries.len() as f64
}

pub(super) fn controller_download_stats_json(transfers: &TransferQueue) -> String {
    let downloads = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0)
        .collect::<Vec<_>>();
    let queued = downloads
        .iter()
        .filter(|entry| entry.status == "queued")
        .count();
    let active = downloads
        .iter()
        .filter(|entry| is_active_transfer_status(&entry.status))
        .count();
    let completed = downloads
        .iter()
        .filter(|entry| is_successful_transfer_status(&entry.status))
        .count();
    let failed = downloads
        .iter()
        .filter(|entry| is_failed_transfer_status(&entry.status))
        .count();
    let cancelled = downloads
        .iter()
        .filter(|entry| entry.status == "cancelled")
        .count();
    let total_size = downloads
        .iter()
        .map(|entry| entry.size.unwrap_or(0))
        .sum::<u64>();
    let bytes_transferred = downloads
        .iter()
        .map(|entry| entry.bytes_transferred)
        .sum::<u64>();
    serde_json::json!({
        "total_downloads": downloads.len(),
        "totalDownloads": downloads.len(),
        "queued": queued,
        "active": active,
        "completed": completed,
        "failed": failed,
        "cancelled": cancelled,
        "total_size": total_size,
        "totalSize": total_size,
        "bytes_transferred": bytes_transferred,
        "bytesTransferred": bytes_transferred,
    })
    .to_string()
}

pub(super) fn controller_accelerated_downloads_json(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> String {
    let username = controller_transfer_query_username(query);
    let (limit, offset) = controller_transfer_query_limit_offset(query);
    let mut candidates = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0)
        .filter(|entry| controller_transfer_matches_query(entry, None, username.as_deref()))
        .filter(|entry| {
            matches!(
                entry.status.as_str(),
                "queued" | "peer_lookup" | "peer_negotiating" | "indirect_pending" | "in_progress"
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.requested_at
            .cmp(&right.requested_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    let count = candidates.len();
    let accelerated = candidates
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|entry| {
            serde_json::json!({
                "id": entry.id.to_string(),
                "username": entry.peer_username.as_deref().unwrap_or_default(),
                "filename": entry.filename,
                "state": controller_transfer_state(&entry.status),
                "bytesTransferred": entry.bytes_transferred,
                "size": entry.size.unwrap_or(0),
                "position": transfers.controller_transfer_position(0, entry.peer_username.as_deref().unwrap_or_default(), entry.id),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "enabled": false,
        "available": count > 0,
        "accelerated": accelerated,
        "count": count,
        "offset": offset,
        "limit": limit,
    })
    .to_string()
}

pub(super) fn controller_stuck_downloads_json(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> String {
    let username = controller_transfer_query_username(query);
    let (limit, offset) = controller_transfer_query_limit_offset(query);
    let mut entries = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0)
        .filter(|entry| controller_transfer_matches_query(entry, None, username.as_deref()))
        .filter(|entry| {
            is_failed_transfer_status(&entry.status)
                || (is_active_transfer_status(&entry.status) && entry.bytes_transferred == 0)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.filename.cmp(&right.filename))
            .then_with(|| left.id.cmp(&right.id))
    });
    let count = entries.len();
    let stuck = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|entry| {
            serde_json::json!({
                "id": entry.id.to_string(),
                "username": entry.peer_username.as_deref().unwrap_or_default(),
                "filename": entry.filename,
                "state": controller_transfer_state(&entry.status),
                "bytesTransferred": entry.bytes_transferred,
                "size": entry.size.unwrap_or(0),
                "reason": public_transfer_reason(&entry.status, entry.reason.as_deref()).unwrap_or(""),
                "updatedAt": entry.updated_at.to_string(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "stuck": stuck,
        "count": count,
        "offset": offset,
        "limit": limit,
    })
    .to_string()
}

pub(super) fn controller_download_user_stats_json(
    _query: Option<&str>,
    transfers: &TransferQueue,
) -> String {
    let mut grouped: BTreeMap<String, (usize, usize, usize, u64, Option<u64>)> = BTreeMap::new();
    for entry in transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0)
    {
        let stats = grouped
            .entry(entry.peer_username.clone().unwrap_or_default())
            .or_insert((0, 0, 0, 0, None));
        stats.0 += 1;
        if is_successful_transfer_status(&entry.status) {
            stats.1 += 1;
            stats.3 = stats.3.saturating_add(entry.bytes_transferred);
        }
        if is_failed_transfer_status(&entry.status) || entry.status == "cancelled" {
            stats.2 += 1;
        }
        if is_terminal_transfer_status(&entry.status) {
            stats.4 = Some(stats.4.unwrap_or(0).max(entry.updated_at));
        }
    }

    let stats = grouped
        .into_iter()
        .map(
            |(username, (total, successful, failed, total_bytes, last_download_at))| {
                let last_download_at = last_download_at.and_then(|timestamp| {
                    i64::try_from(timestamp)
                        .ok()
                        .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
                        .map(|timestamp| {
                            timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                        })
                });
                (
                    username.clone(),
                    serde_json::json!({
                        "username": username,
                        "totalDownloads": total,
                        "successfulDownloads": successful,
                        "failedDownloads": failed,
                        "totalBytes": total_bytes,
                        "lastDownloadAt": last_download_at,
                    }),
                )
            },
        )
        .collect::<BTreeMap<_, _>>();
    serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_owned())
}

pub(super) fn controller_transfer_histogram_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> String {
    let interval = query_params(query.unwrap_or_default())
        .into_iter()
        .find_map(|(name, value)| (name == "interval").then(|| value.parse::<u64>().ok())?)
        .unwrap_or(60);
    let direction = controller_transfer_query_direction(query);
    let username = controller_transfer_query_username(query);
    let entries = transfers
        .entries
        .iter()
        .filter(|entry| controller_transfer_matches_query(entry, direction, username.as_deref()))
        .collect::<Vec<_>>();
    let buckets = if entries.is_empty() {
        Vec::new()
    } else {
        let downloads = entries.iter().filter(|entry| entry.direction == 0).count();
        let uploads = entries.iter().filter(|entry| entry.direction == 1).count();
        let total_bytes = entries
            .iter()
            .map(|entry| entry.bytes_transferred)
            .sum::<u64>();
        vec![serde_json::json!({
            "start": entries.iter().map(|entry| entry.requested_at).min().unwrap_or(0).to_string(),
            "end": entries.iter().map(|entry| entry.updated_at).max().unwrap_or(0).to_string(),
            "count": entries.len(),
            "downloads": downloads,
            "uploads": uploads,
            "totalBytes": total_bytes,
        })]
    };
    serde_json::json!({
        "interval": interval,
        "buckets": buckets,
    })
    .to_string()
}

pub(super) fn controller_versioned_transfer_histogram_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> Result<String, &'static str> {
    let interval_minutes = controller_transfer_query_value(query, "interval")
        .map(|value| value.parse::<u64>().map_err(|_| "Invalid interval"))
        .transpose()?
        .unwrap_or(60);
    if interval_minutes < 5 {
        return Err("Interval must be greater than or equal to 5");
    }
    let now = unix_timestamp();
    let start = controller_transfer_query_unix_timestamp(query, "start")
        .or_else(|| controller_transfer_query_unix_timestamp(query, "startDate"))
        .unwrap_or_else(|| now.saturating_sub(7 * 24 * 60 * 60));
    let end = controller_transfer_query_unix_timestamp(query, "end")
        .or_else(|| controller_transfer_query_unix_timestamp(query, "endDate"))
        .unwrap_or(now);
    if end <= start {
        return Err("End time must be later than start time");
    }
    let interval_seconds = interval_minutes.saturating_mul(60);
    let first_bucket = start - (start % interval_seconds);
    let direction = controller_transfer_query_direction(query);
    let username = controller_transfer_query_username(query);
    let mut buckets = BTreeMap::<String, serde_json::Value>::new();
    let mut bucket_start = first_bucket;
    while bucket_start < end {
        let key = chrono::DateTime::from_timestamp(
            i64::try_from(bucket_start).map_err(|_| "Invalid start time")?,
            0,
        )
        .ok_or("Invalid start time")?
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
        buckets.insert(
            key,
            serde_json::json!({
                "Download": {},
                "Upload": {},
            }),
        );
        bucket_start = bucket_start.saturating_add(interval_seconds);
        if bucket_start == u64::MAX {
            break;
        }
    }

    // Populate the same state maps as the summary projection for completed
    // records whose end time falls into a generated bucket. Fresh-state
    // nominal calls remain exact empty direction maps.
    let mut grouped = BTreeMap::<(String, u32, String), Vec<&TransferEntry>>::new();
    for entry in transfers.entries.iter().filter(|entry| {
        is_terminal_transfer_status(&entry.status)
            && entry.updated_at >= start
            && entry.updated_at <= end
            && controller_transfer_matches_query(entry, direction, username.as_deref())
    }) {
        let bucket_start = entry.updated_at - (entry.updated_at % interval_seconds);
        let key = chrono::DateTime::from_timestamp(
            i64::try_from(bucket_start).map_err(|_| "Invalid transfer time")?,
            0,
        )
        .ok_or("Invalid transfer time")?
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
        grouped
            .entry((
                key,
                entry.direction,
                controller_transfer_summary_state(&entry.status).to_owned(),
            ))
            .or_default()
            .push(entry);
    }
    for ((key, direction, state), entries) in grouped {
        let direction_name = if direction == 0 { "Download" } else { "Upload" };
        let bucket = buckets
            .get_mut(&key)
            .ok_or("Transfer fell outside histogram window")?;
        let direction_map = bucket
            .get_mut(direction_name)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("Invalid histogram direction map")?;
        direction_map.insert(state, controller_versioned_transfer_summary_value(&entries));
    }

    Ok(serde_json::to_string(&buckets).unwrap_or_else(|_| "{}".to_owned()))
}

pub(super) fn controller_transfer_leaderboard_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> Result<String, &'static str> {
    let direction = Some(controller_transfer_query_required_direction(query)?);
    let start = controller_transfer_query_unix_timestamp(query, "start")
        .or_else(|| controller_transfer_query_unix_timestamp(query, "startDate"))
        .unwrap_or(0);
    let end = controller_transfer_query_unix_timestamp(query, "end")
        .or_else(|| controller_transfer_query_unix_timestamp(query, "endDate"))
        .unwrap_or(u64::MAX);
    if end <= start {
        return Err("End time must be later than start time");
    }
    let (limit, offset) = controller_transfer_query_limit_offset(query);
    let sort_by = controller_transfer_query_value(query, "sortBy")
        .unwrap_or_else(|| "Count".to_owned())
        .to_ascii_lowercase();
    let descending = controller_transfer_query_value(query, "sortOrder")
        .map(|value| !value.eq_ignore_ascii_case("ASC"))
        .unwrap_or(true);
    let now = unix_timestamp();
    let mut by_user: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();
    for entry in transfers
        .entries
        .iter()
        .filter(|entry| controller_transfer_matches_query(entry, direction, None))
        .filter(|entry| entry.updated_at >= start && entry.updated_at <= end)
        // Matches the oracle's GetTransferLeaderboard: only completed
        // transfers count toward the leaderboard, not queued/in-progress/
        // cancelled/failed ones -- otherwise a user with many failed
        // attempts could outrank one with fewer, real completions.
        .filter(|entry| controller_transfer_state(&entry.status) == "Completed")
    {
        let username = entry.peer_username.clone().unwrap_or_default();
        let bytes = entry.size.unwrap_or(entry.bytes_transferred);
        let summary = by_user.entry(username).or_insert((0, 0, 0.0));
        summary.0 += 1;
        summary.1 = summary.1.saturating_add(bytes);
        summary.2 += entry.average_speed_at(now);
    }
    let mut rows = by_user
        .into_iter()
        .map(|(username, (count, total_bytes, total_speed))| {
            serde_json::json!({
                "username": username,
                "count": count,
                "totalBytes": total_bytes,
                "averageSpeed": total_speed / count as f64,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let ordering = match sort_by.as_str() {
            "totalbytes" => left["totalBytes"]
                .as_u64()
                .cmp(&right["totalBytes"].as_u64()),
            "averagespeed" => left["averageSpeed"]
                .as_f64()
                .partial_cmp(&right["averageSpeed"].as_f64())
                .unwrap_or(std::cmp::Ordering::Equal),
            _ => left["count"].as_u64().cmp(&right["count"].as_u64()),
        };
        let ordering = if descending {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| left["username"].as_str().cmp(&right["username"].as_str()))
    });
    Ok(serde_json::Value::Array(rows.into_iter().skip(offset).take(limit).collect()).to_string())
}

pub(super) fn controller_transfer_exceptions_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> Result<String, &'static str> {
    let direction = Some(controller_transfer_query_required_direction(query)?);
    let username = controller_transfer_query_username(query);
    let (limit, offset) = controller_transfer_query_limit_offset(query);
    let descending = controller_transfer_query_value(query, "sortOrder")
        .map(|value| !value.eq_ignore_ascii_case("ASC"))
        .unwrap_or(true);
    let mut entries = transfers
        .entries
        .iter()
        .filter(|entry| controller_transfer_matches_query(entry, direction, username.as_deref()))
        .filter(|entry| {
            matches!(
                controller_transfer_state(&entry.status),
                "Cancelled" | "Failed" | "Rejected"
            )
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        let ordering = left.requested_at.cmp(&right.requested_at);
        let ordering = if descending {
            ordering.reverse()
        } else {
            ordering
        };
        ordering
            .then_with(|| left.filename.cmp(&right.filename))
            .then_with(|| left.id.cmp(&right.id))
    });
    let rows = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|entry| {
            serde_json::json!({
                "username": entry.peer_username.as_deref().unwrap_or_default(),
                "direction": if entry.direction == 0 { "Download" } else { "Upload" },
                "filename": entry.filename,
                "state": controller_transfer_state(&entry.status),
                "exception": public_transfer_reason(&entry.status, entry.reason.as_deref())
                    .unwrap_or(controller_transfer_state(&entry.status)),
                "requestedAt": entry.requested_at.to_string(),
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::Value::Array(rows).to_string())
}

pub(super) fn controller_transfer_exceptions_pareto_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> Result<String, &'static str> {
    let direction = Some(controller_transfer_query_required_direction(query)?);
    let username = controller_transfer_query_username(query);
    let (limit, offset) = controller_transfer_query_limit_offset(query);
    let mut grouped: BTreeMap<String, (usize, BTreeMap<String, ()>)> = BTreeMap::new();
    for entry in transfers
        .entries
        .iter()
        .filter(|entry| controller_transfer_matches_query(entry, direction, username.as_deref()))
        .filter(|entry| {
            matches!(
                controller_transfer_state(&entry.status),
                "Cancelled" | "Failed" | "Rejected"
            )
        })
    {
        let exception = public_transfer_reason(&entry.status, entry.reason.as_deref())
            .unwrap_or(controller_transfer_state(&entry.status))
            .to_owned();
        let group = grouped.entry(exception).or_insert((0, BTreeMap::new()));
        group.0 += 1;
        group
            .1
            .insert(entry.peer_username.clone().unwrap_or_default(), ());
    }
    let mut rows = grouped
        .into_iter()
        .map(|(exception, (count, users))| {
            serde_json::json!({
                "exception": exception,
                "count": count,
                "distinctUsers": users.len(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right["count"]
            .as_u64()
            .cmp(&left["count"].as_u64())
            .then_with(|| left["exception"].as_str().cmp(&right["exception"].as_str()))
    });
    Ok(serde_json::Value::Array(rows.into_iter().skip(offset).take(limit).collect()).to_string())
}

pub(super) fn controller_transfer_directories_report(
    query: Option<&str>,
    transfers: &TransferQueue,
) -> String {
    let username = controller_transfer_query_username(query);
    let (limit, offset) = controller_transfer_query_limit_offset(query);
    let mut grouped: BTreeMap<String, (usize, u64, BTreeMap<String, ()>)> = BTreeMap::new();
    // Matches the oracle's GetTransferDirectoryFrequency exactly: this
    // report always answers "which of my shared directories are popular"
    // -- i.e. what other users downloaded from local shares (Upload),
    // never what this instance itself downloaded -- and only counts
    // completed transfers, never queued/in-progress/failed ones.
    for entry in transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1)
        .filter(|entry| controller_transfer_state(&entry.status) == "Completed")
        .filter(|entry| controller_transfer_matches_query(entry, None, username.as_deref()))
    {
        let directory = Path::new(&entry.filename)
            .parent()
            .and_then(|path| path.to_str())
            .filter(|path| !path.is_empty())
            .unwrap_or("")
            .replace('\\', "/");
        let bytes = entry.size.unwrap_or(entry.bytes_transferred);
        let group = grouped.entry(directory).or_insert((0, 0, BTreeMap::new()));
        group.0 += 1;
        group.1 = group.1.saturating_add(bytes);
        group
            .2
            .insert(entry.peer_username.clone().unwrap_or_default(), ());
    }
    let mut rows = grouped
        .into_iter()
        .map(|(directory, (count, total_bytes, users))| {
            serde_json::json!({
                "path": directory,
                "directory": directory,
                "count": count,
                "totalBytes": total_bytes,
                "distinctUsers": users.len(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right["distinctUsers"]
            .as_u64()
            .cmp(&left["distinctUsers"].as_u64())
            .then_with(|| right["count"].as_u64().cmp(&left["count"].as_u64()))
            .then_with(|| left["path"].as_str().cmp(&right["path"].as_str()))
    });
    serde_json::Value::Array(rows.into_iter().skip(offset).take(limit).collect()).to_string()
}

pub(super) fn controller_user_transfer_report(username: &str, transfers: &TransferQueue) -> String {
    let entries = transfers
        .entries
        .iter()
        .filter(|entry| entry.peer_username.as_deref() == Some(username))
        .map(TransferEntry::controller_file_json)
        .collect::<Vec<_>>();
    let direction_report = |direction: u32| {
        let selected = transfers
            .entries
            .iter()
            .filter(|entry| {
                entry.peer_username.as_deref() == Some(username) && entry.direction == direction
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "summary": {},
            "statistics": {
                "total": selected.len(),
                "successful": selected.iter().filter(|entry| is_successful_transfer_status(&entry.status)).count(),
                "errored": selected.iter().filter(|entry| is_failed_transfer_status(&entry.status)).count(),
                "cancelled": selected.iter().filter(|entry| entry.status == "cancelled").count(),
            },
            "exceptions": [],
        })
    };
    serde_json::json!({
        "Upload": direction_report(1),
        "Download": direction_report(0),
        "username": username,
        "count": entries.len(),
        "transfers": entries,
    })
    .to_string()
}
