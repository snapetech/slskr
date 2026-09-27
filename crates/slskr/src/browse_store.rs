use super::{
    browse_path_state::BrowseEntry, group_browse_entries, json_escape, json_option,
    json_u32_option, json_u64_option, json_usize_option, truncate_utf8_bytes, unix_timestamp,
    AppState, RecordListFilter,
};

pub(super) const MAX_BROWSE_RECORDS: usize = 1_024;
pub(super) const MAX_BROWSE_ENTRIES_PER_USER: usize = 10_000;
pub(super) const MAX_TOTAL_BROWSE_ENTRIES: usize = 50_000;
pub(super) const MAX_BROWSE_USERNAME_BYTES: usize = 1024;
pub(super) const MAX_BROWSE_FILENAME_BYTES: usize = 4 * 1024;
pub(super) const MAX_BROWSE_EXTENSION_BYTES: usize = 256;
pub(super) const MAX_BROWSE_REASON_BYTES: usize = 4 * 1024;
pub(super) const MAX_BROWSE_FOLDER_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowseRecord {
    pub(crate) username: String,
    pub(crate) status: &'static str,
    pub(crate) entries: Vec<BrowseEntry>,
    pub(crate) reason: Option<String>,
    pub(crate) folder: Option<String>,
    pub(crate) indirect_token: Option<u32>,
    pub(crate) requested_at: Option<u64>,
    pub(crate) updated_at: u64,
}

impl BrowseRecord {
    pub(crate) fn json(&self) -> String {
        let entries = self
            .entries
            .iter()
            .map(BrowseEntry::json)
            .collect::<Vec<_>>()
            .join(",");
        let total_bytes = self.entries.iter().map(|entry| entry.size).sum::<u64>();
        format!(
            "{{\"username\":\"{}\",\"status\":\"{}\",\"entries\":[{}],\"count\":{},\"total_bytes\":{},\"reason\":{},\"folder\":{},\"indirect_token\":{},\"requested_at\":{},\"updated_at\":{}}}",
            json_escape(&self.username),
            self.status,
            entries,
            self.entries.len(),
            total_bytes,
            json_option(public_browse_reason(self.status, self.reason.as_deref())),
            json_option(self.folder.as_deref()),
            json_u32_option(self.indirect_token),
            json_u64_option(self.requested_at),
            self.updated_at
        )
    }

    pub(crate) fn controller_status_json(&self) -> String {
        let size = self.entries.iter().map(|entry| entry.size).sum::<u64>();
        let complete = matches!(self.status, "ready" | "partial");
        let percent_complete = if complete { 100.0 } else { 0.0 };
        let directory_count = group_browse_entries(&self.entries).len();
        serde_json::json!({
            "username": self.username,
            "status": self.status,
            "state": browse_controller_state(self.status),
            "size": size,
            "bytesTransferred": if complete { size } else { 0 },
            "bytesRemaining": if complete { 0 } else { size },
            "percentComplete": percent_complete,
            "fileCount": self.entries.len(),
            "directoryCount": directory_count,
            "isComplete": complete,
            "reason": public_browse_reason(self.status, self.reason.as_deref()),
            "folder": self.folder,
            "indirectToken": self.indirect_token,
            "requestedAt": self.requested_at,
            "updatedAt": self.updated_at,
        })
        .to_string()
    }
}

fn public_browse_reason(status: &str, reason: Option<&str>) -> Option<&'static str> {
    reason.map(|_| match status {
        "failed" => "browse failed",
        "cancelled" => "browse cancelled",
        "indirect_pending" => "direct browse failed; indirect request pending",
        _ => "browse operation unavailable",
    })
}

fn browse_controller_state(status: &str) -> &'static str {
    match status {
        "ready" | "partial" => "Completed",
        "failed" => "Failed",
        "cancelled" => "Cancelled",
        "indirect_pending" => "Pending",
        _ => "InProgress",
    }
}

fn persisted_browse_status(status: &str) -> &'static str {
    match status {
        "requested" => "requested",
        "ready" => "ready",
        "partial" => "partial",
        "failed" => "failed",
        "cancelled" => "cancelled",
        "indirect_pending" => "indirect_pending",
        _ => "failed",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowseStore {
    pub(crate) records: Vec<BrowseRecord>,
    pub(crate) next_indirect_token: u32,
    pub(crate) updated_at: u64,
    pub(crate) max_records: usize,
    pub(crate) max_entries_per_user: usize,
}

impl BrowseStore {
    pub(crate) fn new() -> Self {
        Self::with_limits(MAX_BROWSE_RECORDS, MAX_BROWSE_ENTRIES_PER_USER)
    }

    pub(crate) fn with_limits(max_records: usize, max_entries_per_user: usize) -> Self {
        Self {
            records: Vec::new(),
            next_indirect_token: 1,
            updated_at: unix_timestamp(),
            max_records: max_records.max(1),
            max_entries_per_user: max_entries_per_user.max(1),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn from_persisted(records: Vec<crate::persistence::BrowseRecord>) -> Self {
        let mut next_indirect_token = 1_u32;
        let mut updated_at = unix_timestamp();
        let mut seen_usernames = std::collections::HashSet::new();
        let mut total_entries = 0_usize;
        let records = records
            .into_iter()
            .map(|mut record| {
                record.username = bounded_browse_username(&record.username);
                record
            })
            .filter(|record| seen_usernames.insert(record.username.clone()))
            .take(MAX_BROWSE_RECORDS)
            .map(|record| {
                let remaining = MAX_TOTAL_BROWSE_ENTRIES.saturating_sub(total_entries);
                let entries = serde_json::from_str::<serde_json::Value>(&record.entries_json)
                    .ok()
                    .and_then(|value| value.as_array().cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|entry| BrowseEntry::from_json_file(entry, None))
                    .take(MAX_BROWSE_ENTRIES_PER_USER.min(remaining))
                    .collect::<Vec<_>>();
                total_entries = total_entries.saturating_add(entries.len());
                let indirect_token = record
                    .indirect_token
                    .and_then(|token| u32::try_from(token).ok());
                if let Some(token) = indirect_token {
                    next_indirect_token = next_indirect_token.max(token.wrapping_add(1).max(1));
                }
                let requested_at = record
                    .requested_at
                    .and_then(|timestamp| u64::try_from(timestamp).ok());
                let record_updated_at =
                    u64::try_from(record.updated_at).unwrap_or_else(|_| unix_timestamp());
                updated_at = updated_at.max(record_updated_at);
                BrowseRecord {
                    username: record.username,
                    status: persisted_browse_status(&record.status),
                    entries,
                    reason: record
                        .reason
                        .map(|reason| truncate_utf8_bytes(reason, MAX_BROWSE_REASON_BYTES)),
                    folder: record
                        .folder
                        .map(|folder| truncate_utf8_bytes(folder, MAX_BROWSE_FOLDER_BYTES)),
                    indirect_token,
                    requested_at,
                    updated_at: record_updated_at,
                }
            })
            .collect();
        Self {
            records,
            next_indirect_token,
            updated_at,
            max_records: MAX_BROWSE_RECORDS,
            max_entries_per_user: MAX_BROWSE_ENTRIES_PER_USER,
        }
    }

    pub(crate) fn next_indirect_token(&mut self) -> u32 {
        let mut candidate = self.next_indirect_token.max(1);
        for _ in 0..=self.records.len() {
            if !self
                .records
                .iter()
                .any(|record| record.indirect_token == Some(candidate))
            {
                self.next_indirect_token = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded browse store must leave an available u32 token")
    }

    pub(crate) fn request(&mut self, username: String) -> Option<BrowseRecord> {
        let username = bounded_browse_username(&username);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = "requested";
            record.reason = None;
            record.folder = None;
            record.indirect_token = None;
            record.requested_at = Some(now);
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = BrowseRecord {
            username,
            status: "requested",
            entries: Vec::new(),
            reason: None,
            folder: None,
            indirect_token: None,
            requested_at: Some(now),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn request_folder(
        &mut self,
        username: String,
        folder: String,
    ) -> Option<BrowseRecord> {
        let username = bounded_browse_username(&username);
        let folder = truncate_utf8_bytes(folder, MAX_BROWSE_FOLDER_BYTES);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = "requested";
            record.entries.clear();
            record.reason = None;
            record.folder = Some(folder);
            record.indirect_token = None;
            record.requested_at = Some(now);
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = BrowseRecord {
            username,
            status: "requested",
            entries: Vec::new(),
            reason: None,
            folder: Some(folder),
            indirect_token: None,
            requested_at: Some(now),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn requested_folder(&self, username: &str) -> Option<Option<String>> {
        let username = bounded_browse_username(username);
        self.records
            .iter()
            .find(|record| record.username == username && record.status == "requested")
            .map(|record| record.folder.clone())
    }

    pub(crate) fn mark_indirect_pending(&mut self, username: &str, reason: String) -> Option<u32> {
        let username = bounded_browse_username(username);
        let reason = truncate_utf8_bytes(reason, MAX_BROWSE_REASON_BYTES);
        let index = self
            .records
            .iter()
            .position(|record| record.username == username && record.status == "requested")?;
        let token = self.next_indirect_token();
        let now = unix_timestamp();
        let record = &mut self.records[index];
        record.status = "indirect_pending";
        record.reason = Some(reason);
        record.indirect_token = Some(token);
        record.updated_at = now;
        self.updated_at = now;
        Some(token)
    }

    pub(crate) fn pending_indirect(&self, username: &str, token: u32) -> Option<Option<String>> {
        let username = bounded_browse_username(username);
        self.records
            .iter()
            .find(|record| {
                record.username == username
                    && record.status == "indirect_pending"
                    && record.indirect_token == Some(token)
            })
            .map(|record| record.folder.clone())
    }

    pub(crate) fn fail_indirect(&mut self, token: u32, reason: String) -> Option<BrowseRecord> {
        let reason = truncate_utf8_bytes(reason, MAX_BROWSE_REASON_BYTES);
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| {
            record.status == "indirect_pending" && record.indirect_token == Some(token)
        })?;
        record.status = "failed";
        record.reason = Some(reason);
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn add_entries(
        &mut self,
        username: String,
        entries: Vec<BrowseEntry>,
        complete: bool,
    ) -> Option<BrowseRecord> {
        let username = bounded_browse_username(&username);
        let total_entries = self.total_entries();
        let now = unix_timestamp();
        let status = if complete { "ready" } else { "partial" };
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = status;
            let per_user_remaining = self
                .max_entries_per_user
                .saturating_sub(record.entries.len());
            let aggregate_remaining = MAX_TOTAL_BROWSE_ENTRIES.saturating_sub(total_entries);
            record.entries.extend(
                entries
                    .into_iter()
                    .take(per_user_remaining.min(aggregate_remaining))
                    .map(bounded_browse_entry),
            );
            record.reason = None;
            record.indirect_token = None;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let entries = entries
            .into_iter()
            .take(
                self.max_entries_per_user
                    .min(MAX_TOTAL_BROWSE_ENTRIES.saturating_sub(total_entries)),
            )
            .map(bounded_browse_entry)
            .collect();
        let record = BrowseRecord {
            username,
            status,
            entries,
            reason: None,
            folder: None,
            indirect_token: None,
            requested_at: None,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn fail(&mut self, username: String, reason: String) -> Option<BrowseRecord> {
        let username = bounded_browse_username(&username);
        let reason = truncate_utf8_bytes(reason, MAX_BROWSE_REASON_BYTES);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = "failed";
            record.reason = Some(reason);
            record.indirect_token = None;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = BrowseRecord {
            username,
            status: "failed",
            entries: Vec::new(),
            reason: Some(reason),
            folder: None,
            indirect_token: None,
            requested_at: None,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn cancel(&mut self, username: String, reason: String) -> Option<BrowseRecord> {
        let username = bounded_browse_username(&username);
        let reason = truncate_utf8_bytes(reason, MAX_BROWSE_REASON_BYTES);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = "cancelled";
            record.reason = if reason.trim().is_empty() {
                None
            } else {
                Some(reason)
            };
            record.indirect_token = None;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = BrowseRecord {
            username,
            status: "cancelled",
            entries: Vec::new(),
            reason: if reason.trim().is_empty() {
                None
            } else {
                Some(reason)
            },
            folder: None,
            indirect_token: None,
            requested_at: None,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn get(&self, username: &str) -> Option<BrowseRecord> {
        let username = bounded_browse_username(username);
        self.records
            .iter()
            .find(|record| record.username == username)
            .cloned()
    }

    pub(crate) fn total_entries(&self) -> usize {
        self.records.iter().map(|record| record.entries.len()).sum()
    }

    pub(crate) fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .status
                    .as_deref()
                    .is_none_or(|status| record.status == status)
            })
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .is_none_or(|q| record.username.to_ascii_lowercase().contains(q))
            })
            .collect::<Vec<_>>();
        let filtered_count = records.len();
        let records = records
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(BrowseRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    pub(crate) fn summary_json(&self) -> String {
        let requested = self
            .records
            .iter()
            .filter(|record| record.status == "requested")
            .count();
        let ready = self
            .records
            .iter()
            .filter(|record| record.status == "ready")
            .count();
        let partial = self
            .records
            .iter()
            .filter(|record| record.status == "partial")
            .count();
        let indirect_pending = self
            .records
            .iter()
            .filter(|record| record.status == "indirect_pending")
            .count();
        let failed = self
            .records
            .iter()
            .filter(|record| record.status == "failed")
            .count();
        let files = self
            .records
            .iter()
            .map(|record| record.entries.len())
            .sum::<usize>();
        let bytes = self
            .records
            .iter()
            .flat_map(|record| record.entries.iter())
            .map(|entry| entry.size)
            .sum::<u64>();
        format!(
            "{{\"total\":{},\"requested\":{},\"indirect_pending\":{},\"partial\":{},\"ready\":{},\"failed\":{},\"files\":{},\"bytes\":{},\"updated_at\":{}}}",
            self.records.len(),
            requested,
            indirect_pending,
            partial,
            ready,
            failed,
            files,
            bytes,
            self.updated_at
        )
    }
}

fn bounded_browse_username(username: &str) -> String {
    truncate_utf8_bytes(username.to_owned(), MAX_BROWSE_USERNAME_BYTES)
}

pub(crate) fn bounded_browse_entry(mut entry: BrowseEntry) -> BrowseEntry {
    entry.filename = truncate_utf8_bytes(entry.filename, MAX_BROWSE_FILENAME_BYTES);
    entry.extension = truncate_utf8_bytes(entry.extension, MAX_BROWSE_EXTENSION_BYTES);
    entry
}

pub(super) async fn persist_browse_record_checked(
    state: &AppState,
    record: &BrowseRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let entries_json = serde_json::Value::Array(
        record
            .entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "filename": entry.filename,
                    "size": entry.size,
                    "extension": entry.extension,
                })
            })
            .collect(),
    )
    .to_string();
    let persisted = crate::persistence::BrowseRecord {
        username: record.username.clone(),
        status: record.status.to_owned(),
        entries_json,
        reason: record.reason.clone(),
        folder: record.folder.clone(),
        indirect_token: record.indirect_token.map(i64::from),
        requested_at: record
            .requested_at
            .map(|timestamp| i64::try_from(timestamp).unwrap_or(i64::MAX)),
        updated_at: i64::try_from(record.updated_at).unwrap_or(i64::MAX),
    };
    db.upsert_browse_record(&persisted)
        .await
        .map_err(|error| format!("browse persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn rollback_browse_if_unchanged(
    state: &AppState,
    previous: BrowseStore,
    mutated: &BrowseStore,
) {
    let mut browse = state.browse.write().await;
    if *browse == *mutated {
        *browse = previous;
    }
}
