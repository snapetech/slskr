use super::persistence;
use super::routing::{self, HttpResponse};
use super::search_fallback;
use super::{
    alternate_size_is_eligible, json_bool_option, json_escape, json_option, json_u32_option,
    json_usize_option, normalize_search_status, persisted_search_status, persisted_target,
    search_state_for_status, truncate_utf8_bytes, unix_seconds_rfc3339, unix_timestamp,
    virtual_basename, FileEntry, FileSearchResponse, RecordListFilter, TransferEntry,
    WishlistIgnoredResult, WishlistResultPolicy, DEFAULT_LIST_LIMIT, DEFAULT_SEARCH_TTL_SECONDS,
    MAX_SEARCH_QUERY_BYTES, MAX_SEARCH_RECORDS, MAX_SEARCH_RESULTS_PER_SEARCH,
    MAX_SEARCH_RESULT_EXTENSION_BYTES, MAX_SEARCH_RESULT_FILENAME_BYTES,
    MAX_SEARCH_RESULT_USERNAME_BYTES, MAX_SEARCH_TARGET_NAME_BYTES, MAX_TOTAL_SEARCH_RESULTS,
};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SearchResultEntry {
    pub(super) peer_username: Option<String>,
    pub(super) filename: String,
    pub(super) size: u64,
    pub(super) extension: String,
    pub(super) bit_rate: Option<u32>,
    pub(super) sample_rate: Option<u32>,
    pub(super) bit_depth: Option<u32>,
    pub(super) length_seconds: Option<u32>,
    pub(super) locked: bool,
    pub(super) slot_free: Option<bool>,
    pub(super) average_speed: Option<u32>,
    pub(super) queue_length: Option<u32>,
}

impl SearchResultEntry {
    pub(super) fn from_json_file(
        file: &serde_json::Value,
        peer_username: Option<&str>,
        locked: bool,
        slot_free: Option<bool>,
        average_speed: Option<u32>,
        queue_length: Option<u32>,
    ) -> Option<Self> {
        let filename = file.get("filename")?.as_str()?.to_owned();
        let extension = file
            .get("extension")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| filename.split('.').next_back().unwrap_or("").to_owned());
        let attribute = |names: &[&str]| {
            names.iter().find_map(|name| {
                file.get(*name)
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
            })
        };
        Some(Self {
            peer_username: peer_username.map(|username| {
                truncate_utf8_bytes(username.to_owned(), MAX_SEARCH_RESULT_USERNAME_BYTES)
            }),
            filename: truncate_utf8_bytes(filename, MAX_SEARCH_RESULT_FILENAME_BYTES),
            size: file
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            extension: truncate_utf8_bytes(extension, MAX_SEARCH_RESULT_EXTENSION_BYTES),
            bit_rate: attribute(&["bitRate", "bit_rate", "bitrate"]),
            sample_rate: attribute(&["sampleRate", "sample_rate", "samplerate"]),
            bit_depth: attribute(&["bitDepth", "bit_depth", "bitdepth"]),
            length_seconds: attribute(&["length", "lengthSeconds", "length_seconds"]),
            locked: file
                .get("locked")
                .or_else(|| file.get("isLocked"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(locked),
            slot_free,
            average_speed,
            queue_length,
        })
    }

    fn from_file_entry(entry: &FileEntry) -> Self {
        bounded_search_result_entry(Self {
            peer_username: None,
            filename: entry.filename.clone(),
            size: entry.size,
            extension: entry.extension.clone(),
            bit_rate: file_attribute_value(entry, 0),
            sample_rate: file_attribute_value(entry, 4),
            bit_depth: file_attribute_value(entry, 5),
            length_seconds: file_attribute_value(entry, 1),
            locked: false,
            slot_free: Some(true),
            average_speed: Some(0),
            queue_length: Some(0),
        })
    }

    fn from_peer_response_entry(
        response: &FileSearchResponse,
        entry: &FileEntry,
        locked: bool,
    ) -> Self {
        bounded_search_result_entry(Self {
            peer_username: Some(response.username.clone()),
            filename: entry.filename.clone(),
            size: entry.size,
            extension: entry.extension.clone(),
            bit_rate: file_attribute_value(entry, 0),
            sample_rate: file_attribute_value(entry, 4),
            bit_depth: file_attribute_value(entry, 5),
            length_seconds: file_attribute_value(entry, 1),
            locked,
            slot_free: Some(response.slot_free),
            average_speed: Some(response.average_speed),
            queue_length: Some(response.queue_length),
        })
    }

    fn from_persisted(record: &persistence::SearchResultRecord) -> Self {
        bounded_search_result_entry(Self {
            peer_username: record.peer_username.clone(),
            filename: record.filename.clone(),
            size: record.size.max(0) as u64,
            extension: record.extension.clone(),
            bit_rate: record.bit_rate.and_then(|value| u32::try_from(value).ok()),
            sample_rate: record
                .sample_rate
                .and_then(|value| u32::try_from(value).ok()),
            bit_depth: record.bit_depth.and_then(|value| u32::try_from(value).ok()),
            length_seconds: record
                .length_seconds
                .and_then(|value| u32::try_from(value).ok()),
            locked: record.locked,
            slot_free: record.slot_free,
            average_speed: record
                .average_speed
                .and_then(|value| u32::try_from(value).ok()),
            queue_length: record
                .queue_length
                .and_then(|value| u32::try_from(value).ok()),
        })
    }

    pub(super) fn json(&self) -> String {
        format!(
            "{{\"peer_username\":{},\"filename\":\"{}\",\"size\":{},\"extension\":\"{}\",\"bit_rate\":{},\"sample_rate\":{},\"bit_depth\":{},\"length_seconds\":{},\"locked\":{},\"slot_free\":{},\"average_speed\":{},\"queue_length\":{}}}",
            json_option(self.peer_username.as_deref()),
            json_escape(&self.filename),
            self.size,
            json_escape(&self.extension),
            json_u32_option(self.bit_rate),
            json_u32_option(self.sample_rate),
            json_u32_option(self.bit_depth),
            json_u32_option(self.length_seconds),
            self.locked,
            json_bool_option(self.slot_free),
            json_u32_option(self.average_speed),
            json_u32_option(self.queue_length)
        )
    }

    pub(super) fn controller_file_json(&self) -> serde_json::Value {
        serde_json::json!({
            "filename": self.filename,
            "size": self.size,
            "code": 1,
            "isLocked": self.locked || !self.slot_free.unwrap_or(true),
            "username": self.peer_username.as_deref().unwrap_or_default(),
            "extension": self.extension,
            "bitRate": self.bit_rate,
            "bitDepth": self.bit_depth,
            "length": self.length_seconds,
            "sampleRate": self.sample_rate,
        })
    }

    fn controller_file_json_with_index(&self, result_index: usize) -> serde_json::Value {
        let mut value = self.controller_file_json();
        value["resultIndex"] = serde_json::json!(result_index);
        value
    }
}

pub(super) fn file_attribute_value(entry: &FileEntry, code: u32) -> Option<u32> {
    entry
        .attributes
        .iter()
        .find(|attribute| attribute.code == code)
        .map(|attribute| attribute.value)
}

pub(super) fn bounded_search_result_entry(mut entry: SearchResultEntry) -> SearchResultEntry {
    entry.peer_username = entry
        .peer_username
        .map(|username| truncate_utf8_bytes(username, MAX_SEARCH_RESULT_USERNAME_BYTES));
    entry.filename = truncate_utf8_bytes(entry.filename, MAX_SEARCH_RESULT_FILENAME_BYTES);
    entry.extension = truncate_utf8_bytes(entry.extension, MAX_SEARCH_RESULT_EXTENSION_BYTES);
    entry
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchRecord {
    pub(crate) id: String,
    pub(crate) token: u32,
    pub(crate) query: String,
    pub(crate) target: &'static str,
    pub(crate) target_name: Option<String>,
    pub(crate) status: &'static str,
    pub(crate) results: Vec<SearchResultEntry>,
    pub(crate) raw_response_count: usize,
    pub(crate) filtered_out_count: usize,
    pub(crate) ignored_result_count: usize,
    pub(crate) hidden_locked_count: usize,
    pub(crate) fallback_attempts: usize,
    pub(crate) ttl_seconds: u64,
    pub(crate) expires_at: u64,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
}

#[allow(dead_code)]
impl SearchRecord {
    pub(crate) fn wishlist_item_id(&self) -> Option<&str> {
        (self.target == "wishlist")
            .then_some(self.target_name.as_deref())
            .flatten()
    }

    pub(crate) fn extend_results_with_limit_bounded(
        &mut self,
        results: impl IntoIterator<Item = SearchResultEntry>,
        aggregate_remaining: usize,
        result_limit: usize,
    ) -> Vec<SearchResultEntry> {
        let remaining = result_limit
            .min(MAX_SEARCH_RESULTS_PER_SEARCH)
            .saturating_sub(self.results.len())
            .min(aggregate_remaining);
        if remaining == 0 {
            return Vec::new();
        }
        let mut seen = self
            .results
            .iter()
            .map(search_result_identity)
            .collect::<HashSet<_>>();
        let accepted = results
            .into_iter()
            .filter(|result| seen.insert(search_result_identity(result)))
            .take(remaining)
            .collect::<Vec<_>>();
        self.results.extend(accepted.iter().cloned());
        accepted
    }

    pub(crate) fn json(&self) -> String {
        self.json_with_result_page(0, None)
    }

    pub(crate) fn history_summary(&self) -> serde_json::Value {
        let response_count = self
            .results
            .iter()
            .filter_map(|entry| entry.peer_username.as_deref())
            .collect::<HashSet<_>>()
            .len();
        let locked_file_count = self.results.iter().filter(|entry| entry.locked).count();
        serde_json::json!({
            "id": self.id,
            "token": self.token,
            "query": self.query,
            "searchText": self.query,
            "target": self.target,
            "target_name": self.target_name,
            "wishlistItemId": self.wishlist_item_id(),
            "status": self.status,
            "state": search_state_for_status(self.status),
            "isComplete": self.status != "active",
            "result_count": self.results.len(),
            "fileCount": self.results.len(),
            "lockedFileCount": locked_file_count,
            "responseCount": response_count,
            "responsesAvailable": self.status != "active" || !self.results.is_empty(),
            "rawResponseCount": self.raw_response_count,
            "filteredOutCount": self.filtered_out_count,
            "ignoredResultCount": self.ignored_result_count,
            "hiddenLockedCount": self.hidden_locked_count,
            "fallbackAttempts": self.fallback_attempts,
            "startedAt": unix_seconds_rfc3339(self.created_at),
            "endedAt": (self.status != "active").then(|| unix_seconds_rfc3339(self.updated_at)),
            "expires_at": self.expires_at,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        })
    }

    pub(crate) fn json_with_query(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        self.json_with_result_page(filter.offset, filter.limit)
    }

    pub(crate) fn json_with_result_page(&self, offset: usize, limit: Option<usize>) -> String {
        let results = self
            .results
            .iter()
            .skip(offset)
            .take(limit.unwrap_or(usize::MAX))
            .map(SearchResultEntry::json)
            .collect::<Vec<_>>()
            .join(",");
        let responses = self.controller_responses_json();
        let response_count = self
            .results
            .iter()
            .filter_map(|entry| entry.peer_username.as_deref())
            .collect::<HashSet<_>>()
            .len();
        let locked_file_count = self.results.iter().filter(|entry| entry.locked).count();
        let state = search_state_for_status(self.status);
        let started_at = unix_seconds_rfc3339(self.created_at);
        let ended_at = if self.status == "active" {
            "null".to_owned()
        } else {
            format!(
                "\"{}\"",
                json_escape(&unix_seconds_rfc3339(self.updated_at))
            )
        };
        format!(
            "{{\"id\":\"{}\",\"token\":{},\"query\":\"{}\",\"searchText\":\"{}\",\"target\":\"{}\",\"target_name\":{},\"wishlistItemId\":{},\"status\":\"{}\",\"state\":\"{}\",\"isComplete\":{},\"result_count\":{},\"fileCount\":{},\"lockedFileCount\":{},\"responseCount\":{},\"responsesAvailable\":{},\"rawResponseCount\":{},\"filteredOutCount\":{},\"ignoredResultCount\":{},\"hiddenLockedCount\":{},\"fallbackAttempts\":{},\"responses\":{},\"results\":[{}],\"resultOffset\":{},\"resultLimit\":{},\"startedAt\":\"{}\",\"endedAt\":{},\"expires_at\":{},\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            self.token,
            json_escape(&self.query),
            json_escape(&self.query),
            self.target,
            json_option(self.target_name.as_deref()),
            json_option(self.wishlist_item_id()),
            self.status,
            state,
            self.status != "active",
            self.results.len(),
            self.results.len(),
            locked_file_count,
            response_count,
            self.status != "active" || !self.results.is_empty(),
            self.raw_response_count,
            self.filtered_out_count,
            self.ignored_result_count,
            self.hidden_locked_count,
            self.fallback_attempts,
            responses,
            results,
            offset,
            json_usize_option(limit),
            started_at,
            ended_at,
            self.expires_at,
            self.created_at,
            self.updated_at
        )
    }

    pub(crate) fn controller_responses_json(&self) -> String {
        self.controller_responses_json_with_query(None)
    }

    pub(crate) fn controller_responses_json_with_query(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let mut grouped: BTreeMap<String, Vec<(usize, &SearchResultEntry)>> = BTreeMap::new();
        for (result_index, result) in self.results.iter().enumerate() {
            let username = result.peer_username.clone().unwrap_or_default();
            if filter
                .username
                .as_deref()
                .is_some_and(|needle| !username.eq_ignore_ascii_case(needle))
            {
                continue;
            }
            if filter.q.as_deref().is_some_and(|needle| {
                !username.to_ascii_lowercase().contains(needle)
                    && !result.filename.to_ascii_lowercase().contains(needle)
            }) {
                continue;
            }
            grouped
                .entry(username)
                .or_default()
                .push((result_index, result));
        }
        let responses = grouped
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(|(username, entries)| {
                let files = entries
                    .iter()
                    .filter(|(_, entry)| !entry.locked)
                    .map(|(result_index, entry)| {
                        entry.controller_file_json_with_index(*result_index)
                    })
                    .collect::<Vec<_>>();
                let locked_files = entries
                    .iter()
                    .filter(|(_, entry)| entry.locked)
                    .map(|(result_index, entry)| {
                        entry.controller_file_json_with_index(*result_index)
                    })
                    .collect::<Vec<_>>();
                let file_count = files.len();
                let locked_file_count = locked_files.len();
                let first = entries.first().map(|(_, entry)| *entry);
                serde_json::json!({
                    "username": username,
                    "token": self.token,
                    "hasFreeUploadSlot": first.and_then(|entry| entry.slot_free).unwrap_or(true),
                    "queueLength": first.and_then(|entry| entry.queue_length).unwrap_or(0),
                    "uploadSpeed": first.and_then(|entry| entry.average_speed).unwrap_or(0),
                    "fileCount": file_count,
                    "files": files,
                    "lockedFileCount": locked_file_count,
                    "lockedFiles": locked_files,
                })
            })
            .collect::<Vec<_>>();
        serde_json::Value::Array(responses).to_string()
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    pub(crate) fn from_persisted(record: &persistence::SearchRecord) -> Option<Self> {
        Self::from_persisted_with_results(record, Vec::new())
    }

    pub(crate) fn from_persisted_with_results(
        record: &persistence::SearchRecord,
        result_records: Vec<persistence::SearchResultRecord>,
    ) -> Option<Self> {
        Self::from_persisted_with_results_and_id(record, result_records, None)
    }

    pub(crate) fn from_persisted_with_results_and_id(
        record: &persistence::SearchRecord,
        result_records: Vec<persistence::SearchResultRecord>,
        external_id: Option<&str>,
    ) -> Option<Self> {
        let token = record.id.parse::<u32>().ok()?;
        let target = persisted_target(record.target.as_deref());
        let target_name = match target {
            "user" => record.target.clone(),
            "room" => record.room.clone(),
            "wishlist" => record.room.clone(),
            _ => None,
        };
        Some(Self {
            id: external_id
                .filter(|id| !id.trim().is_empty())
                .unwrap_or(&record.id)
                .to_owned(),
            token,
            query: truncate_utf8_bytes(record.query.clone(), MAX_SEARCH_QUERY_BYTES),
            target,
            target_name: target_name
                .map(|name| truncate_utf8_bytes(name, MAX_SEARCH_TARGET_NAME_BYTES)),
            status: if matches!(record.status.as_str(), "active" | "pending") {
                "expired"
            } else {
                persisted_search_status(&record.status)
            },
            results: if matches!(record.status.as_str(), "active" | "pending") {
                Vec::new()
            } else {
                result_records
                    .iter()
                    .map(SearchResultEntry::from_persisted)
                    .take(MAX_SEARCH_RESULTS_PER_SEARCH)
                    .collect()
            },
            raw_response_count: 0,
            filtered_out_count: 0,
            ignored_result_count: 0,
            hidden_locked_count: 0,
            fallback_attempts: usize::try_from(record.fallback_attempts.max(0)).unwrap_or_default(),
            ttl_seconds: DEFAULT_SEARCH_TTL_SECONDS,
            expires_at: 0,
            created_at: record.created_at.max(0) as u64,
            updated_at: record.completed_at.unwrap_or(record.created_at).max(0) as u64,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchStore {
    pub(crate) records: Vec<SearchRecord>,
    pub(crate) next_token: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SearchCreateError {
    CapacityFull,
    DuplicateId,
    InvalidQuery,
    TokenSpaceExhausted,
}

#[derive(Debug)]
pub(crate) struct SearchCreateOutcome {
    pub(crate) record: SearchRecord,
    pub(crate) evicted: Vec<SearchRecord>,
    pub(crate) expired: Vec<SearchRecord>,
}

fn search_identifier_matches(record: &SearchRecord, identifier: &str) -> bool {
    if record.id == identifier || record.token.to_string() == identifier {
        return true;
    }

    // The versioned controller returns UUID-backed searchId values without
    // dashes. Keep that compatibility identifier usable for every subsequent
    // read and mutation without changing the canonical stored id.
    !identifier.contains('-') && record.id.contains('-') && record.id.replace('-', "") == identifier
}

fn next_search_token_hint(records: &[SearchRecord]) -> u32 {
    records
        .iter()
        .map(|record| record.token)
        .max()
        .unwrap_or(0)
        .wrapping_add(1)
        .max(1)
}

pub(crate) fn search_create_error_response(error: SearchCreateError) -> HttpResponse {
    match error {
        SearchCreateError::DuplicateId => routing::conflict_response("search id already exists"),
        SearchCreateError::InvalidQuery => {
            routing::bad_request_response("search query must not be blank")
        }
        SearchCreateError::CapacityFull => {
            routing::service_unavailable_response("active search capacity is full")
        }
        SearchCreateError::TokenSpaceExhausted => {
            routing::service_unavailable_response("search token space is exhausted")
        }
    }
}

#[allow(dead_code)]
impl SearchStore {
    pub(crate) fn new() -> Self {
        Self {
            records: Vec::new(),
            next_token: 1,
        }
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    pub(crate) fn from_persisted(records: Vec<persistence::SearchRecord>) -> Self {
        let parsed = records
            .iter()
            .filter_map(SearchRecord::from_persisted)
            .collect::<Vec<_>>();
        let next_token = next_search_token_hint(&parsed);
        let mut seen_ids = std::collections::HashSet::new();
        let mut seen_tokens = std::collections::HashSet::new();
        let records = parsed
            .into_iter()
            .filter(|record| seen_ids.insert(record.id.clone()) && seen_tokens.insert(record.token))
            .take(MAX_SEARCH_RECORDS)
            .collect::<Vec<_>>();
        Self {
            records,
            next_token,
        }
    }

    pub(crate) fn from_persisted_with_results(
        records: Vec<persistence::SearchRecord>,
        result_records: Vec<persistence::SearchResultRecord>,
    ) -> Self {
        Self::from_persisted_with_results_and_identities(records, result_records, BTreeMap::new())
    }

    pub(crate) fn from_persisted_with_results_and_identities(
        records: Vec<persistence::SearchRecord>,
        result_records: Vec<persistence::SearchResultRecord>,
        identities: BTreeMap<String, String>,
    ) -> Self {
        let mut results_by_search: BTreeMap<String, Vec<persistence::SearchResultRecord>> =
            BTreeMap::new();
        for result in result_records {
            results_by_search
                .entry(result.search_id.clone())
                .or_default()
                .push(result);
        }
        let mut retained_results = 0_usize;
        let parsed = records
            .iter()
            .filter_map(|record| {
                let remaining = MAX_TOTAL_SEARCH_RESULTS.saturating_sub(retained_results);
                let mut parsed = SearchRecord::from_persisted_with_results(
                    record,
                    results_by_search.remove(&record.id).unwrap_or_default(),
                )?;
                if let Some(external_id) = identities
                    .get(&record.id)
                    .filter(|external_id| !external_id.trim().is_empty())
                {
                    parsed.id = external_id.clone();
                }
                parsed.results.truncate(remaining);
                retained_results = retained_results.saturating_add(parsed.results.len());
                Some(parsed)
            })
            .collect::<Vec<_>>();
        let next_token = next_search_token_hint(&parsed);
        let mut seen_ids = std::collections::HashSet::new();
        let mut seen_tokens = std::collections::HashSet::new();
        let records = parsed
            .into_iter()
            .filter(|record| seen_ids.insert(record.id.clone()) && seen_tokens.insert(record.token))
            .take(MAX_SEARCH_RECORDS)
            .collect::<Vec<_>>();
        Self {
            records,
            next_token,
        }
    }

    pub(crate) fn create(
        &mut self,
        id: Option<String>,
        query: String,
        target: &'static str,
        target_name: Option<String>,
        results: Vec<FileEntry>,
        ttl_seconds: u64,
    ) -> Result<SearchCreateOutcome, SearchCreateError> {
        let query = query.trim().to_owned();
        if query.is_empty() {
            return Err(SearchCreateError::InvalidQuery);
        }
        let id = id.filter(|value| !value.trim().is_empty());
        if id.as_deref().is_some_and(|id| {
            self.records
                .iter()
                .any(|record| search_identifier_matches(record, id))
        }) {
            return Err(SearchCreateError::DuplicateId);
        }
        let token = self.allocate_token()?;
        if id.as_deref().is_some_and(|id| id == token.to_string()) {
            return Err(SearchCreateError::DuplicateId);
        }
        let expired = self.expire_due();
        let mut evicted = Vec::new();
        if self.records.len() >= MAX_SEARCH_RECORDS {
            let Some(index) = self
                .records
                .iter()
                .enumerate()
                .filter(|(_, record)| record.status != "active")
                .min_by_key(|(_, record)| record.updated_at)
                .map(|(index, _)| index)
            else {
                return Err(SearchCreateError::CapacityFull);
            };
            evicted.push(self.records.remove(index));
        }
        let now = unix_timestamp();
        let aggregate_remaining = MAX_TOTAL_SEARCH_RESULTS.saturating_sub(self.total_results());
        let record = SearchRecord {
            id: id.unwrap_or_else(|| token.to_string()),
            token,
            query: truncate_utf8_bytes(query, MAX_SEARCH_QUERY_BYTES),
            target,
            target_name: target_name
                .map(|name| truncate_utf8_bytes(name, MAX_SEARCH_TARGET_NAME_BYTES)),
            status: "active",
            results: results
                .iter()
                .map(SearchResultEntry::from_file_entry)
                .take(MAX_SEARCH_RESULTS_PER_SEARCH.min(aggregate_remaining))
                .collect(),
            raw_response_count: 0,
            filtered_out_count: 0,
            ignored_result_count: 0,
            hidden_locked_count: 0,
            fallback_attempts: 0,
            ttl_seconds,
            expires_at: now.saturating_add(ttl_seconds),
            created_at: now,
            updated_at: now,
        };
        self.next_token = token.wrapping_add(1).max(1);
        self.records.push(record.clone());
        Ok(SearchCreateOutcome {
            record,
            evicted,
            expired,
        })
    }

    pub(crate) fn create_scheduled_wishlist(
        &mut self,
        query: String,
        ttl_seconds: u64,
    ) -> Result<SearchCreateOutcome, SearchCreateError> {
        self.create_scheduled_wishlist_for_item(query, None, ttl_seconds)
    }

    pub(crate) fn create_scheduled_wishlist_for_item(
        &mut self,
        query: String,
        wishlist_item_id: Option<String>,
        ttl_seconds: u64,
    ) -> Result<SearchCreateOutcome, SearchCreateError> {
        self.create(
            None,
            query,
            "wishlist",
            wishlist_item_id,
            Vec::new(),
            ttl_seconds,
        )
    }

    pub(crate) fn allocate_token(&self) -> Result<u32, SearchCreateError> {
        let mut candidate = self.next_token.max(1);
        for _ in 0..=MAX_SEARCH_RECORDS {
            let text = candidate.to_string();
            if !self
                .records
                .iter()
                .any(|record| record.token == candidate || record.id == text)
            {
                return Ok(candidate);
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        Err(SearchCreateError::TokenSpaceExhausted)
    }

    pub(crate) fn get_by_identifier(&self, id: &str) -> Option<SearchRecord> {
        self.records
            .iter()
            .find(|record| search_identifier_matches(record, id))
            .cloned()
    }

    pub(crate) fn remove_by_identifier(&mut self, id: &str) -> Option<SearchRecord> {
        if let Some(pos) = self
            .records
            .iter()
            .position(|record| search_identifier_matches(record, id))
        {
            Some(self.records.remove(pos))
        } else {
            None
        }
    }

    pub(crate) fn complete(&mut self, token: u32) -> Option<(SearchRecord, bool)> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.token == token)?;
        if record.status != "active" {
            return Some((record.clone(), false));
        }
        let transitioned = true;
        record.status = "completed";
        record.updated_at = unix_timestamp();
        Some((record.clone(), transitioned))
    }

    pub(crate) fn set_status_by_token(
        &mut self,
        token: u32,
        status: &'static str,
    ) -> Option<(SearchRecord, bool)> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.token == token)?;
        if record.status != "active" && record.status != status {
            return Some((record.clone(), false));
        }
        let transitioned = record.status != status;
        record.status = status;
        if transitioned {
            record.updated_at = unix_timestamp();
        }
        Some((record.clone(), transitioned))
    }

    pub(crate) fn update_by_identifier(
        &mut self,
        id: &str,
        query: Option<String>,
        status: Option<&str>,
    ) -> Option<(SearchRecord, bool)> {
        let record = self
            .records
            .iter_mut()
            .find(|record| search_identifier_matches(record, id))?;
        let mut updated = false;
        if let Some(query) = query.map(|value| value.trim().to_owned()) {
            let query = truncate_utf8_bytes(query, MAX_SEARCH_QUERY_BYTES);
            if !query.is_empty() && query != record.query {
                record.query = query;
                updated = true;
            }
        }
        if let Some(status) = status.and_then(normalize_search_status) {
            if status != record.status && record.status == "active" {
                record.status = status;
                updated = true;
            }
        }
        if updated {
            record.updated_at = unix_timestamp();
        }
        Some((record.clone(), updated))
    }

    pub(crate) fn expire_due(&mut self) -> Vec<SearchRecord> {
        self.expire_due_excluding_target(None)
    }

    pub(crate) fn expire_due_excluding_target(
        &mut self,
        excluded_target: Option<&str>,
    ) -> Vec<SearchRecord> {
        let now = unix_timestamp();
        let mut expired = Vec::new();
        for record in &mut self.records {
            if record.status == "active"
                && record.expires_at <= now
                && excluded_target != Some(record.target)
            {
                record.status = "expired";
                record.updated_at = now;
                expired.push(record.clone());
            }
        }
        expired
    }

    pub(crate) fn reset_for_fallback(
        &mut self,
        token: u32,
        query: String,
        ttl_seconds: u64,
    ) -> Option<SearchRecord> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.token == token)?;
        if record.target != "wishlist"
            || record.fallback_attempts >= search_fallback::MAXIMUM_FALLBACK_QUERIES
        {
            return None;
        }
        let query = query.trim();
        if query.is_empty() {
            return None;
        }
        let now = unix_timestamp();
        record.query = truncate_utf8_bytes(query.to_owned(), MAX_SEARCH_QUERY_BYTES);
        record.status = "active";
        record.results.clear();
        record.raw_response_count = 0;
        record.filtered_out_count = 0;
        record.ignored_result_count = 0;
        record.hidden_locked_count = 0;
        record.fallback_attempts = record.fallback_attempts.saturating_add(1);
        record.ttl_seconds = ttl_seconds;
        record.expires_at = now.saturating_add(ttl_seconds);
        record.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn prune_expired(&mut self) -> Vec<SearchRecord> {
        self.expire_due();
        let mut pruned = Vec::new();
        self.records.retain(|record| {
            if record.status == "expired" {
                pruned.push(record.clone());
                false
            } else {
                true
            }
        });
        pruned
    }

    pub(crate) fn add_peer_response(
        &mut self,
        response: &FileSearchResponse,
    ) -> Option<SearchRecord> {
        self.add_peer_response_filtered(response, &[], None)
            .map(|(record, _)| record)
    }

    pub(crate) fn add_peer_response_filtered(
        &mut self,
        response: &FileSearchResponse,
        ignored_results: &[WishlistIgnoredResult],
        wishlist_policy: Option<&WishlistResultPolicy>,
    ) -> Option<(SearchRecord, Vec<SearchResultEntry>)> {
        let aggregate_remaining = MAX_TOTAL_SEARCH_RESULTS.saturating_sub(self.total_results());
        let record = self
            .records
            .iter_mut()
            .find(|record| record.token == response.token && record.status == "active")?;
        let result_limit = wishlist_policy
            .map(|policy| policy.max_results)
            .unwrap_or(MAX_SEARCH_RESULTS_PER_SEARCH);
        if let Some(policy) = wishlist_policy {
            record.raw_response_count = record.raw_response_count.saturating_add(1);
            for (entry, locked) in response
                .results
                .iter()
                .map(|entry| (entry, false))
                .chain(response.private_results.iter().map(|entry| (entry, true)))
            {
                if !policy.filter.matches_file_entry(entry) {
                    record.filtered_out_count = record.filtered_out_count.saturating_add(1);
                } else if ignored_results
                    .iter()
                    .any(|rule| rule.matches(&response.username, &entry.filename))
                {
                    record.ignored_result_count = record.ignored_result_count.saturating_add(1);
                } else if locked {
                    record.hidden_locked_count = record.hidden_locked_count.saturating_add(1);
                }
            }
        }
        let before = record.results.len();
        let mut appended = record.extend_results_with_limit_bounded(
            response
                .results
                .iter()
                .filter(|entry| {
                    wishlist_policy.is_none_or(|policy| policy.filter.matches_file_entry(entry))
                        && !ignored_results
                            .iter()
                            .any(|rule| rule.matches(&response.username, &entry.filename))
                })
                .map(|entry| SearchResultEntry::from_peer_response_entry(response, entry, false)),
            aggregate_remaining,
            result_limit,
        );
        let aggregate_remaining = aggregate_remaining.saturating_sub(record.results.len() - before);
        if wishlist_policy.is_none() {
            appended.extend(
                record.extend_results_with_limit_bounded(
                    response
                        .private_results
                        .iter()
                        .filter(|entry| {
                            !ignored_results
                                .iter()
                                .any(|rule| rule.matches(&response.username, &entry.filename))
                        })
                        .map(|entry| {
                            SearchResultEntry::from_peer_response_entry(response, entry, true)
                        }),
                    aggregate_remaining,
                    result_limit,
                ),
            );
        }
        let now = unix_timestamp();
        record.updated_at = now;
        record.expires_at = now.saturating_add(record.ttl_seconds);
        Some((record.clone(), appended))
    }

    pub(crate) fn suppress_ignored_result(
        &mut self,
        rule: &WishlistIgnoredResult,
    ) -> Vec<SearchRecord> {
        let mut changed = Vec::new();
        for record in &mut self.records {
            if record.wishlist_item_id() != Some(rule.wishlist_item_id.as_str()) {
                continue;
            }
            let before = record.results.len();
            record.results.retain(|entry| {
                let Some(username) = entry.peer_username.as_deref() else {
                    return true;
                };
                !rule.matches(username, &entry.filename)
            });
            if record.results.len() != before {
                record.updated_at = unix_timestamp();
                changed.push(record.clone());
            }
        }
        changed
    }

    pub(crate) fn get(&self, token: u32) -> Option<SearchRecord> {
        self.records
            .iter()
            .find(|record| record.token == token)
            .cloned()
    }

    pub(crate) fn total_results(&self) -> usize {
        self.records.iter().map(|record| record.results.len()).sum()
    }

    pub(crate) fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self.filtered_records(&filter);
        let filtered_count = records.len();
        let expired = self
            .records
            .iter()
            .filter(|record| record.status == "expired")
            .count();
        let records = records
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(SearchRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"filtered_count\":{},\"expired\":{},\"offset\":{},\"limit\":{},\"next_token\":{}}}",
            records,
            self.records.len(),
            filtered_count,
            expired,
            filter.offset,
            json_usize_option(filter.limit),
            self.next_token
        )
    }

    pub(crate) fn controller_list_json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .filtered_records(&filter)
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(SearchRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }

    pub(crate) fn wishlist_history_json(&self, item_id: &str, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .rev()
            .filter(|record| record.wishlist_item_id() == Some(item_id))
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(DEFAULT_LIST_LIMIT))
            .map(SearchRecord::history_summary)
            .collect::<Vec<_>>();
        serde_json::Value::Array(records).to_string()
    }

    pub(crate) fn filtered_records<'a>(
        &'a self,
        filter: &RecordListFilter,
    ) -> Vec<&'a SearchRecord> {
        self.records
            .iter()
            .filter(|record| {
                filter
                    .status
                    .as_deref()
                    .is_none_or(|status| record.status == status)
            })
            .filter(|record| {
                filter
                    .target
                    .as_deref()
                    .is_none_or(|target| record.target == target)
            })
            .filter(|record| {
                filter.q.as_deref().is_none_or(|q| {
                    record.query.to_ascii_lowercase().contains(q)
                        || record
                            .target_name
                            .as_deref()
                            .is_some_and(|target| target.to_ascii_lowercase().contains(q))
                })
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn summary_json(&self) -> String {
        let active = self
            .records
            .iter()
            .filter(|record| record.status == "active")
            .count();
        let completed = self
            .records
            .iter()
            .filter(|record| record.status == "completed")
            .count();
        let expired = self
            .records
            .iter()
            .filter(|record| record.status == "expired")
            .count();
        let results = self
            .records
            .iter()
            .map(|record| record.results.len())
            .sum::<usize>();
        let global = self
            .records
            .iter()
            .filter(|record| record.target == "global")
            .count();
        let user = self
            .records
            .iter()
            .filter(|record| record.target == "user")
            .count();
        let room = self
            .records
            .iter()
            .filter(|record| record.target == "room")
            .count();
        let wishlist = self
            .records
            .iter()
            .filter(|record| record.target == "wishlist")
            .count();
        format!(
            "{{\"total\":{},\"active\":{},\"completed\":{},\"expired\":{},\"results\":{},\"global\":{},\"user\":{},\"room\":{},\"wishlist\":{},\"next_token\":{}}}",
            self.records.len(),
            active,
            completed,
            expired,
            results,
            global,
            user,
            room,
            wishlist,
            self.next_token
        )
    }

    pub(crate) fn transfer_alternatives_json(&self, transfer: &TransferEntry) -> String {
        let original_basename = virtual_basename(&transfer.filename).to_ascii_lowercase();
        let alternatives = self
            .records
            .iter()
            .flat_map(|record| record.results.iter())
            .filter(|result| {
                result.peer_username.is_some()
                    && result.peer_username != transfer.peer_username
                    && virtual_basename(&result.filename).eq_ignore_ascii_case(&original_basename)
            })
            .map(|result| {
                serde_json::json!({
                    "username": result.peer_username.as_deref().unwrap_or_default(),
                    "filename": result.filename,
                    "size": result.size,
                    "locked": result.locked,
                    "slot_free": result.slot_free,
                    "average_speed": result.average_speed,
                    "queue_length": result.queue_length,
                })
            })
            .collect::<Vec<_>>();
        let count = alternatives.len();
        serde_json::json!({
            "transfer_id": transfer.id,
            "filename": transfer.filename,
            "alternatives": alternatives,
            "count": count,
        })
        .to_string()
    }

    pub(crate) fn find_transfer_alternative(
        &self,
        transfer: &TransferEntry,
        username: &str,
        filename: Option<&str>,
    ) -> Option<SearchResultEntry> {
        let original_basename = virtual_basename(&transfer.filename);
        self.records
            .iter()
            .flat_map(|record| record.results.iter())
            .find(|result| {
                result
                    .peer_username
                    .as_deref()
                    .is_some_and(|peer| peer.eq_ignore_ascii_case(username))
                    && filename.map_or_else(
                        || {
                            virtual_basename(&result.filename)
                                .eq_ignore_ascii_case(original_basename)
                        },
                        |filename| result.filename == filename,
                    )
            })
            .cloned()
    }

    pub(crate) fn first_transfer_alternative(
        &self,
        transfer: &TransferEntry,
    ) -> Option<SearchResultEntry> {
        let original_basename = virtual_basename(&transfer.filename);
        self.records
            .iter()
            .flat_map(|record| record.results.iter())
            .find(|result| {
                result.peer_username.is_some()
                    && result.peer_username != transfer.peer_username
                    && virtual_basename(&result.filename).eq_ignore_ascii_case(original_basename)
            })
            .cloned()
    }

    pub(crate) fn best_transfer_alternative<F>(
        &self,
        transfer: &TransferEntry,
        size_tolerance_percent: f64,
        peer_is_cooling_down: F,
    ) -> Option<SearchResultEntry>
    where
        F: Fn(&str) -> bool,
    {
        let original_basename = virtual_basename(&transfer.filename);
        self.records
            .iter()
            .flat_map(|record| record.results.iter())
            .filter(|result| {
                let Some(username) = result.peer_username.as_deref() else {
                    return false;
                };
                !result.locked
                    && transfer
                        .peer_username
                        .as_deref()
                        .is_none_or(|original| !username.eq_ignore_ascii_case(original))
                    && !peer_is_cooling_down(username)
                    && virtual_basename(&result.filename).eq_ignore_ascii_case(original_basename)
                    && alternate_size_is_eligible(
                        transfer.size,
                        result.size,
                        size_tolerance_percent,
                    )
            })
            .max_by(|left, right| {
                left.slot_free
                    .unwrap_or(false)
                    .cmp(&right.slot_free.unwrap_or(false))
                    .then_with(|| {
                        left.average_speed
                            .unwrap_or(0)
                            .cmp(&right.average_speed.unwrap_or(0))
                    })
                    .then_with(|| {
                        right
                            .queue_length
                            .unwrap_or(u32::MAX)
                            .cmp(&left.queue_length.unwrap_or(u32::MAX))
                    })
            })
            .cloned()
    }
}

fn search_result_identity(result: &SearchResultEntry) -> (String, String, u64) {
    (
        result
            .peer_username
            .as_deref()
            .unwrap_or_default()
            .to_owned(),
        result
            .filename
            .replace('\\', "/")
            .trim()
            .to_ascii_lowercase(),
        result.size,
    )
}
