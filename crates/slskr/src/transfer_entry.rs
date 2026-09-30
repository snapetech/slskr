use super::{
    controller_transfer_state, is_failed_transfer_status, is_successful_transfer_status,
    is_terminal_transfer_status, json_escape, json_option, json_u32_option, json_u64_option,
    truncate_utf8_bytes, unix_seconds_rfc3339, unix_timestamp, virtual_basename,
    MAX_TRANSFER_BATCH_ID_BYTES, MAX_TRANSFER_FILENAME_BYTES, MAX_TRANSFER_LOCAL_PATH_BYTES,
    MAX_TRANSFER_METADATA_TEXT_BYTES, MAX_TRANSFER_REASON_BYTES, MAX_TRANSFER_REQUEST_ID_BYTES,
    MAX_TRANSFER_REQUEST_NAME_BYTES, MAX_TRANSFER_STATUS_BYTES, MAX_TRANSFER_USERNAME_BYTES,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct TransferEntry {
    pub(super) id: u64,
    pub(super) direction: u32,
    pub(super) token: u32,
    pub(super) peer_username: Option<String>,
    pub(super) filename: String,
    pub(super) local_path: Option<String>,
    #[serde(default)]
    pub(super) batch_id: Option<String>,
    #[serde(default)]
    pub(super) request_id: Option<String>,
    #[serde(default)]
    pub(super) wishlist_item_id: Option<String>,
    #[serde(default)]
    pub(super) request_name: Option<String>,
    #[serde(default)]
    pub(super) destination_directory: Option<String>,
    #[serde(default)]
    pub(super) bit_rate: Option<u32>,
    #[serde(default)]
    pub(super) sample_rate: Option<u32>,
    #[serde(default)]
    pub(super) bit_depth: Option<u32>,
    #[serde(default)]
    pub(super) length_seconds: Option<u32>,
    #[serde(default)]
    pub(super) artist: Option<String>,
    #[serde(default)]
    pub(super) album: Option<String>,
    #[serde(default)]
    pub(super) title: Option<String>,
    #[serde(default)]
    pub(super) track_number: Option<u32>,
    #[serde(default)]
    pub(super) year: Option<u32>,
    #[serde(default = "default_transfer_attempts")]
    pub(super) attempts: u32,
    #[serde(default)]
    pub(super) auto_replace_attempts: u32,
    #[serde(default)]
    pub(super) next_attempt_at: Option<u64>,
    pub(super) size: Option<u64>,
    pub(super) bytes_transferred: u64,
    pub(super) status: String,
    pub(super) reason: Option<String>,
    pub(super) requested_at: u64,
    #[serde(default)]
    pub(super) started_at: Option<u64>,
    #[serde(default)]
    pub(super) start_offset: u64,
    pub(super) updated_at: u64,
    #[serde(default)]
    pub(super) updated_at_ms: u64,
    // SignalR's transfer activity DTO includes the state before the
    // transition. This is process-local metadata and is intentionally not
    // persisted; a rehydrated transfer has no in-process transition to report.
    #[serde(skip)]
    pub(super) previous_status: Option<String>,
}

fn default_transfer_attempts() -> u32 {
    1
}

impl TransferEntry {
    #[allow(dead_code)]
    pub(super) fn json(&self) -> String {
        format!(
            "{{\"id\":{},\"direction\":{},\"token\":{},\"peer_username\":{},\"filename\":\"{}\",\"local_path\":{},\"batch_id\":{},\"request_id\":{},\"request_name\":{},\"destination_directory\":{},\"bit_rate\":{},\"sample_rate\":{},\"bit_depth\":{},\"length_seconds\":{},\"artist\":{},\"album\":{},\"title\":{},\"track_number\":{},\"year\":{},\"attempts\":{},\"auto_replace_attempts\":{},\"next_attempt_at\":{},\"size\":{},\"bytes_transferred\":{},\"status\":\"{}\",\"reason\":{},\"failure_code\":{},\"recovery_action\":{},\"recovery_label\":{},\"requested_at\":{},\"started_at\":{},\"start_offset\":{},\"updated_at\":{}}}",
            self.id,
            self.direction,
            self.token,
            json_option(self.peer_username.as_deref()),
            json_escape(&self.filename),
            "null",
            json_option(self.batch_id.as_deref()),
            json_option(self.request_id.as_deref()),
            json_option(self.request_name.as_deref()),
            json_option(self.destination_directory.as_deref()),
            json_u32_option(self.bit_rate),
            json_u32_option(self.sample_rate),
            json_u32_option(self.bit_depth),
            json_u32_option(self.length_seconds),
            json_option(self.artist.as_deref()),
            json_option(self.album.as_deref()),
            json_option(self.title.as_deref()),
            json_u32_option(self.track_number),
            json_u32_option(self.year),
            self.attempts,
            self.auto_replace_attempts,
            json_u64_option(self.next_attempt_at),
            json_u64_option(self.size),
            self.bytes_transferred,
            json_escape(&self.status),
            json_option(public_transfer_reason(&self.status, self.reason.as_deref())),
            json_option(transfer_failure_code(&self.status, self.reason.as_deref())),
            json_option(transfer_recovery_action(&self.status, self.reason.as_deref())),
            json_option(transfer_recovery_label(&self.status, self.reason.as_deref())),
            self.requested_at,
            json_u64_option(self.started_at),
            self.start_offset,
            self.updated_at
        )
    }

    pub(super) fn elapsed_seconds_at(&self, now: u64) -> Option<u64> {
        let started_at = self.started_at?;
        let ended_at = if is_terminal_transfer_status(&self.status) {
            self.updated_at
        } else {
            now
        };
        Some(ended_at.saturating_sub(started_at))
    }

    pub(super) fn average_speed_at(&self, now: u64) -> f64 {
        let Some(elapsed) = self.elapsed_seconds_at(now) else {
            return 0.0;
        };
        let transferred = self.bytes_transferred.saturating_sub(self.start_offset);
        if transferred == 0 {
            0.0
        } else {
            transferred as f64 / elapsed.max(1) as f64
        }
    }

    pub(super) fn controller_file_json(&self) -> serde_json::Value {
        let size = self.size.unwrap_or(0);
        let bytes_remaining = size.saturating_sub(self.bytes_transferred);
        let percent_complete = if size == 0 {
            0.0
        } else {
            ((self.bytes_transferred as f64 / size as f64) * 100.0).min(100.0)
        };
        let now = unix_timestamp();
        let average_speed = self.average_speed_at(now);
        let elapsed_seconds = self.elapsed_seconds_at(now);
        let started_at = self
            .started_at
            .map(|value| value.to_string())
            .unwrap_or_default();
        let ended_at = if is_terminal_transfer_status(&self.status) {
            self.updated_at.to_string()
        } else {
            String::new()
        };
        let remaining_seconds = (!is_terminal_transfer_status(&self.status)
            && average_speed > 0.0
            && bytes_remaining > 0)
            .then(|| (bytes_remaining as f64 / average_speed).ceil() as u64);
        serde_json::json!({
            "id": self.id.to_string(),
            "username": self.peer_username.as_deref().unwrap_or_default(),
            "direction": if self.direction == 0 { "Download" } else { "Upload" },
            "filename": self.filename,
            "batchId": self.batch_id,
            "requestId": self.request_id,
            "requestName": self.request_name,
            "destinationDirectory": self.destination_directory,
            "bitRate": self.bit_rate,
            "sampleRate": self.sample_rate,
            "bitDepth": self.bit_depth,
            "length": self.length_seconds,
            "artist": self.artist,
            "album": self.album,
            "title": self.title,
            "trackNumber": self.track_number,
            "year": self.year,
            "attempts": self.attempts,
            "nextAttemptAt": self.next_attempt_at.map(unix_seconds_rfc3339),
            "size": size,
            "startOffset": self.start_offset,
            "state": controller_transfer_state(&self.status),
            "requestedAt": self.requested_at.to_string(),
            "enqueuedAt": self.requested_at.to_string(),
            "startedAt": started_at,
            "endedAt": ended_at,
            "bytesTransferred": self.bytes_transferred,
            "averageSpeed": average_speed,
            "bytesRemaining": bytes_remaining,
            "elapsedTime": elapsed_seconds.map(format_transfer_duration).unwrap_or_default(),
            "percentComplete": percent_complete,
            "remainingTime": remaining_seconds.map(format_transfer_duration).unwrap_or_default(),
            "failureCode": transfer_failure_code(&self.status, self.reason.as_deref()),
            "recoveryAction": transfer_recovery_action(&self.status, self.reason.as_deref()),
            "recoveryLabel": transfer_recovery_label(&self.status, self.reason.as_deref()),
        })
    }
}

pub(super) fn transfer_directory_name(filename: &str) -> String {
    filename
        .rsplit_once('/')
        .map(|(directory, _)| directory.to_owned())
        .or_else(|| {
            filename
                .rsplit_once('\\')
                .map(|(directory, _)| directory.to_owned())
        })
        .unwrap_or_default()
}

pub(super) fn format_transfer_duration(seconds: u64) -> String {
    let hours = seconds / 3_600;
    let minutes = seconds % 3_600 / 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub(super) fn public_transfer_reason(status: &str, reason: Option<&str>) -> Option<&'static str> {
    reason.map(|_| match status {
        "failed" => "transfer failed",
        "rejected" => "transfer rejected",
        "errored" => "transfer failed",
        "cancelled" => "transfer cancelled",
        _ => "transfer operation unavailable",
    })
}

pub(super) fn transfer_failure_code(status: &str, reason: Option<&str>) -> Option<&'static str> {
    if !is_failed_transfer_status(status) {
        return None;
    }
    let reason = reason?.to_ascii_lowercase();
    Some(
        if reason.contains("too many megabytes") || reason.contains("overwhelmed") {
            "peer_capacity"
        } else if reason.contains("not shared") || reason.contains("file not found") {
            "remote_file_unavailable"
        } else if reason.contains("size mismatch")
            || reason.contains("does not match expected size")
        {
            "size_mismatch"
        } else if reason.contains("offline") || reason.contains("unavailable") {
            "peer_offline"
        } else if reason.contains("connection closed")
            || reason.contains("connection lost")
            || reason.contains("reset by peer")
        {
            "connection_lost"
        } else if reason.contains("internal error") {
            "remote_internal"
        } else {
            "transfer_failed"
        },
    )
}

pub(super) fn transfer_recovery_action(status: &str, reason: Option<&str>) -> Option<&'static str> {
    match transfer_failure_code(status, reason)? {
        "peer_offline" => Some("wait"),
        _ => Some("retry"),
    }
}

pub(super) fn transfer_recovery_label(status: &str, reason: Option<&str>) -> Option<&'static str> {
    match transfer_failure_code(status, reason)? {
        "peer_capacity" => Some("Retry later"),
        "remote_file_unavailable" | "size_mismatch" | "remote_internal" => {
            Some("Find other sources")
        }
        "peer_offline" => Some("Wait for online"),
        "connection_lost" => Some("Retry connection"),
        _ => Some("Retry"),
    }
}

pub(super) fn download_request_state(entries: &[&TransferEntry]) -> &'static str {
    if entries
        .iter()
        .any(|entry| is_successful_transfer_status(&entry.status))
    {
        "Completed"
    } else if entries.iter().any(|entry| {
        matches!(
            entry.status.as_str(),
            "queued"
                | "accepted"
                | "peer_lookup"
                | "peer_negotiating"
                | "indirect_pending"
                | "in_progress"
        )
    }) {
        "Active"
    } else if entries.iter().all(|entry| entry.status == "cancelled") {
        "Cancelled"
    } else {
        "Failed"
    }
}

pub(super) fn download_request_projection(
    entries: &[&TransferEntry],
    include_attempts: bool,
) -> serde_json::Value {
    let first = entries
        .iter()
        .min_by_key(|entry| entry.requested_at)
        .copied()
        .expect("download request projection requires an attempt");
    let current = entries
        .iter()
        .filter(|entry| entry.status != "cancelled")
        .max_by_key(|entry| entry.requested_at)
        .or_else(|| entries.iter().max_by_key(|entry| entry.requested_at))
        .copied();
    let state = download_request_state(entries);
    let completed_at = matches!(state, "Completed" | "Failed" | "Cancelled")
        .then(|| entries.iter().map(|entry| entry.updated_at).max())
        .flatten();
    let request = serde_json::json!({
        "id": first.request_id,
        "name": first.request_name.as_deref().unwrap_or_else(|| virtual_basename(&first.filename)),
        "originalFilename": first.filename,
        "size": first.size,
        "batchId": first.batch_id,
        "destinationDirectory": first.destination_directory,
        "state": state,
        "stateDescription": state,
        "createdAt": first.requested_at,
        "completedAt": completed_at,
        "bitRate": first.bit_rate,
        "sampleRate": first.sample_rate,
        "bitDepth": first.bit_depth,
        "length": first.length_seconds,
        "artist": first.artist,
        "album": first.album,
        "title": first.title,
        "trackNumber": first.track_number,
        "year": first.year,
    });
    let current =
        current.and_then(|entry| serde_json::from_str::<serde_json::Value>(&entry.json()).ok());
    if include_attempts {
        let attempts = entries
            .iter()
            .filter_map(|entry| serde_json::from_str::<serde_json::Value>(&entry.json()).ok())
            .collect::<Vec<_>>();
        serde_json::json!({"request": request, "attempts": attempts, "current": current})
    } else {
        serde_json::json!({"request": request, "attemptCount": entries.len(), "current": current})
    }
}

pub(super) fn native_download_status(status: &str) -> &str {
    match status {
        "queued" | "peer_lookup" | "peer_negotiating" | "indirect_pending" => "queued",
        "accepted" | "in_progress" => "running",
        "succeeded" | "completed" => "completed",
        "cancelled" => "cancelled",
        "failed" | "rejected" | "errored" | "timed_out" => "failed",
        _ => "failed",
    }
}

pub(super) fn native_compatibility_download_json(entry: &TransferEntry) -> serde_json::Value {
    let size = entry.size.unwrap_or(0);
    let id = entry
        .request_id
        .clone()
        .unwrap_or_else(|| entry.id.to_string());
    serde_json::json!({
        "Id": id,
        "User": entry.peer_username,
        "RemotePath": entry.filename,
        "LocalPath": entry.local_path,
        "Status": native_download_status(&entry.status),
        "Progress": if size == 0 { 0.0 } else { entry.bytes_transferred as f64 / size as f64 },
        "Size": size,
        "Remaining": size.saturating_sub(entry.bytes_transferred),
        "Speed": entry.average_speed_at(unix_timestamp()),
    })
}

pub(super) fn public_transfer_events_error(error: Option<&str>) -> Option<&'static str> {
    error.map(|_| "transfer events unavailable")
}

pub(super) fn public_transfer_state_error(error: Option<&str>) -> Option<&'static str> {
    error.map(|_| "transfer state unavailable")
}

pub(super) fn bounded_transfer_username(username: &str) -> String {
    truncate_utf8_bytes(username.to_owned(), MAX_TRANSFER_USERNAME_BYTES)
}

pub(super) fn bounded_transfer_filename(filename: &str) -> String {
    truncate_utf8_bytes(filename.to_owned(), MAX_TRANSFER_FILENAME_BYTES)
}

pub(super) fn bounded_transfer_reason(reason: Option<String>) -> Option<String> {
    reason.map(|reason| truncate_utf8_bytes(reason, MAX_TRANSFER_REASON_BYTES))
}

pub(super) fn bounded_transfer_entry(mut entry: TransferEntry) -> TransferEntry {
    entry.attempts = entry.attempts.max(1);
    if matches!(
        entry.status.as_str(),
        "succeeded" | "completed" | "cancelled" | "errored"
    ) {
        entry.next_attempt_at = None;
    }
    if entry.updated_at_ms == 0 {
        entry.updated_at_ms = entry.updated_at.saturating_mul(1_000);
    }
    entry.peer_username = entry
        .peer_username
        .map(|username| truncate_utf8_bytes(username, MAX_TRANSFER_USERNAME_BYTES));
    entry.filename = truncate_utf8_bytes(entry.filename, MAX_TRANSFER_FILENAME_BYTES);
    entry.local_path = entry
        .local_path
        .filter(|path| path.len() <= MAX_TRANSFER_LOCAL_PATH_BYTES);
    entry.batch_id = entry
        .batch_id
        .map(|batch_id| truncate_utf8_bytes(batch_id, MAX_TRANSFER_BATCH_ID_BYTES));
    entry.request_id = entry
        .request_id
        .map(|request_id| truncate_utf8_bytes(request_id, MAX_TRANSFER_REQUEST_ID_BYTES));
    entry.wishlist_item_id = entry
        .wishlist_item_id
        .map(|item_id| truncate_utf8_bytes(item_id, MAX_TRANSFER_REQUEST_ID_BYTES));
    if entry.direction == 0 && entry.request_id.is_none() {
        entry.request_id = Some(uuid::Uuid::new_v4().to_string());
    }
    entry.request_name = entry
        .request_name
        .or_else(|| (entry.direction == 0).then(|| virtual_basename(&entry.filename).to_owned()))
        .map(|name| truncate_utf8_bytes(name, MAX_TRANSFER_REQUEST_NAME_BYTES));
    entry.destination_directory = entry
        .destination_directory
        .filter(|path| path.len() <= MAX_TRANSFER_LOCAL_PATH_BYTES);
    entry.artist = entry
        .artist
        .map(|value| truncate_utf8_bytes(value, MAX_TRANSFER_METADATA_TEXT_BYTES));
    entry.album = entry
        .album
        .map(|value| truncate_utf8_bytes(value, MAX_TRANSFER_METADATA_TEXT_BYTES));
    entry.title = entry
        .title
        .map(|value| truncate_utf8_bytes(value, MAX_TRANSFER_METADATA_TEXT_BYTES));
    entry.status = truncate_utf8_bytes(entry.status, MAX_TRANSFER_STATUS_BYTES);
    entry.reason = bounded_transfer_reason(entry.reason);
    entry
}
