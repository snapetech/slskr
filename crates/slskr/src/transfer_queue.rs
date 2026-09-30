use super::transfer_durability::reset_transfer_durability_state;
use super::transfer_state_io::{
    load_transfer_state, transfer_events_path, transfer_state_path, write_transfer_events_header,
};
use super::{
    bounded_transfer_entry, bounded_transfer_filename, bounded_transfer_reason,
    bounded_transfer_username, is_active_transfer_status, is_failed_transfer_status,
    is_remote_queue_response, is_successful_transfer_status, is_terminal_transfer_status,
    json_escape, json_option, json_usize_option, next_transfer_updated_at_ms,
    public_transfer_events_error, public_transfer_state_error, transfer_directory_name,
    truncate_utf8_bytes, unix_timestamp, unix_timestamp_millis, virtual_basename, AppConfig,
    RecordListFilter, TransferEntry, MAX_TRANSFER_LOCAL_PATH_BYTES, MAX_TRANSFER_STATUS_BYTES,
};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug)]
pub(super) struct TransferQueue {
    pub(super) entries: Vec<TransferEntry>,
    pub(super) progress_persisted_at: BTreeMap<u64, u64>,
    pub(super) next_id: u64,
    pub(super) next_token: u32,
    pub(super) history_limit: usize,
    pub(super) events_path: PathBuf,
    pub(super) state_path: PathBuf,
    pub(super) events_error: Option<String>,
    pub(super) state_error: Option<String>,
    pub(super) updated_at: u64,
}

#[derive(Clone, Debug, Default)]
pub(super) struct TransferRequestDetails {
    pub(super) request_id: Option<String>,
    pub(super) wishlist_item_id: Option<String>,
    pub(super) request_name: Option<String>,
    pub(super) destination_directory: Option<String>,
    pub(super) bit_rate: Option<u32>,
    pub(super) sample_rate: Option<u32>,
    pub(super) bit_depth: Option<u32>,
    pub(super) length_seconds: Option<u32>,
    pub(super) artist: Option<String>,
    pub(super) album: Option<String>,
    pub(super) title: Option<String>,
    pub(super) track_number: Option<u32>,
    pub(super) year: Option<u32>,
    pub(super) attempts: u32,
    pub(super) auto_replace_attempts: u32,
    pub(super) next_attempt_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct AudioTechnicalMetadata {
    pub(super) bit_rate: Option<u32>,
    pub(super) sample_rate: Option<u32>,
    pub(super) bit_depth: Option<u32>,
    pub(super) length_seconds: Option<u32>,
}

#[allow(dead_code)]
impl TransferQueue {
    pub(super) fn new(config: &AppConfig) -> Self {
        let events_path = transfer_events_path(&config.state_dir);
        let state_path = transfer_state_path(&config.state_dir);
        let events_error = write_transfer_events_header(&events_path).err();
        let (entries, state_error) =
            load_transfer_state(&state_path, config.transfer_history_limit)
                .map(|entries| (entries, None))
                .unwrap_or_else(|error| (Vec::new(), Some(error)));
        let next_id = entries
            .iter()
            .map(|entry| entry.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let next_token = entries
            .iter()
            .map(|entry| entry.token)
            .max()
            .unwrap_or(0)
            .wrapping_add(1)
            .max(1);
        let mut queue = Self {
            entries,
            progress_persisted_at: BTreeMap::new(),
            next_id,
            next_token,
            history_limit: config.transfer_history_limit,
            events_path,
            state_path,
            events_error,
            state_error,
            updated_at: unix_timestamp(),
        };
        reset_transfer_durability_state(&queue.state_path);
        if queue.state_error.is_none() {
            queue.mark_state_dirty();
        }
        queue
    }

    pub(super) async fn rehydrate_from_database(
        &mut self,
        db: &crate::persistence::DatabaseManager,
    ) {
        let max_transfer_id = db.max_transfer_id().await.ok();
        if let Some(max_transfer_id) = max_transfer_id {
            self.next_id = self.next_id.max(max_transfer_id.saturating_add(1).max(1));
        }
        let records = match db.list_transfers(None, 1000, 0).await {
            Ok(records) => records,
            Err(_) => return,
        };
        if records.is_empty() {
            return;
        }
        let mut sqlite_entries: Vec<TransferEntry> = records
            .into_iter()
            .filter_map(|record| {
                let id = record.id.parse::<u64>().ok()?;
                let token = u32::try_from(id).ok()?.max(1);
                let size = u64::try_from(record.filesize).ok();
                let started_at = u64::try_from(record.started_at).ok()?;
                let direction = if record.direction == "upload" { 1 } else { 0 };
                let status = match record.status.as_str() {
                    "queued" | "Queued" => "queued",
                    // Convert in_progress to queued for retry after restart
                    "in_progress" | "InProgress" | "accepted" | "Accepted" | "peer_lookup"
                    | "PeerLookup" | "peer_negotiating" | "PeerNegotiating"
                    | "indirect_pending" | "IndirectPending" => "queued",
                    "succeeded" | "Succeeded" => "succeeded",
                    "completed" | "Completed" => "completed",
                    "failed" | "Failed" => "failed",
                    "rejected" | "Rejected" => "rejected",
                    "cancelled" | "Cancelled" => "cancelled",
                    "errored" | "Errored" => "errored",
                    "timed_out" | "TimedOut" | "timeout" | "Timeout" | "aborted" | "Aborted" => {
                        "failed"
                    }
                    _ => return None,
                };
                let bytes_transferred = u64::try_from(record.progress)
                    .unwrap_or(0)
                    .min(size.unwrap_or(u64::MAX));
                Some(TransferEntry {
                    id,
                    token,
                    direction,
                    peer_username: Some(record.peer_username),
                    filename: record.filename,
                    size,
                    status: status.to_owned(),
                    started_at: Some(started_at),
                    updated_at: record
                        .completed_at
                        .and_then(|value| u64::try_from(value).ok())
                        .unwrap_or(started_at),
                    reason: record.reason,
                    bytes_transferred,
                    local_path: record.local_path,
                    destination_directory: record.destination_directory,
                    request_id: record.request_id,
                    wishlist_item_id: record.wishlist_item_id,
                    request_name: record.request_name,
                    batch_id: record.batch_id,
                    bit_rate: record.bit_rate.and_then(|rate| u32::try_from(rate).ok()),
                    sample_rate: record.sample_rate.and_then(|rate| u32::try_from(rate).ok()),
                    bit_depth: record.bit_depth.and_then(|depth| u32::try_from(depth).ok()),
                    length_seconds: record
                        .length_seconds
                        .and_then(|secs| u32::try_from(secs).ok()),
                    artist: record.artist,
                    album: record.album,
                    title: record.title,
                    track_number: record
                        .track_number
                        .and_then(|number| u32::try_from(number).ok()),
                    year: record.year.and_then(|year| u32::try_from(year).ok()),
                    attempts: u32::try_from(record.attempts).unwrap_or(1).max(1),
                    auto_replace_attempts: u32::try_from(record.auto_replace_attempts).unwrap_or(0),
                    next_attempt_at: record
                        .next_attempt_at
                        .and_then(|value| u64::try_from(value).ok()),
                    requested_at: started_at,
                    start_offset: 0,
                    updated_at_ms: u64::try_from(record.updated_at_ms).unwrap_or(0),
                    previous_status: None,
                })
            })
            .collect();
        let existing_ids: std::collections::HashSet<u64> =
            self.entries.iter().map(|entry| entry.id).collect();
        for entry in sqlite_entries.drain(..) {
            if !existing_ids.contains(&entry.id) {
                self.entries.push(entry);
            }
        }
        self.entries.truncate(self.history_limit);
        self.progress_persisted_at.clear();
        self.next_id = self
            .entries
            .iter()
            .map(|entry| entry.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        if let Some(max_transfer_id) = max_transfer_id {
            self.next_id = self.next_id.max(max_transfer_id.saturating_add(1).max(1));
        }
        self.next_token = self
            .entries
            .iter()
            .map(|entry| entry.token)
            .max()
            .unwrap_or(0)
            .wrapping_add(1)
            .max(1);
        self.updated_at = unix_timestamp();
        if self.state_error.is_none() {
            self.mark_state_dirty();
        }
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    pub(super) fn new_in_memory(history_limit: usize) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!(
            "slskr-transfer-test-{}-{unique}",
            std::process::id()
        ));
        let events_path = base.with_extension("tsv");
        let state_path = base.with_extension("json");
        let events_error = write_transfer_events_header(&events_path).err();
        let queue = Self {
            entries: Vec::new(),
            progress_persisted_at: BTreeMap::new(),
            next_id: 1,
            next_token: 1,
            history_limit,
            events_path,
            state_path,
            events_error,
            state_error: None,
            updated_at: 0,
        };
        reset_transfer_durability_state(&queue.state_path);
        queue
    }

    pub(super) fn record_rejected_request(
        &mut self,
        direction: u32,
        token: u32,
        filename: String,
        size: Option<u64>,
        reason: String,
    ) -> TransferEntry {
        let now = unix_timestamp();
        let now_ms = unix_timestamp_millis();
        let id = self.allocate_id();
        let entry = TransferEntry {
            id,
            direction,
            token,
            peer_username: None,
            filename,
            local_path: None,
            batch_id: None,
            request_id: None,
            wishlist_item_id: None,
            request_name: None,
            destination_directory: None,
            bit_rate: None,
            sample_rate: None,
            bit_depth: None,
            length_seconds: None,
            artist: None,
            album: None,
            title: None,
            track_number: None,
            year: None,
            attempts: 1,
            auto_replace_attempts: 0,
            next_attempt_at: None,
            size,
            bytes_transferred: 0,
            status: "rejected".to_owned(),
            reason: Some(reason),
            requested_at: now,
            started_at: None,
            start_offset: 0,
            updated_at: now,
            updated_at_ms: now_ms,
            previous_status: None,
        };
        self.push_entry(entry)
    }

    pub(super) fn record_accepted_inbound_request(
        &mut self,
        direction: u32,
        token: u32,
        peer_username: Option<String>,
        filename: String,
        local_path: String,
        size: u64,
    ) -> TransferEntry {
        let now = unix_timestamp();
        let now_ms = unix_timestamp_millis();
        let id = self.allocate_id();
        let entry = TransferEntry {
            id,
            direction,
            token,
            peer_username,
            filename,
            local_path: Some(local_path),
            batch_id: None,
            request_id: None,
            wishlist_item_id: None,
            request_name: None,
            destination_directory: None,
            bit_rate: None,
            sample_rate: None,
            bit_depth: None,
            length_seconds: None,
            artist: None,
            album: None,
            title: None,
            track_number: None,
            year: None,
            attempts: 1,
            auto_replace_attempts: 0,
            next_attempt_at: None,
            size: Some(size),
            bytes_transferred: 0,
            status: "accepted".to_owned(),
            reason: None,
            requested_at: now,
            started_at: None,
            start_offset: 0,
            updated_at: now,
            updated_at_ms: now_ms,
            previous_status: None,
        };
        self.push_entry(entry)
    }

    pub(super) fn create(
        &mut self,
        direction: u32,
        peer_username: Option<String>,
        filename: String,
        local_path: Option<String>,
        size: Option<u64>,
    ) -> TransferEntry {
        self.create_with_batch(direction, peer_username, filename, local_path, size, None)
    }

    pub(super) fn create_with_batch(
        &mut self,
        direction: u32,
        peer_username: Option<String>,
        filename: String,
        local_path: Option<String>,
        size: Option<u64>,
        batch_id: Option<String>,
    ) -> TransferEntry {
        let details = if direction == 0 {
            TransferRequestDetails {
                request_id: Some(uuid::Uuid::new_v4().to_string()),
                request_name: Some(virtual_basename(&filename).to_owned()),
                ..TransferRequestDetails::default()
            }
        } else {
            TransferRequestDetails::default()
        };
        self.create_with_details(
            direction,
            peer_username,
            filename,
            local_path,
            size,
            batch_id,
            details,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_with_batch_for_wishlist(
        &mut self,
        direction: u32,
        peer_username: Option<String>,
        filename: String,
        local_path: Option<String>,
        size: Option<u64>,
        batch_id: Option<String>,
        wishlist_item_id: String,
    ) -> TransferEntry {
        let details = if direction == 0 {
            TransferRequestDetails {
                request_id: Some(uuid::Uuid::new_v4().to_string()),
                wishlist_item_id: Some(wishlist_item_id),
                request_name: Some(virtual_basename(&filename).to_owned()),
                ..TransferRequestDetails::default()
            }
        } else {
            TransferRequestDetails::default()
        };
        self.create_with_details(
            direction,
            peer_username,
            filename,
            local_path,
            size,
            batch_id,
            details,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_with_details(
        &mut self,
        direction: u32,
        peer_username: Option<String>,
        filename: String,
        local_path: Option<String>,
        size: Option<u64>,
        batch_id: Option<String>,
        details: TransferRequestDetails,
    ) -> TransferEntry {
        let now = unix_timestamp();
        let now_ms = unix_timestamp_millis();
        let id = self.allocate_id();
        let token = self.allocate_token();
        let entry = TransferEntry {
            id,
            direction,
            token,
            peer_username,
            filename,
            local_path,
            batch_id,
            request_id: details.request_id,
            wishlist_item_id: details.wishlist_item_id,
            request_name: details.request_name,
            destination_directory: details.destination_directory,
            bit_rate: details.bit_rate,
            sample_rate: details.sample_rate,
            bit_depth: details.bit_depth,
            length_seconds: details.length_seconds,
            artist: details.artist,
            album: details.album,
            title: details.title,
            track_number: details.track_number,
            year: details.year,
            attempts: details.attempts.max(1),
            auto_replace_attempts: details.auto_replace_attempts,
            next_attempt_at: details.next_attempt_at,
            size,
            bytes_transferred: 0,
            status: "queued".to_owned(),
            reason: None,
            requested_at: now,
            started_at: None,
            start_offset: 0,
            updated_at: now,
            updated_at_ms: now_ms,
            previous_status: None,
        };
        self.push_entry(entry)
    }

    pub(super) fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.entries.len() {
            if !self.entries.iter().any(|entry| entry.id == candidate) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("finite transfer history must leave an available u64 id")
    }

    pub(super) fn allocate_token(&mut self) -> u32 {
        let mut candidate = self.next_token.max(1);
        for _ in 0..=self.entries.len() {
            if !self.entries.iter().any(|entry| entry.token == candidate) {
                self.next_token = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded transfer history must leave an available u32 token")
    }

    pub(super) fn update_status(
        &mut self,
        id: u64,
        status: &str,
        bytes_transferred: Option<u64>,
        reason: Option<String>,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        let now = unix_timestamp();
        entry.previous_status = Some(entry.status.clone());
        if status == "in_progress" && entry.started_at.is_none() {
            entry.started_at = Some(now);
            entry.start_offset = entry.bytes_transferred;
        }
        entry.status = truncate_utf8_bytes(status.to_owned(), MAX_TRANSFER_STATUS_BYTES);
        if let Some(bytes_transferred) = bytes_transferred {
            entry.bytes_transferred = bytes_transferred;
        }
        entry.reason = bounded_transfer_reason(reason);
        if is_terminal_transfer_status(status) {
            entry.next_attempt_at = None;
        }
        entry.updated_at = now;
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_durable_mutation(Some(entry.clone()));
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn update_local_execution(
        &mut self,
        id: u64,
        status: &str,
        bytes_transferred: u64,
        size: Option<u64>,
        reason: Option<String>,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        let now = unix_timestamp();
        entry.previous_status = Some(entry.status.clone());
        if entry.started_at.is_none() && bytes_transferred > entry.bytes_transferred {
            entry.started_at = Some(now);
            entry.start_offset = entry.bytes_transferred;
        }
        entry.status = truncate_utf8_bytes(status.to_owned(), MAX_TRANSFER_STATUS_BYTES);
        entry.bytes_transferred = bytes_transferred;
        if size.is_some() {
            entry.size = size;
        }
        entry.reason = bounded_transfer_reason(reason);
        if is_terminal_transfer_status(status) {
            entry.next_attempt_at = None;
        }
        entry.updated_at = now;
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_durable_mutation(Some(entry.clone()));
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn update_local_path(
        &mut self,
        id: u64,
        local_path: String,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.previous_status = None;
        entry.local_path = Some(truncate_utf8_bytes(
            local_path,
            MAX_TRANSFER_LOCAL_PATH_BYTES,
        ));
        entry.updated_at = unix_timestamp();
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_state_dirty();
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn update_progress(
        &mut self,
        id: u64,
        bytes_transferred: u64,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        if !is_active_transfer_status(&entry.status) {
            return None;
        }
        let now = unix_timestamp();
        entry.previous_status = Some(entry.status.clone());
        if entry.started_at.is_none() {
            entry.started_at = Some(now);
            entry.start_offset = entry.bytes_transferred;
        }
        entry.status = "in_progress".to_owned();
        entry.bytes_transferred = bytes_transferred;
        entry.reason = None;
        entry.next_attempt_at = None;
        entry.updated_at = now;
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_durable_mutation(Some(entry.clone()));
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn update_retry_state(
        &mut self,
        id: u64,
        attempt: u32,
        next_attempt_at: Option<u64>,
        status: &str,
        reason: Option<String>,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.previous_status = Some(entry.status.clone());
        entry.attempts = attempt.max(1);
        entry.next_attempt_at = next_attempt_at;
        entry.status = truncate_utf8_bytes(status.to_owned(), MAX_TRANSFER_STATUS_BYTES);
        entry.reason = bounded_transfer_reason(reason);
        entry.updated_at = unix_timestamp();
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_durable_mutation(Some(entry.clone()));
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn update_audio_metadata(
        &mut self,
        id: u64,
        metadata: AudioTechnicalMetadata,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.bit_rate = metadata.bit_rate.or(entry.bit_rate);
        entry.sample_rate = metadata.sample_rate.or(entry.sample_rate);
        entry.bit_depth = metadata.bit_depth.or(entry.bit_depth);
        entry.length_seconds = metadata.length_seconds.or(entry.length_seconds);
        entry.updated_at = unix_timestamp();
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_state_dirty();
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn pending_peer_transfer(&self, username: &str) -> Option<TransferEntry> {
        let username = bounded_transfer_username(username);
        self.entries
            .iter()
            .find(|entry| {
                entry.peer_username.as_deref() == Some(username.as_str())
                    && (entry.status == "peer_lookup" || entry.status == "peer_negotiating")
            })
            .cloned()
    }

    pub(super) fn pending_indirect_transfer(
        &self,
        username: &str,
        token: u32,
    ) -> Option<TransferEntry> {
        let username = bounded_transfer_username(username);
        self.entries
            .iter()
            .find(|entry| {
                entry.peer_username.as_deref() == Some(username.as_str())
                    && entry.token == token
                    && entry.status == "indirect_pending"
            })
            .cloned()
    }

    pub(super) fn pending_inbound_file_transfer(
        &self,
        token: Option<u32>,
    ) -> Option<TransferEntry> {
        self.entries
            .iter()
            .find(|entry| {
                entry.local_path.is_some()
                    && (entry.status == "accepted" || entry.status == "in_progress")
                    && token.is_none_or(|token| entry.token == token)
            })
            .cloned()
    }

    pub(super) fn accept_queued_inbound_negotiation(
        &mut self,
        id: u64,
        token: u32,
        size: Option<u64>,
    ) -> Result<Option<TransferEntry>, String> {
        if token == 0 {
            return Err("queued transfer token must be nonzero".to_owned());
        }
        if self.entries.iter().any(|entry| {
            entry.id != id && entry.token == token && !is_terminal_transfer_status(&entry.status)
        }) {
            return Err("queued transfer token is already active".to_owned());
        }
        let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.id == id
                && entry.direction == 0
                && entry.local_path.is_some()
                && entry.status == "peer_negotiating"
        }) else {
            return Ok(None);
        };
        entry.previous_status = Some(entry.status.clone());
        entry.token = token;
        if size.is_some() {
            entry.size = size;
        }
        entry.status = "accepted".to_owned();
        entry.reason = None;
        entry.updated_at = unix_timestamp();
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_durable_mutation(Some(entry.clone()));
        self.updated_at = unix_timestamp();
        Ok(Some(entry))
    }

    pub(super) fn accept_pending_remote_upload_request(
        &mut self,
        username: &str,
        filename: &str,
        token: u32,
        size: Option<u64>,
    ) -> Option<TransferEntry> {
        let username = bounded_transfer_username(username);
        let filename = bounded_transfer_filename(filename);
        let entry = self.entries.iter_mut().find(|entry| {
            entry.direction == 0
                && entry.peer_username.as_deref() == Some(username.as_str())
                && entry.filename == filename
                && entry.local_path.is_some()
                && entry.status == "queued"
                && entry
                    .reason
                    .as_deref()
                    .map(is_remote_queue_response)
                    .unwrap_or(false)
        })?;
        entry.previous_status = Some(entry.status.clone());
        entry.token = token;
        if size.is_some() {
            entry.size = size;
        }
        entry.status = "accepted".to_owned();
        entry.reason = None;
        entry.updated_at = unix_timestamp();
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_durable_mutation(Some(entry.clone()));
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn accept_pending_remote_download_request(
        &mut self,
        username: &str,
        filename: &str,
        token: u32,
        size: Option<u64>,
    ) -> Option<TransferEntry> {
        let username = bounded_transfer_username(username);
        let filename = bounded_transfer_filename(filename);
        let entry = self.entries.iter_mut().find(|entry| {
            entry.direction == 1
                && entry.peer_username.as_deref() == Some(username.as_str())
                && entry.filename == filename
                && entry.local_path.is_some()
                && entry.status == "queued"
                && entry
                    .reason
                    .as_deref()
                    .map(is_remote_queue_response)
                    .unwrap_or(false)
        })?;
        entry.previous_status = Some(entry.status.clone());
        entry.token = token;
        if size.is_some() {
            entry.size = size;
        }
        entry.status = "accepted".to_owned();
        entry.reason = None;
        entry.updated_at = unix_timestamp();
        entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        let entry = entry.clone();
        self.mark_durable_mutation(Some(entry.clone()));
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    pub(super) fn active_count_excluding(&self, id: Option<u64>) -> usize {
        self.entries
            .iter()
            .filter(|entry| id != Some(entry.id) && is_active_transfer_status(&entry.status))
            .count()
    }

    pub(super) fn push_entry(&mut self, entry: TransferEntry) -> TransferEntry {
        let entry = bounded_transfer_entry(entry);
        self.entries.push(entry.clone());
        if self.entries.len() > self.history_limit {
            let extra = self.entries.len() - self.history_limit;
            self.entries.drain(0..extra);
        }
        let retained_ids = self
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<std::collections::HashSet<_>>();
        self.progress_persisted_at
            .retain(|id, _| retained_ids.contains(id));
        self.updated_at = unix_timestamp();
        self.mark_durable_mutation(Some(entry.clone()));
        entry
    }

    pub(super) fn remove_entries(&mut self, ids: &[u64]) -> Vec<TransferEntry> {
        let mut removed = Vec::new();
        self.entries.retain(|entry| {
            if ids.contains(&entry.id) {
                removed.push(entry.clone());
                false
            } else {
                true
            }
        });
        if !removed.is_empty() {
            for entry in &removed {
                self.progress_persisted_at.remove(&entry.id);
            }
            self.updated_at = unix_timestamp();
            self.mark_state_dirty();
        }
        removed
    }

    pub(super) fn stats_json(&self) -> String {
        let queued = self
            .entries
            .iter()
            .filter(|entry| entry.status == "queued")
            .count();
        let in_progress = self
            .entries
            .iter()
            .filter(|entry| is_active_transfer_status(&entry.status))
            .count();
        let succeeded = self
            .entries
            .iter()
            .filter(|entry| is_successful_transfer_status(&entry.status))
            .count();
        let cancelled = self
            .entries
            .iter()
            .filter(|entry| entry.status == "cancelled")
            .count();
        let failed = self
            .entries
            .iter()
            .filter(|entry| is_failed_transfer_status(&entry.status))
            .count();
        let bytes_transferred = self
            .entries
            .iter()
            .map(|entry| entry.bytes_transferred)
            .sum::<u64>();
        format!(
             "{{\"total\":{},\"queued\":{},\"in_progress\":{},\"succeeded\":{},\"cancelled\":{},\"failed\":{},\"bytes_transferred\":{},\"updated_at\":{}}}",
             self.entries.len(),
             queued,
             in_progress,
             succeeded,
             cancelled,
             failed,
             bytes_transferred,
             self.updated_at
         )
    }

    pub(super) fn summary_json(&self) -> String {
        self.stats_json()
    }

    pub(super) fn controller_transfer_groups(
        &self,
        direction: u32,
        username: Option<&str>,
    ) -> Vec<serde_json::Value> {
        let mut grouped: BTreeMap<String, BTreeMap<String, Vec<&TransferEntry>>> = BTreeMap::new();
        for entry in self.entries.iter().filter(|entry| {
            entry.direction == direction
                && username.is_none_or(|username| entry.peer_username.as_deref() == Some(username))
        }) {
            grouped
                .entry(entry.peer_username.clone().unwrap_or_default())
                .or_default()
                .entry(transfer_directory_name(&entry.filename))
                .or_default()
                .push(entry);
        }

        grouped
            .into_iter()
            .map(|(username, directories)| {
                let directories = directories
                    .into_iter()
                    .map(|(directory, entries)| {
                        let files = entries
                            .into_iter()
                            .map(TransferEntry::controller_file_json)
                            .collect::<Vec<_>>();
                        serde_json::json!({
                            "directory": directory,
                            "fileCount": files.len(),
                            "files": files,
                        })
                    })
                    .collect::<Vec<_>>();
                serde_json::json!({
                    "username": username,
                    "directories": directories,
                })
            })
            .collect::<Vec<_>>()
    }

    pub(super) fn controller_transfers_json(
        &self,
        direction: u32,
        username: Option<&str>,
    ) -> String {
        serde_json::Value::Array(self.controller_transfer_groups(direction, username)).to_string()
    }

    pub(super) fn controller_transfer_user_json(
        &self,
        direction: u32,
        username: &str,
    ) -> Option<String> {
        self.controller_transfer_groups(direction, Some(username))
            .into_iter()
            .next()
            .map(|transfer| transfer.to_string())
    }

    pub(super) fn controller_transfer_json(
        &self,
        direction: u32,
        username: &str,
        id: u64,
    ) -> Option<String> {
        let entry = self.entries.iter().find(|entry| {
            entry.direction == direction
                && entry.id == id
                && entry.peer_username.as_deref() == Some(username)
        })?;
        Some(entry.controller_file_json().to_string())
    }

    pub(super) fn controller_transfer_position(
        &self,
        direction: u32,
        username: &str,
        id: u64,
    ) -> usize {
        for (position, entry) in self
            .entries
            .iter()
            .filter(|entry| {
                entry.direction == direction
                    && entry.peer_username.as_deref() == Some(username)
                    && (entry.status == "queued" || is_active_transfer_status(&entry.status))
            })
            .enumerate()
        {
            if entry.id == id {
                return position;
            }
        }
        0
    }

    pub(super) fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let entries = self
            .entries
            .iter()
            .filter(|entry| {
                filter
                    .status
                    .as_deref()
                    .is_none_or(|status| entry.status == status)
            })
            .filter(|entry| {
                filter
                    .direction
                    .as_deref()
                    .and_then(|direction| direction.parse::<u32>().ok())
                    .is_none_or(|direction| entry.direction == direction)
            })
            .filter(|entry| {
                filter
                    .username
                    .as_deref()
                    .is_none_or(|username| entry.peer_username.as_deref() == Some(username))
            })
            .filter(|entry| {
                filter.q.as_deref().is_none_or(|q| {
                    entry.filename.to_ascii_lowercase().contains(q)
                        || entry
                            .peer_username
                            .as_deref()
                            .is_some_and(|username| username.to_ascii_lowercase().contains(q))
                })
            })
            .collect::<Vec<_>>();
        let filtered_count = entries.len();
        let entries = entries
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(TransferEntry::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"history_limit\":{},\"events_file\":\"{}\",\"events_error\":{},\"state_file\":\"{}\",\"state_error\":{},\"updated_at\":{}}}",
            entries,
            self.entries.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.history_limit,
            json_escape(
                self.events_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("transfer-events.tsv")
            ),
            json_option(public_transfer_events_error(self.events_error.as_deref())),
            json_escape(
                self.state_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("transfer-state.json")
            ),
            json_option(public_transfer_state_error(self.state_error.as_deref())),
            self.updated_at
        )
    }
}
